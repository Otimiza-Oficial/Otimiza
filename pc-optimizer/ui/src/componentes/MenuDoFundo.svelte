<script lang="ts">
  // O menu do Fundo: miniaturas dos fundos do Otimiza, a imagem da pessoa (escolher, trocar, tirar) e, nas fotos, o
  // desfoque e o quanto escurece. Fecha ao clicar fora ou no Esc.
  import Icone from "./Icone.svelte";
  import { FUNDOS, opcaoDo, prepararImagem, type AjusteDoFundo, type Fundo } from "../lib/fundos";

  let {
    fundo,
    imagem,
    ajuste,
    onescolher,
    onimagem,
    onajuste,
    onfechar,
  }: {
    fundo: Fundo;
    imagem: string | null;
    ajuste: AjusteDoFundo;
    onescolher: (f: Fundo) => void;
    /** A imagem nova (já reduzida) ou `null` para tirar. Devolve falso quando não coube no armazenamento. */
    onimagem: (dados: string | null) => boolean;
    onajuste: (a: AjusteDoFundo) => void;
    onfechar: () => void;
  } = $props();

  let caixa: HTMLDivElement;
  let entrada: HTMLInputElement;
  let carregando = $state(false);
  let erro = $state<string | null>(null);

  const prontos = FUNDOS.filter((f) => f.id !== "meu");
  const ehFoto = $derived(opcaoDo(fundo).tipo === "foto");

  async function aoEscolherArquivo() {
    const arquivo = entrada.files?.[0];
    entrada.value = "";
    if (!arquivo) return;
    erro = null;
    carregando = true;
    try {
      const dados = await prepararImagem(arquivo);
      if (!onimagem(dados)) {
        erro = "A imagem não coube no armazenamento do app. Tente uma menor.";
        return;
      }
      onescolher("meu");
    } catch (e) {
      erro = e instanceof Error ? e.message : "Não deu para usar essa imagem.";
    } finally {
      carregando = false;
    }
  }

  function tirarImagem() {
    onimagem(null);
    if (fundo === "meu") onescolher("limpo");
  }

  function foraDoMenu(e: PointerEvent) {
    const alvo = e.target as Node;
    if (caixa && !caixa.contains(alvo) && !(alvo as HTMLElement).closest?.(".fundo-botao")) onfechar();
  }
</script>

<svelte:window onpointerdown={foraDoMenu} onkeydown={(e) => e.key === "Escape" && onfechar()} />

