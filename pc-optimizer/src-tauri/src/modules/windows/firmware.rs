// Firmware e hardware: SÓ LÊ. Em placas de consumo a configuração da BIOS fica num bloco proprietário da NVRAM,
// com checksum de cada fabricante e sem API pública; escrever no lugar errado inutiliza a placa-mãe.

use super::shell;
use serde::{Deserialize, Serialize};

// Reexporta de `achados.rs`, para os `use super::firmware::{FindingSeverity, FixLocation}` existentes.
pub use super::achados::{FindingSeverity, FixLocation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MemoryModule {
    device_locator: Option<String>,
    bank_label: Option<String>,
    speed: Option<u32>,
    configured_clock_speed: Option<u32>,
    capacity: Option<u64>,
}

/// Nomes de propriedade do WMI são estáveis em qualquer idioma; o JSON evita depender de colunas.
fn query_json(script: &str) -> Option<String> {
    let output = shell::powershell(script).ok()?;

    if output.success && !output.stdout.trim().is_empty() {
        Some(output.stdout)
    } else {
        None
    }
}

/// `None` = não consegui perguntar: `unwrap_or_default()` juntava a falha com a máquina que não reporta pente.
fn memory_modules() -> Option<Vec<MemoryModule>> {
    // `@()` força array mesmo com um pente só.
    let script = "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-CimInstance \
                  Win32_PhysicalMemory | Select-Object DeviceLocator,BankLabel,Speed,\
                  ConfiguredClockSpeed,Capacity,PartNumber)";

    query_json(script).and_then(|json| serde_json::from_str(&json).ok())
}

/// O dado cru para desenhar os slots (o relatório só devolve achados): três encaixes vazios explicam canal único
/// melhor que uma frase.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoriaInstalada {
    pub slots: Option<u32>,
    pub pentes_gb: Vec<f64>,
    /// Canal único com slots livres: o achado mais comum em PC de jogo montado barato.
    pub canais: usize,
    pub mhz: Option<u32>,
}

