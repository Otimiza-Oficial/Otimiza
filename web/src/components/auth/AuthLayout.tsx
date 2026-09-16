import type { ReactNode } from "react";
import Link from "next/link";
import { OtimizaLogo } from "@/components/brand/OtimizaLogo";
import { cn } from "@/lib/cn";

/**
 * A moldura das telas de entrada: formulário à esquerda, produto à direita.
 *
 * A divisão não é meio a meio. O formulário fica em pouco mais de um terço
 * porque ele tem três campos e uma decisão; o resto é o produto, que é o que
 * lembra a pessoa do que existe do outro lado da porta.
 *
 * No celular a coluna do produto sai inteira, em vez de virar uma faixa
 * espremida: ali só importa entrar.
 */
export function AuthLayout({
  children,
  visual,
  larguraForma = "md",
}: {
  children: ReactNode;
  visual?: ReactNode;
  /** `lg` para o onboarding, que tem campo e prévia lado a lado. */
  larguraForma?: "md" | "lg";
}) {
  return (
    <div className={cn("min-h-dvh bg-white lg:grid", visual ? "lg:grid-cols-[minmax(0,42fr)_minmax(0,58fr)]" : "")}>
      <div className="flex min-h-dvh flex-col px-5 py-6 sm:px-10 lg:px-12 lg:py-8">
        <Link href="/" aria-label="Otimiza — página inicial" className="inline-flex min-h-10 items-center self-start rounded-md">
          <OtimizaLogo mark={26} wordmark />
        </Link>

        <div className="flex flex-1 items-center py-10">
          <div className={cn("w-full", larguraForma === "lg" ? "max-w-[720px]" : "max-w-[380px]", "mx-auto lg:mx-0")}>
            {children}
          </div>
        </div>

        <p className="text-[11.5px] text-subtle">© 2026 Otimiza · Console de desempenho para Windows</p>
      </div>

      {visual && (
        <div className="relative hidden overflow-hidden border-l border-line bg-[#f4f4f3] lg:block">{visual}</div>
      )}
    </div>
  );
}
