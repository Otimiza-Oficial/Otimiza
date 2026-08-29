# Otimiza

Console de desempenho para Windows. Mede o que o PC está fazendo, aplica
otimizações reversíveis e **prova o resultado com número** — inclusive quando o
resultado é "não houve ganho".

## O problema com os otimizadores de PC

Quase todo programa dessa categoria funciona assim: mostra uma barra de
progresso, aplica uma lista de ajustes copiada da internet e anuncia um número
inventado. O cliente não tem como verificar, e o ganho normalmente é zero.

O Otimiza é construído em cima da recusa a fazer isso.

## Como ele é diferente

**Mede antes e depois, e admite quando não mudou nada.** Os limiares de ruído não
foram chutados: vieram de um teste que mede a mesma máquina três vezes sem alterar
coisa alguma. O que variou ali é ruído, e nunca é reportado como ganho. Duas das
seis métricas nem geram veredito, porque a calibração provou que oscilam demais
sozinhas — aparecem marcadas como "só referência".

**Mede a travada, não só a média.** Ninguém reclama de "média de FPS baixa" —
reclama que o jogo *engasga*. Um congelamento de 40 ms arruína a suavidade e quase
não mexe na média de 60 quadros por segundo. O Otimiza mede o atraso do agendador
do Windows direto, que é o que o jogador sente.

**Se recusa a medir quando o PC está ocupado.** Comparar um PC ocupado com um PC
descansado inventa um ganho de dezenas por cento. Acima de 25% de uso de CPU,
nenhum veredito é emitido e o programa explica por quê.

**Lê o hardware antes de oferecer.** Desativar o SysMain ajuda em SSD e atrapalha
em disco mecânico. Desligar a compressão de memória ajuda com RAM sobrando e
piora com 8 GB. O Otimiza detecta a máquina e **não oferece** o que faria mal a
ela.

**Diz quando o PC já está otimizado.** Se a configuração já está aplicada, ele
mostra "já otimizado" em vez de fingir trabalho.

**Tudo é reversível, com o valor exato.** Cada mudança grava o estado anterior
antes de escrever. Desfazer não restaura algo "equivalente": restaura idêntico,
byte a byte.

**Mostra o que está fazendo enquanto faz.** Cada alteração aparece ao vivo, com o
valor que existia antes.

## O que ele se recusa a fazer

- Desativar as proteções da CPU contra Spectre/Meltdown, mesmo rendendo FPS real
- Desligar Windows Update, Defender ou firewall
- "Limpeza de registro", que não tem ganho medível e quebra programa instalado
- Liberar RAM à força, o que deixa o gráfico bonito e o PC mais lento
- Escrever na BIOS — em placa de consumo, errar ali inutiliza a placa-mãe

Sobre a BIOS: o Otimiza **lê** o que ela está fazendo com o desempenho e aponta
onde se resolve — software, BIOS ou troca de peça. Memória em canal único, XMP
desligado, limite de núcleos no boot e estrangulamento térmico são detectados e
explicados, mesmo quando a resposta honesta é "nenhum programa resolve isto".

## Como rodar

```bash
cd pc-optimizer
npm install
npm run tauri dev
```

Para gerar os instaladores:

```bash
npm run tauri build
```

Saem em `pc-optimizer/src-tauri/target/release/bundle/`.

## Arquitetura

- **Backend:** Rust + Tauri 2 (`pc-optimizer/src-tauri/src`)
- **Frontend:** TypeScript + Vite, sem framework
- **Estado das mudanças:** `%APPDATA%\pc-optimizer\changes.json`

A pasta ainda se chama `pc-optimizer`, de antes do produto ganhar nome. Renomear
agora deixaria para trás o histórico de quem já instalou — e sem histórico não
existe "desfazer". O nome feio fica até haver uma migração que preserve os dados.

Leitura do sistema é feita pelo **registro e pelo WMI**, não pela saída de
comandos. O `sc qc` num Windows em português imprime `TIPO_DE_INÍCIO` em vez de
`START_TYPE`; qualquer parsing de texto quebraria justamente nas máquinas do
público-alvo.

## Documentação

- [`pc-optimizer/PROGRESS.md`](pc-optimizer/PROGRESS.md) — o que está pronto, o
  que foi verificado e como, e o que falta
- [`pc-optimizer/docs/LICENCA.md`](pc-optimizer/docs/LICENCA.md) — como o
  sistema de chave funciona, o que fazer antes da primeira venda, e o que
  responder em cada caso que aparece no suporte
- [`pc-optimizer/docs/ASSINATURA.md`](pc-optimizer/docs/ASSINATURA.md) —
  assinatura digital do instalador

O `PROGRESS.md` registra apenas o que foi **verificado**. Funcionalidade que
existe no código mas nunca foi executada aparece como pendente.

## Licença

Código aberto à leitura, não ao uso. Veja [`LICENSE`](LICENSE).

O código está público porque um programa que altera configurações do seu sistema
deveria poder ser auditado por quem instala. Isso não é o mesmo que licença de
uso: copiar, redistribuir ou usar comercialmente exige autorização por escrito.
