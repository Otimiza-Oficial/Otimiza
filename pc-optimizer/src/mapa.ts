import { invoke } from "@tauri-apps/api/core";

/*
 * O MAPA DE DESEMPENHO (2.9).
 *
 * CPU → memória → disco → GPU → VRAM, lidos pelos contadores do Windows
 * durante uma janela, e — se houver jogo aberto — os quadros dele no mesmo
 * intervalo. O classificador (`core::gargalo`) aponta TODOS os gargalos que
 * se sustentaram, cada um com os números que o justificam.
 *
 * Esta tela só traduz estados em frases. Nenhuma decisão mora aqui e nenhum
 * número é inventado: campo que o Windows não entregou aparece como "não
 * medido".
 */

/** As classes do classificador ÚNICO do produto (`modules/gargalo.rs`, 2.8). */
type Classe =
  | "CpuTodosNucleos" | "CpuUmNucleo" | "Gpu" | "MemoriaRam" | "MemoriaVideo" | "Disco"
  | "LimiteTermico" | "LimiteEletrico" | "TetoDeQuadros" | "Engasgo" | "ForaDoHardware"
  | "StreamingDeAssets" | "Rede";

type AchadoDeGargalo = { classe: Classe; forca: "Causa" | "Hipotese"; evidencia: string; idade_ms: number | null };

type DiagnosticoDeGargalo = {
  conclusao: "SemEvidencia" | "SemCarga" | "NadaNoLimite" | "Encontrado";
  achados: AchadoDeGargalo[];
  nao_verificado: { classe: string; falta: string }[];
  classes_avaliadas: number;
  classes_totais: number;
};

type Amostra = {
  cpu_total_pct: number | null;
  cpu_nucleo_max_pct: number | null;
  clock_efetivo_mhz: number | null;
  clock_nominal_mhz: number | null;
  gpu_pct: number | null;
  vram_usada_mb: number | null;
  ram_disponivel_mb: number | null;
  commit_pct: number | null;
  paginas_lidas_s: number | null;
  disco_latencia_ms: number | null;
  disco_ocupado_pct: number | null;
};

type Saude = {
  quadros: number;
  duracao_s: number;
  fps_medio: number;
  frametime_mediano_ms: number;
  p95_ms: number | null;
  p99_ms: number | null;
  low_1pct_fps: number | null;
  low_01pct_fps: number | null;
  engasgos: { micro: number; perceptivel: number; severo: number; extremo: number };
  engasgos_graves_por_minuto: number;
  indice_de_fluidez: number | null;
};

type Suspeito =
  | { tipo: "Disco" }
  | { tipo: "Paginacao" }
  | { tipo: "Vram" }
  | { tipo: "ThreadPrincipal" }
  | { tipo: "SegundoPlano"; processo: string };

type Investigacao = {
  travadas: { instante_ms: number; duracao_ms: number; gravidade: string }[];
  pistas: { suspeito: Suspeito; forca: "Media" | "Alta"; travadas: number }[];
  sem_causa_visivel: number;
};

function fraseDoSuspeito(s: Suspeito): { titulo: string; faz: string } {
  switch (s.tipo) {
    case "Disco":
      return { titulo: "o disco demorou para responder", faz: "Veja se o jogo está num HD mecânico (aba Jogos) e se algo está baixando ou atualizando." };
    case "Paginacao":
      return { titulo: "o Windows foi buscar memória no disco (faltou RAM)", faz: "Feche o navegador e programas pesados antes de jogar. Se acontece sempre, o limite é a quantidade de RAM." };
    case "Vram":
      return { titulo: "a memória da placa de vídeo encheu", faz: "Baixe a qualidade de textura um degrau." };
    case "ThreadPrincipal":
      return { titulo: "o núcleo principal do processador bateu no teto", faz: "Baixe distância de visão e densidade de população/objetos no jogo." };
    case "SegundoPlano":
      return { titulo: `o programa ${s.processo} disparou o uso de processador`, faz: "Feche esse programa antes de jogar, ou ligue o Modo jogo (aba Jogos): ele passa programas assim para o modo econômico durante a partida." };
  }
}

