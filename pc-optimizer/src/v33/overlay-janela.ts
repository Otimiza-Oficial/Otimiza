// A página do FPS por cima do jogo. O backend chama `window.fps(leitura)` duas vezes por segundo com números já
// medidos (`plataforma::overlay::leitura_de`): aqui só se desenha. Sem número, um traço, nunca um palpite.

interface Leitura {
  modo: "Desligado" | "Minimo" | "Detalhado";
  rotulo: string;
  fps: number | null;
  low_1pct: number | null;
  quadro_ms: number | null;
  boost: number | null;
  nota: string | null;
  /** Nível da escala do Frame Boost ("Qualidade"). */
  escala?: string | null;
  /** Gerando quadros: o que chegou à tela. */
  na_tela?: number | null;
}

const n = (v: number | null, casas = 0) => (v === null ? "—" : v.toLocaleString("pt-BR", { maximumFractionDigits: casas, minimumFractionDigits: casas }));

function desenhar(l: Leitura) {
  const c = document.getElementById("cartao");
  if (!c) return;
  c.className = l.modo === "Detalhado" ? "cartao" : "cartao compacto";
  // Mínimo: "66fps", igual ao cl_drawfps. Detalhado: o cartão com o rótulo e o multiplicador.
  const topo =
    l.modo !== "Detalhado"
      ? `<div class="linha"><span class="fps">${n(l.fps)}</span><span class="un">fps</span></div>`
      : `<div class="linha"><span class="fps">${n(l.fps)}</span><span class="un">${l.rotulo === "FPS" ? "FPS" : "FPS NA TELA"}</span>${l.boost ? `<span class="boost">${l.boost}×</span>` : ""}</div>`;
  if (l.nota && l.fps === null) {
    c.innerHTML = topo + (l.modo === "Detalhado" ? `<div class="nota">${l.nota}</div>` : "");
    return;
  }
  if (l.modo !== "Detalhado") {
    c.innerHTML = topo;
    return;
  }
  // Com o Frame Boost: o que chegou à tela e a escala, cada um só quando existe.
  const extra =
    l.boost || l.escala
      ? `${l.na_tela ? `<div class="mini">tela <b>${n(l.na_tela)}</b></div>` : ""}<div class="mini">${l.escala ? `escala <b>${l.escala}</b>` : "Frame Boost do Otimiza"}</div>`
      : `<div class="mini">1% low <b>${n(l.low_1pct)}</b></div><div class="mini">quadro <b>${n(l.quadro_ms, 1)} ms</b></div>`;
  c.innerHTML = topo + extra;
}

(window as unknown as { fps: (l: Leitura) => void }).fps = desenhar;
