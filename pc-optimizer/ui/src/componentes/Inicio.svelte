<script module lang="ts">
  /** "Pronto pra jogar": o que o Otimiza sabe conferir, item a item. Só entra o que foi lido (nada de item inventado);
   *  cada um diz o que falta e leva para onde se resolve. */
  export type ItemDaProntidao = { id: string; ok: boolean; texto: string; acao: () => void; botao?: string };
  export type Prontidao = { itens: ItemDaProntidao[]; ok: number; total: number; fracao: number };
  /** Uma leitura do bloco "Agora", com nome e valor por extenso. A barra só fica em cor quando há o que fazer. */
  export type Medidor = { nome: string; texto: string; fracao: number | null; alerta: boolean };
</script>

<script lang="ts">
  // A tela inicial: a grade de blocos de vidro (pronto pra jogar, jogos, partida, agora, durante o jogo e tela). Cada
  // bloco mostra um dado lido deste PC ou liga uma coisa que existe de verdade. O estado e as ações vêm da casca
  // (`App.svelte`). Saiu do `App.svelte` na 1.0.12 (Frente 1.2), sem mudar o visual.
  import Icone from "./Icone.svelte";
  import Oti, { type Humor } from "./Oti.svelte";
  import type { Temporada } from "../lib/temporadas";
  import type { IdDaArea } from "../lib/areas";
  import type { AbaDoSistema } from "../estado/navegacao.svelte";
  import { iniciais } from "../lib/capas";
  import { fraseDaComparacao, ofereceLiso, textoParaCompartilhar, type Cartao } from "../lib/partida";
  import type { Hardware, JogoAberto, JogoDaLoja, PainelDaBanda, PainelDaVarredura, ResumoDoJogo } from "../lib/ponte";

  let {
    temporada,
    humor,
    prontidao,
    saudacaoDaHora,
    jogo,
    jogoDoPlayer,
    partidaDoPlayer,
    mudancas,
    instalados,
    capaDaUltima,
    cartao,
    lisoGravado,
    gravandoLiso,
    hardware,
    medidores,
    curvaCpu,
    banda,
    varredura,
    telaPrincipal,
    onabrir,
    onjogar,
    onfotoFalhou,
    ondeixarLiso,
    onalternarBanda,
    onalternarVarredura,
  }: {
    temporada: Temporada | null;
    humor: Humor;
    prontidao: Prontidao;
    saudacaoDaHora: string;
    jogo: JogoAberto | null;
    /** O "player": o jogo aberto, se houver; senão a última partida. */
    jogoDoPlayer: string | null;
    /** Só a partida do jogo do player: os números nunca são de outro jogo. */
    partidaDoPlayer: ResumoDoJogo | null;
    mudancas: number | null;
    instalados: { jogo: JogoDaLoja; capa: string | null }[];
    capaDaUltima: string | null;
    /** O cartão da partida que acabou de fechar (nesta abertura do Otimiza). */
    cartao: Cartao | null;
    lisoGravado: string | null;
    gravandoLiso: boolean;
    hardware: Hardware | null;
    medidores: Medidor[];
    /** As últimas leituras do processador: o gráfico vivo do bloco Agora. */
    curvaCpu: { linha: string; area: string } | null;
    banda: PainelDaBanda | null;
    varredura: PainelDaVarredura | null;
    telaPrincipal: Hardware["telas"][number] | null;
    onabrir: (id: IdDaArea, abaInterna?: AbaDoSistema) => void;
    onjogar: (nome: string) => void;
    /** A arte da loja não carregou: a casca troca pela capa de reserva ou pelas iniciais. */
    onfotoFalhou: (nome: string) => void;
    ondeixarLiso: () => void;
    onalternarBanda: () => void;
    onalternarVarredura: () => void;
  } = $props();

  let filtroDeJogo = $state("");
  const jogosFiltrados = $derived(
    instalados.filter((i) => i.jogo.nome.toLowerCase().includes(filtroDeJogo.trim().toLowerCase())).slice(0, 4)
  );
  let cartaoCopiado = $state(false);

  async function copiarCartao() {
    if (!cartao?.medido) return;
    try {
      await navigator.clipboard.writeText(textoParaCompartilhar(cartao));
      cartaoCopiado = true;
      setTimeout(() => (cartaoCopiado = false), 1600);
    } catch {
      // Sem área de transferência: o número continua na tela.
    }
  }

  /** A luz do vidro segue o cursor, como no visionOS. */
  function brilho(no: HTMLElement) {
    const mover = (e: PointerEvent) => {
      const r = no.getBoundingClientRect();
      no.style.setProperty("--mx", `${e.clientX - r.left}px`);
      no.style.setProperty("--my", `${e.clientY - r.top}px`);
    };
    no.addEventListener("pointermove", mover);
    return { destroy: () => no.removeEventListener("pointermove", mover) };
  }

  function inteiro(v: number | null | undefined) {
    return v == null ? "—" : String(Math.round(v));
  }
