// Orçamento de latência: do clique ao pixel
//
// POR QUE ISTO NÃO PUBLICA UM NÚMERO DE LATÊNCIA
//
// Todo otimizador que fala em latência mostra um número só — "38 ms" — e não
// diz de onde ele saiu. Não sai de lugar nenhum: medir latência de verdade, do
// movimento do mouse até o pixel mudar na tela, exige instrumentar o jogo ou
// um aparelho apontado para o monitor. Nada disso está ao alcance de um
// programa rodando ao lado.
//
// O que ESTÁ ao alcance é medir alguns pedaços da corrente e dizer com todas
// as letras quais são os outros. É isso que este módulo faz: ele devolve as
// cinco etapas do caminho, cada uma com o seu valor ou com o motivo de não ter
// valor, e um PISO — a soma só do que foi medido.
//
// PISO, E NÃO TOTAL. A diferença não é modéstia:
//
//   - "A sua latência é 22 ms" é falso quando três das cinco etapas não foram
//     medidas. O número real é maior, e ninguém sabe quanto.
//   - "Pelo menos 22 ms, e três etapas não foram medidas" é verdade, e continua
//     sendo útil: se o piso já passa do que o cliente aceita, o problema está
//     no que foi medido, e aí há o que fazer.
//
// E POR QUE O PISO NÃO VIRA MÉTRICA DO CONTRATO
//
// Porque o nome ganharia vida própria. Um `latency.total` no painel de
// evidências seria lido como "a latência desta máquina" por qualquer um que
// batesse o olho, inclusive por um código futuro deste mesmo produto. Um
// limite inferior precisa carregar a palavra "pelo menos" junto, e um número
// solto num catálogo não carrega.
//
// A REGRA DO ZERO
//
// Nenhuma etapa sem medição vira zero. Um zero somaria como se aquele pedaço
// da corrente não custasse nada, e é exatamente o engano que produz o "1 ms de
// latência" que se vê por aí. Etapa sem medição sai com valor ausente e com o
// motivo escrito.

use serde::{Deserialize, Serialize};

use super::telemetry::{Quality, Telemetry};

/// As etapas do caminho entre o gesto e o pixel.
///
/// São cinco e não quatro nem seis porque esta é a divisão que o que se pode
/// medir permite: juntar jogo e placa numa etapa só é honesto (o quadro que o
/// Windows anuncia já passou pelos dois), e separá-las seria inventar a linha
/// divisória.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Etapa {
    /// Do movimento do mouse até o sistema saber dele.
    Entrada,
    /// O jogo simular o mundo e a placa desenhar o quadro.
    JogoEPlaca,
    /// O quadro esperando a vez na fila do driver.
    Fila,
    /// Do quadro pronto até a tela começar a mostrá-lo.
    Apresentacao,
    /// O painel trocar a cor do pixel de verdade.
    Tela,
}

/// Uma etapa do orçamento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parcela {
    pub etapa: Etapa,
    /// Ausente quando não foi medida. NUNCA zero. Ver a regra do zero acima.
    pub ms: Option<f64>,
    pub qualidade: Quality,
    /// De onde saiu o número, ou o que falta para haver número.
    pub origem: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Orcamento {
    pub parcelas: Vec<Parcela>,
    /// Soma só do que foi medido ou estimado. Ausente quando nada foi.
    ///
    /// É um LIMITE INFERIOR. A latência real é este número mais o custo das
    /// etapas que não puderam ser medidas.
    pub piso_ms: Option<f64>,
    pub etapas_com_valor: usize,
    pub etapas_totais: usize,
    /// O que os números medidos permitem afirmar. Vazio quando não permitem
    /// afirmar nada.
    pub observacoes: Vec<String>,
}

const ETAPAS: usize = 5;

/// Acima disto a diferença entre render e tela deixa de ser arredondamento.
///
/// Dois milissegundos. Abaixo disso, dizer que "o limite é a tela" estaria
/// apoiado na terceira casa decimal de duas medidas que oscilam.
const DIFERENCA_QUE_CONTA_MS: f64 = 2.0;

