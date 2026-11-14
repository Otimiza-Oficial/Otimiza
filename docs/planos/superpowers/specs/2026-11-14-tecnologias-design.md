# Tecnologias: Frame Boost, Turbo por jogo, Rede e ping (14/11/2026)

O dono escolheu as três (o Oti com IA ficou para depois). Regra de sempre: nunca menos FPS, tudo se desfaz, medido ≠
estimado, nada muda no PC sem a pessoa pedir. Cada item abaixo usa o que o Otimiza **já mede**, para não depender de
teste em placa que não temos aqui; o que precisar de jogo aberto para provar fica marcado.

## Rede e ping (primeiro: só lê, risco zero no PC)
- **R-a. O ping por horário se enche sozinho.** A Rede já mostra "quando a conexão piora" (manhã, tarde, noite), mas só
  com as rotas medidas na mão (`rota_historico.rs`). As partidas já gravam janelas de ping automaticamente
  (`rede-partidas.json`). Juntar as duas fontes por período, para o resumo aparecer sem a pessoa medir nada, com a
  origem dita ("das suas partidas" × "das rotas medidas").
- **R-b. Bufferbloat dito de uma vez.** O teste de fila já existe (`fila.rs`); o cartão ganha um veredito em uma linha
  e o número grande (+213 ms enviando), com o "o que fazer" no roteador.

## Turbo por jogo
- **T-a. A receita de cada jogo à vista.** Os perfis por jogo já existem (`turbo.rs`, `Perfil`). A página do jogo (aba
  Jogos) ganha o cartão "Turbo neste jogo": a receita guardada, o resultado da prova (medido) e o desfazer.
- **T-b. Competitivo por padrão.** CS2, Valorant e outros competitivos já são reconhecidos (`competitivo`): a receita
  deles prefere menor atraso; deixar isso dito na tela e escolhível por jogo.

## Frame Boost
- **F-a. Auto que respeita jogo competitivo.** Em jogo competitivo, o Auto escolhe 2× (menos atraso) em vez de encher o
  monitor; nos outros, continua enchendo. A escolha aparece na tela ("Auto: 2× por ser competitivo").
- **F-b. O que a última sessão provou, por jogo.** `fb_por_jogo.rs` já guarda a escolha aprovada pela guarda; mostrar no
  Frame Boost "da última vez neste jogo: 3×, manteve 97% do FPS" com o botão de usar de novo.
- Precisa de jogo aberto para provar: F-a (o atraso medido), a ser conferido com o dono antes de publicar.

## Ordem
R-a → R-b → T-a → F-b → F-a → T-b. Cada um: teste do que é puro, `cargo test` do contrato quando mudar comando,
`svelte-check`, prints, commit. Nada vai para o instalador público sem o dono testar o de teste.

## Andamento
- [x] R-a — o "quando piora" usa também o ping das partidas (`rota_historico::das_partidas`, campo `das_partidas`)
- [x] R-b — nota da fila A+…F (`ui/src/lib/fila.ts`, com teste)
- [ ] T-a — próximo: comando `turbo_receitas` (LIVRES, só lê) listando os perfis de `turbo.rs` com o nome do catálogo
  (`jogos::CATALOGO` pelo id) e o cartão "Turbo neste jogo" na aba Jogos (casar por `chaveDoJogo`)
- [ ] F-b, F-a, T-b
