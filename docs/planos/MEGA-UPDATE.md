# OTIMIZA — Mega Update: Auditoria + Plano (Fases A–E) e primeira entrega (Fase F)

## Context
O dono quer transformar o OTIMIZA de "otimizador de FiveM" em "plataforma de performance por jogo", com
resultado medido (FPS nativo, 1% low, latência, Frame Boost), sem números falsos e com UI nova. Antes de mexer
no código, foi pedida uma auditoria do repositório real e um plano incremental. Também foi decidido trazer um
gate de feedback pelo Discord no estilo do concorrente "Beyond" (as screenshots são DELE, não do OTIMIZA —
servem só de referência do que não copiar). Decisões do dono:
- O gate abre **após 10 minutos de uso acumulado** depois da ativação da chave paga. Só libera quando há
  feedback no canal `1543436859529629707` (≥30 caracteres + print com FPS). Se o feedback for apagado, trava de novo.
- Validação por **endpoint assinado** no qrbot (Square Cloud): `GET /v1/feedback/{OTZ}` → JSON assinado Ed25519.

---

## FASE A — Auditoria (resumo do que foi encontrado)

**Stack**: Tauri 2 (Rust) + TypeScript puro/Vite (sem framework). Versão 3.2.0, só Windows.
Site `web/` = Next.js 16 estático no GitHub Pages (landing, checkout Pix/cartão via API externa, painel por licença
em localStorage). Bot de licença = `pc-optimizer/bot/otimiza-licenca.cjs` (assinador Ed25519 sem dependências).
O bot de vendas real (qrbot) fica FORA do repositório.

**Tamanho**: ~74,6k linhas de Rust, 150 `#[tauri::command]`, ~1.500 `#[test]`. Frontend com 21,7k linhas:
`main.ts` 8.887, `styles.css` 6.786, `index.html` 2.325.

**Mapa**
- `src-tauri/src/core/` — telemetria, PDH, sensores, fluidez, travadas, estatística.
- `src-tauri/src/modules/` — benchmark, baseline, histórico, medições, portão ("nunca menos FPS"), repetições,
  prova/provaalternada, gargalo, latência, licença, convite, atualização (banner via GitHub API), relatório.
- `src-tauri/src/modules/windows/` — ~100 módulos: jogos (Steam/Epic/registro), fivem/citizenfx/configjogo,
  framegen + `geracao/` (gerador próprio), presentmon (sidecar), nvdriver (NVAPI DRS), nvml, display/vrr,
  planoenergia/motorenergia, catalog (catálogo de tweaks), naofazemos (lista de placebo), anticheat, restore/restaurar.
- Frontend: barra lateral com 5 áreas (Início, Otimizar, Jogos [Jogos/Geração de quadros/Placa], Sistema
  [Windows/Energia/Núcleos/BIOS], Histórico) + portão de licença + paleta Ctrl K.

**O que funciona e deve ser preservado (é o diferencial do produto)**
- PresentMon como fonte única de frames (3.2), que já separa frames do jogo dos frames gerados (Lossless/AFMF/OTIMIZA).
- `framegen.ts` já marca cada número como MEDIDO/ESTIMADO/DESCONHECIDO (`selo()`), o que atende a regra §3.
- Gerador de quadros próprio (`geracao/`): fora do processo, Desktop Duplication → movimento na GPU → janela
  DirectComposition por cima. Não injeta nada (seguro com anticheat) e expõe `fps_reais`, `fps_apresentados` e `atraso_ms`.
- Portão "nunca menos FPS" (`portao.rs`): desfaz sozinho se piorar ≥5% com significância estatística.
- Filosofia de tweaks com evidência (`naofazemos.rs`, `evidencia.rs`, níveis) e reversibilidade byte a byte.
- Licença Ed25519 offline (`licenca.rs`) e convite remoto validado (`convite.rs`).

**Dívida técnica / riscos**
1. `main.ts` monolítico (8,9k linhas, 111 `innerHTML`, ~28 `let` globais). Os planos internos já o declaram "não reaproveitável".
2. CSS com dois esquemas de tokens, componentes duplicados (`.btn` ×4, `.modal` ×2) e tokens que se referenciam (`--sup-selecionada`).
3. `commands.rs` com 4.352 linhas e `windows/mod.rs` com 3.473. Pares duplicados (conflicts/conflitos,
   registry/registro, restore/restaurar). 12 comandos que a UI nunca chama e `autoajuste.rs` morto.
