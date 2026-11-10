// O cartão da partida (`src/lib/partida.ts`): o relatório certo, a partida curta dita como curta e a faixa como faixa.
import { test } from "node:test";
import assert from "node:assert/strict";
import { comparar, duracao, eDestaPartida, montarCartao, textoParaCompartilhar, type RelatorioUsado } from "../src/lib/partida.ts";

const agora = 1_800_000_000;
const relatorio = (r: Partial<RelatorioUsado> = {}): RelatorioUsado => ({
  jogo: "FiveM",
  inicio: agora - 3500,
  fim: agora - 300,
  janelas_comparaveis: 4,
  fps_medio: 85,
  pior_low_1pct: 54,
  trancos: 8,
  janelas_com_trancos: 4,
  anteriores: { partidas: 5, mediana_fps: 78, menor_fps: 70, maior_fps: 82 },
  ...r,
});
const fim = { jogo: "FiveM", executavel: "C:/FiveM/FiveM.exe", segundos: 3600, pelo_jogar: true };

test("o relatório de outra partida, de antes desta começar, não vira o cartão", () => {
  assert.equal(eDestaPartida(fim, relatorio({ fim: agora - 3600 * 5 }), agora), false);
  assert.equal(eDestaPartida(fim, relatorio({ jogo: "Counter-Strike 2" }), agora), false);
  assert.equal(eDestaPartida(fim, relatorio(), agora), true);
});

test("Roblox Player e Roblox são o mesmo jogo", () => {
  assert.equal(eDestaPartida({ ...fim, jogo: "Roblox Player" }, relatorio({ jogo: "Roblox" }), agora), true);
});

test("partida curta diz que foi curta, em vez de mostrar outra", () => {
  const c = montarCartao({ ...fim, segundos: 120 }, relatorio({ fim: agora - 3600 * 5 }), agora);
  assert.equal(c.medido, false);
  assert.equal(c.medido === false && c.porque, "curta");
  const longa = montarCartao(fim, null, agora);
  assert.equal(longa.medido === false && longa.porque, "sem-medicao");
});

test("trancos por minuto saem das janelas que os leram (20 s cada)", () => {
  const c = montarCartao(fim, relatorio(), agora);
  assert.ok(c.medido);
  // 8 trancos em 4 janelas de 20 s = 80 s = 6 por minuto.
  assert.equal(c.medido && c.trancosPorMinuto, 6);
});

test("a comparação é faixa: acima, dentro ou abaixo, e some com menos de duas anteriores", () => {
  const faixa = { partidas: 5, mediana_fps: 78, menor_fps: 70, maior_fps: 82 };
  assert.equal(comparar(85, faixa)?.sentido, "acima");
  assert.equal(comparar(75, faixa)?.sentido, "dentro");
  assert.equal(comparar(60, faixa)?.sentido, "abaixo");
  assert.equal(comparar(85, { ...faixa, partidas: 1 }), null);
  assert.equal(comparar(null, faixa), null);
  // NaN no Rust chega como null: sem faixa, sem comparação.
  assert.equal(comparar(85, { ...faixa, maior_fps: null }), null);
});

test("duração e texto do Discord só com número medido", () => {
  assert.equal(duracao(4320), "1h12");
  assert.equal(duracao(3600), "1h");
  assert.equal(duracao(30), "1 min");
  const c = montarCartao(fim, relatorio({ trancos: null }), agora);
  assert.ok(c.medido);
  const texto = c.medido ? textoParaCompartilhar(c) : "";
  assert.match(texto, /^FiveM · 1h de partida: 85 FPS médio · 1% pior 54\./);
  assert.doesNotMatch(texto, /trancos/);
});
