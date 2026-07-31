Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.4.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.4.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

## Esta versão responde a uma reclamação

Quem usa o Otimiza para atender cliente trouxe o problema: as otimizações
funcionam, mas o cliente **não sente** a diferença. E é verdade — ajuste de
registro rende pouco que uma pessoa perceba no dia a dia.

Então esta versão foi atrás do que a pessoa realmente percebe.

**Quanto o seu PC demora para ligar.** Ninguém nota 5% de FPS. Todo mundo nota um
PC que ligava em dois minutos e passa a ligar em quarenta segundos. O Windows
mede isso sozinho a cada inicialização, guarda o nome de cada programa que
atrasou e quantos segundos cada um custou — e nenhum otimizador do mercado mostra
esse registro. Agora o Otimiza mostra, separando o tempo até a área de trabalho
aparecer do tempo depois disso, que é quase sempre a maior fatia e é o que o dono
sente como "liga mas não dá para usar".

Ele também mostra o tipo de cada inicialização recente, o que responde uma
pergunta comum: *"reiniciei e não melhorou"*. Com a Inicialização Rápida ligada,
o Windows não desliga de verdade — ele guarda o núcleo do sistema e restaura.
Mudança que exige reiniciar só vale depois de um desligamento completo.

**Por que o processador não entrega tudo.** Um notebook empoeirado a 95 graus
corta o processador para uma fração da velocidade, e nenhum ajuste de software
resolve isso — o técnico limpa, otimiza, mede, e nada melhora, porque o problema
é físico. Só que plano de energia mal configurado causa exatamente o mesmo
sintoma e se resolve num clique.

Confundir os dois é caro dos dois lados: manda o cliente abrir um PC que não tem
problema, ou deixa passar um que tem. Então o Otimiza elimina causa por causa —
bateria, plano de energia, registro térmico do Windows, limite elétrico do
hardware — e **só fala em calor quando o próprio Windows registrou o evento**,
citando a data. Quando nenhuma causa conhecida explica, ele diz que não sabe. Um
palpite aqui faz você trocar peça à toa.

## A interface foi refeita por dentro

Não era falta de enfeite. Eram 21 valores de espaçamento escolhidos caso a caso,
11 tamanhos de fonte, 8 espaçamentos de letra e nenhum sistema ligando nada
disso.

- Os textos do programa chegavam a **319 caracteres por linha** em monitor
  grande. A faixa legível é 45 a 75. Agora são 61.
- A cor do texto explicativo reprovava no contraste mínimo da norma de
  acessibilidade — todo o texto de apoio do produto estava apagado demais para
  ser lido com conforto.
- Painéis lado a lado terminavam em degrau, com vazio de um lado.
- Foco de teclado agora existe em tudo que recebe Tab.
- Estado de carregamento com movimento: antes, um painel esperando resposta
  ficava parado e parecia travado.

**Sobre a animação:** ela é toda em CSS, sem biblioteca. Foi uma decisão medida.
As bibliotecas de animação populares custam mais de 70 KB — mais que o dobro do
aplicativo inteiro — para fazer o que o CSS já faz em zero byte. Num programa
vendido para tirar peso de PC fraco, embutir isso seria contradizer o próprio
propósito.

E a medição achou um desperdício **nosso**: as barras de carga por núcleo
animavam de um jeito que obrigava o navegador a refazer o layout inteiro a cada
2 segundos. Corrigido. A animação também se desliga sozinha em máquina de 4 GB ou
2 núcleos — se a interface engasga no PC que ela deveria estar consertando, ela
se desmente antes de aplicar a primeira otimização.

## O que o Otimiza não faz

Existe uma lista de coisas que rendem desempenho e que nós **recusamos** fazer.
Ela está dentro do programa, na aba Otimizações:

- Desativar as proteções da CPU contra Spectre e Meltdown
- Desligar Windows Update, Defender ou firewall
- "Limpeza de registro", que não tem ganho medível e quebra programa instalado
- Liberar memória à força, o que deixa o gráfico bonito e o PC mais lento
- Escrever na BIOS — em placa de consumo, errar ali inutiliza a placa-mãe

## Limites desta versão, ditos aqui

O tempo de inicialização vem de um log protegido: **é preciso abrir o Otimiza
como administrador** para lê-lo. E em parte das máquinas o Windows simplesmente
para de gravar essa medição, sem que haja como forçá-lo. Quando isso acontece o
programa diz que não há dado — não inventa um número nem trata a ausência como
boa notícia.

A detecção de calor foi verificada contra falso positivo: máquina fria, com o
processador a 100% por 45 segundos, não acusa nada. A detecção num notebook
realmente quente ainda não foi verificada em campo. É por isso que o texto na
tela cita o registro do Windows com data, em vez de afirmar por conta própria.

## Tudo é reversível

Cada mudança grava o valor anterior antes de escrever. "Desfazer tudo" restaura o
que existia, não algo parecido — inclusive programas de inicialização, tarefas
agendadas e serviços. As duas exceções são apagar arquivos e limpar o cache de
atualizações, ambas marcadas como **sem volta** e fora do "Otimizar agora".

## Ainda não há versão para macOS e Linux

O motor de otimização é todo específico do Windows. Um instalador para as outras
plataformas hoje abriria um programa sem nenhuma função — preferimos não publicar
a publicar algo que não entrega.