4. Acoplamento ao FiveM: perfis, medições de campo e `configjogo.rs` são centrados nele. Não existe abstração `GameProfile`.
5. `anticheat::permite` só é chamado por 3 módulos e `anticheat::pode` é código morto. Ainda não é um gate obrigatório.
6. "custa X dos quadros" (`configjogo.rs:84-128`, exibido em `main.ts:6568`) são faixas fixas sem rótulo ESTIMADO, o que viola §3.
7. Bugs conhecidos do gerador próprio (A3.2 §6.3): Hz do monitor primário em vez do monitor do jogo, iGPU em
   laptops, formato fixo B8G8R8A8/sem HDR, morre ao redimensionar, latência estimada.
8. Sem updater Tauri e sem assinatura de código (SmartScreen avisa).
9. A prova alternada só detecta efeitos grandes (~22% no FPS médio, ~53% no 1% low), o que limita o "ganho real" que dá para afirmar.

10. **`gerador_ligar` (`commands.rs:1424`) só confere a licença e NÃO passa por `anticheat::permite`.** O enum `Acao`
    não tem caso para geração de quadros. Um overlay topmost + Desktop Duplication pode ser sinalizado por
    anticheats que inspecionam overlays. → Corrigir no M2 (Vanguard/FACEIT: bloquear ou avisar).
11. Sobreposições reais: `telemetry.rs`×`core/telemetria.rs`, `bottleneck.rs`×`gargalo.rs`, `frames.rs` (ETW legado)×`presentmon.rs`,
    `benchmark.rs`×`baseline.rs`, `historico.rs`×`medicoes.rs`, três módulos de energia, `safety.rs` legado. (conflicts/conflitos,
    registry/registro e restore/restaurar NÃO são duplicatas, cada um tem papel distinto.)
12. `csp: null` no Tauri. 54 chamadas de PowerShell/WMI (custo e fragilidade). Teste de integração vazio.

