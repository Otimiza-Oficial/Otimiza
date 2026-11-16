---
name: medir-latencia
description: "Use para medir atraso e ritmo de verdade (não estimado) no Otimiza: PresentMon no jogo e na sobreposição do Frame Boost, como ler PresentMode, msBetweenDisplayChange e a latência, e o que conta como tranco. Para afirmar 'menos atraso' ou 'sem tranco' numa nota de versão."
---

# Medir latência e ritmo

## Com o Otimiza
O Otimiza já roda PresentMon por dentro (`modules/windows/presentmon.rs`: `Vivo` no jogo e "OtimizaNaTela" na
sobreposição). O registro (Ajustes → Registro do Otimiza → Salvar em Downloads) traz o resumo de cada sessão.

## O que olhar
- **PresentMode da sobreposição:** `Composed: Flip` = passa pelo DWM (um quadro a mais); `Hardware: Independent Flip`
  = direto. O Frame Boost atual é `Composed`.
- **Ritmo:** intervalos entre quadros exibidos (`msBetweenDisplayChange`); p99 acima de 1,5× a mediana é tranco
  que o olho vê. Repetição de quadro (a mesma imagem por duas atualizações) também.
- **Atraso do Frame Boost:** para o mesmo quadro, quando o jogo apresentou × quando a sobreposição exibiu. A
  interpolação exige esperar o próximo quadro real: (M−1)/M de intervalo é o mínimo.
- **FPS real do jogo** com e sem o Frame Boost: perda acima da régua da guarda é defeito.

## Regras
- Número na tela ou na nota: **medido** (PresentMon) ou **estimado**, sempre dito.
- Uma sessão de 60 s no mesmo lugar do jogo, sem e com, na mesma máquina. Testadores: GTX 1650 + 144 Hz.
- Agente: `latencia-e-ritmo`.
