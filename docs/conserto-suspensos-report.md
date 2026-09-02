# Conserto: processos suspensos nunca devolvidos ao fechar/desligar

Branch: `conserto-suspensos`

## Resumo do problema

`modules/windows/suspend.rs` suspende processos de segundo plano durante o
modo jogo (nunca mata — decisão deliberada, documentada no cabeçalho do
arquivo). Até este conserto, a devolução (`retomar_tudo`) só acontecia em
dois lugares: quando o jogo fechava (`gamemode.rs`) e na abertura seguinte do
Otimiza (`retomar_pendentes`, em `lib.rs`). Não havia devolução ao fechar o
Otimiza nem ao encerrar a sessão do Windows (logoff/desligamento).

Cadeia de falha confirmada na máquina do dono: Otimiza suspende processos →
Otimiza fecha ou trava sem devolver → cliente desliga ou faz logoff → uma
thread suspensa não responde à mensagem de fim de sessão → o Windows não
consegue descarregar a colmeia de registro do usuário (evento 1512) → na
sessão seguinte a colmeia `UsrClass.dat` falha ao carregar (evento 1542) →
sem os registros COM do shell, o Explorer não abre e os atalhos da barra de
tarefas não fazem nada.

## O que foi construído

### 1. Devolução ao fechar o Otimiza (`src/lib.rs`)

Trocado `.run(tauri::generate_context!())` por `.build(...)` seguido de
`.run(|_app_handle, event| { ... })`, capturando `RunEvent::ExitRequested` e
`RunEvent::Exit` para chamar `suspend::retomar_tudo()` antes do processo
morrer. Os dois eventos são tratados juntos, de propósito redundante:
`retomar_tudo` devolver um processo que já está rodando não faz nada, então
chamar duas vezes não tem custo, e cobre o caso de um dos dois eventos não
chegar a disparar.

### 2. Devolução ao encerrar a sessão do Windows (`src/modules/windows/sessao.rs`, novo módulo)

**Mecanismo escolhido: `WM_QUERYENDSESSION`/`WM_ENDSESSION` na janela
principal — não `SetConsoleCtrlHandler`.**

Por quê: o Otimiza é uma aplicação de janela (carrega `user32.dll`), e a
própria documentação da Microsoft descreve que um processo assim não é
tratado como console para fins de logoff/desligamento — o `HandlerRoutine`
registrado por `SetConsoleCtrlHandler` simplesmente não é chamado para
`CTRL_LOGOFF_EVENT`/`CTRL_SHUTDOWN_EVENT` num processo que carregou
`user32.dll`. Confirmei isso via busca na documentação/discussões da
Microsoft antes de implementar (não assumi de cabeça) — a recomendação para
aplicação de janela é exatamente `WM_QUERYENDSESSION`.

O que verifiquei no código-fonte local (não só na doc): a versão do `tao`
usada por este projeto (`tao 0.35.3`, trazida por `tauri 2.11.5`) **não
processa `WM_QUERYENDSESSION`** —
`tao-0.35.3/src/platform_impl/windows/event_loop.rs:2382-2383` tem a
chamada comentada, com uma nota dizendo que o mecanismo equivalente ao
`ExitRequested` do Tauri ainda não foi introduzido para essa mensagem. E o
tratamento que dá a `WM_ENDSESSION` só derruba o laço de eventos
internamente (`event_loop_runner.loop_destroyed()`), sem gerar nenhum
`RunEvent` que o `.run()` do Tauri exponha. Ou seja: depender do sistema de
eventos do Tauri para esta notificação especificamente não funcionaria nesta
versão do projeto.

Por isso `sessao.rs` registra um subclasse PRÓPRIO na janela, usando
`SetWindowSubclass`/`DefSubclassProc` (comctl32) — a mesma API que o `tao` já
usa por baixo (confirmado grepando `tao-0.35.3/src/platform_impl/windows/event_loop.rs`,
que chama `SetWindowSubclass`/`RemoveWindowSubclass`/`DefSubclassProc`
diretamente). Essa API foi desenhada para permitir múltiplos donos na mesma
janela, cada um encadeando para o próximo — não é um controle paralelo por
fora do Tauri, é o mecanismo suportado para se somar ao que já está lá.

O handler chama `suspend::retomar_tudo()` já na consulta
(`WM_QUERYENDSESSION`), não só no aviso final (`WM_ENDSESSION`): a consulta
chega primeiro, e o orçamento de tempo antes de o Windows considerar o
processo travado conta a partir dela. Age nas duas mensagens — de novo,
redundante e sem custo, pelo mesmo motivo do item 1.

Precisei adicionar a feature `Win32_UI_Shell` ao `windows-sys` em
`Cargo.toml` (é onde os metadados do Win32 colocam essas duas funções, apesar
de a API em si não ter nada de shell).

O `HWND` chega em `lib.rs` via `janela.hwnd()` do Tauri (tipo do crate
`windows`) e é passado como `*mut c_void` cru para `sessao::instalar`, que o
reconstrói no tipo `HWND` do `windows-sys` — as duas representações são
idênticas por baixo, só evitando misturar os dois crates por um tipo que já é
o mesmo.