**A.2 Detalhes do backend relevantes ao plano**
- Medição real: PresentMon 2.6 com `--track_frame_type`. 1%/0.1% low = média dos piores 1%/0.1% (mínimo de 1.000/10.000 frames, senão `None`). Fallback ETW próprio. Frames de AFMF/XeFG são identificados. **DLSS FG e Smooth Motion não se identificam** (`presentmon.rs:18`).
- Detecção de jogo já é genérica (`deteccao.rs`: janela em tela + motor 3D + tempo + lista de exclusão). Os nomes ficam em `gamemode.rs:20-38` (15 jogos). O acoplamento ao FiveM está em `configjogo.rs`, `receita.rs` (só fivem/gta5) e `framegen.rs:194-223`.
- Loop automático (`lib.rs:465-733`): detecta jogo → mede 20 s → grava `medicoes.json` → portão. Emite `prova:automatica`. **É a base do "Início durante jogo" e do histórico por jogo.**
- Licença: `licenca::exigir()` protege 34 pontos de comando, e o gate de feedback deve entrar no mesmo lugar. Dados em `%APPDATA%\pc-optimizer\`. ID de máquina em `maquina.rs`.
- Catálogo: 41 specs, 30 aposentadas (mantidas só para desfazer) e 11 oferecidas. Já classificado (Essencial/Condicional/Expert, `ExpectedGain`, `RiscoDeFps`), o que cobre a classificação de §45.
- Segurança existente: `changelog.rs` (valor anterior), `transacao.rs` (diário de intenção), `restore.rs`, `restaurar.rs`, `guardiao.rs` (deriva), safe mode após 2 crashes (`utils/diagnostico.rs`). Esta é a base de §42/§43; não precisa criar outra.

---

## FASE B — Viabilidade (fontes oficiais, out/2026)

| Tecnologia | Veredito para o OTIMIZA | Como |
|---|---|---|
| NVIDIA Smooth Motion (RTX 40/50, via NVIDIA App/driver) | **PARCIAL** | Detectar: só pela leitura do perfil DRS (o PresentMon NÃO distingue frames do Smooth Motion; hoje `framegen.rs` só infere pela arquitetura). Ativar: não há setting oficial no `NvApiDriverSettings.h`, só o ID comunitário 0xB0D384C0. → Recomendar e guiar o usuário na NVIDIA App. Escrever o ID não documentado só atrás de flag experimental, com rollback. |
| DLSS FG / MFG (Streamline) | **IMPOSSÍVEL externamente** | Streamline/NGX são SDKs que o jogo integra. O OTIMIZA só pode detectar se o jogo tem e recomendar ligar no menu do jogo. A NVIDIA App tem override de DLSS em jogos que suportam. |
| NVIDIA Reflex | **IMPOSSÍVEL de injetar** (anticheat) | Só existe se o jogo integra. Alternativa legítima: Low Latency Mode no perfil DRS (documentado: PRERENDERLIMIT/ULL) + limitador de FPS do driver, que já existe (`limitar_fps_nvidia`). |
| AMD AFMF / AFMF 2 | **POSSÍVEL** | ADLX oficial: `IADLX3DAMDFluidMotionFrames(1)` com IsSupported/IsEnabled/SetEnabled e algoritmo/modo. Precisa de FFI com o ADLX (DLL do driver). Exige tela cheia, é global ou por app conforme o driver. |
| FSR 3/4 FG | **IMPOSSÍVEL externamente** | Integração no jogo. Só detectar e recomendar. |
| AMD Anti-Lag 1 | **POSSÍVEL** | ADLX `IADLX3DAntiLag`. |
| AMD Anti-Lag 2 | **IMPOSSÍVEL externamente** | SDK que o jogo integra (GPUOpen). Só detectar e recomendar. |
| Intel XeSS FG / MFG override | **PARCIAL** | Funciona só em jogos com XeSS 2 FG, via override no Intel Graphics Software. Controle externo via IGCL: **PRECISA DE POC**. No começo, só detectar e guiar. |
| FG externo universal (próprio, estilo LSFG) | **JÁ EXISTE (beta)** | `geracao/`. Corrigir os bugs §6.3. É a única opção universal para FiveM (não tem FG nativo), mas acrescenta ~1 frame de latência e exige janela sem bordas. |
| Medição de frames apresentados | **POSSÍVEL** | PresentMon (já é sidecar) separa frames do app e gerados (colunas FrameType/MsBetweenDisplayChange). |

**Suposições do pedido que precisam de correção (§60)**
- "Frame Boost reduz latência / melhora 1% low": **falso.** FG aumenta a latência e não muda o 1% low renderizado.
  A UI mostra Frame Boost e Latency Boost como métricas separadas, nunca somadas.
- "52 → 204 FPS em FiveM": só seria alcançável com o gerador próprio em 4× e num monitor ≥240 Hz, com artefatos e +latência. No
  FiveM não há DLSS/FSR FG. Decisão do dono: mostrar só o "FPS na tela" medido (ver "Regra de exibição do Frame Boost").
- "Latência 31→18 ms": sem Reflex no jogo, o OTIMIZA não MEDE a latência de clique até o pixel. PresentMon dá
  uma estimativa de latência de render (*PC latency* só aparece com Reflex markers). A UI deve rotular como ESTIMADO.
- "+23% potencial estimado" na Home: os planos internos (PIOS §26) proíbem predição pontual. Usar faixa com base
  no histórico local e mostrar só depois de existir baseline daquele jogo naquela máquina.
- "Perfil competitivo vs. RP": o dono já decidiu, só Max FPS / Balanceado / Qualidade. Mapeamento: Competitivo=Max FPS.

---

## FASE C — Arquitetura proposta (evolução, não rewrite)

Novo módulo Rust `src-tauri/src/plataforma/` que *orquestra* o que já existe:

```
plataforma/
  hardware.rs     HardwareProfile (cache em disco + invalidação por driver/monitor). Reusa windows/hardware.rs, nvml.rs, display.rs, vrr.rs, topologia.rs
  jogos/
    mod.rs        trait GameAdapter { id, executaveis, renderer(), anticheat(), config_files(), tweaks_suportados() }
    registro.rs   catálogo de adapters (FiveM, GTA V, CS2, Fortnite, Valorant, CoD, Minecraft, Roblox, Cyberpunk, RDR2, Forza, Apex, Rocket League) + GenericAdapter
    fivem.rs      wrapper sobre windows/fivem.rs + citizenfx.rs + configjogo.rs (sem mover a lógica)
  sessao.rs       detecta o jogo em execução (polling leve de processos 2 s → 10 s quando nada roda) e emite `jogo:detectado`/`jogo:fechou`
  metricas.rs     modelo único: NativeFps, PresentedFps, GeneratedFps, FrameTime, P1Low, P01Low, P95/P99, Latencia{valor,origem}, RefreshRate, cada um com Origem {Medido, Estimado, Desconhecido}. Fonte: presentmon.rs
  compat.rs       CompatibilityEngine: (Feature, Jogo, GPU, Renderer, Modo de tela) → Suportado/Parcial/Nao{motivo}. Absorve anticheat::permite como gate obrigatório
  frameboost/
    mod.rs        trait FrameBackend { detectar(), disponivel(&Contexto)->Compat, ativar(), desativar(), estado()->Metricas }
    otimiza.rs    adapter sobre windows/geracao (existente)
    amd_afmf.rs   ADLX (fase 2, PoC)
    nvidia_sm.rs  detecção e guia; escrita DRS atrás de flag
    nativo.rs     "o jogo tem DLSS/FSR/XeSS FG: ligue no menu"
    auto.rs       escolhe: nativo > driver (AFMF/SM) > OTIMIZA; nunca oferece incompatível
  latencia.rs     NVIDIA ULL / AMD Anti-Lag 1 / limitador; reusa modules/latencia.rs
  lab.rs          fluxo Testar Performance: baseline → aplicar → medir → comparar (reusa benchmark.rs, baseline.rs, repeticoes.rs, portao.rs) com nível de confiança
  historico.rs    histórico por jogo (JSON em %APPDATA%/Otimiza/jogos/<id>.json): data, hw hash, driver, perfil, baseline, resultado, FG
  feedback.rs     gate de 10 min + verificação assinada (seção abaixo)
  flags.rs        feature flags em preferences: newHome, gameLibrary, performanceLab, frameBoostAuto, newShell