</script>

<div class="grade">
  <!-- Pronto pra jogar: a prontidão lida neste PC, item a item, e as duas ações da tela -->
  <section class="bloco b-heroi" use:brilho>
    <div class="heroi-ola">
      <span class="heroi-oti"><Oti humor={prontidao.ok === prontidao.total ? "feliz" : humor} tamanho={64} segueCursor={false} /></span>
      <div>
        <p class="sec">{temporada?.saudacao ?? saudacaoDaHora}</p>
        <h2 class="heroi-titulo">{prontidao.ok === prontidao.total ? "Pronto pra jogar" : prontidao.total - prontidao.ok === 1 ? "Quase pronto pra jogar" : "Dá pra deixar melhor"}</h2>
        <p class="sec">{prontidao.ok} de {prontidao.total} em ordem neste PC</p>
      </div>
    </div>
    <ul class="heroi-itens">
      {#each prontidao.itens as it (it.id)}
        <li>
          <button class="item-pronto" class:ok={it.ok} onclick={it.acao} disabled={it.ok && it.id !== "otimizado"} title={it.botao ? `${it.botao} durante o jogo` : undefined}>
            <span class="marca-pronto" aria-hidden="true">{it.ok ? "✓" : "!"}</span>
            <span class="texto-pronto">{it.texto}</span>
            {#if it.botao}<span class="seta-pronto" aria-hidden="true">›</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
    <div class="heroi-acoes">
      <div class="anel" style:--fr={prontidao.fracao} aria-label={`${prontidao.ok} de ${prontidao.total} em ordem`}>
        <svg viewBox="0 0 64 64" aria-hidden="true">
          <circle cx="32" cy="32" r="27" class="anel-fundo" />
          <circle cx="32" cy="32" r="27" class="anel-cheio" pathLength="100" />
        </svg>
        <span class="numero">{Math.round(prontidao.fracao * 100)}<small>%</small></span>
      </div>
      <div class="heroi-botoes">
        <button class="botao-principal" onclick={() => onabrir("turbo")}>Otimizar agora</button>
        {#if jogoDoPlayer && !jogo}
          <button class="botao-secundario" onclick={() => jogoDoPlayer && onjogar(jogoDoPlayer)}>Jogar {jogoDoPlayer}</button>
        {:else if mudancas}
          <button class="botao-secundario" onclick={() => onabrir("historico")}>O que foi mudado ({mudancas})</button>
        {/if}
      </div>
    </div>
  </section>

  <!-- Seus jogos: busca e as capas mais recentes -->
  <section class="bloco b-jogos" use:brilho>
    <label class="campo-busca">
      <Icone nome="busca" tamanho={15} />
      <input bind:value={filtroDeJogo} placeholder="Buscar nos seus jogos" aria-label="Buscar nos seus jogos" />
    </label>
    <div class="tres">
      {#each jogosFiltrados as { jogo: j, capa } (j.nome)}
        <div class="capa-jogo">
          <button class="capa-abrir" onclick={() => onabrir("jogos")} title={j.nome} aria-label={`${j.nome}: ver partidas`}>
            {#if capa}<img src={capa} alt="" onerror={() => onfotoFalhou(j.nome)} />{:else}<span class="ini">{iniciais(j.nome)}</span>{/if}
            {#if partidaDoPlayer?.ultima && j.nome === partidaDoPlayer.nome}<span class="fps-na-capa"><strong>{inteiro(partidaDoPlayer.ultima.fps_nativo)}</strong> FPS</span>{/if}
          </button>
          <div class="capa-pe">
            <span class="capa-nome">{j.nome}</span>
            {#if !jogo}<button class="jogar" onclick={() => onjogar(j.nome)} aria-label={`Jogar ${j.nome}`}>Jogar</button>{/if}
          </div>
        </div>
      {:else}
        <p class="sec vazio">{instalados.length ? "Nenhum jogo com esse nome." : "Lendo seus jogos…"}</p>
      {/each}
    </div>
  </section>

  <!-- A última partida: só número medido, com nome -->
  <section class="bloco b-partida" use:brilho>
    {#if cartao && !jogo}
      <h3>A partida que acabou</h3>
      <p class="cartao-jogo"><strong>{cartao.jogo}</strong> <span class="sec">· {cartao.duracao}</span></p>
      {#if cartao.medido}
        <dl class="numeros">
          <div><dt>FPS médio</dt><dd class="numero">{cartao.fps != null ? inteiro(cartao.fps) : "—"}</dd></div>
          <div><dt>1% pior</dt><dd class="numero">{cartao.low != null ? inteiro(cartao.low) : "—"}</dd></div>
          {#if cartao.trancosPorMinuto != null}
            <div><dt>Trancos/min</dt><dd class="numero">{cartao.trancosPorMinuto.toLocaleString("pt-BR", { maximumFractionDigits: 1 })}</dd></div>
          {/if}
        </dl>
        <p class="sec pequeno">
          {#if lisoGravado}{lisoGravado}{:else if cartao.rede && cartao.rede.firmeza !== "Firme"}{cartao.rede.frase}{:else if cartao.comparacao}{fraseDaComparacao(cartao.comparacao)}{:else}Com mais partidas medidas, aparece a faixa das anteriores.{/if}
        </p>
        {@const liso = !lisoGravado && ofereceLiso(cartao, hardware?.placa.fabricante === "Nvidia")}
        <div class="acoes-cartao">
          {#if liso}
            <button class="botao-liso" disabled={gravandoLiso} onclick={ondeixarLiso}>{gravandoLiso ? "Gravando…" : "Deixar liso"}</button>
          {/if}
          <button class="link" onclick={() => onabrir("prova")}>Ver a partida <Icone nome="seta_direita" tamanho={14} /></button>
          {#if !liso}<button class="link" onclick={copiarCartao}>{cartaoCopiado ? "Copiado" : "Copiar para o Discord"}</button>{/if}
        </div>
      {:else}
        <p class="sec pequeno">
          {cartao.porque === "curta"
            ? "Curta demais para medir: a primeira medição sai três minutos depois de o jogo abrir."
            : "Esta partida não foi medida. Com o Otimiza como administrador, a medição usa o PresentMon."}
        </p>
      {/if}
    {:else}
    <div class="cabeca-bloco">
      <h3>{jogo ? "Jogando agora" : "Última partida"}</h3>
      <button class="link pequeno" onclick={() => onabrir("jogos")}>Ver partidas <Icone nome="seta_direita" tamanho={12} /></button>
    </div>
    <div class="faixa">
      <span class="capa">
        {#if capaDaUltima}<img src={capaDaUltima} alt="" onerror={() => jogoDoPlayer && onfotoFalhou(jogoDoPlayer)} />{:else if jogoDoPlayer}<span class="ini">{iniciais(jogoDoPlayer)}</span>{/if}
      </span>
      <strong class="faixa-nome">{jogoDoPlayer ?? "Nenhuma partida ainda"}</strong>
    </div>
    {#if partidaDoPlayer?.ultima}
      <dl class="numeros">
        <div><dt>FPS médio</dt><dd class="numero">{inteiro(partidaDoPlayer.ultima.fps_nativo)}</dd></div>
        <div><dt>1% pior</dt><dd class="numero">{inteiro(partidaDoPlayer.ultima.low_1pct)}</dd></div>
      </dl>
      {#if partidaDoPlayer.ganho_low.estado === "Medido" && partidaDoPlayer.ganho_low.pct != null}
        <p class="sec pequeno"><span class="selo medido">medido</span> {partidaDoPlayer.ganho_low.pct >= 0 ? "+" : ""}{inteiro(partidaDoPlayer.ganho_low.pct)}% no 1% pior desde que otimizou</p>
      {/if}
    {:else}
      <p class="sec pequeno">Jogue com o Otimiza aberto: ele mede o FPS sozinho, sem nada por cima do jogo.</p>
    {/if}
    {/if}
  </section>

  <!-- Agora: os quatro números do PC, cada um com nome -->
  <section class="bloco b-agora" use:brilho>
    <h3>Agora</h3>
    <ul class="medidores">
      {#each medidores.slice(0, 2) as m (m.nome)}
        <li>
          <div class="medidor-linha">
            <span>{m.nome}</span>
            <span class="numero">{m.texto}</span>
          </div>
          <div class="trilha" class:alerta={m.alerta}><div style:width={`${m.fracao == null ? 0 : Math.max(2, Math.min(100, m.fracao * 100))}%`}></div></div>
        </li>
      {/each}
    </ul>
    {#if curvaCpu && !jogo}
      <figure class="vivo">
        <svg viewBox="0 0 120 36" preserveAspectRatio="none" aria-hidden="true">
          <defs>
            <linearGradient id="vivo-preenchido" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="currentColor" stop-opacity="0.22" />
              <stop offset="1" stop-color="currentColor" stop-opacity="0" />
            </linearGradient>
          </defs>
          <path d={curvaCpu.area} fill="url(#vivo-preenchido)" />
          <path d={curvaCpu.linha} fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
        </svg>
      </figure>
    {/if}
    {#if jogo}<p class="sec pequeno">Leitura pausada com o jogo aberto.</p>{/if}
    <button class="link" onclick={() => onabrir("sistema", "agora")}>Ver quem está pesando <Icone nome="seta_direita" tamanho={14} /></button>
  </section>

  <!-- Durante o jogo: o que liga sozinho quando um jogo abre -->
  <section class="bloco b-fundo" use:brilho>
    <h3>Durante o jogo</h3>
    <ul class="chaves">
      <li>
        <div>
          <span>Segurar downloads de fundo</span>
          <span class="sec pequeno">
            {banda?.agindo_em ? `Segurando durante ${banda.agindo_em}` : banda?.impedimento ? "Precisa de administrador" : "Nuvem, torrent, lojas e Windows Update"}
          </span>
        </div>
        <button
          class="chave"
          role="switch"
          aria-checked={banda?.ligado ?? false}
          aria-label="Segurar downloads de fundo durante o jogo"
          disabled={!banda || (!banda.ligado && banda.impedimento !== null)}
          title={banda?.impedimento ?? undefined}
          onclick={onalternarBanda}
        ></button>
      </li>
      <li>
        <div>
          <span>Wi-Fi sem procurar redes</span>
          <span class="sec pequeno">{varredura?.agindo_em ? `Agindo em ${varredura.agindo_em}` : "Tira os picos de ping que vêm a cada minuto"}</span>
        </div>
        <button class="chave" role="switch" aria-checked={varredura?.ligado ?? false} aria-label="Wi-Fi sem procurar redes durante o jogo" disabled={!varredura} onclick={onalternarVarredura}></button>
      </li>
    </ul>
  </section>

  <!-- Tela: a taxa em uso contra a que o monitor aguenta -->
  <section class="bloco b-tela" use:brilho>
    <h3>Tela</h3>
    <span class="numero gigante">{telaPrincipal ? telaPrincipal.hz_atual : "—"}<small>Hz</small></span>
    <p class="sec pequeno">
      {#if !telaPrincipal}
        Lendo o monitor…
      {:else if telaPrincipal.hz_atual < telaPrincipal.hz_maximo}
        {telaPrincipal.nome} aguenta {telaPrincipal.hz_maximo} Hz.
      {:else}
        {telaPrincipal.nome} no máximo, {telaPrincipal.largura}×{telaPrincipal.altura}.
      {/if}
    </p>
    {#if telaPrincipal && telaPrincipal.hz_atual < telaPrincipal.hz_maximo}
      <button class="botao-pequeno escuro" onclick={() => onabrir("tela")}>Usar {telaPrincipal.hz_maximo} Hz</button>
    {:else}
      <button class="link" onclick={() => onabrir("tela")}>Ver a tela <Icone nome="seta_direita" tamanho={14} /></button>
    {/if}
  </section>
</div>

<style>
  .sec {
    color: var(--sec);
  }
  /* Sem vidro atrás (fundo limpo ou a área de trabalho), o desfoque do cartão não tem o que desfocar.
     As regras com `.painel.solto` miram o painel da casca (`App.svelte`). A especificidade é a de quando moravam lá: o
     Svelte 5 só dá peso à primeira classe de escopo do seletor (a do `.painel`), e aqui ela vai no `.bloco`. */
  :global(:root.fundo-limpo .painel.solto) .bloco,
  :global(:root.janela-transparente .painel.solto) .bloco {
    backdrop-filter: none;
  }
  :global(.painel.solto) .bloco {
    background:
      radial-gradient(260px circle at var(--mx) var(--my), rgb(255 255 255 / 0.22), transparent 60%),
      var(--bloco-solto);
    backdrop-filter: blur(30px) saturate(1.5);
    box-shadow:
      0 1px 0 rgb(255 255 255 / 0.35) inset,
      0 10px 30px rgb(40 30 20 / 0.14);
  }
  :global(:root.jogo-aberto .painel.solto) .bloco {
    backdrop-filter: none;
    background: rgb(232 227 220 / 0.97);
  }
  :global(:root.jogo-aberto[data-tema="noite"] .painel.solto) .bloco {
    background: rgb(40 40 44 / 0.98);
  }
  .grade {
    height: 100%;
    min-height: 500px;
    display: grid;
    grid-template-columns: 1fr 1fr 1.1fr 1.15fr;
    grid-template-rows: auto minmax(0, 1.15fr) minmax(0, 1fr);
    grid-template-areas:
      "heroi heroi heroi heroi"
      "jogos jogos jogos partida"
      "agora fundo fundo tela";
    gap: 12px;
  }
  .b-jogos {
    grid-area: jogos;
  }
  .b-partida {
    grid-area: partida;
  }
  /* A faixa "Pronto pra jogar": saudação e o Oti, os itens lidos, o anel e as duas ações. */
  .bloco.b-heroi {
    grid-area: heroi;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 24px;
    padding: 16px 20px;
    background:
      radial-gradient(60% 120% at 0% 0%, var(--marca-suave), transparent 60%),
      radial-gradient(260px circle at var(--mx) var(--my), rgb(255 255 255 / 0.18), transparent 60%),
      var(--bloco);
  }
  .heroi-ola {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .heroi-titulo {
    font-family: var(--fonte-titulo);
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1.1;
    margin: 2px 0;
  }
  .heroi-itens {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px 8px;
  }
  .heroi-itens .item-pronto {
    width: 100%;
  }
  .texto-pronto {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }
  .item-pronto {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 6px 0 5px;
    border-radius: 999px;
    border: 1px solid var(--vidro-borda);
    background: var(--bloco-2);
    color: var(--texto);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    transition: background var(--mov-rapido) ease, transform var(--mov-rapido) var(--curva-mola);
  }
  .item-pronto:hover:not(:disabled) {
    transform: translateY(-1px);
  }
  .item-pronto:disabled {
    cursor: default;
  }
  .marca-pronto {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--atencao) 22%, transparent);
    color: var(--atencao-texto);
    font-size: 12px;
    font-weight: 800;
  }
  .item-pronto.ok .marca-pronto {
    background: color-mix(in srgb, var(--ok) 20%, transparent);
    color: var(--ok-texto);
  }
  .item-pronto.ok .texto-pronto {
    color: var(--sec);
  }
  .seta-pronto {
    padding: 0 4px;
    color: var(--sec);
    font-size: 16px;
    line-height: 1;
  }
  .heroi-acoes {
    display: flex;
    align-items: center;
    gap: 20px;
  }
  .anel {
    position: relative;
    width: 84px;
    height: 84px;
    display: grid;
    place-items: center;
  }
  .anel svg {
    position: absolute;
    inset: 0;
    transform: rotate(-90deg);
    /* Sem isto o brilho do anel era cortado pelo quadrado do SVG (aparecia um quadrado rosado atrás). */
    overflow: visible;
  }
  .anel circle {
    fill: none;
    stroke-width: 6;
  }
  .anel-fundo {
    stroke: var(--bloco-2);
  }
  .anel-cheio {
    stroke: var(--marca);
    stroke-linecap: round;
    stroke-dasharray: 100;
    stroke-dashoffset: calc(100 - var(--fr) * 100);
    filter: drop-shadow(0 0 6px color-mix(in srgb, var(--marca) 60%, transparent));
    transition: stroke-dashoffset 900ms var(--curva-saida);
  }
  .anel .numero {
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .anel small {
    font-size: 13px;
    color: var(--sec);
  }
  .heroi-botoes {
    display: grid;
    gap: 8px;
    width: 176px;
  }
  .botao-secundario {
    height: 38px;
    padding: 0 16px;
    border-radius: 12px;
    border: 1px solid var(--vidro-borda);
    background: var(--bloco-2);
    color: var(--texto);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fps-na-capa {
    position: absolute;
    left: 8px;
    top: 8px;
    padding: 3px 8px;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.55);
    color: #fff;
    font-size: 11px;
  }
  .fps-na-capa strong {
    font-family: var(--fonte-titulo);
    font-size: 13px;
  }

  .b-agora {
    grid-area: agora;
  }
  .b-fundo {
    grid-area: fundo;
  }

  .b-tela {
    grid-area: tela;
  }

  /* O bloco: vidro calmo. A luz segue o cursor; ele não pula nem cresce. */
  .bloco {
    --mx: 50%;
    --my: -40%;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
    min-height: 0;
    padding: 16px;
    border-radius: 20px;
    background:
      radial-gradient(260px circle at var(--mx) var(--my), rgb(255 255 255 / 0.22), transparent 60%),
      var(--bloco);
    border: 1px solid var(--vidro-borda);
    overflow: hidden;
  }
  :global(:root[data-tema="noite"]) .bloco {
    background:
      radial-gradient(260px circle at var(--mx) var(--my), rgb(255 255 255 / 0.06), transparent 60%),
      var(--bloco);
  }
  /* Título do bloco com a ação dele na mesma linha: não gasta uma linha do card com o link. */
  .cabeca-bloco {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }
  .cabeca-bloco .link {
    font-size: 12px;
  }
  .bloco h3 {
    margin: 0;
    color: var(--sec);
    font-family: var(--fonte-texto);
    font-size: 12px;
    font-weight: 500;
    line-height: 1.2;
  }
  .pequeno {
    font-size: 12px;
    line-height: 1.4;
  }

  /* Botões do painel: um principal escuro, um pequeno de contorno e o link de texto. */
  .botao-principal {
    height: 40px;
    border: 0;
    border-radius: 12px;
    background: var(--texto);
    color: var(--contra);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity var(--mov-rapido) ease;
  }
  .botao-principal:hover {
    opacity: 0.88;
  }
  .botao-pequeno {
    flex: none;
    align-self: center;
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--vidro-borda);
    border-radius: 9px;
    background: var(--bloco-2);
    color: var(--texto);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
  .botao-pequeno:hover {
    background: var(--bloco);
    border-color: var(--sec);
  }
  .botao-pequeno.escuro {
    align-self: flex-start;
    margin-top: auto;
    border-color: transparent;
    background: var(--texto);
    color: var(--contra);
  }
  .botao-pequeno.escuro:hover {
    opacity: 0.88;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    margin-top: auto;
    padding: 2px 0;
    border: 0;
    background: none;
    color: var(--texto);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  /* Interruptor verde */
  .chave {
    position: relative;
    flex: none;
    width: 40px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 12px;
    background: rgb(120 120 128 / 0.32);
    cursor: pointer;
    transition: background var(--mov-medio) ease;
  }
  .chave::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.2);
    transition: transform 240ms cubic-bezier(0.3, 1.4, 0.5, 1);
  }
  .chave[aria-checked="true"] {
    background: var(--verde);
  }
  .chave[aria-checked="true"]::after {
    transform: translateX(16px);
  }
  .chave:disabled {
    opacity: 0.45;
    cursor: default;
  }

  /* Seus jogos */
  .campo-busca {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 12px;
    border-radius: 10px;
    background: var(--bloco-2);
    color: var(--sec);
  }
  .campo-busca:focus-within {
    outline: 2px solid var(--texto);
    outline-offset: -1px;
  }
  .campo-busca input::placeholder {
    color: var(--sec);
  }
  .campo-busca input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--texto);
    font-size: 13px;
    user-select: text;
  }
  .tres {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    min-height: 0;
  }
  .capa-jogo {
    position: relative;
    display: grid;
    min-width: 0;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: 12px;
    background: var(--bloco-2);
    color: #fff;
    cursor: pointer;
    overflow: hidden;
  }
  /* A capa abre o jogo na área Jogos; o "Jogar" no pé abre o jogo de verdade. */
  .capa-abrir {
    grid-area: 1 / 1;
    display: grid;
    min-height: 0;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
    overflow: hidden;
  }
  .capa-pe {
    grid-area: 1 / 1;
    align-self: end;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 18px 8px 8px 10px;
    background: linear-gradient(transparent, rgb(0 0 0 / 0.62));
    pointer-events: none;
  }
  .capa-pe .capa-nome {
    flex: 1;
    min-width: 0;
    padding: 0;
    background: none;
  }
  .jogar {
    pointer-events: auto;
    flex: none;
    height: 26px;
    padding: 0 12px;
    border: 0;
    border-radius: 13px;
    background: #fff;
    color: #1d1d1f;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    box-shadow: 0 2px 8px rgb(0 0 0 / 0.25);
    transition: transform var(--mov-rapido) ease;
  }
  .jogar:hover {
    transform: scale(1.05);
  }
  /* A ação da partida com tranco: escura e pequena, cabe no card sem empurrar o rodapé. */
  .botao-liso {
    align-self: flex-start;
    flex: none;
    height: 30px;
    padding: 0 14px;
    border: 0;
    border-radius: 15px;
    background: var(--texto);
    color: var(--contra, #fff);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .botao-liso:disabled {
    opacity: 0.6;
  }
  .cartao-jogo {
    margin: 0;
    font-size: 15px;
    line-height: 1.3;
  }
  .acoes-cartao {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    margin-top: auto;
  }
  .capa-jogo img,
  .capa-jogo .ini {
    grid-area: 1 / 1;
    width: 100%;
    height: 100%;
    min-height: 0;
    object-fit: cover;
    transition: transform 300ms ease;
  }
  .capa-abrir:hover img {
    transform: scale(1.03);
  }
  /* O nome por cima do pé da capa: a arte fica inteira e o nome não briga com um rodapé claro. */
  .capa-nome {
    grid-area: 1 / 1;
    align-self: end;
    padding: 18px 10px 8px;
    background: linear-gradient(transparent, rgb(0 0 0 / 0.62));
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ini {
    display: grid;
    place-items: center;
    background: var(--bloco-2);
    color: var(--sec);
    font-family: var(--fonte-titulo);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .vazio {
    grid-column: 1 / -1;
    font-size: 12px;
  }

  /* Última partida */
  .faixa {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .capa {
    display: grid;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 10px;
    overflow: hidden;
    background: var(--bloco-2);
  }
  .capa img,
  .capa .ini {
    width: 100%;
    height: 100%;
    object-fit: cover;
    font-size: 12px;
  }
  .faixa-nome {
    min-width: 0;
    font-size: 15px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .numeros {
    display: flex;
    gap: 24px;
    margin: 0;
  }
  .numeros dt {
    color: var(--sec);
    font-size: 12px;
  }
  .numeros dd {
    margin: 0;
    font-size: 28px;
    font-weight: 600;
    line-height: 1.1;
  }

  /* Otimizar */
  .b-heroi .botao-principal {
    margin: 2px 0 6px;
  }

  /* Agora */
  .medidores {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-top: 6px;
  }
  .medidor-linha {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 6px;
    font-size: 13px;
  }
  .medidor-linha .numero {
    color: var(--sec);
    font-size: 13px;
    white-space: nowrap;
  }
  .trilha {
    height: 4px;
    border-radius: 2px;
    background: rgb(120 120 128 / 0.2);
    overflow: hidden;
  }
  .trilha div {
    height: 100%;
    border-radius: 2px;
    background: var(--texto);
    transition: width 600ms ease;
  }
  .trilha.alerta div {
    background: var(--laranja);
  }

  /* Durante o jogo */
  .chaves {
    list-style: none;
    margin: 0;
    padding: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: space-around;
  }
  .chaves li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 0;
  }
  .chaves li + li {
    border-top: 1px solid var(--vidro-borda);
  }
  .chaves li > div {
    display: grid;
    gap: 1px;
    min-width: 0;
    font-size: 13px;
  }

  /* O gráfico vivo do processador: desce até o rodapé do bloco Agora. */
  .vivo {
    display: grid;
    gap: 4px;
    margin: auto 0 0;
    color: var(--texto);
  }
  .vivo svg {
    width: 100%;
    height: 30px;
    opacity: 0.85;
  }

  /* Com um aviso só (ou nenhum), o Oti ocupa o resto do bloco em vez de deixar um buraco. */

  /* Avisos */

  /* Tela */
  .gigante {
    font-size: 40px;
    font-weight: 600;
    line-height: 1;
    letter-spacing: -0.03em;
  }
  .gigante small {
    margin-left: 4px;
    color: var(--sec);
    font-size: 16px;
    font-weight: 500;
    letter-spacing: 0;
  }

  /* Janela estreita: a grade vira uma coluna dupla que rola. */
  @media (max-width: 1100px) {
    .grade {
      height: auto;
      grid-template-columns: 1fr 1fr;
      grid-template-rows: none;
      grid-auto-rows: minmax(180px, auto);
      grid-template-areas:
        "heroi heroi"
        "jogos jogos"
        "partida agora"
        "fundo fundo"
        "tela tela";
    }
    /* Com a grade rolando, a altura vem do conteúdo: sem isto a capa (em pé) cresce com a largura da janela. */
    .tres {
      flex: none;
      height: 190px;
    }
  }</style>
