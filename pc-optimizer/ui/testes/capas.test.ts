import { test } from "node:test";
import assert from "node:assert/strict";

import { capaDepoisDaFalha, capaReserva } from "../src/lib/capas.ts";

test("jogo conhecido tem capa de reserva, qualquer que seja o nome da loja", () => {
  assert.equal(capaReserva("Counter-Strike 2"), "/capas/counterstrike2.svg");
  assert.equal(capaReserva("Roblox Player"), "/capas/roblox.svg");
  assert.equal(capaReserva("VALORANT"), "/capas/valorant.svg");
  assert.equal(capaReserva("Minecraft"), null);
});

test("a arte da loja falhou: vai para a reserva, e a reserva falhando vai para as iniciais", () => {
  const arte = "https://cdn.cloudflare.steamstatic.com/steam/apps/730/library_600x900_2x.jpg";
  assert.equal(capaDepoisDaFalha("Counter-Strike 2", arte), "/capas/counterstrike2.svg");
  assert.equal(capaDepoisDaFalha("Counter-Strike 2", "/capas/counterstrike2.svg"), null);
  assert.equal(capaDepoisDaFalha("Minecraft", "https://exemplo/arte.jpg"), null);
});
