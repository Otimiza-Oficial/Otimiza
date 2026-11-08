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
  import Energia from "./componentes/Energia.svelte";
  import Jogos from "./componentes/Jogos.svelte";
  import Prova from "./componentes/Prova.svelte";
  import Sistema from "./componentes/Sistema.svelte";
  import Turbo from "./componentes/Turbo.svelte";
  import Latencia from "./componentes/Latencia.svelte";
  import Oti, { type Humor } from "./componentes/Oti.svelte";
  import EfeitoDeEpoca from "./componentes/EfeitoDeEpoca.svelte";
  import { porId, temporadaDe } from "./lib/temporadas";
  import Rede from "./componentes/Rede.svelte";
  import Tela from "./componentes/Tela.svelte";
  import { areas, type IdDaArea } from "./lib/areas";
  import { avisos as lerAvisos } from "./lib/clima";
  import { capaRemota } from "./lib/capas";
  import { abertura, entender, jogoAbriu, jogoFechou, type Acao, type Fala } from "./lib/conversa";
  import {
    noTauri,
    ponte,
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
    { id: "jogos", icone: "pilha" },
    { id: "rede", icone: "wifi" },
    { id: "latencia", icone: "cronometro" },
    { id: "tela", icone: "monitor" },
    { id: "historico", icone: "relogio" },
    { id: "ajustes", icone: "engrenagem" },
  ];
  const ABAS: { id: IdDaArea; nome: string }[] = [
    { id: "inicio", nome: "Visão geral" },
    { id: "rede", nome: "Rede" },
    { id: "latencia", nome: "Latência" },
    { id: "tela", nome: "Tela" },
    { id: "historico", nome: "Histórico" },
  ];
  /** O ícone de cada área no cabeçalho do painel (inclusive as que só abrem pelo "+"). */
  const ICONE_DA_AREA: Record<IdDaArea, NomeDoIcone> = {
    inicio: "casa",
    turbo: "raio",
    jogos: "pilha",
    rede: "wifi",
    latencia: "cronometro",
    energia: "bateria",
    tela: "monitor",
    sistema: "chip",
    prova: "grafico",
    historico: "relogio",
    ajustes: "engrenagem",
  };
  const OUTRAS: IdDaArea[] = ["turbo", "jogos", "energia", "sistema", "prova", "ajustes"];

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

  const lista = $derived(lerAvisos(hardware, metricas, jogo !== null));
  const area = $derived(areas.find((a) => a.id === aberta) ?? areas[0]);
  const humor = $derived<Humor>(reacao ?? (jogo ? "jogando" : digitando ? "falando" : (fala?.humor ?? "atento")));
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

  function abrir(id: IdDaArea) {
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

  /** Sem capa da Steam: um degradê com a cor do nome do jogo (sempre a mesma) e o controle no meio. */
  function corDoJogo(nome: string) {
    let h = 0;
    for (const c of nome) h = (h * 31 + c.charCodeAt(0)) % 360;
    return `linear-gradient(140deg, hsl(${h} 70% 62%), hsl(${(h + 40) % 360} 65% 45%))`;
  }

  function nomeCurto(nome: string) {
    return nome.replace(/\((R|TM)\)/gi, "").replace(/\s+CPU\s*@.*$/i, "").replace(/^NVIDIA\s+/i, "").replace(/\s+/g, " ").trim();
  }

  /** O anel de um medidor: a fração vira o traço do círculo. */
  function anel(f: number | null) {
    const c = 2 * Math.PI * 17;
    return `${Math.max(0, Math.min(1, f ?? 0)) * c} ${c}`;
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

    const ajustar = () => void medirPilula();
    window.addEventListener("resize", ajustar);
    return () => {
      vivo = false;
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
      <span class="mic"><Icone nome="microfone" tamanho={16} /></span>
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
          <button class="aviso" onclick={() => abrir(a.id.startsWith("hz-") ? "tela" : "inicio")}>
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
        <Icone nome={t.icone} tamanho={18} cheio={aberta === t.id && t.id === "inicio"} />
      </button>
    {/each}
    <button class="trilho-botao" aria-label="Mais áreas" onclick={() => (mostrarMais = !mostrarMais)}>
      <Icone nome="seta_baixo" tamanho={18} />
    </button>
  </nav>

  <!-- Painel -->
  <main class="painel vidro">
    {#key aberta}
      <div class="conteudo" in:entrar>
        {#if aberta === "inicio"}
          <div class="grade">
            <!-- Jogos instalados (o bloco de busca da referência) -->
            <section class="bloco b-jogos" use:brilho>
              <label class="campo-busca">
                <span class="logo-bloco"><Icone nome="controle" tamanho={14} /></span>
                <input bind:value={filtroDeJogo} placeholder="Buscar nos seus jogos" aria-label="Buscar nos seus jogos" />
                <Icone nome="busca" tamanho={14} />
              </label>
              <div class="tres">
                {#each jogosFiltrados as { jogo: j, capa } (j.nome)}
                  <button class="mini-bloco" onclick={() => abrir("jogos")} title={j.nome}>
                    {#if capa}<img src={capa} alt="" onerror={() => fotoFalhou(j.nome)} />{:else}<span class="ini" style:background={corDoJogo(j.nome)}><Icone nome="controle" tamanho={26} /></span>{/if}
                    <span>{j.nome}</span>
                  </button>
                {:else}
                  <p class="sec vazio">{instalados.length ? "Nenhum jogo com esse nome." : "Lendo seus jogos…"}</p>
                {/each}
              </div>
            </section>

            <!-- Banda do segundo plano (o bloco "Smart TV") -->
            <section class="bloco b-banda" use:brilho>
              <div class="cabeca">
                <h3>Banda de fundo</h3>
                <button
                  class="chave"
                  role="switch"
                  aria-checked={banda?.ligado ?? false}
                  aria-label="Segurar a banda do segundo plano durante o jogo"
                  disabled={!banda || (!banda.ligado && banda.impedimento !== null)}
                  title={banda?.impedimento ?? undefined}
                  onclick={alternarBanda}
                ></button>
              </div>
              <div class="apps">
                <span class="app" style:--c="#0a84ff" title="Nuvem (OneDrive, Google Drive, Dropbox, MEGA)"><Icone nome="nuvem" tamanho={17} /></span>
                <span class="app" style:--c="#30d158" title="Torrents"><Icone nome="ima" tamanho={17} /></span>
                <span class="app" style:--c="#5e5ce6" title="Lojas de jogo (Epic, Battle.net, EA, Ubisoft)"><Icone nome="loja" tamanho={17} /></span>
                <span class="app" style:--c="#ff9f0a" title="Windows Update em segundo plano"><Icone nome="atualizar" tamanho={17} /></span>
                <button class="app mais-app" aria-label="Ver na Rede" onclick={() => abrir("rede")}><Icone nome="mais" tamanho={14} /></button>
              </div>
              <p class="sec pequeno">
                {banda?.agindo_em ? `Segurando durante ${banda.agindo_em}` : banda?.impedimento ? "Precisa de administrador" : "Envio de nuvem e torrent, só no jogo"}
              </p>
            </section>

            <!-- Último jogo (o player da referência) -->
            <section class="bloco b-jogo" use:brilho>
              <div class="faixa">
                <span class="capa">
                  {#if capaDaUltima}<img src={capaDaUltima} alt="" onerror={() => jogoDoPlayer && fotoFalhou(jogoDoPlayer)} />{:else if jogoDoPlayer}<span class="ini" style:background={corDoJogo(jogoDoPlayer)}><Icone nome="controle" tamanho={20} /></span>{/if}
                </span>
                <div class="faixa-texto">
                  <strong>{jogoDoPlayer ?? "Nenhuma partida ainda"}</strong>
                  <span class="sec">{jogo ? "Jogando agora" : partidaDoPlayer?.ultima ? "Última partida" : "Jogue com o Otimiza aberto"}</span>
                </div>
                {#if partidaDoPlayer?.ganho_low.estado === "Medido" && partidaDoPlayer.ganho_low.pct != null}
                  <span class="selo-medido">{partidaDoPlayer.ganho_low.pct >= 0 ? "+" : ""}{inteiro(partidaDoPlayer.ganho_low.pct)}%</span>
                {/if}
              </div>
              {#if partidaDoPlayer?.ultima}
                {@const fps = partidaDoPlayer.ultima.fps_nativo ?? 0}
                {@const low = partidaDoPlayer.ultima.low_1pct ?? 0}
                <div class="progresso">
                  <span class="numero sec">{inteiro(low)}</span>
                  <div class="trilha"><div style:width={`${fps > 0 ? Math.min(100, (low / fps) * 100) : 0}%`}></div></div>
                  <span class="numero sec">{inteiro(fps)} FPS</span>
                </div>
              {/if}
              <div class="controles-jogo">
                <button aria-label="Histórico de partidas" onclick={() => abrir("jogos")}><Icone nome="voltar" tamanho={16} cheio /></button>
                <button class="grande" aria-label="Prova antes e depois" onclick={() => abrir("prova")}><Icone nome={jogo ? "pausa" : "play"} tamanho={20} cheio /></button>
                <button aria-label="Próximo jogo" onclick={() => abrir("jogos")}><Icone nome="avancar" tamanho={16} cheio /></button>
              </div>
            </section>

            <!-- Avisos (o bloco "Reminders") -->
            <section class="bloco b-avisos" use:brilho>
              <h3>Avisos</h3>
              <ul class="lembretes">
                {#each lista.slice(0, 2) as a (a.id)}
                  <li><span class="bolinha-vazia"></span>{a.titulo}</li>
                {:else}
                  <li class="sec"><span class="bolinha-vazia"></span>Nada atrapalhando</li>
                {/each}
              </ul>
              <span class="numero enorme">{lista.length}</span>
            </section>

            <!-- Anéis (o bloco dos dispositivos) -->
            <section class="bloco b-aneis" use:brilho>
              {#each [{ n: "Processador", v: metricas?.cpu.overall ?? null, i: "chip" }, { n: "Memória", v: memoriaPct, i: "memoria" }, { n: "Tela", v: telaPrincipal ? (telaPrincipal.hz_atual / Math.max(1, telaPrincipal.hz_maximo)) * 100 : null, i: "monitor" }, { n: "Wi-Fi", v: sinalWifi, i: "wifi" }] as m (m.n)}
                <div class="anel" title={`${m.n}: ${pct(m.v)}`}>
                  <svg viewBox="0 0 40 40" width="46" height="46">
                    <circle cx="20" cy="20" r="17" class="anel-fundo" />
                    <circle cx="20" cy="20" r="17" class="anel-valor" stroke-dasharray={anel(m.v == null ? null : m.v / 100)} />
                  </svg>
                  <span class="anel-icone"><Icone nome={m.i as NomeDoIcone} tamanho={15} /></span>
                </div>
              {/each}
            </section>

            <!-- Ações rápidas (o bloco AirDrop/SharePlay/Copiar) -->
            <section class="bloco b-acoes" use:brilho>
              <div class="atalhos">
                {#each [{ id: "rede", n: "Rede", i: "wifi", c: "#0a84ff" }, { id: "latencia", n: "Latência", i: "cronometro", c: "#30d158" }, { id: "tela", n: "Tela", i: "monitor", c: "#5e5ce6" }, { id: "historico", n: "Histórico", i: "relogio", c: "#ff9f0a" }] as x (x.id)}
                  <button class="atalho" onclick={() => abrir(x.id as IdDaArea)}>
                    <span class="atalho-icone" style:--c={x.c}><Icone nome={x.i as NomeDoIcone} tamanho={18} /></span>
                    <span>{x.n}</span>
                  </button>
                {/each}
              </div>
              <button class="linha-acao destaque" onclick={() => abrir("rede")}>
                <span>Medir a rota até o jogo</span><Icone nome="rota" tamanho={16} />
              </button>
              <div class="grupo">
                <button class="linha-acao" onclick={() => abrir("historico")}><span>Ver o que foi mudado{mudancas ? ` (${mudancas})` : ""}</span><Icone nome="lista" tamanho={16} /></button>
                <button class="linha-acao" onclick={() => abrir("historico")}><span>Desfazer tudo</span><Icone nome="desfazer" tamanho={16} /></button>
                <button class="linha-acao" onclick={() => abrir("latencia")}><span>Achar o driver que trava</span><Icone nome="escudo" tamanho={16} /></button>
              </div>
            </section>

            <!-- Tela (o bloco do ar-condicionado) -->
            <section class="bloco b-tela" use:brilho>
              <div class="cabeca">
                <h3>Tela</h3>
                <button
                  class="chave"
                  role="switch"
                  aria-checked={!!telaPrincipal && telaPrincipal.hz_atual >= telaPrincipal.hz_maximo}
                  aria-label="Monitor na taxa máxima"
                  disabled={!telaPrincipal}
                  onclick={() => abrir("tela")}
                ></button>
              </div>
              <div class="medida">
                <span class="numero gigante">{telaPrincipal ? telaPrincipal.hz_atual : "—"}<small>Hz</small></span>
                <div class="setas">
                  <button aria-label="Mais Hz" onclick={() => abrir("tela")}><Icone nome="seta_cima" tamanho={14} /></button>
                  <button aria-label="Menos Hz" onclick={() => abrir("tela")}><Icone nome="seta_baixo" tamanho={14} /></button>
                </div>
              </div>
              <dl class="linhas">
                <dt>Monitor</dt>
                <dd><span class="etiqueta">{telaPrincipal?.nome ?? "—"}</span></dd>
                <dt>Resolução</dt>
                <dd><span class="etiqueta">{telaPrincipal ? `${telaPrincipal.largura}×${telaPrincipal.altura}` : "—"}</span></dd>
                <dt>Aguenta</dt>
                <dd><span class="etiqueta">{telaPrincipal ? `${telaPrincipal.hz_maximo} Hz` : "—"}</span></dd>
                <dt>Quadros</dt>
                <dd class="barras">
                  {#each [1, 2, 3, 4, 5] as b (b)}
                    <span class:cheia={!!telaPrincipal && telaPrincipal.hz_atual / Math.max(1, telaPrincipal.hz_maximo) >= b / 5 - 0.05} style:height={`${4 + b * 3}px`}></span>
                  {/each}
                </dd>
              </dl>
            </section>

            <!-- Wi-Fi sem varredura (o "Smart Light") -->
            <section class="bloco b-wifi" use:brilho>
              <h3>Wi-Fi sem procurar redes</h3>
              <p class="sec pequeno">{varredura?.agindo_em ? `Agindo em ${varredura.agindo_em}` : "Menos pico de ping no jogo"}</p>
              <button class="chave" role="switch" aria-checked={varredura?.ligado ?? false} aria-label="Wi-Fi sem procurar redes durante o jogo" disabled={!varredura} onclick={alternarVarredura}></button>
            </section>

            <!-- Caçador de DPC (o "Vacuum Cleaner") -->
            <section class="bloco b-dpc" use:brilho>
              <h3>Caçador de DPC</h3>
              <p class="sec pequeno">{recadoDpc ?? "Acha o driver que trava"}</p>
              <button class="chave" role="switch" aria-checked={medindoDpc} aria-label="Medir o DPC por 10 segundos" onclick={medirDpc}></button>
            </section>
          </div>
        {:else}
          <div class="area-aberta">
            <header class="area-topo">
              <span class="area-icone"><Icone nome={ICONE_DA_AREA[aberta]} tamanho={18} /></span>
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
              {:else if aberta === "prova"}
                <Prova jogo={jogo?.nome ?? null} />
              {:else if aberta === "sistema"}
                <Sistema />
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
  .mic {
    color: var(--sec);
    display: grid;
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
    transform: scale(1.08);
  }
  .trilho-botao.ativo {
    background: var(--ativo);
    color: var(--ativo-texto);
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.15);
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
    padding: 18px;
  }
  .grade {
    height: 100%;
    min-height: 470px;
    display: grid;
    grid-template-columns: 1fr 1fr 1.7fr 1.55fr;
    grid-template-rows: 1fr 1.15fr 0.85fr;
    grid-template-areas:
      "jogos jogos banda jogo"
      "avisos aneis acoes tela"
      "wifi dpc acoes tela";
    gap: 14px;
  }
  .b-jogos {
    grid-area: jogos;
  }
  .b-banda {
    grid-area: banda;
  }
  .b-jogo {
    grid-area: jogo;
  }
  .b-avisos {
    grid-area: avisos;
  }
  .b-aneis {
    grid-area: aneis;
  }
  .b-acoes {
    grid-area: acoes;
  }
  .b-tela {
    grid-area: tela;
  }
  .b-wifi {
    grid-area: wifi;
  }
  .b-dpc {
    grid-area: dpc;
  }

  .bloco {
    --mx: 50%;
    --my: -40%;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
    min-height: 0;
    padding: 14px;
    border-radius: 22px;
    background:
      radial-gradient(240px circle at var(--mx) var(--my), rgb(255 255 255 / 0.28), transparent 60%),
      var(--bloco);
    border: 1px solid var(--vidro-borda);
    transition:
      transform 260ms cubic-bezier(0.3, 1.3, 0.5, 1),
      box-shadow 260ms ease;
  }
  :global(:root[data-tema="noite"]) .bloco {
    background:
      radial-gradient(240px circle at var(--mx) var(--my), rgb(255 255 255 / 0.1), transparent 60%),
      var(--bloco);
  }
  .bloco:hover {
    transform: translateY(-2px) scale(1.01);
    box-shadow: 0 14px 30px rgb(40 30 20 / 0.12);
  }
  .bloco h3 {
    margin: 0;
    font-family: var(--fonte-titulo);
    font-size: 15px;
    font-weight: 600;
    line-height: 1.2;
  }
  .cabeca {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .pequeno {
    font-size: 12px;
    line-height: 1.35;
  }

  /* Chave verde */
  .chave {
    position: relative;
    flex: none;
    width: 44px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 13px;
    background: rgb(120 120 128 / 0.32);
    cursor: pointer;
    transition: background 220ms ease;
  }
  .chave::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 6px rgb(0 0 0 / 0.2);
    transition: transform 320ms cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  .chave[aria-checked="true"] {
    background: var(--verde);
  }
  .chave[aria-checked="true"]::after {
    transform: translateX(18px);
  }
  .chave:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .b-wifi .chave,
  .b-dpc .chave {
    margin-top: auto;
  }

  /* Jogos */
  .campo-busca {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 12px 0 6px;
    border-radius: 17px;
    background: var(--bloco-2);
    color: var(--sec);
  }
  .logo-bloco {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 7px;
    background: #ff3b30;
    color: #fff;
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
    gap: 8px;
    min-height: 0;
  }
  .mini-bloco {
    display: grid;
    grid-template-rows: 1fr auto;
    justify-items: center;
    gap: 4px;
    min-width: 0;
    padding: 8px 6px 6px;
    border: 0;
    border-radius: 14px;
    background: var(--bloco-2);
    color: var(--texto);
    font-size: 11px;
    cursor: pointer;
    overflow: hidden;
  }
  .mini-bloco img {
    width: 100%;
    height: 100%;
    min-height: 0;
    object-fit: cover;
    border-radius: 9px;
  }
  .mini-bloco > span:last-child {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ini {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    border-radius: 9px;
    color: rgb(255 255 255 / 0.92);
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.3);
  }
  .vazio {
    grid-column: 1 / -1;
    font-size: 12px;
  }

  /* Banda (ícones dos programas) */
  .apps {
    display: flex;
    gap: 6px;
  }
  .app {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 9px;
    background: var(--c, #1d1d1f);
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    box-shadow: 0 2px 6px rgb(0 0 0 / 0.15);
  }
  .mais-app {
    --c: var(--bloco-2);
    color: var(--texto);
    cursor: pointer;
    box-shadow: none;
    border: 1px solid var(--vidro-borda);
  }

  /* Último jogo */
  .faixa {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .capa {
    display: grid;
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 10px;
    overflow: hidden;
    background: rgb(0 0 0 / 0.08);
  }
  .capa img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .faixa-texto {
    flex: 1;
    min-width: 0;
    display: grid;
    font-size: 13px;
  }
  .faixa-texto strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .faixa-texto span {
    font-size: 12px;
  }
  .progresso {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
  }
  .trilha {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: rgb(120 120 128 / 0.25);
    overflow: hidden;
  }
  .trilha div {
    height: 100%;
    background: var(--texto);
    border-radius: 2px;
    transition: width 600ms ease;
  }
  .controles-jogo {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 18px;
    margin-top: auto;
  }
  .controles-jogo button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--texto);
    cursor: pointer;
  }
  .controles-jogo .grande {
    width: 40px;
    height: 40px;
    background: var(--bloco-2);
  }

  /* Avisos */
  .lembretes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
    font-size: 12px;
    line-height: 1.3;
  }
  .lembretes li {
    display: flex;
    gap: 6px;
    align-items: baseline;
  }
  .bolinha-vazia {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1.5px solid var(--sec);
    transform: translateY(1px);
  }
  .enorme {
    margin-top: auto;
    font-size: 30px;
    font-weight: 600;
    line-height: 1;
  }

  /* Anéis */
  .b-aneis {
    display: grid;
    grid-template-columns: 1fr 1fr;
    place-items: center;
  }
  .anel {
    position: relative;
    display: grid;
    place-items: center;
  }
  .anel svg {
    transform: rotate(-90deg);
  }
  .anel-fundo {
    fill: var(--bloco-2);
    stroke: rgb(120 120 128 / 0.25);
    stroke-width: 3.5;
  }
  .anel-valor {
    fill: none;
    stroke: var(--verde);
    stroke-width: 3.5;
    stroke-linecap: round;
    transition: stroke-dasharray 700ms cubic-bezier(0.3, 1, 0.4, 1);
  }
  .anel-icone {
    position: absolute;
    display: grid;
    color: var(--texto);
  }

  /* Ações rápidas */
  .atalhos {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
  }
  .atalho {
    display: grid;
    justify-items: center;
    gap: 5px;
    padding: 4px 0;
    border: 0;
    background: none;
    color: var(--texto);
    font-size: 11px;
    cursor: pointer;
  }
  .atalho-icone {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--c);
    color: #fff;
    box-shadow: 0 4px 10px color-mix(in srgb, var(--c) 40%, transparent);
    transition: transform 200ms cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  .atalho:hover .atalho-icone {
    transform: scale(1.1);
  }
  .linha-acao {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    border: 0;
    background: var(--bloco-2);
    color: var(--texto);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .linha-acao.destaque {
    border-radius: 12px;
  }
  .grupo {
    display: grid;
    gap: 1px;
    border-radius: 12px;
    overflow: hidden;
  }
  .linha-acao:hover {
    background: var(--ativo);
    color: var(--ativo-texto);
  }

  /* Tela */
  .medida {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .gigante {
    font-size: 46px;
    font-weight: 600;
    line-height: 1;
    letter-spacing: -0.03em;
  }
  .gigante small {
    margin-left: 4px;
    font-size: 18px;
    font-weight: 500;
  }
  .setas {
    display: grid;
    gap: 6px;
  }
  .setas button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 28px;
    border: 0;
    border-radius: 9px;
    background: var(--bloco-2);
    color: var(--texto);
    cursor: pointer;
  }
  .linhas {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 8px 10px;
    margin: auto 0 0;
    font-size: 12px;
  }
  .linhas dt {
    color: var(--texto);
  }
  .linhas dd {
    margin: 0;
    justify-self: end;
    min-width: 0;
  }
  .etiqueta {
    display: inline-block;
    max-width: 120px;
    padding: 3px 9px;
    border-radius: 7px;
    background: var(--bloco-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    vertical-align: middle;
  }
  .barras {
    display: flex;
    align-items: flex-end;
    gap: 3px;
    height: 20px;
  }
  .barras span {
    width: 5px;
    border-radius: 2px;
    background: rgb(120 120 128 / 0.3);
  }
  .barras span.cheia {
    background: var(--texto);
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
  .area-icone {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--ativo);
    color: var(--ativo-texto);
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
