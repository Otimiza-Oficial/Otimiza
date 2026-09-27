# ULTIMATE PERFORMANCE SERVICE — MASTER PLAN

Versão analisada: main e61fbe2 (A3.2 pronta localmente, sem publicar) · 26/09/2026

Documento de direção. Nenhum código foi alterado para escrevê-lo. O plano técnico e as tarefas do
TOP 10 estão em `MASTER-PLAN-TOP10.md`.

## Leitura rápida

- **O salto não é fazer 110 recursos.** Quase tudo que a visão pede já existe no Otimiza como peça solta:
  medição por PresentMon, prova alternada no jogo, estatística com intervalo de confiança, portão
  "nunca menos FPS", diagnóstico de gargalo, crashes do FiveM, changelog reversível, modo seguro. O
  trabalho é **fundir as peças num motor com um contrato de evidência só** e expor isso como serviço
  contínuo: diagnosticar, prescrever, provar, lembrar e vigiar.
- **Escala real.** Um dono, desenvolvimento assistido, um bot de vendas, um site e clientes na maioria
  de FiveM no Brasil, com compra única por chave e licença que funciona sem servidor. Tudo que exige
  backend de telemetria, equipe de suporte ou coortes vai para NEXT ou LATER, com o pré-requisito escrito.
- **A 3.2 sai primeiro.** O código está pronto e o ensaio de publicação passou; falta o teste do dono no
  PC dele com FiveM. Começar plataforma nova com a versão parada seria o erro clássico.
- **Métrica principal:** CPCR, a fração de clientes com **melhoria comprovada e nenhuma regressão
  aberta** (seção Q). É a única que o marketing não consegue inflar.
- **Rejeitado com motivo:** launcher próprio, ML agora, "bata seu recorde" em mundo aberto, controlador
  que muda gráfico sozinho, serviço SYSTEM residente, Copilot com modelo de linguagem agora, coleta sem
  consentimento (seção T).

## Premissas e regras que não mudam

- Nunca menos FPS. Nunca limite de FPS automático. Nada que tire FPS para "ganhar" em outra métrica sem
  o cliente escolher.
- Nenhum número inventado: ganho só com medição válida; sem medição, a tela diz "não medido".
- Toda mudança no Windows passa por changelog/transação e tem volta; o que não tem volta não entra.
- Nada de handle, injeção ou hook em processo de jogo com anticheat. Nunca escrever BIOS/firmware.
- Dado do cliente só sai do PC com ação dele (diagnóstico copiado) ou consentimento explícito.
- Publicação só com ordem do dono; verificador de publicação antes de qualquer tag.

## Inventário: o que já existe

| Capacidade | Onde está hoje | Estado |
|---|---|---|
| Medição de quadros | `windows/presentmon.rs` (captura e sessão contínua `Vivo`), `medicoes.rs` (partida automática), `frames.rs` | Confiável; separa quadro do jogo de quadro gerado |
| Estatística | `core/estatistica.rs`, `repeticoes.rs` (IC 95% com t de Student) | Regra de amostra: 1% piores ≥1000 quadros, 0,1% ≥10000 |
| Prova no jogo | `provaalternada.rs` (ABBABAAB, aquecimento 60 s), `prova.rs` | Só plano de energia hoje |
| Validação entre sessões | `portao.rs`, `regressao.rs`, `deriva.rs`, `historico.rs` | Portão decide o modo jogo |
| Experimento | `windows/experimento.rs`, `grupos.rs`, `autoajuste.rs`, `motorenergia*.rs`, `cpuset.rs` | Correção de deriva térmica no motor de energia |
| Diagnóstico | `gargalo.rs`, `bottleneck.rs`, `veredito.rs`, `causas.rs`, `core/travadas.rs`, `core/fluidez.rs`, `streaming.rs`, `crashes.rs`, `vram.rs`, `pressao.rs` | Espalhado em telas diferentes |
| Sensores | `core/telemetria.rs` (PDH), `core/pdh.rs`, `nvml.rs`, `core/sensores.rs`, `thermal.rs` | NVIDIA completo; AMD/Intel sem SDK no repositório |
| Tela e jogo | `display.rs`, `vrr.rs` (EDID), `janelas.rs`, `tetos.rs`, `deteccao.rs`, `jogos.rs`, `configjogo.rs`, `citizenfx.rs`, `fivem.rs`, `anticheat.rs` | FiveM é o jogo mais coberto |
| Confiabilidade | `changelog.rs`, `transacao.rs`, `registro.rs`, `utils/diagnostico.rs`, `utils/webview.rs`, `planoenergia::vistoriar` | Modo seguro depois de 2 quedas seguidas |
| Suporte | `suporte.rs` (anonimizado, sem SID), `labcompat.rs`, `report.rs`, `naofazemos.rs` | O cliente vê o texto antes de mandar |
| Licença | `licenca.rs`, `licenca_prova.rs` | Assinatura local, sem servidor |
| QA | 1373 testes marcados `#[test]`, guardas de arquitetura, `npm run fumaca`, verificador de publicação | CI no GitHub Actions |

