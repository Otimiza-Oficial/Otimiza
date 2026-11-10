// A dívida de tipos da migração: comandos que a interface ainda chama com `invoke("nome")` e tipo escrito à mão, em
// vez do contrato gerado do Rust (`src-tauri/src/contrato.rs` → `src/lib/tipos.ts`). Tipo à mão já quebrou a tela com
// dado real (o "desfecho" do Histórico, que chega como objeto). A lista só diminui: comando novo entra pelo contrato.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const raiz = new URL("../src/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const divida: string[] = JSON.parse(readFileSync(new URL("./invoke-solto.json", import.meta.url), "utf8"));

function arquivos(pasta: string): string[] {
  return readdirSync(pasta).flatMap((n) => {
    const c = join(pasta, n);
    if (statSync(c).isDirectory()) return arquivos(c);
    return /\.(ts|svelte)$/.test(n) && n !== "tipos.ts" ? [c] : [];
  });
}

const usados = new Set(
  arquivos(raiz).flatMap((f) => [...readFileSync(f, "utf8").matchAll(/invoke(?:<[^(]*?>)?\(\s*"([a-z_]+)"/gs)].map((m) => m[1])),
);

test("nenhum comando novo com invoke solto: entra pelo contrato gerado", () => {
  const novos = [...usados].filter((c) => !divida.includes(c)).sort();
  assert.deepEqual(novos, [], `use o contrato (ui/LEIA-ME.md, "Comando novo na interface") para: ${novos.join(", ")}`);
});

test("o que saiu do invoke solto sai da lista (ela só diminui)", () => {
  const pagos = divida.filter((c) => !usados.has(c));
  assert.deepEqual(pagos, [], `tire de ui/testes/invoke-solto.json: ${pagos.join(", ")}`);
});
