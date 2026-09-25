// Registro do Windows. Toda escrita devolve o valor anterior, para o ChangeLog: sem ele a mudança seria
// irreversível.

use crate::modules::changelog::PreviousValue;
use winreg::enums::{RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::{RegKey, RegValue};

fn root(hive: &str) -> Result<RegKey, String> {
    match hive.to_uppercase().as_str() {
        "HKLM" => Ok(RegKey::predef(HKEY_LOCAL_MACHINE)),
        "HKCU" => Ok(RegKey::predef(HKEY_CURRENT_USER)),
        other => Err(format!("Unsupported registry hive: {}", other)),
    }
}

/// `AbsentKey` faz o desfazer APAGAR a chave. Qualquer erro virava `AbsentKey`, e uma chave que existia e não se
/// leu (ACL restritiva) era apagada "restaurando o anterior". Pura: o caso real pede DACL restritiva, que não se
/// monta na esteira.
pub fn chave_inexistente(erro: std::io::ErrorKind) -> bool {
    matches!(erro, std::io::ErrorKind::NotFound)
}

pub fn read(hive: &str, path: &str, name: &str) -> Result<PreviousValue, String> {
    let key = match root(hive)?.open_subkey_with_flags(path, KEY_READ) {
        Ok(key) => key,
        // Distinto de "chave existe sem o valor": só aqui a reversão apaga a chave também.
        Err(e) if chave_inexistente(e.kind()) => return Ok(PreviousValue::AbsentKey),
        // Desconhecimento não pode virar permissão para apagar.
        Err(e) => {
            return Err(format!(
                "Não consegui ler {}\\{}: {}. Nada foi alterado.",
                hive, path, e
            ))
        }
    };

    if let Ok(value) = key.get_value::<u32, _>(name) {
        return Ok(PreviousValue::Dword(value));
    }

    if let Ok(value) = key.get_value::<String, _>(name) {
        return Ok(PreviousValue::Text(value));
    }

    // O estado dos programas de inicialização é guardado assim.
    if let Ok(raw) = key.get_raw_value(name) {
        return Ok(PreviousValue::Binary(raw.bytes));
    }

    Ok(PreviousValue::Absent)
}

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
    .map_err(|e| {
        explicar_falha_de_escrita(
            &format!("{}\\{}\\{}", hive, path, name),
            &e.to_string(),
            is_elevated(),
        )
    })?;

    Ok(previous)
}

pub fn set_dword(hive: &str, path: &str, name: &str, value: u32) -> Result<PreviousValue, String> {
    let previous = read(hive, path, name)?;

    let (key, _) = root(hive)?
        .create_subkey(path)
        .map_err(|e| format!("Cannot open {}\\{} for writing: {}", hive, path, e))?;

    key.set_value(name, &value)
        .map_err(|e| {
        explicar_falha_de_escrita(
            &format!("{}\\{}\\{}", hive, path, name),
            &e.to_string(),
            is_elevated(),
        )
    })?;

    Ok(previous)
}

/// Elevado e ainda negado (a política dos Widgets em `HKLM\SOFTWARE\Policies\Microsoft\Dsh`, que o PowerShell
/// grava): é algo barrando a escrita, e "execute como administrador" manda repetir o que já foi feito.
pub fn explicar_falha_de_escrita(caminho: &str, erro: &str, elevado: bool) -> String {
    let baixo = erro.to_lowercase();
    let negado = baixo.contains("os error 5")
        || baixo.contains("acesso negado")
        || baixo.contains("access is denied");

    if !negado {
        return format!("Não foi possível escrever em {}: {}", caminho, erro);
    }

    if !elevado {
        return format!(
            "O Windows negou a escrita em {}. Reabra o Otimiza como administrador.",
            caminho
        );
    }

    format!(
        "O Windows negou a escrita em {} mesmo com o Otimiza aberto como administrador. \
         Isso normalmente é antivírus ou uma proteção de política bloqueando a alteração — \
         não é falta de permissão sua. Nada foi alterado.",
        caminho
    )
}

pub fn set_string(hive: &str, path: &str, name: &str, value: &str) -> Result<PreviousValue, String> {
    let previous = read(hive, path, name)?;

    let (key, _) = root(hive)?
        .create_subkey(path)
        .map_err(|e| format!("Cannot open {}\\{} for writing: {}", hive, path, e))?;

    key.set_value(name, &value.to_string())
        .map_err(|e| {
        explicar_falha_de_escrita(
            &format!("{}\\{}\\{}", hive, path, name),
            &e.to_string(),
            is_elevated(),
        )
    })?;

    Ok(previous)
}

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

            // Só se estiver vazia: outro programa pode ter escrito ali.
            if is_empty_key(hive, path) {
                let _ = root(hive)?.delete_subkey(path);
            }
        }
    }

    Ok(())
}

