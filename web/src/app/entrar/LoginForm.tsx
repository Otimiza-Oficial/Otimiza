"use client";

import { ArrowRight, Eye, EyeOff, LoaderCircle, TriangleAlert } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useId, useState, type FormEvent } from "react";
import { BrandLogo } from "@/components/brand/BrandLogo";
import { Button, ButtonLink } from "@/components/ui/Button";
import { SmartLink } from "@/components/ui/SmartLink";
import { useLicencas } from "@/lib/armazem";
import { conferir, explicar } from "@/lib/licenca";
import { useSessao } from "@/lib/sessao";
import { links } from "@/lib/site";

/*
 * A CHAVE É A SENHA, E ESSA É A DIFERENÇA DESTE LOGIN.
 *
 * O Otimiza não tem conta: a licença é uma assinatura Ed25519 emitida para um
 * computador. Conferir a assinatura aqui prova quem é o cliente sem servidor,
 * sem cadastro e sem senha para alguém esquecer.
 *
 * Por isso o campo se comporta como um campo de senha — mascarado, com botão
 * de mostrar — mesmo que o que se cola nele seja público: quem está de pé atrás
 * da pessoa não precisa ver a chave dela.
 */
export function LoginForm() {
  const [chave, setChave] = useState("");
  const [mostrar, setMostrar] = useState(false);
  const [entrando, setEntrando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const { adicionar } = useLicencas();
  const sessao = useSessao();
  const router = useRouter();
  const idChave = useId();

  // Quem já entrou neste navegador não vê o formulário de novo.
  useEffect(() => {
    if (!sessao.carregando && sessao.entrou) router.replace(sessao.destino);
  }, [sessao.carregando, sessao.entrou, sessao.destino, router]);

  const aoEnviar = async (e: FormEvent) => {
    e.preventDefault();
    if (!chave.trim() || entrando) return;
    setEntrando(true);
    setErro(null);

    const r = await conferir(chave, null);
    if (!r.ok) {
      setErro(explicar(r.recusa));
      setEntrando(false);
      return;
    }

    adicionar(chave);
    // A troca de tela acontece no efeito acima, quando a chave entra no
    // armazém: assim o caminho é o mesmo de quem chega já logado.
  };

  return (
    <>
      <p className="eyebrow">Área do cliente</p>
      <h1 className="font-display mt-3 text-[32px] leading-[1.08] font-semibold tracking-[-0.045em]">
        Bem-vindo de volta.
      </h1>
      <p className="mt-3 text-[14px] leading-[1.6] text-muted">
        Cole a chave da sua licença para entrar. Ela é conferida aqui no seu navegador, com a mesma chave pública do
        programa — nada é enviado para lugar nenhum.
      </p>

      <form onSubmit={aoEnviar} className="mt-8" noValidate>
        <div className="flex items-baseline justify-between">
          <label htmlFor={idChave} className="text-[12.5px] font-semibold">
            A sua chave
          </label>
          <button
            type="button"
            onClick={() => setMostrar((v) => !v)}
            className="inline-flex min-h-8 items-center gap-1.5 text-[12px] text-muted transition-colors hover:text-fg"
          >
            {mostrar ? <EyeOff size={13} strokeWidth={2} aria-hidden="true" /> : <Eye size={13} strokeWidth={2} aria-hidden="true" />}
            {mostrar ? "Ocultar" : "Mostrar"}
          </button>
        </div>

        <input
          id={idChave}
          type={mostrar ? "text" : "password"}
          value={chave}
          onChange={(e) => {
            setChave(e.target.value);
            setErro(null);
          }}
          autoComplete="off"
          spellCheck={false}
          placeholder="Cole aqui a chave que você recebeu"
          aria-invalid={Boolean(erro)}
          aria-describedby={erro ? `${idChave}-erro` : undefined}
          className="campo mt-1.5 font-mono text-[12.5px]"
        />

        {erro && (
          <p
            id={`${idChave}-erro`}
            role="alert"
            className="mt-2.5 flex gap-2 rounded-[7px] bg-[#f4f4f3] p-2.5 text-[12.5px] leading-[1.5] text-[#2b2b2b] shadow-[inset_0_0_0_1px_rgb(10_10_10/0.08)]"
          >
            <TriangleAlert size={14} strokeWidth={2} className="mt-0.5 shrink-0" aria-hidden="true" />
            {erro}
          </p>
        )}

        <Button type="submit" size="md" disabled={!chave.trim() || entrando} className="mt-4 w-full">
          {entrando ? (
            <>
              <LoaderCircle size={15} strokeWidth={2.25} className="animate-spin" aria-hidden="true" />
              Entrando…
            </>
          ) : (
            <>
              Entrar
              <ArrowRight size={15} strokeWidth={2} aria-hidden="true" />
            </>
          )}
        </Button>
      </form>

      <p className="mt-3 text-center text-[12px] text-muted">
        Perdeu a sua chave?{" "}
        <SmartLink href={links.discord} className="font-medium text-fg underline underline-offset-2">
          A gente reemite no Discord
        </SmartLink>
      </p>

      <div className="my-6 flex items-center gap-3 text-[11px] text-subtle" aria-hidden="true">
        <span className="h-px flex-1 bg-line" />
        ainda não tem uma chave
        <span className="h-px flex-1 bg-line" />
      </div>

      <ButtonLink href={links.comprar} variant="secondary" size="md" className="w-full">
        <BrandLogo brand="discord" size={16} decorative />
        Comprar no Discord — R$ 25, uma vez
      </ButtonLink>

      <p className="mt-6 text-[11.5px] leading-[1.55] text-subtle">
        Não há conta nem senha: a chave fica guardada só neste navegador, e sair apaga.
      </p>
    </>
  );
}
