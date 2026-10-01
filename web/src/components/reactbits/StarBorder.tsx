/*
 * StarBorder — adaptado de React Bits (https://reactbits.dev/animations/star-border).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: vira moldura de qualquer bloco (o original era botão); CSS puro em globals.css
 * (`.star-border`), sem JavaScript; menos movimento = moldura parada.
 */
import type { CSSProperties, ReactNode } from "react";

export function StarBorder({
  children,
  cor = "rgb(255 255 255 / 0.9)",
  segundos = 6,
  espessura = 1,
  raio = "var(--radius-card)",
  className = "",
}: {
  children: ReactNode;
  cor?: string;
  segundos?: number;
  espessura?: number;
  raio?: string;
  className?: string;
}) {
  const estilo = { "--star-cor": cor, "--star-tempo": `${segundos}s`, padding: espessura, borderRadius: raio } as CSSProperties;
  return (
    <div className={`star-border ${className}`} style={estilo}>
      <span aria-hidden="true" className="star-border-luz star-border-baixo" />
      <span aria-hidden="true" className="star-border-luz star-border-cima" />
      <div className="relative z-[1] flex h-full w-full" style={{ borderRadius: raio }}>
        {children}
      </div>
    </div>
  );
}
