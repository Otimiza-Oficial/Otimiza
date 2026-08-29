# Plugar o seu bot no Otimiza

## Não existe conexão. E é de propósito.

O bot **não fala** com o Otimiza. Não há porta, não há servidor, não há chamada
de rede entre os dois — o Otimiza não tem camada de rede nenhuma, e essa é a
coisa mais defensável do produto: ele não pode mandar dado da máquina do cliente
para lugar nenhum porque não sabe como.

O que liga os dois é a **matemática**. O bot assina com a chave privada; o
Otimiza confere com a pública. São duas metades do mesmo par. O bot podia rodar
em Marte que a chave abriria aqui.

```
cliente abre o Otimiza  →  copia OTZ-XXXX-XXXX-XXXX da tela
        ↓
manda no Discord + paga
        ↓
o bot chama emitir()
        ↓
o bot responde com a chave  →  cliente cola  →  abre
```

## Gerar o par de chaves (uma vez na vida)

**Clique duas vezes em `gerar-par-de-chaves.bat`.**

É só isso. Não precisa de terminal, não precisa acertar pasta, não precisa de
`npm install`.

Se preferir o terminal, use o caminho completo — caminho relativo só funciona se
você estiver exatamente na pasta certa, e é aí que costuma dar errado:

```
node "C:\caminhote\o\projeto\pc-optimizerot\otimiza-licenca.cjs" novo-par
```

No PowerShell do Windows o separador de comandos é `;`, nunca `&&` — o `&&` só
existe no PowerShell 7 e no Prompt de Comando, e no 5.1 que vem com o Windows
ele dá erro de sintaxe antes de rodar qualquer coisa.

## Instalar

Copie **um arquivo** para dentro do projeto do bot:

```
otimiza-licenca.cjs
```

**Não precisa de `npm install`.** Sem dependência nenhuma. O Node faz Ed25519 nativo desde a versão 12,
e um emissor de licença é a última coisa do mundo que deveria puxar biblioteca
de terceiros: cada pacote no meio é mais alguém com acesso potencial à sua chave
privada.

## A chave privada

Vai numa variável de ambiente, **nunca escrita no código do bot**. Se o
repositório do bot vazar, a chave não vai junto.

No `.env` do bot:

```
OTIMIZA_CHAVE_PRIVADA=cole-aqui-a-linha-PRIVADA
```

E confira que o `.env` está no `.gitignore`.

## Usar

```js
const { emitir } = require("./otimiza-licenca.cjs");

const chave = emitir({
  privada: process.env.OTIMIZA_CHAVE_PRIVADA,
  maquina: "OTZ-WPYY-0J4F-77AB",   // o que o cliente copiou da tela
  comprador: "fulano#1234",         // só para você saber de quem é
});
```

Com prazo, se um dia você vender assinatura:

```js
const chave = emitir({ /* ... */, expira: "2027-08-29" });
```

Sem `expira`, a licença é vitalícia.

## Num comando de barra do discord.js

```js
const { SlashCommandBuilder } = require("discord.js");
const { emitir } = require("./otimiza-licenca.cjs");

module.exports = {
  data: new SlashCommandBuilder()
    .setName("liberar")
    .setDescription("Emite a chave do Otimiza para um cliente")
    .addStringOption((o) =>
      o.setName("maquina")
        .setDescription("O código OTZ-XXXX-XXXX-XXXX que o cliente mandou")
        .setRequired(true))
    .addUserOption((o) =>
      o.setName("cliente")
        .setDescription("Quem comprou")
        .setRequired(true))
    // Só quem administra o servidor emite chave. Sem isto, qualquer pessoa
    // com acesso ao comando libera o produto de graça.
    .setDefaultMemberPermissions(0),

  async execute(interacao) {
    const maquina = interacao.options.getString("maquina");
    const cliente = interacao.options.getUser("cliente");

    let chave;

    try {
      chave = emitir({
        privada: process.env.OTIMIZA_CHAVE_PRIVADA,
        maquina,
        comprador: cliente.tag,
      });
    } catch (erro) {
      // Quase sempre é código de máquina digitado errado. Dizer o que houve
      // evita o pior caso: o cliente paga, recebe uma chave e ela não abre.
      return interacao.reply({ content: `Não deu para emitir: ${erro.message}`, ephemeral: true });
    }

    // A CHAVE VAI POR MENSAGEM DIRETA, NÃO NO CANAL.
    //
    // Ela não é secreta — só abre naquele PC. Mas uma chave no canal público é
    // um convite para outra pessoa tentar usá-la, descobrir que não funciona, e
    // abrir um chamado de suporte que não existiria.
    await cliente.send(
      [
        "Sua chave do Otimiza:",
        "```",
        chave,
        "```",
        "Cole no campo **A sua chave** e clique em Ativar.",
        "Ela vale só neste computador. Trocou de placa-mãe ou formatou? " +
          "Me chame com o código novo que eu reemito sem custo.",
      ].join("\n")
    );

    await interacao.reply({
      content: `Chave enviada para ${cliente} na mensagem direta.`,
      ephemeral: true,
    });
  },
};
```

## Conferir uma chave que o cliente diz que não funciona

Sem pedir o PC dele:

```js
const { conferir } = require("./otimiza-licenca.cjs");

console.log(conferir(chaveQueOClienteMandou, "cole-aqui-a-PUBLICA"));
// { valida: true, dados: { maquina, comprador, emitida, expira } }
```

Se voltar `valida: true` e o cliente continua reclamando, compare o campo
`maquina` com o código que ele está vendo na tela agora: quase sempre ele
formatou, ou trocou de PC.

## Na linha de comando, sem bot

```
node otimiza-licenca.cjs novo-par
node otimiza-licenca.cjs emitir <PRIVADA> OTZ-XXXX-XXXX-XXXX "Nome" [AAAA-MM-DD]
```

## O que garante que a chave do bot abre o produto

Duas implementações independentes do mesmo formato — uma em JavaScript, outra em
Rust — podem divergir de um jeito que nenhum lado percebe sozinho: uma vírgula a
mais no JSON, um base64 com `+` no lugar de `-`, um byte de padding. O prejuízo
desse erro é o pior que este produto pode ter: o cliente paga, recebe a chave, e
ela não abre.

Por isso existe um teste no lado do Rust — `a_chave_emitida_pelo_bot_abre_o_produto`
— com uma chave de verdade emitida por este arquivo. Se os formatos divergirem,
o build quebra antes de a versão sair.

## O que este arquivo NÃO resolve

Ele não cobra. Não sabe se o cliente pagou, e não deve saber: quem decide
liberar é você, no momento em que chama `emitir()`. Se um dia entrar pagamento
automático, o lugar de conferir isso é antes dessa chamada — nunca dentro dela.
