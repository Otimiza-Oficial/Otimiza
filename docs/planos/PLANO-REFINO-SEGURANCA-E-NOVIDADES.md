# Plano — Refino, segurança e novidades

> Três frentes: deixar a interface no nível de produto grande, fechar as portas por onde o Otimiza vaza, e trazer as
> APIs e tecnologias que ainda não usamos. Escrito em 10/11/2026, depois da Onda 2, do "Jogar", do cartão da partida e
> da apresentação nova. Complementa o `PLANO-1.0-NOVA-ERA.md` e o `PLANO-PARTIDA-DE-RESPEITO.md`.

---

## 0. O que foi visto hoje e motiva este plano

| Achado | Onde | Por que importa |
|---|---|---|
| O repositório `Otimiza-Oficial/Otimiza` é **público** | GitHub | O código inteiro, inclusive a conferência da licença, está aberto: dá para compilar uma cópia sem a trava |
| O atestado online só liga com `OTIMIZA_API_LICENCA` na compilação | `modules/licenca_online.rs` | Instalador gerado fora da esteira sai sem revogação e sem ver chave repetida |
| `EXIGE_ATESTADO = false` | `modules/licenca_online.rs:12` | Hoje a chave offline sozinha abre tudo, para sempre |
| `"csp": null` | `src-tauri/tauri.conf.json` | A janela aceita script de qualquer origem; um XSS viraria acesso aos comandos do Rust |
| Instalador sem assinatura | `bundle` | Aviso azul do SmartScreen e nenhuma prova de que o arquivo é nosso |
| 115 comandos com `invoke` e tipo à mão | `ui/testes/invoke-solto.json` | Tipo errado já quebrou o Histórico com dado real |
| Três sessões terminaram sem fechamento normal | `otimiza.log`, `sessao.json` | Sem causa achada; o log de cada passo do fechamento entrou hoje |
| Regra de layout antiga escondia cards abaixo de 1100 px | `App.svelte` (corrigido) | Visto num cliente; a demonstração não mostrava |
| A chave de exemplo do campo era o formato do código da máquina | Ajustes (corrigido) | Um sócio não conseguiu ativar |

---

## 1. Segurança — o Otimiza não pode mais vazar

Regra honesta antes de tudo: **nada que roda no PC do cliente é inquebrável**. Quem tem o executável e tempo tira
qualquer conferência. O objetivo é outro: tornar a cópia pirata **cara de fazer, fácil de descobrir e pior de usar**
que a original, e não deixar a porta aberta de graça, como está hoje.

### 1.1 Fechar a porta que está aberta (urgente)

1. **Repositório privado.** Passar `Otimiza-Oficial/Otimiza` para privado, ou dividir:
   - público: site, documentação, o que é vitrine;
   - privado: `pc-optimizer/` inteiro (Rust, interface, bot).
   O que já foi clonado não volta, mas cada versão nova deixa de sair de graça. **Decisão do dono** (seção 6).
2. **Limpar o histórico de segredo.** O próprio código registra duas chaves privadas que vazaram (`.env.example` e uma
   captura de tela). Já foram trocadas, mas o histórico público ainda tem as antigas. Ao fechar o repositório, revisar
   com `gitleaks` e reescrever o histórico se ainda houver segredo.
3. **Build só pela esteira.** Instalador de cliente sai só do GitHub Actions, com `OTIMIZA_API_LICENCA` e a assinatura.
   Compilação local ganha uma marca ("compilação de desenvolvimento") na janela e no relatório de suporte, para nunca ir
   para cliente por engano, como os instaladores de hoje.

### 1.2 A licença que o servidor confirma

O atestado online já existe (`licenca_online.rs`): o bot devolve, com prazo curto, uma confirmação assinada para a
chave e esta máquina. Falta ligar de verdade, em etapas:

| Etapa | O que muda | Para o cliente |
|---|---|---|
| A | Toda versão publicada renova o atestado (hoje só se a variável existir) | Nada muda |
| B | O servidor marca chave vista em várias máquinas e chave revogada | Nada muda para quem pagou |
| C | `EXIGE_ATESTADO = true`, com tolerância de 30 dias sem rede | Quem joga offline por um mês continua |
| D | O atestado leva o **hash do executável publicado**: build alterado não recebe atestado | O pirata fica trancado depois de 30 dias |

Junto:
- **A conferência espalhada**: hoje é uma função (`licenca::exigir`). Passar a conferir também dentro dos módulos de
  valor (Turbo, Frame Boost, receita), com o atestado, para quem remove uma chamada não abrir tudo.
- **Cada chave carrega o comprador** (já carrega). Chave vazada no Discord mostra de quem veio, e o bot revoga.
- **O que dá valor mora no servidor**: receitas por jogo e perfis assinados, a inteligência coletiva ("PCs parecidos")
  e as atualizações. A cópia pirata fica com uma versão velha e sem as receitas novas.

### 1.3 Assinatura e atualização

1. **Assinatura de código (Authenticode).** O caminho mais barato hoje é o **Azure Trusted Signing**, cobrado por mês e
   sem token físico. Some o aviso do SmartScreen, e o atestado passa a conferir que o executável é o assinado.
