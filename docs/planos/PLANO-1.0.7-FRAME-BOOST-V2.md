# Plano: Otimiza 1.0.7 (Frame Boost v2, zero bug, novos agentes e skills)

## Contexto

O dono não aceita o Frame Boost como está. Os clientes sentem os quatro defeitos ao mesmo tempo:
- **imagem com defeito:** borrão e fantasma na mira, na HUD e em movimento rápido;
- **pesado na mão:** atraso;
- **trava ou desliga sozinho;**
- **tela confusa.**

O pedido também inclui "nenhum bug" e muito mais agentes e skills, para levar o Otimiza a outro patamar.

**Como o Frame Boost é hoje** (investigado em `modules/windows/geracao/`, cerca de 8,4 mil linhas):
- **Captura:** só a DXGI Desktop Duplication (`mod.rs:815`).
- **Fluxo óptico:** DIS na GPU (`shaders.hlsl` `DisPedaco` :814, `DisDenso` :894), com a pirâmide no máximo em 1/4 da resolução.
- **HUD:** protegida só por uma máscara temporal (`Mascara` :319). A mira não tem proteção.
- **Portão de qualidade:** existe só nos testes (`gpu.rs:677` `fracao_ruim`).
- **Apresentação:** sobreposição transparente via DirectComposition.
  - No 2×–4× fixo usa `Present(0)` com uma agenda feita em software (`ritmo::agenda`).
  - Uma thread só faz captura, GPU, apresentação e mensagens. A função `sessao` tem 738 linhas (`mod.rs:1431`).
- **Guarda** (`guarda.rs`): pausa 1,5 s para comparar o FPS do jogo com e sem geração. Esse tranco é visível, e ela desliga tudo de uma vez.
- **Tela:** `FrameBoost.svelte` tem 606 linhas e cerca de 53 condicionais.

**O que a pesquisa trouxe** (com fontes, no relatório desta sessão):
- A GTX 1650 é TU117: não roda o NVIDIA FRUC/Optical Flow, nem o Smooth Motion, nem o AMD AFMF. O gerador próprio precisa ser excelente.
- O Lossless Scaling oferece:
  - captura DXGI ou WGC;
  - "queue target" 0/1/2;
  - "flow scale" (fluxo calculado em resolução menor);
  - modo desempenho;
  - detecção de HUD.
- O Magpie usa WGC por padrão. A Desktop Duplication trava em janela sem bordas com VRR.
- A NVIDIA DRS `PRERENDERLIMIT_ID 0x007BA09E` (fila = 1) reduz o atraso no DX11.
- Licenças permissivas disponíveis: FSR1/FidelityFX (MIT), NIS (MIT), SGSR1 (BSD-3). O RIFE é lento demais para uma GTX 1650 em tempo real.

**Decisões do dono:**
- o padrão para todos é **Equilíbrio**;
- a **fila NVIDIA = 1 entra junto com o Frame Boost** (com registro no Histórico e desfeita ao desligar);
- a **captura é automática** (WGC ou Duplication, com fallback);
- a **proteção da mira vem ligada**, numa área de cerca de 3%, ajustável.

**Regras de sempre:**
- nunca menos FPS real; tudo reversível; medido ≠ estimado;
- notas de versão como correção de bugs;
- build com `CARGO_TARGET_DIR=.cargo-target -j 2`, uma tarefa pesada por vez (o C: está com cerca de 20 GB livres);
- teste que lê a máquina leva `#[ignore]`;
- publicar só com o ok explícito do dono.

---

## Onda 0: agentes e skills (só documentos, sem build)

**Agentes novos** (`.claude/agents/*.md`, no mesmo formato dos existentes):

| Agente | O que faz |
|---|---|
| `laboratorio-frame-boost` | Roda o laboratório de qualidade (Onda 1) e lê as métricas: SSIM, fantasma, erro na HUD e ritmo. |
| `latencia-e-ritmo` | Revisa a apresentação, a fila e o ritmo, e lê as colunas do PresentMon (PresentMode, msBetweenDisplayChange, latência). |
| `guardiao-da-escrita` | Escrita atômica, panic nos laços, marcadores de queda e encerramento que nunca trava. |
| `auditor-de-seguranca` | Licença, atualizador, ações elevadas e o que entra em comando do PowerShell. |
| `custo-do-otimiza` | Quanto o próprio Otimiza custa com o jogo aberto: processador abaixo de 0,5%, DPC, sessões ETW e laços. |
| `especialista-rede` | A área Rede: o que é fato de rede e o que é mito. |
| `especialista-energia` | A área Energia: planos, EPP, X3D e Intel híbrido. |
| `especialista-mira-cores` | As áreas Mira e Cores. |
| `especialista-bios` | A área BIOS. |
| `instalador-e-atualizador` | Instalador, atualizador e o desinstalador que restaura o PC. |
| `triagem-de-suporte` | Leva a reclamação até o log certo e a causa. |
| `redator-de-notas` | Escreve as notas de versão como correção de bugs. |

