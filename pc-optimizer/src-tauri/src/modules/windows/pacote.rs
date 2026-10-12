// Ajustes dos dois pacotes do dono (11/10/2026) que não são uma chave de registro só: elemento do `bcdedit`,
// parâmetro de TCP, offload global, recurso de cada placa de rede e o `DisableDynamicPstate` da placa NVIDIA.
//
// O desfazer lê o nome e o valor anterior do histórico (`changes.json`), que é um arquivo do usuário, e o Otimiza roda
// como administrador. Por isso nada do histórico vira comando sem passar pelas listas abaixo e por `palavra_segura`.

use super::{registry, shell};
use crate::modules::changelog::{ChangeRecord, PreviousValue};

pub const BOOT_PERMITIDOS: &[&str] = &["disabledynamictick", "useplatformtick", "tscsyncpolicy"];

pub const TCP_PERMITIDOS: &[&str] = &[
    "CongestionProvider",
    "InitialRto",
    "MinRto",
    "MaxSynRetransmissions",
    "NonSackRttResiliency",
    "Timestamps",
    "EcnCapability",
    "ScalingHeuristics",
    "AutoTuningLevelLocal",
];

pub const OFFLOAD_PERMITIDOS: &[&str] = &["ReceiveSideScaling", "ReceiveSegmentCoalescing"];

pub const ADAPTADOR_PERMITIDOS: &[&str] = &["Lso", "ChecksumOffload"];

const CLASSE_DE_VIDEO: &str =
    r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
const PSTATE: &str = "DisableDynamicPstate";

/// Só letras e números: é o que vai para o `bcdedit` e para o PowerShell.
pub fn palavra_segura(texto: &str) -> bool {
    !texto.is_empty() && texto.len() <= 32 && texto.chars().all(|c| c.is_ascii_alphanumeric())
}

fn conferir(lista: &[&str], nome: &str, valor: &str) -> Result<(), String> {
    if !lista.contains(&nome) {
        return Err(format!("`{}` não é um ajuste que o Otimiza faz.", nome));
    }
    if !palavra_segura(valor) {
        return Err(format!("O valor de `{}` no histórico não é válido.", nome));
    }
    Ok(())
}

// ---------- bcdedit ----------

/// **Pura.** O valor de um elemento no `bcdedit /enum {current}`: `Some(None)` quando o elemento não está lá
/// (padrão do Windows). Os nomes saem em inglês em qualquer idioma; os valores booleanos são normalizados.
pub fn valor_no_boot(saida: &str, nome: &str) -> Option<String> {
    saida.lines().find_map(|linha| {
        let mut partes = linha.split_whitespace();
        let chave = partes.next()?.to_lowercase();
        if chave != nome {
            return None;
        }
        let valor = partes.next()?.to_lowercase();
        Some(
            super::firmware::palavra_chave_booleana(&valor)
                .map(str::to_string)
                .unwrap_or(valor),
        )
    })
}

/// `None`: não deu para ler (sem administrador o `bcdedit` não responde).
pub fn ler_boot(nome: &str) -> Option<Option<String>> {
    let saida = shell::run("bcdedit", &["/enum", "{current}"]).ok()?;
    if !saida.success {
        return None;
    }
    Some(valor_no_boot(&saida.stdout, nome))
}

pub fn definir_boot(nome: &str, valor: &str) -> Result<(), String> {
    conferir(BOOT_PERMITIDOS, nome, valor)?;
    shell::run_checked("bcdedit", &["/set", "{current}", nome, valor]).map(|_| ())
}

pub fn desfazer_boot(nome: &str, anterior: Option<&str>) -> Result<(), String> {
    match anterior {
        Some(valor) => definir_boot(nome, valor),
        None => {
            conferir(BOOT_PERMITIDOS, nome, "x")?;
            shell::run_checked("bcdedit", &["/deletevalue", "{current}", nome]).map(|_| ())
        }
    }
}

// ---------- TCP e offload ----------

fn ler_powershell(script: &str) -> Option<String> {
    let saida = shell::powershell(script).ok()?;
    if !saida.success {
        return None;
    }
    let valor = saida.stdout.trim().to_string();
    (!valor.is_empty()).then_some(valor)
}

