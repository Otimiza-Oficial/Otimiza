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

/// A chave não existe, ou eu não consegui abri-la?
///
/// A diferença decide se o desfazer APAGA a chave do cliente. `AbsentKey` quer
/// dizer "não existia antes de eu mexer", e a reversão trata isso ao pé da
/// letra: apaga o valor e, se a chave ficar vazia, apaga a chave.
///
/// Enquanto QUALQUER erro de abertura virava `AbsentKey`, uma chave que EXISTIA
/// e não pôde ser lida — ACL restritiva, elevação perdida no meio do caminho —
/// era gravada no histórico como "não existia". O desfazer então apagava uma
/// chave preexistente, em nome de restaurar o estado anterior. Perder o que o
/// cliente tinha é pior que não conseguir mudar.
///
/// Função pura de propósito: reproduzir o caso real exigiria uma chave com DACL
/// restritiva, que não se monta numa esteira. Aqui a decisão fica testável
/// sozinha, e o resto do caminho continua sendo o mesmo.
pub fn chave_inexistente(erro: std::io::ErrorKind) -> bool {
    matches!(erro, std::io::ErrorKind::NotFound)
}

/// Lê o valor atual, seja ele DWORD, texto ou inexistente.
pub fn read(hive: &str, path: &str, name: &str) -> Result<PreviousValue, String> {
    let key = match root(hive)?.open_subkey_with_flags(path, KEY_READ) {
        Ok(key) => key,
        // Chave inexistente é um estado válido, e é distinto de "chave existe sem
        // o valor": só no primeiro caso a reversão precisa apagar a chave também.
        Err(e) if chave_inexistente(e.kind()) => return Ok(PreviousValue::AbsentKey),
        // Qualquer outro motivo é desconhecimento, e desconhecimento não pode
        // virar permissão para apagar.
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
    .map_err(|e| {
        explicar_falha_de_escrita(
            &format!("{}\\{}\\{}", hive, path, name),
            &e.to_string(),
            is_elevated(),
        )
    })?;

    Ok(previous)
}

/// Escreve um DWORD, criando a chave se necessário, e devolve o valor anterior.
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

/// Traduz a falha de escrita no registro para uma frase que o cliente entende.
///
/// "os error 5" não diz nada a ninguém. E o caso que motivou isto é pior que
/// feio: na máquina de desenvolvimento, com o programa JÁ elevado, a escrita em
/// `HKLM\SOFTWARE\Policies\Microsoft\Dsh` (a política dos Widgets) volta negada,
/// enquanto o PowerShell escreve na mesma chave sem reclamar. Quando somos
/// administrador e ainda assim o Windows nega, a causa não é permissão que o
/// usuário possa conceder — é alguma coisa na máquina barrando a escrita,
/// tipicamente antivírus ou proteção de política. Dizer "execute como
/// administrador" nesse caso manda o cliente fazer o que ele já fez.
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

/// Escreve um valor de texto, criando a chave se necessário, e devolve o valor anterior.
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

/// Apaga um valor.
///
/// ESTADO DESEJADO JÁ ATINGIDO NÃO É ERRO. Chave ausente ou valor ausente
/// querem dizer que não há o que apagar, e a reversão terminou o trabalho dela.
///
/// TUDO O MAIS É ERRO, E PRECISA SUBIR. Antes, esta função engolia duas falhas
/// diferentes — não conseguir abrir a chave para escrita, e não conseguir
/// apagar o valor — e devolvia `Ok(())` nos dois casos.
///
/// O estrago não era só a mensagem errada na tela. `ChangeLog::take` já tinha
/// consumido a entrada do histórico quando a reversão começa; então o valor
/// continuava no registro do cliente, a tela dizia que tinha desfeito, e o
/// produto perdia a capacidade de desfazer aquilo de novo — a única anotação de
/// como voltar já tinha sido gasta.
///
/// Falhar em voz alta preserva a entrada do histórico, e o cliente pode tentar
/// outra vez com elevação.
fn delete_value(hive: &str, path: &str, name: &str) -> Result<(), String> {
    let key = match root(hive)?.open_subkey_with_flags(path, KEY_WRITE) {
        Ok(key) => key,
        // A chave sumiu: não há valor para apagar, e é isso que se queria.
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
        // O valor já não estava lá.
        Err(e) if chave_inexistente(e.kind()) => Ok(()),
        Err(e) => Err(format!(
            "Não consegui apagar {}\\{}\\{} ao desfazer: {}. A mudança continua aplicada.",
            hive, path, name, e
        )),
    }
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
mod tests_1_8 {
    use super::*;
    use std::io::ErrorKind;

    /// A classificação que decide se o desfazer APAGA a chave do cliente.
    ///
    /// Só "não encontrado" pode virar `AbsentKey`. Qualquer outro motivo é
    /// desconhecimento, e desconhecimento não autoriza apagar: uma chave que
    /// existia e não pôde ser lida seria gravada como "não existia", e a
    /// reversão a removeria em nome de restaurar o estado anterior.
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

    /// Ler uma chave que realmente não existe continua sendo `AbsentKey`, e não
    /// erro. Esse caminho é o normal — a maioria das otimizações escreve em
    /// chave que o Windows ainda não criou.
    #[test]
    fn chave_que_nao_existe_continua_sendo_ausente() {
        let lido = read(
            "HKCU",
            r"Software\Otimiza\NaoExisteEstaChaveDeTeste\NemEsta",
            "QualquerValor",
        );

        assert_eq!(lido, Ok(PreviousValue::AbsentKey));
    }

    /// Hive desconhecida continua sendo erro, e não ausência.
    #[test]
    fn hive_desconhecida_e_erro_e_nao_ausencia() {
        assert!(read("HKXX", r"Software\Otimiza", "X").is_err());
    }

    /// Apagar um valor que não está lá é sucesso: o estado desejado já foi
    /// atingido. É o caso que o `delete_value` precisa continuar perdoando
    /// depois de deixar de perdoar todo o resto.
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
        // Medido na máquina do dono: elevado, e o Windows nega a escrita na
        // política dos Widgets mesmo assim, enquanto o PowerShell elevado grava
        // na mesma chave. Mandar "execute como administrador" aqui é mandar o
        // cliente repetir o que ele já fez, e deixá-lo achando que errou.
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

    #[test]
    fn chave_que_nao_existe_nao_tem_valores_e_nao_e_falha() {
        // A distinção que o C.1 trouxe corta nos dois sentidos: não conseguir
        // ler deixou de virar vazio, e chave inexistente NÃO pode ter virado
        // erro — senão toda máquina sem preferência de placa gravada, ou sem
        // programa na chave `Run` do usuário, passaria a mostrar lacuna.
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

/// Nomes dos valores de uma chave.
///
/// DEVOLVE `Result`, E A DOC ANTIGA EXPLICA POR QUÊ.
///
/// Até a 2.0 esta função devolvia `Vec<String>` e a doc dizia "lista vazia
/// quando a chave não existe". Ela mentia por omissão: o `Err(_) => Vec::new()`
/// devolvia vazio também para acesso negado, chave corrompida e hive
/// indisponível — e o chamador não tinha COMO saber a diferença.
///
/// O estrago aparecia longe daqui. `startup.rs` enumera as chaves `Run` com
/// isto; uma leitura que falhasse fazia a tela escrever "Nenhum programa nas
/// chaves de inicialização" sobre uma máquina que podia ter vinte.
///
/// A 1.8 aplicou essa disciplina ao `read` e ao `delete_value`, e parou ali.
/// Isto é a mesma regra descendo até o alicerce.
///
/// Chave que NÃO EXISTE continua sendo lista vazia, e isso não é a mesma
/// mentira: a chave `Run` de um usuário novo e a de preferência de placa de quem
/// nunca fixou nenhuma simplesmente não foram criadas. Não ter valores é a
/// resposta certa ali. O que deixou de virar vazio é não conseguir abrir.
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

/// Lê um valor de texto. Três respostas, não duas.
///
/// - `Ok(Some(_))` — leu, e é texto.
/// - `Ok(None)` — a chave ou o valor não existe, ou existe e é de outro tipo.
///   As duas são "não há texto aqui", que é uma resposta legítima.
/// - `Err(_)` — NÃO CONSEGUIU LER. Não é o mesmo que ausência.
///
/// O `Option` sozinho não dava conta: o `_ => None` jogava fora justamente a
/// distinção que o `read` de baixo tinha acabado de fazer na 1.8. Em
/// `devices.rs`, isso decidia se uma placa de rede era FÍSICA — o filtro que o
/// PROGRESS.md registra como "o que mais deu trabalho acertar", porque separa a
/// placa real dos WAN Miniports de VPN. Um `ComponentId` ilegível fazia a placa
/// de verdade do cliente ser descartada como virtual, e as otimizações de rede
/// dela sumiam da lista sem uma palavra.
pub fn read_text(hive: &str, path: &str, name: &str) -> Result<Option<String>, String> {
    match read(hive, path, name)? {
        PreviousValue::Text(value) => Ok(Some(value)),
        _ => Ok(None),
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

#[cfg(test)]
mod tests_2_0 {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    /// Nenhuma escrita PRESUME o estado anterior.
    ///
    /// POR QUE ESTA TRAVA EXISTE
    ///
    /// A 1.8 ensinou `registry::read` a distinguir "não existe" de "não
    /// consegui ler", e consertou o único chamador que na época convertia essa
    /// falha em ausência (`network.rs`, o DNS). A disciplina parou ali.
    ///
    /// Duas outras escritas continuaram presumindo, e as duas doem:
    ///
    ///   - `gpupref::definir` gravava `PreviousValue::Absent` diante de uma
    ///     leitura falha. O desfazer APAGARIA a escolha de placa que o cliente
    ///     já tinha. A 1.9 pôs um botão em cima dessa função.
    ///   - `gamemode` gravava `AbsentKey`, e ali o estrago era imediato: o
    ///     caminho de desativar RESTAURA `anterior` na hora, então a leitura
    ///     falha virava escrita errada sem esperar desfazer nenhum.
    ///
    /// Consertar os dois casos de hoje não impede o terceiro. Esta varredura
    /// impede — é o mesmo recurso da trava de prosa em `commands.rs`: procurar
    /// a FORMA do defeito, e não as ocorrências conhecidas.
    ///
    /// A regra é estreita de propósito: só reprova `registry::read(...)`
    /// seguido de `.unwrap_or`. Um `.unwrap_or_default()` sobre `read_text`
    /// não cai aqui, porque aquilo é leitura para MOSTRAR, não para desfazer —
    /// é outro problema, com outro conserto.
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

    /// O texto entre `registry::read(` e o fim da expressão, sem comentários.
    ///
    /// Junta as linhas porque a chamada costuma quebrar em duas — foi assim
    /// que os dois defeitos se escondiam: o `.unwrap_or` ficava na linha de
    /// baixo, longe do olho de quem lia a de cima.
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

            // Este arquivo fala de si mesmo nos comentários acima.
            if arquivo.file_name().and_then(|n| n.to_str()) == Some("registry.rs") {
                continue;
            }

            let plano = sem_comentarios(&conteudo);

            for trecho in plano.split("registry::read(").skip(1) {
                chamadas += 1;

                // O que vem logo depois do fecha-parênteses da chamada.
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
