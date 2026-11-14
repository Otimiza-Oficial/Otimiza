<script lang="ts">
  // O cenário da época, em duas camadas leves (só CSS e SVG; nada de canvas nem laço de quadro):
  //   1. o varal: um fio que pende no vão entre a barra de cima e o painel, com as peças da época (luzinhas no Natal,
  //      bandeirinhas na Junina, brilhos dourados no Ano Novo…). Fica no vão, então nunca cobre texto.
  //   2. o fundo: partículas atrás do vidro (neve, confete, brilho), visíveis nas margens e entre os blocos.
  // A cor da marca, o filete dos cartões e a geada do Natal vêm do `data-epoca` na raiz (`tokens.css`).
  // Com jogo aberto a regra global `.jogo-aberto *` para tudo; com "menos movimento", as peças ficam paradas.
  import type { Efeito } from "../lib/temporadas";

  let { efeito }: { efeito: Efeito } = $props();

  // Posições fixas (sem sortear a cada abertura): o mesmo céu sempre.
  const PARTICULAS = Array.from({ length: 26 }, (_, i) => ({
    x: (i * 37 + 11) % 100,
    atraso: -((i * 29) % 31),
    duracao: 18 + ((i * 7) % 14),
    tamanho: 3 + ((i * 13) % 7),
    desfoque: (i * 5) % 3,
    deriva: ((i * 17) % 60) - 30,
    tom: i,
  }));

  /** As cores de cada época: a primeira é a da marca durante a época. */
  const PALETA: Record<Efeito, string[]> = {
    neve: ["#ff4d4d", "#ffd36b", "#5fd38d", "#ffffff"],
    fogos: ["#ffd36b", "#ffffff", "#ffe9a8", "#f7c948"],
    confete: ["#ff4fa3", "#8b5cf6", "#2ec4b6", "#ffd36b"],
    ovinhos: ["#c4b5fd", "#fbcfe8", "#a7f3d0", "#fde68a"],
    bandeirinhas: ["#ef4444", "#3b82f6", "#facc15", "#22c55e", "#a855f7"],
    baloes: ["#3b82f6", "#facc15", "#ef4444", "#22c55e"],
    morcegos: ["#ff8a1f", "#a855f7", "#ffb347", "#7c3aed"],
  };

  /** As partículas do fundo: a neve é branca (a cor fica nas luzinhas); nas outras épocas, a paleta da época. */
  const COR_DA_PARTICULA: Partial<Record<Efeito, string[]>> = {
    neve: ["#ffffff", "#eef4ff"],
    ovinhos: ["#ffffff", "#fbcfe8"],
  };

  // O fio do varal: uma curva com barriga entre pontos presos. `n` peças distribuídas pelo fio.
  const VAOS = 6;
  const PECAS_DO_VARAL = 44;
  function yNoFio(x: number) {
    // x de 0 a 1000 (viewBox); cada vão é um arco para baixo.
    const vao = 1000 / VAOS;
    const t = (x % vao) / vao;
    return 4 + Math.sin(Math.PI * t) * 14;
  }
  const fio = Array.from({ length: VAOS }, (_, k) => {
    const x0 = (1000 / VAOS) * k;
    const x1 = x0 + 1000 / VAOS;
    return `M${x0},4 Q${(x0 + x1) / 2},${4 + 28} ${x1},4`;
  }).join(" ");
  const pecasDoVaral = Array.from({ length: PECAS_DO_VARAL }, (_, i) => {
    const x = ((i + 0.5) / PECAS_DO_VARAL) * 1000;
    return { x, y: yNoFio(x), i };
  });

  const sobe = $derived(efeito === "baloes" || efeito === "fogos");
</script>

