// Que jogos existem nesta máquina. Principal motivo: SEGURANÇA. A escrita em IFEO é o mecanismo de sequestrar
// execução, e só vale para executável DENTRO de uma biblioteca de jogo. `UserGpuPreferences` do Windows NUNCA é
// limpa (listava um Fortnite desinstalado): entra como histórico, e cada caminho é conferido no disco.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origem {
    Steam,
    Epic,
    Windows,
    /// Riot, Blizzard, EA, Ubisoft, GOG, Rockstar, Roblox e FiveM registram aqui: um leitor para todas.
    Instalado,
    /// Garante que nenhum jogo fica de fora por não estar numa loja conhecida.
    Detectado,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JogoInstalado {
    pub nome: String,
    pub origem: Origem,
    pub pasta: PathBuf,
    pub executavel: Option<PathBuf>,
    pub ultima_vez: u64,
    pub bytes: u64,
    #[serde(default)]
    pub appid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Biblioteca {
    pub jogos: Vec<JogoInstalado>,
    /// Autoriza a escrita em IFEO: executável fora de todas não é jogo instalado.
    pub raizes: Vec<PathBuf>,
    pub lacunas: Vec<String>,
    /// Não é raiz de biblioteca: os jogos podem estar em outro disco.
    #[serde(default)]
    pub raiz_steam: Option<PathBuf>,
}

/// Formato `"chave"<tab>"valor"`, um par por linha. Sem biblioteca de VDF: um analisador completo teria mais
/// superfície de erro que este.
pub fn valor_vdf(conteudo: &str, chave: &str) -> Option<String> {
    pares_vdf(conteudo)
        .into_iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(chave))
        .map(|(_, v)| v)
}

pub fn pares_vdf(conteudo: &str) -> Vec<(String, String)> {
    let mut pares = Vec::new();

    for linha in conteudo.lines() {
        let mut partes = linha.split('"').skip(1).step_by(2);

        let (Some(chave), Some(valor)) = (partes.next(), partes.next()) else {
            continue;
        };

        // Terceiro campo na linha: não é par simples, e interpretar traria lixo.
        if partes.next().is_some() {
            continue;
        }

        pares.push((chave.to_string(), valor.to_string()));
    }

    pares
}

/// O jogo pesado costuma estar num HD separado.
pub fn raizes_steam(libraryfolders: &str) -> Vec<PathBuf> {
    pares_vdf(libraryfolders)
        .into_iter()
        .filter(|(chave, _)| chave.eq_ignore_ascii_case("path"))
        .map(|(_, valor)| PathBuf::from(valor.replace("\\\\", "\\")))
        .collect()
}

pub fn jogo_do_manifest(conteudo: &str, raiz: &Path) -> Option<JogoInstalado> {
    let nome = valor_vdf(conteudo, "name")?;
    let pasta_relativa = valor_vdf(conteudo, "installdir")?;

    if nome.trim().is_empty() || pasta_relativa.trim().is_empty() {
        return None;
    }

    let numero = |chave: &str| {
        valor_vdf(conteudo, chave)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(0)
    };

    Some(JogoInstalado {
        nome,
        origem: Origem::Steam,
        pasta: raiz.join("steamapps").join("common").join(pasta_relativa),
        // A Steam não guarda o executável; varrer a pasta é caro e fica para quem precisar.
        executavel: None,
        ultima_vez: numero("LastPlayed"),
        bytes: numero("SizeOnDisk"),
        appid: valor_vdf(conteudo, "appid").and_then(|v| v.trim().parse().ok()),
    })
}

#[cfg(target_os = "windows")]
fn pasta_da_steam() -> Option<PathBuf> {
    use crate::modules::changelog::PreviousValue;

    let PreviousValue::Text(caminho) =
        super::registry::read("HKCU", r"SOFTWARE\Valve\Steam", "SteamPath").ok()?
    else {
        return None;
    };

    Some(PathBuf::from(caminho.replace('/', "\\")))
}

#[cfg(not(target_os = "windows"))]
fn pasta_da_steam() -> Option<PathBuf> {
    None
}

