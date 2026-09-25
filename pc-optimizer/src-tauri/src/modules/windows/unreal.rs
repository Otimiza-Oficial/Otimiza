// Acha o `GameUserSettings.ini` de um jogo Unreal (seção `[ScalabilityGroups]`); quem lê é `tetos.rs`. O ajustador
// saiu na 3.0 com a Biblioteca; o desfazer segue no histórico (`ChangeRecord::GameConfig`).

use std::path::{Path, PathBuf};

// ------------------------------------------------------------ achar o arquivo

/// **Pura.** `...\<Projeto>\Binaries\Win64\<Nome>-Win64-Shipping.exe`: a pasta de configuração usa às vezes o
/// projeto (`FortniteGame`, `TslGame`), às vezes o executável (`VALORANT`).
pub fn candidatos(executavel: &Path) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(stem) = executavel.file_stem().and_then(|s| s.to_str()) {
        let base = stem.split("-Win64").next().unwrap_or(stem);
        v.push(base.to_string());
        if let Some(sem_client) = base.strip_suffix("Client") {
            v.push(format!("{}Game", sem_client));
            v.push(sem_client.to_string());
        }
    }
    let partes: Vec<String> = executavel
        .components()
        .filter_map(|c| c.as_os_str().to_str().map(str::to_string))
        .collect();
    if let Some(i) = partes.iter().position(|p| p.eq_ignore_ascii_case("Binaries")) {
        if i >= 1 {
            v.push(partes[i - 1].clone());
        }
        if i >= 2 {
            v.push(partes[i - 2].clone());
        }
    }
    v.retain(|s| !s.is_empty());
    v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    v
}

/// Procura `GameUserSettings.ini` sob `<pasta>\Saved\Config\` até três níveis.
fn configs_em(pasta: &Path) -> Vec<PathBuf> {
    fn descer(p: &Path, nivel: u8, v: &mut Vec<PathBuf>) {
        let Ok(entradas) = std::fs::read_dir(p) else { return };
        for e in entradas.flatten() {
            let c = e.path();
            if c.is_dir() && nivel < 3 {
                descer(&c, nivel + 1, v);
            } else if c.file_name().map(|n| n.eq_ignore_ascii_case("GameUserSettings.ini")).unwrap_or(false) {
                v.push(c);
            }
        }
    }
    let mut v = Vec::new();
    descer(&pasta.join("Saved").join("Config"), 0, &mut v);
    v
}

/// O `GameUserSettings.ini` deste jogo, se houver. Com mais de um (contas
/// diferentes, como no Valorant), o modificado por último — é o da conta
/// que jogou por último.
pub fn config_do_jogo(executavel: &Path) -> Option<PathBuf> {
    config_do_jogo_em(&PathBuf::from(std::env::var("LOCALAPPDATA").ok()?), executavel)
}

/// O mesmo, com a pasta de dados por parâmetro (para teste).
pub fn config_do_jogo_em(local: &Path, executavel: &Path) -> Option<PathBuf> {
    let mut achados: Vec<PathBuf> = Vec::new();
    for nome in candidatos(executavel) {
        let pasta = local.join(&nome);
        if pasta.is_dir() {
            achados.extend(configs_em(&pasta));
        }
    }
    achados
        .into_iter()
        .filter(|c| std::fs::read_to_string(c).map(|t| t.to_ascii_lowercase().contains("[scalabilitygroups]")).unwrap_or(false))
        .max_by_key(|c| std::fs::metadata(c).and_then(|m| m.modified()).ok())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn candidatos_pelo_executavel() {
        let fortnite = Path::new(r"C:\Epic\Fortnite\FortniteGame\Binaries\Win64\FortniteClient-Win64-Shipping.exe");
        let c = candidatos(fortnite);
        assert!(c.iter().any(|x| x == "FortniteGame"), "{:?}", c);
        let valorant = Path::new(r"C:\Riot Games\VALORANT\live\ShooterGame\Binaries\Win64\VALORANT-Win64-Shipping.exe");
        assert!(candidatos(valorant).iter().any(|x| x == "VALORANT"));
        let pubg = Path::new(r"C:\Steam\steamapps\common\PUBG\TslGame\Binaries\Win64\TslGame.exe");
        assert!(candidatos(pubg).iter().any(|x| x == "TslGame"));
    }

    #[test]
    fn acha_so_o_arquivo_com_a_secao_de_qualidade() {
        let local = std::env::temp_dir().join(format!("otimiza_unreal_{}", std::process::id()));
        let config = local.join("FortniteGame").join("Saved").join("Config").join("WindowsClient");
        std::fs::create_dir_all(&config).unwrap();
        let exe = Path::new(r"C:\Epic\Fortnite\FortniteGame\Binaries\Win64\FortniteClient-Win64-Shipping.exe");

        std::fs::write(config.join("GameUserSettings.ini"), "[/Script/Engine.GameUserSettings]\r\nbUseVSync=False\r\n").unwrap();
        assert_eq!(config_do_jogo_em(&local, exe), None, "sem [ScalabilityGroups] não é o arquivo");

        std::fs::write(config.join("GameUserSettings.ini"), "[ScalabilityGroups]\r\nsg.ShadowQuality=3\r\n").unwrap();
        assert_eq!(config_do_jogo_em(&local, exe), Some(config.join("GameUserSettings.ini")));

        let _ = std::fs::remove_dir_all(&local);
    }
}
