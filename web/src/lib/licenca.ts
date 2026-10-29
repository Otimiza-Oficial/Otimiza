/**
 * A conferência da licença no navegador, com a mesma regra de licenca.rs: <dados>.<assinatura> em base64url, Ed25519.
 * A chave pública confere e não cria. A assinatura é conferida antes de qualquer campo ser lido. Mudou no app, muda aqui.
 */
export const CHAVE_PUBLICA = "Fz2AWNqDmj2iLeaEqkfMWgsfVdXuXJn2+eAn006pKSM=";
/** A pública anterior (a privada se perdeu com a hospedagem do bot em 04/10/2026): só para licença já emitida. */
export const CHAVE_PUBLICA_ANTERIOR = "sR3nmVzmAtjoDmAWr8McycSq+vhDUCy2YnLDhJfy5LU=";
/** Quem ficou com a privada perdida assina qualquer data: a anterior só vale para licença emitida até a troca. */
export const FIM_DA_CHAVE_ANTERIOR = "2026-10-05"; // o bot carimba em UTC: venda da noite de 04/10 sai com 05

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
 * OTZ- e três blocos de quatro caracteres, sem I, O, S e Z (maquina.rs).
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

export function desmontar(chave: string): { dados: Uint8Array; assinatura: Uint8Array } | null {
  const limpa = chave.replace(/\s+/g, "");
  const ponto = limpa.indexOf(".");
  if (ponto <= 0 || ponto === limpa.length - 1) return null;
  const dados = base64urlParaBytes(limpa.slice(0, ponto));
  const assinatura = base64urlParaBytes(limpa.slice(ponto + 1));
  if (!dados || !assinatura) return null;
  return { dados, assinatura };
}

/** Separada, como conferir_com no Rust: o teste gera o próprio par. maquina é opcional no navegador. */
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
  if (dados.expira && hoje > dados.expira) {
    return { ok: false, recusa: { tipo: "expirada", em: dados.expira } };
  }
  return { ok: true, dados };
}

const bytesDaPublica = (texto: string) => Uint8Array.from(atob(texto), (c) => c.charCodeAt(0));

export const hojeISO = () => {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
};

/**
 * Como `licenca::conferir`: a atual decide; só a recusa de assinatura passa para a anterior, que vale até a troca.
 * (A lista de máquinas da anterior mora só no app: o painel não libera nada, só mostra.)
 */
export async function conferir(chave: string, maquina: string | null): Promise<Resultado> {
  const hoje = hojeISO();
  const atual = await conferirCom(bytesDaPublica(CHAVE_PUBLICA), chave, maquina, hoje);
  if (atual.ok || atual.recusa.tipo !== "assinatura-invalida") return atual;

  const anterior = await conferirCom(bytesDaPublica(CHAVE_PUBLICA_ANTERIOR), chave, maquina, hoje);
  if (anterior.ok && anterior.dados.emitida > FIM_DA_CHAVE_ANTERIOR) {
    return { ok: false, recusa: { tipo: "assinatura-invalida" } };
  }
  return anterior;
}

export function formatarData(iso: string): string {
  const [a, m, d] = iso.split("-");
  return a && m && d ? `${d}/${m}/${a}` : iso;
}
