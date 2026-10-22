# Otimiza 3.5 — Turbo: o jogo mais leve que o cliente sente

> Estado lido: `main` em `ddc71b5` (3.4.0), 431 commits desde setembro.
> ~100 mil linhas de Rust, interface em `src/v33/` (2 mil linhas) com a antiga
> ainda ao lado. Branch deste trabalho: `feature/3.5-escala`.
>
> **Nenhuma linha de produto foi alterada para escrever este documento.** O
> único código já escrito é `geracao/escala.rs` (puro, com testes), o primeiro
> passo do marco M1.

---

## 0. Resumo em uma tela

**A dor:** o Otimiza trabalha muito, e o cliente não sente o jogo mais leve.

**A causa:** o FPS não está onde o Otimiza mexe. O próprio código já mediu isso:

- `configjogo.rs`: a configuração do jogo pesa *dezenas* de por cento, e os
  ajustes de Windows somados pesam *alguns*.
- `ritmo_estavel.rs`: no FiveM do dono, a mediana é de 45 fps, mas há 74
  trancos por minuto. O que incomoda é a oscilação, não a média.
- `memoria_jogo.rs`: num PC de 8 GB, o tranco vem da memória acabando, e nenhum
  ajuste de registro resolve isso.

**O que já existe e está solto:** gerador de quadros próprio, teto de FPS pelo
driver (Jogo Liso), energia de jogo, memória para o jogo, receita por gargalo,
contador por cima do jogo, configuração do GTA/FiveM, NVAPI e ADLX. São botões
separados, e o cliente precisa saber qual apertar.

**O que falta:** a alavanca que mais rende numa placa de vídeo no limite, que é
**renderizar menos pixels e ampliar**. O sucesso do Lossless Scaling vem
exatamente de juntar escala + geração.

### A 3.5 em uma frase
> **Um botão, Turbo, que mede a máquina e o jogo, escolhe a receita certa
> (escala, geração, teto, energia, memória, configuração), aplica e mostra
> o antes × depois medido.**

### Três decisões que guiam tudo
1. **A receita depende do gargalo.** Com a placa no limite, a escala dá FPS
   real. Com o processador no limite (o caso do PC do dono, um i3-10100F), a
   escala não adianta: aí entram geração + teto estável + energia + memória. O
   `gargalo.rs` já classifica, e o Turbo só obedece.
2. **Tudo por fora do jogo.** Nada de injeção, hook ou leitura de memória. Isso
   é o que deixa o Otimiza seguro com Vanguard, EAC, BattlEye e VAC, e é
   argumento de venda.
3. **Só número medido.** A regra de sempre: FPS do jogo (renderizado) e FPS na
   tela (exibido) separados, e "não houve ganho" dito quando não houver.

---

## 1. Por que o cliente não sente hoje (o diagnóstico em detalhe)

| Sintoma que o cliente descreve | Causa real | Quem resolve | Otimiza hoje |
|---|---|---|---|
| "FPS baixo" com a placa a 99% | pixels demais | **escala** | não tem |
| "FPS baixo" com o processador no limite | CPU | geração + energia + memória | geração e energia existem, soltas |
| "Trava mesmo com FPS alto" | oscilação de ritmo | teto estável (Jogo Liso) | existe, exige reabrir o jogo |
| "Trava ao girar a câmera" | memória acabando | fechar o que ocupa | existe, manual |
| "Não sinto diferença nenhuma" | o ganho não aparece na tela | prova visível ao vivo | prova existe, escondida |

A última linha importa tanto quanto as outras: **ganho que o cliente não vê é
ganho que ele não paga.**

---

## 2. Pilar A — Escala no gerador

### 2.1 Como funciona
1. O jogo roda **em janela sem bordas, menor que o monitor** (por exemplo
   1280×720 num monitor 1920×1080).
2. A captura continua igual: Duplicação de Área de Trabalho recortando a área
   do cliente do jogo (`area_do_cliente`). A sobreposição fica fora da captura
   (`WDA_EXCLUDEFROMCAPTURE`), então o jogo pequeno é visto limpo mesmo
   coberto.
3. A sobreposição passa a cobrir **o monitor inteiro** (`escala::encaixe`
   decide onde cai a imagem, com barras quando a proporção é outra).
4. A cadeia de GPU **continua na resolução do jogo**: fluxo ótico e
   interpolação ficam até mais baratos que hoje. Só o último passo muda:
   `compor` escreve num alvo intermediário e um passe novo amplia para a swap
   chain.
