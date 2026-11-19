# Plano 2.0: Otimiza leve e rápido (reestruturação)

> 19/11/2026. Pedido do dono: "refazer grande parte do código para melhor, melhorando muito a leveza no jogo do
> cliente e o FPS, não só no FiveM". Feito a partir de sete auditorias da equipe (só leitura do código, sem medir na
> máquina): custo do Otimiza com o jogo aberto, arquitetura do Frame Boost, FPS/Turbo/processador, arquitetura geral,
> vários jogos, concorrência e estado da arte, medição e testes. Os números de custo abaixo são **estimados** até a
> Onda 1 medir a linha de base.
>
> As regras do produto não mudam: **nunca menos FPS, tudo se desfaz, medido ≠ estimado, nada muda sem a pessoa
> pedir, a preferência do dono não vira padrão sem perguntar.**

---

## 0. O diagnóstico em uma página

**O Otimiza pesa no jogo que ele quer ajudar.** Com o jogo aberto rodam 11 temporizadores no Rust e mais os da
tela (≈150 despertares por minuto), até **4 PresentMon** ao mesmo tempo (cada um uma sessão ETW e um processo),
varreduras completas de processos a cada 3–6 s, um `powershell.exe` residente (60–90 MB), a janela WebView2
escondida mas perguntando ao Rust a cada 1,5 s (abrindo sessão do driver NVIDIA a cada pergunta) e arquivos JSON
lidos do disco até 10×/s pelo contador por cima do jogo. Estimativa: 400–500 MB na árvore do Otimiza.

**O Frame Boost gasta placa demais e mostra de menos.**
- Na GTX 1650, FiveM a 61 FPS em 180 Hz: ≈2,4 ms por quadro real + ≈1,3 ms por gerado ≈ **30% da placa**, a mesma
  do jogo. O grosso é refazer, em cada gerado, a escolha de vetor que podia ser feita uma vez por real
  (≈250 leituras de textura por pixel no `FinalBidirecional`).
- No modo padrão (Equilíbrio = Auto no ritmo do monitor), **a tela mostra "Auto (agora 1×)" enquanto gera**
  (`geracao/mod.rs:1993`, `FrameBoost.svelte:151`): o testador lê que nada está acontecendo. Provável causa direta
  do "muda bem pouco".
- O Equilíbrio já começa leve, mas a guarda usa a régua do modo normal (0,96) e no Auto desliga direto
  (`mod.rs:447`, `:1728`): some depois de 3 comparações num FiveM que perde 5–35%.
- Tudo numa thread (`sessao`, `mod.rs:1497-2364`): captura, fluxo, apresentação, log e guarda disputam o mesmo
  prazo; sem `Flush` depois do fluxo, ele só vai para a placa colado no `Present`.

**Os sistemas de FPS brigam entre si e as provas estão contaminadas.**
- Prioridade **alta** gravada no IFEO em toda abertura do jogo (`gamemode.rs:507`): pode picotar áudio e mouse.
- O Jogo liso grava o teto mesmo quando o teto é **da pessoa** (`teto_do_jogo.rs:66`).
- Limite da AMD (FRTC) global, fora do dono do teto (`amddriver.rs:39`).
- **Cinco caminhos escrevem no plano de energia**; energia de jogo, governador e afinidade continuam mexendo durante
  as provas A-B, então metade da prova recebe o EPP 0 e a outra não.
- Sete vigias e quatro detectores de jogo diferentes; nove motores de prova com critérios diferentes.
- O "Auto CPU Set" é afinidade obrigatória (não CPU set), provada com 3×10 s num jogo com ≈22% de ruído.

