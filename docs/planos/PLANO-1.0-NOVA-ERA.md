# Otimiza 1.0 — Nova Geração

> Plano mestre da maior atualização da história do Otimiza. Marca nova (o mascote de vidro), interface nova e
> autoral ("o céu do seu PC"), motor melhorado, sistemas novos (Rede, Latência, Energia 2.0, Tela, Sistema Voando,
> Assistente), integrações com as APIs de NVIDIA, AMD, Intel, Microsoft, Valve, Discord e fabricantes de periféricos,
> e uma bancada pública para provar que somos os melhores.
>
> Escrito em 05/11/2026 a partir do repositório na 3.6.0. Referências visuais em `pc-optimizer/brand/nova-geracao/`:
> `mascote-logo.png` (a logo) e `referencia-interface.png` (a inspiração da interface).

---

## Índice

0. Decisões já fechadas
1. A visão em uma tela
2. O que não muda (a alma)
3. Identidade — o mascote e a marca
4. Interface — "o céu do seu PC"
5. O Assistente — o mascote que entende o seu PC
6. Motor — o que já existe, melhorado
7. Sistemas novos
8. Integrações com empresas — APIs e parcerias
9. Ciência e prova — como provar que somos os melhores
10. Plataforma — conta, nuvem, celular
11. Fundação de engenharia
12. Lançamento, site e comunidade
13. Roteiro em ondas
14. Metas (KPIs)
15. O que continua fora
16. Riscos
17. Decisões que ainda são do dono

---

## 0. Decisões já fechadas

| Decisão | Como fica |
|---|---|
| **Versão** | Por dentro, **4.0.0** (`Cargo.toml`, `tauri.conf.json`, `package.json`, tag `v4.0.0`). Para o cliente, **"Otimiza 1.0"** em todo lugar. Regra fixa: *versão exibida = (maior − 3).menor.correção* → 4.0.0 = 1.0, 4.1.2 = 1.1.2. O atualizador, o instalador e o histórico do Desfazer continuam funcionando para quem está na 3.x |
| **Logo** | O mascote de vidro: bolha translúcida azul-lavanda, dois olhos âmbar em cápsula, duas "orelhas" de vidro, fundo em degradê pêssego → céu |
| **Inspiração da interface** | Área de trabalho em céu azul com nuvens, janelas e cartões de vidro, objetos-ícone em 3D, widgets à direita, dock embaixo, barra de menu em cima, seletor de tema por bolinhas de cor |

**Como a versão exibida é implementada:**
- Uma função só, `versao_de_exibicao()`, no Rust, usada pela interface, pelo relatório, pelo log e pelas notas.
- O teste `as_quatro_declaracoes_de_versao_concordam` ganha a quinta: a exibida bate com a regra.
- No release do GitHub: tag `v4.0.0`, **título "Otimiza 1.0 — Nova Geração"**, instalador `Otimiza-1.0-Setup.exe`.
- "Aplicativos instalados" do Windows continua mostrando 4.0.0 (`DisplayVersion`). **Não regravar esse valor:** o
  instalador do Tauri o lê e compara com a versão nova (`SemverCompare`) para decidir entre atualizar, reinstalar ou
  rebaixar; com "1.0" ali, um 4.0.0 por cima de um 4.1 passaria por atualização. Conferido no modelo NSIS embutido
  no `@tauri-apps/cli` em 05/11/2026. O nome do produto no instalador pode dizer "Otimiza 1.0" sem mexer no número.

---

## 1. A visão em uma tela

**Hoje (3.6):** o motor mais honesto do mercado, escondido atrás de telas técnicas. O cliente não *sente* o que o
Otimiza sabe.

**A 1.0:** você abre o Otimiza e vê **o céu do seu PC**. Céu limpo: está tudo bem. Nuvens: tem coisa atrapalhando.
Tempestade: algo sério (temperatura, disco morrendo, Wi-Fi caindo). O mascote vive ali, olha para você, reage ao
estado da máquina e explica em português simples o que está acontecendo. Um toque no **Turbo** e o céu abre. Depois
do jogo, ele mostra o cartão: *"+22 no 1% pior, 0 trancos, ping estável. Medido."*

**Os seis pilares:**

| # | Pilar | Uma frase |
|---|---|---|
| 1 | **Marca** | Um mascote que vira o rosto do produto: no app, na bandeja, no jogo, no Discord, no teclado RGB |
| 2 | **Interface** | Uma área de trabalho de vidro sobre o céu, linda parada e que custa zero com o jogo aberto |
| 3 | **Inteligência** | O Assistente que lê os dados reais do PC e explica, sugere e prova |
| 4 | **Desempenho** | Rede, Latência, Energia 2.0, Tela, Sistema Voando e perfis para os jogos grandes |
| 5 | **Integrações** | Tudo o que NVIDIA, AMD, Intel, Microsoft, Valve e Discord abrem para nós, usado a favor do cliente |
| 6 | **Prova** | Uma bancada pública que compara com os concorrentes e mostra até onde não ganhamos |

---

## 2. O que não muda (a alma)

A cara muda inteira. As regras que fazem o cliente confiar ficam:

1. **Nunca menos FPS.** Toda mudança automática que encosta em jogo é medida e desfeita sozinha se piorar.
2. **Tudo desfaz, com o valor exato.** Estado anterior *lido*, nunca presumido.
3. **Medido ≠ estimado.** Todo número tem selo. "FPS na tela" nunca é vendido como FPS do jogo.
4. **Lê o hardware antes de oferecer.** O que faz mal à máquina não aparece.
5. **Sem injeção em jogo, sem driver de kernel próprio, sem escrever na BIOS, sem desligar Defender/UAC/Update.**
6. **Admite quando não houve ganho.**

Na 1.0 essas regras viram *design*: o mascote nunca comemora o que não foi medido. Os olhos dele são o selo.

---

## 3. Identidade — o mascote e a marca

### 3.1 O mascote

**Nome.** Sugestões, para o dono decidir: **Oti**, **Bolha**, **Pixel**, **Gota**. O nome curto vira verbo na
comunidade ("passa o Oti").

**Anatomia (da logo):** bolha de vidro, olhos âmbar em cápsula, "orelhas" de vidro dos lados. Os **olhos** são a
expressão inteira. Não ganha boca, braços nem roupa: menos é mais, e é isso que deixa ele reconhecível em 16 px.

**Estados (máquina de estados, cada um ligado a um dado real):**

| Estado | Quando | Como aparece |
|---|---|---|
| Descansando | PC ocioso | Olhos meio fechados, respiração lenta (a bolha pulsa 2%) |
| Atento | Otimiza aberto, nada errado | Olhos seguem o cursor |
| Focado | Jogo detectado | Olhos estreitos, brilho azul mais forte |
| Turbo | Turbo ligado | Reflexo correndo pela bolha, "orelhas" brilham |
| Medindo | Prova em andamento | Olhos piscam no ritmo da amostragem |
| Feliz | Prova com ganho **medido** | Olhos em arco por 2 s (só com MEDIDO) |
| Honesto | Prova sem diferença | Olhos neutros, pisca uma vez: "já estava bom" |
| Preocupado | Temperatura alta, disco falhando, rede caindo | Bolha fica âmbar nas bordas, olhar para baixo |
| Dormindo | Bandeja, sem jogo há muito tempo | Olhos fechados |