5. **Escala também funciona sozinha, sem geração** (multiplicador 1). É o caso
   de quem não quer atraso nenhum: só o FPS real a mais. O laço de hoje já
   apresenta só os reais quando cabe 1×; falta permitir o 1. Sem geração, o
   `receber` pula a pirâmide de luma e o fluxo, e só copia a imagem: a escala
   sozinha custa a cópia + os dois passes do FSR, nada mais.

### 2.1.1 Quando a sobreposição some (regra nova)
Hoje a sobreposição **some** com 0,5 s sem quadro novo, na pausa da guarda e
com o jogo fora da frente: "nunca uma imagem congelada por cima do jogo". Com a
escala, sumir mostraria o jogo **pequeno** no meio do monitor, e uma tela
parada do jogo (menu, carregamento) é legítima. Por isso:

| Situação | Sem escala (hoje) | Com escala |
|---|---|---|
| 0,5 s sem quadro do jogo | some | **fica**, com o último real ampliado |
| Pausa da guarda | some, sem captura | **fica**, só com os reais ampliados (sem geração) |
| Jogo fora da frente | some | some |

A guarda continua medindo o custo da **geração**; na escala ela compara "com
geração" × "só escala", o que é exatamente a decisão que o cliente precisa.

### 2.2 Ampliador
- **FSR 1** (EASU para ampliar + RCAS para nitidez), código aberto da AMD (MIT),
  portado para `shaders.hlsl` em compute. É o padrão.
- **Nítido leve** (bilinear + RCAS): para placa integrada ou quando a guarda
  pedir alívio, no mesmo mecanismo do `modo_leve` que já existe.
- **HDR:** o EASU espera espaço perceptual. Na captura FP16 (scRGB linear), o
  passe converte para gama antes e volta depois. Se a medida mostrar custo alto,
  o HDR vai de "Nítido leve" na 3.5.
- O **NIS da NVIDIA** (também aberto, MIT) fica como alternativa possível para a
  3.6. Não entra agora: dois ampliadores na mesma versão dobram o teste.

### 2.3 Níveis (nomes e proporções do FSR, que o jogador já conhece)
| Nível | Escala por eixo | Pixels renderizados | 1080p renderiza em |
|---|---|---|---|
| Ultra qualidade | 1,3× | 59% | 1477×831 |
| Qualidade *(padrão)* | 1,5× | 44% | 1280×720 |
| Equilibrado | 1,7× | 35% | 1129×635 |
| Desempenho | 2,0× | 25% | 960×540 |

### 2.4 A janela do jogo
- O Otimiza **redimensiona a janela do jogo** para o nível (`SetWindowPos`),
  grava o tamanho antigo e devolve ao desligar. Isso segue a regra da casa: tudo
  reversível, com o valor exato.
- Jogo que repõe o próprio tamanho: o Otimiza percebe em 2 s e mostra qual
  resolução escolher no menu do jogo, com o nome que o jogo usa.
- **Jogos DX11 em janela:** **não** religar a "otimização para jogos em
  janela" (`SwapEffectUpgradeEnable`). A 3.3.1 mediu pelo PresentMon que ela
  não tira o jogo da apresentação por cópia, e o Frame Boost deixou de usá-la.
  Na escala, o PresentMon diz o modelo de apresentação do jogo, e a tela avisa
  quando ele estiver em cópia (mais atraso).

### 2.5 Cursor e entrada
O cursor do Windows fica na posição da janela pequena, e a imagem está
ampliada. A solução tem três partes:
- esconder o cursor do sistema sobre o jogo e desenhá-lo na sobreposição, na
  posição de `escala::jogo_para_tela`;
- limitar o mouse à janela do jogo (`ClipCursor`), devolvido ao desligar;
- jogos de mira (raw input) não são afetados. Menus e jogos de mouse
  (estratégia, MOBA) precisam do cursor desenhado, por isso ele faz parte do
  M2, não é extra.

### 2.6 Orçamento de desempenho (metas, a medir no laboratório)
| Item | Meta |
|---|---|
| Custo do EASU + RCAS por quadro exibido, 1080p, placa de entrada | ≤ 0,8 ms |
| Ganho de FPS do jogo, nível Qualidade, jogo preso na placa | ≥ +35% |
| Escala + geração 2×, FPS na tela contra o nativo | ≥ 2,4× |
| P99 do intervalo de quadros | não piora |
| Qualidade (SSIM contra o quadro nativo) | maior que o bilinear em todos os quadros gravados |

Se uma meta não for atingida, **a versão diz o número que deu** e não promete o
alvo.

---

