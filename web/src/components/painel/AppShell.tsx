"use client";

import { X } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { TelaDeEspera } from "@/app/boas-vindas/BoasVindas";
import { Busca } from "@/components/painel/Busca";
import { Sidebar } from "@/components/painel/Sidebar";
import { TopBar } from "@/components/painel/TopBar";
import { cn } from "@/lib/cn";
import { useGuarda } from "@/lib/sessao";

/**
 * A moldura do produto: barra preta de 34px em cima, lateral clara de 212px à
 * esquerda, e o resto é conteúdo sobre cinza claro.
 *
 * Nada de coluna centralizada com largura máxima: isto é software, e software
 * usa a tela que tem. A margem externa é de 14px justamente para não sobrar
 * moldura em volta do trabalho.
 */
export function AppShell({ children }: { children: ReactNode }) {
  const sessao = useGuarda(["/painel/"]);
  const [menu, setMenu] = useState(false);
  const [busca, setBusca] = useState(false);

  // Ctrl+K / ⌘K abre a busca de qualquer lugar do painel.
  useEffect(() => {
    const aoTeclar = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setBusca((v) => !v);
      }
    };
    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, []);

  if (sessao.carregando) return <TelaDeEspera />;

  return (
    <div className="flex min-h-dvh flex-col bg-[#f6f6f6]">
      <TopBar aoAbrirMenu={() => setMenu(true)} aoAbrirBusca={() => setBusca(true)} />

      <div className="flex min-h-0 flex-1">
        {/* Lateral fixa no desktop */}
        <aside className="sticky top-[34px] hidden h-[calc(100dvh-34px)] w-[212px] shrink-0 overflow-y-auto border-r border-line bg-white lg:block">
          <Sidebar />
        </aside>

        {/* Gaveta no celular */}
        {menu && (
          <div className="fixed inset-0 z-50 lg:hidden" onClick={() => setMenu(false)}>
            <div className="absolute inset-0 bg-[rgb(10_10_10/0.35)]" />
            <div
              className="absolute inset-y-0 left-0 w-[232px] animate-[deslizar_160ms_ease-out] bg-white"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex h-[34px] items-center justify-between px-3">
                <span className="text-[11px] font-medium tracking-[0.1em] text-subtle uppercase">Painel</span>
                <button
                  type="button"
                  onClick={() => setMenu(false)}
                  aria-label="Fechar as seções"
                  className="grid size-7 place-items-center rounded-[5px] text-muted hover:bg-[#f4f4f3]"
                >
                  <X size={15} strokeWidth={2} aria-hidden="true" />
                </button>
              </div>
              <Sidebar aoNavegar={() => setMenu(false)} />
            </div>
          </div>
        )}

        <main id="conteudo" className="min-w-0 flex-1 p-3.5 lg:p-5">
          {children}
        </main>
      </div>

      {/* A `key` remonta a caixa a cada abertura: o campo volta vazio sem
          precisar de um efeito escrevendo estado. */}
      <Busca key={busca ? "aberta" : "fechada"} aberta={busca} aoFechar={() => setBusca(false)} />
    </div>
  );
}

/** O cabeçalho de uma página do painel: título à esquerda, controles à direita. */
export function CabecalhoPagina({
  icone,
  titulo,
  texto,
  acoes,
}: {
  icone?: ReactNode;
  titulo: string;
  texto?: string;
  acoes?: ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 px-0.5 pb-3.5">
      <div className="flex items-center gap-2.5">
        {icone && <span className="grid size-[26px] place-items-center rounded-[6px] bg-white text-fg shadow-[0_0_0_1px_rgb(10_10_10/0.08)]">{icone}</span>}
        <div>
          <h1 className="font-display text-[17px] leading-tight font-semibold tracking-[-0.03em]">{titulo}</h1>
          {texto && <p className="mt-0.5 text-[12px] text-muted">{texto}</p>}
        </div>
      </div>
      {acoes && <div className="flex items-center gap-1.5">{acoes}</div>}
    </div>
  );
}

/** Um cartão do painel. Borda de 1px, canto de 8px, sombra quase nenhuma. */
export function Cartao({
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
        "rounded-[8px]",
        escuro
          ? "bg-[#050505] text-white"
          : "bg-white shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_1px_2px_rgb(10_10_10/0.04)]",
        className,
      )}
    >
      {children}
    </section>
  );
}
