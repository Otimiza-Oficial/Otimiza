# Otimiza 1.5 — o produto passa a medir o que ninguém mostra

**Data:** 03/09/2026
**Estado:** rascunho para aprovação do dono
**Recorte:** versão grande, de capacidade nova. A 1.3 foi de conserto; esta é
de alcance.

---

## De onde esta versão vem

A pesquisa que precedeu a 1.0 levantou cinco lacunas reais. Três viraram a 1.0 e
a 1.1 (reparo do Windows, limpeza do WinSxS, configuração do jogo). **Duas
ficaram**, e uma terceira apareceu depois:

| | Situação hoje |
|---|---|
| Perda de pacote até o servidor do jogo | `network.rs` mede DNS e **nunca** mede perda. Verificado. |
| Configurações do driver de vídeo | Não existe nada. `gpupref.rs` escolhe *qual placa*, não *como ela se comporta*. |
| Qual driver está travando o sistema | Nenhuma ocorrência de DPC no projeto. |

Nenhuma das três existe. Conferido no código antes de escrever — este plano já
propôs duas vezes coisa que estava pronta, e a conferência custa um comando.

---

## Pilar 1 — A perda de pacote, que é a causa real do teleporte

**É o item que mais casa com este produto, e o mais barato dos três.**

O `network.rs` abre dizendo, com todas as letras, que reduzir ping é mentira:
ping é distância física mais roteamento, e nenhum ajuste no PC do cliente muda
isso. Essa recusa está certa e fica.

Mas há um número que **ninguém no mercado mostra** e que é a causa real da
queixa do jogador de FiveM: **perda de pacote**. Quando o personagem teleporta,
quando o carro anda sozinho, quando o tiro não registra — isso quase nunca é
FPS, e quase nunca é ping. É pacote que não chegou.

E é medível, com precisão, sem prometer nada.

### O que ele faz

Mede, contra o servidor em que o cliente está jogando: perda de pacote, a
variação do tempo de resposta (jitter de rede, que é diferente do jitter de
quadros que o produto já mede), e o tempo de resposta em si.

### O que ele NÃO faz, e vai escrito na tela

**Não promete melhorar nada.** Perda de pacote quase sempre é do provedor, do
cabo, do Wi-Fi ou do servidor — não do PC. O produto mede e diz onde está,
porque saber que o problema é o Wi-Fi economiza ao cliente uma tarde
reinstalando driver e uma otimização paga que não ia adiantar.

Isto é a mesma promessa da aba de reparo, invertida: lá o produto conserta o que
ninguém conserta; aqui ele mede o que ninguém mede, e admite que não conserta.

### A descoberta que muda o argumento de venda deste pilar

A pesquisa trouxe uma frase que vale mais que o recurso:

> *A instabilidade muitas vezes vem do roteamento instável. Quando a conexão
> oscila, os quadros podem congelar, produzindo um engasgo **que parece
> idêntico a perda de desempenho**.*

Ou seja: **travamento de rede é indistinguível de FPS baixo para quem está
jogando.** O carro que anda sozinho, o tiro que não registra, a travada de meio
segundo — tudo isso o jogador chama de "PC ruim".

O que isso significa comercialmente: **parte dos clientes que compram um
otimizador de FPS têm um problema que nunca foi FPS.** Eles vão otimizar, medir,
não ver diferença, e pedir reembolso — com razão, porque o produto não podia
resolver o problema deles.

Medir perda de pacote não é só um recurso a mais. É o que separa "não adiantou"
de "não era isso, e aqui está a prova" — e é a diferença entre um reembolso e um
cliente que confia.

### A parte difícil, e ela é real

Descobrir contra QUEM medir. O FiveM se conecta a um servidor cujo endereço não
está numa configuração fixa. Precisa sair das conexões ativas do processo do
jogo. **Se não der para descobrir com segurança, o produto diz que não
descobriu** — não mede contra um alvo qualquer e apresenta como se fosse o
servidor do jogo.

---

## Pilar 2 — As configurações do driver de vídeo

**É o que o concorrente vende, e é o único dos três que mexe em algo.**

O painel da NVIDIA e o da AMD têm ajustes que mudam latência e estabilidade de
quadro. Hoje o Otimiza não toca em nenhum.

### A moldura honesta, e ela decide o pilar inteiro

"Prefer Maximum Performance" **não dá FPS quando a placa já está saturada.** Ele
ajuda quando o jogo é limitado pelo processador e a placa baixa o clock sozinha.
É ganho de latência e de estabilidade, não de média.

Então este pilar entra medindo: aplica, mede antes e depois com o `prova.rs` que
já existe, e mostra o resultado como ele for — inclusive zero. Sem isso ele vira
o "+300 FPS" do concorrente com outro nome.