## 3. Pilar B — Escala pelo driver (para tela cheia exclusiva)

Para quem joga em tela cheia exclusiva ou não quer sobreposição:

- **NVIDIA Image Scaling** pela NVAPI DRS (`nvdriver.rs`): ligar e ajustar a
  nitidez. Mesmo cuidado do módulo: confere o nome do número com
  `NvAPI_DRS_GetSettingNameFromId` antes de escrever, e só entra com
  restauração de padrão.
- **AMD Radeon Super Resolution** pelo ADLX (`amddriver.rs`): o serviço de
  ajustes 3D tem leitura e escrita oficiais de RSR. A posição na vtable sai do
  cabeçalho do SDK, como as outras.
- **Intel:** fica de fora. Não há API pública.
- A tela diz o limite: os dois só ampliam quando **o jogo** está em tela cheia
  numa resolução menor. O Otimiza liga o recurso e mostra qual resolução
  escolher no jogo.

---

## 4. Pilar C — O Turbo: a receita automática

### 4.1 O fluxo
1. **Medir 20 s** do jogo aberto (PresentMon + carga de CPU e GPU), com a
   regra de sempre: acima de 25% de CPU antes do jogo, nada de veredito.
2. **Classificar o gargalo** (`gargalo.rs`, que já existe).
3. **Montar a receita** (`receita.rs` já liga causa → ajuste e lembra o que
   piorou cada jogo; o Turbo é uma camada fina por cima):

| Gargalo | Receita do Turbo |
|---|---|
| Placa de vídeo | Escala (nível pelo FPS alvo) + geração se ainda faltar + teto estável |
| Processador | Geração + teto estável + energia de jogo + memória (oferece fechar) |
| Memória acabando | Memória primeiro; o resto depois de medir de novo |
| Ritmo oscilando | Teto estável primeiro |
| Já liso e rápido | **"Seu jogo já está no limite desta máquina"**, e não liga nada |

4. **Escolher o nível sozinho:** o menor nível de escala cujo FPS estimado
   alcança o alvo (Hz do monitor ÷ multiplicador). A estimativa é marcada
   ESTIMADO até a medida do depois.
5. **Aplicar, medir de novo e mostrar** (pilar E).
6. **Lembrar por jogo:** a receita aprovada vira o perfil daquele jogo naquela
   máquina. Na próxima vez que o jogo abrir, o Turbo oferece ligar com um
   clique (ou liga sozinho, se a pessoa marcar).

### 4.2 O que não muda
- Cada peça continua disponível sozinha para quem quiser controle (as telas de
  hoje viram o "modo avançado").
- Receita que piorou o jogo não volta a ser oferecida (a memória do
  `receita.rs`).

---

## 5. Pilar D — Configuração por jogo + REA

### 5.1 Configuração
O `configjogo.rs` provou o padrão em GTA V / FiveM: escrita única, arquivo
inteiro guardado, recusa com o jogo aberto. A 3.5 generaliza:

- `configjogo/` vira pasta: um **formato** por motor (INI da Unreal
  `GameUserSettings.ini`, `video.txt` da Source 2, XML do RAGE) e um **perfil**
  por jogo em cima dele.
- **Primeira leva** (pelos jogos que os clientes abrem, que saem de
  `plataforma/jogos.rs`): Fortnite, Valorant, CS2, Warzone, Apex.
- Cada perfil marca o que custa caro (MSAA, sombras, distância de visão,
  resolução de render) com o ganho **medido por terceiros**, como o
  `AjusteCaro` já faz. Se o jogo tem escala própria (FSR/DLSS no menu), o Turbo
  **prefere a do jogo**, que é melhor que qualquer ampliação por fora, e diz
  isso ao cliente.

### 5.2 Onde o REA entra (bancada do desenvolvedor, nunca no cliente)
O REA (engenharia reversa) acelera o trabalho de **entender formatos**:
- descobrir chaves de configuração escondidas e os valores aceitos;
- saber o que o jogo relê ao abrir e o que ele sobrescreve;
- confirmar se o jogo aceita o redimensionamento da janela (2.4) ou repõe o
  tamanho.

O resultado de cada investigação vira uma **ficha** em
`docs/jogos/<jogo>.md` (o que mexe, onde, o que o jogo repõe, a fonte) e o
perfil no código cita a ficha. O REA roda **na máquina de desenvolvimento,
contra arquivos e executáveis locais**. Nada do produto injeta, faz hook ou lê
memória de jogo. O `anticheat.rs` continua sendo a trava, e um teste novo
garante que nenhum perfil de configuração toque jogo protegido fora do arquivo
de configuração.

