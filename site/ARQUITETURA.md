# O site do Otimiza — arquitetura e segurança

Este documento existe porque a combinação pedida (site estático no GitHub Pages
+ login + Mercado Pago + emissão de licença) **não fecha sozinha**, e o motivo
não é preferência de ninguém: é uma restrição dura.

---

## A restrição que decide tudo

A licença do Otimiza é assinada com uma **chave privada Ed25519**. O produto
carrega só a metade pública, que apenas *confere*. É esse desenho que impede um
gerador de chaves pirata — está explicado em `pc-optimizer/docs/LICENCA.md`.

**Se a privada for parar no site estático, ela vai junto com o JavaScript para o
navegador de todo visitante.** Qualquer pessoa abre o DevTools, copia a chave e
emite licença infinita. Não existe ofuscação que resolva: código entregue ao
navegador é código que o usuário possui.

> **A privada só pode existir num servidor que você controla. Isso não é
> negociável em nenhuma arquitetura.**

O mesmo vale para o *access token* do Mercado Pago e para os *client secrets* do
OAuth. Nenhum dos três pode aparecer no repositório do site.

## O que isso significa na prática

GitHub Pages serve arquivo estático. Ele não roda código. Então:

| Parte | Onde vive | Pode ser estático? |
|---|---|---|
| Landing (hero, pilares, preços, FAQ) | GitHub Pages | **Sim** |
| Botão "Comprar" | GitHub Pages | **Sim** |
| Login Discord/Google | Servidor | Não |
| Cobrança no Mercado Pago | Servidor | Não |
| Webhook de confirmação | Servidor | Não |
| **Emissão da chave** | Servidor | **Nunca** |

A boa notícia: isso muda pouco do plano. A landing continua estática e rápida no
Pages. Só o checkout mora em outro lugar — num subdomínio, por exemplo
`comprar.otimiza.app`, ou numa rota servida por Vercel/Cloudflare Workers.

---

## O fluxo completo

```
  LANDING (estática, GitHub Pages)
      │  clique em "Comprar"
      ▼
  CHECKOUT (servidor)
      │
      ├─ 1. Login  ──────────  Discord OAuth2 (PKCE)  ou  Google OIDC
      │                        Só para saber quem comprou e onde entregar.
      │
      ├─ 2. Código da máquina   O cliente cola o OTZ-XXXX-XXXX-XXXX que o
      │                        Otimiza mostra na tela dele.
      │                        VALIDAR O FORMATO AQUI, antes de cobrar.
      │
      ├─ 3. Preferência de pagamento  ──►  API do Mercado Pago
      │                        Pix (QR), cartão de crédito, boleto.
      │
      ├─ 4. Cliente paga  ────────────►  ambiente do Mercado Pago
      │                        Dado de cartão NUNCA passa pelo nosso servidor.
      │
      ├─ 5. Webhook do MP  ◄──────────  confirma pagamento
      │                        Validar assinatura. Reconsultar a API do MP.
      │                        Nunca confiar no corpo do webhook sozinho.
      │
      ├─ 6. EMITIR A LICENÇA   Assina com a privada. Só aqui.
      │                        Reusa pc-optimizer/bot/otimiza-licenca.cjs —
      │                        o mesmo emissor do bot, mesmo formato, mesmo
      │                        teste do lado Rust que garante que abre.
      │
      └─ 7. Entrega            Tela + e-mail + DM no Discord (se logou por lá).
```

O passo 6 usar **o mesmo arquivo** que o bot já usa não é economia de código: é
o que garante que a chave vendida pelo site e a chave vendida pelo Discord têm
exatamente o mesmo formato. Existe um teste no lado Rust
(`a_chave_emitida_pelo_bot_abre_o_produto`) que quebra o build se os dois
divergirem.

---

## Segurança — o que precisa estar certo

O cliente pediu "alta segurança porque estaremos lidando com dados sigilosos".
Concordo, e a melhor decisão de segurança aqui é **não guardar quase nada**.

### Dado de cartão: nunca toque nele

Use **Checkout Pro** ou os **Bricks** do Mercado Pago. Nos dois, o número do
cartão vai do navegador do cliente direto para o Mercado Pago — o nosso servidor
recebe só um identificador de pagamento.

Isso tira o projeto de quase todo o escopo de PCI-DSS. Montar formulário próprio
de cartão significa herdar a obrigação de proteger número de cartão, e não há
motivo nenhum para aceitar esse risco.

### O que guardar, e só isso

| Dado | Por quê | Retenção |
|---|---|---|
| E-mail | Entregar a chave e dar suporte | Enquanto a licença existir |
| ID do Discord ou do Google | Reemitir sem o cliente provar quem é | Idem |
| Código da máquina (`OTZ-…`) | Reemitir depois de troca de placa | Idem |
| ID do pagamento no MP | Conferir e conciliar | Obrigação fiscal |
| A chave emitida | Reenviar sem cobrar de novo | Idem |