### Reversibilidade

Toda mudança entra no histórico com o valor anterior, como qualquer outra. Isso
é inegociável e já é a regra da casa.

### O risco a decidir antes de começar

Mexer no driver de vídeo por caminho não documentado é diferente de mexer no
registro do Windows. Precisa de investigação própria: qual é a superfície
suportada, o que é gravável com segurança, e o que acontece quando o driver
atualiza por cima. **Se a resposta for "não há caminho seguro", este pilar não
entra** — e isso é um resultado legítimo, não um fracasso.

---

## Pilar 3 — Qual driver está travando o sistema

**O mais difícil e o mais valioso quando acerta.**

O produto já mede engasgo (`jitter.rs`) e sabe dizer que travou. Não sabe dizer
**quem** travou. Um driver com latência alta de DPC congela o áudio, engasga o
mouse e produz travadas que nenhum ajuste resolve — e o cliente reinstala
Windows sem nunca descobrir que era a placa de rede.

### Por que fica por último

Depende de instrumentação de eventos do Windows que o projeto ainda não usa, e o
risco de apontar o driver errado é alto. **Acusar o driver errado é pior que não
acusar nada:** o cliente desinstala algo que funcionava.

Regra: só nomeia um driver quando a medição sustentar. Caso contrário diz que
achou latência alta sem conseguir atribuir — que já é mais do que ele sabia.

---

---

## Pilar 4 — O relatório que o cliente cola no atendimento

**Este nasceu de uma coisa que aconteceu esta semana, duas vezes.**

Um cliente relatou que programas não abriam. O dono não tinha como ver a máquina
dele e ofereceu AnyDesk. E foi preciso escrever um script de PowerShell **à mão**
para diagnosticar a máquina de alguém que já tinha o produto instalado.

O Otimiza estava naquela máquina, sabia tudo o que era preciso saber, e não tinha
como contar.

### O que ele faz

Um botão que copia para a área de transferência um bloco de texto curto com o
que o atendimento precisa: versão instalada, o que está congelado agora, quais
mudanças o produto aplicou, o que a leitura de saúde diz, e o que ele **não
conseguiu** ler.

O cliente cola no Discord ou no WhatsApp. Acabou o AnyDesk para 90% dos casos.

### O que já existe e NÃO serve

O `report.rs` gera um PDF para o técnico entregar ao cliente, provando o serviço.
É outra coisa, e continua. PDF não se cola numa conversa.

### As regras que decidem o formato

- **Cabe numa mensagem.** Se não couber, não é colável, e ninguém vai anexar
  arquivo no meio de um atendimento.
- **Não leva dado pessoal.** Nem nome de usuário, nem caminho com o nome da
  pessoa, nem nada que identifique além da máquina. O produto já guarda só o
  código da placa-mãe na tabela de compras, e este relatório segue a mesma
  regra.
- **Diz o que não conseguiu ler**, com a mesma voz das leituras de saúde: não
  conseguir medir não é o mesmo que estar bem.

---

## Pilar 5 — O aviso de versão nova dentro do programa

**Este é uma lacuna que a venda por WhatsApp acabou de criar.**

Hoje quem descobre que saiu versão nova é quem está no Discord: o bot avisa por
mensagem direta. Mas o produto passou a ser vendido **fora** do Discord, e quem
compra por WhatsApp pode nunca entrar no servidor.

Para essa pessoa, o Otimiza é um programa que **nunca atualiza**. Ela vai ficar
na versão do dia da compra para sempre, inclusive quando essa versão tiver um
defeito que já foi consertado — e foi exatamente isso que aconteceu esta semana,
com um cliente relatando um problema já corrigido numa versão que ele não sabia
que existia.

### O que ele faz

O programa pergunta ao GitHub qual é a versão mais nova e, quando for maior que
a instalada, mostra um aviso discreto com o que mudou e o link para baixar.

### As regras

- **Pergunta, não é avisado.** O produto não abre porta nem fala com servidor
  nosso — a consulta é anônima e ao GitHub, como o bot já faz.
- **Não interrompe.** Não é caixa modal no meio do trabalho: é uma faixa que o
  cliente vê e fecha.
- **Falha de rede é silêncio, não erro.** Não conseguir perguntar não é
  problema do cliente e não vira alarme na tela dele.
- **Não instala nada sozinho.** Baixar e instalar continua sendo escolha dele.

---

## Pilar 6 — O `CitizenFX.ini`, que é do público que já compra