pub fn ler_tcp(parametro: &str) -> Option<String> {
    if !TCP_PERMITIDOS.contains(&parametro) {
        return None;
    }
    ler_powershell(&format!(
        "(Get-NetTCPSetting -SettingName Internet -ErrorAction Stop | Select-Object -First 1).{}",
        parametro
    ))
}

pub fn definir_tcp(parametro: &str, valor: &str) -> Result<(), String> {
    conferir(TCP_PERMITIDOS, parametro, valor)?;
    shell::powershell_checked(&format!(
        "Set-NetTCPSetting -SettingName Internet -{} {} -ErrorAction Stop",
        parametro, valor
    ))
    .map(|_| ())
}

pub fn ler_offload(parametro: &str) -> Option<String> {
    if !OFFLOAD_PERMITIDOS.contains(&parametro) {
        return None;
    }
    ler_powershell(&format!(
        "(Get-NetOffloadGlobalSetting -ErrorAction Stop).{}",
        parametro
    ))
}

pub fn definir_offload(parametro: &str, valor: &str) -> Result<(), String> {
    conferir(OFFLOAD_PERMITIDOS, parametro, valor)?;
    shell::powershell_checked(&format!(
        "Set-NetOffloadGlobalSetting -{} {} -ErrorAction Stop",
        parametro, valor
    ))
    .map(|_| ())
}

// ---------- recurso de cada placa de rede ----------

/// O nome da placa vem do Windows (e no desfazer, do histórico): dentro de aspas simples do PowerShell só a aspa
/// simples precisa de escape. Comparado com `-eq`, e não passado em `-Name`, que aceita curinga.
fn aspas(nome: &str) -> String {
    format!("'{}'", nome.replace('\'', "''"))
}

