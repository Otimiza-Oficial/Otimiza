---
name: ler-log-do-cliente
description: Use quando um cliente do Otimiza reclamar de FPS, travada, "o frame boost parou", "piorou" ou mandar o otimiza.log. Passo a passo para ler o log, montar a linha do tempo e achar a causa com as linhas que provam, antes de qualquer hipótese.
---

# Ler o log do cliente

**Regra:** a causa sai do log, não da última coisa que mudamos nem do último
caso que vimos. Não presuma que é uma assinatura conhecida: confirme pelas
linhas.

## Onde está e como chega

`%APPDATA%\pc-optimizer\otimiza.log` (o anterior, depois de 2 MB, é
`otimiza.log.1`). Desde a 1.0.5 o cliente usa **Ajustes → Registro do
Otimiza → Salvar em Downloads** (`Otimiza-log-<data>.txt`, os dois arquivos
juntos, sem o nome de usuário). Antes disso: `%APPDATA%\pc-optimizer` no
Win+R.

Versões: interna `4.0.X` = a que o cliente vê `1.0.X` (4.0.0 = "1.0").

## Passos (L = caminho do log)

1. **Tamanho, versões e a janela da reclamação:**
   `wc -l L; grep -n "Versão do Otimiza\|PC Performance Optimizer iniciado" L | tail -30`
   Pergunte (ou estime) **quando** foi o problema. O log pode ir além do
   horário da reclamação, e o cliente pode ter trocado de versão no meio
   (inclusive voltado para uma anterior).
2. **Máquina — o relatório da sessão da reclamação, não o primeiro:**
   `grep -n "RELATÓRIO DO SISTEMA" L | tail -5` e leia ~35 linhas a partir do
   último antes da reclamação (`sed -n 'N,+35p' L`). Os **Hz do monitor** não
   estão no relatório: `grep -n "Hz" L | grep -v "GHz" | tail -5` (aparecem em
   `turbo: ... Some(144) Hz` e nas linhas do gerador).
   Administrador por sessão: `grep -n "Administrador" L | tail -10` — o app
   costuma abrir sem e reabrir com; vale o da sessão do jogo.
3. **Eventos que mudam o FPS** (sem cortar a linha; filtre o ruído):
   ```
   grep -nE "Versão do Otimiza|jogo aberto|jogo fechou|medição automática|prova do Turbo|limite NVIDIA|jogo liso|desfazendo|restaurar|turbo:|gerador|guarda|modo leve|FPS base|energia de jogo|\[WARN\]|\[ERROR\]" L \
     | grep -v "Partida de comparação" | grep -v "fechando: gerador de quadros desligado"
   ```
   Ruído conhecido (não é evento): `Modo jogo: Partida de comparação...`
   (governador, centenas de linhas) e `fechando: gerador de quadros desligado`
   (toda saída do app; **não** significa que o Frame Boost rodou).
4. **O que NÃO aparece também é dado:** sem `gerador:` (com dois pontos) nem
   `guarda` = Frame Boost não rodou; sem `limite NVIDIA` = nenhum teto gravado.
5. **Compare FPS só em faixas.** `medição automática` é de 20 s num lugar
   qualquer do jogo; uma contra outra não prova nada. Compare a faixa de
   várias antes × várias depois, no mesmo executável (o FiveM troca de build:
   `b3258` × `b3889` não se compara).
6. **Gatilho:** o log não diz se um teto do Jogo liso foi clicado pela pessoa
   ou gravado depois da partida. `jogo liso: teto` = botão "Travar";
   `jogo liso depois da partida` = cartão "Deixar liso". Se não der para
   saber, diga.
7. **Confirme no código** que a correção da assinatura existe na versão atual
   (coluna "Confirmar" abaixo) antes de dizer "já corrigido".

## Assinaturas conhecidas

| Linha no log | O que significa | Confirmar no código |
|---|---|---|
| `limite NVIDIA \`X\`: N FPS` + `jogo liso: teto de N`, N ≈ Hz/2, faixa de FPS bem acima | Teto do Jogo liso cortando o jogo (até a 1.0.3) | `git grep -n PERDA_MAXIMA pc-optimizer/src-tauri/src/plataforma/jogo_liso.rs` |
| `frame boost: FPS base de X travado em N` e depois FPS preso em ~N sem linhas `gerador:` | Teto do Frame Boost ficou no driver depois que ele desligou | `plataforma/teto_do_jogo.rs` (1.0.5) |
| `o Windows recusou \`powercfg -setacvalueindex <plano> ...\`: ... does not exist` em toda abertura | Plano de energia apagado, registro de devolução preso; a prova do Turbo aborta na rodada 3 | `git grep -n "não existe mais" pc-optimizer/src-tauri/src/plataforma/energia_jogo.rs` |
| `turbo: a prova automática não terminou: ...` | Prova do Turbo abortada; o motivo vem depois dos dois pontos | `plataforma/turbo.rs` |
| `gerador: o jogo perdia FPS (razão R); modo leve` | Guarda viu perda e entrou no modo leve | `geracao/mod.rs` |
| `gerador desligado pela guarda: razão R` | Frame Boost desligou: o jogo mantinha só R do FPS real | `geracao/guarda.rs` |
| sequência longa de `desfazendo: ...` no mesmo segundo | "Restaurar tudo" (pelo app ou pela desinstalação) | `modules/windows/restaurar.rs` |
| `exige executar o programa como administrador` | Sessão sem administrador: vários ajustes não aplicaram | — |

Assinatura nova achada: acrescente nesta tabela, com o comando de confirmar.

## Entrega

Siga o formato do agente `analista-de-log`: máquina, linha do tempo, causa com
linhas citadas, se é defeito do Otimiza (e se a correção já existe, conferida),
o que o cliente faz agora, o que falta saber. Não prometa conserto sem dizer
o que ainda não foi testado no jogo.
