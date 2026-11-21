import { test } from "node:test";
import assert from "node:assert/strict";

import { Relogio } from "../src/lib/relogio.ts";

/** Um relógio de mentira: o tempo só anda quando o teste manda, e o temporizador é um só. */
function falso() {
  let agora = 0;
  let visivel = true;
  let pendente: { fn: () => void; quando: number } | null = null;
  let agendados = 0;
  const r = new Relogio({
    agora: () => agora,
    visivel: () => visivel,
    agendar: (fn, ms) => {
      agendados++;
      pendente = { fn, quando: agora + ms };
      return pendente;
    },
    cancelar: (t) => {
      if (pendente === t) pendente = null;
    },
  });
  const andar = (ms: number) => {
    const fim = agora + ms;
    while (pendente && pendente.quando <= fim) {
      agora = pendente.quando;
      const p = pendente;
      pendente = null;
      p.fn();
    }
    agora = fim;
  };
  /** O tempo pula sem o temporizador disparar no meio (a máquina dormiu); ao acordar, ele dispara uma vez. */
  const dormir = (ms: number) => {
    agora += ms;
    const p = pendente;
    pendente = null;
    p?.fn();
  };
  return {
    r,
    andar,
    dormir,
    esconder: (v: boolean) => (visivel = !v),
    temporizadores: () => (pendente ? 1 : 0),
    get agendados() {
      return agendados;
    },
  };
}

test("vários compromissos dividem um temporizador só", () => {
  const f = falso();
  let a = 0;
  let b = 0;
  f.r.cada(1000, () => a++);
  f.r.cada(1500, () => b++);
  assert.equal(f.temporizadores(), 1);
  f.andar(3000);
  assert.equal(a, 3);
  assert.equal(b, 2);
});

test("com a janela escondida nada que consulta roda; o contador local continua", () => {
  const f = falso();
  let consulta = 0;
  let contador = 0;
  f.r.cada(1500, () => consulta++);
  f.r.cada(1000, () => contador++, { escondida: true });
  f.esconder(true);
  f.andar(6000);
  assert.equal(consulta, 0);
  assert.equal(contador, 6);
});

test("ao voltar, quem pediu relê na hora", () => {
  const f = falso();
  let lidas = 0;
  f.r.cada(5000, () => lidas++, { aoVoltar: true });
  f.esconder(true);
  f.andar(20000);
  assert.equal(lidas, 0);
  f.esconder(false);
  f.r.voltou();
  assert.equal(lidas, 1);
});

test("desligar o último compromisso apaga o temporizador", () => {
  const f = falso();
  const parar = f.r.cada(1000, () => {});
  assert.equal(f.r.ativos, 1);
  parar();
  assert.equal(f.r.ativos, 0);
  assert.equal(f.temporizadores(), 0);
});

test("depois de dormir, não dispara uma rajada atrasada", () => {
  const f = falso();
  let n = 0;
  f.r.cada(1000, () => n++);
  // A máquina dormiu 60 s: o temporizador acorda uma vez só, e o próximo fica a 1 s dali.
  f.dormir(60000);
  assert.equal(n, 1);
  f.andar(999);
  assert.equal(n, 1);
  f.andar(1);
  assert.equal(n, 2);
});

test("um compromisso que quebra não derruba os outros", () => {
  const f = falso();
  let ok = 0;
  const original = globalThis.queueMicrotask;
  globalThis.queueMicrotask = () => {};
  try {
    f.r.cada(1000, () => {
      throw new Error("quebrou");
    });
    f.r.cada(1000, () => ok++);
    f.andar(2000);
  } finally {
    globalThis.queueMicrotask = original;
  }
  assert.equal(ok, 2);
});
