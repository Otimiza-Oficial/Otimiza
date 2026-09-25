// O produto olhando o próprio trabalho: na 2.1.0 o FPS de um cliente caiu de ~200 para 80-120, as medições de
// antes e depois estavam no disco dele, e ninguém comparou. Regras: só o mesmo jogo; só medição confiável;
// duas de cada lado, pela MEDIANA; margem larga (8%), porque 5% entre sessões é o normal. Não desfaz sozinho
// (sessões e dias diferentes): avisa com os números e um botão.

use serde::{Deserialize, Serialize};

use crate::modules::medicoes::MedicaoAutomatica;

/// Duas, e não três: o cliente que perdeu FPS percebe em dez minutos, não depois de uma hora de jogo.
pub const AMOSTRAS_MINIMAS: usize = 2;

pub const MARGEM_DE_RUIDO_PCT: f64 = 8.0;

/// O caso que originou o módulo foi de cerca de 50%.
pub const QUEDA_GRAVE_PCT: f64 = 20.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lado {
    pub fps: f64,
    pub low_1pct: f64,
    pub amostras: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "desfecho")]
pub enum Desfecho {
    /// NÃO é "está tudo bem": é "ainda não dá para dizer", com essas palavras.
    SemAmostra,
    Igual,
    Melhorou,
    Piorou,
    /// O único que interrompe o cliente em vez de esperar ele abrir a tela.
    PiorouMuito,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Veredito {
    pub jogo: String,
    pub desfecho: Desfecho,
    /// `None` quando faltou amostra: a tela não inventa número.
    pub antes: Option<Lado>,
    pub depois: Option<Lado>,
    pub variacao_fps_pct: Option<f64>,
    /// Descreve o engasgo, e por isso decide junto com o FPS.
    pub variacao_low_pct: Option<f64>,
}

/// `None` para lista vazia, em vez de zero: zero FPS significa alguma coisa.
fn mediana(mut valores: Vec<f64>) -> Option<f64> {
    if valores.is_empty() {
        return None;
    }

    valores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let meio = valores.len() / 2;
    Some(if valores.len() % 2 == 0 {
        (valores[meio - 1] + valores[meio]) / 2.0
    } else {
        valores[meio]
    })
}

fn resumir(medicoes: &[&MedicaoAutomatica]) -> Option<Lado> {
    if medicoes.len() < AMOSTRAS_MINIMAS {
        return None;
    }

    Some(Lado {
        fps: mediana(medicoes.iter().map(|m| m.fps).collect())?,
        low_1pct: mediana(medicoes.iter().map(|m| m.low_1pct).collect())?,
        amostras: medicoes.len(),
    })
}

/// `None` com `antes` zero ou negativo: infinito na tela é pior que a ausência.
fn variacao_pct(antes: f64, depois: f64) -> Option<f64> {
    if antes <= 0.0 {
        return None;
    }
    Some((depois - antes) / antes * 100.0)
}

/// `mudancas_na_epoca` separa os lados, e já é gravado desde a 2.0: funciona com o histórico que existe.
pub fn comparar(jogo: &str, medicoes: &[MedicaoAutomatica], aplicadas_hoje: usize) -> Veredito {
    let deste_jogo: Vec<&MedicaoAutomatica> = medicoes
        .iter()
        .filter(|m| m.jogo.eq_ignore_ascii_case(jogo) && m.confiavel)
        .collect();

    let antes: Vec<&MedicaoAutomatica> = deste_jogo
        .iter()
        .copied()
        .filter(|m| m.mudancas_aplicadas < aplicadas_hoje)
        .collect();

    let depois: Vec<&MedicaoAutomatica> = deste_jogo
        .iter()
        .copied()
        .filter(|m| m.mudancas_aplicadas >= aplicadas_hoje)
        .collect();

    let (antes, depois) = (resumir(&antes), resumir(&depois));

    // Sem os dois lados, `Igual` seria dizer "não mudou" sobre uma conta que não foi feita.
    let (Some(a), Some(d)) = (antes.clone(), depois.clone()) else {
        return Veredito {
            jogo: jogo.to_string(),
            desfecho: Desfecho::SemAmostra,
            antes,
            depois,
            variacao_fps_pct: None,
            variacao_low_pct: None,
        };
    };

    let variacao_fps_pct = variacao_pct(a.fps, d.fps);
    let variacao_low_pct = variacao_pct(a.low_1pct, d.low_1pct);

    // A pior das duas manda: segurar a média e destruir o 1% pior é jogo pior de jogar.
    let pior = [variacao_fps_pct, variacao_low_pct]
        .into_iter()
        .flatten()
        .fold(f64::INFINITY, f64::min);

    let desfecho = if !pior.is_finite() {
        Desfecho::SemAmostra
    } else if pior <= -QUEDA_GRAVE_PCT {
        Desfecho::PiorouMuito
    } else if pior <= -MARGEM_DE_RUIDO_PCT {
        Desfecho::Piorou
    } else if variacao_fps_pct.unwrap_or(0.0) >= MARGEM_DE_RUIDO_PCT {
        Desfecho::Melhorou
    } else {
        Desfecho::Igual
    };

    Veredito {
        jogo: jogo.to_string(),
        desfecho,
        antes,
        depois,
        variacao_fps_pct,
        variacao_low_pct,
    }
}

/// Do pior para o melhor: quem tem um jogo que piorou precisa vê-lo primeiro.
pub fn todos(medicoes: &[MedicaoAutomatica], aplicadas_hoje: usize) -> Vec<Veredito> {
    let mut jogos: Vec<String> = Vec::new();
    for m in medicoes {
        if !jogos.iter().any(|j| j.eq_ignore_ascii_case(&m.jogo)) {
            jogos.push(m.jogo.clone());
        }
    }

    let mut vereditos: Vec<Veredito> = jogos
        .iter()
        .map(|j| comparar(j, medicoes, aplicadas_hoje))
        .collect();

    vereditos.sort_by(|a, b| {
        let chave = |v: &Veredito| v.variacao_fps_pct.unwrap_or(f64::INFINITY);
        chave(a)
            .partial_cmp(&chave(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    vereditos
}

/// Usado para interromper na abertura: aviso que só aparece para quem procura não teria evitado a 2.1.0.
pub fn pior_regressao(vereditos: &[Veredito]) -> Option<&Veredito> {
    vereditos
        .iter()
        .find(|v| matches!(v.desfecho, Desfecho::Piorou | Desfecho::PiorouMuito))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn medicao(jogo: &str, fps: f64, low: f64, aplicadas: usize) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: jogo.to_string(),
            quando: 0,
            fps,
            low_1pct: low,
            engasgos_por_minuto: 0.0,
            segundos: 20.0,
            placa: None,
            confiavel: true,
            mudancas_aplicadas: aplicadas,
            ambiente: None,
            frametime_medio_ms: None,
            frametime_p95_ms: None,
            frametime_p99_ms: None,
            cpu_uso_pct: None,
            gpu_uso_pct: None,
            trancos_com_disco_pct: None,
            trancos_medidos: None,
            governador: None,
        }
    }

    #[test]
    fn a_queda_de_200_para_100_e_apontada_como_grave() {
        let medicoes = vec![
            medicao("FiveM.exe", 205.0, 150.0, 0),
            medicao("FiveM.exe", 198.0, 144.0, 0),
            medicao("FiveM.exe", 96.0, 70.0, 14),
            medicao("FiveM.exe", 108.0, 74.0, 14),
        ];

        let v = comparar("FiveM.exe", &medicoes, 14);

        assert_eq!(v.desfecho, Desfecho::PiorouMuito);
        assert!(v.variacao_fps_pct.unwrap() < -40.0);
        assert_eq!(v.antes.unwrap().amostras, 2);
    }

    #[test]
    fn variacao_pequena_nao_vira_acusacao() {
        let medicoes = vec![
            medicao("gta.exe", 100.0, 80.0, 0),
            medicao("gta.exe", 104.0, 82.0, 0),
            medicao("gta.exe", 98.0, 79.0, 9),
            medicao("gta.exe", 101.0, 78.0, 9),
        ];

        assert_eq!(comparar("gta.exe", &medicoes, 9).desfecho, Desfecho::Igual);
    }

    #[test]
    fn ganho_de_verdade_e_reconhecido() {
        let medicoes = vec![
            medicao("gta.exe", 60.0, 45.0, 0),
            medicao("gta.exe", 62.0, 46.0, 0),
            medicao("gta.exe", 88.0, 66.0, 7),
            medicao("gta.exe", 91.0, 68.0, 7),
        ];

        assert_eq!(comparar("gta.exe", &medicoes, 7).desfecho, Desfecho::Melhorou);
    }

    #[test]
    fn media_estavel_com_o_um_por_cento_pior_desabando_nao_e_igual() {
        let medicoes = vec![
            medicao("fivem.exe", 120.0, 95.0, 0),
            medicao("fivem.exe", 118.0, 93.0, 0),
            medicao("fivem.exe", 119.0, 55.0, 11),
            medicao("fivem.exe", 121.0, 52.0, 11),
        ];

        let v = comparar("fivem.exe", &medicoes, 11);

        assert_eq!(v.desfecho, Desfecho::PiorouMuito);
        assert!(v.variacao_fps_pct.unwrap().abs() < 3.0, "o FPS médio mal se moveu");
    }

    #[test]
    fn um_lado_so_nao_vira_veredito() {
        let medicoes = vec![
            medicao("fivem.exe", 200.0, 150.0, 0),
            medicao("fivem.exe", 198.0, 148.0, 0),
            medicao("fivem.exe", 90.0, 60.0, 14),
        ];

        let v = comparar("fivem.exe", &medicoes, 14);

        assert_eq!(v.desfecho, Desfecho::SemAmostra);
        assert!(v.depois.is_none(), "uma medição só não resume nada");
        assert!(v.variacao_fps_pct.is_none(), "não há variação para mostrar");
    }

    #[test]
    fn medicao_nao_confiavel_fica_de_fora_dos_dois_lados() {
        let mut curta = medicao("fivem.exe", 30.0, 10.0, 14);
        curta.confiavel = false;

        let medicoes = vec![
            medicao("fivem.exe", 100.0, 80.0, 0),
            medicao("fivem.exe", 102.0, 81.0, 0),
            medicao("fivem.exe", 101.0, 80.0, 14),
            medicao("fivem.exe", 99.0, 79.0, 14),
            curta,
        ];

        let v = comparar("fivem.exe", &medicoes, 14);

        assert_eq!(v.desfecho, Desfecho::Igual);
        assert_eq!(v.depois.unwrap().amostras, 2, "a medição curta entrou na conta");
    }

    #[test]
    fn o_fps_de_um_jogo_nao_entra_na_conta_do_outro() {
        let medicoes = vec![
            medicao("fivem.exe", 100.0, 80.0, 0),
            medicao("fivem.exe", 102.0, 81.0, 0),
            medicao("cs2.exe", 400.0, 300.0, 0),
            medicao("cs2.exe", 410.0, 305.0, 0),
            medicao("fivem.exe", 99.0, 79.0, 5),
            medicao("fivem.exe", 101.0, 80.0, 5),
        ];

        let v = comparar("fivem.exe", &medicoes, 5);

        assert_eq!(v.antes.unwrap().amostras, 2, "o CS entrou na conta do FiveM");
        assert_eq!(v.desfecho, Desfecho::Igual);
    }

    #[test]
    fn a_lista_traz_o_pior_jogo_primeiro() {
        let medicoes = vec![
            medicao("bom.exe", 60.0, 50.0, 0),
            medicao("bom.exe", 61.0, 50.0, 0),
            medicao("bom.exe", 90.0, 70.0, 6),
            medicao("bom.exe", 92.0, 71.0, 6),
            medicao("ruim.exe", 200.0, 150.0, 0),
            medicao("ruim.exe", 202.0, 152.0, 0),
            medicao("ruim.exe", 100.0, 70.0, 6),
            medicao("ruim.exe", 98.0, 68.0, 6),
        ];

        let lista = todos(&medicoes, 6);

        assert_eq!(lista[0].jogo, "ruim.exe");
        assert_eq!(pior_regressao(&lista).unwrap().jogo, "ruim.exe");
    }

    #[test]
    fn sem_nenhuma_regressao_nao_ha_o_que_avisar() {
        let medicoes = vec![
            medicao("bom.exe", 60.0, 50.0, 0),
            medicao("bom.exe", 61.0, 50.0, 0),
            medicao("bom.exe", 90.0, 70.0, 6),
            medicao("bom.exe", 92.0, 71.0, 6),
        ];

        assert!(pior_regressao(&todos(&medicoes, 6)).is_none());
    }

    #[test]
    fn arquivo_vazio_nao_inventa_veredito() {
        assert!(todos(&[], 0).is_empty());
    }

    #[test]
    fn fps_zero_antes_nao_vira_divisao_por_zero() {
        let medicoes = vec![
            medicao("x.exe", 0.0, 0.0, 0),
            medicao("x.exe", 0.0, 0.0, 0),
            medicao("x.exe", 100.0, 80.0, 3),
            medicao("x.exe", 102.0, 81.0, 3),
        ];

        let v = comparar("x.exe", &medicoes, 3);

        assert!(v.variacao_fps_pct.is_none(), "infinito não vai para a tela");
        assert_eq!(v.desfecho, Desfecho::SemAmostra);
    }
}
