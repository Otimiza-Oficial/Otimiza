// Autoajuste: o laço que desfaz por padrão
//
// O QUE ESTE MÓDULO É
//
// O supervisor de UMA sessão de autoajuste: medir, aplicar, medir de novo e
// decidir. As quatro peças já existem — `repeticoes` mede com incerteza,
// `orquestrador` escolhe o que aplicar, `transacao` e `changelog` aplicam e
// desfazem. O que não existia era o laço, e principalmente a regra do fim dele.
//
// A REGRA DO FIM: O PADRÃO É DESFAZER
//
// Três resultados possíveis, e dois deles terminam em desfazer:
//
//   - provou que melhorou ....... fica
//   - provou que piorou ......... desfaz
//   - NÃO PROVOU NADA ........... desfaz
//
// A terceira é a que define o módulo. Um laço automático que mantém o que não
// conseguiu provar vai, ao longo de dez iterações, deixar dez mudanças na
// máquina do cliente sem nenhuma evidência de que alguma delas serviu para
// alguma coisa — e a soma disso é um sistema que ninguém mais consegue
// explicar. "Não sei se ajudou" e "sei que não ajudou" levam ao mesmo lugar
// quando é uma máquina que não é minha.
//
// E POR QUE ISSO DIFERE DE `windows::experimento`
//
// O experimento dos nove grupos mantém o resultado neutro, e está certo: lá
// quem conduz é o cliente, um grupo de cada vez, com ele vendo cada passo — e
// um ajuste neutro que ele pediu não custa nada ficar. Aqui o laço roda em
// sequência e sem ninguém olhando cada etapa. A diferença entre os dois padrões
// é a diferença entre uma escolha acompanhada e um acúmulo silencioso.
//
// AS RECUSAS ANTES DE COMEÇAR
//
// O laço não começa sem: o que aplicar (um plano do orquestrador), e um
// protocolo de repetições que a máquina consiga cumprir. Sem os dois, a única
// resposta honesta é não começar — e dizer por quê.
//
// UMA MUDANÇA DE CADA VEZ
//
// Nunca duas. Duas mudanças juntas produzem um número e nenhuma informação:
// não há como saber qual delas rendeu, nem se uma anulou a outra. É a mesma
// lição que `windows::experimento` escreveu para os nove grupos.
//
// O QUE ELE NÃO FAZ
//
// Não aplica nada sozinho. Devolve o PRÓXIMO PASSO, e quem executa é o caminho
// que já tem diário de intenção e desfazer. Um laço que escreve no sistema sem
// passar por ali seria um laço sem caminho de volta — que é precisamente o que
// ele não pode ser, já que a metade das vezes o desfecho é desfazer.

use serde::{Deserialize, Serialize};

use super::repeticoes::{comparar, Diferenca, Resumo};

/// Quantas mudanças uma sessão pode tentar antes de parar sozinha.
///
/// Três. Não é timidez: cada mudança custa duas medições repetidas, e uma
/// sessão que tenta dez prende a máquina por bem mais de uma hora. Um laço que
/// não sabe parar é um laço que o cliente mata no meio — e aí fica uma mudança
/// aplicada e não medida, que é o pior estado possível.
pub const LIMITE_DE_MUDANCAS: usize = 3;

