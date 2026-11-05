<script lang="ts">
  // O fundo inteiro. A cor segue a hora (ou o tema escolhido) e as nuvens seguem o diagnóstico. Nuvem é gradiente
  // radial, não `filter: blur`: o desfoque ao vivo em tela cheia custaria placa o tempo todo.
  import type { Clima, Hora, Tema } from "../lib/clima";

  let { clima, hora, tema }: { clima: Clima; hora: Hora; tema: Tema } = $props();

  const paleta = $derived(tema === "auto" ? hora : tema);

  // Posições fixas: o mesmo céu para o mesmo diagnóstico, sem nuvem pulando de lugar a cada leitura.
  const posicoes = [
    { x: 8, y: 14, l: 34, a: 0 },
    { x: 58, y: 8, l: 28, a: 8 },
    { x: 30, y: 62, l: 40, a: 3 },
    { x: 74, y: 56, l: 30, a: 12 },
    { x: -6, y: 40, l: 26, a: 6 },
    { x: 46, y: 30, l: 22, a: 15 },
    { x: 84, y: 22, l: 24, a: 9 },
  ];

  const quantas = $derived(clima === "limpo" ? 1 : clima === "nuvens" ? 3 : 7);
</script>

<div class="ceu" data-paleta={paleta} data-clima={clima} aria-hidden="true">
  {#each posicoes.slice(0, quantas) as n, i (i)}
    <div
      class="nuvem"
      class:fiapo={clima === "limpo"}
      style:left={`${n.x}%`}
      style:top={`${n.y}%`}
      style:width={`${n.l}vw`}
      style:animation-delay={`-${n.a * 7}s`}
    ></div>
  {/each}
</div>

<style>
  .ceu {
    position: fixed;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    background: var(--ceu-fundo);
    transition: background 1.2s ease;
  }

  [data-paleta="manha"] {
    --ceu-fundo: linear-gradient(160deg, #ffe6cc 0%, #f3ecf6 30%, #cfe0fa 70%, #a9c4f3 100%);
  }
  [data-paleta="tarde"] {
    --ceu-fundo: linear-gradient(180deg, #4f80d6 0%, #79a2e6 35%, #b8d0f5 75%, #e3edfc 100%);
  }
  [data-paleta="por-do-sol"] {
    --ceu-fundo: linear-gradient(180deg, #6a7fd0 0%, #b59bd8 40%, #f3b7a0 75%, #ffd9b0 100%);
  }
  [data-paleta="lavanda"] {
    --ceu-fundo: linear-gradient(170deg, #c9c4f4 0%, #b4c3f3 45%, #dfe6fb 100%);
  }
  [data-paleta="noite"] {
    --ceu-fundo: radial-gradient(120% 80% at 70% 0%, #2a3a78 0%, #141d45 45%, #0b1130 100%);
  }

  [data-clima="nublado"] {
    filter: saturate(0.55) brightness(0.92);
  }

  .nuvem {
    position: absolute;
    aspect-ratio: 2.6;
    background:
      radial-gradient(closest-side at 30% 60%, rgb(255 255 255 / 0.9), transparent),
      radial-gradient(closest-side at 55% 40%, rgb(255 255 255 / 0.95), transparent),
      radial-gradient(closest-side at 75% 62%, rgb(255 255 255 / 0.85), transparent),
      radial-gradient(closest-side at 50% 75%, rgb(236 242 252 / 0.9), transparent 90%);
    opacity: 0.85;
    animation: deriva 90s linear infinite alternate;
    will-change: transform;
  }

  .nuvem.fiapo {
    opacity: 0.35;
  }

  [data-paleta="noite"] .nuvem {
    opacity: 0.18;
  }

  [data-clima="nublado"] .nuvem {
    opacity: 0.95;
    background:
      radial-gradient(closest-side at 30% 60%, rgb(226 231 240 / 0.95), transparent),
      radial-gradient(closest-side at 55% 40%, rgb(240 243 249 / 0.95), transparent),
      radial-gradient(closest-side at 75% 62%, rgb(214 221 233 / 0.9), transparent),
      radial-gradient(closest-side at 50% 75%, rgb(200 208 222 / 0.9), transparent 90%);
  }

  @keyframes deriva {
    from {
      transform: translateX(-3vw);
    }
    to {
      transform: translateX(4vw);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .nuvem {
      animation: none;
    }
  }

  :global(.jogo-aberto) .nuvem,
  :global(.jogo-aberto) .ceu {
    animation: none;
    transition: none;
  }
</style>
