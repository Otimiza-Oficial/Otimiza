// Monitor de desempenho — a coleta contínua da tela principal
//
// Toda leitura daqui sai também pelo contrato de `telemetry.rs`, com origem e
// qualidade. Os campos antigos continuam existindo para a tela que já os lê,
// mas os que antes eram sempre número agora são OPCIONAIS: o que não foi medido
// chega como `null`, e não como zero. O cabeçalho de `telemetry.rs` conta por
// que essa diferença é o ponto de partida do produto inteiro.
//
// O QUE ESTE COLETOR MEDE DE VERDADE
//
// Pelo sysinfo: uso de CPU agregado e por núcleo, memória, capacidade dos
// volumes, taxa de rede e uptime.
//
// Pelos contadores de desempenho do Windows, no Windows: clock REPORTADO e
// clock EFETIVO — que são grandezas diferentes e por isso têm ids diferentes —,
// as duas flags de limite de firmware, o `% Performance Limit` e quantos
// núcleos estão estacionados. É o mesmo amostrador que o motor de energia usa
// para decidir se um plano de energia fica ou é revertido; a tela só não tinha
// acesso a ele.
//
// Temperatura sai como ESTIMATED: é zona térmica ACPI, que pode não ser o
// sensor do processador, e muitas placas publicam um valor fixo. A janela de
// leituras aqui aplica a mesma regra do motor de energia e DESCARTA a zona que
// não se move sob carga.
//
// Uso da placa, VRAM em uso, ocupação do disco e frequência do monitor saem de
// uma leitura CARA — a consulta ao WMI custa mais de um segundo. Ela roda fora
// do laço, a cada dez segundos, e o que a tela recebe é a última leitura com a
// IDADE escrita no contrato. Passados trinta segundos ela é descartada: um uso
// de GPU de meio minuto atrás não descreve mais esta máquina.
//
// Sensor da placa (clock, temperatura, potência), potência de pacote da CPU,
// latência, entrada e quadros saem como UNKNOWN, cada um dizendo o que
// exigiria. Quadros não são "sem provedor": o Otimiza os mede por ETW em
// `windows::frames`, durante a partida — só não neste laço.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use sysinfo::{Disks, Networks, System};

use super::telemetry::{id_do_nucleo, Metric, Telemetry, Unit};

