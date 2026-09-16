import { invoke } from "@tauri-apps/api/core";

/*
 * O LABORATÓRIO DE GERAÇÃO DE QUADROS.
 *
 * O motor mora em `src-tauri/src/modules/windows/framegen.rs`: ele decide, e
 * manda ESTADOS (enums). Esta tela escolhe a frase e a cor para cada estado —
 * nunca o contrário. Nenhuma comparação aqui olha texto vindo do Rust.
 *
 * O que a tela nunca faz: ligar geração de quadros. A pessoa liga no jogo, no
 * driver ou no Lossless Scaling; o Otimiza mede antes e depois e diz o que os
 * números mostram. O único ajuste que o laboratório aplica é o limite de FPS
 * no driver NVIDIA — pelo mesmo comando da aba Jogos, que entra no histórico
 * e é desfeito sozinho se a medição seguinte sair pior.
 */

// ------------------------------------------------------------------ os tipos

type Fabricante = "Nvidia" | "Amd" | "Intel" | "Desconhecido";
type Arquitetura =
  | "Rtx50" | "Rtx40" | "Rtx30" | "Rtx20" | "GtxOuAnterior"
  | "Rx9000" | "Rx7000" | "Rx6000" | "RxAnterior"
  | "IntelArc" | "IntelIntegrada" | "Desconhecida";
type Tecnologia = "DlssFg" | "FsrFg" | "SmoothMotion" | "Afmf" | "LosslessScaling";
type Tipo = "Nativa" | "Driver" | "Externa";
type Disponibilidade =
  | "DependeDoJogo" | "DependeDoDriver" | "Instalado" | "NaoEncontrado" | "NaoSuportada" | "SemConfirmacao";
type JogoConhecido = "FiveM" | "GtaVEnhanced" | "GtaVLegacy";
type Base =
  | "MedidoVezesMultiplicador" | "MedidoDivididoPeloMultiplicador" | "IntervaloDoQuadro" | "UmQuadroRetido" | "SemGeracao";
type MotivoDesconhecido = "ExigeSensor" | "GeradorSemQuadros";
type Valor =
  | { origem: "Medido"; valor: number }
  | { origem: "Estimado"; valor: number; base: Base }
  | { origem: "Desconhecido"; motivo: MotivoDesconhecido };
type Prontidao = "NaoRecomendada" | "Marginal" | "Boa" | "Excelente" | "Desnecessaria";
type MotivoProntidao =
  | "BaseMuitoBaixa" | "BaseBaixa" | "BaseAceitavel" | "BaseIdeal"
  | "RitmoIrregular" | "JaNoLimiteDoMonitor" | "LimitadoPeloProcessador" | "AmostraCurta";
type Limite =
  | "CpuUmNucleo" | "CpuTodos" | "Gpu" | "MemoriaVideo" | "MemoriaRam" | "Disco" | "NaoIdentificado" | "SemCarga";
type Perfil = "Competitivo" | "Equilibrado" | "MaximaFluidez";
type Decisao = "Manter" | "Desligar" | "Inconclusivo";
type MotivoDecisao =
  | "AmostraCurta" | "RenderizadoCaiu" | "RitmoPiorou" | "AtrasoAcrescentadoAlto" | "ArtefatosIncomodos"
  | "PontuacaoSubiu" | "PontuacaoCaiu" | "DiferencaPequena" | "ExibidoDesconhecido" | "PassouDoMonitor";
type Casamento = "AcimaDoMonitor" | "NoLimite" | "AbaixoDoMonitor" | "MonitorDesconhecido";
type Artefato = "Fantasmas" | "InterfaceTremida" | "BordasQuebradas" | "RespostaPesada" | "Borrado";
type Intensidade = "Nenhuma" | "Leve" | "Forte";
type NivelDeArtefato = "Limpo" | "Aceitavel" | "Incomodo";

type Opcao = {
  tecnologia: Tecnologia;
  tipo: Tipo;
  disponibilidade: Disponibilidade;
  multiplicadores: number[];
  o_jogo_oferece: boolean | null;
};

type Deteccao = {
  placa: { nome: string; fabricante: Fabricante; arquitetura: Arquitetura; driver: string | null };
  tela: { largura: number; altura: number; hz_atual: number; hz_maximo: number } | null;
  jogo: JogoConhecido | null;
  processo: string | null;
  lossless_scaling: boolean | null;
  opcoes: Opcao[];
};

type Ritmo = {
  amostras: number;
  media_ms: number;
  p95_ms: number;
  p99_ms: number;
  low_1pct_fps: number;
  oscilacao_ms: number;
  picos_por_minuto: number;
  consistencia: number;
  confiavel: boolean;
};

type Rodada = {
  tecnologia: Tecnologia | null;
  multiplicador: number;
  fps_renderizado: Valor;
  fps_exibido: Valor;
  ritmo_renderizado: Ritmo;
  ritmo_exibido: Ritmo | null;
  latencia: { renderizacao_ms: Valor; acrescimo_da_geracao_ms: Valor; jogo_ms: Valor; tela_ms: Valor };
  amostra_ms: number[];
  segundos: number;
};

type ResultadoDaRodada = {
  rodada: Rodada;
  prontidao: { nivel: Prontidao; motivos: MotivoProntidao[] } | null;
  gargalo: {
    limite: Limite;
    cpu_total: number;
    cpu_max_core: number;
    gpu_percent: number;
    vram_used_mb: number;
    vram_total_mb: number | null;
    cpu_limitada: boolean;
  } | null;
  tela: { casamento: Casamento; limite_sugerido: number | null };
};

type Pontuacao = { total: number; fluidez: number; resposta: number; estabilidade: number };

type ComparacaoDaTela = {
  comparacao: {
    perfil: Perfil;
    pontos_desligado: Pontuacao;
    pontos_ligado: Pontuacao;
    exibido_pct: number | null;
    renderizado_pct: number | null;
    atraso_acrescentado_ms: number | null;
    decisao: Decisao;
    motivos: MotivoDecisao[];
  };
  artefatos: { nota: number; nivel: NivelDeArtefato } | null;
};

type OptimizationOutcome = { id: string; success: boolean; applied: boolean; message: string };

type Tom = "ok" | "aviso" | "erro" | "neutro";

// ------------------------------------------------------- as frases da tela

const NOME: Record<Tecnologia, string> = {
  DlssFg: "NVIDIA DLSS Frame Generation",
  FsrFg: "AMD FSR 3 Frame Generation",
  SmoothMotion: "NVIDIA Smooth Motion",
  Afmf: "AMD Fluid Motion Frames",
  LosslessScaling: "Lossless Scaling",
};

