// A conversa da interface nova com o Rust. Os tipos espelham os comandos de `src-tauri/src/commands.rs`; na Onda 0
// do plano eles passam a ser gerados pelo tauri-specta, e este arquivo vira só as chamadas.
//
// Fora do Tauri (o navegador do `npm run nova:web`) não há backend: a ponte devolve uma máquina de demonstração e a
// barra de menu diz isso. Dado de demonstração nunca aparece como se fosse do PC de quem está olhando.

import { invoke } from "@tauri-apps/api/core";

export type Fabricante = "Nvidia" | "Amd" | "Intel" | "Desconhecido";

export interface Tela {
  nome: string;
  dispositivo: string;
  principal: boolean;
  largura: number;
  altura: number;
  hz_atual: number;
  hz_maximo: number;
  vrr: "Anunciado" | "Provavel" | "NaoAnunciado" | "Desconhecido";
}

export interface Hardware {
  processador: string;
  nucleos_logicos: number;
  memoria_gb: number;
  disco_do_sistema: string;
  placa: { nome: string; fabricante: Fabricante; familia: string; driver: string | null };
  telas: Tela[];
  windows_build: number | null;
  windows_modificado?: { nome: string; autor: string | null } | null;
  lido_em: number;
}

export interface JogoAberto {
  pid: number;
  executavel: string;
  nome: string;
}

export interface MetricasAoVivo {
  cpu: { overall: number | null };
  ram: { total_gb: number; used_gb: number };
}

export type Ganho =
  | { estado: "Medido"; pct: number; folga_pct: number }
  | { estado: "SemDiferenca" }
  | { estado: "Medindo"; antes: number; depois: number };

export interface Partida {
  quando: number;
  fps_nativo: number;
  fps_na_tela: number | null;
  low_1pct: number;
  frametime_ms: number | null;
}

export interface ResumoDoJogo {
  id: string;
  nome: string;
  partidas: number;
  ultima: Partida | null;
  ganho_fps: Ganho;
  ganho_low: Ganho;
  melhor_fps: number | null;
}

export interface JogoDaLoja {
  nome: string;
  loja: "Steam" | "Epic Games" | "Instalado";
  appid: number | null;
  ultima_vez: number;
  gb: number;
}

export interface VersaoDoApp {
  interna: string;
  exibida: string;
}

export const noTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const demo = {
  versao: { interna: "4.0.0", exibida: "1.0" } satisfies VersaoDoApp,
  hardware: {
    processador: "Intel Core i3-10100F",
    nucleos_logicos: 8,
    memoria_gb: 7.9,
    disco_do_sistema: "SSD",
    placa: { nome: "NVIDIA GeForce GTX 1650", fabricante: "Nvidia", familia: "Turing", driver: "617.14" },
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
  } satisfies Hardware,
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
  ] satisfies ResumoDoJogo[],
  instalados: [
    { nome: "FiveM", loja: "Instalado", appid: null, ultima_vez: 3, gb: 12 },
    { nome: "Counter-Strike 2", loja: "Steam", appid: 730, ultima_vez: 2, gb: 33 },
    { nome: "Fortnite", loja: "Epic Games", appid: null, ultima_vez: 1, gb: 98 },
  ] satisfies JogoDaLoja[],
};

let demoTick = 0;

export const ponte = {
  versao: () => (noTauri ? invoke<VersaoDoApp>("versao_do_app") : Promise.resolve(demo.versao)),
  hardwareUltimo: () => (noTauri ? invoke<Hardware | null>("hardware_ultimo") : Promise.resolve(demo.hardware)),
  hardware: () => (noTauri ? invoke<Hardware>("hardware_perfil", { atualizar: false }) : Promise.resolve(demo.hardware)),
  jogoAberto: () => (noTauri ? invoke<JogoAberto | null>("jogo_aberto") : Promise.resolve(null)),
  biblioteca: () => (noTauri ? invoke<ResumoDoJogo[]>("biblioteca_de_jogos") : Promise.resolve(demo.biblioteca)),
  instalados: () => (noTauri ? invoke<JogoDaLoja[]>("jogos_instalados") : Promise.resolve(demo.instalados)),
  capa: (appid: number) => (noTauri ? invoke<string | null>("capa_do_jogo", { appid }) : Promise.resolve(null)),
  iniciarMonitor: () => (noTauri ? invoke<void>("start_monitoring") : Promise.resolve()),
  metricas: (): Promise<MetricasAoVivo> => {
    if (noTauri) return invoke<MetricasAoVivo>("get_performance_metrics");
    demoTick += 1;
    return Promise.resolve({
      cpu: { overall: 12 + 6 * Math.sin(demoTick / 3) },
      ram: { total_gb: 7.9, used_gb: 5.1 + 0.2 * Math.cos(demoTick / 5) },
    });
  },
};