**Onde ele mora:**
- No app (canto, e grande na Início e na primeira execução).
- **Ícone da bandeja em 3 estados** (ocioso, jogo, Turbo), feito a partir dos olhos.
- Na sobreposição do jogo, pequeno, opcional.
- No **widget da Xbox Game Bar** (ver 8.4).
- No **teclado e mouse RGB** (ver 8.7): a cor do mascote acompanha o estado.
- No Discord (Rich Presence, figurinhas e emojis do servidor).
- No instalador, no site e no vídeo de lançamento.

**Ferramenta de animação:** **Rive** (máquina de estados nativa, executor web leve, arquivo pequeno). O mascote em 3D
pré-renderizado vira as versões estáticas (ícone, PNG grande). Sem `three.js` rodando o tempo todo.

### 3.2 Paleta (tirada da logo e da referência)

| Token | Uso | Valor inicial (afinar com o designer) |
|---|---|---|
| `ceu-100` | Fundo claro, degradê de cima | `#EAF2FD` |
| `ceu-300` | Vidro azulado | `#C9DCF8` |
| `ceu-500` | Azul de destaque (botões, foco) | `#5B86E5` |
| `ceu-700` | Azul profundo (texto em vidro, céu da tarde) | `#2F5FA8` |
| `lavanda-400` | Borda do vidro, brilho do mascote | `#A9B8F2` |
| `pessego-200` | Calor do degradê, luz da manhã | `#FFE6CC` |
| `ambar-500` | Olhos do mascote; **acento de marca** | `#C97A1E` |
| `noite-900` | Tema noite / modo jogo | `#0E1630` |
| `sucesso` / `atencao` / `perigo` | Só para estado, nunca decoração | verde-água / âmbar / coral |

Regra: o âmbar dos olhos é a cor de marca. O vermelho só aparece quando algo piorou de verdade.

### 3.3 Material: vidro
- **Vidro claro:** fundo translúcido, borda de 1 px mais clara em cima (luz), sombra suave embaixo, desfoque do
  fundo. Igual à bolha do mascote.
- **Vidro denso:** para leitura (tabelas, textos longos), quase opaco.
- **Brilho de borda:** gradiente lavanda → pêssego na borda, como na logo, só em elementos de destaque (Turbo, Cartão de Prova).

### 3.4 Tipografia
- Interface: uma sans geométrica arredondada que combine com a bolha (candidatas livres: **Geist**, **Plus
  Jakarta Sans**, **Manrope**; ou uma fonte própria no futuro).
- Números: monoespaçada com dígitos tabulares (**Geist Mono** / **JetBrains Mono**). Número ao vivo não pode "dançar".

### 3.5 Ícones-objeto (o toque da referência)
A referência usa objetos reais (pasta, câmera, sintetizador) em vez de ícones chapados. O Otimiza ganha **seus
objetos próprios em 3D**, no mesmo vidro do mascote:

| Área | Objeto |
|---|---|
| Turbo | Foguete de vidro / raio dentro de uma bolha |
| Jogos | Controle de vidro |
| Rede | Antena com ondas |
| Latência | Cronômetro / mouse de vidro |
| Energia | Bateria/raio âmbar |
| Tela | Monitor de vidro |
| Sistema | Engrenagem translúcida |
| Prova | Frasco de laboratório |
| Histórico | Ampulheta |
| Ajustes | Botão giratório |

Feitos uma vez em 3D (Blender), exportados em PNG/WebP em 3 tamanhos. Bonitos e de custo zero na execução.

### 3.6 Som (opcional, desligável)
Três sons curtos e macios: Turbo ligado, prova concluída, alerta. Desligados por padrão com jogo aberto.

### 3.7 Entregáveis de marca
Logo (símbolo, horizontal com "Otimiza", monocromático, versão 16 px), mascote em Rive com todos os estados, 10
objetos 3D, ícone do app (`.ico` multi-resolução), bandeja em 3 estados, imagens do instalador, papéis de parede de
céu (manhã, tarde, pôr do sol, noite), kit Discord (emojis, figurinhas, banner), manual de marca de 2 páginas, tokens
em um arquivo único usado pelo app e pelo site.

---

## 4. Interface — "o céu do seu PC"

### 4.1 O conceito
O Otimiza vira uma **área de trabalho própria**, como na referência:

```
┌──────────────────────────────────────────────────────────────────────────┐
│ ◉ Otimiza   Turbo  Jogos  Ajuda          🎮 FiveM aberto · Turbo ON   21:40 │  ← barra de menu
├────┬───────────────────────────────────────────────────┬─────────────────┤
│    │                                                   │ ○ ○ ○ ○  tema    │
│ 🎮 │        (mascote grande)                           │ ┌─────┐┌──────┐ │
│ 🎮 │        "Seu PC está pronto. Céu limpo."           │ │ FPS ││ PING │ │
│ 🎮 │                                                   │ │ 144 ││ 28ms │ │  ← widgets
│ 🎮 │   [foguete]  [controle]  [antena]  [cronômetro]   │ └─────┘└──────┘ │
│    │                                                   │ ┌─────────────┐ │
│ ⊕  │   [bateria]  [monitor]   [engrenagem] [frasco]    │ │ Último jogo  │ │
│    │                                                   │ │ +22 1% pior  │ │
│    │                                                   │ └─────────────┘ │
│    │          ┌─────────────────────────────┐          │ ┌─────────────┐ │
│    │          │ 🚀 🎮 📡 ⏱ ⚡ 🖥 ⚙ 🧪 ⏳ 🎛 │  ← dock    │ │ Temperatura │ │
└────┴──────────┴─────────────────────────────┴──────────┴─────────────────┘
   ↑ trilho de jogos (como os "espaços" da referência)
```

- **Fundo: o céu que é o diagnóstico.** Limpo (tudo bem), algumas nuvens (avisos), nublado (vários problemas),
  tempestade (crítico). O céu também segue a hora do dia (manhã pêssego, tarde azul, noite estrelada) quando está tudo bem.
  O cliente entende a saúde do PC **sem ler nada**.
- **Barra de menu em cima:** jogo detectado, estado do Turbo, alertas, hora.
- **Trilho à esquerda:** os jogos do cliente como "espaços" (capas redondas), um clique abre o espaço daquele jogo.
- **Mesa no centro:** o mascote e os objetos-ícone de cada área.
- **Widgets à direita:** FPS, ping, temperatura, último resultado, energia. O cliente escolhe quais.
- **Dock embaixo:** as 10 áreas, sempre a um clique.
- **Seletor de tema por bolinhas:** Céu (padrão), Pôr do sol, Lavanda, **Noite** (escuro, para quem joga à noite).

### 4.2 As telas (cada uma abre como uma janela de vidro sobre o céu)