fn ler_steam(biblioteca: &mut Biblioteca) {
    let Some(steam) = pasta_da_steam() else {
        return;
    };

    // Guardada ANTES de qualquer leitura poder falhar.
    biblioteca.raiz_steam = Some(steam.clone());

    let arquivo = steam.join("steamapps").join("libraryfolders.vdf");

    let Ok(conteudo) = std::fs::read_to_string(&arquivo) else {
        biblioteca.lacunas.push(
            "A Steam está instalada, mas a lista de bibliotecas dela não pôde ser lida."
                .to_string(),
        );
        return;
    };

    for raiz in raizes_steam(&conteudo) {
        if !raiz.exists() {
            // Disco removido ou pendrive fora: não é erro, é raiz que não vale hoje.
            continue;
        }

        biblioteca.raizes.push(raiz.clone());

        let Ok(entradas) = std::fs::read_dir(raiz.join("steamapps")) else {
            continue;
        };

        for entrada in entradas.flatten() {
            let caminho = entrada.path();
            let nome_arquivo = caminho.file_name().unwrap_or_default().to_string_lossy();

            if !nome_arquivo.starts_with("appmanifest_") || !nome_arquivo.ends_with(".acf") {
                continue;
            }

            let Ok(conteudo) = std::fs::read_to_string(&caminho) else {
                continue;
            };

            if let Some(jogo) = jogo_do_manifest(&conteudo, &raiz) {
                // Pacotes de sistema que a Steam instala como jogo, com pasta e manifest.
                if e_pacote_de_sistema(&jogo.nome) {
                    continue;
                }

                if jogo.pasta.exists() {
                    biblioteca.jogos.push(jogo);
                }
            }
        }
    }
}

fn e_pacote_de_sistema(nome: &str) -> bool {
    let minusculo = nome.to_lowercase();

    minusculo.contains("redistributable")
        || minusculo.contains("steamworks")
        || minusculo.starts_with("proton")
        || minusculo.contains("steam linux runtime")
}

fn ler_epic(biblioteca: &mut Biblioteca) {
    let Ok(dados) = std::env::var("PROGRAMDATA") else {
        return;
    };

    let pasta = PathBuf::from(dados)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");

    let Ok(entradas) = std::fs::read_dir(&pasta) else {
        return;
    };

    for entrada in entradas.flatten() {
        let caminho = entrada.path();

        if caminho.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }

        let Ok(conteudo) = std::fs::read_to_string(&caminho) else {
            continue;
        };

        let Ok(json) = serde_json::from_str::<serde_json::Value>(&conteudo) else {
            continue;
        };

        let (Some(nome), Some(local)) = (
            json.get("DisplayName").and_then(|v| v.as_str()),
            json.get("InstallLocation").and_then(|v| v.as_str()),
        ) else {
            continue;
        };

        let pasta_do_jogo = PathBuf::from(local);

        if !pasta_do_jogo.exists() {
            continue;
        }

        let executavel = json
            .get("LaunchExecutable")
            .and_then(|v| v.as_str())
            .map(|relativo| pasta_do_jogo.join(relativo.replace('/', "\\")))
            .filter(|caminho| caminho.exists());

        biblioteca.raizes.push(pasta_do_jogo.clone());
        biblioteca.jogos.push(JogoInstalado {
            nome: nome.to_string(),
            origem: Origem::Epic,
            pasta: pasta_do_jogo,
            executavel,
            ultima_vez: 0,
            bytes: 0,
            appid: None,
        });
    }
}

/// O Windows nunca limpa esta lista (ver o topo do arquivo).
#[cfg(target_os = "windows")]
fn ler_windows(biblioteca: &mut Biblioteca) {
    const CHAVE: &str = r"SOFTWARE\Microsoft\DirectX\UserGpuPreferences";

    let caminhos = match super::registry::value_names("HKCU", CHAVE) {
        Ok(caminhos) => caminhos,
        Err(erro) => {
            biblioteca
                .lacunas
                .push(format!("Preferências de placa de vídeo do Windows: {}", erro));
            return;
        }
    };

    for caminho_texto in caminhos {
        // Só entra se o arquivo ainda existir.
        let executavel = PathBuf::from(&caminho_texto);

        if !executavel.exists() {
            continue;
        }

        let Some(pasta) = executavel.parent().map(PathBuf::from) else {
            continue;
        };

        if biblioteca
            .jogos
            .iter()
            .any(|j| executavel.starts_with(&j.pasta))
        {
            continue;
        }

        let nome = executavel
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "programa desconhecido".to_string());

        biblioteca.jogos.push(JogoInstalado {
            nome,
            origem: Origem::Windows,
            pasta,
            executavel: Some(executavel),
            ultima_vez: 0,
            bytes: 0,
            appid: None,
        });
    }
}

