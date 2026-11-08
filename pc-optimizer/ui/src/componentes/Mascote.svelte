<script lang="ts">
  // O mascote em vetor, fiel à logo (`brand/nova-geracao/mascote-logo.png`), enquanto a versão em Rive com a máquina
  // de estados do plano (3.1) não chega. É uma bolha de VIDRO: quase sem preenchimento, o céu aparece através dela; o
  // que dá corpo é a borda grossa, os reflexos e as orelhas de vidro. Os olhos são a expressão inteira. Com jogo aberto
  // ele para: nada de seguir o cursor nem piscar, para não gastar quadro.
  import { onMount } from "svelte";
  import type { EstadoDoMascote as Estado } from "../lib/clima";

  let { estado = "atento", tamanho = 220 }: { estado?: Estado; tamanho?: number } = $props();

  // Ids únicos: dois mascotes na mesma página (menu e início) não podem disputar o mesmo gradiente.
  const id = `m${Math.random().toString(36).slice(2, 8)}`;

  let olhar = $state({ x: 0, y: 0 });
  let raiz: SVGSVGElement | undefined = $state();

  const segueCursor = $derived(estado === "atento" && tamanho >= 64);

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
        const alcance = Math.min(1, d / 500);
        olhar = { x: (dx / d) * 6 * alcance, y: (dy / d) * 4 * alcance };
      });
    };

    window.addEventListener("pointermove", mover, { passive: true });
    return () => {
      window.removeEventListener("pointermove", mover);
      cancelAnimationFrame(quadro);
    };
  });

  const deslocamento = $derived(
    estado === "atento" ? olhar : estado === "preocupado" ? { x: 0, y: 5 } : { x: 0, y: 0 }
  );

  // Geometria tirada da logo: cabeça de raio 74 em (120, 96). Olhos a ±0,36 r do centro, um pouco acima do meio,
  // com 0,24 r × 0,42 r. Orelhas saem 0,23 r de cada lado, na altura do centro.
  const R = 74;
  const CX = 120;
  const CY = 96;
</script>

<svg
  bind:this={raiz}
  class="mascote"
  data-estado={estado}
  width={tamanho}
  height={tamanho * (192 / 240)}
  viewBox="0 0 240 192"
  role="img"
  aria-label="Mascote do Otimiza"
