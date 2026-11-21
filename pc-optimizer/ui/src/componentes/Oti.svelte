<script lang="ts" module>
  /** O momento do Oti. Na marca v2 ele não tem expressão: cada momento só muda PARA ONDE ele olha (`OLHAR`). O nome
   *  continua porque a bandeja (`plataforma/bandeja.rs`) e a conversa (`lib/conversa.ts`) falam assim. */
  export type Humor =
    | "atento"
    | "falando"
    | "feliz"
    | "pensando"
    | "preocupado"
    | "surpreso"
    | "jogando"
    | "dormindo";

  /** Quantos graus os olhos giram em volta do centro, a partir do olhar de sempre (para o alto, à direita: para onde
   *  as partículas se juntam). Positivo desce no sentido do relógio. */
  export const OLHAR: Record<Humor, number> = {
    atento: 0,
    falando: 0,
    feliz: 0,
    surpreso: -10,
    pensando: -42,
    jogando: 32,
    preocupado: 78,
    dormindo: 128,
  };

  /** O olhar de sempre, em graus (o olho da frente fica em (0,50; −0,35) do raio): o cursor conta a partir daqui. */
  const OLHAR_DE_SEMPRE = (Math.atan2(-0.35, 0.5) * 180) / Math.PI;

  /** A menor volta entre dois ângulos: o olhar nunca dá a volta inteira para ir de 179° a −179°. */
  export function voltaMaisCurta(de: number, para: number) {
    const d = ((((para - de) % 360) + 540) % 360) - 180;
    return de + d;
  }

  /** O giro dos olhos para olhar um ponto (dx, dy) a partir do centro do Oti. */
  export function giroPara(dx: number, dy: number) {
    return (Math.atan2(dy, dx) * 180) / Math.PI - OLHAR_DE_SEMPRE;
  }
</script>

<script lang="ts">
  // O Oti da marca v2 (`ui/DESIGN.md`, entrega aprovada pelo dono): disco chapado e dois olhos escuros em cápsula,
  // inclinados −32°, no canto de cima à direita; o de trás encurtado, cortado pela borda. Sem boca, brilho, reflexo,
  // sombra, cor nem pose. O único movimento é o olhar: os olhos giram em volta do centro. Ele pode seguir o cursor; com
  // jogo aberto ou "reduzir movimento" o olhar fica parado no de sempre. A exceção são as épocas do ano: aí ele ganha o
  // enfeite da época (decisão do dono, 21/11/2026), posto de lado para não cobrir os olhos.
  //
  // No tema claro sai o negativo (disco carvão, olhos brancos): o Oti branco nunca vai em fundo claro. `tom` força um
  // dos dois quando o fundo dele não é o da página (o avatar escuro da barra, por exemplo).
  import { onMount, untrack } from "svelte";
  import { movimentoParado } from "../lib/movimento";
  import type { Acessorio } from "../lib/temporadas";

  let {
    humor = "atento",
    tamanho = 200,
    segueCursor = false,
    tom = "auto",
    rotulo = "Oti, o assistente do Otimiza",
    acessorio = null,
  }: {
    /** O enfeite da época (`lib/temporadas.ts`): gorro no Natal, orelhas na Páscoa… Decisão do dono (21/11/2026): as
     *  épocas continuam, com o Oti enfeitado. */
    acessorio?: Acessorio | null;
    humor?: Humor;
    tamanho?: number;
    segueCursor?: boolean;
    /** "auto" segue o tema; "branco" é o Oti original (para fundo escuro); "negativo" é o do fundo claro. */
    tom?: "auto" | "branco" | "negativo";
    /** O nome para o leitor de tela. Vazio quando o Oti é só enfeite ao lado de um texto que já diz tudo. */
    rotulo?: string;
  } = $props();

  // Os clipes do SVG precisam de id único por página.
  const id = `oti${Math.random().toString(36).slice(2, 8)}`;

  // Até 32 px valem os olhos da versão pequena (`simbolo-16`): mais grossos e para dentro, para não sumirem.
  const pequeno = $derived(tamanho <= 32);
  // Medidas da entrega (`tokens-marca.json` → `oti.geometria`), no quadro 64 × 64 com raio 30.
  const OLHOS_GRANDES = [
    { x: 44.45, y: 13.7, w: 5.1, h: 15.6, cx: 47, cy: 21.5 },
    { x: 53.75, y: 8.0, w: 3.3, h: 15.6, cx: 55.4, cy: 15.8 },
  ];
  const OLHOS_PEQUENOS = [
    { x: 36.5, y: 16.1, w: 9.0, h: 19.8, cx: 41, cy: 26 },
    { x: 49.25, y: 9.8, w: 7.5, h: 18.6, cx: 53, cy: 19.1 },
  ];
  const olhos = $derived(pequeno ? OLHOS_PEQUENOS : OLHOS_GRANDES);

  let raiz: SVGSVGElement | undefined = $state();
  let doCursor = $state<number | null>(null);
  let giro = $state(0);

  $effect(() => {
    const alvo = doCursor ?? OLHAR[humor] ?? 0;
    giro = voltaMaisCurta(untrack(() => giro), alvo);
  });

  onMount(() => {
    if (!segueCursor) return;
    let quadro = 0;
    let ultimo: PointerEvent | null = null;
    const mover = (e: PointerEvent) => {
      ultimo = e;
      // Com jogo aberto a casca congela: nem o olhar acompanha.
      if (quadro || movimentoParado() || !raiz) return;
      quadro = requestAnimationFrame(() => {
        quadro = 0;
        if (!ultimo || !raiz) return;
        const r = raiz.getBoundingClientRect();
        const dx = ultimo.clientX - (r.left + r.width / 2);
        const dy = ultimo.clientY - (r.top + r.height / 2);
        // Cursor em cima do Oti: volta ao olhar do momento.
        doCursor = Math.hypot(dx, dy) < r.width * 0.6 ? null : giroPara(dx, dy);
      });
    };
    const sair = () => (doCursor = null);
    window.addEventListener("pointermove", mover, { passive: true });
    document.addEventListener("pointerleave", sair);
    return () => {
      window.removeEventListener("pointermove", mover);
      document.removeEventListener("pointerleave", sair);
      cancelAnimationFrame(quadro);
    };
  });
