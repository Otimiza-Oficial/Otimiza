// A nota da fila (bufferbloat): quanto o ping sobe com a linha cheia, na mesma escala dos testes públicos de
// bufferbloat (A+ até 5 ms a mais, A até 30, B até 60, C até 200, D até 400, F daí para cima). Pura, para testar.

export type NotaDaFila = "A+" | "A" | "B" | "C" | "D" | "F";

/** A nota pelo pior aumento entre baixando e enviando. `null` quando nenhum dos dois encheu a linha. */
export function notaDaFila(baixando: number | null | undefined, enviando: number | null | undefined): NotaDaFila | null {
  const valores = [baixando, enviando].filter((v): v is number => v != null && Number.isFinite(v));
  if (valores.length === 0) return null;
  const pior = Math.max(...valores);
  if (pior < 5) return "A+";
  if (pior < 30) return "A";
  if (pior < 60) return "B";
  if (pior < 200) return "C";
  if (pior < 400) return "D";
  return "F";
}

/** O tom da nota na tela: A e B estão bons para jogar; C já se sente; D e F pedem conserto no roteador. */
export function tomDaNota(n: NotaDaFila): "ok" | "atencao" | "perigo" {
  return n === "A+" || n === "A" || n === "B" ? "ok" : n === "C" ? "atencao" : "perigo";
}