| Área | Pergunta do jogador | O que mostra |
|---|---|---|
| **Início** | "Como está meu PC?" | Céu + mascote, nota de prontidão, gargalo atual, último jogo, **botão Turbo** |
| **Turbo** | "Me dá mais FPS" | A receita automática por gargalo, Frame Boost, Jogo liso, escala, os 3 números separados (real, na tela, leveza) |
| **Jogos** | "E no meu jogo?" | Biblioteca com capas; por jogo: perfil gráfico, histórico, prova, ajustes do jogo |
| **Rede** | "Por que meu ping pula?" | Ping/jitter/perda ao vivo, onde está o problema, Wi-Fi, controle de banda |
| **Latência** | "Por que o jogo parece atrasado?" | Clique → imagem medido, mouse, driver, Caçador de DPC |
| **Energia** | "Meu processador está no máximo?" | Plano dinâmico, clock e temperatura ao vivo, limites atingidos |
| **Tela** | "Meu monitor está certo?" | Hz real × máximo, VRR, HDR, modo de apresentação do jogo, cores |
| **Sistema** | "O Windows atrapalha?" | Catálogo por grupo, inicialização, serviços, disco, BIOS |
| **Prova** | "Funcionou mesmo?" | Laboratório, prova alternada, relatórios, bancada |
| **Histórico** | "O que mudou?" | Linha do tempo, Desfazer por item, Desfazer tudo, Restaurar meu PC |
| **Ajustes** | — | Tema, mascote, sons, sobreposição, licença, beta, privacidade |

**Paleta de comandos (Ctrl+K)** com o mascote ("o que você quer fazer?").

### 4.3 Primeira execução — o momento "uau"
1. Céu nublado, mascote acorda.
2. "Vou olhar o seu PC." Varredura ao vivo: os objetos acendem um a um (processador, placa, memória, disco, rede,
   monitor) com o que foi encontrado.
3. As nuvens viram cartões: "Seu monitor de 165 Hz está em 60 Hz", "Seu jogo está no HD", "O Wi-Fi está em 2,4 GHz".
4. **Plano personalizado** em 3 grupos: Seguro agora · Precisa reiniciar · Escolha sua (trocas).
5. Aplicar → diário ao vivo → o céu vai abrindo conforme cada coisa é resolvida.
6. "Agora jogue 10 minutos que eu meço." → depois do jogo, Cartão de Prova.

### 4.4 Componentes-assinatura
- **Cartão de Prova:** antes → depois, barra de incerteza, selo MEDIDO, mascote no estado certo. Feito para virar
  print no Discord, com a marca no canto.
- **Linha de frametime ao vivo:** fina, âmbar sobre vidro; trancos marcados como pequenas nuvens.
- **Selo de origem** em todo número (MEDIDO, ESTIMADO, NÃO MEDIDO).
- **Diário ao vivo:** cada alteração aparece enquanto acontece, com o valor de antes.
- **Desfazer a um clique** de qualquer coisa.
- **Medidor-bolha:** os medidores de uso (CPU, placa, memória) são bolhas de vidro enchendo de líquido, não barras.

### 4.5 Desempenho da própria interface (regra de produto, com teste)
Vidro com desfoque é bonito e **caro** no WebView2. O Otimiza não pode roubar quadro do jogo:
- **Jogo aberto → a interface congela:** sem animação, sem desfoque ao vivo, sem gráfico redesenhando; o céu vira
  imagem parada. Só o laço do Rust trabalha. Meta: < 0,5% de um núcleo, 0% de placa (fora o Frame Boost, medido à parte).
- **Desfoque barato:** o céu é uma imagem; o "vidro" usa uma versão já desfocada dela (pré-calculada), não
  `backdrop-filter` ao vivo em tudo. `backdrop-filter` só nas janelas da frente.
- **Modo leve automático** em PC fraco ou em notebook na bateria: sem desfoque, sem animação do céu.
- **Bandeja:** < 0,2% de processador; memória do WebView2 liberada ao minimizar.
- **Abertura:** < 1,5 s até a primeira tela útil num i3 com SSD SATA.
- A CI mede o tamanho do pacote e o tempo de abertura e falha se passar do teto.

### 4.6 Pilha técnica da interface
| Escolha | Recomendação | Por quê |
|---|---|---|
| Framework | **Svelte 5** + TypeScript + Vite | Reatividade fina, runtime pequeno, ótimo para números ao vivo |
| Contrato com o Rust | **tauri-specta** | Tipos gerados: comando renomeado quebra na compilação |
| Mascote | **Rive** (executor web) | Máquina de estados ligada aos dados |
| Gráficos ao vivo | Canvas próprio | Custo controlado |
| Ícones-objeto | PNG/WebP pré-renderizados | Zero custo em execução |
| Acessibilidade | Respeitar "reduzir movimento" e alto contraste do Windows; navegação por teclado; contraste AA mesmo no vidro | Vidro claro com texto claro é o erro clássico; o vidro denso existe por isso |

A interface nasce numa pasta nova. **`src/main.ts` (modo clássico) e `src/v33/` são apagados no lançamento.**

### 4.7 Processo de design
1. Moodboard a partir da referência + mascote → 2. Três explorações da Início (céu, mesa, widgets) → 3. Protótipo
navegável com dados reais gravados de PCs de verdade → 4. Teste com 5 jogadores (gravar a tela) → 5. Sistema de
componentes → 6. Telas → 7. Teste de custo com o FiveM aberto.

---

## 5. O Assistente — o mascote que entende o seu PC

O diferencial que nenhum concorrente tem com honestidade: **o mascote responde perguntas com os dados reais da
máquina**, não com texto genérico.

**O que ele faz:**
- "Por que meu FiveM trava?" → lê a última sessão (trancos, gargalo, causa da travada, DPC, rede) e responde:
  "Na sua última partida o processador estava a 99% e o Discord Canary usava meio núcleo. Quer que eu acalme ele
  durante o jogo?" com o botão da ação.
- Explica qualquer item do catálogo em linguagem simples.
- Lê o relatório de suporte e já sugere a resposta (para a equipe, no Discord).
- Resume a semana: "3 sessões, 1% pior subiu 14%, ping estável."

**Como (duas camadas):**
1. **Camada local, sempre ligada, grátis:** regras do próprio Otimiza (o veredito, a causa da travada, o gargalo)
   transformadas em frases. Funciona sem internet e responde 80% das perguntas.
2. **Camada de IA na nuvem, opcional e com consentimento:** um modelo de linguagem (ex.: **Claude Haiku 5.5** para
   respostas rápidas e baratas; **Claude Sonnet 5.5** para o diagnóstico profundo e o suporte) recebe **só o resumo
   técnico** (números, nomes de componentes, nunca arquivos pessoais, nome de usuário ou IP) e devolve a explicação e
   as ações sugeridas.
   - A chamada passa por um servidor nosso (a chave da API nunca fica no executável), com limite por licença.
   - **O assistente nunca aplica nada sozinho:** sugere, o cliente toca no botão, e a ação passa pelo mesmo caminho
     de sempre (portão, histórico, desfazer).
   - A resposta só pode citar números que vieram da medição (o servidor confere: número que não está no resumo é cortado).

**Local no futuro:** em PCs com NPU (Copilot+), um modelo pequeno rodando localmente via ONNX Runtime/Windows ML
para a camada 1 ficar mais esperta sem internet. Pesquisa, não promessa da 1.0.

---

## 6. Motor — o que já existe, melhorado

### 6.1 Turbo
- **Um toque na Início**, ligado ao jogo aberto ou ao último jogado.
- **Receita explicada em uma linha** pelo mascote.
- **Turbo automático** quando o jogo abre, unificando o "modo jogo" e o Turbo num lugar só.
- Juntar tudo o que ainda está espalhado (Jogo liso, perfil do GTA, Auto CPU Set, Energia de jogo, controle de banda).
- **Turbo por fase do jogo:** com o CS2 informando se está em partida ou no menu (8.6), o Turbo aperta na partida e
  alivia no menu.

