"use client";

/*
 * ShinyText — adaptado de React Bits (https://reactbits.dev/text-animations/shiny-text).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: só CSS (o original animava quadro a quadro em JavaScript); para quem pede menos movimento o
 * brilho não passa e o texto fica na cor base. A animação mora em globals.css (`.shiny-text`).
 */
import type { CSSProperties } from "react";

export function ShinyText({
  text,
  className = "",
  color = "#0a0a0a",
  shineColor = "#a3a3a3",
  segundos = 3.2,
}: {
  text: string;
  className?: string;
  color?: string;
  shineColor?: string;
  /** Uma passada inteira do brilho, com a pausa. */
  segundos?: number;
}) {
  const estilo = {
    "--shiny-cor": color,
    "--shiny-brilho": shineColor,
    "--shiny-tempo": `${segundos}s`,
  } as CSSProperties;
  return (
    <span className={`shiny-text ${className}`} style={estilo}>
      {text}
    </span>
  );
}
