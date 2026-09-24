// Deriva de desempenho (2.9)
//
// O jogo que rodava a 140 na semana passada roda a 110 hoje, e ninguém mexeu
// em nada — só o driver de vídeo atualizou sozinho, ou o Windows. O cliente
// culpa o otimizador; o otimizador não sabe de nada.
//
// Cada medição automática passa a carregar a versão do driver de vídeo e do
// Windows daquele momento. Este módulo procura, por jogo, a última vez em que
// um dos dois mudou, e compara as partidas de antes e de depois com a regra
// comum do produto (`modules::repeticoes`: intervalos de 95% que não se
// tocam): piora com os intervalos separados e de pelo menos 5% vira
// "desempenho caiu depois de X", com os números dos dois lados.
//
// Sem mudança de driver ou de Windows, compara as 3 partidas mais recentes
// com as anteriores — a deriva sem culpado aparente (poeira, temperatura,
// algo novo rodando junto) também é informação.
//
// Ele não desfaz driver nem Windows: aponta, com número, e diz o que conferir.
// Função pura.

use serde::{Deserialize, Serialize};

use crate::modules::repeticoes::{comparar, resumir, Diferenca};
use crate::modules::medicoes::MedicaoAutomatica;

/// O ambiente em que uma medição foi feita.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ambiente {
    pub driver: Option<String>,
    pub windows: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo")]
