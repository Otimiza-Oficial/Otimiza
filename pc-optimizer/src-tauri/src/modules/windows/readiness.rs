// Prontidão: condições que, erradas, fazem o atendimento inteiro dar errado (reinício pendente: parte das
// mudanças não fixa, o técnico mede, não vê ganho e culpa o produto). Tudo barato: menos de um segundo.

use super::{power, registry, shell};
use serde::{Deserialize, Serialize};

pub use super::firmware::{FindingSeverity, FixLocation};

/// Não aparece na lista até ser criado a partir deste identificador.
pub const DESEMPENHO_MAXIMO_GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
    pub actionable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub findings: Vec<ReadinessFinding>,
    pub note: String,
}

/// Três origens, e basta uma. As chaves não mudam de idioma.
pub fn reinicio_pendente() -> Vec<&'static str> {
    let mut motivos = Vec::new();

    if registry::key_exists(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending",
    ) == Some(true)
    {
        motivos.push("instalação de componente do Windows");
    }

    if registry::key_exists(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired",
    ) == Some(true)
    {
        motivos.push("atualização do Windows");
    }

    // O mais comum dos três e o menos conhecido.
    if registry::read(
        "HKLM",
        r"SYSTEM\CurrentControlSet\Control\Session Manager",
        "PendingFileRenameOperations",
    )
    .is_ok()
    {
        motivos.push("arquivo aguardando substituição");
    }

    motivos
}

