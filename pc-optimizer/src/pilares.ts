/**
 * OS TRÊS PILARES DA MÁQUINA.
 *
 * Uma coluna clássica para cada coisa que sustenta o desempenho de um PC:
 *
 *   1º pilar   ←  processador
 *   2º pilar   ←  memória
 *   3º pilar   ←  disco
 *
 * Cada uma se desfaz de cima para baixo conforme a SUA medição. Um computador
 * inteiro mostra três colunas de pé; um com a memória estourada mostra a do
 * meio comida pela metade, com os cacos boiando por cima.
 *
 * POR QUE ISTO NÃO É PAPEL DE PAREDE
 *
 * O produto tem uma regra fundadora: nunca mostrar um número que não foi
 * medido. Uma imagem bonita e inventada no meio de uma tela de medições
 * contradiz isso em silêncio — o cliente não tem como saber que aquela metade
 * da tela é decoração e a outra é dado.
 *
 * Então aqui não há nenhum valor escolhido por gosto. A altura de cada ruína
 * sai de uma leitura que acabou de chegar do backend, e a cor sai do veredito.
 * Sem medição nenhuma, as três colunas ficam inteiras e apagadas — que é a
 * aparência honesta de "ainda não sei nada desta máquina", e não uma ruína
 * inventada para assustar quem acabou de instalar.
 *
 * COMO O DESENHO É FEITO
 *
 * Em duas etapas, e a segunda é a que dá a aparência.
 *
 * 1. Uma função diz, para qualquer ponto, o quanto aquele lugar é claro: as
 *    colunas são cilindros iluminados de um lado, com caneluras, capitel e
 *    base. É tudo geometria — não existe imagem carregada de lugar nenhum.
 *
 * 2. Esse campo de luz é reduzido a uma GRADE GROSSA de quadrados, por
 *    pontilhado ordenado (matriz de Bayer). É a técnica de meio-tom das
 *    impressoras: com um único tom de tinta, a densidade dos pontos é que
 *    produz a impressão de sombra.
 *
 * O CUSTO, QUE NUM OTIMIZADOR NÃO É DETALHE
 *
 * Este produto é vendido com a promessa de deixar PC fraco mais rápido. Uma
 * animação que engasgue na máquina que ele deveria estar consertando o
 * desmente antes de aplicar a primeira otimização.
 *
 * Por isso o desenho é PARADO: acontece uma vez por leitura do monitor, e não
 * sessenta vezes por segundo. E a grade é o que torna isso barato — são
 * milhares de células, não centenas de milhares de pixels.
 */

export interface LeituraDosPilares {
  /** Uso do processador agora, de 0 a 100. */
  cpu: number;
  /** Uso da memória, de 0 a 100. */
  memoria: number;
  /** Uso do disco, de 0 a 100. */
  disco: number;
  /** O veredito, que decide a cor. */
  nivel: "ok" | "importante" | "critico";
}

export interface OpcoesDosPilares {
  /**
   * Para que lado a imagem se dissolve no preto.
   *
   * Serve para a imagem encostar num bloco de texto sem uma borda reta
   * separando os dois — o corte duro entregaria que são dois retângulos
   * colados, em vez de uma página só.
   */
  dissolve?: "esquerda" | "direita" | "nenhum";

  /**
   * O tamanho de cada quadrado da grade, em pixels.
   *
   * É o controle da granulação: menor dá uma imagem mais fina e mais cara,
   * maior dá o aspecto de meio-tom grosso.
   */
  passo?: number;
}

const COR = {
  ok: "233, 231, 227",
  importante: "232, 178, 58",
  critico: "255, 92, 115",
} as const;

/**
 * A matriz de Bayer 4×4.
 *
 * Em vez de decidir cada célula com um único limiar — o que daria manchas
 * chapadas e uma borda dura entre claro e escuro —, cada posição da grade tem
 * o SEU limiar. O resultado é o padrão xadrez que se vê nas zonas de meio-tom,
 * e é ele que faz a sombra parecer contínua com uma tinta só.
 *
 * Os números são a ordem canônica da matriz; dividir por 16 põe cada limiar
 * entre 0 e 1.
 */
const BAYER = [
  [0, 8, 2, 10],
  [12, 4, 14, 6],
  [3, 11, 1, 9],
  [15, 7, 13, 5],
];

/**
 * Ruído estável, sem `Math.random`.
 *
 * Precisa ser estável porque o desenho é refeito a cada medição: com sorteio
 * de verdade, os cacos da ruína dançariam de lugar a cada dois segundos e a
 * imagem parada viraria um chuvisco. Com esta função, a mesma célula recebe
 * sempre o mesmo valor, e só a MEDIÇÃO muda o desenho.
 */
