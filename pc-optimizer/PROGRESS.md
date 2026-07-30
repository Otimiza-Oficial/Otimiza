# PC Performance Optimizer — Estado real do projeto

> Este documento registra apenas o que foi **verificado**. Funcionalidade que não
> foi executada e conferida aparece como pendente, mesmo que o código exista.

## Verificado

| Item | Como foi verificado |
|---|---|
| Backend Rust compila | `cargo check` e `cargo build` sem erros |
| 77 testes unitários passam, zero avisos | `cargo test --lib` |
| Instalador gerado | `Otimiza_0.1.0_x64-setup.exe` (2,2 MB) e `.msi` (3,3 MB) |
| Monitor de processos funciona nesta máquina | Discord ×6 · 9,2% da CPU · 1019 MB · marcado como inicialização |
| Ciclo real de inicialização restaura bytes idênticos | Desligou e religou o Discord; bytes conferidos com PowerShell, fora do nosso código |
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

### Antes de vender
1. **Testar as otimizações que exigem administrador** — `disable_startup_delay` e o
   ciclo de inicialização foram executados de ponta a ponta. Plano de energia,
   telemetria, Game DVR, agendamento de GPU e os ajustes de energia do processador
   ainda não rodaram contra um sistema, por falta de sessão elevada
2. **Instalar o pacote gerado numa máquina limpa** — o instalador compila, mas
   nunca foi instalado e aberto de fato
3. Assinatura digital do executável — sem ela o Windows mostra aviso de editor
   desconhecido na primeira execução, o que assusta cliente pagante
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