## A. Visão

O Otimiza deixa de ser "o programa que aplica ajustes" e vira **o serviço que responde, com prova, três
perguntas de quem joga**:

1. **Por que meu jogo roda assim?** Diagnóstico com evidência desta máquina, incluindo quando a resposta
   é "é o limite do seu hardware" ou "é o servidor".
2. **O que muda se eu mexer?** Cada ação tem hipótese, risco e forma de prova; o resultado vem medido no
   jogo do cliente, e o que piorou é desfeito.
3. **Continua bom?** Depois de atualização do Windows, driver novo, config de jogo nova ou calor, o
   serviço percebe e diz o que mudou.

Três estágios: **diagnosticar e provar** (NOW), **cuidar continuamente** (NOW/NEXT), **aprender** (NEXT
por máquina, LATER com dados agregados e consentidos).

"Ser o melhor" só vale se for medível. As oito dimensões e como cada uma se mede:

| Dimensão | Como se mede | Hoje |
|---|---|---|
| Resultado comprovado | CPCR (seção Q) | Não medido fora do PC do cliente |
| Estabilidade | Sessões do Otimiza sem queda; crashes de jogo com Otimiza no período de 48 h | Local (`diagnostico.rs`, `crashes.rs`) |
| Segurança | Incidentes de ban, boot quebrado ou dado perdido atribuíveis: meta 0 | Sem registro formal |
| Transparência | % das mudanças com tela "o que muda", volta testada e registro no changelog: meta 100% | Guarda `registro.rs` cobre as escritas conhecidas |
| Personalização | % das decisões tomadas com medição desta máquina | Só energia e modo jogo |
| Suporte | Tempo do primeiro contato até a causa identificada | Não medido |
| Compatibilidade | Jogos e hardware com teste registrado | FiveM/GTA e NVIDIA |
| Melhoria contínua | Versão N+1 não regride no laboratório | Laboratório de 1 PC |

## B. Gap Analysis

| Lacuna | Por que importa | Onde entra |
|---|---|---|
| 1. Sem contrato único de sessão e evidência | Cada módulo mede do seu jeito; prova, portão e diagnóstico não se falam | TOP 1 |
| 2. Sem memória de efetividade por máquina | O Otimiza pode oferecer de novo o que já piorou esta máquina | TOP 3 |
| 3. Diagnóstico espalhado em telas | O cliente precisa juntar gargalo, trancos, calor e crash sozinho | TOP 2 |
| 4. Vigilância de deriva só no plano de energia | Atualização do Windows desfaz ajustes em silêncio | TOP 5 |
| 5. Desinstalação não oferece restaurar | Quem sai leva mudanças que não sabe que tem | TOP 6 |
| 6. Sem relatório pós-partida | O valor contínuo não aparece para o cliente | TOP 7 |
| 7. Zero métrica agregada | A empresa não sabe se o produto funciona na base | TOP 8 |
| 8. Laboratório de 1 PC | Regressões em AMD, Intel e notebook só aparecem no cliente | Seção L |
| 9. Sem certificado de assinatura de código | SmartScreen assusta na instalação; é custo, não código | Decisão do dono |
| 10. `anticheat::permite` só é chamado em 3 lugares (`afinidade.rs`, `frames.rs`, `gamemode.rs`) | Um item novo pode esquecer a checagem | Seção O |

