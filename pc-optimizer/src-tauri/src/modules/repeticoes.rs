// Repetições e incerteza. O limiar fixo de 3% de `prova.rs` e `baseline.rs` descarta ganho real em máquina
// estável e vende ruído em máquina instável. Com várias medições o ruído é a dispersão medida. Regra: duas médias
// só são diferentes quando os intervalos de 95% NÃO SE TOCAM. Abaixo de três repetições a resposta é "não sei",
// nunca "não houve diferença".

use serde::{Deserialize, Serialize};

/// Com duas amostras o desvio existe e não significa nada: qualquer par sorteado tem um.
pub const REPETICOES_MINIMAS: usize = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resumo {
    pub id: String,
    /// Vai junto SEMPRE: margem sem o número de repetições esconde se veio de três ou de trinta.
    pub n: usize,
    pub media: f64,
    /// O afastamento da média é sinal de que algo aconteceu numa repetição.
    pub mediana: f64,
    pub desvio: Option<f64>,
    /// `None` abaixo de `REPETICOES_MINIMAS`: fingir margem estreita é pior que não ter margem.
    pub margem: Option<f64>,
}

impl Resumo {
    pub fn intervalo(&self) -> Option<(f64, f64)> {
        let m = self.margem?;
        Some((self.media - m, self.media + m))
    }
}

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

    // `n - 1`: dividir por `n` subestima a dispersão justamente com poucas repetições, o caso comum aqui.
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

/// Student a 95%, bicaudal. Tabela, não fórmula (seria a beta incompleta). Com três amostras é 4,3; usar o 1,96
/// da normal daria margem estreita demais, e ruído viraria ganho.
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
        // Daqui para cima a diferença para a normal é menor que 2%.
        _ => 1.96,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Diferenca {
    /// NÃO é "não houve diferença": é não ter medido o suficiente.
    SemRepeticoes { falta: String },
    Indistinguivel { delta: f64, sobreposicao: f64 },
    Real {
        delta: f64,
        pct: Option<f64>,
        /// Um ganho de 4% com folga de 3,8% é real e apertado, e quem lê precisa saber.
        folga: f64,
    },
}

/// Sobreposição, não limiar: a pergunta certa é se estas medições conseguem distinguir os dois.
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
        folga: -sobreposicao,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Protocolo {
    pub repeticoes: usize,
    pub segundos_por_repeticao: u64,
    /// A primeira roda com cache frio e shader compilando. Descartar precisa ser DITO: quem joga fora a pior amostra
    /// em silêncio está a um passo de jogar fora a que não convém.
    pub descarta_primeira: bool,
}

impl Protocolo {
    pub fn execucoes(&self) -> usize {
        self.repeticoes + usize::from(self.descarta_primeira)
    }

    /// Para a tela avisar ANTES: benchmark que prende a máquina sem avisar é cancelado no meio.
    pub fn duracao_estimada_s(&self) -> u64 {
        self.execucoes() as u64 * self.segundos_por_repeticao
    }
}

/// Ocioso: cinco curtas (estável, amostra barata). CPU, placa e disco: quatro mais longas (a carga precisa
/// estabilizar). Jogo: três de vinte segundos (mais que isso, a partida muda debaixo da medição).
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
        let r = resumir("fps", &[80.0, 90.0]).expect("duas amostras");

        assert_eq!(r.n, 2);
        assert!(r.desvio.is_some());
        assert_eq!(r.margem, None, "abaixo do mínimo de repetições");
    }

    #[test]
    fn a_margem_aperta_com_mais_repeticoes() {
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
        let r = resumir("fps", &[86.0, 87.0, 88.0, 20.0]).expect("quatro");

        assert_eq!(r.mediana, 86.5);
        assert!(r.media < 75.0, "a média é puxada pela repetição ruim");
    }

    #[test]
    fn intervalos_que_se_tocam_nao_sustentam_diferenca() {
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
        // O caso que o limiar fixo de 3% descartaria.
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
        // O caso que o limiar fixo aprovaria.
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
        // NaN de uma leitura que falhou não pode contaminar a média.
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
