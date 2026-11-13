<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  // O resto da placa de vídeo (trazido da interface antiga, `src/main.ts`): em notebook com duas placas, qual delas
  // cada jogo usa (o maior ganho que o produto dá ali, sem administrador nem reinício); o cache de shader do driver
  // (apagar só o velho, nunca o que está em uso); e o Resizable BAR lido do Windows.
  import { onMount } from "svelte";
    import { noTauri } from "../lib/ponte";

  type Pref = "Automatica" | "Economia" | "Desempenho";
  type GpuPref = { placas: string[]; tem_placa_dupla: boolean; definidos: [string, Pref][]; findings: { id: string; title: string; measured: string; advice: string }[]; erro_de_leitura: string | null };
  type Shaders = { caches: { id: string; name: string; formatted: string; files: number; stale: boolean; bytes: number }[]; total_formatted: string; driver_age_days: number | null; note: string };
  type Rbar = { estado: "Ligado" | "DesligadoESuportado" | "DesligadoSemSuporte" | "NaoSei"; modelo: string; nota: string };

  const PREF: Record<Pref, string> = { Automatica: "o Windows escolhe", Economia: "placa de economia (integrada)", Desempenho: "placa de desempenho" };
  const RBAR: Record<Rbar["estado"], string> = { Ligado: "ligado", DesligadoESuportado: "desligado, mas a placa suporta", DesligadoSemSuporte: "desligado (sem suporte)", NaoSei: "não deu para ler" };

  let gpu = $state<GpuPref | null>(null);
  let shaders = $state<Shaders | null>(null);
  let rbar = $state<Rbar | null>(null);
  let ocupado = $state<string | null>(null);
  let recado = $state<string | null>(null);

  async function carregar() {
    if (!noTauri) {
      gpu = { placas: ["Intel UHD Graphics", "NVIDIA GeForce RTX 3050 Laptop"], tem_placa_dupla: true, definidos: [["C:\\Games\\FiveM\\FiveM.exe", "Automatica"]], findings: [], erro_de_leitura: null };
      shaders = { caches: [{ id: "nv-dx", name: "Cache DirectX da NVIDIA", formatted: "1,8 GB", files: 912, stale: true, bytes: 1.8e9 }], total_formatted: "1,8 GB", driver_age_days: 140, note: "O driver é de 140 dias atrás: o cache guarda shaders de versões velhas do driver." };
      rbar = { estado: "DesligadoESuportado", modelo: "RTX 3050 Laptop", nota: "A placa suporta Resizable BAR, mas ele está desligado na BIOS (veja a aba BIOS)." };
      return;
    }
    const [g, s, r] = await Promise.allSettled([como<GpuPref>(commands.analyzeGpuPreference()), como<Shaders>(commands.analyzeShaders()), como<Rbar>(commands.analyzeRbar())]);
    gpu = g.status === "fulfilled" ? g.value : null;
    shaders = s.status === "fulfilled" ? s.value : null;
    rbar = r.status === "fulfilled" ? r.value : null;
  }

  onMount(carregar);

  async function agir(chave: string, f: () => Promise<{ message?: string } | unknown>) {
    ocupado = chave;
    recado = null;
    try {
      const r = (await f()) as { message?: string } | undefined;
      recado = r?.message ?? null;
      await carregar();
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = null;
    }
  }

  const exe = (c: string) => c.split(/[\\/]/).pop() ?? c;
</script>

<section class="secao placa">
  <h2>Placa de vídeo</h2>

  {#if gpu?.tem_placa_dupla}
    <div class="bloco">
      <span class="sub">Qual placa cada jogo usa</span>
      <p class="apoio pequeno">Este PC tem duas placas ({gpu.placas.join(" e ")}). Jogo na integrada perde muito FPS. A mudança vale na próxima vez que o jogo abrir, sem administrador nem reinício, e se desfaz no Histórico.</p>
      {#if gpu.erro_de_leitura}<p class="aviso">{gpu.erro_de_leitura}</p>{/if}
      {#each gpu.definidos as [caminho, pref] (caminho)}
        <div class="linha">
          <div><span>{exe(caminho)}</span><p class="apoio pequeno">{PREF[pref]}</p></div>
          {#if pref !== "Desempenho"}
            <button class="btn" disabled={ocupado !== null} onclick={() => agir(caminho, () => (noTauri ? commands.setGpuPreference(caminho, true) : Promise.resolve({ message: "Demonstração: feito." })))}>Passar para a placa de desempenho</button>
          {/if}
        </div>
      {/each}
      {#each gpu.findings as f (f.id)}<p class="apoio pequeno"><b>{f.title}</b> {f.measured} {f.advice}</p>{/each}
    </div>
  {/if}

  {#if shaders}
    <div class="bloco">
      <span class="sub">Cache de shader do driver · {shaders.total_formatted}</span>
      <p class="apoio pequeno">{shaders.note}</p>
      {#each shaders.caches as c (c.id)}
        <div class="linha">
          <div><span>{c.name} · {c.formatted}</span><p class="apoio pequeno">{c.files} arquivos{c.stale ? " · velho" : ""}</p></div>
          {#if c.stale && c.bytes > 0}
            <button class="btn" disabled={ocupado !== null} onclick={() => confirm(`Apagar ${c.name}? O primeiro minuto dos jogos pode dar trancos enquanto o driver refaz.`) && agir(c.id, () => (noTauri ? commands.cleanShaderCache(c.id) : Promise.resolve({ message: "Demonstração: apagado." })))}>Limpar</button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if rbar}
    <div class="bloco">
      <span class="sub">Resizable BAR · {RBAR[rbar.estado]}</span>
      <p class="apoio pequeno">{rbar.nota}</p>
    </div>
  {/if}
  {#if recado}<p class="recado">{recado}</p>{/if}
</section>

<style>
  .placa,
  .bloco {
    display: grid;
    gap: 8px;
  }
  .bloco + .bloco {
    padding-top: 10px;
    border-top: 1px solid var(--linha);
  }
  .sub {
    font-weight: 500;
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
