# Reparo do Windows — o Otimiza passa a consertar, não só a ajustar

**Data:** 31/08/2026
**Estado:** aprovado para virar plano de implementação

---

## O problema

O dono relatou que comandos de terminal sugeridos por um assistente ajudaram a
máquina dele mais que o Otimiza. A investigação achou o motivo, e ele não é de
opinião — é de código:

```
$ grep -rniE "sfc |scannow|dism|chkdsk|RestoreHealth" --include=*.rs .
(nenhum resultado)
```

**O produto tem 42 ajustes e nenhum reparo.** Os comandos que costumam ser
sugeridos — `sfc /scannow`, `DISM /RestoreHealth`, `chkdsk` — não são
otimização: são conserto. Eles atacam uma classe de defeito que o Otimiza
inteiro não trata.

Isso explica a experiência do dono sem precisar supor nada. Numa máquina com
arquivo de sistema corrompido, nenhum dos 42 ajustes adianta, e um reparo
resolve. O produto media, diagnosticava, aplicava — e a máquina continuava
ruim, pelo mesmo motivo que `health.rs` já descreve para disco morrendo: o
problema estava fora do alcance do que ele sabia fazer.

## O que este documento cobre

A primeira fatia: **o executor de tarefa longa e a aba de Reparo**, mais a
limpeza do WinSxS, que é o segundo consumidor natural do mesmo executor.

Ficam para especificações próprias, nesta ordem:

- Medir perda de pacote até o servidor do jogo
- Configurações do driver de vídeo (NVIDIA/AMD)
- Descobrir qual driver está travando o sistema

## O que já existe, e por isso não entra aqui

Verificado no código antes de escrever, porque este plano já propôs duas vezes
coisa que estava pronta:

| | |
|---|---|
| **Tempo de boot** | `boot.rs` lê do log de eventos, com os milissegundos de cada programa |
| **Cache do FiveM** | `fivem.rs` limpa, e protege a pasta que parece cache e guarda a sessão |
| **Sobreposições** | `conflicts.rs` reconhece Discord, Afterburner, Steam e GeForce |
| **Arquivo de paginação** | `memory.rs` |
| **Limpeza de temporários** | `cleanup.rs` e `diskspace.rs`, categoria por categoria |
| **Saúde do disco** | `health.rs` — e este documento depende dele |

---

## Princípio: reparo não é otimização

A regra fundadora do produto é que **toda mudança é reversível, com o valor
anterior guardado**. `sfc` não muda ajuste nenhum: ele devolve um arquivo
corrompido ao original. Não há valor anterior a guardar, e "desfazer"
significaria recorromper de propósito.

Três diferenças que decidem a arquitetura:

| | otimização | reparo |
|---|---|---|
| **Duração** | instantânea | 10 a 30 minutos |
| **Desfazer** | obrigatório | não faz sentido |
| **Reiniciar** | às vezes | `chkdsk /f` sempre |

Por isso o reparo **não entra no catálogo**. Ele ganha aba própria, e a tela diz
com todas as letras que ali não existe desfazer porque não há o que desfazer.

A alternativa considerada — marcar como irreversível dentro do catálogo, como
já é o `clean_temp_files` — foi recusada: ela mistura "vou apagar uns
temporários" com "vou mexer nos arquivos de sistema por meia hora" na mesma
lista, e a segunda merece uma tela que peça atenção.

---

## Arquitetura

### 1. `tarefa_longa.rs` — o executor que o produto não tem

Hoje todo comando do Otimiza responde em milissegundos, e a interface espera a
resposta. Um `DISM` de vinte minutos nesse formato congela a janela.

**Responsabilidade única:** rodar um processo externo, transmitir o andamento e
aceitar cancelamento.

- Estado num `Mutex` global: qual tarefa roda, desde quando, o que já saiu.
- A saída é lida linha a linha e emitida como evento Tauri, para a tela mostrar
  o andamento de verdade em vez de uma barra que anda sozinha.
- **Uma tarefa por vez.** Duas ferramentas de reparo simultâneas disputam os
  mesmos arquivos, e o resultado é imprevisível para os dois lados.
- Cancelar mata o processo filho. O que isso significa muda por ferramenta, e
  está documentado em cada uma abaixo — cancelar não é uniformemente seguro, e
  a tela precisa dizer a verdade sobre cada caso.

### 2. `reparo.rs` — as três ferramentas

#### Verificar arquivos de sistema — `sfc /scannow`

Compara os arquivos protegidos do Windows com as cópias boas e reescreve o que
estiver diferente. De 5 a 15 minutos.

- **Cancelar é seguro.** Interrompido, ele para; rodar de novo recomeça.
- **A LEITURA DO RESULTADO NÃO PODE SER PELO TEXTO DO CONSOLE.** Ele é
  traduzido: comparar "não encontrou nenhuma violação" quebraria em qualquer
  Windows que não seja português. É o mesmo defeito que `readiness.rs` já
  documenta para o `powercfg`. A leitura é feita nas entradas `[SR]` do
  `%windir%\Logs\CBS\CBS.log`, que não são traduzidas.

