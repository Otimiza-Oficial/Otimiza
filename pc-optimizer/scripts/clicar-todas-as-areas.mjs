// Teste de fumaça da interface (A3.2, QA): abre a página de PRODUÇÃO num Chrome sem janela, com a ponte do Tauri
// trocada por uma que recusa tudo (o pior caso: nenhum comando responde), clica em cada área e em cada sub-aba, e
// falha se aparecer qualquer exceção não tratada (erro que a tela mostra ao cliente não conta: é o esperado aqui). Foi uma aba que derrubava o programa (a BIOS, 3.1) que
// motivou o teste: toda tela precisa abrir mesmo sem resposta do Windows.
//
// Uso: `npm run build` e depois `node scripts/clicar-todas-as-areas.mjs`. Precisa do Google Chrome instalado.

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { readFile, mkdtemp, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "dist");
const CHROME = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
].find((c) => existsSync(c));

if (!existsSync(path.join(RAIZ, "index.html"))) {
  console.error("Falta a página de produção: rode `npm run build` antes.");
  process.exit(2);
}
if (!CHROME) {
  console.error("Google Chrome não encontrado.");
  process.exit(2);
}

const TIPOS = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2" };
const servidor = createServer(async (req, res) => {
  const pedido = decodeURIComponent(new URL(req.url, "http://x").pathname);
  const arquivo = path.join(RAIZ, pedido === "/" ? "index.html" : pedido);
  let corpo;
  try {
    corpo = await readFile(arquivo);
  } catch {
    res.writeHead(404);
    res.end();
    return;
  }
  res.writeHead(200, { "content-type": TIPOS[path.extname(arquivo)] ?? "application/octet-stream" });
  res.end(corpo);
});
await new Promise((r) => servidor.listen(0, "127.0.0.1", r));
const porta = servidor.address().port;

const perfil = await mkdtemp(path.join(tmpdir(), "otimiza-fumaca-"));
const chrome = spawn(CHROME, ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${perfil}`, "--window-size=1400,1000", "about:blank"], { stdio: ["ignore", "ignore", "pipe"] });
const endereco = await new Promise((resolver, rejeitar) => {
  let saida = "";
  chrome.stderr.on("data", (d) => {
    saida += d;
    const achado = saida.match(/DevTools listening on (ws:\/\/\S+)/);
    if (achado) resolver(achado[1]);
  });
  setTimeout(() => rejeitar(new Error("o Chrome não abriu a porta de depuração")), 15000);
});
const base = new URL(endereco);
const alvos = await (await fetch(`http://${base.host}/json`)).json();
const ws = new WebSocket(alvos.find((t) => t.type === "page").webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));

let proximo = 0;
const pendentes = new Map();
const erros = [];
ws.onmessage = (m) => {
  const d = JSON.parse(m.data);
  if (d.id && pendentes.has(d.id)) pendentes.get(d.id)(d);
  if (d.method === "Runtime.exceptionThrown") {
    erros.push(d.params.exceptionDetails.exception?.description ?? d.params.exceptionDetails.text);
  }
};
const cdp = (method, params = {}) =>
  new Promise((r) => {
    const n = ++proximo;
    pendentes.set(n, r);
    ws.send(JSON.stringify({ id: n, method, params }));
  });
const espera = (ms) => new Promise((r) => setTimeout(r, ms));

await cdp("Runtime.enable");
await cdp("Page.enable");
await cdp("Page.addScriptToEvaluateOnNewDocument", {
  source: `
    window.__TAURI_INTERNALS__ = {
      transformCallback: (f) => { const k = Math.random(); window['_' + k] = f; return k; },
      invoke: async (cmd) => {
        if (cmd.startsWith('plugin:event')) return 1;
        // O pior caso: nenhum comando do Windows responde. A tela precisa aguentar.
        throw 'sem resposta do Windows (teste de fumaça)';
      },
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
    };`,
});
await cdp("Page.navigate", { url: `http://127.0.0.1:${porta}/` });
await espera(3000);

const telas = await cdp("Runtime.evaluate", {
  expression: `(async () => {
    const pausa = (ms) => new Promise((r) => setTimeout(r, ms));
    const visitadas = [];
    for (const area of [...document.querySelectorAll('.nav[data-tab]')]) {
      area.click();
      await pausa(400);
      visitadas.push(area.dataset.tab);
      // A barra é redesenhada a cada clique: procura o botão de novo pelo id da tela.
      const ids = [...document.querySelectorAll('.subnav-item[data-subtela]')].map((b) => b.dataset.subtela);
      for (const id of ids) {
        const sub = document.querySelector('.subnav-item[data-subtela="' + id + '"]');
        sub.click();
        await pausa(400);
        visitadas.push(area.dataset.tab + '/' + sub.dataset.subtela);
        const aberta = [...document.querySelectorAll('.tab-panel')].filter((p) => !p.hidden).map((p) => p.id);
        if (aberta.length !== 1 || aberta[0] !== 'tab-' + id) {
          throw new Error('a sub-aba ' + id + ' abriu ' + aberta.join(','));
        }
      }
    }
    return visitadas;
  })()`,
  awaitPromise: true,
  returnByValue: true,
});
await espera(1500);

ws.close();
chrome.kill();
servidor.close();
await rm(perfil, { recursive: true, force: true }).catch(() => {});

const falhaDoRoteiro = telas.result?.exceptionDetails?.exception?.description;
const visitadas = Array.isArray(telas.result?.result?.value) ? telas.result.result.value : [];
console.log(`Telas visitadas (${visitadas.length}): ${visitadas.join(", ")}`);
if (falhaDoRoteiro) erros.push(falhaDoRoteiro);
if (visitadas.length < 5) erros.push(`só ${visitadas.length} telas visitadas`);

if (erros.length) {
  console.error(`\n${erros.length} erro(s):\n` + erros.map((e) => ` - ${e}`).join("\n"));
  process.exit(1);
}
console.log("Nenhum erro: toda tela abre sem resposta do Windows.");
