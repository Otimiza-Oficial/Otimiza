# PC Performance Optimizer — Estado real do projeto

> Este documento registra apenas o que foi **verificado**. Funcionalidade que não
> foi executada e conferida aparece como pendente, mesmo que o código exista.

## Verificado

| Item | Como foi verificado |
|---|---|
| Backend Rust compila | `cargo check` e `cargo build` sem erros |
| 707 testes unitários passam, zero avisos | `cargo test --lib` nesta máquina: 707 passaram, 0 falharam, 14 ignorados |
| O executável final compila com 11 avisos | `cargo build --release` — caminho diferente do teste. Os 11 são código sem uso, todos de `nvdriver.rs`, o módulo NVIDIA que ainda não tem tela |
| Janela nasce cabendo na tela | Programa da 1.9 aberto nesta máquina: **1283 × 818** numa tela de 1920×1080, centralizada. Era 1440×900 fixo |
| Detector escolhe o processo do jogo, não o subprocesso | Com o FiveM aberto aqui: `FiveM_b3258_GTAProcess.exe`, pelos quatro sinais (janela em primeiro plano, motor 3D a 58%, aberto há 759 s, nome conhecido). A varredura por lista devolvia `FiveM_ChromeBrowser` |
| Instalador gerado | `Otimiza_0.3.0_x64-setup.exe` e `.msi`, compilados pela esteira do GitHub |
| Monitor de processos funciona nesta máquina | Discord ×6 · 9,2% da CPU · 1019 MB · marcado como inicialização |
| Ciclo real de inicialização restaura bytes idênticos | Desligou e religou o Discord; bytes conferidos com PowerShell, fora do nosso código |
| Esteira do GitHub compila e testa em máquina limpa | Oito áreas de teste verdes; instaladores anexados ao release |
| Liberador de espaço varre esta máquina | 971 MB recuperáveis, por categoria |
| Diagnóstico de memória acha problema real aqui | 11,3 GB prometidos para 7,9 GB físicos |
| Detector de conflitos acha problema real aqui | Driver Booster 13, entre 253 programas examinados |
| Auditor de tarefas lê o agendador desta máquina | 203 tarefas, 17 de terceiros |
| Detector de fábrica não marca driver como lixo | Teste com 7 drivers reais (NVIDIA, Realtek, Intel, AMD, Visual C++, .NET, DirectX) |
| Ícone da barra de tarefas é a nossa logo | Extraído do `.exe` compilado e conferido visualmente |
| Perfil de hardware desta máquina | SSD, 7,9 GB de RAM, 8 núcleos lógicos |
| Benchmark produz números reais | Executado nesta máquina: 669 Mops/s em 1 núcleo, 4450 em todos, 3600 MHz sob carga |
| **Aplicar e desfazer funcionam de verdade** | Ciclo completo executado contra o registro do Windows; restauração conferida com PowerShell, fora do nosso código |
| Limiares de ruído são medidos, não chutados | `noise_calibration` mede a mesma máquina 3× sem mudar nada |
| Não reporta ruído como ganho | `null_test` mede 2× sem mudar nada e falha se algum indicador acusar melhora |
| Frontend compila | `npx tsc --noEmit` e `npm run build` |
| Leitura de serviços independe do idioma | Teste lê `RpcSs` do registro nesta máquina |
| Parsing do `powercfg` funciona em português | Conferido contra a saída real deste Windows |
| Identificador desta máquina é estável e único | `OTZ-WPYY-0J4F-77AB`, tirado do número de série da placa-mãe. `ProcessorId` foi medido (`BFEBFBFF000A0653`) e RECUSADO: é igual em todo processador do mesmo modelo |
| Chave de licença assinada é aceita | Chave emitida para o ID desta máquina ativou o produto |
| Chave adulterada é recusada | Um caractere trocado nos dados, e outro na assinatura; os dois recusados. Chave assinada por outro par também |
| Chave de outra máquina não ativa aqui | Emitida para `OTZ-AAAA-BBBB-CCCC`, recusada nesta máquina |
| Sem licença, o produto fica trancado | Arquivo apagado; `ativa: false` |
| Caminho da taxa do monitor funciona no driver | Ensaio com `CDS_TEST` nos dois AOC 24G4: driver aprovou 144 Hz e 120 Hz sem nada ser alterado |
| Chaves de registro do Firewall batem com o `netsh` | Os três perfis conferidos contra `netsh advfirewall show allprofiles state` |
| Chave pública de produção instalada | Conferida como Ed25519 de 32 bytes antes de entrar no código |
| Bot do dono emite chave que o produto aceita | `/licenca` carregado no bot real (8 comandos); chave assinada em JavaScript conferida pelo Rust |
| Metade de par trocada é recusada na emissão | Privada de um par com pública de outro: o emissor recusa antes de a chave sair |

## O que existe hoje

### Diagnóstico e monitoramento
- Detecção de plataforma (Windows/Linux/macOS)
- Análise de CPU, RAM, disco e detecção de GPU
- Health score 0–100 a partir dos gargalos encontrados
- Métricas em tempo real na interface, atualizando a cada 2s

### Otimizações (Windows)
Catálogo em `src-tauri/src/modules/windows/catalog.rs`, todas reversíveis:

| Otimização | Ganho declarado |
|---|---|
| Plano de energia Alto Desempenho | Mensurável |
| Desativar Game DVR | Mensurável |
| Agendamento de GPU por hardware | Situacional (exige reiniciar) |
| Prioridade do sistema para jogos | Situacional (exige reiniciar) |
| Desativar telemetria (DiagTrack) | Resposta do sistema |
| Efeitos visuais para desempenho | Resposta do sistema |
| Remover atraso de inicialização | Resposta do sistema |
| Rede de baixa latência (Nagle) | Situacional (exige reiniciar) |
| Desativar aceleração do mouse | Resposta do sistema |
| Prioridade para o programa em primeiro plano | Situacional (exige reiniciar) |
| Desativar serviços do Xbox | Resposta do sistema |
| Desativar SysMain (SuperFetch) | Resposta do sistema |
| Desativar hibernação | Situacional |
| Limpar arquivos temporários | Resposta do sistema — **irreversível** |

**O que pesa NESTA máquina.** Cada otimização declara em que tipo de hardware ela
vale muito mais que a média — pouca RAM, disco mecânico, poucos núcleos. O
produto cruza isso com o hardware detectado e marca as que importam para aquele
PC, subindo-as na lista.

Não é promessa de milagre. É reconhecer que desligar efeito visual muda pouco num
PC forte e muda muito num de 4 GB, e dizer isso ao cliente em vez de entregar a
mesma lista de vinte itens para todo mundo.

**Otimizações para máquina modesta:**

| Otimização | O que ataca |
|---|---|
| Desligar indexação de busca | O indexador lê disco e gasta CPU sem hora marcada — pesa em HD e em CPU fraca |
| Desligar transparência das janelas | O efeito é redesenhado a cada quadro pela GPU; em vídeo integrado aparece |
| Limpar instaladores de atualizações | A limpeza que mais devolve espaço, geralmente vários GB |

**Otimizações de hardware** — exigem descobrir o dispositivo no registro, porque
o identificador muda de PC para PC:

| Otimização | O que ataca |
|---|---|
| Interrupções diretas da placa de vídeo (MSI) | A GPU avisa a CPU por fila própria em vez de disputar uma compartilhada. É latência, não FPS médio |
| Impedir que a placa de rede durma | O Windows desliga a placa e o primeiro pacote depois atrasa — causa real de pico de ping |

