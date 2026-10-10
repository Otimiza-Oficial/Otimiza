// A capa de cada jogo: a arte da Steam deste PC quando existe (`capa_do_jogo`), senão uma desenhada aqui, em preto e
// branco, no estilo do jogo. A desenhada é guardada em memória: desenhar custa uns milissegundos por capa.

import { ponte } from "./ponte";

function rng(semente: number) {
  let s = semente % 2147483647 || 1;
  return () => (s = (s * 16807) % 2147483647) / 2147483647;
}

function semente(texto: string) {
  return texto.split("").reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 2147483647, 7);
}

export function desenhar(id: string, titulo: string): HTMLCanvasElement {
  const W = 600;
  const H = 800;
  const c = document.createElement("canvas");
  c.width = W;
  c.height = H;
  const x = c.getContext("2d")!;
  const r = rng(semente(id));

  const ceu = x.createLinearGradient(0, 0, 0, H);
  ceu.addColorStop(0, "#1d1d1d");
  ceu.addColorStop(0.6, "#0a0a0a");
  ceu.addColorStop(1, "#030303");
  x.fillStyle = ceu;
  x.fillRect(0, 0, W, H);

  const brilho = (cx: number, cy: number, raio: number, a: number) => {
    const g = x.createRadialGradient(cx, cy, 0, cx, cy, raio);
    g.addColorStop(0, `rgba(255,255,255,${a})`);
    g.addColorStop(1, "rgba(255,255,255,0)");
    x.fillStyle = g;
    x.fillRect(0, 0, W, H);
  };
  const predios = (base: number, alt: number, larg: number, tom: string, janelas: number) => {
    let px = -20;
    while (px < W) {
      const w = larg * (0.6 + r() * 0.8);
      const h = alt * (0.4 + r());
      x.fillStyle = tom;
      x.fillRect(px, base - h, w, h + 400);
      if (janelas) {
        x.fillStyle = "rgba(255,255,255,.55)";
        for (let wy = base - h + 10; wy < base - 8; wy += 11) for (let wx = px + 5; wx < px + w - 5; wx += 9) if (r() < janelas) x.fillRect(wx, wy, 3, 4);
      }
      px += w + 2;
    }
  };

  if (id === "fivem" || id === "gta5" || id === "gta5-enhanced" || id === "redm") {
    brilho(420, 210, 260, 0.22);
    x.fillStyle = "#e6e6e6";
    x.beginPath();
    x.arc(420, 210, 74, 0, 7);
    x.fill();
    predios(560, 260, 60, "#151515", 0.12);
    predios(610, 180, 46, "#0d0d0d", 0.25);
    x.strokeStyle = "#070707";
    for (const [tx, th] of [[90, 330], [150, 280], [510, 300]]) {
      x.lineWidth = 9;
      x.beginPath();
      x.moveTo(tx, 700);
      x.quadraticCurveTo(tx + 24, 700 - th / 2, tx + 10, 700 - th);
      x.stroke();
      for (let k = 0; k < 7; k++) {
        const a = (k / 7) * Math.PI * 2;
        x.lineWidth = 5;
        x.beginPath();
        x.moveTo(tx + 10, 700 - th);
        x.quadraticCurveTo(tx + 10 + Math.cos(a) * 50, 700 - th - 30 + Math.sin(a) * 10, tx + 10 + Math.cos(a) * 90, 700 - th + 30 + Math.abs(Math.sin(a)) * 30);
        x.stroke();
      }
    }
    x.fillStyle = "#050505";
    x.fillRect(0, 690, W, 200);
  } else if (id === "cs2") {
    for (let by = 0; by < H; by += 40)
      for (let bx = (by / 40) % 2 ? -40 : 0; bx < W; bx += 80) {
        const t = (24 + r() * 14) | 0;
        x.fillStyle = `rgb(${t},${t},${t})`;
        x.fillRect(bx + 1, by + 1, 78, 38);
      }
    brilho(300, 420, 260, 0.35);
    x.fillStyle = "#050505";
    x.beginPath();
    x.moveTo(150, 800);
    x.lineTo(150, 420);
    x.arc(300, 420, 150, Math.PI, 0);
    x.lineTo(450, 800);
    x.fill();
  } else if (id === "cyberpunk") {
    brilho(300, 120, 300, 0.16);
    for (let k = 0; k < 5; k++) predios(800, 520 - k * 60, 70 - k * 8, `rgb(${10 + k * 5},${10 + k * 5},${10 + k * 5})`, 0.18 + k * 0.05);
    x.strokeStyle = "rgba(255,255,255,.12)";
    x.lineWidth = 1;
    for (let k = 0; k < 220; k++) {
      const rx = r() * W;
      const ry = r() * H;
      x.beginPath();
      x.moveTo(rx, ry);
      x.lineTo(rx - 8, ry + 30);
      x.stroke();
    }
  } else {
    // Genérica: estilhaços de luz com o nome do jogo.
    brilho(380, 300, 340, 0.22);
    for (let k = 0; k < 9; k++) {
      x.fillStyle = `rgba(255,255,255,${0.03 + r() * 0.1})`;
      x.beginPath();
      const cx = r() * W;
      const cy = r() * H * 0.8;
      x.moveTo(cx, cy);
      x.lineTo(cx + 260 * (r() - 0.3), cy + 340 * r());
      x.lineTo(cx - 200 * r(), cy + 200 * r());
      x.fill();
    }
  }

  const v = x.createRadialGradient(W / 2, H / 2, H * 0.3, W / 2, H / 2, H * 0.75);
  v.addColorStop(0, "rgba(0,0,0,0)");
  v.addColorStop(1, "rgba(0,0,0,.75)");
  x.fillStyle = v;
  x.fillRect(0, 0, W, H);
  const fundo = x.createLinearGradient(0, H * 0.62, 0, H);
  fundo.addColorStop(0, "rgba(0,0,0,0)");
  fundo.addColorStop(1, "rgba(0,0,0,.85)");
  x.fillStyle = fundo;
  x.fillRect(0, H * 0.6, W, H * 0.4);
  x.fillStyle = "#fff";
  x.font = "300 54px Geist, 'Segoe UI', sans-serif";
  const titulo_ = titulo.toUpperCase().slice(0, 18);
  x.fillText(titulo_, 40, H - 70);
  x.font = "400 18px GeistMono, monospace";
  x.fillStyle = "rgba(255,255,255,.55)";
  x.fillText("OTIMIZA · PERFIL DO JOGO", 42, H - 36);
  return c;
}

const cache = new Map<string, Promise<string>>();

/** A capa como URL para `<img>`: a da Steam se houver, senão a desenhada. */
export function capa(id: string, nome: string, appid: number | null): Promise<string> {
  const chave = `${id}:${appid ?? ""}`;
  let p = cache.get(chave);
  if (!p) {
    p = (async () => {
      if (appid) {
        try {
          const steam = await ponte.capa(appid);
          if (steam) return steam;
        } catch {
          // Sem Steam: cai na desenhada.
        }
      }
      return desenhar(id, nome).toDataURL("image/jpeg", 0.85);
    })();
    cache.set(chave, p);
  }
  return p;
}

export function carregarImagem(url: string): Promise<HTMLImageElement> {
  return new Promise((ok, falha) => {
    const img = new Image();
    img.onload = () => ok(img);
    img.onerror = falha;
    img.src = url;
  });
}
