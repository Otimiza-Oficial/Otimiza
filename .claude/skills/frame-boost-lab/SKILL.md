---
name: frame-boost-lab
description: "Use para medir a qualidade da imagem do Frame Boost (gerador de quadros do Otimiza) antes e depois de mexer em shaders.hlsl, gpu.rs ou na composição do quadro gerado: métricas puras, shaders reais no WARP e clipes gravados do FiveM com o quadro do meio real como gabarito."
---

# Laboratório do Frame Boost

O gabarito é o próprio jogo: grava-se a 120–144 FPS, gera-se o quadro do meio a partir do 1º e do 3º, e compara-se
com o 2º real.

## Três níveis
1. **Métricas puras** (`geracao/metrica.rs`, qualquer máquina): SSIM/PSNR, erro na HUD, fantasma, cintilação, ritmo.
   `cargo test --lib -j 2 -- metrica`
2. **Shaders no WARP** (GPU em software, qualquer máquina, 320×180): cenas sintéticas (quadrado andando, mira fixa
   sobre panorâmica, corte de cena). `cargo test --lib -j 2 -- warp`
3. **Clipe gravado** (`#[ignore]`, só na máquina com o jogo): grave 2–3 s do FiveM com o Otimiza aberto, rode o teste
   ignorado do laboratório, e o relatório JSON sai em `.cargo-target/lab/<data>.json` (fora do C: quando possível).

## Ler o relatório
- Compare sempre com a linha de base **"repetir o quadro real"**: gerar só vale onde ganha dela.
- A métrica que manda é o **erro na HUD/mira** e o **fantasma**; SSIM geral sozinho engana.
- WARP não mede custo. Custo e atraso: skill `medir-latencia` na GTX 1650.
- Agente: `laboratorio-frame-boost`.
