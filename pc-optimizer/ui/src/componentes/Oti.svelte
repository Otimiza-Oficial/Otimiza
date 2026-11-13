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
  import type { Acessorio } from "../lib/temporadas";

  let {
    humor = "atento",
    gesto = "nenhum",
    tamanho = 200,
    segueCursor = true,
    acessorio = null,
  }: {
    humor?: Humor;
    gesto?: Gesto;
    tamanho?: number;
    segueCursor?: boolean;
    /** O enfeite da época (lib/temporadas.ts): gorro no Natal, orelhas na Páscoa… */
    acessorio?: Acessorio | null;
  } = $props();

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

    {#if acessorio === "mascara"}
      <!-- Carnaval: a máscara ao longo da linha dos olhos, com plumas. -->
      <g class="acessorio">
        <path d="M54 136 L162 64" stroke="#7a2bd6" stroke-width="64" stroke-linecap="round" />
        <path d="M50 142 L166 60" stroke="#f4c430" stroke-width="4" stroke-dasharray="2 9" stroke-linecap="round" opacity="0.9" />
        <path d="M164 60 Q190 20 176 -6 Q196 28 182 62 Z" fill="#ff4f9a" />
        <path d="M170 64 Q206 40 212 8 Q214 46 184 70 Z" fill="#2ec4b6" />
      </g>
    {/if}

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

    {#if acessorio && acessorio !== "mascara"}
      <g class="acessorio" transform="rotate(-10 100 100)">
        {#if acessorio === "gorro"}
          <!-- Natal: gorro vermelho com a ponta caída e o pompom. -->
          <path d="M22 78 Q34 2 112 -6 Q170 -10 202 44 Q172 26 152 38 Q122 28 22 78 Z" fill="#d62f3a" />
          <path d="M52 40 Q96 6 150 6" stroke="#fff" stroke-width="5" stroke-linecap="round" opacity="0.25" fill="none" />
          <path d="M18 80 Q100 30 176 52" stroke="#ffffff" stroke-width="22" stroke-linecap="round" fill="none" />
          <circle cx="204" cy="48" r="14" fill="#ffffff" />
        {:else if acessorio === "cartola"}
          <!-- Ano Novo: cartola com a fita dourada. -->
          <ellipse cx="104" cy="22" rx="62" ry="12" fill="#1d1d1f" />
          <rect x="70" y="-52" width="68" height="74" rx="8" fill="#1d1d1f" />
          <rect x="70" y="4" width="68" height="12" fill="#e6b422" />
          <rect x="78" y="-46" width="8" height="48" rx="4" fill="#fff" opacity="0.15" />
        {:else if acessorio === "orelhas"}
          <!-- Páscoa: orelhas de coelho. -->
          <g>
            <ellipse cx="72" cy="-14" rx="19" ry="50" transform="rotate(-14 72 -14)" fill="#f7f7f8" stroke="#d9d9de" stroke-width="2" />
            <ellipse cx="72" cy="-10" rx="9" ry="36" transform="rotate(-14 72 -10)" fill="#ffb6c8" />
            <ellipse cx="128" cy="-18" rx="19" ry="50" transform="rotate(12 128 -18)" fill="#f7f7f8" stroke="#d9d9de" stroke-width="2" />
            <ellipse cx="128" cy="-14" rx="9" ry="36" transform="rotate(12 128 -14)" fill="#ffb6c8" />
          </g>
        {:else if acessorio === "chapeu-de-palha"}
          <!-- Festa Junina: chapéu de palha com remendo. -->
          <ellipse cx="100" cy="30" rx="90" ry="18" fill="#e3b85c" />
          <path d="M52 30 Q58 -28 100 -30 Q142 -28 148 30 Z" fill="#edc773" />
          <path d="M54 22 Q100 8 146 22" stroke="#c0392b" stroke-width="10" fill="none" />
          <path d="M60 0 L80 -2 M70 -14 L92 -16 M110 -16 L132 -12 M116 2 L138 4" stroke="#b8893a" stroke-width="3" stroke-linecap="round" />
          <rect x="112" y="-14" width="18" height="16" fill="#3b7dd8" transform="rotate(8 121 -6)" />
        {:else if acessorio === "bone"}
          <!-- Dia das Crianças: boné colorido de aba para o lado. -->
          <path d="M34 60 Q40 -6 104 -8 Q164 -6 166 56 Z" fill="#3b82f6" />
          <path d="M100 -8 Q104 26 100 56" stroke="#facc15" stroke-width="10" fill="none" />
          <ellipse cx="190" cy="56" rx="36" ry="10" fill="#ef4444" transform="rotate(10 190 56)" />
          <circle cx="104" cy="-8" r="7" fill="#ef4444" />
        {:else if acessorio === "chapeu-de-bruxa"}
          <!-- Halloween: chapéu de bruxa com a ponta torta. -->
          <ellipse cx="100" cy="30" rx="86" ry="15" fill="#2b2240" />
          <path d="M56 28 Q80 -30 118 -58 Q150 -84 176 -70 Q140 -60 136 -20 Q132 6 146 28 Z" fill="#3a2d57" />
          <path d="M60 18 Q100 2 142 16" stroke="#f97316" stroke-width="11" fill="none" />
          <rect x="92" y="4" width="14" height="14" rx="2" fill="none" stroke="#facc15" stroke-width="3" />
        {/if}
      </g>
    {/if}
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
    transition: rotate var(--mov-entrar) var(--curva-mola);
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
