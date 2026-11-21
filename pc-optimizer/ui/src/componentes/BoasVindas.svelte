<script lang="ts" module>
  export type ModoDasBoasVindas = "completa" | "licenca";
  /** Os Ajustes pedem a primeira execução de novo por este evento (ela mora no App, por cima de tudo). */
  export const ABRIR_BOAS_VINDAS = "otimiza:boas-vindas";
</script>

<script lang="ts">
  // A primeira execução guiada (1.0.12, Frente 1.5): um fluxo só, com progresso, do zero até a prova.
  //   Oi → 1. Licença → 2. Medir antes → 3. Primeiro ajuste → 4. Provar → Pronto.
  // Cada passo usa a peça de verdade do app (a ativação, o laboratório, o Otimizar agora): o que se faz aqui fica feito
  // lá. Dá para pular qualquer passo e voltar depois; nada aqui afirma o que não foi lido ou medido. O atalho "Ativar
  // licença" da barra abre só o passo da licença. Com "reduzir movimento" ou jogo aberto, nada se mexe.
  import { tick } from "svelte";
  import { fade } from "svelte/transition";
  import Oti from "./Oti.svelte";
  import Brilho from "./marca/Brilho.svelte";
  import Icone from "./Icone.svelte";
  import Botao from "./base/Botao.svelte";
  import Estado from "./base/Estado.svelte";
  import AtivarLicenca from "./AtivarLicenca.svelte";
  import MedicaoGuiada from "./MedicaoGuiada.svelte";
  import OtimizarAgora from "./OtimizarAgora.svelte";
  import type { EstadoDaLicenca, Hardware, ResultadoDoLab } from "../lib/ponte";

  type Tema = "claro" | "escuro";

  let {
    modo = "completa",
    hardware,
    licenca,
    tema,
    onescolherTema,
    onlicenca,
    onfim,
  }: {
    modo?: ModoDasBoasVindas;
    hardware: Hardware | null;
    licenca: EstadoDaLicenca | null;
    tema: Tema;
    onescolherTema: (t: Tema) => void;
    onlicenca: (l: EstadoDaLicenca) => void;
    onfim: () => void;
  } = $props();

  type Passo = "oi" | "licenca" | "medir" | "ajustar" | "provar" | "pronto";
  /** Os quatro passos que contam no progresso (o "oi" e o "pronto" são a porta e a saída). */
  const ETAPAS: { id: Passo; nome: string }[] = [
    { id: "licenca", nome: "Licença" },
    { id: "medir", nome: "Medir" },
    { id: "ajustar", nome: "Ajustar" },
    { id: "provar", nome: "Provar" },
  ];
  const PASSOS = $derived<Passo[]>(modo === "licenca" ? ["licenca"] : ["oi", "licenca", "medir", "ajustar", "provar", "pronto"]);

  let indice = $state(0);
  const passo = $derived<Passo>(PASSOS[indice]);
  const ultimo = $derived(indice === PASSOS.length - 1);
  const etapa = $derived(ETAPAS.findIndex((e) => e.id === passo));
  const parado =
    typeof window !== "undefined" &&
    (window.matchMedia("(prefers-reduced-motion: reduce)").matches || document.documentElement.classList.contains("jogo-aberto"));
  const dur = (ms: number) => (parado ? 0 : ms);

  /** O que já foi feito em cada passo (para o progresso e para o "Pronto" dizer a verdade). */
  let lab = $state<ResultadoDoLab | null>(null);
  let ajustou = $state(false);
  const feito = $derived<Record<Passo, boolean>>({
    oi: true,
    licenca: !!licenca?.ativa,
    medir: (lab?.rodadas_antes ?? 0) > 0,
    ajustar: ajustou,
    provar: (lab?.rodadas_depois ?? 0) > 0,
    pronto: true,
  });

  let titulo: HTMLElement | undefined = $state();
  async function ir(n: number) {
    if (n < 0 || n >= PASSOS.length) return;
    indice = n;
    // O foco vai para o título do passo: quem usa leitor de tela ouve onde está.
    await tick();
    titulo?.focus();
  }
  const avancar = () => (ultimo ? onfim() : ir(indice + 1));

  function teclado(e: KeyboardEvent) {
    if (e.key === "Escape") onfim();
    // Setas só fora de campo, botão de escolha e rádio: ali elas são do controle.
    const alvo = e.target as HTMLElement;
    if (alvo?.closest("input, textarea, [role=radiogroup], [role=tablist]")) return;
    if (e.key === "ArrowRight") avancar();
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
    const l: { rotulo: string; valor: string; atencao?: string }[] = [
      { rotulo: "Processador", valor: nome(hardware.processador) },
      { rotulo: "Placa de vídeo", valor: nome(hardware.placa.nome) },
      { rotulo: "Memória", valor: hardware.memoria_gb != null ? `${hardware.memoria_gb} GB` : "não lida" },
    ];
    const t = telaPrincipal;
    if (t)
      l.push({
        rotulo: "Monitor",
        valor: `${t.hz_atual} Hz`,
        atencao: t.hz_maximo > t.hz_atual + 1 ? `aguenta ${t.hz_maximo} Hz` : undefined,
      });
    return l;
  });

  /** O texto do botão de seguir: diz o que acontece, e admite quando o passo foi pulado. */
  const rotuloDoSeguir = $derived.by(() => {
    if (modo === "licenca") return licenca?.ativa ? "Fechar" : "Agora não";
    if (passo === "oi") return "Começar";
    if (ultimo) return "Ir para o Otimiza";
    if (!feito[passo]) return passo === "licenca" ? "Continuar sem licença" : "Pular este passo";
    return "Continuar";
  });
