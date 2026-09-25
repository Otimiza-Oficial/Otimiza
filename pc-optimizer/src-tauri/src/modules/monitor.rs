// Coleta contínua da tela principal, publicada também pelo contrato de `telemetry.rs`: o que não foi medido chega
// como `null`, não zero. sysinfo: CPU, memória, volumes, rede, uptime. PDH: clock reportado e EFETIVO, limites de
// firmware, `% Performance Limit`, núcleos estacionados (o amostrador do motor de energia), e placa, VRAM e disco
// (`windows::placa`). Temperatura é ESTIMATED (zona ACPI, descartada se não se move sob carga). Quadros não são
// medidos aqui: lê o que `medicoes.rs` guardou (só existe uma sessão de rastreamento). Sensor da placa, potência
// de pacote, latência e entrada saem UNKNOWN, com o motivo.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use sysinfo::{Disks, Networks, System};

use super::telemetry::{id_do_nucleo, Metric, Telemetry, Unit};

#[derive(Debug, Serialize, Deserialize)]
pub struct CPUMetrics {
    pub overall: Option<f32>,
    pub per_core: Vec<f32>,
    /// Sem provedor. Era `0.0` com um "Placeholder" ao lado, e chegava à tela como zero grau.
    pub temperature: Option<f32>,
    /// Informado pelo sistema: não é clock efetivo.
    pub frequency: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RAMMetrics {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub cached_gb: Option<f64>,
    pub usage_percent: Option<f32>,
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
    /// `None` na primeira leitura e quando os contadores não separam disco parado de leitura que falhou.
    pub read_speed_mbps: Option<f64>,
    pub write_speed_mbps: Option<f64>,
    pub usage_percent: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub download_speed_mbps: Option<f64>,
    pub upload_speed_mbps: Option<f64>,
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
    pub uptime_hours: f64,
    pub telemetry: Telemetry,
    /// Junto, não num comando separado: senão a tela mostraria o diagnóstico de uma coleta e os números de outra.
    pub gargalo: super::gargalo::Diagnostico,
    /// Junto pela mesma razão do gargalo.
    pub vram: super::vram::Analise,
    /// Um PISO: três das cinco etapas não se medem daqui, e o orçamento diz quais.
    pub latencia: super::latencia::Orcamento,
}

/// Uso de CPU é diferença entre dois instantes; o `sysinfo` exige pelo menos 200 ms entre os refreshes.
const ESPERA_DE_AMOSTRAGEM_MS: u64 = 200;

/// Duas chamadas coladas dividiriam por quase zero.
const INTERVALO_MINIMO_S: f64 = 0.05;

pub struct PerformanceMonitor {
    monitoring_active: bool,
    system: System,
    /// Vivos entre chamadas: disco e rede só existem como diferença entre duas leituras.
    disks: Disks,
    networks: Networks,
    last_sample: Option<std::time::Instant>,
    /// Com a lista mudada (pendrive, volume, adaptador), a diferença deixa de ser sobre o mesmo conjunto.
    discos_vistos: Option<BTreeSet<String>>,
    interfaces_vistas: Option<BTreeSet<String>>,
    /// Medida acumulada: sem ela toda máquina ligada seria acusada de transbordo. Ver `vram::Piso`.
    piso_compartilhada: super::vram::Piso,
    /// O amostrador do motor de energia (`motorenergia_maquina::Amostrador`), agora também para a tela. Vivo entre
    /// coletas: o PDH entrega a diferença entre dois `PdhCollectQueryData`.
    #[cfg(target_os = "windows")]
    contadores: Option<crate::modules::windows::motorenergia_maquina::Amostrador>,
    /// Uma leitura não distingue "28 °C" de "publica 28 °C o dia inteiro": a janela aplica
    /// `motorenergia::resumir_cpu`.
    #[cfg(target_os = "windows")]
    amostras_recentes:
        std::collections::VecDeque<crate::modules::windows::motorenergia::AmostraCpu>,
    /// `Arc` com a tarefa de fundo. A coleta nunca espera: serve o que tem, com a idade escrita.
    #[cfg(target_os = "windows")]
    placa: std::sync::Arc<std::sync::Mutex<Option<(std::time::Instant, LeituraLenta)>>>,
    /// Sem isto, um laço atrasado empilharia leituras de fundo.
    #[cfg(target_os = "windows")]
    placa_em_curso: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Fato de hardware: não muda com o programa aberto.
    #[cfg(target_os = "windows")]
    vram_total_gb: Option<Option<f64>>,
    /// Vivos entre coletas: o PDH entrega a diferença entre duas consultas.
    #[cfg(target_os = "windows")]
    contadores_placa: Option<crate::modules::windows::placa::Contadores>,
    /// Outro ritmo: são vinte pings contra o servidor do jogo, e o Otimiza não vai martelar a hospedagem do cliente.
    #[cfg(target_os = "windows")]
    rede: std::sync::Arc<
        std::sync::Mutex<
            Option<(
                std::time::Instant,
                crate::modules::windows::rede::MedidaDeRede,
            )>,
        >,
    >,
    #[cfg(target_os = "windows")]
    rede_em_curso: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(target_os = "windows")]
const INTERVALO_DA_REDE: std::time::Duration = std::time::Duration::from_secs(60);

/// Passado isso o cliente pode ter trocado de servidor ou saído do jogo.
#[cfg(target_os = "windows")]
const VALIDADE_DA_REDE: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// `resumir_cpu` só julga com oito ou mais; doze dá folga.
#[cfg(target_os = "windows")]
const JANELA_TERMICA: usize = 12;

/// O que só dá para ler devagar: o modo de vídeo e o histórico de quadros em disco.
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Default)]
struct LeituraLenta {
    hz: Option<u32>,
    /// Só LÊ o que o vigia de `medicoes.rs` guardou: uma segunda sessão de rastreamento derrubaria a dele.
    quadros: Option<crate::modules::medicoes::MedicaoAutomatica>,
}

/// O modo de vídeo só muda quando alguém o troca, e o histórico é reescrito no máximo a cada vinte minutos.
#[cfg(target_os = "windows")]
const INTERVALO_DA_PLACA: std::time::Duration = std::time::Duration::from_secs(10);

