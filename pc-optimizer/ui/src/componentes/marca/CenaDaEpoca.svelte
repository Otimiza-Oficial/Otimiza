<script lang="ts">
  // A cena de fundo de cada época (1.0.13, pedido do dono: "um fundo de verdade" nas épocas). Uma paisagem em camadas,
  // atrás da interface: o céu da época com a luz dela, relevo em dois planos com os objetos da época (pinheiros com
  // neve, cidade com fogos, palco com serpentina, morros com ovos, fogueira com balões, balões e pipa, lua com abóboras)
  // e, no canto de baixo à esquerda, o Oti da marca com o enfeite da época olhando o brilho.
  //
  // É um SVG só, desenhado uma vez, com semente fixa (nada de `Math.random`, nada de canvas): não custa nada parado. O
  // que se mexe é pouco e por CSS (luzes que piscam devagar, a cena que aparece ao abrir) e para com "reduzir movimento"
  // e com o jogo aberto (regras globais do `tokens.css`). As partículas que caem continuam no `EfeitoDeEpoca`.
  //
  // A composição respeita onde a cena aparece: a faixa de baixo (atrás das abas), a coluna da esquerda (abaixo do
  // trilho) e o céu de cima (atrás da barra, sem nada claro sob o texto). Ancorada embaixo (`xMidYMax slice`).
  import Oti from "../Oti.svelte";
  import Brilho from "./Brilho.svelte";
  import { estrela, sorteio } from "../../lib/acaso";
  import type { Temporada } from "../../lib/temporadas";

  let { temporada }: { temporada: Temporada } = $props();

  const L = 1600;
  const A = 1000;
  const f = (n: number) => n.toFixed(1);

  /** A linha de um morro: uma soma de ondas, para os objetos pousarem em cima dela. */
  function altura(x: number, base: number, amp: number, fase: number) {
    return base - amp * (0.55 * Math.sin(x / 260 + fase) + 0.3 * Math.sin(x / 113 + fase * 2.1) + 0.15 * Math.sin(x / 47 + fase * 3.7));
  }
  function morro(base: number, amp: number, fase: number) {
    let d = `M0 ${A}`;
    for (let x = 0; x <= L; x += 20) d += ` L${x} ${f(altura(x, base, amp, fase))}`;
    return `${d} L${L} ${A} Z`;
  }
  function crista(base: number, amp: number, fase: number) {
    let d = "";
    for (let x = 0; x <= L; x += 20) d += `${x ? " L" : "M"}${x} ${f(altura(x, base, amp, fase))}`;
    return d;
  }

  // Os dois planos de relevo de toda cena.
  const LONGE = { base: 905, amp: 26, fase: 1.3 };
  const PERTO = { base: 952, amp: 16, fase: 4.1 };
  const noLonge = (x: number) => altura(x, LONGE.base, LONGE.amp, LONGE.fase);
  const noPerto = (x: number) => altura(x, PERTO.base, PERTO.amp, PERTO.fase);

  // Onde os objetos ficam visíveis com a janela em 1280 × 800: a faixa de baixo dos dois lados das abas.
  const LADO_ESQUERDO = [130, 210, 290, 370, 450, 530];
  const LADO_DIREITO = [1070, 1150, 1230, 1310, 1390, 1470, 1550];

  const ceu = $derived.by(() => {
    const acaso = sorteio(31 + temporada.id.length);
    return {
      estrelas: Array.from({ length: 140 }, () => ({
        x: acaso() * L,
        y: Math.pow(acaso(), 1.6) * 820,
        r: 0.4 + acaso() * 1.1,
        o: 0.15 + acaso() * 0.6,
      })),
      brilhos: Array.from({ length: 9 }, () => ({ d: estrela(acaso() * L, 40 + acaso() * 520, 2.5 + acaso() * 4), o: 0.25 + acaso() * 0.4 })),
    };
  });

  function pinheiro(x: number, y: number, h: number) {
    const w = h * 0.42;
    const camadas = [0, 0.3, 0.56].map((k, i) => {
      const topo = y - h + k * h;
      const larg = w * (0.55 + i * 0.25);
      return `M${f(x)} ${f(topo)} L${f(x + larg)} ${f(topo + h * 0.42)} L${f(x - larg)} ${f(topo + h * 0.42)} Z`;
    });
    return { arvore: camadas.join(" "), tronco: `M${f(x - 3)} ${f(y - h * 0.04)} h6 v${f(h * 0.1)} h-6 Z`, neve: camadas.map((_, i) => ({ x, y: y - h + [0, 0.3, 0.56][i] * h, w: w * (0.55 + i * 0.25), h })) };
  }

  const natal = $derived.by(() => {
    const acaso = sorteio(1225);
    const longe = [...LADO_ESQUERDO, ...LADO_DIREITO, 40, 70].map((x) => ({ ...pinheiro(x + acaso() * 30, noLonge(x) + 4, 60 + acaso() * 40), luzes: acaso() > 0.55 }));
    const perto = [150, 330, 500, 1100, 1260, 1420, 1580].map((x) => pinheiro(x + acaso() * 20, noPerto(x) + 6, 90 + acaso() * 40));
    const luzes = longe
      .filter((p) => p.luzes)
      .flatMap((p, i) =>
        Array.from({ length: 6 }, (_, k) => {
          const t = (k + 1) / 7;
          const yy = p.neve[0].y + t * p.neve[0].h * 0.9;
          return { x: p.neve[0].x + (k % 2 ? 1 : -1) * t * p.neve[0].w * 1.1, y: yy, cor: ["#ffd36b", "#ff4d4d", "#5fd38d"][(i + k) % 3], d: (k * 0.37 + i) % 3 };
        }),
      );
    const flocos = Array.from({ length: 16 }, () => {
      const esquerda = acaso() > 0.5;
      return { x: acaso() * 110 + (esquerda ? 0 : 1480), y: (esquerda ? 480 : 140) + acaso() * (esquerda ? 300 : 620), r: 3 + acaso() * 5, o: 0.3 + acaso() * 0.5 };
    });
    return { longe, perto, luzes, flocos };
  });

  const anoNovo = $derived.by(() => {
    const acaso = sorteio(2026);
    const predios: { x: number; y: number; w: number; h: number; janelas: { x: number; y: number }[] }[] = [];
    for (let x = 0; x < L; ) {
      const w = 26 + acaso() * 46;
      const h = 50 + acaso() * (x < 120 || x > 1040 ? 150 : 70);
      const y = 1000 - h;
      const janelas = [];
      for (let jy = y + 10; jy < 990; jy += 11)
        for (let jx = x + 6; jx < x + w - 6; jx += 9) if (acaso() > 0.72) janelas.push({ x: jx, y: jy });
      predios.push({ x, y, w, h, janelas });
      x += w + 2;
    }
    const fogo = (cx: number, cy: number, r: number, cor: string, n: number) => ({
      cx,
      cy,
      r,
      cor,
      raios: Array.from({ length: n }, (_, i) => {
        const a = (i / n) * Math.PI * 2 + acaso() * 0.1;
        const r1 = r * (0.25 + acaso() * 0.15);
        const r2 = r * (0.8 + acaso() * 0.25);
        return { x1: cx + Math.cos(a) * r1, y1: cy + Math.sin(a) * r1, x2: cx + Math.cos(a) * r2, y2: cy + Math.sin(a) * r2 };
      }),
    });
    return {
      predios,
      fogos: [fogo(62, 640, 52, "#f2c14e", 22), fogo(1490, 300, 90, "#ffffff", 28), fogo(1380, 560, 54, "#ffd36b", 18), fogo(76, 520, 36, "#ffe9a8", 16)],
      bokeh: Array.from({ length: 22 }, () => ({ x: acaso() * L, y: 80 + acaso() * 760, r: 4 + acaso() * 14, o: 0.06 + acaso() * 0.12 })),
    };
  });

  const carnaval = $derived.by(() => {
    const acaso = sorteio(47);
    const cores = ["#ff4fa3", "#8b5cf6", "#2ec4b6", "#ffd36b"];
    const serpentina = (x0: number, y0: number, comp: number, cor: string) => {
      let d = `M${f(x0)} ${f(y0)}`;
      for (let i = 1; i <= 8; i++) d += ` Q${f(x0 + (i - 0.5) * (comp / 8))} ${f(y0 + (i % 2 ? -18 : 18))} ${f(x0 + i * (comp / 8))} ${f(y0 + Math.sin(i) * 10)}`;
      return { d, cor };
    };
    return {
      fachos: [
        { x: 260, cor: "#ff4fa3", a: -18 },
        { x: 1300, cor: "#8b5cf6", a: 16 },
        { x: 1480, cor: "#2ec4b6", a: -8 },
        { x: 60, cor: "#ffd36b", a: 12 },
      ],
      serpentinas: [serpentina(0, 640, 120, cores[0]), serpentina(0, 740, 110, cores[2]), serpentina(1060, 930, 520, cores[1]), serpentina(120, 945, 440, cores[3]), serpentina(1460, 520, 140, cores[0])],
      confete: Array.from({ length: 90 }, () => {
        const lado = acaso() > 0.5;
        return { x: lado ? acaso() * 115 : acaso() * L, y: lado ? 500 + acaso() * 380 : 880 + acaso() * 120, r: acaso() * 180, cor: cores[Math.floor(acaso() * cores.length)], w: 3 + acaso() * 4 };
      }),
    };
  });

  const pascoa = $derived.by(() => {
    const acaso = sorteio(412);
    const pasteis = ["#fbcfe8", "#c4b5fd", "#a7f3d0", "#fde68a", "#bae6fd"];
    const ovos = [150, 270, 420, 1120, 1290, 1450, 1560].map((x, i) => ({ x, y: noPerto(x) + 2, cor: pasteis[i % pasteis.length], faixa: pasteis[(i + 2) % pasteis.length], r: (acaso() - 0.5) * 30, s: 0.8 + acaso() * 0.4 }));
    const flores = Array.from({ length: 46 }, () => {
      const x = acaso() > 0.5 ? 110 + acaso() * 460 : 1040 + acaso() * 560;
      return { x, y: noPerto(x) + 4 + acaso() * 40, cor: pasteis[Math.floor(acaso() * pasteis.length)], r: 2.2 + acaso() * 2 };
    });
    return { ovos, flores };
  });

  const junina = $derived.by(() => {
    const acaso = sorteio(624);
    const baloes = [
      { x: 58, y: 560, s: 0.9, cor: "#ff8a1f" },
      { x: 26, y: 660, s: 0.6, cor: "#facc15" },
      { x: 96, y: 700, s: 0.5, cor: "#ef4444" },
      { x: 1520, y: 360, s: 0.9, cor: "#facc15" },
      { x: 1440, y: 520, s: 0.65, cor: "#ff8a1f" },
      { x: 1560, y: 640, s: 0.5, cor: "#ef4444" },
    ];
    const chamas = Array.from({ length: 7 }, (_, i) => ({ dx: (i - 3) * 6, h: 26 + acaso() * 22, cor: i % 2 ? "#ffd36b" : "#ff8a1f" }));
    const brasas = Array.from({ length: 18 }, () => ({ dx: (acaso() - 0.5) * 60, dy: -30 - acaso() * 120, r: 0.8 + acaso() * 1.4 }));
    return { baloes, chamas, brasas };
  });

  const criancas = $derived.by(() => {
    const acaso = sorteio(1012);
    const cores = ["#3b82f6", "#facc15", "#ef4444", "#22c55e", "#a855f7"];
    const baloes = [
      { x: 40, y: 540, s: 0.9 },
      { x: 88, y: 600, s: 0.7 },
      { x: 30, y: 680, s: 0.6 },
      { x: 1480, y: 300, s: 1.1 },
      { x: 1560, y: 420, s: 0.8 },
      { x: 1420, y: 620, s: 0.7 },
    ].map((b, i) => ({ ...b, cor: cores[i % cores.length], fio: (acaso() - 0.5) * 20 }));
    return { baloes, catavento: { x: 470, y: noPerto(470) - 50 } };
  });

  const halloween = $derived.by(() => {
    const acaso = sorteio(1031);
    const abobora = [180, 400, 1110, 1330, 1520].map((x) => ({ x, y: noPerto(x) + 2, s: 0.8 + acaso() * 0.5 }));
    const morcegos = [
      { x: 74, y: 560, s: 1 },
      { x: 26, y: 620, s: 0.7 },
      { x: 104, y: 660, s: 0.55 },
      { x: 1460, y: 260, s: 0.9 },
      { x: 1540, y: 330, s: 0.6 },
    ];
    const galho = (x: number, y: number, h: number, lado: number) => {
      const t = y - h;
      return `M${f(x)} ${f(y)} C${f(x + 4 * lado)} ${f(y - h * 0.4)} ${f(x - 6 * lado)} ${f(y - h * 0.7)} ${f(x + 10 * lado)} ${f(t)} M${f(x + 2 * lado)} ${f(y - h * 0.55)} q${f(22 * lado)} -14 ${f(34 * lado)} -32 M${f(x - 2 * lado)} ${f(y - h * 0.75)} q${f(-18 * lado)} -10 ${f(-26 * lado)} -30 M${f(x + 6 * lado)} ${f(y - h * 0.86)} q${f(12 * lado)} -6 ${f(20 * lado)} -18`;
    };
    return {
      abobora,
      morcegos,
      arvores: [galho(150, noLonge(150) + 6, 120, 1), galho(1180, noLonge(1180) + 6, 140, -1), galho(1500, noLonge(1500) + 6, 110, 1)],
    };
  });

  function morcego(x: number, y: number, s: number) {
    return `M${f(x)} ${f(y)} c${f(-6 * s)} ${f(-8 * s)} ${f(-16 * s)} ${f(-8 * s)} ${f(-24 * s)} ${f(-2 * s)} c${f(6 * s)} 0 ${f(8 * s)} ${f(4 * s)} ${f(8 * s)} ${f(8 * s)} c${f(4 * s)} ${f(-4 * s)} ${f(8 * s)} ${f(-4 * s)} ${f(12 * s)} ${f(-2 * s)} c${f(2 * s)} ${f(-3 * s)} ${f(2 * s)} ${f(-3 * s)} ${f(4 * s)} 0 c${f(4 * s)} ${f(-2 * s)} ${f(8 * s)} ${f(-2 * s)} ${f(12 * s)} ${f(2 * s)} c0 ${f(-4 * s)} ${f(2 * s)} ${f(-8 * s)} ${f(8 * s)} ${f(-8 * s)} c${f(-8 * s)} ${f(-6 * s)} ${f(-18 * s)} ${f(-6 * s)} ${f(-24 * s)} ${f(2 * s)} Z`;
  }
