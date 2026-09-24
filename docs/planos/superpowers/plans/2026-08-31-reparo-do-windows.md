# Reparo do Windows — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dar ao Otimiza a capacidade de CONSERTAR o Windows (`sfc`, `DISM`, `chkdsk`) e de limpar o WinSxS, num executor de tarefa longa que informa andamento e aceita cancelamento.

**Architecture:** Um módulo novo (`tarefa_longa.rs`) roda um processo externo numa thread, lê a saída linha a linha e emite eventos Tauri. Um segundo módulo (`reparo.rs`) descreve as ferramentas e lê os resultados de fontes NÃO traduzidas (código de saída e `CBS.log`). Reparo fica fora do catálogo: ele não é reversível porque não muda ajuste nenhum.

**Tech Stack:** Rust 2021, Tauri 2, `std::process::Command` com `CREATE_NO_WINDOW`, `tokio::sync::Mutex` no estado do app, TypeScript puro no frontend.

**Spec:** `docs/superpowers/specs/2026-08-31-reparo-do-windows-design.md`

## Global Constraints

- **Código e comentários em português do Brasil.** Comentário explica POR QUE, não O QUE.
- **Todo commit é feito como EduardoxDev:** `git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" commit`
- **Nenhum texto de console traduzido é comparado.** A leitura sai de código de saída e de `%windir%\Logs\CBS\CBS.log`. Precedente: `readiness.rs` e o `powercfg`.
- **Todo comando externo passa por `shell.rs`** e roda com `CREATE_NO_WINDOW` (`0x0800_0000`). Sem isso, um console preto pisca na tela do cliente.
- **Módulo novo entra na lane de `.github/workflows/release.yml`**, ou a guarda `ci_coverage` reprova a publicação.
- **Verificar é livre; consertar exige licença.** As listas ficam em `commands.rs:1853` (`LIVRES`) e `commands.rs:1910` (`EXIGEM_LICENCA`), e as guardas `quem_so_le_nao_pede_licenca` e `as_duas_listas_cobrem_todos_os_comandos` cobram as duas.
- **Nada aqui entra em `catalog.rs`.** Reparo não é otimização.
- Rodar os testes com `cargo test --lib` a partir de `pc-optimizer/src-tauri`.

---

## Estrutura de arquivos

| Arquivo | Responsabilidade |
|---|---|
| `modules/windows/tarefa_longa.rs` | **Criar.** Roda um processo, transmite as linhas, cancela. Não sabe o que é `sfc`. |
| `modules/windows/reparo.rs` | **Criar.** Descreve as ferramentas e lê os resultados. Não sabe rodar processo. |
| `modules/windows/cbslog.rs` | **Criar.** Lê o `CBS.log`. Separado porque é a única parte com formato de terceiro para interpretar, e a que mais vai precisar de teste com saída gravada. |
| `modules/windows/mod.rs` | Modificar: três `pub mod` novos, em ordem alfabética |
| `commands.rs` | Modificar: comandos novos e as duas listas |
| `lib.rs` | Modificar: `generate_handler!` |
| `.github/workflows/release.yml:136` | Modificar: a lane de testes |

A divisão em três não é cerimônia: `tarefa_longa` é genérica e vai servir ao WinSxS e a tudo que demorar depois; `cbslog` é a única parte que interpreta formato alheio.

---

## Task 1: O executor de tarefa longa

**Files:**
- Create: `pc-optimizer/src-tauri/src/modules/windows/tarefa_longa.rs`
- Modify: `pc-optimizer/src-tauri/src/modules/windows/mod.rs`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `pub struct Andamento { pub linha: String, pub numero: usize }`
  - `pub enum Desfecho { Terminou { codigo: i32 }, Cancelada, NaoComecou { motivo: String } }`
  - `pub struct TarefaLonga` com `pub fn nova() -> Self`, `pub fn ocupada(&self) -> bool`, `pub fn cancelar(&self) -> bool`, e
    `pub fn rodar<F: FnMut(Andamento) + Send + 'static>(&self, programa: &str, args: &[&str], ao_progredir: F) -> Result<Desfecho, String>`

- [ ] **Step 1: Escrever o teste que falha**

