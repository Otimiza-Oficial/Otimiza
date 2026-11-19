---
name: onda-segura
description: "Use ao fazer QUALQUER onda de mudança no Otimiza (código Rust ou Svelte em pc-optimizer/). O ritual que evita estragar o PC do cliente: build com o alvo do projeto, testes, revisão pelos agentes certos para a área, e commit pequeno. Também diz quando parar e perguntar ao dono."
---

# Onda segura

Cada onda é pequena, testada e revisada antes da próxima. Nada é publicado sem o "pode publicar" do dono.

## 1. Antes de mexer
- Leia o plano da versão em `docs/planos/` e a memória do projeto.
- Mudança que altera o comportamento **para todos os clientes** (padrão novo, item que entra ou sai do lote): pergunte
  ao dono antes. A preferência dele não vira padrão sem perguntar.

## 2. Build e testes (uma tarefa pesada por vez: o C: está quase cheio)
Em `pc-optimizer/src-tauri`:
```
CARGO_TARGET_DIR="<raiz>/.cargo-target" cargo test --lib -j 2
```
Comando Tauri novo ou mudado: `OTIMIZA_GERAR_TIPOS=1 cargo test --lib contrato` (regenera `ui/src/lib/tipos.ts`) e
registre em `commands/mod.rs` (LIVRES ou EXIGEM_LICENCA), `lib.rs` (generate_handler) e `contrato.rs`.

Em `pc-optimizer`:
```
npx svelte-check --workspace ui --tsconfig ./tsconfig.json
npm run testes:ui
npm run nova:build && npm run fumaca:nova
```
Teste que lê a máquina leva `#[ignore = "lê esta máquina"]`. Decisão nova leva teste puro com os números do caso real.

## 3. Revisão (agentes do projeto)
Tarefa grande ou ambígua: comece pelo `lider-tecnico` (plano em ondas e quem chamar). Retorno de cliente: pelo
`gerente-de-produto`. Frame Boost: a skill `referencias-frame-boost` antes de propor.

| Área mexida | Agente |
|---|---|
| FPS: Frame Boost, Turbo, Jogo liso, teto, energia de jogo, governador | `guardiao-do-fps` |
| Apresentação, fila, ritmo, PresentMon | `latencia-e-ritmo` |
| Qualidade da imagem do Frame Boost | `laboratorio-frame-boost` |
| Gravar estado, laços, recuperação, encerramento | `guardiao-da-escrita` |
| Admin, PowerShell, licença, atualizador | `auditor-de-seguranca` |
| Laços e custo com o jogo aberto | `custo-do-otimiza` |
| Ajuste do Windows no catálogo | `auditor-de-ajuste` |
| Tela | `designer-otimiza` (skill `tela-otimiza`) |
| Técnica nova do gerador, comparação com concorrente | `pesquisador-de-geracao` (implementa: `engenheiro-de-shaders`) |
| Turbo, afinidade, CPU sets, prioridade | `especialista-processador` |
| Perfil do driver, NVML, ReBAR, placa híbrida | `especialista-placa-e-driver` |
| Hz, VRR, HDR, tela cheia | `especialista-tela` |
| Trancos, DPC, travamentos, temperatura | `especialista-diagnostico` |
| FiveM | `especialista-fivem` |
| Oti (IA) | `especialista-oti` |
| Número novo na tela, prova A-B | `cientista-de-medicao` |
| Testes e regressão | `engenheiro-de-qa` |
| Site (`web/`) | `especialista-site` |
| Qualquer commit | `revisor-otimiza` |

Fora da pasta `Otimiza-novo` os agentes do projeto não carregam: chame um agente geral que leia o `.md` do agente.

## 4. Commit
Mensagem em português dizendo o que o cliente sente. Termina com a linha de coautoria. Nunca `--no-verify`.

## 5. Scripts de patch
Script Python em arquivo no scratchpad, rodado com `python -I`; `assert s.count(a) == 1` antes de trocar. Barra
invertida de continuação de linha do Rust: use `chr(92)` (heredoc com aspas a perde, e o teste
`nenhum_texto_tem_a_continuacao_de_linha_perdida` acusa).
