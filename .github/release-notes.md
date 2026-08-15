Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.16.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.16.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# A tela

Esta versão é sobre o que você olha. Fundo novo, cartões com profundidade, e
bastante coisa que estava à mostra sem precisar.

## Um interruptor que estava furado desde sempre

O produto tem um interruptor que desliga toda animação em máquina fraca — um
otimizador que engasga na própria interface se desmente na frente do cliente.

Ele estava escrito assim:

```css
.sem-animacao, .sem-animacao * { animation: none }
```

E o `*` **não alcança `::before` nem `::after`**. Qualquer animação declarada
num pseudo-elemento continuava rodando no PC de 4 GB, com o interruptor ligado.

Como o fundo bonito naturalmente mora num pseudo-elemento, escrever o fundo
antes de achar isso teria posto o programa a gastar GPU exatamente na máquina
que ele foi contratado para aliviar. Corrigido antes do primeiro pixel.

E o multiplicador de movimento (`--anim`), que existia desde a primeira versão
com um comentário dizendo "o JavaScript zera isto", era lido zero vezes e
escrito zero vezes — um token morto fingindo ser um sistema. Agora vale: em
máquina intermediária o fundo desacelera de 90 para 257 segundos por ciclo, em
vez de simplesmente parar.

## O fundo

Malha técnica, manchas de luz à deriva e granulação. Tudo em CSS.

A referência pedida desenha fundos assim com shader em WebGL, a 60 quadros por
segundo, continuamente. Num otimizador de PC isso é o produto gastando placa de
vídeo para se enfeitar — e num notebook, gastando bateria. É o mesmo motivo
pelo qual este projeto recusou uma biblioteca de animação que pesava mais que o
aplicativo inteiro.

O que anima aqui é só `transform`, que o navegador resolve sem repintar nada.
Foram recusados por medição de custo o desfoque animado (força repintura de
tela cheia, e gradiente radial já nasce difuso) e o gradiente cônico girando
(regera a imagem a cada quadro).

## Menos coisa gritando por atenção

**Dezenove botões usavam o destaque mais forte da tela** — o mesmo de "Otimizar
agora" — para ações que **não mudam nada** na sua máquina. Sobraram cinco, e
todos alteram alguma coisa. Medir virou botão comum.

**Seis verbos viraram um.** Analisar, Verificar, Ler, Medir, Procurar e Mapear
faziam a mesma coisa em painéis diferentes. Agora é "Analisar", com o tempo
estimado no próprio botão.

**Dezoito caixas de resultado ficavam completamente vazias** — sem texto, sem
contorno — até alguém clicar. Você precisava clicar para descobrir o que o
botão fazia. Agora cada uma diz o que o exame lê, quanto demora e **se altera
alguma coisa**. Esse terceiro item é o argumento do produto, e até agora não era
dito em lugar nenhum antes do clique.

**A aba Diagnóstico encolheu 44%.** Ela tinha nove cartões, e quatro deles
remediam o que o programa já mede sozinho na abertura. Continuam inteiros,
recolhidos atrás de "ver cada exame em detalhe".

## O programa abriu mais rápido, e ainda não o bastante

O diagnóstico inicial caiu de 47 para 31 segundos nesta máquina de teste.

Duas causas foram consertadas: o exame de prontidão abria dois `powershell.exe`
para responder uma pergunta só, e a lista de planos de energia era consultada
três vezes por análise — três processos para a mesma resposta.

**Trinta e um segundos ainda é muito**, e vale dizer em vez de esconder. O que
sobra é a arquitetura: cada módulo de diagnóstico abre o seu próprio processo
do PowerShell, e cada um custa memória na máquina que estamos justamente
medindo por falta dela. Consertar isso de verdade é trocar essas chamadas por
leitura direta dos contadores do Windows, e é a próxima versão.

---

## O que esta versão não promete

A tela ficou melhor; a tela não cria FPS. O que move o número continua sendo a
configuração do próprio jogo e o hardware — e o programa continua dizendo isso
na primeira coisa que você lê.
