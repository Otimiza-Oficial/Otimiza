# PERFORMANCE INTELLIGENCE PLATFORM — RESULTADO DO PRIMEIRO VERTICAL SLICE

Versão analisada: branch servico-t1 · 29/09/2026 · slice "causa da travada" (PIP-ENGENHARIA.md, seções 8–11)

Etapas 12 a 18 do pedido. Tudo aqui é medido, por teste automatizado ou por laboratório nesta máquina (8 núcleos
lógicos). O que ficou sem medir está dito como tal.

## Leitura rápida

| Pergunta | Resposta medida |
|---|---|
| O motor novo acha uma causa plantada? | Sim, em 100% de 200 sessões sintéticas (critério: ≥ 90%) |
| Ele inventa causa quando não há relação? | 0,5% das vezes (critério: ≤ 5%). **O detector atual (V1) inventa em 57%** nos mesmos dados |
| O teste ativo confirma uma causa real? | 88,5% das vezes com 60% menos travadas (critério: ≥ 80%), em média 9,7 min |
| O teste ativo "confirma" causa falsa? | 3,0% (critério: ≤ 5%) |
| Quanto custa observar? | Otimiza 1,4–1,8% de UM núcleo; PresentMon 0,5–1,0% de um núcleo; análise 4–10 ms (laboratório real, 4 rodadas) |
| Em hardware real, a causa plantada foi achada? | **Não.** Em 4 rodadas o motor nunca inventou causa (4 de 4 corretas em não afirmar), mas a causa plantada não chegou a causar as travadas de forma distinguível — confirmado nos dados crus. O caminho positivo em hardware real **segue sem prova** |
| Defeitos que já estavam no produto | 2 corrigidos (P0): "ganho confirmado" com medidor dobrado; Mapa medindo pelo canal antigo |

## 12. Implementação

