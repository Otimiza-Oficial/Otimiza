<!--
  Rascunho das notas da 3.2. No dia da publicação, com ordem do dono, isto substitui
  .github/release-notes.md (o arquivo que o release.yml lê). Conferir antes os nomes dos
  arquivos com o número da versão, e tirar deste rascunho o que não tiver sido testado.
-->

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
