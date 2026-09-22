/**
 * ONDE FICA A API QUE VENDE — E POR QUE ELA NÃO MORA AQUI.
 *
 * Este site é estático: tudo o que ele compila vira arquivo público no GitHub
 * Pages. A chave privada que assina uma licença não pode existir num arquivo
 * público, então quem cobra e quem emite é o serviço do bot, fora daqui. O
 * site só conversa com ele.
 *
 * COMO LIGAR
 *
 * `NEXT_PUBLIC_OTIMIZA_API` na compilação. O prefixo `NEXT_PUBLIC_` é do Next
 * e quer dizer "este valor vai para o navegador" — o que está certo: endereço
 * de API não é segredo. Segredo nenhum entra neste arquivo.
 *
 *   No computador:  NEXT_PUBLIC_OTIMIZA_API=http://localhost:8787
 *   No ar:          NEXT_PUBLIC_OTIMIZA_API=https://api.seu-dominio.com.br
 *
 * VAZIO É O PADRÃO, e é um estado válido: a página de compra mostra o caminho
 * do Discord e não promete checkout nenhum. Melhor que um botão que chama um
 * endereço que não responde.
 *
 * O QUE AINDA FALTA PARA LIGAR DE VERDADE (não é código):
 *
 * 1. o serviço do bot rodando 24 horas, fora do PC do dono;
 * 2. HTTPS nele — uma página https não pode chamar uma API http, e o
 *    navegador bloqueia antes de sair da máquina do cliente;
 * 3. o endereço na lista de origens do CORS do bot;
 * 4. termos de uso e política de privacidade publicados, antes da primeira
 *    venda pelo site.
 */
export const API = (process.env.NEXT_PUBLIC_OTIMIZA_API ?? "").replace(/\/+$/, "");

/** O checkout do site está ligado nesta compilação? */
export const TEM_CHECKOUT = API.length > 0;

/**
 * De quanto em quanto tempo perguntar se o Pix foi pago.
 *
 * Três segundos é o meio-termo entre parecer instantâneo e não martelar a API.
 * O bot consulta o provedor no ritmo dele; perguntar mais rápido só gastaria
 * requisição para receber a mesma resposta.
 */
export const INTERVALO_CONSULTA_MS = 3000;

/**
 * Quando desistir de perguntar.
 *
 * O Pix vale 10 minutos. Doze cobrem a validade inteira com folga para o
 * relógio do cliente estar adiantado — e param sozinhos depois. Uma aba
 * esquecida aberta a noite toda não pode ficar batendo na API até o navegador
 * fechar.
 */
export const LIMITE_CONSULTA_MS = 12 * 60 * 1000;

/** O que o serviço responde ao criar uma compra. */
export type Compra = {
  token: string;
  /** ISO. Quando o Pix vence. */
  expiraEm: string;
  pix?: { qrBase64?: string; copiaECola: string };
};

export type EstadoDaCompra = "pendente" | "pago" | "vencida" | "cancelada" | "estornada";

export type Consulta = {
  estado: EstadoDaCompra;
  /** Só vem quando `estado` é `pago`. É a licença. */
  chave?: string;
};

/**
 * Uma página https NÃO pode chamar uma API http: o navegador bloqueia, e o
 * erro que aparece não explica nada. Aqui isso vira uma frase antes do clique.
 */
export function misturaInsegura(): boolean {
  if (!TEM_CHECKOUT || typeof window === "undefined") return false;
  return window.location.protocol === "https:" && API.startsWith("http://");
}
