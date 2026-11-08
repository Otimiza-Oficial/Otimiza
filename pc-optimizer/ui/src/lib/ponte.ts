// A conversa da interface nova com o Rust. Os comandos e os tipos vêm de `tipos.ts`, gerado a partir do Rust pelo
// teste `contrato` (`src-tauri/src/contrato.rs`): um comando renomeado ou um campo trocado quebra a compilação daqui,
// não a tela do cliente. Comando novo na interface = entra na lista do `contrato.rs` primeiro.
//
// Fora do Tauri (o navegador do `npm run nova:web`) não há backend: a ponte devolve uma máquina de demonstração e a
// barra de menu diz isso. Dado de demonstração nunca aparece como se fosse do PC de quem está olhando.

import {
  commands,
  type DiagnosticoDaTela_Serialize,
  type DiagnosticoDoWifi_Serialize,
  type FimDaTroca,
  type HistoricoNaTela_Serialize,
  type JogoDaLoja_Serialize,
  type OptimizationOutcome_Serialize,
  type OptimizationInfo_Serialize,
  type StartupEntry,
  type ItemDaLimpeza,
  type ResultadoDaLimpeza,
  type RestoreStatus,
  type PainelDaBanda_Serialize,
  type PainelDaEnergiaDeJogo_Serialize,
  type PainelDoTurbo_Serialize,
  type Painel_Serialize,
  type PerfilDeHardware_Serialize,
  type Relatorio_Serialize,
  type ResultadoDaBanda_Serialize,
  type ResultadoDaCaptura_Serialize,
  type ResultadoDoLab_Serialize,
  type RespostaDoOti_Serialize,
  type TrocaComOti,
  type Preferences_Serialize,
  type EstadoDaLicenca_Serialize,
  type AvisoDeVersao_Serialize,
  type EstadoDaSobreposicao_Serialize,
  type ModoDaSobreposicao,
  type CantoDaSobreposicao,
  type LadoDoLab,
  type RelatorioDoPlano_Serialize,
  type RelatorioDeTetos_Serialize,
  type VistoriaDoPlano,
  type DiagnosticoDeEnergia,
  type ResultadoDaProva_Serialize,
  type ResumoDoJogo_Serialize,
  type Rota_Serialize,
  type HistoricoDaRede_Serialize,
  type TrocaAplicada,
  type VersaoDoApp,
} from "./tipos";

export type { Ganho, Parte, Tela, VersaoDoApp } from "./tipos";
export type Hardware = PerfilDeHardware_Serialize;
export type ResumoDoJogo = ResumoDoJogo_Serialize;
export type JogoDaLoja = JogoDaLoja_Serialize;
export type Historico = HistoricoNaTela_Serialize;
export type Mudanca = Historico["mudancas"][number];
export type Desfeito = OptimizationOutcome_Serialize;
export type RelatorioDeDesfazer = Relatorio_Serialize;
export type { FimDaTroca, TrocaAplicada } from "./tipos";
export type DiagnosticoDaTela = DiagnosticoDaTela_Serialize;
export type { TrechoDaRota } from "./tipos";
export type Rota = Rota_Serialize;
export type SaltoDaRota = Rota["saltos"][number];
export type HistoricoDaRede = HistoricoDaRede_Serialize;
export type NumerosDoPeriodo = HistoricoDaRede["resumo"]["periodos"][number];
export type DiagnosticoDoWifi = DiagnosticoDoWifi_Serialize;
export type PainelDaVarredura = Painel_Serialize;
export type CapturaDpc = ResultadoDaCaptura_Serialize;
export type PainelDaBanda = PainelDaBanda_Serialize;
export type PainelDoTurbo = PainelDoTurbo_Serialize;
export type RelatorioDoPlano = RelatorioDoPlano_Serialize;
export type AjusteDoCatalogo = OptimizationInfo_Serialize;
export type ResultadoDoLab = ResultadoDoLab_Serialize;
export type RespostaDoOti = RespostaDoOti_Serialize;
export type { TrocaComOti };
export type Preferencias = Preferences_Serialize;
export type EstadoDaLicenca = EstadoDaLicenca_Serialize;
export type AvisoDeVersao = AvisoDeVersao_Serialize;
export type Sobreposicao = EstadoDaSobreposicao_Serialize;
export type { ModoDaSobreposicao, CantoDaSobreposicao };
export type { LadoDoLab };
export type { StartupEntry, ItemDaLimpeza, ResultadoDaLimpeza, RestoreStatus };
export type EnergiaDeJogo = PainelDaEnergiaDeJogo_Serialize;
export type RelatorioDeTetos = RelatorioDeTetos_Serialize;
export type { VistoriaDoPlano, DiagnosticoDeEnergia };
export type ProvaDoTurbo = ResultadoDaProva_Serialize;
export type ResultadoDaBanda = ResultadoDaBanda_Serialize;
export type RelatorioDpc = Extract<CapturaDpc, { estado: "Medido" }>["relatorio"];
export type DriverMedido = RelatorioDpc["drivers"][number];

/** Só o que a casca usa do jogo aberto. */
export interface JogoAberto {
  nome: string;
}

/**
 * O recorte das métricas que a casca mostra. `null` é "não medi" (um `f64` NaN ou infinito chega assim do Rust), e
 * nunca vira zero na tela.
 */
export interface MetricasAoVivo {
  cpu: { overall: number | null };
  ram: { total_gb: number | null; used_gb: number | null };
}

