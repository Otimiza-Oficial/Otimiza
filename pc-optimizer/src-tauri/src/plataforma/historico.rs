// O histórico de cada jogo, para a biblioteca e para a tela do jogo: última partida, como era antes do Otimiza e como
// ficou depois. Não grava nada novo: lê as partidas que a medição automática já guarda (`modules::medicoes`).
//
// "Antes" são as partidas sem nenhuma mudança do Otimiza aplicada; "depois", as com. O ganho só aparece quando a
// estatística separa os dois lados (`repeticoes::comparar`): com poucas partidas a tela diz "medindo", não um número.

use serde::Serialize;

use crate::modules::medicoes::MedicaoAutomatica;
use crate::modules::repeticoes::{self, Diferenca};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Partida {
    pub quando: u64,
    pub fps_nativo: f64,
    /// Só quando o PresentMon mediu: com geração de quadros, o que chegou à tela.
    pub fps_na_tela: Option<f64>,
    pub low_1pct: f64,
    pub frametime_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "estado")]
pub enum Ganho {
    /// Diferença real: os intervalos de antes e depois não se encostam.
    Medido {
        pct: f64,
        folga_pct: f64,
    },
    SemDiferenca,
    /// Faltam partidas de um dos lados.
    Medindo {
        antes: usize,
        depois: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResumoDoJogo {
    pub id: String,
    pub nome: String,
    pub partidas: usize,
    pub ultima: Option<Partida>,
    pub ganho_fps: Ganho,
    pub ganho_low: Ganho,
    pub melhor_fps: Option<f64>,
}

/// A chave do jogo: o id do catálogo, ou o executável em minúsculas para jogo fora dele.
pub fn id_do_jogo(executavel: &str) -> (String, String) {
    match super::jogos::identificar(executavel) {
        Some(p) => (p.id.to_string(), p.nome.to_string()),
        None => {
            let exe = executavel
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or(executavel)
                .to_lowercase();
            let nome = exe.trim_end_matches(".exe").to_string();
            (exe, nome)
        }
    }
}

fn partida(m: &MedicaoAutomatica) -> Partida {
    Partida {
        quando: m.quando,
        fps_nativo: m.fps,
        fps_na_tela: m
            .presentmon
            .as_ref()
            .map(|r| r.fps_exibido)
            .filter(|t| *t > m.fps * 1.15),
        low_1pct: m.low_1pct,
        frametime_ms: m.frametime_medio_ms,
    }
}

fn ganho(antes: &[f64], depois: &[f64]) -> Ganho {
    let (Some(a), Some(d)) = (
        repeticoes::resumir("antes", antes),
        repeticoes::resumir("depois", depois),
    ) else {
        return Ganho::Medindo {
            antes: antes.len(),
            depois: depois.len(),
        };
    };
    match repeticoes::comparar(&a, &d) {
        Diferenca::Real {
            pct: Some(pct),
            folga,
            ..
        } => Ganho::Medido {
            pct,
            folga_pct: if a.media != 0.0 {
                folga / a.media * 100.0
            } else {
                0.0
            },
        },
        Diferenca::Real { pct: None, .. } | Diferenca::Indistinguivel { .. } => Ganho::SemDiferenca,
        Diferenca::SemRepeticoes { .. } => Ganho::Medindo {
            antes: antes.len(),
            depois: depois.len(),
        },
    }
}

/// Pura. Só partidas confiáveis e comparáveis (sem gerador por fora: o FPS do jogo cai com ele ligado).
pub fn resumir(medicoes: &[MedicaoAutomatica]) -> Vec<ResumoDoJogo> {
    let mut por_jogo: std::collections::BTreeMap<String, (String, Vec<&MedicaoAutomatica>)> =
        Default::default();
    for m in medicoes
        .iter()
        .filter(|m| m.confiavel && crate::modules::medicoes::comparavel(m))
    {
        let (id, nome) = id_do_jogo(&m.jogo);
        por_jogo
            .entry(id)
            .or_insert_with(|| (nome, Vec::new()))
            .1
            .push(m);
    }

    let mut resumos: Vec<ResumoDoJogo> = por_jogo
        .into_iter()
        .map(|(id, (nome, mut ms))| {
            ms.sort_by_key(|m| m.quando);
            let antes: Vec<&&MedicaoAutomatica> =
                ms.iter().filter(|m| m.mudancas_aplicadas == 0).collect();
            let depois: Vec<&&MedicaoAutomatica> =
                ms.iter().filter(|m| m.mudancas_aplicadas > 0).collect();
            let fps = |v: &[&&MedicaoAutomatica]| v.iter().map(|m| m.fps).collect::<Vec<_>>();
            let low = |v: &[&&MedicaoAutomatica]| v.iter().map(|m| m.low_1pct).collect::<Vec<_>>();

            ResumoDoJogo {
                partidas: ms.len(),
                ultima: ms.last().map(|m| partida(m)),
                ganho_fps: ganho(&fps(&antes), &fps(&depois)),
                ganho_low: ganho(&low(&antes), &low(&depois)),
                melhor_fps: ms
                    .iter()
                    .map(|m| m.fps)
                    .fold(None, |a: Option<f64>, f| Some(a.map_or(f, |a| a.max(f)))),
                id,
                nome,
            }
        })
        .collect();

    // O jogado por último primeiro: é o que a pessoa procura.
    resumos.sort_by(|a, b| {
        let qa = a.ultima.as_ref().map(|p| p.quando).unwrap_or(0);
        let qb = b.ultima.as_ref().map(|p| p.quando).unwrap_or(0);
        qb.cmp(&qa)
    });
    resumos
}

pub fn biblioteca() -> Result<Vec<ResumoDoJogo>, String> {
    Ok(resumir(&crate::modules::medicoes::ler_para_comparar()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(jogo: &str, quando: u64, fps: f64, low: f64, mudancas: usize) -> MedicaoAutomatica {
        let mut v: MedicaoAutomatica = serde_json::from_value(serde_json::json!({
            "jogo": jogo, "quando": quando, "fps": fps, "low_1pct": low, "engasgos_por_minuto": 0.0,
            "segundos": 20.0, "confiavel": true, "mudancas_aplicadas": mudancas
        }))
        .expect("medição de teste");
        v.frametime_medio_ms = Some(1000.0 / fps);
        v
    }

    #[test]
    fn builds_do_fivem_viram_um_jogo_so() {
        let r = resumir(&[
            m("FiveM_b3258_GTAProcess.exe", 1, 52.0, 37.0, 0),
            m("FiveM_b3570_GTAProcess.exe", 2, 53.0, 38.0, 0),
        ]);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "fivem");
        assert_eq!(r[0].partidas, 2);
    }

    #[test]
    fn ganho_so_aparece_com_partidas_dos_dois_lados() {
        let r = resumir(&[
            m("cs2.exe", 1, 200.0, 120.0, 0),
            m("cs2.exe", 2, 240.0, 150.0, 3),
        ]);
        assert!(matches!(
            r[0].ganho_fps,
            Ganho::Medindo {
                antes: 1,
                depois: 1
            }
        ));
    }

    #[test]
    fn ganho_real_vira_porcentagem_medida() {
        let mut ms = Vec::new();
        for (i, f) in [52.0, 51.0, 53.0].iter().enumerate() {
            ms.push(m(
                "FiveM_b1_GTAProcess.exe",
                i as u64,
                *f,
                37.0 + i as f64 * 0.5,
                0,
            ));
        }
        for (i, f) in [67.0, 66.0, 68.0].iter().enumerate() {
            ms.push(m(
                "FiveM_b1_GTAProcess.exe",
                10 + i as u64,
                *f,
                52.0 + i as f64 * 0.5,
                4,
            ));
        }
        match resumir(&ms)[0].ganho_fps {
            Ganho::Medido { pct, .. } => assert!((pct - 28.8).abs() < 1.0, "{}", pct),
            ref outro => panic!("{:?}", outro),
        }
    }

    #[test]
    fn diferenca_dentro_do_ruido_nao_vira_ganho() {
        let mut ms = Vec::new();
        for (i, f) in [60.0, 64.0, 58.0].iter().enumerate() {
            ms.push(m("cs2.exe", i as u64, *f, 40.0, 0));
        }
        for (i, f) in [61.0, 65.0, 59.0].iter().enumerate() {
            ms.push(m("cs2.exe", 10 + i as u64, *f, 40.0, 2));
        }
        assert_eq!(resumir(&ms)[0].ganho_fps, Ganho::SemDiferenca);
    }

    #[test]
    fn partida_nao_confiavel_fica_de_fora() {
        let mut ruim = m("cs2.exe", 1, 999.0, 1.0, 0);
        ruim.confiavel = false;
        assert!(resumir(&[ruim]).is_empty());
    }

    #[test]
    fn jogo_fora_do_catalogo_usa_o_executavel() {
        let r = resumir(&[m(r"D:\Jogos\Indie\meujogo.exe", 1, 90.0, 60.0, 0)]);
        assert_eq!(r[0].id, "meujogo.exe");
        assert_eq!(r[0].nome, "meujogo");
    }

    #[test]
    fn ultimo_jogado_vem_primeiro() {
        let r = resumir(&[
            m("cs2.exe", 5, 200.0, 100.0, 0),
            m("FiveM_b1_GTAProcess.exe", 9, 60.0, 40.0, 0),
        ]);
        assert_eq!(r[0].id, "fivem");
    }
}
