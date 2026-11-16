---
name: jogo-no-catalogo
description: Use quando for pôr um jogo novo no catálogo do Otimiza (painel próprio na aba Jogos) ou mudar o perfil de um jogo existente (FiveM, Valorant, CS2, Fortnite, Apex...). O caminho de ponta a ponta — ficha, reconhecimento, perfil de config, cache de shader, anticheat, tela e testes.
---

# Jogo no catálogo

O catálogo dá a cada jogo: a **configuração do próprio jogo** (perfil leve),
o **cache de shader** dele e o que o **anticheat** deixa fazer. Tudo
reversível, tudo com fonte.

## Ordem

1. **Ficha primeiro.** Peça ao agente `pesquisador-de-jogo` a ficha
   `docs/jogos/<id>.md`. Sem ficha, não há perfil.
2. **Reconhecimento** — `pc-optimizer/src-tauri/src/plataforma/jogos.rs`,
   `CATALOGO` e `identificar(executavel)`: o executável vira o id do jogo
   (todas as versões do FiveM contam como `fivem`).
3. **Perfil de config** — `modules/windows/configmotor.rs`:
   - `perfil(id)`: formato (`IniDaUnreal { secao }`, KeyValues da Valve…) e
     as chaves com valor e `por_que` (uma linha, sem `\n` escapado).
   - `arquivos(id)`: onde o arquivo mora; o mais novo vence.
   - Regras que já existem e não mudam: só chave que **já existe**, recusa com
     o jogo aberto, guarda o arquivo inteiro (`ChangeRecord::GameConfig`,
     histórico `config_motor_<id>`).
   - Teste puro com um arquivo de exemplo real: o que muda e o que **nunca**
     muda (textura, serrilhado, distância de visão).
4. **Cache de shader** — `modules/windows/shaders.rs::locais()`: caches do
   driver valem para todos; cache do Steam com
   `shadercache_da_steam(<appid>)`.
5. **Anticheat** — `anticheat.rs`: se o anticheat barra janela por cima
   (`SobreporJanela`), o Frame Boost e a mira são recusados com o jogo aberto.
   Na dúvida, recuse.
6. **Tela** — a entrada do jogo em `ui/src/lib/catalogo.ts` (id, como
   reconhecer pelo nome/appid, perfil, caches, texto do anticheat, painéis
   extras). `CatalogoDoJogo.svelte` desenha a partir disso; nada de regex
   solto no `Jogos.svelte`. Siga a skill `tela-otimiza`.
7. **Comandos novos?** Registre em `commands/mod.rs` (`LIVRES` ou
   `EXIGEM_LICENCA`), `lib.rs` (`generate_handler`) e `contrato.rs`, e
   regenere os tipos:
   `OTIMIZA_GERAR_TIPOS=1 cargo test --lib contrato` (com
   `CARGO_TARGET_DIR=<raiz>/.cargo-target`, `-j 2`).

## Pronto quando

- `cargo test --lib`, `svelte-check` e `npm run testes:ui` passam (inclui o
  teste de catálogo: todo jogo tem perfil, caches e texto de anticheat).
- O `revisor-otimiza` não bloqueia.
- A ficha lista o que ficou "a confirmar" no jogo de verdade, e as notas da
  versão pedem esse teste.