>
  <defs>
    <!-- Corpo: quase transparente, um pouco mais denso na borda, como uma bolha de verdade. -->
    <radialGradient id="{id}-corpo" cx="50%" cy="46%" r="52%">
      <stop offset="0%" stop-color="#f4f6ff" stop-opacity="0.18" />
      <stop offset="62%" stop-color="#e3e8ff" stop-opacity="0.26" />
      <stop offset="88%" stop-color="#c3cdfa" stop-opacity="0.42" />
      <stop offset="100%" stop-color="#aab6f3" stop-opacity="0.6" />
    </radialGradient>
    <!-- Iridescência: pêssego em cima à esquerda, lavanda embaixo à direita. -->
    <radialGradient id="{id}-pessego" cx="30%" cy="22%" r="55%">
      <stop offset="0%" stop-color="#ffe2c8" stop-opacity="0.75" />
      <stop offset="100%" stop-color="#ffe2c8" stop-opacity="0" />
    </radialGradient>
    <radialGradient id="{id}-lavanda" cx="72%" cy="86%" r="60%">
      <stop offset="0%" stop-color="#b7a9f5" stop-opacity="0.5" />
      <stop offset="100%" stop-color="#b7a9f5" stop-opacity="0" />
    </radialGradient>
    <!-- Borda grossa de vidro: clara em cima, lavanda nos lados, mais funda embaixo. -->
    <linearGradient id="{id}-aro" x1="0.2" y1="0" x2="0.8" y2="1">
      <stop offset="0%" stop-color="#f3f1ff" />
      <stop offset="35%" stop-color="#b9c3f6" />
      <stop offset="70%" stop-color="#9aa8ef" />
      <stop offset="100%" stop-color="#c2b6f2" />
    </linearGradient>
    <linearGradient id="{id}-orelha" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#f4f5ff" stop-opacity="0.9" />
      <stop offset="55%" stop-color="#cdd4fa" stop-opacity="0.7" />
      <stop offset="100%" stop-color="#a6b2f1" stop-opacity="0.9" />
    </linearGradient>
    <!-- Olho: âmbar em cima, marrom-escuro embaixo. -->
    <linearGradient id="{id}-olho" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#f0a443" />
      <stop offset="38%" stop-color="#b8681f" />
      <stop offset="72%" stop-color="#5a3014" />
      <stop offset="100%" stop-color="#2b170b" />
    </linearGradient>
    <radialGradient id="{id}-reflexo-olho" cx="50%" cy="104%" r="34%">
      <stop offset="0%" stop-color="#a9c0ff" stop-opacity="0.9" />
      <stop offset="100%" stop-color="#9fb8ff" stop-opacity="0" />
    </radialGradient>
    <clipPath id="{id}-cabeca">
      <circle cx={CX} cy={CY} r={R} />
    </clipPath>
    <filter id="{id}-aura" x="-30%" y="-30%" width="160%" height="160%">
      <feGaussianBlur stdDeviation="7" />
    </filter>
    <filter id="{id}-suave" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="1.6" />
    </filter>
  </defs>

  <g class="flutua">
    <!-- Orelhas: gotas de vidro presas à cabeça, desenhadas atrás dela. -->
    {#each [-1, 1] as lado (lado)}
      {@const x = CX + lado * (R - 3)}
      <g>
        <path
          d="M {x} {CY - 30} C {x + lado * 28} {CY - 26}, {x + lado * 29} {CY + 28}, {x} {CY + 33} Z"
          fill="url(#{id}-orelha)"
          stroke="#9eabee"
          stroke-width="2"
          stroke-linejoin="round"
        />
        <path
          d="M {x + lado * 6} {CY - 20} C {x + lado * 19} {CY - 15}, {x + lado * 20} {CY + 2}, {x + lado * 15} {CY + 12}"
          fill="none"
          stroke="#ffffff"
          stroke-opacity="0.75"
          stroke-width="2"
          stroke-linecap="round"
        />
      </g>
    {/each}

    <!-- Halo âmbar do "preocupado": por fora, sem trocar a cor do vidro. -->
    <circle class="aura" cx={CX} cy={CY} r={R + 4} fill="none" stroke="#f0a443" stroke-width="10" opacity="0" filter="url(#{id}-aura)" />

    <!-- Cabeça -->
    <circle cx={CX} cy={CY} r={R} fill="url(#{id}-corpo)" />
    <g clip-path="url(#{id}-cabeca)">
      <circle cx={CX} cy={CY} r={R} fill="url(#{id}-pessego)" />
      <circle cx={CX} cy={CY} r={R} fill="url(#{id}-lavanda)" />
      <!-- Luz que volta do fundo da bolha. -->
      <ellipse cx={CX + 6} cy={CY + 58} rx="46" ry="16" fill="#ffffff" opacity="0.32" filter="url(#{id}-suave)" />
    </g>

    <!-- Aro: a espessura do vidro. -->
    <circle cx={CX} cy={CY} r={R - 3.5} fill="none" stroke="url(#{id}-aro)" stroke-width="7" />
    <circle cx={CX} cy={CY} r={R - 7.5} fill="none" stroke="#ffffff" stroke-opacity="0.55" stroke-width="1" />
    <circle cx={CX} cy={CY} r={R} fill="none" stroke="#8796e6" stroke-opacity="0.55" stroke-width="1" />

    <!-- Brilho especular: arco em cima à esquerda e um ponto. -->
    <path
      d="M {CX - 52} {CY - 30} A {R - 12} {R - 12} 0 0 1 {CX - 14} {CY - 60}"
      fill="none"
      stroke="#ffffff"
      stroke-opacity="0.85"
      stroke-width="3.2"
      stroke-linecap="round"
    />
    <circle cx={CX - 58} cy={CY - 16} r="2" fill="#ffffff" opacity="0.8" />
    <!-- Reflexo rosado embaixo à direita, no aro. -->
    <path
      d="M {CX + 40} {CY + 58} A {R - 4} {R - 4} 0 0 0 {CX + 62} {CY + 34}"
      fill="none"
      stroke="#f6c9e4"
      stroke-opacity="0.7"
      stroke-width="2.4"
      stroke-linecap="round"
    />

    <g class="olhos" style:transform={`translate(${deslocamento.x}px, ${deslocamento.y}px)`}>
      {#each [-1, 1] as lado (lado)}
        {@const ox = CX + lado * 27 - 9}
        <g class="olho">
          <rect x={ox} y={CY - 21} width="18" height="31" rx="9" fill="url(#{id}-olho)" />
          <rect x={ox} y={CY - 21} width="18" height="31" rx="9" fill="url(#{id}-reflexo-olho)" />
          <rect x={ox + 7.5} y={CY - 17} width="3" height="11" rx="1.5" fill="#ffffff" opacity="0.55" />
        </g>
      {/each}
    </g>
  </g>
</svg>

<style>
  .mascote {
    display: block;
    overflow: visible;
    filter: drop-shadow(0 14px 22px rgb(60 80 170 / 0.18));
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
    animation: respirar 6s ease-in-out infinite;
    transform-origin: 120px 96px;
  }

  [data-estado="descansando"] .olho {
    animation: none;
    transform: scaleY(0.3);
  }

  [data-estado="focado"] .olho {
    animation: none;
    transform: scaleY(0.72);
  }

  [data-estado="focado"] .flutua {
    animation: none;
  }

  [data-estado="preocupado"] .aura {
    opacity: 0.45;
  }

  @keyframes piscar {
    0%,
    95%,
    100% {
      transform: scaleY(1);
    }
    97.5% {
      transform: scaleY(0.08);
    }
  }

  @keyframes respirar {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-3px);
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
