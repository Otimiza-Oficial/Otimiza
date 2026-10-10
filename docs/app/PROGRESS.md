# PC Performance Optimizer — Estado real do projeto

> Este documento registra apenas o que foi **verificado**. Funcionalidade que não
> foi executada e conferida aparece como pendente, mesmo que o código exista.

## 3.3 (em andamento) — Mega Update

Plano: `docs/planos/MEGA-UPDATE.md`. Protótipo aprovado da UI: `docs/planos/prototipo-3.3.html`.

| Item | Estado | Como foi verificado |
|---|---|---|
| 1578 testes passam | verificado | `cargo test --lib` em 06/10/2026: 1578 passaram, 0 falharam, 77 ignorados |
| Pedido de feedback (10 min) | **bot verificado em produção em 10/10/2026** (404 → 200 `ok: true` com feedback real → `ok: false` apagado, assinatura conferida com a pública do Otimiza); falta publicar a `api` no `convite.json` junto com a 3.3 | Regras e atestado assinado com testes (par de chaves sorteado). Sem `api` em `.github/convite.json` fica desligado. O endpoint do bot está especificado em `docs/app/FEEDBACK.md` e ainda não existe |
| Bot: `atestarFeedback` e `validarFeedback` | verificado | Executado com Node: atestado conferido pela chave pública, mensagem curta e sem print recusada |
| Modelo de métricas medido/estimado | verificado por teste | Frame Boost só mostra "FPS na tela" medido; nunca nativo × multiplicador |
| Gerador de quadros barrado por anticheat | verificado por teste | Vanguard ou FACEIT abertos recusam a janela por cima do jogo; o serviço do Vanguard no boot sozinho não recusa |
| Perfil de hardware em cache | verificado nesta máquina | i3-10100F, GTX 1650, 7,9 GB, monitor 24G4 a 180 Hz com VRR provável |
| Motor de compatibilidade | verificado por teste | Smooth Motion só RTX 40/50, AFMF a partir da RX 6000, DX9/OpenGL sem geração de driver, multiplicador nunca acima do monitor |
| Driver AMD pelo ADLX (Anti-Lag, AFMF, limitador, cores, sensores) | **pendente: precisa de uma Radeon** | Posições das funções copiadas do ADLX SDK 2.0. Nesta máquina (NVIDIA) responde "sem placa AMD" sem travar. Aplicar e desfazer nunca rodaram numa placa AMD |
| Driver Intel pelo IGCL (baixa latência, limitador) | **pendente: precisa de uma placa Intel** | Estruturas conferidas com o tamanho do `igcl_api.h` 1.1 (56 e 36 bytes). Nesta máquina responde "sem placa Intel" sem travar. Cores pela Intel ainda fora |
| Catálogo de 15 jogos e sessão do jogo aberto | verificado por teste | FiveM é um jogo do catálogo; Valorant e Call of Duty barram janela por cima; a geração própria do jogo sai dos arquivos da pasta (DLSS-G, FSR FG, XeSS FG) |
| Histórico por jogo | verificado por teste | Builds do FiveM juntas; ganho só com a estatística separando antes e depois |
| Interface 3.3 (shell, Início 3D, Jogos, Boost, Mira, Cores, Lab, Ajustes) | **verificado no app aberto nesta máquina** | `tauri dev` em 05/10/2026: as sete telas abriram com dados reais (i3-10100F, GTX 1650 driver 617.14, 180 Hz, licença vitalícia). Cena 3D carrega depois da tela (three.js em pacote separado). Não testado com jogo aberto nem como administrador |
| Mira na tela | **verificado com o jogo de teste** | Em 05/10/2026, com `examples/jogo_de_teste` e `OTIMIZA_JOGO_DE_TESTE` (só na compilação de desenvolvimento): a mira apareceu no centro da janela do jogo, não aceitou clique, pediu confirmação no jogo sem anticheat conhecido, sumiu com o jogo minimizado, voltou ao restaurar e desligou pelo botão. Não testada num jogo real nem em dois monitores |
| Mira no CS2 (comandos de console) | verificado por teste | Comandos `cl_crosshair*` gerados a partir da mesma mira; não colados num CS2 de verdade |
| FOV do FiveM | **verificado no jogo do dono** (10/10/2026: gravada a posição 4 com o FiveM fechado, o menu do jogo abriu em 4; o de antes, 5, ficou guardado) | 09/10/2026: o FiveM guarda a opção do menu (Câmera → campo de visão em primeira pessoa) em `seta "profile_fpsFieldOfView" "5"` no `%APPDATA%\CitizenFX\fivem.cfg`. `plataforma/fov_fivem.rs` troca só essa linha com o FiveM fechado (pendente enquanto ele está aberto, como o `cl_drawFPS`), guarda a de antes e devolve byte a byte; entra no Desfazer tudo. Leitura real no PC do dono: posição 5. Cartão na aba Mira (`fov_fivem` LIVRES, `fov_fivem_definir` EXIGEM, `fov_fivem_devolver` LIVRES). Falta: conferir no jogo que 0 e 10 são as pontas do controle. GTA V fora do FiveM continua sem mapear (fica no perfil binário em Documentos) |
| Cores no monitor (NVIDIA + rampa do Windows) | **verificado no app aberto nesta máquina** | 05/10/2026, GTX 1650 driver 617.14, dois monitores: a tela leu a vibração real (100%), aplicou 50% e o Competitivo (vibração 80% + rampa com brilho/contraste/gama), e "Voltar ao normal" devolveu exatamente 100% e a rampa identidade, apagando `cores-originais.json`. Os números do NVAPI para a vibração (`GetDVCInfoEx` 0x0E45002D, `SetDVCLevelEx` 0x4A82C2B1) não estão no cabeçalho público: provados aqui, interruptor `vibracao_nvidia` ligado, e se o driver não responder cai para a rampa. Perfil por jogo com "só com o jogo aberto" (liga na detecção, devolve ao fechar e na abertura depois de uma queda): testado por unidade, **não testado com jogo real** |
| Cores pela AMD (ADLX) | **pendente: precisa de uma Radeon** | Saturação, matiz, brilho e contraste pelo `IADLXDisplayCustomColor`, gama pela rampa |
| Frame Boost no app (Boost) | **verificado no app aberto com o jogo de teste** | 05/10/2026: o painel escolheu o Gerador do Otimiza, mostrou 2×/3×/4×, ligou e exibiu só o FPS na tela medido (130, "medido pelo gerador"), com o atraso marcado como estimado; o nativo não aparece. Pedido de 3× ficou em 2× porque passaria dos 180 Hz, e a tela agora diz isso. Desligar devolveu o estado. Sem medição nativa do jogo, não mostra estimativa (correto) |
| Gerador: Hz do monitor do jogo | verificado no jogo de teste | Lê o Hz do monitor onde a janela está (`\.DISPLAYn` da saída DXGI), não o do principal. Testado num monitor só: com dois, falta provar no monitor secundário |
| Gerador: janela redimensionada, movida, minimizada, troca de modo de vídeo | **verificado no app aberto e no jogo de teste** | Antes desligava ("Ligue de novo"); agora se refaz sozinho em ~1 s: 1280x720 → 1024x576, minimizar e restaurar, e no app 1280x720 → 1100x620. Seis refeitas seguidas de menos de 5 s = desiste com explicação |
| Gerador: HDR (FP16) | verificado com FP16 forçado numa tela SDR | `DuplicateOutput1` no formato da área de trabalho, comparação em espaço com gama e saída scRGB linear (swap chain FP16). Com `OTIMIZA_FG_FP16`: 2× estável, custo ~1,2 ms, quadros reais e gerados com a mesma distribuição de cor do SDR. Descoberto e corrigido: o primeiro quadro vem em BGRA mesmo pedindo FP16. **Não testado num monitor HDR de verdade** |
| Gerador: notebook com duas placas | verificado por teste (unidade) | Detecta quando o monitor está na integrada e avisa; a recusa de captura do Windows vira instrução ("Economia de energia" nas configurações de gráficos). **Não testado num notebook híbrido** |
| Gerador: só conta o que é do jogo | **verificado no app aberto e no jogo de teste** | Antes, qualquer coisa que mudasse no monitor (a cena 3D do Otimiza, um vídeo, a recomposição da própria sobreposição) contava como quadro do jogo: o FPS real aparecia acima do verdadeiro e o multiplicador caía para 1× (no app: 106-181 FPS "medidos" a 1×). Agora conta só o que mudou dentro da janela do jogo (retângulos sujos) e que é diferente do último quadro (luma comparada na placa). Resultado: 60 reais com o jogo a 60, e no app 3× = 180 FPS no monitor de 180 Hz |
| Gerador: monitor girado (em pé) | **verificado no jogo de teste e no app** | 06/10/2026, monitor 2 a 270°: a captura vem na orientação do painel (1920x1080). A área do jogo é convertida e a imagem girada de volta na entrada; quadros reais e gerados saem em pé. O próprio Windows entrega a captura desse monitor com uma vinheta marrom nas bordas (vista na textura crua, fora do gerador): o Otimiza copia fiel |
| Gerador: janela movida para outro monitor | **verificado no app aberto** | Ligado no monitor 1, movido para o 2: refez sozinho em ~1 s em `\.DISPLAY2` |
| Frame Boost: AFMF (AMD) pelo automático | verificado por teste (unidade) | O botão Frame Boost liga o AFMF pelo ADLX com histórico e desfazer; o FPS na tela vem do PresentMon ("Medir agora", 6 s), nunca nativo × 2. **Pendente: Radeon** |
| Frame Boost: Smooth Motion (NVIDIA), experimental | **recusa provada nesta máquina; ligação pendente: RTX 40/50** | Atrás do interruptor em Ajustes (desligado por padrão). Escreve 0xB0D384C0 só no perfil do jogo, com o anterior guardado; se o driver chamar esse número de outra coisa, recusa. Na GTX 1650 (driver 617.14) o driver não conhece o número e recusa a escrita: a sessão não é salva e nenhum perfil fica para trás (teste ignorado `liga_e_desfaz_no_perfil_do_jogo_de_teste`) |
| FPS por cima do jogo (Mínimo / Detalhado / Desligado) | **verificado no app aberto com o jogo de teste** | 06/10/2026: em Ajustes, Detalhado + "Mostrar agora no jogo"; pediu confirmação (jogo sem anticheat conhecido), apareceu no canto do jogo só com ele na frente e sumiu quando outra janela veio para frente. Sem administrador mostrou "— FPS · Abra o Otimiza como administrador para medir" (não inventa); com o Frame Boost, "180 FPS NA TELA · 3×", sem o nativo. Fora de captura (não entra no gerador nem em gravação). Liga sozinho quando o jogo abre, se a pessoa escolheu um modo. **Não testado como administrador** (FPS, 1% low e tempo de quadro do PresentMon ao vivo: só por teste de unidade) |
| Versão 3.3.0 e limpeza (M10) | verificado | 06/10/2026: versão 3.3.0 nos quatro arquivos (teste `as_quatro_declaracoes_de_versao_concordam`), notas em `.github/release-notes.md`, "custa X dos quadros" marcado ESTIMADO, `autoajuste.rs` e o comando que a tela nunca chamava removidos. Instalador de teste gerado: `src-tauri/target/release/bundle/nsis/Otimiza_3.3.0_x64-setup.exe` (não publicado). Juntar as duplicatas da auditoria (telemetry/telemetria, frames/presentmon, benchmark/baseline) e tirar o `main.ts` antigo ficam para depois: o Modo clássico ainda usa as telas dele |
| Fluxo óptico do AMD (FSR 3) no gerador | medido, **não virou padrão** | 07/10/2026: busca completa ±8 px por bloco 8×8 em pirâmide 1/16→1/2, previsão pelo melhor entre pai, vizinhos, mediana e parado, e escolha célula a célula na grade de 1/8 (`Busca`/`Amplia`, licença MIT em `LICENCAS-DE-TERCEIROS.md`). Nos 30 pares gravados do FiveM: SSIM 0,655 contra 0,663 da Grossa/Fina, com 3,2 ms contra 1,4 ms de placa. Melhorou o cenário (banco, quiosques) e rasgou o rosto do personagem parado. O FSR 3 depende dos vetores de movimento do próprio jogo, e o FiveM não os entrega para fora. Fica atrás de `OTIMIZA_FG_BUSCA=blocos`; `OTIMIZA_FG_VER_VETORES` desenha o campo de vetores |
| Fluxo denso DIS no gerador (padrão) | medido nos quadros gravados, **não medido ao vivo** | 07/10/2026: DIS (Dense Inverse Search, Kroeger et al. 2016) simétrico, em pirâmide 1/16→1/4. Pedaços 8×8 a cada 4 px, partida pelo melhor entre o campo de cima, o parado (com folga absoluta, que segura o rosto do personagem) e a câmera, refino de Gauss-Newton e campo denso pela média ponderada dos pedaços. Um grupo de 64 threads por pedaço (com uma thread por pedaço o nível 1/4 custava 1,6 ms). Nos 29 pares do FiveM: SSIM 0,6643 contra 0,6629, PSNR 24,65 contra 24,45 dB, melhor em 19 pares e pior em 2, 1,34 ms de estimativa (antes 1,41) e 1,32 ms por quadro gerado (antes 0,92). Ao vivo (jogo de teste no monitor 2, GTX 1650, 1080×1920): limitado a 51 fps, 2× deu 100 na tela, 3× deu 150 e 4× (a 46 fps) deu 180, o teto do monitor, com 0% de perda no jogo nos três. Com o jogo sem limite e a placa a 100% (210 fps), o gerador só aproveita ~40 reais/s (a busca antiga, ~55) e o jogo perde 6,5% (a antiga, 10,6%). `SetGPUThreadPriority(7)` não mudou nada e foi tirado. A busca inteira também em 1/4 (`OTIMIZA_FG_DIS_RAIO=2,2,2,0`) dá SSIM 0,6655 por +1,2 ms, sem diferença visível: não entrou. `OTIMIZA_FG_BUSCA=antiga` volta à Grossa/Fina; `OTIMIZA_FG_DIS`/`OTIMIZA_FG_DIS_NIVEIS` são para medir |
| Jogo liso: trava o FPS num ritmo estável | medido (teto no laboratório, baseline no FiveM do dono) | 08/10/2026. A medição do próprio Otimiza no FiveM do dono (`medicoes.json`): 45,6 fps de mediana mas **74 trancos/min** e P99 de 72 ms, gargalo na CPU (99% de uso) — o cliente sente a oscilação, não o número. `plataforma/ritmo_estavel.rs` escolhe o teto = maior fração do monitor ≤ FPS sustentado (180 Hz, 45 fps → 45 = 180/4), `plataforma/jogo_liso.rs` mede e aplica pelo driver. Laboratório (`nvdriver::laboratorio_do_limite`): o teto do driver **só vale ao reabrir o jogo** (com o jogo já aberto não muda nada), por isso o cartão avisa "reabra o jogo". O "Otimizar" **parou de tirar o limite de FPS do FiveM/GTA** (`optimize_now`/`aplicar_teto_do_jogo`): sem limite o tempo de quadro oscila mais num jogo preso no processador. Comandos `jogo_liso_medir` (LIVRES)/`jogo_liso_aplicar` (EXIGEM), cartão "Jogo liso" no Boost. Pesquisa: NVOFA não existe no TU117 do dono, RIFE não é tempo real na GTX 1650, nenhum motor de FG aberto com licença comercial (LSFG/Universal-FrameFX fechados, MVTools/lsfg-vk GPL/NC) |
| Memória para o jogo e Energia de jogo | unidade + leitura real | 08/10/2026. Medido no PC do dono: 1,4 GB livres sem o FiveM (Brave 2,6 GB, DiscordCanary 1,3 GB); `plataforma/memoria_jogo.rs` lista e fecha a pedido (`memoria_para_o_jogo` LIVRES / `memoria_fechar` EXIGEM). Plano OTIMIZA do dono: núcleos acordados e boost agressivo já estavam, mas o EPP em 33 (o motor de energia nunca rodou: exige medição longa). `plataforma/energia_jogo.rs` escreve EPP 0 (ou mínimo 100% sem Speed Shift) no plano ativo só com o jogo aberto, guarda o valor de antes em `energia_de_jogo.json` e devolve ao fechar, na abertura seguinte e no Desfazer tudo; notebook só na tomada. Comandos `energia_de_jogo` (LIVRES) / `energia_de_jogo_definir` (EXIGEM). Falta medir antes × depois no FiveM |
| Oclusão por ida e volta + mediana dinâmica | medido no harness, **sem rodar no FiveM** | 08/10/2026, depois da pesquisa (NVOFA indisponível no TU117 do dono; RIFE não é tempo real na GTX 1650; FSR 3 depende dos vetores do jogo; SVP/MVTools é o parente certo). O DIS roda também de ida e de volta em 1/16–1/8 (`rotacao` = direção em `DisPedaco`/`DisDenso`), `Consistencia` marca o que some/aparece, `FinalBidirecional` tira cor só do lado que existe e usa a mediana de A deslocado, B deslocado e a mistura parada onde os dois existem (SVP modo 13). Harness: PSNR 24,65 → 24,75 dB, SSIM 0,6643 → 0,6647, estimativa 1,34 → 2,42 ms; borda do rosto e postes mais limpos, banco a ~500 px por par continua transparente. `OTIMIZA_FG_OCLUSAO=0` volta. Também: o Frame Boost não abre mais o contador (`overlay` versão 2 da config desliga o padrão antigo), "passar do limite" saiu da tela, e o painel avisa quando memória < 0,7 GB ou placa ≥ 90% (`frameboost::aviso_de_folga`) |
| Contador com o FPS do jogo | verificado por teste | 08/10/2026, decisão do dono: com o Frame Boost, o FPS por cima do jogo mostra o FPS do próprio jogo (PresentMon; sem ele, o que o gerador capturou), como o `cl_drawfps`. O que vende é a fluidez. Os quadros na tela seguem medidos no painel e no log (`com_frame_boost_mostra_o_fps_do_jogo`) |
| Auto no ritmo do monitor | medido no laboratório, **sem rodar no FiveM** | 08/10/2026: no Auto dentro dos Hz, a swap chain com objeto de espera (latência 1) e `Present(1)`: um quadro por atualização da tela, cada um com o `t` do instante que mostra, pelo carimbo `LastPresentTime` do real suavizado (`CarimboSuave`, tira o degrau 16,7/22,2 ms de 50 fps em 180 Hz) e um atraso de intervalo + preparo + 5 ms (`AtrasoDaExibicao`). Jogo de teste a 45–50 fps, 180 Hz: atualizações repetidas 26% (primeira versão, pela hora de processamento) → 5–6% em regime, 160–170 quadros novos/s, perda ≤0,5%. `GetFrameStatistics` da swap chain de composição não serve (devolve números parados). No FiveM o dono viu o contador em Detalhado: com "no lugar do cl_drawfps", agora é sempre o Mínimo |
| Passar do limite do monitor | medido no laboratório, **sem rodar no FiveM** | 07/10/2026: `Configuracao::sem_limite` tira o corte pelos Hz (até 6×) e o Auto mira 2× os Hz. Jogo de teste a 50 fps no monitor de 180 Hz: 6× e Auto deram ~300 apresentados/s, perda 0%. O monitor mostra no máximo os Hz: o contador troca "FPS na tela" por "FPS" acima deles (`acima_dos_hz_nao_diz_na_tela`) |
| Frame Boost sem trancos, Auto, contador firme e bandeja | verificado por teste, **sem rodar no FiveM** | 07/10/2026, do log de uma sessão do dono no FiveM: sobreposição "escondida 15%" o tempo todo. A guarda pausava 1,5 s a cada ~13 s e, com a conferência descartada, repetia para sempre. Agora a rotina é de 300 s e a que falha dobra a espera (`conferencia_que_falha_espera_cada_vez_mais`). O log de 10 s diz por que ficou escondida (sem quadro, guarda, jogo atrás). Auto (`ritmo::quadros_no_automatico`): enche o monitor, até 6 quadros por real. Contador: `overlay::Suavizador` (média móvel, segura 3 s, nunca inventa). Bandeja: com jogo aberto, fechar esconde; sai sozinho quando o jogo fecha; uma cópia só (`bandeja::ja_aberto_traz_para_frente`). O Auto não foi medido ao vivo: o laboratório parou porque o dono estava jogando |
| Contador no lugar do cl_drawfps | verificado por teste, **sem rodar com o FiveM** | 07/10/2026: `plataforma/contador_fivem.rs` troca só a linha `cl_drawFPS` do `%APPDATA%\CitizenFX\fivem.cfg`, guarda o valor de antes e só grava com o FiveM fechado (ele regrava o arquivo ao sair). Desligar devolve o valor exato |
| Jogo continua aberto atrás do Otimiza | **verificado com o FiveM aberto nesta máquina** | 06/10/2026, achado no teste do dono: a sessão só via o jogo em primeiro plano, então o Alt+Tab para o Otimiza "fechava" o jogo e o botão Frame Boost ficava cinza (o jogo de teste escondia isso por entrar por outro caminho). Agora o jogo fica aberto enquanto o processo existir, e um jogo do catálogo com janela conta mesmo sem nunca ter vindo para frente. Teste ignorado `acha_o_jogo_aberto_agora` achou `FiveM_b3258_GTAProcess.exe` atrás de outras janelas. A janela do gerador só aparece com o jogo na frente |
| Teto de FPS do driver no FiveM | **causa medida e corrigida; falta medir com o teto valendo** | 09/10/2026: o "Travar em 60" do Jogo liso nunca valeu no FiveM (60 gravado em 08/10, o jogo seguiu a 64–87). O FiveM cadastra o próprio caminho num perfil do driver (`Cfx.re: <caminho>`), e o driver prefere essa entrada à do perfil pelo nome que o Otimiza criava. `nvdriver::aplicar_limite` acha o perfil pelo caminho do executável aberto (`perfil_em_uso`), e o histórico guarda o caminho para o desfazer. Laboratório com o jogo de teste e um perfil pelo caminho (`o_caminho_ganha_do_nome`): pelo nome 554–912 fps (ignorado), pelo caminho 41–46 (teto 45 valendo). O laboratório antigo usava carga 50, em que o jogo de teste já roda a ~46 fps: não provava nada. O Jogo liso mostra o teto que o driver usa e avisa quando um teto gravado não chega ao jogo; o log diz, a cada abertura, se o teto sobreviveu (o FiveM pode recriar o perfil dele) |
| Tela cheia × janela no FiveM | **medido no FiveM do dono** | 09/10/2026, PresentMon, mesmo lugar do mapa: tela cheia ("Hardware Composed: Independent Flip") 85 fps, 1% low 31, p99 21,9 ms, 11,5 ms de CPU por quadro; janela sem bordas ("Composed: Copy with GPU GDI") 61 fps, 1% low 24, p99 30,2 ms, 16,2 ms de CPU. O Jogo liso mostra como o quadro chega à tela e, no modo de cópia, pede tela cheia (cita os números só quando o gargalo é o processador) |
| Programas que derrubam a tela cheia | verificado por teste + leitura real | 09/10/2026: em tela cheia o FiveM minimizava sozinho de 0,2 a 9 s depois de voltar à frente. Vigia de foco: o primeiro plano ia para "nenhuma janela" antes da barra de tarefas. Causa: MyDockFinder (`dock_64.exe`), com janelas "sempre por cima" no monitor do jogo (barra 1920×24 no topo, faixa de 1 px embaixo). Fechado, parou. `plataforma/por_cima.rs` lista as janelas sempre por cima de outros programas no monitor do jogo (sem o Windows, o jogo e o Otimiza) e o Jogo liso diz o nome. Ao vivo, com o Dock fechado: nenhuma |
| Frame Boost: diagnóstico de 10/10 | **medido nos logs do dono** | Nas sessões reais do FiveM (i3-10100F, GTX 1650, 180 Hz) a guarda desligou o gerador 7 vezes: o jogo perdia de 5% a 35% do FPS com ele ligado (razão 0,65–0,95). Na sessão de 17 min de 08/10: 61 reais/s → 138/s apresentados num monitor de 180 Hz. Sem bordas o Windows copia cada quadro (61 fps contra 85 em tela cheia), e o gerador precisa de janela ou sem bordas. Custo de placa no laboratório: 2,42 ms por real (DIS + oclusão) + 1,32 ms por gerado, ~30% da placa a 60 fps em 3×. Plano: `~/.claude/plans/quero-que-fa-a-uma-federated-cascade.md` |
| Frame Boost: medição honesta | verificado por teste e no jogo de teste | 10/10: `gpu::Cronometro` mede o tempo de PLACA do gerador ao vivo (carimbos lidos sem esperar, roda de 16 marcas); o log de 10 s diz "placa X ms/s (Y% do tempo), Z ms por real" e o painel "usa Y% da placa (medido)". Jogo de teste: 8 ms/s, 0,46 ms por real. O "FPS na tela" passou a ser o que o PresentMon viu CHEGAR à tela (`Vivo::iniciar_exibidos` no processo do Otimiza, coluna `DisplayedTime`); sem administrador, o apresentado aparece como ESTIMADO. Placar `scripts/laboratorio/ab-frameboost.ps1` + `ab-resumo.mjs` (conferido contra a medição de 09/10: 85,0 × 61,5 fps). Falta: rodar no FiveM |
| Frame Boost: modo leve e guarda que alivia | verificado por teste, **sem rodar no FiveM** | 10/10: modo leve = busca principal até 1/8 (sem o passe de 1/4), oclusão mantida (sem ela a troca de cena sairia). Começa leve com a placa ≥ 85%; quando a guarda vê perda, primeiro fica leve e confere do zero, e só desliga se perder de novo (`reacao_a_perda`). Falta: medir SSIM × custo do leve nos quadros gravados e a razão da guarda no FiveM |
| Frame Boost: otimização de janela do Windows | verificado por teste, **efeito não medido** | 10/10: ao ligar, o Frame Boost grava `SwapEffectUpgradeEnable=1;` no valor do jogo em `HKCU\Software\Microsoft\DirectX\UserGpuPreferences` (mantendo o resto da linha, ex. o `AppStatus=0;` do FiveM), com desfazer. Deve tirar o jogo sem bordas do modelo de cópia; vale ao reabrir. Falta: confirmar no placar que o FiveM sem bordas sai de "Composed: Copy with GPU GDI" e quanto FPS volta |
| Frame Boost: revisão final (prova no painel) | verificado por teste e em print do painel com dados simulados, **sem rodar no FiveM** | 10/10: (1) o painel ligado mostra o FPS do jogo e quanto ele MANTEVE com o Frame Boost (a razão da guarda, com o número de comparações); (2) desligado, "FPS na tela da última vez" com o jogo, o tempo e o que ele manteve (`geracao::resumo`, só com 30 s ou mais gerando; medido só se o PresentMon contou 90% da sessão, senão a sessão toda vai como estimado); (3) Destravar a base vale até a pessoa tocar em "Travar de novo" (antes, religar travava de novo sozinho; `base-solta.json`); (4) medição de base feita sob um teto desfeito é descartada (o 27 medido sob o 36 do dono); (5) a tela inicial mostrava os quadros apresentados como "medido pelo gerador": agora medido só o que chegou à tela, senão estimado; (6) o laboratório do modo clássico chamava de "custo na placa" um tempo de processador: agora mostra a placa medida; (7) texto dos Ajustes alinhado (com o Frame Boost o contador mostra o FPS do jogo) |
| Lab "Testar performance" | verificado por teste, **sem rodar com um jogo** | Três rodadas mínimas por lado, confiança alta/média/sem diferença/insuficiente; rodada com geração de quadros fica de fora |

