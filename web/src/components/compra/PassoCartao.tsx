"use client";

import dynamic from "next/dynamic";
import { Loader2, Lock } from "lucide-react";
import { useEffect, useState } from "react";
import { Aviso } from "@/components/ui/Aviso";
import { MP_PUBLIC_KEY } from "@/lib/api";

const CardPayment = dynamic(() => import("@mercadopago/sdk-react").then((m) => m.CardPayment), { ssr: false });

export type DadosDoCartao = {
  token: string;
  issuer_id: string;
  payment_method_id: string;
  installments: number;
  payer: { email?: string; identification?: { type: string; number: string } };
};

let iniciado = false;

const PRAZO_PARA_ABRIR_MS = 15_000;

/**
 * Os campos do cartão são janelas do Mercado Pago dentro da página: o número
 * nunca passa pelo nosso código. O que chega aqui é um token de uso único.
 */
export function PassoCartao({
  valor,
  erro,
  aoPagar,
  aoVoltar,
}: {
  valor: number;
  erro: string | null;
  aoPagar: (dados: DadosDoCartao) => Promise<void>;
  aoVoltar: () => void;
}) {
  const [pronto, setPronto] = useState(false);
  const [falhou, setFalhou] = useState(false);

  useEffect(() => {
    if (iniciado) return;
    import("@mercadopago/sdk-react").then(({ initMercadoPago }) => {
      initMercadoPago(MP_PUBLIC_KEY, { locale: "pt-BR" });
      iniciado = true;
    });
  }, []);

  // Com chave inválida o Mercado Pago falha sem chamar `onError`: sem prazo, a
  // tela ficaria em "abrindo" para sempre.
  useEffect(() => {
    if (pronto) return;
    const prazo = setTimeout(() => setFalhou(true), PRAZO_PARA_ABRIR_MS);
    return () => clearTimeout(prazo);
  }, [pronto]);

  return (
    <div className="space-y-4">
      {!pronto && !falhou && (
        <p className="flex items-center gap-2 py-6 text-[13px] text-subtle">
          <Loader2 size={15} className="animate-spin" aria-hidden="true" />
          Abrindo o formulário seguro do Mercado Pago…
        </p>
      )}

      {falhou ? (
        <Aviso>O formulário do cartão não abriu agora. Tente de novo em instantes, ou pague por Pix.</Aviso>
      ) : (
        <CardPayment
          locale="pt-BR"
          initialization={{ amount: valor }}
          customization={{
            paymentMethods: { maxInstallments: 1, types: { included: ["credit_card"] } },
            visual: {
              style: {
                theme: "default",
                customVariables: { baseColor: "#111111", borderRadiusMedium: "8px", borderRadiusLarge: "12px" },
              },
            },
          }}
          onReady={() => setPronto(true)}
          onError={() => setFalhou(true)}
          onSubmit={async (dados) =>
            aoPagar({
              token: dados.token,
              issuer_id: dados.issuer_id,
              payment_method_id: dados.payment_method_id,
              installments: dados.installments,
              payer: {
                email: dados.payer?.email,
                identification: dados.payer?.identification as DadosDoCartao["payer"]["identification"],
              },
            })
          }
        />
      )}

      {erro && <Aviso>{erro}</Aviso>}

      <div className="flex items-center justify-between gap-3 border-t border-line pt-4">
        <p className="flex items-center gap-1.5 text-[12px] text-subtle">
          <Lock size={12} aria-hidden="true" />
          Processado pelo Mercado Pago
        </p>
        <button type="button" onClick={aoVoltar} className="text-[12.5px] font-medium text-muted underline underline-offset-2">
          Trocar forma de pagamento
        </button>
      </div>
    </div>
  );
}
