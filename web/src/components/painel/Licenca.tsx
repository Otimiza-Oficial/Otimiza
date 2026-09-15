"use client";

import { Check, Copy, KeyRound, LoaderCircle, Trash2, TriangleAlert } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/cn";
import { conferir, explicar, formatarData, type Resultado } from "@/lib/licenca";

/** Confere uma chave (e, se houver, contra um código de máquina). */
export function useConferencia(chave: string, maquina: string | null) {
  const [resultado, setResultado] = useState<Resultado | null>(null);
  const [conferida, setConferida] = useState<string | null>(null);
  const alvo = `${chave}|${maquina ?? ""}`;

  useEffect(() => {
    let vivo = true;
    conferir(chave, maquina).then((r) => {
      if (!vivo) return;
      setResultado(r);
      setConferida(alvo);
    });
    return () => {
      vivo = false;
    };
  }, [chave, maquina, alvo]);

  // Enquanto a conferência do alvo atual não volta, o estado é "conferindo" —
  // nunca o resultado de uma chave anterior.
  return conferida === alvo ? resultado : null;
}

export function StatusLicenca({ resultado, className }: { resultado: Resultado | null; className?: string }) {
  if (!resultado) {
    return (
      <span className={cn("inline-flex items-center gap-1.5 text-[12px] font-medium text-muted", className)}>
        <LoaderCircle size={13} strokeWidth={2.25} className="animate-spin" aria-hidden="true" />
        Conferindo
      </span>
    );
  }
  return resultado.ok ? (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md bg-ink px-2 py-1 text-[11.5px] font-semibold text-white",
        className,
      )}
    >
      <Check size={12} strokeWidth={3} aria-hidden="true" />
      {resultado.dados.expira ? "Válida" : "Vitalícia"}
    </span>
  ) : (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md bg-[#f1f1f0] px-2 py-1 text-[11.5px] font-semibold text-fg shadow-[inset_0_0_0_1px_rgb(10_10_10/0.1)]",
        className,
      )}
    >
      <TriangleAlert size={12} strokeWidth={2.5} aria-hidden="true" />
      Não confere
    </span>
  );
}

export function CartaoLicenca({
  chave,
  adicionadaEm,
  maquina,
  onRemover,
}: {
  chave: string;
  adicionadaEm: string;
  maquina: string | null;
  onRemover: () => void;
}) {
  const resultado = useConferencia(chave, maquina);
  const [copiada, setCopiada] = useState(false);
  const [confirmando, setConfirmando] = useState(false);

  const copiar = async () => {
    try {
      await navigator.clipboard.writeText(chave);
      setCopiada(true);
      setTimeout(() => setCopiada(false), 1800);
    } catch {}
  };

  const dados = resultado?.ok ? resultado.dados : null;

  return (
    <article className="rounded-[14px] bg-white p-5 shadow-[0_0_0_1px_rgb(10_10_10/0.08),0_1px_2px_rgb(0_0_0/0.04)] sm:p-6">
      <div className="flex items-start justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <span className="grid size-10 shrink-0 place-items-center rounded-[10px] bg-[#f1f1f0]">
            <KeyRound size={18} strokeWidth={2} aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <h3 className="font-display text-[16px] font-semibold tracking-[-0.02em]">
              {dados ? "Licença do Otimiza" : "Chave guardada"}
            </h3>
            <p className="truncate font-mono text-[12px] text-subtle">{chave.slice(0, 18)}…</p>
          </div>
        </div>
        <StatusLicenca resultado={resultado} className="shrink-0" />
      </div>

      {dados && (
        <dl className="mt-5 grid gap-x-6 gap-y-3 border-t border-line pt-5 text-[13px] sm:grid-cols-2">
          <Campo nome="Computador" valor={<span className="font-mono">{dados.maquina}</span>} />
          <Campo nome="Em nome de" valor={dados.comprador || "—"} />
          <Campo nome="Emitida em" valor={formatarData(dados.emitida)} />
          <Campo nome="Validade" valor={dados.expira ? `Até ${formatarData(dados.expira)}` : "Vitalícia"} />
        </dl>
      )}

      {resultado && !resultado.ok && (
        <p className="mt-5 border-t border-line pt-5 text-[13px] leading-[1.55] text-[#2b2b2b]">
          {explicar(resultado.recusa)}
        </p>
      )}

      <div className="mt-5 flex flex-wrap items-center justify-between gap-3">
        <p className="text-[11.5px] text-subtle">
          Guardada neste navegador em {new Date(adicionadaEm).toLocaleDateString("pt-BR")}
        </p>
        <div className="flex gap-2">
          <Button variant="secondary" onClick={copiar}>
            {copiada ? <Check size={14} strokeWidth={2.5} aria-hidden="true" /> : <Copy size={14} strokeWidth={2} aria-hidden="true" />}
            {copiada ? "Copiada" : "Copiar chave"}
          </Button>
          {confirmando ? (
            <Button onClick={onRemover}>Remover mesmo</Button>
          ) : (
            <Button variant="secondary" onClick={() => setConfirmando(true)} aria-label="Remover esta chave do navegador">
              <Trash2 size={14} strokeWidth={2} aria-hidden="true" />
            </Button>
          )}
        </div>
      </div>
    </article>
  );
}

function Campo({ nome, valor }: { nome: string; valor: React.ReactNode }) {
  return (
    <div>
      <dt className="text-[11.5px] text-subtle">{nome}</dt>
      <dd className="mt-0.5 font-medium">{valor}</dd>
    </div>
  );
}
