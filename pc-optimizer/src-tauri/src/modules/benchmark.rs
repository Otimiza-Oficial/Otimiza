// Benchmark — medição real antes e depois
//
// Este é o módulo que separa o produto dos "otimizadores" de mentira. Eles
// mostram uma barra de progresso e afirmam "PC 200% mais rápido". Aqui, o número
// vem de uma medição repetível, e o veredito pode perfeitamente ser
// "nenhuma melhora mensurável" — inclusive depois de otimizar.
//
// Três decisões sustentam a honestidade da medição:
//
// 1. Cada carga de trabalho roda várias vezes e usa o MELHOR resultado. O melhor
//    é o que menos sofreu interferência de outros processos; a média seria
//    puxada para baixo por ruído aleatório.
// 2. Cada métrica tem um limiar de ruído. Variação abaixo dele é reportada como
//    "dentro da margem de erro", nunca como ganho.
// 3. O baseline é gravado em disco, então otimizações que exigem reiniciar o PC
//    podem ser medidas corretamente depois do boot.

use serde::{Deserialize, Serialize};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use sysinfo::System;

/// Duração de cada repetição da carga de CPU.
const WORKLOAD_DURATION: Duration = Duration::from_millis(500);
/// Repetições por carga. Mais repetições aumentam a chance de pegar uma rodada
/// limpa, sem interferência de outros programas.
const WORKLOAD_ROUNDS: u32 = 5;
/// Tempo de acomodação antes de medir o consumo ocioso. Sem essa pausa, a medição
/// pega a máquina ainda ocupada com o que veio antes e reporta um valor inflado.
const IDLE_SETTLE: Duration = Duration::from_secs(2);
/// Amostras de uso ocioso de CPU, espaçadas por `IDLE_SAMPLE_INTERVAL`.
const IDLE_SAMPLES: u32 = 10;
const IDLE_SAMPLE_INTERVAL: Duration = Duration::from_millis(500);
/// Acima deste consumo ocioso, o PC está ocupado com outra coisa e a medição não
/// vale. Comparar um PC ocupado com um PC descansado produz um ganho fantasma de
/// dezenas por cento — o erro mais fácil de cometer e o mais fácil de vender.
const BUSY_CPU_PERCENT: f64 = 25.0;

/// Uma fotografia mensurável do desempenho da máquina.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSnapshot {
    pub timestamp: u64,
    /// Uso de CPU com o PC parado. Cai quando serviços de fundo são desativados.
    pub idle_cpu_percent: f64,
    /// RAM em uso com o PC parado, em GB.
    pub idle_ram_gb: f64,
    /// Processos em execução.
    pub process_count: f64,
    /// Operações por segundo em um único núcleo (em milhões).
    pub cpu_single_thread_mops: f64,
    /// Operações por segundo usando todos os núcleos (em milhões).
    pub cpu_multi_thread_mops: f64,
    /// Frequência da CPU sob carga, em MHz. É aqui que o plano de energia aparece.
    pub cpu_frequency_under_load_mhz: f64,
    /// Atraso do agendador no pior 1% dos casos, em ms. É a travada que se sente.
    /// `default` mantém compatível o baseline gravado antes desta métrica existir.
    #[serde(default)]
    pub scheduler_p99_delay_ms: f64,
    /// Engasgos por minuto: pausas maiores que a duração de um quadro.
    #[serde(default)]
    pub hitches_per_minute: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Improved,
    Worsened,
    /// A diferença ficou dentro do ruído de medição — não é ganho nem perda.
    NoMeasurableChange,
    /// A métrica é útil de mostrar, mas oscila demais sozinha para que qualquer
    /// variação possa ser atribuída à otimização. Medida na máquina de teste, a
    /// CPU ociosa variou 244% sem nada ter mudado no sistema.
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
    /// Frase única e honesta sobre o resultado geral, para mostrar ao cliente.
    pub summary: String,
}

