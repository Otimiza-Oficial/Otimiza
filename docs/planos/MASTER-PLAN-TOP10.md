# MASTER PLAN — TOP 10: fichas, plano técnico e tarefas

Versão analisada: main e61fbe2 · 26/09/2026 · complemento de `MASTER-PLAN.md`

Cada item tem a ficha completa, o plano técnico com os arquivos que já existem e as tarefas. Ciclo de
cada tarefa: DESIGN → IMPLEMENT → TEST → BENCHMARK (quando mexe em desempenho) → VERIFY (`revisor-otimiza`)
→ DOCUMENT → próxima. Nada é publicado sem ordem do dono; tudo vai em commits locais até a versão fechar.

## Ordem de execução

```
0 Lançar a 3.2 (ordem do dono)
│
├─► 1 Núcleo de Sessão e Evidência ─┬─► 2 Doctor + FiveM Lab ─► 3 Prescrição + memória ─► 9 Relatório compartilhável
│                                   └─► 7 Relatório de sessão
├─► 4 Identidade + "O que mudou?" ──┬─► 10 Primeira execução
│                                   └─► 5 Guardião de deriva
├─► 6 Restaurar meu PC + desinstalação   (independente; pode andar em paralelo)
└─► 8 Telemetria opt-in + regras assinadas + beta   (NEXT; depende de decisões do dono)
```

## 0. Lançar a 3.2

Não é iniciativa nova: é o pré-requisito. Falta o teste do dono no PC dele com FiveM (build em
`target-teste\release\pc-optimizer.exe`): interface, guarda do gerador, painel de crashes, "Copiar
diagnóstico", abrir e fechar para o log mostrar onde está o tempo de abertura. Depois: verificador de
publicação, número da versão em 4 lugares, `NOTAS-3.2.md` → `.github/release-notes.md`, tag, site e aviso
do bot, nessa ordem e só com ordem.

## 1. Núcleo de Sessão e Evidência

- **Name:** Núcleo de Sessão e Evidência (`SessaoDeJogo`).
- **Customer Problem:** números diferentes em telas diferentes para a mesma partida; o cliente não sabe em
  qual acreditar.
- **Business Value:** uma resposta só, com fonte; base de tudo que o serviço promete.
- **Technical Value:** hoje `MedicaoAutomatica` (`medicoes.rs`), `provaalternada::Rodada`, `portao` e a
  prova antiga guardam campos parecidos de jeitos diferentes. Um contrato só elimina a divergência.
- **Architecture:** evoluir `MedicaoAutomatica` em vez de criar um tipo paralelo: ela já tem FPS, 1%,
  ritmo, placa, CPU/GPU, trancos com disco, geração e resumo do PresentMon. Faltam: `id` estável,
  `fonte` (PresentMon ou reserva, enum), `configuracao_do_jogo` (hash da config lida por `configjogo.rs`),
  `quadros` (contagem, para a regra de amostra), `eventos` (tranco, calor, processo, com instante relativo)
  e `versao_do_formato`. Uma função pura `comparavel(a, b) -> Result<(), Motivo>` concentra as regras que
  hoje estão em `portao`, `regressao` e `provaalternada` (mesma fonte, mesmo gerador, mesma config).
- **Feito (branch `servico-t1`):** T1.1 e T1.2. A revisão achou que filtrar a série automática pela
  configuração do jogo cegaria o portão: o item vigiado é o próprio perfil gráfico, que muda o arquivo. Por
  isso a série não passa por `evidencia`; só as provas passam. Ficou aberta a T1.2b.
  T1.3 feita (eventos só da janela medida, dizendo o que foi lido). A T1.4 virou o guarda
  `ninguem_compara_contexto_fora_daqui`, dentro da T1.2. O laboratório da T1.2b está pronto e espera uma partida.
- **Dependencies:** nenhuma.
- **Risks:** migração quebrar histórico antigo. Mitigação: todos os campos novos com `serde(default)` e
  teste de leitura de arquivo da 3.1 e da 3.2.
