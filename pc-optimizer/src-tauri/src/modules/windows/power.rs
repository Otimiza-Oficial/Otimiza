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

/// O nome dado à cópia, quando o Alto Desempenho não existe na máquina.
///
/// Existe para que a cópia seja REENCONTRADA na próxima execução. Sem nome
/// nosso, cada tentativa criaria mais um plano chamado "Alto desempenho" e o
/// cliente acabaria com uma lista deles.
pub const NOME_DA_COPIA: &str = "OTIMIZA Alto Desempenho";

/// O GUID do plano de alto desempenho desta máquina, SEM CRIAR NADA.
///
/// `None` quer dizer que ele não existe aqui — não que a leitura falhou; nesse
/// caso a lista vem vazia e a resposta também é `None`, e quem chama trata as
/// duas como "não sei se está satisfeito".
pub fn alto_desempenho_existente() -> Option<String> {
    use super::planoenergia;

    let lista = shell::run_checked("powercfg", &["/list"]).ok()?;
    let planos = planoenergia::planos_da_saida(&lista);

    if planos.iter().any(|(g, _)| g == HIGH_PERFORMANCE_GUID) {
        return Some(HIGH_PERFORMANCE_GUID.to_string());
    }

    planoenergia::achar_na_lista(&planos, NOME_DA_COPIA)
}

/// Garante que exista um plano de alto desempenho E DEVOLVE O GUID DELE.
///
/// ESTA FUNÇÃO DEVOLVIA `()`, E ESSE ERA O DEFEITO QUE MAIS DOÍA NO PC DO
/// CLIENTE. Em notebook com Modern Standby e em imagem OEM enxuta o Alto
/// Desempenho não existe. O código de antes chamava `-duplicatescheme`, que CRIA
/// UMA CÓPIA COM GUID NOVO, jogava a resposta fora, e em seguida mandava ativar
/// o GUID FIXO — que continua não existindo ali. Conferido nesta máquina: a
/// duplicação respondeu `GUID do Esquema de Energia: 15a86c79-…`, diferente do
/// pedido.
///
/// Resultado no cliente: a otimização falhava, e cada tentativa deixava mais um
/// plano órfão para trás. Na máquina de desenvolvimento nada disso aparecia,
/// porque aqui o Alto Desempenho existe.
pub fn garantir_alto_desempenho() -> Result<String, String> {
    if let Some(guid) = alto_desempenho_existente() {
        return Ok(guid);
    }

    let saida = shell::run_checked("powercfg", &["-duplicatescheme", HIGH_PERFORMANCE_GUID])
        .map_err(|e| {
            format!(
                "Este Windows não tem o plano Alto Desempenho e não deixou copiá-lo: {}",
                e
            )
        })?;

    let novo = parse_active_guid(&saida).ok_or_else(|| {
        format!(
            "O Windows copiou o plano mas não disse qual é o GUID dele. Resposta: {}",
            saida.trim()
        )
    })?;

    super::planoenergia::validar_guid_novo(&novo, HIGH_PERFORMANCE_GUID)?;

    // O nome é o que reencontra a cópia na próxima execução.
    shell::run_checked("powercfg", &["-changename", &novo, NOME_DA_COPIA, ""])
        .map_err(|e| format!("O plano foi copiado mas não pôde ser nomeado: {}", e))?;

    Ok(novo)
}