</script>

<svg
  bind:this={raiz}
  class="oti"
  data-tom={tom}
  width={tamanho}
  height={tamanho}
  viewBox="0 0 64 64"
  role={rotulo ? "img" : undefined}
  aria-label={rotulo || undefined}
  aria-hidden={rotulo ? undefined : "true"}
>
  <defs>
    <clipPath id="{id}-disco"><circle cx="32" cy="32" r="29.7" /></clipPath>
  </defs>
  <circle class="disco" cx="32" cy="32" r="30" />
  {#if acessorio === "mascara"}
    <!-- Carnaval: a máscara ao longo da linha dos olhos, com plumas. Gira junto com o olhar. -->
    <g class="olhos acessorio" style:transform={`rotate(${giro}deg)`}>
      <path d="M38.5 27.5 L62 11.5" stroke="#7a2bd6" stroke-width="13" stroke-linecap="round" />
      <path d="M37.5 28.6 L63 11.2" stroke="#f4c430" stroke-width="1.2" stroke-dasharray="0.6 2.8" stroke-linecap="round" opacity="0.9" />
      <path d="M62 11 Q70 -1 66 -9 Q73 1 67 13 Z" fill="#ff4f9a" />
      <path d="M64 13 Q75 6 77 -4 Q78 9 68 16 Z" fill="#2ec4b6" />
    </g>
  {/if}
  <g clip-path="url(#{id}-disco)">
    <g class="olhos" style:transform={`rotate(${giro}deg)`}>
      {#each olhos as o, i (i)}
        <rect class="olho" x={o.x} y={o.y} width={o.w} height={o.h} rx={o.w / 2} transform={`rotate(-32 ${o.cx} ${o.cy})`} />
      {/each}
    </g>
  </g>
  {#if acessorio && acessorio !== "mascara"}
    <!-- Os enfeites de época foram desenhados no quadro 200 × 200 do Oti antigo (disco de raio 92): a escala os traz para
         o quadro 64 × 64. Inclinados para o alto à esquerda, para não cobrir os olhos, que ficam no alto à direita. -->
    <g class="acessorio" transform="translate(32 32) scale(0.326) translate(-100 -100)">
      <g transform="rotate(-38 100 100) translate(0 -4)">
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
          <ellipse cx="72" cy="-14" rx="19" ry="50" transform="rotate(-14 72 -14)" fill="#f7f7f8" stroke="#d9d9de" stroke-width="2" />
          <ellipse cx="72" cy="-10" rx="9" ry="36" transform="rotate(-14 72 -10)" fill="#ffb6c8" />
          <ellipse cx="128" cy="-18" rx="19" ry="50" transform="rotate(12 128 -18)" fill="#f7f7f8" stroke="#d9d9de" stroke-width="2" />
          <ellipse cx="128" cy="-14" rx="9" ry="36" transform="rotate(12 128 -14)" fill="#ffb6c8" />
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
    </g>
  {/if}
</svg>

<style>
  .oti {
    display: block;
    flex: none;
    overflow: visible;
  }
  .disco {
    fill: var(--oti-disco-tema, #ffffff);
  }
  .olho {
    fill: var(--oti-olho-tema, #12121c);
  }
  .oti[data-tom="branco"] .disco {
    fill: var(--oti-disco);
  }
  .oti[data-tom="branco"] .olho {
    fill: var(--oti-olho);
  }
  .oti[data-tom="negativo"] .disco {
    fill: var(--carvao);
  }
  .oti[data-tom="negativo"] .olho {
    fill: var(--branco);
  }
  /* O olhar gira em volta do centro do disco, devagar: é o único movimento do Oti. */
  .olhos {
    transform-origin: 32px 32px;
    transition: transform var(--mov-lento) var(--curva-padrao);
  }
</style>