**O código dificulta tudo isso.** 124 mil linhas de Rust em 237 arquivos; `windows/mod.rs` com 4232 linhas,
`lib.rs` com um `setup` de ≈830, `App.svelte` com 2724. 110 `static`, 117 `spawn_blocking` + 30 threads + 24
tarefas. 49 chamadas de PowerShell e 40 `Get-CimInstance` (abrir um PowerShell custa 2,26 s, cada consulta WMI
mais de 1 s de processador no `WmiPrvSE`, medidos e anotados no próprio código). **Dois riscos de perder o
desfazer:** o Histórico tem dois donos (`frameboost.rs:642`, `rede.rs:80` gravam um `ChangeLog::load()` lido do
disco, fora da trava) e nove arquivos de estado lido na recuperação são gravados sem ser atômicos.

**Um jogo é descrito em cinco lugares** (`plataforma/jogos.rs`, `anticheat.rs`, `configmotor.rs`, `configjogo.rs`,
`ui/src/lib/catalogo.ts`) que discordam entre si, e o FiveM é reconhecido por `contains("fivem")` em sete pontos.

**A medição não prova a reestruturação.** Ninguém mede o peso do próprio Otimiza; "tranco" tem três definições; a
comparação estatística só vê diferença acima de 6–10% de FPS; os testes só rodam ao publicar uma tag; os 269
comandos Tauri não têm teste.

---

## 1. O que muda para o cliente (as metas)

| Meta | Hoje (estimado) | 2.0 |
|---|---|---|
| CPU da árvore do Otimiza com o jogo aberto, sem Frame Boost | não medido | **< 0,5% de um núcleo** (média de 60 s) |
| Memória da árvore com a janela escondida | 400–500 MB | **< 200 MB** |
| Processos filhos novos e PowerShell entre abrir e fechar o jogo | vários | **zero** |
| Sessões ETW do Otimiza | até 4 | **no máximo 1**, nenhuma órfã ao sair |
| Despertares do Otimiza | ≈150/min | **≤ 30/min** |
| Placa gasta pelo Frame Boost (GTX 1650, 1080p, 60 base, 180 Hz) | ≈30% | **≤ 15%** |
| Tela com Frame Boost (log "tela medida") | sem dado | quadros novos ≥ 95% dos Hz, repetidas ≤ 5%, ≤ 1 engasgo a cada 10 s |
| FPS do jogo com o Frame Boost (guarda) | perde 5–35% | razão ≥ 0,96 no Equilíbrio, sem desligar em 15 min |
| FPS/1% pior de cada onda contra a anterior (A-B pareado, 95%) | — | FPS ≥ −1%, 1% pior ≥ −2%, trancos/min ≤ +10% |
| Jogos com perfil de verdade | FiveM (+4 parciais) | FiveM, CS2, Fortnite, Valorant, Roblox, LoL, Apex, Minecraft Java |

Nenhum desses números vai para nota de versão ou site antes de medido na bancada (Onda 1).

---

## 2. A arquitetura alvo

```
app/        Tauri fino: comandos por domínio, UMA lista (tauri-specta também em produção), licença como atributo
nucleo/     Acao {ler_antes, aplicar, conferir, desfazer, temporaria}; Transacao (diário); Historico com DONO ÚNICO
            (um ator); Agendador (um tique, orçamento por tarefa, estados Ocioso/Partida); Barramento de eventos
            (JogoAbriu, JogoFechou, Medido); Erro tipado (precisa de admin / anticheat barrou / falhou)
partida/    o MOTOR DA PARTIDA: único vigia de jogo e único que escreve nos recursos de FPS; recursos com dono
            (teto, energia/EPP, prioridade/afinidade, fundo, geração/escala, fila do driver); ordem fixa de aplicar e
            desfazer; prova única pareada; orçamento de perda de FPS
frameboost/ captura | fluxo | apresentação | telemetria, em 4 threads; fluxo uma vez por real; composição barata
jogos/      manifesto por jogo (dado) + adaptadores por engine (RAGE/FiveM, Unreal, Source 2, Riot, Respawn, Roblox)
dominios/   energia, tela, placa, rede, processador, medicao, diagnostico, oti
so/         registro, serviços, powrprof, processos (UMA foto por tique), PDH único, WMI-COM, DXGI, NVAPI/ADL,
            ETW (um PresentMon compartilhado; depois consumidor ETW dentro do processo)
ui/         só assina eventos; dorme com o jogo aberto e a janela escondida
```

