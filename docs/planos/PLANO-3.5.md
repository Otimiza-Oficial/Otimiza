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
   de quem não quer atraso nenhum: só o FPS real a mais.

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
