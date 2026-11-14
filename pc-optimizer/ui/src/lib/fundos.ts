// Os fundos atrás do vidro: os de sempre (limpo, cômodo, área de trabalho), as cenas feitas em CSS, as fotos do
// FiveM e a imagem que a pessoa escolhe. A escolha e a imagem moram no localStorage; o que dá para testar é puro.

export type Fundo =
  | "limpo"
  | "comodo"
  | "area"
  | "aurora"
  | "brasa"
  | "oceano"
  | "neon"
  | "cidade"
  | "morro"
  | "meu";

/** Como o fundo é desenhado: grafite firme, o cômodo, a janela transparente, uma cena animada ou uma foto. */
export type TipoDoFundo = "limpo" | "comodo" | "area" | "cena" | "foto";

export interface OpcaoDeFundo {
  id: Fundo;
  nome: string;
  tipo: TipoDoFundo;
  /** A foto do fundo (as do FiveM); o "meu" usa a imagem guardada. */
  foto?: string;
}

export const FUNDOS: OpcaoDeFundo[] = [
  { id: "limpo", nome: "Limpo", tipo: "limpo" },
  { id: "aurora", nome: "Aurora", tipo: "cena" },
  { id: "brasa", nome: "Brasa", tipo: "cena" },
  { id: "oceano", nome: "Oceano", tipo: "cena" },
  { id: "neon", nome: "Neon", tipo: "cena" },
  { id: "cidade", nome: "Cidade", tipo: "foto", foto: "/previas/fivem-rua.jpg" },
  { id: "morro", nome: "Morro", tipo: "foto", foto: "/previas/fivem-morro.jpg" },
  { id: "comodo", nome: "Cômodo", tipo: "comodo" },
  { id: "area", nome: "Área de trabalho", tipo: "area" },
  { id: "meu", nome: "Sua imagem", tipo: "foto" },
];

export const CHAVE_DO_FUNDO = "otimiza.fundo";
export const CHAVE_DA_IMAGEM = "otimiza.fundo.imagem";
export const CHAVE_DO_AJUSTE = "otimiza.fundo.ajuste";

/** Desfoque (px) e escurecer (0 a 1) das fotos: o bastante para os cards lerem bem por cima. */
export interface AjusteDoFundo {
  desfoque: number;
  escurecer: number;
}
export const AJUSTE_PADRAO: AjusteDoFundo = { desfoque: 6, escurecer: 0.45 };

export function opcaoDo(id: Fundo): OpcaoDeFundo {
  return FUNDOS.find((f) => f.id === id) ?? FUNDOS[0];
}

/**
 * O valor guardado vira um fundo que existe. "sem" e "com" são de versões antigas (o botão "Sem fundo"); "meu" sem
 * imagem guardada (apagada, ou o armazenamento limpo) volta para o limpo, em vez de abrir num fundo vazio.
 */
export function lerFundoGuardado(v: string | null, temImagem: boolean): Fundo {
  if (v === "sem") return "area";
  if (v === "com") return "comodo";
  if (v === "meu") return temImagem ? "meu" : "limpo";
  return FUNDOS.some((f) => f.id === v) ? (v as Fundo) : "limpo";
}

/** O ajuste guardado, com cada número de volta para a faixa do controle. Lixo vira o padrão. */
export function lerAjusteGuardado(v: string | null): AjusteDoFundo {
  try {
    const bruto = v ? (JSON.parse(v) as Partial<AjusteDoFundo>) : {};
    const num = (x: unknown, padrao: number, min: number, max: number) =>
      typeof x === "number" && Number.isFinite(x) ? Math.min(max, Math.max(min, x)) : padrao;
    return {
      desfoque: num(bruto.desfoque, AJUSTE_PADRAO.desfoque, 0, 30),
      escurecer: num(bruto.escurecer, AJUSTE_PADRAO.escurecer, 0, 0.85),
    };
  } catch {
    return { ...AJUSTE_PADRAO };
  }
}

/** O tamanho da imagem depois de reduzida: o lado maior cabe em `maximo`, sem nunca aumentar. */
export function tamanhoReduzido(largura: number, altura: number, maximo: number): { largura: number; altura: number } {
  const maior = Math.max(largura, altura);
  if (maior <= maximo || maior <= 0) return { largura, altura };
  const k = maximo / maior;
  return { largura: Math.round(largura * k), altura: Math.round(altura * k) };
}

/** Acima disto o arquivo nem é aberto: foto de celular de 12 MP passa; um RAW de 80 MB, não. */
export const LIMITE_DO_ARQUIVO = 30 * 1024 * 1024;

/**
 * Lê a imagem escolhida, reduz para no máximo 2560 px no lado maior e devolve um JPEG em data URL (cabe folgado no
 * localStorage). GIF e WebP animados viram o primeiro quadro.
 */
export async function prepararImagem(arquivo: File): Promise<string> {
  if (!arquivo.type.startsWith("image/")) throw new Error("Esse arquivo não é uma imagem.");
  if (arquivo.size > LIMITE_DO_ARQUIVO) throw new Error("Imagem grande demais (máximo de 30 MB).");
  const bitmap = await createImageBitmap(arquivo).catch(() => {
    throw new Error("Não deu para abrir essa imagem.");
  });
  const { largura, altura } = tamanhoReduzido(bitmap.width, bitmap.height, 2560);
  const tela = document.createElement("canvas");
  tela.width = largura;
  tela.height = altura;
  const ctx = tela.getContext("2d");
  if (!ctx) throw new Error("Não deu para preparar a imagem.");
  ctx.drawImage(bitmap, 0, 0, largura, altura);
  bitmap.close();
  return tela.toDataURL("image/jpeg", 0.86);
}
