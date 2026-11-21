import { test } from "node:test";
import assert from "node:assert/strict";

import { FRASE_PROVISORIO, recadoDaConferencia, resumoDoFeedback } from "../src/lib/feedback.ts";

test("servidor fora do ar: o recado diz que o problema é nosso e a folha fecha sozinha", () => {
  const r = recadoDaConferencia({
    situacao: "Provisorio",
    desde: "2026-11-21T12:00:00Z",
    aviso: "O problema é do nosso lado (o servidor respondeu 503), não seu. As correções continuam liberadas.",
  });
  assert.equal(r.tom, "ok");
  assert.match(r.texto, /nosso lado/);
  assert.match(r.texto, /não seu/);
  assert.ok(r.fecharEmMs !== null, "a folha não pode ficar aberta como se fosse bloqueio");
});

test("sem aviso do Rust, o provisório usa a frase própria", () => {
  const r = recadoDaConferencia({ situacao: "Provisorio", desde: "x" });
  assert.equal(r.texto, FRASE_PROVISORIO);
});

test("feedback encontrado fecha rápido; não encontrado fica aberto com o aviso", () => {
  assert.equal(recadoDaConferencia({ situacao: "Liberado", verificado_em: "x" }).fecharEmMs, 1400);
  const nao = recadoDaConferencia({ situacao: "Exigido", apagado: false, aviso: "Ainda não achamos a sua mensagem." });
  assert.equal(nao.tom, "atencao");
  assert.equal(nao.fecharEmMs, null);
  assert.equal(nao.texto, "Ainda não achamos a sua mensagem.");
});

test("os Ajustes mostram o provisório como liberado, sem pedir nada", () => {
  const t = resumoDoFeedback({ situacao: "Provisorio", desde: "x" });
  assert.match(t, /Liberado/);
  assert.match(t, /nosso servidor/);
  assert.equal(resumoDoFeedback({ situacao: "Contando", restante_segundos: 61 }), "O pedido aparece em 2 min.");
});
