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

/*
 * A ORIGEM DO SERVIÇO QUE COBRA, quando o checkout está ligado na compilação.
 * Sem ela no `connect-src`, a política bloquearia o próprio pagamento — e o
 * erro apareceria só no navegador do cliente, na hora de pagar.
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

/**
 * O formulário do cartão é do Mercado Pago: o SDK, os campos seguros em janela
 * própria e o antifraude falam com domínios do grupo Mercado Livre. Eles só
 * entram na política quando o cartão está ligado nesta compilação.
 */
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

/** De onde a página pode buscar cada coisa. */
function politica(hashes) {
  return [
    "default-src 'self'",
    `script-src ${junto("'self'", hashes.map((h) => `'sha256-${h}'`), cartao("script"))}`,
    // Estilo em linha é o preço da animação sem servidor.
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