#[cfg(target_os = "windows")]
pub fn memoria_instalada() -> MemoriaInstalada {
    // Aqui a lista vazia serve: quem distingue "não li" de "não há" é `achados_de_memoria`.
    let modules = memory_modules().unwrap_or_default();

    MemoriaInstalada {
        slots: memory_slots(),
        pentes_gb: modules
            .iter()
            .filter_map(|m| m.capacity)
            .map(|bytes| bytes as f64 / 1_073_741_824.0)
            .collect(),
        canais: occupied_channels(&modules),
        // A configurada, não a nominal: um pente de 3200 a 2133 é o caso que o cliente não vê.
        mhz: modules.iter().filter_map(|m| m.configured_clock_speed).max(),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn memoria_instalada() -> MemoriaInstalada {
    MemoriaInstalada { slots: None, pentes_gb: Vec::new(), canais: 0, mhz: None }
}

fn memory_slots() -> Option<u32> {
    let script = "(Get-CimInstance Win32_PhysicalMemoryArray).MemoryDevices";
    query_json(script)?.trim().parse().ok()
}

/// Slots como "Controller0-ChannelA-DIMM0". O DIMM sai (senão DIMM0 e DIMM1 no mesmo canal contariam dois); o
/// controlador FICA (em placas Intel os canais são "Controller0-ChannelA" e "Controller1-ChannelA").
fn occupied_channels(modules: &[MemoryModule]) -> usize {
    let mut channels: Vec<String> = modules
        .iter()
        .filter_map(|module| {
            let label = module
                .device_locator
                .as_deref()
                .or(module.bank_label.as_deref())?
                .to_uppercase();

            if !label.contains("CHANNEL") {
                return None;
            }

            let key: Vec<&str> = label
                .split('-')
                .filter(|part| !part.starts_with("DIMM"))
                .collect();

            Some(key.join("-"))
        })
        .collect();

    channels.sort();
    channels.dedup();

    // Sem rótulo reconhecível, cada pente conta como um canal: o palpite menos alarmista.
    if channels.is_empty() {
        modules.len()
    } else {
        channels.len()
    }
}

/// Só memória, em milissegundos pelo WMI, para a tela inicial: o `analyze()` completo leva ~12 s de carga.

/// `None` = não consegui perguntar; `Some(vec![])` = perguntei e não há pente. Juntos, a tela calava sobre
/// memória, e o canal único é o achado mais valioso.
fn achados_de_memoria(
    modules: Option<Vec<MemoryModule>>,
) -> Result<Vec<FirmwareFinding>, String> {
    let Some(modules) = modules else {
        return Err(
            "Não foi possível ler os pentes de memória desta máquina, então não sabemos se ela \
             está em canal único nem se o perfil de velocidade está ativo."
                .to_string(),
        );
    };

    if modules.is_empty() {
        return Ok(Vec::new());
    }

    let mut findings = Vec::new();
    analisar_pentes(&modules, &mut findings);
    Ok(findings)
}

pub fn analyze_memory_ou_lacuna() -> Result<Vec<FirmwareFinding>, String> {
    achados_de_memoria(memory_modules())
}

fn analisar_pentes(modules: &[MemoryModule], findings: &mut Vec<FirmwareFinding>) {
    let slots = memory_slots().unwrap_or(0);
    let channels = occupied_channels(&modules);
    let total_gb: f64 = modules
        .iter()
        .filter_map(|m| m.capacity)
        .sum::<u64>() as f64
        / 1_073_741_824.0;

    if channels < 2 && slots >= 2 {
        findings.push(FirmwareFinding {
            id: "memory_single_channel".to_string(),
            title: "Memória em canal único".to_string(),
            measured: format!(
                "{} pente de {:.0} GB em {} slots da placa, ocupando 1 canal.",
                modules.len(),
                total_gb,
                slots
            ),
            advice: "Acrescente um segundo pente igual para ativar o canal duplo. \
                     Costuma render de 10% a 25% em jogos limitados por processador — \
                     mais que qualquer ajuste de software. Nenhum programa faz isso: \
                     depende de peça."
                .to_string(),
            severity: FindingSeverity::Critical,
            fix_location: FixLocation::Hardware,
        });
    } else if channels >= 2 {
        findings.push(FirmwareFinding {
            id: "memory_dual_channel".to_string(),
            title: "Memória em canal duplo".to_string(),
            measured: format!("{} pentes ocupando {} canais.", modules.len(), channels),
            advice: String::new(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::None,
        });
    }

    let nominal = modules.iter().filter_map(|m| m.speed).max();
    let running = modules.iter().filter_map(|m| m.configured_clock_speed).max();

    if let (Some(nominal), Some(running)) = (nominal, running) {
        // O firmware costuma reportar 2667 para um pente de 2666.
        if running + 1 < nominal {
            findings.push(FirmwareFinding {
                id: "memory_xmp_off".to_string(),
                title: "Memória abaixo da velocidade do pente".to_string(),
                measured: format!("Rodando a {} MHz; o pente é de {} MHz.", running, nominal),
                advice: format!(
                    "Entre na BIOS e ative o perfil XMP (Intel) ou EXPO/DOCP (AMD). \
                     São {} MHz de banda de memória parados. Não há como ligar isso \
                     por software.",
                    nominal - running
                ),
                severity: FindingSeverity::Critical,
                fix_location: FixLocation::Bios,
            });
        } else {
            findings.push(FirmwareFinding {
                id: "memory_at_rated_speed".to_string(),
                title: "Memória na velocidade nominal".to_string(),
                measured: format!("Rodando a {} MHz, o nominal do pente.", running),
                advice: "Passar disso exigiria pente mais rápido E placa que suporte — \
                         não é ajuste, é troca de peça."
                    .to_string(),
                severity: FindingSeverity::Ok,
                fix_location: FixLocation::None,
            });
        }
    }
}

/// Sequela de `msconfig` seguindo tutorial ruim: o Windows usa só parte do processador ou da RAM, para sempre.
/// Os nomes do `bcdedit` são em inglês em qualquer idioma.
pub fn parse_boot_limits(output: &str) -> Vec<(String, String)> {
    const LIMITS: [&str; 3] = ["numproc", "truncatememory", "removememory"];

    output
        .lines()
        .filter_map(|line| {
            let lowered = line.trim().to_lowercase();
            let key = LIMITS.iter().find(|limit| lowered.starts_with(**limit))?;
            let value = lowered[key.len()..].trim().to_string();
            Some((key.to_string(), value))
        })
        .collect()
}

/// O valor IMPRESSO pelo `bcdedit` como a palavra-chave que o `bcdedit /set` aceita: no desfazer, o impresso vira
/// argumento, e só coincidiria enquanto o Windows imprimisse em inglês (aqui, em português, os valores saem em
/// inglês por sorte). `None`: não reconhecido, e o item não é oferecido. Cada elemento tem seu vocabulário: o
/// `hypervisorlaunchtype` aceita `off`/`auto`/`on` e RECUSA `no` (ver `palavra_chave_do_hipervisor`).
pub fn palavra_chave_booleana(valor: &str) -> Option<&'static str> {
    const SIM: &[&str] = &["yes", "true", "on", "1", "sim", "oui", "ja", "si", "sí", "sì"];
    const NAO: &[&str] = &["no", "false", "off", "0", "não", "nao", "non", "nein"];

    let v = valor.trim().to_lowercase();

    if SIM.contains(&v.as_str()) {
        Some("yes")
    } else if NAO.contains(&v.as_str()) {
        Some("no")
    } else {
        None
    }
}

pub fn palavra_chave_do_hipervisor(valor: &str) -> Option<&'static str> {
    match valor.trim().to_lowercase().as_str() {
        "off" | "desativado" | "désactivé" | "deaktiviert" => Some("off"),
        "auto" | "automático" | "automatico" | "automatique" | "automatisch" => Some("auto"),
        "on" | "ativado" | "activé" | "aktiviert" => Some("on"),
        _ => None,
    }
}

/// HPET forçado na configuração de boot: "dica de FPS" que obriga um temporizador mais lento e costuma dar
/// engasgo. Remover é desfazer estrago, não otimizar.
pub fn forced_platform_clock() -> Option<String> {
    let output = shell::run("bcdedit", &["/enum", "{current}"]).ok()?;

    if !output.success {
        return None;
    }

    output
        .stdout
        .lines()
        .map(|line| line.trim().to_lowercase())
        .find(|line| line.starts_with("useplatformclock"))
        // SEM `unwrap_or`: um `"sim"` cravado ia para `bcdedit /set` no desfazer e era recusado.
        .and_then(|line| line.split_whitespace().nth(1).map(str::to_string))
        .and_then(|valor| palavra_chave_booleana(&valor).map(str::to_string))
}

pub fn boot_limits() -> Vec<(String, String)> {
    match shell::run("bcdedit", &["/enum", "{current}"]) {
        Ok(output) if output.success => parse_boot_limits(&output.stdout),
        _ => Vec::new(),
    }
}

/// 0 desligada, 1 ativada sem rodar, 2 ativada e rodando.
pub fn vbs_running() -> Option<bool> {
    let script = "(Get-CimInstance -Namespace root\\Microsoft\\Windows\\DeviceGuard \
                  -ClassName Win32_DeviceGuard).VirtualizationBasedSecurityStatus";

    let value: u32 = query_json(script)?.trim().parse().ok()?;
    Some(value == 2)
}

/// 1 Credential Guard, 2 integridade de código pelo hypervisor, 3 System Guard. VBS LIGADO E VAZIO cobra o custo
/// sem proteção nenhuma: desligar aí não é trocar segurança por FPS.
pub fn vbs_servicos_ativos() -> Option<usize> {
    let script = "$g = Get-CimInstance -Namespace root\\Microsoft\\Windows\\DeviceGuard \
                  -ClassName Win32_DeviceGuard; \
                  @($g.SecurityServicesRunning | Where-Object { $_ -gt 0 }).Count";

    query_json(script)?.trim().parse().ok()
}

/// Só a virtualização de segurança, para o Início: duas consultas curtas ao Windows, sem carga nenhuma.
pub fn achados_do_vbs() -> Vec<FirmwareFinding> {
    let mut findings = Vec::new();
    analyze_vbs(&mut findings);
    findings
}

fn analyze_vbs(findings: &mut Vec<FirmwareFinding>) {
    match vbs_running() {
        Some(true) => {
            let servicos = vbs_servicos_ativos().unwrap_or(1);

            if servicos == 0 {
                findings.push(FirmwareFinding {
                    id: "vbs_sem_uso".to_string(),
                    title: "Virtualização de segurança ligada sem nada usando".to_string(),
                    measured: "VBS ativa e em execução, com 0 serviços de segurança \
                               rodando em cima dela."
                        .to_string(),
                    advice: "Este é o caso incomum em que desligar o VBS não custa \
                             proteção: nenhum serviço está usando a virtualização, então \
                             a máquina paga o preço em desempenho sem receber nada em \
                             troca. Ainda assim é você quem decide — se você usa ou \
                             pretende usar Hyper-V, WSL ou Sandbox, eles dependem disto."
                        .to_string(),
                    severity: FindingSeverity::Important,
                    fix_location: FixLocation::Software,
                });

                return;
            }

            findings.push(FirmwareFinding {
                id: "vbs_running".to_string(),
                title: "Virtualização de segurança ligada".to_string(),
                measured: format!(
                    "VBS ativa e em execução, com {} serviço(s) de segurança usando.",
                    servicos
                ),
                advice: "Custa desempenho em jogos, mais em processadores de 8ª a 10ª \
                         geração. A otimização \"Desligar virtualização de segurança\" \
                         desliga — leia o aviso de segurança antes, porque aqui você \
                         perde proteção real que está em uso. Se você nunca ligou isto: \
                         a partir de outubro de 2026 o Windows 11 liga a Integridade de Memória \
                         sozinho, por atualização, nos PCs compatíveis. Quem já tinha \
                         desligado continua desligado, diz a Microsoft."
                    .to_string(),
                severity: FindingSeverity::Important,
                fix_location: FixLocation::Software,
            })
        }
        Some(false) => findings.push(FirmwareFinding {
            id: "vbs_off".to_string(),
            title: "Virtualização de segurança desligada".to_string(),
            measured: "VBS não está em execução.".to_string(),
            advice: String::new(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::None,
        }),
        None => {}
    }
}

#[cfg(test)]
mod tests_1_7 {
    use super::*;

    /// Leitura falha não pode virar silêncio sobre memória.
    #[test]
    fn leitura_de_memoria_que_falhou_vira_lacuna_e_nao_silencio() {
        let erro = achados_de_memoria(None)
            .expect_err("leitura falha precisa virar Err, que o veredito exibe como lacuna");

        assert!(
            erro.to_lowercase().contains("memória") || erro.to_lowercase().contains("memoria"),
            "a lacuna precisa dizer o que não foi verificado: {}",
            erro
        );
    }

    #[test]
    fn maquina_sem_pente_reportado_nao_e_falha_de_leitura() {
        // Acontece em máquina virtual.
        let achados = achados_de_memoria(Some(Vec::new()))
            .expect("máquina sem pente reportado não é falha de leitura");

        assert!(achados.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(locator: &str) -> MemoryModule {
        MemoryModule {
            device_locator: Some(locator.to_string()),
            bank_label: None,
            speed: Some(3200),
            configured_clock_speed: Some(3200),
            capacity: Some(8_589_934_592),
        }
    }

    #[test]
    fn counts_distinct_channels() {
        let modules = vec![
            module("Controller0-ChannelA-DIMM0"),
            module("Controller1-ChannelB-DIMM0"),
        ];
        assert_eq!(occupied_channels(&modules), 2);
    }

    #[test]
    fn two_sticks_in_the_same_channel_is_still_single_channel() {
        let modules = vec![
            module("Controller0-ChannelA-DIMM0"),
            module("Controller0-ChannelA-DIMM1"),
        ];
        assert_eq!(occupied_channels(&modules), 1);
    }

    #[test]
    fn two_controllers_with_the_same_channel_letter_are_two_channels() {
        // Olhar só a letra acusaria canal único num PC correto e mandaria comprar RAM.
        let modules = vec![
            module("Controller0-ChannelA-DIMM0"),
            module("Controller1-ChannelA-DIMM0"),
        ];
        assert_eq!(occupied_channels(&modules), 2);
    }

    #[test]
    fn without_channel_labels_each_stick_counts_as_a_channel() {
        let modules = vec![module("DIMM0"), module("DIMM1")];
        assert_eq!(occupied_channels(&modules), 2);
    }

    #[test]
    fn o_valor_lido_do_bcdedit_volta_como_palavra_chave_que_ele_aceita() {
        assert_eq!(palavra_chave_booleana("Yes"), Some("yes"));
        assert_eq!(palavra_chave_booleana("No"), Some("no"));
        assert_eq!(palavra_chave_booleana("  TRUE "), Some("yes"));
        assert_eq!(palavra_chave_booleana("Sim"), Some("yes"));
        assert_eq!(palavra_chave_booleana("Não"), Some("no"));
    }

    #[test]
    fn valor_desconhecido_nao_vira_argumento_invalido() {
        assert_eq!(palavra_chave_booleana("talvez"), None);
        assert_eq!(palavra_chave_booleana(""), None);
    }

    #[test]
    fn o_hipervisor_tem_vocabulario_proprio() {
        assert_eq!(palavra_chave_do_hipervisor("Off"), Some("off"));
        assert_eq!(palavra_chave_do_hipervisor("Auto"), Some("auto"));
        assert_eq!(palavra_chave_do_hipervisor("On"), Some("on"));
        assert_eq!(palavra_chave_do_hipervisor("yes"), None);
    }

    #[test]
    fn finds_boot_limits_in_bcdedit_output() {
        let output = "identifier              {current}\nnumproc                 4\ntruncatememory          0x100000000\n";
        let limits = parse_boot_limits(output);

        assert_eq!(limits.len(), 2);
        assert_eq!(limits[0], ("numproc".to_string(), "4".to_string()));
    }

    #[test]
    fn clean_bcdedit_output_has_no_limits() {
        let output = "identifier              {current}\ndescription             Windows 11\n";
        assert!(parse_boot_limits(output).is_empty());
    }

}