---

## 6. Pilar E — A prova que o cliente sente

1. **Cartão do resultado** ao desligar ou fechar o jogo, a partir do
   `geracao/resumo.rs` e da prova que já existem: "Valorant — FPS do jogo
   142 → 191, na tela 382, trancos −60%". Tem botão de compartilhar
   (`compartilhar.rs` já existe), e propaganda feita pelo cliente vale mais que
   qualquer anúncio.
2. **Comparador de imagem:** o gerador já grava quadros
   (`gravar_proximos_quadros`). A tela mostra lado a lado, com um divisor
   arrastável, o quadro nativo e o ampliado. O cliente vê que a imagem continua
   boa, e é isso que faz ele manter a escala ligada.
3. **Contador por cima do jogo** (`overlay.rs`) ganha a linha do Turbo: "jogo
   75 · tela 150 · escala Qualidade", só com números medidos, como hoje.

---

## 7. Pilar F — A interface da 3.5

Escopo fechado: **a tela do jogo e o Turbo**, não o app inteiro.

- A tela do jogo aberto (`sessao.rs` já a alimenta) ganha **um botão grande,
  Turbo**, com o estado em linguagem de jogador: "Medindo… → Placa no limite →
  Escala Qualidade + 2× → +38% no jogo".
- O Frame Boost ganha o seletor de **nível de escala** (2.3) e a linha
  "jogo 1280×720 → tela 1920×1080".
- O cartão do resultado e o comparador (pilar E).
- Movimento e acabamento seguem `MOTION.md` e a identidade da 3.4 (logo nova).
- O resto da migração para `src/v33/` fica para a 3.6.

---

## 8. Pilar G — Casa arrumada (só onde a 3.5 encosta)

A faxina inteira é a 3.6. Na 3.5, a regra é **deixar melhor cada arquivo que
for tocado**:
- os comandos do Frame Boost e do Turbo saem do `commands.rs` (5,4 mil linhas)
  para `commands/frameboost.rs`;
- decidir o destino do `framegen.rs` (o laboratório antigo, 2 mil linhas) ao
  lado de `geracao/`: o que ainda for usado migra, o resto sai;
- comentário que conta história vira uma linha; a história vai para o commit;
- o `docs/app/PROGRESS.md` (que ainda fala da 1.8) é reescrito para a 3.5;
- testes novos sempre dentro de prefixo que o `release.yml` já executa (a trava
  do `ci_coverage.rs`).

---

## 9. Fora da 3.5
- Linux: só depois de a escala estar provada no Windows. macOS fica fora, porque
  não há o que otimizar para jogos ali.
- NIS como segundo ampliador, geração com modelo treinado (estilo LSFG 3):
  3.6+.
- Faxina geral do repositório e CI de lint em todo PR: 3.6.

---

## 10. Marcos

Cada marco termina **funcionando e medido**, nunca pela metade.

| Marco | Entrega | Pronto quando |
|---|---|---|
| **M1 — Escala liga** | `escala.rs` ✅, EASU + RCAS, `gpu::ampliar`, sobreposição no monitor, escala sem geração | jogo de teste a 720p aparece ampliado em 1080p; teste de SSIM passa; custo medido |
| **M2 — Escala jogável** | redimensionar a janela + devolver, cursor desenhado, `ClipCursor`, aviso de apresentação por cópia, escala + geração juntas | uma partida inteira de 3 jogos reais sem cursor errado; P99 não piora |
| **M3 — Driver** | NIS (NVIDIA) e RSR (AMD) com restauração de padrão | aplicar → reabrir → desfazer idêntico, nas duas marcas |
| **M4 — Turbo** | receita por gargalo, nível automático, perfil lembrado por jogo, tela do Turbo | jogo preso na placa e jogo preso no processador recebem receitas diferentes e ambas provam ganho (ou dizem que não houve) |
| **M5 — Configuração** | `configjogo/` por motor, 5 jogos com ficha | cada perfil escreve, desfaz byte a byte e recusa com o jogo aberto |
| **M6 — Prova e lançamento** | cartão do resultado, comparador, contador, notas da versão, medição A-B-A-B no laboratório | `verificador-de-publicacao` dá "pode publicar" |

Ordem possível em paralelo: M3 e M5 não dependem de M1/M2 e podem andar ao
lado deles.

### 10.1 M1 em detalhe (a ordem de implementação)

