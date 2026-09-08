/**
 * O contrato de conteudo do site, em tres linguas.
 *
 * POR QUE ESTE ARQUIVO EXISTE
 * Um site trilingue que perde uma frase em silencio e pior que um monolingue:
 * o visitante ve um buraco e conclui que o produto e mal-acabado — numa pagina
 * que se vende justamente por acabamento e honestidade. Entao nada aqui e
 * opcional por acidente. Cada arquivo de traducao (pt.ts, en.ts, es.ts) declara
 * `const x: Conteudo`, e uma chave faltando vira ERRO DE COMPILACAO, nao texto
 * sumido na tela.
 *
 * `npm run build` roda `astro check` antes de `astro build` exatamente por
 * isso: sem o check, o esbuild apaga os tipos e a falta passa batido.
 *
 * REGRA QUE VALE PARA AS TRES LINGUAS
 * Nenhum numero muda de lingua para lingua. 677 testes, 1,2 s, 3,7 s, 25% de
 * CPU sao medicoes, e medicao nao se traduz — so muda o separador decimal.
 * E nada de "melhorar" o texto na traducao: se o portugues diz "admite quando
 * nao mudou nada", o ingles diz isso, e nao "maximize your performance".
 * Marketing vazio na versao em ingles destruiria o unico argumento do produto.
 */

/** As tres linguas. O portugues nao tem prefixo de rota; as outras duas tem. */
export type Idioma = "pt" | "en" | "es";

export type Legenda = {
  readonly titulo: string;
  readonly descricao: string;
};

export type ItemLista = {
  readonly titulo: string;
  readonly texto: string;
};

export type ItemFaq = {
  readonly pergunta: string;
  readonly resposta: readonly string[];
  /** true = decisao comercial do dono, ainda nao confirmada. */
  readonly confirmar?: boolean;
};

