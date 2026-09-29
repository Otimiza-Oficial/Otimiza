# PERFORMANCE INTELLIGENCE PLATFORM — ENGENHARIA EXECUTÁVEL

Versão analisada: branch servico-t1 b96bd46 · 29/09/2026 · 149 arquivos Rust (71.680 linhas), 1.458 testes, 61 de laboratório

Este documento cobre as etapas 1 a 11 do pedido (do reality check aos critérios de aceite). O relatório de resultado
(etapas 12 a 18) fica em `PIP-RESULTADO.md`, escrito só depois de medir.

## 1. Reality check do código atual

Tudo abaixo foi conferido no código ou nos dados desta máquina; nada foi assumido porque "foi implementado numa
fase anterior".

### 1.1 A evidência que a empresa tem hoje

| Fonte | O que existe de fato |
|---|---|
| Medições automáticas (`medicoes.json`, PC do dono) | **9 medições, todas FiveM, uma máquina.** 6 são de antes do PresentMon: 218, 203, 200, 266, 153 e 183 FPS num monitor travado em 180 Hz — valores do medidor que dobra os quadros do FiveM (medido em 28/09: canal antigo = 2,00× PresentMon) |
| Prova alternada (`prova_alternada.json`) | **1 prova**, 8 rodadas de 45 s, FiveM: "Indistinguível" (64,0 vs 62,8 FPS) |
| Prova antes/depois (`prova.json`) | 1 medição guardada: **346 FPS num monitor de 180 Hz** — medidor dobrado, ainda servindo de "antes" |
| Telemetria de campo | **Zero.** Nenhum dado sai de nenhum PC |
| Laboratório | 1 PC (o do dono) |

**Conclusão honesta:** a empresa não tem, hoje, evidência de ganho de FPS de nenhuma otimização em nenhum jogo. Tem
um instrumento de prova bom e uma prova que deu "sem diferença". Qualquer frase de marketing com número seria
invenção.

### 1.2 A descoberta mais importante: o instrumento não tem poder para o que promete

Calculado dos próprios dados da prova alternada (4 rodadas por lado, FiveM, mesmo lugar):

| Métrica | Variação entre rodadas | Menor efeito detectável (80% de poder, 5%) — análise atual (grupos) | — análise pareada (vizinhos) |
|---|---|---|---|
| FPS médio | CV 6,6% | **≈ 22%** | ≈ 13% |
| 1% pior | CV 23% | **≈ 53%** | ≈ 72% (4 pares é pouco para estimar) |

Pares necessários para detectar 10% no 1% pior, pareado: **~95** (≈ 2 h 20 min de jogo). Para 5% no FPS médio: ~13 pares.

Consequências:
1. Nenhum ajuste de Windows realista (2–10%) é provável num FiveM de mundo aberto com o desenho atual. O
   "Indistinguível" era o único resultado possível. Não é que o ajuste não funcione: **a prova não enxerga**.
2. A análise atual (`repeticoes::comparar`, Welch entre grupos) joga fora o pareamento que a ordem ABBABAAB existe
   para criar. É um defeito de método, não de código.
3. O 1% pior por janela de 45 s é uma métrica ruim para A/B em mundo aberto: é dominado por 1–3 eventos de cena.
4. Isso decide a estratégia da plataforma (seções 8 e 9): **intervenção dirigida a uma causa, medida na métrica que
   essa causa move** (travadas daquela causa por minuto), onde o efeito esperado é grande — não ajuste genérico
   medido no FPS médio.

### 1.3 O que existe, o que existe pela metade, o que está mal feito