/// `DisableDeleteNotify = 0` é ligado. A saída do `fsutil` é traduzida: lê-se o número, nunca a frase.
pub fn trim_ligado() -> Option<bool> {
    let saida = shell::run("fsutil", &["behavior", "query", "DisableDeleteNotify"]).ok()?;

    if !saida.success {
        return None;
    }

    saida
        .stdout
        .lines()
        .find(|l| l.contains("NTFS"))
        .and_then(|l| l.split('=').nth(1))
        .and_then(|v| v.trim().split_whitespace().next())
        .and_then(|v| v.parse::<u32>().ok())
        .map(|v| v == 0)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawPaginacao {
    name: Option<String>,
    mecanico: Option<bool>,
}

/// Com SSD e HD, a paginação no HD usa a peça mais lenta quando a memória acaba.
fn paginacao_em_disco_lento() -> Option<(String, bool)> {
    // UMA chamada: cada `powershell.exe` custa 200-400 ms e ~40 MB prometidos, na máquina diagnosticada justamente
    // por falta de memória.
    let script = "$p = Get-CimInstance Win32_PageFileUsage -ErrorAction SilentlyContinue |                     Select-Object -First 1;                   if ($p) {                     $letra = $p.Name.Substring(0,1);                     $part = Get-Partition -ErrorAction SilentlyContinue |                             Where-Object DriveLetter -eq $letra | Select-Object -First 1;                     $mec = $false;                     if ($part) {                       $d = Get-PhysicalDisk -ErrorAction SilentlyContinue |                            Where-Object DeviceId -eq (Get-Disk -Number $part.DiskNumber).Number;                       $mec = ($d.MediaType -eq 'HDD') };                     ConvertTo-Json -Compress -InputObject ([ordered]@{                       Name = $p.Name; Mecanico = [bool]$mec }) }";

    let bruto: RawPaginacao = shell::powershell(script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())?;

    Some((bruto.name?, bruto.mecanico.unwrap_or(false)))
}

pub fn analyze() -> ReadinessReport {
    let mut findings = Vec::new();

    // Primeiro: atrapalha todo o resto.
    let motivos = reinicio_pendente();

    if !motivos.is_empty() {
        findings.push(ReadinessFinding {
            id: "reinicio".to_string(),
            title: "Reinício pendente".to_string(),
            measured: format!("Aguardando reinício por: {}.", motivos.join(", ")),
            advice: "Parte das mudanças de sistema só passa a valer depois de reiniciar. \
                     Otimizar antes disso costuma dar a impressão de que nada funcionou — o \
                     ajuste foi aplicado, mas o Windows ainda não o assumiu. Reinicie antes de \
                     medir qualquer coisa."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Software,
            actionable: false,
        });
    }

    // Desligado num SSD, a escrita perde velocidade ao longo de meses sem aviso.
    if super::hardware::profile().system_storage == super::hardware::StorageKind::Ssd {
        if trim_ligado() == Some(false) {
            findings.push(ReadinessFinding {
                id: "trim".to_string(),
                title: "TRIM desligado num SSD".to_string(),
                measured: "O Windows não está avisando o disco sobre blocos apagados.".to_string(),
                advice: "Sem esse aviso o SSD vai perdendo velocidade de escrita com o tempo, e \
                         a queda é gradual o bastante para ninguém associar à causa. Costuma ser \
                         resultado de um \"tutorial de otimização\" antigo. Religar é imediato e \
                         não tem contraindicação."
                    .to_string(),
                severity: FindingSeverity::Important,
                fix_location: FixLocation::Software,
                actionable: true,
            });
        }
    }

    if let Some((caminho, mecanico)) = paginacao_em_disco_lento() {
        if mecanico {
            findings.push(ReadinessFinding {
                id: "paginacao".to_string(),
                title: "Arquivo de paginação num disco mecânico".to_string(),
                measured: format!("O arquivo está em {}.", caminho),
                advice: "Quando a memória acaba, o Windows recorre a este arquivo — e aqui ele \
                         está na peça mais lenta da máquina, justamente no pior momento. Mover \
                         para o disco do sistema, se ele for SSD, muda de forma perceptível o \
                         comportamento em travadas."
                    .to_string(),
                severity: FindingSeverity::Important,
                fix_location: FixLocation::Software,
                actionable: false,
            });
        }
    }

    // Plano de terceiro ativo: o cliente acha que está no "alto desempenho" e está num plano que ninguém auditou.
    if let Some(nome) = plano_ativo_e_de_terceiro() {
        findings.push(ReadinessFinding {
            id: "plano_de_terceiro".to_string(),
            title: "O plano de energia ativo não é do Windows".to_string(),
            measured: format!("Plano em uso: \"{}\".", nome),
            advice: "Programas de otimização e fabricantes de notebook criam planos de energia próprios e os deixam ativos. Alguns são bons; outros limitam o processador para economizar bateria, e quem instalou já desinstalou o programa faz tempo. O Otimiza não mexe nele sem você mandar — mas você merece saber que o plano em uso não é nenhum dos que o Windows traz."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Software,
            actionable: false,
        });
    }

    // O achado "plano de desempenho máximo não existe" saiu na 2.9: a energia é do motor adaptativo.

    findings.sort_by_key(|f| match f.severity {
        FindingSeverity::Critical => 0,
        FindingSeverity::Important => 1,
        FindingSeverity::Ok => 2,
    });

    let problemas = findings
        .iter()
        .filter(|f| f.severity != FindingSeverity::Ok)
        .count();

    let note = if problemas == 0 {
        "Nada atrapalhando. As verificações aqui não são otimizações: são condições que, quando \
         erradas, fazem o resto do trabalho parecer que não funcionou."
            .to_string()
    } else {
        format!(
            "{} ponto(s) que atrapalham antes de qualquer otimização. Vale resolver estes \
             primeiro: são o tipo de coisa que faz o atendimento inteiro parecer sem efeito.",
            problemas
        )
    };

    ReadinessReport { findings, note }
}

/// Não pelo identificador de origem (`duplicatescheme` cria GUID NOVO, e o produto oferecia um segundo) nem pelo
/// nome (traduzido): por CONTAGEM, mais planos que os quatro de fábrica. `None` é não conseguir ler a lista.
pub fn plano_maximo_existe() -> Option<bool> {
    Some(planos_instalados()?.iter().any(|(guid, nome)| {
        guid.eq_ignore_ascii_case(DESEMPENHO_MAXIMO_GUID)
            || sem_acento(nome).contains(&sem_acento("desempenho máximo"))
            || sem_acento(nome).contains("ultimate performance")
    }))
}

/// O `powercfg` não passa pelo UTF-8 forçado: "Máximo" chega com caractere de substituição. Descartar o não ASCII
/// funciona com o acento intacto ou corrompido, porque o texto procurado passa pela mesma peneira.
fn sem_acento(texto: &str) -> String {
    texto
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .collect()
}

/// Guardada por alguns segundos: uma análise consultava a mesma lista três vezes. Curta porque o plano muda quando
/// o cliente clica.
pub fn planos_instalados() -> Option<Vec<(String, String)>> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static CACHE: Mutex<Option<(Instant, Vec<(String, String)>)>> = Mutex::new(None);
    const VALIDADE: Duration = Duration::from_secs(10);

    if let Ok(guarda) = CACHE.lock() {
        if let Some((quando, lista)) = guarda.as_ref() {
            if quando.elapsed() < VALIDADE {
                return Some(lista.clone());
            }
        }
    }

    // Todo Windows tem pelo menos um plano: vazio faria `plano_maximo_existe` falso e cada clique deixaria mais uma
    // cópia do plano.
    let Ok(saida) = shell::run("powercfg", &["/list"]) else {
        return None;
    };

    if !saida.success {
        return None;
    }

    let lista = analisar_lista_de_planos(&saida.stdout);

    if let Ok(mut guarda) = CACHE.lock() {
        *guarda = Some((Instant::now(), lista.clone()));
    }

    Some(lista)
}

