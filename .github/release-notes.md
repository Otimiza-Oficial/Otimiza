Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza-instalador.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza-instalador.exe.sha256` | A soma do instalador, para conferir que o arquivo chegou inteiro |
| `Otimiza_3.1.0_x64-setup.exe` | O mesmo instalador, com o número da versão no nome |
| `Otimiza_3.1.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits. A sua chave continua valendo: ela é presa ao
computador, não à versão.

---

# A 3.1 tira o que não dá FPS

## Menos abas

Saem as abas **Diagnóstico**, **Espaço**, **Programas** e **Reparo**. Nenhuma
delas deixava o jogo mais rápido ou mais liso. Ficam nove: Início,
Otimizações, Núcleos, Placa de vídeo, Jogos, Geração de quadros, Energia,
BIOS e Sistema.

## O Início mostra tudo

- **Todos os problemas da máquina** aparecem no Início, cada um com o
  conserto de um clique quando o Otimiza sabe fazer. Passando de quatro, o
  resto abre ali mesmo.
- **Integridade de Memória ligada**: custa FPS, e o Windows 11 passa a
  ligá-la sozinho. Agora o Início avisa.
- **Processador ou memória limitados no boot**, sobra comum de mexida no
  msconfig: o Início aponta e o catálogo corrige.

## O que saiu do catálogo

- **Liberar o Armazenamento Reservado**: só liberava disco. Quem aplicou
  continua conseguindo desfazer pelo histórico.

## Onde foi parar

- O relatório para o suporte está na aba **Otimizações**.
- A limpeza de disco, o mapa de pastas, o instalador de programas e o reparo
  do Windows saíram. Para liberar espaço, a Limpeza de Disco do próprio
  Windows (cleanmgr) faz o mesmo.
