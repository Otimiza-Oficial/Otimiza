/**
 * Onde fica a API que vende.
 *
 * O site e estatico: ele nao cobra e nao emite licenca — nao pode. A chave
 * privada Ed25519 que assina a licenca so existe no servidor do bot, e e por
 * isso que ela nao pode existir aqui: tudo que este projeto compila vira
 * arquivo publico no GitHub Pages.
 *
 * COMO CONFIGURAR
 *
 * A URL vem de `PUBLIC_OTIMIZA_API`, lida na hora de compilar. O prefixo
 * `PUBLIC_` e do Astro e significa "este valor vai para o navegador" — o que
 * esta certo aqui, porque um endereco de API nao e segredo. Segredo nenhum
 * pode entrar neste arquivo.
 *
 * Em desenvolvimento:  PUBLIC_OTIMIZA_API=http://localhost:8787
 * Em producao:         PUBLIC_OTIMIZA_API=https://api.seu-dominio.com.br
 *
 * VAZIO E UM ESTADO VALIDO, e o padrao. Sem a variavel, a pagina de compra
 * mostra o caminho pelo Discord e nao promete checkout nenhum. Isso e melhor
 * que apontar para um endereco que nao responde: o cliente clicaria, esperaria,
 * e receberia um erro que nao explica nada.
 */
export const API = (import.meta.env.PUBLIC_OTIMIZA_API ?? '').replace(/\/+$/, '');

/** O checkout do site esta ligado nesta compilacao? */
export const TEM_CHECKOUT = API.length > 0;

/**
 * De quanto em quanto tempo perguntar se o Pix foi pago.
 *
 * Tres segundos e o meio termo entre parecer instantaneo e nao martelar a API.
 * O bot consulta o Mercado Pago no ritmo dele; perguntar mais rapido que isso
 * so gastaria requisicao para receber a mesma resposta.
 */
export const INTERVALO_CONSULTA_MS = 3000;

/**
 * Quando desistir de perguntar.
 *
 * O Pix do bot vale 10 minutos. Doze minutos de consulta cobrem a validade
 * inteira com folga para o relogio do cliente estar adiantado, e param sozinhos
 * depois — uma aba esquecida aberta a noite toda nao pode ficar batendo na API
 * ate o navegador fechar.
 */
export const LIMITE_CONSULTA_MS = 12 * 60 * 1000;
