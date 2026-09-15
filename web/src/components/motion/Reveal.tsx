"use client";

import { motion } from "framer-motion";
import type { ReactNode } from "react";
import { useReducedMotionSafe } from "./useReducedMotionSafe";

/**
 * Entrada de conteúdo: 8px de subida e opacidade, uma vez só.
 *
 * O movimento existe para dar sensação de acabamento, não para ser notado.
 *
 * A PRIMEIRA VERSÃO TROCAVA DE COMPONENTE com movimento reduzido — `<div>` no
 * cliente, `motion.div` no servidor —, e isso quebrava a hidratação para quem
 * usa a opção de acessibilidade. Agora a árvore é sempre a mesma, e o que muda
 * é só a duração: zero, para quem pediu menos movimento.
 */
export function Reveal({
  children,
  delay = 0,
  className,
}: {
  children: ReactNode;
  delay?: number;
  className?: string;
}) {
  const reduce = useReducedMotionSafe();

  return (
    <motion.div
      className={className}
      initial={{ opacity: 0, y: 8 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "0px 0px -10% 0px" }}
      transition={reduce ? { duration: 0 } : { duration: 0.5, delay, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </motion.div>
  );
}
