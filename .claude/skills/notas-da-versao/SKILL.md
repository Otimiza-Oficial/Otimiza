---
name: notas-da-versao
description: "Use para escrever as notas de uma versão nova do Otimiza (.github/release-notes.md e release-titulo.txt) e o recado do aviso de versão: como correção de bugs, em português do cliente, com instruções conferidas no código."
---

# Notas da versão

1. Liste os commits desde a última tag: `git log v4.0.X..HEAD --oneline`.
2. Para cada mudança que o cliente sente, um item: **o que acontecia** (como ele sentia) → **o que mudou** → **o que
   fazer se foi afetado**.
3. Confira no código cada nome citado (item do Histórico, caminho na tela: `git grep "texto"`).
4. Nada de número que não foi medido (skill `medir-latencia`).
5. Seção nova `# 1.0.X: Correções de bugs ...` acima da anterior, depois de `---`; troque os nomes dos instaladores
   `Otimiza_4.0.X_x64-setup.exe` e `Otimiza_4.0.X_x64_pt-BR.msi`; título em `release-titulo.txt`.
6. Agente: `redator-de-notas`. Depois, skill `publicar-versao` (só com o ok do dono).
