<script lang="ts">
  // Quando foi que piorou (trazido da interface 3.6, `renderHistorico`): compara as duas últimas medições repetidas de
  // cada métrica e lista o que aconteceu entre elas. Contra as duas últimas, e não contra o melhor já medido: o melhor
  // de sempre é a medição que mais teve sorte. A margem vai SEMPRE junto, é ela que diz se a diferença é real. Sem
  // nenhuma comparação, a seção não aparece (`historico_de_desempenho`).
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Lado = { media: number; n: number; margem: number | null };
  type Regressao = {
    id: string;
    anterior: Lado;
    atual: Lado;
    piorou: boolean;
    suspeitos: { mudancas: string[]; maquina_mudou: string[]; descartados: number; aviso: string; como_provar: string };
  };
  type HistoricoDeDesempenho = { historico: { registros: unknown[]; descartados: number }; regressoes: Regressao[] };

  let regressoes = $state<Regressao[]>([]);

  onMount(async () => {
    if (!noTauri) return;
    try {
      const h = await invoke<HistoricoDeDesempenho>("historico_de_desempenho");
      regressoes = [...h.regressoes].sort((a, b) => Number(b.piorou) - Number(a.piorou));
    } catch {
      regressoes = [];
    }
  });

  const pioraram = $derived(regressoes.filter((r) => r.piorou).length);
  const lado = (l: Lado) =>
    `${l.media.toLocaleString("pt-BR", { maximumFractionDigits: 1 })}${l.margem === null ? "" : ` ± ${l.margem.toLocaleString("pt-BR", { maximumFractionDigits: 1 })}`} (${l.n}×)`;
</script>

{#if regressoes.length}
  <section class="secao">
    <div class="topo">
      <h2>Quando foi que piorou</h2>
      <span class="apoio">{pioraram === 0 ? "nada piorou" : `${pioraram} de ${regressoes.length} pioraram`}</span>
    </div>
    <p class="apoio texto">
      As duas últimas medições repetidas de cada métrica, com o que aconteceu entre elas. Estar no mesmo período não
      prova que foi a causa.
    </p>
    <ul class="lista-piora">
      {#each regressoes as r (r.id)}
        <li>
          <div class="cabeca">
            <strong>{r.id}</strong>
            <span class="estado">{#if r.piorou}<span class="ponto" data-tom="atencao"></span>{/if}{r.piorou ? "piorou" : "sem queda provada"}</span>
          </div>
          <p class="pequeno">Antes {lado(r.anterior)} · agora {lado(r.atual)}</p>
          {#if r.suspeitos.mudancas.length}
            <p class="pequeno">Entre as duas: {r.suspeitos.mudancas.join(", ")}.</p>
            <p class="apoio pequeno">{r.suspeitos.aviso} {r.suspeitos.como_provar}</p>
          {/if}
          {#if r.suspeitos.maquina_mudou.length}<p class="apoio pequeno">A própria máquina mudou no intervalo: {r.suspeitos.maquina_mudou.join("; ")}.</p>{/if}
          {#if r.suspeitos.descartados > 0}
            <p class="apoio pequeno">
              {r.suspeitos.descartados === 1 ? "1 registro antigo já saiu" : `${r.suspeitos.descartados} registros antigos já saíram`} do histórico por limite de tamanho.
            </p>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .topo,
  .cabeca {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }
  .texto {
    max-width: 64ch;
  }
  .lista-piora {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
  }
  .lista-piora li {
    display: grid;
    gap: 3px;
    padding: 10px 0;
    border-top: 1px solid var(--linha);
  }
  .lista-piora li:first-child {
    border-top: 0;
  }
  .estado {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--tinta-2);
    font-size: 12px;
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
</style>
