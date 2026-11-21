<script lang="ts">
  import Esqueleto from "./base/Esqueleto.svelte";
  import Abas from "./base/Abas.svelte";
  // A área Jogos: os jogos instalados (Steam, Epic e instaladores) com a capa da Steam deste PC, e, para os que o
  // Otimiza já mediu, a última partida e o ganho. O ganho só aparece como número quando é MEDIDO (os intervalos de
  // antes e depois não se encostam); senão a tela diz que não houve diferença ou quantas partidas faltam.
  import { onMount } from "svelte";
  import OndeOsJogosMoram from "./OndeOsJogosMoram.svelte";
  import Cs2Integracao from "./Cs2Integracao.svelte";
  import FivemPainel from "./FivemPainel.svelte";
  import CatalogoDoJogo from "./CatalogoDoJogo.svelte";
  import GraficosGta from "./GraficosGta.svelte";
  import Cartao from "./base/Cartao.svelte";
  import { CATALOGO, doCatalogo } from "../lib/catalogo";
  import PcsParecidos from "./PcsParecidos.svelte";
  import TurboDoJogo from "./TurboDoJogo.svelte";
  import LinhaDoTempo from "./LinhaDoTempo.svelte";
  import Icone from "./Icone.svelte";
  import { capaDepoisDaFalha, capaRemota, chaveDoJogo, iniciais } from "../lib/capas";
  import { JOGAR } from "../lib/partida";

  /** A "loja" de um jogo que o Otimiza mediu mas não achou instalado: sem caminho para o "Jogar". */
  const SO_MEDIDO = "Medido pelo Otimiza";
  import { ponte, type Ganho, type JogoDaLoja, type ResumoDoJogo } from "../lib/ponte";

  type Item = { jogo: JogoDaLoja; capa: string | null; medido: ResumoDoJogo | null };

  let itens = $state<Item[] | null>(null);
  let erro = $state<string | null>(null);
  let filtro = $state("");
  let soMedidos = $state(false);
  let escolhido = $state<string | null>(null);

  const visiveis = $derived(
    (itens ?? []).filter(
      (i) => (!soMedidos || i.medido) && i.jogo.nome.toLowerCase().includes(filtro.trim().toLowerCase())
    )
  );
  const atual = $derived(itens?.find((i) => i.jogo.nome === escolhido) ?? null);
  /** O jogo escolhido no catálogo (lib/catalogo.ts): configuração, caches, anticheat e painéis próprios. */
  const doJogo = $derived(atual ? doCatalogo(atual.jogo.nome, atual.jogo.appid) : null);

  // A página do jogo em sub-abas, cada uma com uma pergunta: como está (Resumo), o que fazer (Otimizar), o que só este
  // jogo tem (Ferramentas, só quando há) e o que mudou (Histórico). A aba escolhida vale ao trocar de jogo; sem ela no
  // jogo novo, volta para o Resumo.
  type AbaDoJogo = "resumo" | "otimizar" | "ferramentas" | "historico";
  let aba = $state<AbaDoJogo>("resumo");
  const abasDoJogo = $derived(
    (
      [
        ["resumo", "Resumo"],
        ["otimizar", "Otimizar"],
        ["ferramentas", "Ferramentas"],
        ["historico", "Histórico"],
      ] as [AbaDoJogo, string][]
    ).filter(([id]) => id !== "ferramentas" || (doJogo?.extras.length ?? 0) > 0),
  );
  const abaAtual = $derived(abasDoJogo.some(([id]) => id === aba) ? aba : "resumo");

  /** Mesmo jogo, mesma chave: a biblioteca medida e a lista de instalados vêm de fontes com nomes diferentes. */
  const chave = chaveDoJogo;

  onMount(async () => {
    try {
      const [instalados, biblioteca] = await Promise.all([ponte.instalados(), ponte.biblioteca().catch(() => [])]);
      const medidos = new Map(biblioteca.map((b) => [chave(b.nome), b]));
      const lista: Item[] = instalados.map((j) => ({
        jogo: j,
        capa: capaRemota(j.nome, j.appid),
        medido: medidos.get(chave(j.nome)) ?? null,
      }));
      // Medidos que não aparecem como instalados (FiveM pelo instalador próprio, por exemplo) também entram.
      for (const b of biblioteca) {
        if (!lista.some((i) => chave(i.jogo.nome) === chave(b.nome))) {
          lista.push({
            jogo: { nome: b.nome, loja: SO_MEDIDO, appid: null, ultima_vez: b.ultima?.quando ?? 0, gb: null },
            capa: capaRemota(b.nome, null),
            medido: b,
          });
        }
      }
      itens = lista;
      escolhido = lista.find((i) => i.medido)?.jogo.nome ?? lista[0]?.jogo.nome ?? null;
      for (const [n, i] of lista.entries()) {
        if (i.jogo.appid == null) continue;
        const capa = await ponte.capa(i.jogo.appid).catch(() => null);
        if (capa && itens) itens[n] = { ...itens[n], capa };
      }
    } catch (e) {
      erro = String(e);
    }
  });

  /** A foto não carregou: vai para a capa de reserva e, sem ela, para a capa desenhada. */
  function fotoFalhou(nome: string) {
    if (itens) itens = itens.map((i) => (i.jogo.nome === nome ? { ...i, capa: capaDepoisDaFalha(nome, i.capa) } : i));
  }

  /** `null` quando a loja não informou (0) ou o número não é uma data: a tela omite em vez de inventar. */
  function quando(segundos: number): string | null {
    if (!segundos || segundos < 1_000_000_000) return null;
    const dias = Math.floor((Date.now() / 1000 - segundos) / 86400);
    if (dias <= 0) return "hoje";
    if (dias === 1) return "ontem";
    if (dias < 30) return `há ${dias} dias`;
    return new Date(segundos * 1000).toLocaleDateString("pt-BR", { day: "numeric", month: "short", year: "numeric" });
  }

  function gb(v: number | null) {
    return v == null || v <= 0 ? null : `${v >= 10 ? Math.round(v) : v.toFixed(1).replace(".", ",")} GB`;
  }

  function inteiro(v: number | null | undefined) {
    return v == null ? "—" : String(Math.round(v));
  }

  function ganho(g: Ganho): { texto: string; tom: "ok" | "perigo" | "neutro"; medido: boolean } {
    if (g.estado === "Medido" && g.pct != null) {
      return { texto: `${g.pct >= 0 ? "+" : ""}${g.pct.toFixed(1).replace(".", ",")}%`, tom: g.pct >= 0 ? "ok" : "perigo", medido: true };
    }
    if (g.estado === "SemDiferenca") return { texto: "Sem diferença", tom: "neutro", medido: true };
    if (g.estado === "Medindo") {
      return { texto: `Medindo (${g.antes} antes · ${g.depois} depois)`, tom: "neutro", medido: false };
    }
    return { texto: "—", tom: "neutro", medido: false };
  }