const CURTO: Record<Tecnologia, string> = {
  DlssFg: "DLSS FG",
  FsrFg: "FSR 3 FG",
  SmoothMotion: "Smooth Motion",
  Afmf: "AFMF",
  LosslessScaling: "Lossless Scaling",
};

const TIPO: Record<Tipo, { rotulo: string; explica: string }> = {
  Nativa: { rotulo: "NATIVA", explica: "o jogo gera, dentro do próprio motor" },
  Driver: { rotulo: "DRIVER", explica: "o driver gera, depois que o jogo entregou o quadro" },
  Externa: { rotulo: "EXTERNA", explica: "outro programa gera, na janela dele" },
};

const ONDE_LIGAR: Record<Tecnologia, string> = {
  DlssFg: "No menu gráfico do jogo, em DLSS → Geração de quadros. Só existe em jogos que trazem a opção.",
  FsrFg: "No menu gráfico do jogo, em FSR → Geração de quadros. Só existe em jogos que trazem a opção.",
  SmoothMotion:
    "No app NVIDIA ou no Painel de Controle NVIDIA, nas configurações do programa do jogo → Smooth Motion. Exige driver recente.",
  Afmf: "No AMD Software: Adrenalin Edition → Jogos → Gráficos → AMD Fluid Motion Frames.",
  LosslessScaling:
    "Abra o Lossless Scaling, escolha LSFG e o multiplicador, deixe o jogo em janela sem bordas e clique em Scale.",
};

const DISPONIBILIDADE: Record<Disponibilidade, { texto: string; tom: Tom }> = {
  DependeDoJogo: { texto: "a placa suporta — depende do jogo trazer", tom: "ok" },
  DependeDoDriver: { texto: "a placa suporta — depende da versão do driver", tom: "ok" },
  Instalado: { texto: "instalado nesta máquina", tom: "ok" },
  NaoEncontrado: { texto: "não achei na pasta padrão da Steam", tom: "neutro" },
  NaoSuportada: { texto: "sem suporte nesta placa", tom: "erro" },
  SemConfirmacao: { texto: "pode funcionar — o fabricante não garante nesta placa", tom: "aviso" },
};

const ARQUITETURA: Record<Arquitetura, string> = {
  Rtx50: "GeForce RTX 50",
  Rtx40: "GeForce RTX 40",
  Rtx30: "GeForce RTX 30",
  Rtx20: "GeForce RTX 20",
  GtxOuAnterior: "GeForce GTX ou anterior",
  Rx9000: "Radeon RX 9000",
  Rx7000: "Radeon RX 7000",
  Rx6000: "Radeon RX 6000",
  RxAnterior: "Radeon anterior à RX 6000",
  IntelArc: "Intel Arc",
  IntelIntegrada: "Intel integrada",
  Desconhecida: "não identificada",
};

const JOGO: Record<JogoConhecido, string> = {
  FiveM: "FiveM — sem geração de quadros própria",
  GtaVEnhanced: "GTA V Enhanced — traz DLSS e FSR com geração",
  GtaVLegacy: "GTA V Legacy — sem geração de quadros própria",
};

const PRONTIDAO: Record<Prontidao, { rotulo: string; tom: Tom }> = {
  Excelente: { rotulo: "EXCELENTE", tom: "ok" },
  Boa: { rotulo: "BOA", tom: "ok" },
  Marginal: { rotulo: "MARGINAL", tom: "aviso" },
  NaoRecomendada: { rotulo: "NÃO RECOMENDADA", tom: "erro" },
  Desnecessaria: { rotulo: "DESNECESSÁRIA", tom: "neutro" },
};

const MOTIVO_PRONTIDAO: Record<MotivoProntidao, string> = {
  BaseMuitoBaixa:
    "Menos de 30 quadros reais. O quadro gerado nasce de imagens muito distantes entre si: vira borrão, e o atraso dobra.",
  BaseBaixa: "Entre 30 e 45 quadros reais. Funciona, mas o artefato aparece e o atraso é sentido.",
  BaseAceitavel: "Entre 45 e 60 quadros reais: a faixa mínima que os fabricantes consideram aceitável.",
  BaseIdeal: "60 ou mais quadros reais: a faixa para a qual a geração de quadros foi pensada.",
  RitmoIrregular: "O ritmo dos quadros já é irregular. A geração copia a irregularidade — por isso o nível caiu um degrau.",
  JaNoLimiteDoMonitor: "O jogo já entrega o que o monitor consegue mostrar. Quadro gerado a mais não aparece na tela.",
  LimitadoPeloProcessador:
    "O processador é o teto. A geração não depende dele, então pode subir a fluidez — mas não muda a resposta.",
  AmostraCurta: "Poucos quadros medidos para julgar o ritmo. Meça por mais tempo.",
};

const LIMITE: Record<Limite, { texto: string; cpu: boolean }> = {
  CpuUmNucleo: { texto: "Um núcleo do processador no limite", cpu: true },
  CpuTodos: { texto: "Todos os núcleos do processador no limite", cpu: true },
  Gpu: { texto: "A placa de vídeo é o teto", cpu: false },
  MemoriaVideo: { texto: "A memória de vídeo acabou", cpu: false },
  MemoriaRam: { texto: "A memória do sistema acabou", cpu: false },
  Disco: { texto: "O disco não deu conta", cpu: false },
  NaoIdentificado: { texto: "Nada perto do limite", cpu: false },
  SemCarga: { texto: "Máquina parada durante a medição", cpu: false },
};

const BASE: Record<Base, string> = {
  MedidoVezesMultiplicador:
    "Quadros do jogo medidos × multiplicador. A geração no driver acontece depois da entrega que o Windows conta.",
  MedidoDivididoPeloMultiplicador:
    "Quadros medidos ÷ multiplicador. Na geração nativa os gerados passam pela mesma entrega dos reais.",
  IntervaloDoQuadro: "1000 ÷ quadros renderizados por segundo.",
  UmQuadroRetido: "Para interpolar, o quadro real mais novo espera cerca de um intervalo de quadro.",
  SemGeracao: "Sem geração ligada, não há espera de interpolação.",
};

const DESCONHECIDO: Record<MotivoDesconhecido, string> = {
  ExigeSensor: "Medir isso exige sensor na tela ou NVIDIA Reflex Analyzer. Software sozinho não vê.",
  GeradorSemQuadros: "O Lossless Scaling não entregou quadros durante a medição. Ele estava escalando o jogo?",
};

const PERFIL: Record<Perfil, { rotulo: string; explica: string }> = {
  Competitivo: {
    rotulo: "Competitivo",
    explica: "Resposta acima de tudo. Quadro gerado não vale ponto; qualquer atraso a mais pesa contra.",
  },
  Equilibrado: { rotulo: "Equilibrado", explica: "Fluidez e resposta com o mesmo peso." },
  MaximaFluidez: {
    rotulo: "Máxima fluidez",
    explica: "A imagem mais lisa possível. Atraso pesa pouco — ainda assim, nunca é escondido.",
  },
};

