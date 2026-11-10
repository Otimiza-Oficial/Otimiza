# Plano — Partida de respeito

> O cliente abre o jogo e sente: liso, sem tranco, FPS lá em cima onde a máquina permite, e o Otimiza provando com
> número o que mudou. Este plano junta as mudanças visuais e estruturais que mais pesam nessa experiência.
>
> Escrito em 10/11/2026, depois da Onda 2 (interface nova, a antiga apagada). Complementa o
> `PLANO-1.0-NOVA-ERA.md`, sem repetir o que ele já decidiu.

---

## 0. A promessa, dita do jeito certo

**"Liso sempre. FPS mais alto onde a máquina tem folga. E a prova na sua tela."**

- **Liso** é o que o olho sente: ritmo constante de quadros, sem tranco. Um jogo a 60 fps cravados parece melhor que
  um a 40–76. Isso dá para entregar em quase toda máquina.
- **FPS mais alto** só existe onde há folga: monitor na taxa errada, energia segurando o processador, memória sem XMP,
  gráfico pesado demais para a placa, programa disputando núcleo. Onde a máquina já está no limite, o Otimiza diz
  isso em vez de inventar ganho.
- **A prova** é o que nos separa dos "otimizadores" de registro: antes × depois medido, e "piorou" desfeito sozinho.

As regras do produto continuam: nunca menos FPS, tudo desfaz, medido ≠ estimado, nada de placebo (lista na seção 9).

---

## 1. O problema de hoje: as peças existem, mas ficam espalhadas

O motor já tem o que um jogo liso precisa: Jogo liso (`plataforma/jogo_liso.rs` e `ritmo_estavel.rs`), Frame Boost
com base travada (`base_do_jogo.rs`), energia de jogo (`energia_jogo.rs`), banda do segundo plano (`banda.rs`),
memória para o jogo (`memoria_jogo.rs`), guarda térmica (`guarda_termica.rs`), Auto CPU Set (`cpuset.rs`), "pronto
para jogar" (`prontojogo.rs`), o que fica por cima do jogo (`por_cima.rs`), o Turbo (receita pelo gargalo), a prova
A-B (`provaalternada.rs`) e o relatório da partida.

Na hora de jogar, o cliente:

1. precisa saber que cada peça existe, e elas moram em 13 áreas diferentes;
2. precisa ligar cada uma, na ordem certa (o teto do driver só vale ao **reabrir** o jogo);
3. não vê, ao sair do jogo, o que mudou: o relatório fica numa área que ele não abre.

**A maior diferença não está num ajuste novo: está em fazer o que já existe acontecer sozinho, na hora certa, e
mostrar o resultado.** Esse é o Pilar A.

---

## 2. Pilar A — Modo Partida (estrutural, o mais importante)

Um ciclo só, igual para todo jogo: **antes → durante → depois**. Tudo com o que a pessoa já autorizou uma vez.

### A1. Antes do jogo: "Jogar" pelo Otimiza

- **Botão "Jogar" na capa do jogo** (tela inicial e área Jogos). Ele faz o "pronto para jogar" (`prontojogo.rs`), aplica
  a receita lembrada desse jogo e **abre o jogo** (Steam: `steam://rungameid/<appid>`; Epic e FiveM pelo executável
  ou protocolo). Hoje o Otimiza não abre jogo nenhum; por isso o teto do driver sempre "vale na próxima abertura".
  Abrindo pelo Otimiza, vale **já nesta partida**.
- **Ordem fixa e testada:** o que precisa estar antes do jogo abrir vem primeiro (teto do driver, base do Frame Boost,
  perfil gráfico do arquivo de configuração, Hz do monitor). O resto liga com o jogo aberto (energia, banda, CPU Set).
- **Na primeira vez, uma folha só:** "Para o FiveM, o Otimiza vai: travar em 60 fps (o que ele segura), segurar
  downloads de fundo, subir o processador sem hesitar. Tudo volta quando você fechar." Botões: Jogar assim · Ajustar.
  Nas vezes seguintes, um toque.
- **Abriu pelo atalho do Steam, sem passar pelo Otimiza?** O vigia de sessão (`sessao.rs`) detecta e aplica o que
  ainda dá; o cartão diz "o teto vale na próxima vez: abra pelo Jogar do Otimiza".

### A2. Durante o jogo: o Otimiza some e vigia

- A interface congela (já existe: `jogo-aberto`). Na bandeja, o Oti mostra o humor (já existe).
- **Guardas ligadas sozinhas,** cada uma só lendo, com aviso discreto (notificação do Windows ou overlay, nunca uma
  janela por cima): calor (`guarda_termica.rs`), memória acabando, memória de vídeo cheia, programa novo disparando o
  processador, janela "sempre por cima" que derruba a tela cheia (`por_cima.rs`).
