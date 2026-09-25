// O protocolo A/B: medir, aplicar UM grupo, medir, comparar, manter ou reverter. Vinte ajustes juntos dão um
// número e nenhuma informação; nove grupos em sequência dão nove respostas, inclusive "aqui não mudou nada".
// Veredito só com o mesmo jogo, amostra confiável, duas medições de cada lado e diferença acima do ruído (regras
// de `regressao`, não reimplementadas); reverter sozinho só se o grupo não exigir reinício.

use serde::{Deserialize, Serialize};

use super::grupos::{exige_reinicio, pode_reverter_sozinho, Grupo};
use crate::modules::medicoes::MedicaoAutomatica;
use crate::modules::regressao::{comparar, Desfecho, Veredito};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "fase")]
pub enum Fase {
    FaltaOAntes,
    ProntoParaAplicar { amostras_antes: usize },
    EsperandoODepois {
        amostras_depois: usize,
        faltam: usize,
    },
    Concluido,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decisao")]
pub enum Decisao {
    Esperar { falta: String },
    Manter { ganho_pct: f64 },
    /// Também é resposta: deixa a pessoa parar de mexer nisso.
    NaoMudouNada,
    ReverterSozinho { queda_pct: f64 },
    /// Atravessou um reinício: avisa e oferece o botão.
    PiorouMasNaoReverto { queda_pct: f64, porque: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Experimento {
    pub grupo: Grupo,
    pub letra: char,
    pub nome: String,
    pub descricao: String,
    pub itens: Vec<String>,
    pub exige_reinicio: bool,
    pub fase: Fase,
    pub decisao: Decisao,
    pub veredito: Option<Veredito>,
}

/// Espelha `regressao::AMOSTRAS_MINIMAS`, com teste: números diferentes deixariam o cliente esperando para sempre.
pub const AMOSTRAS_POR_LADO: usize = crate::modules::regressao::AMOSTRAS_MINIMAS;

/// `aplicado` separa "falta medir o antes" de "falta medir o depois".
pub fn decidir(veredito: &Veredito, grupo: Grupo, aplicado: bool) -> Decisao {
    match veredito.desfecho {
        Desfecho::SemAmostra => Decisao::Esperar {
            falta: falta_o_que(veredito, aplicado),
        },
        Desfecho::Melhorou => Decisao::Manter {
            ganho_pct: veredito.variacao_fps_pct.unwrap_or(0.0),
        },
        Desfecho::Igual => Decisao::NaoMudouNada,
        Desfecho::Piorou | Desfecho::PiorouMuito => {
            let queda_pct = veredito.variacao_fps_pct.unwrap_or(0.0);

            if pode_reverter_sozinho(grupo) {
                Decisao::ReverterSozinho { queda_pct }
            } else {
                Decisao::PiorouMasNaoReverto {
                    queda_pct,
                    porque: "Este grupo exige reiniciar o computador, então a medição de \
                             antes e a de depois caíram em sessões diferentes — outra \
                             ocupação de memória, outros programas abertos, outro lugar do \
                             mapa. Isso basta para avisar, e não basta para o Otimiza \
                             desfazer sozinho o que você pediu. O botão está aqui do lado."
                        .to_string(),
                }
            }
        }
    }
}

fn falta_o_que(veredito: &Veredito, aplicado: bool) -> String {
    let conta = |lado: &Option<crate::modules::regressao::Lado>| {
        lado.as_ref().map(|l| l.amostras).unwrap_or(0)
    };

    let antes = conta(&veredito.antes);
    let depois = conta(&veredito.depois);

    if !aplicado {
        if antes >= AMOSTRAS_POR_LADO {
            return "A medição de antes está pronta. Aplique este grupo e jogue de novo."
                .to_string();
        }

        return format!(
            "Faltam {} medições com este grupo DESLIGADO. O Otimiza mede sozinho depois de \
             alguns minutos de jogo, e repete a cada vinte — é só jogar.",
            AMOSTRAS_POR_LADO.saturating_sub(antes)
        );
    }

    if depois >= AMOSTRAS_POR_LADO && antes < AMOSTRAS_POR_LADO {
        return "Este grupo já está aplicado e medido, mas não há medição de ANTES para \
                comparar. Desfaça o grupo, jogue um pouco, e depois aplique de novo — ou \
                siga sem comparação, que aí o Otimiza não vai saber dizer se rendeu."
            .to_string();
    }

    format!(
        "Faltam {} medições com este grupo aplicado. Jogue mais um pouco.",
        AMOSTRAS_POR_LADO.saturating_sub(depois)
    )
}

pub fn fase(veredito: &Veredito, aplicado: bool) -> Fase {
    let conta = |lado: &Option<crate::modules::regressao::Lado>| {
        lado.as_ref().map(|l| l.amostras).unwrap_or(0)
    };

    let antes = conta(&veredito.antes);
    let depois = conta(&veredito.depois);

    if antes >= AMOSTRAS_POR_LADO && depois >= AMOSTRAS_POR_LADO {
        return Fase::Concluido;
    }

    if !aplicado {
        return if antes >= AMOSTRAS_POR_LADO {
            Fase::ProntoParaAplicar { amostras_antes: antes }
        } else {
            Fase::FaltaOAntes
        };
    }

    Fase::EsperandoODepois {
        amostras_depois: depois,
        faltam: AMOSTRAS_POR_LADO.saturating_sub(depois),
    }
}

/// `aplicadas_quando_o_grupo_estava_desligado` separa os dois lados, e vem do histórico.
pub fn montar(
    grupo: Grupo,
    jogo: &str,
    medicoes: &[MedicaoAutomatica],
    aplicadas_hoje: usize,
    aplicado: bool,
) -> Experimento {
    let veredito = comparar(jogo, medicoes, aplicadas_hoje);

    Experimento {
        grupo,
        letra: grupo.letra(),
        nome: grupo.nome().to_string(),
        descricao: grupo.descricao().to_string(),
        itens: super::grupos::itens_do_grupo(grupo)
            .iter()
            .map(|s| s.to_string())
            .collect(),
        exige_reinicio: exige_reinicio(grupo),
        fase: fase(&veredito, aplicado),
        decisao: decidir(&veredito, grupo, aplicado),
        veredito: Some(veredito),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::regressao::Lado;

    fn veredito(desfecho: Desfecho, antes: usize, depois: usize, pct: Option<f64>) -> Veredito {
        let lado = |n: usize| {
            (n > 0).then(|| Lado { fps: 100.0, low_1pct: 70.0, amostras: n })
        };

        Veredito {
            jogo: "FiveM.exe".to_string(),
            desfecho,
            antes: lado(antes),
            depois: lado(depois),
            variacao_fps_pct: pct,
            variacao_low_pct: pct,
        }
    }

    #[test]
    fn o_protocolo_e_o_comparador_pedem_a_mesma_amostra() {
        assert_eq!(AMOSTRAS_POR_LADO, crate::modules::regressao::AMOSTRAS_MINIMAS);
    }

    #[test]
    fn grupo_que_exige_reinicio_avisa_mas_nao_reverte_sozinho() {
        let v = veredito(Desfecho::PiorouMuito, 2, 2, Some(-40.0));
        let d = decidir(&v, Grupo::Video, true);

        match d {
            Decisao::PiorouMasNaoReverto { queda_pct, porque } => {
                assert_eq!(queda_pct, -40.0);
                assert!(porque.contains("sessões diferentes"));
                assert!(porque.contains("botão"), "precisa oferecer o caminho manual");
            }
            outro => panic!("era para avisar sem reverter, veio {outro:?}"),
        }
    }

    #[test]
    fn grupo_sem_reinicio_que_piora_e_desfeito_sozinho() {
        let v = veredito(Desfecho::Piorou, 2, 2, Some(-15.0));

        assert_eq!(
            decidir(&v, Grupo::Higiene, true),
            Decisao::ReverterSozinho { queda_pct: -15.0 }
        );
    }

    #[test]
    fn empate_e_resultado_e_nao_ganho() {
        let v = veredito(Desfecho::Igual, 2, 2, Some(1.0));
        assert_eq!(decidir(&v, Grupo::Fundo, true), Decisao::NaoMudouNada);
    }

    #[test]
    fn ganho_medido_e_ganho() {
        let v = veredito(Desfecho::Melhorou, 2, 2, Some(22.0));
        assert_eq!(
            decidir(&v, Grupo::Energia, true),
            Decisao::Manter { ganho_pct: 22.0 }
        );
    }

    #[test]
    fn sem_o_antes_o_texto_diz_o_que_fazer() {
        let v = veredito(Desfecho::SemAmostra, 0, 0, None);

        let Decisao::Esperar { falta } = decidir(&v, Grupo::Energia, false) else {
            panic!("era para esperar");
        };

        assert!(falta.contains("DESLIGADO"), "falta dizer que é com o grupo desligado");
        assert!(falta.contains("jogar"), "falta dizer o que a pessoa faz: {falta}");
    }

    #[test]
    fn com_o_antes_pronto_o_texto_manda_aplicar() {
        let v = veredito(Desfecho::SemAmostra, 2, 0, None);

        let Decisao::Esperar { falta } = decidir(&v, Grupo::Energia, false) else {
            panic!("era para esperar");
        };

        assert!(falta.contains("Aplique este grupo"));
    }

    /// Quem aplicou tudo antes do protocolo existir tem depois e não tem antes: não se finge veredito.
    #[test]
    fn com_o_depois_mas_sem_o_antes_o_produto_explica_a_saida() {
        let v = veredito(Desfecho::SemAmostra, 0, 3, None);

        let Decisao::Esperar { falta } = decidir(&v, Grupo::Energia, true) else {
            panic!("era para esperar");
        };

        assert!(falta.contains("não há medição de ANTES"));
        assert!(falta.contains("Desfaça o grupo"));
    }

    #[test]
    fn as_fases_seguem_a_ordem_do_protocolo() {
        assert_eq!(fase(&veredito(Desfecho::SemAmostra, 0, 0, None), false), Fase::FaltaOAntes);

        assert_eq!(
            fase(&veredito(Desfecho::SemAmostra, 2, 0, None), false),
            Fase::ProntoParaAplicar { amostras_antes: 2 }
        );

        assert_eq!(
            fase(&veredito(Desfecho::SemAmostra, 2, 1, None), true),
            Fase::EsperandoODepois { amostras_depois: 1, faltam: 1 }
        );

        assert_eq!(fase(&veredito(Desfecho::Igual, 2, 2, Some(0.0)), true), Fase::Concluido);
    }

    #[test]
    fn todo_grupo_monta_um_experimento_com_itens() {
        for grupo in Grupo::TODOS {
            let e = montar(*grupo, "FiveM.exe", &[], 0, false);

            assert!(!e.itens.is_empty(), "grupo {} sem itens", e.letra);
            assert!(!e.descricao.is_empty());
            assert_eq!(e.fase, Fase::FaltaOAntes);
        }
    }
}