2. **Atualizador assinado** (`tauri-plugin-updater`, assinatura minisign). O Otimiza só aceita atualização assinada
   com a nossa chave: um instalador adulterado no meio do caminho não entra.

### 1.4 A janela e os comandos

1. **Ligar a CSP** no `tauri.conf.json`: só os scripts do próprio app, imagens da CDN da Steam e das lojas dos jogos,
   e conexões só para a API do bot. Testar com o `fumaca:nova`.
2. **Capacidades mínimas**: revisar `capabilities/default.json` para cada janela (principal, sobreposição, mira) ter só
   os comandos de que precisa. A sobreposição não precisa de "aplicar ajuste".
3. **Sem ferramentas de desenvolvedor na versão publicada**, conferido por teste no binário de release.
4. A licença já é conferida no Rust e não na tela (`licenca::exigir`); manter a guarda que impede comando de ajuste sem
   classificação de licença (`commands/mod.rs`).

### 1.5 O bot e o servidor

- **A chave privada num cofre** (o segredo da hospedagem), nunca em arquivo de texto. A terceira chave se perdeu com a
  hospedagem antiga: backup cifrado num segundo lugar.
- **Limite de pedidos** no `/atestado` e na emissão, e registro de quem pediu o quê.
- **Roteiro de troca de chave escrito**, testado uma vez, para o dia em que vazar de novo.
- Emissão só por mensagem privada do bot, nunca em canal público.

### 1.6 Cadeia de dependências

- `cargo deny` e `cargo audit` na esteira (licenças e falhas conhecidas), `npm audit --omit=dev` na interface.
- Versões travadas e revisadas antes de subir; lista de dependências (SBOM) publicada com cada versão.
- `gitleaks` na esteira, para nenhum segredo entrar num commit de novo.

### 1.7 Pronto quando

- O repositório do programa é privado e o histórico não tem segredo.
- Instalador publicado é assinado, renova o atestado e recusa atualização sem a nossa assinatura.
- Um executável alterado (teste com um byte trocado) deixa de receber atestado e tranca em 30 dias.
- A CSP está ligada e o `fumaca:nova` passa.

---

## 2. Refino da interface

### 2.1 Fundação

1. **Pagar a dívida de tipos.** Os 115 comandos de `ui/testes/invoke-solto.json` passam para o contrato gerado, área
   por área, começando pelas que mostram número ao cliente (Histórico, Prova, Energia). O teste já impede comando novo
   fora do contrato.
2. **Dados reais nos testes.** Gravar respostas de verdade do Rust (NVIDIA, AMD, sem jogo, sem licença) e rodar o
   `fumaca:nova` com elas além da demonstração. A demonstração escondeu dois defeitos hoje.
3. **Tokens únicos.** Hoje a casca usa `--texto`/`--sec` e as áreas `--tinta`/`--tinta-2`, com mapeamentos que deram
   azul-marinho no escuro. Uma camada só de tokens, com claro, escuro e "sem fundo" definidos num lugar.
4. **Componentes de base**: o deslizante fino (`.deslizante`, entrou hoje) em todas as áreas; um `Interruptor`, um
   `Segmento`, uma `Ficha` de estado ("No monitor agora" / "Só na prévia") e um `Cartao` iguais em todo lugar.

### 2.2 Estrutura

1. **4 abas no lugar de 13 áreas**: Jogar, Meu PC, Prova, Ajustes (proposta do plano da Partida, seção 5.1).
2. **Topo de área com contexto**: em vez de só título, o que a área já sabe ("Monitor em 60 Hz, aceita 180") e a ação
   principal dela.
3. **Estados vazios com caminho**: toda área sem dado diz o que fazer para ter ("Jogue 3 minutos com o Otimiza aberto").
4. **Carregamento com esqueleto** do conteúdo, nunca "Lendo…" solto.
5. **Janela estreita e escala de 125%/150%** testadas no `fumaca:nova` (1000 px já entrou; falta 1366×768 a 125%).

### 2.3 Acabamento visual

1. **Movimento com sistema**: uma escala de durações e curvas (entrar, sair, destacar) usada por todas as áreas, e tudo
   parado com jogo aberto e com "reduzir movimento".
2. **Tipografia**: três tamanhos de texto e dois de número em toda área; números sempre tabulares.
3. **Gráficos de verdade**: o tempo de quadro da partida (o da apresentação é ilustração) com os dados do PresentMon,
   antes e depois sobrepostos.
4. **Revisão do escuro e do "sem fundo"** em todas as áreas, com print automático das 20 telas nos três modos.
5. **Ícone e mascote**: o Oti reagindo ao último cartão da partida na bandeja e na tela inicial.

### 2.4 Pronto quando

- `invoke-solto.json` vazio.
- As 20 telas passam no `fumaca:nova` com dados reais, nos modos claro, escuro e sem fundo, em 1000 px e em 1366 px.
- Um cliente novo acha "Jogar" e "Ativar licença" sem ajuda, em teste com 5 pessoas.

