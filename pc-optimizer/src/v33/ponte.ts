// A conversa da tela nova com o backend: tipos e chamadas num lugar só. Os nomes espelham `src-tauri/src/plataforma`.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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
  /** Windows modificado ("lite", "anti-lag"), quando a imagem se apresenta assim no registro. */
  windows_modificado?: { nome: string; autor: string | null } | null;
  lido_em: number;
}

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

export interface Compatibilidade {
  hardware: Hardware;
  recursos: RecursoAvaliado[];
  frame_boost_auto: RecursoAvaliado | null;
}

export interface PerfilDoJogo {
  id: string;
  nome: string;
  executaveis: string[];
  steam_appid: number | null;
  anticheat: string;
  api: string;
  sobreposicao: "Segura" | { Desaconselhada: string } | { Barrada: string };
  mira_no_jogo: boolean;
  fov: "NaoTem" | "NoArquivo" | "AindaNaoMapeado";
  nota: string | null;
}

export interface JogoAberto {
  pid: number;
  executavel: string;
  nome: string;
  perfil: PerfilDoJogo;
  no_catalogo: boolean;
  geracao_propria: string[];
  recursos: RecursoAvaliado[];
  frame_boost_auto: RecursoAvaliado | null;
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

export type Confianca = "Alta" | "Media" | "SemDiferenca" | "Insuficiente";

export interface LinhaDoLab {
  metrica: string;
  antes: number | null;
  depois: number | null;
  diferenca_pct: number | null;
  confianca: Confianca;
  menor_e_melhor: boolean;
}

export interface ResultadoDoLab {
  nome: string;
  rodadas_antes: number;
  rodadas_depois: number;
  minimo_por_lado: number;
  linhas: LinhaDoLab[];
  descartadas: number;
}

export type SituacaoDoFeedback =
  | { situacao: "NaoSeAplica" }
  | { situacao: "Contando"; restante_segundos: number }
  | { situacao: "Liberado"; verificado_em: string }
  | { situacao: "Exigido"; apagado: boolean };

export type EstadoDoFeedback = SituacaoDoFeedback & {
  maquina: string;
  canal: string | null;
  canal_id: string;
  aviso: string | null;
};

export interface Licenca {
  ativa: boolean;
  maquina: string;
  origem: string;
  sobrevive_formatacao: boolean;
  comprador: string | null;
  expira: string | null;
  motivo: string | null;
}

export interface Flags {
  novo_shell: boolean;
  nova_inicio: boolean;
  biblioteca: boolean;
  lab: boolean;
  frame_boost_auto: boolean;
  overlay: boolean;
  smooth_motion_drs: boolean;
  vibracao_nvidia: boolean;
}

export interface Resultado {
  id: string;
  name: string;
  success: boolean;
  applied: boolean;
  message: string;
  requires_restart?: boolean;
}

/** Os níveis do "Otimizar agora" que vão de uma vez (o Experimental é um grupo por vez, medindo). */
export type NivelDoOtimizar = "Seguro" | "Competitivo";

export interface NivelNaTela {
  id: string;
  nome: string;
  promessa: string;
  exigencia: string;
  itens: string[];
  acrescenta: string[];
  aplica_de_uma_vez: boolean;
}

export type SituacaoDoGerador = "Parado" | "ProcurandoJanela" | "Gerando" | "SemQuadros" | "Erro" | "DesligadoPorPerdaDeFps";

export interface Valor {
  valor: number | null;
  origem: "Medido" | "Estimado" | "Desconhecido";
}

/** O pedaço de `get_performance_metrics` que o Início usa. `null` = não medido (nunca zero inventado). */
export interface MetricasAoVivo {
  cpu: { overall: number | null };
  ram: { total_gb: number; used_gb: number };
  telemetry: { metrics: Record<string, { value: number | null } | undefined> };
}

export interface JogoDaLoja {
  nome: string;
  loja: "Steam" | "Epic Games" | "Instalado";
  appid: number | null;
  ultima_vez: number;
  gb: number;
}

/** Um item do catálogo como `list_optimizations` o entrega (o pedaço que a tela usa). */
export interface ItemDoCatalogo {
  id: string;
  name: string;
  honest_effect: string;
  state: "Applied" | "AlreadyOptimal" | "Available" | "Unavailable" | "Unknown";
  detail: string | null;
}

export interface EntradaDeInicio {
  name: string;
  executable: string;
  hive: string;
  enabled: boolean;
  classe: "Essencial" | "Util" | "Opcional" | "Desconhecido";
}

export interface ItemDeLimpeza {
  id: string;
  nome: string;
  explica: string;
  bytes: number;
  arquivos: number;
  precisa_admin: boolean;
}

export interface EnergiaDeJogo {
  ligado: boolean;
  agindo_em: string | null;
  impedimento: string | null;
  o_que_muda: string | null;
}

export interface MemoriaParaOJogo {
  total_gb: number;
  livre_gb: number;
  pesados: { nome: string; mb: number; processos: number; aviso: string | null }[];
}

/** FOV de primeira pessoa do FiveM: a posição do controle do menu do jogo, gravada no fivem.cfg. */
export interface FovFivem {
  atual: number | null;
  pedido: number | null;
  pendente: boolean;
  fivem_aberto: boolean;
  alterado: boolean;
  maximo: number;
}

export interface PlanoJogoLiso {
  fps_agora: number;
  hz: number;
  teto: number | null;
  multiplicador_do_boost: number;
  trancos_por_minuto: number;
  motivo: string;
  /** O teto do perfil que o driver usa neste jogo (0 = sem teto); null sem leitura. */
  teto_no_driver: number | null;
  aviso_do_teto: string | null;
  /** Como o quadro chegou à tela na medição. */
  tela: {
    apresentacao: "Direta" | "Composta" | "Copiada" | "Desconhecida";
    modo: string;
    por_cima: string[];
    aviso: string | null;
  } | null;
}

export interface ConfigDoJogo {
  id: string;
  nome: string;
  /** O arquivo de configuração achado; null = jogo não instalado ou nunca aberto. */
  arquivo: string | null;
  aberto: boolean;
  previsao: { chave: string; antes: string; depois: string; por_que: string }[];
  /** O arquivo existe mas não deu para ler. */
  ilegivel: string | null;
  ficha: string;
  historico: string;
  aplicado: boolean;
}

export type Gargalo = "Placa" | "Processador" | "Memoria" | "Nenhum" | "SemCarga";

export interface ReceitaDoTurbo {
  gargalo: Gargalo;
  escala: NivelDeEscala | null;
  /** 1 = só escala, 0 = Auto, 2+ = fixo; null = sem Frame Boost. */
  multiplicador: number | null;
  energia: boolean;
  memoria: boolean;
  explicacao: string;
}

export type DesfechoDaProva = "Ganhou" | "Piorou" | "Indistinguivel" | "SemComparacaoJusta";

export interface ResumoDaProva {
  n: number;
  media: number;
  margem: number | null;
}

/** A prova A-B (a mesma forma da prova alternada do plano de energia). */
export interface ProvaDoTurbo {
  jogo: string;
  quando: number;
  segundos_por_rodada: number;
  rodadas: unknown[];
  fps_sem: ResumoDaProva | null;
  fps_com: ResumoDaProva | null;
  diferenca_fps: { Real?: { delta: number; pct: number | null; folga: number } } | null;
  /** O 1% piores quadros: é o que a pessoa sente como leveza (engasgo some, ritmo firma). */
  low_sem?: ResumoDaProva | null;
  low_com?: ResumoDaProva | null;
  diferenca_low?: { Real?: { delta: number; pct: number | null; folga: number } } | null;
  desfecho: DesfechoDaProva;
  leitura: string;
}

export interface PassoDaProvaDoTurbo {
  indice: number;
  total: number;
  com_turbo: boolean | null;
  segundos: number;
}

export interface PainelTurbo {
  jogo: string | null;
  /** "Menor atraso": o Turbo não liga geração de quadros neste jogo. Escolha da pessoa; sem ela, jogo de tiro vem assim. */
  menor_atraso: boolean;
  perfil: { receita: ReceitaDoTurbo; fps_antes: number | null; medido_em: number; prova?: DesfechoDaProva | null } | null;
  prova?: ProvaDoTurbo | null;
  provando?: boolean;
  /** A prova que está rodando é a automática, da primeira vez da receita neste jogo. */
  prova_sozinha?: boolean;
  duracao_da_prova_s?: number;
  passos: string[];
  ligado: boolean;
  /** Desligado: o antes × depois da última vez (comparação simples, não prova A-B). */
  resultado?: {
    fps_antes: number;
    fps_do_jogo: number;
    ganho_pct: number;
    fps_na_tela: number | null;
    na_tela_medido: boolean;
    minutos: number;
    escala: NivelDeEscala | null;
  } | null;
}

/** Os níveis da escala, com as proporções do FSR (1,3× a 2× por eixo). */
export type NivelDeEscala = "UltraQualidade" | "Qualidade" | "Equilibrado" | "Desempenho";

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
    /** % da placa que o gerador do Otimiza ocupou (medido). */
    placa_pct: number | null;
    /** Modo leve: placa sem folga, ou a guarda aliviou depois de ver o jogo perder FPS. */
    leve: boolean;
    /** FPS do jogo agora, com o gerador ligado. */
    fps_do_jogo: number | null;
    /** Quanto do FPS o jogo mantém com o gerador (medido pela guarda), em %. */
    manteve_pct: number | null;
    comparacoes: number;
    /** Com escala: o nível, o tamanho em que o jogo renderiza e o da imagem ampliada. */
    escala?: NivelDeEscala | null;
    jogo_px?: [number, number] | null;
    tela_px?: [number, number] | null;
  } | null;
  aviso: string | null;
  /** O teto do FPS base que o Otimiza gravou para este jogo. */
  base: { fps: number; multiplicador: number; valendo: boolean } | null;
  /** O que aconteceu com o FPS base ao ligar. */
  nota_da_base: string | null;
  /** A pessoa destravou o FPS base deste jogo: o Frame Boost não trava sozinho. */
  base_solta: boolean;
  /** Gerador desligado: a última vez que ele ficou 30 s ou mais ligado. */
  ultima_sessao: {
    jogo: string;
    minutos: number;
    fps_do_jogo: number;
    fps_na_tela: Valor;
    manteve_pct: number | null;
    comparacoes: number;
    desligou_por_perda: boolean;
  } | null;
  /** O monitor do jogo e, por nível de escala, a resolução em que o jogo deve rodar. */
  monitor?: [number, number] | null;
  escalas?: { nivel: NivelDeEscala; largura: number; altura: number }[];
}

