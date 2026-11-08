<script lang="ts" module>
  /** O que o Oti está sentindo: só os olhos mudam. */
  export type Humor =
    | "atento"
    | "falando"
    | "feliz"
    | "pensando"
    | "preocupado"
    | "surpreso"
    | "jogando"
    | "dormindo";

  /** Mantido para quem já chamava com gesto: o Oti novo não tem mãos, o gesto vira um balanço da cabeça. */
  export type Gesto = "nenhum" | "acenar" | "apontar" | "comemorar";
</script>

<script lang="ts">
  // O Oti: um disco branco liso, de cabeça meio de lado, com dois olhos em cápsula inclinados, cavados com um
  // chanfro claro na borda (referência do dono). Não tem boca nem mãos: os olhos são a expressão inteira. Eles seguem o
  // cursor, piscam, e cada humor muda o desenho deles. Com jogo aberto a casca congela as animações (`jogo-aberto`).
  import { onMount } from "svelte";

  let {
    humor = "atento",
    gesto = "nenhum",
    tamanho = 200,
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
        const dy = ultimo.clientY - (r.top + r.height / 2);
        const d = Math.hypot(dx, dy) || 1;
        const k = Math.min(1, d / 500);
        olhar = { x: (dx / d) * 9 * k, y: (dy / d) * 7 * k };
      });
    };
    window.addEventListener("pointermove", mover, { passive: true });
    return () => {
      window.removeEventListener("pointermove", mover);
      cancelAnimationFrame(quadro);
    };
  });

  const desvio = $derived(
    humor === "pensando"
      ? { x: 6, y: -8 }
      : humor === "preocupado"
        ? { x: -2, y: 5 }
        : humor === "jogando"
          ? { x: 2, y: 2 }
          : segueCursor
            ? olhar
            : { x: 0, y: 0 }
  );

  // Os dois olhos: centro e inclinação. A cabeça meio de lado põe o direito mais alto.
  const OLHOS = [
    { cx: 86, cy: 112, giro: -33 },
    { cx: 130, cy: 88, giro: -33 },
  ];
</script>

<svg
  bind:this={raiz}
  class="oti"
  data-humor={humor}
  data-gesto={gesto}
  width={tamanho}
  height={tamanho}
  viewBox="0 0 200 200"
  role="img"
  aria-label="Oti, o assistente do Otimiza"
