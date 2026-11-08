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
  | "ajustes";

export interface Area {
  id: IdDaArea;
  nome: string;
  pergunta: string;
  /** Traço do objeto, num quadro de 24×24. */
  traco: string;
  /** O objeto 3D da mesa e do dock (`public/objetos/`, Fluent Emoji da Microsoft, licença MIT). */
  objeto: string;
  vai_ter: string[];
  onda: number;
}

export const areas: Area[] = [
  {
    id: "inicio",
    objeto: "/objetos/inicio.png",
    nome: "Início",
    pergunta: "Como está meu PC?",
    traco: "M4 11 12 4l8 7v9h-5v-6H9v6H4z",
    vai_ter: [],
    onda: 1,
  },
  {
    id: "turbo",
    objeto: "/objetos/turbo.png",
    nome: "Turbo",
    pergunta: "Me dá mais FPS",
    traco: "M13 2 5 14h6l-1 8 8-12h-6z",
    vai_ter: [
      "A receita automática pelo gargalo do seu jogo",
      "Frame Boost, Jogo liso e escala no mesmo lugar",
      "FPS do jogo, FPS na tela e leveza, sempre separados",
    ],
    onda: 2,
  },
  {
    id: "jogos",
    objeto: "/objetos/jogos.png",
    nome: "Jogos",
    pergunta: "E no meu jogo?",
    traco: "M6 8h12a4 4 0 0 1 4 4v2a3 3 0 0 1-5.4 1.8L15 14H9l-1.6 1.8A3 3 0 0 1 2 14v-2a4 4 0 0 1 4-4zM8 10v3M6.5 11.5h3M16 11h.01M18 13h.01",
    vai_ter: ["Perfil gráfico de cada jogo", "Histórico de partidas", "Prova antes e depois por jogo"],
    onda: 2,
  },
  {
    id: "rede",
    objeto: "/objetos/rede.png",
    nome: "Rede",
    pergunta: "Por que meu ping pula?",
    traco: "M12 20v-6M8.5 10.5a5 5 0 0 1 7 0M5.5 7.5a9 9 0 0 1 13 0M12 14h.01",
    vai_ter: [
      "Ping, jitter e perda ao vivo até o servidor do jogo",
      "Onde está o problema: sua casa, a operadora ou o servidor",
      "Downloads de fundo segurados enquanto você joga",
    ],
    onda: 3,
  },
  {
    id: "latencia",
    objeto: "/objetos/latencia.png",
    nome: "Latência",
    pergunta: "Por que o jogo parece atrasado?",
    traco: "M12 8v5l3 2M9 2h6M12 22a8 8 0 1 0 0-16 8 8 0 0 0 0 16z",
    vai_ter: ["Tempo do clique até a imagem, medido", "Taxa do seu mouse", "O driver que causa os picos"],
    onda: 3,
  },
  {
    id: "energia",
    objeto: "/objetos/energia.png",
    nome: "Energia",
    pergunta: "Meu processador está no máximo?",
    traco: "M7 7h10v13H7zM10 4h4v3M12 10l-2 4h4l-2 4",
    vai_ter: ["Plano que muda sozinho entre Jogo e Descanso", "Clock e temperatura ao vivo", "O que está segurando o processador"],
    onda: 3,
  },
  {
    id: "tela",
    objeto: "/objetos/tela.png",
    nome: "Tela",
    pergunta: "Meu monitor está certo?",
    traco: "M3 5h18v11H3zM8 20h8M12 16v4",
    vai_ter: ["Hz que o monitor aguenta e o que ele está usando", "VRR e HDR", "Como o quadro chega à tela"],
    onda: 3,
  },
  {
    id: "sistema",
    objeto: "/objetos/sistema.png",
    nome: "Sistema",
    pergunta: "O Windows está atrapalhando?",
    traco: "M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
    vai_ter: ["Todos os ajustes, por grupo", "Inicialização e serviços", "Espaço em disco e BIOS"],
    onda: 2,
  },
  {
    id: "prova",
    objeto: "/objetos/prova.png",
    nome: "Prova",
    pergunta: "Funcionou mesmo?",
    traco: "M9 3h6M10 3v6l-5 9a2 2 0 0 0 1.8 3h10.4a2 2 0 0 0 1.8-3l-5-9V3M7.5 15h9",
    vai_ter: ["Laboratório de antes e depois", "Relatório para compartilhar"],
    onda: 2,
  },
  {
    id: "historico",
    objeto: "/objetos/historico.png",
    nome: "Histórico",
    pergunta: "O que foi mudado?",
    traco: "M6 2h12M6 22h12M7 2c0 6 10 6 10 10S7 16 7 22M17 2c0 6-10 6-10 10s10 4 10 10",
    vai_ter: ["Tudo o que o Otimiza mudou, com o valor de antes", "Desfazer um por um ou tudo de uma vez"],
    onda: 1,
  },
  {
    id: "ajustes",
    objeto: "/objetos/ajustes.png",
    nome: "Ajustes",
    pergunta: "",
    traco: "M4 6h16M4 12h16M4 18h16M9 4v4M15 10v4M7 16v4",
    vai_ter: ["Tema, mascote e sons", "Sobreposição no jogo", "Licença e canal beta"],
    onda: 2,
  },
];
