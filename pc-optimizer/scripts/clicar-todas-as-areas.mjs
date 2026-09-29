// Teste de fumaça da interface (A3.2, QA): abre a página de PRODUÇÃO num Chrome sem janela, com a ponte do Tauri
// trocada por uma que recusa tudo (o pior caso: nenhum comando responde), clica em cada área e em cada sub-aba, e
// falha se aparecer qualquer exceção não tratada (erro que a tela mostra ao cliente não conta: é o esperado aqui). Foi uma aba que derrubava o programa (a BIOS, 3.1) que
// motivou o teste: toda tela precisa abrir mesmo sem resposta do Windows.
//
// Uso: `npm run build` e depois `node scripts/clicar-todas-as-areas.mjs`. Precisa do Google Chrome instalado.

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { readFile, mkdtemp, rm, mkdir, writeFile } from "node:fs/promises";
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

// Regressões de interação: a navegação deve preservar os alvos do teclado.
const motion = await cdp("Runtime.evaluate", {
  expression: `(async () => {
    const falhas = [];
    const conferir = (ok, mensagem) => { if (!ok) falhas.push(mensagem); };
    // Só no navegador de teste, sem licença/backend: deixa a UI receber foco.
    document.querySelector('#portao').hidden = true;
    document.querySelector('.console').removeAttribute('inert');
    document.querySelector('.nav[data-tab="sistema"]').click();
    const aba = document.querySelector('[data-subtela="energia"]');
    aba.focus(); aba.click();
    conferir(aba.isConnected && document.activeElement === aba, 'subaba perdeu identidade/foco');
    const abrir = document.querySelector('#abrir-comandos-topo');
    abrir.focus(); abrir.click();
    const primeira = document.querySelector('.comando');
    document.querySelector('#comandos-busca').dispatchEvent(new KeyboardEvent('keydown', {key:'ArrowDown', bubbles:true}));
    conferir(primeira.isConnected, 'seta reconstruiu lista de comandos');
    document.dispatchEvent(new KeyboardEvent('keydown', {key:'Escape', bubbles:true}));
    conferir(document.activeElement === abrir, 'fechar paleta não restaurou foco');
    for (let i = 0; i < 12; i++) {
      abrir.click();
      document.dispatchEvent(new KeyboardEvent('keydown', {key:'Escape', bubbles:true}));
    }
    conferir(document.querySelector('#comandos').hidden, 'paleta ficou aberta após interrupções');
    await new Promise(r => setTimeout(r, 350));
    conferir(getComputedStyle(document.querySelector('#comandos')).display === 'none', 'overlay fechado intercepta interface');
    return falhas;
  })()`,
  awaitPromise: true,
  returnByValue: true,
});
erros.push(...(motion.result?.result?.value ?? ["roteiro de motion não retornou"]));
if (motion.result?.exceptionDetails) erros.push(motion.result.exceptionDetails.text);

const avaliar = async (expression) => {
  const r = await cdp("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.result?.exceptionDetails) erros.push(r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text);
  return r.result?.result?.value;
};

// Frames reais entre abrir/fechar/reabrir, além do teste síncrono acima.
const interrupcoes = await avaliar(`(async () => {
  const pausa = ms => new Promise(r => setTimeout(r, ms));
  const abrir = document.querySelector('#abrir-comandos-topo');
  const caixa = document.querySelector('#comandos');
  for (let i = 0; i < 5; i++) {
    abrir.click(); await pausa(35);
    document.dispatchEvent(new KeyboardEvent('keydown', {key:'Escape', bubbles:true}));
    await pausa(35);
  }
  abrir.click(); await pausa(220);
  const ok = !caixa.hidden && !caixa.inert && getComputedStyle(caixa).opacity === '1';
  const inicioDaLista = document.querySelector('#comandos-lista').scrollTop === 0;
  const campoInteiro = document.querySelector('#comandos-busca').getBoundingClientRect().height >= 48;
  const ultimo = caixa.querySelector('.comando:last-child');
  ultimo.focus();
  ultimo.dispatchEvent(new KeyboardEvent('keydown', {key:'Tab', bubbles:true, cancelable:true}));
  const foco = document.activeElement === document.querySelector('#comandos-busca');
  document.dispatchEvent(new KeyboardEvent('keydown', {key:'Escape', bubbles:true}));
  return ok && foco && inicioDaLista && campoInteiro;
})()`);
if (!interrupcoes) erros.push("interrupção/foco do overlay falhou");

await cdp("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
await espera(100);
const reducao = await avaliar(`document.body.classList.contains('sem-animacao') &&
  getComputedStyle(document.querySelector('.tab-panel:not([hidden])')).animationName === 'none'`);
if (!reducao) erros.push("reduced motion não atualizou com o app aberto");
await cdp("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "no-preference" }] });
await espera(100);

// Stress visual isolado: não chama comandos de otimização nem modifica dados do Windows.
const stress = await avaliar(`(async () => {
  const host = document.createElement('div');
  host.style.cssText = 'position:fixed;inset:100px 80px;overflow:auto;z-index:100;background:var(--panel)';
  host.innerHTML = Array.from({length:1000}, (_,i) => '<div class="startup">Item ' + i + '</div>').join('');
  document.body.append(host);
  const animados = host.getAnimations({subtree:true}).length;
  host.scrollTop = host.scrollHeight;
  await new Promise(requestAnimationFrame);
  host.remove();
  const inicio = performance.now();
  for (let i=0; i<20; i++) {
    document.querySelector('.nav[data-tab="historico"]').click();
    document.querySelector('.nav[data-tab="sistema"]').click();
  }
  const ms = performance.now() - inicio;
  await new Promise(r => setTimeout(r, 250));
  return {animados, navegacoes:40, ms, paineis:document.querySelectorAll('.tab-panel:not([hidden])').length};
})()`);
console.log("Stress motion:", JSON.stringify(stress));
if (!stress || stress.animados > 8 || stress.paineis !== 1) erros.push("lista/navegação excedeu orçamento de animações");

if (process.env.MOTION_QA_DIR) {
  const pasta = path.resolve(process.env.MOTION_QA_DIR);
  await mkdir(pasta, { recursive: true });
  for (const [nome, largura, altura, paleta] of [["desktop", 1400, 1000, false], ["compacto", 900, 650, false], ["paleta", 900, 650, true]]) {
    await cdp("Emulation.setDeviceMetricsOverride", { width: largura, height: altura, deviceScaleFactor: 1, mobile: false });
    if (paleta) await avaliar(`document.querySelector('#abrir-comandos-topo').click()`);
    await espera(250);
    const captura = await cdp("Page.captureScreenshot", { format: "png" });
    await writeFile(path.join(pasta, nome + ".png"), Buffer.from(captura.result.data, "base64"));
  }
  await avaliar(`document.dispatchEvent(new KeyboardEvent('keydown', {key:'Escape', bubbles:true}))`);
  // Registra todas as páginas finais, incluindo os estados de erro de backend.
  for (const tela of ['painel', 'otimizacoes', 'jogos', 'framegen', 'gpu', 'energia', 'nucleos', 'bios', 'historico']) {
    await avaliar(`window.dispatchEvent(new CustomEvent('otimiza:ir', {detail:${JSON.stringify(tela)}}))`);
    await espera(230);
    const captura = await cdp("Page.captureScreenshot", { format: "png" });
    await writeFile(path.join(pasta, tela + ".png"), Buffer.from(captura.result.data, "base64"));
  }
  console.log("Capturas:", pasta);
}

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
