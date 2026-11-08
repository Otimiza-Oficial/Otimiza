# Direção visual da interface nova

Lido antes de qualquer tela nova. Código é a fonte da verdade: os valores moram em `src/tokens.css`.

## Do que se trata
O Otimiza diz como o PC de um jogador brasileiro está, conserta o que dá e **prova** com número medido. A marca é
uma coisa só: o **mascote de bolha de vidro** sobre um **céu**. Todo o resto é quieto, para os números falarem.

Referências: `brand/nova-geracao/mascote-logo.png` (o mascote de verdade) e `brand/nova-geracao/referencia-interface.png`
(a área de trabalho no céu: janela de verdade, texto pequeno, quase nenhum enfeite).

## Regras
- **Três tamanhos de texto**: 12 (apoio), 14 (corpo), 20 (título). Números grandes: 28. Peso 400/500; 600 só em título.
- **Espaço**: 4, 8, 12, 16, 24, 32, 48. Mais espaço entre grupos do que dentro deles.
- **Uma cor de ação** (`--acao`). Verde, âmbar e vermelho só como estado, num ponto de 8 px ao lado do texto.
- **Superfície em vez de cartão**: uma janela com seções separadas por linha fina. Cartão dentro de cartão, nunca.
- **Sombras**: três degraus (`--sombra-1..3`). Botão não tem brilho nem degradê.
- **O vidro é do mascote.** As superfícies são claras e quase opacas; desfoque só no fundo das janelas.
- **Nunca**: borda colorida na lateral, pílula com ícone, número decorativo, texto em degradê, brilho em botão,
  borrão roxo, maiúsculas espaçadas como enfeite, "Voltar" em pílula onde cabe um X de janela.
- **Estados são parte da tela**: lendo, vazio, erro e sem administrador sempre desenhados, com o que fazer.
- **Jogo aberto**: nada anima, nada desfoca (classe `jogo-aberto`).

## O mascote
Bolha **transparente**: quase sem preenchimento, borda grossa de vidro lavanda, reflexo pêssego em cima à esquerda
e lavanda embaixo, brilho especular, "orelhas" de vidro em forma de gota nos lados, um pouco abaixo do meio. Olhos
são cápsulas em pé, âmbar em cima indo a marrom-escuro, com um traço de luz e um reflexo azulado embaixo. Os olhos
são a expressão inteira.