A migração é por **estrangulamento**: o código novo nasce ao lado do velho, assume uma responsabilidade por vez, e o
velho sai quando o novo provou. Cada onda sai como versão (1.0.9, 1.1.x) e **deixa a versão anterior desfazendo o
que ela mesma aplicou**: formato e nome dos arquivos de estado, nomes dos comandos Tauri e o `--restaurar-tudo` do
desinstalador não mudam (ou ganham um leitor da versão antiga).

---

## 3. As ondas

Cada onda: o que muda, quem implementa, quem revisa (skill `onda-segura`), o teste que trava, e o aceite. Quem
coordena é o `lider-tecnico`; nada é publicado sem o "pode publicar" do dono.

### Onda 0 · 1.0.9: o que dói hoje (pequeno, alto impacto)
1. **Frame Boost diz a verdade no Auto:** o multiplicador do Auto no ritmo do monitor vem dos quadros gerados de
   fato por real (hoje fica 1×). Teste puro com 61 reais/s em 180 Hz.
2. **Régua da guarda do Equilíbrio** coerente com o modo leve em que ele começa (pergunta 1 ao dono: quanto de
   FPS real o Equilíbrio pode perder, mostrado na tela).
3. **Prioridade do jogo:** "acima do normal" no lugar de "alta" no IFEO, e a alta antiga desfeita na abertura
   (pergunta 2: muda o padrão para todos).
4. **Teto da pessoa é da pessoa:** o Jogo liso recusa gravar sobre `Origem::Pessoa`
   (`decidir(Some(Pessoa 141), JogoLiso(72))` → `Recusar`).
5. **Provas sem contaminação:** energia de jogo, governador e afinidade param durante qualquer prova
   (`em_prova()`, `provaalternada::em_andamento()`).
6. **Histórico com um dono:** fim do `ChangeLog::load()` que grava; os nove `fs::write` de estado lido na
   recuperação passam pelo `utils/arquivo.rs` atômico.
7. **Tela que não pesa:** o painel do Frame Boost para de perguntar com a janela escondida (`visibilitychange`); o
   contador por cima do jogo guarda a configuração em memória em vez de ler 2 JSON 10×/s.
- Implementa: sessão principal. Revisa: `guardiao-do-fps`, `latencia-e-ritmo`, `especialista-processador`,
  `guardiao-da-escrita`, `revisor-otimiza`. Aceite: testes puros + log do sócio com "tela medida".

### Onda 1 · a bancada (antes de refazer qualquer coisa)
1. **Esteira em cada PR** (hoje só na tag): `cargo test`, `svelte-check`, `testes:ui`, fumaça.
2. **Testes de caracterização** que travam o comportamento antigo: `presentmon::resumir` e `frames::estatistica`
   sobre CSVs reais guardados no repositório; a guarda do Frame Boost com contagens de logs reais; as decisões de
   prova (`regressao`, `provaalternada`, `experimento`); o contrato dos 269 comandos contra uma cópia do
   `tipos.ts`; os `changes.json` antigos (25 variantes) ainda se desfazendo.
3. **Peso do próprio Otimiza no log:** a cada minuto de partida, CPU e memória da árvore (Otimiza, WebView2,
   PresentMon, PowerShell), despertares por tarefa, filhos abertos, sessões ETW. Linha de base medida da 1.0.9.
4. **Uma definição de tranco** (a do PresentMon, com piso proporcional ao FPS em vez de 50 ms fixo) e **comparação
   pareada** (diferença dentro de cada par A-B, bootstrap ou t pareado) no `repeticoes.rs`.
