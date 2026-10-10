// "Testar performance": várias rodadas antes, a pessoa otimiza, várias rodadas depois, e a comparação diz quanto dá
// para confiar. Uma rodada sozinha não prova nada (uma noite pesada no servidor muda 10%): o mínimo é o de
// `repeticoes` por lado, e o resultado sai com o nível de confiança, não só com um número.
//
// Medir não muda o computador. O Lab só guarda as rodadas em `lab.json`, de um jogo por vez.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::modules::repeticoes::{self, Diferenca, REPETICOES_MINIMAS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lado {
    Antes,
    Depois,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rodada {
    pub quando: u64,
    pub fps: f64,
    pub low_1pct: Option<f64>,
    pub frametime_ms: f64,
    pub p99_ms: f64,
    pub engasgos_por_minuto: f64,
    /// Com geração de quadros visível: a rodada vale para o Frame Boost, não para a comparação de otimização.
    pub com_geracao: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Teste {
    /// A chave do jogo (`historico::id_do_jogo`): rodada de outro jogo não entra.
    pub jogo: String,
    pub nome: String,
    pub antes: Vec<Rodada>,
    pub depois: Vec<Rodada>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Confianca {
    /// Separados com folga de 3% ou mais da média.
    Alta,
    /// Separados, mas por pouco.
    Media,
    /// Os intervalos se encostam: a mudança não foi maior que o ruído.
    SemDiferenca,
    /// Faltam rodadas.
    Insuficiente,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Linha {
    pub metrica: &'static str,
    pub antes: Option<f64>,
    pub depois: Option<f64>,
    /// Só com confiança alta ou média.
    pub diferenca_pct: Option<f64>,
    pub confianca: Confianca,
    /// Para frametime, menos é melhor.
    pub menor_e_melhor: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Resultado {
    pub nome: String,
    pub rodadas_antes: usize,
    pub rodadas_depois: usize,
    pub minimo_por_lado: usize,
    pub linhas: Vec<Linha>,
    /// Rodadas descartadas por ter geração de quadros no meio.
    pub descartadas: usize,
}

fn linha(metrica: &'static str, antes: &[f64], depois: &[f64], menor_e_melhor: bool) -> Linha {
    let a = repeticoes::resumir(metrica, antes);
    let d = repeticoes::resumir(metrica, depois);
    let (confianca, pct) = match (&a, &d) {
        (Some(a), Some(d)) => match repeticoes::comparar(a, d) {
            Diferenca::Real { pct, folga, .. } => {
                let folga_pct = if a.media != 0.0 {
                    folga / a.media.abs() * 100.0
                } else {
                    0.0
                };
                (
                    if folga_pct >= 3.0 {
                        Confianca::Alta
                    } else {
                        Confianca::Media
                    },
                    pct,
                )
            }
            Diferenca::Indistinguivel { .. } => (Confianca::SemDiferenca, None),
            Diferenca::SemRepeticoes { .. } => (Confianca::Insuficiente, None),
        },
        _ => (Confianca::Insuficiente, None),
    };
    Linha {
        metrica,
        antes: a.map(|r| r.media),
        depois: d.map(|r| r.media),
        diferenca_pct: pct,
        confianca,
        menor_e_melhor,
    }
}

/// Pura. Só rodadas sem geração entram: com gerador ligado o FPS do jogo cai, e a otimização pareceria piora.
pub fn comparar(t: &Teste) -> Resultado {
    let limpas = |v: &[Rodada]| {
        v.iter()
            .filter(|r| !r.com_geracao)
            .cloned()
            .collect::<Vec<_>>()
    };
    let a = limpas(&t.antes);
    let d = limpas(&t.depois);
    let col =
        |v: &[Rodada], f: fn(&Rodada) -> Option<f64>| v.iter().filter_map(f).collect::<Vec<_>>();

    Resultado {
        nome: t.nome.clone(),
        rodadas_antes: a.len(),
        rodadas_depois: d.len(),
        minimo_por_lado: REPETICOES_MINIMAS,
        descartadas: t.antes.len() + t.depois.len() - a.len() - d.len(),
        linhas: vec![
            linha(
                "FPS médio",
                &col(&a, |r| Some(r.fps)),
                &col(&d, |r| Some(r.fps)),
                false,
            ),
            linha(
                "1% low",
                &col(&a, |r| r.low_1pct),
                &col(&d, |r| r.low_1pct),
                false,
            ),
            linha(
                "Tempo de quadro",
                &col(&a, |r| Some(r.frametime_ms)),
                &col(&d, |r| Some(r.frametime_ms)),
                true,
            ),
            linha(
                "Pior 1% do tempo de quadro",
                &col(&a, |r| Some(r.p99_ms)),
                &col(&d, |r| Some(r.p99_ms)),
                true,
            ),
            linha(
                "Engasgos por minuto",
                &col(&a, |r| Some(r.engasgos_por_minuto)),
                &col(&d, |r| Some(r.engasgos_por_minuto)),
                true,
            ),
        ],
    }
}

/// Pura. Acrescenta a rodada; outro jogo começa um teste novo, e "antes" depois de já haver "depois" também (a pessoa
/// recomeçou).
pub fn acrescentar(t: &mut Teste, jogo: &str, nome: &str, lado: Lado, r: Rodada) {
    if t.jogo != jogo || (lado == Lado::Antes && !t.depois.is_empty()) {
        *t = Teste {
            jogo: jogo.to_string(),
            nome: nome.to_string(),
            ..Teste::default()
        };
    }
    match lado {
        Lado::Antes => t.antes.push(r),
        Lado::Depois => t.depois.push(r),
    }
}

fn caminho() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("pc-optimizer").join("lab.json")
}

fn ler_de(c: &Path) -> Teste {
    fs::read_to_string(c)
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

fn gravar_em(c: &Path, t: &Teste) -> Result<(), String> {
    if let Some(p) = c.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(t).map_err(|e| e.to_string())?;
    let tmp = crate::modules::changelog::caminho_temporario(c);
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, c).map_err(|e| e.to_string())
}

pub fn atual() -> Teste {
    ler_de(&caminho())
}

pub fn zerar() -> Result<(), String> {
    gravar_em(&caminho(), &Teste::default())
}

/// Mede uma rodada com o PresentMon no processo do jogo e guarda no lado pedido.
#[cfg(target_os = "windows")]
pub fn medir_rodada(processo: &str, lado: Lado, segundos: u32) -> Result<Teste, String> {
    use crate::modules::windows::{frames, presentmon};

    let (pid, nome) = frames::encontrar_processo(processo).ok_or_else(|| {
        format!(
            "Não encontrei o jogo `{}` aberto. Abra o jogo e entre na partida.",
            processo
        )
    })?;
    let (_, r) = presentmon::medir_como_antes(pid, &nome, segundos.clamp(10, 120))?;

    let rodada = Rodada {
        quando: crate::modules::changelog::now_timestamp(),
        fps: r.fps_do_jogo_medio,
        low_1pct: r.low_1pct,
        frametime_ms: r.quadro_medio_ms,
        p99_ms: r.quadro_p99_ms,
        engasgos_por_minuto: r.engasgos_por_minuto,
        com_geracao: !matches!(
            crate::modules::medicoes::geracao_agora(Some(&r)),
            crate::modules::medicoes::GeracaoNaPartida::NenhumaVisivel
        ),
    };

    let (id, nome_do_jogo) = super::historico::id_do_jogo(&nome);
    let mut t = atual();
    acrescentar(&mut t, &id, &nome_do_jogo, lado, rodada);
    gravar_em(&caminho(), &t)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(fps: f64) -> Rodada {
        Rodada {
            quando: 0,
            fps,
            low_1pct: Some(fps * 0.7),
            frametime_ms: 1000.0 / fps,
            p99_ms: 2000.0 / fps,
            engasgos_por_minuto: 1.0,
            com_geracao: false,
        }
    }

    fn teste(antes: &[f64], depois: &[f64]) -> Teste {
        Teste {
            jogo: "fivem".to_string(),
            nome: "FiveM".to_string(),
            antes: antes.iter().map(|f| r(*f)).collect(),
            depois: depois.iter().map(|f| r(*f)).collect(),
        }
    }

    #[test]
    fn ganho_grande_com_rodadas_suficientes_e_confianca_alta() {
        let res = comparar(&teste(&[52.0, 51.0, 53.0], &[67.0, 66.0, 68.0]));
        let fps = &res.linhas[0];
        assert_eq!(fps.confianca, Confianca::Alta);
        assert!((fps.diferenca_pct.unwrap() - 28.8).abs() < 1.0);
        let tempo = &res.linhas[2];
        assert!(tempo.menor_e_melhor);
        assert!(tempo.diferenca_pct.unwrap() < 0.0, "tempo de quadro caiu");
    }

    #[test]
    fn duas_rodadas_por_lado_nao_bastam() {
        let res = comparar(&teste(&[52.0, 51.0], &[67.0, 66.0]));
        assert_eq!(res.linhas[0].confianca, Confianca::Insuficiente);
        assert_eq!(
            res.linhas[0].diferenca_pct, None,
            "sem confiança, sem número"
        );
    }

    #[test]
    fn ruido_vira_sem_diferenca() {
        let res = comparar(&teste(&[60.0, 66.0, 56.0], &[61.0, 67.0, 57.0]));
        assert_eq!(res.linhas[0].confianca, Confianca::SemDiferenca);
    }

    #[test]
    fn rodada_com_geracao_fica_de_fora_e_e_contada() {
        let mut t = teste(&[52.0, 51.0, 53.0], &[67.0, 66.0, 68.0]);
        let mut g = r(120.0);
        g.com_geracao = true;
        t.depois.push(g);
        let res = comparar(&t);
        assert_eq!(res.rodadas_depois, 3);
        assert_eq!(res.descartadas, 1);
    }

    #[test]
    fn outro_jogo_ou_recomeco_zera_o_teste() {
        let mut t = teste(&[52.0], &[67.0]);
        acrescentar(&mut t, "cs2.exe", "cs2", Lado::Antes, r(200.0));
        assert_eq!(t.jogo, "cs2.exe");
        assert_eq!((t.antes.len(), t.depois.len()), (1, 0));

        acrescentar(&mut t, "cs2.exe", "cs2", Lado::Depois, r(230.0));
        acrescentar(&mut t, "cs2.exe", "cs2", Lado::Antes, r(201.0));
        assert_eq!(
            (t.antes.len(), t.depois.len()),
            (1, 0),
            "antes depois de depois recomeça"
        );
    }

    #[test]
    fn gravar_e_ler_devolve_o_mesmo() {
        let dir = std::env::temp_dir().join("otimiza-lab");
        let _ = fs::remove_dir_all(&dir);
        let c = dir.join("lab.json");
        // Números exatos: o serde_json padrão não garante ida e volta do último bit de 1000/52.
        let exata = Rodada {
            quando: 9,
            fps: 50.0,
            low_1pct: Some(35.5),
            frametime_ms: 20.0,
            p99_ms: 40.0,
            engasgos_por_minuto: 0.5,
            com_geracao: false,
        };
        let t = Teste {
            jogo: "fivem".to_string(),
            nome: "FiveM".to_string(),
            antes: vec![exata.clone()],
            depois: vec![exata],
        };
        gravar_em(&c, &t).unwrap();
        assert_eq!(ler_de(&c), t);
    }
}
