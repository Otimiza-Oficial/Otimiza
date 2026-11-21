<script module lang="ts">
  /** "Pronto pra jogar": o que o Otimiza sabe conferir, item a item. Só entra o que foi lido (nada de item inventado);
   *  cada um diz o que falta e leva para onde se resolve. */
  export type ItemDaProntidao = { id: string; ok: boolean; texto: string; acao: () => void; botao?: string };
  export type Prontidao = { itens: ItemDaProntidao[]; ok: number; total: number; fracao: number };
  /** Uma leitura do bloco "Agora", com nome e valor por extenso. A barra só fica em cor quando há o que fazer. */
  export type Medidor = { nome: string; texto: string; fracao: number | null; alerta: boolean };
</script>

<script lang="ts">
  // A tela inicial (1.0.12, Frente 1.4, marca v2): a faixa "Pronto pra jogar" com o Oti olhando o brilho, e a grade de
  // cartões (jogos, partida, agora, durante o jogo e tela). Cada cartão mostra um dado lido deste PC ou liga uma coisa
  // que existe de verdade. O estado e as ações vêm da casca (`App.svelte`).
  import Icone from "./Icone.svelte";
  import Oti, { type Humor } from "./Oti.svelte";
  import Brilho from "./marca/Brilho.svelte";
  import Botao from "./base/Botao.svelte";
  import Interruptor from "./base/Interruptor.svelte";
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
    instalados.filter((i) => i.jogo.nome.toLowerCase().includes(filtroDeJogo.trim().toLowerCase())).slice(0, 3)
  );
  let cartaoCopiado = $state(false);

  const tudoPronto = $derived(prontidao.total > 0 && prontidao.ok === prontidao.total);
  const titulo = $derived(
    prontidao.total === 0
      ? "Lendo o seu PC"
      : tudoPronto
        ? "Pronto pra jogar"
        : prontidao.total - prontidao.ok === 1
          ? "Quase pronto pra jogar"
          : "Dá pra deixar melhor"
  );

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

  function inteiro(v: number | null | undefined) {
    return v == null ? "—" : String(Math.round(v));
  }
</script>

