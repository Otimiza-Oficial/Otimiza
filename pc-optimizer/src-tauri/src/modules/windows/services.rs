// Controle de serviços do Windows
//
// Desativar serviço é a otimização mais perigosa do produto: errar aqui quebra
// o PC do cliente. Por isso o tipo de inicialização anterior é sempre lido antes
// de qualquer alteração.
//
// A LEITURA é feita no registro, não pelo `sc qc`. Num Windows em português o
// `sc qc` imprime "TIPO_DE_INÍCIO" em vez de "START_TYPE", e ainda usa a página
// de código OEM — qualquer parsing de texto quebraria justamente nas máquinas do
// público-alvo. O valor no registro é o mesmo número em todos os idiomas.
//
// A ESCRITA usa `sc config`, que avisa o gerenciador de serviços na hora, em vez
// de só valer no próximo boot.

use super::registry;
use crate::modules::changelog::PreviousValue;

const SERVICES_KEY: &str = r"SYSTEM\CurrentControlSet\Services";

/// Traduz o valor `Start` do registro para a palavra-chave aceita pelo `sc config`.
///
/// 0 = boot, 1 = system (drivers de núcleo, nunca reconfigurados),
/// 2 = automático, 3 = manual, 4 = desativado.
pub fn start_type_keyword(code: u32, delayed: bool) -> Option<&'static str> {
    match code {
        2 if delayed => Some("delayed-auto"),
        2 => Some("auto"),
        3 => Some("demand"),
        4 => Some("disabled"),
        _ => None,
    }
}

fn service_key(service: &str) -> String {
    format!("{}\\{}", SERVICES_KEY, service)
}

/// O ESTADO NUMÉRICO na saída do `sc query`, sem procurar o rótulo.
///
/// O CÓDIGO ANTIGO PROCURAVA `"STATE"` OU `"ESTADO"`, e isso é o defeito
/// clássico de parsear texto traduzido: num Windows em francês o rótulo é
/// `ÉTAT`, em alemão `STATUS`, em italiano `STATO`. Nenhum bate, a busca falha,
/// e o resultado antigo era `false` — "o serviço não está rodando".
///
/// Conferido nesta máquina, em português:
///
/// ```text
///         TIPO               : 20  WIN32_SHARE_PROCESS
///         ESTADO              : 1  STOPPED
/// ```
///
/// Duas coisas NÃO são traduzidas e são usadas aqui: o número, e a constante
/// em inglês ao lado dele. A constante identifica a linha certa mesmo que o
/// `sc` mude a ordem das linhas um dia.
pub fn estado_da_saida_do_sc(stdout: &str) -> Option<u32> {
    // Os sete estados que o Gerenciador de Serviços do Windows define. Aparecem
    // em inglês em qualquer idioma do sistema.
    const ESTADOS: &[&str] = &[
        "STOPPED",
        "START_PENDING",
        "STOP_PENDING",
        "RUNNING",
        "CONTINUE_PENDING",
        "PAUSE_PENDING",
        "PAUSED",
    ];

    for linha in stdout.lines() {
        let Some((_, valor)) = linha.split_once(':') else {
            continue;
        };

        let mut partes = valor.split_whitespace();

        let Some(numero) = partes.next().and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };

        // A constante ao lado é o que separa a linha do estado da linha do
        // tipo, que também começa com número.
        if partes.next().is_some_and(|c| ESTADOS.contains(&c)) {
            return Some(numero);
        }
    }

    None
}

/// `None` é NÃO CONSEGUI SABER, e não "parado".
///
/// A diferença é perigosa aqui, e não só imprecisa: esta função decide se o
/// Windows Update precisa ser parado ANTES de o produto apagar o cache de
/// atualização. Com `false` falso — `sc` bloqueado, sem elevação, ou Windows em
/// outro idioma — o Otimiza apagava `SoftwareDistribution\Download` COM A
/// ATUALIZAÇÃO EM ANDAMENTO, que é exatamente o que os comentários de
/// `cleanup.rs` dizem querer evitar.
pub fn is_running(service: &str) -> Option<bool> {
    let saida = super::shell::run("sc", &["query", service]).ok()?;

    if !saida.success {
        return None;
    }

    estado_da_saida_do_sc(&saida.stdout).map(|estado| estado == 4)
}

/// Inicia um serviço. Um serviço já em execução não é tratado como erro.
pub fn start(service: &str) -> Result<(), String> {
    let output = super::shell::run("sc", &["start", service])?;

    // 1056 = já está em execução.
    if output.success || output.stdout.contains("1056") || output.stderr.contains("1056") {
        Ok(())
    } else {
        Err(format!(
            "Não foi possível iniciar o serviço `{}`: {}",
            service,
            output.stdout.trim()
        ))
    }
}

/// Verifica se um serviço existe nesta instalação do Windows.
///
/// `None` é "não deu para ler", e quem chama precisa tratar separado: dizer
/// "este Windows não tem o serviço" sobre uma chave que a ACL negou faz a
/// otimização sumir da lista com a frase errada.
pub fn exists(service: &str) -> Option<bool> {
    registry::key_exists("HKLM", &service_key(service))
}

/// Lê o tipo de inicialização atual de um serviço.
pub fn query_start_type(service: &str) -> Result<String, String> {
    let path = service_key(service);

    let code = match registry::read("HKLM", &path, "Start")? {
        PreviousValue::Dword(code) => code,
        _ => return Err(format!("Service `{}` has no readable start type", service)),
    };

    let delayed = matches!(
        registry::read("HKLM", &path, "DelayedAutostart")?,
        PreviousValue::Dword(1)
    );

    start_type_keyword(code, delayed).map(|s| s.to_string()).ok_or_else(|| {
        format!(
            "Service `{}` is a kernel driver (start type {}) and must not be modified",
            service, code
        )
    })
}

