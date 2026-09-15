"use client";

import { ArrowLeft, Download, KeyRound, LayoutGrid, LifeBuoy, ShieldCheck } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { ButtonLink } from "@/components/ui/Button";
import { cn } from "@/lib/cn";
import { links, site } from "@/lib/site";

const ITENS = [
  { href: "/painel/", label: "Visão geral", icone: LayoutGrid },
  { href: "/painel/licencas/", label: "Licenças", icone: KeyRound },
  { href: "/painel/downloads/", label: "Downloads", icone: Download },
  { href: "/painel/suporte/", label: "Suporte", icone: LifeBuoy },
];

/**
 * A moldura do painel: lateral fixa no desktop, abas roláveis no celular.
 * `usePathname` devolve o caminho SEM o basePath, então as comparações abaixo
 * valem igual no computador e no GitHub Pages.
 */
export function PainelShell({ children }: { children: ReactNode }) {
  const caminho = usePathname();
  const ativo = (href: string) => {
    const normal = caminho.endsWith("/") ? caminho : `${caminho}/`;
    return href === "/painel/" ? normal === "/painel/" : normal.startsWith(href);
  };

  return (
    <div className="min-h-dvh bg-[#f4f4f3] lg:grid lg:grid-cols-[248px_1fr]">

      {/* Lateral (desktop) */}
      <aside className="sticky top-0 hidden h-dvh flex-col border-r border-line bg-white px-4 py-5 lg:flex">
        <Link href="/" aria-label="Otimiza — página inicial" className="rounded-md px-2 py-1">
          <OtimizaLogo mark={26} wordmark />
        </Link>

        <p className="eyebrow mt-9 px-2 text-[10px]">Painel</p>
        <nav aria-label="Painel" className="mt-3">
          <ul className="space-y-1">
            {ITENS.map(({ href, label, icone: Icone }) => (
              <li key={href}>
                <Link
                  href={href}
                  aria-current={ativo(href) ? "page" : undefined}
                  className={cn(
                    "flex h-9 items-center gap-3 rounded-[8px] px-2.5 text-[13.5px] font-medium transition-colors",
                    ativo(href)
                      ? "bg-ink text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.14)]"
                      : "text-[#3a3a3a] hover:bg-[#f2f2f1] hover:text-fg",
                  )}
                >
                  <Icone size={16} strokeWidth={2} aria-hidden="true" />
                  {label}
                </Link>
              </li>
            ))}
          </ul>
        </nav>

        <div className="mt-auto space-y-3">
          <div className="card p-4">
            <p className="flex items-center gap-2 text-[12.5px] font-semibold">
              <ShieldCheck size={15} strokeWidth={2} aria-hidden="true" />
              Sem conta, sem servidor
            </p>
            <p className="mt-1.5 text-[12px] leading-[1.5] text-muted">
              O que você guarda aqui fica só neste navegador.
            </p>
          </div>
          <Link
            href="/"
            className="flex h-9 items-center gap-2 rounded-[8px] px-2.5 text-[13px] text-muted transition-colors hover:text-fg"
          >
            <ArrowLeft size={15} strokeWidth={2} aria-hidden="true" />
            Voltar para o site
          </Link>
        </div>
      </aside>

      <div className="min-w-0">
        {/* Topo */}
        <header className="sticky top-0 z-40 border-b border-line bg-white/85 backdrop-blur-md">
          <div className="flex h-14 items-center justify-between gap-3 px-5 lg:h-16 lg:px-10">
            <Link href="/" aria-label="Otimiza — página inicial" className="inline-flex min-h-11 items-center rounded-md lg:hidden">
              <OtimizaLogo mark={24} wordmark />
            </Link>
            <p className="hidden text-[13px] text-muted lg:block">
              Otimiza <span className="text-subtle">/</span>{" "}
              <span className="font-medium text-fg">{ITENS.find((i) => ativo(i.href))?.label ?? "Painel"}</span>
            </p>
            <ButtonLink href={links.baixar}>
              <Download size={14} strokeWidth={2.25} aria-hidden="true" />
              <span className="hidden sm:inline">Baixar a versão {site.versao}</span>
              <span className="sm:hidden">Baixar</span>
            </ButtonLink>
          </div>

          {/* Abas (celular e tablet) */}
          <nav aria-label="Painel" className="border-t border-line lg:hidden">
            <ul className="flex gap-1 overflow-x-auto px-3 py-2 [scrollbar-width:none]">
              {ITENS.map(({ href, label, icone: Icone }) => (
                <li key={href} className="shrink-0">
                  <Link
                    href={href}
                    aria-current={ativo(href) ? "page" : undefined}
                    className={cn(
                      "flex h-10 items-center gap-2 rounded-[8px] px-3 text-[13px] font-medium",
                      ativo(href) ? "bg-ink text-white" : "text-[#3a3a3a]",
                    )}
                  >
                    <Icone size={15} strokeWidth={2} aria-hidden="true" />
                    {label}
                  </Link>
                </li>
              ))}
            </ul>
          </nav>
        </header>

        <main id="conteudo" className="mx-auto w-full max-w-[1080px] px-5 py-8 lg:px-10 lg:py-10">
          {children}
        </main>
      </div>
    </div>
  );
}

/** Cabeçalho de página do painel. */
export function PainelTitulo({ titulo, texto, acao }: { titulo: string; texto?: string; acao?: ReactNode }) {
  return (
    <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
      <div>
        <h1 className="font-display text-[28px] leading-[1.1] font-semibold tracking-[-0.045em] sm:text-[32px]">
          {titulo}
        </h1>
        {texto && <p className="mt-2 max-w-[620px] text-[14.5px] leading-[1.6] text-muted">{texto}</p>}
      </div>
      {acao}
    </div>
  );
}

/**
 * Um bloco do painel. A cor de fundo é variante, e não classe passada por fora:
 * `cn` só junta classes, e `bg-white` com `bg-[#0b0b0b]` no mesmo elemento deixa
 * a decisão para a ordem do CSS — foi assim que o bloco do instalador saiu
 * branco com texto branco na primeira versão.
 */
export function Bloco({
  children,
  className,
  escuro = false,
}: {
  children: ReactNode;
  className?: string;
  escuro?: boolean;
}) {
  return (
    <section
      className={cn(
        "rounded-[16px]",
        escuro
          ? "grain relative overflow-hidden bg-[#0b0b0b] text-white shadow-[0_0_0_1px_#000,0_20px_40px_-24px_rgb(0_0_0/0.5)]"
          : "bg-white shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_1px_2px_rgb(0_0_0/0.04)]",
        className,
      )}
    >
      {children}
    </section>
  );
}
