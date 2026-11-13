<script lang="ts">
  // A área Prova: o Lab de antes e depois. A pessoa mede o jogo aberto algumas vezes ANTES de mudar algo, muda, e mede
  // DEPOIS; o Lab compara com a margem de cada lado e só diz "melhorou" quando os intervalos não se encostam. Com
  // poucas rodadas, diz quantas faltam. Abaixo, a última prova alternada do "Otimizar" e o texto para o Discord.
  import { onMount } from "svelte";
  import Diagnostico from "./Diagnostico.svelte";
  import Partida from "./Partida.svelte";
  import EsperandoODisco from "./EsperandoODisco.svelte";
  import MapaDeDesempenho from "./MapaDeDesempenho.svelte";
  import TestarGrupos from "./TestarGrupos.svelte";
  import { ponte, type LadoDoLab, type ProvaDoTurbo, type ResultadoDoLab } from "../lib/ponte";

  const SEGUNDOS = 30;
  const CONFIANCA: Record<string, { texto: string; tom: "ok" | "neutro" | "atencao" }> = {
    Alta: { texto: "diferença clara", tom: "ok" },
    Media: { texto: "diferença pequena, mas real", tom: "ok" },
    SemDiferenca: { texto: "dentro do ruído", tom: "neutro" },
    Insuficiente: { texto: "faltam rodadas", tom: "atencao" },
  };

  let { jogo = null }: { jogo?: string | null } = $props();

  let lab = $state<ResultadoDoLab | null>(null);
  let prova = $state<ProvaDoTurbo | null>(null);
  let medindo = $state<LadoDoLab | null>(null);
  let passou = $state(0);
  let erro = $state<string | null>(null);
  let copiado = $state<string | null>(null);

  const faltaAntes = $derived(lab ? Math.max(0, lab.minimo_por_lado - lab.rodadas_antes) : 0);
  const faltaDepois = $derived(lab ? Math.max(0, lab.minimo_por_lado - lab.rodadas_depois) : 0);
  const passo = $derived(!lab ? 0 : faltaAntes > 0 ? 1 : lab.rodadas_depois === 0 ? 2 : faltaDepois > 0 ? 3 : 4);

  onMount(async () => {
    lab = await ponte.labEstado().catch((e) => {
      erro = String(e);
      return null;
    });
    prova = await ponte.provaGuardada().catch(() => null);
  });

  async function medir(lado: LadoDoLab) {
    erro = null;
    const processo = await ponte.processoDoJogo().catch(() => null);
    if (!processo) {
      erro = "Abra o jogo primeiro: o Lab mede o jogo aberto.";
      return;
    }
    medindo = lado;
    passou = 0;
    const t = setInterval(() => (passou += 1), 1000);
    try {
      lab = await ponte.labMedir(processo, lado, SEGUNDOS);
    } catch (e) {
      erro = String(e);
    } finally {
      clearInterval(t);
      medindo = null;
    }
  }

  async function zerar() {
    await ponte.labZerar().catch(() => {});
    lab = await ponte.labEstado().catch(() => lab);
  }

  async function compartilhar() {
    try {
      const texto = await ponte.compartilharProva();
      await navigator.clipboard.writeText(texto);
      copiado = "Copiado! Cole no Discord.";
    } catch (e) {
      copiado = String(e);
    }
    setTimeout(() => (copiado = null), 3500);
  }

  function num(v: number | null | undefined, casas = 0) {
    return v == null ? "—" : v.toFixed(casas).replace(".", ",");
  }
</script>