type Diagnostico = {
  placa: { nome: string; vram_total_mb: number } | null;
  jogo: string | null;
  amostras: Amostra[];
  saude: Saude | null;
  quadros_erro: string | null;
  gargalo: DiagnosticoDeGargalo;
  travadas: Investigacao | null;
};

/** O que cada gargalo quer dizer, e o que fazer — escrito para quem joga. */
/** O que cada classe quer dizer, e o que fazer — escrito para quem joga. */
const NA_TELA: Record<Classe, { titulo: string; explica: string; faz: string; no: No }> = {
  CpuUmNucleo: {
    titulo: "O jogo está preso em um núcleo do processador",
    explica: "Um núcleo está no limite enquanto os outros sobram. O uso total da CPU parece baixo, mas o jogo não consegue dividir esse trabalho.",
    faz: "Placa de vídeo nova NÃO resolve isto. Ajuda: baixar distância de visão e densidade de população/objetos no jogo, fechar programas pesados, RAM com XMP ligado e o plano de energia medido para a máquina (aba Energia).",
    no: "cpu",
  },
  CpuTodosNucleos: {
    titulo: "O processador inteiro está no limite",
    explica: "Todos os núcleos ocupados: a placa de vídeo recebe quadros mais devagar do que consegue desenhar.",
    faz: "Feche o que roda em segundo plano (ou ligue o modo jogo) e reduza o que pesa na CPU: população, física, distância.",
    no: "cpu",
  },
  Gpu: {
    titulo: "A placa de vídeo é o limite",
    explica: "A placa está no máximo. É o caso em que ajuste gráfico rende mais FPS.",
    faz: "Baixe primeiro sombras, pós-processamento, anti-serrilhado pesado e resolução de renderização (ficha do jogo, na Biblioteca). Upscaling e o gerador de quadros também entram aqui.",
    no: "gpu",
  },
  MemoriaRam: {
    titulo: "A memória RAM está apertada",
    explica: "Com a RAM no limite o Windows passa a buscar no disco — e cada busca é uma travada em potencial.",
    faz: "Feche navegador e programas pesados antes de jogar. Se acontece sempre, o limite é a quantidade de RAM.",
    no: "ram",
  },
  MemoriaVideo: {
    titulo: "A memória da placa de vídeo está apertada",
    explica: "Quando a VRAM enche, textura vai para a memória comum, muito mais lenta — aparece como travada, não como FPS menor.",
    faz: "Baixe a qualidade de textura um degrau e feche navegador e outros programas que usam a placa.",
    no: "vram",
  },
  Disco: {
    titulo: "O disco está no limite",
    explica: "O disco está ocupado a maior parte do tempo; o jogo espera quando precisa ler algo.",
    faz: "Veja se o jogo está num HD mecânico (aba Jogos) e se algo está baixando ou atualizando.",
    no: "disco",
  },
  LimiteTermico: {
    titulo: "O processador está sendo segurado por temperatura",
    explica: "O próprio Windows reporta que o firmware reduziu a velocidade por calor.",
    faz: "Nenhum ajuste de software resolve: limpeza, pasta térmica e circulação de ar.",
    no: "cpu",
  },
  LimiteEletrico: {
    titulo: "O processador está sendo segurado por energia",
    explica: "O firmware limitou a potência (notebook na bateria, fonte ou limite da placa-mãe).",
    faz: "No notebook, jogue na tomada. Em desktop, confira os limites de potência na BIOS (aba Diagnóstico).",
    no: "cpu",
  },
  TetoDeQuadros: {
    titulo: "O FPS está preso na taxa do monitor",
    explica: "O jogo entrega exatamente o que o monitor mostra — V-Sync ou limitador.",
    faz: "Se é a taxa do monitor, é o esperado. Os limites escondidos abaixo dela aparecem logo abaixo.",
    no: "tela",
  },
  Engasgo: {
    titulo: "Travadas frequentes durante a partida",
    explica: "Muitos quadros bem acima do normal. A causa não está nesta linha — o detetive de travadas, logo abaixo, mostra o que coincidiu com elas.",
    faz: "Veja as pistas do detetive de travadas.",
    no: "tela",
  },
  ForaDoHardware: {
    titulo: "O limite não está no hardware",
    explica: "Processador e placa sobrando ao mesmo tempo, durante a partida. O limite é o motor do jogo, o servidor, um teto de quadros ou uma espera que os contadores não mostram.",
    faz: "Ajuste de Windows não muda isto. Confira os limites de FPS escondidos e, no FiveM, o servidor.",
    no: "tela",
  },
  StreamingDeAssets: {
    titulo: "O jogo está esperando o disco",
    explica: "As travadas caem justamente quando o disco está ocupado: o jogo carregando conteúdo.",
    faz: "Instale o jogo num SSD (aba Jogos mostra em qual disco ele está) e evite downloads durante a partida.",
    no: "disco",
  },
  Rede: {
    titulo: "A conexão está instável ou perdendo pacote",
    explica: "Rede que engasga parece FPS baixo: teleporte, tiro que não registra.",
    faz: "Meça a perda de pacote até o servidor (aba Diagnóstico). Cabo em vez de Wi-Fi resolve a maioria.",
    no: "tela",
  },
};

