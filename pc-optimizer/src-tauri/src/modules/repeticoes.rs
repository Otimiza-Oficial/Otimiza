// Repetições e incerteza: quanto vale um número medido uma vez
//
// POR QUE ISTO EXISTE
//
// `prova.rs` e `baseline.rs` decidem se uma diferença é ganho comparando-a com
// 3%. O número é bem escolhido e está documentado nos dois: duas medições
// seguidas, sem mexer em nada, variam nessa ordem de grandeza.
//
// Mas 3% é um palpite sobre TODA máquina e TODA métrica. Numa máquina estável
// o ruído real é menor, e o produto está descartando ganhos verdadeiros. Numa
// máquina instável ele é muito maior, e o produto está vendendo ruído como
// ganho — exatamente o que ele existe para não fazer.
//
// Medindo VÁRIAS vezes, o ruído deixa de ser palpite: ele é a dispersão das
// próprias amostras, nesta máquina, nesta métrica, hoje.
//
// A REGRA QUE SUBSTITUI O LIMIAR FIXO
//
// Duas médias com margem de erro só são diferentes quando os intervalos NÃO SE
// TOCAM. Dizer que 87 ± 6 é maior que 84 ± 5 é afirmar uma diferença que as
// próprias medições não sustentam — os dois valores cabem no mesmo lugar.
//
// UMA REPETIÇÃO NÃO TEM INCERTEZA
//
// Com uma amostra não há dispersão a calcular, e com duas a estimativa é fraca
// demais para valer. Abaixo de três repetições a resposta é "não sei" — nunca
// "não houve diferença". São coisas diferentes, e confundi-las é como se
// aprova uma mudança que não fez nada.

use serde::{Deserialize, Serialize};

/// Mínimo de repetições para haver incerteza calculável.
///
/// Três. Com duas amostras o desvio existe matematicamente e não significa
/// nada: qualquer par de números tem um desvio, inclusive dois números
/// sorteados.
pub const REPETICOES_MINIMAS: usize = 3;

/// Uma métrica medida várias vezes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resumo {
    pub id: String,
    /// Quantas amostras entraram. Vai junto SEMPRE: uma margem de erro sem o
    /// número de repetições esconde se ela veio de três medições ou de trinta.
    pub n: usize,
    pub media: f64,
    /// O do meio. Diferente da média quando há uma repetição fora da curva, e
    /// o afastamento entre as duas é sinal de que algo aconteceu numa delas.
    pub mediana: f64,
    /// Desvio padrão amostral. `None` com menos de duas amostras.
    pub desvio: Option<f64>,
    /// Metade da largura do intervalo de confiança de 95%.
    ///
    /// `None` abaixo de `REPETICOES_MINIMAS`: sem repetições que bastem, o
    /// produto não sabe a incerteza, e fingir uma margem estreita seria pior
    /// que não ter margem nenhuma.
    pub margem: Option<f64>,
}

impl Resumo {
    /// O intervalo, quando há margem.
    pub fn intervalo(&self) -> Option<(f64, f64)> {
        let m = self.margem?;
        Some((self.media - m, self.media + m))
    }
}

/// Resume uma série de amostras da MESMA métrica.
///
/// **Função pura.** Lista vazia devolve `None`: não existe resumo de nada.
pub fn resumir(id: &str, amostras: &[f64]) -> Option<Resumo> {
    let uteis: Vec<f64> = amostras.iter().copied().filter(|v| v.is_finite()).collect();

    if uteis.is_empty() {
        return None;
    }

    let n = uteis.len();
    let media = uteis.iter().sum::<f64>() / n as f64;

    let mut ordenadas = uteis.clone();
    ordenadas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mediana = if n.is_multiple_of(2) {
        (ordenadas[n / 2 - 1] + ordenadas[n / 2]) / 2.0
    } else {
        ordenadas[n / 2]
    };

    // Desvio AMOSTRAL, com `n - 1` no denominador. Estas repetições são uma
    // amostra do que a máquina faz, não a população inteira dela; dividir por
    // `n` subestimaria a dispersão justamente no caso de poucas repetições,
    // que é o caso comum aqui.
    let desvio = (n >= 2).then(|| {
        let soma = uteis.iter().map(|v| (v - media).powi(2)).sum::<f64>();
        (soma / (n - 1) as f64).sqrt()
    });

    let margem = match desvio {
        Some(s) if n >= REPETICOES_MINIMAS => Some(t_95(n - 1) * s / (n as f64).sqrt()),
        _ => None,
    };

    Some(Resumo {
        id: id.to_string(),
        n,
        media,
        mediana,
        desvio,
        margem,
    })
}

