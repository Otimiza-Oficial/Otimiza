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

# 1.8.0 — o produto passa no próprio teste

O Otimiza se vende dizendo que só afirma o que mediu, e que admite quando não
sabe. Esta versão nasceu de uma auditoria que fez essa pergunta ao próprio
produto — e achou seis lugares onde ele não estava cumprindo isso consigo
mesmo.

Nenhum recurso novo. É uma versão inteira de tirar afirmação que não se
sustentava, e de tapar dois buracos por onde a promessa de reversibilidade
vazava.

## A primeira tela dizia que o seu disco estava cheio sem ter olhado

Quando o Otimiza não conseguia encontrar o volume do Windows — drive de rede
mapeado, disco sem letra, uma enumeração que falha — ele registrava zero. E
zero, para a tela inicial, era indistinguível de disco lotado. O diagnóstico
anunciava:

> **O disco do sistema está quase sem espaço.**
> Restam 0.0 GB livres no disco do Windows.

Com severidade máxima, no primeiro lugar que você olha, sobre uma máquina que
podia estar com meio terabyte livre.

Agora, não conseguir medir aparece como **lacuna** — "não consegui olhar isto"
—, que é uma frase honesta e que já existia no produto para outros casos.

## E oferecia mexer na sua memória sem ter medido nada

Este era pior, porque não parava na frase. Quando a consulta de memória do
Windows falhava, todos os números viravam zero — e zero de RAM com zero de
paginação satisfaz exatamente as condições do alerta mais grave do módulo:

> **Arquivo de paginação desativado.**
> Nenhuma paginação configurada, com 0.0 GB de RAM.

Junto com um botão que **altera a configuração do seu Windows**. A partir de
uma medição que nunca aconteceu.

Nos dois casos — disco e memória —, a proteção correta já existia a poucas
linhas de distância, no mesmo arquivo, aplicada a um alerta vizinho.

## O "desfazer" podia apagar algo que já era seu

O Otimiza guarda o valor anterior antes de mudar qualquer coisa. Quando o
valor anterior não existia, desfazer significa apagar o que ele criou — para
não deixar sujeira.

O problema: **não conseguir ler** era registrado como **não existia**. Uma
configuração que já era sua, num lugar onde o Otimiza não teve permissão de
ler, entrava no histórico como "isto não estava aqui antes" — e o Desfazer a
removia, achando que estava limpando a própria bagunça.

Agora só "não encontrado" conta como ausência. Qualquer outro motivo para uma
leitura falhar. Perder o que é seu é pior do que não conseguir mudar nada.

Junto disso: o desfazer também podia **relatar sucesso sem ter desfeito**. Se
apagar o valor falhasse, a tela dizia que tinha revertido, a anotação de como
voltar era consumida, e a mudança continuava aplicada — sem chance de tentar
de novo.

## Uma queda de energia podia apagar o desfazer de tudo

O histórico de mudanças era gravado direto por cima do arquivo antigo. Um
desligamento no meio dessa escrita deixava o arquivo pela metade — e um arquivo
pela metade era lido, na abertura seguinte, como **histórico vazio**.

O resultado: todas as otimizações voltavam a aparecer como disponíveis,
"Desfazer tudo" respondia que não havia nada a fazer, e as mudanças seguiam
aplicadas na sua máquina. A promessa central do produto morria por causa de
uma tomada, sem uma palavra na tela.

Agora a gravação é atômica: o arquivo ou é o antigo inteiro, ou o novo inteiro.
E quando algo assim já tiver acontecido, o Otimiza **avisa na tela** e guarda o
arquivo ilegível em vez de passar por cima dele.

## E um arquivo corrompido punia justamente quem pagou

O mesmo padrão valia para a licença. Um arquivo de licença danificado virava
"cliente sem licença", e você via a tela de ativação como se nunca tivesse
comprado — sem nenhuma indicação de que havia uma licença gravada ali.

Agora o Otimiza diz o que aconteceu e o que fazer. O programa continua
trancado, porque licença ilegível não é licença válida, mas você deixa de ser
tratado como quem nunca comprou.

## O caminho até o suporte não sumia mais quando você ativa