## C. Customer Journey

```
Download ─► Instalação ─► 1ª execução ─► Diagnóstico ─► Otimização ─► Jogo
   │            │              │              │             │           │
 site +     NSIS, aviso    perfil em       Doctor com   prescrição   medição
 hash       SmartScreen    ~20 s, nada     60–90 s de   com prova    automática
 publicado  até haver      alterado        jogo                      e custo
            certificado                                              medido
                                                                        │
Desinstalação ◄─ Recuperação ◄─ Atualização ◄─ Suporte ◄─ Manutenção ◄─ Validação
 oferece         modo seguro,    aviso; beta    diagnóstico  guardião de   prova
 restaurar       "Restaurar      opt-in         anonimizado  deriva        alternada
 tudo            meu PC"                                                   e portão
```

| Etapa | O que o cliente deve sentir | O que já existe | Falta |
|---|---|---|---|
| Download | "é o arquivo certo" | Hash `.sha256` no release | Certificado de código |
| Instalação | "é só avançar" | NSIS em português | — |
| 1ª execução | "ele entendeu meu PC" | Leituras soltas de hardware | Tela de perfil (TOP 10) |
| Diagnóstico | "agora sei por quê" | Gargalo, trancos, crashes em telas diferentes | Doctor (TOP 2) |
| Otimização | "sei o que vai mudar e como volta" | 5 passos da 3.2 | Prescrição ligada à causa (TOP 3) |
| Jogo | "nem percebo que está rodando" | Medição automática | Orçamento medido (TOP 7) |
| Validação | "está provado" | Prova alternada de energia | Prova para outras ações (TOP 3) |
| Manutenção | "ele avisa quando piora" | Deriva, regressão, vistoria de energia | Guardião geral (TOP 5) |
| Suporte | "resolveram rápido" | Diagnóstico anonimizado | Fluxo "piorou depois de otimizar" |
| Atualização | "confio em atualizar" | Aviso pelo GitHub | Canal beta (TOP 8) |
| Recuperação | "dá para voltar tudo" | `revert_all`, modo seguro | Botão único e verificação (TOP 6) |
| Desinstalação | "saiu limpo" | Desinstalador NSIS padrão | Oferecer restaurar (TOP 6) |

## D. Arquitetura técnica

```
            ┌──────────────────────── Interface (TS) ────────────────────────┐
            │ decide só por enum/bool; nunca por texto do backend            │
            └───────────────▲────────────────────────────────▲──────────────┘
                            │ comandos async (fora da thread da janela)
┌─────────────┐   ┌─────────┴────────┐   ┌──────────────┐   ┌──────────────┐
│ 1 Sensores  │──►│ 2 Sessão de jogo │──►│ 3 Diagnóstico│──►│ 4 Decisão    │
│ PresentMon, │   │ SessaoDeJogo:    │   │ Doctor:      │   │ estratégias  │
│ PDH, NVML,  │   │ quadros+contad.+ │   │ ≤3 causas c/ │   │ declarativas │
│ EDID, logs  │   │ eventos, 1 janela│   │ evidência    │   │ + porteiros  │
└─────────────┘   └──────────────────┘   └──────────────┘   └──────┬───────┘
                                                                   ▼
┌──────────────────────┐   ┌────────────────────┐   ┌──────────────────────┐
│ 7 Conhecimento       │◄──│ 6 Validação        │◄──│ 5 Aplicação          │
│ identidade + memória │   │ prova alternada,   │   │ changelog+transação+ │
│ de efetividade (JSON │   │ portão, regressão  │   │ registro; sempre com │
│ local versionado)    │   │ KEEP/REVERT/ROLLB. │   │ volta                │
└──────────────────────┘   └────────────────────┘   └──────────────────────┘

Online, opcional, sem ponto único de falha: aviso de versão (GitHub) · regras assinadas
(kill switch) · telemetria opt-in. Sem internet, tudo acima continua funcionando.
```

Regras da arquitetura:

