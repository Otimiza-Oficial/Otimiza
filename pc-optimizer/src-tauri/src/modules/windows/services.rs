// Serviços: a otimização mais perigosa do produto, então o tipo de início anterior é sempre lido antes. A
// leitura é pelo registro, e não pelo `sc qc`, que é traduzido e usa página de código OEM; a escrita pelo `sc
// config`, que avisa o gerenciador na hora.

use super::registry;
use crate::modules::changelog::PreviousValue;

const SERVICES_KEY: &str = r"SYSTEM\CurrentControlSet\Services";

/// 0 = boot, 1 = system (drivers de núcleo, nunca reconfigurados), 2 = automático, 3 = manual, 4 = desativado.
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

/// `None` é "não deu para ler": dizer "este Windows não tem o serviço" sobre uma ACL negada tira a otimização
/// da lista com a frase errada.
pub fn exists(service: &str) -> Option<bool> {
    registry::key_exists("HKLM", &service_key(service))
}

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

/// ESCREVE E RELÊ: `sc config` já devolveu zero com política de domínio e ACL negada, e o serviço ficou como
/// estava. Sem reler, o produto diria "desativei" sobre um serviço ligado.
pub fn set_start_type(service: &str, start_type: &str) -> Result<(), String> {
    // O `sc config` exige `start= valor`, com o espaço depois do sinal.
    super::shell::run_checked("sc", &["config", service, "start=", start_type])?;

    match query_start_type(service) {
        Ok(agora) if agora.eq_ignore_ascii_case(start_type) => Ok(()),
        Ok(agora) => Err(format!(
            "O comando não deu erro, mas o serviço `{service}` continua em \
             \"{agora}\" e não em \"{start_type}\". Costuma ser política do \
             Windows ou permissão negada na chave do serviço."
        )),
        Err(erro) => Err(format!(
            "O serviço `{service}` foi configurado, mas não deu para reler o \
             estado dele para confirmar: {erro}"
        )),
    }
}

pub fn stop(service: &str) -> Result<(), String> {
    let output = super::shell::run("sc", &["stop", service])?;

    // 1062 = serviço não iniciado. O código aparece em qualquer idioma.
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
    fn mudar_um_servico_relê_o_estado_antes_de_dar_por_feito() {
        let producao = include_str!("services.rs").split("#[cfg(test)]").next().unwrap();
        let corpo = producao
            .split("pub fn set_start_type")
            .nth(1)
            .expect("a função continua existindo");

        assert!(
            corpo.contains("query_start_type"),
            "`set_start_type` voltou a confiar no código de saída do `sc` sem reler"
        );
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
        // 0 (boot) e 1 (system) são drivers de núcleo: reconfigurá-los pode impedir o Windows de iniciar.
        assert_eq!(start_type_keyword(0, false), None);
        assert_eq!(start_type_keyword(1, false), None);
    }

    #[test]
    fn reads_real_service_start_type_from_registry() {
        // RpcSs existe em toda instalação e é automático; falharia se a leitura dependesse do idioma.
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