- **Security Impact:** nenhum.
- **Anti-Cheat Impact:** nenhum; a coleta continua pelo PresentMon (ETW), sem abrir o processo.
- **Performance Overhead:** nenhum novo durante o jogo: os campos vêm de leituras que já existem; eventos
  guardados como contagem e instante, não como série.
- **How We Test:** testes de migração com JSON real de versões antigas; testes de `comparavel` com os casos
  que hoje estão espalhados; guarda de arquitetura: prova, portão e regressão só comparam por `comparavel`.
- **Success Criteria:** os três módulos usam `comparavel`; nenhum teste antigo muda de resultado; o
  arquivo de medições da 3.1 abre sem perda.
- **Failure Criteria:** qualquer comparação diferente da de hoje sem motivo escrito; perda de histórico.
- **Rollback:** reverter o commit; o formato novo continua legível pela versão anterior porque só
  acrescenta campos.
- **Priority:** NOW, primeiro.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T1.1 | Campos novos na medição | Dar identidade e fonte a cada partida | `medicoes.rs` | — | `id`, `fonte`, `quadros`, `configuracao_do_jogo`, `versao_do_formato`, todos `serde(default)` | Ler JSON de 3.1 e 3.2 gravados | Lê tudo, nada zerado | Reverter commit | Baixo | NOW |
| T1.2 | `comparavel(a, b)` | Uma regra de comparabilidade | novo `modules/evidencia.rs` | T1.1 | Função pura com `Motivo` enum; migrar os casos de `portao`, `regressao`, `provaalternada` | Um teste por motivo; os testes antigos dos três módulos seguem verdes | Três módulos chamam a função | Reverter | Médio | NOW |
| T1.2b | Os dois medidores na mesma partida | Decidir se a série automática pode misturar PresentMon e canal antigo | script de laboratório, `docs/` | T1.2 | Medir a mesma cena pelos dois canais ao mesmo tempo, várias vezes, e registrar o viés | — | Viés medido e decisão escrita | — | Baixo | NOW |
| T1.3 | Eventos da partida | Instante do tranco, calor e processo | `medicoes.rs`, `core/travadas.rs` | T1.1 | Lista curta (máx. 50) de eventos com instante relativo | Fixture de sessão com trancos conhecidos | Eventos batem com a fixture | Reverter | Baixo | NOW |
| T1.4 | Guarda de arquitetura | Ninguém compara fora do contrato | `tests/` | T1.2 | Teste que procura comparação de FPS entre medições fora de `evidencia.rs` | O próprio teste falha se alguém comparar na mão | Guarda verde | Remover o teste | Baixo | NOW |

## 2. Performance Doctor + FiveM Lab

A ficha está na seção F do `MASTER-PLAN.md`. Plano técnico:

- **Entrada:** uma `SessaoDeJogo` de 60–90 s (o cliente joga normalmente; a tela mostra a contagem).
- **Regras puras**, cada uma em função testável, devolvendo `Option<Causa>`:
  - `gargalo`: CPUBusy × GPUBusy do PresentMon (`presentmon::Resumo`), já existe em `gargalo.rs`/`bottleneck.rs`.
  - `teto`: limite de FPS, V-Sync, Hz do monitor (`tetos.rs`, `vrr.rs`).
  - `energia_e_calor`: PDH `Performance Limit Flags` e NVML (`core/telemetria.rs`, `nvml.rs`, `thermal.rs`).
  - `memoria`: VRAM e pressão de RAM (`vram.rs`, `pressao.rs`).
  - `trancos`: gravidade e coincidência com disco (`core/travadas.rs`, `streaming.rs`).
  - `apresentacao`: cópia/composição em vez de flip (`presentmon.rs`).
  - `crashes` (FiveM): `crashes.rs` com as pistas MesmoDefeito, PoolCheio, MemoriaNoLimite, MesmoVeiculo, JaAcontecia.
