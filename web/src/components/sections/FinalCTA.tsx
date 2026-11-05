import { Download } from "lucide-react";
import { Mascote } from "@/components/brand/Mascote";
import { Reveal } from "@/components/motion/Reveal";
import { ButtonLink } from "@/components/ui/Button";
import { links, VERSAO_EXIBIDA } from "@/lib/site";

/**
 * O fechamento: o mascote, uma frase e um botão. Depois de tanta explicação, nada aqui compete com a única ação.
 */
export function FinalCTA() {
  return (
    <section aria-labelledby="cta-titulo">
      <div className="frame">
        <div className="frame-inner pb-20 md:pb-24">
          <Reveal>
            <div className="card brilho-borda px-6 py-14 text-center sm:py-20">
              <div className="flex justify-center">
                <Mascote tamanho={150} className="h-auto w-[120px] sm:w-[150px]" />
              </div>
              <h2
                id="cta-titulo"
                className="font-display mx-auto mt-6 max-w-[720px] text-[34px] leading-[1.06] font-semibold tracking-[-0.05em] text-balance sm:text-[48px]"
              >
                Meça o seu PC antes de pagar qualquer coisa.
              </h2>
              <p className="mx-auto mt-4 max-w-[480px] text-[15px] leading-[1.6] text-muted">
                Baixar e ver o que está travando é grátis. A chave só libera as otimizações que escrevem no Windows.
              </p>
              <div className="mt-8 flex justify-center">
                <ButtonLink href={links.baixar} size="md">
                  <Download size={16} strokeWidth={2.25} aria-hidden="true" />
                  Baixar o Otimiza {VERSAO_EXIBIDA}
                </ButtonLink>
              </div>
            </div>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
