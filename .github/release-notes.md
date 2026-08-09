Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.12.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.12.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

## Cache de shader: 8,8 GB na máquina de teste

Todo jogo compila pedaços do próprio código gráfico na primeira vez que precisa
deles e guarda o resultado em disco — é por isso que a primeira partida engasga
e a segunda não.

Só que esse cache **não é limpo quando o driver de vídeo é atualizado**. Entrada
compilada por um driver que já nem existe mais continua lá, e o driver novo às
vezes recompila no meio da partida. É a explicação mais comum para *"atualizei o
driver e começou a travar"* — queixa quase sempre atribuída ao driver novo,
quando o culpado é o cache velho.

Na máquina onde isto foi desenvolvido havia **8,8 GB**, com arquivos de fevereiro
sobre um driver instalado em julho. O Otimiza compara a data da entrada mais
antiga com a data do driver e marca o cache como obsoleto quando ela é anterior
— que é exatamente o caso que causa o problema.

Apagar é seguro: o conteúdo é recalculável por definição. A contrapartida está
escrita no aviso — a primeira partida depois da limpeza compila de novo e pode
engasgar, e da segunda em diante fica melhor que estava.

## Antes de otimizar

Um painel novo no Diagnóstico, com verificações que **não são otimizações**: são
condições que, quando erradas, fazem o atendimento inteiro parecer sem efeito.

A que motivou o painel é reinício pendente. Parte das mudanças de sistema só
passa a valer depois de reiniciar; o técnico aplica tudo, mede, não vê ganho e
conclui que o produto não funciona. O motivo estava lá desde o começo, numa
chave de registro que ninguém olha. A máquina de desenvolvimento tinha um.

Também verifica **TRIM desligado num SSD** — que degrada a velocidade de escrita
aos poucos, de um jeito que ninguém associa à causa —, **arquivo de paginação
num disco mecânico** numa máquina que tem SSD, e a ausência do **plano de
desempenho máximo**, que o Windows tem escondido e não cria sozinho.

## Prioridade que não some quando o jogo fecha

O "Priorizar o jogo" valia só para a sessão, e isso estava documentado como
limitação. Agora dá para fixar de vez: o Windows passa a criar o processo já em
prioridade alta, em toda abertura.

A chave do registro usada para isso é a mesma que malware usa há décadas para
sequestrar execução — quem escreve um valor chamado `Debugger` ali faz o Windows
abrir outro programa no lugar do pedido. Por isso o Otimiza só escreve o valor de
prioridade, só na subchave certa, e só para executável que ele reconhece como
jogo. Nome vindo de fora é recusado.

Ressalva dita na tela: o ajuste é por nome de arquivo, e o processo do FiveM
carrega o número da compilação. Quando o FiveM atualiza, o nome muda e é preciso
aplicar de novo.

## O visual mudou

Tudo era canto vivo. Canto vivo passa seriedade e era uma escolha deliberada,
mas em quantidade vira dureza — uma tela com quarenta retângulos de canto reto
parece um formulário, não um produto. Agora há arredondamento em três degraus,
do controle pequeno ao painel, e as barras de carga são cápsulas.

As cantoneiras que marcavam dois cantos de cada painel saíram junto. Elas eram
duas marcas de 9px em cantos diagonais, assimétricas de propósito, e não
convivem com borda curva: viravam dois riscos soltos fora da curva.

A marca subiu para o topo da barra lateral, e no lugar dela apareceu uma
**busca no topo** — clicar nela abre a mesma paleta do Ctrl K, então existe um
só caminho para achar qualquer ação.

Cada seção ganhou um **cabeçalho com trilha e ícone**, no lugar de começar
direto nos painéis. O nome e o ícone vêm da própria navegação lateral, e não de
uma segunda lista: assim eles nunca divergem.

E os painéis respiram mais. A referência que orientou este passe é bem mais
espaçada, e densidade alta serve para tabela de números — para texto
explicativo, que é onde vive o argumento deste produto, ela cansa.

## A navegação mudou de lugar

As sete seções viviam numa faixa de abas no topo, e cada painel novo apertava
mais. Agora existe uma **barra lateral**, agrupada por assunto — Monitorar e
Agir — que cresce para baixo sem disputar espaço e tem lugar para a contagem de
cada área.

Ela recolhe para uma faixa de ícones quando você quer mais espaço, e a escolha
fica guardada. Em tela de 1366 ou menor ela recolhe sozinha: perder a navegação
inteira num notebook pequeno seria pior que apertar.

Os ícones são desenhados em CSS, sem arquivo externo. Não há biblioteca de
ícones nem requisição de rede — num programa que precisa abrir sem internet,
isso conta.

## Buscar ação, com Ctrl K

O Otimiza passou de quarenta botões espalhados por sete seções. Quem usa isto
todo dia sabe o nome do que quer e não deveria precisar lembrar em qual aba
mora.

Agora **Ctrl K** abre uma busca de ações: digite "relatório", "cache",
"memória", e vá direto. Funciona sem acento — quem digita "memoria" acha
"memória". Setas escolhem, Enter vai, Esc fecha.

