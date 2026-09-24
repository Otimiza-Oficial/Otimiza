import { TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";

export function Aviso({ children }: { children: ReactNode }) {
  return (
    <p
      role="status"
      className="flex items-start gap-2 rounded-[var(--radius-control)] bg-aviso px-3 py-2.5 text-[12.5px] leading-[1.5] text-aviso-fg shadow-[0_0_0_1px_var(--color-aviso-line)]"
    >
      <TriangleAlert size={14} className="mt-0.5 shrink-0" aria-hidden="true" />
      <span>{children}</span>
    </p>
  );
}
