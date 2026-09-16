import type { Metadata } from "next";
import { site } from "@/lib/site";
import { BoasVindas } from "./BoasVindas";

export const metadata: Metadata = {
  title: "Bem-vindo",
  description: "O primeiro acesso à área do cliente do Otimiza.",
  alternates: { canonical: `${site.url}/boas-vindas/` },
  robots: { index: false, follow: true },
};

export default function Page() {
  return (
    <main id="conteudo">
      <BoasVindas />
    </main>
  );
}
