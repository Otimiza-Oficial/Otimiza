# Análise dos pacotes de otimização do dono (11/10/2026)

O dono pediu para levar ao Otimiza tudo dos dois pacotes que lhe deram FPS:
- `Downloads\d`: 27 arquivos .reg/.bat/.cmd ("rmstech", "BareFox").
- `Downloads\guide\guide`: guia "W10 22h2" de @vitor_rr, @khorvietech e @TXNSU, com 323 arquivos.

Cada arquivo de texto foi lido. Nada foi executado. Os programas de terceiros (.exe) não foram rodados nem embutidos.

**Estado do PC do dono (lido, sem mudar nada):**
- UAC, Defender (tempo real), DEP, MMCSS e IPv6 estão ligados, e as proteções Spectre/Meltdown estão no padrão.
- O Isolamento de Núcleo (HVCI) está ligado.
- Dos pacotes, só `GlobalTimerResolutionRequests=1` está ativo.
- As opções de CPU do GTA no FiveM já estão no mínimo: CityDensity, variedade de pessoas e veículos, LodScale e MaxLodScale todos em 0, e LOD de pessoas e veículos em -0,5.

## Decisão do dono (11/10/2026): entra tudo o que mexe em desempenho

O dono aplicou os dois pacotes, sentiu o jogo mais liso e pediu tudo no Otimiza. Entrou, item a item, com o efeito
escrito, o preço escrito e o desfazer. Todos são **"só se pedir"** (`catalog::CONDICIONAIS`): nenhum entra sozinho no
"Otimizar agora" de um cliente.

**Voltaram dos RETIRADOS**, com os valores dos pacotes:
- `system_responsiveness_gaming` (SystemResponsiveness 0, NetworkThrottlingIndex desligado).
- `foreground_priority` (0x26) e `mmcss_games` (8/6/High/High + Affinity 0, Background Only False, Clock Rate 10000).
- `disable_power_throttling`, `disable_sysmain` e `disable_memory_compression` (sem a trava de 12 GB; o risco de
  engasgo continua escrito e o item fora do lote).
- `disable_xbox_services`, `disable_search_indexing`, `disable_telemetry`, `telemetry_policy`, `error_reporting_off`
  (+ WerSvc e Fault Tolerant Heap).
- `notifications_off` (+ central de notificações; o aviso de segurança continua).
- `disable_startup_delay`, `disable_hibernation`, `disable_transparency`, `background_apps_off`, `disable_widgets`,
  `start_menu_web_search_off`, `remote_assistance_off`, `maps_auto_update_off`, `store_auto_download_off`.
- `edge_background_off`: agora também Chrome e Firefox. Corrigido o nome da política do Edge (`StartupBoostEnabled`;
  o Otimiza escrevia `StartupBoost`, que o Edge não lê).

**Itens novos:**

| Item | O que faz |
|---|---|
| `fila_de_entrada_curta` | MouseDataQueueSize e KeyboardDataQueueSize 30 |
| `windows_responde_rapido` | MenuShowDelay, MouseHoverTime, tempos de desligamento, AutoEndTasks |
| `prefetch_desligado` | EnablePrefetcher 0 |
| `privacidade_do_pacote` | Permissões do ConsentStore, coleta de digitação, CEIP, dicas, histórico de atividades |
| `servicos_bluetooth_fora`, `servicos_impressora_fora`, `servicos_hyperv_fora`, `servicos_pouco_usados`, `servicos_de_empresa_fora` | Os serviços do "apply all.bat", do "Desabilitar arquivos desnecessários.reg" e do "Tweaking Windows.bat", em grupos com o preço de cada um |
| `placa_no_clock_maximo` | DisableDynamicPstate=1 em cada placa NVIDIA (`Action::ClockMaximoDaPlaca`) |
| `tick_do_pacote` | bcdedit disabledynamictick, useplatformtick, tscsyncpolicy legacy (`Action::BootValor`) |
| `tcp_do_pacote` | TcpTimedWaitDelay, MaxUserPort, DefaultTTL, prioridades do ServiceProvider |
| `compartilhamento_do_pacote` | LanmanServer do "reduce_delay.reg" |
| `bufferbloat_do_pacote` | Set-NetTCPSetting, RSS e RSC, LSO e checksum (`Action::RedeTcp`/`RedeOffload`/`RedeAdaptador`) |
| `tela_cheia_sem_otimizacao` | As flags GameDVR_FSE* do "Disabilitar Game DVR.reg" |

