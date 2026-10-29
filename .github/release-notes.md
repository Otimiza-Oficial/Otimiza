Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza-instalador.exe.sha256` | A soma do instalador, para conferir que o arquivo chegou inteiro |
| `Otimiza_3.5.0_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_3.5.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão. Se o Otimiza disser que a sua chave é do sistema de
licenças antigo, fale no suporte com o código da máquina: a chave nova sai sem
custo.

---

# 3.5 Turbo (3.5.0): o jogo mais leve, com um botão

## Turbo

Um botão novo no topo do Boost. Com o jogo aberto, o Turbo mede por 10 segundos o que está no limite e liga só o que
ajuda **naquele jogo, naquele PC**:

| O que está no limite | O que o Turbo liga |
|---|---|
| Placa de vídeo | A escala (o jogo desenha menos pixels: FPS real a mais) e, se ainda faltar para o monitor, a geração de quadros |
| Processador | Tira a disputa do processador (programas de fundo em modo econômico enquanto o jogo roda), a geração de quadros e a energia de jogo. Diminuir a resolução não ajudaria: a placa já sobra |
| Memória | Tira a disputa do processador e mostra o que ocupa memória para você fechar (nunca fecha sozinho) |
| Nada | Não liga nada, e diz que o jogo já roda o que o seu PC consegue |

Em jogo competitivo (CS2, Valorant, Fortnite, Call of Duty, Apex, Rocket League) o Turbo não liga a geração de
quadros: ela acrescenta atraso. A receita fica lembrada para cada jogo: da próxima vez, um clique, sem medir de novo.
Ao desligar, o cartão mostra o FPS do jogo antes e depois.

**Provar:** na primeira vez que o Turbo liga uma receita num jogo, ele faz a prova sozinho (e depois, quando você quiser,
pelo botão). Por uns 5 minutos, enquanto você joga, o Turbo liga e
desliga sozinho em 8 rodadas alternadas e mede o FPS do jogo em cada uma. O resultado sai com margem de erro
("41 ± 1 → 58 ± 2 fps"), e dá para copiar e colar no Discord. Se o Turbo deixou o jogo pior, ele se desliga sozinho e
não liga de novo aquela receita naquele jogo: nunca menos FPS.

**Menor atraso ou mais FPS:** você escolhe por jogo se o Turbo pode ligar a geração de quadros. Jogos de tiro já vêm em
"Menor atraso".

**Núcleos rápidos:** com o processador no limite, depois de uma prova que não piorou o jogo, o Turbo mede em cerca de
1 minuto se prender o jogo nos núcleos mais rápidos rende. Só fica se render.

**Leveza:** além do FPS médio, o cartão e o "Copiar resultado" mostram o 1% piores quadros antes e depois, com o selo
"mais liso" quando o ganho passa da margem de erro.

**Ritmo estável:** quando o jogo trava várias vezes por minuto, o Turbo mostra quantas e oferece travar o FPS no que o
seu PC sustenta (placa NVIDIA, vale ao reabrir o jogo, desfaz em um clique). É uma oferta: o teto troca um pouco da
média por um ritmo constante, e quem decide é você.

Se você liga o modo jogo por conta própria, o Turbo não mexe nele.

Com o jogo aberto, o Turbo é o botão principal da tela inicial.

## "Otimizar agora" muito mais completo

O botão aplicava poucos ajustes: quase tudo dos pacotes de otimização estava como "só se pedir". Agora ele tem dois
níveis, e o **Completo** é o padrão:

- **Completo:** além do de sempre, 17 ajustes do registro, de serviços e da rede: o jogo com prioridade no processador e no
  agendador de multimídia, o timer fino valendo para o sistema todo, telemetria, Edge e widgets parados, a placa de rede sem economia de energia (o pico de ping de quando ela
  acorda no meio da partida), e o Windows sem as esperas de fábrica no menu e no boot. Nada que possa
  custar quadro, nada que troque segurança e nada que tire uma função do Windows (Bluetooth, impressora, Xbox, os avisos
  dos apps da Store, a manutenção do disco e os drivers pelo Windows Update ficam como estão). Parte deles só vale depois de reiniciar, e o resultado diz quantos.
