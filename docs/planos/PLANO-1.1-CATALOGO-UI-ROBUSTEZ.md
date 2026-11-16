# Plano: a primeira grande atualização (catálogo, UI/UX premium e sistemas robustos)

## Contexto

A 1.0.3 e a 1.0.4 mostraram dois problemas de fundo.

**1. Sistemas que brigam entre si sem ninguém ver.**
- O Jogo liso travou o FiveM de um cliente em 72 FPS. O Frame Boost e o Jogo liso gravam o **mesmo** limite NVIDIA (`limite_nvidia:<exe>` em `WindowsOptimizer::limitar_fps_nvidia`, `modules/windows/mod.rs:1169`) e quem grava por último vence.
- Um plano de energia apagado travou a energia de jogo e a prova do Turbo.
- O diagnóstico só saiu porque o log foi lido à mão.

**2. A tela de Jogos cresceu sem sistema visual.** Hoje, só no destaque do jogo:
- convivem quatro estilos de cartão diferentes;
- o CSS do `CatalogoDoJogo` e do `FivemPainel` foi copiado de um para o outro;
- os raios e tamanhos de fonte são fixos no código, fora dos tokens;
- `.aviso` está definido em três lugares;
- a limpeza de cache pergunta com o `confirm()` do navegador;
- tudo fica empilhado numa coluna estreita.

**Objetivo:** chegar ao lançamento com:
- um catálogo de 5 jogos (FiveM, Valorant, CS2, Fortnite e Apex);
- um painel visivelmente mais bonito e coerente;
- sistemas que não se sabotam.

**Decisões do dono:**
- catálogo de 5 jogos;
- **um único dono do teto de FPS**;
- entrega **em ondas pequenas**: 1.0.5 robustez → 1.0.6 catálogo → 1.1 UI premium.

**Regras de sempre:** nunca menos FPS, tudo se desfaz, medido ≠ estimado, nada muda sem a pessoa pedir. Preferência do dono não vira padrão para todos sem perguntar. Notas de versão escritas como correção de bugs. Build com `CARGO_TARGET_DIR=.cargo-target -j 2`. Nada é publicado sem o ok do dono.

---

## Onda 0: novas ferramentas de trabalho (agentes e skills do projeto)

Essas ferramentas são criadas antes de qualquer código. Elas ficam em `.claude/agents/` e `.claude/skills/`. Já existem 3 agentes: `revisor-otimiza`, `verificador-de-publicacao` e `verificador-do-bot`. As skills são escritas com a skill `writing-skills`.

**Agentes novos** (`.claude/agents/*.md`):

**`analista-de-log`** (sonnet, só leitura)
- **Entrada:** o `otimiza.log` de um cliente.
- **Saída:**
  - uma linha do tempo por versão e sessão, com o hardware;
  - as medições automáticas (FPS e 1% piores);
  - todo `limite NVIDIA`, `jogo liso`, `gerador:`, `guarda`, `turbo:` e `WARN`;
  - um diagnóstico com as linhas citadas.
- **Assinaturas conhecidas que ele já reconhece:**
  - teto na metade do monitor;
  - `powercfg` recusado por plano que sumiu;
  - guarda desligando, com a razão.

**`guardiao-do-fps`** (só leitura)
- Revisa toda mudança em `geracao/`, `turbo.rs`, `jogo_liso.rs`, `base_do_jogo.rs`, `energia_jogo.rs`, `gamemode`/`governador`.
- **Pergunta 1:** existe algum caminho em que esse sistema tira FPS real e continua ligado?
- **Pergunta 2:** dois sistemas escrevem no mesmo lugar?
- **Pergunta 3:** o erro trava um estado para sempre?
- Ele exige um teste puro para cada resposta.

