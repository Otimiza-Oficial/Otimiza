/**
 * A barra da janela.
 *
 * O Windows desenha a dele com botoes grandes no canto direito, numa cor que
 * nao e a nossa e num tamanho que nao combina com nada do resto. Desligar a
 * decoracao e desenhar a propria e o que faz o programa parecer um produto, e
 * nao uma pagina dentro de uma moldura emprestada.
 *
 * O preco e este arquivo: arrastar, minimizar, maximizar e fechar passam a ser
 * nossa responsabilidade.
 *
 * ARRASTAR NAO ESTA AQUI. Ele e feito pelo atributo `data-tauri-drag-region` no
 * HTML, que o proprio Tauri interpreta — e isso importa: arrastar janela pelo
 * JavaScript, ouvindo o mouse, fica visivelmente atrasado em relacao ao
 * ponteiro. O sistema operacional faz isso melhor do que qualquer laco nosso.
 */
import { getCurrentWindow } from "@tauri-apps/api/window";

export function ligarBarraDaJanela() {
  // Fora do Tauri — no navegador, durante o desenvolvimento — nao ha janela
  // para controlar. A barra continua desenhada, e os botoes nao fazem nada em
  // vez de estourar no console.
  const dentroDoTauri = "__TAURI_INTERNALS__" in window;

  const janela = dentroDoTauri ? getCurrentWindow() : null;

  const ligar = (id: string, acao: () => Promise<unknown>) => {
    const botao = document.getElementById(id);
    if (!botao) return;

    botao.addEventListener("click", () => {
      if (!janela) return;
      void acao().catch(() => undefined);
    });
  };

  ligar("janela-minimizar", () => janela!.minimize());
  ligar("janela-maximizar", () => janela!.toggleMaximize());
  ligar("janela-fechar", () => janela!.close());

  // A CLASSE DIZ SE ESTA MAXIMIZADA.
  //
  // O canto arredondado da janela precisa sumir quando ela ocupa a tela
  // inteira: canto redondo colado na borda do monitor deixa quatro triangulos
  // da area de trabalho aparecendo, e o efeito e de janela mal encaixada.
  const conferirMaximizada = async () => {
    if (!janela) return;

    const cheia = await janela.isMaximized().catch(() => false);
    document.body.classList.toggle("janela-cheia", cheia);
  };

  void conferirMaximizada();
  void janela?.onResized(() => { void conferirMaximizada(); });
}

export default { ligarBarraDaJanela };
