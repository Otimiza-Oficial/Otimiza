import Link from "next/link";
import type { ComponentProps } from "react";

/**
 * Rotas deste site vão pelo `Link`; endereços de fora viram `<a>` em outra aba.
 * `//host` é externo, e o `rel` vem por último para nenhuma prop o desligar.
 */
export function SmartLink({ href, ...props }: Omit<ComponentProps<"a">, "href"> & { href: string }) {
  const interno = (href.startsWith("/") && !href.startsWith("//")) || href.startsWith("#");

  if (interno) {
    return <Link href={href} {...props} />;
  }

  const baixa = href.endsWith(".exe");
  return (
    <a
      href={href}
      {...props}
      {...(baixa ? {} : { target: "_blank", rel: "noopener noreferrer" })}
    />
  );
}
