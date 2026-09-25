/**
 * A barra da janela, desenhada por nós (a do Windows não combina com nada). Arrastar NÃO está aqui: é o
 * `data-tauri-drag-region` no HTML, porque arrastar pelo JavaScript fica atrasado em relação ao ponteiro.
 */
import { getCurrentWindow } from "@tauri-apps/api/window";

export function ligarBarraDaJanela() {
  // Fora do Tauri (navegador em desenvolvimento) os botões não fazem nada em vez de estourar no console.
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

  // Maximizada, o canto arredondado some: colado na borda do monitor deixaria quatro triângulos da área de
  // trabalho aparecendo.
  const conferirMaximizada = async () => {
    if (!janela) return;

    const cheia = await janela.isMaximized().catch(() => false);
    document.body.classList.toggle("janela-cheia", cheia);
  };

  void conferirMaximizada();
  void janela?.onResized(() => { void conferirMaximizada(); });
}

export default { ligarBarraDaJanela };
