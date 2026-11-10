<script lang="ts">
  // A configuração gráfica do GTA V / FiveM (`settings.xml`), trazida de `src/main.ts`: o que custa caro no arquivo
  // (com o ganho medido por terceiros, dito assim), e três perfis com a prévia chave por chave antes de aplicar. Aplicar
  // entra no histórico e se desfaz de lá.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Relatorio = { arquivo: string | null; jogo: string; caros: { chave: string; valor: string; onde: string; ganho: string }[]; findings: { id: string; title: string; measured: string; advice: string }[] };
  const PERFIS = [
    { id: "competitivo", nome: "Competitivo", resumo: "O máximo de FPS: corta o que custa caro e quase não se vê." },
    { id: "equilibrado", nome: "Equilibrado", resumo: "Tira o que pesa muito e mantém a cara do jogo." },
    { id: "sem_teto", nome: "Sem teto", resumo: "Só tira os limites (FPS travado, sincronia vertical), sem mexer na imagem." },
  ];

  let rel = $state<Relatorio | null>(null);
  let escolhido = $state<string | null>(null);
  let previa = $state<[string, string, string, string][] | null>(null);
  let ocupado = $state(false);
  let recado = $state<string | null>(null);

  onMount(async () => {
    rel = noTauri
      ? await invoke<Relatorio>("analyze_game_config").catch(() => null)
      : { arquivo: "settings.xml", jogo: "GTA V / FiveM", caros: [{ chave: "MSAA", valor: "4", onde: "Gráficos › MSAA", ganho: "até 40% de FPS a menos (medido por Digital Foundry)" }], findings: [] };
  });

  async function verPrevia(id: string) {
    escolhido = id;
    recado = null;
    try {
      previa = noTauri
        ? await invoke<[string, string, string, string][]>("preview_game_profile", { perfil: id })
        : [["MSAA", "4", "0", "o anti-serrilhado mais caro do jogo"], ["Grass", "3", "1", "grama alta pesa nas áreas abertas"]];
    } catch (e) {
      previa = null;
      recado = String(e);
    }
  }

  async function aplicar() {
    if (!escolhido) return;
    ocupado = true;
    try {
      const r = noTauri ? await invoke<string[]>("apply_game_profile", { perfil: escolhido }) : ["MSAA: 4 → 0"];
      recado = r.length ? `Aplicado: ${r.join("; ")}. Vale ao abrir o jogo de novo; desfaz pelo Histórico.` : "Nada a mudar: o arquivo já estava assim.";
      previa = null;
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = false;
    }
  }
</script>

{#if rel?.arquivo}
  <div class="gta">
    <span class="titulo">Gráficos do GTA V / FiveM</span>
    {#each rel.caros as c (c.chave)}
      <p class="apoio pequeno"><b>{c.chave} = {c.valor}</b> ({c.onde}): {c.ganho}</p>
    {/each}
    {#each rel.findings as f (f.id)}<p class="apoio pequeno"><b>{f.title}</b> {f.measured} {f.advice}</p>{/each}
    <div class="perfis">
      {#each PERFIS as p (p.id)}
        <button aria-pressed={escolhido === p.id} onclick={() => verPrevia(p.id)}>
          <b>{p.nome}</b><span class="apoio pequeno">{p.resumo}</span>
        </button>
      {/each}
    </div>
    {#if previa}
      {#if previa.length === 0}
        <p class="apoio">Nada a mudar neste perfil.</p>
      {:else}
        <ul>
          {#each previa as [chave, antes, depois, porque] (chave)}
            <li><b>{chave}</b> {antes} → {depois} <span class="apoio pequeno">· {porque}</span></li>
          {/each}
        </ul>
        <button class="btn primario" disabled={ocupado} onclick={aplicar}>Aplicar este perfil</button>
      {/if}
    {/if}
    {#if recado}<p class="recado">{recado}</p>{/if}
  </div>
{/if}

<style>
  .gta {
    display: grid;
    gap: 8px;
    padding: 16px 0 0;
    border-top: 1px solid var(--linha);
  }
  .titulo {
    font-weight: 600;
  }
  p {
    margin: 0;
  }
  .perfis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 8px;
  }
  .perfis button {
    display: grid;
    gap: 2px;
    text-align: left;
    padding: 10px 12px;
    border-radius: var(--raio-2);
    border: 1px solid var(--linha);
    background: var(--superficie);
    color: var(--tinta);
    font: inherit;
    cursor: pointer;
  }
  .perfis button:hover {
    border-color: var(--linha-forte);
  }
  .perfis button[aria-pressed="true"] {
    border-color: var(--acao);
    box-shadow: inset 0 0 0 1px var(--acao);
  }
  ul {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 4px;
    font-size: 13px;
  }
  .btn {
    justify-self: start;
  }
  .pequeno {
    font-size: 12px;
  }
  .recado {
    font-size: 13px;
  }
</style>
