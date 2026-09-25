// Benchmark antes e depois, com veredito que pode ser "nenhuma melhora mensurável". Cada carga roda várias vezes e
// usa o MELHOR (o que menos sofreu interferência); cada métrica tem limiar de ruído; o baseline fica em disco para
// medir o que exige reiniciar.

use serde::{Deserialize, Serialize};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use sysinfo::System;

const WORKLOAD_DURATION: Duration = Duration::from_millis(500);
const WORKLOAD_ROUNDS: u32 = 5;
/// Sem a pausa, pega a máquina ocupada com o que veio antes.
const IDLE_SETTLE: Duration = Duration::from_secs(2);
const IDLE_SAMPLES: u32 = 10;
const IDLE_SAMPLE_INTERVAL: Duration = Duration::from_millis(500);
/// Comparar PC ocupado com PC descansado produz ganho fantasma de dezenas por cento.
const BUSY_CPU_PERCENT: f64 = 25.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSnapshot {
    pub timestamp: u64,
    pub idle_cpu_percent: f64,
    pub idle_ram_gb: f64,
    pub process_count: f64,
    pub cpu_single_thread_mops: f64,
    pub cpu_multi_thread_mops: f64,
    pub cpu_frequency_under_load_mhz: f64,
    /// `default` mantém legível o baseline gravado antes desta métrica.
    #[serde(default)]
    pub scheduler_p99_delay_ms: f64,
    #[serde(default)]
    pub hitches_per_minute: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Improved,
    Worsened,
    NoMeasurableChange,
    /// Oscila demais para atribuir variação à otimização: a CPU ociosa variou 244% sem nada mudar.
    TooNoisyToJudge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDelta {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub before: f64,
    pub after: f64,
    pub change_percent: f64,
    pub verdict: Verdict,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub before: BenchmarkSnapshot,
    pub after: BenchmarkSnapshot,
    pub metrics: Vec<MetricDelta>,
    pub summary: String,
}

struct MetricSpec {
    key: &'static str,
    label: &'static str,
    unit: &'static str,
    higher_is_better: bool,
    noise_percent: f64,
    /// Percentual sozinho engana com número pequeno (1,0 → 1,4 ms é 40%): precisa percentual E diferença absoluta.
    min_absolute_delta: f64,
    judgeable: bool,
    read: fn(&BenchmarkSnapshot) -> f64,
    explanation: &'static str,
}

/// Limiares do teste `noise_calibration` (mesma máquina três vezes, sem mudar nada) com ~50% de margem. Só
/// quatro das seis métricas sustentam veredito; as outras aparecem como referência.
static METRICS: &[MetricSpec] = &[
    MetricSpec {
        key: "cpu_single_thread_mops",
        label: "Desempenho de 1 núcleo",
        unit: "Mops/s",
        higher_is_better: true,
        noise_percent: 15.0,
        min_absolute_delta: 0.0,
        judgeable: true,
        read: |s| s.cpu_single_thread_mops,
        explanation: "Manda na maioria dos jogos. Num PC com programas abertos a medição varia até 11%, então só ganhos acima de 15% são reportados.",
    },
    MetricSpec {
        key: "cpu_multi_thread_mops",
        label: "Desempenho de todos os núcleos",
        unit: "Mops/s",
        higher_is_better: true,
        noise_percent: 0.0,
        min_absolute_delta: 0.0,
        // Viés SISTEMÁTICO: a segunda medição começa mais quente e a CPU reduz a frequência. No teste nulo marcou -19%.
        judgeable: false,
        read: |s| s.cpu_multi_thread_mops,
        explanation: "Mostrado apenas como referência: depende da temperatura da CPU no momento da medição, então não serve para provar ganho.",
    },
    MetricSpec {
        key: "cpu_frequency_under_load_mhz",
        label: "Frequência da CPU sob carga",
        unit: "MHz",
        higher_is_better: true,
        noise_percent: 3.0,
        min_absolute_delta: 0.0,
        judgeable: true,
        read: |s| s.cpu_frequency_under_load_mhz,
        explanation: "O indicador mais confiável do conjunto: sobe quando o plano de energia deixa de limitar a CPU.",
    },
    MetricSpec {
        key: "scheduler_p99_delay_ms",
        label: "Travada no pior caso",
        unit: "ms",
        higher_is_better: false,
        // Oscilou 49% sem nada mudar; o piso de 3 ms é um quinto de quadro, abaixo do perceptível.
        noise_percent: 60.0,
        min_absolute_delta: 3.0,
        judgeable: true,
        read: |s| s.scheduler_p99_delay_ms,
        explanation: "Quanto o Windows demora para devolver a CPU no pior 1% das vezes. É este atraso que vira engasgo no meio do jogo — e nenhuma média de FPS mostra ele.",
    },
    MetricSpec {
        key: "hitches_per_minute",
        label: "Engasgos por minuto",
        unit: "",
        higher_is_better: false,
        noise_percent: 50.0,
        // O teste nulo pegou 0 → 5 por minuto sem mudança; a partir de 6 (um a cada dez segundos) não é acaso.
        min_absolute_delta: 6.0,
        judgeable: true,
        read: |s| s.hitches_per_minute,
        explanation: "Pausas maiores que a duração de um quadro. Zero é o alvo.",
    },
    MetricSpec {
        key: "idle_cpu_percent",
        label: "CPU consumida em segundo plano",
        unit: "%",
        higher_is_better: false,
        noise_percent: 0.0,
        min_absolute_delta: 0.0,
        judgeable: false,
        read: |s| s.idle_cpu_percent,
        explanation: "Mostrado apenas como referência: oscila demais sozinho para provar qualquer coisa.",
    },
    MetricSpec {
        key: "idle_ram_gb",
        label: "RAM ocupada em segundo plano",
        unit: "GB",
        higher_is_better: false,
        noise_percent: 8.0,
        min_absolute_delta: 0.0,
        judgeable: true,
        read: |s| s.idle_ram_gb,
        explanation: "Mais RAM livre significa menos travadas por falta de memória.",
    },
    MetricSpec {
        key: "process_count",
        label: "Processos em execução",
        unit: "",
        higher_is_better: false,
        noise_percent: 5.0,
        min_absolute_delta: 0.0,
        judgeable: true,
        read: |s| s.process_count,
        explanation: "Menos processos disputando CPU e disco.",
    },
];

impl BenchmarkSnapshot {
    pub fn is_reliable(&self) -> bool {
        self.idle_cpu_percent <= BUSY_CPU_PERCENT
    }
}

/// Avisa já na medição inicial, para refazer agora e não descobrir 20 minutos depois.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResult {
    pub snapshot: BenchmarkSnapshot,
    pub reliable: bool,
    pub warning: Option<String>,
}

