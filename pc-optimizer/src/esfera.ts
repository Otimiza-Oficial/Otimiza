/**
 * A máquina como nuvem de pontos: cada propriedade sai de uma leitura (pontos ← núcleos lógicos; vibração ← uso de
 * CPU; casco ← memória livre; cor ← veredito). Sem leitura fica esparsa e parada. Obedece ao interruptor global de
 * animação (`--anim` zero ou `.sem-animacao`): desenha UM quadro e para.
 */

export interface LeituraDaEsfera {
  nucleos: number;
  cpu: number;
  memoria: number;
  nivel: "ok" | "importante" | "critico";
}

interface Ponto {
  x: number;
  y: number;
  z: number;
  fase: number;
}

const COR = {
  ok: "10, 10, 10",
  importante: "138, 97, 0",
  critico: "180, 35, 24",
} as const;

/** Latitude e longitude sorteadas agrupam nos polos; a espiral de Fibonacci distribui de verdade. */
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
    // Piso para continuar densa com 2 núcleos; teto para 32 núcleos não virar um disco branco.
    const quantidade = Math.round(Math.min(4200, Math.max(1400, nucleos * 300)));

    if (this.pontos.length !== quantidade) this.pontos = semear(quantidade);
  }

  atualizar(leitura: LeituraDaEsfera) {
    this.semearPara(leitura.nucleos);
    this.leitura = leitura;
  }

  /** Veredito e monitor chegam por caminhos diferentes: sem isto, frase vermelha ao lado de esfera neutra. */
  definirNivel(nivel: LeituraDaEsfera["nivel"]) {
    this.leitura = this.leitura
      ? { ...this.leitura, nivel }
      : { nucleos: 8, cpu: 0, memoria: 0, nivel };
  }

  /** `avancar` falso em máquina fraca: o mesmo desenho uma vez. */
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

    // Sem medição não inventa movimento.
    const cpu = medida ? Math.min(100, Math.max(0, medida.cpu)) : 0;
    const memoria = medida ? Math.min(100, Math.max(0, medida.memoria)) : 0;
    const cor = COR[medida?.nivel ?? "ok"];

    if (avancar) {
      this.giro += 0.0015 + (cpu / 100) * 0.004;
    }

    const sen = Math.sin(this.giro);
    const cos = Math.cos(this.giro);

    const vazios = memoria / 140;
    const tremor = (cpu / 100) * 1.6;

    for (let i = 0; i < this.pontos.length; i += 1) {
      const p = this.pontos[i];

      // Sorteio ESTÁVEL por ponto, senão a esfera cintilaria.
      if (vazios > 0 && ((p.fase * 7919) % 1000) / 1000 < vazios) continue;

      const x = p.x * cos - p.z * sen;
      const z = p.x * sen + p.z * cos;

      const balanco = avancar
        ? Math.sin(this.giro * 6 + p.fase) * tremor
        : 0;

      const px = meioX + x * raio + balanco;
      const py = meioY + p.y * raio + balanco;

      // O volume vem da SILHUETA: perto da borda a vista atravessa mais pontos, e ela acumula brilho.
      const frente = (z + 1) / 2;

      const daBorda = Math.sqrt(x * x + p.y * p.y);
      const silhueta = daBorda * daBorda;

      const tamanho = 0.6 + frente * 1.5;
      const opacidade = Math.min(1, 0.06 + frente * 0.55 + silhueta * 0.5);

      ctx.fillStyle = `rgba(${cor}, ${opacidade.toFixed(3)})`;
      ctx.fillRect(px, py, tamanho, tamanho);
    }
  }

  ligar() {
    this.parar();

    const parado = document.body.classList.contains("sem-animacao")
      || Number(getComputedStyle(document.body).getPropertyValue("--anim") || 1) === 0;

    if (parado) {
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

  redesenhar() {
    if (this.quadro === null) this.desenhar(false);
  }
}

export default Esfera;