const DECISAO: Record<Decisao, { rotulo: string; tom: Tom }> = {
  Manter: { rotulo: "VALE MANTER LIGADO", tom: "ok" },
  Desligar: { rotulo: "MELHOR DESLIGAR", tom: "erro" },
  Inconclusivo: { rotulo: "INCONCLUSIVO", tom: "aviso" },
};

const MOTIVO_DECISAO: Record<MotivoDecisao, string> = {
  AmostraCurta: "Pelo menos uma das medições teve poucos quadros. Meça de novo por mais tempo.",
  RenderizadoCaiu: "O custo de gerar tirou quadros reais demais — o jogo passou a desenhar menos.",
  RitmoPiorou: "O ritmo dos quadros ficou mais irregular com a geração ligada.",
  AtrasoAcrescentadoAlto: "No perfil competitivo, o atraso estimado acrescentado passou de 10 ms.",
  ArtefatosIncomodos: "No teste visual você marcou artefato forte.",
  PontuacaoSubiu: "A pontuação subiu com a geração ligada, sem nenhuma regra contra.",
  PontuacaoCaiu: "A pontuação caiu com a geração ligada.",
  DiferencaPequena: "A diferença ficou dentro da margem de 3 pontos: não dá para afirmar melhora nem piora.",
  ExibidoDesconhecido: "Os quadros exibidos não puderam ser medidos nesta rodada.",
  PassouDoMonitor: "A tela recebeu mais quadros do que o monitor mostra. Limite o FPS em vez de desligar.",
};

const CASAMENTO: Record<Casamento, { texto: string; tom: Tom }> = {
  AcimaDoMonitor: {
    texto: "Mais quadros exibidos do que o monitor mostra: o excedente vira rasgo na imagem ou fila de atraso.",
    tom: "aviso",
  },
  NoLimite: { texto: "Quadros exibidos casando com a taxa do monitor.", tom: "ok" },
  AbaixoDoMonitor: { texto: "Quadros exibidos abaixo da taxa do monitor: há espaço na tela.", tom: "neutro" },
  MonitorDesconhecido: { texto: "Não consegui ler a taxa do monitor principal.", tom: "neutro" },
};

const ARTEFATOS: { id: Artefato; rotulo: string; como: string }[] = [
  { id: "InterfaceTremida", rotulo: "Interface tremendo", como: "Mira, minimapa, velocímetro e textos duplicando ao mover a câmera." },
  { id: "Fantasmas", rotulo: "Fantasmas", como: "Rastro atrás de carros, pessoas ou postes passando rápido." },
  { id: "BordasQuebradas", rotulo: "Bordas rasgando", como: "Contorno de personagem ou carro deformando ao girar a câmera." },
  { id: "Borrado", rotulo: "Borrado", como: "Imagem menos nítida em movimento do que parada." },
  { id: "RespostaPesada", rotulo: "Resposta pesada", como: "A tela parece lisa, mas o mouse parece atrasado." },
];

const INTENSIDADES: { id: Intensidade; rotulo: string }[] = [
  { id: "Nenhuma", rotulo: "Não vi" },
  { id: "Leve", rotulo: "Leve" },
  { id: "Forte", rotulo: "Forte" },
];

const NIVEL_ARTEFATO: Record<NivelDeArtefato, { rotulo: string; tom: Tom }> = {
  Limpo: { rotulo: "limpo", tom: "ok" },
  Aceitavel: { rotulo: "aceitável", tom: "aviso" },
  Incomodo: { rotulo: "incômodo", tom: "erro" },
};

// ------------------------------------------------------------ utilidades

const esc = (s: string) =>
  s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);

const num = (n: number, casas = 0) =>
  n.toLocaleString("pt-BR", { minimumFractionDigits: casas, maximumFractionDigits: casas });

const pct = (n: number | null) => (n === null ? "—" : `${n > 0 ? "+" : ""}${num(n, 1)}%`);

const numero = (v: Valor): number | null => (v.origem === "Desconhecido" ? null : v.valor);

function selo(v: Valor): string {
  const dica =
    v.origem === "Medido"
      ? "Contado pelo Windows durante a medição."
      : v.origem === "Estimado"
        ? BASE[v.base]
        : DESCONHECIDO[v.motivo];
  return `<span class="fg-selo" data-origem="${v.origem}" title="${esc(dica)}">${
    v.origem === "Medido" ? "MEDIDO" : v.origem === "Estimado" ? "ESTIMADO" : "DESCONHECIDO"
  }</span>`;
}

function valor(v: Valor, unidade: string, casas = 0): string {
  const n = numero(v);
  return `<span class="fg-par"><span class="fg-valor">${n === null ? "—" : `${num(n, casas)}<small>${unidade}</small>`}</span>${selo(v)}</span>`;
}

const $ = <T extends HTMLElement>(raiz: HTMLElement, seletor: string) => raiz.querySelector<T>(seletor);

// ----------------------------------------------------------------- o estado

type Ligada = { resultado: ResultadoDaRodada; artefatos: Partial<Record<Artefato, Intensidade>> };

const estado = {
  deteccao: null as Deteccao | null,
  processo: "fivem",
  segundos: 20,
  perfil: "Equilibrado" as Perfil,
  desligado: null as ResultadoDaRodada | null,
  ligadas: [] as Ligada[],
  selecionada: -1,
  escolha: { tecnologia: null as Tecnologia | null, multiplicador: 2 },
  medindo: false,
  erro: "",
  elevado: false,
  melhor: null as number | null,
  comparacao: null as ComparacaoDaTela | null,
  limite: null as { id: string; fps: number; antes: Rodada } | null,
  aviso_limite: "",
};

let raiz: HTMLElement;
let pedirAdmin: (motivo: string) => void = () => {};

const hz = () => estado.deteccao?.tela?.hz_atual ?? 0;

// ------------------------------------------------------------- as ações

async function detectar() {
  estado.erro = "";
  try {
    estado.deteccao = await invoke<Deteccao>("framegen_detectar", { processo: estado.processo });
    const primeira = estado.deteccao.opcoes.find((o) => o.disponibilidade !== "NaoSuportada");
    if (!estado.escolha.tecnologia && primeira) {
      estado.escolha = { tecnologia: primeira.tecnologia, multiplicador: primeira.multiplicadores[0] ?? 2 };
    }
  } catch (e) {
    estado.erro = String(e);
  }
  desenhar();
}

