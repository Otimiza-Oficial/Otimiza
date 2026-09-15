import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/**
 * O card da grade de recursos: ícone preto, título em duas linhas, uma frase
 * curta, e um "palco" embaixo onde uma tela do produto entra inclinada e sai
 * pela borda — o card corta a tela, e isso dá profundidade sem enfeite.
 */
export function FeatureCard({
  icon: Icon,
  title,
  lead,
  text,
  children,
  className,
  stageClassName,
}: {
  icon: LucideIcon;
  title: string;
  lead: string;
  text?: string;
  children: ReactNode;
  className?: string;
  stageClassName?: string;
}) {
  return (
    <article className={cn("card group flex h-full flex-col", className)}>
      <div className="relative z-10 px-6 pt-6 sm:px-7 sm:pt-7">
        <Icon size={22} strokeWidth={2.25} className="text-fg" aria-hidden="true" />
        <h3 className="font-display mt-5 text-[21px] leading-[1.22] font-semibold tracking-[-0.035em] text-fg sm:text-[23px]">
          {title}
          <br />
          {lead}
        </h3>
        {text && <p className="mt-3 max-w-[340px] text-[14px] leading-[1.55] text-muted">{text}</p>}
      </div>
      <div className={cn("relative mt-7 flex-1", stageClassName)}>{children}</div>
    </article>
  );
}

/** A etiqueta que toda tela com número inventado carrega. */
export function Ilustrativo({ className }: { className?: string }) {
  return <p className={cn("text-center text-[10.5px] text-subtle", className)}>Prévia ilustrativa</p>;
}
