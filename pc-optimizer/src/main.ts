import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ---------------------------------------------------------------- contratos

type Verdict = "Improved" | "Worsened" | "NoMeasurableChange" | "TooNoisyToJudge";
type State = "Applied" | "AlreadyOptimal" | "Available" | "Unavailable";
type Gain = "Measurable" | "Situational" | "Responsiveness";
type Category = "System" | "Gaming" | "Network" | "Startup" | "Privacy";

interface OptimizationInfo {
  id: string;
  name: string;
  description: string;
  honest_effect: string;
  category: Category;
  expected_gain: Gain;
  requires_admin: boolean;
  requires_restart: boolean;
  reversible: boolean;
  security_tradeoff: boolean;
  recommended: boolean;
  state: State;
  detail: string | null;
}

interface ProcessImpact {
  name: string;
  cpu_percent: number;
  ram_mb: number;
  instances: number;
  in_startup: boolean;
}

interface Preferences {
  restore_point_before_batch: boolean;
  metrics_interval_seconds: number;
  show_unavailable: boolean;
}

interface RestorePoint {
  sequence: number;
  description: string;
  created_at: string;
}

interface RestoreStatus {
  available: boolean;
  message: string;
  points: RestorePoint[];
}

interface StartupEntry {
  name: string;
  command: string;
  executable: string;
  hive: string;
  enabled: boolean;
}

interface BatchStep {
  index: number;
  total: number;
  name: string;
  stage: "started" | "finished";
  message: string;
  changes: string[];
  success: boolean;
}

interface Conflict {
  id: string;
  title: string;
  found: string[];
  explanation: string;
  advice: string;
  severity: Severity;
}

interface ConflictReport {
  conflicts: Conflict[];
  programs_scanned: number;
}

interface ScheduledTask {
  name: string;
  path: string;
  author: string;
  enabled: boolean;
  microsoft: boolean;
}

type BloatKind = "OemUtility" | "TrialSecurity" | "Sponsored" | "StoreApp";

interface BloatItem {
  name: string;
  publisher: string;
  kind: BloatKind;
  size_mb: number | null;
  reason: string;
  package: string | null;
  removable_here: boolean;
}

interface BloatReport {
  items: BloatItem[];
  total_mb: number;
  unmeasured: number;
  programs_scanned: number;
}

interface SpaceFinding {
  id: string;
  name: string;
  explanation: string;
  bytes: number;
  formatted: string;
  cleanable: boolean;
  requires_admin: boolean;
  warning: string | null;
}

interface DiskReport {
  drive: string;
  total_bytes: number;
  free_bytes: number;
  free_percent: number;
  pressure: string | null;
  recoverable_bytes: number;
  findings: SpaceFinding[];
}

interface MemoryFinding {
  id: string;
  title: string;
  measured: string;
  advice: string;
  severity: Severity;
  fix_location: FixLocation;
}

interface MemoryReport {
  total_ram_gb: number;
  available_ram_gb: number;
  committed_gb: number;
  pagefile_automatic: boolean;
  pagefile_size_gb: number;
  pagefile_peak_gb: number;
  pagefile_location: string;
  findings: MemoryFinding[];
}

type Severity = "Critical" | "Important" | "Ok";
type FixLocation = "Software" | "Bios" | "Hardware" | "None";

interface FirmwareFinding {
  id: string;
  title: string;
  measured: string;
  advice: string;
  severity: Severity;
  fix_location: FixLocation;
}

interface FirmwareReport {
  board: string;
  cpu: string;
  findings: FirmwareFinding[];
}

interface PerformanceMetrics {
  timestamp: number;
  cpu: {
    overall: number;
    per_core: number[];
    temperature: number;
    frequency: number;
  };
  ram: {
    total_gb: number;
    used_gb: number;
    available_gb: number;
    cached_gb: number;
    usage_percent: number;
  };
  disk: {
    read_speed_mbps: number;
    write_speed_mbps: number;
    /** Espaço ocupado, não atividade. */
    usage_percent: number;
  };
  network: {
    download_speed_mbps: number;
    upload_speed_mbps: number;
    total_received_gb: number;
    total_transmitted_gb: number;
  };
  uptime_hours: number;
}

interface OptimizationOutcome {
  id: string;
  name: string;
  success: boolean;
  applied: boolean;
  message: string;
  requires_restart: boolean;
  changes_count: number;
}

interface BenchmarkSnapshot {
  timestamp: number;
  idle_cpu_percent: number;
  idle_ram_gb: number;
  process_count: number;
  cpu_single_thread_mops: number;
  cpu_multi_thread_mops: number;
  cpu_frequency_under_load_mhz: number;
  scheduler_p99_delay_ms: number;
  hitches_per_minute: number;
}

interface BaselineResult {
  snapshot: BenchmarkSnapshot;
  reliable: boolean;
  warning: string | null;
}

interface MetricDelta {
  key: string;
  label: string;
  unit: string;
  before: number;
  after: number;
  change_percent: number;
  verdict: Verdict;
  explanation: string;
}

interface BenchmarkComparison {
  before: BenchmarkSnapshot;
  after: BenchmarkSnapshot;
  metrics: MetricDelta[];
  summary: string;
}

// ------------------------------------------------------------------ estado

const HISTORY_SAMPLES = 60; // 2 minutos a cada 2s
const cpuHistory: number[] = [];
let optimizations: OptimizationInfo[] = [];
let activeCategory: Category | "Todas" = "Todas";
let isElevated = false;
let preferences: Preferences = {
  restore_point_before_batch: true,
  metrics_interval_seconds: 2,
  show_unavailable: true,
};
/** Handle do laço de medição, para poder trocar o intervalo sem recarregar. */
let metricsTimer: number | null = null;
/** Categorias recolhidas pelo usuário, preservadas entre recarregamentos da lista. */
const collapsedGroups = new Set<string>();

const GAIN_LABELS: Record<Gain, string> = {
  Measurable: "ganho mensurável",
  Situational: "ganho situacional",
  Responsiveness: "resposta do sistema",
};

const CATEGORY_LABELS: Record<Category, string> = {
  System: "sistema",
  Gaming: "jogos",
  Network: "rede",
  Startup: "inicialização",
  Privacy: "privacidade",
};

const STATE_LABELS: Record<State, string> = {
  Applied: "aplicada",
  AlreadyOptimal: "já otimizado",
  Available: "disponível",
  Unavailable: "não se aplica",
};

const VERDICT_LABELS: Record<Verdict, string> = {
  Improved: "melhorou",
  Worsened: "piorou",
  NoMeasurableChange: "sem diferença",
  TooNoisyToJudge: "só referência",
};

// ------------------------------------------------------------------- início

window.addEventListener("DOMContentLoaded", async () => {
  wireControls();

  await ajustarMovimento();
  await listenToBatchProgress();
  // As preferências vêm antes de tudo: elas decidem o intervalo de medição e o
  // que a lista mostra.
  await loadPreferences();

  await Promise.all([
    loadIdentity(),
    checkAccess(),
    loadBaselineState(),
    loadOptimizations(),
    loadStartup(),
    loadRestoreStatus(),
    loadScheduledTasks(),
    loadThirdPartyServices(),
    loadProfiles(),
  ]);

  await startMonitoring();
});

/**
 * Decide se a interface pode se mexer.
 *
 * Duas fontes, e qualquer uma delas basta para desligar tudo: a preferencia do
 * sistema operacional, e o proprio hardware desta maquina — que o Otimiza ja
 * mede para outra finalidade.
 *
 * O motivo nao e estetico. Este programa e vendido com a promessa de deixar PC
 * fraco mais rapido; se a interface dele engasgar no PC que ele deveria estar
 * consertando, ele se desmente na frente do cliente antes de aplicar a primeira
 * otimizacao. Animacao e a primeira coisa a ser cortada, nao a ultima.
 */
async function ajustarMovimento() {
  const sistemaPedeCalma = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let maquinaFraca = false;
  try {
    const perfil = await invoke<{ total_ram_gb: number; logical_cores: number }>(
      "get_hardware_profile"
    );
    // Os limites sao o proprio publico-alvo do produto: 4 GB e 2 nucleos e a
    // maquina que o dono descreve como "PC fraco".
    maquinaFraca = perfil.total_ram_gb <= 4.5 || perfil.logical_cores <= 2;
  } catch {
    // Sem perfil, o padrao e animar. Errar para o lado de nao piorar a
    // aparencia de quem tem maquina boa.
  }

  document.body.classList.toggle("sem-animacao", sistemaPedeCalma || maquinaFraca);
}

function element<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

// ---------------------------------------------------------------------- abas

/**
 * Troca de aba. Só a seção escolhida fica visível; o cabeçalho com os sinais
 * vitais e o rodapé permanecem, então nunca se perde o contato com a máquina.
 */
function showTab(name: string) {
  document.querySelectorAll<HTMLElement>(".tab-panel").forEach((panel) => {
    panel.hidden = panel.id !== `tab-${name}`;
  });

  document.querySelectorAll<HTMLButtonElement>(".tab").forEach((tab) => {
    tab.setAttribute("aria-selected", String(tab.dataset.tab === name));
  });
}

/**
 * Selo numérico na aba. Mostrar o número aqui transforma a navegação em
 * informação: dá para saber que há algo esperando sem abrir a seção.
 */
function setBadge(id: string, count: number, tone?: "warn" | "bad") {
  const badge = element(id);
  badge.hidden = count <= 0;
  badge.textContent = String(count);

  if (tone) {
    badge.dataset.tone = tone;
  } else {
    delete badge.dataset.tone;
  }
}

function text(id: string, value: string) {
  element(id).textContent = value;
}

function escapeHtml(value: string): string {
  const node = document.createElement("div");
  node.textContent = value;
  return node.innerHTML;
}

// --------------------------------------------------------------- identidade

async function loadIdentity() {
  try {
    const platform = await invoke<{ os_type: string; version: string; arch: string }>(
      "get_platform_info"
    );
    text("ident-os", `${platform.version} · ${platform.arch}`);
  } catch (error) {
    text("ident-os", "indisponível");
    console.error(error);
  }

  try {
    const hardware = await invoke<{
      storage: string;
      total_ram_gb: number;
      logical_cores: number;
    }>("get_hardware_profile");

    text("ident-storage", hardware.storage);
    text("ident-ram", `${hardware.total_ram_gb.toFixed(1)} GB`);
  } catch (error) {
    text("ident-storage", "indisponível");
    console.error(error);
  }
}

