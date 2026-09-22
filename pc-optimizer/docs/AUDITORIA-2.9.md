# Auditoria 2.9 — tudo o que o produto faz hoje, item por item

> Base auditada: branch `versao-2.9`, que já contém a 2.8.0 publicada.
> ~75 mil linhas de Rust, 13 abas, cerca de 60 painéis.
> **Nenhuma linha de código foi alterada para produzir este documento.**
>
> Classificação usada:
> - **MANTER** — funciona como está.
> - **COND** (manter condicionalmente) — só entra quando o hardware ou o cenário pede.
> - **REFAT** (refatorar) — a ideia é boa, a implementação não.
> - **EXPER** (experimental) — nunca é aplicado automaticamente, só com medição antes e depois.
> - **REMOVER** — sai do produto.
>
> Toda afirmação técnica abaixo vem do que o código escreve de fato (extraído do
> fonte), e não do nome do botão.

---

## 1. Os três achados mais graves

| # | Onde | O que acontece | Por que é grave |
|---|---|---|---|
| 1 | `catalog.rs` · `disable_power_throttling` | `HKLM\…\Power\PowerThrottling\PowerThrottlingOff = 1` | **Desliga o EcoQoS no Windows inteiro.** O modo jogo da 2.9 (`governador.rs`) acalma os programas de fundo justamente com EcoQoS. Com este item aplicado, o modo jogo perde metade do efeito. Em CPU híbrida, também tira do escalonador o jeito de mandar trabalho de fundo para os núcleos E |
| 2 | `fivem.rs` · `priorizar_jogo` (botão "Priorizar o jogo") | põe o processo do FiveM em prioridade **Alta** | É a "prioridade cega" que a 2.9 retirou em outros dois lugares. Abre um handle no processo do jogo, que é a ação mais visível para um anticheat. Só rende com disputa real de CPU, e isso quem resolve agora é o governador |
| 3 | `nvdriver.rs` · opção `vsync` | força **V-Sync desligado no perfil global** | Vale para todo jogo. Em monitor com G-SYNC/VRR, a própria NVIDIA recomenda V-Sync ligado no driver junto com limite abaixo da taxa. Forçar desligado causa rasgo de imagem fora da faixa VRR |

---

## 2. Catálogo de ajustes — o que cada um escreve

**Legenda.** "Rein." = precisa reiniciar. Ganho é o que o próprio catálogo declara: M = mensurável, R = responsividade, S = situacional, 0 = nenhum. Os 9 marcados com † já foram retirados na 2.9 e só guardam o desfazer.