Em `tarefa_longa.rs`, no fim:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transmite_cada_linha_e_devolve_o_codigo() {
        let tarefa = TarefaLonga::nova();
        let colhidas = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let dentro = colhidas.clone();

        let desfecho = tarefa
            .rodar("cmd", &["/c", "echo um&echo dois"], move |a| {
                dentro.lock().unwrap().push(a.linha);
            })
            .expect("a tarefa não chegou a rodar");

        let linhas = colhidas.lock().unwrap().clone();
        assert_eq!(linhas.len(), 2, "linhas transmitidas: {:?}", linhas);
        assert!(matches!(desfecho, Desfecho::Terminou { codigo: 0 }));
    }

    #[test]
    fn uma_de_cada_vez() {
        // Duas ferramentas de reparo ao mesmo tempo disputam os mesmos
        // arquivos, e o resultado é imprevisível para as duas.
        let tarefa = TarefaLonga::nova();
        assert!(!tarefa.ocupada(), "nasceu ocupada");
    }
}
```

- [ ] **Step 2: Rodar e confirmar que falha**

Rodar em `pc-optimizer/src-tauri`: `cargo test --lib modules::windows::tarefa_longa`
Esperado: FALHA — `cannot find struct TarefaLonga`.

- [ ] **Step 3: Implementar**

Conteúdo de `tarefa_longa.rs`:

```rust
// Tarefas que demoram minutos, e não milissegundos
//
// Todo comando do Otimiza responde em milissegundos, e a interface espera a
// resposta. Um `DISM` de vinte minutos nesse formato congela a janela.
//
// Este módulo não sabe o que é reparo. Ele roda um processo, entrega as linhas
// conforme elas saem, e aceita ser interrompido — e é isso que o torna útil
// também para a limpeza do WinSxS e para o que vier depois.

