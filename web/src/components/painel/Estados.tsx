"use client";

import { Check, Circle, type LucideIcon } from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/**
 * O estado vazio: o que aparece quando a conta ainda não tem aquilo.
 *
 * É o oposto de encher a tela com número de mentira. Quem acabou de comprar
 * não faturou nada, não tem histórico e não deve ver um painel simulando que
 * tem — deve ver o que falta fazer.
 */
export function EstadoVazio({
  icone: Icone,
  titulo,
  texto,
  acao,
  compacto = false,
}: {
  icone: LucideIcon;
  titulo: string;
  texto: string;
  acao?: ReactNode;
  compacto?: boolean;
}) {
  return (
    <div className={cn("grid place-items-center px-5 text-center", compacto ? "py-7" : "py-12")}>
      <span className="grid size-9 place-items-center rounded-[9px] bg-[#f4f4f3]">
        <Icone size={16} strokeWidth={2} aria-hidden="true" />
      </span>
      <p className="mt-3 text-[13px] font-semibold">{titulo}</p>
      <p className="mt-1 max-w-[320px] text-[12px] leading-[1.55] text-muted">{texto}</p>
      {acao && <div className="mt-4">{acao}</div>}
    </div>
  );
}

export type ItemDaLista = { id: string; titulo: string; detalhe?: string; feito: boolean; href?: string };

/**
 * A lista do começo: some sozinha quando tudo estiver feito.
 *
 * Nenhum item nasce marcado. Marcar uma etapa que a pessoa não fez para a
 * barra parecer adiantada é a primeira mentira de um produto.
 */
export function Checklist({ titulo, itens }: { titulo: string; itens: ItemDaLista[] }) {
  const feitos = itens.filter((i) => i.feito).length;
  if (feitos === itens.length) return null;

  return (
    <div className="rounded-[8px] bg-white shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_1px_2px_rgb(10_10_10/0.04)]">
      <div className="flex items-center justify-between gap-3 border-b border-line px-3.5 py-2.5">
        <p className="text-[12.5px] font-semibold">{titulo}</p>
        <p className="tabular text-[11.5px] text-muted">
          {feitos} de {itens.length} concluídos
        </p>
      </div>

      <ul className="divide-y divide-line">
        {itens.map((item) => {
          const conteudo = (
            <>
              <span
                className={cn(
                  "mt-px grid size-[18px] shrink-0 place-items-center rounded-full",
                  item.feito ? "bg-ink text-white" : "text-subtle ring-1 ring-inset ring-[rgb(10_10_10/0.16)]",
                )}
                aria-hidden="true"
              >
                {item.feito ? <Check size={11} strokeWidth={3} /> : <Circle size={8} strokeWidth={0} className="opacity-0" />}
              </span>
              <span className="min-w-0">
                <span className={cn("block text-[12.5px]", item.feito ? "text-muted line-through decoration-black/20" : "font-medium")}>
                  {item.titulo}
                </span>
                {item.detalhe && !item.feito && <span className="mt-0.5 block text-[11.5px] text-subtle">{item.detalhe}</span>}
              </span>
            </>
          );

          return (
            <li key={item.id}>
              {item.href && !item.feito ? (
                <Link href={item.href} className="flex items-start gap-2.5 px-3.5 py-2.5 transition-colors hover:bg-[#f9f9f8]">
                  {conteudo}
                </Link>
              ) : (
                <div className="flex items-start gap-2.5 px-3.5 py-2.5">{conteudo}</div>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
