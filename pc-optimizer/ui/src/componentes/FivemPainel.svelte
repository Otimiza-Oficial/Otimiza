<script lang="ts">
  import { como } from "../lib/contrato";
  import { confirmar } from "../lib/confirmar";
  import { commands } from "../lib/tipos";
  // O FiveM no destaque do jogo (trazido da interface antiga, `src/main.ts`): as pastas de cache com o que cada uma
  // custa apagar (nunca o perfil, nunca com o FiveM aberto), os crashes com as pistas — mesmo defeito, pool cheio do
  // servidor, memória no limite, já acontecia antes do Otimiza — e a nota do CitizenFX.ini. Em duas partes: `cache`
  // entra no cartão de cache do catálogo (aba Otimizar), `crashes` vira o cartão da aba Ferramentas.
  import { onMount } from "svelte";
  import { noTauri } from "../lib/ponte";
  import Cartao from "./base/Cartao.svelte";
  import Esqueleto from "./base/Esqueleto.svelte";

  let { parte }: { parte: "cache" | "crashes" } = $props();

  type Pasta = { id: string; name: string; path: string; bytes: number; formatted: string; cleanable: boolean; explanation: string; tradeoff: string | null };
  type Relatorio = { installed: boolean; running: boolean; game_running: boolean; folders: Pasta[]; cleanable_bytes: number; protected_bytes: number; note: string };
  type Crash = { quando: number; erro: string | null; hash: string | null; memoria_pct: number | null; veiculo: string | null; minutos_de_jogo: number | null; mudancas_antes: string[] };
  type Crashes = { crashes: Crash[]; leituras: { pista: string; frase: string }[]; antes_do_otimiza: number; depois_do_otimiza: number; sem_fivem: boolean };
  type CitizenFx = { existe: boolean; note: string };

  let rel = $state<Relatorio | null>(null);
  let crashes = $state<Crashes | null>(null);
  let cfx = $state<CitizenFx | null>(null);
  let lendo = $state(true);
  let falhou = $state(false);
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
      lendo = false;
      return;
    }
    // Cada parte lê só o que mostra.
    if (parte === "cache") {
      const r = await como<Relatorio>(commands.analyzeFivem()).catch(() => null);
      rel = r;
      falhou = r === null;
    } else {
      const [c, x] = await Promise.allSettled([como<Crashes>(commands.crashesDoJogo()), como<CitizenFx>(commands.analyzeCitizenfx())]);
      crashes = c.status === "fulfilled" ? c.value : null;
      cfx = x.status === "fulfilled" ? x.value : null;
      falhou = crashes === null;
    }
    lendo = false;
  }

  onMount(carregar);

  async function limpar(p: Pasta) {
    if (!(await confirmar({ titulo: `Apagar ${p.name}?`, texto: `Libera ${p.formatted}. ${p.tradeoff ?? ""}`.trim(), sim: "Apagar", perigo: true }))) return;
    limpando = p.id;
    recado = null;
    try {
      const r = noTauri ? await como<{ freed_mb: number; message: string }>(commands.cleanFivem(p.id)) : { message: `Demonstração: ${p.formatted} liberados.` };
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

{#if parte === "cache" && lendo}
  <div class="pastas"><Esqueleto linhas={2} rotulo="Medindo as pastas do FiveM" /></div>
{:else if parte === "cache" && falhou}
  <div class="pastas"><p class="aviso">Não consegui medir as pastas do FiveM agora.</p></div>
{:else if parte === "cache" && rel && !rel.installed}
  <div class="pastas"><p class="apoio">Não achei a pasta do FiveM neste PC. Abra o FiveM uma vez e as pastas de cache aparecem aqui.</p></div>
{:else if parte === "cache" && rel?.installed}
  <div class="pastas">
    <span class="rotulo-grupo">Pastas do FiveM</span>
    <p class="apoio">{rel.note}</p>
    {#if rel.running || rel.game_running}<p class="aviso">Feche o FiveM para limpar: com ele aberto os arquivos estão em uso.</p>{/if}
    <ul class="linhas">
      {#each rel.folders as p (p.id)}
        <li>
          <div>
            <span>{p.name} · <b>{p.formatted}</b></span>
            <span class="apoio pequeno">{p.explanation}{p.tradeoff ? ` ${p.tradeoff}` : ""}</span>
          </div>
          {#if p.cleanable && p.bytes > 0}
            <button class="btn" disabled={limpando !== null || rel.running || rel.game_running} onclick={() => limpar(p)}>{limpando === p.id ? "Limpando…" : "Limpar"}</button>
          {:else if !p.cleanable}
            <span class="selo">protegido</span>
          {/if}
        </li>
      {/each}
    </ul>
    {#if recado}<p class="recado" role="status">{recado}</p>{/if}
  </div>
{:else if parte === "crashes" && lendo}
  <Cartao titulo="Crashes do FiveM"><Esqueleto linhas={3} rotulo="Lendo os crashes do FiveM" /></Cartao>
{:else if parte === "crashes" && (falhou || !crashes)}
  <Cartao titulo="Crashes do FiveM"><p class="aviso">Não consegui ler os relatórios de crash do FiveM agora.</p></Cartao>
{:else if parte === "crashes" && crashes?.sem_fivem}
  <Cartao titulo="Crashes do FiveM"><p class="apoio">Não achei o FiveM neste PC. Os crashes aparecem aqui depois que ele for instalado e aberto.</p></Cartao>
{:else if parte === "crashes" && crashes}
  {@const total = crashes.antes_do_otimiza + crashes.depois_do_otimiza}
  <Cartao titulo="Crashes do FiveM" apoio={total ? `${total} ${total === 1 ? "crash" : "crashes"}: ${crashes.antes_do_otimiza} antes do Otimiza, ${crashes.depois_do_otimiza} depois.${crashes.crashes.length > 5 ? " Aqui, os 5 mais recentes." : ""}` : null}>
    {#if crashes.crashes.length === 0}
      <p class="apoio">Nenhum relatório de crash do FiveM neste PC.</p>
    {:else}
      {#each crashes.leituras as l (l.pista)}<p class="aviso">{l.frase}</p>{/each}
      <ul class="linhas">
        {#each crashes.crashes.slice(0, 5) as c (c.quando)}
          <li>
            <div>
              <span>{data(c.quando)}{c.minutos_de_jogo != null ? ` · ${c.minutos_de_jogo} min de jogo` : ""}</span>
              <span class="apoio pequeno">{c.erro ?? "erro não registrado"}{c.hash ? ` · ${c.hash}` : ""}{c.memoria_pct != null ? ` · memória a ${c.memoria_pct}%` : ""}{c.veiculo ? ` · veículo ${c.veiculo}` : ""}</span>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
    {#if cfx?.existe}<p class="apoio pequeno">{cfx.note}</p>{/if}
  </Cartao>
{/if}

<style>
  .pastas {
    display: grid;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--linha);
  }
</style>
