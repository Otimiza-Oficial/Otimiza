"use client";

import { CreditCard, Loader2, QrCode } from "lucide-react";
import type { ReactNode } from "react";
import { Aviso } from "@/components/ui/Aviso";
import type { Metodo } from "@/lib/api";

export function PassoMetodo({
  codigo,
  temCartao,
  carregando,
  erro,
  aoEscolher,
  aoTrocarCodigo,
}: {
  codigo: string;
  temCartao: boolean;
  carregando: Metodo | null;
  erro: string | null;
  aoEscolher: (metodo: Metodo) => void;
  aoTrocarCodigo: () => void;
}) {
  return (
    <div className="space-y-4">
      <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-[12.5px] text-subtle">
        Chave para <span className="font-mono text-[12.5px] text-fg">{codigo}</span>
        <button type="button" onClick={aoTrocarCodigo} className="font-medium text-fg underline underline-offset-2">
          trocar
        </button>
      </p>

      <div className={`grid gap-3 ${temCartao ? "sm:grid-cols-2" : ""}`}>
        <Opcao
          icone={<QrCode size={20} aria-hidden="true" />}
          titulo="Pix"
          detalhe="Aprovado em segundos"
          carregando={carregando === "pix"}
          desabilitada={carregando !== null}
          aoClicar={() => aoEscolher("pix")}
        />
        {temCartao && (
          <Opcao
            icone={<CreditCard size={20} aria-hidden="true" />}
            titulo="Cartão de crédito"
            detalhe="À vista, sem parcelas"
            carregando={carregando === "cartao"}
            desabilitada={carregando !== null}
            aoClicar={() => aoEscolher("cartao")}
          />
        )}
      </div>

      {erro && <Aviso>{erro}</Aviso>}
    </div>
  );
}

function Opcao({
  icone,
  titulo,
  detalhe,
  carregando,
  desabilitada,
  aoClicar,
}: {
  icone: ReactNode;
  titulo: string;
  detalhe: string;
  carregando: boolean;
  desabilitada: boolean;
  aoClicar: () => void;
}) {
  return (
    <button
      type="button"
      onClick={aoClicar}
      disabled={desabilitada}
      className="group flex items-center gap-3 rounded-[var(--radius-inner)] bg-surface p-4 text-left shadow-[0_0_0_1px_var(--color-line-strong)] transition-all hover:shadow-[0_0_0_1.5px_var(--color-fg)] disabled:cursor-default disabled:opacity-60"
    >
      <span className="grid size-10 shrink-0 place-items-center rounded-[var(--radius-control)] bg-card text-fg transition-colors group-hover:bg-ink group-hover:text-white">
        {carregando ? <Loader2 size={18} className="animate-spin" aria-hidden="true" /> : icone}
      </span>
      <span>
        <span className="block text-[14px] font-semibold">{titulo}</span>
        <span className="block text-[12.5px] text-subtle">{carregando ? "Preparando…" : detalhe}</span>
      </span>
    </button>
  );
}
