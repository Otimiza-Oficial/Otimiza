// Planos de energia do Windows.

use super::shell;

pub const HIGH_PERFORMANCE_GUID: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";

pub fn parse_active_guid(output: &str) -> Option<String> {
    let after_colon = output.split(':').nth(1)?;
    let guid = after_colon.split_whitespace().next()?;

    if guid.len() == 36 {
        Some(guid.to_lowercase())
    } else {
        None
    }
}

pub fn active_scheme() -> Result<String, String> {
    let output = shell::run_checked("powercfg", &["/getactivescheme"])?;

    parse_active_guid(&output)
        .ok_or_else(|| format!("Could not parse active power scheme from: {}", output.trim()))
}

pub fn set_active_scheme(guid: &str) -> Result<(), String> {
    shell::run_checked("powercfg", &["/setactive", guid])?;
    Ok(())
}

/// `None` é NÃO CONSEGUI LER, não "desligada": com `bool`, a leitura quebrada dizia "já desativada" com o
/// `hiberfil.sys` no disco, e a mesma função conferia a escrita, validando a si mesma.
pub fn hibernation_enabled() -> Option<bool> {
    use crate::modules::changelog::PreviousValue;

    match super::registry::read(
        "HKLM",
        r"SYSTEM\CurrentControlSet\Control\Power",
        "HibernateEnabled",
    ) {
        Ok(PreviousValue::Dword(v)) => Some(v == 1),
        // Ausência LIDA: o Windows a trata como desligada.
        Ok(PreviousValue::Absent) | Ok(PreviousValue::AbsentKey) => Some(false),
        _ => None,
    }
}

pub fn set_hibernation(enabled: bool) -> Result<(), String> {
    shell::run_checked("powercfg", &["/hibernate", if enabled { "on" } else { "off" }])?;
    Ok(())
}

/// Do registro, não do `powercfg /q`: o comando traduz os rótulos, e a reversão precisa funcionar em qualquer
/// idioma.
fn power_setting_path(scheme: &str, subgroup: &str, setting: &str) -> String {
    format!(
        r"SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes\{}\{}\{}",
        scheme, subgroup, setting
    )
}

/// Ausente: o plano herda o padrão do Windows.
pub fn read_power_setting(
    scheme: &str,
    subgroup: &str,
    setting: &str,
) -> Result<crate::modules::changelog::PreviousValue, String> {
    super::registry::read(
        "HKLM",
        &power_setting_path(scheme, subgroup, setting),
        "ACSettingIndex",
    )
}

/// O NOME do elemento é inglês em qualquer idioma; o VALOR é normalizado para a palavra-chave do `bcdedit /set`
/// (ver `firmware::palavra_chave_do_hipervisor`), senão o desfazer só funcionaria com o Windows em inglês. `None`
/// inclui "li e não reconheço".
pub fn parse_hypervisor_launch_type(saida: &str) -> Option<String> {
    saida
        .lines()
        .map(|linha| linha.trim().to_lowercase())
        .find(|linha| linha.starts_with("hypervisorlaunchtype"))
        .and_then(|linha| linha.split_whitespace().nth(1).map(|v| v.to_string()))
        .and_then(|valor| {
            super::firmware::palavra_chave_do_hipervisor(&valor).map(str::to_string)
        })
}

/// `None` = não se leu (sem elevação, por exemplo): sem o estado anterior não há como prometer a reversão.
pub fn hypervisor_launch_type() -> Option<String> {
    let saida = shell::run("bcdedit", &["/enum", "{current}"]).ok()?;

    if !saida.success {
        return None;
    }

    parse_hypervisor_launch_type(&saida.stdout)
}

/// O `powercfg` não apaga valor: devolver o padrão declarado é a única volta sem inventar número.
fn valor_padrao(scheme: &str, subgroup: &str, setting: &str, indice: &str) -> Option<u32> {
    let caminho = format!(
        r"SYSTEM\CurrentControlSet\Control\Power\PowerSettings\{}\{}\DefaultPowerSchemeValues\{}",
        subgroup, setting, scheme
    );

    match super::registry::read("HKLM", &caminho, indice) {
        Ok(valor) => indice_do_valor(&valor),
        _ => None,
    }
}

pub fn resolver_valor(no_plano: Option<u32>, padrao: Option<u32>) -> Option<u32> {
    no_plano.or(padrao)
}

