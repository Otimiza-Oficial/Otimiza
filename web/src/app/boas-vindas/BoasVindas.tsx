"use client";

import { ArrowRight, Check, Circle, Download, KeyRound, LifeBuoy, MonitorCheck } from "lucide-react";
import { AuthLayout } from "@/components/auth/AuthLayout";
import { ComposicaoProduto } from "@/components/auth/ComposicaoProduto";
import { Button } from "@/components/ui/Button";
import { useLicencaAtiva, useGuarda, useOnboarding } from "@/lib/sessao";

const PASSOS = [
  { icone: MonitorCheck, titulo: "Dizer qual é este computador", texto: "O código que o programa mostra na tela." },
  { icone: Download, titulo: "Baixar e instalar o Otimiza", texto: "Grátis, e o diagnóstico roda antes de tudo." },
  { icone: KeyRound, titulo: "Ativar com a sua chave", texto: "A conferência acontece na sua máquina." },
  { icone: LifeBuoy, titulo: "Saber onde pedir ajuda", texto: "O suporte é no Discord, com gente." },
];

/**
 * A tela entre entrar e usar.
 *
 * Ela existe porque jogar quem acabou de colar a chave direto num painel vazio
 * não explica nada — e porque os quatro passos abaixo são o que separa uma
 * licença comprada de um PC otimizado. Pular continua sendo possível: nada
 * aqui é obrigatório para a licença valer.
 */
export function BoasVindas() {
  const sessao = useGuarda(["/boas-vindas/"]);
  const { comecar, concluir } = useOnboarding();
  const { licenca } = useLicencaAtiva();

  if (sessao.carregando) return <TelaDeEspera />;

  const nome = licenca?.dados.comprador?.trim();

  return (
    <AuthLayout visual={<ComposicaoProduto />}>
      <p className="eyebrow">Primeiro acesso</p>
      <h1 className="font-display mt-3 text-[32px] leading-[1.08] font-semibold tracking-[-0.045em]">
        {nome ? `Bem-vindo, ${nome.split(" ")[0]}.` : "Bem-vindo à Otimiza."}
      </h1>
      <p className="mt-3 text-[14px] leading-[1.6] text-muted">
        A sua chave confere. Agora são quatro passos curtos até o seu PC estar medido — e você pode fazer todos
        depois, se preferir.
      </p>

      <ol className="mt-8 space-y-1">
        {PASSOS.map(({ icone: Icone, titulo, texto }, i) => (
          <li key={titulo} className="flex items-start gap-3 rounded-[8px] p-2.5 transition-colors hover:bg-[#f6f6f5]">
            <span className="mt-0.5 grid size-7 shrink-0 place-items-center rounded-[7px] bg-[#f3f3f2]">
              <Icone size={14} strokeWidth={2} aria-hidden="true" />
            </span>
            <div className="min-w-0">
              <p className="text-[13.5px] font-semibold">{titulo}</p>
              <p className="mt-0.5 text-[12.5px] leading-[1.5] text-muted">{texto}</p>
            </div>
            <span className="ml-auto pt-1 text-subtle" aria-hidden="true">
              {i === 0 ? <Circle size={13} strokeWidth={2} /> : <Circle size={13} strokeWidth={2} className="opacity-40" />}
            </span>
          </li>
        ))}
      </ol>

      <div className="mt-8 flex flex-col gap-2">
        <Button
          size="md"
          className="w-full"
          onClick={comecar}
        >
          Configurar meu acesso
          <ArrowRight size={15} strokeWidth={2} aria-hidden="true" />
        </Button>

        <Button variant="secondary" size="md" className="w-full" onClick={concluir}>
          Fazer isso depois
        </Button>
      </div>

      <p className="mt-6 flex items-start gap-2 text-[11.5px] leading-[1.55] text-subtle">
        <Check size={13} strokeWidth={2.5} className="mt-0.5 shrink-0 text-fg" aria-hidden="true" />
        A sua licença já está válida. Estes passos são para o programa, não para a compra.
      </p>
    </AuthLayout>
  );
}

/** O que aparece enquanto o navegador ainda não disse o que tem guardado. */
export function TelaDeEspera() {
  return (
    <div className="grid min-h-dvh place-items-center bg-white">
      <p className="sr-only">Carregando</p>
      <span aria-hidden="true" className="size-5 animate-spin rounded-full border-2 border-line-strong border-t-fg" />
    </div>
  );
}
