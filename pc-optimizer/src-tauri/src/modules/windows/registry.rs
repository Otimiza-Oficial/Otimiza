// Acesso ao registro do Windows
//
// Toda escrita passa por `set_dword`/`set_string`, que devolvem o valor anterior
// para ser guardado no ChangeLog. Escrever sem capturar o valor anterior torna a
// mudança irreversível — o que este produto não faz.

use crate::modules::changelog::PreviousValue;
use winreg::enums::{RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::{RegKey, RegValue};

/// Resolve o nome textual da hive para a chave raiz correspondente.
fn root(hive: &str) -> Result<RegKey, String> {
    match hive.to_uppercase().as_str() {
        "HKLM" => Ok(RegKey::predef(HKEY_LOCAL_MACHINE)),
        "HKCU" => Ok(RegKey::predef(HKEY_CURRENT_USER)),
        other => Err(format!("Unsupported registry hive: {}", other)),
    }
}

/// Lê o valor atual, seja ele DWORD, texto ou inexistente.
pub fn read(hive: &str, path: &str, name: &str) -> Result<PreviousValue, String> {
    let key = match root(hive)?.open_subkey_with_flags(path, KEY_READ) {
        // Chave inexistente é um estado válido, e é distinto de "chave existe sem
        // o valor": só no primeiro caso a reversão precisa apagar a chave também.
        Err(_) => return Ok(PreviousValue::AbsentKey),
        Ok(key) => key,
    };

    if let Ok(value) = key.get_value::<u32, _>(name) {
        return Ok(PreviousValue::Dword(value));
    }

    if let Ok(value) = key.get_value::<String, _>(name) {
        return Ok(PreviousValue::Text(value));
    }

    // Último recurso: valor bruto. É como o Windows guarda o estado dos
    // programas de inicialização.
    if let Ok(raw) = key.get_raw_value(name) {
        return Ok(PreviousValue::Binary(raw.bytes));
    }

    Ok(PreviousValue::Absent)
}

/// Escreve um valor binário (REG_BINARY) e devolve o valor anterior.
pub fn set_binary(hive: &str, path: &str, name: &str, bytes: &[u8]) -> Result<PreviousValue, String> {
    let previous = read(hive, path, name)?;

    let (key, _) = root(hive)?
        .create_subkey(path)
        .map_err(|e| format!("Cannot open {}\\{} for writing: {}", hive, path, e))?;

    key.set_raw_value(
        name,
        &RegValue {
            bytes: bytes.to_vec(),
            vtype: RegType::REG_BINARY,
        },
    )
    .map_err(|e| format!("Cannot set {}\\{}\\{}: {}", hive, path, name, e))?;

    Ok(previous)
}

/// Escreve um DWORD, criando a chave se necessário, e devolve o valor anterior.
pub fn set_dword(hive: &str, path: &str, name: &str, value: u32) -> Result<PreviousValue, String> {
    let previous = read(hive, path, name)?;

    let (key, _) = root(hive)?
        .create_subkey(path)
        .map_err(|e| format!("Cannot open {}\\{} for writing: {}", hive, path, e))?;

    key.set_value(name, &value)
        .map_err(|e| format!("Cannot set {}\\{}\\{}: {}", hive, path, name, e))?;

    Ok(previous)
}

/// Escreve um valor de texto, criando a chave se necessário, e devolve o valor anterior.
pub fn set_string(hive: &str, path: &str, name: &str, value: &str) -> Result<PreviousValue, String> {
    let previous = read(hive, path, name)?;

    let (key, _) = root(hive)?
        .create_subkey(path)
        .map_err(|e| format!("Cannot open {}\\{} for writing: {}", hive, path, e))?;

    key.set_value(name, &value.to_string())
        .map_err(|e| format!("Cannot set {}\\{}\\{}: {}", hive, path, name, e))?;

    Ok(previous)
}

/// Restaura um valor ao estado registrado antes da otimização.
pub fn restore(hive: &str, path: &str, name: &str, previous: &PreviousValue) -> Result<(), String> {
    match previous {
        PreviousValue::Dword(value) => {
            set_dword(hive, path, name, *value)?;
        }
        PreviousValue::Text(value) => {
            set_string(hive, path, name, value)?;
        }
        PreviousValue::Binary(bytes) => {
            set_binary(hive, path, name, bytes)?;
        }
        PreviousValue::Absent => {
            delete_value(hive, path, name)?;
        }
        PreviousValue::AbsentKey => {
            delete_value(hive, path, name)?;

            // A chave foi criada por nós. Removê-la completa a reversão, mas só
            // se estiver vazia: outro programa pode ter escrito ali nesse meio-tempo.
            if is_empty_key(hive, path) {
                let _ = root(hive)?.delete_subkey(path);
            }
        }
    }

    Ok(())
}

/// Apaga um valor. Um valor já ausente não é erro — o estado desejado já foi atingido.
fn delete_value(hive: &str, path: &str, name: &str) -> Result<(), String> {
    if let Ok(key) = root(hive)?.open_subkey_with_flags(path, KEY_WRITE) {
        let _ = key.delete_value(name);
    }
    Ok(())
}

/// Uma chave sem valores e sem subchaves pode ser removida com segurança.
fn is_empty_key(hive: &str, path: &str) -> bool {
    match root(hive).and_then(|root| {
        root.open_subkey_with_flags(path, KEY_READ)
            .map_err(|e| e.to_string())
    }) {
        Ok(key) => key.enum_values().count() == 0 && key.enum_keys().count() == 0,
        Err(_) => false,
    }
}

/// Lista as subchaves de um caminho. Usado para enumerar interfaces de rede,
/// cujos GUIDs são diferentes em cada máquina.
pub fn subkeys(hive: &str, path: &str) -> Result<Vec<String>, String> {
    let key = root(hive)?
        .open_subkey_with_flags(path, KEY_READ)
        .map_err(|e| format!("Cannot open {}\\{}: {}", hive, path, e))?;

    Ok(key.enum_keys().filter_map(|k| k.ok()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confere a detecção de elevação contra uma fonte independente do nosso
    /// código: o nível de integridade do token, informado pelo próprio Windows.
    ///
    /// Os SIDs são iguais em qualquer idioma — S-1-16-12288 é "alto"
    /// (administrador) e S-1-16-16384 é "sistema". Este teste existe porque a
    /// implementação anterior errava silenciosamente, e um erro aqui trava o
    /// usuário num pedido de permissão que ele já tem.
    #[test]
    fn elevation_matches_the_token_integrity_level() {
        let saida = super::super::shell::run("whoami", &["/groups"])
            .expect("whoami falhou");

        let alto = saida.stdout.contains("S-1-16-12288");
        let sistema = saida.stdout.contains("S-1-16-16384");
        let esperado = alto || sistema;

        assert_eq!(
            is_elevated(),
            esperado,
            "detecção de elevação discorda do nível de integridade do token"
        );
    }
}

/// Nomes dos valores de uma chave. Lista vazia quando a chave não existe.
pub fn value_names(hive: &str, path: &str) -> Vec<String> {
    match root(hive).and_then(|root| {
        root.open_subkey_with_flags(path, KEY_READ)
            .map_err(|e| e.to_string())
    }) {
        Ok(key) => key.enum_values().filter_map(|entry| entry.ok().map(|(name, _)| name)).collect(),
        Err(_) => Vec::new(),
    }
}

/// Lê um valor de texto, ignorando valores de outro tipo.
pub fn read_text(hive: &str, path: &str, name: &str) -> Option<String> {
    match read(hive, path, name) {
        Ok(PreviousValue::Text(value)) => Some(value),
        _ => None,
    }
}

/// Verifica se uma chave existe.
pub fn key_exists(hive: &str, path: &str) -> bool {
    root(hive)
        .and_then(|root| {
            root.open_subkey_with_flags(path, KEY_READ)
                .map_err(|e| e.to_string())
        })
        .is_ok()
}

/// Verifica se o processo tem privilégios de administrador.
///
/// Pergunta ao próprio Windows, lendo o token do processo. A versão anterior
/// tentava abrir `HKLM\SOFTWARE` para escrita e concluía elevação a partir disso
/// — e estava errada: essa chave tem permissões restritas em parte das máquinas,
/// então falhava MESMO com administrador. O sintoma seria o pior possível: o app
/// pedindo elevação para quem já é administrador, num laço sem saída.
///
/// A lição vale além daqui: permissão de chave é um palpite sobre elevação; o
/// token é a resposta.
pub fn is_elevated() -> bool {
    use std::mem;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();

        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned = 0u32;

        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        );

        CloseHandle(token);

        ok != 0 && elevation.TokenIsElevated != 0
    }
}