**Skills novas** (`.claude/skills/<nome>/SKILL.md`):
- `frame-boost-lab`: gravar o clipe, rodar o laboratório e ler o relatório.
- `medir-latencia`: o PresentMon no jogo e na sobreposição.
- `onda-segura`: o ritual de cada onda (build, testes, `guardiao-do-fps`, `revisor-otimiza`, commit).
- `escrita-atomica`.
- `notas-da-versao`.
- `triagem-de-relato`.

**Como conferir:** o `revisor-otimiza` checa o formato. Atualizar também o `MEMORY.md` e a memória `otimiza-atualizacao-1-1.md`.

---

## Onda 1: o laboratório de qualidade (vem antes de mexer na imagem)

1. **`geracao/metrica.rs` novo, puro e só no processador.** Trabalha com quadros BGRA e calcula:
   - SSIM e PSNR, inclusive por bloco;
   - cintilação;
   - erro na região da HUD;
   - "fantasma" (borda dobrada em volta do que se mexe);
   - ritmo: p50, p99 e a fração de intervalos acima de 1,5× a mediana.

   Testes com imagens sintéticas: quadrado andando, mira fixa sobre fundo em panorâmica e corte de cena. Rodam no `cargo test` normal.
2. **`Gpu::nova_warp()`** ao lado de `gpu.rs:350`, usando a GPU em software do Windows. Os shaders reais rodam em 320×180 sobre pares sintéticos e são pontuados pelo `metrica.rs`, sem `#[ignore]`. Isso mede qualidade, nunca custo.
3. **Laboratório com clipe gravado** (`#[ignore]`):
   - grava 2–3 s do FiveM a 120–144 FPS pela captura que já existe;
   - gera o quadro do meio a partir de A e C e compara com o B real;
   - salva um relatório JSON comparável entre versões, fora do C:, em `.cargo-target/lab`.

---

## Onda 2: qualidade da imagem (defeito 1)

Mudanças em `shaders.hlsl` e `gpu.rs`:
1. **Proteção da mira:** um retângulo no centro, padrão de cerca de 3% e ajustável, que sempre vem do quadro real mais próximo. Fica perto de `FinalBidirecional` :1007.
2. **Detecção de HUD:** pixel parado por N quadros reais enquanto o fluxo em volta é grande é recomposto a partir do quadro real, sem deformar. É a técnica que a AMD recomenda no FSR3.
3. **Recuo por confiança:** o `Consistencia` (:975) grava a confiança por bloco. Onde ela é baixa, o pixel recua para o quadro real ou para a mistura.
4. **Portão de qualidade ao vivo, sem travar o processador:** a fração ruim é reduzida na GPU para 1×1 e lida numa fila de 3 texturas de leitura, com um quadro de atraso. Liga o `ritmo::quadro_aprovado` (`ritmo.rs:191`) ao caminho ao vivo: quadro reprovado repete o real.
5. **"Escala do fluxo"** (1/4, 1/6 ou 1/8) a partir do `ritmo::niveis` (`ritmo.rs:333`), escolhida pelo preset. Na GTX 1650, a 1080p ou mais, usa 1/8.
6. **Corte de cena com histerese:** um quadro ruim sozinho não liga e desliga a geração.

**Critério:** o erro na HUD fica menor ou igual ao de simplesmente repetir o quadro real, e o fantasma cai no laboratório. Depois, sessão real no FiveM com o dono e os testadores.

---

## Onda 3: apresentação e atraso (defeito 2)

1. **Dividir a `sessao`** (`mod.rs:1431`) em `captura.rs`, `apresentacao.rs` e `sessao.rs`. Ficam duas threads:
   - captura + GPU;
   - apresentação, que espera o waitable da swap chain e cuida das mensagens.
2. **Prioridades:**
   - `IDXGIDevice::SetGPUThreadPriority(+7)`, conferido e registrado no log;
   - `AvSetMmThreadCharacteristicsW("Games")` na thread de apresentação, com a feature `Win32_System_Threading` no crate `windows`;
   - tudo desfeito ao sair.
3. **Fila 0/1/2 também no 2×–4× fixo:** passa a usar o waitable com `SetMaximumFrameLatency(1)` (`mod.rs:1118`) no lugar do `Present(0)` (:1202). Mantém a agenda do `ritmo::agenda`.
4. **Fila NVIDIA = 1** (`PRERENDERLIMIT_ID 0x007BA09E`) no perfil do jogo, junto com o Frame Boost. Reaproveita `aplicar_limite` e `desfazer_perfil_do_jogo` (`nvdriver.rs:917`, `:1033`) e o `ler_anterior_para_gravar` da 1.0.5. Fica no Histórico e é desfeita ao desligar e ao fechar o jogo, pelo caminho do `teto_do_jogo::ao_fechar_o_jogo`.
5. **Atraso medido de verdade:** para o mesmo quadro, o momento em que o jogo apresentou (`presentmon::Vivo`) contra o momento em que a sobreposição exibiu ("OtimizaNaTela"). A tela diz "medido" ou "estimado". Estende `AtrasoDaExibicao` (`ritmo.rs:159`).
6. **Experimental, desligado:** janela opaca de tela inteira em flip model, tentando o independent flip. Conferir o `PresentMode` pelo `presentmon.rs:162` e voltar para a composição se não pegar.