<!-- Fundo: atrás do vidro -->
<div class="fundo-epoca" data-efeito={efeito} aria-hidden="true">
  {#each PARTICULAS as p (p.tom)}
    <span
      class="particula"
      class:sobe
      style:left={`${p.x}%`}
      style:width={`${p.tamanho}px`}
      style:height={`${efeito === "confete" ? p.tamanho * 1.8 : p.tamanho}px`}
      style:filter={p.desfoque ? `blur(${p.desfoque}px)` : undefined}
      style:background={(COR_DA_PARTICULA[efeito] ?? PALETA[efeito])[p.tom % (COR_DA_PARTICULA[efeito] ?? PALETA[efeito]).length]}
      style:animation-delay={`${p.atraso}s`}
      style:animation-duration={`${p.duracao}s`}
      style:--deriva={`${p.deriva}px`}
    ></span>
  {/each}
  {#if efeito === "morcegos"}<div class="nevoa"></div>{/if}
</div>

<!-- Varal: no vão entre a barra de cima e o painel -->
<div class="varal-epoca" data-efeito={efeito} aria-hidden="true">
  <svg viewBox="0 0 1000 40" preserveAspectRatio="none" class="fio">
    <path d={fio} fill="none" stroke="currentColor" stroke-width="1.2" vector-effect="non-scaling-stroke" />
  </svg>
  {#each pecasDoVaral as v (v.i)}
    {@const cor = PALETA[efeito][v.i % PALETA[efeito].length]}
    <span
      class="peca"
      data-forma={efeito}
      style:left={`${v.x / 10}%`}
      style:top={`${(v.y / 40) * 100}%`}
      style:--cor={cor}
      style:animation-delay={`${-((v.i * 0.37) % 3)}s`}
    ></span>
  {/each}
</div>

<style>
  .fundo-epoca {
    position: fixed;
    inset: 0;
    /* Entre o fundo (0) e a casca (1): aparece nas margens e entre os blocos, nunca sobre o texto. */
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
  }
  .particula {
    position: absolute;
    top: -4%;
    border-radius: 50%;
    opacity: 0.45;
    animation-name: cair;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
    will-change: transform;
  }
  [data-efeito="confete"] .particula {
    border-radius: 2px;
  }
  [data-efeito="fogos"] .particula {
    opacity: 0.9;
    box-shadow: 0 0 8px currentColor;
    animation-name: piscar-subindo;
  }
  .particula.sobe {
    top: auto;
    bottom: -4%;
    animation-name: subir;
  }
  [data-efeito="fogos"] .particula.sobe {
    animation-name: piscar-subindo;
  }
  .nevoa {
    position: absolute;
    inset: 40% -20% -10%;
    background:
      radial-gradient(40% 50% at 30% 60%, rgb(168 85 247 / 0.18), transparent 70%),
      radial-gradient(35% 45% at 75% 70%, rgb(255 138 31 / 0.14), transparent 70%);
    filter: blur(20px);
    animation: nevoa 18s ease-in-out infinite alternate;
  }
  @keyframes cair {
    from {
      transform: translate(0, 0);
    }
    to {
      transform: translate(var(--deriva), 108vh);
    }
  }
  @keyframes subir {
    from {
      transform: translate(0, 0);
    }
    to {
      transform: translate(var(--deriva), -108vh);
    }
  }
  @keyframes piscar-subindo {
    0% {
      transform: translate(0, 0) scale(0.6);
      opacity: 0;
    }
    20% {
      opacity: 0.95;
    }
    100% {
      transform: translate(var(--deriva), -108vh) scale(1);
      opacity: 0.2;
    }
  }
  @keyframes nevoa {
    to {
      transform: translateX(6%);
    }
  }

  /* O varal: uma faixa de 34 px logo abaixo da barra de cima (que vai de 14 a 68 px). */
  .varal-epoca {
    position: fixed;
    left: 110px;
    right: 40px;
    top: 66px;
    height: 34px;
    z-index: 2;
    pointer-events: none;
    color: rgb(255 255 255 / 0.22);
  }
  :global(:root[data-tema="dia"]) .varal-epoca {
    color: rgb(0 0 0 / 0.2);
  }
  .fio {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .peca {
    position: absolute;
    width: 7px;
    height: 10px;
    margin-left: -3.5px;
    border-radius: 45% 45% 50% 50%;
    background: var(--cor);
    transform-origin: 50% 0;
  }
  /* Natal: luzinhas que acendem e apagam devagar, cada uma no seu tempo. */
  .peca[data-forma="neve"],
  .peca[data-forma="morcegos"],
  .peca[data-forma="ovinhos"] {
    box-shadow: 0 0 10px 2px var(--cor);
    animation: luz 3s ease-in-out infinite;
  }
  .peca[data-forma="ovinhos"] {
    box-shadow: none;
    width: 8px;
    height: 11px;
    border-radius: 50% 50% 50% 50% / 60% 60% 40% 40%;
  }
  /* Junina e Dia das Crianças: bandeirinhas que balançam. */
  .peca[data-forma="bandeirinhas"],
  .peca[data-forma="baloes"] {
    width: 13px;
    height: 15px;
    margin-left: -6.5px;
    border-radius: 0;
    clip-path: polygon(0 0, 100% 0, 50% 100%);
    animation: balancar 2.6s ease-in-out infinite alternate;
  }
  .peca[data-forma="baloes"] {
    clip-path: none;
    width: 10px;
    height: 12px;
    margin-left: -5px;
    border-radius: 50% 50% 46% 46%;
  }
  /* Ano Novo e Carnaval: brilhos e serpentina. */
  .peca[data-forma="fogos"] {
    width: 9px;
    height: 9px;
    margin-left: -4.5px;
    border-radius: 0;
    clip-path: polygon(50% 0, 62% 38%, 100% 50%, 62% 62%, 50% 100%, 38% 62%, 0 50%, 38% 38%);
    filter: drop-shadow(0 0 4px var(--cor));
    animation: luz 2.2s ease-in-out infinite;
  }
  .peca[data-forma="confete"] {
    width: 4px;
    height: 12px;
    margin-left: -2px;
    border-radius: 2px;
    animation: balancar 2s ease-in-out infinite alternate;
  }
  @keyframes luz {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }
  @keyframes balancar {
    from {
      transform: rotate(-7deg);
    }
    to {
      transform: rotate(7deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .particula,
    .peca,
    .nevoa {
      animation: none;
    }
    .particula {
      display: none;
    }
  }
</style>
