<script lang="ts" module>
  export type ModoDasBoasVindas = "completa" | "licenca";
  /** Os Ajustes pedem a apresentação de novo por este evento (ela mora no App, por cima de tudo). */
  export const ABRIR_BOAS_VINDAS = "otimiza:boas-vindas";
</script>

<script lang="ts">
  // A apresentação da primeira abertura (e o atalho "Ativar licença" da barra, que abre só o passo da chave). Seis
  // passos: o Oti, como o Otimiza trabalha, o PC lido de verdade, a licença, a aparência e o "Jogar". Nada aqui
  // afirma o que não foi lido: o PC aparece com o que a ficha de hardware trouxe, ou "lendo". Com "reduzir movimento"
  // no Windows, as animações param.
  import { fade, fly, scale } from "svelte/transition";
  import { backOut, cubicOut } from "svelte/easing";
  import Oti from "./Oti.svelte";
  import Icone from "./Icone.svelte";
  import AtivarLicenca from "./AtivarLicenca.svelte";
  import type { EstadoDaLicenca, Hardware } from "../lib/ponte";

  type Tema = "claro" | "escuro";

  let {
    modo = "completa",
    hardware,
    licenca,
    tema,
    semFundo,
    onescolherTema,
    onalternarFundo,
    onlicenca,
    onfim,
  }: {
    modo?: ModoDasBoasVindas;
    hardware: Hardware | null;
    licenca: EstadoDaLicenca | null;
    tema: Tema;
    semFundo: boolean;
    onescolherTema: (t: Tema) => void;
    onalternarFundo: () => void;
    onlicenca: (l: EstadoDaLicenca) => void;
    onfim: () => void;
  } = $props();

  type Passo = "oi" | "como" | "pc" | "licenca" | "visual" | "pronto";
  const PASSOS = $derived<Passo[]>(modo === "licenca" ? ["licenca"] : ["oi", "como", "pc", "licenca", "visual", "pronto"]);

  let indice = $state(0);
  let direcao = $state(1);
  const passo = $derived<Passo>(PASSOS[indice]);
  const ultimo = $derived(indice === PASSOS.length - 1);
  const parado = typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const dur = (ms: number) => (parado ? 0 : ms);

  function ir(n: number) {
    if (n < 0 || n >= PASSOS.length) return;
    direcao = n > indice ? 1 : -1;
    indice = n;
  }
  const avancar = () => (ultimo ? onfim() : ir(indice + 1));

  function teclado(e: KeyboardEvent) {
    // Enter dentro do campo da chave é da chave.
    if ((e.target as HTMLElement)?.tagName === "TEXTAREA") return;
    if (e.key === "ArrowRight" || e.key === "Enter") avancar();
    if (e.key === "ArrowLeft") ir(indice - 1);
  }

  const nome = (s: string) =>
    s
      .replace(/\((R|TM)\)/gi, "")
      .replace(/\s+CPU.*$/i, "")
      .replace(/\s+@.*$/, "")
      .replace(/\s+/g, " ")
      .trim();
  const telaPrincipal = $derived(hardware?.telas.find((t) => t.principal) ?? hardware?.telas[0] ?? null);
  const pecas = $derived.by(() => {
    if (!hardware) return [];
    const l: { rotulo: string; valor: string; nota?: string; atencao?: boolean }[] = [
      { rotulo: "Processador", valor: nome(hardware.processador), nota: `${hardware.nucleos_logicos} núcleos lógicos` },
      { rotulo: "Placa de vídeo", valor: nome(hardware.placa.nome), nota: hardware.placa.driver ? `driver ${hardware.placa.driver}` : "driver não lido" },
      { rotulo: "Memória", valor: hardware.memoria_gb != null ? `${hardware.memoria_gb} GB` : "não lida" },
    ];
    const t = telaPrincipal;
    if (t) {
      const abaixo = t.hz_maximo > t.hz_atual + 1;
      l.push({
        rotulo: "Monitor",
        valor: `${t.largura}×${t.altura} · ${t.hz_atual} Hz`,
        nota: abaixo ? `Ele aguenta ${t.hz_maximo} Hz: eu subo isso na área Tela, e é o ganho que mais se sente.` : "Já está na taxa máxima.",
        atencao: abaixo,
      });
    }
    return l;
  });

  // "Como eu trabalho" como uma história: o tempo de cada quadro com trancos (antes), a leitura passando, os picos
  // achatando (o ajuste) e as duas linhas juntas (a prova). É ilustração, e a tela diz isso: número de verdade só sai
  // da medição desta máquina.
  const PONTOS = 48;
  const PICOS: Record<number, number> = { 5: 30, 6: 52, 13: 26, 19: 44, 20: 18, 27: 38, 33: 22, 34: 48, 41: 30 };
  const comTrancos = Array.from({ length: PONTOS }, (_, i) => PICOS[i] ?? 104 + Math.sin(i * 1.7) * 7 + Math.cos(i * 0.6) * 5);
  const liso = Array.from({ length: PONTOS }, (_, i) => 100 + Math.sin(i * 0.5) * 2.2);
  const caminho = (v: number[]) => v.map((y, i) => `${i ? "L" : "M"}${((i * 640) / (PONTOS - 1)).toFixed(1)} ${y.toFixed(1)}`).join(" ");
  const ANTES = caminho(comTrancos);
  const DEPOIS = caminho(liso);
  const BATIDAS = [
    { titulo: "Eu meço", texto: "Leio cada quadro do jogo enquanto você joga, sem nada por cima dele." },
    { titulo: "Eu ajusto", texto: "Só o que rende nesta máquina. Tudo tem desfazer." },
    { titulo: "Eu provo", texto: "Antes e depois no mesmo jogo. Piorou, eu volto." },
  ];
  /** 0 medindo, 1 ajustando, 2 provado. Sem movimento, já começa provado. */
  let fase = $state(0);
  $effect(() => {
    if (passo !== "como") return;
    fase = parado ? 2 : 0;
    if (parado) return;
    const t1 = setTimeout(() => (fase = 1), 2300);
    const t2 = setTimeout(() => (fase = 2), 3900);
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
    };
  });