impl BaselineResult {
    pub fn from(snapshot: BenchmarkSnapshot) -> Self {
        let reliable = snapshot.is_reliable();

        let warning = if reliable {
            None
        } else {
            Some(format!(
                "O PC estava ocupado durante a medição ({:.0}% da CPU em uso). \
                 Feche os programas e meça de novo, senão a comparação não vai valer.",
                snapshot.idle_cpu_percent
            ))
        };

        BaselineResult {
            snapshot,
            reliable,
            warning,
        }
    }
}

pub struct Benchmark;

impl Benchmark {
    pub fn new() -> Self {
        Benchmark
    }

    /// O ocioso é medido ANTES das cargas, senão o próprio benchmark apareceria como consumo de fundo.
    pub fn run(&self) -> BenchmarkSnapshot {
        let mut system = System::new_all();

        let (idle_cpu_percent, idle_ram_gb, process_count) = measure_idle(&mut system);

        // Antes das cargas pesadas: mede o agendador com o PC como está, não ocupado por nós.
        let jitter = crate::modules::jitter::measure();

        let cpu_single_thread_mops = measure_single_thread();
        let (cpu_multi_thread_mops, cpu_frequency_under_load_mhz) = measure_multi_thread(&mut system);

        BenchmarkSnapshot {
            timestamp: crate::modules::changelog::now_timestamp(),
            idle_cpu_percent,
            idle_ram_gb,
            process_count,
            cpu_single_thread_mops,
            cpu_multi_thread_mops,
            cpu_frequency_under_load_mhz,
            scheduler_p99_delay_ms: jitter.p99_delay_ms,
            hitches_per_minute: jitter.hitches_per_minute,
        }
    }
}

