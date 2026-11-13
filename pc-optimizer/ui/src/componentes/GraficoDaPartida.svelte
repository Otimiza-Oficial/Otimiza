<script lang="ts">
  // O gráfico da partida (Plano de refino 2.3.3): cada janela de 20 s que a medição automática gravou, com o FPS médio
  // e o 1% pior, sobre a faixa das partidas anteriores deste jogo. Só número medido; a ilustração da apresentação é
  // outra coisa. Um eixo só (os dois são FPS), linhas finas, rótulo na ponta, legenda, cruz com os números ao passar o
  // mouse e uma tabela para leitor de tela.

  type Ponto = { quando: number; fps: number | null; low_1pct: number | null; comparavel: boolean };
  type Faixa = { menor_fps: number | null; maior_fps: number | null } | null;

  let { pontos, anteriores = null }: { pontos: Ponto[]; anteriores?: Faixa } = $props();

  const L = 640;
  const A = 180;
  const M = { esq: 36, dir: 64, cima: 12, baixo: 24 };
  const w = L - M.esq - M.dir;
  const h = A - M.cima - M.baixo;

  const validos = $derived(pontos.filter((p) => p.fps != null));
  const inicio = $derived(validos[0]?.quando ?? 0);
  const minutos = (q: number) => (q - inicio) / 60;
  const fim = $derived(Math.max(1, minutos(validos.at(-1)?.quando ?? inicio)));
  const teto = $derived.by(() => {
    const vals = validos.flatMap((p) => [p.fps ?? 0, p.low_1pct ?? 0]).concat(anteriores?.maior_fps ?? 0);
    const m = Math.max(10, ...vals);
    const passo = m > 200 ? 50 : m > 100 ? 25 : 10;
    return Math.ceil((m * 1.08) / passo) * passo;
  });
  const x = (q: number) => M.esq + (minutos(q) / fim) * w;
  const y = (v: number) => M.cima + h - (v / teto) * h;
  const linha = (vals: [number, number | null][]) =>
    vals
      .filter(([, v]) => v != null)
      .map(([q, v], i) => `${i ? "L" : "M"}${x(q).toFixed(1)},${y(v as number).toFixed(1)}`)
      .join(" ");
  const caminhoFps = $derived(linha(validos.map((p) => [p.quando, p.fps])));
  const caminhoLow = $derived(linha(validos.map((p) => [p.quando, p.low_1pct])));
  const marcas = $derived([0, teto / 2, teto]);
  const temLow = $derived(validos.some((p) => p.low_1pct != null));
  const ultimo = $derived(validos.at(-1));
  const ultimoLow = $derived([...validos].reverse().find((p) => p.low_1pct != null));

  let foco = $state<Ponto | null>(null);
  function mover(e: PointerEvent) {
    const svg = e.currentTarget as SVGSVGElement;
    const r = svg.getBoundingClientRect();
    const px = ((e.clientX - r.left) / r.width) * L;
    let melhor: Ponto | null = null;
    let dist = Infinity;
    for (const p of validos) {
      const d = Math.abs(x(p.quando) - px);
      if (d < dist) [melhor, dist] = [p, d];
    }
    foco = melhor;
  }
  const n = (v: number | null) => (v == null ? "—" : Math.round(v).toLocaleString("pt-BR"));
  const min = (q: number) => `${Math.round(minutos(q))} min`;
</script>

