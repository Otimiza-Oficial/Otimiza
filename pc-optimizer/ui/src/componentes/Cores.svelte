<script lang="ts">
  // A área Cores (trazida da 3.3, `src/v33/app.ts`, `telaCores`): saturação (vibração digital na NVIDIA), brilho,
  // contraste, gama e matiz do monitor, pelo driver da placa ou pela gama do Windows, com perfil por jogo. A prévia é
  // uma cena processada pixel a pixel, metade antes e metade depois. "Voltar ao normal" devolve o que era antes.
  import { onMount } from "svelte";
  import { ferramentas, type CaminhoDasCores, type EstadoDasCores } from "../lib/ferramentas";
  import { NORMAL, PREDEFINICOES, previa, type Ajuste } from "../lib/cores";

  const CAMINHO: Record<CaminhoDasCores, string> = { Amd: "driver AMD", Nvidia: "driver NVIDIA", Gama: "Windows" };

  let st = $state<EstadoDasCores | null>(null);
  let jogo = $state<{ id: string; nome: string } | null>(null);
  let a = $state<Ajuste>({ ...NORMAL });
  let soComJogo = $state(false);
  let aoVivo = $state(false);
  let corte = $state(50);
  let recado = $state<{ texto: string; tom: "ok" | "atencao" } | null>(null);
  let canvas = $state<HTMLCanvasElement | null>(null);
  let espera: ReturnType<typeof setTimeout> | undefined;
  let lido = $state(false);

  const controles = $derived<[keyof Ajuste, string, number, number][]>([
    ["saturacao", st?.caminho === "Nvidia" ? "Vibração digital" : "Saturação", 0, 100],
    ["brilho", "Brilho", -40, 40],
    ["contraste", "Contraste", -40, 40],
    ["gama", "Gama", 70, 150],
    ["matiz", "Matiz", -30, 30],
  ]);

  onMount(async () => {
    [st, jogo] = await Promise.all([ferramentas.coresEstado().catch(() => null), ferramentas.jogoAberto().catch(() => null)]);
    const salvo = jogo ? st?.perfis.por_jogo[jogo.id] : undefined;
    // Começa do que está salvo para o jogo, ou do valor real do driver: quem já usa 100% não é jogado para 50.
    a = salvo ? { ...salvo } : { ...NORMAL, saturacao: st?.saturacao_atual ?? NORMAL.saturacao };
    soComJogo = st?.perfis.so_com_jogo ?? false;
    aoVivo = st?.aplicado ?? false;
    lido = true;
  });

  $effect(() => {
    const ajuste = $state.snapshot(a) as Ajuste;
    if (canvas) previa(canvas, ajuste, corte / 100);
  });

  const vale = (k: keyof Ajuste) => st?.suporta[k] ?? false;
  const igual = (x: Ajuste, y: Ajuste) => (Object.keys(x) as (keyof Ajuste)[]).every((k) => x[k] === y[k]);

  function valor(k: keyof Ajuste) {
    const v = a[k];
    if (k === "saturacao") return `${v}%`;
    if (k === "gama") return (v / 100).toFixed(2).replace(".", ",");
    if (k === "matiz") return `${v}°`;
    return `${v > 0 ? "+" : ""}${v}`;
  }

  async function salvar() {
    if (jogo) await ferramentas.coresSalvarPerfil(jogo.id, $state.snapshot(a) as Ajuste, soComJogo).catch(() => {});
  }

  async function aplicar() {
    try {
      const c = await ferramentas.coresAplicar($state.snapshot(a) as Ajuste);
      aoVivo = true;
      await salvar();
      recado = { texto: `Aplicado pelo ${CAMINHO[c]}.${soComJogo ? " Volta ao normal quando o jogo fechar." : " Fica até você voltar ao normal."}`, tom: "ok" };
    } catch (e) {
      recado = { texto: String(e), tom: "atencao" };
    }
  }

  /** Depois do primeiro "Aplicar", arrastar já muda o monitor, como no painel do driver. */
  function mudou() {
    if (!aoVivo) return;
    clearTimeout(espera);
    espera = setTimeout(() => void aplicar(), 150);
  }

  function predefinir(nome: string) {
    const p = PREDEFINICOES[nome];
    // O que o caminho não muda fica no normal, para a prévia não prometer o que o monitor não vai mostrar.
    a = Object.fromEntries((Object.keys(p) as (keyof Ajuste)[]).map((k) => [k, vale(k) ? p[k] : NORMAL[k]])) as unknown as Ajuste;
    mudou();
  }

  async function normal() {
    clearTimeout(espera);
    aoVivo = false;
    try {
      await ferramentas.coresRestaurar();
      const depois = await ferramentas.coresEstado().catch(() => null);
      a = { ...NORMAL, saturacao: depois?.saturacao_atual ?? NORMAL.saturacao };
      if (jogo) await ferramentas.coresSalvarPerfil(jogo.id, null, soComJogo).catch(() => {});
      recado = { texto: "As cores voltaram ao que eram antes do Otimiza.", tom: "ok" };
    } catch (e) {
      recado = { texto: String(e), tom: "atencao" };
    }
  }

  async function alternarSoComJogo() {
    soComJogo = !soComJogo;
    await salvar();
    recado = {
      texto: soComJogo ? `Salvo para o ${jogo?.nome ?? "jogo"}: liga sozinho quando ele abrir.` : "Desligado: o ajuste fica só quando você aplicar.",
      tom: "ok",
    };
  }