| Id | O que altera, exatamente | Ganho | Rein. | Avaliação técnica | Decisão |
|---|---|---|---|---|---|
| `plano_otimiza` | plano próprio, baseado no Equilibrado, com ajustes básicos (disco sem suspensão, Wi-Fi sem economia, sem ociosidade em multimídia) e ajustes avançados de CPU (mín. 100%, EPP 0, estacionamento, boost) só por escolha. Bateria intocada em notebook | M | não | a parte de CPU é receita fixa, e o motor adaptativo já mede isso por máquina | **REFAT** → fundir no motor de energia |
| `disable_gamedvr` | `GameDVR_Enabled=0`, `AppCaptureEnabled=0`, política `AllowGameDVR=0` | M | não | a gravação em segundo plano do Game Bar custa codificação de vídeo contínua **quando está ligada**; desligada, não muda nada | **COND**: só oferecer se estiver ligada |
| `gpu_hardware_scheduling` | `GraphicsDrivers\HwSchMode=2` | S | sim | depende de placa e driver; é pré-requisito do DLSS Frame Generation; já causou engasgo em driver antigo | **EXPER** com antes e depois |
| `system_responsiveness_gaming` | `SystemResponsiveness=10` e **`NetworkThrottlingIndex=0xFFFFFFFF`** | S | sim | o 2º só age sobre tráfego de rede durante reprodução multimídia, e não muda FPS nem ping de jogo; o 1º afeta só programas registrados no MMCSS. Os dois estão na sua lista de placebos | **REMOVER** |
| `visual_effects_performance` | `VisualFXSetting=2`, `UserPreferencesMask`, `DragFullWindows=0`, `MinAnimate=0`, `MenuShowDelay=0` | R | não | deixa a área de trabalho mais ágil em PC fraco; jogo em tela cheia não é afetado | **COND**: modo "área de trabalho", fora do fluxo de jogo |
| `disable_startup_delay` | `Serialize\StartupDelayInMSec=0` | R | não | os programas de inicialização abrem sem esperar; é boot, não jogo | **COND**, na seção Inicialização |
| `mouse_precision_off` | `MouseSpeed=0`, `MouseThreshold1=0`, `MouseThreshold2=0` | R | não | tira a aceleração do Windows; jogo com raw input não é afetado, jogo sem raw input e a área de trabalho são. É preferência de mira. `mouse.rs` (2.8) já diagnostica isso | **MANTER** dentro de "Caminho do mouse" (juntar com `mouse.rs`) |
| `foreground_priority` | `Win32PrioritySeparation=0x26` | S | sim | 0x26 é praticamente o padrão de estação de trabalho (quantum curto, variável, reforço 3×). Não há ganho reproduzível | **REMOVER** |
| `disable_sysmain` | serviço SysMain desativado | R | não | o Windows gerencia sozinho; em SSD o efeito some; em HD pode deixar abrir programas mais lento | **REMOVER** |
| `disable_hibernation` | `powercfg /hibernate off` | S | não | libera o tamanho da RAM em disco (`hiberfil.sys`). Espaço, não desempenho | **COND**, na aba Espaço |
| `disable_power_throttling` | `PowerThrottlingOff=1` | S | sim | ver achado grave nº 1 | **REMOVER** |
| `disable_memory_compression` | `Disable-MMAgent -MemoryCompression` | S | sim | com 8 GB piora (mais paginação); com 32 GB não muda nada | **REMOVER** |
| `gpu_msi_mode` | `MSISupported=1` na chave da GPU | S | sim | a maioria dos drivers atuais já usa MSI; a função existente só age se estiver desligado, e só na GPU | **EXPER** (Expert), só se desligado, com backup |
| `nic_power_saving_off` | economia de energia da placa de rede desligada | S | não | real quando a placa dorme e perde pacote | **COND**: só se o teste de perda de pacote acusar |
| `background_apps_off` | `GlobalUserDisabled=1`, `BackgroundAppGlobalToggle=0` | R | não | efeito pequeno, e apps da Loja param de notificar | **MANTER** fora do lote (já está) |
| `delivery_optimization_off` | `DODownloadMode=0` (sem troca P2P) | S | não | evita que o PC envie atualizações para estranhos durante a partida | **COND**: interferência de rede |
| `clear_boot_limits` | remove `numproc`/`truncatememory` e afins do BCD | M | sim | desfaz limite deixado por "tweaker" (processador ou memória cortados) | **MANTER** (recuperação P0) |
| `disable_vbs` | `EnableVirtualizationBasedSecurity=0`, HVCI `Enabled=0`, `hypervisorlaunchtype off` | M | sim | ganho real em parte dos jogos presos na CPU; custo de segurança real | **EXPER** (Expert), com aviso e antes e depois obrigatórios |
| `disable_search_indexing` | serviço WSearch desativado | R | não | a busca fica lenta; ganho só em HD | **EXPER** (Expert) |
| `disable_transparency` | `EnableTransparency=0` | R | não | custo mínimo em iGPU fraca | **COND** (área de trabalho) |
| `clean_update_cache` | apaga `SoftwareDistribution\Download` | R | não | duplicado de `limpeza.rs` ("windows_update") | **REMOVER** do catálogo (fica em Limpeza) |
| `disable_reserved_storage` | `Set-WindowsReservedStorageState Disabled` | R | não | espaço, não desempenho | **COND**, na aba Espaço |
| `remove_forced_hpet` | remove `useplatformclock` que outro programa forçou | M | sim | devolve o relógio que o Windows escolhe; forçar HPET custa desempenho | **MANTER** (recuperação) |
| `mmcss_games` | `Tasks\Games`: `GPU Priority=8`, `Priority=6`, `Scheduling Category=High`, `SFIO Priority=High` | S | sim | só afeta programa que se registra na tarefa MMCSS "Games" — quase nenhum jogo | **REMOVER** |
| `stop_sponsored_apps` | quatro flags do `ContentDeliveryManager` = 0 | R | não | impede o Windows de instalar apps patrocinados em segundo plano | **MANTER** (debloat leve) |
| `disable_widgets` | política `AllowNewsAndInterests=0` | R | não | tira o processo de Widgets (RAM) | **COND**: com pressão de memória |
| `notifications_off` | `ToastEnabled=0` (todas as notificações, sempre) | R | não | o Windows 11 já ativa o "não perturbe" sozinho durante o jogo | **REMOVER** |
| `store_auto_download_off` | política `AutoDownload=2` | S | não | evita download em segundo plano; os apps da Loja ficam desatualizados | **COND** (opt-in) |
| `error_reporting_off` (2.8) | `WER\Disabled=1` | R | não | tira os segundos de PC parado quando um jogo cai; perde o registro da falha | **COND** (opt-in, com o preço escrito) |
| `edge_background_off` (2.8) | políticas `StartupBoost=0`, `BackgroundModeEnabled=0` | R | não | RAM; o Edge passa a mostrar "gerenciado pela organização" | **COND**: com pressão de memória |
| `accessibility_keys_off` | flags de Teclas de Aderência e Filtragem | R | não | evita o pop-up de acessibilidade no meio da partida | **MANTER** |
| `clean_temp_files` | apaga `%TEMP%` | R | não | duplicado de `limpeza.rs` ("temporarios") | **REMOVER** do catálogo |
| † `network_low_latency` | Nagle (`TcpAckFrequency`, `TCPNoDelay`) | S | sim | já retirado | só desfazer |
| † `disable_xbox_services`, † `disable_telemetry`, † `telemetry_policy`, † `maps_auto_update_off`, † `settings_sync_off`, † `remote_assistance_off`, † `start_menu_web_search_off`, † `disable_copilot` | serviços e políticas | R/0 | — | já retirados | só desfazer |

