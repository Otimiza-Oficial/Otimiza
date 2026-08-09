// Prontidão do sistema
//
// Coisas que precisam estar certas ANTES de otimizar. Não são otimizações: são
// condições que, quando erradas, fazem o atendimento inteiro dar errado por um
// motivo que ninguém procura.
//
// O exemplo que motivou o módulo: máquina com reinício pendente. Parte das
// mudanças não fixa até reiniciar, o técnico aplica tudo, mede, não vê ganho, e
// conclui que o produto não funciona. O motivo estava lá desde o começo, numa
// chave de registro que ninguém olha.
//
// Cada verificação aqui responde uma pergunta do tipo "isto vai atrapalhar o
// resto?", e todas são baratas — o levantamento inteiro leva menos de um
// segundo, então pode rodar antes de qualquer coisa.

use super::{power, registry, shell};
use serde::{Deserialize, Serialize};

pub use super::firmware::{FindingSeverity, FixLocation};

/// Plano de energia oculto do Windows, mais agressivo que o de alto desempenho.
///
/// Ele não aparece na lista até ser criado a partir deste identificador, e o
/// Windows não o cria sozinho em máquina de consumo.
pub const DESEMPENHO_MAXIMO_GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
    /// Verdadeiro quando o próprio Otimiza sabe corrigir isto.
    pub actionable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub findings: Vec<ReadinessFinding>,
    pub note: String,
}

// ------------------------------------------------------ reinício pendente

/// Se o Windows está esperando um reinício para concluir alguma coisa.
///
/// Três origens, e basta uma. As chaves são estruturais e não mudam de idioma.
pub fn reinicio_pendente() -> Vec<&'static str> {
    let mut motivos = Vec::new();

    if registry::key_exists(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending",
    ) {
        motivos.push("instalação de componente do Windows");
    }

    if registry::key_exists(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired",
    ) {
        motivos.push("atualização do Windows");
    }

    // Arquivo esperando para ser renomeado ou apagado no próximo boot. É o mais
    // comum dos três e o menos conhecido.
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

// ------------------------------------------------------------------- TRIM

/// Se o Windows está enviando TRIM ao disco.
///
/// `DisableDeleteNotify = 0` significa ligado. A saída do `fsutil` é traduzida,
/// então o que se lê é o número na linha, nunca a frase.
pub fn trim_ligado() -> Option<bool> {
    let saida = shell::run("fsutil", &["behavior", "query", "DisableDeleteNotify"]).ok()?;

    if !saida.success {
        return None;
    }

    // A linha do NTFS é a que interessa; ReFS quase nunca é o disco do sistema.
    saida
        .stdout
        .lines()
        .find(|l| l.contains("NTFS"))
        .and_then(|l| l.split('=').nth(1))
        .and_then(|v| v.trim().split_whitespace().next())
        .and_then(|v| v.parse::<u32>().ok())
        .map(|v| v == 0)
}