impl Default for Benchmark {
    fn default() -> Self {
        Self::new()
    }
}

/// Mediana, para descartar picos isolados.
fn measure_idle(system: &mut System) -> (f64, f64, f64) {
    std::thread::sleep(IDLE_SETTLE);

    let mut cpu_samples = Vec::with_capacity(IDLE_SAMPLES as usize);

    for _ in 0..IDLE_SAMPLES {
        std::thread::sleep(IDLE_SAMPLE_INTERVAL);
        system.refresh_cpu_all();

        let cpus = system.cpus();
        if !cpus.is_empty() {
            let usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
            cpu_samples.push(usage as f64 / cpus.len() as f64);
        }
    }

    system.refresh_memory();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let idle_ram_gb = system.used_memory() as f64 / 1_073_741_824.0;
    let process_count = system.processes().len() as f64;

    (median(&mut cpu_samples), idle_ram_gb, process_count)
}

/// Sem o `black_box`, o compilador descarta o laço e mede o nada.
fn integer_workload(deadline: Instant) -> u64 {
    let mut operations: u64 = 0;
    let mut accumulator: u64 = 1;

    // Consultar o relógio a cada iteração mediria o relógio.
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

    operations
}

fn measure_single_thread() -> f64 {
    (0..WORKLOAD_ROUNDS)
        .map(|_| {
            let started = Instant::now();
            let operations = integer_workload(started + WORKLOAD_DURATION);
            throughput_mops(operations, started.elapsed())
        })
        .fold(0.0, f64::max)
}

fn measure_multi_thread(system: &mut System) -> (f64, f64) {
    let threads = num_cpus::get().max(1);
    let mut best_throughput = 0.0f64;
    let mut best_frequency = 0.0f64;

    for _ in 0..WORKLOAD_ROUNDS {
        let started = Instant::now();
        let deadline = started + WORKLOAD_DURATION;

        let handles: Vec<_> = (0..threads)
            .map(|_| std::thread::spawn(move || integer_workload(deadline)))
            .collect();

        // Lida com as threads rodando: é sob carga que o plano de energia se mostra.
        std::thread::sleep(WORKLOAD_DURATION / 2);
        system.refresh_cpu_all();
        let frequency = system
            .cpus()
            .iter()
            .map(|cpu| cpu.frequency() as f64)
            .fold(0.0, f64::max);

        let operations: u64 = handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .sum();

        best_throughput = best_throughput.max(throughput_mops(operations, started.elapsed()));
        best_frequency = best_frequency.max(frequency);
    }

    (best_throughput, best_frequency)
}

fn throughput_mops(operations: u64, elapsed: Duration) -> f64 {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        return 0.0;
    }
    operations as f64 / seconds / 1_000_000.0
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let middle = values.len() / 2;

    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

