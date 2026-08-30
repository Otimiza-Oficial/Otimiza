Console de desempenho para Windows. Mede o que o seu PC está fazendo, aplica
otimizações reversíveis e mostra o resultado em número — inclusive quando o
resultado é que não mudou nada.

## O que instalar

| Arquivo | Quando usar |
|---|---|
| `Otimiza_0.24.0_x64-setup.exe` | **Comece por este.** Instalador comum, em português |
| `Otimiza_0.24.0_x64_en-US.msi` | Para instalação em rede ou por política de empresa |

Windows 10 ou 11, 64 bits.

**Na primeira execução o Windows vai mostrar "editor desconhecido".** É esperado:
o instalador ainda não tem assinatura digital. Clique em *Mais informações* e
depois em *Executar assim mesmo*.

---

# Correções críticas de usabilidade

A versão 0.24.0 corrige dois problemas que impediam o uso normal do programa.

## O que foi corrigido

### Scroll funcional
O problema: a tela não rolava para baixo. Seções longas ficavam cortadas e
inacessíveis — você via que havia mais conteúdo mas não conseguia chegar nele.

A causa: `overflow: hidden` no contêiner errado. O layout em grid com flex
precisava de `overflow` nos lugares certos para o navegador calcular a altura
disponível e permitir scroll no `.stage`.

Agora funciona: o conteúdo rola normalmente. Painéis longos, listas de
otimizações e diagnósticos completos são acessíveis com scroll natural.

### Fundo limpo e discreto
O problema: o fundo tinha gradientes verdes que não faziam parte da paleta do
produto, e uma grade quadriculada que competia visualmente com os dados.

O que mudou:
- Grade quadriculada removida — fundo liso
- Gradientes verdes removidos
- Apenas gradientes sutis em accent (branco) e âmbar
- Três camadas radiais em posições estratégicas
- Opacidades baixas (0.06 a 0.12) para não competir com o conteúdo
- Grão muito sutil com blend mode overlay
- Animação suave sem rotação

O resultado é um fundo **minimalista e profissional** que não disputa atenção
com os números que importam.

## O que não mudou

O núcleo continua o mesmo:
- Medições reais desta máquina
- Veredito baseado em dados
- Otimizações reversíveis
- Comparação honesta antes/depois
- Os três pilares desenhados com as próprias leituras

---

## Sobre a v0.23.0

A versão anterior (0.23.0) trouxe o redesign completo da interface mas saiu com
esses dois problemas. Esta versão corrige ambos.

Se você está na 0.22.0 ou anterior, a 0.24.0 traz tanto o redesign visual da
0.23.0 quanto as correções desta versão.