/// O que a sessão está pedindo agora.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Passo {
    /// Não há como começar. O motivo sai junto.
    NaoComecar { porque: String },
    /// Falta o retrato de antes.
    MedirAntes { repeticoes: usize },
    /// O antes está pronto; aplicar a mudança.
    Aplicar { mudanca: String },
    /// Aplicado; falta medir de novo, com o MESMO protocolo.
    MedirDepois { repeticoes: usize },
    /// Há os dois lados e um desfecho.
    Concluir(Desfecho),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Desfecho {
    /// Provou que melhorou, nesta máquina, com esta incerteza.
    Manter {
        /// Variação medida. `None` quando o "antes" era zero.
        pct: Option<f64>,
        /// A menor diferença que estas medições ainda distinguiriam.
        folga: f64,
    },
    Reverter(MotivoDeReverter),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MotivoDeReverter {
    /// As medições mostram que ficou pior.
    Piorou { pct: Option<f64>, folga: f64 },
    /// Os intervalos se tocam: as medições não distinguem antes de depois.
    ///
    /// NÃO é "ficou igual". É o produto dizendo que não conseguiu provar — e,
    /// num laço automático, não provar é motivo suficiente para desfazer.
    NaoProvou { sobreposicao: f64 },
    /// Não houve repetições que bastassem de algum dos lados.
    SemRepeticoes { falta: String },
}

/// Uma sessão de autoajuste, do jeito que a tela e o comando precisam.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sessao {
    /// O nome da mudança em teste. Uma só.
    pub mudanca: String,
    /// A métrica que decide, e o sentido dela.
    pub metrica: String,
    /// Para esta métrica, maior é melhor?
    ///
    /// SEM VALOR PADRÃO, e é de propósito. Quadros por segundo melhoram
    /// subindo; tempo de quadro e latência melhoram descendo. Um padrão aqui
    /// faria o laço reverter automaticamente toda melhora de latência que ele
    /// mesmo produzisse, e o defeito só apareceria em produção.
    pub maior_e_melhor: bool,
    pub repeticoes: usize,
    pub antes: Option<Resumo>,
    pub depois: Option<Resumo>,
    /// A mudança está aplicada agora.
    pub aplicada: bool,
    /// Quantas mudanças esta sessão já tentou.
    pub mudancas_tentadas: usize,
}

/// O próximo passo da sessão. **Função pura.**
pub fn proximo_passo(s: &Sessao) -> Passo {
    use super::repeticoes::REPETICOES_MINIMAS;

    if s.mudanca.trim().is_empty() {
        return Passo::NaoComecar {
            porque: "nenhuma mudança foi escolhida; o plano de ajuste é quem diz o que testar"
                .to_string(),
        };
    }

    // O limite é conferido antes de qualquer medição: gastar dois protocolos
    // para descobrir no fim que a sessão já tinha acabado seria prender a
    // máquina à toa.
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
        // Medição de depois sem medição de antes: o caso em que alguém aplicou
        // por fora. Não há comparação possível, e inventar uma referência seria
        // a pior saída.
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

/// A decisão do fim. **Função pura.**
///
/// A comparação é a de `repeticoes`: sobreposição de intervalos, e não limiar
/// em porcentagem. Aqui só se acrescenta o SENTIDO — subir é melhor em quadros
/// por segundo e pior em tempo de quadro — e o padrão de desfazer.
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

    /// Uma série apertada em torno de um valor.
    fn serie(centro: f64) -> Resumo {
        resumir(
            "fps.average",
            &[centro - 0.4, centro, centro + 0.4, centro - 0.2, centro + 0.2],
        )
        .expect("resumo")
    }

    /// Uma série larga: mede o mesmo centro com muito mais balanço.
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

    /// Aplicado por fora, sem retrato de antes: não há comparação possível.
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

    /// Ganho medido e separado do ruído: fica.
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

    /// A REGRA QUE DEFINE O MÓDULO.
    ///
    /// Os intervalos se tocam: as medições não distinguem antes de depois. Num
    /// laço automático isso não é "ficou igual, deixa ficar" — é desfazer, para
    /// a máquina não acumular mudanças que ninguém conseguiu justificar.
    #[test]
    fn nao_provar_nada_tambem_reverte() {
        // Mesmo centro, muito balanço: os intervalos se sobrepõem com folga.
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

    /// O sentido da métrica decide o desfecho, e trocá-lo inverte tudo.
    ///
    /// É o teste que existe por causa do defeito que um valor padrão criaria:
    /// tempo de quadro que CAI é melhora, e um laço que assumisse "maior é
    /// melhor" reverteria sozinho toda melhora de latência que produzisse.
    #[test]
    fn o_sentido_da_metrica_inverte_o_desfecho() {
        let antes = serie(20.0);
        let depois = serie(14.0);

        // Tempo de quadro caindo: melhora.
        assert!(matches!(
            decidir(&antes, &depois, false),
            Desfecho::Manter { .. }
        ));

        // A mesma medição lida como "maior é melhor" vira queda.
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

    /// Chegando ao fim, o passo é concluir — e o desfecho vem junto.
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
