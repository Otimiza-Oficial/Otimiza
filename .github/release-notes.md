Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_..._x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_..._x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# 1.3.0 — o congelamento deixa de ser invisivel

Versao de conserto. Nasceu de um caso concreto: um cliente abriu o
gerenciador de tarefas, viu **"Steam — Suspenso"**, depois o Discord, depois
o Chrome, e concluiu que o Otimiza tinha quebrado a maquina dele.

Ele nao estava errado em concluir isso. O produto congelava programas de
propósito, e **nao mostrava nada disso na tela nem oferecia como desfazer**.

## Agora voce ve o que esta congelado, e desfaz com um clique

Enquanto houver programa congelado, a tela mostra quais sao e um botao
**Descongelar agora**. Ele devolve tudo na hora, sem fechar o jogo.

O bloco so aparece quando ha algo congelado, e some sozinho quando nao ha.

E ele diz a frase que faltava: **no Gerenciador de Tarefas isso aparece como
"Suspenso", e e proposital.** Era essa ponte que nao existia entre o que voce
ve no Windows e o que a gente explica aqui.

**Descongelar nao precisa de licenca.** Desfazer o que o produto fez nunca vai
depender de licenca — uma que vence nao pode deixar voce com o Discord parado.

## Quem ja tinha o modo jogo ligado vai ser avisado

O texto dessa opcao foi corrigido na 1.1.2 para dizer que ela congela
programas. Quem ligou ANTES disso nunca leu o texto novo — e e exatamente
esse cliente que se assustou.

Na primeira vez que voce abrir a 1.3 com a opcao ligada, ela explica uma vez
o que faz, com **Manter** e **Desligar**. Desligar tambem devolve na hora o
que estiver congelado.

## "Nao consegui medir" para de virar "esta tudo bem"

Duas leituras de saude diziam que estava tudo bem quando, na verdade, nao
tinham conseguido medir:

- **O contador de erros do disco.** Muitos SSDs nao publicam esse numero, e a
  leitura tambem falha sem administrador. Nos dois casos o produto concluia
  "zero erros" e nao dizia nada.
- **O limite termico do processador.** Falha de leitura virava "nao ha
  throttling" — no modulo cuja funcao e justamente detectar throttling.

Agora os dois dizem, com todas as letras, quando nao foi possivel conferir. E
a frase deixa claro o que isso **nao** significa: nao conseguir medir nao e o
mesmo que estar bem.

## E o diagnostico nao cai mais por causa de um modulo

Se um dos modulos do diagnostico rapido falhar, ele agora vira uma lacuna
declarada — o produto diz o que nao conseguiu ver. Antes, um modulo com
problema derrubava a tela inteira.

## Tambem nesta versao

- O aviso do "Desligar o Controle de Conta de Usuario" passa a dizer que, com
  ele desligado, **aplicativo da Loja da Microsoft nao abre**. O ganho dessa
  opcao continua sendo zero, e agora o custo esta escrito por inteiro.
- O conserto do disco para de mostrar "10 a 60 minutos" ao lado de um clique
  que volta em segundos: esse tempo e o da verificacao no proximo boot.
- Durante o reparo, as linhas de erro ficam destacadas das de progresso — a
  razao de uma falha do DISM nao se perde mais no meio da percentagem.

---

# 1.1.2 — a Steam nunca mais é congelada, e a tela diz o que faz

**Se voce viu "Suspenso" ao lado da Steam, do Discord ou do navegador no
gerenciador de tarefas: nao era defeito da sua maquina. Era o modo jogo
automatico do Otimiza, e esta versao conserta os dois motivos de isso ter
assustado.**

## A Steam nunca mais e congelada

O modo jogo congela programas de segundo plano para devolver a memoria ao
jogo. Ate agora a Steam entrava nessa lista quando nao havia partida com
anticheat rodando.

Na pratica isso e ruim mesmo com o jogo fechado: **Steam congelada nao abre
jogo, nao baixa e nao responde** — e ela e justamente o programa que voce usa
para comecar a jogar. A memoria que se ganha nao paga isso.

A partir desta versao, nenhum lancador de loja e congelado: Steam, Epic,
Battle.net, Riot, EA e Ubisoft.

## E a tela passou a dizer o que a opcao faz

O texto da opcao **"Ligar o modo jogo sozinho quando um jogo abrir"** falava
so de plano de energia, e terminava com "nenhum programa e encerrado".

Era verdade e enganoso ao mesmo tempo. Ele nao dizia que o Discord e o
navegador sao **congelados** — e quem ligou aquilo esperando uma troca de
plano de energia viu o navegador parar sem ter como ligar uma coisa a outra.

Agora a opcao diz, com todas as letras: congela programas de segundo plano,
quais, que nada se perde, e que **enquanto o jogo estiver aberto esses
programas nao respondem**.

## Se algo ficou congelado na sua maquina

**Abra o Otimiza.** So abrir ja descongela tudo na hora, sem reiniciar nada.
Se nao quiser mais esse comportamento, desligue a opcao em **Preferencias**.

---

# 1.1.1 — conserto urgente: programa suspenso quebrava o Explorador

**Se o seu Explorador de Arquivos parou de abrir, ou se clicar num programa
na barra de tarefas não abre nada, esta versão conserta a causa. Atualize e
reinicie o computador.**

## O que estava acontecendo

Durante o jogo, o Otimiza **suspende** programas de segundo plano em vez de
fechá-los — para que a memória volte para o jogo sem você perder nada. Essa
parte continua igual, e é de propósito: fechar o navegador de alguém com
quinze abas abertas é pior que o problema que viemos resolver.

