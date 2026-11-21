<script lang="ts">
  // O fundo "Limpo" da marca v2: preto (papel no claro) com poeira de luz espalhada e um halo bem baixo no alto, à
  // direita, para onde o Oti olha. Estático e desenhado uma vez: com o jogo aberto não custa um quadro.
  import { estrela, sorteio } from "../../lib/acaso";

  const acaso = sorteio(2026);
  // Mais poeira no alto à direita (o lado do olhar), rareando para baixo e para a esquerda.
  const poeira = Array.from({ length: 180 }, () => {
    const x = Math.pow(acaso(), 0.7) * 1600;
    const y = Math.pow(acaso(), 1.4) * 1000;
    const perto = Math.max(0, 1 - Math.hypot(x - 1320, y - 160) / 900);
    return { x, y, r: 0.4 + acaso() * 0.9, o: 0.08 + perto * 0.45 * acaso() + acaso() * 0.12 };
  });
  const estrelas = Array.from({ length: 10 }, () => {
    const x = 900 + acaso() * 680;
    const y = 30 + acaso() * 420;
    return { d: estrela(x, y, 2.5 + acaso() * 4), o: 0.15 + acaso() * 0.35 };
  });
</script>

<svg class="ceu" viewBox="0 0 1600 1000" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
  <defs>
    <radialGradient id="ceu-halo" cx="0.83" cy="0.12" r="0.55">
      <stop offset="0" stop-color="currentColor" stop-opacity="0.09" />
      <stop offset="0.5" stop-color="currentColor" stop-opacity="0.025" />
      <stop offset="1" stop-color="currentColor" stop-opacity="0" />
    </radialGradient>
  </defs>
  <rect width="1600" height="1000" fill="url(#ceu-halo)" />
  <g fill="currentColor">
    {#each poeira as p, i (i)}<circle cx={p.x.toFixed(1)} cy={p.y.toFixed(1)} r={p.r.toFixed(2)} opacity={p.o.toFixed(2)} />{/each}
    {#each estrelas as e, i (i)}<path d={e.d} opacity={e.o.toFixed(2)} />{/each}
  </g>
</svg>

<style>
  .ceu {
    position: fixed;
    inset: 0;
    z-index: 0;
    width: 100%;
    height: 100%;
    color: var(--tinta);
    pointer-events: none;
  }
  /* No claro a poeira é carvão: mais baixa, para o papel continuar limpo. */
  :global(:root[data-tema="dia"]) .ceu {
    opacity: 0.55;
  }
</style>