- **Seguro:** o de antes, para quem quer o mínimo.

## Plano de energia OTIMIZA, de verdade

O plano agora decide pela máquina. Em **desktop**, o processador fica em desempenho total na tomada, com todos os
núcleos acordados, boost agressivo e o caminho até a placa de vídeo e o USB sem economia de energia. Em **Intel de
12ª geração em diante** (núcleos P e E), a escolha entre os núcleos continua com o Windows. Em **notebook**, só o que
é seguro na tomada; o resto continua com o motor de energia, que mede antes de mudar, porque fixo ele esquentava
notebook.

## Escala no Frame Boost (FSR 1)

O Frame Boost agora também **amplia**: o jogo roda numa janela menor e o Otimiza amplia para o monitor inteiro com o
FSR 1 da AMD, nítido e sem halo. Quatro níveis, com os nomes que você já conhece do menu dos jogos (Ultra qualidade,
Qualidade, Equilibrado, Desempenho). Junto com a geração de quadros, ou sozinha, sem atraso nenhum.

- O Otimiza põe o jogo no tamanho do nível e devolve a janela exatamente como estava ao desligar.
- O cursor aparece ampliado, no lugar certo, e o mouse fica preso na janela do jogo.
- Com escala, a imagem nunca some no meio da partida: menu e tela de carregamento ficam na tela.
- **Comparar a imagem:** um botão no Frame Boost mostra o centro da tela lado a lado, ampliado do jeito comum e pelo
  FSR do Otimiza, com um divisor para arrastar.
- O contador por cima do jogo mostra a escala e, com a geração, o que chega à tela.

Tudo por fora do jogo: nada é injetado, como no Frame Boost de sempre.

## Escala do driver, para tela cheia

Para quem joga em tela cheia exclusiva, onde nada fica por cima do jogo: o **NVIDIA Image Scaling** e o **Radeon Super
Resolution** agora ligam pelo Otimiza, em Driver da placa de vídeo, com o desfazer de sempre. Escolha no jogo uma
resolução menor que a do monitor e o driver amplia.

## Configuração do jogo: Fortnite, CS2 e Apex

O que custa caro e quase não se vê (sombras, oclusão de ambiente, partículas, pós-processamento) no arquivo que o
próprio jogo grava. O Otimiza só muda o que já existe no arquivo, nunca com o jogo aberto, e o desfazer devolve o
arquivo exatamente como estava. No CS2, as sombras dos jogadores ficam como estão: elas mostram onde o inimigo está.
Valorant e Warzone estão em estudo e ainda não são alterados.

## Licença mais protegida

- A licença foi reforçada contra uso sem compra. Quem comprou não precisa fazer nada: a chave de sempre continua
  abrindo no mesmo computador.
- Chaves do sistema de licenças antigo (de antes de 04/10/2026) só abrem nos computadores para os quais foram
  vendidas. Se a sua for recusada, o suporte troca sem custo.
- O Otimiza chama os programas do Windows (PowerShell, plano de energia, inicialização) sempre pela pasta do sistema,
  nunca por um arquivo com o mesmo nome em outra pasta.
- A sobreposição de FPS e o modo dinâmico de energia, quando ligam sozinhos com o jogo, conferem a licença como os
  botões que os ligam. Voltar ao normal continua livre.
- A licença continua sendo conferida no próprio computador, sem precisar de internet.

---

# 3.4 Catálogo completo (3.4.0): a logo nova e tudo dos pacotes de otimização

## Logo nova

O Otimiza ganhou marca nova: no app, no ícone da barra de tarefas, no instalador e no site.

## Tudo dos pacotes de otimização, um por um, com desfazer

Os ajustes dos pacotes de otimização mais usados (.reg e .bat que circulam no Discord e no YouTube) agora estão no
Otimiza, cada um com o que faz, o preço escrito e o botão de desfazer. Você escolhe item por item: nenhum entra
sozinho no "Otimizar agora", porque o que ajuda num PC pode atrapalhar no outro.

