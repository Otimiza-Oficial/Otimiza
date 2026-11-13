// A página do FPS por cima do jogo, na interface nova. O backend chama `window.fps(leitura)` duas vezes por segundo
// com números já medidos (`plataforma::overlay::leitura_de`): aqui só se desenha. Sem número, um traço, nunca um palpite.
//
// O Oti pequeno: no Detalhado fica sempre ao lado do número; no Mínimo (o contador no estilo do jogo) só aparece no
// alerta. Âmbar e pulsando quando o 1% pior cai em relação ao que a própria partida vinha fazendo (`lib/queda.ts`).
// Desenho em SVG puro, sem Svelte: a janela fica por cima do jogo e precisa custar quase nada.

import { nova, passo } from "./lib/queda";

interface Leitura {
  modo: "Desligado" | "Minimo" | "Detalhado";
  rotulo: string;
  fps: number | null;
  low_1pct: number | null;
  quadro_ms: number | null;
  boost: number | null;
  nota: string | null;
  escala?: string | null;
  na_tela?: number | null;
}

const vigia = nova();
const inicio = performance.now();

const n = (v: number | null | undefined, casas = 0) =>
  v == null ? "—" : v.toLocaleString("pt-BR", { maximumFractionDigits: casas, minimumFractionDigits: casas });

/** O Oti em 200×200: disco branco e os dois olhos inclinados (o mesmo desenho do `Oti.svelte`, sem os enfeites). */
function oti(alerta: boolean) {
  const olho = (cx: number, cy: number) =>
    `<g transform="rotate(-33 ${cx} ${cy})"><rect x="${cx - 15}" y="${cy - 28}" width="30" height="56" rx="15" fill="#0b0b0c"/></g>`;
  return `<svg class="oti${alerta ? " alerta" : ""}" viewBox="0 0 200 200" aria-hidden="true">
    <circle class="corpo" cx="100" cy="100" r="92" fill="#f2f2f3"/>
    <circle cx="100" cy="100" r="91" fill="none" stroke="#000" stroke-opacity="0.18" stroke-width="3"/>
    ${olho(86, 112)}${olho(130, 88)}
  </svg>`;
}

function desenhar(l: Leitura) {
  const c = document.getElementById("cartao");
  if (!c) return;
  const alerta = passo(vigia, performance.now() - inicio, l.low_1pct);
  const detalhado = l.modo === "Detalhado";
  c.className = detalhado ? "cartao" : "cartao compacto";

  if (!detalhado) {
    // "66fps", igual ao cl_drawfps; o Oti só se o 1% pior caiu.
    c.innerHTML = `<div class="linha"><span class="fps">${n(l.fps)}</span><span class="un">fps</span>${oti(alerta)}</div>`;
    return;
  }
  const topo = `<div class="linha">${oti(alerta)}<span class="fps">${n(l.fps)}</span><span class="un">${
    l.rotulo === "FPS" ? "FPS" : "FPS NA TELA"
  }</span>${l.boost ? `<span class="boost">${l.boost}×</span>` : ""}</div>`;
  if (l.nota && l.fps === null) {
    c.innerHTML = topo + `<div class="nota">${l.nota}</div>`;
    return;
  }
  const extra =
    l.boost || l.escala
      ? `${l.na_tela ? `<div class="mini">tela <b>${n(l.na_tela)}</b></div>` : ""}<div class="mini">${
          l.escala ? `escala <b>${l.escala}</b>` : "Frame Boost do Otimiza"
        }</div>`
      : `<div class="mini">1% pior <b>${n(l.low_1pct)}</b>${alerta ? " · caiu" : ""}</div><div class="mini">quadro <b>${n(
          l.quadro_ms,
          1,
        )} ms</b></div>`;
  c.innerHTML = topo + extra;
}

(window as unknown as { fps: (l: Leitura) => void }).fps = desenhar;
