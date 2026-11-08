<script lang="ts">
  // A casca da interface nova: o Oti no centro, conversando. Ele diz o que leu do PC e oferece respostas; a barra de
  // baixo aceita perguntas; os ícones abrem as áreas. Com uma área aberta ele vai para a esquerda e narra; com um jogo
  // aberto ele bota o headset, vai para a barra de cima e a interface para de animar. Regras em `ui/DESIGN.md`.
  import { onMount } from "svelte";
  import { cubicOut } from "svelte/easing";
  import Historico from "./componentes/Historico.svelte";
  import Latencia from "./componentes/Latencia.svelte";
  import Rede from "./componentes/Rede.svelte";
  import Tela from "./componentes/Tela.svelte";
  import Oti, { type Gesto, type Humor } from "./componentes/Oti.svelte";
  import { areas, type IdDaArea } from "./lib/areas";
  import { avisos as lerAvisos } from "./lib/clima";
  import { abertura, cutucada, entender, jogoAbriu, jogoFechou, narracao, ociosa, type Acao, type Fala } from "./lib/conversa";
  import { noTauri, ponte, type Hardware, type JogoAberto, type MetricasAoVivo, type VersaoDoApp } from "./lib/ponte";

  type Tema = "escuro" | "claro";
  const CHAVE_DO_TEMA = "otimiza.tema";

  let versao = $state<VersaoDoApp | null>(null);
  let hardware = $state<Hardware | null>(null);
  let metricas = $state<MetricasAoVivo | null>(null);
  let jogo = $state<JogoAberto | null>(null);
  let tema = $state<Tema>(lerTema());
  let aberta = $state<IdDaArea>("inicio");

  let fala = $state<Fala>({ texto: "", humor: "atento", acoes: [] });
  let digitado = $state("");
  let digitando = $state(false);
  let reacao = $state<{ humor?: Humor; gesto?: Gesto } | null>(null);
  let pergunta = $state("");
  let pulo = $state(false);
  let primeiraFala = false;

  /** O jogo "vale" na interface (classe `jogo-aberto`) só depois da cena do headset. */
  let jogoNaTela = $state(false);
  let noCanto = $state(false);

  const lista = $derived(lerAvisos(hardware, metricas, jogo !== null));
  const area = $derived(areas.find((a) => a.id === aberta) ?? areas[0]);
  const modo = $derived<"centro" | "lado" | "canto">(noCanto ? "canto" : aberta === "inicio" ? "centro" : "lado");
  const humor = $derived<Humor>(
    reacao?.humor ?? (jogo && !noCanto ? "jogando" : digitando ? "falando" : modo === "lado" ? "atento" : fala.humor)
  );
  const gesto = $derived<Gesto>(reacao?.gesto ?? (digitando ? "nenhum" : (fala.gesto ?? "nenhum")));
  const memoriaPct = $derived.by(() => {
    const total = metricas?.ram.total_gb ?? null;
    const usada = metricas?.ram.used_gb ?? null;
    return total != null && usada != null && total > 0 ? usada / total : null;
  });
  const telaPrincipal = $derived(hardware?.telas.find((t) => t.principal) ?? hardware?.telas[0] ?? null);

  function lerTema(): Tema {
    try {
      return localStorage.getItem(CHAVE_DO_TEMA) === "claro" ? "claro" : "escuro";
    } catch {
      return "escuro";
    }
  }

  function trocarTema() {
    tema = tema === "escuro" ? "claro" : "escuro";
    try {
      localStorage.setItem(CHAVE_DO_TEMA, tema);
    } catch {
      // Só não lembra na próxima abertura.
    }
  }

  $effect(() => {
    document.documentElement.classList.toggle("jogo-aberto", jogoNaTela);
    document.documentElement.dataset.tema = tema === "claro" ? "dia" : "noite";
  });

  // ---------------------------------------------------------------------------------------------------------------
  // Falar: a frase aparece letra a letra, com a boca mexendo. Com jogo aberto ou movimento reduzido, aparece inteira.

  let maquina: ReturnType<typeof setInterval> | undefined;
  function dizer(f: Fala) {
    fala = f;
    clearInterval(maquina);
    const instantaneo = jogoNaTela || window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (instantaneo) {
      digitado = f.texto;
      digitando = false;
      return;
    }
    digitado = "";
    digitando = true;
    let i = 0;
    maquina = setInterval(() => {
      i += 1;
      digitado = f.texto.slice(0, i);
      if (i >= f.texto.length) {
        clearInterval(maquina);
        digitando = false;
      }
    }, 24);
  }

  let tempoDaReacao: ReturnType<typeof setTimeout> | undefined;
  function reagir(r: { humor?: Humor; gesto?: Gesto }, ms: number) {
    clearTimeout(tempoDaReacao);
    reacao = r;
    tempoDaReacao = setTimeout(() => (reacao = null), ms);
  }

  function executar(a: Acao) {
    if (a.tipo === "abrir") abrir(a.area);
    else dizer(a.fala());
  }

  function abrir(id: IdDaArea) {
    if (id === "inicio") {
      fechar();
      return;
    }
    aberta = id;
    const a = areas.find((x) => x.id === id);
    dizer({ texto: narracao(id, a?.onda ?? 2, a?.nome ?? ""), humor: "atento", acoes: [] });
  }

  function fechar() {
    aberta = "inicio";
    dizer(ociosa());
  }

  function perguntar(e: SubmitEvent) {
    e.preventDefault();
    const texto = pergunta.trim();
    if (!texto) return;
    pergunta = "";
    const r = entender(texto);
    // Um instante "pensando" antes da resposta: ler a pergunta tem que parecer ler.
    reagir({ humor: "pensando" }, 650);
    setTimeout(() => {
      dizer(r.fala);
      if (r.area) setTimeout(() => abrir(r.area!), 700);
    }, 650);
  }

  function cutucar() {
    if (jogoNaTela || modo !== "centro") return;
    reagir({ humor: "feliz", gesto: "comemorar" }, 1300);
    pulo = true;
    setTimeout(() => (pulo = false), 600);
    dizer(cutucada());
  }

  // A primeira fala espera a ficha do PC (aviso de Hz) E a primeira leitura de memória (aviso de memória cheia):
  // falar antes diria "está tudo certo" sobre o que ainda não foi lido.
  $effect(() => {
    if (primeiraFala || !hardware || (!metricas && !jogo)) return;
    primeiraFala = true;
    if (!jogo) {
      const f = abertura(lista);
      dizer(f);
      if (f.gesto === "acenar") reagir({ humor: "feliz", gesto: "acenar" }, 2200);
    }
  });

  // O jogo abriu ou fechou. Na primeira leitura (o Otimiza abriu com o jogo rodando) não há cena.
  let jogoAnterior: string | null | undefined = undefined;
  $effect(() => {
    const nome = jogo?.nome ?? null;
    if (jogoAnterior === undefined) {
      jogoAnterior = nome;
      if (nome) {
        noCanto = true;
        jogoNaTela = true;
      }
      return;
    }
    if (nome === jogoAnterior) return;
    const antes = jogoAnterior;
    jogoAnterior = nome;
    if (nome && !antes) {
      aberta = "inicio";
      dizer(jogoAbriu(nome));
      // Headset, um "bom jogo", e só então vai para o canto e a interface congela.
      setTimeout(() => (noCanto = true), 2200);
      setTimeout(() => (jogoNaTela = true), 3000);
    } else if (!nome && antes) {
      jogoNaTela = false;
      noCanto = false;
      setTimeout(() => dizer(jogoFechou(antes)), 500);
    }
  });

  // ---------------------------------------------------------------------------------------------------------------

  function pct(v: number | null | undefined) {
    return v == null ? "—" : `${Math.round(v)}%`;
  }

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

  async function alternarJogoDeDemonstracao() {
    ponte.simularJogo(jogo ? null : "FiveM");
    jogo = await ponte.jogoAberto();
  }

  function entrarPainel(_node: HTMLElement) {
    return {
      duration: 520,
      easing: cubicOut,
      css: (t: number, u: number) => `transform: translateX(${u * 60}px) scale(${0.97 + 0.03 * t}); opacity: ${t}`,
    };
  }

  onMount(() => {
    let vivo = true;
    let espera: ReturnType<typeof setTimeout> | undefined;

    void (async () => {
      versao = await ponte.versao().catch(() => null);
      hardware = await ponte.hardwareUltimo().catch(() => null);
      ponte.hardware().then((h) => (hardware = h)).catch(() => {});
      await ponte.iniciarMonitor().catch(() => {});
    })();

    // Um laço só. Com o jogo aberto ele só confere se o jogo continua lá, devagar.
    const laco = async () => {
      if (!vivo) return;
      jogo = await ponte.jogoAberto().catch(() => null);
      if (!jogo && !document.hidden) metricas = await ponte.metricas().catch(() => metricas);
      espera = setTimeout(laco, jogo ? 5000 : document.hidden ? 4000 : 2000);
    };
    void laco();

    return () => {
      vivo = false;
      clearTimeout(espera);
      clearInterval(maquina);
    };
  });