| Classe | Achado | Onde | Gravidade |
|---|---|---|---|
| **Defeito vivo** | Prova "Antes e depois": com medidor diferente nos dois lados, a tela diz **"Ganho medido e confirmado."** em verde, e a ressalva afirma "diferença pequena pode ser do medidor" — no FiveM a diferença é o dobro | `modules/prova.rs:226` (`vale_como_prova` não olha `mudou_o_medidor`), `src/main.ts:8113` | **P0** |
| **Defeito vivo** | Diagnóstico ao vivo do Mapa mede quadros pelo **canal antigo** (`frames::medir_par`): no FiveM, FPS dobrado e série de intervalos misturada alimentando o detector de travadas e o gargalo | `commands.rs:1741` | **P0** |
| Método fraco | Detector de travadas conta coincidência **sem taxa de base**: "disco em 9 de 14 travadas" vira "forte" mesmo se o disco estiver ocupado 60% do tempo. Sem evidência contrária, sem abstenção por amostra pequena | `core/travadas.rs:143` (`forca: n*2 >= total`) | Alto |
| Método fraco | Detector de travadas usa a **mediana da janela inteira**: em mundo aberto, trecho pesado vira "travada" (é FPS baixo, não microstutter) | `core/travadas.rs:19` | Médio |
| Método fraco | Prova alternada analisada como grupos independentes (perde o pareamento) | `modules/repeticoes.rs`, `provaalternada.rs` | Alto (seção 1.2) |
| Duplicação | **Três painéis "provar resultado"** com rigores diferentes: "Prova de resultado" (laço sintético de CPU), "Antes e depois, medido" (1 medição por lado, ruído fixo 3%), "Provar com o jogo aberto" (alternada). Mais o portão automático e o `experimento`/`grupos`. Cinco formatos de evidência para a mesma pergunta | `benchmark.rs`, `prova.rs`, `provaalternada.rs`, `portao.rs`, `windows/experimento.rs` | Alto |
| Duplicação | Oito superfícies de diagnóstico: `por_que_o_fps_esta_baixo` (causas), `diagnostico_da_partida` (doctor), `veredito`, `diagnostico_ao_vivo` (Mapa), `quedas_de_desempenho`, `nota_do_jogo`, `pronto_para_jogar`, `relatorio_da_ultima_partida` | `commands.rs` | Médio |
| Duplicação | Dois medidores de quadros (PresentMon e o canal antigo) ainda vivos em 13 arquivos | `frames::` em `commands.rs`, `lib.rs`, `framegen.rs`, `cpuset.rs`, `motorenergia_maquina.rs`… | Médio |
| Código morto | 12 comandos registrados que a interface nunca chama: `capturar_baseline`, `capturar_baseline_repetido`, `comparar_com_baseline`, `protocolo_do_perfil`, `passo_do_autoajuste`, `recuperacao_pendente`, `concluir_recuperacao`, `descartar_pendencia`, `stop_monitoring`, `fix_readiness`, `set_automatic_pagefile`, `set_max_refresh_rate` | `lib.rs` (handler) | Médio |
| Código morto (o próprio compilador avisa) | `anticheat::pode` (a regra de anticheat tem um método que ninguém chama), `sensores::julgar` e `LimiteDaPlaca`, `pontuacao::explicar`, `provaalternada::duracao_estimada_s`, `medicoes::id`, `NAO_ALTERAM_O_WINDOWS` fora dos testes | avisos de `cargo build` | Médio (`anticheat::pode` merece olhar: política que não é chamada) |
| Teatro de arquitetura | `autoajuste.rs` (308 linhas): máquina de estados "medir, aplicar uma, medir, decidir" sem nenhum chamador na interface; testada, nunca usada | `modules/autoajuste.rs` | — |
| Teatro de arquitetura | "Prova de resultado" mede um laço sintético (`black_box`) de CPU antes e depois e promete dizer "na sua cara" se houve ganho: mede o plano de energia num laço, não o jogo | `modules/benchmark.rs`, `index.html:898` | Alto (confiança) |
| Pela metade | Nenhuma medição do **custo do próprio Otimiza** durante o jogo existe | — | Alto (a regra "monitorar não pode destruir performance" nunca foi verificada) |
| Pela metade | Amostragem de processos guarda só os **3 maiores** por amostra: um processo em rajada curta perde para os constantes | `core/telemetria.rs:381` | Médio |

### 1.4 O que tem evidência real

