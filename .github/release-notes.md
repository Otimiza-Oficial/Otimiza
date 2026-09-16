Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_2.7.0_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_2.7.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão.

---

# A 2.7 traz a geração de quadros do próprio Otimiza

## Otimiza Frame Gen (experimental)

Na aba **Geração de quadros**, o painel **Gerador do Otimiza** liga uma geração
de quadros feita pelo próprio Otimiza — sem instalar nenhum outro programa.

Ela funciona por fora do jogo, como o Lossless Scaling: captura a janela,
estima o movimento na placa de vídeo e mostra os quadros intermediários numa
camada por cima. **Nada é injetado no processo do jogo.**

O que ela faz, dito com clareza: **aumenta os quadros que chegam à tela** (2×,
3× ou 4×). O jogo continua desenhando os mesmos quadros — o contador de FPS do
próprio jogo não muda — e o controle fica um pouco mais atrasado.

Medido no FiveM, em 1080p, numa GeForce GTX 1650 dividida com o jogo:

- a tela recebeu cerca do dobro de quadros com 2× (por exemplo, 48 → 96);
- custo de cerca de 2 ms por quadro na placa de vídeo;
- **atraso acrescentado medido: 7 a 14 ms**.

Proteções que vêm ligadas:

- **Nunca tira FPS do jogo.** De tempos em tempos a geração pausa por um
  instante e o FPS real é comparado com e sem ela. Se o jogo perder mais de
  4%, o gerador se desliga sozinho e diz por quê.
- **Não inventa imagem quando não dá.** Em giro de câmera muito rápido, troca
  de cena ou câmera atravessando um carro, o quadro gerado é recusado e a tela
  mostra só o real — fica igual a jogar sem gerador, nunca pior.
- **Nunca passa da taxa do monitor.** Se o jogo já entrega o que o monitor
  mostra, não há geração.
- **Interface parada sai do quadro real**, sem ser deslocada.
- **Ctrl+Alt+G** desliga de qualquer lugar, inclusive de dentro do jogo.

Exige o jogo em **janela sem bordas** — tela cheia exclusiva não pode ser
capturada.

## Plano de energia: nunca menos FPS

- **O plano que você já usa é o piso.** Um candidato com menos FPS ou pior 1%
  low que o seu plano atual nunca é recomendado; se nenhum ganha dele, ele fica.
- **Autoajuste em duas etapas.** Depois da primeira bateria, o vencedor ganha
  vizinhos (preferência de energia um pouco acima e abaixo, estacionamento de
  núcleos trocado), medidos na mesma cena.
- **Candidato "resposta máxima"** — tudo no máximo, o estilo de planos de
  concorrentes — medido lado a lado em desktop. Se ele for o melhor no seu PC,
  é o escolhido.
- **Teste de quadros sem abrir o jogo**, para autoajustar mesmo com ele fechado.
- **Teste interrompido não deixa a máquina presa.** Se o Otimiza fechar no meio
  do autoajuste, a abertura seguinte volta ao plano anterior.

---

## O que esta versão não promete

**Geração de quadros não é FPS a mais.** Ela deixa a imagem mais lisa. Para o
jogo desenhar mais quadros, o caminho continua sendo a configuração do jogo, o
driver e o gargalo do processador — as abas Jogos e Energia.

**O gerador é experimental.** Ainda aparecem defeitos em interface
semitransparente sobre fundo em movimento, em minimapa girando e em carro
passando muito rápido perto da câmera. E os números acima foram medidos numa
máquina só: em outra placa, jogo e resolução, eles mudam — o laboratório da aba
mede antes e depois na sua.

**O "editor desconhecido" continua aparecendo.** O aviso do SmartScreen é o
Windows dizendo, com razão, que não sabe quem publicou este instalador.
