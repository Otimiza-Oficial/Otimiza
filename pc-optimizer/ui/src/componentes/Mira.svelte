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
</script>

<div class="area-mira">
  <section class="secao duas">
    <div class="previa">
      <svg viewBox="-60 -60 120 120" width="160" height="160" aria-label="Prévia da mira">{@html desenharMira(mira, 1.6)}</svg>
    </div>
    <div class="controles">
      <span class="rotulo">Estilo</span>
      <div class="estilos">
        {#each ESTILOS as [id, nome] (id)}
          <button aria-pressed={mira.estilo === id} title={nome} aria-label={nome} onclick={() => (mira.estilo = id)}>
            <svg viewBox="-14 -14 28 28">{@html desenharMira({ ...mira, estilo: id, tamanho: 6, espessura: 1.6, abertura: 2.5, contorno: false, opacidade: 1, cor: "#fff" })}</svg>
          </button>
        {/each}
      </div>
      <span class="rotulo">Cor</span>
      <div class="cores">
        {#each CORES as c (c)}
          <button aria-pressed={mira.cor === c} style:background={c} aria-label={`Cor ${c}`} onclick={() => (mira.cor = c)}></button>
        {/each}
      </div>
      {#each CONTROLES as [k, nome, min, max, passo] (k)}
        <label class="controle">
          <span>{nome}<b>{k === "opacidade" ? `${Math.round(mira[k] * 100)}%` : mira[k]}</b></span>
          <input type="range" {min} {max} step={passo} bind:value={mira[k]} />
        </label>
      {/each}
      <div class="linha-opcao">
        <span>Contorno preto</span>
        <button class="interruptor" role="switch" aria-checked={mira.contorno} aria-label="Contorno preto" onclick={() => (mira.contorno = !mira.contorno)}></button>
      </div>
      <div class="linha-opcao">
        <span>Esconder ao mirar com o botão direito</span>
        <button class="interruptor" role="switch" aria-checked={!!mira.esconder_ao_mirar} aria-label="Esconder ao mirar" onclick={() => (mira.esconder_ao_mirar = !mira.esconder_ao_mirar)}></button>
      </div>
    </div>
  </section>

  <section class="secao">
    <div class="acoes">
      <button class="btn" class:primario={!ligada} onclick={alternar}>{ligada ? "Tirar do jogo" : "Mostrar no jogo"}</button>
      <button class="btn" onclick={copiarCs2}>Copiar para o console do CS2</button>
    </div>
    {#if recado}
      <p class="recado" data-tom={recado.tom}>
        {recado.texto}
        {#if recado.confirmar}<button class="btn quieto" onclick={() => ligar(true)}>Ligar mesmo assim</button>{/if}
      </p>
    {/if}
    <p class="apoio">A mira fica numa janela do Otimiza por cima do jogo. Aparece com o jogo em janela ou sem bordas; em tela cheia exclusiva o Windows não deixa nada por cima.</p>
  </section>

  <section class="secao duas">
    <div>
      <h2>Sensibilidade</h2>
      <label class="campo-linha">DPI do mouse <input class="campo" type="number" min="100" max="32000" bind:value={dpi} /></label>
      <label class="campo-linha">Sensibilidade no CS2 <input class="campo" type="number" min="0.05" step="0.05" bind:value={sens} /></label>
      <div class="campo-linha">Volta completa <b class="numero">{cm ? `${n(cm, 1)} cm` : "—"}</b></div>
      <div class="campo-linha">A mesma mira no Valorant <b class="numero">{n(val, 3)}</b></div>
      <p class="apoio pequeno">CS2, Apex e Valorant giram 0,022° por contagem do mouse. Mesma volta em centímetros, mesma memória muscular.</p>
    </div>
    <div>
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

  <section class="secao">
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
      <label class="controle">
        <span>Campo de visão em primeira pessoa (posição no menu)<b>{fovPosicao}</b></span>
        <input type="range" min="0" max={fov.maximo} step="1" bind:value={fovPosicao} />
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
  }
  .duas {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 24px;
  }
  @media (max-width: 760px) {
    .duas {
      grid-template-columns: 1fr;
    }
  }
  .previa {
    display: grid;
    place-items: center;
    min-height: 220px;
    border-radius: var(--raio-3);
    background: radial-gradient(circle at 50% 40%, #3b4a5e, #11161f);
  }
  .controles {
    display: grid;
    gap: 8px;
  }
  .rotulo {
    font-size: var(--t-apoio);
    color: var(--tinta-2);
  }
  .estilos,
  .cores {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .estilos button {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    border: 1px solid var(--linha);
    background: #1a2230;
    cursor: pointer;
  }
  .estilos button[aria-pressed="true"],
  .cores button[aria-pressed="true"] {
    outline: 2px solid var(--acao);
    outline-offset: 2px;
  }
  .cores button {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 1px solid var(--linha-forte);
    cursor: pointer;
  }
  .controle {
    display: grid;
    gap: 4px;
    font-size: 13px;
  }
  .controle span {
    display: flex;
    justify-content: space-between;
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
    font-size: 13px;
    margin: 0;
  }
  .recado[data-tom="ok"] {
    color: var(--ok);
  }
  .recado[data-tom="atencao"] {
    color: var(--atencao);
  }
  .campo-linha {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 6px 0;
    font-size: 13px;
  }
  .campo-linha .campo {
    width: 110px;
  }
  .pequeno {
    font-size: 12px;
  }
  .treino {
    position: relative;
    height: 170px;
    border-radius: var(--raio-2);
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
</style>
