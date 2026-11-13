import { test } from "node:test";
import assert from "node:assert/strict";

import { contextoDa, type DadosDoContexto } from "../src/lib/contexto.ts";

const base: DadosDoContexto = {
  tela: { hz: 60, aguenta: 180 },
  ligacao: { tipo: "wifi", sinal: 66 },
  memoriaPct: 67,
  mudancas: 4,
  jogosMedidos: 3,
  licenca: { ativa: true, expira: null },
  jogoAberto: null,
};

test("a Tela diz quanto o monitor aceita, e pede atenção abaixo do máximo", () => {
  assert.deepEqual(contextoDa("tela", base), { texto: "Monitor em 60 Hz, aceita 180", tom: "atencao" });
  assert.equal(contextoDa("tela", { ...base, tela: { hz: 180, aguenta: 180 } })?.tom, "ok");
  assert.equal(contextoDa("tela", { ...base, tela: null }), null);
});

test("a Rede diz por onde o PC sai, e sinal fraco pede atenção", () => {
  assert.equal(contextoDa("rede", base)?.texto, "Wi-Fi com sinal de 66%");
  assert.equal(contextoDa("rede", { ...base, ligacao: { tipo: "wifi", sinal: 30 } })?.tom, "atencao");
  assert.equal(contextoDa("rede", { ...base, ligacao: { tipo: "cabo" } })?.texto, "Você está no cabo");
});

test("plural certo e nada inventado sem dado", () => {
  assert.equal(contextoDa("historico", { ...base, mudancas: 1 })?.texto, "1 mudança no histórico, cada uma com desfazer");
  assert.equal(contextoDa("jogos", { ...base, jogosMedidos: 0 }), null);
  assert.equal(contextoDa("latencia", base), null);
  assert.equal(contextoDa("ajustes", { ...base, licenca: { ativa: false, expira: null } })?.tom, "atencao");
});