**`designer-otimiza`**
- Gera os prints com `scripts/prints-nova.mjs` (claro e escuro) e lê as imagens.
- Confere cada tela contra `ui/DESIGN.md`, `tokens.css` e o spec `docs/planos/superpowers/specs/2026-11-13-ui-premium-design.md`.
- Aponta valores fixos fora dos tokens, cartões fora do padrão, mais de um botão primário por bloco e textos longos.
- Usa as skills `design-craft` e `frontend-design` como régua.

**`pesquisador-de-jogo`**
- Para um jogo novo, levanta com fonte:
  - onde mora a config;
  - quais chaves existem;
  - o que pesa sem tirar visibilidade;
  - o anticheat;
  - o cache de shader.
- Escreve a ficha em `docs/jogos/<id>.md`. Não edita código.

**Skills novas** (`.claude/skills/<nome>/SKILL.md`):

**`ler-log-do-cliente`**
- O passo a passo que o `analista-de-log` segue: os comandos `grep` e as assinaturas.
- Onde está o log: `%APPDATA%\pc-optimizer\otimiza.log`.

**`publicar-versao`**
- O ritual que já fizemos duas vezes:
  - os 7 arquivos de versão: package.json, as linhas 3 e 9 do package-lock, Cargo.toml, Cargo.lock, tauri.conf.json, `release-titulo.txt` e `release-notes.md`;
  - PR de `ui/premium` para `main`;
  - checagens, merge, tag anotada `vX.Y.Z` e acompanhar a "Publicar versão".
- Chama o `verificador-de-publicacao` antes da tag.

**`jogo-no-catalogo`**
- Como pôr um jogo no catálogo de ponta a ponta:
  - `plataforma/jogos.rs` (`CATALOGO`);
  - o perfil em `configmotor.rs` e os testes dele;
  - o cache em `shaders.rs::locais()`;
  - a ficha;
  - a entrada em `ui/src/lib/catalogo.ts` (criado na Onda 2);
  - a regra do anticheat (`anticheat.rs`).

**`tela-otimiza`**
- As regras visuais e de interação:
  - os tokens;
  - o componente de cartão;
  - estados de carregando, vazio e erro;
  - demonstração fora do Tauri;
  - o ciclo de prints.

**Mudança no app nesta onda:** um botão **"Salvar log para o suporte"** em Ajustes.
- Copia `otimiza.log` e `otimiza.log.1` para Downloads, usando `Logger::caminho()` em `utils/logger.rs`.
- Fica ao lado do `relatorio_de_suporte` que já existe (`modules/windows/suporte.rs`).
- Hoje o cliente precisa achar o arquivo pelo Win+R.

---

## Onda 1 (1.0.5): robustez

### 1.1 Um dono só para o teto de FPS
- **Módulo novo:** `plataforma/teto_do_jogo.rs`. É a **única** porta para gravar ou tirar o limite NVIDIA de um jogo.
- Ele guarda **quem** pôs o teto (`Origem::FrameBoost | JogoLiso | Pessoa`) num arquivo ao lado do de `base_do_jogo`.
- **Regras:**
  - Com o Frame Boost ligado para o jogo, vale o teto dele (`base_do_jogo::teto_da_base`). O Jogo liso só **mostra** o que faria e diz "o Frame Boost está cuidando do ritmo deste jogo".
  - Sem o Frame Boost, vale o do Jogo liso (`jogo_liso::planejar`, já com o limite de 15% da 1.0.4).
  - Desligar o Frame Boost devolve o teto que existia antes dele, e não "sem limite".
  - O teto da Pessoa nunca é sobrescrito sem aviso.
- **Chamadores a migrar** (todos passam a usar `teto_do_jogo`):
  - `jogo_liso::aplicar` e `gravar_depois_da_partida`;
  - `base_do_jogo::travar`;
  - `frameboost::travar_base`, chamado em `commands/frame_boost.rs:180, 297`, `commands/turbo.rs:101, 169` e `lib.rs:1393`.
