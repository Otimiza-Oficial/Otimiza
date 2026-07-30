// Firmware e hardware
//
// A BIOS é onde estão os maiores ganhos de desempenho de um PC — e é justamente
// onde nenhum programa pode escrever com segurança.
//
// Em placas de consumo (ASUS, MSI, Gigabyte, ASRock) as configurações não ficam
// em variáveis UEFI documentadas: ficam num bloco proprietário da NVRAM, com
// checksum próprio de cada fabricante e sem API pública. Só Dell, HP e Lenovo
// corporativos publicam interface WMI para isso. Escrever no lugar errado dessa
// NVRAM não derruba o Windows — inutiliza a placa-mãe.
//
// Então este módulo faz a única coisa honesta possível: LÊ o que a BIOS e o
// hardware estão fazendo com o desempenho, e diz a verdade — inclusive quando a
// verdade é "não existe software que resolva isto".

use super::shell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingSeverity {
    /// Custa desempenho de forma grande e comprovada.
    Critical,
    /// Vale corrigir, com ganho menor ou dependente do caso.
    Important,
    /// Está correto. Dizer o que está certo evita vender conserto de coisa boa.
    Ok,
}

/// Onde o problema se resolve. Serve para o produto não fingir que conserta o
/// que só se resolve trocando peça ou entrando na BIOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixLocation {
    /// Dá para corrigir por software, aqui mesmo.
    Software,
    /// Só na configuração da BIOS/UEFI, na mão, com o PC reiniciando.
    Bios,
    /// Só trocando ou acrescentando peça.
    Hardware,
    /// Nada a corrigir.
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareFinding {
    pub id: String,
    pub title: String,
    /// O que foi medido nesta máquina, com números.
    pub measured: String,
    /// O que fazer. Vazio quando não há nada a fazer.
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

// ---------------------------------------------------------------- memória

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MemoryModule {
    device_locator: Option<String>,
    bank_label: Option<String>,
    /// Velocidade nominal do pente, em MHz.
    speed: Option<u32>,
    /// Velocidade em que ele está realmente rodando.
    configured_clock_speed: Option<u32>,
    capacity: Option<u64>,
}

/// Consulta o WMI devolvendo JSON.
///
/// Os nomes das propriedades do WMI são estáveis em qualquer idioma do Windows,
/// ao contrário do texto formatado de quase todo comando do sistema. Serializar
/// para JSON evita depender de alinhamento de colunas.
fn query_json(script: &str) -> Option<String> {
    let output = shell::run("powershell", &["-NoProfile", "-Command", script]).ok()?;

    if output.success && !output.stdout.trim().is_empty() {
        Some(output.stdout)
    } else {
        None
    }
}

fn memory_modules() -> Vec<MemoryModule> {
    // O `@()` força array mesmo com um único pente, senão o JSON viria como objeto.
    let script = "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-CimInstance \
                  Win32_PhysicalMemory | Select-Object DeviceLocator,BankLabel,Speed,\
                  ConfiguredClockSpeed,Capacity,PartNumber)";

    query_json(script)
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

/// Total de slots de memória da placa.
fn memory_slots() -> Option<u32> {
    let script = "(Get-CimInstance Win32_PhysicalMemoryArray).MemoryDevices";
    query_json(script)?.trim().parse().ok()
}

/// Conta canais distintos ocupados.
///
/// O Windows nomeia os slots como "Controller0-ChannelA-DIMM0". Duas armadilhas
/// aqui, e errar qualquer uma inverte o diagnóstico:
///
/// - O número do DIMM precisa sair do identificador, senão dois pentes no MESMO
///   canal (DIMM0 e DIMM1) contariam como dois canais.
/// - O número do controlador precisa FICAR, porque em placas Intel os dois canais
///   costumam ser "Controller0-ChannelA" e "Controller1-ChannelA": olhar só a
///   letra juntaria os dois e acusaria canal único num PC saudável.
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

            // Mantém controlador e canal, descarta a posição do pente.
            let key: Vec<&str> = label
                .split('-')
                .filter(|part| !part.starts_with("DIMM"))
                .collect();

            Some(key.join("-"))
        })
        .collect();

    channels.sort();
    channels.dedup();

    // Sem rótulo de canal reconhecível, cada pente conta como um canal: é o
    // palpite menos alarmista, e não inventamos um problema que não vimos.
    if channels.is_empty() {
        modules.len()
    } else {
        channels.len()
    }
}

