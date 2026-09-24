# Assinatura digital do instalador

## O que está pronto e o que depende de você

A configuração de build já está preparada para assinar. O que **não** dá para
resolver por código é o certificado em si: ele exige compra, verificação de
identidade do titular e, hoje, um dispositivo físico ou serviço em nuvem que
guarde a chave privada. Isso é decisão e desembolso seus.

## Por que importa

Sem assinatura, o Windows SmartScreen mostra **"O Windows protegeu o seu PC —
editor desconhecido"** na primeira execução. O usuário precisa clicar em "Mais
informações" e depois em "Executar assim mesmo". Quem já te conhece faz isso.
Quem não te conhece desiste — e some com a venda antes de o produto abrir.

## Tipos de certificado

| Tipo | Reputação no SmartScreen | Onde fica a chave |
|---|---|---|
| **OV** (Organization Validation) | Precisa ser construída ao longo de downloads e tempo | Token USB ou HSM em nuvem |
| **EV** (Extended Validation) | Imediata, sem período de carência | Token USB obrigatório |

Desde 2023 as autoridades certificadoras não emitem mais certificado de
assinatura de código como arquivo `.pfx` simples: a chave privada precisa ficar
em hardware certificado. Qualquer tutorial que mande gerar um `.pfx` e assinar
com ele está desatualizado.

**Emissoras conhecidas:** DigiCert, Sectigo, SSL.com, Certum. A faixa de preço
costuma ficar entre algumas centenas de dólares por ano, variando bastante entre
OV e EV. Existe também o **Azure Trusted Signing**, com mensalidade bem menor,
mas com regras de elegibilidade próprias quanto ao país e ao tempo de existência
da empresa. **Confirme preço e elegibilidade direto com a emissora** — essas
condições mudam com frequência e não vale confiar em número de terceiro.

Para pessoa física brasileira, o caminho mais comum é um certificado OV emitido
por uma dessas autoridades, com token enviado pelo correio.

## Depois que o certificado estiver na mão

### Certificado em token USB ou HSM

Instale o token, descubra a impressão digital do certificado:

```bash
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Format-List Subject, Thumbprint
```

E acrescente ao `src-tauri/tauri.conf.json`, dentro de `bundle.windows`:

```json
"certificateThumbprint": "COLE_A_IMPRESSAO_DIGITAL_AQUI",
"digestAlgorithm": "sha256",
"timestampUrl": "http://timestamp.digicert.com"
```

O `timestampUrl` não é opcional na prática: sem carimbo de tempo, tudo o que você
assinou para de ser considerado válido no dia em que o certificado expira. Com
carimbo, o que foi assinado durante a validade continua confiável para sempre.

### Verificar se funcionou

```bash
Get-AuthenticodeSignature "src-tauri\target\release\bundle\nsis\Otimiza_0.2.0_x64-setup.exe" | Format-List Status, SignerCertificate
```

`Status` precisa ser `Valid`. Qualquer outra coisa significa que o instalador
seguirá mostrando o aviso de editor desconhecido.

## Nunca versione o certificado

O `.gitignore` da raiz já bloqueia `*.pfx`, `*.p12` e `*.snk`. Uma chave privada
de assinatura vazada permite que qualquer pessoa publique malware assinado com o
seu nome — o estrago passa longe de perder o certificado.

## Enquanto não houver assinatura

Vale avisar o cliente antes da instalação: "o Windows vai mostrar um aviso de
editor desconhecido, clique em Mais informações e depois em Executar assim
mesmo". Avisar antes transforma um susto em um passo esperado — e é mais honesto
que deixar a pessoa descobrir sozinha.
