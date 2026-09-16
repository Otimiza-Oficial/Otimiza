"use client";

import { ChevronDown, LifeBuoy, LogOut, Menu, Search, Sparkles } from "lucide-react";
import Link from "next/link";
import { useEffect, useRef, useState } from "react";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { limparTudo } from "@/lib/armazem";
import { cn } from "@/lib/cn";
import { useUltimaRelease } from "@/lib/release";
import { useLicencaAtiva } from "@/lib/sessao";
import { links } from "@/lib/site";

/**
 * A barra preta do topo: 34px, e é ela que separa o produto do site.
 *
 * Ela carrega pouca coisa de propósito — marca, busca e conta. Uma barra de
 * ferramentas cheia de ícones parece poderosa na captura de tela e vira um
 * campo minado no uso diário.
 */
export function TopBar({ aoAbrirMenu, aoAbrirBusca }: { aoAbrirMenu: () => void; aoAbrirBusca: () => void }) {
  const { licenca } = useLicencaAtiva();
  const release = useUltimaRelease();
  const [conta, setConta] = useState(false);
  const caixa = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!conta) return;
    const fora = (e: MouseEvent) => {
      if (caixa.current && !caixa.current.contains(e.target as Node)) setConta(false);
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setConta(false);
    document.addEventListener("mousedown", fora);
    document.addEventListener("keydown", esc);
    return () => {
      document.removeEventListener("mousedown", fora);
      document.removeEventListener("keydown", esc);
    };
  }, [conta]);

  const nome = licenca?.dados.comprador?.trim() || "Minha licença";
  const versao = release.estado === "ok" ? release.release.versao : null;

  return (
    <header className="sticky top-0 z-50 flex h-[34px] items-center gap-3 bg-[#050505] px-2.5 text-white">
      <button
        type="button"
        onClick={aoAbrirMenu}
        aria-label="Abrir as seções"
        className="grid size-[26px] shrink-0 place-items-center rounded-[5px] text-white/70 hover:bg-white/10 hover:text-white lg:hidden"
      >
        <Menu size={15} strokeWidth={2} aria-hidden="true" />
      </button>

      <Link href="/painel/" aria-label="Otimiza — início do painel" className="shrink-0 rounded-[5px] px-1 text-white">
        <OtimizaLogo mark={17} />
      </Link>

      {/* A busca ocupa o centro, como na referência, e abre a paleta de comandos. */}
      <button
        type="button"
        onClick={aoAbrirBusca}
        className="mx-auto hidden h-[24px] w-full max-w-[320px] items-center gap-2 rounded-[6px] bg-white/[0.07] px-2.5 text-[11.5px] text-white/45 ring-1 ring-white/10 transition-colors hover:bg-white/[0.12] hover:text-white/70 sm:flex"
      >
        <Search size={12} strokeWidth={2} aria-hidden="true" />
        Pesquisar na Otimiza
        <kbd className="ml-auto rounded-[4px] bg-white/10 px-1.5 py-px font-mono text-[9.5px] text-white/50">Ctrl K</kbd>
      </button>

      <div className="ml-auto flex shrink-0 items-center gap-1 sm:ml-0">
        {versao && (
          <span className="hidden items-center gap-1.5 rounded-[5px] px-2 py-1 text-[11px] text-white/55 md:inline-flex">
            <Sparkles size={12} strokeWidth={2} aria-hidden="true" />
            Versão {versao}
          </span>
        )}

        <a
          href={links.discord}
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Suporte no Discord"
          className="grid size-[26px] place-items-center rounded-[5px] text-white/70 transition-colors hover:bg-white/10 hover:text-white"
        >
          <LifeBuoy size={14} strokeWidth={2} aria-hidden="true" />
        </a>

        <div className="relative" ref={caixa}>
          <button
            type="button"
            onClick={() => setConta((v) => !v)}
            aria-expanded={conta}
            aria-haspopup="menu"
            className="flex h-[26px] items-center gap-1.5 rounded-[5px] pr-1 pl-1.5 text-[11.5px] text-white/80 transition-colors hover:bg-white/10 hover:text-white"
          >
            <span className="grid size-[18px] place-items-center rounded-full bg-white/15 font-mono text-[9.5px] font-semibold">
              {nome.slice(0, 1).toUpperCase()}
            </span>
            <span className="hidden max-w-[130px] truncate sm:inline">{nome}</span>
            <ChevronDown size={12} strokeWidth={2} aria-hidden="true" className={cn("transition-transform", conta && "rotate-180")} />
          </button>

          {conta && (
            <div
              role="menu"
              className="absolute right-0 top-[30px] w-[210px] origin-top-right animate-[surgir_140ms_ease-out] rounded-[8px] bg-white p-1 text-fg shadow-[0_0_0_1px_rgb(10_10_10/0.1),0_10px_24px_-12px_rgb(10_10_10/0.35)]"
            >
              <p className="px-2.5 py-2 text-[11px] text-subtle">
                {licenca ? `Licença ${licenca.dados.expira ? "válida" : "vitalícia"}` : "Nenhuma licença conferida"}
              </p>
              <Link
                href="/painel/conta/"
                role="menuitem"
                onClick={() => setConta(false)}
                className="flex h-8 items-center rounded-[6px] px-2.5 text-[12.5px] hover:bg-[#f4f4f3]"
              >
                Preferências
              </Link>
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  limparTudo();
                  setConta(false);
                }}
                className="flex h-8 w-full items-center gap-2 rounded-[6px] px-2.5 text-left text-[12.5px] hover:bg-[#f4f4f3]"
              >
                <LogOut size={13} strokeWidth={2} aria-hidden="true" />
                Sair deste navegador
              </button>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
