// Memória e arquivo de paginação
//
// Em PC de 4 a 8 GB, a maior parte dos travamentos que o dono descreve como "o
// PC congela" não é falta de processador: é memória acabando. E o culpado mais
// comum é o arquivo de paginação mal configurado — quase sempre porque alguém
// seguiu um tutorial que mandava desativá-lo "para ganhar desempenho".
//
// Desativar a paginação num PC com pouca RAM não ganha desempenho: faz programa
// fechar sozinho com erro de memória. Este módulo detecta isso e explica.

use super::shell;
use serde::{Deserialize, Serialize};

// Um vocabulário só para achados no produto inteiro: severidade e onde se
// resolve são as mesmas do diagnóstico de firmware.
pub use super::firmware::{FindingSeverity, FixLocation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReport {
    pub total_ram_gb: f64,
    pub available_ram_gb: f64,
    /// Memória prometida a programas. Passar da RAM física significa que o PC
    /// só se sustenta paginando para o disco.
    pub committed_gb: f64,
    /// Se o Windows gerencia o arquivo de paginação sozinho.
    pub pagefile_automatic: bool,
    pub pagefile_size_gb: f64,
    /// Maior uso de paginação desde que o PC ligou.
    pub pagefile_peak_gb: f64,
    pub pagefile_location: String,
    pub findings: Vec<MemoryFinding>,
}

fn powershell(script: &str) -> Option<String> {
    let output = shell::powershell(script).ok()?;

    if output.success && !output.stdout.trim().is_empty() {
        Some(output.stdout.trim().to_string())
    } else {
        None
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawMemory {
    total_visible_kb: Option<u64>,
    free_physical_kb: Option<u64>,
    total_virtual_kb: Option<u64>,
    free_virtual_kb: Option<u64>,
    automatic: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawPagefile {
    name: Option<String>,
    allocated_base_size_mb: Option<u64>,
    peak_usage_mb: Option<u64>,
}

fn ler_memoria() -> RawMemory {
    // Um único JSON evita três chamadas de PowerShell, que custam centenas de
    // milissegundos cada. Os nomes das propriedades do WMI são estáveis em
    // qualquer idioma do Windows.
    let script = "$os = Get-CimInstance Win32_OperatingSystem; \
                  $cs = Get-CimInstance Win32_ComputerSystem; \
                  ConvertTo-Json -Compress -InputObject ([ordered]@{ \
                    TotalVisibleKb = $os.TotalVisibleMemorySize; \
                    FreePhysicalKb = $os.FreePhysicalMemory; \
                    TotalVirtualKb = $os.TotalVirtualMemorySize; \
                    FreeVirtualKb  = $os.FreeVirtualMemory; \
                    Automatic      = $cs.AutomaticManagedPagefile })";

    powershell(script)
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn ler_paginacao() -> Option<RawPagefile> {
    let script = "$p = Get-CimInstance Win32_PageFileUsage | Select-Object -First 1; \
                  if ($p) { ConvertTo-Json -Compress -InputObject ([ordered]@{ \
                    Name = $p.Name; \
                    AllocatedBaseSizeMb = $p.AllocatedBaseSize; \
                    PeakUsageMb = $p.PeakUsage }) }";

    powershell(script).and_then(|json| serde_json::from_str(&json).ok())
}

const KB_EM_GB: f64 = 1_048_576.0;
const MB_EM_GB: f64 = 1024.0;

/// Analisa memória e paginação, e explica o que estiver errado.
pub fn analyze() -> MemoryReport {
    let bruto = ler_memoria();
    let paginacao = ler_paginacao();

    let total_ram_gb = bruto.total_visible_kb.unwrap_or(0) as f64 / KB_EM_GB;
    let available_ram_gb = bruto.free_physical_kb.unwrap_or(0) as f64 / KB_EM_GB;

    // Memória prometida = total virtual menos o que sobra dele.
    let committed_gb = (bruto.total_virtual_kb.unwrap_or(0) as f64
        - bruto.free_virtual_kb.unwrap_or(0) as f64)
        / KB_EM_GB;

    let pagefile_size_gb = paginacao
        .as_ref()
        .and_then(|p| p.allocated_base_size_mb)
        .unwrap_or(0) as f64
        / MB_EM_GB;

    let pagefile_peak_gb = paginacao
        .as_ref()
        .and_then(|p| p.peak_usage_mb)
        .unwrap_or(0) as f64
        / MB_EM_GB;

    let pagefile_location = paginacao
        .as_ref()
        .and_then(|p| p.name.clone())
        .unwrap_or_else(|| "não encontrado".to_string());

    let pagefile_automatic = bruto.automatic.unwrap_or(false);

    let findings = diagnosticar(
        total_ram_gb,
        pagefile_size_gb,
        pagefile_peak_gb,
        pagefile_automatic,
        committed_gb,
    );

    MemoryReport {
        total_ram_gb,
        available_ram_gb,
        committed_gb,
        pagefile_automatic,
        pagefile_size_gb,
        pagefile_peak_gb,
        pagefile_location,
        findings,
    }
}

/// Regras de diagnóstico, separadas da leitura para poderem ser testadas sem
/// depender da máquina em que rodam.
pub fn diagnosticar(
    total_ram_gb: f64,
    pagefile_gb: f64,
    peak_gb: f64,
    automatico: bool,
    committed_gb: f64,
) -> Vec<MemoryFinding> {
    let mut findings = Vec::new();
    let pouca_ram = total_ram_gb <= 8.5;

    // --- paginação desligada ---
    if pagefile_gb <= 0.01 {
        let (severidade, conselho) = if pouca_ram {
            (
                FindingSeverity::Critical,
                "Com esta quantidade de memória, o arquivo de paginação é o que impede \
                 programas de fecharem sozinhos quando a RAM acaba. Desativá-lo não ganha \
                 desempenho — é a causa mais comum de \"o programa fechou sozinho\" e de \
                 travamentos em jogo. Ligue de volta em Configurações Avançadas do Sistema, \
                 ou pelo botão desta tela."
                    .to_string(),
            )
        } else {
            (
                FindingSeverity::Important,
                "Sua memória é folgada, então o risco é menor, mas alguns programas exigem \
                 arquivo de paginação para abrir. Recomendamos deixar o Windows gerenciar."
                    .to_string(),
            )
        };

        findings.push(MemoryFinding {
            id: "pagefile_off".to_string(),
            title: "Arquivo de paginação desativado".to_string(),
            measured: format!("Nenhuma paginação configurada, com {:.1} GB de RAM.", total_ram_gb),
            advice: conselho,
            severity: severidade,
            fix_location: FixLocation::Software,
        });
    } else {
        // --- paginação pequena demais para o que já foi usado ---
        // Ter chegado a 80% do arquivo significa que faltou pouco para acabar.
        if peak_gb > 0.0 && peak_gb >= pagefile_gb * 0.8 {
            findings.push(MemoryFinding {
                id: "pagefile_small".to_string(),
                title: "Arquivo de paginação perto do limite".to_string(),
                measured: format!(
                    "Já chegou a usar {:.1} GB de {:.1} GB disponíveis.",
                    peak_gb, pagefile_gb
                ),
                advice: "Este PC já esteve perto de ficar sem memória. Deixar o Windows \
                         gerenciar o tamanho evita que um programa feche sozinho quando isso \
                         acontecer de novo."
                    .to_string(),
                severity: FindingSeverity::Important,
                fix_location: FixLocation::Software,
            });
        } else if !automatico {
            findings.push(MemoryFinding {
                id: "pagefile_manual".to_string(),
                title: "Arquivo de paginação com tamanho fixo".to_string(),
                measured: format!("{:.1} GB fixos, definidos manualmente.", pagefile_gb),
                advice: "Tamanho fixo só ajuda quando alguém mediu a necessidade real da \
                         máquina. Na dúvida, deixar o Windows gerenciar é mais seguro: ele \
                         cresce quando falta e devolve o espaço quando sobra."
                    .to_string(),
                severity: FindingSeverity::Important,
                fix_location: FixLocation::Software,
            });
        } else {
            findings.push(MemoryFinding {
                id: "pagefile_ok".to_string(),
                title: "Arquivo de paginação bem configurado".to_string(),
                measured: format!("{:.1} GB, gerenciado pelo Windows.", pagefile_gb),
                advice: String::new(),
                severity: FindingSeverity::Ok,
                fix_location: FixLocation::None,
            });
        }
    }

    // --- memória prometida acima da física ---
    if committed_gb > total_ram_gb && total_ram_gb > 0.0 {
        findings.push(MemoryFinding {
            id: "over_committed".to_string(),
            title: "Programas pedindo mais memória do que existe".to_string(),
            measured: format!(
                "{:.1} GB prometidos para {:.1} GB de memória física.",
                committed_gb, total_ram_gb
            ),
            advice: "O PC está se sustentando com disco no lugar de memória, e disco é \
                     ordens de grandeza mais lento. Feche o que não estiver usando, reveja \
                     os programas de inicialização — e, se isso for rotina, acrescentar \
                     memória resolve mais que qualquer ajuste de software."
                .to_string(),
            severity: FindingSeverity::Critical,
            fix_location: FixLocation::Hardware,
        });
    }

    // --- pouca RAM para o Windows atual ---
    if total_ram_gb > 0.0 && total_ram_gb < 6.0 {
        findings.push(MemoryFinding {
            id: "low_ram".to_string(),
            title: "Memória abaixo do confortável para o Windows".to_string(),
            measured: format!("{:.1} GB de memória física.", total_ram_gb),
            advice: "Nenhum ajuste de software cria memória. Dá para aliviar bastante — \
                     efeitos visuais, programas de inicialização, aplicativos em segundo \
                     plano — e o Otimiza marca justamente esses como os que pesam nesta \
                     máquina. Mas o salto de verdade vem de acrescentar um pente."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Hardware,
        });
    }

    findings.sort_by_key(|f| match f.severity {
        FindingSeverity::Critical => 0,
        FindingSeverity::Important => 1,
        FindingSeverity::Ok => 2,
    });

    findings
}

/// Devolve o gerenciamento do arquivo de paginação ao Windows.
///
/// É a correção certa em quase todo caso: o Windows cresce o arquivo quando
/// falta e devolve o espaço quando sobra. Exige reiniciar para valer.
pub fn set_automatic_pagefile() -> Result<String, String> {
    if !super::registry::is_elevated() {
        return Err("Alterar o arquivo de paginação exige executar como administrador.".to_string());
    }

    let script = "$cs = Get-CimInstance Win32_ComputerSystem; \
                  if (-not $cs.AutomaticManagedPagefile) { \
                    Set-CimInstance -InputObject $cs -Property @{ AutomaticManagedPagefile = $true } \
                  }";

    shell::powershell_checked(script)
        .map_err(|e| format!("Não foi possível alterar o arquivo de paginação: {}", e))?;

    Ok("O Windows voltou a gerenciar o arquivo de paginação. Reinicie o PC para valer.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tem(findings: &[MemoryFinding], id: &str) -> bool {
        findings.iter().any(|f| f.id == id)
    }

    #[test]
    fn paginacao_desligada_com_pouca_ram_e_critico() {
        // O erro mais comum de tutorial ruim, e o mais caro num PC de 4 GB.
        let f = diagnosticar(4.0, 0.0, 0.0, false, 3.0);

        assert!(tem(&f, "pagefile_off"));
        let achado = f.iter().find(|f| f.id == "pagefile_off").unwrap();
        assert_eq!(achado.severity, FindingSeverity::Critical);
        assert!(achado.advice.contains("fecharem sozinhos"));
    }

    #[test]
    fn paginacao_desligada_com_muita_ram_e_so_importante() {
        // Com 32 GB o risco existe, mas não é a mesma emergência.
        let f = diagnosticar(32.0, 0.0, 0.0, false, 10.0);
        let achado = f.iter().find(|f| f.id == "pagefile_off").unwrap();
        assert_eq!(achado.severity, FindingSeverity::Important);
    }

    #[test]
    fn paginacao_gerenciada_pelo_windows_nao_vira_problema() {
        let f = diagnosticar(16.0, 4.0, 0.5, true, 8.0);
        assert!(tem(&f, "pagefile_ok"));
        assert!(!tem(&f, "pagefile_manual"));
    }

    #[test]
    fn paginacao_que_quase_encheu_e_apontada() {
        // Chegou a 3,6 de 4 GB: faltou pouco para o programa fechar sozinho.
        let f = diagnosticar(8.0, 4.0, 3.6, true, 7.0);
        assert!(tem(&f, "pagefile_small"));
    }

    #[test]
    fn tamanho_fixo_e_apontado_como_risco() {
        let f = diagnosticar(16.0, 2.0, 0.2, false, 8.0);
        assert!(tem(&f, "pagefile_manual"));
    }

    #[test]
    fn memoria_prometida_acima_da_fisica_aponta_para_hardware() {
        let f = diagnosticar(8.0, 8.0, 1.0, true, 11.0);
        let achado = f.iter().find(|f| f.id == "over_committed").unwrap();

        assert_eq!(achado.severity, FindingSeverity::Critical);
        // Software nenhum cria memória: o diagnóstico precisa dizer isso.
        assert_eq!(achado.fix_location, FixLocation::Hardware);
    }

    #[test]
    fn pouca_ram_nao_promete_solucao_por_software() {
        let f = diagnosticar(4.0, 4.0, 1.0, true, 3.0);
        let achado = f.iter().find(|f| f.id == "low_ram").unwrap();

        assert_eq!(achado.fix_location, FixLocation::Hardware);
        assert!(achado.advice.contains("Nenhum ajuste de software cria memória"));
    }

    #[test]
    fn problemas_vem_antes_do_que_esta_certo() {
        let f = diagnosticar(4.0, 0.0, 0.0, false, 6.0);
        let ordem: Vec<u8> = f
            .iter()
            .map(|f| match f.severity {
                FindingSeverity::Critical => 0,
                FindingSeverity::Important => 1,
                FindingSeverity::Ok => 2,
            })
            .collect();

        assert!(ordem.windows(2).all(|p| p[0] <= p[1]));
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();
        println!(
            "RAM {:.1} GB ({:.1} livre) · prometido {:.1} GB · paginação {:.1} GB em {} (auto: {})",
            r.total_ram_gb,
            r.available_ram_gb,
            r.committed_gb,
            r.pagefile_size_gb,
            r.pagefile_location,
            r.pagefile_automatic
        );
        for f in &r.findings {
            println!("  [{:?}] {} — {}", f.severity, f.title, f.measured);
        }

        assert!(r.total_ram_gb > 0.5, "não leu a memória da máquina");
    }
}