- **Um dono por camada.** Sensores não decidem; Decisão não mede; Aplicação não interpreta resultado.
- **Contrato de evidência único** (`SessaoDeJogo`): toda afirmação na tela aponta para uma sessão
  medida, com fonte (PresentMon ou reserva), quantidade de quadros, gerador ligado ou não e config do jogo.
- **Estratégia declarativa**: id, causa que trata, pré-condições (anticheat, notebook, build, GPU),
  risco, como aplicar, como desfazer, como provar. O porteiro de anticheat é obrigatório no tipo.
- **Estado local versionado**: todo JSON em `%APPDATA%\pc-optimizer` ganha campo de versão e migração
  testada; arquivo corrompido vira "começa do zero com aviso", nunca pânico.

**Rejeitado:** serviço do Windows separado rodando como SYSTEM. Aumenta a superfície de ataque e de
elevação, exige instalador e atualização mais complexos, e o ganho é pequeno: o app na bandeja já
acompanha a partida.

## E. Service Architecture

Papéis na escala de hoje:

| Papel | Quem | Responsabilidade |
|---|---|---|
| Produto e release | Dono | Prioridade, decisão de publicar, preço |
| Suporte nível 1 | Dono + bot | Ticket no Discord, pedir o diagnóstico, triagem |
| Engenharia e QA | Desenvolvimento assistido | Código, testes, revisão (`revisor-otimiza`), verificadores |
| Laboratório | PC do dono + 2–3 voluntários do canal beta | Validar em hardware diferente antes do estável |

Fluxo de melhoria: **ticket → etiqueta fixa → issue no GitHub → correção com teste que falha antes →
nota de versão → FAQ do bot**.

Incidentes:

| Nível | Exemplo | Resposta |
|---|---|---|
| SEV-0 | Risco de ban, boot quebrado, dado perdido | Desligar a estratégia por regra assinada na hora; aviso no Discord; hotfix |
| SEV-1 | Crash do Otimiza em muitos PCs, regressão de FPS confirmada | Hotfix em até 48 h |
| SEV-2 | Tela errada, leitura falha em um hardware | Próxima versão |
| SEV-3 | Texto, cosmético | Quando der |

Postmortem sem culpa em `docs/postmortems/AAAA-MM-DD-nome.md`: o que aconteceu, linha do tempo, causa,
por que os testes não pegaram, o teste que entrou.

## F. Performance Doctor

**Ficha**

- **Name:** Performance Doctor (com FiveM Lab).
- **Customer Problem:** "Meu jogo trava / o FPS é baixo e não sei por quê."
- **Business Value:** transforma o primeiro uso em resposta concreta; reduz ticket de "não melhorou".
- **Technical Value:** junta 10 módulos de diagnóstico num resultado só, com o mesmo contrato de evidência.
- **Architecture:** o cliente escolhe o jogo → uma `SessaoDeJogo` de 60–90 s jogando → regras puras sobre
  a sessão: gargalo (CPUBusy × GPUBusy do PresentMon), trancos (gravidade e coincidência com disco,
  paginação, processo, driver), tetos (limite, V-Sync, Hz), energia e calor (PDH `Performance Limit
  Flags`, NVML), VRAM, crashes recentes, modo de apresentação → **no máximo 3 causas**, cada uma com
  evidência, confiança, "o que ajuda" e **"o que não vai ajudar"**.
- **Dependencies:** TOP 1.
- **Risks:** causa errada dita com confiança. Mitigação: sem sinal medível, a causa sai como
  "hipótese" e não entra na prescrição automática.
- **Security Impact:** nenhum; só leitura.
- **Anti-Cheat Impact:** nenhum; PresentMon lê eventos ETW sem abrir o processo do jogo.
- **Performance Overhead:** o do PresentMon durante 60–90 s (medido hoje em captura; meta: medir e
  publicar no documento do item).
- **How We Test:** sessões gravadas (CSV do PresentMon + contadores) como fixtures; cada regra tem caso
  positivo e negativo; o PC do dono em FiveM com gargalo conhecido.
- **Success Criteria:** nas fixtures, a causa principal bate com a rotulada à mão; nenhuma causa sem
  evidência aparece como certa.