Também ganharam o que os pacotes fazem a mais: `drivers_fora_do_windows_update` (políticas de DriverSearching e
Device Metadata), `visual_effects_performance` (animação da barra, Aero Peek) e `manutencao_automatica_fora`
(Sensor de Armazenamento).

O desfazer das ações novas lê nome e valor do histórico; `windows/pacote.rs` só aceita os nomes da lista e valores de
letras e números, para o `changes.json` não virar comando como administrador.

**Continuam fora** (a lista pública está em `naofazemos.rs`): ativador do Windows (pirataria); Defender, central de
segurança, SmartScreen/Smart App Control e UAC desligados; DEP e mitigações de processo; IPv6 desligado; watchdog e
chaves de DPC, LatencyTolerance, IRQ8Priority, LazyMode, TimeIncrement, QuantumLength e as `D3D*` (placebo ou
perigoso); apagar logs de eventos e a pasta Prefetch; atualização de navegador desligada; `LowLevelHooksTimeout` (o
contrário do que promete); `graphicsCaptureWithoutBorder` negado (atrapalha captura); NlaSvc apontando para o Google;
"Take Ownership" no menu; `Parents`/`Children` do GameConfigStore (jogos do PC de outra pessoa); `Chimney` (não existe
no Windows 10/11); MTU 1500 (já é o padrão); o plano de energia `.pow` e o perfil `.nip` de terceiros (o Otimiza tem
plano próprio e escreve o driver NVIDIA pela NVAPI); programas `.exe` de terceiros. No PC do dono Defender, UAC e DEP
estavam ligados: o ganho que ele sentiu não veio desses.

## O Otimiza já tem

**Correção (11/10):** a primeira versão desta análise dizia que o Otimiza "já faz" itens que as auditorias 2.9 e A3.2
RETIRARAM por falta de ganho reproduzível. Eles continuam no catálogo só para desfazer em quem tinha aplicado
(`catalog.rs`, `RETIRADOS`).

| Ajuste dos pacotes | No Otimiza | No PC do dono hoje |
|---|---|---|
| `SystemResponsiveness` 0, MMCSS "Games" alto, `Win32PrioritySeparation` 0x26 | **Retirados** (A2.9: sem ganho reproduzível) | aplicados (0; 8/6/High/High; 38) |
| `PowerThrottlingOff` | **Retirado** (desliga o EcoQoS que o governador usa) | aplicado (1) |
| SysMain, compressão de memória | **Retirados** (o Windows gerencia) | SysMain ligado |
| Transparência, atraso de inicialização, hibernação | **Retirados** (A3.2: conforto, sem efeito em jogo) | Inicialização Rápida desligada |
| `NetworkThrottlingIndex` | **Retirado** (`network_low_latency`) | aplicado (desligado) |
| Game DVR, Modo de Jogo | ativos (condicionais) | |
| WSearch, Spooler, DiagTrack | ativos (WSearch) ou retirados (telemetria) | |
| Aceleração do mouse, teclas de aderência | ativos ("só se pedir") | fila do mouse 50 |
| MSI mode da placa de vídeo, VBS/HVCI | ativos (Expert; o VBS é troca de segurança) | HVCI LIGADO |
| `useplatformclock`, Nagle, energia da placa de rede | ativos | |
| NVIDIA: energia, latência, textura, V-Sync, cache de shader 10 GB | ativos (`nvdriver`) | |
| Plano de energia, limpar temporários | ativos (`PlanoOtimiza`, Remoção de lixo) | |

