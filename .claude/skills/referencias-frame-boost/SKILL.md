---
name: referencias-frame-boost
description: "Use antes de propor ou revisar qualquer mudança no Frame Boost (geração de quadros, fila, apresentação, captura, medida pelo PresentMon) e quando um cliente disser que 'muda pouco' ou 'ficou com atraso'. As fontes primárias (AMD FSR 3, Microsoft DXGI, PresentMon, Lossless Scaling) resumidas, com o que cada uma diz e o que isso significa no código do Otimiza. Não confie em memória: confira aqui e no link."
---

# Referências do Frame Boost (fontes primárias)

Regra: afirmação técnica sobre geração de quadros, fila ou medida cita uma destas fontes ou uma medida nossa. Fonte
de fórum ou Reddit é pista, não prova. Conferido em 19/11/2026; antes de citar detalhe de API, abra o link.

## 1. O limite físico (o que nenhum ajuste muda)
- Interpolar exige esperar o real seguinte: o quadro do meio entre N e N+1 só existe depois que N+1 chegou. O atraso
  mínimo acrescentado é perto de um intervalo do jogo, e piora com FPS base baixo (30→60 sente mais que 72→144).
  Fontes: [GFFE, arXiv 2406.18551](https://arxiv.org/pdf/2406.18551) (interpolação aumenta o atraso clique→tela);
  [SIGGRAPH 2010, frame rate up-conversion](http://and.intercon.ru/rtfrucvg_html_slides/) ("um quadro de atraso a mais").
- Quadro gerado não carrega estado novo do jogo: deixa o movimento mais liso, **não** deixa o controle mais rápido
  ([XDA](https://www.xda-developers.com/this-gpu-driver-setting-cut-my-frametime-variance-in-half-and-competitive-fps-games-feel-completely-different-now/)).
- Consequência para o produto: "muda pouco" pode ser a verdade física quando o jogo já está perto dos Hz do monitor
  (não cabe quadro gerado: `ritmo::multiplicador_que_cabe`) ou quando a pessoa julga pela mão, e não pelo olho.

## 2. AMD FSR 3 / FSR Frame Generation (o gerador aberto de referência)
- Manuais: [Frame Generation 3.1.6](https://gpuopen.com/manuals/fsr_sdk/techniques/frame-interpolation/),
  [API](https://gpuopen.com/manuals/fsr_sdk/techniques/frame-interpolation-api/),
  [visão geral](https://gpuopen.com/fsr3/), [detalhes de integração](https://gpuopen.com/fsr3-in-games-technical-details/),
  [GDC 2024 (Snowdrop)](https://GPUOpen.com/gdc-presentations/2024/GDC2024_High_Performance_Rendering_in_Snowdrop_Using_AMD_FidelityFX_Super_Resolution_3.pdf),
  [FSR 3.1, integração](https://gpuopen.com/presentations/2024/FidelityFX_Super_Resolution_3-1_Release-Overview_and_Integration.pdf).
- Etapas: fluxo óptico → geração e composição → espera de ritmo, numa swap chain própria que apresenta reais e
  gerados (fila assíncrona opcional). Usa os vetores de movimento do jogo; o Otimiza **não tem** esses vetores (fica
  por fora do jogo) e estima o movimento só pela imagem, como o AFMF e o Lossless Scaling.
- HUD: o FSR 3 recebe a interface separada (callback de UI ou a imagem sem UI × com UI). Sem isso, o caminho do
  Otimiza é detectar pixel parado com fluxo grande em volta (`ErroNaCruzSemHud` em `geracao/shaders.hlsl`).
- Ritmo: a diferença grande é com e sem VSync. Sobreposição de terceiros que intercepta DXGI pode quebrar o FSR 3:
  o Frame Boost não intercepta nada (captura + janela própria), o que o deixa compatível com anticheat.

## 3. Microsoft DXGI: fila e atraso da apresentação
- [Reduzir latência com swap chains DXGI 1.3](https://msdn.microsoft.com/en-us/library/ie/dn448914(v=win.10)): criar
  com `DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT` e esperar o objeto **antes** de desenhar cada quadro. A
  flag não pode ser trocada depois (`ResizeBuffers` falha).
- [`IDXGISwapChain2::SetMaximumFrameLatency`](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_3/nf-dxgi1_3-idxgiswapchain2-setmaximumframelatency):
  só vale com a flag acima (senão `DXGI_ERROR_INVALID_CALL`); o padrão é 1. Sem o objeto, a latência efetiva chega
  a 3 ([amostra da Intel](https://intel.com/content/www/us/en/developer/articles/code-sample/sample-application-for-direct3d-12-flip-model-swap-chains.html)):
  era o caso dos modos fixos até a 1.0.7.
- Independent flip exige janela sem bordas em tela cheia e flip model fora do modo janela
  ([NVIDIA, swap chains](https://developer.nvidia.com/blog/advanced-api-performance-swap-chains)). A sobreposição do
  Otimiza é composta pelo DWM (DirectComposition), então normalmente fica em "Composed: Flip": um passo de
  composição a mais. Só afirme o modo pelo `PresentMode` do PresentMon.

## 4. PresentMon: o que cada número quer dizer
- Repositório e versões: [GameTechDev/PresentMon](https://github.com/GameTechDev/PresentMon/releases). Desde a 2.0
  as métricas partem de quando a CPU **começou** o quadro (`CPUStartQPC`), não do `Present()`.
- `CPUBusy`: do começo do quadro até a chamada do `Present`. `DisplayLatency`: do começo do quadro até a tela.
  `DisplayLatency − CPUBusy` = da chamada do `Present` até a tela (`presentmon::fila_ate_a_tela`).
- `MsBetweenDisplayChange` (v1) / intervalo entre `DisplayedTime`: o que o olho vê; é ele que mostra repetição e
  tranco, e não o tempo entre `Present`s ([Wccftech, tempos de exibição](https://wccftech.com/display-times-frame-times-and-the-truth-about-smoothness/)).
- `AnimationError`: com sinal; quadro mostrado tarde é negativo, cedo é positivo
  ([GamersNexus](https://gamersnexus.net/gpus-gn-extras-cpus/problem-gpu-benchmarks-reality-vs-numbers-animation-error-methodology-white)).
  Bom candidato para medir o ritmo do gerado contra o instante certo.

## 5. Lossless Scaling (o concorrente que o cliente conhece)
- [Página oficial do modo adaptativo](https://losslessscaling.com/adaptive-frame-generation/) e
  [guia da Corsair](https://www.corsair.com/explorer/gamer/gaming-pcs/lossless-scaling-everything-you-need-to-know/).
- Captura: fila alvo de 1 por padrão; "sem fila" usa sempre o último quadro (menos atraso); fila 2 só para base
  instável. Captura DXGI é a recomendada (mais rápida); WGC quando dá tela preta.
- Modo adaptativo (AFG): gera a maior parte dos quadros para chegar num FPS alvo; mais carga e um pouco menos de
  qualidade que o multiplicador fixo. Abaixo de 10 FPS base, desliga a geração.
- Pede base estável: o cálculo espera um ritmo regular. É o mesmo motivo do teto de FPS do jogo no Otimiza.

## 6. AMD AFMF 2 / 2.1 e NVIDIA Smooth Motion (geração pelo driver)
- AFMF 2 com Anti-Lag: 28% menos atraso que o AFMF 1, números da AMD
  ([Tom's Hardware](https://www.tomshardware.com/pc-components/gpus/amd-fluid-motion-frames-2-lowers-latency-by-28-percent)).
- **Search Mode:** em movimento rápido, desliga a geração em vez de mostrar quadro ruim; o 2.1 testa "Repeat
  Frame" e "Blended Frame" ([notas da prévia](https://www.amd.com/en/resources/support-articles/release-notes/RN-RAD-WIN-AFMF2-TECH-Preview.html),
  [igor'sLAB](https://www.igorslab.de/en/amd-expands-afmf-2-1-with-fast-motion-response-finally-an-answer-to-fast-game-scenes/)).
  É o modelo do "recuo em movimento rápido" do Plano 2.0.
- Base mínima recomendada: 55 FPS em 1080p, 70 em 1440p ou mais (AFMF 1,
  [notas](https://www.amd.com/en/resources/support-articles/release-notes/RN-RAD-WIN-23-30-AFMF-TECH-PREVIEW.html));
  FSR 3 pede 60 e, com VSync, teto em metade dos Hz ([GPUOpen](https://gpuopen.com/fsr3-in-games-technical-details/)).
- Smooth Motion: modelo de IA no driver, só RTX 40/50
  ([NVIDIA](https://www.nvidia.com/en-us/geforce/news/nvidia-app-global-dlss-overrides-rtx-40-series-smooth-motion/));
  não serve para a GTX 1650.

## 7. Escala do fluxo (Lossless Scaling)
- O movimento é estimado numa imagem reduzida e o quadro sai em resolução cheia; mais alto custa bem mais placa,
  mais baixo dá mais artefato. Comuns: 80–100% em 1080p, ≈75% em 1440p, 40–50% em 4K (comunidade:
  [guia](https://sageinfinity.github.io/docs/Guides/LS%20settings%20Info)). LSFG 3: ≈40% menos carga que o 2 no
  X2 ([anúncio](https://losslessscaling.com/announcing-lsfg-3/)).

## 8. Limitadores de FPS, fila e o que é placebo
- LDAT no Overwatch: limitador do jogo ≈16 ms, Reflex ≈16 ms, RTSS e "Max Frame Rate" da NVIDIA ≈18 ms, sem teto
  ≈24 ms ([Blur Busters](https://forums.blurbusters.com/viewtopic.php?f=5&p=86590&t=9151),
  [G-SYNC 101](https://www.blurbusters.com/gsync/gsync101-input-lag-tests-and-settings/11/)). O do driver não
  injeta nada (seguro com anticheat); o RTSS intercepta o `Present` (arriscado).
- Reflex exige o kit dentro do jogo; de fora, o que dá é o "Low Latency Mode" do perfil do driver
  ([NVIDIA](https://www.nvidia.com/en-us/geforce/news/reflex-low-latency-platform/)).
- `ALLOW_TEARING` dá menos atraso que só o objeto de espera, mesmo em janela, com MPO
  ([Microsoft](https://devblogs.microsoft.com/directx/dxgi-flip-model/)).
- HAGS: a Microsoft diz que "os usuários não devem notar mudanças significativas"
  ([DevBlog](https://devblogs.microsoft.com/directx/hardware-accelerated-gpu-scheduling/)); testes mistos
  ([Tom's](https://www.tomshardware.com/uk/news/gpu-hardware-scheduling-tested)). Não vender como ganho.
- "Game boosters": ≈5% no melhor caso medido ([GameStar](https://www.gamestar.de/artikel/game-booster-test,3383476.html));
  números de Hone/ExitLag são da própria empresa.

## Como usar no código
| Pergunta | Onde olhar no Otimiza | Fonte |
|---|---|---|
| A fila está em 1? | `Sobreposicao::nova`, `ritmo::vez_no_modo_fixo` | §3 |
| O atraso na tela é honesto? | `ritmo::atraso_acrescentado`, `presentmon::fila_ate_a_tela` | §1, §4 |
| A HUD está protegida? | `ErroNaCruzSemHud`, `FinalBidirecional` | §2 |
| A tela está lisa? | `gerador: tela medida` no log, `ritmo::ritmo_na_tela` | §4 |
| Por que "muda pouco"? | `multiplicador_que_cabe`, FPS base × Hz no log | §1, §5 |
