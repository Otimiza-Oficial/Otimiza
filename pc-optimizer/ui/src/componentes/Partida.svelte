<script lang="ts">
  // A última partida, por que o jogo roda assim e o que este PC permite (trazidos da interface 3.6, `src/main.ts`:
  // `carregarRelatorioDaPartida`, `carregarDiagnosticoDaPartida` e `carregarPerfilDoPc`). Só o que foi medido: o que
  // não se leu diz que não se leu, sem virar zero. A causa é sempre inferida dos números, por isso "evidência forte",
  // nunca "medido". Tipos escritos à mão, os mesmos da 3.6.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { noTauri } from "../lib/ponte";

  type Gargalo = "Gpu" | "Cpu" | "Espera" | "NaoDeuParaSaber";
  type RelatorioDaPartida = {
    jogo: string;
    inicio: number;
    fim: number;
    janelas: number;
    janelas_comparaveis: number;
    fps_medio: number | null;
    pior_low_1pct: number | null;
    gargalo: Gargalo | null;
    trancos: number | null;
    janelas_com_trancos: number;
    trancos_com_disco: number | null;
    freios: number | null;
    janelas_com_freio_lido: number;
    ja_freava: boolean;
    anteriores: { partidas: number; mediana_fps: number; menor_fps: number; maior_fps: number } | null;
  };

  type TipoDeCausa = "Processador" | "PlacaDeVideo" | "PresoNaTaxaDoMonitor" | "LimiteDeFps" | "CalorOuEnergia" | "DiscoNosTrancos" | "QuadroCopiado";
  type Ajuste =
    | "PerfilGraficoMaisLeve"
    | "MenosCargaNoProcessador"
    | "PlanoDeEnergiaOtimiza"
    | "FecharProgramasPesados"
    | "TirarLimiteDeFps"
    | "ConferirRefrigeracao"
    | "JogoNoSsd"
    | "TelaCheiaOuSemBordaComFlip"
    | "MaisResolucaoOuQualidade"
    | "BaixarResolucao";
  type Causa = { tipo: TipoDeCausa; confianca: "Alta" | "Media" | "Hipotese"; evidencia: string; ajuda: Ajuste[]; nao_ajuda: Ajuste[]; janelas: number };
  type Diagnostico = { jogo: string | null; janelas_lidas: number; causas: Causa[]; lacunas: string[] };
  type PerfilDoJogo = "SemTeto" | "Equilibrado" | "Competitivo";
  type Estrategia = { tipo: "PlanoOtimiza" } | { tipo: "PerfilGrafico"; perfil: PerfilDoJogo } | { tipo: "ModoJogo" };
  type Oferta =
    | { estado: "Oferecer" }
    | { estado: "NaoOferecer"; quando: number; contexto: "Igual" | "NaoLido" }
    | { estado: "ProvarDeNovo"; quando: number }
    | { estado: "JaProvado"; resultado: "Ganhou" | "Piorou" | "SemDiferenca"; quando: number };
  type Passo = { ajuste: Ajuste; estrategia: Estrategia | null; aplicado: boolean; oferta: Oferta; como_provar: "ProvaNoJogo" | "ProximasPartidas" | "PeloCliente" };
  type Receita = { diagnostico: Diagnostico; memoria_ilegivel: string | null; por_causa: [Causa, Passo[]][] };

  type Limitacao =
    | { tipo: "MonitorAbaixoDoMaximo"; atual: number; maximo: number }
    | { tipo: "MonitorLimitaATela"; hz: number }
    | { tipo: "PoucaMemoria"; gb: number }
    | { tipo: "PoucosNucleos"; nucleos: number }
    | { tipo: "PoucaMemoriaDeVideo"; gb: number }
    | { tipo: "VideoIntegrado" }
    | { tipo: "SistemaEmHd" }
    | { tipo: "Notebook" };
  type PerfilDoPc = { limitacoes: Limitacao[]; nao_lido: string[] };

  const GARGALO: Record<Gargalo, string> = {
    Cpu: "o processador",
    Gpu: "a placa de vídeo",
    Espera: "limite de FPS ou V-Sync",
    NaoDeuParaSaber: "nenhum ficou claro",
  };
  const TITULO_DA_CAUSA: Record<TipoDeCausa, string> = {
    Processador: "O processador segura o FPS",
    PlacaDeVideo: "A placa de vídeo segura o FPS",
    PresoNaTaxaDoMonitor: "O FPS já está no máximo que o monitor mostra",
    LimiteDeFps: "Um limite de FPS segura o jogo",
    CalorOuEnergia: "A placa freou por calor ou energia",
    DiscoNosTrancos: "Os trancos vêm do disco",
    QuadroCopiado: "O Windows copia cada quadro",
  };
  const NOME_DO_AJUSTE: Record<Ajuste, string> = {
    PerfilGraficoMaisLeve: "gráfico mais leve no jogo",
    MenosCargaNoProcessador: "menos distância de visão e população no jogo",
    PlanoDeEnergiaOtimiza: "plano de energia OTIMIZA",
    FecharProgramasPesados: "fechar programas pesados",
    TirarLimiteDeFps: "tirar o limite de FPS",
    ConferirRefrigeracao: "conferir a ventilação e a limpeza do PC",
    JogoNoSsd: "jogo instalado num SSD",
    TelaCheiaOuSemBordaComFlip: "tela cheia exclusiva (no Windows 10, janela sem borda nem sempre resolve)",
    MaisResolucaoOuQualidade: "subir a qualidade",
    BaixarResolucao: "baixar a resolução",
  };
  const CONFIANCA = { Alta: "evidência forte", Media: "provável", Hipotese: "hipótese" } as const;
  const NOME_DO_PERFIL: Record<PerfilDoJogo, string> = { SemTeto: "Sem teto", Equilibrado: "Equilibrado", Competitivo: "Máximo de FPS" };
  const COMO_PROVAR: Record<Passo["como_provar"], string> = {
    ProvaNoJogo: "a prova no jogo, no Turbo, confirma",
    ProximasPartidas: "as próximas partidas confirmam sozinhas, e o Otimiza desfaz se piorar",
    PeloCliente: "é você quem faz; as próximas partidas medidas mostram o efeito",
  };

  let partida = $state<RelatorioDaPartida | null | undefined>(undefined);
  let receita = $state<Receita | null>(null);
  let diagnostico = $state<Diagnostico | null>(null);
  let erroDoDiagnostico = $state<string | null>(null);
  let perfil = $state<PerfilDoPc | null>(null);

  onMount(async () => {
    if (!noTauri) {
      partida = DEMO_PARTIDA;
      receita = DEMO_RECEITA;
      diagnostico = DEMO_RECEITA.diagnostico;
      perfil = { limitacoes: [{ tipo: "MonitorAbaixoDoMaximo", atual: 60, maximo: 180 }, { tipo: "PoucaMemoria", gb: 8 }], nao_lido: [] };
      return;
    }
    const [p, r, f] = await Promise.allSettled([
      invoke<RelatorioDaPartida | null>("relatorio_da_ultima_partida"),
      invoke<Receita>("receita_da_partida"),
      invoke<PerfilDoPc>("perfil_do_pc"),
    ]);
    partida = p.status === "fulfilled" ? p.value : null;
    perfil = f.status === "fulfilled" ? f.value : null;
    if (r.status === "fulfilled") {
      receita = r.value;
      diagnostico = r.value.diagnostico;
    } else {
      // Sem a receita, o diagnóstico sozinho.
      try {
        diagnostico = await invoke<Diagnostico>("diagnostico_da_partida");
      } catch (e) {
        erroDoDiagnostico = String(e);
      }
    }
  });

  const n = (v: number) => v.toLocaleString("pt-BR", { maximumFractionDigits: 0 });
  const hora = (q: number) => new Date(q * 1000).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });
  const data = (q: number) => new Date(q * 1000).toLocaleDateString("pt-BR");
  const plural = (q: number, um: string, varios: string) => `${q} ${q === 1 ? um : varios}`;

  /** As linhas da partida: cada uma diz quanto da partida a leitura cobriu, quando não cobriu inteira. */
  function linhasDaPartida(r: RelatorioDaPartida): { rotulo: string; texto: string }[] {
    const minutos = Math.round((r.fim - r.inicio) / 60);
    const cobertura = (lidas: number) => (lidas < r.janelas ? ` (em ${lidas} de ${r.janelas} janelas)` : "");
    const fora = r.janelas - r.janelas_comparaveis;
    const l = [
      { rotulo: "Quando", texto: `${hora(r.inicio)}${minutos > 0 ? `, ${minutos} min medidos` : ""} · ${plural(r.janelas, "janela", "janelas")} de 20 s` },
      {
        rotulo: "FPS",
        texto:
          r.fps_medio == null
            ? "Todas as janelas tinham gerador de quadros: ficaram fora da conta."
            : `${n(r.fps_medio)} de média${r.pior_low_1pct != null ? `; o 1% pior chegou a ${n(r.pior_low_1pct)}` : ""}${fora > 0 ? ` (${plural(fora, "janela com gerador ficou fora", "janelas com gerador ficaram fora")})` : ""}`,
      },
    ];
    if (r.gargalo) l.push({ rotulo: "Limite", texto: GARGALO[r.gargalo] });
    l.push({
      rotulo: "Trancos",
      texto:
        r.trancos == null
          ? "Sem o instante de cada tranco nesta partida."
          : `${r.trancos} nas janelas medidas${r.trancos_com_disco ? `, ${r.trancos_com_disco} com o disco ocupado na hora` : ""}${cobertura(r.janelas_com_trancos)}`,
    });
    const placa =
      r.freios == null
        ? "Freio não lido (só placas NVIDIA informam)."
        : r.ja_freava
          ? "Já estava freando por calor ou energia quando a medição começou."
          : r.freios === 0
            ? "Não freou por calor ou energia nas janelas medidas."
            : `Começou a frear ${plural(r.freios, "vez", "vezes")} nas janelas medidas.`;
    l.push({ rotulo: "Placa", texto: `${placa}${r.freios == null ? "" : cobertura(r.janelas_com_freio_lido)}` });
    return l;
  }

  function ondeAplicar(e: Estrategia): string {
    if (e.tipo === "PlanoOtimiza") return "na área Energia";
    if (e.tipo === "ModoJogo") return "nos Ajustes, modo jogo automático";
    return `em Jogos, configuração do jogo, perfil "${NOME_DO_PERFIL[e.perfil]}"`;
  }

  function passosDe(tipo: TipoDeCausa) {
    return receita?.por_causa.find(([c]) => c.tipo === tipo)?.[1] ?? null;
  }

  function lembranca(p: Passo): string {
    if (p.oferta.estado === "ProvarDeNovo") return `Piorou em ${data(p.oferta.quando)}, mas o driver ou o Windows mudou desde então: vale provar de novo.`;
    if (p.oferta.estado === "JaProvado")
      return p.oferta.resultado === "Ganhou" ? `Já provou ganho aqui em ${data(p.oferta.quando)}.` : `Em ${data(p.oferta.quando)} não fez diferença medida aqui.`;
    return "";
  }

  function fraseDaLimitacao(l: Limitacao): { forte: string; resto: string } {
    switch (l.tipo) {
      case "MonitorAbaixoDoMaximo":
        return {
          forte: `Monitor em ${l.atual} Hz, mas ele aceita ${l.maximo} Hz.`,
          resto: `A tela mostra no máximo ${l.atual} quadros por segundo. Subir a taxa deixa o jogo visivelmente mais fluido (o contador de FPS não muda), e o Otimiza faz isso na área Tela.`,
        };
      case "MonitorLimitaATela":
        return {
          forte: `Monitor de ${l.hz} Hz.`,
          resto: `A tela mostra no máximo ${l.hz} quadros por segundo: FPS acima disso ainda reduz um pouco o atraso, mas não aparece como quadro a mais. Aqui o que mais se ganha é estabilidade, não número.`,
        };
      case "PoucaMemoria":
        return {
          forte: `${l.gb} GB de memória.`,
          resto: "Com menos de 12 GB, FiveM e jogos atuais costumam fazer o Windows trocar memória com o disco no meio da partida, e isso vira tranco. Fechar o navegador antes de jogar ajuda; mais memória costuma resolver.",
        };
      case "PoucosNucleos":
        return {
          forte: `${l.nucleos} núcleos no processador.`,
          resto: "Jogos com muita gente na tela (FiveM, battle royale) seguram o FPS no processador; programas abertos em segundo plano pesam mais aqui.",
        };
      case "PoucaMemoriaDeVideo":
        return {
          forte: `${l.gb.toLocaleString("pt-BR", { maximumFractionDigits: 1 })} GB de memória de vídeo.`,
          resto: "Texturas no alto podem não caber: textura média costuma evitar tranco de carregamento, e custa qualidade, não FPS.",
        };
      case "VideoIntegrado":
        return { forte: "Vídeo integrado.", resto: "Não há placa de vídeo dedicada: o vídeo usa a memória do sistema. Gráfico no baixo e resolução menor são o que mais pesa aqui." };
      case "SistemaEmHd":
        return { forte: "Windows num HD, não SSD.", resto: "Carregamento e trancos ao entrar em áreas novas vêm do disco: nenhum ajuste de Windows compensa, só um SSD." };
      case "Notebook":
        return { forte: "Notebook.", resto: "Energia e calor limitam: jogue na tomada e com a ventilação livre. Na bateria o Windows costuma cortar desempenho." };
    }
  }

  const lacunas = $derived(
    [
      ...(diagnostico?.lacunas ?? []),
      receita?.memoria_ilegivel ? `Não consegui ler o registro do que já foi provado neste PC (${receita.memoria_ilegivel}): não dá para dizer o que já piorou.` : "",
    ].filter(Boolean)
  );

  const DEMO_PARTIDA: RelatorioDaPartida = {
    jogo: "FiveM",
    inicio: Date.now() / 1000 - 3600,
    fim: Date.now() / 1000 - 1080,
    janelas: 126,
    janelas_comparaveis: 126,
    fps_medio: 85,
    pior_low_1pct: 41,
    gargalo: "Cpu",
    trancos: 14,
    janelas_com_trancos: 126,
    trancos_com_disco: 3,
    freios: 0,
    janelas_com_freio_lido: 126,
    ja_freava: false,
    anteriores: { partidas: 11, mediana_fps: 81, menor_fps: 70, maior_fps: 92 },
  };
  const DEMO_RECEITA: Receita = {
    diagnostico: {
      jogo: "FiveM",
      janelas_lidas: 126,
      causas: [
        {
          tipo: "Processador",
          confianca: "Alta",
          evidencia: "Em 104 de 126 janelas o processador terminou o quadro depois da placa, que ficou 38% do tempo esperando.",
          ajuda: ["MenosCargaNoProcessador", "PlanoDeEnergiaOtimiza", "FecharProgramasPesados"],
          nao_ajuda: ["BaixarResolucao"],
          janelas: 104,
        },
      ],
      lacunas: [],
    },
    memoria_ilegivel: null,
    por_causa: [],
  };
  DEMO_RECEITA.por_causa = [
    [
      DEMO_RECEITA.diagnostico.causas[0],
      [
        { ajuste: "PlanoDeEnergiaOtimiza", estrategia: { tipo: "PlanoOtimiza" }, aplicado: false, oferta: { estado: "Oferecer" }, como_provar: "ProvaNoJogo" },
        { ajuste: "MenosCargaNoProcessador", estrategia: { tipo: "PerfilGrafico", perfil: "Competitivo" }, aplicado: false, oferta: { estado: "Oferecer" }, como_provar: "ProximasPartidas" },
        { ajuste: "FecharProgramasPesados", estrategia: null, aplicado: false, oferta: { estado: "Oferecer" }, como_provar: "PeloCliente" },
      ],
    ],
  ];
