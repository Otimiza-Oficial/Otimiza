<script lang="ts">
  // "No jogo" (trazido do Boost da 3.3, `src/v33/app.ts`: `cartaoJogoLiso`, `cartaoMemoria`, `cartaoConfigDosJogos`):
  // - Jogo liso: mede 20 s e propõe travar o FPS num ritmo constante (o que o olho sente, mais que o número).
  // - Memória para o jogo: quanto está livre e o que pesa, com "Fechar" para cada programa.
  // - Configuração do jogo: o perfil leve no arquivo que o próprio jogo grava (só com o jogo fechado; desfaz byte a byte).
  import { onMount } from "svelte";
  import { ponte } from "../lib/ponte";
  import { ferramentas, type ConfigDoJogo, type MemoriaParaOJogo, type PlanoJogoLiso } from "../lib/ferramentas";

  const TELA = { Direta: "direto ao monitor", Copiada: "copiado pelo Windows", Composta: "composto pelo Windows", Desconhecida: "não identificado" } as const;

  let liso = $state<PlanoJogoLiso | null>(null);
  let lisoAntes = $state<{ trancos: number; fps: number } | null>(null);
  let medindo = $state(false);
  let lisoRecado = $state<string | null>(null);
  let travado = $state(false);

  let memoria = $state<MemoriaParaOJogo | null>(null);
  let fechando = $state<string | null>(null);
  let memRecado = $state<string | null>(null);

  let configs = $state<ConfigDoJogo[] | null>(null);
  let cfgOcupado = $state<string | null>(null);
  let cfgRecado = $state<string | null>(null);

  const n = (v: number, c = 0) => v.toLocaleString("pt-BR", { maximumFractionDigits: c, minimumFractionDigits: c });

  onMount(() => {
    ferramentas.memoriaParaOJogo().then((m) => (memoria = m)).catch(() => (memRecado = "Não consegui ler a memória."));
    void lerConfigs();
  });

  async function lerConfigs() {
    configs = await ferramentas.configDosJogos().catch(() => {
      cfgRecado = "Não consegui ler a configuração dos jogos.";
      return null;
    });
  }

  async function medirLiso() {
    medindo = true;
    lisoRecado = null;
    travado = false;
    try {
      const p = await ferramentas.jogoLisoMedir();
      try {
        lisoAntes = JSON.parse(localStorage.getItem("otimiza-liso-ultima") ?? "null");
        localStorage.setItem("otimiza-liso-ultima", JSON.stringify({ trancos: p.trancos_por_minuto, fps: p.fps_agora, quando: Date.now() }));
      } catch {
        lisoAntes = null;
      }
      liso = p;
    } catch (e) {
      lisoRecado = String(e);
    } finally {
      medindo = false;
    }
  }

  async function travar() {
    try {
      await ferramentas.jogoLisoAplicar();
      travado = true;
    } catch (e) {
      lisoRecado = String(e);
    }
  }

  async function fechar(nome: string) {
    fechando = nome;
    memRecado = null;
    try {
      memoria = await ferramentas.memoriaFechar(nome);
    } catch (e) {
      memRecado = String(e);
    } finally {
      fechando = null;
    }
  }

  async function config(j: ConfigDoJogo) {
    cfgOcupado = j.id;
    cfgRecado = null;
    try {
      if (j.aplicado) await ponte.desfazer(j.historico);
      else await ferramentas.configAplicar(j.id);
      await lerConfigs();
    } catch (e) {
      cfgRecado = String(e);
    } finally {
      cfgOcupado = null;
    }
  }

  const achados = $derived((configs ?? []).filter((j) => j.arquivo));
</script>

