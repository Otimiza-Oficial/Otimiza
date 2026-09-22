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

## O que saiu, porque não mudava nada

- **Nove ajustes:** Nagle, serviços do Xbox, as duas de telemetria, Mapas,
  Sincronização, Assistência Remota, busca na internet do menu Iniciar e
  Copilot. Nenhum muda FPS nem fluidez. Quem já aplicou continua podendo
  desfazer; o motivo de cada um está em "o que o Otimiza não faz".
- **O painel de DNS** — o DNS não participa dos pacotes da partida.
- **Prioridade alta fixa do jogo** — só dá para remover o que já foi fixado.

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
