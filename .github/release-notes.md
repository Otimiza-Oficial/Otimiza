Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_2.6.0_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_2.6.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão.

---

# A 2.6 mede antes de escolher: energia e geração de quadros

## O plano de energia deixou de ser uma lista fixa

Até a 2.5, o plano OTIMIZA escrevia os mesmos números em todo PC: preferência
de energia (EPP) em 0, estado mínimo do processador em 100%, núcleos todos
acordados e boost agressivo. Num desktop isso é quase inofensivo; num notebook
é calor a mais pelo mesmo FPS, e numa Intel híbrida é tirar do Windows a escolha
entre núcleo de desempenho e de eficiência.

A nova aba **Energia** troca isso por um motor que:

- **identifica o processador** pela própria CPU — família, modelo, se é
  híbrido, se tem Speed Shift, EPP ou CPPC — e se a máquina é desktop ou
  notebook;
- **lê o que este Windows expõe** (`powercfg /qh`), sem lista fixa: ajuste
  que não existe aqui, ou valor fora da faixa que o Windows publica, não é
  escrito;
- **testa candidatos feitos para a arquitetura**, medindo a resposta da CPU
  com rajadas de carga, o clock efetivo, os limites de firmware e — com o
  jogo aberto — FPS, 1% low, 0,1% low e P99;
- **escolhe pelo resultado.** 1% low, P99 e tempo de resposta pesam mais que
  FPS médio, e esquentar conta contra. Se nenhum candidato ganha do padrão do
  Windows com margem, a recomendação é o padrão do Windows.

O seu plano de energia nunca é escrito: o motor trabalha no plano OTIMIZA e
guarda um backup de todos os valores do plano anterior. **Restaurar plano
anterior** e **Restaurar padrão do Windows** estão na aba.

No notebook, o lado da bateria nunca é mexido.

Também na aba: laboratório de EPP, teste de economia do PCI Express e do USB,
perfis por jogo e o **modo dinâmico** — o perfil entra quando o jogo abre e sai
quando ele fecha.

**O plano OTIMIZA antigo mudou junto.** Ele não aplica mais sozinho EPP 0,
mínimo 100%, núcleos acordados, boost agressivo, ASPM desligado nem suspensão do
USB desligada. Esses valores só entram se o motor medir que eles ganham nesta
máquina.

## Laboratório de geração de quadros

A nova aba **Geração de quadros** não liga nada: você liga DLSS, FSR, Smooth
Motion, AFMF ou Lossless Scaling onde eles moram, e o Otimiza mede antes e
depois, sem encostar no processo do jogo.

- Separa sempre **quadros renderizados** (o que o jogo desenhou) de **quadros
  exibidos** (o que chegou à tela), e marca cada número como MEDIDO, ESTIMADO
  ou DESCONHECIDO.
- Diz se a máquina está pronta (EXCELENTE, BOA, MARGINAL, NÃO RECOMENDADA) a
  partir dos quadros reais, e avisa quando o processador é o limite.
- Compara 2×, 3× e 4× com pontuação por perfil — competitivo, equilibrado ou
  máxima fluidez — e uma decisão com o grau de confiança.
- Confere se o multiplicador do Lossless Scaling é o que chegou à tela, e se a
  cena medida se repetiu.
- Teste visual guiado de artefatos, e limite de FPS pela taxa do monitor (no
  driver NVIDIA, desfeito sozinho se a medição seguinte sair pior).

---

## O que esta versão não promete

**Nenhum ganho fixo.** O motor de energia pode concluir que o padrão do Windows
é o melhor para o seu PC — e aí é isso que ele diz.

**Geração de quadros não diminui atraso.** O laboratório estima o atraso que ela
acrescenta e nunca mostra o contrário.

**Temperatura e consumo do processador** só aparecem quando o Windows os expõe.
Na maioria dos PCs ele não expõe sem driver de terceiros, e o Otimiza não
instala um: nesses casos a tela diz "desconhecido".

**O "editor desconhecido" continua aparecendo.** O aviso do SmartScreen é o
Windows dizendo, com razão, que não sabe quem publicou este instalador.
