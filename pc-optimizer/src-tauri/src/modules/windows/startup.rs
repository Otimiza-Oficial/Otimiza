use super::registry;
use crate::modules::changelog::{ChangeRecord, PreviousValue};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// A chave que o Gerenciador de Tarefas usa ao clicar em "Desabilitar": a entrada do cliente não é removida.
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

const ENABLED_MARK: u8 = 0x02;
const DISABLED_MARK: u8 = 0x03;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupEntry {
    pub name: String,
    pub command: String,
    pub executable: String,
    pub hive: String,
    pub enabled: bool,
    pub classe: Classe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classe {
    Essencial,
    Util,
    Opcional,
    Desconhecido,
}

const ESSENCIAIS: &[&str] = &[
    "securityhealthsystray", "msmpeng", "rtkaud", "rtkngui", "ravcpl", "realtek", "waves", "nahimic",
    "igfx", "nvcplui", "nvidia", "radeonsoftware", "amdrsserv", "synaptics", "syntp", "etdctrl", "elan",
    "bthudtask", "bluetooth", "sgrmbroker",
];
const UTEIS: &[&str] = &[
    "onedrive", "googledrivefs", "dropbox", "megasync", "icloud", "lghub", "logioptions", "razer",
    "icue", "steelseries", "hyperx", "corsair", "armoury", "msi center", "dragon center",
];
const OPCIONAIS: &[&str] = &[
    "steam", "epicgameslauncher", "riotclient", "battle.net", "eadesktop", "origin", "ubisoftconnect",
    "upc", "galaxyclient", "rockstar", "spotify", "discord", "teams", "skype", "zoom", "whatsapp",
    "telegram", "msedge", "chrome", "opera", "brave", "firefox", "utorrent", "qbittorrent", "adobe",
    "ccleaner", "cortana", "yourphone", "phonelink", "roblox",
];

pub fn classificar(executavel: &str, comando: &str) -> Classe {
    let alvo = format!("{} {}", executavel, comando).to_lowercase();
    let tem = |lista: &[&str]| lista.iter().any(|n| alvo.contains(n));
    if tem(ESSENCIAIS) {
        Classe::Essencial
    } else if tem(UTEIS) {
        Classe::Util
    } else if tem(OPCIONAIS) {
        Classe::Opcional
    } else {
        Classe::Desconhecido
    }
}

/// O Gerenciador de Tarefas grava a data nos bytes 4 a 11; zeros funcionam igual (o Windows só lê o primeiro
/// byte) e não inventam um horário no registro do cliente.
fn approval_bytes(enabled: bool) -> Vec<u8> {
    let mut bytes = vec![0u8; 12];
    bytes[0] = if enabled { ENABLED_MARK } else { DISABLED_MARK };
    bytes
}

/// Ausência em `StartupApproved` significa habilitada: o Windows só grava ali quando alguém desliga algo.
pub fn is_entry_enabled(hive: &str, name: &str) -> bool {
    match registry::read(hive, APPROVED_KEY, name) {
        Ok(PreviousValue::Binary(bytes)) => bytes.first() != Some(&DISABLED_MARK),
        _ => true,
    }
}

/// `Err` quando uma chave não se lê: lista vazia diria "nenhum programa" sobre uma máquina que pode ter vinte.
pub fn entries() -> Result<Vec<StartupEntry>, String> {
    let mut entries = Vec::new();

    for hive in ["HKCU", "HKLM"] {
        for name in registry::value_names(hive, RUN_KEY)? {
            let Some(command) = registry::read_text(hive, RUN_KEY, &name)? else {
                continue;
            };

            let executable = executable_from_command(&command).unwrap_or_default();
            entries.push(StartupEntry {
                classe: classificar(&executable, &command),
                executable,
                enabled: is_entry_enabled(hive, &name),
                name,
                command,
                hive: hive.to_string(),
            });
        }
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

pub fn set_enabled(hive: &str, name: &str, enabled: bool) -> Result<ChangeRecord, String> {
    let previous = registry::set_binary(hive, APPROVED_KEY, name, &approval_bytes(enabled))?;

    Ok(ChangeRecord::RegistryValue {
        hive: hive.to_string(),
        path: APPROVED_KEY.to_string(),
        name: name.to_string(),
        previous,
    })
}

/// Pelo nome do arquivo, que casa com o processo. Chave ilegível fica de fora do conjunto; a lista em si
/// (`entries`) devolve o erro.
pub fn startup_executables() -> HashSet<String> {
    let mut executables = HashSet::new();

    for hive in ["HKCU", "HKLM"] {
        let Ok(nomes) = registry::value_names(hive, RUN_KEY) else {
            continue;
        };

        for name in nomes {
            if let Ok(Some(command)) = registry::read_text(hive, RUN_KEY, &name) {
                if let Some(executable) = executable_from_command(&command) {
                    executables.insert(executable);
                }
            }
        }
    }

    executables
}

/// Pegar o trecho errado faria o cruzamento com os processos falhar em silêncio.
pub fn executable_from_command(command: &str) -> Option<String> {
    let trimmed = command.trim();

    // Discord, Slack e Teams (Squirrel) registram `Update.exe --processStart Discord.exe`: o processo é o alvo, não
    // o lançador.
    if let Some(target) = process_start_target(trimmed) {
        return Some(target);
    }

    let path = if trimmed.starts_with('"') {
        trimmed[1..].split('"').next()?.to_string()
    } else {
        // Sem aspas, cortar no primeiro espaço quebra `C:\Riot Games\...` em "riot": corta no fim do ".exe".
        let lowered = trimmed.to_lowercase();

        match lowered.find(".exe") {
            Some(position) => trimmed[..position + 4].to_string(),
            None => trimmed.split_whitespace().next()?.to_string(),
        }
    };

    let file = path
        .rsplit(['\\', '/'])
        .next()?
        .trim()
        .to_lowercase();

    if file.is_empty() {
        None
    } else {
        Some(file)
    }
}

fn process_start_target(command: &str) -> Option<String> {
    let lowered = command.to_lowercase();
    let position = lowered
        .find("--processstart")
        .or_else(|| lowered.find("--process-start"))?;

    lowered[position..]
        .split_whitespace()
        .nth(1)
        .map(|target| target.trim_matches('"').to_string())
        .filter(|target| target.ends_with(".exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classe_pelo_nome() {
        assert_eq!(classificar("securityhealthsystray.exe", ""), Classe::Essencial);
        assert_eq!(classificar("onedrive.exe", ""), Classe::Util);
        assert_eq!(classificar("steam.exe", r#""C:\Steam\steam.exe" -silent"#), Classe::Opcional);
        assert_eq!(classificar("discord.exe", ""), Classe::Opcional);
        assert_eq!(classificar("rundll32.exe", "rundll32 algo.dll"), Classe::Desconhecido);
    }

    #[test]
    #[ignore = "lê a inicialização desta máquina"]
    fn classes_desta_maquina() {
        for e in entries().unwrap() {
            println!("{:?} {} ({})", e.classe, e.name, e.executable);
        }
    }

    #[test]
    fn resolves_squirrel_launcher_to_its_target() {
        assert_eq!(
            executable_from_command("\"C:\\Users\\x\\AppData\\Local\\Discord\\Update.exe\" --processStart Discord.exe").as_deref(),
            Some("discord.exe")
        );
    }

    #[test]
    fn reads_quoted_path_with_arguments() {
        assert_eq!(
            executable_from_command("\"C:\\Program Files\\App\\app.exe\" --minimized").as_deref(),
            Some("app.exe")
        );
    }

    #[test]
    fn reads_unquoted_path() {
        assert_eq!(
            executable_from_command("C:\\Windows\\System32\\rundll32.exe").as_deref(),
            Some("rundll32.exe")
        );
    }

    #[test]
    fn reads_unquoted_path_containing_spaces() {
        assert_eq!(
            executable_from_command("C:\\Riot Games\\Riot Client\\RiotClientServices.exe --launch")
                .as_deref(),
            Some("riotclientservices.exe")
        );
        assert_eq!(
            executable_from_command("C:\\Program Files\\Notion\\Notion.exe").as_deref(),
            Some("notion.exe")
        );
    }

    #[test]
    fn reads_bare_executable_name() {
        assert_eq!(
            executable_from_command("steam.exe -silent").as_deref(),
            Some("steam.exe")
        );
    }

    #[test]
    fn ignores_empty_command() {
        assert_eq!(executable_from_command("   "), None);
        assert_eq!(executable_from_command(""), None);
    }

    #[test]
    fn approval_bytes_use_the_marks_windows_expects() {
        assert_eq!(approval_bytes(true)[0], ENABLED_MARK);
        assert_eq!(approval_bytes(false)[0], DISABLED_MARK);
        assert_eq!(approval_bytes(true).len(), 12);
    }

    #[test]
    fn missing_approval_value_means_enabled() {
        assert!(is_entry_enabled("HKCU", "EntradaQueNaoExiste_123"));
    }

    #[test]
    fn lists_real_startup_entries_of_this_machine() {
        let list = entries().expect("as chaves Run desta máquina precisam ser legíveis");

        for entry in &list {
            println!(
                "[{}] {:<28} {} · {}",
                entry.hive,
                entry.name,
                if entry.enabled { "ligado" } else { "desligado" },
                entry.executable
            );
        }

        for entry in &list {
            assert!(!entry.name.trim().is_empty());
            assert!(!entry.command.trim().is_empty());
        }
    }

    #[test]
    fn reads_this_machine_startup_list() {
        let executables = startup_executables();
        println!("{:?}", executables);
    }
}
