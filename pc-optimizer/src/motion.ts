/** Motion não governa o estado: abrir/fechar é síncrono, CSS só apresenta a mudança. */
const abertos = new Map<HTMLElement, HTMLElement | null>();
const focaveis = 'button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex="0"]';

export function abrirOverlay(caixa: HTMLElement) {
  if (!abertos.has(caixa)) {
    abertos.set(caixa, document.activeElement instanceof HTMLElement ? document.activeElement : null);
  }
  caixa.inert = false;
  caixa.removeAttribute("aria-hidden");
  caixa.hidden = false;
}

export function fecharOverlay(caixa: HTMLElement, restaurarFoco = true) {
  const origem = abertos.get(caixa);
  // Sai do mapa mesmo se já estiver escondida: uma entrada esquecida prenderia o Tab no app inteiro.
  abertos.delete(caixa);
  if (caixa.hidden) return;
  // A saída visual nunca deixa botões ou backdrop capturando ações.
  caixa.inert = true;
  caixa.hidden = true;
  caixa.setAttribute("aria-hidden", "true");
  if (restaurarFoco && origem?.isConnected && !origem.closest('[hidden], [inert]')) {
    origem.focus({ preventScroll: true });
  }
}

export function ligarMotion() {
  const topo = () => { const caixas = [...abertos.keys()]; return caixas[caixas.length - 1]; };
  const alvos = (caixa: HTMLElement) => [...caixa.querySelectorAll<HTMLElement>(focaveis)]
    .filter(el => !el.closest('[hidden], [inert]') && el.getClientRects().length > 0);
  const teclado = (event: KeyboardEvent) => {
    const caixa = topo();
    if (!caixa || event.key !== "Tab") return;
    const itens = alvos(caixa);
    const primeiro = itens[0];
    const ultimo = itens[itens.length - 1];
    if (!primeiro) { event.preventDefault(); return; }
    if (event.shiftKey && (document.activeElement === primeiro || !caixa.contains(document.activeElement))) {
      event.preventDefault(); ultimo.focus();
    } else if (!event.shiftKey && (document.activeElement === ultimo || !caixa.contains(document.activeElement))) {
      event.preventDefault(); primeiro.focus();
    }
  };
  const foco = (event: FocusEvent) => {
    const caixa = topo();
    if (caixa && !caixa.contains(event.target as Node)) alvos(caixa)[0]?.focus({ preventScroll: true });
  };
  const visibilidade = () => {
    document.documentElement.toggleAttribute("data-motion-pausado", document.hidden);
  };
  document.addEventListener("keydown", teclado);
  document.addEventListener("focusin", foco);
  document.addEventListener("visibilitychange", visibilidade);
  visibilidade();
  return () => {
    document.removeEventListener("keydown", teclado);
    document.removeEventListener("focusin", foco);
    document.removeEventListener("visibilitychange", visibilidade);
    abertos.clear();
  };
}
