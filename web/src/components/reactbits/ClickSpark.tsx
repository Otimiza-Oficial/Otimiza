"use client";

/*
 * ClickSpark — adaptado de React Bits (https://reactbits.dev/animations/click-spark).
 * Copyright (c) 2026 David Haz. MIT + Commons Clause (uso permitido como parte do site; os componentes não são
 * revendidos). Texto da licença: https://github.com/DavidHDev/react-bits/blob/main/LICENSE.md
 *
 * Mudanças do Otimiza: uma tela só, fixa sobre a página, que escuta os cliques do documento (não embrulha o site);
 * faíscas brancas com `mix-blend-mode: difference`, que ficam pretas no fundo claro e brancas nos blocos pretos;
 * o laço de desenho só roda enquanto há faísca (o original desenhava sempre); menos movimento = nada.
 */
import { useEffect, useRef } from "react";
import { useReducedMotionSafe } from "@/components/motion/useReducedMotionSafe";

type Faisca = { x: number; y: number; angulo: number; inicio: number };

export function ClickSpark({ tamanho = 9, raio = 18, quantidade = 8, duracao = 420 }: { tamanho?: number; raio?: number; quantidade?: number; duracao?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const reduce = useReducedMotionSafe();

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas || reduce) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    let faiscas: Faisca[] = [];
    let quadro: number | null = null;

    const medir = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.round(window.innerWidth * dpr);
      canvas.height = Math.round(window.innerHeight * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const desenhar = (agora: number) => {
      ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
      faiscas = faiscas.filter((f) => {
        const t = (agora - f.inicio) / duracao;
        if (t >= 1) return false;
        const e = t * (2 - t);
        const d = e * raio;
        const l = tamanho * (1 - e);
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 1.6;
        ctx.beginPath();
        ctx.moveTo(f.x + d * Math.cos(f.angulo), f.y + d * Math.sin(f.angulo));
        ctx.lineTo(f.x + (d + l) * Math.cos(f.angulo), f.y + (d + l) * Math.sin(f.angulo));
        ctx.stroke();
        return true;
      });
      quadro = faiscas.length ? requestAnimationFrame(desenhar) : null;
    };

    const aoClicar = (e: PointerEvent) => {
      if (e.pointerType !== "mouse") return;
      const agora = performance.now();
      for (let i = 0; i < quantidade; i++) faiscas.push({ x: e.clientX, y: e.clientY, angulo: (2 * Math.PI * i) / quantidade, inicio: agora });
      if (quadro === null) quadro = requestAnimationFrame(desenhar);
    };

    medir();
    window.addEventListener("resize", medir);
    document.addEventListener("pointerdown", aoClicar, { passive: true });
    return () => {
      window.removeEventListener("resize", medir);
      document.removeEventListener("pointerdown", aoClicar);
      if (quadro !== null) cancelAnimationFrame(quadro);
    };
  }, [reduce, tamanho, raio, quantidade, duracao]);

  return <canvas ref={ref} aria-hidden="true" className="pointer-events-none fixed inset-0 z-[60] h-full w-full mix-blend-difference" />;
}
