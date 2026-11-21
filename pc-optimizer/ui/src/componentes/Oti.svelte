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
  // sombra, cor, pose ou acessório. O único movimento é o olhar: os olhos giram em volta do centro. Ele pode seguir o
  // cursor; com jogo aberto ou "reduzir movimento" o olhar fica parado no de sempre.
  //
  // No tema claro sai o negativo (disco carvão, olhos brancos): o Oti branco nunca vai em fundo claro. `tom` força um
  // dos dois quando o fundo dele não é o da página (o avatar escuro da barra, por exemplo).
  import { onMount, untrack } from "svelte";

  let {
    humor = "atento",
    tamanho = 200,
    segueCursor = false,
    tom = "auto",
    rotulo = "Oti, o assistente do Otimiza",
  }: {
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
    const reduzir = window.matchMedia("(prefers-reduced-motion: reduce)");
    let quadro = 0;
    let ultimo: PointerEvent | null = null;
    const mover = (e: PointerEvent) => {
      ultimo = e;
      // Com jogo aberto a casca congela: nem o olhar acompanha.
      if (quadro || reduzir.matches || document.documentElement.classList.contains("jogo-aberto") || !raiz) return;
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
  <g clip-path="url(#{id}-disco)">
    <g class="olhos" style:transform={`rotate(${giro}deg)`}>
      {#each olhos as o, i (i)}
        <rect class="olho" x={o.x} y={o.y} width={o.w} height={o.h} rx={o.w / 2} transform={`rotate(-32 ${o.cx} ${o.cy})`} />
      {/each}
    </g>
  </g>
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