**Processador e memória:** SystemResponsiveness 0, NetworkThrottlingIndex, Win32PrioritySeparation, perfil de
multimídia para jogos completo, limitação de energia por processo, SysMain e compressão de memória (com o aviso de
engasgo em PC de 8 GB).

**Placa de vídeo:** NVIDIA sempre no clock máximo (DisableDynamicPstate), sobreposições MPO desligadas, otimizações de
tela cheia do Windows desligadas e, no driver NVIDIA, a taxa de atualização "a mais alta disponível".

**Boot e relógio:** timer global do Windows 11, tick fixo (disabledynamictick, useplatformtick, tscsyncpolicy), sem
Inicialização Rápida, serviços agrupados, Prefetch desligado, fila curta do mouse e do teclado.

**O que roda por trás:** serviços em grupos (Bluetooth, impressora e scanner, máquina virtual, serviços pouco usados,
serviços de empresa), Xbox, indexação, telemetria, relatório de erros, notificações, Manutenção Automática,
aplicativos e navegadores em segundo plano (Edge, Chrome e Firefox), Windows Update sem trocar o driver de vídeo.

**Rede:** os parâmetros de TCP dos pacotes e o conjunto "Bufferbloat" completo. O Bufferbloat tira trabalho da placa
de rede e passa para o processador: em jogo preso no processador ele pode custar quadro, e o item diz isso.

**Troca de segurança, só com aviso:** proteções contra Spectre e Meltdown desligadas, como o Isolamento de Núcleo.

**O que continua fora, e por quê** (tela "O que não fazemos"): ativador do Windows, antivírus, UAC, DEP, SmartScreen e
IPv6 desligados, chaves de DPC e de latência que o Windows não lê, apagar os logs de eventos. Ou tiram a proteção do
seu PC sem dar quadro, ou não fazem nada.

**Corrigido:** o ajuste do Edge gravava a política com um nome que o Edge não lê. Agora ele para de verdade de ficar
aberto por trás.

## Frame Boost na tela cheia

O Frame Boost não recusa mais jogo em tela cheia. A tela cheia dos jogos de hoje passa pela composição do Windows e
aguenta o Frame Boost por cima sem perder FPS (medido com o jogo de teste). Se o jogo usar a tela cheia exclusiva
antiga e ficar saindo e voltando, o Frame Boost percebe, desliga e pede Sem bordas.

## Windows modificado

Em **Ajustes**, o Otimiza diz quando o Windows do PC é uma versão modificada (as "ISOs otimizadas") e quem fez. Nelas
parte do Windows vem cortada, e é bom saber disso antes de culpar o jogo ou o Otimiza.

---

# 3.3 Ajustes (3.3.1): o Frame Boost não promete o que não foi medido

A 3.3.0 ligava, junto com o Frame Boost, a "otimização para jogos em janela" do Windows para o jogo, prometendo tirar o
modo sem bordas da cópia de cada quadro. Medimos com o PresentMon: o jogo continuou na cópia em todos os quadros, com a
opção do jogo e com a global. Ela sai do Frame Boost, e o que a 3.3.0 gravou volta como estava na próxima vez que ele
ligar. O resto da 3.3 continua igual.

---

# A 3.3 olha para o jogo que você está jogando

## Chaves de licença

**As chaves novas do bot passam a abrir.** Em 04/10 o bot de vendas trocou o par de chaves das licenças, e a 3.2 só
conhecia o antigo: toda chave emitida desde então era recusada com "Esta chave não foi reconhecida". A 3.3 aceita as
chaves novas, e quem já tinha ativado continua ativado — não precisa de chave nova.

## Interface nova

Início, Jogos, Boost, Mira, Cores, Lab e Ajustes, numa barra lateral com
nome em cada ícone. As telas da 3.2 continuam em **Ajustes → Modo clássico**.