O filtro de placa de rede foi o que mais deu trabalho acertar: a classe de rede do
Windows lista WAN Miniports de VPN, adaptadores do Hyper-V e o do depurador de
kernel junto com as placas reais. O primeiro teste pegou **onze** dispositivos,
dez deles virtuais. O `ComponentId` separa os dois mundos — físico começa com o
barramento (`pci\`, `usb\`).

**Otimizações de consumo de fundo:**

| Otimização | O que ataca |
|---|---|
| Desligar aplicativos em segundo plano | Apps da Store rodando sem você usar |
| Tirar a busca na internet do menu Iniciar | O menu espera resposta da web para achar programa já instalado |
| Parar de compartilhar atualizações | O Windows usa sua banda de subida para servir outras máquinas |

**Otimizações profundas** (as que separam o produto de uma lista de tweaks
copiada da internet):

| Otimização | O que ataca |
|---|---|
| Desligar estacionamento de núcleos | Engasgo: acordar um núcleo adormecido custa milissegundos |
| Estado mínimo do processador em 100% | Atraso de a CPU subir de frequência |
| Desligar limitação de energia por processo | Windows classificando errado o que está em uso |
| Desligar compressão de memória | CPU gasta comprimindo RAM (só com RAM sobrando) |
| Liberar limites de inicialização | Núcleos ou RAM limitados no boot por mexida no msconfig |
| Desligar virtualização de segurança (VBS) | Camada de virtualização que custa FPS — **reduz segurança** |

**"Otimizar agora" nunca inclui o que troca segurança por desempenho.** O VBS dá
ganho real e mensurável, mas protege suas senhas do Windows contra roubo. Abrir
mão disso é decisão consciente do dono do PC, então exige clique no item, com o
aviso em vermelho na frente. Marcado no catálogo por `security_tradeoff` e
travado por teste.

Estas mexem no agendador de energia do processador — as opções que o Windows
esconde do painel de controle. São lidas e revertidas pelo registro, não pela
saída traduzida do `powercfg`.

**Perfil de hardware manda no catálogo.** O produto lê o tipo do disco, a RAM e o
número de núcleos antes de oferecer qualquer coisa, e recusa o que faria mal:

- *Desativar SysMain* não é oferecido em HD mecânico — lá o serviço ajuda
- *Desligar compressão de memória* não é oferecido abaixo de 12 GB de RAM
- Quando o tipo do disco não é identificado, a resposta é "não oferecemos":
  preferimos perder a venda a chutar

**Detecção de estado real.** Antes de oferecer qualquer coisa, o programa lê o
sistema e classifica cada otimização em: *disponível*, *já otimizado*,
*aplicada por nós* ou *não se aplica a esta máquina*. Um PC que já está
configurado não recebe oferta de "otimização" — é o oposto do truque de cobrar
por serviço que não foi executado.

**"Otimizar agora" exclui deliberadamente:** o que já está aplicado, o que a
máquina já tinha, e tudo que for irreversível. Apagar arquivo nunca acontece por
um clique genérico.

Cada entrada declara na interface o **ganho real esperado**, incluindo os casos em
que o ganho é zero. Nenhuma toca em Windows Update, Defender, firewall ou drivers
de núcleo — protegido por teste automatizado e pelo `SafetyValidator`.

### Prova de resultado (o diferencial)
Módulo em `src-tauri/src/modules/benchmark.rs`. Mede seis indicadores antes e
depois de otimizar:

| Indicador | Por que importa |
|---|---|
| **Travada no pior caso** | **O engasgo em si — a medida que nenhum concorrente tem** |
| **Engasgos por minuto** | **Pausas maiores que um quadro** |
| Desempenho de 1 núcleo | Manda na maioria dos jogos |
| Desempenho de todos os núcleos | Renderização, compilação, compactação |
| Frequência da CPU sob carga | É onde o plano de energia aparece |
| CPU consumida em segundo plano | Cai quando serviços inúteis param |
| RAM ocupada em segundo plano | Menos travadas por falta de memória |
| Processos em execução | Menos disputa por CPU e disco |

**Só 4 dos 6 indicadores geram veredito.** Os outros dois aparecem marcados como
"só referência", porque a calibração provou que eles não sustentam conclusão:

- *CPU em segundo plano* variou **244%** entre medições sem nada ter mudado
- *Desempenho de todos os núcleos* marcou **-19%** no teste nulo: carga em todos
  os núcleos esquenta a CPU, e a segunda medição sempre começa mais quente que a
  primeira. É viés sistemático, não ruído que mais repetições resolvam

**O medidor de engasgos** (`modules/jitter.rs`) é o diferencial de medição.
Ninguém reclama de "média de FPS baixa" — reclama que o jogo *trava por um
segundo*. Um congelamento de 40 ms arruína a suavidade e quase não mexe na média
de 60 quadros por segundo, então média nenhuma mostra isso. Medimos direto: pedir
para dormir 1 ms, 1500 vezes, e ver quanto o Windows realmente demorou para
acordar. O atraso é o que o jogador sente.

Regras que impedem número inflado:
- Cada carga roda 5 vezes e usa o **melhor** resultado, não a média
- Uso ocioso usa a **mediana**, com 2s de acomodação antes de amostrar
- Limiares vêm da **calibração medida** (1 núcleo 15%, RAM 8%, processos 5%,
  frequência 3%), não de chute
- **Piso absoluto além do percentual**: travada saindo de 1,0 ms para 1,4 ms é 40%
  de variação e não significa nada. Só conta acima de 3 ms de diferença
- Sair de zero (0 engasgos → 12) é julgado pelo sentido da diferença, senão a pior
  regressão possível ficaria escondida atrás de uma divisão por zero
- **Se o PC estiver ocupado (>25% de CPU), nenhum veredito é emitido.** Comparar
  um PC ocupado com um PC descansado gera ganho fantasma de dezenas por cento —
  foi observado na prática: +28,9% "de melhora" que era só o PC ter descansado
- O resumo pode dizer *"Nenhuma diferença mensurável"* ou *"Recomendamos desfazer"*
- O baseline fica em disco, então otimizações que exigem reiniciar continuam mensuráveis

### Rede de segurança — ponto de restauração do Windows

Camada abaixo do nosso histórico: serve para o que não previmos.

**Não confiamos no comando do Windows.** `Checkpoint-Computer` falha em silêncio em
duas situações comuns, e um produto que anuncia "criamos um ponto de restauração"
sem verificar está vendendo segurança que não existe:

- A Proteção do Sistema vem **desligada** em muitas instalações do Windows 10 e 11
- O Windows recusa criar mais de um ponto a cada 24 horas

Então contamos os pontos antes e depois e confirmamos que um novo apareceu —
independente do idioma do Windows. Cada falha vem com o motivo provável explicado,
não um "erro" genérico.

**Sem administrador o Windows nega até a leitura da lista.** Uma lista vazia nesse
caso não prova que a proteção está desligada — prova que não conseguimos olhar. O
app diz isso, e há teste travando essa distinção.

Achado na máquina de desenvolvimento: **nenhum ponto de restauração, proteção
desligada**. Era exatamente a armadilha prevista.

O "Otimizar agora" tenta criar um ponto antes de mexer em qualquer coisa, informa
o resultado real no registro ao vivo, e segue mesmo se não der — porque o
histórico já reverte item por item.

### Publicação de versões

`.github/workflows/release.yml` compila em máquina limpa do GitHub ao empurrar
uma tag `v*`, roda os testes antes de empacotar e anexa os instaladores ao
release. Versão que não passa nos testes não vira instalador.

A descrição fica em `.github/release-notes.md`, versionada como qualquer outro
texto do produto — passa por revisão, não fica escondida dentro do YAML.

O release sai como **rascunho**: a descrição é conferida antes de ficar visível.
Para publicar direto, trocar `releaseDraft` para `false` no workflow.

**Os testes rodam separados por área** porque o log detalhado de uma execução só
é acessível com autenticação. Dividido em passos, o próprio painel do GitHub
aponta onde quebrou. E sem `continue-on-error`: com ele o resultado do passo é
reescrito para "sucesso" e o painel passa a mentir sobre o que aconteceu.

A primeira execução em máquina limpa encontrou um teste ruim: ele rejeitava
adaptador de rede cujo nome contivesse "virtual", e os runners são máquinas
virtuais da Azure, onde a placa se chama "Mellanox ConnectX Virtual Ethernet
Adapter" — dispositivo PCI de verdade. O filtro do código estava certo; o teste
é que conferia o nome em vez do critério. Agora confere o `ComponentId`.

**macOS e Linux estão desligados de propósito.** Todo o motor de otimização é
exclusivo do Windows: são 27 pontos de código com `cfg(windows)`, e fora dele
cada comando responde "não implementado". Publicar esses instaladores entregaria
um programa que abre, mostra painéis vazios e não faz nada, carregando a marca.
Os blocos estão prontos no workflow, comentados, aguardando motor próprio.

### Instalador

`npm run tauri build` produz dois pacotes prontos para distribuir:

| Pacote | Tamanho |
|---|---|
| `Otimiza_0.2.0_x64-setup.exe` (NSIS, instalador em português) | 2,2 MB |
| `Otimiza_0.2.0_x64_en-US.msi` | 3,3 MB |

A versão subiu para **0.2.0** porque a 0.1.0 já está na mão de clientes: sem
número novo não há como saber quem está com o quê quando alguém relatar problema.

Ficam em `src-tauri/target/release/bundle/`. O executável carrega a logo, e o
instalador NSIS está em português, instalando para o usuário atual.

### Gerenciador de inicialização

Liga e desliga programas que sobem com o Windows, escrevendo **no mesmo lugar que
o Gerenciador de Tarefas** (`StartupApproved\Run`): a entrada do cliente nunca é
apagada, só marcada como desligada.

- Exigiu suporte a `REG_BINARY` no módulo de registro — o Windows guarda esse
  estado em 12 bytes, com `0x02` habilitado e `0x03` desabilitado
- Reversão **byte-exata**: o Windows grava a data/hora do desligamento nos bytes
  4 a 11, e restaurar "equivalente" deixaria rastro nosso no registro do cliente.
  Teste verifica que o ciclo desligar→ligar devolve os bytes idênticos
- Entradas de HKLM valem para todos os usuários e pedem elevação, com o mesmo
  diálogo de administrador
- Entram no histórico com id próprio (`startup:HKCU:Discord`), então "Desfazer
  tudo" também devolve a inicialização ao estado original

### Sete otimizações da 0.3.0

| Otimização | O que faz |
|---|---|
| Liberar o Armazenamento Reservado | Devolve de 7 a 10 GB guardados só para atualizações |
| Impedir instalação automática de apps | Sem isso, o que se desinstala hoje volta na próxima atualização |
| Remover relógio de plataforma forçado (HPET) | Conserta uma das dicas de FPS mais repetidas e mais erradas |
| Perfil de multimídia para jogos | Prioridade de CPU, vídeo e disco para o jogo em primeiro plano |
| Desligar Widgets | Painel de notícias que carrega conteúdo em segundo plano |
| Desligar o Copilot | Processo que fica pronto esperando |
| Fixar a coleta de dados no mínimo | Faz a desativação da telemetria sobreviver a atualizações |

Duas delas não são ajustes, são **consertos**: remover o HPET forçado desfaz
estrago de tutorial ruim, e impedir a instalação automática é o que faz a limpeza
de bloatware durar mais que a próxima atualização grande.

**Correção de honestidade encontrada nesta rodada.** Três verificações dependem
de comandos que o Windows só responde com elevação — Armazenamento Reservado,
HPET forçado e limites de boot. Sem administrador a leitura voltava vazia, e o
código concluía "já otimizado" ou "não se aplica a esta máquina". Era afirmar o
que não foi verificado, exatamente o que este produto existe para não fazer.

Agora esses itens aparecem como disponíveis com o aviso *"só dá para conferir o
estado atual como administrador"*, e há teste travando a regra. O caso dos
limites de boot estava errado desde que foi escrito.

### Detector de programas de fábrica

Notebook de loja chega com vários GB de utilitário do fabricante, antivírus em
teste e joguinho patrocinado. É justamente a máquina mais fraca que vem com mais
peso morto — e o dono nem sabe que aquilo está lá.

**Aqui o risco é invertido.** No resto do projeto, errar significa não otimizar.
Aqui, errar significa marcar o driver de vídeo como lixo e o cliente
desinstalar. Por isso a ordem das regras é: **primeiro o que nunca pode ser
marcado**, depois o que pode.

A lista de proteção — driver, chipset, áudio, runtime, redistribuível,
framework, `.NET`, Visual C++, DirectX — é consultada antes de qualquer regra de
detecção, e vence sempre. Um driver de firewall da McAfee casaria com o padrão
"mcafee"; a proteção impede. Há teste com sete drivers reais garantindo isso.

**Nada é desinstalado às escondidas:**

- Aplicativo da Loja sai pela chamada do Windows e **volta pela Loja** quando o
  usuário quiser
- Programa comum **não é desinstalado por nós**: abre-se a tela oficial do
  Windows, porque desinstalador de fabricante faz perguntas e imitá-lo é o
  caminho para deixar instalação pela metade

O identificador do pacote entra num comando do PowerShell, então é validado
caractere a caractere antes — com teste tentando injetar comando.

**Tamanho desconhecido não vira zero.** Apps da Loja ficam em pasta protegida e o
tamanho não é legível; mostrar "0 MB" passaria a impressão de que não ocupam
espaço. O relatório diz "tamanho não informado" e o total avisa que é parcial.

### Detector de conflitos

O sistema que nenhum concorrente tem — e por um motivo simples: metade do que ele
denuncia são os próprios concorrentes.

PC lento raramente é culpa de um programa só. É de dois fazendo a mesma coisa ao
mesmo tempo. Detecta quatro brigas:

| Conflito | Por que custa caro |
|---|---|
| Dois antivírus com proteção em tempo real | Cada leitura de disco é verificada duas vezes, e um passa a inspecionar o outro |
| Outro otimizador instalado | Duas ferramentas desfazem a configuração uma da outra |
| Três ou mais sobreposições de jogo | Cada uma injeta código no mesmo ponto de entrada — causa conhecida de engasgo |
| Vários sincronizadores de nuvem | Cada um vigia pastas e lê disco continuamente |

**Não desinstala nada.** Desinstalar é decisão do dono da máquina, e
desinstalador de terceiro é interativo. O que se faz é mostrar o conflito com
nome e sobrenome.

Achado na máquina de desenvolvimento, entre 253 programas: **Driver Booster 13**
— que, aliás, é quem trocou o plano de energia desta máquina por um próprio.

O bit de proteção em tempo real quase passou errado: a primeira versão comparava
igualdade com `0x10` no byte do meio do `productState`, e falhava com `0x061100`,
que é o valor do próprio Defender **ativo**. Concluiria que um antivírus ligado
está desligado — e nunca apontaria o conflito. É teste de bit, não de igualdade.

### Auditor de tarefas agendadas

O Windows executa dezenas de tarefas em segundo plano em horários que ninguém
escolheu. As de terceiros — atualizadores, utilitários de fabricante — acordam
sozinhas o dia inteiro.

Lista **só as de terceiros**, com ligar/desligar reversível gravado no histórico.
Tarefas do próprio Windows são recusadas: desligar tarefa do sistema é da mesma
família de desligar serviço do sistema. O critério é o caminho `\Microsoft\`, que
é estrutura fixa em qualquer idioma — não o autor, que vem em branco ou traduzido.

Na máquina de desenvolvimento: **203 tarefas no total, 17 de terceiros**, entre
elas `IObit SUM2026Sale` e `iTopML SUM2026 Task` — tarefas de promoção de venda
agendadas no PC do cliente.

### Liberador de espaço

Em PC fraco, disco cheio é o problema que mais se disfarça de "PC lento". Abaixo
de 10% livre o Windows perde folga para o arquivo de paginação e para
atualizações — e a culpa cai no processador.

Varre **categoria por categoria**, mostrando quanto cada uma ocupa e explicando o
que ela é: temporários, instaladores de atualização, relatórios de erro, cache de
compartilhamento, registros de atualização e instalação anterior do Windows.

Duas regras:

- **O total recuperável só conta o que limpamos por aqui.** Somar o que não
  removemos seria prometer espaço que o usuário não vai ver
- **`Windows.old` é reportado, não removido.** A pasta pertence ao
  TrustedInstaller e resiste a remoção comum; apagar metade é pior que apontar a
  ferramenta certa. O produto mostra o tamanho e manda usar a Limpeza de Disco

Teste automatizado impede que alguém acrescente uma categoria apontando para
fora das pastas conhecidas — a barreira contra apagar algo que importa.

### Memória e arquivo de paginação

Em PC de 4 a 8 GB, a maioria dos travamentos que o dono descreve como "o PC
congela" é memória acabando, não falta de processador. O culpado mais comum é
alguém ter **desativado o arquivo de paginação** seguindo tutorial ruim — o que
não ganha desempenho nenhum e faz programa fechar sozinho.

Diagnostica e explica:

| Achado | Severidade |
|---|---|
| Paginação desativada com pouca RAM | Crítico — causa de "o programa fechou sozinho" |
| Paginação perto do limite já atingido | Importante |
| Tamanho fixo definido à mão | Importante |
| Memória prometida acima da física | Crítico — **aponta para hardware** |
| RAM abaixo do confortável | Importante — **aponta para hardware** |

Os dois últimos dizem explicitamente que nenhum ajuste de software cria memória.
A correção que existe — devolver o gerenciamento ao Windows — está a um clique.

As regras de diagnóstico ficam separadas da leitura do sistema, então são
testadas sem depender da máquina onde rodam.

### Transparência ao vivo

**Quem está pesando agora.** Painel em tempo real com os processos que mais
consomem, atualizando a cada 2s. Responde a pergunta que o cliente realmente faz
— "o que está deixando meu PC lento?" — apontando o programa pelo nome.

Duas correções que a maioria das ferramentas erra:
- O `sysinfo` reporta CPU relativa a **um núcleo**: um processo aparece com 380%
  numa máquina de 4 núcleos. Normalizamos pelo número de núcleos
- Programas modernos abrem vários processos com o mesmo nome. Somamos por nome:
  "Discord ×6: 9,2%" em vez de seis linhas de 1,5% que ninguém interpreta

Cada processo mostra também se **volta sozinho no próximo boot**, cruzando com as
chaves `Run` do registro.

**Registro ao vivo da otimização.** O Rust emite um evento por passo, e a
interface mostra cada mudança enquanto acontece — com o valor que existia antes:

```
✓ Desligar estacionamento de núcleos
  → energia · ajuste 0cc5b647 (antes: não existia)
✓ Plano de energia Alto Desempenho
  → plano de energia trocado
```

Barra de progresso é o que os concorrentes mostram porque não têm nada real para
exibir. Aqui o cliente acompanha em vez de confiar.

### Firmware e hardware — o que a BIOS está fazendo com o desempenho

**Não escrevemos na BIOS, e isso é decisão técnica, não falta de vontade.** Em
placas de consumo (ASUS, MSI, Gigabyte, ASRock) as configurações não ficam em
variáveis UEFI documentadas: ficam num bloco proprietário da NVRAM, com checksum
de cada fabricante e sem API pública. Só Dell, HP e Lenovo corporativos publicam
interface WMI. Escrever no lugar errado não derruba o Windows — inutiliza a placa.
Quem promete "otimizar sua BIOS" está mentindo ou brincando com o hardware do
cliente.

O que fazemos é ler, medir e dizer **onde** se resolve — software, BIOS ou troca
de peça:

| Verificação | Como é medida |
|---|---|
| Canal único de memória | Conta canais distintos, não pentes. Dois pentes no mesmo canal continuam sendo canal único |
| XMP/EXPO desligado | Compara velocidade real com a nominal do pente |
| Limites de boot do msconfig | Lê `numproc`, `truncatememory`, `removememory` |
| Estrangulamento térmico/energia | Mede a queda de trabalho entregue ao fim de 10s de carga |
| VBS ligado | Consulta o DeviceGuard pelo WMI |

Achado na máquina de desenvolvimento: **1 pente em 4 slots, canal único**.
Corrigir isso rende mais que todo o catálogo de software somado — e o produto diz
exatamente isso, mesmo não tendo como faturar em cima.

O medidor de estrangulamento não pergunta a frequência ao Windows (que reporta o
nominal, não o real): mede a **consequência**, comparando o trabalho entregue no
primeiro e nos últimos segundos de carga.

### Preferências

Três, e cada uma muda comportamento real. Interruptor que não altera nada é
enfeite, e enfeite numa ferramenta de sistema é o começo da desconfiança: se um
botão mente, por que os números não mentiriam?

| Preferência | Padrão | O que muda |
|---|---|---|
| Ponto de restauração antes de otimizar | ligado | O lote pula a criação, que leva dezenas de segundos |
| Mostrar o que não se aplica | ligado | Some da lista o que a máquina não comporta |
| Intervalo das medições | 2s | Ler mais rápido custa CPU do próprio programa |

Gravadas em `%APPDATA%\pc-optimizer\preferences.json`. Valor fora da faixa é
corrigido na leitura e na gravação — o arquivo pode ter sido editado à mão, e um
intervalo de zero ocuparia justamente a CPU que o programa deveria liberar.

### Interface em abas (0.2.0)

A tela única com dez painéis não escalava — e agora há usuários reais. Passou a
cinco abas, cada uma com uma pergunta própria:

| Aba | Responde |
|---|---|
| **Painel** | O que está acontecendo agora |
| **Otimizações** | O que dá para melhorar |
| **Diagnóstico** | O que está errado, e onde se resolve |
| **Resultado** | O que mudou de fato |
| **Sistema** | O que sobe com o Windows, e como voltar atrás |

Duas decisões sustentam a mudança:

- **Sinais vitais fixos no topo.** CPU, memória e disco ao vivo seguem visíveis em
  qualquer aba. Trocar de seção não pode custar o contato com a máquina — é o que
  separa um console de um formulário com páginas.
- **Selos numéricos nas abas.** "Otimizações 6", "Diagnóstico 2" em vermelho,
  "Sistema 9" em âmbar quando há mais de cinco programas na inicialização. A
  navegação carrega informação em vez de ser só rótulo.

Acessibilidade: `role="tablist"`, `aria-selected`, navegação por setas e foco
visível. Layout verificado em 1440, 960 e 900px (o mínimo da janela) sem estouro
horizontal.

### Identidade e interface
- Marca gerada por equação em `brand/make_logo.py`: um astroide de quatro pontas
  com um traço vazado que separa a folha. O mesmo número gera o SVG da interface
  e todos os ícones, então vetor e bitmap são exatamente a mesma curva
- Ícones do aplicativo regerados (barra de tarefas, janela, `.ico` com 7 resoluções)
- Paleta tirada da logo: branco quente `#f5f3f0` sobre preto `#0b0b0b`. Verde,
  âmbar e vermelho só aparecem onde carregam significado — carga e veredito
- Lista de otimizações agrupada por categoria e recolhível: cada item ocupa 43px
  em vez de 140px, e a lista rola dentro do painel em vez de esticar a página

### Pedido de elevação
O Windows não permite que um processo ganhe privilégio sozinho. Ao clicar numa
otimização que precisa de administrador, o programa explica isso e oferece
reabrir com permissão — o aviso do próprio Windows é quem decide. Recusar não
altera nada.

### Segurança e reversibilidade
- `ChangeLog` grava o valor anterior de cada alteração em `%APPDATA%\pc-optimizer\changes.json`
- Falha no meio de uma otimização desfaz o que já foi aplicado
- "Desfazer Tudo" restaura o estado original
- Otimizações que exigem administrador falham com mensagem clara em vez de silenciosamente

### Licença

Assinatura Ed25519 de chave pública. O dono assina com uma chave privada que
nunca sai da máquina dele; o programa carrega só a pública, que **só confere** e
não cria. Extraí-la do executável não permite forjar nada.

Sem servidor, a chave nasce presa à máquina: o identificador vem do número de
série da placa-mãe (sobrevive à formatação) com o `MachineGuid` do Windows como
reserva. Quem repassa a chave descobre que ela não abre no PC do outro.

O bloqueio mora no backend, não na tela. Vinte e dois comandos que alteram o
computador conferem a licença na primeira linha. Ler e desfazer continuam
livres, de propósito: o diagnóstico alimenta a tela de compra, e trancar o
"reverter" deixaria o PC de quem pagou alterado sem caminho de volta.

O emissor de chaves vive em `examples/`, que o `cargo build --release` não
compila — é a única peça que toca na chave privada e ela fica do lado de cá da
cerca. O manual está em [`docs/LICENCA.md`](docs/LICENCA.md).

A chave pública de produção foi instalada em 29/08/2026. A privada
correspondente está com o dono e no segredo do bot.

### O portão

Primeira tela de quem instala. Ocupa a janela inteira, some quando a licença é
aceita. À esquerda a compra, à direita a ativação. O diagnóstico roda por trás,
e o achado principal aparece na tela de compra — o problema real daquela
máquina, medido na hora, em vez de texto de propaganda.

## A varredura do `unwrap_or_default`, e o que ela achou de verdade

O levantamento inicial contou **67 leituras** em `modules/windows/` caindo em
`unwrap_or_default()` e apontou quatro módulos suspeitos pelo cruzamento "muitos
achados tranquilizadores + muitas leituras que podem falhar caladas".

Conferidos um a um, o placar foi:

| Módulo | Suspeita | Veredito |
|---|---|---|
| `health.rs` | 15 achados Ok · 4 leituras | **Defeito real** — a primeira tela afirmava a causa de uma leitura que nunca aconteceu |
| `firmware.rs` | 9 achados Ok · 1 leitura | **Defeito real** — memória ilegível virava silêncio, sem achado e sem lacuna |
| `suporte.rs` | 6 achados Ok · 3 leituras | **Correto.** O `unwrap_or_default` alimenta um `match` que trata o caso vazio explicitamente |
| `reparo.rs` | 10 achados Ok · 1 leitura | **Falso positivo.** O `unwrap_or_default` está DENTRO de um teste, sobre um campo de texto — não é leitura de sistema |

Dois de quatro. Vale registrar os dois que **não** eram defeito com o mesmo
destaque dos que eram: a contagem cruzada serve para escolher onde olhar, não
para concluir. Tratar o número como veredito teria "consertado" código correto.

**A regra que sai da 1.6 e da 1.7, e que vale para o que vier:** leitura de
sistema devolve `Option`. `Some(vec![])` é "perguntei e não há"; `None` é "não
consegui perguntar". Juntar os dois num `unwrap_or_default()` é como todos os
defeitos desta rodada nasceram — e o resultado é sempre o mesmo: o produto
afirma o que não verificou, às vezes com achado tranquilizador, às vezes com
silêncio, que é pior porque nem aparece na tela.

## Os 13 testes que não rodam na esteira

Ficavam sem inventário: a esteira roda o que não está marcado como `ignore`, e
ninguém sabia de cabeça o que os ignorados cobrem nem por quê. Sem essa lista,
"está ignorado" vira o mesmo silêncio que este documento cobra do produto.

Nenhum deles é candidato a promover, e por razões diferentes:

| Teste | Por que não roda sozinho |
|---|---|
| `real_admin_optimizations_apply_and_revert` | **Altera o sistema de verdade.** Serviço, energia, boot |
| `real_apply_and_revert_cycle_restores_the_system` | Idem, no registro |
| `real_startup_cycle_restores_exact_bytes` | Idem, na inicialização do cliente |
| `real_full_cycle_with_measurement` | Idem, e leva ~20 s |
| `mede_quadros_de_verdade` | Exige administrador **e algo desenhando na tela** |
| `ensaio_de_taxa_nesta_maquina` | Depende do monitor e do driver de vídeo |
| `noise_calibration` | Mede a mesma máquina várias vezes; é o que DEFINE os limiares |
| `null_test_never_reports_a_gain` | Duas medições completas em release |
| `real_benchmark_produces_plausible_numbers` | ~8 s ocupando todos os núcleos |
| `inspecao::dump` | Escreve arquivo e depende do Edge |
| `onde_vai_o_tempo` · `onde_vai_o_tempo_da_prontidao` · `onde_vai_o_tempo_do_perfil` | Instrumentos de medição: imprimem, não afirmam |

**O caso que dói é o `null_test_never_reports_a_gain`.** Ele é a garantia central
de honestidade do produto — mede duas vezes sem mudar nada e falha se qualquer
indicador acusar melhora. É a diferença entre um medidor honesto e um vendedor de
ilusão, e a esteira nunca o executa.

Mas ele não pode ser promovido como está: o runner do GitHub é máquina virtual
compartilhada, com ruído de vizinhança maior que o desta máquina. Um teste que
compara medições ali falharia por causa do vizinho, e **teste que falha sozinho
ensina a equipe a ignorar vermelho** — que é justamente o que este documento já
registra sobre teste que depende da máquina.

O caminho, quando for a hora: rodá-lo em máquina conhecida antes de publicar
versão, como parte do ritual de release, e não dentro da esteira. Fica anotado
como decisão consciente, não como esquecimento.

### O disco da máquina de desenvolvimento encheu

Durante a 1.7 o `cargo` parou com `os error 112` — **espaço insuficiente**. O
`C:` estava com **0 GB livres de 464,7 GB**, e 33 deles eram pastas `target/` de
compilação do próprio Otimiza.

Vale registrar pela ironia e pela lição: a ferramenta que tem liberador de espaço
encheu o disco da máquina do dono compilando a si mesma. Limpas as duas
`target/`, o disco voltou a **26,6 GB (5,72%)** — acima do crítico, ainda abaixo
dos 10% que o próprio produto usa como limiar de folga.

Suspeitei que isso tivesse contaminado a medição — e **remedir com folga derrubou
a suspeita**: o `detect_system_storage` deu 1440 ms com disco livre contra 957 ms
com disco cheio. É variação grande, não efeito do disco. A hipótese estava
errada, e fica registrada como errada.

### Quatro erros de medição numa rodada só

A 1.7 mediu muito, e errou a medição quatro vezes. Todas do mesmo tipo:
**cronometrar uma coisa e concluir sobre outra.**

1. **O comentário de agosto** dizia "prontidão 4,8 s · saúde 4,5 s". Os dois
   números eram do `hardware::profile`, pago por quem chamasse primeiro. A ordem
   "caros primeiro" foi calibrada com custo que muda de dono.
2. **A primeira medição do veredito** deu "prontidão = 58% do tempo". Mesma
   causa: a prontidão só era a primeira da fila.
3. **A verificação do conserto da CPU** mostrou o número parado, e quase virou
   "o conserto não funcionou" — o medidor continuava cronometrando
   `refresh_cpu_all()`, o código velho, com a produção já usando o novo.
4. **Os "79 ms" da consulta de disco.** A consulta rápida foi medida DEPOIS da
   antiga, que já tinha carregado o módulo `Storage` e aquecido o WMI. Medido em
   processo frio, na posição certa: **232 ms**.

Nenhum desses foi pego por teste. Todos foram pegos por estranhar um número.

**Os números honestos da detecção de disco:**

| Caminho | Frio |
|---|---|
| Antigo — três cmdlets do módulo `Storage` | 1571–3581 ms |
| Novo — CIM direto, em processo frio | **232 ms** |
| Novo — dentro do produto | **957–1440 ms** |

O ganho é real, mas o produto ainda paga **4 a 6× mais** que o mesmo comando num
processo frio. Não é o disco e não é o módulo `Storage`. Sobra a sessão
persistente de PowerShell (`sessao.rs`) — hipótese ainda **não verificada**, e
anotada como pendência em vez de conclusão.

## A 1.8, e a pergunta que ela fez ao próprio produto

O Otimiza cobra do mercado que só se afirme o que foi medido. A 1.8 nasceu de
virar essa régua para dentro — e ela achou seis lugares onde o produto não
estava cumprindo isso consigo mesmo.

**Nenhum recurso novo.** É uma versão inteira de tirar afirmação que não se
sustentava.

### O que ele afirmava sem ter medido

| Onde | O que a tela dizia | O que era |
|---|---|---|
| `veredito.rs:640` + `diskspace.rs:365` | "Restam 0.0 GB livres no disco do Windows", **Critical** | `disk_usage()` devolvia `(0,0)` quando não achava o volume do sistema |
| `memory.rs:70` + `veredito.rs:253` | "Nenhuma paginação configurada, com 0.0 GB de RAM", **Critical**, com botão que escreve | `ler_memoria()` terminava em `unwrap_or_default()` |

**Nos dois casos o guard certo já existia no mesmo arquivo**, aplicado a um
achado vizinho: `diskspace.rs:676` protegia `pressure` com `total_bytes > 0`, e
`memory.rs:257` protegia outro achado com `total_ram_gb > 0.0`. Não faltou
conhecimento — faltou aplicar o mesmo cuidado ao número principal.

O da memória era o pior dos dois, e por um motivo específico: ele não parava na
frase falsa. O achado é mapeado para `set_automatic_pagefile`, então o produto
**convidava o cliente a escrever no sistema** a partir de uma medição que não
aconteceu.

Os dois viraram `Lacuna` pelo mecanismo que `Firmware`, `Saude` e
`Esgotamento` já usavam. Nenhuma estrutura nova foi inventada.

### Onde a reversibilidade não se sustentava

O produto promete que toda mudança volta, byte a byte. Quatro defeitos comiam
essa promessa por baixo, e nenhum é honestidade — é integridade.

1. **`registry::read` confundia "não existe" com "não consegui abrir"**
   (`registry.rs:21`). Qualquer erro virava `AbsentKey`, que a reversão trata
   ao pé da letra: apaga o valor e, se a chave ficar vazia, apaga a chave. Uma
   chave que **existia** e não pôde ser lida por ACL era gravada como "não
   existia", e o Desfazer removia do cliente algo que era dele.

2. **`network::set_dns` estava acoplado ao item acima** (`network.rs:302`).
   Fazia `unwrap_or(Absent)`, inalcançável enquanto o `read` só falhava com
   hive desconhecida. Consertar o `read` **sem** tocar nesta linha teria criado
   o mesmo defeito no DNS. Os dois no mesmo commit.

3. **`delete_value` reportava sucesso sem desfazer** (`registry.rs:176`). E o
   estrago não era só a mensagem: `ChangeLog::take` já consumiu a entrada
   quando a reversão começa, então o valor continuava aplicado **e** a única
   anotação de como voltar tinha sido gasta.

4. **`changes.json` sem escrita atômica** (`changelog.rs:265`). Uma queda no
   meio da gravação deixava JSON truncado, que era lido como histórico vazio:
   tudo voltava a aparecer como disponível, "Desfazer tudo" dizia que não havia
   nada a fazer, e as mudanças seguiam no registro do cliente.

O mesmo padrão valia para o `licenca.json`, com preço diferente: arquivo
truncado punia **quem pagou**, devolvendo o portão de ativação a quem já tinha
comprado. O portão continua fechado — licença ilegível não é licença válida —,
mas a frase mudou.

### Uma corrida que a própria correção trouxe

A gravação atômica com nome de temporário fixo criou uma disputa: dois
caminhos gravando ao mesmo tempo brigam pelo mesmo arquivo, o primeiro a
renomear leva embora, e o segundo falha com "não encontrado". No produto isso
aconteceria com duas janelas abertas.

**Foi um teste antigo que pegou** — `take_removes_entry_so_it_can_be_reverted_once`.
O nome do temporário passou a levar processo e nanossegundos. Vale registrar
porque é o tipo de coisa que um conserto traz junto sem ninguém procurar.

### O convite do Discord ganhou conserto remoto

Ele vence em 28/09/2026, e o `main.ts` explicava por que isso não tinha
solução: *"Este produto não tem camada de rede nenhuma — zero dependências
HTTP, por decisão de projeto"*.

**A premissa estava errada.** O produto tem rede desde antes: `reqwest` é
dependência e `atualizacao.rs` consulta o GitHub. Era a quarta ocorrência da
mesma afirmação falsa — as outras três estavam no site e no `SECURITY.md`.

Agora o convite vem de `.github/convite.json`, lido **no clique** e não na
abertura: quem nunca pede suporte não paga requisição, e a 1.7 gastou uma
versão inteira derrubando o tempo de abertura. O embutido continua como
reserva, e o valor vindo da rede só passa se tiver a forma exata
`https://discord.gg/<código>` — ele abre uma janela no navegador do cliente.

**E o cliente pagante não tinha caminho até o suporte.** `CONVITE_DISCORD` era
usado num único ponto: dentro do portão, que some quando a licença é aceita. O
rodapé ganhou "Falar com o suporte", pelo mesmo motivo que o tutorial já estava
lá.

### Três documentos que afirmavam o que não verificaram

| Arquivo | Afirmava | Realidade |
|---|---|---|
| `SECURITY.md:66` e `:108` | "o instalador é assinado digitalmente" | Não é — e o arquivo **linkava para o documento que diz o contrário** |
| `site/src/data/i18n/*` (12 lugares) | "não tem camada de rede" | Tem |
| `docs/LICENCA.md:43` | a chave pública é "de teste" | É a de produção desde 29/08/2026 |

O `SECURITY.md` é o que um comprador desconfiado abre primeiro, e duas linhas
antes da afirmação falsa ele prega: *"Funcionalidade não testada aparece como
pendente em PROGRESS.md"*.

### O que a 1.8 NÃO fez, e por quê

Cortado de propósito — um plano que faz tudo não é rigor, é papa:

- **Quatro módulos que dizem "nada encontrado" em verde quando a leitura
  falhou**: conflitos (`conflicts.rs:101`), tarefas (`tasks.rs:50`), serviços
  (`servicesaudit.rs:164`) e bloatware (`bloatware.rs:159`). O conserto é
  mecânico e o modelo já existe, mas são quatro módulos e quatro telas.
- **`analyze_gpu_preference` / `set_gpu_preference`**: registrados, testados,
  classificados — e **sem botão**. *(Fechado na 1.9.)*
- **`prova_guardada`**: registrado e nunca chamado. *(Fechado na 1.9.)*

Os três eram valor pronto sem porta, não defeito de honestidade. Os dois
últimos foram ligados na 1.9; o primeiro continua de pé.

## A 1.9, e o valor que estava preso do lado de dentro

A 1.8 tirou do produto tudo que ele afirmava sem ter medido. A 1.9 é a outra
metade da mesma auditoria: **funcionalidade pronta, testada e reversível que
nunca chegava ao cliente porque faltava um botão.**

Quase nada aqui foi escrito do zero. O trabalho foi ligar o que já existia.

### O que estava calado, e passou a ter tela

| Motor | Desde quando existia | O que faltava |
|---|---|---|
| `analyze_gpu_preference` / `set_gpu_preference` | registrado e testado antes da 1.8 | painel; o diagnóstico acusava e não havia onde clicar |
| `prova_guardada` | registrado e nunca chamado | a aba nunca perguntava por ele |
| `frametime_mediano_ms`, `low_1pct`, `engasgos_por_minuto` | calculados no backend | a interface não os declarava — atravessavam o IPC e eram descartados |
| `deteccao::procurar` (4 sinais) | desde a 0.13 | ninguém usava para preencher o campo do jogo |
| `acao` nos achados não eleitos | o campo existia no dado | `renderDiagnostic` o descartava em silêncio |

### O mapa de botões cobria 4 achados de ~30

`acao_de` foi de 4 para 8 entradas. Os quatro que entraram tinham, no próprio
texto do achado, o **endereço da aba** onde o cliente deveria resolver:

    thermal.rs:281   "Aplicar o plano de alto desempenho NA ABA OTIMIZAÇÕES"
    firmware.rs:419  "A otimização 'Liberar limites de inicialização' corrige"

E `renderDiagnostic` passou a desenhar botão em **todo** achado que tem
conserto, não só no eleito. O caso que mais doía era a taxa do monitor — que o
próprio código chama de *"a maior diferença de fluidez que existe num PC"* e
que perde a eleição para qualquer crítico de memória, disco ou térmico. Ou
seja: sumia justamente nas máquinas com problema.

**O erro simétrico tem teste próprio.** `achado_de_hardware_continua_sem_botao`
trava memória em canal único, RAM insuficiente, desgaste de disco e pressão
recorrente **fora** do mapa. Prometer clique para o que só se resolve comprando
peça seria o mesmo defeito, virado do avesso.

### Os três pedidos do outro dono

| Pedido | O que foi feito |
|---|---|
| "barra de carregamento nas ações" | `<progress>` nativo, alimentado pelo `index`/`total` que o backend já emitia. Sem transição de largura: deslizar entre dois passos é a aparência de progresso inventado |
| "mudar o tamanho quando abre, está quase na tela inteira" | era fixo em 1440×900. Passa a medir a tela: 2/3 da largura × 3/4 da altura, com mínimo utilizável e teto na própria tela |
| "cor em cada atualização ou uma só" | uma só. Trocar a identidade a cada versão é o que faz produto parecer instável |
| "deixar as coisas maiores ou deixa assim mesmo talvez" | maiores, e só o que se lê |

### O tamanho dos elementos, respondido com a tela aberta

Ele mesmo ficou em dúvida, então a resposta veio de abrir o programa compilado
e olhar, e não de opinar no escuro.

O achado: **o texto de leitura estava em 12px e os rótulos em 11px, e a escala
inteira já era um conjunto de dez variáveis** — ou seja, mexer nisso era trocar
três linhas, não redesenhar.

- `--t-body` 12 → 13, `--t-lead` 13 → 14, `--t-value` 14 → 15
- `--t-micro`, `--t-label` e `--t-nota` **ficaram onde estavam**. São rótulos em
  MAIÚSCULA com espaçamento entre letras, e maiúscula espaçada fica *pior* de
  ler quando cresce: a palavra alarga, a linha quebra, e o rótulo passa a
  competir com o dado que ele nomeia
- `--t-number` e `--t-huge` também ficaram: já dominam a tela

E a olhada achou três coisas que não estavam no pedido:

1. `.nav-atalho` estava em **9px**, abaixo do piso de 10px que o comentário da
   própria escala declara. Passou a usar `--t-micro`
2. `.gpupref-nome` e `.gpupref-estado` — escritos nesta mesma versão — tinham
   pixel na mão em vez de variável. Voltaram para a escala
3. `.opt-name`, o **nome da otimização no catálogo**, era `nowrap` com
   reticências. Com o texto maior, "Rede de baixa latência (desativar Nagle)"
   virava "…(desativar Nagl…". É a lista onde a pessoa DECIDE o que aplicar:
   passou a quebrar em duas linhas em vez de cortar. As reticências de
   `.process-name` e `.startup-name` ficaram — são listas ao vivo, e lista que
   reflui a cada segundo é pior que lista cortada

O tamanho fixo era pior do que ele descreveu: **em notebook de 1366×768 a
janela não cabia, nas duas dimensões.** Número fixo não resolve, porque não
existe número que sirva para 1366×768 e para 4K.

A ordem das contas tem teste próprio: numa tela menor que o mínimo, a resposta
certa é a tela inteira, e não o mínimo — aplicar o mínimo por último recriaria
o defeito original.

### O defeito que um teste achou sozinho

`executavel_do_jogo` varria a lista de nomes conhecidos e devolvia o **primeiro**
processo que casasse. Com o FiveM aberto isso devolvia `FiveM_ChromeBrowser` —
o navegador embutido — em vez de `FiveM_b3258_GTAProcess.exe`. Os dois casam com
a chave `fivem_`, e qual vencia dependia da ordem de enumeração: sorteio.

Como a 1.9 preenche o nome do jogo sozinho, isso mediria FPS de um subprocesso
de navegador na tela cujo trabalho inteiro é provar número.

E a afirmação do teste estava errada junto: ele exigia `.exe` no fim de todo
executável de jogo, e o `FiveM_ChromeBrowser` existe em disco **sem extensão**.
O teste passava só porque nenhuma máquina de teste tinha o FiveM aberto.

### O que a 1.9 NÃO fez, e por quê

- **Os quatro módulos que dizem "nada encontrado" em verde quando a leitura
  falhou** — conflitos (`conflicts.rs:101`), tarefas (`tasks.rs:50`), serviços
  (`servicesaudit.rs:164`) e bloatware (`bloatware.rs:159`). Continuam de pé
  desde a 1.8. São quatro módulos e quatro telas; é uma versão inteira sozinho.
- **O painel da NVAPI** — 1.151 linhas de código de driver NVIDIA sem ponto de
  entrada na interface. Ligar isso exige uma máquina com placa NVIDIA para
  provar cada ajuste, e este produto não aceita ligar botão que ninguém viu
  funcionar.
- **As duas linhas do tempo** — a pressão de 14 dias e o esgotamento de 30 dias
  são calculados e não têm gráfico. É desenho, não motor: fica para quando
  houver tela pensada para isso.
- **`start_monitoring` / `stop_monitoring`** — comandos mortos, ninguém chama.
  Remover é faxina, e faxina no meio de uma versão de recursos mistura o
  histórico do que mudou.

## A 2.0 — em andamento

### O congelamento de programas saiu do produto

Pedido do dono, e com razão: foi a opção que mais machucou cliente.

Até a 1.9, o modo jogo automático suspendia Discord, navegador, Spotify e afins
durante a partida, para devolver memória ao jogo. Suspender em vez de matar era
melhor que o mercado — nada se perdia —, e mesmo assim:

| Versão | O que aconteceu |
|---|---|
| 1.1.1 | Programa suspenso não responde ao aviso de desligamento; o Windows não descarrega o perfil, e no login seguinte o Explorador e a barra de tarefas **não abriam nada** |
| 1.1.2 | A Steam congelada **não abria jogo, não baixava, não respondia** |
| — | O relatório de suporte (`suporte.rs`) nasceu de um cliente pagante dizendo que **os programas não abriam mais** |

Cada incidente ganhou uma rede de segurança, e as redes funcionavam. Mas quatro
redes para não quebrar o PC de quem comprou é caro demais por memória que o
cliente nem vê.

**O que saiu:** a chamada no vigia (`gamemode::passo`), `suspender_fundo` e toda
a decisão de quem suspender (`SUSPENSIVEIS`, `NUNCA_SUSPENDER`, `LANCADORES`,
`pode_suspender`), a chamada de suspensão da API do Windows, as duas ações do
anticheat que só existiam para isso, os comandos `congelados_agora` e
`descongelar_agora`, o bloco "Congelado agora" da aba Jogos, o modal de
reconsentimento, o campo `game_mode_avisado` das preferências e a linha
"Congelados agora" do relatório de suporte.

**O que ficou, de propósito:** as quatro redes que DEVOLVEM — abertura,
fechamento, fim de sessão do Windows e prazo de dez minutos. Quem atualizar de
uma versão antiga com algo registrado como suspenso recebe esses programas de
volta na primeira abertura. Com o registro vazio, que é o caso de todo mundo
daqui para frente, nenhuma delas faz nada.

**E a trava que impede a volta:** `o_produto_nao_congela_mais_nenhum_programa`
varre o código inteiro, sem os comentários, atrás das chamadas que suspendem
processo no Windows e do próprio `suspender_fundo`, e reprova o build se achar.

O modo jogo continua existindo, e agora é só o que o nome promete: plano de alto
desempenho enquanto o jogo está aberto, e prioridade para ele no processador.

Preferências gravadas antes da 2.0 continuam carregando: o arquivo antigo ainda
tem a chave `game_mode_avisado`, e há teste garantindo que uma chave que sobrou
não derruba a leitura.

## Pendente

### O que a 1.9 entregou sem ter visto funcionar

Esta lista existe porque o produto cobra isso do mercado, e a régua vale para
ele mesmo.

| Item | Por que não foi visto |
|---|---|
| **Painel da preferência de placa de vídeo** | Metade foi vista: esta máquina tem **uma** placa (GTX 1650, conferido com `Win32_VideoController`) e o painel **ficou escondido**, que é o comportamento certo. A outra metade — a lista, o botão e o que acontece depois do clique — só aparece com **duas** placas, e falta abrir num notebook com placa dupla. O motor por trás tem teste de unidade e é reversível |
| **Barra de progresso durante uma ação real** | O elemento aparece e é alimentado pelos números que o backend já emitia, mas as ações que a movem exigem administrador e mexem no sistema. Foi conferida pelo caminho do código, não vendo uma otimização correr do começo ao fim |
| **Restauro da medição guardada** | O código lê `prova_guardada` ao abrir a aba. Aqui não havia medição guardada para restaurar, então o caminho de "existe algo salvo" não foi visto na tela |

Nenhum dos três é chute: os três motores têm teste. O que falta é o olho.

### Cobertura real das otimizações de administrador

O ciclo com elevação aplica cada otimização, confere contra o sistema, desfaz e
exige que o estado final seja idêntico ao inicial. **4 otimizações percorreram o
ciclo completo** na máquina de desenvolvimento. As demais já estão no estado
final aqui, e testá-las exigiria desconfigurar o PC de quem roda o teste.

Tipos de ação já executados contra um sistema real:

| Tipo de ação | Executada de verdade |
|---|---|
| Valor de registro (DWORD e texto) | sim |
| Valor de registro binário (inicialização) | sim, com reversão byte-exata |
| Enumeração de interfaces de rede (Nagle) | sim, com elevação |
| Enumeração de classe de dispositivo (placa de rede) | sim, com elevação |
| Política de máquina em `HKLM\SOFTWARE\Policies` | sim, com elevação |
| **Desativar serviço** | **sim** — `WSearch` foi de Automático a Desativado e voltou |
| **Ajuste fino de energia** | **sim** — estado mínimo do processador, já gravando tomada E bateria |
| **Configuração de boot (`bcdedit`)** | **sim** — remoção do HPET forçado, aplicada e revertida |
| Trocar plano de energia | **não** — já está em Alto Desempenho aqui |
| MSI da placa de vídeo | **não** — já estava ativo aqui |
| Hibernação, limites de boot, compressão de memória | **não** — indisponíveis aqui |

As três primeiras linhas em negrito eram "não" até a 1.7. O ciclo real foi
executado contra esta máquina com o build da 1.7 e fechou as três — **quatro
otimizações percorreram o ciclo completo**: rede de baixa latência (10 valores de
registro), estado mínimo do processador, busca do menu Iniciar e indexação de
busca.

**E ele achou uma contradição que a própria 1.7 tinha criado.** A correção de
honestidade do Armazenamento Reservado ensinou a INSPEÇÃO a dizer "não sabemos,
então oferecemos" — mas o caminho da APLICAÇÃO continuava respondendo "este
Windows não tem o recurso". O item passou a ser oferecido e falhava afirmando,
na hora de agir, exatamente o que se acabara de admitir não saber. Nenhum teste
de unidade veria isso: os dois lados estavam certos sozinhos e errados juntos.

As três primeiras lacunas são otimizações de alto impacto. Elas funcionam segundo
o código e os testes de unidade, mas nunca foram vistas aplicando e revertendo
numa máquina.

### Antes de vender
1. **Fechar as lacunas acima numa máquina de teste** — de preferência uma que
   ainda não tenha sido otimizada, onde as 14 apareçam como disponíveis
2. **Instalar o pacote gerado numa máquina limpa** — o instalador compila, mas
   nunca foi instalado e aberto de fato
3. **Publicar um convite permanente em `.github/convite.json`** — o que está
   lá hoje (`discord.gg/fmeQVJphC`) foi conferido na API e **vence em
   28/09/2026**. Precisa ser um com "Expira em: Nunca" e "Usos: Sem limite".

   **O que mudou na 1.8:** este item deixou de ser sem saída. A justificativa
   antiga — "o produto não tem camada de rede, então convite morto no
   executável não tem conserto remoto" — estava errada: o produto consulta o
   GitHub desde antes. Agora ele lê o convite daquele arquivo no momento do
   clique, então **trocar o valor lá conserta o link inclusive para quem já
   instalou**, sem publicar versão nova.

   O que continua sendo urgente é publicar o convite permanente. O embutido
   segue como reserva para quem estiver sem internet, e ele vence na data
   acima.
4. Assinatura digital do executável — ver
   [`docs/ASSINATURA.md`](docs/ASSINATURA.md). Depende de compra de certificado e
   verificação de identidade; a configuração de build já está preparada
5. `icons/icon.icns` continua sendo o ícone antigo do Tauri — só afeta empacotamento
   para macOS, que ainda não é alvo

### Faxina feita nesta rodada
Removido o esqueleto gerado no início que nunca foi ligado a nada: `CoreEngine`
(orquestrador que não orquestrava — a interface nunca o chamou), a trait
`OptimizationModule` sem implementação, `config.rs` inteiro (licenciamento vai
exigir projeto próprio; 100 linhas mortas prometendo isso era pior que nada) e o
comando `greet` do template.

Em troca, o `Logger` — que existia sem uso — passou a registrar as duas falhas que
o código engolia em silêncio: reversão parcial e serviço que não para. **Zero
avisos de compilação**, de 18 que havia.

### Depois
- Limpeza de arquivos temporários
- Gerenciamento de programas de inicialização
- Otimizações para Linux e macOS

## Como rodar

```bash
cd pc-optimizer
npm install
npm run tauri dev
```

Para aplicar otimizações de sistema, abra como administrador.

## Arquitetura

- **Backend:** Rust + Tauri 2 + sysinfo + winreg
- **Frontend:** TypeScript + Vite
- **IPC:** comandos Tauri assíncronos
- **Estado:** `tokio::sync::Mutex` (os guards atravessam pontos de `await`)
