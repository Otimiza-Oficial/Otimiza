import Image from "next/image";
import { cn } from "@/lib/cn";

/**
 * Retrato redondo. Só aparece dentro de telas ilustrativas — nunca como
 * depoimento —, e cada foto é de uma pessoa diferente (`pessoas` em site.ts).
 */
export function Avatar({
  src,
  nome,
  size = 32,
  className,
}: {
  src: string;
  nome: string;
  size?: number;
  className?: string;
}) {
  return (
    <Image
      src={src}
      alt={nome}
      width={size}
      height={size}
      sizes={`${size * 2}px`}
      className={cn("shrink-0 rounded-full object-cover ring-1 ring-black/5", className)}
      style={{ width: size, height: size }}
    />
  );
}