5. **Bancada sintética:** um executável D3D11 de teste com tempo de quadro controlado, para medir a razão de FPS e
   o p99 com o Otimiza parado, no laço e com Frame Boost, na própria esteira.
- Implementa: `engenheiro-de-qa` (testes) + sessão principal. Revisa: `cientista-de-medicao`.

### Onda 2 · modo partida (o Otimiza some do caminho do jogo)
1. **Agendador único** com orçamento por tarefa e estados Ocioso/Partida; os laços saem do `lib.rs`.
2. **Estado do jogo publicado uma vez** (`tokio::sync::watch`); um único detector; "fechou" por evento
   (`RegisterWaitForSingleObject` no PID), sem varrer processos durante a partida.
3. **Uma foto de processos** compartilhada, no máximo a cada 10 s na partida, abrindo só processos novos.
4. **Um PresentMon compartilhado** com contagem de uso (contador, guarda, medição); zero quando ninguém precisa.
5. **Zero PowerShell** entre abrir e fechar o jogo; a sessão residente fecha quando o jogo abre.
6. **A tela dorme** com o jogo aberto e a janela escondida; o Rust avisa por evento.
7. Preferências em memória; threads do Otimiza com EcoQoS na partida (exceto o gerador); guarda térmica com o
   amostrador vivo; medição automática com NVML a 1 Hz em vez do `\GPU Engine(*)` a 5 Hz.
- Revisa: `custo-do-otimiza`, `guardiao-do-fps`. Aceite: as metas de CPU, memória, filhos, ETW e despertares da
  seção 1, medidas na bancada e numa partida de FiveM.

### Onda 3 · Frame Boost 2.0: metade do custo na placa
1. `Flush` depois do fluxo; telemetria do gerador numa thread própria.
2. **Vetor escolhido uma vez por real** (em 1/2, `t = 0,5`) e **textura "parado"** uma vez por real: o
   `FinalBidirecional` cai de ≈250 para ≈20 leituras por pixel.
3. **Escala do fluxo ajustável** (como o Flow Scale do Lossless Scaling): fluxo em resolução reduzida, composição
   em resolução cheia; padrão escolhido por placa e resolução.
4. Câmera em redução paralela; ida e volta reduzidas (só 1/16, ou derivadas do campo simétrico) se o laboratório
   aceitar.
- Implementa: `engenheiro-de-shaders`. Revisa: `laboratorio-frame-boost`, `guardiao-do-fps`. Aceite: laboratório
  sem piora (erro na HUD/mira e fantasma contra "repetir o real"; PSNR dos quadros gravados ≥ atual − 0,05 dB) e
  "ms por real" −30% ou mais.

### Onda 4 · Frame Boost 2.0: o efeito que se vê
1. **Modo liso** (o que os concorrentes fazem para a geração aparecer): base do jogo travada em Hz÷N pelo perfil
   do driver (sem injetar nada) e geração ×N até exatamente os Hz; se a base não aguenta, baixa o N em vez de
   deixar a saída oscilar. Resolve "muda pouco" e "o jogo perde FPS" juntos: o custo do gerador cabe na folga da
   base travada.
2. **Recuo em movimento rápido** (como o Search Mode do AFMF 2): consistência ruim em área grande → repete ou
   mistura o real em vez de mostrar um quadro quebrado.
3. **Quatro threads** (captura | fluxo | apresentação | telemetria), par pronto atômico, fence entre fluxo e
   composição; captura com fila 0/1.
4. **Experimental:** `ALLOW_TEARING` e independent flip na sobreposição, só com o `PresentMode` do PresentMon
   provando.
5. **Atraso casado quadro a quadro** (o mesmo quadro: jogo apresentou × sobreposição exibiu): só então "medido".
- Revisa: `latencia-e-ritmo`, `especialista-tela`, `guardiao-do-fps`. Aceite: as metas de tela e de guarda da
  seção 1 na GTX 1650 a 144 e 180 Hz; atraso igual ou menor que a 1.0.8.

