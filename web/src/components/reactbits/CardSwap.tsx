"use client";

/*
 * CardSwap — adaptado de React Bits (https://reactbits.dev/components/card-swap).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: `framer-motion` no lugar de GSAP (o site já usa; nada novo para baixar); o card da frente
 * cai e volta para o fundo da pilha num só caminho; pausa com o mouse em cima, fora da tela e com a aba escondida;
 * menos movimento = pilha parada; o leitor de tela ignora a pilha (é ilustração).
 */
import { motion, useInView } from "framer-motion";
import { Children, useEffect, useRef, useState, type ReactNode } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

type Vaga = { x: number; y: number; z: number; zIndex: number };

const vaga = (i: number, dx: number, dy: number, total: number): Vaga => ({ x: i * dx, y: -i * dy, z: -i * dx * 1.5, zIndex: total - i });

export function CardSwap({
  children,
  largura = 440,
  altura = 300,
  distanciaX = 46,
  distanciaY = 40,
  intervalo = 4200,
  inclinacao = 4,
  className = "",
}: {
  children: ReactNode;
  largura?: number;
  altura?: number;
  distanciaX?: number;
  distanciaY?: number;
  intervalo?: number;
  inclinacao?: number;
  className?: string;
}) {
  const cards = Children.toArray(children);
  const total = cards.length;
  const caixa = useRef<HTMLDivElement>(null);
  const naTela = useInView(caixa, { margin: "-10% 0px" });
  const reduce = useReducedMotionSafe();
  const [giro, setGiro] = useState(0);
  const [sobre, setSobre] = useState(false);

  useEffect(() => {
    if (reduce || sobre || !naTela || total < 2) return;
    const t = setInterval(() => {
      if (!document.hidden) setGiro((g) => g + 1);
    }, intervalo);
    return () => clearInterval(t);
  }, [reduce, sobre, naTela, total, intervalo]);

  // Posição de cada card na pilha: 0 = frente.
  const posicoes = cards.map((_, i) => (((i - giro) % total) + total) % total);

  return (
    <div
      ref={caixa}
      aria-hidden="true"
      onMouseEnter={() => setSobre(true)}
      onMouseLeave={() => setSobre(false)}
      className={`relative ${className}`}
      style={{ width: largura, height: altura, perspective: 900 }}
    >
      {cards.map((card, i) => {
        const p = posicoes[i];
        const v = vaga(p, distanciaX, distanciaY, total);
        // O card que acabou de ir para o fundo é sempre o que estava na frente: ele cai, passa por baixo e volta.
        const desceu = giro > 0 && p === total - 1;
        const frente = vaga(0, distanciaX, distanciaY, total);
        return (
          <motion.div
            key={i}
            className="absolute top-1/2 left-1/2 [transform-style:preserve-3d] [backface-visibility:hidden]"
            style={{ width: largura, height: altura, zIndex: v.zIndex, x: "-50%", y: "-50%", skewY: inclinacao }}
            initial={false}
            animate={
              desceu
                ? { translateX: [frente.x, frente.x, v.x], translateY: [frente.y, frente.y + 420, v.y], translateZ: [frente.z, frente.z, v.z] }
                : { translateX: v.x, translateY: v.y, translateZ: v.z }
            }
            transition={
              reduce
                ? { duration: 0 }
                : desceu
                  ? { duration: 1.6, times: [0, 0.45, 1], ease: [0.22, 1, 0.36, 1] }
                  : { type: "spring", stiffness: 120, damping: 18, delay: 0.25 + p * 0.08 }
            }
          >
            {card}
          </motion.div>
        );
      })}
    </div>
  );
}