</script>

<div class="cena" data-epoca={temporada.id} aria-hidden="true">
  <svg viewBox="0 0 {L} {A}" preserveAspectRatio="xMidYMax slice">
    <defs>
      <linearGradient id="cena-ceu" x1="0" y1="0" x2="0" y2="1">
        <stop class="sc-ceu-1" offset="0" />
        <stop class="sc-ceu-2" offset="0.72" />
        <stop class="sc-ceu-3" offset="1" />
      </linearGradient>
      <radialGradient id="cena-luz-a" cx="0.5" cy="0.5" r="0.5">
        <stop class="sc-luz-a" offset="0" stop-opacity="0.55" />
        <stop class="sc-luz-a" offset="1" stop-opacity="0" />
      </radialGradient>
      <radialGradient id="cena-luz-b" cx="0.5" cy="0.5" r="0.5">
        <stop class="sc-luz-b" offset="0" stop-opacity="0.45" />
        <stop class="sc-luz-b" offset="1" stop-opacity="0" />
      </radialGradient>
      <radialGradient id="cena-quente" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0" stop-color="#ffd36b" stop-opacity="0.9" />
        <stop offset="0.35" stop-color="#ff8a1f" stop-opacity="0.35" />
        <stop offset="1" stop-color="#ff8a1f" stop-opacity="0" />
      </radialGradient>
      <radialGradient id="cena-lua" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0" stop-color="#fff4d6" stop-opacity="0.5" />
        <stop offset="0.4" stop-color="#fff4d6" stop-opacity="0.12" />
        <stop offset="1" stop-color="#fff4d6" stop-opacity="0" />
      </radialGradient>
      <linearGradient id="cena-facho" x1="0" y1="1" x2="0" y2="0">
        <stop offset="0" stop-color="white" stop-opacity="0.5" />
        <stop offset="1" stop-color="white" stop-opacity="0" />
      </linearGradient>
      <linearGradient id="cena-chao" x1="0" y1="0" x2="0" y2="1">
        <stop class="sc-relevo-2" offset="0" />
        <stop class="sc-relevo-3" offset="1" />
      </linearGradient>
    </defs>

    <!-- O céu e a luz da época -->
    <rect width={L} height={A} fill="url(#cena-ceu)" />
    <ellipse cx="120" cy="760" rx="520" ry="380" fill="url(#cena-luz-a)" />
    <ellipse cx="1420" cy="380" rx="560" ry="420" fill="url(#cena-luz-b)" />
    <g class="estrelas f-estrela">
      {#each ceu.estrelas as s, i (i)}<circle cx={f(s.x)} cy={f(s.y)} r={s.r.toFixed(2)} opacity={s.o.toFixed(2)} />{/each}
      {#each ceu.brilhos as b, i (i)}<path d={b.d} opacity={b.o.toFixed(2)} />{/each}
    </g>

    <!-- O que fica no céu de cada época -->
    {#if temporada.id === "natal"}
      <g class="f-estrela">
        {#each natal.flocos as k, i (i)}
          <g class="s-estrela" transform="translate({f(k.x)} {f(k.y)})" opacity={k.o.toFixed(2)} stroke-width="0.9" stroke-linecap="round">
            {#each [0, 60, 120] as a (a)}<line x1={-k.r} y1="0" x2={k.r} y2="0" transform="rotate({a})" />{/each}
          </g>
        {/each}
      </g>
    {:else if temporada.id === "ano-novo"}
      <g>
        {#each anoNovo.bokeh as b, i (i)}<circle cx={f(b.x)} cy={f(b.y)} r={f(b.r)} fill="#f2c14e" opacity={b.o.toFixed(2)} />{/each}
        {#each anoNovo.fogos as fg, i (i)}
          <g class="pisca" style:animation-delay={`${-i * 0.9}s`}>
            <circle cx={fg.cx} cy={fg.cy} r={fg.r * 1.2} fill={fg.cor} opacity="0.06" />
            {#each fg.raios as r, k (k)}
              <line x1={f(r.x1)} y1={f(r.y1)} x2={f(r.x2)} y2={f(r.y2)} stroke={fg.cor} stroke-width="1.6" stroke-linecap="round" opacity="0.8" />
              <circle cx={f(r.x2)} cy={f(r.y2)} r="1.8" fill={fg.cor} />
            {/each}
          </g>
        {/each}
      </g>
    {:else if temporada.id === "carnaval"}
      <g>
        {#each carnaval.fachos as fx, i (i)}
          <path d="M{fx.x - 14} 1000 L{fx.x - 150} 120 L{fx.x + 150} 120 L{fx.x + 14} 1000 Z" fill={fx.cor} opacity="0.13" transform="rotate({fx.a} {fx.x} 1000)" />
        {/each}
        {#each carnaval.serpentinas as s, i (i)}<path d={s.d} fill="none" stroke={s.cor} stroke-width="3" stroke-linecap="round" opacity="0.85" />{/each}
        {#each carnaval.confete as c, i (i)}<rect x={f(c.x)} y={f(c.y)} width={f(c.w)} height={f(c.w * 1.7)} rx="1" fill={c.cor} opacity="0.85" transform="rotate({f(c.r)} {f(c.x)} {f(c.y)})" />{/each}
      </g>
    {:else if temporada.id === "festa-junina"}
      <g>
        {#each junina.baloes as b, i (i)}
          <g transform="translate({b.x} {b.y}) scale({b.s})" class="pisca" style:animation-delay={`${-i * 0.7}s`}>
            <circle cx="0" cy="10" r="46" fill="url(#cena-quente)" opacity="0.6" />
            <path d="M-14 -22 L14 -22 L22 18 Q0 30 -22 18 Z" fill={b.cor} />
            <path d="M-14 -22 L14 -22 L22 18 Q0 30 -22 18 Z" fill="white" opacity="0.15" transform="translate(4 0) scale(0.5 1)" />
            <path d="M-18 -4 h36 M-20 8 h40" stroke="#1d1d1f" stroke-opacity="0.25" stroke-width="1.5" />
            <rect x="-6" y="24" width="12" height="7" rx="2" fill="#ffd36b" />
          </g>
        {/each}
      </g>
    {:else if temporada.id === "dia-das-criancas"}
      <g>
        {#each criancas.baloes as b, i (i)}
          <g transform="translate({b.x} {b.y}) scale({b.s})">
            <path class="s-estrela" d="M0 34 q{b.fio} 40 {-b.fio / 2} 90" fill="none" stroke-opacity="0.4" stroke-width="1" />
            <ellipse cx="0" cy="0" rx="24" ry="30" fill={b.cor} />
            <ellipse cx="-8" cy="-11" rx="6" ry="9" fill="white" opacity="0.35" />
            <path d="M-4 30 L4 30 L0 36 Z" fill={b.cor} />
          </g>
        {/each}
        <!-- A pipa, no alto à direita -->
        <g transform="translate(1320 220) rotate(-12)">
          <path d="M0 -40 L28 0 L0 46 L-28 0 Z" fill="#ef4444" />
          <path d="M0 -40 L0 46 M-28 0 L28 0" stroke="white" stroke-opacity="0.5" stroke-width="1.2" />
          <path class="s-estrela" d="M0 46 q10 30 -6 60 q-14 30 6 62" fill="none" stroke-opacity="0.45" stroke-width="1" />
          {#each [70, 100, 130] as y (y)}<path d="M-5 {y} l5 -6 l5 6 l-5 6 Z" fill="#facc15" />{/each}
        </g>
      </g>
    {:else if temporada.id === "halloween"}
      <g>
        <circle cx="66" cy="610" r="150" fill="url(#cena-lua)" />
        <circle cx="66" cy="610" r="62" fill="#f3ead2" />
        <circle cx="48" cy="592" r="9" fill="#e2d6b6" />
        <circle cx="86" cy="630" r="13" fill="#e2d6b6" />
        <circle cx="72" cy="578" r="5" fill="#e2d6b6" />
        {#each halloween.morcegos as m, i (i)}<path class="f-silhueta" d={morcego(m.x, m.y, m.s * 1.4)} />{/each}
      </g>
    {/if}

    <!-- O plano de longe -->
    <path class="f-relevo-1" d={morro(LONGE.base, LONGE.amp, LONGE.fase)} />
    {#if temporada.id === "natal"}
      <path class="s-neve" d={crista(LONGE.base, LONGE.amp, LONGE.fase)} fill="none" stroke-width="3" opacity="0.7" />
      {#each natal.longe as p, i (i)}<path class="f-pinheiro" d={p.arvore} />{/each}
      {#each natal.luzes as l, i (i)}<circle cx={f(l.x)} cy={f(l.y)} r="1.9" fill={l.cor} class="pisca" style:animation-delay={`${-l.d}s`} />{/each}
      <!-- A casinha com a janela acesa -->
      <g transform="translate(1500 {f(noLonge(1500) - 2)})">
        <path class="f-silhueta" d="M-22 0 V-24 L0 -42 L22 -24 V0 Z" />
        <path class="s-neve" d="M-26 -22 L0 -46 L26 -22" fill="none" stroke-width="4" stroke-linecap="round" />
        <rect x="-7" y="-20" width="12" height="11" fill="#ffd36b" />
        <circle cx="-1" cy="-15" r="26" fill="url(#cena-quente)" opacity="0.5" />
      </g>
    {:else if temporada.id === "ano-novo"}
      {#each anoNovo.predios as p, i (i)}
        <rect class="f-silhueta" x={f(p.x)} y={f(p.y)} width={f(p.w)} height={f(p.h)} />
        {#each p.janelas as j, k (k)}<rect x={f(j.x)} y={f(j.y)} width="3" height="4" fill="#ffd36b" opacity="0.55" />{/each}
      {/each}
    {:else if temporada.id === "festa-junina"}
      <!-- A capelinha no morro -->
      <g class="f-silhueta" transform="translate(1330 {f(noLonge(1330) + 2)})">
        <path d="M-24 0 V-34 L0 -54 L24 -34 V0 Z" />
        <rect x="-6" y="-84" width="12" height="32" />
        <path d="M-2 -96 h4 v8 h5 v4 h-5 v8 h-4 v-8 h-5 v-4 h5 Z" />
        <rect x="-5" y="-26" width="10" height="16" rx="5" fill="#ffd36b" opacity="0.8" />
      </g>
    {:else if temporada.id === "halloween"}
      {#each halloween.arvores as d, i (i)}<path class="s-silhueta" {d} fill="none" stroke-width="6" stroke-linecap="round" />{/each}
    {/if}

    <!-- O plano de perto -->
    <path d={morro(PERTO.base, PERTO.amp, PERTO.fase)} fill="url(#cena-chao)" />
    {#if temporada.id === "natal"}
      <path class="s-neve" d={crista(PERTO.base, PERTO.amp, PERTO.fase)} fill="none" stroke-width="6" stroke-linecap="round" opacity="0.85" />
      {#each natal.perto as p, i (i)}
        <path class="f-pinheiro-perto" d={p.arvore} />
        <path class="f-silhueta" d={p.tronco} />
        {#each p.neve as n, k (k)}<path class="s-neve" d="M{f(n.x - n.w * 0.28)} {f(n.y + n.h * 0.12)} L{f(n.x)} {f(n.y)} L{f(n.x + n.w * 0.28)} {f(n.y + n.h * 0.12)}" fill="none" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" opacity="0.8" />{/each}
      {/each}
    {:else if temporada.id === "pascoa"}
      {#each pascoa.flores as fl, i (i)}
        <g transform="translate({f(fl.x)} {f(fl.y)})">
          {#each [0, 72, 144, 216, 288] as a (a)}<circle cx="0" cy={-fl.r} r={fl.r * 0.8} fill={fl.cor} transform="rotate({a})" />{/each}
          <circle r={fl.r * 0.6} fill="#fde68a" />
        </g>
      {/each}
      {#each pascoa.ovos as o, i (i)}
        <g transform="translate({o.x} {f(o.y)}) rotate({f(o.r)}) scale({o.s.toFixed(2)})">
          <ellipse cx="0" cy="-22" rx="16" ry="22" fill={o.cor} />
          <path d="M-15 -26 l5 -5 l5 5 l5 -5 l5 5 l5 -5 l5 5" fill="none" stroke={o.faixa} stroke-width="3" stroke-linejoin="round" />
          <path d="M-15 -14 h30" stroke="#ffffff" stroke-opacity="0.55" stroke-width="3" />
          <ellipse cx="-6" cy="-32" rx="4" ry="6" fill="white" opacity="0.35" />
        </g>
      {/each}
    {:else if temporada.id === "festa-junina"}
      <!-- A fogueira -->
      <g transform="translate(300 {f(noPerto(300) + 8)})">
        <circle cx="0" cy="-26" r="120" fill="url(#cena-quente)" opacity="0.7" />
        <path d="M-34 0 L30 -14 M-30 -14 L34 0" stroke="#5b3a1e" stroke-width="9" stroke-linecap="round" />
        {#each junina.chamas as c, i (i)}
          <path d="M{c.dx - 9} -6 Q{c.dx - 6} {-c.h * 0.6} {c.dx} {-c.h} Q{c.dx + 6} {-c.h * 0.6} {c.dx + 9} -6 Z" fill={c.cor} class="pisca" style:animation-delay={`${-i * 0.4}s`} />
        {/each}
        {#each junina.brasas as b, i (i)}<circle cx={f(b.dx)} cy={f(b.dy)} r={b.r.toFixed(2)} fill="#ffd36b" opacity="0.8" />{/each}
      </g>
    {:else if temporada.id === "dia-das-criancas"}
      <!-- O catavento -->
      <g transform="translate({criancas.catavento.x} {f(criancas.catavento.y)})">
        <path class="s-silhueta" d="M0 0 V56" stroke-width="3" />
        {#each ["#ef4444", "#facc15", "#3b82f6", "#22c55e"] as cor, i (cor)}<path d="M0 0 L0 -26 Q16 -20 0 0 Z" fill={cor} transform="rotate({i * 90})" />{/each}
        <circle r="3" fill="white" />
      </g>
    {:else if temporada.id === "halloween"}
      {#each halloween.abobora as a, i (i)}
        <g transform="translate({a.x} {f(a.y)}) scale({a.s.toFixed(2)})">
          <circle cx="0" cy="-16" r="34" fill="url(#cena-quente)" opacity="0.55" />
          <ellipse cx="-10" cy="-15" rx="12" ry="15" fill="#e8741c" />
          <ellipse cx="10" cy="-15" rx="12" ry="15" fill="#e8741c" />
          <ellipse cx="0" cy="-15" rx="12" ry="16" fill="#f97316" />
          <path d="M-1 -30 q2 -8 7 -10" stroke="#3f6212" stroke-width="3" fill="none" stroke-linecap="round" />
          <path d="M-11 -19 l4 -5 l4 5 Z M3 -19 l4 -5 l4 5 Z M-10 -9 q10 6 20 0 l-3 3 l-3 -3 l-4 3 l-4 -3 l-3 3 Z" fill="#ffd36b" class="pisca" style:animation-delay={`${-i * 0.6}s`} />
        </g>
      {/each}
    {/if}
  </svg>

  <!-- O Oti da marca, com o enfeite da época, olhando o brilho -->
  <span class="cena-oti"><Oti acessorio={temporada.acessorio} humor="pensando" tamanho={56} rotulo="" /></span>
  <span class="cena-brilho"><Brilho tamanho={110} densidade={1.1} semente={temporada.id.length * 7} /></span>
</div>

<style>
  /* As cores da cena vêm das variáveis do tema e da época: atributo do SVG não lê var(). */
  .f-estrela {
    fill: var(--estrela);
  }
  .f-pinheiro {
    fill: var(--pinheiro);
  }
  .f-pinheiro-perto {
    fill: var(--pinheiro-perto);
  }
  .f-relevo-1 {
    fill: var(--relevo-1);
  }
  .f-silhueta {
    fill: var(--silhueta);
  }
  .sc-ceu-1 {
    stop-color: var(--ceu-1);
  }
  .sc-ceu-2 {
    stop-color: var(--ceu-2);
  }
  .sc-ceu-3 {
    stop-color: var(--ceu-3);
  }
  .sc-luz-a {
    stop-color: var(--luz-a);
  }
  .sc-luz-b {
    stop-color: var(--luz-b);
  }
  .sc-relevo-2 {
    stop-color: var(--relevo-2);
  }
  .sc-relevo-3 {
    stop-color: var(--relevo-3);
  }
  .s-estrela {
    stroke: var(--estrela);
  }
  .s-neve {
    stroke: var(--neve);
  }
  .s-silhueta {
    stroke: var(--silhueta);
  }
  .cena {
    position: fixed;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    animation: cena-aparece var(--mov-lento) var(--curva-saida) both;
    /* Escuro (o padrão): a cena noturna da marca. */
    --ceu-1: #000000;
    --ceu-2: #07080d;
    --ceu-3: #0c0d14;
    --relevo-1: #0d0f15;
    --relevo-2: #08090d;
    --relevo-3: #040406;
    --silhueta: #050507;
    --estrela: #ffffff;
    --neve: #e9f1ff;
    --pinheiro: #0f2620;
    --pinheiro-perto: #0c2019;
    --luz-a: #ffffff;
    --luz-b: #ffffff;
  }
  .cena svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .cena[data-epoca="natal"] {
    --ceu-2: #07101f;
    --ceu-3: #0d1a2e;
    --relevo-1: #0c1626;
    --relevo-2: #0a1220;
    --relevo-3: #070c16;
    --luz-a: #d62f3a;
    --luz-b: #3a8f6a;
  }
  .cena[data-epoca="ano-novo"] {
    --ceu-2: #110d06;
    --ceu-3: #1a1408;
    --luz-a: #f2c14e;
    --luz-b: #ffffff;
  }
  .cena[data-epoca="carnaval"] {
    --ceu-1: #05020a;
    --ceu-2: #140726;
    --ceu-3: #1d0a33;
    --relevo-1: #12081f;
    --relevo-2: #0c0516;
    --relevo-3: #07030d;
    --luz-a: #ff4fa3;
    --luz-b: #8b5cf6;
  }
  .cena[data-epoca="pascoa"] {
    --ceu-2: #160f20;
    --ceu-3: #2a1a2c;
    --relevo-1: #11201a;
    --relevo-2: #0c1813;
    --relevo-3: #08100c;
    --luz-a: #fbcfe8;
    --luz-b: #a7f3d0;
  }
  .cena[data-epoca="festa-junina"] {
    --ceu-2: #120a06;
    --ceu-3: #22120a;
    --relevo-1: #140d08;
    --relevo-2: #0e0905;
    --relevo-3: #070503;
    --luz-a: #ff8a1f;
    --luz-b: #facc15;
  }
  .cena[data-epoca="dia-das-criancas"] {
    --ceu-2: #081228;
    --ceu-3: #0d1d3d;
    --relevo-1: #0b1a2a;
    --relevo-2: #08131f;
    --relevo-3: #050b13;
    --luz-a: #3b82f6;
    --luz-b: #facc15;
  }
  .cena[data-epoca="halloween"] {
    --ceu-1: #040208;
    --ceu-2: #12071c;
    --ceu-3: #1f0c2a;
    --relevo-1: #120a18;
    --relevo-2: #0b0610;
    --relevo-3: #050308;
    --luz-a: #f97316;
    --luz-b: #a855f7;
  }
  /* Claro: a mesma cena de dia, em tons de papel; o relevo fica em cinzas da época e as estrelas somem. */
  :global(:root[data-tema="dia"]) .cena {
    --ceu-1: #f5f5f7;
    --ceu-2: #eceef3;
    --ceu-3: #e2e5ec;
    --relevo-1: #d9dce4;
    --relevo-2: #c9cdd8;
    --relevo-3: #bcc1ce;
    --silhueta: #9ea4b3;
    --estrela: #8e8e93;
    --neve: #ffffff;
    --pinheiro: #8fa79c;
    --pinheiro-perto: #7d978b;
  }
  :global(:root[data-tema="dia"]) .cena[data-epoca="natal"] {
    --ceu-2: #e6eef8;
    --ceu-3: #d9e5f3;
    --relevo-1: #d3dce8;
    --relevo-2: #c4cfdd;
  }
  :global(:root[data-tema="dia"]) .cena[data-epoca="carnaval"] {
    --ceu-2: #f3e8f7;
    --ceu-3: #ead9f2;
    --relevo-1: #ddd0e6;
    --relevo-2: #cfbfdb;
    --relevo-3: #c3b1d1;
  }
  :global(:root[data-tema="dia"]) .cena[data-epoca="pascoa"] {
    --ceu-2: #f8eef3;
    --ceu-3: #f2e1ea;
    --relevo-1: #d5e6da;
    --relevo-2: #c2dbc8;
    --relevo-3: #b2d0b9;
  }
  :global(:root[data-tema="dia"]) .cena[data-epoca="festa-junina"],
  :global(:root[data-tema="dia"]) .cena[data-epoca="ano-novo"] {
    --ceu-2: #f5ede0;
    --ceu-3: #efe1c9;
    --relevo-1: #e2d6c3;
    --relevo-2: #d6c7b0;
    --relevo-3: #cab99f;
  }
  :global(:root[data-tema="dia"]) .cena[data-epoca="dia-das-criancas"] {
    --ceu-2: #e7f0fd;
    --ceu-3: #d9e7fb;
    --relevo-1: #cfdcee;
    --relevo-2: #bfcfe6;
    --relevo-3: #b1c3de;
  }
  :global(:root[data-tema="dia"]) .cena[data-epoca="halloween"] {
    --ceu-2: #f1e9f5;
    --ceu-3: #e7d9ee;
    --relevo-1: #dacde2;
    --relevo-2: #cbbbd6;
    --relevo-3: #bfadcb;
  }
  /* O Oti mora na coluna da esquerda, abaixo do trilho, sentado no morro de perto. */
  .cena-oti {
    position: absolute;
    left: 22px;
    bottom: 64px;
  }
  .cena-brilho {
    position: absolute;
    left: 40px;
    bottom: 128px;
  }
  /* As luzes da cena piscam devagar; param com o jogo aberto e com "reduzir movimento" (regras globais). */
  .pisca {
    animation: pisca 3.2s ease-in-out infinite alternate;
  }
  @keyframes pisca {
    from {
      opacity: 1;
    }
    to {
      opacity: 0.45;
    }
  }
  @keyframes cena-aparece {
    from {
      opacity: 0;
    }
  }
  @media (max-height: 640px) {
    .cena-oti,
    .cena-brilho {
      display: none;
    }
  }
</style>
