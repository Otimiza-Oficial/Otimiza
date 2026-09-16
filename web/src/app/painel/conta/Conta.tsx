"use client";

import { LogOut, RotateCcw, Settings, ShieldCheck, Trash2 } from "lucide-react";
import { useState } from "react";
import { CabecalhoPagina, Cartao } from "@/components/painel/AppShell";
import { Button } from "@/components/ui/Button";
import { limparTudo, useCodigoDaMaquina, useLicencas } from "@/lib/armazem";
import { codigoValido } from "@/lib/licenca";
import { useOnboarding } from "@/lib/sessao";

/**
 * O que este navegador guarda — dito com todas as letras, e com o botão de
 * apagar ao lado.
 *
 * Numa área de cliente comum esta página seria "dados da conta". Aqui não há
 * conta: o que existe é o que ficou gravado nesta máquina, e a pessoa tem o
 * direito de ver a lista e de limpar.
 */
export function Conta() {
  const { licencas } = useLicencas();
  const [codigo] = useCodigoDaMaquina();
  const { onboarding, reiniciar } = useOnboarding();
  const [confirmando, setConfirmando] = useState(false);

  const itens = [
    {
      nome: "Chaves de licença",
      valor: licencas.length === 0 ? "Nenhuma" : licencas.length === 1 ? "1 chave" : `${licencas.length} chaves`,
      detalhe: "Conferidas por assinatura a cada visita. Nunca saem daqui.",
    },
    {
      nome: "Código do computador",
      valor: codigoValido(codigo) ? codigo : "Não informado",
      detalhe: "Serve para dizer se a chave é deste PC e para preencher as mensagens do suporte.",
      mono: codigoValido(codigo),
    },
    {
      nome: "Primeiro acesso",
      valor:
        onboarding.estado === "concluido"
          ? "Concluído"
          : onboarding.estado === "em_andamento"
            ? `Parou no passo ${onboarding.passo + 1}`
            : "Não iniciado",
      detalhe: "Só para o painel saber se mostra a lista do começo.",
    },
  ];

  return (
    <>
      <CabecalhoPagina
        icone={<Settings size={14} strokeWidth={2} aria-hidden="true" />}
        titulo="Preferências"
        texto="Tudo o que a área do cliente guardou neste navegador."
      />

      <div className="grid gap-2 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
        <Cartao className="overflow-hidden">
          <div className="border-b border-line px-3.5 py-2.5">
            <h2 className="text-[12.5px] font-semibold">Guardado neste navegador</h2>
          </div>
          <dl className="divide-y divide-line">
            {itens.map((i) => (
              <div key={i.nome} className="flex items-start justify-between gap-4 px-3.5 py-3">
                <div className="min-w-0">
                  <dt className="text-[12.5px] font-medium">{i.nome}</dt>
                  <dd className="mt-0.5 text-[11.5px] leading-[1.5] text-subtle">{i.detalhe}</dd>
                </div>
                <dd className={i.mono ? "shrink-0 font-mono text-[12px]" : "shrink-0 text-[12.5px] font-semibold"}>{i.valor}</dd>
              </div>
            ))}
          </dl>
        </Cartao>

        <div className="space-y-2">
          <Cartao className="p-3.5">
            <p className="flex items-center gap-2 text-[12.5px] font-semibold">
              <ShieldCheck size={14} strokeWidth={2} aria-hidden="true" />
              Sem conta, sem servidor
            </p>
            <p className="mt-1.5 text-[11.5px] leading-[1.55] text-muted">
              Não há cadastro, senha nem banco de dados nosso. A chave é conferida aqui, com a chave pública do
              programa, e fica guardada só neste navegador — em outro computador, o painel começa vazio.
            </p>
          </Cartao>

          <Cartao className="p-3.5">
            <p className="text-[12.5px] font-semibold">Recomeçar</p>
            <p className="mt-1.5 text-[11.5px] leading-[1.55] text-muted">
              Ver de novo a introdução e os quatro passos do primeiro acesso. As chaves continuam guardadas.
            </p>
            <Button
              variant="secondary"
              className="mt-3 w-full"
              onClick={reiniciar}
            >
              <RotateCcw size={14} strokeWidth={2} aria-hidden="true" />
              Refazer o primeiro acesso
            </Button>
          </Cartao>

          <Cartao className="p-3.5">
            <p className="flex items-center gap-2 text-[12.5px] font-semibold">
              <Trash2 size={14} strokeWidth={2} aria-hidden="true" />
              Apagar tudo e sair
            </p>
            <p className="mt-1.5 text-[11.5px] leading-[1.55] text-muted">
              Remove as chaves, o código da máquina e o andamento do primeiro acesso deste navegador. A sua licença
              continua valendo no computador onde ela foi ativada.
            </p>
            {confirmando ? (
              <div className="mt-3 flex gap-2">
                <Button
                  className="flex-1"
                  onClick={limparTudo}
                >
                  Apagar mesmo
                </Button>
                <Button variant="secondary" onClick={() => setConfirmando(false)}>
                  Cancelar
                </Button>
              </div>
            ) : (
              <Button variant="secondary" className="mt-3 w-full" onClick={() => setConfirmando(true)}>
                <LogOut size={14} strokeWidth={2} aria-hidden="true" />
                Sair deste navegador
              </Button>
            )}
          </Cartao>
        </div>
      </div>
    </>
  );
}