/// As placas com o recurso ligado (em qualquer protocolo). `Err`: não deu para ler.
pub fn placas_com(recurso: &str) -> Result<Vec<String>, String> {
    let filtro = match recurso {
        "Lso" => "Get-NetAdapterLso -ErrorAction Stop | Where-Object { $_.IPv4Enabled -or $_.IPv6Enabled -or $_.V1IPv4Enabled }",
        "ChecksumOffload" => "Get-NetAdapterChecksumOffload -ErrorAction Stop | Where-Object { \
             \"$($_.IpIPv4Enabled)$($_.TcpIPv4Enabled)$($_.TcpIPv6Enabled)$($_.UdpIPv4Enabled)$($_.UdpIPv6Enabled)\" \
             -ne 'DisabledDisabledDisabledDisabledDisabled' }",
        _ => return Err(format!("`{}` não é um recurso que o Otimiza muda.", recurso)),
    };
    let saida = shell::powershell(&format!("{} | ForEach-Object {{ $_.Name }}", filtro))?;
    if !saida.success {
        return Err(format!(
            "O Windows não informou o {} das placas de rede.",
            recurso
        ));
    }
    Ok(saida
        .stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn mudar_na_placa(recurso: &str, placa: &str, ligar: bool) -> Result<(), String> {
    if !ADAPTADOR_PERMITIDOS.contains(&recurso) {
        return Err(format!("`{}` não é um recurso que o Otimiza muda.", recurso));
    }
    let verbo = if ligar { "Enable" } else { "Disable" };
    shell::powershell_checked(&format!(
        "Get-NetAdapter -ErrorAction Stop | Where-Object {{ $_.Name -eq {} }} | {}-NetAdapter{} -ErrorAction Stop",
        aspas(placa),
        verbo,
        recurso
    ))
    .map(|_| ())
}

// ---------- DisableDynamicPstate ----------

/// As pastas da classe de vídeo que são placas NVIDIA. `Err`: a classe não pôde ser lida.
pub fn placas_nvidia() -> Result<Vec<String>, String> {
    let mut placas = Vec::new();
    for sub in registry::subkeys("HKLM", CLASSE_DE_VIDEO)? {
        // `Configuration` e `Properties` não são placas.
        if sub.len() != 4 || !sub.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let caminho = format!(r"{}\{}", CLASSE_DE_VIDEO, sub);
        let fabricante = registry::read_text("HKLM", &caminho, "ProviderName")
            .ok()
            .flatten()
            .unwrap_or_default();
        if fabricante.to_lowercase().contains("nvidia") {
            placas.push(caminho);
        }
    }
    Ok(placas)
}

/// `None`: sem placa NVIDIA (não se aplica). `Some(true)`: todas já no clock fixo.
pub fn clock_maximo_ligado() -> Result<Option<bool>, String> {
    let placas = placas_nvidia()?;
    if placas.is_empty() {
        return Ok(None);
    }
    Ok(Some(placas.iter().all(|caminho| {
        matches!(registry::read("HKLM", caminho, PSTATE), Ok(PreviousValue::Dword(1)))
    })))
}

/// Grava em cada placa NVIDIA e deixa um registro comum de registro por placa: o desfazer é o de qualquer chave.
pub fn ligar_clock_maximo(changes: &mut Vec<ChangeRecord>) -> Result<usize, String> {
    let mut mudadas = 0;
    for caminho in placas_nvidia()? {
        let anterior = registry::read("HKLM", &caminho, PSTATE)?;
        if anterior == PreviousValue::Dword(1) {
            continue;
        }
        registry::set_dword("HKLM", &caminho, PSTATE, 1)?;
        changes.push(ChangeRecord::RegistryValue {
            hive: "HKLM".to_string(),
            path: caminho,
            name: PSTATE.to_string(),
            previous: anterior,
        });
        mudadas += 1;
    }
    Ok(mudadas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_letras_e_numeros_viram_comando() {
        assert!(palavra_segura("Disabled"));
        assert!(palavra_segura("2000"));
        assert!(palavra_segura("CTCP"));
        assert!(!palavra_segura(""));
        assert!(!palavra_segura("x; Remove-Item"));
        assert!(!palavra_segura("$(calc)"));
        assert!(!palavra_segura("a b"));
    }

    #[test]
    fn nome_fora_da_lista_e_recusado_mesmo_com_valor_limpo() {
        assert!(definir_boot("nx", "AlwaysOff").is_err());
        assert!(definir_tcp("Comando", "1").is_err());
        assert!(definir_offload("Chimney", "Disabled").is_err());
        assert!(mudar_na_placa("Rss", "Ethernet", false).is_err());
        assert!(desfazer_boot("hypervisorlaunchtype", None).is_err());
    }

    #[test]
    fn le_o_elemento_do_boot_e_normaliza_o_sim() {
        let saida = "Identificador           {current}\n\
                     nx                      OptIn\n\
                     disabledynamictick      Sim\n\
                     tscsyncpolicy           Legacy\n";
        assert_eq!(valor_no_boot(saida, "disabledynamictick").as_deref(), Some("yes"));
        assert_eq!(valor_no_boot(saida, "tscsyncpolicy").as_deref(), Some("legacy"));
        assert_eq!(valor_no_boot(saida, "useplatformtick"), None);
    }

    /// Só lê. `cargo test --lib pacote::tests::mostra_o_estado_deste_pc -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn mostra_o_estado_deste_pc() {
        for p in TCP_PERMITIDOS {
            println!("TCP {p} = {:?}", ler_tcp(p));
        }
        for p in OFFLOAD_PERMITIDOS {
            println!("offload {p} = {:?}", ler_offload(p));
        }
        for r in ADAPTADOR_PERMITIDOS {
            println!("{r} ligado em {:?}", placas_com(r));
        }
        println!("NVIDIA: {:?} -> clock fixo {:?}", placas_nvidia(), clock_maximo_ligado());
        for b in BOOT_PERMITIDOS {
            println!("boot {b} = {:?}", ler_boot(b));
        }
    }

    #[test]
    fn aspas_simples_do_nome_da_placa_nao_fecham_o_texto() {
        assert_eq!(aspas("Rede do João"), "'Rede do João'");
        assert_eq!(aspas("x' ; calc ; '"), "'x'' ; calc ; '''");
    }
}
