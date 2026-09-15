/**
 * A conferência da licença, feita no navegador.
 *
 * É a MESMA regra de `pc-optimizer/src-tauri/src/modules/licenca.rs`:
 *
 *     <dados em base64url>.<assinatura em base64url>
 *
 * A assinatura Ed25519 é conferida com a chave PÚBLICA do Otimiza — a mesma que
 * viaja no instalador. Ela confere e não cria: estar neste arquivo não permite
 * gerar chave nenhuma. E a ordem também é a mesma do app: a assinatura é
 * conferida ANTES de qualquer campo ser lido como verdade.
 *
 * Se a chave pública do app mudar, esta aqui precisa mudar junto.
 */
export const CHAVE_PUBLICA = "sR3nmVzmAtjoDmAWr8McycSq+vhDUCy2YnLDhJfy5LU=";

/** O que a licença afirma (espelho de `Dados` no Rust). */
export type Dados = {
  maquina: string;
  comprador: string;
  emitida: string;
  expira: string | null;
};

export type Recusa =
  | { tipo: "malformada" }
  | { tipo: "assinatura-invalida" }
  | { tipo: "outra-maquina"; emitidaPara: string }
  | { tipo: "expirada"; em: string }
  | { tipo: "sem-suporte" };

export type Resultado = { ok: true; dados: Dados } | { ok: false; recusa: Recusa };

/** Os textos são os do app, para o cliente ler a mesma coisa nos dois lugares. */
export function explicar(recusa: Recusa): string {
  switch (recusa.tipo) {
    case "malformada":
      return "Esta chave não está completa. Confira se copiou tudo, do começo ao fim, sem espaço sobrando.";
    case "assinatura-invalida":
      return "Esta chave não foi reconhecida. Ou faltou um pedaço na cópia, ou ela não foi emitida pelo Otimiza.";
    case "outra-maquina":
      return "Esta chave foi emitida para outro computador. Cada chave vale em uma máquina só — é o que impede que ela seja repassada. Se você trocou de PC ou de placa-mãe, fale no Discord com o código desta máquina que a gente emite outra.";
    case "expirada":
      return `Esta chave venceu em ${formatarData(recusa.em)}.`;
    case "sem-suporte":
      return "Este navegador não sabe conferir a assinatura da chave (Ed25519). Atualize o navegador ou use o Chrome, o Edge, o Firefox ou o Safari mais recentes.";
  }
}

/*
 * O código da máquina: OTZ- e três blocos de quatro caracteres, num alfabeto
 * sem I, O, S e Z — as letras que se confundem com número (`maquina.rs`).
 */
const CODIGO = /^OTZ-[A-HJ-NP-RT-Y0-9]{4}-[A-HJ-NP-RT-Y0-9]{4}-[A-HJ-NP-RT-Y0-9]{4}$/;

export const normalizarCodigo = (codigo: string) => codigo.trim().toUpperCase();
export const codigoValido = (codigo: string) => CODIGO.test(normalizarCodigo(codigo));

function base64urlParaBytes(texto: string): Uint8Array | null {
  if (!/^[A-Za-z0-9_-]+$/.test(texto)) return null;
  const padrao = texto.replace(/-/g, "+").replace(/_/g, "/");
  const completo = padrao + "=".repeat((4 - (padrao.length % 4)) % 4);
  try {
    const binario = atob(completo);
    return Uint8Array.from(binario, (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
}

/** Separa a chave nas duas partes. Espaço e quebra de linha são perdoados. */
export function desmontar(chave: string): { dados: Uint8Array; assinatura: Uint8Array } | null {
  const limpa = chave.replace(/\s+/g, "");
  const ponto = limpa.indexOf(".");
  if (ponto <= 0 || ponto === limpa.length - 1) return null;
  const dados = base64urlParaBytes(limpa.slice(0, ponto));
  const assinatura = base64urlParaBytes(limpa.slice(ponto + 1));
  if (!dados || !assinatura) return null;
  return { dados, assinatura };
}

/**
 * Confere a chave com uma chave pública dada. Existe separada pelo mesmo motivo
 * do `conferir_com` do Rust: o teste gera o próprio par e assina.
 *
 * `maquina` é opcional aqui, ao contrário do app: no navegador a pessoa pode
 * querer só ver para quem a chave foi emitida.
 */
export async function conferirCom(
  publica: Uint8Array,
  chave: string,
  maquina: string | null,
  hoje: string,
): Promise<Resultado> {
  const partes = desmontar(chave);
  if (!partes || partes.assinatura.length !== 64) return { ok: false, recusa: { tipo: "malformada" } };

  let valida: boolean;
  try {
    const cripto = globalThis.crypto.subtle;
    const chaveCripto = await cripto.importKey("raw", publica as BufferSource, { name: "Ed25519" }, false, ["verify"]);
    valida = await cripto.verify(
      { name: "Ed25519" },
      chaveCripto,
      partes.assinatura as BufferSource,
      partes.dados as BufferSource,
    );
  } catch {
    return { ok: false, recusa: { tipo: "sem-suporte" } };
  }
  if (!valida) return { ok: false, recusa: { tipo: "assinatura-invalida" } };

  // Só agora, com a assinatura conferida, os dados são lidos.
  let dados: Dados;
  try {
    const bruto = JSON.parse(new TextDecoder().decode(partes.dados));
    if (typeof bruto?.maquina !== "string" || typeof bruto?.emitida !== "string") throw new Error();
    dados = {
      maquina: bruto.maquina,
      comprador: typeof bruto.comprador === "string" ? bruto.comprador : "",
      emitida: bruto.emitida,
      expira: typeof bruto.expira === "string" ? bruto.expira : null,
    };
  } catch {
    return { ok: false, recusa: { tipo: "assinatura-invalida" } };
  }

  if (maquina && dados.maquina !== normalizarCodigo(maquina)) {
    return { ok: false, recusa: { tipo: "outra-maquina", emitidaPara: dados.maquina } };
  }
  // "AAAA-MM-DD" compara certo como texto, como no app.
  if (dados.expira && hoje > dados.expira) {
    return { ok: false, recusa: { tipo: "expirada", em: dados.expira } };
  }
  return { ok: true, dados };
}

const publicaDoOtimiza = () => Uint8Array.from(atob(CHAVE_PUBLICA), (c) => c.charCodeAt(0));

export const hojeISO = () => {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
};

export function conferir(chave: string, maquina: string | null): Promise<Resultado> {
  return conferirCom(publicaDoOtimiza(), chave, maquina, hojeISO());
}

export function formatarData(iso: string): string {
  const [a, m, d] = iso.split("-");
  return a && m && d ? `${d}/${m}/${a}` : iso;
}
