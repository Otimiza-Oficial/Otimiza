// Laboratório de streaming de assets: "trava quando ando pelo mapa". Não conclui pela configuração (jogo no HD
// é fato, não diagnóstico): o que prova é a COINCIDÊNCIA dos trancos com disco ocupado. Aí separa três causas
// com respostas opostas: memória acabando (checada PRIMEIRO; mover não resolve), mídia lenta (só aqui mover
// resolve), disco rápido e ocupado por outra coisa. Não promete quadros: aponta como medir depois.

use serde::{Deserialize, Serialize};

use super::gargalo::{ENGASGOS_POR_MINUTO, TRANCOS_COM_DISCO_PCT};
use super::telemetry::Telemetry;

/// Espelha `windows::discodojogo::Midia` em vez de importar: o laboratório é puro e roda em qualquer sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Midia {
    Ssd,
    Mecanico,
    NaoDeuParaLer,
}

#[cfg(target_os = "windows")]
impl From<super::windows::discodojogo::Midia> for Midia {
    fn from(m: super::windows::discodojogo::Midia) -> Self {
        use super::windows::discodojogo::Midia as D;
        match m {
            D::Ssd => Midia::Ssd,
            D::Mecanico => Midia::Mecanico,
            D::NaoDeuParaLer => Midia::NaoDeuParaLer,
        }
    }
}

/// SSD responde em menos de 1 ms, HD passa de 15: dez fica entre os dois sem encostar em nenhum.
pub const LATENCIA_DE_DISCO_LENTO_MS: f64 = 10.0;

