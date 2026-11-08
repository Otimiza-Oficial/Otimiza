# Direção visual da interface nova

Lido antes de qualquer tela nova. Código é a fonte da verdade: os valores moram em `src/tokens.css` e `src/App.svelte`.

## Do que se trata
O Otimiza diz como o PC de um jogador brasileiro está, conserta o que dá e **prova** com número medido. A interface
é uma **área de trabalho no céu**, como `brand/nova-geracao/referencia-interface.png` (escolhida pelo dono):

- **Mesa** azul viva, clareando para o horizonte. Nuvem só aparece quando há aviso (o céu é o diagnóstico).
- **Bloco de perfil** no alto: o mascote no lugar da foto, um rótulo pequeno em maiúsculas, uma frase e um link.
- **Objetos 3D** soltos na mesa, um por área, com legenda pequena. Arrastáveis; soltos no mascote, ele os engole e
  abre a área. Vêm do Fluent Emoji da Microsoft (licença MIT, `public/objetos/LICENCA-fluentui-emoji.txt`).
- **Widgets de vidro** compactos à direita, com rótulo em maiúsculas; dá para puxar um para a mesa e devolver.
- **Dock** com os mesmos objetos em ícones claros, com lupa sob o ponteiro e ponto na área aberta.
- **Janela de verdade** para cada área, que cresce a partir do objeto que a abriu. Dentro dela, o texto escuro e as
  peças comuns do `tokens.css` (seção com linha fina, botão, interruptor, ponto de estado).

## O mascote
Bolha de vidro **transparente e clara**: aro grosso de vidro, reflexo pêssego em cima e lavanda embaixo, brilho
especular, orelhas em gota nos lados. Olhos em cápsula, âmbar indo a marrom, reflexo azul só na ponta. Os olhos são a
expressão inteira: atento (segue o cursor), focado, descansando, preocupado (brilho âmbar), surpreso (objeto chegando),
feliz (olhos em arco e bochechas).

Cenas: clique = pulo amassado e uma dica; objeto solto nele = engole; **jogo abriu** = pula, gira e voa para a barra de
menu (e só então a interface congela); **jogo fechou** = volta da barra comemorando.

## Regras
- Texto da mesa é branco com sombra leve; dentro das janelas, `--tinta`.
- Rótulo em maiúsculas espaçadas só nos rótulos de widget e do perfil (como na referência), nunca como enfeite solto.
- Uma cor de ação (`--acao`). Verde, âmbar e vermelho só como estado, num ponto de 8 px ao lado do texto.
- **Nunca**: borda colorida na lateral, pílula com ícone, número decorativo, texto em degradê, botão com brilho.
- **Estados são parte da tela**: lendo, vazio, erro e sem administrador sempre desenhados, com o que fazer.
- **Jogo aberto**: nada anima, nada desfoca (classe `jogo-aberto`), depois da cena de saída do mascote.
