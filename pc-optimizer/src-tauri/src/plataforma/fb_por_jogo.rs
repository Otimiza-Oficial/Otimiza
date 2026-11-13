// O FRAME BOOST QUE A MÁQUINA PROVOU, POR JOGO (Plano de refino, FB8). Quem liga o Frame Boost na mão e joga com ele
// não deveria ter de escolher tudo de novo na próxima partida. Quando o jogo fecha, se a guarda aprovou a sessão (tempo
// bastante, comparações feitas, o jogo manteve o FPS e não houve desligamento por perda), a escolha fica guardada por
// jogo; no próximo "Jogar" ela volta sozinha. O Turbo vem antes: jogo com receita dele segue a receita.
//
// Guardado em `%APPDATA%\pc-optimizer\frameboost-jogos.json`, pelo id do jogo (`historico::id_do_jogo`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::modules::windows::geracao::escala::Nivel;

/// Menos que isto de sessão não prova nada: a guarda nem terminou de comparar.
pub const SEGUNDOS_MINIMOS: f64 = 5.0 * 60.0;
/// O jogo precisa ter mantido pelo menos isto do FPS dele com o gerador ligado.
pub const MANTEVE_MINIMO_PCT: f64 = 90.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "EscolhaFbPorJogo"))]
pub struct Escolha {
    /// 0 = Auto, 1 = só escala, 2+ = fixo.
    pub multiplicador: u8,
    pub escala: Option<Nivel>,
    pub alvo_fps: Option<u32>,
    pub manteve_pct: f64,
    pub quando: u64,
}

/// O que a sessão que acabou deixou, como a guarda viu.
#[derive(Debug, Clone, PartialEq)]
pub struct SessaoVista {
    pub segundos: f64,
    pub manteve_pct: Option<f64>,
    pub comparacoes: u32,
    pub desligou_por_perda: bool,
    /// Só escala não passa pela comparação da guarda (não gera quadros): vale pelo tempo.
    pub so_escala: bool,
}

/// **Pura.** A guarda aprovou esta sessão?
pub fn aprovada(s: &SessaoVista) -> bool {
    if s.desligou_por_perda || s.segundos < SEGUNDOS_MINIMOS {
        return false;
    }
    s.so_escala || (s.comparacoes > 0 && s.manteve_pct.is_some_and(|m| m >= MANTEVE_MINIMO_PCT))
}

/// **Pura.** Em palavras, para o balão do "Jogar".
pub fn descrever(e: &Escolha) -> String {
    let modo = match e.multiplicador {
        0 => match e.alvo_fps {
            Some(a) => format!("Auto até {} FPS", a),
            None => "Auto".to_string(),
        },
        1 => "só escala".to_string(),
        m => format!("{}×", m),
    };
    match e.escala {
        Some(n) if e.multiplicador != 1 => format!("Frame Boost {} com escala {}", modo, n.nome()),
        Some(n) => format!("Frame Boost: escala {}", n.nome()),
        None => format!("Frame Boost {}", modo),
    }
}

fn caminho() -> PathBuf {
    std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("frameboost-jogos.json")
}

fn ler_tudo() -> BTreeMap<String, Escolha> {
    fs::read_to_string(caminho())
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

pub fn escolha(id_do_jogo: &str) -> Option<Escolha> {
    ler_tudo().remove(id_do_jogo)
}

pub fn lembrar(id_do_jogo: &str, e: Escolha) -> Result<(), String> {
    let mut tudo = ler_tudo();
    tudo.insert(id_do_jogo.to_string(), e);
    let c = caminho();
    if let Some(d) = c.parent() {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    fs::write(&c, serde_json::to_string_pretty(&tudo).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// O jogo `executavel` fechou: se o gerador do Otimiza estava nele e a guarda aprovou, guarda a escolha.
#[cfg(target_os = "windows")]
pub fn ao_fechar_o_jogo(executavel: &str) {
    use crate::modules::windows::geracao::{self, Situacao};
    let e = geracao::estado();
    if e.processo.is_empty() || !e.processo.eq_ignore_ascii_case(executavel) {
        return;
    }
    let (id, _) = super::historico::id_do_jogo(executavel);
    // O Turbo cuida dos jogos que têm receita dele.
    if super::turbo::perfil(&id).is_some() {
        return;
    }
    let Some(resumo) = e.sessao.resumo() else {
        return;
    };
    let so_escala = e.escala.is_some() && !e.automatico && e.multiplicador <= 1;
    let manteve = e.razao_fps.filter(|_| e.comparacoes > 0).map(|r| r * 100.0);
    let vista = SessaoVista {
        segundos: resumo.segundos,
        manteve_pct: manteve,
        comparacoes: e.comparacoes,
        desligou_por_perda: e.situacao == Situacao::DesligadoPorPerdaDeFps,
        so_escala,
    };
    if !aprovada(&vista) {
        return;
    }
    let escolha = Escolha {
        multiplicador: if e.automatico { 0 } else { e.multiplicador },
        escala: e.escala,
        alvo_fps: super::frameboost::alvo_ligado(),
        manteve_pct: manteve.unwrap_or(100.0),
        quando: crate::modules::changelog::now_timestamp(),
    };
    match lembrar(&id, escolha.clone()) {
        Ok(()) => crate::utils::Logger::info(&format!("frame boost: {} lembrado para {}", descrever(&escolha), id)),
        Err(err) => crate::utils::Logger::warn(&format!("frame boost: não guardei a escolha de {}: {}", id, err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sessao(segundos: f64, manteve: Option<f64>, comparacoes: u32) -> SessaoVista {
        SessaoVista { segundos, manteve_pct: manteve, comparacoes, desligou_por_perda: false, so_escala: false }
    }

    #[test]
    fn so_a_sessao_que_a_guarda_aprovou_fica_lembrada() {
        assert!(aprovada(&sessao(900.0, Some(96.0), 3)));
        assert!(!aprovada(&sessao(120.0, Some(99.0), 3)), "curta");
        assert!(!aprovada(&sessao(900.0, Some(80.0), 3)), "perdeu FPS");
        assert!(!aprovada(&sessao(900.0, None, 0)), "sem comparação");
        assert!(!aprovada(&SessaoVista { desligou_por_perda: true, ..sessao(900.0, Some(99.0), 3) }));
        // Só escala não gera quadros: vale pelo tempo.
        assert!(aprovada(&SessaoVista { so_escala: true, ..sessao(900.0, None, 0) }));
    }

    #[test]
    fn a_escolha_em_palavras() {
        let e = Escolha { multiplicador: 0, escala: Some(Nivel::Qualidade), alvo_fps: Some(120), manteve_pct: 95.0, quando: 0 };
        let t = descrever(&e);
        assert!(t.contains("Auto até 120") && t.contains("escala"), "{t}");
        assert_eq!(descrever(&Escolha { multiplicador: 2, escala: None, ..e }), "Frame Boost 2×");
    }
}
