// A chave colada do jeito que chega do Discord (`src/lib/chave.ts`).
import { test } from "node:test";
import assert from "node:assert/strict";
import { lerCampo, limparChave } from "../src/lib/chave.ts";

const chave = "eyJtYXF1aW5hIjoiT1RaLVdQWVkiLCJleHBpcmEiOm51bGx9.Q2h2Zk1XbmR2b1Bja2xzU2RmZ2hKa2w";

test("o que o Discord acrescenta sai: crases, aspas, espaços e quebras de linha", () => {
  assert.equal(limparChave("```" + chave + "```"), chave);
  assert.equal(limparChave(` "${chave}" `), chave);
  assert.equal(limparChave(chave.slice(0, 30) + "\n" + chave.slice(30)), chave);
});

test("o código da máquina colado no lugar da chave é reconhecido", () => {
  assert.equal(lerCampo("OTZ-WPYY-0J4F-77AB"), "codigo-da-maquina");
  assert.equal(lerCampo(" otz-wpyy-0j4f-77ab "), "codigo-da-maquina");
});

test("vazia, pela metade e pronta", () => {
  assert.equal(lerCampo("   "), "vazia");
  assert.equal(lerCampo(chave.slice(0, 40)), "incompleta");
  assert.equal(lerCampo(chave), "pronta");
});
