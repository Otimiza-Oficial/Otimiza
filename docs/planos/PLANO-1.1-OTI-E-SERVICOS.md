# Plano 1.1 — o Oti de verdade e todos os serviços melhores

> 08/11/2026. Continua o `PLANO-1.0-NOVA-ERA.md` (as Ondas 0 a 2 estão no `docs/app/PROGRESS.md`) e soma à pesquisa de
> `APIS-E-PARCERIAS.md` (05/11). As regras do produto não mudam: **nunca menos FPS, tudo se desfaz, medido ≠ estimado,
> nada muda sem a pessoa pedir.**

## 0. A ideia em uma frase

O Otimiza deixa de ser "um painel com botões" e vira **o Oti: um assistente que conhece o seu PC, mede, explica com
número e conserta com a sua permissão** — e que prova depois que funcionou.

O que nenhum concorrente tem, e é o nosso fosso:
1. **Prova medida** (Lab, prova alternada A-B, selo Medido). Os outros prometem "+30% FPS"; nós mostramos o antes e o depois.
2. **Tudo se desfaz**, inclusive "Desfazer tudo" sem licença.
3. **Um assistente que lê os números reais da sua máquina**, em português, e não chuta.

---

## 1. O Oti como assistente de IA

### 1.1 O que ele faz (exemplos reais de conversa)
- "Por que meu FiveM trava?" → ele lê a última partida (1% pior, trancos por minuto), o Caçador de DPC e a rota de rede, e
  responde: *"O tranco não é a placa: o driver de rede segurou o processador 5 vezes acima de 1 ms. Teste por cabo; se
  quiser, ligo o Wi-Fi sem varredura durante o jogo."* — com o botão **[Ligar]**.
- "Vale a pena trocar a placa?" → compara o gargalo medido (processador a 98% num núcleo, placa folgada) e diz que **não**,
  que o gargalo é o processador.
- "O que você mudou no meu PC?" → lê o Histórico e resume, com **[Desfazer tudo]**.
- "Meu ping pula à noite" → mede a rota agora, compara com a de antes e aponta o trecho (casa × operadora × servidor).

### 1.2 Arquitetura (duas camadas)
1. **Camada local (já existe, sempre grátis, sem internet):** as falas de `lib/conversa.ts`, o entendimento por
   palavra-chave e os avisos lidos. É o Oti de hoje.
2. **Camada de IA (nova):** a pergunta que a camada local não entende vai para a IA.

```
App (Tauri) ──pergunta + fatos do PC──▶ Servidor do Otimiza (Node, o mesmo do bot)
                                          │  valida a licença, limita uso, guarda a chave
                                          ▼
                                    API do Claude (tool use)
                                          │  pede "rodar ferramenta X"
App ◀── tool_use ─────────────────────────┘
 │  executa no PC (Rust): leitura roda sozinha;
 │  mudança SÓ com o botão [Aplicar] da pessoa
 └──▶ tool_result ──▶ servidor ──▶ Claude ──▶ resposta final ──▶ balão do Oti
```

- **A chave da API nunca vai no app.** Fica no servidor (mesmo padrão da Steam Web API e da Riot: chave em executável
  distribuído é chave vazada). O app manda a licença; o servidor confere com a assinatura que já existe (`atestado`).
- **As ferramentas rodam no PC da pessoa**, pelos comandos tipados que já existem. O modelo só *pede*; quem executa é o
  Rust. Ferramentas de leitura (diagnóstico, medir, rota, DPC, histórico) rodam sozinhas. **Ferramentas que mudam algo
  nunca rodam sozinhas**: viram um botão na fala do Oti e só executam com o clique. O desfazer de sempre vale.
- **Só fatos saem do PC**: modelo do processador e da placa, números medidos, nomes dos ajustes. Nada de arquivo, nome de
  usuário ou caminho de disco. A tela diz isso na primeira vez e a IA vem **desligada até a pessoa ligar**.

### 1.3 Modelo, custo e detalhes técnicos (conferidos na referência oficial da API, 08/11/2026)
- **Modelo:** `claude-opus-5-5` (US$ 4 / 20 por milhão de tokens de entrada / saída; leitura de cache US$ 0,20), com
  `effort: "low"` para conversa (o padrão dele é `medium`; conversa rende bem em `low` e custa menos).
  Alternativa mais barata, **decisão do dono**: `claude-haiku-5-5` (US$ 0,10 / 0,50) para as perguntas simples.
- **Custo estimado por resposta com Opus 5.5:** ~6 mil tokens de instrução + ferramentas em cache (~US$ 0,001), ~2 mil de
  fatos e pergunta (~US$ 0,008) e ~500 de resposta (~US$ 0,01) ≈ **US$ 0,02 por resposta**. Uma conversa de 5 trocas
  ≈ US$ 0,10. Com Haiku 5.5, ≈ 20× menos. **Estimativa, não medida**: medir com `usage` nas primeiras 100 conversas.
