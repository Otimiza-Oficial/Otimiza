// A prova: antes e depois medidos no jogo do cliente, com a frase honesta. Dois minutos do mesmo jogo dão números
// muito diferentes (menu a 300, rua cheia a 90), e o produto não sabe onde a pessoa estava: por isso toda
// comparação traz a ressalva escrita. Diz o que mudou, se passou do ruído, e diz quando piorou.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prova {
    pub jogo: String,
    pub quando: u64,
    pub fps: f64,
    pub low_1pct: f64,
    pub engasgos_por_minuto: f64,
    pub segundos: f64,
    pub confiavel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparacao {
    pub antes: Prova,
    pub depois: Prova,

    pub fps_delta: f64,
    pub fps_pct: f64,
    pub low_delta: f64,
    pub low_pct: f64,
    pub engasgos_delta: f64,

    pub veredito: String,
    pub ressalvas: Vec<String>,
    pub vale_como_prova: bool,
}

/// Duas medições seguidas do MESMO jogo, sem mexer em nada, variam nessa ordem: abaixo disso é ruído.
const RUIDO_PCT: f64 = 3.0;

const DIFERENCA_DE_DURACAO_ACEITAVEL: f64 = 0.5;

fn caminho() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    base.join("pc-optimizer").join("prova.json")
}

pub fn guardar(prova: &Prova) -> Result<(), String> {
    let destino = caminho();

    if let Some(dir) = destino.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("não consegui criar a pasta: {}", e))?;
    }

    let bruto = serde_json::to_string_pretty(prova)
        .map_err(|e| format!("não consegui serializar a medição: {}", e))?;

    fs::write(&destino, bruto).map_err(|e| format!("não consegui gravar a medição: {}", e))
}

pub fn guardada() -> Option<Prova> {
    fs::read_to_string(caminho())
        .ok()
        .and_then(|bruto| serde_json::from_str(&bruto).ok())
}

fn variacao(antes: f64, depois: f64) -> f64 {
    if antes <= 0.0 {
        return 0.0;
    }
    ((depois - antes) / antes) * 100.0
}