- O medidor PresentMon separando quadro do jogo de gerado e o achado do canal antigo 2× (medido, 28/09).
- A prova alternada como **procedimento** (ordem, aquecimento, mesmo medidor/config/gerador): correto. A análise
  estatística é que perde poder.
- Transação, diário, recuperação na abertura, modo seguro: testados, com falha injetada nos testes.
- Governador: acalma e devolve prioridade/EcoQoS lendo o estado anterior; testado num processo real.

## 2. Architecture truth map (atual → alvo)

Maturidade: 0 inexistente · 1 protótipo · 2 funciona sem prova · 3 testado · 4 medido em campo.

| Sistema | Atual | Dívida técnica | Alvo | Migração | Depende de |
|---|---|---|---|---|---|
| Observação (quadros) | 3 | Dois medidores; Mapa no errado | 4 | Tudo pelo PresentMon; canal antigo só onde o PresentMon não roda, marcado não comparável | — |
| Observação (sistema) | 3 | Top 3 processos; sem custo próprio medido | 4 | Carimbo QPC em toda amostra; custo medido e exibido | — |
| Linha do tempo sincronizada | 1 | Relógios diferentes (ms do coletor, soma de intervalos) | 3 | QPC como relógio único; quadros e amostras na mesma escala | Observação |
| Causa-raiz | 2 | Sem taxa de base, sem evidência contrária, sem abstenção | 4 | V2 com teste de associação e abstenção (este slice); V1 fica em sombra até V2 provar | Linha do tempo |
| Experimento | 3 | Sem pareamento, sem parada sequencial, métrica sem poder | 4 | Teste de taxas de travada dirigido (este slice); pareamento na alternada (próximo) | Causa-raiz |
| Decisão | 2 | Regras espalhadas (portão, receita, doctor) | 3 | Valor esperado simples sobre a memória | Conhecimento |
| Política | 2 | `anticheat::permite`, protegidos, modo seguro em lugares diferentes | 3 | Função única chamada antes de agir | — |
| Execução | 3 | — | 3 | Manter | — |
| Validação | 2 | Cinco formatos de prova | 3 | Um formato de resultado com IC, efeito e decisão | Experimento |
| Conhecimento | 2 | Memória só de portão e prova de energia | 3 | Tabela de resultados (inclui negativos) por máquina × jogo × hipótese | Validação |

## 3. Top 10 problemas técnicos ainda sem solução

1. **Poder estatístico em mundo aberto.** O ruído de cena é maior que o efeito de qualquer ajuste genérico (seção 1.2).
   Sem resolver isso, "provar o que melhora" é impossível para o jogo que paga a empresa.
2. **Causa sem taxa de base.** O diagnóstico confunde coincidência com associação.
3. **Um relógio só.** Quadros, contadores e eventos em escalas diferentes impedem "o que houve 500 ms antes deste quadro".
4. **Custo próprio desconhecido.** Nunca medido; o produto não pode afirmar que não atrapalha.
5. **Dois medidores de quadros vivos.** Toda comparação precisa saber de qual medidor veio cada lado; dois painéis erram hoje.
6. **Microstutter vs FPS baixo.** Sem referência local, trecho pesado vira "travada".
7. **Nenhum dado fora de uma máquina.** Coorte, "funciona em 72% das máquinas", conhecimento global: tudo bloqueado.
8. **Identidade do jogo.** O FiveM tem build (b3258) no nome; outros jogos não: "patch mudou tudo" não é detectável.
9. **Cinco formatos de evidência.** Nenhum consumidor (receita, portão, compartilhar) consegue juntar resultados.
10. **Abstenção.** Nenhum motor sabe dizer "não sei" de forma estruturada; diagnóstico errado custa mais que "causa não determinada".

## 4. Cinco fossos técnicos defensáveis

