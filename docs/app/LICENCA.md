# A licença do Otimiza — o manual do dono

Este arquivo é para você, não para o cliente. Ele explica como o sistema de
chave funciona, o que fazer antes da primeira venda, e o que fazer em cada
situação que vai aparecer no Discord.

---

## Em uma frase

Você assina cada licença com uma chave que só você tem; o Otimiza confere a
assinatura com uma chave que qualquer um pode ver.

## Por que isso não é o jeito comum — e por que importa

O jeito comum é o programa guardar dentro dele o segredo que gera a chave e
comparar. O problema é que esse segredo viaja dentro do executável que o
cliente baixa: quem abrir o arquivo com um editor acha o segredo e escreve um
gerador de chaves. Foi assim que praticamente todo software dos anos 90 foi
pirateado.

Aqui é diferente. Existem **duas** chaves, e elas fazem coisas diferentes:

| | O que faz | Onde fica |
|---|---|---|
| **Privada** | Cria assinatura | Só com você. Nunca no repositório, nunca em conversa. |
| **Pública** | Confere assinatura | Dentro do Otimiza, à vista de todo mundo. |

A pública **só sabe conferir**. Arrancá-la do executável não permite forjar
nada — é como ter a fechadura sem ter a chave. O algoritmo chama-se **Ed25519**.

## O que este sistema NÃO faz

Precisa estar escrito, porque a diferença entre um produto honesto e um que
promete demais mora aqui: **qualquer** licença que roda no PC do cliente pode
ser contornada por alguém que edite o executável e arranque a conferência.
Nenhuma é inquebrável, nem a da Adobe.

O que esta entrega é impedir o repasse casual — a chave do vizinho não abre
aqui — e exigir habilidade real de quem quiser quebrar. Isso é o suficiente
para o público deste produto.

---

## O PAR DE CHAVES — já está feito

**A chave pública em `src-tauri/src/modules/licenca.rs` é a de produção**,
instalada em 29/08/2026 e gerada pelo dono. A privada correspondente está com
ele e no segredo do bot.

Até a versão anterior esta seção dizia que a chave era "de teste" e que vender
com ela era "vender com a porta destrancada". Isso deixou de ser verdade em
29/08/2026, e o documento não foi atualizado junto — num projeto cuja regra é
só afirmar o que verificou, um aviso que assusta sem motivo é o mesmo defeito
que um aviso que falta.

O resto desta seção descreve **como trocar o par**, e vale para o único caso em
que isso deve acontecer: um vazamento da privada. Leia o aviso do fim antes.

**Passo 1 — gerar o par**, num terminal que só você está vendo:

No PowerShell do Windows — que é o terminal padrão — o separador é `;`, e não
`&&`. O `&&` só existe no PowerShell 7 e no Prompt de Comando; no 5.1, que vem
com o Windows, ele dá erro de sintaxe antes de rodar qualquer coisa:

```bash
cd pc-optimizer\src-tauri; cargo run --example gerar_chave -- novo-par
```

Ou, mais simples, pelo Node — que não precisa compilar nada e leva um segundo:

```bash
node pc-optimizer\bot\otimiza-licenca.cjs novo-par
```

Os dois geram o mesmo tipo de par. Use o que for mais rápido para você.

Sai algo assim:

```
PUBLICA  (vai para CHAVE_PUBLICA em licenca.rs):
C0fmvRgj2Sb01AfppfzEx7VTlhc3VnvNF3qqYbq8nLA=

PRIVADA  (guarde; nunca versione, nunca cole em conversa):
e0VViM/y9yT1xuwRPk81x9IrFUWlgBIOIM6YHk6sdCg=
```

**Passo 2 — a PÚBLICA vai para o código.** Abra `licenca.rs`, ache
`const CHAVE_PUBLICA` e troque o valor. Atualize o comentário logo acima com a
data nova e o motivo da troca — ele é o que conta a história de qual chave está
valendo desde quando.

**Passo 3 — a PRIVADA você guarda em DOIS lugares.** Um gerenciador de senhas e
um pendrive, por exemplo. Nunca em pasta do projeto, nunca em mensagem, nunca
em captura de tela.

### Por que perder a privada é grave

Não dá para recuperá-la a partir da pública — é justamente essa
impossibilidade que faz o sistema funcionar. Perdendo, o único caminho é:
gerar par novo, publicar uma versão do Otimiza com a pública nova, e **reemitir
a licença de todos os clientes**. Guarde em dois lugares.

