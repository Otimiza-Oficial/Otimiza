/**
 * A MÁQUINA, DESENHADA COM AS PRÓPRIAS MEDIÇÕES.
 *
 * Uma nuvem de pontos em forma de esfera. Cada ponto é uma amostra; a esfera é
 * o computador do cliente.
 *
 * ELA NÃO É ENFEITE, E ISSO É A DECISÃO INTEIRA DESTE ARQUIVO.
 *
 * O produto tem uma regra fundadora: nunca mostrar um número que não foi
 * medido. Uma imagem bonita e inventada no meio de uma tela de medições
 * contradiz isso em silêncio — o cliente não tem como saber que aquela metade
 * da tela é decoração e a outra é dado.
 *
 * Então cada propriedade visual sai de uma leitura de verdade:
 *
 *   quantos pontos   ←  núcleos lógicos da CPU
 *   como eles vibram ←  uso de CPU agora
 *   quanto do casco  ←  memória livre; a esfera se esvazia conforme enche
 *   a cor            ←  o veredito: neutro, âmbar ou vermelho
 *
 * Sem leitura nenhuma, ela desenha o estado "não medido" — esparsa e parada —
 * em vez de fingir movimento. Silêncio aqui é honestidade, não tela vazia.
 *
 * O CUSTO, QUE NUM OTIMIZADOR NÃO É DETALHE
 *
 * Este produto é vendido com a promessa de deixar PC fraco mais rápido. Uma
 * animação que engasgue na máquina que ele deveria estar consertando o
 * desmente antes de aplicar a primeira otimização.
 *
 * Por isso ela obedece ao mesmo interruptor global do resto da interface: com
 * `--anim` em zero, ou com `.sem-animacao` no corpo, a esfera desenha UM quadro
 * e para. Continua sendo a mesma imagem, e custa uma vez.
 */

export interface LeituraDaEsfera {
  /** Núcleos lógicos. Decide quantos pontos existem. */
  nucleos: number;
  /** Uso de CPU agora, de 0 a 100. Decide o quanto eles vibram. */
  cpu: number;
  /** Uso de memória, de 0 a 100. Decide o quanto do casco se abre. */
  memoria: number;
  /** O veredito, que decide a cor. */
  nivel: "ok" | "importante" | "critico";
}

interface Ponto {
  /** Posição na esfera unitária. */
  x: number;
  y: number;
  z: number;
  /** Fase própria, para os pontos não pulsarem em uníssono. */
  fase: number;
}

const COR = {
  ok: "233, 231, 227",
  importante: "232, 178, 58",
  critico: "255, 92, 115",
} as const;

/**
 * Distribui pontos por igual sobre a esfera.
 *
 * Sortear latitude e longitude ao acaso AGRUPA nos polos — é o erro clássico, e
 * fica visível: a esfera ganha duas manchas. A espiral de Fibonacci distribui
 * de verdade, e é o que faz a nuvem parecer uma superfície em vez de um
 * amontoado.
 */
function semear(quantidade: number): Ponto[] {
  const pontos: Ponto[] = [];
  const dourado = Math.PI * (3 - Math.sqrt(5));

  for (let i = 0; i < quantidade; i += 1) {
    const y = 1 - (i / (quantidade - 1)) * 2;
    const raio = Math.sqrt(Math.max(0, 1 - y * y));
    const teta = dourado * i;

    pontos.push({
      x: Math.cos(teta) * raio,
      y,
      z: Math.sin(teta) * raio,
      fase: (i * 97) % 360,
    });
  }

  return pontos;
}

export class Esfera {
  private readonly ctx: CanvasRenderingContext2D | null;

  private pontos: Ponto[] = [];

  private leitura: LeituraDaEsfera | null = null;

  private giro = 0;

  private quadro: number | null = null;

  constructor(private readonly canvas: HTMLCanvasElement) {
    this.ctx = canvas.getContext("2d");
    this.semearPara(8);
  }

  private semearPara(nucleos: number) {
    // Entre 1.400 e 4.200 pontos. O piso existe para a esfera continuar densa
    // num PC de 2 núcleos — abaixo disso ela vira uma peneira e perde a forma.
    // O teto existe para um processador de 32 núcleos não virar um disco branco:
    // passada certa densidade a silhueta some e o custo sobe à toa.
    const quantidade = Math.round(Math.min(4200, Math.max(1400, nucleos * 300)));

    if (this.pontos.length !== quantidade) this.pontos = semear(quantidade);
  }

  /** Recebe uma medição nova. Chamar a cada leitura do monitor. */
  atualizar(leitura: LeituraDaEsfera) {
    this.semearPara(leitura.nucleos);
    this.leitura = leitura;
  }