```

**Regra de exibição do Frame Boost (decisão do dono)**
- Com o Frame Boost ligado, a tela mostra **só o FPS apresentado** (ex.: "204 · FPS na tela", selo FRAME BOOST).
  O FPS nativo (52) **não aparece** na tela do Frame Boost, no card do Início nem no card do jogo na biblioteca.
- O número precisa ser o *PresentedFps* **medido** (PresentMon no PID do gerador/jogo; no gerador próprio, conferido contra o contador dele).
  Nunca `nativo × multiplicador`. Antes de ligar: só faixa estimada ("~180–220") com selo ESTIMADO.
- O nativo continua gravado (`metricas.rs`, histórico, portão "nunca menos FPS" e guarda de perda de FPS usam o nativo), mas fica fora da UI do Frame Boost.
- O valor apresentado nunca passa do Hz do monitor (o `ritmo.rs` já limita). Num monitor de 60 Hz não existe "204". Nesse caso a UI sugere configurar o monitor em vez de mostrar o número.
- Comparações de *otimização* (Lab, "+29%") usam nativo × nativo. O "+%" do Frame Boost é exibido separado e rotulado como Frame Boost.

**Gate de feedback (`feedback.rs`)**
- Contador de uso acumulado (tick de 30 s enquanto a janela está aberta), persistido junto da licença e assinado
  localmente com HMAC da máquina, só para dificultar edição casual.
- Aos 10 min sem `feedback_ok`, emite `feedback:exigido`. A UI mostra o modal com o código OTZ e os botões
  "Abrir canal" (deep link `discord://`/https para o canal) e "Já enviei".
- "Já enviei" → `GET {API}/v1/feedback/{OTZ}` (reqwest, timeout 5 s, mesmo padrão de `convite.rs`). Resposta:
  `{maquina, ok, mensagem_id, verificado_em}.assinatura` conferida com a MESMA chave pública de `licenca.rs`. Recusa se a máquina for diferente ou a data tiver >7 dias.
- Rechecagem a cada abertura do app, no máximo 1×/24 h. `ok:false` (mensagem apagada) → volta a exigir.
  Sem rede → mantém o último estado assinado válido por até 7 dias (não pune quem fica offline).
- Do lado do qrbot (fora do repo — entregar spec + snippet): listener do canal 1543436859529629707. Valida
  ≥30 caracteres + anexo de imagem, liga o autor ao OTZ pela licença que ele comprou, guarda o mensagem_id e assina com `emitir()`.
  No evento `messageDelete`, marca ok:false. Endpoint HTTP no mesmo processo.
- Bloqueio = mesmo mecanismo `inert` do portão atual (`montarPortao`, `main.ts:565`). No backend, `licenca::exigir()`
  passa a conferir também `feedback::liberado()`, então os mesmos 34 pontos recusam alterações. Medição e leitura continuam.