/// Os mesmos 90% do `gargalo`: o sistema troca página antes de encostar no topo.
pub const MEMORIA_QUE_TROCA_PAGINA: f64 = 90.0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Veredito {
    SemMedicao,
    SemEngasgos,
    /// Fecha a porta do disco e devolve a pergunta ao gargalo.
    NaoEODisco,
    TrocaDeMemoria,
    AssetsDeMidiaLenta,
    OutraCoisaNoDisco,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Analise {
    pub veredito: Veredito,
    pub engasgos_por_minuto: Option<f64>,
    pub coincidencia_pct: Option<f64>,
    pub latencia_ms: Option<f64>,
    pub memoria_pct: Option<f64>,
    pub midia: Option<Midia>,
    pub falta: Vec<String>,
    pub explicacao: String,
    pub proximo_passo: Option<String>,
}

/// Grudado na sugestão: é a diferença entre sugerir e comprovar.
const COMO_COMPROVAR: &str = "Antes de mover, guarde a linha de base desta máquina; depois de \
                              mover, repita a medição de quadros e compare. Só a comparação com \
                              repetição diz se a mudança valeu — uma partida antes e uma depois \
                              não distinguem melhora de variação normal.";

/// `midia` NUNCA conclui sozinha: só entra depois da coincidência medida.
pub fn analisar(t: &Telemetry, midia: Option<Midia>) -> Analise {
    let engasgos = t.value("frametime.stutters_per_minute");
    let coincidencia = t.value("frametime.stutters_with_disk");
    let latencia = t.value("storage.latency");
    let memoria = t.value("ram.usage");

    let mut falta = Vec::new();

    let base = |veredito, explicacao: String, proximo_passo, falta: Vec<String>| Analise {
        veredito,
        engasgos_por_minuto: engasgos,
        coincidencia_pct: coincidencia,
        latencia_ms: latencia,
        memoria_pct: memoria,
        midia,
        falta,
        explicacao,
        proximo_passo,
    };

    let Some(engasgos_medidos) = engasgos else {
        falta.push("frametime.stutters_per_minute".to_string());
        return base(
            Veredito::SemMedicao,
            "Ninguém mediu os quadros durante uma partida ainda. Sem isso não há tranco para \
             investigar, e o disco onde o jogo mora não responde a pergunta sozinho."
                .to_string(),
            Some(
                "Rode a medição de quadros com o jogo aberto, andando pelo mapa — é andando que \
                 o jogo pede asset novo."
                    .to_string(),
            ),
            falta,
        );
    };

    if engasgos_medidos < ENGASGOS_POR_MINUTO {
        return base(
            Veredito::SemEngasgos,
            format!(
                "A partida medida teve {engasgos_medidos:.1} trancos por minuto, abaixo do que se \
                 sente como engasgo. Não há o que investigar no disco."
            ),
            None,
            falta,
        );
    }

    let Some(coincidencia) = coincidencia else {
        falta.push("frametime.stutters_with_disk".to_string());
        return base(
            Veredito::SemMedicao,
            format!(
                "A partida teve {engasgos_medidos:.1} trancos por minuto, mas não se mediu se eles \
                 caíram junto com atividade de disco. Sem esse instante, apontar o disco seria \
                 chute — shader compilando e disputa de memória fazem o mesmo buraco."
            ),
            None,
            falta,
        );
    };

    if coincidencia < TRANCOS_COM_DISCO_PCT {
        return base(
            Veredito::NaoEODisco,
            format!(
                "Os trancos existem — {engasgos_medidos:.1} por minuto — mas só {coincidencia:.0}% \
                 deles caíram com o disco ocupado. O disco não é a explicação, e mover o jogo de \
                 unidade não mudaria isto. A causa está no diagnóstico de gargalo."
            ),
            None,
            falta,
        );
    }

    // PRIMEIRO: com a memória no fim, o disco ocupado é consequência, não causa.
    if memoria.is_some_and(|m| m >= MEMORIA_QUE_TROCA_PAGINA) {
        let m = memoria.unwrap_or_default();
        return base(
            Veredito::TrocaDeMemoria,
            format!(
                "Os trancos caem com o disco ocupado ({coincidencia:.0}% deles), mas a memória do \
                 sistema está em {m:.0}%. Nesse estado o Windows troca página com o disco, e a \
                 atividade que coincide com o tranco é ISSO — não o jogo lendo asset. Mover o jogo \
                 de disco ameniza e não resolve."
            ),
            Some(
                "Feche o que estiver consumindo memória e meça de novo. Se os trancos sumirem com \
                 a memória folgada, a resposta era memória, e é aí que vale investir."
                    .to_string(),
            ),
            falta,
        );
    }

    if memoria.is_none() {
        falta.push("ram.usage".to_string());
    }

    // Latência medida agora (melhor) ou tipo de mídia lido do sistema: as duas valem.
    let latencia_ruim = latencia.is_some_and(|ms| ms >= LATENCIA_DE_DISCO_LENTO_MS);
    let midia_lenta = midia == Some(Midia::Mecanico);

    if latencia_ruim || midia_lenta {
        let evidencia = match (latencia.filter(|_| latencia_ruim), midia_lenta) {
            (Some(ms), true) => {
                format!("o disco responde em {ms:.1} ms por transferência e é um disco mecânico")
            }
            (Some(ms), false) => format!("o disco responde em {ms:.1} ms por transferência"),
            (None, _) => "o jogo está num disco mecânico".to_string(),
        };

        return base(
            Veredito::AssetsDeMidiaLenta,
            format!(
                "{coincidencia:.0}% dos trancos caíram com o disco ocupado, e {evidencia}. Este é \
                 o caso em que o jogo está esperando o asset chegar: andar pelo mapa pede conteúdo \
                 que ainda não está na memória, e ele vem devagar."
            ),
            Some(format!(
                "Mover este jogo para o disco mais rápido da máquina é a mudança que ataca a causa \
                 medida aqui. {COMO_COMPROVAR}"
            )),
            falta,
        );
    }

    if latencia.is_none() {
        falta.push("storage.latency".to_string());
    }
    if midia.is_none() {
        falta.push("o disco onde o jogo está instalado".to_string());
    }

    base(
        Veredito::OutraCoisaNoDisco,
        format!(
            "{coincidencia:.0}% dos trancos caíram com o disco ocupado, mas o disco está dando \
             conta{}. Então não é a mídia: é outra coisa mexendo no disco durante a partida — \
             antivírus varrendo, gravação de vídeo, ou shader sendo compilado para o cache. Trocar \
             o jogo de disco não muda nada disto.",
            match latencia {
                Some(ms) => format!(" ({ms:.1} ms por transferência)"),
                None => String::new(),
            }
        ),
        Some(
            "Meça de novo com a gravação de vídeo desligada e sem varredura em andamento. Se os \
             trancos continuarem, o próximo suspeito é a compilação de shader, que aparece forte \
             nas primeiras partidas depois de atualizar o driver."
                .to_string(),
        ),
        falta,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::telemetry::{Metric, Telemetry, Unit};

    fn com(pares: &[(&str, f64)]) -> Telemetry {
        let mut t = Telemetry::new(0, None);
        for (id, v) in pares {
            let unidade = match *id {
                "frametime.stutters_per_minute" => Unit::Count,
                "storage.latency" => Unit::Milliseconds,
                _ => Unit::Percent,
            };
            t.set(id, Metric::measured(*v, unidade, "teste"));
        }
        t.finish(0)
    }

    fn partida_ruim() -> Vec<(&'static str, f64)> {
        vec![
            ("frametime.stutters_per_minute", 14.0),
            ("frametime.stutters_with_disk", 85.0),
        ]
    }

    #[test]
    fn jogo_em_disco_mecanico_sem_medicao_nao_vira_diagnostico() {
        let a = analisar(&com(&[]), Some(Midia::Mecanico));

        assert_eq!(a.veredito, Veredito::SemMedicao);
        assert!(a
            .falta
            .contains(&"frametime.stutters_per_minute".to_string()));
        assert!(
            !a.proximo_passo
                .as_deref()
                .unwrap_or_default()
                .contains("Mover"),
            "{:?}",
            a.proximo_passo
        );
    }

    #[test]
    fn partida_lisa_nao_investiga_disco() {
        let a = analisar(
            &com(&[("frametime.stutters_per_minute", 1.0)]),
            Some(Midia::Mecanico),
        );

        assert_eq!(a.veredito, Veredito::SemEngasgos);
        assert!(a.proximo_passo.is_none());
    }

    #[test]
    fn trancos_que_nao_coincidem_com_disco_devolvem_a_pergunta() {
        let a = analisar(
            &com(&[
                ("frametime.stutters_per_minute", 20.0),
                ("frametime.stutters_with_disk", 15.0),
            ]),
            Some(Midia::Mecanico),
        );

        assert_eq!(a.veredito, Veredito::NaoEODisco);
        assert!(a.proximo_passo.is_none(), "não há o que mover");
    }

    #[test]
    fn sem_a_coincidencia_nao_conclui() {
        let a = analisar(
            &com(&[("frametime.stutters_per_minute", 20.0)]),
            Some(Midia::Mecanico),
        );

        assert_eq!(a.veredito, Veredito::SemMedicao);
        assert!(a
            .falta
            .contains(&"frametime.stutters_with_disk".to_string()));
    }

    #[test]
    fn memoria_no_fim_ganha_da_midia_lenta() {
        let mut pares = partida_ruim();
        pares.push(("ram.usage", 96.0));
        pares.push(("storage.latency", 22.0));

        let a = analisar(&com(&pares), Some(Midia::Mecanico));

        assert_eq!(a.veredito, Veredito::TrocaDeMemoria);
        assert!(!a
            .proximo_passo
            .as_deref()
            .unwrap_or_default()
            .contains("Mover"));
    }

    #[test]
    fn disco_lento_com_memoria_folgada_e_asset_streaming() {
        let mut pares = partida_ruim();
        pares.push(("ram.usage", 55.0));
        pares.push(("storage.latency", 24.0));

        let a = analisar(&com(&pares), Some(Midia::Mecanico));

        assert_eq!(a.veredito, Veredito::AssetsDeMidiaLenta);
        let passo = a.proximo_passo.expect("passo");
        assert!(passo.contains("Mover"));
        assert!(passo.contains("linha de base"), "{passo}");
    }

    #[test]
    fn latencia_ruim_conclui_sem_o_tipo_de_midia() {
        let mut pares = partida_ruim();
        pares.push(("ram.usage", 40.0));
        pares.push(("storage.latency", 31.0));

        let a = analisar(&com(&pares), None);

        assert_eq!(a.veredito, Veredito::AssetsDeMidiaLenta);
        assert!(a.explicacao.contains("31.0 ms"), "{}", a.explicacao);
    }

    #[test]
    fn disco_rapido_e_ocupado_nao_manda_mover_nada() {
        let mut pares = partida_ruim();
        pares.push(("ram.usage", 50.0));
        pares.push(("storage.latency", 0.3));

        let a = analisar(&com(&pares), Some(Midia::Ssd));

        assert_eq!(a.veredito, Veredito::OutraCoisaNoDisco);
        let passo = a.proximo_passo.expect("passo");
        assert!(!passo.contains("Mover"), "{passo}");
        assert!(a.explicacao.contains("shader"), "{}", a.explicacao);
    }

    #[test]
    fn midia_ilegivel_nao_e_absolvida_nem_condenada() {
        let mut pares = partida_ruim();
        pares.push(("ram.usage", 50.0));
        pares.push(("storage.latency", 0.4));

        let a = analisar(&com(&pares), Some(Midia::NaoDeuParaLer));

        assert_eq!(a.veredito, Veredito::OutraCoisaNoDisco);
    }

    #[test]
    fn a_conclusao_nao_apaga_o_que_faltou() {
        let a = analisar(&com(&partida_ruim()), None);

        assert_eq!(a.veredito, Veredito::OutraCoisaNoDisco);
        assert!(a.falta.contains(&"ram.usage".to_string()));
        assert!(a.falta.contains(&"storage.latency".to_string()));
    }
}