---

## 3. Tecnologias e APIs novas

Só entra o que é oficial, documentado e não toca no processo do jogo.

### 3.1 Medição

| O quê | Para quê |
|---|---|
| **PresentMon 2 / PresentMon Service** (Intel, aberto) | Medição contínua sem abrir um processo por janela, com tempo de exibição e latência de clique |
| **ETW direto** (`Microsoft-Windows-DxgKrnl`, `Microsoft-Windows-Win32k`) | Saber o modo de apresentação (flip, cópia) e o MPO sem depender de outro programa |
| **NVML, ADLX e IGCL completos** | Temperatura, clock e motivo do freio também nas placas AMD e Intel (hoje só NVIDIA tem a guarda) |

### 3.2 Ajuste

| O quê | Para quê |
|---|---|
| **ADLX: Radeon Chill, Boost, Anti-Lag, Image Sharpening** | A AMD hoje tem menos ajustes que a NVIDIA no Otimiza |
| **IGCL: modo de baixa latência e nitidez nas Arc** | O mesmo para Intel |
| **Otimizações para jogos em janela** (`DirectXUserGlobalSettings`, `SwapEffectUpgrade`) | Menos atraso e tranco em janela sem bordas, como experimento medido |
| **Auto HDR e HDR por jogo** (configuração do Windows) | Ligar só nos jogos em que ajuda |
| **Agendamento de GPU por hardware (HAGS)** com prova | Hoje ele só aparece na lista de ajustes; passar a medir antes e depois por máquina |

### 3.3 Sistema e plataforma

| O quê | Para quê |
|---|---|
| **Notificações do Windows** (`tauri-plugin-notification`) | O cartão da partida chega mesmo com o Otimiza na bandeja |
| **Widget da Xbox Game Bar** | FPS e avisos dentro do jogo onde o anticheat não deixa janela por cima |
| **Manifestos da Epic e da Riot** | O "Jogar" abrir pela loja certa (hoje a Epic sem executável não abre) |
| **Opções de lançamento da Steam** (`localconfig.vdf`) | Mostrar e sugerir opções que rendem, sem escrever sem a pessoa pedir |
| **Relatório de falha com minidump**, opcional | Achar a causa de sessões como as três de hoje que fecharam sem registro |
| **Atualizador assinado** (seção 1.3) | Atualizar com um toque, só com instalador nosso |

### 3.4 Inteligência

- **Receitas pela comunidade**: o servidor agrega, com opt-in, o que rendeu em máquinas parecidas e devolve receitas
  assinadas. É também a melhor defesa contra pirataria (seção 1.2).
- **Oti com IA** recebendo o cartão da partida e o mapa de 40 s, para responder "por que meu jogo travou ontem?" com os
  números certos. A regra continua: só número medido, nunca age sozinho.
- **Modelo local em NPU** (Windows ML / DirectML): só depois, e só se rodar sem pesar no jogo.

---

## 4. Ajustes e consertos pequenos que ficaram

1. Investigar as sessões que terminam sem fechamento normal (log de cada passo entrou hoje).
2. O "Jogar" para a Epic sem executável e para o Roblox sem jogo escolhido.
3. O MSI em inglês (`en-US`): gerar em português ou publicar só o NSIS.
4. A primeira vez sem receita do Turbo: medir e montar na própria partida, em vez de só oferecer o Turbo.
5. Abertura em 1,9 s no debug: medir a versão final e cortar o que roda antes da janela.
6. Placeholder e textos de toda a interface revisados com as regras do `ui/DESIGN.md`.

---

## 5. Ordem sugerida

| Fase | O que entra | Por quê primeiro |
|---|---|---|
| **1** | Repositório privado, build só pela esteira, CSP, `gitleaks` (1.1, 1.4, 1.6) | Fecha o vazamento de graça; nenhuma linha de produto muda |
| **2** | Assinatura e atualizador assinado (1.3) | Some o aviso azul e prepara o atestado com hash |
| **3** | Atestado etapas A e B, e as receitas assinadas pelo servidor (1.2, 3.4) | O pirata passa a ficar para trás |
| **4** | Dívida de tipos, dados reais, tokens únicos e componentes (2.1) | Base para o refino não quebrar |
| **5** | 4 abas, estados vazios, esqueleto, movimento, gráficos reais (2.2, 2.3) | O salto visual |
| **6** | APIs: notificações, ADLX/IGCL, PresentMon 2, Game Bar (3.1–3.3) | Mais ajuste e medição para AMD e Intel |
| **7** | Atestado etapas C e D (1.2) | Só depois de a etapa A mostrar que os clientes renovam |

---

## 6. Decisões que são do dono

1. **Repositório privado** (tudo) ou **dividido** (site público, programa privado)?
2. **Assinatura**: contratar o Azure Trusted Signing (mensal) ou um certificado tradicional?
3. **Atestado obrigatório**: aceitar a tolerância de 30 dias sem rede?
4. **Receitas pela comunidade**: coletar, com opt-in, o resultado das partidas para montar as receitas?
