"use client";

import { useEffect, useId, useRef } from "react";
import { cn } from "@/lib/cn";

/**
 * O mascote do Otimiza, o Oti: um disco branco liso, de cabeça meio de lado, com dois olhos em cápsula inclinados
 * (o mesmo desenho de `Oti.svelte` no app e do ícone do programa). No estado "atento", os olhos seguem o cursor.
 *
 * O olhar é escrito direto no elemento (um `transform` por quadro, no máximo), sem estado do React: mexer o mouse
 * não renderiza nada. Com menos movimento pedido, ele não segue, não pisca e não respira.
 */
export function Mascote({
  tamanho = 220,
  className,
  rotulo = "Oti, o mascote do Otimiza: um disco branco com dois olhos",
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
      className={cn("block overflow-visible drop-shadow-[0_18px_28px_rgb(40_30_20/0.22)]", className)}
      width={tamanho}
      height={tamanho}
      viewBox="0 0 200 200"
      role="img"
      aria-label={rotulo}
    >
      <defs>
        <radialGradient id={g("corpo")} cx="36%" cy="30%" r="78%">
          <stop offset="0%" stopColor="#ffffff" />
          <stop offset="55%" stopColor="#f0f0f1" />
          <stop offset="85%" stopColor="#dcdcde" />
          <stop offset="100%" stopColor="#c9c9cc" />
        </radialGradient>
        <linearGradient id={g("olho")} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#050505" />
          <stop offset="100%" stopColor="#1c1c1e" />
        </linearGradient>
      </defs>

      <g className="mascote-flutua">
        <circle cx="100" cy="100" r="92" fill={`url(#${g("corpo")})`} />
        <ellipse cx="70" cy="52" rx="40" ry="20" fill="#ffffff" opacity="0.6" transform="rotate(-28 70 52)" />
        <circle cx="100" cy="100" r="91" fill="none" stroke="#ffffff" strokeWidth="1.6" opacity="0.95" />
        <circle cx="100" cy="100" r="92.5" fill="none" stroke="#000000" strokeWidth="1" opacity="0.08" />

        <g ref={olhos} className="mascote-olhos">
          {[
            [86, 112],
            [130, 88],
          ].map(([cx, cy]) => (
            <g key={cx} transform={`rotate(-33 ${cx} ${cy})`}>
              <g className="mascote-olho">
                <rect x={cx - 17} y={cy - 30} width="34" height="60" rx="17" fill="#ffffff" />
                <rect x={cx - 17} y={cy - 30} width="34" height="60" rx="17" fill="none" stroke="#000000" strokeOpacity="0.1" />
                <rect x={cx - 13} y={cy - 26} width="26" height="52" rx="13" fill={`url(#${g("olho")})`} />
              </g>
            </g>
          ))}
        </g>
      </g>
    </svg>
  );
}
