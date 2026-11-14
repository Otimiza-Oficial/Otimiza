# Interface premium do Otimiza (13/11/2026)

## Por quê
O motion de lançamento vai ser gravado em cima do app. O dono quer a interface **muito acima da do Beyond** em todas as
abas, para o vídeo ficar bom pelo produto e não só pela edição. Sai como **changelog de correções** (sem versão nova),
junto com os bugs achados no caminho e alguns sistemas novos. As épocas do ano (Natal etc.) hoje são um gorro, partículas
e uma saudação; o dono achou "feito de qualquer jeito" e quer que elas se integrem ao app inteiro.

O dono delegou as decisões de design ("veja do bom e do melhor"). Tudo abaixo marcado **(decisão)** foi escolha do
construtor e pode ser revisto.

## Metas
- Todas as abas (Jogar, Meu PC, Prova, Ajustes e as sub-abas) com hierarquia clara: o número ou a ação principal em
  destaque, o resto em cartões limpos, os detalhes recolhidos. Nada de parede de texto.
- Fundo limpo por padrão; vidro escuro nítido; tipografia e espaçamento consistentes.
- Épocas que mudam o app inteiro com bom gosto (cor, luz, cartões, Oti, textos), sem atrapalhar o uso e paradas com o
  jogo aberto.
- Consertar o que for achado em cada aba, com teste quando houver lógica.

## Não-metas
- Trocar de tecnologia (continua Svelte 5 + Tauri 2 + os tokens de `ui/src/tokens.css`).
- Mudar o comportamento de otimizações, licença ou contrato com o Rust só por estética.
- Versão nova no instalador antes de o dono ver (a publicação é decisão dele).

## Anti-metas (falha mesmo se "funcionar")
- Enfeite que finge função (as regras do `ui/DESIGN.md` continuam valendo: cada número tem nome, controle faz o que
  parece).
- Época que cobre conteúdo, pesa o PC ou anima com o jogo aberto.
- Menos FPS: nada de desfoque nem animação contínua com `jogo-aberto`.

## Direção visual (decisão)
- **Fundo**: três opções em Ajustes e no botão da barra — **Limpo** (padrão novo: grafite com uma luz suave), **Cômodo**
  (o de hoje) e **Área de trabalho** (o "Sem fundo" de hoje).
- **Cor de marca**: o vermelho do vídeo (`#ff3b3b` no escuro, `#e5262c` no claro), só em momentos de marca: indicador da
  aba ativa, segmento escolhido, número-herói, foco. Botão principal continua branco/preto (como no vídeo). Os estados
  (ok, atenção, perigo, âmbar do "Medido") ficam como estão; o perigo vira um rosa-salmão para não brigar com a marca.
- **Escuro como tema padrão** para quem nunca escolheu (o claro continua e ganha a mesma passada).
- **Tipografia**: quatro tamanhos (apoio, corpo, título, número) + um **display** para o número-herói de cada aba.
- **Cartões**: grafite quase opaco, filete de luz em cima, sombra mais longa, raio 20; a luz que segue o cursor fica.
- **Movimento**: entrada em cascata dos cartões ao abrir a área, número que conta ao aparecer, interruptor com mola;
  tudo desliga com `jogo-aberto` e com "reduzir movimento".

## Épocas (decisão)
Cada época vira um **tema completo**: tokens próprios (cor de marca e luz do fundo), uma camada de cenário discreta
(ex.: Natal com cordão de luzes desfocado no topo e geada no filete dos cartões; Halloween com névoa roxa e laranja;
Junina com bandeirinhas no topo), o Oti com acessório **e** comportamento (ex.: no Natal ele "sopra" neve quando você
clica nele), textos do clima nos estados vazios e na saudação. Partícula só no fundo, pouca, e nenhuma com jogo aberto.

## Ondas
| Onda | Entrega | Pronto quando |
|---|---|---|
| 1 | Fundação: tokens, fundo Limpo, casca, barra, trilho, abas, botões, cartões | Visão geral aberta nos dois temas sem nada quebrado |
| 2 | Visão geral refeita (sem buracos: "Agora" com gráfico vivo, "Avisos" com estado vazio bom) | Print aprovado |
| 3 | Jogar: Jogos, Turbo + Frame Boost em componentes, Mira, Cores | Cada aba com herói + cartões + detalhes recolhidos |
| 4 | Meu PC: Tela, Energia, Rede, Latência, Sistema | idem |
| 5 | Prova, Histórico, Ajustes, balão do Oti, avisos, boas-vindas | idem |
| 6 | As 7 épocas como temas completos | Prévia de cada uma por `?temporada=` |
| 7 | Passada no tema claro, `jogo-aberto`, prints finais, texto do changelog | Testes da UI e svelte-check passando |

Cada onda: `svelte-check`, `npm run testes:ui`, prints dos dois temas com `scripts/prints-nova.mjs`, commit.

## O que fica para o construtor
Disposição exata de cada aba, textos curtos, quais detalhes recolher. Regra de desempate: o que deixa o número ou a ação
principal mais claro em menos tempo de olhar.

## Andamento (atualizado a cada commit)
- [x] Onda 1 — fundo Limpo padrão, escuro padrão, cor da marca, cabeçalho das áreas, seções em cartões
- [x] Visão geral — capas de reserva com logo, curva do processador, Oti nos avisos
- [x] Frame Boost em componentes · Jogos como página de jogo · Tela com medidor de Hz · Histórico em linha do tempo
- [x] Épocas — varal por época, partículas atrás do vidro, cor da marca e filete por época, geada no Natal
- [ ] Rede (herói do sinal), Turbo (modos), Mira, Cores, Latência, Energia, Prova (gráfico com a marca)
- [ ] Balão do Oti, apresentação, avisos
- [ ] Passada no tema claro, prints finais, texto do changelog