- **Failure Criteria:** o Doctor diz "processador" onde a fixture é GPU; ou a tela mostra número que não
  veio de medição.
- **Rollback:** a tela antiga (Início com veredito) continua; o Doctor é uma tela nova.
- **Priority:** NOW.

"Rede" e "engine" não têm sinal confiável no que medimos: entram só como hipótese rotulada, nunca
como causa. **FiveM Lab** vem primeiro, porque é a base de clientes: crashes (`crashes.rs`), pools
estourados, veículo repetido, config do jogo (`configjogo.rs`), e a frase honesta "é o servidor" quando a
evidência aponta para lá.

## G. Performance Prescription

Cada causa do Doctor gera ações candidatas. Uma ação só entra se tiver:

| Campo | Exemplo |
|---|---|
| Causa que trata | Gargalo de CPU com núcleo principal a 100% |
| Métrica que deve mexer | 1% piores quadros |
| Risco | Baixo / Médio (com o motivo) |
| Reversível | Sim, pelo changelog |
| Como será provada | Ao vivo por prova alternada, ou entre sessões pelo portão |
| Pré-condições | Sem anticheat que recuse, não é notebook na bateria, build do Windows |

"Aplicar recomendados" executa → mede → decide:

- **KEEP**: melhora significativa (intervalos sem sobreposição) e nenhuma métrica guardiã piorou.
- **NEUTRO**: dentro do ruído. Fica **só** se não custa nada e o cliente escolheu manter; o padrão é voltar.
- **ROLLBACK**: piorou qualquer métrica guardiã. Desfaz na hora e grava na memória de efetividade.

**Regra dura:** nenhuma ação sem hipótese ligada a uma causa. O "aplicar tudo" do catálogo antigo não volta.

## H. Session Intelligence

Relatório pós-partida montado a partir das janelas já medidas pela medição automática (janelas a cada N
minutos, não medição contínua): FPS do jogo, 1% piores, trancos com o instante, gargalo dominante, calor,
e o que estava diferente (driver, config, gerador).

**Orçamento, medido antes de prometer "zero distração":**

| Recurso | Meta durante a partida |
|---|---|
| CPU do Otimiza | média < 1% de um núcleo fora das janelas de medição |
| Disco | nenhuma escrita além do log e do resultado da medição |
| Rede | nenhuma |
| Interface | sem animação, sem checagem de versão, sem PowerShell |

Se a medição do próprio custo não bater a meta, o recurso não sai.

## I. Performance History

Linha do tempo por jogo: medições (`historico.rs`) e eventos (driver, build do Windows, versão do
Otimiza, config do jogo, mudança aplicada ou desfeita). A resposta a "quando piorou?" já existe em
`historico.rs` e `deriva.rs`: coincidência no tempo sai como **suspeito**, não como causa.

**Crítica:** comparar FPS de partidas em lugares diferentes do mapa engana. A história mostra
**tendência com dispersão**, e só afirma queda com o critério de `deriva.rs`.

"Recorde pessoal" só de **prova alternada** ou benchmark de cena fixa. **Rejeitado:** "bata seu recorde"
em mundo aberto (FiveM): o recorde seria sorte de cena, e o cliente seria treinado a desconfiar do número.

## J. Machine Intelligence

**Identidade de performance** (sem dado pessoal): CPU, GPU, RAM, disco, build do Windows, driver de
vídeo, BIOS (versão, só leitura), monitores e Hz, notebook ou não, comportamento térmico e de energia
observado, jogos e API gráfica, gargalos típicos.

**Memória de efetividade:** para cada estratégia × contexto (jogo, driver, build), o último resultado
(KEEP, NEUTRO, ROLLBACK) com o intervalo de confiança e a data. Regras:

- Estratégia que piorou neste contexto **não é oferecida de novo automaticamente**.
- Volta a ser candidata só se o contexto mudar (driver, build ou jogo), e a tela diz por quê.
- O cliente vê e pode apagar a memória.

Isso é regra determinística com dados desta máquina. Não é "IA".

## K. Global Knowledge