| # | Capacidade | Por que importa | Por que não se copia rápido | Dados | Engenharia | Tempo | Valor ao cliente |
|---|---|---|---|---|---|---|---|
| 1 | **Diagnóstico de travada com taxa de base e abstenção** | Aponta a causa com a evidência a favor e contra, ou diz "não sei" | Exige relógio único de quadros + sistema, estatística certa e disciplina de abster | Cada sessão investigada vira caso rotulado | QPC, PresentMon, PDH, teste exato | Meses de calibração em casos reais | Sabe por que trava, sem chute |
| 2 | **Teste ativo dirigido à causa** | Transforma suspeita em causa demonstrada em minutos, no jogo do cliente | Precisa do fosso 1 para saber o que testar e de execução reversível | Resultados positivos e negativos por contexto | Controle de processo reversível, janelas, taxas | Acumula por uso | "Testamos na sua máquina: era isto" |
| 3 | **Memória de efetividade com resultado negativo** | Não repete o que não funcionou; diz "testamos, não recomendamos" | Só acumula com 1 e 2 | Por máquina hoje; global com opt-in | Tabela simples, proveniência | Cresce com o tempo | Menos mudanças inúteis |
| 4 | **Instrumento de medição por jogo** (FiveM 2×, cadeias, gerador) | Número errado destrói tudo acima | Descoberta empírica, não está em fórum | Casos por jogo | Separação de cadeia/tipo de quadro | Um jogo por vez | Números em que se pode confiar |
| 5 | **Desenho de experimento com poder conhecido** | Diz antes quanto tempo a prova precisa e o que ela consegue enxergar | Concorrente mede antes/depois e publica | Variância por jogo acumulada | Pareamento, sequencial, taxas | Variância por jogo leva tempo | Não vende o que não se mediu |

## 5. Arquitetura estrela-guia

Os nove sistemas propostos são conceitos certos, mas **nove camadas com contratos próprios seriam teatro** para uma
equipe de uma pessoa. A arquitetura alvo junta em cinco, num processo só:

```
┌─ OBSERVAÇÃO ─────────────────────────────────────────────────────────────┐
│ PresentMon (quadros, QPC) · PDH/sysinfo (sistema, QPC) · NVML (freio)     │
│ Uma sessão de captura por vez; custo medido e anexado a cada resultado    │
└──────────────┬───────────────────────────────────────────────────────────┘
               ▼ Linha do tempo (vetores ordenados por QPC, em memória)
┌─ ENTENDIMENTO (puro) ───────────────────────────────────────────────────┐
│ Travadas (referência local) → Hipóteses → Associação com taxa de base     │
│ → Evidência a favor/contra → Confiança ou abstenção → Teste recomendado   │
└──────────────┬───────────────────────────────────────────────────────────┘
               ▼
┌─ EXPERIMENTO ──────────┐   ┌─ POLÍTICA (pura) ─┐   ┌─ EXECUÇÃO ─────────┐
│ Janelas alternadas,    │◄──┤ permitido? motivo │──►│ ação reversível    │
│ taxas, sequencial, IC  │   └───────────────────┘   │ com diário e volta │
└──────────────┬─────────┘                            └────────────────────┘
               ▼
┌─ CONHECIMENTO ───────────────────────────────────────────────────────────┐
│ Resultado (inclui negativo) × máquina × jogo × hipótese × contexto × data │
└──────────────────────────────────────────────────────────────────────────┘
```

Decisões e rejeições:
- **Barramento de eventos: rejeitado.** Um processo, poucos consumidores, captura sob demanda: um barramento só
  acrescenta indireção. O que o pedido quer do barramento (sem coleta duplicada) resolve-se com **uma sessão de
  captura por vez** (já existe o `UMA_POR_VEZ` do PresentMon) e a linha do tempo como valor passado adiante.
- **Amostragem adaptativa:** o "modo leve" já é o padrão — o Otimiza não mede nada contínuo durante o jogo, só
  janelas espaçadas. A "captura profunda" é a investigação sob demanda deste slice. O gatilho automático (caixa-preta)
  espera o custo do PresentMon contínuo ser medido.
- **Microsserviços, serviço residente, IA decidindo: rejeitados.** Núcleo puro testável + casca fina de Windows.
- **Motor de política:** função pura, não DSL.

## 6. Remover / simplificar

