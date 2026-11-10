<script lang="ts">
  // O driver da placa de vídeo (trazido da interface antiga: `src/main.ts`, ajustes da NVIDIA, perfis e limitador, e
  // `src/v33/app.ts`, AMD e Intel). Só aparece o que o driver respondeu: botão que o driver recusaria é teatro. Cada
  // ajuste entra no histórico e se desfaz sozinho. O driver só grava com o Otimiza como administrador.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri, ponte } from "../lib/ponte";

  type Resultado = { id: string; name: string; success: boolean; applied: boolean; message: string };
  type AjusteNvidia = { id: string; titulo: string; explicacao: string; historico: string; aplicado: boolean };
  type PainelNvidia = {
    estado: "Disponivel" | "SemPlacaNvidia" | "NaoCarregou";
    nota: string;
    ajustes: AjusteNvidia[];
    limites: { executavel: string; fps: number; historico: string }[];
  };
  type PainelAmd = {
    estado: string;
    nota: string;
    ajustes: { anti_lag: boolean | null; afmf: boolean | null; rsr?: boolean | null } | null;
    aplicados: string[];
  };
  type PainelIntel = { estado: string; nota: string; ajustes: { baixa_latencia: number | null } | null; aplicados: string[] };
  type Linha = { titulo: string; texto: string; ligado: boolean; acao: () => Promise<unknown>; porVoce: boolean };

  /** Três perfis, combinações dos seis ajustes da NVIDIA (a tela mostra o que cada um liga). */
  const PERFIS = [
    { nome: "Equilibrado", resumo: "A placa para de baixar o clock entre quadros e o cache de shader fica ligado e sem limite de tamanho.", ajustes: ["energia", "cache_shader", "cache_shader_tamanho"] },
    { nome: "Competitivo", resumo: "O de cima, mais a fila de quadros curta e a sincronia vertical desligada. Troca suavidade por resposta, e pode aparecer rasgo na imagem.", ajustes: ["energia", "cache_shader", "cache_shader_tamanho", "latencia", "vsync"] },
    { nome: "Tudo que há", resumo: "Os seis ajustes. Inclui a filtragem de textura em desempenho, o único que muda como o jogo se parece.", ajustes: ["energia", "cache_shader", "cache_shader_tamanho", "latencia", "vsync", "textura"] },
  ];
  const LIMITES = [60, 120, 144, 165, 240];

  let fabricante = $state<string | null>(null);
  let placa = $state<string | null>(null);
  let nvidia = $state<PainelNvidia | null>(null);
  let linhas = $state<Linha[]>([]);
  let nota = $state<string | null>(null);
  let recado = $state<string | null>(null);
  let ocupado = $state(false);
  let admin = $state<boolean | null>(null);
  let limite = $state(60);

  const demoNvidia: PainelNvidia = {
    estado: "Disponivel",
    nota: "Pelo driver da NVIDIA (NVAPI), como o Painel de Controle da NVIDIA grava.",
    ajustes: [
      { id: "energia", titulo: "Modo de energia: máximo desempenho", explicacao: "A placa para de baixar o clock entre quadros.", historico: "nv-energia", aplicado: true },
      { id: "cache_shader", titulo: "Cache de shader ligado", explicacao: "Menos tranco na primeira vez que um efeito aparece.", historico: "nv-cache", aplicado: false },
      { id: "cache_shader_tamanho", titulo: "Cache de shader sem limite", explicacao: "O cache não é apagado quando enche.", historico: "nv-cache-t", aplicado: false },
      { id: "latencia", titulo: "Modo de baixa latência", explicacao: "Fila de quadros curta: menos atraso com a placa no limite.", historico: "nv-lat", aplicado: false },
      { id: "vsync", titulo: "Sincronia vertical desligada", explicacao: "Menos atraso; pode aparecer rasgo.", historico: "nv-vsync", aplicado: false },
      { id: "textura", titulo: "Filtragem de textura: desempenho", explicacao: "Um pouco mais de FPS, com textura menos nítida de longe.", historico: "nv-tex", aplicado: false },
    ],
    limites: [],
  };

  async function carregar() {
    nota = null;
    const hw = await ponte.hardwareUltimo().catch(() => null);
    fabricante = hw?.placa.fabricante ?? "Desconhecido";
    placa = hw ? `${hw.placa.nome}${hw.placa.driver ? ` · driver ${hw.placa.driver}` : ""}` : null;
    admin = await ponte.administrador().catch(() => null);
    try {
      if (fabricante === "Nvidia") {
        nvidia = noTauri ? await invoke<PainelNvidia>("ajustes_do_driver_nvidia") : structuredClone(demoNvidia);
        nota = nvidia.nota;
        linhas = nvidia.estado !== "Disponivel" ? [] : nvidia.ajustes.map((a) => ({
          titulo: a.titulo,
          texto: a.explicacao,
          ligado: a.aplicado,
          porVoce: false,
          acao: () => (a.aplicado ? ponte.desfazer(a.historico) : invoke<Resultado>("aplicar_ajuste_nvidia", { opcao: a.id })),
        }));
      } else if (fabricante === "Amd") {
        const p = noTauri ? await invoke<PainelAmd>("driver_amd") : ({ estado: "NaoCarregou", nota: "Demonstração: sem placa AMD.", ajustes: null, aplicados: [] } as PainelAmd);
        nota = p.nota;
        const l: Linha[] = [];
        const item = (valor: boolean | null | undefined, id: string, titulo: string, texto: string) => {
          if (valor === null || valor === undefined) return;
          const meu = p.aplicados.includes(id);
          l.push({ titulo, texto, ligado: !!valor, porVoce: !!valor && !meu, acao: () => (meu ? ponte.desfazer(id) : invoke<Resultado>("aplicar_ajuste_amd", { opcao: id })) });
        };
        if (p.estado === "Disponivel" && p.ajustes) {
          item(p.ajustes.anti_lag, "amd-antilag", "Radeon Anti-Lag", "Menos atraso quando a placa está no limite.");
          item(p.ajustes.afmf, "amd-afmf", "AMD Fluid Motion Frames", "Geração de quadros do driver. Pede tela cheia ou sem bordas.");
          item(p.ajustes.rsr, "amd-rsr", "Escala do driver (Radeon Super Resolution)", "Para jogos em tela cheia: escolha no jogo uma resolução menor que a do monitor e o driver amplia. Em janela sem bordas, use a escala do Frame Boost.");
        }
        linhas = l;
      } else if (fabricante === "Intel") {
        const p = noTauri ? await invoke<PainelIntel>("driver_intel") : ({ estado: "NaoCarregou", nota: "Demonstração: sem placa Intel.", ajustes: null, aplicados: [] } as PainelIntel);
        nota = p.nota;
        if (p.estado === "Disponivel" && p.ajustes) {
          const meu = p.aplicados.includes("intel-latencia");
          const ligado = (p.ajustes.baixa_latencia ?? 0) > 0;
          linhas = [{
            titulo: "Baixa latência",
            texto: "Menos atraso entre o clique e a tela.",
            ligado,
            porVoce: ligado && !meu,
            acao: () => (meu ? ponte.desfazer("intel-latencia") : invoke<Resultado>("aplicar_ajuste_intel", { opcao: "intel-latencia" })),
          }];
        } else linhas = [];
      } else {
        linhas = [];
        nota = "Não identifiquei a placa de vídeo. Os ajustes do Windows continuam valendo.";
      }
    } catch (e) {
      linhas = [];
      nota = String(e);
    }
  }

  onMount(carregar);

  async function agir(f: () => Promise<unknown>) {
    ocupado = true;
    recado = null;
    try {
      const r = (await f()) as { message?: string } | undefined;
      if (r?.message) recado = r.message;
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = false;
      await carregar();
    }
  }

  async function aplicarPerfil(ids: string[]) {
    if (!nvidia) return;
    const ligados = new Set(nvidia.ajustes.filter((a) => a.aplicado).map((a) => a.id));
    ocupado = true;
    // Um de cada vez, parando no primeiro que falhar: metade de um perfil não é nada.
    for (const id of ids.filter((x) => !ligados.has(x))) {
      try {
        if (noTauri) await invoke<Resultado>("aplicar_ajuste_nvidia", { opcao: id });
      } catch (e) {
        recado = String(e);
        ocupado = false;
        await carregar();
        return;
      }
    }
    recado = "Pronto. Cada ajuste entrou no histórico e pode ser desfeito sozinho.";
    ocupado = false;
    await carregar();
  }

  async function limitar() {
    const jogo = noTauri ? await invoke<{ executavel: string } | null>("jogo_aberto").catch(() => null) : { executavel: "FiveM_GTAProcess.exe" };
    if (!jogo) {
      recado = "Abra o jogo e volte aqui: o limite vai no perfil do executável que estiver rodando.";
      return;
    }
    await agir(() => (noTauri ? invoke<Resultado>("limitar_fps_nvidia", { executavel: jogo.executavel, fps: limite }) : Promise.resolve({ message: `${jogo.executavel} limitado a ${limite} FPS pelo driver.` })));
  }
