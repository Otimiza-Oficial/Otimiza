import { test } from "node:test";
import assert from "node:assert/strict";

import { notaDaFila, tomDaNota } from "../src/lib/fila.ts";

test("a nota da fila vem do pior aumento entre baixando e enviando", () => {
  assert.equal(notaDaFila(2, 3), "A+");
  assert.equal(notaDaFila(10, 46), "B");
  assert.equal(notaDaFila(46, 213), "D");
  assert.equal(notaDaFila(null, 450), "F");
  assert.equal(notaDaFila(null, undefined), null);
});

test("A e B são bons para jogar, C já se sente, D e F pedem conserto", () => {
  assert.equal(tomDaNota("B"), "ok");
  assert.equal(tomDaNota("C"), "atencao");
  assert.equal(tomDaNota("F"), "perigo");
});
