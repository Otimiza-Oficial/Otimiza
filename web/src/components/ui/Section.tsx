import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/**
 * Uma seção inteira: fio horizontal no topo, coluna emoldurada e o respiro
 * vertical do sistema. É isso que mantém gutters e ritmo iguais do hero ao
 * rodapé.
 */
export function Section({
  id,
  children,
  className,
  innerClassName,
  labelledBy,
}: {
  id?: string;
  children: ReactNode;
  className?: string;
  innerClassName?: string;
  labelledBy?: string;
}) {
  return (
    <section id={id} aria-labelledby={labelledBy} className={cn("border-t border-line", className)}>
      <div className="frame">
        <div className={cn("frame-inner py-20 md:py-24", innerClassName)}>{children}</div>
      </div>
    </section>
  );
}

/**
 * Título em duas cores: a afirmação em preto e a continuação em cinza, cada
 * uma na sua linha. Dá hierarquia sem precisar de um subtítulo em outro tamanho.
 */
export function SectionHeading({
  id,
  title,
  lead,
  className,
  align = "left",
  size = "lg",
}: {
  id: string;
  title: ReactNode;
  lead?: ReactNode;
  className?: string;
  align?: "left" | "center";
  size?: "lg" | "xl";
}) {
  return (
    <div className={cn(align === "center" ? "mx-auto max-w-[820px] text-center" : "max-w-[860px]", className)}>
      <h2
        id={id}
        className={cn(
          "font-display font-semibold text-fg text-balance",
          size === "xl"
            ? "text-[44px] leading-[1.02] tracking-[-0.055em] sm:text-[60px] lg:text-[76px]"
            : "text-[30px] leading-[1.14] tracking-[-0.045em] sm:text-[36px] lg:text-[42px]",
        )}
      >
        {title}
        {lead && <span className="block text-muted">{lead}</span>}
      </h2>
    </div>
  );
}
