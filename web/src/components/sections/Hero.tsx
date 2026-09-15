import { ArrowRight, Download } from "lucide-react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { Reveal } from "@/components/motion/Reveal";
import { ButtonLink } from "@/components/ui/Button";
import { links, PRECO_BRL, site } from "@/lib/site";

export function Hero() {
  return (
    <section aria-labelledby="hero-titulo">
      <div className="frame">
        <div className="frame-inner pt-16 pb-20 sm:pt-24 md:pb-24 lg:pt-28 lg:pb-28">
          <Reveal>
            <p className="inline-flex items-center gap-2.5 rounded-full bg-white py-1 pr-3 pl-1 text-[12.5px] text-muted shadow-[0_0_0_1px_rgb(10_10_10/0.08),0_1px_2px_rgb(0_0_0/0.04)]">
              <span className="grid size-6 place-items-center rounded-full bg-ink text-white">
                <OtimizaLogo mark={14} className="text-white" />
              </span>
              <span>
                <span className="font-semibold text-fg">Versão {site.versao}</span> · Windows 10 e 11
              </span>
            </p>
          </Reveal>

          <Reveal delay={0.04}>
            <h1
              id="hero-titulo"
              className="font-display mt-7 max-w-[980px] text-[34px] leading-[1.1] font-semibold tracking-[-0.05em] text-balance text-fg sm:text-[44px] lg:text-[54px] lg:leading-[1.08]"
            >
              Seu PC medido, otimizado e provado com número.{" "}
              <span className="text-muted">
                Diagnóstico na hora, ajustes que se desfazem byte a byte e o antes e depois de cada mudança —
                inclusive quando nada mudou.
              </span>
            </h1>
          </Reveal>

          <Reveal delay={0.1}>
            <div className="mt-9 flex flex-col gap-3 sm:flex-row sm:items-center">
              <ButtonLink href={links.baixar} size="md">
                <Download size={15} strokeWidth={2.25} aria-hidden="true" />
                Baixar para Windows
              </ButtonLink>
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
