// Orçamento de latência, do clique ao pixel. Não publica "a sua latência": sem instrumentar o jogo ou um
// aparelho na tela, só dá para medir pedaços. Devolve as cinco etapas, cada uma com valor ou motivo, e um PISO
// (soma só do medido: "pelo menos"). Não vira métrica do contrato, para não ser lido como total. Etapa sem
// medição nunca vira zero: é assim que nasce o "1 ms de latência" do mercado.

use serde::{Deserialize, Serialize};

use super::telemetry::{Quality, Telemetry};

/// Cinco porque é a divisão que o medível permite: o quadro que o Windows anuncia já passou por jogo e placa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Etapa {
    Entrada,
    JogoEPlaca,
    Fila,
    Apresentacao,
    Tela,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parcela {
    pub etapa: Etapa,
    /// Ausente quando não foi medida. NUNCA zero.
    pub ms: Option<f64>,
    pub qualidade: Quality,
    pub origem: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Orcamento {
    pub parcelas: Vec<Parcela>,
    /// LIMITE INFERIOR: a latência real é isto mais as etapas não medidas.
    pub piso_ms: Option<f64>,
    pub etapas_com_valor: usize,
    pub etapas_totais: usize,
    pub observacoes: Vec<String>,
}

const ETAPAS: usize = 5;

/// Abaixo de 2 ms, "o limite é a tela" estaria apoiado na terceira casa de duas medidas que oscilam.
const DIFERENCA_QUE_CONTA_MS: f64 = 2.0;

pub fn orcar(t: &Telemetry) -> Orcamento {
    let mut parcelas = Vec::with_capacity(ETAPAS);

    // Pior caso de um ciclo de varredura do mouse. A taxa não é publicada por interface nenhuma: vem da contagem
    // em `modules::mouse`, e sem ela a etapa fica ausente.
    let entrada = t
        .get("input.polling_rate")
        .and_then(|m| m.value)
        .filter(|hz| *hz > 0.0)
        .map(|hz| 1000.0 / hz);

    match entrada {
        Some(ms) => parcelas.push(Parcela {
            etapa: Etapa::Entrada,
            ms: Some(ms),
            // Estimativa: conta a chegada ao nosso processo, não o instante do sensor.
            qualidade: Quality::Estimated,
            origem: "pior caso de um ciclo de varredura do mouse, de input.polling_rate".to_string(),
        }),
        None => parcelas.push(Parcela {
            etapa: Etapa::Entrada,
            ms: None,
            qualidade: Quality::Unknown,
            origem: "o Windows não publica a taxa de varredura do mouse; medi-la exige contar os \
                     relatos que chegam à janela deste aplicativo"
                .to_string(),
        }),
    }

    // Jogo e placa juntos: o evento do Windows não separa um do outro. A qualidade é herdada da métrica.
    match t.get("frametime.mean").filter(|m| m.value.is_some()) {
        Some(m) => parcelas.push(Parcela {
            etapa: Etapa::JogoEPlaca,
            ms: m.value,
            qualidade: m.quality,
            origem: format!("frametime.mean, por {}", m.source),
        }),
        None => parcelas.push(Parcela {
            etapa: Etapa::JogoEPlaca,
            ms: None,
            qualidade: Quality::Unknown,
            origem: "exige uma medição de quadros durante a partida (frametime.mean)".to_string(),
        }),
    }

    // A espera na fila do driver exige os eventos de conclusão da GPU, que não são capturados.
    parcelas.push(Parcela {
        etapa: Etapa::Fila,
        ms: None,
        qualidade: Quality::Unknown,
        origem: "exige os eventos de conclusão da GPU; hoje só é capturado o início da \
                 apresentação"
            .to_string(),
    });

    // O PIOR caso de um ciclo inteiro, como estimativa: a sincronia com a tela não se lê daqui, e a média de meio
    // ciclo pareceria melhor do que é.
    let apresentacao = t
        .get("display.refresh")
        .and_then(|m| m.value)
        .filter(|hz| *hz > 0.0)
        .map(|hz| 1000.0 / hz);

    match apresentacao {
        Some(ms) => parcelas.push(Parcela {
            etapa: Etapa::Apresentacao,
            ms: Some(ms),
            qualidade: Quality::Estimated,
            origem: "pior caso de um ciclo da tela, de display.refresh; depende de o jogo estar \
                     sincronizado com ela, o que não é lido daqui"
                .to_string(),
        }),
        None => parcelas.push(Parcela {
            etapa: Etapa::Apresentacao,
            ms: None,
            qualidade: Quality::Unknown,
            origem: "a taxa de atualização da tela não foi lida".to_string(),
        }),
    }

    // O tempo de resposta do fabricante é de laboratório: repeti-lo seria propaganda como se fosse medição.
    parcelas.push(Parcela {
        etapa: Etapa::Tela,
        ms: None,
        qualidade: Quality::Unknown,
        origem: "só um aparelho apontado para o monitor mede isto; o número do fabricante é de \
                 laboratório"
            .to_string(),
    });

    let com_valor: Vec<f64> = parcelas
        .iter()
        .filter(|p| p.qualidade != Quality::Unknown)
        .filter_map(|p| p.ms)
        .collect();

    // Soma vazia é `None`, não `0.0`.
    let piso_ms = (!com_valor.is_empty()).then(|| com_valor.iter().sum());

    let mut observacoes = Vec::new();

    // Se esperar um ciclo da tela custa mais que o quadro inteiro, quem limita é a tela, e trocar peça não muda isso.
    if let (Some(quadro), Some(tela)) = (
        parcelas
            .iter()
            .find(|p| p.etapa == Etapa::JogoEPlaca)
            .and_then(|p| p.ms),
        apresentacao,
    ) {
        if tela > quadro + DIFERENCA_QUE_CONTA_MS {
            observacoes.push(format!(
                "A máquina desenha um quadro em {quadro:.1} ms e a tela leva até {tela:.1} ms para \
                 mostrá-lo. Nesta parte da corrente quem manda é a tela: mais quadros por segundo \
                 não encurtam essa espera."
            ));
        } else if quadro > tela + DIFERENCA_QUE_CONTA_MS {
            observacoes.push(format!(
                "A tela está pronta a cada {tela:.1} ms e a máquina entrega um quadro a cada \
                 {quadro:.1} ms. Aqui quem manda é a máquina, e o que foi medido dela está no \
                 diagnóstico de gargalo."
            ));
        }
    }

    Orcamento {
        etapas_com_valor: com_valor.len(),
        etapas_totais: ETAPAS,
        parcelas,
        piso_ms,
        observacoes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::telemetry::{Metric, Unit};

    fn vazia() -> Telemetry {
        Telemetry::new(0, None)
    }

    #[test]
    fn sem_nenhuma_medida_o_piso_e_ausente_e_nao_zero() {
        let o = orcar(&vazia().finish(0));

        assert_eq!(o.piso_ms, None, "zero afirmaria latência nenhuma");
        assert_eq!(o.etapas_com_valor, 0);
        assert_eq!(o.etapas_totais, ETAPAS);

        assert_eq!(o.parcelas.len(), ETAPAS);
        assert!(o.parcelas.iter().all(|p| p.ms.is_none()));
        assert!(o.parcelas.iter().all(|p| !p.origem.is_empty()));
    }

    #[test]
    fn a_taxa_do_mouse_preenche_a_etapa_da_entrada() {
        let mut t = vazia();
        t.set(
            "input.polling_rate",
            Metric::measured(1000.0, Unit::Hertz, "janela do aplicativo"),
        );

        let o = orcar(&t.finish(0));
        let entrada = o
            .parcelas
            .iter()
            .find(|p| p.etapa == Etapa::Entrada)
            .expect("etapa");

        assert_eq!(entrada.ms, Some(1.0));
        assert_eq!(entrada.qualidade, Quality::Estimated);
        assert_eq!(o.etapas_com_valor, 1);
    }

    #[test]
    fn taxa_de_mouse_invalida_nao_vira_divisao_por_zero() {
        let mut t = vazia();
        t.set(
            "input.polling_rate",
            Metric::measured(0.0, Unit::Hertz, "janela do aplicativo"),
        );

        let o = orcar(&t.finish(0));
        let entrada = o
            .parcelas
            .iter()
            .find(|p| p.etapa == Etapa::Entrada)
            .expect("etapa");

        assert_eq!(entrada.ms, None);
        assert_eq!(entrada.qualidade, Quality::Unknown);
    }

    #[test]
    fn soma_so_o_que_foi_medido() {
        let mut t = vazia();
        t.set(
            "frametime.mean",
            Metric::measured(10.0, Unit::Milliseconds, "etw"),
        );
        t.set(
            "display.refresh",
            Metric::measured(100.0, Unit::Hertz, "win32"),
        );

        let o = orcar(&t.finish(0));

        assert_eq!(o.piso_ms, Some(20.0));
        assert_eq!(o.etapas_com_valor, 2);

        assert_eq!(
            o.parcelas
                .iter()
                .filter(|p| p.qualidade == Quality::Unknown)
                .count(),
            3
        );
    }

    #[test]
    fn a_etapa_herda_a_qualidade_da_medida() {
        let mut t = vazia();
        t.set(
            "frametime.mean",
            Metric::estimated(14.0, Unit::Milliseconds, "etw", "partida anterior")
                .com_idade(1_200_000),
        );

        let o = orcar(&t.finish(0));
        let quadro = o
            .parcelas
            .iter()
            .find(|p| p.etapa == Etapa::JogoEPlaca)
            .expect("etapa");

        assert_eq!(quadro.qualidade, Quality::Estimated);
        assert_eq!(quadro.ms, Some(14.0));
    }

    #[test]
    fn a_espera_pela_tela_e_o_ciclo_inteiro() {
        let mut t = vazia();
        t.set(
            "display.refresh",
            Metric::measured(60.0, Unit::Hertz, "win32"),
        );

        let o = orcar(&t.finish(0));
        let tela = o
            .parcelas
            .iter()
            .find(|p| p.etapa == Etapa::Apresentacao)
            .expect("etapa");

        let ms = tela.ms.expect("valor");
        assert!((ms - 16.666).abs() < 0.01, "{ms}");
        assert_eq!(tela.qualidade, Quality::Estimated);
    }

    #[test]
    fn taxa_de_tela_invalida_nao_vira_divisao_por_zero() {
        let mut t = vazia();
        t.set(
            "display.refresh",
            Metric::measured(0.0, Unit::Hertz, "win32"),
        );

        let o = orcar(&t.finish(0));
        let tela = o
            .parcelas
            .iter()
            .find(|p| p.etapa == Etapa::Apresentacao)
            .expect("etapa");

        assert_eq!(tela.ms, None);
        assert_eq!(tela.qualidade, Quality::Unknown);
    }

    #[test]
    fn tela_lenta_com_maquina_rapida_diz_que_o_limite_e_a_tela() {
        let mut t = vazia();
        t.set(
            "frametime.mean",
            Metric::measured(5.0, Unit::Milliseconds, "etw"),
        );
        t.set(
            "display.refresh",
            Metric::measured(60.0, Unit::Hertz, "win32"),
        );

        let o = orcar(&t.finish(0));

        assert!(
            o.observacoes
                .iter()
                .any(|x| x.contains("quem manda é a tela")),
            "{:?}",
            o.observacoes
        );
    }

    #[test]
    fn maquina_lenta_com_tela_rapida_devolve_a_pergunta_ao_gargalo() {
        let mut t = vazia();
        t.set(
            "frametime.mean",
            Metric::measured(25.0, Unit::Milliseconds, "etw"),
        );
        t.set(
            "display.refresh",
            Metric::measured(144.0, Unit::Hertz, "win32"),
        );

        let o = orcar(&t.finish(0));

        assert!(
            o.observacoes
                .iter()
                .any(|x| x.contains("quem manda é a máquina")),
            "{:?}",
            o.observacoes
        );
    }

    /// Empate não vira frase.
    #[test]
    fn diferenca_pequena_nao_gera_afirmacao() {
        let mut t = vazia();
        t.set(
            "frametime.mean",
            Metric::measured(16.0, Unit::Milliseconds, "etw"),
        );
        t.set(
            "display.refresh",
            Metric::measured(60.0, Unit::Hertz, "win32"),
        );

        let o = orcar(&t.finish(0));

        assert!(o.observacoes.is_empty(), "{:?}", o.observacoes);
    }
}
