// O que o Oti diz. Cada fala vem de um dado lido (os avisos de `clima.ts`) ou explica uma área; nenhuma promete o que a
// área não faz. A barra de pergunta entende por palavra-chave, não é IA, e quando não entende diz isso.

import type { Gesto, Humor } from "../componentes/Oti.svelte";
import type { IdDaArea } from "./areas";
import type { Aviso } from "./clima";

export type Acao =
  | { rotulo: string; tipo: "abrir"; area: IdDaArea; principal?: boolean }
  | { rotulo: string; tipo: "falar"; fala: () => Fala; principal?: boolean };

export interface Fala {
  texto: string;
  detalhe?: string;
  humor: Humor;
  gesto?: Gesto;
  acoes: Acao[];
}

const SUGESTOES: Acao[] = [
  { rotulo: "Meu ping pula", tipo: "abrir", area: "rede" },
  { rotulo: "O jogo parece atrasado", tipo: "abrir", area: "latencia" },
  { rotulo: "Meu monitor está certo?", tipo: "abrir", area: "tela" },
  { rotulo: "O que você mudou no PC?", tipo: "abrir", area: "historico" },
];

export function ociosa(): Fala {
  return {
    texto: "O que você quer melhorar hoje?",
    detalhe: "Escolha um assunto ou me pergunte ali embaixo.",
    humor: "atento",
    acoes: SUGESTOES,
  };
}

function explicarHz(): Fala {
  return {
    texto: "Hz é quantas imagens o monitor mostra por segundo.",
    detalhe:
      "Em 60 Hz, tudo que sua placa faz acima de 60 FPS não chega aos seus olhos. Na taxa certa, o jogo fica mais liso e o clique aparece antes. Dá para desfazer pelo Histórico.",
    humor: "falando",
    gesto: "apontar",
    acoes: [
      { rotulo: "Arrumar agora", tipo: "abrir", area: "tela", principal: true },
      { rotulo: "Depois", tipo: "falar", fala: ociosa },
    ],
  };
}

/** A primeira coisa que ele diz ao abrir, a partir do que foi lido. */
export function abertura(avisos: Aviso[]): Fala {
  const tela = avisos.find((a) => a.id.startsWith("hz-"));
  if (tela) {
    return {
      texto: `${tela.titulo}.`,
      detalhe: "Corrigir leva um clique, e dá para desfazer quando quiser. Quer que eu arrume?",
      humor: "preocupado",
      gesto: "apontar",
      acoes: [
        { rotulo: "Arrumar", tipo: "abrir", area: "tela", principal: true },
        { rotulo: "Por quê?", tipo: "falar", fala: explicarHz },
        { rotulo: "Depois", tipo: "falar", fala: ociosa },
      ],
    };
  }
  const outro = avisos[0];
  if (outro) {
    return {
      texto: `${outro.titulo}.`,
      detalhe: outro.detalhe,
      humor: "preocupado",
      acoes: [{ rotulo: "Entendi", tipo: "falar", fala: ociosa, principal: true }],
    };
  }
  return {
    texto: "Oi! Olhei a tela e a memória e está tudo certo.",
    detalhe: "Rede, temperatura e disco eu aprendo nas próximas versões. O que você quer melhorar?",
    humor: "feliz",
    gesto: "acenar",
    acoes: SUGESTOES,
  };
}

/** O que ele diz ao lado de cada área aberta. */
export function narracao(area: IdDaArea, onda: number, nome: string): string {
  switch (area) {
    case "rede":
      return "Aqui eu meço a rota até o servidor, salto a salto, e digo se o problema é sua casa, a operadora ou o servidor.";
    case "latencia":
      return "Use o PC normal enquanto eu meço. Eu acho o driver que segura o processador e vira travadinha.";
    case "tela":
      return "Eu leio o que o monitor aguenta direto da ficha dele. Se der para melhorar num clique, eu mostro.";
    case "historico":
      return "Tudo o que eu mudei está aqui, com o valor de antes. Desfazer nunca depende de licença.";
    default:
      return `${nome} ainda está sendo construída. Chega na Onda ${onda}; até lá, ela continua no Otimiza de sempre.`;
  }
}

const PALAVRAS: { area: IdDaArea; chaves: string[] }[] = [
  { area: "rede", chaves: ["ping", "lag", "internet", "wifi", "wi-fi", "rede", "conex", "servidor", "perda", "jitter", "roteador"] },
  {
    area: "latencia",
    chaves: ["atras", "delay", "travad", "engasg", "input", "dpc", "audio", "áudio", "picot", "latên", "laten", "stutter"],
  },
  { area: "tela", chaves: ["hz", "monitor", "tela", "hertz", "taxa", "vrr", "hdr", "g-sync", "freesync"] },
  { area: "historico", chaves: ["desfaz", "mudou", "mudanç", "histór", "voltar", "reverter", "restaur"] },
  { area: "turbo", chaves: ["fps", "turbo", "desempenho", "otimiz", "rápid", "rapid", "lento"] },
  { area: "jogos", chaves: ["jogo", "fivem", "cs2", "valorant", "fortnite", "roblox", "minecraft", "gta"] },
  { area: "energia", chaves: ["energia", "bateria", "plano", "processador", "cpu", "clock", "temperatura"] },
];

/** A pergunta da barra vira uma área, ou ele admite que não entendeu. */
export function entender(pergunta: string): { area: IdDaArea | null; fala: Fala } {
  const p = pergunta.toLowerCase();
  const achada = PALAVRAS.find((g) => g.chaves.some((c) => p.includes(c)));
  if (!achada) {
    return {
      area: null,
      fala: {
        texto: "Ainda não sei responder isso.",
        detalhe: "Por enquanto eu entendo de ping e rede, atraso e travadinha, monitor e Hz, e do que eu mudei no PC.",
        humor: "pensando",
        acoes: SUGESTOES,
      },
    };
  }
  return {
    area: achada.area,
    fala: { texto: "Deixa comigo.", humor: "feliz", gesto: "apontar", acoes: [] },
  };
}

const DICAS = [
  "Ei! Pode me perguntar coisas como \"por que meu ping pula?\".",
  "Quando você abre um jogo, eu boto o headset e fico quietinho.",
  "Tudo o que eu mudo no PC dá para desfazer no Histórico.",
  "Número com o selo Medido saiu de uma medição, nunca de chute.",
];

let dica = 0;
export function cutucada(): Fala {
  return { texto: DICAS[dica++ % DICAS.length], humor: "feliz", gesto: "comemorar", acoes: SUGESTOES };
}

export function jogoAbriu(nome: string): Fala {
  return {
    texto: `Bom jogo no ${nome}!`,
    detalhe: "Vou ficar quietinho no canto para não pesar.",
    humor: "jogando",
    acoes: [],
  };
}

export function jogoFechou(nome: string): Fala {
  return {
    texto: "E aí, como foi?",
    detalhe: `Se você jogou ${nome} comigo aberto, a partida está em Jogos.`,
    humor: "feliz",
    gesto: "comemorar",
    acoes: [
      { rotulo: "Ver a partida", tipo: "abrir", area: "jogos", principal: true },
      { rotulo: "Depois", tipo: "falar", fala: ociosa },
    ],
  };
}