## Verificado

| Item | Como foi verificado |
|---|---|
| Backend Rust compila | `cargo check` e `cargo build` sem erros |
| 1395 testes unitários passam | `cargo test --lib` nesta máquina na 2.9: 1395 passaram, 0 falharam, 50 ignorados (os ignorados são os que exigem hardware ou jogo aberto) |
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

### "Otimizar agora" e o Windows que chegou sem serviços essenciais

Em 10 e 11/09/2026, no PC do dono, o clique em "Otimizar agora" foi seguido de
"os programas não abriam" — nas duas vezes. O que o levantamento provou, e o que
não conseguiu provar:

**Provado.** A máquina roda uma imagem modificada ("Team AntiLag · SnyX OS")
com cerca de 180 serviços desativados, entre eles Plug and Play, Implantação
AppX, Licenças de Cliente, Isolamento de Chave CNG e Gerente de Contas de
Segurança. O Otimiza não desativa nenhum deles. O log do Windows mostra o plano
de energia sendo reaplicado segundos antes de cada reação do dono, e o histórico
do Otimiza **não registrou nada** nas duas vezes.

**Não provado, e por quê.** O Otimiza não guardava registro em arquivo, e nenhum
comando do lote tinha prazo. Não dá para dizer qual passo rodou, quanto demorou
nem onde parou.

