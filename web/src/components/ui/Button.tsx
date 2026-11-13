import { SmartLink } from "./SmartLink";
import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/cn";

type Variant = "primary" | "secondary" | "light" | "ghost";
type Size = "sm" | "md";

const base =
  "inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-[var(--radius-control)] font-semibold tracking-[-0.01em] " +
  "transition-[background-color,box-shadow,color,transform] duration-150 ease-out " +
  "active:translate-y-px disabled:pointer-events-none disabled:opacity-50";

const variants: Record<Variant, string> = {
  // A pílula azul do app ("Ligar o Turbo"): degradê do céu, brilho de vidro em cima. Branco sobre ela: 6,3:1.
  primary:
    "bg-ink bg-[linear-gradient(180deg,rgb(255_255_255/0.16),rgb(255_255_255/0))] text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.35),0_0_0_1px_rgb(47_95_168/0.6),0_8px_20px_-8px_rgb(47_95_168/0.65)] hover:bg-[var(--botao-hover)]",
  // Vidro claro com borda: fica ao lado do azul sem sumir no céu.
  secondary:
    "bg-surface text-fg shadow-[inset_0_1px_0_rgb(255_255_255/0.5),0_0_0_1px_var(--color-line-strong),0_4px_12px_-6px_rgb(30_60_120/0.25)] backdrop-blur-md hover:bg-card-2",
  // Branco sólido, para cima de um bloco escuro (a noite).
  light:
    "bg-[#ffffff] text-[#13254d] shadow-[inset_0_-1px_0_rgb(19_37_77/0.12),0_6px_18px_-6px_rgb(0_0_0/0.5)] hover:bg-ceu-100",
  ghost: "text-muted hover:text-fg",
};

// 40px até 1024px, para o polegar; um pouco mais compacto no desktop.
const sizes: Record<Size, string> = {
  sm: "h-10 px-4 text-[13px] lg:h-9",
  md: "h-12 px-5 text-[14.5px] lg:h-11",
};

type Common = { variant?: Variant; size?: Size; className?: string; children: ReactNode };

export function ButtonLink({
  variant = "primary",
  size = "sm",
  className,
  children,
  ...props
}: Common & ComponentProps<typeof SmartLink>) {
  return (
    <SmartLink className={cn(base, variants[variant], sizes[size], className)} {...props}>
      {children}
    </SmartLink>
  );
}

export function Button({
  variant = "primary",
  size = "sm",
  className,
  children,
  type = "button",
  ...props
}: Common & ComponentProps<"button">) {
  return (
    <button type={type} className={cn(base, variants[variant], sizes[size], className)} {...props}>
      {children}
    </button>
  );
}
