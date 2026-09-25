// Serviços de terceiros (atualizador, licença, telemetria). "Terceiro" pelo CAMINHO do executável fora de
// %SystemRoot%, não pelo nome, que é texto livre. Oferece MANUAL, não Desativado: em Manual o serviço para de
// subir no boot mas sobe quando o programa pede, e nada quebra.

use super::{registry, shell};
use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartMode {
    Automatic,
    Manual,
    Disabled,
    Kernel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub name: String,
    pub display_name: String,
    pub path: String,
    pub start_mode: StartMode,
    pub running: bool,
    /// Serviço em `svchost` compartilhado não se mede separado: fica em branco, sem chute.
    pub ram_mb: Option<f64>,
    pub protected: Option<String>,
}

/// Antes de qualquer classificação: antivírus, áudio e vídeo moram fora de %SystemRoot% e contariam como
/// terceiros, mas mexer neles é estragar.
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

pub fn e_do_sistema(caminho: &str, system_root: &str) -> bool {
    if caminho.trim().is_empty() {
        // Sem caminho, fica de fora: na dúvida, não mexe.
        return true;
    }

    let executavel = caminho.trim().trim_start_matches('"');
    let normalizado = executavel.replace('/', "\\").to_lowercase();
    let raiz = system_root.replace('/', "\\").to_lowercase();

    normalizado.starts_with(&raiz)
}

/// Números iguais em qualquer idioma, ao contrário do `sc qc`, que já quebrou este projeto.
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
    #[serde(default, skip)]
    _process_id: Option<u32>,
    working_set_mb: Option<f64>,
    /// Acima de 1, a memória não é atribuível a nenhum deles.
    shares_process: Option<u32>,
}

fn system_root() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
}

/// `Err` quando o WMI não responde: lista vazia diria "Nenhum serviço de terceiros".
fn consultar() -> Result<Vec<RawService>, String> {
    // Um PowerShell só resolve serviço, processo e compartilhamento: três chamadas levariam segundos em máquina
    // fraca. A lista de processos pode falhar: a memória vira "sem medida".
    let script = "\
        $svc = @(Get-CimInstance Win32_Service -ErrorAction Stop); \
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

    shell::json_da_saida(shell::powershell(script), "os serviços do Windows")
}

pub fn listar_de_terceiros() -> Result<Vec<ServiceEntry>, String> {
    let raiz = system_root();

    let mut lista: Vec<ServiceEntry> = consultar()?
        .into_iter()
        .filter_map(|bruto| {
            let name = bruto.name?;
            let path = bruto.path_name.unwrap_or_default();

            if e_do_sistema(&path, &raiz) {
                return None;
            }

            let display_name = bruto.display_name.unwrap_or_else(|| name.clone());

            // Do registro, a mesma fonte que a escrita usa: lista e ação não divergem.
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

    Ok(lista)
}

/// Não há opção de desativar, de propósito.
pub fn definir_inicio(name: &str, automatico: bool) -> Result<ChangeRecord, String> {
    if !registry::is_elevated() {
        return Err("Mexer em serviços exige executar como administrador.".to_string());
    }

    let raiz = system_root();

    // Refeita aqui: o comando vem por IPC, e tela com dado velho não pode virar permissão para mexer no sistema.
    let bruto = consultar()?
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
        assert!(e_do_sistema("\"C:\\Windows\\System32\\spoolsv.exe\"", raiz));
        assert!(e_do_sistema("c:\\windows\\system32\\lsass.exe", raiz));

        assert!(!e_do_sistema("\"C:\\Program Files\\Google\\Update\\GoogleUpdate.exe\"", raiz));
        assert!(!e_do_sistema("C:\\Program Files (x86)\\IObit\\service.exe", raiz));
    }

    #[test]
    fn nome_enganoso_nao_engana_o_criterio() {
        assert!(!e_do_sistema(
            "\"C:\\Program Files\\Coisa\\Windows Update Helper.exe\"",
            "C:\\Windows"
        ));
    }

    #[test]
    fn caminho_vazio_erra_para_o_lado_de_nao_mexer() {
        assert!(e_do_sistema("", "C:\\Windows"));
        assert!(e_do_sistema("   ", "C:\\Windows"));
    }

    #[test]
    fn antivirus_e_audio_nunca_entram_na_lista_de_acao() {
        assert!(protecao("WinDefend", "Antivírus do Microsoft Defender").is_some());
        assert!(protecao("avast! Antivirus", "Avast").is_some());
        assert!(protecao("RtkAudioUniversalService", "Realtek Audio Universal").is_some());
        assert!(protecao("NVDisplay.ContainerLocalSystem", "NVIDIA Display").is_some());

        assert!(protecao("gupdate", "Serviço do Google Update").is_none());
    }

    #[test]
    fn codigo_de_inicio_vem_do_numero_e_nao_do_texto() {
        assert_eq!(modo_do_codigo(2), StartMode::Automatic);
        assert_eq!(modo_do_codigo(3), StartMode::Manual);
        assert_eq!(modo_do_codigo(4), StartMode::Disabled);
        assert_eq!(modo_do_codigo(0), StartMode::Kernel);
        assert_eq!(modo_do_codigo(1), StartMode::Kernel);
    }

    #[test]
    fn lista_desta_maquina_nao_traz_servico_do_windows() {
        let raiz = system_root();
        let lista = listar_de_terceiros().expect("os serviços desta máquina precisam ser legíveis");

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
            assert_ne!(s.start_mode, StartMode::Kernel, "{}", s.display_name);
        }
    }

    #[test]
    fn ordem_poe_o_que_custa_no_topo() {
        let lista = listar_de_terceiros().expect("os serviços desta máquina precisam ser legíveis");

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
        let erro = definir_inicio("Spooler", false).unwrap_err();

        assert!(
            erro.contains("próprio Windows") || erro.contains("administrador"),
            "recusa inesperada: {}",
            erro
        );
    }
}