async function medir(tecnologia: Tecnologia | null, multiplicador: number): Promise<ResultadoDaRodada | null> {
  if (!estado.elevado) {
    pedirAdmin(
      "Medir quadros exige permissão de administrador: o canal de eventos do Windows que conta cada quadro só abre assim. Podemos reabrir o Otimiza com essa permissão?",
    );
    return null;
  }
  estado.medindo = true;
  estado.erro = "";
  desenhar();
  iniciarContagem(estado.segundos);
  try {
    return await invoke<ResultadoDaRodada>("framegen_medir", {
      processo: estado.processo,
      segundos: estado.segundos,
      tecnologia,
      multiplicador,
      hz: hz(),
    });
  } catch (e) {
    estado.erro = String(e);
    return null;
  } finally {
    estado.medindo = false;
  }
}

async function medirDesligado() {
  const r = await medir(null, 1);
  if (r) {
    estado.desligado = r;
    estado.comparacao = null;
    estado.melhor = null;
  }
  desenhar();
}

async function medirLigado() {
  const { tecnologia, multiplicador } = estado.escolha;
  if (!tecnologia) return;
  const r = await medir(tecnologia, multiplicador);
  if (r) {
    estado.ligadas.push({ resultado: r, artefatos: {} });
    estado.selecionada = estado.ligadas.length - 1;
    await recalcular();
  }
  desenhar();
}

async function recalcular() {
  const ligada = estado.ligadas[estado.selecionada];
  if (!estado.desligado || !ligada) {
    estado.comparacao = null;
    return;
  }
  const artefatos = Object.entries(ligada.artefatos) as [Artefato, Intensidade][];
  estado.comparacao = await invoke<ComparacaoDaTela>("framegen_comparar", {
    desligado: estado.desligado.rodada,
    ligado: ligada.resultado.rodada,
    perfil: estado.perfil,
    hz: hz(),
    artefatos,
  });
  estado.melhor = await invoke<number | null>("framegen_melhor", {
    desligado: estado.desligado.rodada,
    testadas: estado.ligadas.map((l) => l.resultado.rodada),
    perfil: estado.perfil,
    hz: hz(),
  });
}

/** O limite de FPS no driver NVIDIA, com desfazer automático se piorar. */
async function aplicarLimite(fps: number) {
  const executavel = estado.deteccao?.processo;
  const base = estado.ligadas[estado.selecionada]?.resultado.rodada ?? estado.desligado?.rodada;
  if (!executavel || !base) return;
  if (!estado.elevado) {
    pedirAdmin("O driver da NVIDIA só salva ajustes com permissão de administrador. Podemos reabrir o Otimiza com essa permissão?");
    return;
  }
  try {
    const r = await invoke<OptimizationOutcome>("limitar_fps_nvidia", { executavel, fps });
    estado.limite = r.success ? { id: r.id, fps, antes: base } : null;
    estado.aviso_limite = r.message;
  } catch (e) {
    estado.aviso_limite = String(e);
  }
  desenhar();
}

async function desfazerLimite(motivo: string) {
  if (!estado.limite) return;
  try {
    await invoke<OptimizationOutcome>("revert_optimization", { id: estado.limite.id });
    estado.aviso_limite = motivo;
    estado.limite = null;
  } catch (e) {
    estado.aviso_limite = String(e);
  }
  desenhar();
}

/**
 * Mede com o limite aplicado e compara com a rodada de antes dele. Se a régua
 * disser "pior", o limite é desfeito sem a pessoa precisar lembrar.
 */
async function conferirLimite() {
  const limite = estado.limite;
  if (!limite) return;
  const r = await medir(limite.antes.tecnologia, limite.antes.multiplicador);
  if (!r) return desenhar();
  const c = await invoke<ComparacaoDaTela>("framegen_comparar", {
    desligado: limite.antes,
    ligado: r.rodada,
    perfil: estado.perfil,
    hz: hz(),
    artefatos: [],
  });
  if (c.comparacao.decisao === "Desligar") {
    await desfazerLimite(
      `Com o limite de ${limite.fps} FPS a pontuação foi de ${num(c.comparacao.pontos_desligado.total)} para ${num(
        c.comparacao.pontos_ligado.total,
      )}. O limite foi desfeito automaticamente.`,
    );
  } else {
    estado.aviso_limite =
      c.comparacao.decisao === "Manter"
        ? `Com o limite de ${limite.fps} FPS a pontuação subiu para ${num(c.comparacao.pontos_ligado.total)}. O limite ficou.`
        : `Com o limite de ${limite.fps} FPS a diferença ficou dentro da margem. O limite ficou; desfaça se preferir.`;
    desenhar();
  }
}

let relogio: number | undefined;
function iniciarContagem(segundos: number) {
  const inicio = Date.now();
  window.clearInterval(relogio);
  relogio = window.setInterval(() => {
    const barra = $(raiz, ".fg-progresso-barra");
    const texto = $(raiz, ".fg-progresso-texto");
    const passado = (Date.now() - inicio) / 1000;
    if (!estado.medindo || passado > segundos + 5) {
      window.clearInterval(relogio);
      return;
    }
    if (barra) barra.style.width = `${Math.min(100, (passado / segundos) * 100)}%`;
    if (texto) texto.textContent = `Medindo… ${Math.max(0, Math.ceil(segundos - passado))} s. Continue jogando na mesma cena.`;
  }, 250);
}

// ---------------------------------------------------------------- o desenho

function painel(titulo: string, tag: string, corpo: string, extra = "") {
  return `<section class="panel fg-painel" ${extra}>
    <div class="panel-head"><h2>${titulo}</h2>${tag ? `<span class="panel-tag">${tag}</span>` : ""}</div>
    ${corpo}
  </section>`;
}

const chip = (texto: string, tom: Tom) => `<span class="fg-chip" data-tom="${tom}">${esc(texto)}</span>`;

function desenharPrincipio() {
  return painel(
    "Laboratório de geração de quadros",
    "mede, não liga",
    `<p class="fg-lead">Geração de quadros não é FPS grátis. O quadro gerado é uma imagem <strong>interpolada</strong> entre dois quadros que o jogo desenhou de verdade: a tela fica mais lisa, mas o jogo não responde mais rápido — e, para interpolar, o quadro real mais novo precisa esperar.</p>
    <p class="fg-lead">Por isso o laboratório separa sempre <strong>quadros renderizados</strong> (o que o jogo desenhou) de <strong>quadros exibidos</strong> (o que chegou à tela), e marca cada número como <span class="fg-selo" data-origem="Medido">MEDIDO</span> <span class="fg-selo" data-origem="Estimado">ESTIMADO</span> ou <span class="fg-selo" data-origem="Desconhecido">DESCONHECIDO</span>.</p>

    <div class="fg-fluxo" aria-label="Como um quadro é gerado">
      <div class="fg-quadro" data-real="true"><span>QUADRO REAL</span><b>N</b></div>
      <i aria-hidden="true">→</i>
      <div class="fg-quadro fg-analise"><span>ANÁLISE DE MOVIMENTO</span><b>vetores</b></div>
      <i aria-hidden="true">→</i>
      <div class="fg-quadro" data-real="false"><span>QUADRO GERADO</span><b>N + ½</b></div>
      <i aria-hidden="true">→</i>
      <div class="fg-quadro" data-real="true"><span>QUADRO REAL</span><b>N + 1</b></div>
    </div>

    <div class="fg-pipeline">
      <div><b>RENDER</b><span>o jogo desenha</span><em>${TIPO.Nativa.rotulo} age aqui</em></div>
      <div><b>INTERPOLAÇÃO</b><span>quadro inventado entre dois reais</span><em>${TIPO.Driver.rotulo} age aqui</em></div>
      <div><b>DISPLAY</b><span>o que o monitor mostra</span><em>${TIPO.Externa.rotulo} desenha na própria janela</em></div>
    </div>`,
  );
}