**Saldo do catálogo (32 ativos):** 6 ficam (um deles, o do mouse, fundido na tela "Caminho do mouse"), 12 viram condicionais (só aparecem quando o diagnóstico pede), 4 ficam no Expert como experimentais, 1 é refeito e **9 saem**: `system_responsiveness_gaming`, `foreground_priority`, `disable_sysmain`, `disable_power_throttling`, `disable_memory_compression`, `mmcss_games`, `notifications_off` e os duplicados `clean_update_cache` e `clean_temp_files`.

---

## 3. O que altera o sistema fora do catálogo

| Recurso | Arquivo | O que altera | Reversível | Avaliação | Decisão |
|---|---|---|---|---|---|
| Ajustes globais NVIDIA | `nvdriver.rs` | perfil GLOBAL: energia "desempenho máximo", pré-renderizados = 1, filtro de textura "desempenho", **V-Sync forçado desligado**, cache de shader ligado | sim (restaura o padrão da NVIDIA) | global demais: energia máxima mantém a placa acordada até na área de trabalho; V-Sync desligado quebra G-SYNC (achado 3); cache de shader já é ligado de fábrica | **REFAT**: por jogo, com valor atual e novo na tela, perfis PADRÃO / COMPETITIVO / BAIXA LATÊNCIA / QUALIDADE; V-Sync só com VRR detectado |
| Limite de FPS por jogo | `nvdriver.rs` | FRL no perfil do executável | sim | a pessoa escolhe; o Otimiza nunca limita sozinho | **MANTER** |
| GPU preferida por jogo | `gpupref.rs` | `UserGpuPreferences` do executável | sim | real em notebook com duas placas | **MANTER** |
| Taxa do monitor | `display.rs` | `ChangeDisplaySettingsEx` com teste antes | sim | o maior ganho de fluidez quando está errado | **MANTER** (P0) |
| Configuração do jogo FiveM/GTA | `configjogo.rs` | arquivo `settings.xml` inteiro | sim, byte a byte | o maior ganho de FPS medido; já está sob o portão "nunca menos FPS" | **MANTER** |
| Configuração de jogos Unreal | `unreal.rs` | `GameUserSettings.ini` (só `sg.*`, só desce) | sim, byte a byte | provado em teste, não num jogo Unreal instalado | **MANTER**, validar num jogo real |
| Governador (modo jogo) | `governador.rs` | prioridade abaixo do normal + EcoQoS em programa de fundo que usa CPU; devolve ao fechar | sim, com recuperação se o Otimiza fechar no meio | ataca a disputa de CPU sem congelar nada; ao vivo, acalmou e devolveu um processo real | **MANTER** — e **REFAT**: mostrar ao vivo quem foi acalmado (processo, CPU, RAM) |
| **Priorizar o jogo (FiveM)** | `fivem.rs` · `priorizar_jogo` | prioridade Alta no FiveM | sessão | achado grave 2 | **REMOVER** |
| Prioridade fixa (IFEO) | `mod.rs` | `PerfOptions\CpuPriorityClass` | sim | já só permite remover | manter só o remover |
| Afinidade (2.8, aba Núcleos) | `afinidade.rs`, `nucleos.rs`, `topologia.rs` | `SetProcessAffinityMask` no processo aberto; topologia P/E lida do Windows | sessão | a leitura de topologia é correta; a aplicação é manual e sem medição | **REFAT** → "Auto CPU Set" com perfis (Automático / só P-cores / P+E / Custom) e antes e depois obrigatório |
| Plano "desempenho máximo" | `readiness.rs` · `criar_plano_maximo` | duplica o Ultimate Performance | parcial | receita universal, e compete com o motor de energia | **REMOVER** |
| Ligar TRIM | `readiness.rs` · `ligar_trim` | `fsutil behavior set disabledeletenotify 0` | sim | TRIM desligado é erro de configuração de verdade | **MANTER** |
| Serviços de terceiros → manual | `servicesaudit.rs` | tipo de início | sim | a pessoa escolhe, item a item | **MANTER** |
| Religar serviços essenciais | `essenciais.rs` | tipo de início | sim | conserta Windows "lite" quebrado | **MANTER** |
| Tarefas agendadas de terceiros | `tasks.rs` | liga/desliga | sim | a pessoa escolhe | **MANTER** |
| Inicialização | `startup.rs` | `StartupApproved` | sim | com classes (essencial / útil / opcional) desde a 2.9 | **MANTER** |
| MSI da GPU / economia da placa de rede | `devices.rs` | registro por dispositivo | sim | ver catálogo | **EXPER** / **COND** |
| Motor de energia | `motorenergia*.rs` | configurações do plano, com teste e escolha medida | sim, com recuperação de teste interrompido | o caminho de gravação **nunca rodou como administrador numa máquina real** | **MANTER**, validar como admin |
| Plano OTIMIZA / `power.rs` | `planoenergia.rs`, `power.rs` | receita | sim | três sistemas de energia | **REFAT**: fundir no motor |
| Gerador de quadros | `geracao/` | nada no sistema (sobreposição) | — | aumenta quadros exibidos, com guardas | **MANTER** |
| Limpeza do sistema (2.8) | `limpeza.rs`, `limpar.rs` | apaga temporários, cache do WU, cache de entrega, miniaturas, relatórios de erro, lixeira (desmarcada) | não (apagar) | honesta, cada item diz o que se perde | **MANTER** — e **fundir** o Liberador de espaço nela |
| Liberador de espaço | `diskspace.rs` | DISM, temporários e afins | não | duplicado da Limpeza | **REFAT** → fundir |
| Cache do navegador | `browsers.rs` | apaga o cache de um navegador | não | é espaço, não desempenho; hoje fica num painel que parece otimização | **REFAT**: mover para Limpeza |
| Cache de shader | `shaders.rs` | apaga o cache de shader | não | **apagar causa engasgo nas primeiras partidas**; só se justifica com cache obsoleto em relação ao driver | **COND**: só com o cache obsoleto detectado, e com o aviso |
| Cache do FiveM | `fivem.rs` · `limpar` | apaga caches do FiveM | não | conserto de crash e de disco cheio | **COND** (solução de problema) |
| Instalar programas (winget) | `programas.rs`, `winget.rs` | instala pelo gerenciador do Windows | pelo desinstalador | utilidade do técnico, não otimização | **MANTER**, fora do fluxo de jogo |
| Ponto de restauração | `restore.rs` | cria ponto | — | rede de segurança | **MANTER** |
| SFC / DISM | `reparo.rs` | repara o sistema | — | real quando há corrupção | **MANTER** |
| Retomar suspensos | `suspend.rs`, `sessao.rs` | devolve o que versões antigas congelaram | — | legado necessário | **MANTER** até sumir da base instalada |

