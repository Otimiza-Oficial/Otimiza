---
name: tela-otimiza
description: Use ao criar ou mudar qualquer tela, cartão ou componente Svelte do Otimiza (pc-optimizer/ui). As regras visuais e de interação — tokens, cartão padrão, estados de carregando/vazio/erro, selos de medido/estimado, modo demonstração fora do Tauri, desempenho com o jogo aberto — e o ciclo de prints para conferir.
---

# Tela do Otimiza

Svelte 5 (runes: `$state`, `$derived`, `$props`), em português, marca v2:
monocromática (preto, grafite, cinzas, branco), Geist, cor só para estado, o Oti
só muda o olhar. Use os componentes de `ui/src/componentes/base/` (Botao,
Interruptor, Campo, Estado, Esqueleto, SeloDeEstado, Dica, Abas) antes de
inventar o seu; erro passa por `explicarErro` (`lib/erros.ts`) e intervalo pelo
relógio central (`lib/relogio.ts`), nunca `setInterval`. Referências:
`pc-optimizer/ui/DESIGN.md`, `pc-optimizer/ui/src/tokens.css`,
`docs/planos/superpowers/specs/2026-11-13-ui-premium-design.md`. Use também as
skills `design-craft` e `frontend-design`.

## Regras

- **Só tokens.** Espaço `--e-1…8`, raio `--raio-1/2/3/4`, texto `--t-apoio/corpo/titulo/numero`,
  cores `--tinta*`, `--superficie*`, `--linha*`, `--ok/--atencao/--perigo`,
  movimento `--mov-*` e `--curva-*`. Nada de `14px`, `13px` ou `#hex` no
  componente. Faltou um token: crie no `tokens.css`, nos dois temas.
- **Classes globais antes de CSS local:** `.btn` (`.primario`, `.perigo`,
  `.quieto`), `.segmento`, `.selo` (`.medido`, `.estimado`), `.apoio`,
  `.ponto[data-tom]`, `.secao`, `.campo`, `.lista`, `.interruptor`. Não
  redefina `.recado` ou `.aviso` no componente.
- **Um botão primário por bloco.** Ação que apaga ou muda o PC pede
  confirmação **dentro do app** (componente de confirmação), nunca
  `confirm()` do navegador.
- **Três estados sempre** (o `Estado` da base): carregando (`Esqueleto`), vazio (com a
  próxima ação: "Abra o jogo uma vez e feche"), erro (o que aconteceu e o que
  fazer). "Não consegui ler" ≠ "não há nada".
- **Número com origem:** selo `medido` ou `estimado`. Sem promessa de ganho
  antes de medir.
- **Frases curtas** no idioma do jogador. Termo técnico (EPP, DXGI, shader)
  ganha meia frase de explicação.
- **Demonstração:** `noTauri` (de `lib/ponte.ts`) é **verdadeiro dentro do
  Tauri**. Fora dele, o componente mostra dados de demonstração (uma função
  `demo()`, não um objeto que lê `$props` — evita o aviso
  `state_referenced_locally`).
- **Comandos:** `commands.*` de `lib/tipos.ts` (gerado), embrulhados em
  `como<T>()` de `lib/contrato.ts`.
- **Jogo aberto:** a raiz recebe `.jogo-aberto`; blur e animação param. Efeito
  novo precisa respeitar isso e o `prefers-reduced-motion`.

## Conferir

Em `pc-optimizer`:
1. `npx svelte-check --workspace ui --tsconfig ./tsconfig.json` → 0 erros,
   0 avisos.
2. `npm run testes:ui` e `npm run fumaca:nova` (clica em todas as áreas).
3. `npm run nova:build` e `node scripts/prints-nova.mjs` (e `node scripts/prints-telas.mjs` para as telas da 1.0.12) → leia os prints
   `claro-*` e `escuro-*` da área (Read no PNG). O catálogo de cada jogo:
   `node scripts/prints-catalogo.mjs`. Ou chame o agente `designer-otimiza`.
4. Para ver ao vivo: `npm run nova:web` (porta 1420, modo demonstração).
