import type { ReactNode } from "react";
import { Footer } from "@/components/layout/Footer";
import { Navbar } from "@/components/layout/Navbar";

/*
 * AS DUAS PÁGINAS LEGAIS, COM A MESMA MOLDURA.
 *
 * Elas existem porque vender pelo site exige: quem cobra precisa dizer o que
 * está vendendo, em que condições, e o que faz com os dados de quem compra.
 *
 * O TEXTO É COLADO NO PRODUTO, e não copiado de um modelo. Cada afirmação aqui
 * corresponde a uma decisão que está no código: a licença é uma assinatura
 * presa a um computador (`licenca.rs`), não existe conta nem senha
 * (`sessao.ts`), o programa só faz uma pergunta para fora — a de versão
 * (`atualizacao.rs`) —, e a chave fica guardada no navegador de quem a colou.
 *
 * ONDE ESTÁ `[...]`, FALTA UM DADO QUE SÓ O DONO TEM. Nome empresarial, CNPJ
 * ou CPF e e-mail de contato não podem ser inventados por quem escreve a
 * página: eles identificam quem responde legalmente pela venda.
 */

export const REVISAR = "[...]";

export function PaginaLegal({
  titulo,
  atualizada,
  resumo,
  children,
}: {
  titulo: string;
  atualizada: string;
  resumo: string;
  children: ReactNode;
}) {
  return (
    <>
      <Navbar />
      <main id="conteudo" className="frame">
        <div className="frame-inner py-14 sm:py-20">
          <div className="max-w-2xl">
            <p className="font-mono text-[11px] tracking-[0.22em] text-subtle uppercase">Documento</p>
            <h1 className="font-display mt-5 text-[clamp(26px,3.6vw,38px)] leading-[1.1] font-semibold tracking-[-0.035em]">
              {titulo}
            </h1>
            <p className="mt-4 text-[15px] leading-[1.6] text-muted">{resumo}</p>
            <p className="mt-3 text-[12.5px] text-subtle">Última atualização: {atualizada}.</p>
          </div>

          <div className="mt-10 max-w-2xl space-y-8">{children}</div>
        </div>
      </main>
      <Footer />
    </>
  );
}

export function Secao({ titulo, children }: { titulo: string; children: ReactNode }) {
  return (
    <section>
      <h2 className="font-display text-[19px] font-semibold tracking-[-0.02em]">{titulo}</h2>
      <div className="mt-3 space-y-3 text-[14px] leading-[1.65] text-muted">{children}</div>
    </section>
  );
}
