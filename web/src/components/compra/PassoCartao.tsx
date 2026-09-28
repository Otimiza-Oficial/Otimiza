"use client";

import dynamic from "next/dynamic";
import { Loader2, Lock } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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

/** Uma vez por página: o Mercado Pago guarda a instância. A promessa evita iniciar duas vezes. */
let iniciando: Promise<void> | null = null;
function iniciarMercadoPago(): Promise<void> {
  iniciando ??= import("@mercadopago/sdk-react").then(({ initMercadoPago }) => {
    initMercadoPago(MP_PUBLIC_KEY, { locale: "pt-BR" });
  });
  return iniciando;
}

const PRAZO_PARA_ABRIR_MS = 20_000;

/*
 * O `CardPayment` do Mercado Pago DESTRÓI e reconstrói o formulário sempre que `initialization`, `customization`
 * ou qualquer callback muda de identidade (é a lista de dependências do efeito dele). Objeto ou função nova a cada
 * desenho reconstruía o formulário ao abrir, ao clicar em Pagar (o Checkout limpa o erro e redesenha) e quando o
 * banco recusava, apagando o que o cliente digitou. Por isso tudo que vai para ele é estável.
 */
const PERSONALIZACAO = {
  paymentMethods: { maxInstallments: 1, types: { included: ["credit_card" as const] } },
  visual: {
    hideFormTitle: true,
    style: {
      theme: "default",
      customVariables: { baseColor: "#111111", borderRadiusMedium: "8px", borderRadiusLarge: "12px" },
    },
  },
};

/**
 * Os campos do cartão são janelas do Mercado Pago dentro da página: o número
 * nunca passa pelo nosso código. O que chega aqui é um token de uso único.
 *
 * Só erro `critical` derruba o formulário. Os `non_critical` (número incompleto,
 * bandeira que não carregou num instante) o próprio formulário mostra no campo;
 * tratá-los como falha trocava o formulário pelo aviso no meio da digitação.
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
  const [iniciado, setIniciado] = useState(false);
  const [pronto, setPronto] = useState(false);
  // O código do erro vai no aviso: é o que o atendimento precisa para saber a causa.
  const [falha, setFalha] = useState<string | null>(null);
  // Trocar a chave monta o formulário de novo do zero.
  const [tentativa, setTentativa] = useState(0);

  const inicializacao = useMemo(() => ({ amount: valor }), [valor]);
  // A função mais nova do pai, sem mudar a identidade do que o Mercado Pago recebe.
  const aoPagarAtual = useRef(aoPagar);
  useEffect(() => {
    aoPagarAtual.current = aoPagar;
  }, [aoPagar]);
  const aoFicarPronto = useCallback(() => setPronto(true), []);
  const aoErrar = useCallback((e: { type?: string; cause?: string; message?: string }) => {
    console.warn("Mercado Pago:", e?.type, e?.cause, e?.message);
    if (e?.type === "critical") setFalha(e.cause || "erro_critico");
  }, []);
  const aoEnviar = useCallback(
    async (dados: {
      token: string;
      issuer_id: string;
      payment_method_id: string;
      installments: number;
      payer?: { email?: string; identification?: unknown };
    }) =>
      aoPagarAtual.current({
        token: dados.token,
        issuer_id: dados.issuer_id,
        payment_method_id: dados.payment_method_id,
        installments: dados.installments,
        payer: {
          email: dados.payer?.email,
          identification: dados.payer?.identification as DadosDoCartao["payer"]["identification"],
        },
      }),
    []
  );

  useEffect(() => {
    let vivo = true;
    iniciarMercadoPago()
      .then(() => vivo && setIniciado(true))
      .catch(() => vivo && setFalha("sdk_nao_carregou"));
    return () => {
      vivo = false;
    };
  }, []);

  // Com chave inválida ou rede bloqueando o Mercado Pago, ele falha sem chamar
  // `onError`: sem prazo, a tela ficaria em "abrindo" para sempre.
  useEffect(() => {
    if (pronto || falha) return;
    const prazo = setTimeout(() => setFalha("demorou_para_abrir"), PRAZO_PARA_ABRIR_MS);
    return () => clearTimeout(prazo);
  }, [pronto, falha, tentativa]);

  function tentarDeNovo() {
    setFalha(null);
    setPronto(false);
    setTentativa((t) => t + 1);
  }

  return (
    <div className="space-y-4">
      {!pronto && !falha && (
        <p className="flex items-center gap-2 py-6 text-[13px] text-subtle">
          <Loader2 size={15} className="animate-spin" aria-hidden="true" />
          Abrindo o formulário seguro do Mercado Pago…
        </p>
      )}

      {falha ? (
        <div className="space-y-3">
          <Aviso>
            O formulário do cartão não abriu agora. Tente de novo, ou pague por Pix.{" "}
            <span className="text-subtle">(código: {falha})</span>
          </Aviso>
          <button
            type="button"
            onClick={tentarDeNovo}
            className="text-[12.5px] font-medium text-muted underline underline-offset-2"
          >
            Tentar de novo
          </button>
        </div>
      ) : (
        iniciado && (
          <CardPayment
            key={tentativa}
            locale="pt-BR"
            initialization={inicializacao}
            customization={PERSONALIZACAO}
            onReady={aoFicarPronto}
            onError={aoErrar}
            onSubmit={aoEnviar}
          />
        )
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
