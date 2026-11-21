<script lang="ts">
  // As partículas da marca (o conceito da v2: o Oti olha para um ponto onde a luz se junta). Um núcleo de bolhas de
  // luz com brilho, poeira em volta e brilhos de quatro pontas, sempre na cor do Oti no tema (branco no escuro, carvão
  // no claro). Desenhado uma vez, com semente fixa: não se mexe, não custa nada com o jogo aberto.
  import { estrela, sino, sorteio } from "../../lib/acaso";

  let { tamanho = 160, semente = 7, densidade = 1 }: { tamanho?: number; semente?: number; densidade?: number } = $props();

  const id = `brilho${Math.random().toString(36).slice(2, 8)}`;
  const desenho = $derived.by(() => {
    const acaso = sorteio(semente);
    const nucleo = Array.from({ length: 8 }, (_, i) => ({
      x: 50 + sino(acaso) * (i < 3 ? 3 : 7),
      y: 50 + sino(acaso) * (i < 3 ? 3 : 7),
      r: i < 3 ? 4.2 - i * 0.7 : 1 + acaso() * 1.8,
    }));
    const poeira = Array.from({ length: Math.round(70 * densidade) }, () => ({
      x: 50 + sino(acaso) * 17,
      y: 50 + sino(acaso) * 15,
      r: 0.18 + acaso() * 0.4,
      o: 0.25 + acaso() * 0.7,
    }));
    const estrelas = Array.from({ length: Math.round(9 * densidade) }, () => {
      const x = 50 + sino(acaso) * 20;
      const y = 50 + sino(acaso) * 18;
      return { d: estrela(x, y, 0.9 + acaso() * 1.4), o: 0.45 + acaso() * 0.5 };
    });
    return { nucleo, poeira, estrelas };
  });
</script>

<svg class="brilho" width={tamanho} height={tamanho} viewBox="0 0 100 100" aria-hidden="true">
  <defs>
    <radialGradient id="{id}-halo">
      <stop offset="0" stop-color="currentColor" stop-opacity="0.28" />
      <stop offset="0.45" stop-color="currentColor" stop-opacity="0.07" />
      <stop offset="1" stop-color="currentColor" stop-opacity="0" />
    </radialGradient>
    <filter id="{id}-luz" x="-1" y="-1" width="3" height="3"><feGaussianBlur stdDeviation="1.6" /></filter>
  </defs>
  <circle cx="50" cy="50" r="40" fill="url(#{id}-halo)" />
  <g fill="currentColor" filter="url(#{id}-luz)" opacity="0.9">
    {#each desenho.nucleo as b, i (i)}<circle cx={b.x} cy={b.y} r={b.r} />{/each}
  </g>
  <g fill="currentColor">
    {#each desenho.nucleo as b, i (i)}<circle cx={b.x} cy={b.y} r={b.r} />{/each}
    {#each desenho.poeira as p, i (i)}<circle cx={p.x} cy={p.y} r={p.r} opacity={p.o} />{/each}
    {#each desenho.estrelas as e, i (i)}<path d={e.d} opacity={e.o} />{/each}
  </g>
</svg>

<style>
  .brilho {
    display: block;
    flex: none;
    overflow: visible;
    color: var(--oti-disco-tema);
    pointer-events: none;
  }
</style>