#[cfg(not(target_os = "windows"))]
fn ler_windows(_biblioteca: &mut Biblioteca) {}

pub fn varrer() -> Biblioteca {
    let mut biblioteca = Biblioteca::default();

    ler_steam(&mut biblioteca);
    ler_epic(&mut biblioteca);
    ler_instalados(&mut biblioteca);
    ler_detectados(&mut biblioteca);
    ler_windows(&mut biblioteca);

    biblioteca.jogos.sort_by(|a, b| b.ultima_vez.cmp(&a.ultima_vez));
    biblioteca.raizes.sort();
    biblioteca.raizes.dedup();

    biblioteca
}

const EDITORAS_DE_JOGO: &[&str] = &[
    "riot games", "blizzard", "activision", "electronic arts", "ubisoft", "gog.com", "rockstar",
    "roblox", "mojang", "cfx.re", "bethesda", "hoyoverse", "mihoyo", "garena", "krafton",
    "bandai namco", "square enix", "capcom", "warner bros", "2k", "cd projekt", "nexon", "wargaming",
    "embark studios", "respawn", "bungie", "epic games", "valve",
];

const NAO_E_JOGO: &[&str] = &[
    "launcher", "sdk", "studio", "redistributable", "riot client", "battle.net", "ea app",
    "ubisoft connect", "gog galaxy", "social club", "epic online services", "steam", "anti-cheat",
    "anticheat", "vanguard", "easyanticheat", "battleye", "uninstall", "setup",
];

pub fn instalado_e_jogo(nome: &str, editora: &str) -> bool {
    let editora = editora.to_lowercase();
    let nome = nome.to_lowercase();
    EDITORAS_DE_JOGO.iter().any(|e| editora.contains(e)) && !NAO_E_JOGO.iter().any(|n| nome.contains(n))
}

pub fn executavel_do_icone(icone: &str) -> Option<PathBuf> {
    let limpo = icone.trim();
    let sem_indice = match limpo.rfind(',') {
        Some(i) if limpo[i + 1..].trim().parse::<i32>().is_ok() => &limpo[..i],
        _ => limpo,
    };
    let caminho = sem_indice.trim().trim_matches('"');
    let minusculo = caminho.to_lowercase();
    (minusculo.ends_with(".exe") && !minusculo.contains("install") && !minusculo.contains("setup"))
        .then(|| PathBuf::from(caminho))
}

