import { cn } from "@/lib/cn";

/**
 * A logo do Otimiza 1.0: o Oti, disco branco com dois olhos em cápsula inclinados (o mesmo do app e do ícone do
 * programa), em tamanho de marca. Em 16–30 px o reflexo vira ruído, então aqui só ficam o disco com um aro fino e os
 * olhos. Cores fixas: funciona sobre fundo claro e escuro. A versão grande e viva é `<Mascote />`.
 */
export function MarcaMascote({ tamanho = 24, className }: { tamanho?: number; className?: string }) {
  return (
    <svg
      aria-hidden="true"
      className={cn("block shrink-0", className)}
      width={tamanho}
      height={tamanho}
      viewBox="0 0 64 64"
    >
      <circle cx="32" cy="32" r="29.5" fill="#f4f4f5" stroke="#c9c9cc" strokeWidth="1.5" />
      <rect x="22" y="26" width="10.5" height="19" rx="5.25" fill="#111113" transform="rotate(-33 27.25 35.5)" />
      <rect x="36" y="18.5" width="10.5" height="19" rx="5.25" fill="#111113" transform="rotate(-33 41.25 28)" />
    </svg>
  );
}

export function OtimizaLogo({
  className,
  mark = 24,
  wordmark = false,
}: {
  className?: string;
  mark?: number;
  wordmark?: boolean;
}) {
  return (
    <span className={cn("inline-flex items-center gap-2 text-fg", className)}>
      <MarcaMascote tamanho={mark} />
      {wordmark ? (
        <span className="font-display text-[16px] font-bold tracking-[-0.03em]">Otimiza</span>
      ) : (
        <span className="sr-only">Otimiza</span>
      )}
    </span>
  );
}
