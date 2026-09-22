/**
 * Tudo o que o site afirma sobre o Otimiza mora aqui.
 *
 * Os números e as frases saíram do próprio produto: `site/src/data/i18n/pt.ts`,
 * `site/src/data/precos.ts`, `site/src/data/produto.ts` e o catálogo do app.
 * Nenhum foi arredondado para soar melhor, e nenhum foi inventado — o Otimiza
 * se vende dizendo que o mercado anuncia número que ninguém confere.
 */

export const site = {
  name: "Otimiza",
  /*
   * Endereço público, SEM barra no fim. É a base das URLs canônicas e do Open
   * Graph. Se um dia houver domínio próprio, troque aqui e o PAGES_BASE_PATH
   * da esteira (`.github/workflows/site.yml`).
   */
  url: "https://otimiza-oficial.github.io/Otimiza",
  titulo: "Otimiza — Console de desempenho para Windows",
  description:
    "Mede o que o seu PC faz, otimiza o que dá, e prova com número — inclusive quando o número diz que não mudou nada.",
  /*
   * O NÚMERO QUE O BOTÃO DE BAIXAR MOSTRA — e que precisa andar junto com a
   * tag, nesta ordem: primeiro a tag `v<versao>`, depois este arquivo em
   * `main`.
   *
   * O motivo é que `links.baixar` aponta para `releases/latest`, e não para
   * uma versão fixa. Se este número subir antes de a versão existir, o botão
   * passa a dizer "Baixar o Otimiza 2.8.0" e entregar o instalador da 2.7.0 —
   * o site prometendo o que ainda não dá para baixar, que é exatamente o tipo
   * de coisa que o produto inteiro existe para não fazer.
   *
   * Na direção contrária o erro é pequeno e se conserta sozinho: o painel
   * prefere a versão que vem da API de releases e só cai neste valor quando a
   * leitura falha.
   */
  versao: "2.8.0",
} as const;

export const links = {
  /** Endereço permanente do instalador mais novo, publicado pela esteira. */
  baixar: "https://github.com/Otimiza-Oficial/Otimiza/releases/latest/download/Otimiza-instalador.exe",
  /** O mesmo convite que o app abre (`.github/convite.json`). */
  discord: "https://discord.gg/ultimus",
  /** A página que explica o preço. Com o checkout ligado, ela também cobra;
      sem ele, ela leva ao Discord, onde a compra já funciona. */
  comprar: "/comprar/",
  entrar: "/entrar/",
  painel: "/painel/",
} as const;

export const nav = [
  { label: "Recursos", href: "#recursos" },
  { label: "Suporte", href: "#suporte" },
  { label: "Preço", href: "#preco" },
  { label: "Perguntas", href: "#perguntas" },
] as const;

/** As sete abas do app, na ordem da lateral, com a captura real de cada uma. */
export const telas = [
  { id: "painel", nome: "Painel", legenda: "O que está travando este PC, medido na hora e dito na cara." },
  { id: "diagnostico", nome: "Diagnóstico", legenda: "Monitores, memória instalada e os achados desta máquina." },
  { id: "otimizacoes", nome: "Otimizações", legenda: "Medir antes, otimizar, medir de novo — e o catálogo dizendo o que já está otimizado." },
  { id: "jogos", nome: "Jogos", legenda: "A placa de vídeo, o driver e a configuração do jogo que mais mexe no FPS." },
  { id: "espaco", nome: "Espaço", legenda: "O que dá para liberar — e, antes disso, onde o disco realmente foi parar." },
  { id: "sistema", nome: "Sistema", legenda: "O que sobe junto com o Windows, nos três lugares onde isso se esconde." },
  { id: "reparo", nome: "Reparo", legenda: "As ferramentas do Windows que devolvem arquivo de sistema danificado ao original." },
] as const;

/*
 * Fotos (licença Unsplash). Aparecem SÓ dentro de telas ilustrativas, cada uma
 * de uma pessoa diferente, e nunca como depoimento ou cliente real.
 */
const foto = (id: string) => `https://images.unsplash.com/${id}?w=160&h=160&fit=crop&crop=faces&q=80`;
export const pessoas = {
  rafael: { nome: "Rafael", foto: foto("photo-1500648767791-00dcc994a43e") },
  camila: { nome: "Camila", foto: foto("photo-1580489944761-15a19d654956") },
  diego: { nome: "Diego", foto: foto("photo-1507003211169-0a1dd7228f2d") },
  julia: { nome: "Júlia", foto: foto("photo-1494790108377-be9c29b29330") },
  bruno: { nome: "Bruno", foto: foto("photo-1531750026848-8ada78f641c2") },
  larissa: { nome: "Larissa", foto: foto("photo-1607503873903-c5e95f80d7b9") },
} as const;

export const texturaCetim =
  "https://images.unsplash.com/photo-1705674337411-3b89e5afcc11?w=2000&q=75";

/* O preço é o mesmo que o bot do Discord cobra (`site/src/data/precos.ts`). */
export const PRECO_BRL = 25;
export const PRECO_ADICIONAL_BRL = 25;

export const formatarReais = (valor: number) =>
  valor.toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
