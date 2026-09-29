# Motion do Otimiza

O frontend usa TypeScript direto no DOM, CSS e Tauri 2. Não foi adicionada dependência.
As alterações preservam as cores, tipografia, estrutura e decisões de negócio existentes.

## Linguagem

Tokens em `src/styles.css`: pressionar 80 ms; interação 120 ms; superfície 160 ms;
contexto 200 ms; dados 220 ms. Curva comum `cubic-bezier(0.2, 0.75, 0.25, 1)`.
Sem spring: uma curva sem oscilação atende melhor a este console de desempenho.

- Páginas: deslocamento horizontal de 5 px, com direção conforme a ordem dos painéis.
  A seleção e o conteúdo mudam imediatamente; não se aguarda a saída da página anterior.
- Subpainéis: a mesma entrada curta. Onboarding: 4 px, sem atrasar o botão de continuar.
- Overlays: opacidade, 4 px e escala 0,985; entrada/saída reversíveis por CSS.
  `display allow-discrete` e `@starting-style` são melhorias progressivas. Em motores
  antigos o fechamento é imediato. `hidden`, `inert` e foco não esperam pela animação.
- Listas: no máximo os oito primeiros filhos recebem entrada, sem stagger prolongado.
  Não se faz FLIP, mede centenas de retângulos ou anima reordenação de listas inteiras.
- Controles: feedback de cor e pressão de 1 px; foco permanece imediato.
- Dados: barras de telemetria usam `scaleX`, matriz usa `scaleY`; skeleton usa opacidade.

## Performance e acessibilidade

`src/motion.ts` controla foco e estado dos overlays; não usa timers nem cálculos por frame.
Subabas mantêm os mesmos nós. Setas na paleta atualizam a seleção sem reconstruir a lista.
A rolagem da paleta é reiniciada na filtragem; o campo mantém sua altura na janela compacta.

`src/esfera.ts` suspende o canvas fora da viewport e com o documento oculto, com cleanup
de observer e listener. Animações CSS ficam pausadas quando o documento está oculto.
Não há promoção permanente de cada barra de núcleo com `will-change`.

`prefers-reduced-motion` é aplicado pelo CSS e atualizado em runtime no controle de
movimento. A opção já existente para hardware fraco permanece válida. Sem delays residuais.

## Verificação reproduzível

```powershell
npm run fumaca
# Opcional: registra capturas nas resoluções 1400×1000 e 900×650.
$env:MOTION_QA_DIR = 'C:/caminho/para/capturas'
npm run fumaca
```

O comando inclui TypeScript estrito, build Vite e Chrome real sem janela. Verifica 12
rotas/subabas, foco, identidade dos nós, abrir/fechar/reabrir rapidamente, redução de
movimento em runtime, 1.000 itens sintéticos (máximo oito animações) e 40 navegações rápidas.
O backend é simulado como indisponível, exercitando também estados de erro sem executar
otimizações do Windows. Não substitui medição de FPS no WebView2 com telemetria real.

Na rodada de QA de 27/09/2026, as 40 chamadas de navegação levaram aproximadamente 6,2 ms
de JavaScript no Chrome. Esse número não inclui pintura e não é uma garantia de FPS.
O bundle JavaScript passou de 83,92 para 84,84 kB gzip; sem biblioteca adicional.
O projeto não possui script de lint frontend separado; `tsc` habilita strict,
noUnusedLocals, noUnusedParameters e noFallthroughCasesInSwitch.

Validação nativa: `cargo test --lib --locked` concluiu com 1.362 testes passando,
zero falhas e 58 ignorados. `cargo build --locked --features tauri/custom-protocol`
gerou `src-tauri/target/debug/pc-optimizer.exe`, incorporando o frontend de produção.
É um build de desenvolvimento, não um instalador de release. O executável abriu no
WebView2 e sua árvore de acessibilidade mostrou os painéis e dados reais.

Limite da revisão nativa: o Windows Graphics Capture falhou com `0x80070422`
(serviço desativado), impedindo captura visual e cliques por geometria. A revisão de
layout e os testes repetidos de interação foram executados no Chrome. A tentativa de
atalho nativo encontrou entrada concorrente do usuário; não foi contada como teste aprovado.
Os avisos de compilação Rust são de código nativo que não foi modificado nesta tarefa.

As alterações já existentes em guardião, Rust, HTML e release foram preservadas.
