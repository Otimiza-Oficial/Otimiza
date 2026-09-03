import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Esfera } from "./esfera";
import { Pilares } from "./pilares";
import { ligarBarraDaJanela } from "./janela";

// ---------------------------------------------------------------- contratos

type Verdict = "Improved" | "Worsened" | "NoMeasurableChange" | "TooNoisyToJudge";
type State = "Applied" | "AlreadyOptimal" | "Available" | "Unavailable";
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
  /** Se a pessoa já viu a tela que explica que o modo jogo congela programas. */
  game_mode_avisado: boolean;
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

// ------------------------------------------------------------- o veredito

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
  auto_game_mode: false,
  metrics_interval_seconds: 2,
  show_unavailable: true,
  game_mode_avisado: false,
};
/** Handle do laço de medição, para poder trocar o intervalo sem recarregar. */
let metricsTimer: number | null = null;
/** Categorias recolhidas pelo usuário, preservadas entre recarregamentos da lista. */
const collapsedGroups = new Set<string>();

/**
 * O que cada nível de ganho significa, dito sem eufemismo.
 *
 * "resposta do sistema" era o rótulo antigo de `Responsiveness`, e o próprio
 * comentário do enum no backend define esse nível como "não muda FPS". Um
 * cliente que aplica dezessete itens rotulados "resposta do sistema" espera
 * dezessete melhoras — e não recebe nenhuma no jogo, porque não é isso que
 * eles fazem. O rótulo agora diz o que o código sempre soube.
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
};

const VERDICT_LABELS: Record<Verdict, string> = {
  Improved: "melhorou",
  Worsened: "piorou",
  NoMeasurableChange: "sem diferença",
  TooNoisyToJudge: "só referência",
};

// ------------------------------------------------------------------- início

/**
 * Três invariantes de tela, conferidos só em desenvolvimento.
 *
 * A folha de estilo deste produto tem um comentário explicando que 58% dela
 * foi anexada depois do que era o fim do arquivo, em camadas sucessivas — e é
 * exatamente assim que uma tela limpa vira uma tela poluída: nunca de uma vez,
 * sempre por acréscimo.
 *
 * Estas três linhas custam nada em produção e reclamam no console no dia em
 * que alguém acrescentar o painel de número 22, o décimo botão de ênfase, ou
 * uma caixa de resultado que não diz o que vai aparecer nela.
 */
