"use client";

/*
 * BlurText — adaptado de React Bits (https://reactbits.dev/text-animations/blur-text).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: `framer-motion` no lugar de `motion/react` (mesma API); escolhe a tag (o título continua
 * sendo <h1>); leitor de tela recebe a frase inteira, não palavra por palavra; menos movimento = aparece pronto,
 * com a mesma árvore (a hidratação não quebra).
 */
import { motion } from "framer-motion";
import { useEffect, useRef, useState, type ElementType } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

export function BlurText({
  text,
  as: Tag = "p",
  id,
  className = "",
  delay = 90,
  stepDuration = 0.32,
  distancia = 14,
}: {
  text: string;
  as?: ElementType;
  id?: string;
  className?: string;
  /** Atraso entre palavras, em ms. */
  delay?: number;
  stepDuration?: number;
  /** De onde cada palavra desce, em px (o original usava 50: alto demais para um título de 34 px). */
  distancia?: number;
}) {
  const reduce = useReducedMotionSafe();
  const ref = useRef<HTMLElement>(null);
  const [visto, setVisto] = useState(false);
  const palavras = text.split(" ");

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const olho = new IntersectionObserver(
      ([e]) => {
        if (e.isIntersecting) {
          setVisto(true);
          olho.disconnect();
        }
      },
      { threshold: 0.1 },
    );
    olho.observe(el);
    return () => olho.disconnect();
  }, []);

  const de = { filter: "blur(10px)", opacity: 0, y: -distancia };
  const para = { filter: ["blur(10px)", "blur(5px)", "blur(0px)"], opacity: [0, 0.5, 1], y: [-distancia, 3, 0] };

  return (
    <Tag ref={ref} id={id} className={className} aria-label={text}>
      {palavras.map((p, i) => (
        <motion.span
          key={i}
          aria-hidden="true"
          initial={de}
          animate={visto || reduce ? para : de}
          transition={
            reduce ? { duration: 0 } : { duration: stepDuration * 2, times: [0, 0.5, 1], delay: (i * delay) / 1000, ease: "easeOut" }
          }
          style={{ display: "inline-block", willChange: "transform, filter, opacity" }}
        >
          {p}
          {i < palavras.length - 1 && " "}
        </motion.span>
      ))}
    </Tag>
  );
}
