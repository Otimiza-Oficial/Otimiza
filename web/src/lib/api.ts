/**
 * O site é estático e não guarda segredo: quem cobra e emite a chave é a API do
 * bot. Os dois valores abaixo entram na compilação e são públicos por natureza.
 */
export const API = (process.env.NEXT_PUBLIC_OTIMIZA_API ?? "").replace(/\/+$/, "");
export const MP_PUBLIC_KEY = (process.env.NEXT_PUBLIC_MP_PUBLIC_KEY ?? "").trim();

export const TEM_CHECKOUT = API.length > 0;
export const TEM_CARTAO = TEM_CHECKOUT && MP_PUBLIC_KEY.length > 0;

export const INTERVALO_CONSULTA_MS = 3000;

/** Depois do vencimento, ainda se pergunta por este tempo: o banco pode confirmar atrasado. */
export const FOLGA_DEPOIS_DO_PRAZO_MS = 90 * 1000;

export type Metodo = "pix" | "cartao";

export type Compra = {
  token: string;
  expiraEm: number;
  pix?: { qrBase64?: string | null; copiaECola: string };
};

export type EstadoDaCompra =
  | "pendente"
  | "em_analise"
  | "pago"
  | "entregue"
  | "recusada"
  | "vencida"
  | "cancelada"
  | "estornada";

export type Consulta = {
  estado: EstadoDaCompra;
  chave?: string;
  codigo?: string;
  motivo?: string;
  expiraEm?: number;
};

/** Página https não pode chamar API http: o navegador bloqueia sem explicar. */
export function misturaInsegura(): boolean {
  if (!TEM_CHECKOUT || typeof window === "undefined") return false;
  return window.location.protocol === "https:" && API.startsWith("http://");
}

export async function consultar(token: string): Promise<Consulta | null> {
  const r = await fetch(`${API}/v1/compras/${encodeURIComponent(token)}`, { headers: { accept: "application/json" } });
  if (!r.ok) return null;
  return (await r.json()) as Consulta;
}