---

## 4. Diagnóstico e medição — o que existe

| Recurso | Onde | Estado |
|---|---|---|
| Quadros por ETW (FPS, 1%, 0,1%, P95, P99, travadas) | `frames.rs`, `core/fluidez.rs` | **bom**; conferido ao vivo no Roblox (59,5 FPS) |
| Classificador de gargalo (CPU inteira, um núcleo, GPU, RAM, VRAM, disco, limite térmico, limite elétrico, teto, travada, fora do hardware, streaming, rede) | `gargalo.rs` (2.8) | **bom**, e agora é o único |
| Mapa de desempenho + detetive de travadas | `mapa.ts`, `core/janela.rs`, `core/travadas.rs` | bom — é o "FPS Analyzer" que o pedido descreve |
| Telemetria | `monitor.rs` + `telemetry.rs` (2.8) **e** `core/telemetria.rs` (2.9) | **duplicado** — duas coletas por PDH |
| Memória de vídeo com piso de transbordo | `vram.rs` (2.8) | bom |
| Latência "do clique ao pixel" | `latencia.rs` (2.8) | piso honesto, sem inventar total |
| Antes e depois | `benchmark.rs`, `prova.rs`, `experimento.rs`, `baseline.rs` + `repeticoes.rs` (2.8), `motorenergia`, `framegen::comparar`, `portao.rs` | **muito duplicado** — sete caminhos |
| Histórico / "quando piorou" | `historico.rs` (2.8), `regressao.rs`, `deriva.rs` (2.9) | **duplicado** |
| Porque o FPS está baixo | `bottleneck.rs`, `causas.rs`, `veredito.rs` + desempenho perdido | três painéis para a mesma pergunta |
| Térmico | `thermal.rs`, flags de firmware no PDH, clock efetivo | existe em pedaços — falta a frase "seu FPS está limitado por temperatura" num lugar só |
| Temperatura e clock da GPU | — | **não existe** (`gpu.clock`, `gpu.power` = desconhecido) |
| DPC/ISR | — | **não existe** |
| Driver (versão e idade) | `shaders.rs` lê a idade; `core/telemetria` lê a versão | sem tela própria |
| AMD GPU | — | **não existe** (só NVIDIA) |
| Modo Expert | — | **não existe**: tudo aparece para todo mundo |
| Assistente com IA | ícone no `index.html`, sem função | **não existe** |

