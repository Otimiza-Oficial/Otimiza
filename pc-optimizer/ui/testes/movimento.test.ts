// Movimento sóbrio (1.0.12, Frente 1.6): sem curva que passa do ponto (quique), toda transição do Svelte pergunta se o
// movimento está parado ("reduzir movimento" ou jogo aberto), e as regras globais que param tudo continuam no lugar.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SRC = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "src");
const arquivos = (pasta: string): string[] =>
  readdirSync(pasta, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? arquivos(path.join(pasta, e.name)) : /\.(svelte|css)$/.test(e.name) ? [path.join(pasta, e.name)] : []
  );

test("nenhuma curva passa do ponto (sem quique nem mola)", () => {
  const achados: string[] = [];
  for (const f of arquivos(SRC))
    for (const m of readFileSync(f, "utf8").matchAll(/cubic-bezier\(([^)]*)\)/g)) {
      const [, y1, , y2] = m[1].split(",").map(Number);
      if (y1 < 0 || y1 > 1 || y2 < 0 || y2 > 1) achados.push(`${path.relative(SRC, f)}: ${m[0]}`);
    }
  assert.deepEqual(achados, []);
});

test("toda transição do Svelte respeita o movimento parado", () => {
  const achados: string[] = [];
  for (const f of arquivos(SRC).filter((x) => x.endsWith(".svelte"))) {
    const s = readFileSync(f, "utf8");
    if (/\s(?:in|out|transition):(?:fade|fly|scale|slide|blur)\b/.test(s) && !/duracao|\bdur\(/.test(s)) achados.push(path.relative(SRC, f));
  }
  assert.deepEqual(achados, []);
});

test("as regras que param tudo com jogo aberto e com reduzir movimento estão no tokens.css", () => {
  const css = readFileSync(path.join(SRC, "tokens.css"), "utf8");
  assert.match(css, /\.jogo-aberto \*,[\s\S]*?animation: none !important/);
  assert.match(css, /@media \(prefers-reduced-motion: reduce\)[\s\S]*?animation: none !important/);
});

test("a cena da época é estática: nada se anima dentro do SVG grande (repintaria tudo a cada quadro)", () => {
  const s = readFileSync(path.join(SRC, "componentes", "marca", "CenaDaEpoca.svelte"), "utf8");
  const estilo = s.slice(s.indexOf("<style>"));
  const animacoes = [...estilo.matchAll(/animation:\s*([\w-]+)/g)].map((m) => m[1]);
  assert.deepEqual(animacoes, ["cena-aparece"]);
});
