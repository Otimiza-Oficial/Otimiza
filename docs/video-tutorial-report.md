# Video tutorial do dono — onde entrou e por que

Video: https://youtu.be/6bmxhhEJGoo (o `?si=...` que o dono mandou junto e so
o token de rastreio do compartilhamento do YouTube; usei a URL limpa).

## A. Bot (qrbot, branch central-de-atendimento)

Commit: `2ddfb41` — "Adiciona o video tutorial do dono na mensagem de entrega"

Arquivos: `src/config/index.js`, `.env.example`, `src/services/entregaService.js`,
`scripts/smoke-test.js`.

- `config.otimiza.tutorial` foi adicionado do mesmo jeito que `config.otimiza.download`:
  le `OTIMIZA_TUTORIAL_URL`, cai no video acima como padrao, e passa pela mesma
  validacao de URL (`/^https?:\/\/\S+$/`) que ja existia para o download —
  recusa o boot se a variavel vier preenchida com algo que nao e endereco.
- `.env.example` ganhou a variavel comentada, sem valor (o arquivo e versionado
  e a checagem `conferirMolde()` so vigia segredo, entao isso e seguro).
- `mensagemDaEntrega` agora sempre acrescenta um bloco `**Tutorial**` com o
  link, no fim da mensagem, respeitando a estrutura de linhas + separadores
  em branco que o arquivo ja usava.

  **Decisao: o link vai nos dois casos — com chave e no pack sozinho.**
  Ambos abrem o Otimiza pela primeira vez sem nunca ter visto a tela; o video
  ensina a usar o produto, e "usar o produto" nao depende de qual parte foi
  comprada. Por isso o bloco fica fora dos `if (temChave)` / `if (...PACK...)`
  e e adicionado incondicionalmente, como apoio, depois do que foi pago.

  **Venda externa:** o bloco entra tambem quando `compra.origem === 'externa'`.
  Quem recebe a mensagem colada no WhatsApp e o cliente de verdade — a mesma
  pessoa que abriria um atendimento se ficasse perdida —, entao nao ha razao
  para omitir. A mensagem continua lendo bem colada: o aviso "VENDA EXTERNA —
  cole no WhatsApp" fica so no topo, para o dono, e o resto (chave/pack/tutorial)
  e exatamente o que o cliente precisa ler.

- Testes novos (canario): "A entrega com chave leva o link do tutorial" e
  "O pack sozinho tambem leva o link do tutorial" — ambos comparam
  `texto.includes(config.otimiza.tutorial)`. Revertendo o bloco adicionado em
  `mensagemDaEntrega`, as duas asserções falham (a string procurada some do
  texto), confirmando que os testes checam o comportamento novo e nao algo
  que ja era verdade antes.

Suite: `node scripts/smoke-test.js` → **168 testes passaram** (165 anteriores
+ 3 novos).

## B. App (Otimiza, branch main)

Commit: `2ec774b` — "Adiciona link permanente para o video tutorial no rodape do console"

Arquivos: `pc-optimizer/index.html`, `pc-optimizer/src/main.ts`,
`pc-optimizer/src/styles.css`.

- **Onde:** rodape do console (`.statusbar`, o `<footer>` que fica visivel
  em qualquer aba, ao lado da marca "Otimiza · tudo roda no seu PC"). Nao
  criei painel novo: o app so tinha UM lugar que ja linkava para fora — o
  convite do Discord na tela de ativacao (`#portao-discord`), que so aparece
  uma vez, antes da compra. O rodape e o analogo para depois da compra: e a
  unica faixa que persiste em toda navegacao, entao e onde um link "para
  sempre" precisa morar para o cliente confuso tres semanas depois encontrar.
- **Como o app abre link externo:** ja existia o padrao — `<a target="_blank"
  rel="noreferrer">` mais a permissao `opener:default` (plugin
  `@tauri-apps/plugin-opener`), ja concedida em
  `pc-optimizer/src-tauri/capabilities/default.json`. O Tauri 2 intercepta o
  clique em link com `target="_blank"` quando esse plugin/permissao esta
  presente e abre no navegador padrao do sistema, sem passar por dentro do
  webview. Segui exatamente o mesmo padrao do link do Discord — **nao precisei
  mudar nenhuma capability**, porque a permissao necessaria ja estava la.
- O `href` do novo link (`#statusbar-tutorial`) e preenchido em tempo de
  execucao dentro de `wireControls()`, a partir da constante `TUTORIAL_URL`
  definida perto de `CONVITE_DISCORD` em `main.ts` — mesmo padrao que o
  Discord ja usava (constante hardcoded, com comentario explicando o porque
  de nao vir de configuracao).

Suites:
- `cargo test --lib` (a partir de `pc-optimizer/src-tauri`) → **495 passaram**,
  11 ignorados (testes marcados como so rodando manualmente nesta maquina),
  0 falharam. Nao toquei em nenhum modulo Rust; a mudanca e so HTML/CSS/TS.
- `npx tsc --noEmit` (a partir de `pc-optimizer`) → **sem erros**.

Nao ha suite de teste automatizado para o TypeScript/DOM neste repositorio
alem do `tsc`; a mudanca de UI (link estatico com `href` preenchido em JS) foi
conferida por leitura e pela checagem de tipos, que passou.

`pc-optimizer/src-tauri/Cargo.lock` mudou como efeito colateral de rodar
`cargo test` (sincronizou a versao do lockfile com a do `Cargo.toml`, de
1.1.2 para 1.3.0) — revertido antes do commit por ser algo fora do escopo
deste pedido.
