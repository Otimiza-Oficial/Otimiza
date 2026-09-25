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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareReport {
    pub board: String,
    pub cpu: String,
    pub findings: Vec<FirmwareFinding>,
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

fn analyze_memory(findings: &mut Vec<FirmwareFinding>) {
    let Some(modules) = memory_modules() else {
        return;
    };

    if modules.is_empty() {
        return;
    }

    analisar_pentes(&modules, findings);
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

/// Sem elevação o `bcdedit` não responde e a lista volta vazia: sem isto o painel dava a inicialização por limpa
/// enquanto o catálogo dizia "só como administrador".
fn boot_limits_finding(limits: &[(String, String)], elevated: bool) -> FirmwareFinding {
    if limits.is_empty() && !elevated {
        return FirmwareFinding {
            id: "boot_limits_desconhecido".to_string(),
            title: "Limites de inicialização não verificados".to_string(),
            measured: "O Windows só responde a esta consulta para um programa aberto como \
                       administrador."
                .to_string(),
            advice: "Reabra o Otimiza como administrador para conferir se há núcleos ou memória \
                     limitados na inicialização."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Software,
        };
    }

    if limits.is_empty() {
        return FirmwareFinding {
            id: "boot_limits_clear".to_string(),
            title: "Inicialização sem limites artificiais".to_string(),
            measured: "Nenhum limite de núcleos ou memória na configuração de boot.".to_string(),
            advice: String::new(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::None,
        };
    }

    let described: Vec<String> = limits
        .iter()
        .map(|(key, value)| format!("{} = {}", key, value))
        .collect();

    FirmwareFinding {
        id: "boot_limits_present".to_string(),
        title: "Inicialização limitando o hardware".to_string(),
        measured: described.join(", "),
        advice: "O Windows está usando de propósito menos processador ou menos memória do que \
                 você tem. Isso quase sempre é sobra de mexida no msconfig. A otimização \
                 \"Liberar limites de inicialização\" corrige."
            .to_string(),
        severity: FindingSeverity::Critical,
        fix_location: FixLocation::Software,
    }
}

fn analyze_boot_limits(findings: &mut Vec<FirmwareFinding>) {
    findings.push(boot_limits_finding(
        &boot_limits(),
        super::registry::is_elevated(),
    ));
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

/// Mede a consequência, não a frequência (o Windows reporta a nominal): trabalho no fim de dez segundos de carga
/// contra o primeiro segundo.
pub fn measure_sustained_decay() -> f64 {
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    const SLICES: usize = 10;
    const SLICE: Duration = Duration::from_secs(1);

    let mut throughput = Vec::with_capacity(SLICES);

    for _ in 0..SLICES {
        let started = Instant::now();
        let deadline = started + SLICE;
        let mut operations: u64 = 0;
        let mut accumulator: u64 = 1;

        while Instant::now() < deadline {
            for _ in 0..4096 {
                accumulator = accumulator
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                accumulator ^= accumulator >> 33;
                black_box(accumulator);
            }
            operations += 4096;
        }

        let seconds = started.elapsed().as_secs_f64();
        throughput.push(operations as f64 / seconds.max(f64::MIN_POSITIVE));
    }

    decay_percent(&throughput)
}

pub fn decay_percent(throughput: &[f64]) -> f64 {
    if throughput.len() < 4 {
        return 0.0;
    }

    let first = throughput[0];
    if first <= 0.0 {
        return 0.0;
    }

    // Duas fatias, para um soluço no fim não virar superaquecimento.
    let tail = &throughput[throughput.len() - 2..];
    let last = tail.iter().sum::<f64>() / tail.len() as f64;

    (first - last) / first * 100.0
}

fn analyze_throttling(findings: &mut Vec<FirmwareFinding>) {
    let decay = measure_sustained_decay();

    // Abaixo de 8%, outros processos disputando a CPU explicam a variação.
    if decay >= 8.0 {
        findings.push(FirmwareFinding {
            id: "sustained_decay".to_string(),
            title: "Processador perde força sob carga longa".to_string(),
            measured: format!(
                "Entregou {:.0}% menos trabalho no fim de 10 segundos de carga do que no começo.",
                decay
            ),
            advice: "Sinal de limite de temperatura ou de energia. Verifique a refrigeração \
                     (pasta térmica, poeira, ventoinhas) e, na BIOS, os limites de potência \
                     do processador. Nenhum ajuste de software recupera isto."
                .to_string(),
            severity: FindingSeverity::Critical,
            fix_location: FixLocation::Hardware,
        });
    } else {
        findings.push(FirmwareFinding {
            id: "sustained_ok".to_string(),
            title: "Processador sustenta o desempenho".to_string(),
            // Terminar mais rápido é variação de medição: zero.
            measured: format!(
                "Perdeu apenas {:.0}% ao fim de 10 segundos de carga.",
                decay.max(0.0)
            ),
            advice: String::new(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::None,
        });
    }
}

fn board_name() -> String {
    let script = "$b = Get-CimInstance Win32_BaseBoard; \"$($b.Manufacturer) $($b.Product)\"";
    query_json(script)
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| "placa não identificada".to_string())
}

fn cpu_name() -> String {
    let mut system = sysinfo::System::new();

    // Só o NOME do processador: `refresh_cpu_all()` pagaria quase um segundo de amostragem à toa.
    system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing());

    system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_string())
        .unwrap_or_else(|| "processador não identificado".to_string())
}

pub fn analyze() -> FirmwareReport {
    let mut findings = Vec::new();

    analyze_memory(&mut findings);
    analyze_boot_limits(&mut findings);
    analyze_vbs(&mut findings);
    analyze_throttling(&mut findings);

    findings.sort_by_key(|finding| match finding.severity {
        FindingSeverity::Critical => 0,
        FindingSeverity::Important => 1,
        FindingSeverity::Ok => 2,
    });

    FirmwareReport {
        board: board_name(),
        cpu: cpu_name(),
        findings,
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
mod tests_1_6 {
    use super::*;

    #[test]
    fn sem_elevacao_uma_leitura_vazia_nao_e_boot_limpo() {
        let finding = boot_limits_finding(&[], false);
        assert_ne!(finding.severity, FindingSeverity::Ok);
        assert!(finding.measured.contains("administrador"));
    }

    #[test]
    fn com_elevacao_uma_leitura_vazia_e_boot_limpo_de_verdade() {
        assert_eq!(boot_limits_finding(&[], true).severity, FindingSeverity::Ok);
    }

    #[test]
    fn limite_encontrado_vale_com_ou_sem_elevacao() {
        let limites = vec![("numproc".to_string(), "4".to_string())];
        assert_eq!(
            boot_limits_finding(&limites, false).severity,
            FindingSeverity::Critical
        );
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

    #[test]
    fn stable_throughput_shows_no_decay() {
        let throughput = vec![100.0, 100.0, 99.0, 100.0, 99.0, 100.0];
        assert!(decay_percent(&throughput).abs() < 2.0);
    }

    #[test]
    fn falling_throughput_is_detected_as_decay() {
        let throughput = vec![100.0, 95.0, 88.0, 80.0, 72.0, 70.0];
        assert!(decay_percent(&throughput) > 25.0);
    }

    #[test]
    fn a_single_slow_slice_at_the_end_does_not_alone_decide() {
        let throughput = vec![100.0, 100.0, 100.0, 100.0, 100.0, 60.0];
        let decay = decay_percent(&throughput);
        assert!(decay > 0.0 && decay < 25.0);
    }

    #[test]
    fn analyzes_this_machine() {
        let report = analyze();
        println!("Placa: {}\nCPU:   {}", report.board, report.cpu);

        for finding in &report.findings {
            println!(
                "[{:?}/{:?}] {} — {}",
                finding.severity, finding.fix_location, finding.title, finding.measured
            );
        }

        assert!(!report.findings.is_empty());
        let severities: Vec<u8> = report
            .findings
            .iter()
            .map(|f| match f.severity {
                FindingSeverity::Critical => 0,
                FindingSeverity::Important => 1,
                FindingSeverity::Ok => 2,
            })
            .collect();
        assert!(severities.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}
