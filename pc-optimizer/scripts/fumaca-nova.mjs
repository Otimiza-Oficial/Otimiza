// Teste de fumaça da interface nova (Onda 2, "pronto quando o fumaca clica em todas as áreas"): abre a página de
// PRODUÇÃO (`dist-nova/`) num Chrome sem janela, com a ponte do Tauri trocada por uma que recusa tudo (o pior caso:
// nenhum comando do Windows responde), clica em cada aba, em cada área do "+" e em cada aba interna do Sistema, e
// falha se aparecer exceção não tratada ou se uma área abrir vazia. Erro que a tela mostra ao cliente não conta: é o
// esperado aqui.
//
// Uso: `npm run nova:build` e depois `node scripts/fumaca-nova.mjs` (ou `npm run fumaca:nova`, que faz os dois).
// Precisa do Google Chrome ou do Microsoft Edge instalado.

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { readFile, mkdtemp, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "dist-nova");
const NAVEGADOR = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
].find((c) => existsSync(c));

if (!existsSync(path.join(RAIZ, "index.html"))) {
  console.error("Falta a página de produção: rode `npm run nova:build` antes.");
  process.exit(2);
}
if (!NAVEGADOR) {
  console.error("Nem o Google Chrome nem o Microsoft Edge foram encontrados.");
  process.exit(2);
}

const TIPOS = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
};
const servidor = createServer(async (req, res) => {
  const pedido = decodeURIComponent(new URL(req.url, "http://x").pathname);
  const arquivo = path.join(RAIZ, pedido === "/" ? "index.html" : pedido);
  try {
    const corpo = await readFile(arquivo);
    res.writeHead(200, { "content-type": TIPOS[path.extname(arquivo)] ?? "application/octet-stream" });
    res.end(corpo);
  } catch {
    res.writeHead(404);
    res.end();
  }
});
await new Promise((r) => servidor.listen(0, "127.0.0.1", r));
const porta = servidor.address().port;

const perfil = await mkdtemp(path.join(tmpdir(), "otimiza-fumaca-nova-"));
const navegador = spawn(
  NAVEGADOR,
  ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${perfil}`, "--window-size=1440,900", "about:blank"],
  { stdio: ["ignore", "ignore", "pipe"] }
);
const endereco = await new Promise((resolver, rejeitar) => {
  let saida = "";
  navegador.stderr.on("data", (d) => {
    saida += d;
    const achado = saida.match(/DevTools listening on (ws:\/\/\S+)/);
    if (achado) resolver(achado[1]);
  });
  setTimeout(() => rejeitar(new Error("o navegador não abriu a porta de depuração")), 15000);
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
  if (d.id && pendentes.has(d.id)) {
    pendentes.get(d.id)(d);
    pendentes.delete(d.id);
  }
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
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };`,
});
// Primeira abertura: a apresentação aparece sozinha, com o botão de começar.
await cdp("Page.navigate", { url: `http://127.0.0.1:${porta}/` });
await espera(3000);
const primeira = await cdp("Runtime.evaluate", {
  expression: `!!document.querySelector('[role=dialog][aria-label="Boas-vindas ao Otimiza"]')`,
  returnByValue: true,
});
if (primeira.result?.result?.value !== true) erros.push("a apresentação não abriu na primeira abertura");
// Daqui em diante, como quem já viu a apresentação.
await cdp("Runtime.evaluate", { expression: `localStorage.setItem("otimiza.boas-vindas", "vista")` });
await cdp("Page.navigate", { url: `http://127.0.0.1:${porta}/` });
await espera(3000);

