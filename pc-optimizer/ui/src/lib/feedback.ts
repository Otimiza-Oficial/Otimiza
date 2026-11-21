// O que a folha do feedback diz depois do "Já mandei" (`PedidoDeFeedback.svelte`). Puro, para o teste
// (`testes/feedback.test.ts`).
//
// `Provisorio` é erro NOSSO (servidor fora do ar, sem rede): o Rust já liberou as correções e confere sozinho depois.
// A frase tem que deixar claro que o cliente não fez nada errado, e a folha fecha sozinha para não parecer bloqueio.

export type SituacaoDoFeedback =
  | { situacao: "NaoSeAplica" }
  | { situacao: "Contando"; restante_segundos: number }
  | { situacao: "Liberado"; verificado_em: string }
  | { situacao: "Provisorio"; desde: string }
  | { situacao: "Exigido"; apagado: boolean };

export type RecadoDoFeedback = { tom: "ok" | "atencao"; texto: string; fecharEmMs: number | null };

export const FRASE_PROVISORIO =
  "O problema é do nosso lado, não seu. As correções continuam liberadas e o Otimiza confere o seu feedback sozinho assim que o servidor voltar.";

export function recadoDaConferencia(s: SituacaoDoFeedback & { aviso?: string | null }): RecadoDoFeedback {
  switch (s.situacao) {
    case "Liberado":
      return { tom: "ok", texto: "Feedback encontrado. As correções estão liberadas.", fecharEmMs: 1400 };
    case "Provisorio":
      return { tom: "ok", texto: s.aviso ?? FRASE_PROVISORIO, fecharEmMs: 5000 };
    default:
      return { tom: "atencao", texto: s.aviso ?? "Ainda não achamos a sua mensagem. Tente de novo em um minuto.", fecharEmMs: null };
  }
}

/** A linha "Feedback" dos Ajustes. */
export function resumoDoFeedback(s: Exclude<SituacaoDoFeedback, { situacao: "NaoSeAplica" }>): string {
  switch (s.situacao) {
    case "Liberado":
      return "Recebido. Obrigado.";
    case "Provisorio":
      return "Liberado enquanto o nosso servidor volta. O Otimiza confere sozinho.";
    case "Exigido":
      return "As correções estão pausadas até chegar o seu feedback.";
    case "Contando":
      return `O pedido aparece em ${Math.max(1, Math.ceil(s.restante_segundos / 60))} min.`;
  }
}