/// Descrição de uma métrica: como lê-la, como interpretá-la e quanto de variação
/// é apenas ruído.
struct MetricSpec {
    key: &'static str,
    label: &'static str,
    unit: &'static str,
    higher_is_better: bool,
    /// Variação percentual abaixo da qual o resultado é considerado empate.
    noise_percent: f64,
    /// Diferença absoluta mínima para o resultado contar.
    ///
    /// Percentual sozinho engana quando o número é pequeno: sair de 1,0 ms para
    /// 1,4 ms de travada é 40% de variação e não significa nada. Aqui as duas
    /// condições precisam ser atendidas — percentual E diferença absoluta.
    min_absolute_delta: f64,
    /// Se `false`, a métrica é exibida mas nunca gera veredito de melhora ou piora.
    judgeable: bool,
    read: fn(&BenchmarkSnapshot) -> f64,
    explanation: &'static str,
}

/// Os limiares NÃO foram escolhidos no chute: vieram do teste `noise_calibration`,
/// que mede a mesma máquina três vezes sem mudar nada. O que variou ali é ruído.
/// Cada limiar é a variação observada com margem de segurança de ~50%.
///
/// Medido na máquina de calibração: 1 núcleo 3,8% · todos os núcleos 10,5% (com
/// picos de 19% no teste nulo) · frequência 0% · RAM 1,0% · processos 0,9% ·
/// CPU ociosa 244%.
///
/// Só quatro das seis métricas sustentam um veredito. As outras duas continuam
/// visíveis para o cliente, marcadas como referência — mostrar o número e admitir
/// que ele não prova nada é melhor que esconder ou que fingir que prova.
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
        // Carga em todos os núcleos aquece a CPU, e CPU quente reduz a própria
        // frequência. A segunda medição começa mais quente que a primeira, o que
        // cria um viés SISTEMÁTICO contra ela — não é ruído que mais repetições
        // resolvam. No teste nulo esta métrica marcou -19% sem nada ter mudado.
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
        // Calibração: o valor oscilou entre 1,0 e 1,4 ms sem nada mudar — 49%.
        // Daí o percentual alto e, principalmente, o piso de 3 ms: abaixo de um
        // quinto de quadro, nenhum jogador percebe diferença.
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
        // O teste nulo pegou saltos de 0 para 2 e 5 engasgos por minuto sem nada
        // ter mudado no sistema — qualquer programa de fundo que acorde produz
        // isso. Só a partir de 6 por minuto, um engasgo a cada dez segundos, a
        // diferença é grande demais para ser acaso e perceptível para o jogador.
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
        // Na calibração esta métrica variou 244% sem nada ter mudado. Qualquer
        // veredito aqui seria adivinhação vendida como resultado.
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
    /// Uma medição só é confiável se o PC estava razoavelmente parado.
    pub fn is_reliable(&self) -> bool {
        self.idle_cpu_percent <= BUSY_CPU_PERCENT
    }
}

