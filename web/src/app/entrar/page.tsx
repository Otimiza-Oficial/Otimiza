import type { Metadata } from "next";
import { AuthLayout } from "@/components/auth/AuthLayout";
import { ComposicaoProduto } from "@/components/auth/ComposicaoProduto";
import { site } from "@/lib/site";
import { LoginForm } from "./LoginForm";

export const metadata: Metadata = {
  title: "Entrar",
  description: "Entre na área do cliente do Otimiza com a chave da sua licença.",
  alternates: { canonical: `${site.url}/entrar/` },
  robots: { index: false, follow: true },
};

export default function EntrarPage() {
  return (
    <main id="conteudo">
      <AuthLayout visual={<ComposicaoProduto />}>
        <LoginForm />
      </AuthLayout>
    </main>
  );
}