use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Sem isto, cada comando abre um console preto piscando na tela do cliente.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Uma linha de saída, com a posição dela.
///
/// O número existe para a tela poder dizer "parado há 4 minutos na mesma
/// linha" — que é diferente de "travado", e é a informação que impede o
/// cliente de desistir no meio do `DISM`.
#[derive(Debug, Clone)]
pub struct Andamento {
    pub linha: String,
    pub numero: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Desfecho {
    Terminou { codigo: i32 },
    Cancelada,
    NaoComecou { motivo: String },
}

pub struct TarefaLonga {
    /// Qual processo está rodando agora, para poder matá-lo.
    atual: Mutex<Option<u32>>,
    cancelar_pedido: Arc<AtomicBool>,
}

impl Default for TarefaLonga {
    fn default() -> Self {
        Self::nova()
    }
}

impl TarefaLonga {
    pub fn nova() -> Self {
        Self {
            atual: Mutex::new(None),
            cancelar_pedido: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn ocupada(&self) -> bool {
        self.atual.lock().map(|a| a.is_some()).unwrap_or(false)
    }

    /// Pede para a tarefa parar. Devolve `false` se não havia nada rodando.
    pub fn cancelar(&self) -> bool {
        let Ok(atual) = self.atual.lock() else {
            return false;
        };

        let Some(pid) = *atual else {
            return false;
        };

        self.cancelar_pedido.store(true, Ordering::SeqCst);

        // `taskkill /T` leva junto os processos filhos. O `DISM` cria um, e
        // matar só o pai deixaria o filho segurando os arquivos.
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .is_ok()
    }

    /// Roda o programa até o fim, chamando `ao_progredir` a cada linha.
    ///
    /// BLOQUEIA a thread que chamou. Quem chama é um comando do Tauri, que já
    /// roda fora da thread da interface.
    pub fn rodar<F>(
        &self,
        programa: &str,
        args: &[&str],
        mut ao_progredir: F,
    ) -> Result<Desfecho, String>
    where
        F: FnMut(Andamento) + Send + 'static,
    {
        if self.ocupada() {
            return Ok(Desfecho::NaoComecou {
                motivo: "Já existe uma tarefa em andamento.".into(),
            });
        }

        self.cancelar_pedido.store(false, Ordering::SeqCst);

        let mut filho = Command::new(programa)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("não consegui iniciar `{}`: {}", programa, e))?;

        if let Ok(mut atual) = self.atual.lock() {
            *atual = Some(filho.id());
        }

        // A saída é lida ENQUANTO o processo roda. Guardar para ler no fim
        // seria o mesmo que não ter andamento nenhum — e pior, encheria o cano
        // do sistema até o processo travar esperando alguém ler.
        if let Some(saida) = filho.stdout.take() {
            let mut numero = 0;
            for linha in BufReader::new(saida).lines().map_while(Result::ok) {
                numero += 1;
                ao_progredir(Andamento { linha, numero });
            }
        }

        let status = filho.wait().map_err(|e| format!("o processo sumiu: {}", e))?;

        if let Ok(mut atual) = self.atual.lock() {
            *atual = None;
        }

        if self.cancelar_pedido.load(Ordering::SeqCst) {
            return Ok(Desfecho::Cancelada);
        }

        Ok(Desfecho::Terminou {
            codigo: status.code().unwrap_or(-1),
        })
    }
}
```

E em `mod.rs`, junto dos outros `pub mod`, em ordem alfabética (depois de `pub mod tasks;`):

```rust
pub mod tarefa_longa;
```

- [ ] **Step 4: Rodar e confirmar que passa**

Rodar: `cargo test --lib modules::windows::tarefa_longa`
Esperado: PASSA, 2 testes.

- [ ] **Step 5: Comprometer**

```bash
git add pc-optimizer/src-tauri/src/modules/windows/tarefa_longa.rs pc-optimizer/src-tauri/src/modules/windows/mod.rs
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" commit -m "Executor de tarefa longa: minutos, e nao milissegundos"
```

---

## Task 2: Ler o CBS.log

**Files:**
- Create: `pc-optimizer/src-tauri/src/modules/windows/cbslog.rs`
- Modify: `pc-optimizer/src-tauri/src/modules/windows/mod.rs`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `pub enum ResultadoSfc { SemCorrupcao, Corrigiu { quantos: usize }, NaoConseguiu { quantos: usize }, NaoSei { motivo: String } }`
  - `pub fn interpretar(conteudo: &str) -> ResultadoSfc`
  - `pub fn caminho_do_log() -> std::path::PathBuf`

- [ ] **Step 1: Escrever o teste que falha**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Trecho no formato real: as marcas `[SR]` do CBS.log NÃO são traduzidas,
    /// mesmo num Windows em português. É por isso que a leitura vem daqui e
    /// não do texto do console.
    const SEM_CORRUPCAO: &str = "\
2026-08-31 12:00:01, Info CSI 00000001 [SR] Verifying 100 (0x00000064) components
2026-08-31 12:00:02, Info CSI 00000002 [SR] Verify complete
2026-08-31 12:00:03, Info CSI 00000003 [SR] Repairing 0 components";

    const CORRIGIU: &str = "\
2026-08-31 12:00:01, Info CSI 00000001 [SR] Cannot repair member file [l:20]'ntdll.dll'
2026-08-31 12:00:02, Info CSI 00000002 [SR] Repairing corrupted file \\??\\C:\\Windows\\ntdll.dll
2026-08-31 12:00:03, Info CSI 00000003 [SR] Repair complete";

    const NAO_CONSEGUIU: &str = "\
2026-08-31 12:00:01, Info CSI 00000001 [SR] Cannot repair member file [l:20]'ntdll.dll'
2026-08-31 12:00:02, Info CSI 00000002 [SR] Cannot repair member file [l:18]'user32.dll'";

    #[test]
    fn log_limpo_e_sem_corrupcao() {
        assert_eq!(interpretar(SEM_CORRUPCAO), ResultadoSfc::SemCorrupcao);
    }

    #[test]
    fn reparo_concluido_e_corrigiu() {
        assert_eq!(interpretar(CORRIGIU), ResultadoSfc::Corrigiu { quantos: 1 });
    }

    #[test]
    fn sem_reparo_concluido_e_nao_conseguiu() {
        assert_eq!(
            interpretar(NAO_CONSEGUIU),
            ResultadoSfc::NaoConseguiu { quantos: 2 }
        );
    }

    #[test]
    fn log_vazio_e_nao_sei_e_nunca_sem_corrupcao() {
        // "não consegui ler" virando "está tudo bem" é o mesmo defeito que o
        // vigia de pagamento evita ao separar "não sei" de "não pago".
        assert!(matches!(interpretar(""), ResultadoSfc::NaoSei { .. }));
    }

    #[test]
    fn nenhuma_frase_traduzida_e_comparada() {
        // A saída do console do `sfc` é traduzida; a do CBS.log não é. Comparar
        // texto em português quebraria em qualquer Windows de outro idioma.
        let fonte = include_str!("cbslog.rs");
        for proibida in ["não encontrou", "violação", "nenhuma viola"] {
            assert!(
                !fonte.contains(&format!("contains(\"{}", proibida)),
                "está comparando texto traduzido: {}",
                proibida
            );
        }
    }
}
```

- [ ] **Step 2: Rodar e confirmar que falha**

Rodar: `cargo test --lib modules::windows::cbslog`
Esperado: FALHA — `cannot find function interpretar`.

- [ ] **Step 3: Implementar**

```rust
// A leitura do resultado do `sfc`
//
// POR QUE NÃO LER A SAÍDA DO CONSOLE
//
// O `sfc` escreve "não encontrou nenhuma violação de integridade" — em
// português, num Windows em português. Comparar essa frase quebraria em
// qualquer outro idioma, e é exatamente o defeito que `readiness.rs` já
// documenta para o `powercfg`.
//
// As marcas `[SR]` do CBS.log não são traduzidas. Elas são o mesmo texto em
// qualquer instalação do Windows, e é por isso que a leitura vem daqui.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoSfc {
    SemCorrupcao,
    Corrigiu { quantos: usize },
    NaoConseguiu { quantos: usize },
    /// Log ausente, vazio ou ilegível.
    ///
    /// NUNCA vira `SemCorrupcao`. "Não consegui conferir" virando "está tudo
    /// bem" é o mesmo defeito que o vigia de pagamento evita ao separar "não
    /// sei" de "não pago".
    NaoSei { motivo: String },
}

pub fn caminho_do_log() -> PathBuf {
    let raiz = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    PathBuf::from(raiz).join("Logs").join("CBS").join("CBS.log")
}

pub fn interpretar(conteudo: &str) -> ResultadoSfc {
    let marcas: Vec<&str> = conteudo
        .lines()
        .filter(|l| l.contains("[SR]"))
        .collect();

    if marcas.is_empty() {
        return ResultadoSfc::NaoSei {
            motivo: "o registro do Windows não trouxe nenhuma verificação".into(),
        };
    }

    let nao_reparados = marcas
        .iter()
        .filter(|l| l.contains("Cannot repair member file"))
        .count();

    if nao_reparados == 0 {
        return ResultadoSfc::SemCorrupcao;
    }

    // "Repairing corrupted file" é o registro de que o conserto aconteceu. Sem
    // ele, o que ficou foi só a lista do que não deu para consertar.
    let reparados = marcas
        .iter()
        .filter(|l| l.contains("Repairing corrupted file"))
        .count();

    if reparados > 0 {
        ResultadoSfc::Corrigiu { quantos: reparados }
    } else {
        ResultadoSfc::NaoConseguiu {
            quantos: nao_reparados,
        }
    }
}
```

E em `mod.rs`, em ordem alfabética (depois de `pub mod catalog;`):

```rust
pub mod cbslog;
```

- [ ] **Step 4: Rodar e confirmar que passa**

Rodar: `cargo test --lib modules::windows::cbslog`
Esperado: PASSA, 5 testes.

- [ ] **Step 5: Comprometer**

```bash
git add pc-optimizer/src-tauri/src/modules/windows/cbslog.rs pc-optimizer/src-tauri/src/modules/windows/mod.rs
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" commit -m "Ler o resultado do sfc no CBS.log, e nao no texto traduzido"
```

---

## Task 3: As ferramentas de reparo, e as travas

**Files:**
- Create: `pc-optimizer/src-tauri/src/modules/windows/reparo.rs`
- Modify: `pc-optimizer/src-tauri/src/modules/windows/mod.rs`

**Interfaces:**
- Consumes: `cbslog::{ResultadoSfc, interpretar, caminho_do_log}`, `tarefa_longa::{TarefaLonga, Desfecho}`
- Produces:
  - `pub enum Ferramenta { VerificarArquivos, RepararImagem, VerificarDisco, ConsertarDisco, AnalisarWinSxS, LimparWinSxS { resetar_base: bool } }`
  - `pub struct Receita { pub programa: &'static str, pub args: Vec<String>, pub minutos_tipicos: (u32, u32), pub cancelar_e_seguro: bool, pub aviso: Option<&'static str> }`
  - `pub fn receita(f: &Ferramenta) -> Receita`
  - `pub fn consertar_disco_e_permitido(disco_saudavel: bool) -> bool`

- [ ] **Step 1: Escrever o teste que falha**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verificar_disco_roda_sem_reiniciar() {
        // O que se ensina por aí é `chkdsk /f`, que reinicia a máquina e prende
        // a pessoa numa tela azul por tempo indeterminado. `/scan` roda com o
        // Windows ligado e acha o problema sem consertar.
        let r = receita(&Ferramenta::VerificarDisco);
        assert!(r.args.iter().any(|a| a == "/scan"), "args: {:?}", r.args);
        assert!(!r.args.iter().any(|a| a == "/f"), "ofereceu /f na verificação");
        assert!(r.cancelar_e_seguro, "o /scan só lê; cancelar tem que ser seguro");
    }

