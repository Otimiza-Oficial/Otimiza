import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

import { CATALOGO, doCatalogo } from "../src/lib/catalogo.ts";

const raiz = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const rust = (p: string) => readFileSync(path.join(raiz, "pc-optimizer/src-tauri/src", p), "utf8");

test("cada perfil do catálogo existe no configmotor.rs", () => {
  const motor = rust("modules/windows/configmotor.rs");
  for (const j of CATALOGO) {
    if (j.perfil == null) {
      assert.ok(j.configEmOutroLugar, `${j.id}: sem perfil, a tela precisa dizer onde fica a configuração`);
      continue;
    }
    assert.ok(motor.includes(`id: "${j.perfil}"`), `${j.id}: o perfil "${j.perfil}" não existe no configmotor.rs`);
  }
});

test("cada cache do catálogo existe no shaders.rs", () => {
  const shaders = rust("modules/windows/shaders.rs");
  for (const j of CATALOGO) {
    assert.ok(j.caches.length > 0, j.id);
    for (const c of j.caches) assert.ok(shaders.includes(`"${c}"`), `${j.id}: o cache "${c}" não existe no shaders.rs`);
  }
});

test("todo jogo tem ficha, texto do anticheat e id único", () => {
  const ids = new Set<string>();
  for (const j of CATALOGO) {
    assert.ok(!ids.has(j.id), `id repetido: ${j.id}`);
    ids.add(j.id);
    assert.ok(existsSync(path.join(raiz, j.ficha)), `${j.id}: falta a ficha ${j.ficha}`);
    assert.ok(j.anticheat.length > 40, `${j.id}: o texto do anticheat é a explicação para a pessoa`);
  }
});

test("reconhece os nomes que as bibliotecas mostram", () => {
  assert.equal(doCatalogo("FiveM")?.id, "fivem");
  assert.equal(doCatalogo("VALORANT")?.id, "valorant");
  assert.equal(doCatalogo("Counter-Strike 2")?.id, "cs2");
  assert.equal(doCatalogo("qualquer nome", 730)?.id, "cs2", "pelo appid da Steam");
  assert.equal(doCatalogo("Fortnite")?.id, "fortnite");
  assert.equal(doCatalogo("Apex Legends")?.id, "apex");
  assert.equal(doCatalogo("Apex Legends™", 1172470)?.id, "apex");
  assert.equal(doCatalogo("Counter-Strike: Global Offensive"), null, "o CS antigo não é o CS2");
  assert.equal(doCatalogo("Minecraft"), null);
});
