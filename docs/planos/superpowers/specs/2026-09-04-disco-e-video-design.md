# O disco inteiro, e a placa de vídeo — desenho

Data: 2026-09-04

## Por que isto existe

O pedido foi "mais otimizações, do tipo que os outros fazem por CMD, para o PC
do cliente ficar limpo". A medição na máquina do dono mudou o desenho.

### O que a medição disse, e ela contradiz o pedido original

Disco C: **8,7 GB livres de 476 GB — 1,8%**. O próprio `diskspace.rs` explica
por que isso importa: abaixo de 10% o Windows perde folga para gerenciar o
arquivo de paginação, o Explorer engasga e as atualizações falham.

Onde os 467 GB estão:

| Pasta | Tamanho |
|---|---|
| `~\AppData` | 176,2 GB |
| `Program Files (x86)\Steam` | 122,3 GB |
| `~\Downloads` | 48,4 GB |
| `C:\Windows` | 29,5 GB |
| `~\Documents` | 11,0 GB |

E o que o produto sabe limpar hoje — temporários, cache de update, relatórios de
erro, Delivery Optimization, logs, lixeira — soma **poucos GB. Menos de 1% do
problema.**

**Acrescentar a categoria de limpeza número 7 não move nada.** O cliente
clicaria em "limpar", veria 2 GB liberados num disco com 467 GB ocupados, e
concluiria — com razão — que o produto não fez nada.

### O furo real

`foldermap.rs` existe, funciona e já está na tela. Mas ele varre
`perfil_do_usuario()` — só a pasta do usuário.

**Os 122 GB de Steam ficam em `Program Files (x86)`, fora do alcance dele.** O
mapa que existe para responder "cadê meu disco?" está cego para a segunda maior
coisa da máquina, num produto cujo público inteiro é jogador.

### E o segundo eixo

O dono trouxe uma lista de ajustes de hardware (PBO, overclock de GPU,
XMP/EXPO, Resizable BAR). Confrontada com o produto:

- **XMP/EXPO** — já detectado (`firmware.rs`, achado `memory_xmp_off`)
- **Ajustes de Windows e driver** — 42 itens no catálogo, saturado
- **Resizable BAR** — **não detectado. É a lacuna real da lista.**
- **PBO e overclock de GPU** — fora, por decisão registrada abaixo

## Regra que manda neste documento

A regra fundadora do produto, aplicada aqui:

- Nunca mostrar número que não foi medido.
- Admitir quando o ganho é zero.
- "Não consegui verificar" nunca vira "está tudo bem".
- Toda mudança reversível, com o valor anterior registrado.

E a decisão do dono, tomada para este trabalho: **apagar só o que o Windows
recria sozinho.** Ponto de restauração, hibernação e arquivo de usuário ficam
fora. Apagar arquivo é a única coisa que este produto não desfaz.

---

# Pilar 1 — o mapa enxerga o disco inteiro

## O que muda em `foldermap.rs`

Ele já resolve as três partes difíceis, e **nada disso é reescrito**:

1. junction e link simbólico — seguir conta o mesmo arquivo duas vezes e, no
   pior caso, entra em laço infinito;
2. pasta sem permissão — metade de `C:\Windows` e todo perfil de outro usuário;
3. prazo de varredura, com aviso honesto quando estoura, em vez de devolver um
   número menor fingindo ser o total.

O que muda é **para onde ele aponta** e **como apresenta**.

### Duas camadas, porque varrer 476 GB leva minutos

