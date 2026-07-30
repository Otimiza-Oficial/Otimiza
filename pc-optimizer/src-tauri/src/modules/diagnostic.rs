// Diagnostic Engine
// Analyzes system performance and identifies bottlenecks

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{System, Disks};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentType {
    CPU,
    RAM,
    Disk,
    GPU,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,  // Impacto severo no desempenho
    High,      // Impacto significativo
    Medium,    // Impacto moderado
    Low,       // Impacto mínimo
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bottleneck {
    pub component: ComponentType,
    pub severity: Severity,
    pub description: String,
    pub impact: String,
    pub suggested_fix: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub cpu_name: String,
    pub cpu_cores: u32,
    pub total_ram_gb: f64,
    pub gpu_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub timestamp: u64,
    pub system_info: SystemInfo,
    pub bottlenecks: Vec<Bottleneck>,
    pub health_score: u8,  // 0-100
    pub recommendations: Vec<String>,
}

pub struct DiagnosticEngine {
    system: System,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        DiagnosticEngine {
            system: System::new_all(),
        }
    }

    /// Executa análise completa do sistema
    pub async fn run_full_diagnostic(&mut self) -> Result<DiagnosticReport, String> {
        // Atualiza informações do sistema
        self.system.refresh_all();
        
        let system_info = self.collect_system_info().await?;
        let bottlenecks = self.identify_bottlenecks().await?;
        let health_score = self.calculate_health_score(&bottlenecks);
        let recommendations = self.generate_recommendations(&bottlenecks);

        Ok(DiagnosticReport {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            system_info,
            bottlenecks,
            health_score,
            recommendations,
        })
    }

    /// Coleta informações do sistema
    async fn collect_system_info(&mut self) -> Result<SystemInfo, String> {
        self.system.refresh_all();
        
        let cpu_name = self.system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let total_ram_bytes = self.system.total_memory();
        let total_ram_gb = total_ram_bytes as f64 / 1_073_741_824.0; // Bytes to GB

        Ok(SystemInfo {
            os: std::env::consts::OS.to_string(),
            os_version: self.get_os_version(),
            cpu_name,
            cpu_cores: self.system.cpus().len() as u32,
            total_ram_gb,
            gpu_name: self.detect_gpu(),
        })
    }

    /// Obtém versão do OS
    fn get_os_version(&self) -> String {
        System::name()
            .and_then(|name| {
                System::os_version().map(|version| format!("{} {}", name, version))
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Detecta GPU (específico por plataforma)
    fn detect_gpu(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            self.detect_gpu_windows()
        }
        
        #[cfg(target_os = "linux")]
        {
            self.detect_gpu_linux()
        }
        
        #[cfg(target_os = "macos")]
        {
            self.detect_gpu_macos()
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            "GPU Detection Not Supported".to_string()
        }
    }

    #[cfg(target_os = "windows")]
    fn detect_gpu_windows(&self) -> String {
        use std::process::Command;
        
        // Tenta usar WMIC para obter informações da GPU
        if let Ok(output) = Command::new("wmic")
            .args(&["path", "win32_VideoController", "get", "name"])
            .output()
        {
            if let Ok(result) = String::from_utf8(output.stdout) {
                let lines: Vec<&str> = result.lines().collect();
                if lines.len() > 1 {
                    return lines[1].trim().to_string();
                }
            }
        }
        
        "Unknown GPU (Windows)".to_string()
    }

    #[cfg(target_os = "linux")]
    fn detect_gpu_linux(&self) -> String {
        use std::process::Command;
        
        // Tenta usar lspci para detectar GPU
        if let Ok(output) = Command::new("lspci")
            .output()
        {
            if let Ok(result) = String::from_utf8(output.stdout) {
                for line in result.lines() {
                    if line.contains("VGA") || line.contains("3D") {
                        if let Some(gpu_info) = line.split(':').nth(2) {
                            return gpu_info.trim().to_string();
                        }
                    }
                }
            }
        }
        
        "Unknown GPU (Linux)".to_string()
    }

    #[cfg(target_os = "macos")]
    fn detect_gpu_macos(&self) -> String {
        use std::process::Command;
        
        // Tenta usar system_profiler para obter informações da GPU
        if let Ok(output) = Command::new("system_profiler")
            .args(&["SPDisplaysDataType"])
            .output()
        {
            if let Ok(result) = String::from_utf8(output.stdout) {
                for line in result.lines() {
                    if line.trim().starts_with("Chipset Model:") {
                        if let Some(gpu_name) = line.split(':').nth(1) {
                            return gpu_name.trim().to_string();
                        }
                    }
                }
            }
        }
        
        "Unknown GPU (macOS)".to_string()
    }

    /// Analisa uso de CPU e identifica gargalos
    async fn analyze_cpu(&mut self) -> Option<Bottleneck> {
        self.system.refresh_cpu_all();
        
        // Aguarda um pouco para obter medições precisas
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        self.system.refresh_cpu_all();

        let cpu_usage: f32 = self.system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>() / self.system.cpus().len() as f32;

        if cpu_usage > 90.0 {
            Some(Bottleneck {
                component: ComponentType::CPU,
                severity: Severity::Critical,
                description: format!("CPU usage is critically high: {:.1}%", cpu_usage),
                impact: "Severe performance degradation, system may freeze".to_string(),
                suggested_fix: "Close unnecessary programs, check for malware, consider CPU upgrade".to_string(),
            })
        } else if cpu_usage > 75.0 {
            Some(Bottleneck {
                component: ComponentType::CPU,
                severity: Severity::High,
                description: format!("CPU usage is high: {:.1}%", cpu_usage),
                impact: "Noticeable slowdowns, reduced responsiveness".to_string(),
                suggested_fix: "Identify and close CPU-intensive programs".to_string(),
            })
        } else if cpu_usage > 60.0 {
            Some(Bottleneck {
                component: ComponentType::CPU,
                severity: Severity::Medium,
                description: format!("CPU usage is elevated: {:.1}%", cpu_usage),
                impact: "Minor performance impact during intensive tasks".to_string(),
                suggested_fix: "Monitor background processes".to_string(),
            })
        } else {
            None
        }
    }

    /// Analisa uso de RAM e identifica gargalos
    async fn analyze_ram(&mut self) -> Option<Bottleneck> {
        self.system.refresh_memory();

        let total = self.system.total_memory() as f64;
        let used = self.system.used_memory() as f64;
        let usage_percent = (used / total) * 100.0;

        if usage_percent > 95.0 {
            Some(Bottleneck {
                component: ComponentType::RAM,
                severity: Severity::Critical,
                description: format!("RAM usage is critically high: {:.1}%", usage_percent),
                impact: "System may crash, heavy disk swapping".to_string(),
                suggested_fix: "Close programs, add more RAM, enable memory compression".to_string(),
            })
        } else if usage_percent > 85.0 {
            Some(Bottleneck {
                component: ComponentType::RAM,
                severity: Severity::High,
                description: format!("RAM usage is high: {:.1}%", usage_percent),
                impact: "Significant slowdowns due to disk paging".to_string(),
                suggested_fix: "Close memory-intensive applications, consider RAM upgrade".to_string(),
            })
        } else if usage_percent > 70.0 {
            Some(Bottleneck {
                component: ComponentType::RAM,
                severity: Severity::Medium,
                description: format!("RAM usage is elevated: {:.1}%", usage_percent),
                impact: "Reduced performance in memory-intensive tasks".to_string(),
                suggested_fix: "Monitor memory usage, close unnecessary programs".to_string(),
            })
        } else {
            None
        }
    }

    /// Analisa discos e identifica gargalos
    async fn analyze_disk(&mut self) -> Option<Bottleneck> {
        let disks = Disks::new_with_refreshed_list();
        
        for disk in &disks {
            let total = disk.total_space() as f64;
            let available = disk.available_space() as f64;
            let used_percent = ((total - available) / total) * 100.0;

            if used_percent > 95.0 {
                return Some(Bottleneck {
                    component: ComponentType::Disk,
                    severity: Severity::Critical,
                    description: format!("Disk space critically low: {:.1}% full", used_percent),
                    impact: "System instability, cannot save files".to_string(),
                    suggested_fix: "Free up disk space immediately, run disk cleanup".to_string(),
                });
            } else if used_percent > 85.0 {
                return Some(Bottleneck {
                    component: ComponentType::Disk,
                    severity: Severity::High,
                    description: format!("Disk space low: {:.1}% full", used_percent),
                    impact: "Performance degradation, limited storage".to_string(),
                    suggested_fix: "Delete temporary files, uninstall unused programs".to_string(),
                });
            }
        }

        None
    }

    /// Identifica gargalos de desempenho
    async fn identify_bottlenecks(&mut self) -> Result<Vec<Bottleneck>, String> {
        let mut bottlenecks = Vec::new();
        
        // Análise de CPU
        if let Some(cpu_bottleneck) = self.analyze_cpu().await {
            bottlenecks.push(cpu_bottleneck);
        }

        // Análise de RAM
        if let Some(ram_bottleneck) = self.analyze_ram().await {
            bottlenecks.push(ram_bottleneck);
        }

        // Análise de Disk
        if let Some(disk_bottleneck) = self.analyze_disk().await {
            bottlenecks.push(disk_bottleneck);
        }

        // GPU analysis - placeholder
        // Network analysis - placeholder

        Ok(bottlenecks)
    }

    /// Calcula pontuação geral de saúde do sistema (0-100)
    fn calculate_health_score(&self, bottlenecks: &[Bottleneck]) -> u8 {
        if bottlenecks.is_empty() {
            return 100;
        }

        let penalty: u8 = bottlenecks
            .iter()
            .map(|b| match b.severity {
                Severity::Critical => 25,
                Severity::High => 15,
                Severity::Medium => 8,
                Severity::Low => 3,
            })
            .sum();

        100u8.saturating_sub(penalty)
    }

    /// Gera recomendações baseadas nos gargalos identificados
    fn generate_recommendations(&self, bottlenecks: &[Bottleneck]) -> Vec<String> {
        bottlenecks
            .iter()
            .map(|b| b.suggested_fix.clone())
            .collect()
    }
}

impl Default for DiagnosticEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_calculation() {
        let engine = DiagnosticEngine::new();
        
        let no_bottlenecks = vec![];
        assert_eq!(engine.calculate_health_score(&no_bottlenecks), 100);

        let critical_bottleneck = vec![Bottleneck {
            component: ComponentType::CPU,
            severity: Severity::Critical,
            description: "Test".to_string(),
            impact: "Test".to_string(),
            suggested_fix: "Test".to_string(),
        }];
        assert_eq!(engine.calculate_health_score(&critical_bottleneck), 75);
    }
}
