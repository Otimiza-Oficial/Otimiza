// Programas de inicialização
//
// Serve para cruzar com o monitor de processos: saber que um programa consome CPU
// é útil, mas saber que ele *volta sozinho a cada boot* é o que explica por que o
// PC do cliente vive lento.

use super::registry;
use crate::modules::changelog::{ChangeRecord, PreviousValue};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Onde o Windows guarda quais entradas de inicialização estão desligadas.
/// É a mesma chave que o Gerenciador de Tarefas usa quando você clica em
/// "Desabilitar" — não removemos a entrada do cliente, apenas a desligamos.
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// Primeiro byte do valor em `StartupApproved`: 0x02 habilitado, 0x03 desabilitado.
const ENABLED_MARK: u8 = 0x02;
const DISABLED_MARK: u8 = 0x03;

/// Um programa que sobe com o Windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupEntry {
    /// Nome do valor no registro. É a chave para ligar/desligar.
    pub name: String,
    pub command: String,
    /// Nome do executável em minúsculas, usado para cruzar com os processos.
    pub executable: String,
    /// "HKCU" (só este usuário) ou "HKLM" (todos os usuários, exige administrador).
    pub hive: String,
    pub enabled: bool,
}

/// Monta o valor de 12 bytes que o Windows espera em `StartupApproved`.
///
/// O Gerenciador de Tarefas grava a data/hora do desligamento nos bytes 4 a 11.
/// Zeros funcionam igual — o Windows só lê o primeiro byte para decidir — e
/// evitam inventar um horário falso no registro do cliente.
fn approval_bytes(enabled: bool) -> Vec<u8> {
    let mut bytes = vec![0u8; 12];
    bytes[0] = if enabled { ENABLED_MARK } else { DISABLED_MARK };
    bytes
}

/// Lê se uma entrada está habilitada.
///
/// Ausência de valor em `StartupApproved` significa habilitada: o Windows só
/// grava ali quando alguém desliga alguma coisa.
pub fn is_entry_enabled(hive: &str, name: &str) -> bool {
    match registry::read(hive, APPROVED_KEY, name) {
        Ok(PreviousValue::Binary(bytes)) => bytes.first() != Some(&DISABLED_MARK),
        _ => true,
    }
}

/// Todos os programas de inicialização das chaves `Run`.
pub fn entries() -> Vec<StartupEntry> {
    let mut entries = Vec::new();

    for hive in ["HKCU", "HKLM"] {
        for name in registry::value_names(hive, RUN_KEY) {
            let Some(command) = registry::read_text(hive, RUN_KEY, &name) else {
                continue;
            };

            entries.push(StartupEntry {
                executable: executable_from_command(&command).unwrap_or_default(),
                enabled: is_entry_enabled(hive, &name),
                name,
                command,
                hive: hive.to_string(),
            });
        }
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries
}

/// Liga ou desliga um programa de inicialização.
///
/// Devolve o registro da mudança para o histórico, com o valor anterior — assim
/// "Desfazer tudo" também devolve a inicialização ao estado original.
pub fn set_enabled(hive: &str, name: &str, enabled: bool) -> Result<ChangeRecord, String> {
    let previous = registry::set_binary(hive, APPROVED_KEY, name, &approval_bytes(enabled))?;

    Ok(ChangeRecord::RegistryValue {
        hive: hive.to_string(),
        path: APPROVED_KEY.to_string(),
        name: name.to_string(),
        previous,
    })
}

/// Nomes de executável que sobem com o Windows, em minúsculas.
///
/// Guardamos o nome do arquivo, não a linha de comando inteira: é o nome que dá
/// para casar com o processo em execução.
pub fn startup_executables() -> HashSet<String> {
    let mut executables = HashSet::new();

    for hive in ["HKCU", "HKLM"] {
        for command in registry::value_names(hive, RUN_KEY)
            .into_iter()
            .filter_map(|name| registry::read_text(hive, RUN_KEY, &name))
        {
            if let Some(executable) = executable_from_command(&command) {
                executables.insert(executable);
            }
        }
    }

    executables
}

/// Extrai o nome do executável de uma linha de comando do registro.
///
/// As entradas vêm em formatos variados: com aspas, com argumentos, com caminho
/// completo ou sem. Pegar o trecho errado faria o cruzamento com os processos
/// falhar silenciosamente — o pior tipo de bug, porque a tela continua bonita.
pub fn executable_from_command(command: &str) -> Option<String> {
    let trimmed = command.trim();

    // Discord, Slack, Teams e todo aplicativo empacotado com Squirrel registram
    // um lançador (`Update.exe --processStart Discord.exe`). O que aparece na
    // lista de processos é o alvo, não o lançador — casar pelo lançador daria
    // "não está na inicialização" para justamente os programas mais pesados.
    if let Some(target) = process_start_target(trimmed) {
        return Some(target);
    }

    // Com aspas, o caminho é o que está entre elas: "C:\App\app.exe" --minimized
    let path = if trimmed.starts_with('"') {
        trimmed[1..].split('"').next()?.to_string()
    } else {
        // Sem aspas, cortar no primeiro espaço quebra em caminhos como
        // `C:\Riot Games\...\RiotClientServices.exe`, e o resultado sai "riot".
        // Cortar no fim do ".exe" acerta esses casos.
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

/// Alvo de um lançador Squirrel, se houver.
fn process_start_target(command: &str) -> Option<String> {
    let lowered = command.to_lowercase();
    let position = lowered
        .find("--processstart")
        .or_else(|| lowered.find("--process-start"))?;

    // O alvo é o primeiro argumento depois da flag.
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
    fn resolves_squirrel_launcher_to_its_target() {
        // O Discord registra o lançador, mas quem consome CPU é o Discord.exe.
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
        // Instaladores desleixados gravam sem aspas. Cortar no primeiro espaço
        // devolveria "riot" e o programa jamais casaria com o processo real.
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
        // O Windows espera exatamente 12 bytes nesse valor.
        assert_eq!(approval_bytes(true).len(), 12);
    }

    #[test]
    fn missing_approval_value_means_enabled() {
        // O Windows só grava em StartupApproved quando algo é desligado.
        // Tratar ausência como "desligado" mostraria a lista inteira errada.
        assert!(is_entry_enabled("HKCU", "EntradaQueNaoExiste_123"));
    }

    #[test]
    fn lists_real_startup_entries_of_this_machine() {
        let list = entries();

        for entry in &list {
            println!(
                "[{}] {:<28} {} · {}",
                entry.hive,
                entry.name,
                if entry.enabled { "ligado" } else { "desligado" },
                entry.executable
            );
        }

        // Cada entrada precisa ter nome e comando: uma linha vazia na tela do
        // cliente é pior que não mostrar a linha.
        for entry in &list {
            assert!(!entry.name.trim().is_empty());
            assert!(!entry.command.trim().is_empty());
        }
    }

    #[test]
    fn reads_this_machine_startup_list() {
        let executables = startup_executables();
        println!("{:?}", executables);
        // A lista pode ser vazia num Windows recém-instalado; o que não pode é
        // a leitura estourar.
    }
}