- O endereço da API vem de `.github/convite.json` (já buscado por `convite.rs`), com um campo novo `api`. Assim dá para trocar o host sem publicar versão nova.
- **Pré-requisito externo:** o qrbot precisa saber qual usuário do Discord tem qual código OTZ (ele já recebe o código na compra).
  Sem isso não dá para ligar a mensagem à máquina. Opção de reserva: pedir que o código OTZ apareça no texto do feedback.

**Frontend**: migrar incrementalmente para componentes TS puros (sem framework, para manter o bundle e a RAM ≤150 MB):
`src/ui/tokens.css` (fonte única de tokens), `src/ui/componentes/*.ts` (Card, Metrica, Botao, Modal, Selo,
Grafico, Estado), `src/telas/{inicio,jogos,jogo,boost,lab,ajustes}.ts`, `src/estado.ts` (store simples com
assinaturas). `main.ts` continua servindo as telas antigas atrás da flag até cada tela nova substituir a anterior.

---

## FASE D — UI/UX

**Das screenshots do Beyond (concorrente): o que NÃO fazer**
- O modal de feedback cobre ~40% da tela, sem pausar, e o produto some por trás. Os checkboxes "≥30 caracteres" e
  "print com FPS" parecem clicáveis mas não são. A mensagem amarela "Ainda não encontramos" aparece antes de qualquer tentativa.
- Hero com fotos de personagens/pessoas, 8 ícones sem rótulo, "Modo Limpeza/Liberar Memória/Parar processos" (placebo que o OTIMIZA já recusa), GPUSENSE como gráfico decorativo.

**O que preservar do OTIMIZA**: tema escuro grafite, branco como contraste, fontes Jakarta/Geist/GeistMono
(número em GeistMono como protagonista), selos MEDIDO/ESTIMADO, Ctrl K.

**Sitemap novo (5 itens)**: **Início · Jogos · Boost · Lab · Ajustes** (Histórico vira aba dentro de Jogo e do Lab;
Sistema/Energia/BIOS/Núcleos vão para Boost → "Avançado").
- **Início sem jogo**: estado do PC (CPU/GPU/RAM/monitor Hz), "N oportunidades verificadas" (só itens com achado real) e [Otimizar PC].
- **Início com jogo**: ● jogo em execução, FPS nativo grande, 1% low, GPU/CPU, [Otimizar jogo]. Depois de medir: antes→depois com selo. Card Frame Boost (disponível/estimativa em faixa/ativo).
- **Jogos**: biblioteca (detectados + instalados), card = status, último FPS medido, ganho medido, perfil, Frame Boost.
- **Jogo (detalhe)**: resumo → expandir "Detalhes avançados" (frametime, P95/P99, compat, tweaks aplicados, histórico).
- **Boost**: modos Max FPS/Balanceado/Qualidade/Custom; Frame Boost Auto; Latência; Avançado.
- **Lab**: Testar Performance (fluxo guiado com cena fixa e contagem regressiva), comparação, gráfico de frametime, confiança.
- **Ajustes**: licença, feedback, overlay, flags, Restaurar tudo/Safe Mode.
- **Modal de feedback (redesign próprio)**: painel compacto, não bloqueante até o fim dos 10 min. Depois, folha central com 3 passos numerados, código OTZ copiável e estado real (aguardando / verificando… / não encontrado + motivo / liberado). Nunca exibir erro antes da primeira verificação.

Design system: unificar tokens (`--espaco-*`, `--raio-*`, `--texto-*`, `--cor-{fundo,superficie,borda,texto,sucesso,aviso,erro,boost}`), estados padrão
(loading/skeleton, pronto, ativo, desabilitado, não suportado, erro, parcial, requer reinício, jogo não rodando).
Microinterações: count-up que sempre termina no valor medido (`motion.ts` existente), respeitando reduced-motion.

---

## FASE E — Roadmap (milestones pequenas)