<div class="menu" role="menu" aria-label="Fundo" bind:this={caixa}>
  <p class="rotulo">Fundos do Otimiza</p>
  <div class="grade">
    {#each prontos as f (f.id)}
      <button role="menuitemradio" aria-checked={fundo === f.id} class="opcao" onclick={() => onescolher(f.id)}>
        <span class="mini" data-mini={f.id} style:background-image={f.foto ? `url('${f.foto}')` : null}></span>
        <span class="nome">{f.nome}</span>
      </button>
    {/each}
  </div>

  <p class="rotulo">Sua imagem</p>
  <div class="minha">
    {#if imagem}
      <button role="menuitemradio" aria-checked={fundo === "meu"} class="opcao larga" aria-label="Usar a sua imagem" onclick={() => onescolher("meu")}>
        <span class="mini" style:background-image="url('{imagem}')"></span>
      </button>
      <div class="acoes">
        <button class="acao" onclick={() => entrada.click()} disabled={carregando}>{carregando ? "Preparando…" : "Trocar"}</button>
        <button class="acao discreta" onclick={tirarImagem}>Tirar</button>
      </div>
    {:else}
      <button class="enviar" onclick={() => entrada.click()} disabled={carregando}>
        <span class="mais"><Icone nome="mais" tamanho={18} /></span>
        <span>
          <strong>{carregando ? "Preparando a imagem…" : "Escolher uma imagem"}</strong>
          <span class="dica">JPG, PNG ou WebP. Fica só neste PC.</span>
        </span>
      </button>
    {/if}
    <input bind:this={entrada} type="file" accept="image/png,image/jpeg,image/webp,image/gif,image/bmp" hidden onchange={aoEscolherArquivo} />
  </div>
  {#if erro}<p class="erro" role="alert">{erro}</p>{/if}

  {#if ehFoto}
    <div class="ajustes">
      <label>
        <span>Desfoque <em>{ajuste.desfoque}px</em></span>
        <input
          type="range"
          min="0"
          max="30"
          step="1"
          value={ajuste.desfoque}
          oninput={(e) => onajuste({ ...ajuste, desfoque: Number(e.currentTarget.value) })}
        />
      </label>
      <label>
        <span>Escurecer <em>{Math.round(ajuste.escurecer * 100)}%</em></span>
        <input
          type="range"
          min="0"
          max="0.85"
          step="0.05"
          value={ajuste.escurecer}
          oninput={(e) => onajuste({ ...ajuste, escurecer: Number(e.currentTarget.value) })}
        />
      </label>
    </div>
  {/if}
</div>

<style>
  .menu {
    position: absolute;
    right: 0;
    top: 44px;
    z-index: 30;
    display: grid;
    gap: 8px;
    width: 340px;
    padding: 14px;
    border-radius: 20px;
    border: 1px solid var(--vidro-borda);
    background: rgb(250 250 252 / 0.98);
    box-shadow: 0 24px 60px rgb(0 0 0 / 0.4);
    color: var(--texto);
  }
  :global(:root[data-tema="noite"]) .menu {
    background: rgb(26 26 30 / 0.98);
  }
  .rotulo {
    margin: 2px 2px 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--texto-2, var(--tinta-2));
  }
  .grade {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .opcao {
    display: grid;
    gap: 5px;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .mini {
    position: relative;
    display: block;
    height: 58px;
    border-radius: 12px;
    border: 1px solid var(--linha-forte);
    background-size: cover;
    background-position: center;
    overflow: hidden;
    transition: transform 0.18s ease, box-shadow 0.18s ease;
  }
  .opcao:hover .mini {
    transform: translateY(-2px);
    box-shadow: 0 8px 18px -6px rgb(0 0 0 / 0.5);
  }
  .opcao[aria-checked="true"] .mini {
    border-color: transparent;
    box-shadow: 0 0 0 2px var(--marca), 0 8px 20px -8px var(--marca);
  }
  .opcao[aria-checked="true"] .mini::after {
    content: "✓";
    position: absolute;
    right: 5px;
    top: 5px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--marca);
    color: #fff;
    font-size: 11px;
    font-weight: 800;
  }
  .nome {
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  [data-mini="limpo"] {
    background: linear-gradient(180deg, #2a2a2f, #0b0b0d);
  }
  [data-mini="aurora"] {
    background:
      radial-gradient(circle at 20% 20%, #2dd4bf, transparent 55%),
      radial-gradient(circle at 85% 60%, #7c3aed, transparent 55%),
      #0a0a16;
  }
  [data-mini="brasa"] {
    background:
      radial-gradient(circle at 50% 110%, #ff3b3b, transparent 60%),
      radial-gradient(circle at 90% 30%, #f97316aa, transparent 50%),
      #120708;
  }
  [data-mini="oceano"] {
    background:
      radial-gradient(circle at 20% 20%, #0ea5e9, transparent 55%),
      radial-gradient(circle at 80% 80%, #1d4ed8, transparent 55%),
      #04101c;
  }
  [data-mini="neon"] {
    background:
      linear-gradient(180deg, transparent 48%, #f472b6 49%, transparent 52%),
      linear-gradient(rgb(236 72 153 / 0.5) 1px, transparent 1px) 0 0 / 10px 8px,
      linear-gradient(180deg, #1a0630, #0a0414);
  }
  [data-mini="comodo"] {
    background: linear-gradient(180deg, #d9d4cd 70%, #a79c8f 70%);
  }
  [data-mini="area"] {
    background: repeating-conic-gradient(var(--linha-forte) 0 25%, transparent 0 50%) 0 0 / 10px 10px;
  }
  .minha {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .larga {
    flex: 1;
  }
  .larga .mini {
    height: 64px;
  }
  .acoes {
    display: grid;
    gap: 6px;
  }
  .acao {
    height: 29px;
    padding: 0 14px;
    border: 1px solid var(--linha-forte);
    border-radius: 10px;
    background: var(--bloco-2);
    color: inherit;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .acao.discreta {
    background: none;
  }
  .enviar {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border: 1.5px dashed var(--linha-forte);
    border-radius: 14px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.18s ease, background 0.18s ease;
  }
  .enviar:hover:not(:disabled) {
    border-color: var(--marca);
    background: var(--marca-suave);
  }
  .mais {
    display: grid;
    place-items: center;
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: var(--marca);
    color: #fff;
  }
  .enviar strong {
    display: block;
    font-size: 13px;
  }
  .dica {
    font-size: 11.5px;
    opacity: 0.7;
  }
  .erro {
    margin: 0;
    font-size: 12px;
    color: var(--perigo, #ff453a);
  }
  .ajustes {
    display: grid;
    gap: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--linha-forte);
  }
  .ajustes label {
    display: grid;
    gap: 4px;
    font-size: 12px;
    font-weight: 600;
  }
  .ajustes label span {
    display: flex;
    justify-content: space-between;
  }
  .ajustes em {
    font-style: normal;
    font-variant-numeric: tabular-nums;
    opacity: 0.7;
  }
  .ajustes input {
    width: 100%;
    accent-color: var(--marca);
  }
</style>
