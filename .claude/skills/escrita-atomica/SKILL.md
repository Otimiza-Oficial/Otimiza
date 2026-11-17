---
name: escrita-atomica
description: "Use ao gravar qualquer arquivo de estado do Otimiza (histórico, teto do jogo, base do jogo, portão, marcadores de teste em andamento, preferências) ou ao escrever um laço de fundo. Checklist para não corromper estado com queda de energia ou disco cheio e não engolir erro nem panic."
---

# Escrita atômica e laços que não morrem calados

## Gravar estado
1. Use o helper atômico `crate::utils::arquivo::escrever_atomico(caminho, bytes)` (temporário + flush + rename). Antes
   dele existir, o padrão está em `plataforma/jogos.rs` e `plataforma/hardware.rs`.
2. Nunca `let _ = gravar(...)` em estado que o desfazer, a recuperação ou o portão vão ler: o erro vai para
   `crate::utils::Logger::warn` e, quando a ação depende dele, a ação não acontece.
3. Formato novo de arquivo: `#[serde(default)]` nos campos novos (o cliente instala por cima da versão antiga).

## Marcador de "em andamento" (prova, calibração, energia de teste)
Grave **e confira** antes de mudar o PC. Sem marcador gravado, a mudança não começa. A abertura do app lê o marcador
e desfaz o que ficou pela metade.

## Laço de fundo
`let _ = spawn_blocking(...).await` engole o panic. O laço passa pelo supervisor (`vigiar(nome, tarefa)`) que registra
o panic e reinicia com espera.

## Encerramento
Nada espera para sempre ao fechar: join com tempo-limite.

Agente: `guardiao-da-escrita`.