| # | Arquivo | O quê |
|---|---|---|
| 1 | `geracao/escala.rs` | ✅ níveis, encaixe, cursor (puro, 9 testes) |
| 2 | `geracao/shaders.hlsl` | ✅ `Easu` e `Rcas` como passes de pixel, lendo com `Load` (12 e 5 leituras, sem `Gather`): o mesmo kernel do `ffx_fsr1.h`, legível e sem as macros do SDK |
| 3 | `geracao/gpu.rs` | ✅ `passe_na_area` (viewport com deslocamento); `preparar_ampliacao(l, a)` cria o alvo do tamanho do jogo e o ampliado; `compor_ampliado` = `compor` no intermediário **codificado** (no HDR, a `Sair` só roda no fim) → `Easu` → `Rcas` na área do encaixe, barras em preto; `sem_geracao` faz o `receber` só copiar |
| 4 | `geracao/mod.rs` | ✅ `Configuracao.escala: Option<Nivel>`; multiplicador 1 aceito com escala; sobreposição no monitor inteiro; regras de 2.1.1; a sobreposição fica escondida até o primeiro real (senão cobriria o monitor de preto) |
| 5 | `plataforma/frameboost.rs` + `commands.rs` + `v33/ponte.ts` | ✅ a escala chega do comando de ligar (campo opcional: chamadas antigas continuam valendo); só escala não trava o FPS base (travar tiraria o ganho real) |
| 6 | `gpu.rs` (teste `#[ignore]`) | ✅ `escala_na_placa`: medido no notebook (21/10), 640×360 → 960×540, **FSR SSIM 0,990 / PSNR 29,1 dB contra bilinear 0,953 / 23,5 dB**; barras pretas e a área do encaixe idêntica |
| 7 | `LICENCAS-DE-TERCEIROS.md` | ✅ a licença MIT do FSR 1 |
| 8 | interface | ✅ seletor **Escala** (desligada é o padrão até o M2 redimensionar a janela sozinho) com a resolução que o jogo deve usar no monitor dele (`escala::opcoes`); "Só escala" no multiplicador; ligado: selo do nível e "jogo 1280×720 → tela 1920×1080"; só escala não mostra a comparação da guarda (não há geração para comparar) |
| 9 | PC de casa | `sobre_o_jogo_de_teste` com `OTIMIZA_FG_ESCALA=Qualidade OTIMIZA_FG_MULT=1` e com `OTIMIZA_FG_MULT=2`: custo medido na placa e FPS do jogo nativo × escala |

O teste de GPU é `#[ignore]`, como os outros da placa: a esteira roda em
máquina sem placa de vídeo. Ele roda à mão no notebook e no PC de casa.

### 10.2 M2 em detalhe

| # | Onde | O quê |
|---|---|---|
| 1 | `mod.rs` `JanelaNoNivel` | ✅ com escala, a janela do jogo vai para a resolução do nível, centrada no monitor dela (`escala::janela_no_nivel`, conta a borda da janela). Maximizada sai do maximizado. **Volta exatamente como estava** quando o gerador sai, por qualquer caminho (é uma guarda com `Drop`) |
| 2 | `mod.rs` `aviso_da_escala` | ✅ jogo que repõe o tamanho do monitor: a tela diz qual resolução escolher no menu dele |
| 3 | `cursor.rs` `MousePreso` | ✅ `ClipCursor` na área do jogo enquanto a sobreposição está na tela, refeito a cada 0,5 s (trocar de janela desfaz); devolve o recorte que havia antes (o jogo pode ter o dele) |
| 4 | `cursor.rs` `forma_em_bgra` + `mod.rs` | ✅ a forma e a posição do cursor vêm da própria captura (`GetFramePointerShape`, `PointerPosition`); os três formatos (monocromático, colorido, com máscara) viram BGRA; o que inverte a tela sai branco |
| 5 | `gpu.rs` + `shaders.hlsl` `Cursor` | ✅ o cursor desenhado ampliado por cima da imagem, alfa pré-multiplicado (sem franja escura); com o jogo parado e o mouse andando, o mesmo real é reapresentado com o cursor no lugar novo |
| 6 | `cursor.rs` `CursorDoSistema` | ✅ o cursor do Windows escondido (API de Lupa, `MagShowSystemCursor`) só enquanto a sobreposição está na tela; devolvido ao esconder e ao sair |
| 7 | testes | ✅ 9 puros novos (janela, cursor, formas) e `cursor_na_placa` (GPU): o opaco cobre, o transparente deixa a imagem intacta |
| 8 | PC de casa | **medir com o jogo de teste e 3 jogos reais**: janela redimensionada e devolvida; cursor no lugar em menu; o mouse não escapa; o cursor do sistema volta ao desligar, ao fechar o jogo e com o Otimiza fechado à força (Gerenciador de Tarefas) |
| 9 | depois | aviso de apresentação por cópia (PresentMon); monitor girado sem cursor ampliado |