export const noTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const demo: {
  versao: VersaoDoApp;
  hardware: Hardware;
  biblioteca: ResumoDoJogo[];
  instalados: JogoDaLoja[];
} = {
  versao: { interna: "4.0.0", exibida: "1.0" },
  hardware: {
    processador: "Intel Core i3-10100F",
    nucleos_logicos: 8,
    memoria_gb: 7.9,
    disco_do_sistema: "SSD",
    placa: { nome: "NVIDIA GeForce GTX 1650", fabricante: "Nvidia", familia: "GtxOuAnterior", driver: "617.14" },
    telas: [
      {
        nome: "24G4",
        dispositivo: "\\\\.\\DISPLAY1",
        principal: true,
        largura: 1920,
        altura: 1080,
        hz_atual: 60,
        hz_maximo: 180,
        vrr: "Provavel",
      },
    ],
    windows_build: 26200,
    windows_modificado: null,
    lido_em: 0,
  },
  biblioteca: [
    {
      id: "fivem",
      nome: "FiveM",
      partidas: 12,
      ultima: { quando: Date.now() / 1000 - 3600, fps_nativo: 85, fps_na_tela: null, low_1pct: 54, frametime_ms: 11.7 },
      ganho_fps: { estado: "Medido", pct: 9.4, folga_pct: 3.1 },
      ganho_low: { estado: "Medido", pct: 22.0, folga_pct: 6.5 },
      melhor_fps: 92,
    },
  ],
  instalados: [
    { nome: "FiveM", loja: "Instalado", appid: null, ultima_vez: Math.floor(Date.now() / 1000) - 3600, gb: 12 },
    { nome: "Counter-Strike 2", loja: "Steam", appid: 730, ultima_vez: Math.floor(Date.now() / 1000) - 3 * 86400, gb: 33 },
    { nome: "Fortnite", loja: "Epic Games", appid: null, ultima_vez: 0, gb: 98 },
  ],
};

/** No navegador o desfazer só tira da lista de demonstração: nada no PC de quem olha é tocado. */
const historicoDemo: Historico = {
  leitura: { estado: "Ok" },
  mudancas: [
    {
      id: "plano_otimiza",
      nome: "Plano de energia OTIMIZA",
      quando: Date.now() / 1000 - 600,
      o_que_mudou: ["energia · plano ativo (antes: Equilibrado)"],
      precisa_de_admin: null,
      precisa_reiniciar: null,
      desfazivel: true,
    },
    {
      id: "game_dvr",
      nome: "Desligar a gravação em segundo plano do Xbox",
      quando: Date.now() / 1000 - 86400 - 3000,
      o_que_mudou: [
        "registro · HKCU\\System\\GameConfigStore\\GameDVR_Enabled (antes: 1)",
        "registro · HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\GameDVR\\AppCaptureEnabled (antes: 1)",
      ],
      precisa_de_admin: false,
      precisa_reiniciar: false,
      desfazivel: true,
    },
    {
      id: "hags",
      nome: "Agendamento de GPU acelerado por hardware",
      quando: Date.now() / 1000 - 3 * 86400,
      o_que_mudou: ["registro · HKLM\\SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers\\HwSchMode (antes: 1)"],
      precisa_de_admin: true,
      precisa_reiniciar: true,
      desfazivel: true,
    },
    {
      id: "limpar_temporarios",
      nome: "Limpar arquivos temporários",
      quando: Date.now() / 1000 - 5 * 86400,
      o_que_mudou: ["arquivos · 1,2 GB apagados de %TEMP%"],
      precisa_de_admin: false,
      precisa_reiniciar: false,
      desfazivel: false,
    },
  ],
};

function desfeitoDemo(m: Mudanca): Desfeito {
  return {
    id: m.id,
    name: m.nome,
    success: true,
    applied: false,
    message: "Desfeito (demonstração: nada neste PC foi tocado).",
    requires_restart: m.precisa_reiniciar === true,
    requires_logoff: false,
    duration_ms: 0,
    changes_count: m.o_que_mudou.length,
    changes: m.o_que_mudou,
    actions: [],
  };
}

/** Monitor de demonstração: 60 Hz num painel de 180, o caso mais comum que a área Tela resolve. */
let telaDemoHz = 60;
let tokenDemo = 0;

function telasDemo(): DiagnosticoDaTela[] {
  const noMaximo = telaDemoHz >= 180;
  return [
    {
      dispositivo: "\\\\.\\DISPLAY1",
      nome: "24G4",
      principal: true,
      atual: { largura: 1920, altura: 1080, bits: 32, hz: telaDemoHz, flags: 0, orientacao: 0, x: 0, y: 0 },
      hz_maximo_aqui: 180,
      nativa: { largura: 1920, altura: 1080, fonte: "Edid" },
      hz_maximo_na_nativa: 180,
      cabo: { estado: "NaoLimita" },
      faixa_max_hz: 180,
      veredito: noMaximo ? { estado: "NoMaximo", hz_atual: telaDemoHz } : { estado: "UmClique", hz_atual: telaDemoHz, hz_alvo: 180 },
      texto: noMaximo
        ? `24G4 já está na maior taxa desta resolução (${telaDemoHz} Hz).`
        : `24G4 aguenta 180 Hz e está em ${telaDemoHz}. Corrigir leva um clique e se desfaz pelo histórico.`,
    },
  ];
}

function redeHistoricoDemo(): HistoricoDaRede {
  const agora = Math.floor(Date.now() / 1000);
  const recentes = [21, 20, 10, 22, 9, 15, 21, 11].map((hora, i) => {
    const noite = hora >= 18;
    return {
      quando: agora - i * 30 * 3600,
      hora,
      dia_da_semana: i % 7,
      destino: "185.40.64.12",
      do_jogo: true,
      chegou: true,
      mediana_ms: noite ? 58 + i : 31 + i,
      jitter_ms: noite ? 9 : 2,
      perda_pct: noite ? 20 : 0,
      veredito: noite ? "perda" : "limpa",
      trecho: noite ? ("operadora" as const) : null,
      salto: noite ? 4 : null,
      operadora: "Vivo",
    };
  });
  return {
    recentes,
    resumo: {
      medidas: 8,
      periodos: [
        { periodo: "manha", medidas: 3, ping_ms: 34, perda_pct: 0, com_problema: 0 },
        { periodo: "tarde", medidas: 1, ping_ms: 36, perda_pct: 0, com_problema: 0 },
        { periodo: "noite", medidas: 4, ping_ms: 61, perda_pct: 20, com_problema: 4 },
      ],
      frase: "À noite a conexão perde mais pacotes: 20% em média, contra 0% de manhã. 4 de 4 medidas desse horário tiveram problema.",
      na_operadora: 4,
    },
    relatorio:
      "Relatório de rota (medido pelo Otimiza, no estilo do MTR: 10 pacotes por salto)\nOperadora: Vivo\nMedidas com problema dentro da rede da operadora: 4 de 8\n\n- (demonstração) destino 185.40.64.12 · começa no salto 4 · perda de 20% · ping 61 ms\n",
  };
}

