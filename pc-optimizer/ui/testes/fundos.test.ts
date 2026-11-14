import { test } from "node:test";
import assert from "node:assert/strict";

import { AJUSTE_PADRAO, lerAjusteGuardado, lerFundoGuardado, tamanhoReduzido } from "../src/lib/fundos.ts";

test("o fundo guardado vira um fundo que existe, com os valores antigos traduzidos", () => {
  assert.equal(lerFundoGuardado("aurora", false), "aurora");
  assert.equal(lerFundoGuardado("sem", false), "area");
  assert.equal(lerFundoGuardado("com", false), "comodo");
  assert.equal(lerFundoGuardado("qualquer", false), "limpo");
  assert.equal(lerFundoGuardado(null, false), "limpo");
});

test("a imagem própria sem imagem guardada volta para o limpo", () => {
  assert.equal(lerFundoGuardado("meu", true), "meu");
  assert.equal(lerFundoGuardado("meu", false), "limpo");
});

test("o ajuste guardado fica dentro da faixa e lixo vira o padrão", () => {
  assert.deepEqual(lerAjusteGuardado('{"desfoque":12,"escurecer":0.3}'), { desfoque: 12, escurecer: 0.3 });
  assert.deepEqual(lerAjusteGuardado('{"desfoque":99,"escurecer":-1}'), { desfoque: 30, escurecer: 0 });
  assert.deepEqual(lerAjusteGuardado("{quebrado"), AJUSTE_PADRAO);
  assert.deepEqual(lerAjusteGuardado(null), AJUSTE_PADRAO);
});

test("a imagem reduz pelo lado maior e nunca aumenta", () => {
  assert.deepEqual(tamanhoReduzido(4000, 3000, 2560), { largura: 2560, altura: 1920 });
  assert.deepEqual(tamanhoReduzido(1080, 1920, 1280), { largura: 720, altura: 1280 });
  assert.deepEqual(tamanhoReduzido(800, 600, 2560), { largura: 800, altura: 600 });
});
