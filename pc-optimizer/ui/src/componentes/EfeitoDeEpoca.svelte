<script lang="ts">
  // O enfeite de fundo da época: neve no Natal, fogos no Ano Novo, confete no Carnaval, ovinhos na Páscoa,
  // bandeirinhas na Festa Junina, balões no Dia das Crianças, morcegos no Halloween. Só CSS (nada de canvas nem laço de
  // quadro): 18 peças leves, atrás do vidro, sem receber clique. Com jogo aberto a regra global `.jogo-aberto *` para
  // tudo; com "menos movimento" pedido no Windows, as peças ficam paradas.
  import type { Efeito } from "../lib/temporadas";

  let { efeito }: { efeito: Efeito } = $props();

  // Posições fixas (sem sortear a cada abertura): o mesmo céu sempre, e nada de peça nascendo em cima do texto.
  const PECAS = Array.from({ length: 18 }, (_, i) => ({
    x: (i * 53) % 100,
    atraso: -((i * 37) % 23),
    duracao: 14 + ((i * 7) % 11),
    tamanho: 0.7 + ((i * 13) % 6) / 10,
    giro: (i * 47) % 360,
  }));

  const SIMBOLO: Record<Efeito, string[]> = {
    neve: ["❄", "❅", "•"],
    fogos: ["✦", "✧", "✶"],
    confete: ["▮", "●", "▲"],
    ovinhos: ["🥚", "🐣", "🌸"],
    bandeirinhas: ["▼"],
    baloes: ["🎈"],
    morcegos: ["🦇", "🎃", "🕸"],
  };

  const CORES: Record<Efeito, string[]> = {
    neve: ["#ffffff", "#eaf2ff", "#ffffff"],
    fogos: ["#f4c430", "#ffffff", "#ff8fb8"],
    confete: ["#7a2bd6", "#ff4f9a", "#2ec4b6", "#f4c430"],
    ovinhos: ["#ffffff"],
    bandeirinhas: ["#ef4444", "#3b82f6", "#facc15", "#22c55e", "#a855f7"],
    baloes: ["#ffffff"],
    morcegos: ["#ffffff"],
  };
</script>

<div class="epoca" data-efeito={efeito} aria-hidden="true">
  {#if efeito === "bandeirinhas"}
    <!-- Festa Junina: duas fileiras de bandeirinhas no alto, balançando de leve. -->
    {#each [0, 1] as fileira (fileira)}
      <div class="varal" style:top={`${4 + fileira * 7}%`} style:animation-delay={`${-fileira * 1.5}s`}>
        {#each Array.from({ length: 22 }, (_, i) => i) as i (i)}
          <span style:color={CORES.bandeirinhas[(i + fileira) % 5]}>▼</span>
        {/each}
      </div>
    {/each}
  {:else}
    {#each PECAS as p, i (i)}
      <span
        class="peca"
        style:left={`${p.x}%`}
        style:animation-delay={`${p.atraso}s`}
        style:animation-duration={`${p.duracao}s`}
        style:font-size={`${p.tamanho * 18}px`}
        style:color={CORES[efeito][i % CORES[efeito].length]}
        style:--giro={`${p.giro}deg`}
        style:--y={`${Math.round((p.giro / 360) * 90)}%`}
      >{SIMBOLO[efeito][i % SIMBOLO[efeito].length]}</span>
    {/each}
  {/if}
</div>

<style>
  /* Por cima do vidro (senão a neve some atrás dos painéis brancos), mas sem receber clique. */
  .epoca {
    position: fixed;
    inset: 0;
    z-index: 30;
    overflow: hidden;
    pointer-events: none;
  }
  .peca {
    position: absolute;
    top: -8%;
    opacity: 0.8;
    text-shadow:
      0 0 1px rgb(90 110 150 / 0.55),
      0 2px 6px rgb(40 60 100 / 0.2);
    animation-name: cair;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
    will-change: transform;
  }
  /* Balões e fogos sobem; o resto cai. */
  [data-efeito="baloes"] .peca,
  [data-efeito="fogos"] .peca {
    top: auto;
    bottom: -8%;
    animation-name: subir;
  }
  [data-efeito="morcegos"] .peca {
    animation-name: voar;
  }
  /* Emoji colorido (ovinhos, balões, morcegos) não precisa de contorno, e transparente fica apagado. */
  [data-efeito="morcegos"] .peca,
  [data-efeito="ovinhos"] .peca,
  [data-efeito="baloes"] .peca {
    opacity: 0.95;
    text-shadow: none;
  }
  @keyframes cair {
    from {
      transform: translate(0, 0) rotate(0deg);
    }
    to {
      transform: translate(40px, 112vh) rotate(var(--giro));
    }
  }
  @keyframes subir {
    from {
      transform: translate(0, 0);
    }
    to {
      transform: translate(-30px, -112vh);
    }
  }
  @keyframes voar {
    0% {
      transform: translate(0, 10vh);
    }
    50% {
      transform: translate(30px, 50vh) rotate(8deg);
    }
    100% {
      transform: translate(-20px, 112vh);
    }
  }
  .varal {
    position: absolute;
    left: -2%;
    right: -2%;
    display: flex;
    justify-content: space-between;
    font-size: 20px;
    border-top: 1px solid rgb(0 0 0 / 0.15);
    transform-origin: top center;
    animation: balancar 5s ease-in-out infinite alternate;
  }
  @keyframes balancar {
    from {
      transform: rotate(-0.6deg);
    }
    to {
      transform: rotate(0.6deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .peca,
    .varal {
      animation: none;
    }
    .peca {
      top: var(--y);
    }
  }
</style>