**O Início virou um painel:**
- No topo, o jogo, com a arte da Steam do seu PC, e o FPS medido da última partida (ou do jogo aberto).
- O **status do PC**: Energia de jogo, memória livre, ritmo do jogo e monitor, cada um conferido agora. O anel conta
  só o que passou na conferência, nunca um "100%" de enfeite.
- O **desempenho ao vivo** de processador, placa de vídeo e memória, com o gráfico dos últimos 60 segundos.
- **Ações rápidas** para Energia de jogo, Liberar memória, Jogo liso, Frame Boost, Driver e Testar FPS.
- **A imagem do topo é sua:** passe o mouse no topo e clique em "Pôr imagens" (várias de uma vez; JPG, PNG ou WebP, até 6 MB cada, até 8
  imagens, que se alternam). Ficam só no seu PC. Sem imagem sua, o topo usa a arte do jogo da sua Steam.
- Preto e branco, em todas as abas: a única cor é a das imagens. A coluna de jogos saiu das telas (os jogos ficam
  na aba Jogos, agora com a arte de cada um), e os cartões "Em breve" do Boost também.
- **A aba Jogos mostra a sua prateleira:** os jogos instalados pela Steam, pela Epic e pelos instaladores de jogo
  (Riot, Rockstar, FiveM…), com a capa da Steam do seu PC, ao lado dos já medidos. Jogo ainda não medido diz
  "ainda não medido" em vez de um número.

## Memória para o jogo

Num PC de 8 GB, o navegador e o Discord abertos podem ocupar metade da memória.
Quando o jogo precisa de mais, o Windows usa o disco no lugar da RAM, e o jogo
trava justamente ao carregar cenário novo (o tranco ao virar a câmera rápido).
O cartão mostra, medido, quanta memória está livre e quem está ocupando, e fecha
cada programa com um clique — só o que você escolher, nunca sozinho.

## Mouse e teclado

Em **Boost → Mouse e teclado** (e pelo atalho na aba Mira), a **correção de precisão**:
- **Sem aceleração do mouse:** o mesmo movimento da mão leva a mira sempre à mesma distância.
- **Ponteiro 1:1:** a velocidade do Windows na marca em que cada contagem do sensor vira um pixel, sem pular nem
  perder movimento (a ideia do MarkC Mouse Fix). Para mudar a sensibilidade, use o DPI do mouse.
- **Teclado sem demora de repetição:** a tecla segurada repete na hora.
- **Teclas de acessibilidade sem querer:** segurar o Shift no jogo não liga mais a Filtragem de Teclas.

Nada disso mira por você nem muda o FPS: é o Windows parando de mexer no que a sua mão faz. Cada item se desfaz em
um clique.

## Remoção de lixo

Em **Boost**, o tamanho medido antes de limpar, por item: temporários do seu usuário, temporários do Windows e
relatórios de erro. Só arquivo com mais de 2 dias, e o que estiver em uso fica. Nunca a Lixeira, Downloads ou arquivos
seus. Libera disco; não muda o FPS.

## Windows mais leve

Efeitos visuais no mínimo (menus e alt-tab abrem na hora) e a lista do que abre com o Windows, com uma chave para
cada programa. Não muda o FPS: deixa a área de trabalho leve e sobra memória para o jogo.

**Tema claro de verdade.** O botão de sol, no alto, agora muda o app inteiro: fundo claro, cartões brancos, texto
escuro e o botão principal preto. O que fica sobre imagem (o topo do Início, as capas) continua escuro para a arte
aparecer.

**Ícones novos**, desenhados para o Otimiza: velocímetro no Boost, frasco no Lab, monitor nas Cores, pente de memória,
placa de vídeo e o gráfico que sai irregular e termina reto no Jogo liso.

## Energia de jogo

