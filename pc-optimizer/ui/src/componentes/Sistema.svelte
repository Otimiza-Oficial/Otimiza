<script lang="ts">
  // A área Sistema: todos os ajustes do catálogo por grupo (cada um com o efeito honesto, aplicar e desfazer), o que
  // abre junto com o Windows, a limpeza de temporários e o ponto de restauração. Ajuste que pode custar FPS diz quando;
  // ajuste que não deu para verificar diz isso, em vez de fingir "já está bom".
  import { onMount } from "svelte";
  import AoVivo from "./AoVivo.svelte";
  import Bios from "./Bios.svelte";
  import SistemaAvancado from "./SistemaAvancado.svelte";
  import {
    ponte,
    type AjusteDoCatalogo,
    type ItemDaLimpeza,
    type RestoreStatus,
    type StartupEntry,
  } from "../lib/ponte";

  export type Aba = "agora" | "ajustes" | "inicializacao" | "limpeza" | "restauracao" | "bios" | "avancado";

  let { abaInicial = "ajustes" }: { abaInicial?: Aba } = $props();

  const GRUPO: Record<string, string> = {
    Gaming: "Jogos",
    System: "Sistema",
    Network: "Rede",
    Startup: "Inicialização",
    Privacy: "Privacidade",
  };
  const ORDEM = ["Gaming", "System", "Network", "Startup", "Privacy"];
  const ESTADO: Record<string, { texto: string; tom: "ok" | "neutro" | "atencao" }> = {
    Applied: { texto: "Aplicado", tom: "ok" },
    AlreadyOptimal: { texto: "Já está assim", tom: "neutro" },
    Available: { texto: "Disponível", tom: "atencao" },
    Unavailable: { texto: "Não se aplica aqui", tom: "neutro" },
    Unknown: { texto: "Não deu para verificar", tom: "neutro" },
  };
  const GANHO: Record<string, string> = {
    Measurable: "ganho medível",
    Situational: "depende do PC",
    Responsiveness: "resposta mais rápida",
    NoGain: "não muda FPS",
  };
  const CLASSE: Record<string, string> = {
    Essencial: "essencial (som, mouse, drivers)",
    Util: "útil",
    Opcional: "pode desligar",
    Desconhecido: "desconhecido",
  };

  // svelte-ignore state_referenced_locally
  let aba = $state<Aba>(abaInicial);
  let catalogo = $state<AjusteDoCatalogo[] | null>(null);
  let inicializacao = $state<StartupEntry[] | null>(null);
  let limpeza = $state<ItemDaLimpeza[] | null>(null);
  let restauracao = $state<RestoreStatus | null>(null);
  let soRecomendados = $state(true);
  let avancados = $state(false);
  let ocupado = $state<string | null>(null);
  let recado = $state<{ tom: "ok" | "perigo" | "atencao"; texto: string } | null>(null);
  let erro = $state<string | null>(null);

  const grupos = $derived.by(() => {
    const lista = (catalogo ?? []).filter(
      (a) =>
        (!a.retirado || a.state === "Applied") &&
        (avancados || !a.expert || a.state === "Applied") &&
        (!soRecomendados || a.recommended || a.state === "Applied")
    );
    return ORDEM.map((g) => ({ id: g, itens: lista.filter((a) => a.category === g) })).filter((g) => g.itens.length > 0);
  });
  const aplicados = $derived((catalogo ?? []).filter((a) => a.state === "Applied").length);

  async function carregar(a: Aba) {
    erro = null;
    try {
      if (a === "ajustes" && !catalogo) catalogo = await ponte.catalogo();
      if (a === "inicializacao" && !inicializacao) inicializacao = await ponte.inicializacao();
      if (a === "limpeza" && !limpeza) limpeza = await ponte.medirLimpeza();
      if (a === "restauracao" && !restauracao) restauracao = await ponte.restauracao();
    } catch (e) {
      erro = String(e);
    }
  }

  onMount(() => void carregar("ajustes"));
  $effect(() => {
    void carregar(aba);
  });

  async function alternarAjuste(a: AjusteDoCatalogo) {
    ocupado = a.id;
    recado = null;
    try {
      const r = a.state === "Applied" ? await ponte.desfazerAjuste(a.id) : await ponte.aplicarAjuste(a.id);
      recado = r.success
        ? { tom: r.requires_restart ? "atencao" : "ok", texto: `${a.name}: ${r.applied ? "aplicado" : "desfeito"}.${r.requires_restart ? " Vale depois de reiniciar." : ""}` }
        : { tom: "perigo", texto: `${a.name}: ${r.message}` };
      catalogo = await ponte.catalogo();
    } catch (e) {
      recado = { tom: "perigo", texto: `${a.name}: ${String(e)}` };
    } finally {
      ocupado = null;
    }
  }

  async function alternarInicializacao(e: StartupEntry) {
    ocupado = `ini:${e.name}`;
    recado = null;
    try {
      const r = await ponte.definirInicializacao(e, !e.enabled);
      if (!r.success) recado = { tom: "perigo", texto: `${e.name}: ${r.message}` };
      inicializacao = await ponte.inicializacao();
    } catch (x) {
      recado = { tom: "perigo", texto: `${e.name}: ${String(x)}` };
    } finally {
      ocupado = null;
    }
  }

  async function limpar(item: ItemDaLimpeza) {
    ocupado = `limpa:${item.id}`;
    recado = null;
    try {
      const r = await ponte.limpar(item.id);
      recado = {
        tom: "ok",
        texto: `${item.nome}: ${tamanho(r.bytes_liberados)} liberados (${r.apagados} arquivos).${r.ficaram ? ` ${r.ficaram} estavam em uso e ficaram.` : ""}`,
      };
      limpeza = await ponte.medirLimpeza();
    } catch (e) {
      recado = { tom: "perigo", texto: `${item.nome}: ${String(e)}` };
    } finally {
      ocupado = null;
    }
  }

  async function criarPonto() {
    ocupado = "ponto";
    recado = null;
    try {
      recado = { tom: "ok", texto: await ponte.criarPontoDeRestauracao() };
      restauracao = await ponte.restauracao();
    } catch (e) {
      recado = { tom: "perigo", texto: String(e) };
    } finally {
      ocupado = null;
    }
  }

  function tamanho(bytes: number) {
    if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1).replace(".", ",")} GB`;
    if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
    return `${Math.round(bytes / 1e3)} KB`;
  }
</script>

<div class="sistema">
  <div class="segmento abas-internas" role="tablist" aria-label="Partes do Sistema">
    {#each [["agora", "Agora"], ["ajustes", "Ajustes"], ["inicializacao", "Inicialização"], ["limpeza", "Limpeza"], ["restauracao", "Restauração"], ["bios", "BIOS"], ["avancado", "Avançado"]] as [id, nome] (id)}
      <button role="tab" aria-selected={aba === id} onclick={() => (aba = id as Aba)}>{nome}</button>
    {/each}
  </div>

  {#if erro}
    <div class="recado" role="alert"><span class="ponto" data-tom="perigo"></span><div><span>{erro}</span></div></div>
  {/if}
  {#if recado}
    <div class="recado" role="status"><span class="ponto" data-tom={recado.tom}></span><div><span>{recado.texto}</span></div></div>
  {/if}

  {#if aba === "agora"}
    <AoVivo />
  {:else if aba === "ajustes"}
    {#if catalogo === null}
      <p class="vazio">Lendo os ajustes deste PC…</p>
    {:else}
      <div class="filtros">
        <span class="apoio"><b class="numero">{aplicados}</b> {aplicados === 1 ? "aplicado" : "aplicados"} de {catalogo.length}</span>
        <label class="marcar"><input type="checkbox" bind:checked={soRecomendados} /> Só os recomendados</label>
        <label class="marcar"><input type="checkbox" bind:checked={avancados} /> Mostrar avançados</label>
      </div>
      {#each grupos as g (g.id)}
        <section class="secao">
          <h2>{GRUPO[g.id] ?? g.id}</h2>
          <ul class="lista">
            {#each g.itens as a (a.id)}
              {@const est = ESTADO[a.state] ?? { texto: a.state, tom: "neutro" }}
              <li class="ajuste">
                <div class="info">
                  <strong>{a.name}</strong>
                  <span class="apoio">{a.honest_effect}</span>
                  <span class="etiquetas">
                    <span class="estado"><span class="ponto" data-tom={est.tom}></span>{est.texto}</span>
                    <span>· {GANHO[a.expected_gain] ?? a.expected_gain}</span>
                    {#if a.requires_admin}<span>· administrador</span>{/if}
                    {#if a.requires_restart}<span>· reiniciar</span>{/if}
                  </span>
                  {#if a.risco_de_fps.risco === "PodeCustar"}
                    <span class="risco">Pode custar FPS: {a.risco_de_fps.rotulo} ({a.risco_de_fps.quando})</span>
                  {/if}
                  {#if a.security_tradeoff}<span class="risco">Troca segurança por desempenho.</span>{/if}
                  {#if a.detail}<span class="apoio">{a.detail}</span>{/if}
                </div>
                {#if a.state === "Applied" && a.reversible}
                  <button class="btn" disabled={ocupado !== null} onclick={() => alternarAjuste(a)}>
                    {ocupado === a.id ? "Desfazendo…" : "Desfazer"}
                  </button>
                {:else if a.state === "Available"}
                  <button class="btn primario" disabled={ocupado !== null} onclick={() => alternarAjuste(a)}>
                    {ocupado === a.id ? "Aplicando…" : "Aplicar"}
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    {/if}
  {:else if aba === "inicializacao"}
    {#if inicializacao === null}
      <p class="vazio">Lendo o que abre com o Windows…</p>
    {:else if inicializacao.length === 0}
      <p class="vazio">Nada abre junto com o Windows além do próprio sistema.</p>
    {:else}
      <section class="secao">
        <p class="apoio">O que abre junto com o Windows. Desligar aqui é o mesmo que no Gerenciador de Tarefas, e dá para ligar de novo.</p>
        <ul class="lista">
          {#each inicializacao as e (e.hive + e.name)}
            <li>
              <div class="info">
                <strong>{e.name}</strong>
                <span class="apoio comando" title={e.command}>{e.executable} · {CLASSE[e.classe] ?? e.classe}</span>
              </div>
              <button
                class="interruptor"
                role="switch"
                aria-checked={e.enabled}
                aria-label={`${e.name} abre com o Windows`}
                disabled={ocupado !== null}
                onclick={() => alternarInicializacao(e)}
              ></button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {:else if aba === "limpeza"}
    {#if limpeza === null}
      <p class="vazio">Medindo quanto dá para liberar…</p>
    {:else}
      <section class="secao">
        <p class="apoio">Só arquivo com mais de 2 dias, em pasta temporária ou de relatório. O que estiver em uso fica. Apagar não tem volta.</p>
        <ul class="lista">
          {#each limpeza as item (item.id)}
            <li>
              <div class="info">
                <strong>{item.nome}</strong>
                <span class="apoio">{item.explica}</span>
              </div>
              <span class="numero tamanho">{tamanho(item.bytes)}</span>
              <button class="btn" disabled={ocupado !== null || item.bytes < 1e6} onclick={() => limpar(item)}>
                {ocupado === `limpa:${item.id}` ? "Limpando…" : "Limpar"}
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {:else if aba === "bios"}
    <Bios />
  {:else if aba === "avancado"}
    <SistemaAvancado />
  {:else}
    {#if restauracao === null}
      <p class="vazio">Lendo a proteção do sistema…</p>
    {:else}
      <section class="secao">
        <div class="linha">
          <div class="info">
            <strong>Ponto de restauração</strong>
            <span class="apoio">{restauracao.message}</span>
          </div>
          <button class="btn primario" disabled={ocupado !== null || !restauracao.available} onclick={criarPonto}>
            {ocupado === "ponto" ? "Criando… pode levar um minuto" : "Criar agora"}
          </button>
        </div>
        {#if restauracao.points.length > 0}
          <ul class="lista">
            {#each restauracao.points.slice(-6).reverse() as p (p.sequence)}
              <li><span class="ponto"></span><span class="info">{p.description}</span><span class="apoio">{p.created_at}</span></li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .sistema {
    display: grid;
    gap: 8px;
    padding-top: 10px;
  }
  .vazio {
    padding: 20px 0;
    color: var(--tinta-2);
  }
  .filtros {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 18px;
    padding-top: 6px;
  }
  .marcar {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--tinta-2);
    cursor: pointer;
  }
  .lista > li {
    align-items: flex-start;
  }
  .info {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 3px;
  }
  .info strong {
    font-weight: 500;
  }
  .etiquetas {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 12px;
    color: var(--tinta-2);
  }
  .estado {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--tinta);
  }
  .risco {
    font-size: 12px;
    color: var(--atencao);
  }
  .comando {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tamanho {
    align-self: center;
    min-width: 70px;
    text-align: right;
    font-weight: 600;
  }
  .lista > li > .btn,
  .lista > li > .interruptor {
    align-self: center;
  }
  .linha {
    display: flex;
    align-items: center;
    gap: 16px;
  }
</style>
