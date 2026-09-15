import { ArrowRight, ShieldCheck } from "lucide-react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { ButtonLink } from "@/components/ui/Button";
import { links } from "@/lib/site";

/*
 * O Otimiza não tem contas: a licença é conferida no próprio PC, e compra e
 * suporte acontecem no Discord. Por isso "entrar" é abrir o painel — que guarda
 * tudo neste navegador — e não um formulário de e-mail que não teria para onde
 * mandar nada.
 */
export function LoginForm() {
  return (
    <div className="mt-6">
      <ButtonLink href={links.painel} size="md" className="w-full">
        Abrir meu painel
        <ArrowRight size={15} strokeWidth={2} aria-hidden="true" />
      </ButtonLink>

      <div className="my-5 flex items-center gap-3 text-[11px] text-subtle" aria-hidden="true">
        <span className="h-px flex-1 bg-line" />
        ou
        <span className="h-px flex-1 bg-line" />
      </div>

      <ButtonLink href={links.discord} variant="secondary" size="md" className="w-full">
        <BrandLogo brand="discord" size={17} decorative />
        Comprar ou pedir ajuda no Discord
      </ButtonLink>

      <p className="mt-6 flex gap-2 rounded-[10px] bg-[#f4f4f3] p-3 text-[12px] leading-[1.5] text-muted">
        <ShieldCheck size={15} strokeWidth={2} className="mt-px shrink-0 text-fg" aria-hidden="true" />
        Não há conta nem senha. Suas chaves ficam guardadas só neste navegador e são conferidas com a mesma chave
        pública do programa.
      </p>
    </div>
  );
}
