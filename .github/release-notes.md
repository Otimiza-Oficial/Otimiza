Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_2.9.0_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_2.9.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão.

---

# A 2.9 é feita para qualquer jogo — e nunca para tirar FPS

## Nunca menos FPS

Todo ajuste de configuração de jogo que o Otimiza aplica fica **em
observação**. O Otimiza compara as partidas de antes com as de depois. Se o FPS
médio ou o 1% pior caírem de verdade — com os intervalos de confiança
separados, e em pelo menos 5% —, **o ajuste é desfeito sozinho** e a ficha do
jogo mostra os números. Se melhorou, ou ficou igual, fica registrado também.

Isso depende da medição automática das partidas, que exige o Otimiza aberto
como administrador e a opção ligada em Sistema → Preferências.

## Todo jogador ganha igual

Não existe mais perfil "competitivo" contra "equilibrado" decidindo quem ganha
o quê. A única escolha é **quanto de imagem você aceita trocar por FPS**:
Máximo de FPS, Equilibrado ou Qualidade. O Otimiza **nunca põe limite de FPS
sozinho** — limite baixa o FPS médio por definição.

## Mais jogos com ajuste de verdade

- **Jogos em Unreal Engine** (Fortnite, Valorant, PUBG, Marvel Rivals e boa
  parte dos lançamentos): a ficha do jogo na Biblioteca baixa só o que mais
  custa FPS — sombras, pós-processamento, efeitos, folhagem —, mostra antes o
  que vai mudar, nunca sobe nada, nunca mexe em resolução, distância de visão
  ou limite de FPS, e guarda o arquivo inteiro para desfazer byte a byte.
- **Biblioteca maior:** além de Steam e Epic, entra o que o Windows registra
  como instalado por Riot, Blizzard, EA, Ubisoft, GOG, Rockstar, Roblox e
  FiveM — e **todo jogo que o Otimiza vê rodando** entra sozinho.

## O que está segurando o seu jogo

- **Mapa de desempenho** (Painel): mede 40 segundos com o jogo aberto —
  processador núcleo por núcleo, memória, disco, placa, memória de vídeo e os
  quadros do jogo — e o **mesmo classificador do painel ao vivo** diz onde
  está o limite. Mostra FPS, 1% e 0,1% piores, P99 e cada travada por
  gravidade.
- **Detetive de travadas:** para cada travada, o que a máquina fazia naquele
  meio segundo — disco, falta de RAM, memória de vídeo cheia, núcleo no teto
  ou **qual programa disparou**. É coincidência no tempo, e a tela diz isso.
- **Limites de FPS escondidos:** limitador global e V-Sync forçado no driver da
  NVIDIA, RTSS aberto, limite gravado no arquivo de jogos Unreal, e o **limite
  de 60 FPS de fábrica do Roblox** — visto nesta máquina: Roblox a 59,5 FPS
  com processador e placa a 35% num monitor de 180 Hz. Para cada um, onde
  tirar.
- **Pronto para jogar?** (Biblioteca): em 2 segundos, monitor abaixo da taxa
  que suporta, limites escondidos, programa pesando agora e memória apertada.
- **O desempenho caiu?** Cada partida medida guarda a versão do driver de
  vídeo e do Windows. Se um jogo cai depois de uma atualização, a ficha dele
  mostra a queda e o que mudou.
- **Desempenho perdido:** o diagnóstico diz se o hardware está entregando menos
  do que pode, com cada problema em prioridade P0 a P3.

## Modo jogo novo

Saíram o plano "Alto Desempenho" fixo e a prioridade alta dada ao jogo às
cegas. No lugar: enquanto o jogo roda, **programas em segundo plano que estão
disputando processador** passam a rodar em prioridade baixa e no modo
econômico do Windows. Nada é fechado nem congelado; Discord, OBS, áudio e
anticheats nunca são tocados; tudo volta quando o jogo fecha. A energia do
jogo fica com o motor de energia, que usa o perfil medido para cada jogo.

## Driver NVIDIA: por jogo, não no PC inteiro

- **"Desempenho máximo" e V-Sync forçado desligado não são mais aplicados no
  perfil global.** O primeiro deixa a placa acordada até na área de trabalho;
  o segundo quebra o G-SYNC/FreeSync de todo jogo. Quem já tinha aplicado
  continua vendo o botão de desfazer.
