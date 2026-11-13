<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  import Esqueleto from "./Esqueleto.svelte";
  // Sistema avançado (trazido da interface antiga, `src/main.ts`): os serviços essenciais que "Windows lite" e outros
  // otimizadores desligam (e o religar), os serviços de outros programas, as tarefas agendadas que não são da
  // Microsoft e os programas que vieram de fábrica. Cada mudança entra no histórico. Nada é marcado sem motivo escrito,
  // e o que é proteção, áudio ou vídeo nunca aparece como mexível.
  import { onMount } from "svelte";
    import { noTauri } from "../lib/ponte";

  type Inicio = "Desativado" | "Ativo" | "NaoExiste" | "NaoConsegui";
  type Checagem = { servicos: { servico: string; inicio: Inicio }[]; desativados: number; fabricante: string | null; modelo: string | null };
  type Servico = { name: string; display_name: string; path: string; start_mode: "Automatic" | "Manual" | "Disabled" | "Kernel"; running: boolean; ram_mb: number | null; protected: string | null };
  type Tarefa = { name: string; path: string; author: string; enabled: boolean; microsoft: boolean };
  type Bloat = { name: string; publisher: string; kind: "OemUtility" | "TrialSecurity" | "Sponsored" | "StoreApp"; size_mb: number | null; reason: string; package: string | null; removable_here: boolean };
  type Relatorio = { items: Bloat[]; total_mb: number; unmeasured: number; programs_scanned: number; lacunas: string[] };

  const TIPO: Record<Bloat["kind"], string> = { OemUtility: "do fabricante", TrialSecurity: "antivírus de teste", Sponsored: "patrocinado", StoreApp: "da Loja" };
  const INICIO: Record<Servico["start_mode"], string> = { Automatic: "automático", Manual: "manual", Disabled: "desativado", Kernel: "do sistema" };

  let checagem = $state<Checagem | null>(null);
  let servicos = $state<Servico[] | null>(null);
  let tarefas = $state<Tarefa[] | null>(null);
  let bloat = $state<Relatorio | null>(null);
  let ocupado = $state<string | null>(null);
  let recado = $state<string | null>(null);
  let verMicrosoft = $state(false);

  const demo = {
    checagem: { servicos: [{ servico: "PlugPlay", inicio: "Ativo" }, { servico: "AppXSvc", inicio: "Desativado" }, { servico: "TokenBroker", inicio: "Ativo" }], desativados: 1, fabricante: null, modelo: null } as Checagem,
    servicos: [
      { name: "EpicOnlineServices", display_name: "Epic Online Services", path: "C:\\Program Files (x86)\\Epic Games\\EpicOnlineServices.exe", start_mode: "Automatic", running: true, ram_mb: 24, protected: null },
      { name: "AdobeARMservice", display_name: "Adobe Acrobat Update Service", path: "C:\\Program Files (x86)\\Common Files\\Adobe\\ARM\\armsvc.exe", start_mode: "Automatic", running: true, ram_mb: 3.1, protected: null },
      { name: "NVDisplay.ContainerLocalSystem", display_name: "NVIDIA Display Container LS", path: "C:\\Windows\\System32\\DriverStore\\nvcontainer.exe", start_mode: "Automatic", running: true, ram_mb: 31, protected: "driver de vídeo" },
    ] as Servico[],
    tarefas: [
      { name: "GoogleUpdateTaskMachineCore", path: "\\", author: "Google", enabled: true, microsoft: false },
      { name: "CCleanerSkipUAC", path: "\\", author: "Piriform", enabled: true, microsoft: false },
    ] as Tarefa[],
    bloat: {
      items: [
        { name: "McAfee LiveSafe", publisher: "McAfee", kind: "TrialSecurity", size_mb: 412, reason: "Antivírus de teste que veio no PC; com ele e o Defender juntos, os dois varrem o mesmo arquivo.", package: null, removable_here: false },
        { name: "Candy Crush Saga", publisher: "King", kind: "StoreApp", size_mb: null, reason: "Jogo patrocinado instalado pelo Windows.", package: "king.com.CandyCrushSaga", removable_here: true },
      ],
      total_mb: 412, unmeasured: 1, programs_scanned: 84, lacunas: [],
    } as Relatorio,
  };

  async function carregar() {
    if (!noTauri) {
      ({ checagem, servicos, tarefas, bloat } = structuredClone(demo));
      return;
    }
    const [c, s, t, b] = await Promise.allSettled([
      como<Checagem>(commands.checarEssenciais()),
      como<Servico[]>(commands.listThirdPartyServices()),
      como<Tarefa[]>(commands.listScheduledTasks()),
      como<Relatorio>(commands.analyzeBloatware()),
    ]);
    checagem = c.status === "fulfilled" ? c.value : null;
    servicos = s.status === "fulfilled" ? s.value : [];
    tarefas = t.status === "fulfilled" ? t.value : [];
    bloat = b.status === "fulfilled" ? b.value : null;
  }

  onMount(carregar);

  async function agir(chave: string, f: () => Promise<unknown>) {
    ocupado = chave;
    recado = null;
    try {
      const r = (await f()) as { message?: string } | string | undefined;
      recado = typeof r === "string" ? r : (r?.message ?? null);
      await carregar();
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = null;
    }
  }

  const demoOk = (m: string) => Promise.resolve({ message: `Demonstração: ${m}` });
  const visiveis = $derived((tarefas ?? []).filter((t) => verMicrosoft || !t.microsoft));
  const n = (v: number, c = 0) => v.toLocaleString("pt-BR", { maximumFractionDigits: c, minimumFractionDigits: c });