| # | Objetivo | Arquivos | Risco | Teste / critério |
|---|---|---|---|---|
| M0 | Flags + modelo de métricas com Origem | `plataforma/{flags,metricas}.rs`, `preferences.rs`, `lib.rs` | baixo | unit: conversão PresentMon→Metricas; nada visível muda |
| M1 | **Gate de feedback 10 min** (pedido do dono) | `plataforma/feedback.rs`, `licenca.rs` (reuso da verificação), `commands.rs`, `main.ts` (modal novo), `styles.css`, spec do qrbot em `docs/app/FEEDBACK.md` | médio (bloqueia usuário) | unit: contador, assinatura, máquina errada, expiração, offline 7 d; manual: fluxo completo com o endpoint mock |
| M2 | HardwareProfile em cache + Compat engine (anticheat como gate) | `plataforma/{hardware,compat}.rs`, `windows/anticheat.rs` | médio | unit por GPU vendor/monitor 60/144/240/360 |
| M3 | GameAdapter + detecção da sessão (FiveM via wrapper + 12 jogos) | `plataforma/jogos/*`, `sessao.rs` | médio | unit: nomes de exe, anticheat por jogo; overhead do polling <0,2% de núcleo |
| M4 | Histórico por jogo + Lab "Testar Performance" | `plataforma/{historico,lab}.rs` reusando benchmark/baseline/repeticoes | médio | unit: comparação, confiança |
| M5 | Tokens + componentes + Início novo (atrás de `newHome`) | `src/ui/*`, `src/telas/inicio.ts` | médio | `npm run fumaca`, estados de UI |
| M6 | Frame Boost: trait + adapter do gerador próprio + correções §6.3 + Auto | `plataforma/frameboost/*`, `windows/geracao/*` | alto | Presented medido pelo PresentMon igual ao do gerador ±5% |
| M7 | Biblioteca de Jogos + detalhe | `src/telas/jogos.ts, jogo.ts` | baixo | — |
| M8 | AFMF/Anti-Lag via ADLX (PoC) | `frameboost/amd_afmf.rs` | alto | PoC numa Radeon |
| M9 | Overlay opcional (Minimal/Detailed/Off) | janela Tauri transparente sem foco | médio | overhead medido |
| M10 | Migração completa, remoção de mortos/duplicados, rótulo ESTIMADO em `configjogo.rs` | vários | médio | suíte inteira verde |

---

## FASE F — Implementação: TUDO (M0 → M10) = OTIMIZA 3.3
Decisão do dono: entregar o escopo inteiro na versão **3.3.0**, sem cortar nada, mesmo que demore. Ordem de execução
(cada milestone: alteração → `cargo test` + `tsc` + build → corrigir regressões → anotar em `docs/app/PROGRESS.md` → seguir):

1. `docs/planos/MEGA-UPDATE.md` (este plano + fontes) e `docs/app/FEEDBACK.md` (spec + snippet para o qrbot).
2. M0 flags + métricas → M1 gate de feedback → M2 hardware/compat/anticheat no gerador → M3 adapters de jogos → M4 histórico + Lab.
3. **M5–M7 Redesign completo da UI/UX** (o centro visual da 3.3, superar o Beyond):
   - **Shell novo**: barra superior flutuante própria do OTIMIZA (logo à esquerda, 5 destinos **com rótulo** — Início, Jogos,
     Boost, Lab, Ajustes —, chip do jogo em execução à direita com ● ao vivo, Ctrl K). Substitui a barra lateral atual. Nada de 8 ícones mudos.
   - **Sem fotos de personagens/pessoas**: hero = o próprio número. Fundo grafite com gradiente sutil e grão. Acento funcional
     só em 3 cores (sucesso, aviso, Frame Boost). Cada métrica com selo MEDIDO/ESTIMADO discreto.
   - **Início** com 2 estados (sem jogo / jogo detectado). Pontuação do PC só a partir de medições reais. Botão principal único.
   - **Jogos**: grade de cards com capa (ícone do exe/Steam header local quando houver, senão monograma), FPS e ganho medidos.
   - **Jogo (detalhe)**, **Boost** (modos + Frame Boost Auto + Latência + Avançado com as telas antigas de Sistema/Energia/BIOS
     migradas como componentes), **Lab** (fluxo guiado com animação de benchmark e gráfico de frametime em canvas), **Ajustes**.
   - **Frame Boost**: número gigante em GeistMono do FPS na tela (count-up até o valor medido), multiplicador 2×/3×/4× só se compatível, estados reais.
   - **Modal de feedback** próprio, compacto e com estados reais (ver Fase D).
   - Design system em `src/ui/tokens.css` + componentes. Remover os tokens duplicados/auto-referentes e os `.btn`/`.modal` repetidos.
     Todas as telas com os estados de §54. Orçamento: abrir ≤1,5 s, trocar de aba ≤100 ms, ≤150 MB.
   - Antes de codar o shell, gerar **protótipo HTML clicável** das telas principais (artifact) para você aprovar a direção visual. É o único ponto de parada.