function desenharDeteccao() {
  const d = estado.deteccao;
  const entrada = `<div class="fg-linha">
      <label class="fg-campo">Processo do jogo
        <input id="fg-processo" list="fg-jogos" value="${esc(estado.processo)}" spellcheck="false" />
        <datalist id="fg-jogos"><option value="fivem"></option><option value="gta5_enhanced"></option><option value="gta5"></option></datalist>
      </label>
      <button class="btn" data-acao="detectar">${d ? "Detectar de novo" : "Detectar"}</button>
    </div>`;

  if (!d) return painel("O que existe nesta máquina", "", `${entrada}<p class="fg-nota">Abra o jogo e clique em Detectar.</p>`);

  const t = d.tela;
  const itens: [string, string][] = [
    ["Placa de vídeo", esc(d.placa.nome || "não identificada")],
    ["Arquitetura", ARQUITETURA[d.placa.arquitetura]],
    ["Driver", esc(d.placa.driver ?? "não lido")],
    ["Resolução", t ? `${t.largura} × ${t.altura}` : "não lida"],
    ["Taxa do monitor", t ? `${t.hz_atual} Hz${t.hz_maximo > t.hz_atual ? ` <small>(aceita ${t.hz_maximo} Hz)</small>` : ""}` : "não lida"],
    ["Jogo", d.processo ? `${esc(d.processo)}${d.jogo ? `<small>${JOGO[d.jogo]}</small>` : ""}` : "não está aberto"],
    ["Tela cheia ou sem bordas", `<span class="fg-desconhecido">DESCONHECIDO</span><small>Confira no menu do jogo. Lossless Scaling exige janela sem bordas.</small>`],
    ["VRR (G-SYNC / FreeSync)", `<span class="fg-desconhecido">DESCONHECIDO</span><small>Confira no painel do fabricante. Com VRR, o limite sugerido abaixo mantém a imagem dentro da faixa.</small>`],
  ];

  const linhas = d.opcoes
    .map((o) => {
      const disp = DISPONIBILIDADE[o.disponibilidade];
      const jogo =
        o.o_jogo_oferece === null ? "" : o.o_jogo_oferece ? chip("este jogo traz", "ok") : chip("este jogo não traz", "erro");
      return `<tr>
        <td><b>${NOME[o.tecnologia]}</b><small>${ONDE_LIGAR[o.tecnologia]}</small></td>
        <td><span class="fg-tipo" title="${TIPO[o.tipo].explica}">${TIPO[o.tipo].rotulo}</span></td>
        <td>${chip(disp.texto, disp.tom)} ${jogo}</td>
        <td class="fg-mono">${o.multiplicadores.length ? o.multiplicadores.map((m) => `${m}×`).join(" ") : "—"}</td>
      </tr>`;
    })
    .join("");

  const fivem =
    d.jogo === "FiveM"
      ? `<div class="fg-aviso"><b>Perfil FiveM.</b> O FiveM não tem geração de quadros própria e o Otimiza não mexe dentro dele — o anticheat procura exatamente isso. As opções possíveis ficam no driver (Smooth Motion, AFMF — conforme a placa) ou fora do jogo (Lossless Scaling), e a tabela abaixo diz quais existem aqui. Em servidor cheio o teto costuma ser o processador: meça sempre no mesmo lugar do mapa.</div>`
      : "";

  return painel(
    "O que existe nesta máquina",
    PLACA_TAG(d),
    `${entrada}
    <dl class="fg-grade">${itens.map(([k, v]) => `<div><dt>${k}</dt><dd>${v}</dd></div>`).join("")}</dl>
    ${fivem}
    <table class="fg-tabela"><thead><tr><th>Tecnologia e onde ligar</th><th>Tipo</th><th>Nesta máquina</th><th>Multiplicador</th></tr></thead><tbody>${linhas}</tbody></table>`,
  );
}

const PLACA_TAG = (d: Deteccao) =>
  d.placa.fabricante === "Nvidia" ? "NVIDIA" : d.placa.fabricante === "Amd" ? "AMD" : d.placa.fabricante === "Intel" ? "Intel" : "—";

function desenharPerfil() {
  const botoes = (Object.keys(PERFIL) as Perfil[])
    .map(
      (p) =>
        `<button class="fg-segmento" data-perfil="${p}" aria-pressed="${estado.perfil === p}">${PERFIL[p].rotulo}</button>`,
    )
    .join("");
  return painel(
    "O que você quer",
    "perfil",
    `<div class="fg-segmentos">${botoes}</div><p class="fg-nota">${PERFIL[estado.perfil].explica}</p>`,
  );
}

function grafico(amostra: number[], ritmo: Ritmo) {
  if (amostra.length < 2) return "";
  const topo = Math.max(ritmo.p99_ms * 1.4, ritmo.media_ms * 2, 1);
  const w = 600;
  const h = 90;
  const pontos = amostra
    .map((ms, i) => `${((i / (amostra.length - 1)) * w).toFixed(1)},${(h - (Math.min(ms, topo) / topo) * h).toFixed(1)}`)
    .join(" ");
  const media = h - (ritmo.media_ms / topo) * h;
  return `<figure class="fg-grafico">
    <svg viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" aria-label="Tempo de cada quadro">
      <line x1="0" x2="${w}" y1="${media.toFixed(1)}" y2="${media.toFixed(1)}" class="fg-grafico-media" />
      <polyline points="${pontos}" class="fg-grafico-linha" />
    </svg>
    <figcaption>Tempo de quadro dos primeiros ${amostra.length} quadros · linha fina = média ${num(ritmo.media_ms, 1)} ms</figcaption>
  </figure>`;
}

