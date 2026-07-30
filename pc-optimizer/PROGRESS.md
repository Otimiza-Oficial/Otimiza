# PC Performance Optimizer — Estado real do projeto

> Este documento registra apenas o que foi **verificado**. Funcionalidade que não
> foi executada e conferida aparece como pendente, mesmo que o código exista.

## Verificado

| Item | Como foi verificado |
|---|---|
| Backend Rust compila | `cargo check` e `cargo build` sem erros |
| 112 testes unitários passam, zero avisos | `cargo test --lib` |
| Instalador gerado | `Otimiza_0.2.0_x64-setup.exe` (2,2 MB) e `.msi` (3,3 MB) |
| Monitor de processos funciona nesta máquina | Discord ×6 · 9,2% da CPU · 1019 MB · marcado como inicialização |
| Ciclo real de inicialização restaura bytes idênticos | Desligou e religou o Discord; bytes conferidos com PowerShell, fora do nosso código |
| Esteira do GitHub compila e testa em máquina limpa | Oito áreas de teste verdes; instaladores anexados ao release |
| Liberador de espaço varre esta máquina | 971 MB recuperáveis, por categoria |
| Diagnóstico de memória acha problema real aqui | 11,3 GB prometidos para 7,9 GB físicos |
| Detector de conflitos acha problema real aqui | Driver Booster 13, entre 253 programas examinados |
| Auditor de tarefas lê o agendador desta máquina | 203 tarefas, 17 de terceiros |
| Ícone da barra de tarefas é a nossa logo | Extraído do `.exe` compilado e conferido visualmente |
| Perfil de hardware desta máquina | SSD, 7,9 GB de RAM, 8 núcleos lógicos |
| Benchmark produz números reais | Executado nesta máquina: 669 Mops/s em 1 núcleo, 4450 em todos, 3600 MHz sob carga |
| **Aplicar e desfazer funcionam de verdade** | Ciclo completo executado contra o registro do Windows; restauração conferida com PowerShell, fora do nosso código |
| Limiares de ruído são medidos, não chutados | `noise_calibration` mede a mesma máquina 3× sem mudar nada |
| Não reporta ruído como ganho | `null_test` mede 2× sem mudar nada e falha se algum indicador acusar melhora |
| Frontend compila | `npx tsc --noEmit` e `npm run build` |
| Leitura de serviços independe do idioma | Teste lê `RpcSs` do registro nesta máquina |
| Parsing do `powercfg` funciona em português | Conferido contra a saída real deste Windows |

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

## Pendente

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
| Desativar serviço | **não** |
| Trocar plano de energia | **não** |
| Ajuste fino de energia (estacionamento de núcleos) | **não** |
| MSI da placa de vídeo | **não** — já estava ativo aqui |
| Hibernação, limites de boot, compressão de memória | **não** — indisponíveis aqui |

As três primeiras lacunas são otimizações de alto impacto. Elas funcionam segundo
o código e os testes de unidade, mas nunca foram vistas aplicando e revertendo
numa máquina.

### Antes de vender
1. **Fechar as lacunas acima numa máquina de teste** — de preferência uma que
   ainda não tenha sido otimizada, onde as 14 apareçam como disponíveis
2. **Instalar o pacote gerado numa máquina limpa** — o instalador compila, mas
   nunca foi instalado e aberto de fato
3. Assinatura digital do executável — ver
   [`docs/ASSINATURA.md`](docs/ASSINATURA.md). Depende de compra de certificado e
   verificação de identidade; a configuração de build já está preparada
4. `icons/icon.icns` continua sendo o ícone antigo do Tauri — só afeta empacotamento
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
- Licenciamento / versão PRO
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