</script>

<!-- O que o FiveM põe dentro dos cartões do catálogo (aba Otimizar). -->
{#snippet graficosDoGta()}<GraficosGta />{/snippet}
{#snippet pastasDoFivem()}<FivemPainel parte="cache" />{/snippet}

<div class="jogos">
  {#if erro}
    <div class="recado" role="alert">
      <span class="ponto" data-tom="perigo"></span>
      <div><strong>Não consegui ler seus jogos.</strong><span class="apoio">{erro}</span></div>
    </div>
  {:else if itens === null}
    <Esqueleto linhas={4} rotulo="Lendo seus jogos" />
  {:else if itens.length === 0}
    <p class="vazio">Não achei jogos instalados pela Steam, pela Epic ou por instalador. Jogo instalado em outro lugar também serve: abra ele com o Otimiza aberto, e a partida é reconhecida e medida.</p>
  {:else}
    {#if atual}
      <section class="secao destaque">
        <!-- A capa desfocada vira a luz do cartão, como a página de um jogo numa loja -->
        {#if atual.capa}<div class="luz-da-capa" style:background-image={`url("${atual.capa}")`} aria-hidden="true"></div>{/if}
        <div class="capa-grande">
          {#if atual.capa}<img src={atual.capa} alt="" onerror={() => fotoFalhou(atual.jogo.nome)} />{:else}<span class="reserva grande">{iniciais(atual.jogo.nome)}</span>{/if}
        </div>
        <div class="info">
          <div class="titulo-jogo">
            <h3>{atual.jogo.nome}</h3>
            {#if atual.jogo.loja !== SO_MEDIDO}
              <button class="btn primario" onclick={() => window.dispatchEvent(new CustomEvent(JOGAR, { detail: atual.jogo.nome }))}>Jogar</button>
            {/if}
          </div>
          <p class="apoio">
            {[atual.jogo.loja, gb(atual.jogo.gb), quando(atual.jogo.ultima_vez) && `jogado ${quando(atual.jogo.ultima_vez)}`]
              .filter(Boolean)
              .join(" · ")}
          </p>
          {#if atual.medido?.ultima}
            {@const u = atual.medido.ultima}
            <div class="numeros">
              <div><span class="numero valor">{inteiro(u.fps_nativo)}</span><span class="apoio">FPS na última</span></div>
              <div><span class="numero valor">{inteiro(u.low_1pct)}</span><span class="apoio">no 1% pior</span></div>
              {#if u.fps_na_tela != null}
                <div><span class="numero valor">{inteiro(u.fps_na_tela)}</span><span class="apoio">na tela</span></div>
              {/if}
              <div><span class="numero valor">{inteiro(atual.medido.melhor_fps)}</span><span class="apoio">melhor FPS</span></div>
              {#if u.clique_ms != null}
                <div title="Do clique do mouse até a imagem mudar na tela, medido pelo PresentMon na última partida">
                  <span class="numero valor">{Math.round(u.clique_ms)}<small> ms</small></span><span class="apoio">clique até a tela</span>
                </div>
              {/if}
            </div>
            <span class="selo medido origem">medido neste PC</span>
          {:else}
            <p class="sem-medida">Ainda não medi este jogo. Jogue uma partida com o Otimiza aberto e os números aparecem aqui.</p>
          {/if}
        </div>
      </section>

      <section class="secao pagina-do-jogo">
        <Abas
          abas={abasDoJogo.map(([id, rotulo]) => ({ id, rotulo }))}
          ativa={abaAtual}
          rotulo={`Página do ${atual.jogo.nome}`}
          prefixo="jogo"
          idDoPainel="painel-do-jogo"
          onmudar={(id) => (aba = id)}
        />
        <div id="painel-do-jogo" class="grade-cartoes" class:coluna={abaAtual === "historico"} class:otimizar={abaAtual === "otimizar"} role="tabpanel" aria-labelledby={`jogo-aba-${abaAtual}`}>
          {#key `${atual.jogo.nome}|${abaAtual}`}
            {#if abaAtual === "resumo"}
              <div class="largura-toda"><TurboDoJogo jogo={atual.jogo.nome} /></div>
              {#if atual.medido?.ultima}
              <Cartao titulo="Ganho desde o Otimiza">
                {#if atual.medido?.ultima}
                  {@const gf = ganho(atual.medido.ganho_fps)}
                  {@const gl = ganho(atual.medido.ganho_low)}
                  <ul class="lista ganhos">
                    <li>
                      <span class="ponto" data-tom={gf.tom}></span><span class="rot">No FPS</span>
                      <span class="numero">{gf.texto}</span>{#if gf.medido}<span class="selo medido">medido</span>{/if}
                    </li>
                    <li>
                      <span class="ponto" data-tom={gl.tom}></span><span class="rot">No 1% pior</span>
                      <span class="numero">{gl.texto}</span>{#if gl.medido}<span class="selo medido">medido</span>{/if}
                    </li>
                    <li>
                      <span class="ponto"></span><span class="rot">Partidas medidas</span><span class="numero">{atual.medido.partidas}</span>
                    </li>
                  </ul>
                  <p class="apoio pequeno">O ganho só vira número quando as partidas de antes e de depois não se confundem na margem de erro.</p>
                {/if}
              </Cartao>
              {/if}
              <PcsParecidos jogo={atual.jogo.nome} />
            {:else if abaAtual === "otimizar"}
              {#if doJogo}
                {@const fivem = doJogo.extras.includes("fivem")}
                <CatalogoDoJogo jogo={doJogo} configExtra={fivem ? graficosDoGta : undefined} cacheExtra={fivem ? pastasDoFivem : undefined} />
              {:else}
                <Cartao titulo="Fora do catálogo por enquanto">
                  <p class="apoio">
                    O catálogo tem a configuração, o cache e as regras do anticheat de {CATALOGO.map((j) => j.nome).join(", ")}. Para
                    este jogo, o Turbo, o Frame Boost e a medição de partidas funcionam do mesmo jeito.
                  </p>
                  {#snippet acoes()}<button class="btn" onclick={() => (aba = "resumo")}>Ver o Resumo</button>{/snippet}
                </Cartao>
              {/if}
            {:else if abaAtual === "ferramentas"}
              {#if doJogo?.extras.includes("fivem")}<FivemPainel parte="crashes" />{/if}
              {#if doJogo?.extras.includes("cs2_gsi")}<Cs2Integracao />{/if}
            {:else}
              <LinhaDoTempo jogo={atual.jogo.nome} />
            {/if}
          {/key}
        </div>
      </section>
    {/if}

    <section class="secao todos">
    <div class="barra">
      <label class="campo-busca">
        <Icone nome="busca" tamanho={14} />
        <input bind:value={filtro} placeholder="Buscar jogo" aria-label="Buscar jogo" />
      </label>
      <div class="segmento" role="radiogroup" aria-label="Quais jogos">
        <button role="radio" aria-checked={!soMedidos} onclick={() => (soMedidos = false)}>Todos ({itens.length})</button>
        <button role="radio" aria-checked={soMedidos} onclick={() => (soMedidos = true)}>
          Medidos ({itens.filter((i) => i.medido).length})
        </button>
      </div>
    </div>

    {#if visiveis.length === 0}
      <p class="vazio">Nenhum jogo com esse filtro.</p>
    {:else}
      <div class="grade">
        {#each visiveis as i (i.jogo.nome)}
          <button class="cartao" class:ativo={escolhido === i.jogo.nome} onclick={() => (escolhido = i.jogo.nome)}>
            <span class="capa">
              {#if i.capa}<img src={i.capa} alt="" loading="lazy" onerror={() => fotoFalhou(i.jogo.nome)} />{:else}<span class="reserva">{iniciais(i.jogo.nome)}</span>{/if}
              {#if i.medido}<span class="selo medido marca-medido" title="FPS medido pelo Otimiza neste PC">medido</span>{/if}
            </span>
            <span class="nome">{i.jogo.nome}</span>
            <span class="apoio">{i.jogo.loja}</span>
          </button>
        {/each}
      </div>
    {/if}
    </section>
  {/if}
  <OndeOsJogosMoram />
</div>

<style>
  .titulo-jogo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .jogos {
    display: grid;
    gap: 18px;
    padding-top: 14px;
  }
  .vazio {
    padding: 24px 0;
    color: var(--tinta-2);
  }

  .destaque {
    display: flex;
    gap: 26px;
    overflow: hidden;
    isolation: isolate;
  }
  .luz-da-capa {
    position: absolute;
    inset: -30%;
    z-index: -1;
    background-size: cover;
    background-position: center;
    filter: blur(60px) saturate(1.3);
    opacity: 0.28;
    pointer-events: none;
  }
  /* No claro, capa escura desfocada vira um cinza sujo: só um toque, clareado. */
  :global(:root[data-tema="dia"]) .luz-da-capa {
    opacity: 0.1;
    filter: blur(60px) saturate(1.6) brightness(1.5);
  }
  .capa-grande {
    flex: none;
    align-self: flex-start;
    width: 176px;
    aspect-ratio: 2 / 3;
    border-radius: var(--raio-2);
    overflow: hidden;
    box-shadow: var(--sombra-2);
  }
  .capa-grande img,
  .capa img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .reserva {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    background: var(--superficie-2);
    color: var(--tinta-2);
    font-family: var(--fonte-titulo);
    font-size: var(--t-titulo);
    font-weight: 600;
  }
  .reserva.grande {
    font-size: var(--t-numero);
  }
  .info {
    flex: 1;
    min-width: 0;
    display: grid;
    align-content: start;
    gap: 10px;
  }
  .info h3 {
    margin: 0;
    font-family: var(--fonte-titulo);
    font-size: var(--t-destaque);
    font-weight: 700;
    letter-spacing: -0.025em;
  }
  .numeros {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin: 6px 0 2px;
  }
  /* Cada número num ladrilho de vidro: o número grande, o nome embaixo. */
  .numeros > div {
    min-width: 104px;
    padding: 12px 16px 10px;
    border-radius: var(--raio-2);
    background: var(--bloco, var(--superficie-2));
    border: 1px solid var(--linha);
  }
  .numeros div {
    display: grid;
  }
  .valor {
    font-size: var(--t-destaque);
    font-weight: 700;
    line-height: 1.05;
    letter-spacing: -0.03em;
  }
  .origem {
    justify-self: start;
  }
  .ganhos > li {
    padding: 9px 0;
  }
  .ganhos .rot {
    flex: 1;
  }

  .pagina-do-jogo {
    gap: 16px;
  }
  /* Os cartões da sub-aba lado a lado; numa janela estreita, um embaixo do outro. O Histórico é uma linha do tempo:
     uma coluna só, na largura de leitura. */
  .grade-cartoes {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    align-items: start;
    gap: 14px;
  }
  .grade-cartoes.coluna {
    grid-template-columns: minmax(0, 78ch);
  }
  .largura-toda,
  .grade-cartoes.otimizar > :global(:last-child) {
    grid-column: 1 / -1;
  }
  .largura-toda {
    display: grid;
  }
  .sem-medida {
    max-width: 52ch;
    color: var(--tinta-2);
  }

  .barra {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .campo-busca {
    flex: 1;
    min-width: 180px;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    border-radius: var(--raio-3);
    background: var(--superficie-2);
    color: var(--tinta-2);
  }
  .campo-busca input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--tinta);
    user-select: text;
  }

  .grade {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
    gap: 14px;
  }
  .cartao {
    display: grid;
    gap: 4px;
    padding: 6px;
    border: 0;
    border-radius: var(--raio-2);
    background: transparent;
    color: var(--tinta);
    text-align: left;
    cursor: pointer;
    transition:
      background var(--mov-rapido) ease,
      transform var(--mov-medio) var(--curva-mola);
  }
  .cartao:hover {
    background: var(--superficie-2);
  }
  .cartao.ativo {
    background: var(--superficie-2);
    box-shadow: inset 0 0 0 2px var(--tinta);
  }
  .capa {
    position: relative;
    aspect-ratio: 2 / 3;
    border-radius: var(--raio-1);
    overflow: hidden;
    box-shadow: 0 6px 16px rgb(0 0 0 / 0.14);
  }
  .marca-medido {
    position: absolute;
    right: 6px;
    top: 6px;
  }
  .nome {
    font-size: var(--t-linha);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
