# Direção visual da interface nova

Lido antes de qualquer tela nova. Código é a fonte da verdade: `src/tokens.css`, `src/App.svelte`,
`src/componentes/Oti.svelte` e `src/lib/conversa.ts`.

## Do que se trata
O Otimiza diz como o PC de um jogador brasileiro está, conserta o que dá e **prova** com número medido. A interface é
o **Oti no centro, conversando** (escolha do dono, 07/11/2026): ele diz o que leu do PC e oferece respostas; quase não
há menus.

- **Cena escura** (`#0b0c1f`) com um único enfeite: o brilho que respira atrás do Oti. Tema claro opcional.
- **A fala** aparece letra a letra, com a boca dele mexendo. Embaixo, o detalhe e as **respostas** em pílulas (uma
  principal em roxo).
- **Pergunte ao Oti**: a barra de baixo entende por palavra-chave (`entender` em `conversa.ts`). Não é IA, e quando
  não entende ele diz isso e mostra os assuntos que conhece.
- **Áreas**: ícones de traço próprios (`lib/areas.ts`), sempre embaixo. Uma área abre num painel à direita; o Oti vai
  para a esquerda e narra o que ela faz.
- **Barra de cima**: só a marca, processador, memória e Hz, e o jogo aberto.

## O Oti
Bichinho de vidro roxo-azulado (`#b3ccff` → `#7d8dff` → `#8b5ff0`), olhos grandes azul-noite com **reflexo âmbar**
(a cor da marca) e brilhos brancos, antena com lâmpada âmbar, bochechas rosadas, duas mãozinhas que flutuam.

- Humores: atento (segue o cursor), falando, feliz (olhos em arco), pensando (sobrancelha, lâmpada pulsando),
  preocupado (sobrancelhas, gota de suor), surpreso, jogando (headset e controle), dormindo.
- Gestos: acenar, apontar, comemorar.
- Cenas: clique = pulo e uma dica; **jogo abriu** = bota o headset e segura o controle, diz "bom jogo", vai para a
  barra de cima e só então a interface congela; **jogo fechou** = volta comemorando e pergunta como foi.
- O ícone do app sai dele: `brand/nova-geracao/oti-icone.png` → `npx tauri icon`.
- Para conferir o desenho: `npm run nova:web` e abrir `/?galeria` (todos os humores) ou `/?icone`.

## Regras
- Uma cor de ação: o roxo do Oti (`--acao`). Verde, âmbar e vermelho só como estado, num ponto ao lado do texto.
- Dentro dos painéis: seção com linha fina, tabela, ponto de estado. Cartão dentro de cartão, nunca.
- **Nunca**: borda colorida na lateral, pílula com ícone, número decorativo, texto em degradê, botão com brilho.
- **Estados são parte da tela**: lendo, vazio, erro e sem administrador sempre desenhados, com o que fazer.
- **Jogo aberto**: nada anima, nada desfoca (classe `jogo-aberto`), depois da cena do headset.
