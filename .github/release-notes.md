Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.14.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.14.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# O Otimiza deixou de ser um otimizador de FiveM

A versão anterior reconhecia cinco jogos, e três eram da família GTA. Fortnite,
Minecraft, LoL, Roblox, qualquer lançamento novo — nenhum existia para o
programa. O modo jogo nunca ligava, o segundo plano nunca era pausado.

Esta versão troca a lista por medição.

## Reconhece qualquer jogo, mesmo o que ninguém cadastrou

Um programa é jogo quando três coisas valem ao mesmo tempo: a janela em
primeiro plano cobre o monitor, **o mesmo processo** está consumindo o motor 3D
da placa, e está aberto há tempo suficiente.

Os três juntos, não pontuação somada. É o par janela+3D que carrega o peso, e
ele resolve de uma vez o caso que derruba todo detector ingênuo: num navegador,
quem desenha é um processo auxiliar que **não tem janela**, e o processo que tem
a janela não consome 3D. Isso cobre Chrome, Discord, Spotify e Teams sem
precisar citar nenhum deles. Vídeo em tela cheia também não passa, porque
consome o motor de decodificação e não o 3D.

O OBS é o único caso que a medição não separa — usa 3D pesado em tela cheia
igual a um jogo. Esse entra numa lista de recusa, com o motivo escrito. E
recusar é o certo: ligar o modo jogo porque alguém começou a gravar seria mexer
na máquina pelo motivo errado.

A lista de nomes continua, com outro papel: dar nome bonito. Passou de 5 para 15
jogos. Um jogo fora dela é reconhecido do mesmo jeito, só aparece pelo nome do
executável.

## Seu monitor pode estar rodando a um terço do que aguenta

Monitor de 144 ou 180 Hz configurado em 60 Hz é comum — acontece depois de troca
de driver ou de cabo, e ninguém percebe. É a maior diferença de fluidez que
existe num PC.

Na máquina onde esta versão foi desenvolvida, **dois monitores de 180 Hz estavam
os dois em 60 Hz**, e o dono vinha reclamando que o jogo não parecia fluido.

O Otimiza passa a ler isso e avisar. E não promete o que não entrega: subir a
taxa do monitor **não aumenta o FPS**. Ela não cria quadros — deixa de segurar
os que a placa já entrega. O jogo fica mais suave e o contador continua onde
estava. Está escrito assim na tela, e há um teste que quebra se alguém mudar
esse texto para prometer FPS.

## Jogo rodando na placa de vídeo errada

Em notebook com duas placas, o Windows às vezes abre o jogo na placa integrada.
O jogo abre normalmente e só roda mal, e o dono conclui que o PC é fraco —
quando muitas vezes a placa boa está parada do lado. Quando esse é o caso,
corrigir vale de duas a cinco vezes mais FPS.

Em máquina com uma placa só, o ganho é exatamente zero, e o Otimiza não mostra
nada. Também há teste para isso.

## Anticheat: onde o produto tira a mão

Esta é a mudança da qual mais me orgulho, e ela **reduz** o que o programa faz.

O Otimiza suspende programas de segundo plano durante o jogo e escreve numa
chave do registro que fixa prioridade. Enquanto a lista tinha cinco jogos e três
eram GTA, isso quase nunca encostava num anticheat. Com Valorant, Fortnite e
PUBG reconhecidos, encosta.

Agora o programa detecta Vanguard, Easy Anti-Cheat, BattlEye, VAC e FACEIT — e
quando encontra um deles ativo, **se recusa a trabalhar**, dizendo por quê:

> Não pausei nenhum programa: o Riot Vanguard está rodando agora. Pausar
> programas com um anticheat de núcleo ativo é risco de banimento, e nenhum
> ganho de FPS compensa perder a sua conta.

Um detalhe que exigiu cuidado: o Vanguard sobe junto com o Windows e fica
vigiando com o Valorant fechado. Olhar só os programas abertos daria "nenhum
anticheat" numa máquina onde a Riot está observando desde que o PC ligou.

Também saiu de circulação a suspensão da Steam e do lançador da Epic enquanto há
partida: o anticheat conversa com eles durante o jogo.

## Dois defeitos que a versão anterior tinha

**A prioridade só funcionava no FiveM.** O modo jogo detectava Counter-Strike,
pedia a prioridade a uma função que procurava o processo por um filtro literal
`FiveM*GTAProcess*`, e ela respondia *"o jogo não está aberto"* — com o jogo
aberto na frente do cliente.

**A detecção casava com o que não devia.** A comparação era por pedaço do nome,
então a chave `cs2` reconhecia `docs2pdf.exe` como Counter-Strike e mudava o
plano de energia da máquina. Agora é por nome completo, com exceção declarada e
testada só para os jogos cujo executável carrega número de compilação.

## A trava do registro mudou de base

A chave que fixa prioridade de processo é a mesma usada por programas que
sequestram a execução de outros. Quem autorizava a escrita era a lista de nomes
de jogo — e com a detecção genérica ela deixou de servir.

A nova trava não é o detector: heurística não pode virar autoridade de
segurança. É o **caminho**. O executável precisa estar dentro de uma biblioteca
de jogo declarada pela Steam ou pela Epic. `cmd.exe` e `sethc.exe` nunca vão
estar.

## Limpeza na interface

Saíram 45 linhas de estilo que mantinham viva a nota de saúde de 0 a 100
removida na versão passada, e 23 cores que eram reescritas à mão recriando
cores que já existiam como token — trocar o vermelho da marca exigia caçar
todas. Zero classes sem uso no arquivo.

---

## O que esta versão não promete

Nada aqui inventa FPS onde falta hardware. Numa máquina de 8 GB em canal único
o teto continua sendo o teto, e o produto continua dizendo isso na primeira
tela — antes de qualquer botão.
