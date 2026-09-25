// Autoajuste: medir, aplicar UMA mudança, medir de novo e decidir. O padrão é DESFAZER: fica só o que provou
// que melhorou; piorou ou não provou nada, desfaz. Um laço que mantém o que não provou deixa a máquina cheia de
// mudanças sem justificativa (no `windows::experimento` o neutro fica porque o cliente conduz cada passo). Não
// aplica nada sozinho: devolve o próximo passo para o caminho que tem diário e desfazer.

use serde::{Deserialize, Serialize};

use super::repeticoes::{comparar, Diferenca, Resumo};

/// Três: cada mudança custa duas medições, e um laço longo é morto no meio, deixando algo aplicado e não medido.
pub const LIMITE_DE_MUDANCAS: usize = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Passo {
    NaoComecar { porque: String },
    MedirAntes { repeticoes: usize },
    Aplicar { mudanca: String },
    MedirDepois { repeticoes: usize },
    Concluir(Desfecho),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Desfecho {
    Manter {
        pct: Option<f64>,
        folga: f64,
    },
    Reverter(MotivoDeReverter),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MotivoDeReverter {
    Piorou { pct: Option<f64>, folga: f64 },
    /// NÃO é "ficou igual": é não ter provado, e no laço automático isso desfaz.
    NaoProvou { sobreposicao: f64 },
    SemRepeticoes { falta: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sessao {
    pub mudanca: String,
    pub metrica: String,
    /// SEM valor padrão: FPS melhora subindo, tempo de quadro descendo. Um padrão faria o laço desfazer toda melhora
    /// de latência que ele produzisse.
    pub maior_e_melhor: bool,
    pub repeticoes: usize,
    pub antes: Option<Resumo>,
    pub depois: Option<Resumo>,
    pub aplicada: bool,
    pub mudancas_tentadas: usize,
}

pub fn proximo_passo(s: &Sessao) -> Passo {
    use super::repeticoes::REPETICOES_MINIMAS;

    if s.mudanca.trim().is_empty() {
        return Passo::NaoComecar {
            porque: "nenhuma mudança foi escolhida; o plano de ajuste é quem diz o que testar"
                .to_string(),
        };
    }

    // Conferido antes de medir: gastar dois protocolos para descobrir que a sessão acabou prenderia a máquina à toa.
    if s.mudancas_tentadas >= LIMITE_DE_MUDANCAS {
        return Passo::NaoComecar {
            porque: format!(
                "esta sessão já testou {LIMITE_DE_MUDANCAS} mudanças; medir mais prenderia a \
                 máquina por tempo demais de uma vez"
            ),
        };
    }

    if s.repeticoes < REPETICOES_MINIMAS {
        return Passo::NaoComecar {
            porque: format!(
                "o protocolo pede {} repetições e esta sessão tem {}; abaixo disso não há \
                 incerteza medida, e sem incerteza não dá para separar ganho de variação normal",
                REPETICOES_MINIMAS, s.repeticoes
            ),
        };
    }

    match (&s.antes, s.aplicada, &s.depois) {
        (None, false, _) => Passo::MedirAntes {
            repeticoes: s.repeticoes,
        },
        // Aplicado por fora, sem antes: inventar uma referência seria a pior saída.
        (None, true, _) => Passo::NaoComecar {
            porque: "a mudança já está aplicada e não há retrato de antes; desfaça, meça, e \
                     aplique de novo — sem o antes não existe comparação"
                .to_string(),
        },
        (Some(_), false, _) => Passo::Aplicar {
            mudanca: s.mudanca.clone(),
        },
        (Some(_), true, None) => Passo::MedirDepois {
            repeticoes: s.repeticoes,
        },
        (Some(antes), true, Some(depois)) => {
            Passo::Concluir(decidir(antes, depois, s.maior_e_melhor))
        }
    }
}

/// A comparação é a de `repeticoes` (intervalos que não se tocam); aqui só entra o sentido e o padrão de desfazer.
pub fn decidir(antes: &Resumo, depois: &Resumo, maior_e_melhor: bool) -> Desfecho {
    match comparar(antes, depois) {
        Diferenca::SemRepeticoes { falta } => {
            Desfecho::Reverter(MotivoDeReverter::SemRepeticoes { falta })
        }
        Diferenca::Indistinguivel { sobreposicao, .. } => {
            Desfecho::Reverter(MotivoDeReverter::NaoProvou { sobreposicao })
        }
        Diferenca::Real { delta, pct, folga } => {
            let melhorou = if maior_e_melhor { delta > 0.0 } else { delta < 0.0 };

            if melhorou {
                Desfecho::Manter { pct, folga }
            } else {
                Desfecho::Reverter(MotivoDeReverter::Piorou { pct, folga })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::repeticoes::{resumir, REPETICOES_MINIMAS};

    fn sessao() -> Sessao {
        Sessao {
            mudanca: "plano de energia Otimiza".to_string(),
            metrica: "fps.average".to_string(),
            maior_e_melhor: true,
            repeticoes: REPETICOES_MINIMAS,
            antes: None,
            depois: None,
            aplicada: false,
            mudancas_tentadas: 0,
        }
    }

    fn serie(centro: f64) -> Resumo {
        resumir(
            "fps.average",
            &[centro - 0.4, centro, centro + 0.4, centro - 0.2, centro + 0.2],
        )
        .expect("resumo")
    }

    fn serie_larga(centro: f64) -> Resumo {
        resumir(
            "fps.average",
            &[
                centro - 18.0,
                centro + 16.0,
                centro - 12.0,
                centro + 14.0,
                centro,
            ],
        )
        .expect("resumo")
    }

    #[test]
    fn a_sessao_comeca_medindo_o_antes() {
        assert_eq!(
            proximo_passo(&sessao()),
            Passo::MedirAntes {
                repeticoes: REPETICOES_MINIMAS
            }
        );
    }

    #[test]
    fn com_o_antes_pronto_o_passo_e_aplicar() {
        let mut s = sessao();
        s.antes = Some(serie(100.0));

        assert_eq!(
            proximo_passo(&s),
            Passo::Aplicar {
                mudanca: s.mudanca.clone()
            }
        );
    }

    #[test]
    fn aplicada_sem_o_depois_pede_a_segunda_medicao() {
        let mut s = sessao();
        s.antes = Some(serie(100.0));
        s.aplicada = true;

        assert_eq!(
            proximo_passo(&s),
            Passo::MedirDepois {
                repeticoes: REPETICOES_MINIMAS
            }
        );
    }

    #[test]
    fn sem_o_antes_e_ja_aplicada_a_sessao_recusa() {
        let mut s = sessao();
        s.aplicada = true;

        assert!(matches!(proximo_passo(&s), Passo::NaoComecar { .. }));
    }

    #[test]
    fn protocolo_curto_demais_nao_comeca() {
        let mut s = sessao();
        s.repeticoes = REPETICOES_MINIMAS - 1;

        let Passo::NaoComecar { porque } = proximo_passo(&s) else {
            panic!("devia recusar");
        };
        assert!(porque.contains("incerteza"), "{porque}");
    }

    #[test]
    fn o_laco_sabe_parar() {
        let mut s = sessao();
        s.mudancas_tentadas = LIMITE_DE_MUDANCAS;

        assert!(matches!(proximo_passo(&s), Passo::NaoComecar { .. }));
    }

    #[test]
    fn ganho_provado_mantem() {
        let d = decidir(&serie(100.0), &serie(112.0), true);

        assert!(matches!(d, Desfecho::Manter { .. }), "{d:?}");
        let Desfecho::Manter { pct, .. } = d else {
            unreachable!()
        };
        assert!(pct.expect("pct") > 10.0);
    }

    #[test]
    fn queda_provada_reverte() {
        let d = decidir(&serie(100.0), &serie(88.0), true);

        assert!(matches!(
            d,
            Desfecho::Reverter(MotivoDeReverter::Piorou { .. })
        ));
    }

    #[test]
    fn nao_provar_nada_tambem_reverte() {
        let d = decidir(&serie_larga(100.0), &serie_larga(103.0), true);

        assert!(
            matches!(d, Desfecho::Reverter(MotivoDeReverter::NaoProvou { .. })),
            "{d:?}"
        );
    }

    #[test]
    fn sem_repeticoes_reverte_e_diz_o_que_faltou() {
        let magro = resumir("fps.average", &[100.0]).expect("resumo");
        let d = decidir(&magro, &serie(120.0), true);

        let Desfecho::Reverter(MotivoDeReverter::SemRepeticoes { falta }) = d else {
            panic!("devia reverter por falta de repetição: {d:?}");
        };
        assert!(!falta.is_empty());
    }

    #[test]
    fn o_sentido_da_metrica_inverte_o_desfecho() {
        let antes = serie(20.0);
        let depois = serie(14.0);

        assert!(matches!(
            decidir(&antes, &depois, false),
            Desfecho::Manter { .. }
        ));

        assert!(matches!(
            decidir(&antes, &depois, true),
            Desfecho::Reverter(MotivoDeReverter::Piorou { .. })
        ));
    }

    #[test]
    fn sessao_sem_mudanca_escolhida_nao_comeca() {
        let mut s = sessao();
        s.mudanca = "   ".to_string();

        assert!(matches!(proximo_passo(&s), Passo::NaoComecar { .. }));
    }

    #[test]
    fn com_os_dois_lados_o_passo_e_concluir() {
        let mut s = sessao();
        s.antes = Some(serie(100.0));
        s.depois = Some(serie(115.0));
        s.aplicada = true;

        let Passo::Concluir(d) = proximo_passo(&s) else {
            panic!("devia concluir");
        };
        assert!(matches!(d, Desfecho::Manter { .. }), "{d:?}");
    }
}