- **Fusão:** ordena por evidência (tamanho do efeito medido) e corta em 3. Cada `Causa` tem: `tipo` (enum),
  `evidencia` (números da sessão), `confianca` (Alta/Média/Hipótese), `ajuda` e `nao_ajuda` (ids de
  estratégia).
- **Interface:** tela nova em Início ("Por que meu jogo roda assim?"); decide só pelo enum `tipo` e pela
  `confianca`, nunca por texto.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T2.1 | Fixtures de sessões rotuladas | Base de teste do Doctor | `tests/fixtures/doctor/` | T1.1 | 6–10 sessões reais do PC do dono (GPU, CPU, teto, calor, trancos de disco, FiveM com crash), rotuladas à mão | — | Fixtures versionadas | Apagar pasta | Baixo | NOW |
| T2.2 | Regras de causa | Uma função por causa | novo `modules/doctor.rs` | T2.1 | Reusar os módulos existentes; nada de leitura nova | Positivo e negativo por regra | Todas as fixtures corretas | Reverter | Médio | NOW |
| T2.3 | Fusão em 3 causas | Resposta curta | `doctor.rs` | T2.2 | Ordenar por efeito; `Hipotese` nunca entra na prescrição | Fixture com 5 sinais corta em 3 | Corte estável | Reverter | Baixo | NOW |
| T2.4 | FiveM Lab | Crash e servidor na mesma resposta | `doctor.rs`, `crashes.rs` | T2.2 | Causa "servidor/asset" quando a pista é MesmoVeiculo ou PoolCheio | Fixture de crash do PC do dono | Diz "servidor" com a evidência | Reverter | Baixo | NOW |
| T2.5 | Tela do Doctor | O cliente vê a resposta | `index.html`, `main.ts` | T2.3 | Comando `doctor_da_sessao`; tela lê enum | `npm run fumaca` + caso mockado | Abre sem resposta do Windows | Esconder a tela | Baixo | NOW |

## 3. Prescrição validada + Memória de Efetividade

- **Name:** Prescrição validada com memória por máquina.
- **Customer Problem:** "apliquei e não sei se ajudou" e "o programa sugeriu de novo o que já tinha piorado".
- **Business Value:** o diferencial central; alimenta o CPCR.
- **Technical Value:** generaliza a prova alternada, hoje só do plano de energia, para qualquer estratégia
  que dê para ligar e desligar no meio da partida; o que não dá vai para o portão entre sessões.
- **Architecture:** tipo `Estrategia { id, trata: TipoDeCausa, acao_anticheat: Acao, precondicoes,
  prova: ComoProvar::{AoVivo, EntreSessoes}, aplicar, desfazer }`. `ComoProvar::AoVivo` reusa o motor de
  `provaalternada.rs` (ABBABAAB, aquecimento, mesma fonte/config/gerador). Memória em
  `%APPDATA%\pc-optimizer\efetividade.json`: `(estrategia, jogo, driver, build) → {desfecho, ic, quando}`.
- **Dependencies:** T1.2, TOP 2.
- **Risks:** a prova alternada troca estado no meio da partida; estratégia que precisa reiniciar o jogo
  não serve para AoVivo. Mitigação: o tipo obriga declarar; o que não é trocável a quente só usa o portão.
- **Security Impact:** as ações continuam as do catálogo atual, pelo changelog.
- **Anti-Cheat Impact:** `acao_anticheat` é campo obrigatório; `anticheat::permite` roda antes de aplicar
  e antes de cada troca da prova.
- **Performance Overhead:** o da prova alternada (6 min com rodadas de 45 s), só quando o cliente pede.
- **How We Test:** testes puros da decisão KEEP/NEUTRO/ROLLBACK com amostras sintéticas; teste da memória
  (piorou → não oferece; driver mudou → volta a oferecer com motivo); prova real no PC do dono.
