/** Tudo o que o site afirma sobre o Otimiza. Nenhum número foi arredondado para soar melhor, nem inventado. */

export const site = {
  name: "Otimiza",
  /*
   * Sem barra no fim. Com domínio próprio, troque aqui e o PAGES_BASE_PATH da esteira.
   */
  url: "https://otimiza-oficial.github.io/Otimiza",
  titulo: "Otimiza 1.0 — o céu do seu PC, medido",
  description:
    "Otimizador de Windows para quem joga: lê o seu PC, liga só o que ajuda nele e prova com número antes e depois — inclusive quando o número diz que não mudou nada.",
  /*
   * O número do botão de baixar: primeiro a tag v<versao>, depois este arquivo.
   * links.baixar aponta para releases/latest; subir antes faria o botão prometer uma versão e entregar a anterior.
   *
   * Nova Geração: por dentro 4.0.0, para o cliente "1.0" (regra do plano: exibida = (maior − 3).menor). O site só
   * mostra a exibida, por `versaoDeExibicao`.
   */
  versao: "4.0.0",
} as const;

export const links = {
  baixar: "https://github.com/Otimiza-Oficial/Otimiza/releases/latest/download/Otimiza-instalador.exe",
  discord: "https://discord.gg/ultimus",
  comprar: "/comprar/",
  entrar: "/entrar/",
  painel: "/painel/",
} as const;

export const nav = [
  { label: "O que faz", href: "#recursos" },
  { label: "Prova", href: "#prova" },
  { label: "Preço", href: "#preco" },
  { label: "Suporte", href: "#suporte" },
  { label: "Perguntas", href: "#perguntas" },
] as const;

/**
 * A versão como o cliente a vê. Da 4.0.0 em diante, a Nova Geração recomeça a contagem: 4.0.0 → "1.0",
 * 4.1.2 → "1.1.2". Antes disso (3.x) o número é o de sempre.
 */
export function versaoDeExibicao(interna: string): string {
  const [maior, menor = "0", correcao = "0"] = interna.replace(/^v/, "").split(".");
  const m = Number(maior);
  if (!Number.isFinite(m) || m < 4) return interna.replace(/^v/, "");
  return `${m - 3}.${menor}${correcao !== "0" ? `.${correcao}` : ""}`;
}

/** "1.0" — o nome da versão atual em todo texto do site. */
export const VERSAO_EXIBIDA = versaoDeExibicao(site.versao);

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

/*
 * O mesmo preço que o bot cobra (catalogo.js, item otimiza, centavos 4000). Desalinhado, o site anunciaria um preço e cobraria outro.
 */
export const PRECO_BRL = 40;
export const PRECO_ADICIONAL_BRL = 40;

export const formatarReais = (valor: number) =>
  valor.toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