async function checkAccess() {
  const badge = element("access-badge");

  try {
    isElevated = await invoke<boolean>("is_elevated");
    badge.dataset.level = isElevated ? "admin" : "limited";
    badge.querySelector(".access-text")!.textContent = isElevated
      ? "Administrador"
      : "Acesso limitado";
  } catch {
    badge.dataset.level = "limited";
    badge.querySelector(".access-text")!.textContent = "Acesso desconhecido";
  }
}

// ------------------------------------------------------------ monitoramento

async function startMonitoring() {
  try {
    await invoke("start_monitoring");
  } catch (error) {
    console.error("Não foi possível iniciar o monitoramento:", error);
  }

  await tick();
  restartMetricsLoop();
}

/**
 * (Re)inicia o laço de medição com o intervalo escolhido nas preferências.
 * Trocar o intervalo não recarrega a tela: só troca o relógio.
 */
function restartMetricsLoop() {
  if (metricsTimer !== null) {
    window.clearInterval(metricsTimer);
  }

  metricsTimer = window.setInterval(tick, preferences.metrics_interval_seconds * 1000);
}

async function tick() {
  try {
    const metrics = await invoke<PerformanceMetrics>("get_performance_metrics");
    renderMetrics(metrics);
  } catch (error) {
    console.error("Erro ao coletar métricas:", error);
  }

  try {
    const processes = await invoke<ProcessImpact[]>("top_processes");
    renderProcesses(processes);
  } catch (error) {
    console.error("Erro ao ler processos:", error);
  }
}

/**
 * Quem está pesando agora. Responde a pergunta que o cliente faz de verdade,
 * apontando o programa pelo nome em vez de mostrar um número agregado que não
 * ajuda ninguém a decidir nada.
 */
function renderProcesses(processes: ProcessImpact[]) {
  const target = element("process-list");

  if (processes.length === 0) {
    target.innerHTML = `<p class="empty">Nada consumindo de forma relevante.</p>`;
    return;
  }

  const heaviest = Math.max(...processes.map((p) => p.cpu_percent), 1);

  target.innerHTML = processes
    .map((process) => {
      const share = (process.cpu_percent / heaviest) * 100;
      const instances = process.instances > 1 ? ` ×${process.instances}` : "";
      const startup = process.in_startup
        ? `<span class="startup-flag" title="Sobe com o Windows">boot</span>`
        : "";

      return `
        <div class="process">
          <div class="process-top">
            <span class="process-name">${escapeHtml(process.name)}${instances}</span>
            ${startup}
            <span class="process-cpu">${process.cpu_percent.toFixed(1)}%</span>
          </div>
          <div class="process-bar"><i style="width:${share.toFixed(0)}%"></i></div>
          <span class="process-ram">${process.ram_mb.toFixed(0)} MB</span>
        </div>
      `;
    })
    .join("");
}

function renderMetrics(metrics: PerformanceMetrics) {
  const cpu = Math.min(100, Math.max(0, metrics.cpu.overall));

  // Anel principal. O perímetro (2πr, r=86) é 540, igual ao dasharray do CSS.
  const gauge = element<SVGCircleElement & HTMLElement>("gauge-cpu");

  // Na primeira leitura o anel subia de 540 direto para o valor, sem gesto
  // nenhum. Agora ele sobe devagar uma unica vez, como instrumento ligando —
  // e volta a velocidade normal em seguida, porque um medidor que reinicia a
  // cada 2 segundos pareceria quebrado, nao caro.
  const aro = gauge.closest(".gauge") as HTMLElement | null;
  if (aro && !aro.dataset.iniciado) {
    aro.dataset.iniciado = "sim";
    aro.dataset.entrada = "true";
    window.setTimeout(() => delete aro.dataset.entrada, 1000);
  }
  gauge.style.strokeDashoffset = String(540 - (540 * cpu) / 100);
  gauge.style.stroke = loadColor(cpu);

  // Faixa fixa do topo, viva em qualquer aba.
  text("vital-cpu", `${cpu.toFixed(0)}%`);
  setBar("vital-cpu-bar", cpu);
  text("vital-ram", `${metrics.ram.usage_percent.toFixed(0)}%`);
  setBar("vital-ram-bar", metrics.ram.usage_percent);
  text("vital-disk", `${metrics.disk.usage_percent.toFixed(0)}%`);
  setBar("vital-disk-bar", metrics.disk.usage_percent);

  text("cpu-value", cpu.toFixed(0));
  text("cpu-freq", `${metrics.cpu.frequency.toFixed(0)} MHz · ${metrics.cpu.per_core.length} núcleos`);
  text("core-count", `${metrics.cpu.per_core.length} lógicos`);
  text("tick-clock", new Date().toLocaleTimeString("pt-BR"));

  renderCores(metrics.cpu.per_core);
  pushHistory(cpu);

  const ram = metrics.ram;
  text("ram-value", `${ram.usage_percent.toFixed(0)}%`);
  text("ram-note", `${ram.used_gb.toFixed(1)} de ${ram.total_gb.toFixed(1)} GB em uso`);
  setBar("ram-bar", ram.usage_percent);

  text("disk-value", `${metrics.disk.usage_percent.toFixed(0)}%`);
  setBar("disk-bar", metrics.disk.usage_percent);

  text("net-value", `${metrics.network.total_received_gb.toFixed(1)} GB`);

  renderFlow(metrics);

  text("status-right", `atualizado às ${new Date().toLocaleTimeString("pt-BR")}`);
}

/**
 * Taxa em unidade legível. Abaixo de 1 MB/s a leitura em MB vira "0,0" e some;
 * em KB/s o mesmo valor aparece como 340 e se enxerga.
 */
function formatRate(mbPerSecond: number): string {
  if (mbPerSecond >= 1) return `${mbPerSecond.toFixed(1)} MB/s`;
  if (mbPerSecond >= 0.01) return `${(mbPerSecond * 1024).toFixed(0)} KB/s`;
  return "parado";
}

function renderFlow(metrics: PerformanceMetrics) {
  text("flow-read", formatRate(metrics.disk.read_speed_mbps));
  text("flow-write", formatRate(metrics.disk.write_speed_mbps));
  text(
    "flow-net",
    `${formatRate(metrics.network.download_speed_mbps)} · ${formatRate(
      metrics.network.upload_speed_mbps
    )}`
  );

  const horas = metrics.uptime_hours;
  const dias = Math.floor(horas / 24);

  text(
    "flow-uptime",
    dias >= 1 ? `${dias}d ${Math.floor(horas % 24)}h` : `${horas.toFixed(1)}h`
  );

  // Muitos dias sem reiniciar é uma causa real de lentidão que não aparece em
  // lugar nenhum: memória vazada por programas e drivers vai se acumulando, e o
  // PC melhora sozinho com um reinício. Vale dizer antes de otimizar qualquer
  // coisa — seria constrangedor cobrar por um ajuste que um reinício resolveria.
  const noteBar = element("flow-uptime-note");
  if (dias >= 7) {
    noteBar.textContent = "muitos dias sem reiniciar — reinicie antes de otimizar";
    noteBar.className = "readout-note warn";
  } else {
    noteBar.textContent = "desde o último boot";
    noteBar.className = "readout-note";
  }
}

function loadColor(percent: number): string {
  if (percent >= 85) return "var(--red)";
  if (percent >= 60) return "var(--amber)";
  return "var(--cyan)";
}

function setBar(id: string, percent: number) {
  const bar = element(id);
  bar.style.width = `${Math.min(100, Math.max(0, percent))}%`;
  bar.style.background = loadColor(percent);
}

/**
 * Matriz de núcleos: uma barra por núcleo lógico. É o instrumento principal da
 * tela porque mostra a assimetria que uma média esconde — um núcleo saturado
 * enquanto os outros dormem é exatamente o que trava um jogo.
 */
function renderCores(perCore: number[]) {
  const matrix = element("core-matrix");

  const primeiraVez = matrix.children.length !== perCore.length;

  if (primeiraVez) {
    // O índice vai para o CSS como variável: é ele que escalona a cascata de
    // entrada sem precisar de um temporizador por barra em JavaScript.
    matrix.innerHTML = perCore
      .map((_, i) => `<div class="core" style="--i:${i}"><i></i></div>`)
      .join("");
    matrix.dataset.entrada = "true";
  }

  perCore.forEach((load, index) => {
    const core = matrix.children[index] as HTMLElement;
    const fill = core.firstElementChild as HTMLElement;

    // `scaleY` em vez de `height`: não força relayout. Ver o comentário longo
    // em `.core i` no styles.css — esta linha era o maior desperdício da
    // interface, repetido a cada 2 segundos.
    fill.style.transform = `scaleY(${Math.min(100, Math.max(0, load)) / 100})`;
    core.dataset.load = load >= 85 ? "critical" : load >= 60 ? "high" : "normal";
  });

  // A cascata é só da entrada. Deixá-la ligada faria cada atualização chegar
  // escalonada, e o painel pareceria atrasado em vez de vivo.
  if (primeiraVez) {
    window.setTimeout(() => delete matrix.dataset.entrada, perCore.length * 14 + 500);
  }
}

function pushHistory(cpu: number) {
  cpuHistory.push(cpu);
  if (cpuHistory.length > HISTORY_SAMPLES) cpuHistory.shift();

  if (cpuHistory.length < 2) return;

  const points = cpuHistory.map((value, index) => {
    const x = (index / (HISTORY_SAMPLES - 1)) * 300;
    const y = 68 - (value / 100) * 64;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });

  element("cpu-history-line").setAttribute("points", points.join(" "));

  const lastX = ((cpuHistory.length - 1) / (HISTORY_SAMPLES - 1)) * 300;
  element("cpu-history-area").setAttribute(
    "points",
    `0,70 ${points.join(" ")} ${lastX.toFixed(1)},70`
  );
}

// -------------------------------------------------------------- diagnóstico

async function runDiagnostic() {
  const button = element<HTMLButtonElement>("run-diagnostic");
  const target = element("diagnostic-result");

  button.disabled = true;
  target.innerHTML = `<p class="empty">Analisando…</p>`;

  try {
    const report = await invoke<any>("run_diagnostic");
    renderDiagnostic(report);
  } catch (error) {
    target.innerHTML = `<p class="status error">${escapeHtml(String(error))}</p>`;
  } finally {
    button.disabled = false;
  }
}

