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
  vai_ter: string[];
  onda: number;
}

export const areas: Area[] = [
  {
    id: "inicio",
    nome: "Início",
    pergunta: "Como está meu PC?",
    vai_ter: [],
    onda: 1,
  },
  {
    id: "turbo",
    nome: "Turbo",
    pergunta: "Me dá mais FPS",
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
    vai_ter: ["Perfil gráfico de cada jogo", "Histórico de partidas", "Prova antes e depois por jogo"],
    onda: 2,
  },
  {
    id: "rede",
    nome: "Rede",
    pergunta: "Por que meu ping pula?",
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
    vai_ter: ["Tempo do clique até a imagem, medido", "Taxa do seu mouse", "O driver que causa os picos"],
    onda: 3,
  },
  {
    id: "energia",
    nome: "Energia",
    pergunta: "Meu processador está no máximo?",
    vai_ter: ["Plano que muda sozinho entre Jogo e Descanso", "Clock e temperatura ao vivo", "O que está segurando o processador"],
    onda: 3,
  },
  {
    id: "tela",
    nome: "Tela",
    pergunta: "Meu monitor está certo?",
    vai_ter: ["Hz que o monitor aguenta e o que ele está usando", "VRR e HDR", "Como o quadro chega à tela"],
    onda: 3,
  },
  {
    id: "sistema",
    nome: "Sistema",
    pergunta: "O Windows está atrapalhando?",
    vai_ter: ["Todos os ajustes, por grupo", "Inicialização e serviços", "Espaço em disco e BIOS"],
    onda: 2,
  },
  {
    id: "prova",
    nome: "Prova",
    pergunta: "Funcionou mesmo?",
    vai_ter: ["Laboratório de antes e depois", "Relatório para compartilhar"],
    onda: 2,
  },
  {
    id: "historico",
    nome: "Histórico",
    pergunta: "O que foi mudado?",
    vai_ter: ["Tudo o que o Otimiza mudou, com o valor de antes", "Desfazer um por um ou tudo de uma vez"],
    onda: 1,
  },
  {
    id: "mira",
    nome: "Mira",
    pergunta: "Mira na tela, sensibilidade e FiveM",
    vai_ter: [],
    onda: 2,
  },
  {
    id: "cores",
    nome: "Cores",
    pergunta: "Cores do monitor, por jogo",
    vai_ter: [],
    onda: 2,
  },
  {
    id: "ajustes",
    nome: "Ajustes",
    pergunta: "",
    vai_ter: ["Tema, mascote e sons", "Sobreposição no jogo", "Licença e canal beta"],
    onda: 2,
  },
];