/** Rota de demonstração com a variação começando na operadora: o caso que a área Rede existe para apontar. */
function rotaDemo(destino: string): Rota {
  const salto = (
    numero: number,
    endereco: string | null,
    mediana: number | null,
    trecho: SaltoDaRota["trecho"],
    operadora: string | null,
    jitter: number | null
  ): SaltoDaRota => ({
    numero,
    endereco,
    enviados: 10,
    respondidos: endereco ? 10 : 0,
    perda_pct: endereco ? 0 : 100,
    melhor_ms: mediana != null ? mediana - 1 : null,
    mediana_ms: mediana,
    pior_ms: mediana != null ? mediana + (jitter ?? 0) * 3 : null,
    jitter_ms: jitter,
    trecho,
    asn: operadora ? 28573 : null,
    operadora,
  });
  return {
    destino,
    chegou: true,
    saltos: [
      salto(1, "192.168.0.1", 2, "casa", null, 0.4),
      salto(2, "100.64.0.1", 9, "operadora", "Claro", 1.1),
      salto(3, null, null, "sem_resposta", null, null),
      salto(4, "187.16.0.9", 14, "operadora", "Claro", 21.5),
      salto(5, "172.70.0.1", 18, "caminho", "Cloudflare", 22.8),
      salto(6, destino, 19, "servidor", "Cloudflare", 23.1),
    ],
    veredito: { tipo: "problema", problema: "variacao", trecho: "operadora", salto: 4, tamanho: 23.1, operadora: "Claro" },
    frase: "A variação começa na operadora (Claro), no salto 4, e vai até o servidor: 23 ms de jitter. Não é a sua casa.",
    observacao: "Salto 3 não responde ping: é o roteador poupando resposta, não perda.",
    atribuicao: "IP address data is powered by IPinfo",
  };
}

const wifiDemo: DiagnosticoDoWifi = {
  ligacao: {
    tipo: "wi_fi",
    placa: "Intel Wi-Fi 6 AX201",
    leitura: {
      estado: "lida",
      conexao: {
        ssid: "Casa",
        banda: "2,4 GHz",
        canal: 6,
        rssi_dbm: -67,
        sinal: "Bom",
        qualidade: 66,
        padrao: "Wi-Fi 4 (802.11n)",
        recepcao_mbps: 72,
        envio_mbps: 72,
      },
      vizinhanca: { mesmo_canal: 4, sobrepostas: 3, total_ouvidas: 14 },
    },
  },
  achados: [
    { gravidade: "Atencao", frase: "Conectado em 2,4 GHz com 7 redes disputando o canal. Se o roteador tiver 5 GHz, use." },
    { gravidade: "Info", frase: "Para jogar, o cabo ainda é o que menos varia." },
  ],
};

let varreduraDemo: PainelDaVarredura = { ligado: false, agindo_em: null, observacao: null };

/** Captura de demonstração: o driver de rede com picos, o caso que o Caçador existe para apontar. */
function capturaDemo(segundos: number): CapturaDpc {
  const driver = (
    arquivo: string,
    nome_amigavel: string,
    categoria: DriverMedido["categoria"],
    contagem: number,
    pct: number,
    maximo_us: number,
    p99: number,
    acima_500us: number,
    acima_1ms: number,
    acima_2ms: number,
    nivel: DriverMedido["nivel"],
    acao: string | null
  ): DriverMedido => ({
    arquivo,
    nome_amigavel,
    categoria,
    atribuicao: "Exata",
    contagem,
    contagem_dpc: Math.round(contagem * 0.8),
    contagem_isr: Math.round(contagem * 0.2),
    tempo_total_ms: (pct / 100) * segundos * 1000,
    pct_de_um_nucleo: pct,
    maximo_us,
    p99_ate_us: p99,
    acima_500us,
    acima_1ms,
    acima_2ms,
    abaixo_da_resolucao: 0,
    nivel,
    acao,
  });
  return {
    estado: "Medido",
    relatorio: {
      origem: "MEDIDO",
      duracao_s: segundos,
      nucleos: 8,
      relogio: "Qpc",
      resolucao_ns: 100,
      eventos: 184_220,
      descartados: 0,
      eventos_perdidos: 0,
      buffers_perdidos: 0,
      modulos_vistos: 212,
      sessao: "Otimiza-DPC",
      pct_total_de_um_nucleo: 1.9,
      pct_total_do_processador: 0.24,
      drivers: [
        driver("ndis.sys", "Rede (NDIS)", "Rede", 41_200, 0.9, 2_480, 310, 14, 5, 2, "Pico",
          "Driver da placa de rede ou Wi-Fi: atualize pelo site do fabricante da placa-mãe ou da placa. Teste jogando por cabo: se o pico some, é o Wi-Fi."),
        driver("nvlddmkm.sys", "Vídeo NVIDIA", "Video", 88_100, 0.7, 640, 160, 3, 0, 0, "Atencao",
          "Atualize ou reinstale o driver de vídeo limpo (desinstalar o atual e instalar o mais novo do site do fabricante)."),
        driver("Wdf01000.sys", "Framework de drivers do Windows", "FrameworkWdf", 30_400, 0.2, 180, 60, 0, 0, 0, "Saudavel", null),
        driver("ntoskrnl.exe", "Windows", "Windows", 24_520, 0.1, 95, 30, 0, 0, 0, "Saudavel", null),
      ],
      picos: [],
      picos_sem_carimbo: 0,
      veredito: {
        tipo: "DriversComPico",
        culpados: ["ndis.sys"],
        texto: "Rede (NDIS) segurou o processador por até 2,5 ms, 5 vezes acima de 1 ms. É isso que vira travadinha e áudio picotando.",
      },
      avisos: [],
    },
  };
}

