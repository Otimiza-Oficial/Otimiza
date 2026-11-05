<script lang="ts">
  // O mascote em vetor, enquanto a versão em Rive (com a máquina de estados do plano, 3.1) não chega. Os olhos são a
  // expressão inteira. Com jogo aberto ele para: nada de seguir o cursor nem piscar, para não gastar quadro.
  import { onMount } from "svelte";
  import type { EstadoDoMascote as Estado } from "../lib/clima";

  let { estado = "atento", tamanho = 220 }: { estado?: Estado; tamanho?: number } = $props();

  let olhar = $state({ x: 0, y: 0 });
  let raiz: SVGSVGElement | undefined = $state();

  const segueCursor = $derived(estado === "atento");

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
        const alcance = Math.min(1, d / 400);
        olhar = { x: (dx / d) * 7 * alcance, y: (dy / d) * 5 * alcance };
      });
    };

    window.addEventListener("pointermove", mover, { passive: true });
    return () => {
      window.removeEventListener("pointermove", mover);
      cancelAnimationFrame(quadro);
    };
  });

  const deslocamento = $derived(
    estado === "atento" ? olhar : estado === "preocupado" ? { x: 0, y: 6 } : { x: 0, y: 0 }
  );
</script>

<svg
  bind:this={raiz}
  class="mascote"
  data-estado={estado}
  width={tamanho}
  height={tamanho * 0.82}
  viewBox="0 0 220 180"
  role="img"
  aria-label="Mascote do Otimiza"
>
  <defs>
    <radialGradient id="m-corpo" cx="38%" cy="30%" r="75%">
      <stop offset="0%" stop-color="#ffffff" stop-opacity="0.95" />
      <stop offset="45%" stop-color="#e3ebfd" stop-opacity="0.85" />
      <stop offset="85%" stop-color="#b9c8f6" stop-opacity="0.8" />
      <stop offset="100%" stop-color="#9fb2f0" stop-opacity="0.9" />
    </radialGradient>
    <linearGradient id="m-borda" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#ffe2c2" />
      <stop offset="50%" stop-color="#a9b8f2" />
      <stop offset="100%" stop-color="#7f97ea" />
    </linearGradient>
    <linearGradient id="m-orelha" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#ffffff" stop-opacity="0.9" />
      <stop offset="100%" stop-color="#b3c3f5" stop-opacity="0.75" />
    </linearGradient>
    <linearGradient id="m-olho" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#e8a24a" />
      <stop offset="55%" stop-color="#9a5418" />
      <stop offset="100%" stop-color="#4a2a10" />
    </linearGradient>
  </defs>

  <g class="flutua">
    <path d="M44 62 Q0 91 44 120 Z" fill="url(#m-orelha)" stroke="url(#m-borda)" stroke-width="1.5" />
    <path d="M176 62 Q220 91 176 120 Z" fill="url(#m-orelha)" stroke="url(#m-borda)" stroke-width="1.5" />

    <circle class="aura" cx="110" cy="91" r="80" fill="none" stroke="#e39a3c" stroke-width="6" opacity="0" />
    <circle cx="110" cy="91" r="76" fill="url(#m-corpo)" stroke="url(#m-borda)" stroke-width="3" />
    <circle cx="110" cy="91" r="70" fill="none" stroke="#ffffff" stroke-opacity="0.55" stroke-width="1.2" />
    <ellipse cx="80" cy="46" rx="26" ry="11" fill="#ffffff" opacity="0.55" transform="rotate(-24 80 46)" />

    <g class="olhos" style:transform={`translate(${deslocamento.x}px, ${deslocamento.y}px)`}>
      <g class="olho">
        <rect x="83" y="72" width="17" height="34" rx="8.5" fill="url(#m-olho)" />
        <rect x="87" y="76" width="4" height="10" rx="2" fill="#ffffff" opacity="0.7" />
      </g>
      <g class="olho">
        <rect x="120" y="72" width="17" height="34" rx="8.5" fill="url(#m-olho)" />
        <rect x="124" y="76" width="4" height="10" rx="2" fill="#ffffff" opacity="0.7" />
      </g>
    </g>
  </g>
</svg>

<style>
  .mascote {
    display: block;
    overflow: visible;
    filter: drop-shadow(0 18px 30px rgb(47 95 168 / 0.25));
  }

  .olhos {
    transition: transform 220ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .olho {
    transform-box: fill-box;
    transform-origin: center;
    animation: piscar 6s infinite;
  }

  .flutua {
    animation: respirar 5s ease-in-out infinite;
    transform-origin: 110px 91px;
  }

  [data-estado="descansando"] .olho {
    animation: none;
    transform: scaleY(0.35);
  }

  [data-estado="focado"] .olho {
    animation: none;
    transform: scaleY(0.75);
  }

  [data-estado="focado"] .flutua {
    animation: none;
  }

  /* Preocupado: um halo âmbar suave por fora, sem trocar a cor do vidro. */
  [data-estado="preocupado"] .aura {
    opacity: 0.35;
  }

  @keyframes piscar {
    0%,
    94%,
    100% {
      transform: scaleY(1);
    }
    97% {
      transform: scaleY(0.1);
    }
  }

  @keyframes respirar {
    0%,
    100% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.02);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .olho,
    .flutua {
      animation: none;
    }
  }

  :global(.jogo-aberto) .olho,
  :global(.jogo-aberto) .flutua,
  :global(.jogo-aberto) .olhos {
    animation: none;
    transition: none;
  }
</style>
