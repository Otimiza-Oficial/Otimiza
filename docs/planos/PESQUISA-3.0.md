# Pesquisa para a 3.0 — o que o cliente sente, e o que tirar

Feita em 24/09/2026. Cada número abaixo vem de uma fonte citada no fim; o que
não tem fonte é dito como opinião.

## A conclusão em uma frase

O cliente não sente média de FPS: sente **travada** (1% low, tempo de quadro
irregular), **atraso do comando** (latência) e **quanto tempo até o PC ficar
usável**. E as maiores diferenças nessas três coisas não vêm de chave de
registro: vêm de **configuração errada** da máquina, que o Otimiza já detecta
em parte e mostra escondido.

## 1. O que faz diferença que se sente — em ordem

### Nível A: grande, e comum

| Achado | Quanto | O app hoje |
|---|---|---|
| Monitor rodando abaixo da taxa máxima (144/165/180 Hz a 60 Hz) | a maior diferença de fluidez que um monitor dá | **detecta** (`display.rs`) — falta consertar com um clique |
| Cabo do monitor na placa-mãe em vez da placa de vídeo | o jogo roda no vídeo integrado | **não detecta** |
| Memória sem XMP/EXPO | 5–12% de FPS em jogo preso na CPU; mais em AMD | **detecta** (`causas.rs`) — só orienta |
| Memória em canal único | 1% low 16–27% pior; até 23% de FPS | **detecta** (`causas.rs`) |
| VRR (G-Sync/FreeSync) sem limite de FPS 3–5% abaixo da taxa, e V-Sync no driver | fora da faixa o VRR vira V-Sync comum, com atraso | **não orienta** |
| Reflex / Anti-Lag 2 desligados no jogo | ~50% menos latência com a placa no limite | só aparece junto de geração de quadros |

### Nível B: real, em parte das máquinas

| Achado | Quanto | O app hoje |
|---|---|---|
| Integridade de Memória / VBS ligada | 3–6% de média, até 15% de 1% low | **tem o ajuste** (`disable_vbs`, item a item, com troca de segurança) |
| **O Windows vai ligar a Integridade de Memória sozinho a partir de outubro de 2026** em PCs elegíveis | idem | **novo**: avisar quando foi o Windows que ligou. A Microsoft diz que não religa em quem já desligou |
| "Otimizações para jogos em janela" desligada (Win 11) | tira o atraso do modo sem borda — o modo em que muita gente roda o FiveM | **não detecta** |
| Ryzen X3D de dois CCDs usando o CCD errado | o jogo cai no CCD sem cache 3D | **não detecta** |
| Intel APO (14ª geração e Core Ultra 200S, jogos da lista) | até 14% de média, 21% de 1% low nos jogos listados | **não detecta** |
| Cache de shader da NVIDIA pequeno | recompila shader e engasga | **não ajusta** (o app já fala com a NVAPI) |
| Modo Xbox em tela cheia (Win 11, 2026) | ~9% menos RAM; um teste mediu até 8,6% de FPS | **não sugere** |

### A inicialização

| Achado | Quanto | O app hoje |
|---|---|---|
| Medir as fases: firmware, Windows e depois do login | é o que diz onde está o tempo | **mede** (`boot.rs`, evento 100) |
| AM5 com EXPO sem Memory Context Restore | o treino da memória a cada boot; MCR corta até ~1 min | **não detecta** |
| Programas que sobem junto + atraso de 10 s do Windows | até ~20 s até os programas subirem | **tem** (`startup.rs`, `disable_startup_delay`) |
| CSM ligado / Fast Boot do UEFI | 2–5 s | na lista da BIOS (`bios.rs`) |
| Inicialização Rápida do Windows | poupa segundos em SSD, muito em HD | `disable_hibernation` **desliga ela** — o texto já avisa |

## 2. O que tirar

- **A Biblioteca**, e a varredura de Steam e Epic na abertura do app (já no plano).
- **Apagar cache de shader como rotina.** Cache apagado é engasgo garantido
  na próxima partida, até recompilar. Deve sobrar só como conserto, depois de
  troca de driver com defeito, e dizendo o custo.
- **`disable_hibernation` fora de qualquer fluxo de "ligar mais rápido"**:
  ele deixa o boot mais lento. Continua valendo para quem precisa de disco.
- **A repetição de diagnóstico** (três painéis de "por que o FPS está baixo",
  sete motores de antes e depois) — já no plano.
- Os ajustes aposentados na 2.9 (`RETIRADOS`) seguem fora; a pesquisa não
  achou nada que os traga de volta.

## 3. Linux e Mac

- **Mac:** o Modo de Jogo do macOS já faz sozinho o que dá para fazer (prioridade
  de CPU e GPU para o jogo, Bluetooth com o dobro da taxa), e liga ao entrar em
  tela cheia. Sobra pouco para um otimizador fazer. **Não vale produto.**
