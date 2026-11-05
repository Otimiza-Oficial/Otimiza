import { ArrowRight, Check, Download } from "lucide-react";
import { Mascote } from "@/components/brand/Mascote";
import { Reveal } from "@/components/motion/Reveal";
import { ButtonLink } from "@/components/ui/Button";
import { links, PRECO_BRL, VERSAO_EXIBIDA } from "@/lib/site";
import { CartaoDeProva } from "./CartaoDeProva";

/*
 * O topo: a promessa à esquerda; à direita, o mascote olhando para quem chega e o Cartão de Prova — um número medido,
 * não uma promessa (plano 1.0, cap. 12). O título não diz quanto o PC vai ganhar, porque isso só a medição diz.
 */
export function Hero() {
  return (
    <section aria-labelledby="hero-titulo">
      <div className="frame">
        <div className="frame-inner pt-10 pb-16 sm:pt-16 lg:pt-20 lg:pb-24">
          <div className="grid items-center gap-y-12 lg:grid-cols-[1.05fr_1fr] lg:gap-x-12">
            <div className="min-w-0">
              <Reveal>
                <p className="vidro inline-flex items-center gap-2 rounded-full! py-1 pr-3.5 pl-1.5 text-[12.5px] text-muted">
                  <span className="rounded-full bg-ink px-2 py-0.5 text-[11px] font-semibold text-white">Novo</span>
                  <span>
                    <span className="font-semibold text-fg">Otimiza {VERSAO_EXIBIDA}</span> · Nova Geração
                  </span>
                </p>
              </Reveal>

              <Reveal delay={0.05}>
                <h1
                  id="hero-titulo"
                  className="font-display mt-6 text-[40px] leading-[1.04] font-semibold tracking-[-0.05em] text-balance text-fg sm:text-[54px] lg:text-[62px]"
                >
                  Menos travada no jogo.{" "}
                  <span className="text-ceu-700 dark:text-lavanda-400">Medido, não prometido.</span>
                </h1>
              </Reveal>

              <Reveal delay={0.12}>
                <p className="mt-6 max-w-[540px] text-[16px] leading-[1.6] text-pretty text-muted sm:text-[17px]">
                  O Otimiza lê o seu PC, liga só o que ajuda nele e mede antes e depois. Quando nada mudou, ele diz. E
                  tudo o que ele altera volta ao valor exato com um clique.
                </p>
              </Reveal>

              <Reveal delay={0.18}>
                <div className="mt-8 flex flex-col gap-3 sm:flex-row sm:items-center">
                  <ButtonLink href={links.baixar} size="md" className="w-full sm:w-auto">
                    <Download size={16} strokeWidth={2.25} aria-hidden="true" />
                    Baixar o Otimiza {VERSAO_EXIBIDA}
                  </ButtonLink>
                  <ButtonLink href="#preco" variant="secondary" size="md" className="w-full sm:w-auto">
                    Ver preço
                    <ArrowRight size={15} strokeWidth={2} aria-hidden="true" />
                  </ButtonLink>
                </div>
                <ul className="mt-6 flex flex-wrap gap-x-5 gap-y-2 text-[13px] text-muted">
                  {["Grátis para baixar e medir", `Ativação R$ ${PRECO_BRL}, uma vez`, "Windows 10 e 11, 64 bits"].map(
                    (t) => (
                      <li key={t} className="flex items-center gap-1.5">
                        <Check size={14} strokeWidth={2.5} className="text-ceu-700 dark:text-lavanda-400" aria-hidden="true" />
                        {t}
                      </li>
                    ),
                  )}
                </ul>
              </Reveal>
            </div>

            <Reveal delay={0.12} className="relative min-w-0">
              <div className="relative mx-auto max-w-[440px]">
                <div className="relative z-10 -mb-10 flex justify-center sm:-mb-12">
                  <Mascote tamanho={190} className="h-auto w-[150px] sm:w-[190px]" />
                </div>
                <CartaoDeProva />
              </div>
            </Reveal>
          </div>
        </div>
      </div>
    </section>
  );
}