- **Cache de prompt:** instrução do Oti e lista de ferramentas fixas, em ordem fixa, com `cache_control`. Nada de data ou
  hora na instrução (invalida o cache).
- **Ferramentas com `strict: true`** e `tool_choice: auto` (o forçado dá erro 400 no Opus 5.5).
- **Recusa:** tratar `stop_reason: "refusal"` e ligar o `fallbacks: "default"` do servidor.
- **Respostas em streaming**, letra a letra no balão (o balão já faz isso).
- **SDK oficial TypeScript** (`@anthropic-ai/sdk`) no servidor Node; nada de chamada HTTP solta.
- **Limites:** por licença, N perguntas por dia (o servidor conta); sem licença, só a camada local.

### 1.4 Avaliação antes de lançar
Um conjunto de 50 perguntas reais de clientes (do Discord e do suporte) com a resposta certa conhecida, rodado a cada
mudança de instrução. Critérios: **nunca promete FPS sem número medido**, nunca manda aplicar sem botão, diz "não sei"
quando não há dado, responde em português simples.

### 1.5 O mascote: o Oti vivo
- **Rive** (já previsto no plano mestre) com máquina de estados: os humores de hoje (atento, falando, pensando,
  preocupado, feliz, surpreso, jogando, dormindo) viram animação de verdade, com transição entre eles.
- **Ele reage ao PC:** fica preocupado quando a temperatura sobe, cochila quando o PC está parado, comemora quando a
  prova dá "Ganhou".
- **Ele aparece no jogo, discreto:** na sobreposição, um Oti pequeno no canto pisca âmbar quando o 1% pior cai (só onde o
  anticheat deixa).
- **Bandeja do Windows:** o ícone é o Oti com os olhos do humor atual.
- **Som opcional** (desligado por padrão): um "plim" curto quando termina uma medição.

---

## 2. Cada serviço, melhor

| Área | Hoje | Próximo passo | Fora da caixa |
|---|---|---|---|
| **Turbo** | Mede 10 s, monta a receita, prova A-B | Receita por **jogo + mapa/modo** (servidor de RP pesado × leve); sugerir a qualidade gráfica que mais rende por FPS | **Turbo da comunidade:** receitas que ganharam na prova em PCs iguais ao seu (mesma CPU + placa), anônimas e opt-in |
| **Jogos** | Instalados, capas, última partida, ganho medido | Histórico de partidas em gráfico; perfis prontos para CS2, Valorant, Fortnite, Roblox, Minecraft | **CS2 Game State Integration:** o Oti sabe quando você está em partida, no menu ou morto, e só mede o que importa |
| **Rede** | Rota salto a salto, Wi-Fi, banda de fundo | Histórico do ping por horário ("à noite sua operadora piora") | **Relatório para a operadora:** PDF com a rota e os horários, pronto para abrir chamado na Vivo/Claro/TIM |
| **Latência** | Caçador de DPC | Clique → imagem pelo PresentMon (`MsPCLatency`), taxa real do mouse pelo Raw Input | **Teste de reflexo** dentro do app, para a pessoa sentir o antes e o depois |
| **Energia** | Plano OTIMIZA, energia de jogo, tetos | A calibração por candidatos (o motor já existe no Rust) | **Guarda térmica:** se o notebook passar do limite, o Oti avisa antes de o jogo travar |
| **Tela** | Hz certo em um clique, desfazer | VRR (G-Sync/FreeSync) e HDR conferidos | **Teste de ghosting** com a câmera do celular (QR code abre a página do teste) |
| **Sistema** | Catálogo, inicialização, limpeza, restauração | Saúde do disco (SMART), mover jogo para o SSD | **Boot medido:** quanto o PC leva para ligar, antes e depois |
| **Prova** | Lab e prova alternada | Relatório bonito para compartilhar (imagem pronta para o Discord) | **Bancada pública:** os números agregados no site, por placa e jogo |
| **Ajustes** | Sobreposição, preferências, licença | Canal beta, exportar/importar configuração | **Conta na nuvem:** backup do histórico e das receitas |

---

## 3. APIs novas (pesquisa de 08/11; soma à de 05/11)