**LATER**, e só com consentimento explícito. Esquema mínimo: hardware em faixas (não o modelo exato
quando raro), build, jogo, estratégia, efeito com intervalo. Sem identificador persistente de máquina.
Filtros contra ruído: amostra mínima, prova válida, sem gerador misturado, máquina estável (sem queda
recente).

Pré-requisitos: TOP 8 (telemetria opt-in), política de privacidade publicada, base ativa grande o
bastante para o dado dizer algo.

Até lá, o "conhecimento global" é o **banco curado à mão** por jogo (configs conhecidas, anticheat,
estratégias proibidas), versionado no repositório e distribuído como regra assinada.

## L. Experimentation

```
hipótese ─► pesquisa ─► protótipo ─► lab (PC do dono + voluntários) ─► canal beta opt-in ─► estável
                                            │                               │
                                       prova alternada                 provas dos
                                       em cada máquina                 voluntários
```

- Toda estratégia nova nasce **desligada por padrão** e com id próprio.
- Regra remota assinada pode **desligar** uma estratégia por id. Nunca executa código, nunca liga algo
  que a versão instalada não conhece.
- Coortes percentuais (liberar para 5%, 25%...): LATER. Sem telemetria com volume, não há como ler o
  resultado de uma coorte.

## M. Reliability

Já temos: changelog e transação, rollback, modo seguro (`QUEDAS_PARA_MODO_SEGURO = 2`), relatório de
pânico, recuperação do WebView2, prova alternada que se recupera na abertura.

NOW:

- **Guardião de deriva** para toda mudança gerenciada (TOP 5).
- **Testes de caos na CI:** processo morto no meio da transação, JSON de histórico/perfil corrompido,
  disco cheio, pasta sem permissão, internet caindo no meio da checagem de versão.
- **Migração versionada** dos JSON locais, com teste para cada versão antiga.

SLOs (por exemplo "99,5% das sessões sem queda") **só depois de ter o baseline** da telemetria opt-in.
Escrever o número antes seria inventar.

## N. Security

- **Revisão formal** antes do NEXT: fronteira de elevação (o que roda como administrador e por quê),
  comandos com lista fixa, validação do IPC, carga de DLL por caminho absoluto (PresentMon, NVML),
  arquivos temporários, logs sem segredo.
- **Regras remotas:** formato declarativo, assinatura Ed25519 (a mesma infraestrutura da licença),
  número de versão que só sobe, tamanho limitado, campos conhecidos. Regra inválida é ignorada e anotada.
- **Certificado de assinatura de código:** custo e decisão do dono (`docs/app/ASSINATURA.md`).
- **Repositório público:** nada de SDK com licença restrita (ADLX da AMD) em código derivado aqui.

## O. Anti-Cheat Safety

- `anticheat::permite` vira **porteiro obrigatório no tipo da estratégia**: não compila uma estratégia
  que mexe em processo ou em chave de execução sem declarar a ação de anticheat. Hoje só 3 módulos chamam.
- Jogo desconhecido com anticheat detectado = **modo somente leitura** para ações que tocam o processo.
- Nada de handle, injeção ou hook em processo de jogo protegido. O gerador de quadros continua externo,
  por captura.
- Matriz de compatibilidade por jogo no banco curado: o que é seguro, o que é proibido, fonte da regra.

## P. Support

- O cliente gera o diagnóstico (já existe, anonimizado, sem SID) e cola no ticket. O atendente vê:
  identidade da máquina, mudanças aplicadas, provas, crashes, quedas do Otimiza, últimos avisos do log.
- **Fluxo "piorou depois de otimizar"** (automático, antes de qualquer sugestão): driver ou Windows
  mudou? config do jogo mudou? calor? as medições são comparáveis (mesmo gerador, mesma fonte)? processo
  novo em segundo plano? Só depois disso sugere desfazer, e diz o que desfazer.
- **Restaurar meu PC:** desfaz só o que é do Otimiza (`revert_all`), mostra item a item o que voltou e o
  que não pôde voltar. Também oferecido pelo desinstalador (TOP 6).