Até a 1.7, o endereço do Discord existia num único ponto do programa: dentro da
tela de ativação. Essa tela some quando a licença é aceita — então **quem
comprou era exatamente quem ficava sem nenhum caminho até nós**, de dentro do
produto.

Agora há "Falar com o suporte" no rodapé, ao lado do tutorial, pelo mesmo
motivo que o tutorial está lá: o rodapé não some.

E o convite deixou de ser um endereço fixo que envelhece dentro do executável.
O Otimiza agora pergunta qual é o convite atual no momento em que você clica —
o que conserta o link também para quem instalou versões antigas.

## O que esta versão não promete

**O "editor desconhecido" continua aparecendo.** O instalador ainda não tem
assinatura digital, e o aviso do Windows está certo em dizer que não sabe quem
publicou o arquivo. Resolver isso é comprar um certificado, e essa compra ainda
não foi feita.

Corrigimos, isso sim, o SECURITY.md deste repositório, que afirmava o
contrário — que o instalador era assinado — e ainda apontava para o documento
que explica que ele não é.

---

# 1.7.0 — abre mais rápido, e para de dizer que está tudo bem quando não olhou

Versão de conserto e de medida. O programa passou a **abrir em cerca de um terço
do tempo**, e três telas que tranquilizavam sem ter conseguido verificar nada
passaram a dizer a verdade.

## Abre bem mais rápido

A abertura caiu de **~3,7 s para ~1,2 s** nesta máquina de testes. Dois motivos,
e os dois eram desperdício puro:

**A leitura do processador.** O Otimiza pedia ao Windows a medição completa de
uso da CPU — que custa quase um segundo, porque exige duas amostras separadas por
um intervalo — quando só precisava do **nome** do processador, que vem de graça.

**A detecção do tipo de disco.** A consulta antiga acionava três comandos
encadeados do módulo de armazenamento do Windows, e só carregar esse módulo
levava de 1,5 a 3,5 segundos. Agora a mesma informação é lida direto, e o
caminho antigo continua guardado como reserva para a máquina onde a consulta
rápida não responder.

Nada foi trocado por atalho: o tipo de disco decide se otimizações como o
SysMain são oferecidas, e a classificação continua idêntica — SSD, HD ou
desconhecido, com desconhecido continuando sem virar palpite.

## A saúde do disco parou de tranquilizar sem ter medido

Este é o conserto mais importante da versão, e fica na **primeira tela**.

Quando o Windows não respondia à consulta de desgaste do disco, o Otimiza dizia:
*"Sem dados de saúde disponíveis — acontece em máquinas virtuais e em alguns
controladores antigos"*, marcado como se estivesse tudo certo.

Só que essa explicação era um chute. A consulta podia ter simplesmente falhado —
e a tela afirmava a causa de uma leitura que nunca aconteceu, sobre um disco que
ninguém mediu.

Agora a tela separa quatro situações, e quando não sabe, diz:
**"Não sabemos o estado deste disco — isto não é o mesmo que dizer que ele está
bem."**

## A memória parou de sumir da tela em silêncio

Se a leitura dos pentes de memória falhasse, o diagnóstico não mostrava **nada**
sobre memória. Sem achado e sem aviso — o que, na prática, é indistinguível de
dizer que a memória está boa.

E some justamente com o achado mais valioso do produto: a **memória em canal
único**, que rende mais que todo o catálogo de ajustes somado. Agora, quando a
leitura falha, a tela diz que não conseguiu verificar.

## Quando o Windows barra o Otimiza, ele explica

Duas operações desta versão podem ser negadas pelo Windows mesmo com o programa
aberto como administrador — costuma ser antivírus ou proteção de política.

Antes isso aparecia como um código de erro. Agora a mensagem diz que a causa não
é falta de permissão sua, aponta a origem provável, e afirma que **nada foi
alterado**. E o Otimiza recusa agir quando não consegue ler o estado anterior:
sem saber o que havia antes, não há como prometer o desfazer.

## Debaixo do capô

O ciclo que aplica cada otimização, confere contra o sistema e desfaz voltou a
ser executado nesta máquina, e três tipos de ação que nunca tinham sido provados
foram fechados: **desativar serviço**, **ajuste fino de energia** e
**configuração de inicialização**.

