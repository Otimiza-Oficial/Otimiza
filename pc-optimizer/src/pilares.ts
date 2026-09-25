/**
 * Os três pilares da máquina (processador, memória, disco): cada coluna se desfaz conforme a SUA medição, nada
 * escolhido por gosto; sem medição ficam inteiras e apagadas. Um campo de luz geométrico reduzido a uma grade
 * grossa por pontilhado ordenado (Bayer). Desenho PARADO, uma vez por leitura: animação que engasga desmentiria
 * o produto num PC fraco.
 */

export interface LeituraDosPilares {
  cpu: number;
  memoria: number;
  disco: number;
  nivel: "ok" | "importante" | "critico";
}

export interface OpcoesDosPilares {
  /** Para encostar num bloco de texto sem borda reta entre os dois. */
  dissolve?: "esquerda" | "direita" | "nenhum";

  /** O controle da granulação: menor é mais fino e mais caro. */
  passo?: number;
}

const COR = {
  ok: "233, 231, 227",
  importante: "232, 178, 58",
  critico: "255, 92, 115",
} as const;

/** Cada posição tem o SEU limiar: um único limiar daria manchas chapadas. Dividir por 16 põe o limiar entre 0 e 1. */
const BAYER = [
  [0, 8, 2, 10],
  [12, 4, 14, 6],
  [3, 11, 1, 9],
  [15, 7, 13, 5],
];

/** Estável, sem `Math.random`: redesenhado a cada medição, o sorteio faria os cacos dançarem. */
function ruido(x: number, y: number): number {
  const n = Math.sin(x * 127.1 + y * 311.7) * 43758.5453;
  return n - Math.floor(n);
}

interface Coluna {
  centro: number;
  meia: number;
  integridade: number;
}

export class Pilares {
  private readonly ctx: CanvasRenderingContext2D | null;

  private leitura: LeituraDosPilares | null = null;

  private readonly dissolve: "esquerda" | "direita" | "nenhum";

  private readonly passo: number;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    opcoes: OpcoesDosPilares = {}
  ) {
    this.ctx = canvas.getContext("2d");
    this.dissolve = opcoes.dissolve ?? "nenhum";
    this.passo = Math.max(3, Math.round(opcoes.passo ?? 6));

    // Desenha já: sem isto o canvas fica preto até o monitor responder.
    this.desenhar();
  }

  atualizar(leitura: LeituraDosPilares) {
    this.leitura = leitura;
    this.desenhar();
  }

  /** Veredito e monitor chegam por caminhos diferentes: sem isto, frase vermelha ao lado de imagem neutra. */
  definirNivel(nivel: LeituraDosPilares["nivel"]) {
    this.leitura = this.leitura
      ? { ...this.leitura, nivel }
      : { cpu: 0, memoria: 0, disco: 0, nivel };

    this.desenhar();
  }

  /** Em fração do quadro, para servir às duas telas. */
  private luz(x: number, y: number, colunas: Coluna[]): number {
    // Cortadas embaixo pela moldura: parecem continuar para fora, e não apoiadas no nada.
    const PISO = 1.02;
    const TOPO_DO_FUSTE = 0.12;

    let melhor = 0;

    for (const c of colunas) {
      const u = (x - c.centro) / c.meia;

      const yRuina = PISO - (PISO - TOPO_DO_FUSTE) * c.integridade;

      const largo = Math.abs(u) <= 1.42;
      const noCapitel = largo && y > yRuina && y < yRuina + 0.055;
      const naBase = largo && y > PISO - 0.075 && y < PISO;

      let valor = 0;

      if (noCapitel || naBase) {
        const chanfro = 1 - Math.pow(Math.abs(u) / 1.42, 6);
        valor = (0.2 + 0.62 * chanfro) * (naBase ? 0.9 : 1);
      } else if (Math.abs(u) <= 1 && y >= TOPO_DO_FUSTE && y <= PISO) {
        // `asin` dá o ângulo da normal do cilindro a partir da posição horizontal.
        const angulo = Math.asin(Math.max(-1, Math.min(1, u)));

        // Sem direção de luz o cilindro não tem volume.
        const lambert = Math.max(0, Math.cos(angulo + 0.55));

        // Poucas e fracas: vinte sulcos fortes viravam ruído quadriculado na grade grossa.
        const canelura = 0.5 + 0.5 * Math.cos(angulo * 14);

        // O expoente alto estreita o realce: senão as colunas viravam lajes brancas.
        valor = Math.pow(lambert, 1.45) * (0.88 + 0.12 * canelura);

        // Sem as juntas a altura deixava de ser legível.
        const junta = Math.abs(((y * 11) % 1) - 0.5) * 2;
        valor *= 0.74 + 0.26 * Math.min(1, junta * 6);

        valor *= 0.86 + 0.14 * (y - TOPO_DO_FUSTE);
      } else {
        continue;
      }

      // O ruído estável transforma um degrau reto num desmoronamento.
      if (y < yRuina) {
        const acima = (yRuina - y) / 0.26;
        valor -= acima * (0.55 + 0.9 * ruido(x * 900, y * 900));
      }

      if (valor > melhor) melhor = valor;
    }

    // Empurra os tons para os extremos: no meio tudo vira o mesmo xadrez.
    return Math.max(0, Math.min(1, (melhor - 0.52) * 1.55 + 0.46));
  }

  private desenhar() {
    const ctx = this.ctx;
    if (!ctx) return;

    const largura = this.canvas.width;
    const altura = this.canvas.height;
    const passo = this.passo;

    ctx.clearRect(0, 0, largura, altura);

    const m = this.leitura;

    // Ruína por padrão seria acusação inventada contra quem acabou de instalar.
    const colunas: Coluna[] = [
      // Cerca de sete diâmetros de altura (dórica): mais gorda lê como pilastra de garagem. Larguras diferentes para não
      // virar padrão repetido.
      { centro: 0.23, meia: 0.052, integridade: m ? 1 - m.cpu / 100 : 1 },
      { centro: 0.5, meia: 0.06, integridade: m ? 1 - m.memoria / 100 : 1 },
      { centro: 0.775, meia: 0.048, integridade: m ? 1 - m.disco / 100 : 1 },
    ];

    ctx.fillStyle = `rgb(${COR[m?.nivel ?? "ok"]})`;

    const lado = Math.max(1, passo - 1);

    for (let py = 0; py < altura; py += passo) {
      const fy = (py + passo / 2) / altura;
      const linha = BAYER[(py / passo) & 3];

      for (let px = 0; px < largura; px += passo) {
        const fx = (px + passo / 2) / largura;

        let valor = this.luz(fx, fy, colunas);
        if (valor <= 0) continue;

        // Entra no valor, não é máscara: as células rareiam e o preto continua puro.
        if (this.dissolve === "direita") {
          valor *= Math.max(0, Math.min(1, (0.92 - fx) / 0.42));
        } else if (this.dissolve === "esquerda") {
          valor *= Math.max(0, Math.min(1, (fx - 0.08) / 0.42));
        }

        if (valor > (linha[(px / passo) & 3] + 0.5) / 16) {
          ctx.fillRect(px, py, lado, lado);
        }
      }
    }
  }

  redesenhar() {
    this.desenhar();
  }
}

export default Pilares;
