import { ArrowRight, Download } from "lucide-react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { SmartLink } from "@/components/ui/SmartLink";
import { links } from "@/lib/site";

/**
 * A faixa preta no alto do painel, em três partes: o que fazer agora, a marca
 * no meio e onde pedir ajuda.
 *
 * As marcas ao redor do símbolo são as que o Otimiza usa de verdade — Discord
 * (compra e suporte), Pix e Mercado Pago (pagamento). Nenhuma outra entra aqui
 * só para encher a composição.
 */
export function Banner() {
  return (
    <section className="grid min-h-[112px] overflow-hidden rounded-[10px] bg-[#050505] text-white lg:grid-cols-[1fr_auto_1fr]">
      {/* esquerda */}
      <div className="flex flex-col justify-center gap-2 px-5 py-4">
        <p className="text-[11.5px] text-white/55">Comece por aqui</p>
        <p className="font-display text-[19px] leading-[1.1] font-semibold tracking-[-0.035em]">
          Instale e meça
          <br />o seu PC
        </p>
        <SmartLink
          href={links.baixar}
          className="mt-1 inline-flex h-[26px] w-fit items-center gap-1.5 rounded-[6px] bg-white px-2.5 text-[11.5px] font-semibold text-fg transition-colors hover:bg-[#ececec]"
        >
          <Download size={12} strokeWidth={2.5} aria-hidden="true" />
          Baixar o instalador
        </SmartLink>
      </div>

      {/* centro: a marca, ladeada pelas integrações reais */}
      <div className="relative hidden items-center justify-center gap-4 px-8 lg:flex">
        <span
          aria-hidden="true"
          className="absolute inset-0 [background-image:radial-gradient(ellipse_60%_80%_at_50%_50%,rgba(255,255,255,.09),transparent_70%)]"
        />
        <MarcaNoBanner>
          <BrandLogo brand="discord" size={17} decorative />
        </MarcaNoBanner>
        <span className="relative grid size-[54px] place-items-center rounded-[14px] bg-white text-fg">
          <OtimizaLogo mark={30} />
        </span>
        <MarcaNoBanner>
          <BrandLogo brand="pix" size={17} decorative />
        </MarcaNoBanner>
        <MarcaNoBanner>
          <BrandLogo brand="mercadopago" size={18} decorative />
        </MarcaNoBanner>
      </div>

      {/* direita */}
      <div className="flex flex-col justify-center gap-2 border-t border-white/10 px-5 py-4 lg:items-end lg:border-t-0 lg:border-l lg:text-right">
        <p className="text-[11.5px] text-white/55">Quando precisar</p>
        <p className="font-display text-[19px] leading-[1.1] font-semibold tracking-[-0.035em]">
          Suporte com
          <br />
          gente de verdade
        </p>
        <SmartLink
          href={links.discord}
          className="mt-1 inline-flex h-[26px] w-fit items-center gap-1.5 rounded-[6px] px-2.5 text-[11.5px] font-semibold text-white ring-1 ring-white/20 transition-colors hover:bg-white/10"
        >
          Abrir o Discord
          <ArrowRight size={12} strokeWidth={2.5} aria-hidden="true" />
        </SmartLink>
      </div>
    </section>
  );
}

function MarcaNoBanner({ children }: { children: React.ReactNode }) {
  return (
    <span className="relative grid size-[38px] place-items-center rounded-[10px] bg-white/[0.07] text-white ring-1 ring-white/10">
      {children}
    </span>
  );
}
