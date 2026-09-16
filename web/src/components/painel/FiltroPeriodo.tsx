"use client";

import { CalendarDays, Check, ChevronDown } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";

export const PERIODOS = [
  { meses: 6, label: "Últimos 6 meses" },
  { meses: 12, label: "Últimos 12 meses" },
  { meses: 24, label: "Últimos 24 meses" },
] as const;

/** Menu curto de período. Fecha no Esc, no clique fora e na escolha. */
export function FiltroPeriodo({ valor, aoEscolher }: { valor: number; aoEscolher: (meses: number) => void }) {
  const [aberto, setAberto] = useState(false);
  const caixa = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!aberto) return;
    const fora = (e: MouseEvent) => {
      if (caixa.current && !caixa.current.contains(e.target as Node)) setAberto(false);
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setAberto(false);
    document.addEventListener("mousedown", fora);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("mousedown", fora);
      document.removeEventListener("keydown", esc);
    };
  }, [aberto]);

  const atual = PERIODOS.find((p) => p.meses === valor) ?? PERIODOS[1];

  return (
    <div className="relative" ref={caixa}>
      <button
        type="button"
        onClick={() => setAberto((v) => !v)}
        aria-expanded={aberto}
        aria-haspopup="listbox"
        className="flex h-[26px] items-center gap-1.5 rounded-[6px] bg-white px-2 text-[11.5px] font-medium text-[#3a3a3a] shadow-[0_0_0_1px_rgb(10_10_10/0.1)] transition-colors hover:bg-[#f9f9f8]"
      >
        <CalendarDays size={12} strokeWidth={2} aria-hidden="true" />
        {atual.label}
        <ChevronDown size={12} strokeWidth={2} aria-hidden="true" className={cn("transition-transform", aberto && "rotate-180")} />
      </button>

      {aberto && (
        <ul
          role="listbox"
          className="absolute right-0 top-[30px] z-20 w-[170px] animate-[surgir_140ms_ease-out] rounded-[8px] bg-white p-1 shadow-[0_0_0_1px_rgb(10_10_10/0.1),0_10px_24px_-12px_rgb(10_10_10/0.35)]"
        >
          {PERIODOS.map((p) => (
            <li key={p.meses}>
              <button
                type="button"
                role="option"
                aria-selected={p.meses === valor}
                onClick={() => {
                  aoEscolher(p.meses);
                  setAberto(false);
                }}
                className="flex h-8 w-full items-center gap-2 rounded-[6px] px-2 text-left text-[12px] hover:bg-[#f4f4f3]"
              >
                <Check size={12} strokeWidth={2.5} className={cn("shrink-0", p.meses !== valor && "opacity-0")} aria-hidden="true" />
                {p.label}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