### 6.2 Prova
- **Prova curta (F6 da 3.6):** teste sequencial que para quando a diferença está clara. Meta: menos de 1 minuto
  quando o efeito é grande.
- **Prova mais sensível:** mais rodadas curtas alternadas e o frametime inteiro, para enxergar ganhos de 5–8% (hoje
  só ~22% no FPS médio).
- **Prova passiva:** cada sessão de jogo já é uma medida. Depois de algumas sessões com e sem um ajuste, o Otimiza
  diz se ele rendeu, sem pedir teste.
- **Relatório compartilhável assinado** (não dá para forjar o print) com a marca nova.

### 6.3 Frame Boost (gerador próprio)
- Medir no FiveM em **tela cheia** (o caminho que sobrou).
- **Modo leve** padrão em placa de entrada.
- Captura WGC num Windows original (bloqueada no Windows modificado do dono).
- **Aceleração pelo hardware de fluxo óptico quando existir** (NVIDIA Optical Flow SDK em RTX 20+; o TU117 do dono não
  tem): estimativa de movimento quase de graça, sobra placa para o jogo. Ver 8.1.
- Nunca vender o "na tela" como FPS do jogo.

### 6.4 Processador
- Governador por perfil de máquina (agressivo em 4 núcleos, quase parado em 16).
- **Ryzen X3D:** jogo no CCD com cache (driver `amd3dvcache` + CPU Sets, medido).
- **Intel híbrida:** thread principal do jogo em núcleo P (CPU Sets medidos, Intel APO quando presente).
- **Ranking de núcleos:** o Windows e o processador sabem quais núcleos chegam no clock mais alto (CPPC). O Otimiza
  lê e prefere os melhores para o jogo, medido.

### 6.5 Dívidas a pagar no caminho
- Dividir `commands.rs` (6.032 linhas) por domínio.
- Juntar duplicatas: `telemetry`×`core/telemetria`, `frames`×`presentmon`, `benchmark`×`baseline`,
  `conflicts`×`conflitos`, `registry`×`registro`, `restore`×`restaurar`, `bottleneck`×`gargalo`.
- Apagar comandos que nenhuma tela chama.
- Abstração `PerfilDeJogo` (sair do acoplamento ao FiveM).
- `anticheat::permite` obrigatório em todo recurso que toca janela ou processo de jogo.
- Corrigir a contradição pública: o README diz que o Otimiza se recusa a desligar Spectre/Meltdown, e o catálogo tem
  `mitigacoes_spectre_fora`. Texto e produto têm que dizer a mesma coisa.

---

## 7. Sistemas novos

### 7.1 Otimiza Rede

**Promessa honesta:** não diminuímos a distância até o servidor (o ping base). **Tiramos os picos, a perda e o jitter
que nascem no PC e na casa**, e dizemos com número onde está o problema quando não é no PC.

**Diagnóstico (só lê):**
| Recurso | Como | Responde |
|---|---|---|
| **Ping passivo do jogo** | Estatísticas TCP por conexão do Windows (IP Helper, `GetPerTcpConnectionEStats`: RTT e retransmissões da conexão do próprio jogo), sem mandar um pacote sequer | O ping que o jogo está vivendo, sem servidor bloqueando ICMP |
| Ping, jitter e perda ao vivo | Base atual de `rede.rs` durante a sessão (para jogos em UDP) | "Seu ping pula de 30 para 180 a cada 60 s" |
| **Onde está o problema** | Rota salto a salto (estilo MTR) até o servidor + nome do provedor de cada trecho (consulta de ASN) | "Na sua casa", "na sua operadora (Vivo)", "no servidor" |
| **Teste de bufferbloat** | Ping ao roteador e ao servidor enquanto gera carga de download/upload por ~10 s (servidores abertos de medição, ex.: M-Lab ndt7; pedindo antes, porque usa dados) | Nota A–F: "quando alguém baixa algo, seu ping sobe 140 ms" |
| **Wi-Fi** | WLAN API nativa: sinal, banda 2,4/5/6 GHz, canal, velocidade do enlace, trocas de ponto de acesso, canais vizinhos lotados | "2,4 GHz com sinal fraco e 9 redes no seu canal: use 5 GHz ou cabo" |
| Placa de rede | Velocidade negociada, driver, economia de energia, moderação de interrupção | "Seu cabo negociou 100 Mb: cabo ou porta com defeito" |
| Quem usa a rede agora | Contadores por processo | "OneDrive enviando 8 Mb/s durante a partida" |
| DNS | Tempo de vários resolvedores | Honesto: acelera abrir site e entrar no servidor, **não muda o ping dentro do jogo** |

**Ações (reversíveis, medidas, "só durante o jogo" quando faz sentido):**
| Ação | Efeito real | Cuidado |
|---|---|---|
| **Controle de banda do segundo plano** | Políticas QoS do Windows por programa (limite de taxa) para Steam, Epic, Battle.net, OneDrive, navegadores, Windows Update e Otimização de Entrega enquanto o jogo está aberto | O maior inimigo do ping do lado do PC. Nunca toca em voz (Discord) nem no jogo. Volta ao fechar |
| **Pausar downloads automáticos** | Windows Update, Loja, sincronizações durante a partida | Nunca desliga o Update; só adia enquanto joga |
| **Wi-Fi sem varredura durante o jogo** | Para a busca periódica de redes que causa pico de ping a cada ~60 s em muitas placas | Só conectado em Wi-Fi; guardião religa na abertura seguinte se algo cair |
| Placa de rede sem economia de energia, sem "Ethernet Verde"/EEE | Menos atraso ao acordar a placa | Medido |
| **Prioridade DSCP do jogo** | Pacotes do jogo marcados | Só ajuda se o roteador respeitar; a tela diz |
| Moderação de interrupção / filas RSS | Laboratório | Pode custar quadro em jogo preso no processador |
| DNS mais rápido | Troca medida, anterior guardado | Sem promessa de ping |

**No jogo:** ping, jitter e perda na sobreposição e no widget da Game Bar.
**Na prova:** picos de ping por minuto antes × depois, como os trancos.

### 7.2 Otimiza Latência — do clique à imagem

**Promessa honesta:** medimos as etapas que dá para medir e somamos só o medido ("pelo menos X ms"). Nunca o "1 ms"
do mercado. Base pronta: `latencia.rs` (5 etapas), `mouse.rs`.

| Recurso | Como | Hoje |
|---|---|---|
| **Clique → imagem medido** | `ClickToPhotonLatency` e `AllInputToPhotonLatency` do PresentMon (o cabeçalho já é lido) | Lido, nunca mostrado |
| **Latência do PC com Reflex** | Jogos com NVIDIA Reflex publicam marcadores de latência (PC Latency) que o PresentMon lê sem injetar nada | Novo |
| **Taxa do mouse** | Teste "mexa o mouse 3 s" com Raw Input/GameInput na janela do Otimiza (sem gancho global) | A conta existe, ninguém alimenta |
| **Caçador de DPC/ISR** | Sessão ETW de kernel por ~30 s; nomeia o driver que causa picos (como o LatencyMon, com código próprio) | O analisador do laboratório já provou a leitura |
| **Baixa latência do driver unificada** | NVIDIA (perfil DRS), AMD Anti-Lag (ADLX), Intel (IGCL) | Pedaços existem, juntar e medir |
| Reflex / Anti-Lag 2 no jogo | Ler a config do jogo e ligar quando é arquivo seguro | Novo, por jogo |
| Modo de apresentação | Independent Flip × cópia (medido no FiveM: 85 × 61 fps) | Existe, ganha destaque |
| VRR + teto | Teto alguns quadros abaixo dos Hz | **Só como escolha** "Latência mínima" (teto baixa média) |
| Afinidade de interrupção | Por dispositivo | Laboratório, só com medida |
| **Teste de reflexo** | Minijogo de 30 s na janela do Otimiza: clique quando a bolha mudar de cor; mede o tempo do sistema (entrada → tela) antes e depois | Divertido, compartilhável e didático |

