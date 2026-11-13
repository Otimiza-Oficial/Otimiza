# Plano — Refino, segurança e novidades

> Cinco frentes: deixar a interface no nível de produto grande, fechar as portas por onde o Otimiza vaza, trazer as
> APIs e tecnologias que ainda não usamos, deixar mais fortes os sistemas que já existem e garantir que nenhuma
> otimização trave o PC. Escrito em 10/11/2026, depois da Onda 2, do "Jogar", do cartão da partida e
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
   O que já foi clonado não volta, mas cada versão nova deixa de sair de graça. **Decisão do dono** (seção 8).
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

## 4. Sistemas atuais: mais fortes

O que já existe, e o próximo passo de cada um. "Apelão" aqui quer dizer mais ganho sentido e menos artefato, sempre
medido: nenhum item entra sem prova antes e depois numa máquina real.

### 4.1 Frame Boost

Hoje: gerador por fora do jogo (captura da área de trabalho, movimento estimado na placa em blocos, mediana e
interpolação; `modules/windows/geracao/`), escala FSR 1, Auto que enche o monitor, base travada numa fração do monitor
(`base_do_jogo.rs`), guarda que desliga se o jogo perde FPS (`guarda.rs`), modo leve e aviso de placa integrada.

| # | Melhoria | O que muda para quem joga |
|---|---|---|
| FB1 | **Fluxo óptico da AMD FidelityFX** (Optical Flow e Frame Interpolation do FidelityFX SDK, licença MIT, compute shaders que rodam em qualquer placa DX12) no lugar do casamento de blocos próprio | Menos borda tremendo, menos "fantasma" em volta de personagem e carro em movimento |
| FB2 | **HUD protegido**: as regiões que não mudam entre quadros reais (mapa, mira, FPS, chat) não são interpoladas, saem do quadro real | Acaba o HUD tremido, o artefato que mais denuncia gerador por fora |
| FB3 | **FPS alvo**: a pessoa escolhe 144 e o multiplicador vira fracionário (2,4×), em vez de 2× ou 3× fixos | A tela chega exatamente no monitor sem forçar a base |
| FB4 | **Segunda placa**: em PC com vídeo integrado e placa dedicada (todo Intel com UHD, Ryzen com gráfico), o gerador roda na integrada | O jogo fica com a placa inteira; o custo da geração some do FPS real |
| FB5 | **Latência medida, não estimada**: o PresentMon dá o tempo até a tela; o painel mostra quanto o gerador acrescentou de verdade | O jogador competitivo decide com número |
| FB6 | **Nitidez escolhível na escala**: FSR 1 com RCAS ajustável e NVIDIA Image Scaling (NIS, licença MIT) como opção, mais escala inteira para jogo em pixel | Imagem ampliada mais nítida, do jeito que cada um gosta |
| FB7 | **HDR e VRR**: gerar em 10 bits com o jogo em HDR, e respeitar o G-Sync/FreeSync com os quadros gerados | Hoje o HDR é um aviso; passa a funcionar |
| FB8 | **Perfil por jogo aprendido**: o multiplicador, a escala e a base que a guarda aprovou em cada jogo voltam sozinhos no "Jogar" | Um toque, e já no melhor que esta máquina provou |

Fora: injetar no jogo para pegar os vetores de movimento dele (é o que FSR 3 e DLSS fazem por dentro). Isso exige o
jogo integrar, e por fora quebraria a regra do anticheat.

### 4.2 Rede

Hoje: rota salto a salto com a operadora de cada trecho (`rota.rs`, `operadora.rs`), histórico que diz quando piora
(`rota_historico.rs`), perda e variação até o servidor do jogo (`rede.rs`), Wi-Fi lido pela API nativa (`wifi.rs`),
Wi-Fi sem varredura no jogo (`wifi_varredura.rs`) e envio de segundo plano limitado (`banda.rs`).