A lista é montada a partir da própria interface, e não escrita à mão: cada
botão novo entra na busca sozinho, e nenhum fica de fora porque alguém esqueceu
de cadastrar.

## Por que o seu FPS está baixo

A pergunta que todo cliente faz e que nenhum otimizador responde. A resposta
honesta quase nunca é "falta otimizar" — é uma peça específica que chegou no
limite enquanto as outras estão sobrando.

O painel mede processador, placa de vídeo, memória de vídeo, memória do sistema
e disco durante dez segundos e diz qual travou. **Ele não otimiza nada: ele
explica.** E explicar é o que permite a próxima decisão ser certa, seja mexer
numa configuração, trocar uma peça, ou não fazer nada.

**O caso mais comum em FiveM é o mais mal interpretado.** O jogo depende muito de
um único núcleo. Numa máquina de oito núcleos é normal ver um a 100% e os outros
sete quase parados, com a placa de vídeo a 40%. O dono abre o Gerenciador de
Tarefas, lê "CPU 25%", conclui que sobra máquina e não entende por que trava.

Sobra máquina — na parte errada. Comprar um processador com mais núcleos não
resolve nada; um com núcleo mais rápido resolve. Placa de vídeo melhor também
não, porque ela já está esperando. Essa é uma informação cara de descobrir
errado, e agora ela aparece com o número na mão.

O painel também sabe quando **não** deve responder. Com o PC parado não existe
limite a encontrar, e ele diz isso em vez de apontar um culpado. Quando nada
chega perto do limite, o veredito é "não identificamos" — nunca o palpite mais
vendável.

## Modo jogo, e por que ele desliga sozinho

Liga o plano de alto desempenho quando um jogo abre e **desliga quando ele
fecha**. Essa segunda metade é o ponto inteiro.

Os "modos turbo" do mercado ligam e deixam ligado. Em notebook isso significa
gastar bateria e esquentar o dia todo por causa de duas horas de jogo à noite —
e, ironicamente, calor sustentado é uma das causas de perda de desempenho que
este mesmo programa detecta na aba Diagnóstico. Aplicar só na hora que importa
entrega o mesmo ganho sem cobrar o resto do dia.

O modo automático vem **desligado**, e fica em Preferências. Um programa que
muda a configuração do seu PC sozinho, sem você pedir, é justamente o que
criticamos nos outros — mesmo quando a mudança é boa. Quem quiser, liga sabendo
o que vai acontecer.

Tudo o que ele faz entra no histórico. Se o Otimiza fechar no meio, travar ou o
PC desligar na tomada com o modo ligado, a mudança continua registrada e o
"Desfazer tudo" devolve o plano de energia anterior.

**Ele não fecha nenhum programa.** "Encerrar processos desnecessários" aparece em
toda lista de ideias de otimizador e é a forma mais rápida de alguém perder
trabalho não salvo. O que pesa em segundo plano já está listado nas abas Painel
e Sistema, com nome, para você decidir.

## Quadros por segundo, medidos de fora

Faltava a única medida que quem joga realmente olha. Agora ela existe, e a
forma como foi feita importa: o Windows publica um aviso a cada quadro que um
programa manda para a tela, e o Otimiza escuta esse canal **de fora**. Nada é
injetado no processo do jogo — a mesma decisão do overlay, e pelo mesmo motivo.

Precisa de administrador, porque o canal só abre com essa permissão. Cobre jogos
em Direct3D 10 ou mais novo, o que inclui o GTA V e portanto o FiveM. Se nenhum
quadro for contado, o programa diz que não conseguiu medir — **nunca mostra
zero**, porque zero seria um número inventado disfarçado de medição.

## Rede e DNS, com a promessa falsa desmentida na tela

Esta é a área com mais propaganda enganosa do mercado, então a primeira coisa
que o painel diz é o que **não** dá para fazer: nenhum ajuste no seu PC reduz o
ping. Ping é distância física mais roteamento. Quem promete encurtar isso com um
botão está vendendo o que não existe, e agora isso está escrito dentro do
programa.

O que existe de verdade é trocar o servidor de DNS, que acelera achar o endereço
de um site — carregar página, abrir lista de servidores, começar um download.
Não muda o ping em jogo, porque depois de conectado a conversa é direta com o
servidor.

E em vez de afirmar que um DNS é mais rápido, o Otimiza **mede**: faz consultas
reais a cada servidor, cronometra e mostra os tempos lado a lado, incluindo o
que a sua máquina já usa. O botão de trocar só aparece quando a diferença passa
de 5 ms. Na máquina onde isto foi desenvolvido a diferença foi de 6 ms, e o
programa chamou isso de "real, mas pequena" — que é o que era.

A troca entra no histórico e o "Desfazer tudo" devolve o DNS anterior. E só
aceita resolvedores públicos conhecidos: apontar o DNS de uma máquina para um
endereço arbitrário é o mecanismo clássico de sequestro de navegação, e o
comando recusa mesmo que peçam.

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
