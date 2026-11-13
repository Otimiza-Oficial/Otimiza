// A prévia da aba Cores: uma cena colorida processada pixel a pixel com o ajuste pedido, metade antes e metade depois.
// É só a prévia: o ajuste de verdade vai para o driver da placa (ou para a gama do Windows, sem driver compatível).

export interface Ajuste {
  saturacao: number; // 0 a 100, 50 é o normal
  brilho: number; // -40 a 40
  contraste: number; // -40 a 40
  gama: number; // 70 a 150, 100 é o normal
  matiz: number; // -30 a 30 graus
}

export const NORMAL: Ajuste = { saturacao: 50, brilho: 0, contraste: 0, gama: 100, matiz: 0 };

export const PREDEFINICOES: Record<string, Ajuste> = {
  natural: NORMAL,
  vivo: { saturacao: 70, brilho: 0, contraste: 5, gama: 100, matiz: 0 },
  competitivo: { saturacao: 80, brilho: 8, contraste: 10, gama: 120, matiz: 0 },
  cinema: { saturacao: 42, brilho: -6, contraste: 14, gama: 92, matiz: 4 },
};

function rng(s: number) {
  return () => (s = (s * 16807) % 2147483647) / 2147483647;
}

let base: ImageData | null = null;

function cenaBase(): ImageData {
  if (base) return base;
  const W = 640;
  const H = 360;
  const c = document.createElement("canvas");
  c.width = W;
  c.height = H;
  const x = c.getContext("2d")!;
  const r = rng(99);
  const ceu = x.createLinearGradient(0, 0, 0, H * 0.62);
  ceu.addColorStop(0, "#2b1d5c");
  ceu.addColorStop(0.45, "#b8456b");
  ceu.addColorStop(0.8, "#f08a4b");
  ceu.addColorStop(1, "#f7c46c");
  x.fillStyle = ceu;
  x.fillRect(0, 0, W, H);
  const sol = x.createRadialGradient(430, 205, 0, 430, 205, 140);
  sol.addColorStop(0, "rgba(255,235,170,1)");
  sol.addColorStop(0.25, "rgba(255,200,120,.8)");
  sol.addColorStop(1, "rgba(255,150,90,0)");
  x.fillStyle = sol;
  x.fillRect(0, 0, W, H);
  let px = 0;
  while (px < W) {
    const w = 18 + r() * 34;
    const h = 40 + r() * 110;
    x.fillStyle = "#2a1838";
    x.fillRect(px, 225 - h, w, h);
    x.fillStyle = "rgba(255,210,120,.8)";
    for (let wy = 230 - h; wy < 220; wy += 8) for (let wx = px + 3; wx < px + w - 3; wx += 6) if (r() < 0.2) x.fillRect(wx, wy, 2, 3);
    px += w + 1;
  }
  const mar = x.createLinearGradient(0, 225, 0, H);
  mar.addColorStop(0, "#1f6f8b");
  mar.addColorStop(1, "#0b2a3d");
  x.fillStyle = mar;
  x.fillRect(0, 225, W, H);
  x.fillStyle = "#2f7a3a";
  x.beginPath();
  x.moveTo(0, 300);
  x.quadraticCurveTo(120, 260, 230, 300);
  x.lineTo(230, H);
  x.lineTo(0, H);
  x.fill();
  x.fillStyle = "#d13b3b";
  x.fillRect(300, 290, 90, 26);
  x.fillStyle = "#f3d23a";
  x.fillRect(306, 296, 14, 6);
  base = x.getImageData(0, 0, W, H);
  return base;
}

/** Desenha a prévia no canvas: à esquerda de `corte` (0 a 1) o original, à direita o ajustado. */
export function previa(cv: HTMLCanvasElement, a: Ajuste, corte: number) {
  const d = cenaBase();
  cv.width = d.width;
  cv.height = d.height;
  const x = cv.getContext("2d")!;
  const saida = x.createImageData(d.width, d.height);
  const o = saida.data;
  const e = d.data;
  const s = a.saturacao / 50;
  const b = a.brilho / 100;
  const k = 1 + a.contraste / 100;
  const g = 1 / (a.gama / 100);
  const h = (a.matiz * Math.PI) / 180;
  const ch = Math.cos(h);
  const sh = Math.sin(h);
  // Matriz de rotação de matiz em luminância (a mesma do filtro hue-rotate do CSS).
  const m = [
    0.213 + ch * 0.787 - sh * 0.213, 0.715 - ch * 0.715 - sh * 0.715, 0.072 - ch * 0.072 + sh * 0.928,
    0.213 - ch * 0.213 + sh * 0.143, 0.715 + ch * 0.285 + sh * 0.14, 0.072 - ch * 0.072 - sh * 0.283,
    0.213 - ch * 0.213 - sh * 0.787, 0.715 - ch * 0.715 + sh * 0.715, 0.072 + ch * 0.928 + sh * 0.072,
  ];
  const limite = Math.round(corte * d.width);
  for (let i = 0; i < e.length; i += 4) {
    if ((i / 4) % d.width < limite) {
      o[i] = e[i];
      o[i + 1] = e[i + 1];
      o[i + 2] = e[i + 2];
      o[i + 3] = 255;
      continue;
    }
    const r0 = e[i] / 255;
    const g0 = e[i + 1] / 255;
    const b0 = e[i + 2] / 255;
    const r1 = m[0] * r0 + m[1] * g0 + m[2] * b0;
    const g1 = m[3] * r0 + m[4] * g0 + m[5] * b0;
    const b1 = m[6] * r0 + m[7] * g0 + m[8] * b0;
    const l = 0.2126 * r1 + 0.7152 * g1 + 0.0722 * b1;
    const f = (v: number) => {
      v = l + (v - l) * s;
      v = (v - 0.5) * k + 0.5 + b;
      return Math.pow(Math.min(1, Math.max(0, v)), g) * 255;
    };
    o[i] = f(r1);
    o[i + 1] = f(g1);
    o[i + 2] = f(b1);
    o[i + 3] = 255;
  }
  x.putImageData(saida, 0, 0);
}
