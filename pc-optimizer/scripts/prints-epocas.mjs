// Prints de cada época do ano (Natal, Ano Novo, Carnaval, Páscoa, Festa Junina, Dia das Crianças e Halloween), nos temas
// escuro e claro, da página de PRODUÇÃO (`dist-nova/`) com a demonstração: o Início e uma área (Turbo), com `?temporada=`.
// Feito a partir do `prints-telas.mjs`. Não falha: só grava, e lista as exceções da página no fim.
//
// Uso: `npm run nova:build` e depois `node scripts/prints-epocas.mjs [pasta] [largura] [altura]` (padrão: `prints-epocas/`).

import { writeFile, mkdir, readFile, mkdtemp, rm } from "node:fs/promises";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const AQUI = path.dirname(fileURLToPath(import.meta.url));
const RAIZ = path.resolve(process.env.OTIMIZA_DIST ?? path.join(AQUI, "..", "dist-nova"));
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

const TIPOS = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".png": "image/png", ".jpg": "image/jpeg", ".woff2": "font/woff2" };
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

const perfil = await mkdtemp(path.join(tmpdir(), "otimiza-prints-telas-"));
const navegador = spawn(NAVEGADOR, ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${perfil}`, "--window-size=1440,900", "about:blank"], {
  stdio: ["ignore", "ignore", "pipe"],
});
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
  if (d.method === "Runtime.exceptionThrown") erros.push(d.params.exceptionDetails.exception?.description ?? d.params.exceptionDetails.text);
};
const cdp = (method, params = {}) =>
  new Promise((r) => {
    const n = ++proximo;
    pendentes.set(n, r);
    ws.send(JSON.stringify({ id: n, method, params }));
  });
const espera = (ms) => new Promise((r) => setTimeout(r, ms));

const SAIDA = path.resolve(process.argv[2] ?? path.join(AQUI, "..", "prints-epocas"));
await mkdir(SAIDA, { recursive: true });

await cdp("Runtime.enable");
await cdp("Page.enable");
await cdp("Emulation.setDeviceMetricsOverride", { width: Number(process.argv[3] ?? 1280), height: Number(process.argv[4] ?? 800), deviceScaleFactor: 1, mobile: false });

const avaliar = async (expressao) =>
  (await cdp("Runtime.evaluate", { expression: expressao, awaitPromise: true, returnByValue: true })).result?.result?.value;
let total = 0;
const fotografar = async (nome) => {
  const r = await cdp("Page.captureScreenshot", { format: "png" });
  await writeFile(path.join(SAIDA, nome + ".png"), Buffer.from(r.result.data, "base64"));
  total++;
};
const clicarTexto = (seletor, texto) =>
  avaliar(`(() => { const el = [...document.querySelectorAll(${JSON.stringify(seletor)})].find((b) => b.textContent.trim().startsWith(${JSON.stringify(texto)})); el?.click(); return !!el; })()`);
const abrir = async (grupo, area) => {
  await avaliar(`document.querySelector('.trilho-botao[aria-label="${grupo}"]')?.click()`);
  await espera(400);
  await clicarTexto("nav.abas .aba", area);
  await espera(1200);
};
const rolarAte = (seletor) => avaliar(`document.querySelector(${JSON.stringify(seletor)})?.scrollIntoView({ block: "start" })`);

const EPOCAS = ["natal", "ano-novo", "carnaval", "pascoa", "festa-junina", "dia-das-criancas", "halloween"];
for (const tema of ["escuro", "claro"]) {
  await cdp("Page.navigate", { url: `http://127.0.0.1:${porta}/` });
  await espera(1200);
  await avaliar(`localStorage.clear(); localStorage.setItem("otimiza.boas-vindas", "vista"); localStorage.setItem("otimiza.tema", "${tema}"); localStorage.setItem("otimiza.fundo", "limpo")`);
  for (const epoca of EPOCAS) {
    await cdp("Page.navigate", { url: `http://127.0.0.1:${porta}/?temporada=${epoca}` });
    await espera(2600);
    await avaliar(`document.querySelector('.fechar-balao')?.click()`);
    await espera(400);
    await fotografar(`${tema}-${epoca}-inicio`);
    await abrir("Jogar", "Turbo");
    await fotografar(`${tema}-${epoca}-turbo`);
  }
  // A prévia nos Ajustes, escolhida pelo próprio seletor (não pela URL).
  await cdp("Page.navigate", { url: `http://127.0.0.1:${porta}/` });
  await espera(2200);
  await abrir("Ajustes", "Ajustes");
  await clicarTexto(".epoca-op", "Halloween");
  await espera(900);
  await rolarAte(".previa-da-epoca");
  await espera(300);
  await fotografar(`${tema}-ajustes-previa-da-epoca`);
  await clicarTexto(".epoca-op", "Pela data");
}

console.log(`${total} prints em ${SAIDA}${erros.length ? ` · ${erros.length} exceções: ${erros.slice(0, 3).join(" | ")}` : ""}`);
ws.close();
navegador.kill();
servidor.close();
await rm(perfil, { recursive: true, force: true }).catch(() => {});
