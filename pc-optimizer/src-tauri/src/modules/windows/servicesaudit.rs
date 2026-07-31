// Auditor de serviços de terceiros
//
// Programa instalado quase nunca se contenta em ser um programa. Ele deixa um
// serviço para trás — atualizador, licenciamento, telemetria, "assistente" —
// que sobe junto com o Windows, antes mesmo de você fazer login, e fica lá o
// dia inteiro. É a terceira perna do problema: inicialização, tarefa agendada e
// serviço. As duas primeiras já estão no produto; esta faltava.
//
// DUAS DECISÕES QUE DEFINEM ESTE MÓDULO
//
// 1. O critério de "terceiro" é o CAMINHO DO EXECUTÁVEL, não o nome. Serviço do
//    Windows mora dentro de %SystemRoot%. Nome pode ser qualquer coisa — existe
//    malware chamado "Windows Update Helper" e existe serviço legítimo com nome
//    esquisito. O caminho não mente e não muda de idioma.
//
// 2. Oferecemos MANUAL, não Desativado. Desativar é o que os tutoriais mandam
//    fazer, e é errado: quando o programa precisa do serviço, ele não sobe, e o
//    programa quebra de um jeito que ninguém liga ao "otimizador" que rodou
//    semana passada. Em Manual, o serviço para de subir sozinho no boot mas
//    ainda sobe quando alguém pede. Você ganha o boot e não quebra nada.

use super::{registry, shell};
use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartMode {
    /// Sobe sozinho no boot.
    Automatic,
    /// Sobe só quando alguém pede. É para cá que levamos os de terceiros.
    Manual,
    Disabled,
    /// Boot e System: carregados antes do Windows subir. Nunca são de terceiros
    /// comuns, e nunca são mexidos aqui.
    Kernel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub name: String,
    pub display_name: String,
    pub path: String,
    pub start_mode: StartMode,
    pub running: bool,
    /// Memória do processo do serviço, quando ele está rodando e o processo é
    /// dele sozinho. Serviço hospedado em `svchost` compartilhado não tem como
    /// ser medido separado, e aí fica em branco em vez de receber um chute.
    pub ram_mb: Option<f64>,
    /// Preenchido quando o serviço não pode ser mexido, com o motivo.
    pub protected: Option<String>,
}

/// Serviços que nunca são oferecidos para mudar, aconteça o que acontecer.
///
/// A lista de proteção vem antes de qualquer classificação, igual ao detector
/// de programas de fábrica. Antivírus, áudio e vídeo moram fora de %SystemRoot%
/// e portanto contam como "de terceiros" pelo critério do caminho — mas mexer
/// neles é a diferença entre otimizar e estragar. Placa de som que emudece e
/// antivírus que não sobe é o tipo de estrago que o cliente descobre depois e
/// não perdoa.
const NUNCA_MEXER: [(&str, &str); 12] = [
    ("defender", "proteção do Windows"),
    ("antivir", "antivírus"),
    ("avast", "antivírus"),
    ("avg", "antivírus"),
    ("kaspersky", "antivírus"),
    ("mcafee", "antivírus"),
    ("norton", "antivírus"),
    ("eset", "antivírus"),
    ("bitdefender", "antivírus"),
    ("realtek", "áudio ou rede da placa-mãe"),
    ("nvidia", "vídeo"),
    ("amd", "vídeo ou chipset"),
];

/// Motivo pelo qual o serviço não pode ser mexido, se houver.
pub fn protecao(nome: &str, exibicao: &str) -> Option<String> {
    let alvo = format!("{} {}", nome, exibicao).to_lowercase();

    NUNCA_MEXER.iter().find_map(|(marca, tipo)| {
        alvo.contains(marca).then(|| {
            format!(
                "O Otimiza não mexe em serviço de {} — o risco de estragar é maior \
                 que qualquer ganho.",
                tipo
            )
        })
    })
}

