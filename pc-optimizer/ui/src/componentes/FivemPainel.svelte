<script lang="ts">
  // O FiveM no destaque do jogo (trazido da interface antiga, `src/main.ts`): as pastas de cache com o que cada uma
  // custa apagar (nunca o perfil, nunca com o FiveM aberto), os crashes com as pistas — mesmo defeito, pool cheio do
  // servidor, memória no limite, já acontecia antes do Otimiza — e a nota do CitizenFX.ini.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";
  import GraficosGta from "./GraficosGta.svelte";

  type Pasta = { id: string; name: string; path: string; bytes: number; formatted: string; cleanable: boolean; explanation: string; tradeoff: string | null };
  type Relatorio = { installed: boolean; running: boolean; game_running: boolean; folders: Pasta[]; cleanable_bytes: number; protected_bytes: number; note: string };
  type Crash = { quando: number; erro: string | null; hash: string | null; memoria_pct: number | null; veiculo: string | null; minutos_de_jogo: number | null; mudancas_antes: string[] };
  type Crashes = { crashes: Crash[]; leituras: { pista: string; frase: string }[]; antes_do_otimiza: number; depois_do_otimiza: number; sem_fivem: boolean };
  type CitizenFx = { existe: boolean; note: string };

  let rel = $state<Relatorio | null>(null);
  let crashes = $state<Crashes | null>(null);
  let cfx = $state<CitizenFx | null>(null);
  let limpando = $state<string | null>(null);
  let recado = $state<string | null>(null);

  const demo = {
    rel: {
      installed: true, running: false, game_running: false, cleanable_bytes: 3.2e9, protected_bytes: 1.1e9,
      note: "Limpar o cache resolve textura quebrada e carregamento travado; o FiveM baixa de novo o que precisar.",
      folders: [
        { id: "data/server-cache", name: "Cache dos servidores", path: "", bytes: 2.6e9, formatted: "2,6 GB", cleanable: true, explanation: "Arquivos baixados de cada servidor.", tradeoff: "A próxima entrada em cada servidor demora mais: ele baixa tudo de novo." },
        { id: "data/cache", name: "Cache do jogo", path: "", bytes: 0.6e9, formatted: "600 MB", cleanable: true, explanation: "Shaders e dados temporários.", tradeoff: "O primeiro minuto de jogo pode dar trancos enquanto refaz." },
        { id: "citizen", name: "Perfil e configurações", path: "", bytes: 1.1e9, formatted: "1,1 GB", cleanable: false, explanation: "Suas teclas, configurações e chave.", tradeoff: null },
      ],
    } as Relatorio,
    crashes: {
      crashes: [{ quando: Date.now() / 1000 - 7200, erro: "fragInstGta Pool Full, Size == 1600", hash: "bacon-vegan-carbon", memoria_pct: 91, veiculo: null, minutos_de_jogo: 42, mudancas_antes: [] }],
      leituras: [{ pista: "PoolCheio", frase: "Um limite do motor do jogo estourou (pool cheio): depende do servidor, não do Windows. Avise a equipe do servidor." }],
      antes_do_otimiza: 3, depois_do_otimiza: 1, sem_fivem: false,
    } as Crashes,
    cfx: { existe: true, note: "CitizenFX.ini sem aumento de pools: o padrão do FiveM." } as CitizenFx,
  };

  async function carregar() {
    if (!noTauri) {
      ({ rel, crashes, cfx } = structuredClone(demo));
      return;
    }
    const [r, c, x] = await Promise.allSettled([
      invoke<Relatorio>("analyze_fivem"),
      invoke<Crashes>("crashes_do_jogo"),
      invoke<CitizenFx>("analyze_citizenfx"),
    ]);
    rel = r.status === "fulfilled" ? r.value : null;
    crashes = c.status === "fulfilled" ? c.value : null;
    cfx = x.status === "fulfilled" ? x.value : null;
  }

  onMount(carregar);

  async function limpar(p: Pasta) {
    if (!confirm(`Apagar ${p.name} (${p.formatted})? ${p.tradeoff ?? ""}`)) return;
    limpando = p.id;
    recado = null;
    try {
      const r = noTauri ? await invoke<{ freed_mb: number; message: string }>("clean_fivem", { id: p.id }) : { message: `Demonstração: ${p.formatted} liberados.` };
      recado = r.message;
      await carregar();
    } catch (e) {
      recado = String(e);
    } finally {
      limpando = null;
    }
  }

  const data = (s: number) => new Date(s * 1000).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });
</script>

{#if rel?.installed}
  <div class="fivem">
    <span class="titulo">FiveM</span>

    <div class="bloco">
      <span class="sub">Cache</span>
      <p class="apoio">{rel.note}</p>
      {#if rel.running || rel.game_running}<p class="aviso">Feche o FiveM para limpar: com ele aberto os arquivos estão em uso.</p>{/if}
      <ul>
        {#each rel.folders as p (p.id)}
          <li>
            <div>
              <span>{p.name} · {p.formatted}</span>
              <p class="apoio pequeno">{p.explanation}{p.tradeoff ? ` ${p.tradeoff}` : ""}</p>
            </div>
            {#if p.cleanable && p.bytes > 0}
              <button class="btn" disabled={limpando !== null || rel.running || rel.game_running} onclick={() => limpar(p)}>{limpando === p.id ? "Limpando…" : "Limpar"}</button>
            {:else if !p.cleanable}
              <span class="apoio pequeno">protegido</span>
            {/if}
          </li>
        {/each}
      </ul>
    </div>

    {#if crashes && !crashes.sem_fivem}
      <div class="bloco">
        <span class="sub">Crashes</span>
        {#if crashes.crashes.length === 0}
          <p class="apoio">Nenhum relatório de crash do FiveM neste PC.</p>
        {:else}
          <p class="apoio">{crashes.crashes.length} relatório(s): {crashes.antes_do_otimiza} antes do Otimiza e {crashes.depois_do_otimiza} depois.</p>
          {#each crashes.leituras as l (l.pista)}<p class="aviso">{l.frase}</p>{/each}
          <ul>
            {#each crashes.crashes.slice(0, 5) as c (c.quando)}
              <li>
                <div>
                  <span>{data(c.quando)}{c.minutos_de_jogo != null ? ` · ${c.minutos_de_jogo} min de jogo` : ""}</span>
                  <p class="apoio pequeno">{c.erro ?? "erro não registrado"}{c.hash ? ` · ${c.hash}` : ""}{c.memoria_pct != null ? ` · memória a ${c.memoria_pct}%` : ""}{c.veiculo ? ` · veículo ${c.veiculo}` : ""}</p>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}

    <GraficosGta />
    {#if cfx?.existe}<p class="apoio pequeno">{cfx.note}</p>{/if}
    {#if recado}<p class="recado">{recado}</p>{/if}
  </div>
{/if}

<style>
  .fivem {
    display: grid;
    gap: 10px;
    padding: 14px 16px;
    border-radius: var(--raio-2);
    background: var(--superficie-2);
  }
  .titulo {
    font-weight: 600;
  }
  .bloco {
    display: grid;
    gap: 6px;
  }
  .sub {
    font-size: var(--t-apoio);
    color: var(--tinta-2);
  }
  p {
    margin: 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }
  .pequeno {
    font-size: 12px;
  }
  .aviso {
    color: var(--atencao);
    font-size: 13px;
  }
  .recado {
    font-size: 13px;
  }
</style>
