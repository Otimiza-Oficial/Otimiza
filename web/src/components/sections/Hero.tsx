import { ArrowRight, Check, Download } from "lucide-react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { Reveal } from "@/components/motion/Reveal";
import { Magnet } from "@/components/reactbits/Magnet";
import { RotatingText } from "@/components/reactbits/RotatingText";
import { ShinyText } from "@/components/reactbits/ShinyText";
import { Waves } from "@/components/reactbits/Waves";
import { ButtonLink } from "@/components/ui/Button";
import { links, PRECO_BRL, site } from "@/lib/site";
import { HeroPilha } from "./HeroPilha";

/*
 * O topo em duas colunas: a promessa à esquerda, o programa à direita (telas que se revezam). O título é a maior
 * coisa da página depois do preço — a faixa de frases logo abaixo é menor que ele de propósito.
 */
export function Hero() {
  return (
    <section aria-labelledby="hero-titulo">
      <div className="frame">
        <div className="frame-inner relative isolate overflow-hidden pt-14 pb-16 sm:pt-20 lg:pt-24 lg:pb-24">
          {/* Linhas que respondem ao mouse no fundo inteiro; a máscara as apaga perto do texto e nas bordas. */}
          <Waves
            lineColor="rgb(10 10 10 / 0.11)"
            className="-z-10 [mask-image:radial-gradient(ellipse_70%_75%_at_72%_50%,black_25%,transparent_75%)]"
          />

          <div className="grid items-center gap-y-14 lg:grid-cols-[1.08fr_1fr] lg:gap-x-10">
            <div className="min-w-0">
              <Reveal>
                <p className="inline-flex items-center gap-2.5 rounded-full bg-white/80 py-1 pr-3 pl-1 text-[12.5px] text-muted shadow-[0_0_0_1px_rgb(10_10_10/0.08),0_1px_2px_rgb(0_0_0/0.04)] backdrop-blur-sm">
                  <span className="grid size-6 place-items-center rounded-full bg-ink text-white">
                    <OtimizaLogo mark={14} className="text-white" />
                  </span>
                  <span>
                    <ShinyText text={`Versão ${site.versao}`} className="font-semibold" /> · Windows 10 e 11
                  </span>
                </p>
              </Reveal>

              <Reveal delay={0.05}>
                <h1
                  id="hero-titulo"
                  className="font-display mt-7 text-[44px] leading-[1.02] font-semibold tracking-[-0.055em] text-fg sm:text-[56px] lg:text-[64px]"
                >
                  {/* A frase inteira para o leitor de tela; a de cima é só desenho (a palavra do meio gira). */}
                  <span className="sr-only">Seu PC medido, otimizado e provado com número.</span>
                  <span aria-hidden="true" className="block">
                    Seu PC
                  </span>
                  <span aria-hidden="true" className="mt-1.5 block">
                    <RotatingText
                      palavras={["medido", "otimizado", "provado"]}
                      pilulaClassName="rounded-[14px] bg-ink px-3.5 pb-1.5 pt-0.5 text-white shadow-[0_18px_40px_-18px_rgb(0_0_0/0.6)] sm:rounded-[18px] sm:px-4"
                    />
                  </span>
                  <span aria-hidden="true" className="mt-1 block text-muted">
                    com número.
                  </span>
                </h1>
              </Reveal>

              <Reveal delay={0.12}>
                <p className="mt-6 max-w-[520px] text-[16px] leading-[1.6] text-pretty text-muted sm:text-[17px]">
                  Diagnóstico na hora, ajustes que se desfazem byte a byte e o antes e depois de cada mudança —
                  inclusive quando nada mudou.
                </p>
              </Reveal>

              <Reveal delay={0.18}>
                <div className="mt-9 flex flex-col gap-3 sm:flex-row sm:items-center">
                  <Magnet className="block sm:inline-block" innerClassName="[&>a]:w-full sm:[&>a]:w-auto">
                    <ButtonLink href={links.baixar} size="md">
                      <Download size={15} strokeWidth={2.25} aria-hidden="true" />
                      Baixar para Windows
                    </ButtonLink>
                  </Magnet>
                  <ButtonLink href="#preco" variant="secondary" size="md">
                    Ver preço
                    <ArrowRight size={15} strokeWidth={2} aria-hidden="true" />
                  </ButtonLink>
                </div>
                <ul className="mt-6 flex flex-wrap gap-x-5 gap-y-2 text-[12.5px] text-subtle">
                  {["Grátis para baixar", `Ativação R$ ${PRECO_BRL}, uma vez`, "Windows 10 e 11, 64 bits"].map((t) => (
                    <li key={t} className="flex items-center gap-1.5">
                      <Check size={13} strokeWidth={2.5} className="text-fg" aria-hidden="true" />
                      {t}
                    </li>
                  ))}
                </ul>
              </Reveal>
            </div>

            <Reveal delay={0.15} className="relative min-w-0">
              {/*
                A pilha cresce para cima e para a direita. No celular ela fica numa caixa de altura fixa e é desenhada
                por cima (absoluta, reduzida), para não alargar a coluna e empurrar o texto para fora da tela.
              */}
              <div className="relative h-[330px] sm:h-[400px] lg:h-[420px]">
                <div className="absolute top-[56%] left-1/2 origin-center -translate-x-[58%] -translate-y-1/2 scale-[0.74] sm:scale-100 lg:left-auto lg:right-12 lg:translate-x-0">
                  <HeroPilha />
                </div>
              </div>
            </Reveal>
          </div>
        </div>
      </div>
    </section>
  );
}