/// NEM TODO ÍNDICE É `REG_DWORD`: medido, o ajuste do adaptador sem fio ficou `REG_BINARY {0,0,0,0}` (o tipo
/// segue o plano de origem). Lendo só `Dword`, o ajuste era reescrito sempre e o desfazer gravava o padrão do
/// Windows por cima do valor do cliente. Quatro bytes little-endian; outro tamanho não vira palpite.
pub fn indice_do_valor(valor: &crate::modules::changelog::PreviousValue) -> Option<u32> {
    use crate::modules::changelog::PreviousValue;

    match valor {
        PreviousValue::Dword(v) => Some(*v),
        PreviousValue::Binary(bytes) if bytes.len() == 4 => {
            Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
        _ => None,
    }
}

pub fn valor_efetivo(scheme: &str, subgroup: &str, setting: &str, bateria: bool) -> Option<u32> {
    let indice = if bateria { "DCSettingIndex" } else { "ACSettingIndex" };

    let no_plano = match super::registry::read(
        "HKLM",
        &power_setting_path(scheme, subgroup, setting),
        indice,
    ) {
        Ok(valor) => indice_do_valor(&valor),
        _ => None,
    };

    resolver_valor(no_plano, valor_padrao(scheme, subgroup, setting, indice))
}

/// Sem valor anterior, volta a herdar o padrão em vez de ficar com um número que inventamos.
pub fn restore_power_setting(
    scheme: &str,
    subgroup: &str,
    setting: &str,
    previous: &crate::modules::changelog::PreviousValue,
    previous_dc: Option<&crate::modules::changelog::PreviousValue>,
) -> Result<(), String> {
    // SÓ pelo `powercfg`: as chaves de `PowerSchemes` são do SISTEMA e negam escrita até a administrador.
    let escrever = |indice: &str, valor: u32| -> Result<(), String> {
        shell::run_checked(
            "powercfg",
            &[indice, scheme, subgroup, setting, &valor.to_string()],
        )
        .map(|_| ())
    };

    // `indice_do_valor`, não `PreviousValue::Dword`: um índice `REG_BINARY` cairia no braço de baixo e gravaria o
    // padrão por cima.
    match indice_do_valor(previous) {
        Some(valor) => escrever("-setacvalueindex", valor)?,
        None => {
            if let Some(padrao) = valor_padrao(scheme, subgroup, setting, "ACSettingIndex") {
                escrever("-setacvalueindex", padrao)?;
            }
        }
    }

    match previous_dc.map(|v| (v, indice_do_valor(v))) {
        Some((_, Some(valor))) => escrever("-setdcvalueindex", valor)?,
        Some((_, None)) => {
            if let Some(padrao) = valor_padrao(scheme, subgroup, setting, "DCSettingIndex") {
                escrever("-setdcvalueindex", padrao)?;
            }
        }
        // `None`: mudança gravada antes de o produto escrever a bateria; ela nunca foi mexida.
        None => {}
    }

    set_active_scheme(scheme)
}

/// `Get-MMAgent` responde em inglês em qualquer idioma.
pub fn memory_compression_enabled() -> Option<bool> {
    let output = shell::powershell("(Get-MMAgent).MemoryCompression").ok()?;

    if !output.success {
        return None;
    }

    match output.stdout.trim() {
        "True" => Some(true),
        "False" => Some(false),
        _ => None,
    }
}

pub fn set_memory_compression(enabled: bool) -> Result<(), String> {
    let command = if enabled {
        "Enable-MMAgent -mc"
    } else {
        "Disable-MMAgent -mc"
    };

    shell::powershell_checked(command)?;
    Ok(())
}

/// Leitura recusada não é "sem recurso": elevado, no Windows 11 Pro, `Get-WindowsReservedStorageState` responde
/// "Acesso negado", e o produto dizia "não se aplica".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoReservado {
    Ligado,
    Desligado,
    SemRecurso,
    NaoVerificavel,
}

pub fn classificar_armazenamento_reservado(sucesso: bool, saida: &str) -> EstadoReservado {
    if !sucesso {
        return EstadoReservado::NaoVerificavel;
    }

    match saida.trim() {
        "Enabled" => EstadoReservado::Ligado,
        "Disabled" => EstadoReservado::Desligado,
        _ => EstadoReservado::SemRecurso,
    }
}

pub fn estado_do_armazenamento_reservado() -> EstadoReservado {
    match shell::powershell("(Get-WindowsReservedStorageState).ReservedStorageState") {
        Ok(saida) => classificar_armazenamento_reservado(saida.success, &saida.stdout),
        Err(_) => EstadoReservado::NaoVerificavel,
    }
}

