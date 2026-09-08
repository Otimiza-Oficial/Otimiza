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

/// Lê `hypervisorlaunchtype` da saída do `bcdedit`.
///
/// Separado da execução para poder ser testado: é esta linha que decide se o
/// hipervisor sobe no boot, e é dela que sai o valor guardado para reverter.
/// Os nomes das opções do `bcdedit` são em inglês em qualquer idioma.
pub fn parse_hypervisor_launch_type(saida: &str) -> Option<String> {
    saida
        .lines()
        .map(|linha| linha.trim().to_lowercase())
        .find(|linha| linha.starts_with("hypervisorlaunchtype"))
        .and_then(|linha| linha.split_whitespace().nth(1).map(|v| v.to_string()))
}

/// Como o hipervisor está configurado para subir nesta máquina.
///
/// `None` quer dizer que não conseguimos ler — sem elevação, por exemplo. Não
/// significa "desligado", e a diferença importa: sem saber o estado anterior não
/// há como prometer a reversão.
pub fn hypervisor_launch_type() -> Option<String> {
    let saida = shell::run("bcdedit", &["/enum", "{current}"]).ok()?;

    if !saida.success {
        return None;
    }

    parse_hypervisor_launch_type(&saida.stdout)
}

/// Valor atual de um ajuste no modo "na bateria".
///
/// Ausente significa que o plano herda o padrão do Windows — e o padrão da
/// bateria é conservador de propósito: 5% de estado mínimo do processador, por
/// exemplo. Herdar, ali, é o mesmo que não ter aplicado nada.
pub fn read_power_setting_dc(
    scheme: &str,
    subgroup: &str,
    setting: &str,
) -> Result<crate::modules::changelog::PreviousValue, String> {
    super::registry::read(
        "HKLM",
        &power_setting_path(scheme, subgroup, setting),
        "DCSettingIndex",
    )
}

/// O valor que o próprio Windows declara como padrão para um ajuste.
///
/// Serve à reversão de um ajuste que não tinha valor antes: o plano herdava o
/// padrão, e o `powercfg` não sabe apagar um valor. Ler o padrão declarado é a
/// única forma de devolver o estado original sem inventar número — e esta
/// chave, ao contrário das de `PowerSchemes`, é legível.
fn valor_padrao(scheme: &str, subgroup: &str, setting: &str, indice: &str) -> Option<u32> {
    use crate::modules::changelog::PreviousValue;

    let caminho = format!(
        r"SYSTEM\CurrentControlSet\Control\Power\PowerSettings\{}\{}\DefaultPowerSchemeValues\{}",
        subgroup, setting, scheme
    );

    match super::registry::read("HKLM", &caminho, indice) {
        Ok(PreviousValue::Dword(valor)) => Some(valor),
        _ => None,
    }
}

/// O valor lido do plano, ou o padrão que ele herda quando não tem o próprio.
pub fn resolver_valor(no_plano: Option<u32>, padrao: Option<u32>) -> Option<u32> {
    no_plano.or(padrao)
}

/// O valor que a máquina realmente usa para um ajuste.
pub fn valor_efetivo(scheme: &str, subgroup: &str, setting: &str, bateria: bool) -> Option<u32> {
    use crate::modules::changelog::PreviousValue;

    let indice = if bateria { "DCSettingIndex" } else { "ACSettingIndex" };

    let no_plano = match super::registry::read(
        "HKLM",
        &power_setting_path(scheme, subgroup, setting),
        indice,
    ) {
        Ok(PreviousValue::Dword(valor)) => Some(valor),
        _ => None,
    };

    resolver_valor(no_plano, valor_padrao(scheme, subgroup, setting, indice))
}

/// Se o ajuste vale nos DOIS modos de alimentação.
///
/// A regra existe por um defeito medido: o produto gravava só o valor da tomada
/// e dava a otimização por aplicada. Num notebook fora da tomada — o público
/// deste produto — ela não mudava nada, e a tela dizia que sim.
pub fn power_setting_satisfeito(ac: Option<u32>, dc: Option<u32>, alvo: u32) -> bool {
    ac == Some(alvo) && dc == Some(alvo)
}

