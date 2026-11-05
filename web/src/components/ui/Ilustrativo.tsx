import { cn } from "@/lib/cn";

/** A etiqueta que toda tela com número inventado carrega. */
export function Ilustrativo({ className }: { className?: string }) {
  return <p className={cn("text-center text-[11px] text-subtle", className)}>Prévia ilustrativa</p>;
}
