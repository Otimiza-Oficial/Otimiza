<script lang="ts">
  // A casca da interface nova, no formato da referência do dono (painel de vidro estilo visionOS): barra de cima com
  // boas-vindas, a busca do Oti e Claro/Escuro; trilho de ícones à esquerda; uma grade de blocos de vidro no meio; abas
  // embaixo. Cada bloco mostra um dado lido deste PC ou liga uma coisa que existe de verdade; o que ainda não migrou
  // abre a área dizendo em que onda chega. Regras em `ui/DESIGN.md`.
  import { onMount, tick } from "svelte";
  import { cubicOut } from "svelte/easing";
  import Historico from "./componentes/Historico.svelte";
  import Icone, { type NomeDoIcone } from "./componentes/Icone.svelte";
  import Ajustes from "./componentes/Ajustes.svelte";
  import Mira from "./componentes/Mira.svelte";
  import Cores from "./componentes/Cores.svelte";
  import Energia from "./componentes/Energia.svelte";
  import Jogos from "./componentes/Jogos.svelte";
  import Prova from "./componentes/Prova.svelte";
  import Sistema from "./componentes/Sistema.svelte";
  import PedidoDeFeedback from "./componentes/PedidoDeFeedback.svelte";
  import Turbo from "./componentes/Turbo.svelte";
  import Latencia from "./componentes/Latencia.svelte";
  import Oti, { type Humor } from "./componentes/Oti.svelte";
  import EfeitoDeEpoca from "./componentes/EfeitoDeEpoca.svelte";
  import { porId, temporadaDe } from "./lib/temporadas";
  import Rede from "./componentes/Rede.svelte";
  import Tela from "./componentes/Tela.svelte";
  import { areas, type IdDaArea } from "./lib/areas";
  import { avisos as lerAvisos } from "./lib/clima";
  import { capaRemota, iniciais } from "./lib/capas";
  import { abertura, entender, jogoAbriu, jogoFechou, type Acao, type Fala } from "./lib/conversa";
  import {
    contaQueRoda,
    noTauri,
    ponte,
    type ContaQueRoda,
    type DiagnosticoDoWifi,
    type Hardware,
    type JogoAberto,
    type JogoDaLoja,
    type MetricasAoVivo,
    type PainelDaBanda,
    type PainelDaVarredura,
    type ResumoDoJogo,
    type TrocaComOti,
    type VersaoDoApp,
  } from "./lib/ponte";

  type Tema = "claro" | "escuro";
  const CHAVE_DO_TEMA = "otimiza.tema";

  // O trilho da esquerda e as abas de baixo: os mesmos destinos, como na referência.
  const TRILHO: { id: IdDaArea; icone: NomeDoIcone }[] = [
    { id: "inicio", icone: "casa" },
    { id: "turbo", icone: "raio" },
    { id: "jogos", icone: "controle" },
    { id: "rede", icone: "wifi" },
    { id: "latencia", icone: "cronometro" },
    { id: "tela", icone: "monitor" },
    { id: "historico", icone: "relogio" },
    { id: "ajustes", icone: "ajustes" },
  ];
  const ABAS: { id: IdDaArea; nome: string }[] = [
    { id: "inicio", nome: "Visão geral" },
    { id: "rede", nome: "Rede" },
    { id: "latencia", nome: "Latência" },
    { id: "tela", nome: "Tela" },
    { id: "historico", nome: "Histórico" },
  ];
  const OUTRAS: IdDaArea[] = ["turbo", "jogos", "energia", "mira", "cores", "sistema", "prova", "ajustes"];

  let versao = $state<VersaoDoApp | null>(null);
  let hardware = $state<Hardware | null>(null);
  let metricas = $state<MetricasAoVivo | null>(null);
  let jogo = $state<JogoAberto | null>(null);
  let biblioteca = $state<ResumoDoJogo[]>([]);
  let instalados = $state<{ jogo: JogoDaLoja; capa: string | null }[]>([]);
  let banda = $state<PainelDaBanda | null>(null);
  let varredura = $state<PainelDaVarredura | null>(null);
  let wifi = $state<DiagnosticoDoWifi | null>(null);
  let mudancas = $state<number | null>(null);
  let tema = $state<Tema>(lerTema());
  let aberta = $state<IdDaArea>("inicio");
  /** O "Ver o sistema" do bloco Agora abre o Sistema já na aba Agora; o resto entra pelos Ajustes. */
  let abaDoSistema = $state<"agora" | "ajustes">("ajustes");
  let busca = $state("");
  let filtroDeJogo = $state("");
  let mostrarAvisos = $state(false);
  let mostrarMais = $state(false);
  let medindoDpc = $state(false);
  let recadoDpc = $state<string | null>(null);

  // O Oti fala num balão que desce da busca.
  let fala = $state<Fala | null>(null);
  let digitado = $state("");
  let digitando = $state(false);
  let reacao = $state<Humor | null>(null);

  let jogoNaTela = $state(false);

  // A época do ano (Natal, Páscoa…): muda sozinha pela data; desliga nos Ajustes ("otimiza.epoca" = "desligada").
  // `?temporada=natal` na URL mostra uma época fora da data, para conferir o visual.
  const CHAVE_DA_EPOCA = "otimiza.epoca";
  const previa = porId(new URLSearchParams(location.search).get("temporada"));
  let hoje = $state(new Date());
  let epocaLigada = $state(lerEpoca());
  const temporada = $derived(epocaLigada ? (previa ?? temporadaDe(hoje)) : null);

  function lerEpoca() {
    try {
      return localStorage.getItem("otimiza.epoca") !== "desligada";
    } catch {
      return true;
    }
  }

  let conta = $state<ContaQueRoda | null>(null);
  /** Os avisos do PC e, por cima, o da conta: aberto como outro usuário, metade dos ajustes cairia no perfil errado. */
  const lista = $derived([
    ...(conta?.conta.estado === "Diferente"
      ? [{ id: "conta", titulo: "O Otimiza está aberto por outra conta do Windows", detalhe: conta.explicacao.split(/\n\n/)[0], gravidade: "serio" as const }]
      : []),
    ...lerAvisos(hardware, metricas, jogo !== null),
  ]);
  const area = $derived(areas.find((a) => a.id === aberta) ?? areas[0]);
  const humor = $derived<Humor>(reacao ?? (jogo ? "jogando" : digitando ? "falando" : (fala?.humor ?? "atento")));
  // O ícone da bandeja acompanha o humor; 1,5 s parado antes de trocar (falando/pensando passam rápido).
  $effect(() => {
    const h = humor;
    const t = setTimeout(() => void ponte.bandejaHumor(h), 1500);
    return () => clearTimeout(t);
  });
  const ultima = $derived(
    biblioteca.filter((j) => j.ultima).sort((a, b) => (b.ultima?.quando ?? 0) - (a.ultima?.quando ?? 0))[0] ?? null
  );
  /** Capas que não carregaram (sem internet, arte retirada): esses jogos ficam com a capa desenhada. */
  let semFoto = $state(new Set<string>());
  /** O "player": o jogo aberto, se houver; senão a última partida. Os números só aparecem se forem DESSE jogo. */
  const jogoDoPlayer = $derived(jogo?.nome ?? ultima?.nome ?? null);
  const partidaDoPlayer = $derived(ultima && ultima.nome === jogoDoPlayer ? ultima : null);
  const capaDaUltima = $derived(
    jogoDoPlayer
      ? (instalados.find((i) => i.jogo.nome === jogoDoPlayer)?.capa ?? (semFoto.has(jogoDoPlayer) ? null : capaRemota(jogoDoPlayer, null)))
      : null
  );
  function fotoFalhou(nome: string) {
    semFoto = new Set(semFoto).add(nome);
    instalados = instalados.map((i) => (i.jogo.nome === nome ? { ...i, capa: null } : i));
  }
  const telaPrincipal = $derived(hardware?.telas.find((t) => t.principal) ?? hardware?.telas[0] ?? null);
  const memoriaPct = $derived.by(() => {
    const t = metricas?.ram.total_gb ?? null;
    const u = metricas?.ram.used_gb ?? null;
    return t != null && u != null && t > 0 ? (u / t) * 100 : null;
  });
  const sinalWifi = $derived.by(() => {
    const l = wifi?.ligacao;
    if (l?.tipo !== "wi_fi") return null;
    const c = l.leitura.estado === "lida" ? l.leitura.conexao : l.leitura.estado === "lista_vazia" ? l.leitura.conexao : null;
    return c ? c.qualidade : null;
  });
  /** O bloco "Agora": quatro leituras, cada uma com nome e valor por extenso. Barra só fica em cor quando há o que fazer. */
  const medidores = $derived.by(() => {
    const t = telaPrincipal;
    const ramTotal = metricas?.ram.total_gb ?? null;
    return [
      { nome: "Processador", texto: pct(metricas?.cpu.overall), fracao: metricas?.cpu.overall == null ? null : metricas.cpu.overall / 100, alerta: false },
      {
        nome: "Memória",
        texto: memoriaPct == null ? "—" : `${pct(memoriaPct)} de ${Math.round(ramTotal ?? 0)} GB`,
        fracao: memoriaPct == null ? null : memoriaPct / 100,
        alerta: (memoriaPct ?? 0) >= 85,
      },
      {
        nome: "Tela",
        texto: t ? `${t.hz_atual} de ${t.hz_maximo} Hz` : "—",
        fracao: t ? t.hz_atual / Math.max(1, t.hz_maximo) : null,
        alerta: !!t && t.hz_atual < t.hz_maximo,
      },
      {
        nome: "Wi-Fi",
        texto: wifi?.ligacao.tipo === "cabo" ? "no cabo" : sinalWifi == null ? "—" : `sinal ${pct(sinalWifi)}`,
        fracao: wifi?.ligacao.tipo === "cabo" ? 1 : sinalWifi == null ? null : sinalWifi / 100,
        alerta: sinalWifi != null && sinalWifi < 40,
      },
    ];
  });
  const jogosFiltrados = $derived(
    instalados.filter((i) => i.jogo.nome.toLowerCase().includes(filtroDeJogo.trim().toLowerCase())).slice(0, 3)
  );

  function lerTema(): Tema {
    try {
      return localStorage.getItem(CHAVE_DO_TEMA) === "escuro" ? "escuro" : "claro";
    } catch {
      return "claro";
    }
  }

  function escolherTema(t: Tema) {
    tema = t;
    try {
      localStorage.setItem(CHAVE_DO_TEMA, t);
    } catch {
      // Só não lembra na próxima abertura.
    }
  }

  $effect(() => {
    document.documentElement.classList.toggle("jogo-aberto", jogoNaTela);
    document.documentElement.dataset.tema = tema === "escuro" ? "noite" : "dia";
  });

  // ---------------------------------------------------------------------------------------------------------------
  // O Oti falando

  let maquina: ReturnType<typeof setInterval> | undefined;
  let fecharFala: ReturnType<typeof setTimeout> | undefined;
  function dizer(f: Fala, some = 0) {
    fala = f;
    clearInterval(maquina);
    clearTimeout(fecharFala);
    if (jogoNaTela || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      digitado = f.texto;
      digitando = false;
    } else {
      digitado = "";
      digitando = true;
      let i = 0;
      maquina = setInterval(() => {
        i += 1;
        digitado = f.texto.slice(0, i);
        if (i >= f.texto.length) {
          clearInterval(maquina);
          digitando = false;
        }
      }, 22);
    }
    if (some) fecharFala = setTimeout(() => (fala = null), some);
  }

  function reagir(h: Humor, ms: number) {
    reacao = h;
    setTimeout(() => (reacao = null), ms);
  }

  function executar(a: Acao) {
    if (a.tipo === "abrir") {
      fala = null;
      abrir(a.area);
    } else dizer(a.fala());
  }

  // O Oti com IA: liga nos Ajustes ("otimiza.ia" = "ligada"). Só entra quando as respostas prontas não entendem.
  let conversaComIa = $state<TrocaComOti[]>([]);

  function iaLigada() {
    try {
      return localStorage.getItem("otimiza.ia") === "ligada";
    } catch {
      return false;
    }
  }

  /** Só fatos: números medidos, nomes de peças e avisos. Nada de arquivo, usuário ou caminho de disco. */
  function fatosDoPc() {
    return {
      processador: hardware ? nomeCurto(hardware.processador) : null,
      placa: hardware ? nomeCurto(hardware.placa.nome) : null,
      memoria_gb: hardware?.memoria_gb ?? null,
      tela: telaPrincipal ? { hz: telaPrincipal.hz_atual, aguenta: telaPrincipal.hz_maximo } : null,
      uso_agora: jogo ? null : { processador_pct: metricas?.cpu.overall ?? null, memoria_pct: memoriaPct },
      jogo_aberto: jogo?.nome ?? null,
      ultima_partida: partidaDoPlayer?.ultima
        ? {
            jogo: partidaDoPlayer.nome,
            fps: partidaDoPlayer.ultima.fps_nativo,
            um_por_cento_pior: partidaDoPlayer.ultima.low_1pct,
            ganho_medido_pct: partidaDoPlayer.ganho_low.estado === "Medido" ? partidaDoPlayer.ganho_low.pct : null,
          }
        : null,
      avisos: lista.map((a) => a.titulo),
    };
  }

  async function perguntarComIa(texto: string) {
    reacao = "pensando";
    dizer({ texto: "Deixa eu olhar o seu PC…", humor: "pensando", acoes: [] });
    try {
      const r = await ponte.perguntarAoOti(texto, fatosDoPc(), conversaComIa);
      conversaComIa = [...conversaComIa, { papel: "pessoa", texto }, { papel: "oti", texto: r.resposta }].slice(-6);
      reacao = null;
      dizer({
        texto: r.resposta,
        detalhe: r.restantes != null && r.restantes <= 3 ? `Restam ${r.restantes} perguntas para a IA hoje.` : undefined,
        humor: (["atento", "feliz", "pensando", "preocupado"].includes(r.humor) ? r.humor : "atento") as Humor,
        acoes: r.acoes
          .filter((a) => areas.some((x) => x.id === a.area))
          .map((a, i) => ({ rotulo: a.rotulo, tipo: "abrir" as const, area: a.area as IdDaArea, principal: i === 0 })),
      });
    } catch (e) {
      reacao = null;
      dizer({ texto: String(e), humor: "preocupado", acoes: [] });
    }
  }

  function perguntar(e: SubmitEvent) {
    e.preventDefault();
    const texto = busca.trim();
    if (!texto) return;
    busca = "";
    const r = entender(texto);
    if (!r.area && iaLigada()) {
      void perguntarComIa(texto);
      return;
    }
    reagir("pensando", 600);
    setTimeout(() => {
      dizer(r.fala, r.area ? 2200 : 0);
      if (r.area) setTimeout(() => abrir(r.area!), 650);
    }, 600);
  }

  // A primeira fala espera a ficha do PC e a primeira leitura de memória: antes disso ela afirmaria sem ter lido.
  let primeiraFala = false;
  $effect(() => {
    if (primeiraFala || !hardware || (!metricas && !jogo)) return;
    primeiraFala = true;
    if (!jogo && lista.length > 0) dizer(abertura(lista));
    else if (!jogo) reagir("feliz", 1300);
  });

  // Jogo abriu: o Oti avisa e só então a interface congela. Na primeira leitura não há cena.
  let jogoAnterior: string | null | undefined = undefined;
  $effect(() => {
    const nome = jogo?.nome ?? null;
    if (jogoAnterior === undefined) {
      jogoAnterior = nome;
      jogoNaTela = nome !== null;
      return;
    }
    if (nome === jogoAnterior) return;
    const antes = jogoAnterior;
    jogoAnterior = nome;
    if (nome && !antes) {
      dizer(jogoAbriu(nome), 2600);
      setTimeout(() => (jogoNaTela = true), 2800);
    } else if (!nome && antes) {
      jogoNaTela = false;
      setTimeout(() => dizer(jogoFechou(antes)), 400);
    }
  });

  // ---------------------------------------------------------------------------------------------------------------
  // Navegação, abas e blocos

  function abrir(id: IdDaArea, abaInterna: "agora" | "ajustes" = "ajustes") {
    abaDoSistema = abaInterna;
    // Navegar fecha o balão, a não ser que a navegação tenha vindo da pergunta (ela some sozinha depois).
    if (!digitando) fala = null;
    aberta = id;
    mostrarMais = false;
    mostrarAvisos = false;
  }

  // A pílula branca das abas desliza até a aba ativa.
  let abasEl: HTMLElement | undefined = $state();
  let pilula = $state({ x: 0, w: 0 });
  async function medirPilula() {
    await tick();
    const alvo = abasEl?.querySelector<HTMLElement>(".aba.ativa");
    if (!alvo || !abasEl) {
      pilula = { x: pilula.x, w: 0 };
      return;
    }
    pilula = { x: alvo.offsetLeft, w: alvo.offsetWidth };
  }
  $effect(() => {
    void aberta;
    void medirPilula();
  });

  /** A luz do vidro segue o cursor, como no visionOS. */
  function brilho(no: HTMLElement) {
    const mover = (e: PointerEvent) => {
      const r = no.getBoundingClientRect();
      no.style.setProperty("--mx", `${e.clientX - r.left}px`);
      no.style.setProperty("--my", `${e.clientY - r.top}px`);
    };
    no.addEventListener("pointermove", mover);
    return { destroy: () => no.removeEventListener("pointermove", mover) };
  }

  function entrar(_no: HTMLElement) {
    return {
      duration: 420,
      easing: cubicOut,
      css: (t: number, u: number) => `opacity: ${t}; transform: scale(${0.97 + 0.03 * t}); filter: blur(${u * 8}px)`,
    };
  }

  async function alternarBanda() {
    if (!banda) return;
    try {
      await ponte.definirBanda(!banda.ligado);
      banda = await ponte.banda();
    } catch {
      banda = await ponte.banda().catch(() => banda);
    }
  }

  async function alternarVarredura() {
    if (!varredura) return;
    varredura = await ponte.definirVarredura(!varredura.ligado).catch(() => varredura);
  }

  async function medirDpc() {
    if (medindoDpc) {
      await ponte.cancelarDpc().catch(() => {});
      return;
    }
    medindoDpc = true;
    recadoDpc = "Medindo 10 s…";
    const r = await ponte.capturarDpc(10).catch((e) => ({ estado: "Falhou" as const, etapa: "", codigo: 0, texto: String(e) }));
    medindoDpc = false;
    recadoDpc = r.estado === "Medido" ? r.relatorio.veredito.texto : r.texto;
  }

  // ---------------------------------------------------------------------------------------------------------------

  function pct(v: number | null | undefined) {
    return v == null ? "—" : `${Math.round(v)}%`;
  }

  function inteiro(v: number | null | undefined) {
    return v == null ? "—" : String(Math.round(v));
  }

  /** Onde cada aviso se resolve. */
  function areaDoAviso(id: string): IdDaArea {
    if (id.startsWith("hz-")) return "tela";
    if (id === "memoria") return "sistema";
    if (id === "conta") return "ajustes";
    return "inicio";
  }

  function nomeCurto(nome: string) {
    return nome.replace(/\((R|TM)\)/gi, "").replace(/\s+CPU\s*@.*$/i, "").replace(/^NVIDIA\s+/i, "").replace(/\s+/g, " ").trim();
  }

  function trocouHz(dispositivo: string, hz: number) {
    if (!hardware) return;
    hardware = {
      ...hardware,
      telas: hardware.telas.map((t) => (t.dispositivo.toLowerCase() === dispositivo.toLowerCase() ? { ...t, hz_atual: hz } : t)),
    };
  }

  async function janela(acao: "minimizar" | "maximizar" | "fechar") {
    if (!noTauri) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const w = getCurrentWindow();
    if (acao === "minimizar") await w.minimize();
    else if (acao === "maximizar") await w.toggleMaximize();
    else await w.close();
  }

  async function alternarJogoDeDemonstracao() {
    ponte.simularJogo(jogo ? null : "FiveM");
    jogo = await ponte.jogoAberto();
  }

  onMount(() => {
    let vivo = true;
    let espera: ReturnType<typeof setTimeout> | undefined;

    void (async () => {
      versao = await ponte.versao().catch(() => null);
      hardware = await ponte.hardwareUltimo().catch(() => null);
      ponte.hardware().then((h) => (hardware = h)).catch(() => {});
      ponte.biblioteca().then((b) => (biblioteca = b)).catch(() => {});
      ponte.banda().then((b) => (banda = b)).catch(() => {});
      ponte.varredura().then((v) => (varredura = v)).catch(() => {});
      ponte.wifi().then((w) => (wifi = w)).catch(() => {});
      contaQueRoda().then((c) => (conta = c));
      ponte.historico().then((h) => (mudancas = h.mudancas.length)).catch(() => {});
      ponte
        .instalados()
        .then(async (l) => {
          const recentes = [...l].sort((a, b) => b.ultima_vez - a.ultima_vez).slice(0, 8);
          // Primeiro a arte pela internet (aparece na hora); depois, se houver, a do cache da Steam deste PC.
          instalados = recentes.map((j) => ({ jogo: j, capa: capaRemota(j.nome, j.appid) }));
          for (const [i, j] of recentes.entries()) {
            if (j.appid == null) continue;
            const capa = await ponte.capa(j.appid).catch(() => null);
            if (vivo && capa) instalados[i] = { jogo: j, capa };
          }
        })
        .catch(() => {});
      await ponte.iniciarMonitor().catch(() => {});
    })();

    const laco = async () => {
      if (!vivo) return;
      jogo = await ponte.jogoAberto().catch(() => null);
      if (!jogo && !document.hidden) metricas = await ponte.metricas().catch(() => metricas);
      espera = setTimeout(laco, jogo ? 5000 : document.hidden ? 4000 : 2000);
    };
    void laco();

    // A data muda à meia-noite (e a época junto); conferir de hora em hora basta.
    const relogioDaEpoca = setInterval(() => (hoje = new Date()), 60 * 60 * 1000);
    const aoMudarEpoca = () => (epocaLigada = lerEpoca());
    window.addEventListener(CHAVE_DA_EPOCA, aoMudarEpoca);

    // Guarda térmica: o Oti avisa quando o PC começa a ser freado por calor no jogo.
    let deixarCalor: (() => void) | undefined;
    void ponte
      .aoAvisoTermico((a) => {
        dizer(
          {
            texto: a.frase,
            humor: "preocupado",
            acoes: [{ rotulo: "Ver energia", tipo: "abrir", area: "energia", principal: true }],
          },
          30000,
        );
      })
      .then((d) => {
        if (vivo) deixarCalor = d;
        else d();
      });

    const ajustar = () => void medirPilula();
    window.addEventListener("resize", ajustar);
    return () => {
      vivo = false;
      deixarCalor?.();
      clearTimeout(espera);
      clearInterval(maquina);
      window.removeEventListener("resize", ajustar);
      window.removeEventListener(CHAVE_DA_EPOCA, aoMudarEpoca);
      clearInterval(relogioDaEpoca);
    };
  });
