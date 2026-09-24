"use client";

import { ArrowRight } from "lucide-react";
import { useState } from "react";
import { Aviso } from "@/components/ui/Aviso";
import { codigoValido, normalizarCodigo } from "@/lib/licenca";

export function PassoCodigo({ inicial, aoConfirmar }: { inicial: string; aoConfirmar: (codigo: string) => void }) {
  const [codigo, setCodigo] = useState(inicial);
  const [erro, setErro] = useState<string | null>(null);

  function enviar(evento: React.FormEvent) {
    evento.preventDefault();
    const limpo = normalizarCodigo(codigo);
    if (!codigoValido(limpo)) {
      setErro("O código tem o formato OTZ-XXXX-XXXX-XXXX e aparece na tela de ativação do Otimiza.");
      return;
    }
    setErro(null);
    aoConfirmar(limpo);
  }

  return (
    <form onSubmit={enviar} className="space-y-4">
      <div>
        <label htmlFor="compra-codigo" className="block text-[13.5px] font-semibold">
          Código deste computador
        </label>
        <p id="compra-ajuda" className="mt-1 text-[12.5px] leading-[1.5] text-subtle">
          Abra o Otimiza e copie o código da tela de ativação. A chave vale para esse computador.
        </p>
      </div>

      <input
        id="compra-codigo"
        value={codigo}
        onChange={(e) => setCodigo(e.target.value)}
        placeholder="OTZ-XXXX-XXXX-XXXX"
        autoComplete="off"
        spellCheck={false}
        autoFocus={!inicial}
        aria-describedby="compra-ajuda"
        aria-invalid={erro ? true : undefined}
        className="h-12 w-full rounded-[var(--radius-control)] bg-surface px-4 font-mono text-[15px] tracking-[0.06em] uppercase shadow-[0_0_0_1px_var(--color-line-strong)] transition-shadow outline-none placeholder:text-subtle/60 focus:shadow-[0_0_0_2px_var(--color-fg)]"
      />

      {erro && <Aviso>{erro}</Aviso>}

      <button
        type="submit"
        className="group inline-flex h-12 w-full items-center justify-center gap-2 rounded-[var(--radius-control)] bg-ink text-[14px] font-semibold text-white transition-opacity hover:opacity-90"
      >
        Continuar
        <ArrowRight size={16} className="transition-transform group-hover:translate-x-0.5" aria-hidden="true" />
      </button>
    </form>
  );
}
