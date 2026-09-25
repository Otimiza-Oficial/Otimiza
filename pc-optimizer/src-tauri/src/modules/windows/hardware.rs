// Perfil de hardware: o mesmo ajuste é bom numa máquina e ruim noutra (SysMain em SSD ou HD, compressão de
// memória com RAM sobrando ou 8 GB), então se lê o hardware antes de oferecer.

use super::shell;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Ssd,
    Hdd,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct HardwareProfile {
    pub system_storage: StorageKind,
    pub total_ram_gb: f64,
    pub logical_cores: usize,
    pub cpu_name: String,
    pub gpu_name: String,
}

static PROFILE: OnceLock<HardwareProfile> = OnceLock::new();

/// Detectado uma vez: o disco passa pelo PowerShell e leva centenas de milissegundos.
pub fn profile() -> &'static HardwareProfile {
    PROFILE.get_or_init(detect)
}

fn detect() -> HardwareProfile {
    let mut system = sysinfo::System::new();
    system.refresh_memory();

    // NÃO `refresh_cpu_all()`: ele dorme entre duas amostras de uso (963 ms medidos) e daqui só sai o nome, que
    // vem na primeira leitura. Há teste garantindo que a leitura barata devolve o mesmo nome.
    system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing());

    HardwareProfile {
        system_storage: detect_system_storage(),
        total_ram_gb: system.total_memory() as f64 / 1_073_741_824.0,
        logical_cores: num_cpus::get(),
        cpu_name: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|nome| !nome.is_empty())
            .unwrap_or_else(|| "processador não identificado".to_string()),
        gpu_name: detect_gpu(),
    }
}

/// O filtro por `AdapterRAM` > 0 descarta adaptadores virtuais sem lista de nomes, que envelheceria.
fn detect_gpu() -> String {
    let script = "@(Get-CimInstance Win32_VideoController | \
                  Where-Object { $_.AdapterRAM -gt 0 } | \
                  Select-Object -ExpandProperty Name) -join ' + '";

    match shell::powershell(script) {
        Ok(output) if output.success && !output.stdout.trim().is_empty() => {
            output.stdout.trim().to_string()
        }
        _ => "placa de vídeo não identificada".to_string(),
    }
}

/// Windows em `D:` existe (dois sistemas, partição de recuperação na frente); com `C` cravado, SSD ou mecânico
/// saía do disco errado, e isso liga ou desliga o SysMain. Só uma letra A-Z passa: vai para dentro de um script.
fn letra_do_sistema() -> char {
    std::env::var("SystemDrive")
        .ok()
        .and_then(|d| d.chars().next())
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| c.is_ascii_alphabetic())
        .unwrap_or('C')
}

/// `MediaType` vem "SSD"/"HDD" em qualquer idioma.
fn detect_system_storage() -> StorageKind {
    #[cfg(target_os = "windows")]
    if let Some(conhecido) = detect_system_storage_rapido() {
        return conhecido;
    }

    let script = format!(
        "$n = (Get-Partition -DriveLetter {} | Get-Disk).Number; \
         (Get-PhysicalDisk | Where-Object DeviceId -eq $n).MediaType",
        letra_do_sistema()
    );

    let output = match shell::powershell(&script) {
        Ok(output) if output.success => output.stdout,
        _ => return StorageKind::Unknown,
    };

    parse_media_type(&output)
}

/// Direto pelo WMI, sem carregar o módulo `Storage`: 79 ms contra 1571-3581 ms. Aponta para o disco do SISTEMA,
/// não o primeiro da lista (seria o pendrive). `None` cai no caminho antigo.
#[cfg(target_os = "windows")]
fn detect_system_storage_rapido() -> Option<StorageKind> {
    let script = format!(
        "$ns = 'root\\Microsoft\\Windows\\Storage'; \
         $n = (Get-CimInstance -Namespace $ns -ClassName MSFT_Partition | \
               Where-Object DriveLetter -eq '{}').DiskNumber; \
         (Get-CimInstance -Namespace $ns -ClassName MSFT_PhysicalDisk | \
          Where-Object DeviceId -eq \"$n\").MediaType",
        letra_do_sistema()
    );

    let saida = shell::powershell(&script).ok()?;

    if !saida.success {
        return None;
    }

    match parse_media_type(&saida.stdout) {
        StorageKind::Unknown => None,
        conhecido => Some(conhecido),
    }
}

/// `2700` é WDDM 2.7. `None` é "não deu para ler", NÃO "é antiga". Lido de `FeatureSetUsage` (número, não
/// texto traduzido). Conferido: GTX 1650, Windows 10 19045 = 2700.
pub fn wddm_version() -> Option<u32> {
    use crate::modules::changelog::PreviousValue;

    match super::registry::read(
        "HKLM",
        r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers\FeatureSetUsage",
        "WddmVersion_Max",
    ) {
        Ok(PreviousValue::Dword(v)) => Some(v),
        _ => None,
    }
}

/// Não ler responde `false`: o recurso entra no registro sem erro e não vale nada, então oferecer no escuro é
/// prometer o que não se confere.
pub fn alcanca_wddm(lido: Option<u32>, minimo: u32) -> bool {
    matches!(lido, Some(v) if v >= minimo)
}

#[cfg(test)]
mod testes_do_wddm {
    use super::alcanca_wddm;

    #[test]
    fn wddm_igual_ou_mais_novo_passa() {
        assert!(alcanca_wddm(Some(2700), 2700));
        assert!(alcanca_wddm(Some(3000), 2700));
    }

    #[test]
    fn wddm_mais_antigo_nao_passa() {
        assert!(!alcanca_wddm(Some(2600), 2700));
        assert!(!alcanca_wddm(Some(0), 2700));
    }