- **Overlay mínimo, opcional** (`overlay.rs`): FPS e 1% pior, redesenhado no visual novo (seção 5.6).
- **Desfaz sozinho o que piorar,** pela regra que já vale no Frame Boost (desliga se o jogo perde FPS) e no protocolo
  de grupos (`ReverterSozinho`).

### A3. Depois do jogo: o cartão da partida

Quando o jogo fecha, o Otimiza volta tudo ao que era e mostra **um cartão** (notificação clicável e o primeiro bloco
da tela inicial):

```
FiveM · 1h12 de partida
85 FPS médio   ·  1% pior 54  ·  3 trancos/min
Contra as suas 11 partidas anteriores: 1% pior +22%, trancos −60%   [medido]
O que ajudou: teto em 60, downloads segurados.  O que não mudou nada: CPU Set (desligado).
[Ver a partida]  [Compartilhar no Discord]
```

- Números do relatório da partida (`relatoriodapartida.rs`) e da comparação com as anteriores (`regressao.rs`).
- "Compartilhar" gera a imagem do resultado (`compartilhar.rs`): o cliente mostra a prova no Discord, e isso vende.
- Se a partida foi pior que a média, o cartão diz isso, com a causa provável (`receita_da_partida`).

**Pronto quando:** o cliente toca "Jogar" uma vez, joga, fecha, e vê o cartão com o antes × depois, sem abrir nenhuma
outra área. Medido num i3 com FiveM e num Ryzen com CS2.

---

## 3. Pilar B — Liso primeiro (tempo de quadro, não só FPS)

O que o cliente chama de "travando" quase sempre é **oscilação** (trancos), não FPS médio baixo.

| # | Mudança | Estado | Por que importa |
|---|---|---|---|
| B1 | **Jogo liso ligado por padrão no Modo Partida** quando os trancos/min passam do limite | peça existe, falta o automático | Medido no FiveM do dono: 45 fps de mediana, 74 trancos/min; o teto no que ele segura corta a oscilação |
| B2 | **Regra da tela com VRR:** G-Sync/FreeSync ligado, teto 3 fps abaixo do Hz, V-Sync no driver | novo (`vrr.rs` já lê o EDID) | É a combinação que tira o rasgo de imagem sem o atraso do V-Sync puro |
| B3 | **Base do Frame Boost travada** numa fração do monitor (60 num de 180) | existe (`base_do_jogo.rs`) | Sem isso, o gerador multiplica a oscilação (o olho vê judder) |
| B4 | **Shader cache aquecido:** cache do driver sem limite e aviso de "primeira partida depois do driver novo" | parte existe (`shaders.rs`) | O tranco de compilação de shader é o mais comum depois de atualizar o driver |
| B5 | **Memória antes do jogo:** oferecer fechar o que ocupa quando a memória vai faltar | existe (`memoria_jogo.rs`), falta ir para o "Jogar" | Com 8 GB, o FiveM trava ao carregar cenário quando a memória acaba |
| B6 | **Jogo no SSD:** mostrar que ele está no HD e mover com um botão (Steam: "Mover pasta de instalação") | leitura existe (`discodojogo.rs`) | Trancos de streaming não se resolvem com ajuste nenhum |
| B7 | **MPO (Multiplane Overlay):** em placas e drivers onde ele causa tranco ou piscada em janela sem bordas, desligar como experimento, com prova e desfazer | novo, só como experimento medido | Correção real em parte das máquinas, placebo nas outras: por isso só fica se a prova mostrar ganho |
| B8 | **Otimizações para jogos em janela** (modelo de apresentação flip em DX10/11 sem bordas) | conferir `janelas.rs` | Menos atraso e menos tranco em janela sem bordas, que é como quase todo mundo joga FiveM |

---

## 4. Pilar C — FPS nas alturas, onde a máquina tem folga

Em ordem de ganho real. Cada item mostra o ganho **estimado** antes e o **medido** depois.

1. **Monitor na taxa máxima** (já existe, área Tela). É o "ganho" mais fácil: 60 Hz num monitor de 180.
2. **Processador sem hesitar no jogo:** energia de jogo (EPP) e plano OTIMIZA (já existem).
3. **Memória no perfil certo (XMP/EXPO):** detectar a memória abaixo da velocidade de fábrica e guiar na BIOS com
   passo a passo por placa-mãe (`fichabios.rs`, área BIOS). Em jogo preso no processador, como o FiveM, costuma ser o
   maior ganho de hardware sem comprar nada. O Otimiza não mexe na BIOS: ele mede, explica e prova depois.
