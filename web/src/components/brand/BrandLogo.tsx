import { siDiscord, siMercadopago, siNvidia, siPix } from "simple-icons";
import { cn } from "@/lib/cn";

/**
 * Logos de empresas reais, e só das que o Otimiza de fato usa:
 * Discord (compra e suporte), Pix e Mercado Pago (pagamento), NVIDIA (ajustes
 * de driver pela NVAPI). Adicionar uma marca aqui é afirmar uma integração.
 *
 * Os traços vêm do Simple Icons (CC0), exatamente como o pacote entrega.
 */
const BRANDS = {
  discord: siDiscord,
  pix: siPix,
  mercadopago: siMercadopago,
  nvidia: siNvidia,
} as const;

export type Brand = keyof typeof BRANDS;

export function BrandLogo({
  brand,
  size = 16,
  tone = "mono",
  className,
  decorative = false,
}: {
  brand: Brand;
  size?: number;
  /** `mono` herda a cor do texto — o site é preto e branco; `brand` usa a cor oficial. */
  tone?: "brand" | "mono";
  className?: string;
  /** Verdadeiro quando o nome da marca já está escrito ao lado. */
  decorative?: boolean;
}) {
  const icon = BRANDS[brand];

  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      className={cn("shrink-0", className)}
      fill={tone === "brand" ? `#${icon.hex}` : "currentColor"}
      role={decorative ? undefined : "img"}
      aria-hidden={decorative ? true : undefined}
      aria-label={decorative ? undefined : icon.title}
    >
      {!decorative && <title>{icon.title}</title>}
      <path d={icon.path} />
    </svg>
  );
}