pub fn compare(before: &BenchmarkSnapshot, after: &BenchmarkSnapshot) -> BenchmarkComparison {
    // Medição com o PC ocupado: veredito aqui venderia o descanso do PC como otimização.
    let trustworthy = before.is_reliable() && after.is_reliable();

    let metrics: Vec<MetricDelta> = METRICS
        .iter()
        .map(|spec| {
            let before_value = (spec.read)(before);
            let after_value = (spec.read)(after);
            let change_percent = percent_change(before_value, after_value);
            let absolute_delta = (after_value - before_value).abs();

            let verdict = if !spec.judgeable || !trustworthy {
                Verdict::TooNoisyToJudge
            } else if absolute_delta < spec.min_absolute_delta {
                Verdict::NoMeasurableChange
            } else if before_value == 0.0 {
                // Sair de zero não tem porcentagem, e ignorar isso esconderia um PC que passou a engasgar.
                judge_by_direction(after_value - before_value, spec.higher_is_better)
            } else {
                judge(change_percent, spec.higher_is_better, spec.noise_percent)
            };

            MetricDelta {
                key: spec.key.to_string(),
                label: spec.label.to_string(),
                unit: spec.unit.to_string(),
                before: before_value,
                after: after_value,
                change_percent,
                verdict,
                explanation: spec.explanation.to_string(),
            }
        })
        .collect();

    let summary = if trustworthy {
        summarize(&metrics)
    } else {
        let busy = before.idle_cpu_percent.max(after.idle_cpu_percent);
        format!(
            "Medição não confiável: o PC estava ocupado ({:.0}% da CPU em uso por outros programas). \
             Feche tudo, espere um minuto e meça de novo — comparar um PC ocupado com um PC \
             descansado inventa um ganho que não existe.",
            busy
        )
    };

    BenchmarkComparison {
        before: before.clone(),
        after: after.clone(),
        metrics,
        summary,
    }
}

fn percent_change(before: f64, after: f64) -> f64 {
    if before == 0.0 {
        return 0.0;
    }
    (after - before) / before.abs() * 100.0
}

fn judge(change_percent: f64, higher_is_better: bool, noise_percent: f64) -> Verdict {
    if change_percent.abs() < noise_percent {
        return Verdict::NoMeasurableChange;
    }

    let improved = if higher_is_better {
        change_percent > 0.0
    } else {
        change_percent < 0.0
    };

    if improved {
        Verdict::Improved
    } else {
        Verdict::Worsened
    }
}

/// Só chamado depois de a diferença absoluta passar do piso da métrica.
fn judge_by_direction(delta: f64, higher_is_better: bool) -> Verdict {
    let improved = if higher_is_better { delta > 0.0 } else { delta < 0.0 };

    if improved {
        Verdict::Improved
    } else {
        Verdict::Worsened
    }
}

fn summarize(metrics: &[MetricDelta]) -> String {
    let improved = metrics.iter().filter(|m| m.verdict == Verdict::Improved).count();
    let worsened = metrics.iter().filter(|m| m.verdict == Verdict::Worsened).count();

    match (improved, worsened) {
        (0, 0) => "Nenhuma diferença mensurável. O PC já estava bem configurado nesses pontos, ou o gargalo é o hardware.".to_string(),
        (0, w) => format!(
            "{} indicador(es) pioraram e nenhum melhorou. Recomendamos desfazer as otimizações.",
            w
        ),
        (i, 0) => format!("{} indicador(es) melhoraram, nenhum piorou.", i),
        (i, w) => format!(
            "{} indicador(es) melhoraram e {} pioraram. Vale revisar quais otimizações manter.",
            i, w
        ),
    }
}

pub struct BaselineStore;