/// Estado desejado já atingido não é erro; todo o resto sobe. Engolir a falha dizia "desfeito" com o valor
/// ainda lá, e `ChangeLog::take` já tinha gasto a única anotação de como voltar.
fn delete_value(hive: &str, path: &str, name: &str) -> Result<(), String> {
    let key = match root(hive)?.open_subkey_with_flags(path, KEY_WRITE) {
        Ok(key) => key,
        Err(e) if chave_inexistente(e.kind()) => return Ok(()),
        Err(e) => {
            return Err(format!(
                "Não consegui abrir {}\\{} para desfazer: {}. A mudança continua aplicada.",
                hive, path, e
            ))
        }
    };

    match key.delete_value(name) {
        Ok(()) => Ok(()),
        Err(e) if chave_inexistente(e.kind()) => Ok(()),
        Err(e) => Err(format!(
            "Não consegui apagar {}\\{}\\{} ao desfazer: {}. A mudança continua aplicada.",
            hive, path, name, e
        )),
    }
}

fn is_empty_key(hive: &str, path: &str) -> bool {
    match root(hive).and_then(|root| {
        root.open_subkey_with_flags(path, KEY_READ)
            .map_err(|e| e.to_string())
    }) {
        Ok(key) => key.enum_values().count() == 0 && key.enum_keys().count() == 0,
        Err(_) => false,
    }
}

/// Chave inexistente é lista vazia; só não conseguir abrir vira `Err`.
pub fn subkeys(hive: &str, path: &str) -> Result<Vec<String>, String> {
    let key = match root(hive)?.open_subkey_with_flags(path, KEY_READ) {
        Ok(key) => key,
        Err(e) if chave_inexistente(e.kind()) => return Ok(Vec::new()),
        Err(e) => return Err(format!("Não consegui ler {}\\{}: {}", hive, path, e)),
    };

    Ok(key.enum_keys().filter_map(|k| k.ok()).collect())
}

#[cfg(test)]
mod tests_1_8 {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn so_chave_nao_encontrada_pode_virar_ausente() {
        assert!(chave_inexistente(ErrorKind::NotFound));

        for negado in [
            ErrorKind::PermissionDenied,
            ErrorKind::Other,
            ErrorKind::InvalidData,
            ErrorKind::TimedOut,
        ] {
            assert!(
                !chave_inexistente(negado),
                "{:?} nao pode ser lido como chave inexistente: o desfazer apagaria                  uma chave que o cliente tinha",
                negado
            );
        }
    }

    /// O caminho normal: a maioria das otimizações escreve em chave que o Windows ainda não criou.
    #[test]
    fn chave_que_nao_existe_continua_sendo_ausente() {
        let lido = read(
            "HKCU",
            r"Software\Otimiza\NaoExisteEstaChaveDeTeste\NemEsta",
            "QualquerValor",
        );

        assert_eq!(lido, Ok(PreviousValue::AbsentKey));
    }

    #[test]
    fn hive_desconhecida_e_erro_e_nao_ausencia() {
        assert!(read("HKXX", r"Software\Otimiza", "X").is_err());
    }

    #[test]
    fn desfazer_valor_ja_ausente_e_sucesso() {
        let apagado = delete_value(
            "HKCU",
            r"Software\Otimiza\NaoExisteEstaChaveDeTeste",
            "QualquerValor",
        );

        assert_eq!(apagado, Ok(()));
    }
}

#[cfg(test)]
mod tests_1_6 {
    use super::*;

    #[test]
    fn negado_sem_elevacao_manda_reabrir_como_administrador() {
        let msg = explicar_falha_de_escrita("HKLM\\X\\Y", "Acesso negado. (os error 5)", false);
        assert!(msg.contains("administrador"));
    }

    #[test]
    fn negado_ja_elevado_nao_manda_fazer_o_que_ja_foi_feito() {
        let msg = explicar_falha_de_escrita(
            "HKLM\\SOFTWARE\\Policies\\Microsoft\\Dsh\\AllowNewsAndInterests",
            "Acesso negado. (os error 5)",
            true,
        );

        assert!(msg.contains("antivírus") || msg.contains("proteção"));
        assert!(msg.contains("Nada foi alterado"));
        assert!(
            !msg.contains("Reabra"),
            "não pode mandar reabrir como administrador quem já está: {}",
            msg
        );
    }

