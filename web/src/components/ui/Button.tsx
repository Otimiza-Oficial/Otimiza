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
  // Preto com brilho interno de 1px no topo: parece peça, não retângulo chapado.
  primary:
    "bg-ink text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.16),0_0_0_1px_#000,0_2px_4px_-1px_rgb(0_0_0/0.35)] hover:bg-[#262626]",
  // Branco com borda e sombra curta, para ficar ao lado do preto sem sumir.
  secondary:
    "bg-white text-fg shadow-[0_0_0_1px_rgb(10_10_10/0.1),0_1px_2px_rgb(0_0_0/0.06)] hover:bg-[#f6f6f6]",
  // O botão claro sobre fundo preto.
  light:
    "bg-white text-fg shadow-[inset_0_-1px_0_rgb(0_0_0/0.12),0_0_0_1px_rgb(255_255_255/0.2),0_6px_18px_-6px_rgb(0_0_0/0.6)] hover:bg-[#ececec]",
  ghost: "text-muted hover:text-fg",
};

// Compacto no desktop, como a referência; 40px até 1024px, para o polegar.
const sizes: Record<Size, string> = {
  sm: "h-10 px-3.5 text-[13px] lg:h-[34px]",
  md: "h-11 px-4 text-[14px] lg:h-10",
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
