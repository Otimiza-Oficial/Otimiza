<script lang="ts">
  // A área Mira (trazida da 3.3, `src/v33/app.ts`, `telaMira`): a mira numa janela do Otimiza por cima do jogo (sem
  // tocar nele), os comandos para a mira do próprio CS2, a sensibilidade em cm por volta, o treino de tracking e, para
  // o FiveM, o campo de visão e o contador de FPS do jogo. Nada aqui mira por ninguém.
  import { onDestroy, onMount } from "svelte";
  import { ferramentas, type ContadorFivem, type FovFivem } from "../lib/ferramentas";
  import { cmPorVolta, CORES, desenharMira, ESTILOS, gravarMira, lerMira, sensValorant, treinar, type Mira } from "../lib/mira";

  const CONTROLES: [keyof Pick<Mira, "tamanho" | "espessura" | "abertura" | "opacidade">, string, number, number, number][] = [
    ["tamanho", "Tamanho", 2, 30, 1],
    ["espessura", "Espessura", 1, 6, 1],
    ["abertura", "Abertura do centro", 0, 14, 1],
    ["opacidade", "Opacidade", 0.2, 1, 0.05],
  ];

  let mira = $state<Mira>(lerMira());
  let ligada = $state(false);
  let recado = $state<{ texto: string; tom: "ok" | "atencao"; confirmar?: boolean } | null>(null);
  let enviar: ReturnType<typeof setTimeout> | undefined;

  let dpi = $state(800);
  let sens = $state(1.2);
  const cm = $derived(cmPorVolta(dpi, sens));
  const val = $derived(sensValorant(sens));

  let canvas = $state<HTMLCanvasElement | null>(null);
  let treinando = $state(false);
  let resultado = $state<string | null>(null);
  let recorde = $state(lerRecorde());

  let fov = $state<FovFivem | null>(null);
  let fovPosicao = $state(5);
  let fovErro = $state<string | null>(null);
  let contador = $state<ContadorFivem | null>(null);

  function lerRecorde() {
    try {
      return Number(localStorage.getItem("otimiza-recorde-tracking") ?? 0);
    } catch {
      return 0;
    }
  }

  onMount(async () => {
    ligada = (await ferramentas.miraEstado().catch(() => null))?.ligada ?? false;
    fov = await ferramentas.fovFivem().catch(() => null);
    fovPosicao = fov?.pedido ?? fov?.atual ?? 5;
    contador = await ferramentas.contadorFivem().catch(() => null);
  });
  onDestroy(() => clearTimeout(enviar));

  // Cada mudança vai para a prévia na hora, fica guardada e, com 120 ms de calma, para a janela no jogo.
  $effect(() => {
    const m = $state.snapshot(mira) as Mira;
    gravarMira(m);
    clearTimeout(enviar);
    enviar = setTimeout(() => void ferramentas.miraAtualizar(m).catch(() => {}), 120);
  });

  async function ligar(confirmado: boolean) {
    try {
      await ferramentas.miraLigar($state.snapshot(mira) as Mira, confirmado);
      ligada = true;
      recado = { texto: "Mira ligada. Ela some quando o jogo sai da frente.", tom: "ok" };
    } catch (e) {
      const t = String(e);
      recado = t.startsWith("CONFIRMAR:") ? { texto: t.slice(10), tom: "atencao", confirmar: true } : { texto: t, tom: "atencao" };
    }
  }

  async function alternar() {
    if (ligada) {
      await ferramentas.miraDesligar().catch(() => {});
      ligada = false;
      recado = null;
    } else {
      await ligar(false);
    }
  }

  async function copiarCs2() {
    try {
      await navigator.clipboard.writeText(await ferramentas.miraCs2($state.snapshot(mira) as Mira));
      recado = { texto: "Copiado. No CS2, abra o console e cole: a mira fica no próprio jogo, sem janela por cima.", tom: "ok" };
    } catch (e) {
      recado = { texto: String(e), tom: "atencao" };
    }
  }

  function comecarTreino() {
    if (!canvas) return;
    treinando = true;
    treinar(canvas, 15, (pct) => {
      treinando = false;
      if (pct > recorde) {
        recorde = pct;
        try {
          localStorage.setItem("otimiza-recorde-tracking", String(pct));
        } catch {
          // Sem armazenamento: o recorde vale nesta sessão.
        }
      }
      resultado = `No alvo ${pct}% do tempo. Treine com a sensibilidade que você usa no jogo.`;
    });
  }

  async function gravarFov(f: () => Promise<FovFivem>) {
    fovErro = null;
    try {
      fov = await f();
      fovPosicao = fov.pedido ?? fov.atual ?? fovPosicao;
    } catch (e) {
      fovErro = String(e);
    }
  }

  async function alternarContador() {
    if (!contador) return;
    try {
      contador = await ferramentas.contadorFivemDefinir(!contador.ligado);
    } catch (e) {
      fovErro = String(e);
    }
  }

  const n = (v: number | null, casas = 0) =>
    v == null ? "—" : v.toLocaleString("pt-BR", { maximumFractionDigits: casas, minimumFractionDigits: casas });
  const pct = (v: number, min: number, max: number) => `${((v - min) / (max - min)) * 100}%`;

  /** O fundo da prévia: quadros de FiveM de verdade. A mira precisa aparecer no claro (rua de dia), no meio do mato e
   *  no escuro (a mesma rua à noite). */
  const FUNDOS = [
    ["rua", "Rua"],
    ["morro", "Morro"],
    ["noite", "Noite"],
  ] as const;
  let fundo = $state<(typeof FUNDOS)[number][0]>("rua");
