# Otimiza 1.3 — o congelamento deixa de ser invisível

**Data:** 02/09/2026
**Estado:** aprovado para virar plano de implementação
**Recorte pedido pelo dono:** consertar os bugs grandes, com microajustes em
funcionalidade. Não é versão de novidade.

---

## O que aconteceu, e é a razão desta versão existir

Um cliente abriu o gerenciador de tarefas, viu `Steam — Suspenso`, depois o
Discord, depois o Chrome, e escreveu:

> *"onde que eu ativo aqui o bglh pra não bugar o discord"*
> *"Mn, tá mt bugado meu pc"*

Ele não sabia que tinha sido o Otimiza. Não tinha como saber: a tela não mostra
nada sobre congelamento, e não existe botão para desfazer.

A 1.1.2 consertou duas causas — a Steam saiu da lista, e o texto da opção passou
a dizer que congela. **Mas nenhuma das duas ajuda quem já está congelado.** Para
socorrer aquele cliente foi preciso escrever um script de PowerShell à mão, para
alguém que tinha o produto instalado.

**Esse é o defeito grande: o produto faz uma coisa poderosa e não a mostra.**

---

## O que esta versão faz

### 1. O congelamento aparece na tela, e tem botão para desfazer

Enquanto houver programa congelado, a tela do modo jogo mostra quantos e quais,
com um botão **Descongelar agora**.

Regras que decidem o desenho:

- **Aparece só quando há algo congelado.** Um painel vazio permanente vira
  ruído, e ruído é o que faz o cliente parar de ler a tela.
- **Diz o nome que a pessoa reconhece** — "Google Chrome", não `chrome.exe`.
- **O botão devolve tudo, na hora**, sem fechar o jogo e sem mexer no plano de
  energia. Descongelar não é desligar o modo jogo.
- **Some sozinho quando não houver mais nada congelado**, sem a pessoa precisar
  recarregar nada.

Isto substitui, dentro do produto, o script de socorro que hoje existe fora dele.

### 2. Quem já tinha o modo jogo ligado é avisado uma vez

O texto da opção mudou na 1.1.2 e passou a dizer que congela programas. Quem
**ligou antes disso nunca leu esse texto** — e é exatamente esse o cliente que se
assustou.

Na primeira abertura da 1.3, se o modo jogo automático estiver ligado, aparece
uma vez o que ele faz de verdade, com **Manter** e **Desligar**. Quem escolher
fica com a escolha registrada e não vê de novo.

Não é pedido de permissão retroativo por formalidade: é a única chance de contar
para quem ligou às cegas.

### 3. O `uac_off` passa a dizer que quebra aplicativo

O catálogo já diz que o ganho é **zero** e que é troca de segurança. Não diz o
que mais importa na prática: **com o UAC desligado, aplicativo da Loja da
Microsoft se recusa a abrir.**

É o mesmo defeito do modo jogo — o aviso fala de uma consequência e esconde a
que o cliente vai sentir.

**A opção continua no catálogo.** O dono a pediu por paridade com o concorrente,
e tirar item do catálogo é decisão de negócio, não de engenharia. O que muda é o
aviso. (Se o dono decidir remover, é uma linha.)

### 4. A cor do desfecho do reparo para de sair de comparar frase

`main.ts` decide a cor com `desfecho === "Terminou."`. É a **terceira aparição**
do mesmo defeito nesta base:

| Onde | Quando |
|---|---|
| `Corrigiu` escondendo arquivos não reparados | corrigido na Task 2 |
| A tela pintando `CorrigiuEmParte` de verde por prefixo | corrigido na Task 5 |
| **O desfecho da execução, por igualdade exata** | agora |

`reparo_executar` passa a devolver `{ tom, texto }`, como o vizinho
`reparo_ultimo_resultado` já faz. Enquanto a decisão sair de comparar prosa, ela
volta.

### 5. A linha de erro do DISM deixa de se misturar com o progresso

`Andamento` carrega só `linha` e `numero`. O `stderr` é drenado numa thread
separada e cai no mesmo lugar do progresso — então **"precisa de internet"**
chega embaralhado no meio de centenas de linhas de percentagem, e o cliente não
tem como saber qual delas é a razão da falha.

`Andamento` ganha a origem, e a tela distingue as duas.

### 6. A saída do reparo para de crescer sem limite

Trinta minutos de DISM acumulam a saída inteira num único elemento da página.
Passa a ter teto, mantendo o fim — que é onde está o resultado.

### 7. "Não consegui medir" para de virar "está tudo bem"

Esta é a regra fundadora do produto, e a varredura achou **duas leituras de
saúde que a quebram em silêncio**. As duas foram encontradas procurando o padrão
de propósito, não por acaso.

**Contador de erro do disco** — `health.rs:203`

```rust
let erros = contador.read_errors_total.unwrap_or(0)
          + contador.write_errors_total.unwrap_or(0);
if erros > 0 { /* avisa o cliente */ }
```

Muitos SSDs não publicam esse contador, e a consulta também falha sem
administrador. Nos dois casos o valor vira `0`, o `if` não entra, e **nenhum
achado é emitido** — o disco recebe atestado de saúde que ninguém mediu.