</script>

<section class="secao driver">
  <div class="topo">
    <h2>Driver da placa de vídeo</h2>
    {#if placa}<span class="apoio pequeno">{placa}</span>{/if}
  </div>
  {#if admin === false && linhas.length}
    <p class="aviso">
      O driver só grava ajustes com o Otimiza aberto como administrador.
      <button class="btn quieto" onclick={() => ponte.abrirComoAdministrador()}>Abrir como administrador</button>
    </p>
  {/if}

  {#if fabricante === "Nvidia" && nvidia?.estado === "Disponivel"}
    <div class="perfis">
      {#each PERFIS as p (p.nome)}
        {@const ligados = p.ajustes.filter((id) => nvidia!.ajustes.find((a) => a.id === id)?.aplicado).length}
        <div class="perfil" class:inteiro={ligados === p.ajustes.length}>
          <span class="titulo">{p.nome} <span class="apoio pequeno">{ligados} de {p.ajustes.length}</span></span>
          <p class="apoio pequeno">{p.resumo}</p>
          <button class="btn" disabled={ocupado || ligados === p.ajustes.length || admin === false} onclick={() => aplicarPerfil(p.ajustes)}>
            {ligados === p.ajustes.length ? "Já está aplicado" : "Aplicar os que faltam"}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if linhas.length}
    <ul class="lista-ajustes">
      {#each linhas as l (l.titulo)}
        <li>
          <div>
            <span>{l.titulo}</span>
            <p class="apoio pequeno">{l.texto}{l.porVoce ? " Ligado por você no painel do driver." : ""}</p>
          </div>
          <button class="interruptor" role="switch" aria-checked={l.ligado} aria-label={l.titulo} disabled={ocupado || l.porVoce || admin === false} onclick={() => agir(l.acao)}></button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if fabricante === "Nvidia" && nvidia?.estado === "Disponivel"}
    <div class="limitador">
      <span class="titulo">Limitar o FPS do jogo aberto pelo driver</span>
      <p class="apoio pequeno">Um teto estável tira o tranco de quem oscila acima do monitor. Vai no perfil do executável, e se desfaz sozinho.</p>
      <div class="segmento" role="radiogroup" aria-label="Limite de FPS">
        {#each LIMITES as f (f)}
          <button role="radio" aria-checked={limite === f} onclick={() => (limite = f)}>{f}</button>
        {/each}
      </div>
      <button class="btn" disabled={ocupado || admin === false} onclick={limitar}>Limitar o jogo aberto</button>
      {#each nvidia.limites as lim (lim.historico)}
        <div class="limite">
          <span>{lim.executavel} · limitado a {lim.fps} FPS</span>
          <button class="btn quieto" disabled={ocupado} onclick={() => agir(() => ponte.desfazer(lim.historico))}>Desfazer</button>
        </div>
      {/each}
    </div>
  {/if}

  {#if nota}<p class="apoio pequeno">{nota}</p>{/if}
  {#if recado}<p class="recado">{recado}</p>{/if}
</section>

<style>
  .driver {
    display: grid;
    gap: 12px;
  }
  .topo {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
  }
  /* Os níveis do driver em lista: nome e resumo à esquerda, o botão à direita. Três cartões com o mesmo botão
     repetido viravam uma vitrine. */
  .perfis {
    display: grid;
  }
  .perfil {
    display: grid;
    grid-template-columns: 1fr auto;
    column-gap: 16px;
    row-gap: 2px;
    padding: 10px 0;
    border-top: 1px solid var(--linha);
  }
  .perfil:first-child {
    border-top: 0;
  }
  .perfil > .btn {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
  }
  .titulo {
    font-weight: 500;
  }
  p {
    margin: 0;
  }
  .lista-ajustes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .lista-ajustes li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-size: 13px;
  }
  .limitador {
    display: grid;
    gap: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--linha);
    justify-items: start;
  }
  .limite {
    display: flex;
    gap: 12px;
    align-items: center;
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