<div class="no-jogo">
  <section class="secao">
    <h2>Jogo liso</h2>
    {#if !liso}
      <p class="apoio">Trava o FPS do jogo num ritmo constante: é o que o olho sente como liso, mais que o número. Abra o jogo e meça.</p>
    {:else}
      <p>O jogo entrega <b>{n(liso.fps_agora)} fps</b> e trava <b>{n(liso.trancos_por_minuto)}×/min</b> · monitor de {liso.hz} Hz. <span class="selo medido">medido</span></p>
      {#if lisoAntes}
        <p class="apoio">Medição anterior: {n(lisoAntes.trancos)}×/min a {n(lisoAntes.fps)} fps → agora {n(liso.trancos_por_minuto)}×/min a {n(liso.fps_agora)} fps.</p>
      {/if}
      <p class="apoio">{liso.motivo}</p>
      {#if liso.aviso_do_teto}
        <p class="aviso">{liso.aviso_do_teto}</p>
      {:else if liso.teto_no_driver}
        <p class="apoio">Travado no driver em <b>{liso.teto_no_driver} fps</b> neste jogo.</p>
      {/if}
      {#if liso.tela}
        <p class="apoio">Chegada à tela: <b>{TELA[liso.tela.apresentacao]}</b></p>
        {#if liso.tela.aviso}<p class="aviso">{liso.tela.aviso}</p>{/if}
      {/if}
    {/if}
    <div class="acoes">
      <button class="btn" class:primario={!liso} disabled={medindo} onclick={medirLiso}>{medindo ? "Medindo 20 s…" : liso ? "Medir de novo" : "Medir o jogo"}</button>
      {#if liso?.teto != null}
        <button class="btn primario" disabled={travado} onclick={travar}>{travado ? `Travado em ${liso.teto} fps — reabra o jogo` : `Travar em ${liso.teto} fps`}</button>
      {/if}
    </div>
    {#if liso?.teto != null}<p class="apoio pequeno">Vale na próxima vez que você abrir o jogo. Desfaz pelo Histórico.</p>{/if}
    {#if lisoRecado}<p class="aviso">{lisoRecado}</p>{/if}
  </section>

  <section class="secao">
    <h2>Memória para o jogo <span class="selo medido">medido</span></h2>
    {#if memoria}
      {@const apertada = memoria.livre_gb < 2.5 && memoria.total_gb <= 16}
      {@const soma = memoria.pesados.reduce((s, p) => s + p.mb, 0)}
      <p><b>{n(memoria.livre_gb, 1)} GB livres</b> de {n(memoria.total_gb, 1)} GB.{apertada ? " Pouco para jogar: sem memória, o jogo busca no disco e trava quando carrega cenário novo." : ""}</p>
      {#if memoria.pesados.length}
        <p class="apoio">Feche antes de jogar para liberar até <b>{n(soma / 1024, 1)} GB</b>:</p>
        <ul class="lista-pesados">
          {#each memoria.pesados as p (p.nome)}
            <li>
              <div>
                <span>{p.nome} · {n(p.mb / 1024, 1)} GB{p.processos > 1 ? ` (${p.processos} processos)` : ""}</span>
                {#if p.aviso}<p class="apoio pequeno">{p.aviso}</p>{/if}
              </div>
              <button class="btn" disabled={fechando !== null} onclick={() => fechar(p.nome)}>{fechando === p.nome ? "Fechando…" : "Fechar"}</button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="apoio">Nada pesado aberto além do jogo.</p>
      {/if}
    {/if}
    {#if memRecado}<p class="aviso">{memRecado}</p>{/if}
  </section>

  <section class="secao">
    <h2>Configuração do jogo</h2>
    <p class="apoio">O que custa caro e quase não se vê, no arquivo que o próprio jogo grava. Só muda o que já existe nele, nunca com o jogo aberto, e tudo se desfaz byte a byte.</p>
    {#if configs}
      {#if achados.length === 0}
        <p class="apoio">Nenhum dos jogos cobertos (Fortnite, CS2, Apex) foi achado neste PC.</p>
      {/if}
      {#each achados as j (j.id)}
        <div class="jogo-cfg">
          <div>
            <span class="titulo">{j.nome}</span>
            {#if j.aplicado}
              <p class="apoio pequeno">Perfil leve aplicado pelo Otimiza. O desfazer devolve o arquivo exatamente como estava.</p>
            {:else if j.ilegivel}
              <p class="aviso">{j.ilegivel}</p>
            {:else if j.previsao.length === 0}
              <p class="apoio pequeno">Já está leve: nada a mudar.</p>
            {:else}
              {#if j.aberto}<p class="aviso">Feche o jogo para aplicar: ele reescreve a configuração ao sair.</p>{/if}
              <ul class="previsao">
                {#each j.previsao as m (m.chave)}
                  <li>{m.por_que} <span class="apoio">({m.antes} → {m.depois})</span></li>
                {/each}
              </ul>
            {/if}
          </div>
          {#if j.aplicado || j.previsao.length}
            <button class="btn" disabled={cfgOcupado !== null || (!j.aplicado && j.aberto)} onclick={() => config(j)}>
              {cfgOcupado === j.id ? "…" : j.aplicado ? "Desfazer" : "Aplicar"}
            </button>
          {/if}
        </div>
      {/each}
      <p class="apoio pequeno">Valorant e Warzone: em estudo. A configuração deles ainda não é escrita.</p>
    {/if}
    {#if cfgRecado}<p class="aviso">{cfgRecado}</p>{/if}
  </section>
</div>

<style>
  .no-jogo {
    display: grid;
    gap: 16px;
  }
  p {
    margin: 0;
  }
  .secao {
    display: grid;
    gap: 8px;
  }
  .acoes {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .aviso {
    color: var(--atencao);
    font-size: 13px;
  }
  .pequeno {
    font-size: 12px;
  }
  .lista-pesados {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .lista-pesados li,
  .jogo-cfg {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-size: 13px;
  }
  .jogo-cfg {
    padding-top: 8px;
    border-top: 1px solid var(--linha);
    align-items: flex-start;
  }
  .titulo {
    font-weight: 500;
  }
  .previsao {
    margin: 4px 0 0;
    padding-left: 18px;
    display: grid;
    gap: 4px;
    font-size: 12px;
  }
</style>