  /**
   * Troca só a cor, sem esperar a próxima medição.
   *
   * O veredito e o monitor chegam por caminhos diferentes e em velocidades
   * diferentes. Sem isto, a tela conseguia mostrar uma frase vermelha ao lado
   * de uma esfera neutra — dois pedaços da mesma tela discordando sobre o
   * estado da máquina, que é exatamente o tipo de coisa que faz o cliente
   * duvidar do diagnóstico inteiro.
   */
  definirNivel(nivel: LeituraDaEsfera["nivel"]) {
    this.leitura = this.leitura
      ? { ...this.leitura, nivel }
      : { nucleos: 8, cpu: 0, memoria: 0, nivel };
  }

  /**
   * Desenha um quadro.
   *
   * `avancar` só é verdade quando a interface pode se mexer. Em máquina fraca
   * ele vem falso, e o mesmo desenho acontece uma vez.
   */
  private desenhar(avancar: boolean) {
    const ctx = this.ctx;
    if (!ctx) return;

    const largura = this.canvas.width;
    const altura = this.canvas.height;
    const meioX = largura / 2;
    const meioY = altura / 2;
    const raio = Math.min(largura, altura) * 0.42;

    ctx.clearRect(0, 0, largura, altura);

    const medida = this.leitura;

    // SEM MEDIÇÃO, A ESFERA NÃO INVENTA MOVIMENTO. Ela fica esparsa e parada —
    // que é a aparência honesta de "ainda não sei nada desta máquina".
    const cpu = medida ? Math.min(100, Math.max(0, medida.cpu)) : 0;
    const memoria = medida ? Math.min(100, Math.max(0, medida.memoria)) : 0;
    const cor = COR[medida?.nivel ?? "ok"];

    if (avancar) {
      // Gira mais rápido sob carga. É a leitura mais direta do desenho: uma
      // máquina ocupada tem uma esfera inquieta.
      this.giro += 0.0015 + (cpu / 100) * 0.004;
    }

    const sen = Math.sin(this.giro);
    const cos = Math.cos(this.giro);

    // A memória abre buracos no casco: quanto mais cheia, mais pontos somem.
    // É a leitura que o cliente deste produto mais precisa ver — a máquina que
    // motivou o Otimiza trava por falta de memória.
    const vazios = memoria / 140;
    const tremor = (cpu / 100) * 1.6;

    for (let i = 0; i < this.pontos.length; i += 1) {
      const p = this.pontos[i];

      // Um sorteio ESTÁVEL por ponto: o mesmo ponto some sempre, em vez de a
      // esfera cintilar inteira a cada quadro.
      if (vazios > 0 && ((p.fase * 7919) % 1000) / 1000 < vazios) continue;

      const x = p.x * cos - p.z * sen;
      const z = p.x * sen + p.z * cos;

      const balanco = avancar
        ? Math.sin(this.giro * 6 + p.fase) * tremor
        : 0;

      const px = meioX + x * raio + balanco;
      const py = meioY + p.y * raio + balanco;

      // PROFUNDIDADE E SILHUETA — o que faz uma nuvem de pontos parecer um
      // corpo, e não um borrão.
      //
      // Profundidade sozinha (o de trás mais apagado) dá uma bola cinza chapada.
      // O que dá volume de verdade é a SILHUETA: num casco de pontos, a vista
      // atravessa mais material perto da borda do que no centro, então a borda
      // acumula brilho. É o mesmo motivo pelo qual uma bolha de sabão tem o
      // contorno mais nítido que o meio.
      const frente = (z + 1) / 2;

      // Distância do centro na tela, de 0 (meio) a 1 (borda).
      const daBorda = Math.sqrt(x * x + p.y * p.y);
      const silhueta = daBorda * daBorda;

      const tamanho = 0.6 + frente * 1.5;
      const opacidade = Math.min(1, 0.06 + frente * 0.55 + silhueta * 0.5);

      ctx.fillStyle = `rgba(${cor}, ${opacidade.toFixed(3)})`;
      ctx.fillRect(px, py, tamanho, tamanho);
    }
  }

  /** Liga o desenho. Respeita o interruptor de movimento do produto. */
  ligar() {
    this.parar();

    const parado = document.body.classList.contains("sem-animacao")
      || Number(getComputedStyle(document.body).getPropertyValue("--anim") || 1) === 0;

    if (parado) {
      // Um quadro, e acabou. A imagem continua a mesma; o custo é pago uma vez.
      this.desenhar(false);
      return;
    }

    const laco = () => {
      this.desenhar(true);
      this.quadro = requestAnimationFrame(laco);
    };

    this.quadro = requestAnimationFrame(laco);
  }

  parar() {
    if (this.quadro !== null) cancelAnimationFrame(this.quadro);
    this.quadro = null;
  }

  /** Redesenha uma vez. Para quando a medição muda com a animação desligada. */
  redesenhar() {
    if (this.quadro === null) this.desenhar(false);
  }
}

export default Esfera;