impl BaselineStore {
    fn path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("baseline.json")
    }

    pub fn save(snapshot: &BenchmarkSnapshot) -> Result<(), String> {
        let path = Self::path();

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
        }

        let raw = serde_json::to_string_pretty(snapshot)
            .map_err(|e| format!("Failed to serialize baseline: {}", e))?;

        fs::write(&path, raw).map_err(|e| format!("Failed to write baseline: {}", e))
    }

    pub fn load() -> Option<BenchmarkSnapshot> {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(single: f64, idle_cpu: f64) -> BenchmarkSnapshot {
        BenchmarkSnapshot {
            timestamp: 0,
            idle_cpu_percent: idle_cpu,
            idle_ram_gb: 8.0,
            process_count: 200.0,
            cpu_single_thread_mops: single,
            cpu_multi_thread_mops: 1000.0,
            cpu_frequency_under_load_mhz: 3000.0,
            scheduler_p99_delay_ms: 2.0,
            hitches_per_minute: 0.0,
        }
    }

    #[test]
    fn small_variation_is_reported_as_noise_not_gain() {
        assert_eq!(judge(2.0, true, 4.0), Verdict::NoMeasurableChange);
        assert_eq!(judge(-2.0, true, 4.0), Verdict::NoMeasurableChange);
    }

    #[test]
    fn clear_gain_is_reported_as_improvement() {
        assert_eq!(judge(12.0, true, 4.0), Verdict::Improved);
    }

    #[test]
    fn regression_is_reported_as_worse() {
        assert_eq!(judge(-12.0, true, 4.0), Verdict::Worsened);
    }

    #[test]
    fn lower_is_better_metrics_invert_the_verdict() {
        assert_eq!(judge(-40.0, false, 25.0), Verdict::Improved);
        assert_eq!(judge(40.0, false, 25.0), Verdict::Worsened);
    }

    #[test]
    fn identical_snapshots_summarize_as_no_change() {
        let comparison = compare(&snapshot(100.0, 5.0), &snapshot(100.0, 5.0));

        assert!(comparison
            .metrics
            .iter()
            .all(|m| m.verdict != Verdict::Improved && m.verdict != Verdict::Worsened));
        assert!(comparison.summary.contains("Nenhuma diferença mensurável"));
    }

    #[test]
    fn busy_machine_invalidates_the_whole_comparison() {
        // Caso real: baseline com o PC a 92% de CPU, a seguinte parado; os 29% vieram do descanso.
        let mut before = snapshot(500.0, 92.0);
        before.idle_cpu_percent = 92.0;
        let after = snapshot(645.0, 5.0);

        let comparison = compare(&before, &after);

        assert!(
            comparison
                .metrics
                .iter()
                .all(|m| m.verdict == Verdict::TooNoisyToJudge),
            "com o PC ocupado nenhum indicador pode virar veredito"
        );
        assert!(comparison.summary.contains("não confiável"));
        assert!(comparison.summary.contains("92%"));
    }

    #[test]
    fn idle_machine_allows_verdicts() {
        let comparison = compare(&snapshot(100.0, 5.0), &snapshot(130.0, 5.0));
        assert!(comparison
            .metrics
            .iter()
            .any(|m| m.verdict == Verdict::Improved));
    }

    #[test]
    fn large_percentage_over_a_tiny_number_is_not_a_result() {
        let mut before = snapshot(100.0, 5.0);
        before.scheduler_p99_delay_ms = 1.0;
        let mut after = snapshot(100.0, 5.0);
        after.scheduler_p99_delay_ms = 1.6;

        let stutter = compare(&before, &after)
            .metrics
            .into_iter()
            .find(|m| m.key == "scheduler_p99_delay_ms")
            .unwrap();

        assert_eq!(stutter.verdict, Verdict::NoMeasurableChange);
    }

    #[test]
    fn a_real_stutter_reduction_is_reported() {
        let mut before = snapshot(100.0, 5.0);
        before.scheduler_p99_delay_ms = 40.0;
        let mut after = snapshot(100.0, 5.0);
        after.scheduler_p99_delay_ms = 4.0;

        let stutter = compare(&before, &after)
            .metrics
            .into_iter()
            .find(|m| m.key == "scheduler_p99_delay_ms")
            .unwrap();

        assert_eq!(stutter.verdict, Verdict::Improved);
    }

    #[test]
    fn a_pc_that_started_stuttering_is_not_hidden_by_division_by_zero() {
        let mut before = snapshot(100.0, 5.0);
        before.hitches_per_minute = 0.0;
        let mut after = snapshot(100.0, 5.0);
        after.hitches_per_minute = 12.0;

        let comparison = compare(&before, &after);
        let hitches = comparison
            .metrics
            .iter()
            .find(|m| m.key == "hitches_per_minute")
            .unwrap();

        assert_eq!(hitches.verdict, Verdict::Worsened);
        assert!(comparison.summary.contains("desfazer"));
    }

    #[test]
    fn noisy_metrics_never_claim_a_gain() {
        let before = snapshot(100.0, 20.0);
        let after = snapshot(100.0, 5.0);

        let comparison = compare(&before, &after);
        let idle = comparison
            .metrics
            .iter()
            .find(|m| m.key == "idle_cpu_percent")
            .unwrap();

        assert_eq!(idle.verdict, Verdict::TooNoisyToJudge);
        assert!(comparison.summary.contains("Nenhuma diferença mensurável"));
    }

    #[test]
    fn regression_summary_recommends_undoing() {
        let comparison = compare(&snapshot(100.0, 5.0), &snapshot(70.0, 5.0));
        assert!(comparison.summary.contains("desfazer"));
    }

    #[test]
    fn improvement_summary_counts_metrics() {
        let comparison = compare(&snapshot(100.0, 5.0), &snapshot(130.0, 5.0));
        assert!(comparison.summary.starts_with("1 indicador"));
    }

    #[test]
    fn zero_baseline_does_not_invent_infinite_gain() {
        assert_eq!(percent_change(0.0, 50.0), 0.0);
    }

    #[test]
    fn median_ignores_isolated_spikes() {
        let mut values = vec![4.0, 5.0, 90.0, 5.0, 6.0];
        assert_eq!(median(&mut values), 5.0);
    }

    /// Teste nulo: duas medições sem mudar nada; qualquer veredito é falso positivo.
    /// `cargo test --release --lib -- --ignored --nocapture null_test`
    #[test]
    #[ignore]
    fn null_test_never_reports_a_gain() {
        let before = Benchmark::new().run();
        let after = Benchmark::new().run();
        let comparison = compare(&before, &after);

        for metric in &comparison.metrics {
            println!(
                "{:<38} {:>9.1} -> {:>9.1}  {:>7.1}%  {:?}",
                metric.label, metric.before, metric.after, metric.change_percent, metric.verdict
            );
        }

        let false_positives: Vec<&str> = comparison
            .metrics
            .iter()
            .filter(|m| m.verdict == Verdict::Improved || m.verdict == Verdict::Worsened)
            .map(|m| m.label.as_str())
            .collect();

        assert!(
            false_positives.is_empty(),
            "ruído reportado como resultado em: {:?}",
            false_positives
        );
    }

    /// Calibração que define o limiar de cada métrica.
    /// `cargo test --release --lib -- --ignored --nocapture noise_calibration`
    #[test]
    #[ignore]
    fn noise_calibration() {
        let runs: Vec<BenchmarkSnapshot> = (0..3).map(|_| Benchmark::new().run()).collect();

        for spec in METRICS {
            let values: Vec<f64> = runs.iter().map(|s| (spec.read)(s)).collect();
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let spread = if min == 0.0 { 0.0 } else { (max - min) / min * 100.0 };

            println!(
                "{:<38} {:>28}  variação: {:>6.1}%  limiar atual: {:.0}%",
                spec.label,
                format!("{:?}", values.iter().map(|v| (v * 10.0).round() / 10.0).collect::<Vec<_>>()),
                spread,
                spec.noise_percent
            );
        }
    }

    /// ~8 segundos com todos os núcleos: `cargo test --lib -- --ignored --nocapture benchmark`
    #[test]
    #[ignore]
    fn real_benchmark_produces_plausible_numbers() {
        let snapshot = Benchmark::new().run();
        println!("{:#?}", snapshot);

        assert!(snapshot.cpu_single_thread_mops > 0.0);
        assert!(
            snapshot.cpu_multi_thread_mops >= snapshot.cpu_single_thread_mops,
            "todos os núcleos não podem render menos que um só"
        );
        assert!(snapshot.process_count > 10.0);
        assert!(snapshot.idle_ram_gb > 0.0);
        assert!((0.0..=100.0).contains(&snapshot.idle_cpu_percent));
    }

    #[test]
    fn workload_actually_runs_and_is_not_optimized_away() {
        let started = Instant::now();
        let operations = integer_workload(started + Duration::from_millis(50));
        assert!(operations > 0, "a carga de trabalho não executou");
    }
}