    #[test]
    fn disco_reprovado_nao_recebe_consertar() {
        // Num disco morrendo, o chkdsk é justamente o que costuma matá-lo de
        // vez — e o health.rs já sabe reconhecer esse disco.
        assert!(!consertar_disco_e_permitido(false));
        assert!(consertar_disco_e_permitido(true));
    }

    #[test]
    fn resetar_base_muda_os_argumentos_e_avisa() {
        let sem = receita(&Ferramenta::LimparWinSxS { resetar_base: false });
        let com = receita(&Ferramenta::LimparWinSxS { resetar_base: true });

        assert!(!sem.args.iter().any(|a| a == "/ResetBase"));
        assert!(com.args.iter().any(|a| a == "/ResetBase"));

        // O cliente perde a capacidade de desinstalar atualizações. Isso não
        // pode ficar só na cabeça de quem escreveu a tela.
        assert!(com.aviso.is_some(), "/ResetBase saiu sem aviso");
        assert!(sem.aviso.is_none());
    }

    #[test]
    fn cancelar_o_dism_nao_e_de_graca() {
        // Interrompido no meio de uma escrita, o DISM pode deixar uma operação
        // pendente que só se resolve rodando de novo até o fim.
        assert!(!receita(&Ferramenta::RepararImagem).cancelar_e_seguro);
        assert!(receita(&Ferramenta::VerificarArquivos).cancelar_e_seguro);
    }

