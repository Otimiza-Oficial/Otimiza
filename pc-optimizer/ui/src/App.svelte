<script lang="ts">
  // A casca da interface nova, no formato da referência do dono (painel de vidro estilo visionOS): barra de cima com
  // boas-vindas, a busca do Oti e Claro/Escuro; trilho de ícones à esquerda; uma grade de blocos de vidro no meio; abas
  // embaixo. Cada bloco mostra um dado lido deste PC ou liga uma coisa que existe de verdade; o que ainda não migrou
  // abre a área dizendo em que onda chega. Regras em `ui/DESIGN.md`.
  import { onMount } from "svelte";
  import { cubicOut } from "svelte/easing";
  import Historico from "./componentes/Historico.svelte";
  import Icone from "./componentes/Icone.svelte";
  import Inicio, { type ItemDaProntidao } from "./componentes/Inicio.svelte";
  import Trilho from "./componentes/Trilho.svelte";
  import BarraDeAbas from "./componentes/BarraDeAbas.svelte";
  import { Navegacao, type AbaDoSistema } from "./estado/navegacao.svelte";
  import Ajustes from "./componentes/Ajustes.svelte";
  import Mira from "./componentes/Mira.svelte";
  import Cores from "./componentes/Cores.svelte";
  import Energia from "./componentes/Energia.svelte";
  import Jogos from "./componentes/Jogos.svelte";
  import Prova from "./componentes/Prova.svelte";
  import Sistema from "./componentes/Sistema.svelte";
  import PedidoDeFeedback from "./componentes/PedidoDeFeedback.svelte";
  import Confirmar from "./componentes/base/Confirmar.svelte";
  import BoasVindas, { ABRIR_BOAS_VINDAS, type ModoDasBoasVindas } from "./componentes/BoasVindas.svelte";
  import Turbo from "./componentes/Turbo.svelte";
  import Latencia from "./componentes/Latencia.svelte";
  import Oti, { type Humor } from "./componentes/Oti.svelte";
  import EfeitoDeEpoca from "./componentes/EfeitoDeEpoca.svelte";
  import { porId, temporadaDe } from "./lib/temporadas";
  import Rede from "./componentes/Rede.svelte";
  import Tela from "./componentes/Tela.svelte";
  import { ABRIR_AREA, areas, type Grupo, type IdDaArea } from "./lib/areas";
  import { contextoDa } from "./lib/contexto";
  import { avisos as lerAvisos } from "./lib/clima";
  import { capaDepoisDaFalha, capaRemota } from "./lib/capas";
  import { abertura, entender, jogoAbriu, jogoFechou, partidaPreparada, partidaTerminou, receitaMedida, type Acao, type Fala } from "./lib/conversa";
  import { JOGAR, montarCartao, type Cartao, type Terminada } from "./lib/partida";
  import {
    contaQueRoda,
    noTauri,
    ponte,
    type ContaQueRoda,
    type EstadoDaLicenca,
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
  import {
    CHAVE_DA_IMAGEM,
    CHAVE_DO_AJUSTE,
    CHAVE_DO_FUNDO,
    lerAjusteGuardado,
    lerFundoGuardado,
    opcaoDo,
    type AjusteDoFundo,
    type Fundo,
  } from "./lib/fundos";
  import PapelDeFundo from "./componentes/PapelDeFundo.svelte";
  import CeuDePoeira from "./componentes/marca/CeuDePoeira.svelte";
  import Logo from "./componentes/marca/Logo.svelte";
  import MenuDoFundo from "./componentes/MenuDoFundo.svelte";

  type Tema = "claro" | "escuro";
  const CHAVE_DO_TEMA = "otimiza.tema";
  /** Abaixo disto livre ao tocar em Jogar, o Oti avisa: FiveM e jogos atuais pedem uns 3 GB a mais na partida. */
  const MEMORIA_LIVRE_MINIMA_GB = 3;
  /** A apresentação já foi vista nesta máquina (ou pulada): não abre de novo sozinha. */
  const CHAVE_DAS_BOAS_VINDAS = "otimiza.boas-vindas";

  // Onde a pessoa está: a área aberta, a aba do trilho e a última área de cada aba (`estado/navegacao.svelte.ts`).
  const nav = new Navegacao();

  let versao = $state<VersaoDoApp | null>(null);
  let hardware = $state<Hardware | null>(null);
  let metricas = $state<MetricasAoVivo | null>(null);
  /** As últimas leituras do processador (uma a cada 2 s): o gráfico vivo do bloco Agora. */
  let historicoCpu = $state<number[]>([]);
  const curvaCpu = $derived.by(() => {
    const v = historicoCpu;
    if (v.length < 3) return null;
    // A curva ocupa a largura toda desde as primeiras leituras e vai ficando mais fina até juntar 2 minutos.
    const passo = 120 / (v.length - 1);
    const x0 = 0;
    // O teto acompanha o maior valor (no mínimo 25%): com o PC parado a 15% a curva ainda mostra o que mexe.
    const teto = Math.min(100, Math.max(25, Math.max(...v) * 1.3));
    const pts = v.map((c, i) => `${(x0 + i * passo).toFixed(2)},${(34 - (Math.max(0, Math.min(teto, c)) / teto) * 30).toFixed(2)}`);
    const linha = "M" + pts.join(" L");
    return { linha, area: `${linha} L120,36 L${x0.toFixed(2)},36 Z` };
  });
  let jogo = $state<JogoAberto | null>(null);
  let biblioteca = $state<ResumoDoJogo[]>([]);
  let instalados = $state<{ jogo: JogoDaLoja; capa: string | null }[]>([]);
  let banda = $state<PainelDaBanda | null>(null);
  let varredura = $state<PainelDaVarredura | null>(null);
  let wifi = $state<DiagnosticoDoWifi | null>(null);
  let mudancas = $state<number | null>(null);
  let tema = $state<Tema>(lerTema());
  /** A imagem que a pessoa escolheu para o fundo (JPEG reduzido, em data URL), guardada só neste PC. */
  let imagemDoFundo = $state<string | null>(lerGuardado(CHAVE_DA_IMAGEM));
  /** O desfoque e o quanto escurece nas fotos. */
  let ajusteDoFundo = $state<AjusteDoFundo>(lerAjusteGuardado(lerGuardado(CHAVE_DO_AJUSTE)));
  /** O fundo atrás do vidro. "Área de trabalho": a janela fica transparente e o Windows aparece entre os cards. */
  let fundo = $state<Fundo>(lerFundoGuardado(lerGuardado(CHAVE_DO_FUNDO), lerGuardado(CHAVE_DA_IMAGEM) !== null));
  const nomeDoFundo = $derived(opcaoDo(fundo).nome);
  const fundoPapel = $derived(opcaoDo(fundo).tipo === "cena" || opcaoDo(fundo).tipo === "foto");
  const semFundo = $derived(fundo === "area");
  let menuDoFundo = $state(false);
  /** A apresentação aberta: inteira (primeira vez, ou pedida nos Ajustes) ou só o passo da licença (botão da barra). */
  let boasVindas = $state<ModoDasBoasVindas | null>(null);
  let licenca = $state<EstadoDaLicenca | null>(null);
  let busca = $state("");
  let mostrarAvisos = $state(false);

  // O Oti fala num balão que desce da busca.
  let fala = $state<Fala | null>(null);
  /** O cartão da partida que acabou de fechar (nesta abertura do Otimiza). */
  let cartao = $state<Cartao | null>(null);
  let cartaoEm = 0;
  /** O fim da partida do cartão (o executável e o caminho vão para o "Deixar liso"). */
  let terminada = $state<Terminada | null>(null);
  let lisoGravado = $state<string | null>(null);
  let gravandoLiso = $state(false);
  let digitado = $state("");
  let digitando = $state(false);
  let reacao = $state<Humor | null>(null);

  let jogoNaTela = $state(false);

  // A época do ano (Natal, Páscoa…): vem desligada (marca v2) e liga nos Ajustes ("otimiza.epoca" = "ligada"); aí muda
  // sozinha pela data.
  // `?temporada=natal` na URL mostra uma época fora da data, para conferir o visual.
  const CHAVE_DA_EPOCA = "otimiza.epoca";
  const previa = porId(new URLSearchParams(location.search).get("temporada"));
  let hoje = $state(new Date());
  let epocaLigada = $state(lerEpoca());
  const temporada = $derived(previa ?? (epocaLigada ? temporadaDe(hoje) : null));

  function lerEpoca() {
    try {
      return localStorage.getItem("otimiza.epoca") === "ligada";
    } catch {
      return false;
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
  /** Sem conversa, o Oti fica com a cara da última partida (Plano de refino 2.3.5): feliz acima da faixa das anteriores,
   *  preocupado abaixo. O ícone da bandeja acompanha (logo abaixo). */
  const humorDaPartida = $derived<Humor>(
    cartao?.medido && cartao.comparacao
      ? cartao.comparacao.sentido === "acima"
        ? "feliz"
        : cartao.comparacao.sentido === "abaixo"
          ? "preocupado"
          : "atento"
      : "atento",
  );
  const humor = $derived<Humor>(reacao ?? (jogo ? "jogando" : digitando ? "falando" : (fala?.humor ?? humorDaPartida)));
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
  /** A capa da última partida depois que a arte da loja falhou (a de reserva, ou nenhuma). */
  let reservaDaUltima = $state<string | null>(null);
  /** O "player": o jogo aberto, se houver; senão a última partida. Os números só aparecem se forem DESSE jogo. */
  const jogoDoPlayer = $derived(jogo?.nome ?? ultima?.nome ?? null);
  const partidaDoPlayer = $derived(ultima && ultima.nome === jogoDoPlayer ? ultima : null);
  const capaDaUltima = $derived(
    jogoDoPlayer
      ? (instalados.find((i) => i.jogo.nome === jogoDoPlayer)?.capa ?? (semFoto.has(jogoDoPlayer) ? reservaDaUltima : capaRemota(jogoDoPlayer, null)))
      : null
  );
  /** A arte da loja não carregou (sem internet, CDN fora): vai para a capa de reserva e, sem ela, para as iniciais. */
  function fotoFalhou(nome: string) {
    if (nome === jogoDoPlayer) {
      reservaDaUltima = capaDepoisDaFalha(nome, capaDaUltima);
      semFoto = new Set(semFoto).add(nome);
    }
    instalados = instalados.map((i) => (i.jogo.nome === nome ? { ...i, capa: capaDepoisDaFalha(nome, i.capa) } : i));
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
  /** "Pronto pra jogar": o que o Otimiza sabe conferir, item a item (`Inicio.svelte`, `ItemDaProntidao`). */
  const prontidao = $derived.by(() => {
    const itens: ItemDaProntidao[] = [];
    const t = telaPrincipal;
    if (t) {
      const max = t.hz_atual >= t.hz_maximo;
      itens.push({ id: "tela", ok: max, texto: max ? `Monitor a ${t.hz_atual} Hz` : `Monitor ${t.hz_atual} de ${t.hz_maximo} Hz`, acao: () => abrir("tela") });
    }
    // Os dois interruptores do "Durante o jogo": o item desligado liga com um toque (o mesmo do bloco).
    if (banda && (banda.ligado || banda.impedimento === null))
      itens.push({ id: "banda", ok: banda.ligado, texto: banda.ligado ? "Downloads segurados" : "Downloads soltos", acao: alternarBanda, botao: banda.ligado ? undefined : "Segurar" });
    if (wifi?.ligacao.tipo === "wi_fi" && varredura)
      itens.push({ id: "wifi", ok: varredura.ligado, texto: varredura.ligado ? "Wi-Fi sem varredura" : "Wi-Fi varrendo redes", acao: alternarVarredura, botao: varredura.ligado ? undefined : "Ligar" });
    if (mudancas != null) itens.push({ id: "otimizado", ok: mudancas > 0, texto: mudancas > 0 ? `${mudancas} ajustes ativos` : "Ainda não otimizado", acao: () => abrir(mudancas ? "historico" : "turbo") });
    // O aviso do monitor já é o primeiro item; aqui entram os outros.
    const avisoSerio = lista.find((a) => !a.id.startsWith("hz-"));
    itens.push({ id: "avisos", ok: !avisoSerio, texto: avisoSerio ? avisoSerio.titulo : "Nenhum aviso", acao: () => (avisoSerio ? abrir(areaDoAviso(avisoSerio.id)) : undefined) });
    const ok = itens.filter((i) => i.ok).length;
    return { itens, ok, total: itens.length, fracao: itens.length ? ok / itens.length : 0 };
  });
  const saudacaoDaHora = $derived.by(() => {
    const h = hoje.getHours();
    return h < 5 ? "Boa madrugada" : h < 12 ? "Bom dia" : h < 18 ? "Boa tarde" : "Boa noite";
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
  /** O que a área aberta já sabe, para o topo dela (`lib/contexto.ts`). */
  const contexto = $derived(
    contextoDa(nav.aberta, {
      tela: telaPrincipal ? { hz: telaPrincipal.hz_atual, aguenta: telaPrincipal.hz_maximo } : null,
      ligacao:
        wifi?.ligacao.tipo === "cabo"
          ? { tipo: "cabo" }
          : wifi?.ligacao.tipo === "wi_fi"
            ? { tipo: "wifi", sinal: sinalWifi }
            : null,
      memoriaPct,
      mudancas,
      jogosMedidos: biblioteca.length,
      licenca: licenca ? { ativa: licenca.ativa, expira: licenca.expira ?? null } : null,
      jogoAberto: jogo?.nome ?? null,
    }),
  );

  function lerTema(): Tema {
    try {
      // Quem nunca escolheu começa no escuro: o grafite é a cara do Otimiza.
      return localStorage.getItem(CHAVE_DO_TEMA) === "claro" ? "claro" : "escuro";
    } catch {
      return "escuro";
    }
  }

  function lerGuardado(chave: string): string | null {
    try {
      return localStorage.getItem(chave);
    } catch {
      return null;
    }
  }

  function escolherFundo(f: Fundo) {
    fundo = f;
    // As fotos pedem o ajuste à mão: o menu fica aberto nelas. Os outros fecham na hora.
    if (opcaoDo(f).tipo !== "foto") menuDoFundo = false;
    try {
      localStorage.setItem(CHAVE_DO_FUNDO, f);
    } catch {
      // Só não lembra na próxima abertura.
    }
  }

  /** Guarda (ou tira) a imagem do fundo. Falso quando não coube no armazenamento: a de antes continua. */
  function guardarImagemDoFundo(dados: string | null): boolean {
    try {
      if (dados) localStorage.setItem(CHAVE_DA_IMAGEM, dados);
      else localStorage.removeItem(CHAVE_DA_IMAGEM);
    } catch {
      return false;
    }
    imagemDoFundo = dados;
    return true;
  }

  function mudarAjusteDoFundo(a: AjusteDoFundo) {
    ajusteDoFundo = a;
    try {
      localStorage.setItem(CHAVE_DO_AJUSTE, JSON.stringify(a));
    } catch {
      // Vale até fechar o app.
    }
  }

  function abrirBoasVindas(modo: ModoDasBoasVindas) {
    boasVindas = modo;
    void ponte.licenca().then((l) => (licenca = l)).catch(() => {});
  }

  function fecharBoasVindas() {
    boasVindas = null;
    try {
      localStorage.setItem(CHAVE_DAS_BOAS_VINDAS, "vista");
    } catch {
      // Abre de novo na próxima vez; não atrapalha.
    }
  }

  /** Setas no Claro/Escuro (grupo de rádio): trocam o tema e levam o foco junto. */
  function teclaDoTema(e: KeyboardEvent) {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) return;
    e.preventDefault();
    const outro: Tema = tema === "claro" ? "escuro" : "claro";
    escolherTema(outro);
    (e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[aria-checked="false"]`)?.focus();
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
    document.documentElement.classList.toggle("janela-transparente", semFundo);
    document.documentElement.classList.toggle("fundo-limpo", fundo === "limpo");
    document.documentElement.classList.toggle("fundo-papel", fundoPapel);
    document.documentElement.dataset.fundo = fundo;
    // A época na raiz: os tokens (cor da marca, filete dos cartões, geada do Natal) mudam o app inteiro.
    if (temporada) document.documentElement.dataset.epoca = temporada.id;
    else delete document.documentElement.dataset.epoca;
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

  /** "Jogar": o Rust abre o jogo e liga a receita lembrada quando ele aparecer (`partida:preparada`). */
  async function jogarJogo(nome: string) {
    try {
      const l = await ponte.jogar(nome);
      // Memória apertada antes de abrir é o tranco de carregar cenário no meio da partida (Partida de respeito, B5).
      const total = metricas?.ram.total_gb ?? null;
      const usada = metricas?.ram.used_gb ?? null;
      const livre = total != null && usada != null ? total - usada : null;
      const apertada = livre != null && livre < MEMORIA_LIVRE_MINIMA_GB;
      dizer(
        {
          texto:
            l.caminho === "Site"
              ? `Abrindo o ${l.jogo}: escolha o jogo na página e entre.`
              : `Abrindo o ${l.jogo}${l.caminho === "Steam" ? " pela Steam" : l.caminho === "Epic" ? " pela Epic" : ""}…`,
          detalhe: apertada
            ? `Só ${livre!.toLocaleString("pt-BR", { maximumFractionDigits: 1 })} GB de memória livre: o jogo pode travar ao carregar cenário. Feche o navegador e o que não for usar antes de entrar.`
            : "Quando ele aparecer, eu ligo a receita dele.",
          humor: apertada ? "preocupado" : "jogando",
          acoes: apertada ? [{ rotulo: "Ver quem está pesando", tipo: "falar", fala: () => { abrir("sistema", "agora"); return { texto: "Esses são os que mais ocupam agora.", humor: "atento", acoes: [] }; } }] : [],
        },
        apertada ? 0 : 5000,
      );
      if (!noTauri) jogo = await ponte.jogoAberto();
    } catch (e) {
      dizer({ texto: `Não consegui abrir o ${nome}.`, detalhe: String(e), humor: "preocupado", acoes: [] });
    }
  }

  /** "Deixar liso": o teto calculado pela partida que acabou, para a próxima abertura do jogo. */
  async function deixarLiso() {
    if (!cartao?.medido || !terminada || cartao.fps == null || cartao.trancosPorMinuto == null) return;
    gravandoLiso = true;
    try {
      const r = await ponte.jogoLisoDepoisDaPartida(terminada.executavel, terminada.caminho ?? null, cartao.fps, cartao.trancosPorMinuto);
      lisoGravado = r.teto ? `Teto de ${r.teto} FPS gravado: vale na próxima vez que o jogo abrir.` : r.motivo;
      dizer({ texto: r.teto ? `Pronto: ${cartao.jogo} travado em ${r.teto} FPS.` : "Não precisou de teto.", detalhe: r.motivo, humor: "feliz", acoes: [] }, 9000);
    } catch (e) {
      lisoGravado = null;
      dizer({ texto: "Não consegui gravar o teto.", detalhe: String(e), humor: "preocupado", acoes: [] });
    } finally {
      gravandoLiso = false;
    }
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
      // O cartão da partida (`partida:terminou`) diz mais; a frase genérica só se ele não vier.
      setTimeout(() => {
        if (Date.now() - cartaoEm > 10000) dizer(jogoFechou(antes));
      }, 2500);
    }
  });

  // ---------------------------------------------------------------------------------------------------------------
  // Navegação, abas e blocos

  function abrirGrupo(g: Grupo) {
    abrir(nav.areaDoGrupo(g));
  }

  function abrir(id: IdDaArea, abaInterna: AbaDoSistema = "ajustes") {
    nav.abrir(id, abaInterna);
    // Navegar fecha o balão, a não ser que a navegação tenha vindo da pergunta (ela some sozinha depois).
    if (!digitando) fala = null;
    mostrarAvisos = false;
  }

  /** A troca de área: sobe 6 px e aparece, sem desfoque nem escala (movimento sóbrio, 1.0.12 Frente 1.6). Parada com
   *  jogo aberto e com "reduzir movimento". */
  function entrar(_no: HTMLElement) {
    const parado = jogoNaTela || window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    return {
      duration: parado ? 0 : 280,
      easing: cubicOut,
      css: (t: number, u: number) => `opacity: ${t}; transform: translateY(${u * 6}px)`,
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

  // ---------------------------------------------------------------------------------------------------------------

  function pct(v: number | null | undefined) {
    return v == null ? "—" : `${Math.round(v)}%`;
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
      if (!jogo && !document.hidden) {
        const lido = await ponte.metricas().catch(() => null);
        if (lido) {
          metricas = lido;
          if (lido.cpu.overall != null) historicoCpu = [...historicoCpu.slice(-59), lido.cpu.overall];
        }
      }
      espera = setTimeout(laco, jogo ? 5000 : document.hidden ? 4000 : 2000);
    };
    void laco();

    // A data muda à meia-noite (e a época junto); conferir de hora em hora basta.
    const relogioDaEpoca = setInterval(() => (hoje = new Date()), 60 * 60 * 1000);
    const aoMudarEpoca = () => (epocaLigada = lerEpoca());
    window.addEventListener(CHAVE_DA_EPOCA, aoMudarEpoca);

    // Guarda térmica: o Oti avisa quando o PC começa a ser freado por calor no jogo.
    let deixarCalor: (() => void) | undefined;
    let deixarPartida: (() => void)[] = [];
    // A licença, para o botão "Ativar licença" da barra, e a apresentação na primeira abertura (`?boasvindas` força).
    void ponte.licenca().then((l) => (licenca = l)).catch(() => {});
    let vista = false;
    try {
      vista = localStorage.getItem(CHAVE_DAS_BOAS_VINDAS) === "vista";
    } catch {
      vista = false;
    }
    if (!vista || new URLSearchParams(location.search).has("boasvindas")) boasVindas = "completa";
    const pediuBoasVindas = (e: Event) => abrirBoasVindas(((e as CustomEvent).detail as ModoDasBoasVindas) ?? "completa");
    window.addEventListener(ABRIR_BOAS_VINDAS, pediuBoasVindas);
    const pediuJogar = (e: Event) => void jogarJogo(String((e as CustomEvent).detail));
    window.addEventListener(JOGAR, pediuJogar);
    void Promise.all([
      ponte.aoPrepararPartida((p) => dizer(partidaPreparada(p), p.recusa || p.sem_receita ? 0 : 8000)),
      ponte.aoMedirReceita((r) => dizer(receitaMedida(r), 12000)),
      ponte.aoAvisoDeEstabilidade((a) =>
        dizer({
          texto: a.desfeitos.length ? "Desfiz um ajuste que deixou o PC instável." : "O PC caiu depois de um ajuste.",
          detalhe: [a.frase, a.desfeitos.length ? `Desfeito: ${a.desfeitos.join(", ")}.` : "", a.falhas.length ? `Não consegui desfazer: ${a.falhas.join("; ")}.` : ""]
            .filter(Boolean)
            .join(" "),
          humor: "preocupado",
          acoes: [{ rotulo: "Ver o histórico", tipo: "abrir", area: "historico", principal: true }],
        }),
      ),
      ponte.aoTerminarPartida(async (t) => {
        const r = await ponte.relatorioDaPartida().catch(() => null);
        cartao = montarCartao(t, r, Date.now() / 1000);
        terminada = t;
        lisoGravado = null;
        cartaoEm = Date.now();
        dizer(partidaTerminou(cartao));
      }),
    ]).then((d) => {
      if (vivo) deixarPartida = d;
      else d.forEach((f) => f());
    });
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

    const pediuArea = (e: Event) => abrir((e as CustomEvent<IdDaArea>).detail);
    window.addEventListener(ABRIR_AREA, pediuArea);
    return () => {
      window.removeEventListener(ABRIR_AREA, pediuArea);
      vivo = false;
      deixarCalor?.();
      deixarPartida.forEach((f) => f());
      window.removeEventListener(JOGAR, pediuJogar);
      window.removeEventListener(ABRIR_BOAS_VINDAS, pediuBoasVindas);
      clearTimeout(espera);
      clearInterval(maquina);
      window.removeEventListener(CHAVE_DA_EPOCA, aoMudarEpoca);
      clearInterval(relogioDaEpoca);
    };
  });
</script>

<!-- O fundo "Limpo" da marca v2: preto (papel no claro) com poeira de luz para onde o Oti olha. -->
{#if fundo === "limpo"}<CeuDePoeira />{/if}
<!-- O cômodo desfocado (fundo "Cômodo"). Feito em CSS: parede, janela com luz, chão, sofá e luminária. -->
<div class="comodo" aria-hidden="true">
  <div class="parede"></div>
  <div class="janela-luz"></div>
  <div class="chao"></div>
  <div class="sofa"></div>
  <div class="luminaria"><span></span></div>
  <div class="mesa"></div>
</div>
<PapelDeFundo {fundo} imagem={imagemDoFundo} ajuste={ajusteDoFundo} />

{#if temporada}
  <EfeitoDeEpoca efeito={temporada.efeito} />
{/if}

<div class="casca">
  <!-- Barra de cima -->
  <header class="topo" data-tauri-drag-region>
    <div class="marca-do-topo" data-tauri-drag-region>
      <Logo altura={19} />
      <span class="versao" data-tauri-drag-region>{versao?.exibida ?? ""}{versao && !versao.oficial ? " · compilação local" : ""}{hardware ? ` · ${nomeCurto(hardware.processador)}` : ""}</span>
    </div>

    <form class="busca" onsubmit={perguntar}>
      <span class="orbe"><Oti {humor} tamanho={20} segueCursor rotulo="" /></span>
      <input bind:value={busca} placeholder="Pergunte ao Oti… por que meu ping pula?" aria-label="Pergunte ao Oti" />
    </form>

    <div class="direita" data-tauri-drag-region>
      {#if !noTauri}
        <button class="link-demo" onclick={alternarJogoDeDemonstracao}>{jogo ? "Fechar jogo de teste" : "Simular jogo"}</button>
      {/if}
      <!-- svelte-ignore a11y_interactive_supports_focus -->
      <div class="segmento tema" role="radiogroup" aria-label="Tema" onkeydown={teclaDoTema}>
        <button role="radio" aria-checked={tema === "claro"} tabindex={tema === "claro" ? 0 : -1} onclick={() => escolherTema("claro")}>
          <Icone nome="sol" tamanho={14} /> Claro
        </button>
        <button role="radio" aria-checked={tema === "escuro"} tabindex={tema === "escuro" ? 0 : -1} onclick={() => escolherTema("escuro")}>
          <Icone nome="lua" tamanho={14} /> Escuro
        </button>
      </div>
      {#if licenca && !licenca.ativa}
        <button class="ativar-botao" onclick={() => abrirBoasVindas("licenca")}>Ativar licença</button>
      {/if}
      <div class="fundo-escolha">
        <button class="fundo-botao" aria-haspopup="menu" aria-expanded={menuDoFundo} title="O que fica atrás do painel" onclick={() => (menuDoFundo = !menuDoFundo)}>
          Fundo: {nomeDoFundo}
        </button>
        {#if menuDoFundo}
          <div in:entrar>
            <MenuDoFundo
              {fundo}
              imagem={imagemDoFundo}
              ajuste={ajusteDoFundo}
              onescolher={escolherFundo}
              onimagem={guardarImagemDoFundo}
              onajuste={mudarAjusteDoFundo}
              onfechar={() => (menuDoFundo = false)}
            />
          </div>
        {/if}
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
    <div class="balao superficie-flutuante" role="status" in:entrar>
      <span class="balao-oti"><Oti {humor} tamanho={40} rotulo="" /></span>
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
    <div class="avisos superficie-flutuante" in:entrar>
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
  <nav class="trilho" aria-label="Seções">
    <Trilho ativo={nav.grupoAtivo.id} onabrir={abrirGrupo} />
  </nav>

  <!-- Painel -->
  <main class="painel" class:inicio={nav.aberta === "inicio"}>
    {#key nav.aberta}
      <div id="painel-da-area" class="conteudo" role="tabpanel" aria-labelledby={`areas-aba-${nav.aberta}`} in:entrar>
        {#if nav.aberta === "inicio"}
          <Inicio
            {temporada}
            {humor}
            {prontidao}
            {saudacaoDaHora}
            {jogo}
            {jogoDoPlayer}
            {partidaDoPlayer}
            {mudancas}
            {instalados}
            {capaDaUltima}
            {cartao}
            {lisoGravado}
            {gravandoLiso}
            {hardware}
            {medidores}
            {curvaCpu}
            {banda}
            {varredura}
            {telaPrincipal}
            onabrir={abrir}
            onjogar={jogarJogo}
            onfotoFalhou={fotoFalhou}
            ondeixarLiso={deixarLiso}
            onalternarBanda={alternarBanda}
            onalternarVarredura={alternarVarredura}
          />
        {:else}
          <div class="area-aberta">
            <header class="area-topo">
              <div class="area-nome">
                <h2>{nav.area.nome}</h2>
                {#if nav.area.pergunta}<p class="sec">{nav.area.pergunta}</p>{/if}
              </div>
              {#if contexto}<p class="contexto"><span class="ponto" data-tom={contexto.tom}></span>{contexto.texto}</p>{/if}
            </header>
            <div class="area-corpo">
              {#if nav.aberta === "historico"}
                <Historico />
              {:else if nav.aberta === "rede"}
                <Rede />
              {:else if nav.aberta === "latencia"}
                <Latencia />
              {:else if nav.aberta === "tela"}
                <Tela aoMudarHz={trocouHz} />
              {:else if nav.aberta === "jogos"}
                <Jogos />
              {:else if nav.aberta === "ajustes"}
                <Ajustes />
              {:else if nav.aberta === "mira"}
                <Mira />
              {:else if nav.aberta === "cores"}
                <Cores />
              {:else if nav.aberta === "prova"}
                <Prova jogo={jogo?.nome ?? null} />
              {:else if nav.aberta === "sistema"}
                <Sistema abaInicial={nav.abaDoSistema} />
              {:else if nav.aberta === "energia"}
                <Energia />
              {:else if nav.aberta === "turbo"}
                <Turbo jogo={jogo?.nome ?? null} />
              {:else}
                <div class="em-breve">
                  <Oti humor="dormindo" tamanho={64} rotulo="" />
                  <p>{nav.area.nome} chega na Onda {nav.area.onda}. Até lá, ela continua no Otimiza de sempre.</p>
                  <ul class="lista">
                    {#each nav.area.vai_ter as item (item)}<li>{item}</li>{/each}
                  </ul>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/key}
  </main>

  <!-- As áreas da aba ativa -->
  <nav class="abas superficie-flutuante" aria-label={`Áreas de ${nav.grupoAtivo.nome}`}>
    <BarraDeAbas grupo={nav.grupoAtivo} aberta={nav.aberta} onabrir={(id) => abrir(id)} />
  </nav>

  <PedidoDeFeedback />
  <Confirmar />

  {#if boasVindas}
    <BoasVindas
      modo={boasVindas}
      {hardware}
      {licenca}
      {tema}
      onescolherTema={escolherTema}
      onlicenca={(l) => (licenca = l)}
      onfim={fecharBoasVindas}
    />
  {/if}

  {#if jogoNaTela && jogo}
    <p class="pausado" role="status"><span class="ponto" data-tom="ok"></span>{jogo.nome} aberto · a interface parou de se mexer para não pesar no jogo</p>
  {/if}
</div>

<style>
  /* ---------- O cômodo (fundo "Cômodo"): só aparece quando escolhido ---------- */
  .comodo {
    position: fixed;
    inset: -40px;
    z-index: 0;
    display: none;
    filter: blur(16px) saturate(0.4);
    background: #c9c3bb;
  }
  :global(:root[data-fundo="comodo"]) .comodo {
    display: block;
  }
  :global(:root[data-tema="noite"]) .comodo {
    filter: blur(16px) brightness(0.32) saturate(0.3);
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
    background: linear-gradient(90deg, transparent 6%, #9a7a58 6% 8%, transparent 8% 92%, #9a7a58 92% 94%, transparent 94%);
  }

  /* ---------- A casca ----------
     Marca v2: fundo preto (papel no claro), superfícies sólidas em grafite com borda fina e o fio de luz no alto. Sem
     vidro: o desfoque só volta quando há uma foto ou uma cena atrás, e para com o jogo aberto. */
  .casca {
    position: relative;
    z-index: 1;
    height: 100vh;
    color: var(--tinta);
    font-family: var(--fonte-texto);
    --superficie-casca: var(--superficie);
  }
  :global(:root.fundo-papel) .casca,
  :global(:root[data-fundo="comodo"]) .casca {
    --superficie-casca: color-mix(in srgb, var(--superficie) 86%, transparent);
  }
  :global(:root.janela-transparente) .casca {
    --superficie-casca: color-mix(in srgb, var(--superficie) 96%, transparent);
  }
  .superficie-flutuante {
    border: 1px solid var(--linha);
    background: var(--superficie-casca);
    box-shadow: var(--fio), var(--sombra-2);
  }
  :global(:root.fundo-papel) .superficie-flutuante,
  :global(:root[data-fundo="comodo"]) .superficie-flutuante {
    backdrop-filter: var(--desfoque);
  }
  .sec {
    color: var(--tinta-2);
  }

  /* ---------- Barra de cima: o logotipo, a pergunta ao Oti e os controles. Sem caixa: é a borda da janela. ---------- */
  .topo {
    position: absolute;
    z-index: 5;
    left: 0;
    right: 0;
    top: 0;
    height: 64px;
    display: flex;
    align-items: center;
    gap: var(--e-4);
    padding: 0 var(--e-3) 0 28px;
  }
  .marca-do-topo {
    display: flex;
    align-items: center;
    gap: var(--e-3);
    min-width: 0;
    white-space: nowrap;
  }
  .versao {
    padding-top: 3px;
    color: var(--tinta-3);
    font-size: var(--t-apoio);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .busca {
    flex: 1;
    max-width: 440px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    padding: 0 14px 0 10px;
    border: 1px solid var(--linha);
    border-radius: var(--raio-pilula);
    background: var(--superficie-casca);
    box-shadow: var(--fio);
    transition: border-color var(--mov-rapido) ease;
  }
  .busca:hover {
    border-color: var(--linha-forte);
  }
  .busca:focus-within {
    border-color: var(--foco);
  }
  .orbe {
    display: grid;
    place-items: center;
  }
  .busca input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: none;
    color: var(--tinta);
    font-size: var(--t-linha);
    user-select: text;
  }
  .busca input::placeholder {
    color: var(--tinta-3);
  }
  .direita {
    display: flex;
    align-items: center;
    gap: var(--e-2);
  }
  .link-demo {
    border: 0;
    background: none;
    color: var(--tinta-3);
    font-size: var(--t-apoio);
    cursor: pointer;
  }
  .link-demo:hover {
    color: var(--tinta);
  }
  /* Sem licença, o caminho para ativar fica à vista na barra: escondido nos Ajustes, ninguém achava. */
  .ativar-botao {
    height: 34px;
    padding: 0 var(--e-4);
    border: 0;
    border-radius: var(--raio-pilula);
    background: var(--acao);
    color: var(--contra);
    font-size: var(--t-linha);
    font-weight: 600;
    cursor: pointer;
  }
  .ativar-botao:hover {
    background: var(--acao-forte);
  }
  .fundo-botao {
    height: 34px;
    padding: 0 var(--e-3);
    border: 1px solid var(--linha);
    border-radius: var(--raio-pilula);
    background: transparent;
    color: var(--tinta-2);
    font-size: var(--t-linha);
    cursor: pointer;
  }
  .fundo-botao:hover {
    border-color: var(--linha-forte);
    color: var(--tinta);
  }
  .fundo-escolha {
    position: relative;
  }
  .segmento.tema {
    border-radius: var(--raio-pilula);
  }
  .segmento.tema button {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    border-radius: var(--raio-pilula);
    font-size: var(--t-apoio);
  }
  .redondo {
    position: relative;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border: 1px solid var(--linha);
    border-radius: 50%;
    background: transparent;
    color: var(--tinta-2);
    cursor: pointer;
  }
  .redondo:hover {
    border-color: var(--linha-forte);
    color: var(--tinta);
  }
  /* O número de avisos: o único lugar da barra com cor, porque é estado. */
  .contador {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 17px;
    height: 17px;
    padding: 0 4px;
    border-radius: 9px;
    background: var(--atencao);
    color: var(--contra);
    font-size: var(--t-apoio);
    font-weight: 700;
    line-height: 17px;
    box-shadow: 0 0 0 2px var(--fundo);
  }
  .controles {
    display: flex;
    margin-left: var(--e-1);
  }
  .controles button {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--raio-1);
    background: transparent;
    color: var(--tinta-3);
    cursor: pointer;
  }
  .controles svg,
  .fechar-balao svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
  }
  .controles button:hover {
    background: var(--acao-suave);
    color: var(--tinta);
  }
  /* Fechar a janela: o vermelho é estado ("isto fecha"), não enfeite. */
  .controles .x:hover {
    background: var(--perigo);
    color: var(--contra);
  }

  /* ---------- Balão do Oti e avisos ---------- */
  .balao {
    position: absolute;
    z-index: 20;
    left: 50%;
    top: 68px;
    width: min(520px, calc(100% - 48px));
    margin-left: calc(min(520px, calc(100% - 48px)) / -2);
    display: flex;
    gap: var(--e-3);
    padding: var(--e-4);
    border-radius: var(--raio-4);
    border-color: var(--linha-forte);
    background: var(--superficie-menu);
    box-shadow: var(--fio), var(--sombra-3);
  }
  .balao-oti {
    flex: none;
    padding-top: 2px;
  }
  .balao-texto {
    flex: 1;
    display: grid;
    gap: 6px;
  }
  .balao-fala {
    min-height: 1.4em;
    font-family: var(--fonte-titulo);
    font-size: var(--t-subtitulo);
    font-weight: 600;
    letter-spacing: var(--rastreio-subtitulo);
    line-height: 1.35;
  }
  .balao-detalhe {
    color: var(--tinta-2);
    font-size: var(--t-linha);
    line-height: 1.5;
  }
  .balao-acoes {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: var(--e-1);
  }
  .pilula {
    height: 32px;
    padding: 0 14px;
    border: 1px solid var(--linha-forte);
    border-radius: var(--raio-pilula);
    background: transparent;
    color: var(--tinta);
    font-size: var(--t-linha);
    font-weight: 500;
    cursor: pointer;
  }
  .pilula:hover {
    background: var(--acao-suave);
  }
  .pilula.principal {
    border-color: transparent;
    background: var(--acao);
    color: var(--contra);
    font-weight: 600;
  }
  .fechar-balao {
    align-self: flex-start;
    flex: none;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--tinta-3);
    cursor: pointer;
  }
  .fechar-balao:hover {
    background: var(--acao-suave);
    color: var(--tinta);
  }
  .avisos {
    position: absolute;
    z-index: 21;
    right: 70px;
    top: 60px;
    width: 340px;
    display: grid;
    gap: var(--e-1);
    padding: var(--e-3);
    border-radius: var(--raio-3);
    background: var(--superficie-menu);
    box-shadow: var(--fio), var(--sombra-3);
  }
  .avisos h3 {
    margin: 0 0 var(--e-1);
    padding: var(--e-1) var(--e-2) 0;
    font-size: var(--t-rotulo);
    font-weight: 500;
    letter-spacing: var(--rastreio-rotulo);
    text-transform: uppercase;
    color: var(--tinta-2);
  }
  .avisos > .sec {
    padding: 0 var(--e-2) var(--e-2);
    font-size: var(--t-linha);
  }
  .aviso {
    display: flex;
    gap: 10px;
    align-items: baseline;
    padding: var(--e-2);
    border: 0;
    border-radius: var(--raio-2);
    background: transparent;
    color: var(--tinta);
    text-align: left;
    font-size: var(--t-linha);
    cursor: pointer;
  }
  .aviso:hover {
    background: var(--acao-suave);
  }

  /* ---------- Trilho: a coluna da esquerda, sem caixa ---------- */
  .trilho {
    position: absolute;
    left: 14px;
    top: 80px;
  }

  /* ---------- Painel: as áreas ficam direto no fundo; cada seção é o seu cartão (`tokens.css`) ---------- */
  .painel {
    position: absolute;
    left: 92px;
    right: 16px;
    top: 64px;
    bottom: 76px;
  }
  .conteudo {
    height: 100%;
    overflow: auto;
    /* O espaço da barra fica reservado: a página não pula de lado quando uma sub-aba é curta e a outra rola. */
    scrollbar-gutter: stable;
    padding: var(--e-4) var(--e-3) var(--e-5);
  }

  /* ---------- Área aberta ---------- */
  .area-aberta {
    display: grid;
    gap: var(--e-4);
    max-width: 1040px;
    margin: 0 auto;
    padding: var(--e-2) var(--e-2) var(--e-5);
  }
  /* O cabeçalho da área: o nome grande, a pergunta embaixo e o estado lido à direita. */
  .area-topo {
    display: flex;
    align-items: flex-end;
    gap: var(--e-4);
    padding: var(--e-1) var(--e-1) var(--e-2);
  }
  .area-nome {
    flex: 1;
    min-width: 0;
  }
  .area-topo h2 {
    font-family: var(--fonte-titulo);
    font-size: var(--t-titulo-area);
    font-weight: 700;
    letter-spacing: var(--rastreio-titulo);
    line-height: 1.1;
  }
  .area-topo .area-nome .sec {
    margin-top: var(--e-1);
    font-size: var(--t-corpo);
  }
  .area-topo .contexto {
    display: flex;
    align-items: center;
    gap: var(--e-2);
    flex: none;
    max-width: 46%;
    margin: 0;
    padding: 6px 14px;
    border: 1px solid var(--linha);
    border-radius: var(--raio-pilula);
    background: var(--superficie-casca);
    font-size: var(--t-linha);
    font-weight: 500;
    color: var(--tinta);
  }
  .area-corpo {
    display: grid;
    gap: var(--e-3);
  }
  .em-breve {
    display: grid;
    justify-items: start;
    gap: var(--e-3);
    padding: var(--e-4) 0;
  }

  /* ---------- Abas de baixo ---------- */
  .abas {
    position: absolute;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    padding: 4px;
    border-radius: var(--raio-pilula);
  }

  .pausado {
    position: absolute;
    left: 50%;
    bottom: 70px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: var(--e-2);
    margin: 0;
    padding: 4px 12px;
    border: 1px solid var(--linha);
    border-radius: var(--raio-pilula);
    background: var(--superficie);
    color: var(--tinta-2);
    font-size: var(--t-apoio);
  }

  /* Janela estreita: a versão sai da barra. */
  @media (max-width: 1100px) {
    .versao {
      display: none;
    }
  }
</style>