**Rejeitado agora:** "Copilot" com modelo de linguagem. Risco de inventar diagnóstico, custo por uso e
dado saindo do PC. A explicação em linguagem simples sai de **texto gerado das regras e das evidências**,
determinístico e testável. RESEARCH depois, e só sobre dados estruturados, sem poder de executar ação.

## Q. Company Metrics

**Métrica principal — CPCR: clientes com melhoria comprovada e sem regressão.**

> % dos clientes ativos nos últimos 30 dias que têm **pelo menos uma prova válida** (IC 95%, amostra
> mínima atendida) mostrando ganho em FPS do jogo ou em 1% piores, **e nenhuma regressão confirmada sem
> reverter** no mesmo período.

Por que esta: exige prova (não conta instalação nem clique), pune regressão, e não sobe com marketing.

Métricas guardiãs:

| Métrica | Protege contra |
|---|---|
| Sessões do Otimiza sem queda | Ganho à custa de estabilidade |
| Taxa de ROLLBACK pela validação | Prescrição ruim escondida |
| % das partidas medidas pelo PresentMon | Medição pela reserva, menos confiável |
| Tempo até a causa no suporte | Suporte lento |
| Reembolsos | Promessa que o produto não cumpre |

**Proibido:** métrica de vaidade ("milhões de otimizações aplicadas").

Hoje nada disso é medido fora do PC do cliente. Até o TOP 8, o CPCR existe **por máquina** (na tela) e no
diagnóstico de suporte.

## R. Competitive Moat

| Vantagem | Por que é difícil copiar |
|---|---|
| 1. Prova estatística no jogo do cliente, com rodadas alternadas | Exige medição separando quadro gerado, estatística e disciplina de não publicar ganho sem prova |
| 2. Memória de efetividade por máquina | Só acumula com uso real; concorrente de "tweak" não mede |
| 3. Diagnóstico que diz a causa e o limite do hardware | Admitir "é o seu hardware" custa venda no curto prazo; ganha confiança no longo |
| 4. Inteligência de jogo curada, começando por FiveM | Conhecimento de crash, pool e config do FiveM brasileiro |
| 5. Histórico longitudinal "o que mudou quando piorou" | Precisa de identidade da máquina e changelog confiável desde o começo |

## S. Competitive Testing

Protocolo:

- Mesma máquina, mesma imagem de disco, mesmo driver e build; reinício entre produtos.
- Mesma cena e mesma temperatura inicial; ≥5 rodadas por produto, ordem alternada.
- Métricas: média, mediana, 1% e 0,1% piores, P99 do tempo de quadro, trancos, latência até a tela,
  CPU e RAM do próprio programa, estabilidade em 7 dias.
- Tudo que o concorrente muda no Windows é registrado antes/depois (registro, serviços, tarefas, planos).

Primeiro alvo: Psiu Framer (já inspecionado). Resultado desfavorável vira investigação e issue, não
desculpa. Resultado favorável só vai para o site com o protocolo e os dados publicados.

## T. Roadmap

**0 — antes de tudo:** lançar a 3.2 (teste do dono com FiveM, verificador de publicação, ordem do dono).