    #[test]
    fn falha_que_nao_e_permissao_nao_vira_conversa_de_permissao() {
        let msg = explicar_falha_de_escrita("HKLM\\X\\Y", "The system cannot find the file", true);
        assert!(!msg.contains("administrador"));
        assert!(!msg.contains("antivírus"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confere contra o nível de integridade do token (SIDs iguais em qualquer idioma): a implementação anterior
    /// errava calada e prendia o usuário num pedido de permissão que ele já tinha.
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

    #[test]
    fn chave_que_nao_existe_nao_tem_valores_e_nao_e_falha() {
        // Chave inexistente NÃO pode virar erro: toda máquina sem preferência de placa mostraria lacuna.
        const NAO_EXISTE: &str = r"Software\OtimizaChaveQueNaoExiste2026";

        assert_eq!(value_names("HKCU", NAO_EXISTE), Ok(Vec::new()));
        assert_eq!(read_text("HKCU", NAO_EXISTE, "Qualquer"), Ok(None));
    }

    #[test]
    fn le_os_nomes_de_uma_chave_que_existe() {
        let nomes = value_names("HKLM", r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
            .expect("a chave de versão do Windows é legível por qualquer usuário");

        assert!(nomes.iter().any(|nome| nome == "CurrentBuild"), "veio: {:?}", nomes);
    }
}

/// `Result`: o `Err(_) => Vec::new()` fazia a tela dizer "nenhum programa na inicialização" sobre uma leitura
/// negada. Chave que não existe continua lista vazia.
pub fn value_names(hive: &str, path: &str) -> Result<Vec<String>, String> {
    let key = match root(hive)?.open_subkey_with_flags(path, KEY_READ) {
        Ok(key) => key,
        Err(e) if chave_inexistente(e.kind()) => return Ok(Vec::new()),
        Err(e) => return Err(format!("Não consegui ler {}\\{}: {}", hive, path, e)),
    };

    Ok(key
        .enum_values()
        .filter_map(|entry| entry.ok().map(|(name, _)| name))
        .collect())
}

/// Três respostas: `Ok(Some)` texto, `Ok(None)` ausente ou outro tipo, `Err` não se leu. Com `Option`, um
/// `ComponentId` ilegível descartava a placa de rede física como virtual.
pub fn read_text(hive: &str, path: &str, name: &str) -> Result<Option<String>, String> {
    match read(hive, path, name)? {
        PreviousValue::Text(value) => Ok(Some(value)),
        _ => Ok(None),
    }
}

/// `None` é NÃO DEU PARA SABER: com a ACL negando `Services\<svc>`, quatro itens sumiam como "não se aplica".
pub fn key_exists(hive: &str, path: &str) -> Option<bool> {
    let raiz = root(hive).ok()?;

    match raiz.open_subkey_with_flags(path, KEY_READ) {
        Ok(_) => Some(true),
        Err(e) if chave_inexistente(e.kind()) => Some(false),
        Err(_) => None,
    }
}

/// Pelo token do processo: abrir `HKLM\SOFTWARE` para escrita falhava mesmo com administrador, num laço de
/// pedido de elevação sem saída.
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

#[cfg(test)]
mod tests_2_0 {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    /// Nenhuma escrita PRESUME o estado anterior: reprova `registry::read(...)` seguido de `.unwrap_or` (gravava
    /// `Absent` diante de leitura falha, e o desfazer apagaria a escolha do cliente). Procura a FORMA do defeito.
    /// `.unwrap_or_default()` sobre `read_text` fica fora: é leitura para mostrar.
    fn arquivos_rust(dir: &Path, achados: &mut Vec<PathBuf>) {
        let Ok(entradas) = std::fs::read_dir(dir) else {
            return;
        };

        for entrada in entradas.flatten() {
            let caminho = entrada.path();

            if caminho.is_dir() {
                arquivos_rust(&caminho, achados);
            } else if caminho.extension().and_then(|e| e.to_str()) == Some("rs") {
                achados.push(caminho);
            }
        }
    }

    /// Junta as linhas: o `.unwrap_or` se escondia na linha de baixo.
    fn sem_comentarios(conteudo: &str) -> String {
        conteudo
            .lines()
            .map(|l| match l.find("//") {
                Some(i) => &l[..i],
                None => l,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn nenhuma_escrita_presume_o_estado_anterior() {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

        let mut arquivos = Vec::new();
        arquivos_rust(&raiz, &mut arquivos);

        assert!(
            arquivos.len() > 40,
            "a varredura achou só {} arquivos — o caminho provavelmente está errado",
            arquivos.len()
        );

        let mut culpados = BTreeSet::new();
        let mut chamadas = 0usize;

        for arquivo in &arquivos {
            let Ok(conteudo) = std::fs::read_to_string(arquivo) else {
                continue;
            };

            if arquivo.file_name().and_then(|n| n.to_str()) == Some("registry.rs") {
                continue;
            }

            let plano = sem_comentarios(&conteudo);

            for trecho in plano.split("registry::read(").skip(1) {
                chamadas += 1;

                let Some(fim) = trecho.find(')') else { continue };
                let depois = trecho[fim..].trim_start_matches(')').trim_start();

                if depois.starts_with(".unwrap_or") {
                    culpados.insert(
                        arquivo
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("?")
                            .to_string(),
                    );
                }
            }
        }

        assert!(
            chamadas > 3,
            "a varredura achou só {} chamadas de registry::read — a expressão \
             provavelmente parou de casar",
            chamadas
        );

        assert!(
            culpados.is_empty(),
            "estes arquivos presumem o estado anterior em vez de ler: {:?}.\n\
             Uma leitura que falhou não é 'não existia'. Gravar isso no histórico \
             faz o desfazer APAGAR o que o cliente tinha, em vez de devolver.\n\
             Use `registry::read(...)?` e deixe a falha subir — nada foi escrito ainda.",
            culpados
        );
    }
}
