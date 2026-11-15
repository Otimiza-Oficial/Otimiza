// A aba Mira: o desenho da mira (o mesmo SVG vai para a prévia e para a janela da mira no jogo), o conversor de
// sensibilidade e o treino de tracking. Nada daqui mira por ninguém: só mostra a mira e mede a pessoa.

export type Estilo = "ponto" | "cruz" | "cruz-ponto" | "circulo" | "t";

export interface Mira {
  estilo: Estilo;
  cor: string;
  tamanho: number;
  espessura: number;
  abertura: number;
  opacidade: number;
  contorno: boolean;
  /** Some enquanto o botão direito está apertado: a mira do jogo aparece ao mirar. */
  esconder_ao_mirar?: boolean;
}

export const ESTILOS: [Estilo, string][] = [
  ["ponto", "Ponto"],
  ["cruz", "Cruz"],
  ["cruz-ponto", "Cruz com ponto"],
  ["circulo", "Círculo"],
  ["t", "T"],
];

export const CORES = ["#ffffff", "#00e5ff", "#7cff4f", "#ffe14a", "#ff4fd8", "#ff4a4a"];

export const PADRAO: Mira = { estilo: "cruz-ponto", cor: "#ffffff", tamanho: 10, espessura: 2, abertura: 4, opacidade: 1, contorno: true };

const CHAVE = "otimiza-mira";

export function lerMira(): Mira {
  try {
    const bruto = localStorage.getItem(CHAVE);
    if (bruto) return { ...PADRAO, ...(JSON.parse(bruto) as Partial<Mira>) };
  } catch {
    // Sem armazenamento: vale o padrão.
  }
  return { ...PADRAO };
}

export function gravarMira(m: Mira) {
  try {
    localStorage.setItem(CHAVE, JSON.stringify(m));
  } catch {
    // A mira continua valendo nesta sessão.
  }
}

/** O SVG da mira, centrado em (0,0). `escala` aumenta tudo para a prévia. */
export function desenharMira(m: Mira, escala = 1): string {
  const s = m.tamanho * escala;
  const e = m.espessura * escala;
  const g = m.abertura * escala;
  const borda = `stroke="#000" stroke-width="${e + 2 * escala}"`;
  const linha = (x1: number, y1: number, x2: number, y2: number) =>
    (m.contorno ? `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" ${borda}/>` : "") +
    `<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke="${m.cor}" stroke-width="${e}"/>`;
  const ponto = () =>
    (m.contorno ? `<rect x="${-e / 2 - escala}" y="${-e / 2 - escala}" width="${e + 2 * escala}" height="${e + 2 * escala}" fill="#000"/>` : "") +
    `<rect x="${-e / 2}" y="${-e / 2}" width="${e}" height="${e}" fill="${m.cor}"/>`;

  let o = "";
  if (m.estilo === "ponto") o = ponto();
  if (m.estilo === "cruz" || m.estilo === "cruz-ponto" || m.estilo === "t") {
    if (m.estilo !== "t") o += linha(0, -g, 0, -g - s);
    o += linha(0, g, 0, g + s) + linha(-g, 0, -g - s, 0) + linha(g, 0, g + s, 0);
    if (m.estilo === "cruz-ponto") o += ponto();
  }
  if (m.estilo === "circulo") {
    o =
      (m.contorno ? `<circle r="${g + s / 2}" fill="none" stroke="#000" stroke-width="${e + 2 * escala}"/>` : "") +
      `<circle r="${g + s / 2}" fill="none" stroke="${m.cor}" stroke-width="${e}"/>` +
      ponto();
  }
  return `<g opacity="${m.opacidade}">${o}</g>`;
}

/** CS2, Apex e Valorant giram 0,022° por contagem do mouse vezes a sensibilidade; o Valorant usa 1/3,18 da do CS2. */
export function cmPorVolta(dpi: number, sensCs2: number): number | null {
  if (!(dpi > 0) || !(sensCs2 > 0)) return null;
  return (360 / (dpi * sensCs2 * 0.022)) * 2.54;
}

