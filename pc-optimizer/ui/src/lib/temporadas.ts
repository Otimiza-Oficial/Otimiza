// As épocas do ano no Otimiza: o painel muda sozinho no Natal, Ano Novo, Carnaval, Páscoa, Festa Junina, Dia das
// Crianças e Halloween. Puro: recebe a data e devolve a época (ou nenhuma). Páscoa e Carnaval mudam de data todo ano e
// são calculados (algoritmo de Gauss/Meeus para a Páscoa; Carnaval é 47 dias antes).
//
// Datas no fuso de quem usa (a data local do PC). A ordem da lista decide quando duas se sobrepõem: a mais específica
// vem primeiro (Ano Novo dentro do Natal).

export type IdDaTemporada =
  | "ano-novo"
  | "natal"
  | "carnaval"
  | "pascoa"
  | "festa-junina"
  | "dia-das-criancas"
  | "halloween";

/** O que o Oti usa na cabeça. */
export type Acessorio = "gorro" | "cartola" | "mascara" | "orelhas" | "chapeu-de-palha" | "bone" | "chapeu-de-bruxa";

/** O que cai no fundo do painel. */
export type Efeito = "neve" | "fogos" | "confete" | "ovinhos" | "bandeirinhas" | "baloes" | "morcegos";

export interface Temporada {
  id: IdDaTemporada;
  nome: string;
  saudacao: string;
  acessorio: Acessorio;
  efeito: Efeito;
}

const TEMPORADAS: Record<IdDaTemporada, Temporada> = {
  "ano-novo": { id: "ano-novo", nome: "Ano Novo", saudacao: "Feliz Ano Novo!", acessorio: "cartola", efeito: "fogos" },
  natal: { id: "natal", nome: "Natal", saudacao: "Boas festas!", acessorio: "gorro", efeito: "neve" },
  carnaval: { id: "carnaval", nome: "Carnaval", saudacao: "Bom Carnaval!", acessorio: "mascara", efeito: "confete" },
  pascoa: { id: "pascoa", nome: "Páscoa", saudacao: "Feliz Páscoa!", acessorio: "orelhas", efeito: "ovinhos" },
  "festa-junina": { id: "festa-junina", nome: "Festa Junina", saudacao: "Arraiá do Otimiza!", acessorio: "chapeu-de-palha", efeito: "bandeirinhas" },
  "dia-das-criancas": { id: "dia-das-criancas", nome: "Dia das Crianças", saudacao: "Feliz Dia das Crianças!", acessorio: "bone", efeito: "baloes" },
  halloween: { id: "halloween", nome: "Halloween", saudacao: "Feliz Halloween!", acessorio: "chapeu-de-bruxa", efeito: "morcegos" },
};

export const TODAS: Temporada[] = Object.values(TEMPORADAS);

/** Domingo de Páscoa (calendário gregoriano), à meia-noite local. */
export function pascoa(ano: number): Date {
  const a = ano % 19;
  const b = Math.floor(ano / 100);
  const c = ano % 100;
  const d = Math.floor(b / 4);
  const e = b % 4;
  const f = Math.floor((b + 8) / 25);
  const g = Math.floor((b - f + 1) / 3);
  const h = (19 * a + b - d - g + 15) % 30;
  const i = Math.floor(c / 4);
  const k = c % 4;
  const l = (32 + 2 * e + 2 * i - h - k) % 7;
  const m = Math.floor((a + 11 * h + 22 * l) / 451);
  const mes = Math.floor((h + l - 7 * m + 114) / 31);
  const dia = ((h + l - 7 * m + 114) % 31) + 1;
  return new Date(ano, mes - 1, dia);
}

const DIA = 86_400_000;

function semHora(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** Entre (mês/dia) e (mês/dia), inclusive, aceitando a virada do ano (ex.: 1/11 a 6/1). Mês de 1 a 12. */
function entre(d: Date, de: [number, number], ate: [number, number]): boolean {
  const md = (d.getMonth() + 1) * 100 + d.getDate();
  const a = de[0] * 100 + de[1];
  const b = ate[0] * 100 + ate[1];
  return a <= b ? md >= a && md <= b : md >= a || md <= b;
}

/** Entre `antes` dias antes e `depois` dias depois de uma data, inclusive. */
function perto(d: Date, centro: Date, antes: number, depois: number): boolean {
  const x = semHora(d);
  const c = semHora(centro);
  return x >= c - antes * DIA && x <= c + depois * DIA;
}

/** A época desta data, ou null. */
export function temporadaDe(data: Date): Temporada | null {
  const p = pascoa(data.getFullYear());
  const carnaval = new Date(p.getTime() - 47 * DIA);

  if (entre(data, [12, 27], [1, 2])) return TEMPORADAS["ano-novo"];
  // No Brasil o clima de Natal começa em novembro, com as lojas e as luzes.
  if (entre(data, [11, 1], [1, 6])) return TEMPORADAS.natal;
  // Do sábado antes até a Quarta-feira de Cinzas.
  if (perto(data, carnaval, 3, 1)) return TEMPORADAS.carnaval;
  // A Semana Santa e a segunda depois.
  if (perto(data, p, 7, 1)) return TEMPORADAS.pascoa;
  // Junina e julina: o arraiá vai até meados de julho.
  if (entre(data, [6, 1], [7, 15])) return TEMPORADAS["festa-junina"];
  if (entre(data, [10, 5], [10, 12])) return TEMPORADAS["dia-das-criancas"];
  if (entre(data, [10, 20], [10, 31])) return TEMPORADAS.halloween;
  return null;
}

export function porId(id: string | null): Temporada | null {
  return id && id in TEMPORADAS ? TEMPORADAS[id as IdDaTemporada] : null;
}