/// Define o tipo de inicialização de um serviço.
/// `start_type` deve ser auto, delayed-auto, demand ou disabled.
pub fn set_start_type(service: &str, start_type: &str) -> Result<(), String> {
    // O `sc config` exige o formato `start= valor`, com o espaço depois do sinal.
    super::shell::run_checked("sc", &["config", service, "start=", start_type])?;
    Ok(())
}

/// Para um serviço em execução. Um serviço já parado não é tratado como erro.
pub fn stop(service: &str) -> Result<(), String> {
    let output = super::shell::run("sc", &["stop", service])?;

    // 1062 = serviço não iniciado. O código numérico aparece em qualquer idioma.
    if output.success || output.stdout.contains("1062") || output.stderr.contains("1062") {
        Ok(())
    } else {
        Err(format!(
            "Could not stop service `{}`: {}",
            service,
            output.stdout.trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saída REAL desta máquina, em português, com os acentos já estragados
    /// pelo código de página do console — que é como ela chega ao produto.
    const SAIDA_EM_PORTUGUES: &str = "NOME_DO_SERVI\u{fffd}O: wuauserv \n\
        \x20       TIPO               : 20  WIN32_SHARE_PROCESS  \n\
        \x20       ESTADO              : 1  STOPPED \n\
        \x20       C\u{fffd}DIGO_DE_SA\u{fffd}DA_DO_WIN32    : 1077  (0x435)\n\
        \x20       PONTO_DE_VERIFICA\u{fffd}\u{fffd}O         : 0x0\n";

    #[test]
    fn o_estado_e_lido_sem_procurar_o_rotulo_traduzido() {
        // O código antigo procurava "STATE" ou "ESTADO". Num Windows em francês
        // o rótulo é ÉTAT, em alemão STATUS, em italiano STATO — nenhum batia, e
        // o resultado era "serviço parado" sobre um serviço rodando.
        assert_eq!(estado_da_saida_do_sc(SAIDA_EM_PORTUGUES), Some(1));

        let em_frances = "        \u{c9}TAT               : 4  RUNNING \n";
        assert_eq!(estado_da_saida_do_sc(em_frances), Some(4));

        let em_alemao = "        STATUS             : 4  RUNNING \n";
        assert_eq!(estado_da_saida_do_sc(em_alemao), Some(4));
    }

    #[test]
    fn a_linha_do_tipo_nao_e_confundida_com_a_do_estado() {
        // As duas começam com número depois dos dois-pontos. O que separa é a
        // constante em inglês ao lado — `WIN32_SHARE_PROCESS` não é um estado.
        let so_o_tipo = "        TIPO               : 20  WIN32_SHARE_PROCESS  \n";
        assert_eq!(estado_da_saida_do_sc(so_o_tipo), None);
    }

    #[test]
    fn saida_que_nao_da_para_entender_vira_nao_sei() {
        // E `None` NÃO pode virar "parado": é ele que decide se o Windows
        // Update é parado antes de o produto apagar o cache de atualização.
        assert_eq!(estado_da_saida_do_sc(""), None);
        assert_eq!(estado_da_saida_do_sc("acesso negado"), None);
        assert_eq!(
            estado_da_saida_do_sc("[SC] EnumQueryServicesStatus:OpenService FALHOU 1060"),
            None
        );
    }

    #[test]
    fn todos_os_sete_estados_sao_reconhecidos() {
        for (numero, constante) in [
            (1, "STOPPED"),
            (2, "START_PENDING"),
            (3, "STOP_PENDING"),
            (4, "RUNNING"),
            (5, "CONTINUE_PENDING"),
            (6, "PAUSE_PENDING"),
            (7, "PAUSED"),
        ] {
            let linha = format!("        ESTADO : {}  {} \n", numero, constante);
            assert_eq!(
                estado_da_saida_do_sc(&linha),
                Some(numero),
                "estado {} não foi reconhecido",
                constante
            );
        }
    }

    #[test]
    fn maps_registry_start_codes_to_sc_keywords() {
        assert_eq!(start_type_keyword(2, false), Some("auto"));
        assert_eq!(start_type_keyword(2, true), Some("delayed-auto"));
        assert_eq!(start_type_keyword(3, false), Some("demand"));
        assert_eq!(start_type_keyword(4, false), Some("disabled"));
    }

    #[test]
    fn refuses_kernel_driver_start_codes() {
        // 0 (boot) e 1 (system) são drivers de núcleo: reconfigurá-los pode
        // impedir o Windows de iniciar.
        assert_eq!(start_type_keyword(0, false), None);
        assert_eq!(start_type_keyword(1, false), None);
    }

    #[test]
    fn reads_real_service_start_type_from_registry() {
        // RpcSs existe em toda instalação do Windows e é automático.
        // Este teste falharia se a leitura dependesse do idioma do sistema.
        let start_type = query_start_type("RpcSs").expect("RpcSs deve existir");
        assert!(
            start_type == "auto" || start_type == "delayed-auto",
            "RpcSs deveria ser automático, veio: {}",
            start_type
        );
    }

    #[test]
    fn detects_existing_and_missing_services() {
        assert_eq!(exists("RpcSs"), Some(true));
        assert_eq!(exists("ServicoQueNaoExiste123"), Some(false));
    }
}