| Item | Ação | Motivo | Risco de remover |
|---|---|---|---|
| "Prova de resultado" (laço sintético de CPU) | **Remover da tela** | Promete provar ganho medindo um laço, não o jogo | Baixo |
| "Antes e depois, medido" | **Simplificar**: medidores diferentes = não comparável (corrigido neste slice); a médio prazo, virar a primeira rodada da alternada | Uma medição por lado em mundo aberto tem ruído maior que o efeito | Baixo |
| `autoajuste.rs` + `passo_do_autoajuste` | **Remover** | Sem chamador; a alternada faz o papel | Nenhum |
| `capturar_baseline*`, `comparar_com_baseline`, `protocolo_do_perfil` | **Remover comandos** (manter `baseline.rs` só no que outros módulos usam) | Sem chamador | Nenhum |
| `stop_monitoring`, `fix_readiness`, `set_automatic_pagefile`, `set_max_refresh_rate`, `recuperacao_pendente`, `concluir_recuperacao`, `descartar_pendencia` | **Conferir um a um e remover** o que não tiver uso interno | Superfície de IPC sem uso | Baixo |
| Canal antigo de quadros | **Restringir** a reserva explícita, nunca misturado | Dobra no FiveM | Médio (máquinas sem PresentMon) |
| `core/travadas.rs` (V1) | **Substituir** depois que a V2 provar melhor nos casos gravados; até lá, em sombra | Sem taxa de base | Baixo |
| Oito superfícies de diagnóstico | **Convergir** em "Por que trava / por que o FPS está baixo" com uma resposta | Respostas diferentes para a mesma pergunta | Médio (UI) |

Remoções visíveis ao cliente (os dois primeiros itens) são decisão do dono; as de código morto entram no próximo passo.

## 7. Laboratório autônomo — desenho

**Premissa:** hoje há um PC. O desenho abaixo é o alvo; cada peça tem o gatilho que a liga.

| Peça | Desenho | Liga quando |
|---|---|---|
| Definição de teste | Arquivo Rust tipado (`Experimento { hipotese, metrica_primaria, efeito_minimo, cenario, orcamento, criterios_de_invalidacao }`) versionado no repositório — **DSL rejeitada**: o tipo já dá validação, versão e revisão | Já (este slice usa) |
| Pré-registro | O experimento é commitado antes de rodar; o resultado aponta o commit | Já |
| Agente de laboratório | O próprio Otimiza em modo `--laboratorio <experimento>`: valida estado (temperatura inicial, carga de fundo < 5%, plano), roda, grava o artefato, restaura | 2ª máquina |
| Orquestrador / fila | Pasta compartilhada com jobs JSON; cada agente pega o que casa com seu hardware | ≥ 3 máquinas |
| Repetibilidade | Cenário fixo por jogo (FiveM: parado, mesmo lugar, sem menu); janelas alternadas; aquecimento | Já |
| Reset de ambiente | "Restaurar meu PC" (já existe) + conferência do estado antes do próximo job | Já |
| Artefatos | Linha do tempo compactada (QPC, intervalos, amostras) por execução | Já |
| Banco de resultados | JSON por resultado → tabela; SQLite só quando passar de milhares de linhas | Volume |
| Estatística | Seção 9 | Já |
| Anomalia de máquina | Variância da máquina de referência numa carga fixa; fora de 3 desvios, sai do pool | 2ª máquina |
| Máquina de referência | O PC do dono vira a golden system: config congelada e registrada | Já |
| Laboratório distribuído | Voluntários do beta rodando o mesmo experimento pré-registrado | Opt-in (decisão do dono) |

## 8. Motor de causa-raiz — desenho (V2)

**Travada (bad frame):** intervalo do quadro do jogo ≥ 2× a **mediana local** (61 quadros em volta) **e** ≥ mediana
local + 8 ms. O mínimo absoluto separa o perceptível: a 180 FPS, dobrar de 5,5 para 11 ms não se sente; a 60 FPS,
16,7 → 33 ms sim. Quadros em sequência a menos de 250 ms formam **um episódio** (a unidade do teste; travadas coladas
não são independentes).

