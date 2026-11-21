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
  /** O fundo que volta quando a apresentação desliga a "área de trabalho". */
  let fundoAntes: Fundo = "limpo";
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
    if (f !== "area") fundoAntes = f;
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

  /** A apresentação só liga e desliga a área de trabalho. */
  function alternarFundo() {
    escolherFundo(fundo === "area" ? fundoAntes : "area");
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

<!-- O cômodo desfocado atrás do vidro. Feito em CSS: parede, janela com luz, chão, sofá e luminária. -->
<div class="limpo" data-epoca={temporada?.id ?? null} aria-hidden="true"></div>
<div class="comodo" data-epoca={temporada?.id ?? null} aria-hidden="true">
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
  <header class="topo vidro" data-tauri-drag-region>
    <div class="boas-vindas" data-tauri-drag-region>
      <span class="avatar"><Oti humor={humor} tamanho={30} segueCursor={false} /></span>
      <div class="ola">
        <strong>{temporada?.saudacao ?? "Bem-vindo de volta"}</strong>
        <span class="sec">Otimiza {versao?.exibida ?? ""}{versao && !versao.oficial ? " · compilação local" : ""}{hardware ? ` · ${nomeCurto(hardware.processador)}` : ""}</span>
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
    <div class="balao vidro-forte" role="status" in:entrar>
      <span class="balao-oti"><Oti {humor} tamanho={44} /></span>
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
  <nav class="trilho vidro" aria-label="Abas">
    <Trilho ativo={nav.grupoAtivo.id} onabrir={abrirGrupo} />
  </nav>

  <!-- Painel -->
  <main class="painel vidro" class:solto={nav.aberta === "inicio"}>
    {#key nav.aberta}
      <div class="conteudo" in:entrar>
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
                  <Oti humor="dormindo" tamanho={90} segueCursor={false} />
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
  <nav class="abas vidro" aria-label={`Áreas de ${nav.grupoAtivo.nome}`}>
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
      {semFundo}
      onescolherTema={escolherTema}
      onalternarFundo={alternarFundo}
      onlicenca={(l) => (licenca = l)}
      onfim={fecharBoasVindas}
    />
  {/if}

  {#if jogoNaTela && jogo}
    <p class="pausado">{jogo.nome} aberto · interface pausada para não pesar no jogo</p>
  {/if}
</div>

<style>
  /* ---------- O fundo limpo (padrão): grafite com uma luz de cima e um respiro da marca embaixo ---------- */
  .limpo {
    position: fixed;
    inset: 0;
    z-index: 0;
    display: none;
    background:
      radial-gradient(70% 55% at 50% -10%, rgb(255 255 255 / 0.07), transparent 70%),
      radial-gradient(60% 40% at 50% 115%, color-mix(in srgb, var(--marca) 14%, transparent), transparent 70%),
      linear-gradient(180deg, #121215 0%, #0b0b0d 100%);
  }
  :global(:root[data-tema="dia"]) .limpo {
    background:
      radial-gradient(70% 55% at 50% -10%, rgb(255 255 255 / 0.95), transparent 70%),
      radial-gradient(60% 40% at 50% 115%, color-mix(in srgb, var(--marca) 8%, transparent), transparent 70%),
      linear-gradient(180deg, #f3f4f7 0%, #e6e8ee 100%);
  }
  :global(:root.fundo-limpo) .limpo {
    display: block;
  }
  :global(:root.fundo-limpo) .comodo,
  :global(:root.fundo-papel) .comodo {
    display: none;
  }
  /* A luz da época também entra no fundo limpo, bem mais baixa que no cômodo (lá ela passa pelo desfoque). */
  .limpo::after {
    content: "";
    position: absolute;
    inset: 0;
    opacity: 0;
    transition: opacity 1.2s ease;
  }
  .limpo[data-epoca]::after {
    opacity: 0.55;
  }

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
  .comodo[data-epoca="natal"]::after,
  .limpo[data-epoca="natal"]::after {
    background:
      radial-gradient(ellipse 40% 50% at 12% 70%, rgb(214 47 58 / 0.35), transparent 70%),
      radial-gradient(ellipse 35% 45% at 88% 30%, rgb(34 139 84 / 0.3), transparent 70%),
      radial-gradient(ellipse 30% 30% at 60% 85%, rgb(255 210 120 / 0.35), transparent 70%);
  }
  .comodo[data-epoca="ano-novo"]::after,
  .limpo[data-epoca="ano-novo"]::after {
    background:
      radial-gradient(ellipse 45% 50% at 80% 25%, rgb(230 180 34 / 0.35), transparent 70%),
      radial-gradient(ellipse 40% 40% at 20% 80%, rgb(255 255 255 / 0.35), transparent 70%);
  }
  .comodo[data-epoca="carnaval"]::after,
  .limpo[data-epoca="carnaval"]::after {
    background:
      radial-gradient(ellipse 40% 45% at 15% 30%, rgb(122 43 214 / 0.3), transparent 70%),
      radial-gradient(ellipse 40% 45% at 85% 70%, rgb(255 79 154 / 0.3), transparent 70%),
      radial-gradient(ellipse 30% 35% at 55% 20%, rgb(46 196 182 / 0.25), transparent 70%);
  }
  .comodo[data-epoca="pascoa"]::after,
  .limpo[data-epoca="pascoa"]::after {
    background:
      radial-gradient(ellipse 45% 50% at 20% 30%, rgb(255 182 200 / 0.4), transparent 70%),
      radial-gradient(ellipse 45% 50% at 80% 75%, rgb(190 230 200 / 0.4), transparent 70%);
  }
  .comodo[data-epoca="festa-junina"]::after,
  .limpo[data-epoca="festa-junina"]::after {
    background:
      radial-gradient(ellipse 50% 45% at 50% 95%, rgb(255 140 40 / 0.4), transparent 70%),
      radial-gradient(ellipse 35% 35% at 15% 20%, rgb(250 204 21 / 0.3), transparent 70%);
  }
  .comodo[data-epoca="dia-das-criancas"]::after,
  .limpo[data-epoca="dia-das-criancas"]::after {
    background:
      radial-gradient(ellipse 40% 45% at 20% 25%, rgb(59 130 246 / 0.3), transparent 70%),
      radial-gradient(ellipse 40% 45% at 80% 75%, rgb(250 204 21 / 0.3), transparent 70%);
  }
  .comodo[data-epoca="halloween"]::after,
  .limpo[data-epoca="halloween"]::after {
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
    --bloco-solto: rgb(255 255 255 / 0.5);
    --bloco-2: rgb(255 255 255 / 0.55);
    --ativo: #ffffff;
    --ativo-texto: #1d1d1f;
    --verde: #34c759;
    /* Barra que pede atenção (memória cheia, tela abaixo do máximo). */
    --laranja: #c2730f;
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
    --bloco-solto: rgb(38 38 42 / 0.62);
    --bloco-2: rgb(255 255 255 / 0.14);
    --ativo: #f5f5f7;
    --ativo-texto: #1d1d1f;
    --laranja: #ffa53a;
  }
  /* Janela transparente: o cômodo some e a área de trabalho aparece entre os cards. O desfoque não alcança o que
     está fora da janela, então o vidro fica mais firme (e sem desfoque, que ali só gastaria placa). */
  :global(:root.janela-transparente) .comodo {
    display: none;
  }
  :global(:root.janela-transparente) .casca {
    --vidro-fundo: rgb(246 244 241 / 0.9);
    --vidro-borda: rgb(255 255 255 / 0.75);
    --bloco-solto: rgb(250 249 247 / 0.92);
  }
  :global(:root.janela-transparente[data-tema="noite"]) .casca {
    --vidro-fundo: rgb(34 34 38 / 0.92);
    --vidro-borda: rgb(255 255 255 / 0.12);
    --bloco-solto: rgb(38 38 42 / 0.94);
  }
  /* Fundo limpo: o vidro vira grafite firme (o desfoque não tem o que desfocar e só gastaria placa). */
  :global(:root.fundo-limpo) .casca {
    --vidro-fundo: rgb(250 250 252 / 0.82);
    --vidro-borda: rgb(255 255 255 / 0.9);
    --bloco: rgb(0 0 0 / 0.035);
    --bloco-solto: rgb(255 255 255 / 0.92);
    --bloco-2: rgb(0 0 0 / 0.06);
  }
  :global(:root.fundo-limpo[data-tema="noite"]) .casca {
    --vidro-fundo: rgb(24 24 27 / 0.86);
    --vidro-borda: rgb(255 255 255 / 0.08);
    --bloco: rgb(255 255 255 / 0.045);
    --bloco-solto: rgb(26 26 30 / 0.96);
    --bloco-2: rgb(255 255 255 / 0.09);
  }
  :global(:root.fundo-limpo) .vidro,
  :global(:root.fundo-limpo) .vidro-forte,
  :global(:root.janela-transparente) .vidro,
  :global(:root.janela-transparente) .vidro-forte {
    backdrop-filter: none;
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
    /* Acima do painel: o menu do Fundo abre por cima dos blocos. */
    z-index: 5;
    left: 96px;
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
    font-size: 12px;
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
  /* Sem licença, o caminho para ativar fica à vista na barra: escondido nos Ajustes, ninguém achava. */
  .ativar-botao {
    height: 36px;
    padding: 0 15px;
    border: 0;
    border-radius: 18px;
    background: var(--texto);
    color: var(--contra, #fff);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    animation: chamar 2.4s ease-in-out 3;
  }
  @keyframes chamar {
    50% {
      box-shadow: 0 0 0 6px color-mix(in srgb, var(--texto) 14%, transparent);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ativar-botao {
      animation: none;
    }
  }
  /* Liga e desliga o fundo: apertado, o fundo sumiu. */
  .fundo-botao {
    height: 36px;
    padding: 0 13px;
    border-radius: 18px;
    border: 1px solid var(--vidro-borda);
    background: var(--bloco);
    color: var(--texto);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .fundo-escolha {
    position: relative;
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
    transition: background var(--mov-medio) ease, color var(--mov-medio) ease;
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

  /* ---------- Painel e grade ---------- */
  .painel {
    position: absolute;
    left: 96px;
    right: 24px;
    top: 80px;
    bottom: 84px;
    border-radius: 32px;
    overflow: hidden;
  }
  /* Na tela inicial o painel some e só os cards ficam sobre o céu: cada um leva o próprio vidro. */
  .painel.solto {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
    backdrop-filter: none;
  }
  :global(:root.jogo-aberto) .painel.solto {
    background: transparent;
  }
  .conteudo {
    height: 100%;
    overflow: auto;
    /* O espaço da barra fica reservado: a página não pula de lado quando uma sub-aba é curta e a outra rola. */
    scrollbar-gutter: stable;
    padding: 16px;
  }


  /* ---------- Área aberta dentro do painel ---------- */
  .area-aberta {
    display: grid;
    gap: 14px;
    max-width: 1000px;
    margin: 0 auto;
    padding: 10px 14px 24px;
    /* As áreas usam os tokens de `tokens.css` (os mesmos tons da casca); aqui só as superfícies viram vidro. */
    --superficie: rgb(255 255 255 / 0.4);
    --superficie-2: rgb(255 255 255 / 0.5);
    --acao: #1d1d1f;
    --acao-forte: #000;
  }
  :global(:root[data-tema="noite"]) .area-aberta {
    --superficie: rgb(255 255 255 / 0.08);
    --superficie-2: rgb(255 255 255 / 0.12);
    --acao: #f5f5f7;
    --acao-forte: #fff;
  }
  /* O cabeçalho da área: o nome grande, a pergunta embaixo e o estado lido à direita. */
  .area-topo {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 4px 4px 6px;
  }
  .area-nome {
    flex: 1;
    min-width: 0;
  }
  .area-topo h2 {
    font-family: var(--fonte-titulo);
    font-size: 30px;
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 1.15;
  }
  .area-topo .area-nome .sec {
    margin-top: 2px;
    font-size: 14px;
  }
  .area-topo .contexto {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
    max-width: 46%;
    margin: 0;
    padding: 8px 14px;
    border-radius: 999px;
    background: var(--bloco);
    border: 1px solid var(--vidro-borda);
    font-size: 13px;
    font-weight: 500;
    color: var(--texto);
  }
  /* O corpo não é mais uma caixa: cada seção vira o seu cartão (`tokens.css`, `.area-corpo .secao`). */
  .area-corpo {
    display: grid;
    gap: 12px;
  }
  .em-breve {
    display: grid;
    justify-items: start;
    gap: 12px;
    padding: 16px 0;
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

  /* Janela estreita: a saudação sai da barra. A grade estreita fica na regra de cima ("a grade vira uma coluna
     dupla que rola"); a que morava aqui usava os blocos de um desenho antigo e escondia três cards. */
  @media (max-width: 1100px) {
    .ola {
      display: none;
    }
  }
</style>