**O que mudou:**

| Conserto | Onde | Trava |
|---|---|---|
| Todo comando tem prazo: `run` 60 s, PowerShell 120 s; estourou, o processo é encerrado e vira erro com o nome do comando. A sessão viva do PowerShell passou a ler por uma thread com canal | `shell.rs` | `a_sessao_que_trava_e_encerrada_no_prazo`, `run_com_prazo_encerra_o_programa_que_nao_termina`, `desistir_de_esperar_mata_o_processo…` (subiu do `diskspace.rs`) |
| Prazo longo onde a espera é legítima: aviso de administrador (10 min), `wsreset` (5 min), esvaziar a lixeira (10 min), ponto de restauração (3 min) | `commands.rs`, `diskspace.rs`, `restore.rs` | — |
| Registro em arquivo: `%APPDATA%\pc-optimizer\otimiza.log`, com milissegundos, girando em 2 MB. Cada aplicação e cada desfazer anotam o começo, **cada ação antes de executá-la**, e o fim com a duração | `utils/logger.rs`, `windows/mod.rs` | `cada_linha_chega_ao_arquivo_na_hora_e_em_ordem`, `o_arquivo_gira…` |
| Ponto de restauração pulado, com o motivo, quando a Cópia de Sombra de Volume ou o provedor dela estão desativados; um ponto que aparece depois do prazo continua contando como criado | `restore.rs` | `servico_desativado_e_dito_pelo_nome_e_no_plural_certo` |
| "Desligar aplicativos em segundo plano" saiu de todo lote e dos três perfis que o citavam; continua um a um | `catalog::FORA_DO_LOTE`, `profiles.rs` | `aplicativos_em_segundo_plano_nao_entram_no_lote`, `nenhum_perfil_cita_item_que_fica_fora_do_lote` |
| Antes de todo lote, a tela confere 11 serviços essenciais. Com algum desativado, avisa quais, o que cada um quebra (pela descrição da Microsoft) e o fabricante registrado, e oferece **Religar os essenciais**, **Otimizar mesmo assim** ou **Agora não** | `essenciais.rs`, `main.ts` | `nao_conseguir_ler_nunca_vira_desativado`, `religar_so_volta_para_um_padrao_do_windows`, `o_catalogo_nunca_desliga_um_essencial` |
| Religar volta cada serviço ao tipo de início **padrão** do Windows (tabela oficial da Microsoft), entra no histórico com id próprio, e o "Desfazer" os devolve a desligados | `WindowsOptimizer::religar_essenciais` | — |

