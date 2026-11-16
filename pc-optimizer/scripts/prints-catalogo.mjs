// Prints do catálogo de cada jogo na aba Jogos (FiveM, CS2 e Fortnite da demonstração), nos temas escuro e claro, com o
// painel do catálogo rolado para o topo. Feito a partir do `prints-nova.mjs`.
//
// Uso: `npm run nova:build` e depois `node scripts/prints-catalogo.mjs [pasta]` (padrão: `prints-nova/`).

import { writeFile, mkdir } from "node:fs/promises";
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

const perfil = await mkdtemp(path.join(tmpdir(), "otimiza-prints-nova-"));
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


const SAIDA = path.resolve(process.argv[2] ?? path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "prints-nova"));
await mkdir(SAIDA, { recursive: true });

await cdp("Runtime.enable");
await cdp("Page.enable");
await cdp("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });

const avaliar = async (expressao) =>
  (await cdp("Runtime.evaluate", { expression: expressao, awaitPromise: true, returnByValue: true })).result?.result?.value;
const fotografar = async (nome) => {
  const r = await cdp("Page.captureScreenshot", { format: "png" });
  await writeFile(path.join(SAIDA, nome + ".png"), Buffer.from(r.result.data, "base64"));
};

for (const [modo, tema] of [["escuro", "escuro"], ["claro", "claro"]]) {
  await cdp("Page.navigate", { url: `http://127.0.0.1:${porta}/` });
  await espera(2000);
  await avaliar(`localStorage.setItem("otimiza.boas-vindas", "vista"); localStorage.setItem("otimiza.tema", "${tema}"); localStorage.setItem("otimiza.fundo", "")`);
  await cdp("Page.navigate", { url: `http://127.0.0.1:${porta}/` });
  await espera(2500);
  await avaliar(`document.querySelector('.fechar-balao')?.click()`);
  await avaliar(`document.querySelector('.trilho-botao[aria-label="Jogar"]').click()`);
  await espera(500);
  await avaliar(`[...document.querySelectorAll('nav.abas .aba')].find((b) => b.textContent.trim() === "Jogos").click()`);
  await espera(1500);
  for (const jogo of ["FiveM", "Counter-Strike 2", "Fortnite"]) {
    const achou = await avaliar(`(() => { const b = [...document.querySelectorAll('button.cartao')].find((x) => x.textContent.includes(${JSON.stringify(jogo)})); b?.click(); return !!b; })()`);
    await espera(1500);
    await avaliar(`document.querySelector('.catalogo')?.scrollIntoView({ block: "start" })`);
    await espera(500);
    await fotografar(`catalogo-${modo}-${jogo.replace(/[^A-Za-z0-9]+/g, "-")}`);
    console.log(modo, jogo, achou ? "ok" : "não achei o cartão");
  }
}
console.log(erros.length ? "erros: " + erros.join(" | ") : "sem erros no console");
navegador.kill();
servidor.close();
await rm(perfil, { recursive: true, force: true }).catch(() => {});
process.exit(0);
