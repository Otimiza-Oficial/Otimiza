<script lang="ts">
  // A casca da interface nova: uma área de trabalho no céu (referência em `brand/nova-geracao/referencia-interface.png`,
  // regras em `ui/DESIGN.md`). O mascote mora na mesa; cada área é um objeto que se abre numa janela; os widgets
  // ficam no painel da direita e podem ser puxados para a mesa. Com jogo aberto, o mascote voa para a barra de menu e a
  // interface para de animar.
  import { onMount, tick } from "svelte";
  import { cubicIn, backOut } from "svelte/easing";
  import Ceu from "./componentes/Ceu.svelte";
  import Historico from "./componentes/Historico.svelte";
  import Latencia from "./componentes/Latencia.svelte";
  import Rede from "./componentes/Rede.svelte";
  import Tela from "./componentes/Tela.svelte";
  import Mascote from "./componentes/Mascote.svelte";
  import { areas, type IdDaArea } from "./lib/areas";
  import { avisos as lerAvisos, climaDe, fraseDoMascote, horaDe, type EstadoDoMascote, type Tema } from "./lib/clima";
  import {
    noTauri,
    ponte,
    type Hardware,
    type JogoAberto,
    type JogoDaLoja,
    type MetricasAoVivo,
    type ResumoDoJogo,
    type VersaoDoApp,
  } from "./lib/ponte";

  type Ponto = { x: number; y: number };
  type IdDoWidget = "agora" | "jogo" | "pc";

  const CHAVE_DO_TEMA = "otimiza.tema";
  const CHAVE_DA_MESA = "otimiza.mesa";
  const CHAVE_DOS_WIDGETS = "otimiza.widgets";
  const WIDGETS: IdDoWidget[] = ["agora", "jogo", "pc"];

  // As dicas que o mascote dá quando alguém clica nele: todas são coisas que a interface faz de verdade.
  const DICAS = [
    "Arraste um objeto até mim que eu abro ele pra você.",
    "Puxe um widget do painel da direita para a mesa.",
    "Tudo o que eu mudo no PC dá para desfazer no Histórico.",
    "Número com o selo Medido saiu de uma medição, nunca de chute.",
    "Quando você abre um jogo, eu vou pro cantinho para não pesar.",
  ];

  let versao = $state<VersaoDoApp | null>(null);
  let hardware = $state<Hardware | null>(null);
  let metricas = $state<MetricasAoVivo | null>(null);
  let jogo = $state<JogoAberto | null>(null);
  let biblioteca = $state<ResumoDoJogo[]>([]);
  let trilho = $state<{ jogo: JogoDaLoja; capa: string | null }[]>([]);
  let agora = $state(new Date());
  let tema = $state<Tema>(lerTema());
  let aberta = $state<IdDaArea>("inicio");
  let origem = $state<Ponto | null>(null);

  // A mesa: o deslocamento de cada objeto e os widgets que saíram do painel.
  let posicoes = $state<Record<string, Ponto>>(ler(CHAVE_DA_MESA, {}));
  let fixados = $state<Partial<Record<IdDoWidget, Ponto>>>(ler(CHAVE_DOS_WIDGETS, {}));

  // O mascote: reação curta por cima do estado, fala e a animação em curso.
  let reacao = $state<EstadoDoMascote | null>(null);
  let fala = $state<string | null>(null);
  let animacao = $state<"" | "pula" | "engole" | "pro-jogo" | "volta">("");
  let voo = $state({ dx: 0, dy: 0, s: 1 });
  /** O jogo já está "valendo" na interface (classe `jogo-aberto`): só depois da animação de saída do mascote. */
  let jogoNaTela = $state(false);
  let chegouNoCanto = $state(false);

  let palco: HTMLDivElement | undefined = $state();
  let mini: HTMLSpanElement | undefined = $state();
  let mesa: HTMLElement | undefined = $state();
  let lateral: HTMLElement | undefined = $state();

  // Arrastar: um objeto da mesa ou um widget.
  type Arrasto = {
    tipo: "objeto" | "widget";
    id: string;
    inicio: Ponto;
    base: Ponto;
    ativo: boolean;
    sobreMascote: boolean;
    /** Widget saindo do painel: onde o fantasma está e onde o ponteiro pegou nele. */
    fantasma?: Ponto;
    pega?: Ponto;
  };
  let arrasto = $state<Arrasto | null>(null);
  let engolindo = $state<string | null>(null);

  const lista = $derived(lerAvisos(hardware, metricas, jogo !== null));
  const clima = $derived(climaDe(lista));
  const hora = $derived(horaDe(agora));
  const noturno = $derived(tema === "noite" || (tema === "auto" && hora === "noite"));
  const estadoBase = $derived<EstadoDoMascote>(
    jogo ? "focado" : lista.some((a) => a.gravidade === "serio") ? "preocupado" : "atento"
  );
  const estado = $derived<EstadoDoMascote>(reacao ?? (arrasto?.sobreMascote ? "surpreso" : estadoBase));
  const ultimaPartida = $derived(
    biblioteca
      .filter((j) => j.ultima)
      .sort((a, b) => (b.ultima?.quando ?? 0) - (a.ultima?.quando ?? 0))[0] ?? null
  );
  const capaDaUltima = $derived(trilho.find((t) => t.jogo.nome === ultimaPartida?.nome)?.capa ?? null);
  const area = $derived(areas.find((a) => a.id === aberta) ?? areas[0]);
  const objetos = $derived(areas.filter((a) => a.id !== "inicio"));
  const telaPrincipal = $derived(hardware?.telas.find((t) => t.principal) ?? hardware?.telas[0] ?? null);
  const memoriaPct = $derived.by(() => {
    const total = metricas?.ram.total_gb ?? null;
    const usada = metricas?.ram.used_gb ?? null;
    return total != null && usada != null && total > 0 ? usada / total : null;
  });
  const mesaMexida = $derived(Object.keys(posicoes).length > 0 || Object.keys(fixados).length > 0);
  const avisoDeTela = $derived(lista.find((a) => a.id.startsWith("hz-")) ?? null);

  function ler<T>(chave: string, padrao: T): T {
    try {
      const t = localStorage.getItem(chave);
      return t ? (JSON.parse(t) as T) : padrao;
    } catch {
      return padrao;
    }
  }

  function gravar(chave: string, valor: unknown) {
    try {
      localStorage.setItem(chave, JSON.stringify(valor));
    } catch {
      // Sem armazenamento: só não lembra na próxima abertura.
    }
  }

  function lerTema(): Tema {
    const salvo = ler<string>(CHAVE_DO_TEMA, "auto");
    // Gravado como texto puro nas versões anteriores da casca.
    try {
      const cru = localStorage.getItem(CHAVE_DO_TEMA);
      if (cru === "auto" || cru === "por-do-sol" || cru === "lavanda" || cru === "noite") return cru;
    } catch {
      // segue
    }
    return salvo === "por-do-sol" || salvo === "lavanda" || salvo === "noite" ? salvo : "auto";
  }

  function escolherTema(t: Tema) {
    tema = t;
    try {
      localStorage.setItem(CHAVE_DO_TEMA, t);
    } catch {
      // Só não lembra na próxima abertura.
    }
  }

  $effect(() => {
    document.documentElement.classList.toggle("jogo-aberto", jogoNaTela);
    document.documentElement.dataset.tema = noturno ? "noite" : "dia";
  });

  // ---------------------------------------------------------------------------------------------------------------
  // Mascote

  let tempoDaReacao: ReturnType<typeof setTimeout> | undefined;
  let tempoDaFala: ReturnType<typeof setTimeout> | undefined;

  function reagir(r: EstadoDoMascote, ms: number) {
    clearTimeout(tempoDaReacao);
    reacao = r;
    tempoDaReacao = setTimeout(() => (reacao = null), ms);
  }

  function falar(texto: string, ms = 3800) {
    clearTimeout(tempoDaFala);
    fala = texto;
    tempoDaFala = setTimeout(() => (fala = null), ms);
  }

  function animar(a: typeof animacao, ms: number) {
    animacao = a;
    setTimeout(() => {
      if (animacao === a) animacao = "";
    }, ms);
  }

  let dica = 0;
  function cutucar() {
    if (jogoNaTela || animacao) return;
    animar("pula", 650);
    reagir("feliz", 1400);
    falar(DICAS[dica++ % DICAS.length]);
  }

  /** Do mascote da mesa até o da barra de menu: a distância e a escala do voo. */
  function medirVoo() {
    if (!palco || !mini) return;
    const a = palco.getBoundingClientRect();
    const b = mini.getBoundingClientRect();
    voo = {
      dx: b.left + b.width / 2 - (a.left + a.width / 2),
      dy: b.top + b.height / 2 - (a.top + a.height / 2),
      s: b.width / a.width,
    };
  }

  // O jogo abriu ou fechou: o mascote vai para o canto ou volta. Na primeira leitura (o Otimiza abriu com o jogo
  // já rodando) não há cena: ele já começa no canto.
  let jogoAnterior: string | null | undefined = undefined;
  $effect(() => {
    const nome = jogo?.nome ?? null;
    if (jogoAnterior === undefined) {
      jogoAnterior = nome;
      jogoNaTela = nome !== null;
      chegouNoCanto = nome !== null;
      return;
    }
    if (nome === jogoAnterior) return;
    const antes = jogoAnterior;
    jogoAnterior = nome;
    if (nome && !antes) {
      aberta = "inicio";
      medirVoo();
      falar(`${nome} aberto. Vou pro cantinho para não pesar no jogo.`, 2600);
      animacao = "pro-jogo";
      setTimeout(() => (chegouNoCanto = true), 1250);
      // Só agora a interface congela: a cena acima precisa das animações.
      setTimeout(() => {
        animacao = "";
        jogoNaTela = true;
      }, 1700);
    } else if (!nome && antes) {
      jogoNaTela = false;
      chegouNoCanto = false;
      medirVoo();
      animar("volta", 900);
      reagir("feliz", 2600);
      falar(`Voltei! Se você jogou ${antes} comigo aberto, a partida está no Último jogo.`, 4200);
    }
  });

  // ---------------------------------------------------------------------------------------------------------------
  // Janelas

  function abrir(id: IdDaArea, de: Ponto | null) {
    origem = de;
    aberta = id;
  }

  function centro(el: Element): Ponto {
    const r = el.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  }

  /** A janela cresce a partir do objeto que a abriu (ou do mascote, se foi engolido). */
  function deOrigem(node: HTMLElement, { de, saida = false }: { de: Ponto | null; saida?: boolean }) {
    const r = node.getBoundingClientRect();
    const dx = de ? de.x - (r.left + r.width / 2) : 0;
    const dy = de ? de.y - (r.top + r.height / 2) : 24;
    return {
      duration: saida ? 180 : 420,
      easing: saida ? cubicIn : backOut,
      css: (t: number, u: number) =>
        `transform: translate(${u * dx}px, ${u * dy}px) scale(${0.12 + 0.88 * t}); opacity: ${Math.min(1, t * 2.5)}`,
    };
  }

  // ---------------------------------------------------------------------------------------------------------------
  // Arrastar

  function sobre(el: Element | undefined, p: Ponto, folga = 0) {
    if (!el) return false;
    const r = el.getBoundingClientRect();
    return p.x >= r.left - folga && p.x <= r.right + folga && p.y >= r.top - folga && p.y <= r.bottom + folga;
  }

  function pegarObjeto(e: PointerEvent, id: string) {
    if (jogoNaTela || e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    arrasto = {
      tipo: "objeto",
      id,
      inicio: { x: e.clientX, y: e.clientY },
      base: posicoes[id] ?? { x: 0, y: 0 },
      ativo: false,
      sobreMascote: false,
    };
  }

  function moverObjeto(e: PointerEvent) {
    const a = arrasto;
    if (!a || a.tipo !== "objeto") return;
    const dx = e.clientX - a.inicio.x;
    const dy = e.clientY - a.inicio.y;
    if (!a.ativo && Math.hypot(dx, dy) < 5) return;
    a.ativo = true;
    posicoes[a.id] = { x: a.base.x + dx, y: a.base.y + dy };
    a.sobreMascote = sobre(palco, { x: e.clientX, y: e.clientY }, 24);
  }

  async function soltarObjeto(e: PointerEvent) {
    const a = arrasto;
    if (!a || a.tipo !== "objeto") return;
    arrasto = null;
    const el = e.currentTarget as HTMLElement;
    if (!a.ativo) {
      abrir(a.id as IdDaArea, centro(el));
      return;
    }
    if (a.sobreMascote && palco) {
      // Engolido: o objeto vai até o centro do mascote encolhendo, o mascote engole e a janela sai dele.
      const alvo = centro(palco);
      const agoraEm = centro(el);
      const atual = posicoes[a.id] ?? { x: 0, y: 0 };
      engolindo = a.id;
      posicoes[a.id] = { x: atual.x + alvo.x - agoraEm.x, y: atual.y + alvo.y - agoraEm.y };
      setTimeout(() => {
        animar("engole", 520);
        reagir("feliz", 1300);
        falar(`Nhac! Abrindo ${areas.find((x) => x.id === a.id)?.nome ?? ""}.`, 1400);
      }, 260);
      setTimeout(async () => {
        if (a.base.x === 0 && a.base.y === 0) delete posicoes[a.id];
        else posicoes[a.id] = a.base;
        engolindo = null;
        gravar(CHAVE_DA_MESA, posicoes);
        await tick();
        abrir(a.id as IdDaArea, alvo);
      }, 560);
      return;
    }
    gravar(CHAVE_DA_MESA, posicoes);
  }

  function pegarWidget(e: PointerEvent, id: IdDoWidget) {
    if (jogoNaTela || e.button !== 0 || (e.target as Element).closest("button, a")) return;
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const r = el.getBoundingClientRect();
    arrasto = {
      tipo: "widget",
      id,
      inicio: { x: e.clientX, y: e.clientY },
      base: fixados[id] ?? { x: 0, y: 0 },
      ativo: false,
      sobreMascote: false,
      fantasma: { x: r.left, y: r.top },
      pega: { x: e.clientX - r.left, y: e.clientY - r.top },
    };
  }

  function moverWidget(e: PointerEvent) {
    const a = arrasto;
    if (!a || a.tipo !== "widget" || !a.pega) return;
    if (!a.ativo && Math.hypot(e.clientX - a.inicio.x, e.clientY - a.inicio.y) < 5) return;
    a.ativo = true;
    a.fantasma = { x: e.clientX - a.pega.x, y: e.clientY - a.pega.y };
  }

  function soltarWidget(e: PointerEvent) {
    const a = arrasto;
    if (!a || a.tipo !== "widget" || !a.pega) return;
    arrasto = null;
    if (!a.ativo || !mesa) return;
    const id = a.id as IdDoWidget;
    const p = { x: e.clientX, y: e.clientY };
    if (sobre(lateral, p)) {
      // Devolvido ao painel.
      delete fixados[id];
    } else if (sobre(mesa, p)) {
      const m = mesa.getBoundingClientRect();
      fixados[id] = {
        x: Math.max(8, Math.min(m.width - 260, e.clientX - a.pega.x - m.left)),
        y: Math.max(8, Math.min(m.height - 120, e.clientY - a.pega.y - m.top)),
      };
    }
    gravar(CHAVE_DOS_WIDGETS, fixados);
  }

  function arrumarMesa() {
    posicoes = {};
    fixados = {};
    gravar(CHAVE_DA_MESA, {});
    gravar(CHAVE_DOS_WIDGETS, {});
  }

  // ---------------------------------------------------------------------------------------------------------------
  // Formatos

  function pct(v: number | null | undefined) {
    return v == null ? "—" : `${Math.round(v)}%`;
  }

  /** `null` é "não medido": aparece como travessão, nunca como zero. */
  function decimal(v: number | null) {
    return v == null ? "—" : v.toFixed(1).replace(".", ",");
  }

  function inteiro(v: number | null) {
    return v == null ? "—" : String(Math.round(v));
  }

  /** "Intel(R) Core(TM) i3-10100F CPU @ 3.60GHz" vira "Intel Core i3-10100F": a ficha cabe numa linha. */
  function nomeCurto(nome: string) {
    return nome
      .replace(/\((R|TM)\)/gi, "")
      .replace(/\s+CPU\s*@.*$/i, "")
      .replace(/^NVIDIA\s+/i, "")
      .replace(/\s+/g, " ")
      .trim();
  }

  /** Sem capa: as iniciais do nome ("Counter-Strike 2" → "CS"), não uma letra solta. */
  function iniciais(nome: string) {
    const palavras = nome.split(/[\s:-]+/).filter((p) => /^[A-Za-zÀ-ú]/.test(p));
    return (palavras.length > 1 ? palavras[0][0] + palavras[1][0] : nome.slice(0, 2)).toUpperCase();
  }

  /** O retrato de hardware é de quando foi lido: depois de uma troca na área Tela, o widget pega o Hz novo dela. */
  function trocouHz(dispositivo: string, hz: number) {
    if (!hardware) return;
    hardware = {
      ...hardware,
      telas: hardware.telas.map((t) => (t.dispositivo.toLowerCase() === dispositivo.toLowerCase() ? { ...t, hz_atual: hz } : t)),
    };
  }

  async function janela(acao: "minimizar" | "maximizar" | "fechar") {
    if (!noTauri) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const w = getCurrentWindow();
    if (acao === "minimizar") await w.minimize();
    else if (acao === "maximizar") await w.toggleMaximize();
    else await w.close();
  }

  onMount(() => {
    let vivo = true;
    let espera: ReturnType<typeof setTimeout> | undefined;

    void (async () => {
      versao = await ponte.versao().catch(() => null);
      hardware = await ponte.hardwareUltimo().catch(() => null);
      ponte.hardware().then((h) => (hardware = h)).catch(() => {});
      ponte.biblioteca().then((b) => (biblioteca = b)).catch(() => {});
      ponte
        .instalados()
        .then(async (lista) => {
          const recentes = [...lista].sort((a, b) => b.ultima_vez - a.ultima_vez).slice(0, 5);
          trilho = recentes.map((j) => ({ jogo: j, capa: null }));
          for (const [i, j] of recentes.entries()) {
            if (j.appid == null) continue;
            const capa = await ponte.capa(j.appid).catch(() => null);
            if (vivo && capa) trilho[i] = { jogo: j, capa };
          }
        })
        .catch(() => {});
      await ponte.iniciarMonitor().catch(() => {});
    })();

    // Um laço só. Com o jogo aberto ele só confere se o jogo continua lá, devagar; com a janela escondida, nem isso
    // além do jogo. A interface não pode ser o motivo de um quadro perdido.
    const laco = async () => {
      if (!vivo) return;
      jogo = await ponte.jogoAberto().catch(() => null);
      if (!jogo && !document.hidden) {
        metricas = await ponte.metricas().catch(() => metricas);
        agora = new Date();
      }
      espera = setTimeout(laco, jogo ? 5000 : document.hidden ? 4000 : 2000);
    };
    void laco();

    return () => {
      vivo = false;
      clearTimeout(espera);
    };
  });

  /** Só no navegador: simula um jogo abrindo e fechando para ver a cena do mascote. */
  async function alternarJogoDeDemonstracao() {
    ponte.simularJogo(jogo ? null : "FiveM");
    jogo = await ponte.jogoAberto();
  }
</script>

{#snippet widget(id: IdDoWidget)}
  {#if id === "agora"}
    <div class="par">
      <div class="vidro pequeno">
        <span class="rotulo">Processador</span>
        <span class="numero enorme">{jogo ? "—" : pct(metricas?.cpu.overall)}</span>
        <span class="legenda">{jogo ? "pausado no jogo" : "em uso agora"}</span>
      </div>
      <div class="vidro pequeno">
        <span class="rotulo">Memória</span>
        <span class="numero enorme">{jogo || memoriaPct == null ? "—" : pct(memoriaPct * 100)}</span>
        <span class="legenda">
          {jogo || metricas?.ram.used_gb == null
            ? "pausado no jogo"
            : `${decimal(metricas.ram.used_gb)} de ${decimal(metricas.ram.total_gb)} GB`}
        </span>
      </div>
    </div>
  {:else if id === "jogo"}
    <div class="vidro">
      <div class="cabeca-widget">
        <span class="rotulo">Último jogo</span>
        <button class="mini-botao" onclick={() => abrir("jogos", null)}>Jogos</button>
      </div>
      {#if ultimaPartida?.ultima}
        <div class="cartao-branco">
          <div class="linha-jogo">
            <span class="capinha">
              {#if capaDaUltima}<img src={capaDaUltima} alt="" />{:else}{iniciais(ultimaPartida.nome)}{/if}
            </span>
            <div>
              <strong>{ultimaPartida.nome}</strong>
              <span class="cinza">
                <span class="numero">{inteiro(ultimaPartida.ultima.fps_nativo)}</span> FPS ·
                <span class="numero">{inteiro(ultimaPartida.ultima.low_1pct)}</span> no 1% pior
              </span>
            </div>
          </div>
          {#if ultimaPartida.ganho_low.estado === "Medido" && ultimaPartida.ganho_low.pct != null}
            <div class="linha-ganho">
              <span class="ganho">
                {ultimaPartida.ganho_low.pct >= 0 ? "+" : ""}{decimal(ultimaPartida.ganho_low.pct)}% no 1% pior
              </span>
              <span class="selo-medido">Medido</span>
            </div>
          {/if}
        </div>
      {:else}
        <p class="legenda">Jogue uma partida com o Otimiza aberto e o resultado aparece aqui.</p>
      {/if}
    </div>
  {:else if hardware}
    <div class="vidro">
      <div class="cabeca-widget">
        <span class="rotulo">Este PC</span>
        <button class="mini-botao" onclick={() => abrir("tela", null)}>Tela</button>
      </div>
      {#if telaPrincipal}
        <div class="destaque-pc">
          <img src="/objetos/tela.png" alt="" width="30" height="30" />
          <span class="numero enorme">{telaPrincipal.hz_atual}</span>
          <span class="legenda">Hz{telaPrincipal.hz_maximo > telaPrincipal.hz_atual ? ` · aguenta ${telaPrincipal.hz_maximo}` : " · no máximo"}</span>
        </div>
      {/if}
      <ul class="ficha">
        <li><span>Processador</span><span>{nomeCurto(hardware.processador)}</span></li>
        <li><span>Placa</span><span>{nomeCurto(hardware.placa.nome)}</span></li>
        <li>
          <span>Memória</span><span>{decimal(hardware.memoria_gb)} GB</span>
        </li>
      </ul>
      {#if memoriaPct != null && !jogo}
        <div class="trilha"><div style:width={`${Math.min(100, memoriaPct * 100)}%`}></div></div>
      {/if}
    </div>
  {/if}
{/snippet}

<Ceu {clima} {hora} {tema} />

<div class="casca" class:arrastando={arrasto?.ativo}>
  <header class="menu" data-tauri-drag-region>
    <div class="marca" data-tauri-drag-region>
      <span class="mini" class:chegou={chegouNoCanto} bind:this={mini}>
        <Mascote estado={chegouNoCanto ? "focado" : "atento"} tamanho={20} />
      </span>
      <strong>Otimiza</strong>
      {#if versao}<span class="fraco">{versao.exibida}</span>{/if}
      {#if aberta !== "inicio"}<span class="fraco">› {area.nome}</span>{/if}
    </div>

    <div class="situacao" data-tauri-drag-region>
      {#if !noTauri}
        <button class="texto-menu" onclick={alternarJogoDeDemonstracao}>
          {jogo ? "Fechar o jogo de teste" : "Simular jogo aberto"}
        </button>
        <span class="demo">Demonstração</span>
      {/if}
      {#if jogo}
        <span class="jogo"><span class="ponto" data-tom="ok"></span>{jogo.nome}</span>
      {/if}
      <span class="fraco">{agora.toLocaleDateString("pt-BR", { weekday: "short", day: "numeric", month: "short" })}</span>
      <span class="numero">{agora.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })}</span>
    </div>

    <div class="controles">
      <button aria-label="Minimizar" onclick={() => janela("minimizar")}>
        <svg viewBox="0 0 12 12" width="11" height="11"><path d="M2 6h8" /></svg>
      </button>
      <button aria-label="Maximizar" onclick={() => janela("maximizar")}>
        <svg viewBox="0 0 12 12" width="11" height="11"><rect x="2" y="2" width="8" height="8" rx="1.5" /></svg>
      </button>
      <button aria-label="Fechar" class="fechar" onclick={() => janela("fechar")}>
        <svg viewBox="0 0 12 12" width="11" height="11"><path d="M3 3l6 6M9 3l-6 6" /></svg>
      </button>
    </div>
  </header>

  <nav class="trilho" aria-label="Seus jogos">
    <button class="aba" onclick={() => abrir("jogos", null)}><span>Jogos</span></button>
    {#each trilho as { jogo: j, capa } (j.nome)}
      <button class="disco" title={j.nome} aria-label={j.nome} onclick={() => abrir("jogos", null)}>
        {#if capa}<img src={capa} alt="" />{:else}<span>{iniciais(j.nome)}</span>{/if}
      </button>
    {/each}
  </nav>

  <main class="mesa" bind:this={mesa}>
    <section class="perfil">
      <div class="palco-mascote">
        {#if fala && aberta === "inicio"}
          <div class="fala" role="status">{fala}</div>
        {/if}
        <div
          class="mascote-anima {animacao}"
          class:no-canto={jogoNaTela}
          bind:this={palco}
          style:--dx={`${voo.dx}px`}
          style:--dy={`${voo.dy}px`}
          style:--s={voo.s}
        >
          <button class="mascote-botao" aria-label="Mascote do Otimiza" onclick={cutucar}>
            <Mascote {estado} tamanho={150} />
          </button>
        </div>
      </div>
      <div class="perfil-texto">
        <span class="rotulo">{jogo ? "Jogando agora" : "Seu PC agora"}</span>
        <h1>
          {#if jogo}
            {jogo.nome} aberto. Estou no cantinho da barra de menu, quieto.
          {:else}
            {fraseDoMascote(clima, null, lista)}
          {/if}
        </h1>
        <p>
          {#if jogo}
            Com o jogo aberto a interface para de animar e só confere, a cada 5 segundos, se ele continua lá.
          {:else if avisoDeTela}
            {avisoDeTela.titulo}. {avisoDeTela.detalhe}
          {:else if lista.length > 0}
            {lista[0].titulo}. {lista[0].detalhe}
          {:else}
            Olhei a tela e a memória. Rede, temperatura e disco entram nas próximas ondas.
          {/if}
        </p>
        {#if !jogo}
          {#if avisoDeTela}
            <button class="link" onclick={(e) => abrir("tela", centro(e.currentTarget))}>Corrigir na Tela →</button>
          {:else}
            <button class="link" onclick={(e) => abrir("historico", centro(e.currentTarget))}>Ver o que foi mudado →</button>
          {/if}
        {/if}
      </div>
    </section>

    <section class="objetos" aria-label="Áreas">
      {#each objetos as a (a.id)}
        {@const p = posicoes[a.id] ?? { x: 0, y: 0 }}
        <button
          class="objeto"
          class:pego={arrasto?.tipo === "objeto" && arrasto.id === a.id && arrasto.ativo}
          class:engolido={engolindo === a.id}
          style:transform={`translate(${p.x}px, ${p.y}px)${engolindo === a.id ? " scale(0.15)" : ""}`}
          title={a.pergunta || a.nome}
          onpointerdown={(e) => pegarObjeto(e, a.id)}
          onpointermove={moverObjeto}
          onpointerup={soltarObjeto}
          onpointercancel={() => (arrasto = null)}
          onclick={(e) => {
            // Só o teclado (Enter/Espaço): o clique do mouse já abriu no `pointerup`.
            if (e.detail === 0) abrir(a.id, centro(e.currentTarget));
          }}
        >
          <img src={a.objeto} alt="" width="76" height="76" draggable="false" />
          <span>{a.nome}</span>
        </button>
      {/each}
    </section>

    {#each WIDGETS.filter((w) => fixados[w]) as id (id)}
      {@const p = fixados[id]!}
      <div
        class="widget fixado"
        class:sumindo={arrasto?.tipo === "widget" && arrasto.id === id && arrasto.ativo}
        style:left={`${p.x}px`}
        style:top={`${p.y}px`}
        role="group"
        aria-label="Widget na mesa"
        onpointerdown={(e) => pegarWidget(e, id)}
        onpointermove={moverWidget}
        onpointerup={soltarWidget}
      >
        {@render widget(id)}
      </div>
    {/each}

    {#if mesaMexida}
      <button class="arrumar" onclick={arrumarMesa}>Arrumar a mesa</button>
    {/if}

    {#if aberta !== "inicio"}
      {#key aberta}
        <section
          class="janela"
          aria-labelledby="titulo-da-area"
          in:deOrigem={{ de: origem }}
          out:deOrigem={{ de: origem, saida: true }}
        >
          <header class="janela-topo">
            <img src={area.objeto} alt="" width="28" height="28" />
            <div>
              <h1 id="titulo-da-area">{area.nome}</h1>
              {#if area.pergunta}<p class="apoio">{area.pergunta}</p>{/if}
            </div>
            <button class="fechar-janela" aria-label="Fechar" title="Fechar" onclick={() => (aberta = "inicio")}>
              <svg viewBox="0 0 12 12" width="12" height="12"><path d="M3 3l6 6M9 3l-6 6" /></svg>
            </button>
          </header>
          <div class="janela-corpo">
            {#if aberta === "historico"}
              <Historico />
            {:else if aberta === "rede"}
              <Rede />
            {:else if aberta === "latencia"}
              <Latencia />
            {:else if aberta === "tela"}
              <Tela aoMudarHz={trocouHz} />
            {:else}
              <div class="em-breve">
                <img src={area.objeto} alt="" width="96" height="96" />
                <p>
                  <strong>{area.nome}</strong> chega na Onda {area.onda} da Nova Geração. Até lá, ela continua na
                  interface atual do Otimiza.
                </p>
                {#if area.vai_ter.length > 0}
                  <ul class="lista">
                    {#each area.vai_ter as item (item)}
                      <li>{item}</li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {/if}
          </div>
        </section>
      {/key}
    {/if}
  </main>

  <aside class="lateral" aria-label="Widgets" bind:this={lateral}>
    <div class="vidro linha-tema">
      <span class="rotulo">Escolha o céu</span>
      <div class="temas" role="radiogroup" aria-label="Céu">
        {#each [["auto", "Céu da hora"], ["por-do-sol", "Pôr do sol"], ["lavanda", "Lavanda"], ["noite", "Noite"]] as [id, nome] (id)}
          <button
            role="radio"
            aria-checked={tema === id}
            aria-label={nome}
            title={nome}
            class="bolinha"
            data-cor={id}
            onclick={() => escolherTema(id as Tema)}
          ></button>
        {/each}
      </div>
    </div>

    {#each WIDGETS.filter((w) => !fixados[w]) as id (id)}
      <div
        class="widget"
        class:sumindo={arrasto?.tipo === "widget" && arrasto.id === id && arrasto.ativo}
        role="group"
        aria-label="Widget: arraste para a mesa"
        title="Arraste para a mesa"
        onpointerdown={(e) => pegarWidget(e, id)}
        onpointermove={moverWidget}
        onpointerup={soltarWidget}
      >
        {@render widget(id)}
      </div>
    {/each}
  </aside>

  <nav class="dock" aria-label="Áreas">
    {#each areas as a (a.id)}
      <button
        class="icone"
        class:ativa={aberta === a.id}
        aria-label={a.nome}
        onclick={(e) => (a.id === "inicio" ? (aberta = "inicio") : abrir(a.id, centro(e.currentTarget)))}
      >
        <img src={a.objeto} alt="" draggable="false" />
        <span class="dica">{a.nome}</span>
      </button>
    {/each}
  </nav>
</div>

{#if arrasto?.tipo === "widget" && arrasto.ativo && arrasto.fantasma}
  <div class="widget fantasma" style:left={`${arrasto.fantasma.x}px`} style:top={`${arrasto.fantasma.y}px`}>
    {@render widget(arrasto.id as IdDoWidget)}
  </div>
{/if}

<style>
  .casca {
    position: relative;
    z-index: 1;
    height: 100vh;
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) 276px;
    grid-template-rows: 30px minmax(0, 1fr) 88px;
    grid-template-areas:
      "menu menu menu"
      "trilho mesa lateral"
      "trilho dock lateral";
    color: #fff;
  }
  .casca.arrastando {
    cursor: grabbing;
  }

  .rotulo {
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    opacity: 0.85;
  }

  /* Barra de menu */
  .menu {
    grid-area: menu;
    display: flex;
    align-items: center;
    gap: 16px;
    padding-left: 12px;
    background: rgb(255 255 255 / 0.16);
    backdrop-filter: var(--desfoque);
    font-size: 12px;
    text-shadow: 0 1px 1px rgb(0 20 70 / 0.2);
  }
  .marca {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .marca strong {
    font-weight: 600;
  }
  .mini {
    display: inline-flex;
    width: 20px;
    height: 16px;
    align-items: center;
  }
  .mini.chegou {
    animation: chegar 520ms cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  @keyframes chegar {
    0% {
      transform: scale(0.4);
    }
    60% {
      transform: scale(1.7);
    }
    100% {
      transform: scale(1);
    }
  }
  .fraco {
    opacity: 0.8;
  }
  .situacao {
    flex: 1;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 14px;
    height: 100%;
  }
  .texto-menu {
    border: 0;
    background: none;
    color: #fff;
    font-size: 12px;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
    opacity: 0.9;
  }
  .demo {
    padding: 1px 6px;
    border-radius: 4px;
    background: rgb(255 214 150 / 0.25);
    color: #ffe2b8;
  }
  .jogo {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 500;
  }
  .controles {
    display: flex;
    height: 100%;
  }
  .controles button {
    width: 44px;
    border: 0;
    background: transparent;
    color: #fff;
    cursor: pointer;
  }
  .controles svg,
  .fechar-janela svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
  }
  .controles button:hover {
    background: rgb(255 255 255 / 0.18);
  }
  .controles .fechar:hover {
    background: #c42b1c;
  }

  /* Trilho: a aba "Jogos" em pé e os discos com as capas, como na referência. */
  .trilho {
    grid-area: trilho;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 64px 0 16px;
  }
  .aba {
    display: grid;
    place-items: center;
    width: 32px;
    height: 84px;
    margin-bottom: 6px;
    border: 1px solid rgb(255 255 255 / 0.4);
    border-radius: 8px;
    background: rgb(255 255 255 / 0.22);
    backdrop-filter: var(--desfoque);
    color: #fff;
    cursor: pointer;
  }
  .aba span {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }
  .disco {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border: 2px solid rgb(255 255 255 / 0.85);
    border-radius: 50%;
    background: rgb(255 255 255 / 0.3);
    box-shadow: 0 4px 10px rgb(10 40 110 / 0.25);
    overflow: hidden;
    cursor: pointer;
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    transition: transform 160ms ease;
  }
  .disco:hover {
    transform: scale(1.1);
  }
  .disco img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  /* Mesa */
  .mesa {
    grid-area: mesa;
    position: relative;
    min-height: 0;
    display: grid;
    align-content: start;
    justify-items: center;
    gap: 40px;
    padding: 56px 24px 0;
  }

  .perfil {
    width: min(620px, 100%);
    display: flex;
    align-items: center;
    gap: 22px;
    text-shadow: 0 1px 2px rgb(0 25 80 / 0.25);
  }
  .palco-mascote {
    position: relative;
    flex: none;
  }
  .mascote-botao {
    display: block;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
  }
  .mascote-anima {
    will-change: transform;
  }
  .mascote-anima.no-canto {
    opacity: 0;
  }
  .mascote-anima.pula {
    animation: pula 650ms ease;
  }
  .mascote-anima.engole {
    animation: engole 520ms ease;
  }
  .mascote-anima.pro-jogo {
    animation: pro-jogo 1500ms cubic-bezier(0.5, 0, 0.6, 1) forwards;
  }
  .mascote-anima.volta {
    animation: volta 900ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
  }
  @keyframes pula {
    0% {
      transform: none;
    }
    22% {
      transform: scale(1.14, 0.84);
    }
    52% {
      transform: translateY(-22px) scale(0.92, 1.1);
    }
    78% {
      transform: scale(1.05, 0.95);
    }
    100% {
      transform: none;
    }
  }
  @keyframes engole {
    0% {
      transform: none;
    }
    30% {
      transform: scale(1.18, 0.86);
    }
    60% {
      transform: scale(0.92, 1.1);
    }
    100% {
      transform: none;
    }
  }
  @keyframes pro-jogo {
    0% {
      transform: none;
    }
    18% {
      transform: translateY(-30px) scale(1.06, 0.96);
    }
    30% {
      transform: translateY(0) scale(1.12, 0.86);
    }
    55% {
      transform: translateY(-46px) rotate(360deg) scale(0.9);
      opacity: 1;
    }
    100% {
      transform: translate(var(--dx), var(--dy)) rotate(720deg) scale(var(--s));
      opacity: 0.4;
    }
  }
  @keyframes volta {
    0% {
      transform: translate(var(--dx), var(--dy)) scale(var(--s));
      opacity: 0.3;
    }
    65% {
      transform: translateY(-24px) scale(1.06);
      opacity: 1;
    }
    85% {
      transform: scale(0.96, 1.04);
    }
    100% {
      transform: none;
    }
  }

  .fala {
    position: absolute;
    left: 50%;
    bottom: calc(100% + 6px);
    z-index: 3;
    width: max-content;
    max-width: 240px;
    padding: 8px 12px;
    border-radius: 12px;
    background: #fff;
    color: var(--tinta-claro, #16213f);
    font-size: 12.5px;
    line-height: 1.4;
    text-shadow: none;
    box-shadow: 0 10px 24px rgb(10 40 110 / 0.25);
    transform: translateX(-50%);
    animation: balao 260ms cubic-bezier(0.3, 1.5, 0.5, 1);
  }
  .fala::after {
    content: "";
    position: absolute;
    left: 50%;
    bottom: -6px;
    width: 12px;
    height: 12px;
    background: #fff;
    transform: translateX(-50%) rotate(45deg);
    border-radius: 2px;
  }
  @keyframes balao {
    from {
      transform: translateX(-50%) translateY(6px) scale(0.85);
      opacity: 0;
    }
  }

  .perfil-texto {
    display: grid;
    gap: 6px;
    justify-items: start;
  }
  .perfil-texto h1 {
    font-family: var(--fonte-titulo);
    font-size: 22px;
    font-weight: 600;
    line-height: 1.2;
    letter-spacing: -0.01em;
    text-wrap: balance;
  }
  .perfil-texto p {
    font-size: 13px;
    opacity: 0.92;
    max-width: 52ch;
    text-wrap: pretty;
  }
  .link {
    margin-top: 2px;
    padding: 0;
    border: 0;
    background: none;
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  /* Objetos da mesa */
  .objetos {
    width: min(660px, 100%);
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    row-gap: 22px;
  }
  .objeto {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 8px 4px;
    border: 0;
    border-radius: 12px;
    background: none;
    color: #fff;
    cursor: grab;
    touch-action: none;
    transition:
      transform 300ms cubic-bezier(0.4, 0, 0.6, 1),
      opacity 300ms ease;
  }
  .objeto img {
    filter: drop-shadow(0 10px 12px rgb(10 30 90 / 0.3));
    transition: transform 180ms cubic-bezier(0.3, 1.6, 0.5, 1);
    pointer-events: none;
  }
  .objeto span {
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 500;
    text-shadow: 0 1px 2px rgb(0 25 80 / 0.45);
  }
  .objeto:hover img {
    transform: translateY(-4px) scale(1.06);
  }
  .objeto:focus-visible span {
    background: var(--acao);
  }
  .objeto.pego {
    z-index: 5;
    cursor: grabbing;
    transition: none;
  }
  .objeto.pego img {
    transform: scale(1.12) rotate(-4deg);
    filter: drop-shadow(0 22px 22px rgb(10 30 90 / 0.4));
  }
  .objeto.engolido {
    opacity: 0;
  }

  .arrumar {
    position: absolute;
    left: 16px;
    bottom: 10px;
    border: 0;
    background: none;
    color: #fff;
    font-size: 11px;
    opacity: 0.8;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  /* Janela de uma área: de verdade, por cima da mesa. */
  .janela {
    position: absolute;
    z-index: 10;
    top: 16px;
    left: 50%;
    width: min(760px, calc(100% - 32px));
    max-height: calc(100% - 24px);
    margin-left: calc(min(760px, calc(100% - 32px)) / -2);
    display: flex;
    flex-direction: column;
    border-radius: 14px;
    background: var(--superficie);
    backdrop-filter: var(--desfoque);
    box-shadow:
      0 0 0 1px rgb(255 255 255 / 0.4),
      0 30px 70px rgb(8 30 90 / 0.35);
    color: var(--tinta);
    overflow: hidden;
  }
  .janela-topo {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 12px 12px 18px;
    border-bottom: 1px solid var(--linha);
  }
  .janela-topo > div {
    flex: 1;
  }
  .janela-topo h1 {
    font-family: var(--fonte-titulo);
    font-size: 17px;
    font-weight: 600;
    line-height: 1.2;
  }
  .fechar-janela {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--tinta-2);
    cursor: pointer;
  }
  .fechar-janela:hover {
    background: var(--linha);
    color: var(--tinta);
  }
  .janela-corpo {
    min-height: 0;
    overflow: auto;
    padding: 6px 22px 22px;
  }
  .em-breve {
    display: grid;
    justify-items: start;
    gap: 14px;
    padding: 18px 0 4px;
  }
  .em-breve p {
    max-width: 54ch;
  }
  .em-breve .lista {
    width: 100%;
    color: var(--tinta-2);
  }

  /* Painel da direita */
  .lateral {
    grid-area: lateral;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 14px 14px 0;
    overflow: auto;
  }
  .widget {
    cursor: grab;
    touch-action: none;
  }
  .widget.sumindo {
    opacity: 0.25;
  }
  .widget.fixado {
    position: absolute;
    z-index: 4;
    width: 250px;
  }
  .widget.fantasma {
    position: fixed;
    z-index: 50;
    width: 262px;
    pointer-events: none;
    color: #fff;
    transform: rotate(-2deg) scale(1.03);
    filter: drop-shadow(0 24px 30px rgb(8 30 90 / 0.35));
  }
  .vidro {
    display: grid;
    gap: 10px;
    padding: 12px 14px;
    border: 1px solid rgb(255 255 255 / 0.38);
    border-radius: 14px;
    background: rgb(255 255 255 / 0.2);
    backdrop-filter: var(--desfoque);
    box-shadow: 0 8px 24px rgb(10 40 110 / 0.12);
  }
  :global(:root[data-tema="noite"]) .vidro {
    background: rgb(255 255 255 / 0.07);
    border-color: rgb(255 255 255 / 0.12);
  }
  :global(:root.jogo-aberto) .vidro {
    background: rgb(70 120 210 / 0.95);
  }
  :global(:root.jogo-aberto[data-tema="noite"]) .vidro {
    background: rgb(20 30 70 / 0.97);
  }
  .linha-tema {
    grid-template-columns: 1fr auto;
    align-items: center;
  }
  .temas {
    display: flex;
    gap: 6px;
  }
  .bolinha {
    width: 18px;
    height: 18px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid rgb(255 255 255 / 0.9);
    cursor: pointer;
  }
  .bolinha[aria-checked="true"] {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }
  .bolinha[data-cor="auto"] {
    background: linear-gradient(180deg, #1d58c0, #9dc4f0);
  }
  .bolinha[data-cor="por-do-sol"] {
    background: linear-gradient(180deg, #6a6fc0, #f4b98f);
  }
  .bolinha[data-cor="lavanda"] {
    background: linear-gradient(180deg, #7f8ad9, #d3d7f6);
  }
  .bolinha[data-cor="noite"] {
    background: #0e1a42;
  }
  .par {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .pequeno {
    gap: 2px;
  }
  .enorme {
    font-size: 26px;
    font-weight: 600;
    line-height: 1.15;
  }
  .legenda {
    font-size: 11.5px;
    opacity: 0.85;
  }
  .cabeca-widget {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .mini-botao {
    padding: 2px 8px;
    border: 1px solid rgb(255 255 255 / 0.5);
    border-radius: 6px;
    background: rgb(255 255 255 / 0.15);
    color: #fff;
    font-size: 11px;
    cursor: pointer;
  }
  .mini-botao:hover {
    background: rgb(255 255 255 / 0.3);
  }
  .cartao-branco {
    display: grid;
    gap: 10px;
    padding: 10px;
    border-radius: 10px;
    background: rgb(255 255 255 / 0.95);
    color: #16213f;
  }
  :global(:root[data-tema="noite"]) .cartao-branco {
    background: rgb(255 255 255 / 0.1);
    color: #fff;
  }
  .linha-jogo {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .linha-jogo > div {
    display: grid;
  }
  .capinha {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 8px;
    background: #dfe8fb;
    color: #3461d1;
    font-size: 12px;
    font-weight: 700;
    overflow: hidden;
  }
  .capinha img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cinza {
    font-size: 12px;
    opacity: 0.7;
  }
  .linha-ganho {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 8px;
    border-top: 1px solid rgb(22 33 63 / 0.08);
  }
  .ganho {
    color: #1d936e;
    font-size: 12.5px;
    font-weight: 600;
  }
  :global(:root[data-tema="noite"]) .ganho {
    color: #4cc59a;
  }
  .destaque-pc {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }
  .destaque-pc img {
    align-self: center;
    margin-right: 2px;
  }
  .ficha {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 5px;
    font-size: 12px;
  }
  .ficha li {
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }
  .ficha li span:first-child {
    opacity: 0.8;
  }
  .ficha li span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .trilha {
    height: 4px;
    border-radius: 2px;
    background: rgb(255 255 255 / 0.25);
    overflow: hidden;
  }
  .trilha div {
    height: 100%;
    background: #fff;
    border-radius: 2px;
    transition: width 600ms ease;
  }

  /* Dock com lupa: o ícone sob o ponteiro cresce, os vizinhos crescem menos. */
  .dock {
    grid-area: dock;
    justify-self: center;
    align-self: end;
    margin-bottom: 12px;
    display: flex;
    align-items: flex-end;
    gap: 6px;
    height: 62px;
    padding: 0 8px 7px;
    border-radius: 18px;
    background: rgb(255 255 255 / 0.22);
    border: 1px solid rgb(255 255 255 / 0.4);
    backdrop-filter: var(--desfoque);
    box-shadow: 0 12px 30px rgb(10 40 110 / 0.2);
  }
  :global(:root[data-tema="noite"]) .dock {
    background: rgb(255 255 255 / 0.08);
    border-color: rgb(255 255 255 / 0.14);
  }
  .icone {
    position: relative;
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    padding: 0;
    border: 0;
    border-radius: 12px;
    background: linear-gradient(180deg, rgb(255 255 255 / 0.85), rgb(225 236 252 / 0.7));
    box-shadow:
      0 1px 0 rgb(255 255 255 / 0.9) inset,
      0 4px 10px rgb(10 40 110 / 0.18);
    cursor: pointer;
    transform-origin: bottom center;
    transition:
      transform 160ms cubic-bezier(0.3, 1.4, 0.5, 1),
      margin 160ms ease;
  }
  :global(:root[data-tema="noite"]) .icone {
    background: linear-gradient(180deg, rgb(70 90 160 / 0.9), rgb(35 50 105 / 0.9));
    box-shadow: 0 4px 10px rgb(0 0 0 / 0.3);
  }
  .icone img {
    width: 32px;
    height: 32px;
    pointer-events: none;
  }
  .icone:hover {
    transform: scale(1.4) translateY(-4px);
    margin: 0 10px;
  }
  .icone:hover + .icone,
  .icone:has(+ .icone:hover) {
    transform: scale(1.16) translateY(-2px);
    margin: 0 4px;
  }
  .icone.ativa::after {
    content: "";
    position: absolute;
    bottom: -6px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: #fff;
  }
  .dica {
    position: absolute;
    bottom: calc(100% + 10px);
    padding: 3px 8px;
    border-radius: 6px;
    background: rgb(20 30 60 / 0.85);
    color: #fff;
    font-size: 10px;
    white-space: nowrap;
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }
  .icone:hover .dica {
    opacity: 1;
  }

  @media (max-width: 1100px) {
    .casca {
      grid-template-columns: 56px minmax(0, 1fr);
      grid-template-rows: 30px minmax(0, 1fr) auto 84px;
      grid-template-areas:
        "menu menu"
        "trilho mesa"
        "trilho lateral"
        "dock dock";
    }
    .lateral {
      flex-direction: row;
      flex-wrap: wrap;
      padding: 0 14px 8px 0;
    }
    .lateral > :global(*) {
      flex: 1 1 220px;
    }
    .objetos {
      grid-template-columns: repeat(4, 1fr);
    }
  }
</style>
