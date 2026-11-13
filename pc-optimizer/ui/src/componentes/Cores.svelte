<script lang="ts">
  // A área Cores (trazida da 3.3, `src/v33/app.ts`, `telaCores`): saturação (vibração digital na NVIDIA), brilho,
  // contraste, gama e matiz do monitor, pelo driver da placa ou pela gama do Windows, com perfil por jogo. A cena é
  // processada pixel a pixel, antes e depois com o divisor em cima da própria imagem. "Voltar ao normal" devolve o
  // que era antes.
  import { onMount } from "svelte";
  import { ferramentas, type CaminhoDasCores, type EstadoDasCores } from "../lib/ferramentas";
  import { NORMAL, PREDEFINICOES, previa, type Ajuste } from "../lib/cores";

  const CAMINHO: Record<CaminhoDasCores, string> = { Amd: "driver AMD", Nvidia: "driver NVIDIA", Gama: "Windows" };
  const NOME_DA_PREDEFINICAO: Record<string, string> = { natural: "Natural", vivo: "Vivo", competitivo: "Competitivo", cinema: "Cinema" };

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

  /** A miniatura de cada predefinição: a mesma cena, inteira com o ajuste dela. */
  function miniatura(no: HTMLCanvasElement, ajuste: Ajuste) {
    previa(no, ajuste, 0);
  }

  const vale = (k: keyof Ajuste) => st?.suporta[k] ?? false;
  const igual = (x: Ajuste, y: Ajuste) => (Object.keys(x) as (keyof Ajuste)[]).every((k) => x[k] === y[k]);
  const pct = (v: number, min: number, max: number) => `${((v - min) / (max - min)) * 100}%`;

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