**Não visto funcionando:**

- **O aviso na tela.** Tem teste de tipo e o motor foi lido nesta máquina (11
  essenciais, a maioria desativada), mas o modal só abre dentro do programa
  compilado.
- **Religar nesta máquina.** Mexer em serviço do Windows é decisão do dono, e o
  clique é dele. Alguns desses serviços podem recusar o `sc config` mesmo como
  administrador; a mensagem diz quais.
- **O gatilho do incidente (U.6).** Com o registro em arquivo pronto, o próximo
  passo é o dono aplicar os itens do lote um a um e, depois de cada um, conferir
  se um programa ainda abre. A última linha do `otimiza.log` diz o que rodou por
  último.

### A leitura de registro que falha deixou de virar lista vazia (C.1)

A 1.8 separou "não existe" de "não consegui ler" no `read` e no `delete_value`,
e parou ali. `value_names` e `read_text` continuavam achatando as duas, e
ficam embaixo de dezenas de leituras.

Agora as duas devolvem `Err` quando não conseguem ler. Chave que **não existe**
continua sendo vazio — a chave `Run` de um usuário novo não foi criada, e isso
é resposta, não falha (`chave_que_nao_existe_nao_tem_valores_e_nao_e_falha`).

| Onde a falha virava resposta | O que passou a acontecer |
|---|---|
| Lista de inicialização: "Nenhum programa nas chaves de inicialização" | A tela recebe o erro |
| Placa de rede com `ComponentId` ilegível descartada como virtual | O erro sobe; só as subchaves numeradas da classe são lidas como adaptador, porque `Properties` é negada a qualquer um (`so_subchave_numerada_e_adaptador`) |
| Preferência de placa: "o Windows está escolhendo sozinho para todos" | "Não consegui ler as preferências de placa gravadas" |
| DNS ilegível entrando como "automático, vindo do roteador" | O adaptador sai da comparação, e a linha atual diz que não leu |
| Jogos com preferência de placa gravada | Lacuna na biblioteca |

