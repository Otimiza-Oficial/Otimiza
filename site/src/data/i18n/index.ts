/**
 * O registro dos idiomas e as funcoes de URL.
 *
 * ROTAS
 *   /      -> portugues (padrao, SEM prefixo)
 *   /en    -> ingles
 *   /es    -> espanhol
 *
 * Por que o portugues nao tem prefixo: e o idioma padrao e o mercado principal.
 * Um redirecionamento de /pt para / (ou o contrario) custaria uma requisicao a
 * mais e duplicaria a URL da pagina mais importante do site aos olhos do
 * buscador. As outras duas ganham prefixo porque precisam de URL propria para
 * o hreflang funcionar.
 *
 * NAO HA DETECCAO AUTOMATICA DE IDIOMA, de proposito. Redirecionar por IP ou
 * por navigator.language prende quem quer ler noutra lingua (um brasileiro no
 * exterior, um leitor que estuda a lingua, o proprio dono conferindo a
 * traducao) e atrapalha o buscador, que rastreia de varios paises. Quem escolhe
 * a lingua e o leitor, clicando.
 */
import type { Conteudo, Idioma } from "./tipos";
import { pt } from "./pt";
import { en } from "./en";
import { es } from "./es";

export type { Conteudo, Idioma, ItemFaq, ItemLista, Legenda } from "./tipos";

/** A ordem aqui e a ordem do seletor no cabecalho: PT · EN · ES. */
export const IDIOMAS = ["pt", "en", "es"] as const satisfies readonly Idioma[];

export const IDIOMA_PADRAO: Idioma = "pt";

export const traducoes: Record<Idioma, Conteudo> = { pt, en, es };

/** O prefixo de rota de cada idioma. O padrao nao tem. */
const PREFIXO: Record<Idioma, string> = { pt: "", en: "/en", es: "/es" };

/**
 * O BASE_URL do Astro ja vem com barra no fim ("/Otimiza/"). Ela e removida
 * antes de qualquer concatenacao — senao sai "/Otimiza//en".
 *
 * Sempre use estas funcoes em vez de escrever caminho na mao: o site e servido
 * em subpasta, e um "/en" cru levaria para fora do site em producao.
 */
const BASE = import.meta.env.BASE_URL.replace(/\/+$/, "");

/**
 * Caminho absoluto de uma pagina, ja com o BASE_URL e o prefixo de idioma.
 * `caminho("en")` -> "/Otimiza/en/"
 * `caminho("pt", "/comprar")` -> "/Otimiza/comprar"
 */
export function caminho(idioma: Idioma, sufixo = "/"): string {
  const raiz = `${BASE}${PREFIXO[idioma]}`;
  if (sufixo === "/") return `${raiz}/`;
  return `${raiz}${sufixo}`;
}

/**
 * URL absoluta, para canonical e hreflang. Precisa do `site` do
 * astro.config.mjs; sem ele (nao deveria acontecer) cai no caminho relativo.
 */
export function urlAbsoluta(idioma: Idioma, site: URL | undefined): string {
  const rel = caminho(idioma);
  return site ? new URL(rel, site).href : rel;
}
