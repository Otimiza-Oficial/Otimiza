import type { Metadata } from "next";
import { site } from "@/lib/site";
import { Onboarding } from "./Onboarding";

export const metadata: Metadata = {
  title: "Configurar",
  description: "Os quatro passos entre a chave e o PC medido.",
  alternates: { canonical: `${site.url}/comecar/` },
  robots: { index: false, follow: true },
};

export default function Page() {
  return (
    <main id="conteudo">
      <Onboarding />
    </main>
  );
}