- **Success Criteria:** nenhuma estratégia com ROLLBACK é oferecida de novo no mesmo contexto; toda
  estratégia KEEP tem prova válida gravada.
- **Failure Criteria:** qualquer caminho que deixa uma estratégia aplicada depois de ROLLBACK; prova que
  mistura fonte ou gerador.
- **Rollback:** a memória é um arquivo; apagar volta ao comportamento de hoje. Cada estratégia desfaz pelo changelog.
- **Priority:** NOW.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T3.1 | Tipo `Estrategia` | Contrato declarativo | novo `modules/estrategia.rs` | T1.2 | Campos da ficha; porteiro de anticheat no construtor | Guarda: toda estratégia declara ação | Compila só com os campos | Reverter | Médio | NOW |
| T3.2 | Prova alternada genérica | Provar além da energia | `provaalternada.rs` | T3.1 | Extrair `executar` para receber `&dyn Trocavel` | Testes atuais seguem verdes + estratégia falsa | Energia continua igual | Reverter | Médio | NOW |
| T3.3 | Memória de efetividade | Não repetir o que piorou | novo `modules/efetividade.rs` | T3.1 | JSON versionado; `deve_oferecer(estrategia, contexto) -> Oferta` | Casos: piorou, neutro, contexto mudou, arquivo corrompido | Todos os casos | Apagar o arquivo | Baixo | NOW |
| T3.4 | Prescrição a partir do Doctor | Ação ligada à causa | `doctor.rs`, `estrategia.rs` | T2.3, T3.3 | `prescrever(causas, memoria) -> Vec<Receita>` | Fixture de CPU gera só estratégias de CPU | Nenhuma receita sem causa | Reverter | Baixo | NOW |
| T3.5 | Tela da prescrição | Aplicar, provar, decidir | `index.html`, `main.ts` | T3.4 | Passo 2–5 do fluxo Otimizar lê as receitas | Fumaça + mock | Abre e mostra "como será provado" | Voltar ao fluxo da 3.2 | Médio | NOW |

## 4. Identidade da Máquina + "O que mudou?"

- **Name:** Identidade da Máquina e linha do tempo.
- **Customer Problem:** "estava bom semana passada".
- **Business Value:** responde o ticket mais comum sem o atendente adivinhar.
- **Technical Value:** junta `baseline::Identidade`, `deriva::Ambiente`, `historico.rs` e o changelog, que
  hoje guardam pedaços da mesma coisa.
- **Architecture:** `Identidade` ganha driver de vídeo, versão da BIOS (leitura), monitores e Hz, notebook,
  versão do Otimiza. Linha do tempo = medições (TOP 1) + eventos de identidade (quando algo mudou) +
  changelog. A resposta continua a de `historico.rs`: suspeitos, não causas.
- **Feito (branch `servico-t1`):** a identidade saiu das medições automáticas que o cliente já tem, e não do
  benchmark: cada partida grava placa, driver, Windows, BIOS (só lida) e versão do Otimiza. A área Histórico
  ganhou "O que mudou no seu jogo", com partidas, trocas e mudanças do Otimiza, e um veredito de três estados
  (caiu / não caiu / poucas partidas). "Poucas partidas" nunca aparece como verde (achado da revisão).
- **Dependencies:** T1.1.
- **Risks:** identidade com dado pessoal. Mitigação: nada de nome de usuário, serial ou SID; teste que
  procura esses padrões.
