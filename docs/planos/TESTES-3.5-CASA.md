# 3.5 — roteiro de testes no PC de casa

A seção 11.1 do `PLANO-3.5.md` em passos: o comando, o que olhar e onde anotar. Tudo que não depende de jogo já
rodou no notebook (seção 11.2). Comandos no PowerShell, a partir de `pc-optimizer/src-tauri`, salvo indicação. Anote
cada resultado na coluna "deu" e, no fim, passe os números para a 11.2 do plano.

## 0. Preparar

1. `git pull` na `main` (com a `feature/3.5-medidas` já mesclada).
2. `cargo test --lib` — tem de dar 0 falhas antes de tudo.
3. Abrir o app de desenvolvimento com o jogo de teste valendo como jogo:
   `$env:OTIMIZA_JOGO_DE_TESTE = "jogo_de_teste"; npm --prefix .. run tauri dev`
   e, noutro terminal, `cargo run --example jogo_de_teste`.
4. O REA aberto, para as chaves de configuração (item 4).

## 1. Escala ao vivo

| # | Passo | O que tem de acontecer | deu |
|---|---|---|---|
| 1.1 | `cargo test --lib -- --ignored escala_na_placa cursor_na_placa custo_da_escala_na_placa --nocapture --test-threads=1` | 3 ok; anote o custo em ms na placa dedicada (na Intel UHD foi 2,4–2,7 ms) | |
| 1.2 | Frame Boost → Só escala → Qualidade, com o jogo de teste | a janela do jogo encolhe, a imagem enche o monitor, o cursor anda certo e não sai da janela | |
| 1.3 | Desligar pelo botão | a janela volta ao tamanho de antes, o mouse fica livre e o cursor normal | |
| 1.4 | Ligar de novo e **matar o Otimiza** (`Stop-Process -Name pc-optimizer -Force`; no instalado, `-Name Otimiza`) | anote: o cursor do sistema voltou? o mouse ficou preso? Ao reabrir o Otimiza, a janela volta e o mouse solta? | |
| 1.5 | Ligar e sair pela bandeja (Sair) | janela, mouse e cursor devolvidos antes de fechar (correção 3533350) | |
| 1.6 | Com dois monitores, o jogo no secundário | a escala usa o Hz e a área do monitor do jogo; o mouse não fica preso num monitor só depois de desligar | |
| 1.7 | Três jogos reais (ex.: FiveM, CS2 em janela sem bordas, um DX12) | liga, o FPS sobe, o cursor funciona; anote o FPS antes e depois | |

## 2. Escala pelo driver

| # | Passo | O que tem de acontecer | deu |
|---|---|---|---|
| 2.1 | NVIDIA: `cargo test --lib os_numeros_da_tabela_batem_com_o_que_o_driver_chama -- --nocapture` | passa afirmando os nomes (sem NVIDIA ele só pula) | |
| 2.2 | Boost → Driver da placa → Escala do driver (NIS), jogo em tela cheia numa resolução menor | o driver amplia com nitidez; aparece "NVIDIA Quality upscaling" no Painel de Controle | |
| 2.3 | NIS ligado, jogo na resolução **nativa** | anote o FPS com e sem: o NIS só com nitidez custa quadro? | |
| 2.4 | Desfazer | o Painel de Controle volta ao que era | |
| 2.5 | Radeon (se tiver): o mesmo com o RSR | anote se o Adrenalin mudou mais alguma coisa junto (escala pela GPU, escala inteira, Radeon Boost) e se o desfazer devolve tudo | |

## 3. Turbo

| # | Passo | O que tem de acontecer | deu |
|---|---|---|---|
| 3.1 | Um jogo preso na **placa**: Turbo → Mais FPS → Medir e ligar | receita com escala (e geração, se faltar para o monitor); FPS do jogo sobe | |
| 3.2 | Provar (~5 min) | 8 rodadas; anote o desfecho e o "sem → com" com a margem | |
| 3.3 | O mesmo jogo, Menor atraso | receita sem geração | |
| 3.4 | Um jogo preso no **processador** | sem escala; energia de jogo ligada; prova | |
| 3.5 | Desligar o Turbo, fechar e abrir o Otimiza, conferir a energia de jogo | se foi o Turbo que ligou, "Desligar Turbo" depois de reabrir a devolve (correção 3533350) | |
| 3.6 | Se alguma prova der "piorou": Medir de novo | recusa a mesma receita e a reprovação continua (correção 3533350) | |

## 4. Configuração do jogo

| # | Passo | O que tem de acontecer | deu |
|---|---|---|---|
| 4.1 | CS2 e Apex: conferir com o REA que as chaves de `configmotor.rs` existem no arquivo de verdade | as chaves batem; anote as que não existem | |
| 4.2 | Aplicar com o jogo fechado, abrir o jogo | o jogo aceita, a opção aparece mudada no menu | |
| 4.3 | Medir o ganho nos três (Fortnite, CS2, Apex) | anote o FPS antes e depois | |
| 4.4 | Desfazer | o arquivo volta byte a byte (compare o hash antes e depois) | |

## 5. Otimizar agora (Completo, 17 ajustes) e plano de energia num desktop

| # | Passo | O que tem de acontecer | deu |
|---|---|---|---|
| 5.1 | Otimizar agora no Completo, reiniciar | o resultado diz quantos valem só depois de reiniciar; nada quebrou (Bluetooth, impressora, notificações, Store) | |
| 5.2 | Lab: A-B-A-B no mesmo trecho de um jogo | anote o ganho e a margem (metas da seção 2.6) | |
| 5.3 | `cargo test --lib planoenergia -- --ignored --nocapture` como administrador | o desktop entra nas regras de desktop | |
| 5.4 | Restaurar tudo | volta tudo; o Windows fica como antes | |

## 6. Ritual da versão

1. Subir para 3.5.0 nas quatro declarações (`pc-optimizer/package.json`, `src-tauri/Cargo.toml`,
   `src-tauri/tauri.conf.json` e `web/src/lib/site.ts`; o `package-lock.json` e o `Cargo.lock` acompanham).
2. Rodar o `verificador-de-publicacao` na ref da tag.
3. Só com "pode publicar": `git tag v3.5.0` e enviar.