impl PerformanceMonitor {
    pub fn new() -> Self {
        PerformanceMonitor {
            monitoring_active: false,
            // `new_all()` varreria a tabela de processos no arranque, e este coletor não olha processos.
            system: System::new(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_sample: None,
            discos_vistos: None,
            interfaces_vistas: None,
            piso_compartilhada: super::vram::Piso::default(),
            // Na primeira coleta: abrir a consulta do PDH custa, e muita sessão nunca olha o painel.
            #[cfg(target_os = "windows")]
            contadores: None,
            #[cfg(target_os = "windows")]
            amostras_recentes: std::collections::VecDeque::with_capacity(JANELA_TERMICA),
            #[cfg(target_os = "windows")]
            placa: std::sync::Arc::new(std::sync::Mutex::new(None)),
            #[cfg(target_os = "windows")]
            placa_em_curso: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            #[cfg(target_os = "windows")]
            vram_total_gb: None,
            #[cfg(target_os = "windows")]
            contadores_placa: None,
            #[cfg(target_os = "windows")]
            rede: std::sync::Arc::new(std::sync::Mutex::new(None)),
            #[cfg(target_os = "windows")]
            rede_em_curso: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn start_monitoring(&mut self) {
        self.monitoring_active = true;
    }

    pub fn stop_monitoring(&mut self) {
        self.monitoring_active = false;
    }

    /// Para quem classifica fora deste laço (o Mapa de desempenho).
    pub fn piso_de_vram(&self) -> super::vram::Piso {
        self.piso_compartilhada
    }

    /// Sem `refresh_all`: varria os processos e cada coletor refazia o próprio refresh. Monitor que pesa no
    /// desempenho é o pior defeito possível aqui.
    pub async fn collect_metrics(&mut self) -> Result<PerformanceMetrics, String> {
        let comeco = std::time::Instant::now();

        // O intervalo é medido uma vez: cada coletor dividiria por uma janela diferente.
        let elapsed = self.elapsed_since_last();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut telemetry = Telemetry::new(timestamp, elapsed.map(|s| (s * 1000.0).round() as u64));

        let cpu = self.collect_cpu_metrics(&mut telemetry).await;
        let ram = self.collect_ram_metrics(&mut telemetry);
        let gpu = self.collect_gpu_metrics(&mut telemetry);
        let disk = self.collect_disk_metrics(elapsed, &mut telemetry);
        let network = self.collect_network_metrics(elapsed, &mut telemetry);

        let uptime = uptime_hours();
        telemetry.set(
            "system.uptime",
            Metric::measured(uptime, Unit::Hours, "windows"),
        );

        declarar_o_que_este_laco_nao_mede(&mut telemetry);

        // Onde há medição de quadros guardada, ela entra por cima do motivo genérico.
        #[cfg(target_os = "windows")]
        self.telemetria_dos_quadros(&mut telemetry);

        #[cfg(target_os = "windows")]
        self.telemetria_da_rede(&mut telemetry);

        let telemetry = telemetry.finish(comeco.elapsed().as_millis() as u64);

        // O piso aprende ANTES da análise: a coleta em repouso já serve de referência para si mesma.
        self.piso_compartilhada.observar(
            telemetry.value("vram.shared_used"),
            telemetry.value("gpu.usage"),
        );
        let vram = super::vram::avaliar(&telemetry, &self.piso_compartilhada);
        let gargalo = super::gargalo::classificar_com(&telemetry, &vram);

        Ok(PerformanceMetrics {
            timestamp,
            cpu,
            ram,
            gpu,
            disk,
            network,
            uptime_hours: uptime,
            latencia: super::latencia::orcar(&telemetry),
            telemetry,
            gargalo,
            vram,
        })
    }

    async fn collect_cpu_metrics(&mut self, t: &mut Telemetry) -> CPUMetrics {
        // ANTES da espera: abrir a consulta já conta como a primeira leitura. Aberta junto da leitura, a janela seria de
        // zero segundo (`% Processor Time` 100 e o efetivo empatando com o reportado, visto na primeira ligação).
        #[cfg(target_os = "windows")]
        if self.contadores.is_none() {
            self.contadores = crate::modules::windows::motorenergia_maquina::Amostrador::novo();
        }

        #[cfg(target_os = "windows")]
        if self.contadores_placa.is_none() {
            self.contadores_placa = crate::modules::windows::placa::Contadores::novo();
        }

        self.system.refresh_cpu_all();
        tokio::time::sleep(tokio::time::Duration::from_millis(ESPERA_DE_AMOSTRAGEM_MS)).await;
        self.system.refresh_cpu_all();

        let cpus = self.system.cpus();
        let per_core: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();

        for (indice, uso) in per_core.iter().enumerate() {
            t.set_series(
                id_do_nucleo(indice),
                Metric::measured(*uso as f64, Unit::Percent, "sysinfo").require_range(0.0, 100.0),
            );
        }

        // Sem núcleo a média viraria NaN: o coletor diz que não sabe.
        let overall = if per_core.is_empty() {
            t.set(
                "cpu.usage.overall",
                Metric::unknown(Unit::Percent, "o sistema não listou nenhum núcleo"),
            );
            t.set(
                "cpu.cores.logical",
                Metric::unknown(Unit::Count, "o sistema não listou nenhum núcleo"),
            );
            None
        } else {
            let media = per_core.iter().sum::<f32>() / per_core.len() as f32;

            let metrica =
                Metric::measured(media as f64, Unit::Percent, "sysinfo").require_range(0.0, 100.0);
            let aceita = metrica.value.map(|v| v as f32);
            t.set("cpu.usage.overall", metrica);

            t.set(
                "cpu.cores.logical",
                Metric::measured(per_core.len() as f64, Unit::Count, "sysinfo"),
            );

            aceita
        };

        // ESTIMATED: é o clock da tabela de frequências, e de um núcleo só (em CPU híbrida não representa os outros).
        // `cpu.clock.effective` é outra grandeza.
        let frequency = match cpus.first().map(|cpu| cpu.frequency()) {
            Some(mhz) if mhz > 0 => {
                t.set(
                    "cpu.clock.reported",
                    Metric::estimated(
                        mhz as f64,
                        Unit::Megahertz,
                        "sysinfo",
                        "clock informado pelo sistema para o primeiro núcleo; não é clock efetivo",
                    ),
                );
                Some(mhz as f32)
            }
            _ => {
                t.set(
                    "cpu.clock.reported",
                    Metric::unknown(Unit::Megahertz, "o sistema não informou a frequência"),
                );
                None
            }
        };

        t.set(
            "cpu.clock.effective",
            Metric::unknown(
                Unit::Megahertz,
                "exige contador de desempenho do Windows; não medido nesta coleta",
            ),
        );
        t.set(
            "cpu.temperature",
            Metric::unknown(
                Unit::Celsius,
                "exige acesso de baixo nível ao sensor; sem provedor nesta versão",
            ),
        );

        #[cfg(target_os = "windows")]
        let temperatura = self.telemetria_dos_contadores(t);
        #[cfg(not(target_os = "windows"))]
        let temperatura = None;

        CPUMetrics {
            overall,
            per_core,
            temperature: temperatura,
            frequency,
        }
    }

    /// UMA AMOSTRA por coleta: a temperatura sai ESTIMATED; serve à tela, não a veredito térmico.
    #[cfg(target_os = "windows")]
    fn telemetria_dos_contadores(&mut self, t: &mut Telemetry) -> Option<f32> {
        let Some(amostrador) = self.contadores.as_ref() else {
            t.set(
                "cpu.clock.effective",
                Metric::unknown(
                    Unit::Megahertz,
                    "os contadores de desempenho do Windows não abriram nesta máquina",
                ),
            );
            return None;
        };

        let a = amostrador.coletar();

        match a.clock_efetivo_mhz() {
            Some(mhz) => t.set(
                "cpu.clock.effective",
                Metric::measured(mhz, Unit::Megahertz, "pdh"),
            ),
            None => t.set(
                "cpu.clock.effective",
                Metric::unknown(Unit::Megahertz, "o contador de desempenho não respondeu"),
            ),
        }

        if let Some(mhz) = a.clock_reportado_mhz() {
            t.set(
                "cpu.clock.reported",
                Metric::measured(mhz, Unit::Megahertz, "pdh"),
            );
        }

        match a.flags {
            Some(f) => {
                t.set(
                    "cpu.throttling.thermal",
                    Metric::measured((f & 1 != 0) as u8 as f64, Unit::Boolean, "pdh"),
                );
                t.set(
                    "cpu.throttling.power",
                    Metric::measured((f & 2 != 0) as u8 as f64, Unit::Boolean, "pdh"),
                );
            }
            None => {
                const MUDO: &str = "o contador de limites de firmware não respondeu";
                t.set(
                    "cpu.throttling.thermal",
                    Metric::unknown(Unit::Boolean, MUDO),
                );
                t.set("cpu.throttling.power", Metric::unknown(Unit::Boolean, MUDO));
            }
        }

        match a.limite_pct {
            Some(pct) => t.set(
                "cpu.performance_limit",
                Metric::measured(pct, Unit::Percent, "pdh").require_range(0.0, 100.0),
            ),
            None => t.set(
                "cpu.performance_limit",
                Metric::unknown(Unit::Percent, "o contador de limite não respondeu"),
            ),
        }

        match a.nucleos_acordados.zip(a.nucleos_total) {
            Some((acordados, total)) => t.set(
                "cpu.cores.parked",
                Metric::measured(total.saturating_sub(acordados) as f64, Unit::Count, "pdh"),
            ),
            None => t.set(
                "cpu.cores.parked",
                Metric::unknown(Unit::Count, "o contador de estacionamento não respondeu"),
            ),
        }

        self.julgar_temperatura(a, t)
    }

    /// A regra de `motorenergia::resumir_cpu`, reusada: com carga e zona que não se move, a leitura é descartada
    /// (esta máquina publica 27,85 °C com a CPU a 50%).
    #[cfg(target_os = "windows")]
    fn julgar_temperatura(
        &mut self,
        a: crate::modules::windows::motorenergia::AmostraCpu,
        t: &mut Telemetry,
    ) -> Option<f32> {
        use crate::modules::windows::motorenergia::resumir_cpu;

        if self.amostras_recentes.len() == JANELA_TERMICA {
            self.amostras_recentes.pop_front();
        }
        self.amostras_recentes.push_back(a);

        if a.temperatura_c.is_none() {
            t.set(
                "cpu.temperature",
                Metric::unknown(
                    Unit::Celsius,
                    "esta máquina não publica zona térmica ACPI utilizável",
                ),
            );
            return None;
        }

        let janela: Vec<_> = self.amostras_recentes.iter().copied().collect();
        let fechada = janela.len() >= 8;
        let julgada = fechada.then(|| resumir_cpu(&janela).and_then(|r| r.temperatura_max_c));

        match julgada {
            Some(Some(_)) | None => {
                let atual = a.temperatura_c.expect("verificado acima");
                let motivo = if fechada {
                    "zona térmica ACPI, que pode não ser o sensor do processador"
                } else {
                    "zona térmica ACPI; ainda sem leituras suficientes para descartar zona travada"
                };
                t.set(
                    "cpu.temperature",
                    Metric::estimated(atual, Unit::Celsius, "acpi", motivo),
                );
                Some(atual as f32)
            }
            Some(None) => {
                t.set(
                    "cpu.temperature",
                    Metric::unknown(
                        Unit::Celsius,
                        "a zona térmica ACPI desta placa não se move sob carga: \
                         é um valor fixo, não uma medição",
                    ),
                );
                None
            }
        }
    }

    fn collect_ram_metrics(&mut self, t: &mut Telemetry) -> RAMMetrics {
        self.system.refresh_memory();

        let total_bytes = self.system.total_memory();
        let used_bytes = self.system.used_memory();
        let available_bytes = self.system.available_memory();

        let total_gb = bytes_to_gb(total_bytes);
        let used_gb = bytes_to_gb(used_bytes);
        let available_gb = bytes_to_gb(available_bytes);

        t.set(
            "ram.total",
            Metric::measured(total_gb, Unit::Gigabytes, "sysinfo"),
        );
        t.set(
            "ram.used",
            Metric::measured(used_gb, Unit::Gigabytes, "sysinfo"),
        );
        t.set(
            "ram.available",
            Metric::measured(available_gb, Unit::Gigabytes, "sysinfo"),
        );
        t.set(
            "ram.cached",
            Metric::unknown(
                Unit::Gigabytes,
                "o sysinfo não expõe memória em cache no Windows",
            ),
        );

        // Zero diria que a RAM está livre.
        let usage_percent = if total_bytes == 0 {
            t.set(
                "ram.usage",
                Metric::unknown(Unit::Percent, "o sistema informou memória total zero"),
            );
            None
        } else {
            let pct = used_bytes as f64 / total_bytes as f64 * 100.0;
            let metrica = Metric::measured(pct, Unit::Percent, "sysinfo").require_range(0.0, 100.0);
            let aceita = metrica.value.map(|v| v as f32);
            t.set("ram.usage", metrica);
            aceita
        };

        RAMMetrics {
            total_gb,
            used_gb,
            available_gb,
            cached_gb: None,
            usage_percent,
        }
    }

    /// Uso e memória da placa vêm do PDH; o sensor (clock, temperatura, potência) exige NVML, ADL ou o equivalente da
    /// Intel.
    fn collect_gpu_metrics(&mut self, t: &mut Telemetry) -> Option<GPUMetrics> {
        const SEM_SENSOR: &str = "exige NVML/ADL/Intel; o contador do Windows não expõe sensor";
        t.set("gpu.clock", Metric::unknown(Unit::Megahertz, SEM_SENSOR));
        t.set(
            "gpu.temperature",
            Metric::unknown(Unit::Celsius, SEM_SENSOR),
        );
        t.set("gpu.power", Metric::unknown(Unit::Watts, SEM_SENSOR));

        // Motivo escrito aqui: o produto SABE medir (`modules::mouse::taxa_de_varredura`), falta ligar a captura. É uma
        // tarefa, não uma lacuna.
        t.set(
            "input.polling_rate",
            Metric::unknown(
                Unit::Hertz,
                "exige contar os relatos de entrada que chegam à janela do aplicativo; a captura \
                 ainda não está ligada",
            ),
        );

        #[cfg(target_os = "windows")]
        self.telemetria_da_placa(t);

        #[cfg(not(target_os = "windows"))]
        {
            const FORA: &str = "os contadores de placa de vídeo são do Windows";
            t.set("gpu.usage", Metric::unknown(Unit::Percent, FORA));
            t.set("vram.used", Metric::unknown(Unit::Gigabytes, FORA));
            t.set("vram.total", Metric::unknown(Unit::Gigabytes, FORA));
            t.set("vram.usage", Metric::unknown(Unit::Percent, FORA));
            t.set("vram.shared_used", Metric::unknown(Unit::Gigabytes, FORA));
            t.set("storage.busy", Metric::unknown(Unit::Percent, FORA));
            t.set("display.refresh", Metric::unknown(Unit::Hertz, FORA));
        }

        // O bloco `gpu` antigo exigia cinco números obrigatórios; preenchê-lo inventaria três. O que a placa entrega vai
        // pelo contrato.
        None
    }

    /// Não mede (só existe UMA sessão de rastreamento) e não preenche `fps.rendered`, `generated` nem `displayed`: o
    /// vigia mede um processo, e separar os três exige `frames::medir_par`.
    #[cfg(target_os = "windows")]
    fn telemetria_dos_quadros(&mut self, t: &mut Telemetry) {
        let Some((quando, leitura)) = self.placa.lock().ok().and_then(|g| g.clone()) else {
            return;
        };
        let Some(medicao) = leitura.quadros else {
            t.set(
                "fps.average",
                Metric::unknown(
                    Unit::Fps,
                    "ainda não há medição de quadros guardada nesta máquina",
                ),
            );
            return;
        };

        // Carimbo de relógio mais carimbo de processo: a idade real é a soma.
        let agora = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let desde_a_medicao = std::time::Duration::from_secs(agora.saturating_sub(medicao.quando));
        let idade = desde_a_medicao + std::time::Instant::now().duration_since(quando);

        if idade > VALIDADE_DOS_QUADROS {
            let velha = format!(
                "a última medição de quadros é de {} min atrás, em {}",
                idade.as_secs() / 60,
                medicao.jogo
            );
            t.set("fps.average", Metric::unknown(Unit::Fps, velha.clone()));
            t.set("fps.low_1pct", Metric::unknown(Unit::Fps, velha));
            return;
        }

        let idade_ms = idade.as_millis() as u64;
        let jogo = medicao.jogo.clone();

        t.set(
            "fps.average",
            Metric::estimated(
                medicao.fps,
                Unit::Fps,
                "etw",
                format!("medido em {jogo} durante a partida"),
            )
            .com_idade(idade_ms),
        );

        // Cauda de amostra curta é ruído: 1% low e engasgos ficam de fora.
        if !medicao.confiavel {
            const CURTA: &str =
                "a amostra foi curta demais para a cauda da distribuição significar algo";
            t.set("fps.low_1pct", Metric::unknown(Unit::Fps, CURTA));
            t.set(
                "frametime.stutters_per_minute",
                Metric::unknown(Unit::Count, CURTA),
            );
            return;
        }

        t.set(
            "fps.low_1pct",
            Metric::estimated(
                medicao.low_1pct,
                Unit::Fps,
                "etw",
                format!("medido em {jogo} durante a partida"),
            )
            .com_idade(idade_ms),
        );
        t.set(
            "frametime.stutters_per_minute",
            Metric::estimated(
                medicao.engasgos_por_minuto,
                Unit::Count,
                "etw",
                format!("quadros acima do dobro da mediana em {jogo}"),
            )
            .com_idade(idade_ms),
        );

        // Com três buracos, "67% com disco" são dois: o total vem junto para o corte existir.
        const TRANCOS_MINIMOS: usize = 8;

        match (medicao.trancos_com_disco_pct, medicao.trancos_medidos) {
            (Some(pct), Some(total)) if total >= TRANCOS_MINIMOS => t.set(
                "frametime.stutters_with_disk",
                Metric::estimated(
                    pct,
                    Unit::Percent,
                    "etw+pdh",
                    format!("{total} trancos cruzados com o disco em {jogo}"),
                )
                .com_idade(idade_ms)
                .require_range(0.0, 100.0),
            ),
            (_, Some(total)) => t.set(
                "frametime.stutters_with_disk",
                Metric::unknown(
                    Unit::Percent,
                    format!("só {total} trancos na partida: poucos para uma proporção"),
                ),
            ),
            _ => t.set(
                "frametime.stutters_with_disk",
                Metric::unknown(
                    Unit::Percent,
                    "esta medição é de uma versão anterior, que não cruzava trancos com o disco",
                ),
            ),
        }

        for (id, valor) in [
            ("match.cpu_usage", medicao.cpu_uso_pct),
            ("match.gpu_usage", medicao.gpu_uso_pct),
        ] {
            match valor {
                Some(pct) => t.set(
                    id,
                    Metric::estimated(
                        pct,
                        Unit::Percent,
                        "pdh",
                        format!("medido durante a partida em {jogo}"),
                    )
                    .com_idade(idade_ms)
                    .require_range(0.0, 100.0),
                ),
                None => t.set(
                    id,
                    Metric::unknown(
                        Unit::Percent,
                        "esta medição é de uma versão anterior, que não guardava o contexto",
                    ),
                ),
            }
        }

        // FPS maior com ritmo pior não é melhora, e é o P99 que denuncia. Medição antiga chega `None`.
        for (id, valor) in [
            ("frametime.mean", medicao.frametime_medio_ms),
            ("frametime.p95", medicao.frametime_p95_ms),
            ("frametime.p99", medicao.frametime_p99_ms),
        ] {
            match valor {
                Some(ms) => t.set(
                    id,
                    Metric::estimated(
                        ms,
                        Unit::Milliseconds,
                        "etw",
                        format!("medido em {jogo} durante a partida"),
                    )
                    .com_idade(idade_ms),
                ),
                None => t.set(
                    id,
                    Metric::unknown(
                        Unit::Milliseconds,
                        "esta medição é de uma versão anterior, que não guardava a distribuição",
                    ),
                ),
            }
        }
    }

    /// Mede com `windows::rede`, que só mede contra o SERVIDOR DO JOGO e separa perda real de ping descartado por
    /// regra. Aqui só se traduz para o contrato.
    #[cfg(target_os = "windows")]
    fn telemetria_da_rede(&mut self, t: &mut Telemetry) {
        use crate::modules::windows::rede::Perda;
        use std::sync::atomic::Ordering;

        let agora = std::time::Instant::now();
        let guardada = self.rede.lock().ok().and_then(|g| g.clone());

        let precisa = match &guardada {
            None => true,
            Some((quando, _)) => agora.duration_since(*quando) >= INTERVALO_DA_REDE,
        };

        if precisa && !self.rede_em_curso.swap(true, Ordering::AcqRel) {
            let destino = self.rede.clone();
            let bandeira = self.rede_em_curso.clone();

            tokio::task::spawn_blocking(move || {
                let medida = crate::modules::windows::rede::medir_agora();
                if let Ok(mut g) = destino.lock() {
                    *g = Some((std::time::Instant::now(), medida));
                }
                bandeira.store(false, Ordering::Release);
            });
        }

        const IDS: [&str; 3] = ["network.latency", "network.jitter", "network.packet_loss"];

        let Some((quando, medida)) = guardada else {
            const PRIMEIRA: &str = "a primeira medição de rede ainda não voltou";
            t.set(
                "network.latency",
                Metric::unknown(Unit::Milliseconds, PRIMEIRA),
            );
            t.set(
                "network.jitter",
                Metric::unknown(Unit::Milliseconds, PRIMEIRA),
            );
            t.set(
                "network.packet_loss",
                Metric::unknown(Unit::Percent, PRIMEIRA),
            );
            return;
        };

        let idade = agora.duration_since(quando);

        if idade > VALIDADE_DA_REDE {
            let velha = format!(
                "a última medição de rede tem {} min: o servidor ou a rota podem ter mudado",
                idade.as_secs() / 60
            );
            for id in IDS {
                let unidade = if id.ends_with("loss") {
                    Unit::Percent
                } else {
                    Unit::Milliseconds
                };
                t.set(id, Metric::unknown(unidade, velha.clone()));
            }
            return;
        }

        let idade_ms = idade.as_millis() as u64;
        let alvo = medida
            .alvo
            .clone()
            .unwrap_or_else(|| "sem alvo".to_string());

        for (id, valor) in [
            ("network.latency", medida.tempo_ms),
            ("network.jitter", medida.jitter_ms),
        ] {
            match valor {
                Some(ms) => t.set(
                    id,
                    Metric::estimated(
                        ms,
                        Unit::Milliseconds,
                        "icmp",
                        format!("medido contra {alvo}"),
                    )
                    .com_idade(idade_ms),
                ),
                None => t.set(
                    id,
                    Metric::unknown(
                        Unit::Milliseconds,
                        "não houve resposta suficiente para este número",
                    ),
                ),
            }
        }

        // Só `Medida` é medição: as outras variantes são a sonda dizendo que não sabe.
        match medida.perda {
            Perda::Medida { enviados, perdidos } if enviados > 0 => t.set(
                "network.packet_loss",
                Metric::estimated(
                    perdidos as f64 / enviados as f64 * 100.0,
                    Unit::Percent,
                    "icmp",
                    format!("{perdidos} de {enviados} pacotes contra {alvo}"),
                )
                .com_idade(idade_ms)
                .require_range(0.0, 100.0),
            ),
            Perda::Medida { .. } | Perda::NaoMedi => t.set(
                "network.packet_loss",
                Metric::unknown(
                    Unit::Percent,
                    "nenhum pacote saiu: não é 0% de perda nem 100%, é ausência de dado",
                ),
            ),
            Perda::NaoRespondePing { .. } => t.set(
                "network.packet_loss",
                Metric::unknown(
                    Unit::Percent,
                    "o servidor descarta ping mas aceita conexão na porta do jogo: \
                     não dá para saber se houve perda real",
                ),
            ),
            Perda::PingLimitado { .. } => t.set(
                "network.packet_loss",
                Metric::unknown(
                    Unit::Percent,
                    "o servidor limita a taxa de ping: não dá para saber se houve perda real",
                ),
            ),
        }
    }

    /// A coleta NUNCA espera a leitura de fundo: olha o que ela deixou, carimba a idade e segue.
    #[cfg(target_os = "windows")]
    fn telemetria_da_placa(&mut self, t: &mut Telemetry) {
        use std::sync::atomic::Ordering;

        let agora = std::time::Instant::now();
        let guardada = self.placa.lock().ok().and_then(|g| g.clone());

        let precisa = match guardada {
            None => true,
            Some((quando, _)) => agora.duration_since(quando) >= INTERVALO_DA_PLACA,
        };

        if precisa && !self.placa_em_curso.swap(true, Ordering::AcqRel) {
            let destino = self.placa.clone();
            let bandeira = self.placa_em_curso.clone();

            tokio::task::spawn_blocking(move || {
                let leitura = ler_placa_devagar();
                if let Ok(mut g) = destino.lock() {
                    *g = Some((std::time::Instant::now(), leitura));
                }
                bandeira.store(false, Ordering::Release);
            });
        }

        let total = *self
            .vram_total_gb
            .get_or_insert_with(crate::modules::windows::bottleneck::vram_total_gb_opt);

        match total {
            Some(gb) => t.set(
                "vram.total",
                Metric::measured(gb, Unit::Gigabytes, "registro"),
            ),
            None => t.set(
                "vram.total",
                Metric::unknown(
                    Unit::Gigabytes,
                    "o driver não publicou o tamanho da memória da placa no registro",
                ),
            ),
        }

        // Não envelhece como o resto: muda quando alguém troca o modo.
        match guardada {
            Some((quando, leitura)) => marcar_hz(t, leitura.hz, agora.duration_since(quando)),
            None => t.set(
                "display.refresh",
                Metric::unknown(
                    Unit::Hertz,
                    "a primeira leitura do modo de vídeo ainda não voltou",
                ),
            ),
        }

        let Some(contadores) = self.contadores_placa.as_ref() else {
            const SEM: &str = "os contadores de placa e disco não abriram nesta máquina";
            t.set("gpu.usage", Metric::unknown(Unit::Percent, SEM));
            t.set("vram.used", Metric::unknown(Unit::Gigabytes, SEM));
            t.set("vram.usage", Metric::unknown(Unit::Percent, SEM));
            t.set("vram.shared_used", Metric::unknown(Unit::Gigabytes, SEM));
            t.set("storage.busy", Metric::unknown(Unit::Percent, SEM));
            t.set("storage.latency", Metric::unknown(Unit::Milliseconds, SEM));
            return;
        };

        let a = contadores.coletar();

        match a.gpu_pct {
            Some(pct) => t.set(
                "gpu.usage",
                Metric::measured(pct, Unit::Percent, "pdh").require_range(0.0, 100.0),
            ),
            None => t.set(
                "gpu.usage",
                Metric::unknown(
                    Unit::Percent,
                    "esta máquina não publica o contador de motores 3D",
                ),
            ),
        }

        match a.disco_ocupado_pct {
            Some(pct) => t.set(
                "storage.busy",
                Metric::measured(pct, Unit::Percent, "pdh").require_range(0.0, 100.0),
            ),
            None => t.set(
                "storage.busy",
                Metric::unknown(Unit::Percent, "o contador de disco físico não respondeu"),
            ),
        }

        // 100% ocupado com 0,2 ms dá conta; 40% com 30 ms trava o jogo.
        match a.disco_latencia_ms {
            Some(ms) => t.set(
                "storage.latency",
                Metric::measured(ms, Unit::Milliseconds, "pdh"),
            ),
            None => t.set(
                "storage.latency",
                Metric::unknown(
                    Unit::Milliseconds,
                    "o contador de tempo por transferência não respondeu",
                ),
            ),
        }

        match a.vram_mb {
            Some(mb) => {
                let gb = mb / 1024.0;
                t.set("vram.used", Metric::measured(gb, Unit::Gigabytes, "pdh"));

                match total {
                    Some(tot) if tot > 0.0 => t.set(
                        "vram.usage",
                        Metric::measured(gb / tot * 100.0, Unit::Percent, "pdh")
                            .require_range(0.0, 100.0),
                    ),
                    _ => t.set(
                        "vram.usage",
                        Metric::unknown(
                            Unit::Percent,
                            "sem o total da placa não há porcentagem a calcular",
                        ),
                    ),
                }
            }
            None => {
                const MUDO: &str = "o contador de memória da placa não respondeu";
                t.set("vram.used", Metric::unknown(Unit::Gigabytes, MUDO));
                t.set("vram.usage", Metric::unknown(Unit::Percent, MUDO));
            }
        }

        // Separa cache cheio (normal) de "não coube, indo pelo PCIe". Interpretada em `modules::vram`.
        match a.vram_compartilhada_mb {
            Some(mb) => t.set(
                "vram.shared_used",
                Metric::measured(mb / 1024.0, Unit::Gigabytes, "pdh"),
            ),
            None => t.set(
                "vram.shared_used",
                Metric::unknown(
                    Unit::Gigabytes,
                    "esta máquina não publica o contador de memória compartilhada da placa",
                ),
            ),
        }
    }

    fn elapsed_since_last(&mut self) -> Option<f64> {
        let agora = std::time::Instant::now();
        let anterior = self.last_sample.replace(agora)?;

        let segundos = agora.duration_since(anterior).as_secs_f64();

        (segundos >= INTERVALO_MINIMO_S).then_some(segundos)
    }

    fn collect_disk_metrics(&mut self, elapsed: Option<f64>, t: &mut Telemetry) -> DiskMetrics {
        // Mantida entre chamadas: o sysinfo entrega a diferença desde o refresh anterior.
        self.disks.refresh(true);

        let mut total_space = 0u64;
        let mut used_space = 0u64;
        let mut read_bytes = 0u64;
        let mut written_bytes = 0u64;
        let mut agora: BTreeSet<String> = BTreeSet::new();
        // O mesmo volume pode aparecer montado em dois lugares.
        let mut volumes_contados: BTreeSet<String> = BTreeSet::new();

        for disk in &self.disks {
            let ponto = disk.mount_point().to_string_lossy().to_string();
            agora.insert(ponto.clone());

            let usage = disk.usage();
            read_bytes += usage.read_bytes;
            written_bytes += usage.written_bytes;

            // ISO montado está sempre 100% cheio e não diz nada da máquina.
            if disk.is_removable() || disk.is_read_only() {
                continue;
            }

            if !volumes_contados.insert(ponto) {
                continue;
            }

            total_space += disk.total_space();
            used_space += disk.total_space().saturating_sub(disk.available_space());
        }

        let lista_mudou = self
            .discos_vistos
            .replace(agora.clone())
            .is_some_and(|antes| antes != agora);

        let usage_percent = if total_space == 0 {
            t.set(
                "storage.capacity_used",
                Metric::unknown(Unit::Percent, "nenhum volume fixo foi listado"),
            );
            None
        } else {
            let pct = used_space as f64 / total_space as f64 * 100.0;
            // ESTIMATED: um disco cheio dividido com outro vazio some na soma.
            let metrica = Metric::estimated(
                pct,
                Unit::Percent,
                "sysinfo",
                "soma de todos os volumes fixos; não separa disco físico",
            )
            .require_range(0.0, 100.0);
            let aceita = metrica.value.map(|v| v as f32);
            t.set("storage.capacity_used", metrica);
            aceita
        };

        let (read, write) =
            match self.taxa_de_disco(elapsed, lista_mudou, read_bytes, written_bytes) {
                Ok((r, w)) => {
                    // ESTIMATED: o sysinfo devolve zero quando o IOCTL falha num volume; um total que avançou prova só que ALGUM
                    // respondeu.
                    const MOTIVO: &str =
                        "contadores do sysinfo; falha de leitura em um volume chega como zero";
                    t.set(
                        "storage.read_rate",
                        Metric::estimated(r, Unit::MegabytesPerSecond, "sysinfo", MOTIVO),
                    );
                    t.set(
                        "storage.write_rate",
                        Metric::estimated(w, Unit::MegabytesPerSecond, "sysinfo", MOTIVO),
                    );
                    (Some(r), Some(w))
                }
                Err(motivo) => {
                    t.set(
                        "storage.read_rate",
                        Metric::unknown(Unit::MegabytesPerSecond, motivo),
                    );
                    t.set(
                        "storage.write_rate",
                        Metric::unknown(Unit::MegabytesPerSecond, motivo),
                    );
                    (None, None)
                }
            };

        DiskMetrics {
            read_speed_mbps: read,
            write_speed_mbps: write,
            usage_percent,
        }
    }

    /// Sem contador avançando, NÃO é "0 MB/s": o sysinfo devolve zero tanto para parado quanto para falha.
    fn taxa_de_disco(
        &self,
        elapsed: Option<f64>,
        lista_mudou: bool,
        read_bytes: u64,
        written_bytes: u64,
    ) -> Result<(f64, f64), &'static str> {
        let Some(segundos) = elapsed else {
            return Err("primeira leitura da sessão: taxa é diferença entre duas leituras");
        };

        if lista_mudou {
            return Err("a lista de volumes mudou entre as duas leituras");
        }

        if read_bytes == 0 && written_bytes == 0 {
            return Err(
                "nenhum contador de E/S avançou; o sysinfo não distingue disco parado de leitura que falhou",
            );
        }

        Ok((
            bytes_to_mb(read_bytes) / segundos,
            bytes_to_mb(written_bytes) / segundos,
        ))
    }

    fn collect_network_metrics(
        &mut self,
        elapsed: Option<f64>,
        t: &mut Telemetry,
    ) -> NetworkMetrics {
        self.networks.refresh(true);

        let mut total_received = 0u64;
        let mut total_transmitted = 0u64;
        let mut received = 0u64;
        let mut transmitted = 0u64;
        let mut agora: BTreeSet<String> = BTreeSet::new();

        for (nome, network) in &self.networks {
            agora.insert(nome.clone());
            total_received += network.total_received();
            total_transmitted += network.total_transmitted();
            received += network.received();
            transmitted += network.transmitted();
        }

        let lista_mudou = self
            .interfaces_vistas
            .replace(agora.clone())
            .is_some_and(|antes| antes != agora);

        let (download, upload) = match (elapsed, lista_mudou) {
            (Some(segundos), false) => {
                let d = bytes_to_mb(received) / segundos;
                let u = bytes_to_mb(transmitted) / segundos;

                // MEASURED honesto: interface que falha some da lista em vez de reportar zero.
                t.set(
                    "network.download_rate",
                    Metric::measured(d, Unit::MegabytesPerSecond, "sysinfo"),
                );
                t.set(
                    "network.upload_rate",
                    Metric::measured(u, Unit::MegabytesPerSecond, "sysinfo"),
                );
                (Some(d), Some(u))
            }
            (_, true) => {
                const MOTIVO: &str = "a lista de interfaces mudou entre as duas leituras";
                t.set(
                    "network.download_rate",
                    Metric::unknown(Unit::MegabytesPerSecond, MOTIVO),
                );
                t.set(
                    "network.upload_rate",
                    Metric::unknown(Unit::MegabytesPerSecond, MOTIVO),
                );
                (None, None)
            }
            (None, false) => {
                const MOTIVO: &str =
                    "primeira leitura da sessão: taxa é diferença entre duas leituras";
                t.set(
                    "network.download_rate",
                    Metric::unknown(Unit::MegabytesPerSecond, MOTIVO),
                );
                t.set(
                    "network.upload_rate",
                    Metric::unknown(Unit::MegabytesPerSecond, MOTIVO),
                );
                (None, None)
            }
        };

        const SEM_SONDA: &str = "a taxa de rede não responde latência; ver a sonda";
        t.set(
            "network.latency",
            Metric::unknown(Unit::Milliseconds, SEM_SONDA),
        );
        t.set(
            "network.jitter",
            Metric::unknown(Unit::Milliseconds, SEM_SONDA),
        );
        t.set(
            "network.packet_loss",
            Metric::unknown(Unit::Percent, SEM_SONDA),
        );

        NetworkMetrics {
            download_speed_mbps: download,
            upload_speed_mbps: upload,
            total_received_gb: bytes_to_gb(total_received),
            total_transmitted_gb: bytes_to_gb(total_transmitted),
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Para quadros "sem provedor" seria falso: o Otimiza mede por ETW em `windows::frames`, durante a partida, e não
/// num laço de painel.
fn declarar_o_que_este_laco_nao_mede(t: &mut Telemetry) {
    const QUADROS: &str = "medido durante a partida por ETW (windows::frames), \
                           com o jogo em primeiro plano; o painel não mede quadros";

    for id in [
        "fps.rendered",
        "fps.generated",
        "fps.displayed",
        "fps.average",
        "fps.low_1pct",
        "fps.low_01pct",
    ] {
        t.set(id, Metric::unknown(Unit::Fps, QUADROS));
    }
    for id in ["frametime.mean", "frametime.p95", "frametime.p99"] {
        t.set(id, Metric::unknown(Unit::Milliseconds, QUADROS));
    }

    t.set(
        "input.mouse_polling",
        Metric::unknown(
            Unit::Hertz,
            "exige ler o descritor USB do mouse; sem provedor nesta versão",
        ),
    );
    t.set(
        "input.consistency",
        Metric::unknown(
            Unit::Percent,
            "exige captura de entrada durante o jogo; sem provedor nesta versão",
        ),
    );
    t.set(
        "cpu.package_power",
        Metric::unknown(
            Unit::Watts,
            "exige RAPL por driver assinado; o Otimiza não instala driver",
        ),
    );
}

#[cfg(target_os = "windows")]
fn marcar_hz(t: &mut Telemetry, hz: Option<u32>, idade: std::time::Duration) {
    match hz {
        Some(v) if v > 0 => t.set(
            "display.refresh",
            Metric::measured(v as f64, Unit::Hertz, "win32")
                .com_idade_estavel(idade.as_millis() as u64),
        ),
        _ => t.set(
            "display.refresh",
            Metric::unknown(
                Unit::Hertz,
                "o Windows não informou o modo do monitor principal",
            ),
        ),
    }
}

/// Numa thread de bloqueio: o modo de vídeo e o arquivo de quadros bloqueiam.
#[cfg(target_os = "windows")]
fn ler_placa_devagar() -> LeituraLenta {
    use crate::modules::windows::display;

    let hz = display::monitores()
        .into_iter()
        .find(|m| m.principal)
        .map(|m| m.hz_atual);

    // Pelo CARIMBO, não pela posição no arquivo.
    let quadros = crate::modules::medicoes::ler()
        .unwrap_or_default()
        .into_iter()
        .max_by_key(|m| m.quando);

    LeituraLenta { hz, quadros }
}

/// O vigia mede no máximo a cada vinte minutos: quase sempre há uma na janela durante a partida.
#[cfg(target_os = "windows")]
const VALIDADE_DOS_QUADROS: std::time::Duration = std::time::Duration::from_secs(30 * 60);

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1_073_741_824.0
}

/// Muitos dias sem reiniciar acumulam memória vazada: é a primeira coisa a descartar.
pub fn uptime_hours() -> f64 {
    System::uptime() as f64 / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::telemetry::{Quality, SCHEMA_VERSION};

    #[tokio::test]
    async fn coleta_real_classifica_tudo_o_que_entrega() {
        let mut monitor = PerformanceMonitor::new();
        let m = monitor.collect_metrics().await.expect("coleta");

        assert_eq!(m.telemetry.schema_version, SCHEMA_VERSION);

        // A invariante: quem lê `value` sabe que alguém mediu.
        for (id, metric) in &m.telemetry.metrics {
            match metric.quality {
                Quality::Unknown => {
                    assert!(metric.value.is_none(), "{id} é UNKNOWN e tem valor");
                    assert!(metric.reason.is_some(), "{id} é UNKNOWN e não diz por quê");
                }
                _ => {
                    assert!(metric.value.is_some(), "{id} tem qualidade sem valor");
                    assert!(
                        metric.value.unwrap().is_finite(),
                        "{id} entregou número não finito"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn primeira_leitura_nao_afirma_disco_e_rede_parados() {
        let mut monitor = PerformanceMonitor::new();
        let m = monitor.collect_metrics().await.expect("coleta");

        // O coletor antigo devolvia `(0.0, 0.0)`, "parado", na primeira leitura.
        assert_eq!(m.network.download_speed_mbps, None);
        assert_eq!(m.network.upload_speed_mbps, None);
        assert_eq!(m.disk.read_speed_mbps, None);

        let taxa = m.telemetry.get("network.download_rate").expect("catálogo");
        assert_eq!(taxa.quality, Quality::Unknown);
        assert!(taxa.reason.as_deref().unwrap().contains("primeira leitura"));
    }

    #[tokio::test]
    async fn o_que_nao_tem_sensor_aparece_declarado() {
        let mut monitor = PerformanceMonitor::new();
        let m = monitor.collect_metrics().await.expect("coleta");

        assert_eq!(m.ram.cached_gb, None);

        for id in [
            "cpu.package_power",
            "gpu.clock",
            "gpu.temperature",
            "gpu.power",
            "fps.rendered",
            "fps.generated",
            "fps.displayed",
            "network.latency",
            "network.jitter",
            "input.mouse_polling",
        ] {
            let metric = m.telemetry.get(id).unwrap_or_else(|| panic!("{id} sumiu"));
            assert_eq!(metric.quality, Quality::Unknown, "{id}");
            assert!(metric.value.is_none(), "{id}");
        }
    }

    /// Garantia de que a ponte com o amostrador do motor de energia não se solte.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn os_contadores_do_windows_chegam_ao_contrato() {
        let mut monitor = PerformanceMonitor::new();
        let m = monitor.collect_metrics().await.expect("coleta");

        for id in [
            "cpu.clock.effective",
            "cpu.clock.reported",
            "cpu.throttling.thermal",
            "cpu.throttling.power",
            "cpu.performance_limit",
            "cpu.cores.parked",
        ] {
            let metric = m.telemetry.get(id).unwrap_or_else(|| panic!("{id} sumiu"));
            assert_eq!(
                metric.quality,
                Quality::Measured,
                "{id}: {:?}",
                metric.reason
            );
            assert_eq!(metric.source, "pdh", "{id}");
        }

        // Só empatam com a CPU cravada em 100%: o empate denunciou a janela de PDH de zero segundo.
        let reportado = m.telemetry.value("cpu.clock.reported").expect("medido");
        let efetivo = m.telemetry.value("cpu.clock.effective").expect("medido");
        assert!(
            efetivo <= reportado + 0.01,
            "efetivo {efetivo} > reportado {reportado}"
        );
        assert!(reportado > 0.0);
    }

    /// Esta máquina publica 27,85 °C fixos: afirma a INVARIANTE (valor presente ⟺ qualidade não UNKNOWN), não um
    /// número.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn zona_termica_travada_e_descartada_ao_vivo() {
        let mut monitor = PerformanceMonitor::new();

        let mut ultima = None;
        for _ in 0..9 {
            ultima = Some(monitor.collect_metrics().await.expect("coleta"));
        }
        let m = ultima.expect("nove coletas");

        let metric = m.telemetry.get("cpu.temperature").expect("catálogo");
        assert_eq!(
            metric.value.is_some(),
            m.cpu.temperature.is_some(),
            "o campo antigo e o contrato precisam concordar"
        );

        match metric.quality {
            Quality::Unknown => {
                assert!(metric.value.is_none());
                assert!(metric.reason.is_some());
            }
            _ => {
                let c = metric.value.expect("tem valor");
                assert!((5.0..=120.0).contains(&c), "{c} °C não é leitura de CPU");
                assert!(
                    !metric
                        .reason
                        .as_deref()
                        .unwrap_or("")
                        .contains("ainda sem leituras"),
                    "a janela já fechou"
                );
            }
        }
    }

    #[tokio::test]
    async fn o_que_esta_maquina_mede_de_fato() {
        let mut monitor = PerformanceMonitor::new();
        let m = monitor.collect_metrics().await.expect("coleta");

        for id in [
            "cpu.usage.overall",
            "cpu.cores.logical",
            "ram.total",
            "ram.usage",
        ] {
            let metric = m.telemetry.get(id).unwrap_or_else(|| panic!("{id} sumiu"));
            assert_eq!(metric.quality, Quality::Measured, "{id}");
        }

        assert!(!m.cpu.per_core.is_empty(), "nenhum núcleo listado");
        assert_eq!(
            m.telemetry.value("cpu.cores.logical"),
            Some(m.cpu.per_core.len() as f64)
        );
        assert!(m.telemetry.summary.measured > 0);
        assert_eq!(
            m.telemetry.summary.total,
            m.telemetry.metrics.len(),
            "o resumo conta a coleta inteira"
        );
    }

    #[tokio::test]
    async fn segunda_leitura_passa_a_ter_intervalo() {
        let mut monitor = PerformanceMonitor::new();
        let primeira = monitor.collect_metrics().await.expect("coleta");
        assert_eq!(primeira.telemetry.since_previous_ms, None);

        let segunda = monitor.collect_metrics().await.expect("coleta");
        let intervalo = segunda
            .telemetry
            .since_previous_ms
            .expect("a segunda leitura tem anterior");

        assert!(
            intervalo >= ESPERA_DE_AMOSTRAGEM_MS,
            "intervalo de {intervalo} ms"
        );

        assert!(segunda.telemetry.collection_duration_ms >= ESPERA_DE_AMOSTRAGEM_MS);
    }

    /// `cargo test -- --nocapture`. Se medidas, estimadas e desconhecidas não fecham o total, alguma métrica ficou fora.
    #[tokio::test]
    async fn retrato_desta_maquina() {
        let mut monitor = PerformanceMonitor::new();

        // Várias coletas: o painel depois de alguns segundos aberto, não o pior caso.
        let mut m = monitor.collect_metrics().await.expect("coleta");
        let limite = std::time::Instant::now() + std::time::Duration::from_secs(25);
        while std::time::Instant::now() < limite && m.telemetry.value("display.refresh").is_none() {
            m = monitor.collect_metrics().await.expect("coleta");
        }
        let s = m.telemetry.summary;

        println!(
            "medidas {} · estimadas {} · desconhecidas {} · total {} · coleta {} ms",
            s.measured, s.estimated, s.unknown, s.total, m.telemetry.collection_duration_ms
        );

        for (id, metric) in &m.telemetry.metrics {
            println!(
                "  {id:<28} {:?} {:?} {}",
                metric.quality,
                metric.value,
                metric.reason.as_deref().unwrap_or("")
            );
        }

        println!(
            "\ngargalo: {:?} · {} de {} classes avaliadas",
            m.gargalo.conclusao, m.gargalo.classes_avaliadas, m.gargalo.classes_totais
        );
        for a in &m.gargalo.achados {
            println!("  {:?} [{:?}] {}", a.classe, a.forca, a.evidencia);
        }
        for n in &m.gargalo.nao_verificado {
            println!("  não verificado: {} — {}", n.classe, n.falta);
        }

        assert_eq!(s.measured + s.estimated + s.unknown, s.total);
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn a_placa_e_lida_agora_e_nao_envelhecida() {
        let mut monitor = PerformanceMonitor::new();

        let m = monitor.collect_metrics().await.expect("coleta");

        for id in [
            "gpu.usage",
            "vram.used",
            "vram.usage",
            "storage.busy",
            "storage.latency",
        ] {
            let metric = m.telemetry.get(id).unwrap_or_else(|| panic!("{id} sumiu"));

            // Sem o contador, "não sei", nunca zero: aceita os dois desfechos, coerentes.
            match metric.quality {
                Quality::Unknown => assert!(metric.value.is_none() && metric.reason.is_some()),
                _ => {
                    assert_eq!(metric.quality, Quality::Measured, "{id} não é estimativa");
                    assert_eq!(metric.source, "pdh", "{id}");
                    assert_eq!(metric.age_ms, None, "{id} é desta coleta, não tem idade");
                }
            }
        }
    }

    /// Sem jogo, a sonda não mede contra ninguém: as três métricas ficam desconhecidas COM motivo.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn sem_jogo_a_rede_nao_inventa_numero() {
        let mut monitor = PerformanceMonitor::new();

        let limite = std::time::Instant::now() + std::time::Duration::from_secs(40);
        let mut ultima = monitor.collect_metrics().await.expect("coleta");

        while std::time::Instant::now() < limite {
            let m = monitor.collect_metrics().await.expect("coleta");
            let motivo = m
                .telemetry
                .get("network.packet_loss")
                .and_then(|x| x.reason.clone())
                .unwrap_or_default();

            ultima = m;
            if !motivo.contains("ainda não voltou") {
                break;
            }
        }

        for id in ["network.latency", "network.jitter", "network.packet_loss"] {
            let metric = ultima.telemetry.get(id).expect("catálogo");

            match metric.quality {
                Quality::Unknown => {
                    assert!(metric.value.is_none(), "{id} é UNKNOWN e tem valor");
                    assert!(metric.reason.is_some(), "{id} não diz por quê");
                }
                _ => {
                    assert!(metric.value.is_some(), "{id} tem qualidade sem valor");
                    assert_eq!(metric.source, "icmp", "{id}");
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn o_modo_de_video_chega_pela_tarefa_de_fundo() {
        let mut monitor = PerformanceMonitor::new();

        let primeira = monitor.collect_metrics().await.expect("coleta");
        let hz = primeira.telemetry.get("display.refresh").expect("catálogo");
        assert_eq!(
            hz.quality,
            Quality::Unknown,
            "a primeira leitura não voltou ainda"
        );

        let limite = std::time::Instant::now() + std::time::Duration::from_secs(25);
        while std::time::Instant::now() < limite {
            let m = monitor.collect_metrics().await.expect("coleta");
            if let Some(v) = m.telemetry.value("display.refresh") {
                assert!((20.0..=1000.0).contains(&v), "{v} Hz não é taxa de monitor");
                return;
            }
        }
    }

    #[test]
    fn leitura_velha_deixa_de_ser_medicao() {
        use crate::modules::telemetry::Metric;

        let fresca = Metric::measured(80.0, Unit::Percent, "wmi");
        assert_eq!(fresca.quality, Quality::Measured);
        assert_eq!(fresca.age_ms, None);

        let velha = fresca.com_idade(12_000);
        assert_eq!(velha.quality, Quality::Estimated);
        assert_eq!(velha.age_ms, Some(12_000));
        assert_eq!(
            velha.value,
            Some(80.0),
            "o valor continua sendo o que foi lido"
        );
        assert!(velha.reason.unwrap().contains("12 s"));

        let ausente = Metric::unknown(Unit::Percent, "sem provedor").com_idade(9_000);
        assert_eq!(ausente.age_ms, None);
        assert_eq!(ausente.quality, Quality::Unknown);
    }

    #[tokio::test]
    async fn o_gargalo_acompanha_a_coleta() {
        use crate::modules::gargalo::{Conclusao, Forca};

        let mut monitor = PerformanceMonitor::new();
        let m = monitor.collect_metrics().await.expect("coleta");
        let g = &m.gargalo;

        assert_ne!(g.conclusao, Conclusao::SemEvidencia);
        assert!(g.classes_avaliadas > 0);

        assert!(g.classes_avaliadas < g.classes_totais);
        assert!(!g.nao_verificado.is_empty());

        for a in &g.achados {
            assert!(!a.evidencia.is_empty(), "achado sem evidência");
            if a.forca == Forca::Causa {
                assert!(
                    a.idade_ms.is_none_or(|ms| ms <= 5_000),
                    "causa apoiada em leitura velha"
                );
            }
        }
    }

    /// Coleta → contrato → baseline → comparação, sem mexer em nada: nenhuma métrica desconhecida pode virar delta.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn dois_retratos_seguidos_comparam_sem_inventar() {
        use crate::modules::baseline::{comparar, Baseline, Identidade, Perfil};

        let mut monitor = PerformanceMonitor::new();

        let identidade = Identidade {
            cpu: "teste".into(),
            ..Default::default()
        };

        let primeira = monitor.collect_metrics().await.expect("coleta");
        let antes = Baseline::novo(0, Perfil::Ocioso, identidade.clone(), primeira.telemetry);

        let segunda = monitor.collect_metrics().await.expect("coleta");
        let depois = Baseline::novo(1, Perfil::Ocioso, identidade, segunda.telemetry);

        let c = comparar(&antes, &depois).expect("mesma máquina, mesmo perfil");

        assert!(!c.deltas.is_empty(), "alguma coisa foi medida nos dois");

        for d in &c.deltas {
            assert!(d.antes.is_finite() && d.depois.is_finite(), "{}", d.id);
            assert!(d.firme || d.ressalva.is_some(), "{} sem ressalva", d.id);
        }

        for n in &c.nao_comparaveis {
            assert!(!n.motivo.is_empty(), "{} sem motivo", n.id);
        }

        assert!(
            !c.deltas.iter().any(|d| d.id == "gpu.temperature"),
            "sensor sem provedor não pode virar diferença"
        );
    }

    #[test]
    fn uptime_e_positivo() {
        assert!(uptime_hours() >= 0.0);
    }
}