### 7.3 Energia 2.0 — o plano OTIMIZA de verdade

Hoje: plano OTIMIZA próprio + motor por arquitetura (Intel legada/moderna/híbrida, Zen 2–5) + Energia de jogo (EPP 0
só com o jogo aberto). **Problema:** o motor adaptativo quase nunca roda, porque exige uma medição longa.

1. **Dois estados automáticos: Jogo e Descanso.** O jogo abre → Jogo (EPP 0, boost agressivo, núcleos acordados,
   dicas de latência no máximo, PCIe sem ASPM, USB sem suspensão seletiva, Wi-Fi em desempenho máximo, placa no modo
   de desempenho do driver). Fecha → Descanso (frio e silencioso). Rápido no jogo, fresco no resto do dia.
2. **Calibração sozinha:** o motor por arquitetura aprende nas primeiras sessões de jogo, com os dados que a sessão já
   coleta, em vez de um teste longo.
3. **Por arquitetura:** Intel híbrida (sem estacionar núcleo P, política heterogênea para P), Ryzen X3D (CCD com
   cache), notebook (estado Jogo **só na tomada**).
4. **Limites de potência visíveis:** PL1/PL2/térmico (Intel), PPT/TDC/EDC (AMD, via Ryzen Master Monitoring SDK — ver
   8.2). "Seu processador está sendo segurado pelo limite de potência do notebook" é resposta, não mistério.
5. **Guarda térmica:** se a temperatura ou o estrangulamento subirem, o estado Jogo recua um degrau sozinho e o mascote avisa.
6. **Placa de vídeo:** limite de potência e temperatura da placa lidos (NVML/ADLX), modo "desempenho máximo" do
   driver só com jogo aberto, e o veredito "sua placa está no limite de temperatura, não de força".
7. **Prova:** o plano novo passa pela prova alternada como qualquer item.

### 7.4 Otimiza Tela *(novo — o ganho mais fácil do mercado)*
Uma quantidade enorme de jogadores tem monitor de 144/165/240 Hz rodando a 60 Hz e nunca soube.
| Recurso | Responde |
|---|---|
| **Hz real × máximo do monitor** (lido da EDID e dos modos) | "Seu monitor aguenta 165 Hz e está em 60. Corrigir?" — um clique, reversível |
| Cabo e porta | "Esse cabo/porta não passa de 60 Hz nessa resolução: use DisplayPort" |
| VRR (G-Sync/FreeSync) | Ligado no monitor? no driver? no Windows? |
| HDR | Ligado sem conteúdo HDR (deixando tudo cinza)? Auto HDR? |
| Modo de apresentação do jogo | Tela cheia x sem bordas com o custo medido |
| Escala e resolução | Escala do Windows atrapalhando jogo antigo |
| Cores | O que já existe (vibração, rampa), com a marca nova |
| Dois monitores com Hz diferentes | O conhecido tranco do segundo monitor rodando vídeo |

### 7.5 Sistema Voando
| Recurso | Prova |
|---|---|
| **Tempo de boot medido** (rastreamento de boot do Windows) | "Liga em 38 s → 21 s" |
| Inicialização com custo medido por programa | "O X atrasa seu boot em 6 s" |
| Serviços e tarefas (auditores existentes) | — |
| Espaço + cache de shader velho | GB liberados |
| **Saúde do armazenamento:** SMART/NVMe, TRIM, SSD quase cheio, jogo no HD (`discodojogo.rs`), DirectStorage disponível | "Seu jogo está no HD: mover para o SSD tira os trancos de carregamento" |
| **Mover jogo para o SSD** (Steam/Epic pelos caminhos oficiais delas) | Guiado, sem quebrar a biblioteca |
| Driver de vídeo: idade, versão, link oficial; aviso de "atualizou e começou a travar" (cache de shader velho) | — |
| Memória: quem ocupa, antes de jogar (já existe) | GB livres |
| **Tempo de abertura dos apps** (navegador, Discord) | "O Discord abre em 4 s → 2 s" |

### 7.6 Perfis por jogo — os grandes do Brasil
Cada perfil: leitura e escrita do **arquivo de config do próprio jogo** (com o jogo fechado, cópia de antes, desfazer,
portão), opções que pesam no processador × na placa ligadas ao gargalo medido, Reflex/Anti-Lag, modo de tela, limite.

| Ordem | Jogo | Observação |
|---|---|---|
| 1 | FiveM / GTA V | Já existe; completar |
| 2 | **CS2** | Config + `autoexec` + Game State Integration (8.6) |
| 3 | **Valorant** | Só arquivo de config; Vanguard respeitado (nada por cima do jogo) |
| 4 | **Fortnite** | `GameUserSettings.ini`, modo de desempenho do jogo |
| 5 | Warzone / CoD | Arquivo de config; Ricochet respeitado |
| 6 | Apex | `videoconfig.txt` |
| 7 | **Roblox** | Base gigante em PC fraco (limite de FPS, qualidade) |
| 8 | **Minecraft** | Java: memória da JVM, argumentos; recomendação de mods de desempenho oficiais da comunidade (Sodium etc.) com link, sem instalar |
| 9 | LoL, Free Fire (emulador), Rocket League, Rainbow Six, PUBG | Pela demanda do Discord |

### 7.7 Memória e jogo
- "Memória para o jogo" (existe) com o mascote: "Feche o Chrome (2,6 GB) antes de jogar?"
- Arquivo de paginação no SSD certo e tamanho adequado em PC de 8 GB.
- Continua fora: "liberar RAM à força".

### 7.8 Saúde e temperatura
- Temperaturas de processador, placa e SSD com histórico por sessão.
- **Ventoinhas:** só leitura (sem driver de kernel). Aviso de poeira quando a temperatura sobe com o tempo na mesma carga.
- Eventos de hardware (WHEA) e travamentos do jogo (`crashes.rs`) explicados.

### 7.9 Guardião
- `deriva.rs` com tela: "A atualização do Windows desfez 3 ajustes seus. Reaplicar?"
- Conferência na abertura do Windows de tudo que é "só durante o jogo" (rede, energia, Wi-Fi, controle de banda):
  nada fica preso se o Otimiza fechar no meio.

### 7.10 Inteligência coletiva (o fosso)
- **Telemetria opt-in, anônima:** "em PCs com i3-10100F + GTX 1650, o item X rendeu +6% de 1% pior em 41 de 50 provas".
- **Regras assinadas remotas:** desligar na hora um item que piora num driver novo, sem lançar versão.
- O Turbo escolhe a receita pela evidência de máquinas parecidas: "recomendado por 230 provas em PCs como o seu".

---

## 8. Integrações com empresas — APIs e parcerias