**Ficou achatado, de propósito e com o motivo escrito:** o conjunto de
executáveis que marca "sobe com o Windows" na lista de processos (ali não há
onde dizer "não li"), a enumeração de dispositivos PCI para o MSI (a lista
serve para escrever, e dispositivo que não se identifica não é tocado), o
fabricante na checagem dos essenciais e o PDF, que ainda não tem onde escrever
lacuna na seção de inicialização (C.4).

### Os quatro módulos que diziam "nada encontrado" sem ter lido (C.2)

Conflitos, tarefas agendadas, serviços de terceiros e programas de fábrica liam
pelo PowerShell com `-ErrorAction SilentlyContinue` e terminavam em
`_ => Vec::new()`. Leitura que falhava virava lista vazia, e lista vazia virava
verde.

Nesta máquina isso não era hipótese: o Agendador de Tarefas está parado e o
serviço de aplicativos da Loja desligado. As duas telas diziam "nenhum".

**Uma regra só**, em `shell::json_da_saida`: comando que não rodou, saída com
erro, saída vazia ou JSON que não casa viram `Err`; `[]` continua sendo lista
vazia. As quatro consultas passaram a `-ErrorAction Stop`, porque dentro da
sessão viva um erro que não interrompe o script chega como sucesso — e há trava
para isso não voltar (`as_quatro_consultas_param_no_erro`).

