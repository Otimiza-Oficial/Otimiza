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

// 1.0.12 (3.6): a CSP de produção (`tauri.conf.json`) libera imagem só destes servidores. Capa de um servidor fora da
// lista some na máquina do cliente (a <img> é bloqueada e cai na reserva); servidor na lista sem capa é porta aberta
// sem motivo.
test("a CSP de produção libera exatamente os servidores das capas, e nada em linha", async () => {
  const { readFile } = await import("node:fs/promises");
  const raiz = new URL("../../", import.meta.url);
  const conf = JSON.parse(await readFile(new URL("src-tauri/tauri.conf.json", raiz), "utf8"));
  const csp: Record<string, string> = conf.app.security.csp;
  const fonte = await readFile(new URL("ui/src/lib/capas.ts", raiz), "utf8");

  const usados = new Set([...fonte.matchAll(/https:\/\/([a-z0-9.-]+)\//g)].map((m) => m[1]));
  const liberados = new Set([...csp["img-src"].matchAll(/https:\/\/([a-z0-9.-]+)/g)].map((m) => m[1]));
  assert.ok(usados.size > 0, "não achei nenhum servidor de capa em capas.ts");
  assert.deepEqual([...usados].sort(), [...liberados].sort());

  for (const [diretiva, valor] of Object.entries(csp)) {
    assert.ok(!/unsafe-/.test(valor), `${diretiva} de produção com ${valor}`);
  }
});