pub enum OQueMudou {
    Driver { de: String, para: String },
    Windows { de: String, para: String },
    /// Nenhuma mudança registrada: só o tempo passou.
    Nada,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Deriva {
    pub jogo: String,
    pub mudou: OQueMudou,
    pub fps_antes: f64,
    pub fps_depois: f64,
    pub queda_pct: f64,
    pub partidas_antes: usize,
    pub partidas_depois: usize,
}

const MINIMO: usize = 3;
const MAXIMO: usize = 8;
const QUEDA_MINIMA_PCT: f64 = 5.0;

fn queda(antes: &[&MedicaoAutomatica], depois: &[&MedicaoAutomatica]) -> Option<(f64, f64, f64)> {
    let a = resumir("fps.average", &antes.iter().map(|m| m.fps).collect::<Vec<_>>())?;
    let d = resumir("fps.average", &depois.iter().map(|m| m.fps).collect::<Vec<_>>())?;
    match comparar(&a, &d) {
        Diferenca::Real { delta, pct: Some(p), .. } if delta < 0.0 && -p >= QUEDA_MINIMA_PCT => Some((a.media, d.media, -p)),
        _ => None,
    }
}

/// Procura deriva num jogo (medições já filtradas para ele, qualquer ordem).
pub fn procurar_no_jogo(jogo: &str, medicoes: &[MedicaoAutomatica]) -> Option<Deriva> {
    let mut ms: Vec<&MedicaoAutomatica> = medicoes.iter().collect();
    ms.sort_by_key(|m| m.quando);

    // A última troca de ambiente com partidas suficientes dos dois lados.
    for i in (1..ms.len()).rev() {
        let (a, b) = (&ms[i - 1].ambiente, &ms[i].ambiente);
        let mudou = match (a, b) {
            (Some(a), Some(b)) if a.driver.is_some() && b.driver.is_some() && a.driver != b.driver => OQueMudou::Driver {
                de: a.driver.clone().unwrap_or_default(),
                para: b.driver.clone().unwrap_or_default(),
            },
            (Some(a), Some(b)) if a.windows.is_some() && b.windows.is_some() && a.windows != b.windows => OQueMudou::Windows {
                de: a.windows.clone().unwrap_or_default(),
                para: b.windows.clone().unwrap_or_default(),
            },
            _ => continue,
        };
        let antes: Vec<&MedicaoAutomatica> = ms[..i].iter().rev().take(MAXIMO).copied().collect();
        let depois: Vec<&MedicaoAutomatica> = ms[i..].iter().take(MAXIMO).copied().collect();
        if antes.len() < MINIMO || depois.len() < MINIMO {
            continue;
        }
        return queda(&antes, &depois).map(|(fa, fd, q)| Deriva {
            jogo: jogo.to_string(),
            mudou,
            fps_antes: fa,
            fps_depois: fd,
            queda_pct: q,
            partidas_antes: antes.len(),
            partidas_depois: depois.len(),
        });
    }

    // Sem troca de ambiente: as 3 últimas contra as anteriores.
    if ms.len() >= MINIMO * 2 {
        let n = ms.len();
        let depois: Vec<&MedicaoAutomatica> = ms[n - MINIMO..].to_vec();
        let antes: Vec<&MedicaoAutomatica> = ms[..n - MINIMO].iter().rev().take(MAXIMO).copied().collect();
        return queda(&antes, &depois).map(|(fa, fd, q)| Deriva {
            jogo: jogo.to_string(),
            mudou: OQueMudou::Nada,
            fps_antes: fa,
            fps_depois: fd,
            queda_pct: q,
            partidas_antes: antes.len(),
            partidas_depois: depois.len(),
        });
    }
    None
}

/// Todas as derivas, uma por jogo.
pub fn procurar(medicoes: &[MedicaoAutomatica]) -> Vec<Deriva> {
    let mut jogos: Vec<String> = medicoes.iter().map(|m| m.jogo.to_lowercase()).collect();
    jogos.sort();
    jogos.dedup();
    jogos
        .into_iter()
        .filter_map(|j| {
            let do_jogo: Vec<MedicaoAutomatica> = medicoes.iter().filter(|m| m.jogo.to_lowercase() == j).cloned().collect();
            let nome = do_jogo.last().map(|m| m.jogo.clone()).unwrap_or(j);
            procurar_no_jogo(&nome, &do_jogo)
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn m(quando: u64, fps: f64, driver: &str) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: "Jogo.exe".into(),
            quando,
            fps,
            low_1pct: fps * 0.6,
            engasgos_por_minuto: 0.0,
            segundos: 30.0,
            placa: None,
            confiavel: true,
            mudancas_aplicadas: 0,
            frametime_medio_ms: None,
            frametime_p95_ms: None,
            frametime_p99_ms: None,
            cpu_uso_pct: None,
            gpu_uso_pct: None,
            trancos_com_disco_pct: None,
            trancos_medidos: None,
            governador: None,
            ambiente: Some(Ambiente { driver: Some(driver.into()), windows: Some("19045.1".into()) }),
        }
    }

    #[test]
    fn queda_depois_do_driver_novo() {
        let mut v = Vec::new();
        for (i, f) in [140.0, 142.0, 138.0, 141.0].iter().enumerate() {
            v.push(m(i as u64, *f, "32.0.15.6094"));
        }
        for (i, f) in [112.0, 110.0, 113.0].iter().enumerate() {
            v.push(m(100 + i as u64, *f, "32.0.15.7000"));
        }
        let d = procurar_no_jogo("Jogo.exe", &v).expect("deriva");
        assert_eq!(d.mudou, OQueMudou::Driver { de: "32.0.15.6094".into(), para: "32.0.15.7000".into() });
        assert!(d.queda_pct > 18.0);
    }

    #[test]
    fn driver_novo_sem_queda_nao_acusa() {
        let mut v = Vec::new();
        for (i, f) in [140.0, 142.0, 138.0].iter().enumerate() {
            v.push(m(i as u64, *f, "a"));
        }
        for (i, f) in [141.0, 139.0, 143.0].iter().enumerate() {
            v.push(m(100 + i as u64, *f, "b"));
        }
        assert!(procurar_no_jogo("Jogo.exe", &v).is_none());
    }

    #[test]
    fn deriva_sem_culpado() {
        let mut v = Vec::new();
        for (i, f) in [140.0, 142.0, 138.0, 141.0, 139.0].iter().enumerate() {
            v.push(m(i as u64, *f, "a"));
        }
        for (i, f) in [118.0, 117.0, 119.0].iter().enumerate() {
            v.push(m(100 + i as u64, *f, "a"));
        }
        assert_eq!(procurar_no_jogo("Jogo.exe", &v).unwrap().mudou, OQueMudou::Nada);
    }

    #[test]
    fn medicao_antiga_sem_ambiente_nao_quebra() {
        let mut v: Vec<MedicaoAutomatica> = (0..6).map(|i| MedicaoAutomatica { ambiente: None, ..m(i, 100.0, "a") }).collect();
        v.push(m(10, 100.0, "a"));
        assert!(procurar(&v).is_empty());
    }
}