</script>

<!-- O cômodo desfocado atrás do vidro. Feito em CSS: parede, janela com luz, chão, sofá e luminária. -->
<div class="comodo" data-epoca={temporada?.id ?? null} aria-hidden="true">
  <div class="parede"></div>
  <div class="janela-luz"></div>
  <div class="chao"></div>
  <div class="sofa"></div>
  <div class="luminaria"><span></span></div>
  <div class="mesa"></div>
</div>

{#if temporada}
  <EfeitoDeEpoca efeito={temporada.efeito} />
{/if}

<div class="casca">
  <!-- Barra de cima -->
  <header class="topo vidro" data-tauri-drag-region>
    <div class="boas-vindas" data-tauri-drag-region>
      <span class="avatar"><Oti acessorio={temporada?.acessorio ?? null} humor={humor} tamanho={30} segueCursor={false} /></span>
      <div class="ola">
        <strong>{temporada?.saudacao ?? "Bem-vindo de volta"}</strong>
        <span class="sec">Otimiza {versao?.exibida ?? ""}{hardware ? ` · ${nomeCurto(hardware.processador)}` : ""}</span>
      </div>
    </div>

    <form class="busca" onsubmit={perguntar}>
      <span class="orbe"><Oti {humor} tamanho={22} /></span>
      <input bind:value={busca} placeholder="Pergunte ao Oti… por que meu ping pula?" aria-label="Pergunte ao Oti" />
    </form>

    <div class="direita" data-tauri-drag-region>
      {#if !noTauri}
        <button class="link-demo" onclick={alternarJogoDeDemonstracao}>{jogo ? "Fechar jogo de teste" : "Simular jogo"}</button>
      {/if}
      <div class="segmento" role="radiogroup" aria-label="Tema">
        <button role="radio" aria-checked={tema === "claro"} onclick={() => escolherTema("claro")}>
          <Icone nome="sol" tamanho={14} /> Claro
        </button>
        <button role="radio" aria-checked={tema === "escuro"} onclick={() => escolherTema("escuro")}>
          <Icone nome="lua" tamanho={14} /> Escuro
        </button>
      </div>
      <button class="redondo" aria-label="Avisos" onclick={() => (mostrarAvisos = !mostrarAvisos)}>
        <Icone nome="sino" tamanho={18} />
        {#if lista.length > 0}<span class="contador">{lista.length}</span>{/if}
      </button>
      <div class="controles">
        <button aria-label="Minimizar" onclick={() => janela("minimizar")}><svg viewBox="0 0 12 12" width="10" height="10"><path d="M2 6h8" /></svg></button>
        <button aria-label="Maximizar" onclick={() => janela("maximizar")}><svg viewBox="0 0 12 12" width="10" height="10"><rect x="2" y="2" width="8" height="8" rx="1.5" /></svg></button>
        <button aria-label="Fechar" class="x" onclick={() => janela("fechar")}><svg viewBox="0 0 12 12" width="10" height="10"><path d="M3 3l6 6M9 3l-6 6" /></svg></button>
      </div>
    </div>
  </header>

  <!-- O balão do Oti -->
  {#if fala}
    <div class="balao vidro-forte" role="status" in:entrar>
      <span class="balao-oti"><Oti acessorio={temporada?.acessorio ?? null} {humor} tamanho={44} /></span>
      <div class="balao-texto">
        <p class="balao-fala">{digitado}</p>
        {#if fala.detalhe && !digitando}<p class="balao-detalhe">{fala.detalhe}</p>{/if}
        {#if fala.acoes.length > 0 && !digitando}
          <div class="balao-acoes">
            {#each fala.acoes.slice(0, 4) as a (a.rotulo)}
              <button class="pilula" class:principal={a.principal} onclick={() => executar(a)}>{a.rotulo}</button>
            {/each}
          </div>
        {/if}
      </div>
      <button class="fechar-balao" aria-label="Fechar" onclick={() => (fala = null)}>
        <svg viewBox="0 0 12 12" width="10" height="10"><path d="M3 3l6 6M9 3l-6 6" /></svg>
      </button>
    </div>
  {/if}

  {#if mostrarAvisos}
    <div class="avisos vidro-forte" in:entrar>
      <h3>Avisos</h3>
      {#if lista.length === 0}
        <p class="sec">Nada atrapalhando no que eu sei olhar.</p>
      {:else}
        {#each lista as a (a.id)}
          <button class="aviso" onclick={() => abrir(areaDoAviso(a.id))}>
            <span class="ponto" data-tom={a.gravidade === "serio" ? "perigo" : "atencao"}></span>
            <span><strong>{a.titulo}</strong><br /><span class="sec">{a.detalhe}</span></span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Trilho -->
  <nav class="trilho vidro" aria-label="Áreas">
    {#each TRILHO as t (t.id)}
      <button
        class="trilho-botao"
        class:ativo={aberta === t.id}
        aria-label={areas.find((a) => a.id === t.id)?.nome}
        title={areas.find((a) => a.id === t.id)?.nome}
        onclick={() => abrir(t.id)}
      >
        <Icone nome={t.icone} tamanho={20} />
      </button>
    {/each}
    <button class="trilho-botao" aria-label="Mais áreas" title="Mais áreas" aria-expanded={mostrarMais} onclick={() => (mostrarMais = !mostrarMais)}>
      <Icone nome="grade" tamanho={20} />
    </button>
  </nav>

  <!-- Painel -->
  <main class="painel vidro">
    {#key aberta}
      <div class="conteudo" in:entrar>
        {#if aberta === "inicio"}
          <div class="grade">
            <!-- Seus jogos: busca e as três capas mais recentes -->
            <section class="bloco b-jogos" use:brilho>
              <label class="campo-busca">
                <Icone nome="busca" tamanho={15} />
                <input bind:value={filtroDeJogo} placeholder="Buscar nos seus jogos" aria-label="Buscar nos seus jogos" />
              </label>
              <div class="tres">
                {#each jogosFiltrados as { jogo: j, capa } (j.nome)}
                  <button class="capa-jogo" onclick={() => abrir("jogos")} title={j.nome}>
                    {#if capa}<img src={capa} alt="" onerror={() => fotoFalhou(j.nome)} />{:else}<span class="ini">{iniciais(j.nome)}</span>{/if}
                    <span class="capa-nome">{j.nome}</span>
                  </button>
                {:else}
                  <p class="sec vazio">{instalados.length ? "Nenhum jogo com esse nome." : "Lendo seus jogos…"}</p>
                {/each}
              </div>
            </section>

            <!-- A última partida: só número medido, com nome -->
            <section class="bloco b-partida" use:brilho>
              <h3>{jogo ? "Jogando agora" : "Última partida"}</h3>
              <div class="faixa">
                <span class="capa">
                  {#if capaDaUltima}<img src={capaDaUltima} alt="" onerror={() => jogoDoPlayer && fotoFalhou(jogoDoPlayer)} />{:else if jogoDoPlayer}<span class="ini">{iniciais(jogoDoPlayer)}</span>{/if}
                </span>
                <strong class="faixa-nome">{jogoDoPlayer ?? "Nenhuma partida ainda"}</strong>
              </div>
              {#if partidaDoPlayer?.ultima}
                <dl class="numeros">
                  <div><dt>FPS médio</dt><dd class="numero">{inteiro(partidaDoPlayer.ultima.fps_nativo)}</dd></div>
                  <div><dt>1% pior</dt><dd class="numero">{inteiro(partidaDoPlayer.ultima.low_1pct)}</dd></div>
                </dl>
                {#if partidaDoPlayer.ganho_low.estado === "Medido" && partidaDoPlayer.ganho_low.pct != null}
                  <p class="sec pequeno"><span class="selo-medido">Medido</span> {partidaDoPlayer.ganho_low.pct >= 0 ? "+" : ""}{inteiro(partidaDoPlayer.ganho_low.pct)}% no 1% pior desde que otimizou</p>
                {/if}
              {:else}
                <p class="sec pequeno">Jogue com o Otimiza aberto: ele mede o FPS sozinho, sem nada por cima do jogo.</p>
              {/if}
              <button class="link" onclick={() => abrir("jogos")}>Ver partidas <Icone nome="seta_direita" tamanho={14} /></button>
            </section>

            <!-- Otimizar: a ação principal da tela e os atalhos que respondem uma pergunta -->
            <section class="bloco b-acoes" use:brilho>
              <h3>Otimizar</h3>
              <p class="sec pequeno">Ajustes conferidos neste PC. Cada um fica no Histórico com o valor de antes.</p>
              <button class="botao-principal" onclick={() => abrir("turbo")}>Otimizar agora</button>
              <ul class="atalhos">
                <li><button onclick={() => abrir("rede")}>Medir a rota até o jogo<Icone nome="seta_direita" tamanho={14} /></button></li>
                <li><button onclick={() => abrir("latencia")}>Achar o driver que trava<Icone nome="seta_direita" tamanho={14} /></button></li>
                <li><button onclick={() => abrir("prova")}>Provar antes e depois<Icone nome="seta_direita" tamanho={14} /></button></li>
                <li>
                  <button onclick={() => abrir("historico")}>
                    <span>O que foi mudado{mudancas ? ` (${mudancas})` : ""}</span><Icone nome="seta_direita" tamanho={14} />
                  </button>
                </li>
              </ul>
              {#if mudancas}
                <button class="link desfazer" onclick={() => abrir("historico")}><Icone nome="desfazer" tamanho={14} /> Desfazer tudo</button>
              {/if}
            </section>

            <!-- Agora: os quatro números do PC, cada um com nome -->
            <section class="bloco b-agora" use:brilho>
              <h3>Agora</h3>
              <ul class="medidores">
                {#each medidores as m (m.nome)}
                  <li>
                    <div class="medidor-linha">
                      <span>{m.nome}</span>
                      <span class="numero">{m.texto}</span>
                    </div>
                    <div class="trilha" class:alerta={m.alerta}><div style:width={`${m.fracao == null ? 0 : Math.max(2, Math.min(100, m.fracao * 100))}%`}></div></div>
                  </li>
                {/each}
              </ul>
              {#if jogo}<p class="sec pequeno">Leitura pausada com o jogo aberto.</p>{/if}
              <button class="link" onclick={() => abrir("sistema", "agora")}>Ver quem está pesando <Icone nome="seta_direita" tamanho={14} /></button>
            </section>

            <!-- Durante o jogo: o que liga sozinho quando um jogo abre -->
            <section class="bloco b-fundo" use:brilho>
              <h3>Durante o jogo</h3>
              <ul class="chaves">
                <li>
                  <div>
                    <span>Segurar downloads de fundo</span>
                    <span class="sec pequeno">
                      {banda?.agindo_em ? `Segurando durante ${banda.agindo_em}` : banda?.impedimento ? "Precisa de administrador" : "Nuvem, torrent, lojas e Windows Update"}
                    </span>
                  </div>
                  <button
                    class="chave"
                    role="switch"
                    aria-checked={banda?.ligado ?? false}
                    aria-label="Segurar downloads de fundo durante o jogo"
                    disabled={!banda || (!banda.ligado && banda.impedimento !== null)}
                    title={banda?.impedimento ?? undefined}
                    onclick={alternarBanda}
                  ></button>
                </li>
                <li>
                  <div>
                    <span>Wi-Fi sem procurar redes</span>
                    <span class="sec pequeno">{varredura?.agindo_em ? `Agindo em ${varredura.agindo_em}` : "Tira os picos de ping que vêm a cada minuto"}</span>
                  </div>
                  <button class="chave" role="switch" aria-checked={varredura?.ligado ?? false} aria-label="Wi-Fi sem procurar redes durante o jogo" disabled={!varredura} onclick={alternarVarredura}></button>
                </li>
                <li>
                  <div>
                    <span>Driver que trava</span>
                    <span class="sec pequeno">{recadoDpc ?? "Mede 10 s e aponta o culpado. Não muda nada."}</span>
                  </div>
                  <button class="botao-pequeno" onclick={medirDpc}>{medindoDpc ? "Parar" : "Medir"}</button>
                </li>
              </ul>
            </section>

            <!-- Avisos: cada um leva para onde se resolve -->
            <section class="bloco b-avisos" use:brilho>
              <h3>Avisos</h3>
              {#if lista.length === 0}
                <p class="aviso-linha sec"><span class="ponto" data-tom="ok"></span>Nada atrapalhando no que eu sei olhar.</p>
              {:else}
                <ul class="lembretes">
                  {#each lista.slice(0, 3) as a (a.id)}
                    <li>
                      <button class="aviso-linha" onclick={() => abrir(areaDoAviso(a.id))}>
                        <span class="ponto" data-tom={a.gravidade === "serio" ? "perigo" : "atencao"}></span>
                        <span>{a.titulo}</span>
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </section>

            <!-- Tela: a taxa em uso contra a que o monitor aguenta -->
            <section class="bloco b-tela" use:brilho>
              <h3>Tela</h3>
              <span class="numero gigante">{telaPrincipal ? telaPrincipal.hz_atual : "—"}<small>Hz</small></span>
              <p class="sec pequeno">
                {#if !telaPrincipal}
                  Lendo o monitor…
                {:else if telaPrincipal.hz_atual < telaPrincipal.hz_maximo}
                  {telaPrincipal.nome} aguenta {telaPrincipal.hz_maximo} Hz.
                {:else}
                  {telaPrincipal.nome} no máximo, {telaPrincipal.largura}×{telaPrincipal.altura}.
                {/if}
              </p>
              {#if telaPrincipal && telaPrincipal.hz_atual < telaPrincipal.hz_maximo}
                <button class="botao-pequeno escuro" onclick={() => abrir("tela")}>Usar {telaPrincipal.hz_maximo} Hz</button>
              {:else}
                <button class="link" onclick={() => abrir("tela")}>Ver a tela <Icone nome="seta_direita" tamanho={14} /></button>
              {/if}
            </section>
          </div>
        {:else}
          <div class="area-aberta">
            <header class="area-topo">
              <div>
                <h2>{area.nome}</h2>
                {#if area.pergunta}<p class="sec">{area.pergunta}</p>{/if}
              </div>
            </header>
            <div class="area-corpo">
              {#if aberta === "historico"}
                <Historico />
              {:else if aberta === "rede"}
                <Rede />
              {:else if aberta === "latencia"}
                <Latencia />
              {:else if aberta === "tela"}
                <Tela aoMudarHz={trocouHz} />
              {:else if aberta === "jogos"}
                <Jogos />
              {:else if aberta === "ajustes"}
                <Ajustes />
              {:else if aberta === "mira"}
                <Mira />
              {:else if aberta === "cores"}
                <Cores />
              {:else if aberta === "prova"}
                <Prova jogo={jogo?.nome ?? null} />
              {:else if aberta === "sistema"}
                <Sistema abaInicial={abaDoSistema} />
              {:else if aberta === "energia"}
                <Energia />
              {:else if aberta === "turbo"}
                <Turbo jogo={jogo?.nome ?? null} />
              {:else}
                <div class="em-breve">
                  <Oti acessorio={temporada?.acessorio ?? null} humor="dormindo" tamanho={90} segueCursor={false} />
                  <p>{area.nome} chega na Onda {area.onda}. Até lá, ela continua no Otimiza de sempre.</p>
                  <ul class="lista">
                    {#each area.vai_ter as item (item)}<li>{item}</li>{/each}
                  </ul>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/key}
  </main>

  {#if mostrarMais}
    <div class="mais vidro-forte" in:entrar>
      {#each OUTRAS as id (id)}
        <button onclick={() => abrir(id)}>{areas.find((a) => a.id === id)?.nome}</button>
      {/each}
    </div>
  {/if}

  <!-- Abas -->
  <nav class="abas vidro" aria-label="Seções" bind:this={abasEl}>
    <span class="pilula-ativa" style:transform={`translateX(${pilula.x}px)`} style:width={`${pilula.w}px`} style:opacity={pilula.w ? 1 : 0}></span>
    {#each ABAS as a (a.id)}
      <button class="aba" class:ativa={aberta === a.id} onclick={() => abrir(a.id)}>{a.nome}</button>
    {/each}
    <button class="aba-mais" aria-label="Mais áreas" onclick={() => (mostrarMais = !mostrarMais)}><Icone nome="mais" tamanho={16} /></button>
  </nav>

  <PedidoDeFeedback />

  {#if jogoNaTela && jogo}
    <p class="pausado">{jogo.nome} aberto · interface pausada para não pesar no jogo</p>
  {/if}
</div>

<style>
  /* ---------- O cômodo atrás do vidro ---------- */
  .comodo {
    position: fixed;
    inset: -40px;
    z-index: 0;
    filter: blur(16px) saturate(1.05);
    background: #c9c3bb;
  }
  /* A cor da época no cômodo, por baixo do desfoque: luz ambiente, não papel de parede. */
  .comodo::after {
    content: "";
    position: absolute;
    inset: 0;
    opacity: 0;
    transition: opacity 1.2s ease;
  }
  .comodo[data-epoca]::after {
    opacity: 1;
  }
  .comodo[data-epoca="natal"]::after {
    background:
      radial-gradient(ellipse 40% 50% at 12% 70%, rgb(214 47 58 / 0.35), transparent 70%),
      radial-gradient(ellipse 35% 45% at 88% 30%, rgb(34 139 84 / 0.3), transparent 70%),
      radial-gradient(ellipse 30% 30% at 60% 85%, rgb(255 210 120 / 0.35), transparent 70%);
  }
  .comodo[data-epoca="ano-novo"]::after {
    background:
      radial-gradient(ellipse 45% 50% at 80% 25%, rgb(230 180 34 / 0.35), transparent 70%),
      radial-gradient(ellipse 40% 40% at 20% 80%, rgb(255 255 255 / 0.35), transparent 70%);
  }
  .comodo[data-epoca="carnaval"]::after {
    background:
      radial-gradient(ellipse 40% 45% at 15% 30%, rgb(122 43 214 / 0.3), transparent 70%),
      radial-gradient(ellipse 40% 45% at 85% 70%, rgb(255 79 154 / 0.3), transparent 70%),
      radial-gradient(ellipse 30% 35% at 55% 20%, rgb(46 196 182 / 0.25), transparent 70%);
  }
  .comodo[data-epoca="pascoa"]::after {
    background:
      radial-gradient(ellipse 45% 50% at 20% 30%, rgb(255 182 200 / 0.4), transparent 70%),
      radial-gradient(ellipse 45% 50% at 80% 75%, rgb(190 230 200 / 0.4), transparent 70%);
  }
  .comodo[data-epoca="festa-junina"]::after {
    background:
      radial-gradient(ellipse 50% 45% at 50% 95%, rgb(255 140 40 / 0.4), transparent 70%),
      radial-gradient(ellipse 35% 35% at 15% 20%, rgb(250 204 21 / 0.3), transparent 70%);
  }
  .comodo[data-epoca="dia-das-criancas"]::after {
    background:
      radial-gradient(ellipse 40% 45% at 20% 25%, rgb(59 130 246 / 0.3), transparent 70%),
      radial-gradient(ellipse 40% 45% at 80% 75%, rgb(250 204 21 / 0.3), transparent 70%);
  }
  .comodo[data-epoca="halloween"]::after {
    background:
      radial-gradient(ellipse 45% 50% at 20% 75%, rgb(249 115 22 / 0.4), transparent 70%),
      radial-gradient(ellipse 45% 45% at 85% 25%, rgb(88 28 135 / 0.4), transparent 70%);
  }
  :global(:root[data-tema="noite"]) .comodo {
    filter: blur(16px) brightness(0.38) saturate(0.9);
  }
  .parede {
    position: absolute;
    inset: 0 0 26% 0;
    background:
      linear-gradient(90deg, rgb(0 0 0 / 0.06) 0 1px, transparent 1px) 0 0 / 180px 100%,
      linear-gradient(180deg, #d9d4cd 0%, #cdc7bf 60%, #c2bbb2 100%);
  }
  .janela-luz {
    position: absolute;
    right: 4%;
    top: 4%;
    width: 30%;
    height: 66%;
    background:
      repeating-linear-gradient(90deg, rgb(255 255 255 / 0.85) 0 26px, rgb(235 232 226 / 0.6) 26px 34px),
      #f2efe9;
    opacity: 0.85;
  }
  .chao {
    position: absolute;
    inset: 74% 0 0 0;
    background: linear-gradient(180deg, #b7ada1, #a79c8f);
  }
  .sofa {
    position: absolute;
    left: -2%;
    bottom: 12%;
    width: 26%;
    height: 22%;
    border-radius: 40px 40px 10px 10px;
    background: #6f6c68;
  }
  .luminaria {
    position: absolute;
    right: 9%;
    bottom: 18%;
    width: 4px;
    height: 46%;
    background: #3c3833;
  }
  .luminaria span {
    position: absolute;
    left: -40px;
    top: -30px;
    width: 84px;
    height: 46px;
    border-radius: 40px 40px 6px 6px;
    background: #ebe5da;
  }
  .mesa {
    position: absolute;
    left: 30%;
    right: 30%;
    bottom: 8%;
    height: 16%;
    background:
      linear-gradient(90deg, transparent 6%, #9a7a58 6% 8%, transparent 8% 92%, #9a7a58 92% 94%, transparent 94%),
      transparent;
  }

  /* ---------- Vidro ---------- */
  .casca {
    --vidro-fundo: rgb(255 255 255 / 0.26);
    --vidro-borda: rgb(255 255 255 / 0.5);
    --bloco: rgb(255 255 255 / 0.36);
    --bloco-2: rgb(255 255 255 / 0.55);
    --texto: #1d1d1f;
    --sec: #5d5d63;
    --ativo: #ffffff;
    --ativo-texto: #1d1d1f;
    --verde: #34c759;
    /* Barra que pede atenção (memória cheia, tela abaixo do máximo). */
    --laranja: #c2730f;
    /* O botão principal é a cor do texto com a letra do avesso: preto no claro, branco no escuro. */
    --contra: #ffffff;
    /* O interruptor das áreas (`tokens.css`) fica verde aqui, como os blocos. */
    --ligado: #34c759;
    position: relative;
    z-index: 1;
    height: 100vh;
    color: var(--texto);
    font-family: var(--fonte-texto);
  }
  :global(:root[data-tema="noite"]) .casca {
    --vidro-fundo: rgb(40 40 44 / 0.42);
    --vidro-borda: rgb(255 255 255 / 0.14);
    --bloco: rgb(255 255 255 / 0.08);
    --bloco-2: rgb(255 255 255 / 0.14);
    --texto: #f5f5f7;
    --sec: #a1a1a8;
    --ativo: #f5f5f7;
    --ativo-texto: #1d1d1f;
    --laranja: #ffa53a;
    --contra: #1d1d1f;
  }
  .vidro,
  .vidro-forte {
    background: var(--vidro-fundo);
    border: 1px solid var(--vidro-borda);
    backdrop-filter: blur(34px) saturate(1.5);
    box-shadow:
      0 1px 0 rgb(255 255 255 / 0.35) inset,
      0 20px 50px rgb(40 30 20 / 0.18);
  }
  .vidro-forte {
    background: color-mix(in srgb, var(--vidro-fundo) 40%, rgb(250 250 250 / 0.75));
  }
  :global(:root[data-tema="noite"]) .vidro-forte {
    background: rgb(44 44 48 / 0.88);
  }
  :global(:root.jogo-aberto) .vidro,
  :global(:root.jogo-aberto) .vidro-forte {
    backdrop-filter: none;
    background: rgb(220 214 206 / 0.97);
  }
  :global(:root.jogo-aberto[data-tema="noite"]) .vidro,
  :global(:root.jogo-aberto[data-tema="noite"]) .vidro-forte {
    background: rgb(36 36 40 / 0.98);
  }
  .sec {
    color: var(--sec);
  }

  /* ---------- Barra de cima ---------- */
  .topo {
    position: absolute;
    left: 84px;
    right: 24px;
    top: 14px;
    height: 54px;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 8px 0 10px;
    border-radius: 27px;
  }
  .boas-vindas {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-right: 8px;
  }
  .avatar {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: #1d1d1f;
    box-shadow: 0 0 0 2px rgb(255 255 255 / 0.7);
  }
  .ola {
    display: grid;
    line-height: 1.2;
    white-space: nowrap;
  }
  .ola strong {
    font-family: var(--fonte-titulo);
    font-size: 14px;
    font-weight: 600;
  }
  .ola span {
    font-size: 11px;
  }
  .busca {
    flex: 1;
    max-width: 440px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    padding: 0 14px 0 8px;
    border-radius: 19px;
    background: var(--bloco);
    border: 1px solid var(--vidro-borda);
  }
  .orbe {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 30%, #3a3a3f, #0e0e10);
  }
  .busca input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: none;
    color: var(--texto);
    font-size: 13px;
    user-select: text;
  }
  .busca input::placeholder {
    color: var(--sec);
  }
  .direita {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .link-demo {
    border: 0;
    background: none;
    color: var(--sec);
    font-size: 12px;
    cursor: pointer;
  }
  .segmento {
    display: flex;
    padding: 3px;
    border-radius: 18px;
    background: var(--bloco);
    border: 1px solid var(--vidro-borda);
  }
  .segmento button {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 11px;
    border: 0;
    border-radius: 14px;
    background: transparent;
    color: var(--sec);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 200ms ease, color 200ms ease;
  }
  .segmento button[aria-checked="true"] {
    background: var(--ativo);
    color: var(--ativo-texto);
    box-shadow: 0 2px 8px rgb(0 0 0 / 0.12);
  }
  .redondo {
    position: relative;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border: 1px solid var(--vidro-borda);
    border-radius: 50%;
    background: var(--bloco);
    color: var(--texto);
    cursor: pointer;
  }
  .contador {
    position: absolute;
    top: -2px;
    right: -2px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: #ff3b30;
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    line-height: 16px;
  }
  .controles {
    display: flex;
    margin-left: 4px;
  }
  .controles button {
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--sec);
    cursor: pointer;
  }
  .controles svg,
  .fechar-balao svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
  }
  .controles button:hover {
    background: var(--bloco-2);
    color: var(--texto);
  }
  .controles .x:hover {
    background: #ff3b30;
    color: #fff;
  }

  /* ---------- Balão do Oti e avisos ---------- */
  .balao {
    position: absolute;
    z-index: 20;
    left: 50%;
    top: 78px;
    width: min(520px, calc(100% - 48px));
    margin-left: calc(min(520px, calc(100% - 48px)) / -2);
    display: flex;
    gap: 14px;
    padding: 16px 16px 16px 14px;
    border-radius: 24px;
  }
  .balao-oti {
    flex: none;
  }
  .balao-texto {
    flex: 1;
    display: grid;
    gap: 6px;
  }
  .balao-fala {
    min-height: 1.4em;
    font-family: var(--fonte-titulo);
    font-size: 16px;
    font-weight: 600;
    line-height: 1.35;
  }
  .balao-detalhe {
    color: var(--sec);
    font-size: 13px;
    line-height: 1.5;
  }
  .balao-acoes {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }
  .pilula {
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--vidro-borda);
    border-radius: 16px;
    background: var(--bloco);
    color: var(--texto);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
  .pilula.principal {
    border-color: transparent;
    background: #1d1d1f;
    color: #fff;
  }
  :global(:root[data-tema="noite"]) .pilula.principal {
    background: #f5f5f7;
    color: #1d1d1f;
  }
  .fechar-balao {
    align-self: flex-start;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 50%;
    background: var(--bloco);
    color: var(--sec);
    cursor: pointer;
  }
  .avisos {
    position: absolute;
    z-index: 21;
    right: 70px;
    top: 74px;
    width: 320px;
    display: grid;
    gap: 6px;
    padding: 14px;
    border-radius: 20px;
  }
  .avisos h3 {
    margin: 0 0 4px;
    font-size: 13px;
  }
  .aviso {
    display: flex;
    gap: 10px;
    align-items: baseline;
    padding: 8px;
    border: 0;
    border-radius: 12px;
    background: var(--bloco);
    color: var(--texto);
    text-align: left;
    font-size: 13px;
    cursor: pointer;
  }

  /* ---------- Trilho ---------- */
  .trilho {
    position: absolute;
    left: 18px;
    top: 50%;
    transform: translateY(-50%);
    display: grid;
    gap: 6px;
    padding: 7px;
    border-radius: 27px;
  }
  .trilho-botao {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--texto);
    cursor: pointer;
    transition:
      background 200ms ease,
      transform 200ms cubic-bezier(0.3, 1.5, 0.5, 1);
  }
  .trilho-botao:hover {
    background: var(--bloco-2);
  }
  /* O lugar onde você está: cheio e escuro, para achar de relance sobre o vidro claro. */
  .trilho-botao.ativo {
    background: var(--texto);
    color: var(--contra, #fff);
    box-shadow: 0 2px 6px rgb(0 0 0 / 0.18);
  }
  .trilho-botao:focus-visible {
    outline-offset: 0;
  }

  /* ---------- Painel e grade ---------- */
  .painel {
    position: absolute;
    left: 84px;
    right: 24px;
    top: 80px;
    bottom: 84px;
    border-radius: 32px;
    overflow: hidden;
  }
  .conteudo {
    height: 100%;
    overflow: auto;
    padding: 16px;
  }
  .grade {
    height: 100%;
    min-height: 500px;
    display: grid;
    grid-template-columns: 1fr 1fr 1.1fr 1.15fr;
    grid-template-rows: minmax(0, 1.05fr) minmax(0, 1fr) minmax(0, 0.9fr);
    grid-template-areas:
      "jogos jogos partida acoes"
      "agora fundo fundo acoes"
      "agora avisos avisos tela";
    gap: 12px;
  }
  .b-jogos {
    grid-area: jogos;
  }
  .b-partida {
    grid-area: partida;
  }
  .b-acoes {
    grid-area: acoes;
  }
  .b-agora {
    grid-area: agora;
  }
  .b-fundo {
    grid-area: fundo;
  }
  .b-avisos {
    grid-area: avisos;
  }
  .b-tela {
    grid-area: tela;
  }

  /* O bloco: vidro calmo. A luz segue o cursor; ele não pula nem cresce. */
  .bloco {
    --mx: 50%;
    --my: -40%;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
    min-height: 0;
    padding: 16px;
    border-radius: 20px;
    background:
      radial-gradient(260px circle at var(--mx) var(--my), rgb(255 255 255 / 0.22), transparent 60%),
      var(--bloco);
    border: 1px solid var(--vidro-borda);
    overflow: hidden;
  }
  :global(:root[data-tema="noite"]) .bloco {
    background:
      radial-gradient(260px circle at var(--mx) var(--my), rgb(255 255 255 / 0.06), transparent 60%),
      var(--bloco);
  }
  .bloco h3 {
    margin: 0;
    color: var(--sec);
    font-family: var(--fonte-texto);
    font-size: 12px;
    font-weight: 500;
    line-height: 1.2;
  }
  .pequeno {
    font-size: 12px;
    line-height: 1.4;
  }

  /* Botões do painel: um principal escuro, um pequeno de contorno e o link de texto. */
  .botao-principal {
    height: 40px;
    border: 0;
    border-radius: 12px;
    background: var(--texto);
    color: var(--contra);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 120ms ease;
  }
  .botao-principal:hover {
    opacity: 0.88;
  }
  .botao-pequeno {
    flex: none;
    align-self: center;
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--vidro-borda);
    border-radius: 9px;
    background: var(--bloco-2);
    color: var(--texto);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
  .botao-pequeno:hover {
    background: var(--bloco);
    border-color: var(--sec);
  }
  .botao-pequeno.escuro {
    align-self: flex-start;
    margin-top: auto;
    border-color: transparent;
    background: var(--texto);
    color: var(--contra);
  }
  .botao-pequeno.escuro:hover {
    opacity: 0.88;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    margin-top: auto;
    padding: 2px 0;
    border: 0;
    background: none;
    color: var(--texto);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  /* Interruptor verde */
  .chave {
    position: relative;
    flex: none;
    width: 40px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 12px;
    background: rgb(120 120 128 / 0.32);
    cursor: pointer;
    transition: background 200ms ease;
  }
  .chave::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.2);
    transition: transform 240ms cubic-bezier(0.3, 1.4, 0.5, 1);
  }
  .chave[aria-checked="true"] {
    background: var(--verde);
  }
  .chave[aria-checked="true"]::after {
    transform: translateX(16px);
  }
  .chave:disabled {
    opacity: 0.45;
    cursor: default;
  }

  /* Seus jogos */
  .campo-busca {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 12px;
    border-radius: 10px;
    background: var(--bloco-2);
    color: var(--sec);
  }
  .campo-busca:focus-within {
    outline: 2px solid var(--texto);
    outline-offset: -1px;
  }
  .campo-busca input::placeholder {
    color: var(--sec);
  }
  .campo-busca input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--texto);
    font-size: 13px;
    user-select: text;
  }
  .tres {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    min-height: 0;
  }
  .capa-jogo {
    position: relative;
    display: grid;
    min-width: 0;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: 12px;
    background: var(--bloco-2);
    color: #fff;
    cursor: pointer;
    overflow: hidden;
  }
  .capa-jogo img,
  .capa-jogo .ini {
    grid-area: 1 / 1;
    width: 100%;
    height: 100%;
    min-height: 0;
    object-fit: cover;
    transition: transform 300ms ease;
  }
  .capa-jogo:hover img {
    transform: scale(1.03);
  }
  /* O nome por cima do pé da capa: a arte fica inteira e o nome não briga com um rodapé claro. */
  .capa-nome {
    grid-area: 1 / 1;
    align-self: end;
    padding: 18px 10px 8px;
    background: linear-gradient(transparent, rgb(0 0 0 / 0.62));
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ini {
    display: grid;
    place-items: center;
    background: var(--bloco-2);
    color: var(--sec);
    font-family: var(--fonte-titulo);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .vazio {
    grid-column: 1 / -1;
    font-size: 12px;
  }

  /* Última partida */
  .faixa {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .capa {
    display: grid;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 10px;
    overflow: hidden;
    background: var(--bloco-2);
  }
  .capa img,
  .capa .ini {
    width: 100%;
    height: 100%;
    object-fit: cover;
    font-size: 12px;
  }
  .faixa-nome {
    min-width: 0;
    font-size: 15px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .numeros {
    display: flex;
    gap: 24px;
    margin: 0;
  }
  .numeros dt {
    color: var(--sec);
    font-size: 12px;
  }
  .numeros dd {
    margin: 0;
    font-size: 26px;
    font-weight: 600;
    line-height: 1.1;
  }

  /* Otimizar */
  .b-acoes .botao-principal {
    margin: 2px 0 6px;
  }
  .atalhos {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .atalhos li + li {
    border-top: 1px solid var(--vidro-borda);
  }
  .atalhos button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    min-height: 42px;
    padding: 0 2px;
    border: 0;
    background: none;
    color: var(--texto);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .atalhos button :global(.icone) {
    color: var(--sec);
    transition: transform 160ms ease;
  }
  .atalhos button:hover :global(.icone) {
    color: var(--texto);
    transform: translateX(2px);
  }
  .desfazer {
    margin-top: 4px;
    color: var(--sec);
  }

  /* Agora */
  .medidores {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-top: 6px;
  }
  .medidor-linha {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 6px;
    font-size: 13px;
  }
  .medidor-linha .numero {
    color: var(--sec);
    font-size: 13px;
    white-space: nowrap;
  }
  .trilha {
    height: 4px;
    border-radius: 2px;
    background: rgb(120 120 128 / 0.2);
    overflow: hidden;
  }
  .trilha div {
    height: 100%;
    border-radius: 2px;
    background: var(--texto);
    transition: width 600ms ease;
  }
  .trilha.alerta div {
    background: var(--laranja);
  }

  /* Durante o jogo */
  .chaves {
    list-style: none;
    margin: 0;
    padding: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: space-around;
  }
  .chaves li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 0;
  }
  .chaves li + li {
    border-top: 1px solid var(--vidro-borda);
  }
  .chaves li > div {
    display: grid;
    gap: 1px;
    min-width: 0;
    font-size: 13px;
  }

  /* Avisos */
  .lembretes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .aviso-linha {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    padding: 4px 0;
    border: 0;
    background: none;
    color: var(--texto);
    font-size: 13px;
    line-height: 1.35;
    text-align: left;
  }
  button.aviso-linha {
    cursor: pointer;
  }
  button.aviso-linha:hover span:last-child {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .aviso-linha .ponto {
    transform: translateY(-1px);
  }

  /* Tela */
  .gigante {
    font-size: 40px;
    font-weight: 600;
    line-height: 1;
    letter-spacing: -0.03em;
  }
  .gigante small {
    margin-left: 4px;
    color: var(--sec);
    font-size: 16px;
    font-weight: 500;
    letter-spacing: 0;
  }

  /* Janela estreita: a grade vira uma coluna dupla que rola. */
  @media (max-width: 1100px) {
    .grade {
      height: auto;
      grid-template-columns: 1fr 1fr;
      grid-template-rows: none;
      grid-auto-rows: minmax(180px, auto);
      grid-template-areas:
        "jogos jogos"
        "partida acoes"
        "agora acoes"
        "fundo fundo"
        "avisos tela";
    }
  }

  /* ---------- Área aberta dentro do painel ---------- */
  .area-aberta {
    display: grid;
    gap: 8px;
    max-width: 860px;
    margin: 0 auto;
    padding: 6px 10px;
    /* As áreas usam os tokens de `tokens.css`: aqui eles viram vidro. */
    --superficie: rgb(255 255 255 / 0.4);
    --superficie-2: rgb(255 255 255 / 0.5);
    --tinta: var(--texto);
    --tinta-2: var(--sec);
    --linha: rgb(0 0 0 / 0.08);
    --linha-forte: rgb(0 0 0 / 0.14);
    --acao: #1d1d1f;
    --acao-forte: #000;
  }
  :global(:root[data-tema="noite"]) .area-aberta {
    --superficie: rgb(255 255 255 / 0.08);
    --superficie-2: rgb(255 255 255 / 0.12);
    --linha: rgb(255 255 255 / 0.08);
    --linha-forte: rgb(255 255 255 / 0.16);
    --acao: #f5f5f7;
    --acao-forte: #fff;
  }
  .area-topo {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-bottom: 6px;
  }
  .area-topo h2 {
    font-family: var(--fonte-titulo);
    font-size: 20px;
    font-weight: 600;
  }
  .area-corpo {
    padding: 4px 18px 18px;
    border-radius: 24px;
    background: var(--bloco);
    border: 1px solid var(--vidro-borda);
  }
  .em-breve {
    display: grid;
    justify-items: start;
    gap: 12px;
    padding: 16px 0;
  }

  /* ---------- Mais áreas ---------- */
  .mais {
    position: absolute;
    z-index: 21;
    left: 50%;
    bottom: 84px;
    transform: translateX(-50%);
    display: flex;
    gap: 4px;
    padding: 6px;
    border-radius: 22px;
  }
  .mais button {
    height: 34px;
    padding: 0 14px;
    border: 0;
    border-radius: 17px;
    background: transparent;
    color: var(--texto);
    font-size: 13px;
    white-space: nowrap;
    cursor: pointer;
  }
  .mais button:hover {
    background: var(--ativo);
    color: var(--ativo-texto);
  }

  /* ---------- Abas ---------- */
  .abas {
    position: absolute;
    left: 50%;
    bottom: 20px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px;
    border-radius: 26px;
  }
  .pilula-ativa {
    position: absolute;
    left: 0;
    top: 6px;
    height: 38px;
    border-radius: 19px;
    background: var(--ativo);
    box-shadow: 0 4px 14px rgb(0 0 0 / 0.14);
    transition:
      transform 420ms cubic-bezier(0.3, 1.25, 0.4, 1),
      width 420ms cubic-bezier(0.3, 1.25, 0.4, 1),
      opacity 200ms ease;
  }
  .aba {
    position: relative;
    height: 38px;
    padding: 0 18px;
    border: 0;
    border-radius: 19px;
    background: transparent;
    color: var(--texto);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: color 200ms ease;
  }
  .aba.ativa {
    color: var(--ativo-texto);
    font-weight: 600;
  }
  .aba-mais {
    position: relative;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    margin-left: 4px;
    border: 0;
    border-radius: 50%;
    background: var(--ativo);
    color: var(--ativo-texto);
    cursor: pointer;
  }

  .pausado {
    position: absolute;
    left: 50%;
    bottom: 76px;
    transform: translateX(-50%);
    padding: 4px 12px;
    border-radius: 12px;
    background: rgb(0 0 0 / 0.5);
    color: #fff;
    font-size: 12px;
  }

  @media (max-width: 1100px) {
    .grade {
      grid-template-columns: 1fr 1fr 1.4fr;
      grid-template-rows: auto auto auto auto;
      grid-template-areas:
        "jogos jogos banda"
        "avisos aneis jogo"
        "acoes acoes tela"
        "wifi dpc tela";
    }
    .ola {
      display: none;
    }
  }
</style>