4. **Resizable BAR** desligado numa placa que suporta: mesmo caminho da BIOS (`rbar.rs` já lê).
5. **Gráfico certo para a placa, por jogo:** perfis prontos (Máximo de FPS, Equilibrado) para FiveM/GTA, CS2,
   Valorant, Fortnite, Roblox e Minecraft, decididos pelo gargalo (`orquestrador.rs` já recusa baixar gráfico quando o
   gargalo é o processador). É a maior alavanca quando a placa está no limite.
6. **Escala (FSR) e Frame Boost** quando a placa está no limite e o monitor tem Hz sobrando (já existem; o Turbo decide).
7. **Núcleos para o jogo:** Auto CPU Set nos processadores híbridos e X3D (`cpuset.rs`, `x3d.rs`), só quando medido
   que rende.
8. **Driver certo:** do fabricante, não o genérico do Windows (a aba Agora já avisa); oferecer o link certo e
   lembrar de reabrir o jogo para recompilar os shaders.

**O que o cliente vê:** na página do jogo, uma linha por item com o estado ("feito", "falta você fazer na BIOS",
"não rende nesta máquina") e quanto rendeu aqui. Saber que algo **não rende** também é resposta: deixa parar de mexer.

---

## 5. Pilar D — Visual e experiência

### 5.1 Menos áreas, organizadas pela intenção

Hoje são 13 áreas. Proposta de 4 abas principais, com o resto dentro delas:

- **Jogar:** a tela inicial, com as capas, o "Jogar", o pronto para jogar e o último cartão da partida.
- **Meu PC:** Tela, Energia, Rede, Latência, Sistema, BIOS (o que é da máquina).
- **Prova:** partidas, antes × depois, histórico e desfazer (o que foi medido).
- **Ajustes:** licença, feedback, aparência (incluindo o "Sem fundo"), avançado.

O trilho fica com essas 4 e a busca (Ctrl+K / "Pergunte ao Oti") leva a qualquer item pelo nome.

### 5.2 A capa do jogo como protagonista

- Capa grande, com a arte certa de cada jogo (feito: FiveM com o logo dele, Roblox unificado).
- Sobre a capa, os 3 números que importam (FPS, 1% pior, trancos/min) e o estado da receita em fichas
  ("teto 60 · downloads segurados · FSR qualidade").
- Um botão principal: **Jogar**.

### 5.3 O gráfico da partida (o "antes × depois" que se vê)

- Linha do tempo de quadro da partida (PresentMon já entrega os intervalos), com os trancos marcados e o que aconteceu
  na hora de cada um (o detetive de travadas já existe no mapa de 40 s).
- Duas partidas sobrepostas: a de antes e a de agora. É a prova que o cliente entende sem ler número.

### 5.4 Primeira execução: o check-up de 60 segundos

Ao instalar, o Otimiza mede por 60 s e mostra **as 3 coisas que mais pesam nesta máquina**, com o ganho estimado e
um botão cada: "Monitor em 60 Hz, aceita 180", "Memória sem XMP", "O jogo está no HD". É o momento "uau" do plano
mestre (4.3), agora com o conteúdo definido.

### 5.5 O Oti como termômetro

O humor do mascote já sai do estado medido. Ligar a ele o resultado da última partida: feliz quando o 1% pior subiu,
preocupado quando uma guarda disparou, dormindo sem jogo. Um olhar na bandeja diz como foi.

### 5.6 Overlay e widget com o visual novo

- Overlay mínimo: pílula de vidro escura no canto, FPS e 1% pior, sem moldura. Detalhado: com o gráfico de tempo de
  quadro de 10 s.
- Widget da Xbox Game Bar (plano mestre, Onda 4) para jogos com anticheat que barra sobreposição.

### 5.7 Acabamento

- Carregamento com esqueleto do conteúdo, nunca "Lendo…" solto.
- Transição entre áreas curta (já há `in:entrar`), sempre desligada com jogo aberto.
- Tema e "Sem fundo" lembrados (feito); tamanho e posição da janela lembrados.
- Revisão de textos: todo número com nome, todo botão dizendo o que faz (regras do `ui/DESIGN.md`).

---

## 6. Pilar E — Prova que convence sem a pessoa pedir

- **Prova automática em segundo plano:** nas primeiras partidas de cada jogo, o Otimiza alterna com e sem a receita
  (`provaalternada.rs`) e só mantém o que ganhou. O cliente não precisa saber o que é A-B: ele vê "provado em 4
  partidas".
- **"O Otimiza desfez X porque piorou"** aparece no cartão da partida e no Histórico. É raro, e por isso gera confiança.
- **Resumo do mês:** "Este mês: 23 partidas, 1% pior médio +18% no FiveM, 2 coisas desfeitas por piorar." Com
  Compartilhar.

---

## 7. Pilar F — Fundação que isso exige

