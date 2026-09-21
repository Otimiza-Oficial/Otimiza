# Otimiza 2.9 — auditoria e plano (entrega antes de qualquer código)

> Estado auditado: `main` em `089e295` (2.7.0 publicada, com 3 instaladores no
> release). 65.691 linhas de Rust em 88 arquivos, 14.289 de interface
> (`index.html` + 7 `.ts`), 1.106 testes. Nove abas: Painel, Diagnóstico,
> Otimizações, Energia, Jogos, Geração de quadros, Sistema, Espaço, Reparo.
>
> **Nenhuma linha de código foi alterada para produzir este documento.**

---

## 0. Resumo em uma tela

O pedido da 2.9 é **menos otimizador e mais diagnóstico que prova**. Mais da
metade das peças já existe: ETW de quadros, análise de gargalo, leitura de
BIOS/XMP/PCIe/ReBAR, eventos de memória, histórico reversível, A/B por grupo,
regressão automática, motor de energia adaptativo, gerador de quadros próprio.

**O problema não é faltar peça. É que as peças não conversam.** Hoje existem:

- **4 sistemas de medição de antes/depois:** `benchmark.rs`, `prova.rs`,
  `experimento.rs` e `motorenergia` (mais `framegen::comparar`).
- **3 sistemas de energia:** `power.rs`, `planoenergia.rs` e `motorenergia*.rs`.
  O modo jogo ainda liga o "Alto Desempenho" fixo e passa por cima do perfil
  que o motor validou.
- **3 classificadores de causa:** `bottleneck.rs`, `causas.rs` e `veredito.rs`,
  com `achados.rs` servindo de vocabulário.
- **2 detectores de conflito:** `conflicts.rs` e `conflitos.rs`.
- **1 monitor que não monitora:** `monitor.rs`. `GPUMetrics` devolve
  `"not yet implemented"` e `monitoring_active` nunca é lido.

A 2.9 é, antes de tudo, **consolidação num núcleo comum**. Depois vem o que
falta de verdade:

- P95/P99, 0,1% e a severidade dos engasgos;
- telemetria de GPU e VRAM;
- janela de contexto em volta do engasgo;
- base de desempenho por jogo;
- estatística com repetição A-B-A-B.

**Das 41 entradas do catálogo:** 11 saem, 6 vão para Expert, 8 viram
adaptativas (só entram quando um diagnóstico pede) e 4 vão para a aba
Espaço. 2 são absorvidas por outro sistema (energia e mira) e 1 é reescrita.
Só **9 continuam no caminho de desempenho** como estão. A categoria
Privacidade deixa de existir: nada nela mudava FPS ou fluidez.

**Revisão 2 (depois da sua resposta):**

- **Todo jogador ganha igual.** Não existe mais "RP" contra "PvP", nem
  "competitivo" contra "equilibrado" decidindo quem ganha o quê (seção 14).
- **Saiu tudo que não ajuda:** 11 ajustes, 7 painéis e 3 módulos (seção 15).
- **No lugar, entram 12 alavancas que mexem de verdade** em FPS, 1% low e
  engasgo, valendo para qualquer jogo (seção 16).
- **A biblioteca passa a cobrir todos os jogos,** e não só o FiveM
  (seção 17).

---

## 1. Arquitetura atual

```
index.html + main.ts (8.841 linhas)  energia.ts  framegen.ts  pilares.ts  esfera.ts
            │  invoke (commands.rs, 4.895 linhas; LIVRES / EXIGEM_LICENCA)
            ▼
┌──────────────────────── modules/ ────────────────────────────────────────┐
│ licença · máquina · preferências · atualização · convite                 │
│ medição: benchmark · jitter · prova · medicoes · regressao · pontuacao   │
│ changelog (desfazer) · safety · report · monitor(stub)                   │
│ windows/                                                                  │
│  catálogo + mod.rs (aplicar/desfazer 41 itens) · niveis · profiles · grupos│
│  diagnóstico: hardware firmware bios cpugeracao pcie rbar display thermal│
│               health memory exhaustion pressao bottleneck causas veredito│
│               conflicts conflitos readiness essenciais contadousuario     │
│  jogo: deteccao jogos gamemode frames configjogo citizenfx fivem shaders │
│        discodojogo nvdriver gpupref anticheat framegen + geracao/ (GPU)  │
│  energia: power · planoenergia · motorenergia · motorenergia_maquina     │
│  sistema: processes startup servicesaudit services tasks bloatware       │
│           browsers sysparams acessibilidade network rede                 │
│  espaço/reparo: diskspace foldermap cleanup reparo cbslog tarefa_longa   │
│  base: registry shell restore suspend sessao suporte cabecalho labcompat │
└───────────────────────────────────────────────────────────────────────────┘
lib.rs: vigias em segundo plano (jogo a cada 6 s, pressão de memória,
        medição automática, energia dinâmica, recuperação de teste interrompido)
```

O que já está certo e **é a base da 2.9**:

- **Reversibilidade:** `changelog.rs` grava o valor anterior de cada mudança.
- **Honestidade embutida:** `ExpectedGain::{Measurable, Responsiveness,
  Situational, NoGain}` já classifica o catálogo.
- **Travas em teste:** comando sem classificação de licença reprova o build,
  e há guardas de prosa, de cor e de "fora do lote".
- **Medição sem injeção:** os quadros vêm de ETW (`frames.rs`).

---

## 2. Auditoria item a item

Legenda das decisões:

- **KEEP** — continua como está.
- **IMPROVE** — continua, com melhoria.
- **REWRITE** — é refeito.
- **MERGE** — é fundido com outro sistema.
- **ADAPTIVE** — só entra quando um diagnóstico pede e uma medição confirma.
- **EXPERT** — some da tela normal e fica no modo Expert.
- **REMOVE** — sai do produto.

Suporte comum a todos os itens: Windows 10/11 x64.

### 2.1 Catálogo de ajustes (41 itens)

"Mede?" diz se existe hoje um jeito de provar o efeito dele nesta máquina.
Todos têm desfazer, exceto as limpezas, que não voltam por natureza.

