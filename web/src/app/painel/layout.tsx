import type { Metadata } from "next";
import { AppShell } from "@/components/painel/AppShell";
import { site } from "@/lib/site";

export const metadata: Metadata = {
  title: { default: "Painel", template: "%s · Painel · Otimiza" },
  description: "Suas licenças do Otimiza, conferidas neste navegador, e o instalador da versão mais nova.",
  alternates: { canonical: `${site.url}/painel/` },
  // O painel é pessoal e vazio para quem chega de busca: não vale indexar.
  robots: { index: false, follow: true },
};

export default function PainelLayout({ children }: LayoutProps<"/painel">) {
  return <AppShell>{children}</AppShell>;
}
