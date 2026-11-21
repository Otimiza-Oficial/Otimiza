// A leitura do Frame Boost ligado (1.0.12, Frente 1.4, A3 + U9 do plano): o multiplicador REAL (o que chegou à tela
// dividido pelo FPS do jogo), por que ele está assim e o que fazer agora. Antes a tela dizia "Auto (agora 1×)" enquanto
// gerava (o `multiplicador` do Rust é o do último quadro real, e no Auto ele oscila), e o estado não dizia o que fazer.
// Puro, para o teste (`testes/frameboost.test.ts`).

export type SituacaoDoGerador = "Gerando" | "ProcurandoJanela" | "SemQuadros" | string;

export interface LigadoAgora {
  multiplicador: number;
  automatico: boolean;
  situacao: SituacaoDoGerador;
  hz: number;
  pelo_driver: boolean;
  /** O que chegou à tela (medido ou estimado); `null` enquanto não há número. */
  fps_na_tela: number | null;
  fps_do_jogo?: number | null;
  manteve_pct?: number | null;
  degrau?: string | null;
  /** Ligado só com a escala, sem gerar quadros. */
  so_escala: boolean;
}

export type Tom = "ok" | "atencao" | "neutro";

export interface Leitura {
  /** "2,0×", "1× (só escala)" ou `null` enquanto não dá para calcular. */
  multiplicador: string | null;
  /** Por que está assim, em uma frase. */
  motivo: string;
  /** O próximo passo, em uma frase. */
  proximo: string;
  tom: Tom;
  /** Para onde o próximo passo leva, quando leva a outra área. */
  area?: "tela" | "prova";
}

/** Abaixo disto do FPS mantido, o Frame Boost está custando FPS de verdade ao jogo. */
export const MANTEVE_BOM_PCT = 90;
/** Até aqui o "real" é 1×: o gerador quase não tem o que fazer. */
const QUASE_UM = 1.15;

const n = (v: number, casas = 0) => v.toLocaleString("pt-BR", { maximumFractionDigits: casas, minimumFractionDigits: casas });

/** O multiplicador de verdade: o que chegou à tela sobre o FPS do jogo. `null` sem os dois números. */
export function multiplicadorReal(fpsNaTela: number | null | undefined, fpsDoJogo: number | null | undefined): number | null {
  if (fpsNaTela == null || fpsDoJogo == null || fpsDoJogo <= 0 || fpsNaTela <= 0) return null;
  return Math.round((fpsNaTela / fpsDoJogo) * 10) / 10;
}

export function lerFrameBoost(a: LigadoAgora): Leitura {
  if (a.situacao === "ProcurandoJanela")
    return {
      multiplicador: null,
      motivo: "Procurando a janela do jogo para se encaixar nela.",
      proximo: "Deixe o jogo em janela sem bordas e em primeiro plano; leva poucos segundos.",
      tom: "neutro",
    };
  if (a.situacao === "SemQuadros")
    return {
      multiplicador: null,
      motivo: "O jogo parou de desenhar (menu, tela de carregamento ou minimizado).",
      proximo: "Volte para o jogo: a sobreposição reaparece sozinha.",
      tom: "neutro",
    };
  if (a.so_escala)
    return {
      multiplicador: "1× (só escala)",
      motivo: "O jogo desenha menos pixels e o Otimiza amplia: o FPS a mais é do próprio jogo.",
      proximo: "Se a imagem ficar mole, suba a nitidez da escala.",
      tom: "ok",
    };

  const real = multiplicadorReal(a.fps_na_tela, a.fps_do_jogo);
  const mult = real != null ? `${n(real, 1)}×` : !a.automatico && a.multiplicador > 1 ? `${a.multiplicador}×` : null;

  if (a.pelo_driver)
    return {
      multiplicador: mult,
      motivo: "Quem gera os quadros é o driver da placa; o Otimiza só mede o que chega à tela.",
      proximo: real == null ? "Toque em “Medir agora” com o jogo rodando para ver o número de verdade." : "Para conferir o ganho no seu jogo, meça na Prova.",
      tom: real == null ? "neutro" : "ok",
      area: real == null ? undefined : "prova",
    };

  let motivo: string;
  if (a.degrau) motivo = a.degrau;
  else if (real != null && real < QUASE_UM && a.automatico)
    motivo = `O jogo já chega perto dos ${a.hz} Hz do monitor: não há quadro para gerar agora.`;
  else if (a.automatico && a.fps_do_jogo != null)
    motivo = `No Auto: o jogo dá ${n(a.fps_do_jogo)} fps e o monitor mostra até ${a.hz}, então ele gera ${mult ?? "o que falta"}.`;
  else if (!a.automatico) motivo = `Fixo em ${a.multiplicador}×: ${a.multiplicador} quadros na tela para cada um do jogo.`;
  else motivo = "No Auto: enche o monitor com o que falta de quadros.";

  if (real != null && real < QUASE_UM && a.automatico)
    return {
      multiplicador: mult,
      motivo,
      proximo: "Nada a fazer. Se o monitor aceita mais Hz, suba na área Tela e o Frame Boost passa a ter o que gerar.",
      tom: "neutro",
      area: "tela",
    };
  if (a.manteve_pct == null)
    return { multiplicador: mult, motivo, proximo: "Jogue normalmente por um minuto: o Otimiza está conferindo se o jogo perde FPS com ele.", tom: "neutro" };
  if (a.manteve_pct < MANTEVE_BOM_PCT)
    return {
      multiplicador: mult,
      motivo,
      proximo: `O jogo perdeu ${n(100 - a.manteve_pct)}% do FPS com o Frame Boost. Troque para o modo Desempenho ou para 2×, ou desligue.`,
      tom: "atencao",
    };
  return {
    multiplicador: mult,
    motivo,
    proximo: `O jogo manteve ${n(a.manteve_pct, 1)}% do FPS: está valendo. Para medir o ganho com calma, use a Prova.`,
    tom: "ok",
    area: "prova",
  };
}

/** O que a pessoa pediu ao Frame Boost quando deu erro. */
export type AcaoDoFrameBoost = "ligar" | "desligar" | "medir" | "base";

const TITULO_DA_ACAO: Record<AcaoDoFrameBoost, string> = {
  ligar: "O Frame Boost não ligou",
  desligar: "O Frame Boost não desligou",
  medir: "Não consegui medir agora",
  base: "Não consegui mudar o FPS base",
};

const SAIDA = "Se o jogo travou, Ctrl+Alt+G desliga de dentro dele.";

/** O erro de uma ação, em título e próximo passo (1.0.14). Antes todo erro virava "O Frame Boost não respondeu" e o
 *  motivo do Rust (que já é uma frase com o que fazer: "volte para o jogo", "abra como administrador") ficava recolhido
 *  no "Detalhe para o suporte": a pessoa via só "não respondeu". Agora o motivo é o próximo passo. */
export function erroDaAcao(acao: AcaoDoFrameBoost, erro: string): { titulo: string; fazer: string } {
  const motivo = erro.replace(/^Error:\s*/, "").trim();
  const fazer = motivo ? motivo : "Tente de novo.";
  return { titulo: TITULO_DA_ACAO[acao], fazer: acao === "desligar" ? `${fazer} ${SAIDA}` : fazer };
}