| Item | Benefício real | Regressão possível / risco | Mede? | Decisão |
|---|---|---|---|---|
| `plano_otimiza` | depende da CPU | calor, e FPS em notebook | sim (motor) | **MERGE** no Power Engine; deixa de ser receita fixa |
| `disable_gamedvr` | alto quando a gravação está ligada | nenhuma | sim | **KEEP**, como *recuperação* (Game Bar gravando) |
| `gpu_hardware_scheduling` | depende de placa e driver; é pré-requisito do DLSS FG | stutter em driver antigo; exige reinício | A/B | **ADAPTIVE** |
| `system_responsiveness_gaming` | sem evidência sólida | baixa | A/B difícil | **EXPERT** |
| `mmcss_games` | idem | áudio pode picotar | A/B difícil | **EXPERT** |
| `foreground_priority` (Win32PrioritySeparation) | pequeno, controverso | stutter em algumas CPUs | A/B | **EXPERT** + A/B obrigatório |
| `network_low_latency` (Nagle) | quase nulo: a maioria dos jogos usa UDP | nenhuma | não | **REMOVE** (tweak de rede sem medir rede) |
| `nic_power_saving_off` | real quando a placa dorme e perde pacote | nenhuma | sim (`rede.rs`) | **ADAPTIVE**: só se o teste de rede acusar |
| `mouse_precision_off` | é preferência, não desempenho | muda a sensação da mira | — | **MERGE** no AimTracking Lab |
| `disable_xbox_services` | ~0: os serviços são sob demanda | quebra Game Pass e alguns jogos | não | **REMOVE** |
| `disable_sysmain` | positivo em HD e com pouca RAM; negativo em SSD com RAM folgada | abertura de apps mais lenta | sim (responsividade) | **ADAPTIVE**: só com HD e pressão |
| `disable_hibernation` | espaço em disco | perde a inicialização rápida | — | **MOVE** para Espaço (não é desempenho) |
| `disable_power_throttling` | real em notebook e CPU híbrida | consumo | A/B | **ADAPTIVE** (notebook na tomada, híbrida) |
| `disable_memory_compression` | positivo com RAM folgada; **negativo com 8 GB** | falta de memória | sim (pressão) | **ADAPTIVE**: só com RAM folgada e medido |
| `gpu_msi_mode` | a maioria dos drivers já usa MSI | tela preta se o driver não suportar | não | **EXPERT** |
| `background_apps_off` | pequeno | notificações de apps somem | não | **KEEP** fora do lote (já está) |
| `start_menu_web_search_off` | responsividade do Iniciar | nenhuma | parcial | **REMOVE**: não muda FPS nem fluidez |
| `delivery_optimization_off` | real: upload em segundo plano | nenhuma | sim (interferência de rede) | **ADAPTIVE**, dentro de Interferência de Rede |
| `clear_boot_limits` | **alto quando existe** (numproc/truncatememory) | nenhuma | sim | **KEEP**, como recuperação (P0) |
| `disable_vbs` | medível (5–10% em alguns jogos) | **segurança** | A/B | **EXPERT**, com a troca escrita e A/B obrigatório |
| `disable_search_indexing` | pequeno, mais em HD | a busca some | parcial | **EXPERT** |
| `disable_transparency` | pequeno, em iGPU fraca | visual | parcial | **KEEP** no Desktop Fast |
| `visual_effects_performance` | responsividade em PC fraco | visual | parcial | **KEEP** no Desktop Fast |
| `disable_startup_delay` | inicialização | nenhuma | sim (boot) | **KEEP** no Desktop Fast |
| `clean_update_cache` | espaço | não volta | espaço liberado | **MOVE** para Espaço |
| `disable_reserved_storage` | espaço | atualização pode falhar com o disco cheio | espaço | **MOVE** para Espaço |
| `remove_forced_hpet` | **alto quando alguém forçou** | nenhuma | sim | **KEEP**, como recuperação (P0/P1) |
| `stop_sponsored_apps` | sossego | nenhuma | não | **KEEP**, no Background Governor: impede instalação em segundo plano |
| `disable_widgets` | pequeno (RAM) | nenhuma | parcial | **ADAPTIVE**: só com pressão de memória medida |
| `disable_copilot` | pequeno | nenhuma | não | **REMOVE**: não muda FPS nem fluidez |
| `disable_telemetry` + `telemetry_policy` | ~0 em FPS | nenhuma | não | **REMOVE** os dois |
| `notifications_off` | evita interrupção no jogo | perde avisos | não | **REWRITE**: não perturbar só *durante* o jogo |
| `store_auto_download_off` | real (download em segundo plano) | apps desatualizados | sim (interferência) | **ADAPTIVE**, no Background Governor |
| `maps_auto_update_off` | nenhum (NoGain) | — | — | **REMOVE** |
| `settings_sync_off` | nenhum (NoGain) | — | — | **REMOVE** |
| `remote_assistance_off` | nenhum (NoGain) | — | — | **REMOVE** |
| `uac_off` | **nenhum** (NoGain) | **segurança** | — | **REMOVE** |
| `firewall_off` | **nenhum** (NoGain) | **segurança** | — | **REMOVE** |
| `accessibility_keys_off` | evita o Alt+Tab acidental no jogo | nenhuma | — | **KEEP** |
| `clean_temp_files` | espaço | não volta | espaço liberado | **MOVE** para Espaço |

Os removidos continuam no `naofazemos.rs`, que ganha uma entrada por item
dizendo por quê. É ali que o cliente que compara listas encontra a resposta.
O desfazer de quem já os aplicou **continua funcionando**: o histórico
guarda o id, e a remoção tira só o *aplicar*.

### 2.2 Módulos