type No = "cpu" | "ram" | "disco" | "gpu" | "vram" | "tela";

type Teto =
  | { tipo: "LimiteGlobalNvidia"; fps: number }
  | { tipo: "VsyncForcadoNvidia" }
  | { tipo: "Rtss" }
  | { tipo: "LimiteNoJogo"; jogo: string; fps: number; arquivo: string }
  | { tipo: "VsyncNoJogo"; jogo: string; arquivo: string }
  | { tipo: "RobloxLimitado"; fps: number | null; arquivo: string };

type RelatorioDeTetos = { monitor_hz: number | null; tetos: Teto[]; lacunas: string[] };

function frasesDoTeto(t: Teto, hz: number | null): { titulo: string; onde: string } {
  const monitor = hz ? `o monitor mostra ${hz}` : "o monitor mostra mais";
  switch (t.tipo) {
    case "LimiteGlobalNvidia":
      return {
        titulo: `O driver da NVIDIA segura TODO jogo em ${t.fps} FPS, e ${monitor}.`,
        onde: "Painel de Controle da NVIDIA → Gerenciar configurações 3D → aba Configurações globais → Taxa de quadros máxima → Desligada.",
      };
    case "VsyncForcadoNvidia":
      return {
        titulo: "O driver da NVIDIA está forçando V-Sync em todo jogo.",
        onde: "Painel de Controle da NVIDIA → Gerenciar configurações 3D → Configurações globais → Sincronização vertical → Usar configuração do aplicativo 3D.",
      };
    case "Rtss":
      return {
        titulo: "O RivaTuner (RTSS) está aberto — ele costuma ter um limite de FPS ligado.",
        onde: "Abra o RTSS e confira \"Framerate limit\" do perfil Global e do jogo. 0 é sem limite.",
      };
    case "LimiteNoJogo":
      return {
        titulo: `${t.jogo} está com limite de ${t.fps} FPS no arquivo de configuração, e ${monitor}.`,
        onde: `No menu do jogo, opção de limite de FPS. Arquivo: ${t.arquivo}`,
      };
    case "RobloxLimitado":
      return {
        titulo: t.fps
          ? `O Roblox está limitado a ${t.fps} FPS, e ${monitor}.`
          : `O Roblox está com o limite de quadros no padrão de fábrica (60 FPS), e ${monitor}.`,
        onde: "Dentro do Roblox: Configurações → Taxa de quadros máxima → escolha a do seu monitor.",
      };
    case "VsyncNoJogo":
      return {
        titulo: `${t.jogo} está com V-Sync ligado.`,
        onde: `No menu de vídeo do jogo. Arquivo: ${t.arquivo}`,
      };
  }
}