3b. **M2b — Paridade AMD (e Intel) com a NVIDIA.** Hoje o OTIMIZA só otimiza driver em NVIDIA (`nvdriver.rs` via NVAPI DRS, `nvml.rs`
   para sensores, `pcie.rs`/`rbar.rs` via `nvidia-smi`). Quem tem Radeon fica sem nada, e o produto é vendido a terceiros com qualquer placa. Plano:
   - Novo `windows/amddriver.rs`: FFI ao **ADLX** oficial (`amdadlx64.dll`, que vem com o driver Adrenalin; `LoadLibrary` em runtime,
     mesmo padrão do `nvdriver.rs`; se não existir, "Atualize o driver AMD"). Ajustes via `IADLX3DSettingsServices(1)`, todos com
     valor anterior lido → `changelog.rs` → desfazer byte a byte:
     | NVIDIA hoje (`nvdriver.rs`) | Equivalente AMD (ADLX, documentado) |
     |---|---|
     | Low Latency Mode | Radeon Anti-Lag (`IADLX3DAntiLag`) |
     | Limitador de FPS | Frame Rate Target Control / Chill (`IADLX3DFrameRateTargetControl`, `IADLX3DChill`) |
     | VSync | Wait for Vertical Refresh (`IADLX3DWaitForVerticalRefresh`) + Enhanced Sync |
     | Qualidade de filtragem de textura | Texture Filtering Quality / Anisotropic (`IADLX3DAnisotropicFiltering`) |
     | Shader cache | Reset Shader Cache (`IADLX3DResetShaderCache`, só sob pedido) |
     | Modo de energia | sem equivalente seguro (tuning de GPU fica FORA: risco) |
     | Smooth Motion | AFMF (`IADLX3DAMDFluidMotionFrames`) → backend do Frame Boost |
     | (G-SYNC) | FreeSync (`IADLXDisplayFreeSync`): só ler e recomendar |
   - Sensores AMD: `IADLXPerformanceMonitoringServices` (temperatura, uso, clock, VRAM, potência) → mesmo modelo de `core/sensores.rs`, que hoje só tem NVML.
   - Intel Arc/iGPU: `windows/inteldriver.rs` via **IGCL** (`ControlLib.dll`): limitador de FPS, low latency, VSync e sensores. Onde a IGCL não expuser, guiar o usuário.
   - `windows/gpudriver.rs`: trait `GpuDriver { vendor, ler(), aplicar(), desfazer(), sensores() }` com 3 implementações. A UI do Boost mostra
     os mesmos cartões para qualquer vendor (texto e nome da tecnologia mudam: "Anti-Lag" vs "Modo de baixa latência"), e o Compat engine decide.
   - `pcie.rs`/`rbar.rs`: substituir o `nvidia-smi` por leitura via SetupAPI/DXGI, que funciona em qualquer vendor.
   - Testes: unidade com um ADLX/IGCL falso (trait), cenários NVIDIA/AMD/Intel/iGPU/híbrido (laptop com iGPU + dGPU).
     A validação real precisa de uma Radeon e de uma Arc: fica marcada como pendente em `PROGRESS.md` até alguém rodar.
4. M6 Frame Boost (trait + gerador próprio corrigido + Auto + regra de exibição) → M8 backend AFMF no Frame Boost (reusa o `amddriver.rs` do M2b) + override XeSS FG via IGCL se houver na doc + Smooth Motion DRS atrás de flag → M9 overlay.
5. M10 limpeza: remover `main.ts` legado depois que todas as telas migrarem, unificar as duplicatas reais (telemetry/telemetria,
   frames/presentmon, benchmark/baseline, historico/medicoes), rótulo ESTIMADO em `configjogo.rs`, comandos mortos, `autoajuste.rs`.
6. Versão 3.3.0 em `tauri.conf.json`, `package.json`, `Cargo.toml`, `.github/release-notes.md`. CI (`ci_coverage.rs`) cobrindo os módulos novos.

