<script lang="ts" module>
  export type ModoDasBoasVindas = "completa" | "licenca";
  /** Os Ajustes pedem a primeira execução de novo por este evento (ela mora no App, por cima de tudo). */
  export const ABRIR_BOAS_VINDAS = "otimiza:boas-vindas";
</script>

<script lang="ts">
  // A primeira execução guiada (1.0.12, Frente 1.5; 1.0.13 devolveu a história e a escolha de fundo, decisão do dono):
  // um fluxo só, com progresso, do zero até a prova.
  //   Oi → 1. Conhecer (a história) → 2. Licença → 3. Medir antes → 4. Primeiro ajuste → 5. Provar → 6. Aparência → Pronto.
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
  import { duracao } from "../lib/movimento";

  type Tema = "claro" | "escuro";

  let {
    modo = "completa",
    hardware,
    licenca,
    tema,
    onescolherTema,
    semFundo,
    onalternarFundo,
    onlicenca,
    onfim,
  }: {
    modo?: ModoDasBoasVindas;
    hardware: Hardware | null;
    licenca: EstadoDaLicenca | null;
    tema: Tema;
    onescolherTema: (t: Tema) => void;
    /** O fundo é a área de trabalho (a janela transparente). */
    semFundo: boolean;
    onalternarFundo: () => void;
    onlicenca: (l: EstadoDaLicenca) => void;
    onfim: () => void;
  } = $props();

  type Passo = "oi" | "como" | "licenca" | "medir" | "ajustar" | "provar" | "aparencia" | "pronto";
  /** Os quatro passos que contam no progresso (o "oi" e o "pronto" são a porta e a saída). */
  const ETAPAS: { id: Passo; nome: string }[] = [
    { id: "como", nome: "Conhecer" },
    { id: "licenca", nome: "Licença" },
    { id: "medir", nome: "Medir" },
    { id: "ajustar", nome: "Ajustar" },
    { id: "provar", nome: "Provar" },
    { id: "aparencia", nome: "Aparência" },
  ];
  const PASSOS = $derived<Passo[]>(modo === "licenca" ? ["licenca"] : ["oi", "como", "licenca", "medir", "ajustar", "provar", "aparencia", "pronto"]);

  let indice = $state(0);
  const passo = $derived<Passo>(PASSOS[indice]);
  const ultimo = $derived(indice === PASSOS.length - 1);
  const etapa = $derived(ETAPAS.findIndex((e) => e.id === passo));
  const dur = duracao;

  /** O que já foi feito em cada passo (para o progresso e para o "Pronto" dizer a verdade). */
  let lab = $state<ResultadoDoLab | null>(null);
  let ajustou = $state(false);
  const feito = $derived<Record<Passo, boolean>>({
    oi: true,
    // A história e a aparência contam como feitas quando a pessoa passa por elas.
    como: indice >= PASSOS.indexOf("como"),
    aparencia: indice >= PASSOS.indexOf("aparencia"),
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

  // "Como eu trabalho" como uma história: o tempo de cada quadro com trancos (antes), a leitura passando, os picos
  // achatando (o ajuste) e as duas linhas juntas (a prova). É ilustração, e a tela diz isso: número de verdade só sai
  // da medição desta máquina. Sem movimento (reduzir movimento ou jogo aberto), já começa provado.
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
  /** 0 medindo, 1 ajustando, 2 provado. */
  let fase = $state(0);
  $effect(() => {
    if (passo !== "como") return;
    const parado = dur(1) === 0;
    fase = parado ? 2 : 0;
    if (parado) return;
    const t1 = setTimeout(() => (fase = 1), 2300);
    const t2 = setTimeout(() => (fase = 2), 3900);
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
    };
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
                <p class="lead">Em poucos passos eu deixo o seu PC pronto e provo com número o que mudou: ativar, medir, ajustar e provar.</p>
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
              </div>
            </div>
          {:else if passo === "como"}
            <p class="rotulo">Passo {etapa + 1} de {ETAPAS.length}</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>Como eu trabalho</h2>
            <p class="sub">O que você sente como "travando" é isto: alguns quadros demorando muito mais que os outros.</p>
            <figure class="historia" class:parada={dur(1) === 0} data-fase={fase}>
              <figcaption>
                <span class="apoio">Tempo de cada quadro</span>
                <span class="etiqueta">{fase === 2 ? "antes e depois" : fase === 1 ? "ajustando" : "medindo"}</span>
                <span class="selo estimado ilustracao">ilustração</span>
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
                  <span class="numero marca-batida">{i + 1}</span>
                  <strong>{b.titulo}</strong>
                  <span class="apoio">{b.texto}</span>
                </li>
              {/each}
            </ol>
          {:else if passo === "licenca"}
            <p class="rotulo">Passo {etapa + 1} de {ETAPAS.length}</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>{licenca?.ativa ? "Sua licença está ativa" : "Ative a sua licença"}</h2>
            <p class="sub">
              {licenca?.ativa
                ? "Está tudo certo por aqui. Siga para medir o seu jogo."
                : "Sem licença você mede e vê o diagnóstico. Com ela, o Otimiza ajusta, liga o Turbo e o Jogar."}
            </p>
            <div class="caixa"><AtivarLicenca {licenca} onativada={onlicenca} /></div>
          {:else if passo === "medir"}
            <p class="rotulo">Passo {etapa + 1} de {ETAPAS.length}</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>Meça o jogo antes de mudar qualquer coisa</h2>
            <p class="sub">
              Abra o seu jogo e vá para um lugar que você conhece (o mesmo mapa, a mesma rua). Cada rodada mede 30 segundos, sem
              nada por cima do jogo. Três rodadas dão um resultado confiável.
            </p>
            <div class="caixa"><MedicaoGuiada lado="Antes" onmudou={(l) => (lab = l)} /></div>
          {:else if passo === "ajustar"}
            <p class="rotulo">Passo {etapa + 1} de {ETAPAS.length}</p>
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
            <p class="rotulo">Passo {etapa + 1} de {ETAPAS.length}</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>Prove o que mudou</h2>
            <p class="sub">
              Volte ao mesmo lugar do jogo e meça de novo. Com rodadas dos dois lados, o Otimiza compara com a margem de cada um e só
              diz "melhorou" quando a diferença passa do acaso.
            </p>
            <div class="caixa"><MedicaoGuiada lado="Depois" onmudou={(l) => (lab = l)} /></div>
          {:else if passo === "aparencia"}
            <p class="rotulo">Passo {etapa + 1} de {ETAPAS.length}</p>
            <h2 id="passo-titulo" tabindex="-1" bind:this={titulo}>Do seu jeito</h2>
            <p class="sub">O tema e o fundo atrás do painel. Dá para trocar depois, na barra de cima ("Fundo").</p>
            <div class="temas" role="group" aria-label="Aparência">
              {#each [["escuro", "Escuro"], ["claro", "Claro"]] as [id, rotulo] (id)}
                <button class="tema-op" aria-pressed={tema === id} onclick={() => onescolherTema(id as Tema)}>
                  <span class="mini" data-tema={id}><span></span><span></span><span></span></span>
                  {rotulo}
                </button>
              {/each}
              <button class="tema-op" aria-pressed={semFundo} onclick={onalternarFundo}>
                <span class="mini sem-fundo" data-tema={tema}><span></span><span></span><span></span></span>
                Sem fundo
              </button>
            </div>
            <p class="apoio">"Sem fundo" mostra a sua área de trabalho entre os cartões. Os outros fundos (cenas, fotos, a sua imagem) ficam no botão "Fundo".</p>
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
                  {#each ETAPAS.filter((e) => e.id !== "como" && e.id !== "aparencia") as e (e.id)}
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
    grid-template-columns: repeat(6, 1fr);
    gap: var(--e-2);
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
  /* A história: o tempo de cada quadro. Trancos em atenção, liso em ok: as duas cores aqui são estado. */
  .historia {
    margin: var(--e-1) 0 0;
    padding: var(--e-3) var(--e-4) 10px;
    border: 1px solid var(--linha);
    border-radius: var(--raio-3);
    background: var(--superficie-2);
  }
  .historia figcaption {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: var(--t-apoio);
  }
  .etiqueta {
    padding: 2px 9px;
    border-radius: var(--raio-pilula);
    background: var(--superficie-3);
    font-size: var(--t-apoio);
    font-weight: 600;
  }
  .ilustracao {
    margin-left: auto;
  }
  .historia svg {
    display: block;
    width: 100%;
    height: 96px;
    margin-top: 6px;
    overflow: visible;
    color: var(--tinta);
  }
  .guia {
    stroke: currentColor;
    stroke-opacity: 0.12;
    stroke-dasharray: 4 6;
  }
  .linha,
  .fantasma {
    fill: none;
    stroke-width: 2.5;
    stroke-linejoin: round;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
  }
  .linha {
    stroke: var(--atencao);
    stroke-dasharray: 2400;
    stroke-dashoffset: 2400;
    animation: desenhar 2s var(--curva-padrao) forwards;
    transition:
      d 1.1s var(--curva-padrao),
      stroke var(--mov-lento) ease;
  }
  [data-fase="1"] .linha,
  [data-fase="2"] .linha {
    stroke: var(--ok);
  }
  .fantasma {
    stroke: var(--atencao);
    opacity: 0;
    transition: opacity var(--mov-lento) ease;
  }
  [data-fase="2"] .fantasma {
    opacity: 0.3;
  }
  .leitura {
    fill: currentColor;
    opacity: 0.07;
    animation: varrer 2s var(--curva-padrao) forwards;
  }
  /* Parada (reduzir movimento ou jogo aberto): as linhas aparecem inteiras, sem a varredura. */
  .historia.parada .linha {
    stroke-dashoffset: 0;
  }
  .historia.parada .leitura {
    opacity: 0;
  }
  @keyframes desenhar {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes varrer {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(570px);
      opacity: 0;
    }
  }
  .legenda {
    display: flex;
    gap: 14px;
    margin-top: 6px;
    font-size: var(--t-apoio);
    color: var(--tinta-2);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .tag::before {
    content: "";
    width: 14px;
    height: 3px;
    border-radius: 2px;
  }
  .tag.antes::before {
    background: var(--atencao);
  }
  .tag.depois::before {
    background: var(--ok);
  }
  .batidas {
    list-style: none;
    margin: var(--e-1) 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--e-4);
  }
  .batidas li {
    display: grid;
    align-content: start;
    gap: 4px;
    padding-top: var(--e-3);
    border-top: 2px solid var(--superficie-3);
    color: var(--tinta-3);
    transition:
      color var(--mov-lento) ease,
      border-color var(--mov-lento) ease;
  }
  .batidas li.acesa {
    color: var(--tinta);
  }
  .batidas li.atual {
    border-top-color: var(--tinta);
  }
  .marca-batida {
    font-size: var(--t-apoio);
    color: var(--tinta-2);
  }
  .batidas strong {
    font-size: var(--t-subtitulo);
  }
  /* A aparência: três cartões com a miniatura do painel em cada um. */
  .temas {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--e-3);
    margin-top: var(--e-2);
  }
  .tema-op {
    display: grid;
    gap: 10px;
    justify-items: center;
    padding: var(--e-3);
    border: 1px solid var(--linha);
    border-radius: var(--raio-3);
    background: var(--superficie-2);
    color: var(--tinta);
    font-weight: 500;
    cursor: pointer;
    transition: border-color var(--mov-rapido) ease;
  }
  .tema-op:hover {
    border-color: var(--linha-forte);
  }
  .tema-op[aria-pressed="true"] {
    border-color: var(--tinta);
    box-shadow: inset 0 0 0 1px var(--tinta);
  }
  .mini {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 5px;
    width: 100%;
    aspect-ratio: 16 / 10;
    padding: 8px;
    border-radius: var(--raio-2);
    background: var(--preto);
    overflow: hidden;
  }
  .mini span {
    border-radius: 4px;
    background: var(--grafite-2);
    border: 1px solid rgb(255 255 255 / 0.08);
  }
  .mini span:first-child {
    grid-column: 1 / -1;
  }
  .mini[data-tema="claro"] {
    background: var(--papel);
  }
  .mini[data-tema="claro"] span {
    background: var(--branco);
    border-color: rgb(0 0 0 / 0.08);
  }
  /* Sem fundo: a área de trabalho (um degradê qualquer, como um papel de parede) aparece entre os cartões. */
  .mini.sem-fundo {
    background: linear-gradient(135deg, #3a4a63, #6b5a7a 50%, #a67c6d);
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