| Peça | Arquivo | O que faz |
|---|---|---|
| Motor de causa-raiz V2 (puro) | `modules/causadatravada.rs` | Travada por mediana local; episódios; lado (processador/placa) pelo CPUBusy/GPUBusy; 6 famílias de hipótese; associação por intervalo contra a taxa de base (Poisson-binomial exata, Bonferroni); evidência contra; abstenção; "não medido" |
| Teste ativo (puro) | `modules/testedatravada.rs` | Janelas de 30 s ABBA·BAAB; taxas quasi-Poisson; O'Brien-Fleming em 3 olhadas; melhoria mínima de 30%; guarda de FPS pareada; memória com resultados negativos |
| Casca Windows | `modules/windows/investigacao.rs` | Captura PresentMon + contadores no mesmo relógio (QPC); custo próprio; política; acalmar/devolver com anotação em disco; recuperação; vigia do fim do jogo |
| Tela | `src/travadas.ts`, `index.html` (#travadas) | Conclusão, conta visível, evidência contra, contrato do experimento, resultado com o que voltou |
| Integração | `commands.rs`, `lib.rs`, `governador.rs`, `restaurar.rs`, `registro.rs`, `release.yml` | 3 comandos; recuperação na abertura; medição automática e modo jogo pausam durante o teste; "Restaurar meu PC" desfaz; registro central; esteira roda os testes |
| P0 | `prova.rs`, `commands.rs` (Mapa) | Medidores diferentes não valem como prova; Mapa pelo PresentMon |

## 13. Testes

- Suíte inteira depois de todas as correções: **1.444 passaram, 0 falharam** (68 de laboratório ignorados); os testes
  de laboratório que mexem em processo (pânico, app morto) rodados à parte: passam.
- Critérios de aceite como testes (`aceite_deteccao_e_falso_positivo_em_simulacao`,
  `aceite_poder_e_falso_positivo_do_teste`), com sementes fixas e a verdade conhecida.

### O que falhou no caminho (registro honesto)

O critério do teste ativo foi escrito ANTES de implementar (PIP-ENGENHARIA, seção 11) e não foi mudado:

| Tentativa | Poder (60% menos travadas) | Falso positivo | O que se mudou |
|---|---|---|---|
| Desenho 1: 8 janelas, 4 min, 2 olhadas Bonferroni | **0,32** — reprovado | 0,02 | — |
| Grade de poder (exploração) | 0,33 (4 min) · 0,67 (8 min) · 0,82 (12 min) | — | decidiu o tamanho |
| Desenho 2: 24 janelas, 3 olhadas Bonferroni | **0,69** — reprovado | 0,025 | tamanho |
| Desenho 2 com O'Brien-Fleming | **0,78** — reprovado | — | forma de gastar o α (padrão de ensaios sequenciais) |
| + correção de defeito: a guarda de FPS parava o teste na 1ª olhada por ruído | **0,885** — aprovado | 0,035 | defeito de lógica, não de critério |

Cada rodada usou sementes novas. A lição que vale para a empresa: **um teste de 4 minutos, que parecia razoável, não
enxerga nem uma redução de 60% nas travadas em mundo aberto.** Qualquer "prova rápida" de concorrente, nesse regime,
é ruído.

## 14. Benchmark

### 14.1 Motor contra o método atual (entrega 18, "competitive test")

Mesmas 200 sessões sintéticas, rajadas de um programa em 30% do tempo e travadas **independentes** dele:

| Método | Aponta causa falsa |
|---|---|
| Detector atual (`core::travadas`, conta coincidências) | **57%** |
| Motor V2.1 (intervalo + taxa de base + correção) | **0,5%** |

E com causa real plantada, o V2 acerta 100%. O método tradicional (olhar o que coincidiu com a travada) erra mais da
metade das vezes quando não há causa — é exatamente o "diagnóstico" que otimizadores mostram.

### 14.2 Custo do motor

| Volume | Tempo (release) |
|---|---|
| Investigação de 60 s (5,4 mil quadros, 121 amostras, 5 hipóteses) | 1,7 ms |
| Teste de 12 min a ~180 FPS (130 mil quadros, 3 olhadas) | 62 ms |

### 14.3 Laboratório em hardware real (causa plantada)

Fonte de quadros: janela do Chrome com animação que gasta ~4 ms de processador por quadro (~180 FPS). Causa plantada:
`rajada.exe` disparando todos os núcleos 300–600 ms a cada 1,5–4 s. Caminho real do produto (`investigar_em`,
`testar_em`), PresentMon e contadores reais, elevado.

| Rodada | Montagem | Resultado |
|---|---|---|
| 1 | carga em prioridade normal | 10.788 quadros, 1 travada em 60 s → **abstenção** ("poucas para apontar uma causa") |
| 2 | carga acima do normal | 10.777 quadros, 5 travadas → **"causa ainda não determinada"**: nenhuma coincidia com a carga |
| 3 | fonte de quadros abaixo do normal, carga normal | 9.932 quadros, **43 travadas/min** (37 do lado do processador) → "causa ainda não determinada": a carga coincidiu com 14 de 43, o esperado ao acaso (lift 1,18) |
| 4 | igual à 3, com a associação por intervalo e captura crua gravada | 8.821 quadros, 54 travadas/min → "causa ainda não determinada" (lift da carga 0,99). **Contaminada por erro meu**: rodei a suíte de testes (cargo/rustc) ao mesmo tempo |

**O que os dados crus da rodada 4 mostraram** (replay offline da captura gravada):
- A carga plantada nunca passou de 36% da máquina: disputando com tudo em prioridade normal, as rajadas se diluem.
- As travadas acompanham a **carga total**: 3,74 por amostra com o processador acima de 60%, 0,27 abaixo. Esse limiar
  foi achado DEPOIS de olhar os dados: é hipótese para o próximo experimento, não conclusão.
- Pelo critério fixado antes (hipótese nova "processador inteiro ocupado", ≥ 90%), **não há associação** (lift 0,98).
  O motor disse "não determinada"; a leitura honesta dos dados é a mesma.

**Mudança de método que o laboratório provocou (V2.1):** a travada deixou de ser um instante (o começo do quadro) e
passou a ser o intervalo em que o atraso aconteceu. O `CPUStartQPC` do PresentMon marca o COMEÇO do quadro; um atraso
de 80 ms pode cruzar para a amostra seguinte, e o método por instante olhava a amostra errada. A nula agora leva a
duração de cada travada em conta (uma travada longa toca o sinal ao acaso com mais chance), com cauda
Poisson-binomial exata. Na simulação: falso positivo caiu de 1% para 0,5%, acerto igual.

Achado das rodadas 1 e 2: o Chrome sobe a prioridade das próprias threads de desenho e não trava com a carga. O motor
**não inventou causa** em nenhuma das quatro — o comportamento certo, e o que o método V1 não garante. O que falta é
uma fonte de quadros que se comporte como jogo (ou o próprio FiveM) para provar o caminho positivo fora da simulação.

## 15. Injeção de falhas

| Falha | Como foi forçada | Reação | Prova |
|---|---|---|---|
| Contador indisponível | amostras sem latência de disco | hipótese vira "Não medido", nunca "sem evidência" | `contador_que_nao_respondeu_e_nao_medido` |
| Contadores não cobrem as travadas | investigação sem amostras | abstém com "não cobriram", nunca "bom sinal" | `poucas_travadas_abstem` |
| PresentMon falha | — (erro propagado) | investigação recusa; **não cai no canal antigo** | código: `investigar_em` |
| Jogo fecha no meio do teste | quadros cortados na janela 5 | janelas seguintes inválidas com o motivo; julgamento com o que valeu | `jogo_fechado_no_meio_invalida_as_janelas_sem_quadro` |
| PC suspenso | 5 s sem quadro no meio da janela | janela inválida ("PC suspenso ou jogo pausado") | `pc_suspenso_no_meio_da_janela_invalida` |
| Um lado inteiro sem medida | janelas "acalmado" inválidas | decisão Inválido | `um_lado_inteiro_sem_medida_e_invalido` |
| Sem FPS pareado | só um lado válido por bloco | **não mantém** (antes mantinha — achado da revisão) | `sem_fps_pareado_nao_mantem` |
| Processo some antes de acalmar | pid inexistente | erro "fechou", nada anotado | `processo_que_sumiu_nao_e_acalmado_nem_anotado` |
| **Pânico no meio do teste** | `panic!` com processo real acalmado | `Drop` devolve a prioridade e apaga a anotação | `panico_no_meio_do_teste_devolve_a_prioridade` (processo filho) |
| **Otimiza morre com o programa mantido** | `Estado` descartado com `manter` | a abertura seguinte devolve pela anotação | `app_morto_com_programa_mantido_a_abertura_devolve` (processo filho) |
| Anotação corrompida | JSON quebrado | guardada à parte, erro dito, nada apagado | `anotacao_ilegivel_e_guardada_a_parte_e_dita` |
| Programa já fechou na abertura | anotação de pid inexistente | limpa e diz "já tinham fechado" | `na_abertura_processo_que_ja_fechou_limpa_a_anotacao` |
| Memória dos testes corrompida | JSON quebrado | erro (não "nunca testado"); sem teste oferecido; cópia guardada | `memoria_ilegivel_e_erro_e_nao_e_sobrescrita` |
| Acesso negado | — | erro com o motivo (programa como administrador) | código: `Estado::acalmar` |
| Sem APPDATA | — | teste recusa (não anota na pasta atual) | código |

## 16. Revisão de segurança e anticheat

**Revisor (`revisor-otimiza`): 5 bloqueadores, todos corrigidos antes do commit:**
1. O modo jogo podia acalmar o mesmo programa por cima do teste e deixá-lo em prioridade baixa para sempre → o
   governador não age durante o teste nem sobre o programa mantido (`reservado_pelo_teste`).
2. O vigia do fim do jogo desistia se uma investigação estivesse rodando → só o TESTE bloqueia a devolução, e a
   vigia tenta de novo.
3. Sem FPS pareado o teste mantinha, e a tela prometia "sem perder FPS" → não mantém sem FPS medido; o texto diz o
   limite real (FPS médio de janelas vizinhas não cair mais de 5%).
4. A tela dizia "nada ficou mudado" mesmo com devolução falhando → a frase do motor não afirma devolução; a tela só
   diz "voltou ao normal" quando voltou.
5. Memória ilegível virava "nunca testado" e era sobrescrita → erro, sem oferta de teste, cópia guardada.

**Segunda revisão:** os 5 confirmados como resolvidos; 1 bloqueador novo, corrigido — ao manter, se a devolução final
tivesse falhado num processo, acalmar de novo gravaria o estado do próprio teste como "anterior" (agora não mantém).
Também corrigidos: corrida entre a conferência do modo jogo e a marca do teste; aviso velho depois de devolução bem
sucedida; o comando de teste relê a memória. **Ficam abertos, com motivo:**
- Manter aceita até 5% de queda no FPS médio (estimativa pareada) e não mede o 1% pior: é decisão do dono.
- "Processador inteiro ocupado" recomenda fechar programas, mas o próprio jogo pode ser quem enche o processador; falta
  separar o uso do jogo na série.
- O Mapa agora depende do PresentMon (administrador), como o canal antigo também dependia; sem elevação, mostra o erro.
- Com outra captura do PresentMon rodando, o teste sai "inválido" (honesto, mas gasta o tempo do cliente).

**Segurança:**
- Superfície nova: 3 comandos IPC. `investigar_travadas` limita a duração (30–120 s); `testar_causa_da_travada` exige
  licença e **só aceita um processo que a última investigação deste jogo apontou (≤ 30 min)** — a tela não consegue
  mandar acalmar um processo arbitrário (antivírus, sistema).
- Privilégio: mexe só em prioridade e EcoQoS (memória do processo, some ao reiniciar), com o mesmo código já usado
  pelo modo jogo; lista de protegidos (sistema, antivírus, voz, transmissão, anticheats) e exclusão do jogo, da pasta
  dele e do próprio Otimiza.
- Arquivos: só em `%APPDATA%\pc-optimizer`, caminhos fixos, JSON com serde; nada vem da tela.
- Rede: nenhuma. Nada sai do PC. A memória guarda nome do programa, driver e build do Windows, localmente.
- PresentMon: executável de caminho fixo, argumentos fixos, prazo máximo (pré-existente).
- Condição de corrida: o pid é conferido pelo horário de início na devolução (não se devolve a outro processo que
  herdou o número). Janela residual: queda entre `acalmar` e a anotação (milissegundos) deixa a prioridade baixa até o
  programa fechar ou o Windows reiniciar — sem efeito persistente.

**Anticheat:**
- O jogo nunca é aberto por handle, injetado ou lido: quadros por ETW (PresentMon, de fora) e contadores do Windows.
- O teste mexe em OUTRO programa. Anticheats estão protegidos por nome e agora por prefixo (Tencent ACE/SGuard, EA,
  PunkBuster, GameGuard, XIGNCODE, miHoYo, EQU8, nProtect) e por qualquer nome com "anticheat" — ampliação que também
  vale para o modo jogo. **Risco residual:** um anticheat com nome desconhecido, apontado como suspeito, poderia ser
  acalmado; a maioria roda como serviço protegido e o Windows recusa (acesso negado, dito na tela).

## 17. Custo próprio (overhead)

| Medida (laboratório real, 60 s) | Valor | Orçamento (PIP-ENGENHARIA, seção 11) |
|---|---|---|
| Processo do Otimiza durante a investigação | 1,4% a 1,8% de um núcleo em 4 rodadas (≈ 0,2% da máquina de 8 núcleos) | < 2% de um núcleo — **dentro**, com pouca folga |
| PresentMon | 0,5% a 1,0% de um núcleo | — |
| Análise | 4–10 ms por investigação real; 62 ms para 130 mil quadros | — |
| Fora da investigação/teste | zero: nada novo roda contínuo | — |

O custo do Otimiza inclui o próprio medidor de custo; na primeira versão ele relia todos os processos a cada 0,5 s
(corrigido para reler só o PresentMon).

## 18. Experiência do cliente

Um painel no Início, "Por que o jogo trava": um botão, 60 s jogando. O resultado é uma frase ("Associação forte com o
OneDrive.exe…"), a conta visível, a evidência contra, e — só quando existe — o teste, com o contrato antes (o que
muda, por quê, duração, risco, como volta, o que é medido). Depois: "Causa demonstrada" com os números e "Devolver
agora", ou "Não encontramos melhoria confiável… não fica", e a memória lembra para não repetir. Complexidade
estatística escondida; conta disponível para quem quer.

## Valor para o negócio (sem números inventados)

- **Confiança:** o produto diz "não sei" e "não funcionou"; quando diz "causa demonstrada", foi testado na máquina do
  cliente. Isso é o contrário do "+40% de FPS" que o cliente já aprendeu a desconfiar.
- **Suporte:** a investigação responde a pergunta que mais chega no Discord ("por que trava?") com evidência que o
  atendente pode ler, em vez de uma lista de tweaks.
- **Menos mudança inútil:** só fica o que provou, e o que não provou não é repetido.
- **Diferenciação:** o teste mostrou que o método comum (coincidência) aponta causa falsa em 57% dos casos sem causa;
  a nossa, em 1%.

## O que NÃO está provado e o próximo passo

1. **O caminho positivo em hardware real não está provado.** A simulação mostra que o motor e o teste funcionam
   quando a causa existe; o laboratório mostrou que o motor não inventa causa quando ela não se distingue. Falta um
   caso real em que um programa cause as travadas. O teste certo é no FiveM, com a máquina quieta (sem compilação,
   meu erro na rodada 4) e, se possível, com um programa conhecido por disparar (atualizador, sincronizador de
   nuvem). Como a captura crua agora é gravada, cada sessão real vira caso de replay para as versões seguintes do
   motor (o "golden session dataset" do pedido).
2. **Nenhum dado de FiveM real ainda.** O próximo passo é o dono abrir o FiveM, usar "Investigar travadas" e mandar o
   resultado (a investigação não mexe em nada). A variância real entre janelas (φ) do FiveM é o número que diz se o
   desenho de 12 min basta.
3. Limitação conhecida: depois de "Manter", o portão e a regressão não sabem que um programa está acalmado pelo teste
   (a medição automática da partida seguinte inclui o efeito). Aceitável na primeira versão porque só dura até o jogo
   fechar; entra no traço de decisão (PIOS I-4).
4. A V1 (`core::travadas`) continua no Mapa; sai quando a V2 mostrar o mesmo desempenho em sessões reais gravadas.
