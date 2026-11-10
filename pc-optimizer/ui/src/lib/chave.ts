// A chave da licença como ela chega: colada do Discord, às vezes quebrada em linhas, entre crases (bloco de código) ou
// com espaço sobrando. O formato é `<dados base64>.<assinatura base64>` (`src-tauri/src/modules/licenca.rs`), preso ao
// código desta máquina (`OTZ-XXXX-XXXX-XXXX`). Quem cola o código da máquina no lugar da chave ouve isso, em vez do
// "não reconhecida" genérico. Testado em `ui/testes/chave.test.ts`.

/** Tira o que o Discord e a cópia acrescentam: crases, aspas, espaços e quebras de linha. */
export function limparChave(texto: string): string {
  return texto
    .trim()
    .replace(/^`{1,3}|`{1,3}$/g, "")
    .replace(/^["']|["']$/g, "")
    .replace(/\s+/g, "");
}

/** O código desta máquina, que vai PARA o bot, e não a chave que volta dele. */
export function eCodigoDaMaquina(texto: string): boolean {
  return /^OTZ(-[A-Z0-9]{4}){3}$/i.test(limparChave(texto));
}

/** Tem a cara de uma chave: duas partes em base64 separadas por ponto. Quem decide de verdade é o Rust. */
export function pareceChave(texto: string): boolean {
  const c = limparChave(texto);
  return /^[A-Za-z0-9+/=_-]{16,}\.[A-Za-z0-9+/=_-]{16,}$/.test(c);
}

export type Leitura = "vazia" | "codigo-da-maquina" | "incompleta" | "pronta";

export function lerCampo(texto: string): Leitura {
  if (!limparChave(texto)) return "vazia";
  if (eCodigoDaMaquina(texto)) return "codigo-da-maquina";
  return pareceChave(texto) ? "pronta" : "incompleta";
}