| API | O que dá | Situação | Veredito |
|---|---|---|---|
| **API do Claude (Anthropic)** | O Oti com IA, tool use, streaming, cache | Pública; chave só no servidor | **Pode entrar já** (seção 1) |
| **Riot Games API** (Valorant, LoL) | Partidas e conta (RSO) | Chave de produção exige aprovação (relatos de meses); chave nunca no binário; produto pago precisa ser registrado | **Precisa de pedido**: abrir já, porque demora |
| **Windows Auto Super Resolution** | Upscaling por IA do próprio Windows | Expandido em 2026, ainda ligado a NPU | **Acompanhar**: só ler se está ligado e explicar |
| **Advanced Shader Delivery** (DirectX) | Shaders pré-compilados no download (menos tranco na primeira vez) | Amplamente disponível segundo o blog do DirectX; datas conflitantes nas fontes | **Acompanhar**: explicar ao cliente quando o jogo tiver |
| **NVIDIA Reflex 2 / Frame Warp** | Menos atraso | SDK ainda não público em meados de 2026 | **Não serve ainda** |
| **Roblox** | — | A Open Cloud é do lado do servidor; o teto de FPS do cliente é 240 e o arquivo de configuração não vale mais | **Ler, não mexer**: o aviso "Roblox preso em 60" continua, mas o conserto é pelo menu do jogo |

Continuam valendo as prioridades de 05/11: PresentMon 2.6, telemetria pelo driver (NVAPI/NVML/ADLX/IGCL), IPinfo Lite, QoS
e Wi-Fi nativos, CS2 GSI e Discord Rich Presence.

---

## 4. Ideias fora da caixa (as melhores)

1. **"Pergunte ao Oti" dentro do jogo:** atalho de teclado abre um balão por cima do jogo (onde o anticheat deixa), sem
   sair da partida.
2. **Modo campeonato:** um toque e o Oti confere tudo antes de uma partida importante (rede, temperatura, Hz, fundo
   segurado) e mostra um "pronto" verde.
3. **Diário do PC:** toda semana o Oti resume o que mudou (driver novo, Windows atualizou, FPS caiu 8% no FiveM) com a
   provável causa.
4. **Detector de regressão:** o Windows atualizou e o FPS caiu? O Oti mostra a data, o número e oferece voltar o driver.
5. **Receita por PC parecido:** "neste i3-10100F + GTX 1650, o que mais ganhou no FiveM foi X" (inteligência coletiva,
   opt-in).
6. **Oti no Discord:** o bot que já existe responde dúvidas de suporte com a mesma IA e manda o cliente abrir o app com o
   botão certo.

---

## 5. Ondas

| Onda | Entrega | Pronto quando |
|---|---|---|
| **3a** | Oti IA: servidor (Node, SDK oficial), 8 ferramentas de leitura, botões de confirmação, limites por licença, avaliação com 50 perguntas | Responde "por que meu jogo trava?" com os números da última partida, e nunca aplica sem clique |
| **3b** | Rede, Latência e Energia novas (seção 2), PresentMon clique → imagem | Cada uma com prova antes × depois numa máquina real |
| **4** | Oti em Rive, bandeja, Oti no jogo, CS2 GSI, Discord Rich Presence | Animações a 60 quadros sem custar FPS no jogo (medido) |
| **5** | Comunidade (receitas, bancada pública), conta na nuvem | Números publicados e opt-in claro |

Antes da 3a: abrir o app de verdade e aprovar a interface nova; apagar a interface antiga (`main.ts`, `v33/`).

---

## 6. Decisões do dono

1. **Modelo da IA:** Opus 5.5 em tudo (~US$ 0,02 por resposta) ou Haiku 5.5 nas perguntas simples (~20× menos)?
2. **Quantas perguntas por dia** por licença? E quem não comprou: só a camada local ou umas poucas perguntas para provar?
3. **IA ligada por padrão ou desligada até a pessoa ligar?** (Recomendado: desligada, com convite na primeira vez.)
4. **Pedir já a chave de produção da Riot?** (Demora.)
5. **Designer para o Oti em Rive:** interno ou freelancer?
6. **Inteligência coletiva:** envio anônimo opt-in das receitas que ganharam na prova?

Fontes da pesquisa de 08/11: [Riot — Valorant](https://developer.riotgames.com/docs/valorant),
[Riot — política](https://developer.riotgames.com/policies.html),
[DevForum Roblox — teto de 240](https://devforum.roblox.com/t/roblox-caps-fps-at-240-regardless-of-globalbasicsettings13xml-framerate-cap/4451048),
[DirectX Developer Blog 2026](https://devblogs.microsoft.com/directx/2026/),
[Pureinfotech — Windows 11 em 2026](https://pureinfotech.com/windows-11-gaming-upgrades-2026/),
[NVIDIA — Reflex 2](https://www.nvidia.com/en-us/geforce/news/reflex-2-even-lower-latency-gameplay-with-frame-warp),
[itc.ua — Reflex 2 ainda não lançado](https://itc.ua/ua/novini/nvidia-reklamuye-funktsiyu-reflex-2-yaku-dosi-ne-vypustyla-za-18-misyatsiv-pislya-anonsu/amp/).