<div class="prova">
  <Diagnostico />
  <Partida />
  <EsperandoODisco />
  <MapaDeDesempenho />
  {#if erro}
    <div class="recado" role="alert"><span class="ponto" data-tom="perigo"></span><div><span>{erro}</span></div></div>
  {/if}

  <section class="secao">
    <h2>Laboratório de antes e depois</h2>
    <p class="apoio texto">
      Mede o jogo {SEGUNDOS} s por vez, sempre no mesmo lugar do jogo. Faça pelo menos {lab?.minimo_por_lado ?? 3} rodadas
      antes de mudar algo e {lab?.minimo_por_lado ?? 3} depois: com uma só, não dá para separar a mudança do acaso.
    </p>

    <ol class="passos">
      {#each ["Medir antes", "Mudar algo (Turbo, ajuste, driver)", "Medir depois", "Ver o resultado"] as p, i (p)}
        <li class:feito={passo > i + 1} class:agora={passo === i + 1 || (i === 1 && passo === 2)}>
          <span class="bola numero">{i + 1}</span><span>{p}</span>
        </li>
      {/each}
    </ol>

    <div class="lados">
      {#each [["Antes", lab?.rodadas_antes ?? 0, faltaAntes], ["Depois", lab?.rodadas_depois ?? 0, faltaDepois]] as [lado, feitas, falta] (lado)}
        <div class="lado">
          <span class="apoio">{lado}</span>
          <span class="numero grande">{feitas}</span>
          <span class="apoio">{Number(falta) > 0 ? `faltam ${falta}` : "rodadas suficientes"}</span>
          <button
            class="btn"
            class:primario={(lado === "Antes" && passo === 1) || (lado === "Depois" && passo >= 2)}
            disabled={medindo !== null}
            onclick={() => medir(lado as LadoDoLab)}
          >
            {medindo === lado ? `Medindo… ${passou}/${SEGUNDOS} s` : `Medir ${String(lado).toLowerCase()}`}
          </button>
        </div>
      {/each}
    </div>
    {#if !jogo}<p class="apoio">Abra o jogo para medir.</p>{/if}
  </section>

  {#if lab && (lab.rodadas_antes > 0 || lab.rodadas_depois > 0)}
    <section class="secao">
      <div class="cabeca">
        <h2>Resultado {lab.nome ? `· ${lab.nome}` : ""}</h2>
        <button class="btn quieto" onclick={zerar}>Recomeçar o teste</button>
      </div>
      <table class="tabela">
        <thead><tr><th>Medida</th><th class="n">Antes</th><th class="n">Depois</th><th class="n">Diferença</th><th>Confiança</th></tr></thead>
        <tbody>
          {#each lab.linhas as l (l.metrica)}
            {@const c = CONFIANCA[l.confianca] ?? { texto: l.confianca, tom: "neutro" }}
            <tr>
              <td>{l.metrica}</td>
              <td class="n numero">{num(l.antes, 1)}</td>
              <td class="n numero">{num(l.depois, 1)}</td>
              <td class="n numero" class:bom={(l.diferenca_pct ?? 0) > 0} class:ruim={(l.diferenca_pct ?? 0) < 0}>
                {l.diferenca_pct == null ? "—" : `${l.diferenca_pct > 0 ? "+" : ""}${num(l.diferenca_pct, 1)}%`}
              </td>
              <td><span class="conf"><span class="ponto" data-tom={c.tom}></span>{c.texto}</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if lab.descartadas > 0}
        <p class="apoio">{lab.descartadas} rodadas ficaram de fora porque tinham geração de quadros ligada.</p>
      {/if}
    </section>
  {/if}

  <section class="secao">
    <h2>Prova do Otimizar</h2>
    {#if prova}
      <p class="texto">{prova.leitura}</p>
      <div class="cabeca">
        <span class="apoio">
          {prova.jogo} · {new Date(prova.quando * 1000).toLocaleDateString("pt-BR")} · FPS {num(prova.fps_sem?.media)} → {num(prova.fps_com?.media)}
        </span>
        <span class="selo-medido">Medido</span>
      </div>
    {:else}
      <p class="apoio texto">Ainda não há prova alternada guardada. Ela é feita pelo Turbo (“Provar”), alternando com e sem a mudança no mesmo jogo.</p>
    {/if}
    <div class="compartilhar">
      <button class="btn" onclick={compartilhar}>Copiar resultado para o Discord</button>
      {#if copiado}<span class="apoio">{copiado}</span>{/if}
    </div>
  </section>
  <TestarGrupos />
</div>

<style>
  .prova {
    display: grid;
    gap: 4px;
    padding-top: 8px;
  }
  .texto {
    max-width: 64ch;
  }
  .passos {
    list-style: none;
    margin: 4px 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    font-size: 13px;
    color: var(--tinta-2);
  }
  .passos li {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .bola {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--superficie-2);
    font-size: 12px;
    font-weight: 600;
  }
  .passos li.agora {
    color: var(--tinta);
    font-weight: 500;
  }
  .passos li.agora .bola {
    background: var(--tinta);
    color: var(--contra, #fff);
  }
  .passos li.feito .bola {
    background: var(--ok);
    color: #fff;
  }
  .lados {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .lado {
    display: grid;
    justify-items: start;
    gap: 4px;
    padding: 4px 0 4px 16px;
    border-left: 1px solid var(--linha-forte);
  }
  .grande {
    font-size: 28px;
    font-weight: 600;
    line-height: 1.1;
  }
  .lado .btn {
    margin-top: 6px;
  }
  .cabeca {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .tabela {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
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
    padding: 9px 8px 9px 0;
    border-bottom: 1px solid var(--linha);
  }
  .tabela tr:last-child td {
    border-bottom: 0;
  }
  .n {
    text-align: right;
  }
  .bom {
    color: var(--ok);
    font-weight: 600;
  }
  .ruim {
    color: var(--perigo);
    font-weight: 600;
  }
  .conf {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .compartilhar {
    display: flex;
    align-items: center;
    gap: 12px;
  }
</style>
