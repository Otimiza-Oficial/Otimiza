/**
 * Emissor de licença do Otimiza — para o bot do Discord.
 *
 * ISTO NÃO SE CONECTA AO OTIMIZA. E é de propósito.
 *
 * O Otimiza não tem camada de rede: zero dependências HTTP, nenhuma porta
 * aberta, nenhuma chamada para fora. Um otimizador que fala com um servidor é
 * um otimizador que pode mandar dados da máquina do cliente para algum lugar, e
 * o cliente não tem como saber. Abrir esse caminho para automatizar venda seria
 * trocar a coisa mais defensável do produto por comodidade.
 *
 * O que conecta os dois é a MATEMÁTICA, não a rede. Este arquivo assina com a
 * mesma chave privada e no mesmo formato que o Otimiza confere. Ele podia rodar
 * em Marte: a chave que sai daqui abre lá.
 *
 * ZERO DEPENDÊNCIAS. O Node faz Ed25519 nativo desde a versão 12. Um emissor de
 * licença é a última coisa do mundo que deveria puxar biblioteca de terceiros:
 * cada pacote no meio é mais alguém com acesso potencial à sua chave privada.
 *
 * POR QUE `.cjs` E NÃO `.js`
 *
 * O `package.json` do projeto declara `"type": "module"`, o que faz todo `.js`
 * dentro dele ser tratado como módulo ES — e aí `require` não existe. A
 * extensão `.cjs` diz explicitamente "isto é CommonJS", que é o que a maioria
 * dos bots de Discord usa. Um bot em módulo ES também consegue:
 * `import { emitir } from "./otimiza-licenca.cjs"`.
 *
 * COMO USAR NO BOT
 *
 *     const { emitir } = require("./otimiza-licenca.cjs");
 *
 *     const chave = emitir({
 *       privada: process.env.OTIMIZA_CHAVE_PRIVADA,
 *       maquina: "OTZ-WPYY-0J4F-77AB",   // o cliente copia da tela do Otimiza
 *       comprador: "fulano#1234",
 *     });
 *
 * A chave privada vem de variável de ambiente, nunca escrita no código do bot.
 * Se o repositório do bot vazar, a chave não vai junto.
 *
 * COMO USAR NA MÃO
 *
 *     node otimiza-licenca.cjs emitir <PRIVADA> OTZ-XXXX-XXXX-XXXX "Nome" [AAAA-MM-DD]
 *     node otimiza-licenca.cjs novo-par
 */

"use strict";

const crypto = require("crypto");

// Os prefixos que transformam 32 bytes crus numa chave que o Node aceita.
//
// O Node trabalha com chave em DER; o Otimiza e o bot trocam 32 bytes em
// base64, que é o formato curto que cabe numa mensagem. Estes cabeçalhos fixos
// fazem a ponte: são a mesma sequência para toda chave Ed25519, e é por isso
// que podem ser constantes.
const DER_PRIVADA = Buffer.from("302e020100300506032b657004220420", "hex");
const DER_PUBLICA = Buffer.from("302a300506032b6570032100", "hex");

/** Cria um par de chaves. Roda UMA vez na vida do produto. */
function novoPar() {
  const { publicKey, privateKey } = crypto.generateKeyPairSync("ed25519");

  return {
    publica: publicKey.export({ type: "spki", format: "der" }).subarray(12).toString("base64"),
    privada: privateKey.export({ type: "pkcs8", format: "der" }).subarray(16).toString("base64"),
  };
}

/** A forma do código de máquina que o Otimiza mostra na tela. */
const FORMA_DA_MAQUINA = /^OTZ-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/;

/**
 * Emite uma licença.
 *
 * @param {object} pedido
 * @param {string} pedido.privada    Chave privada em base64, 32 bytes.
 * @param {string} pedido.maquina    Código OTZ-XXXX-XXXX-XXXX que o cliente copiou.
 * @param {string} pedido.comprador  Quem comprou. Só para você saber de quem é.
 * @param {string} [pedido.expira]   "AAAA-MM-DD". Sem isto, a licença é vitalícia.
 * @param {string} [pedido.emitida]  "AAAA-MM-DD". Padrão: hoje.
 * @param {string} [pedido.publica]  Se vier, a chave é conferida contra ela
 *   antes de sair. Ver "O par trocado" abaixo.
 * @returns {string} A chave, para mandar ao cliente.
 */
