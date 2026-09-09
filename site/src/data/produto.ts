/**
 * O que o produto e, em dados que NAO se traduzem.
 *
 * Nome e numero de versao sao os mesmos nas tres linguas: sao identificadores,
 * nao texto. Tudo que muda de lingua para lingua vive em src/data/i18n/.
 *
 * NAO acrescente link para o REPOSITORIO nem para a pagina de Releases: eles
 * foram removidos a pedido do dono. O link do INSTALADOR abaixo e a unica
 * excecao, e e deliberada — sem ele o funil nao existe, porque o codigo da
 * maquina so aparece depois de instalar.
 *
 * A afirmacao de que o codigo-fonte e publico PODE ser feita: o repositorio
 * segue publico por decisao do dono, entao a frase e verdadeira.
 */
export const produto = {
  nome: "Otimiza",
  versao: "1.9.0",

  /**
   * Endereco PERMANENTE do instalador da versao mais nova.
   *
   * Nao aponta para a pagina do release de proposito: la existem dois arquivos
   * parecidos (.exe e .msi) mais os codigos-fonte, e cada escolha que a pessoa
   * nao sabe fazer e uma desistencia possivel.
   *
   * Este nome fixo e publicado a cada versao por um passo dedicado do
   * .github/workflows/release.yml, que falha alto se nao achar o arquivo. E o
   * mesmo link que o bot do Discord manda para quem vai comprar — site e bot
   * precisam mandar o mesmo, senao uma hora divergem.
   */
  instalador:
    "https://github.com/Otimiza-Oficial/Otimiza/releases/latest/download/Otimiza-instalador.exe",
} as const;
