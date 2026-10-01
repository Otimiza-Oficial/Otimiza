"use client";

/*
 * Spotlight — adaptado do SpotlightCard de React Bits (https://reactbits.dev/components/spotlight-card).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: vira uma camada que se pendura no card que já existe (o card continua sendo o do site, não um
 * card escuro novo); a posição vai por variável CSS, sem re-render; a luz é tinta preta bem fraca, porque o site é
 * claro. A aparência mora em globals.css (`.spotlight`).
 */
import { useEffect, useRef, type CSSProperties } from "react";

export function Spotlight({ cor = "rgb(10 10 10 / 0.06)", raio = 320 }: { cor?: string; raio?: number }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const camada = ref.current;
    const card = camada?.parentElement;
    if (!camada || !card || !window.matchMedia("(hover: hover) and (pointer: fine)").matches) return;
    const mover = (e: PointerEvent) => {
      const r = card.getBoundingClientRect();
      camada.style.setProperty("--spot-x", `${e.clientX - r.left}px`);
      camada.style.setProperty("--spot-y", `${e.clientY - r.top}px`);
    };
    card.addEventListener("pointermove", mover, { passive: true });
    return () => card.removeEventListener("pointermove", mover);
  }, []);

  return (
    <div
      ref={ref}
      aria-hidden="true"
      className="spotlight"
      style={{ "--spot-cor": cor, "--spot-raio": `${raio}px` } as CSSProperties}
    />
  );
}
