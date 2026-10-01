"use client";

/*
 * RotatingText — adaptado de React Bits (https://reactbits.dev/text-animations/rotating-text).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: `framer-motion`; a pílula tem a largura da palavra mais longa (todas empilhadas invisíveis na
 * mesma célula), então nada em volta pula a cada troca; parado fora da tela, com a aba escondida e para quem pede
 * menos movimento (mostra a primeira palavra); o leitor de tela não ouve as trocas — quem usa o componente dá a frase
 * inteira por fora.
 */
import { AnimatePresence, motion, useInView } from "framer-motion";
import { useEffect, useRef, useState } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

export function RotatingText({
  palavras,
  intervalo = 2400,
  className = "",
  pilulaClassName = "",
}: {
  palavras: string[];
  intervalo?: number;
  className?: string;
  pilulaClassName?: string;
}) {
  const [i, setI] = useState(0);
  const ref = useRef<HTMLSpanElement>(null);
  const naTela = useInView(ref);
  const reduce = useReducedMotionSafe();

  useEffect(() => {
    if (reduce || !naTela) return;
    const t = setInterval(() => {
      if (!document.hidden) setI((n) => (n + 1) % palavras.length);
    }, intervalo);
    return () => clearInterval(t);
  }, [reduce, naTela, intervalo, palavras.length]);

  const palavra = palavras[reduce ? 0 : i];

  return (
    <span ref={ref} aria-hidden="true" className={`relative inline-grid overflow-hidden align-baseline ${pilulaClassName}`}>
      {/* Todas as palavras na mesma célula, invisíveis: a maior define a largura da pílula. */}
      {palavras.map((p) => (
        <span key={p} className={`invisible col-start-1 row-start-1 ${className}`}>
          {p}
        </span>
      ))}
      <span className="col-start-1 row-start-1">
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.span key={palavra} className={`inline-flex ${className}`}>
            {Array.from(palavra).map((c, k) => (
              <motion.span
                key={k}
                className="inline-block"
                initial={{ y: "110%", opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                exit={{ y: "-110%", opacity: 0 }}
                transition={reduce ? { duration: 0 } : { type: "spring", damping: 26, stiffness: 320, delay: k * 0.025 }}
              >
                {c}
              </motion.span>
            ))}
          </motion.span>
        </AnimatePresence>
      </span>
    </span>
  );
}