---

## 5. Código duplicado e legado

1. **Sete caminhos de antes e depois.** A regra estatística já é uma só (`repeticoes.rs`), mas as telas são sete. Deve haver **um** fluxo: baseline → aplicar → medir → comparar → manter ou desfazer.
2. **Três sistemas de energia:** `power.rs`, `planoenergia.rs`, `motorenergia`. Somem com o `criar_plano_maximo`.
3. **Duas telemetrias:** `monitor.rs` e `core/telemetria.rs`.
4. **Três "por que o FPS está baixo"** mais o Mapa.
5. **Três limpadores:** Liberador de espaço, Limpeza do sistema e os itens de limpeza do catálogo.
6. **Painéis ao vivo repetidos no Painel:** os "vivos" da 2.8 no topo e, mais abaixo, "Uso agora", "Quem está pesando", "Carga por núcleo", "Histórico de CPU" e "Movimento agora".
7. **Mouse:** o item de catálogo `mouse_precision_off` e o diagnóstico `mouse.rs`.
8. **Três escritores de histórico:** `regressao.rs`, `historico.rs` e `deriva.rs`.

---

## 6. O que está bom e fica

Foram medidos ou conferidos em máquina real:

- **ETW de quadros** e o **classificador de gargalo** da 2.8;
- **Mapa + detetive de travadas**;
- **limites escondidos**, inclusive os 60 FPS do Roblox;
- **configuração de jogo com desfazer byte a byte**, e o **portão "nunca menos FPS"**;
- **governador** (ao vivo);
- **taxa do monitor**;
- **GPU preferida**;
- **recuperação P0**: boot limitado, HPET forçado, TRIM;
- **transação com diário** (2.8), que resolve o requisito de *fail safe* do pedido;
- **gerador de quadros** com as guardas;
- **topologia P/E** lida do Windows.