| Módulo | Antes | Agora |
|---|---|---|
| Tarefas agendadas | "Nenhuma tarefa de terceiros neste PC" | A tela mostra o erro do Agendador |
| Serviços de terceiros | "Nenhum serviço de terceiros neste PC" | A tela mostra o erro do WMI |
| Programas de fábrica | Lista sem nenhum app da Loja, calada | O que foi lido aparece, e o que faltou vai escrito junto ("Não consegui ler: …") |
| Conflitos | Fabricava "Nenhum conflito entre programas" em verde, que contava como verificação aprovada; escrevia "0 programas examinados" | O verde só aparece com tudo lido (`nenhum_conflito_so_aparece_quando_tudo_foi_lido`); sem lista, "programas instalados não lidos"; no veredito, leitura falha sem conflito vira lacuna |

E o "Encontrados: ." que o achado de nenhum conflito produzia no veredito — um
`measured` vazio, contra a regra de `achados.rs` — passou a dizer quantos
programas foram examinados.

**Os testes que leem a máquina de verdade** param com uma linha dizendo o que
não deu para ler, quando não dá — é o caso das tarefas agendadas aqui. Não há
o que afirmar sobre uma lista que não chegou; a regra do erro tem teste próprio,
sem depender de máquina quebrada.

### Os ajustes do driver NVIDIA ganharam tela (E.2)

