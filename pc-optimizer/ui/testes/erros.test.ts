import { test } from "node:test";
import assert from "node:assert/strict";

import { explicarErro, mensagemDe } from "../src/lib/erros.ts";

test("a frase do Rust que já diz o que fazer passa como está", () => {
  assert.equal(explicarErro("Abra o jogo para guardar o perfil dele."), "Abra o jogo para guardar o perfil dele.");
  assert.equal(
    explicarErro("A calibração troca o plano de energia: abra o Otimiza como administrador."),
    "A calibração troca o plano de energia: abra o Otimiza como administrador."
  );
});

test("a frase que só diz o que aconteceu ganha o próximo passo", () => {
  const t = explicarErro("Já tem uma calibração rodando");
  assert.match(t, /^Já tem uma calibração rodando\. /);
  assert.match(t, /Tente de novo/);
  assert.match(t, /Versão e suporte/);
});

test("o erro cru do Windows vira português com o que fazer", () => {
  assert.equal(
    explicarErro(new Error("Access is denied. (os error 5)")),
    "O Windows não deixou: precisa de administrador. Abra o Otimiza como administrador (botão direito no atalho › Executar como administrador) e tente de novo."
  );
  assert.match(explicarErro("The process cannot access the file because it is being used by another process. (os error 32)"), /Feche o jogo/);
  assert.match(explicarErro("operation timed out"), /demorou demais/);
});

test("erro vazio ainda diz alguma coisa útil", () => {
  assert.match(explicarErro(null), /^Algo deu errado\. Tente de novo/);
  assert.match(explicarErro("Error: "), /^Algo deu errado/);
});

test("tira o prefixo técnico e põe ponto final", () => {
  assert.match(explicarErro("Error: sem resposta do Windows"), /^Sem resposta do Windows\. /);
});

test("lê a mensagem de qualquer formato de erro", () => {
  assert.equal(mensagemDe(new Error("x")), "x");
  assert.equal(mensagemDe({ message: "y" }), "y");
  assert.equal(mensagemDe("z"), "z");
});

test("nenhuma tela mostra o erro cru: passa pelo explicarErro (ou vai só para o detalhe do suporte)", async () => {
  const { readdirSync, readFileSync } = await import("node:fs");
  const path = await import("node:path");
  const { fileURLToPath } = await import("node:url");
  const pasta = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "src", "componentes");
  // Estes guardam a mensagem crua só para o "Detalhe para o suporte" de um `Estado`, ou leem o código do Rust.
  const DETALHE = new Set(["FrameBoost.svelte", "Jogos.svelte", "Prova.svelte", "OtimizarAgora.svelte", "MedicaoGuiada.svelte", "CatalogoDoJogo.svelte", "AtivarLicenca.svelte"]);
  const achados: string[] = [];
  for (const nome of readdirSync(pasta).filter((n: string) => n.endsWith(".svelte") && !DETALHE.has(n)))
    readFileSync(path.join(pasta, nome), "utf8")
      .split("\n")
      .forEach((l: string, i: number) => {
        if (/String\((e|erro|err)\)/.test(l) && !/detalhe|anotar|const t = String\(e\)/.test(l)) achados.push(`${nome}:${i + 1}`);
      });
  assert.deepEqual(achados, []);
});