pub fn set_reserved_storage(enabled: bool) -> Result<(), String> {
    let estado = if enabled { "Enabled" } else { "Disabled" };
    let script = format!(
        "Set-WindowsReservedStorageState -State {} -ErrorAction Stop",
        estado
    );

    shell::powershell_checked(&script).map_err(|e| {
        format!(
            "O Windows recusou alterar o Armazenamento Reservado. \
             Isso costuma acontecer quando há atualização em andamento: {}",
            e
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leitura_recusada_nao_vira_recurso_inexistente() {
        assert_eq!(
            classificar_armazenamento_reservado(false, ""),
            EstadoReservado::NaoVerificavel
        );
    }

    #[test]
    fn resposta_vazia_com_comando_ok_e_ausencia_do_recurso() {
        assert_eq!(
            classificar_armazenamento_reservado(true, ""),
            EstadoReservado::SemRecurso
        );
    }

    #[test]
    fn estados_declarados_sao_lidos() {
        assert_eq!(
            classificar_armazenamento_reservado(true, "Enabled"),
            EstadoReservado::Ligado
        );
        assert_eq!(
            classificar_armazenamento_reservado(true, " Disabled \r\n"),
            EstadoReservado::Desligado
        );
    }

    #[test]
    fn le_o_tipo_de_carga_do_hipervisor_na_saida_do_bcdedit() {
        let saida = "identifier              {current}\nhypervisorlaunchtype    Auto\nnx                      OptOut\n";
        assert_eq!(parse_hypervisor_launch_type(saida), Some("auto".to_string()));
    }

    #[test]
    fn hipervisor_desligado_e_reconhecido() {
        let saida = "identifier              {current}\nhypervisorlaunchtype    Off\n";
        assert_eq!(parse_hypervisor_launch_type(saida), Some("off".to_string()));
    }

    #[test]
    fn saida_sem_a_linha_nao_vira_palpite() {
        let saida = "identifier              {current}\nnx                      OptOut\n";
        assert_eq!(parse_hypervisor_launch_type(saida), None);
    }

    #[test]
    fn valor_do_plano_vence_o_padrao() {
        assert_eq!(resolver_valor(Some(100), Some(5)), Some(100));
    }

    #[test]
    fn sem_valor_no_plano_vale_o_padrao_declarado() {
        // Ausente é herdar o padrão: tratar como "não aplicado" aplicaria e desfaria para o mesmo número.
        assert_eq!(resolver_valor(None, Some(5)), Some(5));
        assert_eq!(resolver_valor(None, Some(100)), Some(100));
    }

    #[test]
    fn sem_valor_e_sem_padrao_continua_desconhecido() {
        assert_eq!(resolver_valor(None, None), None);
    }

    #[test]
    fn indice_gravado_como_binario_e_lido_igual_ao_dword() {
        use crate::modules::changelog::PreviousValue;

        assert_eq!(indice_do_valor(&PreviousValue::Dword(100)), Some(100));
        assert_eq!(indice_do_valor(&PreviousValue::Binary(vec![0, 0, 0, 0])), Some(0));
        assert_eq!(indice_do_valor(&PreviousValue::Binary(vec![100, 0, 0, 0])), Some(100));
        assert_eq!(
            indice_do_valor(&PreviousValue::Binary(vec![0xff, 0xff, 0xff, 0xff])),
            Some(u32::MAX)
        );
    }

    #[test]
    fn binario_de_outro_tamanho_nao_vira_palpite() {
        use crate::modules::changelog::PreviousValue;

        assert_eq!(indice_do_valor(&PreviousValue::Binary(vec![1, 0])), None);
        assert_eq!(indice_do_valor(&PreviousValue::Binary(vec![1, 0, 0, 0, 0])), None);
        assert_eq!(indice_do_valor(&PreviousValue::Absent), None);
        assert_eq!(indice_do_valor(&PreviousValue::AbsentKey), None);
        assert_eq!(indice_do_valor(&PreviousValue::Text("100".into())), None);
    }

    #[test]
    fn parses_active_scheme_guid() {
        let output = "Power Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced)\r\n";
        assert_eq!(
            parse_active_guid(output).as_deref(),
            Some("381b4222-f694-41f0-9685-ff5bb260df2e")
        );
    }

    #[test]
    fn parses_localized_output() {
        // A saída é traduzida; só o GUID é estável.
        let output = "GUID do Esquema de Energia: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (Alto desempenho)\r\n";
        assert_eq!(
            parse_active_guid(output).as_deref(),
            Some("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c")
        );
    }

    #[test]
    fn rejects_output_without_guid() {
        assert_eq!(parse_active_guid("no guid here"), None);
        assert_eq!(parse_active_guid("Power Scheme GUID: short (X)"), None);
    }
}