<div class="grade">
  <!-- Pronto pra jogar: o Oti olhando o brilho, a prontidão lida neste PC e as duas ações da tela -->
  <section class="bloco b-heroi" aria-labelledby="inicio-titulo">
    <div class="cena" aria-hidden="true">
      <span class="cena-oti"><Oti humor={tudoPronto ? "feliz" : humor} tamanho={72} rotulo="" /></span>
      <span class="cena-brilho"><Brilho tamanho={120} densidade={tudoPronto ? 1.3 : 0.8} /></span>
    </div>
    <div class="heroi-texto">
      <p class="rotulo">{temporada?.saudacao ?? saudacaoDaHora}</p>
      <h2 id="inicio-titulo" class="heroi-titulo">{titulo}</h2>
      <div class="progresso" role="img" aria-label={`${prontidao.ok} de ${prontidao.total} itens em ordem`}>
        {#each prontidao.itens as it, i (it.id)}<span class="degrau" class:cheio={i < prontidao.ok}></span>{/each}
      </div>
      <p class="sec">{prontidao.ok} de {prontidao.total} em ordem neste PC</p>
    </div>
    <ul class="heroi-itens" aria-label="O que o Otimiza conferiu">
      {#each prontidao.itens as it (it.id)}
        <li>
          <button class="item-pronto" class:ok={it.ok} onclick={it.acao} disabled={it.ok && it.id !== "otimizado"}>
            <span class="ponto" data-tom={it.ok ? "ok" : "atencao"} aria-hidden="true"></span>
            <span class="texto-pronto">{it.texto}</span>
            {#if it.botao}
              <span class="acao-pronto">{it.botao}</span>
            {:else if !it.ok || it.id === "otimizado"}
              <span class="seta-pronto" aria-hidden="true"><Icone nome="seta_direita" tamanho={14} /></span>
            {/if}
            {#if !it.ok}<span class="so-leitor"> — pede atenção</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
    <div class="heroi-botoes">
      <Botao variante="principal" onclick={() => onabrir("turbo")}>Otimizar agora</Botao>
      {#if jogoDoPlayer && !jogo}
        <Botao onclick={() => jogoDoPlayer && onjogar(jogoDoPlayer)}>Jogar {jogoDoPlayer}</Botao>
      {:else if mudancas}
        <Botao onclick={() => onabrir("historico")}>O que foi mudado ({mudancas})</Botao>
      {/if}
    </div>
  </section>

  <!-- Seus jogos: busca e as capas mais recentes -->
  <section class="bloco b-jogos" aria-label="Seus jogos">
    <div class="cabeca-bloco">
      <h3 class="rotulo">Seus jogos</h3>
      <label class="campo-busca">
        <Icone nome="busca" tamanho={14} />
        <input bind:value={filtroDeJogo} placeholder="Buscar" aria-label="Buscar nos seus jogos" />
      </label>
    </div>
    <div class="tres">
      {#each jogosFiltrados as { jogo: j, capa } (j.nome)}
        <div class="capa-jogo">
          <button class="capa-abrir" onclick={() => onabrir("jogos")} aria-label={`${j.nome}: ver a página do jogo`}>
            {#if capa}<img src={capa} alt="" onerror={() => onfotoFalhou(j.nome)} />{:else}<span class="ini">{iniciais(j.nome)}</span>{/if}
            {#if partidaDoPlayer?.ultima && j.nome === partidaDoPlayer.nome}<span class="fps-na-capa"><strong>{inteiro(partidaDoPlayer.ultima.fps_nativo)}</strong> FPS</span>{/if}
          </button>
          <div class="capa-pe">
            <span class="capa-nome">{j.nome}</span>
            {#if !jogo}<button class="jogar" onclick={() => onjogar(j.nome)} aria-label={`Jogar ${j.nome}`}>Jogar</button>{/if}
          </div>
        </div>
      {:else}
        <p class="sec vazio">
          {instalados.length
            ? "Nenhum jogo com esse nome. Apague a busca para ver todos."
            : "Procurando jogos na Steam, Epic e Riot. Jogo de outra loja aparece depois da primeira partida com o Otimiza aberto."}
        </p>
      {/each}
    </div>
  </section>

  <!-- A última partida: só número medido, com nome -->
  <section class="bloco b-partida" aria-label={cartao && !jogo ? "A partida que acabou" : jogo ? "Jogando agora" : "Última partida"}>
    {#if cartao && !jogo}
      <h3 class="rotulo">A partida que acabou</h3>
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
            <Botao variante="principal" pequeno trabalhando={gravandoLiso} textoTrabalhando="Gravando…" onclick={ondeixarLiso}>Deixar liso</Botao>
          {/if}
          <Botao variante="link" onclick={() => onabrir("prova")}>Ver a partida</Botao>
          {#if !liso}<Botao variante="link" onclick={copiarCartao}>{cartaoCopiado ? "Copiado" : "Copiar para o Discord"}</Botao>{/if}
        </div>
      {:else}
        <p class="sec pequeno">
          {cartao.porque === "curta"
            ? "Curta demais para medir: a primeira medição sai três minutos depois de o jogo abrir. Jogue um pouco mais na próxima."
            : "Esta partida não foi medida. Abra o Otimiza como administrador na próxima: aí a medição usa o PresentMon."}
        </p>
      {/if}
    {:else}
      <div class="cabeca-bloco">
        <h3 class="rotulo">{jogo ? "Jogando agora" : "Última partida"}</h3>
        <Botao variante="link" pequeno onclick={() => onabrir("jogos")}>Ver partidas</Botao>
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
          <p class="sec pequeno ganho"><span class="selo medido">medido</span> {partidaDoPlayer.ganho_low.pct >= 0 ? "+" : ""}{inteiro(partidaDoPlayer.ganho_low.pct)}% no 1% pior desde que otimizou</p>
        {/if}
      {:else}
        <p class="sec pequeno">Jogue com o Otimiza aberto: ele mede o FPS sozinho, sem nada por cima do jogo.</p>
      {/if}
    {/if}
  </section>

  <!-- Agora: os números do PC, cada um com nome -->
  <section class="bloco b-agora" aria-label="Agora">
    <h3 class="rotulo">Agora</h3>
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
      <figure class="vivo" aria-hidden="true">
        <svg viewBox="0 0 120 36" preserveAspectRatio="none">
          <defs>
            <linearGradient id="vivo-preenchido" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stop-color="currentColor" stop-opacity="0.16" />
              <stop offset="1" stop-color="currentColor" stop-opacity="0" />
            </linearGradient>
          </defs>
          <path d={curvaCpu.area} fill="url(#vivo-preenchido)" />
          <path d={curvaCpu.linha} fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
        </svg>
      </figure>
    {/if}
    {#if jogo}<p class="sec pequeno">Leitura pausada com o jogo aberto, para não pesar.</p>{/if}
    <span class="ao-pe"><Botao variante="link" onclick={() => onabrir("sistema", "agora")}>Ver quem está pesando</Botao></span>
  </section>

  <!-- Durante o jogo: o que liga sozinho quando um jogo abre -->
  <section class="bloco b-fundo" aria-label="Durante o jogo">
    <h3 class="rotulo">Durante o jogo</h3>
    <div class="chaves">
      <Interruptor
        ligado={banda?.ligado ?? false}
        rotulo="Segurar downloads de fundo"
        apoio={banda?.agindo_em ? `Segurando agora, durante ${banda.agindo_em}` : "Nuvem, torrent, lojas e Windows Update esperam o jogo fechar"}
        impedimento={banda?.impedimento ? "Precisa do Otimiza como administrador: feche e abra com o botão direito › Executar como administrador." : null}
        disabled={!banda}
        onalternar={onalternarBanda}
      />
      <Interruptor
        ligado={varredura?.ligado ?? false}
        rotulo="Wi-Fi sem procurar redes"
        apoio={varredura?.agindo_em ? `Agindo agora, em ${varredura.agindo_em}` : "Tira os picos de ping que vêm a cada minuto"}
        disabled={!varredura}
        onalternar={onalternarVarredura}
      />
    </div>
  </section>

  <!-- Tela: a taxa em uso contra a que o monitor aguenta -->
  <section class="bloco b-tela" aria-label="Tela">
    <h3 class="rotulo">Tela</h3>
    <span class="numero gigante">{telaPrincipal ? telaPrincipal.hz_atual : "—"}<small>Hz</small></span>
    <p class="sec pequeno">
      {#if !telaPrincipal}
        Lendo o monitor…
      {:else if telaPrincipal.hz_atual < telaPrincipal.hz_maximo}
        O {telaPrincipal.nome} aguenta {telaPrincipal.hz_maximo} Hz e está em {telaPrincipal.hz_atual}.
      {:else}
        {telaPrincipal.nome} no máximo, {telaPrincipal.largura}×{telaPrincipal.altura}.
      {/if}
    </p>
    <span class="ao-pe">
      {#if telaPrincipal && telaPrincipal.hz_atual < telaPrincipal.hz_maximo}
        <Botao variante="principal" pequeno onclick={() => onabrir("tela")}>Usar {telaPrincipal.hz_maximo} Hz</Botao>
      {:else}
        <Botao variante="link" onclick={() => onabrir("tela")}>Ver a tela</Botao>
      {/if}
    </span>
  </section>
</div>

<style>
  .sec {
    color: var(--tinta-2);
  }
  .grade {
    height: 100%;
    min-height: 520px;
    display: grid;
    grid-template-columns: 1fr 1fr 1.1fr 1.15fr;
    grid-template-rows: auto minmax(0, 1.15fr) minmax(0, 1fr);
    grid-template-areas:
      "heroi heroi heroi heroi"
      "jogos jogos jogos partida"
      "agora fundo fundo tela";
    gap: var(--e-3);
  }
  .b-heroi {
    grid-area: heroi;
  }
  .b-jogos {
    grid-area: jogos;
  }
  .b-partida {
    grid-area: partida;
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

  /* O cartão: superfície firme, borda fina e o fio de luz no alto. Não pula nem cresce. */
  .bloco {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--e-3);
    min-width: 0;
    min-height: 0;
    padding: 18px var(--e-4) var(--e-4);
    border: 1px solid var(--linha);
    border-radius: var(--raio-4);
    background: var(--superficie-casca, var(--superficie));
    box-shadow: var(--fio), var(--sombra-1);
    overflow: hidden;
  }
  :global(:root.fundo-papel) .bloco,
  :global(:root[data-fundo="comodo"]) .bloco {
    backdrop-filter: var(--desfoque);
  }
  .rotulo {
    margin: 0;
    line-height: 1.2;
  }
  .cabeca-bloco {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--e-2);
    min-height: 28px;
  }
  .pequeno {
    font-size: var(--t-apoio);
    line-height: 1.45;
  }
  .ao-pe {
    margin-top: auto;
  }

  /* ---------- A faixa "Pronto pra jogar" ---------- */
  .b-heroi {
    display: grid;
    grid-template-columns: auto minmax(200px, 0.9fr) minmax(0, 1.3fr) auto;
    align-items: center;
    gap: var(--e-5);
    padding: var(--e-5) var(--e-5) var(--e-5) var(--e-4);
  }
  /* A cena da marca: o Oti olhando para o alto, à direita, onde o brilho se junta. */
  .cena {
    position: relative;
    width: 150px;
    height: 104px;
  }
  .cena-oti {
    position: absolute;
    left: 4px;
    bottom: 2px;
  }
  .cena-brilho {
    position: absolute;
    right: -18px;
    top: -26px;
  }
  .heroi-texto {
    display: grid;
    gap: 6px;
    min-width: 0;
  }
  .heroi-titulo {
    font-family: var(--fonte-titulo);
    font-size: var(--t-titulo-area);
    font-weight: 700;
    letter-spacing: var(--rastreio-titulo);
    line-height: 1.05;
  }
  .heroi-texto .sec {
    font-size: var(--t-linha);
  }
  /* O progresso em degraus: um por item conferido, cheio quando está em ordem. */
  .progresso {
    display: flex;
    gap: 4px;
    margin-top: var(--e-2);
    max-width: 220px;
  }
  .degrau {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--superficie-3);
  }
  .degrau.cheio {
    background: var(--tinta);
  }
  .heroi-itens {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 2px var(--e-4);
    min-width: 0;
  }
  .item-pronto {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 36px;
    padding: 0 var(--e-2);
    border: 0;
    border-bottom: 1px solid var(--linha);
    border-radius: 0;
    background: transparent;
    color: var(--tinta);
    font-size: var(--t-linha);
    text-align: left;
    cursor: pointer;
    transition: background var(--mov-rapido) ease;
  }
  .item-pronto:hover:not(:disabled) {
    background: var(--acao-suave);
  }
  .item-pronto:disabled {
    cursor: default;
  }
  .item-pronto:focus-visible {
    outline-offset: -2px;
    border-radius: var(--raio-1);
  }
  .item-pronto.ok .texto-pronto {
    color: var(--tinta-2);
  }
  .texto-pronto {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acao-pronto {
    flex: none;
    color: var(--tinta);
    font-size: var(--t-apoio);
    font-weight: 600;
    text-decoration: underline;
    text-decoration-color: var(--linha-forte);
    text-underline-offset: 3px;
  }
  .seta-pronto {
    display: grid;
    color: var(--tinta-3);
  }
  .heroi-botoes {
    display: grid;
    gap: var(--e-2);
    width: 180px;
  }
  .heroi-botoes :global(.btn) {
    min-height: 40px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .so-leitor {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* ---------- Seus jogos ---------- */
  .campo-busca {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 200px;
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--linha);
    border-radius: var(--raio-pilula);
    color: var(--tinta-3);
    transition: border-color var(--mov-rapido) ease;
  }
  .campo-busca:hover {
    border-color: var(--linha-forte);
  }
  .campo-busca:focus-within {
    border-color: var(--foco);
  }
  .campo-busca input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--tinta);
    font-size: var(--t-linha);
    user-select: text;
  }
  .campo-busca input::placeholder {
    color: var(--tinta-3);
  }
  .tres {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--e-3);
    min-height: 0;
  }
  .capa-jogo {
    position: relative;
    display: grid;
    min-width: 0;
    min-height: 0;
    border-radius: var(--raio-3);
    background: var(--superficie-2);
    color: var(--branco);
    overflow: hidden;
    box-shadow: inset 0 0 0 1px var(--linha);
  }
  /* A capa abre a página do jogo; o "Jogar" no pé abre o jogo de verdade. */
  .capa-abrir {
    grid-area: 1 / 1;
    display: grid;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: inherit;
    background: none;
    cursor: pointer;
    overflow: hidden;
  }
  .capa-abrir:focus-visible {
    outline-offset: -3px;
  }
  .capa-jogo img,
  .capa-jogo .ini {
    grid-area: 1 / 1;
    width: 100%;
    height: 100%;
    min-height: 0;
    object-fit: cover;
    transition: transform var(--mov-entrar) var(--curva-padrao);
  }
  .capa-abrir:hover img {
    transform: scale(1.02);
  }
  .capa-pe {
    grid-area: 1 / 1;
    align-self: end;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 28px 8px 8px 12px;
    background: linear-gradient(transparent, rgb(0 0 0 / 0.72));
    pointer-events: none;
  }
  .capa-nome {
    flex: 1;
    min-width: 0;
    font-size: var(--t-linha);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Sobre a capa a escrita é sempre branca (a capa é escura no pé), nos dois temas. */
  .jogar {
    pointer-events: auto;
    flex: none;
    height: 28px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--raio-pilula);
    background: var(--branco);
    color: var(--carvao);
    font-size: var(--t-apoio);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--mov-rapido) ease;
  }
  .jogar:hover {
    background: var(--papel);
  }
  .jogar:focus-visible {
    outline-color: var(--branco);
  }
  .fps-na-capa {
    position: absolute;
    left: 8px;
    top: 8px;
    padding: 2px 8px;
    border-radius: var(--raio-pilula);
    background: rgb(0 0 0 / 0.62);
    color: var(--branco);
    font-size: var(--t-apoio);
  }
  .fps-na-capa strong {
    font-family: var(--fonte-titulo);
  }
  .ini {
    display: grid;
    place-items: center;
    background: var(--superficie-2);
    color: var(--tinta-2);
    font-family: var(--fonte-titulo);
    font-size: var(--t-subtitulo);
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .vazio {
    grid-column: 1 / -1;
    align-self: center;
    max-width: 52ch;
    font-size: var(--t-linha);
  }

  /* ---------- Última partida ---------- */
  .cartao-jogo {
    font-size: var(--t-subtitulo);
    line-height: 1.3;
  }
  .acoes-cartao {
    display: flex;
    align-items: center;
    gap: var(--e-4);
    flex-wrap: wrap;
    margin-top: auto;
  }
  .faixa {
    display: flex;
    align-items: center;
    gap: var(--e-3);
    min-width: 0;
  }
  .capa {
    display: grid;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: var(--raio-2);
    overflow: hidden;
    background: var(--superficie-2);
    box-shadow: inset 0 0 0 1px var(--linha);
  }
  .capa img,
  .capa .ini {
    width: 100%;
    height: 100%;
    object-fit: cover;
    font-size: var(--t-apoio);
  }
  .faixa-nome {
    min-width: 0;
    font-size: var(--t-subtitulo);
    font-weight: 600;
    letter-spacing: var(--rastreio-subtitulo);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .numeros {
    display: flex;
    gap: var(--e-5);
    margin: 0;
  }
  .numeros dt {
    color: var(--tinta-2);
    font-size: var(--t-apoio);
  }
  .numeros dd {
    margin: 2px 0 0;
    font-size: var(--t-numero);
    font-weight: 600;
    letter-spacing: var(--rastreio-titulo);
    line-height: 1;
  }
  .ganho {
    display: flex;
    align-items: baseline;
    gap: 6px;
    flex-wrap: wrap;
  }

  /* ---------- Agora ---------- */
  .medidores {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--e-4);
  }
  .medidor-linha {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--e-2);
    margin-bottom: 6px;
    font-size: var(--t-linha);
  }
  .medidor-linha .numero {
    color: var(--tinta-2);
    font-weight: 500;
    letter-spacing: 0;
    white-space: nowrap;
  }
  .trilha {
    height: 3px;
    border-radius: 2px;
    background: var(--superficie-3);
    overflow: hidden;
  }
  .trilha div {
    height: 100%;
    border-radius: 2px;
    background: var(--tinta);
    transition: width var(--mov-lento) var(--curva-padrao);
  }
  .trilha.alerta div {
    background: var(--atencao);
  }
  .vivo {
    margin: 0;
    color: var(--tinta-2);
  }
  .vivo svg {
    display: block;
    width: 100%;
    height: 30px;
  }

  /* ---------- Durante o jogo ---------- */
  .chaves {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: space-around;
  }
  .chaves > :global(.linha-interruptor) {
    padding: var(--e-3) 0;
  }
  .chaves > :global(.linha-interruptor + .linha-interruptor) {
    border-top: 1px solid var(--linha);
  }

  /* ---------- Tela ---------- */
  .gigante {
    font-size: var(--t-destaque);
    font-weight: 600;
    line-height: 1;
    letter-spacing: var(--rastreio-titulo);
  }
  .gigante small {
    margin-left: 4px;
    color: var(--tinta-2);
    font-size: var(--t-subtitulo);
    font-weight: 500;
    letter-spacing: 0;
  }

  /* Janela estreita: a grade vira uma coluna dupla que rola, e a faixa de cima se arruma em duas linhas. */
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
    .b-heroi {
      grid-template-columns: auto minmax(0, 1fr) auto;
    }
    .heroi-itens {
      grid-column: 1 / -1;
      grid-row: 2;
    }
    .tres {
      flex: none;
      height: 190px;
    }
  }
</style>