export function sensValorant(sensCs2: number): number | null {
  return sensCs2 > 0 ? sensCs2 / 3.18 : null;
}

/** Treino de tracking: um alvo que se move, e quanto tempo o cursor ficou nele. */
export function treinar(cv: HTMLCanvasElement, segundos: number, fim: (pct: number) => void) {
  const r = cv.getBoundingClientRect();
  cv.width = r.width * 2;
  cv.height = r.height * 2;
  const x = cv.getContext("2d")!;
  let mx = -99;
  let my = -99;
  let no = 0;
  let total = 0;
  const t0 = performance.now();
  const mover = (e: PointerEvent) => {
    const b = cv.getBoundingClientRect();
    mx = (e.clientX - b.left) * 2;
    my = (e.clientY - b.top) * 2;
  };
  cv.addEventListener("pointermove", mover);

  const passo = (t: number) => {
    const s = (t - t0) / 1000;
    const W = cv.width;
    const H = cv.height;
    const ax = W / 2 + Math.sin(s * 1.3) * W * 0.36 + Math.sin(s * 3.1) * W * 0.06;
    const ay = H / 2 + Math.sin(s * 1.9 + 1) * H * 0.3;
    const raio = 22;
    const dentro = Math.hypot(mx - ax, my - ay) < raio * 1.4;
    total++;
    if (dentro) no++;
    x.fillStyle = "rgba(5,5,5,.35)";
    x.fillRect(0, 0, W, H);
    x.beginPath();
    x.arc(ax, ay, raio, 0, 7);
    x.fillStyle = dentro ? "#fff" : "rgba(255,255,255,.25)";
    x.shadowColor = "#fff";
    x.shadowBlur = dentro ? 24 : 0;
    x.fill();
    x.shadowBlur = 0;
    x.fillStyle = "rgba(255,255,255,.7)";
    x.font = "500 22px GeistMono, monospace";
    x.fillText(`${Math.max(0, segundos - s).toFixed(1)} s · ${Math.round((no / total) * 100)}%`, 18, 34);
    if (s < segundos) requestAnimationFrame(passo);
    else {
      cv.removeEventListener("pointermove", mover);
      fim(Math.round((no / total) * 100));
    }
  };
  requestAnimationFrame(passo);
}

export function sensCs2DeValorant(sensValorant: number): number | null {
  return sensValorant > 0 ? sensValorant * 3.18 : null;
}

const limitar = (v: number, min: number, max: number) => Math.min(max, Math.max(min, Math.round(v)));
const fracao = (v: number) => String(Math.round(Math.min(1, Math.max(0, v)) * 1000) / 1000);

/**
 * O código de perfil de mira do Valorant (Configurações > Mira > Importar perfil). Lá a mira do Otimiza por cima do jogo
 * é barrada pelo Vanguard: o caminho é a mira do próprio jogo. Cor personalizada (`c;8;u;RRGGBBAA`), linhas internas
 * para a cruz, ponto central, linhas externas e erro de tiro/movimento desligados. Círculo e T não existem no Valorant:
 * saem como a cruz mais parecida, e a `nota` diz isso.
 */
export function codigoValorant(m: Mira): { codigo: string; nota: string | null } {
  const cor = /^#[0-9a-f]{6}$/i.test(m.cor) ? m.cor.slice(1).toUpperCase() : "FFFFFF";
  const ponto = m.estilo === "ponto" || m.estilo === "cruz-ponto" || m.estilo === "circulo";
  const linhas = m.estilo !== "ponto";
  const partes = ["0", "P", "c", "8", "u", `${cor}FF`, "h", m.contorno ? "1" : "0"];
  if (m.contorno) partes.push("t", "1", "o", "1");
  partes.push("d", ponto ? "1" : "0");
  if (ponto) partes.push("z", String(limitar(m.espessura, 1, 6)), "a", fracao(m.opacidade));
  partes.push("0b", linhas ? "1" : "0");
  if (linhas) {
    partes.push(
      "0t", String(limitar(m.espessura, 0, 10)),
      "0l", String(limitar(m.tamanho, 0, 20)),
      "0o", String(limitar(m.abertura, 0, 20)),
      "0a", fracao(m.opacidade),
      "0m", "0", "0f", "0",
    );
  }
  partes.push("1b", "0");
  const nota =
    m.estilo === "circulo" || m.estilo === "t"
      ? `O Valorant não tem mira em ${m.estilo === "t" ? "T" : "círculo"}: o código traz a cruz mais parecida.`
      : m.tamanho > 20
        ? "O Valorant vai até 20 de tamanho: o código usa 20."
        : null;
  return { codigo: partes.join(";"), nota };
}