> **Atualizado em 05/11/2026 pela checagem da Onda 0** (`APIS-E-PARCERIAS.md`). Saem ou mudam:
> - pausar o Windows Update por API (não funciona desde o Windows 10, e não há política no Home);
> - FrameView SDK (não é público);
> - "configurações ideais" da NVIDIA e APO da Intel (sem API);
> - V-Cache da AMD (só Linux);
> - widget da Game Bar (exige UWP na Store: vai para depois da 1.0);
> - M-Lab, RIPEstat e IGDB (exigem acordo para uso comercial).
>
> As tabelas abaixo são a visão original; quando divergirem, vale `APIS-E-PARCERIAS.md`.

> Regra para todas: **antes de entrar no produto, cada API passa por uma checagem na Onda 0** — licença de
> redistribuição, termos de uso comercial, se exige conta/chave/aprovação, se funciona sem o programa do fabricante
> instalado, e se algum anticheat reclama. A coluna "A conferir" diz o que já sabemos que precisa ser checado.

### 8.1 NVIDIA
| API / programa | O que nos dá | Hoje no Otimiza | A conferir |
|---|---|---|---|
| **NVAPI** (SDK público no GitHub) | Perfis do driver (DRS): limite de FPS, baixa latência, modo de energia, V-Sync, shader cache, vibração | Usada (`nvdriver.rs`) | Ampliar para o modo de energia "desempenho máximo" por jogo e o tamanho do cache de shader |
| **NVML** | Clock, temperatura, potência, **motivo do estrangulamento** (térmico, potência, confiabilidade), uso de memória | Usada (`nvml.rs`) | Expor os motivos de estrangulamento na Energia |
| **Reflex / PC Latency (marcadores)** | Latência do PC medida nos jogos com Reflex, lida pelo PresentMon sem injetar | Não | Quais jogos publicam; colunas na versão do PresentMon que embarcamos |
| **Optical Flow SDK** | Hardware de fluxo óptico em RTX 20+: estimativa de movimento do Frame Boost quase de graça | Não (o TU117 do dono não tem) | Licença de redistribuição; ganho medido contra o DIS próprio |
| **G-Assist (plugins)** | O assistente de IA da NVIDIA aceita plugins: "G-Assist, otimiza meu PC" chama o Otimiza | Não | Formato do plugin, distribuição, se exige aprovação da NVIDIA |
| **FrameView SDK** | Medição de quadros e potência no padrão da NVIDIA | Não | É distribuído sob acordo; pedir acesso |
| **NVIDIA App — configurações ideais** | Não tem API pública | — | Só por parceria (ver 8.10) |

### 8.2 AMD
| API / programa | O que nos dá | Hoje | A conferir |
|---|---|---|---|
| **ADLX** | Anti-Lag, AFMF, RSR, Chill, Boost, limitador, cores, sensores e métricas da Radeon | Usada (`amddriver.rs`), **nunca testada numa Radeon** | Testar no laboratório (prioridade alta) |
| **Ryzen Master Monitoring SDK** | Por núcleo: clock, temperatura, PPT/TDC/EDC, CCD | Não | Licença e se exige o driver do Ryzen Master instalado |
| **Driver de V-Cache (`amd3dvcache`)** | Preferência de CCD em X3D | Lê (`x3d.rs`) | Medir com jogo real num X3D |
| **Anti-Lag 2 / FSR** | Integrados pelo jogo, não por nós | — | Só recomendar ligar no jogo |

### 8.3 Intel
| API / programa | O que nos dá | Hoje | A conferir |
|---|---|---|---|
| **IGCL** | Baixa latência, limitador, cores e métricas da Arc/Iris | Usada (`inteldriver.rs`), **nunca numa Intel Arc** | Laboratório |
| **PresentMon Service API** | O PresentMon como serviço com API em C, no lugar de rodar o executável e ler CSV: mais leve, mais métricas, sem processo extra | Usamos o executável (sidecar) | Licença (o PresentMon é aberto) e se o serviço pode ser embarcado no nosso instalador |
| **Intel APO** | Otimização de threads em processadores suportados | Lê (`apo.rs`) | Quais jogos/CPUs; oferecer onde existir |
| Thread Director (pelo Windows) | Classe de núcleo P/E | Lê | — |

### 8.4 Microsoft / Windows
| API | O que nos dá | Hoje |
|---|---|---|
| **ETW** (kernel, DPC/ISR, boot, disco, rede) | Caçador de DPC, boot medido, causa da travada | Usada em partes |
| **Xbox Game Bar — Widget SDK** | **Um widget oficial do Otimiza dentro da Game Bar (Win+G)**: FPS, ping, temperatura, botão Turbo e o mascote, sobre o jogo, **sem a nossa janela por cima** (o caminho mais seguro com anticheat) | Não — **prioridade alta** |
| **GameInput** | Entrada moderna com carimbo de tempo: taxa do mouse/controle medida com precisão | Não |
| **QoS (políticas de rede) e IP Helper** | Controle de banda por programa, DSCP, ping passivo por conexão TCP | Não |
| **WLAN API** | Tudo do Wi-Fi | Não |
| **Windows Update Agent** | Adiar atualização durante a partida | Não |
| **DXGI / DirectComposition / D3D11** | Frame Boost | Usada |
| **DirectStorage** (detecção) | Saber se o jogo pode usar e se o SSD/ambiente permite | Não |
| **ONNX Runtime / Windows ML / DirectML** | IA local (assistente, e no futuro interpolação por rede neural em placa forte/NPU) | Pesquisa |
| **Auto SR / Auto HDR** | Detectar e explicar (Auto SR só em Copilot+) | Não |

### 8.5 Medição de rede (serviços abertos)
| Serviço | Uso | A conferir |
|---|---|---|
| **M-Lab (ndt7)** | Velocidade e latência sob carga para o teste de bufferbloat | Política de uso aceitável |
| Endpoints de velocidade da Cloudflare | Alternativa ao M-Lab | Termos de uso comercial |
| **Consulta de ASN** (ex.: dados públicos de registro de IP) | Nomear a operadora de cada salto da rota | Limites e licença da base |
| RIPE Atlas | Comparar a rota do cliente com sondas da mesma cidade (avançado) | Créditos e termos |

### 8.6 Jogos e lojas
| API | O que nos dá |
|---|---|
| **CS2 / Dota 2 — Game State Integration (oficial da Valve)** | O jogo avisa o Otimiza se está em partida, no menu ou morto: Turbo por fase, medida só durante a partida, relatório por mapa |
| **Steam** (arquivos locais + Web API) | Biblioteca, caminhos, tempo de jogo, mover jogo para o SSD |
| Epic, Battle.net, EA, Ubisoft, Riot (manifestos locais) | Detecção de jogos instalados |
| **Cfx.re (FiveM)** | Informação pública dos servidores (endereço, jogadores): ping até o servidor certo |
| **SteamGridDB / IGDB** | Capas bonitas para o trilho de jogos (`capas.rs` existe) — exige chave |

### 8.7 Periféricos e RGB — o mascote no teclado
| SDK | Uso |
|---|---|
| **Razer Chroma SDK** | Teclado/mouse na cor do estado: azul (pronto), âmbar (quente), pulso no Turbo |
| **Logitech LIGHTSYNC (G HUB)** | Mesmo |
| **Corsair iCUE SDK** | Mesmo |
| **SteelSeries GameSense** | Mesmo + mostrar FPS/ping no OLED dos teclados que têm |
| **OpenRGB (SDK de rede)** | Todo o resto, sem o programa do fabricante |