</script>

<!-- Sua última partida -->
{#if partida}
  <section class="secao">
    <div class="topo">
      <h2>Sua última partida</h2>
      <span class="apoio">{partida.jogo}</span>
    </div>
    <dl class="linhas">
      {#each linhasDaPartida(partida) as l (l.rotulo)}
        <dt>{l.rotulo}</dt>
        <dd>{l.texto}</dd>
      {/each}
    </dl>
    <p class="apoio pequeno">
      {#if partida.anteriores}
        Nas suas {plural(partida.anteriores.partidas, "partida anterior", "partidas anteriores")} deste jogo, o FPS médio ficou entre {n(partida.anteriores.menor_fps)}
        e {n(partida.anteriores.maior_fps)} (mediana {n(partida.anteriores.mediana_fps)}). Lugares diferentes do mapa dão números diferentes: isto é
        referência, não prova. Queda provada aparece no Histórico.
      {:else}
        Com mais partidas medidas, aparece aqui a faixa das anteriores para comparar.
      {/if}
    </p>
  </section>
{:else if partida === null}
  <section class="secao">
    <h2>Sua última partida</h2>
    <p class="apoio">Ainda não há partida medida. Jogue alguns minutos com o Otimiza aberto: ele mede sozinho, sem nada por cima do jogo.</p>
  </section>
{/if}

<!-- Por que seu jogo roda assim -->
{#if diagnostico || erroDoDiagnostico}
  <section class="secao">
    <div class="topo">
      <h2>Por que seu jogo roda assim</h2>
      {#if diagnostico?.jogo}<span class="apoio">{diagnostico.jogo}</span>{/if}
    </div>
    {#if erroDoDiagnostico}
      <p class="apoio">Não consegui montar o diagnóstico agora: {erroDoDiagnostico}</p>
    {:else if diagnostico && diagnostico.causas.length === 0}
      <p class="apoio">
        {diagnostico.janelas_lidas > 0
          ? `Nas ${plural(diagnostico.janelas_lidas, "janela medida", "janelas medidas")}, nenhuma causa apareceu com clareza.`
          : "Sem partida medida ainda: o diagnóstico aparece depois da primeira."}
      </p>
    {:else if diagnostico}
      <ul class="causas">
        {#each diagnostico.causas as c (c.tipo)}
          {@const passos = passosDe(c.tipo)}
          <li>
            <div class="causa-topo">
              <strong>{c.confianca === "Hipotese" ? "Talvez: " : ""}{TITULO_DA_CAUSA[c.tipo]}</strong>
              <span class="selo" data-tom={c.confianca === "Alta" ? "atencao" : null}>{CONFIANCA[c.confianca]}</span>
            </div>
            <p class="apoio">{c.evidencia}</p>
            {#if !c.ajuda.length}
              <p class="pequeno">Não há o que fazer pelo FPS aqui.</p>
            {:else if passos}
              <ul class="passos">
                {#each passos as p (p.ajuste)}
                  {#if p.oferta.estado === "NaoOferecer"}
                    <li class="apoio">
                      <s>{NOME_DO_AJUSTE[p.ajuste]}</s>: já deixou este jogo pior neste PC em {data(p.oferta.quando)}, {p.oferta.contexto === "Igual"
                        ? "com o mesmo driver e Windows de agora"
                        : "e não deu para conferir se o driver ou o Windows mudou desde então"}. Não é oferecido de novo.
                    </li>
                  {:else}
                    <li>
                      <span class="ajuste">{NOME_DO_AJUSTE[p.ajuste]}</span><span class="apoio">{p.estrategia ? (p.aplicado ? " (já aplicado)" : ` ${ondeAplicar(p.estrategia)}`) : ""}: {COMO_PROVAR[p.como_provar]}. {lembranca(p)}</span>
                    </li>
                  {/if}
                {/each}
              </ul>
            {:else}
              <p class="pequeno">Ajuda: {c.ajuda.map((x) => NOME_DO_AJUSTE[x]).join(", ")}.</p>
            {/if}
            {#if c.nao_ajuda.length}<p class="apoio pequeno">Não ajuda: {c.nao_ajuda.map((x) => NOME_DO_AJUSTE[x]).join(", ")}.</p>{/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if lacunas.length}<p class="apoio pequeno">{lacunas.join(" ")}</p>{/if}
  </section>
{/if}

<!-- O que este PC permite -->
{#if perfil}
  <section class="secao">
    <h2>O que este PC permite</h2>
    <p class="apoio">
      {perfil.limitacoes.length === 0 && perfil.nao_lido.length === 0
        ? "Nada neste PC segura o jogo por si só. O que o Otimiza fizer vai ser medido nas suas partidas: ganho que não aparecer na medição não é contado."
        : perfil.limitacoes.length === 0
          ? "No que deu para ler, nada segura o jogo por si só; parte não foi lida (veja abaixo). O que o Otimiza fizer vai ser medido nas suas partidas."
          : "O que este hardware permite. O Otimiza não troca peça nenhuma; o que ele fizer vai ser medido nas suas partidas."}
    </p>
    {#if perfil.limitacoes.length}
      <ul class="limitacoes">
        {#each perfil.limitacoes as l (l.tipo)}
          {@const f = fraseDaLimitacao(l)}
          <li><strong>{f.forte}</strong> <span class="apoio">{f.resto}</span></li>
        {/each}
      </ul>
    {/if}
    {#if perfil.nao_lido.length}<p class="apoio pequeno">Não deu para ler: {perfil.nao_lido.join(", ")}.</p>{/if}
  </section>
{/if}

<style>
  .topo {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }
  .linhas {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 20px;
    margin: 0;
    font-size: 13px;
  }
  .linhas dt {
    color: var(--tinta-2);
  }
  .linhas dd {
    margin: 0;
  }
  .causas,
  .limitacoes,
  .passos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
  }
  .causas > li {
    display: grid;
    gap: 6px;
    padding: 12px 0;
    border-top: 1px solid var(--linha);
  }
  .causas > li:first-child {
    border-top: 0;
    padding-top: 0;
  }
  .causa-topo {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .passos {
    gap: 6px;
    padding-left: 14px;
    border-left: 1px solid var(--linha-forte);
    font-size: 13px;
  }
  .ajuste {
    font-weight: 500;
  }
  .limitacoes {
    gap: 10px;
    font-size: 13px;
  }
  p {
    margin: 0;
  }
  .pequeno {
    font-size: 12px;
  }
</style>
