import type { Metadata } from "next";
import { Check, MessageCircle } from "lucide-react";
import { Footer } from "@/components/layout/Footer";
import { Navbar } from "@/components/layout/Navbar";
import { SmartLink } from "@/components/ui/SmartLink";
import { TEM_CHECKOUT } from "@/lib/api";
import { formatarReais, links, PRECO_ADICIONAL_BRL, PRECO_BRL, site } from "@/lib/site";
import { CheckoutPix } from "@/components/compra/CheckoutPix";

export const metadata: Metadata = {
  title: "Comprar",
  description: `Licença vitalícia do Otimiza por ${formatarReais(PRECO_BRL)}, para um computador. Pagamento por Pix ou pelo Discord.`,
  alternates: { canonical: `${site.url}/comprar/` },
};

/*
 * A PÁGINA DE COMPRA EXISTE NOS DOIS ESTADOS.
 *
 * Com o checkout ligado (`NEXT_PUBLIC_OTIMIZA_API` na compilação), ela cobra
 * por Pix e entrega a chave na tela. Sem ele, ela continua sendo a página que
 * explica o preço e leva ao Discord, onde a compra já funciona hoje.
 *
 * Os dois caminhos entregam A MESMA COISA e por isso ficam na mesma página: o
 * que muda é quem digita o código da máquina — a pessoa aqui, ou o atendente
 * lá. Esconder o Discord quando o Pix está ligado seria tirar a saída de quem
 * tem problema com o pagamento; esconder o preço quando ele está desligado
 * seria fingir que a página não vende.
 */
export default function ComprarPage() {
  return (
    <>
      <Navbar />
      <main id="conteudo" className="frame">
        <div className="frame-inner py-14 sm:py-20">
          <div className="max-w-2xl">
            <p className="font-mono text-[11px] tracking-[0.22em] text-subtle uppercase">Licença</p>
            <h1 className="font-display mt-5 text-[clamp(28px,4vw,42px)] leading-[1.08] font-semibold tracking-[-0.035em]">
              Uma chave, um computador, para sempre.
            </h1>
            <p className="mt-4 text-[15px] leading-[1.6] text-muted">
              O Otimiza é grátis para baixar e medir. A chave libera as otimizações que escrevem no Windows — e ela é
              vitalícia: não há mensalidade, não há renovação.
            </p>
          </div>

          <div className="mt-10 grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)]">
            <section className="rounded-[var(--radius-card)] bg-white p-5 shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_1px_2px_rgb(10_10_10/0.04)]">
              <p className="font-display text-[34px] leading-none font-semibold tracking-[-0.03em]">
                {formatarReais(PRECO_BRL)}
              </p>
              <p className="mt-1.5 text-[13px] text-subtle">uma vez, por computador</p>

              <ul className="mt-5 space-y-2.5">
                {[
                  "Todas as otimizações reversíveis, com o valor anterior guardado",
                  "Medição antes e depois, e o desfazer automático do que piorar",
                  "Atualizações do produto, sem cobrar de novo",
                  "Reemissão da chave no Discord, sem custo, quando formatar",
                  `Segundo computador por ${formatarReais(PRECO_ADICIONAL_BRL)}`,
                ].map((item) => (
                  <li key={item} className="flex gap-2 text-[13.5px] leading-[1.5]">
                    <Check size={15} strokeWidth={2.25} className="mt-0.5 shrink-0" aria-hidden="true" />
                    {item}
                  </li>
                ))}
              </ul>

              <p className="mt-5 border-t border-line pt-4 text-[12.5px] leading-[1.5] text-subtle">
                O Otimiza não guarda cartão e não cobra assinatura. O pagamento é Pix, e quem processa é o provedor —
                este site nunca vê dado de pagamento.
              </p>
            </section>

            <section className="rounded-[var(--radius-card)] bg-[#f4f4f3] p-5">
              {TEM_CHECKOUT ? (
                <>
                  <h2 className="font-display text-[18px] font-semibold tracking-[-0.02em]">Pagar por Pix</h2>
                  <p className="mt-1.5 mb-4 text-[13px] text-muted">
                    A chave é emitida na hora, para o computador cujo código você informar.
                  </p>
                  <CheckoutPix />
                </>
              ) : (
                <>
                  <h2 className="font-display text-[18px] font-semibold tracking-[-0.02em]">Comprar no Discord</h2>
                  <p className="mt-1.5 text-[13px] leading-[1.55] text-muted">
                    Hoje a compra acontece no Discord, com uma pessoa: você manda o código do seu computador, paga por
                    Pix e recebe a chave ali mesmo. É o mesmo canal onde a chave é reemitida se você formatar.
                  </p>
                  <SmartLink
                    href={links.discord}
                    className="mt-4 inline-flex h-11 items-center gap-2 rounded-[8px] bg-ink px-4 text-[13.5px] font-semibold text-white"
                  >
                    <MessageCircle size={15} aria-hidden="true" />
                    Abrir o Discord
                  </SmartLink>
                  <p className="mt-4 text-[12.5px] leading-[1.5] text-subtle">
                    O pagamento direto no site está pronto e desligado: ele só entra quando o serviço que emite a chave
                    estiver no ar em endereço próprio, com HTTPS. Um botão de pagar que não funciona seria pior que
                    botão nenhum.
                  </p>
                </>
              )}
            </section>
          </div>

          <p className="mt-8 text-[12.5px] text-subtle">
            Antes de comprar, instale e meça: o download é grátis e mostra o que a sua máquina tem.{" "}
            <SmartLink href={links.baixar} className="font-medium text-fg underline underline-offset-2">
              Baixar o Otimiza
            </SmartLink>
          </p>
        </div>
      </main>
      <Footer />
    </>
  );
}