function blocoRitmo(r: Ritmo, titulo: string) {
  const itens: [string, string][] = [
    ["Tempo médio", `${num(r.media_ms, 1)} ms`],
    ["P95", `${num(r.p95_ms, 1)} ms`],
    ["P99", `${num(r.p99_ms, 1)} ms`],
    ["1% piores", `${num(r.low_1pct_fps, 0)} FPS`],
    ["Oscilação", `${num(r.oscilacao_ms, 2)} ms`],
    ["Picos", `${num(r.picos_por_minuto, 1)}/min`],
    ["Consistência", `${num(r.consistencia)}/100`],
  ];
  return `<div class="fg-ritmo"><h3>${titulo}${r.confiavel ? "" : ` ${chip("amostra curta", "aviso")}`}</h3>
    <dl>${itens.map(([k, v]) => `<div><dt>${k}</dt><dd>${v}</dd></div>`).join("")}</dl></div>`;
}

function blocoLatencia(l: Rodada["latencia"]) {
  const linhas: [string, Valor][] = [
    ["Renderização (um quadro real)", l.renderizacao_ms],
    ["Acrescentado pela geração", l.acrescimo_da_geracao_ms],
    ["Jogo (entrada até simulação)", l.jogo_ms],
    ["Monitor", l.tela_ms],
  ];
  return `<div class="fg-latencia"><h3>Atraso, por parte</h3>
    ${linhas.map(([k, v]) => `<div><span>${k}</span>${valor(v, " ms", 1)}</div>`).join("")}
    <p class="fg-nota">Geração de quadros não reduz atraso. Quem reduz é NVIDIA Reflex ou AMD Radeon Chill/Boost, que são outra coisa — e este número nunca vai dizer o contrário.</p></div>`;
}

function blocoGargalo(g: ResultadoDaRodada["gargalo"]) {
  if (!g) return "";
  const cpu = LIMITE[g.limite].cpu;
  return `<div class="fg-gargalo" data-cpu="${cpu}">
    ${cpu ? `<b class="fg-cpu-limit">CPU LIMIT DETECTED</b>` : ""}
    <span>${LIMITE[g.limite].texto}</span>
    <span class="fg-mono">CPU ${num(g.cpu_total)}% · núcleo mais carregado ${num(g.cpu_max_core)}% · GPU ${num(g.gpu_percent)}%${
      g.vram_total_mb ? ` · VRAM ${num(g.vram_used_mb / 1024, 1)}/${num(g.vram_total_mb / 1024, 1)} GB` : ""
    }</span>
    ${cpu ? `<small>Com o processador no teto, a placa espera por ele. A geração de quadros é um dos poucos recursos que sobem a fluidez nesse caso — sem mudar a resposta, que continua presa ao processador.</small>` : ""}
  </div>`;
}

function progresso() {
  return estado.medindo
    ? `<div class="fg-progresso"><div class="fg-progresso-trilho"><div class="fg-progresso-barra"></div></div><span class="fg-progresso-texto">Medindo… mantenha o jogo em primeiro plano.</span></div>`
    : "";
}

function desenharPassoDesligado() {
  const d = estado.desligado;
  const duracoes = [10, 20, 30, 60]
    .map((s) => `<option value="${s}" ${estado.segundos === s ? "selected" : ""}>${s} s</option>`)
    .join("");

  const cabecalho = `<ol class="fg-passos"><li>Desligue toda geração de quadros — no jogo, no driver e no Lossless Scaling.</li><li>Vá para uma cena que você consegue repetir (o mesmo lugar do mapa, o mesmo trajeto).</li><li>Clique em medir e continue jogando até terminar.</li></ol>
    <div class="fg-linha">
      <label class="fg-campo">Duração <select id="fg-segundos">${duracoes}</select></label>
      <button class="btn btn-primary" data-acao="medir-desligado" ${estado.medindo ? "disabled" : ""}>${d ? "Medir desligado de novo" : "Medir com a geração desligada"}</button>
    </div>
    ${estado.elevado ? "" : `<p class="fg-nota" data-tom="aviso">Medir quadros exige abrir o Otimiza como administrador.</p>`}`;

  if (!d) return painel("1 · Base, com a geração desligada", "", cabecalho + progresso());

  const p = d.prontidao;
  const prontidao = p
    ? `<div class="fg-prontidao" data-tom="${PRONTIDAO[p.nivel].tom}">
        <span>FRAME GEN READINESS</span><b>${PRONTIDAO[p.nivel].rotulo}</b>
        <ul>${p.motivos.map((m) => `<li>${MOTIVO_PRONTIDAO[m]}</li>`).join("")}</ul>
      </div>`
    : "";

  return painel(
    "1 · Base, com a geração desligada",
    `${num(d.rodada.segundos)} s medidos`,
    `${cabecalho}${progresso()}
    <div class="fg-numeros">
      <div><span>Renderizados</span>${valor(d.rodada.fps_renderizado, " FPS")}</div>
      <div><span>Exibidos</span>${valor(d.rodada.fps_exibido, " FPS")}</div>
    </div>
    ${prontidao}
    ${blocoGargalo(d.gargalo)}
    ${blocoRitmo(d.rodada.ritmo_renderizado, "Ritmo dos quadros")}
    ${grafico(d.rodada.amostra_ms, d.rodada.ritmo_renderizado)}
    ${blocoLatencia(d.rodada.latencia)}`,
  );
}

function desenharPassoLigado() {
  const d = estado.deteccao;
  if (!estado.desligado) return "";
  const opcoes = (d?.opcoes ?? []).filter((o) => o.disponibilidade !== "NaoSuportada");
  if (!opcoes.length) {
    return painel("2 · Com a geração ligada", "", `<p class="fg-nota">Detecte a máquina primeiro para ver o que dá para testar aqui.</p>`);
  }
  const atual = opcoes.find((o) => o.tecnologia === estado.escolha.tecnologia) ?? opcoes[0];
  const tecnologias = opcoes
    .map((o) => `<option value="${o.tecnologia}" ${o === atual ? "selected" : ""}>${NOME[o.tecnologia]} · ${TIPO[o.tipo].rotulo}</option>`)
    .join("");
  const mults = atual.multiplicadores
    .map((m) => `<button class="fg-segmento" data-mult="${m}" aria-pressed="${estado.escolha.multiplicador === m}">${m}×</button>`)
    .join("");

  return painel(
    "2 · Com a geração ligada",
    `${estado.ligadas.length} rodada${estado.ligadas.length === 1 ? "" : "s"}`,
    `<div class="fg-linha">
      <label class="fg-campo">Tecnologia <select id="fg-tecnologia">${tecnologias}</select></label>
      <div class="fg-campo">Multiplicador <div class="fg-segmentos">${mults}</div></div>
    </div>
    <p class="fg-onde"><b>Ligue agora:</b> ${ONDE_LIGAR[atual.tecnologia]}</p>
    <p class="fg-nota">Volte para a mesma cena da base. Para comparar multiplicadores, meça 2×, 3× e 4× um de cada vez.</p>
    <button class="btn btn-primary" data-acao="medir-ligado" ${estado.medindo ? "disabled" : ""}>Medir com ${CURTO[atual.tecnologia]} ${estado.escolha.multiplicador}×</button>
    ${progresso()}`,
  );
}