1. **Orquestrador da partida no Rust:** um só lugar que recebe "jogo abriu" e "jogo fechou" (o vigia de `sessao.rs`)
   e roda a receita na ordem certa, com transação (`transacao.rs`) para voltar tudo mesmo se o Otimiza cair.
2. **Receita por jogo guardada e versionada:** o que ligar, em que ordem, e o resultado da última prova de cada peça.
3. **Lançador de jogo:** Steam, Epic, FiveM, Roblox e executável, com o processo do jogo reconhecido logo ao abrir.
4. **Tipos gerados para tudo:** a migração deixou vários componentes com `invoke` e tipos escritos à mão. O defeito do
   Histórico de hoje (o "desfecho" que chega como objeto) é dessa classe. Passar esses comandos para o contrato gerado
   (`contrato.rs` → `tipos.ts`), como o `ui/LEIA-ME.md` já manda.
5. **Dados reais nos testes da interface:** gravar respostas de verdade do Rust (uma máquina com NVIDIA, uma com AMD,
   uma sem jogo) e usar no `fumaca:nova`, além da demonstração. A demonstração escondeu o defeito de hoje.
6. **Orçamento do próprio Otimiza com o jogo aberto:** menos de 0,5% de um núcleo (meta do plano mestre), medido na CI
   com o Modo Partida inteiro ligado.

---

## 8. Ordem sugerida

| Fase | O que entra | Impacto | Esforço | Pronto quando |
|---|---|---|---|---|
| **1** | Cartão da partida ao fechar o jogo (A3) + tipos gerados nos componentes migrados (F4) | Alto | Baixo | O cartão aparece com números certos em 3 jogos reais |
| **2** | Botão "Jogar" e lançador (A1, F3) + orquestrador da partida (F1, F2) | Muito alto | Médio | Um toque aplica a receita, abre o jogo e o teto já vale nesta partida |
| **3** | Liso por padrão: Jogo liso automático, VRR + teto, memória antes do jogo (B1, B2, B5) | Muito alto | Médio | Trancos/min −50% nos PCs com oscilação, medido |
| **4** | Check-up de 60 s e o caminho da BIOS para XMP e rBAR (5.4, C3, C4) | Alto | Médio | Em 10 PCs de teste, o check-up acha o item certo em todos |
| **5** | 4 abas, capa protagonista, gráfico da partida (5.1–5.3) | Alto (percepção) | Médio | Um cliente novo acha o "Jogar" sem ajuda, em teste com 5 pessoas |
| **6** | Prova automática, resumo do mês, compartilhar (E) | Médio (confiança e venda) | Baixo | "Provado em N partidas" aparece sem a pessoa abrir a Prova |
| **7** | Experimentos medidos: MPO, flip em janela, shader cache (B4, B7, B8) | Médio, varia por máquina | Baixo cada | Cada um só fica ligado onde a prova mostrou ganho |
| **8** | Overlay e widget novos (5.6) | Médio | Médio | Overlay abaixo de 0,2% de um núcleo, medido |

As fases 1 e 2 mudam a experiência sozinhas: o cliente passa a sentir o Otimiza toda vez que joga, não só quando abre
o app.

---

## 9. O que continua fora (a confiança vem daqui)

O mesmo do plano mestre, seção 15: nada de desligar Defender ou UAC, "limpeza de registro", liberar RAM à força,
chaves placebo, driver de kernel próprio, injeção no jogo, "ping booster", prioridade tempo real, fechar programa sem a
pessoa escolher. E **nunca prometer FPS** que não foi medido nesta máquina: a tela mostra estimado como estimado.

---

## 10. Como saber que funcionou

| Meta | Medida |
|---|---|
| Trancos/min **−50%** nas máquinas com oscilação | Cartões da partida, antes × depois |
| 1% pior **maior** no FiveM, CS2 e Roblox em pelo menos 70% das máquinas | Prova automática |
| **0** peça que piorou e ficou ligada | Portão e Histórico |
| Cliente usa o "Jogar" em **mais da metade** das partidas | Telemetria opt-in |
| Cartão da partida compartilhado no Discord **toda semana** | Canal de feedback |
| Otimiza com o jogo aberto **abaixo de 0,5% de um núcleo** | CI |

---

## 11. Decisões que são do dono

1. **Abrir o jogo pelo Otimiza** (o botão "Jogar") como caminho principal? Muda o hábito do cliente, e é o que faz o teto
   valer já na partida.
2. **Modo Partida ligado por padrão** depois da primeira confirmação, ou sempre perguntar?
3. **4 abas** no lugar das 13 áreas: aprovar a divisão da seção 5.1.
4. **Compartilhar no Discord** com a imagem do resultado: entra já na fase 1 ou fica para a 6?