- **Perfil NVIDIA na ficha do jogo** (Biblioteca): Competitivo ou Baixa
  latência. Antes de aplicar, a ficha mostra cada ajuste com o valor de hoje e
  o valor que fica, lidos do driver. Vale só para aquele executável; nenhum
  perfil mexe em V-Sync nem põe limite de FPS, e ajuste que já está mais rápido
  fica como está. Desfazer apaga o perfil que o Otimiza criou, ou devolve cada
  ajuste ao que era. O perfil entra na mesma vigília "nunca menos FPS" dos
  ajustes de jogo: se as próximas partidas medidas caírem de verdade, ele é
  desfeito sozinho. Ganho **não validado** nesta versão.

## O ajuste certo para este computador

- **Ajustes que dependem da máquina só aparecem quando valem.** O Game DVR só
  aparece com a gravação em segundo plano ligada; efeitos visuais e
  transparência, em PC com até 8 GB ou até 4 núcleos; hibernação e
  Armazenamento Reservado, com menos de 20 GB livres; Widgets e Edge em
  segundo plano, com até 16 GB. Cada um diz por que apareceu. Leitura que
  falha não esconde nem aplica.
- **Modo Expert** (aba de ajustes): agendamento de GPU, MSI da placa, VBS e
  indexação de busca só aparecem nele, nunca entram no "Otimizar agora", e
  pedem medição antes e depois. Nele também ficam dois diagnósticos, só de
  leitura:
  - **Interrupções por dispositivo (MSI):** como cada dispositivo avisa o
    processador. O Otimiza só muda isso na placa de vídeo — em outro
    dispositivo, um driver que não aguenta impede o Windows de ligar.
  - **Drivers prendendo o processador (DPC):** quanto de cada núcleo foi para
    interrupção durante 10 segundos. Diz qual núcleo, não qual driver.

## Otimizar e testar, num clique

Na ficha do jogo (Biblioteca), **"Otimizar e testar"** aplica o ajuste gráfico
Equilibrado e o perfil NVIDIA Competitivo de uma vez, cada um com o estado
anterior guardado — e depois não promete nada: os dois ficam em observação, e o
que piorar nas próximas partidas medidas é desfeito sozinho. Quem quiser
escolher o orçamento de imagem continua escolhendo, logo abaixo.

## Tudo o que o Otimiza altera, num lugar só

No Painel, ao lado de "o que o Otimiza se recusa a fazer", entra a outra
metade: **as 44 alterações que o produto pode fazer neste computador**, cada
uma com o que muda, o risco, se pede reiniciar e **como se desfaz**. A lista
sai do próprio código, e um teste da esteira reprova a versão se algum pedaço
do produto passar a escrever sem aparecer nela.

O Painel também ganhou **"Pronto para jogar?"** (que saiu da Biblioteca, porque
olha a máquina e não um jogo) e **"Suas últimas partidas"**, com o que a
medição automática registrou por jogo.

## Auto CPU Set (processador híbrido)

Em processador com núcleos de desempenho e de eficiência, **"Testar e
decidir"** (aba Núcleos, com o jogo em partida) mede o jogo em todos os
núcleos e só nos de desempenho, alternando, por cerca de um minuto. O jogo
fica nos de desempenho só se o FPS ou o 1% piores melhorarem de verdade sem
nenhum dos dois piorar; empate volta para todos. A escolha é reaplicada
quando o jogo abre de novo, e "Esquecer" desfaz. Com anticheat rodando, o
Otimiza não mexe nos núcleos do jogo. **Não validado em máquina híbrida.**

## A placa durante a partida

Esta é a pergunta que o Otimiza não conseguia responder: **"a minha placa
estava sendo limitada por temperatura enquanto eu jogava?"**. A temperatura só
era lida quando você clicava — com o jogo fechado e a placa já fria, que é
justamente quando ela não diz nada.

Agora o Otimiza amostra a placa **na mesma janela** em que mede os quadros, e
mostra o resultado em dois lugares: no **Mapa de desempenho**, logo abaixo do
mapa, e na **ficha do jogo**, junto da última partida medida. Uma frase por
caso:

- *"A placa foi limitada por temperatura em 23% desta partida (máx. 84 °C,
  1650 MHz, 120 W de 125 W)."* — isso é físico, e nenhum ajuste de Windows
  resolve: poeira, ventoinha, fluxo de ar.
- *"A placa acionou o freio de hardware em 8% desta partida."* — fonte,
  conector de energia ou proteção da placa.
- *"Ela passou 92% do tempo no teto de energia."* — o funcionamento **normal**
  de uma placa em carga máxima, e a tela diz isso em vez de vender como
  defeito.
- Driver que não informa o motivo vira "não deu para ler", nunca "placa livre".

O dado vem da biblioteca oficial do driver (NVML), lida dentro do próprio
programa: nada de abrir processo a cada amostra. A pergunta cara — o motivo de
o clock estar segurado — custa 11 ms nesta máquina e por isso é feita uma vez
por segundo, enquanto o resto é lido a cada 200 ms. O painel de temperatura
também passou a usar esse caminho: 73 ms contra 135 ms de antes.