</script>

{#if recado}<p class="recado" role="status">{recado}</p>{/if}

<section class="secao">
  <h2>Serviços essenciais do Windows</h2>
  {#if !checagem}
    <Esqueleto linhas={2} />
  {:else}
    <p class="apoio">
      {checagem.desativados
        ? `${checagem.desativados} ${checagem.desativados === 1 ? "serviço que o Windows precisa está desligado" : "serviços que o Windows precisa estão desligados"} — em geral por um "Windows lite" ou outro otimizador. Sem eles, a Loja, o login de conta e o reconhecimento de dispositivo falham.`
        : "Todos os serviços que o Windows precisa estão como deveriam."}
    </p>
    <ul class="lista">
      {#each checagem.servicos.filter((s) => s.inicio !== "NaoExiste") as s (s.servico)}
        <li><span>{s.servico}</span><span class="estado" data-tom={s.inicio === "Desativado" ? "atencao" : s.inicio === "Ativo" ? "ok" : "neutro"}>{s.inicio === "Desativado" ? "desligado" : s.inicio === "Ativo" ? "ligado" : "não deu para ler"}</span></li>
      {/each}
    </ul>
    {#if checagem.desativados}
      <button class="btn primario" disabled={ocupado !== null} onclick={() => agir("essenciais", () => (noTauri ? commands.religarEssenciais() : demoOk("religados.")))}>Religar os essenciais</button>
    {/if}
  {/if}
</section>

<section class="secao">
  <h2>Serviços de outros programas</h2>
  <p class="apoio">O que outros programas deixam rodando sempre. Passar para manual: ele só abre quando o programa precisar. Proteção, áudio e vídeo não são mexíveis.</p>
  {#if servicos === null}
    <Esqueleto linhas={2} />
  {:else if servicos.length === 0}
    <p class="apoio">Nenhum serviço de outro programa.</p>
  {:else}
    <ul class="lista">
      {#each servicos as s (s.name)}
        <li>
          <div>
            <span>{s.display_name}</span>
            <p class="apoio pequeno">{INICIO[s.start_mode]}{s.running ? " · rodando" : ""}{s.ram_mb != null ? ` · ${n(s.ram_mb, 1)} MB` : ""}{s.protected ? ` · ${s.protected}: não mexer` : ""}</p>
          </div>
          {#if !s.protected && (s.start_mode === "Automatic" || s.start_mode === "Manual")}
            <button class="btn" disabled={ocupado !== null} onclick={() => agir(s.name, () => (noTauri ? commands.setServiceStart(s.name, s.start_mode !== "Automatic") : demoOk(`${s.display_name} mudado.`)))}>
              {s.start_mode === "Automatic" ? "Passar para manual" : "Voltar para automático"}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="secao">
  <div class="topo">
    <h2>Tarefas agendadas</h2>
    <label class="apoio pequeno"><input type="checkbox" bind:checked={verMicrosoft} /> mostrar as da Microsoft</label>
  </div>
  <p class="apoio">Atualizadores e "ajudantes" que abrem sozinhos no meio do jogo. Desligar não apaga: dá para religar aqui.</p>
  {#if tarefas === null}
    <Esqueleto linhas={2} />
  {:else if visiveis.length === 0}
    <p class="apoio">Nenhuma tarefa de outro programa.</p>
  {:else}
    <ul class="lista">
      {#each visiveis as t (t.path + t.name)}
        <li>
          <div><span>{t.name}</span><p class="apoio pequeno">{t.author || "autor não informado"}{t.microsoft ? " · Microsoft" : ""}</p></div>
          <button class="interruptor" role="switch" aria-checked={t.enabled} aria-label={t.name} disabled={ocupado !== null} onclick={() => agir(t.name, () => (noTauri ? commands.setScheduledTask(t.path, t.name, !t.enabled) : demoOk(`${t.name} mudada.`)))}></button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="secao">
  <h2>Programas que vieram de fábrica</h2>
  {#if !bloat}
    <p class="apoio">Examinando os programas…</p>
  {:else}
    <p class="apoio">
      {bloat.items.length ? `${bloat.items.length} de ${bloat.programs_scanned} programas, ${n(bloat.total_mb / 1024, 1)} GB medidos${bloat.unmeasured ? ` (+${bloat.unmeasured} sem tamanho)` : ""}.` : `Nada para tirar entre os ${bloat.programs_scanned} programas.`}
      Driver, áudio e biblioteca de sistema nunca aparecem aqui.
    </p>
    <ul class="lista">
      {#each bloat.items as b (b.name)}
        <li>
          <div>
            <span>{b.name} <span class="apoio pequeno">· {TIPO[b.kind]}{b.size_mb != null ? ` · ${n(b.size_mb)} MB` : ""}</span></span>
            <p class="apoio pequeno">{b.reason}</p>
          </div>
          {#if b.removable_here && b.package}
            <button class="btn" disabled={ocupado !== null} onclick={() => confirm(`Remover ${b.name}?`) && agir(b.name, () => (noTauri ? commands.removeStoreApp(b.package ?? "") : demoOk(`${b.name} removido.`)))}>Remover</button>
          {:else}
            <button class="btn quieto" onclick={() => (noTauri ? como<string>(commands.openAppsSettings()).catch(() => {}) : undefined)}>Abrir o desinstalador</button>
          {/if}
        </li>
      {/each}
    </ul>
    {#if bloat.lacunas.length}<p class="apoio pequeno">{bloat.lacunas.join(" ")}</p>{/if}
  {/if}
</section>

<style>
  .lista {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .lista li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-size: 13px;
  }
  .lista p {
    margin: 2px 0 0;
  }
  .topo {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }
  .estado[data-tom="ok"] {
    color: var(--ok);
  }
  .estado[data-tom="atencao"] {
    color: var(--atencao);
  }
  .pequeno {
    font-size: 12px;
  }
  .secao > .btn {
    justify-self: start;
    width: fit-content;
  }
  .recado {
    font-size: 13px;
    margin: 0 0 8px;
  }
</style>
