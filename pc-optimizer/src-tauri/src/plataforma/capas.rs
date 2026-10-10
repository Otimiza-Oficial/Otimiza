// As capas dos jogos vêm do próprio PC: a Steam guarda a arte vertical de cada jogo da biblioteca em
// `appcache\librarycache`. Nada é baixado. Jogo sem Steam (FiveM, Valorant) fica com a capa desenhada pela tela.
//
// A Steam mudou o formato em 2024: antes `<appid>_library_600x900.jpg` solto; agora uma pasta `<appid>` com a arte
// dentro (às vezes numa subpasta). Os dois são procurados.

use base64::Engine;
use std::path::{Path, PathBuf};

/// Arte maior que isto não é capa (e não vai pelo IPC como texto).
const TAMANHO_MAXIMO: u64 = 3 * 1024 * 1024;

/// Pura sobre a lista de nomes: a ordem de preferência da arte vertical, depois a horizontal.
pub fn melhor_arquivo(appid: u32, nomes: &[String]) -> Option<String> {
    let preferidos = [
        format!("{}_library_600x900_2x.jpg", appid),
        format!("{}_library_600x900.jpg", appid),
        "library_600x900_2x.jpg".to_string(),
        "library_600x900.jpg".to_string(),
        format!("{}_header.jpg", appid),
        "header.jpg".to_string(),
    ];
    preferidos
        .iter()
        .find_map(|p| nomes.iter().find(|n| n.to_lowercase() == *p).cloned())
}

#[cfg(target_os = "windows")]
fn pasta_da_steam() -> Option<PathBuf> {
    let caminho =
        crate::modules::windows::registry::read_text("HKCU", r"Software\Valve\Steam", "SteamPath")
            .ok()??;
    let p = PathBuf::from(caminho.replace('/', "\\"));
    p.is_dir().then_some(p)
}

#[cfg(not(target_os = "windows"))]
fn pasta_da_steam() -> Option<PathBuf> {
    None
}

fn arquivos(p: &Path) -> Vec<(String, PathBuf)> {
    std::fs::read_dir(p)
        .map(|it| {
            it.flatten()
                .take(500)
                .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn achar(appid: u32) -> Option<PathBuf> {
    achar_com(appid, |nomes| melhor_arquivo(appid, nomes))
}

/// **Pura sobre a lista de nomes.** A arte horizontal grande (o topo da página do jogo na Steam): é a que enche o
/// topo do Início.
pub fn melhor_arquivo_largo(appid: u32, nomes: &[String]) -> Option<String> {
    let preferidos = [
        format!("{}_library_hero.jpg", appid),
        "library_hero.jpg".to_string(),
    ];
    preferidos
        .iter()
        .find_map(|p| nomes.iter().find(|n| n.to_lowercase() == *p).cloned())
}

fn achar_com(appid: u32, escolher: impl Fn(&[String]) -> Option<String>) -> Option<PathBuf> {
    let cache = pasta_da_steam()?.join("appcache").join("librarycache");

    // Formato antigo: solto na pasta.
    let soltos = arquivos(&cache);
    let nomes: Vec<String> = soltos
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| n.starts_with(&appid.to_string()))
        .collect();
    if let Some(n) = escolher(&nomes) {
        return soltos.into_iter().find(|(x, _)| *x == n).map(|(_, p)| p);
    }

    // Formato novo: a pasta do jogo e uma subpasta abaixo.
    let pasta = cache.join(appid.to_string());
    let mut candidatos = arquivos(&pasta);
    for (_, sub) in arquivos(&pasta).into_iter().filter(|(_, p)| p.is_dir()) {
        candidatos.extend(arquivos(&sub));
    }
    let nomes: Vec<String> = candidatos.iter().map(|(n, _)| n.clone()).collect();
    let n = escolher(&nomes)?;
    candidatos
        .into_iter()
        .find(|(x, _)| *x == n)
        .map(|(_, p)| p)
}

/// A capa como `data:image/jpeg;base64,...`, pronta para um `<img>` ou para a textura da cena 3D.
pub fn capa(appid: u32) -> Option<String> {
    em_base64(&achar(appid)?)
}

/// A arte larga, para o topo do Início. Sem ela, `None`: a tela usa a vertical.
pub fn capa_larga(appid: u32) -> Option<String> {
    em_base64(&achar_com(appid, |nomes| {
        melhor_arquivo_largo(appid, nomes)
    })?)
}

fn em_base64(caminho: &Path) -> Option<String> {
    if std::fs::metadata(caminho).ok()?.len() > TAMANHO_MAXIMO {
        return None;
    }
    let bytes = std::fs::read(caminho).ok()?;
    Some(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nomes(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_arte_larga_e_o_topo_da_pagina_do_jogo() {
        // A pasta do GTA V no PC do dono (formato de 2024).
        let n = nomes(&[
            "library_600x900.jpg",
            "library_hero.jpg",
            "library_hero_blur.jpg",
        ]);
        assert_eq!(
            melhor_arquivo_largo(271590, &n).as_deref(),
            Some("library_hero.jpg")
        );
        assert_eq!(
            melhor_arquivo_largo(271590, &nomes(&["library_600x900.jpg"])),
            None
        );
        let antigo = nomes(&["271590_library_hero.jpg"]);
        assert_eq!(
            melhor_arquivo_largo(271590, &antigo).as_deref(),
            Some("271590_library_hero.jpg")
        );
    }

    #[test]
    fn prefere_a_arte_vertical_grande() {
        let n = nomes(&[
            "730_header.jpg",
            "730_library_600x900.jpg",
            "730_library_600x900_2x.jpg",
        ]);
        assert_eq!(
            melhor_arquivo(730, &n).as_deref(),
            Some("730_library_600x900_2x.jpg")
        );
    }

    #[test]
    fn formato_novo_sem_o_appid_no_nome() {
        let n = nomes(&["logo.png", "library_600x900.jpg"]);
        assert_eq!(
            melhor_arquivo(730, &n).as_deref(),
            Some("library_600x900.jpg")
        );
    }

    #[test]
    fn sem_vertical_usa_a_horizontal() {
        assert_eq!(
            melhor_arquivo(1, &nomes(&["header.jpg"])).as_deref(),
            Some("header.jpg")
        );
    }

    #[test]
    fn arte_de_outro_jogo_nao_serve() {
        assert_eq!(
            melhor_arquivo(730, &nomes(&["570_library_600x900.jpg"])),
            None
        );
    }

    #[test]
    #[ignore = "depende da Steam desta máquina"]
    fn mostra_a_capa_do_cs2_nesta_maquina() {
        println!("{:?}", achar(730));
    }
}