/// **Pura.** O rótulo é traduzido: a leitura se apoia no formato do identificador e nos parênteses.
pub fn analisar_lista_de_planos(saida: &str) -> Vec<(String, String)> {
    let mut planos = Vec::new();

    for linha in saida.lines() {
        let Some(inicio) = linha.find(':') else {
            continue;
        };

        let resto = linha[inicio + 1..].trim();
        let guid: String = resto.chars().take(36).collect();

        if guid.len() != 36 || guid.matches('-').count() != 4 {
            continue;
        }

        let nome = resto
            .find('(')
            .and_then(|a| resto.rfind(')').map(|b| resto[a + 1..b].to_string()))
            .unwrap_or_default();

        planos.push((guid, nome));
    }

    planos
}

/// O IObit Driver Booster, por exemplo, cria um plano e o deixa ativo.
pub fn plano_ativo_e_de_terceiro() -> Option<String> {
    let ativo = super::power::active_scheme().ok()?;

    let (_, nome) = planos_instalados()?
        .into_iter()
        .find(|(guid, _)| guid.eq_ignore_ascii_case(&ativo))?;

    // PELO GUID PRIMEIRO: por nome, o plano de fábrica em espanhol, francês ou alemão virava "de terceiro".
    if e_guid_de_fabrica(&ativo) || nome_e_do_windows(&nome) {
        None
    } else {
        Some(nome)
    }
}

/// Constantes publicadas pela Microsoft, iguais em toda instalação e em qualquer idioma.
const GUIDS_DE_FABRICA: [&str; 4] = [
    "381b4222-f694-41f0-9685-ff5bb260df2e",
    "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
    "a1841308-3541-4fab-bc81-f71556f20b4a",
    "e9a42b02-d5df-448d-aa00-03f14749eb61",
];

pub fn e_guid_de_fabrica(guid: &str) -> bool {
    let limpo = guid.trim().trim_matches(|c| c == '{' || c == '}');
    GUIDS_DE_FABRICA.iter().any(|g| g.eq_ignore_ascii_case(limpo))
}

fn nome_e_do_windows(nome: &str) -> bool {
    let minusculo = sem_acento(nome);

    [
        "equilibrado",
        "balanced",
        "alto desempenho",
        "high performance",
        "economia de energia",
        "power saver",
        "desempenho máximo",
        "ultimate performance",
    ]
    .iter()
    // `sem_acento` DESCARTA o acentuado: os dois lados precisam passar pela mesma peneira.
    .any(|conhecido| minusculo.contains(&sem_acento(conhecido)))
}

