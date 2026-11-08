<script lang="ts">
  // A área Jogos: os jogos instalados (Steam, Epic e instaladores) com a capa da Steam deste PC, e, para os que o
  // Otimiza já mediu, a última partida e o ganho. O ganho só aparece como número quando é MEDIDO (os intervalos de
  // antes e depois não se encostam); senão a tela diz que não houve diferença ou quantas partidas faltam.
  import { onMount } from "svelte";
  import Cs2Integracao from "./Cs2Integracao.svelte";
  import Icone from "./Icone.svelte";
  import { capaRemota } from "../lib/capas";
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

  /** Mesmo nome, mesmo jogo: a biblioteca medida e a lista de instalados vêm de fontes diferentes. */
  function chave(nome: string) {
    return nome.toLowerCase().replace(/[^a-z0-9]/g, "");
  }

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
            jogo: { nome: b.nome, loja: "Medido pelo Otimiza", appid: null, ultima_vez: b.ultima?.quando ?? 0, gb: null },
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

  /** A foto não carregou: o jogo fica com a capa desenhada. */
  function fotoFalhou(nome: string) {
    if (itens) itens = itens.map((i) => (i.jogo.nome === nome ? { ...i, capa: null } : i));
  }

  function corDoJogo(nome: string) {
    let h = 0;
    for (const c of nome) h = (h * 31 + c.charCodeAt(0)) % 360;
    return `linear-gradient(140deg, hsl(${h} 70% 62%), hsl(${(h + 40) % 360} 65% 45%))`;
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

<div class="jogos">
  {#if erro}
    <div class="recado" role="alert">
      <span class="ponto" data-tom="perigo"></span>
      <div><strong>Não consegui ler seus jogos.</strong><span class="apoio">{erro}</span></div>
    </div>
  {:else if itens === null}
    <p class="vazio">Lendo seus jogos…</p>
  {:else if itens.length === 0}
    <p class="vazio">Não achei jogos instalados pela Steam, pela Epic ou por instalador.</p>
  {:else}
    {#if atual}
      <section class="destaque">
        <div class="capa-grande">
          {#if atual.capa}<img src={atual.capa} alt="" onerror={() => fotoFalhou(atual.jogo.nome)} />{:else}<span class="reserva" style:background={corDoJogo(atual.jogo.nome)}><Icone nome="controle" tamanho={40} /></span>{/if}
        </div>
        <div class="info">
          <h3>{atual.jogo.nome}</h3>
          <p class="apoio">
            {[atual.jogo.loja, gb(atual.jogo.gb), quando(atual.jogo.ultima_vez) && `jogado ${quando(atual.jogo.ultima_vez)}`]
              .filter(Boolean)
              .join(" · ")}
          </p>
          {#if atual.medido?.ultima}
            {@const u = atual.medido.ultima}
            {@const gf = ganho(atual.medido.ganho_fps)}
            {@const gl = ganho(atual.medido.ganho_low)}
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
            <ul class="lista ganhos">
              <li>
                <span class="ponto" data-tom={gf.tom}></span><span class="rot">Ganho no FPS</span>
                <span class="numero">{gf.texto}</span>{#if gf.medido}<span class="selo-medido">Medido</span>{/if}
              </li>
              <li>
                <span class="ponto" data-tom={gl.tom}></span><span class="rot">Ganho no 1% pior</span>
                <span class="numero">{gl.texto}</span>{#if gl.medido}<span class="selo-medido">Medido</span>{/if}
              </li>
              <li>
                <span class="ponto"></span><span class="rot">Partidas medidas</span><span class="numero">{atual.medido.partidas}</span>
              </li>
            </ul>
          {:else}
            <p class="sem-medida">Ainda não medi este jogo. Jogue uma partida com o Otimiza aberto e os números aparecem aqui.</p>
          {/if}
          {#if atual.jogo.appid === 730 || /counter-strike 2|^cs2$/i.test(atual.jogo.nome)}
            <Cs2Integracao />
          {/if}
        </div>
      </section>
    {/if}

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
              {#if i.capa}<img src={i.capa} alt="" loading="lazy" onerror={() => fotoFalhou(i.jogo.nome)} />{:else}<span class="reserva" style:background={corDoJogo(i.jogo.nome)}><Icone nome="controle" tamanho={28} /></span>{/if}
              {#if i.medido}<span class="marca-medido" title="Medido pelo Otimiza"><Icone nome="grafico" tamanho={11} /></span>{/if}
            </span>
            <span class="nome">{i.jogo.nome}</span>
            <span class="apoio">{i.jogo.loja}</span>
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
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
    gap: 20px;
    padding-bottom: 18px;
    border-bottom: 1px solid var(--linha);
  }
  .capa-grande {
    flex: none;
    width: 150px;
    aspect-ratio: 2 / 3;
    border-radius: 14px;
    overflow: hidden;
    box-shadow: 0 12px 28px rgb(0 0 0 / 0.18);
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
    color: rgb(255 255 255 / 0.92);
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
    font-size: 22px;
    font-weight: 600;
  }
  .numeros {
    display: flex;
    flex-wrap: wrap;
    gap: 22px;
  }
  .numeros div {
    display: grid;
  }
  .valor {
    font-size: 28px;
    font-weight: 600;
    line-height: 1.1;
  }
  .ganhos > li {
    padding: 9px 0;
  }
  .ganhos .rot {
    flex: 1;
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
    border-radius: 18px;
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
  .segmento {
    display: flex;
    padding: 3px;
    border-radius: 18px;
    background: var(--superficie-2);
  }
  .segmento button {
    height: 30px;
    padding: 0 12px;
    border: 0;
    border-radius: 15px;
    background: transparent;
    color: var(--tinta-2);
    font-size: 13px;
    cursor: pointer;
  }
  .segmento button[aria-checked="true"] {
    background: var(--tinta);
    color: var(--superficie-solida, #fff);
  }
  :global(:root[data-tema="noite"]) .segmento button[aria-checked="true"] {
    color: #1d1d1f;
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
    border-radius: 14px;
    background: transparent;
    color: var(--tinta);
    text-align: left;
    cursor: pointer;
    transition:
      background 160ms ease,
      transform 200ms cubic-bezier(0.3, 1.4, 0.5, 1);
  }
  .cartao:hover {
    background: var(--superficie-2);
    transform: translateY(-3px);
  }
  .cartao.ativo {
    background: var(--superficie-2);
    box-shadow: inset 0 0 0 2px var(--tinta);
  }
  .capa {
    position: relative;
    aspect-ratio: 2 / 3;
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 6px 16px rgb(0 0 0 / 0.14);
  }
  .marca-medido {
    position: absolute;
    right: 6px;
    top: 6px;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #34c759;
    color: #fff;
  }
  .nome {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cartao .apoio {
    font-size: 11px;
  }
</style>
