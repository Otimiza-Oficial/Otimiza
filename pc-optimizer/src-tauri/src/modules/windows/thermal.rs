// Por que o processador não está entregando tudo
//
// Um notebook empoeirado a 95 graus reduz o processador para uma fração da
// velocidade. É uma causa enorme e completamente invisível de "PC lento": o
// técnico limpa, otimiza, mede, e nada melhora — porque o problema é físico e
// nenhum ajuste de software resolve. Se o Otimiza detectar e explicar isso, ele
// responde a pergunta que fica sem resposta em todo atendimento.
//
// POR QUE ESTE MÓDULO É CAUTELOSO A ESSE PONTO
//
// A investigação que originou este arquivo testou os caminhos óbvios e derrubou
// quase todos:
//
// - `MSAcpi_ThermalZoneTemperature` exige elevação e muitos fabricantes
//   devolvem valor fixo. Temperatura sozinha, além disso, não prova perda de
//   desempenho nenhuma.
// - `Win32_Processor.CurrentClockSpeed` fica CONGELADO no valor nominal do
//   SMBIOS. Medido: marcava 3600 MHz enquanto a frequência real oscilava em
//   4100. Um detector baseado nele acusaria ou inocentaria ao acaso.
// - `% Limite de Desempenho` mistura calor com plano de energia na mesma
//   leitura. A própria documentação da Microsoft diz que o valor cai tanto por
//   política de energia quanto por superaquecimento.
//
// Acusar calor com base em qualquer um deles seria dar um diagnóstico falso —
// inclusive para quem só está com o plano "Economia de energia" ligado, que é
// comum e tem solução trivial. Então o módulo elimina causas em ordem, e só
// afirma calor quando o próprio Windows registrou um evento térmico.
//
// O QUE AINDA NÃO FOI PROVADO
//
// A ausência de falso positivo foi verificada: máquina fria, processador a 100%
// de carga por 45 segundos, nenhum limite reportado. A detecção positiva num
// notebook realmente quente NÃO foi verificada — faltou o hardware. Por isso o
// texto que vai para a tela cita a data e o número do evento do Windows, em vez
// de afirmar por conta própria.

use super::{power, shell};
use serde::{Deserialize, Serialize};

/// Grupo "processador" nas configurações de energia.
const SUB_PROCESSADOR: &str = "54533251-82be-4824-96c1-47b60b740d00";
/// Teto de desempenho do processador, em porcentagem.
const PROCTHROTTLEMAX: &str = "bc5038f7-23e0-4960-96da-33abaf5935ec";

