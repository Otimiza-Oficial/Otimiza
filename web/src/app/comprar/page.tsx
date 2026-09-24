import type { Metadata } from "next";
import { Check, Lock, MessageCircle } from "lucide-react";
import { Checkout } from "@/components/compra/Checkout";
import { Footer } from "@/components/layout/Footer";
import { Navbar } from "@/components/layout/Navbar";
import { SmartLink } from "@/components/ui/SmartLink";
import { TEM_CARTAO, TEM_CHECKOUT } from "@/lib/api";
import { formatarReais, links, PRECO_ADICIONAL_BRL, PRECO_BRL, site } from "@/lib/site";

export const metadata: Metadata = {
  title: "Comprar",
  description: `Licença vitalícia do Otimiza por ${formatarReais(PRECO_BRL)}, para um computador. Pix ou cartão, com a chave na hora.`,
  alternates: { canonical: `${site.url}/comprar/` },
};

const INCLUI = [
  "Todas as otimizações, cada uma com o valor anterior guardado",
  "Medição antes e depois, e o desfazer automático do que piorar",
  "Atualizações sem cobrar de novo",
  "Reemissão grátis da chave quando você formatar",
];

export default function ComprarPage() {
  const formas = TEM_CARTAO ? "Pix ou cartão de crédito" : "Pix";

  return (
    <>
      <Navbar />
      <main id="conteudo" className="frame">
        <div className="frame-inner py-12 sm:py-16">
          <div className="grid gap-8 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] lg:gap-12">
            <section>
              <p className="font-mono text-[11px] tracking-[0.22em] text-subtle uppercase">Licença vitalícia</p>
              <h1 className="font-display mt-4 text-[clamp(26px,3.4vw,36px)] leading-[1.1] font-semibold tracking-[-0.035em]">
                Uma chave, um computador, para sempre.
              </h1>
              <p className="mt-3 text-[14.5px] leading-[1.6] text-muted">
                Baixar e medir é grátis. A chave libera as otimizações que escrevem no Windows.
              </p>

              <div className="mt-7 flex items-baseline gap-2">
                <span className="font-display text-[40px] leading-none font-semibold tracking-[-0.03em]">
                  {formatarReais(PRECO_BRL)}
                </span>
                <span className="text-[13px] text-subtle">uma vez · {formas}</span>
              </div>

              <ul className="mt-6 space-y-2.5">
                {INCLUI.map((item) => (
                  <li key={item} className="flex gap-2.5 text-[13.5px] leading-[1.5]">
                    <Check size={15} strokeWidth={2.25} className="mt-0.5 shrink-0" aria-hidden="true" />
                    {item}
                  </li>
                ))}
              </ul>

              <p className="mt-6 text-[12.5px] text-subtle">
                Segundo computador por {formatarReais(PRECO_ADICIONAL_BRL)}.
              </p>

              <p className="mt-6 flex gap-2 border-t border-line pt-5 text-[12.5px] leading-[1.55] text-subtle">
                <Lock size={13} className="mt-0.5 shrink-0" aria-hidden="true" />
                Quem processa o pagamento é o Mercado Pago. Os dados do cartão são digitados nos campos seguros dele e
                nunca passam por este site nem pelos nossos servidores.
              </p>
            </section>

            <section className="rounded-[var(--radius-card)] bg-surface p-5 shadow-[var(--shadow-float)] sm:p-7">
              {TEM_CHECKOUT ? (
                <Checkout />
              ) : (
                <>
                  <h2 className="font-display text-[18px] font-semibold tracking-[-0.02em]">Comprar no Discord</h2>
                  <p className="mt-1.5 text-[13px] leading-[1.55] text-muted">
                    Você manda o código do seu computador, paga por Pix e recebe a chave ali mesmo.
                  </p>
                  <SmartLink
                    href={links.discord}
                    className="mt-4 inline-flex h-11 items-center gap-2 rounded-[var(--radius-control)] bg-ink px-4 text-[13.5px] font-semibold text-white"
                  >
                    <MessageCircle size={15} aria-hidden="true" />
                    Abrir o Discord
                  </SmartLink>
                </>
              )}
            </section>
          </div>

          <div className="mt-10 space-y-2 text-[12.5px] text-subtle">
            <p>
              Ao comprar você concorda com os{" "}
              <SmartLink href="/termos/" className="font-medium text-fg underline underline-offset-2">
                termos de uso
              </SmartLink>{" "}
              e com a{" "}
              <SmartLink href="/privacidade/" className="font-medium text-fg underline underline-offset-2">
                política de privacidade
              </SmartLink>
              . Sete dias para desistir, contados do pagamento.
            </p>
            <p>
              Ainda não instalou?{" "}
              <SmartLink href={links.baixar} className="font-medium text-fg underline underline-offset-2">
                Baixe o Otimiza
              </SmartLink>{" "}
              e meça antes: o código da máquina aparece na tela de ativação. Problema com o pagamento?{" "}
              <SmartLink href={links.discord} className="font-medium text-fg underline underline-offset-2">
                Fale no Discord
              </SmartLink>
              .
            </p>
          </div>
        </div>
      </main>
      <Footer />
    </>
  );
}
