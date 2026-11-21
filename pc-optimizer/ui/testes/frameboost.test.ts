import { test } from "node:test";
import assert from "node:assert/strict";

import { erroDaAcao, lerFrameBoost, multiplicadorReal, type LigadoAgora } from "../src/lib/frameboost.ts";

const base: LigadoAgora = {
  multiplicador: 1,
  automatico: true,
  situacao: "Gerando",
  hz: 144,
  pelo_driver: false,
  fps_na_tela: 138,
  fps_do_jogo: 69,
  manteve_pct: 96.4,
  degrau: null,
  so_escala: false,
};

test("o multiplicador é o real (tela ÷ jogo), não o do último quadro: o caso do 'Auto (agora 1×)' gerando 2×", () => {
  // O Rust mandava multiplicador 1 no Auto entre dois quadros reais, com 69 fps do jogo e 138 na tela.
  const l = lerFrameBoost(base);
  assert.equal(l.multiplicador, "2,0×");
  assert.match(l.motivo, /69 fps.*144.*2,0×/);
  assert.equal(l.tom, "ok");
});

test("sem os dois números, não inventa multiplicador no Auto", () => {
  assert.equal(multiplicadorReal(null, 60), null);
  assert.equal(multiplicadorReal(120, 0), null);
  assert.equal(lerFrameBoost({ ...base, fps_na_tela: null, manteve_pct: null }).multiplicador, null);
  // Fixo em 3×: o pedido vale até medir.
  assert.equal(lerFrameBoost({ ...base, automatico: false, multiplicador: 3, fps_na_tela: null }).multiplicador, "3×");
});

test("jogo perto do monitor no Auto: diz que não há o que gerar e aponta a Tela", () => {
  const l = lerFrameBoost({ ...base, fps_do_jogo: 140, fps_na_tela: 144 });
  assert.equal(l.multiplicador, "1,0×");
  assert.match(l.motivo, /perto dos 144 Hz/);
  assert.equal(l.area, "tela");
});

test("o jogo perdendo FPS pede para trocar de modo ou desligar", () => {
  const l = lerFrameBoost({ ...base, manteve_pct: 82 });
  assert.equal(l.tom, "atencao");
  assert.match(l.proximo, /perdeu 18% do FPS/);
});

test("enquanto confere o custo, pede um minuto de jogo", () => {
  assert.match(lerFrameBoost({ ...base, manteve_pct: null }).proximo, /um minuto/);
});

test("procurando a janela e jogo parado dizem o que fazer", () => {
  assert.match(lerFrameBoost({ ...base, situacao: "ProcurandoJanela" }).proximo, /janela sem bordas/);
  assert.match(lerFrameBoost({ ...base, situacao: "SemQuadros" }).proximo, /Volte para o jogo/);
});

test("o degrau da guarda vira o motivo", () => {
  assert.equal(lerFrameBoost({ ...base, degrau: "A placa encheu: passei para o modo leve." }).motivo, "A placa encheu: passei para o modo leve.");
});

test("pelo driver, sem medida, pede para medir", () => {
  const l = lerFrameBoost({ ...base, pelo_driver: true, fps_na_tela: null, fps_do_jogo: null });
  assert.equal(l.multiplicador, null);
  assert.match(l.proximo, /Medir agora/);
});

test("1.0.14: o erro do ligar mostra o motivo do Rust como próximo passo, não 'não respondeu'", () => {
  const volte = "Para travar o FPS base, o Otimiza mede o jogo por 15 s com ele na frente. Ligue o Frame Boost e volte para o jogo em seguida (você tem 20 s).";
  const e = erroDaAcao("ligar", volte);
  assert.equal(e.titulo, "O Frame Boost não ligou");
  assert.equal(e.fazer, volte);
  assert.ok(!e.titulo.includes("respondeu"));
  // Erro vazio ou com o prefixo do JS: nunca uma linha em branco.
  assert.equal(erroDaAcao("ligar", "").fazer, "Tente de novo.");
  assert.equal(erroDaAcao("base", "Error: Abra o jogo primeiro.").fazer, "Abra o jogo primeiro.");
  // Desligar que falhou sempre lembra a saída de dentro do jogo.
  assert.ok(erroDaAcao("desligar", "x").fazer.includes("Ctrl+Alt+G"));
});