Severidade pelo excesso sobre a mediana local: leve 8–16 ms · moderada 16–33 ms · severa 33–100 ms · extrema > 100 ms.

**Lado do quadro:** o PresentMon dá `CPUBusy` e `GPUBusy` por quadro. Episódio em que o CPUBusy do pior quadro passou
de 1,5× a mediana local e o GPUBusy não = lado processador; o inverso = lado placa; senão indefinido. Evidência
contra "outro programa disputando o processador" é travada do lado da placa.

**Hipóteses** (cada uma com o sinal que a indica, medido nas amostras de 500 ms com carimbo QPC):

| Hipótese | Sinal ativo na amostra | Teste ativo possível |
|---|---|---|
| Disputa por processo X | CPU de X ≥ 8% da máquina e ≥ 3× o normal dele | **Sim: acalmar X (prioridade abaixo do normal + EcoQoS), alternando** |
| Disco | latência ≥ 20 ms e ≥ 3× a normal | Não (recomendação) |
| Paginação (falta de RAM) | páginas lidas ≥ 200/s e ≥ 3× o normal | Não (recomendação: fechar programas) |
| VRAM | usada ≥ 95% ou compartilhada subiu ≥ 100 MB | Não (recomendação: textura) |
| Núcleo saturado (o próprio jogo) | núcleo mais ocupado ≥ 95% com a mediana < 85% | Não |

**Associação com taxa de base:** para cada hipótese, `p0` = fração das amostras em que o sinal está ativo (a taxa de
base). `k` de `n` episódios caem numa amostra ativa. Sob a hipótese nula (travada independente do sinal),
`k ~ Binomial(n, p0)`. Valor-p exato da cauda superior, corrigido por Bonferroni pelo número de hipóteses testadas.
`lift = (k/n)/p0`.

**V2.1 (depois do laboratório, ver PIP-RESULTADO.md):** a travada é o INTERVALO em que o atraso aconteceu (do
`CPUStartQPC` ao fim do quadro), não um instante; conta como "com o sinal" se tocar qualquer amostra com ele ligado, e
a nula usa, por travada, a chance de um intervalo daquela duração tocar o sinal ao acaso (cauda Poisson-binomial
exata). Hipótese acrescentada: processador inteiro ocupado (≥ 90%).

**Confiança** (sempre com a conta escrita na tela):

| Nível | Critério |
|---|---|
| Forte | p corrigido < 0,01, lift ≥ 2, k ≥ 3 |
| Moderada | p corrigido < 0,05, lift ≥ 1,5, k ≥ 3 |
| Sem evidência | o resto: "o sinal aparece tanto nas travadas quanto fora delas" |
| Não medido | o contador não respondeu: nunca vira "ausente" |
| **Abstenção** | n < 5 episódios: "poucas travadas para concluir" |

**Evidência contra** (reduz o nível um degrau quando presente): taxa de base alta (p0 > 0,5); maioria dos episódios
associados do lado da placa numa hipótese de processador; cobertura baixa (a hipótese explica < 25% dos episódios).

**Explicação:** "11 de 14 travadas aconteceram com o OneDrive usando o processador; fora delas, ele usava em 9% do
tempo." O texto vem da conta, nunca de um modelo.

**Degrau de causalidade:** sem teste ativo, o máximo é **associação**. "Causa demonstrada" só depois do teste da seção 9.

## 9. Desenho de experimento

**Métrica primária do teste dirigido:** episódios de travada por minuto de jogo. Um processo que causa travada, se
acalmado, derruba essa taxa muito mais que o FPS médio: efeito grande onde o ruído de cena importa pouco.

**Desenho:** janelas de 30 s alternadas A (normal) / B (processo acalmado), ordem ABBA·BAAB (anula deriva linear),
**uma** captura contínua do PresentMon e troca em instantes QPC registrados; 2 s depois de cada troca são descartados.

**Teste:** comparação de duas taxas de Poisson com **correção de superdispersão** (quasi-Poisson): a dispersão φ é
estimada da variação das contagens entre janelas do mesmo lado (piso 1). Razão de taxas `RR = (kB/tB)/(kA/tA)`, IC 95%
em log: `log RR ± z·√(φ(1/kA + 1/kB))`.