function ruido(x: number, y: number): number {
  const n = Math.sin(x * 127.1 + y * 311.7) * 43758.5453;
  return n - Math.floor(n);
}

/** Uma coluna: onde ela está e o quanto dela ainda está de pé. */
interface Coluna {
  /** Centro, em fração da largura. */
  centro: number;
  /** Meia-largura do fuste, em fração da largura. */
  meia: number;
  /** De 0 (ruína completa) a 1 (inteira). */
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

    // DESENHA JÁ, ANTES DA PRIMEIRA MEDIÇÃO.
    //
    // Sem isto o canvas fica vazio até o monitor responder — e se o backend
    // demorar, falhar ou a máquina estiver ocupada, o cliente encara metade da
    // tela de ativação em preto liso. As colunas inteiras são o estado "ainda
    // não medi nada", e é ele que tem que aparecer enquanto se espera.
    this.desenhar();
  }

  /** Recebe uma medição nova e redesenha. */
  atualizar(leitura: LeituraDosPilares) {
    this.leitura = leitura;
    this.desenhar();
  }

  /**
   * Troca só a cor, sem esperar a próxima medição.
   *
   * O veredito e o monitor chegam por caminhos diferentes e em velocidades
   * diferentes. Sem isto, a tela conseguia mostrar uma frase vermelha ao lado
   * de uma imagem neutra — dois pedaços da mesma tela discordando sobre o
   * estado da máquina, que é o tipo de coisa que faz o cliente duvidar do
   * diagnóstico inteiro.
   */
  definirNivel(nivel: LeituraDosPilares["nivel"]) {
    this.leitura = this.leitura
      ? { ...this.leitura, nivel }
      : { cpu: 0, memoria: 0, disco: 0, nivel };

    this.desenhar();
  }

  /**
   * O quanto o ponto (x, y) é claro, de 0 a 1.
   *
   * `x` e `y` vêm em fração do quadro, para o desenho não depender do tamanho
   * do canvas: a mesma cena serve para a tela de ativação e para a de chegada.
   */
  private luz(x: number, y: number, colunas: Coluna[]): number {
    // O piso e o teto da cena. As colunas são cortadas embaixo pela moldura —
    // é o corte que dá a impressão de que elas continuam para fora da tela, em
    // vez de estarem apoiadas no nada.
    const PISO = 1.02;
    const TOPO_DO_FUSTE = 0.12;

    let melhor = 0;

    for (const c of colunas) {
      const u = (x - c.centro) / c.meia;

      // A altura em que esta coluna se rompe. Quanto mais usada a peça que ela
      // representa, mais baixa a linha da ruína.
      const yRuina = PISO - (PISO - TOPO_DO_FUSTE) * c.integridade;

      // ---- capitel e base: blocos mais largos que o fuste ----------------
      const largo = Math.abs(u) <= 1.42;
      const noCapitel = largo && y > yRuina && y < yRuina + 0.055;
      const naBase = largo && y > PISO - 0.075 && y < PISO;

      let valor = 0;

      if (noCapitel || naBase) {
        // Bloco quase chapado, com um leve arredondamento nas pontas e uma
        // linha de sombra no meio da altura — o suficiente para não virar um
        // retângulo branco.
        const chanfro = 1 - Math.pow(Math.abs(u) / 1.42, 6);
        valor = (0.2 + 0.62 * chanfro) * (naBase ? 0.9 : 1);
      } else if (Math.abs(u) <= 1 && y >= TOPO_DO_FUSTE && y <= PISO) {
        // ---- o fuste: um cilindro canelado -------------------------------
        //
        // A normal da superfície de um cilindro visto de frente varia com o
        // ângulo: o meio aponta para quem olha, as beiradas apontam para os
        // lados. `asin` devolve esse ângulo a partir da posição horizontal.
        const angulo = Math.asin(Math.max(-1, Math.min(1, u)));

        // Luz vinda de cima e da esquerda. É o que separa uma coluna de uma
        // barra cinza: sem direção de luz, o cilindro não tem volume.
        const lambert = Math.max(0, Math.cos(angulo + 0.55));

        // As caneluras — os sulcos verticais da coluna clássica. São o detalhe
        // que faz a forma ser lida como coluna e não como cano.
        //
        // SÃO POUCAS E SÃO FRACAS, de propósito. Com vinte sulcos e um contraste
        // forte, a grade grossa não tinha células suficientes para descrever
        // cada um: o padrão virava ruído quadriculado e a coluna perdia o
        // volume que as caneluras deveriam reforçar. Sete sulcos suaves cabem
        // na resolução que temos.
        const canelura = 0.5 + 0.5 * Math.cos(angulo * 14);

        // O EXPOENTE ALTO É O QUE ESTREITA O REALCE.
        //
        // Com uma queda suave, o fuste inteiro ficava acima do limiar e as três
        // colunas viravam lajes brancas chapadas: nenhuma forma, nenhum volume.
        // Elevado, o brilho fica restrito a uma faixa e o resto do cilindro cai
        // para a zona granulada — que é onde o meio-tom mostra a curvatura.
        valor = Math.pow(lambert, 1.45) * (0.88 + 0.12 * canelura);

        // AS JUNTAS DOS TAMBORES.
        //
        // Coluna clássica não é peça única: é uma pilha de cilindros de pedra.
        // Sem as juntas, o fuste virava uma listra vertical uniforme de cima a
        // baixo, e a altura da coluna deixava de ser legível — nada marcava a
        // distância percorrida pelo olho.
        const junta = Math.abs(((y * 11) % 1) - 0.5) * 2;
        valor *= 0.74 + 0.26 * Math.min(1, junta * 6);

        // Um afinamento suave para cima (a êntase grega), que tira o aspecto
        // de tubo extrudado.
        valor *= 0.86 + 0.14 * (y - TOPO_DO_FUSTE);
      } else {
        continue;
      }

      // ---- a ruína ------------------------------------------------------
      //
      // Acima da linha de ruptura a pedra se desfaz: quanto mais alto, menos
      // sobrou. O ruído estável é o que transforma um degrau reto num
      // desmoronamento com cacos soltos.
      if (y < yRuina) {
        const acima = (yRuina - y) / 0.26;
        valor -= acima * (0.55 + 0.9 * ruido(x * 900, y * 900));
      }

      if (valor > melhor) melhor = valor;
    }

    // CONTRASTE, E É ELE QUE FAZ O MEIO-TOM PARECER MEIO-TOM.
    //
    // Sem esta curva o desenho inteiro caía na faixa do meio, e um valor de
    // meio produz sempre o mesmo xadrez: a imagem virava um retângulo
    // quadriculado uniforme, sem áreas cheias nem vazias. O que dá a aparência
    // de impressão é a CONVIVÊNCIA de branco sólido, preto sólido e transição
    // granulada — e para isso os tons precisam ser empurrados para os extremos.
    return Math.max(0, Math.min(1, (melhor - 0.52) * 1.55 + 0.46));
  }

  /** Desenha a cena inteira. Uma vez por medição — não é um laço. */
  private desenhar() {
    const ctx = this.ctx;
    if (!ctx) return;

    const largura = this.canvas.width;
    const altura = this.canvas.height;
    const passo = this.passo;

    ctx.clearRect(0, 0, largura, altura);

    const m = this.leitura;

    // SEM MEDIÇÃO, AS COLUNAS FICAM INTEIRAS.
    //
    // O contrário — ruína por padrão — seria uma acusação inventada contra a
    // máquina de quem acabou de instalar o programa. Não sabemos nada ainda, e
    // a imagem tem que dizer isso.
    const colunas: Coluna[] = [
      // A PROPORÇÃO É CLÁSSICA, E NÃO ESCOLHIDA NO OLHO.
      //
      // Coluna dórica tem cerca de sete diâmetros de altura. Mais gorda que
      // isso, ela lê como pilastra de garagem — foi o que aconteceu na
      // primeira tentativa, e o desenho perdia a referência inteira.
      //
      // As larguras diferem de propósito: três colunas idênticas viram um
      // padrão repetido, e um padrão não parece um lugar.
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

        // A dissolução na direção do texto. Não é máscara por cima: entra no
        // próprio valor, então as células vão RAREANDO em vez de a imagem
        // inteira ficar translúcida — que é o que mantém o preto puro.
        if (this.dissolve === "direita") {
          valor *= Math.max(0, Math.min(1, (0.92 - fx) / 0.42));
        } else if (this.dissolve === "esquerda") {
          valor *= Math.max(0, Math.min(1, (fx - 0.08) / 0.42));
        }

        // O pontilhado ordenado. Cada posição da grade tem o seu limiar, e é
        // por isso que a sombra sai granulada em vez de chapada.
        if (valor > (linha[(px / passo) & 3] + 0.5) / 16) {
          ctx.fillRect(px, py, lado, lado);
        }
      }
    }
  }

  /** Redesenha com a última medição. Para quando o canvas muda de tamanho. */
  redesenhar() {
    this.desenhar();
  }
}

export default Pilares;