let bandaDemo: PainelDaBanda = {
  ligado: false,
  agindo_em: null,
  politicas: [],
  update_limitado: false,
  impedimento: null,
  o_que_faz:
    "Com o jogo aberto, OneDrive, Google Drive, Dropbox, torrents e lançadores (Epic, Battle.net, EA, Ubisoft) enviam no máximo 2 Mb/s, e o Windows Update baixa em segundo plano a até 2 Mb/s. Envio saturado é o que mais faz o ping pular em casa. O Windows não limita o DOWNLOAD de outros programas: quem estiver baixando muito aparece no aviso. Voz, transmissão, navegador, a Steam e o jogo não são tocados. Tudo volta quando o jogo fecha.",
};

/** No navegador não há jogo de verdade: a barra de menu deixa simular um, para ver a tela com jogo aberto. */
let jogoDemo: string | null = null;

/** O Turbo de demonstração: mede 2 s em vez de 10, prova em 3 s em vez de 5 min, e nada no PC é tocado. */
const turboDemo: { ligado: boolean; menor: boolean; lembrado: boolean; provou: boolean; provando: boolean } = {
  ligado: false,
  menor: false,
  lembrado: false,
  provou: false,
  provando: false,
};

function provaDemo(): ProvaDoTurbo {
  const resumo = (m: number) => ({ id: "demo", n: 3, media: m, mediana: m, desvio: 1.8, margem: 2.4 });
  return {
    jogo: "FiveM",
    quando: Math.floor(Date.now() / 1000),
    segundos_por_rodada: 45,
    rodadas: [],
    fps_sem: resumo(85),
    fps_com: resumo(96),
    low_sem: resumo(54),
    low_com: resumo(66),
    diferenca_fps: { Real: { delta: 11, pct: 12.9, folga: 5.1 } },
    diferenca_low: { Real: { delta: 12, pct: 22.2, folga: 8.4 } },
    desfecho: "Ganhou",
    leitura: "Com o Turbo, o FiveM fez 13% mais FPS e o 1% pior subiu 22%, com folga acima do ruído.",
    contra_o_equilibrado: false,
    ficou_com_otimiza: true,
    falha_ao_desfazer: null,
  };
}

function painelDemo(): PainelDoTurbo {
  const jogo = jogoDemo;
  return {
    jogo,
    menor_atraso: turboDemo.menor,
    perfil:
      jogo && turboDemo.lembrado
        ? {
            receita: {
              gargalo: "Processador",
              escala: "Qualidade",
              multiplicador: turboDemo.menor ? null : 0,
              energia: true,
              memoria: false,
              explicacao:
                "O processador segura o FiveM (98% num núcleo, a placa folgada). Escala Qualidade alivia a placa sem custo de processador, o plano de energia segura o clock, e o Frame Boost dobra o que chega à tela.",
            },
            fps_antes: 85,
            medido_em: Math.floor(Date.now() / 1000) - 60,
            prova: turboDemo.provou ? "Ganhou" : null,
          }
        : null,
    passos: turboDemo.ligado
      ? ["Plano de energia OTIMIZA ligado", "Escala Qualidade na janela do jogo", "Frame Boost em Auto (até 180 Hz)"]
      : [],
    ligado: turboDemo.ligado,
    resultado: null,
    prova: turboDemo.provou ? provaDemo() : null,
    provando: turboDemo.provando,
    prova_sozinha: false,
    duracao_da_prova_s: 300,
    ritmo: jogo ? { fps: 85, trancos_por_minuto: 14, teto: 80 } : null,
  };
}

/** Energia de demonstração: o plano OTIMIZA ainda não existe, a energia de jogo está desligada. */
const energiaDemo = { plano: false, jogo: false };
const maquinaDemo = {
  notebook: false,
  tem_bateria: false,
  fabricante_da_cpu: "Intel" as const,
  cpu: "Intel Core i3-10100F",
  nucleos_logicos: 8,
  modern_standby: false,
  build_do_windows: 26200,
  windows11: true,
};

function relatorioDemo(simulacao: boolean): RelatorioDoPlano {
  const ajuste = (nome: string, porque: string, antes: number, alvo: number, unidade = "") => ({
    nome,
    subgrupo: "Processador",
    ajuste: nome,
    classe: "Recomendada" as const,
    porque: porque + unidade,
    suportado: true,
    ac_antes: antes,
    dc_antes: antes,
    ac_alvo: alvo,
    dc_alvo: alvo,
    ac_depois: simulacao ? null : alvo,
    dc_depois: simulacao ? null : alvo,
    status: (simulacao ? "Mudaria" : "Aplicado") as "Mudaria" | "Aplicado",
    mensagem: "",
  });
  return {
    maquina: maquinaDemo,
    simulacao,
    plano_existia: energiaDemo.plano,
    guid_do_plano: null,
    guid_anterior: null,
    plano_ativo: !simulacao,
    ajustes: [
      ajuste("Estado mínimo do processador", "Núcleo que dorme a 5% demora a acordar no meio da partida.", 5, 100),
      ajuste("Estacionamento de núcleos", "Núcleos estacionados atrasam o primeiro quadro de cada pico.", 10, 100),
      ajuste("Modo de boost", "O turbo do processador sobe mais cedo e fica mais tempo.", 1, 2),
      ajuste("Suspensão seletiva USB", "Mouse e headset sem dormir no meio do jogo.", 1, 0),
    ],
    aplicados: simulacao ? 0 : 4,
    ja_estavam_bons: 6,
    nao_suportados: 1,
    falhas: 0,
    desfecho: "Sucesso",
  };
}