function renderDiagnostic(report: any) {
  const info = report.system_info;
  text("ident-cpu", info.cpu_name);
  text("ident-ram", `${info.total_ram_gb.toFixed(1)} GB`);
  text("ident-gpu", info.gpu_name);

  const score: number = report.health_score;
  const color = score >= 80 ? "var(--green)" : score >= 60 ? "var(--amber)" : "var(--red)";

  const bottlenecks: string = report.bottlenecks.length
    ? report.bottlenecks
        .map(
          (item: any) => `
            <div class="bottleneck" data-severity="${item.severity}">
              <div class="bottleneck-title">${escapeHtml(item.description)}</div>
              <div class="bottleneck-detail">${escapeHtml(item.suggested_fix)}</div>
            </div>`
        )
        .join("")
    : `<p class="empty">Nenhum gargalo acima do limite. Isso não quer dizer que o
       PC é rápido — quer dizer que CPU, memória e disco não estão saturados agora.</p>`;

  element("diagnostic-result").innerHTML = `
    <div class="health">
      <span class="health-score" style="color:${color}">${score}</span>
      <span class="health-max">/ 100 saúde</span>
    </div>
    <div class="health-bar"><i style="width:${score}%;background:${color}"></i></div>
    ${bottlenecks}
  `;
}

// --------------------------------------------------------- firmware e hardware

const FIX_LABELS: Record<FixLocation, string> = {
  Software: "corrige por software",
  Bios: "só na BIOS",
  Hardware: "só trocando peça",
  None: "nada a fazer",
};

