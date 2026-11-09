<script lang="ts">
  // O processador no jogo (trazido da interface antiga, `src/main.ts`): núcleos de desempenho e de eficiência (classe
  // dada pelo Windows, não por tabela), prender o jogo aberto nos de desempenho, o teste que mede os dois jeitos
  // alternando e fica com o que rendeu, e o modo jogo. Processador uniforme: NÃO HÁ O QUE FAZER, escrito — um botão ali
  // seria um jeito de piorar.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Nucleos = {
    topologia: { nucleos: unknown[]; hibrido: boolean };
    conselho: { cabe: boolean; explicacao: string };
    fisicos: number;
    mascara_de_desempenho: string | null;
    jogo_nome: string | null;
    jogo_pid: number | null;
    jogo_mascara: string | null;
    jogo_exe: string | null;
  };
  type Resultado = { executavel: string; quando: number; escolha: "SoDesempenho" | "TodosOsNucleos"; fps_todos: number; fps_desempenho: number; low_todos: number; low_desempenho: number };
  type ModoJogo = { game_running: boolean; game: string | null; active: boolean; applied: string[] };

  let nucleos = $state<Nucleos | null>(null);
  let erro = $state<string | null>(null);
  let resultados = $state<Resultado[]>([]);
  let modo = $state<ModoJogo | null>(null);
  let ocupado = $state<string | null>(null);
  let recado = $state<string | null>(null);

  const demo = {
    nucleos: {
      topologia: { nucleos: Array(20), hibrido: true },
      conselho: { cabe: true, explicacao: "Este processador tem 6 núcleos de desempenho e 8 de eficiência. Jogo que cai nos de eficiência perde FPS; prender nos de desempenho costuma ajudar, mas só a medida diz." },
      fisicos: 14, mascara_de_desempenho: "4095", jogo_nome: "FiveM", jogo_pid: 4242, jogo_mascara: "1048575", jogo_exe: "FiveM_GTAProcess.exe",
    } as Nucleos,
    resultados: [{ executavel: "FiveM_GTAProcess.exe", quando: Date.now() / 1000 - 86400, escolha: "SoDesempenho", fps_todos: 88, fps_desempenho: 94, low_todos: 51, low_desempenho: 58 }] as Resultado[],
    modo: { game_running: true, game: "FiveM", active: false, applied: [] } as ModoJogo,
  };

  async function carregar() {
    if (!noTauri) {
      ({ nucleos, resultados, modo } = structuredClone(demo));
      return;
    }
    const [n, r, m] = await Promise.allSettled([
      invoke<Nucleos>("nucleos_da_maquina"),
      invoke<Resultado[]>("cpuset_resultados"),
      invoke<ModoJogo>("game_mode_status"),
    ]);
    nucleos = n.status === "fulfilled" ? n.value : null;
    erro = n.status === "rejected" ? String(n.reason) : null;
    resultados = r.status === "fulfilled" ? r.value : [];
    modo = m.status === "fulfilled" ? m.value : null;
  }

  onMount(carregar);

  async function agir(chave: string, f: () => Promise<unknown>) {
    ocupado = chave;
    recado = null;
    try {
      const r = await f();
      if (typeof r === "string") recado = r;
      await carregar();
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = null;
    }
  }

  const preso = $derived(!!nucleos?.jogo_mascara && nucleos.jogo_mascara === nucleos.mascara_de_desempenho);
  const doJogo = $derived(resultados.find((r) => r.executavel.toLowerCase() === nucleos?.jogo_exe?.toLowerCase()));
  const n = (v: number) => v.toLocaleString("pt-BR", { maximumFractionDigits: 0 });
</script>

<section class="secao">
  <h2>Processador no jogo</h2>
  {#if erro}
    <p class="apoio">{erro}</p>
  {:else if !nucleos}
    <p class="apoio">Lendo os núcleos…</p>
  {:else}
    <p class="apoio">{nucleos.conselho.explicacao}</p>
    {#if nucleos.conselho.cabe}
      {#if nucleos.jogo_pid != null && nucleos.mascara_de_desempenho}
        <div class="linha">
          <div>
            <span>{nucleos.jogo_nome}: {preso ? "preso nos núcleos de desempenho" : "em todos os núcleos"}</span>
            <p class="apoio pequeno">Vale até o jogo fechar; não entra no histórico.</p>
          </div>
          <button class="btn" disabled={ocupado !== null} onclick={() => agir("prender", () => (noTauri ? invoke("prender_jogo_nos_nucleos", { pid: nucleos!.jogo_pid, prender: !preso }) : Promise.resolve("Demonstração: feito.")))}>
            {preso ? "Soltar" : "Prender nos de desempenho"}
          </button>
        </div>
        <div class="linha">
          <div>
            <span>Testar os dois jeitos</span>
            <p class="apoio pequeno">Mede o jogo em todos os núcleos e só nos de desempenho, alternando, e fica com o que rendeu. Continue jogando na mesma cena (uns 2 minutos).</p>
          </div>
          <button class="btn primario" disabled={ocupado !== null} onclick={() => agir("testar", () => (noTauri ? invoke("cpuset_testar", { pid: nucleos!.jogo_pid, executavel: nucleos!.jogo_exe }) : Promise.resolve("Demonstração: medido.")))}>
            {ocupado === "testar" ? "Medindo…" : "Testar"}
          </button>
        </div>
      {:else}
        <p class="apoio">Abra o jogo para prender ou testar.</p>
      {/if}
      {#if doJogo}
        <p class="apoio">
          Último teste no {doJogo.executavel}: todos os núcleos {n(doJogo.fps_todos)} fps (1% pior {n(doJogo.low_todos)}) ×
          só desempenho {n(doJogo.fps_desempenho)} fps (1% pior {n(doJogo.low_desempenho)}) →
          <b>{doJogo.escolha === "SoDesempenho" ? "fica nos de desempenho" : "fica em todos"}</b>.
          <button class="btn quieto" onclick={() => agir("esquecer", () => (noTauri ? invoke("cpuset_esquecer", { executavel: doJogo!.executavel }) : Promise.resolve()))}>Esquecer</button>
        </p>
      {/if}
    {/if}
  {/if}

  {#if modo}
    <div class="linha">
      <div>
        <span>Modo jogo agora</span>
        <p class="apoio pequeno">
          {modo.active ? `Ligado${modo.game ? ` no ${modo.game}` : ""}: ${modo.applied.join(", ") || "nada a mudar"}.` : modo.game_running ? `Desligado, com ${modo.game} aberto.` : "Abra um jogo para ligar."}
          O automático fica nos Ajustes.
        </p>
      </div>
      <button class="interruptor" role="switch" aria-checked={modo.active} aria-label="Modo jogo" disabled={ocupado !== null || (!modo.active && !modo.game_running)}
        onclick={() => agir("modo", () => (noTauri ? invoke("set_game_mode", { active: !modo!.active }) : Promise.resolve("Demonstração: feito.")))}></button>
    </div>
  {/if}
  {#if recado}<p class="recado">{recado}</p>{/if}
</section>

<style>
  .secao {
    display: grid;
    gap: 10px;
  }
  p {
    margin: 0;
  }
  .linha {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-size: 13px;
  }
  .linha p {
    margin-top: 2px;
  }
  .pequeno {
    font-size: 12px;
  }
  .recado {
    font-size: 13px;
  }
</style>
