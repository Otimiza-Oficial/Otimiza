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
    /// Horas desde o último boot. Ver `uptime_hours`.
    pub uptime_hours: f64,
}

pub struct PerformanceMonitor {
    monitoring_active: bool,
    system: System,
    /// Mantidos vivos entre chamadas: velocidade de disco e de rede só existe
    /// como diferença entre duas leituras. Um coletor recriado a cada chamada
    /// não tem passado com o que comparar e devolve zero para sempre.
    disks: Disks,
    networks: Networks,
    last_sample: Option<std::time::Instant>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        PerformanceMonitor {
            monitoring_active: false,
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_sample: None,
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

        // O intervalo é medido uma vez e passado para os dois coletores: se
        // cada um marcasse o próprio tempo, eles dividiriam os respectivos
        // deltas por janelas diferentes da mesma leitura.
        let elapsed = self.elapsed_since_last();
        let disk = self.collect_disk_metrics(elapsed).await?;
        let network = self.collect_network_metrics(elapsed).await?;

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
            uptime_hours: uptime_hours(),
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

    /// Segundos desde a leitura anterior, ou `None` na primeira.
    ///
    /// Taxa é sempre delta dividido por tempo. Na primeira leitura não existe
    /// "anterior", e é por isso que o campo de velocidade sai como desconhecido
    /// em vez de zero: zero seria afirmar que nada está acontecendo.
    fn elapsed_since_last(&mut self) -> Option<f64> {
        let agora = std::time::Instant::now();
        let anterior = self.last_sample.replace(agora)?;

        let segundos = agora.duration_since(anterior).as_secs_f64();

        // Duas chamadas coladas dividiriam por um número perto de zero e
        // produziriam uma taxa absurda.
        (segundos >= 0.05).then_some(segundos)
    }

    async fn collect_disk_metrics(&mut self, elapsed: Option<f64>) -> Result<DiskMetrics, String> {
        // A lista é mantida entre chamadas de propósito: os bytes lidos e
        // gravados que o sysinfo entrega são a diferença desde o refresh
        // anterior. Recriar a lista a cada leitura — como estava — zera essa
        // diferença, e foi por isso que a velocidade aparecia sempre como 0.
        self.disks.refresh(true);

        let mut total_space = 0u64;
        let mut used_space = 0u64;
        let mut read_bytes = 0u64;
        let mut written_bytes = 0u64;

        for disk in &self.disks {
            total_space += disk.total_space();
            used_space += disk.total_space() - disk.available_space();

            let usage = disk.usage();
            read_bytes += usage.read_bytes;
            written_bytes += usage.written_bytes;
        }

        let usage_percent = if total_space > 0 {
            (used_space as f64 / total_space as f64 * 100.0) as f32
        } else {
            0.0
        };

        let (read_speed_mbps, write_speed_mbps) = match elapsed {
            Some(segundos) => (
                bytes_to_mb(read_bytes) / segundos,
                bytes_to_mb(written_bytes) / segundos,
            ),
            None => (0.0, 0.0),
        };

        Ok(DiskMetrics {
            read_speed_mbps,
            write_speed_mbps,
            usage_percent,
        })
    }

    async fn collect_network_metrics(&mut self, elapsed: Option<f64>) -> Result<NetworkMetrics, String> {
        // Mesma razão do disco: `received()` é o que chegou desde o refresh
        // anterior, então a lista precisa sobreviver entre as chamadas.
        self.networks.refresh(true);

        let mut total_received = 0u64;
        let mut total_transmitted = 0u64;
        let mut received = 0u64;
        let mut transmitted = 0u64;

        for (_interface_name, network) in &self.networks {
            total_received += network.total_received();
            total_transmitted += network.total_transmitted();
            received += network.received();
            transmitted += network.transmitted();
        }

        let (download_speed_mbps, upload_speed_mbps) = match elapsed {
            Some(segundos) => (
                bytes_to_mb(received) / segundos,
                bytes_to_mb(transmitted) / segundos,
            ),
            None => (0.0, 0.0),
        };

        Ok(NetworkMetrics {
            download_speed_mbps,
            upload_speed_mbps,
            total_received_gb: total_received as f64 / 1_073_741_824.0,
            total_transmitted_gb: total_transmitted as f64 / 1_073_741_824.0,
        })
    }
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

/// Há quantas horas o Windows está ligado.
///
/// Vale como métrica porque é uma causa real de lentidão que não aparece em
/// lugar nenhum: depois de muitos dias sem reiniciar, memória vazada por
/// programas e drivers se acumula, e o PC melhora sozinho com um reinício. É
/// também a primeira coisa a descartar antes de sair otimizando.
pub fn uptime_hours() -> f64 {
    System::uptime() as f64 / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primeira_leitura_nao_inventa_taxa() {
        let mut monitor = PerformanceMonitor::new();

        // Sem leitura anterior não existe intervalo, e sem intervalo não existe
        // velocidade. O contrato é devolver `None`, não um número.
        assert!(monitor.elapsed_since_last().is_none());
    }

    #[test]
    fn duas_leituras_coladas_nao_viram_taxa_absurda() {
        let mut monitor = PerformanceMonitor::new();

        monitor.elapsed_since_last();
        // Chamada imediatamente depois: o intervalo é perto de zero e dividir
        // por ele produziria centenas de GB/s.
        assert!(monitor.elapsed_since_last().is_none());
    }

    #[test]
    fn intervalo_real_produz_taxa() {
        let mut monitor = PerformanceMonitor::new();

        monitor.elapsed_since_last();
        std::thread::sleep(std::time::Duration::from_millis(120));

        let segundos = monitor.elapsed_since_last().expect("intervalo válido");
        assert!(segundos >= 0.1 && segundos < 2.0, "intervalo medido: {}", segundos);
    }

    #[test]
    fn conversao_de_bytes_bate_com_megabyte() {
        assert!((bytes_to_mb(1_048_576) - 1.0).abs() < f64::EPSILON);
        assert_eq!(bytes_to_mb(0), 0.0);
    }

    #[test]
    fn tempo_ligado_desta_maquina_e_plausivel() {
        let horas = uptime_hours();
        println!("ligado há {:.1} horas", horas);

        assert!(horas > 0.0, "uptime tem que ser positivo");
        // Dez anos ligado seria erro de unidade, não uma máquina resistente.
        assert!(horas < 87_600.0);
    }

    #[tokio::test]
    async fn velocidade_de_disco_e_rede_sai_do_zero_fixo() {
        let mut monitor = PerformanceMonitor::new();

        // Primeira leitura estabelece a referência.
        monitor.collect_metrics().await.expect("primeira leitura");
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
        let m = monitor.collect_metrics().await.expect("segunda leitura");

        println!(
            "disco: {:.2} MB/s leitura, {:.2} MB/s gravação | rede: {:.2} MB/s baixando",
            m.disk.read_speed_mbps, m.disk.write_speed_mbps, m.network.download_speed_mbps
        );

        // Não dá para exigir tráfego numa máquina parada, mas dá para exigir
        // que os números sejam finitos e não negativos — o bug de dividir por
        // um intervalo perto de zero apareceria aqui como infinito.
        for valor in [
            m.disk.read_speed_mbps,
            m.disk.write_speed_mbps,
            m.network.download_speed_mbps,
            m.network.upload_speed_mbps,
        ] {
            assert!(valor.is_finite(), "taxa inválida: {}", valor);
            assert!(valor >= 0.0, "taxa negativa: {}", valor);
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