async function desenharTetos() {
  const alvo = raiz?.querySelector<HTMLElement>("#mapa-tetos");
  if (!alvo) return;
  alvo.innerHTML = `<p class="fg-nota">Procurando limites de FPS escondidos…</p>`;
  try {
    const r = await invoke<RelatorioDeTetos>("tetos_escondidos");
    const lacunas = r.lacunas.length ? `<p class="fg-nota">Não deu para ler: ${r.lacunas.map(esc).join("; ")}</p>` : "";
    if (!r.tetos.length) {
      alvo.innerHTML = `<p class="fg-nota">Nenhum limite de FPS escondido encontrado no driver, no RTSS ou nos arquivos dos jogos que o Otimiza sabe ler.</p>${lacunas}`;
      return;
    }
    alvo.innerHTML = `
      <h3 class="fg-sub">Limites de FPS escondidos</h3>
      ${r.tetos
        .map((t) => {
          const f = frasesDoTeto(t, r.monitor_hz);
          return `<article class="mapa-gargalo"><b>${esc(f.titulo)}</b><p class="fg-nota"><strong>Onde tirar:</strong> ${esc(f.onde)}</p></article>`;
        })
        .join("")}
      <p class="fg-nota">O Otimiza nunca põe limite de FPS, e não tira sozinho: às vezes o limite foi escolhido de propósito (menos calor, menos ruído).</p>
      ${lacunas}`;
  } catch (e) {
    alvo.innerHTML = `<p class="fg-nota">Não deu para procurar limites: ${esc(String(e))}</p>`;
  }
}

const SEGUNDOS = 20;
const ESPERA_PARA_VOLTAR_AO_JOGO = 8;

let raiz: HTMLElement | null = null;
let rodando = false;

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

function num(v: number | null | undefined, casas = 0, sufixo = ""): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return `<span class="fg-desconhecido">NÃO MEDIDO</span>`;
  return `${v.toLocaleString("pt-BR", { maximumFractionDigits: casas, minimumFractionDigits: casas })}${sufixo}`;
}

/** Número da cauda: `null` aqui é falta de amostra, não falta de medição. */
function cauda(v: number | null, casas = 0, sufixo = ""): string {
  return v === null ? `<span class="fg-desconhecido">AMOSTRA CURTA</span>` : num(v, casas, sufixo);
}