- **Security Impact:** nenhum; só leitura.
- **Anti-Cheat Impact:** nenhum.
- **Performance Overhead:** leitura na abertura e depois da partida, fora do jogo.
- **How We Test:** testes puros de `diferencas`; teste de anonimização; linha do tempo com fixture.
- **Success Criteria:** trocar driver na fixture aparece como suspeito com a data certa.
- **Failure Criteria:** suspeito dito como causa; dado pessoal no arquivo.
- **Rollback:** campos novos opcionais; a tela antiga de Histórico continua.
- **Priority:** NOW.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T4.1 | Identidade completa | Um retrato da máquina | `baseline.rs`, `deriva.rs` | T1.1 | Unificar `Identidade` e `Ambiente`; campos novos opcionais | Leitura de arquivos antigos; teste sem dado pessoal | Um tipo só | Reverter | Baixo | NOW |
| T4.2 | Eventos de identidade | Saber quando mudou | `historico.rs` | T4.1 | Gravar evento quando `diferencas` não é vazio | Fixture com troca de driver | Evento com data | Reverter | Baixo | NOW |
| T4.3 | Tela "O que mudou?" | Resposta para o cliente | área Histórico | T4.2 | Linha do tempo com medições, eventos e mudanças | Fumaça | Abre sem resposta | Voltar à tela da 3.2 | Baixo | NOW |

## 5. Guardião de Deriva

- **Name:** Guardião de Deriva.
- **Customer Problem:** atualização do Windows desfaz ajustes em silêncio, e o jogo piora sem motivo aparente.
- **Business Value:** o "cuidar continuamente" que justifica o serviço.
- **Technical Value:** generaliza `planoenergia::vistoriar` (hoje só plano de energia) para toda mudança
  registrada no changelog.
- **Architecture:** para cada `ChangeRecord` aplicado, uma leitura "o valor atual ainda é o que o Otimiza
  gravou?". Resultado: Intacto, MudouPorFora (com o valor encontrado), NaoLeu. Roda na abertura e depois de
  mudança de build detectada pelo TOP 4. **Nunca restaura sozinho:** mostra e pergunta.
- **Feito (branch `servico-t1`):** a vistoria relê cada item do catálogo aplicado e avisa na área Otimizar o
  que não está mais como o Otimiza deixou. "Refazer" é desfazer + aplicar pelo caminho normal: escrever por
  fora do histórico deixaria mudanças sem volta (achado da revisão). Sem administrador, o que não se lê fica
  "não conferido"; o plano OTIMIZA fica fora (o próprio Otimiza o troca e ele tem vistoria própria).
- **Dependencies:** T4.1.
- **Risks:** restaurar algo que o cliente mudou de propósito. Mitigação: só pergunta; mostra o valor atual e o nosso.
- **Security Impact:** leitura; a restauração usa o mesmo caminho de aplicação.
- **Anti-Cheat Impact:** nenhum na leitura; a restauração passa pelo porteiro.
- **Performance Overhead:** leitura de registro/serviços na abertura; medir e manter < 200 ms fora do jogo.
- **How We Test:** teste puro da classificação por tipo de `ChangeRecord`; teste em máquina de CI com
  chave alterada por fora.
- **Success Criteria:** chave alterada por fora aparece como MudouPorFora; nada é escrito sem clique.
- **Failure Criteria:** qualquer escrita automática; falso MudouPorFora em chave intacta.
- **Rollback:** desligar a vistoria (preferência); nada foi escrito.
- **Priority:** NOW.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T5.1 | Vistoria por `ChangeRecord` | Saber o que mudou por fora | `changelog.rs`, novo `modules/guardiao.rs` | T4.1 | Uma leitura por variante do enum; puro na classificação | Um teste por variante | Todas cobertas | Reverter | Baixo | NOW |
| T5.2 | Aviso e restauração com clique | O cliente decide | `main.ts` | T5.1 | Aviso em Início; "restaurar" reaplica pela transação | Fumaça + mock | Nada sem clique | Esconder aviso | Baixo | NOW |

## 6. Restaurar meu PC + Desinstalação limpa

- **Name:** Restaurar meu PC.
- **Customer Problem:** "quero voltar como era" e "desinstalei e ficou alguma coisa".
- **Business Value:** confiança para comprar; reduz reembolso por medo.
- **Technical Value:** `revert_all` (`windows/mod.rs:1281`) já existe; falta o relatório item a item e o
  gancho no desinstalador.