Sempre opcional, desligado por padrão, e sem custo com o jogo aberto (atualiza 1×/s, nunca por quadro).

### 8.8 Comunidade
| API | Uso |
|---|---|
| **Discord Rich Presence / Social SDK** | "Jogando FiveM · otimizado pelo Otimiza · 1% pior +22" no perfil (opcional): marketing orgânico |
| Bot de licença (já existe) | Conta, feedback, suporte; ganha o assistente para responder tickets com o relatório do cliente |
| Login com Discord no painel web | Conta sem senha nova |

### 8.9 IA
| API | Uso |
|---|---|
| **Claude API (Anthropic)** | Camada de nuvem do Assistente (5): Haiku 5.5 para respostas rápidas, Sonnet 5.5 para diagnóstico profundo e suporte |
| ONNX Runtime | Camada local (pesquisa) |

### 8.10 Parcerias (trilha de negócio, em paralelo ao código)
- **NVIDIA:** programa de desenvolvedores (acesso a SDKs sob acordo, como o FrameView), conversa sobre plugin do G-Assist
  e destaque; NVIDIA Inception se o Assistente virar produto de IA.
- **AMD e Intel:** programas de desenvolvedores para acesso antecipado a drivers e hardware de teste (resolve parte do
  laboratório).
- **Cfx.re:** o FiveM é nosso maior público; um canal oficial com a equipe deles vale ouro (perfis, limites, regras).
- **Fabricantes de periféricos:** listagem nos "apps compatíveis" dos SDKs de RGB.
- **Criadores de conteúdo:** programa de parceiros com código e bancada pública (12).
- **Fabricantes de PC/montadoras brasileiras:** Otimiza pré-instalado ou licença na compra do PC gamer.

---

## 9. Ciência e prova — como provar que somos os melhores

1. **Bancada pública:** 6 máquinas reais (do i3 + GTX 1650 ao Ryzen X3D + RTX 40, uma Radeon, uma Intel Arc, um
   notebook com duas placas), Windows original, rota fixa em 6 jogos.
2. **Comparação justa e cega:** mesmo PC, prova alternada; Windows puro × concorrentes × Otimiza. Métricas: FPS
   médio, **1% pior**, **trancos/min**, **clique → imagem**, **picos de ping/min**, tempo de boot, temperatura.
3. **Publicar tudo:** números, método, arquivos brutos e **onde não ganhamos**. Nenhum concorrente aguenta isso.
4. **Página "Resultados"** no site, alimentada pelos relatórios assinados (opt-in) dos clientes.
5. **Selo de verificação:** o print do Cartão de Prova leva um código; qualquer um confere no site que é real.

---

## 10. Plataforma — conta, nuvem, celular

| Recurso | O que é |
|---|---|
| **Conta Otimiza** (painel web existe) | Licenças, máquinas, histórico de provas, login com Discord |
| Backup de perfis na nuvem (opt-in) | Formatou o PC? Seus perfis de jogo voltam (os ajustes do Windows são reaplicados pelo caminho normal, com medida) |
| **Otimiza no celular** (onda 3) | Segunda tela pela rede local, pareada por QR: FPS, temperatura, ping ao vivo enquanto joga, botão Turbo, mascote. Começa como página web servida pelo próprio Otimiza (sem loja de apps) |
| Multi-PC | Ver os PCs da casa/lan house no painel |
| **Modo LAN house** (B2B) | Perfil padrão aplicado em vários PCs, relatório de saúde de todos |

---

## 11. Fundação de engenharia

| Item | Por quê |
|---|---|
| **Assinatura de código** (comprar já: a verificação de identidade demora) | Sem ela o SmartScreen assusta o comprador no primeiro clique |
| **Atualizador do Tauri assinado** | Atualizar com um clique, conferindo a assinatura |
| **Laboratório de hardware** | Radeon, Intel Arc, RTX 40/50, Ryzen X3D, Intel híbrida, notebook com duas placas, monitor HDR — o PROGRESS lista tudo que nunca rodou por falta de máquina |
| VM com Windows original | Instalar → atualizar → desinstalar limpo em toda versão |
| Relatório de falha opt-in | Saber do travamento antes do suporte |
| CI | `cargo test`, `clippy`, `tsc`, tamanho do pacote, tempo de abertura, `npm run fumaca` |
| Canal beta (existe) | Toda onda passa por ele |
| Servidor do Otimiza | Para a IA (chave fora do executável), regras assinadas, telemetria opt-in e verificação de prova. Mínimo, barato, sem dado pessoal |
| Privacidade | Documento claro do que sai do PC, tudo opt-in, LGPD |

---

## 12. Lançamento, site e comunidade

- **Site novo** com o céu e o mascote, mesmos tokens do app. Herói: o Cartão de Prova real, não promessa.
- **Páginas por jogo** (SEO: "como aumentar FPS no FiveM", "ping alto no Valorant", "monitor 144 Hz preso em 60").
- **Vídeo de lançamento:** o mascote acorda num céu nublado, o PC real roda antes × depois com a linha de frametime na tela.
- **Revelação do mascote** antes do app: teaser no Discord, concurso de nome com a comunidade (se o dono quiser).
- **Beta fechado → beta aberto → 1.0.** Clientes da 3.x recebem a 1.0 como atualização.
- **Programa de criadores** com a bancada pública como material.
- Figurinhas, emojis e papéis de parede de céu para a comunidade.

---

## 13. Roteiro em ondas

Cada onda termina com algo **funcionando e medido** no canal beta.

### Onda 0 — Base (decisões, fundação)
- Versão 4.0.0 com `versao_de_exibicao()` "1.0", teste e gancho do instalador.
- Pasta da nova UI, Svelte 5 + tauri-specta, tokens provisórios, CI de tamanho e abertura.
- Dividir `commands.rs`; juntar duplicatas.
- **Checagem de todas as APIs do capítulo 8** (licença, termos, acesso) → tabela final do que entra.
- Comprar o certificado de assinatura. Abrir contato com os programas de desenvolvedores.
- **Pronto quando:** a UI nova abre chamando 3 comandos tipados e a tabela de APIs está decidida.

**Estado (05/11/2026, branch `nova-geracao/onda-0`):**
| Item | Estado |
|---|---|
| Versão 4.0.0 nos quatro arquivos; `atualizacao::de_exibicao` e `versao_de_exibicao` ("1.0"); comando `versao_do_app`; aviso de atualização escreve a versão exibida | feito, testes passando (`a_nova_geracao_aparece_como_1_0_e_o_passado_fica_igual`, `a_nova_geracao_e_atualizacao_para_quem_esta_na_3`) |
| `DisplayVersion` do instalador | **não mexer** (o instalador do Tauri compara com ele; ver capítulo 0) |
| Casca da UI nova em `pc-optimizer/ui/` (Svelte 5): céu-diagnóstico, mascote em vetor, barra de menu com controles da janela, trilho de jogos, widgets, dock, temas, congelamento com jogo aberto | feito, `npm run nova:build` sem erro; visto no navegador com dados de demonstração. **Não aberto ainda dentro do Tauri** (`npm run nova`) |
| Contrato tipado (tauri-specta): `src-tauri/src/contrato.rs` gera `ui/src/lib/tipos.ts` dos 9 comandos da casca; teste falha se o arquivo ficar velho; a esteira roda o teste e o `svelte-check`. Specta só em `dev-dependencies` (o executável não muda) | feito. Já pegou 13 lugares em que a interface tratava como sempre presente um número que o Rust pode mandar `null` (`f64` NaN/infinito), e uma família de placa inventada na demonstração |
| Manifesto do comctl32 6 pelo linker em todos os alvos (`windows/app.manifest`, mesmo conteúdo do tauri-build): sem ele, teste que toca no Tauri não abre no Windows | feito; conferido que o executável leva o manifesto uma vez |
| Dividir `commands.rs`, juntar duplicatas, checagem das APIs, certificado | a fazer |