/// Se o executável do serviço mora dentro da pasta do Windows.
///
/// Este é o critério de "é do sistema". Comparar nome não serve: nome é texto
/// livre que qualquer instalador escolhe.
pub fn e_do_sistema(caminho: &str, system_root: &str) -> bool {
    if caminho.trim().is_empty() {
        // Sem caminho não dá para afirmar que é de terceiro, e na dúvida o
        // serviço fica de fora da lista — errar para o lado de não mexer.
        return true;
    }

    // O caminho vem com aspas e argumentos: `"C:\Windows\system32\svchost.exe" -k netsvcs`.
    let executavel = caminho.trim().trim_start_matches('"');
    let normalizado = executavel.replace('/', "\\").to_lowercase();
    let raiz = system_root.replace('/', "\\").to_lowercase();

    normalizado.starts_with(&raiz)
}

/// Traduz o código `Start` do registro.
///
/// Os valores são numéricos e iguais em qualquer idioma do Windows — ao
/// contrário da saída do `sc qc`, que sai traduzida e já quebrou este projeto
/// uma vez.
pub fn modo_do_codigo(start: u32) -> StartMode {
    match start {
        0 | 1 => StartMode::Kernel,
        2 => StartMode::Automatic,
        3 => StartMode::Manual,
        4 => StartMode::Disabled,
        _ => StartMode::Manual,
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawService {
    name: Option<String>,
    display_name: Option<String>,
    path_name: Option<String>,
    state: Option<String>,
    // O PID em si não é usado: ele serve só para o PowerShell casar processo
    // com serviço antes de devolver a memória já resolvida.
    #[serde(default, skip)]
    _process_id: Option<u32>,
    working_set_mb: Option<f64>,
    /// Quantos serviços dividem o mesmo processo. Acima de 1 a memória não é
    /// atribuível a nenhum deles.
    shares_process: Option<u32>,
}

fn system_root() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
}

fn consultar() -> Vec<RawService> {
    // Um único PowerShell resolve serviço, processo e quantos serviços dividem
    // aquele processo. Fazer isso em três chamadas separadas levaria segundos
    // numa máquina fraca, que é justamente a máquina deste produto.
    let script = "\
        $svc = @(Get-CimInstance Win32_Service -ErrorAction SilentlyContinue); \
        $porProcesso = @{}; \
        foreach ($s in $svc) { if ($s.ProcessId -gt 0) { \
          $porProcesso[$s.ProcessId] = 1 + [int]$porProcesso[$s.ProcessId] } } \
        $mem = @{}; \
        foreach ($p in @(Get-Process -ErrorAction SilentlyContinue)) { \
          $mem[$p.Id] = [math]::Round($p.WorkingSet64/1MB, 1) } \
        ConvertTo-Json -Compress -Depth 3 -InputObject @($svc | ForEach-Object { \
          [ordered]@{ Name = $_.Name; DisplayName = $_.DisplayName; \
            PathName = $_.PathName; State = $_.State; ProcessId = $_.ProcessId; \
            WorkingSetMb = $mem[[int]$_.ProcessId]; \
            SharesProcess = $porProcesso[$_.ProcessId] } })";

    match shell::powershell(script) {
        Ok(o) if o.success && !o.stdout.trim().is_empty() => {
            serde_json::from_str(&o.stdout).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

/// Todos os serviços que não são do Windows.
pub fn listar_de_terceiros() -> Vec<ServiceEntry> {
    let raiz = system_root();

    let mut lista: Vec<ServiceEntry> = consultar()
        .into_iter()
        .filter_map(|bruto| {
            let name = bruto.name?;
            let path = bruto.path_name.unwrap_or_default();

            if e_do_sistema(&path, &raiz) {
                return None;
            }

            let display_name = bruto.display_name.unwrap_or_else(|| name.clone());

            // O modo vem do registro, não do WMI: é a mesma fonte que a
            // aplicação usa para escrever, então lista e ação não divergem.
            let start_mode = match registry::read(
                "HKLM",
                &format!(r"SYSTEM\CurrentControlSet\Services\{}", name),
                "Start",
            ) {
                Ok(crate::modules::changelog::PreviousValue::Dword(codigo)) => {
                    modo_do_codigo(codigo)
                }
                _ => return None,
            };

            let running = bruto.state.as_deref() == Some("Running");

            // Memória só quando o processo é deste serviço sozinho.
            let ram_mb = match (running, bruto.shares_process.unwrap_or(0)) {
                (true, 1) => bruto.working_set_mb,
                _ => None,
            };

            Some(ServiceEntry {
                protected: protecao(&name, &display_name),
                name,
                display_name,
                path,
                start_mode,
                running,
                ram_mb,
            })
        })
        .collect();

    // Quem sobe sozinho no boot primeiro: é onde está o custo.
    lista.sort_by(|a, b| {
        let peso = |s: &ServiceEntry| match (s.protected.is_some(), s.start_mode) {
            (false, StartMode::Automatic) => 0,
            (false, _) => 1,
            (true, _) => 2,
        };

        peso(a)
            .cmp(&peso(b))
            .then_with(|| b.ram_mb.unwrap_or(0.0).total_cmp(&a.ram_mb.unwrap_or(0.0)))
            .then_with(|| a.display_name.cmp(&b.display_name))
    });

    lista
}

/// Leva um serviço de terceiro para Manual, ou devolve para Automático.
///
/// Não existe opção de desativar aqui, e isso é de propósito — ver o cabeçalho
/// do arquivo.
pub fn definir_inicio(name: &str, automatico: bool) -> Result<ChangeRecord, String> {
    if !registry::is_elevated() {
        return Err("Mexer em serviços exige executar como administrador.".to_string());
    }

    let raiz = system_root();

    // A checagem é refeita aqui, e não confiada à interface: o comando é
    // exposto por IPC, e uma tela com dado velho não pode virar permissão para
    // mexer num serviço do sistema.
    let bruto = consultar()
        .into_iter()
        .find(|s| s.name.as_deref() == Some(name))
        .ok_or_else(|| format!("Serviço `{}` não existe nesta máquina.", name))?;

    let caminho = bruto.path_name.clone().unwrap_or_default();
    let exibicao = bruto.display_name.clone().unwrap_or_else(|| name.to_string());

    if e_do_sistema(&caminho, &raiz) {
        return Err(format!(
            "`{}` é um serviço do próprio Windows e não é mexido pelo Otimiza.",
            exibicao
        ));
    }

    if let Some(motivo) = protecao(name, &exibicao) {
        return Err(motivo);
    }

    let anterior = super::services::query_start_type(name)?;
    let alvo = if automatico { "auto" } else { "demand" };

    if anterior == alvo {
        return Err(format!("`{}` já está nesse modo.", exibicao));
    }

    super::services::set_start_type(name, alvo)?;

    Ok(ChangeRecord::ServiceStartType {
        service: name.to_string(),
        previous: anterior,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminho_dentro_do_windows_e_do_sistema() {
        let raiz = "C:\\Windows";

        assert!(e_do_sistema("C:\\Windows\\system32\\svchost.exe -k netsvcs", raiz));
        // Vem com aspas na maioria dos serviços.
        assert!(e_do_sistema("\"C:\\Windows\\System32\\spoolsv.exe\"", raiz));
        // Maiúsculas e minúsculas não podem decidir isso.
        assert!(e_do_sistema("c:\\windows\\system32\\lsass.exe", raiz));

        assert!(!e_do_sistema("\"C:\\Program Files\\Google\\Update\\GoogleUpdate.exe\"", raiz));
        assert!(!e_do_sistema("C:\\Program Files (x86)\\IObit\\service.exe", raiz));
    }

    #[test]
    fn nome_enganoso_nao_engana_o_criterio() {
        // O nome diz Windows, o executável está em Program Files. É de terceiro,
        // e é exatamente por isso que o critério é o caminho.
        assert!(!e_do_sistema(
            "\"C:\\Program Files\\Coisa\\Windows Update Helper.exe\"",
            "C:\\Windows"
        ));
    }

    #[test]
    fn caminho_vazio_erra_para_o_lado_de_nao_mexer() {
        // Sem caminho não dá para afirmar nada, e a resposta segura é tratar
        // como do sistema — o serviço some da lista em vez de virar um botão.
        assert!(e_do_sistema("", "C:\\Windows"));
        assert!(e_do_sistema("   ", "C:\\Windows"));
    }

    #[test]
    fn antivirus_e_audio_nunca_entram_na_lista_de_acao() {
        assert!(protecao("WinDefend", "Antivírus do Microsoft Defender").is_some());
        assert!(protecao("avast! Antivirus", "Avast").is_some());
        assert!(protecao("RtkAudioUniversalService", "Realtek Audio Universal").is_some());
        assert!(protecao("NVDisplay.ContainerLocalSystem", "NVIDIA Display").is_some());

        // Atualizador comum não é protegido: é justamente o alvo.
        assert!(protecao("gupdate", "Serviço do Google Update").is_none());
    }

    #[test]
    fn codigo_de_inicio_vem_do_numero_e_nao_do_texto() {
        assert_eq!(modo_do_codigo(2), StartMode::Automatic);
        assert_eq!(modo_do_codigo(3), StartMode::Manual);
        assert_eq!(modo_do_codigo(4), StartMode::Disabled);
        // Driver de boot: nunca oferecido.
        assert_eq!(modo_do_codigo(0), StartMode::Kernel);
        assert_eq!(modo_do_codigo(1), StartMode::Kernel);
    }

    #[test]
    fn lista_desta_maquina_nao_traz_servico_do_windows() {
        let raiz = system_root();
        let lista = listar_de_terceiros();

        println!("{} serviços de terceiros:", lista.len());
        for s in lista.iter().take(12) {
            println!(
                "  [{:?}{}] {} — {}{}",
                s.start_mode,
                if s.running { ", rodando" } else { "" },
                s.display_name,
                s.ram_mb.map(|m| format!("{:.1} MB", m)).unwrap_or_else(|| "sem medida".into()),
                s.protected.as_ref().map(|_| " (protegido)").unwrap_or("")
            );
        }

        for s in &lista {
            assert!(
                !e_do_sistema(&s.path, &raiz),
                "serviço do Windows vazou para a lista: {} ({})",
                s.display_name,
                s.path
            );
            // Driver de boot não tem o que ser oferecido.
            assert_ne!(s.start_mode, StartMode::Kernel, "{}", s.display_name);
        }
    }

    #[test]
    fn ordem_poe_o_que_custa_no_topo() {
        let lista = listar_de_terceiros();

        // Os protegidos vão para o fim: eles são informação, não ação.
        let primeiro_protegido = lista.iter().position(|s| s.protected.is_some());
        let ultimo_livre = lista.iter().rposition(|s| s.protected.is_none());

        if let (Some(protegido), Some(livre)) = (primeiro_protegido, ultimo_livre) {
            assert!(
                protegido > livre,
                "serviço protegido apareceu antes de um que pode ser mexido"
            );
        }
    }

    #[test]
    fn servico_do_windows_e_recusado_mesmo_se_pedirem_pelo_nome() {
        // O comando é exposto por IPC. Uma tela com dado velho, ou qualquer
        // chamada direta, não pode virar permissão para mexer no sistema.
        let erro = definir_inicio("Spooler", false).unwrap_err();

        // Sem elevação a recusa vem antes, por outro motivo — as duas servem.
        assert!(
            erro.contains("próprio Windows") || erro.contains("administrador"),
            "recusa inesperada: {}",
            erro
        );
    }
}
