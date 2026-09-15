// O protocolo A/B: um grupo de cada vez, com medição dos dois lados
//
// A REGRA MESTRA DO PEDIDO, e é a certa:
//
//   DETECTAR → MEDIR BASELINE → APLICAR → MEDIR → COMPARAR → MANTER OU REVERTER
//
// Este módulo é o pedaço que faltava: o estado do experimento e a DECISÃO no
// fim. Medir já existia (`medicoes`), comparar já existia (`regressao`), aplicar
// já existia (`optimize_now`). O que não existia era alguém amarrando os três e
// dizendo, no fim, se aquilo valeu a pena NESTA máquina.
//
// ─────────────────────────────────────────────────────────────────────────
// POR QUE ISTO VALE MAIS QUE O ROLLBACK AUTOMÁTICO
//
// O pedido diz "se piorar, rollback automático". Concordo com a intenção e o
// produto faz isso onde o dado permite — mas a conta em `grupos.rs` mostrou que
// só três dos nove grupos dispensam reinício, e são os três que menos mexem em
// FPS. Rollback automático sozinho cobriria quase nada.
//
// O que cobre tudo é a outra metade: **testar um grupo de cada vez.** Vinte
// ajustes aplicados juntos produzem um número e nenhuma informação. Nove grupos
// testados em sequência produzem NOVE respostas, e cada uma vale para sempre
// naquela máquina — inclusive as que disserem "aqui não mudou nada", que é a
// resposta que permite parar de mexer.
//
// ─────────────────────────────────────────────────────────────────────────
// AS CINCO CONDIÇÕES PARA UM VEREDITO
//
// Sem as cinco, a resposta é "ainda não dá para dizer" — e isso é um resultado,
// não uma falha:
//
//   1. o MESMO jogo dos dois lados;
//   2. amostra confiável dos dois lados (`MedicaoAutomatica::confiavel`);
//   3. pelo menos duas medições de cada lado;
//   4. a diferença maior que a margem de ruído;
//   5. e, para REVERTER SOZINHO, o grupo não pode exigir reinício.
//
// A quinta é a que separa "posso avisar" de "posso agir". As quatro primeiras
// vivem em `regressao`, e este módulo não as reimplementa: duas implementações
// da mesma regra é como um produto passa a dizer dois números diferentes para o
// mesmo fato.

use serde::{Deserialize, Serialize};

use super::grupos::{exige_reinicio, pode_reverter_sozinho, Grupo};
use crate::modules::medicoes::MedicaoAutomatica;
use crate::modules::regressao::{comparar, Desfecho, Veredito};

/// Em que pé está o teste de um grupo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "fase")]
pub enum Fase {
    /// Ainda não se mediu nada com este grupo desligado.
    FaltaOAntes,
    /// Há medição de antes; falta aplicar e medir de novo.
    ProntoParaAplicar { amostras_antes: usize },
    /// Aplicado, esperando o cliente jogar para medir o depois.
    EsperandoODepois {
        amostras_depois: usize,
        faltam: usize,
    },
    /// Há os dois lados e um veredito.
    Concluido,
}

/// O que fazer com o resultado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decisao")]
pub enum Decisao {
    /// Ainda não há o que decidir. O texto diz o que falta.
    Esperar { falta: String },
    /// Rendeu nesta máquina. Fica.
    Manter { ganho_pct: f64 },
    /// Não mudou nada aqui. Também é resposta — e é a que deixa a pessoa parar
    /// de mexer nisso.
    NaoMudouNada,
    /// Piorou, e o produto pode desfazer sozinho.
    ReverterSozinho { queda_pct: f64 },
    /// Piorou, e o produto NÃO pode desfazer sozinho porque a comparação
    /// atravessou um reinício. Avisa e oferece o botão.
    PiorouMasNaoReverto { queda_pct: f64, porque: String },
}

/// O experimento de um grupo, do jeito que a tela precisa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Experimento {
    pub grupo: Grupo,
    pub letra: char,
    pub nome: String,
    pub descricao: String,
    pub itens: Vec<String>,
    /// Este grupo exige reiniciar. Muda o que o protocolo consegue fazer.
    pub exige_reinicio: bool,
    pub fase: Fase,
    pub decisao: Decisao,
    /// O veredito bruto, para a tela poder mostrar os números dos dois lados.
    pub veredito: Option<Veredito>,
}

/// Quantas medições confiáveis cada lado precisa. Espelha
/// `regressao::AMOSTRAS_MINIMAS` de propósito — e há teste garantindo que os
/// dois números continuem iguais, porque um protocolo que pede duas medições e
/// um comparador que exige três deixariam o cliente esperando para sempre.
pub const AMOSTRAS_POR_LADO: usize = crate::modules::regressao::AMOSTRAS_MINIMAS;

/// A DECISÃO. **Função pura.**
///
/// `aplicado` é se o grupo está aplicado agora — é ele que separa "falta medir
/// o antes" de "falta medir o depois", e sem ele as duas situações produziriam
/// a mesma frase.
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

/// O que ainda falta, em palavras. Regra de produto, e por isso tem teste.
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

/// Em que fase está o teste. **Função pura.**
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

/// Monta o experimento de um grupo. **Função pura.**
///
/// `aplicadas_quando_o_grupo_estava_desligado` é o número de mudanças que havia
/// aplicadas ANTES deste grupo entrar. É ele que separa os dois lados, e ele vem
/// do histórico e não de um palpite.
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

    /// O protocolo e o comparador precisam pedir o MESMO número de medições.
    /// Um pedindo duas e o outro exigindo três deixaria o cliente esperando
    /// para sempre por um veredito que nunca chegaria.
    #[test]
    fn o_protocolo_e_o_comparador_pedem_a_mesma_amostra() {
        assert_eq!(AMOSTRAS_POR_LADO, crate::modules::regressao::AMOSTRAS_MINIMAS);
    }

    /// O grupo de vídeo exige reiniciar, então mesmo piorando ele NÃO é
    /// desfeito sozinho — a comparação atravessou uma reinicialização.
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

    /// Um grupo que dispensa reinício e piorou É desfeito sozinho. É a única
    /// situação em que o produto age por conta própria.
    #[test]
    fn grupo_sem_reinicio_que_piora_e_desfeito_sozinho() {
        let v = veredito(Desfecho::Piorou, 2, 2, Some(-15.0));

        assert_eq!(
            decidir(&v, Grupo::Higiene, true),
            Decisao::ReverterSozinho { queda_pct: -15.0 }
        );
    }

    /// "Não mudou nada" é RESULTADO, e é o que permite a pessoa parar de mexer
    /// naquilo. Transformá-lo em "manter" seria vender ganho que não houve.
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

    /// Sem o antes, o produto diz O QUE FALTA — e não "sem dados".
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

    /// O caso do cliente que já aplicou tudo antes de o protocolo existir: há
    /// depois e não há antes. O produto não pode fingir veredito, e precisa
    /// dizer como sair disso.
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

    /// Nenhum grupo pode produzir um experimento sem itens: um grupo vazio na
    /// tela é uma etapa que o cliente espera e que não faz nada.
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