### 3. Prazo máximo (`suspend::retomar_se_expirado`, usado em `src/lib.rs`)

`Registro` (o `suspensos.json`) ganhou um campo `quando` (timestamp Unix,
`#[serde(default)]` para não quebrar leitura de um registro antigo sem o
campo), gravado toda vez que `suspender_fundo` grava a lista.

O vigia do modo jogo, que já roda a cada 6 segundos em `lib.rs`, chama
`suspend::retomar_se_expirado(PRAZO_MAXIMO_SEGUNDOS)` a cada volta, ANTES da
checagem da preferência `auto_game_mode` (o cliente pode ter desligado o modo
jogo automático depois que algo já ficou suspenso). A função só age quando
há suspenso pendente **e** não há jogo nenhum detectado agora — por mais
longa que seja uma partida legítima, o prazo nunca vence, porque a pergunta
"há jogo agora?" é recalculada a cada chamada.

**Prazo escolhido: 10 minutos (`PRAZO_MAXIMO_SEGUNDOS = 10 * 60`).** O
caminho normal (jogo fecha → `gamemode::passo` devolve na hora, a cada passo
de 6s) já cobre o caso comum; esta rede só entra quando o estado ficou
inconsistente — por exemplo o registro de mudanças (`ChangeLog`) e o
registro de suspensão (`suspensos.json`) são dois arquivos separados que, em
tese, podem sair de sincronia. Dez minutos é folgado o bastante para não
competir com o caminho normal, e curto o bastante para o cliente não conviver
com um programa congelado por uma tarde inteira quando algo deu errado.

### 4. `retomar_tudo` ganhou a mesma trava de PID reciclado que `retomar_pendentes`

Antes, `retomar_tudo` (chamado quando o jogo fecha, e agora também ao fechar
o Otimiza) confiava só no PID. `retomar_pendentes` já conferia `(pid,
inicio)` — o Windows recicla PIDs, e sem essa conferência o Otimiza podia
"retomar" um processo novo, sem relação nenhuma com o que suspendeu. Extraí
a conferência para uma função pura compartilhada,
`ainda_e_o_mesmo_processo`, usada agora pelos três caminhos de devolução
(`retomar_tudo`, `retomar_pendentes`, `retomar_se_expirado`).

## Testes

`cargo test --lib` (de `pc-optimizer/src-tauri`): **470 passaram, 0
falharam, 11 ignorados** (os ignorados já eram os testes de sistema real,
marcados `#[ignore]` antes deste conserto — nenhum novo `#[ignore]` foi
introduzido). Eram 462 antes; os 8 testes novos:

- `suspend::tests::retomar_tudo_recusa_pid_reciclado` — a trava de PID
  reciclado, testada de forma pura (sem depender de processo real).
- `suspend::tests::tempo_esgotado_respeita_o_limite` — a aritmética do prazo,
  incluindo o caso do relógio andando para trás.
- `suspend::tests::retomar_se_expirado_nao_mexe_com_jogo_rodando` — a
  garantia mais importante da rede por prazo: nunca interrompe partida em
  andamento.
- `suspend::tests::retomar_se_expirado_nao_mexe_antes_do_prazo`
- `suspend::tests::retomar_se_expirado_sem_nada_suspenso_nao_faz_nada`
- `suspend::tests::registro_antigo_sem_data_carrega_como_zero` — compatibili-
  dade com um `suspensos.json` gravado antes deste conserto.
- `sessao::tests::instalar_em_janela_nula_nao_estoura`
- `sessao::tests::nao_mata_processo` — a mesma trava de "isto nunca mata"
  que os outros módulos do conjunto já tinham.

O que NÃO dá para testar sem uma sessão gráfica real e um logoff de verdade
(e por isso não foi testado automaticamente, coerente com o pedido): o
recebimento de fato de `WM_QUERYENDSESSION`/`WM_ENDSESSION` pelo Windows.

`sessao.rs` foi adicionado ao passo "Testes — sistema e limpeza" em
`.github/workflows/release.yml` (mesma linha de `suspend::`), e
`cargo test --lib -- ci_coverage::` passa — a trava que reprova módulo com
teste fora da esteira não acusa nada.

`cargo build` (binário completo, não só a lib) também compila sem erro.

## Commits

Ver histórico do branch `conserto-suspensos` para os hashes.

## Revisão pós-aprovação: dois achados fechados antes de subir

### Achado 1 (Important) — corrida entre suspender e retomar, sem tranca

Depois de `retomar_tudo()` passar a ser chamável pela thread da janela
(gancho de fim de sessão e fechamento do Otimiza), ela podia correr junto
com `suspender_fundo()` — que grava o registro, depois suspende um a um numa
thread de fundo. Cenário: jogo abre, `suspender_fundo` já gravou e está no
meio do laço de suspensão, cliente desliga nesse instante; o gancho de fim de
sessão lê o registro, retoma o que já tinha sido suspenso (inofensivo) e
APAGA o arquivo; o laço de suspensão, ainda no meio, suspende o resto sem
registro nenhum — o mesmo defeito deste conserto inteiro, numa janela de
milissegundos.

