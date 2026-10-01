"use client";

/*
 * Waves — adaptado de React Bits (https://reactbits.dev/backgrounds/waves).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: nitidez em tela de alta densidade, posição do mouse recalculada com a rolagem, desenho parado
 * quando o bloco sai da tela ou a aba fica escondida, um quadro estático para quem pede menos movimento, sem o ponto
 * que seguia o cursor e sem `touchmove` bloqueante.
 */
import { useEffect, useRef } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

class Grad {
  constructor(
    public x: number,
    public y: number,
    public z: number,
  ) {}
  dot2(x: number, y: number) {
    return this.x * x + this.y * y;
  }
}

class Noise {
  private grad3 = [
    new Grad(1, 1, 0), new Grad(-1, 1, 0), new Grad(1, -1, 0), new Grad(-1, -1, 0),
    new Grad(1, 0, 1), new Grad(-1, 0, 1), new Grad(1, 0, -1), new Grad(-1, 0, -1),
    new Grad(0, 1, 1), new Grad(0, -1, 1), new Grad(0, 1, -1), new Grad(0, -1, -1),
  ];
  private p = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69, 142, 8, 99, 37, 240,
    21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32, 57, 177, 33, 88,
    237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175, 74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83,
    111, 229, 122, 60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216,
    80, 73, 209, 76, 132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186,
    3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206, 59, 227, 47, 16, 58,
    17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9,
    129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193,
    238, 210, 144, 12, 191, 179, 162, 241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157,
    184, 84, 204, 176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141, 128,
    195, 78, 66, 215, 61, 156, 180,
  ];
  private perm = new Array<number>(512);
  private gradP = new Array<Grad>(512);

  constructor(seed = 0) {
    if (seed > 0 && seed < 1) seed *= 65536;
    seed = Math.floor(seed);
    if (seed < 256) seed |= seed << 8;
    for (let i = 0; i < 256; i++) {
      const v = i & 1 ? this.p[i] ^ (seed & 255) : this.p[i] ^ ((seed >> 8) & 255);
      this.perm[i] = this.perm[i + 256] = v;
      this.gradP[i] = this.gradP[i + 256] = this.grad3[v % 12];
    }
  }
  private fade(t: number) {
    return t * t * t * (t * (t * 6 - 15) + 10);
  }
  private lerp(a: number, b: number, t: number) {
    return (1 - t) * a + t * b;
  }
  perlin2(x: number, y: number) {
    let X = Math.floor(x);
    let Y = Math.floor(y);
    x -= X;
    y -= Y;
    X &= 255;
    Y &= 255;
    const n00 = this.gradP[X + this.perm[Y]].dot2(x, y);
    const n01 = this.gradP[X + this.perm[Y + 1]].dot2(x, y - 1);
    const n10 = this.gradP[X + 1 + this.perm[Y]].dot2(x - 1, y);
    const n11 = this.gradP[X + 1 + this.perm[Y + 1]].dot2(x - 1, y - 1);
    const u = this.fade(x);
    return this.lerp(this.lerp(n00, n10, u), this.lerp(n01, n11, u), this.fade(y));
  }
}

type Ponto = { x: number; y: number; wave: { x: number; y: number }; cursor: { x: number; y: number; vx: number; vy: number } };

