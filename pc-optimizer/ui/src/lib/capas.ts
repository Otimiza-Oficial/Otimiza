// A capa de cada jogo quando o cache da Steam deste PC não tem: a arte oficial pela internet. Ordem de preferência:
// 1) o cache local da Steam (`ponte.capa`, sem rede); 2) a CDN da Steam pelo número do jogo; 3) a arte oficial da loja
// do próprio jogo, para os grandes que não estão na Steam. Sem nada disso (ou se a imagem não carregar), a tela desenha
// a capa com as iniciais do jogo (`iniciais`). As URLs são imagens públicas (carregadas por <img>, sem chamada de API).

const steam = (appid: number) => `https://cdn.cloudflare.steamstatic.com/steam/apps/${appid}/library_600x900_2x.jpg`;

/** Jogos conhecidos pelo nome, para quando a loja do PC não deu o número da Steam. */
const CONHECIDOS: Record<string, number | string> = {
  // O FiveM tem marca própria: a capa leva o logo dele, não a arte do GTA V (que fica para o GTA V).
  fivem: "/capas/fivem.svg",
  // O RedM roda sobre o Red Dead Redemption 2: a capa é a do jogo base.
  grandtheftautov: 271590,
  grandtheftautovlegacy: 271590,
  grandtheftautovenhanced: 3240220,
  redm: 1174180,
  reddeadredemption2: 1174180,
  counterstrike2: 730,
  counterstrikeglobaloffensive: 730,
  dota2: 570,
  apexlegends: 1172470,
  pubgbattlegrounds: 578080,
  rust: 252490,
  rocketleague: 252950,
  callofduty: 1938090,
  callofdutywarzone: 1938090,
  eafc25: 2669320,
  deltaforce: 2507950,
  marvelrivals: 2767030,
  thefinals: 2073850,
  rainbowsixsiege: 359550,
  tomclancysrainbowsixsiege: 359550,
  overwatch2: 2357570,
  destiny2: 1085660,
  warframe: 230410,
  eldenring: 1245620,
  cyberpunk2077: 1091500,
  // Fora da Steam: a arte oficial da loja do jogo.
  fortnite:
    "https://cdn2.unrealengine.com/fnjn-33-10-stormking-egs-launcher-keyart-blade-1200x1600-1200x1600-c2a65f0f0e2e.jpg",
  roblox: "https://images.rbxcdn.com/5348266ea6c5e67b19d6a814cbbb70f6.jpg",
};

/** O mesmo jogo com nome de outra fonte: o instalador chama de "Roblox Player" o que o Otimiza mede como "Roblox". */
const APELIDOS: Record<string, string> = {
  robloxplayer: "roblox",
  robloxplayerbeta: "roblox",
  fivemclient: "fivem",
  citizenfxfivem: "fivem",
};

/** O mesmo jogo escrito de jeitos diferentes ("Grand Theft Auto V", "GTA V Legacy", "Roblox Player") vira a mesma chave. */
export function chaveDoJogo(nome: string): string {
  const chave = nome
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
  return APELIDOS[chave] ?? chave;
}

/** A capa pela internet, ou `null` quando não há arte conhecida (a tela desenha uma). */
export function capaRemota(nome: string, appid: number | null): string | null {
  const conhecido = CONHECIDOS[chaveDoJogo(nome)];
  if (typeof conhecido === "string") return conhecido;
  if (typeof conhecido === "number") return steam(conhecido);
  if (appid != null) return steam(appid);
  return null;
}

/** Sem capa: as iniciais do jogo num quadro neutro. Cor inventada pelo nome parece enfeite e não ajuda a achar. */
export function iniciais(nome: string) {
  const palavras = nome.replace(/[^\p{L}\p{N} ]/gu, " ").split(/\s+/).filter(Boolean);
  return (palavras.length > 1 ? palavras[0][0] + palavras[1][0] : (palavras[0] ?? "?").slice(0, 2)).toUpperCase();
}