**Sequencial (duas olhadas):** após 4 janelas e após 8. Limite por olhada z = 2,24 (α = 0,025 cada, Bonferroni: α
total ≤ 0,05). Para cedo se o IC já decide.

**Melhoria mínima que importa:** redução de ≥ 30% nas travadas (RR ≤ 0,7) **e** IC superior < 1. Abaixo disso,
mesmo significativo, não compensa manter uma mudança.

**Guarda "nunca menos FPS":** FPS médio de B não pode ficar abaixo de A além de 3% (média das janelas, pareada).

**Decisão:** MANTER (efeito ≥ mínimo, IC < 1, FPS guardado) · DESFAZER com "sem melhoria confiável" · DESFAZER com
"piorou" · INVÁLIDO (jogo fechou, processo sumiu, janela com < 50% dos quadros esperados, suspensão do PC detectada
por salto de relógio, < 3 episódios no total).

**Invalidação automática de janela:** quadros < 50% do esperado pela mediana da captura (menu/carregamento/minimizado);
buraco > 1 s entre quadros (PC suspenso ou jogo pausado).

## 10. Primeiro vertical slice

**Escolha: microstutter por disputa de processador** — a sugestão do pedido, mantida, porque:
- ataca o problema 1 da seção 3 pelo único caminho com poder estatístico (efeito grande na métrica certa);
- exercita todas as camadas: observação sincronizada, detecção, hipóteses, taxa de base, evidência contra,
  abstenção, política, execução reversível, experimento sequencial, decisão, memória negativa;
- a ação de teste (acalmar um processo) já existe testada no governador, não toca o jogo e é aceita com anticheat;
- conserta de passagem os dois defeitos P0 da seção 1.3.

Fluxo: Detectar → Capturar (PresentMon + amostras, QPC) → Correlacionar (taxa de base) → Hipóteses → Confiança /
abstenção → Explicar → Testar a correção candidata (acalmar X, janelas alternadas) → Medir → Manter / Desfazer → Lembrar.

Fora do slice, de propósito: gatilho automático (caixa-preta), DPC/ISR, shader/streaming por assinatura,
atribuição de contribuição, Pareto.

## 11. Critérios de aceite (definidos antes de implementar)

**Sucesso:**
1. Em dados sintéticos com causa plantada (travadas geradas quando um processo dispara), o motor aponta o processo
   com confiança Moderada ou Forte em ≥ 90% das simulações.
2. **Falso positivo:** em dados sintéticos sem relação (processo disparando em horas aleatórias, travadas
   independentes), o motor afirma associação em ≤ 5% das simulações.
3. Com menos de 5 episódios, abstém sempre.
4. Contador indisponível vira "não medido", nunca "sem evidência".
5. Experimento sintético: redução real de 60% nas travadas → MANTER em ≥ 80% das simulações; sem efeito real →
   MANTER em ≤ 5%.
6. Teste ativo real: todo processo acalmado volta ao estado anterior ao fim, no erro, e na abertura seguinte se o app
   morrer no meio (arquivo de recuperação do governador).
7. Laboratório na máquina do dono: com uma carga de processador controlada disparando em rajadas junto do jogo, o
   motor aponta a carga, e o teste ativo mostra a redução de travadas.
8. Custo próprio medido e mostrado no resultado.
9. Os dois defeitos P0 corrigidos com teste.
10. Toda a suíte existente continua verde; `revisor-otimiza` sem bloqueio; revisão de segurança sem achado alto.

**Fracasso (e o que se faz):**
- Falso positivo sintético > 5% → o critério de confiança está errado; não sai.
- O teste ativo não reproduz em laboratório uma causa plantada → o desenho de janelas não tem poder; volta ao desenho.
- Custo próprio > 2% de um núcleo durante a investigação → a amostragem cai para 1 s antes de sair.
- Qualquer processo que não volte ao estado anterior → bloqueio de lançamento.