## Placa de vídeo e temperatura

- **Temperatura e limites, num lugar só:** o diagnóstico do processador
  agora mostra também a placa de vídeo NVIDIA — temperatura, clock, potência
  e o motivo de ela estar segurando o clock. Só temperatura e freio de
  hardware viram alerta; teto de energia em carga é o normal de qualquer
  placa, e a tela diz isso.
- **Driver de vídeo:** quem publicou o driver instalado. Se for o genérico do
  Windows, o aviso diz que instalar o do fabricante muda FPS de verdade; nos
  outros casos, que mais novo não é automaticamente mais rápido. Botão para a
  página do fabricante. O Otimiza não instala driver.

## Relatório de alterações

Além do PDF, **uma planilha** com cada alteração: data, ajuste, o que mudou,
o valor de antes (o que volta no desfazer) e o valor novo.

## Uma limpeza só, e energia com um dono por ajuste

- O liberador de espaço e a Limpeza do sistema passam a medir e apagar as
  mesmas pastas pelo mesmo código. **Conserto:** a categoria de entrega
  otimizada do liberador apontava para a fila de downloads do BITS, e apagar
  ali descartava downloads pendentes do Windows. O "Esvaziar Lixeira" do
  liberador saiu — a Lixeira é item da Limpeza, desmarcada.
- O plano de energia OTIMIZA cuida do que vale igual em qualquer máquina
  (disco, Wi-Fi, multimídia, preferência de placa). O que é processador é do
  motor de energia, que mede antes de escolher — o plano não escreve mais a
  receita fixa por cima dele.

## O que saiu, porque não mudava nada ou atrapalhava

A 2.9 começou por uma auditoria de tudo o que o Otimiza altera no Windows
(`docs/AUDITORIA-2.9.md`). Menos ajustes, e melhores.

- **Vinte ajustes** — os dezoito abaixo e as duas limpezas do catálogo
  (temporários e cache do Windows Update), que moram na Limpeza do sistema.
  O catálogo inteiro agora tem desfazer.
- **Dezoito deles:** Nove sem efeito em jogo: Nagle, serviços do Xbox, as
  duas de telemetria, Mapas, Sincronização, Assistência Remota, busca na
  internet do menu Iniciar e Copilot. E nove que ou não mudam nada no Windows
  10/11 atual ou pioram: SystemResponsiveness, prioridade de primeiro plano,
  MMCSS de jogos, desligar o SysMain, desligar a compressão de memória,
  desligar o controle de energia por processo (desligava o modo econômico que
  o novo modo jogo usa), notificações, e as duas limpezas de temporários e de
  cache do Windows Update. Quem já aplicou continua podendo desfazer; o motivo
  de cada um está em "o que o Otimiza não faz".
- **O botão "Priorizar" do FiveM e a prioridade alta fixa do jogo** — subir a
  prioridade às cegas faz o jogo disputar com o áudio e a entrada do mouse.
- **O plano de energia "máximo" sugerido pelo diagnóstico** — a energia é do
  motor de energia, que mede por jogo.
- **O painel de DNS** — o DNS não participa dos pacotes da partida.
- **Painéis repetidos** na aba de processos: uso, carga por núcleo, histórico
  e movimento já aparecem no topo e na aba Núcleos.

## Consertos

- Dois botões de conserto do diagnóstico chamavam uma otimização que não
  existe, e davam erro ao clicar.
- Numa máquina sadia, o diagnóstico dizia que o registro de eventos "não pôde
  ser lido" — era a falta de eventos sendo tratada como falha.
- Quando a leitura falhava, as telas de inicialização e de limite do
  processador diziam "nada encontrado" em verde. Agora dizem que não deu para
  conferir.

---

## O que esta versão não promete

**O ajuste de jogos Unreal foi provado em teste, não num jogo Unreal
instalado:** a máquina onde foi feito não tem nenhum. O formato vem da
documentação da Epic, e o ajustador recusa qualquer arquivo que não reconheça.

**O Roblox não ganhou ajuste gráfico.** Ele já vem no modo automático, que
baixa a qualidade sozinho para segurar o FPS — não há como provar que um
ajuste nosso renderia mais. O que o Otimiza aponta é o limite de 60 FPS.

**Os ajustes de Windows, somados, valem alguns por cento.** O que move o número
de verdade é a configuração do jogo, o driver e o gargalo da sua máquina.

**O "editor desconhecido" continua aparecendo.** O aviso do SmartScreen é o
Windows dizendo, com razão, que não sabe quem publicou este instalador.