| Módulo | Propósito | Benefício real | Decisão | Para onde vai |
|---|---|---|---|---|
| `monitor.rs` | métricas ao vivo | CPU/RAM via `sysinfo`; GPU **stub** | **REWRITE** | Telemetry Core |
| `benchmark.rs` + `jitter.rs` | nota antes/depois de sistema | real, mas uma rodada só | **MERGE** | Benchmark Engine (repetição e validade) |
| `prova.rs` + `medicoes.rs` + `regressao.rs` | antes/depois no jogo, automático, regressão | **forte**: é o que achou o incidente da 2.1 | **MERGE** | Benchmark + Regression + Performance DB |
| `experimento.rs` + `grupos.rs` | A/B por grupo | forte | **MERGE** | Self-Tuning Engine (vira o motor genérico) |
| `pontuacao.rs` | nota que pesa o 1% | bom princípio | **MERGE** | Game Fluidity Index |
| `frames.rs` | FPS, 1% e engasgos via ETW | **forte** | **IMPROVE** | P95/P99, 0,1%, severidade, série para o Stutter Detective |
| `bottleneck.rs` + `causas.rs` + `veredito.rs` + `achados.rs` | por que o FPS está baixo | forte (já olha o núcleo mais carregado) | **MERGE** | Bottleneck Classifier + Decision Engine; `achados` vira o tipo comum |
| `hardware.rs` `cpugeracao.rs` | perfil da máquina | base de tudo | **KEEP** | Hardware Fingerprint |
| `firmware.rs` `bios.rs` `rbar.rs` `pcie.rs` | XMP, canais, ReBAR, faixas PCIe | **alto (recuperação)** | **KEEP / IMPROVE** | Performance Recovery + BIOS Analyzer; só leem, nunca escrevem |
| `display.rs` | Hz e resolução | **o maior P0 que existe** | **IMPROVE** | Display Engine (VRR, HDR, caminho da GPU) |
| `thermal.rs` `health.rs` | throttling e disco morrendo | alto quando existe | **IMPROVE** | Thermal / Storage |
| `memory.rs` `exhaustion.rs` `pressao.rs` | paginação, eventos 2004, janela de pressão | **forte e com prova do Windows** | **KEEP** | Memory Intelligence |
| `gpupref.rs` | GPU certa no notebook | **alto em notebook híbrido** | **KEEP** | Hybrid Laptop Engine |
| `nvdriver.rs` | ajustes NVAPI e limitador por jogo | real | **KEEP** | Render Orchestrator (limite de FPS, V-Sync) |
| `configjogo.rs` `citizenfx.rs` `fivem.rs` | configuração gráfica do GTA/FiveM | **a maior alavanca de FPS medida** (MSAA) | **IMPROVE** | Smart Graphics Tuner (hoje só FiveM/GTA) |
| `shaders.rs` `discodojogo.rs` | cache de shader, jogo em HD | real | **KEEP** | Shader Profiler / Asset Streaming |
| `framegen.rs` + `geracao/` | laboratório de FG e gerador próprio | real (quadros exibidos) | **IMPROVE** | FG Engine: prontidão, casar com o Hz, guarda de latência |
| `motorenergia*.rs` | motor adaptativo | real | **KEEP**, e vira o *único* sistema de energia | Power Engine 2.9 |
| `planoenergia.rs` | o plano OTIMIZA de lista fixa | legado | **MERGE → REMOVE** | a descoberta de configurações e o backup migram para o motor |
| `power.rs` | trocar plano e "Alto Desempenho" | utilitário | **MERGE** | camada de escrita do Power Engine |
| `gamemode.rs` | modo jogo automático | parcial: liga o Alto Desempenho **fixo** | **REWRITE** | Game Mode 2.9: aplica o perfil *validado* do jogo, ou nada |
| `niveis.rs` `profiles.rs` | Seguro/Competitivo/Experimental; PC fraco/Jogos/... | organização | **MERGE** | Profile Engine (contextos: Desktop, Jogo competitivo, Jogo equilibrado, ...) |
| `processes.rs` `startup.rs` `servicesaudit.rs` `tasks.rs` `bloatware.rs` `browsers.rs` | o que roda em segundo plano | real | **MERGE** | Smart Background Governor + Startup Intelligence |
| `conflicts.rs` + `conflitos.rs` | programas / ajustes brigando | real | **MERGE** | Background Interference |
| `network.rs` | ajustes de rede | quase nenhum | **REWRITE** | Network Quality Engine (mede e não ajusta às cegas) |
| `rede.rs` | perda de pacote até o servidor | **forte** | **KEEP** | Network Quality Engine |
| `anticheat.rs` `contadousuario.rs` `readiness.rs` `essenciais.rs` | onde não mexer; pré-condições | proteção | **KEEP** | Game Readiness Scanner |
| `changelog.rs` `registry.rs` `services.rs` `restore.rs` `safety.rs` | desfazer | base | **IMPROVE / MERGE** | Rollback Engine (+ Best Known, Previous, Windows Default) |
| `suspend.rs` `sessao.rs` | devolvem o que versões antigas suspenderam | rede de segurança | **KEEP** | — |
| `utils/logger.rs` `cabecalho.rs` `suporte.rs` `report.rs` | registro e relatórios | real | **IMPROVE** | Logging 2.9 (valor antigo/novo, resultado do comando e do benchmark) |
| `diskspace.rs` `foldermap.rs` `cleanup.rs` | espaço em disco | real, mas **não é desempenho** | **KEEP** na aba Espaço | — |
| `reparo.rs` `cbslog.rs` `tarefa_longa.rs` | SFC/DISM | real | **KEEP** | — |
| `labcompat.rs` | o que a máquina aceita | real | **MERGE** | Performance DB (assinatura de hardware) |
| `sysparams.rs` `acessibilidade.rs` `devices.rs` | aplicar agora; dispositivos | base | **KEEP** | — |
| `naofazemos.rs` | o que o produto não faz | posicionamento | **IMPROVE** | recebe os 5 removidos |

---

## 3. Legado a remover (resumo)

1. **Os 11 itens do catálogo:** UAC, Firewall, Nagle, serviços do Xbox (este
   desliga serviço sem ganho medido e quebra o Game Pass), Mapas,
   Sincronização, Assistência Remota, as duas de telemetria, busca web do
   Iniciar e Copilot. Nenhum muda FPS ou fluidez. A lista completa, com
   painéis e módulos, está na seção 15.
2. **Desativação de serviço por regra:** nenhum serviço é desligado sem um
   diagnóstico que peça.
3. **`planoenergia.rs` como receita.** Ficam só a descoberta das
   configurações e o backup, dentro do motor.
4. **O "Alto Desempenho" fixo do modo jogo** — é uma receita universal
   aplicada sem medir.
5. **`monitor.rs` stub:** `GPUMetrics`, `start/stop_monitoring` e
   `monitoring_active`.
6. **As medições duplicadas.** Os 4 caminhos de antes/depois viram 1 motor e
   4 consumidores.

Não existe hoje, e continua não existindo:

- desligar núcleo ou E-core;
- afinidade fixa;
- prioridade de tempo real;
- desligar o pagefile;
- limpar a memória em espera;
- mexer em timer por `bcdedit`. O produto só **remove** o HPET forçado por
  terceiros.

---

## 4. Sistemas a melhorar

| Sistema | O que falta hoje | Melhoria |
|---|---|---|
| `frames.rs` | só média, 1% e engasgos/min | P95, P99, 0,1% (só com amostra suficiente), variância, severidade (micro / perceptível / severo / extremo) e série temporal |
| `bottleneck.rs` | classifica um limite de cada vez | vários gargalos ao mesmo tempo; *GPU esperando CPU*; limite de FPS; VRAM; térmico; armazenamento |
| `display.rs` | Hz e resolução | VRR, HDR, "suporta 240 Hz e roda a 60" como **P0**, GPU que dirige a saída (notebook) |
| Motor de energia | escrita nunca rodou ao vivo (sem elevação aqui); sem teste de resposta de boost | teste de resposta (ocioso → carga → clock efetivo → sustentado), margem térmica, contextos |
| Gerador de quadros | liga e desliga | prontidão (Excelente/Bom/Marginal/Não recomendado), multiplicador pelo Hz, guarda de latência no modo competitivo |
| `configjogo.rs` | um preset | custo individual por opção, medido, com orçamento de imagem (Máx. FPS / Equilibrado / Qualidade) |

