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
  ]);

  await startMonitoring();
});

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
    const metrics = await invoke<any>("get_performance_metrics");
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

function renderMetrics(metrics: any) {
  const cpu = Math.min(100, Math.max(0, metrics.cpu.overall));

  // Anel principal. O perímetro (2πr, r=86) é 540, igual ao dasharray do CSS.
  const gauge = element<SVGCircleElement & HTMLElement>("gauge-cpu");
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

  text("status-right", `atualizado às ${new Date().toLocaleTimeString("pt-BR")}`);
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

  if (matrix.children.length !== perCore.length) {
    matrix.innerHTML = perCore.map(() => `<div class="core"><i></i></div>`).join("");
  }

  perCore.forEach((load, index) => {
    const core = matrix.children[index] as HTMLElement;
    const fill = core.firstElementChild as HTMLElement;
    fill.style.height = `${Math.min(100, Math.max(0, load))}%`;
    core.dataset.load = load >= 85 ? "critical" : load >= 60 ? "high" : "normal";
  });
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

function renderFinding(finding: FirmwareFinding): string {
  const advice = finding.advice
    ? `<p class="finding-advice">${escapeHtml(finding.advice)}</p>`
    : "";

  return `
    <article class="finding" data-severity="${finding.severity}">
      <div class="finding-top">
        <h3>${escapeHtml(finding.title)}</h3>
        <span class="chip" data-fix="${finding.fix_location}">${FIX_LABELS[finding.fix_location]}</span>
      </div>
      <p class="finding-measured">${escapeHtml(finding.measured)}</p>
      ${advice}
    </article>
  `;
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

function renderOptimizations() {
  const visible = optimizations
    .filter((item) => preferences.show_unavailable || item.state !== "Unavailable")
    .filter((item) => activeCategory === "Todas" || item.category === activeCategory);

  const available = optimizations.filter((item) => item.state === "Available").length;
  const applied = optimizations.filter((item) => item.state === "Applied").length;
  text("optimization-count", `${available} a aplicar · ${applied} ativas`);
  setBadge("badge-otimizacoes", available);

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

  return `
    <details class="opt-group"${open} data-category="${category}">
      <summary class="group-head">
        <span>${CATEGORY_LABELS[category]}</span>
        <span class="group-count">${summary} · ${items.length}</span>
      </summary>
      ${items.map(renderOptimization).join("")}
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

  if (item.requires_restart) chips.push(`<span class="chip">exige reiniciar</span>`);
  if (item.requires_admin) chips.push(`<span class="chip">administrador</span>`);
  if (!item.reversible) chips.push(`<span class="chip" data-warn="true">sem volta</span>`);
  if (item.security_tradeoff)
    chips.push(`<span class="chip" data-warn="true">reduz segurança</span>`);

  const detail = item.detail ? `<p class="detail">${escapeHtml(item.detail)}</p>` : "";

  return `
    <details class="optimization" data-state="${item.state}">
      <summary class="opt-row">
        <span class="gain-dot" data-gain="${item.expected_gain}"></span>
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

async function runBatch(command: string, progress: string) {
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
    const outcomes = await invoke<OptimizationOutcome[]>(command);

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

function setStatus(id: string, message: string, kind: "ok" | "error" | "progress") {
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

  element("optimize-now").addEventListener("click", () =>
    runBatch("optimize_now", "Aplicando o que falta…")
  );
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