    #[test]
    fn nao_conseguir_ler_nao_libera_o_ajuste() {
        // O agendamento por hardware grava sem erro mesmo onde o Windows o ignora, e a releitura não desmente.
        assert!(!alcanca_wddm(None, 2700));
    }
}

pub fn parse_media_type(output: &str) -> StorageKind {
    let value = output.trim().to_uppercase();

    // `MediaType` cru da `MSFT_PhysicalDisk` é número: 3 = HDD, 4 = SSD; o resto é desconhecido.
    match value.as_str() {
        "3" => return StorageKind::Hdd,
        "4" => return StorageKind::Ssd,
        _ => {}
    }

    if value.contains("SSD") {
        StorageKind::Ssd
    } else if value.contains("HDD") {
        StorageKind::Hdd
    } else {
        // "Unspecified" é comum em NVMe atrás de controlador e em máquina virtual: fingir SSD seria palpite.
        StorageKind::Unknown
    }
}

#[cfg(test)]
mod tests_1_7 {
    use super::*;

    /// Prova que a leitura barata devolve o MESMO nome que `refresh_cpu_all()` (~960 ms).
    #[test]
    fn o_nome_da_cpu_e_o_mesmo_sem_amostrar_uso() {
        use sysinfo::{CpuRefreshKind, System};

        let caro = {
            let mut s = System::new();
            s.refresh_cpu_all();
            s.cpus().first().map(|c| c.brand().trim().to_string())
        };

        let barato = {
            let mut s = System::new();
            s.refresh_cpu_specifics(CpuRefreshKind::nothing());
            s.cpus().first().map(|c| c.brand().trim().to_string())
        };

        assert_eq!(
            caro, barato,
            "a leitura barata mudou o nome do processador — a troca não é segura"
        );
    }

    /// Sem mapear o número, `4` não casaria com "SSD" e o SysMain sumiria de toda máquina com SSD.
    #[test]
    fn o_numero_do_wmi_e_traduzido_como_o_texto() {
        assert_eq!(parse_media_type("4"), StorageKind::Ssd);
        assert_eq!(parse_media_type("3"), StorageKind::Hdd);
    }

    #[test]
    fn o_texto_antigo_continua_valendo() {
        assert_eq!(parse_media_type("SSD"), StorageKind::Ssd);
        assert_eq!(parse_media_type("HDD"), StorageKind::Hdd);
        assert_eq!(parse_media_type(" ssd \r\n"), StorageKind::Ssd);
    }

    #[test]
    fn numero_desconhecido_nao_vira_palpite() {
        assert_eq!(parse_media_type("0"), StorageKind::Unknown);
        assert_eq!(parse_media_type("5"), StorageKind::Unknown);
        assert_eq!(parse_media_type(""), StorageKind::Unknown);
        assert_eq!(parse_media_type("Unspecified"), StorageKind::Unknown);
    }

    #[test]
    fn o_perfil_continua_identificando_esta_maquina() {
        let p = profile();

        assert!(!p.cpu_name.is_empty());
        assert!(p.logical_cores >= 1);
        assert!(p.total_ram_gb > 0.0);
    }
}

#[cfg(test)]
mod medicao_de_tempo {
    //! Abre por etapa o tempo de `hardware::profile()` na abertura (a primeira a chamar paga a detecção).
    //! `cargo test --lib -- --ignored --nocapture onde_vai_o_tempo_do_perfil`

    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn onde_vai_o_tempo_do_perfil() {
        let cronometrar = |nome: &str, f: &dyn Fn()| {
            let inicio = Instant::now();
            f();
            println!("  {:<26} {:>6} ms", nome, inicio.elapsed().as_millis());
        };

        println!("\nETAPAS DO PERFIL DE HARDWARE\n");

        cronometrar("detect_system_storage", &|| {
            let _ = detect_system_storage();
        });

        cronometrar("detect_gpu", &|| {
            let _ = detect_gpu();
        });

        cronometrar("sysinfo: memória", &|| {
            let mut s = sysinfo::System::new();
            s.refresh_memory();
        });

        // Os dois lado a lado: a primeira versão cronometrava o código velho e parecia conserto que não funcionou.
        cronometrar("cpu: refresh_cpu_all (antigo)", &|| {
            let mut s = sysinfo::System::new();
            s.refresh_cpu_all();
        });

        cronometrar("cpu: seletivo (em uso)", &|| {
            let mut s = sysinfo::System::new();
            s.refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing());
        });

        cronometrar("num_cpus", &|| {
            let _ = num_cpus::get();
        });

        let inicio = Instant::now();
        let p = profile();
        println!(
            "\n  profile() com cache quente {:>6} ms",
            inicio.elapsed().as_millis()
        );
        println!("  ({} · {:.1} GB · {} núcleos)\n", p.cpu_name, p.total_ram_gb, p.logical_cores);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_media_type_constants() {
        assert_eq!(parse_media_type("SSD\r\n"), StorageKind::Ssd);
        assert_eq!(parse_media_type("HDD\r\n"), StorageKind::Hdd);
    }

    #[test]
    fn unknown_media_type_is_not_guessed_as_ssd() {
        assert_eq!(parse_media_type("Unspecified\r\n"), StorageKind::Unknown);
        assert_eq!(parse_media_type(""), StorageKind::Unknown);
    }

    #[test]
    fn detects_this_machine() {
        let profile = profile();
        println!("{:?}", profile);

        assert!(profile.total_ram_gb > 0.5);
        assert!(profile.logical_cores >= 1);
    }
}
