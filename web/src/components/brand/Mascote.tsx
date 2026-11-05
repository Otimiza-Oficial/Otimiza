"use client";

import { useEffect, useId, useRef } from "react";
import { cn } from "@/lib/cn";

/**
 * O mascote do Otimiza — a bolha de vidro com olhos âmbar. Porte do `Mascote.svelte` do app, no estado "atento":
 * os olhos seguem o cursor.
 *
 * O olhar é escrito direto no elemento (um `transform` por quadro, no máximo), sem estado do React: mexer o mouse
 * não renderiza nada. Com menos movimento pedido, ele não segue, não pisca e não respira.
 */
export function Mascote({
  tamanho = 220,
  className,
  rotulo = "Mascote do Otimiza: uma bolha de vidro com dois olhos âmbar",
}: {
  tamanho?: number;
  className?: string;
  rotulo?: string;
}) {
  const raiz = useRef<SVGSVGElement>(null);
  const olhos = useRef<SVGGElement>(null);
  const id = useId().replace(/:/g, "");

  useEffect(() => {
    const reduzir = window.matchMedia("(prefers-reduced-motion: reduce)");
    let quadro = 0;
    let ultimo: PointerEvent | null = null;

    const mover = (e: PointerEvent) => {
      ultimo = e;
      if (quadro || reduzir.matches) return;
      quadro = requestAnimationFrame(() => {
        quadro = 0;
        if (!ultimo || !raiz.current || !olhos.current) return;
        const r = raiz.current.getBoundingClientRect();
        const dx = ultimo.clientX - (r.left + r.width / 2);
        const dy = ultimo.clientY - (r.top + r.height / 2);
        const d = Math.hypot(dx, dy) || 1;
        const alcance = Math.min(1, d / 400);
        olhos.current.style.transform = `translate(${((dx / d) * 7 * alcance).toFixed(2)}px, ${((dy / d) * 5 * alcance).toFixed(2)}px)`;
      });
    };

    const aoMudarPreferencia = () => {
      if (reduzir.matches && olhos.current) olhos.current.style.transform = "";
    };

    window.addEventListener("pointermove", mover, { passive: true });
    reduzir.addEventListener("change", aoMudarPreferencia);
    return () => {
      window.removeEventListener("pointermove", mover);
      reduzir.removeEventListener("change", aoMudarPreferencia);
      cancelAnimationFrame(quadro);
    };
  }, []);

  const g = (nome: string) => `m-${nome}-${id}`;

  return (
    <svg
      ref={raiz}
      className={cn("block overflow-visible drop-shadow-[0_18px_30px_rgb(47_95_168/0.25)]", className)}
      width={tamanho}
      height={tamanho * 0.82}
      viewBox="0 0 220 180"
      role="img"
      aria-label={rotulo}
    >
      <defs>
        <radialGradient id={g("corpo")} cx="38%" cy="30%" r="75%">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.95" />
          <stop offset="45%" stopColor="#e3ebfd" stopOpacity="0.85" />
          <stop offset="85%" stopColor="#b9c8f6" stopOpacity="0.8" />
          <stop offset="100%" stopColor="#9fb2f0" stopOpacity="0.9" />
        </radialGradient>
        <linearGradient id={g("borda")} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#ffe2c2" />
          <stop offset="50%" stopColor="#a9b8f2" />
          <stop offset="100%" stopColor="#7f97ea" />
        </linearGradient>
        <linearGradient id={g("orelha")} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.9" />
          <stop offset="100%" stopColor="#b3c3f5" stopOpacity="0.75" />
        </linearGradient>
        <linearGradient id={g("olho")} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#e8a24a" />
          <stop offset="55%" stopColor="#9a5418" />
          <stop offset="100%" stopColor="#4a2a10" />
        </linearGradient>
      </defs>

      <g className="mascote-flutua">
        <path d="M44 62 Q0 91 44 120 Z" fill={`url(#${g("orelha")})`} stroke={`url(#${g("borda")})`} strokeWidth="1.5" />
        <path d="M176 62 Q220 91 176 120 Z" fill={`url(#${g("orelha")})`} stroke={`url(#${g("borda")})`} strokeWidth="1.5" />

        <circle cx="110" cy="91" r="76" fill={`url(#${g("corpo")})`} stroke={`url(#${g("borda")})`} strokeWidth="3" />
        <circle cx="110" cy="91" r="70" fill="none" stroke="#ffffff" strokeOpacity="0.55" strokeWidth="1.2" />
        <ellipse cx="80" cy="46" rx="26" ry="11" fill="#ffffff" opacity="0.55" transform="rotate(-24 80 46)" />

        <g ref={olhos} className="mascote-olhos">
          <g className="mascote-olho">
            <rect x="83" y="72" width="17" height="34" rx="8.5" fill={`url(#${g("olho")})`} />
            <rect x="87" y="76" width="4" height="10" rx="2" fill="#ffffff" opacity="0.7" />
          </g>
          <g className="mascote-olho">
            <rect x="120" y="72" width="17" height="34" rx="8.5" fill={`url(#${g("olho")})`} />
            <rect x="124" y="76" width="4" height="10" rx="2" fill="#ffffff" opacity="0.7" />
          </g>
        </g>
      </g>
    </svg>
  );
}
