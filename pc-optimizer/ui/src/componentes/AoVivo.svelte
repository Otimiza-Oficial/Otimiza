<script lang="ts">
  import Esqueleto from "./Esqueleto.svelte";
  // Sistema › Agora (trazido da interface 3.6, `src/main.ts`: `renderProcesses`, `analyzeBottleneck` e `carregarPlaca`):
  // quem está pesando agora, o que segura a máquina medido por 10 s, e a placa com o driver. A lista se atualiza a cada
  // 3 s só com a aba à vista, e para com jogo aberto (a interface congela: `jogo-aberto` no <html>). Tipos escritos à
  // mão, os mesmos da 3.6.
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Processo = { name: string; cpu_percent: number; ram_mb: number; instances: number; in_startup: boolean };
  type Limite = "CpuUmNucleo" | "CpuTodos" | "Gpu" | "MemoriaVideo" | "MemoriaRam" | "Disco" | "NaoIdentificado" | "SemCarga";
  type Gargalo = {
    limite: Limite;
    summary: string;
    advice: string;
    cpu_total: number;
    cpu_max_core: number;
    gpu_percent: number;
    vram_used_mb: number;
    vram_total_mb: number | null;
    ram_available_gb: number;
    ram_total_gb: number;
    disk_percent: number;
    samples: number;
    seconds: number;
  };
  type Placa = {
    marca: string;
    nome: string | null;
    driver: string | null;
    driver_data: string | null;
    driver_origem?: string | null;
    driver_generico?: boolean;
    driver_dias: number | null;
    vram_gb: number;
  };

  const ROTULO: Record<Limite, string> = {
    CpuUmNucleo: "processador, um núcleo",
    CpuTodos: "processador",
    Gpu: "placa de vídeo",
    MemoriaVideo: "memória de vídeo",
    MemoriaRam: "memória",
    Disco: "disco",
    NaoIdentificado: "não identificado",
    SemCarga: "sem carga",
  };
  /** Placa no talo é boa notícia em jogo; sem carga não é diagnóstico. */
  const TOM: Record<Limite, "atencao" | "perigo" | null> = {
    CpuUmNucleo: "atencao",
    CpuTodos: "atencao",
    Gpu: null,
    MemoriaVideo: "perigo",
    MemoriaRam: "perigo",
    Disco: "perigo",
    NaoIdentificado: null,
    SemCarga: null,
  };
  const SITE: Record<string, string> = {
    nvidia: "https://www.nvidia.com/pt-br/drivers/",
    amd: "https://www.amd.com/pt/support/download/drivers.html",
    intel: "https://www.intel.com.br/content/www/br/pt/download-center/home.html",
  };

  let processos = $state<Processo[] | null>(null);
  let erroDosProcessos = $state<string | null>(null);
  let gargalo = $state<Gargalo | null>(null);
  let medindo = $state(false);
  let erroDoGargalo = $state<string | null>(null);
  let placa = $state<Placa | null | undefined>(undefined);

  const maisPesado = $derived(Math.max(1, ...(processos ?? []).map((p) => p.cpu_percent)));

  let relogio: ReturnType<typeof setInterval> | undefined;
  let lendo = false;

  async function lerProcessos() {
    if (lendo || document.hidden || document.documentElement.classList.contains("jogo-aberto")) return;
    lendo = true;
    try {
      processos = noTauri ? await invoke<Processo[]>("top_processes") : DEMO_PROCESSOS;
      erroDosProcessos = null;
    } catch (e) {
      erroDosProcessos = String(e);
    } finally {
      lendo = false;
    }
  }

  onMount(() => {
    void lerProcessos();
    relogio = setInterval(lerProcessos, 3000);
    placa = undefined;
    (noTauri ? invoke<Placa>("placa_de_video") : Promise.resolve(DEMO_PLACA)).then((p) => (placa = p)).catch(() => (placa = null));
  });
  onDestroy(() => clearInterval(relogio));

  async function medirGargalo() {
    medindo = true;
    erroDoGargalo = null;
    try {
      gargalo = noTauri ? await invoke<Gargalo>("analyze_bottleneck", { seconds: 10 }) : await demoGargalo();
    } catch (e) {
      erroDoGargalo = String(e);
    } finally {
      medindo = false;
    }
  }

  const um = (v: number) => v.toLocaleString("pt-BR", { maximumFractionDigits: 1, minimumFractionDigits: 1 });
  const int = (v: number) => v.toLocaleString("pt-BR", { maximumFractionDigits: 0 });

  const DEMO_PROCESSOS: Processo[] = [
    { name: "chrome.exe", cpu_percent: 11.4, ram_mb: 2310, instances: 21, in_startup: false },
    { name: "Discord.exe", cpu_percent: 3.2, ram_mb: 612, instances: 5, in_startup: true },
    { name: "OneDrive.exe", cpu_percent: 1.8, ram_mb: 140, instances: 1, in_startup: true },
  ];
  const DEMO_PLACA: Placa = {
    marca: "nvidia",
    nome: "NVIDIA GeForce GTX 1650",
    driver: "617.14",
    driver_data: "14/07/2026",
    driver_origem: "NVIDIA",
    driver_generico: false,
    driver_dias: 118,
    vram_gb: 4,
  };
  function demoGargalo(): Promise<Gargalo> {
    return new Promise((r) =>
      setTimeout(
        () =>
          r({
            limite: "CpuUmNucleo",
            summary: "Um núcleo do processador está no limite",
            advice: "O jogo usa um núcleo mais que os outros e ele chegou a 98%. Fechar programas pesados e o plano de energia OTIMIZA ajudam; baixar a resolução não muda nada aqui.",
            cpu_total: 46,
            cpu_max_core: 98,
            gpu_percent: 61,
            vram_used_mb: 3120,
            vram_total_mb: 4096,
            ram_available_gb: 2.1,
            ram_total_gb: 8,
            disk_percent: 4,
            samples: 20,
            seconds: 10,
          }),
        1200
      )
    );
  }
