import Link from "next/link";
import type { ComponentProps } from "react";

/**
 * Link que só usa o roteador do Next para o que é DESTE site.
 *
 * Âncoras (`#preco`) e rotas internas (`/entrar`) vão pelo `Link`. Endereços de
 * fora — o instalador no GitHub, o convite do Discord — são um `<a>` comum: o
 * Next não tenta pré-carregá-los, e o Discord abre em outra aba.
 */
export function SmartLink({ href, ...props }: Omit<ComponentProps<"a">, "href"> & { href: string }) {
  if (href.startsWith("/") || href.startsWith("#")) {
    return <Link href={href} {...props} />;
  }

  const baixa = href.endsWith(".exe");
  return (
    <a
      href={href}
      {...(baixa ? {} : { target: "_blank", rel: "noopener noreferrer" })}
      {...props}
    />
  );
}