function emitir({ privada, maquina, comprador, expira = null, emitida = null, publica = null }) {
  if (!privada) {
    throw new Error(
      "Faltou a chave privada. No bot ela vem de variável de ambiente " +
        "(OTIMIZA_CHAVE_PRIVADA), nunca escrita no código."
    );
  }

  const bruta = Buffer.from(String(privada).trim(), "base64");

  if (bruta.length !== 32) {
    throw new Error(
      `A chave privada tem ${bruta.length} bytes; o esperado são 32. ` +
        "Confira se copiou a linha inteira, e se não trocou a PRIVADA pela PÚBLICA."
    );
  }

  const codigo = String(maquina || "").trim().toUpperCase();

  // Conferir aqui é o que impede uma chave inútil de ser vendida. Um erro de
  // digitação no código da máquina só apareceria do outro lado, com o cliente
  // já tendo pago e o Otimiza dizendo "esta chave é de outro computador".
  if (!FORMA_DA_MAQUINA.test(codigo)) {
    throw new Error(
      `"${maquina}" não tem a forma de um código de máquina. Ele é ` +
        "OTZ-XXXX-XXXX-XXXX e o cliente copia direto da tela do Otimiza."
    );
  }

  const nome = String(comprador || "").trim();

  if (!nome) {
    throw new Error("Faltou o nome do comprador.");
  }

  for (const [rotulo, data] of [["emissão", emitida], ["validade", expira]]) {
    if (data !== null && !/^\d{4}-\d{2}-\d{2}$/.test(String(data))) {
      throw new Error(`A data de ${rotulo} precisa ser AAAA-MM-DD. Veio "${data}".`);
    }
  }

  const dados = {
    maquina: codigo,
    comprador: nome,
    emitida: emitida || new Date().toISOString().slice(0, 10),
    expira: expira,
  };

  // O corpo assinado é ESTE texto, byte a byte. O Otimiza confere a assinatura
  // sobre os bytes que chegam e só depois lê os campos — então a ordem das
  // chaves aqui não precisa bater com a do Rust, mas o texto não pode ser
  // remontado no caminho.
  const corpo = Buffer.from(JSON.stringify(dados), "utf8");

  const chavePrivada = crypto.createPrivateKey({
    key: Buffer.concat([DER_PRIVADA, bruta]),
    format: "der",
    type: "pkcs8",
  });

  const assinatura = crypto.sign(null, corpo, chavePrivada);
  const chave = `${base64url(corpo)}.${base64url(assinatura)}`;

  // O PAR TROCADO
  //
  // A privada e a pública saem juntas do gerador, mas são coladas em lugares
  // diferentes por mãos humanas: a pública no código do Otimiza, a privada no
  // .env do bot. Rodar o gerador duas vezes e colar metades de pares
  // diferentes é um erro fácil, silencioso, e caríssimo — tudo parece
  // funcionar, o cliente paga, e a chave não abre. Você só descobre pela
  // reclamação, com o dinheiro já recebido.
  //
  // Quando a pública é informada, a chave recém-assinada é conferida contra
  // ela antes de sair daqui. Custa microssegundos e transforma um defeito que
  // aparece no cliente num erro que aparece em você.
  if (publica) {
    const prova = conferir(chave, publica);

    if (!prova.valida) {
      throw new Error(
        "A chave privada e a chave pública não são do mesmo par. Tudo " +
          "funcionaria até o cliente tentar ativar, e aí não abriria. " +
          "Rode o gerador de novo e cole as DUAS metades da MESMA saída: " +
          "a pública em licenca.rs, a privada no .env."
      );
    }
  }

  return chave;
}

/**
 * Confere uma chave. O bot não precisa disto para vender — serve para você
 * conferir uma chave que um cliente diz que não funciona, sem pedir o PC dele.
 */
function conferir(chave, publica) {
  const limpa = String(chave).replace(/\s+/g, "");
  const partes = limpa.split(".");

  if (partes.length !== 2) {
    return { valida: false, motivo: "A chave não tem as duas partes separadas por ponto." };
  }

  const corpo = Buffer.from(partes[0], "base64url");
  const assinatura = Buffer.from(partes[1], "base64url");
  const bruta = Buffer.from(String(publica).trim(), "base64");

  if (bruta.length !== 32) {
    return { valida: false, motivo: "A chave pública não tem 32 bytes." };
  }

  const chavePublica = crypto.createPublicKey({
    key: Buffer.concat([DER_PUBLICA, bruta]),
    format: "der",
    type: "spki",
  });

  if (!crypto.verify(null, corpo, chavePublica, assinatura)) {
    return { valida: false, motivo: "A assinatura não confere." };
  }

  return { valida: true, dados: JSON.parse(corpo.toString("utf8")) };
}