Pior pela vizinhança: é esta leitura que alimenta o `DiscoSaudavel`, que decide
se o `chkdsk /f` pode ser oferecido. O `DiscoSaudavel` já recusa quando não há
evidência nenhuma, então a trava não caiu — mas o cliente lê "disco bem" quando
o certo era "não consegui conferir".

**Limite térmico** — `thermal.rs:301`

```rust
let flags = contadores.performance_limit_flags.unwrap_or(0);
```

`0` significa "nenhum limite ativo". Se a leitura falhar, o produto conclui que
não há limitação térmica — e o argumento de venda deste módulo é justamente
"seu processador está em throttling e nenhum ajuste de software resolve".
Dizer que não está, sem ter medido, é o defeito no lugar mais caro.

**Nos dois:** o que falta é a terceira resposta. `Some(0)` é "medi e deu zero";
`None` é "não consegui medir"; e hoje os dois viram a mesma coisa. É a mesma
correção que o `cbslog::NaoSei` e o `DiscoSaudavel` já receberam.

### 8. Um módulo que falha não pode derrubar o diagnóstico inteiro

`veredito.rs` roda o diagnóstico rápido em paralelo, e usa `.lock().unwrap()` em
três pontos (`:807`, `:812`, `:828`). Se **qualquer** uma das tarefas entrar em
pânico, a tranca fica envenenada e todos os outros trabalhadores morrem no
`unwrap` seguinte.

O resultado é que um módulo instável derruba a tela que o cliente vê primeiro —
e ela é a parte livre, a que vende o produto antes de qualquer cobrança.

Tranca envenenada passa a ser tratada, e uma tarefa que falha vira lacuna
declarada em vez de queda geral. O produto já sabe mostrar lacuna: o
`ReadinessReport` tem campo para isso.

---

## Os microajustes

| | |
|---|---|
| **Consertar o disco** mostra "10 a 60 minutos" ao lado de um clique que volta em segundos. O trabalho acontece no próximo boot, e o texto tem que dizer isso |
| **A paleta de comandos** enumera os botões na montagem: não enxerga os do reparo, que nascem depois, e enxerga o "Interromper" escondido |
| **O `numero` do andamento** é calculado por duas threads e nunca atravessa para a tela. Ou serve para alguma coisa, ou sai |
| **O `restantes` do CBS.log** conta marcas brutas sem deduplicar por arquivo: se o `sfc` registrar duas tentativas falhas do mesmo arquivo antes de consertá-lo, ele reporta como quebrado um arquivo que foi consertado |
| **`TarefaLonga::ocupada` não é usada em lugar nenhum.** O `clippy` acusa, e o único chamador é um teste. Ou a exclusão volta a ser conferida por ela, ou o método sai — método público que só existe para um teste passar é teste medindo a si mesmo |
| **O `#[derive]` de `Andamento` carrega `numero`, que nunca é lido.** O `clippy` também acusa. Sai junto com a decisão do item acima |

### O que a varredura encontrou e NÃO é bug

Registrado para ninguém "consertar" depois sem motivo:

- `lib.rs:115` tem um `unwrap()` no `get_webview_window`, mas está dentro de
  `#[cfg(debug_assertions)]`: não existe na versão que o cliente instala.
- `monitor.rs:111` faz `duration_since(UNIX_EPOCH).unwrap()`. Só falha com o
  relógio da máquina antes de 1970. Fica.
- Os onze testes `#[ignore]` são deliberados — tocam o sistema de verdade e
  seriam instáveis na esteira. Não são cobertura faltando.

---

## O que esta versão NÃO faz

**Não promete consertar todos os bugs.** É a única frase que o cliente pode
conferir e provar errada, e um travamento depois dela vira munição.

**Não conserta o congelamento que o cliente relatou**, porque ainda não se sabe
o que foi. Falta o resultado do diagnóstico na máquina dele: se ele estava
jogando, se chegou a instalar a 1.1.2, e o que estava congelado. Entrar com
"consertamos isso" sem saber o que era é a promessa que este produto não faz.

Se o diagnóstico chegar antes do fim da implementação, entra.

---

## Verificação

- `cargo test --lib` — 472 hoje. Cada item entra com teste.
- `npx tsc --noEmit` limpo.
- Guarda nova: **nenhuma decisão de cor da interface pode sair de comparar texto
  vindo do backend.** É o defeito que já voltou três vezes; da terceira, vira
  teste.
- Módulo novo entra na lane de `.github/workflows/release.yml`, ou o
  `ci_coverage` reprova.
- As notas de versão precisam citar `1.3.0`.

## Riscos

| Risco | Tratamento |
|---|---|
| Descongelar pela tela enquanto o laço de 6s suspende | A tranca com prazo que a 1.1.1 introduziu já serializa os dois |
| A tela de reconsentimento virar incômodo | Aparece **uma vez**, e a escolha fica gravada |
| O teto da saída esconder o erro | O teto mantém o FIM, que é onde o resultado está |