<div class="estudio">
  <!-- A cena: antes à esquerda do divisor, depois à direita. O divisor é a própria imagem. -->
  <div class="palco">
    <canvas bind:this={canvas}></canvas>
    <span class="canto esquerda">Antes</span>
    <span class="canto direita">Depois</span>
    <span class="estado" class:vivo={aoVivo}><span class="luz"></span>{aoVivo ? "No monitor agora" : "Só na prévia"}</span>
    <div class="divisor" style:left="{corte}%" aria-hidden="true"><span class="pega"><span></span><span></span></span></div>
    <input class="arrastar" type="range" min="0" max="100" bind:value={corte} aria-label="Divisor entre antes e depois" />
  </div>

  <div class="predefinicoes" role="radiogroup" aria-label="Predefinições">
    {#each Object.keys(PREDEFINICOES) as nome (nome)}
      <button class="predef" role="radio" aria-checked={igual(a, PREDEFINICOES[nome])} onclick={() => predefinir(nome)}>
        <canvas use:miniatura={PREDEFINICOES[nome]}></canvas>
        <span>{NOME_DA_PREDEFINICAO[nome] ?? nome}</span>
      </button>
    {/each}
  </div>

  <div class="ajustes">
    {#each controles as [k, nome, min, max] (k)}
      <label class="ajuste" class:apagado={!vale(k)}>
        <span class="cabeca"><span>{nome}</span><b class="numero">{valor(k)}</b></span>
        <input class="deslizante" type="range" {min} {max} bind:value={a[k]} disabled={!vale(k)} oninput={mudou} style:--pct={pct(a[k], min, max)} />
        {#if lido && !vale(k)}<span class="porque">O {CAMINHO[st?.caminho ?? "Gama"]} não muda isto.</span>{/if}
      </label>
    {/each}
  </div>

  <div class="barra">
    <div class="so-jogo">
      <button class="interruptor" role="switch" aria-checked={soComJogo} aria-label="Só com o jogo aberto" disabled={!jogo} onclick={alternarSoComJogo}></button>
      <div>
        <span>Só com {jogo ? `o ${jogo.nome}` : "o jogo"} aberto</span>
        <p class="apoio">{jogo ? "Liga quando ele abre e devolve quando ele fecha." : "Abra um jogo para salvar o ajuste dele."}</p>
      </div>
    </div>
    <div class="acoes">
      <button class="btn" onclick={normal}>Voltar ao normal</button>
      <button class="btn primario" disabled={!st} onclick={aplicar}>Aplicar no monitor</button>
    </div>
  </div>
  {#if recado}<p class="recado" data-tom={recado.tom}>{recado.texto}</p>{/if}
  {#if lido}<p class="apoio nota">{st ? st.nota : "O Otimiza não conseguiu ler as cores deste monitor."}</p>{/if}
</div>

<style>
  .estudio {
    display: grid;
    gap: 16px;
    padding: 8px 0 4px;
  }
  .palco {
    position: relative;
    aspect-ratio: 16 / 7;
    border-radius: 20px;
    overflow: hidden;
    background: #111;
    box-shadow: 0 12px 30px rgb(0 0 0 / 0.18);
  }
  .palco canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .canto,
  .estado {
    position: absolute;
    top: 12px;
    padding: 4px 10px;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.45);
    color: #fff;
    font-size: 12px;
    font-weight: 500;
    backdrop-filter: blur(6px);
    pointer-events: none;
  }
  .canto.esquerda {
    left: 12px;
  }
  .canto.direita {
    right: 12px;
  }
  .estado {
    top: auto;
    bottom: 12px;
    left: 12px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
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
  .divisor {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: #fff;
    box-shadow: 0 0 8px rgb(0 0 0 / 0.5);
    pointer-events: none;
  }
  .pega {
    position: absolute;
    top: 50%;
    left: 50%;
    display: flex;
    gap: 4px;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    margin: -17px 0 0 -17px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 10px rgb(0 0 0 / 0.35);
  }
  .pega span {
    width: 0;
    height: 0;
    border-top: 5px solid transparent;
    border-bottom: 5px solid transparent;
  }
  .pega span:first-child {
    border-right: 6px solid #1d1d1f;
  }
  .pega span:last-child {
    border-left: 6px solid #1d1d1f;
  }
  /* O controle de verdade cobre a cena inteira, invisível: arrastar em qualquer ponto move o divisor. */
  .arrastar {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: ew-resize;
  }
  .predefinicoes {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }
  .predef {
    display: grid;
    gap: 6px;
    padding: 6px 6px 8px;
    border-radius: 14px;
    border: 2px solid transparent;
    background: var(--superficie-2);
    color: inherit;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition:
      border-color 160ms,
      transform 160ms cubic-bezier(0.3, 1.5, 0.5, 1);
  }
  .predef:hover {
    transform: translateY(-2px);
  }
  .predef[aria-checked="true"] {
    border-color: var(--tinta);
  }
  .predef canvas {
    width: 100%;
    object-fit: cover;
    aspect-ratio: 16 / 7;
    border-radius: 9px;
  }
  .ajustes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px 28px;
    padding: 16px 18px;
    border-radius: 18px;
    background: var(--superficie);
    border: 1px solid var(--linha);
  }
  .ajuste {
    display: grid;
    gap: 4px;
    font-size: 13px;
  }
  .cabeca {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .cabeca b {
    font-weight: 600;
  }
  .apagado .cabeca {
    opacity: 0.5;
  }
  .porque {
    font-size: 12px;
    color: var(--tinta-2);
  }
  .barra {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .so-jogo {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }
  .so-jogo p {
    margin: 1px 0 0;
    font-size: 12px;
  }
  .acoes {
    display: flex;
    gap: 10px;
  }
  .recado,
  .nota {
    margin: 0;
    font-size: 12px;
  }
  .recado[data-tom="ok"] {
    color: var(--ok);
  }
  .recado[data-tom="atencao"] {
    color: var(--atencao);
  }
  @media (max-width: 760px) {
    .predefinicoes {
      grid-template-columns: 1fr 1fr;
    }
    .ajustes {
      grid-template-columns: 1fr;
    }
  }
</style>