</script>

<!-- Quem está pesando agora -->
<section class="secao">
  <div class="topo">
    <h2>Quem está pesando agora</h2>
    <span class="apoio pequeno">ao vivo · % do processador inteiro</span>
  </div>
  {#if erroDosProcessos && !processos}
    <p class="apoio">Não consegui ler os programas: {erroDosProcessos}</p>
  {:else if processos === null}
    <Esqueleto linhas={2} />
  {:else if processos.length === 0}
    <p class="apoio">Nada consumindo de forma relevante.</p>
  {:else}
    <ul class="processos">
      {#each processos as p (p.name)}
        <li>
          <div class="linha">
            <span class="nome">{p.name}{#if p.instances > 1}<span class="apoio"> ×{p.instances}</span>{/if}</span>
            {#if p.in_startup}<span class="selo" title="Abre sozinho quando o Windows liga">volta no boot</span>{/if}
            <span class="numero cpu">{um(p.cpu_percent)}%</span>
          </div>
          <div class="barra"><div style:width={`${(p.cpu_percent / maisPesado) * 100}%`}></div></div>
          <span class="apoio pequeno">{int(p.ram_mb)} MB de memória</span>
        </li>
      {/each}
    </ul>
    <p class="apoio pequeno">O que volta no boot dá para tirar em Inicialização, aqui no Sistema.</p>
  {/if}
</section>

<!-- O que está segurando a máquina -->
<section class="secao">
  <div class="topo">
    <h2>O que está segurando a máquina</h2>
    <button class="btn" disabled={medindo} onclick={medirGargalo}>{medindo ? "Medindo 10 s…" : gargalo ? "Medir de novo" : "Medir 10 s"}</button>
  </div>
  {#if !gargalo && !erroDoGargalo}
    <p class="apoio">
      Mede processador (cada núcleo), placa de vídeo, memória e disco por 10 segundos e diz qual chegou no limite. Com o jogo
      aberto, a resposta é sobre o jogo; sem jogo, sobre o que está rodando. Nada é alterado.
    </p>
  {/if}
  {#if erroDoGargalo}
    <div class="recado" role="alert"><span class="ponto" data-tom="perigo"></span><div><span>{erroDoGargalo}</span></div></div>
  {/if}
  {#if gargalo}
    <div class="veredito">
      <p class="titulo">
        {#if TOM[gargalo.limite]}<span class="ponto" data-tom={TOM[gargalo.limite]}></span>{/if}{gargalo.summary}
      </p>
      {#each gargalo.advice.split(/\n\n/) as paragrafo, i (i)}<p class="apoio">{paragrafo}</p>{/each}
    </div>
    <dl class="leituras">
      <div><dt>Processador</dt><dd class="numero">{int(gargalo.cpu_total)}%</dd><span class="apoio pequeno">núcleo mais cheio: {int(gargalo.cpu_max_core)}%</span></div>
      <div><dt>Placa de vídeo</dt><dd class="numero">{int(gargalo.gpu_percent)}%</dd><span class="apoio pequeno">memória: {int(gargalo.vram_used_mb)}{gargalo.vram_total_mb != null ? ` de ${int(gargalo.vram_total_mb)}` : ""} MB</span></div>
      <div><dt>Memória livre</dt><dd class="numero">{um(gargalo.ram_available_gb)} GB</dd><span class="apoio pequeno">de {int(gargalo.ram_total_gb)} GB</span></div>
      <div><dt>Disco</dt><dd class="numero">{int(gargalo.disk_percent)}%</dd><span class="apoio pequeno">ocupado</span></div>
    </dl>
    <p class="apoio pequeno">Limite: {ROTULO[gargalo.limite]} · {gargalo.samples} leituras em {gargalo.seconds} s.</p>
  {/if}
</section>

<!-- Placa de vídeo e driver -->
<section class="secao">
  <h2>Placa de vídeo</h2>
  {#if placa === undefined}
    <Esqueleto linhas={2} />
  {:else if placa === null}
    <p class="apoio">Não consegui ler a placa de vídeo.</p>
  {:else}
    <dl class="ficha">
      <dt>Placa</dt>
      <dd>{placa.nome ?? "Não consegui identificar a placa"}</dd>
      <dt>Memória de vídeo</dt>
      <dd>{placa.vram_gb > 0 ? `${int(placa.vram_gb)} GB` : "não sei dizer"}</dd>
      <dt>Driver</dt>
      <dd>
        {placa.driver ?? "—"}{#if placa.driver_data}<span class="apoio"> · {placa.driver_data}{placa.driver_dias !== null ? `, há ${placa.driver_dias} dias` : ""}</span>{/if}
      </dd>
      <dt>Origem</dt>
      <dd>{placa.driver_origem ?? "não deu para ler"}</dd>
    </dl>
    {#if placa.driver_generico}
      <p class="aviso"><span class="ponto" data-tom="perigo"></span>Este é o driver genérico do Windows: a placa roda sem a aceleração completa. Instalar o driver do fabricante muda o FPS de verdade.</p>
    {:else}
      <p class="apoio pequeno">Driver mais novo não é automaticamente mais rápido: às vezes melhora um jogo e piora outro. Se um jogo caiu depois de atualizar, volte ao anterior e compare as partidas medidas.</p>
    {/if}
    {#if SITE[placa.marca]}
      <a class="btn quieto" href={SITE[placa.marca]} target="_blank" rel="noreferrer">Baixar o driver no site do fabricante</a>
    {/if}
  {/if}
</section>

<style>
  .topo {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .processos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
  }
  .linha {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 4px;
    font-size: 13px;
  }
  .nome {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cpu {
    margin-left: auto;
  }
  .barra {
    height: 4px;
    border-radius: 2px;
    background: var(--linha);
    overflow: hidden;
  }
  .barra div {
    height: 100%;
    border-radius: 2px;
    background: var(--tinta-2);
    transition: width 400ms ease;
  }
  .veredito {
    display: grid;
    gap: 4px;
  }
  .titulo {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .leituras {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 16px;
    margin: 4px 0 0;
  }
  .leituras > div {
    display: grid;
    gap: 2px;
  }
  .leituras dt {
    color: var(--tinta-2);
    font-size: 12px;
  }
  .leituras dd {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    line-height: 1.1;
  }
  .ficha {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 20px;
    margin: 0;
    font-size: 13px;
  }
  .ficha dt {
    color: var(--tinta-2);
  }
  .ficha dd {
    margin: 0;
  }
  .aviso {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 13px;
  }
  a.btn {
    justify-self: start;
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
  @media (max-width: 900px) {
    .leituras {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
