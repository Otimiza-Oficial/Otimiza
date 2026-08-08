Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.6.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.6.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

## FiveM

Esta versão tem uma aba nova, e ela existe porque o público real deste programa
joga FiveM.

O FiveM guarda em disco tudo que baixa de cada servidor em que você entra:
mapas, scripts, sons, texturas. Isso cresce sem limite e é a causa mais comum de
três problemas ao mesmo tempo — disco cheio sem explicação, travada no meio da
partida e crash ao entrar num servidor. Nada disso precisa ficar guardado,
porque o servidor reenvia na próxima conexão.

Na instalação usada para desenvolver isto, essa pasta tinha **10 GB em 21.584
arquivos**.

**A armadilha, e é a mesma do navegador.** A segunda maior pasta da instalação
tinha 3,2 GB e é justamente a que não se pode apagar: ali ficam o seu perfil do
jogo e os dados de sessão da Rockstar Social Club. Apagar deslogaria você da
conta e jogaria fora os seus ajustes. Ela aparece na lista, medida, marcada como
protegida e com o motivo escrito. Tamanho não decide o que é lixo — cada pasta
foi aberta e classificada uma a uma.

Também dá para colocar o jogo em prioridade alta no processador enquanto ele
está rodando. Duas coisas ditas na própria tela: a prioridade some quando o jogo
fecha, então precisa ser aplicada a cada sessão; e é **alta**, nunca "tempo
real" — tempo real coloca o jogo acima do próprio sistema operacional, incluindo
o que cuida de som, mouse e teclado, e o resultado prático costuma ser travar a
máquina inteira.

**Duas coisas que decidimos não fazer, e o motivo.**

Não colocamos overlay de FPS dentro do jogo. Overlay exige injetar código no
processo, e o anticheat do FiveM trata injeção como ameaça. O ganho seria um
número bonito na tela; o risco seria a conta do cliente.

Não "liberamos memória" antes de jogar. Essa função esvazia o conjunto de
trabalho dos processos: o gráfico melhora e o desempenho real piora, porque tudo
precisa ser relido do disco. Já estava na nossa lista de recusas, e não passou a
valer só porque o assunto agora é jogo.

## O navegador, que ninguém olha

Em PC fraco o navegador costuma consumir mais memória que todo o resto junto, e
nenhum otimizador do mercado abre essa porta. Esta versão abre.

Você vê quanto o navegador está usando **agora**, em MB e em porcentagem da sua
memória — o número que a pessoa sente ao fechar. Vê as extensões instaladas com
**o nome real de cada uma**, quantas permissões cada uma pede, quanto ocupam em
disco, e quantas versões antigas ficaram para trás ocupando espaço à toa. E vê
quanto de cache dá para recuperar, com um botão para limpar.

Três decisões que valem ser explicadas.

**Não mostramos memória por extensão.** Era a ideia original, e não se sustenta:
de fora do navegador não existe como saber quanto cada extensão gasta — várias
dividem o mesmo processo e ele não diz quais. O Gerenciador de Tarefas do próprio
Chrome consegue porque roda por dentro. Qualquer número por extensão aqui seria
inventado, então não tem.

**Cache e dado de aplicativo são coisas diferentes, e a diferença é cara.** Numa
máquina de teste, a maior pasta do perfil tinha 1,7 GB e o nome `IndexedDB` —
alvo óbvio de quem varre por tamanho. Só que ali mora conversa de WhatsApp Web,
e-mail guardado para uso sem internet, arquivo de editor online. Apagar desloga a
pessoa de tudo e o que estiver lá some. O Otimiza mede, mostra e **não oferece
limpar**.

**Limpar cache não é ganho puro.** Os sites que você usa carregam mais devagar na
primeira visita depois da limpeza, porque baixam tudo de novo. Está escrito no
aviso antes de você confirmar.

Sobre privacidade: o programa lê manifesto de extensão, tradução de extensão e
**tamanho** de pastas. Não abre histórico, senhas, cookies, favoritos, sessões
nem nada de navegação — nem para contar. Ao medir o dado de aplicativo soma
apenas o total, porque os nomes das subpastas revelariam quais sites você usa.

## A versão anterior respondeu a uma reclamação

Quem usa o Otimiza para atender cliente trouxe o problema: as otimizações
funcionam, mas o cliente **não sente** a diferença. E é verdade — ajuste de
registro rende pouco que uma pessoa perceba no dia a dia.

A versão 0.4.0 foi atrás do que a pessoa realmente percebe, e continua valendo.

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
agendadas e serviços. As exceções são apagar arquivos, limpar o cache de
atualizações e limpar o cache do navegador: as três são marcadas como **sem
volta**, ficam fora do "Otimizar agora" e pedem confirmação antes.

## Ainda não há versão para macOS e Linux

O motor de otimização é todo específico do Windows. Um instalador para as outras
plataformas hoje abriria um programa sem nenhuma função — preferimos não publicar
a publicar algo que não entrega.
