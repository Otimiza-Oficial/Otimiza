/** Tudo o que o site afirma sobre o Otimiza. Nenhum número foi arredondado para soar melhor, nem inventado. */

export const site = {
  name: "Otimiza",
  /*
   * Sem barra no fim. Com domínio próprio, troque aqui e o PAGES_BASE_PATH da esteira.
   */
  url: "https://otimiza-oficial.github.io/Otimiza",
  titulo: "Otimiza — Console de desempenho para Windows",
  description:
    "Mede o que o seu PC faz, otimiza o que dá, e prova com número — inclusive quando o número diz que não mudou nada.",
  /*
   * O número do botão de baixar: primeiro a tag v<versao>, depois este arquivo.
   * links.baixar aponta para releases/latest; subir antes faria o botão prometer uma versão e entregar a anterior.
   */
  versao: "2.9.0",
} as const;

export const links = {
  baixar: "https://github.com/Otimiza-Oficial/Otimiza/releases/latest/download/Otimiza-instalador.exe",
  discord: "https://discord.gg/ultimus",
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
 * Fotos da Unsplash, só dentro de telas ilustrativas; nunca como depoimento ou cliente real.
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

/*
 * O mesmo preço que o bot cobra (catalogo.js, item otimiza, centavos 4000). Desalinhado, o site anunciaria um preço e cobraria outro.
 */
export const PRECO_BRL = 40;
export const PRECO_ADICIONAL_BRL = 40;

export const formatarReais = (valor: number) =>
  valor.toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