- **Architecture:** botão em Sistema que chama `revert_all_optimizations` e mostra por item: voltou, já
  estava, não voltou (com motivo). O instalador é `currentUser` (`tauri.conf.json`); o gancho
  `NSIS_HOOK_PREUNINSTALL` (`installerHooks` do Tauri 2) pergunta "Restaurar o que o Otimiza mudou?" e
  roda `pc-optimizer.exe --restaurar-tudo`, que pede elevação pelo UAC quando precisa.
- **Dependencies:** nenhuma.
- **Risks:** o desinstalador rodar sem o exe presente ou sem elevação. Mitigação: se falhar, o
  desinstalador avisa e mantém o changelog em `%APPDATA%` para uma reinstalação restaurar.
- **Security Impact:** argumento de linha de comando novo; aceitar só esse, sem parâmetro.
- **Anti-Cheat Impact:** nenhum.
- **Performance Overhead:** nenhum.
- **How We Test:** testes atuais de `revert_all`; teste do argumento; desinstalação real numa VM com
  mudanças aplicadas, conferindo registro, serviços e planos antes/depois.
- **Success Criteria:** na VM, depois de desinstalar com "sim", nenhuma mudança do Otimiza sobra.
- **Failure Criteria:** qualquer mudança não relatada; desinstalador travado.
- **Rollback:** remover o gancho do NSIS; o botão é independente.
- **Feito (branch `servico-t1`):** T6.1–T6.3. O "Desfazer tudo" passou a cobrir o motor de energia, as
  regras de núcleos e os modos automáticos, não só o histórico; recusa com mudança pela metade ou prova rodando;
  histórico ou backup ilegível viram falha com motivo. O desinstalador pergunta, fecha o Otimiza, pede
  administrador, lê o resultado e só segue sem restaurar se o cliente confirmar. Na atualização (`/UPDATE`) não
  faz nada. Falta: o teste numa VM limpa, e conferir a ordem modo jogo × motor quando os dois agiram no mesmo jogo.
- **Priority:** NOW.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T6.1 | Relatório de restauração | Item a item | `commands.rs`, `main.ts` | — | Resultado por item com enum Voltou/JaEstava/NaoVoltou | Testes de `revert_all` + fumaça | Tela mostra os três casos | Reverter | Baixo | NOW |
| T6.2 | `--restaurar-tudo` | Chamada pelo desinstalador | `main.rs` | T6.1 | Argumento único; roda `revert_all` e sai com código | Teste do parser | Código 0 quando tudo voltou | Reverter | Médio | NOW |
| T6.3 | Gancho no NSIS | Oferecer ao desinstalar | `tauri.conf.json`, `windows/hooks.nsh` | T6.2 | `NSIS_HOOK_PREUNINSTALL` com pergunta | VM limpa: instalar, aplicar, desinstalar | Nada sobra | Remover gancho | Médio | NOW |

## 7. Relatório de Sessão

- **Name:** Relatório de Sessão.
- **Customer Problem:** o valor contínuo não aparece; o cliente esquece que o Otimiza existe até dar problema.
- **Business Value:** retenção e boca a boca com número honesto.
- **Technical Value:** usa a `SessaoDeJogo` que a medição automática já produz.
- **Architecture:** ao fechar o jogo, um resumo curto na janela do Otimiza (não notificação que rouba
  foco): FPS do jogo, 1% piores, trancos com instante, gargalo, calor, diferença para a mediana das
  últimas partidas comparáveis (por `comparavel`).
- **Feito (branch `servico-t1`):** o cartão "Sua última partida" no Início, montado depois, das janelas já
  medidas: nada novo roda no jogo, então o orçamento de custo não se aplica a ele. A revisão pegou três frases
  desonestas (freio contínuo como "não freou", 1% de janela curta, disco contado sobre a lista cortada), corrigidas.