    #[test]
    fn o_dism_pede_saida_estavel() {
        // `/English` dá uma saída que não muda com o idioma do Windows.
        let r = receita(&Ferramenta::RepararImagem);
        assert!(r.args.iter().any(|a| a == "/English"), "args: {:?}", r.args);
    }
}
```

- [ ] **Step 2: Rodar e confirmar que falha**

Rodar: `cargo test --lib modules::windows::reparo`
Esperado: FALHA — `cannot find enum Ferramenta`.

- [ ] **Step 3: Implementar**

```rust
// As ferramentas de reparo do Windows
//
// O produto tinha 42 ajustes e nenhum reparo. Numa máquina com arquivo de
// sistema corrompido, nenhum dos 42 adianta — e é a explicação, sem supor
// nada, para "os comandos do terminal ajudaram mais que o Otimiza".
//
// REPARO NÃO É OTIMIZAÇÃO, E POR ISSO NÃO ESTÁ NO CATÁLOGO.
//
// Toda mudança do produto é reversível com o valor anterior guardado. O `sfc`
// não muda ajuste nenhum: devolve um arquivo corrompido ao original. Não há
// valor anterior a guardar, e desfazer significaria recorromper de propósito.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ferramenta {
    VerificarArquivos,
    RepararImagem,
    VerificarDisco,
    ConsertarDisco,
    AnalisarWinSxS,
    LimparWinSxS { resetar_base: bool },
}

pub struct Receita {
    pub programa: &'static str,
    pub args: Vec<String>,
    /// Para a tela poder dizer "de 10 a 30 minutos" antes de começar. É o que
    /// impede o cliente de desistir no meio.
    pub minutos_tipicos: (u32, u32),
    pub cancelar_e_seguro: bool,
    pub aviso: Option<&'static str>,
}

fn args(lista: &[&str]) -> Vec<String> {
    lista.iter().map(|s| s.to_string()).collect()
}

pub fn receita(f: &Ferramenta) -> Receita {
    match f {
        Ferramenta::VerificarArquivos => Receita {
            programa: "sfc",
            args: args(&["/scannow"]),
            minutos_tipicos: (5, 15),
            // Interrompido, ele para. Rodar de novo recomeça do zero.
            cancelar_e_seguro: true,
            aviso: None,
        },

        Ferramenta::RepararImagem => Receita {
            programa: "DISM",
            args: args(&[
                "/Online",
                "/Cleanup-Image",
                "/RestoreHealth",
                // Saída que não muda com o idioma do Windows.
                "/English",
            ]),
            minutos_tipicos: (10, 30),
            cancelar_e_seguro: false,
            aviso: Some(
                "Fica parado em 20% por vários minutos, e isso é normal. \
                 Precisa de internet: os arquivos bons vêm do Windows Update.",
            ),
        },

        // `/scan` roda com o Windows LIGADO, em NTFS. Acha sem consertar.
        Ferramenta::VerificarDisco => Receita {
            programa: "chkdsk",
            args: args(&["C:", "/scan"]),
            minutos_tipicos: (2, 20),
            cancelar_e_seguro: true,
            aviso: None,
        },

        Ferramenta::ConsertarDisco => Receita {
            programa: "chkdsk",
            args: args(&["C:", "/f"]),
            minutos_tipicos: (10, 60),
            // Não dá para cancelar: fica agendado para a inicialização. Quem
            // desmarca é o `chkntfs /x`, e a tela oferece isso enquanto a
            // máquina não reiniciou.
            cancelar_e_seguro: false,
            aviso: Some(
                "Exige reiniciar o computador. O conserto acontece antes de o \
                 Windows abrir, e não dá para usar a máquina durante ele.",
            ),
        },

        Ferramenta::AnalisarWinSxS => Receita {
            programa: "DISM",
            args: args(&[
                "/Online",
                "/Cleanup-Image",
                "/AnalyzeComponentStore",
                "/English",
            ]),
            minutos_tipicos: (1, 5),
            cancelar_e_seguro: true,
            aviso: None,
        },

        Ferramenta::LimparWinSxS { resetar_base } => {
            let mut lista = vec![
                "/Online".to_string(),
                "/Cleanup-Image".to_string(),
                "/StartComponentCleanup".to_string(),
                "/English".to_string(),
            ];

            if *resetar_base {
                lista.push("/ResetBase".to_string());
            }

            Receita {
                programa: "DISM",
                args: lista,
                minutos_tipicos: (5, 25),
                cancelar_e_seguro: false,
                aviso: if *resetar_base {
                    Some(
                        "Libera mais espaço, e em troca você perde a capacidade \
                         de desinstalar qualquer atualização já aplicada. Não dá \
                         para voltar atrás depois.",
                    )
                } else {
                    None
                },
            }
        }
    }
}