/// Pura: recebe as duas provas e devolve o veredito, para cada caso ser provado sem jogo aberto.
pub fn comparar(antes: &Prova, depois: &Prova) -> Comparacao {
    let mut ressalvas = Vec::new();

    // Jogos diferentes não se comparam: é impedimento, não ressalva.
    let jogos_batem = antes.jogo.eq_ignore_ascii_case(&depois.jogo);

    if !jogos_batem {
        ressalvas.push(format!(
            "As duas medições são de jogos diferentes: `{}` e `{}`. Não dá para \
             comparar.",
            antes.jogo, depois.jogo
        ));
    }

    // A ressalva que SEMPRE aparece: sem ela, "antes" no trânsito e "depois" no menu viram 200% de ganho.
    ressalvas.push(
        "As duas medições precisam ter sido feitas no MESMO lugar do jogo. \
         Menu e rua movimentada dão números muito diferentes na mesma máquina."
            .to_string(),
    );

    if !antes.confiavel || !depois.confiavel {
        ressalvas.push(
            "Uma das medições foi curta demais para os detalhes serem confiáveis. \
             Meça por mais tempo."
                .to_string(),
        );
    }

    let diferenca_de_duracao = (antes.segundos - depois.segundos).abs()
        / antes.segundos.max(depois.segundos).max(1.0);

    if diferenca_de_duracao > DIFERENCA_DE_DURACAO_ACEITAVEL {
        ressalvas.push(format!(
            "As medições duraram tempos bem diferentes ({:.0}s e {:.0}s).",
            antes.segundos, depois.segundos
        ));
    }

    let fps_delta = depois.fps - antes.fps;
    let fps_pct = variacao(antes.fps, depois.fps);
    let low_delta = depois.low_1pct - antes.low_1pct;
    let low_pct = variacao(antes.low_1pct, depois.low_1pct);
    let engasgos_delta = depois.engasgos_por_minuto - antes.engasgos_por_minuto;

    let passou_do_ruido = fps_pct.abs() >= RUIDO_PCT;

    let veredito = if !jogos_batem {
        "Não dá para comparar medições de jogos diferentes.".to_string()
    } else if !passou_do_ruido {
        format!(
            "O FPS médio praticamente não mudou: {:.0} antes, {:.0} depois. \
             Uma diferença abaixo de {:.0}% é ruído de medição, não ganho.",
            antes.fps, depois.fps, RUIDO_PCT
        )
    } else if fps_delta > 0.0 {
        let engasgo = if engasgos_delta < -0.5 {
            format!(
                " E os engasgos caíram de {:.0} para {:.0} por minuto, que é o que \
                 se sente como travada.",
                antes.engasgos_por_minuto, depois.engasgos_por_minuto
            )
        } else {
            String::new()
        };

        format!(
            "De {:.0} para {:.0} quadros por segundo — {:+.0}%. Nos piores momentos \
             (1% mais lento), de {:.0} para {:.0}.{}",
            antes.fps, depois.fps, fps_pct, antes.low_1pct, depois.low_1pct, engasgo
        )
    } else {
        // PIOROU, e o produto diz: um produto que só reporta ganho está anunciando, não medindo.
        format!(
            "O FPS CAIU: de {:.0} para {:.0} ({:+.0}%). Vale desfazer as mudanças e \
             medir de novo.",
            antes.fps, depois.fps, fps_pct
        )
    };

    Comparacao {
        vale_como_prova: jogos_batem
            && passou_do_ruido
            && fps_delta > 0.0
            && antes.confiavel
            && depois.confiavel,
        antes: antes.clone(),
        depois: depois.clone(),
        fps_delta,
        fps_pct,
        low_delta,
        low_pct,
        engasgos_delta,
        veredito,
        ressalvas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prova(jogo: &str, fps: f64, low: f64, engasgos: f64) -> Prova {
        Prova {
            jogo: jogo.to_string(),
            quando: 0,
            fps,
            low_1pct: low,
            engasgos_por_minuto: engasgos,
            segundos: 20.0,
            confiavel: true,
        }
    }

    #[test]
    fn ganho_grande_vira_frase_com_os_dois_numeros() {
        let c = comparar(
            &prova("FiveM", 200.0, 120.0, 8.0),
            &prova("FiveM", 500.0, 380.0, 2.0),
        );

        assert!(c.vale_como_prova);
        assert!(c.veredito.contains("200"), "{}", c.veredito);
        assert!(c.veredito.contains("500"), "{}", c.veredito);
        assert!(c.veredito.contains("120"), "{}", c.veredito);
        assert!(c.veredito.contains("380"), "{}", c.veredito);
    }

    #[test]
    fn diferenca_dentro_do_ruido_nao_e_ganho() {
        let c = comparar(
            &prova("FiveM", 200.0, 120.0, 8.0),
            &prova("FiveM", 204.0, 122.0, 8.0),
        );

        assert!(!c.vale_como_prova);
        assert!(c.veredito.contains("não mudou"), "{}", c.veredito);
        assert!(c.veredito.contains("ruído"), "{}", c.veredito);
    }

    #[test]
    fn piora_e_dita_em_voz_alta() {
        let c = comparar(
            &prova("FiveM", 200.0, 120.0, 8.0),
            &prova("FiveM", 150.0, 90.0, 14.0),
        );

        assert!(!c.vale_como_prova);
        assert!(c.veredito.contains("CAIU"), "{}", c.veredito);
        assert!(c.veredito.contains("desfazer"), "{}", c.veredito);
    }

    #[test]
    fn a_ressalva_do_lugar_nunca_some() {
        for (antes, depois) in [
            (prova("FiveM", 200.0, 120.0, 8.0), prova("FiveM", 500.0, 380.0, 2.0)),
            (prova("FiveM", 200.0, 120.0, 8.0), prova("FiveM", 201.0, 121.0, 8.0)),
            (prova("FiveM", 200.0, 120.0, 8.0), prova("FiveM", 100.0, 60.0, 20.0)),
        ] {
            let c = comparar(&antes, &depois);
            assert!(
                c.ressalvas.iter().any(|r| r.contains("MESMO lugar")),
                "a ressalva do lugar sumiu de uma comparação"
            );
        }
    }

    #[test]
    fn jogos_diferentes_nao_se_comparam() {
        let c = comparar(
            &prova("gta5", 100.0, 60.0, 10.0),
            &prova("valorant", 500.0, 400.0, 1.0),
        );

        assert!(!c.vale_como_prova);
        assert!(c.veredito.contains("jogos diferentes"), "{}", c.veredito);
    }

    #[test]
    fn medicao_nao_confiavel_nao_vale_como_prova() {
        let mut depois = prova("FiveM", 500.0, 380.0, 2.0);
        depois.confiavel = false;

        let c = comparar(&prova("FiveM", 200.0, 120.0, 8.0), &depois);

        assert!(!c.vale_como_prova);
        assert!(c.ressalvas.iter().any(|r| r.contains("curta demais")));
    }

    #[test]
    fn duracoes_diferentes_viram_ressalva() {
        let mut depois = prova("FiveM", 500.0, 380.0, 2.0);
        depois.segundos = 3.0;

        let c = comparar(&prova("FiveM", 200.0, 120.0, 8.0), &depois);

        assert!(c.ressalvas.iter().any(|r| r.contains("tempos bem diferentes")));
    }

    /// Um "antes" de zero não pode virar divisão por zero nem porcentagem falsa.
    #[test]
    fn antes_zerado_nao_estoura() {
        let c = comparar(&prova("FiveM", 0.0, 0.0, 0.0), &prova("FiveM", 300.0, 200.0, 1.0));

        assert!(c.fps_pct.is_finite(), "porcentagem virou infinito");
        assert!(c.low_pct.is_finite());
    }
}