### Trocar a chave pública invalida tudo

Toda licença já emitida para de valer — as assinaturas que estão com os
clientes foram feitas pela privada correspondente à pública atual, e nenhuma
outra as reconhece.

A primeira instalação já aconteceu (29/08/2026). **Daqui em diante só existe um
motivo para trocar: um vazamento da privada.** E trocar significa republicar o
produto e reemitir a licença de cada cliente, um a um.

---

## A CADA VENDA

O cliente instala, abre, e vê o portão. Ele copia o código da máquina dele — a
forma é `OTZ-XXXX-XXXX-XXXX` — e manda no Discord junto com o pagamento.

Você emite:

```bash
node pc-optimizer\bot\otimiza-licenca.cjs emitir <PRIVADA> OTZ-XXXX-XXXX-XXXX "Nome do comprador"
```

Sai uma linha longa. É a chave. Manda para ele, ele cola no campo, e pronto.

**Com prazo**, se um dia você vender assinatura, basta um argumento a mais:

```bash
node pc-optimizer\bot\otimiza-licenca.cjs emitir <PRIVADA> OTZ-... "Nome" 2027-08-29
```

Sem esse argumento a licença é vitalícia.

### Pelo bot do Discord, que é como vai ser no dia a dia

Rodar `cargo` a cada venda não escala. O mesmo emissor existe em JavaScript, sem
dependência nenhuma, para viver dentro do seu bot:

    pc-optimizer/bot/otimiza-licenca.cjs

Um arquivo, copiado para dentro do projeto do bot. O passo a passo está em
[`bot/README.md`](../bot/README.md).

O bot **não se conecta** ao Otimiza — não há porta nem servidor entre os dois. O
que liga os dois é o par de chaves: o bot assina, o produto confere. Um teste do
lado do Rust guarda uma chave emitida de verdade pelo JavaScript e quebra o
build se os dois formatos divergirem.

### O emissor nunca entra no instalador

`gerar_chave.rs` está em `examples/` de propósito: `cargo build --release` não
compila exemplos. É a única peça do projeto que toca na chave privada, e ela
fica do lado de cá da cerca.

---

## De onde vem o código da máquina

Duas fontes, nesta ordem:

1. **Número de série da placa-mãe.** É a primeira escolha porque **sobrevive à
   formatação**, e formatar é o que o público deste produto mais faz.
2. **`MachineGuid` do Windows**, se a placa não tiver série utilizável. Muitos
   fabricantes deixam em branco ou escrevem literalmente "Default string" — o
   programa confere isso e recusa.

O `ProcessorId` foi testado e **recusado**. Nesta máquina ele é
`BFEBFBFF000A0653`, e esse mesmo valor aparece em todo processador do mesmo
modelo — não é número de série. Usá-lo faria a chave de um cliente abrir o PC
de outro.

O que o programa mostra na tela sai daí: com placa-mãe, ele diz que formatar
não custa chave nova; com `MachineGuid`, ele avisa que custa.

---

## O QUE FAZER NO DISCORD — os casos que vão aparecer

**"Comprei e a chave diz que é de outro computador."**
Ele trocou a placa-mãe, ou formatou numa máquina cujo código vinha do
`MachineGuid`. Peça o código novo e emita outra chave, sem custo. É a promessa
que a própria tela faz.

**"Passei minha chave para um amigo e não funcionou nele."**
Está correto. Cada chave vale em uma máquina só, e é exatamente isso que ela
existe para fazer.

**"A chave não é reconhecida."**
Quase sempre é cópia incompleta — a chave é longa e o Discord quebra linha. O
programa já perdoa espaço e quebra de linha, então o que sobra é falta de
pedaço. Peça para ele colar de novo, inteira.

**"Não foi possível identificar este computador."**
Costuma ser máquina virtual, onde não há série de placa nem `MachineGuid`
legível. Não venda antes de entender o caso.

**"Comprei mas o Otimiza não deixa aplicar nada."**
Confira se a chave foi mesmo colada. Sem licença, o diagnóstico roda e as
correções não — é assim de propósito.

---

## O que continua funcionando sem licença

Isto é decisão de produto, não descuido:

- **Todo o diagnóstico, medição e relatório.** É o que faz o portão mostrar o
  problema real da máquina em vez de texto de propaganda.