/// Se `chkdsk /f` pode ser oferecido.
///
/// Num disco em más condições, o `chkdsk` é justamente o que costuma matá-lo de
/// vez: ele reescreve estrutura em setores que já estão falhando. O `health.rs`
/// já sabe reconhecer esse disco, e esta é a trava que usa essa leitura.
pub fn consertar_disco_e_permitido(disco_saudavel: bool) -> bool {
    disco_saudavel
}
```

E em `mod.rs`, em ordem alfabética (depois de `pub mod readiness;`):

```rust
pub mod reparo;
```

- [ ] **Step 4: Rodar e confirmar que passa**

Rodar: `cargo test --lib modules::windows::reparo`
Esperado: PASSA, 5 testes.

- [ ] **Step 5: Comprometer**

```bash
git add pc-optimizer/src-tauri/src/modules/windows/reparo.rs pc-optimizer/src-tauri/src/modules/windows/mod.rs
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" commit -m "As ferramentas de reparo, com as travas do disco e do ResetBase"
```

---

## Task 4: Os comandos, e as duas listas de licença

**Files:**
- Modify: `pc-optimizer/src-tauri/src/commands.rs` (listas em `:1853` e `:1910`)
- Modify: `pc-optimizer/src-tauri/src/lib.rs` (`generate_handler!`)
- Modify: `.github/workflows/release.yml:136`

**Interfaces:**
- Consumes: `reparo::{Ferramenta, Receita, receita, consertar_disco_e_permitido}`, `cbslog`, `tarefa_longa`
- Produces: comandos `reparo_disponivel`, `reparo_ultimo_resultado`, `reparo_executar`, `reparo_cancelar`

- [ ] **Step 1: Escrever o teste que falha**

Acrescentar dentro do `mod tests` que já existe em `commands.rs`:

```rust
#[test]
fn verificar_e_livre_e_consertar_pede_licenca() {
    // A regra da casa: diagnóstico livre, correção paga. Se a verificação
    // passar a exigir licença, o cliente não consegue nem descobrir que o
    // problema dele existe — e é justamente esse achado que vende.
    assert!(LIVRES.contains(&"reparo_disponivel"));
    assert!(LIVRES.contains(&"reparo_ultimo_resultado"));
    assert!(EXIGEM_LICENCA.contains(&"reparo_executar"));

    // Cancelar é livre DE PROPÓSITO, pelo mesmo motivo que `revert` é: uma
    // licença vencida no meio de um DISM não pode prender a pessoa nele.
    assert!(LIVRES.contains(&"reparo_cancelar"));
}
```

- [ ] **Step 2: Rodar e confirmar que falha**

Rodar: `cargo test --lib commands::tests::verificar_e_livre_e_consertar_pede_licenca`
Esperado: FALHA — o `assert!` reprova, as listas não têm os nomes.

- [ ] **Step 3: Implementar**

Em `commands.rs`, acrescentar às listas (`LIVRES` em `:1853`, `EXIGEM_LICENCA` em `:1910`):

```rust
// em LIVRES
"reparo_disponivel",
"reparo_ultimo_resultado",
"reparo_cancelar",

// em EXIGEM_LICENCA
"reparo_executar",
```

E os quatro comandos, junto dos outros:

```rust
/// O que dá para oferecer nesta máquina.
///
/// Fica em `LIVRES`: é leitura, e é o diagnóstico que mostra ao cliente que o
/// problema dele existe antes de qualquer cobrança.
#[tauri::command]
pub fn reparo_disponivel(disco_saudavel: bool) -> Vec<String> {
    let mut lista = vec![
        "VerificarArquivos".to_string(),
        "RepararImagem".to_string(),
        "VerificarDisco".to_string(),
        "AnalisarWinSxS".to_string(),
        "LimparWinSxS".to_string(),
    ];

    if crate::modules::windows::reparo::consertar_disco_e_permitido(disco_saudavel) {
        lista.push("ConsertarDisco".to_string());
    }

    lista
}

/// O resultado da última verificação de arquivos de sistema.
///
/// Fica em `LIVRES`: é leitura de um registro que o Windows já escreveu.
#[tauri::command]
pub fn reparo_ultimo_resultado() -> String {
    use crate::modules::windows::cbslog::{self, ResultadoSfc};

    let conteudo = std::fs::read_to_string(cbslog::caminho_do_log()).unwrap_or_default();

    match cbslog::interpretar(&conteudo) {
        // Este é o resultado mais comum, e é um resultado BOM. A tela diz isso
        // com todas as letras, sem inventar benefício — mesma regra do
        // `prova.rs`, que se recusa a chamar ruído de ganho.
        ResultadoSfc::SemCorrupcao => "Nenhuma corrupção encontrada.".into(),
        ResultadoSfc::Corrigiu { quantos } => {
            format!("Corrigiu {} arquivo(s) corrompido(s).", quantos)
        }
        ResultadoSfc::NaoConseguiu { quantos } => format!(
            "Encontrou {} arquivo(s) corrompido(s) e não conseguiu corrigir. \
             O próximo passo é reparar a imagem do Windows.",
            quantos
        ),
        ResultadoSfc::NaoSei { motivo } => format!("Não consegui conferir: {}.", motivo),
    }
}

