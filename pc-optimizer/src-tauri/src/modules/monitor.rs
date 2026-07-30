// Performance Monitor
// Real-time system metrics collection

use serde::{Deserialize, Serialize};
use sysinfo::{System, Disks, Networks};

#[derive(Debug, Serialize, Deserialize)]
pub struct CPUMetrics {
    pub overall: f32,      // 0-100%
    pub per_core: Vec<f32>,
    pub temperature: f32,
    pub frequency: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RAMMetrics {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub cached_gb: f64,
    pub usage_percent: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GPUMetrics {
    pub utilization: f32,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub temperature: f32,
    pub fan_speed: f32,
    pub power_draw: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub read_speed_mbps: f64,
    pub write_speed_mbps: f64,
    pub usage_percent: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub download_speed_mbps: f64,
    pub upload_speed_mbps: f64,
    pub total_received_gb: f64,
    pub total_transmitted_gb: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: u64,
    pub cpu: CPUMetrics,
    pub ram: RAMMetrics,
    pub gpu: Option<GPUMetrics>,
    pub disk: DiskMetrics,
    pub network: NetworkMetrics,
}

pub struct PerformanceMonitor {
    monitoring_active: bool,
    system: System,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        PerformanceMonitor {
            monitoring_active: false,
            system: System::new_all(),
        }
    }

    /// Inicia o monitoramento contínuo
    pub fn start_monitoring(&mut self) {
        self.monitoring_active = true;
    }

    /// Para o monitoramento
    pub fn stop_monitoring(&mut self) {
        self.monitoring_active = false;
    }

    /// Coleta snapshot das métricas atuais
    pub async fn collect_metrics(&mut self) -> Result<PerformanceMetrics, String> {
        self.system.refresh_all();
        
        let cpu = self.collect_cpu_metrics().await?;
        let ram = self.collect_ram_metrics().await?;
        let gpu = self.collect_gpu_metrics().await.ok();
        let disk = self.collect_disk_metrics().await?;
        let network = self.collect_network_metrics().await?;

        Ok(PerformanceMetrics {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cpu,
            ram,
            gpu,
            disk,
            network,
        })
    }

    async fn collect_cpu_metrics(&mut self) -> Result<CPUMetrics, String> {
        self.system.refresh_cpu_all();
        
        // Aguarda para medição precisa
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        self.system.refresh_cpu_all();

        let cpus = self.system.cpus();
        let per_core: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();
        let overall = per_core.iter().sum::<f32>() / cpus.len() as f32;

        // Frequência do primeiro core (MHz)
        let frequency = cpus.first()
            .map(|cpu| cpu.frequency() as f32)
            .unwrap_or(0.0);

        Ok(CPUMetrics {
            overall,
            per_core,
            temperature: 0.0, // Placeholder - requer acesso baixo nível
            frequency,
        })
    }

    async fn collect_ram_metrics(&mut self) -> Result<RAMMetrics, String> {
        self.system.refresh_memory();

        let total_bytes = self.system.total_memory();
        let used_bytes = self.system.used_memory();
        let available_bytes = self.system.available_memory();

        let total_gb = total_bytes as f64 / 1_073_741_824.0;
        let used_gb = used_bytes as f64 / 1_073_741_824.0;
        let available_gb = available_bytes as f64 / 1_073_741_824.0;
        let cached_gb = 0.0; // Placeholder

        let usage_percent = (used_bytes as f64 / total_bytes as f64 * 100.0) as f32;

        Ok(RAMMetrics {
            total_gb,
            used_gb,
            available_gb,
            cached_gb,
            usage_percent,
        })
    }

    async fn collect_gpu_metrics(&self) -> Result<GPUMetrics, String> {
        // Placeholder - requer biblioteca específica (NVML, AMD ADL, etc)
        Err("GPU metrics not yet implemented".to_string())
    }

    async fn collect_disk_metrics(&mut self) -> Result<DiskMetrics, String> {
        let disks = Disks::new_with_refreshed_list();
        
        // Calcula uso médio dos discos
        let mut total_space = 0u64;
        let mut used_space = 0u64;

        for disk in &disks {
            total_space += disk.total_space();
            used_space += disk.total_space() - disk.available_space();
        }

        let usage_percent = if total_space > 0 {
            (used_space as f64 / total_space as f64 * 100.0) as f32
        } else {
            0.0
        };

        Ok(DiskMetrics {
            read_speed_mbps: 0.0,  // Requer monitoramento ao longo do tempo
            write_speed_mbps: 0.0, // Requer monitoramento ao longo do tempo
            usage_percent,
        })
    }

    async fn collect_network_metrics(&mut self) -> Result<NetworkMetrics, String> {
        let networks = Networks::new_with_refreshed_list();
        
        let mut total_received = 0u64;
        let mut total_transmitted = 0u64;

        for (_interface_name, network) in &networks {
            total_received += network.total_received();
            total_transmitted += network.total_transmitted();
        }

        let total_received_gb = total_received as f64 / 1_073_741_824.0;
        let total_transmitted_gb = total_transmitted as f64 / 1_073_741_824.0;

        Ok(NetworkMetrics {
            download_speed_mbps: 0.0, // Requer monitoramento ao longo do tempo
            upload_speed_mbps: 0.0,   // Requer monitoramento ao longo do tempo
            total_received_gb,
            total_transmitted_gb,
        })
    }

}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
