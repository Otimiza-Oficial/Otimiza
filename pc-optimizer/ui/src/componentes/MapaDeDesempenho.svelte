<script lang="ts">
  // O mapa de desempenho (trazido da interface 3.6, `src/mapa.ts`): 40 s dos contadores do Windows (processador,
  // memória, disco, placa e memória de vídeo) e, com um jogo aberto, os quadros dele no mesmo intervalo pelo PresentMon
  // (`diagnostico_ao_vivo`). O classificador do Rust aponta TODOS os gargalos sustentados, com os números; o detetive
  // de travadas diz o que coincidiu com cada uma. O que o Windows não entregou aparece como "não medido", nunca zero.
  // Os limites de FPS escondidos moram na área Energia.
  import { onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Classe =
    | "CpuTodosNucleos" | "CpuUmNucleo" | "Gpu" | "MemoriaRam" | "MemoriaVideo" | "Disco"
    | "LimiteTermico" | "LimiteEletrico" | "TetoDeQuadros" | "Engasgo" | "ForaDoHardware"
    | "StreamingDeAssets" | "Rede";
  type Achado = { classe: Classe; forca: "Causa" | "Hipotese"; evidencia: string; idade_ms: number | null };
  type Amostra = {
    cpu_total_pct: number | null;
    cpu_nucleo_max_pct: number | null;
    clock_efetivo_mhz: number | null;
    gpu_pct: number | null;
    vram_usada_mb: number | null;
    ram_disponivel_mb: number | null;
    disco_latencia_ms: number | null;
  };
  type Saude = {
    quadros: number;
    duracao_s: number;
    fps_medio: number;
    p99_ms: number | null;
    low_1pct_fps: number | null;
    low_01pct_fps: number | null;
    engasgos: { micro: number; perceptivel: number; severo: number; extremo: number };
    indice_de_fluidez: number | null;
  };
  type Suspeito =
    | { tipo: "Disco" }
    | { tipo: "Paginacao" }
    | { tipo: "Vram" }
    | { tipo: "ThreadPrincipal" }
    | { tipo: "SegundoPlano"; processo: string };
  type ResumoGpu = {
    amostras: number;
    temperatura_max_c: number | null;
    clock_medio_mhz: number | null;
    potencia_media_w: number | null;
    limite_w: number | null;
    pct_termico: number;
    pct_teto_de_energia: number;
    pct_freio_de_hardware: number;
    motivos_lidos: boolean;
  };
  type Diagnostico = {
    placa: { nome: string; vram_total_mb: number } | null;
    jogo: string | null;
    amostras: Amostra[];
    saude: Saude | null;
    quadros_erro: string | null;
    gargalo: {
      conclusao: "SemEvidencia" | "SemCarga" | "NadaNoLimite" | "Encontrado";
      achados: Achado[];
      nao_verificado: { classe: string; falta: string }[];
      classes_avaliadas: number;
      classes_totais: number;
    };
    travadas: {
      travadas: { instante_ms: number; duracao_ms: number; gravidade: string }[];
      pistas: { suspeito: Suspeito; forca: "Media" | "Alta"; travadas: number }[];
      sem_causa_visivel: number;
    } | null;
    sensores_da_placa: ResumoGpu | null;
  };

  // 40 s: a 60 FPS, 20 s dão uns 1.200 quadros, abaixo dos 2.000 que o 1% pior e o P99 exigem.
  const SEGUNDOS = 40;
  const ESPERA = 8;

  /** O que cada classe quer dizer e o que fazer, escrito para quem joga. */
  const NA_TELA: Record<Classe, { titulo: string; explica: string; faz: string }> = {
    CpuUmNucleo: {
      titulo: "O jogo está preso em um núcleo do processador",
      explica: "Um núcleo está no limite enquanto os outros sobram. O uso total parece baixo, mas o jogo não consegue dividir esse trabalho.",
      faz: "Placa de vídeo nova não resolve. Ajuda baixar distância de visão e densidade de população no jogo, fechar programas pesados e ligar o XMP da memória.",
    },
    CpuTodosNucleos: {
      titulo: "O processador inteiro está no limite",
      explica: "Todos os núcleos ocupados: a placa recebe quadros mais devagar do que consegue desenhar.",
      faz: "Feche o que roda em segundo plano e reduza o que pesa no processador: população, física, distância.",
    },
    Gpu: {
      titulo: "A placa de vídeo é o limite",
      explica: "A placa está no máximo. É o caso em que ajuste gráfico rende mais FPS.",
      faz: "Baixe primeiro sombras, pós-processamento, anti-serrilhado pesado e resolução de renderização. Escala e Frame Boost também entram aqui.",
    },
    MemoriaRam: {
      titulo: "A memória está apertada",
      explica: "Com a memória no limite o Windows passa a buscar no disco, e cada busca é uma travada em potencial.",
      faz: "Feche navegador e programas pesados antes de jogar. Se acontece sempre, o limite é a quantidade de memória.",
    },
    MemoriaVideo: {
      titulo: "A memória da placa de vídeo está apertada",
      explica: "Quando ela enche, textura vai para a memória comum, muito mais lenta: aparece como travada, não como FPS menor.",
      faz: "Baixe a qualidade de textura um degrau e feche programas que usam a placa.",
    },
    Disco: {
      titulo: "O disco está no limite",
      explica: "O disco ficou ocupado a maior parte do tempo; o jogo espera quando precisa ler algo.",
      faz: "Veja na área Jogos se o jogo está num HD e se algo está baixando ou atualizando.",
    },
    LimiteTermico: {
      titulo: "O processador está sendo segurado por temperatura",
      explica: "O próprio Windows reporta que o firmware reduziu a velocidade por calor.",
      faz: "Nenhum ajuste de programa resolve: limpeza, pasta térmica e circulação de ar.",
    },
    LimiteEletrico: {
      titulo: "O processador está sendo segurado por energia",
      explica: "O firmware limitou a potência: notebook na bateria, fonte ou limite da placa-mãe.",
      faz: "No notebook, jogue na tomada. No desktop, confira os limites de potência na BIOS.",
    },
    TetoDeQuadros: {
      titulo: "O FPS está preso na taxa do monitor",
      explica: "O jogo entrega exatamente o que o monitor mostra: V-Sync ou limitador.",
      faz: "Se é a taxa do monitor, é o esperado. Limites escondidos abaixo dela aparecem na área Energia.",
    },
    Engasgo: {
      titulo: "Travadas frequentes durante a partida",
      explica: "Muitos quadros bem acima do normal. O detetive de travadas, logo abaixo, mostra o que coincidiu com elas.",
      faz: "Veja as pistas do detetive.",
    },
    ForaDoHardware: {
      titulo: "O limite não está no hardware",
      explica: "Processador e placa sobrando ao mesmo tempo, durante a partida. O limite é o motor do jogo, o servidor ou um teto de quadros.",
      faz: "Ajuste de Windows não muda isto. Confira os limites de FPS escondidos e, no FiveM, o servidor.",
    },
    StreamingDeAssets: {
      titulo: "O jogo está esperando o disco",
      explica: "As travadas caem justamente quando o disco está ocupado: o jogo carregando conteúdo.",
      faz: "Instale o jogo num SSD e evite downloads durante a partida.",
    },
    Rede: {
      titulo: "A conexão está instável ou perdendo pacote",
      explica: "Rede que engasga parece FPS baixo: teleporte, tiro que não registra.",
      faz: "Perda de pacote vem do Wi-Fi, do provedor ou do servidor. Cabo em vez de Wi-Fi resolve a maioria.",
    },
  };

  let etapa = $state<"parado" | "contando" | "medindo">("parado");
  let restam = $state(0);
  let pct = $state(0);
  let d = $state<Diagnostico | null>(null);
  let erro = $state<string | null>(null);
  let relogio: ReturnType<typeof setInterval> | undefined;

  onDestroy(() => clearInterval(relogio));

  async function medir(espera: number) {
    if (etapa !== "parado") return;
    erro = null;
    try {
      etapa = "contando";
      for (restam = espera; restam > 0; restam--) await new Promise((r) => setTimeout(r, 1000));
      etapa = "medindo";
      const inicio = Date.now();
      pct = 0;
      relogio = setInterval(() => (pct = Math.min(100, ((Date.now() - inicio) / 1000 / SEGUNDOS) * 100)), 400);
      d = noTauri ? await invoke<Diagnostico>("diagnostico_ao_vivo", { segundos: SEGUNDOS }) : await demo();
    } catch (e) {
      erro = String(e);
    } finally {
      clearInterval(relogio);
      etapa = "parado";
    }
  }

  function mediana(vs: (number | null)[]): number | null {
    const v = vs.filter((x): x is number => x !== null && Number.isFinite(x)).sort((a, b) => a - b);
    if (!v.length) return null;
    const m = Math.floor(v.length / 2);
    return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
  }
  const num = (v: number | null | undefined, casas = 0, sufixo = "") =>
    v === null || v === undefined || !Number.isFinite(v) ? "não medido" : `${v.toLocaleString("pt-BR", { maximumFractionDigits: casas, minimumFractionDigits: casas })}${sufixo}`;
  /** Na cauda, `null` é amostra curta, não falta de medição. */
  const cauda = (v: number | null, casas = 0, sufixo = "") => (v === null ? "amostra curta" : num(v, casas, sufixo));

  const medidas = $derived.by(() => {
    if (!d) return [];
    const a = d.amostras;
    const vramTotal = d.placa?.vram_total_mb ?? null;
    return [
      { nome: "Processador", valor: num(mediana(a.map((x) => x.cpu_total_pct)), 0, "%"), sub: `núcleo mais ocupado ${num(mediana(a.map((x) => x.cpu_nucleo_max_pct)), 0, "%")} · ${num(mediana(a.map((x) => x.clock_efetivo_mhz)), 0, " MHz")}` },
      { nome: "Memória livre", valor: num(mediana(a.map((x) => x.ram_disponivel_mb)), 0, " MB"), sub: "" },
      { nome: "Disco", valor: num(mediana(a.map((x) => x.disco_latencia_ms)), 1, " ms"), sub: "latência média" },
      { nome: "Placa de vídeo", valor: num(mediana(a.map((x) => x.gpu_pct)), 0, "%"), sub: d.placa?.nome ?? "placa não identificada" },
      { nome: "Memória de vídeo", valor: num(mediana(a.map((x) => x.vram_usada_mb)), 0, " MB"), sub: vramTotal ? `de ${num(vramTotal, 0, " MB")}` : "total não lido" },
    ];
  });

  /** Temperatura e freio da placa na MESMA janela (só NVIDIA): "limitada por calor em 23% do teste" muda a decisão. */
  const placa = $derived.by(() => {
    const p = d?.sensores_da_placa;
    if (!p || !p.amostras) return null;
    const numeros = [
      p.temperatura_max_c !== null ? `máx. ${Math.round(p.temperatura_max_c)} °C` : null,
      p.clock_medio_mhz !== null ? num(p.clock_medio_mhz, 0, " MHz") : null,
      p.potencia_media_w !== null ? `${Math.round(p.potencia_media_w)} W${p.limite_w !== null ? ` de ${Math.round(p.limite_w)} W` : ""}` : null,
    ].filter(Boolean).join(" · ");
    if (!p.motivos_lidos) return { tom: null, texto: `Placa: ${numeros}. O driver não informou se estava segurando o clock.` };
    if (p.pct_termico >= 5)
      return { tom: "atencao", texto: `A placa foi limitada por temperatura em ${Math.round(p.pct_termico)}% deste teste (${numeros}). É físico: poeira, ventoinhas e fluxo de ar do gabinete. Nenhum ajuste de Windows resolve.` };
    if (p.pct_freio_de_hardware >= 5)
      return { tom: "atencao", texto: `A placa acionou o freio de hardware em ${Math.round(p.pct_freio_de_hardware)}% deste teste (${numeros}). Costuma ser a fonte, o conector de energia ou a proteção térmica dela.` };
    if (p.pct_teto_de_energia >= 60)
      return { tom: null, texto: `Placa: ${numeros}. Passou ${Math.round(p.pct_teto_de_energia)}% do tempo no teto de energia, o normal de uma placa em carga máxima.` };
    return { tom: null, texto: `Placa: ${numeros}. Nada segurou o clock além do normal.` };
  });

  const semAchado = $derived.by(() => {
    if (!d) return "";
    const c = d.gargalo.conclusao;
    if (c === "SemCarga") return "A máquina estava parada durante a medição: sem carga não há gargalo para achar. Meça com o jogo rodando.";
    if (c === "SemEvidencia") return "Não houve medição suficiente para classificar.";
    return d.jogo
      ? "Nada encostou no limite. Com o jogo aberto, isso costuma dizer que o limite é o próprio jogo (o motor, o servidor) ou que a máquina está folgada para essa cena."
      : "Nada encostou no limite. Sem jogo aberto é o esperado: meça com o jogo rodando.";
  });

  function suspeito(s: Suspeito): { titulo: string; faz: string } {
    switch (s.tipo) {
      case "Disco":
        return { titulo: "o disco demorou para responder", faz: "Veja se o jogo está num HD e se algo está baixando ou atualizando." };
      case "Paginacao":
        return { titulo: "o Windows foi buscar memória no disco", faz: "Feche o navegador e programas pesados antes de jogar." };
      case "Vram":
        return { titulo: "a memória da placa de vídeo encheu", faz: "Baixe a qualidade de textura um degrau." };
      case "ThreadPrincipal":
        return { titulo: "o núcleo principal do processador bateu no teto", faz: "Baixe distância de visão e densidade de população no jogo." };
      case "SegundoPlano":
        return { titulo: `o ${s.processo} disparou o uso de processador`, faz: "Feche esse programa antes de jogar." };
    }
  }

  async function demo(): Promise<Diagnostico> {
    await new Promise((r) => setTimeout(r, 600));
    const a: Amostra = { cpu_total_pct: 46, cpu_nucleo_max_pct: 97, clock_efetivo_mhz: 4410, gpu_pct: 61, vram_usada_mb: 5120, ram_disponivel_mb: 3900, disco_latencia_ms: 1.8 };
    return {
      placa: { nome: "NVIDIA GeForce RTX 3060", vram_total_mb: 12288 },
      jogo: "FiveM",
      amostras: [a],
      saude: { quadros: 3480, duracao_s: 40, fps_medio: 87, p99_ms: 21.4, low_1pct_fps: 44, low_01pct_fps: null, engasgos: { micro: 31, perceptivel: 9, severo: 2, extremo: 0 }, indice_de_fluidez: 71 },
      quadros_erro: null,
      gargalo: {
        conclusao: "Encontrado",
        achados: [{ classe: "CpuUmNucleo", forca: "Causa", evidencia: "núcleo mais ocupado em 97% com 46% no total, 38 de 80 amostras", idade_ms: null }],
        nao_verificado: [{ classe: "Rede", falta: "sem servidor conhecido" }],
        classes_avaliadas: 12,
        classes_totais: 13,
      },
      travadas: { travadas: [{ instante_ms: 0, duracao_ms: 120, gravidade: "Severo" }, { instante_ms: 0, duracao_ms: 80, gravidade: "Perceptivel" }], pistas: [{ suspeito: { tipo: "ThreadPrincipal" }, forca: "Alta", travadas: 2 }], sem_causa_visivel: 0 },
      sensores_da_placa: { amostras: 80, temperatura_max_c: 71, clock_medio_mhz: 1830, potencia_media_w: 118, limite_w: 170, pct_termico: 0, pct_teto_de_energia: 0, pct_freio_de_hardware: 0, motivos_lidos: true },
    };
  }
</script>

<section class="secao">
  <h2>O que segura o jogo, medido no jogo</h2>
  <p class="apoio texto">
    Mede por {SEGUNDOS} segundos processador (com o núcleo mais ocupado), memória, disco, placa e memória de vídeo. Com
    um jogo aberto, mede os quadros dele no mesmo intervalo. Os números saem dos contadores do Windows; nada entra no jogo.
  </p>

  {#if etapa === "contando"}
    <p>Volte para o jogo: a medição começa em {restam} s.</p>
  {:else if etapa === "medindo"}
    <div class="progresso" role="progressbar" aria-valuenow={Math.round(pct)} aria-valuemin={0} aria-valuemax={100}>
      <div class="barra" style:width="{pct}%"></div>
    </div>
    <p class="apoio">Medindo, jogue normalmente. {Math.round(pct)}%</p>
  {:else}
    <div class="acoes">
      <button class="btn primario" onclick={() => medir(ESPERA)}>{d ? "Medir de novo no jogo" : "Medir no jogo"}</button>
      <button class="btn" onclick={() => medir(0)}>Medir agora</button>
      <span class="apoio pequeno">"No jogo" dá {ESPERA} s para você voltar para ele.</span>
    </div>
  {/if}
  {#if erro}<p class="erro">{erro}</p>{/if}

  {#if d && etapa === "parado"}
    <dl class="medidas">
      {#each medidas as m (m.nome)}
        <div>
          <dt>{m.nome}</dt>
          <dd><span class="numero">{m.valor}</span>{#if m.sub}<span class="apoio pequeno">{m.sub}</span>{/if}</dd>
        </div>
      {/each}
    </dl>
    {#if placa}<p class="pequeno" class:atencao={placa.tom === "atencao"}>{placa.texto}</p>{/if}

    {#if d.gargalo.achados.length}
      <ul class="achados">
        {#each d.gargalo.achados as a, i (i)}
          {@const t = NA_TELA[a.classe]}
          <li>
            <div class="cabeca">
              <strong>{t.titulo}</strong>
              <span class="selo" class:medido={a.forca === "Causa"} class:estimado={a.forca !== "Causa"}>{a.forca === "Causa" ? "medido agora" : "hipótese"}</span>
            </div>
            <p class="apoio">{t.explica}</p>
            <p class="pequeno"><strong>O que fazer:</strong> {t.faz}</p>
            <p class="apoio pequeno">{a.evidencia}</p>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="apoio">{semAchado}</p>
    {/if}
    <p class="apoio pequeno">
      {d.gargalo.classes_avaliadas} de {d.gargalo.classes_totais} tipos de gargalo avaliados{d.gargalo.nao_verificado.length
        ? `; sem dado para: ${d.gargalo.nao_verificado.map((x) => x.classe).join(", ")}`
        : ""}.
    </p>

    {#if d.saude}
      {@const s = d.saude}
      <h3>Os quadros de {d.jogo ?? "jogo"} <span class="apoio">{num(s.duracao_s, 0, " s")}, {num(s.quadros)} quadros</span></h3>
      <dl class="medidas">
        <div><dt>FPS médio</dt><dd class="numero">{num(s.fps_medio)}</dd></div>
        <div><dt>1% pior</dt><dd class="numero">{cauda(s.low_1pct_fps)}</dd></div>
        <div><dt>0,1% pior</dt><dd class="numero">{cauda(s.low_01pct_fps)}</dd></div>
        <div><dt>P99 do tempo de quadro</dt><dd class="numero">{cauda(s.p99_ms, 1, " ms")}</dd></div>
        <div><dt>Índice de fluidez</dt><dd class="numero">{cauda(s.indice_de_fluidez)}</dd></div>
      </dl>
      <p class="pequeno">
        Travadas: {num(s.engasgos.micro)} leves, {num(s.engasgos.perceptivel)} perceptíveis, {num(s.engasgos.severo)} graves e
        {num(s.engasgos.extremo)} congelamentos de 100 ms ou mais.
      </p>
      <p class="apoio pequeno">
        O 0,1% pior só aparece com 10.000 quadros, e o 1% pior e o P99 com 2.000: abaixo disso seriam poucos quadros para
        dizer alguma coisa. O índice de fluidez sai desses números e nunca aparece sem eles.
      </p>
    {:else if d.jogo && d.quadros_erro}
      <p class="atencao pequeno">O {d.jogo} estava aberto, mas os quadros não puderam ser medidos: {d.quadros_erro}</p>
    {/if}

    {#if d.travadas}
      {@const t = d.travadas}
      {#if !t.travadas.length}
        <p class="apoio pequeno">Nenhuma travada perceptível nesta medição.</p>
      {:else}
        <h3>O que o PC fazia em cada travada</h3>
        <ul class="achados">
          {#each t.pistas as p, i (i)}
            {@const f = suspeito(p.suspeito)}
            <li>
              <div class="cabeca">
                <strong>Em {p.travadas} de {t.travadas.length}, {f.titulo}</strong>
                <span class="apoio pequeno">coincidência {p.forca === "Alta" ? "forte" : "média"}</span>
              </div>
              <p class="pequeno"><strong>O que fazer:</strong> {f.faz}</p>
            </li>
          {/each}
        </ul>
        {#if t.sem_causa_visivel}
          <p class="apoio pequeno">
            {t.sem_causa_visivel === 1 ? "1 travada" : `${t.sem_causa_visivel} travadas`} sem salto de disco, memória, placa ou
            programa na mesma hora. Costuma ser compilação de shader ou o próprio jogo, e nenhum ajuste de Windows resolve.
          </p>
        {/if}
        <p class="apoio pequeno">
          A telemetria é lida a cada meio segundo: isto é coincidência no tempo, não causa provada. Forte quer dizer que
          aconteceu em pelo menos metade das travadas.
        </p>
      {/if}
    {/if}
  {/if}
</section>

<style>
  .texto {
    max-width: 64ch;
  }
  .acoes {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .progresso {
    height: 4px;
    border-radius: 2px;
    background: var(--linha);
    overflow: hidden;
  }
  .barra {
    height: 100%;
    background: var(--tinta);
    transition: width 0.4s linear;
  }
  .medidas {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px 20px;
    margin: 4px 0 0;
  }
  .medidas dt {
    font-size: 12px;
    color: var(--tinta-2);
  }
  .medidas dd {
    margin: 2px 0 0;
    display: grid;
    gap: 2px;
  }
  .medidas .numero {
    font-size: 18px;
    font-weight: 600;
  }
  .achados {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
  }
  .achados li {
    display: grid;
    gap: 3px;
    padding: 10px 0;
    border-top: 1px solid var(--linha);
  }
  .cabeca {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }
  h3 {
    margin: 8px 0 0;
    font-size: 14px;
    font-weight: 600;
  }
  h3 .apoio {
    font-weight: 400;
    font-size: 12px;
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
  .atencao {
    color: var(--atencao);
  }
  .erro {
    color: var(--perigo);
    font-size: 13px;
  }
</style>
