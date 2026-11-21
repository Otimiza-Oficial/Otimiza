Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza-instalador.exe.sha256` | A soma do instalador, para conferir que o arquivo chegou inteiro |
| `Otimiza_4.0.13_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_4.0.13_x64_pt-BR.msi` | Para instalação em rede ou por política de empresa (agora em português) |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão. Se o Otimiza disser que a sua chave é do sistema de
licenças antigo, fale no suporte com o código da máquina: a chave nova sai sem
custo.

---

# 1.0.13: Temas de época com cena própria e o Oti enfeitado

Atualização da 1.0 (número interno 4.0.13).

## O que muda na tela

- **Temas de época de volta, ligados:** Natal, Ano Novo, Carnaval, Páscoa, Festa Junina, Dia das Crianças e Halloween
  voltam a enfeitar o Otimiza na data certa. Se preferir sem eles, é só desligar em Ajustes › Temas de época.
- **Cada época com a sua cena:** um fundo desenhado para cada uma (pinheiros com neve no Natal, fogos no Ano Novo,
  confete no Carnaval, fogueira e balões na Festa Junina, abóboras acesas no Halloween e assim por diante), com
  versão de dia no tema claro. O fundo é uma imagem parada: não pesa no PC, e as animações param com o jogo aberto ou
  com a opção do Windows de reduzir animações.
- **O Oti enfeitado:** em cada época o Oti ganha o seu enfeite (gorro, cartola, máscara, orelhas, chapéu de palha, boné
  ou chapéu de bruxa), sem cobrir os olhos.
- **A tela acompanha a época:** a saudação do Início, a luz atrás do Oti e a marca da aba em que você está ganham a cor
  da época, sempre com contraste conferido para continuar legível.
- **Prévia das épocas:** em Ajustes › Temas de época › "Ver uma época agora" dá para ver qualquer época na hora, sem
  esperar a data. A prévia vale até fechar o Otimiza.
- **Primeira execução com a história e o fundo:** quem instala pela primeira vez volta a ver o "Como eu trabalho" e a
  escolher o fundo (Escuro, Claro ou Sem fundo), com a barra de progresso mostrando as seis etapas.
- **Botão do Discord azul:** o botão que leva ao Discord para deixar a sua opinião voltou
  a ser azul, a cor do Discord.

Esta versão muda só a aparência: os ajustes do Windows, o Frame Boost e as medições continuam os mesmos da 1.0.12.
Não precisa fazer nada: é só atualizar.

---

# 1.0.12: Visual novo com o Oti, telas redesenhadas e correções de estabilidade

Atualização da 1.0 (número interno 4.0.12).

## O que muda na tela

- **Visual novo com o Oti:** o Otimiza inteiro ganhou a identidade nova, com o Oti redesenhado e o ícone novo do app.
  Fundo escuro (ou claro, se você preferir), cartões sólidos e cor só onde ela quer dizer alguma coisa: o estado de
  cada item.
- **Telas redesenhadas:** Início, Otimizar agora, Prova, Frame Boost, Jogos e Ativar licença foram refeitas. No Frame
  Boost, a tela agora mostra o multiplicador de verdade (o FPS na tela dividido pelo FPS do jogo), por que ele está
  assim e o que fazer agora.
- **Primeira execução guiada:** quem instala pela primeira vez é levado passo a passo: ativar a licença, medir o PC
  antes, aplicar o primeiro ajuste e provar o resultado.
- **Mensagens de erro que dizem o que fazer:** quando algo dá errado, o aviso diz o que aconteceu e o próximo passo,
  em vez de um texto técnico.
- **Acessibilidade:** dá para navegar pelas abas com as setas do teclado, o contraste das cores foi conferido para
  ficar legível, nenhum texto fica menor que 12 px, e o Otimiza respeita o modo de Alto Contraste e a opção do Windows
  de reduzir animações.
- **Os temas de época agora vêm desligados.** Quem quiser pode ligar nos Ajustes.

## Correções

- **Mais leve com a janela escondida:** com o Otimiza minimizado ou na bandeja, a tela não fica mais perguntando ao
  Windows a cada poucos segundos se há um jogo aberto. Ela é avisada quando o jogo abre e só volta a ler tudo quando
  você abre a janela.
- **"Desfazer" que podia sumir:** duas mudanças gravadas ao mesmo tempo podiam apagar parte do Histórico. Agora o
  Histórico é gravado de um jeito só, e os arquivos que o Otimiza usa para se recuperar de uma queda de energia não
  ficam pela metade.
- **Teto de FPS:** o limite de FPS que você colocou (no Otimiza ou no app da NVIDIA) agora vence o "Jogo liso".
- **Prioridade do jogo:** o Otimiza parou de regravar a prioridade dos jogos no registro do Windows a cada vez que abre.

Se você foi afetado por algum desses problemas, não precisa fazer nada: é só atualizar. Se quiser conferir um ajuste
antigo, ele continua no Histórico, de onde dá para desfazer.

## Licença

O Otimiza passa a ter licença própria, e o instalador mostra o contrato de uso (em português) antes de instalar. A sua
chave continua valendo do mesmo jeito.

---

# 1.0.11: Correções de bugs no Frame Boost: liso de verdade e mais leve na placa

Atualização da 1.0 (número interno 4.0.11).

## Correções

- **Frame Boost irregular ("o Auto trava", "o 2× deixa o jogo igual"):** para ficar liso, todo gerador de quadros
  precisa do FPS do jogo travado numa fração do monitor (por exemplo 72 num monitor de 144 Hz). Sem o Otimiza aberto
  como administrador, essa trava não acontecia, e o Frame Boost gerava num ritmo irregular. Agora ele só liga com a
  trava valendo: ele mede o jogo por 15 s (com o jogo na frente), trava o FPS no driver da NVIDIA e gera o resto num
  ritmo regular. Se não der para travar, ele não liga e diz o que fazer (abrir como administrador, reabrir o jogo).
- **Jogo perdendo FPS com o Frame Boost:** o gerador ficou bem mais leve para a placa de vídeo. Numa GTX 1650, cada
  quadro gerado passou a custar cerca de um quarto do que custava.

Para usar o Frame Boost: abra o Otimiza como administrador (botão direito → Executar como administrador). Se ele pedir
para reabrir o jogo, é só fechar e abrir: a trava do FPS só vale com o jogo reaberto.

---

# 1.0.10: Correção urgente: Frame Boost travando na 1.0.9

Atualização da 1.0 (número interno 4.0.10).

## Correções

- **Frame Boost travando desde a 1.0.9:** com o FPS alto na tela, o jogo parecia rodar a 30 e a imagem falhava ao
  virar a câmera. A 1.0.9 esperava cada atualização do monitor antes de mandar a imagem: quando a placa estava ocupada
  com o jogo, uma imagem atrasada perdia duas atualizações em vez de uma. Agora o Otimiza pergunta ao Windows quando é
  a próxima atualização e prepara a imagem para ela, sem nunca ficar parado esperando. A economia de placa da 1.0.9
  continua.

Se você usa o Frame Boost e ficou com a 1.0.9, atualize. Se ainda notar travada ou falha na imagem, mande o registro
por **Ajustes → Registro do Otimiza → Salvar em Downloads**.

---

# 1.0.9: Correções de bugs no Frame Boost: a geração que se vê, sem pesar na placa

Atualização da 1.0 (número interno 4.0.9). Feita a partir do registro de um testador com GTX 1650 e monitor de 144 Hz
jogando FiveM.

## Correções

- **Frame Boost gastando a placa à toa:** no Auto (o modo Equilíbrio), o Otimiza desenhava bem mais quadros do que o
  monitor consegue mostrar, e a placa ficava quase toda ocupada com quadros que nunca apareciam. Agora sai no máximo
  um quadro por atualização do monitor, e a placa fica livre para o jogo.
- **2×, 3× e 4× mostrando menos do que prometiam:** os modos fixos passam a acompanhar o monitor, como o Auto. O 2×
  deixa de virar quase 1× na tela. Com o jogo já perto dos Hz do monitor, o Frame Boost não gera quadros que não
  caberiam (antes só somava atraso).
- **"Auto (agora 1×)" com o Frame Boost gerando:** a tela agora mostra quantos quadros chegam à tela para cada quadro
  do jogo.
- **Frame Boost desligando cedo no Equilíbrio:** antes de desligar, ele agora alivia a placa em degraus (modo leve,
  depois um quadro gerado por quadro do jogo). Ele também não aceita mais, calado, o jogo perdendo mais de 5% do FPS
  real: alivia ou desliga e diz por quê.
- **Prioridade fixada do jogo:** passa de "alta" para "acima do normal". A alta podia picotar o som e pesar o mouse no
  carregamento. Quem já tinha fixado é ajustado sozinho na próxima abertura do Otimiza (com administrador), e o
  Histórico continua desfazendo para o que havia antes.
- **Otimiza pesando durante a partida:** a tela do Frame Boost não fica mais perguntando ao Otimiza com a janela
  escondida, e o contador de FPS por cima do jogo lê as suas escolhas uma vez por segundo, não dez.
- **Provas que comparavam coisas diferentes:** durante uma prova (plano de energia, Turbo, teste da travada), o modo
  jogo, a energia de jogo e a afinidade não mexem mais em nada. O resultado compara só o que está sendo provado.
- **Desfazer que podia sumir:** ao ligar o Frame Boost pelo "Jogar", o registro do Smooth Motion e do AFMF podia ser
  apagado por outra parte do Otimiza gravando ao mesmo tempo. E sete arquivos de estado agora são gravados inteiros ou
  não são trocados, mesmo se a energia cair no meio.

Se o Frame Boost ainda parecer pouco diferente, mande o registro por **Ajustes → Registro do Otimiza → Salvar em
Downloads**.

---

# 1.0.8: Correções de bugs no Frame Boost: menos atraso nos modos 2×, 3× e 4×

Atualização da 1.0 (número interno 4.0.8).

## Correções

- **Atraso nos modos 2×, 3× e 4×:** a imagem do Frame Boost podia esperar até 3 quadros na fila do Windows antes de aparecer, e o jogo parecia "pesado" na mão. Agora espera no máximo 1, como já acontecia no Auto. Quando um quadro gerado chegaria atrasado, ele fica de fora; o quadro do jogo sempre aparece.
- **Atraso mostrado na tela:** o número de atraso do Frame Boost agora inclui também o tempo até a imagem chegar à tela, quando o Otimiza consegue medir esse trecho. Por isso ele pode aparecer um pouco maior que na 1.0.7. Continua marcado como estimado.
- **Registro para o suporte:** o registro do Otimiza agora traz quantos quadros gerados ficaram de fora e quanto tempo a imagem levou até a tela. Se notar o Frame Boost menos liso, mande o registro por **Ajustes → Registro do Otimiza → Salvar em Downloads**.

---

# 1.0.7: Correções de bugs no Frame Boost: menos desligamentos, menos atraso e uma tela mais simples

Atualização da 1.0 (número interno 4.0.7).

## Correções

- **Frame Boost desligando sozinho:** nos modos fixos (2×, 3×, 4×), quando o jogo começava a perder FPS, ele desligava de vez. Agora alivia a placa em degraus: primeiro o modo leve, depois um quadro gerado a menos. Só desliga se nada disso bastar, e a tela diz o que fez e por quê. No Auto ele alivia para o modo leve e só desliga se o jogo continuar perdendo FPS.
- **Tranco da conferência:** de tempos em tempos o Frame Boost pausa para conferir se o jogo perde FPS. A pausa ficou mais curta (1 s em vez de 1,5 s).
- **Atraso com placa NVIDIA:** junto com o Frame Boost, a fila de quadros do driver fica em 1 no perfil do jogo (o "Modo de baixa latência" do painel da NVIDIA). Vale a partir da próxima vez que o jogo abrir. Fica no Histórico e sai junto com o Frame Boost.
- **Borrão em volta da mira:** o fundo que passa atrás da mira e da HUD sai um pouco mais certo nos quadros gerados.
- **Frame Boost que não ligava em alguns notebooks e drivers:** quando a captura de sempre não começa, o Otimiza agora tenta a captura moderna do Windows antes de desistir.
- **Gravações pela metade:** se a energia caía na hora de gravar o estado do Otimiza (o teto de FPS do jogo, a base do Frame Boost), o arquivo podia ficar pela metade. Agora ele é gravado inteiro ou não é trocado.
- **Testes de energia e prova do Turbo:** só começam com a marca de recuperação gravada. Se o PC desligar no meio, a abertura seguinte desfaz o que ficou pela metade.

## Novo

- **Frame Boost em três modos:** Qualidade, Equilíbrio (o padrão de quem nunca escolheu) e Desempenho. O Equilíbrio usa o cálculo de movimento mais leve, com menos trabalho para a placa. As opções de antes ficam em "Avançado", e quem já tinha escolhido continua com a escolha.

---

# 1.0.6: Correções de bugs no Turbo, no Frame Boost e para quem tem Valorant instalado

Atualização da 1.0 (número interno 4.0.6).

## Correções

- **"Nenhum gerador funciona aqui" com o Valorant instalado:** o anticheat do Valorant (Vanguard) roda desde que o Windows liga, e o Otimiza achava que ele estava em partida. Com isso o Frame Boost e a mira sumiam em qualquer jogo, até no FiveM. Agora só o Valorant aberto bloqueia. Quando nenhum gerador serve, a tela diz o motivo.
- **Jogo preso em 72 FPS de versões antigas:** quem usou o Jogo liso, o Turbo ou o Frame Boost antes da 1.0.5 podia ter ficado com o jogo travado na metade do monitor (por exemplo, 72 FPS num monitor de 144 Hz, com o jogo rodando a 130). Agora, ao fechar o jogo, o Otimiza desfaz esse teto sozinho. O teto que você travou na mão fica.
- **Receita do Turbo guardada antes da 1.0.6:** com o processador no limite e o jogo perto dos Hz do monitor, ela ligava o Frame Boost a cada "Jogar". Agora o Turbo deixa a geração de fora nesse caso, e a energia de jogo continua.
- **Turbo** usa o FPS de costume do jogo, das suas últimas partidas, e não só os 10 segundos da medição: um trecho ruim não vira a receita da noite.
- **Travadinhas a cada 5 minutos:** o Otimiza conferia de novo, a cada 5 minutos, se o jogo aberto era mesmo um jogo, e essa consulta pesava mais de 1 segundo de processador. Agora o jogo confirmado não é conferido de novo até fechar. A pausa de conferência do Frame Boost também fica mais espaçada (a cada 15 minutos) quando ele já provou que não tira FPS na partida.
- **Frame Boost:** a parte que mostra os quadros gerados passa na frente quando o processador está no limite. Antes ela perdia a vez para o jogo e o quadro gerado saía atrasado (micro-tranco).
- **Jogo liso com G-Sync ou FreeSync:** quando o jogo passa dos Hz do monitor, o teto fica 3 abaixo (141 num monitor de 144 Hz), para a taxa variável continuar valendo.
- **Energia:** com o motor de energia cuidando de um jogo (aba Energia), a energia de jogo não entra junto. Os dois trocavam o plano ao mesmo tempo no começo da partida.
- **Overlay da Steam** não é mais rebaixado pelo modo jogo.
- **Prova do Turbo:** quando a diferença não dá para provar, a tela mostra de quanto a quanto as rodadas variaram, em vez de dizer "sem diferença" ao lado de um FPS maior.

---

# 1.0.5: Correções de bugs no Frame Boost, no Turbo, no desfazer e no "Otimizar agora"

Atualização da 1.0 (número interno 4.0.5).

## Correções

### FPS
- **Turbo e Frame Boost:** com o jogo já perto dos Hz do monitor (por exemplo, FiveM a 130 FPS num monitor de 144 Hz), o Frame Boost travava o jogo na metade do monitor, em 72 FPS reais, e completava com quadros gerados. O contador mostrava 144, mas o jogo ficava pesado na mão. Agora, perto dos Hz, o FPS real fica como está; o Turbo só liga a geração quando o jogo está bem abaixo do monitor.
- **Frame Boost:** depois de desligar (por você ou sozinho), o jogo podia continuar travado no FPS base dele, por exemplo 72 FPS, sem nenhum quadro gerado, partida após partida. Agora o teto do Frame Boost sai junto com ele.
- **Frame Boost e Jogo liso** não brigam mais pelo mesmo teto de FPS. Com o Frame Boost ligado, vale o teto dele, e o Jogo liso explica por quê. Quando o Frame Boost sai, o teto do Jogo liso que você tinha volta. O teto que você trava na mão nunca é trocado.
- **Jogo liso** mostra quem pôs o teto que está valendo: o Frame Boost, o Jogo liso ou você.
- **Frame Boost:** quando ele desliga sozinho porque a placa não aguenta, a tela diz quanto FPS real o jogo mantinha, e ele não volta sozinho naquele jogo no "Jogar".
- **Plano de energia OTIMIZA nos Ryzen 7900X3D, 7950X3D, 9900X3D e 9950X3D:** o plano mantinha todos os núcleos acordados, e o jogo se espalhava para o bloco sem o cache 3D. Agora o plano deixa esse ajuste como a AMD recomenda. Se você tem um desses processadores e já aplicou o plano, desfaça "Plano de energia OTIMIZA" no Histórico: nesses processadores a AMD recomenda o plano Equilibrado do próprio Windows.

### Desfazer e segurança
- **Desfazer tudo** voltava as mudanças na ordem errada e podia deixar a configuração do GTA/FiveM com uma mudança do Otimiza. Agora desfaz do mais novo para o mais antigo.
- Quando um ajuste falhava no meio e a volta automática também falhava, o que tinha mudado ficava sem registro. Agora vai para o Histórico, com Desfazer.
- **Plano de energia:** se o Otimiza não conseguia ler qual plano estava ativo, ele ativava o OTIMIZA mesmo assim, sem ter para onde voltar. Agora não muda nada nesse caso.
- **Limites de boot:** cada limite removido fica anotado na hora, para voltar mesmo se o seguinte falhar.
- **Driver da NVIDIA:** se o Otimiza não conseguia ler o valor que você tinha escolhido, o desfazer devolvia o padrão de fábrica por cima dele. Agora ele não grava sem saber o que desfazer.
- **Placa de rede:** "Impedir que a placa de rede durma" desligava junto o Wake-on-LAN (ligar o PC pela rede). Agora só impede a placa de dormir. Se você usa Wake-on-LAN e já tinha aplicado, desfaça esse item no Histórico e aplique de novo.
- **Sistema → Limpeza:** "Limpar" agora pergunta antes de apagar.
- A remoção de apps da Microsoft Store só aceita os apps da lista do Otimiza.
- **Prova do Turbo:** quando ela para no meio, a tela diz por que e o que fazer.
- **Histórico:** ligar e desligar o Frame Boost podia apagar ou trazer de volta itens do Histórico. Corrigido.
- **Energia, cores e modo jogo:** três casos em que o Otimiza tentava desfazer algo que não existia mais (um plano de energia apagado, um monitor trocado, uma anotação corrompida) e falhava em toda abertura. Agora ele percebe e segue em frente.

### "Otimizar agora" (Completo)
- Saíram do lote três ajustes de registro que não mudam nada no Windows 10 e 11 atuais (prioridade do sistema para jogos, prioridade do primeiro plano e perfil de multimídia). Continuam na lista, um a um, para quem quiser.
- Saiu do lote o ajuste dos navegadores, que fazia o Edge e o Chrome mostrarem "gerenciado pela sua organização". Se o seu navegador mostra esse aviso, desfaça "Impedir os navegadores de ficar abertos por trás" no Histórico.
- Um ajuste que deve ser aplicado sozinho (otimizações para jogos em janela) não entra mais no lote.

### Textos e telas
- Várias mensagens tinham um buraco de espaços no meio da frase, como a de ativar a chave. Corrigido.
- No tema claro, os textos em verde e em laranja ficaram mais legíveis.
- Telas que ficavam "procurando" para sempre quando a leitura falhava agora dizem o que aconteceu e deixam tentar de novo.

## Novo

- **Catálogo de jogos:** FiveM, Valorant, CS2, Fortnite e Apex, cada um com a configuração leve, a limpeza do cache de shader e o que o anticheat deixa fazer. Na aba Jogos, escolha o jogo e abra **Otimizar**.
- **Página do jogo** em abas: Resumo, Otimizar, Ferramentas (crashes do FiveM e integração do CS2) e Histórico.
- Toda ação que apaga ou muda o PC pede confirmação dentro do Otimiza.
- **Ajustes → Registro do Otimiza → Salvar em Downloads:** salva o registro do que o Otimiza fez num arquivo só, sem o seu nome de usuário, para mandar ao suporte quando algo der errado.

---

# 1.0.4: Correções de bugs no Jogo liso e no Turbo

Atualização da 1.0 (número interno 4.0.4).

## Correções

- **Jogo liso** não trava mais o jogo na metade do monitor. Em monitor de 144 Hz, um jogo que rodava a 140 FPS podia ser travado em 72. Agora o teto nunca tira mais que 15% do FPS, e a tela mostra quanto ele tira antes de você travar.
- **Jogo liso** usa o FPS de costume do jogo, tirado das suas últimas partidas, e não só os 20 segundos da medição. Um momento ruim, como cidade cheia ou cenário carregando, não vira o teto da noite inteira.
- **Travar em X fps** grava exatamente o número que aparece na tela. Antes ele media de novo e podia gravar outro.
- **Energia de jogo e prova do Turbo:** quem trocou ou apagou o plano de energia do Windows ficava com a energia de jogo travada e a prova do Turbo parando no meio. Corrigido.

Se o seu jogo ficou mais pesado depois de usar o Jogo liso, abra **Histórico** e desfaça o "limitado a ... FPS", ou meça e trave de novo nesta versão.

---

# 1.0.3: Valorant, CS2 e correções de bugs

Atualização da 1.0 (número interno 4.0.3). Além do FiveM, o Otimiza agora otimiza forte o Valorant e o CS2, com ferramentas de mira que a comunidade pediu. E corrigimos o Frame Boost que parava sozinho depois de um tempo.

## Correções

- O **Frame Boost** não para mais sozinho depois de alguns minutos com a placa de vídeo no limite. Ele desligava quando o jogo perdia um pouco de FPS real, mesmo com a tela ganhando quase o dobro de quadros. Agora, no modo leve, só desliga se o jogo perder mais de 25%, quando o mouse começa a pesar de verdade.

## Novo: catálogo do Valorant e do CS2

Em **Jogos**, escolha o Valorant ou o CS2 e veja o painel de cada um:

- **Configuração do jogo:** o perfil leve tira o que pesa na placa sem tirar o que mostra o inimigo. No Valorant: sombras, pós-processamento e efeitos. No CS2: sombras, oclusão, partículas e detalhe de shader, com a sincronia vertical desligada e o NVIDIA Reflex ligado. Só muda o que já existe no arquivo do jogo, com o jogo fechado, e o **Desfazer** devolve o arquivo exatamente como era.
- **Cache de shader:** limpa o cache do driver e, no CS2, o da Steam. É ele que causa as travadinhas depois de atualizar o driver ou o jogo.
- **Windows e anticheat:** o que o Otimiza faz em cada jogo. No Valorant, o Vanguard não deixa nada desenhar por cima do jogo: o Otimiza respeita, e o ganho vem da configuração, do cache e do Turbo.

## Novo: ferramentas de mira

Na área **Mira**:

- **Copiar para o Valorant:** a mira que você desenha vira um código de perfil. No Valorant: Configurações > Mira > Importar perfil.
- **Treino de flick:** alvos em lugares sorteados por 30 segundos, com acertos, precisão e tempo de reação. O treino de tracking continua lá.
- **Sensibilidade nos dois sentidos:** digite a do CS2 ou a do Valorant, e o Otimiza mostra a outra e os centímetros por volta.
- **Treino no CS2:** duas configs de treino na pasta do jogo. `exec otimiza_treino` mostra a trajetória da granada, com munição e dinheiro infinitos. `exec otimiza_mira` põe 8 bots parados que renascem, para treinar flick. Só funcionam num servidor seu.

---

# 1.0.2: Correções de bugs no Frame Boost

Atualização de correções da 1.0 (número interno 4.0.2). Vocês avisaram que o jogo não estava mais ficando liso com o Frame Boost depois da 1.0.1, e era verdade. Corrigimos e deixamos ele ainda melhor.

## Correções

- A **escala** do Frame Boost voltou a funcionar para todos, como na 1.0. A 1.0.1 tinha desligado ela, e em placa de vídeo fraca é ela que dá FPS de verdade (o jogo desenha menos pixels e o Otimiza amplia; o monitor não muda). Vale de novo no Frame Boost, no Turbo e no **Jogar**. Quem não quiser desliga em Ajustes, em **Escala do Frame Boost**.
- No **Auto**, CS2, Valorant e os outros competitivos voltaram a encher o monitor, em vez de ficar presos em 2×.
- O Frame Boost não desliga mais sozinho quando o jogo perde só um pouquinho de FPS: no modo leve, ele só desliga se o jogo perder mais de 10%.

## Melhorias

- Com a placa de vídeo no limite, o Frame Boost avisa antes de ligar e oferece a escala **Qualidade** num toque: FPS real a mais, e o gerador completa o monitor sem roubar do jogo.
- Cabeçalho das áreas mais limpo, sem o ladrilho colorido.

---

# 1.0.1: Correções de bugs e acabamento

Atualização de correções da 1.0 (número interno 4.0.1). Arrumamos o que vocês reportaram e aproveitamos para dar acabamento à interface. Tudo que o Otimiza muda continua com o valor de antes guardado e o desfazer.

## Correções

- O **Frame Boost** não muda mais a resolução do jogo sem você permitir (opção **Permitir a escala** nos Ajustes). A prova do Turbo também respeita a escala desligada.
- As capas de CS2, Fortnite, Roblox e Valorant não somem mais sem internet: aparece o logo do jogo.
- Na apresentação, a ficha do PC ficava branca com letra branca no tema escuro.
- O menu do **Fundo** abria atrás do varal de luzes e do painel.
- O anel de prontidão da Visão geral tinha um quadrado atrás.
- Os enfeites das épocas do ano não caem mais por cima do texto, e param com o jogo aberto.

## Acabamento da interface

- Todas as abas reorganizadas: cabeçalho com ícone colorido, cada seção no seu cartão e menos parede de texto.
- **Visão geral** com a faixa "Pronto pra jogar" e o processador ao vivo.
- **Jogos** com cara de página de jogo; **Tela** mostra quanto do monitor está em uso (ex.: 60 de 180 Hz); **Histórico** em linha do tempo; **Turbo** com os níveis em cartões e barra de progresso.
- **Mira** e **Cores** com a prévia em cenas reais de FiveM.
- **Fundo:** Aurora, Brasa, Oceano, Neon, Cidade e Morro, ou **uma imagem sua**, com desfoque e escurecer ajustáveis. A imagem fica só no seu PC.
- Natal, Ano Novo, Carnaval e as outras épocas agora mudam o app inteiro.

## Ajustes finos

- **Frame Boost:** no Auto, CS2, Valorant e outros competitivos usam 2× (menos atraso). O jogo mostra o que já foi provado nele, com o botão **Usar de novo**.
- **Turbo por jogo:** escolha **Menor atraso** ou **Mais FPS** direto na página do jogo.
- **Rede:** a nota da fila (bufferbloat) em destaque, e o "quando piora" se enche sozinho com o ping das suas partidas.

---

# 1.0 Nova Geração (4.0.0): o Otimiza de cara nova, e o jogo mais liso

A Nova Geração recomeça a contagem: esta é a **1.0** (o número interno continua subindo, 4.0.0). Interface nova
inteira, e cada coisa que o Otimiza muda continua com o valor de antes e o desfazer.

## Interface nova

- **Quatro abas:** Jogar, Meu PC, Prova e Ajustes, no trilho da esquerda, com as áreas de cada uma embaixo.
- **Jogar** em um toque: o Otimiza abre o jogo (Steam, Epic, Roblox ou o executável) e já liga a receita que este PC
  provou para ele. Na primeira partida sem receita, ele mede o jogo sozinho e guarda a receita para a próxima.
- **Cartão da partida** quando o jogo fecha: FPS médio, 1% pior, trancos por minuto e se a rede ficou firme. Com
  tranco e placa NVIDIA, o botão **Deixar liso** grava um teto de FPS estável para a próxima vez.
- **Gráfico da partida** na aba Prova, com cada medição sobre a faixa das suas partidas anteriores.
- Apresentação nova na primeira abertura, com a ativação da licença à vista; claro, escuro e **sem fundo** (a área de
  trabalho aparece entre os cards).

## Mais FPS e jogo mais liso

- **Frame Boost:** no Auto, escolha o FPS na tela (60 a 240): o gerador gasta menos placa e o jogo fica com mais FPS
  de verdade. Nitidez da escala em três níveis, que muda na hora. O Frame Boost que você usar e a guarda aprovar volta
  sozinho no próximo "Jogar".
- **Nada que trave o PC:** o Otimiza não deixa mais serviço do Windows parado; ajuste que piora os trancos é desfeito
  na primeira partida; PC que cai, dá tela azul ou reinicia o driver de vídeo depois de um ajuste tem o ajuste desfeito
  na abertura seguinte; a temperatura é vigiada por meia hora depois de um ajuste que esquenta.
- Cada ajuste do Sistema mostra **o que pode dar errado** e em que tipo de PC.
- Em Windows modificado ("lite"), ajuste de serviço que a imagem removeu não é gravado.

## Rede

- **Ping com a internet ocupada:** mede o ping parado, baixando e enviando, e diz se é fila no modem (bufferbloat) e
  o que fazer.
- **Quem está usando a internet**, por programa, em 5 segundos.
- **Testar a placa de rede:** moderação de interrupção, Ethernet de economia e controle de fluxo, testados no seu
  ping; só fica o que melhorou.
- **Servidores do FiveM:** cole os links cfx.re/join e veja o ping de cada um, do mais perto para o mais longe.
- **Canal do Wi-Fi** recomendado pelas redes vizinhas, e o jogo com **prioridade na rede** (experimento, depende do
  roteador).

## Segurança

Toda versão confere a licença com o servidor e respeita chave revogada; a janela tem política de segurança (CSP).

A 1.0 traz também tudo da 3.6 e da 3.5 abaixo.

---

# 3.6 Turbo Máximo (3.6.0): o Turbo junta tudo

## Até o monitor, quando quem segura é o jogo

Quando nada está no limite (nem placa, nem processador, nem memória) e o FPS fica abaixo dos Hz do monitor, quem segura
é o próprio jogo: limite dele, do servidor ou do motor, comum no FiveM. Diminuir a resolução ou soltar o processador
não ajuda aí. Agora o Turbo liga a geração de quadros até encher o monitor, e o cartão diz com todas as letras que é FPS
**na tela**, não FPS do jogo. Com "Menor atraso", sem a medida do FPS ou com o monitor quase cheio, ele continua sem
ligar nada.

## Perfil gráfico do FiveM pelo Turbo

O perfil da configuração do GTA no FiveM (MSAA, sombras, reflexos, grama), que só existia na tela clássica, agora está
no cartão do Turbo com o FiveM aberto: **Máximo de FPS** ou **Equilibrado**. Como o jogo reescreve a configuração ao
sair, o pedido fica guardado e é aplicado quando o FiveM fecha, com o arquivo de antes no histórico para desfazer. O
cartão mostra o que está pedido (com Cancelar) e o que já está aplicado. No GTA V fora do FiveM, o perfil continua pelo
botão da tela clássica.

## Mais certinho ao desligar

- O teto de FPS que o Turbo põe na base é tirado quando você desliga o Turbo. O teto que você mesmo pôs pelo Jogo liso
  fica como está.
- A oferta de ritmo estável some quando o Turbo desliga.

A 3.6 também traz tudo da 3.5 abaixo, inclusive a licença reforçada.

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
