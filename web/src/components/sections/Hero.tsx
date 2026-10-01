import { ArrowRight, Download } from "lucide-react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { Reveal } from "@/components/motion/Reveal";
import { BlurText } from "@/components/reactbits/BlurText";
import { Magnet } from "@/components/reactbits/Magnet";
import { ShinyText } from "@/components/reactbits/ShinyText";
import { Waves } from "@/components/reactbits/Waves";
import { ButtonLink } from "@/components/ui/Button";
import { links, PRECO_BRL, site } from "@/lib/site";

export function Hero() {
  return (
    <section aria-labelledby="hero-titulo">
      <div className="frame">
        <div className="frame-inner relative isolate overflow-hidden pt-16 pb-20 sm:pt-24 md:pb-24 lg:pt-28 lg:pb-28">
          {/*
            Linhas que respondem ao mouse, só do lado de fora do texto: a máscara apaga à esquerda (onde está a frase)
            e nas bordas de cima e de baixo, para o bloco não ganhar uma moldura.
          */}
          <Waves
            className="-z-10 [mask-image:linear-gradient(to_right,transparent_0%,transparent_30%,black_75%),linear-gradient(to_bottom,transparent,black_18%,black_82%,transparent)] [mask-composite:intersect] [-webkit-mask-composite:source-in]"
          />

          <Reveal>
            <p className="inline-flex items-center gap-2.5 rounded-full bg-white py-1 pr-3 pl-1 text-[12.5px] text-muted shadow-[0_0_0_1px_rgb(10_10_10/0.08),0_1px_2px_rgb(0_0_0/0.04)]">
              <span className="grid size-6 place-items-center rounded-full bg-ink text-white">
                <OtimizaLogo mark={14} className="text-white" />
              </span>
              <span>
                <ShinyText text={`Versão ${site.versao}`} className="font-semibold" /> · Windows 10 e 11
              </span>
            </p>
          </Reveal>

          <BlurText
            as="h1"
            id="hero-titulo"
            text="Seu PC medido, otimizado e provado com número."
            className="font-display mt-7 max-w-[620px] text-[26px] leading-[1.15] font-semibold tracking-[-0.04em] text-balance text-fg sm:text-[30px] lg:text-[34px]"
          />
          <Reveal delay={0.25}>
            <p className="mt-4 max-w-[560px] text-[15.5px] leading-[1.6] text-pretty text-muted sm:text-[16.5px]">
              Diagnóstico na hora, ajustes que se desfazem byte a byte e o antes e depois de cada mudança — inclusive
              quando nada mudou.
            </p>
          </Reveal>

          <Reveal delay={0.32}>
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
            <p className="mt-5 text-[12.5px] text-subtle">
              Grátis para baixar · ativação R$ {PRECO_BRL}, uma vez · Windows 10 e 11, 64 bits
            </p>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