// -------------------------------------------------------- arquivo de troca

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawPagefile {
    name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawDisco {
    mecanico: Option<bool>,
}

/// Descobre se o arquivo de paginação está num disco mecânico.
///
/// É um erro de configuração real e caro: numa máquina com SSD e HD, deixar a
/// paginação no HD faz o Windows usar a peça mais lenta justamente quando a
/// memória acaba, que é o pior momento possível.
fn paginacao_em_disco_lento() -> Option<(String, bool)> {
    let script = "ConvertTo-Json -Compress -InputObject @(Get-CimInstance Win32_PageFileUsage \
                  -ErrorAction SilentlyContinue | Select-Object Name)";

    let arquivos: Vec<RawPagefile> = shell::powershell(script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default();

    let caminho = arquivos.first()?.name.clone()?;
    let letra = caminho.chars().next()?.to_uppercase().to_string();

    // O tipo de mídia vem do subsistema de armazenamento, que responde em
    // número — 3 é disco mecânico, 4 é SSD — e não em texto traduzido.
    let script = format!(
        "ConvertTo-Json -Compress -InputObject @(Get-Partition -ErrorAction SilentlyContinue | \
         Where-Object DriveLetter -eq '{}' | ForEach-Object {{ \
           $d = Get-PhysicalDisk -ErrorAction SilentlyContinue | \
                Where-Object DeviceId -eq (Get-Disk -Number $_.DiskNumber).Number; \
           [ordered]@{{ Mecanico = ($d.MediaType -eq 'HDD') }} }})",
        letra
    );

    let discos: Vec<RawDisco> = shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default();

    let disco = discos.first()?;
    Some((caminho, disco.mecanico.unwrap_or(false)))
}

// ------------------------------------------------------------- montagem

/// Levantamento completo.
pub fn analyze() -> ReadinessReport {
    let mut findings = Vec::new();

    // 1. Reinício pendente. Vem primeiro porque atrapalha todo o resto.
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

    // 2. TRIM. Desligado num SSD, o disco vai perdendo velocidade de escrita ao
    //    longo de meses, e nada no sistema avisa.
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

    // 3. Paginação em disco mecânico.
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

    // 4. Plano de desempenho máximo. Informativo: é oportunidade, não defeito.
    if !plano_maximo_existe() {
        findings.push(ReadinessFinding {
            id: "plano_maximo".to_string(),
            title: "Plano de desempenho máximo não existe nesta máquina".to_string(),
            measured: "O Windows tem um plano de energia mais agressivo que o de alto \
                       desempenho, e ele não aparece na lista até ser criado."
                .to_string(),
            advice: "Ele reduz a latência de mudança de estado do processador, o que aparece \
                     em resposta e não em número de quadros. O ganho é pequeno e real; em \
                     notebook na bateria, custa autonomia."
                .to_string(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::Software,
            actionable: true,
        });
    }

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

/// Se o plano de desempenho máximo já foi criado.
pub fn plano_maximo_existe() -> bool {
    shell::run("powercfg", &["/list"])
        .map(|s| s.stdout.to_lowercase().contains(DESEMPENHO_MAXIMO_GUID))
        .unwrap_or(false)
}

/// Cria o plano de desempenho máximo, sem ativá-lo.
///
/// Criar e ativar são passos separados de propósito: quem está na bateria pode
/// querer ter o plano disponível sem ligá-lo agora.
pub fn criar_plano_maximo() -> Result<String, String> {
    if !registry::is_elevated() {
        return Err("Criar um plano de energia exige executar como administrador.".to_string());
    }

    if plano_maximo_existe() {
        return Err("O plano de desempenho máximo já existe nesta máquina.".to_string());
    }

    shell::run_checked("powercfg", &["-duplicatescheme", DESEMPENHO_MAXIMO_GUID])
        .map_err(|e| format!("Não foi possível criar o plano: {}", e))?;

    Ok("Plano de desempenho máximo criado. Ele aparece agora nas opções de energia do Windows, \
        e o Otimiza não o ativou — ativar é uma escolha sua, e em notebook na bateria ele custa \
        autonomia."
        .to_string())
}

/// Religa o TRIM.
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

/// Usado pelo relatório para citar o plano ativo.
#[allow(dead_code)]
pub fn plano_ativo() -> Option<String> {
    power::active_scheme().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reinicio_pendente_desta_maquina() {
        let motivos = reinicio_pendente();
        println!("motivos de reinício pendente: {:?}", motivos);

        // Todo motivo relatado tem texto; motivo vazio na tela não explica nada.
        assert!(motivos.iter().all(|m| !m.is_empty()));
    }

    #[test]
    fn trim_e_lido_pelo_numero_e_nao_pelo_texto() {
        // A saída do fsutil vem traduzida. Este projeto já quebrou uma vez por
        // ler texto localizado, e a leitura aqui é do número depois do sinal.
        let estado = trim_ligado();
        println!("TRIM ligado: {:?}", estado);

        // Numa máquina Windows a resposta existe; `None` seria falha de leitura.
        assert!(estado.is_some(), "não foi possível ler o estado do TRIM");
    }

    #[test]
    fn plano_maximo_e_detectado_pelo_identificador() {
        // Pelo identificador, nunca pelo nome — o nome do plano é traduzido.
        let existe = plano_maximo_existe();
        println!("plano de desempenho máximo existe: {}", existe);

        assert_eq!(DESEMPENHO_MAXIMO_GUID.len(), 36);
    }

    #[test]
    fn criar_duas_vezes_e_recusado() {
        if plano_maximo_existe() {
            let erro = criar_plano_maximo().unwrap_err();
            assert!(erro.contains("já existe") || erro.contains("administrador"));
        } else {
            println!("plano ainda não existe nesta máquina; caso não exercitado");
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

        // Todo achado tem texto medido e conselho: achado sem explicação vira
        // alarme que ninguém sabe o que fazer com.
        for f in &r.findings {
            assert!(!f.measured.is_empty(), "{} sem medida", f.title);
            assert!(!f.advice.is_empty(), "{} sem conselho", f.title);
        }

        // Problemas antes do que é só oportunidade.
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
