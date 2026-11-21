// Contraste medido (1.0.12, Frente 1.3): lê os tokens de `src/tokens.css` nos dois temas e confere cada par de texto e
// fundo que o app usa pela fórmula do WCAG 2.x. Texto ≥ 4,5:1 (AA); anel de foco e borda de controle ≥ 3:1. Mudou um
// token e o par caiu: este teste quebra antes de o cliente forçar a vista.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const css = readFileSync(new URL("../src/tokens.css", import.meta.url), "utf8");

/** As declarações `--nome: valor` de um bloco (o primeiro que casa com o seletor). */
function bloco(seletor: string): Map<string, string> {
  const i = css.indexOf(`${seletor} {`);
  assert.ok(i >= 0, `bloco ${seletor} não achado`);
  const corpo = css.slice(i, css.indexOf("\n}", i));
  const mapa = new Map<string, string>();
  for (const m of corpo.matchAll(/(--[\w-]+):\s*([^;]+);/g)) mapa.set(m[1], m[2].replace(/\/\*.*?\*\//g, "").trim());
  return mapa;
}

const claro = bloco(":root");
const escuro = new Map([...claro, ...bloco(':root[data-tema="noite"]')]);

/** Resolve `var(--x)` até chegar num `#rrggbb`. */
function cor(tema: Map<string, string>, nome: string): string {
  let v = tema.get(nome);
  for (let i = 0; v && v.startsWith("var(") && i < 8; i++) v = tema.get(v.slice(4, -1).trim());
  assert.ok(v && /^#[0-9a-f]{6}$/i.test(v), `${nome} não virou uma cor sólida (${v})`);
  return v!;
}

function luminancia(hex: string) {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contraste(a: string, b: string) {
  const [l1, l2] = [luminancia(a), luminancia(b)].sort((x, y) => y - x);
  return (l1 + 0.05) / (l2 + 0.05);
}

const TEXTOS = ["--tinta", "--tinta-2", "--tinta-3", "--ok-texto", "--atencao-texto", "--perigo-texto"];
const FUNDOS = ["--fundo", "--superficie", "--superficie-2"];

for (const [nome, tema] of [["claro", claro], ["escuro", escuro]] as const) {
  test(`tema ${nome}: todo texto passa no AA (4,5:1) sobre o fundo e as superfícies`, () => {
    const falhas: string[] = [];
    for (const t of TEXTOS)
      for (const f of FUNDOS) {
        const r = contraste(cor(tema, t), cor(tema, f));
        if (r < 4.5) falhas.push(`${t} sobre ${f}: ${r.toFixed(2)}:1`);
      }
    assert.deepEqual(falhas, []);
  });

  test(`tema ${nome}: a letra do botão principal e o anel de foco aparecem`, () => {
    assert.ok(contraste(cor(tema, "--contra"), cor(tema, "--acao")) >= 4.5, "letra do botão principal");
    for (const f of FUNDOS) assert.ok(contraste(cor(tema, "--foco"), cor(tema, f)) >= 3, `foco sobre ${f}`);
  });

  test(`tema ${nome}: o ponto de estado se distingue do fundo (3:1, componente gráfico)`, () => {
    for (const t of ["--ok", "--atencao", "--perigo"])
      for (const f of FUNDOS) assert.ok(contraste(cor(tema, t), cor(tema, f)) >= 3, `${t} sobre ${f}`);
  });
}

test("a fórmula confere com os pares do MARCA.md", () => {
  assert.equal(contraste("#ffffff", "#000000").toFixed(0), "21");
  assert.equal(contraste("#5e5e63", "#f5f5f7").toFixed(1), "5.9");
  // O MARCA.md diz 18,9:1 para carvão no papel; a fórmula dá 18,2:1. Os dois passam muito do AA.
  assert.equal(contraste("#0a0a0b", "#f5f5f7").toFixed(1), "18.2");
});
