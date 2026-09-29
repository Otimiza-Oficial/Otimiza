Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza-instalador.exe.sha256` | A soma do instalador, para conferir que o arquivo chegou inteiro |
| `Otimiza_3.2.0_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_3.2.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão.

---

# A 3.2 prova no seu jogo o que muda

## Otimizar, passo a passo

A aba Otimizar agora é um caminho em cinco passos: o que trava este PC, o
que vai mudar e por quê, aplicar, **provar no jogo** e o resultado. Cada
passo diz se já foi feito nesta máquina.

A prova é nova: com o jogo aberto, o Otimiza troca sozinho entre o plano de
energia que você tinha e o dele, oito rodadas intercaladas, e compara o FPS
do jogo e os 1% piores quadros. Se o dele piorar o seu jogo, ele é
desfeito. Se a diferença ficar dentro do ruído, a tela diz isso — sem
inventar ganho.

## O FPS medido é o do jogo

As medições agora usam o **PresentMon** (Intel, código aberto), que separa o
quadro que o jogo desenhou do quadro gerado por Lossless, AFMF ou pelo
gerador do Otimiza. A tela mostra os dois lado a lado e nunca soma um no
outro. Também diz o que limitou o FPS na partida (processador, placa de
vídeo, limite de FPS ou V-Sync) e quanto o quadro levou para chegar à tela.

Se a configuração gráfica do jogo mudou entre o antes e o depois, a
comparação avisa: ganho de qualidade menor não é otimização.

**No FiveM, as versões anteriores mostravam o dobro do FPS real.** O
medidor antigo contava todo quadro que o processo manda para a tela, e o
FiveM manda dois por quadro do jogo (medido: exatamente 2× em oito
rodadas). A 3.2 conta só os quadros do jogo. Se o número caiu pela metade
depois de atualizar, o jogo não piorou: ele sempre foi esse. Por isso as
partidas medidas antes não são comparadas com as de agora, e o ajuste que
ficou sem essa comparação é avisado na área Otimizar.

## Cinco áreas no lugar de nove abas

Início, Otimizar, Jogos, Sistema e Histórico. Geração de quadros e Placa de
vídeo ficam dentro de Jogos; Energia, Núcleos e BIOS, dentro de Sistema. O
Início abre dizendo o que trava o FPS na última partida medida.

## Por que o jogo crashou

Em Jogos, o Otimiza lê os relatórios de crash que o próprio FiveM grava e
mostra, para cada um: o erro, a assinatura, a memória no momento, o veículo
e **o que o Otimiza mudou nas 48 horas antes** — ou que não mudou nada. Quando
o mesmo defeito se repete, quando o limite do jogo estourou ou quando o crash
já acontecia antes do Otimiza, a tela diz.

## Por que o jogo trava

No Início, **Investigar travadas**: com o jogo aberto, o Otimiza mede 60
segundos dos quadros e do que a máquina fazia no mesmo relógio, e procura o
que aparece **mais nas travadas do que fora delas** — um programa disputando
o processador, o disco, a memória, a memória de vídeo. A tela mostra a conta
e também a evidência contra. Com poucas travadas, ou sem nada que se destaque,
ela diz "causa ainda não determinada" em vez de chutar.

Quando o suspeito é um programa, dá para **testar**: até 12 minutos jogando,
com o programa em prioridade baixa em metade do tempo, alternando. Ele só fica
assim se as travadas caírem pelo menos 30% sem o FPS cair, e só até o jogo
fechar. Se não funcionar, o Otimiza lembra e não repete o teste.

## O Início conta a sua última partida

- **Sua última partida:** FPS, os piores momentos, os trancos e o que limitou,
  juntando as medições que o Otimiza já faz sozinho.
- **Por que seu jogo roda assim:** até três causas, cada uma com o número que
  a sustenta, e o passo a passo do que ajuda — lembrando o que já piorou nesta
  máquina para não oferecer de novo.
- **O que esperar deste PC:** na primeira abertura, os limites do hardware
  (monitor, memória, núcleos, disco, notebook) antes de otimizar qualquer coisa.

## O que mudou no seu jogo

Em Histórico, uma linha do tempo por jogo: partidas medidas, troca de driver,
atualização do Windows e o que o Otimiza aplicou ou desfez. Se o FPS caiu
depois de uma troca, a tela aponta quando. E se uma atualização do Windows
desfizer um ajuste do Otimiza, ele avisa em vez de fingir que continua valendo.

## Desfazer tudo, de verdade

**Restaurar meu PC** desfaz tudo o que o Otimiza mudou — histórico, plano de
energia, núcleos por jogo e os modos automáticos — e diz o que não conseguiu.
O desinstalador oferece o mesmo antes de remover o programa.

## Resultado para compartilhar

O resultado da prova no jogo sai num texto pronto para colar no Discord, com a
margem de erro junto — nunca só o número bom.

## Números mais honestos

- **"Antes e depois" não confunde medidor:** uma medição pelo PresentMon e
  outra pelo medidor antigo não viram mais "ganho confirmado".
- **O Mapa de desempenho mede pelo PresentMon**, e não mais pelo medidor antigo
  que contava o dobro no FiveM.

## Interface mais leve

Transições mais curtas, janelas que devolvem o foco ao fechar, e a animação do
Início para quando a janela está escondida ou fora da tela. Com "reduzir
movimento" ligado no Windows, nada se mexe — mesmo mudando com o Otimiza aberto.

## Mais cuidado com a sua máquina

- **A aba BIOS não fecha mais o programa.** A leitura do firmware travava a
  janela por até dois minutos.
- **Modo seguro:** se o Otimiza fechar sem querer duas vezes seguidas, o que
  ele faz sozinho fica parado até você religar, com um aviso na tela.
- **O gerador de quadros mede o próprio jogo** para decidir se tira FPS, e na
  comparação sai da frente de verdade.
- **Perfil do jogo** não grava mais tela cheia exclusiva para quem usa
  gerador de quadros (Lossless ou o do Otimiza), que pararia de gerar.
- **Planos de energia:** desfazer o modo jogo das versões antigas não apaga
  mais o plano do Otimiza, e desfazer os dois devolve o plano que você usava
  antes de tudo.
- O limite de FPS "para G-Sync" só é oferecido quando o monitor anuncia taxa
  variável. Sem ela, ele só tiraria FPS.

## O que saiu do catálogo

Doze ajustes de aparência e de limpeza do Windows, sem efeito medido no
jogo: efeitos visuais, transparência, atraso de inicialização, hibernação,
apps em segundo plano, otimização de entrega, indexação, apps patrocinados,
widgets, download automático da Loja, relatório de erros e Edge em segundo
plano. Os perfis "PC fraco" e "Trabalho" também saíram.

Quem aplicou algum deles numa versão antiga vê, na aba Otimizar, o botão
**Desfazer os aposentados**, que devolve cada um ao que era antes.

## Onde foi parar

- O diagnóstico para o atendimento está em **Sistema**, e agora conta também
  os crashes do jogo, a última partida e se o Otimiza caiu. Você vê o texto
  inteiro antes de mandar, e ele sai sem o seu nome de usuário.
- O que o Otimiza altera neste computador, e "quando foi que piorou", estão
  na área **Histórico**.
