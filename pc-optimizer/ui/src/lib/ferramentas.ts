// A ponte das ferramentas trazidas da interface 3.3 (`src/v33/ponte.ts`): Frame Boost, mira, cores, contador e FOV do
// FiveM. Esses comandos recebem o `AppHandle` ou o estado do app no Rust, e por isso não entram no contrato tipado
// (`contrato.rs` roda com o MockRuntime): aqui vão por `invoke`, com os tipos escritos à mão — os mesmos da 3.3, que já
// conversavam com esses comandos. Fora do Tauri, dados de demonstração.

import { invoke } from "@tauri-apps/api/core";
import { noTauri } from "./ponte";
import type { Mira } from "./mira";

export type Suporte =
  | { estado: "Suportado" }
  | { estado: "Parcial"; motivo: string }
  | { estado: "NaoSuportado"; motivo: string }
  | { estado: "PrecisaAtualizar"; motivo: string };

export type Recurso =
  | "FrameBoostDoJogo"
  | "FrameBoostSmoothMotion"
  | "FrameBoostAfmf"
  | "FrameBoostOtimiza"
  | "BaixaLatenciaDoDriver"
  | "LimitadorDoDriver"
  | "MiraNaTela"
  | "MiraNaConfiguracaoDoJogo"
  | "CoresDoMonitor";

export interface RecursoAvaliado {
  recurso: Recurso;
  nome: string;
  suporte: Suporte;
}

export interface Valor {
  valor: number | null;
  origem: "Medido" | "Estimado" | "Desconhecido";
}

export type SituacaoDoGerador = "Parado" | "ProcurandoJanela" | "Gerando" | "SemQuadros" | "Erro" | "DesligadoPorPerdaDeFps";
export type NivelDeEscala = "UltraQualidade" | "Qualidade" | "Equilibrado" | "Desempenho";

export const NOMES_DA_ESCALA: Record<NivelDeEscala, string> = {
  UltraQualidade: "Ultra qualidade",
  Qualidade: "Qualidade",
  Equilibrado: "Equilibrado",
  Desempenho: "Desempenho",
};

export interface PainelFrameBoost {
  jogo: string | null;
  recomendado: RecursoAvaliado | null;
  opcoes: RecursoAvaliado[];
  multiplicadores: number[];
  hz: number | null;
  estimativas: { multiplicador: number; faixa: { minimo: number; maximo: number; origem: string } }[];
  ativo: {
    recurso: Recurso;
    multiplicador: number;
    automatico: boolean;
    sem_limite: boolean;
    fps_na_tela: Valor;
    situacao: SituacaoDoGerador;
    hz: number;
    hdr: boolean;
    integrada: boolean;
    refeitas: number;
    atraso_ms: Valor;
    pelo_driver: boolean;
    geracao_vista: boolean | null;
    nota: string | null;
    placa_pct: number | null;
    leve: boolean;
    fps_do_jogo: number | null;
    manteve_pct: number | null;
    comparacoes: number;
    escala?: NivelDeEscala | null;
    jogo_px?: [number, number] | null;
    tela_px?: [number, number] | null;
  } | null;
  aviso: string | null;
  base: { fps: number; multiplicador: number; valendo: boolean } | null;
  nota_da_base: string | null;
  base_solta: boolean;
  ultima_sessao: {
    jogo: string;
    minutos: number;
    fps_do_jogo: number;
    fps_na_tela: Valor;
    manteve_pct: number | null;
    comparacoes: number;
    desligou_por_perda: boolean;
  } | null;
  monitor?: [number, number] | null;
  escalas?: { nivel: NivelDeEscala; largura: number; altura: number }[];
}

export function motivo(s: Suporte): string | null {
  return "motivo" in s ? s.motivo : null;
}

export interface FovFivem {
  atual: number | null;
  pedido: number | null;
  pendente: boolean;
  fivem_aberto: boolean;
  alterado: boolean;
  maximo: number;
}

export interface ContadorFivem {
  ligado: boolean;
  pendente: boolean;
}

export type CaminhoDasCores = "Amd" | "Nvidia" | "Gama";

export interface AjusteDeCor {
  saturacao: number;
  brilho: number;
  contraste: number;
  gama: number;
  matiz: number;
}

export interface EstadoDasCores {
  caminho: CaminhoDasCores;
  suporta: Record<keyof AjusteDeCor, boolean>;
  aplicado: boolean;
  perfis: { por_jogo: Record<string, AjusteDeCor>; so_com_jogo: boolean };
  saturacao_atual: number | null;
  nota: string;
}

