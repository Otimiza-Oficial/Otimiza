Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.17.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.17.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# O diagnóstico ficou 5 vezes mais rápido

A versão passada abria em 31 segundos e as notas diziam, com todas as letras,
que aquilo ainda era muito. **Agora são 6.**

## O que estava acontecendo

A suspeita era que as consultas ao Windows fossem lentas. Medimos, e não eram.

Abrir um `powershell.exe` **vazio** — um processo que só executa `1` e termina —
custava **2,26 segundos** nesta máquina de teste. E o programa abria dez deles
para montar o diagnóstico inicial.

Vinte e dois dos trinta e um segundos eram o Windows subindo o PowerShell, dez
vezes seguidas. A consulta em si era praticamente de graça: os módulos que
faziam uma única chamada custavam exatamente os 2,26 segundos do processo, nem
um décimo a mais.

## O conserto

Em vez de um processo por consulta, o programa mantém **um** vivo e conversa com
ele. O custo é pago uma vez.

| | Antes | Agora |
|---|---|---|
| Saúde do disco e bateria | 11,5 s | 0,59 s |
| Memória e paginação | 5,4 s | 0,23 s |
| Firmware e memória instalada | 4,7 s | 0,06 s |
| Registro de eventos | 3,8 s | 0,50 s |
| Monitor | 2,3 s | 0,15 s |
| Placa de vídeo por jogo | 2,3 s | 0,04 s |
| **Diagnóstico completo** | **31 s** | **6 s** |

Nenhum módulo precisou ser reescrito para isso. A mudança vive num arquivo só,
e quem chama nem sabe que a sessão existe.

## Uma armadilha que o conserto criou, e que foi trancada

A codificação de texto tem dois lados, e a sessão só resolvia um.

A **saída** vem certa. A **entrada** não: o PowerShell lê o que recebe usando a
página de código do console, e não há como corrigir isso de dentro — quando a
primeira linha chega, ele já leu com a página errada. Um script contendo
"Ação" chegava lá dentro como "A├º├úo", e o estrago acontecia antes de o script
rodar.

O pior dessa falha é que ela **não quebra**: devolve um resultado errado com
cara de certo.

Qualquer script com acento passa agora pelo caminho antigo, mais lento e
correto. Custa a lentidão de um processo nos poucos casos em que isso acontece,
e elimina a classe inteira de corrupção por construção — não por auditoria dos
scripts de hoje, que poderia envelhecer.

Três testes novos trancam isso: que script com acento não usa a sessão, que a
sessão devolve exatamente o mesmo que o processo avulso, e que um script que
quebra continua sendo reportado como falha — porque a diferença entre "não há
dado" e "a consulta falhou" é a distinção em que este produto inteiro se apoia.

---

## O que esta versão não promete

Velocidade de abertura não é FPS. O que move o número no jogo continua sendo a
configuração dele e o hardware — e o programa continua dizendo isso na primeira
coisa que você lê, agora cinco vezes mais rápido.
