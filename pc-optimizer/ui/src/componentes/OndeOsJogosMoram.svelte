<script lang="ts">
  // Onde cada jogo está instalado (trazido de `src/main.ts`): jogo pesado num HD mecânico demora a carregar e dá
  // tranco quando o cenário muda. Só lê; mover o jogo é pelo próprio iniciador (Steam › Propriedades › Arquivos).
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Onde = { jogo: string; caminho: string; unidade: string | null; midia: "Ssd" | "Mecanico" | "NaoDeuParaLer"; modelo: string | null };

  let lista = $state<Onde[]>([]);

  onMount(async () => {
    const r = noTauri
      ? await invoke<{ jogos: Onde[] }>("onde_os_jogos_moram").catch(() => null)
      : { jogos: [{ jogo: "Counter-Strike 2", caminho: "D:\SteamLibrary", unidade: "D", midia: "Mecanico", modelo: "ST1000DM010" } as Onde] };
    lista = (r?.jogos ?? []).filter((j) => j.midia === "Mecanico");
  });
</script>

{#if lista.length}
  <div class="recado-hd">
    <b>{lista.length === 1 ? "1 jogo está" : `${lista.length} jogos estão`} num HD mecânico</b>
    {#each lista as j (j.jogo)}<p>{j.jogo} · disco {j.unidade ?? "?"}:{j.modelo ? ` (${j.modelo})` : ""}</p>{/each}
    <p class="apoio">HD demora a carregar e dá tranco quando o cenário muda. Se tiver SSD, mova pelo iniciador (na Steam: Propriedades › Arquivos instalados › Mover pasta).</p>
  </div>
{/if}

<style>
  .recado-hd {
    display: grid;
    gap: 4px;
    padding: 12px 14px;
    border-radius: var(--raio-2);
    background: color-mix(in srgb, var(--atencao) 12%, transparent);
    font-size: 13px;
  }
  p {
    margin: 0;
  }
</style>
