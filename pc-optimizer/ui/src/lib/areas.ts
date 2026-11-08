// As áreas do dock (plano, 4.2). Cada uma responde uma pergunta do jogador. Enquanto a tela de uma área não migra,
// a janela dela diz o que vai morar ali e em que onda chega — nunca um botão que finge funcionar.

export type IdDaArea =
  | "inicio"
  | "turbo"
  | "jogos"
  | "rede"
  | "latencia"
  | "energia"
  | "tela"
  | "sistema"
  | "prova"
  | "historico"
  | "mira"
  | "cores"
  | "ajustes";

export interface Area {
  id: IdDaArea;
  nome: string;
  pergunta: string;
  /** Traço do objeto, num quadro de 24×24. */
  traco: string;
  vai_ter: string[];
  onda: number;
}

export const areas: Area[] = [
  {
    id: "inicio",
    nome: "Início",
    pergunta: "Como está meu PC?",
    traco: "M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z",
    vai_ter: [],
    onda: 1,
  },
  {
    id: "turbo",
    nome: "Turbo",
    pergunta: "Me dá mais FPS",
    traco: "M13 2 4 14h7l-1 8 9-12h-7z",
    vai_ter: [
      "A receita automática pelo gargalo do seu jogo",
      "Frame Boost, Jogo liso e escala no mesmo lugar",
      "FPS do jogo, FPS na tela e leveza, sempre separados",
    ],
    onda: 2,
  },
  {
    id: "jogos",
    nome: "Jogos",
    pergunta: "E no meu jogo?",
    traco: "M6 9h12a4 4 0 0 1 4 4v1.5a2.5 2.5 0 0 1-4.5 1.5L16 14H8l-1.5 2A2.5 2.5 0 0 1 2 14.5V13a4 4 0 0 1 4-4zM8 11v3M6.5 12.5h3M15.5 12h.01M17.5 13.5h.01",
    vai_ter: ["Perfil gráfico de cada jogo", "Histórico de partidas", "Prova antes e depois por jogo"],
    onda: 2,
  },
  {
    id: "rede",
    nome: "Rede",
    pergunta: "Por que meu ping pula?",
    traco: "M2 8.5a15 15 0 0 1 20 0M5 12a10 10 0 0 1 14 0M8.5 15.5a5 5 0 0 1 7 0M12 19h.01",
    vai_ter: [
      "Ping, jitter e perda ao vivo até o servidor do jogo",
      "Onde está o problema: sua casa, a operadora ou o servidor",
      "Downloads de fundo segurados enquanto você joga",
    ],
    onda: 3,
  },
  {
    id: "latencia",
    nome: "Latência",
    pergunta: "Por que o jogo parece atrasado?",
    traco: "M12 8v5l3 2M9 2h6M12 22a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
    vai_ter: ["Tempo do clique até a imagem, medido", "Taxa do seu mouse", "O driver que causa os picos"],
    onda: 3,
  },
  {
    id: "energia",
    nome: "Energia",
    pergunta: "Meu processador está no máximo?",
    traco: "M7 6h10a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2zM10 3h4M12.5 10 10 14h4l-2.5 4",
    vai_ter: ["Plano que muda sozinho entre Jogo e Descanso", "Clock e temperatura ao vivo", "O que está segurando o processador"],
    onda: 3,
  },
  {
    id: "tela",
    nome: "Tela",
    pergunta: "Meu monitor está certo?",
    traco: "M4 4h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM8 20h8M12 16v4",
    vai_ter: ["Hz que o monitor aguenta e o que ele está usando", "VRR e HDR", "Como o quadro chega à tela"],
    onda: 3,
  },
  {
    id: "sistema",
    nome: "Sistema",
    pergunta: "O Windows está atrapalhando?",
    traco: "M8 7h8a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H8a1 1 0 0 1-1-1V8a1 1 0 0 1 1-1zM10 3v4M14 3v4M10 17v4M14 17v4M3 10h4M3 14h4M17 10h4M17 14h4",
    vai_ter: ["Todos os ajustes, por grupo", "Inicialização e serviços", "Espaço em disco e BIOS"],
    onda: 2,
  },
  {
    id: "prova",
    nome: "Prova",
    pergunta: "Funcionou mesmo?",
    traco: "M4 20V11M10 20V5M16 20v-8M21 20H3",
    vai_ter: ["Laboratório de antes e depois", "Relatório para compartilhar"],
    onda: 2,
  },
  {
    id: "historico",
    nome: "Histórico",
    pergunta: "O que foi mudado?",
    traco: "M3 12a9 9 0 1 0 2.6-6.4L3 8M3 3v5h5M12 7v5l3 2",
    vai_ter: ["Tudo o que o Otimiza mudou, com o valor de antes", "Desfazer um por um ou tudo de uma vez"],
    onda: 1,
  },
  {
    id: "mira",
    nome: "Mira",
    pergunta: "Mira na tela, sensibilidade e FiveM",
    traco: "M12 3v4M12 17v4M3 12h4M17 12h4M12 12h.01",
    vai_ter: [],
    onda: 2,
  },
  {
    id: "cores",
    nome: "Cores",
    pergunta: "Cores do monitor, por jogo",
    traco: "M12 3a9 9 0 1 0 0 18c1 0 1.5-.8 1.5-1.6 0-1-.8-1.4-.8-2.2 0-.9.7-1.6 1.6-1.6H17a4 4 0 0 0 4-4c0-4.7-4-8.6-9-8.6z",
    vai_ter: [],
    onda: 2,
  },
  {
    id: "ajustes",
    nome: "Ajustes",
    pergunta: "",
    traco: "M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1M15 4v4M9 10v4M17 16v4",
    vai_ter: ["Tema, mascote e sons", "Sobreposição no jogo", "Licença e canal beta"],
    onda: 2,
  },
];