// ── Demonstração ───────────────────────────────────────────────────────────────────────────────────────────────────
let fbDemo: PainelFrameBoost = {
  jogo: "FiveM",
  recomendado: { recurso: "FrameBoostOtimiza", nome: "Frame Boost do Otimiza", suporte: { estado: "Suportado" } },
  opcoes: [
    { recurso: "FrameBoostDoJogo", nome: "Geração do próprio jogo", suporte: { estado: "NaoSuportado", motivo: "O FiveM não tem geração de quadros." } },
    { recurso: "FrameBoostAfmf", nome: "AMD Fluid Motion Frames", suporte: { estado: "NaoSuportado", motivo: "Só em placa AMD RX 6000 ou mais nova." } },
  ],
  multiplicadores: [2, 3],
  hz: 180,
  estimativas: [
    { multiplicador: 2, faixa: { minimo: 150, maximo: 170, origem: "Estimado" } },
    { multiplicador: 3, faixa: { minimo: 180, maximo: 180, origem: "Estimado" } },
  ],
  ativo: null,
  aviso: null,
  base: null,
  nota_da_base: null,
  base_solta: false,
  ultima_sessao: null,
  monitor: [1920, 1080],
  escalas: [
    { nivel: "UltraQualidade", largura: 1477, altura: 831 },
    { nivel: "Qualidade", largura: 1280, altura: 720 },
    { nivel: "Equilibrado", largura: 1130, altura: 635 },
    { nivel: "Desempenho", largura: 960, altura: 540 },
  ],
};

function fbLigadoDemo(multiplicador: number, escala: NivelDeEscala | null): PainelFrameBoost {
  const m = multiplicador === 0 ? 2 : multiplicador;
  fbDemo = {
    ...fbDemo,
    ativo: {
      recurso: "FrameBoostOtimiza",
      multiplicador: m,
      automatico: multiplicador === 0,
      sem_limite: false,
      fps_na_tela: { valor: 158, origem: "Medido" },
      situacao: "Gerando",
      hz: 180,
      hdr: false,
      integrada: false,
      refeitas: 0,
      atraso_ms: { valor: 6.2, origem: "Estimado" },
      pelo_driver: false,
      geracao_vista: true,
      nota: null,
      placa_pct: 7.5,
      leve: false,
      fps_do_jogo: 82,
      manteve_pct: 97.6,
      comparacoes: 3,
      escala,
      jogo_px: escala ? [1280, 720] : null,
      tela_px: escala ? [1920, 1080] : null,
    },
  };
  return structuredClone(fbDemo);
}

let miraDemo = false;
let fovDemo: FovFivem = { atual: 5, pedido: null, pendente: false, fivem_aberto: false, alterado: false, maximo: 10 };
let contadorDemo: ContadorFivem = { ligado: false, pendente: false };
let coresDemo: EstadoDasCores = {
  caminho: "Nvidia",
  suporta: { saturacao: true, brilho: true, contraste: true, gama: true, matiz: false },
  aplicado: false,
  perfis: { por_jogo: {}, so_com_jogo: true },
  saturacao_atual: 50,
  nota: "Pelo driver da NVIDIA (Vibração Digital): a mesma opção do Painel de Controle da NVIDIA.",
};

