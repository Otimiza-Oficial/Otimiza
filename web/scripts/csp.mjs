/*
 * A POLÍTICA DE CONTEÚDO (CSP) DO SITE, CARIMBADA DEPOIS DO BUILD.
 *
 * POR QUE NÃO É UM CABEÇALHO
 *
 * O site é estático no GitHub Pages, e o Pages não deixa configurar cabeçalho
 * nenhum. O que resta é a `<meta http-equiv="Content-Security-Policy">`, que o
 * navegador aplica ao documento inteiro. Duas diretivas NÃO funcionam por meta
 * e por isso não estão aqui: `frame-ancestors` e `report-uri`. Enquanto o site
 * for servido pelo Pages, quem quiser proteção contra enquadramento precisa de
 * um domínio próprio com cabeçalho — está escrito no README do site.
 *
 * POR QUE OS HASHES, E NÃO `'unsafe-inline'`
 *
 * O Next põe scripts em linha em toda página (é assim que ele entrega o estado
 * da renderização). Liberar `'unsafe-inline'` no `script-src` seria escrever a
 * política e desligá-la na mesma linha: qualquer script injetado passaria a
 * valer. Então este passo lê cada HTML compilado, calcula o SHA-256 de cada
 * script em linha DAQUELA página e escreve os hashes na política dela. Um
 * script que não estava lá na compilação não roda.
 *
 * O `style-src` continua com `'unsafe-inline'`, e isso é honesto dizer: React e
 * Framer Motion escrevem `style="..."` no elemento o tempo todo — animação é
 * isso. Sem servidor não há nonce, e proibir estilo em linha quebraria a
 * página. O ganho que importa (script) está preservado.
 *
 * COMO CONFERIR: `node scripts/csp.mjs out` e abrir o site servido de `out/`
 * com o console aberto. Violação aparece lá, em vermelho, dizendo a diretiva.
 */
import { createHash } from "node:crypto";
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const RAIZ = process.argv[2] ?? "out";

/** De onde a página pode buscar cada coisa. */
function politica(hashes) {
  const script = ["'self'", ...hashes.map((h) => `'sha256-${h}'`)].join(" ");
  return [
    "default-src 'self'",
    `script-src ${script}`,
    // Ver o cabeçalho: estilo em linha é o preço da animação sem servidor.
    "style-src 'self' 'unsafe-inline'",
    // As fotos das telas ilustrativas vêm recortadas do Unsplash; `data:` é o
    // que o Next usa para imagem embutida pequena.
    "img-src 'self' data: https://images.unsplash.com",
    // O `next/font` baixa a fonte na compilação e a serve daqui.
    "font-src 'self'",
    // A única chamada que o site faz: a versão mais nova, na API pública do
    // GitHub. Nada da máquina do cliente sai daqui.
    "connect-src 'self' https://api.github.com",
    "form-action 'self'",
    "base-uri 'self'",
    "object-src 'none'",
    "frame-src 'none'",
    "worker-src 'self'",
    "manifest-src 'self'",
    "upgrade-insecure-requests",
  ].join("; ");
}

/** Os scripts EM LINHA de um HTML (os que têm `src` vêm do próprio site). */
function scriptsEmLinha(html) {
  const achados = [];
  const re = /<script([^>]*)>([\s\S]*?)<\/script>/gi;
  let m;
  while ((m = re.exec(html)) !== null) {
    const atributos = m[1] ?? "";
    if (/\ssrc\s*=/i.test(atributos)) continue;
    achados.push(m[2]);
  }
  return achados;
}

const sha256 = (texto) => createHash("sha256").update(texto, "utf8").digest("base64");

async function* htmls(dir) {
  for (const entrada of await readdir(dir, { withFileTypes: true })) {
    const caminho = path.join(dir, entrada.name);
    if (entrada.isDirectory()) yield* htmls(caminho);
    else if (entrada.name.endsWith(".html")) yield caminho;
  }
}

let paginas = 0;
let inline = 0;

for await (const arquivo of htmls(RAIZ)) {
  const html = await readFile(arquivo, "utf8");
  if (html.includes("http-equiv=\"Content-Security-Policy\"")) continue;

  const hashes = [...new Set(scriptsEmLinha(html).map(sha256))];
  const meta =
    `<meta http-equiv="Content-Security-Policy" content="${politica(hashes)}">` +
    `<meta name="referrer" content="strict-origin-when-cross-origin">`;

  // Primeiro filho do `<head>`: a política precisa valer para tudo que vem
  // depois dela — inclusive para o primeiro script da página.
  const marcado = html.replace(/<head([^>]*)>/i, (tag) => `${tag}${meta}`);
  if (marcado === html) {
    console.error(`csp: ${arquivo} não tem <head>; nada foi escrito`);
    process.exitCode = 1;
    continue;
  }

  await writeFile(arquivo, marcado, "utf8");
  paginas += 1;
  inline += hashes.length;
}

console.log(`csp: política escrita em ${paginas} página(s), ${inline} script(s) em linha com hash.`);