export interface ResultadoDoFlick {
  acertos: number;
  erros: number;
  /** Acertos sobre cliques, em %. */
  precisao: number;
  /** Do alvo aparecer até o clique certo, mediana em ms. */
  reacao_ms: number | null;
}

/** Pura: o resumo do treino de flick. */
export function resumirFlick(acertos: number, erros: number, tempos: number[]): ResultadoDoFlick {
  const ordenados = [...tempos].sort((a, b) => a - b);
  const meio = ordenados.length >> 1;
  const mediana =
    ordenados.length === 0 ? null : ordenados.length % 2 ? ordenados[meio] : (ordenados[meio - 1] + ordenados[meio]) / 2;
  const cliques = acertos + erros;
  return {
    acertos,
    erros,
    precisao: cliques ? Math.round((acertos / cliques) * 100) : 0,
    reacao_ms: mediana == null ? null : Math.round(mediana),
  };
}

/** Treino de flick: um alvo por vez em lugar sorteado; clicar nele traz o próximo. Conta acerto, erro e reação. */
export function treinarFlick(cv: HTMLCanvasElement, segundos: number, fim: (r: ResultadoDoFlick) => void) {
  const r = cv.getBoundingClientRect();
  cv.width = r.width * 2;
  cv.height = r.height * 2;
  const x = cv.getContext("2d")!;
  const raio = 18;
  let alvo = { x: 0, y: 0, desde: 0 };
  let acertos = 0;
  let erros = 0;
  const tempos: number[] = [];
  const sortear = () => {
    const m = raio * 2;
    alvo = { x: m + Math.random() * (cv.width - 2 * m), y: m + Math.random() * (cv.height - 2 * m), desde: performance.now() };
  };
  const clicar = (e: PointerEvent) => {
    const b = cv.getBoundingClientRect();
    const px = (e.clientX - b.left) * 2;
    const py = (e.clientY - b.top) * 2;
    if (Math.hypot(px - alvo.x, py - alvo.y) <= raio * 1.15) {
      acertos++;
      tempos.push(performance.now() - alvo.desde);
      sortear();
    } else {
      erros++;
    }
  };
  cv.addEventListener("pointerdown", clicar);
  sortear();
  const t0 = performance.now();

  const passo = (t: number) => {
    const s = (t - t0) / 1000;
    const W = cv.width;
    const H = cv.height;
    x.fillStyle = "rgba(5,5,5,.9)";
    x.fillRect(0, 0, W, H);
    x.beginPath();
    x.arc(alvo.x, alvo.y, raio, 0, 7);
    x.fillStyle = "#fff";
    x.shadowColor = "#fff";
    x.shadowBlur = 18;
    x.fill();
    x.shadowBlur = 0;
    x.fillStyle = "rgba(255,255,255,.7)";
    x.font = "500 22px GeistMono, monospace";
    x.fillText(`${Math.max(0, segundos - s).toFixed(1)} s · ${acertos} acertos · ${erros} erros`, 18, 34);
    if (s < segundos) requestAnimationFrame(passo);
    else {
      cv.removeEventListener("pointerdown", clicar);
      fim(resumirFlick(acertos, erros, tempos));
    }
  };
  requestAnimationFrame(passo);
}
