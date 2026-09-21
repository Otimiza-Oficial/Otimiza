Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_2.8.0_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_2.8.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão.

---

# A 2.8 mede de onde vem cada número, e mostra isso na tela

Esta versão mexeu em duas coisas ao mesmo tempo: **o que o Otimiza sabe sobre
a sua máquina**, e **como ele conta isso para você**.

## Toda medida diz de onde veio

Cada número na tela passou a carregar a própria procedência: **medido**,
**estimado** ou **não foi possível ler**. Não é detalhe de engenharia — é a
diferença entre um painel que informa e um que enfeita.

A regra que sustenta o resto: **"0 ms" nunca aparece sem medição real.** Onde
não deu para medir, a tela diz que não deu para medir, e diz o motivo. Um
número que você não pode conferir vale menos que um espaço em branco honesto.

## Sete leituras novas, cada uma com uma recusa

- **Memória de vídeo.** O que aperta o jogo não é a VRAM dedicada estar cheia —
  o driver usa aquilo como cache de propósito. O sinal de verdade é o derrame
  para a memória compartilhada, e é isso que o painel passou a mostrar.
- **Orçamento de latência**, em cinco etapas do caminho até a tela. O que ele
  entrega é um **piso**, e está escrito que é um piso: o Otimiza não chama
  aquilo de "a sua latência".
- **Laboratório de carregamento.** Engasgo ao carregar cenário não é concluído
  da configuração ("seu jogo está num HD"), e sim de coincidência medida entre
  a travada e o que o disco estava fazendo naquele instante.
- **Orquestrador de renderização**, que **se recusa** a sugerir ajuste de placa
  quando o gargalo medido é o processador, o firmware ou o disco.
- **Autoajuste**, cujo desfecho padrão é **voltar atrás**: provou que melhorou,
  fica; provou que piorou, volta; **não provou nada, também volta.**
- **Histórico de desempenho**, comparando as duas medições mais recentes — e
  não contra o seu melhor resultado de sempre, que é um extremo escolhido a
  dedo e faz qualquer dia normal parecer uma piora.
- **Caminho do mouse**, que aponta aceleração ligada e barra fora do meio — e
  **não mede a sua mira** nem instala gancho de teclado ou mouse.

As comparações antes/depois passaram a usar intervalo de confiança. A conclusão
deixou de ser "a diferença passou de 3%" e virou "os intervalos se sobrepõem,
então não dá para afirmar que mudou".

## A interface inteira foi refeita

Lateral só de ícones, cabeçalho sem repetir o que a lateral já diz, e os
ajustes como **lista**, no formato das configurações do Windows, em vez de uma
parede de cartões.

A mudança que mais importa é pequena de ver e grande de usar: **o que cada
ajuste faz de verdade saiu do bloco que só abria no clique e virou a descrição
da linha.** Aquele texto é o argumento inteiro deste produto, e estava
escondido atrás de um clique que quase ninguém dá.

Junto vieram as capas de verdade dos jogos na biblioteca, lidas do cache da
própria Steam, e a logo acompanhando o tema claro e escuro.

## Ajustes: dois saíram, dois entraram

**Saíram "desligar o UAC" e "desligar o firewall".**

Os dois cobravam segurança de verdade e devolviam **zero** — zero quadro por
segundo, zero milissegundo de ping. O UAC não roda durante o jogo, e o filtro
de rede do Windows decide sobre cada pacote em microssegundos, contra dezenas
de milissegundos de caminho até o servidor. Trocar segurança por desempenho já
é uma decisão pesada; trocar segurança por nada não é decisão.

Eles não sumiram: viraram item da lista **"o que o Otimiza não faz, e por
quê"**, com o fato escrito. Quem vier de outro programa vai achar a explicação
no lugar onde procuraria o botão. Na mesma lista entraram as duas promessas de
afinidade que rendem vídeo e não rendem quadro: o serviço que fixa núcleo
sozinho, e empurrar programa de fundo para os núcleos que o jogo não usa.

**Entraram dois que não prometem FPS — e dizem isso na primeira linha.**

- **Relatório de erros do Windows.** Quando um jogo fecha sozinho, o Windows
  sobe o coletor de falhas, que segura a janela travada copiando gigabytes de
  memória para o disco. É por isso que a queda parece muito pior do que foi.
  O preço de desligar está escrito: você perde o registro daquela falha.
- **Edge carregando no boot e vivo de janela fechada.** São dois
  comportamentos ligados de fábrica que quase ninguém sabe que existem. O
  preço também está escrito, e ele aparece na cara: o Edge passa a mostrar
  "gerenciado pela sua organização" nas configurações dele.

**Se você já aplicou o UAC ou o firewall numa versão anterior, o desfazer
continua funcionando.** O histórico de alterações guarda o valor anterior de
cada mudança e não depende de o ajuste continuar na lista.

## Uma trava nova no código

Ajuste que troca segurança e declara ganho nulo agora **derruba o teste**. Não
é promessa de que não vai acontecer de novo: é o repositório se recusando a
compilar um botão que cobra e não paga.

---

## O que esta versão não promete

**A aba Biblioteca não está pronta, e avisa isso no topo.** Achar e reconhecer
os seus jogos funciona. O ajuste *dentro* de cada jogo, por enquanto, só existe
de verdade para GTA V e FiveM — nos outros títulos a ficha é informação, não
botão. Fica assim de propósito, em vez de mostrar controle que não faz o que
promete.

**Os ajustes de Windows, somados, valem alguns por cento.** Eles não são o
caminho para dobrar o seu FPS, e nunca foram. O que move o número de verdade é
a configuração do jogo, o driver e o gargalo da sua máquina — as abas Jogos,
Placa de vídeo e Energia.

**O "editor desconhecido" continua aparecendo.** O aviso do SmartScreen é o
Windows dizendo, com razão, que não sabe quem publicou este instalador.
