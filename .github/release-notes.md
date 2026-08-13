Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.13.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.13.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# Esta versão nasceu de uma falha nossa

O Otimiza foi testado em duas máquinas reais e falhou nas duas. Num notebook que
travava, e num PC onde o FiveM congelava o computador inteiro — o jogo, o
Discord, o sistema, tudo ao mesmo tempo.

A causa não era falta de otimização agressiva. Era pior: **o produto já tinha
medido o problema e disse que estava tudo bem.**

Numa máquina de 8 GB que travava por falta de memória, a tela do Otimiza
mostrava *"Memória e paginação sem problemas"*. O motivo cabe numa linha de
código: a regra que avisa sobre pouca memória exigia menos de 6 GB para
disparar. E o limiar certo já estava no mesmo arquivo, a noventa linhas de
distância, usado por outra regra.

Esta versão conserta isso e o que estava por trás disso.

## O veredito

O Otimiza abria com números de uso e dezessete botões de análise espalhados por
cinco abas. Para saber o que havia de errado com o PC, o cliente tinha que
clicar em todos e montar a resposta na cabeça. Ninguém monta.

Agora, ao abrir, o programa analisa sozinho e responde com **uma frase**, com o
número que a sustenta:

> **O Windows já registrou falta de memória nesta máquina**
> 3 registros nos últimos 30 dias — o mais recente em 16/07 às 21:21, com 31,7 GB
> comprometidos para 7,9 GB de memória física.

Não é uma nota de 0 a 100. Uma nota é fácil de fotografar e impossível de
conferir; a frase acima o cliente confere sozinho no Visualizador de Eventos do
Windows.

Quando não há nada de errado, o veredito diz isso — com os números que
sustentam a afirmação. Inventar um problema para justificar a compra é o oposto
deste produto.

## O que o Windows já sabia e ninguém lia

Quando a memória acaba, o Windows grava um evento com o nome e o tamanho dos
processos que estavam segurando memória naquele instante. Quando um programa
para de responder, grava outro com o nome e a hora.

O Otimiza passa a ler os dois. Na máquina de teste isso produziu:

```
16/07 21:21 — falta de memória
  claude.exe 9,8 GB · HuntinBuddies 4,4 GB · Arc.exe 2,2 GB
11/08 14:13 — FiveM parou de responder
16/07 21:26 — Discord parou de responder
```

É a diferença entre dizer "você precisa de mais memória" e mostrar a data, a
hora e o nome.

## Achados que agora se encontram

"Memória em canal único" era medido pelo diagnóstico de firmware. "Programas
pedindo mais memória do que existe" era medido pelo diagnóstico de memória. As
duas coisas são a mesma causa, e viviam em abas diferentes — o cliente nunca via
as duas juntas.

Agora aparecem sob a mesma frase, agrupadas pela causa.

## Discord e navegador pausados durante o jogo — e devolvidos depois

Todo otimizador do mercado responde à falta de memória **matando** processos.
O Otimiza não faz isso: fechar o Discord no meio de uma conversa, ou o navegador
com quinze abas de trabalho, é a forma mais rápida de alguém perder o que não
salvou.

Em vez disso, com o modo jogo ligado, o Otimiza **suspende** o que está em
segundo plano. O programa para de consumir processador e devolve memória ao
jogo; quando o jogo fecha, ele volta exatamente de onde parou.

Há uma lista explícita do que nunca é suspenso: áudio, antivírus, núcleo do
Windows, o próprio jogo e o próprio Otimiza. E se o Otimiza for fechado à força
ou o PC perder energia com programas pausados, **a próxima abertura os devolve**
— os identificadores vão para disco antes de qualquer coisa ser pausada.

## A lista de otimizações parou de deixar você contar errado

São 35 otimizações no catálogo. Sete mudam o FPS de forma mensurável; dezessete
são higiene de Windows que devolve algumas centenas de megabytes e **não muda
FPS**. Todas apareciam lado a lado, com o mesmo peso visual — e quem aplica
trinta espera trinta vezes o resultado.

Agora a lista diz isso antes de você clicar, e o que muda o jogo aparece
primeiro. Uma das etiquetas dizia "resposta do sistema"; passou a dizer "não
muda FPS", que é o que o próprio código sempre significou.

## A nota de saúde foi removida

Havia um número de 0 a 100 na aba Diagnóstico. Ele não consultava nenhum dos
módulos que realmente medem a máquina — nem a memória, nem o disco, nem o
firmware — e ainda assim era a coisa mais visível da tela.

Foi removido. No lugar ficou a lista de achados, cada um com o que foi medido
para sustentá-lo.

## O relatório em PDF começa pela conclusão

Antes, o laudo abria com a identificação do hardware e o cliente percorria onze
seções até descobrir o que havia de errado. A conclusão passou a ser a seção 1 —
com o que não pôde ser verificado dito ali mesmo, e não numa nota de rodapé.

## Observação contínua

Enquanto fica aberto, o Otimiza amostra a pressão de memória e guarda um resumo
por hora, por catorze dias. Isso permite dizer "a memória viveu no limite por
seis horas nesta semana, e na pior delas o FiveM segurava 7,2 GB" — o estado
anterior ao travamento, que não gera evento nenhum e sumia do diagnóstico.

O arquivo inteiro ocupa poucos kilobytes: um otimizador que engorda o disco do
cliente seria uma piada de mau gosto.

## VBS: agora sabemos diferenciar dois casos

Desligar a virtualização de segurança rende FPS, e custa proteção real. Mas
existe um caso em que ela está **ligada sem nenhum serviço de segurança usando**
— pagando o preço em desempenho sem entregar nada em troca.

O Otimiza passa a diferenciar os dois, e diz qual é o seu. Continua fora do
botão "Otimizar agora": abrir mão de proteção é decisão consciente, não efeito
colateral de um clique genérico.

---

## O que esta versão não promete

A máquina de teste tem 8 GB num único pente, com quatro encaixes livres na placa.
Nada aqui faz o FiveM caber confortavelmente em 8 GB. Pausar o segundo plano
devolve talvez 1,5 a 2 GB — ajuda, e não resolve.

O segundo pente resolve. O produto passa a dizer isso na primeira tela, que é o
que deveria ter feito desde o começo.
