// node --test ui/testes/temporadas.test.ts — o calendário das épocas, com datas reais conferidas.
import { test } from "node:test";
import assert from "node:assert/strict";
import { pascoa, temporadaDe } from "../src/lib/temporadas.ts";

const d = (a: number, m: number, dia: number) => new Date(a, m - 1, dia, 12);
const id = (x: Date) => temporadaDe(x)?.id ?? null;
const ymd = (x: Date) => `${x.getFullYear()}-${x.getMonth() + 1}-${x.getDate()}`;

test("Páscoa nas datas reais", () => {
  assert.equal(ymd(pascoa(2024)), "2024-3-31");
  assert.equal(ymd(pascoa(2025)), "2025-4-20");
  assert.equal(ymd(pascoa(2026)), "2026-4-5");
  assert.equal(ymd(pascoa(2027)), "2027-3-28");
});

test("Carnaval cai na terça certa (2026: 17/2; 2027: 9/2)", () => {
  assert.equal(id(d(2026, 2, 17)), "carnaval");
  assert.equal(id(d(2026, 2, 18)), "carnaval"); // Quarta de Cinzas
  assert.equal(id(d(2026, 2, 19)), null);
  assert.equal(id(d(2027, 2, 9)), "carnaval");
});

test("Natal de novembro a Reis, com Ano Novo por dentro", () => {
  assert.equal(id(d(2026, 11, 8)), "natal");
  assert.equal(id(d(2026, 12, 25)), "natal");
  assert.equal(id(d(2026, 12, 31)), "ano-novo");
  assert.equal(id(d(2027, 1, 1)), "ano-novo");
  assert.equal(id(d(2027, 1, 5)), "natal");
  assert.equal(id(d(2027, 1, 7)), null);
});

test("Páscoa, Junina, Crianças e Halloween", () => {
  assert.equal(id(d(2026, 4, 3)), "pascoa");
  assert.equal(id(d(2026, 4, 6)), "pascoa");
  assert.equal(id(d(2026, 6, 24)), "festa-junina");
  assert.equal(id(d(2026, 7, 16)), null);
  assert.equal(id(d(2026, 10, 12)), "dia-das-criancas");
  assert.equal(id(d(2026, 10, 31)), "halloween");
  assert.equal(id(d(2026, 3, 15)), null);
});
