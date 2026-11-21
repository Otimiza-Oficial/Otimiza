// Movimento sóbrio (1.0.12, Frente 1.6): as transições do Svelte rodam pela API de animação do navegador, que a regra
// global do `tokens.css` (`animation: none` com "reduzir movimento" e com jogo aberto) não alcança. Toda transição em
// JavaScript pergunta aqui antes de durar alguma coisa.

/** Verdadeiro com "reduzir movimento" no Windows ou com um jogo aberto (a casca põe `jogo-aberto` na raiz). */
export function movimentoParado(): boolean {
  if (typeof window === "undefined") return true;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches || document.documentElement.classList.contains("jogo-aberto");
}

/** A duração pedida, ou zero quando o movimento está parado. */
export function duracao(ms: number): number {
  return movimentoParado() ? 0 : ms;
}
