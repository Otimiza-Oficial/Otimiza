import { asset } from "@/lib/asset";
import { cn } from "@/lib/cn";

const MARCA = asset("/brand/otimiza-mark.svg");

/**
 * A logo do Otimiza.
 *
 * O símbolo é o arquivo enviado pelo dono (círculo com o pássaro vazado),
 * vetorizado sem redesenho em `public/brand/otimiza-mark.svg`. Ele entra como
 * MÁSCARA, então a cor segue o texto ao redor: preto no claro, branco no escuro.
 */
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
    <span className={cn("inline-flex items-center gap-2.5 text-fg", className)}>
      <span
        aria-hidden="true"
        className="inline-block shrink-0 bg-current"
        style={{
          width: mark,
          height: mark,
          WebkitMask: `url(${MARCA}) center / contain no-repeat`,
          mask: `url(${MARCA}) center / contain no-repeat`,
        }}
      />
      {wordmark ? (
        <span className="font-display text-[16px] font-bold tracking-[-0.03em]">Otimiza</span>
      ) : (
        <span className="sr-only">Otimiza</span>
      )}
    </span>
  );
}