export function Waves({
  lineColor = "rgb(10 10 10 / 0.14)",
  waveSpeedX = 0.0125,
  waveSpeedY = 0.005,
  waveAmpX = 32,
  waveAmpY = 16,
  xGap = 10,
  yGap = 32,
  friction = 0.925,
  tension = 0.005,
  maxCursorMove = 100,
  className = "",
}: {
  lineColor?: string;
  waveSpeedX?: number;
  waveSpeedY?: number;
  waveAmpX?: number;
  waveAmpY?: number;
  xGap?: number;
  yGap?: number;
  friction?: number;
  tension?: number;
  maxCursorMove?: number;
  className?: string;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const reduce = useReducedMotionSafe();

  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const noise = new Noise(Math.random());
    let linhas: Ponto[][] = [];
    let largura = 0;
    let altura = 0;
    const mouse = { x: -10, y: 0, lx: 0, ly: 0, sx: 0, sy: 0, v: 0, vs: 0, a: 0, set: false };
    let quadro: number | null = null;
    let visivel = true;

    function medir() {
      const r = container!.getBoundingClientRect();
      largura = r.width;
      altura = r.height;
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas!.width = Math.round(largura * dpr);
      canvas!.height = Math.round(altura * dpr);
      ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
      linhas = [];
      const totalLinhas = Math.ceil((largura + 200) / xGap);
      const totalPontos = Math.ceil((altura + 30) / yGap);
      const xIni = (largura - xGap * totalLinhas) / 2;
      const yIni = (altura - yGap * totalPontos) / 2;
      for (let i = 0; i <= totalLinhas; i++) {
        const pts: Ponto[] = [];
        for (let j = 0; j <= totalPontos; j++) {
          pts.push({ x: xIni + xGap * i, y: yIni + yGap * j, wave: { x: 0, y: 0 }, cursor: { x: 0, y: 0, vx: 0, vy: 0 } });
        }
        linhas.push(pts);
      }
    }

    function mover(t: number) {
      for (const pts of linhas) {
        for (const p of pts) {
          const m = noise.perlin2((p.x + t * waveSpeedX) * 0.002, (p.y + t * waveSpeedY) * 0.0015) * 12;
          p.wave.x = Math.cos(m) * waveAmpX;
          p.wave.y = Math.sin(m) * waveAmpY;
          const dist = Math.hypot(p.x - mouse.sx, p.y - mouse.sy);
          const l = Math.max(175, mouse.vs);
          if (dist < l) {
            const f = Math.cos(dist * 0.001) * (1 - dist / l);
            p.cursor.vx += Math.cos(mouse.a) * f * l * mouse.vs * 0.00065;
            p.cursor.vy += Math.sin(mouse.a) * f * l * mouse.vs * 0.00065;
          }
          p.cursor.vx = (p.cursor.vx - p.cursor.x * tension) * friction;
          p.cursor.vy = (p.cursor.vy - p.cursor.y * tension) * friction;
          p.cursor.x = Math.min(maxCursorMove, Math.max(-maxCursorMove, p.cursor.x + p.cursor.vx * 2));
          p.cursor.y = Math.min(maxCursorMove, Math.max(-maxCursorMove, p.cursor.y + p.cursor.vy * 2));
        }
      }
    }

    const pos = (p: Ponto, comCursor: boolean) => ({
      x: p.x + p.wave.x + (comCursor ? p.cursor.x : 0),
      y: p.y + p.wave.y + (comCursor ? p.cursor.y : 0),
    });

    function desenhar() {
      ctx!.clearRect(0, 0, largura, altura);
      ctx!.beginPath();
      ctx!.strokeStyle = lineColor;
      ctx!.lineWidth = 1;
      for (const pts of linhas) {
        let p = pos(pts[0], false);
        ctx!.moveTo(p.x, p.y);
        pts.forEach((pt, i) => {
          const ultimo = i === pts.length - 1;
          p = pos(pt, !ultimo);
          ctx!.lineTo(p.x, p.y);
        });
      }
      ctx!.stroke();
    }

    function tick(t: number) {
      mouse.sx += (mouse.x - mouse.sx) * 0.1;
      mouse.sy += (mouse.y - mouse.sy) * 0.1;
      const dx = mouse.x - mouse.lx;
      const dy = mouse.y - mouse.ly;
      const d = Math.hypot(dx, dy);
      mouse.v = d;
      mouse.vs = Math.min(100, mouse.vs + (d - mouse.vs) * 0.1);
      mouse.lx = mouse.x;
      mouse.ly = mouse.y;
      mouse.a = Math.atan2(dy, dx);
      mover(t);
      desenhar();
      quadro = requestAnimationFrame(tick);
    }

    const parar = () => {
      if (quadro !== null) cancelAnimationFrame(quadro);
      quadro = null;
    };
    const seguir = () => {
      if (reduce || quadro !== null || !visivel || document.hidden) return;
      quadro = requestAnimationFrame(tick);
    };

    function aoMover(e: PointerEvent) {
      // Recalcula a posição do bloco a cada movimento: com a página rolada, a posição guardada na montagem errava.
      const r = container!.getBoundingClientRect();
      mouse.x = e.clientX - r.left;
      mouse.y = e.clientY - r.top;
      if (!mouse.set) {
        mouse.sx = mouse.lx = mouse.x;
        mouse.sy = mouse.ly = mouse.y;
        mouse.set = true;
      }
    }

    medir();
    if (reduce) {
      mover(0);
      desenhar();
    } else {
      seguir();
    }

    const aoRedimensionar = () => {
      medir();
      if (reduce) {
        mover(0);
        desenhar();
      }
    };
    const olho = new IntersectionObserver(([e]) => {
      visivel = e.isIntersecting;
      if (visivel) seguir();
      else parar();
    });
    olho.observe(container);
    const aoTrocarAba = () => (document.hidden ? parar() : seguir());

    window.addEventListener("resize", aoRedimensionar);
    window.addEventListener("pointermove", aoMover, { passive: true });
    document.addEventListener("visibilitychange", aoTrocarAba);
    return () => {
      parar();
      olho.disconnect();
      window.removeEventListener("resize", aoRedimensionar);
      window.removeEventListener("pointermove", aoMover);
      document.removeEventListener("visibilitychange", aoTrocarAba);
    };
  }, [reduce, lineColor, waveSpeedX, waveSpeedY, waveAmpX, waveAmpY, xGap, yGap, friction, tension, maxCursorMove]);

  return (
    <div ref={containerRef} aria-hidden="true" className={`pointer-events-none absolute inset-0 overflow-hidden ${className}`}>
      <canvas ref={canvasRef} className="block h-full w-full" />
    </div>
  );
}