/// Grava um ajuste nos DOIS modos de alimentação e reativa o plano, para valer
/// na hora e não só no próximo boot.
///
/// Os dois, e não só a tomada: ver `power_setting_satisfeito`.
pub fn set_power_setting(
    scheme: &str,
    subgroup: &str,
    setting: &str,
    value: u32,
) -> Result<(), String> {
    let valor = value.to_string();

    shell::run_checked(
        "powercfg",
        &["-setacvalueindex", scheme, subgroup, setting, &valor],
    )
    .map_err(|e| format!("Este ajuste não existe neste Windows: {}", e))?;

    // A falha na bateria não invalida o que já valeu na tomada, mas não pode
    // passar em silêncio: seria o defeito original de volta, calado.
    shell::run_checked(
        "powercfg",
        &["-setdcvalueindex", scheme, subgroup, setting, &valor],
    )
    .map_err(|e| format!("O ajuste valeu na tomada, mas não na bateria: {}", e))?;

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
    previous_dc: Option<&crate::modules::changelog::PreviousValue>,
) -> Result<(), String> {
    use crate::modules::changelog::PreviousValue;

    // SÓ pelo `powercfg`. As chaves de `PowerSchemes` pertencem ao SISTEMA e
    // negam escrita direta mesmo a um administrador — conferido contra a
    // máquina do dono, onde a tentativa volta com "acesso negado" e a reversão
    // falha, deixando aplicado no PC do cliente o que ele mandou desfazer.
    let escrever = |indice: &str, valor: u32| -> Result<(), String> {
        shell::run_checked(
            "powercfg",
            &[indice, scheme, subgroup, setting, &valor.to_string()],
        )
        .map(|_| ())
    };

    match previous {
        PreviousValue::Dword(valor) => escrever("-setacvalueindex", *valor)?,
        // Não havia valor: o plano herdava o padrão. Devolvemos o padrão que o
        // próprio Windows declara — não há como apagar o valor pelo `powercfg`,
        // e o padrão declarado é o estado que o cliente tinha, não um número
        // inventado por nós.
        _ => {
            if let Some(padrao) = valor_padrao(scheme, subgroup, setting, "ACSettingIndex") {
                escrever("-setacvalueindex", padrao)?;
            }
        }
    }

    match previous_dc {
        Some(PreviousValue::Dword(valor)) => escrever("-setdcvalueindex", *valor)?,
        Some(_) => {
            if let Some(padrao) = valor_padrao(scheme, subgroup, setting, "DCSettingIndex") {
                escrever("-setdcvalueindex", padrao)?;
            }
        }
        // `None` é mudança gravada antes de o produto escrever a bateria: nunca
        // mexemos nela, então não se mexe agora.
        None => {}
    }

    set_active_scheme(scheme)
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
/// O que sabemos sobre o Armazenamento Reservado desta máquina.
///
/// Os dois últimos existem separados por um defeito real: numa leitura recusada
/// o produto concluía que o Windows não tem o recurso, e dizia ao cliente "não
/// se aplica a esta máquina". Medido no Windows 11 Pro da máquina de
/// desenvolvimento, com o programa **elevado**,
/// `Get-WindowsReservedStorageState` responde "Acesso negado" — e a máquina pode
/// muito bem ter o recurso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoReservado {
    Ligado,
    Desligado,
    /// O comando respondeu e não trouxe estado: este Windows não tem o recurso.
    SemRecurso,
    /// O comando não respondeu. Não sabemos, e não vamos fingir que sabemos.
    NaoVerificavel,
}

/// Regra pura da classificação, separada da execução para poder ser testada.
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

// `reserved_storage_enabled` morava aqui, devolvendo `Option<bool>`. Foi
// removida porque o `None` dela era a própria conflação que a 1.7 veio
// desfazer: juntava "o Windows respondeu que não tem o recurso" com "o Windows
// recusou responder". Os dois lados — inspeção e aplicação — agora usam
// `estado_do_armazenamento_reservado`, que separa os dois casos.

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
        // Medido na máquina do dono, Windows 11 Pro, com o programa ELEVADO:
        // `Get-WindowsReservedStorageState` existe e responde "Acesso negado".
        // Concluir daí que o Windows não tem o recurso é afirmar o que não foi
        // verificado — e faz o produto dizer "não se aplica a esta máquina"
        // sobre uma máquina onde ele pode muito bem se aplicar.
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
        // Ausente significa que não conseguimos ler — e sem saber o estado
        // anterior não há como prometer a volta.
        let saida = "identifier              {current}\nnx                      OptOut\n";
        assert_eq!(parse_hypervisor_launch_type(saida), None);
    }

    #[test]
    fn ajuste_so_na_tomada_nao_esta_satisfeito() {
        // O defeito medido na máquina do dono: gravávamos só `ACSettingIndex`,
        // e o valor da bateria seguia herdando o padrão do Windows — 5% de
        // estado mínimo do processador. Num notebook fora da tomada a
        // otimização não fazia nada, e a lista dizia que estava aplicada.
        assert!(!power_setting_satisfeito(Some(100), None, 100));
    }

    #[test]
    fn ajuste_com_bateria_no_padrao_antigo_nao_esta_satisfeito() {
        assert!(!power_setting_satisfeito(Some(100), Some(5), 100));
    }

    #[test]
    fn ajuste_nos_dois_modos_esta_satisfeito() {
        assert!(power_setting_satisfeito(Some(100), Some(100), 100));
    }

    #[test]
    fn valor_do_plano_vence_o_padrao() {
        assert_eq!(resolver_valor(Some(100), Some(5)), Some(100));
    }

    #[test]
    fn sem_valor_no_plano_vale_o_padrao_declarado() {
        // Ausente não é vazio: é herdar o padrão, e o padrão é o que a máquina
        // de fato usa. Tratar como "não aplicado" faria o produto aplicar e
        // depois desfazer para o mesmo número.
        assert_eq!(resolver_valor(None, Some(5)), Some(5));
        assert_eq!(resolver_valor(None, Some(100)), Some(100));
    }

    #[test]
    fn sem_valor_e_sem_padrao_continua_desconhecido() {
        assert_eq!(resolver_valor(None, None), None);
    }

    #[test]
    fn bateria_herdando_padrao_diferente_do_alvo_nao_esta_satisfeita() {
        // Estado mínimo do processador: padrão da bateria 5%, alvo 100%.
        assert!(!power_setting_satisfeito(Some(100), resolver_valor(None, Some(5)), 100));
    }

    #[test]
    fn bateria_herdando_padrao_igual_ao_alvo_esta_satisfeita() {
        // Estacionamento de núcleos no plano Alto Desempenho: padrão já é 100.
        assert!(power_setting_satisfeito(Some(100), resolver_valor(None, Some(100)), 100));
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