#[derive(Debug, Serialize, Deserialize)]
pub struct CPUMetrics {
    /// Média do uso dos núcleos, 0-100%. `None` quando a leitura não fecha.
    pub overall: Option<f32>,
    pub per_core: Vec<f32>,
    /// Sem provedor neste coletor. Era `0.0` com um comentário "Placeholder"
    /// ao lado, o que chegava à tela como zero grau.
    pub temperature: Option<f32>,
    /// Clock informado pelo sistema, em MHz. Não é clock efetivo.
    pub frequency: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RAMMetrics {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    /// Sem provedor neste coletor.
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
    /// `None` na primeira leitura da sessão e sempre que os contadores não
    /// permitem distinguir disco parado de leitura que falhou.
    pub read_speed_mbps: Option<f64>,
    pub write_speed_mbps: Option<f64>,
    /// Espaço ocupado, não atividade.
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
    /// Horas desde o último boot. Ver `uptime_hours`.
    pub uptime_hours: f64,
    /// A mesma coleta, com procedência por métrica. Ver `telemetry.rs`.
    pub telemetry: Telemetry,
    /// Qual recurso está no limite, segundo esta coleta. Ver `gargalo.rs`.
    ///
    /// Vem junto e não num comando separado de propósito: classificar é uma
    /// função pura da telemetria que acabou de ser lida. Num segundo comando,
    /// a tela mostraria um diagnóstico de uma coleta e números de outra.
    pub gargalo: super::gargalo::Diagnostico,
}

/// Quanto a amostragem de CPU espera entre as duas leituras.
///
/// Uso de CPU é diferença entre dois instantes; uma única leitura não tem com o
/// que comparar. O `sysinfo` exige pelo menos 200 ms entre os dois refreshes
/// para o número significar alguma coisa.
const ESPERA_DE_AMOSTRAGEM_MS: u64 = 200;

/// Intervalo mínimo entre coletas para que uma taxa seja calculável.
///
/// Duas chamadas coladas dividiriam por um número perto de zero e produziriam
/// uma taxa absurda.
const INTERVALO_MINIMO_S: f64 = 0.05;

pub struct PerformanceMonitor {
    monitoring_active: bool,
    system: System,
    /// Mantidos vivos entre chamadas: velocidade de disco e de rede só existe
    /// como diferença entre duas leituras. Um coletor recriado a cada chamada
    /// não tem passado com o que comparar.
    disks: Disks,
    networks: Networks,
    last_sample: Option<std::time::Instant>,
    /// Quais dispositivos existiam na leitura anterior.
    ///
    /// Se a lista muda — pendrive plugado, volume montado, adaptador ligado —
    /// a diferença entre as duas leituras deixa de ser sobre o mesmo conjunto,
    /// e a taxa calculada em cima dela não quer dizer nada.
    discos_vistos: Option<BTreeSet<String>>,
    interfaces_vistas: Option<BTreeSet<String>>,
    /// Contadores de desempenho do Windows, vivos entre as coletas.
    ///
    /// É o mesmo amostrador que o motor de energia usa para decidir se um
    /// plano fica ou é revertido (`motorenergia_maquina::Amostrador`). Ele já
    /// existia e já era testado; o que faltava era a tela ter acesso ao que
    /// ele mede — clock EFETIVO, temperatura e os limites de firmware ficavam
    /// desconhecidos no painel enquanto o provedor rodava no mesmo executável.
    ///
    /// Vive entre as chamadas porque o PDH entrega a diferença entre dois
    /// `PdhCollectQueryData`: recriado a cada coleta, ele mediria uma janela
    /// de zero segundo.
    #[cfg(target_os = "windows")]
    contadores: Option<crate::modules::windows::motorenergia_maquina::Amostrador>,
    /// As últimas leituras dos contadores, guardadas para julgar a zona
    /// térmica.
    ///
    /// Uma leitura sozinha não distingue "28 °C" de "esta placa publica 28 °C
    /// o dia inteiro". A regra que separa os dois já existe e já é testada em
    /// `motorenergia::resumir_cpu`; o que faltava era ter amostras suficientes
    /// para aplicá-la. Com uma coleta a cada poucos segundos, a janela fecha em
    /// menos de meio minuto de painel aberto.
    #[cfg(target_os = "windows")]
    amostras_recentes:
        std::collections::VecDeque<crate::modules::windows::motorenergia::AmostraCpu>,
    /// A última leitura cara, com a hora em que foi feita.
    ///
    /// Compartilhada com a tarefa de fundo que a atualiza — por isso o `Arc`.
    /// A coleta nunca espera por ela: serve o que tem, com a idade escrita.
    #[cfg(target_os = "windows")]
    placa: std::sync::Arc<std::sync::Mutex<Option<(std::time::Instant, LeituraLenta)>>>,
    /// Uma leitura cara já está em curso.
    ///
    /// Sem isto, um laço atrasado dispararia uma consulta ao WMI por coleta e
    /// elas se empilhariam — o mesmo defeito que a tela já tinha no `tick`.
    #[cfg(target_os = "windows")]
    placa_em_curso: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Memória total da placa, lida do registro uma vez.
    ///
    /// É fato de hardware: não muda enquanto o programa estiver aberto, e
    /// reler a cada coleta seria oito consultas ao registro por nada.
    #[cfg(target_os = "windows")]
    vram_total_gb: Option<Option<f64>>,
}

/// Quantas leituras a janela térmica guarda.
///
/// `resumir_cpu` só julga a zona travada com oito ou mais; doze dá alguma
/// folga sem virar histórico.
#[cfg(target_os = "windows")]
const JANELA_TERMICA: usize = 12;

/// O que só dá para ler devagar.
///
/// Placa de vídeo, VRAM em uso e ocupação do disco saem de uma consulta ao WMI
/// que custa MAIS DE UM SEGUNDO — o comentário em `bottleneck.rs` já dizia
/// isso. A frequência do monitor vem junto por ser barata e igualmente estável.
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, Default)]
struct LeituraLenta {
    gpu_pct: Option<f64>,
    vram_usada_gb: Option<f64>,
    disco_pct: Option<f64>,
    hz: Option<u32>,
}

