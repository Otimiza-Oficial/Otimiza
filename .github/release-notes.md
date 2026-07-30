Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.3.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.3.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

Boa parte do que o Otimiza faz mexe em configurações protegidas e precisa de
administrador. O programa avisa quando isso acontece e reabre com permissão, se
você deixar — nunca por conta própria.

## Esta versão é sobre PC fraco

Cinco sistemas novos, todos pensados para a máquina que mais precisa de ajuda.

**Liberador de espaço.** Em PC fraco, disco cheio é o problema que mais se
disfarça de "PC lento": abaixo de 10% livre o Windows perde folga e a culpa cai
no processador. Varre categoria por categoria — temporários, instaladores de
atualização, relatórios de erro, registros — mostrando quanto cada uma ocupa e
explicando o que é.

**Memória e paginação.** Em PC de 4 a 8 GB, quase todo "o computador congela" é
memória acabando. O culpado mais comum é alguém ter desativado o arquivo de
paginação seguindo tutorial ruim — o que não ganha desempenho e faz programa
fechar sozinho. O Otimiza detecta e corrige num clique.

**Detector de conflitos.** PC lento raramente é culpa de um programa só: é de
dois fazendo a mesma coisa. Dois antivírus varrendo um ao outro. Três
sobreposições injetando código no mesmo jogo. Dois otimizadores desfazendo a
configuração um do outro. Ele mostra o conflito com nome e sobrenome.

**Auditor de tarefas agendadas.** O Windows executa dezenas de tarefas em
segundo plano em horários que ninguém escolheu. Mostra as que chegaram com
programas instalados e permite desligar — reversível, como tudo aqui.

**Detector de programas de fábrica.** Notebook de loja chega com utilitário do
fabricante, antivírus em teste e joguinho patrocinado. Aqui a regra de segurança
vem primeiro: driver, runtime e biblioteca de sistema **nunca** são marcados,
aconteça o que acontecer.

## Sete otimizações novas

| Otimização | O que faz |
|---|---|
| Liberar o Armazenamento Reservado | Devolve de 7 a 10 GB que o Windows guarda só para atualizações |
| Impedir instalação automática de apps | Sem isso, o que você desinstalar hoje volta na próxima atualização |
| Remover relógio de plataforma forçado | Conserta uma das "dicas de FPS" mais repetidas e mais erradas da internet |
| Perfil de multimídia para jogos | Prioridade de processador, vídeo e disco para o jogo em primeiro plano |
| Desligar Widgets | Tira o painel de notícias que carrega conteúdo em segundo plano |
| Desligar o Copilot | Libera o processo que fica pronto esperando |
| Fixar a coleta de dados no mínimo | Faz a desativação da telemetria sobreviver às atualizações |

São **35 otimizações** no total, e o Otimiza marca quais **pesam na sua máquina**
em particular — pouca memória, disco mecânico ou poucos núcleos mudam o que vale
a pena.

## O que o Otimiza não faz

Existe uma lista de coisas que rendem desempenho e que nós **recusamos** fazer.
Ela está dentro do programa, na aba Otimizações:

- Desativar as proteções da CPU contra Spectre e Meltdown
- Desligar Windows Update, Defender ou firewall
- "Limpeza de registro", que não tem ganho medível e quebra programa instalado
- Liberar memória à força, o que deixa o gráfico bonito e o PC mais lento
- Escrever na BIOS — em placa de consumo, errar ali inutiliza a placa-mãe

## Como conferir o resultado

Na aba **Resultado**: meça antes, otimize, meça depois. Se o ganho não aparecer,
o programa vai dizer isso. Ele se recusa a emitir veredito com o PC ocupado, e
duas das medições aparecem marcadas como "só referência" porque oscilam demais
sozinhas para provar qualquer coisa.

## Tudo é reversível

Cada mudança grava o valor anterior antes de escrever. "Desfazer tudo" restaura o
que existia, não algo parecido — inclusive programas de inicialização e tarefas
agendadas. As duas exceções são apagar arquivos e limpar o cache de
atualizações, ambas marcadas como **sem volta** e fora do "Otimizar agora".

## Ainda não há versão para macOS e Linux

O motor de otimização é todo específico do Windows. Um instalador para as outras
plataformas hoje abriria um programa sem nenhuma função — preferimos não publicar
a publicar algo que não entrega.
