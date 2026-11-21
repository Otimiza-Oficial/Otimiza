<script lang="ts">
  // Um lado do laboratório de antes e depois, para a primeira execução guiada (1.0.12, Frente 1.5): medir o jogo aberto
  // por 30 s, quantas vezes a pessoa quiser, contando as rodadas. No "Depois", mostra o resultado assim que houver os
  // dois lados. Usa o mesmo laboratório da área Prova (`labMedir`): o que se mede aqui aparece lá.
  import { onMount } from "svelte";
  import Botao from "./base/Botao.svelte";
  import Estado from "./base/Estado.svelte";
  import { relogio } from "../lib/relogio";
  import { ponte, type LadoDoLab, type ResultadoDoLab } from "../lib/ponte";

  const SEGUNDOS = 30;
  const CONFIANCA: Record<string, { texto: string; tom: "ok" | "neutro" | "atencao" }> = {
    Alta: { texto: "diferença clara", tom: "ok" },
    Media: { texto: "diferença pequena, mas real", tom: "ok" },
    SemDiferenca: { texto: "dentro do ruído", tom: "neutro" },
    Insuficiente: { texto: "faltam rodadas", tom: "atencao" },
  };

  let { lado, onmudou }: { lado: LadoDoLab; onmudou?: (lab: ResultadoDoLab) => void } = $props();

  let lab = $state<ResultadoDoLab | null>(null);
  let medindo = $state(false);
  let passou = $state(0);
  let erro = $state<{ titulo: string; fazer: string; detalhe?: string } | null>(null);

  const feitas = $derived(lab ? (lado === "Antes" ? lab.rodadas_antes : lab.rodadas_depois) : 0);
  const minimo = $derived(lab?.minimo_por_lado ?? 3);
  const comparavel = $derived(!!lab && lab.rodadas_antes > 0 && lab.rodadas_depois > 0);

  onMount(async () => {
    lab = await ponte.labEstado().catch(() => null);
    if (lab) onmudou?.(lab);
  });

  async function medir() {
    erro = null;
    const processo = await ponte.processoDoJogo().catch(() => null);
    if (!processo) {
      erro = { titulo: "Nenhum jogo aberto", fazer: "Abra o seu jogo, entre numa partida ou no mapa e volte aqui (Alt+Tab). Meça sempre no mesmo lugar." };
      return;
    }
    medindo = true;
    passou = 0;
    // O contador da tela anda pelo relógio central (`lib/relogio.ts`), não por um setInterval próprio.
    const parar = relogio.cada(1000, () => (passou = Math.min(SEGUNDOS, passou + 1)));
    try {
      lab = await ponte.labMedir(processo, lado, SEGUNDOS);
      onmudou?.(lab);
    } catch (e) {
      erro = { titulo: "A medição não terminou", fazer: "Deixe o jogo em primeiro plano durante os 30 segundos e tente de novo.", detalhe: String(e) };
    } finally {
      parar();
      medindo = false;
    }
  }

  function num(v: number | null | undefined, casas = 1) {
    return v == null ? "—" : v.toFixed(casas).replace(".", ",");
  }
</script>

<div class="medicao">
  <div class="contagem">
    <div class="rodadas" aria-label={`${feitas} de ${minimo} rodadas`}>
      {#each Array.from({ length: Math.max(minimo, feitas) }, (_, i) => i) as i (i)}
        <span class="rodada" class:feita={i < feitas} class:agora={medindo && i === feitas}></span>
      {/each}
    </div>
    <p class="apoio">
      {feitas === 0
        ? `Nenhuma rodada ${lado === "Antes" ? "antes" : "depois"} ainda.`
        : `${feitas} ${feitas === 1 ? "rodada" : "rodadas"} ${lado === "Antes" ? "antes" : "depois"}${feitas >= minimo ? ": o suficiente para comparar." : `: faça ${minimo - feitas} a mais para um resultado confiável.`}`}
    </p>
  </div>

  <Botao variante={feitas >= minimo ? "contorno" : "principal"} trabalhando={medindo} textoTrabalhando={`Medindo · ${passou} de ${SEGUNDOS} s`} onclick={medir}>
    {feitas === 0 ? `Medir ${SEGUNDOS} s` : "Medir mais uma rodada"}
  </Botao>

  {#if erro}
    <Estado tipo="erro" compacto titulo={erro.titulo} fazer={erro.fazer} detalhe={erro.detalhe ?? null} />
  {/if}

  {#if lado === "Depois" && comparavel && lab}
    <table class="tabela">
      <caption class="rotulo">Resultado · {lab.nome}</caption>
      <thead><tr><th>Medida</th><th class="n">Antes</th><th class="n">Depois</th><th>Confiança</th></tr></thead>
      <tbody>
        {#each lab.linhas.slice(0, 3) as l (l.metrica)}
          {@const c = CONFIANCA[l.confianca] ?? { texto: l.confianca, tom: "neutro" }}
          <tr>
            <td>{l.metrica}</td>
            <td class="n numero">{num(l.antes)}</td>
            <td class="n numero">{num(l.depois)}</td>
            <td><span class="conf"><span class="ponto" data-tom={c.tom}></span>{c.texto}</span></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .medicao {
    display: grid;
    gap: var(--e-4);
    justify-items: start;
  }
  .contagem {
    display: grid;
    gap: var(--e-2);
  }
  .rodadas {
    display: flex;
    gap: 6px;
  }
  .rodada {
    width: 28px;
    height: 4px;
    border-radius: 2px;
    background: var(--superficie-3);
  }
  .rodada.feita {
    background: var(--tinta);
  }
  .rodada.agora {
    background: var(--tinta-3);
  }
  .tabela {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--t-linha);
  }
  .tabela caption {
    padding-bottom: var(--e-2);
    text-align: left;
  }
  .tabela th {
    padding: 0 8px 6px 0;
    text-align: left;
    font-weight: 500;
    font-size: var(--t-apoio);
    color: var(--tinta-2);
    border-bottom: 1px solid var(--linha);
  }
  .tabela td {
    padding: 8px 8px 8px 0;
    border-bottom: 1px solid var(--linha);
  }
  .tabela tr:last-child td {
    border-bottom: 0;
  }
  .n {
    text-align: right;
  }
  .conf {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
