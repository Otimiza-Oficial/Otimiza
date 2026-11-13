// A REDE NA PARTIDA (Plano de refino, R7). O cartão da partida responde "foi a rede ou o PC?": nas mesmas janelas de
// 20 s da medição automática (a primeira três minutos depois de o jogo abrir, depois uma a cada vinte), um ping a cada
// 250 ms para 1.1.1.1. Nada medido a mais fora dessas janelas, e 80 pacotes pequenos por janela não pesam no jogo.
//
// Mede até a internet, não até o servidor do jogo (muitos não respondem ping): o que aparece aqui é a casa, o Wi-Fi e
// a operadora. Janela sem nenhuma resposta não vira "perdeu tudo": pode ser ping bloqueado, e fica de fora.
//
// Guardado à parte, em `%APPDATA%\pc-optimizer\rede-partidas.json`, pelo mesmo `quando` da medição: o relatório da
// partida junta as janelas dele.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Janelas guardadas (umas 40 horas de jogo).
pub const JANELAS_GUARDADAS: usize = 120;
/// Perda a partir disto já se sente no jogo (teleporte, tiro que não registra).
pub const PERDA_QUE_SE_SENTE_PCT: f64 = 2.0;
/// Variação média entre pings seguidos a partir disto: o ping "pula" no jogo.
pub const VARIACAO_QUE_SE_SENTE_MS: f64 = 15.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JanelaDeRede {
    pub jogo: String,
    pub quando: u64,
    pub ping_ms: f64,
    pub variacao_ms: f64,
    pub perda_pct: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub enum Firmeza {
    Firme,
    Oscilou,
    Perdeu,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct RedeDaPartida {
    pub janelas: usize,
    pub ping_ms: f64,
    pub variacao_ms: f64,
    pub perda_pct: f64,
    pub firmeza: Firmeza,
    pub frase: String,
}

/// **Pura.** Uma janela de ecos vira números. Sem nenhuma resposta, `None` (ping bloqueado não é rede caindo).
pub fn resumir_janela(jogo: &str, quando: u64, ecos: &[Option<u32>]) -> Option<JanelaDeRede> {
    let voltaram: Vec<f64> = ecos.iter().flatten().map(|&v| v as f64).collect();
    if voltaram.is_empty() {
        return None;
    }
    let variacao = if voltaram.len() > 1 {
        voltaram.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (voltaram.len() - 1) as f64
    } else {
        0.0
    };
    Some(JanelaDeRede {
        jogo: jogo.to_string(),
        quando,
        ping_ms: super::fila::mediana(ecos)?,
        variacao_ms: variacao,
        perda_pct: super::fila::perda(ecos),
    })
}

/// **Pura.** As janelas desta partida (mesmo jogo, `quando` entre o início e o fim do relatório) num resumo.
pub fn da_partida(janelas: &[JanelaDeRede], jogo: &str, inicio: u64, fim: u64) -> Option<RedeDaPartida> {
    let estas: Vec<&JanelaDeRede> = janelas
        .iter()
        .filter(|j| j.jogo.eq_ignore_ascii_case(jogo) && j.quando >= inicio && j.quando <= fim)
        .collect();
    if estas.is_empty() {
        return None;
    }
    let n = estas.len() as f64;
    let mut pings: Vec<f64> = estas.iter().map(|j| j.ping_ms).collect();
    pings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let ping = pings[pings.len() / 2];
    let variacao = estas.iter().map(|j| j.variacao_ms).sum::<f64>() / n;
    let perda = estas.iter().map(|j| j.perda_pct).sum::<f64>() / n;
    let firmeza = if perda >= PERDA_QUE_SE_SENTE_PCT {
        Firmeza::Perdeu
    } else if variacao >= VARIACAO_QUE_SE_SENTE_MS {
        Firmeza::Oscilou
    } else {
        Firmeza::Firme
    };
    let frase = match firmeza {
        Firmeza::Firme => format!(
            "Rede firme: {:.0} ms até a internet, sem perda. Se travou, não foi a rede.",
            ping
        ),
        Firmeza::Oscilou => format!(
            "A rede oscilou: o ping pulava {:.0} ms entre um pacote e outro. Teleporte e atraso podem ter sido dela.",
            variacao
        ),
        Firmeza::Perdeu => format!(
            "A rede perdeu {:.1}% dos pacotes até a internet. Tiro que não registra e teleporte podem ter sido dela.",
            perda
        ),
    };
    Some(RedeDaPartida { janelas: estas.len(), ping_ms: ping, variacao_ms: variacao, perda_pct: perda, firmeza, frase })
}

fn caminho() -> PathBuf {
    std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("rede-partidas.json")
}

pub fn ler() -> Vec<JanelaDeRede> {
    fs::read_to_string(caminho())
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

/// Guarda a janela (sem resposta nenhuma, não guarda). Falha só vai para o log: a medição do jogo não depende disto.
pub fn guardar(jogo: &str, quando: u64, ecos: &[Option<u32>]) {
    let Some(janela) = resumir_janela(jogo, quando, ecos) else {
        return;
    };
    let mut todas = ler();
    todas.push(janela);
    let sobra = todas.len().saturating_sub(JANELAS_GUARDADAS);
    todas.drain(..sobra);
    let c = caminho();
    let gravado = c
        .parent()
        .map(fs::create_dir_all)
        .transpose()
        .map_err(|e| e.to_string())
        .and_then(|_| serde_json::to_string(&todas).map_err(|e| e.to_string()))
        .and_then(|t| fs::write(&c, t).map_err(|e| e.to_string()));
    if let Err(e) = gravado {
        crate::utils::Logger::warn(&format!("rede na partida: não gravei a janela: {}", e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(quando: u64, ping: f64, variacao: f64, perda: f64) -> JanelaDeRede {
        JanelaDeRede { jogo: "FiveM_GTAProcess.exe".into(), quando, ping_ms: ping, variacao_ms: variacao, perda_pct: perda }
    }

    #[test]
    fn janela_sem_resposta_nao_vira_rede_caindo() {
        assert_eq!(resumir_janela("x", 1, &[None, None, None]), None);
        let r = resumir_janela("x", 1, &[Some(20), Some(30), None, Some(20)]).unwrap();
        assert_eq!(r.ping_ms, 20.0);
        assert_eq!(r.perda_pct, 25.0);
        assert_eq!(r.variacao_ms, 10.0);
    }

    #[test]
    fn a_partida_junta_so_as_janelas_dela() {
        let todas = vec![j(100, 20.0, 2.0, 0.0), j(1300, 22.0, 3.0, 0.0), j(9000, 300.0, 90.0, 40.0)];
        let r = da_partida(&todas, "fivem_gtaprocess.exe", 100, 1300).unwrap();
        assert_eq!(r.janelas, 2);
        assert_eq!(r.firmeza, Firmeza::Firme);
        assert!(r.frase.contains("não foi a rede"), "{}", r.frase);
        assert_eq!(da_partida(&todas, "outro.exe", 0, 10_000), None);
    }

    #[test]
    fn perda_vence_oscilacao_e_as_duas_aparecem() {
        let perdeu = da_partida(&[j(1, 20.0, 30.0, 5.0)], "FiveM_GTAProcess.exe", 0, 10).unwrap();
        assert_eq!(perdeu.firmeza, Firmeza::Perdeu);
        let oscilou = da_partida(&[j(1, 20.0, 30.0, 0.0)], "FiveM_GTAProcess.exe", 0, 10).unwrap();
        assert_eq!(oscilou.firmeza, Firmeza::Oscilou);
    }
}
