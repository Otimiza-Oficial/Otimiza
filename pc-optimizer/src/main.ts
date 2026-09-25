import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Esfera } from "./esfera";
import { Pilares } from "./pilares";
import { ligarBarraDaJanela } from "./janela";
import { carregarLaboratorioDeGeracao } from "./framegen";
import { carregarMotorDeEnergia } from "./energia";
import { carregarAbaBios } from "./bios";
import { carregarMapaDeDesempenho } from "./mapa";
import { ligarProntidao } from "./prontidao";

type Verdict = "Improved" | "Worsened" | "NoMeasurableChange" | "TooNoisyToJudge";
type State = "Applied" | "AlreadyOptimal" | "Available" | "Unavailable" | "Unknown";
type Gain = "Measurable" | "Situational" | "Responsiveness" | "NoGain";
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
  /** Retirado na 2.9: só aparece enquanto aplicado, para poder ser desfeito. */
  retirado?: boolean;
  /** Só no modo Expert (2.9). */
  expert?: boolean;
  /** Item condicional: por que ele aparece nesta máquina. */
  condicao?: string | null;
  /** O backend manda o ESTADO e o motivo (incidente da 2.1.0); a tela só escolhe como mostrar. */
  risco_de_fps:
    | { risco: "Nenhum" }
    | { risco: "PodeCustar"; o_que: string; rotulo: string; quando: string };
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
  /** Ligar o modo jogo sozinho. Desligado de fábrica, de propósito. */
  auto_game_mode: boolean;
  metrics_interval_seconds: number;
  show_unavailable: boolean;
  /** Medir os quadros sozinho durante as partidas. Ligado de fábrica: não muda nada no sistema. */
  medir_quadros_sozinho: boolean;
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

/**
 * A tela decide só por esta variante, nunca pelo texto de `versaoPublicada`; a guarda de `commands.rs` reprova
 * comparação por prosa.
 */
type Comparacao = "NaoHaNova" | "HaVersaoNova" | "NaoSei";

interface AvisoDeVersao {
  comparacao: Comparacao;
  versao_publicada: string | null;
  pagina: string | null;
}