function linhaDoTempo(multiplicador: number) {
  const blocos: string[] = [];
  for (let i = 0; i < 6; i++) {
    blocos.push(`<span data-real="true">REAL</span>`);
    for (let g = 1; g < multiplicador; g++) blocos.push(`<span data-real="false">GEN</span>`);
  }
  return `<div class="fg-timeline" aria-label="Sequência de quadros reais e gerados">${blocos.join("")}</div>`;
}

function desenharMultiplicadores() {
  const base = estado.desligado;
  if (!base || !estado.ligadas.length) return "";
  const linha = (rotulo: string, r: Rodada, i: number | null) => {
    const selecionada = i !== null && i === estado.selecionada;
    const melhor = i !== null && i === estado.melhor;
    return `<tr ${selecionada ? 'data-selecionada="true"' : ""}>
      <td>${rotulo}${melhor ? ` ${chip("SMART: melhor para este perfil", "ok")}` : ""}</td>
      <td>${valor(r.fps_renderizado, "")}</td>
      <td>${valor(r.fps_exibido, "")}</td>
      <td class="fg-mono">${num((r.ritmo_exibido ?? r.ritmo_renderizado).p99_ms, 1)} ms</td>
      <td>${valor(r.latencia.acrescimo_da_geracao_ms, " ms", 1)}</td>
      <td>${i === null ? "" : `<button class="btn btn-ghost" data-selecionar="${i}">${selecionada ? "em análise" : "analisar"}</button>`}</td>
    </tr>`;
  };
  const smart =
    estado.melhor === null
      ? `<p class="fg-nota"><b>SMART FRAME GEN:</b> nenhuma rodada ligada ganhou da base no perfil ${PERFIL[estado.perfil].rotulo}. A recomendação é deixar desligado.</p>`
      : `<p class="fg-nota"><b>SMART FRAME GEN:</b> no perfil ${PERFIL[estado.perfil].rotulo}, a melhor rodada foi ${CURTO[estado.ligadas[estado.melhor].resultado.rodada.tecnologia!]} ${estado.ligadas[estado.melhor].resultado.rodada.multiplicador}×.</p>`;
  return painel(
    "Comparação de rodadas",
    "OFF · 2× · 3× · 4×",
    `<table class="fg-tabela"><thead><tr><th>Rodada</th><th>Renderizados</th><th>Exibidos</th><th>P99</th><th>Atraso a mais</th><th></th></tr></thead>
    <tbody>${linha("Desligado", base.rodada, null)}${estado.ligadas
      .map((l, i) => linha(`${CURTO[l.resultado.rodada.tecnologia!]} ${l.resultado.rodada.multiplicador}×`, l.resultado.rodada, i))
      .join("")}</tbody></table>${smart}`,
  );
}

function desenharArtefatos() {
  const ligada = estado.ligadas[estado.selecionada];
  if (!ligada) return "";
  const linhas = ARTEFATOS.map((a) => {
    const atual = ligada.artefatos[a.id];
    const botoes = INTENSIDADES.map(
      (i) => `<button class="fg-segmento" data-artefato="${a.id}" data-intensidade="${i.id}" aria-pressed="${atual === i.id}">${i.rotulo}</button>`,
    ).join("");
    return `<div class="fg-artefato"><div><b>${a.rotulo}</b><small>${a.como}</small></div><div class="fg-segmentos">${botoes}</div></div>`;
  }).join("");
  const nota = estado.comparacao?.artefatos;
  return painel(
    "3 · Teste visual de artefatos",
    nota ? `${num(nota.nota)}/100 · ${NIVEL_ARTEFATO[nota.nivel].rotulo}` : "guiado",
    `<p class="fg-nota">Nenhuma leitura automática de imagem é confiável o bastante para julgar artefato de interpolação — então quem julga é você. Com a geração ligada, gire a câmera rápido, dirija perto de postes e olhe a interface por 30 segundos.</p>${linhas}`,
  );
}

function barraPontos(rotulo: string, antes: number, depois: number) {
  return `<div class="fg-barra"><span>${rotulo}</span>
    <div class="fg-barra-par"><i style="width:${antes}%"></i><i data-ligado="true" style="width:${depois}%"></i></div>
    <b class="fg-mono">${num(antes)} → ${num(depois)}</b></div>`;
}

function desenharResultado() {
  const c = estado.comparacao?.comparacao;
  const ligada = estado.ligadas[estado.selecionada];
  if (!c || !ligada || !estado.desligado) return "";
  const r = ligada.resultado.rodada;
  const b = estado.desligado.rodada;
  const dec = DECISAO[c.decisao];

  return painel(
    "Antes e depois",
    `${CURTO[r.tecnologia!]} ${r.multiplicador}× · ${PERFIL[c.perfil].rotulo}`,
    `<div class="fg-decisao" data-tom="${dec.tom}">
      <div><span>FRAME GEN SCORE</span><b>${num(c.pontos_desligado.total)} → ${num(c.pontos_ligado.total)}</b></div>
      <strong>${dec.rotulo}</strong>
    </div>
    <ul class="fg-motivos">${c.motivos.map((m) => `<li>${MOTIVO_DECISAO[m]}</li>`).join("")}</ul>

    <div class="fg-antes-depois">
      <div><span>Renderizados</span><b>${valor(b.fps_renderizado, "")} → ${valor(r.fps_renderizado, "")}</b><em>${pct(c.renderizado_pct)}</em></div>
      <div><span>Exibidos</span><b>${valor(b.fps_exibido, "")} → ${valor(r.fps_exibido, "")}</b><em>${pct(c.exibido_pct)}</em></div>
      <div><span>Atraso estimado a mais</span><b>${c.atraso_acrescentado_ms === null ? "—" : `${num(c.atraso_acrescentado_ms, 1)} ms`}</b><em>${selo({ origem: "Estimado", valor: 0, base: "UmQuadroRetido" })}</em></div>
      <div><span>Precisão de mira</span><b>Não medido</b><em>${chip("requer AimTracking Lab", "neutro")}</em></div>
    </div>

    ${barraPontos("Fluidez", c.pontos_desligado.fluidez, c.pontos_ligado.fluidez)}
    ${barraPontos("Resposta", c.pontos_desligado.resposta, c.pontos_ligado.resposta)}
    ${barraPontos("Estabilidade", c.pontos_desligado.estabilidade, c.pontos_ligado.estabilidade)}
    <p class="fg-nota">A pontuação não é FPS. É fluidez (quadros exibidos contra a taxa do monitor), resposta (atraso estimado) e estabilidade (ritmo), pesadas pelo perfil escolhido.</p>

    <h3 class="fg-sub">Na tela, em sequência</h3>
    ${linhaDoTempo(r.multiplicador)}
    ${r.ritmo_exibido ? blocoRitmo(r.ritmo_exibido, "Ritmo do que chegou à tela") : blocoRitmo(r.ritmo_renderizado, "Ritmo dos quadros do jogo")}
    ${grafico(r.amostra_ms, r.ritmo_renderizado)}
    ${blocoLatencia(r.latencia)}`,
  );
}

