// O CATÁLOGO DE JOGOS: a fonte única do que cada jogo ganha na aba Jogos. Antes, o jogo era reconhecido por expressões
// soltas no `Jogos.svelte` e os caches e o texto do anticheat moravam dentro do `CatalogoDoJogo.svelte`. Um jogo novo
// entra aqui (ver a skill `jogo-no-catalogo`), com a ficha em `docs/jogos/<id>.md`.

export type IdDoCatalogo = "fivem" | "valorant" | "cs2" | "fortnite" | "apex";

/** Painéis próprios de um jogo, além do catálogo comum. */
export type Extra = "fivem" | "cs2_gsi";

export interface JogoDoCatalogo {
  id: IdDoCatalogo;
  nome: string;
  /** Pelo nome que a biblioteca mostra e, quando há, o appid da Steam. */
  nomes: RegExp;
  appid: number | null;
  /** O perfil leve do `configmotor.rs` (o mesmo id), ou `null` quando a configuração do jogo mora em outro lugar. */
  perfil: string | null;
  /** Onde a configuração mora quando não é pelo perfil leve. */
  configEmOutroLugar: string | null;
  /** Os caches de shader que este jogo usa (ids de `shaders.rs::locais`). */
  caches: string[];
  /** Qual anticheat, e o que o Otimiza faz ou deixa de fazer por causa dele. */
  anticheat: string;
  extras: Extra[];
  ficha: string;
}

/** Os caches do driver de vídeo: valem para todo jogo em DirectX. */
const DO_DRIVER = ["nvidia_dx", "amd_dx", "intel", "windows"];

export const CATALOGO: JogoDoCatalogo[] = [
  {
    id: "fivem",
    nome: "FiveM",
    nomes: /fivem/i,
    appid: null,
    perfil: null,
    configEmOutroLugar:
      "A configuração gráfica do GTA no FiveM fica no Turbo (Perfil do jogo): competitivo ou equilibrado, com desfazer.",
    caches: DO_DRIVER,
    anticheat:
      "O FiveM não tem anticheat que proíba sobreposição: o Frame Boost e a mira funcionam. Alguns servidores usam anticheat próprio; o Otimiza não injeta nada nem mexe no processo do jogo.",
    extras: ["fivem"],
    ficha: "docs/jogos/fivem.md",
  },
  {
    id: "valorant",
    nome: "Valorant",
    nomes: /valorant/i,
    appid: null,
    perfil: "valorant",
    configEmOutroLugar: null,
    caches: DO_DRIVER,
    anticheat:
      "O Vanguard vigia tudo que encosta no jogo. Com o Valorant aberto, o Otimiza não mexe no processo dele nem desenha por cima: o Frame Boost e a mira ficam desligados durante a partida. O que vale aqui é a configuração acima, o Turbo deste jogo e o NVIDIA Reflex no menu de vídeo do próprio Valorant.",
    extras: [],
    ficha: "docs/jogos/valorant.md",
  },
  {
    id: "cs2",
    nome: "Counter-Strike 2",
    nomes: /counter-strike 2|^cs2$/i,
    appid: 730,
    perfil: "cs2",
    configEmOutroLugar: null,
    caches: ["steam_cs2", ...DO_DRIVER],
    anticheat:
      "O VAC não trata sobreposição como suspeita: o Frame Boost e a mira funcionam no CS2. A integração oficial da Valve, abaixo, mede o FPS só com a rodada valendo.",
    extras: ["cs2_gsi"],
    ficha: "docs/jogos/cs2.md",
  },
  {
    id: "fortnite",
    nome: "Fortnite",
    nomes: /fortnite/i,
    appid: null,
    perfil: "fortnite",
    configEmOutroLugar: null,
    caches: DO_DRIVER,
    anticheat:
      "O Fortnite usa Easy Anti-Cheat e BattlEye. O Otimiza não injeta nada nem mexe no processo do jogo: o Frame Boost e a mira são janelas por cima, como as do Discord. O ganho mais seguro vem da configuração acima e do modo Desempenho do próprio Fortnite.",
    extras: [],
    ficha: "docs/jogos/fortnite.md",
  },
  {
    id: "apex",
    nome: "Apex Legends",
    nomes: /apex legends|^apex$/i,
    appid: 1172470,
    perfil: "apex",
    configEmOutroLugar: null,
    caches: ["steam_apex", ...DO_DRIVER],
    anticheat:
      "O Apex usa Easy Anti-Cheat. O Otimiza não injeta nada nem mexe no processo do jogo: o Frame Boost e a mira são janelas por cima, como as do Discord.",
    extras: [],
    ficha: "docs/jogos/apex.md",
  },
];

/** O jogo do catálogo, pelo nome que a biblioteca mostra e pelo appid da Steam (quando há). */
export function doCatalogo(nome: string, appid?: number | null): JogoDoCatalogo | null {
  return CATALOGO.find((j) => (j.appid != null && appid === j.appid) || j.nomes.test(nome)) ?? null;
}
