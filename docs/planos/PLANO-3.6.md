# 3.6 — Turbo Máximo: o jogo que a pessoa não reconhece

## Por quê

No beta da 3.5, o sócio ganhou +30 FPS no FiveM num PC que já vinha otimizado (ISO modificada). Bom, e é o mínimo:
a meta da 3.6 é a pessoa entrar no jogo e sentir outro jogo. Isso tem três números, e o Otimiza mostra os três
separados, porque é essa honestidade que faz o cliente acreditar no número:

| O que a pessoa sente | De onde vem | Alavanca |
|---|---|---|
| FPS do jogo (real) | menos pixels, configuração gráfica, processador sem disputa | escala, perfil do jogo, governador, núcleos rápidos |
| FPS na tela | geração de quadros | Frame Boost até encher o monitor (2× a 3× o real) |
| Leveza (ritmo) | tempo de quadro estável: 1% pior alto, sem engasgo | base travada no que a máquina sustenta |

Hoje cada peça existe, mas solta: o Turbo liga escala, energia e geração. O governador de segundo plano, o Auto CPU Set
(núcleos rápidos, só se medido), o Jogo liso (base travada) e o perfil gráfico do GTA moram em outras telas.

## Fatias (cada uma medida, desfazível e com teste)

| # | Fatia | Estado |
|---|---|---|
| F1 | **A prova mostra a leveza.** O cartão e o texto do "Copiar resultado" trazem o 1% pior sem → com (a prova já mede; o desfecho já usa). | feita |
| F2 | **Processador sem disputa.** Com gargalo no processador, o Turbo liga o governador (fundo sem prioridade) e o Auto CPU Set medido; desligar devolve. Entram no lado "com" da prova. | |
| F3 | **Base estável.** Quando a medida mostra ritmo irregular (1% pior muito abaixo da média), o Turbo trava a base no que a máquina sustenta, com ou sem geração. | |
| F4 | **Até o monitor.** "Mais FPS" monta a combinação inteira: base estável + escala se a placa for o limite + geração Auto até os Hz. A meta de "+100 na tela" vem daqui, dito como "na tela". | |
| F5 | **FiveM.** O perfil gráfico do GTA entra na receita como passo "quando o jogo fechar" (o arquivo só pode ser escrito com ele fechado). | |
| F6 | **Prova mais curta.** Parar antes quando a diferença já está clara, sem perder a margem. Com os números do beta. | depois do beta |

## Regras que não mudam

Nunca menos FPS (a prova desliga o que piorar), tudo desfaz, número medido separado de estimado, e "na tela" nunca
vendido como FPS do jogo.

## O que falta do beta

Do sócio: o "Copiar resultado", o print do cartão do Turbo, o `otimiza.log`, processador, placa e Hz do monitor.
Diz qual era o gargalo no FiveM dele e qual fatia rende mais primeiro.
