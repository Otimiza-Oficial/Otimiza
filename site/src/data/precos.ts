/**
 * O preco, como numero. O TEXTO em volta dele mora em src/data/i18n/.
 *
 * Mesma regra do resto: nada aqui foi inventado para soar melhor.
 */
import type { Idioma } from "./i18n/tipos";
import { traducoes } from "./i18n";

/* ==========================================================================
   O PRECO — R$ 20, o mesmo que o bot do Discord ja cobra.
   ==========================================================================
   Vender no site por valor diferente do canal que ja existe cria problema:
   um cliente descobre a diferenca e a conversa vira sobre isso. Todo numero
   da tela sai daqui: trocar este numero troca o site inteiro, nas tres linguas.
   ========================================================================== */
export const PRECO_BRL = 20;

/* ==========================================================================
   MOEDA — CONFIRMAR COM O DONO
   ==========================================================================
   O valor fica em REAL (BRL) nas tres linguas, e as versoes em ingles e em
   espanhol dizem isso com todas as letras ("R$ 20.00 BRL" + a nota de moeda
   em i18n/en.ts e i18n/es.ts).

   O motivo e simples: o checkout e do Mercado Pago e cobra em real. Mostrar
   um valor em dolar ou em peso na tela seria mentir sobre o que vai ser
   cobrado — exatamente o tipo de coisa que este produto acusa o mercado de
   fazer.

   O QUE AINDA NAO FOI DECIDIDO, e precisa da palavra do dono:
     1. O produto vai mesmo ser vendido fora do Brasil?
     2. A conta do Mercado Pago aceita cartao internacional? Pix e Boleto nao
        servem para comprador de fora, e os tres estao listados nas tres
        linguas hoje.
     3. Se um dia houver preco em outra moeda, ele precisa ser um preco de
        verdade no checkout — nunca uma conversao decorativa na tela.
   ========================================================================== */

/* CONFIRMAR COM O DONO — a segunda licenca tem desconto? O dono ainda nao
   decidiu. Enquanto este valor for igual a PRECO_BRL, o cartao secundario
   nao promete desconto nenhum: so informa o preco. */
export const PRECO_LICENCA_ADICIONAL_BRL = PRECO_BRL;

/* CONFIRMAR COM O DONO — parcelamento e condicao do Mercado Pago, e depende
   da conta dele (quantas parcelas sem juros, se e que ha). Enquanto nao
   estiver confirmado, deixe MOSTRAR_PARCELAMENTO em false: e melhor nao
   dizer nada do que anunciar uma condicao que o checkout nao vai cumprir. */
export const MOSTRAR_PARCELAMENTO: boolean = false;
export const PARCELAS_SEM_JUROS = 3;

/* --------------------------------------------------------------------------
   Derivacoes. Nada abaixo desta linha precisa ser editado para trocar o preco.
   -------------------------------------------------------------------------- */

export type PartesPreco = {
  /** "R$" — o simbolo nao muda de lingua: a cobranca e em real. */
  readonly moeda: string;
  readonly inteiro: string;
  readonly centavos: string;
  /** "," em pt e es, "." em en. So a tipografia do numero muda. */
  readonly separador: string;
  /** "R$ 20,00" / "R$ 20.00" — para atributos e para o cartao secundario. */
  readonly completo: string;
};

/**
 * Quebra o valor em partes para o componente compor a tipografia do preco.
 *
 * O separador decimal segue a convencao de quem le: virgula em portugues e em
 * espanhol, ponto em ingles. O VALOR e o mesmo nos tres — 20 reais.
 */
function partes(valor: number, localeNumero: string): PartesPreco {
  const formato = new Intl.NumberFormat(localeNumero, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  const texto = formato.format(valor);
  /* O separador decimal e o ultimo caractere nao-digito da string formatada.
     Perguntar ao Intl em vez de assumir evita errar em locale futuro. */
  const separador = texto.replace(/\d/g, "").slice(-1) || ",";
  const corte = texto.lastIndexOf(separador);
  return {
    moeda: "R$",
    inteiro: texto.slice(0, corte),
    centavos: texto.slice(corte + 1),
    separador,
    completo: `R$ ${texto}`,
  };
}

/** Tudo que a secao de preco precisa, ja no idioma da pagina. */
export function precosDe(idioma: Idioma) {
  const t = traducoes[idioma];
  const locale = t.meta.localeNumero;

  const aVista = partes(PRECO_BRL, locale);
  const adicional = partes(PRECO_LICENCA_ADICIONAL_BRL, locale);

  return {
    aVista,
    adicional,
    /** Ex.: "ou 3x de R$ 6,67 sem juros no cartao". Null quando desligado. */
    parcelamento: MOSTRAR_PARCELAMENTO
      ? t.precos.parcelamento(
          PARCELAS_SEM_JUROS,
          partes(Math.ceil((PRECO_BRL / PARCELAS_SEM_JUROS) * 100) / 100, locale).completo,
        )
      : null,
  };
}
