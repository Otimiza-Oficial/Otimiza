# Interface do Otimiza 1.0

A interface do Otimiza, em Svelte 5. A antiga (`src/main.ts` e `src/v33/`) foi apagada na Onda 2 de
`docs/planos/PLANO-1.0-NOVA-ERA.md`: o que ela fazia mora aqui.

| Comando (em `pc-optimizer/`) | O que faz |
|---|---|
| `npm run nova` | Abre o Otimiza (`tauri dev`) |
| `npm run nova:web` | Só a interface, no navegador, com dados de demonstração (a barra de menu avisa) |
| `npm run nova:build` | Confere os tipos (`svelte-check`) e gera `dist-nova/` |
| `npm run fumaca:nova` | Gera `dist-nova/` e clica em todas as áreas com todo comando do Windows recusado (`scripts/fumaca-nova.mjs`) |

O instalador sai com esta interface: `npm run tauri build` roda `npm run build:app`, que gera `dist-nova/`.
`npm run testes:ui` roda os testes de `ui/testes/`.

## Comando novo na interface

1. No Rust, o comando ganha `#[cfg_attr(test, specta::specta)]` e entra na lista de `src-tauri/src/contrato.rs`.
2. Os tipos que ele devolve ganham `#[cfg_attr(test, derive(specta::Type))]` (o compilador aponta cada um).
3. Em `src-tauri/`: `OTIMIZA_GERAR_TIPOS=1 cargo test --lib contrato` regrava `src/lib/tipos.ts`.
4. A interface chama pelo `commands` de `tipos.ts`, através de `lib/ponte.ts`. Nunca `invoke("nome")` solto.

## Regras

- **Com jogo aberto a interface para:** a classe `jogo-aberto` no `<html>` desliga animação e desfoque, e o laço de
  `App.svelte` só confere se o jogo continua aberto, a cada 5 s.
- **Nuvem é aviso, não enfeite:** cada uma vem de `lib/clima.ts`, de um dado lido.
- **Dado de demonstração nunca parece real:** fora do Tauri, `lib/ponte.ts` devolve a máquina de demonstração e a
  barra de menu diz "Dados de demonstração".
- Área que ainda não migrou mostra o que vai ter e em que onda chega. Nenhum botão finge funcionar.