/** O catálogo de demonstração: um de cada estado, para a tela mostrar todos. */
const catalogoDemo: OptimizationInfo_Serialize[] = (
  [
    ["game_dvr", "Desligar a gravação em segundo plano do Xbox", "A Xbox Game Bar grava os últimos minutos o tempo todo: custa placa e disco no jogo.", "Gaming", "Measurable", "Applied", false],
    ["hags", "Agendamento de GPU acelerado por hardware", "A placa organiza a própria fila de quadros: menos atraso em placas novas.", "Gaming", "Situational", "Available", true],
    ["nagle", "Desligar o algoritmo de Nagle", "Pacotes pequenos saem na hora em vez de esperar juntar: menos atraso em jogo de tiro.", "Network", "Responsiveness", "Available", false],
    ["telemetria", "Reduzir a telemetria do Windows", "Menos envio de dados para a Microsoft. Não muda FPS.", "Privacy", "NoGain", "AlreadyOptimal", false],
    ["sysmain", "Desligar o SysMain (Superfetch)", "Em SSD, para de pré-carregar programas e de mexer no disco no meio do jogo.", "System", "Situational", "Unknown", false],
  ] as const
).map(([id, name, honest_effect, category, expected_gain, state, admin]) => ({
  id,
  name,
  description: honest_effect,
  honest_effect,
  category,
  expected_gain,
  risco_de_fps: { risco: "Nenhum" },
  requires_admin: admin,
  requires_restart: id === "hags",
  reversible: true,
  security_tradeoff: false,
  recommended: id !== "telemetria",
  retirado: false,
  expert: id === "sysmain",
  condicao: null,
  state,
  detail: state === "Unknown" ? "Não consegui ler o serviço agora." : null,
}));

const inicializacaoDemo: StartupEntry[] = [
  { name: "Discord", command: "Discord.exe --start-minimized", executable: "Discord.exe", hive: "HKCU", enabled: true, classe: "Util" },
  { name: "Steam", command: "steam.exe -silent", executable: "steam.exe", hive: "HKCU", enabled: true, classe: "Opcional" },
  { name: "Epic Games Launcher", command: "EpicGamesLauncher.exe -silent", executable: "EpicGamesLauncher.exe", hive: "HKCU", enabled: false, classe: "Opcional" },
  { name: "Realtek Audio", command: "RtkAudUService64.exe", executable: "RtkAudUService64.exe", hive: "HKLM", enabled: true, classe: "Essencial" },
];

let limpezaDemo: ItemDaLimpeza[] = [
  { id: "temp_usuario", nome: "Temporários do seu usuário", explica: "Arquivos que programas deixaram para trás há mais de 2 dias.", bytes: 1_840_000_000, arquivos: 4210, precisa_admin: false },
  { id: "temp_windows", nome: "Temporários do Windows", explica: "A pasta Temp do sistema, só arquivo com mais de 2 dias.", bytes: 312_000_000, arquivos: 380, precisa_admin: true },
  { id: "relatorios", nome: "Relatórios de erro", explica: "Relatórios de travamento que já foram (ou nunca serão) enviados.", bytes: 96_000_000, arquivos: 41, precisa_admin: false },
];

/** O Lab de demonstração: as rodadas variam um pouco, o "depois" é melhor, como numa otimização que funcionou. */
const labDemo: { antes: number[]; depois: number[] } = { antes: [], depois: [] };

function labResultadoDemo(): ResultadoDoLab {
  const media = (v: number[]) => (v.length ? v.reduce((a, b) => a + b, 0) / v.length : null);
  const n = Math.min(labDemo.antes.length, labDemo.depois.length);
  const a = media(labDemo.antes);
  const d = media(labDemo.depois);
  const conf = n < 3 ? "Insuficiente" : "Alta";
  const linha = (metrica: string, k: number, menor: boolean) => ({
    metrica,
    antes: a == null ? null : a * k,
    depois: d == null ? null : d * k,
    diferenca_pct: conf === "Alta" && a && d ? ((d - a) / a) * 100 * (menor ? -1 : 1) : null,
    confianca: conf as ResultadoDoLab["linhas"][number]["confianca"],
    menor_e_melhor: menor,
  });
  return {
    nome: jogoDemo ?? "FiveM",
    rodadas_antes: labDemo.antes.length,
    rodadas_depois: labDemo.depois.length,
    minimo_por_lado: 3,
    linhas: [linha("FPS médio", 1, false), linha("1% pior", 0.62, false), linha("Tempo de quadro (ms)", 1000 / 85 / 85, true)],
    descartadas: 0,
  };
}

let preferenciasDemo: Preferencias = {
  restore_point_before_batch: true,
  metrics_interval_seconds: 2,
  auto_game_mode: false,
  show_unavailable: true,
  medir_quadros_sozinho: true,
};
let sobreposicaoDemo: Sobreposicao = {
  config: { modo: "Minimo", canto: "SuperiorEsquerdo", confirmados: [], com_frame_boost: false, versao: 2 },
  ligado: false,
  jogo: null,
};

let demoTick = 0;