/// `None` é NÃO CONSEGUI LER, e não "desligada".
///
/// Devolvia `bool`, e a diferença custava caro: erro de leitura, permissão
/// negada e valor ausente caíam todos em `false`. Três consequências, todas
/// invisíveis:
///
/// 1. a lista dizia "hibernação já desativada" num PC onde o `hiberfil.sys`
///    continua ocupando o tamanho da RAM;
/// 2. a otimização nunca rodava, e o espaço prometido nunca aparecia;
/// 3. pior de tudo, esta MESMA função era usada para CONFERIR a escrita — a
///    verificação validava a si mesma. Leitura quebrada devolvia "antes:
///    desligada" e "depois: desligada", e o produto dava por conferido o que
///    nunca leu.
pub fn hibernation_enabled() -> Option<bool> {
    use crate::modules::changelog::PreviousValue;

    match super::registry::read(
        "HKLM",
        r"SYSTEM\CurrentControlSet\Control\Power",
        "HibernateEnabled",
    ) {
        Ok(PreviousValue::Dword(v)) => Some(v == 1),
        // Valor ausente: o Windows trata a ausência como desligada, e aqui a
        // ausência foi LIDA — é resposta, não silêncio.
        Ok(PreviousValue::Absent) | Ok(PreviousValue::AbsentKey) => Some(false),
        _ => None,
    }
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
///
/// O NOME do elemento é inglês em qualquer idioma — conferido nesta máquina, que
/// imprime o cabeçalho em português e os nomes em inglês. O VALOR é normalizado
/// para a palavra-chave que o `bcdedit /set` aceita.
///
/// Sem isso, o texto que o `bcdedit` imprime viraria argumento do `bcdedit` no
/// desfazer, e os dois lados só coincidem enquanto o Windows imprimir em inglês.
/// Ver `firmware::palavra_chave_do_hipervisor`.
///
/// `None` continua sendo "não consegui ler" — agora inclui "li e não reconheço",
/// que dá no mesmo para quem precisa prometer a volta.
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

/// O valor que o próprio Windows declara como padrão para um ajuste.
///
/// Serve à reversão de um ajuste que não tinha valor antes: o plano herdava o
/// padrão, e o `powercfg` não sabe apagar um valor. Ler o padrão declarado é a
/// única forma de devolver o estado original sem inventar número — e esta
/// chave, ao contrário das de `PowerSchemes`, é legível.
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

/// O valor lido do plano, ou o padrão que ele herda quando não tem o próprio.
pub fn resolver_valor(no_plano: Option<u32>, padrao: Option<u32>) -> Option<u32> {
    no_plano.or(padrao)
}

/// O número dentro de um `ACSettingIndex` / `DCSettingIndex`.
///
/// NEM TODO ÍNDICE É `REG_DWORD`, e descobrir isso custou um teste contra a
/// máquina de verdade. Medido aqui: depois de gravar a economia de energia do
/// adaptador sem fio (`12bbebe6-…`), o `powercfg /query` mostrava o valor certo
/// e o registro guardava `REG_BINARY {0,0,0,0}` — enquanto todos os outros dez
/// ajustes do mesmo plano ficaram `REG_DWORD`. O tipo acompanha o que o plano de
/// origem já tinha, e plano de origem varia de máquina para máquina.
///
/// Ler só `Dword` fazia duas coisas erradas, e as duas calado:
///
/// 1. O ajuste nunca parecia satisfeito, então era REESCRITO toda vez;
/// 2. Pior, o valor anterior guardado para desfazer virava `Binary`, e
///    `restore_power_setting` trata o que não é `Dword` como "não havia valor" —
///    ou seja, o desfazer GRAVAVA O PADRÃO DO WINDOWS POR CIMA da configuração
///    que o cliente tinha.
///
/// Quatro bytes em little-endian é como o Windows guarda esse índice quando ele
/// vem binário. Tamanho diferente disso não é um índice, e não vira palpite.
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

/// O valor que a máquina realmente usa para um ajuste.
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

// `power_setting_satisfeito` MORAVA AQUI.
//
// A regra que ela carregava — o ajuste precisa valer na TOMADA E NA BATERIA,
// senão um notebook fora da tomada não mudava nada e a tela dizia que sim —
// não foi perdida: ela virou `planoenergia::ja_satisfeito`, que sabe além
// disso que no notebook a bateria pode NÃO ser alvo de propósito. Os testes
// foram junto, inclusive o que combina a herança do padrão do Windows.


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

    // `indice_do_valor`, e não `PreviousValue::Dword` direto: o índice pode ter
    // sido lido como `REG_BINARY`, e nesse caso o braço de baixo gravaria o
    // PADRÃO DO WINDOWS por cima do valor que o cliente tinha. Ver o comentário
    // de `indice_do_valor`.
    match indice_do_valor(previous) {
        Some(valor) => escrever("-setacvalueindex", valor)?,
        // Não havia valor: o plano herdava o padrão. Devolvemos o padrão que o
        // próprio Windows declara — não há como apagar o valor pelo `powercfg`,
        // e o padrão declarado é o estado que o cliente tinha, não um número
        // inventado por nós.
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
    fn indice_gravado_como_binario_e_lido_igual_ao_dword() {
        // MEDIDO NA MÁQUINA, e não deduzido: depois de gravar a economia de
        // energia do adaptador sem fio, o registro guardou `REG_BINARY
        // {0,0,0,0}` enquanto os outros dez ajustes do mesmo plano ficaram
        // `REG_DWORD`. Sem isto, o ajuste era reescrito toda vez e — o que
        // machuca — o desfazer gravava o padrão do Windows por cima do valor do
        // cliente, porque o que não era `Dword` contava como "não havia valor".
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
        // Ausência de valor é diferente de um valor que não sabemos ler, e
        // inventar um número aqui é escrever no PC do cliente por adivinhação.
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
