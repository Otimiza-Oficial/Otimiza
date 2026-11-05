// O céu é o diagnóstico: cada nuvem é um aviso que veio de um dado lido, nunca enfeite. Na Onda 0 a casca só sabe
// olhar a tela e a memória; o resto (rede, temperatura, disco) entra com os sistemas do plano. Por isso o céu limpo
// diz "nada de errado no que eu já sei olhar", e não "seu PC está perfeito".

import type { Hardware, MetricasAoVivo } from "./ponte";

export type Gravidade = "atencao" | "serio";

export interface Aviso {
  id: string;
  titulo: string;
  detalhe: string;
  gravidade: Gravidade;
}

export type Clima = "limpo" | "nuvens" | "nublado" | "tempestade";

export type EstadoDoMascote = "atento" | "focado" | "descansando" | "preocupado";

export type Tema = "auto" | "por-do-sol" | "lavanda" | "noite";

export type Hora = "manha" | "tarde" | "por-do-sol" | "noite";

/** Abaixo disso o monitor está claramente fora do que aguenta (60 em 75 Hz ainda é escolha comum, não descuido). */
const FOLGA_DE_HZ = 1.2;

/** Memória ocupada sem jogo aberto: acima disso, o jogo vai começar disputando. */
const MEMORIA_CHEIA = 0.85;

export function avisos(hw: Hardware | null, m: MetricasAoVivo | null, jogoAberto: boolean): Aviso[] {
  const lista: Aviso[] = [];

  for (const tela of hw?.telas ?? []) {
    if (tela.hz_maximo > 0 && tela.hz_atual > 0 && tela.hz_maximo >= tela.hz_atual * FOLGA_DE_HZ) {
      lista.push({
        id: `hz-${tela.dispositivo}`,
        titulo: `Seu monitor está em ${tela.hz_atual} Hz e aguenta ${tela.hz_maximo}`,
        detalhe: `${tela.nome}: o Windows está mostrando menos quadros do que a tela consegue.`,
        gravidade: "serio",
      });
    }
  }

  if (!jogoAberto && m && m.ram.total_gb > 0 && m.ram.used_gb / m.ram.total_gb >= MEMORIA_CHEIA) {
    const livre = Math.max(0, m.ram.total_gb - m.ram.used_gb);
    lista.push({
      id: "memoria",
      titulo: `Só ${livre.toFixed(1).replace(".", ",")} GB de memória livre`,
      detalhe: "O jogo vai abrir disputando memória com o que já está aberto.",
      gravidade: "atencao",
    });
  }

  return lista;
}

export function climaDe(lista: Aviso[]): Clima {
  if (lista.length === 0) return "limpo";
  if (lista.length >= 3) return "nublado";
  return "nuvens";
}

export function horaDe(data: Date): Hora {
  const h = data.getHours();
  if (h >= 5 && h < 11) return "manha";
  if (h >= 11 && h < 17) return "tarde";
  if (h >= 17 && h < 19) return "por-do-sol";
  return "noite";
}

export function fraseDoMascote(clima: Clima, jogo: string | null, lista: Aviso[]): string {
  if (jogo) return `${jogo} aberto. Fiquei quietinho para não pesar no jogo.`;
  if (clima === "limpo") return "Nada de errado no que eu já sei olhar.";
  if (lista.length === 1) return "Achei uma coisa atrapalhando.";
  return `Achei ${lista.length} coisas atrapalhando.`;
}