/// Interrompe a tarefa em andamento.
///
/// Fica em `LIVRES` DE PROPÓSITO, pelo mesmo motivo que `revert` fica: uma
/// licença que vence no meio de um `DISM` de vinte minutos não pode deixar o
/// cliente preso nele.
#[tauri::command]
pub fn reparo_cancelar(state: tauri::State<'_, AppState>) -> bool {
    state.reparo.cancelar()
}
```

O `reparo_executar` recebe o nome da ferramenta e emite os eventos. Acrescentar
`pub reparo: crate::modules::windows::tarefa_longa::TarefaLonga` ao `AppState`
(em `commands.rs:76`, junto de `monitor` e `changes`) — **sem `Mutex`**, porque
`TarefaLonga` já guarda o próprio estado internamente.

```rust
/// Roda uma ferramenta de reparo, transmitindo o andamento.
///
/// Exige licença: é correção, como todas as outras.
#[tauri::command]
pub fn reparo_executar(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    ferramenta: String,
    resetar_base: bool,
    disco_saudavel: bool,
) -> Result<String, String> {
    use crate::modules::windows::reparo::{self, Ferramenta};
    use crate::modules::windows::tarefa_longa::Desfecho;
    use tauri::Emitter;

    licenca::exigir()?;

    let escolhida = match ferramenta.as_str() {
        "VerificarArquivos" => Ferramenta::VerificarArquivos,
        "RepararImagem" => Ferramenta::RepararImagem,
        "VerificarDisco" => Ferramenta::VerificarDisco,
        "ConsertarDisco" => {
            // A trava do disco é conferida AQUI TAMBÉM, e não só na tela: a
            // tela pode ser contornada, esta chamada não.
            if !reparo::consertar_disco_e_permitido(disco_saudavel) {
                return Err(
                    "O disco desta máquina não está em condições para isso. \
                     Consertar a estrutura num disco que já falha costuma \
                     terminar de estragá-lo."
                        .into(),
                );
            }
            Ferramenta::ConsertarDisco
        }
        "AnalisarWinSxS" => Ferramenta::AnalisarWinSxS,
        "LimparWinSxS" => Ferramenta::LimparWinSxS { resetar_base },
        outra => return Err(format!("não conheço a ferramenta `{}`", outra)),
    };

    let r = reparo::receita(&escolhida);
    let args: Vec<&str> = r.args.iter().map(|s| s.as_str()).collect();

    let desfecho = state.reparo.rodar(r.programa, &args, move |a| {
        let _ = app.emit("reparo-andamento", a.linha);
    })?;

    Ok(match desfecho {
        Desfecho::Terminou { codigo: 0 } => "Terminou.".into(),
        Desfecho::Terminou { codigo } => format!("Terminou com o código {}.", codigo),
        Desfecho::Cancelada => "Interrompida por você.".into(),
        Desfecho::NaoComecou { motivo } => motivo,
    })
}
```

Em `lib.rs`, no `generate_handler!`, junto de `commands::placa_de_video` (linha 88):

```rust
commands::reparo_disponivel,
commands::reparo_ultimo_resultado,
commands::reparo_executar,
commands::reparo_cancelar,
```

Em `.github/workflows/release.yml:136`, acrescentar os três módulos ao fim da linha:

```yaml
run: "cargo test --lib -- ci_coverage:: core:: modules::changelog:: modules::preferences:: modules::prova:: modules::safety:: modules::windows::cbslog:: modules::windows::reparo:: modules::windows::tarefa_longa::"
```

- [ ] **Step 4: Rodar a suíte inteira**

Rodar: `cargo test --lib`
Esperado: PASSA, incluindo `quem_so_le_nao_pede_licenca`, `as_duas_listas_cobrem_todos_os_comandos` e `ci_coverage`.

- [ ] **Step 5: Comprometer**

```bash
git add pc-optimizer/src-tauri/src/commands.rs pc-optimizer/src-tauri/src/lib.rs .github/workflows/release.yml
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" commit -m "Comandos de reparo: verificar e livre, consertar pede licenca"
```

---

## Task 5: A aba de Reparo

**Files:**
- Modify: `pc-optimizer/index.html`
- Modify: `pc-optimizer/src/main.ts`
- Modify: `pc-optimizer/src/styles.css`

**Interfaces:**
- Consumes: os quatro comandos da Task 4 e o evento `reparo-andamento`.
- Produces: nada para tarefas seguintes.

- [ ] **Step 1: Acrescentar o painel ao `index.html`**

Junto dos outros `<section class="tab-panel">`, seguindo o padrão do painel de monitores:

```html
<!--
  A ABA DE REPARO.

  Ela existe separada do catálogo porque aqui NÃO EXISTE DESFAZER — e não por
  descuido: o `sfc` não muda ajuste nenhum, ele devolve um arquivo corrompido
  ao original. Não há valor anterior a guardar, e desfazer significaria
  recorromper de propósito.

  A tela precisa dizer isso, e dizer antes de o cliente clicar.
