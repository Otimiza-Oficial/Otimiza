// A confirmação dentro do app (o `componentes/base/Confirmar.svelte`, montado uma vez no App), no lugar do `confirm()`
// do navegador: a janela cinza do sistema, sem o tema nem o idioma do Otimiza, para a ação que apaga ou muda o PC.

export const PEDIR_CONFIRMACAO = "otimiza:confirmar";

export type Confirmacao = {
  titulo: string;
  /** O que acontece e o que custa, em uma ou duas frases. */
  texto: string;
  /** O verbo do botão: "Apagar", "Reiniciar"… nunca "OK". */
  sim: string;
  perigo?: boolean;
};

export type PedidoDeConfirmacao = Confirmacao & { responder: (sim: boolean) => void };

/** `true` só quando a pessoa escolhe o botão da ação. Sem o diálogo montado, a resposta é `false`: nada se apaga. */
export function confirmar(c: Confirmacao): Promise<boolean> {
  return new Promise((responder) => {
    const pedido = new CustomEvent<PedidoDeConfirmacao>(PEDIR_CONFIRMACAO, { detail: { ...c, responder }, cancelable: true });
    if (window.dispatchEvent(pedido)) responder(false);
  });
}