</script>

<div class="cena" data-modo={modo}>
  <div class="aurora" aria-hidden="true"></div>

  <header class="topo" data-tauri-drag-region>
    <div class="marca" data-tauri-drag-region>
      <span class="mini" class:aqui={modo === "canto"}>
        <Oti humor={modo === "canto" ? "jogando" : "atento"} tamanho={24} segueCursor={false} />
      </span>
      <strong>Otimiza</strong>
      {#if versao}<span class="fraco">{versao.exibida}</span>{/if}
    </div>

    <div class="estado" data-tauri-drag-region>
      {#if !noTauri}
        <button class="texto" onclick={alternarJogoDeDemonstracao}>{jogo ? "Fechar jogo de teste" : "Simular jogo"}</button>
        <span class="demo">Demonstração</span>
      {/if}
      {#if jogo}
        <span class="ao-vivo"><span class="ponto" data-tom="ok"></span>{jogo.nome}</span>
      {:else}
        <span class="fraco">Processador <b class="numero">{pct(metricas?.cpu.overall)}</b></span>
        <span class="fraco">Memória <b class="numero">{memoriaPct == null ? "—" : pct(memoriaPct * 100)}</b></span>
        {#if telaPrincipal}<span class="fraco">Tela <b class="numero">{telaPrincipal.hz_atual} Hz</b></span>{/if}
      {/if}
      <button class="texto" onclick={trocarTema}>{tema === "escuro" ? "Tema claro" : "Tema escuro"}</button>
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

  <!-- O Oti e o que ele está dizendo. A posição muda com o modo, por transição. -->
  <div class="oti" class:pulo>
    <button class="oti-botao" aria-label="Oti, o mascote. Clique para uma dica." onclick={cutucar}>
      <Oti {humor} {gesto} tamanho={modo === "centro" ? 230 : 150} />
    </button>
  </div>

  <section class="conversa" aria-live="polite">
    <p class="fala">{digitado}<span class="cursor" class:some={!digitando}></span></p>
    {#if fala.detalhe && !digitando}
      <p class="detalhe">{fala.detalhe}</p>
    {/if}
    {#if fala.acoes.length > 0 && !digitando && modo === "centro"}
      <div class="respostas">
        {#each fala.acoes as a, i (a.rotulo)}
          <button
            class="resposta"
            class:principal={a.principal}
            style:animation-delay={`${i * 60}ms`}
            onclick={() => executar(a)}>{a.rotulo}</button
          >
        {/each}
      </div>
    {/if}
  </section>

  {#if modo === "centro"}
    <form class="perguntar" onsubmit={perguntar}>
      <input bind:value={pergunta} placeholder="Pergunte ao Oti… por exemplo: por que meu ping pula?" aria-label="Pergunte ao Oti" />
      <button aria-label="Perguntar" disabled={pergunta.trim() === ""}>
        <svg viewBox="0 0 24 24" width="18" height="18"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
      </button>
    </form>
  {/if}

  {#if modo === "canto" && jogo}
    <p class="pausa">{jogo.nome} aberto · interface pausada para não pesar no jogo</p>
  {/if}

  {#if modo === "lado"}
    <section class="painel" aria-labelledby="titulo-da-area" in:entrarPainel>
      <header class="painel-topo">
        <span class="icone"><svg viewBox="0 0 24 24" width="20" height="20"><path d={area.traco} /></svg></span>
        <div>
          <h1 id="titulo-da-area">{area.nome}</h1>
          {#if area.pergunta}<p class="apoio">{area.pergunta}</p>{/if}
        </div>
        <button class="fechar-painel" aria-label="Fechar" onclick={fechar}>
          <svg viewBox="0 0 12 12" width="12" height="12"><path d="M3 3l6 6M9 3l-6 6" /></svg>
        </button>
      </header>
      <div class="painel-corpo">
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
            <p>Esta área chega na Onda {area.onda}. O que vai ter:</p>
            <ul class="lista">
              {#each area.vai_ter as item (item)}
                <li>{item}</li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    </section>
  {/if}

  <nav class="areas" aria-label="Áreas">
    {#each areas as a (a.id)}
      <button class="area" class:ativa={aberta === a.id} onclick={() => abrir(a.id)} aria-current={aberta === a.id}>
        <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true"><path d={a.traco} /></svg>
        <span>{a.nome}</span>
      </button>
    {/each}
  </nav>
</div>

<style>
  .cena {
    --fundo: #0b0c1f;
    --texto: #eef0ff;
    --texto-2: #a7abd0;
    --vidro: rgb(255 255 255 / 0.06);
    --vidro-borda: rgb(255 255 255 / 0.1);
    --roxo: #8d8bff;
    --roxo-forte: #a5a3ff;
    position: relative;
    height: 100vh;
    overflow: hidden;
    background: var(--fundo);
    color: var(--texto);
  }
  :global(:root[data-tema="dia"]) .cena {
    --fundo: #f3f2fb;
    --texto: #1a1838;
    --texto-2: #5f5d84;
    --vidro: rgb(255 255 255 / 0.75);
    --vidro-borda: rgb(26 24 56 / 0.1);
    --roxo: #5b57e8;
    --roxo-forte: #4642d4;
  }

  /* A assinatura: um brilho atrás do Oti, que respira. É o único enfeite da tela. */
  .aurora {
    position: absolute;
    left: 50%;
    top: 31%;
    width: 900px;
    height: 640px;
    transform: translate(-50%, -50%);
    background:
      radial-gradient(closest-side, rgb(125 141 255 / 0.42), transparent 70%),
      radial-gradient(closest-side at 62% 64%, rgb(190 120 255 / 0.3), transparent 70%),
      radial-gradient(closest-side at 38% 70%, rgb(255 179 71 / 0.12), transparent 70%);
    animation: respirar 9s ease-in-out infinite alternate;
    transition:
      left 800ms cubic-bezier(0.2, 0.9, 0.2, 1),
      top 800ms cubic-bezier(0.2, 0.9, 0.2, 1),
      opacity 600ms ease;
    pointer-events: none;
  }
  :global(:root[data-tema="dia"]) .aurora {
    opacity: 0.55;
  }
  [data-modo="lado"] .aurora {
    left: 18%;
    top: 40%;
    opacity: 0.7;
  }
  [data-modo="canto"] .aurora {
    opacity: 0.2;
  }
  @keyframes respirar {
    from {
      transform: translate(-50%, -50%) scale(0.94);
    }
    to {
      transform: translate(-50%, -50%) scale(1.06);
    }
  }

  /* Barra de cima */
  .topo {
    position: relative;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 16px;
    height: 38px;
    padding-left: 14px;
    font-size: 12px;
  }
  .marca {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .marca strong {
    font-family: var(--fonte-titulo);
    font-weight: 600;
    font-size: 13px;
  }
  .mini {
    display: inline-flex;
  }
  .mini.aqui {
    animation: chegar 560ms cubic-bezier(0.3, 1.7, 0.5, 1);
  }
  @keyframes chegar {
    0% {
      transform: scale(0.3);
    }
    60% {
      transform: scale(1.8);
    }
    100% {
      transform: scale(1);
    }
  }
  .fraco {
    color: var(--texto-2);
  }
  .fraco b {
    color: var(--texto);
    font-weight: 500;
  }
  .estado {
    flex: 1;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 16px;
    height: 100%;
  }
  .texto {
    border: 0;
    background: none;
    color: var(--texto-2);
    font-size: 12px;
    cursor: pointer;
  }
  .texto:hover {
    color: var(--texto);
  }
  .demo {
    padding: 1px 7px;
    border-radius: 5px;
    background: rgb(255 179 71 / 0.16);
    color: #ffc163;
  }
  :global(:root[data-tema="dia"]) .demo {
    color: #9a5a14;
  }
  .ao-vivo {
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
    color: var(--texto-2);
    cursor: pointer;
  }
  .controles svg,
  .fechar-painel svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
  }
  .controles button:hover {
    background: var(--vidro);
    color: var(--texto);
  }
  .controles .fechar:hover {
    background: #c42b1c;
    color: #fff;
  }

  /* O Oti: no centro, à esquerda (área aberta) ou sumindo para a barra (jogo). */
  .oti {
    position: absolute;
    z-index: 3;
    left: 50%;
    top: 31%;
    transform: translate(-50%, -50%);
    transition:
      left 800ms cubic-bezier(0.2, 0.9, 0.2, 1),
      top 800ms cubic-bezier(0.2, 0.9, 0.2, 1),
      transform 800ms cubic-bezier(0.5, 0, 0.3, 1),
      opacity 500ms ease;
  }
  [data-modo="lado"] .oti {
    left: 18%;
    top: 38%;
  }
  [data-modo="canto"] .oti {
    left: 24px;
    top: 19px;
    transform: translate(-50%, -50%) scale(0.1);
    opacity: 0;
  }
  .oti.pulo {
    animation: pulo 600ms ease;
  }
  @keyframes pulo {
    0% {
      translate: 0 0;
      scale: 1;
    }
    25% {
      scale: 1.1 0.88;
    }
    55% {
      translate: 0 -26px;
      scale: 0.94 1.08;
    }
    100% {
      translate: 0 0;
      scale: 1;
    }
  }
  .oti-botao {
    display: block;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
  }

  /* O que ele diz */
  .conversa {
    position: absolute;
    z-index: 3;
    left: 50%;
    top: calc(31% + 132px);
    width: min(640px, calc(100% - 48px));
    transform: translateX(-50%);
    display: grid;
    justify-items: center;
    gap: 10px;
    text-align: center;
    transition:
      left 800ms cubic-bezier(0.2, 0.9, 0.2, 1),
      top 800ms cubic-bezier(0.2, 0.9, 0.2, 1),
      width 800ms ease,
      opacity 400ms ease;
  }
  [data-modo="lado"] .conversa {
    left: 18%;
    top: calc(38% + 92px);
    width: min(300px, 30%);
  }
  [data-modo="canto"] .conversa {
    opacity: 0;
  }
  .fala {
    min-height: 1.3em;
    font-family: var(--fonte-titulo);
    font-size: 26px;
    font-weight: 600;
    line-height: 1.25;
    letter-spacing: -0.015em;
    text-wrap: balance;
  }
  [data-modo="lado"] .fala {
    font-size: 16px;
    font-weight: 500;
    line-height: 1.45;
    color: var(--texto-2);
  }
  .cursor {
    display: inline-block;
    width: 2px;
    height: 0.95em;
    margin-left: 2px;
    vertical-align: -0.1em;
    background: var(--roxo);
    animation: piscar 0.9s steps(1) infinite;
  }
  .cursor.some {
    opacity: 0;
    animation: none;
  }
  @keyframes piscar {
    50% {
      opacity: 0;
    }
  }
  .detalhe {
    max-width: 52ch;
    color: var(--texto-2);
    font-size: 15px;
    line-height: 1.5;
    animation: surgir 300ms ease both;
  }
  .respostas {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 10px;
  }
  .resposta {
    min-height: 40px;
    padding: 0 18px;
    border: 1px solid var(--vidro-borda);
    border-radius: 999px;
    background: var(--vidro);
    color: var(--texto);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    animation: surgir 320ms cubic-bezier(0.2, 0.9, 0.3, 1.2) both;
    transition:
      background 140ms ease,
      border-color 140ms ease,
      transform 140ms ease;
  }
  .resposta:hover {
    border-color: var(--roxo);
    transform: translateY(-2px);
  }
  .resposta.principal {
    border-color: transparent;
    background: var(--roxo);
    color: #0b0c1f;
  }
  :global(:root[data-tema="dia"]) .resposta.principal {
    color: #fff;
  }
  .resposta.principal:hover {
    background: var(--roxo-forte);
  }
  @keyframes surgir {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }

  /* Pergunta */
  .perguntar {
    position: absolute;
    z-index: 3;
    left: 50%;
    bottom: 104px;
    width: min(560px, calc(100% - 48px));
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 6px 6px 18px;
    border: 1px solid var(--vidro-borda);
    border-radius: 999px;
    background: var(--vidro);
    backdrop-filter: blur(16px);
    transition: border-color 140ms ease;
  }
  .perguntar:focus-within {
    border-color: var(--roxo);
  }
  .perguntar input {
    flex: 1;
    min-width: 0;
    height: 36px;
    border: 0;
    outline: none;
    background: none;
    color: var(--texto);
    font-size: 14px;
    user-select: text;
  }
  .perguntar input::placeholder {
    color: var(--texto-2);
  }
  .perguntar button {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border: 0;
    border-radius: 50%;
    background: var(--roxo);
    color: #0b0c1f;
    cursor: pointer;
  }
  :global(:root[data-tema="dia"]) .perguntar button {
    color: #fff;
  }
  .perguntar button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .perguntar svg,
  .area svg,
  .icone svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .pausa {
    position: absolute;
    left: 50%;
    top: 46%;
    transform: translateX(-50%);
    color: var(--texto-2);
    font-size: 14px;
  }

  /* Painel de uma área: à direita, com o Oti narrando à esquerda. */
  .painel {
    position: absolute;
    z-index: 4;
    top: 50px;
    right: 20px;
    bottom: 96px;
    width: min(760px, 62%);
    display: flex;
    flex-direction: column;
    border-radius: 18px;
    background: var(--superficie);
    box-shadow:
      0 0 0 1px var(--vidro-borda),
      0 30px 80px rgb(0 0 0 / 0.35);
    color: var(--tinta);
    overflow: hidden;
  }
  .painel-topo {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 14px 14px 20px;
    border-bottom: 1px solid var(--linha);
  }
  .painel-topo > div {
    flex: 1;
  }
  .painel-topo h1 {
    font-family: var(--fonte-titulo);
    font-size: 18px;
    font-weight: 600;
    line-height: 1.2;
  }
  .icone {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: var(--acao-suave);
    color: var(--acao);
  }
  .fechar-painel {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 9px;
    background: transparent;
    color: var(--tinta-2);
    cursor: pointer;
  }
  .fechar-painel:hover {
    background: var(--linha);
    color: var(--tinta);
  }
  .painel-corpo {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 6px 22px 22px;
  }
  .em-breve {
    display: grid;
    gap: 12px;
    padding-top: 16px;
  }
  .em-breve .lista {
    color: var(--tinta-2);
  }

  /* Áreas: a navegação de sempre, embaixo. */
  .areas {
    position: absolute;
    z-index: 5;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    display: flex;
    gap: 2px;
    padding: 6px;
    border: 1px solid var(--vidro-borda);
    border-radius: 18px;
    background: var(--vidro);
    backdrop-filter: blur(16px);
  }
  .area {
    display: grid;
    justify-items: center;
    gap: 3px;
    width: 64px;
    padding: 7px 0 6px;
    border: 0;
    border-radius: 12px;
    background: none;
    color: var(--texto-2);
    cursor: pointer;
    transition:
      color 140ms ease,
      background 140ms ease,
      transform 160ms cubic-bezier(0.3, 1.5, 0.5, 1);
  }
  .area span {
    font-size: 10.5px;
    font-weight: 500;
  }
  .area:hover {
    color: var(--texto);
    transform: translateY(-3px);
  }
  .area.ativa {
    background: var(--roxo);
    color: #0b0c1f;
  }
  :global(:root[data-tema="dia"]) .area.ativa {
    color: #fff;
  }

  @media (max-width: 1000px) {
    .area {
      width: 52px;
    }
    .area span {
      display: none;
    }
    [data-modo="lado"] .oti,
    [data-modo="lado"] .conversa,
    [data-modo="lado"] .aurora {
      left: 14%;
    }
    .painel {
      width: 70%;
    }
  }
</style>
