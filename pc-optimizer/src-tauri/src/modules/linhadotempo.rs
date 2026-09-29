// "O que mudou?" (MASTER-PLAN, item 4): a linha do tempo de cada jogo, feita das medições automáticas que o cliente
// já tem, sem pedir benchmark nenhum. Junta três coisas que antes ficavam em telas diferentes: as partidas medidas,
// as trocas em volta da máquina (driver, Windows, placa, BIOS, versão do Otimiza) e as mudanças do Otimiza.
//
// Coincidência no tempo NÃO é causa: a tela mostra junto e só afirma queda com o critério de `deriva`.
// Das mudanças do Otimiza só aparecem as que continuam aplicadas: o `changelog` esquece o que foi desfeito.

use serde::Serialize;

use crate::modules::deriva::{self, Deriva, Troca};
use crate::modules::medicoes::{self, Fonte, MedicaoAutomatica};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo")]
pub enum Marco {
    Partida {
        quando: u64,
        fps: f64,
        low_1pct: f64,
        fonte: Fonte,
        /// Com gerador de quadros na partida: não entra em comparação nenhuma (`medicoes::comparavel`).
        comparavel: bool,
    },
    /// Algo em volta da máquina trocou entre duas partidas; `quando` é o da primeira partida já com a troca.
    Troca { quando: u64, troca: Troca },
    /// Uma mudança do Otimiza que continua aplicada.
    Otimiza { quando: u64, nome: String },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "estado")]
pub enum Julgamento {
    Caiu { queda: Deriva },
    /// O FPS médio das partidas comparadas não caiu além da margem de erro (só o FPS médio, só a janela da deriva).
    NaoCaiu,
    PoucasPartidas { comparaveis: usize, precisa: usize },
}

