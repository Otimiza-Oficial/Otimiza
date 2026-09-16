"use client";

import { KeyRound } from "lucide-react";
import Link from "next/link";
import { useEffect, useState } from "react";
import { EstadoVazio } from "@/components/painel/Estados";
import { Selo } from "@/components/painel/CartaoMetrica";
import { useLicencas } from "@/lib/armazem";
import { conferir, formatarData, type Dados } from "@/lib/licenca";

type Linha = { chave: string; dados: Dados | null };

/**
 * As máquinas desta conta: uma linha por chave guardada, conferida na hora.
 *
 * É o equivalente honesto da lista de pedidos de um painel de loja — a lista
 * do que a pessoa realmente tem aqui, e não um histórico inventado.
 */
export function ListaMaquinas() {
  const { licencas } = useLicencas();
  const [linhas, setLinhas] = useState<Linha[] | null>(null);

  useEffect(() => {
    let vivo = true;
    (async () => {
      const lidas: Linha[] = [];
      for (const l of licencas) {
        const r = await conferir(l.chave, null);
        lidas.push({ chave: l.chave, dados: r.ok ? r.dados : null });
      }
      if (vivo) setLinhas(lidas);
    })();
    return () => {
      vivo = false;
    };
  }, [licencas]);

  if (licencas.length === 0) {
    return (
      <EstadoVazio
        icone={KeyRound}
        titulo="Nenhuma chave guardada"
        texto="Confira a sua chave para ela ficar à mão quando você formatar ou trocar de computador."
        acao={
          <Link
            href="/painel/licencas/"
            className="inline-flex h-[28px] items-center rounded-[6px] bg-ink px-2.5 text-[11.5px] font-semibold text-white"
          >
            Conferir uma chave
          </Link>
        }
      />
    );
  }

  return (
    <table className="w-full">
      <caption className="sr-only">Licenças guardadas neste navegador</caption>
      <thead>
        <tr className="border-b border-line text-left text-[10.5px] tracking-[0.06em] text-subtle uppercase">
          <th scope="col" className="px-3.5 py-1.5 font-medium">
            Computador
          </th>
          <th scope="col" className="hidden px-2 py-1.5 font-medium sm:table-cell">
            Emitida
          </th>
          <th scope="col" className="px-3.5 py-1.5 text-right font-medium">
            Validade
          </th>
        </tr>
      </thead>
      <tbody className="divide-y divide-line">
        {(linhas ?? licencas.map((l) => ({ chave: l.chave, dados: null }))).map(({ chave, dados }) => (
          <tr key={chave} className="transition-colors hover:bg-[#f9f9f8]">
            <td className="px-3.5 py-2.5">
              <p className="font-mono text-[12px] font-medium">{dados ? dados.maquina : "—"}</p>
              <p className="truncate text-[11px] text-subtle">
                {dados ? dados.comprador || "Sem nome no registro" : linhas ? "Esta chave não confere" : "Conferindo…"}
              </p>
            </td>
            <td className="tabular hidden px-2 py-2.5 text-[11.5px] text-muted sm:table-cell">
              {dados ? formatarData(dados.emitida) : "—"}
            </td>
            <td className="px-3.5 py-2.5 text-right">
              {dados ? (
                <Selo tom={dados.expira ? "claro" : "escuro"}>{dados.expira ? formatarData(dados.expira) : "Vitalícia"}</Selo>
              ) : linhas ? (
                <Selo>Não confere</Selo>
              ) : null}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