export const ponte = {
  versao: () => (noTauri ? commands.versaoDoApp() : Promise.resolve(demo.versao)),
  hardwareUltimo: () => (noTauri ? commands.hardwareUltimo() : Promise.resolve(demo.hardware)),
  hardware: () => (noTauri ? commands.hardwarePerfil(false) : Promise.resolve(demo.hardware)),
  jogoAberto: async (): Promise<JogoAberto | null> => {
    if (!noTauri) return jogoDemo ? { nome: jogoDemo } : null;
    const jogo = await commands.jogoAberto();
    return jogo ? { nome: jogo.nome } : null;
  },
  simularJogo: (nome: string | null) => {
    if (!noTauri) jogoDemo = nome;
  },
  biblioteca: () => (noTauri ? commands.bibliotecaDeJogos() : Promise.resolve(demo.biblioteca)),
  instalados: () => (noTauri ? commands.jogosInstalados() : Promise.resolve(demo.instalados)),
  capa: (appid: number) => (noTauri ? commands.capaDoJogo(appid) : Promise.resolve(null)),
  iniciarMonitor: async () => {
    if (noTauri) await commands.startMonitoring();
  },
  historico: (): Promise<Historico> =>
    noTauri ? commands.historicoDeMudancas() : Promise.resolve(structuredClone(historicoDemo)),
  desfazer: async (id: string): Promise<Desfeito> => {
    if (noTauri) return commands.revertOptimization(id);
    const i = historicoDemo.mudancas.findIndex((m) => m.id === id);
    if (i < 0) throw new Error("Essa mudança não está mais no histórico.");
    const [m] = historicoDemo.mudancas.splice(i, 1);
    return desfeitoDemo(m);
  },
  desfazerTudo: async (): Promise<RelatorioDeDesfazer> => {
    if (noTauri) return commands.restaurarTudo();
    const itens = historicoDemo.mudancas.filter((m) => m.desfazivel).map(desfeitoDemo);
    historicoDemo.mudancas = historicoDemo.mudancas.filter((m) => !m.desfazivel);
    return {
      partes: [
        {
          parte: "Historico",
          desfecho: itens.length ? { estado: "Voltou" } : { estado: "NadaAFazer" },
          feito: itens.map((i) => i.name),
        },
      ],
      itens,
      tudo_voltou: true,
    };
  },
  /** No navegador finge administrador para a demonstração mostrar todos os botões. */
  administrador: () => (noTauri ? commands.isElevated() : Promise.resolve(true)),
  abrirComoAdministrador: () => (noTauri ? commands.relaunchAsAdmin() : Promise.resolve("")),
  telas: () => (noTauri ? commands.telaDiagnostico() : Promise.resolve(telasDemo())),
  trocaDeTelaPendente: () => (noTauri ? commands.telaTrocaPendente() : Promise.resolve(false)),
  /** No navegador nada troca de verdade: só o número do monitor de demonstração. */
  trocarHz: async (dispositivo: string, hz: number): Promise<TrocaAplicada> => {
    if (noTauri) return commands.telaTrocarHz(dispositivo, hz);
    const hz_anterior = telaDemoHz;
    telaDemoHz = hz;
    return { token: ++tokenDemo, dispositivo, hz_anterior, hz_novo: hz, prazo_ms: 15000 };
  },
  confirmarTroca: (token: number): Promise<FimDaTroca> =>
    noTauri ? commands.telaConfirmarTroca(token) : Promise.resolve({ estado: "Mantida", id: "tela:demo", hz: telaDemoHz }),
  voltarTroca: async (token: number): Promise<FimDaTroca> => {
    if (noTauri) return commands.telaVoltarTroca(token);
    telaDemoHz = 60;
    return { estado: "Voltou", hz: 60 };
  },
  /** A rota leva uns 10 s no Rust; a demonstração espera um pouco para a tela de espera aparecer. */
  rota: async (destino: string): Promise<Rota> => {
    if (noTauri) return commands.redeRota(destino);
    await new Promise((r) => setTimeout(r, 1200));
    return rotaDemo(destino);
  },
  /** Na demonstração: uma semana em que a noite perde pacotes na operadora (o caso que o relatório existe para provar). */
  historicoDaRede: (): Promise<HistoricoDaRede> =>
    noTauri ? commands.redeHistorico() : Promise.resolve(redeHistoricoDemo()),
  rotaDoJogo: (): Promise<Rota> =>
    noTauri
      ? commands.redeRotaDoJogo()
      : Promise.reject("Não descobri o servidor do jogo: abra o jogo e entre numa partida antes de medir a rota."),
  wifi: (): Promise<DiagnosticoDoWifi> => (noTauri ? commands.redeWifi() : Promise.resolve(structuredClone(wifiDemo))),
  varredura: (): Promise<PainelDaVarredura> =>
    noTauri ? commands.redeVarreduraWifi() : Promise.resolve({ ...varreduraDemo }),
  definirVarredura: async (ligado: boolean): Promise<PainelDaVarredura> => {
    if (noTauri) return ligado ? commands.redeVarreduraWifiLigar() : commands.redeVarreduraWifiDesligar();
    varreduraDemo = { ...varreduraDemo, ligado };
    return { ...varreduraDemo };
  },
  /** No navegador a "captura" dura 2 s, qualquer que seja o pedido: é só para ver a tela de espera. */
  capturarDpc: async (segundos: number): Promise<CapturaDpc> => {
    if (noTauri) return commands.dpcCapturar(segundos);
    await new Promise((r) => setTimeout(r, 2000));
    return capturaDemo(segundos);
  },
  cancelarDpc: () => (noTauri ? commands.dpcCancelar() : Promise.resolve()),
  banda: (): Promise<PainelDaBanda> => (noTauri ? commands.bandaPainel() : Promise.resolve({ ...bandaDemo })),
  /** No navegador não há jogo aberto: ligar só grava a escolha, como o Rust faz sem jogo. */
  definirBanda: async (ligado: boolean): Promise<ResultadoDaBanda> => {
    if (noTauri) return ligado ? commands.bandaLigar() : commands.bandaDesligar();
    bandaDemo = { ...bandaDemo, ligado };
    return ligado ? { tipo: "NadaALimitar", avisos: [] } : { tipo: "Desligado" };
  },
  turbo: (): Promise<PainelDoTurbo> => (noTauri ? commands.turboPainel() : Promise.resolve(painelDemo())),
  ligarTurbo: async (lembrado: boolean): Promise<PainelDoTurbo> => {
    if (noTauri) return commands.turboLigar(lembrado);
    if (!jogoDemo) throw new Error("Abra o jogo primeiro: o Turbo mede o jogo aberto.");
    await new Promise((r) => setTimeout(r, lembrado ? 400 : 2000));
    turboDemo.ligado = true;
    turboDemo.lembrado = true;
    return painelDemo();
  },
  desligarTurbo: async (): Promise<PainelDoTurbo> => {
    if (noTauri) return commands.turboDesligar();
    turboDemo.ligado = false;
    return painelDemo();
  },
  provarTurbo: async (): Promise<ProvaDoTurbo> => {
    if (noTauri) return commands.turboProvar();
    turboDemo.provando = true;
    await new Promise((r) => setTimeout(r, 3000));
    turboDemo.provando = false;
    turboDemo.provou = true;
    return provaDemo();
  },
  compartilharTurbo: () =>
    noTauri ? commands.turboCompartilhar() : Promise.resolve("Turbo do Otimiza no FiveM: +13% FPS, +22% no 1% pior (prova A-B)."),
  atrasoDoTurbo: async (menor: boolean): Promise<PainelDoTurbo> => {
    if (noTauri) return commands.turboAtraso(menor);
    turboDemo.menor = menor;
    return painelDemo();
  },
  esquecerTurbo: async (): Promise<PainelDoTurbo> => {
    if (noTauri) return commands.turboEsquecer();
    turboDemo.lembrado = false;
    turboDemo.provou = false;
    return painelDemo();
  },
  vistoriarPlano: (): Promise<VistoriaDoPlano> =>
    noTauri ? commands.vistoriarPlanoOtimiza() : Promise.resolve(energiaDemo.plano ? { estado: "Integro" } : { estado: "NaoExiste" }),
  simularPlano: async (): Promise<RelatorioDoPlano> => {
    if (noTauri) return commands.simularPlanoOtimiza();
    await new Promise((r) => setTimeout(r, 700));
    return relatorioDemo(true);
  },
  aplicarPlano: async (): Promise<RelatorioDoPlano> => {
    if (noTauri) return commands.aplicarPlanoOtimiza();
    await new Promise((r) => setTimeout(r, 900));
    energiaDemo.plano = true;
    return relatorioDemo(false);
  },
  repararPlano: (): Promise<RelatorioDoPlano> =>
    noTauri ? commands.repararPlanoOtimiza() : Promise.resolve(relatorioDemo(false)),
  desfazerPlano: async () => {
    if (noTauri) return commands.revertOptimization("plano_otimiza");
    energiaDemo.plano = false;
    return null;
  },
  energiaDeJogo: (): Promise<EnergiaDeJogo> =>
    noTauri
      ? commands.energiaDeJogo()
      : Promise.resolve({
          ligado: energiaDemo.jogo,
          agindo_em: energiaDemo.jogo && jogoDemo ? jogoDemo : null,
          impedimento: null,
          o_que_muda: "Com o jogo aberto, o processador não dorme abaixo de 100% e o boost fica agressivo. Fechou o jogo, volta tudo.",
        }),
  definirEnergiaDeJogo: async (ligado: boolean): Promise<EnergiaDeJogo> => {
    if (noTauri) return commands.energiaDeJogoDefinir(ligado);
    energiaDemo.jogo = ligado;
    return ponte.energiaDeJogo();
  },
  diagnosticoDeEnergia: (): Promise<DiagnosticoDeEnergia> =>
    noTauri
      ? commands.diagnosticoDeEnergia()
      : Promise.resolve({
          maquina: maquinaDemo,
          elevado: true,
          powercfg_responde: true,
          planos_legiveis: true,
          planos: [["381b4222-f694-41f0-9685-ff5bb260df2e", "Equilibrado"]],
          plano_otimiza_existe: energiaDemo.plano,
          registro_de_energia_legivel: true,
          ajustes_suportados: 11,
          ajustes_totais: 12,
          ajustes_ausentes: ["Escalonamento heterogêneo (só em processador com núcleos P e E)"],
          processo_64_bits: true,
          governo_do_processador: "OWindows",
          explicacao_do_governo: "Quem decide o clock é o Windows, pelo plano de energia: os ajustes do Otimiza valem aqui.",
          avisos: [],
        }),
  tetosEscondidos: (): Promise<RelatorioDeTetos> =>
    noTauri
      ? commands.tetosEscondidos()
      : Promise.resolve({ monitor_hz: 180, tetos: [{ tipo: "RobloxLimitado", fps: 60, arquivo: "GlobalBasicSettings_13.xml" }], lacunas: [] }),
  catalogo: (): Promise<AjusteDoCatalogo[]> => (noTauri ? commands.listOptimizations() : Promise.resolve(structuredClone(catalogoDemo))),
  aplicarAjuste: async (id: string): Promise<Desfeito> => {
    if (noTauri) return commands.applyOptimization(id);
    const a = catalogoDemo.find((x) => x.id === id);
    if (a) a.state = "Applied";
    return { id, name: a?.name ?? id, success: true, applied: true, message: "Aplicado (demonstração).", requires_restart: !!a?.requires_restart, requires_logoff: false, duration_ms: 0, changes_count: 1, changes: [], actions: [] };
  },
  desfazerAjuste: async (id: string): Promise<Desfeito> => {
    if (noTauri) return commands.revertOptimization(id);
    const a = catalogoDemo.find((x) => x.id === id);
    if (a) a.state = "Available";
    return { id, name: a?.name ?? id, success: true, applied: false, message: "Desfeito (demonstração).", requires_restart: false, requires_logoff: false, duration_ms: 0, changes_count: 1, changes: [], actions: [] };
  },
  inicializacao: (): Promise<StartupEntry[]> => (noTauri ? commands.listStartup() : Promise.resolve(structuredClone(inicializacaoDemo))),
  definirInicializacao: async (e: StartupEntry, ligado: boolean): Promise<Desfeito> => {
    if (noTauri) return commands.setStartupEnabled(e.hive, e.name, ligado);
    const x = inicializacaoDemo.find((i) => i.name === e.name);
    if (x) x.enabled = ligado;
    return { id: `startup:${e.name}`, name: e.name, success: true, applied: !ligado, message: "", requires_restart: false, requires_logoff: false, duration_ms: 0, changes_count: 1, changes: [], actions: [] };
  },
  medirLimpeza: (): Promise<ItemDaLimpeza[]> => (noTauri ? commands.limpezaMedir() : Promise.resolve(structuredClone(limpezaDemo))),
  limpar: async (id: string): Promise<ResultadoDaLimpeza> => {
    if (noTauri) return commands.limpezaExecutar(id);
    await new Promise((r) => setTimeout(r, 800));
    const item = limpezaDemo.find((i) => i.id === id);
    const r = { bytes_liberados: Math.round((item?.bytes ?? 0) * 0.92), apagados: Math.round((item?.arquivos ?? 0) * 0.9), ficaram: 12 };
    limpezaDemo = limpezaDemo.map((i) => (i.id === id ? { ...i, bytes: (item?.bytes ?? 0) - r.bytes_liberados, arquivos: 12 } : i));
    return r;
  },
  restauracao: (): Promise<RestoreStatus> =>
    noTauri
      ? commands.restoreStatus()
      : Promise.resolve({ available: true, message: "A proteção do sistema está ligada no disco C:.", points: [{ sequence: 41, description: "Otimiza - antes de otimizar", created_at: "2026-11-05 14:20" }] }),
  criarPontoDeRestauracao: async (): Promise<string> => {
    if (noTauri) return commands.createRestorePoint();
    await new Promise((r) => setTimeout(r, 1200));
    return "Ponto de restauração criado (demonstração).";
  },
  /** O executável do jogo aberto (o Lab mede um processo), ou `null` sem jogo. */
  processoDoJogo: async (): Promise<string | null> => {
    if (!noTauri) return jogoDemo ? "FiveM_GTAProcess.exe" : null;
    const j = await commands.jogoAberto();
    return j?.executavel ?? null;
  },
  labEstado: (): Promise<ResultadoDoLab> => (noTauri ? commands.labEstado() : Promise.resolve(labResultadoDemo())),
  labMedir: async (processo: string, lado: LadoDoLab, segundos: number): Promise<ResultadoDoLab> => {
    if (noTauri) return commands.labMedir(processo, lado, segundos);
    await new Promise((r) => setTimeout(r, 1500));
    (lado === "Antes" ? labDemo.antes : labDemo.depois).push((lado === "Antes" ? 85 : 95) + (Math.random() * 4 - 2));
    return labResultadoDemo();
  },
  labZerar: async () => {
    if (noTauri) return commands.labZerar();
    labDemo.antes = [];
    labDemo.depois = [];
    return null;
  },
  provaGuardada: (): Promise<ProvaDoTurbo | null> => (noTauri ? commands.provaAlternadaGuardada() : Promise.resolve(null)),
  compartilharProva: (): Promise<string> =>
    noTauri ? commands.resultadoParaCompartilhar() : Promise.reject("Ainda não há prova com comparação justa para compartilhar."),
  preferencias: (): Promise<Preferencias> => (noTauri ? commands.getPreferences() : Promise.resolve({ ...preferenciasDemo })),
  gravarPreferencias: async (p: Preferencias): Promise<Preferencias> => {
    if (noTauri) return commands.setPreferences(p);
    preferenciasDemo = { ...p };
    return { ...p };
  },
  licenca: (): Promise<EstadoDaLicenca> =>
    noTauri
      ? commands.licencaEstado()
      : Promise.resolve({ ativa: true, maquina: "DEMO-1234", origem: "Licença vitalícia", sobrevive_formatacao: true, comprador: "Demonstração", expira: null, motivo: null }),
  ativarLicenca: (chave: string): Promise<EstadoDaLicenca> =>
    noTauri ? commands.licencaAtivar(chave) : Promise.reject("No navegador não dá para ativar: abra o Otimiza."),
  versaoMaisNova: (): Promise<AvisoDeVersao> =>
    noTauri
      ? commands.versaoMaisNova()
      : Promise.resolve({ comparacao: "NaoHaNova", versao_publicada: "4.0.0", versao_publicada_exibida: "1.0", pagina: null }),
  relatorioDeSuporte: (): Promise<string> =>
    noTauri ? commands.relatorioDeSuporte() : Promise.resolve("Relatório de suporte (demonstração)."),
  sobreposicao: (): Promise<Sobreposicao> => (noTauri ? commands.overlayEstado() : Promise.resolve(structuredClone(sobreposicaoDemo))),
  gravarSobreposicao: async (modo: ModoDaSobreposicao, canto: CantoDaSobreposicao, comFrameBoost: boolean): Promise<Sobreposicao> => {
    if (noTauri) return commands.overlayGravar(modo, canto, comFrameBoost);
    sobreposicaoDemo = { ...sobreposicaoDemo, config: { ...sobreposicaoDemo.config, modo, canto, com_frame_boost: comFrameBoost } };
    return structuredClone(sobreposicaoDemo);
  },
  mostrarSobreposicao: (confirmado: boolean): Promise<Sobreposicao> =>
    noTauri ? commands.overlayLigar(confirmado) : Promise.reject("Abra o jogo primeiro: o FPS fica no canto da janela dele."),
  /** O Oti com IA. No navegador, uma resposta de demonstração (nada vai à internet). */
  perguntarAoOti: async (pergunta: string, fatos: object, historico: TrocaComOti[]): Promise<RespostaDoOti> => {
    if (noTauri) return commands.otiPerguntar(pergunta, JSON.stringify(fatos), historico);
    await new Promise((r) => setTimeout(r, 1200));
    return {
      resposta: "No seu i3-10100F com a GTX 1650, quem segura o FiveM é o processador. Ligue o Turbo com o jogo aberto e prove na Prova.",
      humor: "atento",
      acoes: [{ rotulo: "Abrir o Turbo", area: "turbo" }],
      restantes: 19,
    };
  },
  metricas: async (): Promise<MetricasAoVivo> => {
    if (noTauri) {
      const m = await commands.getPerformanceMetrics();
      return { cpu: { overall: m.cpu.overall }, ram: { total_gb: m.ram.total_gb, used_gb: m.ram.used_gb } };
    }
    demoTick += 1;
    return {
      cpu: { overall: 12 + 6 * Math.sin(demoTick / 3) },
      ram: { total_gb: 7.9, used_gb: 5.1 + 0.2 * Math.cos(demoTick / 5) },
    };
  },
};
