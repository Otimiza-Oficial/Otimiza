# Otimiza 1.5 — o produto passa a medir o que ninguém mostra

**Data:** 03/09/2026
**Estado:** rascunho para aprovação do dono
**Recorte:** versão grande, de capacidade nova. A 1.3 foi de conserto; esta é
de alcance.

---

## De onde esta versão vem

A pesquisa que precedeu a 1.0 levantou cinco lacunas reais. Três viraram a 1.0 e
a 1.1 (reparo do Windows, limpeza do WinSxS, configuração do jogo). **Duas
ficaram**, e uma terceira apareceu depois:

| | Situação hoje |
|---|---|
| Perda de pacote até o servidor do jogo | `network.rs` mede DNS e **nunca** mede perda. Verificado. |
| Configurações do driver de vídeo | Não existe nada. `gpupref.rs` escolhe *qual placa*, não *como ela se comporta*. |
| Qual driver está travando o sistema | Nenhuma ocorrência de DPC no projeto. |

Nenhuma das três existe. Conferido no código antes de escrever — este plano já
propôs duas vezes coisa que estava pronta, e a conferência custa um comando.

---

## Pilar 1 — A perda de pacote, que é a causa real do teleporte

**É o item que mais casa com este produto, e o mais barato dos três.**

O `network.rs` abre dizendo, com todas as letras, que reduzir ping é mentira:
ping é distância física mais roteamento, e nenhum ajuste no PC do cliente muda
isso. Essa recusa está certa e fica.

Mas há um número que **ninguém no mercado mostra** e que é a causa real da
queixa do jogador de FiveM: **perda de pacote**. Quando o personagem teleporta,
quando o carro anda sozinho, quando o tiro não registra — isso quase nunca é
FPS, e quase nunca é ping. É pacote que não chegou.

E é medível, com precisão, sem prometer nada.

### O que ele faz

Mede, contra o servidor em que o cliente está jogando: perda de pacote, a
variação do tempo de resposta (jitter de rede, que é diferente do jitter de
quadros que o produto já mede), e o tempo de resposta em si.

### O que ele NÃO faz, e vai escrito na tela

**Não promete melhorar nada.** Perda de pacote quase sempre é do provedor, do
cabo, do Wi-Fi ou do servidor — não do PC. O produto mede e diz onde está,
porque saber que o problema é o Wi-Fi economiza ao cliente uma tarde
reinstalando driver e uma otimização paga que não ia adiantar.

Isto é a mesma promessa da aba de reparo, invertida: lá o produto conserta o que
ninguém conserta; aqui ele mede o que ninguém mede, e admite que não conserta.

### A parte difícil, e ela é real

Descobrir contra QUEM medir. O FiveM se conecta a um servidor cujo endereço não
está numa configuração fixa. Precisa sair das conexões ativas do processo do
jogo. **Se não der para descobrir com segurança, o produto diz que não
descobriu** — não mede contra um alvo qualquer e apresenta como se fosse o
servidor do jogo.

---

## Pilar 2 — As configurações do driver de vídeo

**É o que o concorrente vende, e é o único dos três que mexe em algo.**

O painel da NVIDIA e o da AMD têm ajustes que mudam latência e estabilidade de
quadro. Hoje o Otimiza não toca em nenhum.

### A moldura honesta, e ela decide o pilar inteiro

"Prefer Maximum Performance" **não dá FPS quando a placa já está saturada.** Ele
ajuda quando o jogo é limitado pelo processador e a placa baixa o clock sozinha.
É ganho de latência e de estabilidade, não de média.

Então este pilar entra medindo: aplica, mede antes e depois com o `prova.rs` que
já existe, e mostra o resultado como ele for — inclusive zero. Sem isso ele vira
o "+300 FPS" do concorrente com outro nome.

### Reversibilidade

Toda mudança entra no histórico com o valor anterior, como qualquer outra. Isso
é inegociável e já é a regra da casa.

### O risco a decidir antes de começar

Mexer no driver de vídeo por caminho não documentado é diferente de mexer no
registro do Windows. Precisa de investigação própria: qual é a superfície
suportada, o que é gravável com segurança, e o que acontece quando o driver
atualiza por cima. **Se a resposta for "não há caminho seguro", este pilar não
entra** — e isso é um resultado legítimo, não um fracasso.

---

## Pilar 3 — Qual driver está travando o sistema

**O mais difícil e o mais valioso quando acerta.**

O produto já mede engasgo (`jitter.rs`) e sabe dizer que travou. Não sabe dizer
**quem** travou. Um driver com latência alta de DPC congela o áudio, engasga o
mouse e produz travadas que nenhum ajuste resolve — e o cliente reinstala
Windows sem nunca descobrir que era a placa de rede.

### Por que fica por último

Depende de instrumentação de eventos do Windows que o projeto ainda não usa, e o
risco de apontar o driver errado é alto. **Acusar o driver errado é pior que não
acusar nada:** o cliente desinstala algo que funcionava.

Regra: só nomeia um driver quando a medição sustentar. Caso contrário diz que
achou latência alta sem conseguir atribuir — que já é mais do que ele sabia.

---

## O que esta versão NÃO promete

**Não promete FPS.** Nenhum dos três pilares é um ajuste que aumenta quadro. O
primeiro mede rede, o segundo troca latência por energia, o terceiro nomeia
culpado. O produto continua sem número mágico.

**Não promete os três.** O Pilar 2 depende de existir caminho seguro no driver,
e o Pilar 3 de a medição sustentar a acusação. Se um cair na investigação, cai —
e a versão sai com o que ficou de pé, dizendo o que não deu.

---

## Ordem

1. **Perda de pacote** — o mais barato, o mais alinhado, e o que serve ao público que já compra
2. **Driver de vídeo** — o que o concorrente vende, com a moldura que ele não tem
3. **Qual driver trava** — investigação primeiro, decisão depois

## Verificação

- `cargo test --lib` — 495 hoje. Cada pilar entra com teste.
- **Na máquina do dono, com o FiveM aberto:** medir a perda contra o servidor
  real, e conferir se o número bate com o que ele sente jogando. É o teste que
  diz se o Pilar 1 vale.
- Módulo novo entra na lane de `.github/workflows/release.yml`.
- As notas precisam citar `1.5.0`.
- A trava da prosa continua valendo: nenhuma decisão de interface sai de comparar
  texto do backend.

## Pendência que atravessa da 1.3

A causa do congelamento relatado por um cliente continua desconhecida. Falta o
resultado do `descongelar.ps1` na máquina dele. Se chegar, vira tarefa; se não,
não entra e a versão não afirma tê-lo consertado.