São 677 verificações automáticas, e o programa continua compilando sem um único
aviso.

# 1.6.0 — otimização que estava lá e não fazia efeito

Versão de conserto, e o conserto é incômodo de admitir: **algumas otimizações
estavam gravando a configuração certa e não mudando o comportamento da
máquina**. Apareciam como aplicadas, e não eram.

Onze correções e um recurso novo. Nenhuma otimização foi removida — todas
continuam no catálogo, agora funcionando.

## Notebook na bateria era o pior caso

Três otimizações de processador — estacionamento de núcleos, estado mínimo e
limitação de energia — gravavam só o valor de *ligado na tomada*. O valor de
*na bateria* continuava no padrão do Windows.

Num notebook fora da tomada elas **não faziam absolutamente nada**, e a lista
dizia que estavam aplicadas.

Medido: com o processador exigido em 100% na tomada, a bateria seguia em 5%.
Agora os dois modos são gravados, e a tela só diz "aplicada" quando vale nos
dois.

## Efeitos visuais mudavam o rótulo, não os efeitos

A opção "melhor desempenho" escrevia o rótulo que a tela de Sistema do Windows
mostra — e deixava as animações ligadas. Faltava a parte que realmente governa
sombra, animação de janela e deslizar de menu.

Junto veio o arraste de janela cheia, um dos efeitos que mais pesam em PC
fraco.

## Ajuste que só valia depois de reiniciar agora vale na hora

Desligar a aceleração do mouse e trocar os efeitos visuais gravavam certo e não
mudavam nada até o próximo logon — em telas que prometem efeito imediato.

O Windows guarda essas preferências em memória desde que você entra na conta, e
o programa não estava avisando que elas mudaram. Agora avisa. Vale ao aplicar e
ao desfazer.

## Desligar o VBS tirava a proteção sem entregar o desempenho

Esta é a correção mais séria, porque é a única otimização do produto que cobra
em segurança.

O VBS roda em cima do hipervisor do Windows. O Otimiza desligava o VBS e deixava
o hipervisor subindo no boot — ou seja, o cliente abria mão da proteção das
senhas do Windows e recebia menos desempenho do que foi prometido.

Agora o hipervisor é desligado junto, e continua reversível.

## Teclas de acessibilidade acionadas sem querer

Recurso novo, e só aparece se estiver acontecendo com você.

Segurar o Shift por oito segundos liga a **Filtragem de Teclas** do Windows. Em
jogo, segurar Shift é agachar, correr, andar devagar — acontece sem ninguém
perceber, uma caixa aparece, a pessoa fecha no reflexo, e dali em diante o
teclado passa a ignorar toques. Chega a atrasar um segundo inteiro por tecla.

O Otimiza detecta e desliga, preservando as suas outras preferências de
teclado. **Se você usa esses recursos por necessidade, não aplique** — eles
existem por um bom motivo, e o texto na tela diz isso.

## "Não sei" parou de virar "não se aplica"

Duas telas afirmavam coisa que não tinham conseguido verificar:

- O **Armazenamento Reservado** aparecia como *não se aplica a esta máquina*
  quando, na verdade, o Windows tinha recusado responder
- Os **limites de inicialização** eram dados por limpos mesmo quando a consulta
  não foi respondida

As duas agora dizem que não sabem, e por quê.

E quando o Windows nega uma alteração mesmo com o programa aberto como
administrador, a mensagem parou de ser um código de erro: ela explica que a
causa costuma ser antivírus ou proteção de política, que não é falta de
permissão sua, e que nada foi alterado.

## Ajuste do driver NVIDIA fora do padrão parou de ser confundido

Ao ler um ajuste do perfil da NVIDIA, o driver responde com erro quando aquele
ajuste **nunca foi gravado** — e a leitura certa disso é "está no padrão de
fábrica", não "não sei".

A diferença aparecia no desfazer: o Otimiza podia escrever um zero que nunca
existiu, e a placa ficava com uma configuração que não era nem a sua nem a de
fábrica. Agora a distinção é explícita.