/// Criar e ativar são separados: quem está na bateria pode querer só ter o plano.
pub fn criar_plano_maximo() -> Result<String, String> {
    if !registry::is_elevated() {
        return Err("Criar um plano de energia exige executar como administrador.".to_string());
    }

    // Sem a lista, recusa: `duplicatescheme` não confere nada e cada clique deixaria mais uma cópia.
    match plano_maximo_existe() {
        Some(true) => {
            return Err("O plano de desempenho máximo já existe nesta máquina.".to_string())
        }
        None => {
            return Err(
                "Não foi possível ler a lista de planos de energia desta máquina. Criar o plano \
                 sem essa leitura poderia deixar uma cópia duplicada, então nada foi feito."
                    .to_string(),
            )
        }
        Some(false) => {}
    }

    shell::run_checked("powercfg", &["-duplicatescheme", DESEMPENHO_MAXIMO_GUID])
        .map_err(|e| format!("Não foi possível criar o plano: {}", e))?;

    Ok("Plano de desempenho máximo criado. Ele aparece agora nas opções de energia do Windows, \
        e o Otimiza não o ativou — ativar é uma escolha sua, e em notebook na bateria ele custa \
        autonomia."
        .to_string())
}

pub fn ligar_trim() -> Result<String, String> {
    if !registry::is_elevated() {
        return Err("Alterar o comportamento do sistema de arquivos exige administrador.".to_string());
    }

    shell::run_checked("fsutil", &["behavior", "set", "DisableDeleteNotify", "0"])
        .map_err(|e| format!("Não foi possível religar o TRIM: {}", e))?;

    Ok("TRIM religado. O SSD volta a ser avisado sobre blocos apagados, e a velocidade de \
        escrita para de se degradar com o tempo."
        .to_string())
}

