<script lang="ts">
  // O fundo inteiro. A cor segue a hora (ou o tema escolhido) e as nuvens seguem o diagnóstico: cada aviso traz nuvem.
  // Nuvem é gradiente radial, não `filter: blur`: o desfoque ao vivo em tela cheia custaria placa o tempo todo.
  import type { Clima, Hora, Tema } from "../lib/clima";

  let { clima, hora, tema }: { clima: Clima; hora: Hora; tema: Tema } = $props();

  const paleta = $derived(tema === "auto" ? hora : tema);

  // Posições fixas: o mesmo céu para o mesmo diagnóstico. As primeiras ficam baixas, como um horizonte; as de aviso
  // sobem pelo céu.
  const posicoes = [
    { x: -8, y: 72, l: 46, a: 0 },
    { x: 62, y: 76, l: 50, a: 8 },
    { x: 28, y: 84, l: 44, a: 3 },
    { x: 70, y: 18, l: 26, a: 12 },
    { x: 6, y: 26, l: 22, a: 6 },
    { x: 40, y: 8, l: 20, a: 15 },
    { x: 86, y: 44, l: 24, a: 9 },
  ];

  const quantas = $derived(clima === "limpo" ? 3 : clima === "nuvens" ? 5 : 7);
</script>

<div class="ceu" data-paleta={paleta} data-clima={clima} aria-hidden="true">
  {#each posicoes.slice(0, quantas) as n, i (i)}
    <div
      class="nuvem"
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
    --ceu-fundo: linear-gradient(180deg, #8fb3ec 0%, #b9d0f4 45%, #e6eefb 80%, #fbefe2 100%);
  }
  [data-paleta="tarde"] {
    --ceu-fundo: linear-gradient(180deg, #3f78d8 0%, #6d9ae4 40%, #a9c6f1 75%, #dce8fa 100%);
  }
  [data-paleta="por-do-sol"] {
    --ceu-fundo: linear-gradient(180deg, #5d79c9 0%, #9a9ad6 38%, #e3b2b3 70%, #f8d5b4 100%);
  }
  [data-paleta="lavanda"] {
    --ceu-fundo: linear-gradient(180deg, #a6b6ee 0%, #c4cdf4 45%, #e4e8fb 85%, #f2eefb 100%);
  }
  [data-paleta="noite"] {
    --ceu-fundo: linear-gradient(180deg, #0f1a3f 0%, #1a2a5e 55%, #2b3c78 100%);
  }

  [data-clima="nublado"] {
    filter: saturate(0.6) brightness(0.95);
  }

  /* Uma nuvem: copas redondas em cima, base achatada e um pouco mais escura embaixo. */
  .nuvem {
    position: absolute;
    aspect-ratio: 2.8;
    background:
      radial-gradient(closest-side at 32% 52%, rgb(255 255 255 / 0.95) 60%, transparent),
      radial-gradient(closest-side at 52% 36%, rgb(255 255 255 / 0.98) 62%, transparent),
      radial-gradient(closest-side at 70% 50%, rgb(255 255 255 / 0.95) 60%, transparent),
      radial-gradient(closest-side at 18% 66%, rgb(250 252 255 / 0.9) 55%, transparent),
      radial-gradient(closest-side at 84% 66%, rgb(250 252 255 / 0.9) 55%, transparent),
      radial-gradient(60% 30% at 50% 82%, rgb(196 210 236 / 0.85), transparent);
    opacity: 0.9;
    animation: deriva 120s linear infinite alternate;
    will-change: transform;
  }

  [data-paleta="noite"] .nuvem {
    opacity: 0.1;
  }

  [data-clima="nublado"] .nuvem {
    filter: grayscale(0.5) brightness(0.95);
  }

  @keyframes deriva {
    from {
      transform: translateX(-2vw);
    }
    to {
      transform: translateX(3vw);
    }
  }
</style>
