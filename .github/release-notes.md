Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.15.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.15.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# Onde o FPS realmente está

Um cliente aplicou todas as otimizações do Otimiza e disse que o jogo continuou
igual. Estava certo — e o motivo é desconfortável para qualquer programa deste
tipo, inclusive o nosso.

Fomos olhar a configuração gráfica do jogo dele:

```
TextureQuality 0 · GrassQuality 0 · ShaderQuality 0 · WaterQuality 0
ParticleQuality 0 · PostFX 0 · ReflectionQuality 0 · CityDensity 0.0
MSAA 4          ← quatro vezes
```

Ele tinha baixado **tudo** ao mínimo e deixado a suavização de serrilhado em 4x.
Numa placa de entrada, essa é a configuração mais cara que existe no GTA V:
análises independentes medem entre 30% e 50% dos quadros só nela. Tudo o que ele
desligou junto vale menos que aquela linha.

A hierarquia real, para quem quer FPS num PC fraco:

```
uma configuração de jogo mal escolhida ..... dezenas de por cento
memória insuficiente ....................... o teto da máquina
ajustes de Windows, todos somados .......... alguns por cento
```

## O Otimiza agora lê a configuração do jogo — e não mexe nela

Esta versão passa a ler a configuração gráfica do GTA V e do FiveM, cruzar com a
placa de vídeo que a máquina tem, e dizer o que está pesando à toa, com o
caminho do menu para mudar.

**O programa não escreve nesse arquivo.** Nunca. Quem decide como o jogo se
parece é quem joga, e há um teste que reprova a compilação se alguém acrescentar
uma escrita nesse módulo.

Mas ficar calado sobre 40% de FPS por causa de uma regra de escopo seria
esconder do cliente a coisa mais valiosa que o programa sabe. Então ele conta —
exatamente como já faz com a taxa de atualização do monitor.

E não fala onde não deve: a mesma configuração numa placa boa não vira alerta
nenhum, e quando não dá para ler a memória da placa, o programa fica quieto em
vez de chutar.

## Seu plano de energia pode não ser do Windows

Programas de otimização e fabricantes de notebook criam planos de energia
próprios e os deixam ativos. Na máquina onde esta versão foi desenvolvida, o
plano em uso era o **"Driver Booster Power Plan"**, do IObit — e o dono não
sabia.

Alguns desses planos são bons. Outros limitam o processador para economizar
bateria, e quem instalou desinstalou o programa faz tempo. O Otimiza não mexe
nele sem você mandar, mas passa a dizer que o plano em uso não é nenhum dos que
o Windows traz.

## Dois defeitos nossos, encontrados na mesma máquina

**O produto não enxergava um plano que existia.** A verificação do "Desempenho
Máximo" procurava um identificador fixo — mas o `powercfg` dá identificador novo
a cada cópia, e o original nunca aparece na lista. Resultado: numa máquina que
já tinha o plano, o Otimiza dizia que não existia e oferecia criar um segundo.

**E o acento derrubava a segunda tentativa de conserto.** Ao passar a comparar
pelo nome, o plano "Desempenho Máximo" chegava como `M` + caractere de lixo +
`ximo`: o `powercfg` não é PowerShell, então não passa pelo trecho do programa
que força UTF-8, e a saída vem no código de página do console. A comparação
agora descarta tudo que não é ASCII nos dois lados — funciona com o acento
inteiro e funciona com ele corrompido.

---

## O que esta versão continua recusando

Um concorrente popular oferece trinta interruptores: notificações,
transparência, tarefas de diagnóstico, telemetria, UAC, Firewall, Windows
Update. **Nenhum deles muda FPS em jogo.** Eles mudam a sensação de que muita
coisa foi feita.

Três da lista são reais, e o Otimiza já tinha: Game Bar, agendamento de GPU por
hardware e plano de energia.

Continuam fora, com o motivo escrito no programa: desligar o Defender, o
Firewall ou o UAC (vender a segurança do cliente por alguns quadros), desligar o
Windows Update (zero FPS e o cliente sem correção de falha), e aumentar o
`TdrDelay` (não dá um quadro; só transforma uma recuperação de driver num
congelamento mais longo, e esconde defeito de hardware).

## O que esta versão não promete

Nada aqui inventa FPS onde falta hardware. Depois da configuração do jogo, o
teto é a memória — e o programa continua dizendo isso na primeira tela, antes de
qualquer botão.