/// Log térmico do núcleo do Windows. Legível SEM elevação.
const LOG_TERMICO: &str = "Microsoft-Windows-Kernel-Power/Thermal-Operational";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Culprit {
    /// Nada está segurando o processador.
    Nenhum,
    /// Na bateria. O Windows limita de propósito, e isso é normal.
    Bateria,
    /// O plano de energia tem um teto configurado abaixo de 100%.
    PlanoDeEnergia,
    /// O Windows registrou evento térmico. É a única situação em que este
    /// módulo diz a palavra "calor".
    Calor,
    /// Limite elétrico do hardware — fonte, bateria degradada, PL1/PL2. É
    /// outra conversa, e não deve ser vendido como sujeira no cooler.
    LimiteEletrico,
    /// A frequência está baixa e nenhuma causa conhecida explica.
    NaoIdentificado,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalReport {
    pub culprit: Culprit,
    /// Frase pronta para a tela, sempre com o que foi medido.
    pub summary: String,
    pub advice: String,
    /// Porcentagem da frequência máxima observada na amostra.
    pub percent_of_max: Option<f64>,
    /// Teto configurado no plano de energia, quando existe.
    pub power_cap_percent: Option<u32>,
    pub on_battery: bool,
    /// Quantos eventos térmicos o Windows registrou recentemente.
    pub thermal_events: usize,
    /// Data do evento térmico mais recente, para o texto poder citá-la.
    pub last_thermal_event: Option<String>,
    /// Se o contador de limite do processador foi lido de verdade.
    ///
    /// Sem este campo, `suporte.rs` (o relatório de atendimento) teria que
    /// adivinhar a lacuna comparando o texto de `summary` — e este projeto
    /// tem uma trava contra decidir qualquer coisa comparando prosa que o
    /// backend escreveu para a tela. `false` só no caso em que
    /// `LimitesDoProcessador::NaoSei` acabou virando `Culprit::Nenhum` por
    /// falta de leitura, e não porque o processador está de fato livre.
    pub medido: bool,
}

// -------------------------------------------------------------- leituras

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawPerf {
    percentof_maximum_frequency: Option<f64>,
    performance_limit_flags: Option<u64>,
}

/// Amostra dos contadores de desempenho do processador.
///
/// A classe WMI expõe os mesmos números que o PDH com nomes de propriedade em
/// inglês e sem exigir elevação. O contador pelo nome traduzido — "Informações
/// do Processador" num Windows em português — não serve: é exatamente o tipo de
/// dependência de idioma que já quebrou este projeto uma vez.
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

/// Se a máquina está na bateria.
fn na_bateria() -> bool {
    // `BatteryStatus` 1 significa descarregando; 2 é ligada na tomada. Desktop
    // não tem instância nenhuma, e aí a resposta é falso.
    let script = "$b = Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue | \
                  Select-Object -First 1; if ($b) { $b.BatteryStatus } else { 2 }";

    shell::powershell(script)
        .ok()
        .and_then(|s| s.stdout.trim().parse::<u32>().ok())
        .map(|estado| estado == 1)
        .unwrap_or(false)
}

/// Teto de desempenho configurado no plano de energia ativo.
///
/// Lido do registro, não do `powercfg`, cuja saída vem traduzida.
fn teto_do_plano() -> Option<u32> {
    let esquema = power::active_scheme().ok()?;

    match power::read_power_setting(&esquema, SUB_PROCESSADOR, PROCTHROTTLEMAX) {
        Ok(crate::modules::changelog::PreviousValue::Dword(valor)) => Some(valor),
        // Ausente significa que o plano herda o padrão do Windows, que é 100%.
        _ => None,
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawTermico {
    when: Option<String>,
}

/// Eventos térmicos registrados pelo Windows nos últimos 30 dias.
///
/// Este é o único sinal em que o módulo confia para dizer "calor". Resfriamento
/// passivo ACPI é, por definição, resposta a temperatura — não há como
/// confundir com plano de energia.
fn eventos_termicos() -> Vec<RawTermico> {
    let script = format!(
        "$e = Get-WinEvent -FilterHashtable @{{ LogName='{}'; \
         StartTime=(Get-Date).AddDays(-30) }} -MaxEvents 40 -ErrorAction SilentlyContinue; \
         ConvertTo-Json -Compress -Depth 3 -InputObject @($e | ForEach-Object {{ \
           [ordered]@{{ when = $_.TimeCreated.ToString('s') }} }})",
        LOG_TERMICO
    );

    shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

// ------------------------------------------------------------- veredito

/// Bit de limite térmico fora da zona ACPI.
pub const LIMITE_TERMICO: u64 = 0x1;
/// Bit de limite elétrico — orçamento de energia do hardware.
pub const LIMITE_ELETRICO: u64 = 0x2;

/// Abaixo disto a frequência é considerada realmente reduzida.
///
/// Não é 100 porque o valor oscila naturalmente alguns pontos, e um limiar
/// colado no teto transformaria ruído em diagnóstico.
pub const FREQUENCIA_BAIXA: f64 = 90.0;

/// O que se sabe sobre os bits de limite de desempenho do processador.
///
/// Mesma razão do `ErrosDoDisco` em `health.rs`: `Some(0)` é "medi e não há
/// limite ativo"; `None` é "não consegui ler". O código antigo usava
/// `unwrap_or(0)`, e uma consulta que falhasse virava exatamente a mesma coisa
/// que um processador livre — dizer que não há throttling é o defeito no lugar
/// mais caro, porque detectar throttling é o que este módulo vende.
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

/// Decide o culpado, eliminando causas em ordem.
///
/// A ordem é o produto inteiro deste módulo. Cada passo elimina uma explicação
/// mais provável e mais barata de resolver antes de chegar na mais cara — e a
/// mais cara, calor, exige prova do próprio Windows.
pub fn decidir(
    na_bateria: bool,
    teto: Option<u32>,
    percentual: Option<f64>,
    flags: u64,
    eventos_termicos: usize,
) -> Culprit {
    // 1. Bateria explica limitação sozinha, e é comportamento correto do
    //    Windows. Acusar calor aqui seria assustar à toa.
    if na_bateria {
        return Culprit::Bateria;
    }

    // 2. Teto configurado é a causa mais comum e a de correção mais fácil.
    //    Vem antes de tudo porque, se ele existe, ele explica o número sozinho.
    if teto.is_some_and(|t| t < 100) {
        return Culprit::PlanoDeEnergia;
    }

    // 3. Evento térmico do Windows: a única prova aceita para dizer "calor".
    if eventos_termicos > 0 {
        return Culprit::Calor;
    }

    // 4. O bit térmico cobre só o caminho fora da zona ACPI, então ele reforça
    //    mas não é obrigatório — por isso vem depois do log, não antes.
    if flags & LIMITE_TERMICO != 0 {
        return Culprit::Calor;
    }

    if flags & LIMITE_ELETRICO != 0 {
        return Culprit::LimiteEletrico;
    }

    // 5. Frequência baixa sem nenhuma causa conhecida. Este é o caso em que o
    //    produto precisa dizer que não sabe, em vez de escolher o palpite mais
    //    vendável.
    match percentual {
        Some(p) if p < FREQUENCIA_BAIXA => Culprit::NaoIdentificado,
        _ => Culprit::Nenhum,
    }
}

/// Texto que vai para a tela do cliente.
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

/// Monta o relatório completo a partir de dados já lidos.
///
/// Extraída de `analyze()` para que a COSTURA entre `avaliar_limites` e o
/// texto que vai para a tela — inclusive o caso `NaoSei` — seja testável sem
/// hardware. Sem esta função, um teste do ajudante puro não provava nada
/// sobre `analyze()`: alguém podia reverter a chamada de volta para
/// `unwrap_or(0)` e nenhum teste acusaria.
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
        // `Nenhum` e `NaoSei` entram como 0 na decisão por bit — mas só `Nenhum`
        // é fato medido. Para `NaoSei` a diferença é corrigida abaixo, no texto.
        LimitesDoProcessador::Nenhum | LimitesDoProcessador::NaoSei => 0,
    };

    let culprit = decidir(bateria, teto, percentual, flags, eventos_termicos);

    let nao_medido = matches!(limites, LimitesDoProcessador::NaoSei) && culprit == Culprit::Nenhum;

    let (summary, advice) = if nao_medido {
        // Sem os outros sinais (bateria, teto, evento térmico) explicando nada,
        // o veredito só chegou a "livre" porque o bit de limite virou 0 na falta
        // de leitura. Isso não é "livre" — é "não conseguimos checar", e dizer
        // que está tudo bem aqui é precisamente o defeito que este módulo existe
        // para evitar.
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
    }
}

/// Análise completa.
pub fn analyze() -> ThermalReport {
    let bateria = na_bateria();
    let teto = teto_do_plano();
    let contadores = amostrar_contadores().unwrap_or_default();
    let termicos = eventos_termicos();
    let ultimo = termicos.first().and_then(|t| t.when.clone());

    montar_relatorio(
        bateria,
        teto,
        contadores.percentof_maximum_frequency,
        contadores.performance_limit_flags,
        termicos.len(),
        ultimo,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bateria_vem_antes_de_tudo() {
        // Na bateria o Windows limita de propósito. Acusar calor aqui seria
        // mandar o cliente abrir um notebook que não tem problema nenhum.
        assert_eq!(
            decidir(true, Some(50), Some(40.0), LIMITE_TERMICO, 9),
            Culprit::Bateria
        );
    }

    #[test]
    fn plano_de_energia_vem_antes_de_calor() {
        // Um teto configurado explica o número sozinho, e é a correção mais
        // barata. Chamar isso de calor mandaria o cliente limpar o cooler para
        // resolver algo que era um clique.
        assert_eq!(
            decidir(false, Some(50), Some(50.0), 0, 0),
            Culprit::PlanoDeEnergia
        );

        // Teto em 100 não é limite.
        assert_eq!(decidir(false, Some(100), Some(100.0), 0, 0), Culprit::Nenhum);
        // Ausente significa herdar o padrão do Windows, que é 100.
        assert_eq!(decidir(false, None, Some(100.0), 0, 0), Culprit::Nenhum);
    }

    #[test]
    fn calor_exige_prova_do_windows() {
        // Frequência baixíssima, sem evento térmico e sem bit: o produto diz
        // que não sabe. É a regra que impede o diagnóstico falso.
        assert_eq!(
            decidir(false, None, Some(35.0), 0, 0),
            Culprit::NaoIdentificado
        );

        // Com evento registrado, aí sim.
        assert_eq!(decidir(false, None, Some(35.0), 0, 3), Culprit::Calor);
        // Ou com o bit térmico do próprio processador.
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
        // O valor oscila alguns pontos sozinho. Um limiar colado em 100
        // transformaria ruído em alarme.
        assert_eq!(decidir(false, None, Some(97.0), 0, 0), Culprit::Nenhum);
        assert_eq!(decidir(false, None, Some(89.0), 0, 0), Culprit::NaoIdentificado);
    }

    #[test]
    fn sem_leitura_nenhuma_nao_acusa_nada() {
        // Sem contador disponível não há o que afirmar.
        assert_eq!(decidir(false, None, None, 0, 0), Culprit::Nenhum);
    }

    #[test]
    fn texto_de_calor_cita_a_data_do_evento() {
        // O produto não afirma por conta própria: ele mostra o registro do
        // Windows, com data. Foi a condição para deixar esta afirmação existir.
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
        // A consequência de chutar, escrita para o cliente entender por que
        // preferimos não dar um culpado.
        assert!(conselho.contains("trocar peça à toa"));
    }

    #[test]
    fn plano_limitado_diz_a_porcentagem() {
        let (resumo, _) = explicar(Culprit::PlanoDeEnergia, Some(50), None);
        assert!(resumo.contains("50%"));
    }

    #[test]
    fn limite_nao_lido_nao_vira_sem_throttling() {
        // Zero significa "nenhum limite ativo". Falha de leitura virando zero faz o
        // produto dizer que não há throttling num módulo cujo argumento de venda é
        // justamente detectar throttling.
        assert!(matches!(avaliar_limites(None), LimitesDoProcessador::NaoSei));
        assert!(matches!(avaliar_limites(Some(0)), LimitesDoProcessador::Nenhum));
        assert!(matches!(
            avaliar_limites(Some(4)),
            LimitesDoProcessador::Ativos(4)
        ));
    }

    #[test]
    fn flags_nao_lidas_sem_outra_causa_viram_nao_foi_possivel_medir() {
        // Esta é a costura que faltava: `avaliar_limites` sozinho não prova que
        // `analyze()` usa a decisão dele no texto. Se alguém reverter
        // `montar_relatorio` para `unwrap_or(0)`, este teste falha — o resumo
        // volta a dizer "livre para trabalhar na velocidade máxima" quando, na
        // verdade, o contador nunca foi lido.
        let r = montar_relatorio(false, None, None, None, 0, None);

        assert_eq!(r.culprit, Culprit::Nenhum);
        assert!(
            r.summary.to_lowercase().contains("não foi possível medir"),
            "resumo não avisou a lacuna: {}",
            r.summary
        );
        // `medido` é o campo que `suporte.rs` usa para saber da lacuna sem
        // comparar a prosa acima — os dois precisam concordar.
        assert!(!r.medido);
    }

    #[test]
    fn flags_lidas_como_zero_ainda_afirmam_processador_livre() {
        // O espelho do teste acima: `Some(0)` é medição de verdade — "não há
        // limite ativo" — e o texto de sempre ("processador está livre")
        // continua correto, sem o aviso de incerteza.
        let r = montar_relatorio(false, None, None, Some(0), 0, None);

        assert_eq!(r.culprit, Culprit::Nenhum);
        assert!(r.summary.contains("livre"));
        assert!(!r.summary.to_lowercase().contains("não foi possível"));
        assert!(r.medido);
    }

    #[test]
    fn flags_nao_lidas_nao_apagam_causa_ja_explicada_por_outro_sinal() {
        // Quando bateria, plano de energia ou evento térmico já respondem por
        // conta própria, o contador de flags nem entrou na decisão — não há
        // porque avisar incerteza sobre um dado que não foi decisivo.
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

        // Calor só pode ser afirmado com prova. Este é o teste que protege o
        // cliente de ser mandado abrir um PC que não tem problema.
        if r.culprit == Culprit::Calor {
            assert!(
                r.thermal_events > 0 || r.percent_of_max.is_some(),
                "acusou calor sem evidência nenhuma"
            );
        }

        // E se não há culpado, não pode haver conselho — conselho sem problema
        // é ruído que faz o cliente achar que tem algo errado.
        if r.culprit == Culprit::Nenhum {
            assert!(r.advice.is_empty());
        }
    }
}