const r = await cdp("Runtime.evaluate", {
  expression: `(async () => {
    const pausa = (ms) => new Promise((r) => setTimeout(r, ms));
    const falhas = [];
    const visitadas = [];
    const conferirArea = (nome) => {
      const titulo = document.querySelector('.area-aberta h2')?.textContent.trim();
      if (titulo !== nome) falhas.push('clicou em ' + nome + ' e abriu ' + (titulo ?? 'nada'));
      const corpo = document.querySelector('.area-corpo')?.textContent.trim() ?? '';
      if (corpo.length < 20) falhas.push('a área ' + nome + ' abriu vazia');
    };

    // A tela inicial: os blocos com nome.
    const blocos = document.querySelectorAll('.grade .bloco').length;
    if (blocos < 7) falhas.push('a tela inicial montou só ' + blocos + ' blocos');
    visitadas.push('Início');

    // As abas de baixo.
    const abas = [...document.querySelectorAll('nav.abas .aba')].map((b) => b.textContent.trim());
    for (const nome of abas.slice(1)) {
      [...document.querySelectorAll('nav.abas .aba')].find((b) => b.textContent.trim() === nome).click();
      await pausa(700);
      conferirArea(nome);
      visitadas.push(nome);
    }

    // As áreas do "+".
    document.querySelector('.aba-mais').click();
    await pausa(300);
    const outras = [...document.querySelectorAll('.mais button')].map((b) => b.textContent.trim());
    if (outras.length < 8) falhas.push('o "+" mostrou só ' + outras.length + ' áreas');
    document.querySelector('.aba-mais').click();
    await pausa(200);
    for (const nome of outras) {
      document.querySelector('.aba-mais').click();
      await pausa(250);
      const b = [...document.querySelectorAll('.mais button')].find((x) => x.textContent.trim() === nome);
      if (!b) { falhas.push('o "+" perdeu ' + nome); continue; }
      b.click();
      await pausa(700);
      conferirArea(nome);
      visitadas.push(nome);

      // O Sistema tem abas internas: cada uma abre com conteúdo.
      if (nome === 'Sistema') {
        for (const t of [...document.querySelectorAll('.abas-internas [role=tab]')].map((x) => x.textContent.trim())) {
          [...document.querySelectorAll('.abas-internas [role=tab]')].find((x) => x.textContent.trim() === t).click();
          await pausa(500);
          const corpo = document.querySelector('.area-corpo')?.textContent.trim() ?? '';
          if (corpo.length < 40) falhas.push('Sistema/' + t + ' abriu vazia');
          visitadas.push('Sistema/' + t);
        }
      }
    }

    // O trilho da esquerda leva aos mesmos lugares.
    for (const b of document.querySelectorAll('.trilho-botao[aria-label]')) {
      if (b.getAttribute('aria-label') === 'Mais áreas') continue;
      b.click();
      await pausa(300);
    }
    document.querySelector('.trilho-botao[aria-label="Início"]')?.click();
    await pausa(400);
    if (document.querySelectorAll('.grade .bloco').length < 7) falhas.push('voltar ao Início pelo trilho não remontou a grade');

    // Com tudo recusado, nenhuma folha (feedback) pode ter aberto por cima.
    if (document.querySelector('.veu')) falhas.push('uma folha abriu por cima da tela sem motivo');
    return { visitadas, falhas };
  })()`,
  awaitPromise: true,
  returnByValue: true,
});
await espera(1000);

// Janela estreita (notebook, tela de 1024 ou escala de 125%): todo card da tela inicial cabe na largura e tem tamanho
// de card. Uma regra de layout antiga já escondeu três deles numa janela abaixo de 1100 px.
await cdp("Emulation.setDeviceMetricsOverride", { width: 1000, height: 720, deviceScaleFactor: 1, mobile: false });
await espera(800);
const estreita = await cdp("Runtime.evaluate", {
  expression: `(() => {
    const largura = document.querySelector('.conteudo')?.getBoundingClientRect();
    return [...document.querySelectorAll('.grade > .bloco')].flatMap((b) => {
      const r = b.getBoundingClientRect();
      const nome = [...b.classList].find((c) => c.startsWith('b-')) ?? 'bloco';
      if (!largura || r.right > largura.right + 1 || r.left < largura.left - 1) return ['janela de 1000 px: o card ' + nome + ' sai da largura'];
      if (r.width < 200 || r.height < 120) return ['janela de 1000 px: o card ' + nome + ' ficou com ' + Math.round(r.width) + '×' + Math.round(r.height)];
      return [];
    });
  })()`,
  returnByValue: true,
});
erros.push(...(estreita.result?.result?.value ?? ["a conferência da janela estreita não retornou"]));

if (r.result?.exceptionDetails) erros.push(r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text);
const resultado = r.result?.result?.value;
erros.push(...(resultado?.falhas ?? ["o roteiro da interface nova não retornou"]));
console.log(`Interface nova (${resultado?.visitadas?.length ?? 0}): ${(resultado?.visitadas ?? []).join(", ")}`);

ws.close();
navegador.kill();
servidor.close();
await espera(300);
await rm(perfil, { recursive: true, force: true }).catch(() => {});

if (erros.length) {
  console.error(`\n${erros.length} problema(s):`);
  for (const e of erros) console.error(" -", e);
  process.exit(1);
}
console.log("Nenhuma exceção e nenhuma área vazia.");