7. **M11 — Mira** (pedido do dono, depois do protótipo v4):
   - **Mira na tela**: janela própria transparente, click-through, `WDA_EXCLUDEFROMCAPTURE` opcional — mesmo padrão da janela do gerador
     (`geracao/mod.rs:297`). Estilos ponto/cruz/cruz+ponto/círculo/T, cor, tamanho, espessura, abertura, opacidade, contorno, esconder com botão direito.
     Por jogo. **Bloqueada** em Vanguard/FACEIT via Compat engine. No CS2 vai para a configuração do próprio jogo (`cl_crosshair*` no cfg do usuário), sem sobreposição.
     Aviso fixo no FiveM: servidores de RP podem proibir.
   - **FOV**: só pela configuração do próprio jogo, com o jogo fechado, valor anterior guardado (`changelog.rs`). Cada adapter (`plataforma/jogos/*`)
     declara se tem FOV e onde. FiveM/GTA V: **PRECISA DE POC** para achar onde o FOV de 1ª pessoa fica gravado. Servidor que fixa o FOV vence.
   - **Mouse**: leitura de "aumentar precisão do ponteiro" (já existe `modules/mouse.rs`), taxa medida, conversor de sensibilidade (cm/360; CS2/Valorant/Apex
     com 0,022°/contagem e fator 3,18), treino de tracking local que mede o tempo no alvo.
   - **Fora de escopo, sempre**: qualquer ajuda de mira (aim assist, "aim tracking" automático, macro, recoil). É trapaça e banimento.
8. **M12 — Cores** (vibração/saturação por jogo, "como o painel NVIDIA, no Otimiza"):
   - AMD: ADLX `IADLXDisplayCustomColor` (saturação, matiz, brilho, contraste, temperatura — documentado).
   - Intel: IGCL Standard Color Correction (hue/saturação/contraste/brilho — documentado).
   - NVIDIA: vibração digital via NVAPI `NvAPI_SetDVCLevel(Ex)` (presente no índice do NVAPI, mas via QueryInterface → atrás de flag até validar numa RTX).
   - Sem driver compatível: rampa de gama do Windows (`SetDeviceGammaRamp`) só para brilho/contraste/gama.
   - Predefinições Natural/Vivo/Competitivo/Cinema; "só com o jogo aberto" liga na detecção (`sessao.rs`) e devolve o valor lido antes ao fechar ou se o Otimiza cair (restauração na abertura, como `suspend::retomar_pendentes`).

**Limites honestos (o que não dá para validar daqui)**: o endpoint do qrbot fica fora do repo (entrego spec + snippet e testo
contra um mock). ADLX/AFMF e Smooth Motion só se validam em Radeon/RTX 40+ reais (o código sai com testes de unidade e
fica atrás de flag até alguém testar no hardware). Nenhuma API vai ser inventada: o que não tiver documentação oficial fica como "guiar o usuário".

## Verificação
- `cd pc-optimizer/src-tauri && cargo test` (suíte inteira, mais os testes novos de `feedback`/`metricas`).
- `cd pc-optimizer && npx tsc --noEmit && npm run build`.
- Manual: `npm run tauri dev`, encurtar o prazo por flag de dev (ex.: 30 s), ver o modal, apontar o endpoint
  para um mock local que devolve JSON assinado com `bot/otimiza-licenca.cjs` (ok:true → libera; ok:false → trava; máquina errada → recusa; sem rede → mantém o estado).

- Fontes do ADLX/IGCL: https://gpuopen.com/adlx/ · https://github.com/intel/drivers.gpu.control-library (conferir cada interface na doc antes de escrever a FFI).

## Fontes
- Smooth Motion RTX 40: https://nvidia.com/en-us/geforce/news/nvidia-app-global-dlss-overrides-rtx-40-series-smooth-motion
- NVAPI DRS: https://developer.nvidia.com/downloads/nvapi-driver-settings-programming-guide
- ADLX AFMF: https://gpuopen.com/manuals/adlx/adlx-sdk-references/adlx-interfaces/3d-graphics/iadlx3damdfluidmotionframes/
- ADLX Anti-Lag: https://gpuopen.com/manuals/adlx/adlx-c__anti_lag/ · Anti-Lag 2 SDK: https://gpuopen.com/learn/integrating-amd-radeon-anti-lag-2-sdk-in-your-game/
- XeSS FG: https://github.com/intel/xess/blob/main/doc/xess_fg_developer_guide_english.md · XeSS 3 MFG override: https://www.tomshardware.com/pc-components/gpu-drivers/intel-enables-xess-3-multi-frame-generation-in-latest-drivers-expanding-frame-generation-across-arc-gpus-and-core-ultra-igpus-mfg-can-be-enabled-across-any-title-with-xess-2-support
