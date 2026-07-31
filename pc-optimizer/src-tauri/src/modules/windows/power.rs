// Planos de energia do Windows
//
// Em notebooks e em muitos desktops o plano "Equilibrado" reduz a frequência da
// CPU sob carga leve, o que causa engasgos e perda de FPS. Trocar para Alto
// Desempenho é uma das poucas otimizações com ganho consistente e mensurável.

use super::shell;

/// GUID fixo do plano "Alto Desempenho" — igual em todas as instalações do Windows.
pub const HIGH_PERFORMANCE_GUID: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";

/// Extrai o GUID da saída do `powercfg /getactivescheme`.
/// Formato: `Power Scheme GUID: 381b4222-... (Balanced)`
pub fn parse_active_guid(output: &str) -> Option<String> {
    let after_colon = output.split(':').nth(1)?;
    let guid = after_colon.split_whitespace().next()?;

    if guid.len() == 36 {
        Some(guid.to_lowercase())
    } else {
        None
    }
}

/// GUID do plano de energia ativo.
pub fn active_scheme() -> Result<String, String> {
    let output = shell::run_checked("powercfg", &["/getactivescheme"])?;

    parse_active_guid(&output)
        .ok_or_else(|| format!("Could not parse active power scheme from: {}", output.trim()))
}

/// Ativa um plano de energia pelo GUID.
pub fn set_active_scheme(guid: &str) -> Result<(), String> {
    shell::run_checked("powercfg", &["/setactive", guid])?;
    Ok(())
}

/// Garante que o plano Alto Desempenho exista.
///
/// Em algumas instalações (notebooks com Modern Standby, imagens OEM enxutas) o
/// plano vem oculto. Nesse caso ele é recriado a partir do modelo do sistema.
pub fn ensure_high_performance_exists() -> Result<(), String> {
    let list = shell::run_checked("powercfg", &["/list"])?;

    if list.to_lowercase().contains(HIGH_PERFORMANCE_GUID) {
        return Ok(());
    }

    shell::run_checked("powercfg", &["-duplicatescheme", HIGH_PERFORMANCE_GUID])
        .map_err(|e| format!("High performance power plan is unavailable on this system: {}", e))?;

    Ok(())
}

/// Se a hibernação está ligada. Lido do registro, que é a fonte de verdade e não
/// depende do idioma do Windows.
pub fn hibernation_enabled() -> bool {
    matches!(
        super::registry::read("HKLM", r"SYSTEM\CurrentControlSet\Control\Power", "HibernateEnabled"),
        Ok(crate::modules::changelog::PreviousValue::Dword(1))
    )
}

/// Liga ou desliga a hibernação. Desligar apaga o `hiberfil.sys`, liberando do
/// disco o equivalente à RAM instalada.
pub fn set_hibernation(enabled: bool) -> Result<(), String> {
    shell::run_checked("powercfg", &["/hibernate", if enabled { "on" } else { "off" }])?;
    Ok(())
}

/// Onde o Windows guarda os ajustes de cada plano de energia.
///
/// Ler daqui, e não da saída do `powercfg /q`, é o que faz a reversão funcionar
/// em Windows de qualquer idioma: o comando traduz os rótulos, o registro não.
fn power_setting_path(scheme: &str, subgroup: &str, setting: &str) -> String {
    format!(
        r"SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes\{}\{}\{}",
        scheme, subgroup, setting
    )
}

/// Valor atual de um ajuste no modo "ligado na tomada".
/// Ausente significa que o plano está herdando o padrão do Windows.
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

/// Grava um ajuste e reativa o plano, para valer na hora e não só no próximo boot.
pub fn set_power_setting(
    scheme: &str,
    subgroup: &str,
    setting: &str,
    value: u32,
) -> Result<(), String> {
    shell::run_checked(
        "powercfg",
        &["-setacvalueindex", scheme, subgroup, setting, &value.to_string()],
    )
    .map_err(|e| format!("Este ajuste não existe neste Windows: {}", e))?;

    set_active_scheme(scheme)
}

/// Devolve o ajuste ao estado anterior. Quando não havia valor, a chave é
/// apagada para o plano voltar a herdar o padrão em vez de ficar com um número
/// fixo que nós inventamos.
pub fn restore_power_setting(
    scheme: &str,
    subgroup: &str,
    setting: &str,
    previous: &crate::modules::changelog::PreviousValue,
) -> Result<(), String> {
    use crate::modules::changelog::PreviousValue;

    match previous {
        PreviousValue::Dword(value) => set_power_setting(scheme, subgroup, setting, *value),
        _ => {
            super::registry::restore(
                "HKLM",
                &power_setting_path(scheme, subgroup, setting),
                "ACSettingIndex",
                previous,
            )?;
            set_active_scheme(scheme)
        }
    }
}

/// Se a compressão de memória está ligada.
///
/// `Get-MMAgent` devolve nomes de propriedade em inglês em qualquer idioma do
/// Windows, então `True`/`False` são estáveis.
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

/// Se o Armazenamento Reservado está ligado.
///
/// O Windows 10 e 11 reservam vários GB do disco para atualizações. Em SSD
/// pequeno isso pesa. `Get-WindowsReservedStorageState` devolve `Enabled` ou
/// `Disabled` em inglês em qualquer idioma do sistema.
pub fn reserved_storage_enabled() -> Option<bool> {
    let output = shell::powershell(
        "(Get-WindowsReservedStorageState -ErrorAction SilentlyContinue).ReservedStorageState",
    )
    .ok()?;

    if !output.success {
        return None;
    }

    match output.stdout.trim() {
        "Enabled" => Some(true),
        "Disabled" => Some(false),
        _ => None,
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
    fn parses_active_scheme_guid() {
        let output = "Power Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced)\r\n";
        assert_eq!(
            parse_active_guid(output).as_deref(),
            Some("381b4222-f694-41f0-9685-ff5bb260df2e")
        );
    }

    #[test]
    fn parses_localized_output() {
        // A saída é traduzida conforme o idioma do Windows; só o GUID é estável.
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
