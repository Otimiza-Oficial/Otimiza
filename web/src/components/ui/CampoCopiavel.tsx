"use client";

import { Check, Copy } from "lucide-react";
import { useState } from "react";

export function CampoCopiavel({ rotulo, valor }: { rotulo: string; valor: string }) {
  const [copiado, setCopiado] = useState(false);

  async function copiar() {
    try {
      await navigator.clipboard.writeText(valor);
      setCopiado(true);
      setTimeout(() => setCopiado(false), 1600);
    } catch {
      setCopiado(false);
    }
  }

  return (
    <div>
      <p className="mb-1.5 text-[11.5px] font-medium text-muted">{rotulo}</p>
      <div className="flex items-stretch gap-2">
        <code className="min-w-0 flex-1 overflow-x-auto rounded-[var(--radius-control)] bg-surface px-3 py-2.5 font-mono text-[12px] break-all shadow-[0_0_0_1px_var(--color-line-strong)]">
          {valor}
        </code>
        <button
          type="button"
          onClick={copiar}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-[var(--radius-control)] bg-surface px-3 text-[12.5px] font-semibold shadow-[0_0_0_1px_var(--color-line-strong)] transition-colors hover:bg-card"
        >
          {copiado ? <Check size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
          <span aria-live="polite">{copiado ? "Copiado" : "Copiar"}</span>
        </button>
      </div>
    </div>
  );
}