</script>

<svelte:window onkeydown={teclado} />

<div class="veu" transition:fade={{ duration: dur(200) }}>
  <div
    class="folha"
    role="dialog"
    aria-modal="true"
    aria-label={modo === "licenca" ? "Ativar a licença" : "Boas-vindas ao Otimiza"}
  >
    <header class="cima">
      {#if modo === "completa"}
        <ol class="progresso" aria-label="Progresso da primeira execução">
          {#each ETAPAS as e, i (e.id)}
            <li class:atual={i === etapa} class:feito={feito[e.id]} aria-current={i === etapa ? "step" : undefined}>
              <button class="etapa" onclick={() => ir(PASSOS.indexOf(e.id))}>
                <span class="barra" aria-hidden="true"></span>
                <span class="nome-etapa"><span class="numero">{i + 1}</span> {e.nome}{#if feito[e.id]}<span class="so-leitor"> (feito)</span>{/if}</span>
              </button>
            </li>
          {/each}
        </ol>
      {:else}
        <span></span>
      {/if}
      <button class="fechar" aria-label={ultimo ? "Fechar" : "Fechar e terminar depois"} onclick={onfim}><Icone nome="fechar" tamanho={16} /></button>
    </header>

    <div class="palco">
      {#key passo}
        <section class="passo" aria-labelledby="passo-titulo" in:fade={{ duration: dur(220) }}>
          {#if passo === "oi"}
            <div class="abertura">
              <div class="cena" aria-hidden="true">
                <span class="cena-oti"><Oti tamanho={112} segueCursor rotulo="" /></span>
                <span class="cena-brilho"><Brilho tamanho={180} densidade={1.2} /></span>
              </div>
              <div class="abertura-texto">
                <p class="rotulo">Boas-vindas</p>
                <h1 id="passo-titulo" tabindex="-1" bind:this={titulo}>Oi, eu sou o Oti.</h1>
                <p class="lead">Em quatro passos eu deixo o seu PC pronto e provo com número o que mudou: ativar, medir, ajustar e provar.</p>
                <p class="apoio">Leva uns dez minutos com o seu jogo aberto. Dá para pular qualquer passo e voltar depois.</p>
                {#if pecas.length}
                  <dl class="pecas" aria-label="Lido agora neste PC">
                    {#each pecas as p (p.rotulo)}
                      <div>
                        <dt>{p.rotulo}</dt>
                        <dd>{p.valor}{#if p.atencao}<span class="atencao"><span class="ponto" data-tom="atencao" aria-hidden="true"></span>{p.atencao}</span>{/if}</dd>
                      </div>
                    {/each}
                  </dl>
                {:else}
                  <Estado tipo="lendo" linhas={2} titulo="Lendo o hardware deste PC" />
                {/if}
                <div class="tema" role="group" aria-label="Tema">
                  <span class="apoio">Tema</span>
                  <div class="segmento">
                    <button aria-pressed={tema === "escuro"} onclick={() => onescolherTema("escuro")}>Escuro</button>
                    <button aria-pressed={tema === "claro"} onclick={() => onescolherTema("claro")}>Claro</button>
                  </div>
                </div>
              </div>
            </div>
          {:else if passo === "licenca"}
            <p class="rotulo">Passo 1 de 4</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>{licenca?.ativa ? "Sua licença está ativa" : "Ative a sua licença"}</h2>
            <p class="sub">
              {licenca?.ativa
                ? "Está tudo certo por aqui. Siga para medir o seu jogo."
                : "Sem licença você mede e vê o diagnóstico. Com ela, o Otimiza ajusta, liga o Turbo e o Jogar."}
            </p>
            <div class="caixa"><AtivarLicenca {licenca} onativada={onlicenca} /></div>
          {:else if passo === "medir"}
            <p class="rotulo">Passo 2 de 4</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>Meça o jogo antes de mudar qualquer coisa</h2>
            <p class="sub">
              Abra o seu jogo e vá para um lugar que você conhece (o mesmo mapa, a mesma rua). Cada rodada mede 30 segundos, sem
              nada por cima do jogo. Três rodadas dão um resultado confiável.
            </p>
            <div class="caixa"><MedicaoGuiada lado="Antes" onmudou={(l) => (lab = l)} /></div>
          {:else if passo === "ajustar"}
            <p class="rotulo">Passo 3 de 4</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>O primeiro ajuste</h2>
            {#if licenca?.ativa}
              <p class="sub">Escolha o nível e aplique. Cada mudança fica no Histórico com o valor de antes e se desfaz num toque.</p>
              <div class="caixa ajuste"><OtimizarAgora onterminou={() => (ajustou = true)} /></div>
            {:else}
              <Estado tipo="vazio" titulo="O ajuste precisa da licença" fazer="Ative no passo 1 e volte aqui. A medição e a prova continuam livres.">
                {#snippet acao()}<Botao pequeno onclick={() => ir(PASSOS.indexOf("licenca"))}>Voltar à licença</Botao>{/snippet}
              </Estado>
            {/if}
          {:else if passo === "provar"}
            <p class="rotulo">Passo 4 de 4</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>Prove o que mudou</h2>
            <p class="sub">
              Volte ao mesmo lugar do jogo e meça de novo. Com rodadas dos dois lados, o Otimiza compara com a margem de cada um e só
              diz "melhorou" quando a diferença passa do acaso.
            </p>
            <div class="caixa"><MedicaoGuiada lado="Depois" onmudou={(l) => (lab = l)} /></div>
          {:else}
            <div class="abertura">
              <div class="cena pequena" aria-hidden="true">
                <span class="cena-oti"><Oti tamanho={88} humor="feliz" rotulo="" /></span>
                <span class="cena-brilho"><Brilho tamanho={140} densidade={1.4} semente={11} /></span>
              </div>
              <div class="abertura-texto">
                <p class="rotulo">Pronto</p>
                <h1 id="passo-titulo" tabindex="-1" bind:this={titulo}>{ETAPAS.every((e) => feito[e.id]) ? "Tudo feito." : "Por agora, é isso."}</h1>
                <ul class="resumo">
                  {#each ETAPAS as e (e.id)}
                    <li>
                      <span class="ponto" data-tom={feito[e.id] ? "ok" : "atencao"} aria-hidden="true"></span>
                      <span>
                        <strong>{e.nome}</strong>
                        {#if e.id === "licenca"}{feito.licenca ? "ativa" : "não ativada: dá para ativar pela barra de cima"}
                        {:else if e.id === "medir"}{feito.medir ? `${lab?.rodadas_antes} ${lab?.rodadas_antes === 1 ? "rodada" : "rodadas"} antes` : "não medido: a área Prova faz isso quando quiser"}
                        {:else if e.id === "ajustar"}{feito.ajustar ? "aplicado: está no Histórico" : "não aplicado: o Turbo tem o Otimizar agora"}
                        {:else}{feito.provar ? `${lab?.rodadas_depois} ${lab?.rodadas_depois === 1 ? "rodada" : "rodadas"} depois: o resultado fica na Prova` : "não provado: meça depois na área Prova"}{/if}
                      </span>
                    </li>
                  {/each}
                </ul>
                <p class="apoio">Daqui em diante, toque em <strong>Jogar</strong> na capa do seu jogo: eu ligo o que rende nele e mostro como foi a partida.</p>
              </div>
            </div>
          {/if}
        </section>
      {/key}
    </div>

    <footer class="baixo">
      {#if indice > 0 && modo === "completa"}<Botao onclick={() => ir(indice - 1)}>Voltar</Botao>{:else}<span></span>{/if}
      <Botao variante={feito[passo] || passo === "oi" || ultimo ? "principal" : "contorno"} onclick={avancar}>{rotuloDoSeguir}</Botao>
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
    padding: var(--e-5);
    background: var(--veu);
    backdrop-filter: var(--desfoque);
  }
  .folha {
    display: grid;
    grid-template-rows: auto 1fr auto;
    width: min(780px, 100%);
    height: min(620px, calc(100vh - 48px));
    border: 1px solid var(--linha-forte);
    border-radius: var(--raio-4);
    background: var(--superficie);
    box-shadow: var(--fio), var(--sombra-3);
    color: var(--tinta);
    overflow: hidden;
  }
  .cima {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--e-4);
    padding: var(--e-4) var(--e-4) 0 var(--e-6);
  }
  /* O progresso: quatro etapas com nome, cada uma com o seu traço. Feita = traço cheio; a de agora = texto cheio. */
  .progresso {
    flex: 1;
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--e-2);
    max-width: 520px;
  }
  .etapa {
    display: grid;
    gap: 6px;
    width: 100%;
    padding: 6px 0;
    border: 0;
    background: none;
    color: var(--tinta-3);
    text-align: left;
    cursor: pointer;
  }
  .barra {
    height: 3px;
    border-radius: 2px;
    background: var(--superficie-3);
    transition: background var(--mov-medio) ease;
  }
  .nome-etapa {
    font-size: var(--t-apoio);
    font-weight: 500;
  }
  .nome-etapa .numero {
    letter-spacing: 0;
  }
  .feito .barra {
    background: var(--tinta-3);
  }
  .atual .barra {
    background: var(--tinta);
  }
  .atual .etapa,
  .feito .etapa {
    color: var(--tinta);
  }
  .fechar {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--tinta-3);
    cursor: pointer;
  }
  .fechar:hover {
    background: var(--acao-suave);
    color: var(--tinta);
  }
  .palco {
    min-height: 0;
    overflow: auto;
    padding: var(--e-5) var(--e-6) var(--e-4);
  }
  .passo {
    display: grid;
    gap: var(--e-3);
    align-content: start;
  }
  h1,
  h2 {
    font-family: var(--fonte-titulo);
    font-weight: 700;
    letter-spacing: var(--rastreio-titulo);
    line-height: 1.08;
  }
  h1 {
    font-size: var(--t-destaque);
  }
  h2 {
    font-size: var(--t-titulo-area);
  }
  h1:focus,
  h2:focus {
    outline: none;
  }
  .sub {
    max-width: 62ch;
    color: var(--tinta-2);
    line-height: 1.5;
  }
  .lead {
    max-width: 46ch;
    font-size: var(--t-subtitulo);
    line-height: 1.45;
  }
  .caixa {
    margin-top: var(--e-2);
  }
  .caixa.ajuste :global(.secao) {
    padding-top: 0;
    border-top: 0;
  }
  .caixa.ajuste :global(.secao > h2) {
    display: none;
  }
  /* A abertura e o fim: a cena da marca à esquerda (o Oti olhando o brilho), o texto à direita. */
  .abertura {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: var(--e-6);
    align-items: center;
    min-height: 100%;
  }
  .abertura-texto {
    display: grid;
    gap: var(--e-3);
    justify-items: start;
  }
  .cena {
    position: relative;
    width: 220px;
    height: 220px;
  }
  .cena-oti {
    position: absolute;
    left: 8px;
    bottom: 24px;
  }
  .cena-brilho {
    position: absolute;
    right: -24px;
    top: -6px;
  }
  .cena.pequena .cena-oti {
    left: 24px;
    bottom: 40px;
  }
  .pecas {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--e-2) var(--e-5);
    width: 100%;
    margin: var(--e-2) 0 0;
    padding: var(--e-3) 0 0;
    border-top: 1px solid var(--linha);
  }
  .pecas dt {
    color: var(--tinta-2);
    font-size: var(--t-apoio);
  }
  .pecas dd {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--e-2);
    margin: 0;
    font-size: var(--t-linha);
    font-weight: 500;
  }
  .atencao {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--tinta-2);
    font-size: var(--t-apoio);
    font-weight: 400;
  }
  .tema {
    display: flex;
    align-items: center;
    gap: var(--e-3);
  }
  .resumo {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--e-2);
    font-size: var(--t-linha);
  }
  .resumo li {
    display: flex;
    align-items: baseline;
    gap: var(--e-2);
  }
  .resumo strong {
    margin-right: 4px;
  }
  .baixo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--e-4) var(--e-6) var(--e-5);
    border-top: 1px solid var(--linha);
  }
  .baixo :global(.btn) {
    min-height: 40px;
    padding: 0 var(--e-5);
  }
  .so-leitor {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  @media (max-width: 760px) {
    .abertura {
      grid-template-columns: 1fr;
    }
    .cena {
      height: 150px;
    }
  }
</style>
