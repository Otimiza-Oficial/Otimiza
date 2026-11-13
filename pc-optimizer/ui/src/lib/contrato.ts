// A ponte entre o contrato gerado (`tipos.ts`) e os tipos que as áreas escreveram à mão antes dele. O contrato garante
// o nome do comando e os parâmetros (comando renomeado no Rust quebra o build, não a tela); o tipo da resposta continua
// o da área, como era com o `invoke<T>` solto. Onde a área passar a usar o tipo gerado, o `como` sai.
//
// A diferença mais comum é o `f64` do Rust: no contrato ele é `number | null` (um NaN chega como `null`), e as áreas
// declararam `number`.

/** A resposta de um comando do contrato, no tipo que a área usa. */
export function como<T>(resposta: Promise<unknown>): Promise<T> {
  return resposta as Promise<T>;
}
