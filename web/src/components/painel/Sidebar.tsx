"use client";

import { Download, House, KeyRound, LifeBuoy, MessageCircleQuestion, Settings, type LucideIcon } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { BrandLogo, type Brand } from "@/components/brand/BrandLogo";
import { cn } from "@/lib/cn";
import { links } from "@/lib/site";

/*
 * A NAVEGAÇÃO É CURTA PORQUE O PRODUTO É CURTO.
 *
 * A área do cliente do Otimiza tem quatro assuntos: a licença, o programa,
 * o suporte e o que este navegador guardou. Inventar "Clientes", "Afiliados"
 * ou "Crescimento" encheria a lateral de portas que não abrem.
 */
type Item = {
  href: string;
  label: string;
  icone?: LucideIcon;
  /** Marca real, quando o item É uma empresa — aí o logo é o dela. */
  marca?: Brand;
  externo?: boolean;
};

const GRUPOS: { titulo: string; itens: Item[] }[] = [
  {
    titulo: "Principal",
    itens: [
      { href: "/painel/", label: "Início", icone: House },
      { href: "/painel/licencas/", label: "Licenças", icone: KeyRound },
      { href: "/painel/downloads/", label: "Downloads", icone: Download },
    ],
  },
  {
    titulo: "Ajuda",
    itens: [
      { href: "/painel/suporte/", label: "Suporte", icone: LifeBuoy },
      { href: links.discord, label: "Discord", marca: "discord", externo: true },
      { href: "/#perguntas", label: "Perguntas", icone: MessageCircleQuestion },
    ],
  },
  {
    titulo: "Sistema",
    itens: [{ href: "/painel/conta/", label: "Preferências", icone: Settings }],
  },
];

export function Sidebar({ aoNavegar }: { aoNavegar?: () => void }) {
  const caminho = usePathname();
  const ativo = (href: string) => {
    if (!href.startsWith("/painel")) return false;
    const normal = caminho.endsWith("/") ? caminho : `${caminho}/`;
    return href === "/painel/" ? normal === "/painel/" : normal.startsWith(href);
  };

  return (
    <nav aria-label="Seções do painel" className="flex flex-col gap-4 px-2.5 py-3">
      {GRUPOS.map((grupo) => (
        <div key={grupo.titulo}>
          <p className="px-2 pb-1.5 text-[10px] font-medium tracking-[0.1em] text-subtle uppercase">{grupo.titulo}</p>
          <ul className="space-y-px">
            {grupo.itens.map((item) => (
              <li key={item.href}>
                <SidebarItem item={item} ativo={ativo(item.href)} aoNavegar={aoNavegar} />
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  );
}

function SidebarItem({ item, ativo, aoNavegar }: { item: Item; ativo: boolean; aoNavegar?: () => void }) {
  const conteudo = (
    <>
      <span className="grid size-[15px] shrink-0 place-items-center" aria-hidden="true">
        {item.marca ? <BrandLogo brand={item.marca} size={14} decorative /> : item.icone ? <item.icone size={15} strokeWidth={1.9} /> : null}
      </span>
      {item.label}
    </>
  );

  const classe = cn(
    "flex h-[31px] items-center gap-2.5 rounded-[6px] px-2 text-[12.5px] transition-colors duration-150",
    ativo ? "bg-[#f1f1f0] font-semibold text-fg" : "font-medium text-[#4a4a4a] hover:bg-[#f6f6f5] hover:text-fg",
  );

  if (item.externo) {
    return (
      <a href={item.href} target="_blank" rel="noopener noreferrer" className={classe} onClick={aoNavegar}>
        {conteudo}
      </a>
    );
  }

  return (
    <Link href={item.href} aria-current={ativo ? "page" : undefined} className={classe} onClick={aoNavegar}>
      {conteudo}
    </Link>
  );
}