Com o jogo aberto, o processador sobe de frequência na hora em vez de esperar a carga
se firmar; quando o jogo fecha, volta ao ajuste de antes. O cartão mostra o número lido
no seu plano de energia (ex.: preferência de energia 33 → 0). Num jogo preso no
processador, cada espera para subir é um quadro atrasado. Notebook: só na tomada.
Precisa do Otimiza aberto como administrador. Se o Otimiza fechar no meio, o ajuste
volta na próxima abertura. **Desfazer tudo** também desliga.
Em PC com até 8 GB de memória ou até 4 núcleos, já vem ligada; quem desliga fica desligado.

**Modo de Jogo do Windows.** Se ele estiver desligado no seu PC (o Windows o traz
ligado), o Otimiza oferece religar: com o jogo aberto, o Windows dá prioridade a ele
sobre os programas de fundo. Desfaz em um clique.

**Jogo liso não trava jogo que já está liso.** Com menos de 2 trancos por minuto, o
cartão diz que está liso e não sugere teto (travar só tiraria FPS).

**Atualizar com o Otimiza aberto.** O instalador confere se o Otimiza ainda está
rodando (inclusive como administrador) e pede para fechar, em vez de terminar sem
trocar o programa.

## Jogo liso

O que faz o jogo travar não é falta de FPS, é o FPS **oscilando** (40, depois 76,
depois 45) — cada queda é um tranco. O Jogo liso mede o seu jogo e **trava o FPS
num ritmo constante** que o seu monitor mostra por igual. O número médio quase não
muda; o que muda é que para de engasgar, e é isso que se sente jogando.

- Mede 20 segundos e mostra, medido: quantos quadros por segundo o jogo entrega
  e quantas vezes por minuto ele trava.
- Trava num valor que o seu PC segura sempre (ex.: 45 num monitor de 180 Hz).
- Vale na próxima vez que você abrir o jogo, e desfaz em um clique.

**Corrigido: o teto não chegava ao FiveM.** O FiveM cadastra o próprio caminho no driver
da NVIDIA, num perfil dele, e o driver usava esse perfil em vez do que o Otimiza
gravava. O "Travar" agora grava no perfil que o driver usa para o jogo aberto, e
o cartão mostra o teto que está valendo de verdade. Se um teto gravado antes não
chega ao jogo, o cartão diz.

**Tela cheia.** O cartão mostra como o quadro chega ao monitor. Em janela, o
Windows copia cada quadro antes de mostrar, e num jogo preso no processador isso
custa FPS: no FiveM medido, 61 fps em janela sem bordas e 85 em tela cheia, no
mesmo lugar do mapa. Quando o jogo está em janela, o cartão pede tela cheia.
Ele também avisa quando um programa fica "sempre por cima" no monitor do jogo
(um dock, uma barra) e faz o jogo minimizar sozinho em tela cheia, dizendo o nome.

**O "Otimizar" parou de tirar o limite de FPS do FiveM.** Num PC preso no
processador, rodar sem limite deixa o ritmo mais irregular — o contrário de liso.
Agora quem trava o FPS é o Jogo liso, num valor estável.

## Frame Boost

Um botão só, que escolhe o melhor gerador de quadros que funciona aqui: o do
próprio jogo (DLSS, FSR ou XeSS), depois o do driver (AMD Fluid Motion
Frames) e por último o gerador do Otimiza, que roda por fora do jogo, sem
injetar nada.

- **O que muda é a fluidez, não o contador.** O FPS por cima do jogo continua
  mostrando o FPS do próprio jogo, como o `cl_drawfps`; o painel do Frame Boost
  mostra, medido, quantos quadros chegam à tela. Antes de ligar, só uma faixa
  marcada como ESTIMADO.
- **Nunca passa do monitor:** num monitor de 180 Hz, o máximo é 180.
- **Nunca tira FPS do jogo sem dizer.** De tempos em tempos ele compara o FPS do jogo com e sem o gerador. Se o jogo
  perde, primeiro passa para o **modo leve** (calcula o movimento numa resolução menor) e confere de novo; só desliga
  se ainda perder. Com a placa já quase cheia, começa leve.