#### Reparar a imagem do Windows — `DISM /Online /Cleanup-Image /RestoreHealth`

O `sfc` conserta usando uma cópia local de referência. Se essa cópia também
estiver corrompida, o `sfc` falha — e é aí que o `DISM` entra, buscando os
arquivos bons no Windows Update.

- **A ordem importa:** quando o `sfc` acusa que não conseguiu corrigir, o certo
  é rodar o `DISM` e depois o `sfc` de novo. A tela encadeia isso sozinha.
- **Precisa de internet.** Sem ela, falha por um motivo que não é defeito do
  cliente, e a mensagem precisa dizer isso em vez de "erro".
- De 10 a 30 minutos, e **fica parado em 20% por vários minutos** — isso é
  normal e vai escrito na tela, porque é o momento em que a pessoa desiste.
- `DISM` aceita `/English`, o que dá uma saída estável para ler.
- **Cancelar aqui não é de graça.** Interrompido no meio de uma escrita, o
  `DISM` pode deixar uma operação pendente que só se resolve rodando de novo
  até o fim. O botão de cancelar diz isso antes de aceitar o clique, e o
  resultado registra que a tarefa foi interrompida — e não que terminou.

#### Verificar o disco — `chkdsk C: /scan`

**Esta é a decisão que separa o produto do que se ensina por aí.** O que se
sugere normalmente é `chkdsk /f`, que exige reiniciar e trava a máquina numa
tela azul de progresso por tempo indeterminado.

`chkdsk C: /scan` roda **com o Windows ligado, sem reiniciar**, em NTFS. Ele
acha os problemas sem consertar.

- Só se oferece `/f` **depois** de o `/scan` achar alguma coisa. Sem achado, não
  há motivo para reiniciar a máquina de ninguém.
- **Trava dura:** `/f` não é oferecido se o `health.rs` disser que o disco está
  em más condições. Num disco morrendo, o `chkdsk` é justamente o que costuma
  matá-lo de vez — e o produto já sabe reconhecer esse disco.
- **Cancelar o `/scan` é seguro:** ele só lê. Já o `/f` não chega a ser
  cancelável pelo Otimiza — ele fica agendado para a próxima inicialização, e
  quem desmarca é o `chkntfs /x`. A tela oferece esse desmarcar enquanto a
  máquina não reiniciou, em vez de deixar o cliente preso a uma decisão que
  ele tomou uma vez.

### 3. WinSxS — a limpeza que a Limpeza de Disco não faz

Segundo consumidor do mesmo executor.

- `DISM /Online /Cleanup-Image /AnalyzeComponentStore` diz quanto dá para
  recuperar **antes** de qualquer decisão, no mesmo espírito do `diskspace.rs`.
- `/StartComponentCleanup` recupera.
- **`/ResetBase` fica desligado por padrão.** Ele libera mais, e em troca o
  cliente perde a capacidade de desinstalar qualquer atualização já aplicada.
  Isso vai escrito na tela, ao lado da caixa de seleção — não numa nota de
  rodapé.

---

## O que a tela diz quando não acha nada

"Nenhuma corrupção encontrada" é um resultado **bom e verdadeiro**, e vai ser o
resultado mais comum. A tela diz isso com todas as letras, sem inventar
benefício e sem transformar em achado o que não é.

É a mesma regra do `prova.rs`, que se recusa a chamar ruído de ganho.

---

## Licença

- **Verificar é livre**, como todo diagnóstico do produto. `sfc /verifyonly`,
  `chkdsk /scan` e o `AnalyzeComponentStore` não mudam nada.
- **Consertar exige licença**, como toda correção. Entra em `EXIGEM_LICENCA`.

A guarda `quem_so_le_nao_pede_licenca` já existe e vai cobrar essa separação —
foi ela que pegou o `apply_game_profile` na lista errada.

## Testes

Não dá para rodar `sfc` na integração contínua. Os testes cobrem o que
realmente pode quebrar:

- A leitura do `CBS.log` nos três desfechos, com saídas gravadas de máquina real
- Nenhum texto traduzido é comparado em lugar nenhum
- `/f` nunca é oferecido com disco em más condições
- `/ResetBase` sai desligado
- Duas tarefas longas não rodam ao mesmo tempo
- Cancelar encerra o processo filho e devolve o estado para ocioso
- Verificar não pede licença; consertar pede

**O módulo novo precisa entrar na lane de `.github/workflows/release.yml`**, ou
a guarda `ci_coverage` reprova a publicação. Foi o que aconteceu com
`modules::prova`.

## Riscos conhecidos

| Risco | Tratamento |
|---|---|
| Saída traduzida quebra a leitura | Ler log e código de saída, nunca texto de console |
| Cliente desiste no "20%" | A tela avisa antes de começar |
| `chkdsk` em disco morrendo | Travado pelo `health.rs` |
| Cliente perde o desinstalar de atualizações | `/ResetBase` desligado e explicado |
| Máquina desligada no meio | Uma tarefa por vez, e cada ferramenta documenta o que acontece |
