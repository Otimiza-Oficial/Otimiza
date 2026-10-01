"use client";

/*
 * Magnet — adaptado de React Bits (https://reactbits.dev/animations/magnet).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: mexe o transform direto no elemento (o original re-renderizava o React a cada movimento do
 * mouse); desligado em tela de toque e para quem pede menos movimento; puxão mais curto (o botão não pode fugir
 * do clique).
 */
import { useEffect, useRef, type ReactNode } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

export function Magnet({
  children,
  padding = 60,
  forca = 6,
  className = "",
  innerClassName = "",
}: {
  children: ReactNode;
  /** Distância em px, além da borda, em que o puxão começa. */
  padding?: number;
  /** Quanto maior, mais fraco o puxão (deslocamento = distância ÷ força). */
  forca?: number;
  className?: string;
  /** Classe do miolo que se move (ex.: esticar o botão no celular). */
  innerClassName?: string;
}) {
  const fora = useRef<HTMLDivElement>(null);
  const dentro = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotionSafe();

  useEffect(() => {
    const el = dentro.current;
    const caixa = fora.current;
    if (!el || !caixa || reduce || !window.matchMedia("(hover: hover) and (pointer: fine)").matches) return;
    let ativo = false;
    const mover = (e: PointerEvent) => {
      const r = caixa.getBoundingClientRect();
      const cx = r.left + r.width / 2;
      const cy = r.top + r.height / 2;
      const perto = Math.abs(cx - e.clientX) < r.width / 2 + padding && Math.abs(cy - e.clientY) < r.height / 2 + padding;
      if (perto) {
        el.style.transition = "transform 0.3s ease-out";
        el.style.transform = `translate3d(${(e.clientX - cx) / forca}px, ${(e.clientY - cy) / forca}px, 0)`;
        ativo = true;
      } else if (ativo) {
        el.style.transition = "transform 0.5s ease-in-out";
        el.style.transform = "translate3d(0, 0, 0)";
        ativo = false;
      }
    };
    window.addEventListener("pointermove", mover, { passive: true });
    return () => {
      window.removeEventListener("pointermove", mover);
      el.style.transform = "";
    };
  }, [padding, forca, reduce]);

  return (
    <div ref={fora} className={`relative ${className || "inline-block"}`}>
      <div ref={dentro} className={innerClassName} style={{ willChange: "transform" }}>
        {children}
      </div>
    </div>
  );
}
