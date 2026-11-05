// A conversa da interface nova com o Rust. Os comandos e os tipos vêm de `tipos.ts`, gerado a partir do Rust pelo
// teste `contrato` (`src-tauri/src/contrato.rs`): um comando renomeado ou um campo trocado quebra a compilação daqui,
// não a tela do cliente. Comando novo na interface = entra na lista do `contrato.rs` primeiro.
//
// Fora do Tauri (o navegador do `npm run nova:web`) não há backend: a ponte devolve uma máquina de demonstração e a
// barra de menu diz isso. Dado de demonstração nunca aparece como se fosse do PC de quem está olhando.

import {
  commands,
  type JogoDaLoja_Serialize,
  type PerfilDeHardware_Serialize,
  type ResumoDoJogo_Serialize,
  type VersaoDoApp,
} from "./tipos";

export type { Ganho, Tela, VersaoDoApp } from "./tipos";
export type Hardware = PerfilDeHardware_Serialize;
export type ResumoDoJogo = ResumoDoJogo_Serialize;
export type JogoDaLoja = JogoDaLoja_Serialize;

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
