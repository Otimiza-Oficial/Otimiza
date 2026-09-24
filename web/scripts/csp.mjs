/*
 * A CSP do site, carimbada depois do build como <meta> (o GitHub Pages não deixa mandar cabeçalho).
 * Por meta não valem frame-ancestors nem report-uri; o enquadramento é tratado por public/moldura.js.
 * Script em linha entra por hash SHA-256 de cada página, nunca por 'unsafe-inline'. style-src tem 'unsafe-inline'
 * porque React e Framer Motion escrevem style no elemento. Conferir: node scripts/csp.mjs out e abrir out/ com o console.
 */
import { createHash } from "node:crypto";
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const RAIZ = process.argv[2] ?? "out";

/*
 * A origem da API de compra: sem ela no connect-src, a política bloquearia o próprio pagamento.
 */
const API_DA_COMPRA = (() => {
  const bruto = process.env.NEXT_PUBLIC_OTIMIZA_API ?? "";
  if (!bruto) return "";
  try {
    return new URL(bruto).origin;
  } catch {
    console.error(`csp: NEXT_PUBLIC_OTIMIZA_API não é um endereço válido: ${bruto}`);
    process.exitCode = 1;
    return "";
  }
})();

export const COM_CARTAO = Boolean((process.env.NEXT_PUBLIC_MP_PUBLIC_KEY ?? "").trim()) && Boolean(API_DA_COMPRA);

export const DOMINIOS_DO_CARTAO = {
  script: ["https://sdk.mercadopago.com", "https://http2.mlstatic.com"],
  estilo: ["https://http2.mlstatic.com"],
  fonte: ["https://http2.mlstatic.com"],
  imagem: ["https://*.mlstatic.com", "https://*.mercadopago.com"],
  conexao: ["https://*.mercadopago.com", "https://*.mercadolibre.com", "https://*.mercadolivre.com", "https://*.mlstatic.com"],
  janela: ["https://*.mercadopago.com", "https://*.mercadolibre.com", "https://*.mercadolivre.com"],
};

const cartao = (tipo) => (COM_CARTAO ? DOMINIOS_DO_CARTAO[tipo] : []);
const junto = (...partes) => partes.flat().filter(Boolean).join(" ");

function politica(hashes) {
  return [
    "default-src 'self'",
    `script-src ${junto("'self'", hashes.map((h) => `'sha256-${h}'`), cartao("script"))}`,
    `style-src ${junto("'self'", "'unsafe-inline'", cartao("estilo"))}`,
    `img-src ${junto("'self'", "data:", "https://images.unsplash.com", cartao("imagem"))}`,
    `font-src ${junto("'self'", cartao("fonte"))}`,
    `connect-src ${junto("'self'", "https://api.github.com", API_DA_COMPRA, cartao("conexao"))}`,
    "form-action 'self'",
    "base-uri 'self'",
    "object-src 'none'",
    `frame-src ${COM_CARTAO ? junto(cartao("janela")) : "'none'"}`,
    "worker-src 'self'",
    "manifest-src 'self'",
    "upgrade-insecure-requests",
  ].join("; ");
}

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

  // Primeiro filho do <head>: a política vale para tudo que vem depois, inclusive o primeiro script.
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