O `nvdriver.rs` existia desde a 1.9 com o desfazer e sem o fazer. Agora há o
comando `aplicar_ajuste_nvidia` (exige licença), o painel na aba Jogos, e o
desfazer é o `revert_optimization` de sempre, com o id que o painel recebe
pronto do backend.

**Conferido nesta máquina (GTX 1650), sem escrever nada:** o driver devolve,
para cada número do catálogo, o nome esperado — `power management mode`,
`maximum pre-rendered frames`, `texture filtering - quality`, `vertical sync`,
`shader cache`.

**Um erro do catálogo, achado contra o `NvApiDriverSettings.h` oficial:** o
ajuste de textura se chamava "alto desempenho" e gravava `0x0A`, que é
`QUALITY_ENHANCEMENTS_PERFORMANCE`; "alto desempenho" é `0x14`. O valor ficou
(é o que a explicação promete e o que menos piora a imagem); o título passou a
dizer "desempenho" (`a_textura_grava_o_valor_que_o_titulo_diz`).

Se o histórico não gravar depois da escrita no driver, o ajuste é desfeito na
hora: mudança no driver sem registro é mudança sem volta pelo Otimiza.

**Não visto funcionando:** aplicar e desfazer de verdade. Os dois escrevem no
driver e exigem administrador, e nenhum teste desta suíte pode escrever no
driver. O caminho de escrita é o mesmo que o desfazer já usava desde a 1.9.

### Limite de FPS por jogo, no perfil do executável (E.3)

`FRL_FPS_ID` (`0x10835002`, de 1 a 1023) é escrito **só no perfil de um jogo,
nunca no global**: um limite global prenderia a área de trabalho e todo outro
jogo no mesmo número. O executável é o do jogo aberto, pelo detector de quatro
sinais da prova de FPS.

- Se o driver já amarra o executável a um perfil (às vezes criado pela NVIDIA),
  o limite vai nele e o valor anterior é guardado. Se não, o Otimiza cria o
  perfil "Otimiza - <executável>" e liga o executável a ele.
- **Nada fica pela metade:** criar perfil, ligar o executável e escrever o
  limite acontecem numa sessão da DRS que só é salva no fim. Qualquer falha
  descarta tudo.
- **Desfazer:** perfil criado pelo Otimiza é achado pelo nome e apagado inteiro
  — nenhum perfil com outro nome é apagado; perfil que já existia só tem o
  limite devolvido.

**Os números das cinco funções novas** (achar aplicativo, criar aplicativo,
criar, achar e apagar perfil) não têm portão de nome como os ajustes: um número
errado chamaria outra função. Foram conferidos em três tabelas independentes
(wiki do nvapi.net, lista do jNizM, `FunctionId.cs` do NvAPIWrapper), que também
batem com os onze números já confirmados pelo driver. As estruturas
(`NVDRS_APPLICATION_V3`, `NVDRS_PROFILE_V1`) seguem a documentação da NVIDIA, com
teste de tamanho (`as_estruturas_de_perfil_tem_o_tamanho_que_o_driver_espera`).

**Conferido nesta máquina, sem escrever:** o driver da GTX 1650 chama o
`0x10835002` de `frame rate limiter`.

**Travas:** executável e faixa validados antes de abrir a sessão, com trava da
ordem no código (`os_portoes_do_limitador_vem_antes_da_sessao`); os dois caminhos
conferem o número do limitador (`o_limitador_confere_o_numero_nos_dois_caminhos`).

**Uma limitação, dita na tela:** o FiveM troca o nome do executável quando o
servidor exige outra versão do jogo (`FiveM_b3258_GTAProcess.exe`), e o limite
fica no nome antigo — é preciso limitar de novo com o jogo aberto.

**Não visto funcionando:** criar o perfil, aplicar o limite e desfazer. Escrevem
no driver, e o primeiro teste de verdade é com o dono, o FiveM aberto e o
Otimiza como administrador.

### A prova que acontece sozinha (E.4)

A prova de antes e depois só existia quando o cliente lembrava de medir. O
cliente do reembolso nunca mediu nada, e ninguém tinha um número das partidas
dele.

Agora um vigia à parte (`modules::medicoes`) olha a cada 30 s se há jogo em
primeiro plano. Com o mesmo jogo aberto há **3 minutos** e o Otimiza como
administrador, ele mede **20 s** pelo canal de eventos do Windows (o mesmo
`frames.rs`, sem tocar no jogo) e guarda FPS, 1% piores, engasgos e **quantas
mudanças do Otimiza estavam aplicadas naquele momento**. No máximo uma medição a
cada **20 minutos**; guarda as últimas 60.

- **Não compara uma medição com outra.** Cada uma foi feita num lugar diferente
  do jogo. A tela da prova mostra as últimas dez lado a lado, com a frase que
  diz que elas não são antes e depois.