O produto já lê o `gta5_settings.xml` — a configuração **gráfica** do jogo. Nunca
tocou no `CitizenFX.ini`, em `%APPDATA%\CitizenFX\`, que é a configuração do
**motor do cliente FiveM**. Verificado: nenhuma ocorrência no projeto.

### O que há lá

`PoolSize` controla o limite interno de veículos que o GTA V mantém. Em servidor
de RP com muito carro personalizado — que é exatamente onde o público deste
produto joga — estourar esse limite produz **engasgo repentino**: a travada que
acontece quando muita coisa aparece de uma vez, e que nenhum ajuste gráfico
resolve porque não é a placa de vídeo que está no limite.

É a mesma família do que a 1.0 já faz com a configuração do jogo: um número
escrito num arquivo, que segura a máquina sem que ninguém saiba.

### A ressalva que precisa vir junto

**Aumentar o pool custa memória.** Numa máquina com pouca RAM, subir esse número
piora — troca um engasgo por outro. O produto já sabe quanta memória a máquina
tem e quantos encaixes estão ocupados; essa leitura tem que **gatilhar a
decisão**, não ficar de enfeite.

Regra: só oferece quando a memória sustenta, e diz o que está trocando. Sem isso
vira o "aumente seu FPS" de sempre, com um número diferente.

### Por que ele entra

É a única coisa da 1.5 que serve **especificamente** ao FiveM, que é onde estão
os clientes de hoje. Os outros cinco pilares valem para qualquer jogo — este
vale para o que eles jogam.

---

## Os bugs que ficaram

**O único real, e ele é de honestidade.** No `cbslog.rs`, quando o registro do
`sfc` tem cinco arquivos com falha e só um deles pode ser interpretado — e esse
um foi consertado —, o produto devolve `Corrigiu { quantos: 1 }`. Isso se lê como
sucesso sem ressalva, enquanto quatro linhas de falha de estado desconhecido
foram descartadas em silêncio.

A 1.3 fechou o caso extremo (nenhuma linha interpretável passou a virar "não
sei", nunca mais "sem corrupção"). O caso **misto** ficou, e ele lê como
tranquilidade quando o quadro é incerto.

## Os microajustes que atravessaram da 1.3

| | |
|---|---|
| O texto de pânico do `veredito` usa um prefixo próprio que nenhum outro `Err` do arquivo usa |
| `congelados()` não tem teste de fiação, ao contrário da convenção do próprio módulo |
| O teto de 500 linhas da saída do reparo limita número de nós, não volume de caracteres: uma linha gigante não é despejada |

## O que esta versão NÃO promete

**Não promete FPS.** Nenhum dos três pilares é um ajuste que aumenta quadro. O
primeiro mede rede, o segundo troca latência por energia, o terceiro nomeia
culpado. O produto continua sem número mágico.

**Não promete os três.** O Pilar 2 depende de existir caminho seguro no driver,
e o Pilar 3 de a medição sustentar a acusação. Se um cair na investigação, cai —
e a versão sai com o que ficou de pé, dizendo o que não deu.

---

## Ordem, e ela não é a ordem de tamanho

1. **O relatório colável** — é o menor dos seis e o que devolve tempo já na
   semana que vem. Cada atendimento que ele encurta paga a implementação.
2. **O aviso de versão nova** — fecha a lacuna que a venda por WhatsApp abriu, e
   é pré-requisito para tudo o mais: não adianta consertar defeito em versão que
   o cliente nunca vai instalar.
3. **Perda de pacote** — o mais alinhado ao produto, e o número que ninguém
   mostra.
4. **O `CitizenFX.ini`** — o único que serve especificamente ao FiveM, que é
   onde os clientes de hoje estão.
5. **Driver de vídeo** — o que o concorrente vende, com a moldura que ele não
   tem. Investigação antes de decidir.
6. **Qual driver trava** — o mais difícil. Investigação primeiro, decisão depois.

Os bugs e os microajustes entram junto do pilar que toca o mesmo arquivo, e não
como fatia separada no fim — fatia de limpeza no fim é a que sempre cai quando o
prazo aperta.

## Verificação

- `cargo test --lib` — 495 hoje. Cada pilar entra com teste.
- **Na máquina do dono, com o FiveM aberto:** medir a perda contra o servidor
  real, e conferir se o número bate com o que ele sente jogando. É o teste que
  diz se o Pilar 1 vale.
- Módulo novo entra na lane de `.github/workflows/release.yml`.
- As notas precisam citar `1.5.0`.
- A trava da prosa continua valendo: nenhuma decisão de interface sai de comparar
  texto do backend.

## Pendência que atravessa da 1.3

A causa do congelamento relatado por um cliente continua desconhecida. Falta o
resultado do `descongelar.ps1` na máquina dele. Se chegar, vira tarefa; se não,
não entra e a versão não afirma tê-lo consertado.
