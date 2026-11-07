// A conversa da interface nova com o Rust. Os comandos e os tipos vêm de `tipos.ts`, gerado a partir do Rust pelo
// teste `contrato` (`src-tauri/src/contrato.rs`): um comando renomeado ou um campo trocado quebra a compilação daqui,
// não a tela do cliente. Comando novo na interface = entra na lista do `contrato.rs` primeiro.
//
// Fora do Tauri (o navegador do `npm run nova:web`) não há backend: a ponte devolve uma máquina de demonstração e a
// barra de menu diz isso. Dado de demonstração nunca aparece como se fosse do PC de quem está olhando.

import {
  commands,
  type DiagnosticoDaTela_Serialize,
  type FimDaTroca,
  type HistoricoNaTela_Serialize,
  type JogoDaLoja_Serialize,
  type OptimizationOutcome_Serialize,
  type PerfilDeHardware_Serialize,
  type Relatorio_Serialize,
  type ResumoDoJogo_Serialize,
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
    { nome: "FiveM", loja: "Instalado", appid: null, ultima_vez: 3, gb: 12 },
    { nome: "Counter-Strike 2", loja: "Steam", appid: 730, ultima_vez: 2, gb: 33 },
    { nome: "Fortnite", loja: "Epic Games", appid: null, ultima_vez: 1, gb: 98 },
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

let demoTick = 0;

export const ponte = {
  versao: () => (noTauri ? commands.versaoDoApp() : Promise.resolve(demo.versao)),
  hardwareUltimo: () => (noTauri ? commands.hardwareUltimo() : Promise.resolve(demo.hardware)),
  hardware: () => (noTauri ? commands.hardwarePerfil(false) : Promise.resolve(demo.hardware)),
  jogoAberto: async (): Promise<JogoAberto | null> => {
    if (!noTauri) return null;
    const jogo = await commands.jogoAberto();
    return jogo ? { nome: jogo.nome } : null;
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