| # | Melhoria | O que muda para quem joga |
|---|---|---|
| R1 | **Teste de fila sob carga** (bufferbloat): ping medido parado, durante um download e durante um envio | Mostra a causa nº 1 de "ping sobe quando alguém usa a internet", com o número e a solução (SQM no roteador, limite de envio) |
| R2 | **Placa de rede com prova**: Interrupt Moderation, Energy Efficient Ethernet, controle de fluxo e economia de energia da placa, cada um testado antes e depois na variação do ping | Só fica o que melhorou nesta placa; hoje ninguém prova isso |
| R3 | **Quem está usando a banda agora**, por programa (contadores e ETW de rede) | "O OneDrive está enviando 8 Mb/s" no lugar de "sua internet está ruim" |
| R4 | **DNS medido**: os públicos (Cloudflare, Google, Quad9, o da operadora) comparados nesta conexão | Troca só se o medido for mais rápido, e desfaz |
| R5 | **Marcação de tráfego do jogo** (QoS por política do Windows, DSCP), como experimento | Em roteador que respeita a marcação, o jogo passa na frente; a tela diz que depende do roteador |
| R6 | **Wi-Fi: canal e banda recomendados** pelas redes vizinhas e pelo sinal, e a agressividade de roaming da placa | "Mude o roteador para o canal 36" com o número de vizinhos de cada canal |
| R7 | **A rede na partida**: perda e variação gravadas durante o jogo entram no cartão da partida | O cartão responde "foi a rede ou o PC?" |
| R8 | **FiveM: ping de cada servidor** da lista do jogador antes de entrar | Escolher o servidor pelo número, não pelo nome |

Fora, como hoje: "ping booster" que promete encurtar a distância até o servidor.

### 4.3 Os outros

| Sistema | Hoje | Próximo passo |
|---|---|---|
| **Turbo** | Mede 10 s, monta a receita pelo gargalo, prova sozinho na primeira vez | Montar a receita na própria partida aberta pelo "Jogar", sem a pessoa abrir o Turbo |
| **Jogo liso** | Teto do driver no FPS que o jogo segura | Ligar sozinho quando os trancos por minuto passam do limite, e o teto acompanhar a cena |
| **Energia** | Plano OTIMIZA, energia de jogo, calibração | Calibração curta automática na primeira semana, e limites de potência visíveis em notebook |
| **Latência** | DPC por driver, taxa do mouse, teste de reflexo | Do driver culpado à correção: versão certa, opção da placa ou "desligue este aparelho USB" |
| **Tela** | Hz, VRR lido do monitor | Aplicar o par VRR + teto 3 fps abaixo do Hz com um toque, e conferir o cabo pelo que o monitor aceita |
| **Processador** | Auto CPU Set, X3D | Programas de fundo para os núcleos de eficiência durante o jogo (CPU Sets), com prova |
| **Memória** | Mostra quem ocupa e fecha o que a pessoa escolhe | Avisar antes do jogo abrir, no "Jogar", e conferir o tamanho do arquivo de paginação |
| **Mira e Cores** | Prévia nova, perfil por jogo nas cores | Mira por jogo, e cores ligando sozinhas no "Jogar" |
| **Oti** | Fala pelo que mediu, IA opcional | Lembrar a última partida de cada jogo e comparar quando a pessoa perguntar |

## 5. Estabilidade: o Otimiza nunca trava o PC

Regra de produto, no mesmo nível de "nunca menos FPS": **otimização que trava, congela, dá tranco ou derruba o PC é
otimização ruim, e o Otimiza não faz.** Quem paga para o jogo ficar liso não pode ganhar uma travada de presente.

### 5.1 O que já protege hoje

- **Portão "nunca menos FPS"** (`modules/portao.rs`): com 3 partidas de cada lado, desfaz sozinho o ajuste que tirou FPS
  médio ou 1% pior.
- **Nada congela** (`modules/windows/suspend.rs`): o modo jogo congelava programas até a 1.9 e foi o que mais machucou
  cliente; uma trava no build reprova se voltar.
- **Governador sem congelar** (`governador.rs`): programa de fundo perde prioridade, nunca é fechado nem parado.
- **O que não fazemos** (`naofazemos.rs`): placebo e ajuste prejudicial recusados, com o motivo.
- **Leitura de quedas**: driver de vídeo que parou (evento 4101) e erros de hardware (`eventoshw.rs`), crashes do
  FiveM cruzados com o que o Otimiza mudou (`crashes.rs`).
- **Modo seguro do próprio Otimiza** depois de quedas seguidas dele (`utils/diagnostico.rs`).

