import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { SmartLink } from "@/components/ui/SmartLink";
import { links } from "@/lib/site";

const COLUNAS = [
  {
    titulo: "Conheça o Otimiza",
    itens: [
      { label: "Recursos", href: "#recursos" },
      { label: "As telas", href: "#programa" },
      { label: "Preço", href: "#preco" },
    ],
  },
  {
    titulo: "Sua licença",
    itens: [
      { label: "Entrar", href: links.entrar },
      { label: "Como funciona", href: "#licenca" },
      { label: "Baixar", href: links.baixar },
    ],
  },
  {
    titulo: "Ajuda",
    itens: [
      { label: "Perguntas", href: "#perguntas" },
      { label: "Discord", href: links.discord },
    ],
  },
];

export function Footer() {
  return (
    <footer className="border-t border-line">
      <div className="frame">
        <div className="frame-inner py-14 md:py-16">
          <div className="grid gap-12 md:grid-cols-[1.3fr_2fr]">
            <div>
              <OtimizaLogo mark={30} />
              <p className="font-display mt-5 text-[22px] leading-[1.2] font-semibold tracking-[-0.035em]">
                Mede. Otimiza.
                <br />
                Prova com número.
              </p>
              <p className="mt-3 max-w-[340px] text-[13.5px] leading-[1.6] text-muted">
                Console de desempenho para Windows 10 e 11.
              </p>
            </div>

            <nav aria-label="Rodapé" className="grid grid-cols-2 gap-8 sm:grid-cols-3">
              {COLUNAS.map((c) => (
                <div key={c.titulo}>
                  <p className="text-[13px] font-semibold text-fg">{c.titulo}</p>
                  <ul className="mt-3 lg:mt-4 lg:space-y-2">
                    {c.itens.map((i) => (
                      <li key={i.label}>
                        <SmartLink
                          href={i.href}
                          className="inline-flex min-h-10 items-center text-[13.5px] text-muted transition-colors hover:text-fg lg:min-h-7"
                        >
                          {i.label}
                        </SmartLink>
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </nav>
          </div>

          <div className="mt-14 flex flex-col gap-2 border-t border-line pt-6 text-[12px] text-subtle sm:flex-row sm:justify-between">
            <p>© 2026 Otimiza. Código aberto à leitura, não ao uso.</p>
            <p>Feito para Windows 10 e 11.</p>
          </div>
        </div>
      </div>
    </footer>
  );
}