impl Marco {
    fn quando(&self) -> u64 {
        match self {
            Marco::Partida { quando, .. } | Marco::Troca { quando, .. } | Marco::Otimiza { quando, .. } => *quando,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LinhaDoJogo {
    pub jogo: String,
    /// Do mais novo para o mais antigo.
    pub marcos: Vec<Marco>,
    /// Pelo critério de `deriva`, só com as partidas comparáveis. "Poucas" não é "não caiu": não houve julgamento.
    pub julgamento: Julgamento,
    pub partidas: usize,
    /// Mudanças do Otimiza aplicadas antes da primeira partida medida deste jogo: já valiam em todas.
    pub otimiza_antes_da_primeira: usize,
}

/// **Pura.** A linha de um jogo. `aplicadas`: `(quando, nome)` das mudanças que continuam aplicadas.
pub fn montar(jogo: &str, medicoes: &[MedicaoAutomatica], aplicadas: &[(u64, String)]) -> LinhaDoJogo {
    let mut do_jogo: Vec<&MedicaoAutomatica> = medicoes.iter().filter(|m| m.jogo.eq_ignore_ascii_case(jogo)).collect();
    do_jogo.sort_by_key(|m| m.quando);

    let mut marcos: Vec<Marco> = do_jogo
        .iter()
        .map(|m| Marco::Partida {
            quando: m.quando,
            fps: m.fps,
            low_1pct: m.low_1pct,
            fonte: m.fonte(),
            comparavel: medicoes::comparavel(m),
        })
        .collect();

    for par in do_jogo.windows(2) {
        if let (Some(a), Some(b)) = (&par[0].ambiente, &par[1].ambiente) {
            marcos.extend(a.trocas(b).into_iter().map(|troca| Marco::Troca { quando: par[1].quando, troca }));
        }
    }

    let primeira = do_jogo.first().map(|m| m.quando);
    let otimiza_antes_da_primeira = aplicadas.iter().filter(|(q, _)| primeira.is_some_and(|p| *q < p)).count();
    marcos.extend(
        aplicadas
            .iter()
            .filter(|(q, _)| primeira.is_some_and(|p| *q >= p))
            .map(|(quando, nome)| Marco::Otimiza { quando: *quando, nome: nome.clone() }),
    );

    // Do mais novo para o mais antigo. No mesmo segundo a partida vem primeiro: a troca aconteceu antes dela.
    let ordem = |m: &Marco| match m {
        Marco::Partida { .. } => 0,
        _ => 1,
    };
    marcos.sort_by(|a, b| b.quando().cmp(&a.quando()).then(ordem(a).cmp(&ordem(b))));

    // Só o medidor da mais recente: o canal antigo dobra os quadros do FiveM (`medicoes::so_do_medidor_atual`).
    let comparaveis: Vec<MedicaoAutomatica> = medicoes::so_do_medidor_atual(
        do_jogo.iter().filter(|m| medicoes::comparavel(m)).map(|m| (*m).clone()).collect(),
    );
    LinhaDoJogo {
        jogo: do_jogo.last().map(|m| m.jogo.clone()).unwrap_or_else(|| jogo.to_string()),
        marcos,
        julgamento: if comparaveis.len() < deriva::PARTIDAS_PARA_JULGAR {
            Julgamento::PoucasPartidas { comparaveis: comparaveis.len(), precisa: deriva::PARTIDAS_PARA_JULGAR }
        } else {
            match deriva::procurar_no_jogo(jogo, &comparaveis) {
                Some(queda) => Julgamento::Caiu { queda },
                None => Julgamento::NaoCaiu,
            }
        },
        partidas: do_jogo.len(),
        otimiza_antes_da_primeira,
    }
}

/// **Pura.** Uma linha por jogo medido, o jogo com a partida mais recente primeiro.
pub fn todos(medicoes: &[MedicaoAutomatica], aplicadas: &[(u64, String)]) -> Vec<LinhaDoJogo> {
    let mut jogos: Vec<(String, u64)> = Vec::new();
    for m in medicoes {
        match jogos.iter_mut().find(|(j, _)| j.eq_ignore_ascii_case(&m.jogo)) {
            Some((_, q)) => *q = (*q).max(m.quando),
            None => jogos.push((m.jogo.clone(), m.quando)),
        }
    }
    jogos.sort_by(|a, b| b.1.cmp(&a.1));
    jogos.into_iter().map(|(j, _)| montar(&j, medicoes, aplicadas)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::deriva::{Ambiente, Campo};
    use crate::modules::medicoes::GeracaoNaPartida;

    fn partida(jogo: &str, quando: u64, fps: f64, driver: Option<&str>) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: jogo.to_string(),
            quando,
            fps,
            low_1pct: fps / 2.0,
            engasgos_por_minuto: 1.0,
            segundos: 20.0,
            confiavel: true,
            mudancas_aplicadas: 0,
            ambiente: driver.map(|d| Ambiente { driver: Some(d.into()), windows: Some("19045.1".into()), ..Default::default() }),
            frametime_medio_ms: None,
            frametime_p95_ms: None,
            frametime_p99_ms: None,
            placa: None,
            cpu_uso_pct: None,
            gpu_uso_pct: None,
            trancos_com_disco_pct: None,
            trancos_medidos: None,
            governador: None,
            geracao: None,
            presentmon: None,
            quadros: None,
            configuracao_do_jogo: None,
            versao_do_formato: 1,
            eventos: None,
        }
    }

    #[test]
    fn a_troca_de_driver_aparece_entre_as_partidas_certas() {
        let ms = [partida("FiveM.exe", 100, 90.0, Some("a")), partida("FiveM.exe", 200, 91.0, Some("b"))];
        let l = montar("FiveM.exe", &ms, &[]);
        assert_eq!(l.marcos.len(), 3);
        // Do mais novo para o mais antigo: a partida que já tinha o driver novo, a troca, a partida de antes.
        assert!(matches!(l.marcos[0], Marco::Partida { quando: 200, .. }));
        assert!(matches!(&l.marcos[1], Marco::Troca { quando: 200, troca } if troca.campo == Campo::Driver && troca.de == "a"));
        assert!(matches!(l.marcos[2], Marco::Partida { quando: 100, .. }));
    }

    #[test]
    fn medicao_antiga_sem_ambiente_nao_inventa_troca() {
        let ms = [partida("FiveM.exe", 100, 90.0, None), partida("FiveM.exe", 200, 91.0, Some("b"))];
        assert!(!montar("FiveM.exe", &ms, &[]).marcos.iter().any(|m| matches!(m, Marco::Troca { .. })));
    }

    #[test]
    fn mudanca_do_otimiza_antes_da_primeira_partida_so_e_contada() {
        let ms = [partida("FiveM.exe", 100, 90.0, None), partida("FiveM.exe", 300, 90.0, None)];
        let aplicadas = [(50, "Plano OTIMIZA".to_string()), (200, "Perfil do jogo".to_string())];
        let l = montar("FiveM.exe", &ms, &aplicadas);
        assert_eq!(l.otimiza_antes_da_primeira, 1);
        let nomes: Vec<&str> = l.marcos.iter().filter_map(|m| match m { Marco::Otimiza { nome, .. } => Some(nome.as_str()), _ => None }).collect();
        assert_eq!(nomes, vec!["Perfil do jogo"]);
    }

    #[test]
    fn partida_com_gerador_aparece_mas_nao_compara() {
        let com = MedicaoAutomatica { geracao: Some(GeracaoNaPartida::LosslessScaling), ..partida("FiveM.exe", 100, 60.0, None) };
        let l = montar("FiveM.exe", &[com], &[]);
        assert!(matches!(l.marcos[0], Marco::Partida { comparavel: false, .. }));
    }

    #[test]
    fn queda_depois_do_driver_novo_sai_com_o_criterio_da_deriva() {
        let mut ms: Vec<MedicaoAutomatica> = (0..4).map(|i| partida("FiveM.exe", i, 140.0 + (i % 2) as f64, Some("a"))).collect();
        ms.extend((4..8).map(|i| partida("FiveM.exe", i, 110.0 + (i % 2) as f64, Some("b"))));
        let l = montar("FiveM.exe", &ms, &[]);
        let Julgamento::Caiu { queda } = l.julgamento else { panic!("caiu mais de 5% com partidas suficientes") };
        assert!(matches!(queda.mudou, deriva::OQueMudou::Driver { .. }));
    }

    #[test]
    fn partidas_com_gerador_nao_contam_para_julgar() {
        let mut ms: Vec<MedicaoAutomatica> = (0..4).map(|i| partida("FiveM.exe", i, 140.0, None)).collect();
        ms.extend((4..6).map(|i| MedicaoAutomatica { geracao: Some(GeracaoNaPartida::LosslessScaling), ..partida("FiveM.exe", i, 70.0, None) }));
        let l = montar("FiveM.exe", &ms, &[]);
        assert_eq!(l.partidas, 6);
        assert_eq!(l.julgamento, Julgamento::PoucasPartidas { comparaveis: 4, precisa: 6 }, "seis partidas, só quatro comparáveis: não é verde");
    }

    #[test]
    fn com_partidas_suficientes_e_sem_queda_e_nao_caiu() {
        let ms: Vec<MedicaoAutomatica> = (0..6).map(|i| partida("FiveM.exe", i, 140.0 + (i % 2) as f64, None)).collect();
        assert_eq!(montar("FiveM.exe", &ms, &[]).julgamento, Julgamento::NaoCaiu);
    }

    #[test]
    fn medidores_diferentes_nao_entram_na_mesma_conta() {
        // Cinco partidas antigas (canal antigo) e uma nova: não há seis comparáveis do mesmo medidor.
        let mut ms: Vec<MedicaoAutomatica> = (0..5).map(|i| partida("FiveM.exe", i, 180.0, Some("a"))).collect();
        let mut nova = partida("FiveM.exe", 10, 90.0, Some("a"));
        nova.presentmon = crate::modules::relatoriodapartida::tests_resumo();
        ms.push(nova);
        assert!(matches!(montar("FiveM.exe", &ms, &[]).julgamento, Julgamento::PoucasPartidas { comparaveis: 1, .. }));
    }

    #[test]
    fn cada_jogo_tem_a_sua_linha_e_o_mais_recente_vem_primeiro() {
        let ms = [partida("GTA5.exe", 100, 90.0, None), partida("FiveM.exe", 300, 90.0, None), partida("fivem.exe", 50, 80.0, None)];
        let linhas = todos(&ms, &[]);
        assert_eq!(linhas.len(), 2);
        assert_eq!(linhas[0].partidas, 2, "o nome do jogo não diferencia maiúscula");
        assert_eq!(linhas[1].jogo, "GTA5.exe");
    }
}