Fechado com um `static TRANCA: Mutex<()>` em `suspend.rs`, segurado:

- Por `suspender_fundo`, do início da gravação até o fim do laço de
  suspensão — **bloqueio sem prazo**, porque quem chama está numa thread de
  fundo (o vigia do modo jogo), sem orçamento de tempo do Windows para
  respeitar.
- Por `retomar_tudo` e por `retomar_se_expirado` — **bloqueio com prazo**,
  via `tentar_travar(PRAZO_TRANCA)`.

**Como resolvi a espera com prazo:** `tentar_travar` tenta `TRANCA.try_lock()`
a cada 10ms, até um teto de 300ms (`PRAZO_TRANCA`). Se conseguir, segura a
tranca normalmente; se o prazo vencer (ou a tranca estiver envenenada por um
pânico anterior), devolve `None` e quem chamou segue SEM a tranca — nunca
espera para sempre. A justificativa está no comentário de `PRAZO_TRANCA`: o
gancho de fim de sessão roda na thread da janela, sob o orçamento curto que o
Windows dá antes de considerar o processo travado e matá-lo; travar o
desligamento do cliente esperando a tranca afetaria TODO cliente que
desligasse com algo suspenso, corrida ou não, enquanto seguir sem a tranca só
reabre a janela rara da corrida — pior que fechá-la de vez, mas
infinitamente melhor que nunca devolver, que era o defeito original. 300ms é
folgado frente ao trabalho real que a tranca protege (gravar um JSON pequeno
e suspender até ~15 threads, sem PowerShell nem rede), então na prática o
prazo quase nunca é atingido — ele só existe para o caso raro em que atinge.

`retomar_pendentes` (a rede de segurança que roda na abertura do programa,
antes do laço de eventos começar) não foi alterada: ela roda sozinha, antes
de qualquer suspensão ou gancho de sessão poder disparar, então não há
corrida nenhuma para fechar ali — mexer nela seria alargar o diff sem
reduzir risco nenhum.

### Achado 2 (Minor) — nada exercitava a fiação de `retomar_se_expirado()`

Todos os testes do prazo passavam pelo `_com`, com o booleano do jogo
escolhido à mão; a linha que liga a função pública ao detector de verdade —
`super::gamemode::jogo_aberto().is_some()` — não tinha teste nenhum, e uma
inversão de uma linha ali (`.is_none()`, ou os argumentos fora de ordem)
derrubaria a garantia de "nunca no meio da partida" sem que nada acusasse.

**Escolha: costura `#[cfg(test)]` que reusa a fiação real, trocando só o
caminho do arquivo — não um teste de integração puro, e não um detector
injetado por parâmetro.** `retomar_se_expirado_no_caminho` (nova, só em
teste) chama exatamente a mesma linha `super::gamemode::jogo_aberto().is_some()`
que a função pública chama, mudando apenas o `Registro::path()` real por um
caminho de teste — o mesmo padrão `_de`/`_em` que o resto do arquivo já usa
para nunca tocar no `suspensos.json` do produto durante os testes. Preferi
isto a um parâmetro de detector injetável porque um detector trocável
permitiria a um teste futuro continuar passando mesmo com a fiação de
produção invertida — o parâmetro de teste vira o que é exercitado, não a
linha real. Aqui, a linha real de produção é literalmente a que roda no
teste.

O teste novo, `a_fiacao_publica_usa_o_jogo_de_verdade`, grava um registro já
vencido (`quando = 0`, prazo `0`) e chama a costura. A conferência é o
ESTADO DO ARQUIVO depois — não a lista devolvida, porque o PID gravado no
teste é inventado e nenhum processo vivo de verdade bate com ele (assunto de
outro teste, `retomar_tudo_recusa_pid_reciclado`). O que importa é o portão
de entrada de `retomar_se_expirado_com`: com `jogo_rodando` errado, ela
devolve cedo e NUNCA limpa o arquivo; com `jogo_rodando` certo, ela passa do
portão e limpa. Na esteira (sem sessão gráfica, sem jogo nenhum rodando de
verdade) `jogo_aberto()` de verdade devolve `None`; com a fiação certa isso
vira `jogo_rodando = false`, o portão libera, e o arquivo some — é essa
consequência observável que o teste tranca. Numa máquina de desenvolvimento
com um jogo de verdade aberto, o teste se abstém (limpa o arquivo de teste e
retorna) em vez de reprovar por um motivo alheio à fiação.

### Verificação

`cargo test --lib`: **471 passaram, 0 falharam, 11 ignorados** (eram 470
antes desta rodada; 1 teste novo, `a_fiacao_publica_usa_o_jogo_de_verdade` —
os outros ajustes do Achado 1 não precisaram de teste novo, só reusaram os
testes de `retomar_tudo`/`retomar_se_expirado` já existentes, que continuam
passando com a tranca no meio do caminho). `cargo build` (binário completo)
também compila sem erro. `cargo test --lib -- ci_coverage::` continua
passando.
