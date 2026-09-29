# PERFORMANCE INTELLIGENCE OS — ARCHITECTURE & R&D MASTER PLAN

Versão analisada: branch servico-t1 dddc575 (itens 1–7, 9 e 10 do master plan anterior) · 29/09/2026

Documento de direção. Nenhum código alterado para escrevê-lo. Complementa `MASTER-PLAN.md`; não repete o que
lá foi decidido.

## Leitura rápida

- **A pergunta nova** ("qual é a causa exata e qual a menor mudança que dá a maior melhora comprovável") tem uma
  resposta de engenharia honesta: **causa só se prova intervindo**. Observar partidas e correlacionar dá
  suspeitos; quem transforma suspeito em causa é o experimento controlado **na máquina do cliente**. O Otimiza já
  tem o instrumento mais raro disso: a prova alternada (ABBABAAB, mesmo medidor, mesma config, IC 95%). O núcleo
  proprietário é generalizar esse instrumento e acumular o que ele aprende.
- **O que já é vantagem real hoje:** medição separando quadro do jogo de quadro gerado; prova alternada;
  portão "nunca menos FPS"; memória de efetividade por máquina; o achado de que o medidor antigo dobra os quadros
  do FiveM (medido em 28/09). Nenhum otimizador de tweak tem isso, e não se copia abrindo o Regedit.
- **O que NÃO dá para construir agora, com motivo:** laboratório autônomo com fazenda de hardware, clustering de
  incidentes em milhares de clientes, inteligência de lançamento de driver, SDK/API pública, módulos de terceiros,
  grafo de conhecimento em banco de grafos, DSL textual de regras, descoberta causal por dados observacionais.
  Todos dependem de coisas que não existem (máquinas, telemetria com volume, equipe) ou trocam um problema
  pequeno por um grande. Estão na seção 26 e no portfólio (27) com o gatilho que os destrava.
- **Primeiro vertical slice escolhido:** **Prova causal de um suspeito do diagnóstico** — o diagnóstico aponta um
  suspeito, o Otimiza intervém nele em rodadas alternadas com o jogo aberto, classifica a causalidade
  (coincidência → correlação → provável → demonstrada), decide com a regra da menor mudança, registra inclusive o
  resultado negativo, e explica. Primeiro alvo: **o modo jogo (acalmar programas) em jogo limitado pelo
  processador** — é a intervenção que liga e desliga em segundos sem reiniciar nada, e hoje ela leva semanas para
  ser julgada pelo portão (e, reprovada uma vez, nunca mais é testada).

## Premissas de realidade

| Fato | Consequência para o plano |
|---|---|
| Uma pessoa (o dono) + desenvolvimento assistido | Nada que exija equipe de pesquisa ou plantão |
| Clientes majoritariamente FiveM, mundo aberto, servidor de terceiros | Não existe benchmark determinístico: a prova tem de ser no jogo do cliente, alternada, não "antes e depois" |
| Um PC de laboratório (o do dono) | "Laboratório" = PCs de clientes voluntários + o do dono, não fazenda |
| Zero telemetria saindo do PC hoje | Toda inteligência global fica bloqueada até o opt-in (item 8 do plano anterior) |
| Licença sem servidor | Nada pode depender de backend para funcionar |
| Regras do produto | Nunca menos FPS; nada sem volta; nada em processo protegido por anticheat; nenhum número inventado |

## 1. Architecture Vision

Um ciclo, não uma lista de módulos:

```
OBSERVAR ──► SUSPEITAR ──► INTERVIR ──► JULGAR ──► DECIDIR ──► EXECUTAR ──► VALIDAR ──► LEMBRAR ──► REVALIDAR
 medição      diagnóstico   experimento   estatística  política     transação    portão      memória      deriva
 (PresentMon) (doctor)      (switchback)  (evidência)  (policy)     (changelog)  (portão)    (efetividade)(guardião)
```

Camadas, cada uma com um dono e um contrato:

| Camada | Responsabilidade | Existe hoje em |
|---|---|---|
| Sensores | Quadros, contadores, eventos, ambiente | `presentmon`, `medicoes`, `core/telemetria`, `nvml` |
| Evidência | Comparabilidade, estatística, qualidade do dado | `evidencia`, `repeticoes`, `estatistica` |
| Diagnóstico | Suspeitos com força da evidência | `doctor`, `causas`, `gargalo`, `crashes` |
| Experimento | Intervenção controlada no jogo | `provaalternada` (só plano de energia) |
| Conhecimento | O que funcionou e o que não funcionou, por contexto | `receita::lembrancas`, `portao` |
| Política | O que é permitido fazer | espalhado: `anticheat::permite`, `safety`, portão |
| Execução | Aplicar e desfazer com diário | `changelog`, `transacao`, `restaurar` |

A mudança de arquitetura desta fase é **uma só**: separar **conhecimento** (o que sabemos) de **política** (o que
permitimos) e colocar o **experimento** no centro, como o único caminho que promove uma hipótese a causa.

## 2. Causal Performance Engine

**Escada de causalidade** (cada afirmação na tela carrega um degrau; nenhum degrau se pula):