/// Monta o orçamento a partir do que a telemetria trouxer.
pub fn orcar(t: &Telemetry) -> Orcamento {
    let mut parcelas = Vec::with_capacity(ETAPAS);

    // ---- entrada
    //
    // O Windows entrega o evento do mouse, não o instante em que o sensor o
    // produziu. A diferença entre os dois é a taxa de varredura do aparelho, e
    // ela não é publicada por nenhuma interface que um programa comum alcance.
    parcelas.push(Parcela {
        etapa: Etapa::Entrada,
        ms: None,
        qualidade: Quality::Unknown,
        origem: "o Windows não publica o instante em que o mouse produziu o evento, só o instante \
                 em que o entregou"
            .to_string(),
    });

    // ---- jogo e placa
    //
    // O tempo de quadro. É o que o jogo levou para simular e a placa para
    // desenhar — os dois juntos, porque o evento que o Windows anuncia já
    // passou pelos dois e não separa um do outro.
    //
    // A qualidade é HERDADA da métrica, e não afirmada aqui: uma medição de
    // quadros de vinte minutos atrás continua sendo de vinte minutos atrás
    // depois de entrar nesta conta.
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

    // ---- fila do driver
    //
    // O Otimiza escuta o início da apresentação de cada quadro. Quanto tempo
    // aquele quadro esperou ANTES disso — na fila que o driver mantém para não
    // deixar a placa ociosa — exige os eventos de conclusão da GPU, que são
    // outro provedor do mesmo canal e não estão sendo capturados.
    parcelas.push(Parcela {
        etapa: Etapa::Fila,
        ms: None,
        qualidade: Quality::Unknown,
        origem: "exige os eventos de conclusão da GPU; hoje só é capturado o início da \
                 apresentação"
            .to_string(),
    });

    // ---- espera pela tela
    //
    // Um quadro pronto no meio da varredura espera a próxima. Quanto ele
    // espera depende de o jogo estar sincronizado com a tela, e isso não se
    // sabe daqui — por isso o valor é o PIOR caso de um ciclo inteiro, marcado
    // como estimativa, e nunca a média de meio ciclo que pareceria melhor.
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

    // ---- painel
    //
    // O tempo de resposta que o fabricante anuncia é de laboratório e varia por
    // modo de imagem. Repetir aquele número aqui seria repetir propaganda como
    // se fosse medição.
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

    // Soma vazia é `None`, e não `0.0`. Um zero aqui viraria "1 ms de
    // latência" na tela, que é a mentira que este módulo existe para não
    // contar.
    let piso_ms = (!com_valor.is_empty()).then(|| com_valor.iter().sum());

    let mut observacoes = Vec::new();

    // A única afirmação que os dois números medidos sustentam sozinhos: se a
    // espera por um ciclo da tela é maior que o quadro inteiro, quem limita a
    // resposta é a tela, e trocar peça dentro do gabinete não muda isso.
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

    /// A regra do zero, que é a razão de o módulo existir.
    #[test]
    fn sem_nenhuma_medida_o_piso_e_ausente_e_nao_zero() {
        let o = orcar(&vazia().finish(0));

        assert_eq!(o.piso_ms, None, "zero afirmaria latência nenhuma");
        assert_eq!(o.etapas_com_valor, 0);
        assert_eq!(o.etapas_totais, ETAPAS);

        // E todas as etapas continuam listadas, com o motivo de cada ausência.
        assert_eq!(o.parcelas.len(), ETAPAS);
        assert!(o.parcelas.iter().all(|p| p.ms.is_none()));
        assert!(o.parcelas.iter().all(|p| !p.origem.is_empty()));
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

        // 10 ms de quadro + 10 ms de um ciclo de tela a 100 Hz.
        assert_eq!(o.piso_ms, Some(20.0));
        assert_eq!(o.etapas_com_valor, 2);

        // E as três que ninguém mediu continuam sem número.
        assert_eq!(
            o.parcelas
                .iter()
                .filter(|p| p.qualidade == Quality::Unknown)
                .count(),
            3
        );
    }

    /// A qualidade da etapa é a da métrica que a alimentou.
    ///
    /// Uma medição de vinte minutos atrás não vira medição de agora ao entrar
    /// numa soma.
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

    /// A espera pela tela é o pior caso, e não a média de meio ciclo.
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
        // Estimativa, e não medição: depende de sincronia que não é lida.
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

    /// Empate não vira frase. Duas medidas que oscilam não sustentam um
    /// veredito decidido na terceira casa decimal.
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