-->
<section class="tab-panel" id="tab-reparo" role="tabpanel" aria-labelledby="tabbtn-reparo" hidden>
  <section class="panel">
    <div class="panel-head">
      <h2>Reparo do Windows</h2>
      <span class="panel-tag" id="reparo-tag">—</span>
    </div>
    <p class="lead" id="reparo-resultado">Lendo o registro do Windows…</p>
    <p class="hint">
      Aqui não existe desfazer, e não é descuido: estas ferramentas não mudam
      ajuste nenhum — elas devolvem arquivos danificados ao original.
    </p>
    <div class="reparo-lista" id="reparo-lista"></div>
    <pre class="reparo-saida" id="reparo-saida" hidden></pre>
    <button class="btn" id="reparo-cancelar" hidden>Interromper</button>
  </section>
</section>
```

E o botão da barra lateral, copiando o `use href` do ícone como os outros fazem.

- [ ] **Step 2: Ligar no `main.ts`**

```ts
/* -------------------------------------------------- a aba de reparo */

/**
 * O andamento vem por evento, e não como retorno da chamada.
 *
 * Um `DISM` leva de dez a trinta minutos. Esperar o retorno para só então
 * mostrar alguma coisa é o mesmo que não ter andamento — e é no minuto oito,
 * parado em 20%, que o cliente conclui que travou e desliga a máquina.
 */
async function carregarReparo() {
  const saida = element("reparo-saida");
  const cancelar = element("reparo-cancelar") as HTMLButtonElement;

  await listen<string>("reparo-andamento", (evento) => {
    saida.hidden = false;
    saida.textContent = `${saida.textContent ?? ""}${evento.payload}\n`;
    saida.scrollTop = saida.scrollHeight;
  });

  cancelar.addEventListener("click", () => {
    void invoke("reparo_cancelar");
  });

  text("reparo-resultado", await invoke<string>("reparo_ultimo_resultado"));
}
```

E chamar `void carregarReparo();` junto de `void carregarMonitores();`.

- [ ] **Step 3: Conferir que compila**

Rodar em `pc-optimizer`: `npx tsc --noEmit`
Esperado: sem erro.

- [ ] **Step 4: Rodar o produto e conferir na tela**

Rodar: `npm run tauri dev`
Conferir: a aba aparece, o resultado da última verificação carrega, e o aviso
de "aqui não existe desfazer" está visível **antes** de qualquer botão.

- [ ] **Step 5: Comprometer**

```bash
git add pc-optimizer/index.html pc-optimizer/src/main.ts pc-optimizer/src/styles.css
git -c user.name="EduardoxDev" -c user.email="eduardo.wankax@gmail.com" commit -m "A aba de reparo, com o aviso de que aqui nao existe desfazer"
```

---

## Revisão do plano contra a especificação

| Requisito da especificação | Onde é atendido |
|---|---|
| Executor com andamento e cancelamento | Task 1 |
| Uma tarefa por vez | Task 1, teste `uma_de_cada_vez` |
| `sfc` lido sem texto traduzido | Task 2, teste `nenhuma_frase_traduzida_e_comparada` |
| "Não sei" nunca vira "sem corrupção" | Task 2, teste `log_vazio_e_nao_sei_e_nunca_sem_corrupcao` |
| `DISM` com `/English` e aviso do 20% | Task 3, testes `o_dism_pede_saida_estavel` e `cancelar_o_dism_nao_e_de_graca` |
| `chkdsk /scan` sem reiniciar | Task 3, teste `verificar_disco_roda_sem_reiniciar` |
| `/f` travado em disco reprovado | Task 3 e Task 4 (a trava é conferida nos dois lugares) |
| `/ResetBase` desligado e avisado | Task 3, teste `resetar_base_muda_os_argumentos_e_avisa` |
| Verificar livre, consertar com licença | Task 4, teste `verificar_e_livre_e_consertar_pede_licenca` |
| "Nenhuma corrupção" dito com todas as letras | Task 4, em `reparo_ultimo_resultado` |
| Módulos novos na lane da CI | Task 4 |
| Nada em `catalog.rs` | Nenhuma tarefa toca nele |

**Pendência conhecida:** o encadeamento automático "`sfc` falhou → rodar `DISM`
→ `sfc` de novo" está descrito na especificação e **não** tem tarefa neste
plano. Ele depende da aba existir e do resultado ser lido em tempo real, então
entra depois da Task 5, num plano curto próprio. Registrado aqui para não
sumir.