| Degrau | Critério | Exemplo |
|---|---|---|
| Coincidência | Aconteceram no mesmo período, sem medida de associação | "o driver mudou na mesma semana" |
| Correlação | Associação medida em janelas, sem intervenção | "60% dos trancos com disco ocupado" |
| Causa provável | Correlação + mecanismo conhecido + direção temporal (a causa precede) | "o freio térmico começou 2 s antes da queda" |
| Causa demonstrada | Intervenção controlada, alternada, reproduzida, IC 95% sem sobreposição | "desligando X em 4 rodadas, o 1% pior subiu além da margem" |
| Desconhecida | Efeito medido sem suspeito que o explique | "trancos sem disco, sem freio, sem processo" |

**Método para "demonstrada":** experimento **switchback** (alternância A/B/B/A no tempo, na mesma partida) — é o
desenho usado quando não há grupo de controle separado, e controla deriva térmica e de cena linearmente
(ABBABAAB). A estatística que já existe (IC 95% de Student, sobreposição) é conservadora e fica. Acrescenta-se:
**reprodução** (duas provas independentes concordando elevam o grau) e **poder** (dizer quando a prova não teria
como detectar o efeito procurado, em vez de "sem diferença").

**Rejeitado:** descoberta causal a partir de dados observacionais (algoritmos PC/FCI, redes bayesianas
aprendidas). Com poucas variáveis medidas, confundidores não medidos (cena do mapa, servidor, jogadores em volta)
e poucas amostras por máquina, o resultado seria causalidade inventada com cara de matemática.

**Contrafactual responsável:** "o desempenho caiu por causa da nossa otimização ou teria caído mesmo assim?" só
tem resposta boa de dois jeitos: (a) **alternância** — o contrafactual é medido, não estimado; (b) quando não dá
para alternar, **diferença-em-diferenças** com uma série de controle (outro jogo, ou partidas sem a mudança) e o
aviso explícito de que é estimativa. O portão atual é (b) sem controle; a prova alternada é (a).

## 3. Performance Digital Twin

Não é simulação. É o **registro tipado do que esta máquina é e de como ela já respondeu**:

```
Identidade (hardware, driver, build, BIOS lida, monitores)          ← deriva::Ambiente, perfil
Comportamento (gargalos por jogo, calor/freio, trancos com disco)    ← medicoes + eventos + doctor
Respostas (estratégia × contexto → resultado com IC e data)          ← receita::lembrancas (portão, prova)
Estabilidade (quedas do app, crashes do jogo, sem base)              ← diagnostico, crashes, portão
```