---

## 7. Plano

### KEEP

Tudo marcado MANTER nas seções 2 e 3.

### REMOVE

- **Nove itens do catálogo** (sete sem ganho e dois duplicados): `system_responsiveness_gaming`, `foreground_priority`, `disable_sysmain`, `disable_power_throttling`, `disable_memory_compression`, `mmcss_games`, `notifications_off`, `clean_temp_files`, `clean_update_cache`.
- `fivem::priorizar_jogo` (botão "Priorizar o jogo").
- `readiness::criar_plano_maximo`.
- `power.rs` como sistema próprio.
- **Painéis ao vivo repetidos:** Uso agora, Quem está pesando, Carga por núcleo, Histórico de CPU, Movimento agora — o topo "vivos" e o Mapa cobrem os mesmos dados.

Todo item retirado mantém o desfazer para quem já aplicou, como os 9 da primeira rodada.

### REWORK

1. **Motor único de otimização**, o que o pedido chama de *Optimization Engine*: toda escrita no sistema — catálogo, NVIDIA, jogo, monitor, afinidade, serviços, tarefas, inicialização — passa a ser uma `Alteracao` com `detectar / pode_aplicar / aplicar / verificar / desfazer`, com risco, confiança, reinício e o que muda (valor atual → valor novo). Hoje o catálogo já tem isso; os outros escrevem por fora.
2. **Um fluxo de antes e depois** no lugar dos sete: baseline → aplicar → medir → comparar → manter ou desfazer (auto-desfazer opcional), usando `repeticoes.rs` e o portão.
3. **NVIDIA por jogo**, com valor atual e novo na tela, nos perfis PADRÃO, COMPETITIVO, BAIXA LATÊNCIA, QUALIDADE e CUSTOM. V-Sync só com VRR detectado.
4. **Afinidade → Auto CPU Set**: perfis Automático / só P-cores / P+E / Custom por jogo, aplicados só no jogo, sempre com antes e depois e o resultado guardado no perfil do jogo.
5. **Energia**: o motor adaptativo como sistema único; o plano OTIMIZA vira o molde dele.
6. **Limpeza** única, e o cache de shader só com o cache obsoleto detectado.
7. **Painel (Home)**: estado do PC, "Pronto para jogar?", processos pesados, recomendações, último benchmark por jogo e último ganho medido.
8. **Página do jogo**: gargalo, último benchmark, configuração recomendada, e um botão **"Otimizar e testar"** no lugar de "aplicar N ajustes".
9. **Itens condicionais**: só aparecem quando o diagnóstico acusa. Por exemplo, o Game DVR só se estiver ligado, e a placa de rede só se houver perda de pacote.

### ADD

1. **Modo Simples / Expert**: o Simples mostra "Analisar meu PC", "Otimizar jogo", "Testar desempenho" e "Reverter"; o Expert mostra registro, serviços, afinidade, MSI e driver.
2. **Placa de vídeo com sensores (NVML)**: temperatura, clock, potência e limites da NVIDIA. Fecha o "limitado por temperatura" do lado da GPU.
3. **Diagnóstico térmico num lugar só**: "seu FPS está limitado por temperatura" (CPU e GPU), com clock efetivo, flags de firmware e NVML.
4. **Diagnóstico de DPC/ISR** por ETW do kernel: qual driver causa pico de latência. Só diagnóstico, nada escrito.
5. **Driver**: versão e data instaladas e a origem, sem prometer que o mais novo é mais rápido, com um link para o fabricante. **Instalação limpa e "NVIDIA debloat" ficam fora desta versão**: são instaladores de terceiros com alto risco de quebrar áudio HDMI, NVENC e afins, e não passam no critério "como desfazer?".
6. **MSI por dispositivo** (Expert): lista os dispositivos com suporte e estado; só age se estiver desligado, com backup.
7. **Relatório de alterações exportável**: data, item, valor anterior, valor novo, resultado e desfazer. O diário e o changelog já guardam tudo; falta a tela.
8. **AMD GPU**: fica para depois. Exige uma integração com ADLX que ainda não existe, e esta máquina não tem placa AMD para validar.

