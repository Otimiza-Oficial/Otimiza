"use client";

/*
 * Tilt — adaptado do TiltedCard de React Bits (https://reactbits.dev/components/tilted-card).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: inclina qualquer conteúdo (o original era uma imagem com legenda); escuta o mouse no card em
 * volta (`.card`), não só em cima da tela, então a tela reage ao mouse no card inteiro; inclinação curta; desligado
 * em tela de toque e para quem pede menos movimento.
 */
import { motion, useSpring } from "framer-motion";
import { useEffect, useRef, type ReactNode } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

const MOLA = { damping: 30, stiffness: 120, mass: 1.6 };

export function Tilt({ children, graus = 6, escala = 1.02, className = "" }: { children: ReactNode; graus?: number; escala?: number; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotionSafe();
  const rotateX = useSpring(0, MOLA);
  const rotateY = useSpring(0, MOLA);
  const scale = useSpring(1, MOLA);

  useEffect(() => {
    const el = ref.current;
    const alvo = (el?.closest(".card") as HTMLElement | null) ?? el?.parentElement;
    if (!el || !alvo || reduce || !window.matchMedia("(hover: hover) and (pointer: fine)").matches) return;
    const mover = (e: PointerEvent) => {
      const r = alvo.getBoundingClientRect();
      const dx = (e.clientX - r.left) / r.width - 0.5;
      const dy = (e.clientY - r.top) / r.height - 0.5;
      rotateX.set(-dy * 2 * graus);
      rotateY.set(dx * 2 * graus);
    };
    const entrar = () => scale.set(escala);
    const sair = () => {
      rotateX.set(0);
      rotateY.set(0);
      scale.set(1);
    };
    alvo.addEventListener("pointermove", mover, { passive: true });
    alvo.addEventListener("pointerenter", entrar);
    alvo.addEventListener("pointerleave", sair);
    return () => {
      alvo.removeEventListener("pointermove", mover);
      alvo.removeEventListener("pointerenter", entrar);
      alvo.removeEventListener("pointerleave", sair);
    };
  }, [reduce, graus, escala, rotateX, rotateY, scale]);

  return (
    <div className={`[perspective:900px] ${className}`}>
      <motion.div ref={ref} className="relative h-full w-full [transform-style:preserve-3d]" style={{ rotateX, rotateY, scale }}>
        {children}
      </motion.div>
    </div>
  );
}