**Camada 1 — as raízes.** `C:\` no nível 1. Segundos. É o que responde a
pergunta imediata: `AppData 176 GB · Steam 122 GB · Downloads 48 GB`.

**Camada 2 — o detalhe.** O cliente clica numa raiz e só então aquela é varrida
a fundo. Ninguém espera olhando tela parada, e o prazo que o módulo já tem
continua valendo.

### Separar o que dá para apagar do que não dá

O mapa vai mostrar `Steam — 122 GB`. **O produto não apaga isso.** Apagar jogo
de cliente é o pior erro possível deste programa, e não existe desfazer.

Cada linha do mapa carrega um estado tipado — nunca uma frase que a tela
interpreta:

- `PodeLimpar` — é uma categoria conhecida do `diskspace.rs`, com botão
- `Seu` — é arquivo do cliente. Mostra o caminho exato e mais nada
- `NaoSei` — não deu para ler (permissão, prazo). **Nunca contado como zero**

O terceiro não é detalhe: uma pasta que não deu para ler aparecendo como 0 GB
faria o total mentir, e o cliente concluiria que o espaço sumiu no nada.

## O que entra em `diskspace.rs`

As categorias que são grandes de verdade, todas recriadas pelo Windows:

| Categoria | Como | Observação |
|---|---|---|
| WinSxS | `DISM /Online /Cleanup-Image /StartComponentCleanup` | ver a ressalva abaixo |
| Cache de disco dos navegadores | apagar conteúdo | Chrome, Edge, Firefox |
| Despejos de memória | `MEMORY.DMP` e `Minidump\` | pode ter o tamanho da RAM |
| Cache da Microsoft Store | `wsreset` | |
| Miniaturas e ícones | apagar conteúdo | o Explorer refaz |

**O WINSXS MENTE SOBRE O TAMANHO, E O PRODUTO NÃO PODE REPETIR A MENTIRA.**

A pasta aparece com 11,5 GB na máquina do dono, mas usa hard links: boa parte
daquilo são os mesmos arquivos de `C:\Windows` contados de novo. O que o DISM
libera de verdade costuma ser 1 a 5 GB.

O produto mostra **o que o próprio DISM estima**
(`/AnalyzeComponentStore`), nunca o tamanho da pasta. Se a análise não rodar,
a categoria aparece como "não consegui estimar" — e não com o número errado.

## O que NÃO entra

- **Prefetch, "limpeza de registro", "liberador de RAM".** Decisão do agente,
  com a regra fundadora como base: são placebo. O `Prefetch` limpo deixa o boot
  MAIS LENTO até o Windows reconstruir. O produto ganha uma linha dizendo que
  não faz e por quê — é a única coisa que ele tem e o concorrente não.
- **Pontos de restauração, hibernação, arquivos do usuário.** Decisão do dono.

---

# Pilar 2 — placa de vídeo e Resizable BAR

## Parte A — Resizable BAR, só leitura

**Como detectar, verificado na máquina do dono:** `nvidia-smi -q` devolve
`BAR1 Memory Usage / Total`. Compara com a VRAM. O `nvidia-smi` acompanha o
driver, então está em toda máquina com placa NVIDIA — mesma escolha do
`nvapi64.dll` abaixo, e do Edge no relatório em PDF.

Medido: `GTX 1650, 4096 MiB` de VRAM contra `BAR1 Total: 256 MiB`. Desligado.

**DUAS PERGUNTAS, E NÃO UMA.** É o que a medição na máquina do dono ensinou:

| Estado | O que a tela diz |
|---|---|
| Ligado | "Está ligado. Nada a fazer." |
| Desligado, e a placa **suporta** | "Ative na BIOS: procure *Resizable BAR* ou *Above 4G Decoding*." |
| Desligado, e a placa **não suporta** | "Sua placa não tem esse recurso. Não há nada para ativar." |
| Não deu para ler | "Não consegui verificar." Nunca "está tudo bem." |

Sem a terceira linha o produto mandaria o dono — cuja GTX 1650 **não suporta
rBAR**, porque a NVIDIA só habilitou da RTX 3000 para cima — entrar na BIOS
procurar uma opção que não existe para ele. Detectar sem entender é pior do que
não detectar.

## Parte B — ajustes de driver pela NVAPI

Escrita, e ela existe porque o veredito da investigação anterior
(`2026-09-03-otimiza-1-5-design.md`) já a aprovou, com o motivo que decide:

> A NVAPI tem um subsistema de configuração (DRS) feito exatamente para isto,
> oficial e documentado, e — o que decide o pilar para este produto — **com
> chamada para RESTAURAR O PADRÃO de uma opção**. Reversível de verdade, não
> "reversível se a gente anotar direitinho".

A `nvapi64.dll` vem com o driver: carregada em tempo de execução, sem
acrescentar dependência ao instalador.

Opções que mexem em FPS e têm padrão restaurável: gerenciamento de energia,
*low latency mode*, filtragem de textura, VSync e cache de shader.

Cada uma entra no histórico como as outras 42 do catálogo — com o valor
anterior — e o "Desfazer" já existente chama a restauração de padrão da NVAPI.

## AMD fica de fora, e a tela DIZ isso

Recomendação literal do veredito: *"entrar com a NVIDIA e DIZER NA TELA que a
AMD ainda não é coberta, em vez de fazer meia coisa nas duas."*

Cliente com AMD lê "ainda não cobrimos sua placa" em vez de ver tela vazia e
achar que o programa quebrou.

## O que NÃO entra, e por quê

**Overclock e undervolt, de CPU e de GPU.** Estavam na lista que o dono
trouxe. Três motivos, e o terceiro sozinho basta:

1. Não há API pública. O MSI Afterburner usa interface não documentada da
   NVIDIA, que quebra a cada driver novo.
2. Overclock precisa de teste de estabilidade POR MÁQUINA. O que roda liso numa
   trava em outra.
3. Se o PC do cliente ficar instável depois deste programa mexer no clock, a
   culpa é do programa — e não há como provar o contrário. O argumento de venda
   é "toda mudança é reversível com o valor anterior registrado". Overclock
   quebra isso.

**Escrever na BIOS.** O `firmware.rs` já explica: em placa de consumo as
configurações ficam num bloco proprietário da NVRAM, com checksum próprio de
cada fabricante e sem API pública. Escrever no lugar errado **inutiliza a
placa-mãe**. XMP, EXPO e o próprio rBAR continuam sendo "o produto detecta e
ensina onde ativar".

---

# Testes

Além do que cada tarefa trouxer:

- **Nenhuma pasta ilegível vira zero.** Uma raiz sem permissão aparece como
  `NaoSei`, e o total diz que está incompleto.
- **O mapa alcança fora do perfil do usuário.** Um teste que falha se a
  varredura voltar a olhar só `perfil_do_usuario()`.
- **O produto nunca apaga arquivo marcado como `Seu`.** Canário: nenhum caminho
  de código leva de `Seu` a uma remoção.
- **WinSxS não usa o tamanho da pasta.** Se a análise do DISM não rodar, a
  categoria diz que não estimou — e não mostra 11,5 GB.
- **rBAR desligado em placa sem suporte não vira conselho de BIOS.** É o
  defeito que a máquina do dono revelou, e ele precisa de teste próprio.
- **Máquina AMD não mostra tela vazia.** Diz que não é coberta.
- **Todo ajuste de driver entra no histórico com o valor anterior**, e o
  "Desfazer tudo" o restaura.
- Módulo novo entra em `.github/workflows/release.yml`, senão a guarda
  `ci_coverage` reprova.
- As notas de versão precisam citar o número da versão, senão a trava recusa
  publicar. E os quatro arquivos de versão precisam concordar entre si.

# O que este documento não promete

O mapa **não libera espaço sozinho** — na máquina que motivou este desenho, 90%
do disco é jogo e download do próprio dono, e nada disso é do produto para
apagar. O que ele entrega é a resposta que ninguém dá: para onde o disco foi, e
qual parte dá para recuperar com segurança.

E o Resizable BAR **não vai ajudar em placa que não suporta** — que é o caso da
máquina onde ele foi verificado. O produto vai dizer isso, em vez de mandar o
cliente procurar na BIOS.