- **Na tela** (Jogo liso, Frame Boost e catálogo do jogo): "Teto valendo: 72 FPS, posto pelo Frame Boost (2× → 144 na tela)", com **Soltar**.
- **Testes puros** na decisão `decidir(origem_atual, pedido, fb_ligado, hz, fps_tipico) -> Acao`, incluindo o caso do cliente.

### 1.2 Frame Boost
- **Erro conhecido:** com o alvo definido, `travar_base` passa o **alvo** como `hz` para `teto_da_base`. A fração, o piso de hz/4 e o texto "fecham os X Hz" usam o número errado. Corrigir e cobrir com teste.
- **Base com o FPS de costume:** usar `jogo_liso::fps_de_costume` para que uma queda medida não vire uma base baixa.
- **Mais claro quando a placa não aguenta:**
  - O log de cada comparação da guarda passa a trazer a razão. Hoje só a decisão é logada.
  - O `DesligadoPorPerdaDeFps` mostra "o Frame Boost tirava X% do FPS real; nesta placa, use o Jogo liso".
  - O `fb_por_jogo` lembra disso para não religar sozinho.
- **Guarda do modo leve:** manter 0,75 até ter dados. O `analista-de-log` mede nos logs dos testadores. Se a perda real média ficar acima de 10%, sobe para 0,85, com a decisão do dono.
- **Teste do laço principal:** extrair as decisões do `sessao()` (~700 linhas sem teste) para funções puras: reação à guarda, troca de fonte de contagem e tela cheia exclusiva. Testar com fontes sintéticas, reaproveitando o `simular`/`simular_com` de `guarda.rs`.

### 1.3 Turbo e energia
- **Prova do Turbo:** quando ela aborta, o motivo vai para a tela com a próxima ação, e não só para o log.
- **Varredura de registros presos:** procurar outros que podem ficar presos como o do plano apagado. Conferir os arquivos de devolução de `energia_jogo`, `base_do_jogo`, `turbo_energia` e `turbo_base`, e a marca da janela.
- **Regra para cada registro:** se o alvo não existe mais, descartar com um aviso no log e nunca falhar para sempre.
- Revisado pelo `guardiao-do-fps`.

---

## Onda 2 (1.0.6): catálogo de 5 jogos

- **Fonte única:** `ui/src/lib/catalogo.ts` diz, para cada jogo (fivem, valorant, cs2, fortnite, apex), como reconhecê-lo (o regex e o appid que hoje estão em `Jogos.svelte`), o perfil do `configmotor`, os caches (o mapa `CACHES` de `CatalogoDoJogo`), o texto do anticheat e os painéis extras (FiveM: `FivemPainel`; CS2: `Cs2Integracao`).
- **Fortnite e Apex:** já têm ficha e perfil em `configmotor.rs`. Falta:
  - o regex no catálogo;
  - o cache de shader;
  - o anticheat (EAC/BattlEye permitem sobreposição? Confirmar com o `pesquisador-de-jogo` e ajustar `anticheat.rs`).
- **FiveM igual aos outros:**
  - a ficha `docs/jogos/fivem.md`;
  - o bloco de configuração do jogo vindo do perfil GTA/FiveM, que hoje mora em `PerfilDoJogoNoTurbo`;
  - a limpeza de cache pelo mesmo componente.
- **Fim da duplicação:** a "Configuração do jogo" de `NoJogo.svelte` (no Turbo) passa a apontar para o catálogo.
- **Testes de catálogo** em `ui/testes/catalogo.test.ts`: cada jogo tem um perfil existente nos tipos, os caches conhecidos e um texto de anticheat.

---

## Onda 3 (1.1): UI e UX premium da tela de Jogos e do painel

Feita com as skills `design-craft` e `frontend-design`, conferida pelo `designer-otimiza`.

