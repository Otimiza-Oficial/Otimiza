import { ArrowRight, Check, Download } from "lucide-react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { Reveal } from "@/components/motion/Reveal";
import { ButtonLink } from "@/components/ui/Button";
import { Section, SectionHeading } from "@/components/ui/Section";
import { cn } from "@/lib/cn";
import { formatarReais, links, PRECO_ADICIONAL_BRL, PRECO_BRL } from "@/lib/site";

/*
 * O Otimiza não tem plano mensal nem anual — é um produto, um preço. Por isso
 * não há seletor de período aqui: um seletor sem nada para alternar seria
 * fingir uma estrutura comercial que não existe.
 */
const OPCOES = [
  {
    id: "gratis",
    rotulo: "Para conhecer",
    nome: "Download",
    resumo: "Instale e veja o que está travando o seu PC antes de decidir qualquer coisa.",
    preco: 0,
    nota: "Sem cadastro",
    cta: { texto: "Baixar grátis", href: links.baixar, icone: "download" as const },
    inclui: [
      "Diagnóstico completo antes de pagar",
      "O principal problema do seu PC, medido na hora",
      "O código da máquina para ativar depois",
    ],
  },
  {
    id: "vitalicia",
    rotulo: "Para 1 computador",
    nome: "Licença vitalícia",
    resumo: "O programa completo. Comprou, é seu — sem assinatura, sem renovação.",
    preco: PRECO_BRL,
    nota: "Pagamento único",
    destaque: true,
    cta: { texto: "Comprar", href: links.comprar, icone: "seta" as const },
    inclui: [
      "Todas as otimizações, reversíveis byte a byte",
      "Prova de resultado antes e depois",
      "Formatar o Windows não invalida a chave",
      "Reemissão gratuita ao trocar a placa-mãe",
      "Verificação na sua máquina, sem servidor",
    ],
  },
  {
    id: "adicional",
    rotulo: "Para outro PC",
    nome: "Licença adicional",
    resumo: "O do trabalho, o do filho: cada computador tem a sua própria chave.",
    preco: PRECO_ADICIONAL_BRL,
    nota: "Por computador",
    cta: { texto: "Comprar outra", href: links.comprar, icone: "seta" as const },
    inclui: ["Também vitalícia", "Presa à placa-mãe daquele PC", "Não é obrigatória para quem tem um PC"],
  },
];

export function Pricing() {
  return (
    <Section id="preco" labelledBy="preco-titulo">
      <Reveal>
        <SectionHeading id="preco-titulo" size="xl" align="center" title="Um preço." lead="Para sempre." />
        <p className="mx-auto mt-6 max-w-[440px] text-center text-[15px] leading-[1.6] text-muted">
          Não há plano mensal, plano anual, nem versão “Pro” com o mesmo programa e um botão a mais.
        </p>
      </Reveal>

      <ul className="mt-14 grid items-stretch gap-4 lg:mt-16 lg:grid-cols-3">
        {OPCOES.map((o, i) => (
          <li key={o.id} className="flex">
            <Reveal delay={i * 0.05} className="flex w-full">
              <article
                aria-label={o.nome}
                className={cn(
                  "flex w-full flex-col rounded-[var(--radius-card)] p-6 sm:p-7",
                  o.destaque
                    ? "grain relative overflow-hidden bg-[#0b0b0b] text-white shadow-[0_0_0_1px_#000,0_30px_60px_-30px_rgb(0_0_0/0.6)]"
                    : "bg-white shadow-[0_0_0_1px_rgb(10_10_10/0.08),0_1px_2px_rgb(0_0_0/0.04)]",
                )}
              >
                <div className="flex items-center justify-between">
                  <p className={cn("eyebrow", o.destaque && "text-[#a3a3a3]")}>{o.rotulo}</p>
                  {o.destaque && (
                    <span className="rounded-md bg-white px-2 py-1 text-[10.5px] font-semibold text-fg">
                      O programa completo
                    </span>
                  )}
                </div>

                <h3 className="font-display mt-6 text-[24px] font-semibold tracking-[-0.04em]">{o.nome}</h3>
                <p className={cn("mt-2 text-[14px] leading-[1.55]", o.destaque ? "text-[#b5b5b5]" : "text-muted")}>
                  {o.resumo}
                </p>

                <p className="mt-8 flex items-baseline gap-2">
                  <span className="font-display tabular text-[44px] leading-none font-semibold tracking-[-0.05em]">
                    {o.preco === 0 ? "Grátis" : formatarReais(o.preco)}
                  </span>
                </p>
                <p className={cn("mt-2 text-[12.5px]", o.destaque ? "text-[#a3a3a3]" : "text-subtle")}>{o.nota}</p>

                <ButtonLink
                  href={o.cta.href}
                  variant={o.destaque ? "light" : "secondary"}
                  size="md"
                  className="mt-7 w-full justify-between"
                >
                  {o.cta.texto}
                  {o.cta.icone === "download" ? (
                    <Download size={15} strokeWidth={2.25} aria-hidden="true" />
                  ) : (
                    <ArrowRight size={15} strokeWidth={2} aria-hidden="true" />
                  )}
                </ButtonLink>

                <div className={cn("mt-7 border-t pt-6", o.destaque ? "border-white/10" : "border-line")}>
                  <p className="text-[12.5px] font-semibold">O que está incluído</p>
                  <ul className="mt-4 space-y-3">
                    {o.inclui.map((item) => (
                      <li key={item} className="flex items-start gap-2.5 text-[13.5px] leading-[1.45]">
                        <Check size={15} strokeWidth={2.5} className="mt-0.5 shrink-0" aria-hidden="true" />
                        <span className={o.destaque ? "text-[#e5e5e5]" : "text-[#2b2b2b]"}>{item}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              </article>
            </Reveal>
          </li>
        ))}
      </ul>

      <Reveal delay={0.1}>
        <div className="mt-8 flex flex-col items-center gap-3 text-center text-[13px] text-muted sm:flex-row sm:justify-center sm:gap-5">
          <span className="flex items-center gap-2 text-fg">
            <BrandLogo brand="pix" size={15} decorative />
            Pix
          </span>
          <span className="hidden text-line-strong sm:inline">·</span>
          <span>Cartão de crédito</span>
          <span className="hidden text-line-strong sm:inline">·</span>
          <span>Boleto</span>
          <span className="hidden text-line-strong sm:inline">·</span>
          <span className="flex items-center gap-2">
            Processado pelo
            <span className="flex items-center gap-1.5 font-medium text-fg">
              <BrandLogo brand="mercadopago" size={16} decorative />
              Mercado Pago
            </span>
          </span>
        </div>
        <p className="mt-3 text-center text-[12px] text-subtle">Windows 10 ou 11, 64 bits.</p>
      </Reveal>
    </Section>
  );
}
