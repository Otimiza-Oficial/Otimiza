# Direção visual da interface nova

Lido antes de qualquer tela nova. Código é a fonte da verdade: `src/App.svelte`, `src/componentes/Oti.svelte`,
`src/componentes/Icone.svelte`, `src/tokens.css` e `src/lib/conversa.ts`.

## Referências do dono (07/11/2026)
- **Mascote**: o disco branco liso com dois olhos em cápsula inclinados, cavados com chanfro claro, cabeça meio de
  lado. Sem boca, sem mãos: os olhos são a expressão inteira.
- **Interface**: painel de vidro fosco no estilo visionOS sobre um cômodo desfocado, com a mesma estrutura:
  - barra de cima em pílula: avatar + "Bem-vindo de volta", busca com o assistente (o Oti no lugar do orbe),
    Claro/Escuro segmentado e o sino de avisos;
  - trilho vertical de ícones à esquerda (item ativo em círculo branco);
  - painel grande com a grade de blocos: Seus jogos (busca + 3 capas), Última partida (FPS médio e 1% pior),
    Otimizar (o botão principal + atalhos), Agora (processador, memória, tela e Wi-Fi com nome e valor), Durante o
    jogo (downloads de fundo, Wi-Fi sem procurar redes, driver que trava), Avisos e Tela (Hz em uso × o que aguenta).
    A estrutura veio da referência; os controles de casa inteligente dela (player, setas, anéis) saíram em 09/11/2026
    porque fingiam funções que não existem;
  - abas embaixo com a pílula branca que desliza, e o "+" com as outras áreas.

## Comportamento
- Cada bloco mostra um dado lido deste PC ou liga algo que existe de verdade. O que ainda não migrou abre a área
  dizendo a onda em que chega. Nada finge funcionar.
- A luz do vidro segue o cursor em cada bloco; bloco sobe um pouco no hover; interruptores com mola; trocar de área
  entra com desfoque e escala; a pílula das abas desliza.
- O Oti fala num balão que desce da busca, letra a letra. A pergunta é entendida por palavra-chave (não é IA) e ele
  admite quando não entende. Navegar fecha o balão.
- **Jogo aberto**: o Oti avisa ("bom jogo") e só então a interface congela (classe `jogo-aberto`: sem animação, sem
  desfoque, vidro opaco).

## Regras
- Texto `#1d1d1f` no claro, `#f5f5f7` no escuro; secundário cinza. Ativo = branco sólido. Ligado = verde `#34c759`.
- Estado em ponto colorido ao lado do texto. Nunca borda colorida na lateral, texto em degradê ou número decorativo.
- **Ícones** (`src/lib/icones.ts`): desenhados para o Otimiza, traço 1,75 num quadro 24×24, pontas redondas, sem
  preenchimento. Só no trilho, no cabeçalho da área e em controle sem palavra (busca, fechar, seta). Linha de lista e
  botão com texto não levam ícone; nada de ícone em bolinha colorida.
- **Blocos da tela inicial**: cada número tem nome ("Processador 16%", nunca um anel só com ícone). Controle faz o que
  parece: interruptor liga algo de verdade, botão diz o que acontece ("Usar 180 Hz", "Medir"); o que só leva a outra
  área é link de texto com seta. Nada de player, setas ou interruptor que apenas navegam.
- **Botões**: um principal por bloco (cor do texto, letra do avesso), um pequeno de contorno, e o link de texto. Hover
  muda cor, não pula nem cresce.
- Jogo sem capa mostra as iniciais num quadro neutro (`iniciais` em `lib/capas.ts`), não degradê inventado.
- Estados (lendo, vazio, erro, sem administrador) sempre desenhados, com o que fazer.
- Para conferir o mascote: `npm run nova:web` e `/?galeria`; o ícone do app sai de `/?icone` →
  `brand/nova-geracao/oti-icone.png` → `npx tauri icon`.
