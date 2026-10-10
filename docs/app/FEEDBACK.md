# Pedido de feedback (3.3)

Quem tem licença usa o Otimiza por **10 minutos**. Depois disso as **correções** pausam até existir um feedback
dessa pessoa no canal de feedbacks (`1543436859529629707`). Medir, ver o diagnóstico e **desfazer** continuam livres.
Se a mensagem for apagada, as correções pausam de novo na próxima conferência.

## Como o programa sabe

O programa não lê o Discord (um token de bot dentro do instalador vazaria). Quem lê é o **qrbot**, que responde:

```
GET {api}/v1/feedback/{OTZ-XXXX-XXXX-XXXX}
200  corpo = atestado assinado (texto puro)
404  nenhum feedback desta máquina
```

O atestado sai de `atestarFeedback()` em `pc-optimizer/bot/otimiza-licenca.cjs`, com a **mesma chave privada** da
licença. O programa confere a assinatura, o `tipo: "feedback"`, o código da máquina e a data (no máximo 7 dias).

| Quando o programa pergunta | |
|---|---|
| O cliente clica em "Já mandei" | sempre |
| Na abertura, depois dos 10 min | no máximo 1× por dia |
| Sem rede | o último atestado bom vale 7 dias |

## O que publicar em `.github/convite.json`

```json
{
  "discord": "https://discord.gg/ultimus",
  "api": "https://SEU-BOT.squareweb.app",
  "canal_feedback": "https://discord.com/channels/<ID DO SERVIDOR>/1543436859529629707"
}
```

**Sem `api` o pedido fica desligado.** Ninguém é trancado enquanto o bot não estiver respondendo. Publique a `api`
só depois de testar o endpoint.

## No bot (feito em 10/10/2026)

O bot de vendas já responde. Arquivos no projeto do bot:

- `src/services/feedbackService.js` — valida a mensagem (30 caracteres e um print), acha a máquina pela compra
  entregue de quem postou ou pelo código OTZ escrito na mensagem (venda do site), e assina o atestado.
- `src/events/feedbackCanal.js` e `src/events/feedbackApagado.js` — o canal `FEEDBACKS_CANAL_ID` e a mensagem
  apagada.
- `src/services/apiDoSite.js` — `GET /v1/feedback/<máquina>` (texto puro; 404 sem feedback; 30 pedidos por
  minuto por endereço).
- `src/database/schema.js` — migração 21, tabela `feedbacks` (só identificadores e códigos OTZ).

A cada pergunta o bot confere no Discord se a mensagem ainda existe: apagada (código 10008), o atestado sai com
`ok: false`. Discord fora do ar não tranca ninguém (vale o que estava gravado). Testes: `npm test` no bot (seção
"Feedback do Otimiza") e, no Otimiza, `o_atestado_do_bot_de_verdade_confere_com_a_publica_de_producao` (atestado
emitido com a privada real do bot).

**Precisa da intent Message Content:** ligada no portal do Discord (Bot → Privileged Gateway Intents) e
`INTENT_MENSAGENS=true` no `.env`, nessa ordem (ao contrário o Discord recusa a conexão). Sem ela a mensagem chega
vazia e o bot responde que ainda não consegue ler o canal, em vez de acusar o cliente.

**Verificado em produção em 10/10/2026** (`https://otimiza-api.squareweb.app`, pacote `2026-10-10.3-feedback`, que a
rota `/v1/saude` informa): 404 antes do feedback; depois de um feedback real no canal, 200 com atestado `ok: true`
que confere com a pública do Otimiza; mensagem apagada, `ok: false`.

**Para ligar nos clientes:** publicar a `api` em `.github/convite.json` só junto com a 3.3 (a 3.2.0 não conhece a
chave do bot).

## Onde está no código

- `pc-optimizer/src-tauri/src/plataforma/feedback.rs` — regra, contador, conferência, rechecagem
- `pc-optimizer/src-tauri/src/plataforma/feedback_prova.rs` — provas com par de chaves sorteado
- `pc-optimizer/src-tauri/src/modules/licenca.rs` — `exigir()` passa a consultar o feedback
- `pc-optimizer/src-tauri/src/modules/convite.rs` — lê `api` e `canal_feedback`
- Comandos: `feedback_estado`, `feedback_verificar`; evento `feedback:situacao`
