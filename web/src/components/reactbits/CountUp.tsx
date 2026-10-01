"use client";

/*
 * CountUp — adaptado de React Bits (https://reactbits.dev/text-animations/count-up).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: `framer-motion`; o HTML do servidor já traz o número FINAL (preço nunca aparece vazio ou
 * errado sem JavaScript); formato brasileiro; menos movimento = número pronto.
 */
import { useInView, useMotionValue, useSpring } from "framer-motion";
import { useEffect, useRef } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

export function CountUp({
  to,
  from = 0,
  duracao = 1.6,
  casas = 0,
  className = "",
}: {
  to: number;
  from?: number;
  duracao?: number;
  casas?: number;
  className?: string;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const reduce = useReducedMotionSafe();
  const valor = useMotionValue(from);
  const mola = useSpring(valor, { damping: 20 + 40 / duracao, stiffness: 100 / duracao });
  const visto = useInView(ref, { once: true, margin: "0px 0px -10% 0px" });
  const formato = (n: number) => n.toLocaleString("pt-BR", { minimumFractionDigits: casas, maximumFractionDigits: casas });

  // Antes de entrar na tela, mostra o ponto de partida (só no cliente, depois da hidratação).
  useEffect(() => {
    if (reduce || visto || !ref.current) return;
    ref.current.textContent = formato(from);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reduce]);

  useEffect(() => {
    if (reduce) {
      if (ref.current) ref.current.textContent = formato(to);
      return;
    }
    const fim = mola.on("change", (v) => {
      if (ref.current) ref.current.textContent = formato(v);
    });
    if (visto) valor.set(to);
    return fim;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visto, reduce, to]);

  return (
    <span ref={ref} className={className}>
      {formato(to)}
    </span>
  );
}
