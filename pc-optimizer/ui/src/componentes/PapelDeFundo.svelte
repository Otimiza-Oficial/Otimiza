<script lang="ts">
  // O fundo atrás do vidro quando é cena ou foto: as cenas são feitas em CSS (luzes que andam devagar), as fotos são
  // as do FiveM ou a imagem da pessoa. Por cima vai um véu que escurece (ou clareia, no tema de dia) para os cards
  // continuarem lendo bem. Com jogo aberto tudo para de animar.
  import { opcaoDo, type AjusteDoFundo, type Fundo } from "../lib/fundos";

  let { fundo, imagem, ajuste }: { fundo: Fundo; imagem: string | null; ajuste: AjusteDoFundo } = $props();

  const opcao = $derived(opcaoDo(fundo));
  const foto = $derived(fundo === "meu" ? imagem : (opcao.foto ?? null));
</script>

{#if opcao.tipo === "cena" || (opcao.tipo === "foto" && foto)}
  <div
    class="papel"
    data-papel={fundo}
    aria-hidden="true"
    style:--desfoque="{ajuste.desfoque}px"
    style:--escurecer={ajuste.escurecer}
  >
    {#if opcao.tipo === "foto" && foto}
      <div class="foto" style:background-image="url('{foto}')"></div>
    {:else}
      <div class="cena"><span class="luz l1"></span><span class="luz l2"></span><span class="luz l3"></span></div>
    {/if}
    <div class="veu"></div>
  </div>
{/if}

<style>
  .papel {
    position: fixed;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    background: #08080b;
  }
  .foto {
    position: absolute;
    inset: calc(var(--desfoque) * -2);
    background-size: cover;
    background-position: center;
    filter: blur(var(--desfoque)) saturate(1.1);
    animation: respira 40s ease-in-out infinite alternate;
  }
  .veu {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(120% 80% at 50% 0%, transparent 40%, rgb(0 0 0 / 0.35)),
      rgb(6 6 10 / var(--escurecer));
  }
  :global(:root[data-tema="dia"]) .veu {
    background: rgb(246 246 250 / calc(var(--escurecer) * 0.9));
  }
  .cena {
    position: absolute;
    inset: 0;
  }
  /* Cada cena: uma base e três luzes grandes e desfocadas que andam devagar. */
  .luz {
    position: absolute;
    width: 70vmax;
    height: 70vmax;
    border-radius: 50%;
    filter: blur(80px);
    opacity: 0.75;
    animation: anda 28s ease-in-out infinite alternate;
  }
  .l1 {
    left: -20vmax;
    top: -25vmax;
  }
  .l2 {
    right: -25vmax;
    top: 10vmax;
    animation-duration: 34s;
    animation-delay: -8s;
  }
  .l3 {
    left: 15vmax;
    bottom: -40vmax;
    animation-duration: 40s;
    animation-delay: -16s;
  }
  [data-papel="aurora"] {
    background: linear-gradient(180deg, #070912, #0c0a18);
  }
  [data-papel="aurora"] .l1 {
    background: radial-gradient(circle, #2dd4bf, transparent 65%);
  }
  [data-papel="aurora"] .l2 {
    background: radial-gradient(circle, #7c3aed, transparent 65%);
  }
  [data-papel="aurora"] .l3 {
    background: radial-gradient(circle, #22c55e, transparent 65%);
    opacity: 0.45;
  }
  [data-papel="brasa"] {
    background: linear-gradient(180deg, #0d0607, #160809);
  }
  [data-papel="brasa"] .l1 {
    background: radial-gradient(circle, #ff3b3b, transparent 65%);
    opacity: 0.5;
  }
  [data-papel="brasa"] .l2 {
    background: radial-gradient(circle, #f97316, transparent 65%);
    opacity: 0.45;
  }
  [data-papel="brasa"] .l3 {
    background: radial-gradient(circle, #e5262c, transparent 65%);
    opacity: 0.8;
  }
  [data-papel="oceano"] {
    background: linear-gradient(180deg, #04101c, #061826);
  }
  [data-papel="oceano"] .l1 {
    background: radial-gradient(circle, #0ea5e9, transparent 65%);
    opacity: 0.55;
  }
  [data-papel="oceano"] .l2 {
    background: radial-gradient(circle, #1d4ed8, transparent 65%);
  }
  [data-papel="oceano"] .l3 {
    background: radial-gradient(circle, #06b6d4, transparent 65%);
    opacity: 0.5;
  }
  /* Neon: horizonte rosa e ciano com uma grade em perspectiva no chão. */
  [data-papel="neon"] {
    background: linear-gradient(180deg, #0a0414 0%, #1a0630 55%, #0a0414 100%);
  }
  [data-papel="neon"] .l1 {
    background: radial-gradient(circle, #ec4899, transparent 65%);
    left: 10vmax;
    top: -20vmax;
    opacity: 0.55;
  }
  [data-papel="neon"] .l2 {
    background: radial-gradient(circle, #22d3ee, transparent 65%);
    opacity: 0.4;
  }
  [data-papel="neon"] .l3 {
    display: none;
  }
  [data-papel="neon"] .cena::after {
    content: "";
    position: absolute;
    left: -50%;
    right: -50%;
    bottom: -10%;
    height: 60%;
    background:
      linear-gradient(rgb(236 72 153 / 0.5) 1px, transparent 1px) 0 0 / 60px 60px,
      linear-gradient(90deg, rgb(34 211 238 / 0.4) 1px, transparent 1px) 0 0 / 60px 60px;
    transform: perspective(400px) rotateX(62deg);
    transform-origin: top;
    mask-image: linear-gradient(180deg, transparent, #000 35%);
    animation: grade 6s linear infinite;
  }
  [data-papel="neon"] .cena::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 40%;
    height: 2px;
    background: linear-gradient(90deg, transparent, #f472b6, #67e8f9, transparent);
    box-shadow: 0 0 30px 6px rgb(236 72 153 / 0.5);
  }
  /* As cenas usam o véu só um pouco: elas já nascem escuras. */
  .cena + .veu {
    background: rgb(6 6 10 / calc(var(--escurecer) * 0.5));
  }
  :global(:root[data-tema="dia"]) .cena + .veu {
    background: rgb(246 246 250 / 0.55);
  }
  @keyframes anda {
    to {
      transform: translate(12vmax, 8vmax) scale(1.15);
    }
  }
  @keyframes respira {
    to {
      transform: scale(1.06);
    }
  }
  @keyframes grade {
    to {
      background-position: 0 60px, 0 0;
    }
  }
  :global(:root.jogo-aberto) .papel *,
  :global(:root.jogo-aberto) .papel *::before,
  :global(:root.jogo-aberto) .papel *::after {
    animation-play-state: paused;
  }
  @media (prefers-reduced-motion: reduce) {
    .papel *,
    .papel *::before,
    .papel *::after {
      animation: none !important;
    }
  }
</style>