/// De quanto em quanto tempo a leitura cara é refeita.
///
/// Dez segundos. A consulta custa mais de um segundo e roda fora do laço; a
/// cada dois segundos, o monitor passaria boa parte do tempo sendo a carga que
/// veio medir.
#[cfg(target_os = "windows")]
const INTERVALO_DA_PLACA: std::time::Duration = std::time::Duration::from_secs(10);

/// Depois disto a leitura para de valer.
///
/// Trinta segundos é tempo de um jogo abrir, fechar, ou a cena mudar por
/// inteiro. Passado isso o número não descreve mais esta máquina, e some.
#[cfg(target_os = "windows")]
const VALIDADE_DA_PLACA: std::time::Duration = std::time::Duration::from_secs(30);

impl PerformanceMonitor {
    pub fn new() -> Self {
        PerformanceMonitor {
            monitoring_active: false,
            // `System::new()` e não `new_all()`: este coletor não olha
            // processos, e `new_all` varre a tabela inteira deles no arranque.
            system: System::new(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_sample: None,
            discos_vistos: None,
            interfaces_vistas: None,
            // Aberto na primeira coleta, não aqui: abrir a consulta do PDH
            // custa, e o monitor é construído mesmo em sessão que nunca vai
            // olhar o painel.
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

    /// Coleta snapshot das métricas atuais.
    ///
    /// Não há `refresh_all` aqui. Ele varria a tabela de processos a cada
    /// coleta e logo em seguida cada coletor refazia o refresh específico de
    /// que precisava — a varredura cara acontecia e era jogada fora. Um
    /// monitor de desempenho que pesa no desempenho é o pior defeito possível
    /// neste produto.
    pub async fn collect_metrics(&mut self) -> Result<PerformanceMetrics, String> {
        let comeco = std::time::Instant::now();

        // O intervalo é medido uma vez e passado para os dois coletores: se
        // cada um marcasse o próprio tempo, eles dividiriam os respectivos
        // deltas por janelas diferentes da mesma leitura.
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

        let telemetry = telemetry.finish(comeco.elapsed().as_millis() as u64);
        let gargalo = super::gargalo::classificar(&telemetry);

        Ok(PerformanceMetrics {
            timestamp,
            cpu,
            ram,
            gpu,
            disk,
            network,
            uptime_hours: uptime,
            telemetry,
            gargalo,
        })
    }

    async fn collect_cpu_metrics(&mut self, t: &mut Telemetry) -> CPUMetrics {
        // ANTES da espera, não depois.
        //
        // Um contador do PDH é a diferença entre duas consultas, e abrir a
        // consulta já conta como a primeira. Abrindo aqui, a leitura lá
        // embaixo cobre pelo menos a espera de amostragem; abrindo junto da
        // leitura, a janela seria de zero segundo e o Windows devolveria
        // números sem sentido — `% Processor Time` sai 100 e o clock efetivo
        // empata com o reportado, que foi exatamente o que apareceu quando
        // esta ponte foi ligada pela primeira vez.
        #[cfg(target_os = "windows")]
        if self.contadores.is_none() {
            self.contadores = crate::modules::windows::motorenergia_maquina::Amostrador::novo();
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

        // Sem núcleo nenhum a média seria uma divisão por zero, e o resultado
        // viajaria como NaN. Não é um caso que aconteça numa máquina sã — é o
        // caso em que o coletor precisa dizer que não sabe.
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

        // Frequência do primeiro núcleo, em MHz.
        //
        // ESTIMATED, e não MEASURED, por duas razões que não se resolvem aqui:
        // é o clock que o sistema informa a partir da tabela de frequências —
        // não o clock efetivo que os núcleos sustentaram no período — e é o de
        // um núcleo só, o que em CPU híbrida nem representa os outros.
        // `cpu.clock.effective` fica UNKNOWN de propósito: são grandezas
        // diferentes, e preencher uma com a outra seria inventar.
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

        // Os contadores do Windows entram por cima do que o sysinfo deu: eles
        // medem o que o sysinfo não alcança e, no caso do clock, medem melhor.
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

    /// Enche o contrato com o que os contadores de desempenho do Windows
    /// sabem, e devolve a temperatura para o campo antigo da tela.
    ///
    /// UMA AMOSTRA SÓ. O motor de energia coleta dezenas antes de decidir
    /// qualquer coisa, e é assim que ele descarta zona térmica travada —
    /// precisa de oito leituras para ver que o número não se move. Aqui há uma
    /// leitura por coleta, então a temperatura sai como ESTIMATED dizendo isso:
    /// serve para a tela, não serve para fechar veredito térmico.
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

        // O clock reportado pelo PDH substitui o do sysinfo: é medido sobre o
        // intervalo entre duas coletas, e não a entrada da tabela de
        // frequências para o primeiro núcleo.
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

    /// Decide o que dizer sobre a temperatura, com a janela de leituras.
    ///
    /// A regra é a de `motorenergia::resumir_cpu`, reusada inteira em vez de
    /// reescrita: com carga na máquina e uma zona térmica que não se move
    /// nada, a leitura é descartada. Muitas placas publicam uma zona ACPI fixa
    /// — esta aqui publica 27,85 °C com a CPU a 50% —, e uma folga térmica
    /// tirada de um número congelado é pior do que não ter folga nenhuma.
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
            // Janela fechada e a zona se mexeu: vale como leitura.
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
            // Janela fechada e o número não se moveu sob carga: não é sensor.
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

        // Sem memória total não existe porcentagem. Zero seria dizer que a
        // máquina está com a RAM livre.
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

    /// GPU e VRAM não têm provedor neste coletor.
    ///
    /// Ler isso de verdade exige NVML, ADL ou o equivalente da Intel, cada um
    /// presente só na máquina com aquela placa. Até lá, o contrato diz que não
    /// sabe — o que é diferente de a GPU não aparecer na resposta.
    fn collect_gpu_metrics(&mut self, t: &mut Telemetry) -> Option<GPUMetrics> {
        // Clock, temperatura e potência da placa continuam sem quem leia: o
        // contador do Windows entrega uso e memória, não sensor. Isso exige
        // NVML, ADL ou o equivalente da Intel, cada um só presente na máquina
        // com aquela placa.
        const SEM_SENSOR: &str = "exige NVML/ADL/Intel; o contador do Windows não expõe sensor";
        t.set("gpu.clock", Metric::unknown(Unit::Megahertz, SEM_SENSOR));
        t.set(
            "gpu.temperature",
            Metric::unknown(Unit::Celsius, SEM_SENSOR),
        );
        t.set("gpu.power", Metric::unknown(Unit::Watts, SEM_SENSOR));

        #[cfg(target_os = "windows")]
        self.telemetria_da_placa(t);

        #[cfg(not(target_os = "windows"))]
        {
            const FORA: &str = "os contadores de placa de vídeo são do Windows";
            t.set("gpu.usage", Metric::unknown(Unit::Percent, FORA));
            t.set("vram.used", Metric::unknown(Unit::Gigabytes, FORA));
            t.set("vram.total", Metric::unknown(Unit::Gigabytes, FORA));
            t.set("vram.usage", Metric::unknown(Unit::Percent, FORA));
            t.set("storage.busy", Metric::unknown(Unit::Percent, FORA));
            t.set("display.refresh", Metric::unknown(Unit::Hertz, FORA));
        }

        // O bloco `gpu` antigo da resposta exigia utilização, memória,
        // temperatura, ventoinha e potência ao mesmo tempo, todos como número
        // obrigatório. Não dá para preenchê-lo sem inventar três deles, então
        // ele continua ausente — o que a placa entrega vai pelo contrato, onde
        // cada campo pode faltar sozinho.
        None
    }

    /// Dispara a leitura cara quando é hora, e publica a última que existe.
    ///
    /// A coleta NUNCA espera pela consulta ao WMI. Ela olha o que a tarefa de
    /// fundo deixou, carimba a idade e segue — um painel que trava dois
    /// segundos para perguntar o uso da placa é um painel que atrapalha o
    /// jogo que ele está medindo.
    #[cfg(target_os = "windows")]
    fn telemetria_da_placa(&mut self, t: &mut Telemetry) {
        use std::sync::atomic::Ordering;

        let agora = std::time::Instant::now();
        let guardada = self.placa.lock().ok().and_then(|g| *g);

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

        // Total da placa: fato de hardware, lido do registro uma vez só.
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

        let Some((quando, leitura)) = guardada else {
            const PRIMEIRA: &str = "a primeira consulta aos contadores da placa ainda não voltou";
            t.set("gpu.usage", Metric::unknown(Unit::Percent, PRIMEIRA));
            t.set("vram.used", Metric::unknown(Unit::Gigabytes, PRIMEIRA));
            t.set("vram.usage", Metric::unknown(Unit::Percent, PRIMEIRA));
            t.set("storage.busy", Metric::unknown(Unit::Percent, PRIMEIRA));
            t.set("display.refresh", Metric::unknown(Unit::Hertz, PRIMEIRA));
            return;
        };

        let idade = agora.duration_since(quando);

        if idade > VALIDADE_DA_PLACA {
            let velha = format!(
                "a última leitura da placa tem {} s e não descreve mais esta máquina",
                idade.as_secs()
            );
            t.set("gpu.usage", Metric::unknown(Unit::Percent, velha.clone()));
            t.set("vram.used", Metric::unknown(Unit::Gigabytes, velha.clone()));
            t.set("vram.usage", Metric::unknown(Unit::Percent, velha.clone()));
            t.set("storage.busy", Metric::unknown(Unit::Percent, velha));
            // A frequência do monitor não estraga com o tempo do mesmo jeito:
            // ela muda quando alguém troca o modo de vídeo, não sozinha.
            marcar_hz(t, leitura.hz, idade);
            return;
        }

        let idade_ms = idade.as_millis() as u64;
        let com_idade = |m: Metric| m.com_idade(idade_ms);

        match leitura.gpu_pct {
            Some(pct) => t.set(
                "gpu.usage",
                com_idade(Metric::measured(pct, Unit::Percent, "wmi").require_range(0.0, 100.0)),
            ),
            None => t.set(
                "gpu.usage",
                Metric::unknown(Unit::Percent, "o contador de motores 3D não respondeu"),
            ),
        }

        match leitura.disco_pct {
            Some(pct) => t.set(
                "storage.busy",
                com_idade(Metric::measured(pct, Unit::Percent, "wmi").require_range(0.0, 100.0)),
            ),
            None => t.set(
                "storage.busy",
                Metric::unknown(Unit::Percent, "o contador de disco físico não respondeu"),
            ),
        }

        match leitura.vram_usada_gb {
            Some(gb) => {
                t.set(
                    "vram.used",
                    com_idade(Metric::measured(gb, Unit::Gigabytes, "wmi")),
                );

                match total {
                    Some(tot) if tot > 0.0 => t.set(
                        "vram.usage",
                        com_idade(
                            Metric::measured(gb / tot * 100.0, Unit::Percent, "wmi")
                                .require_range(0.0, 100.0),
                        ),
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

        marcar_hz(t, leitura.hz, idade);
    }

    /// Segundos desde a leitura anterior, ou `None` na primeira.
    fn elapsed_since_last(&mut self) -> Option<f64> {
        let agora = std::time::Instant::now();
        let anterior = self.last_sample.replace(agora)?;

        let segundos = agora.duration_since(anterior).as_secs_f64();

        (segundos >= INTERVALO_MINIMO_S).then_some(segundos)
    }

    fn collect_disk_metrics(&mut self, elapsed: Option<f64>, t: &mut Telemetry) -> DiskMetrics {
        // A lista é mantida entre chamadas de propósito: os bytes lidos e
        // gravados que o sysinfo entrega são a diferença desde o refresh
        // anterior. Recriar a lista a cada leitura zera essa diferença.
        self.disks.refresh(true);

        let mut total_space = 0u64;
        let mut used_space = 0u64;
        let mut read_bytes = 0u64;
        let mut written_bytes = 0u64;
        let mut agora: BTreeSet<String> = BTreeSet::new();
        // Um mesmo volume pode aparecer duas vezes na lista, montado em dois
        // lugares. Somar os dois contaria o espaço em dobro.
        let mut volumes_contados: BTreeSet<String> = BTreeSet::new();

        for disk in &self.disks {
            let ponto = disk.mount_point().to_string_lossy().to_string();
            agora.insert(ponto.clone());

            let usage = disk.usage();
            read_bytes += usage.read_bytes;
            written_bytes += usage.written_bytes;

            // Pendrive e imagem montada em modo leitura não são o disco do
            // cliente, e um ISO montado está sempre 100% cheio — entrar na
            // conta só puxaria o número para cima sem dizer nada sobre a
            // máquina.
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
            // ESTIMATED: é a soma de todos os volumes fixos tratada como um
            // número só. Dois volumes no mesmo disco físico entram separados,
            // e um disco cheio dividido com outro vazio some na média.
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
                    // ESTIMATED e não MEASURED: o sysinfo não devolve erro quando o
                    // IOCTL de contador falha num volume — ele devolve zero. Um
                    // total que avançou prova que ALGUM volume respondeu, não que
                    // todos responderam.
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

        t.set(
            "storage.latency",
            Metric::unknown(
                Unit::Milliseconds,
                "exige contador de desempenho por disco; sem provedor nesta versão",
            ),
        );

        DiskMetrics {
            read_speed_mbps: read,
            write_speed_mbps: write,
            usage_percent,
        }
    }

    /// Taxa de disco, ou o motivo de não haver taxa.
    ///
    /// O caso importante é o último: quando nenhum contador avançou, o produto
    /// NÃO diz "0 MB/s". Os contadores do sysinfo no Windows devolvem zero
    /// tanto para disco realmente parado quanto para consulta que falhou, e não
    /// há como separar os dois daqui. Afirmar "parado" com base nisso seria
    /// inventar a metade da informação que falta.
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
        // Mesma razão do disco: `received()` é o que chegou desde o refresh
        // anterior, então a lista precisa sobreviver entre as chamadas.
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

                // Aqui MEASURED é honesto: interface que falha some da lista do
                // sysinfo em vez de reportar zero, então zero recebido é zero
                // recebido.
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

        // Latência, jitter e perda de pacote são três perguntas distintas, e
        // nenhuma delas se responde contando bytes. Ficam declaradas como não
        // medidas para que ninguém as confunda com a taxa acima.
        const SEM_SONDA: &str = "exige sonda de rede; sem provedor nesta versão";
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

/// Diz, com precisão, por que as métricas que faltam faltam.
///
/// O motivo padrão do catálogo é "sem provedor de medição nesta versão", e
/// para quadros isso seria falso: o Otimiza MEDE quadros, por ETW, em
/// `windows::frames`. Só que mede durante uma partida, com o jogo em primeiro
/// plano e o programa como administrador — não num laço de painel a cada dois
/// segundos. Dizer "não existe" sobre algo que existe em outro lugar manda o
/// próximo módulo construir um provedor que já está construído.
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

/// A frequência do monitor principal.
///
/// Não envelhece como o uso da placa: ela só muda quando alguém troca o modo
/// de vídeo. A idade vai junto mesmo assim, porque quem lê o contrato decide
/// sozinho o que é velho demais para o que está fazendo.
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

/// A leitura cara, fora do laço de coleta.
///
/// Roda numa thread de bloqueio porque a consulta ao WMI abre um PowerShell e
/// leva mais de um segundo.
#[cfg(target_os = "windows")]
fn ler_placa_devagar() -> LeituraLenta {
    use crate::modules::windows::{bottleneck, display};

    let bruto = bottleneck::amostrar_wmi();

    let hz = display::monitores()
        .into_iter()
        .find(|m| m.principal)
        .map(|m| m.hz_atual);

    LeituraLenta {
        gpu_pct: bruto.gpu,
        vram_usada_gb: bruto.vram_mb.map(|mb| mb / 1024.0),
        disco_pct: bruto.disco,
        hz,
    }
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1_073_741_824.0
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
    use crate::modules::telemetry::{Quality, SCHEMA_VERSION};

    #[tokio::test]
    async fn coleta_real_classifica_tudo_o_que_entrega() {
        let mut monitor = PerformanceMonitor::new();
        let m = monitor.collect_metrics().await.expect("coleta");

        assert_eq!(m.telemetry.schema_version, SCHEMA_VERSION);

        // Nenhuma métrica com valor pode estar marcada como desconhecida, e
        // nenhuma desconhecida pode carregar valor. É a invariante que sustenta
        // o resto: quem lê `value` sabe que alguém mediu aquilo.
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

        // O defeito que este módulo veio consertar: na primeira leitura não há
        // leitura anterior, e o coletor antigo devolvia `(0.0, 0.0)` — "parado".
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
            "gpu.usage",
            "vram.used",
            "fps.rendered",
            "fps.generated",
            "fps.displayed",
            "frametime.p99",
            "storage.latency",
            "network.latency",
            "input.mouse_polling",
            "display.refresh",
        ] {
            let metric = m.telemetry.get(id).unwrap_or_else(|| panic!("{id} sumiu"));
            assert_eq!(metric.quality, Quality::Unknown, "{id}");
            assert!(metric.value.is_none(), "{id}");
        }
    }

    /// O que os contadores de desempenho do Windows passaram a entregar.
    ///
    /// Clock EFETIVO, limites de firmware e estacionamento de núcleo já eram
    /// medidos pelo motor de energia no mesmo executável; a tela é que não
    /// tinha acesso. Este teste é a garantia de que a ponte não se solte.
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

        // Efetivo é reportado DESCONTADO o tempo parado. Só empatam com a CPU
        // cravada em 100%, o que não acontece numa coleta de teste — e foi
        // exatamente o empate que denunciou a janela de PDH com zero segundo.
        let reportado = m.telemetry.value("cpu.clock.reported").expect("medido");
        let efetivo = m.telemetry.value("cpu.clock.effective").expect("medido");
        assert!(
            efetivo <= reportado + 0.01,
            "efetivo {efetivo} > reportado {reportado}"
        );
        assert!(reportado > 0.0);
    }

    /// Zona térmica travada não vira temperatura.
    ///
    /// Esta máquina publica 27,85 °C fixos, então aqui o caminho testado é o
    /// do descarte. Em máquina com sensor de verdade o teste passa pelo outro
    /// lado — por isso ele afirma a INVARIANTE (valor presente ⟺ qualidade
    /// diferente de UNKNOWN), e não um número.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn zona_termica_travada_e_descartada_ao_vivo() {
        let mut monitor = PerformanceMonitor::new();

        let mut ultima = None;
        // A janela precisa de oito leituras para julgar.
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
                // Com a janela cheia, o motivo não pode mais ser o de espera.
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

        // Uso de CPU, núcleos e memória são o piso: se nem isso sair medido, o
        // coletor não está coletando.
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

        // A espera de amostragem da CPU já garante esse piso.
        assert!(
            intervalo >= ESPERA_DE_AMOSTRAGEM_MS,
            "intervalo de {intervalo} ms"
        );

        // A duração da coleta INCLUI essa espera — está documentado no contrato
        // justamente para ninguém a ler como overhead.
        assert!(segunda.telemetry.collection_duration_ms >= ESPERA_DE_AMOSTRAGEM_MS);
    }

    /// Retrato do que esta máquina entrega, impresso para quem for depurar.
    ///
    /// Roda com `cargo test -- --nocapture` para ver a lista. A asserção é
    /// sobre a soma: se medidas, estimadas e desconhecidas não fecham o total,
    /// alguma métrica ficou fora da contagem e o painel de evidências estaria
    /// mostrando um número que não corresponde à coleta.
    #[tokio::test]
    async fn retrato_desta_maquina() {
        let mut monitor = PerformanceMonitor::new();

        // Várias coletas, e não uma: é o painel depois de alguns segundos
        // aberto, com a janela térmica fechada e a leitura cara já de volta.
        // Uma coleta só mostraria o pior caso e não o estado normal.
        let mut m = monitor.collect_metrics().await.expect("coleta");
        let limite = std::time::Instant::now() + std::time::Duration::from_secs(25);
        while std::time::Instant::now() < limite && m.telemetry.value("gpu.usage").is_none() {
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

    /// A leitura cara chega, e chega com a idade escrita.
    ///
    /// A consulta ao WMI custa mais de um segundo e roda fora do laço. Até ela
    /// voltar, a métrica é UNKNOWN dizendo que a primeira consulta não chegou
    /// — nunca um zero no lugar. Quando chega, vale como ESTIMATED: descreve
    /// um instante que já passou, e `age_ms` diz quanto.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn a_leitura_cara_chega_com_idade() {
        let mut monitor = PerformanceMonitor::new();

        let primeira = monitor.collect_metrics().await.expect("coleta");
        let gpu = primeira.telemetry.get("gpu.usage").expect("catálogo");
        assert_eq!(gpu.quality, Quality::Unknown, "a consulta ainda não voltou");
        assert_eq!(gpu.value, None, "e não vira zero enquanto isso");
        assert_eq!(gpu.age_ms, None, "não há leitura para envelhecer");

        // A tarefa de fundo abre um PowerShell; a primeira volta leva alguns
        // segundos. O limite é de tempo e não de tentativas, porque o número
        // de coletas que cabem nesse tempo muda com a carga da máquina.
        let limite = std::time::Instant::now() + std::time::Duration::from_secs(25);
        let mut chegou = None;
        while std::time::Instant::now() < limite {
            let m = monitor.collect_metrics().await.expect("coleta");
            if m.telemetry.value("gpu.usage").is_some() {
                chegou = Some(m);
                break;
            }
        }

        let Some(m) = chegou else {
            // Máquina sem os contadores de GPU do Windows. O contrato tem de
            // continuar dizendo isso, e não inventando um número.
            let m = monitor.collect_metrics().await.expect("coleta");
            let gpu = m.telemetry.get("gpu.usage").expect("catálogo");
            assert_eq!(gpu.quality, Quality::Unknown);
            assert!(gpu.reason.is_some());
            return;
        };

        let gpu = m.telemetry.get("gpu.usage").expect("catálogo");
        assert_eq!(gpu.source, "wmi");
        assert_eq!(
            gpu.quality,
            Quality::Estimated,
            "leitura de segundos atrás não é MEASURED"
        );
        assert!(gpu.age_ms.is_some(), "a idade precisa estar escrita");
        assert!(
            gpu.age_ms.unwrap() <= VALIDADE_DA_PLACA.as_millis() as u64,
            "leitura vencida não deveria ter sido publicada"
        );
        assert!((0.0..=100.0).contains(&gpu.value.unwrap()));
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

        // Não existe UNKNOWN envelhecido: não há leitura para ficar velha.
        let ausente = Metric::unknown(Unit::Percent, "sem provedor").com_idade(9_000);
        assert_eq!(ausente.age_ms, None);
        assert_eq!(ausente.quality, Quality::Unknown);
    }

    /// O diagnóstico vem da MESMA coleta que os números da tela.
    #[tokio::test]
    async fn o_gargalo_acompanha_a_coleta() {
        use crate::modules::gargalo::{Conclusao, Forca};

        let mut monitor = PerformanceMonitor::new();
        let m = monitor.collect_metrics().await.expect("coleta");
        let g = &m.gargalo;

        // Alguma coisa foi medida, então há classificação a fazer.
        assert_ne!(g.conclusao, Conclusao::SemEvidencia);
        assert!(g.classes_avaliadas > 0);

        // E a cobertura nunca finge estar completa: sete classes do produto
        // dependem de medição de quadros, latência ou rede.
        assert!(g.classes_avaliadas < g.classes_totais);
        assert!(!g.nao_verificado.is_empty());

        // Todo achado precisa citar a métrica que o sustenta.
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

    #[test]
    fn uptime_e_positivo() {
        assert!(uptime_hours() >= 0.0);
    }
}