export type Conteudo = {
  /** Metadados de idioma e SEO. Nenhum deles aparece como texto na pagina. */
  readonly meta: {
    /** Valor do atributo <html lang>. */
    readonly htmlLang: string;
    /** Valor de <link rel="alternate" hreflang>. */
    readonly hreflang: string;
    /** Valor de <meta property="og:locale">. */
    readonly ogLocale: string;
    /**
     * Etiqueta BCP-47 usada SO para formatar numero (separador decimal).
     * Nao e a lingua do texto: e a convencao tipografica do numero.
     */
    readonly localeNumero: string;
    /** Sigla do seletor de idioma: PT · EN · ES. */
    readonly sigla: string;
    /** Nome da lingua no proprio idioma, para o title do link. */
    readonly nome: string;
    readonly titulo: string;
    readonly descricao: string;
  };

  /** Texto que so leitor de tela ouve. Traduzir isto tambem nao e opcional. */
  readonly a11y: {
    readonly pularConteudo: string;
    readonly secoes: string;
    readonly abrirMenu: string;
    readonly trocarTema: string;
    readonly idioma: string;
    readonly linksRodape: string;
    readonly numerosMedidos: string;
    readonly capturaPendente: string;
  };

  readonly cabecalho: {
    readonly links: readonly { readonly href: string; readonly texto: string }[];
    readonly comprar: string;
    readonly menu: string;
    readonly preco: string;
  };

  readonly hero: {
    readonly promessa: string;
    readonly chamada: string;
    readonly texto: string;
    /** Acao principal: baixar. O codigo da maquina so existe apos instalar. */
    readonly ctaBaixar: string;
    readonly ctaComprar: string;
    readonly ctaTelas: string;
    /** "Versão" / "Version" / "Versión" — o numero vem do produto.ts. */
    readonly versaoPrefixo: string;
    readonly linhaMeta: string;
    readonly captura: Legenda;
  };

  /**
   * Os quatro numeros medidos. O `valor` NAO e traduzido de conteudo: so o
   * separador decimal muda (1,2 s em pt/es, 1.2 s em en). A `fonte` fica
   * visivel de proposito — numero sem procedencia e exatamente o que este
   * produto acusa o mercado de fazer.
   */
  readonly numeros: readonly {
    readonly valor: string;
    readonly legenda: string;
    readonly fonte: string;
  }[];

  readonly pilares: {
    readonly rotulo: string;
    readonly titulo: string;
    readonly itens: readonly {
      readonly numero: string;
      readonly rotulo: string;
      readonly titulo: string;
      readonly texto: string;
      readonly detalhe: string | null;
      readonly fonte: string;
    }[];
    readonly provaRotulo: string;
    readonly provaTitulo: string;
    readonly provaCaptura: Legenda;
  };

  readonly telas: {
    readonly rotulo: string;
    readonly titulo: string;
    /** `arquivo` bate com public/capturas/. Nome de arquivo nao se traduz. */
    readonly itens: readonly {
      readonly arquivo: string;
      readonly titulo: string;
      readonly descricao: string;
    }[];
  };

  readonly recusas: {
    readonly rotulo: string;
    readonly titulo: string;
    readonly itens: readonly {
      readonly titulo: string;
      readonly porque: string;
    }[];
  };

  readonly privacidade: {
    readonly rotulo: string;
    readonly titulo: string;
    readonly texto: string;
    readonly itens: readonly string[];
  };

  readonly licenca: {
    readonly rotulo: string;
    readonly titulo: string;
    readonly itens: readonly ItemLista[];
    readonly fluxoRotulo: string;
    readonly fluxo: readonly {
      readonly passo: string;
      readonly titulo: string;
      readonly texto: string;
    }[];
    readonly nota: string;
  };

  readonly precos: {
    readonly rotulo: string;
    readonly titulo: string;
    readonly texto: string;

    readonly planoRotulo: string;
    readonly planoTitulo: string;
    readonly planoResumo: string;
    readonly ctaTexto: string;

    /**
     * Caminho do checkout, SEM o BASE_URL (o componente prefixa).
     *
     * CONFIRMAR COM O DONO — hoje cada lingua aponta para o proprio prefixo
     * (/comprar, /en/comprar, /es/comprar). A pagina de checkout ainda nao
     * existe em nenhuma das tres. Se o destino final for um link unico do
     * Mercado Pago, basta repetir o mesmo valor nos tres arquivos.
     */

    readonly formasRotulo: string;
    readonly formas: readonly string[];
    readonly processadoPor: string;
    readonly requisito: string;

    readonly incluiRotulo: string;
    readonly inclui: readonly ItemLista[];

    /** Template do parcelamento. Desligado hoje — ver MOSTRAR_PARCELAMENTO. */
    readonly parcelamento: (parcelas: number, valor: string) => string;

    /**
     * A cobranca e em real, pelo Mercado Pago. Fora do Brasil isso PRECISA
     * estar dito: mostrar o valor em dolar ou em peso seria mentir sobre o
     * que o checkout vai cobrar. `null` em portugues, onde "R$" ja resolve.
     */
    readonly sufixoMoeda: string | null;
    readonly notaMoeda: string | null;

    readonly adicional: {
      readonly rotulo: string;
      readonly titulo: string;
      readonly texto: string;
      readonly observacao: string;
      readonly porComputador: string;
      readonly comprar: string;
    };
  };

  readonly faq: {
    readonly rotulo: string;
    readonly titulo: string;
    readonly texto: string;
    readonly itens: readonly ItemFaq[];
  };

  /**
   * A pagina /comprar.
   *
   * Ela existe porque a ORDEM importa e nao e obvia: a licenca nasce presa ao
   * codigo OTZ-XXXX-XXXX-XXXX, e esse codigo so aparece DEPOIS de instalar.
   * Quem chega querendo pagar primeiro precisa ser avisado, ou paga e nao tem
   * o que colar.
   */
  readonly comprar: {
    readonly tituloPagina: string;
    readonly descricaoPagina: string;
    readonly rotulo: string;
    readonly titulo: string;
    readonly texto: string;
    readonly passos: readonly {
      readonly numero: string;
      readonly titulo: string;
      readonly texto: string;
    }[];
    readonly acaoBaixar: string;
    readonly acaoDiscord: string;
    readonly notaDiscord: string;

    /**
     * O conferidor de formato. Confere APENAS a forma do codigo — nao tem como
     * saber se ele e de uma maquina de verdade, e o texto precisa dizer isso.
     * Existe para pegar o erro caro: pagar com codigo digitado errado.
     */
    readonly conferidor: {
      readonly rotulo: string;
      readonly titulo: string;
      readonly texto: string;
      readonly etiqueta: string;
      readonly botao: string;
      readonly certo: string;
      readonly errado: string;
      readonly vazio: string;
      readonly aviso: string;
    };

    readonly voltar: string;
  };

  readonly rodape: {
    readonly links: readonly { readonly href: string; readonly texto: string }[];
    readonly direitos: string;
  };
};
