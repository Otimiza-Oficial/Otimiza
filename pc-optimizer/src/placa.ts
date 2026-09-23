/*
 * O QUE A PLACA FEZ NUMA JANELA MEDIDA — a frase, num lugar só.
 *
 * Duas telas mostram isto: a ficha do jogo (a última partida medida sozinha) e
 * o Mapa de desempenho (a janela que a pessoa acabou de medir). A conta e os
 * limiares moram no Rust (`core::sensores`); aqui fica só a redação, e ela
 * mora neste arquivo justamente para não existir duas vezes, com duas
 * redações que envelhecem separadas.
 *
 * O NÚMERO QUE IMPORTA NÃO É A TEMPERATURA. É quanto do tempo o DRIVER disse
 * que estava segurando o clock, e por quê. Placa a 78 °C com o clock cheio é
 * uma placa trabalhando; 70 °C segurando o clock metade da partida é um
 * problema físico que nenhum ajuste de Windows resolve.
 */

export type ResumoGpu = {
  amostras: number;
  temperatura_max_c: number | null;
  temperatura_media_c: number | null;
  clock_medio_mhz: number | null;
  potencia_media_w: number | null;
  limite_w: number | null;
  uso_medio_pct: number | null;
  pct_termico: number;
  pct_teto_de_energia: number;
  pct_freio_de_hardware: number;
  motivos_lidos: boolean;
};

/** Os mesmos pisos do `core::sensores` — a tela não inventa piso próprio. */
const TERMICO_ALTO_PCT = 5;
const TETO_DOMINANTE_PCT = 60;

export type FraseDaPlaca = { tom: "aviso" | "nota"; texto: string };

/** Os números crus da janela, em uma linha. */
function numerosDaPlaca(p: ResumoGpu): string {
  return [
    p.temperatura_max_c !== null ? `máx. ${Math.round(p.temperatura_max_c)} °C` : null,
    p.clock_medio_mhz !== null ? `${Math.round(p.clock_medio_mhz)} MHz` : null,
    p.potencia_media_w !== null
      ? `${Math.round(p.potencia_media_w)} W${p.limite_w !== null ? ` de ${Math.round(p.limite_w)} W` : ""}`
      : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

/**
 * O que dizer sobre a placa nesta janela. `null` quando não houve amostra —
 * máquina sem placa NVIDIA, ou medição antiga.
 *
 * `janela` é a palavra que entra na frase: "desta partida", "deste teste".
 */
export function fraseDaPlaca(p: ResumoGpu | null | undefined, janela = "desta partida"): FraseDaPlaca | null {
  if (!p || !p.amostras) return null;
  const numeros = numerosDaPlaca(p);
  const pct = (v: number) => `${v.toFixed(0)}%`;

  if (!p.motivos_lidos) {
    return {
      tom: "nota",
      texto: `<strong>Placa:</strong> ${numeros}. O driver não informou se estava segurando o clock nesta máquina.`,
    };
  }
  if (p.pct_termico >= TERMICO_ALTO_PCT) {
    return {
      tom: "aviso",
      texto: `<strong>A placa foi limitada por temperatura em ${pct(p.pct_termico)} ${janela}</strong> (${numeros}). Isso é físico: limpe a poeira, confira as ventoinhas e o fluxo de ar do gabinete. Nenhum ajuste de Windows resolve isso.`,
    };
  }
  if (p.pct_freio_de_hardware >= TERMICO_ALTO_PCT) {
    return {
      tom: "aviso",
      texto: `<strong>A placa acionou o freio de hardware em ${pct(p.pct_freio_de_hardware)} ${janela}</strong> (${numeros}). Costuma ser a fonte, o conector de energia da placa ou a proteção térmica dela.`,
    };
  }
  if (p.pct_teto_de_energia >= TETO_DOMINANTE_PCT) {
    return {
      tom: "nota",
      texto: `<strong>Placa:</strong> ${numeros}. Ela passou ${pct(p.pct_teto_de_energia)} do tempo no teto de energia — é o funcionamento normal de uma placa em carga máxima, não defeito.`,
    };
  }
  return { tom: "nota", texto: `<strong>Placa:</strong> ${numeros}. Nada segurou o clock além do normal.` };
}