- **Não atrasa o vigia do modo jogo:** detecção e medição rodam fora do runtime,
  num laço próprio.
- **Tentativa que falha conta como tentativa:** medição recusada (anticheat,
  outra medição em andamento) não é repetida a cada meio minuto.
- **Histórico ilegível não é sobrescrito** por uma medição nova
  (`historico_ilegivel_nao_e_apagado_por_uma_medicao_nova`), e ilegível não vira
  "nenhuma medição".
- Preferência nova, `medir_quadros_sozinho`, **ligada por padrão** — ao
  contrário do modo jogo automático, porque medir não muda nada no sistema. Quem
  vem da 1.9 recebe ligada (`quem_atualiza_da_1_9_passa_a_ter_a_medicao_sozinha_ligada`).

**Não visto funcionando:** uma medição automática de verdade — pede o FiveM
aberto por três minutos com o Otimiza como administrador. As regras de quando
medir têm teste puro (`o_acompanhamento_mede_so_o_mesmo_jogo_aberto_sem_parar`,
`sair_do_jogo_ou_trocar_de_jogo_recomeca_a_contagem`).

### A expectativa honesta, antes de pagar (E.5)

A tela de compra mostrava só o achado principal. Agora ganha uma linha, logo
abaixo, com o que o Otimiza consegue e o que não consegue **neste** computador:

- quantos dos problemas achados o Otimiza corrige sozinho (achados com `acao`);
- o que ele não resolve porque é peça (achados com `fix_location` Hardware),
  pelo nome, no máximo dois, com "isso é peça, e nenhum programa troca peça".

Sai inteiro do diagnóstico que já rodou: nenhum número novo e nenhuma promessa
de FPS. Nasceu do reembolso — o cliente aplicou tudo, o jogo continuou igual, e
o teto dele era peça. Dizer antes perde só a venda que não ia dar certo.

**Não visto funcionando:** o portão só aparece sem licença, e esta máquina tem.

## A 2.9 — a auditoria, e o que ela mudou

A versão começou por uma auditoria de **tudo** o que o produto escreve no
Windows (`docs/AUDITORIA-2.9.md`), com a régua do dono: nunca menos FPS, sem
divisão entre tipo de jogador, e fora o que não muda nada.

### O que saiu

Vinte ajustes. Dezoito por não mudarem FPS, fluidez, atraso, carregamento nem
proteção — entre eles `SystemResponsiveness`, `Win32PrioritySeparation`,
`NetworkThrottlingIndex`, MMCSS de jogos, desligar SysMain e desligar a
compressão de memória. Mais as duas limpezas irreversíveis, que foram para a
tela de Limpeza, onde o cliente vê o que se perde antes de apagar: **o catálogo
inteiro passou a ter desfazer**. Saíram também o botão "Priorizar" do FiveM, o
plano de energia máximo sugerido pelo diagnóstico e quatro painéis repetidos.

Um caso merece nome próprio: `PowerThrottlingOff=1` desligava o EcoQoS — o
mesmo mecanismo que o modo jogo novo usa para tirar programa de fundo da
frente do jogo. O produto brigava consigo mesmo.

### O que foi consertado

| Defeito | Onde |
|---|---|
| A categoria "entrega otimizada" do liberador apagava `ProgramData\Microsoft\Network\Downloader`, que é a **fila de downloads do BITS** | `diskspace.rs` |
| "Desempenho máximo" e V-Sync forçado eram escritos no perfil **global** da NVIDIA — o primeiro mantém a placa acordada na área de trabalho, o segundo quebra G-SYNC | `nvdriver.rs` |
| Mexer nos núcleos do jogo **não consultava o anticheat** | `afinidade.rs` |
| O plano de energia podia escrever a receita fixa de processador por cima do motor que mede | `planoenergia.rs` |

### O que foi visto funcionando nesta máquina

| Item | O que foi medido |
|---|---|
| Sensores da placa (`nvidia-smi`) | GTX 1650: 35 °C, 15 W de 75 W, clock 1485 de 2175 MHz, sem carga |
| Prévia do perfil NVIDIA por jogo | Lido do driver, sem escrever: energia e fila de quadros já no valor do perfil, textura em "alto desempenho" |
| Regra "nunca menos FPS" no perfil | A prévia mostrava textura indo de "alto desempenho" para "desempenho" — mais lento. Virou trava: ajuste já igual ou mais rápido fica como está |
| DPC por núcleo | 10 s medidos: núcleo 0 com ~0,9% de DPC e ~1,6% de interrupção, resto em zero. Veredito: normal |
| MSI por dispositivo | 11 dispositivos PCI lidos: NVMe, SATA, rede, USB e a GTX 1650 com MSI ligado; áudio HD declarado e desligado |
| Origem do driver de vídeo | `DriverProviderName` = NVIDIA (não é o genérico do Windows) |
| Planilha de alterações | Gerada do histórico real desta máquina, com data, id, alteração e valor anterior |
| Registro central | 44 alterações listadas, e a trava achou um módulo que escrevia sem estar nela (a afinidade) |

### O que a 2.9 NÃO viu funcionar

| Item | Por quê |
|---|---|
| **Auto CPU Set** | Esta máquina tem 8 núcleos lógicos iguais. Em processador uniforme o botão fica desligado de propósito — prender o jogo em parte dos núcleos só tiraria máquina. Falta um híbrido (Intel 12ª geração ou mais novo) |
| **Escrever e desfazer um perfil NVIDIA no driver** | A leitura foi feita; a escrita no driver e o desfazer dela não foram executados |
| **As telas novas abertas no programa** | Modo Expert, MSI, DPC, registro central, "Suas últimas partidas" e "Otimizar e testar" compilam e passam no `tsc`, mas não foram vistas na janela |
| **O portão desfazendo um perfil NVIDIA sozinho** | Exige três partidas medidas antes e três depois, com a medição automática ligada |

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
