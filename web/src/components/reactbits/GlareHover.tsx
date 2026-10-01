/*
 * GlareHover — adaptado de React Bits (https://reactbits.dev/animations/glare-hover).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: uma camada que se pendura no bloco que já existe (o bloco recebe a classe `glare-alvo`); CSS
 * puro em globals.css, sem JavaScript; reflexo branco fraco, porque passa sobre o bloco preto; menos movimento = nada.
 */
export function GlareHover() {
  return <span aria-hidden="true" className="glare" />;
}