async function analyzeFirmware() {
  const button = element<HTMLButtonElement>("analyze-firmware");
  button.disabled = true;
  setStatus(
    "firmware-status",
    "Lendo o hardware e medindo carga sustentada — cerca de 12 segundos.",
    "progress"
  );

  try {
    const report = await invoke<FirmwareReport>("analyze_firmware");
    text("firmware-board", report.board);
    element("firmware-result").innerHTML = report.findings.map(renderFinding).join("");

    const problems = report.findings.filter((f) => f.severity !== "Ok").length;
    const critical = report.findings.some((f) => f.severity === "Critical");
    setBadge("badge-diagnostico", problems, critical ? "bad" : "warn");

    setStatus(
      "firmware-status",
      problems === 0
        ? "Nada a corrigir no firmware nem no hardware."
        : `${problems} ponto(s) custando desempenho.`,
      problems === 0 ? "ok" : "error"
    );
  } catch (error) {
    setStatus("firmware-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderFinding(finding: FirmwareFinding, indice = 0): string {
  const advice = finding.advice
    ? `<p class="finding-advice">${escapeHtml(finding.advice)}</p>`
    : "";

  return `
    <article class="finding" data-severity="${finding.severity}" style="--i:${indice}">
      <div class="finding-top">
        <h3>${escapeHtml(finding.title)}</h3>
        <span class="chip" data-fix="${finding.fix_location}">${FIX_LABELS[finding.fix_location]}</span>
      </div>
      <p class="finding-measured">${escapeHtml(finding.measured)}</p>
      ${advice}
    </article>
  `;
}

// ------------------------------------------- tempo de inicialização

type BootType = "Full" | "FastStartup" | "Resume";

interface BootMeasurement {
  when: string;
  total_ms: number;
  main_path_ms: number;
  post_boot_ms: number;
  instance: number;
  degraded: boolean;
}

interface BootCulprit {
  name: string;
  path: string;
  total_ms: number;
  degradation_ms: number;
}

interface BootReport {
  needs_admin: boolean;
  last: BootMeasurement | null;
  history: BootMeasurement[];
  culprits: BootCulprit[];
  recent_types: [string, BootType][];
  note: string;
}

const BOOT_TYPE_LABELS: Record<BootType, string> = {
  Full: "boot completo",
  FastStartup: "inicialização rápida",
  Resume: "retomada de hibernação",
};

/** Milissegundos em texto que uma pessoa lê sem converter na cabeça. */
function duracao(ms: number): string {
  const s = ms / 1000;
  if (s >= 60) return `${Math.floor(s / 60)} min ${Math.round(s % 60)} s`;
  return `${s.toFixed(1)} s`;
}

async function analyzeBoot() {
  const button = element<HTMLButtonElement>("analyze-boot");
  button.disabled = true;
  setStatus("boot-status", "Lendo o registro de inicialização do Windows…", "progress");

  try {
    const report = await invoke<BootReport>("analyze_boot");
    renderBootReport(report);
  } catch (error) {
    setStatus("boot-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderBootReport(report: BootReport) {
  const partes: string[] = [];

  if (report.last) {
    const b = report.last;
    text("boot-tag", duracao(b.total_ms));

    // A divisão importa mais que o total: pós-boot alto significa que a área de
    // trabalho apareceu mas a máquina ainda não dava para usar, que é o que o
    // dono sente e não sabe nomear.
    partes.push(`
      <div class="readouts readouts-row">
        <div class="readout">
          <span class="readout-label">Boot completo</span>
          <span class="readout-value">${duracao(b.total_ms)}</span>
          <span class="readout-note">até dar para usar</span>
        </div>
        <div class="readout">
          <span class="readout-label">Até a área de trabalho</span>
          <span class="readout-value">${duracao(b.main_path_ms)}</span>
          <span class="readout-note">o Windows subindo</span>
        </div>
        <div class="readout">
          <span class="readout-label">Depois disso</span>
          <span class="readout-value">${duracao(b.post_boot_ms)}</span>
          <span class="readout-note">programas de inicialização</span>
        </div>
      </div>
    `);
  } else {
    text("boot-tag", report.needs_admin ? "precisa de administrador" : "sem medição");
  }

  if (report.culprits.length > 0) {
    partes.push(`<h3 class="sub">O que mais atrasou</h3>`);
    partes.push(
      report.culprits
        .map(
          (c, i) => `
      <article class="folder" style="--i:${i}">
        <div class="folder-top">
          <span class="folder-name">${escapeHtml(c.name)}</span>
          <span class="folder-size">${duracao(c.total_ms)}</span>
        </div>
        <span class="folder-path">${escapeHtml(c.path)}</span>
      </article>`
        )
        .join("")
    );
    partes.push(
      `<p class="hint">Programa que atrasa o boot quase sempre está na aba
       Sistema, em Inicialização — desligar lá é reversível.</p>`
    );
  }

  if (report.recent_types.length > 0) {
    const resumo = report.recent_types
      .map(([quando, tipo]) => `${quando.slice(0, 10)} · ${BOOT_TYPE_LABELS[tipo]}`)
      .slice(0, 6)
      .join("<br />");
    partes.push(`<h3 class="sub">Últimas inicializações</h3><p class="hint">${resumo}</p>`);
  }

  element("boot-result").innerHTML = partes.join("");

  // A nota carrega a honestidade do painel: ela explica o que não deu para
  // medir e por quê. Nunca fica vazia.
  setStatus(
    "boot-status",
    report.note,
    report.last ? "ok" : report.needs_admin ? "warn" : "warn"
  );
}

// --------------------------------------- limitação do processador

type Culprit =
  | "Nenhum"
  | "Bateria"
  | "PlanoDeEnergia"
  | "Calor"
  | "LimiteEletrico"
  | "NaoIdentificado";

interface ThermalReport {
  culprit: Culprit;
  summary: string;
  advice: string;
  percent_of_max: number | null;
  power_cap_percent: number | null;
  on_battery: boolean;
  thermal_events: number;
  last_thermal_event: string | null;
}

async function analyzeThermal() {
  const button = element<HTMLButtonElement>("analyze-thermal");
  button.disabled = true;
  setStatus("thermal-status", "Medindo o processador e lendo o registro térmico…", "progress");

  try {
    const report = await invoke<ThermalReport>("analyze_thermal");
    renderThermalReport(report);
  } catch (error) {
    setStatus("thermal-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderThermalReport(report: ThermalReport) {
  // Só calor e limite elétrico são problema de peça. Plano de energia é
  // conserto de um clique, e bateria é comportamento correto do Windows —
  // pintar os quatro de vermelho ensinaria a ignorar o alarme.
  const gravidade: Record<Culprit, Severity> = {
    Nenhum: "Ok",
    Bateria: "Ok",
    PlanoDeEnergia: "Important",
    Calor: "Critical",
    LimiteEletrico: "Critical",
    NaoIdentificado: "Important",
  };

  text(
    "thermal-tag",
    report.percent_of_max === null
      ? "sem leitura"
      : `${report.percent_of_max.toFixed(0)}% da velocidade`
  );

  const conselho = report.advice
    ? `<p class="finding-advice">${escapeHtml(report.advice)}</p>`
    : "";

  element("thermal-result").innerHTML = `
    <article class="finding" data-severity="${gravidade[report.culprit]}" style="--i:0">
      <div class="finding-top">
        <h3>${escapeHtml(report.summary)}</h3>
      </div>
      ${conselho}
    </article>
  `;

  setStatus(
    "thermal-status",
    report.culprit === "Nenhum"
      ? "Nada está segurando o processador."
      : report.summary,
    report.culprit === "Nenhum" || report.culprit === "Bateria" ? "ok" : "error"
  );
}

// ------------------------------------------------------- saúde do hardware

interface HealthReport {
  findings: FirmwareFinding[];
  needs_admin: boolean;
}

async function analyzeHealth() {
  const button = element<HTMLButtonElement>("analyze-health");
  button.disabled = true;
  setStatus("health-status", "Consultando o disco e a bateria…", "progress");

  try {
    const report = await invoke<HealthReport>("analyze_health");
    element("health-result").innerHTML = report.findings.map(renderFinding).join("");

    const problemas = report.findings.filter((f) => f.severity !== "Ok").length;
    const critico = report.findings.some((f) => f.severity === "Critical");

    text("health-summary", problemas === 0 ? "nada a corrigir" : `${problemas} a ver`);

    // Faltar permissão não é o mesmo que estar tudo bem, e a diferença aqui é
    // séria: sem elevação não lemos desgaste nem contagem de erro, justamente
    // os dois números que dizem se o disco está indo embora.
    if (report.needs_admin) {
      setStatus(
        "health-status",
        "Estado geral lido, mas desgaste e contagem de erros do disco exigem " +
          "administrador. Reabra como administrador para a leitura completa.",
        "warn"
      );
    } else {
      setStatus(
        "health-status",
        problemas === 0
          ? "Disco e bateria sem sinal de desgaste preocupante."
          : `${problemas} ponto(s) de saúde física — troca de peça, não otimização.`,
        problemas === 0 ? "ok" : "error"
      );
    }

    if (problemas > 0) {
      setBadge("badge-diagnostico", problemas, critico ? "bad" : "warn");
    }
  } catch (error) {
    setStatus("health-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

// ------------------------------------------------------- programas de fábrica

const BLOAT_LABELS: Record<BloatKind, string> = {
  OemUtility: "utilitário do fabricante",
  TrialSecurity: "segurança em teste",
  Sponsored: "patrocinado",
  StoreApp: "app da Loja",
};

async function analyzeBloatware() {
  const button = element<HTMLButtonElement>("analyze-bloat");
  button.disabled = true;
  setStatus("bloat-status", "Lendo programas instalados e apps da Loja…", "progress");

  try {
    const report = await invoke<BloatReport>("analyze_bloatware");
    renderBloatReport(report);
  } catch (error) {
    setStatus("bloat-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderBloatReport(report: BloatReport) {
  text("bloat-summary", `${report.programs_scanned} programas examinados`);

  if (report.items.length === 0) {
    setStatus("bloat-status", "Nenhum programa de fábrica encontrado.", "ok");
    element("bloat-result").innerHTML = "";
    return;
  }

  // O total só menciona espaço quando algo foi realmente medido, e diz que é
  // parcial quando parte dos itens não tem tamanho legível.
  const medido = report.items.length - report.unmeasured;
  const espaco =
    medido > 0
      ? ` Pelo menos ${report.total_mb.toFixed(0)} MB${
          report.unmeasured > 0 ? ` (${report.unmeasured} sem tamanho informado)` : ""
        }.`
      : "";

  setStatus("bloat-status", `${report.items.length} encontrados.${espaco}`, "error");
  element("bloat-result").innerHTML = report.items.map(renderBloatItem).join("");
}

function renderBloatItem(item: BloatItem): string {
  const tamanho =
    item.size_mb === null
      ? `<span class="state-label">tamanho não informado</span>`
      : `<span class="finding-size">${item.size_mb.toFixed(0)} MB</span>`;

  const acao = item.removable_here
    ? `<button class="btn btn-ghost" data-bloat="${escapeHtml(item.package ?? "")}">Remover</button>`
    : "";

  return `
    <article class="finding" data-severity="Important">
      <div class="finding-top">
        <h3>${escapeHtml(item.name)}</h3>
        <span class="chip">${BLOAT_LABELS[item.kind]}</span>
        ${tamanho}
        ${acao}
      </div>
      <p class="finding-advice">${escapeHtml(item.reason)}</p>
    </article>
  `;
}

// ------------------------------------------------------ conflitos e tarefas

async function analyzeConflicts() {
  const button = element<HTMLButtonElement>("analyze-conflicts");
  button.disabled = true;
  setStatus("conflicts-status", "Lendo programas instalados e processos…", "progress");

  try {
    const report = await invoke<ConflictReport>("analyze_conflicts");
    text("conflicts-summary", `${report.programs_scanned} programas examinados`);

    const problemas = report.conflicts.filter((c) => c.severity !== "Ok").length;
    setStatus(
      "conflicts-status",
      problemas === 0
        ? "Nenhum programa disputando função com outro."
        : `${problemas} conflito(s) custando desempenho.`,
      problemas === 0 ? "ok" : "error"
    );

    element("conflicts-result").innerHTML = report.conflicts
      .map(
        (c) => `
        <article class="finding" data-severity="${c.severity}">
          <div class="finding-top"><h3>${escapeHtml(c.title)}</h3></div>
          ${
            c.found.length
              ? `<ul class="conflict-list">${c.found
                  .map((f) => `<li>${escapeHtml(f)}</li>`)
                  .join("")}</ul>`
              : ""
          }
          <p class="finding-advice">${escapeHtml(c.explanation)}</p>
          ${c.advice ? `<p class="finding-advice">${escapeHtml(c.advice)}</p>` : ""}
        </article>`
      )
      .join("");
  } catch (error) {
    setStatus("conflicts-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

async function loadScheduledTasks() {
  try {
    const tasks = await invoke<ScheduledTask[]>("list_scheduled_tasks");
    const ligadas = tasks.filter((t) => t.enabled).length;
    text("tasks-count", `${ligadas} de ${tasks.length} ligadas`);

    element("tasks-list").innerHTML = tasks.length
      ? tasks.map(renderTask).join("")
      : `<p class="empty">Nenhuma tarefa de terceiros neste PC.</p>`;
  } catch (error) {
    element("tasks-list").innerHTML = `<p class="status error">${escapeHtml(String(error))}</p>`;
  }
}

function renderTask(task: ScheduledTask): string {
  const autor = task.author.trim() || "autor não informado";

  return `
    <div class="startup" data-enabled="${task.enabled}">
      <div class="startup-info">
        <span class="startup-name">${escapeHtml(task.name)}</span>
        <span class="startup-exe">${escapeHtml(autor)}</span>
      </div>
      <button class="btn btn-ghost"
              data-task="${escapeHtml(task.name)}"
              data-taskpath="${escapeHtml(task.path)}"
              data-enable="${!task.enabled}">
        ${task.enabled ? "Desligar" : "Ligar"}
      </button>
    </div>
  `;
}

// --------------------------------------------------- mapa de pastas

interface FolderEntry {
  name: string;
  path: string;
  bytes: number;
  formatted: string;
  percent: number;
  explanation: string;
  partial: boolean;
}

interface FolderMap {
  root: string;
  total_bytes: number;
  total_formatted: string;
  folders: FolderEntry[];
  unreadable: number;
  timed_out: boolean;
}

async function mapFolders() {
  const button = element<HTMLButtonElement>("map-folders");
  button.disabled = true;
  setStatus(
    "map-status",
    "Somando pastas… pode levar até um minuto num disco cheio.",
    "progress"
  );

  try {
    const map = await invoke<FolderMap>("map_folders");
    renderFolderMap(map);
  } catch (error) {
    setStatus("map-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderFolderMap(map: FolderMap) {
  text("map-summary", map.root);

  if (map.folders.length === 0) {
    setStatus("map-status", "Nenhuma subpasta encontrada no seu perfil.", "ok");
    element("map-result").innerHTML = "";
    return;
  }

  // Quando a varredura não terminou, isso é a primeira coisa que a pessoa
  // precisa ler — antes de qualquer número. Um piso apresentado como total
  // manda o técnico limpar a pasta errada.
  if (map.timed_out) {
    setStatus(
      "map-status",
      `Pelo menos ${map.total_formatted} no seu perfil. A varredura não terminou ` +
        `dentro do tempo, então as pastas marcadas como "não terminou" têm mais ` +
        `do que o mostrado — e são justamente as maiores.`,
      "warn"
    );
  } else {
    setStatus(
      "map-status",
      `${map.total_formatted} no seu perfil.` +
        (map.unreadable > 0
          ? ` ${map.unreadable} pasta(s) sem permissão de leitura ficaram de fora.`
          : ""),
      "ok"
    );
  }

  element("map-result").innerHTML = map.folders.map(renderFolder).join("");
}

function renderFolder(folder: FolderEntry, indice = 0): string {
  const explicacao = folder.explanation
    ? `<p class="finding-advice">${escapeHtml(folder.explanation)}</p>`
    : "";

  // "pelo menos" no lugar do número seco: é a diferença entre informar e
  // enganar quando a soma foi cortada.
  const tamanho = folder.partial
    ? `pelo menos ${folder.formatted}`
    : folder.formatted;

  return `
    <article class="folder" data-partial="${folder.partial}" style="--i:${indice}">
      <div class="folder-top">
        <span class="folder-name">${escapeHtml(folder.name)}</span>
        <span class="folder-size">${escapeHtml(tamanho)}</span>
      </div>
      <div class="bar"><i style="width:${Math.min(100, folder.percent)}%"></i></div>
      <span class="folder-path">${escapeHtml(folder.path)}${
        folder.partial ? " · não terminou" : ""
      }</span>
      ${explicacao}
    </article>
  `;
}

// ------------------------------------- serviços deixados por programas

type StartMode = "Automatic" | "Manual" | "Disabled" | "Kernel";

interface ServiceEntry {
  name: string;
  display_name: string;
  path: string;
  start_mode: StartMode;
  running: boolean;
  ram_mb: number | null;
  protected: string | null;
}

const START_MODE_LABELS: Record<StartMode, string> = {
  Automatic: "sobe no boot",
  Manual: "sob demanda",
  Disabled: "desativado",
  Kernel: "driver",
};

async function loadThirdPartyServices() {
  try {
    const services = await invoke<ServiceEntry[]>("list_third_party_services");
    const noBoot = services.filter(
      (s) => s.start_mode === "Automatic" && !s.protected
    ).length;

    text("services-count", `${noBoot} de ${services.length} sobem no boot`);

    element("services-list").innerHTML = services.length
      ? services.map(renderService).join("")
      : `<p class="empty">Nenhum serviço de terceiros neste PC.</p>`;

    if (noBoot > 0) setBadge("badge-sistema", noBoot, "warn");
  } catch (error) {
    element("services-list").innerHTML = `<p class="status error">${escapeHtml(
      String(error)
    )}</p>`;
  }
}

function renderService(service: ServiceEntry): string {
  // Memória só aparece quando foi medida de verdade. Serviço hospedado num
  // processo compartilhado não tem número atribuível, e "0 MB" seria mentira.
  const memoria =
    service.ram_mb === null
      ? ""
      : `<span class="finding-size">${service.ram_mb.toFixed(0)} MB</span>`;

  // Protegido é informação, não botão: mostrar o motivo ensina mais do que
  // esconder a linha, e responde de véspera o "por que não aparece meu antivírus".
  if (service.protected) {
    return `
      <div class="startup" data-enabled="false">
        <div class="startup-info">
          <span class="startup-name">${escapeHtml(service.display_name)}</span>
          <span class="startup-exe">${escapeHtml(service.protected)}</span>
        </div>
        <span class="state-label">protegido</span>
      </div>
    `;
  }

  const noBoot = service.start_mode === "Automatic";
  const situacao = `${START_MODE_LABELS[service.start_mode]}${
    service.running ? " · rodando agora" : ""
  }`;

  return `
    <div class="startup" data-enabled="${noBoot}">
      <div class="startup-info">
        <span class="startup-name">${escapeHtml(service.display_name)}</span>
        <span class="startup-exe">${escapeHtml(situacao)}</span>
      </div>
      ${memoria}
      <button class="btn btn-ghost"
              data-service="${escapeHtml(service.name)}"
              data-auto="${!noBoot}">
        ${noBoot ? "Deixar sob demanda" : "Voltar ao boot"}
      </button>
    </div>
  `;
}

// ------------------------------------------------------------ espaço em disco

async function scanDiskSpace() {
  const button = element<HTMLButtonElement>("scan-disk");
  button.disabled = true;
  setStatus("disk-status", "Somando pastas… pode levar alguns segundos.", "progress");

  try {
    const report = await invoke<DiskReport>("scan_disk_space");
    renderDiskReport(report);
  } catch (error) {
    setStatus("disk-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderDiskReport(report: DiskReport) {
  const gb = (bytes: number) => (bytes / 1_073_741_824).toFixed(1);
  const usedPercent = 100 - report.free_percent;

  text("disk-summary", `${gb(report.free_bytes)} GB livres de ${gb(report.total_bytes)} GB`);

  element("disk-meter").hidden = false;
  setBar("disk-used-bar", usedPercent);
  text(
    "disk-meter-note",
    `${report.drive} · ${usedPercent.toFixed(0)}% ocupado · ${report.free_percent.toFixed(0)}% livre`
  );

  // O aviso de disco cheio vem antes de qualquer oferta de limpeza: é o que
  // explica a lentidão que o cliente está sentindo.
  if (report.pressure) {
    setStatus("disk-status", report.pressure, "error");
  } else {
    const recuperavel = (report.recoverable_bytes / 1_048_576).toFixed(0);
    setStatus("disk-status", `${recuperavel} MB podem ser liberados por aqui.`, "ok");
  }

  setBadge("badge-espaco", report.pressure ? 1 : 0, "bad");

  element("disk-result").innerHTML = report.findings.map(renderSpaceFinding).join("");
}

function renderSpaceFinding(item: SpaceFinding): string {
  const aviso = item.warning
    ? `<p class="finding-advice">${escapeHtml(item.warning)}</p>`
    : "";

  // Categoria vazia não ganha botão: oferecer limpeza de zero byte é encher a
  // tela de ação inútil.
  const acao = item.cleanable
    ? `<button class="btn btn-ghost" data-space="${item.id}">Limpar</button>`
    : `<span class="state-label">${item.bytes === 0 ? "vazio" : "pela Limpeza de Disco"}</span>`;

  return `
    <article class="finding" data-severity="${item.bytes === 0 ? "Ok" : "Important"}">
      <div class="finding-top">
        <h3>${escapeHtml(item.name)}</h3>
        <span class="finding-size">${escapeHtml(item.formatted)}</span>
        ${acao}
      </div>
      <p class="finding-advice">${escapeHtml(item.explanation)}</p>
      ${aviso}
    </article>
  `;
}

async function cleanDiskCategory(id: string, button: HTMLButtonElement) {
  button.disabled = true;
  setStatus("disk-status", "Limpando…", "progress");

  try {
    const outcome = await invoke<{ message: string }>("clean_disk_category", { id });
    setStatus("disk-status", outcome.message, "ok");
  } catch (error) {
    setStatus("disk-status", String(error), "error");
  } finally {
    await scanDiskSpace();
  }
}

// -------------------------------------------------------- memória e paginação

async function analyzeMemory() {
  const button = element<HTMLButtonElement>("analyze-memory");
  button.disabled = true;
  setStatus("memory-status", "Lendo memória e paginação…", "progress");

  try {
    const report = await invoke<MemoryReport>("analyze_memory");
    renderMemoryReport(report);
  } catch (error) {
    setStatus("memory-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderMemoryReport(report: MemoryReport) {
  text(
    "memory-summary",
    `${report.total_ram_gb.toFixed(1)} GB · paginação ${report.pagefile_size_gb.toFixed(1)} GB`
  );

  // O botão de correção só aparece quando há o que corrigir.
  const precisaCorrigir = report.findings.some(
    (f) => f.id === "pagefile_off" || f.id === "pagefile_manual" || f.id === "pagefile_small"
  );
  element("fix-pagefile").hidden = !precisaCorrigir;

  const problemas = report.findings.filter((f) => f.severity !== "Ok").length;
  const critico = report.findings.some((f) => f.severity === "Critical");

  setStatus(
    "memory-status",
    problemas === 0
      ? "Memória e paginação sem problemas."
      : `${problemas} ponto(s) afetando o desempenho.`,
    problemas === 0 ? "ok" : "error"
  );

  element("memory-result").innerHTML = report.findings
    .map(
      (f) => `
      <article class="finding" data-severity="${f.severity}">
        <div class="finding-top">
          <h3>${escapeHtml(f.title)}</h3>
          <span class="chip" data-fix="${f.fix_location}">${FIX_LABELS[f.fix_location]}</span>
        </div>
        <p class="finding-measured">${escapeHtml(f.measured)}</p>
        ${f.advice ? `<p class="finding-advice">${escapeHtml(f.advice)}</p>` : ""}
      </article>`
    )
    .join("");

  // O selo do Diagnóstico soma firmware e memória: é a aba inteira que precisa
  // de atenção, não um painel só.
  const firmwareBadge = Number(element("badge-diagnostico").textContent) || 0;
  setBadge("badge-diagnostico", firmwareBadge + problemas, critico ? "bad" : "warn");
}

// -------------------------------------------------------------- otimizações

async function loadOptimizations() {
  try {
    optimizations = await invoke<OptimizationInfo[]>("list_optimizations");
    renderFilters();
    renderOptimizations();
  } catch (error) {
    element("optimization-list").innerHTML =
      `<p class="status error">${escapeHtml(String(error))}</p>`;
  }
}

// ---------------------------------------------------------------- perfis

interface ProfileInfo {
  id: string;
  name: string;
  description: string;
  tradeoff: string;
  optimization_ids: string[];
}

let profiles: ProfileInfo[] = [];
let activeProfile: string | null = null;
/** Texto digitado na busca do catálogo. */
let searchTerm = "";

async function loadProfiles() {
  try {
    profiles = await invoke<ProfileInfo[]>("list_profiles");
    renderProfileChips();
  } catch {
    // Sem perfis a lista continua inteira e utilizável: eles são um atalho,
    // não um pré-requisito.
    element("profile-chips").innerHTML = "";
  }
}

function renderProfileChips() {
  element("profile-chips").innerHTML = profiles
    .map(
      (p) =>
        `<button class="profile-chip" data-profile="${p.id}" aria-pressed="${
          activeProfile === p.id
        }">${escapeHtml(p.name)}</button>`
    )
    .join("");
}

/**
 * Aplica um perfil marcando os itens dele na lista — sem executar nada.
 *
 * A diferença entre isto e o "otimizar tudo" do mercado é essa: o perfil é uma
 * sugestão visível e editável. A pessoa vê o que foi marcado, lê o que o perfil
 * abre mão, e desmarca o que não quiser antes de apertar qualquer botão.
 */
function selectProfile(id: string) {
  // Clicar de novo no mesmo perfil desmarca: o atalho tem volta.
  if (activeProfile === id) {
    activeProfile = null;
    element("profile-detail").hidden = true;
    renderProfileChips();
    renderOptimizations();
    return;
  }

  activeProfile = id;
  renderProfileChips();
  renderOptimizations();

  const perfil = profiles.find((p) => p.id === id);
  if (!perfil) return;

  const alvo = new Set(perfil.optimization_ids);
  const doPerfil = optimizations.filter((o) => alvo.has(o.id));
  const aAplicar = doPerfil.filter((o) => o.state === "Available");
  const jaTem = doPerfil.filter(
    (o) => o.state === "Applied" || o.state === "AlreadyOptimal"
  ).length;

  // O que o perfil deixa de fazer aparece junto com o que ele faz. Um perfil
  // que só se elogia é propaganda, não recomendação.
  const detalhe = element("profile-detail");
  detalhe.hidden = false;
  detalhe.innerHTML = `
    ${escapeHtml(perfil.description)}
    <br /><br />
    <strong>O que este perfil abre mão:</strong> ${escapeHtml(perfil.tradeoff)}
    <br /><br />
    <strong>${aAplicar.length} a aplicar${
      jaTem > 0 ? `, ${jaTem} que o seu PC já tem` : ""
    }.</strong>
    A lista ao lado está mostrando só os itens deste perfil — confira antes de aplicar.
    ${
      aAplicar.length > 0
        ? `<br /><br /><button id="apply-profile" class="btn btn-primary">Aplicar os ${aAplicar.length} deste perfil</button>`
        : ""
    }
  `;
}

function renderFilters() {
  const categories = Array.from(new Set(optimizations.map((item) => item.category)));
  const options: (Category | "Todas")[] = ["Todas", ...categories];

  element("filters").innerHTML = options
    .map((option) => {
      const label = option === "Todas" ? "todas" : CATEGORY_LABELS[option];
      const pressed = option === activeCategory;
      return `<button class="filter" data-category="${option}" aria-pressed="${pressed}">${label}</button>`;
    })
    .join("");
}

/**
 * Casa o texto buscado com uma otimização.
 *
 * Procura no nome, na descrição, no efeito honesto e no id. O efeito honesto
 * entra de propósito: é onde estão as palavras que a pessoa lembra ("boot",
 * "travada", "memória") quando não lembra o nome do ajuste.
 */
function matchesSearch(item: OptimizationInfo, termo: string): boolean {
  if (!termo) return true;

  const alvo = `${item.name} ${item.description} ${item.honest_effect} ${item.id}`;

  // Sem acento dos dois lados: quem digita "memoria" precisa achar "memória".
  const normalizar = (t: string) =>
    t.toLowerCase().normalize("NFD").replace(/\p{Diacritic}/gu, "");

  return normalizar(alvo).includes(normalizar(termo));
}

function renderOptimizations() {
  const termo = searchTerm.trim();

  const visible = optimizations
    .filter((item) => preferences.show_unavailable || item.state !== "Unavailable")
    .filter((item) => activeCategory === "Todas" || item.category === activeCategory)
    .filter((item) => matchesSearch(item, termo))
    // Perfil escolhido reduz a lista ao que ele recomenda. É o que transforma
    // 35 itens numa decisão possível — sem esconder nada: basta desmarcar o
    // perfil para a lista inteira voltar.
    .filter((item) => {
      if (!activeProfile) return true;
      const perfil = profiles.find((p) => p.id === activeProfile);
      return perfil ? perfil.optimization_ids.includes(item.id) : true;
    });

  const available = optimizations.filter((item) => item.state === "Available").length;
  const applied = optimizations.filter((item) => item.state === "Applied").length;
  text("optimization-count", `${available} a aplicar · ${applied} ativas`);
  setBadge("badge-otimizacoes", available);

  // Busca sem resultado precisa dizer isso. Uma lista vazia e silenciosa faz a
  // pessoa achar que o programa travou.
  if (visible.length === 0) {
    element("optimization-list").innerHTML = termo
      ? `<p class="empty">Nada encontrado para "${escapeHtml(termo)}".</p>`
      : activeProfile
        ? `<p class="empty">Nenhum item deste perfil aparece nesta categoria.</p>`
        : `<p class="empty">Nenhuma otimização nesta categoria.</p>`;
    return;
  }

  // Agrupar por categoria mantém a lista curta: cada grupo pode ser recolhido, e
  // percorrer 14 itens deixa de exigir rolar a tela inteira.
  const groups = new Map<Category, OptimizationInfo[]>();
  for (const item of visible) {
    const bucket = groups.get(item.category) ?? [];
    bucket.push(item);
    groups.set(item.category, bucket);
  }

  element("optimization-list").innerHTML = [...groups.entries()]
    .map(([category, items]) => renderGroup(category, items))
    .join("");
}

function renderGroup(category: Category, items: OptimizationInfo[]): string {
  const pending = items.filter((item) => item.state === "Available").length;
  const open = collapsedGroups.has(category) ? "" : " open";
  const summary = pending > 0 ? `${pending} a aplicar` : "tudo certo";

  // O que pesa nesta máquina sobe: quem abre o grupo vê primeiro o que vale a
  // pena para o hardware dele, não a ordem em que escrevemos o catálogo.
  const ordenados = [...items].sort(
    (a, b) => Number(b.recommended) - Number(a.recommended)
  );

  return `
    <details class="opt-group"${open} data-category="${category}">
      <summary class="group-head">
        <span>${CATEGORY_LABELS[category]}</span>
        <span class="group-count">${summary} · ${items.length}</span>
      </summary>
      ${ordenados.map(renderOptimization).join("")}
    </details>
  `;
}

/**
 * Cada otimização é uma linha compacta. Os detalhes ficam dobrados: quem quiser
 * só aplicar vê a lista inteira de uma vez; quem quiser entender abre o item.
 */
function renderOptimization(item: OptimizationInfo): string {
  const chips = [
    `<span class="chip">${GAIN_LABELS[item.expected_gain]}</span>`,
  ];

  if (item.recommended)
    chips.push(`<span class="chip" data-recommended="true">pesa nesta máquina</span>`);
  if (item.requires_restart) chips.push(`<span class="chip">exige reiniciar</span>`);
  if (item.requires_admin) chips.push(`<span class="chip">administrador</span>`);
  if (!item.reversible) chips.push(`<span class="chip" data-warn="true">sem volta</span>`);
  if (item.security_tradeoff)
    chips.push(`<span class="chip" data-warn="true">reduz segurança</span>`);

  const detail = item.detail ? `<p class="detail">${escapeHtml(item.detail)}</p>` : "";

  return `
    <details class="optimization" data-state="${item.state}">
      <summary class="opt-row">
        <span class="gain-dot" data-gain="${item.expected_gain}" ${item.recommended ? 'data-recommended="true"' : ""}></span>
        <span class="opt-name">${escapeHtml(item.name)}</span>
        ${actionControl(item)}
      </summary>
      <div class="opt-body">
        <div class="optimization-meta">${chips.join("")}</div>
        <p class="effect">${escapeHtml(item.honest_effect)}</p>
        ${detail}
      </div>
    </details>
  `;
}

/**
 * O controle muda conforme a situação real da máquina. "Já otimizado" não vira
 * botão: oferecer aplicar o que o PC já tem é o truque de quem cobra por
 * serviço que não executou.
 */
function actionControl(item: OptimizationInfo): string {
  switch (item.state) {
    case "Applied":
      return item.reversible
        ? `<button class="btn btn-ghost" data-id="${item.id}" data-action="revert" data-admin="${item.requires_admin}">Desfazer</button>`
        : `<span class="state-label" data-state="Applied">${STATE_LABELS.Applied}</span>`;
    case "Available":
      return `<button class="btn btn-ghost" data-id="${item.id}" data-action="apply" data-admin="${item.requires_admin}">Aplicar</button>`;
    default:
      return `<span class="state-label" data-state="${item.state}">${STATE_LABELS[item.state]}</span>`;
  }
}

// ------------------------------------------------------------ registro ao vivo

/**
 * O Rust emite um evento por passo do lote, antes e depois de cada otimização.
 * A interface mostra isso enquanto acontece — inclusive o valor que existia
 * antes de cada mudança. É a diferença entre acompanhar e confiar.
 */
async function listenToBatchProgress() {
  await listen<BatchStep>("optimize:step", (event) => appendLogLine(event.payload));
}

function resetLog(title: string) {
  element("live-log").hidden = false;
  element("live-log-title").textContent = title;
  element("live-log-count").textContent = "";
  element("live-log-lines").innerHTML = "";
}

function appendLogLine(step: BatchStep) {
  const lines = element("live-log-lines");

  // O passo 0 é o ponto de restauração: acontece antes do lote e não entra na
  // contagem, porque não é uma otimização.
  if (step.index === 0) {
    const entry = document.createElement("li");
    entry.className = `log-line ${step.success ? "done" : "failed"}`;
    entry.innerHTML = `
      <span class="log-name">${escapeHtml(step.name)}</span>
      <span class="log-message">${escapeHtml(step.message)}</span>
    `;
    lines.appendChild(entry);
    return;
  }

  element("live-log-count").textContent = `${step.index} de ${step.total}`;

  if (step.stage === "started") {
    const entry = document.createElement("li");
    entry.className = "log-line running";
    entry.dataset.index = String(step.index);
    entry.innerHTML = `<span class="log-name">${escapeHtml(step.name)}</span>`;
    lines.appendChild(entry);
    lines.scrollTop = lines.scrollHeight;
    return;
  }

  const entry = lines.querySelector<HTMLElement>(`li[data-index="${step.index}"]`);
  if (!entry) return;

  entry.className = `log-line ${step.success ? "done" : "failed"}`;

  const changes = step.changes.length
    ? `<ul class="log-changes">${step.changes
        .map((change) => `<li>${escapeHtml(change)}</li>`)
        .join("")}</ul>`
    : "";

  entry.innerHTML = `
    <span class="log-name">${escapeHtml(step.name)}</span>
    <span class="log-message">${escapeHtml(step.message)}</span>
    ${changes}
  `;

  lines.scrollTop = lines.scrollHeight;
}

// ---------------------------------------------------------------- elevação

/**
 * Abre o pedido de elevação. O Windows não deixa um processo ganhar privilégio
 * sozinho, então a única saída honesta é explicar e reabrir com autorização.
 */
function askForAdmin(reason: string) {
  element("modal-text").textContent = reason;
  element("admin-modal").hidden = false;
  element<HTMLButtonElement>("modal-confirm").focus();
}

function closeAdminModal() {
  element("admin-modal").hidden = true;
}

async function relaunchAsAdmin() {
  const confirm = element<HTMLButtonElement>("modal-confirm");
  confirm.disabled = true;
  confirm.textContent = "Aguardando o Windows…";

  try {
    // Na versão final este processo encerra aqui e o elevado assume. Em modo de
    // desenvolvimento ele continua vivo e devolve a explicação do porquê.
    const note = await invoke<string>("relaunch_as_admin");
    closeAdminModal();

    if (note) {
      setStatus("optimization-status", note, "ok");
    }
  } catch (error) {
    closeAdminModal();
    setStatus("optimization-status", String(error), "error");
  } finally {
    confirm.disabled = false;
    confirm.textContent = "Reabrir como administrador";
  }
}

/** Otimizações que o lote aplicaria e que dependem de privilégio elevado. */
function pendingAdminCount(): number {
  return optimizations.filter(
    (item) => item.state === "Available" && item.reversible && item.requires_admin
  ).length;
}

async function runBatch(command: string, progress: string, only?: string[]) {
  if (command === "optimize_now" && !isElevated) {
    const count = pendingAdminCount();

    if (count > 0) {
      askForAdmin(
        `${count} das otimizações pendentes mexem em serviços, energia ou registro do ` +
          `sistema, e isso exige permissão de administrador. Podemos reabrir o Otimiza ` +
          `com essa permissão?`
      );
      return;
    }
  }

  const buttons = document.querySelectorAll<HTMLButtonElement>("#optimize-now, #revert-all");
  buttons.forEach((button) => (button.disabled = true));
  setStatus("optimization-status", progress, "progress");
  resetLog(command === "optimize_now" ? "Aplicando" : "Desfazendo");

  try {
    // `only` só existe no lote de aplicar; passar em outros comandos seria
    // ruído no IPC.
    const outcomes = await invoke<OptimizationOutcome[]>(
      command,
      only ? { only } : undefined
    );

    if (outcomes.length === 0) {
      setStatus(
        "optimization-status",
        "Nada a fazer: seu PC já está com todas as otimizações reversíveis aplicadas.",
        "ok"
      );
      return;
    }

    const failures = outcomes.filter((outcome) => !outcome.success);
    const restart = outcomes.some((outcome) => outcome.success && outcome.requires_restart);

    if (failures.length > 0) {
      const detail = failures.map((f) => `${f.name}: ${f.message}`).join(" · ");
      setStatus(
        "optimization-status",
        `${outcomes.length - failures.length} de ${outcomes.length} concluídas. Falhou: ${detail}`,
        "error"
      );
    } else {
      setStatus(
        "optimization-status",
        `${outcomes.length} concluídas.${restart ? " Reinicie o PC para tudo valer." : ""}`,
        "ok"
      );
    }
  } catch (error) {
    setStatus("optimization-status", String(error), "error");
  } finally {
    buttons.forEach((button) => (button.disabled = false));
    await loadOptimizations();
  }
}

// ---------------------------------------------------------------- preferências

async function loadPreferences() {
  try {
    preferences = await invoke<Preferences>("get_preferences");
    renderPreferences();
  } catch (error) {
    console.error("Erro ao ler preferências:", error);
  }
}

function renderPreferences() {
  element<HTMLInputElement>("pref-restore").checked = preferences.restore_point_before_batch;
  element<HTMLInputElement>("pref-unavailable").checked = preferences.show_unavailable;

  document.querySelectorAll<HTMLButtonElement>("#pref-interval button").forEach((button) => {
    const chosen = Number(button.dataset.interval) === preferences.metrics_interval_seconds;
    button.setAttribute("aria-pressed", String(chosen));
  });
}

/**
 * Grava e adota o que o backend devolveu, não o que foi pedido: valores fora da
 * faixa são corrigidos na gravação, e a tela precisa mostrar o valor real.
 */
async function savePreferences(change: Partial<Preferences>) {
  const wanted = { ...preferences, ...change };

  try {
    preferences = await invoke<Preferences>("set_preferences", { preferences: wanted });
    text("preferences-status", "salvo");
    renderPreferences();
    restartMetricsLoop();
    renderOptimizations();
  } catch (error) {
    text("preferences-status", "erro ao salvar");
    console.error(error);
  }
}

// ------------------------------------------------------------ rede de segurança

async function loadRestoreStatus() {
  try {
    const status = await invoke<RestoreStatus>("restore_status");

    text("restore-tag", status.available ? "ativa" : "indisponível");
    setStatus("restore-status", status.message, status.available ? "ok" : "error");

    // O botão de ativar proteção só aparece quando faz sentido: ele consome
    // espaço em disco e é decisão do dono do PC.
    element("enable-protection").hidden = status.available;

    element("restore-list").innerHTML = status.points.length
      ? `<table class="benchmark-table"><tbody>${status.points
          .slice(0, 5)
          .map(
            (point) =>
              `<tr><td>${escapeHtml(point.description)}</td><td class="value">${escapeHtml(
                point.created_at
              )}</td></tr>`
          )
          .join("")}</tbody></table>`
      : "";
  } catch (error) {
    setStatus("restore-status", String(error), "error");
  }
}

/// Ambas as ações podem levar dezenas de segundos: o Windows tira um instantâneo
/// do volume inteiro.
async function runRestoreAction(command: "create_restore_point" | "enable_system_protection") {
  const buttons = document.querySelectorAll<HTMLButtonElement>(
    "#create-restore, #enable-protection"
  );
  buttons.forEach((button) => (button.disabled = true));
  setStatus("restore-status", "Falando com o Windows… isso pode demorar um pouco.", "progress");

  try {
    const message = await invoke<string>(command);
    setStatus("restore-status", message, "ok");
  } catch (error) {
    setStatus("restore-status", String(error), "error");
  } finally {
    buttons.forEach((button) => (button.disabled = false));
    await loadRestoreStatus();
  }
}

// ------------------------------------------------------------- inicialização

async function loadStartup() {
  try {
    const entries = await invoke<StartupEntry[]>("list_startup");
    const enabled = entries.filter((entry) => entry.enabled).length;
    text("startup-count", `${enabled} de ${entries.length} ligados`);

    // Muitos programas subindo com o Windows é a causa mais comum de PC lento
    // ao ligar. Acima de cinco, o selo fica âmbar para chamar atenção.
    setBadge("badge-sistema", enabled, enabled > 5 ? "warn" : undefined);

    element("startup-list").innerHTML = entries.length
      ? entries.map(renderStartupEntry).join("")
      : `<p class="empty">Nenhum programa nas chaves de inicialização.</p>`;
  } catch (error) {
    element("startup-list").innerHTML =
      `<p class="status error">${escapeHtml(String(error))}</p>`;
  }
}

function renderStartupEntry(entry: StartupEntry): string {
  const scope =
    entry.hive === "HKLM"
      ? `<span class="chip" title="Vale para todos os usuários">todos</span>`
      : "";

  return `
    <div class="startup" data-enabled="${entry.enabled}">
      <div class="startup-info">
        <span class="startup-name">${escapeHtml(entry.name)}</span>
        <span class="startup-exe">${escapeHtml(entry.executable || entry.command)}</span>
      </div>
      ${scope}
      <button class="btn btn-ghost"
              data-startup="${escapeHtml(entry.name)}"
              data-hive="${entry.hive}"
              data-enable="${!entry.enabled}">
        ${entry.enabled ? "Desligar" : "Ligar"}
      </button>
    </div>
  `;
}

// ----------------------------------------------------------------- medição

async function loadBaselineState() {
  try {
    const baseline = await invoke<BenchmarkSnapshot | null>("get_baseline");

    if (baseline) {
      const when = new Date(baseline.timestamp * 1000).toLocaleString("pt-BR");
      text("baseline-tag", `medido em ${when}`);
    }
  } catch (error) {
    console.error(error);
  }
}

// ------------------------------------------------------ relatório entregável

interface ReportSaved {
  path: string;
  optimizations: number;
  changes: number;
}

/** Última comparação medida nesta sessão, ou nada se ainda não houve. */
let lastComparison: BenchmarkComparison | null = null;

async function exportReport() {
  const button = element<HTMLButtonElement>("export-report");
  button.disabled = true;
  setStatus("report-status", "Montando o relatório…", "progress");

  try {
    const saved = await invoke<ReportSaved>("export_report", {
      comparison: lastComparison,
    });

    // O caminho completo importa: o técnico precisa achar o arquivo para
    // anexar num e-mail ou copiar para um pendrive.
    setStatus(
      "report-status",
      `Salvo em ${saved.path} — ${saved.optimizations} otimização(ões), ` +
        `${saved.changes} alteração(ões)` +
        (lastComparison ? "." : ", sem medição de antes e depois."),
      "ok"
    );
  } catch (error) {
    setStatus("report-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

async function runBenchmark(command: "measure_baseline" | "measure_and_compare") {
  const buttons = document.querySelectorAll<HTMLButtonElement>(
    "#measure-baseline, #measure-compare"
  );
  buttons.forEach((button) => (button.disabled = true));
  setStatus(
    "benchmark-status",
    "Medindo por cerca de 12 segundos. Não use o PC agora.",
    "progress"
  );

  try {
    if (command === "measure_baseline") {
      const result = await invoke<BaselineResult>(command);
      element("benchmark-result").innerHTML = renderSnapshot(result.snapshot);

      if (result.reliable) {
        setStatus("benchmark-status", "Medição inicial gravada. Agora otimize.", "ok");
        await loadBaselineState();
      } else {
        setStatus("benchmark-status", result.warning ?? "Medição pouco confiável.", "error");
      }
    } else {
      const comparison = await invoke<BenchmarkComparison>(command);
      element("benchmark-result").innerHTML = renderComparison(comparison);
      setStatus("benchmark-status", comparison.summary, toneOf(comparison));

      // Guardado para o relatório. Refazer a medição na hora de exportar
      // custaria mais 12 segundos e mediria um momento diferente daquele que o
      // usuário está vendo na tela.
      lastComparison = comparison;
      text("report-tag", "com medição de antes e depois");
    }
  } catch (error) {
    setStatus("benchmark-status", String(error), "error");
  } finally {
    buttons.forEach((button) => (button.disabled = false));
  }
}

function toneOf(comparison: BenchmarkComparison): "ok" | "error" | "progress" {
  if (comparison.metrics.some((metric) => metric.verdict === "Worsened")) return "error";
  if (comparison.metrics.some((metric) => metric.verdict === "Improved")) return "ok";
  return "progress";
}

function renderSnapshot(snapshot: BenchmarkSnapshot): string {
  const rows: [string, string][] = [
    ["Travada no pior caso", `${snapshot.scheduler_p99_delay_ms.toFixed(1)} ms`],
    ["Engasgos por minuto", `${snapshot.hitches_per_minute.toFixed(0)}`],
    ["1 núcleo", `${snapshot.cpu_single_thread_mops.toFixed(0)} Mops/s`],
    ["Todos os núcleos", `${snapshot.cpu_multi_thread_mops.toFixed(0)} Mops/s`],
    ["Frequência sob carga", `${snapshot.cpu_frequency_under_load_mhz.toFixed(0)} MHz`],
    ["CPU em segundo plano", `${snapshot.idle_cpu_percent.toFixed(1)} %`],
    ["RAM em segundo plano", `${snapshot.idle_ram_gb.toFixed(2)} GB`],
    ["Processos", `${snapshot.process_count.toFixed(0)}`],
  ];

  return `
    <table class="benchmark-table">
      <tbody>
        ${rows
          .map(([label, value]) => `<tr><td>${label}</td><td class="value">${value}</td></tr>`)
          .join("")}
      </tbody>
    </table>
  `;
}

function renderComparison(comparison: BenchmarkComparison): string {
  const rows = comparison.metrics
    .map((metric) => {
      const decimals = metric.unit === "GB" ? 2 : metric.unit === "%" ? 1 : 0;
      const sign = metric.change_percent > 0 ? "+" : "";
      const change =
        metric.verdict === "NoMeasurableChange"
          ? "—"
          : `${sign}${metric.change_percent.toFixed(1)}%`;

      return `
        <tr>
          <td>
            ${escapeHtml(metric.label)}
            <div class="metric-note">${escapeHtml(metric.explanation)}</div>
          </td>
          <td class="value">${metric.before.toFixed(decimals)}</td>
          <td class="value">${metric.after.toFixed(decimals)}</td>
          <td class="value">${change}</td>
          <td><span class="verdict" data-verdict="${metric.verdict}">${VERDICT_LABELS[metric.verdict]}</span></td>
        </tr>`;
    })
    .join("");

  return `
    <table class="benchmark-table">
      <thead>
        <tr><th>Indicador</th><th>Antes</th><th>Depois</th><th>Var.</th><th>Veredito</th></tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

function setStatus(id: string, message: string, kind: "ok" | "warn" | "error" | "progress") {
  const status = element(id);
  status.textContent = message;
  status.className = `status ${kind}`;
}

// ---------------------------------------------------------------- controles

function wireControls() {
  const tabs = Array.from(document.querySelectorAll<HTMLButtonElement>(".tab"));

  tabs.forEach((tab) => {
    tab.addEventListener("click", () => showTab(tab.dataset.tab!));
  });

  // Setas navegam entre abas, como manda o padrão de acessibilidade para
  // barras de abas — e é como quem usa teclado espera que funcione.
  document.querySelector(".tabs")!.addEventListener("keydown", (event) => {
    const key = (event as KeyboardEvent).key;
    if (key !== "ArrowRight" && key !== "ArrowLeft") return;

    const current = tabs.findIndex((tab) => tab.getAttribute("aria-selected") === "true");
    const next = (current + (key === "ArrowRight" ? 1 : tabs.length - 1)) % tabs.length;

    showTab(tabs[next].dataset.tab!);
    tabs[next].focus();
  });

  element("run-diagnostic").addEventListener("click", runDiagnostic);
  element("analyze-firmware").addEventListener("click", analyzeFirmware);

  element("analyze-boot").addEventListener("click", analyzeBoot);
  element("analyze-thermal").addEventListener("click", analyzeThermal);
  element("analyze-health").addEventListener("click", analyzeHealth);
  element("analyze-conflicts").addEventListener("click", analyzeConflicts);

  element("analyze-bloat").addEventListener("click", analyzeBloatware);

  element("open-apps").addEventListener("click", async () => {
    try {
      const message = await invoke<string>("open_apps_settings");
      setStatus("bloat-status", message, "ok");
    } catch (error) {
      setStatus("bloat-status", String(error), "error");
    }
  });

  element("bloat-result").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-bloat]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    button.disabled = true;

    try {
      const message = await invoke<string>("remove_store_app", {
        package: button.dataset.bloat,
      });
      setStatus("bloat-status", message, "ok");
    } catch (error) {
      setStatus("bloat-status", String(error), "error");
    } finally {
      await analyzeBloatware();
    }
  });

  element("tasks-list").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-task]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    // Mexer no agendador exige elevação; pedir antes evita erro seco na tela.
    if (!isElevated) {
      askForAdmin(
        `Ligar e desligar tarefas agendadas exige permissão de administrador. ` +
          `Podemos reabrir o Otimiza com essa permissão?`
      );
      return;
    }

    button.disabled = true;

    try {
      const outcome = await invoke<OptimizationOutcome>("set_scheduled_task", {
        path: button.dataset.taskpath,
        name: button.dataset.task,
        enabled: button.dataset.enable === "true",
      });
      setStatus("tasks-status", outcome.message, outcome.success ? "ok" : "error");
    } catch (error) {
      setStatus("tasks-status", String(error), "error");
    } finally {
      await loadScheduledTasks();
    }
  });

  element("services-list").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-service]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    if (!isElevated) {
      askForAdmin(
        `Mudar o início de um serviço exige permissão de administrador. ` +
          `Podemos reabrir o Otimiza com essa permissão?`
      );
      return;
    }

    button.disabled = true;

    try {
      const outcome = await invoke<OptimizationOutcome>("set_service_start", {
        name: button.dataset.service,
        automatic: button.dataset.auto === "true",
      });
      setStatus("services-status", outcome.message, outcome.success ? "ok" : "error");
    } catch (error) {
      setStatus("services-status", String(error), "error");
    } finally {
      await loadThirdPartyServices();
    }
  });

  element("scan-disk").addEventListener("click", scanDiskSpace);
  element("map-folders").addEventListener("click", mapFolders);

  element("disk-result").addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-space]"
    ) as HTMLButtonElement | null;
    if (button) cleanDiskCategory(button.dataset.space!, button);
  });

  element("empty-recycle").addEventListener("click", async () => {
    const button = element<HTMLButtonElement>("empty-recycle");
    button.disabled = true;

    try {
      const message = await invoke<string>("empty_recycle_bin");
      setStatus("disk-status", message, "ok");
    } catch (error) {
      setStatus("disk-status", String(error), "error");
    } finally {
      button.disabled = false;
    }
  });

  element("analyze-memory").addEventListener("click", analyzeMemory);

  element("fix-pagefile").addEventListener("click", async () => {
    const button = element<HTMLButtonElement>("fix-pagefile");
    button.disabled = true;

    try {
      const message = await invoke<string>("set_automatic_pagefile");
      setStatus("memory-status", message, "ok");
      await analyzeMemory();
    } catch (error) {
      setStatus("memory-status", String(error), "error");
    } finally {
      button.disabled = false;
    }
  });

  element("pref-restore").addEventListener("change", (event) =>
    savePreferences({
      restore_point_before_batch: (event.target as HTMLInputElement).checked,
    })
  );

  element("pref-unavailable").addEventListener("change", (event) =>
    savePreferences({ show_unavailable: (event.target as HTMLInputElement).checked })
  );

  element("pref-interval").addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest("button[data-interval]");
    if (!button) return;

    savePreferences({
      metrics_interval_seconds: Number((button as HTMLElement).dataset.interval),
    });
  });

  element("create-restore").addEventListener("click", () =>
    runRestoreAction("create_restore_point")
  );
  element("enable-protection").addEventListener("click", () =>
    runRestoreAction("enable_system_protection")
  );

  element("measure-baseline").addEventListener("click", () => runBenchmark("measure_baseline"));
  element("measure-compare").addEventListener("click", () => runBenchmark("measure_and_compare"));
  element("export-report").addEventListener("click", exportReport);

  element("optimize-now").addEventListener("click", () =>
    runBatch("optimize_now", "Aplicando o que falta…")
  );

  element("profile-chips").addEventListener("click", (event) => {
    const chip = (event.target as HTMLElement).closest(
      "button[data-profile]"
    ) as HTMLButtonElement | null;
    if (chip) selectProfile(chip.dataset.profile!);
  });

  // O botão de aplicar o perfil é redesenhado a cada escolha, então a escuta
  // fica no painel que sobrevive, não no botão.
  element("profile-detail").addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest("#apply-profile");
    if (!button) return;

    const perfil = profiles.find((p) => p.id === activeProfile);
    if (!perfil) return;

    runBatch(
      "optimize_now",
      `Aplicando o perfil ${perfil.name}…`,
      perfil.optimization_ids
    );
  });

  const busca = element<HTMLInputElement>("optimization-search");
  busca.addEventListener("input", () => {
    searchTerm = busca.value;
    element("search-clear").hidden = searchTerm.length === 0;
    renderOptimizations();
  });

  // Esc limpa a busca: é o gesto que a pessoa já tem na mão vindo de qualquer
  // outro programa.
  busca.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      busca.value = "";
      searchTerm = "";
      element("search-clear").hidden = true;
      renderOptimizations();
    }
  });

  element("search-clear").addEventListener("click", () => {
    busca.value = "";
    searchTerm = "";
    element("search-clear").hidden = true;
    busca.focus();
    renderOptimizations();
  });
  element("revert-all").addEventListener("click", () =>
    runBatch("revert_all_optimizations", "Desfazendo…")
  );

  element("modal-confirm").addEventListener("click", relaunchAsAdmin);
  element("modal-cancel").addEventListener("click", closeAdminModal);

  element("admin-modal").addEventListener("click", (event) => {
    // Clicar fora do cartão fecha, como em qualquer diálogo.
    if (event.target === element("admin-modal")) closeAdminModal();
  });

  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape") closeAdminModal();
  });

  element("startup-list").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-startup]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    const hive = button.dataset.hive!;
    const enable = button.dataset.enable === "true";

    // Entradas de HKLM valem para todos os usuários e exigem elevação.
    if (hive === "HKLM" && !isElevated) {
      askForAdmin(
        `"${button.dataset.startup}" inicia para todos os usuários do PC, e mexer nisso ` +
          `precisa de permissão de administrador. Podemos reabrir o Otimiza com essa permissão?`
      );
      return;
    }

    button.disabled = true;

    try {
      const outcome = await invoke<OptimizationOutcome>("set_startup_enabled", {
        hive,
        name: button.dataset.startup,
        enabled: enable,
      });
      setStatus("startup-status", outcome.message, outcome.success ? "ok" : "error");
    } catch (error) {
      setStatus("startup-status", String(error), "error");
    } finally {
      await loadStartup();
    }
  });

  element("filters").addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest("button[data-category]");
    if (!button) return;

    activeCategory = (button as HTMLElement).dataset.category as Category | "Todas";
    renderFilters();
    renderOptimizations();
  });

  // Um listener na lista cobre todos os botões, inclusive os recriados a cada
  // recarregamento do estado.
  element("optimization-list").addEventListener("click", async (event) => {
    // Lembrar quais grupos o usuário recolheu, já que a lista é redesenhada
    // inteira a cada ação.
    const group = (event.target as HTMLElement).closest(
      "summary.group-head"
    )?.parentElement as HTMLDetailsElement | null;

    if (group?.dataset.category) {
      if (group.open) collapsedGroups.add(group.dataset.category);
      else collapsedGroups.delete(group.dataset.category);
    }

    const button = (event.target as HTMLElement).closest(
      "button[data-id]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    // O botão vive dentro de um <summary>: sem isto, clicar em "Aplicar"
    // também abriria e fecharia os detalhes do item.
    event.preventDefault();

    if (button.dataset.admin === "true" && !isElevated) {
      const item = optimizations.find((entry) => entry.id === button.dataset.id);
      askForAdmin(
        `"${item?.name ?? "Esta otimização"}" mexe em configurações protegidas do ` +
          `Windows e precisa de permissão de administrador. Podemos reabrir o Otimiza ` +
          `com essa permissão?`
      );
      return;
    }

    const command =
      button.dataset.action === "revert" ? "revert_optimization" : "apply_optimization";
    button.disabled = true;

    try {
      const outcome = await invoke<OptimizationOutcome>(command, { id: button.dataset.id });
      setStatus("optimization-status", outcome.message, outcome.success ? "ok" : "error");
    } catch (error) {
      setStatus("optimization-status", String(error), "error");
    } finally {
      await loadOptimizations();
    }
  });
}