export interface EstadoDoGerador {
  situacao: SituacaoDoGerador;
  processo: string;
  multiplicador: number;
  automatico?: boolean;
  fps_reais: number;
  fps_apresentados: number;
  /** O que o PresentMon viu chegar à tela (precisa de administrador); sem ele, só o apresentado, como estimativa. */
  fps_exibidos?: number | null;
  erro: string | null;
  atraso_ms: number | null;
  escala?: NivelDeEscala | null;
}

export interface PainelAmd {
  estado: string;
  nota: string;
  ajustes: { anti_lag: boolean | null; afmf: boolean | null; rsr?: boolean | null; limite_ligado: boolean | null; limite_fps: number | null } | null;
  sensores: { uso_pct: number | null; temperatura_c: number | null } | null;
  aplicados: string[];
}

export interface PainelNvidia {
  estado: "Disponivel" | "SemPlacaNvidia" | "NaoCarregou";
  nota: string;
  ajustes: { id: string; titulo: string; explicacao: string; historico: string; aplicado: boolean }[];
}

export interface PassoDoLote {
  index: number;
  total: number;
  name: string;
  stage: string;
  message: string;
  success: boolean;
}

export interface PainelIntel {
  estado: string;
  nota: string;
  ajustes: { baixa_latencia: number | null; limite_ligado: boolean | null; limite_fps: number | null } | null;
  aplicados: string[];
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

export type ModoDoOverlay = "Desligado" | "Minimo" | "Detalhado";
export type CantoDoOverlay = "SuperiorEsquerdo" | "SuperiorDireito" | "InferiorEsquerdo" | "InferiorDireito";

export interface EstadoDoOverlay {
  config: { modo: ModoDoOverlay; canto: CantoDoOverlay; confirmados: string[]; com_frame_boost: boolean };
  ligado: boolean;
  jogo: string | null;
}

export const ponte = {
  flags: () => invoke<Flags>("flags_ler"),
  gravarFlags: (flags: Flags) => invoke<void>("flags_gravar", { flags }),
  hardwareUltimo: () => invoke<Hardware | null>("hardware_ultimo"),
  hardware: (atualizar = false) => invoke<Hardware>("hardware_perfil", { atualizar }),
  compatibilidade: () => invoke<Compatibilidade>("compatibilidade"),
  jogoAberto: () => invoke<JogoAberto | null>("jogo_aberto"),
  catalogo: () => invoke<PerfilDoJogo[]>("catalogo_de_jogos"),
  biblioteca: () => invoke<ResumoDoJogo[]>("biblioteca_de_jogos"),
  capa: (appid: number) => invoke<string | null>("capa_do_jogo", { appid }),
  capaLarga: (appid: number) => invoke<string | null>("capa_larga_do_jogo", { appid }),
  fundosListar: () => invoke<string[]>("fundos_listar"),
  jogosInstalados: () => invoke<JogoDaLoja[]>("jogos_instalados"),
  limpezaMedir: () => invoke<ItemDeLimpeza[]>("limpeza_medir"),
  limpezaExecutar: (id: string) => invoke<{ bytes_liberados: number; apagados: number; ficaram: number }>("limpeza_executar", { id }),
  fundosAdicionar: (imagem: string) => invoke<number>("fundos_adicionar", { imagem }),
  fundosLimpar: () => invoke<void>("fundos_limpar"),
  iniciarMonitor: () => invoke<void>("start_monitoring"),
  metricas: () => invoke<MetricasAoVivo>("get_performance_metrics"),
  labEstado: () => invoke<ResultadoDoLab>("lab_estado"),
  labMedir: (processo: string, lado: "Antes" | "Depois", segundos: number) =>
    invoke<ResultadoDoLab>("lab_medir", { processo, lado, segundos }),
  labZerar: () => invoke<void>("lab_zerar"),
  feedback: () => invoke<EstadoDoFeedback>("feedback_estado"),
  feedbackVerificar: () => invoke<EstadoDoFeedback>("feedback_verificar"),
  licenca: () => invoke<Licenca>("licenca_estado"),
  /** O "Otimizar agora" no nível escolhido (o Completo é o padrão desde a 3.5). */
  otimizar: (nivel: NivelDoOtimizar = "Competitivo") => invoke<Resultado[]>("optimize_now", { only: null, nivel }),
  niveis: () => invoke<NivelNaTela[]>("niveis_de_otimizacao"),
  driverAmd: () => invoke<PainelAmd>("driver_amd"),
  ajusteAmd: (opcao: string) => invoke<Resultado>("aplicar_ajuste_amd", { opcao }),
  driverIntel: () => invoke<PainelIntel>("driver_intel"),
  ajusteIntel: (opcao: string) => invoke<Resultado>("aplicar_ajuste_intel", { opcao }),
  ajusteNvidia: (opcao: string) => invoke<Resultado>("aplicar_ajuste_nvidia", { opcao }),
  driverNvidia: () => invoke<PainelNvidia>("ajustes_do_driver_nvidia"),
  restaurarTudo: () => invoke<unknown>("restaurar_tudo"),
  miraLigar: (mira: unknown, confirmado: boolean) => invoke<{ ligada: boolean; jogo: string | null }>("mira_ligar", { mira, confirmado }),
  miraAtualizar: (mira: unknown) => invoke<void>("mira_atualizar", { mira }),
  miraDesligar: () => invoke<void>("mira_desligar"),
  miraEstado: () => invoke<{ ligada: boolean; jogo: string | null }>("mira_estado"),
  miraCs2: (mira: unknown) => invoke<string>("mira_cs2", { mira }),
  coresEstado: () => invoke<EstadoDasCores>("cores_estado"),
  coresAplicar: (ajuste: AjusteDeCor) => invoke<CaminhoDasCores>("cores_aplicar", { ajuste }),
  coresRestaurar: () => invoke<void>("cores_restaurar"),
  coresSalvarPerfil: (jogo: string | null, ajuste: AjusteDeCor | null, soComJogo: boolean) =>
    invoke<EstadoDasCores["perfis"]>("cores_salvar_perfil", { jogo, ajuste, soComJogo }),
  overlayEstado: () => invoke<EstadoDoOverlay>("overlay_estado"),
  overlayGravar: (modo: ModoDoOverlay, canto: CantoDoOverlay, comFrameBoost?: boolean) =>
    invoke<EstadoDoOverlay>("overlay_gravar", { modo, canto, comFrameBoost: comFrameBoost ?? null }),
  overlayLigar: (confirmado: boolean) => invoke<EstadoDoOverlay>("overlay_ligar", { confirmado }),
  contadorFivem: () => invoke<{ ligado: boolean; pendente: boolean }>("contador_fivem_estado"),
  contadorFivemLigar: () => invoke<{ ligado: boolean; pendente: boolean }>("contador_fivem_ligar"),
  contadorFivemDesligar: () => invoke<{ ligado: boolean; pendente: boolean }>("contador_fivem_desligar"),
  fovFivem: () => invoke<FovFivem>("fov_fivem"),
  fovFivemDefinir: (posicao: number) => invoke<FovFivem>("fov_fivem_definir", { posicao }),
  fovFivemDevolver: () => invoke<FovFivem>("fov_fivem_devolver"),
  anotarErro: (texto: string) => invoke<void>("anotar_erro_da_tela", { texto }),
  convite: (embutido: string) => invoke<string>("convite_do_discord", { embutido }),
  desfazer: (id: string) => invoke<Resultado>("revert_optimization", { id }),
  itens: () => invoke<ItemDoCatalogo[]>("list_optimizations"),
  aplicarItem: (id: string) => invoke<Resultado>("apply_optimization", { id }),
  inicializacao: () => invoke<EntradaDeInicio[]>("list_startup"),
  inicializacaoDefinir: (hive: string, name: string, enabled: boolean) => invoke<Resultado>("set_startup_enabled", { hive, name, enabled }),
  /** O comparador da escala: pede o próximo quadro real com o jogo na frente e depois busca. */
  escalaPedirComparacao: () => invoke<void>("escala_pedir_comparacao"),
  escalaComparacao: () =>
    invoke<{ jogo: string; tela: string; jogo_px: [number, number]; tela_px: [number, number] } | null>("escala_comparacao"),
  configDosJogos: () => invoke<ConfigDoJogo[]>("config_motor_painel"),
  configAplicar: (jogo: string) => invoke<string[]>("config_motor_aplicar", { jogo }),
  turbo: () => invoke<PainelTurbo>("turbo_painel"),
  /** Mede o jogo (10 s) e aplica; `lembrado` usa a receita guardada do jogo, sem medir. */
  turboLigar: (lembrado = false) => invoke<PainelTurbo>("turbo_ligar", { lembrado }),
  turboDesligar: () => invoke<PainelTurbo>("turbo_desligar"),
  turboEsquecer: () => invoke<PainelTurbo>("turbo_esquecer"),
  turboAtraso: (menor: boolean) => invoke<PainelTurbo>("turbo_atraso", { menor }),
  /** A prova A-B (uns 5 minutos); o passo a passo chega pelo evento `turbo:prova`. */
  turboProvar: () => invoke<ProvaDoTurbo>("turbo_provar"),
  turboCompartilhar: () => invoke<string>("turbo_compartilhar"),
  frameBoost: () => invoke<PainelFrameBoost>("frame_boost_painel"),
  /** `escala`: amplia o jogo para o monitor (FSR 1); com `multiplicador` 1, só escala. */
  frameBoostLigar: (multiplicador: number, semLimite = false, escala: NivelDeEscala | null = null) =>
    invoke<PainelFrameBoost>("frame_boost_ligar", { multiplicador, semLimite, escala }),
  frameBoostDesligar: () => invoke<PainelFrameBoost>("frame_boost_desligar"),
  frameBoostDestravarBase: () => invoke<PainelFrameBoost>("frame_boost_destravar_base"),
  frameBoostTravarBase: () => invoke<PainelFrameBoost>("frame_boost_travar_base"),
  frameBoostMedir: () => invoke<PainelFrameBoost>("frame_boost_medir"),
  jogoLisoMedir: () => invoke<PlanoJogoLiso>("jogo_liso_medir"),
  memoriaParaOJogo: () => invoke<MemoriaParaOJogo>("memoria_para_o_jogo"),
  memoriaFechar: (nome: string) => invoke<MemoriaParaOJogo>("memoria_fechar", { nome }),
  energiaDeJogo: () => invoke<EnergiaDeJogo>("energia_de_jogo"),
  energiaDeJogoDefinir: (ligado: boolean) => invoke<EnergiaDeJogo>("energia_de_jogo_definir", { ligado }),
  jogoLisoAplicar: () => invoke<PlanoJogoLiso>("jogo_liso_aplicar"),
  geradorLigar: (processo: string, multiplicador: number) =>
    invoke<EstadoDoGerador>("gerador_ligar", { processo, multiplicador }),
  geradorDesligar: () => invoke<EstadoDoGerador>("gerador_desligar"),
  geradorEstado: () => invoke<EstadoDoGerador>("gerador_estado"),
};

export function ouvir<T>(evento: string, f: (dado: T) => void): Promise<UnlistenFn> {
  return listen<T>(evento, (e) => f(e.payload));
}

/** O motivo de qualquer suporte que não seja "suportado". */
export function motivo(s: Suporte): string | null {
  return s.estado === "Suportado" ? null : s.motivo;
}

export function usavel(s: Suporte): boolean {
  return s.estado === "Suportado" || s.estado === "Parcial";
}
