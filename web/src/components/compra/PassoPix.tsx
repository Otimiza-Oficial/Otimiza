"use client";

import Image from "next/image";
import { CampoCopiavel } from "@/components/ui/CampoCopiavel";
import type { Compra } from "@/lib/api";
import { formatarRelogio } from "./useAcompanhar";

export function PassoPix({ compra, restante, aoVoltar }: { compra: Compra; restante: number | null; aoVoltar: () => void }) {
  return (
    <div className="space-y-5">
      <div className="flex flex-col items-center gap-4 sm:flex-row sm:items-start">
        {compra.pix?.qrBase64 ? (
          <div className="shrink-0 rounded-[var(--radius-inner)] bg-surface p-3 shadow-[0_0_0_1px_var(--color-line-strong)]">
            <Image
              src={`data:image/png;base64,${compra.pix.qrBase64}`}
              alt="QR Code do Pix"
              width={176}
              height={176}
              unoptimized
            />
          </div>
        ) : null}

        <div className="min-w-0 flex-1 space-y-2 text-center sm:text-left">
          <p className="text-[14px] font-semibold">Pague com o app do seu banco</p>
          <p className="text-[13px] leading-[1.55] text-muted">
            Leia o QR Code ou use o copia e cola. Quando o pagamento cair, a chave aparece aqui sozinha.
          </p>
          {restante !== null && (
            <p className="font-mono text-[12.5px] text-subtle">
              {restante > 0 ? `vale por mais ${formatarRelogio(restante)}` : "conferindo o pagamento…"}
            </p>
          )}
        </div>
      </div>

      <CampoCopiavel rotulo="Pix copia e cola" valor={compra.pix?.copiaECola ?? ""} />

      <div className="flex items-center justify-between gap-3 border-t border-line pt-4">
        <p className="flex items-center gap-2 text-[12.5px] text-subtle">
          <span className="relative flex size-2">
            <span className="absolute inline-flex size-full animate-ping rounded-full bg-fg/40" />
            <span className="relative inline-flex size-2 rounded-full bg-fg" />
          </span>
          Esperando o pagamento
        </p>
        <button type="button" onClick={aoVoltar} className="text-[12.5px] font-medium text-muted underline underline-offset-2">
          Trocar forma de pagamento
        </button>
      </div>
    </div>
  );
}