**Nada de:** número de cartão, CVV, CPF (a não ser que a nota fiscal exija),
endereço, telefone. Dado que não é coletado não vaza.

### Checklist do servidor

- [ ] Privada, token do MP e secrets do OAuth em variável de ambiente ou cofre —
      **nunca** no repositório. `.env` no `.gitignore` desde o primeiro commit
- [ ] Webhook do Mercado Pago com **validação de assinatura** (`x-signature`)
- [ ] Depois do webhook, **reconsultar a API do MP** pelo id do pagamento.
      Webhook pode ser forjado; a resposta autenticada da API, não
- [ ] **Idempotência**: um pagamento gera uma chave. Reprocessar o mesmo webhook
      não pode emitir de novo nem cobrar de novo
- [ ] OAuth com **PKCE** e parâmetro `state` conferido (contra CSRF)
- [ ] Token de OAuth **só no servidor**, em cookie `HttpOnly` `Secure`
      `SameSite=Lax`. Nunca em `localStorage`
- [ ] **Rate limit** na rota que emite chave e na que cria pagamento
- [ ] Validar o formato `OTZ-XXXX-XXXX-XXXX` **antes** de cobrar — o pior caso
      possível é o cliente pagar e receber uma chave que não abre
- [ ] HTTPS com HSTS. CSP restritiva na página de checkout
- [ ] Log de emissão (quem, quando, qual máquina) — sem gravar a privada
- [ ] Backup do banco. Perder o registro de quem comprou é perder o suporte

### LGPD e direito do consumidor

Não sou advogado, e isto precisa de revisão de quem é. Mas dois pontos são
conhecidos o bastante para já entrarem no planejamento:

- **Política de privacidade e termos de uso** precisam existir e estar linkados
  no rodapé antes da primeira venda
- **CDC, art. 49**: compra pela internet dá ao consumidor **7 dias de direito de
  arrependimento**, contados do recebimento. Isso é lei, não política comercial —
  a página de preço não pode dizer "sem reembolso"

---

## Onde hospedar o checkout

A landing fica no GitHub Pages como você escolheu. Para o checkout:

| Opção | A favor | Contra |
|---|---|---|
| **Vercel** (Functions + Neon/Supabase) | Grátis para este volume, deploy por push, fácil | Mais uma conta |
| **Cloudflare Workers + D1** | Rápido no Brasil, barato | Runtime tem particularidades |
| **Junto com o bot do Discord** | A privada já está lá. Um segredo, um lugar | Se o bot cair, a venda cai |

**Recomendo a terceira**, se o bot já roda numa VPS: a chave privada já vive lá,
e espalhar segredo por mais um servidor é aumentar a superfície de risco sem
ganhar nada. O site chama a API do bot; o bot continua sendo o único lugar do
mundo onde a privada existe.

---

## O que já foi resolvido desde que este documento foi escrito

- **A chave privada Ed25519 não estava perdida.** Ela está no `.env` do bot, e
  foi usada para emitir as duas chaves de dono. Este documento afirmou o
  contrário por um tempo, escrito antes de ela ser encontrada — e afirmar sem
  ter conferido é exatamente o defeito que este projeto cobra dos outros.
- **O preço:** R$ 20, o mesmo que o bot já cobra.
- **O checkout foi construído** e está desligado até existir onde ligar. Ver o
  commit `e6059c7`.
- **Não há OAuth.** O desenho abaixo previa login por Discord ou Google; o que
  foi construído não tem login nenhum. A credencial é um token aleatório por
  compra, e o dado pessoal coletado é exatamente nenhum — o que é melhor do que
  este documento planejava.

## O que ainda falta

1. **Onde o bot roda 24 horas.** Hoje ele roda no PC do dono, e o
   `docs/DOCKER.md` do próprio bot é franco: *"Docker não é 24 horas"*. Um
   checkout que só atende com aquele PC ligado é pior que nenhum — no Discord
   quem chega de madrugada espera o atendimento acordar; no site ele clica,
   nada acontece, e vai embora
2. **HTTPS na frente da API.** Sem TLS, o token da compra e a chave de licença
   viajam em texto puro
3. **`PUBLIC_OTIMIZA_API` na esteira do site**, para o checkout deixar de ser
   invisível na compilação de produção
4. **Domínio próprio** — dá para viver sem, mas checkout em
   `usuario.github.io/Otimiza` derruba a confiança na hora de pagar
5. **Política de privacidade e termos de uso**, linkados no rodapé antes da
   primeira venda
