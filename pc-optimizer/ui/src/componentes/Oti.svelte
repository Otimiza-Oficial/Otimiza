<script lang="ts" module>
  /** O que o mascote está sentindo. As reações curtas (feliz, surpreso) a casca liga e desliga sozinha. */
  export type Humor =
    | "atento"
    | "falando"
    | "feliz"
    | "pensando"
    | "preocupado"
    | "surpreso"
    | "jogando"
    | "dormindo";

  /** O que as mãos estão fazendo. */
  export type Gesto = "nenhum" | "acenar" | "apontar" | "comemorar";
</script>

<script lang="ts">
  // O Oti, mascote do Otimiza 1.0. Um bichinho de vidro roxo-azulado com olhos grandes de reflexo âmbar (a cor da
  // marca), antena com lâmpada, bochechas e duas mãozinhas que flutuam ao lado do corpo. Tudo em vetor: escala para o
  // ícone do app e para a tela inteira. Com jogo aberto a casca congela as animações (classe `jogo-aberto`).
  import { onMount } from "svelte";

  let {
    humor = "atento",
    gesto = "nenhum",
    tamanho = 240,
    segueCursor = true,
  }: { humor?: Humor; gesto?: Gesto; tamanho?: number; segueCursor?: boolean } = $props();

  const id = `oti${Math.random().toString(36).slice(2, 8)}`;

  let olhar = $state({ x: 0, y: 0 });
  let raiz: SVGSVGElement | undefined = $state();

  onMount(() => {
    const reduzir = window.matchMedia("(prefers-reduced-motion: reduce)");
    let quadro = 0;
    let ultimo: PointerEvent | null = null;
    const mover = (e: PointerEvent) => {
      ultimo = e;
      if (quadro || !segueCursor || reduzir.matches || !raiz) return;
      quadro = requestAnimationFrame(() => {
        quadro = 0;
        if (!ultimo || !raiz) return;
        const r = raiz.getBoundingClientRect();
        const dx = ultimo.clientX - (r.left + r.width / 2);
        const dy = ultimo.clientY - (r.top + r.height * 0.45);
        const d = Math.hypot(dx, dy) || 1;
        const k = Math.min(1, d / 420);
        olhar = { x: (dx / d) * 5 * k, y: (dy / d) * 4 * k };
      });
    };
    window.addEventListener("pointermove", mover, { passive: true });
    return () => {
      window.removeEventListener("pointermove", mover);
      cancelAnimationFrame(quadro);
    };
  });

  // Para onde os olhos olham além do cursor: pensando olha para cima, preocupado para baixo.
  const desvio = $derived(
    humor === "pensando"
      ? { x: 4, y: -5 }
      : humor === "preocupado"
        ? { x: 0, y: 3 }
        : humor === "jogando"
          ? { x: 0, y: 1 }
          : segueCursor
            ? olhar
            : { x: 0, y: 0 }
  );

  const boca = $derived(
    humor === "feliz"
      ? "M112 150 Q130 172 148 150 Q130 160 112 150 Z"
      : humor === "preocupado"
        ? "M116 158 Q123 152 130 157 Q137 162 144 156"
        : humor === "pensando" || humor === "jogando"
          ? "M121 155 L139 155"
          : humor === "dormindo"
            ? "M122 156 Q130 160 138 156"
            : "M116 151 Q130 163 144 151"
  );

  // O mesmo contorno serve para o corpo e as camadas de luz por cima dele.
  const CORPO = "M130 48 C192 48 216 90 216 130 C216 180 180 210 130 210 C80 210 44 180 44 130 C44 90 68 48 130 48 Z";
</script>

<svg
  bind:this={raiz}
  class="oti"
  data-humor={humor}
  data-gesto={gesto}
  width={tamanho}
  height={tamanho}
  viewBox="0 0 260 260"
  role="img"
  aria-label="Oti, o mascote do Otimiza"
