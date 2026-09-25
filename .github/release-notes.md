Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza-instalador.exe.sha256` | A soma do instalador, para conferir que o arquivo chegou inteiro |
| `Otimiza_3.0.0_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_3.0.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão.

---

# A 3.0 aponta o que está errado neste PC — e onde se resolve

## O Painel diz mais, logo na abertura

O veredito continua elegendo o problema principal, com o número que o
sustenta. Abaixo dele agora aparecem **os outros problemas da máquina**, cada
um com o conserto de um clique quando o Otimiza sabe fazer. Achados novos:

- **Monitor ligado no vídeo da placa-mãe** num PC que tem placa de vídeo: o
  jogo roda no vídeo integrado.
- **Driver de vídeo que caiu** e **erro de hardware** registrados pelo
  próprio Windows, com data.
- **Ryzen X3D de dois blocos** (7900X3D, 7950X3D, 9900X3D, 9950X3D) sem o que
  manda o jogo para o bloco com cache 3D.
- **Intel 13ª e 14ª geração de mesa** sem o microcódigo que a Intel publicou
  contra a instabilidade que degrada o processador.
- **Otimizações para jogos em janela** desligadas no Windows 11.
- **Intel APO disponível e não instalado**, nos processadores da lista da Intel.

## A aba BIOS

O Otimiza **não grava na BIOS** — em placa de consumo, um erro ali deixa a
placa sem ligar. A aba faz o resto:

- a ficha do firmware: placa, versão da BIOS, UEFI, Secure Boot, memória,
  microcódigo, Integridade de Memória e quanto a placa levou no último boot;
- o que olhar no menu, em ordem de risco, com o que foi medido nesta máquina;
- **reiniciar direto na BIOS** com um clique, guardando uma foto antes;
- na volta, **o que mudou** contra a foto;
- copiar ou salvar a ficha num arquivo, para guardar ou mandar ao suporte.

## Fluidez

- O limitador de FPS da NVIDIA sugere o **limite certo para G-Sync e
  FreeSync** no seu monitor (um pouco abaixo da taxa dele), e orienta o
  Reflex. O Otimiza continua **nunca limitando o FPS sozinho**.
- Sexto ajuste do driver NVIDIA: o **cache de shader sem limite de tamanho**,
  que evita recompilar e engasgar.
- O painel de inicialização mostra **quanto a placa-mãe levou** antes do
  Windows.
- O Painel volta a mostrar **quando o FPS de um jogo caiu com o tempo**, e o
  que mudou junto (driver, Windows ou nada).

## Modo jogo

O governador de segundo plano agora **só age com prova**: partidas com ele e
sem ele se alternam, e se o FPS médio ou o 1% pior caírem, ele devolve tudo e
fica parado naquele jogo. Sem a medição automática ligada, ele não age. Há um
botão para zerar a anotação dele, se ela estragar.

## O que saiu

- **A Biblioteca de jogos.** Quem aplicou um ajuste de jogo ou um perfil
  NVIDIA por jogo numa versão anterior continua conseguindo desfazer pelo
  histórico. Com ela saiu a varredura de Steam e Epic na abertura.

## Por baixo

- O instalador vem com a soma SHA-256 publicada ao lado, e a página de
  download mostra a mesma soma.
- O tempo de abertura do programa é medido e vai para o relatório de suporte.