---

## Onda 4: estabilidade (defeito 3)

1. **Guarda sem tranco visível** (`guarda.rs`):
   - vigia contínua: o tempo de quadro do jogo pelo `Vivo` contra uma base de 5 s gravada antes de ligar, o custo de GPU do gerador (`Cronometro`, `gpu.rs:1799`) e o NVML;
   - só pausa quando há sinal de perda;
   - a pausa vira 4 micro-pausas de 250 ms;
   - durante a pausa, o quadro real continua passando, e a sobreposição nunca some.
2. **Degrau em vez de desligar:** `Acao::Descer` desce 4× → 3× → 2× → leve → só escala → desligado, com o motivo na tela.
3. **Captura automática WGC/Duplication** atrás de um `trait Captura`:
   - as features `Graphics_Capture` já estão no `Cargo.toml`;
   - usa `CreateForWindow` na janela do jogo, frame pool free-threaded e `SystemRelativeTime`;
   - escolhe WGC no Windows 11 para janela sem bordas com VRR;
   - volta para a Duplication se falhar ou se `IsBorderRequired` não puder ser desligado.
4. **Histerese na regra de esconder a sobreposição** (500 ms sem quadro): menu e pausa não contam como falha. Todo motivo de parada vai para o `resumo.rs` e para a tela.

---

## Onda 5: tela simples (defeito 4)

O `FrameBoost.svelte` passa a ter:
- um interruptor grande;
- presets **Qualidade / Equilíbrio (padrão) / Desempenho**, cada um definindo modo, escala do fluxo, fila e nitidez;
- uma linha de estado: FPS medido, atraso medido e o motivo do último degrau;
- "Avançado" recolhido, com o que existe hoje e o tamanho da área da mira.

É dividido em componentes (`FrameBoostStatus`, `Presets`, `Avancado`), só com tokens, pela skill `tela-otimiza`, e revisado pelo `designer-otimiza`.

A escolha que a pessoa já tinha feita continua valendo (`fb_por_jogo`). O Equilíbrio é só o padrão de quem nunca escolheu.

---

## Onda 6: robustez ("nenhum bug"), em pedaços ao longo das ondas

1. **`utils/arquivo.rs::escrever_atomico`** (arquivo temporário, flush e rename). Junta as 8 cópias que já existem (`portao.rs:478`, `lab.rs:214`, `overlay.rs:309`, `cores.rs:246`, `baseline.rs:410`…) e troca as escritas que não são atômicas ou que jogam o erro fora:
   - `teto_do_jogo.rs:117`;
   - `base_do_jogo.rs:98/123/165`;
   - `catalog.rs:316`;
   - `estabilidade.rs:134`;
   - `fov_fivem.rs`;
   - `contador_fivem.rs`;
   - `portao.rs:504`;
   - `feedback.rs`.

   Erro de escrita vai para o log.
2. **`vigiar(nome, tarefa)`:** pega o panic dos laços de fundo (`lib.rs:453`–`894`), registra e reinicia com espera. Os laços saem do `run` (1195 linhas) para `lib/rotinas.rs`.
3. **Marcadores de queda conferidos:** `motorenergia_maquina.rs:322`, `turbo.rs:1141`, `restaurar.rs:397` e `frame_boost.rs:117`.
4. **`hz.rs:802/1191`:** sem `expect`; um erro desfaz a troca de Hz.
5. **Encerramento:** o gerador para em até 2 s (join com tempo-limite), com teste.
6. **UI:** os 159 valores fixos em px passam para tokens, pela skill `tela-otimiza`.

---

## Verificação (em toda onda)

1. **Rust:** `cargo test --lib -j 2` (hoje 1997 testes), mais os testes puros de cada mudança, os de WARP dos shaders e os de métrica. `OTIMIZA_GERAR_TIPOS=1 cargo test --lib contrato` quando um comando mudar.
2. **UI:** `npx svelte-check` sem erros, `npm run testes:ui`, `npm run fumaca:nova` e os prints claro e escuro lidos pelo `designer-otimiza`.
3. **Revisão:** `guardiao-do-fps` e `latencia-e-ritmo` em toda onda do Frame Boost, `revisor-otimiza` e `guardiao-da-escrita` na robustez.
4. **Laboratório:** relatório JSON antes e depois de cada mudança na imagem.
5. **No jogo, com o dono, o sócio e o cliente da GTX 1650:**
   - FiveM com o PresentMon;
   - log pelo botão "Salvar em Downloads", lido pelo `analista-de-log`;
   - a frase "menos fantasma, menos atraso, sem tranco" só entra nas notas com essa prova.
6. **Publicação:** skill `publicar-versao` mais `verificador-de-publicacao`, só com o ok explícito do dono.

**O que não dá para provar aqui:** o fantasma real no FiveM, o custo numa GTX 1650, a latência, a compatibilidade da WGC e o independent flip. Esses dependem dos testes no PC do dono e dos testadores.

**Ordem:** 0 → 1 → 2 → 4 → 3 → 5, com a 6 em paralelo, em pedaços. Cada onda é commitada e testada antes da próxima.