### Onda 5 · o motor da partida
1. `partida/`: único vigia e único escritor dos recursos de FPS; cada recurso com dono (o `teto_do_jogo` é o
   modelo); a pessoa vence o provado, que vence o palpite.
2. **Ordem fixa:** energia → fundo → núcleos → teto → escala → geração; desfaz na ordem inversa, pelo diário.
3. **Prova única** (`Experimento { variavel, lados, rodadas }` sobre a comparação pareada), com o resto congelado;
   o resultado vai para a receita do jogo com o contexto de driver e build do Windows, e é refeito se mudarem.
4. **Orçamento de perda** visível antes de aplicar (o Jogo liso tira até 15%, o leve do Frame Boost até X%).
5. Energia de jogo provada sozinha (hoje vai junto com a escala na prova do Turbo); afinidade com 8×30 s e
   avaliação de CPU set de verdade; placebos prováveis (prioridade de primeiro plano, MMCSS, timer, HPET) saem do
   "otimizar tudo" e vão para `experimento.rs`; o `CPMINCORES` do motor de energia vetado em X3D de dois blocos.
- Revisa: `guardiao-do-fps`, `especialista-processador`, `especialista-placa-e-driver`, `especialista-energia`,
  `cientista-de-medicao`.

### Onda 6 · o núcleo de ações reversíveis e o fim do PowerShell
1. Trait `Acao` envolvendo o `ChangeRecord` sem mudar o serde; os ≈13 recursos que guardam estado fora do Histórico
   (energia de jogo, banda, cores, Wi-Fi, governador, cpuset, teto…) migram um por commit, cada um com o teste
   "aplica → cai → abre → desfaz". "Restaurar tudo" vira um laço sobre o diário.
2. PowerShell/WMI → API nativa, um por vez e atrás de flag remota (`catalog::carregar_desligados_do_disco`):
   `powrprof` no lugar do `powercfg`, PDH no lugar do CIM GPUEngine, DXGI/IOCTL para placa e SSD,
   `ChangeServiceConfigW`, COM `ITaskService`, `EvtQuery`, WMI-COM dentro do processo. `bcdedit` e
   `NetQos`/`NetAdapter` ficam.
- Revisa: `guardiao-da-escrita`, `auditor-de-ajuste`, `auditor-de-seguranca`. Risco: semântica AC/DC do
  `powrprof` diferente do `powercfg` → prova A-B antes de ligar a flag.

### Onda 7 · jogo como dado (FPS em outros jogos)
1. **Um manifesto por jogo** (executáveis, engine, anticheat único com política de sobreposição, API gráfica, tela
   cheia, arquivos de configuração, chaves que pesam sem tirar visibilidade, limite de FPS nativo, Reflex, o que
   vale: Frame Boost, perfil do Turbo, teto do driver, energia). `catalogo.ts` gerado dele. Um só `identificar()`:
   saem os sete `contains("fivem")`; os dois enums de anticheat viram um.
2. **Adaptadores por engine** (`trait Motor { ler, prever, aplicar }`, com as regras de hoje: só chaves que já
   existem, recusa com o jogo aberto, arquivo inteiro guardado).
3. **Jogos, em ordem:** Roblox (limite oficial de 60/120/144/240; nunca "unlocker"), CS2 (confirmar chaves),
   Fortnite (modo Performance oficial), League of Legends (novo), Valorant (perfil revisto contra a lista oficial
   da Riot: hoje mexe em Sombras/Pós-processamento/Efeitos, que não estão nela), Minecraft Java, Apex. Call of
   Duty/Warzone só mede.
- Revisa: `pesquisador-de-jogo`, `especialista-fivem`, `auditor-de-seguranca` (anticheat).