Risco a confirmar no item 8: se o Otimiza morrer à força com o cursor
escondido, o Windows precisa devolvê-lo (a Lupa é do processo). Se não
devolver, entra um vigia que mostra o cursor na abertura seguinte.

### 10.3 M3 em detalhe

| # | Onde | O quê |
|---|---|---|
| 1 | `nvdriver.rs` `escala_nis` | ✅ `NV_QUALITY_UPSCALING_ID = 0x10444444` ("NVIDIA Quality upscaling", ON = 1, OFF = padrão), conferido no `NvApiDriverSettings.h` público. Entra no catálogo global: herda a trava do nome (o driver diz o nome do número antes de qualquer escrita), o valor anterior no histórico e o desfazer pela restauração de padrão da NVIDIA |
| 2 | `amddriver.rs` `Opcao::Rsr` | ✅ `GetRadeonSuperResolution` na posição 14 dos serviços 3D (global, sem a placa no argumento) e `IsSupported`/`IsEnabled`/`SetEnabled` em 3/4/5, conferidos no `I3DSettings.h` do ADLX; anterior guardado e desfeito como o Anti-Lag |
| 3 | tela | ✅ a NVIDIA aparece sozinha (o painel vem do catálogo); a AMD ganha a linha "Escala do driver (Radeon Super Resolution)" só quando o driver diz que suporta. A mensagem de tela cheia exclusiva do Frame Boost aponta para a escala do driver |
| 4 | testes | ✅ o número público do NIS fixado; o RSR vai e volta pelo texto do histórico; a posição 14 entra na guarda das tabelas do ADLX |
| 5 | PC de casa | **aplicar → reabrir o jogo em tela cheia numa resolução menor → desfazer**, nas duas marcas: o driver amplia, o FPS sobe, e o desfazer devolve o valor de antes. O teste `os_numeros_da_tabela_batem_com_o_que_o_driver_chama` (NVIDIA) confere o 0x10444444 contra o driver de verdade. O notebook só tem Intel |

### 10.4 M4 em detalhe

| # | Onde | O quê |
|---|---|---|
| 1 | `plataforma/turbo.rs` (puro) | ✅ `gargalo_de` (o `Limite` do `windows::bottleneck` vira Placa, Processador, Memória, Nenhum ou Sem carga); `montar` (a receita da tabela 4.1); `nivel_automatico` (o nível mais bonito cujo FPS estimado alcança o alvo: o monitor só com escala, metade dele com geração; expoente 0,7 nos pixels, para não prometer demais); `competitivo` (CS2, Valorant, Fortnite, CoD, Apex, Rocket League: sem geração) |
| 2 | `turbo.rs` medir | ✅ 10 s com os contadores do Windows (`bottleneck::analisar`) e o PresentMon (FPS do jogo) ao mesmo tempo |
| 3 | `turbo.rs` aplicar/desligar | ✅ energia de jogo e Frame Boost (escala/geração) pelas peças que já existiam; passo que falha (anticheat, sem administrador) vira nota e os outros seguem; desligar devolve só o que o Turbo ligou (a energia, se foi ele) |
| 4 | perfil por jogo | ✅ `%APPDATA%\pc-optimizer\turbo.json`, pelo id do jogo; na próxima vez, **Turbo** aplica a receita lembrada sem medir, com **Medir de novo** e **Esquecer** ao lado |
| 5 | comandos | ✅ `turbo_ligar` (exige licença), `turbo_painel`, `turbo_desligar`, `turbo_esquecer` (livres); o FPS base trava como no Frame Boost (e não trava no só escala) |
| 6 | tela | ✅ cartão Turbo no topo do Boost: o que mediu da última vez, em linguagem de jogador; os passos do que ligou; erro visível sem apagar |
| 7 | testes | ✅ 12 puros (gargalos, nível, receitas, competitivos no catálogo, perfil pelo arquivo) |
| 8 | **fica para depois** | ligar sozinho ao abrir o jogo (decisão 2 do dono, desligado por padrão); o antes × depois medido no cartão (precisa da medida do depois, pilar E); receita que piorou não volta (memória do `receita.rs`) |
| 9 | PC de casa | jogo preso na placa e jogo preso no processador recebem receitas diferentes, e as duas provam ganho (ou dizem que não houve) |

