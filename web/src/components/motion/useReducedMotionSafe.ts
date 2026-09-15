"use client";

import { useSyncExternalStore } from "react";

const QUERY = "(prefers-reduced-motion: reduce)";

const assinar = (aoMudar: () => void) => {
  const mq = window.matchMedia(QUERY);
  mq.addEventListener("change", aoMudar);
  return () => mq.removeEventListener("change", aoMudar);
};

/**
 * A preferência de movimento reduzido, SEM quebrar a hidratação.
 *
 * O `useReducedMotion` do Framer responde `null` no servidor e `true` no
 * cliente de quem tem a preferência ligada. Um componente que decide o que
 * renderizar em cima disso produz HTML diferente dos dois lados — e foi o que
 * aconteceu: a página quebrava a hidratação justamente para quem usa essa
 * opção de acessibilidade. Pego num navegador com "reduzir movimento" ligado.
 *
 * `useSyncExternalStore` com o valor de servidor fixo em `false` resolve: a
 * hidratação usa `false` dos dois lados, e o React troca para a preferência
 * real logo em seguida, sem descompasso.
 */
export function useReducedMotionSafe() {
  return useSyncExternalStore(
    assinar,
    () => window.matchMedia(QUERY).matches,
    () => false,
  );
}
