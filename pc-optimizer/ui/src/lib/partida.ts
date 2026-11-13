// O cartão da partida (Plano "Partida de respeito", fase 1): quando o jogo fecha, o Rust conta quanto tempo ele ficou
// aberto (`partida:terminou`) e a tela junta isso ao relatório que a medição automática já montou
// (`relatorio_da_ultima_partida`). Funções puras, testadas em `ui/testes/partida.test.ts`.
//
// A medição automática grava a primeira janela três minutos depois de o jogo abrir e depois uma a cada vinte: partida
// curta não tem número, e o cartão diz isso em vez de mostrar a partida de outro dia.

/** Uma área pede "Jogar" à casca (que fala pelo Oti): `new CustomEvent(JOGAR, { detail: nome })`. */
export const JOGAR = "otimiza:jogar";

/** O que o cartão usa do relatório (`RelatorioDaPartida` em `tipos.ts`). */
export type RelatorioUsado = {
  jogo: string;
  inicio: number;
  fim: number;
  janelas_comparaveis: number;
  fps_medio: number | null;
  pior_low_1pct: number | null;
  trancos: number | null;
  janelas_com_trancos: number;
  /** `f64` no Rust: um NaN chega como `null`, e aí não há faixa para comparar. */
  anteriores: { partidas: number; mediana_fps: number | null; menor_fps: number | null; maior_fps: number | null } | null;
  /** A rede nas mesmas janelas (`plataforma::rede_partida`). Ausente em versão antiga, `null` sem ping. */
  rede?: RedeUsada | null;
};

export type RedeUsada = { firmeza: "Firme" | "Oscilou" | "Perdeu"; frase: string };

/** `partida:terminou` (`plataforma::partida::Terminada`). */
export type Terminada = { jogo: string; executavel: string; caminho?: string | null; segundos: number; pelo_jogar: boolean };

/** Cada janela da medição automática dura isto (`medicoes::SEGUNDOS_DE_MEDICAO`): base dos trancos por minuto. */
export const SEGUNDOS_DA_JANELA = 20;
/** A primeira janela sai três minutos depois de o jogo abrir (`medicoes::SEGUNDOS_ANTES_DA_PRIMEIRA`). */
export const PRIMEIRA_JANELA_S = 180;

export type Comparacao = { sentido: "acima" | "dentro" | "abaixo"; partidas: number; mediana: number };

export type Cartao =
  | {
      medido: true;
      jogo: string;
      duracao: string;
      fps: number | null;
      low: number | null;
      trancosPorMinuto: number | null;
      comparacao: Comparacao | null;
      /** "Foi a rede ou o PC?" (Plano de refino, R7). */
      rede: RedeUsada | null;
      pelo_jogar: boolean;
    }
  | { medido: false; jogo: string; duracao: string; porque: "curta" | "sem-medicao"; pelo_jogar: boolean };

/** O mesmo jogo com nome de outra fonte ("Roblox Player" e "Roblox"), como `chaveDoJogo` em `capas.ts`. */
function chave(nome: string): string {
  const c = nome.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase().replace(/[^a-z0-9]/g, "");
  return ({ robloxplayer: "roblox", robloxplayerbeta: "roblox", fivemclient: "fivem" } as Record<string, string>)[c] ?? c;
}

export function duracao(segundos: number): string {
  const min = Math.max(1, Math.round(segundos / 60));
  if (min < 60) return `${min} min`;
  const h = Math.floor(min / 60);
  const resto = min % 60;
  return resto ? `${h}h${String(resto).padStart(2, "0")}` : `${h}h`;
}

/** O relatório é DESTA partida? Mesmo jogo e terminado depois que ela começou (com folga de um minuto do vigia). */
export function eDestaPartida(t: Terminada, r: RelatorioUsado | null, agora_s: number): r is RelatorioUsado {
  if (!r) return false;
  const comecou = agora_s - t.segundos;
  const mesmo = chave(r.jogo) === chave(t.jogo) || chave(r.jogo).includes(chave(t.jogo)) || chave(t.jogo).includes(chave(r.jogo));
  return mesmo && r.fim >= comecou - 60;
}

/** Acima, dentro ou abaixo da faixa das partidas anteriores. Faixa, não prova: lugares do mapa mudam o número. */
export function comparar(fps: number | null, anteriores: RelatorioUsado["anteriores"]): Comparacao | null {
  if (fps == null || !anteriores || anteriores.partidas < 2) return null;
  const { menor_fps: menor, maior_fps: maior, mediana_fps: mediana } = anteriores;
  if (menor == null || maior == null || mediana == null) return null;
  const sentido = fps > maior ? "acima" : fps < menor ? "abaixo" : "dentro";
  return { sentido, partidas: anteriores.partidas, mediana };
}

export function montarCartao(t: Terminada, r: RelatorioUsado | null, agora_s: number): Cartao {
  const base = { jogo: t.jogo, duracao: duracao(t.segundos), pelo_jogar: t.pelo_jogar };
  if (!eDestaPartida(t, r, agora_s) || r.janelas_comparaveis === 0) {
    return { ...base, medido: false, porque: t.segundos < PRIMEIRA_JANELA_S + 30 ? "curta" : "sem-medicao" };
  }
  const minutosLidos = (r.janelas_com_trancos * SEGUNDOS_DA_JANELA) / 60;
  return {
    ...base,
    medido: true,
    fps: r.fps_medio,
    low: r.pior_low_1pct,
    trancosPorMinuto: r.trancos != null && minutosLidos > 0 ? r.trancos / minutosLidos : null,
    comparacao: comparar(r.fps_medio, r.anteriores),
    rede: r.rede ?? null,
  };
}

/** A partida com tranco a partir daqui ganha o "Deixar liso" (o Jogo liso considera liso abaixo de 2 por minuto; 5 é
 *  tranco que se sente). Só com placa NVIDIA: o teto vai pelo perfil do driver dela. */
export const TRANCOS_PARA_OFERECER_LISO = 5;

export function ofereceLiso(c: Cartao | null, placaNvidia: boolean): boolean {
  return !!c && c.medido && placaNvidia && c.fps != null && (c.trancosPorMinuto ?? 0) >= TRANCOS_PARA_OFERECER_LISO;
}

const int = (v: number) => Math.round(v).toLocaleString("pt-BR");

/** A frase da comparação, com a faixa dita como faixa. */
export function fraseDaComparacao(c: Comparacao): string {
  const n = `${c.partidas} partidas anteriores`;
  if (c.sentido === "acima") return `Acima de todas as suas ${n} (mediana ${int(c.mediana)}).`;
  if (c.sentido === "abaixo") return `Abaixo de todas as suas ${n} (mediana ${int(c.mediana)}).`;
  return `Dentro da faixa das suas ${n} (mediana ${int(c.mediana)}).`;
}

/** O texto para colar no Discord: só número medido, com o nome de cada um. */
export function textoParaCompartilhar(c: Extract<Cartao, { medido: true }>): string {
  const partes = [
    c.fps != null ? `${int(c.fps)} FPS médio` : null,
    c.low != null ? `1% pior ${int(c.low)}` : null,
    c.trancosPorMinuto != null ? `${c.trancosPorMinuto.toLocaleString("pt-BR", { maximumFractionDigits: 1 })} trancos/min` : null,
  ].filter(Boolean);
  const comparacao = c.comparacao ? ` ${fraseDaComparacao(c.comparacao)}` : "";
  return `${c.jogo} · ${c.duracao} de partida: ${partes.join(" · ")}.${comparacao} Medido pelo Otimiza.`;
}
