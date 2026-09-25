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

/// Pelo NÚMERO, e não pelo rótulo: o rótulo é traduzido (ÉTAT, STATUS, STATO), e a busca por "STATE" ou
/// "ESTADO" dava "parado" sobre serviço rodando. A constante em inglês ao lado identifica a linha certa.
pub fn estado_da_saida_do_sc(stdout: &str) -> Option<u32> {
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

        // A constante separa a linha do estado da do tipo, que também começa com número.
        if partes.next().is_some_and(|c| ESTADOS.contains(&c)) {
            return Some(numero);
        }
    }

    None
}

/// `None` é NÃO CONSEGUI SABER, e não "parado": decide se o Windows Update é parado antes de apagar o cache de
/// atualização. Um `false` falso apagaria `SoftwareDistribution\Download` com a atualização em andamento.
pub fn is_running(service: &str) -> Option<bool> {
    let saida = super::shell::run("sc", &["query", service]).ok()?;

    if !saida.success {
        return None;
    }

    estado_da_saida_do_sc(&saida.stdout).map(|estado| estado == 4)
}

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

    /// Saída REAL desta máquina, em português, com os acentos estragados pelo código de página do console.
    const SAIDA_EM_PORTUGUES: &str = "NOME_DO_SERVI\u{fffd}O: wuauserv \n\
        \x20       TIPO               : 20  WIN32_SHARE_PROCESS  \n\
        \x20       ESTADO              : 1  STOPPED \n\
        \x20       C\u{fffd}DIGO_DE_SA\u{fffd}DA_DO_WIN32    : 1077  (0x435)\n\
        \x20       PONTO_DE_VERIFICA\u{fffd}\u{fffd}O         : 0x0\n";

    #[test]
    fn o_estado_e_lido_sem_procurar_o_rotulo_traduzido() {
        assert_eq!(estado_da_saida_do_sc(SAIDA_EM_PORTUGUES), Some(1));

        let em_frances = "        \u{c9}TAT               : 4  RUNNING \n";
        assert_eq!(estado_da_saida_do_sc(em_frances), Some(4));

        let em_alemao = "        STATUS             : 4  RUNNING \n";
        assert_eq!(estado_da_saida_do_sc(em_alemao), Some(4));
    }

    #[test]
    fn a_linha_do_tipo_nao_e_confundida_com_a_do_estado() {
        let so_o_tipo = "        TIPO               : 20  WIN32_SHARE_PROCESS  \n";
        assert_eq!(estado_da_saida_do_sc(so_o_tipo), None);
    }

    #[test]
    fn saida_que_nao_da_para_entender_vira_nao_sei() {
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