**Previsão** (LIKELY BENEFICIAL / UNCERTAIN / LIKELY NEUTRAL / LIKELY REGRESSION) sai de **encolhimento
hierárquico** (empirical Bayes): o efeito estimado para esta máquina é a média ponderada entre a evidência local e
a do nível acima (jogo → coorte → global), com peso pela quantidade e qualidade de cada nível. Sem dado local, vale
o nível acima com incerteza larga; com prova local, a local domina (a hierarquia pedida em "customer-specific
learning"). Sempre devolve **faixa, confiança, dados usados e limites**. Hoje só existem os níveis "esta máquina" e
"regra do produto" (não há coorte nem global sem telemetria).

## 4. Optimization Compiler

Entrada: máquina (twin) + jogo + objetivo + política + evidência. Saída: **plano ordenado com a menor mudança**:

1. Filtra por **política** (seção 5): o que não pode, sai; o que precisa perguntar, marca.
2. Ordena por **valor esperado** (seção 9): benefício provável × confiança − custo do experimento − risco.
3. Resolve **conflitos e dependências** (seção 7).
4. Escolhe **o primeiro passo só**, experimenta, e recompila com o resultado. Um plano de vinte passos
   executado de uma vez é o contrário da menor mudança: o compilador emite um passo, não um lote.

A "compilação" é código Rust tipado sobre dados declarativos (`Estrategia` com pré-condições, efeitos esperados,
custo, como provar, como desfazer). **Rejeitada a DSL textual** (seção 26).

## 5. Policy Engine

Função pura `decidir(proposta, contexto) → ALLOW | ASK | DENY | EXPERIMENTAL`, com o motivo. Juntar o que hoje está
espalhado:

| Regra | Hoje em | Resultado |
|---|---|---|
| Anticheat do jogo recusa a ação | `anticheat::permite` (3 módulos) | DENY |
| Sem volta verificável | catálogo `reversible` | DENY |
| Já piorou neste contexto | `receita::oferta` | DENY (ou ASK se contexto mudou) |
| Prova rodando, modo seguro, sem administrador | `provaalternada::em_andamento`, `diagnostico::modo_seguro` | DENY |
| Troca segurança por desempenho | `security_tradeoff` | ASK |
| Nunca provado aqui | memória vazia | EXPERIMENTAL (só com a prova) |
| Mexe em BIOS, overclock, tensão | — | DENY sempre |

Regra de arquitetura: **o diagnóstico e o compilador nunca executam**; só a execução executa, e só com ALLOW.
Um teste de arquitetura garante que toda escrita passa pela política (como `registro.rs` já garante o registro).

## 6. Optimization Knowledge Graph

"Grafo" é o **modelo mental**; a implementação é **tabela tipada** (estratégia, contexto, resultado, IC, data,
proveniência, método). Consultas do tipo "o que tem maior evidência de ganho no 1% pior em Zen 3 + RTX 3060 +
FiveM" são filtros e agregações sobre essas linhas. **Rejeitado banco de grafos** (seção 26): dezenas de
estratégias e, por máquina, dezenas de linhas; um banco de grafos só acrescenta dependência e superfície.

Toda linha carrega **proveniência** (qual versão do Otimiza, qual medidor, qual método) e **idade** — o
decaimento de confiança da seção 16 opera sobre ela. O **banco de resultados negativos** é a mesma tabela: "sem
diferença" e "piorou" são linhas de primeira classe, com a condição que as reabre ("só retestar se o driver ou o
build mudar", que já existe em `receita`).

## 7. Conflict Graph

Já existe o embrião certo: `windows/conflitos.rs` só aceita conflito **com o mecanismo escrito** e `grupos.rs`
agrupa. A evolução: cada `Estrategia` declara `exclui`, `requer`, `depois_de` e `mesma_alavanca` (duas que mexem no
mesmo recurso, como plano de energia OTIMIZA e motor de energia). O compilador recusa plano com aresta violada.

**Interações medidas** (A e B bons sozinhos, ruins juntos) só entram com dado: quando duas estratégias aplicadas
provaram ganho separadamente, a prova de A+B contra B confirma se A ainda soma. Não se testa combinação sem
suspeita — ver seção 8.

## 8. Auto-Tuner V2

**Recusado:** otimização bayesiana e bandits sobre "milhares de combinações". O espaço real do Otimiza é pequeno
(uma dúzia de alavancas úteis por jogo), cada avaliação custa minutos de jogo do cliente, e o ruído entre cenas é
grande. Nesse regime, um modelo de superfície (processo gaussiano) não tem amostras para aprender, e o bandit
"explora" jogando o cliente em configuração pior.

**Método escolhido — busca gulosa pela menor mudança, com prova sequencial:**

1. Linha de base medida (o que já existe).
2. Candidato = a estratégia de maior valor esperado permitida pela política.
3. Prova alternada com **parada sequencial**: rodadas até o intervalo decidir (ganho, perda, ou "o efeito, se
   existir, é menor que o mínimo que interessa") ou até o orçamento (seção 9). Isso economiza o tempo do cliente
   quando o efeito é grande ou claramente nulo.
4. Fica só o que provou ganho sem piorar a métrica guardiã. Recompila.
5. **Para** quando: o valor esperado do próximo candidato não paga o custo; o orçamento de tempo acabou; calor
   passou do limite; instabilidade (queda, gerador ligando no meio); ou dois candidatos seguidos sem ganho.

Interações: só testa A+B quando A e B provaram ganho sozinhos (custo linear, não combinatório).

## 9. Multi-Objective Optimization

Métricas: FPS médio, 1% pior, P99 do quadro, atraso até a tela, temperatura, potência. **Regra fixa do produto:
FPS médio não pode cair além da margem** (nunca menos FPS) — é restrição, não objetivo negociável. Dentro dela, o
objetivo do cliente escolhe o que maximizar ("mais fluido" = 1% pior e P99; "mais FPS"; "mais silencioso" =
potência com FPS mantido).

**Pareto honesto:** a fronteira só aparece quando existirem **pelo menos duas configurações provadas** com
trocas reais entre si (ex.: plano OTIMIZA vs motor de energia com menos potência e mesmo FPS). Fronteira desenhada
com estimativas seria tela bonita sobre número inventado.

**Função de valor** (substitui "Improvement × Confidence × Safety − Cost − Risk", que mistura unidades):

```
valor = P(ganho ≥ mínimo que importa) × tamanho esperado do ganho no objetivo
        − custo do experimento (minutos do cliente, reinício, calor)
        e só se: P(violar restrição) < limite  E  reversível  E  política = ALLOW
```

Probabilidade e tamanho saem do twin (seção 3); restrição e reversibilidade são portões, não pesos.

## 10. Gaming Flight Recorder

Gravador em **anel na memória**, sem disco até haver evento:

- Fonte de quadros: a sessão contínua do PresentMon que já existe (`presentmon::Vivo`, hoje usada pela guarda do
  gerador). Guarda-se só `(instante, intervalo, tipo)` por quadro: ~24 bytes; 30 s a 240 FPS ≈ 170 KB.
- Contadores (CPU, disco, GPU, freio NVML) a 5 Hz no mesmo relógio QPC, já amostrados na medição automática.
- Gatilho: tranco acima de limiar (mesma regra de `travadas`), queda de FPS, freio, ou o botão/atalho do cliente.
  Ao disparar, persiste −10 s/+10 s.

**Condição para ligar:** medir o custo do PresentMon contínuo com o jogo aberto. Hoje ele roda contínuo só com o
gerador ligado. Se passar do orçamento (CPU < 1% de um núcleo), fica no modo "janela" de hoje e o gravador liga só
por gatilho do cliente. Sem esse número medido, não se liga para todos.

## 11. Root Cause Analysis

Para cada tranco no gravador, a explicação é uma **tabela de coincidências com o degrau da escada**:

| Suspeito | Sinal | Janela | Degrau máximo sem intervir |
|---|---|---|---|
| Disco | ocupação ≥ 40% ±300 ms | já existe | correlação |
| Freio da placa | NVML térmico/energia começando | já existe | causa provável (precede e tem mecanismo) |
| Gerador de quadros | ligado na janela | já existe | correlação |
| Processo em rajada | CPU de um processo ±300 ms | **falta** (amostragem por processo) | correlação |
| DPC/ISR | ETW de kernel | **pesquisa** (custo e privilégio) | — |

A frase na tela nunca passa do degrau da tabela. "Provável causa: X (82%)" só com modelo calibrado; sem ele,
mostra a contagem ("9 de 14 trancos com disco ocupado") e o degrau. Para subir a "demonstrada", oferece a prova
(seção 2).

## 12. Incident Intelligence

Um **incidente** é um evento com evidência: tranco grave, colapso de FPS, freio, reset de driver (evento do
Windows já lido em `eventoshw`), crash do jogo (`crashes`), queda do Otimiza (`diagnostico`), "sem base" do portão.
Localmente, isso é o que já existe espalhado — juntar numa lista com severidade, evidência, degrau e ação.

**Agrupamento global e alerta precoce** (driver 600.42 quebrou o jogo X em placas Y) exigem telemetria com
volume. Sem isso, o "alerta precoce" possível hoje é o **suporte**: o diagnóstico que o cliente cola no Discord
já traz driver, build, crashes e partidas — um relatório semanal que agrupa os diagnósticos recebidos é o
clustering da escala atual.

## 13. Autonomous Performance Lab

**Rejeitado agora** (seção 26): não há máquinas, e FiveM — o jogo que paga a empresa — não tem benchmark
determinístico nem roda sem servidor. O laboratório da escala atual é **distribuído**: a prova alternada rodando
nos PCs de clientes voluntários do canal beta, com consentimento, e o resultado voltando como linha da tabela de
conhecimento. É mais barato, testa hardware real variado, e o instrumento já existe.

**Gatilho para reabrir:** receita que pague ≥3 máquinas dedicadas + um jogo alvo com benchmark embutido que
represente clientes pagantes.

## 14. Benchmark Orchestration

Toda prova já registra medidor, config do jogo (hash), gerador, driver, build, placa, BIOS lida, versão do
Otimiza. Falta: **nota de qualidade da prova** (seção 16) e **cenário** ("parado num lugar" vs "andando"). Para
FiveM, o "golden benchmark" possível é o **protocolo**: parado, mesmo lugar, sem menu, 8 rodadas de 45 s — já é o
que a prova pede; formalizar como `Cenario` versionado.

## 15. Performance Certification

Estados de uma estratégia, com o que exige cada um:

| Estado | Exige |
|---|---|
| RESEARCH | hipótese e mecanismo escritos |
| LAB VALIDATED | prova alternada no PC do dono com ganho, 2 provas concordantes |
| CANARY | beta opt-in em ≥3 máquinas diferentes, nenhuma piora |
| FIELD VALIDATED | portão decidindo em campo, taxa de "Desfazer" abaixo de limite |
| CERTIFIED | o acima + volta testada + compatibilidade anticheat declarada |
| DEPRECATED | sem ganho medido em campo por N meses, ou irrelevante |
| BLOCKED | piorou em campo ou risco descoberto (desliga por regra assinada) |

O catálogo já tem um esboço disso (`catalog::retirado`, `Classe::Expert`); o estado vira campo tipado e o
"Evidence Pack" é um documento por estratégia em `docs/estrategias/` gerado da tabela.

## 16. Data Quality

Cada medição/prova recebe nota: **ALTA** (PresentMon, amostra ≥ regra, sem gerador, config estável, sem freio no
meio), **MÉDIA** (canal antigo, ou 1% sem amostra de 0,1%), **BAIXA** (janela curta, freio no meio, medidor
misturado), **REJEITADA** (interrompida, config mudou, gerador ligou). Decisão só com ALTA/MÉDIA; tela diz a nota.
**Decaimento**: peso da evidência cai com a idade e zera quando o contexto muda (driver, build, versão do jogo).
**Dado contribuído** (futuro): valores impossíveis (FPS acima do Hz sem gerador com apresentação "Copiada"),
métricas inconsistentes (1% > média) e medidor desconhecido são rejeitados na entrada.

## 17. Machine Learning

**Onde faz sentido:** (a) encolhimento hierárquico da seção 3 — é estatística, não "IA", e é o que dá previsão
com incerteza honesta; (b) similaridade de máquinas por atributos técnicos (arquitetura, núcleos, VRAM, placa de
notebook vs desktop) para coortes, quando houver dados de outras máquinas.
**Onde não faz:** prever FPS absoluto, detectar causa sem intervenção, "IA" que decide ajuste. Sem volume e sem
rótulo confiável, vira número inventado. Critério para qualquer modelo entrar: calibrado (o "80%" acerta ~80%),
com faixa, dados usados e limites, e perde para a regra se não for melhor em validação fora da amostra.

## 18. Privacy

Local-first continua: tudo que está nesta arquitetura roda no PC. Quando o opt-in existir (item 8 anterior):
esquema mínimo (estratégia, contexto em faixas, resultado com IC, nota de qualidade, versão), sem nome de usuário,
sem caminhos, sem SID, sem identificador persistente de máquina (um identificador por envio); retenção limitada;
apagar pelo próprio app; o cliente vê o JSON antes de enviar. O gravador de voo **não** sai do PC; o que sai é o
resumo do incidente, com consentimento por envio (como o diagnóstico de suporte hoje).

## 19. Security

- **Menor privilégio:** hoje o app inteiro roda elevado quando precisa escrever. O **broker privilegiado**
  (serviço mínimo que só executa ações do catálogo por id, com a política verificando) é a arquitetura certa e
  reduz o que um bug de interface consegue fazer — mas exige reescrever a camada de execução e o instalador.
  Classificado NEXT, com gatilho: antes de qualquer comando remoto (regra assinada) existir.
- **Comandos remotos:** só referenciam ids conhecidos e assinados; nunca texto executável (já é a regra do item 8).
- **Cadeia de fornecimento:** `cargo audit`/`npm audit` na esteira, lockfiles versionados (já), PresentMon com
  hash conferido no build, assinatura de código (decisão de compra do dono).
- **Atestado de release:** hash dos artefatos + commit + resultado dos testes publicado junto do release (o
  `.sha256` já existe); build reprodutível completo é PESQUISA (Tauri/MSVC não garantem bit a bit).

## 20. Shadow Mode

Pequeno e barato: toda decisão automática (portão, receita, compilador futuro) grava um **traço de decisão**
(entradas, regras avaliadas, candidatos, recusados, escolhido, versão da lógica). Uma lógica nova roda em sombra
sobre as mesmas entradas e grava "eu teria escolhido X", sem agir. Comparar é ler dois traços. Não precisa de
servidor.

## 21. Champion / Challenger

Na escala atual é o shadow mode + critério de promoção: a lógica desafiante só vira padrão se, nas máquinas do
beta, as decisões dela diferirem e as provas mostrarem resultado melhor ou igual com menos mudanças. Sem beta com
volume, a promoção é por revisão (o agente `revisor-otimiza`) e por teste com casos reais gravados.

## 22. Autopilot

Já existe a maior parte, com outro nome: guardião (ajuste desfeito por fora), deriva (driver/Windows mudou),
portão (desfaz o que piorou), modo seguro, perfil do PC. O "autopilot" é juntar isso num laço de **eventos →
revalidação**: driver novo → marcar provas deste contexto como "a revalidar" → na próxima partida, medição
automática compara → sem regressão, mantém; regressão, oferece prova. **Nunca**: BIOS, overclock, desligar
segurança, mexer em processo com anticheat, experimento sem permissão.

## 23. Self-Healing

Já existe: transação com diário e recuperação, modo seguro por quedas, WebView2 recarregando, histórico ilegível
guardado à parte, prova alternada que se recupera na abertura. Falta: **migração versionada** de todos os JSON
(hoje cada um tem `serde(default)` ad hoc) e **teste de caos** na esteira (processo morto no meio da transação,
arquivo truncado, disco cheio).

## 24. Internal Research Platform

Na escala de uma pessoa: o **caderno de pesquisa** é `docs/pesquisa/AAAA-MM-DD-hipotese.md` (hipótese, método,
máquinas, resultado, conclusão, próxima pergunta, "não retestar a não ser que…"), e os testes `#[ignore]` de
laboratório já criados (`dois_medidores`, `doctor_real`, `perfil_real`, `compartilhar_real`) são o console de
pesquisa. Um console web interno é LATER.

## 25. Competitive Moat

Escolha, com o motivo de cada um ser difícil de copiar:

| Ativo | Por que é fosso | Situação |
|---|---|---|
| **1. Instrumento de prova causal no jogo do cliente** (switchback com estatística, medidor certo, regras de comparabilidade) | Exige engenharia de medição que otimizador de tweak não tem e disciplina de não publicar ganho sem prova | Existe para 1 alavanca; generalizar é o slice |
| **2. Base de efeitos provados e negativos por contexto** (a tabela da seção 6) | Só acumula com uso real e com o instrumento 1; um concorrente começa do zero | Existe por máquina; global depende do opt-in |
| **3. Inteligência de medição por jogo** (ex.: FiveM dobra quadros no canal antigo; pools, crashes) | Descoberta empírica, não está em fórum | Começou (28/09) |
| 4. Encolhimento hierárquico (previsão com incerteza) | Matemática conhecida, mas só vale com 2; é multiplicador, não fosso sozinho | Não existe |
| Laboratório autônomo, SDK, API | Copiável com dinheiro, não com conhecimento | Rejeitado agora |

## 26. Do Not Build

| Ideia | Por que parece boa | Por que não |
|---|---|---|
| DSL textual de regras | "Regras versionadas e bonitas" | Parser, gramática e interpretador para dezenas de regras; mais superfície de ataque (regra remota = código); Rust tipado já dá versão, teste e revisão |
| Banco de grafos de conhecimento | "Knowledge graph" | Dados pequenos e tabulares; dependência nova sem ganho de consulta |
| Descoberta causal observacional | "Causalidade automática" | Confundidores não medidos (cena, servidor) fabricam causas |
| Otimização bayesiana / bandits no auto-tuner | "Busca inteligente" | Espaço pequeno, avaliação cara e ruidosa; explora em cima da experiência do cliente |
| Laboratório autônomo com fazenda de hardware | "Escala de pesquisa" | Sem máquinas e sem benchmark do jogo principal; o beta distribuído faz o mesmo mais barato |
| Inteligência de lançamento de driver | "Recomendamos o driver" | Sem laboratório e sem volume, seria opinião com gráfico |
| SDK / API pública / módulos de terceiros | "Ecossistema" | Sem demanda comprovada; módulo de terceiro com privilégio é risco máximo |
| Porcentagem de "previsão" na tela ("+14,3 FPS") | Vende | Número inventado; quebra a regra do produto |
| Fronteira de Pareto desenhada com estimativas | Tela bonita | Só com duas configurações provadas |
| Garantia comercial de ganho | Vende | O produto já garante o que dá para garantir: nunca menos FPS e voltar ao de antes |
| Monitoramento pesado permanente | "Dados completos" | Custo no jogo; o gravador por gatilho entrega o mesmo |

**Critérios de abandono** (valem para todo projeto experimental): sem efeito medido em ≥3 máquinas após N provas;
custo em jogo acima do orçamento; exige quebrar uma regra do produto; o mesmo resultado sai de algo mais simples.

## 27. Research Portfolio

| Classe | Projetos |
|---|---|
| Core | Escada de causalidade, política unificada, traço de decisão, nota de qualidade, migração de JSON |
| Incremental | Evidence pack por estratégia, estados de certificação, caderno de pesquisa |
| Differentiating | **Prova causal generalizada** (slice), auto-tuner guloso com parada sequencial, gravador de voo por gatilho, encolhimento hierárquico |
| Moonshot (separado, nunca no cliente sem prova) | Gerador de quadros próprio avançado, DPC/ISR por ETW, cadência de quadros nova |

## 28. Architecture Migration

Sem reescrever: cada passo é aditivo e mantém o caminho antigo até o novo provar.

1. Extrair `Trocavel` da prova alternada (interface: aplicar lado A, lado B, voltar ao de antes) sem mudar o
   comportamento do plano de energia — os testes atuais da prova são a rede.
2. Segunda implementação de `Trocavel`: modo jogo (slice).
3. Escada de causalidade como tipo (`Degrau`) usado pelo doctor, receita e prova.
4. Traço de decisão gravado pelo portão e pela receita.
5. Política unificada, chamada antes da execução; teste de arquitetura igual ao do `registro.rs`.
6. Só então: auto-tuner guloso usando 1–5.

## 29. Top 15 Initiatives

Fichas completas nas cinco primeiras (as que entram nesta fase); as demais em formato curto, porque dependem das
primeiras e a ficha delas muda com o que as primeiras ensinarem.

### I-1. Prova causal generalizada (switchback para qualquer alavanca trocável)

- **Problem:** só o plano de energia é provado no jogo; o resto espera semanas pelo portão ou nunca é provado.
- **Why it matters:** é o único caminho de suspeito a causa demonstrada; é o fosso 1.
- **Current limitation:** `provaalternada.rs` é fixo em planos de energia.
- **Proposed architecture:** trait `Trocavel { lado_a(), lado_b(), voltar(), descreve() }`; motor de rodadas
  compartilhado; uma implementação por alavanca.
- **Scientific basis:** experimento switchback (alternância com ordem balanceada ABBABAAB contra deriva linear),
  IC de Student com critério de não sobreposição.
- **Required data:** quadros do jogo pelo PresentMon por rodada; config, gerador e medidor por rodada.
- **Implementation complexity:** média (refatoração com rede de testes existente).
- **Runtime overhead:** só durante a prova (~6 min pedidos pelo cliente).
- **Security:** usa só ações já registradas e reversíveis.
- **Privacy:** nada sai do PC.
- **Compatibility:** alavancas com anticheat que recusa ficam fora (política).
- **Failure modes:** trocar no meio de cena que muda; alavanca que precisa reiniciar o jogo.
- **Fallback:** portão entre partidas (o de hoje).
- **How to test:** testes puros do motor com `Trocavel` falso; os testes atuais da prova de energia seguem verdes.
- **How to benchmark:** prova real no PC do dono com a alavanca nova.
- **Success:** duas alavancas provadas pelo mesmo motor; plano de energia sem mudança de comportamento.
- **Kill:** se nenhuma alavanca além do plano puder ser trocada a quente com segurança.
- **Dependencies:** nenhuma.
- **Expected moat:** alto. **Priority:** P0.

### I-2. Escada de causalidade como tipo

- **Problem:** diagnóstico, receita e relatório falam de causa com confianças soltas (Alta/Média/Hipótese).
- **Why it matters:** impede afirmar causa sem intervenção; dá vocabulário único à tela.
- **Current limitation:** `doctor::Confianca` mistura força da correlação com prova.
- **Proposed architecture:** `enum Degrau { Coincidencia, Correlacao, Provavel, Demonstrada, Desconhecida }` com
  regras de promoção (só `Demonstrada` via prova; `Provavel` exige mecanismo + precedência temporal).
- **Scientific basis:** hierarquia de evidência (associação → mecanismo → intervenção → reprodução).
- **Required data:** o que já existe + resultado da I-1.
- **Complexity:** baixa. **Overhead:** zero. **Security/Privacy:** nenhum impacto.
- **Compatibility:** mantém `Confianca` enquanto a tela migra.
- **Failure modes:** rótulo mais fraco que o anterior irrita ("provável" onde era "alta") — é o ponto.
- **Fallback:** o rótulo anterior. **How to test:** testes de promoção/rebaixamento.
- **Success:** nenhuma frase "causa" sem `Demonstrada` na tela. **Kill:** —.
- **Dependencies:** I-1 para o degrau `Demonstrada`. **Moat:** médio (disciplina). **Priority:** P0.

### I-3. Parada sequencial e poder na prova

- **Problem:** prova de tamanho fixo gasta tempo quando o efeito é grande e diz "indistinguível" sem dizer se
  conseguiria detectar algo.
- **Why it matters:** menos minutos do cliente por decisão; honestidade sobre o que a prova consegue ver.
- **Current limitation:** 8 rodadas fixas; sem "efeito mínimo detectável".
- **Proposed architecture:** após cada par de rodadas, decidir se o intervalo já separa ou já exclui o efeito
  mínimo que importa; relatar o efeito mínimo detectável quando indistinguível.
- **Scientific basis:** testes sequenciais em grupos (correção de alfa para olhadas intermediárias).
- **Complexity:** média. **Overhead:** zero. **Failure modes:** alfa inflado se a correção faltar.
- **How to test:** simulação com efeitos conhecidos (taxa de falso positivo ≤ 5%).
- **Success:** tempo médio de prova menor sem aumentar falso positivo simulado. **Kill:** se a economia for < 20%.
- **Dependencies:** I-1. **Moat:** médio. **Priority:** P1.

### I-4. Traço de decisão e modo sombra

- **Problem:** não se sabe, depois, por que o portão desfez algo ou por que a receita recusou.
- **Proposed architecture:** `TracoDeDecisao { id, versao_da_logica, entradas, regras, candidatos, recusados, escolhido }`
  gravado em anel local (últimas N); lógica desafiante grava o seu traço sem agir.
- **Complexity:** baixa. **Overhead:** gravação pequena fora do jogo. **Privacy:** local; entra no diagnóstico de
  suporte só resumido.
- **How to test:** mesma entrada → mesmo traço (reprodutibilidade).
- **Success:** todo "Desfazer" do portão tem traço. **Dependencies:** nenhuma. **Moat:** baixo (habilitador).
  **Priority:** P1.

### I-5. Política unificada

- **Problem:** as regras de permissão estão espalhadas; um caminho novo pode esquecer uma.
- **Proposed architecture:** `politica::decidir(proposta, contexto) -> Decisao { ALLOW|ASK|DENY|EXPERIMENTAL, motivo }`
  chamada pela execução; teste de arquitetura que falha se uma escrita não passar por ela.
- **Complexity:** média. **Security:** melhora (um ponto só). **Failure modes:** bloquear algo que antes passava
  — os testes de cada caminho existente são a rede.
- **Success:** anticheat, reversibilidade, memória, modo seguro e prova em andamento decididos num lugar.
- **Dependencies:** I-2. **Moat:** baixo (habilitador de autopilot seguro). **Priority:** P1.

### I-6 a I-15 (resumo)

| # | Iniciativa | Depende de | Prioridade | Gatilho / nota |
|---|---|---|---|---|
| 6 | Auto-tuner guloso pela menor mudança (seção 8) | I-1, I-3, I-5 | P2 | — |
| 7 | Nota de qualidade + decaimento da evidência (seção 16) | I-2 | P2 | — |
| 8 | Gravador de voo por gatilho do cliente (atalho "algo pareceu errado") | medir custo do PresentMon contínuo | P2 | liga para todos só dentro do orçamento |
| 9 | Explicação de tranco com processo em rajada (amostragem por processo na janela) | I-8 | P3 | custo medido |
| 10 | Encolhimento hierárquico (previsão com faixa) | I-7 + dados de ≥2 máquinas | P3 | opt-in do item 8 anterior |
| 11 | Estados de certificação + evidence pack por estratégia | I-4, I-7 | P2 | — |
| 12 | Autopilot de revalidação por evento (driver, build, versão do jogo) | I-1, I-4 | P3 | — |
| 13 | Migração versionada dos JSON + teste de caos na esteira | — | P2 | — |
| 14 | Broker privilegiado | I-5 | P3 | antes de qualquer comando remoto |
| 15 | Laboratório distribuído no beta (provas de voluntários voltando como linhas) | I-1, opt-in | P3 | voluntários definidos pelo dono |

## 30. Execution Order

```
I-1 Prova causal generalizada ──┬──► I-3 Parada sequencial ──┐
                                ├──► I-2 Escada de causalidade ──► I-5 Política ──┐
                                │                                                 ├──► I-6 Auto-tuner guloso
I-4 Traço de decisão ───────────┴─────────────────────────────────────────────────┘        │
                                                                                            ▼
I-13 Migração de JSON ──► I-7 Qualidade + decaimento ──► I-11 Certificação ──► I-12 Autopilot
Medir PresentMon contínuo ──► I-8 Gravador de voo ──► I-9 Tranco com processo
Opt-in (item 8 anterior) ──► I-15 Laboratório distribuído ──► I-10 Encolhimento hierárquico
I-5 ──► I-14 Broker privilegiado (antes de comandos remotos)
```

## Codebase Gap Analysis

| Área | Já existe | Reaproveitar | Refatorar | Substituir | Falta |
|---|---|---|---|---|---|
| Medição | PresentMon captura e `Vivo`, medição automática, eventos na janela | Tudo | — | — | Custo medido do contínuo |
| Comparabilidade | `evidencia::diferencas`, `so_do_medidor_atual` | Tudo | — | — | Nota de qualidade |
| Estatística | IC 95% Student, não sobreposição | Tudo | — | — | Sequencial, poder |
| Experimento | `provaalternada` (energia) | Motor de rodadas, recuperação na abertura | Extrair `Trocavel` | — | Outras alavancas |
| Diagnóstico | `doctor`, `causas`, `crashes`, `gargalo` | Regras | `Confianca` → `Degrau` | — | Processo em rajada |
| Conhecimento | `receita::lembrancas` (portão + prova) | Tudo | Tabela única com proveniência | — | Resultados de prova de qualquer alavanca |
| Política | `anticheat::permite`, `safety`, `retirado`, `em_andamento`, `modo_seguro` | Regras | Juntar num ponto | — | Teste de arquitetura |
| Conflitos | `conflitos.rs`, `grupos.rs` | Tudo | Declarar arestas na estratégia | — | Interação medida |
| Execução | `changelog`, `transacao`, `restaurar`, `guardiao` | Tudo | — | — | Broker (P3) |
| Autotuner | `autoajuste.rs` (um passo, padrão desfazer) | Filosofia | Usar a prova no lugar da medição simples | — | Busca e parada |
| Traço | Logs de texto | — | — | Log livre → traço tipado | Modo sombra |
| Dados | JSON com `serde(default)` | — | Versão explícita | — | Migração e caos |

## First Vertical Slice — Prova causal do modo jogo

**Escolha:** "O modo jogo realmente ajuda ESTE jogo neste PC?", respondido em minutos com o jogo aberto.

**Por que este e não outro:**
- Usa o ativo mais forte (a prova alternada) e força a peça que destrava quase tudo (I-1, `Trocavel`).
- A alavanca é trocável a quente, sem reiniciar, reversível, e **já é permitida com anticheat** (o governador
  acalma OUTROS programas, não toca o processo do jogo).
- Resolve um defeito real encontrado na revisão do item 3: o modo jogo reprovado num jogo fica parado nele para
  sempre, sem nova chance quando o contexto muda.
- Atende o público: FiveM limitado pelo processador é o caso em que acalmar programas tem mecanismo plausível.
- O Gravador de voo (a alternativa óbvia) depende de um número de custo que ainda não existe.

**Fluxo end-to-end:**
```
diagnóstico: processador limita (degrau Correlação)
  → política: modo jogo permitido? anticheat, modo seguro, prova rodando
  → prova alternada: lado A = sem governador, lado B = governador acalmando; ABBABAAB, 45 s
  → julgamento: IC 95%, mesmo medidor/config/gerador
  → degrau: Demonstrada (ganho) | sem efeito detectável | piorou
  → decisão pela menor mudança: ganhou → fica ligado para o jogo; senão → desligado e registrado
  → memória: linha nova (inclui negativo), reabre o "reprovado para sempre" quando a prova é nova
  → explicação na tela e no texto de compartilhar
```

**Testes previstos (quinta etapa):** unitários do motor com `Trocavel` falso; os da prova de energia sem mudança;
integração do fluxo com governador falso; falha no meio (processo morto → recuperação na abertura devolve o
estado); desempenho (custo do governador medido nas rodadas); regressão (nenhum teste antigo muda); benchmark
real no FiveM do dono; revisão de segurança (`revisor-otimiza` + `security-review`).

## Decisões que são do dono

| Decisão | Bloqueia |
|---|---|
| Opt-in de dados, endpoint e política de privacidade | I-10, I-15, clustering global |
| Voluntários do beta | I-15, certificação CANARY |
| Certificado de assinatura de código | Parte de I-14 e da cadeia de fornecimento |
| Publicar a 3.2 | Tudo que depende de dado de campo novo |
