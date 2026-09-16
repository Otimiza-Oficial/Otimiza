"use client";

import { useMemo, useState } from "react";
import { cn } from "@/lib/cn";
import { useHistoricoDeVersoes, type ItemDeVersao } from "@/lib/release";

/*
 * O QUE ESTE GRÁFICO PODE MOSTRAR, E POR QUE NÃO É RECEITA.
 *
 * Um painel de cliente costuma abrir com uma curva de faturamento. Aqui não
 * existe faturamento do cliente para desenhar — e o programa não manda nada
 * para lugar nenhum, então também não há uso, sessão ou telemetria.
 *
 * O que existe, público e conferível, é quando cada versão saiu. É isso que a
 * curva mostra: o ritmo do produto que a pessoa comprou, mês a mês, com a
 * versão dela marcada. Dado real, ou nada.
 */

const MESES = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

type Ponto = { chave: string; rotulo: string; total: number; versoes: string[] };

function agrupar(versoes: ItemDeVersao[], meses: number): Ponto[] {
  const hoje = new Date();
  const pontos: Ponto[] = [];
  for (let i = meses - 1; i >= 0; i--) {
    const d = new Date(hoje.getFullYear(), hoje.getMonth() - i, 1);
    const chave = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
    pontos.push({ chave, rotulo: MESES[d.getMonth()], total: 0, versoes: [] });
  }
  const porChave = new Map(pontos.map((p) => [p.chave, p]));
  for (const v of versoes) {
    const chave = v.publicadaEm.slice(0, 7);
    const ponto = porChave.get(chave);
    if (ponto) {
      ponto.total += 1;
      ponto.versoes.push(v.versao);
    }
  }
  return pontos;
}

const L = 640;
const A = 180;
const MARGEM = { topo: 12, baixo: 22, dir: 8, esq: 8 };

export function GraficoVersoes({ periodoEmMeses = 12 }: { periodoEmMeses?: number }) {
  const historico = useHistoricoDeVersoes();
  const [ativo, setAtivo] = useState<number | null>(null);

  const pontos = useMemo(
    () => (historico.estado === "ok" ? agrupar(historico.versoes, periodoEmMeses) : []),
    [historico, periodoEmMeses],
  );

  if (historico.estado !== "ok") {
    return (
      <div className="grid h-[180px] place-items-center text-[12px] text-subtle">
        {historico.estado === "carregando" ? "Lendo as versões publicadas…" : "Não deu para consultar o GitHub agora."}
      </div>
    );
  }

  const maximo = Math.max(2, ...pontos.map((p) => p.total));
  const passo = (L - MARGEM.esq - MARGEM.dir) / Math.max(1, pontos.length - 1);
  const x = (i: number) => MARGEM.esq + i * passo;
  const y = (v: number) => MARGEM.topo + (A - MARGEM.topo - MARGEM.baixo) * (1 - v / maximo);

  const linha = pontos.map((p, i) => `${i === 0 ? "M" : "L"} ${x(i).toFixed(1)} ${y(p.total).toFixed(1)}`).join(" ");
  const area = `${linha} L ${x(pontos.length - 1).toFixed(1)} ${A - MARGEM.baixo} L ${x(0).toFixed(1)} ${A - MARGEM.baixo} Z`;
  const total = pontos.reduce((s, p) => s + p.total, 0);

  return (
    <div className="relative">
      <svg
        viewBox={`0 0 ${L} ${A}`}
        className="h-[180px] w-full"
        role="img"
        aria-label={`Versões publicadas nos últimos ${periodoEmMeses} meses: ${total} no período.`}
        onMouseLeave={() => setAtivo(null)}
      >
        {/* três fios horizontais, quase invisíveis */}
        {[0, 0.5, 1].map((f) => (
          <line
            key={f}
            x1={MARGEM.esq}
            x2={L - MARGEM.dir}
            y1={y(maximo * f)}
            y2={y(maximo * f)}
            stroke="rgb(10 10 10 / 0.07)"
            strokeWidth="1"
          />
        ))}

        <path d={area} fill="rgb(10 10 10 / 0.05)" />
        <path d={linha} fill="none" stroke="#0a0a0a" strokeWidth="1.5" strokeLinejoin="round" strokeLinecap="round" />

        {pontos.map((p, i) => (
          <g key={p.chave}>
            {p.total > 0 && <circle cx={x(i)} cy={y(p.total)} r={ativo === i ? 3.5 : 2.5} fill="#0a0a0a" />}
            {/* faixa invisível: é ela que captura o ponteiro */}
            <rect
              x={x(i) - passo / 2}
              y={0}
              width={passo}
              height={A}
              fill="transparent"
              onMouseEnter={() => setAtivo(i)}
            />
            <text x={x(i)} y={A - 6} textAnchor="middle" className="fill-[#8a8a8a] text-[9px]">
              {p.rotulo}
            </text>
          </g>
        ))}

        {ativo !== null && (
          <line
            x1={x(ativo)}
            x2={x(ativo)}
            y1={MARGEM.topo}
            y2={A - MARGEM.baixo}
            stroke="rgb(10 10 10 / 0.18)"
            strokeWidth="1"
            strokeDasharray="3 3"
          />
        )}
      </svg>

      {ativo !== null && (
        <div
          className={cn(
            "pointer-events-none absolute top-1 rounded-[6px] bg-[#0a0a0a] px-2 py-1.5 text-[11px] text-white",
            ativo > pontos.length / 2 ? "-translate-x-full" : "",
          )}
          style={{ left: `${(x(ativo) / L) * 100}%` }}
        >
          <p className="font-semibold">
            {pontos[ativo].total === 0
              ? "Nenhuma versão"
              : pontos[ativo].total === 1
                ? "1 versão"
                : `${pontos[ativo].total} versões`}
          </p>
          {pontos[ativo].versoes.length > 0 && (
            <p className="text-white/60">{pontos[ativo].versoes.join(" · ")}</p>
          )}
        </div>
      )}
    </div>
  );
}
