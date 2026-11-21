// O teclado das abas (1.0.12, Frente 1.3): o padrão "tabs" do WAI-ARIA. Setas andam entre as abas e dão a volta,
// Home e End vão para a primeira e a última; a aba focada já é a escolhida (ativação automática, como no Windows).
// Puro, para o teste (`testes/abas.test.ts`) e para todo componente com `role="tablist"` usar o mesmo.

export type Orientacao = "horizontal" | "vertical";

/** O índice da próxima aba para a tecla, ou `null` quando a tecla não é de navegação (o evento segue o caminho). */
export function proximaAba(atual: number, tecla: string, total: number, orientacao: Orientacao = "horizontal"): number | null {
  if (total <= 0) return null;
  const voltar = orientacao === "horizontal" ? "ArrowLeft" : "ArrowUp";
  const avancar = orientacao === "horizontal" ? "ArrowRight" : "ArrowDown";
  const de = Math.min(Math.max(atual, 0), total - 1);
  switch (tecla) {
    case avancar:
      return (de + 1) % total;
    case voltar:
      return (de - 1 + total) % total;
    case "Home":
      return 0;
    case "End":
      return total - 1;
    default:
      return null;
  }
}

/** O `tabindex` de cada aba: só a escolhida entra no Tab (as outras se alcançam pelas setas). */
export function indiceDeTab(escolhida: boolean): 0 | -1 {
  return escolhida ? 0 : -1;
}