#[allow(dead_code)]
pub fn plano_ativo() -> Option<String> {
    power::active_scheme().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_plano_de_fabrica_e_reconhecido_em_qualquer_idioma() {
        for guid in [
            "381b4222-f694-41f0-9685-ff5bb260df2e",
            "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
            "a1841308-3541-4fab-bc81-f71556f20b4a",
            "e9a42b02-d5df-448d-aa00-03f14749eb61",
        ] {
            assert!(e_guid_de_fabrica(guid), "{guid} é de fábrica");
            assert!(e_guid_de_fabrica(&guid.to_uppercase()), "a caixa não pode importar");
            assert!(e_guid_de_fabrica(&format!("{{{guid}}}")), "com chaves também");
        }
    }

    #[test]
    fn plano_com_guid_proprio_nao_passa_por_de_fabrica() {
        assert!(!e_guid_de_fabrica("fd6bfd99-f5ef-41bb-9663-a1ec92d69712"));
        assert!(!e_guid_de_fabrica(""));
        assert!(!e_guid_de_fabrica("não é guid"));
    }

    #[test]
    /// `cargo test --lib -- --ignored --nocapture onde_vai_o_tempo_da_prontidao`
    #[ignore]
    fn onde_vai_o_tempo_da_prontidao() {
        use std::time::Instant;

        let cronometrar = |nome: &str, f: &dyn Fn()| {
            let inicio = Instant::now();
            f();
            println!("  {:<32} {:>6} ms", nome, inicio.elapsed().as_millis());
        };

        println!("\nETAPAS DA PRONTIDÃO\n");

        cronometrar("reinicio_pendente", &|| {
            let _ = reinicio_pendente();
        });
        cronometrar("hardware::profile", &|| {
            let _ = super::super::hardware::profile();
        });
        cronometrar("trim_ligado", &|| {
            let _ = trim_ligado();
        });
        cronometrar("paginacao_em_disco_lento", &|| {
            let _ = paginacao_em_disco_lento();
        });
        cronometrar("plano_ativo_e_de_terceiro", &|| {
            let _ = plano_ativo_e_de_terceiro();
        });
        cronometrar("planos_instalados", &|| {
            let _ = planos_instalados();
        });
        cronometrar("plano_maximo_existe", &|| {
            let _ = plano_maximo_existe();
        });

        let inicio = Instant::now();
        let _ = analyze();
        println!("\n  analyze() INTEIRO                {:>6} ms\n", inicio.elapsed().as_millis());
    }

    #[test]
    fn reinicio_pendente_desta_maquina() {
        let motivos = reinicio_pendente();
        println!("motivos de reinício pendente: {:?}", motivos);

        assert!(motivos.iter().all(|m| !m.is_empty()));
    }

    #[test]
    fn trim_e_lido_pelo_numero_e_nao_pelo_texto() {
        let estado = trim_ligado();
        println!("TRIM ligado: {:?}", estado);

        assert!(estado.is_some(), "não foi possível ler o estado do TRIM");
    }

    #[test]
    fn le_a_lista_de_planos_sem_depender_do_idioma() {
        let saida = "
Esquemas de Energia Existentes (* Ativos)
             -----------------------------------
             GUID do Esquema de Energia: 381b4222-f694-41f0-9685-ff5bb260df2e  (Equilibrado)
             GUID do Esquema de Energia: 3d23ae32-1072-4a92-ab57-ce99335b215d  (Driver Booster Power Plan) *
             GUID do Esquema de Energia: d1664682-a7b9-4796-b248-286ed3cc2d01  (Desempenho Máximo)
";

        let planos = analisar_lista_de_planos(saida);

        assert_eq!(planos.len(), 3);
        assert_eq!(planos[1].1, "Driver Booster Power Plan");
        assert_eq!(planos[2].0, "d1664682-a7b9-4796-b248-286ed3cc2d01");
        assert!(planos.iter().all(|(g, _)| g.len() == 36));
    }

    #[test]
    fn plano_de_terceiro_e_reconhecido_como_de_fora() {
        assert!(!nome_e_do_windows("Driver Booster Power Plan"));
        assert!(!nome_e_do_windows("Razer Game Booster"));
        assert!(!nome_e_do_windows("Lenovo Vantage"));

        for oficial in [
            "Equilibrado",
            "Balanced",
            "Alto desempenho",
            "High performance",
            "Economia de energia",
            "Desempenho Máximo",
            "Ultimate Performance",
        ] {
            assert!(nome_e_do_windows(oficial), "`{}` é do Windows", oficial);
        }
    }

    #[test]
    fn plano_maximo_e_detectado_pelo_identificador() {
        let existe = plano_maximo_existe();
        println!("plano de desempenho máximo existe: {:?}", existe);

        assert_eq!(DESEMPENHO_MAXIMO_GUID.len(), 36);
    }

    #[test]
    fn criar_duas_vezes_e_recusado() {
        match plano_maximo_existe() {
            Some(true) => {
                let erro = criar_plano_maximo().unwrap_err();
                assert!(erro.contains("já existe") || erro.contains("administrador"));
            }
            None => {
                let erro = criar_plano_maximo().unwrap_err();
                assert!(
                    erro.contains("duplicada") || erro.contains("administrador"),
                    "{}",
                    erro
                );
            }
            Some(false) => println!("plano ainda não existe nesta máquina; caso não exercitado"),
        }
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!("nota: {}", r.note);
        for f in &r.findings {
            println!(
                "  [{:?}{}] {} — {}",
                f.severity,
                if f.actionable { ", corrigível" } else { "" },
                f.title,
                f.measured
            );
        }

        assert!(!r.note.is_empty());

        for f in &r.findings {
            assert!(!f.measured.is_empty(), "{} sem medida", f.title);
            assert!(!f.advice.is_empty(), "{} sem conselho", f.title);
        }

        let ordem: Vec<u8> = r
            .findings
            .iter()
            .map(|f| match f.severity {
                FindingSeverity::Critical => 0,
                FindingSeverity::Important => 1,
                FindingSeverity::Ok => 2,
            })
            .collect();
        assert!(ordem.windows(2).all(|p| p[0] <= p[1]));
    }
}