- **Base do sistema visual** (`tokens.css`):
  - `.cartao`, `.aviso` e `.recado-texto` globais;
  - fim dos valores fixos (raios 10/14/16 → `--raio-*`; 12/13/15px → `--t-*`);
  - um componente `ui/src/componentes/base/Cartao.svelte` (cabeçalho, corpo, ações e "detalhes recolhidos", como pede o spec premium);
  - um `Confirmar.svelte` no lugar do `confirm()`.
- **Destaque do jogo redesenhado:**
  - a capa e os números ficam em cima;
  - abaixo, **sub-abas** dentro do jogo:
    - **Visão geral**: FPS de costume, último 1% pior, teto valendo, estado do Frame Boost e do Turbo;
    - **Otimizar**: configuração do jogo, cache e Jogo liso;
    - **Ferramentas**: mira, crashes do FiveM, integração do CS2;
    - **Histórico**: linha do tempo.
  - Nada fica mais empilhado numa coluna estreita.
- **Estados:** carregando (`Esqueleto.svelte` já existe), vazio com a próxima ação e erro com o que fazer. Cada número leva o selo medido ou estimado.
- **Telas vizinhas:** depois da tela de Jogos, aplicar o mesmo `Cartao` no Turbo, na Mira e em Ajustes. O `designer-otimiza` passa por todas as áreas nos prints claro e escuro.
- **Desempenho:** com o jogo aberto, a interface congela os efeitos (`.jogo-aberto`). Conferir que as sub-abas novas respeitam isso.

---

## Arquivos principais

**Onda 0**
- Agentes: `.claude/agents/{analista-de-log,guardiao-do-fps,designer-otimiza,pesquisador-de-jogo}.md`
- Skills: `.claude/skills/{ler-log-do-cliente,publicar-versao,jogo-no-catalogo,tela-otimiza}/SKILL.md`

**Onda 1 (Rust)**
- Novo: `plataforma/teto_do_jogo.rs`
- Alterados: `plataforma/jogo_liso.rs`, `base_do_jogo.rs`, `frameboost.rs`, `fb_por_jogo.rs`, `modules/windows/geracao/{mod.rs,guarda.rs}`, `turbo.rs`, `energia_jogo.rs`, `utils/logger.rs`
- Comandos: `commands/{frame_boost,turbo,mod}.rs`, `lib.rs`, `contrato.rs` (e `tipos.ts` regenerado)

**Ondas 2 e 3 (UI)**
- Novos: `ui/src/lib/catalogo.ts`, `componentes/base/{Cartao,Confirmar}.svelte`
- Alterados: `Jogos.svelte`, `CatalogoDoJogo.svelte`, `FivemPainel.svelte`, `NoJogo.svelte`, `Turbo.svelte`, `tokens.css`, `Ajustes.svelte`

**Fichas:** `docs/jogos/{fivem,fortnite,apex}.md`

---

## Verificação (em toda onda)

1. **Rust:** `cargo test --lib -j 2` (hoje 1966 testes), mais os testes puros novos de cada item. `OTIMIZA_GERAR_TIPOS=1 cargo test --lib contrato` quando um comando mudar.
2. **UI:** `npx svelte-check --workspace ui` com 0 erros, `npm run testes:ui` e `npm run fumaca:nova`, que clica em todas as áreas.
3. **Visual:** `scripts/prints-nova.mjs` em claro e escuro, lido pelo `designer-otimiza` antes do PR.
4. **Revisão:** `guardiao-do-fps` em toda mudança da Onda 1 e `revisor-otimiza` em todo commit.
5. **No jogo, com testadores reais:**
   - cada onda vai para 2 ou 3 clientes, incluindo o da GTX 1650, antes da próxima;
   - eles mandam o log pelo botão novo;
   - o `analista-de-log` confirma pelo log que não há teto inesperado, que o Frame Boost não tira FPS real e que nenhum registro ficou preso.
6. **Publicação:** skill `publicar-versao` com o `verificador-de-publicacao`. Só com o ok do dono.