/// Valor crítico de Student a 95%, bicaudal, por graus de liberdade.
///
/// Tabela e não fórmula: a fórmula exigiria a função beta incompleta, e o
/// produto precisa de nove valores. Com poucas repetições este número é MUITO
/// maior que os 1,96 da distribuição normal — com três amostras ele é 4,3 — e
/// usar 1,96 aí produziria uma margem estreita demais, que é o erro que faz
/// ruído virar ganho.
fn t_95(graus: usize) -> f64 {
    match graus {
        0 | 1 => 12.706,
        2 => 4.303,
        3 => 3.182,
        4 => 2.776,
        5 => 2.571,
        6 => 2.447,
        7 => 2.365,
        8 => 2.306,
        9 => 2.262,
        10..=14 => 2.145,
        15..=19 => 2.093,
        20..=29 => 2.045,
        // Daqui para cima a diferença para a normal é menor que 2%, e o
        // produto nunca vai rodar trinta repetições de um benchmark de jogo.
        _ => 1.96,
    }
}

/// O que a comparação de duas séries permite afirmar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Diferenca {
    /// Não há repetições que bastem de pelo menos um dos lados.
    ///
    /// NÃO é "não houve diferença". É o produto dizendo que não mediu o
    /// suficiente para responder.
    SemRepeticoes { falta: String },
    /// Os intervalos se tocam: as medições não sustentam uma diferença.
    Indistinguivel { delta: f64, sobreposicao: f64 },
    /// Os intervalos não se tocam.
    Real {
        delta: f64,
        /// Variação em %. `None` quando o "antes" era zero.
        pct: Option<f64>,
        /// A menor diferença que estas medições ainda distinguiriam.
        ///
        /// É a honestidade do número: um ganho de 4% com folga mínima de 3,8%
        /// é real e apertado, e quem lê precisa saber disso.
        folga: f64,
    },
}

/// Compara duas séries resumidas.
///
/// A regra é a sobreposição dos intervalos, e não um limiar em porcentagem.
/// Um limiar fixo pergunta "a diferença é grande?"; a sobreposição pergunta
/// "estas medições conseguem distinguir os dois?", que é a pergunta certa.
pub fn comparar(antes: &Resumo, depois: &Resumo) -> Diferenca {
    let (Some((a_min, a_max)), Some((d_min, d_max))) = (antes.intervalo(), depois.intervalo())
    else {
        let quem = match (antes.margem.is_some(), depois.margem.is_some()) {
            (false, false) => "as duas medições",
            (false, true) => "a medição de antes",
            _ => "a medição de depois",
        };
        return Diferenca::SemRepeticoes {
            falta: format!(
                "{quem} não tem as {REPETICOES_MINIMAS} repetições mínimas para haver margem de erro"
            ),
        };
    };

    let delta = depois.media - antes.media;

    // Sobreposição: o quanto os dois intervalos dividem. Zero ou menos
    // significa que eles não se tocam.
    let sobreposicao = a_max.min(d_max) - a_min.max(d_min);

    if sobreposicao >= 0.0 {
        return Diferenca::Indistinguivel {
            delta,
            sobreposicao,
        };
    }

    Diferenca::Real {
        delta,
        pct: (antes.media != 0.0).then(|| delta / antes.media.abs() * 100.0),
        // A folga é o tamanho do vão entre os intervalos: `-sobreposicao`.
        folga: -sobreposicao,
    }
}

/// Como medir, por tipo de carga.
///
/// O prompt do produto pede protocolo por carga, e a razão é concreta: medir
/// uma máquina ociosa e medir uma partida não pedem o mesmo cuidado nem o
/// mesmo tempo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Protocolo {
    pub repeticoes: usize,
    pub segundos_por_repeticao: u64,
    /// Descartar a primeira repetição.
    ///
    /// A primeira roda com cache frio, shader compilando e o Windows ainda
    /// acomodando o que acabou de abrir. Ela é sistematicamente pior que as
    /// outras, e mantê-la não acrescenta informação — acrescenta um viés.
    ///
    /// DESCARTAR PRECISA SER DITO. Um produto que joga fora a pior amostra em
    /// silêncio está a um passo de jogar fora a que não convém.
    pub descarta_primeira: bool,
}

impl Protocolo {
    /// Quantas repetições precisam ser EXECUTADAS para sobrarem as pedidas.
    pub fn execucoes(&self) -> usize {
        self.repeticoes + usize::from(self.descarta_primeira)
    }