Junto veio a trava que confere o nome do ajuste antes de escrever: se o driver
chama aquele número de outra coisa, o Otimiza **recusa mexer** em vez de
escrever no escuro. E as duas regras passaram a ser testadas sem depender de ter
uma placa NVIDIA na máquina — o que significa que a esteira de publicação também
as verifica agora.

## Desfazer ficou mais seguro

Numa máquina com várias placas de rede, se a alteração falhasse no meio, as
placas já alteradas ficavam **fora do histórico** — o "Desfazer" não alcançava
elas. Corrigido para as placas de rede e para as de vídeo.

# 1.5.0 — o que o produto nao sabe, ele passa a dizer

Versao de conserto e de medida. Tres recursos novos, sete correcoes, e uma
regra que valeu para todos eles: **quando o Otimiza nao consegue medir uma
coisa, ele diz que nao consegue** — nunca "esta tudo bem".

## Copiar diagnostico

Um botao na aba Sistema monta um relatorio curto do seu PC e copia. Voce cola
no atendimento e quem for te ajudar ja sabe o que esta acontecendo.

Ele leva versao do Windows, memoria, monitores, o que esta congelado, o que o
Otimiza ja mexeu, saude do disco e estado termico. **Nao leva nome, documento,
contato nem nada que identifique voce** — e uma trava no codigo garante isso.

O que nao deu para ler aparece como *nao consegui verificar*, em vez de
sumir do texto.

## Aviso de versao nova

Quem comprou fora do Discord nao ficava sabendo quando saia atualizacao. Agora
o proprio programa avisa, com um botao para baixar.

A faixa aparece so quando ha versao mais nova, some quando voce fecha, e
**nunca interrompe o que voce esta fazendo**.

## Perda de pacote contra o servidor do jogo

Travamento na hora de jogar sente exatamente igual a FPS baixo: o carro que
teleporta, o tiro que nao registra. Se voce otimizou o PC e nao sentiu
diferenca, pode ser a rede — e nao o computador.

O Otimiza mede a perda de pacote **contra o servidor em que voce esta jogando
agora**, nao contra um endereco qualquer. Se ele nao descobre com confianca
qual e o servidor, ele nao mede e diz por que.

E aqui esta a parte que mais deu trabalho: **muito servidor de FiveM bloqueia
ping por seguranca.** Uma medicao ingenua diria "20 de 20 perdidos, sua rede
esta destruida" para quem esta com a conexao perfeita. O Otimiza confere a
porta do jogo antes de acusar, e quando o ping esta bloqueado ou limitado ele
diz isso — que nao e perda.

## CitizenFX.ini

A aba do FiveM passa a mostrar o que ja esta configurado em
`PoolSizesIncrease`, ao lado dos tetos oficiais da Cfx.re.

**Só leitura.** O Otimiza nao escreve nesse arquivo, e nao sugere que voce
aumente nada: sem um caso real de pool estourado, sugerir seria adivinhar.
"Nada configurado" e o normal, e e um bom resultado.

## Correcoes

- **Reparo do Windows com falhas ilegiveis nao le mais como sucesso.** Um
  reparo que corrigiu parte e nao conseguiu ler o resto dizia "corrigido".
  Agora diz o que corrigiu, o que nao deu para ler, e qual e o proximo passo.
- **Relatorio de suporte nao chama mais bateria de disco.** Notebook com a
  bateria no fim e o SSD perfeito saia como "disco critico".
- **Disco que nao pode ser lido nao aparece mais como "saudavel".**
- **A medicao de rede ficou 15x mais rapida no pior caso** — de cerca de 78
  segundos para menos de 5 — e parou de prender o resto do programa junto.
- **Copiar diagnostico nao congela mais a janela**, e avisa que vai demorar.
- **A descoberta do servidor do jogo parou de aceitar qualquer endereco.**
- **Mensagem de erro do CitizenFX.ini parou de chutar a causa.**

## Por baixo

Duas travas novas: uma reprova o build se os quatro arquivos de versao
discordarem entre si, outra se a tag nao bater com o que o binario carrega.
Elas nasceram de um erro real desta propria versao.

543 testes automaticos, todos passando.

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