</script>

<div class="area-mira">
  <section class="secao estudio">
    <!-- A mira em cima de um jogo: troque o fundo para ver se ela aparece no claro e no escuro. -->
    <div class="palco" data-fundo={fundo}>
      <img class="cena" src={fundo === "morro" ? "/previas/fivem-morro.jpg" : "/previas/fivem-rua.jpg"} alt="" draggable="false" />
      <div class="sombra-da-cena" aria-hidden="true"></div>
      <svg class="mira" viewBox="-60 -60 120 120" aria-label="Prévia da mira">{@html desenharMira(mira, 1.6)}</svg>
      <div class="fundos" role="radiogroup" aria-label="Fundo da prévia">
        {#each FUNDOS as [id, nome] (id)}
          <button role="radio" aria-checked={fundo === id} onclick={() => (fundo = id)}>{nome}</button>
        {/each}
      </div>
      <span class="estado" class:vivo={ligada}><span class="luz"></span>{ligada ? "No jogo agora" : "Só na prévia"}</span>
    </div>

    <div class="painel">
      <div class="grupo">
        <span class="rotulo">Estilo</span>
        <div class="estilos">
          {#each ESTILOS as [id, nome] (id)}
            <button aria-pressed={mira.estilo === id} onclick={() => (mira.estilo = id)}>
              <svg viewBox="-14 -14 28 28" aria-hidden="true">{@html desenharMira({ ...mira, estilo: id, tamanho: 6, espessura: 1.6, abertura: 2.5, contorno: false, opacidade: 1, cor: "currentColor" })}</svg>
              <span>{nome}</span>
            </button>
          {/each}
        </div>
      </div>

      <div class="grupo">
        <span class="rotulo">Cor</span>
        <div class="cores">
          {#each CORES as c (c)}
            <button aria-pressed={mira.cor === c} style:--cor={c} aria-label={`Cor ${c}`} onclick={() => (mira.cor = c)}></button>
          {/each}
          <label class="livre" class:escolhida={!CORES.includes(mira.cor)} style:--cor={mira.cor} title="Outra cor">
            <input type="color" bind:value={mira.cor} aria-label="Outra cor" />
          </label>
        </div>
      </div>

      <div class="medidas">
        {#each CONTROLES as [k, nome, min, max, passo] (k)}
          <label class="medida">
            <span class="cabeca"><span>{nome}</span><b class="numero">{k === "opacidade" ? `${Math.round(mira[k] * 100)}%` : mira[k]}</b></span>
            <input class="deslizante" type="range" {min} {max} step={passo} bind:value={mira[k]} style:--pct={pct(mira[k], min, max)} />
          </label>
        {/each}
      </div>

      <div class="opcoes">
        <div class="linha-opcao">
          <span>Contorno preto</span>
          <button class="interruptor" role="switch" aria-checked={mira.contorno} aria-label="Contorno preto" onclick={() => (mira.contorno = !mira.contorno)}></button>
        </div>
        <div class="linha-opcao">
          <span>Some ao mirar com o botão direito</span>
          <button class="interruptor" role="switch" aria-checked={!!mira.esconder_ao_mirar} aria-label="Esconder ao mirar" onclick={() => (mira.esconder_ao_mirar = !mira.esconder_ao_mirar)}></button>
        </div>
      </div>

      <div class="acoes">
        <button class="btn" class:primario={!ligada} onclick={alternar}>{ligada ? "Tirar do jogo" : "Mostrar no jogo"}</button>
        <button class="btn" onclick={copiarCs2}>Copiar para o CS2</button>
      </div>
      {#if recado}
        <p class="recado" data-tom={recado.tom}>
          {recado.texto}
          {#if recado.confirmar}<button class="btn quieto" onclick={() => ligar(true)}>Ligar mesmo assim</button>{/if}
        </p>
      {/if}
    </div>
  </section>
  <p class="apoio pequeno nota">
    A mira fica numa janela do Otimiza por cima do jogo, sem tocar nele. Aparece com o jogo em janela ou sem bordas; em tela
    cheia exclusiva o Windows não deixa nada por cima. "Copiar para o CS2" leva a mesma mira para o console do jogo.
  </p>

  <section class="dupla">
    <div class="cartao">
      <h2>Sensibilidade</h2>
      <div class="volta">
        <span class="numero grande">{cm ? n(cm, 1) : "—"}</span>
        <span class="apoio">cm por volta completa</span>
      </div>
      <div class="entradas">
        <label>DPI do mouse <input class="campo" type="number" min="100" max="32000" bind:value={dpi} /></label>
        <label>Sensibilidade no CS2 <input class="campo" type="number" min="0.05" step="0.05" bind:value={sens} /></label>
      </div>
      <p class="apoio pequeno">
        No Valorant, a mesma volta é <b class="numero">{n(val, 3)}</b>. CS2, Apex e Valorant giram 0,022° por contagem do mouse:
        mesma volta em centímetros, mesma memória muscular.
      </p>
    </div>
    <div class="cartao">
      <h2>Treino de tracking {#if recorde}<span class="apoio pequeno">recorde {recorde}%</span>{/if}</h2>
      <div class="treino">
        <canvas bind:this={canvas}></canvas>
        {#if !treinando}
          <div class="sobre"><button class="btn primario" onclick={comecarTreino}>{resultado ? "De novo" : "Começar · 15 s"}</button></div>
        {/if}
      </div>
      <p class="apoio pequeno">{resultado ?? "Mantenha o cursor sobre o alvo. Mede quanto tempo você ficou nele."}</p>
    </div>
  </section>

  <section class="cartao">
    <h2>FiveM</h2>
    {#if contador}
      <div class="linha-opcao">
        <div>
          <span>Contador de FPS do jogo (cl_drawfps)</span>
          <p class="apoio pequeno">
            {contador.pendente ? "Vale quando o FiveM fechar: com ele aberto, o jogo regrava o arquivo." : "O número de FPS do próprio FiveM, no canto da tela."}
          </p>
        </div>
        <button class="interruptor" role="switch" aria-checked={contador.ligado} aria-label="Contador de FPS do FiveM" onclick={alternarContador}></button>
      </div>
    {/if}
    {#if fov}
      <label class="medida">
        <span class="cabeca"><span>Campo de visão em primeira pessoa (posição no menu)</span><b class="numero">{fovPosicao} de {fov.maximo}</b></span>
        <input class="deslizante" type="range" min="0" max={fov.maximo} step="1" bind:value={fovPosicao} style:--pct={pct(fovPosicao, 0, fov.maximo || 1)} />
      </label>
      <p class="apoio pequeno">
        {#if fov.pendente}
          Grava {fov.pedido === null ? "a volta do de antes" : `a posição ${fov.pedido}`} quando o FiveM fechar.
        {:else if fov.atual === null}
          O FiveM ainda não gravou essa opção neste PC: abra o jogo uma vez.
        {:else}
          No arquivo do FiveM agora: posição {fov.atual} de {fov.maximo}. A mesma opção do menu do jogo; servidor que fixa o FOV vence.
        {/if}
      </p>
      <div class="acoes">
        <button class="btn primario" onclick={() => gravarFov(() => ferramentas.fovFivemDefinir(fovPosicao))}>Gravar no FiveM</button>
        {#if fov.alterado}<button class="btn" onclick={() => gravarFov(ferramentas.fovFivemDevolver)}>Devolver o de antes</button>{/if}
      </div>
    {:else}
      <p class="apoio">Não consegui ler a configuração do FiveM.</p>
    {/if}
    {#if fovErro}<p class="recado" data-tom="atencao">{fovErro}</p>{/if}
  </section>
</div>

<style>
  .area-mira {
    display: grid;
    gap: 16px;
    padding: 8px 0 4px;
  }
  .estudio {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(0, 1fr);
    gap: 18px;
    align-items: start;
  }
  .palco {
    position: relative;
    display: grid;
    place-items: center;
    aspect-ratio: 16 / 10;
    border-radius: 18px;
    overflow: hidden;
    background: #111;
    box-shadow: 0 18px 40px -12px rgb(0 0 0 / 0.5), 0 0 0 1px rgb(255 255 255 / 0.06);
  }
  .cena {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    transform: scale(1.04);
    transition: filter var(--mov-entrar) ease;
  }
  /* A noite é a mesma rua, escurecida e azulada: a mira precisa continuar aparecendo. */
  .palco[data-fundo="noite"] .cena {
    filter: brightness(0.28) saturate(0.7) hue-rotate(195deg) contrast(1.15);
  }
  /* Vinheta e um degradê embaixo, para os rótulos lerem bem sobre o jogo. */
  .sombra-da-cena {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(120% 90% at 50% 45%, transparent 55%, rgb(0 0 0 / 0.45)),
      linear-gradient(180deg, rgb(0 0 0 / 0.25), transparent 25%, transparent 75%, rgb(0 0 0 / 0.35));
    pointer-events: none;
  }
  .mira {
    position: relative;
    z-index: 1;
    width: 40%;
    max-width: 200px;
    aspect-ratio: 1;
  }
  .fundos {
    position: absolute;
    top: 12px;
    left: 12px;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.4);
    backdrop-filter: blur(6px);
  }
  .fundos button {
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: #fff;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .fundos button[aria-checked="true"] {
    background: #fff;
    color: #1d1d1f;
  }
  .estado {
    position: absolute;
    left: 12px;
    bottom: 12px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 4px 10px;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.45);
    color: #fff;
    font-size: 12px;
    backdrop-filter: blur(6px);
  }
  .luz {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: rgb(255 255 255 / 0.5);
  }
  .estado.vivo .luz {
    background: #34c759;
    box-shadow: 0 0 0 3px rgb(52 199 89 / 0.3);
  }
  .painel {
    display: grid;
    gap: 14px;
  }
  .grupo {
    display: grid;
    gap: 6px;
  }
  .rotulo {
    font-size: 12px;
    color: var(--tinta-2);
  }
  .estilos {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 6px;
  }
  .estilos button {
    display: grid;
    justify-items: center;
    gap: 3px;
    padding: 8px 2px 6px;
    border-radius: 12px;
    border: 1px solid var(--linha);
    background: var(--superficie-2);
    color: var(--tinta);
    font: inherit;
    font-size: 10.5px;
    cursor: pointer;
  }
  .estilos svg {
    width: 26px;
    height: 26px;
  }
  .estilos button[aria-pressed="true"] {
    background: var(--tinta);
    color: var(--contra, #fff);
    border-color: transparent;
  }
  .cores {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .cores button,
  .livre {
    position: relative;
    width: 28px;
    height: 28px;
    padding: 0;
    border-radius: 50%;
    border: 0;
    background: var(--cor);
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 0.15);
    cursor: pointer;
    transition: transform 140ms cubic-bezier(0.3, 1.5, 0.5, 1);
  }
  .cores button:hover,
  .livre:hover {
    transform: scale(1.1);
  }
  .cores button[aria-pressed="true"],
  .livre.escolhida {
    box-shadow:
      inset 0 0 0 1px rgb(0 0 0 / 0.15),
      0 0 0 2px var(--superficie, #fff),
      0 0 0 4px var(--tinta);
  }
  /* A cor livre: um disco com o espectro, e o seletor do Windows por baixo. */
  .livre {
    background: conic-gradient(#ff4a4a, #ffe14a, #7cff4f, #00e5ff, #4a6bff, #ff4fd8, #ff4a4a);
  }
  .livre.escolhida {
    background: var(--cor);
  }
  .livre input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .medidas {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 18px;
  }
  .medida {
    display: grid;
    gap: 3px;
    font-size: 13px;
  }
  .cabeca {
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }
  .cabeca b {
    font-weight: 600;
    white-space: nowrap;
  }
  .opcoes {
    display: grid;
    gap: 8px;
  }
  .linha-opcao {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    font-size: 13px;
  }
  .linha-opcao p {
    margin: 2px 0 0;
  }
  .acoes {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .recado {
    margin: 0;
    font-size: 13px;
  }
  .recado[data-tom="ok"] {
    color: var(--ok);
  }
  .recado[data-tom="atencao"] {
    color: var(--atencao);
  }
  .nota {
    margin: -6px 0 0;
  }
  .dupla {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .cartao {
    display: grid;
    gap: 10px;
    align-content: start;
    padding: 16px 18px;
    border-radius: 18px;
    background: var(--superficie);
    border: 1px solid var(--linha);
  }
  .cartao h2 {
    margin: 0;
  }
  .volta {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .numero.grande {
    font-size: 40px;
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1;
  }
  .entradas {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .entradas label {
    display: grid;
    gap: 4px;
    font-size: 12px;
    color: var(--tinta-2);
  }
  .entradas .campo {
    width: 100%;
    box-sizing: border-box;
  }
  .pequeno {
    font-size: 12px;
  }
  .cartao p {
    margin: 0;
  }
  .treino {
    position: relative;
    height: 160px;
    border-radius: 14px;
    background: #11161f;
    overflow: hidden;
  }
  .treino canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
  .sobre {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
  @media (max-width: 860px) {
    .estudio,
    .dupla {
      grid-template-columns: 1fr;
    }
  }
</style>