>
  <defs>
    <linearGradient id="{id}-corpo" x1="0.15" y1="0" x2="0.85" y2="1">
      <stop offset="0%" stop-color="#b3ccff" />
      <stop offset="45%" stop-color="#7d8dff" />
      <stop offset="100%" stop-color="#8b5ff0" />
    </linearGradient>
    <radialGradient id="{id}-luz" cx="32%" cy="24%" r="62%">
      <stop offset="0%" stop-color="#ffffff" stop-opacity="0.8" />
      <stop offset="45%" stop-color="#ffffff" stop-opacity="0.14" />
      <stop offset="100%" stop-color="#ffffff" stop-opacity="0" />
    </radialGradient>
    <radialGradient id="{id}-fundo" cx="62%" cy="98%" r="58%">
      <stop offset="0%" stop-color="#ffc9f0" stop-opacity="0.8" />
      <stop offset="100%" stop-color="#ffc9f0" stop-opacity="0" />
    </radialGradient>
    <radialGradient id="{id}-olho" cx="50%" cy="38%" r="70%">
      <stop offset="0%" stop-color="#2d2b62" />
      <stop offset="70%" stop-color="#15133c" />
      <stop offset="100%" stop-color="#0c0a26" />
    </radialGradient>
    <radialGradient id="{id}-iris" cx="50%" cy="102%" r="60%">
      <stop offset="0%" stop-color="#ffb347" stop-opacity="1" />
      <stop offset="100%" stop-color="#ffb347" stop-opacity="0" />
    </radialGradient>
    <radialGradient id="{id}-lampada" cx="40%" cy="35%" r="70%">
      <stop offset="0%" stop-color="#fff3d6" />
      <stop offset="45%" stop-color="#ffc163" />
      <stop offset="100%" stop-color="#f08a1c" />
    </radialGradient>
    <radialGradient id="{id}-brilho" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#ffc163" stop-opacity="0.85" />
      <stop offset="100%" stop-color="#ffc163" stop-opacity="0" />
    </radialGradient>
    <linearGradient id="{id}-headset" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#40456f" />
      <stop offset="100%" stop-color="#1c1f3a" />
    </linearGradient>
    <filter id="{id}-suave" x="-40%" y="-40%" width="180%" height="180%">
      <feGaussianBlur stdDeviation="3" />
    </filter>
  </defs>

  <!-- Sombra no chão: encolhe quando ele sobe. -->
  <ellipse class="sombra" cx="130" cy="240" rx="58" ry="9" fill="#05061a" opacity="0.3" filter="url(#{id}-suave)" />

  <g class="flutua">
    <!-- Antena com a lâmpada âmbar -->
    <g class="antena">
      <path d="M130 52 Q125 36 136 24" fill="none" stroke="#9496f7" stroke-width="5" stroke-linecap="round" />
      <circle class="halo" cx="137" cy="20" r="19" fill="url(#{id}-brilho)" />
      <circle cx="137" cy="20" r="9" fill="url(#{id}-lampada)" />
      <circle cx="134" cy="17" r="2.6" fill="#fff" opacity="0.9" />
    </g>

    <!-- Corpo de vidro -->
    <path d={CORPO} fill="url(#{id}-corpo)" />
    <path d={CORPO} fill="url(#{id}-fundo)" />
    <path d={CORPO} fill="url(#{id}-luz)" />
    <path d="M72 98 Q80 66 114 58" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" opacity="0.78" />
    <circle cx="68" cy="114" r="3.5" fill="#fff" opacity="0.85" />
    <path d="M88 197 Q130 214 174 195" fill="none" stroke="#efe6ff" stroke-width="3" stroke-linecap="round" opacity="0.55" />

    <!-- Rosto: anda um pouco com o olhar, como se a cabeça virasse. -->
    <g class="rosto" style:transform={`translate(${desvio.x * 0.6}px, ${desvio.y * 0.6}px)`}>
      <g class="sobrancelhas">
        <path class="sob-e" d="M88 90 Q100 84 112 90" fill="none" stroke="#25225a" stroke-width="4" stroke-linecap="round" />
        <path class="sob-d" d="M148 90 Q160 84 172 90" fill="none" stroke="#25225a" stroke-width="4" stroke-linecap="round" />
      </g>

      {#each [-1, 1] as lado (lado)}
        {@const cx = 130 + lado * 31}
        {#if humor === "feliz"}
          <path d="M{cx - 14} 124 Q{cx} 104 {cx + 14} 124" fill="none" stroke="#15133c" stroke-width="7" stroke-linecap="round" />
        {:else if humor === "dormindo"}
          <path d="M{cx - 13} 120 Q{cx} 129 {cx + 13} 120" fill="none" stroke="#15133c" stroke-width="5" stroke-linecap="round" />
        {:else}
          <g class="olho">
            <ellipse {cx} cy="118" rx="16" ry="21" fill="url(#{id}-olho)" />
            <ellipse {cx} cy="118" rx="16" ry="21" fill="url(#{id}-iris)" />
            <g class="pupila" style:transform={`translate(${desvio.x}px, ${desvio.y}px)`}>
              <ellipse cx={cx - 5} cy="110" rx="5.5" ry="6.5" fill="#fff" />
              <circle cx={cx + 6} cy="126" r="2.6" fill="#fff" opacity="0.85" />
            </g>
          </g>
        {/if}
      {/each}

      <ellipse class="bochecha" cx="86" cy="145" rx="13" ry="7" fill="#ff8fb8" filter="url(#{id}-suave)" />
      <ellipse class="bochecha" cx="174" cy="145" rx="13" ry="7" fill="#ff8fb8" filter="url(#{id}-suave)" />

      {#if humor === "falando"}
        <ellipse class="boca-falando" cx="130" cy="154" rx="10" ry="7.5" fill="#2a1240" />
      {:else if humor === "surpreso"}
        <ellipse cx="130" cy="157" rx="7" ry="9" fill="#2a1240" />
      {:else if humor === "feliz"}
        <path d={boca} fill="#2a1240" stroke="#2a1240" stroke-width="3" stroke-linejoin="round" />
      {:else}
        <path d={boca} fill="none" stroke="#2a1240" stroke-width="4.5" stroke-linecap="round" />
      {/if}

      <path class="suor" d="M186 84 Q194 98 186 104 Q178 98 186 84 Z" fill="#c4e8ff" stroke="#fff" stroke-width="1.5" />
    </g>

    <!-- Headset: desce quando o jogo abre. -->
    <g class="headset">
      <path d="M50 124 Q50 38 130 38 Q210 38 210 124" fill="none" stroke="url(#{id}-headset)" stroke-width="11" stroke-linecap="round" />
      <rect x="33" y="106" width="27" height="44" rx="12" fill="url(#{id}-headset)" />
      <rect x="200" y="106" width="27" height="44" rx="12" fill="url(#{id}-headset)" />
      <rect x="38" y="113" width="6" height="30" rx="3" fill="#8b9bff" />
      <rect x="216" y="113" width="6" height="30" rx="3" fill="#8b9bff" />
      <path d="M48 147 Q60 180 98 173" fill="none" stroke="#1c1f3a" stroke-width="5" stroke-linecap="round" />
      <circle cx="100" cy="172" r="6" fill="#ffc163" />
    </g>

    <!-- Controle: nas mãos quando está jogando. -->
    <g class="controle">
      <path
        d="M98 186 h64 a16 16 0 0 1 16 16 v2 a12 12 0 0 1 -21 8 l-7 -7 h-40 l-7 7 a12 12 0 0 1 -21 -8 v-2 a16 16 0 0 1 16 -16 z"
        fill="url(#{id}-headset)"
      />
      <path d="M104 198 h10 M109 193 v10" stroke="#8b9bff" stroke-width="3" stroke-linecap="round" />
      <circle cx="150" cy="195" r="3" fill="#ffc163" />
      <circle cx="158" cy="202" r="3" fill="#ff8fb8" />
    </g>

    <g class="zzz" fill="#d6dcff" font-family="Jakarta, sans-serif" font-weight="700">
      <text x="186" y="70" font-size="18">z</text>
      <text x="200" y="52" font-size="13">z</text>
    </g>
  </g>

  <!-- Mãos: fora do corpo, cada uma com seu ritmo. -->
  <g class="mao esquerda">
    <circle cx="38" cy="168" r="15" fill="url(#{id}-corpo)" />
    <circle cx="38" cy="168" r="15" fill="url(#{id}-luz)" />
  </g>
  <g class="mao direita">
    <circle cx="222" cy="168" r="15" fill="url(#{id}-corpo)" />
    <circle cx="222" cy="168" r="15" fill="url(#{id}-luz)" />
  </g>
</svg>

<style>
  .oti {
    display: block;
    overflow: visible;
  }

  .flutua {
    animation: flutuar 4.2s ease-in-out infinite;
  }
  .sombra {
    transform-box: fill-box;
    transform-origin: center;
    animation: sombra 4.2s ease-in-out infinite;
  }
  @keyframes flutuar {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-8px);
    }
  }
  @keyframes sombra {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.3;
    }
    50% {
      transform: scale(0.85);
      opacity: 0.18;
    }
  }

  .rosto,
  .pupila {
    transition: transform 240ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .olho {
    transform-box: fill-box;
    transform-origin: center;
    animation: piscar 5.5s infinite;
  }
  @keyframes piscar {
    0%,
    94%,
    100% {
      transform: scaleY(1);
    }
    96.5% {
      transform: scaleY(0.08);
    }
  }
  [data-humor="surpreso"] .olho {
    animation: none;
    transform: scale(1.15);
  }
  [data-humor="jogando"] .olho {
    animation: none;
    transform: scaleY(0.8);
  }

  .bochecha {
    opacity: 0.32;
    transition: opacity 240ms ease;
  }
  [data-humor="feliz"] .bochecha,
  [data-humor="falando"] .bochecha {
    opacity: 0.6;
  }

  .sobrancelhas {
    opacity: 0;
    transition: opacity 200ms ease;
  }
  [data-humor="preocupado"] .sobrancelhas,
  [data-humor="pensando"] .sobrancelhas {
    opacity: 1;
  }
  .sob-e,
  .sob-d {
    transform-box: fill-box;
    transition: transform 200ms ease;
  }
  [data-humor="preocupado"] .sob-e {
    transform-origin: right center;
    transform: rotate(-12deg);
  }
  [data-humor="preocupado"] .sob-d {
    transform-origin: left center;
    transform: rotate(12deg);
  }
  [data-humor="pensando"] .sob-d {
    transform: translateY(-4px);
  }

  .boca-falando {
    transform-box: fill-box;
    transform-origin: center;
    animation: falar 240ms ease-in-out infinite alternate;
  }
  @keyframes falar {
    from {
      transform: scaleY(0.3) scaleX(1.1);
    }
    to {
      transform: scaleY(1);
    }
  }

  .suor {
    opacity: 0;
  }
  [data-humor="preocupado"] .suor {
    opacity: 1;
    animation: suar 1.6s ease-in infinite;
  }
  @keyframes suar {
    from {
      transform: translateY(0);
      opacity: 1;
    }
    to {
      transform: translateY(10px);
      opacity: 0;
    }
  }

  .halo {
    transform-box: fill-box;
    transform-origin: center;
    animation: halo 3s ease-in-out infinite;
  }
  [data-humor="pensando"] .halo {
    animation-duration: 0.7s;
  }
  [data-humor="dormindo"] .halo {
    opacity: 0.15;
    animation: none;
  }
  @keyframes halo {
    0%,
    100% {
      opacity: 0.45;
      transform: scale(0.85);
    }
    50% {
      opacity: 1;
      transform: scale(1.15);
    }
  }
  .antena {
    transform-origin: 130px 52px;
    animation: balancar 4.2s ease-in-out infinite;
  }
  @keyframes balancar {
    0%,
    100% {
      transform: rotate(-4deg);
    }
    50% {
      transform: rotate(5deg);
    }
  }

  .headset {
    opacity: 0;
    transform: translateY(-46px);
    transition:
      opacity 280ms ease,
      transform 520ms cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  [data-humor="jogando"] .headset {
    opacity: 1;
    transform: none;
  }

  .zzz {
    opacity: 0;
  }
  [data-humor="dormindo"] .zzz {
    opacity: 1;
    animation: zzz 2.4s ease-in-out infinite alternate;
  }
  @keyframes zzz {
    from {
      transform: translate(0, 4px);
      opacity: 0.25;
    }
    to {
      transform: translate(6px, -6px);
      opacity: 1;
    }
  }

  .mao {
    transition: transform 420ms cubic-bezier(0.3, 1.5, 0.5, 1);
  }
  .mao.esquerda {
    animation: mao 4.2s ease-in-out infinite;
  }
  .mao.direita {
    animation: mao 4.2s ease-in-out -0.6s infinite;
  }
  @keyframes mao {
    0%,
    100% {
      translate: 0 0;
    }
    50% {
      translate: 0 -11px;
    }
  }
  [data-gesto="acenar"] .mao.direita {
    transform-origin: 222px 168px;
    animation: acenar 0.55s ease-in-out infinite alternate;
  }
  @keyframes acenar {
    from {
      transform: translate(-6px, -78px) rotate(-14deg);
    }
    to {
      transform: translate(8px, -84px) rotate(14deg);
    }
  }
  [data-gesto="apontar"] .mao.direita {
    animation: apontar 0.9s ease-in-out infinite alternate;
  }
  @keyframes apontar {
    from {
      transform: translate(10px, 18px);
    }
    to {
      transform: translate(18px, 26px);
    }
  }
  [data-gesto="comemorar"] .mao.esquerda {
    animation: comemorar 0.32s ease-in-out infinite alternate;
  }
  [data-gesto="comemorar"] .mao.direita {
    animation: comemorar 0.32s ease-in-out 0.1s infinite alternate;
  }
  @keyframes comemorar {
    from {
      transform: translate(0, -78px);
    }
    to {
      transform: translate(0, -96px);
    }
  }
  .controle {
    opacity: 0;
    transform: translateY(16px);
    transition:
      opacity 240ms ease,
      transform 420ms cubic-bezier(0.3, 1.5, 0.5, 1);
  }
  [data-humor="jogando"] .controle {
    opacity: 1;
    transform: none;
  }
  /* Jogando: as mãos seguram o controle, com os polegares mexendo. */
  [data-humor="jogando"] .mao {
    transform-box: fill-box;
    transform-origin: center;
  }
  [data-humor="jogando"] .mao.esquerda {
    transform: translate(56px, 34px) scale(0.75);
    animation: polegar 0.5s ease-in-out infinite alternate;
  }
  [data-humor="jogando"] .mao.direita {
    transform: translate(-56px, 34px) scale(0.75);
    animation: polegar 0.5s ease-in-out 0.25s infinite alternate;
  }
  @keyframes polegar {
    from {
      translate: 0 0;
    }
    to {
      translate: 0 -3px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .oti * {
      animation: none !important;
    }
  }
</style>