function conferirInvariantesDaTela() {
  // `import.meta.env` do Vite não está nos tipos deste projeto, e acrescentar
  // a referência de tipos só para isto não se paga. O endereço basta: em
  // produção o app roda de `tauri://`, nunca de `localhost`.
  if (!location.hostname.startsWith("localhost")) return;

  const paineis = document.querySelectorAll(".panel").length;
  const enfase = document.querySelectorAll(".palco .btn-primary").length;
  const caixasMudas = [...document.querySelectorAll(".resultado")].filter(
    (caixa) => !(caixa as HTMLElement).dataset.vazio
  );

  console.assert(paineis <= 35, `painéis demais na tela: ${paineis}`);
  console.assert(enfase <= 8, `ênfase primária demais: ${enfase} botões`);
  console.assert(
    caixasMudas.length === 0,
    `caixa de resultado sem dizer o que vai aparecer: ${caixasMudas
      .map((c) => c.id)
      .join(", ")}`
  );

  // MENSAGEM QUE PISCA É MENSAGEM QUE AINDA ESTÁ ESPERANDO.
  //
  // A classe `.empty` serve para dois casos opostos: "Lendo os serviços…" e
  // "Nada encontrado". O pulso vale só para o primeiro. Quando ele vale para
  // os dois, a pessoa fica esperando um resultado que já chegou.
  //
  // A regra que dá para conferir sozinha: reticências e pulso andam juntos.
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

/* ==========================================================================
   O PORTÃO — a licença

   Duas regras que valem escrever:

   1. Esta tela NÃO é o bloqueio. Ela é HTML dentro de uma janela que tem
      ferramentas de desenvolvedor, e qualquer pessoa a remove em dez segundos.
      O bloqueio de verdade está no Rust, na primeira linha dos 21 comandos que
      alteram o computador. Aqui é conforto: explicar, e não vigiar.

   2. O diagnóstico continua rodando por trás. Uma tela de compra que diz "seu
      PC pode estar lento" é propaganda; uma que diz o que ESTA máquina tem, com
      número medido agora, é outra conversa.
   ========================================================================== */

/**
 * O convite do Discord.
 *
 * É o único endereço que a tela de compra oferece. Um convite errado ou
 * vencido aqui é uma venda perdida sem que ninguém fique sabendo — e o cliente
 * não tem outro caminho para chegar até o dono.
 *
 * ATENÇÃO — CONFERIDO EM 2026-08-29 E ESTE CONVITE VENCE EM 2026-09-28.
 *
 * Convite do Discord expira por padrão. Este produto não tem camada de rede
 * nenhuma — zero dependências HTTP, por decisão de projeto —, então um convite
 * vencido aqui não tem conserto remoto: quem instalou antes fica com um link
 * morto e sem nenhum caminho até o dono.
 *
 * Trocar por um convite com "Expira em: Nunca" e "Usos: Sem limite".
 */
const CONVITE_DISCORD = "https://discord.gg/fmeQVJphC";

interface EstadoLicenca {
  ativa: boolean;
  maquina: string;
  origem: string;
  sobrevive_formatacao: boolean;
  comprador: string | null;
  expira: string | null;
  motivo: string | null;
}

/** Guardado para o resto da tela saber se o portão está de pé. */
let portaoAberto = false;

/**
 * Decide se o portão aparece, e monta o que ele mostra.
 *
 * Roda antes de tudo no arranque. Se a chamada falhar — coisa que não deveria
 * acontecer, porque o comando não toca em nada do sistema —, o portão fica
 * fechado e o programa abre normal: o backend continua recusando o que altera
 * a máquina, então errar para o lado de deixar entrar não solta nada.
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

  // O console atrás fica inalcançável pelo teclado. Sem isto, o Tab passeia
  // por trás da tela e o foco some da vista de quem navega sem mouse.
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
  convite.href = CONVITE_DISCORD;
  text("portao-discord-endereco", CONVITE_DISCORD.replace(/^https?:\/\//, ""));

  // Uma chave gravada que parou de valer — máquina trocada, prazo vencido —
  // precisa dizer o motivo. Sem isso, o cliente que pagou vê a mesma tela de
  // quem nunca comprou e conclui que foi enganado.
  if (estado.motivo) {
    const erro = element("portao-erro");
    erro.hidden = false;
    erro.textContent = estado.motivo;
  }

  element("portao-chave").focus();
}

/**
 * A chegada: o instante entre colar a chave e usar o programa.
 *
 * Sem isto o portão simplesmente sumia e a pessoa caía no painel — que
 * funciona, e trata a compra como um formulário que passou.
 *
 * O nome vem da própria licença, do campo que o bot preencheu ao emitir. Não há
 * consulta a lugar nenhum: a chave que ela colou já carrega quem ela é.
 */
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

  // O console fica inalcançável pelo teclado enquanto esta tela está de pé,
  // pelo mesmo motivo do portão: o foco não pode passear atrás dela.
  document.querySelector(".console")?.setAttribute("inert", "");

  element<HTMLButtonElement>("chegada-entrar").focus();

  element("chegada-entrar").onclick = () => {
    tela.hidden = true;
    document.querySelector(".console")?.removeAttribute("inert");

    // A pessoa acabou de comprar por causa de um problema. Levar direto ao
    // Painel é levar ao veredito — a resposta pela qual ela pagou.
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
      // A área de transferência pode ser negada. O código continua
      // selecionável no próprio elemento (`user-select: all`), então dizer o
      // que fazer resolve melhor do que um erro genérico.
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

  // Enter ativa; Shift+Enter continua quebrando linha, porque o campo é uma
  // caixa de várias linhas e a chave colada de uma mensagem vem partida.
  campo.addEventListener("keydown", (evento) => {
    if (evento.key === "Enter" && !evento.shiftKey) {
      evento.preventDefault();
      void ativar();
    }
  });
}

/**
 * Leva o achado do diagnóstico para a tela de compra.
 *
 * Só o principal, e só quando existe. Repetir a lista inteira ali viraria
 * catálogo de defeitos — o que é assustar para vender, exatamente o que este
 * produto não faz.
 */
function mostrarAchadoNoPortao(v: Veredito) {
  if (!portaoAberto) return;

  const caixa = element("portao-achado");

  if (!v.principal) {
    caixa.hidden = true;
    return;
  }

  caixa.hidden = false;

  // O PONTO TEM A COR DO ACHADO, E NÃO UMA COR FIXA.
  //
  // Ele nasceu sempre âmbar. Numa máquina com a memória estourada isso punha
  // um ponto de "atenção" ao lado de uma frase que diz que o computador está
  // no limite — o mesmo desencontro que a esfera já teve, e o tipo de coisa
  // que faz o cliente duvidar do diagnóstico inteiro logo na tela da compra.
  caixa.dataset.nivel = v.principal.severity === "Critical" ? "critico" : "importante";

  text("portao-achado-titulo", v.frase);
  text("portao-achado-detalhe", v.detalhe);
}

/**
 * A esfera do veredito — a máquina desenhada com as próprias medições.
 *
 * Uma só instância: ela vive enquanto o programa vive, e recebe cada leitura
 * nova do monitor.
 */
let esfera: Esfera | null = null;

/**
 * AS DUAS TELAS GRANDES NÃO USAM A ESFERA, E SIM OS PILARES.
 *
 * A esfera continua sendo o medidor do painel: pequena, ao lado do veredito,
 * girando com o uso de CPU. Ela funciona ali porque é um instrumento.
 *
 * Nas telas de ativação e de chegada a imagem é quase mil pixels e tem outro
 * trabalho — ser a primeira coisa que a pessoa vê do produto. Ampliada, a
 * esfera não aguentava esse papel: virava uma bola de pontinhos, e a diferença
 * entre uma máquina saudável e uma sufocada ficava invisível.
 *
 * As colunas aguentam, e informam MAIS: são três, uma para cada peça que
 * sustenta o desempenho — processador, memória e disco —, cada uma se
 * desfazendo pela sua própria leitura. A esfera dizia "a máquina está mal";
 * os pilares dizem QUAL delas está.
 */
let pilaresDoPortao: Pilares | null = null;
let pilaresDaChegada: Pilares | null = null;

/**
 * Repassa uma medição para todas as esferas que existirem.
 *
 * Sem isto, a do portão ficaria parada no estado "não medido" enquanto a do
 * painel já mostrava a máquina — e as duas telas mostrariam computadores
 * diferentes.
 */
function alimentarEsferas(leitura: Parameters<Esfera["atualizar"]>[0]) {
  esfera?.atualizar(leitura);
  esfera?.redesenhar();
}

/**
 * Repassa uma medição para as duas telas grandes.
 *
 * Elas recebem uma leitura DIFERENTE da esfera, e não a mesma: a esfera precisa
 * de núcleos e de CPU para semear e tremer, os pilares precisam de disco — que
 * a esfera não usa — porque a terceira coluna é o disco.
 */
function alimentarPilares(leitura: Parameters<Pilares["atualizar"]>[0]) {
  pilaresDoPortao?.atualizar(leitura);
  pilaresDaChegada?.atualizar(leitura);
}

/**
 * Tira o nome de quem comprou, do jeito que o bot gravou na licença.
 *
 * O bot emite com `comprador` no formato `fulano#1234 (1234567890)`. O
 * identificador numérico não diz nada para a pessoa — ela quer ver o nome
 * dela, não o número dela.
 */
function primeiroNome(comprador: string | null): string | null {
  if (!comprador) return null;

  // O QUE VEM ENTRE PARÊNTESES É O CÓDIGO DA COMPRA, E ELE NÃO É SÓ NÚMERO.
  //
  // Esta limpeza procurava `(só dígitos)`, supondo o identificador do Discord.
  // O bot emite outra coisa: `fulano (CMP-2026-000004)`. Com letras e traços no
  // meio, a expressão nunca casava e o código ficava colado no nome. O que
  // salvava era o `split` lá embaixo, por acidente — e ele deixaria de salvar
  // no dia em que alguém se chamasse "Ana Paula".
  const semCodigo = comprador.replace(/\s*\([^)]*\)\s*$/, "").trim();
  const semTag = semCodigo.replace(/#\d{4}$/, "").trim();

  // Nome de teste, ou vazio, não vira saudação. "Obrigado, conferencia" seria
  // pior do que só "Obrigado".
  if (!semTag || semTag.length < 2 || /^conferencia$/i.test(semTag)) return null;

  const primeiro = semTag.split(/\s+/)[0];

  // PONTUAÇÃO NO FIM DO NOME SAI, SENÃO A FRASE GANHA DOIS PONTOS.
  //
  // A tela monta "Obrigado, {nome}." — e apelido do Discord pode terminar em
  // pontuação. Um comprador de verdade chamado "exaggerateyourdreams." vira
  // "Obrigado, exaggerateyourdreams..", que parece defeito porque é.
  const limpo = primeiro.replace(/[.,;:!?]+$/, "");

  return limpo.length >= 2 ? limpo : null;
}

window.addEventListener("DOMContentLoaded", async () => {
  // O portão primeiro, e com `await`: se este computador não está ativado, a
  // tela de compra precisa estar de pé antes de o console aparecer por um
  // quadro que seja.
  await montarPortao();

  ligarBarraDaJanela();

  // A DO PAINEL GIRA; AS DUAS GRANDES NÃO.
  //
  // No painel a esfera é um medidor pequeno ao lado do veredito, e o giro dela
  // carrega o uso de CPU. Nas telas de ativação e de chegada ela é imagem
  // grande, quase mil pixels, e ali quem informa é a EROSÃO — o casco furado
  // pela memória ocupada. Erosão não precisa de movimento para ser vista.
  //
  // Trocar o giro pela densidade foi a única forma de a imagem grande ler como
  // pedra em vez de chuvisco sem pôr um laço de sessenta quadros por segundo
  // com dezenas de milhares de pontos na máquina fraca que o Otimiza existe
  // para consertar. Parada, ela é redesenhada uma vez por medição.
  esfera = new Esfera(element<HTMLCanvasElement>("veredito-esfera"));

  // As duas imagens grandes. `dissolve` diz para que lado a imagem some no
  // preto: na chegada ela ocupa a esquerda e precisa se desfazer na direção do
  // texto, para os dois não ficarem separados por uma borda reta.
  pilaresDoPortao = new Pilares(element<HTMLCanvasElement>("portao-pilares"));

  pilaresDaChegada = new Pilares(element<HTMLCanvasElement>("chegada-pilares"), {
    dissolve: "direita",
  });

  // Em desenvolvimento a esfera fica alcancavel pelo console, para dar para
  // conferir o desenho com valores escolhidos a mao. Em producao o app roda de
  // `tauri://`, e esta linha nao acontece.
  if (location.hostname.startsWith("localhost")) {
    (window as unknown as { esfera?: Esfera }).esfera = esfera;
  }

  wireControls();
  ligarSubabas();
  conferirInvariantesDaTela();

  // O VEREDITO VEM PRIMEIRO — e sem `await`, de propósito.
  //
  // Ele é a coisa mais importante da tela: é o que responde "o que há de errado
  // com este PC" sem exigir um clique. Se entrasse na fila de carregamento
  // abaixo, seria a última coisa a aparecer, atrás de dez chamadas que o
  // cliente nem estava esperando. Disparar aqui e deixar solto faz o cartão se
  // preencher enquanto o resto da tela monta.
  void carregarVeredito();

  await ajustarMovimento();
  await listenToBatchProgress();

  // O vigia do modo jogo age sozinho em segundo plano. Quando ele mexe em
  // alguma coisa, a tela precisa contar — mudança silenciosa no sistema é
  // exatamente o que este produto critica nos outros.
  await listen<string>("gamemode:changed", (evento) => {
    setStatus("gamemode-status", evento.payload, "ok");
    void loadGameMode();
    void loadOptimizations();
    // O vigia é quem suspende e devolve programas em segundo plano; é ele
    // quem sabe quando o bloco precisa aparecer ou sumir sozinho.
    void carregarCongelados();
  });
  // As preferências vêm antes de tudo: elas decidem o intervalo de medição e o
  // que a lista mostra.
  await loadPreferences();

  // Depois das preferências carregadas, porque a decisão de mostrar depende
  // dos dois campos que acabaram de chegar do backend.
  checarReconsentimentoDoModoJogo();

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
    loadGameMode(),
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
  let maquinaApertada = false;

  try {
    const perfil = await invoke<{ total_ram_gb: number; logical_cores: number }>(
      "get_hardware_profile"
    );
    // Os limites sao o proprio publico-alvo do produto: 4 GB e 2 nucleos e a
    // maquina que o dono descreve como "PC fraco".
    maquinaFraca = perfil.total_ram_gb <= 4.5 || perfil.logical_cores <= 2;

    // O degrau do meio: maquina que aguenta movimento, mas nao merece um
    // fundo deslizando na velocidade cheia enquanto o cliente joga.
    maquinaApertada = !maquinaFraca && (perfil.total_ram_gb <= 8.5 || perfil.logical_cores <= 4);
  } catch {
    // Sem perfil, o padrao e animar. Errar para o lado de nao piorar a
    // aparencia de quem tem maquina boa.
  }

  const parado = sistemaPedeCalma || maquinaFraca;
  document.body.classList.toggle("sem-animacao", parado);

  // A ESFERA OBEDECE AO MESMO INTERRUPTOR.
  //
  // Num PC fraco ela desenha um quadro e para. É a mesma imagem, e o custo é
  // pago uma vez — um otimizador que engasga na própria interface se desmente
  // antes de aplicar a primeira otimização.
  // Só a esfera tem laço para ligar. Os pilares são desenhados por medição.
  esfera?.ligar();

  // O multiplicador global de movimento.
  //
  // Ele existia desde a primeira versao da folha de estilo, com um comentario
  // dizendo que "o JavaScript zera isto" — e nada escrevia nele, nem nenhuma
  // regra o lia. Era um token morto fingindo ser um sistema.
  //
  // Agora ele vale de verdade: o fundo divide a duracao da propria animacao
  // por este numero, entao 0,35 transforma um ciclo de 90 segundos em um de
  // 257. O movimento continua existindo e para de chamar atencao.
  document.documentElement.style.setProperty(
    "--anim",
    parado ? "0" : maquinaApertada ? "0.35" : "1"
  );
}

function element<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

// ---------------------------------------------------------------------- abas

/**
 * Troca de aba. Só a seção escolhida fica visível; o cabeçalho com os sinais
 * vitais e o rodapé permanecem, então nunca se perde o contato com a máquina.
 */
/**
 * Guias dentro de um painel.
 *
 * Genérico de propósito: qualquer painel que precise responder a mesma
 * pergunta em mais de um lugar declara `.subabas` com botões `data-sub` e
 * blocos `.subpainel` com o mesmo `data-sub`, e funciona. O alternativo seria
 * uma função por painel, que é como uma tela ganha cinco jeitos diferentes de
 * trocar de conteúdo.
 *
 * O escopo é o painel, e não o documento: dois painéis com guias na mesma aba
 * não podem apagar as guias um do outro.
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
 * A ABA DE REPARO SÓ CARREGA QUANDO É ABERTA.
 *
 * Abrir custa `health::analyze()` (duas consultas que o produto anuncia em
 * outro lugar como "cerca de 5 segundos") e a leitura de um CBS.log que
 * costuma ter dezenas de megabytes. Na montagem, isso era cobrado de TODO
 * mundo que inicia o programa, inclusive de quem nunca abre a aba.
 *
 * O gancho fica em `showTab`, e não num ouvinte de clique no item da lateral,
 * porque a aba também se abre pelas setas do teclado e pela paleta de
 * comandos — e as três precisam carregar.
 *
 * Uma vez só: `carregarReparo` registra ouvintes de evento, e registrá-los de
 * novo a cada abertura duplicaria cada linha de andamento na tela.
 */
let reparoCarregado = false;

function showTab(name: string) {
  if (name === "reparo" && !reparoCarregado) {
    reparoCarregado = true;
    void carregarReparo();
  }

  document.querySelectorAll<HTMLElement>(".tab-panel").forEach((panel) => {
    panel.hidden = panel.id !== `tab-${name}`;
  });

  document.querySelectorAll<HTMLButtonElement>(".nav[data-tab]").forEach((item) => {
    const escolhida = item.dataset.tab === name;
    item.setAttribute("aria-selected", String(escolhida));

    // Nome e ícone do cabeçalho vêm do próprio item da navegação. Escrever
    // isso duas vezes faria os dois saírem de sincronia na primeira vez que
    // alguém renomeasse uma seção.
    if (!escolhida) return;

    const rotulo = item.querySelector(".nav-rotulo")?.textContent?.trim() ?? "";

    // O TÍTULO COPIA O DESENHO DA LATERAL, e não um nome que os dois teriam de
    // combinar. Antes cada lado tinha a sua cópia da forma em CSS: renomear ou
    // redesenhar um ícone exigia lembrar do outro, e esquecer não quebrava
    // nada — só deixava a tela mostrando dois desenhos diferentes para a mesma
    // seção. Copiando a referência, é impossível saírem de sincronia.
    const referencia = item
      .querySelector(".nav-icone use")
      ?.getAttribute("href");

    text("secao-nome", rotulo);
    text("trilha-atual", rotulo);

    if (referencia) {
      document
        .getElementById("secao-icone-uso")
        ?.setAttribute("href", referencia);
    }
  });
}

/**
 * Selo numérico na aba. Mostrar o número aqui transforma a navegação em
 * informação: dá para saber que há algo esperando sem abrir a seção.
 */
/**
 * Quantos problemas cada diagnóstico encontrou.
 *
 * POR QUE ISTO EXISTE
 *
 * O selo da aba Diagnóstico era escrito por quatro lugares diferentes, e três
 * deles SOBRESCREVIAM o valor em vez de somar. Quem carregasse por último
 * ganhava: um achado crítico de memória era apagado por um aviso menor de
 * prontidão, e a aba passava a exibir o número do último painel que respondeu
 * — que muda a cada abertura, conforme a ordem em que as chamadas voltam.
 *
 * O quarto lugar tentava somar lendo o próprio texto do selo de volta, o que
 * dependia de o firmware ter carregado antes e de ninguém ter zerado o selo no
 * meio. Também errado, só que de forma mais difícil de enxergar.
 *
 * Agora cada diagnóstico declara o que encontrou, com nome, e o selo é sempre
 * a soma de todos. Ordem de carregamento deixa de importar.
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

  // CONTAGEM QUE NÃO É NÚMERO ESCONDE A BOLINHA, EM VEZ DE VIRAR "NaN".
  //
  // `count <= 0` é falso para `NaN`, então uma medição que não veio passava
  // direto pela guarda e a bolinha aparecia com o texto "NaN" — ou, na lateral
  // recolhida, como uma bola sem nada dentro. Bolinha acesa é uma afirmação:
  // "esta seção tem tantos itens esperando por você". Sem número, ela afirma o
  // quê? O caminho honesto para uma contagem que falhou é não desenhar nada.
  const valido = Number.isFinite(count) && count > 0;

  badge.hidden = !valido;
  badge.textContent = valido ? String(Math.round(count)) : "";

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
    // Processador e placa de vídeo passaram a vir daqui na versão 0.13. Antes
    // só apareciam depois que o cliente clicasse em "Analisar" no diagnóstico
    // legado — que foi removido. A identidade da máquina não pode depender de
    // um clique: é o cabeçalho da tela.
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
  } catch (error) {
    text("ident-storage", "indisponível");
    text("ident-cpu", "indisponível");
    text("ident-gpu", "indisponível");
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

  // A ESFERA RECEBE A MEDIÇÃO.
  //
  // É o que separa ela de um enfeite: cada propriedade do desenho — quantos
  // pontos, o quanto vibram, quantos buracos, a cor — sai de um número que
  // acabou de ser lido desta máquina.
  const nivelAgora =
    (element("veredito").dataset.nivel as "ok" | "importante" | "critico") ?? "ok";

  alimentarEsferas({
    nucleos: metrics.cpu.per_core.length,
    cpu,
    memoria: metrics.ram.usage_percent,
    nivel: nivelAgora,
  });

  // OS TRÊS PILARES RECEBEM AS TRÊS MEDIÇÕES.
  //
  // É o que separa a imagem de um enfeite: a altura de cada ruína sai de um
  // número que acabou de ser lido desta máquina, e não de um gosto nosso.
  alimentarPilares({
    cpu,
    memoria: metrics.ram.usage_percent,
    disco: metrics.disk.usage_percent,
    nivel: nivelAgora,
  });

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

/**
 * Atualiza um medidor do topo.
 *
 * A COR VEM DO CSS, e nao de um `style.background` daqui. O motivo e concreto:
 * pintar tudo — inclusive o normal — fazia a cor perder o significado
 * justamente quando ela precisava avisar. Agora so o âmbar e o vermelho
 * carregam informacao, e quem decide isso e uma regra de folha de estilo que
 * da para ler num lugar so.
 */
function setBar(id: string, percent: number) {
  const bar = element(id);
  const valor = Math.min(100, Math.max(0, percent));

  bar.style.width = `${valor}%`;

  const medidor = bar.closest(".vital") as HTMLElement | null;
  if (!medidor) return;

  // Os mesmos degraus do resto do produto: 75 e 90.
  if (valor >= 90) medidor.dataset.nivel = "critico";
  else if (valor >= 75) medidor.dataset.nivel = "atencao";
  else delete medidor.dataset.nivel;
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

/**
 * A lista completa de achados, com o que foi medido em cada um.
 *
 * Substitui a nota de saúde de 0 a 100 que existia até a versão 0.12. Aquele
 * número era calculado por um módulo legado que não consultava nenhum dos
 * diagnósticos de verdade desta máquina, e era a coisa mais visível da tela —
 * a menos verdadeira ocupando o lugar de maior destaque.
 *
 * O cartão do Painel mostra o achado que decide; aqui ficam todos, para quem
 * quiser conferir item a item.
 */
async function runDiagnostic() {
  const button = element<HTMLButtonElement>("run-diagnostic");
  const target = element("diagnostic-result");

  button.disabled = true;
  target.innerHTML = `<p class="empty carregando">Analisando…</p>`;

  try {
    const v = await invoke<Veredito>("diagnostico_rapido");
    renderDiagnostic(v);
    // O cartão do Painel e esta lista saem da mesma coleta, então não podem
    // divergir na tela.
    aplicarVeredito(v);
  } catch (error) {
    target.innerHTML = `<p class="status error">${escapeHtml(String(error))}</p>`;
  } finally {
    button.disabled = false;
  }
}

function renderDiagnostic(v: Veredito) {
  const problemas = v.achados.filter((a) => a.severity !== "Ok");
  const conferidos = v.achados.filter((a) => a.severity === "Ok");

  const linhas = problemas
    .map(
      (a) => `
        <div class="bottleneck" data-severity="${a.severity}">
          <div class="bottleneck-title">${escapeHtml(a.title)}</div>
          <div class="bottleneck-detail">${escapeHtml(a.measured)}</div>
          ${a.advice ? `<div class="bottleneck-detail">${escapeHtml(a.advice)}</div>` : ""}
        </div>`
    )
    .join("");

  // "Nada encontrado" precisa vir com os números que sustentam a afirmação.
  // Sem eles é só uma tela vazia, e tela vazia parece programa quebrado.
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

// ------------------------------------------------------- rede e DNS

interface DnsMeasurement {
  id: string;
  name: string;
  servers: string;
  median_ms: number | null;
  failures: number;
  current: boolean;
}

interface NetAdapter {
  guid: string;
  name: string;
  dns: string;
  automatic: boolean;
}

interface NetworkReport {
  adapters: NetAdapter[];
  measurements: DnsMeasurement[];
  gain_ms: number | null;
  note: string;
}

let lastNetwork: NetworkReport | null = null;

async function analyzeNetwork() {
  const button = element<HTMLButtonElement>("analyze-network");
  button.disabled = true;
  setStatus("net-status", "Consultando cada servidor de DNS e cronometrando…", "progress");

  try {
    const report = await invoke<NetworkReport>("analyze_network");
    lastNetwork = report;
    renderNetwork(report);
  } catch (error) {
    setStatus("net-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

function renderNetwork(report: NetworkReport) {
  const medidos = report.measurements.filter((m) => m.median_ms !== null);
  const maisRapido = medidos.reduce<DnsMeasurement | null>(
    (melhor, m) => (!melhor || m.median_ms! < melhor.median_ms! ? m : melhor),
    null
  );

  text(
    "net-tag",
    report.gain_ms !== null && report.gain_ms >= 5
      ? `${report.gain_ms.toFixed(0)} ms a ganhar`
      : "sem ganho relevante"
  );

  const linhas = report.measurements
    .map((m, i) => {
      const tempo =
        m.median_ms === null
          ? `<span class="state-label">sem resposta</span>`
          : `<span class="finding-size">${m.median_ms.toFixed(0)} ms</span>`;

      // Só oferece trocar quando há ganho que valha, e nunca para o que já
      // está em uso. Botão para ganhar 2 ms seria venda, não conserto.
      const vale =
        !m.current &&
        m.failures === 0 &&
        m.median_ms !== null &&
        report.gain_ms !== null &&
        report.gain_ms >= 5 &&
        m === maisRapido;

      const acao = vale
        ? `<button class="btn btn-ghost" data-dns="${escapeHtml(m.servers)}">Usar este</button>`
        : m.current
          ? `<span class="state-label">em uso</span>`
          : "";

      return `
        <div class="startup" data-enabled="${m.current}" style="--i:${i}">
          <div class="startup-info">
            <span class="startup-name">${escapeHtml(m.name)}</span>
            <span class="startup-exe">${escapeHtml(m.servers)}${
              m.failures > 0 ? ` · ${m.failures} consulta(s) sem resposta` : ""
            }</span>
          </div>
          ${tempo}
          ${acao}
        </div>`;
    })
    .join("");

  element("net-result").innerHTML = linhas;
  setStatus("net-status", report.note, "warn");
}

// --------------------------------------------- cache de shader

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

// ------------------------------------------ prioridade permanente

async function fixPriority(enable: boolean) {
  const botoes = document.querySelectorAll<HTMLButtonElement>(
    "#fix-priority, #unfix-priority"
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

// ----------------------------------------------- prontidão

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

// ------------------------------------------------- gargalo

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

  // Só é problema quando há um limite identificado que dá para agir. Placa no
  // talo é boa notícia em jogo, e sem carga não é diagnóstico nenhum.
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

// ------------------------------------------------------ modo jogo

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
    // Sem backend o painel continua utilizável; só não mostra a situação.
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
    // Ligar ou desligar o modo jogo pode ter devolvido tudo que estava
    // congelado (desligar sempre devolve), então o bloco precisa acompanhar
    // sem esperar o próximo evento do vigia.
    await carregarCongelados();
  }
}

// ------------------------------------------- reconsentimento do modo jogo

/**
 * Mostra, uma única vez, o que o modo jogo automático realmente faz —
 * congela programas em segundo plano — para quem ligou a opção antes de o
 * texto explicar isso (o texto mudou na 1.1.2; quem ligou antes nunca leu a
 * versão nova). Sem isso, a primeira notícia que essa pessoa tem é abrir o
 * Gerenciador de Tarefas e ver "Suspenso" ao lado do Discord.
 *
 * As duas condições precisam se encontrar: `auto_game_mode` ligado E
 * `game_mode_avisado` ainda desligado. Instalação nova nunca bate as duas —
 * `auto_game_mode` já nasce desligado — então a tela nunca aparece nela.
 */
function checarReconsentimentoDoModoJogo() {
  if (preferences.auto_game_mode && !preferences.game_mode_avisado) {
    element("gamemode-reconsent-modal").hidden = false;
    element<HTMLButtonElement>("reconsent-manter").focus();
  }
}

function fecharReconsentimentoDoModoJogo() {
  element("gamemode-reconsent-modal").hidden = true;
}

/** "Manter": a opção continua ligada, só registra que a pessoa já viu o aviso. */
async function reconsentirMantendo() {
  await savePreferences({ game_mode_avisado: true });
  fecharReconsentimentoDoModoJogo();
}

/**
 * "Desligar": desliga a opção e devolve, agora, qualquer programa que esteja
 * congelado neste exato momento — sem isso a pessoa desligaria o modo jogo e
 * o Discord continuaria "Suspenso" até o jogo fechar sozinho, o que
 * contradiz o próprio botão que ela acabou de apertar. Reaproveita
 * `descongelar_agora`, o mesmo comando do botão "Descongelar agora" da aba
 * Sistema — é o único caminho do produto que já faz exatamente isso.
 */
async function reconsentirDesligando() {
  await savePreferences({ auto_game_mode: false, game_mode_avisado: true });

  try {
    await invoke<number>("descongelar_agora");
  } catch {
    // Sem backend (ou nada congelado) a tela segue utilizável; o painel de
    // congelados abaixo reflete o estado real de qualquer jeito.
  }

  await loadGameMode();
  await carregarCongelados();
  fecharReconsentimentoDoModoJogo();
}

// ---------------------------------------------- congelados pelo modo jogo

interface Congelado {
  pid: number;
  nome: string;
  visivel: string;
  inicio: number;
}

/**
 * Mostra o que está congelado agora, e some sozinho quando não há nada.
 *
 * Existe porque um cliente abriu o Gerenciador de Tarefas, viu "Steam —
 * Suspenso", depois Discord, depois Chrome — e a tela do Otimiza não
 * mostrava nada disso e não tinha botão nenhum. Ele concluiu que o produto
 * tinha quebrado a máquina dele; não estava errado. Segue o mesmo padrão de
 * `carregarMonitores`: busca, e o próprio resultado decide se o bloco
 * aparece.
 */
async function carregarCongelados() {
  const bloco = element("congelados");
  const lista = element("congelados-lista");

  try {
    const congelados = await invoke<Congelado[]>("congelados_agora");

    // Nada congelado é o caso comum — o jogo nem sempre está aberto, e
    // muitos jogos não têm nada para suspender. Um bloco vazio permanente é
    // ruído, e ruído é o que faz o cliente parar de ler a tela.
    bloco.hidden = congelados.length === 0;

    if (congelados.length === 0) {
      return;
    }

    lista.innerHTML = congelados
      .map((c) => `<li class="congelados-item">${escapeHtml(c.visivel)}</li>`)
      .join("");
  } catch {
    // Sem backend o painel continua utilizável; o bloco só não aparece.
    bloco.hidden = true;
  }
}

/**
 * Descongela tudo agora, sem esperar o jogo fechar e sem mexer no plano de
 * energia — descongelar não é a mesma coisa que desligar o modo jogo.
 */
async function descongelarAgora() {
  const botao = element<HTMLButtonElement>("descongelar-agora");
  botao.disabled = true;

  try {
    const quantos = await invoke<number>("descongelar_agora");
    setStatus(
      "gamemode-status",
      quantos > 0
        ? `Descongelei ${quantos} programa${quantos === 1 ? "" : "s"}.`
        : "Não havia nada congelado.",
      "ok"
    );
  } catch (error) {
    setStatus("gamemode-status", String(error), "error");
  } finally {
    botao.disabled = false;
    await carregarCongelados();
  }
}

// ------------------------------------------------- quadros por segundo

interface FrameMeasurement {
  fps: number;
  frames: number;
  seconds: number;
  process: string;
  pid: number;
}

async function measureFrames() {
  const button = element<HTMLButtonElement>("measure-frames");
  const processo = element<HTMLInputElement>("fps-process").value.trim();

  if (!processo) {
    setStatus("fps-status", "Diga o nome do processo do jogo.", "error");
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
    element("fps-result").innerHTML = `
      <div class="readouts readouts-row">
        <div class="readout">
          <span class="readout-label">Quadros por segundo</span>
          <span class="readout-value">${m.fps.toFixed(1)}</span>
          <span class="readout-note">média da janela medida</span>
        </div>
        <div class="readout">
          <span class="readout-label">Quadros contados</span>
          <span class="readout-value">${m.frames}</span>
          <span class="readout-note">em ${m.seconds.toFixed(1)} s</span>
        </div>
        <div class="readout">
          <span class="readout-label">Processo</span>
          <span class="readout-value">${escapeHtml(m.process)}</span>
          <span class="readout-note">pid ${m.pid}</span>
        </div>
      </div>`;

    setStatus(
      "fps-status",
      `${m.frames} quadros em ${m.seconds.toFixed(1)} segundos. Meça de novo depois de ` +
        `otimizar, na mesma cena do jogo — comparar cena diferente não diz nada.`,
      "ok"
    );
  } catch (error) {
    // O módulo devolve erro em vez de zero quando não consegue contar. A
    // mensagem já explica o motivo, então vai inteira para a tela.
    setStatus("fps-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

// --------------------------------------------------------------- FiveM

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

  // Amarelo, não verde: há espaço a recuperar e há uma contrapartida a ler.
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

  // Pasta protegida não ganha botão. Ela existe na lista para explicar por que
  // o espaço não foi recuperado — omitir faria parecer que estamos escondendo.
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

async function prioritizeFiveM() {
  const button = element<HTMLButtonElement>("prioritize-fivem");
  button.disabled = true;

  try {
    const mensagem = await invoke<string>("prioritize_fivem");
    setStatus("fivem-status", mensagem, "ok");
  } catch (error) {
    setStatus("fivem-status", String(error), "error");
  } finally {
    button.disabled = false;
  }
}

// ----------------------------------------------------------- navegador

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

  // O número que o cliente sente ao fechar o navegador. É do navegador
  // inteiro, e não por extensão — ver o comentário no módulo em Rust.
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

  // Limpar só faz sentido com o navegador fechado, e o botão precisa dizer o
  // motivo em vez de simplesmente não funcionar.
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

    registrarProblemas("saude", problemas, critico);
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

  registrarProblemas("memoria", problemas, critico);
}

// -------------------------------------------------------------- otimizações

/**
 * O veredito da máquina.
 *
 * Roda sozinho ao abrir, sem clique, e é a primeira coisa que o cliente lê.
 *
 * A versão anterior do produto exigia dezessete botões de análise espalhados
 * por cinco abas para montar essa resposta na cabeça do usuário — e ele nunca
 * montava. Numa máquina de 8 GB que travava o PC inteiro ao abrir o jogo, a
 * tela dizia "memória e paginação sem problemas", porque cada pedaço da
 * verdade morava num painel diferente e nenhum deles era o veredito.
 */
async function carregarVeredito() {
  try {
    aplicarVeredito(await invoke<Veredito>("diagnostico_rapido"));
  } catch (error) {
    // Falhar aqui também é uma informação. O cartão não pode ficar dizendo
    // "analisando" para sempre, nem passar a impressão de que deu tudo certo.
    const cartao = element("veredito");
    delete cartao.dataset.nivel;
    text("veredito-rotulo", "Não foi possível diagnosticar");
    text("veredito-frase", "O diagnóstico automático não completou.");
    text("veredito-detalhe", String(error));
    text("veredito-conselho", "");
    mostrarAcaoDoVeredito(null);
  }
}

/**
 * Pinta o cartão do veredito.
 *
 * Separado da busca porque duas telas usam o mesmo resultado — o cartão do
 * Painel e a lista da aba Diagnóstico. Se cada uma coletasse por conta própria,
 * elas poderiam mostrar coisas diferentes sobre a mesma máquina, no mesmo
 * momento, e o cliente não teria como saber em qual acreditar.
 */
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

  // Os achados da mesma causa, juntos. Antes viviam em abas separadas e
  // nunca se encontravam na tela — é isto que estava quebrado no produto.
  const junto = element("veredito-junto");
  junto.hidden = v.corroboracoes.length === 0;
  junto.innerHTML = v.corroboracoes
    .map(
      (c) =>
        `<li><strong>${escapeHtml(c.title)}</strong> — ${escapeHtml(c.measured)}</li>`
    )
    .join("");

  mostrarAcaoDoVeredito(v.principal?.acao ?? null);

  // E, quando o portão está de pé, o mesmo achado aparece na tela de compra.
  mostrarAchadoNoPortao(v);

  // A esfera acompanha o veredito: neutra, âmbar ou vermelha. Sem esperar a
  // próxima leitura do monitor — o diagnóstico é a informação mais importante
  // da tela e não pode chegar em duas velocidades, e uma frase vermelha ao lado
  // de uma esfera neutra faz o cliente duvidar das duas.
  esfera?.definirNivel(nivel as "ok" | "importante" | "critico");
  esfera?.redesenhar();

  pilaresDoPortao?.definirNivel(nivel as "ok" | "importante" | "critico");
  pilaresDaChegada?.definirNivel(nivel as "ok" | "importante" | "critico");

  // O aviso que segue visível em qualquer aba. Só para crítico: se aparecesse
  // também nos importantes, viraria enfeite permanente e pararia de ser lido —
  // que é o destino de todo alerta que está sempre ligado.
  const alerta = element("alerta-global");
  alerta.hidden = nivel !== "critico";

  if (nivel === "critico") {
    text("alerta-global-texto", v.frase);
    alerta.onclick = () => showTab("painel");
  }

  // E o que não deu para verificar, dito em voz alta. Silêncio aqui seria
  // indistinguível de aprovação.
  const lacunas = element("veredito-lacunas");
  lacunas.hidden = v.lacunas.length === 0;
  lacunas.innerHTML = v.lacunas
    .map(
      (l) => `<li>${escapeHtml(l.o_que)}: ${escapeHtml(l.por_que)}</li>`
    )
    .join("");
}

/**
 * O botão de conserto do veredito.
 *
 * Aparece só quando o Otimiza sabe resolver aquilo sozinho — o que, neste
 * produto, é a minoria dos casos. Falta de memória, disco morrendo e
 * configuração de BIOS não ganham botão: nenhum programa acrescenta um pente,
 * e inventar um botão ali seria prometer o que não se cumpre.
 */
function mostrarAcaoDoVeredito(acao: Acao | null) {
  const bloco = element("veredito-acao");
  const botao = element<HTMLButtonElement>("veredito-corrigir");

  bloco.hidden = acao === null;
  if (!acao) return;

  botao.textContent = acao.rotulo;
  botao.disabled = false;

  // Dizer que exige administrador ANTES do clique. Descobrir depois, por uma
  // mensagem de erro, faz o cliente achar que o programa não funciona.
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
      // Rediagnostica: o cartão precisa refletir o que acabou de mudar, e não
      // continuar mostrando um problema que já foi resolvido.
      await carregarVeredito();
    } catch (error) {
      text("veredito-acao-nota", String(error));
      botao.disabled = false;
    }
  };
}

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

  // O QUE ESPERAR DA LISTA, ANTES DE APLICAR NADA.
  //
  // Sem isto, o cliente conta itens: aplica trinta e espera trinta vezes o
  // resultado. Só sete tocam FPS de forma mensurável — e é melhor ele saber
  // disso antes de clicar do que depois de jogar.
  const mudamFps = optimizations.filter(
    (item) => item.expected_gain === "Measurable" && item.state === "Available"
  ).length;
  const naoMudamFps = optimizations.filter(
    (item) => item.expected_gain === "Responsiveness" && item.state === "Available"
  ).length;

  // Os que não prometem desempenho nenhum são contados à parte, e em voz alta.
  // Eles existem na lista porque o cliente compara catálogo com catálogo, e
  // esconder que não fazem nada seria usar o tamanho da lista como argumento de
  // venda — que é exatamente o que os concorrentes fazem.
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

  // Duas ordens, nesta sequência:
  //
  // 1. O que muda o FPS vem antes do que não muda. Das 35 otimizações do
  //    catálogo, 17 são higiene de Windows que devolve algumas centenas de MB
  //    e não toca em FPS — e antes apareciam misturadas com as 7 que mudam,
  //    todas com o mesmo peso visual. Um cliente que aplica 30 itens espera 30
  //    vezes o resultado, e recebe o de 7. A ordem agora conta essa verdade
  //    antes de ele clicar.
  // 2. Dentro do mesmo nível, o que pesa NESTA máquina sobe.
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
  element<HTMLInputElement>("pref-gamemode").checked = preferences.auto_game_mode;

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
  /** Falso quando o Edge não estava disponível e só saiu o HTML. */
  is_pdf: boolean;
  optimizations: number;
  changes: number;
  note: string;
}

/** Última comparação medida nesta sessão, ou nada se ainda não houve. */
let lastComparison: BenchmarkComparison | null = null;

async function exportReport() {
  const button = element<HTMLButtonElement>("export-report");
  button.disabled = true;
  // O levantamento consulta WMI e log de eventos; passa de dez segundos.
  setStatus(
    "report-status",
    "Levantando o estado da máquina e montando o PDF… pode levar meio minuto.",
    "progress"
  );

  try {
    const saved = await invoke<ReportSaved>("export_report", {
      comparison: lastComparison,
    });

    // O caminho completo importa: o técnico precisa achar o arquivo para
    // anexar num e-mail ou copiar para um pendrive.
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

// ------------------------------------------------------ paleta de comandos

/**
 * Toda ação do programa, buscável por nome.
 *
 * O aplicativo passou de quarenta botões espalhados por sete seções. Quem usa
 * isto todos os dias sabe o nome do que quer e não deveria precisar lembrar em
 * qual aba ele mora — caçar botão é o gesto mais repetido e mais chato de um
 * console cheio.
 *
 * A lista é montada a partir do próprio HTML, e não escrita à mão: um botão
 * novo entra na paleta sozinho, e nenhum fica para trás porque alguém esqueceu
 * de cadastrar.
 */
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

  // Todo botão de ação de dentro dos painéis. Os botões que a interface gera
  // por linha — limpar esta pasta, desligar este serviço — ficam de fora de
  // propósito: eles só fazem sentido junto do item a que pertencem.
  document.querySelectorAll<HTMLButtonElement>(".tab-panel .btn").forEach((botao) => {
    const rotulo = botao.textContent?.trim();
    const painel = botao.closest<HTMLElement>(".tab-panel");

    if (!rotulo || !painel || botao.hasAttribute("data-fivem")) return;

    const aba = painel.id.replace("tab-", "");

    // O rótulo do BOTÃO e o rótulo do COMANDO deixaram de ser a mesma coisa.
    //
    // Os dezoito botões de exame passaram a se chamar todos "Analisar", porque
    // o título do painel logo acima já diz o assunto e repetir a palavra a
    // quarenta pixels de distância era ruído. Na paleta, porém, dezoito linhas
    // idênticas seriam inúteis — lá o assunto precisa vir junto.
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
    // Montada na hora de abrir: painéis carregam conteúdo depois do início, e
    // uma lista montada uma vez só ficaria desatualizada.
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

  // Um caminho so para a busca. O botao da lateral saiu: a mesma acao em dois
  // lugares da tela obriga a pessoa a escolher entre portas identicas.
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

  // Clicar fora fecha. Sem isso a paleta vira uma janela presa que só some
  // com o teclado.
  caixa.addEventListener("click", (event) => {
    if (event.target === caixa) fechar();
  });
}

// ---------------------------------------------------------------- controles


/* -------------------------------------------------- a aba de reparo */

/**
 * O que o backend devolve para cada ferramenta oferecida nesta máquina —
 * `FerramentaDeReparo`, em `commands.rs`. Duração típica, se cancelar é
 * seguro, e os dois avisos de segurança (`aviso` e `aviso_reset_base`) têm
 * UMA fonte só: `Receita`, do lado do Rust
 * (`src-tauri/src/modules/windows/reparo.rs`). A tela não reescreve nenhum
 * deles — só lê.
 */
interface FerramentaDeReparo {
  nome: string;
  minutos_tipicos: readonly [number, number];
  cancelar_e_seguro: boolean;
  aviso: string | null;
  oferece_reset_base: boolean;
  aviso_reset_base: string | null;
}

/**
 * O tom de `UltimoResultadoReparo`, já decidido pelo backend a partir do
 * dado estruturado (`ResultadoSfc::severidade()`), nunca por um prefixo de
 * texto — é o que fecha o buraco em que `CorrigiuEmParte` (que ainda deixa
 * corrupção na máquina) e `Corrigiu` (sucesso total) tinham a mesma cor
 * porque as duas frases começam com "Corrigiu ".
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

/**
 * O desfecho de `reparo_executar`, mesma forma de `UltimoResultadoReparo` e
 * pelo mesmo motivo: o tom nasce no backend, a partir da variante de
 * `Desfecho` (Rust), nunca da frase de `texto`. A tela costumava decidir a
 * cor comparando o texto formatado (`desfecho === "Terminou."`) — e, como
 * `CorrigiuEmParte` já provou uma vez, frase e cor divergem. Não existe mais
 * essa comparação: só o `tom` é lido.
 */
type DesfechoReparo = UltimoResultadoReparo;

/**
 * De onde uma linha de andamento veio: `stdout` ou `stderr` do processo.
 *
 * O `stderr` é drenado numa thread separada, no Rust, e caía misturado ao
 * progresso: a razão de uma falha do DISM — "precisa de internet" contra "a
 * imagem está corrompida" — chegava embaralhada no meio de centenas de
 * linhas de percentagem, e nada na tela permitia diferenciar uma da outra.
 */
type OrigemAndamento = "saida" | "erro";

interface Andamento {
  linha: string;
  numero: number;
  origem: OrigemAndamento;
}

/**
 * Título e descrição de cada ferramenta — texto de APRESENTAÇÃO, escrito
 * pela própria tela. Isto fica aqui de propósito, e não é a mesma dívida que
 * os avisos de segurança tinham: nenhum destes dois campos muda o risco de
 * um clique, então não precisam de dono único no backend — só a duração, o
 * aviso e o `/ResetBase` precisavam, e esses três agora vêm de
 * `reparo_disponivel()`.
 */
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
  // Este item SÓ APARECE depois de "Verificar o disco" ter achado alguma
  // coisa — quem decide isso é o backend, em `EstadoDoDisco`. A descrição
  // antiga ("Corrige os erros que a verificação encontrou no disco") ficava na
  // tela desde a primeira abertura, afirmando uma medição que nunca tinha
  // acontecido, num produto cuja regra fundadora é não mostrar número que não
  // foi medido.
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

/**
 * Monta um item da lista de reparo: título e descrição (texto da tela),
 * duração típica e avisos (dados do backend), e — só quando
 * `oferece_reset_base` vem `true` — o interruptor do `/ResetBase`, DESLIGADO
 * por padrão e com o aviso ao lado dele, não numa nota de rodapé.
 */
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

/**
 * O andamento vem por evento, e não como retorno da chamada.
 *
 * Um `DISM` leva de dez a trinta minutos. Esperar o retorno para só então
 * mostrar alguma coisa é o mesmo que não ter andamento — e é no minuto oito,
 * parado em 20%, que o cliente conclui que travou e desliga a máquina.
 */
/**
 * Quantas linhas de saída `#reparo-saida` guarda, no máximo.
 *
 * Um `DISM /RestoreHealth` de trinta minutos redesenha a mesma linha de
 * percentagem centenas de vezes (0%, 1%, 2%, ... cada `\r` vira uma linha —
 * ver `drenar` em `tarefa_longa.rs`), e ainda tem mais de um estágio. Sem
 * teto, esse elemento único acumularia milhares de NÓS de DOM na aba pelo
 * resto da execução, para nada: ninguém rola de volta para ver "43%" de
 * novo. O teto limita a QUANTIDADE de nós, não o volume de caracteres — uma
 * única linha gigante não é cortada por ele, e o `<pre>` continuaria
 * crescendo com ela. Isso é aceitável aqui: `sfc` e `DISM` não escrevem
 * linha gigante, escrevem muitas linhas curtas. 500 sobra até para as duas
 * barras de progresso do DISM (scan + restore, uns 200 cada) mais as linhas
 * de texto de verdade em volta, e ainda cabe folgado numa área de rolagem de
 * 220px sem virar um arquivo de log. O corte é sempre do INÍCIO — mantém o
 * FIM, que é onde o resultado está.
 */
const MAX_LINHAS_SAIDA = 500;

async function carregarReparo() {
  const lista = element("reparo-lista");
  const saida = element("reparo-saida");
  const cancelar = element<HTMLButtonElement>("reparo-cancelar");

  /**
   * Acrescenta uma linha de andamento a `#reparo-saida`, destacando as que
   * vieram do `stderr` — ver `OrigemAndamento`. Cada linha é um `<span>`
   * seguido de uma quebra: um `<pre>` preserva essa quebra como texto, e o
   * `<span>` é o que permite colorir só aquela linha sem tocar nas outras.
   */
  function acrescentarLinhaSaida(a: Andamento) {
    const linha = document.createElement("span");
    linha.textContent = a.linha;
    if (a.origem === "erro") {
      linha.className = "reparo-linha-erro";
    }
    saida.appendChild(linha);
    saida.appendChild(document.createTextNode("\n"));

    // Mantém só as últimas `MAX_LINHAS_SAIDA`, cortando do início — ver o
    // comentário da constante. Cada linha é dois nós (o `<span>` e a
    // quebra), então os dois somem juntos.
    while (saida.children.length > MAX_LINHAS_SAIDA) {
      saida.removeChild(saida.firstChild!);
      if (saida.firstChild) {
        saida.removeChild(saida.firstChild);
      }
    }
  }

  const ferramentasPorNome = new Map<string, FerramentaDeReparo>();

  /** A ferramenta em execução, ou `null`. Quem responde se cancelar é seguro. */
  let rodandoAgora: FerramentaDeReparo | null = null;

  /**
   * "Nenhuma corrupção encontrada" é o resultado mais comum, e é um resultado
   * BOM — a tela precisa dizer isso com a mesma cor que usa para sucesso, e
   * não com o cinza neutro que usaria para "não sei dizer". O tom vem pronto
   * do backend (`UltimoResultadoReparo.tom`); a tela só traduz para a classe
   * CSS que `setStatus` espera.
   *
   * ESTA LINHA FALA DO `sfc`, E DE MAIS NADA. Ela era repintada no `finally`
   * de TODA execução: o cliente rodava "Reparar a imagem do Windows", o DISM
   * falhava por falta de internet, e a linha mais destacada do painel
   * repintava em verde "Nenhuma corrupção encontrada" — um veredito do `sfc`,
   * lido naturalmente como o resultado do que acabara de rodar. Agora só o
   * `VerificarArquivos` a atualiza, e a legenda ao lado dela (no `index.html`)
   * diz de que verificação ela está falando.
   */
  async function atualizarUltimoResultado() {
    const resultado = await invoke<UltimoResultadoReparo>("reparo_ultimo_resultado");
    setStatus("reparo-resultado", resultado.texto, tomParaStatus(resultado.tom));
  }

  /**
   * Redesenha a lista a partir do que o backend oferece AGORA.
   *
   * Precisa acontecer depois de cada execução, e não só na abertura: o
   * "Consertar a estrutura do disco" só existe depois de um `/scan` ter achado
   * alguma coisa, e o "Desmarcar" só existe enquanto há conserto agendado.
   * Quem decide os dois é o backend — a tela apenas volta a perguntar.
   */
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

    // O Interromper só aparece para quem tem o que interromper. Antes ele
    // aparecia para toda ferramenta, inclusive as que já tinham terminado o
    // trabalho no primeiro segundo.
    cancelar.hidden = f === null;
    lista.querySelectorAll<HTMLButtonElement>("[data-reparo]").forEach((botao) => {
      botao.disabled = f !== null;
    });
    text("reparo-tag", f !== null ? "rodando…" : "pronto");
  }

  async function executarFerramenta(nome: string) {
    const f = ferramentasPorNome.get(nome);
    const titulo = TEXTOS_REPARO[nome]?.titulo ?? nome;

    // `sfc`, `DISM`, `fsutil`, `chkntfs` e a leitura do CBS.log exigem
    // administrador. Sem isto o cliente recebia um vermelho seco "Terminou com
    // o código 1", sem explicação e sem oferta de reabrir com permissão — e o
    // produto já tem o padrão da casa para isso.
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
      // `resetbase`, uma palavra só — o mesmo nome que `reparo_executar`
      // espera do lado do Rust, sem depender da conversão de caixa entre
      // JavaScript e Rust para um interruptor desta consequência.
      const desfecho = await invoke<DesfechoReparo>("reparo_executar", {
        ferramenta: nome,
        resetbase: resetarBase,
      });

      setStatus("reparo-execucao", desfecho.texto, tomParaStatus(desfecho.tom));
    } catch (error) {
      setStatus("reparo-execucao", String(error), "error");
    } finally {
      definirRodando(null);

      // Só o `sfc` escreve o veredito que esta linha mostra. Ver o comentário
      // de `atualizarUltimoResultado`.
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
    // `cancelar_e_seguro` atravessava o IPC e não era lido em lugar nenhum: o
    // clique cancelava sem perguntar, inclusive no DISM e na limpeza do
    // WinSxS, onde uma interrupção no meio de uma escrita pode deixar operação
    // pendente. A especificação é explícita: "O botão de cancelar diz isso
    // antes de aceitar o clique."
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

/* -------------------------------------------------- os monitores, desenhados */

interface MonitorLido {
  dispositivo: string;
  descricao: string;
  principal: boolean;
  largura: number;
  altura: number;
  hz_atual: number;
  hz_disponiveis: number[];
}

/**
 * Desenha cada monitor com a taxa dele DENTRO da tela.
 *
 * "60 Hz num monitor que aceita 180" é o achado mais fácil de ignorar do
 * produto: passa batido porque não dói. Só que é a maior diferença de fluidez
 * que existe num PC, e a única que se sente antes de abrir qualquer jogo.
 *
 * Numa máquina com dois monitores, a frase sozinha nunca dizia QUAL estava
 * errado. Desenhados lado a lado, isso deixa de ser um problema.
 */
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
              <text class="monitor-unidade" x="100" y="82">HZ</text>
              <rect class="monitor-pe" x="88" y="118" width="24" height="16" rx="2" />
              <rect class="monitor-base" x="62" y="134" width="76" height="8" rx="4" />
            </svg>
            <div>
              <p class="monitor-nome">${escapeHtml(m.descricao)}${m.principal ? " · principal" : ""}</p>
              <p class="monitor-detalhe">${m.largura}×${m.altura}${
                estaAbaixo ? ` · aceita ${maximo} Hz` : " · no máximo"
              }</p>
            </div>
          </div>`;
      })
      .join("");
  } catch {
    lista.innerHTML = '<p class="hint">Não consegui ler os monitores.</p>';
  }
}

/* --------------------------------------------------- a memória, desenhada */

interface MemoriaInstalada {
  slots: number | null;
  pentes_gb: number[];
  canais: number;
  mhz: number | null;
}

/**
 * Desenha os encaixes de memória da placa-mãe, cheios e vazios.
 *
 * "Canal único" é jargão: ninguém que não monta PC sabe o que significa, e a
 * frase sozinha some no meio do diagnóstico. Quatro encaixes com um ocupado e
 * três vazios não precisam de tradução.
 *
 * Os slots são desenhados aqui, e não fixos no HTML, porque a quantidade vem da
 * máquina — duas em notebook, quatro na maioria dos desktops. Desenhar quatro e
 * esconder os que sobram mostraria encaixes que aquela placa não tem.
 */
function desenharMemoria(m: MemoriaInstalada) {
  const total = m.slots ?? Math.max(m.pentes_gb.length, 1);
  const grupo = element("memoria-slots");

  // A largura de cada encaixe sai do espaço disponível dividido pelo número
  // real de slots: dois encaixes largos numa placa de notebook, quatro
  // estreitos num desktop, e nunca um desenho que estoura a moldura.
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

      // Os chips do pente. Quatro por lado é o que cabe legível nesta escala —
      // não é a contagem real, e não pretende ser: é a silhueta de um pente.
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

    // As travas das pontas existem nos dois casos: é o que faz o vazio parecer
    // um encaixe esperando um pente, e não um retângulo qualquer.
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

    // CANAL ÚNICO COM ENCAIXE LIVRE É O ACHADO, e é o único caso em que o
    // desenho muda de cor. Um pente sozinho numa placa que só tem um slot não
    // é problema — é o máximo que aquela máquina aceita, e acusar seria vender
    // conserto de coisa que não tem conserto.
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

/* ------------------------------------------------------ a placa de vídeo */

interface PlacaDeVideo {
  marca: string;
  nome: string | null;
  driver: string | null;
  driver_data: string | null;
  driver_dias: number | null;
  vram_gb: number;
}

/**
 * Desenha a placa que a máquina TEM, em vez de perguntar qual é.
 *
 * O concorrente abre pedindo para escolher entre AMD e NVIDIA. Perguntar o que
 * o produto já leu é fazer o cliente trabalhar de graça, e ainda arrisca ele
 * escolher errado — e a partir daí tudo que a tela mostrar estará baseado numa
 * escolha ruim.
 *
 * A escolha manual existe, escondida, e só aparece quando a leitura falha. Aí
 * ela deixa de ser trabalho inútil e passa a ser a única saída.
 */
async function carregarPlaca() {
  const painel = element("placa-painel");

  try {
    const p = await invoke<PlacaDeVideo>("placa_de_video");

    painel.dataset.marca = p.marca;
    text("placa-marca", p.marca === "desconhecida" ? "placa de vídeo" : p.marca);
    text("placa-nome", p.nome ?? "Não consegui identificar a placa");
    text("placa-driver", p.driver ?? "—");

    // A IDADE DO DRIVER VEM COM A DATA, e não sozinha.
    //
    // "41 dias" sem a data obriga o cliente a confiar na nossa conta. Com as
    // duas, ele confere no Gerenciador de Dispositivos em dez segundos.
    text(
      "placa-driver-idade",
      p.driver_data
        ? `${p.driver_data}${p.driver_dias !== null ? ` · há ${p.driver_dias} dias` : ""}`
        : "—",
    );

    text("placa-vram", p.vram_gb > 0 ? `${p.vram_gb.toFixed(0)} GB` : "não sei dizer");

    // Só pergunta quando não sabe.
    element("placa-escolha").hidden = p.marca !== "desconhecida";
  } catch {
    // Falhar aqui não pode derrubar a aba: o painel é contexto, e os dois
    // painéis abaixo dele — que são os que mudam FPS — continuam funcionando.
    painel.dataset.marca = "desconhecida";
    text("placa-nome", "Não consegui ler a placa de vídeo");
    element("placa-escolha").hidden = false;
  }
}

/* ===========================================================================
   A CONFIGURAÇÃO DO JOGO, E A PROVA

   Este par de painéis é o que separa o produto do resto do mercado, e a razão
   é a hierarquia do ganho — medida, não achada:

       uma configuração de jogo mal escolhida ... dezenas de por cento
       memória insuficiente ..................... o teto da máquina
       ajustes de Windows, todos somados ........ alguns por cento

   O primeiro painel mexe onde o ganho mora. O segundo prova que mexeu.
   =========================================================================== */

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

/** chave, valor de agora, valor novo, o que se perde. */
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
    // Este perfil não pede confirmação de perda porque não há perda nenhuma.
    aviso: "",
  },
  equilibrado: {
    botao: "cfgjogo-equilibrado",
    nome: "Equilibrado",
    aviso: "Isto muda como o jogo se parece.",
  },
  competitivo: {
    botao: "cfgjogo-competitivo",
    nome: "Competitivo",
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

  // A lista do que está caro sai do diagnóstico que já existia — ele ranqueia
  // por custo real, e o MSAA vem primeiro porque sozinho custa mais que o
  // resto somado.
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

/**
 * Mostra o que o perfil MUDARIA, e só aplica depois do "sim".
 *
 * A confirmação lista chave, valor de agora, valor novo e o que se perde. É a
 * diferença entre o cliente aceitar uma mudança e aceitar a palavra "otimizar":
 * ele decide sobre linhas com nome e número, não sobre um botão.
 */
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
    // NADA A MUDAR NÃO É ERRO, E NÃO PODE PARECER UM.
    //
    // A configuração já estar do jeito que o perfil quer é a melhor notícia
    // possível: não há ganho escondido ali. Dizer isso é mais honesto que
    // aplicar e mostrar "pronto!" sem nada ter acontecido.
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

/* ------------------------------------------------------------------ a prova */

function segundosDaMedicao(): number {
  // Vinte segundos: menos que isso não dá amostra para o 1% pior significar
  // alguma coisa, e mais que isso o cliente não espera parado.
  return 20;
}

async function medirAntes() {
  const botao = element<HTMLButtonElement>("prova-antes");
  const processo = element<HTMLInputElement>("prova-processo").value.trim();

  if (!processo) {
    setStatus("prova-status", "Diga o nome do jogo antes de medir.", "error");
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

    // O ESTADO DO STATUS SEGUE O RESULTADO, e não a vontade de comemorar.
    // Uma medição que piorou não pode sair verde.
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
  // A ETIQUETA NUNCA MENTE SOBRE O SINAL.
  //
  // Um produto que só mostra ganho não está medindo, está anunciando — e o
  // cliente que confere sozinho descobre isso na pior hora.
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
  });

  // Setas percorrem as seções, como manda o padrão de acessibilidade para
  // navegação em abas — e é como quem usa teclado espera que funcione. Agora
  // é cima e baixo, porque a lista virou vertical.
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

  // Recolher a lateral. A escolha fica guardada: quem trabalha em tela pequena
  // não quer refazer isso toda vez que abre o programa.
  const corpo = document.querySelector<HTMLElement>(".corpo")!;
  const alternar = element<HTMLButtonElement>("toggle-lateral");

  if (localStorage.getItem("lateral-recolhida") === "sim") {
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
  element("analyze-browsers").addEventListener("click", analyzeBrowsers);
  element("analyze-fivem").addEventListener("click", analyzeFiveM);
  element("analyze-network").addEventListener("click", analyzeNetwork);
  element("analyze-bottleneck").addEventListener("click", analyzeBottleneck);
  element("analyze-shaders").addEventListener("click", analyzeShaders);
  for (const botao of document.querySelectorAll<HTMLButtonElement>("[data-marca-manual]")) {
    botao.addEventListener("click", () => {
      // A escolha manual pinta o desenho e mais nada. O produto não muda
      // NENHUM ajuste por causa dela: tudo que ele decide sobre vídeo vem de
      // medição, e um clique num botão não é medição.
      element("placa-painel").dataset.marca = botao.dataset.marcaManual ?? "desconhecida";
      text("placa-marca", botao.dataset.marcaManual ?? "placa de vídeo");
    });
  }

  void carregarPlaca();
  void carregarMemoria();
  void carregarMonitores();
  void carregarCongelados();
  element("cfgjogo-analisar").addEventListener("click", analisarConfigJogo);
  element("cfgjogo-sem-teto").addEventListener("click", () => aplicarPerfilDoJogo("sem_teto"));
  element("cfgjogo-equilibrado").addEventListener("click", () => aplicarPerfilDoJogo("equilibrado"));
  element("cfgjogo-competitivo").addEventListener("click", () => aplicarPerfilDoJogo("competitivo"));
  element("prova-antes").addEventListener("click", medirAntes);
  element("prova-depois").addEventListener("click", medirDepois);
  element("analyze-readiness").addEventListener("click", analyzeReadiness);
  element("fix-priority").addEventListener("click", () => fixPriority(true));
  element("unfix-priority").addEventListener("click", () => fixPriority(false));

  element("shader-result").addEventListener("click", async (event) => {
    const botao = (event.target as HTMLElement).closest(
      "button[data-shader]"
    ) as HTMLButtonElement | null;
    if (!botao) return;

    // A contrapartida vem antes da confirmação: a primeira partida recompila.
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
  element("descongelar-agora").addEventListener("click", () => descongelarAgora());
  element("measure-frames").addEventListener("click", measureFrames);

  element("flush-dns").addEventListener("click", async () => {
    const button = element<HTMLButtonElement>("flush-dns");
    button.disabled = true;

    try {
      setStatus("net-status", await invoke<string>("flush_dns"), "ok");
    } catch (error) {
      setStatus("net-status", String(error), "error");
    } finally {
      button.disabled = false;
    }
  });

  element("net-result").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-dns]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    if (!isElevated) {
      askForAdmin(
        "Trocar o servidor de DNS exige permissao de administrador. " +
          "Podemos reabrir o Otimiza com essa permissao?"
      );
      return;
    }

    // Um adaptador de cada vez seria pior: a maquina usa o DNS do adaptador
    // ativo, e trocar so um deixaria o resultado dependendo de qual conexao
    // esta em uso na hora.
    const adaptadores = lastNetwork?.adapters ?? [];
    if (adaptadores.length === 0) {
      setStatus("net-status", "Nenhum adaptador ativo para configurar.", "error");
      return;
    }

    button.disabled = true;

    try {
      for (const adaptador of adaptadores) {
        await invoke<unknown>("set_dns", {
          guid: adaptador.guid,
          servers: button.dataset.dns,
        });
      }
      setStatus(
        "net-status",
        "DNS trocado. A troca fica no historico e o botao Desfazer tudo devolve o anterior.",
        "ok"
      );
    } catch (error) {
      setStatus("net-status", String(error), "error");
    } finally {
      await analyzeNetwork();
    }
  });
  element("prioritize-fivem").addEventListener("click", prioritizeFiveM);

  element("fivem-result").addEventListener("click", async (event) => {
    const button = (event.target as HTMLElement).closest(
      "button[data-fivem]"
    ) as HTMLButtonElement | null;
    if (!button) return;

    // Não tem volta, e a contrapartida vai antes da confirmação: em servidor de
    // RP grande, rebaixar tudo leva vários minutos.
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

    // Apagar cache não tem volta. O aviso vem antes, com a contrapartida
    // escrita: limpar deixa o primeiro carregamento mais lento.
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

  element("pref-gamemode").addEventListener("change", (event) =>
    savePreferences({ auto_game_mode: (event.target as HTMLInputElement).checked })
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

  // Reconsentimento do modo jogo. Sem botão "fechar" nem clique fora: as
  // únicas duas saídas são "Manter" e "Desligar" — ambas gravam que a
  // pessoa já viu, então não existe um terceiro caminho que deixaria a tela
  // reaparecendo sem registrar nada.
  element("reconsent-manter").addEventListener("click", reconsentirMantendo);
  element("reconsent-desligar").addEventListener("click", reconsentirDesligando);

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
