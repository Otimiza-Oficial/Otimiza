import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/**
 * O cartão de número: 86px de altura, rótulo de 11px, valor de 20px.
 *
 * A tentação num painel é fazer o número gigante. Gigante serve para um número
 * só; aqui são quatro lado a lado, e o que importa é poder ler os quatro de
 * relance — por isso a altura é baixa e o valor é comedido.
 */
export function CartaoMetrica({
  rotulo,
  valor,
  nota,
  etiqueta,
  extra,
  mono = false,
}: {
  rotulo: string;
  valor: ReactNode;
  nota?: string;
  /** Selo curto à direita do rótulo — modalidade, estado, canal. */
  etiqueta?: ReactNode;
  /** Desenho pequeno no canto inferior direito. */
  extra?: ReactNode;
  mono?: boolean;
}) {
  return (
    <article className="relative flex h-[86px] flex-col justify-between overflow-hidden rounded-[8px] bg-white px-3.5 py-3 shadow-[0_0_0_1px_rgb(10_10_10/0.07),0_1px_2px_rgb(10_10_10/0.04)]">
      <div className="flex items-start justify-between gap-2">
        <p className="text-[11.5px] font-medium text-muted">{rotulo}</p>
        {etiqueta}
      </div>

      <div className="flex items-end justify-between gap-2">
        <div className="min-w-0">
          <p
            className={cn(
              "tabular truncate font-semibold tracking-[-0.02em]",
              mono ? "font-mono text-[14px]" : "font-display text-[20px] leading-none",
            )}
          >
            {valor}
          </p>
          {nota && <p className="mt-1 truncate text-[11px] text-subtle">{nota}</p>}
        </div>
        {extra && <div className="shrink-0 pb-0.5">{extra}</div>}
      </div>
    </article>
  );
}

/** Selo de uma palavra, do tamanho de um rótulo. */
export function Selo({ children, tom = "claro" }: { children: ReactNode; tom?: "claro" | "escuro" }) {
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-[5px] px-1.5 py-0.5 text-[10px] font-semibold",
        tom === "escuro" ? "bg-ink text-white" : "bg-[#f1f1f0] text-muted",
      )}
    >
      {children}
    </span>
  );
}