### Não entra

- **IA integrada:** mandaria dados do PC para fora, e a regra de "telemetria local" exige decisão sua antes (ver pergunta 3).
- **NVIDIA debloat e instalação de driver:** risco alto, sem desfazer confiável.
- **Qualquer "RAM cleaner", limpeza de standby, desligar Defender, desligar Windows Update, TCP/MTU universal, prioridade Tempo Real:** hoje não existem no produto, e continuam de fora.

---

### Andamento (22/09/2026)

| Item | Estado |
|---|---|
| REMOVE — 9 itens do catálogo, "Priorizar" do FiveM, plano máximo, painéis repetidos | feito (`eb55b03`) |
| REWORK 3 — NVIDIA por jogo (Competitivo, Baixa latência), valor atual → novo, nada volta a um valor mais lento | feito (`3cdac45`). Padrão = desfazer. Qualidade e Custom **não** entraram: não há valor documentado que só melhore imagem sem custo. V-Sync fica fora (VRR não é legível pelo driver) |
| REWORK 9 — itens condicionais, medidos nesta máquina | feito (`cac4be6`) |
| ADD 1 — modo Expert | feito como chave na aba de ajustes (`cac4be6`). O modo Simples é o padrão |
| ADD 2 e 3 — sensores da placa e térmico num lugar só | feito por `nvidia-smi` (`fd3dcad`) |
| ADD 4 — DPC/ISR | feito por núcleo, pelos contadores do Windows (`3be8ca4`). **Por driver, não**: exige ETW do kernel |
| ADD 5 — driver | feito (`decc1d2`) |
| ADD 6 — MSI por dispositivo | **só leitura** (`b3cd3d3`). Escrever fora da placa de vídeo não passa no "como desfazer?": driver que não aguenta impede o boot |
| ADD 7 — relatório exportável | feito, planilha + data no PDF (`1ba72b7`) |
| REWORK 1, 2, 4, 5, 6, 7, 8 — motor único, "Otimizar e testar", Auto CPU Set, energia única, limpeza única, Home, página do jogo | pendente |

## 8. Matriz de testes — o que dá e o que não dá para validar

| Configuração | Disponível | Como |
|---|---|---|
| Esta máquina: Win10 19045, GTX 1650, 8 threads, 8 GB em canal único, monitor 180 Hz, Windows "lite" | sim | testes ao vivo (ETW, governador, limites) |
| Máquina da 2.8 (i9-14900HX híbrido, pelo comentário de `nucleos.rs`) | com você | Auto CPU Set, P/E cores |
| Windows 11, AMD Ryzen, GPU AMD, notebook Optimus, 16 e 32 GB | não | `labcompat.rs` coleta o relatório do cliente; os testes puros cobrem a regra, mas a validação real depende dessas máquinas |
| Jogos: CS2, Valorant, Fortnite, Warzone | não instalados aqui | Roblox, FiveM e GTA V estão |

Tudo o que não puder ser visto numa máquina real sai marcado como **"não validado"** nas notas da versão, como já é feito hoje.

---

## 9. Perguntas antes de começar

1. **"Ultimus"**: é um nome novo para o produto (renomear o Otimiza), ou foi só o nome no pedido?
2. **Publicação**: a 2.9.0 atual (pronta e compilada) sai agora, e isto vira a 2.10? Ou isto entra na própria 2.9 antes de publicar?
3. **IA**: se for entrar um dia, pode enviar o resumo do diagnóstico para um serviço externo, com aviso claro na tela, ou tem que ser só local?
