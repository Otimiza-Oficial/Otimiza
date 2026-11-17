---
name: publicar-versao
description: "Use quando o dono aprovar publicar uma versão nova do Otimiza (ex. 'pode publicar a 1.0.5'). O ritual completo: número da versão nos 7 arquivos, notas, PR de ui/premium para main, checagens, merge, tag anotada e acompanhar a esteira 'Publicar versão' até os instaladores saírem."
---

# Publicar versão

**Só com o ok explícito do dono** para esta versão. Aprovação de uma versão não
vale para a próxima.

## Números

Versão interna `4.0.X` = versão que o cliente vê `1.0.X` (a 4.0.0 é a "1.0").
Tag: `v4.0.X`.

## 1. Os 7 arquivos (a partir da raiz `Otimiza-novo`)

| Arquivo | O que muda |
|---|---|
| `pc-optimizer/package.json` | `"version": "4.0.X"` |
| `pc-optimizer/package-lock.json` | `"version"` nas linhas 3 e 9 (só essas duas) |
| `pc-optimizer/src-tauri/Cargo.toml` | `version = "4.0.X"` |
| `pc-optimizer/src-tauri/Cargo.lock` | o bloco `name = "pc-optimizer"` / `version` |
| `pc-optimizer/src-tauri/tauri.conf.json` | `"version": "4.0.X"` |
| `.github/release-titulo.txt` | `Otimiza 1.0.X <resumo curto> (4.0.X)` |
| `.github/release-notes.md` | nomes `Otimiza_4.0.X_x64-setup.exe` e `Otimiza_4.0.X_x64_pt-BR.msi`; seção nova `# 1.0.X: ...` acima da anterior, depois de `---` (o histórico fica) |

Faça por script que **conta as ocorrências antes de trocar** (`assert
s.count(a) == 1`) e preserva o fim de linha (`newline=""`). Escreva o script
num arquivo e rode com `python -I arquivo.py`: heredoc com aspas no Bash
quebra.

**Notas:** escritas como **correção de bugs**, em português do cliente, sem
número inventado. Diga o que a pessoa faz se foi afetada (ex.: desfazer no
Histórico).

## 2. Antes da tag

- `cargo test --lib -j 2` com `CARGO_TARGET_DIR=<raiz>/.cargo-target`
  (C: quase cheio: uma tarefa pesada por vez).
- `npx svelte-check --workspace ui --tsconfig ./tsconfig.json` e
  `npm run testes:ui` em `pc-optimizer`.
- Chame o agente `verificador-de-publicacao`.

## 3. Publicar

```
git push origin ui/premium
gh pr create --base main --head ui/premium --title "1.0.X (4.0.X): ..." --body "..."
gh pr checks <n> --watch --interval 30
gh pr merge <n> --merge
git fetch origin
git tag -a v4.0.X origin/main -m "Otimiza 1.0.X (4.0.X)"
git push origin v4.0.X
gh run list --workflow "Publicar versão" --limit 1
```

A esteira leva ~26 min. Acompanhe em segundo plano
(`gh run watch <id> --exit-status`) e confira:
`gh release view v4.0.X --json isDraft,assets` → não rascunho, com
`Otimiza-instalador.exe`, `.sha256`, `Otimiza_4.0.X_x64-setup.exe` e
`Otimiza_4.0.X_x64_pt-BR.msi`.

## 4. Depois

Avise o dono com o link
`https://github.com/Otimiza-Oficial/Otimiza/releases/tag/v4.0.X` e o que
precisa ser testado no jogo. O aviso no Discord (`/aviso-versao`) é o dono quem
manda.