- **Dependencies:** TOP 1; **orçamento medido** (seção H do plano) antes de ligar por padrão.
- **Risks:** comparar partida em cena diferente. Mitigação: diz "partidas parecidas" só quando comparável,
  e mostra a dispersão.
- **Security Impact:** nenhum.
- **Anti-Cheat Impact:** nenhum.
- **Performance Overhead:** medir; meta da seção H.
- **How We Test:** fixture de sessões; medição do custo do Otimiza com o jogo aberto (PresentMon no
  próprio processo + contador de CPU).
- **Success Criteria:** custo dentro da meta; resumo bate com a fixture.
- **Failure Criteria:** custo acima da meta → não sai.
- **Rollback:** preferência para desligar.
- **Priority:** NEXT.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T7.1 | Medir o custo do Otimiza em jogo | Prova do "zero distração" | `docs/`, script | — | Protocolo e resultado no PC do dono | — | Número publicado no doc | — | Baixo | NEXT |
| T7.2 | Resumo pós-partida | Mostrar a partida | `medicoes.rs`, `main.ts` | T1.3, T7.1 | Resumo puro a partir da sessão | Fixture | Bate com a fixture | Desligar | Baixo | NEXT |

## 8. Telemetria opt-in + regras assinadas + canal beta

- **Name:** Telemetria opt-in, kill switch e beta.
- **Customer Problem:** indireto: problema que afeta muitos só é visto quando vira reclamação.
- **Business Value:** CPCR de verdade; resposta a SEV-0 em minutos.
- **Technical Value:** regras assinadas reusam a assinatura Ed25519 da licença.
- **Architecture:** (a) **regras:** arquivo JSON assinado no GitHub Releases (sem servidor novo), lido na
  abertura, só com "desligar estratégia X" e "aviso Y"; versão monotônica. (b) **telemetria:** desligada
  por padrão; quando ligada, envia só o esquema do `MASTER-PLAN.md` seção K; o cliente vê o que vai.
  (c) **beta:** o aviso de versão lê um canal diferente para quem marcou.
- **Dependencies:** decisão do dono sobre o endpoint (bot na Square Cloud ou serviço novo), política de
  privacidade publicada, revisão de segurança.
- **Risks:** vazamento de dado; regra maliciosa. Mitigação: esquema mínimo; assinatura; regra só desliga.
- **Security Impact:** alto: primeira comunicação de saída além do aviso de versão. Revisão obrigatória.
- **Anti-Cheat Impact:** nenhum.
- **Performance Overhead:** envio fora da partida.
- **How We Test:** regra com assinatura inválida é ignorada; versão velha é ignorada; telemetria desligada
  não abre conexão (teste que falha se abrir).
- **Success Criteria:** SEV-0 simulado desliga a estratégia em todas as máquinas de teste na próxima abertura.
- **Failure Criteria:** qualquer envio com a opção desligada.
- **Rollback:** a versão seguinte para de ler o arquivo; nada no PC depende dele.
- **Priority:** NEXT (as regras podem vir antes da telemetria).

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T8.1 | Regras assinadas | Kill switch | novo `modules/regras.rs`, `licenca.rs` | T3.1 | Ler, verificar, aplicar "desligar id" | Assinatura inválida, versão velha, id desconhecido | Só aplica regra válida | Parar de ler | Médio | NEXT |
| T8.2 | Canal beta | Voluntários antes do estável | `atualizacao.rs` | — | Preferência + release marcada como pre-release | Teste do parser do canal | Beta só para quem marcou | Desmarcar | Baixo | NEXT |
| T8.3 | Telemetria opt-in | CPCR | novo `modules/telemetria_optin.rs` | decisão do dono | Esquema mínimo, pré-visualização, envio fora do jogo | Teste de "desligado não conecta" | Zero envio sem opção | Remover | Alto | NEXT |

## 9. Relatório de Performance compartilhável