function base64url(buffer) {
  return buffer.toString("base64").replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/**
 * Gera um par e ESCREVE a privada direto no .env do bot.
 *
 * POR QUE ISTO EXISTE
 *
 * A versao anterior imprimia as duas metades na tela e mandava o dono copiar
 * cada uma para o seu lugar. Ele colou a privada em `.env.example` — o arquivo
 * VERSIONADO, nao o `.env` — duas vezes em quinze minutos, e nas duas ela
 * apareceu numa captura de tela.
 *
 * Duas vezes seguidas nao e desatencao: e um projeto que pede a coisa errada.
 * Um segredo que precisa passar pela tela e pela area de transferencia ate um
 * arquivo de nome quase identico ao errado vai parar no arquivo errado.
 *
 * Aqui a privada nunca e impressa. Ela vai do gerador direto para o arquivo, e
 * so a publica aparece — que e a unica que precisa ser vista, copiada e
 * enviada.
 *
 * @param {string} caminhoDoEnv  O `.env` do bot.
 * @returns {{publica: string, criou: boolean}}
 */
function instalar(caminhoDoEnv) {
  const fs = require("fs");
  const path = require("path");

  const alvo = path.resolve(String(caminhoDoEnv || "").trim());
  const nome = path.basename(alvo);

  // A CONFERENCIA QUE MOTIVOU ESTA FUNCAO.
  //
  // `.env.example` e o molde que vai para o repositorio. Escrever segredo nele
  // e o erro exato que aconteceu, entao ele e recusado pelo nome — nao por
  // aviso em documentacao que ninguem rele.
  if (nome !== ".env") {
    throw new Error(
      `"${nome}" nao e o arquivo certo. A chave privada vai no ".env", que o ` +
        "git ignora. Qualquer outro — em especial \".env.example\" — vai para o " +
        "repositorio e levaria o segredo junto."
    );
  }

  if (!fs.existsSync(path.dirname(alvo))) {
    throw new Error(`A pasta "${path.dirname(alvo)}" nao existe.`);
  }

  const par = novoPar();
  const existia = fs.existsSync(alvo);
  const atual = existia ? fs.readFileSync(alvo, "utf8") : "";

  const escrever = (texto, chave, valor) => {
    const linha = `${chave}=${valor}`;
    const achou = new RegExp(`^\s*${chave}\s*=.*$`, "m");
    return achou.test(texto)
      ? texto.replace(achou, linha)
      : `${texto.replace(/\s*$/, "")}
${linha}
`;
  };

  let novo = escrever(atual, "OTIMIZA_CHAVE_PRIVADA", par.privada);
  novo = escrever(novo, "OTIMIZA_CHAVE_PUBLICA", par.publica);

  fs.writeFileSync(alvo, novo, "utf8");

  return { publica: par.publica, criou: !existia };
}

module.exports = { novoPar, emitir, conferir, instalar };

// ----------------------------------------------------------- linha de comando

if (require.main === module) {
  const [comando, ...resto] = process.argv.slice(2);

  try {
    if (comando === "novo-par") {
      const par = novoPar();
      console.log("PUBLICA  (vai para CHAVE_PUBLICA em licenca.rs):");
      console.log(par.publica);
      console.log();
      console.log("PRIVADA  (guarde; nunca versione, nunca cole em conversa):");
      console.log(par.privada);
      console.log();
      console.log("Perder a privada obriga a reemitir a licença de todos os clientes.");
    } else if (comando === "instalar") {
      const { publica } = instalar(resto[0]);
      console.log("Par gerado.");
      console.log();
      console.log("A chave PRIVADA foi escrita direto no .env do bot.");
      console.log("Ela nao aparece aqui de proposito: o que passa pela tela acaba");
      console.log("no arquivo errado ou numa captura de tela.");
      console.log();
      console.log("A PUBLICA, que pode ser vista por qualquer um, e esta:");
      console.log();
      console.log("    " + publica);
      console.log();
      console.log("Mande ela para colar em CHAVE_PUBLICA, em licenca.rs.");
    } else if (comando === "emitir") {
      const [privada, maquina, comprador, expira] = resto;
      console.log(emitir({ privada, maquina, comprador, expira: expira || null }));
    } else {
      console.error(
        [
          "Emissor de licença do Otimiza",
          "",
          "  node otimiza-licenca.cjs instalar <caminho do .env do bot>",
          "  node otimiza-licenca.cjs novo-par",
          "  node otimiza-licenca.cjs emitir <PRIVADA> OTZ-XXXX-XXXX-XXXX \"Nome\" [AAAA-MM-DD]",
        ].join("\n")
      );
      process.exit(2);
    }
  } catch (erro) {
    console.error(erro.message);
    process.exit(1);
  }
}