O erro estava em **quando eles voltavam**. Só havia dois momentos: quando o
jogo fechava, e quando você abria o Otimiza de novo. Se o programa fosse
fechado antes disso, os processos ficavam suspensos.

E aí, ao desligar o computador: um programa suspenso não consegue responder
ao aviso de desligamento do Windows. O Windows então não consegue guardar
direito uma parte do seu perfil — justamente a parte que diz ao Explorador
como abrir cada coisa. No login seguinte, o Explorador e os atalhos da barra
de tarefas param de funcionar.

## O que mudou

Agora os programas voltam em três momentos, e não em um:

- **Ao fechar o Otimiza**
- **Ao desligar ou sair do Windows** — mesmo que o Otimiza tenha sido
  encerrado à força
- **Depois de 10 minutos** sem nenhum jogo aberto, como última rede

O aviso de desligamento é respondido na hora, com prazo curto: **o Otimiza
nunca segura o desligamento do seu computador.**

## Se a sua máquina já está com esse problema

Instale esta versão e **reinicie o computador**. O reinício limpo é o que
devolve o Explorador. Não precisa mexer em nada à mão.

Se depois de reiniciar ainda não abrir, fale com a gente no servidor —
nesse caso o perfil do Windows precisa de um reparo, e a gente acompanha.

---

# 1.1.0 — o Otimiza passa a consertar, não só a ajustar

Até aqui o produto sabia **ajustar**: 42 mudanças de configuração do Windows,
todas reversíveis. Ele não sabia **consertar**.

A diferença importa mais do que parece. Quando um arquivo de sistema do Windows
está corrompido, nenhum dos 42 ajustes adianta — o problema não é uma escolha
errada, é um arquivo danificado. O técnico limpa, otimiza, mede, e a máquina
continua ruim. É a mesma história que o Otimiza já contava sobre disco morrendo,
só que desta vez ele passa a resolver em vez de só avisar.

## A aba de Reparo

Quatro ferramentas do próprio Windows, com o que elas fazem dito antes de você
clicar:

| | |
|---|---|
| **Verificar os arquivos do sistema** | Compara os arquivos protegidos do Windows com as cópias boas e reescreve o que estiver diferente |
| **Reparar a imagem do Windows** | Quando a própria cópia de referência está danificada, busca os arquivos bons na Microsoft |
| **Verificar o disco** | Procura erros na estrutura do disco **sem reiniciar a máquina** |
| **Liberar espaço do sistema** | Remove componentes antigos que sobraram de atualizações — são gigabytes que a Limpeza de Disco do Windows não alcança |

**Aqui não existe desfazer, e a tela diz isso antes de qualquer botão.** Não é
descuido: estas ferramentas não mudam ajuste nenhum, elas devolvem arquivos
danificados ao original. Não há valor anterior para guardar, e desfazer
significaria estragar de novo.

## O que este produto faz diferente do que se ensina por aí

**Não pedimos para reiniciar sem motivo.** Todo guia manda rodar `chkdsk /f`,
que reinicia a máquina e prende você numa tela azul por tempo indeterminado. O
Otimiza roda a verificação **com o Windows ligado**, e só oferece o conserto —
esse sim com reinício — **depois de a verificação ter encontrado alguma coisa**.
Sem achado, não há motivo para reiniciar o computador de ninguém.

**E dá para voltar atrás.** Enquanto a máquina não reiniciou, o conserto
agendado pode ser desmarcado.

**Não mexemos em disco que está morrendo.** Se a leitura de saúde acusar
desgaste, erros ou temperatura fora do lugar, o conserto do disco simplesmente
não é oferecido — num disco que já falha, reescrever a estrutura é o que costuma
terminar de matá-lo. E se o Otimiza **não conseguir ler** a saúde do disco, ele
também não oferece: não saber não é o mesmo que estar tudo bem.

## O tempo aparece antes, não depois

Cada ferramenta mostra quanto costuma demorar **antes** de você começar. O
reparo da imagem leva de 10 a 30 minutos e **fica parado em 20% por vários
minutos** — isso é normal, está escrito na tela, e é exatamente o momento em que
as pessoas concluem que travou e desligam a máquina no meio de uma escrita.

O andamento aparece linha a linha enquanto roda, e dá para interromper. Nas duas
ferramentas em que interromper deixa trabalho pela metade, o botão avisa antes
de aceitar o clique.

## E quando não há nada errado

**"Nenhuma corrupção encontrada" é o resultado mais comum, e é um bom
resultado.** A tela diz isso com todas as letras, sem inventar benefício e sem
transformar em problema o que não é.

Quando o reparo conserta uma parte e não consegue o resto, ele também diz isso —
com os dois números, e dizendo qual é o próximo passo. Um reparo pela metade não
é apresentado como sucesso.

## Também nesta versão

- Liberar espaço do sistema mostra quanto dá para recuperar **antes** de você
  decidir
- A opção que libera mais espaço vem desligada, porque ela custa a capacidade de
  desinstalar atualizações do Windows — e isso não tem volta. O aviso fica ao
  lado da caixa, não numa nota de rodapé

---

## O que esta versão não promete

**Reparo não é otimização, e não vai te dar FPS.** Se a sua máquina está bem, a
aba de Reparo vai dizer que está bem e não vai mudar nada. Ela existe para o
caso em que nenhum ajuste adianta porque o problema é outro.

**O "editor desconhecido" continua aparecendo.** O aviso do SmartScreen, lá em
cima, é o Windows dizendo com razão que não sabe quem publicou este instalador.
Resolver isso é comprar um certificado de assinatura, e essa compra ainda não
foi feita.