O conflito é real: o dono sentiu ganho com o pacote "d", que traz justamente os retirados. Só uma medição no FiveM
do dono decide (pacote aplicado × padrão do Windows, mesma cena, PresentMon).

## Candidatos novos (seguros, reversíveis)

Entram como itens do catálogo, com desfazer e medida antes e depois:

1. **Timer global no Windows 11** (`kernel\GlobalTimerResolutionRequests=1`).
2. **Windows Update sem trocar o driver de vídeo** (`ExcludeWUDriversInQualityUpdate=1`, `DriverSearching\SearchOrderConfig=0`).
3. **Sem Inicialização Rápida** (`HiberbootEnabled=0`).
4. **Apps da loja sem rodar em segundo plano** (`BackgroundAccessApplications\GlobalUserDisabled=1`).
5. **Navegadores sem modo em segundo plano nem Startup Boost** (políticas do Edge e do Chrome).
6. **NVIDIA: taxa de atualização preferida "a mais alta"** e Ansel desligado.
7. **Serviços agrupados** (`SvcHostSplitThresholdInKB` = RAM): menos memória em PC de 8 GB. Medir.
8. **Manutenção automática desligada** (`MaintenanceDisabled=1`). Avisar que o TRIM agendado sai junto.
9. **Placa de vídeo sempre no clock máximo** (`DisableDynamicPstate=1` na classe Display). Custa energia e calor. Medir.
10. **MPO desligado** (`Dwm\OverlayTestMode=5`). Corrige tremor em algumas NVIDIA, mas a captura DXGI do Frame Boost depende do MPO no 24H2+. Medir as duas coisas.
11. **Serviços do Xbox e do Bluetooth**: só oferecer a quem não usa.
12. **Laboratório, com evidência fraca:** fila do mouse e do teclado (`DataQueueSize`) e `disabledynamictick`.

## Troca de segurança (só com escolha e aviso, como o VBS)

- **Proteções Spectre/Meltdown desligadas** (`FeatureSettingsOverride=3`, `FeatureSettingsOverrideMask=3`).
  - Ganho real em processador Intel da família Skylake (o i3-10100F é), em código com muita chamada ao sistema. O FiveM tem: veja o E6 no PROGRESS.
  - Custo: abre brecha de leitura de memória entre programas.

## Não entram

| Item | Por quê |
|---|---|
| `Windows Activator.ps1` | Pirataria: risco legal para a marca e para os clientes |
| Defender, central de segurança e SecurityHealthService desligados | Deixa o PC sem antivírus |
| UAC desligado (`EnableLUA=0`) | Todo programa roda como administrador sem perguntar |
| DEP desligado (`bcdedit /set nx AlwaysOff`) e mitigações de processo | Proteções básicas de memória; anticheats podem recusar |
| SmartScreen / Smart App Control desligados | Proteção contra programas maliciosos |
| IPv6 desligado (`Tcpip6`) | A Microsoft recomenda não desligar; quebra recursos do Windows |
| Watchdog e limites de DPC alterados | Pode esconder travamento e levar a tela azul |
| Limpar todos os logs de eventos | Apaga o diagnóstico do PC |
| Limpar a pasta Prefetch | O Windows refaz; programas abrem mais devagar até lá |
| `GameConfigStore\Children` do "Disabilitar Game DVR.reg" | São os jogos do PC de OUTRA pessoa (usuário "pc") |
| `D3D11_*`/`D3D12_*` em `HKLM\SOFTWARE\Microsoft\DirectX`, `TimeIncrement`, `QuantumLength`, `Control\Processor\Cstates` | Chaves que o Windows não lê (placebo) |
| "Bufferbloat": RSS, offload e checksum desligados, autotuning desligado | Mais trabalho para o processador (ruim para o FiveM, preso no CPU) e download mais lento |
| MMCSS desligado | Desliga justamente o agendador que o próprio pacote ajusta |
| MemReduct, UnparkCpu, DeviceCleanup, OpenShell, NvCleanstall, script remoto do Chris Titus (`irm … \| iex`) | Programas de terceiros. O Otimiza faz o mesmo com código próprio quando vale |
