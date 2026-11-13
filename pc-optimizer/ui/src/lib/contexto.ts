// O topo de cada área diz o que ela já sabe (Plano de refino 2.2.2): "Monitor em 60 Hz, aceita 180" em vez de só o
// título. Só com o que a casca já leu (hardware, rede, histórico, licença, jogo aberto): nada é medido para isto.
// Ponto em atenção quando há o que fazer ali. Funções puras, testadas em `ui/testes/contexto.test.ts`.

import type { IdDaArea } from "./areas";

export type DadosDoContexto = {
  tela: { hz: number; aguenta: number } | null;
  /** Por onde o PC sai: cabo, Wi-Fi (com a qualidade 0–100) ou não lido. */
  ligacao: { tipo: "cabo" } | { tipo: "wifi"; sinal: number | null } | null;
  memoriaPct: number | null;
  mudancas: number | null;
  jogosMedidos: number;
  licenca: { ativa: boolean; expira: string | null } | null;
  jogoAberto: string | null;
};

export type Contexto = { texto: string; tom: "ok" | "atencao" | "neutro" };

const plural = (n: number, um: string, varios: string) => `${n} ${n === 1 ? um : varios}`;

export function contextoDa(area: IdDaArea, d: DadosDoContexto): Contexto | null {
  switch (area) {
    case "tela":
      if (!d.tela) return null;
      return d.tela.hz < d.tela.aguenta
        ? { texto: `Monitor em ${d.tela.hz} Hz, aceita ${d.tela.aguenta}`, tom: "atencao" }
        : { texto: `Monitor no máximo dele: ${d.tela.hz} Hz`, tom: "ok" };
    case "rede":
      if (!d.ligacao) return null;
      if (d.ligacao.tipo === "cabo") return { texto: "Você está no cabo", tom: "ok" };
      if (d.ligacao.sinal == null) return { texto: "Você está no Wi-Fi", tom: "neutro" };
      return {
        texto: `Wi-Fi com sinal de ${Math.round(d.ligacao.sinal)}%`,
        tom: d.ligacao.sinal < 40 ? "atencao" : "neutro",
      };
    case "historico":
      if (d.mudancas == null) return null;
      return d.mudancas === 0
        ? { texto: "Nada mudado neste PC ainda", tom: "neutro" }
        : { texto: `${plural(d.mudancas, "mudança", "mudanças")} no histórico, cada uma com desfazer`, tom: "neutro" };
    case "turbo":
      return d.jogoAberto
        ? { texto: `${d.jogoAberto} aberto: o Turbo mede ele agora`, tom: "ok" }
        : { texto: "Abra o jogo: o Turbo mede o jogo aberto", tom: "neutro" };
    case "jogos":
      return d.jogosMedidos > 0 ? { texto: `${plural(d.jogosMedidos, "jogo", "jogos")} com partida medida`, tom: "neutro" } : null;
    case "sistema":
      if (d.memoriaPct == null) return null;
      return d.memoriaPct >= 85
        ? { texto: `Memória em ${Math.round(d.memoriaPct)}%: veja quem está pesando`, tom: "atencao" }
        : { texto: `Memória em ${Math.round(d.memoriaPct)}%`, tom: "neutro" };
    case "ajustes":
      if (!d.licenca) return null;
      if (!d.licenca.ativa) return { texto: "Sem licença ativa", tom: "atencao" };
      return { texto: d.licenca.expira ? `Licença ativa até ${d.licenca.expira}` : "Licença ativa", tom: "ok" };
    default:
      return null;
  }
}
