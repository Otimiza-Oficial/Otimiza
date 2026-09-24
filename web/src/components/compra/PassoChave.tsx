"use client";

import { Check, KeyRound } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { CampoCopiavel } from "@/components/ui/CampoCopiavel";
import { useLicencas } from "@/lib/armazem";

export function PassoChave({ chave }: { chave: string }) {
  const { adicionar } = useLicencas();
  const [guardada, setGuardada] = useState(false);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <span className="grid size-10 place-items-center rounded-full bg-ink text-white">
          <Check size={18} strokeWidth={2.5} aria-hidden="true" />
        </span>
        <div>
          <p className="text-[15px] font-semibold">Pagamento confirmado</p>
          <p className="text-[12.5px] text-subtle">Cole esta chave no Otimiza para ativar.</p>
        </div>
      </div>

      <CampoCopiavel rotulo="Chave da licença" valor={chave} />

      <p className="text-[12.5px] leading-[1.5] text-subtle">
        Guarde a chave agora: esta página a mostra por pouco tempo. Se perder, o suporte reemite no Discord, sem custo.
      </p>

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => {
            adicionar(chave);
            setGuardada(true);
          }}
          disabled={guardada}
          className="inline-flex h-11 items-center gap-2 rounded-[var(--radius-control)] bg-ink px-4 text-[13.5px] font-semibold text-white transition-opacity hover:opacity-90 disabled:opacity-60"
        >
          <KeyRound size={15} aria-hidden="true" />
          {guardada ? "Guardada neste navegador" : "Guardar no navegador"}
        </button>
        <Link
          href="/painel/licencas/"
          className="inline-flex h-11 items-center rounded-[var(--radius-control)] bg-surface px-4 text-[13.5px] font-semibold shadow-[0_0_0_1px_var(--color-line-strong)] transition-colors hover:bg-card"
        >
          Abrir o painel
        </Link>
      </div>
    </div>
  );
}
