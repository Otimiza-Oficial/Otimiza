// node --test ui/testes/queda.test.ts — quando o Oti por cima do jogo acende o âmbar.
import { test } from "node:test";
import assert from "node:assert/strict";
import { nova, passo } from "../src/lib/queda.ts";

/** Alimenta `n` leituras de `low`, meio segundo entre elas, e devolve o último estado. */
function alimentar(v: ReturnType<typeof nova>, t0: number, n: number, low: number | null) {
  let aceso = false;
  for (let i = 0; i < n; i++) aceso = passo(v, t0 + i * 500, low);
  return aceso;
}

test("partida estável não acende", () => {
  const v = nova();
  assert.equal(alimentar(v, 0, 60, 120), false);
});

test("queda forte em duas leituras seguidas acende; uma só não", () => {
  const v = nova();
  alimentar(v, 0, 30, 120);
  assert.equal(passo(v, 15_000, 70), false, "uma leitura baixa é ruído");
  assert.equal(passo(v, 15_500, 70), true);
});

test("fica aceso pelo menos 3 s e apaga quando volta", () => {
  const v = nova();
  alimentar(v, 0, 30, 120);
  passo(v, 15_000, 60);
  passo(v, 15_500, 60);
  assert.equal(passo(v, 16_000, 118), true, "voltou rápido, mas segura 3 s");
  assert.equal(passo(v, 19_000, 118), false);
});

test("sem referência ainda (começo da partida) não acende", () => {
  const v = nova();
  alimentar(v, 0, 10, 120);
  assert.equal(alimentar(v, 5000, 5, 30), false);
});

test("leitura vazia (menu) não muda nada", () => {
  const v = nova();
  alimentar(v, 0, 30, 120);
  passo(v, 15_000, 60);
  passo(v, 15_500, 60);
  assert.equal(passo(v, 16_000, null), true);
});

test("a queda não puxa a referência para baixo", () => {
  const v = nova();
  alimentar(v, 0, 30, 120);
  // Muito tempo caído: continua aceso, porque a referência ainda é a da partida boa.
  assert.equal(alimentar(v, 15_000, 80, 70), true);
});