{#if validos.length >= 2}
  <figure class="grafico">
    <figcaption class="legenda">
      <span><i class="traco fps"></i>FPS médio</span>
      {#if temLow}<span><i class="traco low"></i>1% pior</span>{/if}
      {#if anteriores?.menor_fps != null && anteriores?.maior_fps != null}<span><i class="faixa"></i>Partidas anteriores</span>{/if}
    </figcaption>
    <svg
      viewBox={`0 0 ${L} ${A}`}
      role="img"
      aria-label={`FPS ao longo da partida: ${validos.length} janelas medidas`}
      onpointermove={mover}
      onpointerleave={() => (foco = null)}
    >
      {#if anteriores?.menor_fps != null && anteriores?.maior_fps != null}
        <rect class="area-faixa" x={M.esq} width={w} y={y(anteriores.maior_fps)} height={Math.max(1, y(anteriores.menor_fps) - y(anteriores.maior_fps))} />
      {/if}
      {#each marcas as m (m)}
        <line class="grade" x1={M.esq} x2={M.esq + w} y1={y(m)} y2={y(m)} />
        <text class="eixo" x={M.esq - 6} y={y(m) + 4} text-anchor="end">{n(m)}</text>
      {/each}
      <text class="eixo" x={M.esq} y={A - 6}>0 min</text>
      <text class="eixo" x={M.esq + w} y={A - 6} text-anchor="end">{Math.round(fim)} min</text>
      {#if temLow}<path class="serie low" d={caminhoLow} />{/if}
      <path class="serie fps" d={caminhoFps} />
      {#if ultimo}
        <text class="rotulo" x={x(ultimo.quando) + 6} y={y(ultimo.fps ?? 0) + 4}>{n(ultimo.fps)} FPS</text>
      {/if}
      {#if ultimoLow && Math.abs(y(ultimoLow.low_1pct ?? 0) - y(ultimo?.fps ?? 0)) > 12}
        <text class="rotulo sec" x={x(ultimoLow.quando) + 6} y={y(ultimoLow.low_1pct ?? 0) + 4}>{n(ultimoLow.low_1pct)}</text>
      {/if}
      {#if foco}
        <line class="cruz" x1={x(foco.quando)} x2={x(foco.quando)} y1={M.cima} y2={M.cima + h} />
        <circle class="marca fps" cx={x(foco.quando)} cy={y(foco.fps ?? 0)} r="4" />
        {#if foco.low_1pct != null}<circle class="marca low" cx={x(foco.quando)} cy={y(foco.low_1pct)} r="4" />{/if}
      {/if}
    </svg>
    {#if foco}
      <p class="dica" role="status">
        {min(foco.quando)}: <strong>{n(foco.fps)} FPS</strong>{foco.low_1pct != null ? ` · 1% pior ${n(foco.low_1pct)}` : ""}{foco.comparavel ? "" : " · com gerador de quadros"}
      </p>
    {/if}
    <table class="so-leitor">
      <caption>FPS de cada janela medida</caption>
      <thead><tr><th>Minuto</th><th>FPS médio</th><th>1% pior</th></tr></thead>
      <tbody>
        {#each validos as p (p.quando)}
          <tr><td>{min(p.quando)}</td><td>{n(p.fps)}</td><td>{n(p.low_1pct)}</td></tr>
        {/each}
      </tbody>
    </table>
  </figure>
{/if}

<style>
  .grafico {
    margin: 4px 0 0;
    display: grid;
    gap: 6px;
  }
  .legenda {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    font-size: 12px;
    color: var(--tinta-2);
  }
  .legenda span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .traco {
    display: inline-block;
    width: 14px;
    height: 2px;
    border-radius: 1px;
  }
  .traco.fps {
    background: var(--tinta);
  }
  .traco.low {
    background: var(--tinta-3);
  }
  .faixa {
    display: inline-block;
    width: 14px;
    height: 10px;
    border-radius: 3px;
    background: var(--linha-forte);
  }
  svg {
    width: 100%;
    height: auto;
    touch-action: none;
  }
  .area-faixa {
    fill: var(--linha);
  }
  .grade {
    stroke: var(--linha);
    stroke-width: 1;
  }
  .eixo {
    fill: var(--tinta-3);
    font-size: 11px;
  }
  .serie {
    fill: none;
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .serie.fps {
    stroke: var(--tinta);
  }
  .serie.low {
    stroke: var(--tinta-3);
  }
  .rotulo {
    fill: var(--tinta);
    font-size: 12px;
    font-weight: 600;
  }
  .rotulo.sec {
    fill: var(--tinta-2);
    font-weight: 500;
  }
  .cruz {
    stroke: var(--linha-forte);
    stroke-width: 1;
  }
  .marca {
    stroke: var(--superficie, #fff);
    stroke-width: 2;
  }
  .marca.fps {
    fill: var(--tinta);
  }
  .marca.low {
    fill: var(--tinta-3);
  }
  .dica {
    margin: 0;
    font-size: 12px;
    color: var(--tinta-2);
  }
  .so-leitor {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