### Onda 1 — Marca e casca
- Mascote em Rive com todos os estados, 10 objetos 3D, céus, ícones, bandeja, instalador.
- Casca da interface: céu-diagnóstico, barra de menu, trilho, mesa, widgets, dock, temas, Ctrl+K.
- Componentes-assinatura e o congelamento com jogo aberto (4.5), com teste.
- Início + Histórico/Desfazer completos. Primeira execução.
- **Pronto quando:** o FiveM aberto com o Otimiza na frente não perde FPS medido.

### Onda 2 — Migração total
- Turbo, Jogos, Energia, Sistema, Prova, Ajustes, sobreposição na UI nova.
- `main.ts` e `v33/` apagados.
- **Pronto quando:** tudo da 3.6 existe na UI nova e o `fumaca` clica em todas as áreas.
- **Estado (10/11/2026):** `main.ts`, `v33/` e o link "Ferramentas avançadas" apagados; o instalador sai só com a UI
  nova. `npm run fumaca:nova` passa nas 20 telas. Falta o teste com jogo aberto num PC de verdade. Ficaram de fora,
  por decisão: zerar o modo jogo, plano de renderização, catálogo de jogos e as medições automáticas antigas.

### Onda 3 — Desempenho novo (em paralelo no Rust)
- **Rede:** diagnóstico → controle de banda → Wi-Fi sem varredura → DSCP/DNS.
- **Latência:** clique → imagem na tela, Reflex, taxa do mouse, Caçador de DPC, teste de reflexo.
- **Energia 2.0:** dois estados, calibração sozinha, limites visíveis, guarda térmica.
- **Tela:** Hz, cabo, VRR, HDR.
- **Pronto quando:** cada sistema tem uma prova antes × depois numa máquina real.

### Onda 4 — Integrações e inteligência
- Widget da Xbox Game Bar. CS2 Game State Integration. PresentMon Service API.
- Assistente (camada local + nuvem). Discord Rich Presence. RGB.
- Perfis CS2, Valorant, Fortnite, Roblox, Minecraft.
- Sistema Voando (boot medido, saúde do disco, mover jogo para o SSD).
- **Pronto quando:** o assistente responde "por que meu jogo trava?" com os números certos da última sessão.

### Onda 5 — Prova pública e lançamento
- Bancada pública, site novo, vídeo, notas, programa de criadores.
- Atualizador assinado, relatório de falha, VM limpa.
- **Pronto quando:** números publicados e nenhum defeito grave aberto no beta → **Otimiza 1.0**.

### Depois da 1.0 (1.1, 1.2…)
Otimiza no celular, conta com backup na nuvem, inteligência coletiva com recomendação por máquinas parecidas, modo
LAN house, G-Assist plugin, Optical Flow no Frame Boost, IA local em NPU.

**Paralelismo:** a marca (designer) corre junto com a Onda 0. As frentes da Onda 3 são independentes entre si.
Parcerias (8.10) andam o tempo todo, fora do código.

---

## 14. Metas (KPIs)

| Meta | Como medimos |
|---|---|
| Na bancada, maior ganho médio de **1% pior** entre os otimizadores testados | Bancada pública |
| **−50% de trancos/min** nos PCs com gargalo de processador | Bancada + relatórios opt-in |
| **0 regressões** que fiquem: todo "piorou" desfeito sozinho | Portão + telemetria opt-in |
| Otimiza com jogo aberto: **< 0,5% de um núcleo** | Teste automatizado |
| Abertura **< 1,5 s** num i3 | CI |
| **Monitor em Hz errado corrigido** em X% dos PCs onde existe | Telemetria opt-in |
| Picos de ping/min **−70%** com download em segundo plano | Bancada de rede |
| Pedido de reembolso **< 2%** | Vendas |
| Nota do beta **≥ 9/10** | Pesquisa no Discord |

---

## 15. O que continua fora

- Defender, UAC, SmartScreen, DEP desligados; IPv6 desligado; "limpeza de registro"; liberar RAM à força; chaves
  placebo (`QuantumLength`, `TimeIncrement`, `D3D12_*`); watchdog/DPC alterados; ativador.
- **Driver de kernel próprio**, e também bibliotecas que instalam um (algumas de sensores usam o WinRing0, que o
  Defender passou a marcar como vulnerável): sensores só por APIs dos fabricantes, WMI e contadores do Windows.
- Injeção em processo de jogo. Janela por cima de jogo com anticheat que proíbe (Vanguard, FACEIT): lá, só o widget da
  Game Bar e o celular.
- "Ping booster" que promete diminuir a distância até o servidor.
- Prioridade tempo real no jogo; matar processo do usuário.
- IA que aplica coisas sozinha.

---

## 16. Riscos

| Risco | Como tratamos |
|---|---|
| Vidro e animação pesarem no jogo | Congelamento com jogo aberto, desfoque pré-calculado, modo leve, teste na CI |
| Plano grande demais, nada sai | Ondas com "pronto quando"; cada onda entra no beta; a 3.6 continua vendável |
| API de empresa mudar ou exigir acordo | Checagem na Onda 0; nenhum recurso essencial depende de uma API só; plano B escrito para cada uma |
| Assistente inventar número | Só recebe o resumo medido; o servidor corta número que não veio da medição; nunca age sozinho |
| Custo da IA na nuvem | Camada local responde a maioria; Haiku por padrão; limite por licença |
| Controle de banda/Wi-Fi ficar preso | Estado gravado antes; conferência na abertura e no início do Windows |
| Anticheat reclamar | `anticheat::permite` obrigatório; widget da Game Bar como caminho seguro |
| Hardware que não temos | Laboratório e programas de desenvolvedores; sem máquina de teste = "experimental", desligado por padrão |
| Mascote ficar infantil | Os olhos são a única expressão; tom de voz técnico e direto; tema Noite para quem quer sério |

---

## 17. Decisões que ainda são do dono

1. **Nome do mascote** (Oti, Bolha, Pixel, Gota, concurso com a comunidade?).
2. **Designer:** quem faz o mascote em Rive, os objetos 3D e os céus (interno, freelancer, estúdio)?
3. **Framework da UI:** Svelte 5 (recomendado)?
4. **Assistente com IA na nuvem já na 1.0** ou só a camada local na 1.0 e a nuvem na 1.1?
5. **Clientes da 3.x:** 1.0 grátis para todos?
6. **Telemetria opt-in** na 1.0?
7. **Orçamento** de hardware do laboratório e do certificado (OV ou EV).
8. **Quais parcerias** procurar primeiro (sugestão: Cfx.re, NVIDIA, AMD).
9. **Tema padrão:** Céu claro (como a referência) ou Noite, para jogador que joga no escuro?