- **Name:** Relatório compartilhável.
- **Customer Problem:** o cliente quer mostrar o resultado no Discord.
- **Business Value:** prova social honesta; o concorrente mostra print de FPS, nós mostramos intervalo.
- **Technical Value:** reusa `report.rs` e a prova gravada.
- **Architecture:** imagem/PDF gerado **só de prova alternada válida**, com as rodadas, o intervalo, o
  hardware em faixas e a frase "diferença dentro do ruído" quando for o caso.
- **Dependencies:** TOP 3.
- **Risks:** uso como marketing enganoso. Mitigação: o relatório sai com a margem de erro e não sai sem prova válida.
- **Security Impact:** nenhum dado pessoal.
- **Anti-Cheat Impact:** nenhum.
- **Performance Overhead:** nenhum.
- **How We Test:** teste de que não gera sem prova válida; conferência visual.
- **Success Criteria:** todo relatório tem o intervalo.
- **Failure Criteria:** relatório com ganho sem prova.
- **Rollback:** esconder o botão.
- **Priority:** NEXT.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T9.1 | Relatório da prova | Compartilhar com honestidade | `report.rs`, `provaalternada.rs` | T3.2 | Gerar a partir de `Resultado` válido | Sem prova → recusa | Sempre com intervalo | Esconder | Baixo | NEXT |

## 10. Primeira execução com perfil

- **Name:** Primeira execução.
- **Customer Problem:** "abri e não sei por onde começar"; expectativa errada ("vai dobrar meu FPS").
- **Business Value:** define a expectativa certa no primeiro minuto; menos reembolso.
- **Technical Value:** usa a identidade (TOP 4) e o Doctor (TOP 2) sem aplicar nada.
- **Architecture:** tela única na primeira abertura: lê a identidade (~20 s, nada alterado), mostra a
  limitação provável por regra (ex.: "placa de vídeo de entrada: o ganho esperado aqui vem de ritmo, não de
  FPS médio"), e convida a rodar o Doctor com o jogo. Nenhuma porcentagem de ganho prometida.
- **Dependencies:** TOP 4, TOP 2.
- **Risks:** estimar demais. Mitigação: nenhuma porcentagem; só a limitação e o próximo passo.
- **Security Impact:** nenhum.
- **Anti-Cheat Impact:** nenhum.
- **Performance Overhead:** só na primeira abertura.
- **How We Test:** fumaça com perfil vazio; regras de limitação com identidades de exemplo.
- **Success Criteria:** abre sem resposta do Windows; nunca mostra número de ganho.
- **Failure Criteria:** qualquer promessa numérica.
- **Rollback:** pular a tela.
- **Priority:** NEXT.

| TASK ID | TITLE | PURPOSE | FILES | DEPENDENCIES | IMPLEMENTATION | TEST PLAN | SUCCESS | ROLLBACK | RISK | PRIORITY |
|---|---|---|---|---|---|---|---|---|---|---|
| T10.1 | Regras de limitação | Expectativa honesta | novo `modules/perfil.rs` | T4.1 | Funções puras sobre a identidade | Identidades de exemplo | Nenhuma porcentagem | Reverter | Baixo | NEXT |
| T10.2 | Tela da primeira abertura | Começo guiado | `index.html`, `main.ts` | T10.1, T2.5 | Mostra perfil e leva ao Doctor | Fumaça | Abre sem resposta | Pular | Baixo | NEXT |

## Decisões que são do dono

| Decisão | Por que é dele | Bloqueia |
|---|---|---|
| Ordem de publicar a 3.2 | Regra de publicação | Item 0 |
| Endpoint da telemetria (bot ou serviço novo) e política de privacidade | Custo e responsabilidade legal | T8.3 |
| Certificado de assinatura de código | Desembolso | SmartScreen |
| Voluntários do canal beta | Relação com clientes | Laboratório, T8.2 |
| Repositório privado para código com SDK da AMD | Licença do ADLX | Sensores AMD |