/// Resultado da medição inicial, já avisando se ela vale.
/// O aviso vem aqui, e não só na comparação, para o usuário refazer a medição
/// agora — e não descobrir 20 minutos depois que perdeu o baseline.
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

    /// Executa a medição completa. Leva cerca de 8 segundos.
    ///
    /// A ordem importa: o uso ocioso é medido ANTES das cargas de trabalho, senão
    /// o próprio benchmark apareceria como consumo de segundo plano.
    pub fn run(&self) -> BenchmarkSnapshot {
        let mut system = System::new_all();

        let (idle_cpu_percent, idle_ram_gb, process_count) = measure_idle(&mut system);

        // A medição de engasgos vem antes das cargas pesadas: ela mede o atraso
        // do agendador com o PC como está, não com o PC ocupado por nós mesmos.
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

/// Mede o consumo do sistema parado: CPU, RAM e número de processos.
/// Usa a mediana das amostras para descartar picos isolados de outros programas.
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

/// Carga determinística de inteiros. O `black_box` impede o compilador de
/// descartar o laço — sem ele, a versão otimizada mediria o nada.
fn integer_workload(deadline: Instant) -> u64 {
    let mut operations: u64 = 0;
    let mut accumulator: u64 = 1;

    // O tempo é checado a cada bloco de 4096 operações: consultar o relógio a
    // cada iteração mediria o relógio, não a CPU.
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

/// Milhões de operações por segundo em um núcleo, melhor de N rodadas.
fn measure_single_thread() -> f64 {
    (0..WORKLOAD_ROUNDS)
        .map(|_| {
            let started = Instant::now();
            let operations = integer_workload(started + WORKLOAD_DURATION);
            throughput_mops(operations, started.elapsed())
        })
        .fold(0.0, f64::max)
}

/// Milhões de operações por segundo somando todos os núcleos, mais a frequência
/// atingida sob carga.
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

        // A frequência é lida com as threads ainda rodando: é sob carga que o
        // plano de energia mostra se está ou não segurando a CPU.
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

/// Compara duas medições e emite um veredito por métrica.
pub fn compare(before: &BenchmarkSnapshot, after: &BenchmarkSnapshot) -> BenchmarkComparison {
    // Se qualquer uma das duas medições pegou o PC ocupado, nenhum número aqui
    // sustenta conclusão. Emitir veredito nesse caso seria vender o descanso do
    // PC como se fosse resultado da otimização.
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
                // Grande em porcentagem, irrelevante na prática.
                Verdict::NoMeasurableChange
            } else if before_value == 0.0 {
                // Sair de zero não tem porcentagem — a divisão seria por zero.
                // Ignorar isso esconderia a pior regressão possível: um PC que
                // não engasgava e passou a engasgar.
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
        // Sem base de comparação, qualquer variação é indefinida — reportar 0
        // evita inventar um ganho infinito.
        return 0.0;
    }
    (after - before) / before.abs() * 100.0
}

/// Decide o veredito de uma métrica considerando o ruído de medição.
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

/// Veredito quando não há base para porcentagem: só o sentido da diferença.
/// Só é chamado depois de a diferença absoluta já ter passado do piso da métrica.
fn judge_by_direction(delta: f64, higher_is_better: bool) -> Verdict {
    let improved = if higher_is_better { delta > 0.0 } else { delta < 0.0 };

    if improved {
        Verdict::Improved
    } else {
        Verdict::Worsened
    }
}

/// Frase de resumo. Nunca afirma melhora quando nenhuma métrica melhorou.
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

/// Guarda o baseline em disco para sobreviver a um reinício do PC —
/// otimizações que só valem após reiniciar não poderiam ser medidas sem isso.
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
        // 2% de ganho em CPU está dentro do ruído: prometer melhora aqui seria
        // exatamente o que os concorrentes fazem.
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
        // Uso ocioso de CPU caindo 40% é melhora, não piora.
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
        // Cenário real observado: baseline medido com o PC a 92% de CPU, medição
        // seguinte com o PC parado. A diferença de 29% veio do descanso, não da
        // otimização.
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
        // Travada saindo de 1,0 ms para 1,6 ms é 60% de variação e nenhum
        // jogador percebe. O piso absoluto impede vender isso como piora.
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
        // 40 ms de travada caindo para 4 ms é a diferença entre engasgar e não.
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
        // Zero engasgos por minuto virando dez é a pior regressão possível, e
        // não tem porcentagem que a descreva. Precisa aparecer mesmo assim.
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
        // A CPU ociosa caiu 75% (dentro da faixa confiável, então a comparação
        // vale), mas essa métrica é instável demais para sustentar qualquer
        // afirmação. Reportar melhora aqui seria vender ruído.
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
        // O pico de 90% de um processo aleatório não deve virar o resultado.
        let mut values = vec![4.0, 5.0, 90.0, 5.0, 6.0];
        assert_eq!(median(&mut values), 5.0);
    }

    /// Teste nulo: mede duas vezes sem mudar NADA no sistema. Qualquer veredito de
    /// melhora ou piora aqui é um falso positivo — a falha exata que transforma um
    /// medidor honesto num vendedor de ilusão.
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

    /// Calibração: mede a mesma máquina várias vezes SEM mudar nada.
    /// Toda variação que aparecer aqui é ruído, e define o limiar de cada métrica.
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

    /// Executa a medição real. Marcado como `ignore` porque leva ~8 segundos e
    /// ocupa todos os núcleos; rode sob demanda com:
    /// `cargo test --lib -- --ignored --nocapture benchmark`
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