</script>

<svelte:window onkeydown={teclado} />

<div class="veu" class:parado transition:fade={{ duration: dur(260) }}>
  <div class="aurora" aria-hidden="true"><span class="a1"></span><span class="a2"></span><span class="a3"></span></div>

  <div
    class="folha"
    role="dialog"
    aria-modal="true"
    aria-label={modo === "licenca" ? "Ativar a licença" : "Boas-vindas ao Otimiza"}
    in:scale={{ duration: dur(420), start: 0.94, easing: cubicOut }}
  >
    <header class="cima">
      {#if PASSOS.length > 1}
        <div class="pontos" aria-label={`Passo ${indice + 1} de ${PASSOS.length}`}>
          {#each PASSOS as p, i (p)}
            <button class="ponto-passo" class:feito={i < indice} class:atual={i === indice} aria-label={`Ir para o passo ${i + 1}`} onclick={() => ir(i)}></button>
          {/each}
        </div>
        {#if !ultimo}<button class="pular" onclick={onfim}>Pular apresentação</button>{/if}
      {:else}
        <span></span>
        <button class="fechar" aria-label="Fechar" onclick={onfim}><Icone nome="fechar" tamanho={16} /></button>
      {/if}
    </header>

    <div class="palco">
      {#key passo}
        <section
          class="passo"
          in:fly={{ x: 48 * direcao, duration: dur(460), delay: dur(120), easing: cubicOut }}
          out:fly={{ x: -36 * direcao, duration: dur(220), easing: cubicOut }}
        >
          {#if passo === "oi"}
            <div class="centro">
              <div class="oti-grande" in:scale={{ duration: dur(700), start: 0.4, delay: dur(200), easing: backOut }}>
                <Oti tamanho={168} humor="feliz" gesto="acenar" segueCursor={false} />
              </div>
              <h1 in:fly={{ y: 16, duration: dur(500), delay: dur(500) }}>Oi, eu sou o Oti.</h1>
              <p class="lead" in:fly={{ y: 16, duration: dur(500), delay: dur(650) }}>
                Eu cuido do seu PC para o jogo rodar liso, e provo com número tudo o que eu mudo.
              </p>
            </div>
          {:else if passo === "como"}
            <h2>Como eu trabalho</h2>
            <p class="sub">O que você sente como "travando" é isto: alguns quadros demorando muito mais que os outros.</p>
            <figure class="historia" data-fase={fase} in:fade={{ duration: dur(400), delay: dur(150) }}>
              <figcaption>
                <span>Tempo de cada quadro</span>
                <span class="etiqueta">{fase === 2 ? "antes e depois" : fase === 1 ? "ajustando" : "medindo"}</span>
                <span class="ilustracao">ilustração</span>
              </figcaption>
              <svg viewBox="0 0 640 140" preserveAspectRatio="none" aria-hidden="true">
                <line class="guia" x1="0" x2="640" y1="100" y2="100" />
                <path class="fantasma" d={ANTES} />
                <path class="linha" d={fase === 0 ? ANTES : DEPOIS} />
                <rect class="leitura" x="0" y="0" width="70" height="140" />
              </svg>
              <div class="legenda">
                <span class="tag antes">com trancos</span>
                <span class="tag depois">liso</span>
              </div>
            </figure>
            <ol class="batidas">
              {#each BATIDAS as b, i (b.titulo)}
                <li class:acesa={fase >= i} class:atual={fase === i}>
                  <span class="marca">{i + 1}</span>
                  <strong>{b.titulo}</strong>
                  <span>{b.texto}</span>
                </li>
              {/each}
            </ol>
          {:else if passo === "pc"}
            <h2>Conhecendo o seu PC</h2>
            <p class="sub">Isto foi lido agora, nesta máquina.</p>
            {#if pecas.length}
              <ul class="pecas">
                {#each pecas as p, i (p.rotulo)}
                  <li class:atencao={p.atencao} in:fly={{ x: 24, duration: dur(420), delay: dur(200 + i * 160), easing: cubicOut }}>
                    <span class="rotulo">{p.rotulo}</span>
                    <span class="valor">{p.valor}</span>
                    {#if p.nota}<span class="nota">{p.nota}</span>{/if}
                  </li>
                {/each}
              </ul>
            {:else}
              <div class="lendo"><span></span><span></span><span></span></div>
              <p class="sub">Lendo o hardware…</p>
            {/if}
          {:else if passo === "licenca"}
            <h2>{licenca?.ativa ? "Sua licença" : "Ative a sua licença"}</h2>
            <p class="sub">
              {licenca?.ativa
                ? "Está tudo certo por aqui."
                : "Sem licença você mede e vê o diagnóstico. Com ela, o Otimiza ajusta, liga o Turbo e o Jogar."}
            </p>
            <div class="caixa-licenca"><AtivarLicenca {licenca} onativada={onlicenca} /></div>
          {:else if passo === "visual"}
            <h2>Do seu jeito</h2>
            <p class="sub">Dá para trocar depois, na barra de cima.</p>
            <div class="temas">
              {#each [["claro", "Claro"], ["escuro", "Escuro"]] as [id, rotulo], i (id)}
                <button
                  class="tema-op"
                  class:escolhido={tema === id}
                  aria-pressed={tema === id}
                  onclick={() => onescolherTema(id as Tema)}
                  in:fly={{ y: 20, duration: dur(420), delay: dur(200 + i * 120) }}
                >
                  <span class="mini" data-tema={id}><span></span><span></span><span></span></span>
                  {rotulo}
                </button>
              {/each}
              <button class="tema-op" class:escolhido={semFundo} aria-pressed={semFundo} onclick={onalternarFundo} in:fly={{ y: 20, duration: dur(420), delay: dur(440) }}>
                <span class="mini sem-fundo"><span></span><span></span><span></span></span>
                Sem fundo
              </button>
            </div>
            <p class="rodape">"Sem fundo" mostra a sua área de trabalho entre os cards.</p>
          {:else}
            <div class="centro">
              <div class="oti-grande" in:scale={{ duration: dur(700), start: 0.5, delay: dur(150), easing: backOut }}>
                <Oti tamanho={140} humor="feliz" gesto="comemorar" segueCursor={false} />
              </div>
              <h1 in:fly={{ y: 16, duration: dur(500), delay: dur(400) }}>Tudo pronto.</h1>
              <p class="lead" in:fly={{ y: 16, duration: dur(500), delay: dur(550) }}>
                Toque em <strong>Jogar</strong> na capa do seu jogo: eu ligo o que rende nele e, quando você fechar, mostro como foi
                a partida.
              </p>
            </div>
          {/if}
        </section>
      {/key}
    </div>

    <footer class="baixo">
      {#if indice > 0}<button class="btn" onclick={() => ir(indice - 1)}>Voltar</button>{:else}<span></span>{/if}
      <button class="btn primario grande" onclick={avancar}>
        {#if modo === "licenca"}{licenca?.ativa ? "Fechar" : "Agora não"}
        {:else if passo === "oi"}Começar
        {:else if passo === "licenca" && !licenca?.ativa}Continuar sem licença
        {:else if ultimo}Ir para o Otimiza
        {:else}Continuar{/if}
      </button>
    </footer>
  </div>
</div>

<style>
  .veu {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(14 16 26 / 0.38);
    backdrop-filter: blur(18px) saturate(1.2);
    overflow: hidden;
  }
  /* A luz que anda atrás da folha: três manchas de cor desfocadas, devagar. */
  .aurora span {
    position: absolute;
    width: 46vmax;
    height: 46vmax;
    border-radius: 50%;
    filter: blur(70px);
    opacity: 0.55;
    animation: deriva 18s ease-in-out infinite alternate;
  }
  .a1 {
    left: -10vmax;
    top: -12vmax;
    background: #7c8cff;
  }
  .a2 {
    right: -14vmax;
    top: 10vmax;
    background: #ff9db5;
    animation-delay: -6s !important;
  }
  .a3 {
    left: 20vmax;
    bottom: -20vmax;
    background: #7fe3d0;
    animation-delay: -12s !important;
  }
  @keyframes deriva {
    to {
      transform: translate(8vmax, 6vmax) scale(1.15);
    }
  }
  .folha {
    position: relative;
    display: grid;
    grid-template-rows: auto 1fr auto;
    width: min(760px, 100%);
    height: min(600px, calc(100vh - 48px));
    border-radius: 28px;
    border: 1px solid rgb(255 255 255 / 0.6);
    background: rgb(250 249 247 / 0.86);
    box-shadow:
      0 1px 0 rgb(255 255 255 / 0.6) inset,
      0 40px 90px rgb(20 20 40 / 0.35);
    color: #1d1d1f;
    overflow: hidden;
  }
  :global(:root[data-tema="noite"]) .folha {
    background: rgb(32 32 36 / 0.9);
    border-color: rgb(255 255 255 / 0.12);
    color: #f5f5f7;
  }
  .cima {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px 0;
  }
  .pontos {
    display: flex;
    gap: 6px;
  }
  .ponto-passo {
    width: 8px;
    height: 8px;
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: currentColor;
    opacity: 0.18;
    cursor: pointer;
    transition:
      width 320ms cubic-bezier(0.3, 1.4, 0.5, 1),
      opacity 200ms;
  }
  .ponto-passo.feito {
    opacity: 0.45;
  }
  .ponto-passo.atual {
    width: 26px;
    opacity: 0.9;
  }
  .pular,
  .fechar {
    border: 0;
    background: none;
    color: inherit;
    opacity: 0.6;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .pular:hover,
  .fechar:hover {
    opacity: 1;
  }
  .palco {
    position: relative;
    display: grid;
    overflow: hidden;
  }
  .passo {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 18px 40px 8px;
    overflow-y: auto;
  }
  .centro {
    margin: auto;
    display: grid;
    justify-items: center;
    gap: 12px;
    text-align: center;
    max-width: 460px;
  }
  .oti-grande {
    animation: flutuar 4.5s ease-in-out infinite;
  }
  @keyframes flutuar {
    50% {
      transform: translateY(-8px);
    }
  }
  h1 {
    margin: 8px 0 0;
    font-family: var(--fonte-titulo);
    font-size: 34px;
    font-weight: 650;
    letter-spacing: -0.02em;
  }
  h2 {
    margin: 0;
    font-family: var(--fonte-titulo);
    font-size: 26px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }
  .lead {
    margin: 0;
    font-size: 16px;
    line-height: 1.5;
    opacity: 0.78;
  }
  .sub,
  .rodape {
    margin: 0;
    font-size: 13.5px;
    opacity: 0.66;
  }
  .rodape {
    margin-top: auto;
    padding-top: 6px;
  }
  .historia {
    margin: 10px 0 0;
    padding: 16px 18px 12px;
    border-radius: 20px;
    background: rgb(255 255 255 / 0.62);
    border: 1px solid rgb(0 0 0 / 0.06);
  }
  :global(:root[data-tema="noite"]) .historia {
    background: rgb(255 255 255 / 0.05);
    border-color: rgb(255 255 255 / 0.1);
  }
  .historia figcaption {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
  }
  .historia figcaption > span:first-child {
    opacity: 0.6;
  }
  .etiqueta {
    padding: 2px 9px;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.07);
    font-size: 11.5px;
    font-weight: 600;
  }
  :global(:root[data-tema="noite"]) .etiqueta {
    background: rgb(255 255 255 / 0.12);
  }
  .ilustracao {
    margin-left: auto;
    padding: 1px 8px;
    border: 1px dashed currentColor;
    border-radius: 999px;
    font-size: 11px;
    opacity: 0.5;
  }
  .historia svg {
    display: block;
    width: 100%;
    height: 150px;
    margin-top: 6px;
    overflow: visible;
  }
  .guia {
    stroke: currentColor;
    stroke-opacity: 0.12;
    stroke-dasharray: 4 6;
  }
  .linha,
  .fantasma {
    fill: none;
    stroke-width: 3;
    stroke-linejoin: round;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
  }
  /* Âmbar com os picos enquanto mede; verde e liso depois do ajuste. A forma muda pelo próprio caminho (`d`). */
  .linha {
    stroke: var(--laranja, #c2730f);
    stroke-dasharray: 2400;
    stroke-dashoffset: 2400;
    animation: desenhar 2s cubic-bezier(0.4, 0, 0.2, 1) forwards;
    transition:
      d 1.1s cubic-bezier(0.5, 0, 0.2, 1),
      stroke 0.8s ease;
  }
  [data-fase="1"] .linha,
  [data-fase="2"] .linha {
    stroke: var(--verde, #34c759);
  }
  .fantasma {
    stroke: var(--laranja, #c2730f);
    opacity: 0;
    transition: opacity 0.6s ease;
  }
  [data-fase="2"] .fantasma {
    opacity: 0.3;
  }
  .leitura {
    fill: currentColor;
    opacity: 0.07;
    animation: varrer 2s cubic-bezier(0.4, 0, 0.2, 1) forwards;
  }
  @keyframes desenhar {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes varrer {
    from {
      transform: translateX(-70px);
    }
    90% {
      opacity: 0.07;
    }
    to {
      transform: translateX(640px);
      opacity: 0;
    }
  }
  .legenda {
    display: flex;
    gap: 14px;
    margin-top: 6px;
    font-size: 11.5px;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    opacity: 0.35;
    transition: opacity 0.5s ease;
  }
  .tag::before {
    content: "";
    width: 14px;
    height: 3px;
    border-radius: 2px;
  }
  .tag.antes::before {
    background: var(--laranja, #c2730f);
  }
  .tag.depois::before {
    background: var(--verde, #34c759);
  }
  [data-fase="0"] .tag.antes,
  [data-fase="1"] .tag.depois,
  [data-fase="2"] .tag {
    opacity: 0.85;
  }
  .batidas {
    list-style: none;
    margin: 16px 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 18px;
  }
  .batidas li {
    display: grid;
    align-content: start;
    gap: 4px;
    padding-top: 12px;
    border-top: 2px solid rgb(127 127 127 / 0.2);
    opacity: 0.4;
    transition:
      opacity 0.5s ease,
      border-color 0.5s ease;
  }
  .batidas li.acesa {
    opacity: 1;
  }
  .batidas li.atual {
    border-top-color: currentColor;
  }
  .marca {
    font-size: 11.5px;
    font-weight: 600;
    opacity: 0.55;
    font-variant-numeric: tabular-nums;
  }
  .batidas strong {
    font-size: 15px;
  }
  .batidas li > span:last-child {
    font-size: 12.5px;
    line-height: 1.45;
    opacity: 0.72;
  }
  .parado .linha,
  .parado .leitura {
    animation: none;
    stroke-dashoffset: 0;
    transition: none;
  }
  .parado .leitura {
    opacity: 0;
  }
  .pecas {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .pecas li {
    display: grid;
    grid-template-columns: 130px 1fr;
    column-gap: 16px;
    row-gap: 2px;
    padding: 12px 16px;
    border-radius: 16px;
    background: rgb(255 255 255 / 0.62);
    border: 1px solid rgb(0 0 0 / 0.06);
  }
  .pecas li.atencao {
    border-color: color-mix(in srgb, var(--laranja, #c2730f) 45%, transparent);
  }
  .rotulo {
    font-size: 12.5px;
    opacity: 0.6;
    align-self: center;
  }
  .valor {
    font-weight: 600;
    font-size: 15px;
  }
  .nota {
    grid-column: 2;
    font-size: 12.5px;
    opacity: 0.68;
  }
  .atencao .nota {
    color: var(--laranja, #c2730f);
    opacity: 1;
  }
  .lendo {
    display: flex;
    gap: 8px;
    margin: 40px auto 0;
  }
  .lendo span {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.3;
    animation: piscar 1.2s ease-in-out infinite;
  }
  .lendo span:nth-child(2) {
    animation-delay: 0.2s;
  }
  .lendo span:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes piscar {
    50% {
      opacity: 0.9;
      transform: translateY(-4px);
    }
  }
  .caixa-licenca {
    margin-top: 8px;
    padding: 18px;
    border-radius: 20px;
    background: rgb(255 255 255 / 0.5);
    border: 1px solid rgb(0 0 0 / 0.06);
    --texto: currentColor;
  }
  :global(:root[data-tema="noite"]) .caixa-licenca {
    background: rgb(255 255 255 / 0.05);
    border-color: rgb(255 255 255 / 0.1);
    --superficie-2: rgb(255 255 255 / 0.08);
    --linha: rgb(255 255 255 / 0.1);
    --linha-forte: rgb(255 255 255 / 0.18);
  }
  .temas {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    margin-top: 12px;
  }
  .tema-op {
    display: grid;
    gap: 10px;
    justify-items: center;
    padding: 14px;
    border-radius: 20px;
    border: 2px solid transparent;
    background: rgb(255 255 255 / 0.62);
    color: inherit;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    transition:
      border-color 200ms,
      transform 200ms cubic-bezier(0.3, 1.5, 0.5, 1);
  }
  .tema-op:hover {
    transform: translateY(-2px);
  }
  .tema-op.escolhido {
    border-color: currentColor;
    box-shadow: 0 6px 18px rgb(0 0 0 / 0.12);
  }
  /* Miniatura da casca: céu e três cards. */
  .mini {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 5px;
    width: 100%;
    aspect-ratio: 16 / 10;
    padding: 10px;
    box-sizing: border-box;
    border-radius: 12px;
    background: linear-gradient(160deg, #cfc7bd, #e9c9c6);
  }
  .mini span {
    border-radius: 5px;
    background: rgb(255 255 255 / 0.7);
  }
  .mini span:first-child {
    grid-column: span 2;
  }
  .mini[data-tema="escuro"] {
    background: linear-gradient(160deg, #3a3530, #4b3a3c);
  }
  .mini[data-tema="escuro"] span {
    background: rgb(40 40 44 / 0.85);
  }
  .mini.sem-fundo {
    background:
      repeating-conic-gradient(rgb(0 0 0 / 0.06) 0 25%, transparent 0 50%) 0 0 / 12px 12px,
      linear-gradient(135deg, #1e3a8a, #0ea5e9 55%, #f59e0b);
  }
  .mini.sem-fundo span {
    background: rgb(250 249 247 / 0.92);
  }
  .baixo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 24px 22px;
  }
  /* Os botões neutros da folha: as áreas trocam os tokens de cor, e aqui eles não valem. */
  .baixo .btn:not(.primario) {
    background: rgb(255 255 255 / 0.7);
    border: 1px solid rgb(0 0 0 / 0.1);
    color: inherit;
  }
  :global(:root[data-tema="noite"]) .baixo .btn:not(.primario) {
    background: rgb(255 255 255 / 0.08);
    border-color: rgb(255 255 255 / 0.14);
  }
  .btn.grande {
    min-width: 200px;
    height: 44px;
    font-size: 15px;
  }
  .parado .aurora span,
  .parado .oti-grande,
  .parado .lendo span {
    animation: none;
  }
  @media (max-width: 760px) {
    .batidas,
    .temas {
      grid-template-columns: 1fr;
    }
    .passo {
      padding: 14px 22px 8px;
    }
  }
</style>
