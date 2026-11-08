"use client";

/**
 * A bancada pública: por placa e jogo, a mediana do FPS e do 1% pior que os PCs da comunidade mediram de verdade
 * (Plano 1.1, Onda 5). Vem da API do bot (`GET /v1/bancada`), que só solta grupo com 3 PCs ou mais.
 */
import { useEffect, useMemo, useState } from "react";
import { API } from "@/lib/api";

type Linha = { placa: string; jogo: string; pcs: number; fps: number | null; low_1pct: number | null };
type Bancada = { atualizado: number; minimo: number; linhas: Linha[] };

export function TabelaDaBancada() {
  const [dados, setDados] = useState<Bancada | null>(null);
  const [erro, setErro] = useState(false);
  const [jogo, setJogo] = useState("");

  useEffect(() => {
    if (!API) return;
    fetch(`${API}/v1/bancada`)
      .then((r) => (r.ok ? r.json() : Promise.reject()))
      .then(setDados)
      .catch(() => setErro(true));
  }, []);

  const jogos = useMemo(() => [...new Set((dados?.linhas ?? []).map((l) => l.jogo))].sort(), [dados]);
  const linhas = (dados?.linhas ?? []).filter((l) => !jogo || l.jogo === jogo);

  if (!API || erro) {
    return <p className="text-[14px] text-muted">A bancada não está disponível agora. Tente de novo mais tarde.</p>;
  }
  if (!dados) return <p className="text-[14px] text-muted">Carregando os números…</p>;
  if (dados.linhas.length === 0) {
    return (
      <p className="text-[14px] text-muted">
        Ainda não há {dados.minimo} PCs iguais medindo o mesmo jogo. Quando houver, os números aparecem aqui.
      </p>
    );
  }

  return (
    <div className="space-y-4">
      <label className="flex items-center gap-3 text-[13.5px]">
        Jogo
        <select
          value={jogo}
          onChange={(e) => setJogo(e.target.value)}
          className="rounded-lg border border-line bg-transparent px-3 py-1.5"
        >
          <option value="">Todos</option>
          {jogos.map((j) => (
            <option key={j} value={j}>
              {j}
            </option>
          ))}
        </select>
      </label>
      <div className="overflow-x-auto">
        <table className="w-full text-left text-[13.5px]">
          <thead className="text-[12px] text-subtle">
            <tr>
              <th className="py-2 pr-4 font-medium">Jogo</th>
              <th className="py-2 pr-4 font-medium">Placa de vídeo</th>
              <th className="py-2 pr-4 text-right font-medium">FPS</th>
              <th className="py-2 pr-4 text-right font-medium">1% pior</th>
              <th className="py-2 text-right font-medium">PCs</th>
            </tr>
          </thead>
          <tbody>
            {linhas.map((l) => (
              <tr key={`${l.jogo}-${l.placa}`} className="border-t border-line">
                <td className="py-2 pr-4">{l.jogo}</td>
                <td className="py-2 pr-4 capitalize">{l.placa}</td>
                <td className="py-2 pr-4 text-right font-mono">{l.fps ?? "—"}</td>
                <td className="py-2 pr-4 text-right font-mono">{l.low_1pct ?? "—"}</td>
                <td className="py-2 text-right font-mono text-subtle">{l.pcs}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="text-[12px] text-subtle">
        Mediana de cada PC, medida pelo Otimiza durante partidas de verdade (não benchmark). Configuração gráfica,
        processador e mapa mudam o número: é um retrato, não uma promessa.
      </p>
    </div>
  );
}