- **Linux:** há alavancas reais — kernel 6.14+ com NTSYNC (menos engasgo em jogo
  do Windows pelo Proton), o escalonador `scx_lavd` (feito para 1% low),
  o GameMode da Feral, e Reflex/Anti-Lag 2 já funcionam lá. Mas o app inteiro
  é Windows (`modules/windows/`), e porte é produto novo. **Fora da 3.0**;
  se um dia, começa como diagnóstico e guia, não como otimizador.

## 4. O que muda no plano da 3.0

Entra, nesta ordem de valor:
1. **"O que está errado neste PC"** como primeira tela do Início: Hz abaixo do
   máximo, cabo no vídeo integrado, XMP, canal único, VBS ligada pelo Windows.
   Detecções que já existem, hoje espalhadas.
2. **Taxa do monitor com um clique** (e desfazer).
3. **Cabo na placa-mãe**: monitor ligado ao adaptador integrado havendo placa dedicada.
4. **Guia de fluidez por jogo**: VRR, limite de FPS 3% abaixo da taxa, V-Sync
   no driver, Reflex/Anti-Lag — com o valor do limite calculado para o monitor dele.
5. **Otimizações para jogos em janela**: ler e ligar (reversível).
6. **X3D de dois CCDs**: conferir Game Bar, o serviço de V-Cache da AMD e o driver de chipset.
7. **Cache de shader da NVIDIA** no tamanho máximo, pela NVAPI que já existe.
8. **Boot em fases** na tela, com o AM5/MCR e o CSM entrando como achados.
9. Intel APO e modo Xbox: só aviso, quando a máquina e o jogo baterem.

Cada um passa pela regra de sempre: medido antes e depois, e **nunca menos FPS**.

## Fontes

- Tom's Hardware, VBS/HVCI em jogos: https://www.tomshardware.com/news/windows-11-gaming-benchmarks-performance-vbs-hvci-security
- Integridade de Memória automática em outubro de 2026: https://www.helpnetsecurity.com/2026/09/03/windows-memory-integrity-update/ e https://www.tomshardware.com/software/windows/microsoft-will-expand-windows-11-memory-integrity-feature-to-more-pcs-starting-in-october-security-feature-reduces-gaming-performance-on-some-systems
- Blur Busters, G-SYNC 101: https://blurbusters.com/gsync/gsync101-input-lag-tests-and-settings/
- XMP/EXPO: https://www.xda-developers.com/why-you-should-enable-expo-xmp/ e https://www.rankedram.com/guides/xmp-expo-profiles-explained
- Canal único × duplo: https://www.techspot.com/article/3066-single-stick-vs-dual-channel-ram/
- Tempo de quadro e percepção: https://arxiv.org/pdf/2306.01691
- Otimizações para jogos em janela: https://support.microsoft.com/en-us/windows/optimizations-for-windowed-games-in-windows-11-3f006843-2c7e-4ed0-9a5e-f9389e535952
- Reflex: https://www.nvidia.com/en-us/geforce/news/reflex-2-even-lower-latency-gameplay-with-frame-warp/
- X3D de dois CCDs: https://forums.ea.com/discussions/battlefield-6-technical-issues-en/9950x3d-dual-ccd-x3d-javelin-prevents-core-parking-%E2%80%94-still-unresolved/13577327
- Intel APO: https://www.tomshardware.com/pc-components/cpus/intel-apo-update-boosts-gaming-performance-by-up-to-14-percent-in-select-titles-15-new-games-added-to-the-support-list
- Cache de shader: https://www.xda-developers.com/nvidia-auto-shader-compilation-fixes-stuttering-after-gpu-driver-updates/
- Advanced Shader Delivery: https://devblogs.microsoft.com/directx/advanced-shader-delivery-whats-new-at-gdc-2026/
- Modo Xbox em tela cheia: https://www.techpowerup.com/343221/xbox-full-screen-experience-uses-9-less-ram-than-standard-windows-11-mode
- AM5 e Memory Context Restore: https://www.xda-developers.com/this-am5-bios-change-halves-your-boot-time/
- Evento 100 de inicialização: https://eventlogxp.com/blog/windows-boot-performance-diagnostics-1/
- Atraso de inicialização do Windows: https://www.makeuseof.com/i-removed-windows-built-in-startup-delay-and-my-desktop-became-usable-much-faster/
- Inicialização Rápida e UEFI: https://windowsforum.com/threads/windows-11-fast-startup-pros-cons-and-how-to-toggle-it.383146/
- Modo de Jogo do macOS: https://support.apple.com/en-us/105118 e https://eclecticlight.co/2023/10/18/how-game-mode-manages-cpu-and-gpu/
- NTSYNC: https://www.phoronix.com/news/Linux-6.14-NTSYNC-Driver-Ready e https://www.gamingonlinux.com/2025/01/ntsync-for-proton-wine-now-in-linux-kernel-6-14-that-should-make-many-steamos-users-happy/
- scx_lavd: https://sched-ext.com/docs/scheds/rust/scx_lavd
- GameMode da Feral: https://manpages.ubuntu.com/manpages/focal/man8/gamemoded.8.html
