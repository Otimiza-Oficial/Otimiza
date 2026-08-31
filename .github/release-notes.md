Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_..._x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_..._x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# 1.0.0 — o Otimiza passa a mexer no jogo

Até aqui o produto mexia no Windows. A partir desta versão ele mexe também na
configuração do jogo — e é lá que mora a maior parte do FPS que você está
deixando na mesa.

A ordem de grandeza, medida:

```
uma configuração de jogo mal escolhida ... dezenas de por cento
memória insuficiente ..................... o teto da máquina
ajustes de Windows, todos somados ........ alguns por cento
```

Os 42 ajustes de Windows continuam aqui, e continuam valendo o que valem. O que
mudou é que agora existe a parte grande.

## A trava que não custa nada tirar

Se o seu jogo está limitado a um número de quadros — por VSync ou por limitador
—, ele não está travado pela sua placa: está travado por **um número escrito num
arquivo**. Tirar isso devolve o que a máquina já era capaz de fazer, sem mudar
nada de como o jogo se parece.

O perfil **Tirar o limite de FPS** faz só isso. Nenhuma configuração visual é
tocada.

Os outros dois — **Equilibrado** e **Competitivo** — desligam o que é caro,
começando pela suavização de serrilhado, que numa placa de entrada custa sozinha
entre 30% e 50% dos quadros. Mais do que tudo que este programa faz no Windows,
somado.

**Antes de aplicar, você vê exatamente o que muda:** chave por chave, o valor de
agora, o valor novo e o que se perde. O arquivo inteiro é guardado antes, e
desfazer devolve ele byte a byte.

## Antes e depois, medido

O Otimiza mede o FPS do seu jogo, guarda, e compara depois das mudanças. Mostra
a média, os **1% piores quadros** — que é o que se sente como travada — e
quantos engasgos por minuto.

E se recusa a chamar ruído de ganho:

- Diferença abaixo de 3% é ruído de medição, não melhora. Duas medições seguidas
  sem mexer em nada já variam isso.
- Medição curta demais não vale como prova.
- **Se piorou, ele diz que piorou** e sugere desfazer.

Toda comparação avisa que as duas medições precisam ser feitas no mesmo lugar do
jogo. Menu e rua movimentada dão números muito diferentes na mesma máquina, e
comparar um com o outro produz um ganho que ninguém fez.

## O diagnóstico enxerga mais

A frase principal passou a considerar quatro coisas que antes só apareciam em
abas separadas:

| | |
|---|---|
| **Temperatura** | Um processador em throttling entrega uma fração do que pode, e nenhum ajuste de software resolve. É a resposta que falta em todo atendimento: o técnico limpa, otimiza, mede, e nada melhora porque o problema é físico. |
| **Disco cheio** | Abaixo de 10 GB o Windows falha de formas que ninguém associa a disco. |
| **Driver de vídeo** | Com mais de um ano, jogo recente perde quadros de graça. |
| **Programas em conflito** | Dois antivírus disputando o mesmo arquivo. |

## A máquina, desenhada

Três coisas que o programa já media e nunca mostrava:

- **A sua placa de vídeo**, com o driver, a data e a memória. Ele reconhece —
  não pergunta qual é.
- **Os encaixes de memória**, cheios e vazios. "Canal único" é jargão; quatro
  encaixes com um ocupado não precisa de tradução.
- **Os seus monitores**, cada um com a taxa dentro da tela. Numa máquina com
  dois, agora dá para ver qual está abaixo do máximo.

## Também nesta versão

- Ícones redesenhados em traço
- Fundo preto de verdade — sem camadas animadas pesando na máquina o tempo todo
- Um endereço de download que não muda a cada versão
- Aviso de versão nova por mensagem direta para quem comprou

---

## O que esta versão não promete

**Não existe número mágico.** Você vai ver por aí "até +300 FPS" e coisas do
tipo. O "até" é a palavra que torna a frase impossível de contestar.

O ganho real depende do que está segurando a **sua** máquina, e são três coisas
diferentes:

- Se o seu jogo está com teto de quadros, tirar o teto pode dobrar ou triplicar
  o número — e é instantâneo.
- Se está com as configurações gráficas pesadas para a sua placa, o ganho é de
  dezenas de por cento.
- Se o teto é a memória ou a temperatura, **nenhum software resolve**, e o
  Otimiza vai dizer isso em vez de vender ajuste que não vai adiantar.

Por isso ele mede antes e depois: o número que importa é o da sua máquina, e é
ele que a tela mostra — inclusive quando é zero.

**O "editor desconhecido" continua aparecendo.** O aviso do SmartScreen, lá em
cima, não é frescura do Windows: é ele dizendo, com razão, que não sabe quem
publicou este instalador. Resolver isso é comprar um certificado de assinatura,
e essa compra ainda não foi feita.