### Onda 8 · a casa arrumada
Pastas por domínio (quebrar `windows/mod.rs` e `lib.rs`), comandos numa lista só com a licença como atributo, a
tela assinando eventos e `App.svelte` dividido; `release.yml`, `ci_coverage.rs` e os ≈30 testes com `include_str!`
atualizados. Mecânico e grande: só depois que as ondas 1–7 deixaram tudo coberto por teste.

### Onda 9 · telemetria opt-in (só com o sim do dono)
Desligada por padrão; um registro por sessão de jogo (versão, jogo do catálogo, faixa de placa/processador, Hz,
FPS/1% pior/trancos/clique→tela com a origem, custo do Otimiza, desfechos da guarda); nunca nome, máquina, IP,
processos ou caminhos; identificador aleatório trocado a cada 30 dias. Serve para ver tendência entre clientes;
nota de versão só com a bancada.

---

## 4. Ordem, tamanho e dependências

| Onda | Versão | Depende de | Tamanho estimado |
|---|---|---|---|
| 0 | 1.0.9 | — | pequeno (≈7 commits) |
| 1 | 1.1.0 | 0 | médio |
| 2 | 1.1.1 | 1 | médio-grande |
| 3 | 1.1.2 | 1 | médio |
| 4 | 1.1.3 | 3 | grande |
| 5 | 1.1.4 | 1, 2 | grande |
| 6 | 1.1.5–1.1.6 | 1, 5 | grande (8–12 commits só a parte das ações) |
| 7 | 1.2.0 | 1, 5 | grande |
| 8 | 1.2.x | 1–7 | grande, mecânico |
| 9 | quando o dono decidir | 1 | pequeno |

As ondas 2 e 3 podem andar em paralelo (áreas diferentes do código). Cada uma passa pelo log do sócio antes da
próxima ir para os clientes.

## 5. Riscos e como cada um é contido
- **Uma versão no meio da reforma piorar o jogo:** cada mudança de comportamento sai atrás de uma flag
  (`OTIMIZA_FG_*` ou flag remota) e só vira padrão depois do aceite A-B; a regra de volta é automática.
- **Perder o desfazer de quem já aplicou algo:** caracterização dos `changes.json` antigos na Onda 1; leitor da
  versão antiga sempre que um formato mudar.
- **Recurso migrado ficar aplicado depois do jogo** (Onda 6): toda `Acao` declara se é temporária e tem o teste
  "aplica → cai → abre → desfaz".
- **Anticheat:** nada de injeção, hook ou leitura de memória do jogo, em nenhuma onda; Valorant e Roblox só pelo
  arquivo de configuração oficial, com o jogo fechado.
- **O que só a máquina real prova** (ritmo, atraso, independent flip, custo na GTX 1650): vai para o sócio com o
  leitor de log; os números do plano são estimados até lá.

## 6. Perguntas ao dono
1. **Equilíbrio:** quanto de FPS real ele pode perder (mostrado na tela)? Recomendação: até 5% com o modo liso
   travando a base; acima disso alivia e avisa.
2. **Prioridade do jogo:** trocar a "alta" por "acima do normal" para todos (muda o padrão)? Recomendação: sim.
3. **Agendador novo:** numa versão só, ou com os laços antigos atrás de uma flag por uma versão? Recomendação: flag
   por uma versão.
4. **Canal antigo de quadros (`frames.rs`):** pode sair, já que o PresentMon é o medidor principal?
5. **Medição automática ligada por padrão:** manter (mais barata, Onda 2) ou deixar opcional?
6. **Telemetria opt-in** (Onda 9): quer?

## 7. Fontes
As externas estão na skill `referencias-frame-boost` (AMD FSR 3 e AFMF, Microsoft DXGI e HAGS, PresentMon,
Lossless Scaling, Blur Busters/LDAT para limitadores, Riot e Roblox para os perfis de jogo). As internas são
arquivo:linha nas auditorias, citadas acima.