- **Mostra quanto custa, medido.** Ligado, o painel diz quanto da placa o gerador usa e quanto do FPS o jogo
  manteve ("manteve 98% do FPS, medido em 3 comparações"). Desligado, mostra a última vez: quanto tempo, o FPS do
  jogo e quantos quadros chegaram à tela.
- **"FPS na tela" é o que chegou à tela.** Com o Otimiza como administrador, o número vem do PresentMon contando os
  quadros que o monitor recebeu; sem administrador, aparece como ESTIMADO.
- O gerador do Otimiza agora **se refaz sozinho** quando a janela do jogo muda
  de tamanho, é minimizada, vai para outro monitor ou o Windows troca a
  resolução. Antes, ele desligava.
- Usa o Hz do monitor onde o jogo está (não o do principal), funciona em
  monitor girado (em pé) e em tela HDR.
- **Corrigido:** o gerador contava como quadro do jogo qualquer coisa que
  mudasse na tela (um vídeo, o próprio Otimiza) e às vezes não gerava nada.
  Agora conta só o que muda dentro da janela do jogo.
- **Imagem sem quadradinhos ao girar a câmera.** O gerador do Otimiza passou
  a calcular um movimento para cada pixel, em vez de um para cada bloco: o
  cenário sai liso e o personagem parado no meio da tela não rasga mais.
- **FPS base travado para o Frame Boost.** Ao ligar, o Otimiza trava o FPS do jogo numa fração do monitor (60 num de
  180 Hz) pela medição do Jogo liso: cada quadro real ganha sempre o mesmo número de gerados, em vez de 2 num momento
  e 3 no outro. Vale ao reabrir o jogo; o painel diz quando está valendo e tem o botão Destravar, que fica valendo até
  você tocar em Travar de novo. Nunca trava abaixo de um quarto dos Hz (base baixa assim não fica lisa).
- **Auto: enche o monitor.** Em vez de 2×, 3× ou 4× fixo, o Auto gera
  quantos quadros couberem: com o jogo a 48 fps num monitor de 180 Hz, a tela
  recebe 180 (o 3× parava em 144). É o novo padrão.
- **Auto no ritmo do monitor.** No Auto, cada atualização da tela recebe um
  quadro feito para o instante exato em que ela aparece, pelo horário de cada
  quadro do jogo. Antes os quadros saíam por cronômetro e às vezes dois caíam na
  mesma atualização: o micro-tranco.
- **Menos pixel quebrado na borda de quem se move.** O gerador calcula o
  movimento de ida e de volta entre os dois quadros e sabe o que some e o que
  aparece atrás de cada objeto: a cor sai só do quadro onde aquele pedaço existe,
  e onde os dois lados discordam ele usa o valor do meio em vez de um pixel
  errado (as técnicas do SVP e das máscaras de desoclusão do FSR 3).
- **Avisa quando não vale a pena.** Com a memória do PC no limite ou a placa de
  vídeo cheia, o painel diz antes de ligar que o Frame Boost vai dar tranco.
- **Troca de cena sem quadro quebrado.** Num corte de câmera, teleporte ou ao abrir o mapa, os dois quadros reais não
  têm nada em comum, e o gerador inventava uma mistura das duas cenas. Agora ele percebe (na própria placa, sem custo
  de CPU) e mostra o quadro real. Medido nos quadros gravados do FiveM: pares normais casam de 55% a 100%, um corte de
  2% a 13%; no corte, o quadro mostrado é igual ao real, e nos quadros normais a qualidade não mudou.
- **Sem trancos de tempos em tempos.** Para conferir que o jogo não perde FPS,
  o Frame Boost pausava 1,5 s a cada ~13 s, e quando a conferência falhava ele
  repetia para sempre. Agora são três conferências no começo e depois uma a
  cada 5 minutos; a que falha espera cada vez mais.
- **Fechar o Otimiza no meio do jogo não apaga mais o FPS da tela.** Com o jogo
  aberto, fechar a janela deixa o Otimiza na bandeja do Windows (perto do
  relógio). Clique no ícone para abrir; quando o jogo fecha, ele sai sozinho.