// ── Pontes ─────────────────────────────────────────────────────────────────────────────────────────────────────────
export const ferramentas = {
  frameBoost: (): Promise<PainelFrameBoost> =>
    noTauri ? invoke<PainelFrameBoost>("frame_boost_painel") : Promise.resolve(structuredClone(fbDemo)),
  /** `multiplicador` 0 = Auto (enche o monitor); 1 = só escala (vale com `escala`). */
  frameBoostLigar: (multiplicador: number, escala: NivelDeEscala | null): Promise<PainelFrameBoost> =>
    noTauri
      ? invoke<PainelFrameBoost>("frame_boost_ligar", { multiplicador, semLimite: false, escala })
      : Promise.resolve(fbLigadoDemo(multiplicador, escala)),
  frameBoostDesligar: (): Promise<PainelFrameBoost> => {
    if (noTauri) return invoke<PainelFrameBoost>("frame_boost_desligar");
    fbDemo = {
      ...fbDemo,
      ativo: null,
      ultima_sessao: { jogo: "FiveM", minutos: 12.4, fps_do_jogo: 82, fps_na_tela: { valor: 158, origem: "Medido" }, manteve_pct: 97.6, comparacoes: 6, desligou_por_perda: false },
    };
    return Promise.resolve(structuredClone(fbDemo));
  },
  frameBoostDestravarBase: (): Promise<PainelFrameBoost> =>
    noTauri ? invoke<PainelFrameBoost>("frame_boost_destravar_base") : Promise.resolve(structuredClone(fbDemo)),
  frameBoostTravarBase: (): Promise<PainelFrameBoost> =>
    noTauri ? invoke<PainelFrameBoost>("frame_boost_travar_base") : Promise.resolve(structuredClone(fbDemo)),
  frameBoostMedir: (): Promise<PainelFrameBoost> =>
    noTauri ? invoke<PainelFrameBoost>("frame_boost_medir") : Promise.resolve(structuredClone(fbDemo)),

  /** O jogo aberto e o id com que os perfis (cores) são guardados: o do catálogo, ou o executável em minúsculas. */
  jogoAberto: async (): Promise<{ id: string; nome: string } | null> => {
    if (!noTauri) return { id: "fivem", nome: "FiveM" };
    const j = await invoke<{ nome: string; executavel: string; no_catalogo: boolean; perfil: { id: string } } | null>("jogo_aberto");
    return j ? { id: j.no_catalogo ? j.perfil.id : j.executavel.toLowerCase(), nome: j.nome } : null;
  },
  miraEstado: (): Promise<{ ligada: boolean; jogo: string | null }> =>
    noTauri ? invoke("mira_estado") : Promise.resolve({ ligada: miraDemo, jogo: miraDemo ? "FiveM" : null }),
  /** Recusa com "CONFIRMAR:<motivo>" quando o anticheat pede cuidado (servidor de RP): aí liga com `confirmado`. */
  miraLigar: async (mira: Mira, confirmado: boolean): Promise<{ ligada: boolean; jogo: string | null }> => {
    if (noTauri) return invoke("mira_ligar", { mira, confirmado });
    miraDemo = true;
    return { ligada: true, jogo: "FiveM" };
  },
  miraAtualizar: (mira: Mira): Promise<void> => (noTauri ? invoke("mira_atualizar", { mira }) : Promise.resolve()),
  miraDesligar: async (): Promise<void> => {
    if (noTauri) return invoke("mira_desligar");
    miraDemo = false;
  },
  /** Os comandos do console do CS2 que desenham esta mira no próprio jogo. */
  miraCs2: (mira: Mira): Promise<string> =>
    noTauri ? invoke("mira_cs2", { mira }) : Promise.resolve("cl_crosshairstyle 4; cl_crosshairsize 2.5"),

  fovFivem: (): Promise<FovFivem> => (noTauri ? invoke("fov_fivem") : Promise.resolve({ ...fovDemo })),
  fovFivemDefinir: async (posicao: number): Promise<FovFivem> => {
    if (noTauri) return invoke("fov_fivem_definir", { posicao });
    fovDemo = { ...fovDemo, atual: posicao, alterado: true };
    return { ...fovDemo };
  },
  fovFivemDevolver: async (): Promise<FovFivem> => {
    if (noTauri) return invoke("fov_fivem_devolver");
    fovDemo = { ...fovDemo, atual: 5, alterado: false };
    return { ...fovDemo };
  },

  contadorFivem: (): Promise<ContadorFivem> => (noTauri ? invoke("contador_fivem_estado") : Promise.resolve({ ...contadorDemo })),
  contadorFivemDefinir: async (ligado: boolean): Promise<ContadorFivem> => {
    if (noTauri) return invoke(ligado ? "contador_fivem_ligar" : "contador_fivem_desligar");
    contadorDemo = { ...contadorDemo, ligado };
    return { ...contadorDemo };
  },

  coresEstado: (): Promise<EstadoDasCores> => (noTauri ? invoke("cores_estado") : Promise.resolve(structuredClone(coresDemo))),
  coresAplicar: async (ajuste: AjusteDeCor): Promise<CaminhoDasCores> => {
    if (noTauri) return invoke("cores_aplicar", { ajuste });
    coresDemo = { ...coresDemo, aplicado: true, saturacao_atual: ajuste.saturacao };
    return coresDemo.caminho;
  },
  coresRestaurar: async (): Promise<void> => {
    if (noTauri) return invoke("cores_restaurar");
    coresDemo = { ...coresDemo, aplicado: false, saturacao_atual: 50 };
  },
  /** `ajuste` null tira o perfil do jogo. */
  coresSalvarPerfil: async (jogo: string | null, ajuste: AjusteDeCor | null, soComJogo: boolean): Promise<EstadoDasCores["perfis"]> => {
    if (noTauri) return invoke("cores_salvar_perfil", { jogo, ajuste, soComJogo });
    const por_jogo = { ...coresDemo.perfis.por_jogo };
    if (jogo && ajuste) por_jogo[jogo] = ajuste;
    else if (jogo) delete por_jogo[jogo];
    coresDemo = { ...coresDemo, perfis: { por_jogo, so_com_jogo: soComJogo } };
    return structuredClone(coresDemo.perfis);
  },
};