| Bucket | Iniciativa | Motivo / pré-requisito |
|---|---|---|
| NOW | TOP 1 Núcleo de Sessão e Evidência | Fundação de todo o resto |
| NOW | TOP 2 Performance Doctor + FiveM Lab | Maior valor percebido, só leitura, risco baixo |
| NOW | TOP 3 Prescrição validada + memória de efetividade | O diferencial central |
| NOW | TOP 4 Identidade da máquina + "O que mudou?" | Peças já existem; junta |
| NOW | TOP 5 Guardião de deriva | Updates do Windows desfazem ajustes em silêncio |
| NOW | TOP 6 Restaurar meu PC + desinstalação limpa | Confiança; risco baixo |
| NEXT | TOP 7 Relatório de sessão | Precisa do TOP 1 e do orçamento medido |
| NEXT | TOP 8 Telemetria opt-in + regras assinadas + canal beta | Precisa de endpoint e política de privacidade |
| NEXT | TOP 9 Relatório de performance compartilhável | Precisa do TOP 3 |
| NEXT | TOP 10 Primeira execução com perfil | Precisa do TOP 4 |
| NEXT | Revisão de segurança formal | Antes de qualquer coisa online |
| NEXT | Testes de caos na CI | Parte já cabe no NOW |
| NEXT | Streaming: proteger OBS no governador, escolha de encoder | Pedido de clientes que transmitem |
| NEXT | Multi-monitor com Hz diferentes | `vrr.rs` já lê cada monitor |
| NEXT | Driver Guardian: aconselha versão, nunca baixa sozinho | Precisa da identidade e do histórico |
| LATER | Conhecimento global anônimo | TOP 8 + volume + consentimento |
| LATER | Coortes percentuais | Telemetria com volume |
| LATER | Painel da empresa e SLOs com número | Baseline da telemetria |
| LATER | Benchmarks da comunidade | Conhecimento global |
| LATER | Modo lan house / várias máquinas | Demanda comercial confirmada |
| LATER | Modo silencioso / FPS por watt | Leitura de potência AMD/Intel |
| LATER | Best Settings além de FiveM/GTA | Banco curado por jogo |
| RESEARCH | ETW de DPC/ISR e troca de contexto | Custo e permissão a medir |
| RESEARCH | Limite de FPS escolhido por A/B com latência | Nunca automático; só como sugestão provada |
| RESEARCH | Separar tranco de shader × streaming automaticamente | Sinal ainda fraco |
| RESEARCH | Fronteira qualidade × desempenho | Exige mexer em config gráfica |
| RESEARCH | Copilot sobre dados estruturados | Só depois do Doctor determinístico |
| REJECT | Launcher próprio | A detecção de jogo já funciona; aumenta o tempo até jogar; conflita com launchers de anticheat |
| REJECT | ML agora | Sem volume de dados; regra com evidência vence e é testável |
| REJECT | "Bata seu recorde" em mundo aberto | Recorde seria sorte de cena |
| REJECT | Controlador que muda gráfico sozinho durante a partida | Troca qualidade sem o cliente escolher; quebra comparabilidade |
| REJECT | Serviço SYSTEM residente | Superfície de ataque por pouco ganho |
| REJECT | Medir artefato de gerador de quadros de terceiros | Sem acesso confiável à imagem sem captura pesada |
| REJECT | Ler PL1/PL2 por driver MSR | Driver de kernel de terceiro: risco de segurança e de anticheat |
| REJECT | Coleta ou coorte sem consentimento | Viola o princípio de nenhuma coleta escondida |
| REJECT | Assinatura recorrente como decisão técnica | O modelo é chave única; mudar é decisão comercial |

## TOP 10

Pontuação de 1 a 5 (risco e dificuldade: 5 = baixo risco / fácil). Total = impacto × 2 + risco +
dificuldade + diferenciação × 2.

| # | Iniciativa | Impacto | Risco | Dificuldade | Diferenciação | Total |
|---|---|---|---|---|---|---|
| 1 | Núcleo de Sessão e Evidência | 5 | 4 | 3 | 4 | 25 |
| 2 | Performance Doctor + FiveM Lab | 5 | 5 | 3 | 5 | 28 |
| 3 | Prescrição validada + memória | 5 | 3 | 2 | 5 | 25 |
| 4 | Identidade + "O que mudou?" | 4 | 5 | 4 | 4 | 25 |
| 5 | Guardião de deriva | 4 | 4 | 3 | 3 | 21 |
| 6 | Restaurar meu PC + desinstalação | 3 | 4 | 4 | 3 | 20 |
| 7 | Relatório de sessão | 3 | 5 | 4 | 3 | 21 |
| 8 | Telemetria opt-in + regras + beta | 4 | 3 | 2 | 3 | 19 |
| 9 | Relatório compartilhável | 3 | 5 | 4 | 4 | 22 |
| 10 | Primeira execução com perfil | 4 | 5 | 4 | 3 | 23 |

A ordem de execução não é a da pontuação: é a das dependências (1 antes de 2 e 3; 4 antes de 10). As
fichas completas, o plano técnico e as tarefas estão em `MASTER-PLAN-TOP10.md`.