---

## 5. Sistemas novos da 2.9

| Sistema | Fonte de dado (documentada) | Rótulo de confiabilidade |
|---|---|---|
| **Telemetry Core** | PDH: `Processor Information` (uso por núcleo, `% Processor Performance` → clock efetivo), `GPU Engine` e `GPU Adapter Memory` (uso de GPU e VRAM por processo), `PhysicalDisk` (latência, fila), `Memory` (commit, falhas); DXGI `QueryVideoMemoryInfo` (orçamento de VRAM); NVML para temperatura, clock e potência na NVIDIA | cada métrica é marcada MEASURED / DERIVED / ESTIMATED / UNKNOWN |
| **Baseline Engine** | Telemetry + ETW de quadros + contexto (driver, build, plano, Hz) | — |
| **Statistics Engine** | função pura: média, mediana, desvio, coeficiente de variação, intervalo de confiança | — |
| **Benchmark Engine 2.9** | A-B-A-B, aquecimento, validade (Válido / Questionável / Inválido) | — |
| **Stutter Detective** | buffer circular de ~10 s de telemetria; quando vem um pico, grava antes e depois | a correlação é **HIGH/MEDIUM/LOW**, nunca "causa" |
| **Background Forensics** | CPU, E/S e rede por processo no instante do pico | idem |
| **Performance Recovery Engine** | junta firmware, bios, pcie, rbar, display, thermal, memory, gpupref e nvdriver num relatório P0–P3 | — |
| **Performance DB** | JSON local por (assinatura de hardware, jogo, build, driver) | — |
| **Digital Twin por jogo** | o melhor perfil conhecido de cada jogo (energia, limite, FG, regras de segundo plano) | — |
| **Regression / Drift / Self-Healing** | `regressao.rs` generalizado + detecção de troca de driver ou build | — |
| **Game Readiness + Session Report** | scan leve antes do jogo; relatório depois | — |
| **AimTracking Pro** | Raw Input (`WM_INPUT`) para a taxa de sondagem medida e o jitter | — |
| **Modo Expert** | GUIDs, valores crus, detalhe dos benchmarks | — |

### O que **não dá** para medir honestamente sem driver de kernel

Continua **UNKNOWN**, e a tela diz isso:

- **Temperatura e potência do pacote da CPU.** Exigem ler MSR, o que pede
  driver em kernel. A regra de segurança proíbe instalar driver desconhecido.
  A zona térmica ACPI (WMI) existe em parte das máquinas e mente em outras;
  entra como ESTIMATED.
- **Latência de ponta a ponta,** do mouse ao fóton. Exige hardware de medição
  (sensor na tela). O produto mostra **as partes que mede**: sondagem do
  mouse, fila de apresentação e o atraso do gerador. **Nunca** mostra um
  total.
- **Qualidade de imagem** do upscaler. Não existe métrica automática
  confiável sem uma referência. Fica como avaliação do usuário, com captura
  lado a lado.
- **Ligar DLSS, FSR ou XeSS dentro do jogo.** Só é possível onde o produto
  sabe escrever a configuração do jogo: hoje, só GTA V/FiveM; na 2.9, os
  jogos de nível A da biblioteca (seção 17). Nos outros, o produto **detecta
  e orienta**, não liga.

---

## 6. Top 10 oportunidades de desempenho

Em ordem de P0 a P3. Todas **já têm detector parcial no código**; a 2.9 junta
todas num relatório só e mede o antes e depois.

1. **P0 — Monitor abaixo da taxa que suporta** (`display.rs`). Ganho de
   fluidez maior que qualquer ajuste.
2. **P0 — Boot limitado** (numproc/truncatememory) e **HPET forçado** por
   terceiros.
3. **P1 — Configuração gráfica cara** (MSAA, grama, Post FX no FiveM). É o
   maior ganho de FPS *medido* do produto.
4. **P1 — RAM sem XMP/EXPO ou em canal único** (`firmware.rs`). Orientação de
   BIOS, nunca escrita.
5. **P1 — Notebook usando a GPU integrada** para o jogo (`gpupref.rs`).
6. **P1 — Limite de FPS escondido:** limitador da NVIDIA, V-Sync, RTSS,
   limite dentro do jogo.
7. **P1 — Pressão de memória / paginação** em máquina de 8 GB (`pressao.rs`,
   eventos 2004).
8. **P1 — Throttling térmico ou de potência** no notebook, pelo clock
   efetivo abaixo do nominal sob carga.
9. **P1 — Jogo no HD mecânico** ou disco quase cheio (`discodojogo.rs`,
   `health.rs`).
10. **P2 — Interferência em segundo plano durante o jogo:** atualização,
    sincronização de nuvem, gravação, navegador. Depois vem o perfil de
    energia validado (P2/P3: pequeno em máquina já ajustada, como a sua).

---

## 7. Top 10 riscos técnicos

1. **Custo da telemetria dentro do jogo.** PDH a cada 250 ms custa pouco;
   varredura de processos custa mais. *Mitigação:* orçamento de <0,5% de CPU,
   medido pelo próprio produto, e modo de baixo custo durante o jogo.
2. **Anticheat.** Qualquer leitura por processo fica restrita a contadores do
   sistema. Nada abre handle no jogo (regra que o `anticheat.rs` já aplica).
