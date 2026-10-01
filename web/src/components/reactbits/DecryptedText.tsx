"use client";

/*
 * DecryptedText — adaptado de React Bits (https://reactbits.dev/text-animations/decrypted-text).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: versão enxuta, só "ao aparecer na tela" e revelando da esquerda para a direita; o HTML do
 * servidor já tem o texto certo (quem não roda JavaScript lê normal); a largura não pula (fonte mono + mesmo número
 * de caracteres); menos movimento = texto pronto.
 */
import { useEffect, useRef, useState } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

export function DecryptedText({
  text,
  className = "",
  caracteres = "0123456789ABCDEFGHJKLMNPQRSTUVWXYZ",
  passoMs = 45,
  voltasPorLetra = 4,
}: {
  text: string;
  className?: string;
  caracteres?: string;
  passoMs?: number;
  /** Quantas trocas cada letra faz antes de assentar. */
  voltasPorLetra?: number;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const [mostrado, setMostrado] = useState(text);
  const reduce = useReducedMotionSafe();

  useEffect(() => {
    const el = ref.current;
    if (!el || reduce) return;
    let timer: ReturnType<typeof setInterval> | null = null;
    const sorteio = () => caracteres[Math.floor(Math.random() * caracteres.length)];
    const olho = new IntersectionObserver(
      ([e]) => {
        if (!e.isIntersecting) return;
        olho.disconnect();
        let passo = 0;
        const total = text.length * voltasPorLetra;
        timer = setInterval(() => {
          passo++;
          const fixas = Math.floor(passo / voltasPorLetra);
          setMostrado(
            text
              .split("")
              .map((c, i) => (c === " " || c === "-" || i < fixas ? c : sorteio()))
              .join(""),
          );
          if (passo >= total) {
            if (timer) clearInterval(timer);
            setMostrado(text);
          }
        }, passoMs);
      },
      { threshold: 0.6 },
    );
    olho.observe(el);
    return () => {
      olho.disconnect();
      if (timer) clearInterval(timer);
    };
  }, [text, caracteres, passoMs, voltasPorLetra, reduce]);

  return (
    <span ref={ref} className={className}>
      <span className="sr-only">{text}</span>
      <span aria-hidden="true">{mostrado}</span>
    </span>
  );
}