### 5.2 Serviços do Windows: nenhum fica parado (feito em 10/11/2026)

A lista de serviços "Stopped" (compatibilidade, diagnóstico, SysMain, telemetria) é a marca das ISOs modificadas e dos
otimizadores famosos, e de onde vem travamento e recurso quebrado que ninguém liga à causa. O Otimiza não é mais quem
deixa isso:

- Os 10 itens que paravam serviço (58 serviços: telemetria, Xbox, SysMain, Bluetooth, impressora, Hyper-V, "pouco
  usados" com PcaSvc e DPS, empresa, indexação e relatório de erros) saíram de todos os níveis e da lista de itens.
  Três deles estavam no "Otimizar agora" padrão.
- Ficam só para o desfazer de quem já aplicou: quem aplicou antes vê o item e religa o serviço com um toque.
- `o_otimiza_nao_para_servico_do_windows` reprova o build se um item vivo voltar a parar serviço.
- A recusa está na lista do que não fazemos (`parar_servicos_do_windows`), com o motivo.
- No PC com Windows modificado, o Otimiza continua mostrando os serviços que a ISO tirou (`essenciais.rs`) e só religa
  se a pessoa pedir.

### 5.3 O que falta

| # | Proteção | Por quê |
|---|---|---|
| E1 | **O portão também olha tranco**: trancos por minuto, 0,1% pior e picos de tempo de quadro entram no critério, e piora forte desfaz já na primeira partida | Hoje um ajuste que mantém o FPS médio e enche de tranco passaria |
| E2 | **Vigia de travamento do sistema** depois de cada ajuste: desligamento inesperado (Kernel-Power 41), tela azul (BugCheck 1001), driver de vídeo reiniciado (4101), programa sem responder (1002) e WHEA, cruzados com a hora da mudança | O cliente não liga "o PC travou ontem" ao ajuste de anteontem; o Otimiza liga, desfaz e avisa |
| E3 | **Ajuste que pede reiniciar é provado no boot**: aplicado, o Otimiza grava "a conferir"; se o Windows voltar com desligamento inesperado ou tela azul, desfaz sozinho na abertura seguinte | Hoje um ajuste de boot ruim depende da pessoa achar o Desfazer |
| E4 | **Cada ajuste com risco declarado** no catálogo: o que pode dar errado, em que máquina (notebook, placa antiga, Windows modificado) e qual leitura prova que deu certo | Notebook com estacionamento de núcleo desligado esquenta; placa de rede com economia desligada pode cair; isso precisa estar escrito e testado |
| E5 | **Aquecimento conferido** quando o ajuste aumenta consumo (energia, boost, núcleos sempre acordados): a guarda térmica olha os 30 minutos seguintes | Ganhar 3% e perder 20% quando esquenta é travada disfarçada |
| E6 | **Windows modificado** ("lite", "anti-lag") detectado já hoje: ali, ajustes que dependem de serviço removido não entram | Muitos travamentos vêm do sistema já mexido por outro programa |
| E7 | **Recusar o que sabidamente trava**: resolução do temporizador forçada, HPET e `useplatformclock` pelo `bcdedit`, modo MSI forçado em todo aparelho, estados de ociosidade do processador desligados, prioridade tempo real, serviços essenciais desligados | Estão entre as dicas mais copiadas e entre as que mais causam tranco e tela azul; entram na lista do que não fazemos com o motivo |
| E8 | **Ajuste novo entra em etapas**: uma versão como experimental, com o número de desfeitos pelo portão (opt-in); se muitos desfazem, um interruptor no servidor desliga o ajuste para todos | Um ajuste ruim não chega em todo cliente de uma vez |

### 5.4 O próprio Otimiza não pode pesar

| # | Regra | Como conferir |
|---|---|---|
| E9 | Com o jogo aberto, o Otimiza usa **menos de 0,5% de um núcleo** e não segura a placa de vídeo | Teste na esteira com o Modo Partida inteiro ligado (meta do plano mestre) |
| E10 | **Nenhuma rotina de interrupção (DPC) do Otimiza** acima de 100 µs durante o jogo | O Caçador de DPC do próprio Otimiza rodando contra ele mesmo |
| E11 | Medição pesada (PresentMon, mapa de 40 s, sensores) **só quando pedida ou na janela agendada**, nunca sem parar | Contagem de sessões ETW abertas com o jogo aberto |
| E12 | A interface **congela com o jogo aberto** (já existe) e o fechamento **nunca trava** | O log de cada passo do fechamento, que entrou hoje, e um teste que fecha com calibração e prova rodando |

### 5.5 Pronto quando

- Um ajuste que piora os trancos (teste com um teto de FPS errado de propósito) é desfeito sozinho na primeira partida.
- Um ajuste de boot com desligamento forçado depois é desfeito na abertura seguinte, com o aviso.
- O Otimiza passa os 30 minutos de jogo abaixo de 0,5% de um núcleo e sem DPC dele acima de 100 µs.
- A lista do que não fazemos tem os itens de E7, cada um com o motivo.

---

## 6. Ajustes e consertos pequenos que ficaram

1. Investigar as sessões que terminam sem fechamento normal (log de cada passo entrou hoje).
2. O "Jogar" para a Epic sem executável e para o Roblox sem jogo escolhido.
3. O MSI em inglês (`en-US`): gerar em português ou publicar só o NSIS.
4. A primeira vez sem receita do Turbo: medir e montar na própria partida, em vez de só oferecer o Turbo.
5. Abertura em 1,9 s no debug: medir a versão final e cortar o que roda antes da janela.
6. Placeholder e textos de toda a interface revisados com as regras do `ui/DESIGN.md`.

---

## 7. Ordem sugerida

| Fase | O que entra | Por quê primeiro |
|---|---|---|
| **1** | Estabilidade: o portão olha tranco, vigia de travamento do sistema, ajuste de boot provado, lista do que trava (5.2) | Nenhuma otimização pode travar o PC; isso vem antes de qualquer ajuste novo |
| **2** | Repositório privado, build só pela esteira, CSP, `gitleaks` (1.1, 1.4, 1.6) | Fecha o vazamento de graça; nenhuma linha de produto muda |
| **3** | Assinatura e atualizador assinado (1.3) | Some o aviso azul e prepara o atestado com hash |
| **4** | Atestado etapas A e B, e as receitas assinadas pelo servidor (1.2, 3.4) | O pirata passa a ficar para trás |
| **5** | Dívida de tipos, dados reais, tokens únicos e componentes (2.1) | Base para o refino não quebrar |
| **6** | 4 abas, estados vazios, esqueleto, movimento, gráficos reais (2.2, 2.3) | O salto visual |
| **7** | APIs: notificações, ADLX/IGCL, PresentMon 2, Game Bar (3.1–3.3) | Mais ajuste e medição para AMD e Intel |
| **8** | Frame Boost: fluxo óptico da FidelityFX, HUD protegido, FPS alvo e segunda placa (4.1) | O maior salto de "apelão" que dá para fazer por fora do jogo |
| **9** | Rede: teste de fila sob carga, placa de rede com prova, quem usa a banda (4.2) | A causa de ping ruim que mais existe e menos se mede |
| **10** | Atestado etapas C e D (1.2) | Só depois de a etapa A mostrar que os clientes renovam |

---

## 8. Decisões que são do dono

1. **Repositório privado** (tudo) ou **dividido** (site público, programa privado)?
2. **Assinatura**: contratar o Azure Trusted Signing (mensal) ou um certificado tradicional?
3. **Atestado obrigatório**: aceitar a tolerância de 30 dias sem rede?
4. **Receitas pela comunidade**: coletar, com opt-in, o resultado das partidas para montar as receitas?

---

## 9. Andamento (11/11/2026)

**Feito** (commits locais, nada publicado no GitHub):

- 5.2 Nenhum serviço do Windows fica parado.
- E1 O portão olha tranco; piora forte desfaz na primeira partida.
- E2 e E3 Vigia de estabilidade: PC que cai depois de um ajuste (inclusive o de boot) tem o ajuste desfeito na abertura seguinte.
- E5 Guarda térmica nos 30 minutos depois de um ajuste que esquenta.
- E4 Cada item vivo do catálogo com o risco escrito (o que pode dar errado, onde, e a prova: portão, estabilidade,
  térmica ou releitura) em `windows/riscos.rs`; item novo sem risco reprova o build. A tela mostra o "Pode dar errado"
  nos que mexem em FPS ou estabilidade.
- E6 Ajuste na chave de um serviço que a imagem "lite" removeu não é gravado ("não se aplica"), e o Windows modificado já é identificado.
- E7 (parte) O tick fixo do `bcdedit` aposentado.
- E8 Interruptor remoto: `ajustes_desligados` no `convite.json` desliga um ajuste para todos.
- E10 Por construção: o Otimiza não instala driver (o pacote leva só o PresentMon, programa comum), e DPC é rotina
  de driver.
- E11 Conferido: toda medição pelo PresentMon tem prazo (3 a 120 s, nas janelas agendadas ou quando pedida); o único
  contínuo é o FPS por cima do jogo, desligado por padrão e ligado pela pessoa.
- E12 O desligar do gerador espera no máximo 5 s pela linha dele: placa de vídeo presa não trava mais o fechamento
  nem o botão "Desligar".
- E9 Detecção de jogo não mede de novo a cada volta (o PowerShell da GPU saía a cada 4 s na partida inteira).
- 1.4 CSP ligada na janela; compilação local se identifica na barra e nos Ajustes.
- 1.2 Etapa A: a API pública é o endereço padrão, então toda versão renova o atestado e obedece revogação (o selo
  "compilação local" continua pela variável da esteira). Etapa B já estava no servidor (`licenca_pedidos`: IPs,
  máquinas e executáveis por chave). A licença também é conferida dentro do Frame Boost, da medida do Turbo e do Jogo
  liso, não só no comando.
- 1.6 Esteira de segredos (`gitleaks`), `cargo audit` e `npm audit`; rustls/quinn corrigidos (0 vulnerabilidades).
- R1 Ping com a internet ocupada (bufferbloat), na área Rede.
- R6 Wi-Fi: o canal com bem menos disputa (1/6/11 no 2,4 GHz, sem DFS no 5 GHz), na ficha e no conselho.
- R5 Jogo com prioridade na rede (experimento, desligado por padrão): com o jogo aberto, uma política QoS só na memória
  do Windows marca os pacotes dele com DSCP 46; sai quando o jogo fecha. A tela diz que depende do roteador.
- R3 Quem usa a internet, por programa: 5 s do provedor Kernel-Network do Windows (o mesmo do Monitor de Recursos),
  envio e recebimento por programa, e a frase aponta o envio (o que mais sobe o ping). Medido de verdade no PC do dono.
- R2 Placa de rede com prova: moderação de interrupção, EEE e controle de fluxo testados um por vez no ping (20 s antes
  e depois); fica só o que baixou a variação sem piorar perda nem ping, no histórico com desfazer; placa que não volta
  é desfeita na hora. Só com o jogo fechado e como administrador.
- R8 Servidores do FiveM: a pessoa cola os links cfx.re/join/… ou ip:porta, e o Otimiza mede o ping de cada um
  (conexão TCP na porta do jogo) e ordena; o código vira endereço pela lista pública do FiveM.
- R7 A rede na partida: ping nas janelas da medição automática; o cartão e o Oti dizem se foi a rede ou o PC.
- FB1 e FB2 já existiam: o fluxo óptico no método do AMD FidelityFX foi testado contra o fluxo denso próprio (o denso
  ficou de padrão, melhor nos quadros gravados do FiveM), e a máscara de interface protege o HUD desde a 3.3.
- FB7 (HDR) já existia: o gerador trabalha em FP16 na área de trabalho HDR.
- FB8 O Frame Boost ligado na mão que a guarda aprovou fica lembrado por jogo e volta sozinho no "Jogar" (jogo sem receita do Turbo).
- FB6 Nitidez da escala escolhível (Suave, Padrão, Forte), muda na hora sem religar.
- FB3 FPS alvo no Auto do Frame Boost: a tela chega no alvo (60 a 240, abaixo do monitor) e a base trava numa fração dele.
- 6.2 "Jogar": Epic pelo inicializador dela, Roblox pela página inicial.
- 6.3 MSI em português (`wix.language = pt-BR`).
- 2.1.1 Dívida de tipos paga: os 115 comandos que a tela chamava com `invoke` solto entraram no contrato gerado
  (`invoke-solto.json` vazio). O tipo de cada resposta continua o da área, pelo `como<T>` de `ui/src/lib/contrato.ts`.
- 2.1.3 Tokens únicos: `tokens.css` é a única fonte, com os tons neutros do painel nos dois temas (o azul-marinho
  saiu); `--texto`/`--sec`/`--contra` viraram apelidos dos mesmos tons, e a casca não redefine mais nada.
- 2.2.1 As 4 abas: o trilho mostra Jogar, Meu PC, Prova e Ajustes (com o nome sob o ícone), e a barra de baixo mostra
  as áreas da aba ativa; o "+" saiu porque toda área mora numa aba. A fumaça passa pelas duas camadas (20 telas).
- 2.2.2 Topo de área com contexto: o que a área já sabe, com ponto de estado ("Monitor em 60 Hz, aceita 180", "Wi-Fi
  com sinal de 66%", "4 mudanças no histórico"), de `lib/contexto.ts`, testado.
- 2.2.3 Estados vazios conferidos: todos dizem o que fazer para ter o dado; Histórico e Jogos ganharam o caminho.
- 2.2.4 Esqueleto de carregamento no lugar do "Lendo…" solto (17 lugares, parado com jogo aberto).
- 6.4 A primeira partida sem receita mede o jogo (2 e 8 min) e guarda a receita para o próximo "Jogar".
- 2.2.5 A fumaça confere também o notebook de 1366×768 a 125% (1093×614): cards, trilho, barra de cima e abas cabem.
- 2.3.1 Movimento com sistema: três durações e duas curvas em `tokens.css` (`--mov-*`, `--curva-*`), 23 valores soltos
  trocados por elas; o congelamento com jogo aberto e "reduzir movimento" agora cobre também o `::before`.
- 2.3.2 Tipografia: 30 tamanhos quebrados (11, 11,5, 12,5, 13,5, 26, 30, 34 px) trazidos para a escala; dígitos tabulares
  no app inteiro.
- 2.3.3 Gráfico de verdade da partida: cada janela medida (FPS médio e 1% pior) sobre a faixa das partidas anteriores,
  um eixo só, rótulo na ponta, legenda, cruz com os números e tabela para leitor de tela (`GraficoDaPartida.svelte`). A
  série de tempo de quadro inteira não é guardada de propósito (milhares de carimbos por janela); o gráfico usa as janelas.
- 2.3.4 Prints automáticos: `node scripts/prints-nova.mjs` grava as 20 telas nos três modos (60 prints, em
  `prints-nova/`, fora do Git). Revisados o escuro e o "sem fundo" em todas: nada destoando depois dos tokens únicos.
- 6.1 Investigado no PC do dono: nenhum pânico no log e nenhum arquivo em `falhas/`; toda sessão sem fechamento normal
  foi encerramento forçado (testes de desenvolvimento e o instalador fechando o Otimiza), e a abertura já não conta
  reinício do Windows nem atualização como queda.
- 6.5 Medido na versão final: janela pedida em 1,2 s e primeiro desenho em 1,6 s; antes da janela só roda coisa leve
  (o resto é a WebView2 subindo). Nada que valha cortar.
- 6.6 Textos e marcações conferidos com o `ui/DESIGN.md`: o teste de fila trocou a borda colorida na lateral (proibida)
  pelo ponto de estado.
- Partida de respeito, fase 3: "Deixar liso" no cartão da partida com tranco; aviso de memória apertada no "Jogar".

**Não feitos, com o motivo**: R4 (trocar DNS não muda o ping; está no "o que não fazemos"); FB5 (sem os marcadores
do jogo o atraso só se estima); FB4 (cópia entre placas duas vezes por quadro, sem como medir aqui); VRR do FB7 (a
cadeia de troca da sobreposição, por DirectComposition, não aceita o modo sem sincronia).

**Falta**: 2.1.2 (dados reais nos testes), 2.3.5; atestado C e D (decisão do dono, seção 8, item 3). **Do dono**: seção 8, mais publicar o `convite.json` (canal de feedback novo e a lista de desligados).