function desenharTela() {
  const alvo = estado.ligadas[estado.selecionada]?.resultado ?? estado.desligado;
  if (!alvo) return "";
  const cas = CASAMENTO[alvo.tela.casamento];
  const nvidia = estado.deteccao?.placa.fabricante === "Nvidia";
  const sugerido = alvo.tela.limite_sugerido;
  const mult = alvo.rodada.multiplicador;

  let acao = "";
  if (estado.limite) {
    acao = `<div class="fg-linha">
      <button class="btn btn-primary" data-acao="conferir-limite" ${estado.medindo ? "disabled" : ""}>Medir com o limite de ${estado.limite.fps} FPS</button>
      <button class="btn" data-acao="desfazer-limite">Desfazer o limite</button>
    </div>
    <p class="fg-nota">Se a medição com o limite sair pior pela régua do perfil, o Otimiza desfaz o limite sozinho.</p>`;
  } else if (sugerido && nvidia && estado.deteccao?.processo) {
    acao = `<button class="btn" data-acao="aplicar-limite" data-fps="${sugerido}">Limitar ${esc(estado.deteccao.processo)} a ${sugerido} FPS no driver NVIDIA</button>
      <p class="fg-nota">Vai no perfil do executável do jogo, nunca no global, e entra no histórico com desfazer.</p>`;
  } else if (sugerido) {
    acao = `<p class="fg-nota">Aplique ${sugerido} FPS no limitador do jogo, do AMD Software ou do RivaTuner.</p>`;
  }

  return painel(
    "Taxa do monitor e limite de FPS",
    hz() ? `${hz()} Hz` : "",
    `<p class="fg-nota" data-tom="${cas.tom}">${cas.texto}</p>
    ${
      sugerido
        ? `<div class="fg-numeros"><div><span>Limite sugerido (renderizados)</span><span class="fg-valor">${sugerido}<small> FPS</small></span></div><div><span>Exibidos com ${mult}×</span><span class="fg-valor">${sugerido * mult}<small> FPS</small></span></div></div>
           <p class="fg-nota">Conta: (${hz()} Hz − folga de 3%, no mínimo 3) ÷ ${mult}. A folga mantém a imagem abaixo do teto do monitor, onde G-SYNC e FreeSync continuam agindo.</p>`
        : `<p class="fg-nota">Sem limite sugerido para esta combinação: com este multiplicador o limite ficaria abaixo de 20 FPS reais.</p>`
    }
    ${acao}
    ${estado.aviso_limite ? `<p class="fg-nota" data-tom="aviso">${esc(estado.aviso_limite)}</p>` : ""}`,
  );
}

function desenhar() {
  if (!raiz) return;
  raiz.innerHTML = [
    estado.erro ? `<div class="fg-erro" role="alert">${esc(estado.erro)}</div>` : "",
    desenharPrincipio(),
    desenharDeteccao(),
    desenharPerfil(),
    desenharPassoDesligado(),
    desenharPassoLigado(),
    desenharMultiplicadores(),
    desenharResultado(),
    desenharArtefatos(),
    desenharTela(),
  ].join("");
}

// ------------------------------------------------------------ os eventos

function ligarEventos() {
  raiz.addEventListener("click", async (ev) => {
    const alvo = (ev.target as HTMLElement).closest<HTMLButtonElement>("button");
    if (!alvo) return;
    const acao = alvo.dataset.acao;

    if (acao === "detectar") {
      estado.processo = $<HTMLInputElement>(raiz, "#fg-processo")?.value.trim() || estado.processo;
      alvo.disabled = true;
      await detectar();
    } else if (acao === "medir-desligado") {
      await medirDesligado();
    } else if (acao === "medir-ligado") {
      await medirLigado();
    } else if (acao === "aplicar-limite") {
      await aplicarLimite(Number(alvo.dataset.fps));
    } else if (acao === "desfazer-limite") {
      await desfazerLimite("Limite desfeito. O driver voltou ao valor que tinha antes.");
    } else if (acao === "conferir-limite") {
      await conferirLimite();
    } else if (alvo.dataset.perfil) {
      estado.perfil = alvo.dataset.perfil as Perfil;
      await recalcular();
      desenhar();
    } else if (alvo.dataset.mult) {
      estado.escolha.multiplicador = Number(alvo.dataset.mult);
      desenhar();
    } else if (alvo.dataset.selecionar) {
      estado.selecionada = Number(alvo.dataset.selecionar);
      await recalcular();
      desenhar();
    } else if (alvo.dataset.artefato) {
      const ligada = estado.ligadas[estado.selecionada];
      if (!ligada) return;
      ligada.artefatos[alvo.dataset.artefato as Artefato] = alvo.dataset.intensidade as Intensidade;
      await recalcular();
      desenhar();
    }
  });

  raiz.addEventListener("change", (ev) => {
    const alvo = ev.target as HTMLElement;
    if (alvo.id === "fg-segundos") {
      estado.segundos = Number((alvo as HTMLSelectElement).value);
    } else if (alvo.id === "fg-tecnologia") {
      const tecnologia = (alvo as HTMLSelectElement).value as Tecnologia;
      const opcao = estado.deteccao?.opcoes.find((o) => o.tecnologia === tecnologia);
      estado.escolha = { tecnologia, multiplicador: opcao?.multiplicadores[0] ?? 2 };
      desenhar();
    } else if (alvo.id === "fg-processo") {
      estado.processo = (alvo as HTMLInputElement).value.trim() || estado.processo;
    }
  });
}

let carregado = false;

/** Chamada ao abrir a aba, uma vez só: a detecção passa pelo PowerShell. */
export async function carregarLaboratorioDeGeracao(opcoes: { pedirAdmin: (motivo: string) => void }) {
  if (carregado) return;
  carregado = true;
  pedirAdmin = opcoes.pedirAdmin;
  raiz = document.getElementById("framegen-lab")!;
  if (!raiz) return;
  ligarEventos();
  try {
    estado.elevado = await invoke<boolean>("is_elevated");
  } catch {
    estado.elevado = false;
  }
  desenhar();
  await detectar();
}