    /// Quanto tempo o cliente vai esperar, em segundos.
    ///
    /// Existe para a tela poder dizer isso ANTES de começar. Um benchmark que
    /// prende a máquina por três minutos sem avisar é um benchmark que o
    /// cliente cancela no meio.
    pub fn duracao_estimada_s(&self) -> u64 {
        self.execucoes() as u64 * self.segundos_por_repeticao
    }
}

/// O protocolo de cada carga.
///
/// Os números saem do que cada medida precisa, e não de um padrão redondo:
///
/// - OCIOSO e ÁREA DE TRABALHO: cinco repetições curtas. A máquina está
///   estável, então a dispersão é pequena e mais amostras baratas apertam a
///   margem de graça.
/// - CPU, PLACA e DISCO: quatro repetições um pouco mais longas. A carga
///   precisa de tempo para estabilizar antes de a leitura valer.
/// - JOGO: três repetições de vinte segundos, que é o mesmo da medição
///   automática de quadros. Mais que isso vira tempo demais com o cliente
///   parado esperando, e a partida muda debaixo da medição.
pub fn protocolo(perfil: super::baseline::Perfil) -> Protocolo {
    use super::baseline::Perfil;

    match perfil {
        Perfil::Ocioso | Perfil::AreaDeTrabalho => Protocolo {
            repeticoes: 5,
            segundos_por_repeticao: 3,
            descarta_primeira: false,
        },
        Perfil::Cpu | Perfil::Gpu | Perfil::Disco => Protocolo {
            repeticoes: 4,
            segundos_por_repeticao: 8,
            descarta_primeira: true,
        },
        Perfil::Jogo => Protocolo {
            repeticoes: 3,
            segundos_por_repeticao: 20,
            descarta_primeira: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::baseline::Perfil;

    #[test]
    fn uma_amostra_nao_tem_incerteza() {
        let r = resumir("fps", &[87.0]).expect("uma amostra é um resumo");

        assert_eq!(r.n, 1);
        assert_eq!(r.media, 87.0);
        assert_eq!(r.desvio, None, "não há dispersão de um número só");
        assert_eq!(r.margem, None);
        assert_eq!(r.intervalo(), None);
    }

    #[test]
    fn duas_amostras_tem_desvio_mas_nao_margem() {
        // O desvio de dois números existe e não significa nada: qualquer par
        // sorteado tem um.
        let r = resumir("fps", &[80.0, 90.0]).expect("duas amostras");

        assert_eq!(r.n, 2);
        assert!(r.desvio.is_some());
        assert_eq!(r.margem, None, "abaixo do mínimo de repetições");
    }

    #[test]
    fn a_margem_aperta_com_mais_repeticoes() {
        // A MESMA dispersão, medida mais vezes, dá uma margem menor. É o que
        // torna repetir útil em vez de só demorado.
        let poucas = resumir("fps", &[80.0, 85.0, 90.0]).expect("três");
        let muitas = resumir(
            "fps",
            &[80.0, 85.0, 90.0, 80.0, 85.0, 90.0, 80.0, 85.0, 90.0],
        )
        .expect("nove");

        assert!(
            muitas.margem.unwrap() < poucas.margem.unwrap(),
            "nove repetições: {:?}, três: {:?}",
            muitas.margem,
            poucas.margem
        );
    }

    #[test]
    fn media_e_mediana_se_afastam_com_uma_repeticao_fora_da_curva() {
        // O afastamento é sinal de que algo aconteceu numa das repetições —
        // uma atualização subindo no fundo, por exemplo.
        let r = resumir("fps", &[86.0, 87.0, 88.0, 20.0]).expect("quatro");

        assert_eq!(r.mediana, 86.5);
        assert!(r.media < 75.0, "a média é puxada pela repetição ruim");
    }

    #[test]
    fn intervalos_que_se_tocam_nao_sustentam_diferenca() {
        // 84 e 87, com dispersão larga. A diferença existe nos números e as
        // medições não conseguem distinguir os dois.
        let antes = resumir("fps", &[80.0, 84.0, 88.0]).expect("três");
        let depois = resumir("fps", &[83.0, 87.0, 91.0]).expect("três");

        match comparar(&antes, &depois) {
            Diferenca::Indistinguivel {
                delta,
                sobreposicao,
            } => {
                assert!((delta - 3.0).abs() < 0.01);
                assert!(sobreposicao > 0.0);
            }
            outro => panic!("esperava indistinguível, veio {outro:?}"),
        }
    }

    #[test]
    fn intervalos_separados_sustentam_diferenca() {
        // O mesmo ganho de 3 quadros, agora com medições apertadas. Aqui ele
        // é real — e é o caso que o limiar fixo de 3% descartaria, porque
        // 3/84 é 3,6% mas a máquina mostrou que sabe medir melhor que isso.
        let antes = resumir("fps", &[84.0, 84.1, 83.9, 84.0]).expect("quatro");
        let depois = resumir("fps", &[87.0, 87.1, 86.9, 87.0]).expect("quatro");

        match comparar(&antes, &depois) {
            Diferenca::Real { delta, pct, folga } => {
                assert!((delta - 3.0).abs() < 0.01);
                assert!((pct.unwrap() - 3.57).abs() < 0.1);
                assert!(folga > 0.0, "a folga diz o quanto sobrou do vão");
            }
            outro => panic!("esperava diferença real, veio {outro:?}"),
        }
    }

    #[test]
    fn maquina_instavel_nao_vende_ruido_como_ganho() {
        // O caso que o limiar fixo erra do outro lado: 10% de ganho aparente
        // numa máquina que varia 30% entre repetições. Os 3% aprovariam.
        let antes = resumir("fps", &[60.0, 90.0, 75.0, 50.0]).expect("quatro");
        let depois = resumir("fps", &[70.0, 95.0, 80.0, 60.0]).expect("quatro");

        assert!(
            matches!(comparar(&antes, &depois), Diferenca::Indistinguivel { .. }),
            "com esta dispersão, 10% não se distingue de acaso"
        );
    }

    #[test]
    fn sem_repeticoes_a_resposta_e_nao_sei_e_nao_nao_mudou() {
        let uma = resumir("fps", &[84.0]).expect("uma");
        let varias = resumir("fps", &[87.0, 87.1, 86.9, 87.0]).expect("quatro");

        match comparar(&uma, &varias) {
            Diferenca::SemRepeticoes { falta } => {
                assert!(falta.contains("antes"), "{falta}");
            }
            outro => panic!("esperava sem repetições, veio {outro:?}"),
        }

        // E o espelho: falta do outro lado.
        match comparar(&varias, &uma) {
            Diferenca::SemRepeticoes { falta } => assert!(falta.contains("depois"), "{falta}"),
            outro => panic!("esperava sem repetições, veio {outro:?}"),
        }
    }

    #[test]
    fn nao_existe_porcentagem_sobre_zero() {
        let antes = resumir("engasgos", &[0.0, 0.0, 0.0, 0.0]).expect("quatro");
        let depois = resumir("engasgos", &[5.0, 5.1, 4.9, 5.0]).expect("quatro");

        match comparar(&antes, &depois) {
            Diferenca::Real { pct, delta, .. } => {
                assert_eq!(pct, None);
                assert!((delta - 5.0).abs() < 0.01, "o absoluto continua lá");
            }
            outro => panic!("esperava diferença real, veio {outro:?}"),
        }
    }

    #[test]
    fn amostra_invalida_nao_entra_na_conta() {
        // NaN de uma leitura que falhou não pode contaminar a média inteira.
        let r = resumir("fps", &[86.0, f64::NAN, 88.0, f64::INFINITY]).expect("duas boas");

        assert_eq!(r.n, 2);
        assert_eq!(r.media, 87.0);
    }

    #[test]
    fn nada_medido_nao_vira_resumo() {
        assert_eq!(resumir("fps", &[]), None);
        assert_eq!(resumir("fps", &[f64::NAN]), None);
    }

    #[test]
    fn o_protocolo_de_jogo_e_mais_curto_e_descarta_o_aquecimento() {
        let jogo = protocolo(Perfil::Jogo);
        let ocioso = protocolo(Perfil::Ocioso);

        assert!(
            jogo.descarta_primeira,
            "a primeira roda com shader compilando"
        );
        assert!(
            !ocioso.descarta_primeira,
            "máquina parada não tem aquecimento"
        );

        assert!(jogo.repeticoes < ocioso.repeticoes);
        assert_eq!(jogo.execucoes(), 4, "três úteis mais a descartada");
        assert_eq!(jogo.duracao_estimada_s(), 80);
    }

    #[test]
    fn todo_protocolo_alcanca_o_minimo_de_repeticoes() {
        // Um protocolo que pedisse menos que o mínimo produziria medições sem
        // margem de erro — e todo o módulo existe para que isso não aconteça.
        for perfil in [
            Perfil::Ocioso,
            Perfil::AreaDeTrabalho,
            Perfil::Cpu,
            Perfil::Gpu,
            Perfil::Jogo,
            Perfil::Disco,
        ] {
            let p = protocolo(perfil);
            assert!(
                p.repeticoes >= REPETICOES_MINIMAS,
                "{perfil:?} pede {} repetições",
                p.repeticoes
            );
        }
    }
}
