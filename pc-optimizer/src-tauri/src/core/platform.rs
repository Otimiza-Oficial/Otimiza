// Platform Detection System
// Detects OS, version, and architecture

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub os_type: String,
    pub arch: String,
    pub version: String,
}

pub struct PlatformDetector;

impl PlatformDetector {
    /// Detecta a plataforma atual do sistema
    pub fn detect() -> Platform {
        match env::consts::OS {
            "windows" => Platform::Windows,
            "linux" => Platform::Linux,
            "macos" => Platform::MacOS,
            _ => Platform::Unknown,
        }
    }

    /// Retorna informações detalhadas da plataforma
    pub fn get_info() -> PlatformInfo {
        PlatformInfo {
            platform: Self::detect(),
            os_type: env::consts::OS.to_string(),
            arch: env::consts::ARCH.to_string(),
            version: Self::get_os_version(),
        }
    }

    /// Obtém a versão do sistema operacional
    fn get_os_version() -> String {
        #[cfg(target_os = "windows")]
        {
            Self::get_windows_version()
        }
        
        #[cfg(target_os = "linux")]
        {
            Self::get_linux_version()
        }
        
        #[cfg(target_os = "macos")]
        {
            Self::get_macos_version()
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            "Unknown".to_string()
        }
    }

    #[cfg(target_os = "windows")]
    fn get_windows_version() -> String {
        use std::process::Command;
        
        // Tenta obter versão via comando systeminfo
        if let Ok(output) = Command::new("cmd")
            .args(&["/C", "ver"])
            .output()
        {
            if let Ok(version) = String::from_utf8(output.stdout) {
                return version.trim().to_string();
            }
        }
        
        "Windows (version unknown)".to_string()
    }

    #[cfg(target_os = "linux")]
    fn get_linux_version() -> String {
        use std::fs;
        
        // Tenta ler /etc/os-release
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    return line
                        .trim_start_matches("PRETTY_NAME=")
                        .trim_matches('"')
                        .to_string();
                }
            }
        }
        
        "Linux (distribution unknown)".to_string()
    }

    #[cfg(target_os = "macos")]
    fn get_macos_version() -> String {
        use std::process::Command;
        
        if let Ok(output) = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
        {
            if let Ok(version) = String::from_utf8(output.stdout) {
                return format!("macOS {}", version.trim());
            }
        }
        
        "macOS (version unknown)".to_string()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = PlatformDetector::detect();
        assert_ne!(platform, Platform::Unknown);
    }

}
