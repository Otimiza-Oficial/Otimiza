// O relógio central da interface (1.0.12, Frente 3.1). No lugar de um `setInterval` por componente (eram 22), um só
// temporizador acorda na hora do próximo compromisso e chama quem está devido. Com a janela escondida (o jogo na frente,
// o Otimiza na bandeja) nada que consulta o Windows roda: a UI escondida não pergunta nada. Ao voltar, quem pediu
// `aoVoltar` é chamado na hora, para a tela não mostrar número velho.
//
// O núcleo (`Relogio`) recebe o tempo e o "estou visível?" de fora, para o teste (`testes/relogio.test.ts`).

export interface OpcoesDoCompromisso {
  /** Roda mesmo com a janela escondida. Só para contador local (segundos de uma medição), nunca para consulta. */
  escondida?: boolean;
  /** Ao voltar a aparecer, chama na hora em vez de esperar o próximo intervalo. */
  aoVoltar?: boolean;
}

interface Compromisso extends Required<OpcoesDoCompromisso> {
  id: number;
  ms: number;
  proximo: number;
  fn: () => void;
}

export interface Ambiente {
  agora: () => number;
  visivel: () => boolean;
  agendar: (fn: () => void, ms: number) => unknown;
  cancelar: (t: unknown) => void;
}

export class Relogio {
  #compromissos = new Map<number, Compromisso>();
  #seq = 0;
  #timer: unknown = null;
  #ambiente: Ambiente;

  constructor(ambiente: Ambiente) {
    this.#ambiente = ambiente;
  }

  /** Chama `fn` a cada `ms`. Devolve a função que desliga. */
  cada(ms: number, fn: () => void, opcoes: OpcoesDoCompromisso = {}): () => void {
    const id = ++this.#seq;
    this.#compromissos.set(id, {
      id,
      ms: Math.max(16, ms),
      proximo: this.#ambiente.agora() + ms,
      fn,
      escondida: opcoes.escondida ?? false,
      aoVoltar: opcoes.aoVoltar ?? false,
    });
    this.#reagendar();
    return () => {
      this.#compromissos.delete(id);
      this.#reagendar();
    };
  }

  /** Quantos compromissos estão ligados (para o teste e para o log de peso). */
  get ativos() {
    return this.#compromissos.size;
  }

  /** A janela voltou a aparecer: quem pediu `aoVoltar` roda agora. */
  voltou() {
    const agora = this.#ambiente.agora();
    for (const c of this.#compromissos.values())
      if (c.aoVoltar) {
        c.proximo = agora + c.ms;
        this.#chamar(c);
      }
    this.#reagendar();
  }

  /** Roda o que está devido e marca o próximo de cada um. Público para o teste; na tela, o temporizador chama. */
  tique() {
    this.#timer = null;
    const agora = this.#ambiente.agora();
    const visivel = this.#ambiente.visivel();
    for (const c of [...this.#compromissos.values()]) {
      if (c.proximo > agora) continue;
      // Atrasou (máquina dormiu, aba congelada): pula para o próximo múltiplo, sem rajada de chamadas.
      c.proximo = agora + c.ms;
      if (visivel || c.escondida) this.#chamar(c);
    }
    this.#reagendar();
  }

  #chamar(c: Compromisso) {
    try {
      c.fn();
    } catch (e) {
      // Um compromisso que quebra não derruba os outros; o erro vai para o console (e para o log, pelo `main.ts`).
      queueMicrotask(() => {
        throw e;
      });
    }
  }

  #reagendar() {
    if (this.#timer !== null) this.#ambiente.cancelar(this.#timer);
    this.#timer = null;
    if (this.#compromissos.size === 0) return;
    const proximo = Math.min(...[...this.#compromissos.values()].map((c) => c.proximo));
    this.#timer = this.#ambiente.agendar(() => this.tique(), Math.max(0, proximo - this.#ambiente.agora()));
  }
}

const naTela = typeof window !== "undefined" && typeof document !== "undefined";

/** O relógio da janela. */
export const relogio = new Relogio({
  agora: () => (typeof performance !== "undefined" ? performance.now() : Date.now()),
  visivel: () => !naTela || !document.hidden,
  agendar: (fn, ms) => setTimeout(fn, ms),
  cancelar: (t) => clearTimeout(t as ReturnType<typeof setTimeout>),
});

if (naTela) document.addEventListener("visibilitychange", () => !document.hidden && relogio.voltou());
