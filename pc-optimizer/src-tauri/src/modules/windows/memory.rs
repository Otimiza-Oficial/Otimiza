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
        crate::modules::monitor::uptime_hours(),
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

/// Metade da RAM física já paginada em algum momento significa que a demanda
/// real da máquina passou de uma vez e meia o que ela tem instalado. Abaixo
/// disso o pico é rotina do Windows e não prova nada.
const PICO_QUE_PROVA_ESGOTAMENTO: f64 = 0.5;

/// Antes disto, um pico pequeno não é boa notícia — é falta de observação.
const HORAS_PARA_OBSERVAR: f64 = 1.0;

/// Regras de diagnóstico, separadas da leitura para poderem ser testadas sem
/// depender da máquina em que rodam.
pub fn diagnosticar(
    total_ram_gb: f64,
    pagefile_gb: f64,
    peak_gb: f64,
    automatico: bool,
    committed_gb: f64,
    uptime_horas: f64,
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

    // --- a máquina JÁ esgotou memória desde que ligou ---
    //
    // Este é o achado que faltava, e o motivo de o produto ter dito "sem
    // problemas" para uma máquina que travava. `over_committed`, logo abaixo, é
    // uma foto do instante do clique: com o jogo fechado ele não dispara, e é
    // justamente com o jogo fechado que o cliente abre o Otimiza.
    //
    // O pico de paginação, ao contrário, é marca d'água — o maior uso desde o
    // boot. Ele registra o travamento de ontem à noite mesmo com o PC calmo
    // agora. O dado já era lido e serializado; só era comparado ao tamanho do
    // arquivo de paginação, nunca à RAM física, que é onde mora o sinal.
    let pico_prova_esgotamento =
        total_ram_gb > 0.0 && peak_gb >= total_ram_gb * PICO_QUE_PROVA_ESGOTAMENTO;

    if pico_prova_esgotamento {
        findings.push(MemoryFinding {
            id: "memoria_esgotada_historico".to_string(),
            title: "Esta máquina já ficou sem memória".to_string(),
            measured: format!(
                "Pico de {:.1} GB de paginação desde que o PC ligou, com {:.1} GB de \
                 memória física: a demanda chegou a cerca de {:.1} GB.",
                peak_gb,
                total_ram_gb,
                total_ram_gb + peak_gb
            ),
            advice: if pouca_ram {
                "Quando a memória acaba, não é só o jogo que trava: tudo para junto \
                 esperando o disco — o jogo, o navegador, o programa de voz, o Windows. \
                 É esta a causa de \"o PC inteiro congela\", e nenhum ajuste de software \
                 resolve, porque não existe ajuste que crie memória. Fechar programas \
                 durante o jogo alivia; acrescentar um pente resolve."
                    .to_string()
            } else {
                "A máquina tem memória folgada, então isto foi provavelmente um pico de \
                 uso pesado e não a rotina. Vale acompanhar: se voltar a acontecer com \
                 frequência, o caminho é reduzir o que roda junto, não ajustar o Windows."
                    .to_string()
            },
            severity: if pouca_ram {
                FindingSeverity::Critical
            } else {
                FindingSeverity::Important
            },
            fix_location: FixLocation::Hardware,
        });
    } else if uptime_horas < HORAS_PARA_OBSERVAR {
        // Pico pequeno com o PC recém-ligado não é boa notícia: é ausência de
        // informação. Dizer "sem problemas" aqui seria inventar um resultado.
        findings.push(MemoryFinding {
            id: "memoria_sem_observacao".to_string(),
            title: "Ainda não deu para observar o uso de memória".to_string(),
            measured: format!(
                "PC ligado há {:.0} minutos. O pico de memória é contado desde o boot e \
                 zera a cada reinício.",
                uptime_horas * 60.0
            ),
            advice: "Deixe o PC ligado durante o uso normal — inclusive jogando — e volte \
                     aqui depois. Só assim dá para afirmar se falta memória nesta máquina."
                .to_string(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::None,
        });
    }

    // --- memória prometida acima da física ---
    //
    // Corroboração, nunca a única chance de detectar o problema: ver o achado
    // histórico acima.
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
    //
    // O limiar é o `pouca_ram` do topo desta função. Até a versão 0.12 esta
    // regra escrevia `< 6.0` à mão, e o mesmo arquivo passava a chamar uma
    // máquina de 8 GB de "pouca RAM" numa regra e de "confortável" na outra —
    // com o resultado de a tela dizer "memória sem problemas" para o PC que
    // travava. Um limiar só, com nome, para não divergir de novo.
    if total_ram_gb > 0.0 && pouca_ram {
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
        let f = diagnosticar(4.0, 0.0, 0.0, false, 3.0, 5.0);

        assert!(tem(&f, "pagefile_off"));
        let achado = f.iter().find(|f| f.id == "pagefile_off").unwrap();
        assert_eq!(achado.severity, FindingSeverity::Critical);
        assert!(achado.advice.contains("fecharem sozinhos"));
    }

    #[test]
    fn paginacao_desligada_com_muita_ram_e_so_importante() {
        // Com 32 GB o risco existe, mas não é a mesma emergência.
        let f = diagnosticar(32.0, 0.0, 0.0, false, 10.0, 5.0);
        let achado = f.iter().find(|f| f.id == "pagefile_off").unwrap();
        assert_eq!(achado.severity, FindingSeverity::Important);
    }

    #[test]
    fn paginacao_gerenciada_pelo_windows_nao_vira_problema() {
        let f = diagnosticar(16.0, 4.0, 0.5, true, 8.0, 5.0);
        assert!(tem(&f, "pagefile_ok"));
        assert!(!tem(&f, "pagefile_manual"));
    }

    #[test]
    fn paginacao_que_quase_encheu_e_apontada() {
        // Chegou a 3,6 de 4 GB: faltou pouco para o programa fechar sozinho.
        let f = diagnosticar(8.0, 4.0, 3.6, true, 7.0, 5.0);
        assert!(tem(&f, "pagefile_small"));
    }

    #[test]
    fn tamanho_fixo_e_apontado_como_risco() {
        let f = diagnosticar(16.0, 2.0, 0.2, false, 8.0, 5.0);
        assert!(tem(&f, "pagefile_manual"));
    }

    #[test]
    fn memoria_prometida_acima_da_fisica_aponta_para_hardware() {
        let f = diagnosticar(8.0, 8.0, 1.0, true, 11.0, 5.0);
        let achado = f.iter().find(|f| f.id == "over_committed").unwrap();

        assert_eq!(achado.severity, FindingSeverity::Critical);
        // Software nenhum cria memória: o diagnóstico precisa dizer isso.
        assert_eq!(achado.fix_location, FixLocation::Hardware);
    }

    #[test]
    fn pouca_ram_nao_promete_solucao_por_software() {
        let f = diagnosticar(4.0, 4.0, 1.0, true, 3.0, 5.0);
        let achado = f.iter().find(|f| f.id == "low_ram").unwrap();

        assert_eq!(achado.fix_location, FixLocation::Hardware);
        assert!(achado.advice.contains("Nenhum ajuste de software cria memória"));
    }

    /// O PC que o produto reprovou.
    ///
    /// Números medidos na máquina do dono em 12/08/2026, com o jogo FECHADO:
    /// 7,9 GB de RAM num único pente, 9,5 GB prometidos a programas, pico de
    /// 8,6 GB de paginação desde o boot, paginação automática. O Windows já
    /// tinha registrado esgotamento de memória no evento 2004 e o FiveM já
    /// tinha parado de responder no evento 1002.
    ///
    /// E o Otimiza mostrava "Memória e paginação sem problemas", porque
    /// `low_ram` exigia menos de 6 GB e `over_committed` só olhava o instante
    /// do clique. Este teste existe para que isso não volte.
    #[test]
    fn a_maquina_que_travava_nao_pode_mais_passar_como_saudavel() {
        let f = diagnosticar(7.9, 9.0, 8.6, true, 9.5, 30.0);

        let critico: Vec<&MemoryFinding> = f
            .iter()
            .filter(|f| f.severity == FindingSeverity::Critical)
            .collect();
        assert!(
            !critico.is_empty(),
            "máquina de 7,9 GB com pico de 8,6 GB de paginação não pode sair sem achado crítico"
        );

        // O achado histórico é o que precisa disparar: é o único que não depende
        // de o jogo estar aberto na hora do clique.
        let historico = f
            .iter()
            .find(|f| f.id == "memoria_esgotada_historico")
            .expect("pico de paginação acima de metade da RAM tem que virar achado");
        assert_eq!(historico.severity, FindingSeverity::Critical);
        assert_eq!(historico.fix_location, FixLocation::Hardware);

        // E a máquina de 8 GB precisa ser reconhecida como pouca memória.
        assert!(tem(&f, "low_ram"), "8 GB não pode mais passar por confortável");
        assert!(!tem(&f, "memoria_sem_observacao"));
    }

    #[test]
    fn pc_recem_ligado_admite_que_nao_sabe_em_vez_de_aprovar() {
        // Cinco minutos de ligado, pico baixo. Não é "sem problemas" — é cedo
        // demais para afirmar qualquer coisa.
        let f = diagnosticar(7.9, 9.0, 0.2, true, 5.0, 0.08);

        let lacuna = f
            .iter()
            .find(|f| f.id == "memoria_sem_observacao")
            .expect("pico baixo com PC recém-ligado tem que virar falta de observação");
        assert!(lacuna.measured.contains("zera a cada reinício"));
        assert!(!tem(&f, "memoria_esgotada_historico"));
    }

    #[test]
    fn maquina_folgada_com_pico_alto_nao_vira_emergencia() {
        // 32 GB com pico de 20 GB: aconteceu, mas quem tem essa memória
        // aguenta. Apontar como crítico seria inventar urgência.
        let f = diagnosticar(32.0, 16.0, 20.0, true, 18.0, 30.0);
        let historico = f.iter().find(|f| f.id == "memoria_esgotada_historico").unwrap();

        assert_eq!(historico.severity, FindingSeverity::Important);
        assert!(!tem(&f, "low_ram"));
    }

    #[test]
    fn problemas_vem_antes_do_que_esta_certo() {
        let f = diagnosticar(4.0, 0.0, 0.0, false, 6.0, 5.0);
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
