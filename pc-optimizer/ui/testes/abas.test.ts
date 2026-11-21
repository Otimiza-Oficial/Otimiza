import { test } from "node:test";
import assert from "node:assert/strict";

import { indiceDeTab, proximaAba } from "../src/lib/abas.ts";

test("as setas andam entre as abas e dão a volta nas pontas", () => {
  assert.equal(proximaAba(0, "ArrowRight", 4), 1);
  assert.equal(proximaAba(3, "ArrowRight", 4), 0);
  assert.equal(proximaAba(0, "ArrowLeft", 4), 3);
  assert.equal(proximaAba(2, "ArrowLeft", 4), 1);
});

test("Home e End vão para a primeira e a última", () => {
  assert.equal(proximaAba(2, "Home", 5), 0);
  assert.equal(proximaAba(1, "End", 5), 4);
});

test("no trilho vertical valem as setas de cima e de baixo, não as do lado", () => {
  assert.equal(proximaAba(0, "ArrowDown", 4, "vertical"), 1);
  assert.equal(proximaAba(0, "ArrowUp", 4, "vertical"), 3);
  assert.equal(proximaAba(0, "ArrowRight", 4, "vertical"), null);
});

test("outra tecla não é das abas, e lista vazia não anda", () => {
  assert.equal(proximaAba(0, "Enter", 4), null);
  assert.equal(proximaAba(0, "ArrowRight", 0), null);
  // Índice fora da lista (a aba sumiu) volta para dentro antes de andar.
  assert.equal(proximaAba(9, "ArrowRight", 3), 0);
});

test("só a aba escolhida entra no Tab", () => {
  assert.equal(indiceDeTab(true), 0);
  assert.equal(indiceDeTab(false), -1);
});