</script>

<div class="area-cores">
  <section class="secao duas">
    <div class="lado-previa">
      <div class="previa"><canvas bind:this={canvas}></canvas></div>
      <input type="range" min="0" max="100" bind:value={corte} aria-label="Comparar antes e depois" />
      <p class="apoio pequeno">À esquerda como está, à direita com o ajuste. Arraste para comparar.</p>
    </div>
    <div class="controles">
      <div class="segmento" role="radiogroup" aria-label="Predefinições">
        {#each Object.keys(PREDEFINICOES) as nome (nome)}
          <button role="radio" aria-checked={igual(a, PREDEFINICOES[nome])} onclick={() => predefinir(nome)}>
            {nome[0].toUpperCase() + nome.slice(1)}
          </button>
        {/each}
      </div>
      {#each controles as [k, nome, min, max] (k)}
        <label class="controle" class:apagado={!vale(k)} title={vale(k) ? undefined : `Pelo ${CAMINHO[st?.caminho ?? "Gama"]} isso não muda`}>
          <span>{nome}<b>{valor(k)}</b></span>
          <input type="range" {min} {max} bind:value={a[k]} disabled={!vale(k)} oninput={mudou} />
        </label>
      {/each}
      <div class="linha-opcao">
        <div>
          <span>Só com o jogo aberto</span>
          <p class="apoio pequeno">{jogo ? `Liga quando o ${jogo.nome} abre e devolve quando ele fecha.` : "Abra um jogo para salvar o ajuste dele."}</p>
        </div>
        <button class="interruptor" role="switch" aria-checked={soComJogo} aria-label="Só com o jogo aberto" disabled={!jogo} onclick={alternarSoComJogo}></button>
      </div>
      <div class="acoes">
        <button class="btn primario" disabled={!st} onclick={aplicar}>Aplicar no monitor</button>
        <button class="btn" onclick={normal}>Voltar ao normal</button>
      </div>
      {#if lido}<p class="apoio pequeno">{st ? st.nota : "O Otimiza não conseguiu ler as cores deste monitor."}</p>{/if}
      {#if recado}<p class="recado" data-tom={recado.tom}>{recado.texto}</p>{/if}
    </div>
  </section>
</div>

<style>
  .duas {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr);
    gap: 24px;
  }
  @media (max-width: 760px) {
    .duas {
      grid-template-columns: 1fr;
    }
  }
  .lado-previa {
    display: grid;
    gap: 8px;
    align-content: start;
  }
  .previa {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: var(--raio-3);
    overflow: hidden;
    background: #111;
  }
  .previa canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .controles {
    display: grid;
    gap: 10px;
    align-content: start;
  }
  .segmento {
    display: flex;
    gap: 4px;
    padding: 3px;
    border-radius: 10px;
    background: var(--superficie-2);
    width: fit-content;
  }
  .segmento button {
    border: 0;
    padding: 6px 12px;
    border-radius: 7px;
    background: transparent;
    color: var(--tinta-2);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .segmento button[aria-checked="true"] {
    background: var(--superficie);
    color: var(--tinta);
    box-shadow: var(--sombra-1);
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
  .apagado {
    opacity: 0.4;
  }
  .linha-opcao {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-size: 13px;
  }
  .linha-opcao p,
  .lado-previa p {
    margin: 2px 0 0;
  }
  .acoes {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .pequeno {
    font-size: 12px;
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
</style>
