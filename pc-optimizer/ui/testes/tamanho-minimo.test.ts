// O texto do app nunca fica abaixo de 12 px (1.0.12, Frente 1.3): abaixo disso, no monitor de 1080p a 100%, rótulo
// vira borrão. O mínimo é o token `--t-apoio`. A sobreposição do jogo (`overlay.html`) fica de fora: ela é um HUD por
// cima da partida, medido à parte.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SRC = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "src");

function arquivos(pasta: string): string[] {
  return readdirSync(pasta, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? arquivos(path.join(pasta, e.name)) : /\.(svelte|css)$/.test(e.name) ? [path.join(pasta, e.name)] : []
  );
}

test("nenhum font-size abaixo de 12 px nas telas", () => {
  const achados: string[] = [];
  for (const f of arquivos(SRC)) {
    readFileSync(f, "utf8")
      .split("\n")
      .forEach((linha, i) => {
        for (const m of linha.matchAll(/font(?:-size)?:\s*(?:[a-z-]+\s+)*?(\d+(?:\.\d+)?)px/g))
          if (Number(m[1]) < 12) achados.push(`${path.relative(SRC, f)}:${i + 1} ${m[0]}`);
      });
  }
  assert.deepEqual(achados, []);
});

test("os tokens de texto começam em 12 px", () => {
  const css = readFileSync(path.join(SRC, "tokens.css"), "utf8");
  for (const m of css.matchAll(/--t-[\w-]+:\s*(\d+)px/g)) assert.ok(Number(m[1]) >= 12, m[0]);
});
