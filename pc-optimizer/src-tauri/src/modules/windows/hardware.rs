// Perfil de hardware
//
// A diferença entre um otimizador honesto e uma lista de tweaks copiada da
// internet está aqui: saber COM QUAL máquina se está falando.
//
// Desativar o SysMain é bom em SSD e ruim em HD mecânico. Desativar a compressão
// de memória é bom com RAM sobrando e ruim com 8 GB. Um produto que oferece as
// duas coisas para todo mundo está chutando; este lê o hardware antes de abrir a
// boca.

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
    /// Tipo de mídia do disco onde o Windows está instalado.
    pub system_storage: StorageKind,
    pub total_ram_gb: f64,
    pub logical_cores: usize,
}

static PROFILE: OnceLock<HardwareProfile> = OnceLock::new();

/// Perfil da máquina, detectado uma vez e reaproveitado.
/// A detecção do disco chama o PowerShell e leva centenas de milissegundos —
/// repetir isso a cada listagem deixaria a interface lenta sem motivo.
pub fn profile() -> &'static HardwareProfile {
    PROFILE.get_or_init(detect)
}

fn detect() -> HardwareProfile {
    let mut system = sysinfo::System::new();
    system.refresh_memory();

    HardwareProfile {
        system_storage: detect_system_storage(),
        total_ram_gb: system.total_memory() as f64 / 1_073_741_824.0,
        logical_cores: num_cpus::get(),
    }
}

/// Descobre se o disco do sistema é SSD ou mecânico.
///
/// `MediaType` do PowerShell devolve as constantes "SSD" e "HDD" em qualquer
/// idioma do Windows — ao contrário do texto de quase todo comando do sistema.
fn detect_system_storage() -> StorageKind {
    let script = "$n = (Get-Partition -DriveLetter C | Get-Disk).Number; \
                  (Get-PhysicalDisk | Where-Object DeviceId -eq $n).MediaType";

    let output = match shell::run("powershell", &["-NoProfile", "-Command", script]) {
        Ok(output) if output.success => output.stdout,
        _ => return StorageKind::Unknown,
    };

    parse_media_type(&output)
}

/// Exposto para teste: o parsing é a parte que pode quebrar em máquinas atípicas.
pub fn parse_media_type(output: &str) -> StorageKind {
    let value = output.trim().to_uppercase();

    if value.contains("SSD") {
        StorageKind::Ssd
    } else if value.contains("HDD") {
        StorageKind::Hdd
    } else {
        // "Unspecified" é comum em NVMe atrás de certos controladores e em
        // máquinas virtuais. Fingir que é SSD seria um palpite; não é.
        StorageKind::Unknown
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