- **Desfazer.** Se a licença vencer, o cliente precisa conseguir voltar o PC ao
  que era. Trancar o "reverter" deixaria a máquina dele alterada sem caminho de
  volta pela nossa tela — seria sequestrar o computador de quem pagou.

Os 22 comandos que **alteram** o computador conferem a licença na primeira
linha, no Rust. A tela é conforto; o bloqueio é lá.

---

## O que o código garante sozinho

Três testes reprovam o build se alguém errar:

- `a_chave_privada_nunca_entra_no_produto` — procura sinal de chave privada em
  `licenca.rs`.
- `nenhum_comando_fica_sem_classificacao` — comando novo precisa ser declarado
  como leitura ou alteração.
- `quem_altera_o_computador_pede_licenca` — e, se for alteração, precisa
  conferir a licença na primeira linha do corpo.

A falha provável deste sistema não é alguém quebrar a assinatura. É você
acrescentar um comando daqui a seis meses e esquecer a linha da guarda. Agora
isso não passa.

---

## Depois do vazamento (outubro de 2026)

O que foi conferido e o que mudou. O relatório completo está na descrição do PR
`seguranca/licenca-endurecida`.

**A privada atual (a quarta, `Fz2A…`) não está no repositório.** Todo o
histórico do git foi varrido atrás de qualquer texto de 32 bytes em base64 que
gere uma das cinco públicas que o produto já usou; só apareceu a do par de
exemplo deste manual (`C0fm…`), que nenhuma versão publicada aceita.

**A terceira (`sR3n…`) é o furo mais provável.** A privada dela ficou na
hospedagem do bot perdida em 04/10/2026, e até a 3.5 toda versão a aceitava sem
limite: quem tiver aquele `.env` emite licença para qualquer máquina. A partir
desta versão ela só vale para licença emitida até 05/10/2026 (o bot carimba em
UTC: venda da noite de 04/10 sai com 05) e, com a lista
`MAQUINAS_DA_CHAVE_ANTERIOR` preenchida, só para as máquinas que compraram com
ela. Para gerar a lista a partir do banco do bot (um código por compra antiga):

```bash
node pc-optimizer\bot\otimiza-licenca.cjs resumo-da-maquina OTZ-AAAA-BBBB-CCCC OTZ-DDDD-EEEE-FFFF
```

Cada linha que sai vai para a lista, em `licenca.rs`. Só a data sem a lista não
segura quem tem a privada (ele assina a data que quiser); a lista sim.

**Atestado online (`licenca_online.rs`).** Em duas etapas, as duas pedem a sua
decisão:

1. Compilar com `OTIMIZA_API_LICENCA=https://…` (o mesmo endereço da API do
   site). O programa passa a pedir ao bot, a cada 6 h e só perto de vencer, um
   atestado de 30 dias, mandando a chave, o código da máquina, a versão e o
   SHA-256 do executável. O bot responde `{ atestado: emitirAtestado(...) }` ou
   `{ revogacao: emitirRevogacao(...) }` (só revogação assinada tranca), e a
   licença revogada para de abrir. Sem rede, nada
   muda para o cliente.
2. `EXIGE_ATESTADO = true`: sem atestado em dia, trancado. Só depois de a etapa 1
   mostrar que os clientes renovam.

O que o bot precisa ganhar para a etapa 1: a rota `POST /v1/licencas/atestado`,
uma tabela de revogadas, e o registro de quem pede (máquina, SHA-256 do
executável, IP). A mesma chave vindo de muitas impressões, ou um executável fora
dos publicados, é repasse ou binário remendado.

### Se for preciso trocar a privada de novo (rotação)

1. `node pc-optimizer\bot\otimiza-licenca.cjs instalar <.env do bot>` gera o par
   novo direto no `.env`, sem passar pela tela.
2. A pública que hoje é `CHAVE_PUBLICA` vira a anterior, com a data da troca em
   `FIM_DA_CHAVE_ANTERIOR` e a lista das máquinas de quem já comprou.
3. O bot reemite, com a privada nova, a chave de cada compra paga (ele tem o
   código de máquina de todas) e entrega no painel ou por mensagem. Quem não
   atualizar segue abrindo com a chave antiga na máquina dele; quem atualizar
   recebe a nova sem pedir.
4. A versão com a pública nova só é publicada depois de o bot já estar
   assinando com a privada nova.