function mediana(vs: (number | null)[]): number | null {
  const v = vs.filter((x): x is number => x !== null && Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = Math.floor(v.length / 2);
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
}

function desenharInicio(mensagem = "") {
  if (!raiz) return;
  raiz.innerHTML = `
    <div class="fg-painel">
      <p class="fg-lead">
        Mede por ${SEGUNDOS} segundos o que limita o PC: processador (inclusive o núcleo mais ocupado),
        memória, disco, placa de vídeo e memória de vídeo. Com um jogo aberto, mede os quadros dele
        no mesmo intervalo — FPS, 1% e 0,1% piores, P99 e cada travada por gravidade.
      </p>
      <div class="fg-linha">
        <button class="btn btn-primary" id="mapa-medir-jogo">Medir no jogo (começa em ${ESPERA_PARA_VOLTAR_AO_JOGO} s)</button>
        <button class="btn" id="mapa-medir-agora">Medir agora</button>
      </div>
      <p class="fg-nota">
        "Medir no jogo" dá ${ESPERA_PARA_VOLTAR_AO_JOGO} segundos para você voltar para o jogo e jogar normalmente
        enquanto mede. Os números saem dos contadores do próprio Windows — nada é injetado no jogo.
      </p>
      ${mensagem ? `<p class="fg-erro">${esc(mensagem)}</p>` : ""}
    </div>`;
  raiz.querySelector("#mapa-medir-jogo")?.addEventListener("click", () => void medir(ESPERA_PARA_VOLTAR_AO_JOGO));
  raiz.querySelector("#mapa-medir-agora")?.addEventListener("click", () => void medir(0));
}

function progresso(texto: string, pct: number) {
  if (!raiz) return;
  raiz.innerHTML = `
    <div class="fg-progresso">
      <div class="fg-progresso-trilho"><div class="fg-progresso-barra" style="width:${pct}%"></div></div>
      <span class="fg-progresso-texto">${esc(texto)}</span>
    </div>`;
}

async function medir(espera: number) {
  if (rodando) return;
  rodando = true;
  try {
    for (let s = espera; s > 0; s--) {
      progresso(`Volte para o jogo — a medição começa em ${s} s.`, 0);
      await new Promise((r) => setTimeout(r, 1000));
    }
    const inicio = Date.now();
    const timer = setInterval(() => {
      const pct = Math.min(100, ((Date.now() - inicio) / 1000 / SEGUNDOS) * 100);
      progresso(`Medindo… jogue normalmente. (${Math.round(pct)}%)`, pct);
    }, 400);
    let d: Diagnostico;
    try {
      d = await invoke<Diagnostico>("diagnostico_ao_vivo", { segundos: SEGUNDOS });
    } finally {
      clearInterval(timer);
    }
    desenhar(d);
  } catch (e) {
    desenharInicio(String(e));
  } finally {
    rodando = false;
  }
}

function desenhar(d: Diagnostico) {
  if (!raiz) return;
  const a = d.amostras;
  const cpu = mediana(a.map((x) => x.cpu_total_pct));
  const nucleo = mediana(a.map((x) => x.cpu_nucleo_max_pct));
  const gpu = mediana(a.map((x) => x.gpu_pct));
  const vram = mediana(a.map((x) => x.vram_usada_mb));
  const ram = mediana(a.map((x) => x.ram_disponivel_mb));
  const disco = mediana(a.map((x) => x.disco_latencia_ms));
  const clock = mediana(a.map((x) => x.clock_efetivo_mhz));

  const presos = new Set<No>(d.gargalo.achados.map((g) => NA_TELA[g.classe].no));
  const no = (id: No, titulo: string, valor: string, sub: string) =>
    `<div class="mapa-no" data-preso="${presos.has(id)}">
       <span>${titulo}</span><b>${valor}</b><em>${sub}</em>
     </div>`;

  const vramTotal = d.placa?.vram_total_mb ?? null;
  const mapa = `
    <div class="mapa-fluxo">
      ${no("cpu", "PROCESSADOR", num(cpu, 0, "%"), `núcleo mais ocupado ${num(nucleo, 0, "%")} · ${num(clock, 0, " MHz")} efetivos`)}
      ${no("ram", "MEMÓRIA", num(ram, 0, " MB"), "livre")}
      ${no("disco", "DISCO", num(disco, 1, " ms"), "latência média")}
      ${no("gpu", "PLACA DE VÍDEO", num(gpu, 0, "%"), esc(d.placa?.nome ?? "placa não identificada"))}
      ${no("vram", "MEMÓRIA DE VÍDEO", num(vram, 0, " MB"), vramTotal ? `de ${num(vramTotal, 0, " MB")}` : "total não lido")}
      ${no("tela", "JOGO", d.saude ? num(d.saude.fps_medio, 0, " FPS") : "—", esc(d.jogo ?? "nenhum jogo aberto"))}
    </div>`;

  const g = d.gargalo;
  const gargalos = g.achados.length
    ? g.achados
        .map((a) => {
          const t = NA_TELA[a.classe];
          return `
          <article class="mapa-gargalo">
            <header>
              <b>${esc(t.titulo)}</b>
              <span class="fg-chip">${a.forca === "Causa" ? "medido agora" : "hipótese"}</span>
            </header>
            <p>${esc(t.explica)}</p>
            <p class="fg-nota"><strong>O que fazer:</strong> ${esc(t.faz)}</p>
            <div class="fg-linha"><span class="fg-chip">${esc(a.evidencia)}</span></div>
          </article>`;
        })
        .join("")
    : `<p class="fg-aviso">${
        g.conclusao === "SemCarga"
          ? "A máquina estava parada durante a medição — sem carga não existe gargalo para encontrar. Meça com o jogo rodando."
          : g.conclusao === "SemEvidencia"
            ? "Não houve medição suficiente para classificar."
            : d.jogo
              ? "Nada encostou no limite nesta medição. Com o jogo aberto, isso costuma querer dizer que o limite é o próprio jogo (o motor, o servidor) — ou que a máquina está folgada para essa cena."
              : "Nada encostou no limite. Sem jogo aberto é o esperado: meça com o jogo rodando."
      }</p>`;
  const cobertura = `<p class="fg-nota">${g.classes_avaliadas} de ${g.classes_totais} tipos de gargalo puderam ser avaliados nesta medição${
    g.nao_verificado.length ? ` — sem dado para: ${g.nao_verificado.map((n) => esc(n.classe)).join(", ")}` : ""
  }. Mesmo classificador do painel ao vivo.</p>`;

  let quadros = "";
  if (d.saude) {
    const s = d.saude;
    quadros = `
      <h3 class="fg-sub">Os quadros de ${esc(d.jogo ?? "jogo")} (${num(s.duracao_s, 0, " s")}, ${num(s.quadros)} quadros)</h3>
      <div class="fg-numeros">
        <div><span>FPS médio</span><span class="fg-valor">${num(s.fps_medio, 0)}</span></div>
        <div><span>1% piores</span><span class="fg-valor">${cauda(s.low_1pct_fps, 0)}</span></div>
        <div><span>0,1% piores</span><span class="fg-valor">${cauda(s.low_01pct_fps, 0)}</span></div>
        <div><span>P99 do tempo de quadro</span><span class="fg-valor">${cauda(s.p99_ms, 1, " ms")}</span></div>
        <div><span>Índice de fluidez</span><span class="fg-valor">${cauda(s.indice_de_fluidez, 0)}</span></div>
      </div>
      <dl class="fg-grade">
        <div><dt>Travadas leves</dt><dd>${num(s.engasgos.micro)}</dd></div>
        <div><dt>Perceptíveis</dt><dd>${num(s.engasgos.perceptivel)}</dd></div>
        <div><dt>Graves</dt><dd>${num(s.engasgos.severo)}</dd></div>
        <div><dt>Congelamentos (≥ 100 ms)</dt><dd>${num(s.engasgos.extremo)}</dd></div>
      </dl>
      <p class="fg-nota">
        "0,1% piores" só aparece com pelo menos 10.000 quadros, e "1% piores" e P99 com 2.000 — abaixo disso
        seriam poucos quadros para dizer alguma coisa. O índice de fluidez é calculado a partir desses números
        (consistência do 1%, cauda do P99 e travadas graves por minuto) e nunca aparece sem eles.
      </p>`;
  } else if (d.jogo && d.quadros_erro) {
    quadros = `<p class="fg-aviso">O jogo ${esc(d.jogo)} estava aberto, mas os quadros não puderam ser medidos: ${esc(d.quadros_erro)}</p>`;
  }

  let detetive = "";
  if (d.travadas) {
    const t = d.travadas;
    if (!t.travadas.length) {
      detetive = `<p class="fg-nota">Nenhuma travada perceptível nesta medição.</p>`;
    } else {
      const pistas = t.pistas
        .map((p) => {
          const f = fraseDoSuspeito(p.suspeito);
          return `<article class="mapa-gargalo"><header><b>Em ${p.travadas} de ${t.travadas.length} travada(s), ${esc(f.titulo)}</b><span class="fg-chip">coincidência ${p.forca === "Alta" ? "forte" : "média"}</span></header><p class="fg-nota"><strong>O que fazer:</strong> ${esc(f.faz)}</p></article>`;
        })
        .join("");
      detetive = `
        <h3 class="fg-sub">Detetive de travadas: o que o PC fazia na hora de cada uma</h3>
        ${pistas}
        ${t.sem_causa_visivel ? `<p class="fg-nota">${t.sem_causa_visivel} travada(s) sem nenhum salto de disco, memória, placa ou programa na mesma hora. Isso costuma ser compilação de shader ou o próprio jogo — e nenhum ajuste de Windows resolve.</p>` : ""}
        <p class="fg-nota">A telemetria é lida a cada meio segundo, então isto é coincidência no tempo, não causa provada. Coincidência forte = aconteceu em pelo menos metade das travadas.</p>`;
    }
  }

  raiz.innerHTML = `
    <div class="fg-painel">
      ${mapa}
      ${gargalos}
      ${cobertura}
      ${quadros}
      ${detetive}
      <div id="mapa-tetos"></div>
      <div class="fg-linha">
        <button class="btn" id="mapa-de-novo-jogo">Medir de novo no jogo</button>
        <button class="btn btn-ghost" id="mapa-de-novo">Medir agora</button>
      </div>
    </div>`;
  raiz.querySelector("#mapa-de-novo-jogo")?.addEventListener("click", () => void medir(ESPERA_PARA_VOLTAR_AO_JOGO));
  raiz.querySelector("#mapa-de-novo")?.addEventListener("click", () => void medir(0));
  void desenharTetos();
}

export function carregarMapaDeDesempenho() {
  if (raiz) return;
  raiz = document.getElementById("mapa-desempenho");
  if (!raiz) return;
  desenharInicio();
}
