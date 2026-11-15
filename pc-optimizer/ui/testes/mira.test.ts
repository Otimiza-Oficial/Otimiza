import { test } from "node:test";
import assert from "node:assert/strict";

import { codigoValorant, PADRAO, resumirFlick, sensCs2DeValorant, sensValorant } from "../src/lib/mira.ts";

test("a mira padrão vira um código de perfil do Valorant com cruz, ponto e a cor", () => {
  const { codigo, nota } = codigoValorant(PADRAO);
  assert.equal(nota, null);
  assert.ok(codigo.startsWith("0;P;c;8;u;FFFFFFFF;h;1;"), codigo);
  assert.match(codigo, /;d;1;z;2;a;1;/);
  assert.match(codigo, /;0b;1;0t;2;0l;10;0o;4;0a;1;0m;0;0f;0;1b;0$/);
});

test("só ponto esconde as linhas; círculo e T avisam que saem como cruz", () => {
  assert.match(codigoValorant({ ...PADRAO, estilo: "ponto" }).codigo, /;d;1;.*;0b;0;1b;0$/);
  assert.match(codigoValorant({ ...PADRAO, estilo: "circulo" }).nota ?? "", /círculo/);
  assert.match(codigoValorant({ ...PADRAO, estilo: "t" }).nota ?? "", /T/);
});

test("o código respeita os limites do Valorant e a cor personalizada", () => {
  const { codigo, nota } = codigoValorant({ ...PADRAO, estilo: "cruz", tamanho: 30, cor: "#ff4fd8", opacidade: 0.85, contorno: false });
  assert.match(codigo, /u;FF4FD8FF;h;0;d;0;0b;1;/);
  assert.match(codigo, /0l;20;/);
  assert.match(codigo, /0a;0\.85;/);
  assert.match(nota ?? "", /20/);
});

test("a sensibilidade vai e volta entre CS2 e Valorant", () => {
  assert.equal(sensCs2DeValorant(0), null);
  assert.ok(Math.abs(sensCs2DeValorant(sensValorant(2)!)! - 2) < 1e-9);
});

test("o resumo do flick: precisão, mediana da reação e sem clique nenhum", () => {
  assert.deepEqual(resumirFlick(3, 1, [400, 200, 300]), { acertos: 3, erros: 1, precisao: 75, reacao_ms: 300 });
  assert.deepEqual(resumirFlick(0, 0, []), { acertos: 0, erros: 0, precisao: 0, reacao_ms: null });
  assert.equal(resumirFlick(2, 0, [100, 200]).reacao_ms, 150);
});