### 10.5 M5 em detalhe

Mudança de forma em relação à seção 5.1: em vez de transformar o
`configjogo.rs` em pasta (um teste dele lê o próprio arquivo), os motores
novos moram ao lado, em `modules/windows/configmotor.rs`. O GTA V / FiveM
continua no `configjogo.rs`.

| # | Onde | O quê |
|---|---|---|
| 1 | `configmotor.rs` formatos | ✅ INI da Unreal (`chave=valor` dentro de `[secao]`) e KeyValues da Valve/Respawn (`"chave" "valor"`), puros: trocam só o valor, o resto do arquivo fica byte a byte (inclusive `\r\n`) |
| 2 | **trava nova** | ✅ só troca chave que **já existe** no arquivo do jogo; chave com nome errado no perfil não escreve nada |
| 3 | perfis | ✅ Fortnite (sombras, pós-processamento, efeitos, vegetação), CS2 (sombras, oclusão, partículas; **nunca** as sombras dinâmicas, que entregam posição) e Apex (oclusão, luz volumétrica) |
| 4 | aplicar | ✅ recusa com o jogo aberto; o arquivo inteiro de antes vai para o histórico (`ChangeRecord::GameConfig`) e o `revert_optimization` devolve byte a byte; registrado no `registro.rs` |
| 5 | fichas | ✅ `docs/jogos/` com as cinco: Fortnite, CS2 e Apex escrevem; **Valorant e Warzone não escrevem** (a Riot sincroniza pela conta; o formato da Activision muda a cada jogo), cada uma com o que confirmar no REA |
| 6 | tela | ✅ cartão "Configuração do jogo" no Boost: só os jogos achados, a prévia de cada mudança com o porquê, aplicar e desfazer, botão desligado com o jogo aberto |
| 7 | testes | ✅ 9 (formatos, só o que existe, idempotente, catálogo e fichas, CS2 sem tocar nas sombras dinâmicas); o módulo entra no passo do `release.yml` |
| 8 | **PC de casa + REA** | confirmar os nomes das chaves do CS2 e do Apex na versão atual; medir o ganho antes × depois nos três; estudar Valorant e Warzone |

### 10.6 M6 em detalhe

| # | Onde | O quê |
|---|---|---|
| 1 | cartão do resultado (`turbo.rs` + tela) | ✅ ao desligar o Turbo com o Frame Boost: FPS do jogo antes → depois, o ganho em %, o que chegou à tela (com geração) e a escala. **Com a ressalva escrita**: o antes são os 10 s da medida e o depois a média da sessão, cenas diferentes, não é prova A-B. Por isso **não** tem botão de compartilhar: o `compartilhar.rs` só aceita prova alternada com margem |
| 2 | comparador (`geracao` + tela) | ✅ "Comparar a imagem" no Frame Boost com escala: o próximo quadro real **com o jogo na frente** (com o Otimiza na frente a captura seria a dele) vira o terço central do quadro do jogo e o mesmo pedaço ampliado pelo FSR; a tela mostra os dois com um divisor. O lado do jogo é ampliado do jeito comum (filtro do navegador), não em pixels quadrados: comparar com o pior caso seria propaganda |
| 3 | contador (`overlay.rs`) | ✅ no Detalhado, "tela N" com geração e "escala Qualidade"; no só escala, nada de "1×" |
| 4 | notas | ✅ `.github/release-notes.md` com a 3.5 no topo e `.github/release-titulo.txt` = "Otimiza 3.5 Turbo". O número da versão (Cargo, tauri.conf, package.json, site) **não** sobe aqui: é do ritual de publicação |
| 5 | testes | ✅ 3 do resultado, 3 do comparador (regiões, recorte, BMP), 1 do contador; o `escala_na_placa` agora prova na placa que o quadro de origem do comparador é o do jogo |
| 6 | **PC de casa** | a medida A-B-A-B do laboratório (nativo × escala × escala + geração) e a prova alternada que libera o compartilhar |

### 10.7 Depois do M6: a prova, a rede de segurança e o Turbo na tela do jogo

