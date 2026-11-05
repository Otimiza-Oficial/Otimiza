import { cn } from "@/lib/cn";

/**
 * A logo do Otimiza 1.0: o mascote de vidro, parado, em tamanho de marca.
 *
 * Em 16–30 px degradê e reflexo viram ruído, então o símbolo é chapado: bolha lavanda, borda azul, "orelhas" e os
 * dois olhos âmbar, que são o que o torna reconhecível. As cores são fixas (a bolha não troca de cor com o texto):
 * funciona igual sobre o céu, sobre o vidro e sobre a noite. A versão grande e viva é `<Mascote />`.
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
      <path d="M12.5 23.5 Q2 32 12.5 40.5 Z" fill="#dbe4fb" stroke="#8fa3ea" strokeWidth="1.6" strokeLinejoin="round" />
      <path d="M51.5 23.5 Q62 32 51.5 40.5 Z" fill="#dbe4fb" stroke="#8fa3ea" strokeWidth="1.6" strokeLinejoin="round" />
      <circle cx="32" cy="32" r="22" fill="#eef2fe" stroke="#7f97ea" strokeWidth="2.4" />
      <path d="M17 26 Q20 17 29 14" fill="none" stroke="#ffffff" strokeWidth="3" strokeLinecap="round" opacity="0.9" />
      <rect x="23.5" y="25" width="6" height="13" rx="3" fill="#a8621c" />
      <rect x="34.5" y="25" width="6" height="13" rx="3" fill="#a8621c" />
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