interface StartupEntry {
  classe?: "Essencial" | "Util" | "Opcional" | "Desconhecido";
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

interface Achado {
  id: string;
  origem: string;
  causa: string;
  title: string;
  measured: string;
  advice: string;
  severity: Severity;
  fix_location: string;
  confianca: string;
  // Preenchido só quando o Otimiza sabe consertar sozinho.
  acao: Acao | null;
}

interface Lacuna {
  origem: string;
  o_que: string;
  por_que: string;
}

interface Acao {
  comando: string;
  argumento: string | null;
  rotulo: string;
  exige_admin: boolean;
}

interface Veredito {
  frase: string;
  detalhe: string;
  principal: Achado | null;
  corroboracoes: Achado[];
  achados: Achado[];
  lacunas: Lacuna[];
  recuperacao: { perdido: "Sim" | "Nao" | "Incerto"; itens: [string, "P0" | "P1" | "P2" | "P3"][] };
}

const PRIORIDADE: Record<"P0" | "P1" | "P2" | "P3", string> = {
  P0: "P0 · erro crítico de configuração",
  P1: "P1 · oportunidade grande",
  P2: "P2 · oportunidade moderada",
  P3: "P3 · pequena",
};

const PERDIDO: Record<"Sim" | "Nao" | "Incerto", string> = {
  Sim: "Há desempenho perdido nesta máquina — o hardware pode entregar mais do que está entregando.",
  Nao: "Nenhum desempenho perdido encontrado: a configuração desta máquina está certa. Ganho daqui para frente depende do jogo e das peças.",
  Incerto: "Talvez haja desempenho perdido: os indícios vêm de configuração lida, sem medir o efeito, ou algo não pôde ser verificado.",
};

function mostrarRecuperacao(v: Veredito) {
  const bloco = element("veredito-recuperacao");
  const r = v.recuperacao;
  if (!r) {
    bloco.hidden = true;
    return;
  }
  const porId = new Map(v.achados.map((a) => [a.id, a]));
  const itens = r.itens
    .map(([id, p]) => {
      const a = porId.get(id);
      if (!a) return "";
      return `<li><span class="chip" data-warn="${p === "P0" || p === "P1"}">${PRIORIDADE[p]}</span> <strong>${escapeHtml(a.title)}</strong> — ${escapeHtml(a.measured)}</li>`;
    })
    .join("");
  bloco.hidden = false;
  bloco.innerHTML = `<p class="veredito-detalhe"><strong>Desempenho perdido:</strong> ${PERDIDO[r.perdido]}</p>${itens ? `<ul class="veredito-junto">${itens}</ul>` : ""}`;
}

interface ConflictReport {
  conflicts: Conflict[];
  /** `null` quando a lista de programas não deu para ler. */
  programs_scanned: number | null;
  /** O que não deu para ler. Com algo aqui, a ausência de conflito não é afirmada. */
  lacunas: string[];
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
  /** O que não deu para ler — hoje, os aplicativos da Loja com o serviço deles desligado. */
  lacunas: string[];
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
  // Tipado: sem ele, "não consegui estimar" chega como 0 e a tela pinta "vazio" em verde.
  medida: { tipo: "Medido" } | { tipo: "NaoConsegui" };
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

/** MEASURED e ESTIMATED são número para mostrar; UNKNOWN é texto, nunca zero (ver `modules/telemetry.rs`). */
type Quality = "MEASURED" | "ESTIMATED" | "UNKNOWN";

interface Metric {
  /** `null` sempre que a qualidade é UNKNOWN. */
  value: number | null;
  unit: string;
  source: string;
  quality: Quality;
  /** Por que não foi medido, ou por que é apenas estimado. */
  reason: string | null;
  /** `null` quando é desta coleta: a tela mostra a idade em vez de fingir que é de agora. */
  age_ms: number | null;
}

interface TelemetrySummary {
  measured: number;
  estimated: number;
  unknown: number;
  total: number;
}

interface Telemetry {
  schema_version: number;
  collected_at: number;
  since_previous_ms: number | null;
  /** Inclui a espera de amostragem da CPU. Não é percentual de overhead. */
  collection_duration_ms: number;
  summary: TelemetrySummary;
  metrics: Record<string, Metric>;
}

/** Versão diferente é motivo para avisar: um campo que mudou de significado continuaria desenhando bonito. */
const TELEMETRIA_SUPORTADA = 1;

interface PerformanceMetrics {
  timestamp: number;
  cpu: {
    overall: number | null;
    per_core: number[];
    temperature: number | null;
    frequency: number | null;
  };
  ram: {
    total_gb: number;
    used_gb: number;
    available_gb: number;
    cached_gb: number | null;
    usage_percent: number | null;
  };
  disk: {
    read_speed_mbps: number | null;
    write_speed_mbps: number | null;
    /** Espaço ocupado, não atividade. */
    usage_percent: number | null;
  };
  network: {
    download_speed_mbps: number | null;
    upload_speed_mbps: number | null;
    total_received_gb: number;
    total_transmitted_gb: number;
  };
  uptime_hours: number;
  telemetry: Telemetry;
  gargalo: Diagnostico;
  vram: AnaliseVram;
  latencia: Orcamento;
}

type Etapa = "Entrada" | "JogoEPlaca" | "Fila" | "Apresentacao" | "Tela";

interface Parcela {
  etapa: Etapa;
  /** Ausente quando não foi medida. Nunca zero. */
  ms: number | null;
  qualidade: Quality;
  origem: string;
}

interface Orcamento {
  parcelas: Parcela[];
  /** Limite inferior: a soma só do que foi medido. */
  piso_ms: number | null;
  etapas_com_valor: number;
  etapas_totais: number;
  observacoes: string[];
}

type EstadoVram =
  | "NaoAvaliado"
  | "PlacaIntegrada"
  | "Folgada"
  | "CacheCheio"
  | "Transbordando"
  | "DerramaSemPressao";

interface AnaliseVram {
  estado: EstadoVram;
  dedicada_pct: number | null;
  folga_gb: number | null;
  /** Quanto foi para a memória do sistema ACIMA do piso desta máquina. */
  derramado_gb: number | null;
  piso_gb: number | null;
  falta: string[];
  explicacao: string;
  conselho: { liberar_gb: number; texto: string } | null;
}

type Classe =
  | "CpuTodosNucleos"
  | "CpuUmNucleo"
  | "Gpu"
  | "MemoriaRam"
  | "MemoriaVideo"
  | "Disco"
  | "LimiteTermico"
  | "LimiteEletrico"
  | "TetoDeQuadros"
  | "Engasgo"
  | "ForaDoHardware"
  | "StreamingDeAssets"
  | "Rede";

/** `Causa` = o sistema afirmou o fato agora. `Hipotese` = indireto ou velho. */
type Forca = "Causa" | "Hipotese";

interface Achado {
  classe: Classe;
  forca: Forca;
  /** O número que sustenta o achado, com o id da métrica. */
  evidencia: string;
  idade_ms: number | null;
}

interface NaoVerificado {
  classe: string;
  falta: string;
}

interface Diagnostico {
  conclusao: "SemEvidencia" | "SemCarga" | "NadaNoLimite" | "Encontrado";
  achados: Achado[];
  nao_verificado: NaoVerificado[];
  classes_avaliadas: number;
  classes_totais: number;
}

type ActionStatus =
  | "Verified"
  | "AlreadyOptimized"
  | "Unsupported"
  | "Failed"
  | "VerificationFailed"
  | "NotConfirmed"
  | "Skipped";

/** Mostrado pouco hoje, mas é o que vai no relatório que o atendimento pede. */
interface ActionResult {
  name: string;
  status: ActionStatus;
  message: string;
  before_value: string | null;
  expected_value: string | null;
  after_value: string | null;
  exit_code: number | null;
  stdout: string;
  stderr: string;
  duration_ms: number;
  unsupported_reason: string | null;
}

interface OptimizationOutcome {
  id: string;
  name: string;
  success: boolean;
  applied: boolean;
  message: string;
  requires_restart: boolean;
  requires_logoff: boolean;
  duration_ms: number;
  changes_count: number;
  actions: ActionResult[];
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

const HISTORY_SAMPLES = 60; // 2 minutos a cada 2s
const cpuHistory: number[] = [];
let optimizations: OptimizationInfo[] = [];
let activeCategory: Category | "Todas" = "Todas";
let isElevated = false;
let preferences: Preferences = {
  restore_point_before_batch: true,
  auto_game_mode: false,
  metrics_interval_seconds: 2,
  show_unavailable: true,
  medir_quadros_sozinho: true,
};
let metricsTimer: number | null = null;
/** Categorias recolhidas pelo usuário, preservadas entre recarregamentos da lista. */
const collapsedGroups = new Set<string>();

/**
 * "Resposta do sistema" era o rótulo de `Responsiveness`, que o backend define como "não muda FPS": o rótulo
 * diz o que o código sempre soube.
 */
const GAIN_LABELS: Record<Gain, string> = {
  Measurable: "muda o FPS",
  Situational: "depende da máquina",
  Responsiveness: "não muda FPS",
  NoGain: "não muda desempenho",
};

/** Menor vem primeiro. O que muda o jogo aparece antes do que não muda. */
const GAIN_ORDER: Record<Gain, number> = {
  Measurable: 0,
  Situational: 1,
  Responsiveness: 2,
  NoGain: 3,
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
  // "Não se aplica" afirma algo do computador; com a leitura falha, o produto não sabe nada do item.
  Unknown: "não deu para verificar",
};

const VERDICT_LABELS: Record<Verdict, string> = {
  Improved: "melhorou",
  Worsened: "piorou",
  NoMeasurableChange: "sem diferença",
  TooNoisyToJudge: "só referência",
};

/**
 * Três invariantes de tela, só em desenvolvimento: tela poluída nasce por acréscimo (o painel 22, o décimo botão
 * de ênfase, a caixa vazia sem dizer o que vai aparecer).
 */
function conferirInvariantesDaTela() {
  // `import.meta.env` não está nos tipos: em produção o app roda de `tauri://`, nunca de `localhost`.
  if (!location.hostname.startsWith("localhost")) return;

  const paineis = document.querySelectorAll(".panel").length;
  const enfase = document.querySelectorAll(".palco .btn-primary").length;
  // Caixa que nasce escondida nunca é vista vazia: cobrar texto dela dava falsos positivos.
  const caixasMudas = [...document.querySelectorAll(".resultado")].filter(
    (caixa) => !(caixa as HTMLElement).hidden && !(caixa as HTMLElement).dataset.vazio
  );

  console.assert(paineis <= 35, `painéis demais na tela: ${paineis}`);
  console.assert(enfase <= 8, `ênfase primária demais: ${enfase} botões`);
  console.assert(
    caixasMudas.length === 0,
    `caixa de resultado sem dizer o que vai aparecer: ${caixasMudas
      .map((c) => c.id)
      .join(", ")}`
  );

  // `.empty` serve a "Lendo…" e a "Nada encontrado": o pulso é só do primeiro. Reticências e pulso andam juntos.
  const desencontradas = [...document.querySelectorAll<HTMLElement>(".empty")].filter(
    (caixa) => {
      const esperando = (caixa.textContent ?? "").trim().endsWith("…");
      return esperando !== caixa.classList.contains("carregando");
    }
  );

  console.assert(
    desencontradas.length === 0,
    `mensagem vazia com pulso e texto em desacordo: ${desencontradas
      .map((c) => `"${c.textContent?.trim()}"`)
      .join(", ")}`
  );
}

/*
 * O portão da licença. Esta tela NÃO é o bloqueio (é HTML com ferramentas de desenvolvedor): o bloqueio está
 * no Rust, nos comandos que alteram o computador. E o diagnóstico continua rodando por trás.
 */

/**
 * O convite do Discord de RESERVA (o embutido): `resolverConvite()` pergunta ao repositório no clique e só cai
 * aqui sem resposta (sem internet, arquivo fora do ar). Convite errado é venda perdida sem ninguém saber; troque
 * por um com "Expira em: Nunca".
 */
const CONVITE_DISCORD = "https://discord.gg/ultimus";

/** Nunca lança: o endereço de suporte não pode quebrar por falha de rede. */
async function resolverConvite(): Promise<string> {
  try {
    return await invoke<string>("convite_do_discord", { embutido: CONVITE_DISCORD });
  } catch {
    return CONVITE_DISCORD;
  }
}

/** No rodapé do console: quem fica confuso semanas depois está com o programa aberto, não com o Discord. */
const TUTORIAL_URL = "https://youtu.be/6bmxhhEJGoo";

interface EstadoLicenca {
  ativa: boolean;
  maquina: string;
  origem: string;
  sobrevive_formatacao: boolean;
  comprador: string | null;
  expira: string | null;
  motivo: string | null;
}

let portaoAberto = false;

/**
 * Antes de tudo no arranque. Se a chamada falhar, o portão fica fechado e o programa abre: o backend continua
 * recusando o que altera a máquina.
 */
async function montarPortao() {
  let estado: EstadoLicenca;

  try {
    estado = await invoke<EstadoLicenca>("licenca_estado");
  } catch {
    return;
  }

  ligarBotoesDoPortao();

  if (estado.ativa) return;

  abrirPortao(estado);
}

function abrirPortao(estado: EstadoLicenca) {
  const portao = element("portao");
  portao.hidden = false;
  portaoAberto = true;

  // Sem isto o Tab passeia por trás da tela.
  document.querySelector(".console")?.setAttribute("inert", "");

  text("portao-id", estado.maquina || "não identificado");
  text(
    "portao-nota",
    estado.maquina
      ? `Vem do ${estado.origem}. ` +
          (estado.sobrevive_formatacao
            ? "Formatar o Windows não muda este código; trocar a placa-mãe muda."
            : "Formatar o Windows muda este código, e nesse caso a chave precisa " +
              "ser reemitida no Discord — é sem custo.")
      : "Não foi possível identificar este computador, o que costuma acontecer " +
          "em máquina virtual. Fale no Discord antes de comprar."
  );

  const convite = element("portao-discord") as HTMLAnchorElement;

  // O embutido JÁ, o resolvido quando chegar: quem clicar no instante da abertura não encontra link vazio.
  convite.href = CONVITE_DISCORD;
  text("portao-discord-endereco", CONVITE_DISCORD.replace(/^https?:\/\//, ""));

  void resolverConvite().then((endereco) => {
    convite.href = endereco;
    text("portao-discord-endereco", endereco.replace(/^https?:\/\//, ""));
  });

  // Chave que parou de valer precisa dizer o motivo: senão quem pagou vê a tela de quem nunca comprou.
  if (estado.motivo) {
    const erro = element("portao-erro");
    erro.hidden = false;
    erro.textContent = estado.motivo;
  }

  element("portao-chave").focus();
}

/** A chegada entre colar a chave e usar o programa. O nome vem da própria licença, sem consulta. */
function mostrarChegada(estado: EstadoLicenca) {
  const nome = primeiroNome(estado.comprador);
  const tela = element("chegada");

  text("chegada-titulo", nome ? `Obrigado, ${nome}.` : "Obrigado.");

  text(
    "chegada-frase",
    "O Otimiza está liberado neste computador. O diagnóstico já rodou enquanto "
    + "você ativava — o que ele encontrou está do outro lado deste botão."
  );

  tela.hidden = false;

  document.querySelector(".console")?.setAttribute("inert", "");

  element<HTMLButtonElement>("chegada-entrar").focus();

  element("chegada-entrar").onclick = () => {
    tela.hidden = true;
    document.querySelector(".console")?.removeAttribute("inert");

    // Comprou por causa de um problema: o Painel é o veredito, a resposta pela qual pagou.
    showTab("painel");
  };
}

function fecharPortao() {
  element("portao").hidden = true;
  portaoAberto = false;
  document.querySelector(".console")?.removeAttribute("inert");
}

function ligarBotoesDoPortao() {
  const campo = element("portao-chave") as HTMLTextAreaElement;
  const botao = element("portao-ativar-btn") as HTMLButtonElement;
  const erro = element("portao-erro");

  element("portao-copiar").addEventListener("click", async () => {
    const copiar = element("portao-copiar");

    try {
      await navigator.clipboard.writeText(element("portao-id").textContent ?? "");
      copiar.textContent = "Copiado";
      window.setTimeout(() => (copiar.textContent = "Copiar"), 1600);
    } catch {
      // A área de transferência pode ser negada; o código continua selecionável (`user-select: all`).
      copiar.textContent = "Selecione e copie";
      window.setTimeout(() => (copiar.textContent = "Copiar"), 2600);
    }
  });

  const ativar = async () => {
    const chave = campo.value.trim();

    if (!chave) {
      erro.hidden = false;
      erro.textContent = "Cole a chave que você recebeu no Discord.";
      campo.focus();
      return;
    }

    botao.disabled = true;
    botao.textContent = "Conferindo…";
    erro.hidden = true;

    try {
      const estado = await invoke<EstadoLicenca>("licenca_ativar", { chave });
      botao.textContent = "Ativado";
      fecharPortao();
      mostrarChegada(estado);
    } catch (falha) {
      erro.hidden = false;
      erro.textContent = String(falha);
      botao.disabled = false;
      botao.textContent = "Ativar o Otimiza";
      campo.focus();
    }
  };

  botao.addEventListener("click", () => void ativar());

  // Shift+Enter quebra linha: a chave colada de uma mensagem vem partida.
  campo.addEventListener("keydown", (evento) => {
    if (evento.key === "Enter" && !evento.shiftKey) {
      evento.preventDefault();
      void ativar();
    }
  });
}

/**
 * O que o Otimiza consegue e não consegue NESTE computador antes de pagar (nasceu de um reembolso cujo teto era
 * peça). Pelos achados: `acao` quando corrige sozinho, `fix_location` Hardware quando só peça resolve. Sem
 * promessa de FPS.
 */
function mostrarExpectativaNoPortao(v: Veredito) {
  const caixa = element("portao-expectativa");
  const corrige = v.achados.filter((achado) => achado.acao !== null).length;
  const pecas = v.achados.filter((achado) => achado.fix_location === "Hardware");

  if (corrige === 0 && pecas.length === 0) {
    caixa.hidden = true;
    return;
  }

  const partes: string[] = [];

  if (corrige > 0) {
    partes.push(
      corrige === 1
        ? "O Otimiza corrige sozinho 1 dos problemas que o diagnóstico achou."
        : `O Otimiza corrige sozinho ${corrige} dos problemas que o diagnóstico achou.`
    );
  }

  if (pecas.length > 0) {
    // Dois no máximo: a lista inteira viraria catálogo de defeitos.
    const nomes = pecas
      .slice(0, 2)
      .map((achado) => `"${achado.title}"`)
      .join(" e ");
    const mais = pecas.length > 2 ? ` e mais ${pecas.length - 2}` : "";

    partes.push(`Não resolve ${nomes}${mais}: isso é peça, e nenhum programa troca peça.`);
  }

  caixa.hidden = false;
  caixa.textContent = partes.join(" ");
}

/** Só o principal: repetir a lista seria assustar para vender. */
function mostrarAchadoNoPortao(v: Veredito) {
  if (!portaoAberto) return;

  mostrarExpectativaNoPortao(v);

  const caixa = element("portao-achado");

  if (!v.principal) {
    caixa.hidden = true;
    return;
  }

  caixa.hidden = false;

  // O ponto tem a cor do achado: âmbar fixo ao lado de "memória no limite" desencontrava.
  caixa.dataset.nivel = v.principal.severity === "Critical" ? "critico" : "importante";

  text("portao-achado-titulo", v.frase);
  text("portao-achado-detalhe", v.detalhe);
}

/** Uma só instância, viva enquanto o programa vive. */
let esfera: Esfera | null = null;

/**
 * As telas grandes usam os pilares, não a esfera: ampliada ela virava uma bola de pontinhos; os pilares dizem
 * QUAL peça está mal.
 */
let pilaresDoPortao: Pilares | null = null;
let pilaresDaChegada: Pilares | null = null;

/** Sem isto, a do portão ficaria "não medida" enquanto a do painel mostrava a máquina. */
function alimentarEsferas(leitura: Parameters<Esfera["atualizar"]>[0]) {
  esfera?.atualizar(leitura);
  esfera?.redesenhar();
}

/** Os pilares precisam de disco, que a esfera não usa. */
function alimentarPilares(leitura: Parameters<Pilares["atualizar"]>[0]) {
  pilaresDoPortao?.atualizar(leitura);
  pilaresDaChegada?.atualizar(leitura);
}

/** O bot grava `fulano#1234 (1234567890)`: a pessoa quer ver o nome, não o número. */
function primeiroNome(comprador: string | null): string | null {
  if (!comprador) return null;

  // Entre parênteses vem o código da compra (`CMP-2026-000004`), não só dígitos: o `split` abaixo salvava por
  // acidente, e deixaria de salvar com "Ana Paula".
  const semCodigo = comprador.replace(/\s*\([^)]*\)\s*$/, "").trim();
  const semTag = semCodigo.replace(/#\d{4}$/, "").trim();

  // "Obrigado, conferencia" seria pior que só "Obrigado".
  if (!semTag || semTag.length < 2 || /^conferencia$/i.test(semTag)) return null;

  const primeiro = semTag.split(/\s+/)[0];

  // Apelido pode terminar em pontuação: "Obrigado, exaggerateyourdreams..".
  const limpo = primeiro.replace(/[.,;:!?]+$/, "");

  return limpo.length >= 2 ? limpo : null;
}

window.addEventListener("DOMContentLoaded", async () => {
  // Com `await`: sem ativação, a tela de compra precisa estar de pé antes de o console aparecer por um quadro.
  await montarPortao();

  ligarBarraDaJanela();
  carregarMapaDeDesempenho();

  // A do painel gira (o giro carrega o uso de CPU); as grandes não: informam pela erosão, redesenhadas uma vez por
  // medição, sem laço de sessenta quadros na máquina fraca.
  esfera = new Esfera(element<HTMLCanvasElement>("veredito-esfera"));

  // `dissolve`: para que lado a imagem some no preto, na direção do texto.
  pilaresDoPortao = new Pilares(element<HTMLCanvasElement>("portao-pilares"));

  pilaresDaChegada = new Pilares(element<HTMLCanvasElement>("chegada-pilares"), {
    dissolve: "direita",
  });

  // Só em desenvolvimento: em produção o app roda de `tauri://`.
  if (location.hostname.startsWith("localhost")) {
    (window as unknown as { esfera?: Esfera }).esfera = esfera;
  }

  wireControls();
  requestAnimationFrame(() => setTimeout(() => void invoke("abertura_pronta").catch(() => {}), 0));
  ligarSubabas();
  conferirInvariantesDaTela();

  // O VEREDITO PRIMEIRO, sem `await`: na fila seria o último a aparecer, atrás de dez chamadas.
  void carregarVeredito();

  // Sem `await`: o GitHub pode levar segundos. O `catch` cobre falha do IPC ou pânico, que virariam "unhandled
  // rejection" calado; a faixa simplesmente não aparece.
  void verificarAtualizacao().catch(() => {});

  await ajustarMovimento();
  await listenToBatchProgress();

  // Mudança silenciosa no sistema é o que o produto critica nos outros.
  await listen<string>("gamemode:changed", (evento) => {
    setStatus("gamemode-status", evento.payload, "ok");
    void loadGameMode();
    void loadOptimizations();
  });
  await listen("prova:automatica", () => {
    void carregarMedicoesAutomaticas();
    // Conferir a cada medição faz o aviso aparecer DURANTE a sessão em que o jogo piorou.
    void conferirOProprioTrabalho();
  });

  element("regressao-ver").addEventListener("click", () => {
    showTab("otimizacoes");
    element("regressao-faixa").hidden = true;
  });
  // Fechar esconde só até a próxima medição: "não mostrar mais" ajudaria o cliente a esquecer que perdeu FPS.
  element("regressao-fechar").addEventListener("click", () => {
    element("regressao-faixa").hidden = true;
  });
  await loadPreferences();

  await Promise.all([
    // Lê um arquivo pequeno e não chama o Windows: cabe no orçamento de abertura.
    conferirOProprioTrabalho(),
    // Esta CHAMA o Windows (seis leituras, uma o `nvidia-smi`), mas é a primeira pergunta do cliente de jogo; roda em
    // paralelo sem segurar a pintura.
    carregarPorQueOFpsEstaBaixo(),
    carregarOQueNaoFazemos(),
    loadIdentity(),
    checkAccess(),
    loadBaselineState(),
    loadOptimizations(),
    loadStartup(),
    loadRestoreStatus(),
    loadScheduledTasks(),
    loadThirdPartyServices(),
    loadProfiles(),
    loadGameMode(),
  ]);

  await startMonitoring();
});

/**
 * Qualquer das duas fontes desliga: a preferência do sistema ou o hardware desta máquina. Animação é a primeira
 * coisa cortada: engasgar no PC que ele deveria consertar desmente o produto.
 */
async function ajustarMovimento() {
  const sistemaPedeCalma = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let maquinaFraca = false;
  let maquinaApertada = false;

  try {
    const perfil = await invoke<{ total_ram_gb: number; logical_cores: number }>(
      "get_hardware_profile"
    );
    // 4 GB e 2 núcleos: o "PC fraco" do público.
    maquinaFraca = perfil.total_ram_gb <= 4.5 || perfil.logical_cores <= 2;

    maquinaApertada = !maquinaFraca && (perfil.total_ram_gb <= 8.5 || perfil.logical_cores <= 4);
  } catch {
    // Sem perfil, anima.
  }

  const parado = sistemaPedeCalma || maquinaFraca;
  document.body.classList.toggle("sem-animacao", parado);

  // Só a esfera tem laço para ligar; os pilares são desenhados por medição.
  esfera?.ligar();

  // O fundo divide a duração da animação por este número: 0,35 transforma 90 s em 257 s.
  document.documentElement.style.setProperty(
    "--anim",
    parado ? "0" : maquinaApertada ? "0.35" : "1"
  );
}

function element<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

/** O mesmo nome lido pelo script de abertura. */
const CHAVE_DO_TEMA = "otimiza-tema";

/**
 * Quem aplica o tema na abertura é o script síncrono do `<head>`, antes da primeira pintura; aqui ficam o clique
 * e o acompanhamento do sistema.
 */
function ligarTema() {
  const botao = element<HTMLButtonElement>("tema-botao");
  const midia = window.matchMedia?.("(prefers-color-scheme: dark)");

  const aplicar = (tema: "claro" | "escuro") => {
    document.documentElement.dataset.tema = tema;
  };

  botao.addEventListener("click", () => {
    const novo = document.documentElement.dataset.tema === "escuro" ? "claro" : "escuro";
    aplicar(novo);

    // Guardar a escolha faz o app PARAR de seguir o Windows.
    try {
      localStorage.setItem(CHAVE_DO_TEMA, novo);
    } catch {
      // Sem armazenamento, vale só nesta sessão.
    }
  });

  midia?.addEventListener("change", (e) => {
    let escolhido: string | null = null;
    try {
      escolhido = localStorage.getItem(CHAVE_DO_TEMA);
    } catch {
      escolhido = null;
    }

    if (escolhido === null) aplicar(e.matches ? "escuro" : "claro");
  });
}

/**
 * Genérico: `.subabas` com botões `data-sub` e blocos `.subpainel` com o mesmo `data-sub`. O escopo é o painel:
 * dois painéis na mesma aba não apagam as guias um do outro.
 */
function ligarSubabas() {
  document.querySelectorAll<HTMLElement>(".subabas").forEach((barra) => {
    const painel = barra.closest(".panel");
    if (!painel) return;

    barra.addEventListener("click", (evento) => {
      const botao = (evento.target as HTMLElement).closest<HTMLButtonElement>(
        "button[data-sub]"
      );
      if (!botao) return;

      const escolhida = botao.dataset.sub;

      barra.querySelectorAll<HTMLButtonElement>("button[data-sub]").forEach((item) => {
        item.setAttribute("aria-selected", String(item.dataset.sub === escolhida));
      });

      painel.querySelectorAll<HTMLElement>(".subpainel[data-sub]").forEach((bloco) => {
        bloco.hidden = bloco.dataset.sub !== escolhida;
      });
    });
  });
}

/**
 * O Reparo só carrega ao abrir: `health::analyze()` (~5 s) e um CBS.log de dezenas de MB eram cobrados de todo
 * mundo. Em `showTab` porque a aba abre também por teclado e paleta. Uma vez só: registrar os ouvintes de novo
 * duplicaria as linhas de andamento.
 */
let reparoCarregado = false;
let planoVistoriado = false;
let discosCarregados = false;
let biosCarregada = false;

/**
 * Separada de `showTab` porque roda TAMBÉM na abertura (o programa abria com dois "Início"); `showTab` na
 * abertura dispararia as leituras caras.
 */
function sincronizarCabecalho(item: HTMLElement, name: string) {
  const rotulo = item.querySelector(".nav-rotulo")?.textContent?.trim() ?? "";
  text("secao-nome", rotulo);

  // O grupo da lateral vira o rótulo: sem segunda tabela de nomes.
  let grupo = item.previousElementSibling;
  while (grupo && !grupo.classList.contains("lateral-grupo")) {
    grupo = grupo.previousElementSibling;
  }
  text("secao-grupo", grupo?.textContent?.trim() ?? "");

  // Dois títulos um embaixo do outro é o que faz a tela parecer montada por acréscimo.
  const painel = document.getElementById(`tab-${name}`);
  element("secao-cabecalho").hidden = !!painel?.querySelector(".abertura");
}

function showTab(name: string) {
  if (name === "energia") {
    void carregarMotorDeEnergia({ pedirAdmin: askForAdmin });
  }

  if (name === "framegen") {
    void carregarLaboratorioDeGeracao({ pedirAdmin: askForAdmin });
  }

  if (name === "painel") {
    ligarProntidao();
    void carregarUltimasPartidas();
  }

  if (name === "reparo" && !reparoCarregado) {
    reparoCarregado = true;
    void carregarReparo();
  }

  // Uma vez só: passa pelo PowerShell e não pode entrar na abertura.
  if (name === "bios") {
    void carregarAbaBios();
  }

  if (name === "bios" && !biosCarregada) {
    biosCarregada = true;
    void carregarPassoAPassoDaBios();
  }

  if (name === "jogos" && !discosCarregados) {
    discosCarregados = true;
    void carregarOndeOsJogosMoram();
    void carregarProtocolo();
  }

  // Ao abrir a aba, nunca na abertura (PowerShell e `powercfg`), mas automática: um plano desfeito por outro
  // programa não seria descoberto. Refeita depois de aplicar, reparar ou desfazer.
  if (name === "otimizacoes" && !planoVistoriado) {
    planoVistoriado = true;
    void vistoriarPlano();
  }

  document.querySelectorAll<HTMLElement>(".tab-panel").forEach((panel) => {
    panel.hidden = panel.id !== `tab-${name}`;
  });

  document.querySelectorAll<HTMLButtonElement>(".nav[data-tab]").forEach((item) => {
    const escolhida = item.dataset.tab === name;
    item.setAttribute("aria-selected", String(escolhida));

    // Do próprio item da navegação: escrever duas vezes sairia de sincronia.
    if (!escolhida) return;

    sincronizarCabecalho(item, name);
  });
}

/**
 * Cada diagnóstico declara o que encontrou, e o selo é a soma: antes três lugares SOBRESCREVIAM (ganhava quem
 * carregasse por último) e um quarto relia o próprio texto do selo.
 */
const problemasPorFonte = new Map<string, { n: number; critico: boolean }>();

function registrarProblemas(fonte: string, n: number, critico: boolean) {
  problemasPorFonte.set(fonte, { n, critico });

  let total = 0;
  let algumCritico = false;

  for (const { n: quantos, critico: grave } of problemasPorFonte.values()) {
    total += quantos;
    if (grave && quantos > 0) algumCritico = true;
  }

  setBadge("badge-diagnostico", total, algumCritico ? "bad" : "warn");
}

function setBadge(id: string, count: number, tone?: "warn" | "bad") {
  const badge = element(id);

  // `count <= 0` é falso para `NaN`: sem número, não desenha a bolinha (acesa é uma afirmação).
  const valido = Number.isFinite(count) && count > 0;

  badge.hidden = !valido;
  badge.textContent = valido ? String(Math.round(count)) : "";

  if (tone) {
    badge.dataset.tone = tone;
  } else {
    delete badge.dataset.tone;
  }
}

/** Elemento ausente é ignorado: painéis que saíram (2.9) não derrubam o laço do monitor. */
function text(id: string, value: string) {
  const alvo = document.getElementById(id);
  if (alvo) alvo.textContent = value;
}

function escapeHtml(value: string): string {
  const node = document.createElement("div");
  node.textContent = value;
  return node.innerHTML;
}

async function loadIdentity() {
  try {
    const platform = await invoke<{ os_type: string; version: string; arch: string }>(
      "get_platform_info"
    );
    text("ident-os", `${platform.version} · ${platform.arch}`);
    text("ficha-so", `${platform.version} · ${platform.arch}`);
    fichaDaMaquina.so = platform.version;
  } catch (error) {
    text("ident-os", "indisponível");
    text("ficha-so", "indisponível");
    console.error(error);
  }

  try {
    // A identidade da máquina é o cabeçalho: não pode depender de um clique.
    const hardware = await invoke<{
      storage: string;
      total_ram_gb: number;
      logical_cores: number;
      cpu_name: string;
      gpu_name: string;
    }>("get_hardware_profile");

    text("ident-storage", hardware.storage);
    text("ident-ram", `${hardware.total_ram_gb.toFixed(1)} GB`);
    text("ident-cpu", hardware.cpu_name);
    text("ident-gpu", hardware.gpu_name);

    // A mesma leitura alimenta a ficha da abertura: uma segunda consulta poderia discordar.
    text("ficha-cpu", hardware.cpu_name);
    text("ficha-gpu", hardware.gpu_name);

    fichaDaMaquina.cpu = hardware.cpu_name;
    fichaDaMaquina.gpu = hardware.gpu_name;
    fichaDaMaquina.ram = `${hardware.total_ram_gb.toFixed(0)} GB`;
    text("ficha-ram", `${hardware.total_ram_gb.toFixed(0)} GB · ${hardware.logical_cores} núcleos lógicos`);
  } catch (error) {
    text("ident-storage", "indisponível");
    text("ident-cpu", "indisponível");
    text("ident-gpu", "indisponível");
    text("ficha-cpu", "indisponível");
    text("ficha-gpu", "indisponível");
    text("ficha-ram", "indisponível");
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

/** A chave muda com a versão publicada: fixa, fechar a faixa calaria para sempre as versões futuras. */
const CHAVE_VERSAO_DISPENSADA = "atualizacao-dispensada";

/**
 * SEM `await`: o GitHub pode levar dez segundos (`TIMEOUT_SEGUNDOS` em `atualizacao.rs`). Falha de rede chega
 * como `NaoSei`, não exceção.
 */
async function verificarAtualizacao() {
  const aviso = await invoke<AvisoDeVersao>("versao_mais_nova");

  // Pela VARIANTE ("HaVersaoNova", uma palavra do backend), nunca pelo texto de `versao_publicada`.
  if (aviso.comparacao !== "HaVersaoNova" || !aviso.versao_publicada) return;

  if (localStorage.getItem(CHAVE_VERSAO_DISPENSADA) === aviso.versao_publicada) return;

  const faixa = element("atualizacao-faixa");
  text(
    "atualizacao-texto",
    `Uma nova versão do Otimiza está disponível (${aviso.versao_publicada}).`
  );

  const botaoBaixar = element<HTMLButtonElement>("atualizacao-baixar");
  botaoBaixar.hidden = !aviso.pagina;
  if (aviso.pagina) {
    const pagina = aviso.pagina;
    // No navegador padrão: o programa não instala nada sozinho.
    botaoBaixar.onclick = () => {
      void openUrl(pagina);
    };
  }

  element("atualizacao-fechar").onclick = () => {
    localStorage.setItem(CHAVE_VERSAO_DISPENSADA, aviso.versao_publicada!);
    faixa.hidden = true;
  };

  faixa.hidden = false;
}

async function startMonitoring() {
  try {
    await invoke("start_monitoring");
  } catch (error) {
    console.error("Não foi possível iniciar o monitoramento:", error);
  }

  await tick();
  restartMetricsLoop();
}

/** Trocar o intervalo só troca o relógio. */
function restartMetricsLoop() {
  if (metricsTimer !== null) {
    window.clearInterval(metricsTimer);
  }

  metricsTimer = window.setInterval(tick, preferences.metrics_interval_seconds * 1000);
}

/** A coleta espera 200 ms pela CPU: num intervalo curto ou máquina ocupada as chamadas se empilhariam. */
let tickEmAndamento = false;

async function tick() {
  if (tickEmAndamento) return;
  tickEmAndamento = true;

  try {
    try {
      const metrics = await invoke<PerformanceMetrics>("get_performance_metrics");
      renderMetrics(metrics);
    } catch (error) {
      console.error("Erro ao coletar métricas:", error);
      // Os números são de uma leitura que não aconteceu.
      limparMetricas(String(error));
    }

    try {
      const processes = await invoke<ProcessImpact[]>("top_processes");
      renderProcesses(processes);
    } catch (error) {
      console.error("Erro ao ler processos:", error);
    }
  } finally {
    tickEmAndamento = false;
  }
}

/** Travessão, não zero: CPU a 0% porque a coleta caiu é a mentira que o produto tirou. */
function limparMetricas(motivo: string) {
  for (const id of [
    "vital-cpu",
    "vital-ram",
    "vital-disk",
    "cpu-value",
    "ram-value",
    "disk-value",
    "flow-read",
    "flow-write",
    "flow-net",
    "clock-efetivo",
    "gpu-value",
    "vram-value",
  ]) {
    text(id, "—");
  }

  for (const id of ["vital-cpu-bar", "vital-ram-bar", "vital-disk-bar", "ram-bar", "disk-bar", "clock-bar", "gpu-bar", "vram-bar"]) {
    setBar(id, null);
  }

  for (const id of ["clock-note", "gpu-note", "vram-note"]) {
    const nota = element(id);
    nota.textContent = "sem leitura";
    nota.className = "readout-note";
  }

  // Sem coleta, o veredito anterior afirmaria sobre agora com evidência de antes.
  element("gargalo-cobertura").textContent = "—";
  element("gargalo-conclusao").textContent =
    "A leitura falhou, então não há evidência para classificar nada.";
  element("gargalo-achados").innerHTML = "";
  element("gargalo-faltas").innerHTML = "";

  renderVram({
    estado: "NaoAvaliado",
    dedicada_pct: null,
    folga_gb: null,
    derramado_gb: null,
    piso_gb: null,
    falta: [],
    explicacao: "A leitura falhou, então não há memória de vídeo a avaliar.",
    conselho: null,
  });

  renderLatencia({
    parcelas: [],
    piso_ms: null,
    etapas_com_valor: 0,
    etapas_totais: 5,
    observacoes: [],
  });

  const tag = element("evidencia-tag");
  tag.textContent = "leitura indisponível";
  tag.dataset.estado = "falha";
  element("evidencia-tabela").innerHTML = `<p class="empty">${escapeHtml(motivo)}</p>`;

  text("status-right", `leitura falhou às ${new Date().toLocaleTimeString("pt-BR")}`);
}

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

/** Ausência vira travessão: onde não há número, nenhum número. */
function medida(valor: number | null, formatar: (n: number) => string): string {
  return valor === null ? "—" : formatar(valor);
}

function renderMetrics(metrics: PerformanceMetrics) {
  if (metrics.telemetry.schema_version !== TELEMETRIA_SUPORTADA) {
    limparMetricas(
      `esta tela lê a telemetria versão ${TELEMETRIA_SUPORTADA} e recebeu a versão ${metrics.telemetry.schema_version}`
    );
    return;
  }

  renderAbertura(metrics);
  renderEvidencia(metrics.telemetry);
  renderVram(metrics.vram);
  renderLatencia(metrics.latencia);
  renderGargalo(metrics.gargalo);

  const cpu = metrics.cpu.overall === null ? null : Math.min(100, Math.max(0, metrics.cpu.overall));

  // O perímetro (2πr, r=86) é 540, igual ao dasharray do CSS.
  const gauge = document.getElementById("gauge-cpu") as (SVGCircleElement & HTMLElement) | null;

  // Sobe devagar uma vez só: reiniciar a cada 2 s pareceria quebrado.
  const aro = gauge?.closest(".gauge") as HTMLElement | null;
  if (aro && !aro.dataset.iniciado) {
    aro.dataset.iniciado = "sim";
    aro.dataset.entrada = "true";
    window.setTimeout(() => delete aro.dataset.entrada, 1000);
  }
  // Sem leitura esvazia e fica cinza: parado no número de trinta segundos atrás pareceria vivo.
  if (gauge) gauge.style.strokeDashoffset = String(cpu === null ? 540 : 540 - (540 * cpu) / 100);
  if (gauge) gauge.style.stroke = cpu === null ? "var(--text-muted)" : loadColor(cpu);

  const porcento = (n: number) => `${n.toFixed(0)}%`;

  text("vital-cpu", medida(cpu, porcento));
  setBar("vital-cpu-bar", cpu);
  text("vital-ram", medida(metrics.ram.usage_percent, porcento));
  setBar("vital-ram-bar", metrics.ram.usage_percent);
  text("vital-disk", medida(metrics.disk.usage_percent, porcento));
  setBar("vital-disk-bar", metrics.disk.usage_percent);

  const nivelAgora =
    (element("veredito").dataset.nivel as "ok" | "importante" | "critico") ?? "ok";

  // Só com medição na mão: zero na leitura falha transformaria a promessa em enfeite.
  if (cpu !== null && metrics.ram.usage_percent !== null) {
    alimentarEsferas({
      nucleos: metrics.cpu.per_core.length,
      cpu,
      memoria: metrics.ram.usage_percent,
      nivel: nivelAgora,
    });
  }

  if (cpu !== null && metrics.ram.usage_percent !== null && metrics.disk.usage_percent !== null) {
    alimentarPilares({
      cpu,
      memoria: metrics.ram.usage_percent,
      disco: metrics.disk.usage_percent,
      nivel: nivelAgora,
    });
  }

  text("cpu-value", medida(cpu, (n) => n.toFixed(0)));

  // ESTIMATED (o clock informado do primeiro núcleo); sem ele, só a contagem de núcleos, não "0 MHz".
  const nucleos = `${metrics.cpu.per_core.length} núcleos`;
  const nominal = valorDe(metrics.telemetry, "cpu.clock.reported") ?? metrics.cpu.frequency;
  text(
    "cpu-freq",
    nominal === null ? `clock não informado · ${nucleos}` : `${nominal.toFixed(0)} MHz · ${nucleos}`
  );

  renderClock(metrics.telemetry);
  renderPlaca(metrics.telemetry);
  text("core-count", `${metrics.cpu.per_core.length} lógicos`);
  text("tick-clock", new Date().toLocaleTimeString("pt-BR"));

  renderCores(metrics.cpu.per_core);
  if (cpu !== null) pushHistory(cpu);

  const ram = metrics.ram;
  text("ram-value", medida(ram.usage_percent, porcento));
  text("ram-note", `${ram.used_gb.toFixed(1)} de ${ram.total_gb.toFixed(1)} GB em uso`);
  setBar("ram-bar", ram.usage_percent);

  text("disk-value", medida(metrics.disk.usage_percent, porcento));
  setBar("disk-bar", metrics.disk.usage_percent);

  text("net-value", `${metrics.network.total_received_gb.toFixed(1)} GB`);

  renderFlow(metrics);

  text("status-right", `atualizado às ${new Date().toLocaleTimeString("pt-BR")}`);
}

/**
 * Abaixo de 1 MB/s, em KB/s. "parado" só para taxa MEDIDA (a rede); o disco não distingue parado de falha e chega
 * `null`.
 */
function formatRate(mbPerSecond: number | null): string {
  if (mbPerSecond === null) return "—";
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

  // Seria constrangedor cobrar por um ajuste que um reinício resolveria.
  const noteBar = element("flow-uptime-note");
  if (dias >= 7) {
    noteBar.textContent = "muitos dias sem reiniciar — reinicie antes de otimizar";
    noteBar.className = "readout-note warn";
  } else {
    noteBar.textContent = "desde o último boot";
    noteBar.className = "readout-note";
  }
}

// O painel "de onde veio este número": o não medido aparece pelo nome, com o motivo.

const ROTULO_QUALIDADE: Record<Quality, string> = {
  MEASURED: "medido",
  ESTIMATED: "estimado",
  UNKNOWN: "não medido",
};

const ORDEM_QUALIDADE: Record<Quality, number> = {
  MEASURED: 0,
  ESTIMATED: 1,
  UNKNOWN: 2,
};

const UNIDADE: Record<string, string> = {
  percent: "%",
  megahertz: "MHz",
  hertz: "Hz",
  celsius: "°C",
  gigabytes: "GB",
  megabytes_per_second: "MB/s",
  milliseconds: "ms",
  fps: "FPS",
  watts: "W",
  hours: "h",
  count: "",
  boolean: "",
};

const ID_DE_NUCLEO = /^cpu\.core\.\d+\.usage$/;

function valorLegivel(metric: Metric): string {
  if (metric.value === null) return "—";
  if (metric.unit === "boolean") return metric.value >= 0.5 ? "sim" : "não";

  const casas = metric.unit === "count" ? 0 : 1;
  const unidade = UNIDADE[metric.unit] ?? metric.unit;

  return `${metric.value.toFixed(casas)}${unidade ? ` ${unidade}` : ""}`;
}

function valorDe(telemetry: Telemetry, id: string): number | null {
  return telemetry.metrics[id]?.value ?? null;
}

/** As legendas dos cartões usam o nome do processador: sem ele a legenda é uma contagem solta. */
const fichaDaMaquina: { cpu?: string; gpu?: string; ram?: string; so?: string } = {};

const PONTOS_DA_LINHA = 40;

/** Só na tela: o histórico que responde "quando piorou?" é outro, com repetição e margem (`historico.rs`). */
const linhasDoVivo: Record<string, number[]> = {};

/** Leitura ausente interrompe a série: `null` como zero desenharia uma queda que a máquina nunca teve. */
function desenharLinha(chave: string, valor: number | null, teto: number) {
  const serie = (linhasDoVivo[chave] ??= []);

  if (valor !== null && Number.isFinite(valor)) {
    serie.push(valor);
    if (serie.length > PONTOS_DA_LINHA) serie.shift();
  }

  const traco = element<SVGPathElement & HTMLElement>(`monitor-${chave}-linha`).querySelector(
    ".vivo-traco"
  ) as SVGPathElement | null;
  if (!traco) return;

  if (serie.length < 2) {
    traco.setAttribute("d", "");
    return;
  }

  // Escala pelo maior valor da janela, com piso: contra 100 vira reta, contra o próprio máximo vira montanha.
  const maior = Math.max(teto * 0.25, ...serie);
  const passo = 100 / (serie.length - 1);

  const d = serie
    .map((v, i) => {
      const x = (i * passo).toFixed(2);
      const y = (28 - Math.min(1, v / maior) * 26).toFixed(2);
      return `${i === 0 ? "M" : "L"}${x} ${y}`;
    })
    .join(" ");

  traco.setAttribute("d", d);
}

function pintarVivo(
  chave: string,
  valor: number | null,
  nota: string,
  teto: number,
  casas = 0,
  selo?: { texto: string; tom: string }
) {
  const cartao = document.querySelector<HTMLElement>(`.vivo[data-metrica="${chave}"]`);
  if (cartao) cartao.dataset.semLeitura = valor === null ? "sim" : "nao";

  text(`monitor-${chave}-valor`, valor === null ? "—" : valor.toFixed(casas));
  text(`monitor-${chave}-nota`, nota);
  desenharLinha(chave, valor, teto);

  const marca = element(`monitor-${chave}-marca`);
  if (selo) {
    marca.textContent = selo.texto;
    marca.dataset.tom = selo.tom;
    marca.hidden = false;
  } else {
    marca.hidden = true;
  }
}

function renderAbertura(m: PerformanceMetrics) {
  const t = m.telemetry;

  pintarVivo(
    "cpu",
    valorDe(t, "cpu.usage.overall"),
    [
      fichaDaMaquina.cpu,
      medida(valorDe(t, "cpu.cores.logical"), (n) => `${n.toFixed(0)} núcleos`),
    ]
      .filter((parte) => parte && parte !== "—")
      .join(" · "),
    100
  );

  pintarVivo(
    "gpu",
    valorDe(t, "gpu.usage"),
    [
      fichaDaMaquina.gpu,
      medida(valorDe(t, "vram.total"), (n) => `${n.toFixed(0)} GB`),
    ]
      .filter((parte) => parte && parte !== "—")
      .join(" · "),
    100
  );

  const usada = valorDe(t, "ram.used");
  const total = valorDe(t, "ram.total");
  pintarVivo(
    "ram",
    usada,
    total === null ? "total não lido" : `de ${total.toFixed(0)} GB`,
    total ?? 32,
    1
  );

  // O selo olha o JITTER, não a latência: ping baixo que pula é o que estraga a partida.
  const latencia = valorDe(t, "network.latency");
  const jitter = valorDe(t, "network.jitter");
  pintarVivo(
    "rede",
    latencia,
    medida(jitter, (n) => `variação de ${n.toFixed(0)} ms`),
    100,
    0,
    jitter === null
      ? undefined
      : jitter <= 30
        ? { texto: "Estável", tom: "bom" }
        : { texto: "Instável", tom: "alerta" }
  );

  text(
    "abertura-sub",
    [fichaDaMaquina.so, "monitorando em tempo real"]
      .filter(Boolean)
      .join(" · ")
  );
}

/**
 * Separa "está a 4 GHz" de "entrega 4 GHz". Limite de firmware aparece aqui: nenhum plano o resolve, e o cliente
 * precisa saber antes de pagar.
 */
function renderClock(telemetry: Telemetry) {
  const efetivo = valorDe(telemetry, "cpu.clock.effective");
  const nominal = valorDe(telemetry, "cpu.clock.reported");

  text("clock-efetivo", medida(efetivo, (n) => `${n.toFixed(0)} MHz`));

  // Sem um dos lados, desconhecida, não zero.
  setBar(
    "clock-bar",
    efetivo !== null && nominal !== null && nominal > 0 ? (efetivo / nominal) * 100 : null
  );

  const termico = valorDe(telemetry, "cpu.throttling.thermal");
  const eletrico = valorDe(telemetry, "cpu.throttling.power");
  const limite = valorDe(telemetry, "cpu.performance_limit");

  const nota = element("clock-note");

  if (termico === 1 || eletrico === 1) {
    const causa = termico === 1 ? "temperatura" : "energia";
    nota.textContent = `firmware limitando por ${causa} — nenhum plano resolve isso`;
    nota.className = "readout-note warn";
    return;
  }

  nota.className = "readout-note";

  if (efetivo === null || nominal === null) {
    nota.textContent = "contadores do Windows não responderam";
    return;
  }

  const aproveitamento = nominal > 0 ? (efetivo / nominal) * 100 : 0;
  const folga = limite === null ? "" : limite >= 99.5 ? " · sem limite de firmware" : ` · firmware em ${limite.toFixed(0)}%`;

  nota.textContent = `${aproveitamento.toFixed(0)}% do nominal de ${nominal.toFixed(0)} MHz${folga}`;
}

/** "agora" só para esta coleta: 95% de GPU agora e antes de o jogo fechar levam a vereditos opostos. */
function quandoFoiLido(metric: Metric | undefined): string {
  if (!metric || metric.age_ms === null) return "agora";
  if (metric.age_ms < 1000) return "agora";
  return `há ${(metric.age_ms / 1000).toFixed(0)} s`;
}

/** Sem leitura, o motivo do contrato vai para a nota: barra em zero não é resposta. */
function renderPlaca(telemetry: Telemetry) {
  const gpu = telemetry.metrics["gpu.usage"];
  const uso = gpu?.value ?? null;

  text("gpu-value", medida(uso, (n) => `${n.toFixed(0)}%`));
  setBar("gpu-bar", uso);
  element("gpu-note").textContent =
    uso === null ? (gpu?.reason ?? "não medido") : `motores 3D · ${quandoFoiLido(gpu)}`;

  const usada = telemetry.metrics["vram.used"];
  const pct = telemetry.metrics["vram.usage"]?.value ?? null;
  const total = valorDe(telemetry, "vram.total");

  text("vram-value", medida(pct, (n) => `${n.toFixed(0)}%`));
  setBar("vram-bar", pct);

  const nota = element("vram-note");
  if (usada?.value != null && total !== null) {
    nota.textContent = `${usada.value.toFixed(1)} de ${total.toFixed(1)} GB · ${quandoFoiLido(usada)}`;
  } else if (total !== null) {
    nota.textContent = `${total.toFixed(1)} GB na placa · uso não medido`;
  } else {
    nota.textContent = usada?.reason ?? "não medido";
  }
}

/** O mesmo bloco em todo painel, e SOME sem lacuna: "nada faltou" em quatro painéis viraria ruído. */
function blocoDeFaltas(itens: string[], rotulo: string): string {
  if (itens.length === 0) return "";

  return `<p class="hint faltas"><strong>${escapeHtml(rotulo)}</strong> ${escapeHtml(
    itens.join(", ")
  )}.</p>`;
}

type ClasseDeNucleo = "Desempenho" | "Eficiencia" | "Uniforme";

interface NucleoLogico {
  indice: number;
  fisico: number;
  classe: ClasseDeNucleo;
}

interface NucleosNaTela {
  topologia: { nucleos: NucleoLogico[]; hibrido: boolean };
  conselho: { cabe: boolean; explicacao: string };
  fisicos: number;
  mascara_de_desempenho: string | null;
  jogo_nome: string | null;
  jogo_pid: number | null;
  jogo_mascara: string | null;
  jogo_exe?: string | null;
}

const NOME_DA_CLASSE_DE_NUCLEO: Record<ClasseDeNucleo, string> = {
  Desempenho: "desempenho",
  Eficiencia: "eficiência",
  Uniforme: "iguais",
};

let nucleosCarregados: NucleosNaTela | null = null;

async function carregarNucleos() {
  try {
    const r = await invoke<NucleosNaTela>("nucleos_da_maquina");
    nucleosCarregados = r;
    desenharNucleos(r);
  } catch (error) {
    text("nucleos-resumo", String(error));
    element("nucleos-matriz").innerHTML = "";
  }
}

function desenharNucleos(r: NucleosNaTela) {
  const logicos = r.topologia.nucleos;
  text(
    "nucleos-resumo",
    `${logicos.length} núcleos lógicos em ${r.fisicos} físicos · ${
      r.topologia.hibrido ? "processador híbrido" : "todos iguais"
    }`
  );

  text("nucleos-tag", r.conselho.cabe ? "há o que fazer" : "nada a fazer aqui");
  text("nucleos-conselho", r.conselho.explicacao);

  // BigInt: um número comum perde os bits acima de 53, e o bit perdido é um núcleo.
  const doJogo = r.jogo_mascara === null ? null : BigInt(r.jogo_mascara);

  element("nucleos-matriz").innerHTML = logicos
    .map((n) => {
      const noJogo = doJogo === null ? false : (doJogo >> BigInt(n.indice)) & 1n ? true : false;

      return `
        <span class="nucleo" data-classe="${n.classe}" data-no-jogo="${noJogo}"
              title="Núcleo lógico ${n.indice}, físico ${n.fisico}, ${
                NOME_DA_CLASSE_DE_NUCLEO[n.classe]
              }">
          <span class="nucleo-indice">${String(n.indice).padStart(2, "0")}</span>
        </span>`;
    })
    .join("");

  // Só as classes que EXISTEM nesta máquina.
  const classes = [...new Set(logicos.map((n) => n.classe))];
  element("nucleos-legenda").innerHTML =
    classes
      .map(
        (c) =>
          `<span class="nucleo-chave" data-classe="${c}">${escapeHtml(
            NOME_DA_CLASSE_DE_NUCLEO[c]
          )}</span>`
      )
      .join("") +
    (doJogo === null
      ? ""
      : `<span class="nucleo-chave" data-no-jogo="true">onde o jogo pode rodar</span>`);

  desenharJogoNosNucleos(r);
}

interface ResultadoCpuSet {
  executavel: string;
  quando: number;
  escolha: "SoDesempenho" | "TodosOsNucleos";
  fps_todos: number;
  fps_desempenho: number;
  low_todos: number;
  low_desempenho: number;
}

async function desenharCpuSet() {
  const alvo = document.getElementById("cpuset-resultados");
  if (!alvo) return;
  try {
    const lista = await invoke<ResultadoCpuSet[]>("cpuset_resultados");
    if (!lista.length) {
      alvo.innerHTML = "";
      return;
    }
    const n = (v: number) => Math.round(v).toString();
    alvo.innerHTML = `
      <table class="fg-tabela">
        <thead><tr><th>Jogo</th><th>Todos os núcleos</th><th>Só desempenho</th><th>Ficou</th><th></th></tr></thead>
        <tbody>${lista
          .map(
            (r) => `<tr>
              <td class="fg-mono">${escapeHtml(r.executavel)}</td>
              <td>${n(r.fps_todos)} FPS · 1% ${n(r.low_todos)}</td>
              <td>${n(r.fps_desempenho)} FPS · 1% ${n(r.low_desempenho)}</td>
              <td>${r.escolha === "SoDesempenho" ? "só desempenho (reaplicado ao abrir)" : "todos os núcleos"}</td>
              <td><button class="btn btn-ghost" data-cpuset-esquecer="${escapeHtml(r.executavel)}">Esquecer</button></td>
            </tr>`,
          )
          .join("")}</tbody>
      </table>`;
    alvo.querySelectorAll<HTMLButtonElement>("[data-cpuset-esquecer]").forEach((b) =>
      b.addEventListener("click", async () => {
        b.disabled = true;
        try {
          await invoke("cpuset_esquecer", { executavel: b.dataset.cpusetEsquecer });
          setStatus("nucleos-status", "Esquecido: o jogo volta a abrir em todos os núcleos.", "ok");
        } catch (e) {
          setStatus("nucleos-status", String(e), "error");
        }
        void desenharCpuSet();
      }),
    );
  } catch {
    alvo.innerHTML = "";
  }
}

async function testarCpuSet() {
  const pid = nucleosCarregados?.jogo_pid;
  const exe = nucleosCarregados?.jogo_exe;
  if (pid === null || pid === undefined || !exe) return;
  const botao = element<HTMLButtonElement>("cpuset-testar");
  botao.disabled = true;
  setStatus("nucleos-status", "Medindo… mantenha o jogo em partida por cerca de 1 minuto.", "progress");
  try {
    const r = await invoke<ResultadoCpuSet>("cpuset_testar", { pid, executavel: exe });
    setStatus(
      "nucleos-status",
      r.escolha === "SoDesempenho"
        ? "Rendeu nos núcleos de desempenho: o jogo fica neles, e isso é reaplicado quando ele abrir."
        : "Não rendeu de verdade só nos núcleos de desempenho: o jogo voltou a usar todos.",
      "ok",
    );
    await carregarNucleos();
  } catch (e) {
    setStatus("nucleos-status", String(e), "error");
    botao.disabled = false;
  }
  void desenharCpuSet();
}

function desenharJogoNosNucleos(r: NucleosNaTela) {
  const prender = element<HTMLButtonElement>("nucleos-prender");
  const testar = document.getElementById("cpuset-testar") as HTMLButtonElement | null;
  if (testar) testar.disabled = r.jogo_pid === null || !r.conselho.cabe || !r.jogo_exe;
  void desenharCpuSet();
  const soltar = element<HTMLButtonElement>("nucleos-soltar");

  if (r.jogo_pid === null) {
    text("nucleos-jogo-tag", "nenhum jogo aberto");
    text(
      "nucleos-jogo-estado",
      "Abra o jogo e clique em Reler. A afinidade vale para o processo aberto, então não há o que ajustar com o jogo fechado."
    );
    prender.disabled = true;
    soltar.disabled = true;
    return;
  }

  text("nucleos-jogo-tag", r.jogo_nome ?? "jogo detectado");

  const doJogo = r.jogo_mascara === null ? null : BigInt(r.jogo_mascara);
  const rapidos = r.mascara_de_desempenho === null ? null : BigInt(r.mascara_de_desempenho);
  const todos = BigInt(r.topologia.nucleos.length) === 64n
    ? null
    : (1n << BigInt(r.topologia.nucleos.length)) - 1n;

  const presoNosRapidos = doJogo !== null && rapidos !== null && doJogo === rapidos;
  const solto = doJogo !== null && todos !== null && doJogo === todos;

  text(
    "nucleos-jogo-estado",
    doJogo === null
      ? "Não consegui ler em que núcleos este jogo está. Jogos com anticheat costumam bloquear essa leitura."
      : presoNosRapidos
        ? "Este jogo já está preso nos núcleos de desempenho."
        : solto
          ? "Este jogo pode usar todos os núcleos — que é o estado normal."
          : `Este jogo está limitado a ${
              [...Array(r.topologia.nucleos.length).keys()].filter(
                (i) => (doJogo >> BigInt(i)) & 1n
              ).length
            } núcleos. Alguém ou algum programa mexeu nisso.`
  );

  // Só onde resolve: processador híbrido e jogo ainda não preso.
  prender.disabled = !r.conselho.cabe || presoNosRapidos || doJogo === null;
  soltar.disabled = doJogo === null || solto;
}

async function mexerNosNucleos(prender: boolean) {
  const pid = nucleosCarregados?.jogo_pid;
  if (pid === null || pid === undefined) return;

  const botao = element<HTMLButtonElement>(prender ? "nucleos-prender" : "nucleos-soltar");
  botao.disabled = true;
  setStatus("nucleos-status", prender ? "Prendendo…" : "Soltando…", "progress");

  try {
    const mensagem = await invoke<string>("prender_jogo_nos_nucleos", { pid, prender });
    setStatus("nucleos-status", mensagem, "ok");

    // Relê em vez de assumir.
    await carregarNucleos();
  } catch (error) {
    setStatus("nucleos-status", String(error), "error");
    botao.disabled = false;
  }
}

function ligarNucleos() {
  element("nucleos-prender").addEventListener("click", () => void mexerNosNucleos(true));
  element("nucleos-soltar").addEventListener("click", () => void mexerNosNucleos(false));
  element("nucleos-reler").addEventListener("click", () => void carregarNucleos());
  document.getElementById("cpuset-testar")?.addEventListener("click", () => void testarCpuSet());
}

interface AlvoDeLimpeza {
  id: string;
  nome: string;
  o_que_e: string;
  custo: string;
  padrao: boolean;
  bytes: number | null;
}

interface LimpezaNaTela {
  alvos: AlvoDeLimpeza[];
  marcados: string[];
}

interface ResultadoDaLimpeza {
  id: string;
  bytes_liberados: number;
  arquivos_apagados: number;
  arquivos_pulados: number;
  erro: string | null;
}

let alvosDeLimpeza: AlvoDeLimpeza[] = [];

function emTexto(bytes: number): string {
  const KB = 1024;
  const MB = KB * 1024;
  const GB = MB * 1024;

  if (bytes >= GB) return `${(bytes / GB).toFixed(1)} GB`;
  if (bytes >= MB) return `${Math.round(bytes / MB)} MB`;
  if (bytes >= KB) return `${Math.round(bytes / KB)} KB`;
  return `${bytes} B`;
}

async function medirLimpeza() {
  const botao = element<HTMLButtonElement>("limpeza-medir");
  botao.disabled = true;
  setStatus("limpeza-status", "Somando o que dá para liberar…", "progress");

  try {
    const r = await invoke<LimpezaNaTela>("medir_limpeza");
    alvosDeLimpeza = r.alvos;
    desenharLimpeza(r.marcados);
    setStatus("limpeza-status", "", "ok");
  } catch (error) {
    setStatus("limpeza-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

function desenharLimpeza(marcados: string[]) {
  element("limpeza-lista").innerHTML = alvosDeLimpeza
    .map((a) => {
      // "Não deu para ler" não é "0 B": a ausência faz o técnico tentar como administrador.
      const tamanho =
        a.bytes === null
          ? `<span class="limpeza-tamanho" data-tom="desconhecido">não deu para ler</span>`
          : `<span class="limpeza-tamanho">${emTexto(a.bytes)}</span>`;

      // Clique que não faz nada ensina que os outros também não fazem.
      const vazio = a.bytes === 0;
      const marcado = marcados.includes(a.id) && !vazio;

      return `
        <label class="limpeza-item" data-custa="${a.id === "lixeira"}">
          <input type="checkbox" data-limpar="${escapeHtml(a.id)}"
                 ${marcado ? "checked" : ""} ${vazio ? "disabled" : ""} />
          <span class="limpeza-corpo">
            <span class="limpeza-nome">${escapeHtml(a.nome)}${tamanho}</span>
            <span class="limpeza-oque">${escapeHtml(a.o_que_e)}</span>
            <span class="limpeza-custo">${escapeHtml(a.custo)}</span>
          </span>
        </label>`;
    })
    .join("");

  for (const caixa of element("limpeza-lista").querySelectorAll<HTMLInputElement>(
    "[data-limpar]"
  )) {
    caixa.addEventListener("change", atualizarTotalDaLimpeza);
  }

  atualizarTotalDaLimpeza();
}

function marcadosNaLimpeza(): string[] {
  return [
    ...element("limpeza-lista").querySelectorAll<HTMLInputElement>("[data-limpar]:checked"),
  ].map((c) => c.dataset.limpar!);
}

function atualizarTotalDaLimpeza() {
  const marcados = marcadosNaLimpeza();
  const escolhidos = alvosDeLimpeza.filter((a) => marcados.includes(a.id));

  // O não medido é contado à parte: total que finge cobrir tudo é pior que total parcial declarado.
  const soma = escolhidos.reduce((t, a) => t + (a.bytes ?? 0), 0);
  const semMedida = escolhidos.filter((a) => a.bytes === null).length;

  text(
    "limpeza-total",
    escolhidos.length === 0 ? "nada marcado" : `${emTexto(soma)} selecionados`
  );

  const aviso = element("limpeza-aviso");
  aviso.hidden = semMedida === 0;
  aviso.textContent =
    semMedida === 0
      ? ""
      : `${semMedida} item(ns) marcado(s) não puderam ser medidos, então não estão no total — o que for liberado será mais que o número acima.`;

  element<HTMLButtonElement>("limpeza-limpar").disabled = escolhidos.length === 0;
}

async function limparMarcados() {
  const marcados = marcadosNaLimpeza();
  if (marcados.length === 0) return;

  // A única operação sem Desfazer: a confirmação lista pelo NOME.
  const nomes = alvosDeLimpeza
    .filter((a) => marcados.includes(a.id))
    .map((a) => a.nome)
    .join(", ");

  const temLixeira = marcados.includes("lixeira");
  const aviso = temLixeira
    ? "\n\nA LIXEIRA ESTÁ MARCADA: os arquivos que você mandou para ela vão ser apagados de vez."
    : "";

  if (!window.confirm(`Apagar: ${nomes}.\n\nIsto não tem desfazer.${aviso}`)) return;

  const botao = element<HTMLButtonElement>("limpeza-limpar");
  botao.disabled = true;
  setStatus("limpeza-status", "Limpando…", "progress");

  try {
    const r = await invoke<ResultadoDaLimpeza[]>("limpar_alvos", { ids: marcados });

    const total = r.reduce((t, x) => t + x.bytes_liberados, 0);
    const pulados = r.reduce((t, x) => t + x.arquivos_pulados, 0);
    const falhas = r.filter((x) => x.erro);

    // Arquivo pulado (em uso) é CONTADO: quem esperava 2 GB e liberou 1,4 precisa saber por quê.
    setStatus(
      "limpeza-status",
      `Liberados ${emTexto(total)}.` +
        (pulados > 0 ? ` ${pulados} arquivo(s) em uso foram pulados.` : "") +
        (falhas.length > 0 ? ` ${falhas.map((f) => f.erro).join(" ")}` : ""),
      falhas.length > 0 ? "error" : "ok"
    );

    await medirLimpeza();
  } catch (error) {
    setStatus("limpeza-status", String(error), "error");
    botao.disabled = false;
  }
}

function ligarLimpeza() {
  element("limpeza-medir").addEventListener("click", () => void medirLimpeza());
  element("limpeza-limpar").addEventListener("click", () => void limparMarcados());
}

interface ProgramaNaLista {
  id: string;
  nome: string;
  descricao: string;
  categoria: string;
  instalado: boolean | null;
}

type WingetNaTela =
  | { estado: "Pronto"; versao: string }
  | { estado: "Ausente"; como_resolver: string };

interface ProgramasNaTela {
  programas: ProgramaNaLista[];
  winget: WingetNaTela;
  lacuna: string | null;
}

let programasCarregados: ProgramaNaLista[] = [];
let wingetPronto = false;
let categoriaEscolhida = "Todos";

/**
 * Esqueleto de carga: a caixa vazia durante segundos era igual à de "nada para mostrar". `aria-hidden` no
 * esqueleto; o recado é o `aria-busy` no container.
 */
function esqueletos(quantos: number, forma: "linha" | "bloco"): string {
  return Array.from(
    { length: quantos },
    () => `<div class="esqueleto esqueleto-${forma}" aria-hidden="true"></div>`
  ).join("");
}

function ocupado(id: string, sim: boolean) {
  if (sim) {
    element(id).setAttribute("aria-busy", "true");
  } else {
    element(id).removeAttribute("aria-busy");
  }
}

async function carregarProgramas() {
  // O catálogo tem tamanho conhecido; o que demora é o ESTADO de cada um.
  element("programas-lista").innerHTML = esqueletos(8, "linha");
  ocupado("programas-lista", true);

  try {
    const r = await invoke<ProgramasNaTela>("catalogo_de_programas");
    programasCarregados = r.programas;
    wingetPronto = r.winget.estado === "Pronto";

    const aviso = element("programas-winget");
    if (r.winget.estado === "Pronto") {
      aviso.textContent = `Instalador do Windows pronto (winget ${r.winget.versao}).`;
      aviso.dataset.tom = "ok";
    } else {
      aviso.textContent = r.winget.como_resolver;
      aviso.dataset.tom = "aviso";
    }

    // Sem a lacuna, "nenhum instalado" seria "não consegui olhar".
    const lacuna = element("programas-lacuna");
    lacuna.hidden = !r.lacuna;
    lacuna.textContent = r.lacuna
      ? `O estado de instalação não pôde ser lido: ${r.lacuna}`
      : "";

    const instalados = r.programas.filter((p) => p.instalado === true).length;
    text(
      "programas-tag",
      r.lacuna
        ? `${r.programas.length} programas`
        : `${instalados} de ${r.programas.length} instalados`
    );

    desenharFiltrosDeCategoria();
    desenharProgramas();
  } catch (error) {
    element("programas-lista").innerHTML = `<p class="hint">${escapeHtml(String(error))}</p>`;
  } finally {
    // `finally`: no erro também, senão o leitor de tela anuncia "carregando" para sempre.
    ocupado("programas-lista", false);
  }
}

function desenharFiltrosDeCategoria() {
  const categorias = ["Todos", ...new Set(programasCarregados.map((p) => p.categoria))];

  element("programas-filtros").innerHTML = categorias
    .map(
      (c) => `
      <button class="biblioteca-filtro" role="tab" data-categoria="${escapeHtml(c)}"
              aria-selected="${c === categoriaEscolhida}">${escapeHtml(c)}</button>`
    )
    .join("");

  for (const botao of element("programas-filtros").querySelectorAll<HTMLButtonElement>(
    "[data-categoria]"
  )) {
    botao.onclick = () => {
      categoriaEscolhida = botao.dataset.categoria ?? "Todos";
      desenharFiltrosDeCategoria();
      desenharProgramas();
    };
  }
}

function desenharProgramas() {
  const busca = element<HTMLInputElement>("programas-busca").value.trim().toLowerCase();

  const visiveis = programasCarregados.filter((p) => {
    if (categoriaEscolhida !== "Todos" && p.categoria !== categoriaEscolhida) return false;
    return busca === "" || p.nome.toLowerCase().includes(busca);
  });

  if (visiveis.length === 0) {
    element("programas-lista").innerHTML =
      `<p class="hint">Nenhum programa com esse nome no catálogo.</p>`;
    return;
  }

  element("programas-lista").innerHTML = visiveis
    .map((p) => {
      // Desconhecido NÃO vira "Instalar": instalar por cima é reparo para uns instaladores e primeira instalação para
      // outros.
      const estado =
        p.instalado === null
          ? `<span class="programa-estado" data-tom="desconhecido">não deu para conferir</span>`
          : p.instalado
            ? `<span class="programa-estado" data-tom="ok">instalado</span>`
            : "";

      const botao =
        p.instalado === true
          ? `<button class="btn btn-small" disabled>Já instalado</button>`
          : `<button class="btn btn-small" data-instalar="${escapeHtml(p.id)}" ${
              wingetPronto ? "" : "disabled"
            }>Instalar</button>`;

      return `
        <div class="programa-linha">
          <div class="programa-corpo">
            <span class="programa-nome">${escapeHtml(p.nome)}${estado}</span>
            <span class="programa-descricao">${escapeHtml(p.descricao)}</span>
          </div>
          ${botao}
        </div>`;
    })
    .join("");

  for (const botao of element("programas-lista").querySelectorAll<HTMLButtonElement>(
    "[data-instalar]"
  )) {
    botao.onclick = () => void instalarPrograma(botao);
  }
}

async function instalarPrograma(botao: HTMLButtonElement) {
  const id = botao.dataset.instalar;
  const programa = programasCarregados.find((p) => p.id === id);
  if (!id || !programa) return;

  botao.disabled = true;
  botao.textContent = "Instalando…";
  setStatus(
    "programas-status",
    `Baixando e instalando ${programa.nome} pela fonte oficial. Pode levar alguns minutos.`,
    "progress"
  );

  try {
    await invoke<string>("instalar_programa", { id });
    setStatus("programas-status", `${programa.nome} instalado.`, "ok");

    await carregarProgramas();
  } catch (error) {
    setStatus("programas-status", String(error), "error");
    botao.disabled = false;
    botao.textContent = "Instalar";
  }
}

function ligarProgramas() {
  element<HTMLInputElement>("programas-busca").addEventListener("input", desenharProgramas);
}

type AchadoMouse = "AceleracaoLigada" | "BarraAbaixoDoMeio" | "BarraAcimaDoMeio";

interface CaminhoDoMouse {
  aceleracao: boolean | null;
  barra: number | null;
  taxa_hz: number | null;
  achados: AchadoMouse[];
  falta: string[];
}

interface AchadoNaTela {
  achado: AchadoMouse;
  titulo: string;
  explicacao: string;
  onde_mexer: string;
}

interface CaminhoNaTela {
  caminho: CaminhoDoMouse;
  achados: AchadoNaTela[];
}

const SEGUNDOS_CONTANDO = 3;

/**
 * Conta DENTRO DESTA JANELA com `getCoalescedEvents`: os eventos juntados dariam a taxa da TELA. Nada sai da
 * janela: nenhum gancho global.
 */
function contarRelatos(segundos: number): Promise<number[]> {
  return new Promise((resolve) => {
    const instantes: number[] = [];

    const ouvir = (e: PointerEvent) => {
      const juntados = e.getCoalescedEvents?.() ?? [e];
      for (const p of juntados) instantes.push(p.timeStamp);
    };

    window.addEventListener("pointermove", ouvir, { passive: true });

    window.setTimeout(() => {
      window.removeEventListener("pointermove", ouvir);

      // Em microssegundos. Intervalo não positivo sai (o Rust também descarta).
      const intervalos: number[] = [];
      for (let i = 1; i < instantes.length; i += 1) {
        const dt = Math.round((instantes[i] - instantes[i - 1]) * 1000);
        if (dt > 0) intervalos.push(dt);
      }

      resolve(intervalos);
    }, segundos * 1000);
  });
}

async function lerCaminhoDoMouse() {
  const botao = element<HTMLButtonElement>("mouse-ler");
  botao.disabled = true;
  setStatus(
    "mouse-status",
    `Mexa o mouse em círculos sobre esta janela por ${SEGUNDOS_CONTANDO} segundos…`,
    "progress"
  );

  try {
    const intervalos_us = await contarRelatos(SEGUNDOS_CONTANDO);

    setStatus("mouse-status", "Lendo as opções do ponteiro…", "progress");
    renderCaminhoDoMouse(await invoke<CaminhoNaTela>("caminho_do_mouse", { intervalosUs: intervalos_us }));
    setStatus("mouse-status", "", "ok");
  } catch (error) {
    setStatus("mouse-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

function renderCaminhoDoMouse(r: CaminhoNaTela) {
  const c = r.caminho;
  // "Nada a corrigir" só com as DUAS chaves lidas.
  const leuTudo = c.aceleracao !== null && c.barra !== null;

  text(
    "mouse-tag",
    c.achados.length > 0
      ? `${c.achados.length} coisa(s) no caminho`
      : leuTudo
        ? "movimento 1:1"
        : "não deu para ler"
  );

  // O texto vem do Rust: duas cópias da frase discordariam.
  const achados = r.achados
    .map(
      (a, i) => `
        <article class="finding" data-severity="Important" style="--i:${i}">
          <div class="finding-top">
            <h3>${escapeHtml(a.titulo)}</h3>
          </div>
          <p>${escapeHtml(a.explicacao)}</p>
          <p class="finding-advice">${escapeHtml(a.onde_mexer)}</p>
        </article>`
    )
    .join("");

  const limpo =
    c.achados.length === 0 && leuTudo
      ? `<p class="hint">O Windows não está mexendo no movimento: a aceleração está desligada e a barra de velocidade está no meio. O que chega ao jogo é o que o sensor do mouse mandou.</p>`
      : "";

  const taxa =
    c.taxa_hz === null
      ? ""
      : `<p class="finding-measured">Taxa de varredura medida: ${c.taxa_hz.toFixed(0)} Hz.</p>`;

  element("mouse-result").innerHTML =
    achados + limpo + taxa + blocoDeFaltas(c.falta, "Não entrou na conta:");
}

interface RegressaoNaTela {
  id: string;
  anterior: { media: number; n: number; margem: number | null };
  atual: { media: number; n: number; margem: number | null };
  piorou: boolean;
  suspeitos: {
    mudancas: string[];
    maquina_mudou: string[];
    descartados: number;
    aviso: string;
    como_provar: string;
  };
}

interface HistoricoNaTela {
  historico: { registros: unknown[]; descartados: number };
  regressoes: RegressaoNaTela[];
}

async function lerHistorico() {
  const botao = element<HTMLButtonElement>("historico-ler");
  botao.disabled = true;
  setStatus("historico-status", "Lendo a linha do tempo…", "progress");

  try {
    renderHistorico(await invoke<HistoricoNaTela>("historico_de_desempenho"));
    setStatus("historico-status", "", "ok");
  } catch (error) {
    setStatus("historico-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

function renderHistorico(h: HistoricoNaTela) {
  const pioraram = h.regressoes.filter((r) => r.piorou);

  text(
    "historico-tag",
    h.regressoes.length === 0
      ? "sem comparação ainda"
      : pioraram.length === 0
        ? "nada piorou"
        : `${pioraram.length} de ${h.regressoes.length} pioraram`
  );

  if (h.regressoes.length === 0) {
    element("historico-result").innerHTML = `
      <p class="hint">Ainda não há duas medições da mesma métrica para comparar. Capture a linha de base com repetição hoje e de novo depois de mexer em alguma coisa — a comparação aparece aqui sozinha.</p>`;
    return;
  }

  const ordenadas = [...h.regressoes].sort(
    (a, b) => Number(b.piorou) - Number(a.piorou)
  );

  element("historico-result").innerHTML = ordenadas
    .map((r, i) => {
      // A margem SEMPRE junto: é ela que diz se a diferença é real.
      const lado = (l: RegressaoNaTela["anterior"]) =>
        `${l.media.toFixed(1)}${l.margem === null ? "" : ` ± ${l.margem.toFixed(1)}`} (${l.n}×)`;

      const suspeitos =
        r.suspeitos.mudancas.length > 0
          ? `<p class="finding-measured"><strong>Entre as duas:</strong> ${escapeHtml(
              r.suspeitos.mudancas.join(", ")
            )}</p>
             <p class="hint">${escapeHtml(r.suspeitos.aviso)}</p>
             <p class="finding-advice">${escapeHtml(r.suspeitos.como_provar)}</p>`
          : "";

      const maquina = blocoDeFaltas(
        r.suspeitos.maquina_mudou,
        "A própria máquina mudou no intervalo:"
      );

      // Sem o descarte, "nada entre as medições" seria "o arquivo encheu".
      const descarte =
        r.suspeitos.descartados > 0
          ? `<p class="hint">${r.suspeitos.descartados} registro(s) antigo(s) já saíram do histórico por limite de tamanho.</p>`
          : "";

      return `
        <article class="finding" data-severity="${r.piorou ? "Important" : "Ok"}" style="--i:${i}">
          <div class="finding-top">
            <h3>${escapeHtml(r.id)}</h3>
            <span class="finding-size">${r.piorou ? "piorou" : "sem queda provada"}</span>
          </div>
          <p class="finding-measured">Antes: ${lado(r.anterior)} · Agora: ${lado(r.atual)}</p>
          ${suspeitos}${maquina}${descarte}
        </article>`;
    })
    .join("");
}

const NOME_DA_ETAPA: Record<Etapa, string> = {
  Entrada: "Entrada do mouse e do teclado",
  JogoEPlaca: "Jogo e placa desenhando o quadro",
  Fila: "Fila do driver",
  Apresentacao: "Espera pela tela",
  Tela: "Resposta do painel",
};

function renderLatencia(o: Orcamento) {
  text(
    "latencia-cobertura",
    `${o.etapas_com_valor} de ${o.etapas_totais} etapas medidas`
  );

  // "Pelo menos" é literal: três etapas sem medida. Sem medida nenhuma, nada de "0 ms".
  element("latencia-piso").textContent =
    o.piso_ms === null
      ? "Nenhuma etapa do caminho foi medida ainda. Uma medição de quadros durante a partida preenche a maior delas."
      : `Pelo menos ${o.piso_ms.toFixed(1)} ms, somando só o que foi medido. As ${
          o.etapas_totais - o.etapas_com_valor
        } etapas restantes custam mais do que isso — quanto, ninguém daqui sabe.`;

  element("latencia-etapas").innerHTML = o.parcelas
    .map(
      (p) => `
        <div class="latencia-etapa" data-qualidade="${p.qualidade}">
          <span class="latencia-nome">${escapeHtml(NOME_DA_ETAPA[p.etapa])}</span>
          <span class="latencia-valor">${
            p.ms === null ? "não medida" : `${p.ms.toFixed(1)} ms`
          }</span>
          <span class="latencia-origem">${escapeHtml(p.origem)}</span>
        </div>`
    )
    .join("");

  element("latencia-observacoes").textContent = o.observacoes.join(" ");
}

/** "Cheia, e tudo bem": pintar placa cheia de vermelho faz baixar textura à toa. */
const ESTADO_VRAM: Record<EstadoVram, { rotulo: string; tom: string }> = {
  NaoAvaliado: { rotulo: "não avaliada", tom: "desconhecido" },
  PlacaIntegrada: { rotulo: "vídeo integrado", tom: "neutro" },
  Folgada: { rotulo: "com folga", tom: "ok" },
  CacheCheio: { rotulo: "cheia, e tudo bem", tom: "ok" },
  Transbordando: { rotulo: "transbordando", tom: "alerta" },
  DerramaSemPressao: { rotulo: "outro programa usando", tom: "neutro" },
};

function renderVram(a: AnaliseVram) {
  const { rotulo, tom } = ESTADO_VRAM[a.estado];
  const tag = element("vram-estado");
  tag.textContent = rotulo;
  tag.dataset.estado = tom;

  text("vram-explicacao", a.explicacao);
  text(
    "vram-dedicada",
    medida(a.dedicada_pct, (n) => `${n.toFixed(0)}%`)
  );
  text(
    "vram-folga",
    medida(a.folga_gb, (n) => `${n.toFixed(1)} GB`)
  );

  // Ausente não é "0 GB": normalmente o piso desta máquina ainda não foi observado.
  text(
    "vram-derramado",
    medida(a.derramado_gb, (n) => `${n.toFixed(1)} GB`)
  );

  const nota = element("vram-conselho");
  if (a.conselho) {
    nota.textContent = a.conselho.texto;
    nota.className = "readout-note warn";
  } else if (a.falta.length > 0) {
    nota.textContent = `Ainda não foi possível avaliar por completo: ${a.falta.join(", ")}.`;
    nota.className = "readout-note";
  } else {
    nota.textContent = "Nada a ajustar na memória de vídeo.";
    nota.className = "readout-note";
  }
}

const NOME_DA_CLASSE: Record<Classe, string> = {
  CpuTodosNucleos: "Processador no limite",
  CpuUmNucleo: "Um núcleo no limite",
  Gpu: "Placa de vídeo no limite",
  MemoriaRam: "Memória do sistema apertada",
  MemoriaVideo: "Memória de vídeo apertada",
  Disco: "Disco no limite",
  LimiteTermico: "Firmware segurando por temperatura",
  LimiteEletrico: "Firmware segurando por energia",
  TetoDeQuadros: "Quadros presos na taxa do monitor",
  Engasgo: "Engasgo durante a partida",
  ForaDoHardware: "Limite fora do hardware",
  StreamingDeAssets: "Jogo esperando o disco",
  Rede: "Conexão instável ou perdendo pacote",
};

const SOFTWARE_NAO_RESOLVE: Classe[] = ["LimiteTermico", "LimiteEletrico"];

const CONCLUSAO: Record<Diagnostico["conclusao"], string> = {
  SemEvidencia:
    "Não há medição suficiente para classificar nada. O que falta está listado ao lado.",
  SemCarga:
    "A máquina está parada. Sem carga não existe gargalo para encontrar — medir agora não diria nada sobre um jogo.",
  NadaNoLimite:
    "Há carga e nenhum recurso medido encostou no limite. Isso não é o mesmo que estar tudo bem: veja o que não foi verificado.",
  Encontrado: "",
};

function renderGargalo(d: Diagnostico) {
  const tag = element("gargalo-cobertura");
  tag.textContent = `${d.classes_avaliadas} de ${d.classes_totais} classes avaliadas`;

  const causas = d.achados.filter((a) => a.forca === "Causa").length;

  element("gargalo-conclusao").textContent =
    d.conclusao === "Encontrado"
      ? causas > 0
        ? "O sistema está afirmando o limite abaixo, agora."
        : "Os indícios abaixo são hipóteses: números reais, mas indiretos ou de alguns segundos atrás."
      : CONCLUSAO[d.conclusao];

  const achados = element("gargalo-achados");

  achados.innerHTML =
    d.achados.length === 0
      ? `<p class="empty">Nenhum recurso medido no limite.</p>`
      : d.achados
          .map((a) => {
            const idade =
              a.idade_ms === null || a.idade_ms < 1000
                ? ""
                : ` · leitura de ${(a.idade_ms / 1000).toFixed(0)} s atrás`;

            const aviso = SOFTWARE_NAO_RESOLVE.includes(a.classe)
              ? `<p class="gargalo-aviso">Nenhum plano de energia ou ajuste resolve isto — é refrigeração ou alimentação.</p>`
              : "";

            return `
              <div class="gargalo-achado" data-forca="${a.forca}">
                <span class="gargalo-classe">${escapeHtml(NOME_DA_CLASSE[a.classe])}</span>
                <span class="gargalo-forca">${a.forca === "Causa" ? "causa" : "hipótese"}${idade}</span>
                <span class="gargalo-evidencia">${escapeHtml(a.evidencia)}</span>
                ${aviso}
              </div>
            `;
          })
          .join("");

  element("gargalo-faltas").innerHTML =
    d.nao_verificado.length === 0
      ? `<p class="empty">Todas as classes puderam ser avaliadas.</p>`
      : d.nao_verificado
          .map(
            (n) => `
              <div class="gargalo-falta">
                <span class="gargalo-classe">${escapeHtml(n.classe)}</span>
                <span class="gargalo-evidencia">${escapeHtml(n.falta)}</span>
              </div>
            `
          )
          .join("");
}

function renderEvidencia(telemetry: Telemetry) {
  const { measured, estimated, unknown, total } = telemetry.summary;

  text("evidencia-medido", String(measured));
  text("evidencia-estimado", String(estimated));
  text("evidencia-desconhecido", String(unknown));

  const tag = element("evidencia-tag");
  tag.textContent = `${measured} de ${total} medidos`;
  delete tag.dataset.estado;

  // Inclui a espera de amostragem da CPU: não é o peso que o Otimiza impõe.
  text(
    "evidencia-custo",
    `esta coleta levou ${telemetry.collection_duration_ms} ms, dos quais 200 ms são a espera necessária para amostrar a CPU`
  );

  const nucleos = Object.entries(telemetry.metrics).filter(([id]) => ID_DE_NUCLEO.test(id));

  const linhas = Object.entries(telemetry.metrics)
    .filter(([id]) => !ID_DE_NUCLEO.test(id))
    .sort(([idA, a], [idB, b]) => {
      const porQualidade = ORDEM_QUALIDADE[a.quality] - ORDEM_QUALIDADE[b.quality];
      return porQualidade !== 0 ? porQualidade : idA.localeCompare(idB);
    })
    .map(([id, metric]) => linhaDeEvidencia(id, metric));

  if (nucleos.length > 0) {
    const medidos = nucleos.filter(([, m]) => m.quality === "MEASURED").length;
    linhas.unshift(`
      <div class="evidencia-linha" data-qualidade="${medidos === nucleos.length ? "MEASURED" : "UNKNOWN"}">
        <span class="evidencia-id">cpu.core.*.usage</span>
        <span class="evidencia-valor">${medidos} de ${nucleos.length} núcleos</span>
        <span class="evidencia-origem">sysinfo · ${
          medidos === nucleos.length ? "medido" : "parcial"
        }</span>
      </div>
    `);
  }

  element("evidencia-tabela").innerHTML = linhas.join("");
}

function linhaDeEvidencia(id: string, metric: Metric): string {
  // A idade ao lado da origem: diz se o número é de agora.
  const idade =
    metric.age_ms === null || metric.age_ms < 1000
      ? ""
      : ` · ${(metric.age_ms / 1000).toFixed(0)} s atrás`;

  const origem =
    metric.quality === "UNKNOWN"
      ? escapeHtml(metric.reason ?? "sem motivo declarado")
      : `${escapeHtml(metric.source)} · ${ROTULO_QUALIDADE[metric.quality]}${idade}${
          metric.reason ? ` — ${escapeHtml(metric.reason)}` : ""
        }`;

  return `
    <div class="evidencia-linha" data-qualidade="${metric.quality}">
      <span class="evidencia-id">${escapeHtml(id)}</span>
      <span class="evidencia-valor">${escapeHtml(valorLegivel(metric))}</span>
      <span class="evidencia-origem">${origem}</span>
    </div>
  `;
}

function loadColor(percent: number): string {
  if (percent >= 85) return "var(--red)";
  if (percent >= 60) return "var(--amber)";
  return "var(--cyan)";
}

/** A cor vem do CSS: pintar até o normal tirava o significado da cor; só âmbar e vermelho informam. */
function setBar(id: string, percent: number | null) {
  const bar = document.getElementById(id);
  if (!bar) return;
  const medidorOuNada = bar.closest(".vital") as HTMLElement | null;

  // Vazia E marcada: só vazia seria igual a 0% medido.
  if (percent === null) {
    bar.style.width = "0%";
    if (medidorOuNada) {
      delete medidorOuNada.dataset.nivel;
      medidorOuNada.dataset.estado = "desconhecido";
    }
    return;
  }

  const valor = Math.min(100, Math.max(0, percent));

  bar.style.width = `${valor}%`;

  const medidor = medidorOuNada;
  if (!medidor) return;
  delete medidor.dataset.estado;

  if (valor >= 90) medidor.dataset.nivel = "critico";
  else if (valor >= 75) medidor.dataset.nivel = "atencao";
  else delete medidor.dataset.nivel;
}

/** Mostra a assimetria que a média esconde: um núcleo saturado com os outros dormindo trava o jogo. */
function renderCores(perCore: number[]) {
  const matrix = document.getElementById("core-matrix");
  if (!matrix) return;

  const primeiraVez = matrix.children.length !== perCore.length;

  if (primeiraVez) {
    // O índice vai para o CSS: a cascata sem temporizador por barra.
    matrix.innerHTML = perCore
      .map((_, i) => `<div class="core" style="--i:${i}"><i></i></div>`)
      .join("");
    matrix.dataset.entrada = "true";
  }

  perCore.forEach((load, index) => {
    const core = matrix.children[index] as HTMLElement;
    const fill = core.firstElementChild as HTMLElement;

    // `scaleY`, não `height`: não força relayout (ver `.core i` no styles.css).
    fill.style.transform = `scaleY(${Math.min(100, Math.max(0, load)) / 100})`;
    core.dataset.load = load >= 85 ? "critical" : load >= 60 ? "high" : "normal";
  });

  // Só na entrada: nas atualizações pareceria atrasado.
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

  const linha = document.getElementById("cpu-history-line");
  if (!linha) return;
  linha.setAttribute("points", points.join(" "));

  const lastX = ((cpuHistory.length - 1) / (HISTORY_SAMPLES - 1)) * 300;
  element("cpu-history-area").setAttribute(
    "points",
    `0,70 ${points.join(" ")} ${lastX.toFixed(1)},70`
  );
}

/** Substitui a nota de 0 a 100 (até a 0.12), que não consultava diagnóstico nenhum e ocupava o maior destaque. */
async function runDiagnostic() {
  const button = element<HTMLButtonElement>("run-diagnostic");
  const target = element("diagnostic-result");

  button.disabled = true;
  target.innerHTML = `<p class="empty carregando">Analisando…</p>`;

  try {
    const v = await invoke<Veredito>("diagnostico_rapido");
    renderDiagnostic(v);
    ligarAcoesDosAchados(v);
    aplicarVeredito(v);
  } catch (error) {
    target.innerHTML = `<p class="status error">${escapeHtml(String(error))}</p>`;
  } finally {
    button.disabled = false;
  }
}

/** Uma só para o cartão e a lista: duas cópias divergiriam. */
async function executarAcaoDoAchado(acao: Acao, botao: HTMLButtonElement, nota: HTMLElement) {
  botao.disabled = true;

  try {
    const mensagem = acao.argumento
      ? await invoke<string>(acao.comando, { id: acao.argumento })
      : await invoke<string>(acao.comando);

    nota.textContent = mensagem;

    await carregarVeredito();
  } catch (error) {
    nota.textContent = String(error);
    botao.disabled = false;
  }
}

function ligarAcoesDosAchados(v: Veredito) {
  const problemas = v.achados.filter((a) => a.severity !== "Ok");

  document
    .querySelectorAll<HTMLButtonElement>("#diagnostic-result [data-acao-indice]")
    .forEach((botao) => {
      const indice = Number(botao.dataset.acaoIndice);
      const acao = problemas[indice]?.acao;
      if (!acao) return;

      const nota = botao.parentElement?.querySelector<HTMLElement>(".bottleneck-acao-nota");
      if (!nota) return;

      botao.onclick = () => void executarAcaoDoAchado(acao, botao, nota);
    });
}

function renderDiagnostic(v: Veredito) {
  const problemas = v.achados.filter((a) => a.severity !== "Ok");
  const conferidos = v.achados.filter((a) => a.severity === "Ok");

  const linhas = problemas
    .map((a, indice) => {
      // Cada achado com `acao` ganha botão: até a 1.8 só o eleito ganhava, e o conserto da taxa do monitor sumia
      // justamente nas máquinas com problema.
      const acao = a.acao
        ? `
          <div class="bottleneck-acao">
            <button class="btn btn-small" data-acao-indice="${indice}">
              ${escapeHtml(a.acao.rotulo)}
            </button>
            <span class="bottleneck-acao-nota">${
              a.acao.exige_admin && !isElevated
                ? "Exige abrir o Otimiza como administrador."
                : ""
            }</span>
          </div>`
        : "";

      return `
        <div class="bottleneck" data-severity="${a.severity}">
          <div class="bottleneck-title">${escapeHtml(a.title)}</div>
          <div class="bottleneck-detail">${escapeHtml(a.measured)}</div>
          ${a.advice ? `<div class="bottleneck-detail">${escapeHtml(a.advice)}</div>` : ""}
          ${acao}
        </div>`;
    })
    .join("");

  // "Nada encontrado" vem com os números: tela vazia parece programa quebrado.
  const nada = `<p class="empty">Nenhum problema encontrado — e isso é um
    resultado, não uma tela vazia. ${conferidos.length} verificações passaram
    nesta máquina.</p>`;

  const lacunas = v.lacunas.length
    ? `<div class="bottleneck" data-severity="Ok">
         <div class="bottleneck-title">O que não deu para verificar</div>
         ${v.lacunas
           .map(
             (l) =>
               `<div class="bottleneck-detail">${escapeHtml(l.o_que)}: ${escapeHtml(l.por_que)}</div>`
           )
           .join("")}
       </div>`
    : "";

  element("diagnostic-result").innerHTML =
    (problemas.length ? linhas : nada) + lacunas;

  registrarProblemas(
    "veredito",
    problemas.length,
    problemas.some((a) => a.severity === "Critical")
  );
}

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
    registrarProblemas("firmware", problems, critical);

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

// Retirado na 2.9: DNS não muda ping nem FPS.

/**
 * Espelha `Perda` de `rede.rs` (`#[serde(tag = "tipo")]`): a tela decide por `tipo`, `perdidos` e `alvo`, nunca
 * pela `nota`.
 */
type Perda =
  | { tipo: "Medida"; enviados: number; perdidos: number }
  | { tipo: "NaoMedi" }
  // Servidor de jogo bloqueia ping o tempo todo: vermelho diria rede destruída a quem tem rede boa.
  | { tipo: "NaoRespondePing"; enviados: number }
  | { tipo: "PingLimitado"; enviados: number; perdidos: number };

interface MedidaDeRede {
  alvo: string | null;
  perda: Perda;
  jitter_ms: number | null;
  tempo_ms: number | null;
  nota: string;
}

async function medirPerdaDePacote() {
  const button = element<HTMLButtonElement>("medir-perda");
  button.disabled = true;
  setStatus("perda-status", "Sondando o servidor do jogo…", "progress");

  try {
    const medida = await invoke<MedidaDeRede>("medir_perda_de_pacote");
    renderPerdaDePacote(medida);
  } catch (error) {
    setStatus("perda-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderPerdaDePacote(medida: MedidaDeRede) {
  const semAlvo = medida.alvo === null;
  const perdaTotal = medida.perda.tipo === "Medida" && medida.perda.perdidos === medida.perda.enviados;
  const comPerda = medida.perda.tipo === "Medida" && medida.perda.perdidos > 0;

  text(
    "perda-tag",
    semAlvo
      ? "servidor não identificado"
      : medida.perda.tipo === "NaoMedi"
        ? "medição não rodou"
        : medida.perda.tipo === "NaoRespondePing"
          // Não diz "perdidos": a etiqueta "20/20 perdidos" era o que o cliente lia.
          ? "não responde a ping"
          : medida.perda.tipo === "PingLimitado"
            ? "ping limitado pelo servidor"
            : `${medida.perda.perdidos}/${medida.perda.enviados} perdidos`
  );

  const linhas: string[] = [];

  if (medida.alvo !== null) {
    linhas.push(`
      <div class="startup" style="--i:0">
        <div class="startup-info">
          <span class="startup-name">Servidor</span>
          <span class="startup-exe">${escapeHtml(medida.alvo)}</span>
        </div>
      </div>`);
  }

  if (medida.tempo_ms !== null) {
    linhas.push(`
      <div class="startup" style="--i:1">
        <div class="startup-info">
          <span class="startup-name">Tempo de resposta</span>
        </div>
        <span class="finding-size">${medida.tempo_ms.toFixed(0)} ms</span>
      </div>`);
  }

  if (medida.jitter_ms !== null) {
    linhas.push(`
      <div class="startup" style="--i:2">
        <div class="startup-info">
          <span class="startup-name">Jitter de rede</span>
        </div>
        <span class="finding-size">${medida.jitter_ms.toFixed(0)} ms</span>
      </div>`);
  }

  element("perda-result").innerHTML = linhas.join("");

  // Aviso, nunca erro: a conexão está boa, o que não deu foi medir.
  const nivel =
    semAlvo ||
    medida.perda.tipo === "NaoMedi" ||
    medida.perda.tipo === "NaoRespondePing" ||
    medida.perda.tipo === "PingLimitado"
      ? "warn"
      : perdaTotal
        ? "error"
        : comPerda
          ? "warn"
          : "ok";
  setStatus("perda-status", medida.nota, nivel);
}

interface ShaderCache {
  id: string;
  name: string;
  path: string;
  bytes: number;
  formatted: string;
  files: number;
  oldest: string | null;
  stale: boolean;
}

interface ShaderReport {
  caches: ShaderCache[];
  total_bytes: number;
  total_formatted: string;
  gpu: string | null;
  driver_version: string | null;
  driver_date: string | null;
  driver_age_days: number | null;
  note: string;
}

type VereditoStreaming =
  | "SemMedicao"
  | "SemEngasgos"
  | "NaoEODisco"
  | "TrocaDeMemoria"
  | "AssetsDeMidiaLenta"
  | "OutraCoisaNoDisco";

interface AnaliseStreaming {
  veredito: VereditoStreaming;
  engasgos_por_minuto: number | null;
  coincidencia_pct: number | null;
  latencia_ms: number | null;
  memoria_pct: number | null;
  midia: "Ssd" | "Mecanico" | "NaoDeuParaLer" | null;
  falta: string[];
  explicacao: string;
  proximo_passo: string | null;
}

interface LaboratorioStreaming {
  analise: AnaliseStreaming;
  jogo: { jogo: string; caminho: string; unidade: string | null } | null;
  lacunas: string[];
}

/** Só acende quando mover o jogo resolve. */
const VEREDITO_STREAMING: Record<
  VereditoStreaming,
  { rotulo: string; severidade: string }
> = {
  SemMedicao: { rotulo: "falta medir", severidade: "Info" },
  SemEngasgos: { rotulo: "partida lisa", severidade: "Ok" },
  NaoEODisco: { rotulo: "não é o disco", severidade: "Ok" },
  TrocaDeMemoria: { rotulo: "é a memória", severidade: "Important" },
  AssetsDeMidiaLenta: { rotulo: "é o disco", severidade: "Important" },
  OutraCoisaNoDisco: { rotulo: "outra coisa no disco", severidade: "Info" },
};

async function analyzeStreaming() {
  const button = element<HTMLButtonElement>("analyze-streaming");
  button.disabled = true;
  setStatus("streaming-status", "Lendo os discos e cruzando com a partida…", "progress");

  try {
    const r = await invoke<LaboratorioStreaming>("laboratorio_de_streaming");
    renderStreaming(r);
    setStatus("streaming-status", "", "ok");
  } catch (error) {
    setStatus("streaming-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderStreaming(r: LaboratorioStreaming) {
  const { rotulo, severidade } = VEREDITO_STREAMING[r.analise.veredito];
  text("streaming-tag", rotulo);

  // Os números que sustentaram vão junto, mesmo com algum faltando.
  const numero = (valor: number | null, sufixo: string) =>
    valor === null ? "—" : `${valor.toFixed(1)}${sufixo}`;

  const jogo = r.jogo
    ? `<p class="hint">O jogo que entrou na conta: ${escapeHtml(r.jogo.jogo)}, em ${escapeHtml(
        r.jogo.caminho
      )}.</p>`
    : "";

  const passo = r.analise.proximo_passo
    ? `<p class="finding-advice">${escapeHtml(r.analise.proximo_passo)}</p>`
    : "";

  const falta = blocoDeFaltas(r.analise.falta, "Não entrou na conta, por falta de medida:");

  const lacunas = blocoDeFaltas(r.lacunas, "A leitura dos discos não conseguiu:");

  element("streaming-result").innerHTML = `
    <article class="finding" data-severity="${severidade}" style="--i:0">
      <div class="finding-top">
        <h3>${escapeHtml(rotulo)}</h3>
      </div>
      <p>${escapeHtml(r.analise.explicacao)}</p>
      <div class="readouts readouts-row">
        <div class="readout">
          <span class="readout-label">Trancos por minuto</span>
          <span class="readout-value">${numero(r.analise.engasgos_por_minuto, "")}</span>
        </div>
        <div class="readout">
          <span class="readout-label">Deles, com o disco ocupado</span>
          <span class="readout-value">${numero(r.analise.coincidencia_pct, "%")}</span>
        </div>
        <div class="readout">
          <span class="readout-label">Latência do disco</span>
          <span class="readout-value">${numero(r.analise.latencia_ms, " ms")}</span>
        </div>
        <div class="readout">
          <span class="readout-label">Memória do sistema</span>
          <span class="readout-value">${numero(r.analise.memoria_pct, "%")}</span>
        </div>
      </div>
      ${jogo}${passo}${falta}${lacunas}
    </article>`;
}

async function analyzeShaders() {
  const button = element<HTMLButtonElement>("analyze-shaders");
  button.disabled = true;
  setStatus("shader-status", "Somando os caches de shader…", "progress");

  try {
    const r = await invoke<ShaderReport>("analyze_shaders");
    renderShaders(r);
  } catch (error) {
    setStatus("shader-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderShaders(r: ShaderReport) {
  text("shader-tag", r.total_bytes > 0 ? r.total_formatted : "nada encontrado");

  const placa = r.gpu
    ? `<p class="hint">${escapeHtml(r.gpu)} · driver ${escapeHtml(
        r.driver_version ?? "?"
      )} de ${escapeHtml(r.driver_date ?? "?")}${
        r.driver_age_days !== null ? ` (${r.driver_age_days} dias)` : ""
      }</p>`
    : "";

  element("shader-result").innerHTML =
    placa +
    r.caches
      .map(
        (c, i) => `
    <article class="finding" data-severity="${c.stale ? "Important" : "Ok"}" style="--i:${i}">
      <div class="finding-top">
        <h3>${escapeHtml(c.name)}</h3>
        <span class="finding-size">${escapeHtml(c.formatted)}</span>
        <button class="btn btn-ghost" data-shader="${escapeHtml(c.id)}">Limpar</button>
      </div>
      <p class="finding-advice">${c.files} arquivo(s)${
        c.oldest ? `, mais antigo de ${escapeHtml(c.oldest)}` : ""
      }.${
        c.stale
          ? " <strong>Tem entrada anterior ao driver instalado</strong> — foi compilada por um driver que não existe mais nesta máquina."
          : ""
      }</p>
    </article>`
      )
      .join("");

  setStatus("shader-status", r.note, r.total_bytes > 0 ? "warn" : "ok");
}

async function fixPriority(enable: boolean) {
  const botoes = document.querySelectorAll<HTMLButtonElement>(
    "#unfix-priority"
  );
  botoes.forEach((b) => (b.disabled = true));

  try {
    const executavel = await invoke<string | null>("running_game_executable");

    if (!executavel) {
      setStatus(
        "prio-status",
        "Nenhum jogo conhecido aberto. Abra o jogo primeiro — o ajuste é por nome " +
          "do executável, e é preciso saber qual é.",
        "error"
      );
      return;
    }

    const outcome = await invoke<OptimizationOutcome>("set_persistent_priority", {
      executable: executavel,
      enable,
    });

    text("prio-tag", enable ? "fixada" : "não fixada");
    setStatus("prio-status", outcome.message, "ok");
  } catch (error) {
    setStatus("prio-status", String(error), "error");
  } finally {
    botoes.forEach((b) => (b.disabled = false));
  }
}

interface ReadinessFinding {
  id: string;
  title: string;
  measured: string;
  advice: string;
  severity: Severity;
  fix_location: FixLocation;
  actionable: boolean;
}

interface ReadinessReport {
  findings: ReadinessFinding[];
  note: string;
}

async function analyzeReadiness() {
  const button = element<HTMLButtonElement>("analyze-readiness");
  button.disabled = true;
  setStatus("prontidao-status", "Verificando as condições do sistema…", "progress");

  try {
    const r = await invoke<ReadinessReport>("analyze_readiness");
    renderReadiness(r);
  } catch (error) {
    setStatus("prontidao-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderReadiness(r: ReadinessReport) {
  const problemas = r.findings.filter((f) => f.severity !== "Ok").length;
  text("prontidao-tag", problemas === 0 ? "nada atrapalhando" : `${problemas} a resolver`);

  element("prontidao-result").innerHTML = r.findings.length
    ? r.findings
        .map(
          (f, i) => `
    <article class="finding" data-severity="${f.severity}" style="--i:${i}">
      <div class="finding-top">
        <h3>${escapeHtml(f.title)}</h3>
        ${
          f.actionable
            ? `<button class="btn btn-ghost" data-readiness="${escapeHtml(f.id)}">Corrigir</button>`
            : ""
        }
      </div>
      <p class="finding-measured">${escapeHtml(f.measured)}</p>
      <p class="finding-advice">${escapeHtml(f.advice)}</p>
    </article>`
        )
        .join("")
    : "";

  setStatus("prontidao-status", r.note, problemas === 0 ? "ok" : "warn");
  registrarProblemas("prontidao", problemas, false);
}

type Limite =
  | "CpuUmNucleo"
  | "CpuTodos"
  | "Gpu"
  | "MemoriaVideo"
  | "MemoriaRam"
  | "Disco"
  | "NaoIdentificado"
  | "SemCarga";

interface BottleneckReport {
  limite: Limite;
  summary: string;
  advice: string;
  cpu_total: number;
  cpu_max_core: number;
  gpu_percent: number;
  vram_used_mb: number;
  vram_total_mb: number | null;
  ram_available_gb: number;
  ram_total_gb: number;
  disk_percent: number;
  samples: number;
  seconds: number;
}

const LIMITE_ROTULO: Record<Limite, string> = {
  CpuUmNucleo: "processador, um núcleo",
  CpuTodos: "processador",
  Gpu: "placa de vídeo",
  MemoriaVideo: "memória de vídeo",
  MemoriaRam: "memória",
  Disco: "disco",
  NaoIdentificado: "não identificado",
  SemCarga: "sem carga",
};

async function analyzeBottleneck() {
  const button = element<HTMLButtonElement>("analyze-bottleneck");
  button.disabled = true;
  setStatus(
    "gargalo-status",
    "Medindo processador, placa de vídeo, memória e disco por 10 segundos…",
    "progress"
  );

  try {
    const r = await invoke<BottleneckReport>("analyze_bottleneck", { seconds: 10 });
    renderBottleneck(r);
  } catch (error) {
    setStatus("gargalo-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderBottleneck(r: BottleneckReport) {
  text("gargalo-tag", LIMITE_ROTULO[r.limite]);

  // Placa no talo é boa notícia em jogo; sem carga não é diagnóstico.
  const gravidade: Record<Limite, Severity> = {
    CpuUmNucleo: "Important",
    CpuTodos: "Important",
    Gpu: "Ok",
    MemoriaVideo: "Critical",
    MemoriaRam: "Critical",
    Disco: "Critical",
    NaoIdentificado: "Ok",
    SemCarga: "Ok",
  };

  const vram =
    r.vram_total_mb === null
      ? `${r.vram_used_mb.toFixed(0)} MB`
      : `${r.vram_used_mb.toFixed(0)} de ${r.vram_total_mb.toFixed(0)} MB`;

  element("gargalo-result").innerHTML = `
    <article class="finding" data-severity="${gravidade[r.limite]}" style="--i:0">
      <div class="finding-top"><h3>${escapeHtml(r.summary)}</h3></div>
      <p class="finding-advice">${escapeHtml(r.advice).replace(/\n\n/g, "<br /><br />")}</p>
    </article>

    <div class="readouts readouts-row">
      <div class="readout">
        <span class="readout-label">Processador</span>
        <span class="readout-value">${r.cpu_total.toFixed(0)}%</span>
        <span class="readout-note">pico de um núcleo: ${r.cpu_max_core.toFixed(0)}%</span>
      </div>
      <div class="readout">
        <span class="readout-label">Placa de vídeo</span>
        <span class="readout-value">${r.gpu_percent.toFixed(0)}%</span>
        <span class="readout-note">memória: ${vram}</span>
      </div>
      <div class="readout">
        <span class="readout-label">Memória livre</span>
        <span class="readout-value">${r.ram_available_gb.toFixed(1)} GB</span>
        <span class="readout-note">de ${r.ram_total_gb.toFixed(1)} GB</span>
      </div>
      <div class="readout">
        <span class="readout-label">Disco</span>
        <span class="readout-value">${r.disk_percent.toFixed(0)}%</span>
        <span class="readout-note">ocupado</span>
      </div>
    </div>

    <p class="hint">${r.samples} amostra(s) em ${r.seconds.toFixed(
      1
    )} segundos. O pico de um núcleo usa o maior valor visto, não a média —
    gargalo de um núcleo só aparece em rajadas, e a média esconderia.</p>
  `;

  setStatus(
    "gargalo-status",
    r.summary,
    r.limite === "MemoriaRam" || r.limite === "MemoriaVideo" || r.limite === "Disco"
      ? "error"
      : r.limite === "SemCarga" || r.limite === "NaoIdentificado"
        ? "warn"
        : "ok"
  );
}

interface GameModeStatus {
  game_running: boolean;
  game: string | null;
  active: boolean;
  applied: string[];
}

async function loadGameMode() {
  try {
    const s = await invoke<GameModeStatus>("game_mode_status");

    text(
      "gamemode-tag",
      s.active ? "ligado" : s.game_running ? `${s.game} aberto` : "desligado"
    );

    if (!element("gamemode-status").textContent) {
      setStatus(
        "gamemode-status",
        s.game_running
          ? `${s.game} está aberto agora.`
          : "Nenhum jogo conhecido aberto no momento.",
        "ok"
      );
    }
  } catch {
    // Sem backend o painel continua utilizável.
  }
}

async function setGameMode(active: boolean) {
  const botoes = document.querySelectorAll<HTMLButtonElement>(
    "#gamemode-on, #gamemode-off"
  );
  botoes.forEach((b) => (b.disabled = true));

  try {
    const mensagem = await invoke<string>("set_game_mode", { active });
    setStatus("gamemode-status", mensagem, "ok");
  } catch (error) {
    setStatus("gamemode-status", String(error), "error");
  } finally {
    botoes.forEach((b) => (b.disabled = false));
    await loadGameMode();
  }
}

async function zerarModoJogo() {
  const botao = element<HTMLButtonElement>("gamemode-zerar");
  botao.disabled = true;
  try {
    setStatus("gamemode-status", await invoke<string>("zerar_modo_jogo"), "ok");
  } catch (error) {
    setStatus("gamemode-status", String(error), "error");
  } finally {
    botao.disabled = false;
    await loadGameMode();
  }
}

/** Para colar no Discord em vez de dar acesso remoto (um cliente precisou de AnyDesk e script à mão). */
async function copiarDiagnostico() {
  const botao = element<HTMLButtonElement>("copiar-diagnostico");
  botao.disabled = true;
  // Saúde e térmico levam ~5 s cada: sem aviso, parecia que o PC travou.
  setStatus("diagnostico-status", "Montando o relatório — lê disco e térmico, demora um pouco…", "progress");

  try {
    const texto = await invoke<string>("relatorio_de_suporte");
    await navigator.clipboard.writeText(texto);
    setStatus("diagnostico-status", "Copiado. Cole no atendimento.", "ok");
  } catch (error) {
    // Falha do comando ou recusa da área de transferência: sempre uma frase.
    setStatus("diagnostico-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

interface FrameMeasurement {
  fps: number;
  frames: number;
  seconds: number;
  process: string;
  pid: number;

  // O backend calcula estes três desde antes da 1.9: FPS médio mal se move com disputa de memória ou processador,
  // e engasgo é o que se sente.
  frametime_mediano_ms: number;
  low_1pct: number;
  engasgos_por_minuto: number;
  detalhe_confiavel: boolean;
}

async function measureFrames() {
  const button = element<HTMLButtonElement>("measure-frames");

  await preencherJogoDetectado();

  const processo = element<HTMLInputElement>("fps-process").value.trim();

  if (!processo) {
    setStatus(
      "fps-status",
      "Não achei nenhum jogo aberto. Abra o jogo, entre numa partida, e clique de novo — " +
        "ou escreva o nome do executável aqui do lado.",
      "error"
    );
    return;
  }

  button.disabled = true;
  setStatus("fps-status", `Contando quadros de ${processo} por 8 segundos…`, "progress");

  try {
    const m = await invoke<FrameMeasurement>("measure_frames", {
      process: processo,
      seconds: 8,
    });

    text("fps-tag", `${m.fps.toFixed(0)} FPS`);

    // A média primeiro (é a procurada), depois os que respondem "por que engasga". O PID virou nota: é para suporte.
    const detalhe = m.detalhe_confiavel
      ? `mediana de ${m.frametime_mediano_ms.toFixed(1)} ms entre quadros`
      : "amostra curta — vale medir de novo em movimento";

    element("fps-result").innerHTML = `
      <div class="readouts readouts-row">
        <div class="readout">
          <span class="readout-label">Quadros por segundo</span>
          <span class="readout-value">${m.fps.toFixed(1)}</span>
          <span class="readout-note">média da janela medida</span>
        </div>
        <div class="readout">
          <span class="readout-label">1% piores quadros</span>
          <span class="readout-value">${m.low_1pct.toFixed(1)}</span>
          <span class="readout-note">os momentos ruins da partida</span>
        </div>
        <div class="readout">
          <span class="readout-label">Engasgos por minuto</span>
          <span class="readout-value">${m.engasgos_por_minuto.toFixed(0)}</span>
          <span class="readout-note">${detalhe}</span>
        </div>
        <div class="readout">
          <span class="readout-label">Processo</span>
          <span class="readout-value">${escapeHtml(m.process)}</span>
          <span class="readout-note">${m.frames} quadros · pid ${m.pid}</span>
        </div>
      </div>`;

    setStatus(
      "fps-status",
      `${m.frames} quadros em ${m.seconds.toFixed(1)} segundos. Meça de novo depois de ` +
        `otimizar, na mesma cena do jogo — comparar cena diferente não diz nada.`,
      "ok"
    );
  } catch (error) {
    // Erro em vez de zero: a mensagem vai inteira.
    setStatus("fps-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

interface FiveMFolder {
  id: string;
  name: string;
  path: string;
  bytes: number;
  formatted: string;
  cleanable: boolean;
  explanation: string;
  tradeoff: string | null;
}

interface FiveMReport {
  installed: boolean;
  running: boolean;
  game_running: boolean;
  folders: FiveMFolder[];
  cleanable_bytes: number;
  protected_bytes: number;
  note: string;
}

async function analyzeFiveM() {
  const button = element<HTMLButtonElement>("analyze-fivem");
  button.disabled = true;
  setStatus("fivem-status", "Somando a instalação do FiveM…", "progress");

  try {
    const report = await invoke<FiveMReport>("analyze_fivem");
    renderFiveM(report);
  } catch (error) {
    setStatus("fivem-status", String(error), "error");
  } finally {
    button.disabled = false;
  }

  // Try/catch próprio: não pode apagar o levantamento de pastas acima.
  await analyzeCitizenFx();
}

function renderFiveM(report: FiveMReport) {
  if (!report.installed) {
    text("fivem-tag", "não instalado");
    setStatus("fivem-status", report.note, "ok");
    element("fivem-result").innerHTML = "";
    return;
  }

  const gb = (bytes: number) => (bytes / 1_073_741_824).toFixed(1);
  text("fivem-tag", `${gb(report.cleanable_bytes)} GB a recuperar`);

  const resumo = `
    <div class="readouts readouts-row">
      <div class="readout">
        <span class="readout-label">Dá para recuperar</span>
        <span class="readout-value">${gb(report.cleanable_bytes)} GB</span>
        <span class="readout-note">o servidor reenvia</span>
      </div>
      <div class="readout">
        <span class="readout-label">Protegido</span>
        <span class="readout-value">${gb(report.protected_bytes)} GB</span>
        <span class="readout-note warn">seu perfil e sua conta</span>
      </div>
      <div class="readout">
        <span class="readout-label">FiveM</span>
        <span class="readout-value">${report.running ? "aberto" : "fechado"}</span>
        <span class="readout-note">${
          report.game_running ? "jogo rodando" : "jogo fechado"
        }</span>
      </div>
    </div>
  `;

  element("fivem-result").innerHTML =
    resumo + report.folders.map(renderFiveMFolder).join("");

  setStatus("fivem-status", report.note, "warn");

  if (report.cleanable_bytes > 1_073_741_824) {
    setBadge("badge-jogos", Math.round(report.cleanable_bytes / 1_073_741_824), "warn");
  }
}

function renderFiveMFolder(folder: FiveMFolder, indice: number): string {
  const preco = folder.tradeoff
    ? `<p class="finding-advice"><strong>Ao limpar:</strong> ${escapeHtml(
        folder.tradeoff
      )}</p>`
    : "";

  // Pasta protegida aparece sem botão, para explicar; omitir pareceria esconder.
  const acao = folder.cleanable
    ? `<button class="btn btn-ghost" data-fivem="${escapeHtml(folder.id)}">Limpar</button>`
    : `<span class="state-label">protegido</span>`;

  return `
    <article class="finding" data-severity="${
      folder.cleanable ? "Ok" : "Important"
    }" style="--i:${indice}">
      <div class="finding-top">
        <h3>${escapeHtml(folder.name)}</h3>
        <span class="finding-size">${escapeHtml(folder.formatted)}</span>
        ${acao}
      </div>
      <p class="finding-advice">${escapeHtml(folder.explanation)}</p>
      ${preco}
    </article>
  `;
}

interface PoolAjustado {
  nome: string;
  aumento: number;
  teto_conhecido: number | null;
}

/** Espelha `PoolSizesIncrease` (`#[serde(tag = "status")]`): decide por `pool_sizes.status`, nunca pela `note`. */
type PoolSizesIncrease =
  | { status: "Vazio" }
  | { status: "Configurado"; pools: PoolAjustado[] }
  | { status: "Invalido"; bruto: string };

interface CitizenFxReport {
  existe: boolean;
  caminho: string | null;
  pool_sizes: PoolSizesIncrease | null;
  note: string;
}

/** Junto de `analyzeFiveM`: um clique só em "Analisar". */
async function analyzeCitizenFx() {
  try {
    const report = await invoke<CitizenFxReport>("analyze_citizenfx");
    renderCitizenFx(report);
  } catch (error) {
    text("citizenfx-note", String(error));
    element("citizenfx-result").innerHTML = "";
  }
}

function renderCitizenFx(report: CitizenFxReport) {
  text("citizenfx-note", report.note);

  const resultado = element("citizenfx-result");

  if (!report.pool_sizes || report.pool_sizes.status !== "Configurado") {
    resultado.innerHTML = "";
    return;
  }

  resultado.innerHTML = `
    <div class="readouts readouts-row">
      ${report.pool_sizes.pools
        .map(
          (p) => `
            <div class="readout">
              <span class="readout-label">${escapeHtml(p.nome)}</span>
              <span class="readout-value">${p.aumento}</span>
              <span class="readout-note">${
                p.teto_conhecido !== null
                  ? `teto conhecido: ${p.teto_conhecido}`
                  : "teto não conferido"
              }</span>
            </div>
          `
        )
        .join("")}
    </div>
  `;
}

interface BrowserExtension {
  id: string;
  name: string;
  version: string;
  size_mb: number;
  permissions: number;
  from_webstore: boolean | null;
  stale_versions: number;
}

interface BrowserProfile {
  name: string;
  extensions: BrowserExtension[];
  cache_bytes: number;
  app_data_bytes: number;
}

interface BrowserInfo {
  name: string;
  executable: string;
  is_default: boolean;
  running: boolean;
  ram_mb: number;
  profiles: BrowserProfile[];
}

interface BrowserReport {
  browsers: BrowserInfo[];
  total_cache_mb: number;
  total_app_data_mb: number;
  total_ram_mb: number;
  ram_percent: number;
  total_extensions: number;
  note: string;
}

async function analyzeBrowsers() {
  const button = element<HTMLButtonElement>("analyze-browsers");
  button.disabled = true;
  setStatus("browser-status", "Lendo perfis e somando cache…", "progress");

  try {
    const report = await invoke<BrowserReport>("analyze_browsers");
    renderBrowserReport(report);
  } catch (error) {
    setStatus("browser-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderBrowserReport(report: BrowserReport) {
  if (report.browsers.length === 0) {
    text("browser-tag", "nenhum encontrado");
    setStatus("browser-status", report.note, "ok");
    element("browser-result").innerHTML = "";
    return;
  }

  text("browser-tag", `${report.total_ram_mb.toFixed(0)} MB em uso`);

  // Do navegador inteiro, não por extensão (ver o módulo em Rust).
  const resumo = `
    <div class="readouts readouts-row">
      <div class="readout">
        <span class="readout-label">Memória agora</span>
        <span class="readout-value">${report.ram_percent.toFixed(1)}%</span>
        <span class="readout-note">${report.total_ram_mb.toFixed(0)} MB da sua RAM</span>
      </div>
      <div class="readout">
        <span class="readout-label">Cache</span>
        <span class="readout-value">${report.total_cache_mb.toFixed(0)} MB</span>
        <span class="readout-note">dá para apagar</span>
      </div>
      <div class="readout">
        <span class="readout-label">Dado de aplicativo</span>
        <span class="readout-value">${report.total_app_data_mb.toFixed(0)} MB</span>
        <span class="readout-note warn">não é lixo — não apagamos</span>
      </div>
      <div class="readout">
        <span class="readout-label">Extensões</span>
        <span class="readout-value">${report.total_extensions}</span>
        <span class="readout-note">instaladas</span>
      </div>
    </div>
  `;

  element("browser-result").innerHTML =
    resumo + report.browsers.map(renderBrowser).join("");

  setStatus("browser-status", report.note, "warn");
}

function renderBrowser(browser: BrowserInfo, indice: number): string {
  const extensoes = browser.profiles.flatMap((p) => p.extensions);
  const cacheMb =
    browser.profiles.reduce((soma, p) => soma + p.cache_bytes, 0) / 1_048_576;

  const acao =
    cacheMb < 1
      ? ""
      : browser.running
        ? `<span class="state-label">feche para poder limpar</span>`
        : `<button class="btn btn-ghost" data-browser="${escapeHtml(
            browser.executable
          )}">Limpar ${cacheMb.toFixed(0)} MB de cache</button>`;

  const lista = extensoes.length
    ? extensoes
        .map(
          (e) => `
      <div class="startup">
        <div class="startup-info">
          <span class="startup-name">${escapeHtml(e.name)}</span>
          <span class="startup-exe">v${escapeHtml(e.version)} · ${
            e.permissions
          } permissão(ões)${
            e.stale_versions > 0
              ? ` · ${e.stale_versions} versão(ões) antiga(s) ocupando disco`
              : ""
          }</span>
        </div>
        <span class="finding-size">${e.size_mb.toFixed(1)} MB</span>
      </div>`
        )
        .join("")
    : `<p class="empty">Nenhuma extensão instalada.</p>`;

  return `
    <article class="finding" data-severity="Ok" style="--i:${indice}">
      <div class="finding-top">
        <h3>${escapeHtml(browser.name)}${browser.is_default ? " · padrão" : ""}</h3>
        <span class="chip">${browser.running ? "aberto" : "fechado"}</span>
        <span class="finding-size">${browser.ram_mb.toFixed(0)} MB</span>
        ${acao}
      </div>
      ${lista}
    </article>
  `;
}

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
  culpados_lidos?: boolean;
  firmware_ms?: number | null;
}

const BOOT_TYPE_LABELS: Record<BootType, string> = {
  Full: "boot completo",
  FastStartup: "inicialização rápida",
  Resume: "retomada de hibernação",
};

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

    // Pós-boot alto: a área de trabalho apareceu mas a máquina ainda não dava para usar.
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

  // Acima de 15 s o tempo está na BIOS, onde nenhum ajuste do Windows chega.
  if (report.firmware_ms) {
    const lenta = report.firmware_ms > 15000;
    partes.push(`
      <div class="readouts readouts-row">
        <div class="readout">
          <span class="readout-label">Placa-mãe (BIOS)</span>
          <span class="readout-value">${duracao(report.firmware_ms)}</span>
          <span class="readout-note">antes do Windows começar</span>
        </div>
      </div>
      ${lenta ? `<p class="hint">A placa-mãe está levando mais que o Windows costuma levar. A aba BIOS mostra o que olhar: Fast Boot, modo CSM e, em placa AM5 com EXPO, o Memory Context Restore.</p>` : ""}
    `);
  }

  if (!report.needs_admin && report.culpados_lidos === false) {
    partes.push(
      `<p class="hint">Não deu para ler quais programas atrasam a inicialização — o registro de desempenho do Windows não respondeu. Isso não quer dizer que nenhum atrasa.</p>`
    );
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

  setStatus(
    "boot-status",
    report.note,
    report.last ? "ok" : report.needs_admin ? "warn" : "warn"
  );
}

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
  medido?: boolean;
  eventos_lidos?: boolean;
  placa?: SensoresGpu | null;
}

type EstadoGpu = "Temperatura" | "FreioDeHardware" | "TetoDeEnergiaNormal" | "Livre" | "SemCarga";
type SensoresGpu =
  | {
      tipo: "Lido";
      leitura: {
        nome: string;
        temperatura_c: number | null;
        clock_mhz: number | null;
        clock_max_mhz: number | null;
        potencia_w: number | null;
        limite_potencia_w: number | null;
        uso_pct: number | null;
      };
      estado: { estado: EstadoGpu };
    }
  | { tipo: "NaoDeuParaLer"; motivo: string };

const NA_TELA_DA_PLACA: Record<EstadoGpu, { frase: string; severidade: Severity }> = {
  Temperatura: {
    frase: "O driver está segurando o clock da placa por temperatura. Limpe a poeira, confira as ventoinhas e o fluxo de ar do gabinete.",
    severidade: "Critical",
  },
  FreioDeHardware: {
    frase: "A placa acionou o freio de hardware — costuma ser a fonte, o conector de energia da placa ou a proteção térmica dela.",
    severidade: "Critical",
  },
  TetoDeEnergiaNormal: {
    frase: "Em carga, a placa está no teto de energia. Isso é o funcionamento normal de uma placa em uso máximo, não defeito.",
    severidade: "Ok",
  },
  Livre: { frase: "Em carga, nada está segurando a placa.", severidade: "Ok" },
  SemCarga: {
    frase: "A placa não está em carga agora. Temperatura e freio apareceriam mesmo assim; o resto só se vê com o jogo aberto — analise de novo durante uma partida.",
    severidade: "Ok",
  },
};

function blocoDaPlaca(p: SensoresGpu | null | undefined): string {
  if (!p) return "";
  if (p.tipo === "NaoDeuParaLer") {
    return `<article class="finding" data-severity="Important" style="--i:1"><div class="finding-top"><h3>Placa de vídeo: não deu para ler</h3></div><p class="finding-advice">${escapeHtml(p.motivo)}</p></article>`;
  }
  const l = p.leitura;
  const n = (v: number | null, u: string) => (v === null ? "—" : `${Math.round(v)} ${u}`);
  const tela = NA_TELA_DA_PLACA[p.estado.estado];
  return `
    <article class="finding" data-severity="${tela.severidade}" style="--i:1">
      <div class="finding-top"><h3>${escapeHtml(l.nome)}</h3></div>
      <p class="finding-measured">${n(l.temperatura_c, "°C")} · clock ${n(l.clock_mhz, "MHz")} de ${n(l.clock_max_mhz, "MHz")} · ${n(l.potencia_w, "W")} de ${n(l.limite_potencia_w, "W")} · uso ${n(l.uso_pct, "%")}</p>
      <p class="finding-advice">${escapeHtml(tela.frase)}</p>
    </article>`;
}

interface DispositivoMsi {
  nome: string;
  servico: string;
  msi: boolean | null;
  placa_de_video: boolean;
}

interface NucleoDpc {
  nucleo: string;
  dpc_medio: number;
  dpc_pico: number;
  interrupcao_media: number;
  interrupcao_pico: number;
}
type DiagnosticoDpc = {
  segundos: number;
  nucleos: NucleoDpc[];
  estado: { estado: "Normal" } | { estado: "Alto"; nucleo: string } | { estado: "NaoDeuParaLer"; motivo: string };
};

async function medirDpc() {
  const saida = document.getElementById("dpc-resultado");
  const botao = document.getElementById("dpc-medir") as HTMLButtonElement | null;
  if (!saida) return;
  if (botao) botao.disabled = true;
  saida.innerHTML = `<p class="hint">Medindo por 10 segundos…</p>`;
  try {
    const d = await invoke<DiagnosticoDpc>("diagnostico_dpc");
    if (d.estado.estado === "NaoDeuParaLer") {
      saida.innerHTML = `<p class="hint">Não deu para medir: ${escapeHtml(d.estado.motivo)}</p>`;
      return;
    }
    const pct = (v: number) => `${v.toFixed(1).replace(".", ",")}%`;
    const pior = d.estado.estado === "Alto" ? d.estado.nucleo : null;
    const frase =
      pior === null
        ? "Nenhum núcleo passou da referência (3% em média ou 15% num instante). Se o engasgo continua, meça de novo com o jogo aberto."
        : `O núcleo ${escapeHtml(pior)} passou da referência. Drivers de rede, áudio, USB e placa de vídeo são os suspeitos comuns: atualize-os pelo fabricante e desconecte periféricos um a um, medindo de novo.`;
    saida.innerHTML = `
      <p class="hint">${frase} As referências não foram validadas em muitas máquinas.</p>
      <table class="fg-tabela">
        <thead><tr><th>Núcleo</th><th>DPC médio</th><th>DPC pico</th><th>Interrupção média</th><th>Interrupção pico</th></tr></thead>
        <tbody>${d.nucleos
          .map(
            (n) =>
              `<tr${n.nucleo === pior ? ' data-preso="true"' : ""}><td>${escapeHtml(n.nucleo)}</td><td>${pct(n.dpc_medio)}</td><td>${pct(n.dpc_pico)}</td><td>${pct(n.interrupcao_media)}</td><td>${pct(n.interrupcao_pico)}</td></tr>`,
          )
          .join("")}</tbody>
      </table>`;
  } catch (erro) {
    saida.innerHTML = `<p class="hint">${escapeHtml(String(erro))}</p>`;
  } finally {
    if (botao) botao.disabled = false;
  }
}

async function lerMsi() {
  const saida = document.getElementById("msi-resultado");
  if (!saida) return;
  saida.innerHTML = `<p class="hint">Lendo…</p>`;
  try {
    const lista = await invoke<DispositivoMsi[]>("msi_dispositivos");
    const estado = (d: DispositivoMsi) =>
      d.msi === true ? "MSI ligado" : d.msi === false ? "MSI desligado (o driver declara suporte)" : "o driver não declara MSI";
    saida.innerHTML = `
      <table class="fg-tabela">
        <thead><tr><th>Dispositivo</th><th>Driver</th><th>Interrupção</th></tr></thead>
        <tbody>${lista
          .map(
            (d) =>
              `<tr><td>${escapeHtml(d.nome)}${d.placa_de_video ? " <span class=\"chip\">placa de vídeo</span>" : ""}</td><td class="fg-mono">${escapeHtml(d.servico)}</td><td>${estado(d)}</td></tr>`,
          )
          .join("")}</tbody>
      </table>
      <p class="hint">${lista.length} dispositivo(s). Para a placa de vídeo, o ajuste fica no catálogo (modo Expert), com desfazer.</p>`;
  } catch (erro) {
    saida.innerHTML = `<p class="hint">${escapeHtml(String(erro))}</p>`;
  }
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
  // Só calor e limite elétrico são peça; pintar os quatro de vermelho ensinaria a ignorar o alarme.
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
    ${blocoDaPlaca(report.placa)}
  `;

  // "Nada está segurando" só com as duas leituras (até a 2.7 a falha virava verde).
  const naoLido = report.medido === false || report.eventos_lidos === false;
  const livre = report.culprit === "Nenhum";
  setStatus(
    "thermal-status",
    livre && report.medido === false
      ? "Não deu para medir se o processador está sendo limitado agora."
      : livre
        ? report.eventos_lidos === false
          ? "Nenhum limite ativo agora — mas o registro térmico do Windows não pôde ser lido, então o histórico de calor ficou sem conferir."
          : "Nada está segurando o processador."
        : report.summary,
    livre && naoLido ? "warn" : livre || report.culprit === "Bateria" ? "ok" : "error"
  );
}

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

    // Sem elevação não se lê desgaste nem erros, justamente o que diz se o disco está indo embora.
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

    registrarProblemas("saude", problemas, critico);
  } catch (error) {
    setStatus("health-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

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

  // Até a 2.0, com o serviço da Loja desligado, a lista saía sem apps da Loja e sem uma palavra.
  const leuTudo = report.lacunas.length === 0;
  const faltou = leuTudo ? "" : ` Não consegui ler: ${report.lacunas.join(" · ")}`;

  if (report.items.length === 0) {
    setStatus(
      "bloat-status",
      leuTudo
        ? "Nenhum programa de fábrica encontrado."
        : `Nenhum programa de fábrica entre o que deu para ler.${faltou}`,
      leuTudo ? "ok" : "warn"
    );
    element("bloat-result").innerHTML = "";
    return;
  }

  const medido = report.items.length - report.unmeasured;
  const espaco =
    medido > 0
      ? ` Pelo menos ${report.total_mb.toFixed(0)} MB${
          report.unmeasured > 0 ? ` (${report.unmeasured} sem tamanho informado)` : ""
        }.`
      : "";

  setStatus("bloat-status", `${report.items.length} encontrados.${espaco}${faltou}`, "error");
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

async function analyzeConflicts() {
  const button = element<HTMLButtonElement>("analyze-conflicts");
  button.disabled = true;
  setStatus("conflicts-status", "Lendo programas instalados e processos…", "progress");

  try {
    const report = await invoke<ConflictReport>("analyze_conflicts");
    // "0 programas examinados" sobre lista ilegível seria número falso.
    text(
      "conflicts-summary",
      report.programs_scanned === null
        ? "programas instalados não lidos"
        : `${report.programs_scanned} programas examinados`
    );

    const problemas = report.conflicts.filter((c) => c.severity !== "Ok").length;
    const leuTudo = report.lacunas.length === 0;
    const faltou = leuTudo ? "" : ` Não consegui ler: ${report.lacunas.join(" · ")}`;

    setStatus(
      "conflicts-status",
      problemas > 0
        ? `${problemas} conflito(s) custando desempenho.${faltou}`
        : leuTudo
          ? "Nenhum programa disputando função com outro."
          : `Não dá para dizer que não há conflito.${faltou}`,
      problemas > 0 ? "error" : leuTudo ? "ok" : "warn"
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

/// Decidido no backend (`foldermap::Natureza`): a tela nunca decide pela explicação.
type Natureza =
  | { tipo: "PodeLimpar" }
  | { tipo: "SoOWindowsLimpa" }
  | { tipo: "Seu" }
  | { tipo: "NaoSei" };

interface FolderEntry {
  name: string;
  path: string;
  bytes: number;
  formatted: string;
  percent: number;
  explanation: string;
  partial: boolean;
  natureza: Natureza;
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

  // Não "no seu perfil": `mapear_o_disco` soma também os dois `Program Files`, `ProgramData` e `C:\Windows`.
  if (map.folders.length === 0) {
    setStatus("map-status", "Nenhuma subpasta encontrada nas pastas varridas.", "ok");
    element("map-result").innerHTML = "";
    return;
  }

  // Varredura incompleta primeiro: piso como total manda limpar a pasta errada.
  if (map.timed_out) {
    setStatus(
      "map-status",
      `Pelo menos ${map.total_formatted} nas pastas varridas (perfil, Program Files ` +
        `e Windows). A varredura não terminou dentro do tempo, então as pastas ` +
        `marcadas como "não terminou" têm mais do que o mostrado — e são ` +
        `justamente as maiores.`,
      "warn"
    );
  } else {
    setStatus(
      "map-status",
      `${map.total_formatted} nas pastas varridas (perfil, Program Files e Windows) ` +
        `— não é o disco inteiro.` +
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

  const tamanho = folder.partial
    ? `pelo menos ${folder.formatted}`
    : folder.formatted;

  // `Record` fechado: estado novo em `Natureza` reprova o `tsc` em vez de cair num rótulo genérico.
  const natureza = folder.natureza.tipo;
  const ROTULO: Record<Natureza["tipo"], string> = {
    PodeLimpar: "dá para limpar",
    SoOWindowsLimpa: "só o Windows limpa",
    Seu: "seu arquivo",
    NaoSei: "não consegui ler",
  };

  // O botão só onde leva a algo que age: `PodeLimpar` casa com categoria `cleanable: true` (conferido por
  // `todo_prefixo_com_botao_tem_categoria_que_limpa_de_verdade`). `SoOWindowsLimpa` não: prometer aqui e negar lá.
  // `Seu` não: apagar do cliente não tem desfazer. `NaoSei` não: nem foi lido.
  const acao =
    natureza === "PodeLimpar"
      ? `<button class="btn btn-ghost" data-mapa-limpar="1">Limpar no liberador</button>`
      : `<span class="state-label">${ROTULO[natureza]}</span>`;

  // A frase diz ONDE limpar, já que aqui não é.
  const outroLugar =
    natureza === "SoOWindowsLimpa"
      ? `<p class="finding-advice">Esta pasta é do sistema e o nosso liberador não mexe nela: a remoção comum falha no meio e deixa lixo pela metade. Quem apaga isto é a Limpeza de Disco do Windows (procure por "Limpeza de Disco" no menu Iniciar).</p>`
      : "";

  // Pasta ilegível não é vazia, e o número dela não é o total.
  const semLeitura =
    natureza === "NaoSei"
      ? `<p class="finding-advice">Sem permissão para ler esta pasta: o tamanho acima é um piso, não o total. Não quer dizer que ela esteja vazia.</p>`
      : "";

  return `
    <article class="folder" data-partial="${folder.partial}" data-natureza="${natureza}" style="--i:${indice}">
      <div class="folder-top">
        <span class="folder-name">${escapeHtml(folder.name)}</span>
        <span class="folder-size">${escapeHtml(tamanho)}</span>
        ${acao}
      </div>
      <div class="bar"><i style="width:${Math.min(100, folder.percent)}%"></i></div>
      <span class="folder-path">${escapeHtml(folder.path)}${
        folder.partial ? " · não terminou" : ""
      }</span>
      ${explicacao}
      ${outroLugar}
      ${semLeitura}
    </article>
  `;
}

type EstadoDoRbar =
  | "Ligado"
  | "DesligadoESuportado"
  | "DesligadoSemSuporte"
  | "NaoSei";

interface RelatorioDoRbar {
  estado: EstadoDoRbar;
  modelo: string;
  nota: string;
}

/// UMA tabela de decisão (quatro estados de duas perguntas). Eram duas, e a da faixa (`TOM_DO_RBAR`) não estava
/// provada: `NaoSei` como "ok" passava nos testes e pintava de verde o "ainda não cobrimos placas AMD".
const NA_TELA_DO_RBAR: Record<
  EstadoDoRbar,
  { rotulo: string; severidade: "Ok" | "Important" }
> = {
  Ligado: { rotulo: "ligado", severidade: "Ok" },
  DesligadoESuportado: {
    rotulo: "desligado, e a sua placa aceita",
    severidade: "Important",
  },
  // Diz POR QUE, senão o cliente procura na BIOS uma opção que não existe para ele.
  DesligadoSemSuporte: {
    rotulo: "esta placa não tem",
    severidade: "Ok",
  },
  // Não verificado fica fora do verde.
  NaoSei: { rotulo: "não consegui verificar", severidade: "Important" },
};

/// Derivado da mesma tabela: card e faixa não podem se separar.
function tomDoRbar(estado: EstadoDoRbar): "ok" | "warn" {
  return NA_TELA_DO_RBAR[estado].severidade === "Ok" ? "ok" : "warn";
}

async function analyzeRbar() {
  const button = element<HTMLButtonElement>("analyze-rbar");
  button.disabled = true;
  setStatus("rbar-status", "Lendo a placa de vídeo…", "progress");

  try {
    const relatorio = await invoke<RelatorioDoRbar>("analyze_rbar");
    text("rbar-tag", relatorio.modelo || "placa não identificada");
    element("rbar-result").innerHTML = renderRbar(relatorio);
    setStatus("rbar-status", relatorio.nota, tomDoRbar(relatorio.estado));
  } catch (error) {
    setStatus("rbar-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderRbar(relatorio: RelatorioDoRbar): string {
  const { rotulo, severidade } = NA_TELA_DO_RBAR[relatorio.estado];
  const modelo = relatorio.modelo
    ? `<span class="finding-size">${escapeHtml(relatorio.modelo)}</span>`
    : "";

  return `
    <article class="finding" data-severity="${severidade}" data-estado="${relatorio.estado}">
      <div class="finding-top">
        <h3>Resizable BAR</h3>
        ${modelo}
        <span class="state-label">${escapeHtml(rotulo)}</span>
      </div>
      <p class="finding-advice">${escapeHtml(relatorio.nota)}</p>
    </article>
  `;
}

type StatusDoAjuste =
  | "Aplicado"
  | "JaEstavaBom"
  | "NaoSuportado"
  | "FalhouAoAplicar"
  | "FalhouNaVerificacao"
  | "Mudaria"
  | "Pulado";

type ClasseDoAjuste = "Segura" | "Recomendada" | "Avancada";
type DesfechoDoPlano = "Sucesso" | "EmParte" | "Falhou";
type FabricanteDaCpu = "Intel" | "Amd" | "Outro";

interface MaquinaDoPlano {
  notebook: boolean;
  tem_bateria: boolean;
  fabricante_da_cpu: FabricanteDaCpu;
  cpu: string;
  nucleos_logicos: number;
  modern_standby: boolean;
  build_do_windows: number;
  windows11: boolean;
}

interface AjusteDoPlano {
  nome: string;
  subgrupo: string;
  ajuste: string;
  classe: ClasseDoAjuste;
  porque: string;
  suportado: boolean;
  ac_antes: number | null;
  dc_antes: number | null;
  ac_alvo: number | null;
  dc_alvo: number | null;
  ac_depois: number | null;
  dc_depois: number | null;
  status: StatusDoAjuste;
  mensagem: string;
}

interface RelatorioDoPlano {
  maquina: MaquinaDoPlano;
  simulacao: boolean;
  plano_existia: boolean;
  guid_do_plano: string | null;
  guid_anterior: string | null;
  plano_ativo: boolean;
  ajustes: AjusteDoPlano[];
  aplicados: number;
  ja_estavam_bons: number;
  nao_suportados: number;
  falhas: number;
  desfecho: DesfechoDoPlano;
}

/**
 * O estado vem do Rust, frase e cor daqui. "Não suportado" NÃO é verde: não é problema, mas também não é
 * verificação aprovada.
 */
const NA_TELA_DO_AJUSTE: Record<
  StatusDoAjuste,
  { rotulo: string; severidade: "Ok" | "Important" | "Neutral" }
> = {
  Aplicado: { rotulo: "aplicado e conferido", severidade: "Ok" },
  JaEstavaBom: { rotulo: "já estava bom", severidade: "Ok" },
  NaoSuportado: { rotulo: "não existe neste Windows", severidade: "Neutral" },
  FalhouAoAplicar: { rotulo: "o Windows recusou", severidade: "Important" },
  // Aceito e não entrou: o que um otimizador que confia no código de saída chama de sucesso.
  FalhouNaVerificacao: { rotulo: "não entrou", severidade: "Important" },
  // Só em simulação. Ver `StatusDoAjuste::Mudaria`.
  Mudaria: { rotulo: "vai mudar", severidade: "Neutral" },
  // "NÃO ENTRA": `Pulado` cobre não valer aqui E ser avançado fora do padrão; "não se aplica" contradizia a linha
  // de baixo.
  Pulado: { rotulo: "não entra", severidade: "Neutral" },
};

/** "100 → 100" não é informação. */
function numerosDoAjuste(a: AjusteDoPlano): string {
  if (a.status === "NaoSuportado" || a.status === "Pulado") return "";

  if (a.status === "JaEstavaBom") {
    return `tomada ${valorDoAjuste(a.ac_antes)} · bateria ${valorDoAjuste(a.dc_antes)}`;
  }

  const lado = (antes: number | null, alvo: number | null, nome: string) =>
    alvo === null
      ? `${nome} ${valorDoAjuste(antes)} (não mexe)`
      : `${nome} ${valorDoAjuste(antes)} → ${valorDoAjuste(alvo)}`;

  return `${lado(a.ac_antes, a.ac_alvo, "tomada")} · ${lado(a.dc_antes, a.dc_alvo, "bateria")}`;
}

const NA_TELA_DO_DESFECHO: Record<
  DesfechoDoPlano,
  { frase: string; tom: "ok" | "warn" | "error" }
> = {
  Sucesso: { frase: "Plano OTIMIZA criado, configurado e ativo.", tom: "ok" },
  // Parcial não é falha, e a frase diz.
  EmParte: {
    frase: "Plano OTIMIZA ativo, com ressalvas — veja a lista abaixo.",
    tom: "warn",
  },
  Falhou: {
    frase: "O plano não pôde ser criado ou ativado. Nada foi mudado no seu plano de energia.",
    tom: "error",
  },
};

function valorDoAjuste(v: number | null): string {
  return v === null ? "—" : String(v);
}

function linhaDoAjuste(a: AjusteDoPlano): string {
  const { rotulo, severidade } = NA_TELA_DO_AJUSTE[a.status];

  const numeros = numerosDoAjuste(a);
  const mudanca = numeros
    ? `<span class="finding-size">${escapeHtml(numeros)}</span>`
    : "";

  const mensagem = a.mensagem
    ? `<p class="finding-advice">${escapeHtml(a.mensagem)}</p>`
    : "";

  return `
    <article class="finding" data-severity="${severidade}" data-estado="${a.status}">
      <div class="finding-top">
        <h3>${escapeHtml(a.nome)}</h3>
        ${mudanca}
        <span class="state-label">${escapeHtml(rotulo)}</span>
      </div>
      <p class="finding-advice">${escapeHtml(a.porque)}</p>
      ${mensagem}
    </article>
  `;
}

function frasesDaMaquina(m: MaquinaDoPlano): string {
  const partes = [
    m.cpu,
    `${m.nucleos_logicos} processadores lógicos`,
    m.notebook ? "notebook" : "desktop",
    m.windows11 ? `Windows 11 (build ${m.build_do_windows})` : `Windows 10 (build ${m.build_do_windows})`,
  ];

  if (m.modern_standby) partes.push("Modern Standby");
  if (m.tem_bateria) partes.push("com bateria");

  // Antes do clique: num notebook os valores agressivos valem só na tomada.
  const bateria = m.notebook
    ? " Na bateria, o plano fica no padrão do Windows — desempenho travado fora da tomada é autonomia queimada e calor por nada."
    : "";

  return `${partes.join(" · ")}.${bateria}`;
}

function renderPlano(r: RelatorioDoPlano): string {
  const contagem = [
    `${r.aplicados} aplicados`,
    `${r.ja_estavam_bons} já estavam bons`,
    `${r.nao_suportados} não existem aqui`,
    `${r.falhas} não entraram`,
  ].join(" · ");

  const cabecalho = r.simulacao
    ? `<p class="hint">Simulação: nada foi escrito no seu computador. Abaixo, o que mudaria.</p>`
    : `<p class="hint">${escapeHtml(contagem)}.</p>`;

  return cabecalho + r.ajustes.map(linhaDoAjuste).join("");
}

interface DiagnosticoDeEnergia {
  maquina: MaquinaDoPlano;
  elevado: boolean;
  powercfg_responde: boolean;
  planos_legiveis: boolean;
  planos: [string, string][];
  plano_otimiza_existe: boolean;
  registro_de_energia_legivel: boolean;
  ajustes_suportados: number;
  ajustes_totais: number;
  ajustes_ausentes: string[];
  processo_64_bits: boolean;
  /** Não é bom/ruim: os dois casos são normais. */
  governo_do_processador: "OProcessador" | "OWindows" | "NaoDeuParaLer";
  explicacao_do_governo: string;
  avisos: string[];
}

/** A cor sai daqui; não há verde para "não sei". */
function linhaDaChecagem(rotulo: string, ok: boolean, detalhe: string): string {
  return `
    <article class="finding" data-severity="${ok ? "Ok" : "Important"}">
      <div class="finding-top">
        <h3>${escapeHtml(rotulo)}</h3>
        <span class="state-label">${ok ? "sim" : "não"}</span>
      </div>
      <p class="finding-advice">${escapeHtml(detalhe)}</p>
    </article>
  `;
}

function renderDiagnostico(d: DiagnosticoDeEnergia): string {
  const checagens = [
    linhaDaChecagem(
      "O Otimiza está como administrador",
      d.elevado,
      d.elevado
        ? "Pode criar e ativar plano de energia."
        : "Sem isto, nenhum plano de energia pode ser criado. Reabra como administrador.",
    ),
    linhaDaChecagem(
      "O powercfg responde nesta máquina",
      d.powercfg_responde,
      d.powercfg_responde
        ? "É por ele que o plano é criado, configurado e ativado."
        : "Sem ele não há como mexer em plano de energia neste computador.",
    ),
    linhaDaChecagem(
      "Os planos de energia puderam ser lidos",
      d.planos_legiveis,
      d.planos_legiveis
        ? `${d.planos.length} plano(s) neste computador.`
        : "A lista de planos voltou vazia ou ilegível.",
    ),
    linhaDaChecagem(
      "A árvore de energia do registro pôde ser lida",
      d.registro_de_energia_legivel,
      d.registro_de_energia_legivel
        ? "É dela que sai quais ajustes existem neste Windows."
        : "Sem ela, todo ajuste apareceria como inexistente — e seria mentira.",
    ),
    linhaDaChecagem(
      "O Otimiza está rodando em 64 bits",
      d.processo_64_bits,
      d.processo_64_bits
        ? "Lê o registro verdadeiro do Windows."
        : "Em 32 bits sobre um Windows de 64, as leituras caem num espelho e saem erradas.",
    ),
    linhaDaChecagem(
      "Ajustes de energia disponíveis aqui",
      d.ajustes_suportados === d.ajustes_totais,
      d.ajustes_ausentes.length === 0
        ? `Os ${d.ajustes_totais} ajustes do Otimiza existem neste Windows.`
        : `${d.ajustes_suportados} de ${d.ajustes_totais}. Este Windows não tem: ${d.ajustes_ausentes.join(", ")}. O plano é montado sem eles, e isso não é falha.`,
    ),
  ];

  const avisos = d.avisos.length
    ? `<p class="hint">${d.avisos.map(escapeHtml).join("<br>")}</p>`
    : "";

  const plano = d.plano_otimiza_existe
    ? `<p class="hint">O plano OTIMIZA já existe nesta máquina.</p>`
    : "";

  // Fora das checagens: diz qual ajuste pesa mais NESTE computador.
  const governo = `<p class="bloco-de-prosa">${escapeHtml(d.explicacao_do_governo)}</p>`;

  return `<p class="hint">${escapeHtml(frasesDaMaquina(d.maquina))}</p>${plano}${checagens.join("")}${governo}${avisos}`;
}

type Vistoria =
  | { estado: "NaoExiste" }
  | { estado: "Integro" }
  | { estado: "DesativadoPorFora" }
  | { estado: "Desviado"; ativo: boolean; ajustes: string[] }
  | { estado: "NaoConsegui" };

/** Três nomes e a contagem: com o plano resetado, dez nomes viravam texto que ninguém lê. */
function listarPoucos(nomes: string[], teto = 3): string {
  if (nomes.length <= teto) return nomes.join(", ");

  return `${nomes.slice(0, teto).join(", ")} e mais ${nomes.length - teto}`;
}

/** Só "Desviado" pede ação e liga o reparo; `NaoConsegui` não vira "repare". */
function naTelaDaVistoria(v: Vistoria): {
  frase: string;
  tom: "ok" | "warn" | "error";
  reparar: boolean;
} {
  switch (v.estado) {
    case "NaoExiste":
      return {
        frase: "Não há plano OTIMIZA nesta máquina.",
        tom: "ok",
        reparar: false,
      };
    case "Integro":
      return {
        frase: "O plano OTIMIZA está ativo e como foi deixado.",
        tom: "ok",
        reparar: false,
      };
    case "DesativadoPorFora":
      return {
        frase:
          "O plano OTIMIZA está intacto, mas o computador voltou a usar outro. " +
          "Instalador de driver e utilitário de fabricante costumam trocar o plano ativo sem avisar.",
        tom: "warn",
        reparar: true,
      };
    case "Desviado":
      return {
        frase: `Algum programa mudou ${v.ajustes.length} ajuste(s) do plano OTIMIZA: ${listarPoucos(v.ajustes)}.`,
        tom: "warn",
        reparar: true,
      };
    case "NaoConsegui":
      return {
        frase: "Não foi possível ler o plano de energia para conferir.",
        tom: "error",
        reparar: false,
      };
  }
}

async function vistoriarPlano() {
  try {
    const v = await invoke<Vistoria>("vistoriar_plano_otimiza");
    const { frase, tom, reparar } = naTelaDaVistoria(v);

    element("plano-reparar").hidden = !reparar;
    setStatus("plano-status", frase, tom);
  } catch (error) {
    element("plano-reparar").hidden = true;
    setStatus("plano-status", String(error), "error");
  }
}

async function repararPlano() {
  const botao = element<HTMLButtonElement>("plano-reparar");
  botao.disabled = true;
  setStatus("plano-status", "Reaplicando só o que saiu do lugar…", "progress");

  try {
    const r = await invoke<RelatorioDoPlano>("reparar_plano_otimiza");

    const { frase, tom } = NA_TELA_DO_DESFECHO[r.desfecho];
    mostrarPlano(r, tom, frase);

    // Confere de novo: se algo continuar mexendo, o botão volta.
    await vistoriarPlano();
  } catch (error) {
    setStatus("plano-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

async function diagnosticarEnergia() {
  const botao = element<HTMLButtonElement>("plano-diagnostico");
  botao.disabled = true;
  setStatus("plano-status", "Lendo o que esta máquina permite…", "progress");

  try {
    const d = await invoke<DiagnosticoDeEnergia>("diagnostico_de_energia");

    const area = element("plano-diagnostico-result");
    area.innerHTML = renderDiagnostico(d);
    area.hidden = false;

    // O tom sai dos avisos, não da contagem de ajustes.
    setStatus(
      "plano-status",
      d.avisos.length === 0
        ? "Esta máquina aceita tudo o que o Otimiza faz no plano de energia."
        : "Há ressalvas nesta máquina — veja abaixo.",
      d.avisos.length === 0 ? "ok" : "warn",
    );
  } catch (error) {
    setStatus("plano-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

/** VISÍVEL antes de copiar: cinquenta linhas sobre o computador da pessoa, que precisa ver o que manda. */
let labGerado: string | null = null;

async function gerarLab() {
  const botao = element<HTMLButtonElement>("lab-gerar");
  botao.disabled = true;
  setStatus("lab-status", "Lendo a máquina — não altera nada…", "progress");

  try {
    labGerado = await invoke<string>("relatorio_de_compatibilidade");

    const area = element("lab-texto");
    area.textContent = labGerado;
    area.hidden = false;

    element("lab-copiar").hidden = false;
    text("lab-tag", "pronto");
    setStatus(
      "lab-status",
      "Pronto. Leia se quiser, e mande no atendimento junto da sua dúvida.",
      "ok",
    );
  } catch (error) {
    setStatus("lab-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

async function copiarLab() {
  if (labGerado === null) return;

  try {
    await navigator.clipboard.writeText(labGerado);
    setStatus("lab-status", "Copiado. Cole no atendimento.", "ok");
  } catch (error) {
    setStatus(
      "lab-status",
      `Não consegui copiar (${String(error)}). O texto está aí em cima e pode ser selecionado.`,
      "error",
    );
  }
}

let planoAnterior: string | null = null;

function mostrarPlano(r: RelatorioDoPlano, tom: "ok" | "warn" | "error", frase: string) {
  text("plano-tag", r.simulacao ? "simulação" : r.plano_ativo ? "ativo" : "não ativo");

  const maquina = element("plano-maquina");
  maquina.textContent = frasesDaMaquina(r.maquina);
  maquina.hidden = false;

  element("plano-result").innerHTML = renderPlano(r);
  setStatus("plano-status", frase, tom);
}

async function simularPlano() {
  const botao = element<HTMLButtonElement>("plano-simular");
  botao.disabled = true;
  setStatus("plano-status", "Lendo a máquina e os ajustes de energia…", "progress");

  try {
    const r = await invoke<RelatorioDoPlano>("simular_plano_otimiza");

    mostrarPlano(
      r,
      "ok",
      r.plano_existia
        ? "O plano OTIMIZA já existe nesta máquina. Nada foi escrito."
        : "Nada foi escrito. É isto que mudaria se você aplicar."
    );
  } catch (error) {
    setStatus("plano-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

async function aplicarPlano() {
  const botao = element<HTMLButtonElement>("plano-aplicar");
  botao.disabled = true;
  setStatus("plano-status", "Criando o plano, configurando e conferindo cada ajuste…", "progress");

  try {
    const r = await invoke<RelatorioDoPlano>("aplicar_plano_otimiza");

    const { frase, tom } = NA_TELA_DO_DESFECHO[r.desfecho];
    mostrarPlano(r, tom, frase);

    // Sem o GUID anterior não há volta honesta.
    planoAnterior = r.guid_anterior;
    element("plano-desfazer").hidden = !(r.plano_ativo && planoAnterior !== null);

    // Sem isto a lista ofereceria aplicar o já aplicado.
    void loadOptimizations();

    void vistoriarPlano();
  } catch (error) {
    setStatus("plano-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

async function desfazerPlano() {
  if (planoAnterior === null) return;

  const botao = element<HTMLButtonElement>("plano-desfazer");
  botao.disabled = true;
  setStatus("plano-status", "Voltando ao seu plano de energia…", "progress");

  try {
    // Pelo "Desfazer" do histórico: o item sai junto.
    await invoke("revert_optimization", { id: "plano_otimiza" });

    text("plano-tag", "não ativo");
    element("plano-result").innerHTML = "";
    element("plano-desfazer").hidden = true;
    planoAnterior = null;

    element("plano-reparar").hidden = true;

    setStatus(
      "plano-status",
      "Pronto: o seu plano de energia voltou a ser o ativo e o plano OTIMIZA foi apagado.",
      "ok"
    );

    void loadOptimizations();
  } catch (error) {
    setStatus("plano-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

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
  // Serviço em processo compartilhado não tem número atribuível: "0 MB" seria mentira.
  const memoria =
    service.ram_mb === null
      ? ""
      : `<span class="finding-size">${service.ram_mb.toFixed(0)} MB</span>`;

  // Protegido é informação, não botão (responde "por que não aparece meu antivírus").
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

  // Quem decide é o backend (`Medida`), não `bytes === 0`: o WinSxS sem administrador não é "vazio".
  const naoMedido = item.medida.tipo === "NaoConsegui";
  const vazio = !naoMedido && item.bytes === 0;

  const rotulo = naoMedido ? "não medido" : vazio ? "vazio" : "pela Limpeza de Disco";
  const acao = item.cleanable
    ? `<button class="btn btn-ghost" data-space="${item.id}">Limpar</button>`
    : `<span class="state-label">${rotulo}</span>`;

  // Só o zero medido é verde; o não medido é pendente.
  return `
    <article class="finding" data-severity="${vazio ? "Ok" : "Important"}">
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

  registrarProblemas("memoria", problemas, critico);
}

/**
 * Roda sozinho ao abrir: antes eram dezessete botões em cinco abas, e a tela dizia "memória sem problemas" numa
 * máquina que travava.
 */
async function carregarVeredito() {
  try {
    aplicarVeredito(await invoke<Veredito>("diagnostico_rapido"));
  } catch (error) {
    // Falhar também é informação: nunca "analisando" para sempre.
    const cartao = element("veredito");
    delete cartao.dataset.nivel;
    text("veredito-rotulo", "Não foi possível diagnosticar");
    text("veredito-frase", "O diagnóstico automático não completou.");
    text("veredito-detalhe", String(error));
    text("veredito-conselho", "");
    mostrarAcaoDoVeredito(null);
  }
}

/** Separado da busca: cartão do Painel e lista do Diagnóstico usam o mesmo resultado. */
function aplicarVeredito(v: Veredito) {
  const cartao = element("veredito");

  const nivel = v.principal
    ? v.principal.severity === "Critical"
      ? "critico"
      : "importante"
    : "ok";

  cartao.dataset.nivel = nivel;

  text(
    "veredito-rotulo",
    v.principal
      ? nivel === "critico"
        ? "O que está travando este PC"
        : "Vale corrigir"
      : "Diagnóstico concluído"
  );
  text("veredito-frase", v.frase);
  text("veredito-detalhe", v.detalhe);
  text("veredito-conselho", v.principal?.advice ?? "");

  const junto = element("veredito-junto");
  junto.hidden = v.corroboracoes.length === 0;
  junto.innerHTML = v.corroboracoes
    .map(
      (c) =>
        `<li><strong>${escapeHtml(c.title)}</strong> — ${escapeHtml(c.measured)}</li>`
    )
    .join("");

  mostrarAcaoDoVeredito(v.principal?.acao ?? null);
  mostrarRecuperacao(v);

  mostrarAchadoNoPortao(v);

  // Frase vermelha ao lado de esfera neutra faz duvidar das duas.
  esfera?.definirNivel(nivel as "ok" | "importante" | "critico");
  esfera?.redesenhar();

  pilaresDoPortao?.definirNivel(nivel as "ok" | "importante" | "critico");
  pilaresDaChegada?.definirNivel(nivel as "ok" | "importante" | "critico");

  // Só para crítico: alerta sempre ligado para de ser lido.
  const alerta = element("alerta-global");
  alerta.hidden = nivel !== "critico";

  if (nivel === "critico") {
    text("alerta-global-texto", v.frase);
    alerta.onclick = () => showTab("painel");
  }

  const lacunas = element("veredito-lacunas");
  lacunas.hidden = v.lacunas.length === 0;
  lacunas.innerHTML = v.lacunas
    .map(
      (l) => `<li>${escapeHtml(l.o_que)}: ${escapeHtml(l.por_que)}</li>`
    )
    .join("");
}

/** Só quando o Otimiza resolve sozinho, a minoria dos casos. */
function mostrarAcaoDoVeredito(acao: Acao | null) {
  const bloco = element("veredito-acao");
  const botao = element<HTMLButtonElement>("veredito-corrigir");

  bloco.hidden = acao === null;
  if (!acao) return;

  botao.textContent = acao.rotulo;
  botao.disabled = false;

  // Exige administrador: dizer ANTES do clique.
  text(
    "veredito-acao-nota",
    acao.exige_admin && !isElevated
      ? "Exige abrir o Otimiza como administrador."
      : ""
  );

  botao.onclick = async () => {
    botao.disabled = true;

    try {
      const mensagem = acao.argumento
        ? await invoke<string>(acao.comando, { id: acao.argumento })
        : await invoke<string>(acao.comando);

      text("veredito-acao-nota", mensagem);
      await carregarVeredito();
    } catch (error) {
      text("veredito-acao-nota", String(error));
      botao.disabled = false;
    }
  };
}

/** `Ok`: li, ou não existe porque nada foi aplicado. `Ilegivel` é o perigoso. */
type LeituraDoHistorico =
  | { estado: "Ok" }
  | { estado: "Ilegivel"; motivo: string; guardado_em: string | null };

/**
 * Com o histórico ilegível tudo aparece disponível e "Desfazer tudo" não acha nada, como numa máquina limpa, com
 * as mudanças aplicadas. ANTES da lista: quem chega aqui vai decidir o que aplicar.
 */
async function avisarSeOHistoricoNaoFoiLido() {
  const caixa = element("optimization-historico-aviso");

  let leitura: LeituraDoHistorico;

  try {
    leitura = await invoke<LeituraDoHistorico>("estado_do_historico");
  } catch {
    caixa.hidden = true;
    return;
  }

  if (leitura.estado === "Ok") {
    caixa.hidden = true;
    return;
  }

  const guardado = leitura.guardado_em
    ? ` O arquivo foi preservado em ${leitura.guardado_em} — ele não foi apagado.`
    : "";

  caixa.textContent =
    "Não consegui ler o histórico de mudanças desta máquina, então não sei o que já foi " +
    "aplicado aqui. A lista abaixo pode mostrar como disponível algo que já está ativo, e o " +
    "\u201CDesfazer tudo\u201D não vai encontrar o que desfazer." +
    guardado;

  caixa.hidden = false;
}

async function loadOptimizations() {
  try {
    optimizations = await invoke<OptimizationInfo[]>("list_optimizations");
    renderFilters();
    renderOptimizations();
    await avisarSeOHistoricoNaoFoiLido();
    // Depois da lista: o aviso mostra os NOMES, que vêm dela.
    await carregarConflitosDeAjuste();
    // 21 ajustes podem ser aplicados, conferidos, e não fazer diferença se a conta for outra.
    await carregarContaQueEstaRodando();
    await carregarNiveis();
  } catch (error) {
    element("optimization-list").innerHTML =
      `<p class="status error">${escapeHtml(String(error))}</p>`;
  }
}

interface ProfileInfo {
  id: string;
  name: string;
  description: string;
  tradeoff: string;
  optimization_ids: string[];
}

let profiles: ProfileInfo[] = [];
let activeProfile: string | null = null;
let searchTerm = "";

/** Leitura que falha vira modo Simples, o seguro. */
function lerModoExpert(): boolean {
  try {
    return localStorage.getItem("otimiza.modo") === "expert";
  } catch {
    return false;
  }
}
let modoExpert = lerModoExpert();

function aplicarModoExpert() {
  document.body.dataset.modo = modoExpert ? "expert" : "simples";
  document.querySelectorAll<HTMLElement>("[data-so-expert]").forEach((el) => (el.hidden = !modoExpert));
}

interface NivelNaTela {
  id: string;
  nome: string;
  promessa: string;
  exigencia: string;
  itens: string[];
  acrescenta: string[];
  aplica_de_uma_vez: boolean;
}

let niveis: NivelNaTela[] = [];
let nivelEscolhido: string | null = null;

interface PassoDaBios {
  id: string;
  fase: string;
  titulo: string;
  onde: string;
  o_que_faz: string;
  risco_e_volta: string;
  medido_aqui: boolean;
}

interface BiosNaTela {
  leitura: {
    placa_mae: string | null;
    versao_da_bios: string | null;
    data_da_bios: string | null;
    uefi: boolean | null;
    secure_boot: boolean | null;
    lacunas: string[];
  };
  passos: PassoDaBios[];
}

const NA_TELA_DA_FASE: Record<string, string> = {
  UsarOQueTem: "Fase 1 — ligar o que você já comprou",
  Documentado: "Fase 2 — o que o fabricante documenta",
  ExigeTeste: "Fase 3 — exige teste de estabilidade",
  NaoOrientamos: "Fase 4 — overclock manual",
  UltimoRecurso: "Fase 5 — atualizar a BIOS",
};

async function carregarPassoAPassoDaBios() {
  const alvo = element("bios-passos");

  let b: BiosNaTela;

  try {
    b = await invoke<BiosNaTela>("passo_a_passo_da_bios");
  } catch (erro) {
    alvo.innerHTML = `<p class="status warn">${escapeHtml(String(erro))}</p>`;
    return;
  }

  const l = b.leitura;
  const identificacao = l.placa_mae
    ? `<p class="bloco-de-prosa"><strong>${escapeHtml(l.placa_mae)}</strong>${
        l.versao_da_bios ? ` · BIOS ${escapeHtml(l.versao_da_bios)}` : ""
      }${l.uefi === false ? " · iniciando em modo Legacy" : ""}${
        l.uefi === true ? " · UEFI" : ""
      }. Use este modelo para achar o manual certo — os nomes das opções mudam de
      placa para placa, e um vídeo de outra placa manda você procurar um menu que
      não existe aqui.</p>`
    : "";

  // Fase vazia é etapa que o cliente procura e não encontra.
  let faseAtual = "";
  const corpo = b.passos
    .map((p) => {
      const cabecalho =
        p.fase === faseAtual
          ? ""
          : `<p class="profiles-label">${escapeHtml(NA_TELA_DA_FASE[p.fase] ?? p.fase)}</p>`;
      faseAtual = p.fase;

      // "Medido aqui" separa "vale para você" de "boa ideia em geral".
      const marca = p.medido_aqui
        ? `<span class="chip" data-recommended="true">medido nesta máquina</span>`
        : "";

      return (
        cabecalho +
        `<div class="causa" data-severity="${p.medido_aqui ? "Ok" : "Neutral"}">
           <p class="causa-titulo"><strong>${escapeHtml(p.titulo)}</strong> ${marca}</p>
           <p class="effect">${escapeHtml(p.o_que_faz)}</p>
           <p class="causa-medido"><strong>Onde:</strong> ${escapeHtml(p.onde)}</p>
           <p class="causa-confirmar"><strong>Risco e como voltar:</strong>
              ${escapeHtml(p.risco_e_volta)}</p>
         </div>`
      );
    })
    .join("");

  const lacunas = l.lacunas.length
    ? `<p class="hint">${l.lacunas.map(escapeHtml).join("<br>")}</p>`
    : "";

  alvo.innerHTML =
    `<p class="hint">O Otimiza não altera nada na BIOS e não tem como fazer isso — esta
       lista é para você conferir com o manual da sua placa. Ela está em ordem de risco:
       quem parar na Fase 1 pegou a maior parte do ganho disponível.</p>` +
    identificacao +
    corpo +
    lacunas;
}

async function carregarNiveis() {
  try {
    niveis = await invoke<NivelNaTela[]>("niveis_de_otimizacao");
    renderNiveis();
  } catch {
    element("nivel-chips").innerHTML = "";
  }
}

function renderNiveis() {
  element("nivel-chips").innerHTML = niveis
    .map(
      (n) =>
        `<button class="profile-chip" data-nivel="${escapeHtml(n.id)}"
           aria-pressed="${nivelEscolhido === n.id}">${escapeHtml(n.nome)}</button>`
    )
    .join("");
}

/**
 * Marca as caixas, não aplica. No Experimental manda para os grupos: tudo de uma vez derrubou o FPS de um
 * cliente.
 */
function escolherNivel(id: string) {
  const nivel = niveis.find((n) => n.id === id);
  if (!nivel) return;

  nivelEscolhido = nivelEscolhido === id ? null : id;
  renderNiveis();
  renderOptimizations();

  const detalhe = element("nivel-detalhe");

  if (!nivelEscolhido) {
    detalhe.hidden = true;
    return;
  }

  const doNivel = optimizations.filter((o) => nivel.itens.includes(o.id));
  const aAplicar = doNivel.filter((o) => o.state === "Available");
  const jaTem = doNivel.filter(
    (o) => o.state === "Applied" || o.state === "AlreadyOptimal"
  ).length;

  // Sem botão no Experimental, decidido pelo backend: botão com "não clique" embaixo seria armadilha com aviso.
  const acao = !nivel.aplica_de_uma_vez
    ? `<p class="effect" data-severity="Important">Este nível não tem botão de aplicar
         tudo, e é de propósito. Os ajustes dele rendem numa máquina e custam quadro em
         outra — foi aplicar todos juntos que derrubou o FPS de um cliente. Use o painel
         <strong>“Testar um grupo de cada vez”</strong>, na aba Jogos: ele aplica um grupo
         por vez e compara a medição dos dois lados.</p>`
    : aAplicar.length > 0
      ? `<br /><button id="aplicar-nivel" class="btn btn-primary">Aplicar os ${
          aAplicar.length
        } deste nível</button>`
      : "";

  detalhe.innerHTML =
    `<p><strong>${escapeHtml(nivel.nome)}.</strong> ${escapeHtml(nivel.promessa)}</p>` +
    `<p><strong>O que exige:</strong> ${escapeHtml(nivel.exigencia)}</p>` +
    `<p><strong>${aAplicar.length} a aplicar${
      jaTem > 0 ? `, ${jaTem} que o seu PC já tem` : ""
    }.</strong> A lista ao lado está mostrando só os itens deste nível.</p>` +
    acao;

  detalhe.hidden = false;
}

async function loadProfiles() {
  try {
    profiles = await invoke<ProfileInfo[]>("list_profiles");
    renderProfileChips();
  } catch {
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

/** Sugestão visível e editável: a pessoa vê o que foi marcado e o que o perfil abre mão. */
function selectProfile(id: string) {
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

  // Perfil que só se elogia é propaganda.
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

/** O efeito honesto entra na busca: é onde estão as palavras lembradas ("boot", "travada"). */
function matchesSearch(item: OptimizationInfo, termo: string): boolean {
  if (!termo) return true;

  const alvo = `${item.name} ${item.description} ${item.honest_effect} ${item.id}`;

  const normalizar = (t: string) =>
    t.toLowerCase().normalize("NFD").replace(/\p{Diacritic}/gu, "");

  return normalizar(alvo).includes(normalizar(termo));
}

function renderOptimizations() {
  const termo = searchTerm.trim();

  const visible = optimizations
    .filter((item) => preferences.show_unavailable || item.state !== "Unavailable")
    .filter((item) => modoExpert || !item.expert || item.state === "Applied")
    .filter((item) => activeCategory === "Todas" || item.category === activeCategory)
    .filter((item) => matchesSearch(item, termo))
    .filter((item) => {
      if (!activeProfile) return true;
      const perfil = profiles.find((p) => p.id === activeProfile);
      return perfil ? perfil.optimization_ids.includes(item.id) : true;
    })
    // Perfil diz ONDE, nível diz ATÉ ONDE: interseção, senão o "Seguro" não significaria nada.
    .filter((item) => {
      if (!nivelEscolhido) return true;
      const nivel = niveis.find((n) => n.id === nivelEscolhido);
      return nivel ? nivel.itens.includes(item.id) : true;
    });

  const available = optimizations.filter((item) => item.state === "Available").length;
  const applied = optimizations.filter((item) => item.state === "Applied").length;
  text("optimization-count", `${available} a aplicar · ${applied} ativas`);
  setBadge("badge-otimizacoes", available);

  // Só sete tocam FPS de forma mensurável: melhor saber antes de clicar.
  const mudamFps = optimizations.filter(
    (item) => item.expected_gain === "Measurable" && item.state === "Available"
  ).length;
  const naoMudamFps = optimizations.filter(
    (item) => item.expected_gain === "Responsiveness" && item.state === "Available"
  ).length;

  // Os que não prometem desempenho são contados em voz alta.
  const semGanho = optimizations.filter(
    (item) => item.expected_gain === "NoGain" && item.state === "Available"
  ).length;

  text(
    "optimization-expectativa",
    available === 0
      ? "Nada a aplicar: este PC já está com tudo que o Otimiza sabe fazer."
      : `Das ${available} a aplicar, ${mudamFps} mudam o FPS de forma mensurável. ` +
        `Outras ${naoMudamFps} liberam recursos de fundo e não mudam FPS — ` +
        `valem pela limpeza, não pelo jogo.` +
        (semGanho > 0
          ? ` E ${semGanho} não mudam desempenho nenhum: são higiene e ` +
            `privacidade, e estão aqui porque você pode querer, não porque ` +
            `deixam o PC rápido.`
          : "")
  );

  if (visible.length === 0) {
    element("optimization-list").innerHTML = termo
      ? `<p class="empty">Nada encontrado para "${escapeHtml(termo)}".</p>`
      : activeProfile
        ? `<p class="empty">Nenhum item deste perfil aparece nesta categoria.</p>`
        : `<p class="empty">Nenhuma otimização nesta categoria.</p>`;
    return;
  }

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

  // O que muda FPS antes do que não muda (antes as 7 vinham misturadas com 17 de higiene, com o mesmo peso); dentro
  // do nível, o que pesa NESTA máquina sobe.
  const ordenados = [...items].sort(
    (a, b) =>
      GAIN_ORDER[a.expected_gain] - GAIN_ORDER[b.expected_gain] ||
      Number(b.recommended) - Number(a.recommended)
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
  if (item.expert) chips.push(`<span class="chip" data-warn="true">Expert — meça antes e depois</span>`);
  if (item.retirado)
    chips.push(`<span class="chip" data-warn="true">retirado na 2.9 — só desfazer</span>`);

  // O rótulo vem do backend: "pode custar FPS" sobre um ajuste que ataca o engasgo faria recusar a troca certa.
  const risco_ = item.risco_de_fps;
  const podeCustar = risco_.risco === "PodeCustar";

  if (podeCustar)
    chips.push(`<span class="chip" data-warn="true">${escapeHtml(risco_.rotulo)}</span>`);

  const risco = podeCustar
    ? `<p class="effect" data-severity="Important"><strong>${escapeHtml(risco_.rotulo)}.</strong>
         ${escapeHtml(risco_.quando)}
         <br>Por isso ele não entra no “Otimizar agora”: aplique, reinicie se
         pedir, e meça antes de deixar ligado.</p>`
    : "";

  const detail = item.detail ? `<p class="detail">${escapeHtml(item.detail)}</p>` : "";
  const condicao = item.condicao ? `<p class="detail">${escapeHtml(item.condicao)}</p>` : "";

  // Linha, não cartão: dezessete cartões parecem catálogo; linhas parecem o painel de configurações de um sistema.
  // O efeito honesto fica visível na linha, não dobrado atrás de um clique.
  return `
    <details class="optimization" data-state="${item.state}">
      <summary class="opt-row linha-ajuste">
        <span class="gain-dot" data-gain="${item.expected_gain}" ${item.recommended ? 'data-recommended="true"' : ""}></span>
        <span class="linha-ajuste-corpo">
          <span class="linha-ajuste-nome opt-name">${escapeHtml(item.name)}</span>
          <span class="linha-ajuste-descricao">${escapeHtml(item.honest_effect)}</span>
        </span>
        ${actionControl(item)}
      </summary>
      <div class="opt-body">
        <div class="optimization-meta">${chips.join("")}</div>
        ${risco}
        ${condicao}
        ${detail}
      </div>
    </details>
  `;
}

/** "Já otimizado" não vira botão. */
function actionControl(item: OptimizationInfo): string {
  switch (item.state) {
    // Interruptor só para aplicado E reversível: pela forma ele promete que dá para desligar.
    case "Applied":
      return item.reversible
        ? `<button class="switch" role="switch" aria-checked="true" aria-label="Desfazer ${escapeHtml(item.name)}" data-id="${item.id}" data-action="revert" data-admin="${item.requires_admin}"></button>`
        : `<span class="state-label" data-state="Applied">${STATE_LABELS.Applied}</span>`;
    case "Available":
      return `<button class="switch" role="switch" aria-checked="false" aria-label="Aplicar ${escapeHtml(item.name)}" data-id="${item.id}" data-action="apply" data-admin="${item.requires_admin}"></button>`;
    // Não ler não tira a escolha do dono: "Tentar aplicar" não promete que falta aplicar; fora do lote, item a item.
    case "Unknown":
      return `<button class="btn btn-ghost" data-id="${item.id}" data-action="apply" data-admin="${item.requires_admin}">Tentar aplicar</button>`;
    default:
      return `<span class="state-label" data-state="${item.state}">${STATE_LABELS[item.state]}</span>`;
  }
}

/** O Rust emite um evento por passo, com o valor anterior de cada mudança. */
async function listenToBatchProgress() {
  await listen<BatchStep>("optimize:step", (event) => appendLogLine(event.payload));
}

function resetLog(title: string) {
  element("live-log").hidden = false;
  element("live-log-title").textContent = title;
  element("live-log-count").textContent = "";
  element("live-log-lines").innerHTML = "";

  // Escondida e vazia: senão o lote seguinte começaria com a barra cheia do anterior.
  const barra = element<HTMLProgressElement>("live-log-barra");
  barra.value = 0;
  barra.max = 1;
  barra.hidden = true;
}

function appendLogLine(step: BatchStep) {
  const lines = element("live-log-lines");

  // O passo 0 é o ponto de restauração: não conta como otimização.
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

  // Os MESMOS números do texto: sem estimativa de tempo.
  const barra = element<HTMLProgressElement>("live-log-barra");
  barra.max = step.total;
  barra.value = step.index;
  barra.hidden = step.total === 0;

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

/** O Windows não deixa um processo se elevar sozinho: explica e reabre com autorização. */
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
    // Em desenvolvimento o processo continua vivo e devolve a explicação.
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

type InicioDoEssencial = "Desativado" | "Ativo" | "NaoExiste" | "NaoConsegui";

interface ChecagemDosEssenciais {
  servicos: { servico: string; inicio: InicioDoEssencial }[];
  desativados: number;
  fabricante: string | null;
  modelo: string | null;
}

/** A frase mora aqui; cada consequência segue a descrição que a Microsoft publica do serviço. */
const NA_TELA_DOS_ESSENCIAIS: Record<string, { nome: string; quebra: string }> = {
  PlugPlay: {
    nome: "Plug and Play",
    quebra: "a Microsoft avisa que desligá-lo deixa o sistema instável",
  },
  AppXSvc: {
    nome: "Implantação AppX",
    quebra: "aplicativos da Microsoft Store não instalam e podem não funcionar",
  },
  ClipSVC: {
    nome: "Licenças de Cliente",
    quebra: "aplicativos da Microsoft Store não funcionam direito",
  },
  LicenseManager: {
    nome: "Gerenciador de Licenças do Windows",
    quebra: "o que veio da Microsoft Store não funciona direito",
  },
  StateRepository: {
    nome: "Repositório de Estado",
    quebra: "é a base do modelo de aplicativos do Windows",
  },
  AppReadiness: {
    nome: "Preparação de Aplicativos",
    quebra: "prepara os aplicativos no primeiro login e ao instalar novos",
  },
  KeyIso: {
    nome: "Isolamento de Chave CNG",
    quebra: "isola as chaves privadas usadas em criptografia",
  },
  CryptSvc: {
    nome: "Serviços de Criptografia",
    quebra: "confere a assinatura dos arquivos do Windows e permite instalar programas novos",
  },
  SamSs: {
    nome: "Gerente de Contas de Segurança",
    quebra: "a Microsoft diz para não desligar: outros serviços podem não iniciar",
  },
  TimeBrokerSvc: {
    nome: "Agente de Tempo",
    quebra: "o trabalho em segundo plano dos aplicativos pode não acontecer",
  },
  TokenBroker: {
    nome: "Gerenciador de Conta da Web",
    quebra: "login com conta Microsoft dentro de aplicativos pode falhar",
  },
};

let loteAguardando: { progress: string; only?: string[] } | null = null;

/** Falha na conferência não prende o lote: não conferir não é achar desligado. */
async function essenciaisLiberamOLote(progress: string, only?: string[]): Promise<boolean> {
  let checagem: ChecagemDosEssenciais;

  try {
    checagem = await invoke<ChecagemDosEssenciais>("checar_essenciais");
  } catch (error) {
    console.error("Erro ao conferir os serviços essenciais:", error);
    return true;
  }

  if (checagem.desativados === 0) return true;

  loteAguardando = { progress, only };
  mostrarAvisoDosEssenciais(checagem);
  return false;
}

function mostrarAvisoDosEssenciais(checagem: ChecagemDosEssenciais) {
  const origem = [checagem.fabricante, checagem.modelo]
    .filter((parte): parte is string => Boolean(parte))
    .join(" · ");
  const quantos =
    checagem.desativados === 1
      ? "1 serviço que o Windows precisa está desativado"
      : `${checagem.desativados} serviços que o Windows precisa estão desativados`;

  // Entre aspas e sem adjetivo: fabricante de verdade também se registra ali.
  element("essenciais-origem").textContent = origem
    ? `O Windows deste PC se identifica como "${origem}", e ${quantos}:`
    : `Neste Windows, ${quantos}:`;

  element("essenciais-lista").innerHTML = checagem.servicos
    .filter((s) => s.inicio === "Desativado")
    .map((s) => {
      const tela = NA_TELA_DOS_ESSENCIAIS[s.servico];
      const nome = tela ? tela.nome : s.servico;
      const quebra = tela ? ` — ${tela.quebra}` : "";
      return `<li><strong>${escapeHtml(nome)}</strong>${escapeHtml(quebra)}</li>`;
    })
    .join("");

  element("essenciais-modal").hidden = false;
  element<HTMLButtonElement>("essenciais-religar").focus();
}

function fecharAvisoDosEssenciais() {
  element("essenciais-modal").hidden = true;
  loteAguardando = null;
}

async function otimizarMesmoAssim() {
  const lote = loteAguardando;
  fecharAvisoDosEssenciais();

  if (lote) await runBatch("optimize_now", lote.progress, lote.only, true);
}

async function religarEssenciais() {
  if (!isElevated) {
    fecharAvisoDosEssenciais();
    askForAdmin(
      "Religar serviços do Windows exige permissão de administrador. Podemos " +
        "reabrir o Otimiza com essa permissão?"
    );
    return;
  }

  const botao = element<HTMLButtonElement>("essenciais-religar");
  botao.disabled = true;
  botao.textContent = "Religando…";

  try {
    const outcome = await invoke<OptimizationOutcome>("religar_essenciais");
    fecharAvisoDosEssenciais();
    setStatus("optimization-status", outcome.message, outcome.success ? "ok" : "error");
  } catch (error) {
    fecharAvisoDosEssenciais();
    setStatus("optimization-status", String(error), "error");
  } finally {
    botao.disabled = false;
    botao.textContent = "Religar os essenciais";
    await loadOptimizations();
  }
}

function pendingAdminCount(): number {
  return optimizations.filter(
    (item) => item.state === "Available" && item.reversible && item.requires_admin
  ).length;
}

async function runBatch(
  command: string,
  progress: string,
  only?: string[],
  essenciaisConferidos = false
) {
  // Antes de tudo: senão o que já estava quebrado passa a parecer efeito da otimização.
  if (command === "optimize_now" && !essenciaisConferidos) {
    if (!(await essenciaisLiberamOLote(progress, only))) return;
  }

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

    // Sair da conta não é reiniciar: reiniciar por nada perde tempo, e reiniciar quando precisava sair da conta faz
    // concluir que não funcionou.
    const logoff = outcomes.some((outcome) => outcome.success && outcome.requires_logoff);

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
        `${outcomes.length} concluídas.${restart ? " Reinicie o PC para tudo valer." : ""}${
          !restart && logoff ? " Saia e entre de novo na conta para tudo valer." : ""
        }`,
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
  element<HTMLInputElement>("pref-gamemode").checked = preferences.auto_game_mode;
  element<HTMLInputElement>("pref-medir-sozinho").checked = preferences.medir_quadros_sozinho;

  document.querySelectorAll<HTMLButtonElement>("#pref-interval button").forEach((button) => {
    const chosen = Number(button.dataset.interval) === preferences.metrics_interval_seconds;
    button.setAttribute("aria-pressed", String(chosen));
  });
}

/** Adota o que o backend devolveu: valores fora da faixa são corrigidos. */
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

async function loadRestoreStatus() {
  try {
    const status = await invoke<RestoreStatus>("restore_status");

    text("restore-tag", status.available ? "ativa" : "indisponível");
    setStatus("restore-status", status.message, status.available ? "ok" : "error");

    // Consome espaço em disco: decisão do dono.
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

/// Dezenas de segundos: o Windows tira um instantâneo do volume.
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

async function loadStartup() {
  try {
    const entries = await invoke<StartupEntry[]>("list_startup");
    const enabled = entries.filter((entry) => entry.enabled).length;
    text("startup-count", `${enabled} de ${entries.length} ligados`);

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
  const CLASSE: Record<string, [string, string]> = {
    Essencial: ["essencial", "Segurança, áudio, vídeo ou touchpad. Desligar tira algo do sistema."],
    Util: ["útil", "Sincronização de nuvem ou software de periférico. Dá para abrir na mão quando precisar."],
    Opcional: ["opcional", "Loja de jogo, mensageiro, música ou navegador pré-aberto: ocupa memória desde o boot até alguém usar."],
  };
  const classe = entry.classe && CLASSE[entry.classe]
    ? `<span class="chip" title="${CLASSE[entry.classe][1]}"${entry.classe === "Essencial" ? "" : entry.classe === "Opcional" ? ' data-recommended="true"' : ""}>${CLASSE[entry.classe][0]}</span>`
    : "";
  const scope =
    (entry.hive === "HKLM"
      ? `<span class="chip" title="Vale para todos os usuários">todos</span>`
      : "") + classe;

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

interface ReportSaved {
  path: string;
  is_pdf: boolean;
  optimizations: number;
  changes: number;
  note: string;
}

let lastComparison: BenchmarkComparison | null = null;

async function exportReport() {
  const button = element<HTMLButtonElement>("export-report");
  button.disabled = true;
  setStatus(
    "report-status",
    "Levantando o estado da máquina e montando o PDF… pode levar meio minuto.",
    "progress"
  );

  try {
    const saved = await invoke<ReportSaved>("export_report", {
      comparison: lastComparison,
    });

    setStatus(
      "report-status",
      `${saved.is_pdf ? "PDF" : "Arquivo"} salvo em ${saved.path} — ` +
        `${saved.optimizations} otimização(ões), ${saved.changes} alteração(ões)` +
        (lastComparison ? "." : ", sem medição de antes e depois.") +
        (saved.note ? ` ${saved.note}` : ""),
      saved.is_pdf ? "ok" : "warn"
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

      // Refazer a medição ao exportar custaria 12 s e mediria outro momento.
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

/** Montada do próprio HTML: botão novo entra sozinho. */
interface Comando {
  rotulo: string;
  secao: string;
  executar: () => void;
}

function montarComandos(secoes: HTMLButtonElement[]): Comando[] {
  const nomeDaSecao = new Map<string, string>();

  for (const item of secoes) {
    const rotulo = item.querySelector(".nav-rotulo")?.textContent?.trim() ?? "";
    nomeDaSecao.set(item.dataset.tab!, rotulo);
  }

  const comandos: Comando[] = secoes.map((item) => ({
    rotulo: `Ir para ${nomeDaSecao.get(item.dataset.tab!)}`,
    secao: "Navegação",
    executar: () => showTab(item.dataset.tab!),
  }));

  // Os botões gerados por linha ficam fora: só fazem sentido junto do item.
  document.querySelectorAll<HTMLButtonElement>(".tab-panel .btn").forEach((botao) => {
    const rotulo = botao.textContent?.trim();
    const painel = botao.closest<HTMLElement>(".tab-panel");

    // Sobe pelos ancestrais procurando `hidden` (um DIV escondido deixava o botão entrar), mas PARA no painel: o
    // painel fica `hidden` sempre que a aba não é a ativa, e o comando troca de aba antes de focar.
    let escondido = false;
    for (let el: HTMLElement | null = botao; el && el !== painel; el = el.parentElement) {
      if (el.hidden) {
        escondido = true;
        break;
      }
    }

    if (!rotulo || !painel || botao.hasAttribute("data-fivem") || escondido) return;

    const aba = painel.id.replace("tab-", "");

    // Os botões de exame se chamam todos "Analisar" (o título do painel diz o assunto); na paleta o assunto vem junto.
    const painelPai = botao.closest<HTMLElement>(".panel");
    const assunto = painelPai?.querySelector(".panel-head h2")?.textContent?.trim();

    comandos.push({
      rotulo: assunto && assunto !== rotulo ? `${rotulo} — ${assunto}` : rotulo,
      secao: nomeDaSecao.get(aba) ?? aba,
      executar: () => {
        showTab(aba);
        botao.scrollIntoView({ block: "center" });
        botao.focus();
      },
    });
  });

  return comandos;
}

function wireComandos(secoes: HTMLButtonElement[]) {
  const caixa = element("comandos");
  const campo = element<HTMLInputElement>("comandos-busca");
  const lista = element("comandos-lista");

  let comandos: Comando[] = [];
  let visiveis: Comando[] = [];
  let escolhido = 0;

  const abrir = () => {
    // Na hora de abrir: os painéis carregam conteúdo depois do início.
    comandos = montarComandos(secoes);
    caixa.hidden = false;
    campo.value = "";
    filtrar("");
    campo.focus();
  };

  const fechar = () => {
    caixa.hidden = true;
  };

  function filtrar(termo: string) {
    const normalizar = (t: string) =>
      t.toLowerCase().normalize("NFD").replace(/\p{Diacritic}/gu, "");

    const alvo = normalizar(termo.trim());

    visiveis = comandos
      .filter((c) => !alvo || normalizar(`${c.rotulo} ${c.secao}`).includes(alvo))
      .slice(0, 40);

    escolhido = 0;
    desenhar();
  }

  function desenhar() {
    lista.innerHTML = visiveis.length
      ? visiveis
          .map(
            (c, i) => `
        <button class="comando" data-indice="${i}" aria-selected="${i === escolhido}">
          <span class="comando-rotulo">${escapeHtml(c.rotulo)}</span>
          <span class="comando-secao">${escapeHtml(c.secao)}</span>
        </button>`
          )
          .join("")
      : `<p class="empty">Nada encontrado.</p>`;
  }

  // Um caminho só para a busca.
  element("abrir-comandos-topo").addEventListener("click", abrir);

  document.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      caixa.hidden ? abrir() : fechar();
      return;
    }

    if (caixa.hidden) return;

    if (event.key === "Escape") {
      fechar();
      return;
    }

    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const passo = event.key === "ArrowDown" ? 1 : visiveis.length - 1;
      escolhido = (escolhido + passo) % Math.max(1, visiveis.length);
      desenhar();
      return;
    }

    if (event.key === "Enter" && visiveis[escolhido]) {
      event.preventDefault();
      visiveis[escolhido].executar();
      fechar();
    }
  });

  campo.addEventListener("input", () => filtrar(campo.value));

  lista.addEventListener("click", (event) => {
    const alvo = (event.target as HTMLElement).closest<HTMLElement>("[data-indice]");
    if (!alvo) return;

    visiveis[Number(alvo.dataset.indice)]?.executar();
    fechar();
  });

  caixa.addEventListener("click", (event) => {
    if (event.target === caixa) fechar();
  });
}

/** Duração, se cancelar é seguro e os dois avisos vêm de `Receita` (`reparo.rs`): a tela só lê. */
interface FerramentaDeReparo {
  nome: string;
  minutos_tipicos: readonly [number, number];
  cancelar_e_seguro: boolean;
  aviso: string | null;
  oferece_reset_base: boolean;
  aviso_reset_base: string | null;
}

/**
 * Decidido no backend (`ResultadoSfc::severidade()`): `CorrigiuEmParte` e `Corrigiu` tinham a mesma cor pelo
 * prefixo "Corrigiu ".
 */
type TomResultado = "ok" | "atencao" | "erro";

interface UltimoResultadoReparo {
  tom: TomResultado;
  texto: string;
}

function tomParaStatus(tom: TomResultado): "ok" | "warn" | "error" {
  if (tom === "ok") return "ok";
  if (tom === "erro") return "error";
  return "warn";
}

/** O tom nasce da variante de `Desfecho`: a comparação `desfecho === "Terminou."` não existe mais. */
type DesfechoReparo = UltimoResultadoReparo;

/** A razão de falha do DISM vem no `stderr` e chegava misturada às porcentagens. */
type OrigemAndamento = "saida" | "erro";

interface Andamento {
  linha: string;
  origem: OrigemAndamento;
}

/** Texto de apresentação, escrito pela tela: não muda o risco de um clique. */
interface TextoReparo {
  titulo: string;
  descricao: string;
}

const TEXTOS_REPARO: Record<string, TextoReparo> = {
  VerificarArquivos: {
    titulo: "Verificar arquivos do sistema",
    descricao:
      "Confere os arquivos do Windows contra o original e corrige o que estiver corrompido (sfc /scannow).",
  },
  RepararImagem: {
    titulo: "Reparar a imagem do Windows",
    descricao:
      "Busca arquivos originais no Windows Update para substituir os que a verificação sozinha não conseguiu corrigir (DISM /RestoreHealth).",
  },
  VerificarDisco: {
    titulo: "Verificar o disco",
    descricao:
      "Procura erros de estrutura no disco sem consertar nada, rodando com o Windows ligado — não reinicia a máquina (chkdsk /scan).",
  },
  // Só aparece depois de "Verificar o disco" achar algo (`EstadoDoDisco`): a descrição antiga afirmava uma
  // medição que não aconteceu.
  ConsertarDisco: {
    titulo: "Consertar a estrutura do disco",
    descricao:
      "Agenda o conserto dos erros que a verificação encontrou para a próxima vez que você ligar o computador. O conserto acontece antes de o Windows abrir.",
  },
  DesmarcarConsertoDoDisco: {
    titulo: "Desmarcar o conserto do disco",
    descricao:
      "Cancela o conserto agendado, enquanto você ainda não reiniciou. Depois do reinício não há mais o que desmarcar.",
  },
  AnalisarWinSxS: {
    titulo: "Analisar componentes do Windows (WinSxS)",
    descricao:
      "Mede quanto espaço as versões antigas de componentes do Windows estão ocupando, sem apagar nada (DISM /AnalyzeComponentStore).",
  },
  LimparWinSxS: {
    titulo: "Limpar componentes antigos do Windows",
    descricao:
      "Remove versões antigas de componentes que o Windows já não usa mais (DISM /StartComponentCleanup).",
  },
};

/** O interruptor do `/ResetBase` só com `oferece_reset_base`, DESLIGADO e com o aviso ao lado. */
function desenharItemReparo(f: FerramentaDeReparo): string {
  const texto = TEXTOS_REPARO[f.nome];
  const titulo = texto?.titulo ?? f.nome;
  const descricao = texto?.descricao
    ? `<p class="reparo-item-descricao">${escapeHtml(texto.descricao)}</p>`
    : "";

  const aviso = f.aviso
    ? `<p class="reparo-item-aviso">${escapeHtml(f.aviso)}</p>`
    : "";

  const resetarBase = f.oferece_reset_base
    ? `
      <label class="pref reparo-resetbase">
        <input type="checkbox" id="reparo-resetar-base" />
        <span class="pref-text">
          <span class="pref-name">Também aplicar o /ResetBase</span>
          <span class="pref-note">${escapeHtml(f.aviso_reset_base ?? "")}</span>
        </span>
      </label>`
    : "";

  return `
    <article class="reparo-item">
      <div class="reparo-item-cabecalho">
        <h3>${escapeHtml(titulo)}</h3>
        <span class="reparo-item-duracao">${f.minutos_tipicos[0]}–${f.minutos_tipicos[1]} minutos, tipicamente</span>
      </div>
      ${descricao}
      ${aviso}
      ${resetarBase}
      <div class="reparo-item-rodape">
        <button class="btn" data-reparo="${escapeHtml(f.nome)}">Executar</button>
      </div>
    </article>`;
}

/** Por evento: um DISM leva de dez a trinta minutos, e no minuto oito parado em 20% o cliente desliga a máquina. */
/**
 * Teto de nós de DOM: cada `\r` do DISM vira uma linha (ver `drenar` em `tarefa_longa.rs`), e sem teto seriam
 * milhares. Corta do INÍCIO: o resultado está no fim.
 */
const MAX_LINHAS_SAIDA = 500;

async function carregarReparo() {
  const lista = element("reparo-lista");
  const saida = element("reparo-saida");
  const cancelar = element<HTMLButtonElement>("reparo-cancelar");

  function acrescentarLinhaSaida(a: Andamento) {
    const linha = document.createElement("span");
    linha.textContent = a.linha;
    if (a.origem === "erro") {
      linha.className = "reparo-linha-erro";
    }
    saida.appendChild(linha);
    saida.appendChild(document.createTextNode("\n"));

    // Cada linha são dois nós (o `<span>` e a quebra).
    while (saida.children.length > MAX_LINHAS_SAIDA) {
      saida.removeChild(saida.firstChild!);
      if (saida.firstChild) {
        saida.removeChild(saida.firstChild);
      }
    }
  }

  const ferramentasPorNome = new Map<string, FerramentaDeReparo>();

  let rodandoAgora: FerramentaDeReparo | null = null;

  /**
   * "Nenhuma corrupção" é BOM, com a cor de sucesso. Só o `VerificarArquivos` atualiza esta linha: repintada em
   * toda execução, um DISM que falhou mostrava em verde o veredito do `sfc`.
   */
  async function atualizarUltimoResultado() {
    const resultado = await invoke<UltimoResultadoReparo>("reparo_ultimo_resultado");
    setStatus("reparo-resultado", resultado.texto, tomParaStatus(resultado.tom));
  }

  /** Depois de cada execução: "Consertar" e "Desmarcar" só existem em certos estados, decididos pelo backend. */
  async function recarregarLista() {
    try {
      const disponiveis = await invoke<FerramentaDeReparo[]>("reparo_disponivel");
      ferramentasPorNome.clear();
      disponiveis.forEach((f) => ferramentasPorNome.set(f.nome, f));

      lista.innerHTML = disponiveis.length
        ? disponiveis.map(desenharItemReparo).join("")
        : '<p class="hint">Nenhuma ferramenta de reparo disponível nesta máquina.</p>';

      text("reparo-tag", disponiveis.length ? "pronto" : "indisponível");
    } catch {
      lista.innerHTML =
        '<p class="hint">Não consegui ler as ferramentas de reparo disponíveis.</p>';
      text("reparo-tag", "falhou");
    }
  }

  function definirRodando(f: FerramentaDeReparo | null) {
    rodandoAgora = f;

    cancelar.hidden = f === null;
    lista.querySelectorAll<HTMLButtonElement>("[data-reparo]").forEach((botao) => {
      botao.disabled = f !== null;
    });
    text("reparo-tag", f !== null ? "rodando…" : "pronto");
  }

  async function executarFerramenta(nome: string) {
    const f = ferramentasPorNome.get(nome);
    const titulo = TEXTOS_REPARO[nome]?.titulo ?? nome;

    // Exigem administrador: sem isto vinha "Terminou com o código 1" sem oferta de reabrir.
    if (!isElevated) {
      askForAdmin(
        `As ferramentas de reparo do Windows só rodam com permissão de administrador. ` +
          `Podemos reabrir o Otimiza com essa permissão?`
      );
      return;
    }

    const campoResetarBase = f?.oferece_reset_base
      ? document.getElementById("reparo-resetar-base") as HTMLInputElement | null
      : null;
    const resetarBase = campoResetarBase?.checked ?? false;

    saida.hidden = true;
    saida.textContent = "";
    definirRodando(f ?? null);
    setStatus("reparo-execucao", `Rodando: ${titulo}…`, "progress");

    try {
      // `resetbase`, uma palavra: sem depender da conversão de caixa para um interruptor desta consequência.
      const desfecho = await invoke<DesfechoReparo>("reparo_executar", {
        ferramenta: nome,
        resetbase: resetarBase,
      });

      setStatus("reparo-execucao", desfecho.texto, tomParaStatus(desfecho.tom));
    } catch (error) {
      setStatus("reparo-execucao", String(error), "error");
    } finally {
      definirRodando(null);

      // Só o `sfc` escreve este veredito (ver `atualizarUltimoResultado`).
      if (nome === "VerificarArquivos") {
        await atualizarUltimoResultado();
      }

      await recarregarLista();
    }
  }

  lista.addEventListener("click", (evento) => {
    const botao = (evento.target as HTMLElement).closest<HTMLButtonElement>(
      "button[data-reparo]"
    );
    if (!botao || botao.disabled) return;

    void executarFerramenta(botao.dataset.reparo!);
  });

  await listen<Andamento>("reparo-andamento", (evento) => {
    saida.hidden = false;
    acrescentarLinhaSaida(evento.payload);
    saida.scrollTop = saida.scrollHeight;
  });

  cancelar.addEventListener("click", () => {
    // `cancelar_e_seguro` não era lido: o clique cancelava sem perguntar até no DISM.
    if (rodandoAgora && !rodandoAgora.cancelar_e_seguro) {
      const titulo = TEXTOS_REPARO[rodandoAgora.nome]?.titulo ?? rodandoAgora.nome;
      const ok = window.confirm(
        `Interromper "${titulo}" no meio não é de graça.\n\n` +
          `Uma escrita cortada pela metade pode deixar uma operação pendente, que só ` +
          `se resolve rodando esta mesma ferramenta de novo até o fim.\n\n` +
          `Interromper mesmo assim?`
      );
      if (!ok) return;
    }

    void invoke("reparo_cancelar");
  });

  await recarregarLista();
  await atualizarUltimoResultado();
}

interface MonitorLido {
  dispositivo: string;
  descricao: string;
  principal: boolean;
  largura: number;
  altura: number;
  hz_atual: number;
  hz_disponiveis: number[];
}

/** Com dois monitores a frase nunca dizia QUAL: desenhados lado a lado, sim. */
async function carregarMonitores() {
  const lista = element("monitores-lista");

  try {
    const monitores = await invoke<MonitorLido[]>("monitores");

    if (monitores.length === 0) {
      lista.innerHTML = '<p class="hint">Não consegui ler nenhum monitor.</p>';
      return;
    }

    const abaixo = monitores.filter(
      (m) => Math.max(...m.hz_disponiveis, 0) > m.hz_atual,
    ).length;

    text(
      "monitores-tag",
      abaixo > 0
        ? `${abaixo} abaixo do máximo`
        : `${monitores.length} no máximo`,
    );

    lista.innerHTML = monitores
      .map((m) => {
        const maximo = Math.max(...m.hz_disponiveis, m.hz_atual);
        const estaAbaixo = maximo > m.hz_atual;

        return `
          <div class="monitor" data-abaixo="${estaAbaixo}">
            <svg viewBox="0 0 200 150" aria-hidden="true">
              <rect class="monitor-moldura" x="6" y="6" width="188" height="112" rx="7" />
              <rect class="monitor-tela" x="14" y="14" width="172" height="96" rx="3" />
              <text class="monitor-hz" x="100" y="66">${m.hz_atual}</text>
              <text class="vivo-unidade" x="100" y="82">HZ</text>
              <rect class="monitor-pe" x="88" y="118" width="24" height="16" rx="2" />
              <rect class="monitor-base" x="62" y="134" width="76" height="8" rx="4" />
            </svg>
            <div>
              <p class="vivo-nome">${escapeHtml(m.descricao)}${m.principal ? " · principal" : ""}</p>
              <p class="monitor-detalhe">${m.largura}×${m.altura}${
                estaAbaixo ? ` · aceita ${maximo} Hz` : " · no máximo"
              }</p>
            </div>
          </div>`;
      })
      .join("");

    const principal = monitores.find((m) => m.principal) ?? monitores[0];
    sugerirLimiteParaVrr(principal.hz_atual);
  } catch {
    lista.innerHTML = '<p class="hint">Não consegui ler os monitores.</p>';
  }
}

/** hz − hz²/3600 (144 → 138, 165 → 157, 240 → 224): acima da taxa o VRR vira V-Sync comum. */
function limiteParaVrr(hz: number): number {
  return Math.round(hz - (hz * hz) / 3600);
}

/** SÓ OFERECE: o Otimiza nunca limita sozinho, e sem G-Sync ou FreeSync o limite só tira FPS. */
function sugerirLimiteParaVrr(hz: number) {
  if (!Number.isFinite(hz) || hz < 100) return;
  const alvo = limiteParaVrr(hz);
  const grupo = element("nvlimite-fps");
  grupo.querySelector("[data-vrr]")?.remove();

  const botao = document.createElement("button");
  botao.className = "filter";
  botao.dataset.fps = String(alvo);
  botao.dataset.vrr = "sim";
  botao.textContent = `${alvo} · G-Sync`;
  grupo.prepend(botao);

  text(
    "nvlimite-vrr",
    `Com G-Sync ou FreeSync ligado, o limite certo neste monitor de ${hz} Hz é ${alvo}: acima da taxa do monitor o G-Sync vira V-Sync comum, com atraso. Sem G-Sync ou FreeSync, não use.`,
  );
  element("nvlimite-vrr").hidden = false;
  marcarLimiteEscolhido();
}

interface MemoriaInstalada {
  slots: number | null;
  pentes_gb: number[];
  canais: number;
  mhz: number | null;
}

/** "Canal único" é jargão; quatro encaixes com um ocupado não precisam de tradução. A quantidade vem da máquina. */
function desenharMemoria(m: MemoriaInstalada) {
  const total = m.slots ?? Math.max(m.pentes_gb.length, 1);
  const grupo = element("memoria-slots");

  const margem = 16;
  const vao = 8;
  const largura = (268 - (total - 1) * vao) / total;

  grupo.innerHTML = "";

  for (let i = 0; i < total; i += 1) {
    const x = margem + i * (largura + vao);
    const gb = m.pentes_gb[i];
    const ocupado = gb !== undefined;

    const partes: string[] = [];

    if (ocupado) {
      partes.push(`<rect class="memoria-pente" x="${x}" y="18" width="${largura}" height="66" rx="2" />`);

      // Quatro chips por lado: a silhueta de um pente, não a contagem real.
      for (let c = 0; c < 4; c += 1) {
        const cw = (largura - 10) / 4 - 2;
        partes.push(
          `<rect class="memoria-chip" x="${x + 5 + c * (cw + 2)}" y="${34 + (c % 2 === 0 ? 0 : 0)}" width="${cw}" height="14" rx="1" />`,
        );
      }

      partes.push(`<rect class="memoria-contato" x="${x + 3}" y="80" width="${largura - 6}" height="4" rx="1" />`);
      partes.push(
        `<text class="memoria-gb" x="${x + largura / 2}" y="28">${gb.toFixed(0)} GB</text>`,
      );
    } else {
      partes.push(`<rect class="memoria-slot-vazio" x="${x}" y="30" width="${largura}" height="54" rx="2" />`);
    }

    partes.push(`<rect class="memoria-trava" x="${x - 3}" y="26" width="5" height="12" rx="1.5" />`);
    partes.push(`<rect class="memoria-trava" x="${x + largura - 2}" y="26" width="5" height="12" rx="1.5" />`);

    grupo.insertAdjacentHTML("beforeend", partes.join(""));
  }
}

async function carregarMemoria() {
  const painel = element("memoria-painel");

  try {
    const m = await invoke<MemoriaInstalada>("memoria_instalada");

    desenharMemoria(m);

    const totalGb = m.pentes_gb.reduce((soma, gb) => soma + gb, 0);
    const slots = m.slots ?? m.pentes_gb.length;
    const livres = Math.max(0, slots - m.pentes_gb.length);

    text("memoria-total", totalGb > 0 ? `${totalGb.toFixed(0)} GB` : "não sei dizer");
    text("memoria-usados", slots > 0 ? `${m.pentes_gb.length} de ${slots}` : "—");
    text("memoria-mhz", m.mhz ? `${m.mhz} MHz` : "não sei dizer");
    text("memoria-tag", m.canais > 1 ? `${m.canais} canais` : "canal único");

    // Só canal único COM encaixe livre muda de cor: um pente numa placa de um slot é o máximo dela.
    const canalUnico = m.canais <= 1 && livres > 0;
    painel.dataset.estado = canalUnico ? "canal-unico" : "ok";

    text(
      "memoria-frase",
      canalUnico
        ? `Um pente só, e ${livres} encaixe(s) livre(s). A memória trabalha em metade da `
          + `largura que a placa aceita — acrescentar um segundo pente igual devolve a outra metade.`
        : m.pentes_gb.length === 0
          ? "Não consegui ler os pentes de memória desta máquina."
          : `${m.pentes_gb.length} pentes em ${m.canais} canal(is). A memória está trabalhando na largura cheia.`,
    );
  } catch {
    painel.dataset.estado = "ok";
    text("memoria-frase", "Não consegui ler a memória instalada.");
  }
}

interface PlacaDeVideo {
  marca: string;
  nome: string | null;
  driver: string | null;
  driver_data: string | null;
  driver_origem?: string | null;
  driver_generico?: boolean;
  driver_dias: number | null;
  vram_gb: number;
}

/** Desenha a placa lida em vez de perguntar; a escolha manual só aparece quando a leitura falha. */
type Preferencia = "Automatica" | "Economia" | "Desempenho";

interface GpuPrefReport {
  placas: string[];
  tem_placa_dupla: boolean;
  definidos: [string, Preferencia][];
  erro_de_leitura: string | null;
}

function nomeDoExecutavel(caminho: string): string {
  return caminho.split(/[\\/]/).pop() || caminho;
}

/** O maior ganho de FPS do produto. Escondido fora de máquinas com duas placas. */
async function carregarPreferenciaDeGpu() {
  const painel = element("gpupref-painel");

  let relatorio: GpuPrefReport;

  try {
    relatorio = await invoke<GpuPrefReport>("analyze_gpu_preference");
  } catch {
    // Falha de leitura não vira painel de erro numa aba que a pessoa nem pediu.
    painel.hidden = true;
    return;
  }

  if (!relatorio.tem_placa_dupla) {
    painel.hidden = true;
    return;
  }

  painel.hidden = false;
  text("gpupref-tag", `${relatorio.placas.length} placas`);

  const naEconomia = relatorio.definidos.filter(([, p]) => p === "Economia");
  const lista = element("gpupref-lista");
  const rodape = element("gpupref-rodape");

  if (naEconomia.length > 0) {
    text(
      "gpupref-nota",
      `${naEconomia.length} programa(s) estão fixados na placa que gasta menos. ` +
        "Num PC com duas placas, jogo rodando na fraca entrega uma fração do que a " +
        "máquina consegue — e é invisível para quem joga: o jogo abre normalmente e " +
        "só roda mal."
    );
  } else if (relatorio.definidos.length > 0) {
    text(
      "gpupref-nota",
      "Nenhum jogo está preso à placa mais fraca. Os que têm preferência gravada " +
        "estão abaixo, e dá para mudar qualquer um."
    );
  } else if (relatorio.erro_de_leitura) {
    text(
      "gpupref-nota",
      "Não consegui ler as preferências de placa gravadas neste computador, então " +
        "não dá para dizer se algum jogo está preso à placa mais fraca."
    );
  } else {
    text(
      "gpupref-nota",
      "Nenhum programa tem placa fixada neste computador: o Windows está escolhendo " +
        "sozinho para todos. Não dá para saber daqui se ele está acertando — se um " +
        "jogo específico estiver rodando mal, fixar a placa de desempenho para ele é " +
        "o teste mais barato que existe."
    );
  }

  lista.innerHTML = relatorio.definidos
    .map(([caminho, preferencia], indice) => {
      const fraca = preferencia === "Economia";

      const rotulo =
        preferencia === "Economia"
          ? "na placa de economia"
          : preferencia === "Desempenho"
            ? "na placa de desempenho"
            : "o Windows decide";

      // Botão só onde muda algo.
      const botao = fraca
        ? `<button class="btn btn-small" data-gpupref="${indice}">Passar para a placa de desempenho</button>`
        : "";

      return `
        <div class="gpupref-linha" data-fraca="${fraca}">
          <div class="gpupref-jogo">
            <span class="gpupref-nome">${escapeHtml(nomeDoExecutavel(caminho))}</span>
            <span class="gpupref-estado">${rotulo}</span>
          </div>
          ${botao}
        </div>`;
    })
    .join("");

  rodape.hidden = relatorio.definidos.length === 0;
  rodape.textContent =
    "A mudança vale na próxima vez que o jogo abrir, não exige administrador e " +
    "não reinicia o PC. Como qualquer mudança do Otimiza, ela entra no histórico " +
    "e o Desfazer devolve o valor exato de antes.";

  lista.querySelectorAll<HTMLButtonElement>("[data-gpupref]").forEach((botao) => {
    const indice = Number(botao.dataset.gpupref);
    const entrada = relatorio.definidos[indice];
    if (!entrada) return;

    botao.onclick = async () => {
      botao.disabled = true;
      botao.textContent = "Mudando…";

      try {
        const resultado = await invoke<{ success: boolean; message: string }>(
          "set_gpu_preference",
          { caminho: entrada[0], desempenho: true }
        );

        text("gpupref-nota", resultado.message);

        // Relê do sistema: se a escrita não pegou, a lista mostra o estado real.
        await carregarPreferenciaDeGpu();
      } catch (error) {
        text("gpupref-nota", String(error));
        botao.disabled = false;
        botao.textContent = "Passar para a placa de desempenho";
      }
    };
  });
}

type EstadoDaNvapi = "Disponivel" | "SemPlacaNvidia" | "NaoCarregou";

interface AjusteDoDriver {
  id: string;
  titulo: string;
  explicacao: string;
  historico: string;
  aplicado: boolean;
}

interface LimiteDoDriver {
  executavel: string;
  fps: number;
  historico: string;
}

interface PainelDoDriver {
  estado: EstadoDaNvapi;
  nota: string;
  ajustes: AjusteDoDriver[];
  limites: LimiteDoDriver[];
}

/** 60 de partida: todo monitor mostra 60. */
let limiteDeFpsEscolhido = 60;

function marcarLimiteEscolhido() {
  document.querySelectorAll<HTMLButtonElement>("#nvlimite-fps [data-fps]").forEach((botao) => {
    botao.setAttribute("aria-pressed", String(Number(botao.dataset.fps) === limiteDeFpsEscolhido));
  });
}

function desenharLimitesDoDriver(limites: LimiteDoDriver[]) {
  const lista = element("nvlimite-lista");

  lista.innerHTML = limites
    .map(
      (limite, indice) => `
        <div class="gpupref-linha" data-fraca="false">
          <div class="gpupref-jogo">
            <span class="gpupref-nome">${escapeHtml(limite.executavel)}</span>
            <span class="gpupref-estado">limitado a ${limite.fps} FPS</span>
          </div>
          <button class="btn btn-small" data-nvlimite="${indice}">Desfazer</button>
        </div>`
    )
    .join("");

  lista.querySelectorAll<HTMLButtonElement>("[data-nvlimite]").forEach((botao) => {
    const limite = limites[Number(botao.dataset.nvlimite)];
    if (!limite) return;

    botao.onclick = async () => {
      if (!isElevated) {
        askForAdmin(
          "O driver da NVIDIA só salva ajustes com permissão de administrador. " +
            "Podemos reabrir o Otimiza com essa permissão?"
        );
        return;
      }

      botao.disabled = true;
      botao.textContent = "Desfazendo…";

      try {
        const resultado = await invoke<OptimizationOutcome>("revert_optimization", {
          id: limite.historico,
        });
        text("nvlimite-nota", resultado.message);
        await carregarAjustesDoDriver();
      } catch (error) {
        text("nvlimite-nota", String(error));
        botao.disabled = false;
        botao.textContent = "Desfazer";
      }
    };
  });
}

async function limitarJogoAberto() {
  const botao = element<HTMLButtonElement>("nvlimite-aplicar");

  let executavel: string | null = null;
  try {
    executavel = await invoke<string | null>("running_game_executable");
  } catch {
    executavel = null;
  }

  if (!executavel) {
    text(
      "nvlimite-nota",
      "Não encontrei jogo aberto. Abra o jogo, volte aqui e clique em Limitar: o limite " +
        "vai no perfil do executável que estiver rodando."
    );
    return;
  }

  if (!isElevated) {
    askForAdmin(
      "O driver da NVIDIA só salva ajustes com permissão de administrador. " +
        "Podemos reabrir o Otimiza com essa permissão?"
    );
    return;
  }

  botao.disabled = true;
  botao.textContent = "Limitando…";

  try {
    const resultado = await invoke<OptimizationOutcome>("limitar_fps_nvidia", {
      executavel,
      fps: limiteDeFpsEscolhido,
    });
    text("nvlimite-nota", resultado.message);
    await carregarAjustesDoDriver();
  } catch (error) {
    text("nvlimite-nota", String(error));
  } finally {
    botao.disabled = false;
    botao.textContent = "Limitar o jogo aberto";
  }
}

const NA_TAG_DA_NVAPI: Record<EstadoDaNvapi, string> = {
  Disponivel: "driver encontrado",
  SemPlacaNvidia: "sem placa NVIDIA",
  NaoCarregou: "driver sem resposta",
};

/** Só com o driver respondendo: botão que o driver recusaria é teatro. */
async function carregarAjustesDoDriver() {
  const painel = element("nvdriver-painel");
  const lista = element("nvdriver-lista");
  const rodape = element("nvdriver-rodape");

  let dados: PainelDoDriver;

  try {
    dados = await invoke<PainelDoDriver>("ajustes_do_driver_nvidia");
  } catch (error) {
    painel.hidden = false;
    text("nvdriver-tag", "—");
    text("nvdriver-nota", String(error));
    lista.innerHTML = "";
    rodape.hidden = true;
    element("nvlimite").hidden = true;
    return;
  }

  painel.hidden = false;
  text("nvdriver-tag", NA_TAG_DA_NVAPI[dados.estado]);
  text("nvdriver-nota", dados.nota);

  const disponivel = dados.estado === "Disponivel";
  rodape.hidden = !disponivel;
  element("nvlimite").hidden = !disponivel;

  if (!disponivel) {
    lista.innerHTML = "";
    desenharPerfisDaPlaca(dados);
    return;
  }

  desenharLimitesDoDriver(dados.limites);
  desenharPerfisDaPlaca(dados);

  lista.innerHTML = dados.ajustes
    .map(
      (ajuste, indice) => `
        <div class="gpupref-linha" data-fraca="false">
          <div class="gpupref-jogo">
            <span class="gpupref-nome">${escapeHtml(ajuste.titulo)}</span>
            <span class="gpupref-estado">${escapeHtml(ajuste.explicacao)}</span>
          </div>
          <button class="btn btn-small" data-nvajuste="${indice}">
            ${ajuste.aplicado ? "Desfazer" : "Aplicar"}
          </button>
        </div>`
    )
    .join("");

  lista.querySelectorAll<HTMLButtonElement>("[data-nvajuste]").forEach((botao) => {
    const ajuste = dados.ajustes[Number(botao.dataset.nvajuste)];
    if (!ajuste) return;

    botao.onclick = async () => {
      // O driver só salva elevado.
      if (!isElevated) {
        askForAdmin(
          "O driver da NVIDIA só salva ajustes com permissão de administrador. " +
            "Podemos reabrir o Otimiza com essa permissão?"
        );
        return;
      }

      const rotulo = ajuste.aplicado ? "Desfazer" : "Aplicar";
      botao.disabled = true;
      botao.textContent = ajuste.aplicado ? "Desfazendo…" : "Aplicando…";

      try {
        const resultado = ajuste.aplicado
          ? await invoke<OptimizationOutcome>("revert_optimization", { id: ajuste.historico })
          : await invoke<OptimizationOutcome>("aplicar_ajuste_nvidia", { opcao: ajuste.id });

        text("nvdriver-nota", resultado.message);
        await carregarAjustesDoDriver();
      } catch (error) {
        text("nvdriver-nota", String(error));
        botao.disabled = false;
        botao.textContent = rotulo;
      }
    };
  });
}

/**
 * TRÊS perfis, combinações dos seis ajustes (sete nomes para três respostas seria vender nomes). A tela MOSTRA os
 * ids de cada um.
 */
const PERFIS_DA_PLACA: {
  id: string;
  nome: string;
  resumo: string;
  ajustes: string[];
}[] = [
  {
    id: "equilibrado",
    nome: "Equilibrado",
    resumo:
      "O que quase toda máquina ganha sem trocar nada de lugar: a placa para de baixar o clock entre quadros e o cache de shader fica ligado e sem limite de tamanho.",
    ajustes: ["energia", "cache_shader", "cache_shader_tamanho"],
  },
  {
    id: "competitivo",
    nome: "Competitivo",
    resumo:
      "O de cima, mais a fila de quadros curta e a sincronia vertical desligada. Troca suavidade por resposta — e pode aparecer rasgo na imagem.",
    ajustes: ["energia", "cache_shader", "cache_shader_tamanho", "latencia", "vsync"],
  },
  {
    id: "maximo",
    nome: "Tudo que há",
    resumo:
      "Os seis ajustes. Inclui a filtragem de textura em desempenho, que é o único deles que muda como o jogo se parece.",
    ajustes: ["energia", "cache_shader", "cache_shader_tamanho", "latencia", "vsync", "textura"],
  },
];

/** "Aplicado" é TODOS os ajustes ligados, lido do histórico: guardar a escolha seria uma segunda verdade. */
function desenharPerfisDaPlaca(dados: PainelDoDriver) {
  const disponivel = dados.estado === "Disponivel";
  const ligados = new Set(dados.ajustes.filter((a) => a.aplicado).map((a) => a.id));

  element("gpu-perfis").innerHTML = PERFIS_DA_PLACA.map((perfil) => {
    const inteiro = perfil.ajustes.every((id) => ligados.has(id));
    const quantos = perfil.ajustes.filter((id) => ligados.has(id)).length;

    const nomes = perfil.ajustes
      .map((id) => dados.ajustes.find((a) => a.id === id)?.titulo ?? id)
      .map((t) => `<li>${escapeHtml(t)}</li>`)
      .join("");

    return `
      <article class="gpu-perfil" data-inteiro="${inteiro}">
        <div class="gpu-perfil-topo">
          <span class="gpu-perfil-nome">${escapeHtml(perfil.nome)}</span>
          <span class="gpu-perfil-conta">${quantos} de ${perfil.ajustes.length} ligados</span>
        </div>
        <p class="gpu-perfil-resumo">${escapeHtml(perfil.resumo)}</p>
        <details class="gpu-perfil-detalhe">
          <summary>O que ele liga</summary>
          <ul>${nomes}</ul>
        </details>
        <button class="btn" data-perfil="${perfil.id}" ${
          disponivel && !inteiro ? "" : "disabled"
        }>${inteiro ? "Já está aplicado" : "Aplicar os que faltam"}</button>
      </article>`;
  }).join("");

  for (const botao of element("gpu-perfis").querySelectorAll<HTMLButtonElement>("[data-perfil]")) {
    botao.onclick = () => void aplicarPerfilDaPlaca(botao, dados);
  }
}

async function aplicarPerfilDaPlaca(botao: HTMLButtonElement, dados: PainelDoDriver) {
  const perfil = PERFIS_DA_PLACA.find((p) => p.id === botao.dataset.perfil);
  if (!perfil) return;

  if (!isElevated) {
    askForAdmin(
      "O driver da NVIDIA só salva ajustes com permissão de administrador. " +
        "Podemos reabrir o Otimiza com essa permissão?"
    );
    return;
  }

  const ligados = new Set(dados.ajustes.filter((a) => a.aplicado).map((a) => a.id));
  const faltam = perfil.ajustes.filter((id) => !ligados.has(id));

  botao.disabled = true;
  setStatus("gpu-perfis-status", `Aplicando ${faltam.length} ajuste(s)…`, "progress");

  // Um de cada vez, parando no primeiro que falhar: metade de um perfil não é nada.
  for (const id of faltam) {
    try {
      await invoke<OptimizationOutcome>("aplicar_ajuste_nvidia", { opcao: id });
    } catch (error) {
      setStatus("gpu-perfis-status", String(error), "error");
      await carregarAjustesDoDriver();
      return;
    }
  }

  setStatus(
    "gpu-perfis-status",
    `Pronto. Cada um entrou no histórico e pode ser desfeito sozinho.`,
    "ok"
  );
  await carregarAjustesDoDriver();
}

async function carregarCartaoDaPlaca() {
  try {
    const p = await invoke<PlacaDeVideo>("placa_de_video");

    text("gpu-cartao-nome", p.nome ?? "Placa não identificada");
    text(
      "gpu-cartao-nota",
      p.driver ? `driver ${p.driver}` : "versão do driver não lida"
    );
    element("gpu-cartao").dataset.marca = p.marca;
  } catch {
    text("gpu-cartao-nome", "Placa não identificada");
    text("gpu-cartao-nota", "a leitura da placa falhou");
  }
}

async function carregarPlaca() {
  const painel = element("placa-painel");

  try {
    const p = await invoke<PlacaDeVideo>("placa_de_video");

    painel.dataset.marca = p.marca;
    text("placa-marca", p.marca === "desconhecida" ? "placa de vídeo" : p.marca);
    text("placa-nome", p.nome ?? "Não consegui identificar a placa");
    text("placa-driver", p.driver ?? "—");

    // A idade com a data: o cliente confere no Gerenciador de Dispositivos.
    text(
      "placa-driver-idade",
      p.driver_data
        ? `${p.driver_data}${p.driver_dias !== null ? ` · há ${p.driver_dias} dias` : ""}`
        : "—",
    );

    text("placa-vram", p.vram_gb > 0 ? `${p.vram_gb.toFixed(0)} GB` : "não sei dizer");

    text("placa-driver-origem", p.driver_origem ?? "não deu para ler");
    text(
      "placa-driver-nota",
      p.driver_generico
        ? "Este é o driver genérico do Windows: a placa roda sem a aceleração completa. Instalar o driver do fabricante muda o FPS de verdade."
        : "Driver mais novo não é automaticamente mais rápido: às vezes melhora um jogo e piora outro. Se um jogo caiu depois de atualizar, volte ao driver anterior e compare as partidas medidas.",
    );
    const sites: Record<string, string> = {
      nvidia: "https://www.nvidia.com/pt-br/drivers/",
      amd: "https://www.amd.com/pt/support/download/drivers.html",
      intel: "https://www.intel.com.br/content/www/br/pt/download-center/home.html",
    };
    const site = sites[p.marca];
    const botaoSite = document.getElementById("placa-driver-site") as HTMLButtonElement | null;
    if (botaoSite) {
      botaoSite.hidden = !site;
      botaoSite.onclick = site ? () => void openUrl(site) : null;
    }

    element("placa-escolha").hidden = p.marca !== "desconhecida";
  } catch {
    painel.dataset.marca = "desconhecida";
    text("placa-nome", "Não consegui ler a placa de vídeo");
    element("placa-escolha").hidden = false;
  }
}

/*
 * A configuração do jogo e a prova: a configuração vale dezenas de por cento, a memória é o teto, os ajustes de
 * Windows somados alguns por cento. O primeiro painel mexe onde o ganho mora; o segundo prova.
 */

interface AjusteCaro {
  chave: string;
  valor: string;
  onde: string;
  ganho: string;
}

interface ConfigJogoReport {
  arquivo: string | null;
  jogo: string;
  caros: AjusteCaro[];
  findings: Achado[];
}

type MudancaPrevista = [string, string, string, string];

interface Prova {
  jogo: string;
  quando: number;
  fps: number;
  low_1pct: number;
  engasgos_por_minuto: number;
  segundos: number;
  confiavel: boolean;
}

interface ComparacaoDaProva {
  antes: Prova;
  depois: Prova;
  fps_delta: number;
  fps_pct: number;
  low_delta: number;
  low_pct: number;
  engasgos_delta: number;
  veredito: string;
  ressalvas: string[];
  vale_como_prova: boolean;
}

const PERFIS: Record<string, { botao: string; nome: string; aviso: string }> = {
  sem_teto: {
    botao: "cfgjogo-sem-teto",
    nome: "Tirar o limite de FPS",
    aviso: "",
  },
  equilibrado: {
    botao: "cfgjogo-equilibrado",
    nome: "Equilibrado",
    aviso: "Isto muda como o jogo se parece.",
  },
  competitivo: {
    botao: "cfgjogo-competitivo",
    nome: "Máximo de FPS",
    aviso: "Isto muda bastante como o jogo se parece.",
  },
};

async function analisarConfigJogo() {
  const botao = element<HTMLButtonElement>("cfgjogo-analisar");
  botao.disabled = true;
  setStatus("cfgjogo-status", "Lendo a configuração do jogo…", "progress");

  try {
    const r = await invoke<ConfigJogoReport>("analyze_game_config");
    renderConfigJogo(r);
    setStatus(
      "cfgjogo-status",
      r.caros.length
        ? `${r.caros.length} ajuste(s) pesando na sua placa.`
        : "Nada de caro ficou ligado.",
      r.caros.length ? "warn" : "ok",
    );
  } catch (error) {
    setStatus("cfgjogo-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

function renderConfigJogo(r: ConfigJogoReport) {
  if (!r.arquivo) {
    text("cfgjogo-tag", "nenhum jogo encontrado");
    element("cfgjogo-result").innerHTML =
      '<p class="hint">Não encontrei a configuração de nenhum jogo conhecido neste computador.</p>';
    return;
  }

  text("cfgjogo-tag", r.jogo);

  // O diagnóstico ranqueia por custo real: o MSAA primeiro.
  const caros = r.caros.length
    ? `<ul class="lista">${r.caros
        .map(
          (c) =>
            `<li><strong>${escapeHtml(c.chave)}</strong> em ${escapeHtml(
              c.valor
            )} · custa ${escapeHtml(c.ganho)} dos quadros</li>`
        )
        .join("")}</ul>`
    : '<p class="hint">Nada de caro ficou ligado nesta configuração.</p>';

  element("cfgjogo-result").innerHTML =
    `<p class="hint">Arquivo: <code>${escapeHtml(r.arquivo)}</code></p>${caros}`;
}

interface PlanoRenderizacao {
  plano: {
    decisao: "SemEvidencia" | "NaoResolveAqui" | { Aplicar: string };
    porque: string[];
    contra: string[];
    nao_verificado: string[];
    exige_baseline: boolean;
  };
  perfil_para_aplicar: string | null;
  jogo: string;
}

async function planoDeRenderizacao() {
  const botao = element<HTMLButtonElement>("cfgjogo-plano");
  botao.disabled = true;
  setStatus("cfgjogo-status", "Medindo e cruzando com a configuração do jogo…", "progress");

  try {
    const r = await invoke<PlanoRenderizacao>("plano_de_renderizacao");
    renderPlanoDeRenderizacao(r);
    setStatus("cfgjogo-status", "", "ok");
  } catch (error) {
    setStatus("cfgjogo-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

function renderPlanoDeRenderizacao(r: PlanoRenderizacao) {
  // Senão um plano anterior deixaria dois perfis marcados.
  for (const meta of Object.values(PERFIS)) {
    delete element(meta.botao).dataset.recomendado;
  }

  const escolhido = r.perfil_para_aplicar;
  if (escolhido && PERFIS[escolhido]) {
    element(PERFIS[escolhido].botao).dataset.recomendado = "sim";
  }

  const titulo = escolhido
    ? `O seu caso pede: ${PERFIS[escolhido].nome}`
    : r.plano.decisao === "SemEvidencia"
      ? "Ainda não dá para dizer"
      : "Nenhum destes resolve o seu caso";

  const lista = (itens: string[], rotulo: string, classe: string) =>
    itens.length === 0
      ? ""
      : `<p class="${classe}"><strong>${rotulo}</strong> ${escapeHtml(itens.join(" "))}</p>`;

  // Só quando há o que aplicar: é aí que não medir antes custa a resposta para "melhorou?".
  const base = r.plano.exige_baseline
    ? `<p class="finding-advice">Guarde a linha de base desta máquina ANTES de aplicar. Sem o retrato de antes, não há como responder depois se melhorou — e quanto.</p>`
    : "";

  element("cfgjogo-plano-result").innerHTML = `
    <article class="finding" data-severity="${escolhido ? "Important" : "Ok"}" style="--i:0">
      <div class="finding-top">
        <h3>${escapeHtml(titulo)}</h3>
      </div>
      ${lista(r.plano.porque, "Porque", "finding-measured")}
      ${lista(r.plano.contra, "O que isto não resolve:", "hint")}
      ${base}
      ${lista(r.plano.nao_verificado, "Não foi possível verificar:", "hint")}
    </article>`;
}

/** Só aplica depois do "sim", sobre linhas com nome e número. */
async function aplicarPerfilDoJogo(perfil: string) {
  const meta = PERFIS[perfil];
  const botao = element<HTMLButtonElement>(meta.botao);

  botao.disabled = true;
  setStatus("cfgjogo-status", "Vendo o que mudaria…", "progress");

  let previsto: MudancaPrevista[];

  try {
    previsto = await invoke<MudancaPrevista[]>("preview_game_profile", { perfil });
  } catch (error) {
    setStatus("cfgjogo-status", String(error), "error");
    botao.disabled = false;
    return;
  }

  if (previsto.length === 0) {
    // Nada a mudar é a melhor notícia, e não pode parecer erro.
    setStatus(
      "cfgjogo-status",
      "A configuração já está assim. Não há nada para mudar neste perfil.",
      "ok"
    );
    botao.disabled = false;
    return;
  }

  const linhas = previsto
    .map(
      ([chave, atual, novo, custo]) =>
        `${chave}: ${atual} → ${novo}${custo ? `  (${custo})` : ""}`
    )
    .join("\n");

  const perdas = previsto.filter(([, , , custo]) => custo).length;

  const confirmado = confirm(
    `${meta.nome}\n\n` +
      `${linhas}\n\n` +
      (perdas === 0
        ? "Nenhuma dessas mudanças altera como o jogo se parece.\n\n"
        : `${meta.aviso}\n\n`) +
      "O arquivo é guardado inteiro antes, e dá para desfazer a qualquer momento.\n\n" +
      "Aplicar?"
  );

  if (!confirmado) {
    setStatus("cfgjogo-status", "Nada foi alterado.", "ok");
    botao.disabled = false;
    return;
  }

  setStatus("cfgjogo-status", "Aplicando…", "progress");

  try {
    const mudou = await invoke<string[]>("apply_game_profile", { perfil });

    setStatus(
      "cfgjogo-status",
      mudou.length
        ? `Pronto: ${mudou.join(" · ")}. Abra o jogo e meça de novo abaixo.`
        : "A configuração já estava assim; nada mudou.",
      "ok"
    );

    await analisarConfigJogo();
    await loadOptimizations();
  } catch (error) {
    setStatus("cfgjogo-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

function segundosDaMedicao(): number {
  // Menos de vinte segundos não sustenta o 1% pior.
  return 20;
}

/** Não sobrescreve o que a pessoa digitou: quase sempre é um jogo que o detector não reconhece. */
async function preencherJogoDetectado(): Promise<string | null> {
  // As duas caixas (prova e contagem de quadros) numa consulta só.
  const campos = [
    element<HTMLInputElement>("prova-processo"),
    element<HTMLInputElement>("fps-process"),
  ];

  try {
    const detectado = await invoke<string | null>("running_game_executable");

    if (detectado) {
      for (const campo of campos) {
        if (!campo.value.trim()) campo.value = detectado;
      }
    }

    return detectado;
  } catch {
    return null;
  }
}

/** O fluxo atravessa um reinício: até a 1.8 a tela voltava em branco, com a medição no disco o tempo todo. */
async function restaurarProvaGuardada() {
  let guardada: Prova | null;

  try {
    guardada = await invoke<Prova | null>("prova_guardada");
  } catch {
    return;
  }

  if (!guardada) return;

  element<HTMLInputElement>("prova-processo").value = guardada.jogo;

  text("prova-tag", "antes: " + guardada.fps.toFixed(0) + " FPS");

  const quando = new Date(guardada.quando * 1000);
  const dia = quando.toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });
  const hora = quando.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  const jogo = escapeHtml(guardada.jogo);

  element("prova-result").innerHTML =
    "<p><strong>Existe uma medição guardada.</strong> " + jogo +
    ", em " + dia + " às " + hora + ".</p>" +
    "<ul class=\"lista\">" +
    "<li>Média: <strong>" + guardada.fps.toFixed(0) + " FPS</strong></li>" +
    "<li>1% piores quadros: <strong>" + guardada.low_1pct.toFixed(0) + " FPS</strong></li>" +
    "<li>Engasgos: <strong>" + guardada.engasgos_por_minuto.toFixed(0) + " por minuto</strong></li>" +
    "</ul>" +
    "<p class=\"hint\">Abra <strong>" + jogo + "</strong> no mesmo lugar de antes e clique " +
    "em \"Medir de novo\" para comparar. Medir o \"antes\" outra vez substitui esta.</p>";
}

interface MedicaoAutomatica {
  jogo: string;
  quando: number;
  fps: number;
  low_1pct: number;
  engasgos_por_minuto: number;
  segundos: number;
  confiavel: boolean;
  mudancas_aplicadas: number;
}

/** LADO A LADO, SEM CONCLUSÃO: medições de lugares diferentes do jogo. */

/** É a tela que acusa o próprio produto: o Rust manda estado e números. */
interface LadoDaMedicao {
  fps: number;
  low_1pct: number;
  amostras: number;
}

interface VereditoDeRegressao {
  jogo: string;
  desfecho: { desfecho: "SemAmostra" | "Igual" | "Melhorou" | "Piorou" | "PiorouMuito" };
  antes: LadoDaMedicao | null;
  depois: LadoDaMedicao | null;
  variacao_fps_pct: number | null;
  variacao_low_pct: number | null;
}

/** Avisa sem esperar, mas NÃO desfaz sozinho: sessões diferentes (ver `regressao.rs`). */

/** `NaoDeuParaLer` é a terceira cor: mostra o modelo e diz que não classificou. */
interface OndeOJogoMora {
  jogo: string;
  caminho: string;
  unidade: string | null;
  midia: "Ssd" | "Mecanico" | "NaoDeuParaLer";
  modelo: string | null;
}

interface RelatorioDeDiscos {
  jogos: OndeOJogoMora[];
  lacunas: string[];
}

const NA_TELA_DO_DISCO: Record<
  OndeOJogoMora["midia"],
  { rotulo: string; severidade: string }
> = {
  Ssd: { rotulo: "em SSD", severidade: "Ok" },
  Mecanico: { rotulo: "em disco mecânico", severidade: "Important" },
  NaoDeuParaLer: { rotulo: "tipo de disco não identificado", severidade: "Neutral" },
};

/** Nem a ordem é daqui (`causas::ordenar`): ela inclui acusar o próprio Otimiza. */
interface Suspeito {
  id: string;
  titulo: string;
  medido: string;
  porque: string;
  como_confirmar: string;
  confianca: string;
}

interface Investigacao {
  suspeitos: Suspeito[];
  lacunas: string[];
}

interface NaoFazemos {
  id: string;
  nome: string;
  natureza: "Placebo" | "Redundante" | "Prejudicial";
  porque: string;
}

/** O rótulo mora aqui; o vocabulário das naturezas, no Rust. */
const NA_TELA_DA_NATUREZA: Record<
  NaoFazemos["natureza"],
  { rotulo: string; severidade: string }
> = {
  Placebo: { rotulo: "não faz nada", severidade: "Neutral" },
  Redundante: { rotulo: "o Windows já faz", severidade: "Neutral" },
  Prejudicial: { rotulo: "piora a máquina", severidade: "Important" },
};

type FaseDoGrupo =
  | { fase: "FaltaOAntes" }
  | { fase: "ProntoParaAplicar"; amostras_antes: number }
  | { fase: "EsperandoODepois"; amostras_depois: number; faltam: number }
  | { fase: "Concluido" };

type DecisaoDoGrupo =
  | { decisao: "Esperar"; falta: string }
  | { decisao: "Manter"; ganho_pct: number }
  | { decisao: "NaoMudouNada" }
  | { decisao: "ReverterSozinho"; queda_pct: number }
  | { decisao: "PiorouMasNaoReverto"; queda_pct: number; porque: string };

interface LadoDoVeredito {
  fps: number;
  low_1pct: number;
  amostras: number;
}

interface Experimento {
  grupo: string;
  letra: string;
  nome: string;
  descricao: string;
  itens: string[];
  exige_reinicio: boolean;
  fase: FaseDoGrupo;
  decisao: DecisaoDoGrupo;
  veredito: {
    antes: LadoDoVeredito | null;
    depois: LadoDoVeredito | null;
    variacao_fps_pct: number | null;
  } | null;
}

/** `NaoMudouNada` é NEUTRO: "aqui não rende" deixa a pessoa parar de mexer. */
const NA_TELA_DA_DECISAO: Record<
  DecisaoDoGrupo["decisao"],
  { rotulo: string; severidade: string }
> = {
  Esperar: { rotulo: "ainda medindo", severidade: "Neutral" },
  Manter: { rotulo: "rendeu aqui", severidade: "Ok" },
  NaoMudouNada: { rotulo: "não mudou nada aqui", severidade: "Neutral" },
  ReverterSozinho: { rotulo: "piorou — vamos desfazer", severidade: "Important" },
  PiorouMasNaoReverto: { rotulo: "piorou", severidade: "Important" },
};

function numerosDoExperimento(e: Experimento): string {
  const v = e.veredito;
  if (!v || !v.antes || !v.depois) return "";

  const pct = v.variacao_fps_pct;
  const sinal = pct !== null && pct > 0 ? "+" : "";

  return `<p class="causa-medido">
      Antes: ${v.antes.fps.toFixed(0)} FPS · 1% piores ${v.antes.low_1pct.toFixed(0)}
      (${v.antes.amostras} medições).
      Depois: ${v.depois.fps.toFixed(0)} FPS · 1% piores ${v.depois.low_1pct.toFixed(0)}
      (${v.depois.amostras} medições).
      ${pct !== null ? `Diferença: ${sinal}${pct.toFixed(0)}%.` : ""}
    </p>`;
}

function textoDaDecisao(e: Experimento): string {
  const d = e.decisao;

  switch (d.decisao) {
    case "Esperar":
      return `<p class="effect">${escapeHtml(d.falta)}</p>`;
    case "Manter":
      return `<p class="effect">Este grupo rendeu ${d.ganho_pct.toFixed(0)}% nesta máquina.
                Mantenha.</p>`;
    case "NaoMudouNada":
      return `<p class="effect">Medido dos dois lados e a diferença ficou dentro da
                variação normal entre duas partidas. Não rende aqui — e saber disso é o
                que permite parar de mexer neste grupo.</p>`;
    case "ReverterSozinho":
      return `<p class="effect">Caiu ${Math.abs(d.queda_pct).toFixed(0)}% depois deste
                grupo. Como ele não exige reiniciar, a comparação é limpa e o Otimiza
                desfaz.</p>`;
    case "PiorouMasNaoReverto":
      return `<p class="effect">Caiu ${Math.abs(d.queda_pct).toFixed(0)}% depois deste
                grupo. ${escapeHtml(d.porque)}</p>`;
  }
}

interface ConflitoDeAjuste {
  id: string;
  um: string;
  outro: string;
  tipo: "MesmoLugar" | "SeAnulam" | "DependeDoOutro" | "JuntosCustamCaro";
  mecanismo: string;
  conselho: string;
}

const NA_TELA_DO_CONFLITO: Record<ConflitoDeAjuste["tipo"], string> = {
  MesmoLugar: "escrevem no mesmo lugar",
  SeAnulam: "um anula o outro",
  DependeDoOutro: "um depende do outro",
  JuntosCustamCaro: "juntos custam caro",
};

/** Só com conflito real entre o que ESTÁ aplicado: explica um A/B que deu resposta errada. */

interface ContaDoUsuario {
  conta:
    | { estado: "Mesma"; usuario: string }
    | { estado: "Diferente"; processo: string; shell: string }
    | { estado: "NaoDeuParaLer"; motivo: string };
  ajustes_por_conta: number;
  explicacao: string;
}

/** Só com contas diferentes ou leitura falha: explica vinte e um ajustes aplicados sem efeito. */
async function carregarContaQueEstaRodando() {
  const painel = element("conta-painel");
  const alvo = element("conta-aviso");

  let c: ContaDoUsuario;

  try {
    c = await invoke<ContaDoUsuario>("conta_que_esta_rodando");
  } catch {
    painel.hidden = true;
    return;
  }

  if (c.conta.estado === "Mesma") {
    painel.hidden = true;
    return;
  }

  painel.hidden = false;
  painel.dataset.severity = c.conta.estado === "Diferente" ? "Important" : "Neutral";

  alvo.innerHTML = c.explicacao
    .split("\n\n")
    .map((p) => `<p class="effect">${escapeHtml(p)}</p>`)
    .join("");
}

async function carregarConflitosDeAjuste() {
  const painel = element("conflitos-de-ajuste-painel");
  const alvo = element("conflitos-de-ajuste");

  let conflitos: ConflitoDeAjuste[];

  try {
    conflitos = await invoke<ConflitoDeAjuste[]>("conflitos_entre_ajustes");
  } catch {
    painel.hidden = true;
    return;
  }

  if (conflitos.length === 0) {
    painel.hidden = true;
    return;
  }

  painel.hidden = false;
  alvo.innerHTML = conflitos
    .map(
      (c) => `
      <div class="causa" data-severity="Important">
        <p class="causa-titulo">
          <strong>${escapeHtml(nomeDaOtimizacao(c.um))}</strong> e
          <strong>${escapeHtml(nomeDaOtimizacao(c.outro))}</strong>
          <span class="chip">${escapeHtml(NA_TELA_DO_CONFLITO[c.tipo])}</span>
        </p>
        <p class="effect">${escapeHtml(c.mecanismo)}</p>
        <p class="causa-confirmar"><strong>O que fazer:</strong> ${escapeHtml(c.conselho)}</p>
      </div>`
    )
    .join("");
}

/** Sem a lista, o id: feio e honesto. */
function nomeDaOtimizacao(id: string): string {
  return optimizations.find((o) => o.id === id)?.name ?? id;
}

async function carregarProtocolo() {
  const alvo = element("protocolo");

  let grupos: Experimento[];

  try {
    grupos = await invoke<Experimento[]>("protocolo_de_grupos", { jogo: null });
  } catch (erro) {
    // A recusa explica o que fazer: é a primeira instrução do protocolo.
    alvo.innerHTML = `<p class="bloco-de-prosa">${escapeHtml(String(erro))}</p>`;
    return;
  }

  alvo.innerHTML = grupos
    .map((e) => {
      const naTela = NA_TELA_DA_DECISAO[e.decisao.decisao];
      const reinicio = e.exige_reinicio
        ? `<span class="chip">exige reiniciar</span>`
        : `<span class="chip">sem reiniciar</span>`;

      return `
        <div class="causa" data-severity="${naTela.severidade}">
          <p class="causa-titulo">
            <strong>${escapeHtml(e.letra)} — ${escapeHtml(e.nome)}</strong>
            <span class="chip">${escapeHtml(naTela.rotulo)}</span>
            ${reinicio}
          </p>
          <p class="effect">${escapeHtml(e.descricao)}</p>
          ${numerosDoExperimento(e)}
          ${textoDaDecisao(e)}
          <p class="detail">${e.itens.length} ajuste(s) neste grupo.</p>
        </div>`;
    })
    .join("");
}

interface AlteracaoRegistrada {
  id: string;
  titulo: string;
  modulo: string;
  o_que_muda: string;
  risco: "Baixo" | "Medio" | "Alto";
  escopo: "Windows" | "Jogo" | "Driver" | "Arquivo";
  precisa_reiniciar: boolean;
  refaz_sozinho: boolean;
  desfazer: string;
}

const NA_TELA_DO_RISCO: Record<AlteracaoRegistrada["risco"], { rotulo: string; severidade: Severity }> = {
  Baixo: { rotulo: "risco baixo", severidade: "Ok" },
  Medio: { rotulo: "risco médio", severidade: "Important" },
  Alto: { rotulo: "risco alto", severidade: "Critical" },
};

const NA_TELA_DO_ESCOPO: Record<AlteracaoRegistrada["escopo"], string> = {
  Windows: "Windows",
  Jogo: "um jogo",
  Driver: "driver de vídeo",
  Arquivo: "apaga arquivo",
};

async function carregarOQueAltera() {
  const alvo = document.getElementById("o-que-altera");
  if (!alvo) return;
  let lista: AlteracaoRegistrada[];
  try {
    lista = await invoke<AlteracaoRegistrada[]>("o_que_o_otimiza_altera");
  } catch (erro) {
    alvo.innerHTML = `<p class="status warn">${escapeHtml(String(erro))}</p>`;
    return;
  }
  const ordem = { Alto: 0, Medio: 1, Baixo: 2 } as const;
  lista.sort((a, b) => ordem[a.risco] - ordem[b.risco] || a.titulo.localeCompare(b.titulo, "pt-BR"));
  alvo.innerHTML =
    `<p class="hint">${lista.length} alterações. Nada muda no seu computador sem estar
       nesta lista: um teste da esteira reprova a versão se um pedaço do produto passar
       a escrever sem aparecer aqui. Cada uma diz como se desfaz.</p>` +
    lista
      .map((a) => {
        const naTela = NA_TELA_DO_RISCO[a.risco];
        const marcas = [
          `<span class="chip">${escapeHtml(NA_TELA_DO_ESCOPO[a.escopo])}</span>`,
          `<span class="chip">${escapeHtml(naTela.rotulo)}</span>`,
          a.precisa_reiniciar ? `<span class="chip">exige reiniciar</span>` : "",
          a.refaz_sozinho ? `<span class="chip">o sistema refaz</span>` : "",
        ].join("");
        return `
        <div class="causa" data-severity="${naTela.severidade}">
          <p class="causa-titulo"><strong>${escapeHtml(a.titulo)}</strong> ${marcas}</p>
          <p class="effect">${escapeHtml(a.o_que_muda)}</p>
          <p class="detail"><strong>Desfazer:</strong> ${escapeHtml(a.desfazer)}</p>
        </div>`;
      })
      .join("");
}

async function carregarUltimasPartidas() {
  const alvo = document.getElementById("ultimas-partidas");
  if (!alvo) return;
  let medicoes: MedicaoAutomatica[];
  try {
    medicoes = await invoke<MedicaoAutomatica[]>("medicoes_automaticas");
  } catch (erro) {
    alvo.innerHTML = `<p class="hint">${escapeHtml(String(erro))}</p>`;
    return;
  }
  if (!medicoes.length) {
    alvo.innerHTML = `<p class="hint">Nenhuma partida medida ainda. A medição automática precisa da opção ligada em Sistema → Preferências e do Otimiza aberto como administrador; a partir daí ela mede sozinha enquanto você joga.</p>`;
    return;
  }
  const porJogo = new Map<string, MedicaoAutomatica[]>();
  for (const m of medicoes) {
    const lista = porJogo.get(m.jogo) ?? [];
    lista.push(m);
    porJogo.set(m.jogo, lista);
  }
  const linhas = [...porJogo.entries()]
    .map(([jogo, lista]) => {
      const ordenadas = [...lista].sort((a, b) => b.quando - a.quando);
      return { jogo, ultima: ordenadas[0], quantas: lista.length };
    })
    .sort((a, b) => b.ultima.quando - a.ultima.quando);

  alvo.innerHTML = `
    <table class="fg-tabela">
      <thead><tr><th>Jogo</th><th>Quando</th><th>FPS</th><th>1% piores</th><th>Partidas medidas</th></tr></thead>
      <tbody>${linhas
        .map(
          (l) => `<tr>
            <td class="fg-mono">${escapeHtml(l.jogo)}</td>
            <td>${new Date(l.ultima.quando * 1000).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" })}</td>
            <td>${Math.round(l.ultima.fps)}</td>
            <td>${l.ultima.confiavel ? Math.round(l.ultima.low_1pct).toString() : "amostra curta"}</td>
            <td>${l.quantas}</td>
          </tr>`,
        )
        .join("")}</tbody>
    </table>
    <p class="hint">Cada ajuste de jogo fica em observação: com pelo menos três partidas de cada lado, o Otimiza compara e desfaz sozinho o que piorou.</p>
    <div id="quedas-de-desempenho"></div>`;

  try {
    const quedas = await invoke<Deriva[]>("quedas_de_desempenho");
    const caixa = document.getElementById("quedas-de-desempenho");
    if (caixa && quedas.length) caixa.innerHTML = quedas.map((q) => `<p class="fg-aviso">${fraseDaQueda(q)}</p>`).join("");
  } catch (erro) {
    const caixa = document.getElementById("quedas-de-desempenho");
    if (caixa) caixa.innerHTML = `<p class="hint">Não deu para procurar quedas de FPS nas partidas: ${escapeHtml(String(erro))}</p>`;
  }
}

type Deriva = {
  jogo: string;
  mudou: { tipo: "Driver" | "Windows"; de: string; para: string } | { tipo: "Nada" };
  fps_antes: number;
  fps_depois: number;
  queda_pct: number;
  partidas_antes: number;
  partidas_depois: number;
};

function fraseDaQueda(q: Deriva): string {
  const numeros = `<strong>${escapeHtml(q.jogo)} caiu ${Math.round(q.queda_pct)}%</strong>: ${Math.round(q.fps_antes)} → ${Math.round(q.fps_depois)} FPS de média (${q.partidas_antes} partidas antes, ${q.partidas_depois} depois)`;
  switch (q.mudou.tipo) {
    case "Driver":
      return `${numeros}, depois que o driver de vídeo mudou de ${escapeHtml(q.mudou.de)} para ${escapeHtml(q.mudou.para)}. Voltar ao driver anterior costuma devolver; jogue de novo depois e compare.`;
    case "Windows":
      return `${numeros}, depois que o Windows atualizou (${escapeHtml(q.mudou.de)} → ${escapeHtml(q.mudou.para)}). Confira se o driver de vídeo continua o do fabricante, e não o genérico do Windows.`;
    case "Nada":
      return `${numeros}, sem mudança de driver nem de Windows. Vale olhar a temperatura (aba Diagnóstico) e o que está rodando junto com o jogo.`;
  }
}

async function carregarOQueNaoFazemos() {
  const alvo = element("nao-fazemos");

  let lista: NaoFazemos[];

  try {
    void carregarOQueAltera();
    lista = await invoke<NaoFazemos[]>("o_que_nao_fazemos");
  } catch (erro) {
    alvo.innerHTML = `<p class="status warn">${escapeHtml(String(erro))}</p>`;
    return;
  }

  alvo.innerHTML =
    `<p class="hint">Estes são ajustes que aparecem em toda lista de “aumente
       seu FPS”. O Otimiza não faz nenhum deles, e o motivo de cada um está
       escrito — para você conferir, não para acreditar.</p>` +
    lista
      .map((n) => {
        const naTela = NA_TELA_DA_NATUREZA[n.natureza];

        return `
        <div class="causa" data-severity="${naTela.severidade}">
          <p class="causa-titulo"><strong>${escapeHtml(n.nome)}</strong>
             — ${escapeHtml(naTela.rotulo)}</p>
          <p class="effect">${escapeHtml(n.porque)}</p>
        </div>`;
      })
      .join("");
}

async function carregarPorQueOFpsEstaBaixo() {
  const painel = element("causas-painel");
  const alvo = element("causas");

  let r: Investigacao;

  try {
    r = await invoke<Investigacao>("por_que_o_fps_esta_baixo");
  } catch (erro) {
    painel.hidden = false;
    alvo.innerHTML = `<p class="status warn">${escapeHtml(String(erro))}</p>`;
    return;
  }

  painel.hidden = false;

  // Só o suspeito Otimiza em âmbar: os outros são fatos da máquina.
  const cartoes = r.suspeitos
    .map(
      (s) => `
      <div class="causa" data-severity="${s.id === "foi_o_otimiza" ? "Important" : "Neutral"}">
        <p class="causa-titulo"><strong>${escapeHtml(s.titulo)}</strong></p>
        <p class="causa-medido">${escapeHtml(s.medido)}</p>
        <p class="effect">${escapeHtml(s.porque)}</p>
        <p class="causa-confirmar"><strong>Como confirmar:</strong> ${escapeHtml(s.como_confirmar)}</p>
      </div>`
    )
    .join("");

  // A lista vazia diz o que foi procurado; a frase vem do Rust.
  const corpo =
    r.suspeitos.length > 0
      ? cartoes
      : `<p class="bloco-de-prosa">Nenhuma das causas conhecidas de FPS baixo foi
           encontrada aqui: memória de vídeo curta, jogo em disco mecânico, faixas de
           PCI Express estreitas, memória abaixo da velocidade nominal ou em canal
           único, e limite de temperatura ou energia ativo. Isso não quer dizer que o
           FPS esteja bom — quer dizer que o motivo não é nenhum destes cinco.</p>`;

  const lacunas = r.lacunas.length
    ? `<p class="hint"><strong>Não deu para verificar:</strong><br>
         ${r.lacunas.map(escapeHtml).join("<br>")}</p>`
    : "";

  alvo.innerHTML = corpo + lacunas;
}

async function carregarOndeOsJogosMoram() {
  const painel = element("disco-do-jogo-painel");
  const alvo = element("disco-do-jogo");

  let relatorio: RelatorioDeDiscos;

  try {
    relatorio = await invoke<RelatorioDeDiscos>("onde_os_jogos_moram");
  } catch (erro) {
    painel.hidden = false;
    alvo.innerHTML = `<p class="status warn">${escapeHtml(String(erro))}</p>`;
    return;
  }

  if (relatorio.jogos.length === 0 && relatorio.lacunas.length === 0) {
    painel.hidden = true;
    return;
  }

  painel.hidden = false;

  const linhas = relatorio.jogos
    .map((j) => {
      const naTela = NA_TELA_DO_DISCO[j.midia];
      const modelo = j.modelo ? ` — ${escapeHtml(j.modelo)}` : "";

      // Só no caso PROVADO: disco ilegível não recebe "passe para o SSD".
      const conselho =
        j.midia === "Mecanico"
          ? `<p class="effect">Num servidor de RP o jogo lê textura e modelo o tempo todo
               enquanto você anda pelo mapa, e um disco de prato não entrega isso na
               velocidade que o jogo pede — é o que aparece como travada ao virar a esquina
               e como FPS que muda conforme o lugar. Passar este jogo para o SSD costuma ser
               o maior ganho isolado numa máquina com os dois discos.</p>`
          : "";

      return `
        <div class="linha-do-disco" data-severity="${naTela.severidade}">
          <p><strong>${escapeHtml(j.jogo)}</strong> — ${escapeHtml(j.unidade ?? "?")}:
             ${escapeHtml(naTela.rotulo)}${modelo}</p>
          <p class="detail">${escapeHtml(j.caminho)}</p>
          ${conselho}
        </div>`;
    })
    .join("");

  const lacunas = relatorio.lacunas.length
    ? `<p class="hint">${relatorio.lacunas.map(escapeHtml).join("<br>")}</p>`
    : "";

  alvo.innerHTML = linhas + lacunas;
}

async function conferirOProprioTrabalho() {
  const faixa = element("regressao-faixa");
  const texto = element("regressao-texto");

  let vereditos: VereditoDeRegressao[];

  try {
    vereditos = await invoke<VereditoDeRegressao[]>("conferir_o_proprio_trabalho");
  } catch {
    faixa.hidden = true;
    return;
  }

  const pior = vereditos.find(
    (v) => v.desfecho.desfecho === "Piorou" || v.desfecho.desfecho === "PiorouMuito"
  );

  if (!pior || pior.antes === null || pior.depois === null) {
    faixa.hidden = true;
    return;
  }

  const queda = Math.abs(Math.round(pior.variacao_fps_pct ?? 0));

  texto.innerHTML = `
    <strong>O ${escapeHtml(pior.jogo)} está pior depois que otimizamos.</strong><br>
    Antes: ${pior.antes.fps.toFixed(0)} FPS · 1% piores ${pior.antes.low_1pct.toFixed(0)}.
    Depois: ${pior.depois.fps.toFixed(0)} FPS · 1% piores ${pior.depois.low_1pct.toFixed(0)}.
    São ${queda}% a menos, medidos nesta máquina.
  `;

  faixa.hidden = false;
}

async function carregarMedicoesAutomaticas() {
  const alvo = element("prova-automaticas");

  let medicoes: MedicaoAutomatica[];

  try {
    medicoes = await invoke<MedicaoAutomatica[]>("medicoes_automaticas");
  } catch (error) {
    alvo.hidden = false;
    alvo.innerHTML = `<p class="status warn">${escapeHtml(String(error))}</p>`;
    return;
  }

  if (medicoes.length === 0) {
    alvo.hidden = true;
    return;
  }

  const linhas = medicoes
    .slice(-10)
    .reverse()
    .map((m) => {
      const quando = new Date(m.quando * 1000);
      const dia = quando.toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });
      const hora = quando.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
      const curta = m.confiavel ? "" : " · amostra curta";

      return (
        `<li><strong>${dia} ${hora}</strong> · ${escapeHtml(m.jogo)} · ` +
        `${m.fps.toFixed(0)} FPS · 1% piores ${m.low_1pct.toFixed(0)} · ` +
        `${m.engasgos_por_minuto.toFixed(0)} engasgos/min · ` +
        `${m.mudancas_aplicadas} mudança(s) do Otimiza aplicada(s)${curta}</li>`
      );
    })
    .join("");

  alvo.hidden = false;
  alvo.innerHTML =
    `<p><strong>Medido sozinho durante as partidas</strong></p>` +
    (await notaDoJogoEmHtml()) +
    `<ul class="lista">${linhas}</ul>` +
    `<p class="hint">Cada linha foi medida num momento e num lugar diferentes do jogo, ` +
    `então elas não se comparam entre si como antes e depois. Para isso, use os botões acima.</p>`;
}

/** A frase inteira vem do Rust: a mesma nota com e sem engasgo pede conselhos diferentes. */
async function notaDoJogoEmHtml(): Promise<string> {
  type Nota =
    | { estado: "SemAmostra" }
    | {
        estado: "Calculada";
        nota: number;
        fps_medio: number;
        low_1pct: number;
      };

  let nota: Nota;

  try {
    nota = await invoke<Nota>("nota_do_jogo");
  } catch {
    return "";
  }

  if (nota.estado === "SemAmostra") return "";

  return `
    <p class="bloco-de-prosa"><strong>Nota ${nota.nota} de 100</strong> —
      ${nota.fps_medio.toFixed(0)} FPS de média e ${nota.low_1pct.toFixed(0)} no 1% pior.
      O 1% pior pesa mais que a média nesta conta, porque é ele que você sente.
      A nota descreve esta máquina neste jogo e serve para comparar antes e depois,
      não para comparar com o PC de outra pessoa.</p>`;
}

async function medirAntes() {
  const botao = element<HTMLButtonElement>("prova-antes");

  await preencherJogoDetectado();

  const processo = element<HTMLInputElement>("prova-processo").value.trim();

  if (!processo) {
    setStatus(
      "prova-status",
      "Não achei nenhum jogo aberto. Abra o jogo, entre numa partida, e clique de novo — " +
        "ou escreva o nome do executável aqui do lado.",
      "error"
    );
    return;
  }

  botao.disabled = true;
  setStatus("prova-status", `Medindo ${processo} por 20 segundos… jogue normalmente.`, "progress");

  try {
    const p = await invoke<Prova>("medir_antes", {
      process: processo,
      seconds: segundosDaMedicao(),
    });

    text("prova-tag", `antes: ${p.fps.toFixed(0)} FPS`);

    element("prova-result").innerHTML =
      `<p><strong>Medição guardada.</strong></p>` +
      `<ul class="lista">
         <li>Média: <strong>${p.fps.toFixed(0)} FPS</strong></li>
         <li>1% piores quadros: <strong>${p.low_1pct.toFixed(0)} FPS</strong></li>
         <li>Engasgos: <strong>${p.engasgos_por_minuto.toFixed(0)} por minuto</strong></li>
       </ul>` +
      `<p class="hint">Agora feche o jogo, aplique as mudanças no painel acima, abra o jogo de novo <strong>no mesmo lugar</strong>, e meça outra vez.</p>` +
      (p.confiavel
        ? ""
        : `<p class="hint">A amostra ficou curta: os detalhes acima são pouco confiáveis. Vale medir de novo com o jogo em movimento.</p>`);

    setStatus("prova-status", "Medição guardada. Agora feche o jogo e aplique as mudanças.", "ok");
  } catch (error) {
    setStatus("prova-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

async function medirDepois() {
  const botao = element<HTMLButtonElement>("prova-depois");
  const processo = element<HTMLInputElement>("prova-processo").value.trim();

  botao.disabled = true;
  setStatus("prova-status", `Medindo ${processo} por 20 segundos… jogue normalmente.`, "progress");

  try {
    const c = await invoke<ComparacaoDaProva>("medir_depois", {
      process: processo,
      seconds: segundosDaMedicao(),
    });

    renderComparacao(c);

    // Medição que piorou não sai verde.
    setStatus(
      "prova-status",
      c.vale_como_prova
        ? "Ganho medido e confirmado."
        : "Leia as ressalvas antes de tirar conclusão.",
      c.vale_como_prova ? "ok" : "warn",
    );
  } catch (error) {
    setStatus("prova-status", String(error), "error");
  } finally {
    botao.disabled = false;
  }
}

function renderComparacao(c: ComparacaoDaProva) {
  // A etiqueta nunca mente sobre o sinal.
  const sinal = c.fps_delta > 0 ? "+" : "";
  text("prova-tag", `${sinal}${c.fps_delta.toFixed(0)} FPS`);

  const linha = (rotulo: string, antes: number, depois: number, unidade = "FPS") => {
    const d = depois - antes;
    const seta = d > 0 ? "↑" : d < 0 ? "↓" : "=";
    return `<li>${rotulo}: <strong>${antes.toFixed(0)}</strong> → <strong>${depois.toFixed(
      0
    )}</strong> ${unidade} ${seta}</li>`;
  };

  const ressalvas = c.ressalvas.length
    ? `<p class="hint"><strong>Antes de tirar conclusão:</strong></p><ul class="lista">${c.ressalvas
        .map((r) => `<li>${escapeHtml(r)}</li>`)
        .join("")}</ul>`
    : "";

  element("prova-result").innerHTML =
    `<p class="lead">${escapeHtml(c.veredito)}</p>` +
    `<ul class="lista">
       ${linha("Média", c.antes.fps, c.depois.fps)}
       ${linha("1% piores", c.antes.low_1pct, c.depois.low_1pct)}
       ${linha("Engasgos", c.antes.engasgos_por_minuto, c.depois.engasgos_por_minuto, "por minuto")}
     </ul>` +
    ressalvas;
}

function wireControls() {
  const secoes = Array.from(
    document.querySelectorAll<HTMLButtonElement>(".nav[data-tab]")
  );

  secoes.forEach((item) => {
    item.addEventListener("click", () => showTab(item.dataset.tab!));

    // Com a lateral recolhida o ícone é tudo: a dica copia o rótulo.
    const rotulo = item.querySelector(".nav-rotulo")?.textContent?.trim();
    if (rotulo) item.title = rotulo;
  });

  // Sem isto o programa abre com o título duplicado.
  const inicial = secoes.find((s) => s.getAttribute("aria-selected") === "true");
  if (inicial) sincronizarCabecalho(inicial, inicial.dataset.tab!);

  // Setas de cima e baixo, padrão de acessibilidade para abas verticais.
  document.querySelector(".lateral")!.addEventListener("keydown", (event) => {
    const key = (event as KeyboardEvent).key;
    if (key !== "ArrowDown" && key !== "ArrowUp") return;

    event.preventDefault();

    const atual = secoes.findIndex((s) => s.getAttribute("aria-selected") === "true");
    const proxima =
      (atual + (key === "ArrowDown" ? 1 : secoes.length - 1)) % secoes.length;

    showTab(secoes[proxima].dataset.tab!);
    secoes[proxima].focus();
  });

  const corpo = document.querySelector<HTMLElement>(".corpo")!;
  const alternar = element<HTMLButtonElement>("toggle-lateral");

  // Recolhida por padrão: numa tela de notebook o rótulo ocupa o espaço de um painel.
  if (localStorage.getItem("lateral-recolhida") !== "nao") {
    corpo.dataset.recolhida = "true";
    alternar.setAttribute("aria-expanded", "false");
  }

  alternar.addEventListener("click", () => {
    const recolhida = corpo.dataset.recolhida === "true";

    corpo.dataset.recolhida = String(!recolhida);
    alternar.setAttribute("aria-expanded", String(recolhida));
    localStorage.setItem("lateral-recolhida", recolhida ? "nao" : "sim");
  });

  wireComandos(secoes);

  (element("statusbar-tutorial") as HTMLAnchorElement).href = TUTORIAL_URL;

  // O embutido primeiro, corrigido quando o resolvido chegar.
  const suporte = element("statusbar-suporte") as HTMLAnchorElement;
  suporte.href = CONVITE_DISCORD;
  void resolverConvite().then((endereco) => {
    suporte.href = endereco;
  });

  element("run-diagnostic").addEventListener("click", runDiagnostic);
  element("analyze-firmware").addEventListener("click", analyzeFirmware);

  element("analyze-boot").addEventListener("click", analyzeBoot);
  element("analyze-thermal").addEventListener("click", analyzeThermal);
  document.getElementById("msi-ler")?.addEventListener("click", () => void lerMsi());
  document.getElementById("dpc-medir")?.addEventListener("click", () => void medirDpc());
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
  element("analyze-rbar").addEventListener("click", analyzeRbar);

  // O mapa não apaga: leva ao liberador, que limpa por categoria com confirmação.
  element("map-result").addEventListener("click", (event) => {
    const botao = (event.target as HTMLElement).closest("button[data-mapa-limpar]");
    if (!botao) return;

    element("disk-result").scrollIntoView({ behavior: "smooth", block: "center" });
    scanDiskSpace();
  });
  element("analyze-browsers").addEventListener("click", analyzeBrowsers);
  element("analyze-fivem").addEventListener("click", analyzeFiveM);
  element("medir-perda").addEventListener("click", medirPerdaDePacote);
  element("analyze-bottleneck").addEventListener("click", analyzeBottleneck);
  element("analyze-shaders").addEventListener("click", analyzeShaders);
  element("analyze-streaming").addEventListener("click", analyzeStreaming);
  ligarTema();
  ligarProgramas();
  ligarLimpeza();
  ligarNucleos();
  void carregarNucleos();
  void carregarProgramas();
  void carregarCartaoDaPlaca();
  // Pela mesma função da lateral, senão ela marcaria a seção errada.
  for (const atalho of document.querySelectorAll<HTMLElement>("[data-vai]")) {
    atalho.addEventListener("click", () => showTab(atalho.dataset.vai!));
  }

  element("mouse-ler").addEventListener("click", lerCaminhoDoMouse);
  element("historico-ler").addEventListener("click", lerHistorico);
  for (const botao of document.querySelectorAll<HTMLButtonElement>("[data-marca-manual]")) {
    botao.addEventListener("click", () => {
      // A escolha manual só pinta o desenho: nenhum ajuste muda por ela.
      element("placa-painel").dataset.marca = botao.dataset.marcaManual ?? "desconhecida";
      text("placa-marca", botao.dataset.marcaManual ?? "placa de vídeo");
    });
  }

  void carregarPlaca();
  void carregarPreferenciaDeGpu();
  void carregarAjustesDoDriver();

  element("nvlimite-fps").addEventListener("click", (event) => {
    const botao = (event.target as HTMLElement).closest<HTMLButtonElement>("[data-fps]");
    if (!botao) return;

    limiteDeFpsEscolhido = Number(botao.dataset.fps);
    marcarLimiteEscolhido();
  });
  element("nvlimite-aplicar").addEventListener("click", limitarJogoAberto);
  marcarLimiteEscolhido();
  void carregarMemoria();
  void carregarMonitores();
  element("cfgjogo-analisar").addEventListener("click", analisarConfigJogo);
  element("cfgjogo-plano").addEventListener("click", planoDeRenderizacao);
  element("cfgjogo-sem-teto").addEventListener("click", () => aplicarPerfilDoJogo("sem_teto"));
  element("cfgjogo-equilibrado").addEventListener("click", () => aplicarPerfilDoJogo("equilibrado"));
  element("cfgjogo-competitivo").addEventListener("click", () => aplicarPerfilDoJogo("competitivo"));
  void restaurarProvaGuardada().then(() => void preencherJogoDetectado());
  void carregarMedicoesAutomaticas();
  element("prova-antes").addEventListener("click", medirAntes);
  element("prova-depois").addEventListener("click", medirDepois);
  element("analyze-readiness").addEventListener("click", analyzeReadiness);
  element("unfix-priority").addEventListener("click", () => fixPriority(false));

  element("shader-result").addEventListener("click", async (event) => {
    const botao = (event.target as HTMLElement).closest(
      "button[data-shader]"
    ) as HTMLButtonElement | null;
    if (!botao) return;

    const ok = window.confirm(
      "Isto apaga o cache de shader e não tem volta.\n\n" +
        "Nada se perde além de tempo: o conteúdo é resultado de compilação e o " +
        "jogo refaz sozinho. A primeira partida depois da limpeza vai compilar " +
        "de novo e pode engasgar; da segunda em diante fica melhor.\n\nContinuar?"
    );
    if (!ok) return;

    botao.disabled = true;

    try {
      const outcome = await invoke<{ freed_mb: number; message: string }>(
        "clean_shader_cache",
        { id: botao.dataset.shader }
      );
      setStatus("shader-status", outcome.message, "ok");
    } catch (error) {
      setStatus("shader-status", String(error), "error");
    } finally {
      await analyzeShaders();
    }
  });

  element("prontidao-result").addEventListener("click", async (event) => {
    const botao = (event.target as HTMLElement).closest(
      "button[data-readiness]"
    ) as HTMLButtonElement | null;
    if (!botao) return;

    if (!isElevated) {
      askForAdmin(
        "Corrigir esta condição do sistema exige permissão de administrador. " +
          "Podemos reabrir o Otimiza com essa permissão?"
      );
      return;
    }

    botao.disabled = true;

    try {
      setStatus(
        "prontidao-status",
        await invoke<string>("fix_readiness", { id: botao.dataset.readiness }),
        "ok"
      );
    } catch (error) {
      setStatus("prontidao-status", String(error), "error");
    } finally {
      await analyzeReadiness();
    }
  });
  element("gamemode-on").addEventListener("click", () => setGameMode(true));
  element("gamemode-off").addEventListener("click", () => setGameMode(false));
  element("gamemode-zerar").addEventListener("click", zerarModoJogo);
  element("copiar-diagnostico").addEventListener("click", () => copiarDiagnostico());
  element("measure-frames").addEventListener("click", measureFrames);

  element("fivem-result").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-fivem]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    const ok = window.confirm(
      "Isto apaga o cache do FiveM e não tem volta.\n\n" +
        "Seu perfil do jogo, sua conta da Rockstar e seus mods não são tocados. " +
        "O que sai é o conteúdo que os servidores reenviam sozinhos — e é por " +
        "isso que, na primeira vez que você entrar em cada servidor depois " +
        "disso, ele vai baixar tudo de novo.\n\nContinuar?"
    );
    if (!ok) return;

    button.disabled = true;

    try {
      const outcome = await invoke<{ freed_mb: number; message: string }>("clean_fivem", {
        id: button.dataset.fivem,
      });
      setStatus("fivem-status", outcome.message, "ok");
    } catch (error) {
      setStatus("fivem-status", String(error), "error");
    } finally {
      await analyzeFiveM();
    }
  });

  element("browser-result").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-browser]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    const ok = window.confirm(
      "Isto apaga o cache do navegador e não tem volta.\n\n" +
        "Nada de histórico, senha ou favorito é tocado — só arquivos que o " +
        "navegador baixa de novo sozinho. A contrapartida é que os sites que " +
        "você usa vão carregar mais devagar na primeira visita.\n\nContinuar?"
    );
    if (!ok) return;

    button.disabled = true;

    try {
      const outcome = await invoke<{ freed_mb: number; message: string }>(
        "clean_browser_cache",
        { executable: button.dataset.browser }
      );
      setStatus("browser-status", outcome.message, "ok");
    } catch (error) {
      setStatus("browser-status", String(error), "error");
    } finally {
      await analyzeBrowsers();
    }
  });

  element("disk-result").addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-space]"
    ) as HTMLButtonElement | null;
    if (button) cleanDiskCategory(button.dataset.space!, button);
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

  element("pref-gamemode").addEventListener("change", (event) =>
    savePreferences({ auto_game_mode: (event.target as HTMLInputElement).checked })
  );

  element("pref-medir-sozinho").addEventListener("change", (event) =>
    savePreferences({ medir_quadros_sozinho: (event.target as HTMLInputElement).checked })
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
  document.getElementById("exportar-alteracoes")?.addEventListener("click", async (e) => {
    const botao = e.currentTarget as HTMLButtonElement;
    botao.disabled = true;
    try {
      const caminho = await invoke<string>("exportar_alteracoes");
      text("report-status", `Planilha gravada: ${caminho}`);
    } catch (erro) {
      text("report-status", String(erro));
    } finally {
      botao.disabled = false;
    }
  });

  element("optimize-now").addEventListener("click", () =>
    runBatch("optimize_now", "Aplicando o que falta…")
  );

  element("plano-diagnostico").addEventListener("click", diagnosticarEnergia);
  element("plano-simular").addEventListener("click", simularPlano);
  element("plano-reparar").addEventListener("click", repararPlano);
  element("lab-gerar").addEventListener("click", gerarLab);
  element("lab-copiar").addEventListener("click", copiarLab);
  element("plano-aplicar").addEventListener("click", aplicarPlano);
  element("plano-desfazer").addEventListener("click", desfazerPlano);

  element("nivel-chips").addEventListener("click", (event) => {
    const chip = (event.target as HTMLElement).closest(
      "button[data-nivel]"
    ) as HTMLButtonElement | null;
    if (chip) escolherNivel(chip.dataset.nivel!);
  });

  // A escuta fica no painel: o botão é redesenhado a cada escolha.
  element("nivel-detalhe").addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest("#aplicar-nivel");
    if (!button) return;

    const nivel = niveis.find((n) => n.id === nivelEscolhido);
    // A trava vale nos dois lados: se o botão aparecer por outro caminho no Experimental, recusa.
    if (!nivel || !nivel.aplica_de_uma_vez) return;

    runBatch("optimize_now", `Aplicando o nível ${nivel.nome}…`, nivel.itens);
  });

  element("profile-chips").addEventListener("click", (event) => {
    const chip = (event.target as HTMLElement).closest(
      "button[data-profile]"
    ) as HTMLButtonElement | null;
    if (chip) selectProfile(chip.dataset.profile!);
  });

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

  const caixaExpert = document.getElementById("modo-expert") as HTMLInputElement | null;
  if (caixaExpert) {
    caixaExpert.checked = modoExpert;
    caixaExpert.addEventListener("change", () => {
      modoExpert = caixaExpert.checked;
      try {
        localStorage.setItem("otimiza.modo", modoExpert ? "expert" : "simples");
      } catch {
        /*
         * sem armazenamento: vale só nesta sessão
         */
      }
      aplicarModoExpert();
      renderOptimizations();
    });
  }
  aplicarModoExpert();

  const busca = element<HTMLInputElement>("optimization-search");
  busca.addEventListener("input", () => {
    searchTerm = busca.value;
    element("search-clear").hidden = searchTerm.length === 0;
    renderOptimizations();
  });

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
    if (event.target === element("admin-modal")) closeAdminModal();
  });

  // Não fecha ao clicar fora: é uma decisão, com as três saídas nos botões.
  element("essenciais-religar").addEventListener("click", religarEssenciais);
  element("essenciais-mesmo-assim").addEventListener("click", otimizarMesmoAssim);
  element("essenciais-cancelar").addEventListener("click", fecharAvisoDosEssenciais);

  window.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;

    closeAdminModal();
    if (!element("essenciais-modal").hidden) fecharAvisoDosEssenciais();
  });

  element("startup-list").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-startup]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    const hive = button.dataset.hive!;
    const enable = button.dataset.enable === "true";

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

  element("optimization-list").addEventListener("click", async (event) => {
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

    // Dentro de um <summary>: sem isto, "Aplicar" abriria e fecharia os detalhes.
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