**O contador firme.** O número não pula mais de 140 para 90 e volta, nem cai a
zero num instante: ele acompanha a média e segura o último valor medido por
até 3 segundos quando a medida falha. Nunca mostra um número que não foi medido.

**No FiveM, o FPS no lugar do `cl_drawfps`.** Em **Ajustes → FPS por cima do jogo**,
"No lugar do contador do FiveM" desliga o contador do FiveM no arquivo de
configuração dele (com o jogo fechado, guardando o valor de antes) e põe o do
Otimiza no mesmo canto, no mesmo estilo, com o FPS do jogo. Desligar devolve o contador do FiveM como
estava.

Ele acrescenta um pouco de atraso (a tela mostra quanto, como estimativa) e
precisa do jogo em janela ou sem bordas. Em servidor de RP, confira se o
servidor permite janela por cima do jogo.

## Placas AMD e Intel

Os ajustes de driver deixam de ser só da NVIDIA. Na AMD, pelo ADLX oficial:
Radeon Anti-Lag, Fluid Motion Frames e limite de FPS. Na Intel, pela
biblioteca oficial: baixa latência e limite de FPS. Cada ajuste guarda o
valor de antes e se desfaz em um clique.

É a primeira versão com AMD e Intel: se algum ajuste não aparecer na sua placa,
mande o diagnóstico de **Sistema** para o atendimento.

## Mira

Uma mira fixa no centro da tela, desenhada pelo Otimiza numa janela por cima
do jogo, sem tocar no jogo: ponto, cruz, círculo ou T, cor, tamanho e
contorno, e ela pode sumir enquanto você segura o botão direito. No CS2, o
Otimiza monta os comandos da mira do próprio jogo. Também tem conversor de
sensibilidade e um treino de tracking. Nada disso mira por você.

Em jogos com anticheat que proíbe janela por cima (Valorant, FACEIT), a mira
não liga.

**Campo de visão do FiveM.** Na aba Mira, a mesma opção do menu do jogo (campo de
visão em primeira pessoa), sem abrir o jogo. Grava no arquivo do FiveM com ele
fechado (aberto, fica para quando ele fechar), guarda o valor de antes e desfaz
em um clique. Servidor que fixa o FOV vence.

## Cores

Saturação, brilho, contraste e gama do monitor, com prévia antes e depois. Na
NVIDIA, pela vibração digital do driver; na AMD, pelas cores do Adrenalin; em
qualquer placa, brilho, contraste e gama pelo Windows. Com **"só com o jogo
aberto"**, as cores do jogo ligam quando ele abre e voltam ao normal quando
fecha — e voltam também se o Otimiza fechar sem querer.

## Lab

Testar Performance: rodadas no mesmo trecho antes e depois de otimizar,
medidas pelo PresentMon, e o resultado diz quanto dá para confiar (alta,
média ou sem diferença).

## FPS por cima do jogo

Opcional, em **Ajustes**: Mínimo (só o FPS) ou Detalhado (FPS, 1% low e
tempo de quadro), no canto que você escolher. Só aparece com o jogo na
frente, aparece nos prints como o contador do jogo e não pega clique. Mede pelo PresentMon, então
precisa do Otimiza aberto como administrador; sem isso, ele avisa em vez de
inventar um número.

## Feedback

Quem tem a chave paga usa o Otimiza normalmente por 10 minutos. Depois disso,
as otimizações ficam pausadas até chegar o seu feedback no canal de
feedbacks do Discord (um texto de 30 caracteres ou mais e um print com o
FPS). A medição continua funcionando. Se o feedback for apagado, o pedido
volta.

## Experimental

- **NVIDIA Smooth Motion pelo Otimiza** (RTX 40 e 50), em **Ajustes**,
  desligado por padrão. Usa um ajuste do driver que a NVIDIA não documenta:
  se o seu driver não conhecer, o Otimiza recusa e não muda nada.

## Números mais honestos

- Os "custa X dos quadros" das configurações do jogo agora dizem
  **ESTIMADO**: são faixas medidas por terceiros, não no seu PC.