>
  <defs>
    <radialGradient id="{id}-corpo" cx="36%" cy="30%" r="78%">
      <stop offset="0%" stop-color="#ffffff" />
      <stop offset="55%" stop-color="#f0f0f1" />
      <stop offset="85%" stop-color="#dcdcde" />
      <stop offset="100%" stop-color="#c9c9cc" />
    </radialGradient>
    <radialGradient id="{id}-sombra-interna" cx="72%" cy="82%" r="55%">
      <stop offset="0%" stop-color="#9a9aa0" stop-opacity="0.35" />
      <stop offset="100%" stop-color="#9a9aa0" stop-opacity="0" />
    </radialGradient>
    <linearGradient id="{id}-olho" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#050505" />
      <stop offset="100%" stop-color="#1c1c1e" />
    </linearGradient>
    <filter id="{id}-suave" x="-30%" y="-30%" width="160%" height="160%">
      <feGaussianBlur stdDeviation="4" />
    </filter>
    <filter id="{id}-chao" x="-40%" y="-40%" width="180%" height="180%">
      <feGaussianBlur stdDeviation="5" />
    </filter>
  </defs>

  <ellipse class="chao" cx="100" cy="196" rx="54" ry="5" fill="#000" opacity="0.18" filter="url(#{id}-chao)" />

  <g class="cabeca">
    <!-- Disco: branco liso, sombra por dentro embaixo e um aro fino de luz. -->
    <circle cx="100" cy="100" r="92" fill="url(#{id}-corpo)" />
    <circle cx="100" cy="100" r="92" fill="url(#{id}-sombra-interna)" />
    <ellipse cx="70" cy="52" rx="40" ry="20" fill="#fff" opacity="0.7" transform="rotate(-28 70 52)" filter="url(#{id}-suave)" />
    <circle cx="100" cy="100" r="91" fill="none" stroke="#fff" stroke-width="1.6" opacity="0.95" />
    <circle cx="100" cy="100" r="92.5" fill="none" stroke="#000" stroke-width="1" opacity="0.08" />

    <g class="olhos" style:transform={`translate(${desvio.x}px, ${desvio.y}px)`}>
      {#each OLHOS as o, i (i)}
        <g transform={`rotate(${o.giro} ${o.cx} ${o.cy})`}>
          <g class="olho" style:transform-origin={`${o.cx}px ${o.cy}px`}>
            {#if humor === "feliz"}
              <path
                d="M{o.cx - 13} {o.cy + 6} Q{o.cx} {o.cy - 14} {o.cx + 13} {o.cy + 6}"
                fill="none"
                stroke="#0a0a0b"
                stroke-width="9"
                stroke-linecap="round"
              />
            {:else if humor === "dormindo"}
              <path d="M{o.cx - 13} {o.cy} L{o.cx + 13} {o.cy}" stroke="#0a0a0b" stroke-width="7" stroke-linecap="round" />
            {:else}
              <!-- Chanfro: um aro branco por fora e um fio escuro bem fino, como um furo no disco. -->
              <rect x={o.cx - 17} y={o.cy - 30} width="34" height="60" rx="17" fill="#fff" />
              <rect x={o.cx - 17} y={o.cy - 30} width="34" height="60" rx="17" fill="none" stroke="#000" stroke-opacity="0.1" stroke-width="1" />
              <rect x={o.cx - 13} y={o.cy - 26} width="26" height="52" rx="13" fill="url(#{id}-olho)" />
            {/if}
          </g>
        </g>
      {/each}
    </g>
  </g>
</svg>

<style>
  .oti {
    display: block;
    overflow: visible;
  }

  .cabeca {
    transform-origin: 100px 100px;
    animation: flutuar 5s ease-in-out infinite;
    transition: rotate 400ms cubic-bezier(0.3, 1.4, 0.5, 1);
  }
  .chao {
    transform-box: fill-box;
    transform-origin: center;
    animation: chao 5s ease-in-out infinite;
  }
  @keyframes flutuar {
    0%,
    100% {
      translate: 0 0;
    }
    50% {
      translate: 0 -5px;
    }
  }
  @keyframes chao {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.18;
    }
    50% {
      transform: scale(0.88);
      opacity: 0.1;
    }
  }

  .olhos {
    transition: transform 260ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  /* Piscar: as duas cápsulas achatam no próprio eixo. */
  .olho {
    animation: piscar 5.5s infinite;
    transition: scale 220ms ease;
  }
  @keyframes piscar {
    0%,
    93%,
    100% {
      scale: 1 1;
    }
    95.5% {
      scale: 1 0.1;
    }
  }

  [data-humor="pensando"] .cabeca {
    rotate: -8deg;
  }
  [data-humor="preocupado"] .cabeca {
    rotate: 6deg;
  }
  [data-humor="preocupado"] .olho {
    animation: none;
    scale: 0.9 0.72;
  }
  [data-humor="surpreso"] .olho {
    animation: none;
    scale: 1.14 1.12;
  }
  [data-humor="jogando"] .olho {
    animation: none;
    scale: 1 0.62;
  }
  [data-humor="falando"] .olho {
    animation: falar 0.5s ease-in-out infinite alternate;
  }
  @keyframes falar {
    from {
      scale: 1 1;
    }
    to {
      scale: 1 0.86;
    }
  }
  [data-humor="feliz"] .cabeca,
  [data-gesto="comemorar"] .cabeca,
  [data-gesto="acenar"] .cabeca {
    animation: pulinho 0.6s cubic-bezier(0.3, 1.4, 0.5, 1) 2;
  }
  @keyframes pulinho {
    0%,
    100% {
      translate: 0 0;
      rotate: 0deg;
    }
    50% {
      translate: 0 -10px;
      rotate: -6deg;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .oti * {
      animation: none !important;
    }
  }
</style>