#[cfg(target_os = "windows")]
fn ler_instalados(biblioteca: &mut Biblioteca) {
    use super::registry::{read_text, subkeys};
    const RAIZES: &[(&str, &str)] = &[
        ("HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        ("HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
        ("HKCU", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];
    for (hive, raiz) in RAIZES {
        let chaves = match subkeys(hive, raiz) {
            Ok(c) => c,
            Err(erro) => {
                biblioteca.lacunas.push(format!("Programas instalados: {}", erro));
                continue;
            }
        };
        for chave in chaves {
            let caminho = format!("{}\\{}", raiz, chave);
            let ler = |valor: &str| read_text(hive, &caminho, valor).ok().flatten().unwrap_or_default();
            let (nome, editora) = (ler("DisplayName"), ler("Publisher"));
            if nome.is_empty() || !instalado_e_jogo(&nome, &editora) {
                continue;
            }
            let executavel = executavel_do_icone(&ler("DisplayIcon")).filter(|e| e.exists());
            let local = ler("InstallLocation");
            let pasta = if !local.trim().is_empty() {
                PathBuf::from(local.trim().trim_matches('"'))
            } else if let Some(pai) = executavel.as_ref().and_then(|e| e.parent()) {
                pai.to_path_buf()
            } else {
                continue;
            };
            if !pasta.exists() {
                continue;
            }
            if biblioteca.jogos.iter().any(|j| j.pasta == pasta || pasta.starts_with(&j.pasta)) {
                continue;
            }
            // NÃO entra em `raizes`: só biblioteca lida do arquivo da própria loja autoriza IFEO.
            biblioteca.jogos.push(JogoInstalado {
                nome,
                origem: Origem::Instalado,
                pasta,
                executavel,
                ultima_vez: 0,
                bytes: 0,
                appid: None,
            });
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn ler_instalados(_biblioteca: &mut Biblioteca) {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JogoVisto {
    pub nome: String,
    pub executavel: PathBuf,
    pub primeira_vez: u64,
    pub ultima_vez: u64,
    pub vezes: u32,
}

fn arquivo_de_vistos() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("pc-optimizer").join("jogos_vistos.json"))
}

/// Ilegível é erro, nunca "nenhum jogo" em silêncio.
pub fn ler_vistos_de(caminho: &Path) -> Result<Vec<JogoVisto>, String> {
    match std::fs::read_to_string(caminho) {
        Ok(texto) => serde_json::from_str(&texto).map_err(|e| format!("jogos_vistos.json ilegível: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("jogos_vistos.json: {}", e)),
    }
}

pub fn anotar_visto(lista: &mut Vec<JogoVisto>, nome: &str, executavel: &Path, agora: u64) -> bool {
    const UMA_HORA: u64 = 3600;
    if let Some(v) = lista.iter_mut().find(|v| v.executavel.as_os_str().eq_ignore_ascii_case(executavel.as_os_str())) {
        if agora.saturating_sub(v.ultima_vez) < UMA_HORA {
            return false;
        }
        v.ultima_vez = agora;
        v.vezes = v.vezes.saturating_add(1);
        v.nome = nome.to_string();
        return true;
    }
    lista.push(JogoVisto {
        nome: nome.to_string(),
        executavel: executavel.to_path_buf(),
        primeira_vez: agora,
        ultima_vez: agora,
        vezes: 1,
    });
    true
}

pub fn registrar_visto(nome: &str, executavel: &Path) {
    use std::sync::Mutex;
    static RECENTES: Mutex<Vec<(PathBuf, u64)>> = Mutex::new(Vec::new());
    let agora = crate::modules::changelog::now_timestamp();
    if let Ok(recentes) = RECENTES.lock() {
        if recentes.iter().any(|(e, t)| e == executavel && agora.saturating_sub(*t) < 3600) {
            return;
        }
    }
    let Some(arquivo) = arquivo_de_vistos() else { return };
    // Ilegível: não sobrescreve (apagaria o histórico).
    let mut lista = match ler_vistos_de(&arquivo) {
        Ok(l) => l,
        Err(e) => {
            crate::utils::Logger::info(&format!("biblioteca: {}", e));
            return;
        }
    };
    if anotar_visto(&mut lista, nome, executavel, agora) {
        if let Some(pasta) = arquivo.parent() {
            let _ = std::fs::create_dir_all(pasta);
        }
        match serde_json::to_string_pretty(&lista) {
            Ok(json) => {
                let temporario = arquivo.with_extension("json.tmp");
                if std::fs::write(&temporario, json).is_ok() {
                    let _ = std::fs::rename(&temporario, &arquivo);
                }
            }
            Err(e) => crate::utils::Logger::info(&format!("biblioteca: {}", e)),
        }
    }
    if let Ok(mut recentes) = RECENTES.lock() {
        recentes.retain(|(e, _)| e != executavel);
        recentes.push((executavel.to_path_buf(), agora));
    }
}

fn ler_detectados(biblioteca: &mut Biblioteca) {
    let Some(arquivo) = arquivo_de_vistos() else { return };
    let vistos = match ler_vistos_de(&arquivo) {
        Ok(v) => v,
        Err(e) => {
            biblioteca.lacunas.push(format!("Jogos vistos rodando: {}", e));
            return;
        }
    };
    for v in vistos {
        if !v.executavel.exists() {
            continue;
        }
        if let Some(j) = biblioteca.jogos.iter_mut().find(|j| v.executavel.starts_with(&j.pasta)) {
            if j.executavel.is_none() {
                j.executavel = Some(v.executavel.clone());
            }
            j.ultima_vez = j.ultima_vez.max(v.ultima_vez);
            continue;
        }
        let Some(pasta) = v.executavel.parent().map(PathBuf::from) else { continue };
        biblioteca.jogos.push(JogoInstalado {
            nome: v.nome,
            origem: Origem::Detectado,
            pasta,
            executavel: Some(v.executavel),
            ultima_vez: v.ultima_vez,
            bytes: 0,
            appid: None,
        });
    }
}

/// **Trava de segurança da escrita em IFEO.** Pura de propósito: não pode depender de ter Steam na máquina dos
/// testes.
pub fn dentro_de_biblioteca(executavel: &Path, raizes: &[PathBuf]) -> bool {
    // Relativo ou com `..` é tentativa de escapar da pasta.
    if !executavel.is_absolute()
        || executavel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false;
    }

    raizes.iter().any(|raiz| executavel.starts_with(raiz))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programa_instalado_de_editora_de_jogo() {
        assert!(instalado_e_jogo("Grand Theft Auto V", "Rockstar Games"));
        assert!(instalado_e_jogo("FiveM", "Cfx.re"));
        assert!(instalado_e_jogo("Roblox Player", "Roblox Corporation"));
        assert!(instalado_e_jogo("VALORANT", "Riot Games, Inc"));
        assert!(!instalado_e_jogo("Rockstar Games Launcher", "Rockstar Games"));
        assert!(!instalado_e_jogo("Roblox Studio", "Roblox Corporation"));
        assert!(!instalado_e_jogo("Riot Vanguard", "Riot Games, Inc"));
        assert!(!instalado_e_jogo("Google Chrome", "Google LLC"));
    }

    #[test]
    fn executavel_vem_do_icone_quando_nao_e_instalador() {
        assert_eq!(
            executavel_do_icone(r#""C:\Program Files\Rockstar Games\Games\Grand Theft Auto V\GTA5.exe""#),
            Some(PathBuf::from(r"C:\Program Files\Rockstar Games\Games\Grand Theft Auto V\GTA5.exe"))
        );
        assert_eq!(
            executavel_do_icone(r"C:\Users\U\AppData\Local\FiveM\FiveM.exe,0"),
            Some(PathBuf::from(r"C:\Users\U\AppData\Local\FiveM\FiveM.exe"))
        );
        assert_eq!(executavel_do_icone(r"C:\R\RobloxPlayerInstaller.exe,0"), None);
        assert_eq!(executavel_do_icone(r"C:\x\jogo.ico"), None);
    }

    #[test]
    fn jogo_visto_so_regrava_quando_muda() {
        let mut l = Vec::new();
        let exe = Path::new(r"C:\Jogos\X\x.exe");
        assert!(anotar_visto(&mut l, "X", exe, 1_000));
        assert!(!anotar_visto(&mut l, "X", exe, 1_500), "menos de uma hora: não regrava");
        assert!(anotar_visto(&mut l, "X", exe, 1_000 + 3_600));
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].vezes, 2);
        assert!(anotar_visto(&mut l, "Y", Path::new(r"C:\Jogos\Y\y.exe"), 5_000));
        assert_eq!(l.len(), 2);
    }

    #[test]
    fn arquivo_de_vistos_ilegivel_e_erro_e_ausente_e_vazio() {
        let dir = std::env::temp_dir().join(format!("otimiza-vistos-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        assert_eq!(ler_vistos_de(&dir.join("nao-existe.json")), Ok(Vec::new()));
        let ruim = dir.join("ruim.json");
        std::fs::write(&ruim, "{ quebrado").unwrap();
        assert!(ler_vistos_de(&ruim).is_err());
    }

    const LIBRARYFOLDERS: &str = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"apps"
		{
			"271590"		"129049488815"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}
"#;

    const MANIFEST: &str = r#"
"AppState"
{
	"appid"		"271590"
	"name"		"Grand Theft Auto V Legacy"
	"StateFlags"		"4"
	"installdir"		"Grand Theft Auto V"
	"LastPlayed"		"1785616013"
	"SizeOnDisk"		"129049488815"
}
"#;

    #[test]
    fn le_as_bibliotecas_de_todos_os_discos() {
        let raizes = raizes_steam(LIBRARYFOLDERS);

        assert_eq!(raizes.len(), 2);
        assert_eq!(raizes[0], PathBuf::from(r"C:\Program Files (x86)\Steam"));
        assert_eq!(raizes[1], PathBuf::from(r"D:\SteamLibrary"));
    }

    #[test]
    fn le_o_manifest_com_nome_data_e_tamanho() {
        let raiz = PathBuf::from(r"C:\Program Files (x86)\Steam");
        let jogo = jogo_do_manifest(MANIFEST, &raiz).expect("manifest válido");

        assert_eq!(jogo.nome, "Grand Theft Auto V Legacy");
        assert_eq!(
            jogo.pasta,
            raiz.join("steamapps").join("common").join("Grand Theft Auto V")
        );
        assert_eq!(jogo.ultima_vez, 1785616013);
        assert_eq!(jogo.bytes, 129049488815);
    }

    #[test]
    fn manifest_sem_nome_ou_pasta_e_recusado() {
        assert!(jogo_do_manifest("\"AppState\"\n{\n}\n", Path::new("C:\\")).is_none());
        assert!(jogo_do_manifest("\"name\" \"\"\n\"installdir\" \"x\"", Path::new("C:\\")).is_none());
    }

    #[test]
    fn pacote_de_sistema_nao_e_jogo() {
        assert!(e_pacote_de_sistema("Steamworks Common Redistributables"));
        assert!(e_pacote_de_sistema("Proton 9.0"));
        assert!(!e_pacote_de_sistema("Grand Theft Auto V Legacy"));
    }

    #[test]
    fn a_trava_do_ifeo_so_aceita_caminho_dentro_da_biblioteca() {
        let raizes = vec![
            PathBuf::from(r"C:\Program Files (x86)\Steam"),
            PathBuf::from(r"D:\SteamLibrary"),
        ];

        assert!(dentro_de_biblioteca(
            Path::new(r"D:\SteamLibrary\steamapps\common\Jogo\jogo.exe"),
            &raizes
        ));

        assert!(!dentro_de_biblioteca(
            Path::new(r"C:\Windows\System32\sethc.exe"),
            &raizes
        ));
        assert!(!dentro_de_biblioteca(
            Path::new(r"C:\Users\Cliente\Downloads\coisa.exe"),
            &raizes
        ));

        assert!(!dentro_de_biblioteca(
            Path::new(r"D:\SteamLibrary\..\..\Windows\System32\cmd.exe"),
            &raizes
        ));
        assert!(!dentro_de_biblioteca(Path::new(r"jogo.exe"), &raizes));

        assert!(!dentro_de_biblioteca(
            Path::new(r"D:\SteamLibrary\steamapps\common\Jogo\jogo.exe"),
            &[]
        ));
    }

    #[test]
    fn linha_com_tres_campos_nao_vira_par() {
        let pares = pares_vdf("\"a\" \"b\" \"c\"\n\"d\" \"e\"");

        assert_eq!(pares.len(), 1);
        assert_eq!(pares[0], ("d".to_string(), "e".to_string()));
    }

    #[test]
    fn varre_esta_maquina() {
        let b = varrer();

        println!("  raízes: {:?}", b.raizes);
        for j in &b.jogos {
            println!(
                "  {:?} · {} · {:.1} GB · {}",
                j.origem,
                j.nome,
                j.bytes as f64 / 1_073_741_824.0,
                j.pasta.display()
            );
        }
        for l in &b.lacunas {
            println!("  não deu para ler: {}", l);
        }

        // Uma máquina pode não ter loja; o que se exige é que o relatado exista no disco.
        for j in &b.jogos {
            assert!(j.pasta.exists(), "{} relatado e não existe", j.pasta.display());
        }
    }
}
