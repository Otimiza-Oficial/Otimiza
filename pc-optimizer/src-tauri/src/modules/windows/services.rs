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

/// Verifica se um serviço existe nesta instalação do Windows.
pub fn exists(service: &str) -> bool {
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
        assert!(exists("RpcSs"));
        assert!(!exists("ServicoQueNaoExiste123"));
    }
}
