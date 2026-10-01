"use client";

/*
 * ScrollReveal — adaptado de React Bits (https://reactbits.dev/text-animations/scroll-reveal).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: `framer-motion` (o site já usa) no lugar de GSAP/ScrollTrigger; a frase acende palavra por
 * palavra conforme o título sobe na tela, sem girar o bloco; leitor de tela recebe a frase inteira; menos movimento
 * = texto pronto.
 */
import { motion, useScroll, useTransform, type MotionValue } from "framer-motion";
import { useRef } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

function Palavra({ texto, progresso, de, ate }: { texto: string; progresso: MotionValue<number>; de: number; ate: number }) {
  const opacity = useTransform(progresso, [de, ate], [0.16, 1]);
  const blur = useTransform(progresso, [de, ate], [4, 0]);
  const filter = useTransform(blur, (b) => `blur(${b}px)`);
  return (
    <motion.span aria-hidden="true" style={{ opacity, filter, display: "inline-block", willChange: "opacity, filter" }}>
      {texto}
    </motion.span>
  );
}

/** Uma ou mais frases (cada uma pode ter a sua classe, como o título em duas cores do site). */
export function ScrollReveal({ partes }: { partes: { texto: string; className?: string }[] }) {
  const ref = useRef<HTMLSpanElement>(null);
  const reduce = useReducedMotionSafe();
  const { scrollYProgress } = useScroll({ target: ref, offset: ["start 0.92", "start 0.42"] });
  const palavras = partes.map((p) => p.texto.split(" "));
  const n = palavras.reduce((s, a) => s + a.length, 0);
  // Onde cada parte começa na contagem geral de palavras.
  const inicio = palavras.map((_, i) => palavras.slice(0, i).reduce((s, a) => s + a.length, 0));

  return (
    <span ref={ref}>
      {/* O leitor de tela lê a frase inteira daqui; as palavras animadas abaixo ficam escondidas dele. */}
      <span className="sr-only">{partes.map((p) => p.texto).join(" ")}</span>
      {partes.map((p, i) => (
        <span key={i} className={p.className}>
          {palavras[i].map((t, j, arr) => {
            const idx = inicio[i] + j;
            // Cada palavra acende numa janela curta; as janelas se sobrepõem para a frase correr, não piscar.
            const de = (idx / n) * 0.75;
            const ate = de + 0.25;
            return (
              <span key={j}>
                {reduce ? <span aria-hidden="true">{t}</span> : <Palavra texto={t} progresso={scrollYProgress} de={de} ate={ate} />}
                {j < arr.length - 1 ? " " : ""}
              </span>
            );
          })}
        </span>
      ))}
    </span>
  );
}