fn analyze_memory(findings: &mut Vec<FirmwareFinding>) {
    let modules = memory_modules();

    if modules.is_empty() {
        return;
    }

    let slots = memory_slots().unwrap_or(0);
    let channels = occupied_channels(&modules);
    let total_gb: f64 = modules
        .iter()
        .filter_map(|m| m.capacity)
        .sum::<u64>() as f64
        / 1_073_741_824.0;

    // --- canal único ---
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

    // --- XMP / EXPO ---
    let nominal = modules.iter().filter_map(|m| m.speed).max();
    let running = modules.iter().filter_map(|m| m.configured_clock_speed).max();

    if let (Some(nominal), Some(running)) = (nominal, running) {
        // Margem de 1 MHz: o firmware costuma reportar 2667 para um pente de 2666.
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

// ------------------------------------------------------- limites de boot

/// Limites de núcleos e memória gravados na configuração de inicialização.
///
/// Quase sempre é sequela de alguém ter mexido no `msconfig` seguindo tutorial
/// ruim: o Windows passa a usar só parte do processador ou da RAM, para sempre.
/// Os nomes das opções do `bcdedit` são em inglês em qualquer idioma.
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

/// Se alguém forçou o relógio de plataforma (HPET) na configuração de boot.
///
/// É uma das "dicas de FPS" mais repetidas da internet e uma das mais erradas:
/// forçar o HPET obriga o Windows a usar um temporizador mais lento que o
/// escolhido automaticamente, e o efeito comum é engasgo, não ganho. Quando a
/// opção está presente, o certo é remover — não é otimizar, é desfazer estrago.
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
        .map(|line| line.split_whitespace().nth(1).unwrap_or("sim").to_string())
}

pub fn boot_limits() -> Vec<(String, String)> {
    match shell::run("bcdedit", &["/enum", "{current}"]) {
        Ok(output) if output.success => parse_boot_limits(&output.stdout),
        _ => Vec::new(),
    }
}

fn analyze_boot_limits(findings: &mut Vec<FirmwareFinding>) {
    let limits = boot_limits();

    if limits.is_empty() {
        findings.push(FirmwareFinding {
            id: "boot_limits_clear".to_string(),
            title: "Inicialização sem limites artificiais".to_string(),
            measured: "Nenhum limite de núcleos ou memória na configuração de boot.".to_string(),
            advice: String::new(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::None,
        });
        return;
    }

    let described: Vec<String> = limits
        .iter()
        .map(|(key, value)| format!("{} = {}", key, value))
        .collect();

    findings.push(FirmwareFinding {
        id: "boot_limits_present".to_string(),
        title: "Inicialização limitando o hardware".to_string(),
        measured: described.join(", "),
        advice: "O Windows está usando de propósito menos processador ou menos memória \
                 do que você tem. Isso quase sempre é sobra de mexida no msconfig. \
                 A otimização \"Liberar limites de inicialização\" corrige."
            .to_string(),
        severity: FindingSeverity::Critical,
        fix_location: FixLocation::Software,
    });
}

// ------------------------------------------------------------------- VBS

/// Se a virtualização de segurança está em execução.
///
/// `VirtualizationBasedSecurityStatus`: 0 desligada, 1 ativada sem rodar,
/// 2 ativada e rodando.
pub fn vbs_running() -> Option<bool> {
    let script = "(Get-CimInstance -Namespace root\\Microsoft\\Windows\\DeviceGuard \
                  -ClassName Win32_DeviceGuard).VirtualizationBasedSecurityStatus";

    let value: u32 = query_json(script)?.trim().parse().ok()?;
    Some(value == 2)
}

fn analyze_vbs(findings: &mut Vec<FirmwareFinding>) {
    match vbs_running() {
        Some(true) => findings.push(FirmwareFinding {
            id: "vbs_running".to_string(),
            title: "Virtualização de segurança ligada".to_string(),
            measured: "VBS ativa e em execução.".to_string(),
            advice: "Custa desempenho em jogos, mais em processadores de 8ª a 10ª geração. \
                     A otimização \"Desligar virtualização de segurança\" desliga — leia o \
                     aviso de segurança antes, porque você perde proteção real."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Software,
        }),
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

// -------------------------------------------------- estrangulamento térmico

/// Perda de desempenho da CPU sob carga sustentada.
///
/// Não perguntamos ao Windows a frequência: no Windows o valor reportado é quase
/// sempre o nominal, não o real. Medimos a consequência — quanto de trabalho a
/// CPU entrega no fim de dez segundos de carga comparado ao primeiro segundo.
/// Se o trabalho cai, o processador está sendo freado por temperatura ou por
/// limite de energia, e isso se resolve na BIOS ou na refrigeração.
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

/// Queda percentual do fim em relação ao início da carga.
/// Exposto para teste porque é o cálculo que decide o veredito.
pub fn decay_percent(throughput: &[f64]) -> f64 {
    if throughput.len() < 4 {
        return 0.0;
    }

    let first = throughput[0];
    if first <= 0.0 {
        return 0.0;
    }

    // Média das duas últimas fatias, para um soluço isolado no fim não virar
    // diagnóstico de superaquecimento.
    let tail = &throughput[throughput.len() - 2..];
    let last = tail.iter().sum::<f64>() / tail.len() as f64;

    (first - last) / first * 100.0
}

fn analyze_throttling(findings: &mut Vec<FirmwareFinding>) {
    let decay = measure_sustained_decay();

    // 8% é a fronteira: abaixo disso a variação se explica por outros processos
    // disputando a CPU durante a medição.
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
            // Uma CPU que termina mais rápido do que começou não "ganhou força":
            // é variação de medição. Zero é a leitura honesta.
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

// --------------------------------------------------------------- relatório

fn board_name() -> String {
    let script = "$b = Get-CimInstance Win32_BaseBoard; \"$($b.Manufacturer) $($b.Product)\"";
    query_json(script)
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| "placa não identificada".to_string())
}

fn cpu_name() -> String {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_all();

    system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_string())
        .unwrap_or_else(|| "processador não identificado".to_string())
}

/// Análise completa. Leva cerca de 12 segundos por causa da carga sustentada.
pub fn analyze() -> FirmwareReport {
    let mut findings = Vec::new();

    analyze_memory(&mut findings);
    analyze_boot_limits(&mut findings);
    analyze_vbs(&mut findings);
    analyze_throttling(&mut findings);

    // Problemas primeiro, o que está certo depois.
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
        // O erro clássico de montagem: dois pentes lado a lado, no mesmo canal.
        let modules = vec![
            module("Controller0-ChannelA-DIMM0"),
            module("Controller0-ChannelA-DIMM1"),
        ];
        assert_eq!(occupied_channels(&modules), 1);
    }

    #[test]
    fn two_controllers_with_the_same_channel_letter_are_two_channels() {
        // Arranjo comum em placas Intel. Olhar só a letra do canal juntaria os
        // dois e acusaria canal único num PC que está correto — o pior erro
        // possível aqui, porque mandaria o cliente comprar RAM sem precisar.
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
        // Perda de 30%: comportamento de CPU freada por temperatura.
        let throughput = vec![100.0, 95.0, 88.0, 80.0, 72.0, 70.0];
        assert!(decay_percent(&throughput) > 25.0);
    }

    #[test]
    fn a_single_slow_slice_at_the_end_does_not_alone_decide() {
        // Média das duas últimas fatias evita que um soluço isolado vire
        // diagnóstico de superaquecimento.
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
        // Problemas precisam vir antes do que está certo.
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