| # | Onde | O quê |
|---|---|---|
| 1 | `turbo.rs` `provar` | ✅ **a prova A-B do Turbo**: 8 rodadas na ordem da `provaalternada` (A B B A B A A B), 30 s de aquecimento, 6 s de espera (a janela redimensiona) e 30 s de medida pelo PresentMon em cada uma: uns 5 minutos jogando. Decide pelo critério único da `provaalternada` (margem de 95%) |
| 2 | o que entra | ✅ só o que deve subir o **FPS do jogo**: a escala (sem gerar) e a energia de jogo. A geração aumenta o que chega à tela e custa do jogo; pelo FPS do jogo uma geração boa sairia "piorou", então ela fica com a guarda do gerador |
| 3 | desfecho | ✅ **Piorou**: o Turbo fica desligado e a receita não volta neste jogo (só "Medir de novo"). Senão, a receita inteira volta, com a geração. A prova reserva a mesma vez da `provaalternada`: a medição automática, o guardião e o motor de energia esperam |
| 4 | `medicoes::gera_quadros` | ✅ correção: o Frame Boost só com escala não conta mais como gerador de quadros (anularia toda comparação com e sem a escala) |
| 5 | compartilhar | ✅ `compartilhar.rs` com rótulos; o texto do plano de energia ficou igual, e o do Turbo diz "sem o Turbo × Turbo: escala … + energia de jogo". "Copiar resultado" no cartão |
| 6 | rede de segurança | ✅ na abertura: o cursor do Windows à vista; a janela do jogo devolvida pelo marcador `escala_janela.json` se o Otimiza caiu com a escala ligada; a energia de jogo devolvida se caiu no meio da prova |
| 7 | tela do jogo | ✅ com o jogo aberto, **Turbo** é o botão principal da tela inicial (receita lembrada: um clique; sem ela: mede); "Otimizar agora" fica ao lado |
| 8 | testes | ✅ 7 novos, inclusive uma prova inteira simulada (8 rodadas → Ganhou → texto do Turbo) |
| 9 | **PC de casa** | a prova de verdade num jogo preso na placa (escala) e num preso no processador (energia) |

## 11.1 Antes de publicar a 3.5 (tudo o que ficou para o PC de casa)

1. **Escala ao vivo** (10.1 #9, 10.2 #8): jogo de teste e 3 jogos reais; janela redimensionada e devolvida; cursor e
   mouse; o cursor do sistema volta até com o Otimiza fechado à força.
2. **Driver** (10.3 #5): NIS e RSR aplicados, jogo reaberto em tela cheia menor, desfeitos; o
   `os_numeros_da_tabela_batem_com_o_que_o_driver_chama` numa NVIDIA.
3. **Turbo** (10.4 #9, 10.7 #9): um jogo preso na placa e um preso no processador, com a prova A-B nos dois.
4. **Configuração** (10.5 #8): as chaves do CS2 e do Apex com o REA; o ganho nos três.
5. **Laboratório** (10.6 #6): A-B-A-B e as metas da seção 2.6, com os números que derem.
6. **Ritual**: subir a versão para 3.5.0 nas quatro declarações e rodar o `verificador-de-publicacao`.

---

## 11. Riscos

| Risco | Impacto | Resposta |
|---|---|---|
| Jogo recusa janela pequena | escala não liga nele | orientar a resolução no menu; perfil por jogo lembra |
| Cursor em jogo de mouse | jogo inutilizável | cursor desenhado é parte do M2, não extra |
| Custo do ampliador em placa integrada | perde o que ganhou | "Nítido leve" + guarda que já desliga quando o jogo perde FPS |
| Anticheat estranhar a sobreposição | ban | nada novo dentro do jogo; mesma janela de hoje, já medida |
| Prometer número que não vem | cliente sente-se enganado | metas internas; a tela só mostra medido |
| Atraso da geração | jogador competitivo reclama | Turbo competitivo = só escala + teto, sem geração |

---

## 12. Decisões que são do dono

1. **Nome comercial do botão:** "Turbo" é a proposta.
2. **Ligar sozinho ao abrir o jogo:** desligado por padrão (proposta) ou ligado?
3. **Nível padrão da escala:** Qualidade (proposta).
4. **Perfil competitivo:** jogos de tiro marcados como competitivos ficam sem
   geração por padrão (proposta).
5. **Quais 5 jogos** entram primeiro na configuração (proposta: Fortnite,
   Valorant, CS2, Warzone, Apex).

---

## 13. Ambiente

- Notebook: Rust 1.99 (MSVC), Build Tools C++ 2022, Node 20 e Git instalados
  em 21/10/2026. `cargo test --lib` compila e roda aqui (os 9 testes do
  `escala.rs` passam); o que precisa de placa de vídeo é medido no PC de casa.
- O repositório não está formatado pelo `rustfmt` padrão (há diferença em
  arquivos antigos). Código novo sai formatado; formatar o resto é da faxina
  da 3.6, num commit só de formatação.
- PC de casa: REA e a bancada de medição.