3. **Estatística honesta custa tempo.** A-B-A-B com 3 repetições de 30 s é
   uma bateria de minutos. *Mitigação:* modo rápido (mostra "confiança
   baixa") e modo completo.
4. **Cena de jogo não repetível** (multiplayer). *Mitigação:* o `deriva_da_cena`
   que já existe marca a rodada QUESTIONÁVEL, e o teste sintético de quadros
   cobre a energia.
5. **Elevação.** Energia, serviços e boot exigem admin; a escrita do motor
   de energia **nunca rodou ao vivo**. *Mitigação:* rodar como admin na sua
   máquina antes de qualquer lançamento.
6. **Consolidar 4 motores de medição sem regredir** os 1.106 testes.
   *Mitigação:* o núcleo novo nasce ao lado; os consumidores migram um por
   commit, com os testes antigos preservados.
7. **Temperatura e potência de CPU inacessíveis** sem driver (ver seção 5).
   *Mitigação:* UNKNOWN, e a decisão térmica usa o clock efetivo.
8. **NVML só na NVIDIA, e AMD sem API estável em Rust.** *Mitigação:* AMD cai
   no PDH (uso e VRAM), com temperatura UNKNOWN até a integração com ADLX.
9. **Windows modificado** ("lite", como o SnyX desta máquina): serviços de
   contadores desligados fazem o PDH falhar. *Mitigação:* toda leitura tem
   três estados (lido / não existe / não consegui ler), a regra que a 2.0
   começou.
10. **Escopo.** O pedido tem mais de 50 sistemas. *Mitigação:* as fases
    abaixo, cada uma publicável sozinha, e a regra "não entra sem medição e
    desfazer".

---

## 8. Nova arquitetura

```
                     ┌──────────── UI (Painel · Mapa · Prontidão · Sessões · Recuperado · Expert)
                     │ invoke
┌────────────────────▼────────────────────────── core/ (novo) ────────────────────────────┐
│ telemetria/  coletores PDH · ETW quadros · DXGI VRAM · NVML · Raw Input · rede           │
│              → Amostra { valor, unidade, Confiabilidade::{Medido,Derivado,Estimado,?} }  │
│ estatistica/ média · mediana · desvio · CV · IC · classificação de ganho  (puro)        │
│ baseline/    retrato do sistema + contexto (driver, build, Hz, plano, temperatura)      │
│ benchmark/   A-B-A-B · aquecimento · validade · deriva da cena                          │
│ gargalo/     classificador multi-rótulo (CPU main thread, GPU, VRAM, cap, térmico...)   │
│ fluidez/     Frame Health · severidade de engasgo · Game Fluidity Index                 │
│ decisao/     Achado{prioridade P0–P3, benefício, risco, confiança, nº de testes}         │
│ rollback/    Changelog + Best Known + Previous + Windows Default + fail-safe             │
│ banco/       Performance DB · Digital Twin por jogo · drift · regressão                 │
│ log/         linha estruturada: módulo, ação, antes, depois, comando, benchmark          │
└──────────▲──────────────▲──────────────▲──────────────▲──────────────▲─────────────────┘
           │              │              │              │              │
   Recovery (firmware,  Power Engine   Render (config  Sistema (backg. Input (AimTracking,
   bios, pcie, display, (motorenergia, jogo, nvdriver, governor, rede, polling, USB,
   memória, térmico)    contextos)     FG, cap, VRR)   storage, boot)  crosshair)
```

**Regra de dependência:** os domínios só dependem de `core/`, nunca uns dos
outros. Só o `decisao/` junta tudo. Com isso, os três classificadores e os
quatro motores de medição de hoje deixam de ser possíveis *por construção*.

---

## 9. Plano de migração

1. `core/` nasce **vazio e ao lado** do código atual. Nenhuma tela muda.
2. `estatistica/` e `telemetria/` entram primeiro, com testes puros.
3. `frames.rs` passa a publicar a série de intervalos no `core`. As contas
   antigas (1%, engasgos) continuam dando o mesmo número: um teste de
   equivalência trava isso.
4. `benchmark` / `prova` / `experimento` / `motorenergia` / `framegen::comparar`
   migram **um por commit** para o `benchmark/` comum.
5. `bottleneck` / `causas` / `veredito` migram para `gargalo/` + `decisao/`.
6. O catálogo recebe o campo `destino` (Desempenho / Adaptativo / Expert /
   Espaço). Os 11 removidos saem do *aplicar*, e o desfazer
   continua.
7. `planoenergia` e `power` são absorvidos pelo motor. O modo jogo passa a
   aplicar o perfil validado do jogo, ou nada.
8. A UI nova é feita por último, em cima de dados que já existem.

---

## 10. Plano de testes

- **Funções puras** (maioria): estatística, classificação de engasgo,
  gargalo multi-rótulo, prioridade P0–P3, validade de benchmark, decisão de
  manter ou reverter, drift. O mesmo padrão dos 1.106 testes atuais.
- **Equivalência:** cada migração prova que o número antigo e o novo são
  iguais para a mesma entrada.
- **Travas novas:**
  - nenhuma métrica sai sem rótulo de confiabilidade;
  - nenhum ganho aparece na tela com validade INVÁLIDA;
  - nenhum item do catálogo tem `NoGain` e entra no caminho de desempenho;
  - UAC e Firewall não podem voltar ao catálogo.
- **Ao vivo nesta máquina** (GTX 1650, FiveM), com você presente:
  - custo da telemetria durante o jogo;
  - A-B-A-B do motor de energia **como admin**;
  - o Stutter Detective pegando um engasgo provocado (cópia de arquivo
    grande durante o jogo);
  - a recuperação P0 simulada (monitor a 60 Hz).
- **Esteira:** os módulos novos entram no `release.yml`, senão o
  `ci_coverage.rs` reprova.

---

## 11. Plano de desfazer

- Toda escrita passa pelo `rollback/` e grava o valor antigo **antes** de
  escrever (regra atual, mantida).
- Botões:
  - **Desfazer a última**;
  - **Restaurar módulo**;
  - **Restaurar o melhor conhecido**;
  - **Restaurar o original** (o estado antes do Otimiza);
  - **Padrão do Windows** — só onde o padrão é documentado.
- **Fail-safe:** o marcador de "operação em andamento" (já existe para o
  teste de energia) passa a valer para **todo lote**. Se o Otimiza morrer no
  meio, a próxima abertura desfaz o parcial.
- **Self-healing:** se o perfil "melhor conhecido" de um jogo piora mais que
  o ruído medido após uma troca de driver ou build, ele **para de ser
  aplicado** e o jogo volta para o estado reavaliar.

---

## 12. Fases de implementação

Cada fase é publicável sozinha e passa pelo portão de aceitação: **só aplica
sozinho o que provou ganho**.

| Fase | Conteúdo | Reaproveita | Tamanho |
|---|---|---|---|
| **1 — Fundação** | `core/telemetria` (PDH, DXGI VRAM, NVML), `estatistica`, `baseline`, `benchmark` A-B-A-B com validade, log estruturado, fail-safe de lote | frames, benchmark, prova, logger, changelog | grande |
| **1b — Biblioteca de jogos** | Lojas novas, fichas de jogo, escritor genérico Unreal, adoção automática de jogo detectado, perfil NVIDIA por jogo (seção 17) | jogos, deteccao, gamemode, configjogo, nvdriver | grande |
| **2 — Recuperação** | Relatório de desempenho perdido (P0–P3), gargalo multi-rótulo, CPU main thread, GPU esperando, VRAM, térmico pelo clock efetivo | firmware, bios, pcie, rbar, display, memory, thermal, bottleneck, causas | média |
| **3 — Fluidez** | Frame Health, P95/P99/0,1%, severidade, Stutter Detective, Background Forensics, Game Fluidity Index | frames, pontuacao, processes | média |
| **4 — Energia** | Motor único, teste de resposta de boost, margem térmica, contextos (Área de trabalho, Jogo, notebook na tomada, notebook na bateria), escrita validada como admin | motorenergia*, power, planoenergia | média |
| **5 — Renderização** | Smart FPS Target, limite por jogo, prontidão e casamento com o Hz do FG, guarda de latência para todos, Graphics Tuner para os jogos da biblioteca | nvdriver, framegen, geracao, configjogo | grande |
| **6 — Sistema** | Background Governor, Startup Intelligence, Network Quality (sem tweak pack), Storage, Desktop Fast | processes, startup, servicesaudit, tasks, rede, health | média |
| **7 — Input** | AimTracking Pro, sondagem medida, caminho USB do dispositivo, correlação com quadros, Crosshair Lab | — (novo) | grande |
| **8 — Inteligência** | Performance DB, Digital Twin, drift, self-healing, melhor conhecido | medicoes, regressao, labcompat | média |
| **9 — UI/UX** | Mapa de desempenho, Prontidão, Timeline da sessão, "Desempenho recuperado", Expert | — | grande |

**Limpeza** (seções 2.1 e 15): entra na Fase 1, porque é barata e imediata.
A **Biblioteca** vem logo depois da Fundação, porque é ela que leva todas
as outras fases para além do FiveM.

---

## 13. Decisões que dependem de você

1. **Número da versão.** A atual é 2.7.0. Publicar como **2.9** direto, como
   o pedido diz, ou por partes (2.8 = fases 1, 1b e 2; 2.9 = completo)?
2. ~~Os 3 itens NoGain de privacidade~~ — **decidido: saem** ("retirar tudo
   de inútil").
3. **VBS:** fica no Expert com A/B obrigatório, ou sai? Ele é o único item
   que troca segurança por FPS e, em parte das máquinas, rende de verdade.
4. **Fase 7 (AimTracking Pro)** é a maior parte nova e a que menos mexe em
   FPS. Fica na ordem ou vai para depois da 9?
5. **Os jogos de nível A** da seção 17.4: a lista está certa para o seu
   público, ou falta algum que os seus clientes jogam muito?

**Nada será implementado até você aprovar.**

---

## 14. Regra nova: todo jogador ganha igual

**O pedido:** não importa se a pessoa joga em cidade de roleplay ou de PvP.
Ela tem que ganhar FPS e sentir o jogo mais liso.

**Hoje o produto divide, e isso sai:**

| Onde divide hoje | O que acontece | Na 2.9 |
|---|---|---|
| `configjogo.rs` — perfis `Equilibrado` e `Competitivo` | quem escolhe "equilibrado" fica sem os cortes que mais rendem (grama, Post FX) | **um objetivo só: mais FPS e mais 1% low.** A pessoa escolhe só **quanto de imagem aceita perder** (orçamento: Máx. FPS / Equilibrado / Qualidade), nunca o estilo de jogo |
| `niveis.rs` — Seguro / Competitivo / Experimental | "competitivo" parece a versão que ganha mais | **Normal e Expert.** O Normal já leva tudo que **provou ganho nesta máquina**; o Expert mostra os itens com troca (segurança, visual) |
| Limitador de FPS, no plano anterior | oferecido para RP, negado para competitivo | **nunca aplicado sozinho em ninguém:** um limite baixa o FPS médio por definição, e isso fere a regra "nunca diminuir o FPS". O que o produto faz é o contrário: **acha e remove limites escondidos** que seguram o jogo abaixo do monitor (seção 16, item 4) |
| Guarda de latência do gerador de quadros | só no "modo competitivo" | **vale para todos**, com o mesmo limite medido |
| Textos que falam em "servidor de RP" | a explicação parece não servir a quem joga PvP | viram "servidor com muito conteúdo próprio" — que é o que pesa, nos dois estilos |

**O portão que garante isso,** para todo jogo e todo jogador:

1. Qualquer mudança é medida na próxima partida daquele jogo, com a
   medição automática que já existe (`medicoes.rs`).
2. Se o FPS médio **ou** o 1% low caírem além do ruído medido, a mudança é
   **desfeita sozinha** e o motivo aparece na tela.
3. A tela de resultado mostra FPS, 1% low e engasgos antes e depois. Quando
   o FPS não muda e a fluidez melhora, ela diz **"mais liso"**, não "mais
   FPS".

---

## 15. O que sai do Otimiza (lista completa)

Critério único: **não muda FPS, 1% low, engasgo, atraso, carregamento nem
responsividade, e não protege a máquina.** Se não faz nada disso, sai.

**Ajustes do catálogo (11):**

- UAC e Firewall — nenhum ganho, e tiram proteção;
- Nagle — a maioria dos jogos usa UDP;
- serviços do Xbox — são sob demanda, e desligar quebra o Game Pass;
- as duas de telemetria;
- Mapas, Sincronização e Assistência Remota;
- busca web do Iniciar e Copilot.

**Painéis (7):**

| Painel | Por que sai | Vira |
|---|---|---|
| Rede e DNS | DNS não muda ping nem FPS dentro do jogo | sai; a **perda de pacote até o servidor** (`rede.rs`) fica, porque aquilo o jogador sente |
| Prioridade fixa do jogo (IFEO, alta sempre) | prioridade alta cega é o que o pedido proíbe; ganho só com disputa real de CPU; é o que mais aparece para anticheat | sai; prioridade só por sessão, dentro do A/B |
| "Por que o FPS está baixo" (existe duas vezes) | duplicado no Painel e em Jogos | **um só**, o Mapa de desempenho |
| Uso agora · Carga por núcleo · Histórico de CPU · Movimento agora | quatro painéis ao vivo mostrando pedaços | **um** Mapa de desempenho ao vivo (CPU → memória → jogo → GPU → tela), com o gargalo marcado |
| Testar um grupo de cada vez | o cliente precisava fazer à mão | vira automático: o portão da seção 14 |
| Plano de energia OTIMIZA (receita fixa) | lista fixa de valores | a aba Energia, com o motor adaptativo |
| O navegador | painel solto | entra no Background Governor |

**Módulos e peças internas:**

- `network.rs` — ajustes de rede e DNS;
- a receita do `planoenergia.rs`;
- o `monitor.rs` sem GPU;
- o "Alto Desempenho" fixo do modo jogo;
- `niveis.rs` (substituído por Normal/Expert);
- os perfis Equilibrado/Competitivo do `configjogo.rs`;
- as duas figuras `esfera.ts` e `pilares.ts`, substituídas pelo Mapa de
  desempenho, que mostra a mesma medição com o gargalo nomeado.

O desfazer de quem já aplicou qualquer item removido continua funcionando.

---

## 16. O que entra no lugar: 12 alavancas que mexem de verdade

Todas valem **para qualquer jogo**. Todas passam pelo portão da seção 14 e
têm desfazer. Ordem: da maior para a menor, em ganho típico.

| # | Alavanca | Por que muda o jogo | Onde já tem base |
|---|---|---|---|
| 1 | **Configuração gráfica do jogo, pelo custo medido de cada opção** | é o maior ganho de FPS que existe (o MSAA 4x do FiveM mostrou isso) | `configjogo.rs` → vira genérico (seção 17) |
| 2 | **Monitor na taxa que ele suporta**, com 1 clique e reversível | 60 → 144 Hz é mais fluidez do que qualquer ajuste | `display.rs` |
| 3 | **Placa de vídeo certa no notebook**, para qualquer executável | integrada → dedicada pode multiplicar o FPS | `gpupref.rs` |
| 4 | **Remover limites escondidos:** V-Sync forçado no driver, limitador global da NVIDIA, RTSS, limite no arquivo do jogo abaixo do monitor | o jogo estava preso num teto que ninguém escolheu | `nvdriver.rs`, `conflicts.rs` (RTSS), `configjogo.rs` |
| 5 | **Perfil NVIDIA por jogo:** modo de energia "preferir desempenho máximo" e baixa latência, **só no executável do jogo** e só com A/B | a placa para de baixar o clock no meio da partida | `nvdriver.rs` (perfil por executável já existe) |
| 6 | **Background Governor na partida:** processos opcionais pesados vão para prioridade abaixo do normal e EcoQoS (API documentada do Windows 11); downloads da Store e da Otimização de Entrega pausam; **tudo volta quando o jogo fecha**. Nada é congelado | tira a disputa de CPU e disco que vira engasgo | `processes.rs`, `servicesaudit.rs`; o congelamento continua proibido |
| 7 | **Memória:** paginação mal configurada corrigida, navegador pesado avisado, widgets só com pressão | é o engasgo nº 1 em PC de 8 GB | `memory.rs`, `pressao.rs`, `browsers.rs` |
| 8 | **Energia calibrada por máquina** | clock que sobe mais rápido melhora o 1% low | motor de energia |
| 9 | **Recuperação de hardware:** XMP, canal único, PCIe abaixo do esperado, ReBAR, com passo a passo de BIOS | RAM sem XMP custa FPS em jogo que depende da CPU | `firmware.rs`, `pcie.rs`, `rbar.rs`, `bios.rs` |
| 10 | **Jogo no HD mecânico** → guia para mover a pasta pela própria loja | engasgo de carregamento de textura | `discodojogo.rs` |
| 11 | **Cache de shader:** reconstruído depois de troca de driver quando o engasgo tem cara de compilação; driver velho avisado | engasgo nas primeiras partidas | `shaders.rs` |
| 12 | **Gerador de quadros do Otimiza** em qualquer jogo em janela sem bordas, com guarda de FPS e de latência | mais quadros na tela, sem tirar do jogo | `geracao/` |

**Com A/B obrigatório, no Expert:** agendamento de GPU por hardware (HAGS),
otimizações de tela cheia por jogo e VBS. São itens que ajudam em algumas
máquinas e atrapalham em outras.

---

## 17. Biblioteca de jogos 2.9: todos os jogos, não só o FiveM

### 17.1 O que existe hoje

- **Lojas lidas:** Steam, Epic e o histórico de GPU do Windows, conferido no
  disco (`jogos.rs`).
- **Nomes conhecidos:** 15, em `gamemode.rs`. Qualquer outro jogo já é
  reconhecido pelos sinais medidos (`deteccao.rs`: janela cobrindo o
  monitor, motor 3D em uso, tempo aberto).
- **Configuração gráfica:** lida e escrita **só no GTA V/FiveM**.
- **Medição de FPS por ETW:** já funciona em **qualquer jogo**.

**O furo:** o motor já mede qualquer jogo, mas só sabe *melhorar* um.

### 17.2 Onde o jogo é encontrado

| Fonte | Situação |
|---|---|
| Steam, Epic | já existe |
| Riot Client (Valorant, LoL), Battle.net, EA app, Ubisoft Connect, GOG Galaxy, Rockstar Games Launcher, Xbox / Microsoft Store (pacotes de jogo) | **novo** — uma função por loja, lendo onde ela guarda a lista de instalados |
| FiveM / RedM (pasta do CitizenFX), Roblox, Minecraft (launcher oficial) | **novo** — não têm loja tradicional |
| Programas instalados do Windows | **reserva** — só vira jogo quando há outro sinal |
| **Jogo detectado rodando** (`deteccao.rs`) | **novo e o mais importante:** todo jogo que a pessoa abre entra sozinho na biblioteca, com nome, executável e pasta. Nenhum jogo fica de fora por não estar numa lista |

**Regra:** cada leitor só entra depois de conferido numa máquina com aquela
loja instalada. Uma loja que não pôde ser lida aparece como lacuna, nunca
como "nenhum jogo".

### 17.3 Ficha do jogo

Cada jogo conhecido ganha uma ficha de dados, separada do código:

```
nome, executáveis, lojas
motor           Unreal 4/5 · Source 2 · RAGE · Unity · Frostbite · próprio
API             DX11 · DX12 · Vulkan
anticheat       Vanguard · EAC · BattlEye · FACEIT · Ricochet · nenhum → o que o Otimiza NÃO toca
configuração    caminho, formato, chaves com custo (FPS · VRAM · visual), limite de FPS, V-Sync
recursos        DLSS · FSR · XeSS · Reflex · Anti-Lag · limite interno → só o que foi conferido
gargalo típico  CPU (mapa cheio, muitos jogadores) · GPU · VRAM
```

As fichas moram num arquivo de dados e podem ser atualizadas sem lançar
versão, pelo mesmo caminho do `convite.json`. Com uma diferença obrigatória:
**o arquivo vem assinado com a chave ed25519 que o produto já usa na
licença.** Uma ficha manda o produto escrever num arquivo do jogo, e dado
sem assinatura não pode mandar escrever.

### 17.4 Níveis de suporte

| Nível | O que o Otimiza faz | Jogos (proposta inicial) |
|---|---|---|
| **A — otimiza o jogo** | lê e **escreve** a configuração, com prévia, cópia do arquivo inteiro, medição antes e depois e desfazer | FiveM, GTA V, CS2, Valorant, Fortnite, Apex Legends, PUBG, Rainbow Six Siege, Minecraft Java, Rust, Marvel Rivals, Roblox (só o que o próprio Roblox permite) |
| **B — orienta** | lê a configuração e diz **quais opções custam mais nesta máquina**; a pessoa muda dentro do jogo | League of Legends, Dota 2, Overwatch 2, Call of Duty/Warzone, EA FC, Rocket League, RedM/RDR2, Genshin Impact, Free Fire em emulador |
| **C — qualquer outro jogo** | medição automática, gargalo, alavancas do sistema (seção 16), perfil NVIDIA por executável, gerador de quadros | **todos** os que a detecção pegar |

Nenhum jogo fica sem ganho. O nível C já recebe as alavancas 2 a 12.

**Jogos com anticheat de kernel** (Vanguard, FACEIT, Ricochet): o produto só
mexe no **arquivo de configuração do usuário**, que é o mesmo que o menu do
jogo grava. Nunca toca no processo nem na pasta do jogo. Se a ficha não
tiver confirmação de que o jogo aceita a edição, o nível cai para B.

### 17.5 O escritor genérico de Unreal

A maior parte dos jogos atuais roda em Unreal Engine 4/5. Eles guardam a
qualidade gráfica no mesmo formato: o `GameUserSettings.ini`, com as
chaves de escalabilidade do motor (`sg.ShadowQuality`,
`sg.PostProcessQuality`, `sg.FoliageQuality`, `sg.EffectsQuality`,
`sg.ViewDistanceQuality`, `sg.AntiAliasingQuality`, `sg.TextureQuality`,
`sg.ShadingQuality`, `FrameRateLimit`).

Um escritor só, com teste, cobre de uma vez Fortnite, Valorant, PUBG, Marvel
Rivals e boa parte dos lançamentos. **As chaves e os valores serão
conferidos na documentação da Epic antes de escrever**, e o escritor recusa
arquivo com formato que não reconhece.

O mesmo vale depois para os formatos da Valve (CS2, Dota 2).

### 17.6 O que a ficha alimenta

- **Graphics Tuner:** mede opção por opção quando a cena é repetível; senão,
  usa o custo da ficha como ponto de partida e **mede o conjunto**.
- **Perfil NVIDIA por executável:** para qualquer jogo, inclusive nível C.
- **Digital Twin:** o melhor perfil conhecido de cada jogo, naquele PC.
- **Game Readiness:** antes de abrir o jogo, avisa o que está atrapalhando
  aquele jogo específico.
- **Session Report:** depois de cada partida.

### 17.7 Testes da biblioteca

- **Leitores de loja:** um teste por loja, com arquivo real salvo como
  amostra, como os do `jogos.rs`.
- **Escritores:**
  - **idempotência:** aplicar duas vezes dá o mesmo arquivo, igual ao teste
    que o `configjogo.rs` já tem;
  - **desfazer:** volta o arquivo byte a byte;
  - **formato desconhecido:** o escritor recusa.
- **Ficha sem assinatura válida:** é recusada.
- **Trava contra lista fixa:** reprova se a detecção passar a depender de
  lista de nomes. Um jogo sem ficha tem que continuar sendo detectado.

---

## 18. Andamento (branch `versao-2.9`)

**Feito, testado e commitado:**

| Item | Onde | Conferido nesta máquina |
|---|---|---|
| 11 ajustes retirados (só o desfazer continua) + `naofazemos.rs` | `catalog.rs`, `profiles.rs` | testes |
| Núcleo de estatística (Welch 95%, A B B A, validade) | `core/estatistica.rs` | testes |
| Saúde dos quadros: P95, P99, 1% e 0,1% low, engasgos por gravidade, índice de fluidez | `core/fluidez.rs`, `frames.rs` | testes + equivalência com o 1% antigo |
| Telemetria por PDH/DXGI (CPU, núcleo mais ocupado, clock efetivo, GPU, VRAM, RAM, disco) | `core/telemetria.rs`, `core/pdh.rs` | leitura real: GTX 1650, 4.066 MHz efetivos |
| Classificador de gargalo múltiplo | `core/gargalo.rs` | testes |
| Mapa de desempenho (mede qualquer jogo aberto) | `mapa.ts`, `diagnostico_ao_vivo` | comando real sem jogo; tela conferida |
| Biblioteca: programas instalados + todo jogo visto rodando | `jogos.rs`, `deteccao.rs` | achou GTA V (Steam), FiveM e Roblox |
| Ajustador genérico de Unreal (orçamento de imagem) | `unreal.rs` | teste de ponta a ponta com desfazer byte a byte |
| Tela Seus jogos | `biblioteca.ts` | tela conferida |
| Fora: painel DNS, prioridade fixa; "Competitivo" vira "Máximo de FPS" | vários | testes |
| Guarda de atraso do gerador vale para todo perfil | `framegen.rs` | testes |
| Modo jogo → governador de segundo plano (prioridade baixa + EcoQoS) | `governador.rs`, `gamemode.rs` | ao vivo: acalmou e devolveu um processo real |
| Limites de FPS escondidos (NVIDIA global, RTSS, Unreal) | `tetos.rs` | leitura real: 180 Hz, nenhum teto |
| Portão "nunca menos FPS" para ajuste de jogo | `portao.rs` | testes |

**Segunda rodada:**

| Item | Onde | Conferido nesta máquina |
|---|---|---|
| Desempenho perdido (P0–P3) no veredito | `veredito.rs` | real: "sim" — canal único, RAM curta, esgotamento |
| Botões de conserto que chamavam otimização inexistente | `veredito.rs` | trava em teste |
| Registro de eventos sem eventos deixa de virar "não consegui ler" | `exhaustion.rs` | real: lacuna falsa sumiu |
| Boot e térmico: leitura que falha não vira verde | `boot.rs`, `thermal.rs` | testes |
| Detetive de travadas (travada × disco, paginação, VRAM, núcleo, programa) | `core/travadas.rs` | testes; sem jogo aberto |
| Deriva por jogo (driver / Windows / sem culpado) | `deriva.rs` | versão real lida: driver 32.0.16.1692, build 19045.4170 |
| Verificar antes de jogar | `prontojogo.rs` | real: pronto, recomenda modo jogo |
| Inicialização classificada | `startup.rs` | real: Discord/Spotify/Steam opcionais, LG Hub útil |

Já existiam antes da 2.9 e não foram duplicados: teste de resposta de boost e
clock efetivo no motor de energia; prontidão, multiplicador pelo Hz e guarda
de atraso no gerador de quadros.

Decidido não fazer: ajustador do Roblox — o jogo já vem no gráfico
automático, que baixa a qualidade para segurar o FPS; não há como provar que
um ajuste nosso renderia mais.

**Não visto ainda (precisa de jogo aberto / outra máquina):**

- `diagnostico_ao_vivo` com jogo aberto (quadros por ETW exigem administrador).
- Ajustador Unreal num jogo Unreal real — esta máquina não tem nenhum.
- Portão decidindo com partidas reais (precisa de 3 partidas medidas antes e 3 depois).
- Governador numa partida de verdade.

**Próximas fases:** Recuperação P0–P3 no veredito, energia (teste de resposta
de boost, escrita como admin), Stutter Detective com janela de contexto,
fichas de jogo nível B, AimTracking, UI final e fechamento da versão.
