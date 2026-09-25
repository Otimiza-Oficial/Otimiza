// Por que o processador não entrega tudo (notebook empoeirado a 95 graus: nenhum ajuste resolve). Os caminhos
// óbvios erram: `MSAcpi_ThermalZoneTemperature` exige elevação e vem fixo; `CurrentClockSpeed` fica congelado no
// nominal; `% Limite de Desempenho` mistura calor com plano de energia. Elimina causas em ordem e só diz "calor"
// com evento térmico do Windows. Sem falso positivo verificado; a detecção positiva num notebook quente NÃO foi
// verificada, por isso a tela cita data e número do evento.

use super::{power, shell};
use serde::{Deserialize, Serialize};

const SUB_PROCESSADOR: &str = "54533251-82be-4824-96c1-47b60b740d00";
const PROCTHROTTLEMAX: &str = "bc5038f7-23e0-4960-96da-33abaf5935ec";

/// Legível SEM elevação.
const LOG_TERMICO: &str = "Microsoft-Windows-Kernel-Power/Thermal-Operational";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Culprit {
    Nenhum,
    Bateria,
    PlanoDeEnergia,
    /// A única situação em que este módulo diz "calor".
    Calor,
    /// Fonte, bateria degradada, PL1/PL2: não é sujeira no cooler.
    LimiteEletrico,
    NaoIdentificado,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalReport {
    pub culprit: Culprit,
    pub summary: String,
    pub advice: String,
    pub percent_of_max: Option<f64>,
    pub power_cap_percent: Option<u32>,
    pub on_battery: bool,
    pub thermal_events: usize,
    pub last_thermal_event: Option<String>,
    /// Para `suporte.rs` saber da lacuna sem comparar prosa. `false` só quando `NaoSei` virou `Culprit::Nenhum` por
    /// falta de leitura.
    pub medido: bool,
    /// `false` = não se leu, e "nenhum evento" NÃO pode ser afirmado.
    #[serde(default = "sim")]
    pub eventos_lidos: bool,
}

fn sim() -> bool {
    true
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawPerf {
    percentof_maximum_frequency: Option<f64>,
    performance_limit_flags: Option<u64>,
}

/// Pela classe WMI, com nomes em inglês e sem elevação: o contador pelo nome traduzido quebra por idioma.
fn amostrar_contadores() -> Option<RawPerf> {
    let script = "ConvertTo-Json -Compress -InputObject (Get-CimInstance \
                  Win32_PerfFormattedData_Counters_ProcessorInformation \
                  -ErrorAction SilentlyContinue | Where-Object Name -eq '_Total' | \
                  Select-Object -First 1 PercentofMaximumFrequency,PerformanceLimitFlags,\
                  PercentPerformanceLimit)";

    let saida = shell::powershell(script).ok()?;

    if !saida.success || saida.stdout.trim().is_empty() {
        return None;
    }

    serde_json::from_str(&saida.stdout).ok()
}

fn na_bateria() -> bool {
    // 1 descarregando, 2 na tomada. Desktop não tem instância: falso.
    let script = "$b = Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue | \
                  Select-Object -First 1; if ($b) { $b.BatteryStatus } else { 2 }";

    shell::powershell(script)
        .ok()
        .and_then(|s| s.stdout.trim().parse::<u32>().ok())
        .map(|estado| estado == 1)
        .unwrap_or(false)
}

/// Do registro: a saída do `powercfg` vem traduzida.
fn teto_do_plano() -> Option<u32> {
    let esquema = power::active_scheme().ok()?;

    match power::read_power_setting(&esquema, SUB_PROCESSADOR, PROCTHROTTLEMAX) {
        Ok(crate::modules::changelog::PreviousValue::Dword(valor)) => Some(valor),
        // Ausente herda o padrão, 100%.
        _ => None,
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawTermico {
    when: Option<String>,
}

/// O único sinal para "calor": resfriamento passivo ACPI é resposta a temperatura por definição. `None` = não se
/// leu (virava lista vazia e "nada segurando" em verde); vazio = lido, sem evento.
fn eventos_termicos() -> Option<Vec<RawTermico>> {
    let script = format!(
        "try {{ $e = Get-WinEvent -FilterHashtable @{{ LogName='{}'; \
         StartTime=(Get-Date).AddDays(-30) }} -MaxEvents 40 -ErrorAction Stop }} catch {{ if ($_.FullyQualifiedErrorId -like 'NoMatchingEventsFound*') {{ $e = @() }} else {{ throw }} }}; \
         ConvertTo-Json -Compress -Depth 3 -InputObject @($e | ForEach-Object {{ \
           [ordered]@{{ when = $_.TimeCreated.ToString('s') }} }})",
        LOG_TERMICO
    );

    let s = shell::powershell(&script).ok().filter(|s| s.success)?;
    if s.stdout.trim().is_empty() {
        return Some(Vec::new());
    }
    serde_json::from_str(&s.stdout).ok()
}

pub const LIMITE_TERMICO: u64 = 0x1;
pub const LIMITE_ELETRICO: u64 = 0x2;

/// Não é 100: o valor oscila alguns pontos, e colado no teto ruído viraria diagnóstico.
pub const FREQUENCIA_BAIXA: f64 = 90.0;

/// `Some(0)` = medido e sem limite; `None` = não se leu. Com `unwrap_or(0)`, a falha virava "sem throttling",
/// justamente o que este módulo existe para detectar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitesDoProcessador {
    Nenhum,
    Ativos(u64),
    NaoSei,
}

pub fn avaliar_limites(flags: Option<u64>) -> LimitesDoProcessador {
    match flags {
        None => LimitesDoProcessador::NaoSei,
        Some(0) => LimitesDoProcessador::Nenhum,
        Some(f) => LimitesDoProcessador::Ativos(f),
    }
}

/// A ordem é o módulo: das causas mais prováveis e baratas até a mais cara, calor, que exige prova do Windows.
pub fn decidir(
    na_bateria: bool,
    teto: Option<u32>,
    percentual: Option<f64>,
    flags: u64,
    eventos_termicos: usize,
) -> Culprit {
    // 1. Bateria: limitação correta do Windows.
    if na_bateria {
        return Culprit::Bateria;
    }

    // 2. Teto configurado: o mais comum e o mais fácil; explica o número sozinho.
    if teto.is_some_and(|t| t < 100) {
        return Culprit::PlanoDeEnergia;
    }

    // 3. Evento térmico do Windows: a única prova aceita para "calor".
    if eventos_termicos > 0 {
        return Culprit::Calor;
    }

    // 4. O bit térmico só cobre fora da zona ACPI: reforça, não é obrigatório.
    if flags & LIMITE_TERMICO != 0 {
        return Culprit::Calor;
    }

    if flags & LIMITE_ELETRICO != 0 {
        return Culprit::LimiteEletrico;
    }

    // 5. Frequência baixa sem causa conhecida: dizer que não sabe, não o palpite mais vendável.
    match percentual {
        Some(p) if p < FREQUENCIA_BAIXA => Culprit::NaoIdentificado,
        _ => Culprit::Nenhum,
    }
}

pub fn explicar(culprit: Culprit, teto: Option<u32>, quando: Option<&str>) -> (String, String) {
    match culprit {
        Culprit::Nenhum => (
            "O processador está livre para trabalhar na velocidade máxima.".to_string(),
            String::new(),
        ),

        Culprit::Bateria => (
            "A máquina está na bateria, e o Windows reduz o processador de propósito \
             para a carga durar mais."
                .to_string(),
            "Se você quer desempenho agora, ligue na tomada. Isso não é defeito nem \
             sujeira — é o comportamento correto."
                .to_string(),
        ),

        Culprit::PlanoDeEnergia => (
            format!(
                "O plano de energia está limitando o processador a {}% da velocidade.",
                teto.unwrap_or(0)
            ),
            "Este é o caso mais fácil de resolver e o que mais aparece: alguém — muitas \
             vezes um \"otimizador\" instalado antes — deixou um teto configurado. Aplicar \
             o plano de alto desempenho na aba Otimizações devolve a velocidade inteira."
                .to_string(),
        ),

        Culprit::Calor => (
            match quando {
                Some(data) => format!(
                    "O Windows registrou redução de velocidade por temperatura. \
                     Evento mais recente em {}.",
                    data
                ),
                None => "O processador reporta estar sendo limitado por temperatura.".to_string(),
            },
            "Este é um problema físico, e nenhum ajuste de software resolve — é honesto \
             dizer isso antes de você gastar tempo otimizando. A causa quase sempre é \
             poeira no cooler ou pasta térmica ressecada. Limpeza interna costuma devolver \
             o desempenho por completo."
                .to_string(),
        ),

        Culprit::LimiteEletrico => (
            "O processador está sendo limitado por orçamento de energia do hardware."
                .to_string(),
            "Isso não é sujeira no cooler: costuma ser fonte insuficiente, carregador \
             abaixo do que o notebook pede, ou bateria muito gasta. Vale conferir se o \
             carregador é o original antes de qualquer outra coisa."
                .to_string(),
        ),

        Culprit::NaoIdentificado => (
            "O processador está trabalhando abaixo da velocidade máxima e não \
             identificamos o motivo."
                .to_string(),
            "Descartamos bateria, plano de energia e registro térmico do Windows. Preferimos \
             dizer que não sabemos a escolher um culpado provável — o palpite errado aqui \
             faz você trocar peça à toa."
                .to_string(),
        ),
    }
}

/// Extraída para a costura entre `avaliar_limites` e o texto (inclusive `NaoSei`) ser testável: senão voltar a
/// `unwrap_or(0)` passaria calado.
fn montar_relatorio(
    bateria: bool,
    teto: Option<u32>,
    percentual: Option<f64>,
    flags_lidos: Option<u64>,
    eventos_termicos: usize,
    ultimo_evento: Option<String>,
) -> ThermalReport {
    let limites = avaliar_limites(flags_lidos);
    let flags = match limites {
        LimitesDoProcessador::Ativos(f) => f,
        // `Nenhum` e `NaoSei` entram como 0 na decisão por bit; só `Nenhum` é medido, e o texto corrige abaixo.
        LimitesDoProcessador::Nenhum | LimitesDoProcessador::NaoSei => 0,
    };

    let culprit = decidir(bateria, teto, percentual, flags, eventos_termicos);

    let nao_medido = matches!(limites, LimitesDoProcessador::NaoSei) && culprit == Culprit::Nenhum;

    let (summary, advice) = if nao_medido {
        // Chegou a "livre" só porque o bit virou 0 sem leitura: é "não conseguimos checar".
        (
            "Não foi possível medir se o processador está sendo limitado agora.".to_string(),
            "A leitura desse contador falhou ou exige permissão de administrador. Isso não \
             significa que o processador está liberado — significa que não deu para checar."
                .to_string(),
        )
    } else {
        explicar(culprit, teto, ultimo_evento.as_deref())
    };

    ThermalReport {
        culprit,
        summary,
        advice,
        percent_of_max: percentual,
        power_cap_percent: teto,
        on_battery: bateria,
        thermal_events: eventos_termicos,
        last_thermal_event: ultimo_evento,
        medido: !nao_medido,
        eventos_lidos: true,
    }
}

pub fn analyze() -> ThermalReport {
    let bateria = na_bateria();
    let teto = teto_do_plano();
    let contadores = amostrar_contadores().unwrap_or_default();
    let lidos = eventos_termicos();
    let eventos_lidos = lidos.is_some();
    let termicos = lidos.unwrap_or_default();
    let ultimo = termicos.first().and_then(|t| t.when.clone());

    let mut r = montar_relatorio(
        bateria,
        teto,
        contadores.percentof_maximum_frequency,
        contadores.performance_limit_flags,
        termicos.len(),
        ultimo,
    );
    r.eventos_lidos = eventos_lidos;
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bateria_vem_antes_de_tudo() {
        assert_eq!(
            decidir(true, Some(50), Some(40.0), LIMITE_TERMICO, 9),
            Culprit::Bateria
        );
    }

    #[test]
    fn plano_de_energia_vem_antes_de_calor() {
        assert_eq!(
            decidir(false, Some(50), Some(50.0), 0, 0),
            Culprit::PlanoDeEnergia
        );

        assert_eq!(decidir(false, Some(100), Some(100.0), 0, 0), Culprit::Nenhum);
        assert_eq!(decidir(false, None, Some(100.0), 0, 0), Culprit::Nenhum);
    }

    #[test]
    fn calor_exige_prova_do_windows() {
        assert_eq!(
            decidir(false, None, Some(35.0), 0, 0),
            Culprit::NaoIdentificado
        );

        assert_eq!(decidir(false, None, Some(35.0), 0, 3), Culprit::Calor);
        assert_eq!(
            decidir(false, None, Some(35.0), LIMITE_TERMICO, 0),
            Culprit::Calor
        );
    }

    #[test]
    fn limite_eletrico_nao_e_vendido_como_sujeira() {
        assert_eq!(
            decidir(false, None, Some(60.0), LIMITE_ELETRICO, 0),
            Culprit::LimiteEletrico
        );

        let (_, conselho) = explicar(Culprit::LimiteEletrico, None, None);
        assert!(conselho.contains("não é sujeira"));
        assert!(conselho.contains("carregador"));
    }

    #[test]
    fn oscilacao_normal_nao_vira_diagnostico() {
        assert_eq!(decidir(false, None, Some(97.0), 0, 0), Culprit::Nenhum);
        assert_eq!(decidir(false, None, Some(89.0), 0, 0), Culprit::NaoIdentificado);
    }

    #[test]
    fn sem_leitura_nenhuma_nao_acusa_nada() {
        assert_eq!(decidir(false, None, None, 0, 0), Culprit::Nenhum);
    }

    #[test]
    fn texto_de_calor_cita_a_data_do_evento() {
        let (resumo, conselho) = explicar(Culprit::Calor, None, Some("2026-07-20T14:02:11"));

        assert!(resumo.contains("2026-07-20"));
        assert!(resumo.contains("Windows registrou"));
        assert!(conselho.contains("nenhum ajuste de software resolve"));
        assert!(conselho.contains("poeira"));
    }

    #[test]
    fn nao_identificado_admite_em_vez_de_chutar() {
        let (_, conselho) = explicar(Culprit::NaoIdentificado, None, None);

        assert!(conselho.contains("não sabemos"));
        assert!(conselho.contains("trocar peça à toa"));
    }

    #[test]
    fn plano_limitado_diz_a_porcentagem() {
        let (resumo, _) = explicar(Culprit::PlanoDeEnergia, Some(50), None);
        assert!(resumo.contains("50%"));
    }

    #[test]
    fn limite_nao_lido_nao_vira_sem_throttling() {
        assert!(matches!(avaliar_limites(None), LimitesDoProcessador::NaoSei));
        assert!(matches!(avaliar_limites(Some(0)), LimitesDoProcessador::Nenhum));
        assert!(matches!(
            avaliar_limites(Some(4)),
            LimitesDoProcessador::Ativos(4)
        ));
    }

    #[test]
    fn flags_nao_lidas_sem_outra_causa_viram_nao_foi_possivel_medir() {
        // Se alguém reverter `montar_relatorio` para `unwrap_or(0)`, este teste falha.
        let r = montar_relatorio(false, None, None, None, 0, None);

        assert_eq!(r.culprit, Culprit::Nenhum);
        assert!(
            r.summary.to_lowercase().contains("não foi possível medir"),
            "resumo não avisou a lacuna: {}",
            r.summary
        );
        assert!(!r.medido);
    }

    #[test]
    fn flags_lidas_como_zero_ainda_afirmam_processador_livre() {
        let r = montar_relatorio(false, None, None, Some(0), 0, None);

        assert_eq!(r.culprit, Culprit::Nenhum);
        assert!(r.summary.contains("livre"));
        assert!(!r.summary.to_lowercase().contains("não foi possível"));
        assert!(r.medido);
    }

    #[test]
    fn flags_nao_lidas_nao_apagam_causa_ja_explicada_por_outro_sinal() {
        let r = montar_relatorio(true, None, None, None, 0, None);
        assert_eq!(r.culprit, Culprit::Bateria);
        assert!(!r.summary.to_lowercase().contains("não foi possível"));
        assert!(r.medido);
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!("culpado: {:?}", r.culprit);
        println!("resumo: {}", r.summary);
        println!(
            "  frequencia: {:?}% | teto: {:?} | bateria: {} | eventos termicos: {}",
            r.percent_of_max, r.power_cap_percent, r.on_battery, r.thermal_events
        );

        assert!(!r.summary.is_empty());

        // Calor só com prova: protege o cliente de abrir um PC sem problema.
        if r.culprit == Culprit::Calor {
            assert!(
                r.thermal_events > 0 || r.percent_of_max.is_some(),
                "acusou calor sem evidência nenhuma"
            );
        }

        if r.culprit == Culprit::Nenhum {
            assert!(r.advice.is_empty());
        }
    }
}
