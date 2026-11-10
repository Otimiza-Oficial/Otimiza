<script lang="ts">
  // Suporte e transparência (trazido da interface antiga, `src/main.ts`): o modo seguro (o Otimiza parou o que é
  // automático depois de fechar de repente várias vezes), o relatório para o suporte, a planilha do que foi alterado,
  // a lista completa do que o Otimiza pode alterar (com risco e escopo) e o que ele NÃO faz e por quê.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type ModoSeguro = { ativo: boolean; quedas_seguidas: number };
  type Alteracao = { id: string; titulo: string; o_que_muda: string; risco: "Baixo" | "Medio" | "Alto"; escopo: string; precisa_reiniciar: boolean };
  type NaoFazemos = { id: string; nome: string; natureza: "Placebo" | "Redundante" | "Prejudicial"; porque: string };

  const RISCO = { Baixo: "risco baixo", Medio: "risco médio", Alto: "risco alto" } as const;
  const NATUREZA = { Placebo: "não faz nada", Redundante: "o Windows já faz", Prejudicial: "piora" } as const;

  let modo = $state<ModoSeguro | null>(null);
  let altera = $state<Alteracao[] | null>(null);
  let naoFazemos = $state<NaoFazemos[] | null>(null);
  let recado = $state<string | null>(null);
  let gerando = $state(false);

  onMount(async () => {
    if (!noTauri) {
      modo = { ativo: false, quedas_seguidas: 0 };
      altera = [{ id: "nagle", titulo: "Rede de baixa latência (Nagle)", o_que_muda: "Manda os pacotes pequenos na hora, sem juntar.", risco: "Baixo", escopo: "Windows", precisa_reiniciar: true }];
      naoFazemos = [{ id: "ram-cleaner", nome: "\"Limpador de RAM\"", natureza: "Placebo", porque: "Esvaziar a memória faz o Windows carregar tudo de novo do disco: o jogo trava mais, não menos." }];
      return;
    }
    [modo, altera, naoFazemos] = await Promise.all([
      invoke<ModoSeguro>("modo_seguro").catch(() => null),
      invoke<Alteracao[]>("o_que_o_otimiza_altera").catch(() => null),
      invoke<NaoFazemos[]>("o_que_nao_fazemos").catch(() => null),
    ]);
  });

  async function sairDoModoSeguro() {
    if (noTauri) await invoke("sair_do_modo_seguro").catch(() => {});
    modo = { ativo: false, quedas_seguidas: 0 };
  }

  async function relatorio() {
    gerando = true;
    recado = null;
    try {
      const r = noTauri
        ? await invoke<{ path: string; note: string }>("export_report", { comparison: null })
        : { path: "Documentos\\Otimiza-relatorio.pdf", note: "" };
      recado = `Relatório salvo em ${r.path}. Mande ao suporte pelo Discord.${r.note ? ` ${r.note}` : ""}`;
    } catch (e) {
      recado = String(e);
    } finally {
      gerando = false;
    }
  }

  // O relatório de compatibilidade: como este PC respondeu a cada ajuste de energia (já certo, mudaria, não existe
  // aqui). Só lê. É o dado das máquinas que o projeto não tem: notebook, AMD, Windows de outra edição.
  let compat = $state<string | null>(null);
  let gerandoCompat = $state(false);
  async function compatibilidade() {
    gerandoCompat = true;
    recado = null;
    try {
      compat = noTauri
        ? await invoke<string>("relatorio_de_compatibilidade")
        : "Relatório de compatibilidade (demonstração)\nPlano de energia: 12 já certos, 3 mudariam, 1 não existe neste Windows.";
    } catch (e) {
      recado = String(e);
    } finally {
      gerandoCompat = false;
    }
  }
  async function copiarCompat() {
    if (!compat) return;
    try {
      await navigator.clipboard.writeText(compat);
      recado = "Relatório de compatibilidade copiado. Cole no atendimento junto da sua dúvida.";
    } catch {
      recado = "Não deu para copiar. Selecione o texto e copie à mão.";
    }
  }

  async function planilha() {
    try {
      recado = `Planilha salva em ${noTauri ? await invoke<string>("exportar_alteracoes") : "Documentos\\Otimiza-alteracoes.csv"}.`;
    } catch (e) {
      recado = String(e);
    }
  }
</script>

{#if modo?.ativo}
  <section class="secao alerta">
    <h2>Modo seguro ligado</h2>
    <p class="apoio">O Otimiza fechou de repente {modo.quedas_seguidas} vezes seguidas e parou tudo o que roda sozinho (medição, modo jogo, guarda). Nada do que você aplicou foi desfeito.</p>
    <button class="btn primario" onclick={sairDoModoSeguro}>Religar o automático</button>
  </section>
{/if}

<section class="secao">
  <h2>Suporte e transparência</h2>
  <p class="apoio">O relatório leva hardware, versões e o que foi alterado — nada de senha, token ou arquivo pessoal. Só sai do PC se você mandar.</p>
  <div class="acoes">
    <button class="btn" disabled={gerando} onclick={relatorio}>{gerando ? "Gerando…" : "Gerar relatório para o suporte"}</button>
    <button class="btn" onclick={planilha}>Exportar o que foi alterado (planilha)</button>
    <button class="btn" disabled={gerandoCompat} onclick={compatibilidade}>{gerandoCompat ? "Lendo…" : "Relatório de compatibilidade"}</button>
  </div>
  {#if recado}<p class="apoio">{recado}</p>{/if}
  {#if compat}
    <div class="compat">
      <div class="compat-topo">
        <span class="apoio">Como este PC respondeu aos ajustes de energia. Nada foi alterado para gerar.</span>
        <button class="btn quieto" onclick={copiarCompat}>Copiar</button>
      </div>
      <pre>{compat}</pre>
    </div>
  {/if}

  {#if altera}
    <details>
      <summary>Tudo o que o Otimiza pode alterar ({altera.length})</summary>
      <ul>
        {#each altera as a (a.id)}
          <li><b>{a.titulo}</b> <span class="apoio pequeno">· {RISCO[a.risco]} · {a.escopo}{a.precisa_reiniciar ? " · precisa reiniciar" : ""}</span><p class="apoio pequeno">{a.o_que_muda}</p></li>
        {/each}
      </ul>
    </details>
  {/if}
  {#if naoFazemos}
    <details>
      <summary>O que o Otimiza não faz, e por quê ({naoFazemos.length})</summary>
      <ul>
        {#each naoFazemos as x (x.id)}
          <li><b>{x.nome}</b> <span class="apoio pequeno">· {NATUREZA[x.natureza]}</span><p class="apoio pequeno">{x.porque}</p></li>
        {/each}
      </ul>
    </details>
  {/if}
</section>

<style>
  .acoes {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .alerta {
    border: 1px solid color-mix(in srgb, var(--atencao) 50%, transparent);
  }
  .compat {
    display: grid;
    gap: 6px;
  }
  .compat-topo {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }
  pre {
    max-height: 260px;
    margin: 0;
    padding: 12px;
    overflow: auto;
    border: 1px solid var(--linha);
    border-radius: var(--raio-1);
    background: var(--superficie);
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    user-select: text;
  }
  details {
    font-size: 13px;
  }
  summary {
    cursor: pointer;
    font-weight: 500;
    padding: 4px 0;
  }
  ul {
    margin: 6px 0 0;
    padding-left: 18px;
    display: grid;
    gap: 8px;
  }
  p {
    margin: 2px 0 0;
  }
  .pequeno {
    font-size: 12px;
  }
</style>
