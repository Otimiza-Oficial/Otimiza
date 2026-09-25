// Tarefas de minutos (DISM, limpeza do WinSxS): roda um processo, entrega as linhas conforme saem e aceita ser
// interrompido, sem congelar a janela. Não sabe o que é reparo.

use serde::Serialize;
use std::io::{BufReader, Read};
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A origem viaja com a linha: a razão de falha do DISM (no stderr) chegava embaralhada nas porcentagens, e
/// destacá-la por palavra-chave seria a tela decidindo por prosa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origem {
    Saida,
    Erro,
}

#[derive(Debug, Clone, Serialize)]
pub struct Andamento {
    pub linha: String,
    pub origem: Origem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Desfecho {
    Terminou { codigo: i32 },
    Cancelada,
    NaoComecou { motivo: String },
}

/// A reserva acontece ANTES do processo existir (`pid` vazio): é ela, e não o PID, que recusa a segunda chamada.
struct Estado {
    pid: Option<u32>,
}

/// Devolve a reserva em qualquer saída (retorno, `?`, pânico): uma reserva que sobrevive desliga o reparo pelo
/// resto da sessão.
struct Reserva<'a> {
    dono: &'a TarefaLonga,
}

impl Drop for Reserva<'_> {
    fn drop(&mut self) {
        // Limpa mesmo com veneno: o estado protegido acabou de ser zerado, e recusar para sempre trocaria um defeito
        // por outro.
        match self.dono.atual.lock() {
            Ok(mut atual) => *atual = None,
            Err(envenenada) => {
                *envenenada.into_inner() = None;
                self.dono.atual.clear_poison();
            }
        }
    }
}

/// Em BYTES, nunca UTF-8 estrito: `lines()` + `map_while` ENCERRAVA no primeiro `ç` (o `chkdsk` escreve CP-850,
/// o `sfc` UTF-16), ninguém drenava o cano e o `wait()` nunca voltava (ver `shell.rs`). Quebra em `\r` E `\n`:
/// `sfc` e DISM redesenham a porcentagem com retorno de carro.
fn drenar<L, F>(saida: L, origem: Origem, ao_progredir: &Mutex<F>)
where
    L: Read,
    F: FnMut(Andamento),
{
    let mut leitor = BufReader::new(saida);
    let mut pendente: Vec<u8> = Vec::new();
    let mut bloco = [0u8; 4096];

    let entregar = |pedaco: &[u8]| {
        let linha = decodificar(pedaco);

        // Todo "CR LF" gera um pedaço vazio entre os dois.
        if linha.is_empty() {
            return;
        }
        if let Ok(mut callback) = ao_progredir.lock() {
            callback(Andamento { linha, origem });
        }
    };

    loop {
        let lidos = match leitor.read(&mut bloco) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            // Erro de cano, não de conteúdo: insistir seria laço infinito.
            Err(_) => break,
        };

        pendente.extend_from_slice(&bloco[..lidos]);

        let mut inicio = 0;
        for posicao in 0..pendente.len() {
            if pendente[posicao] == b'\n' || pendente[posicao] == b'\r' {
                entregar(&pendente[inicio..posicao]);
                inicio = posicao + 1;
            }
        }
        pendente.drain(..inicio);
    }

    // Costuma ser a última linha do DISM, a que diz se deu certo.
    entregar(&pendente);
}

/// Os NUL vêm do UTF-16 do `sfc` ("A" = `41 00`) e sobreviveriam ao `from_utf8_lossy`. Não decodifica UTF-16
/// completo e não precisa: o veredito do `sfc` vem do CBS.log, e esta saída é sinal de vida.
fn decodificar(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|c| *c != '\0')
        .collect::<String>()
        .trim_end()
        .to_string()
}

pub struct TarefaLonga {
    atual: Mutex<Option<Estado>>,
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

    pub fn cancelar(&self) -> bool {
        let Ok(atual) = self.atual.lock() else {
            return false;
        };

        let Some(Estado { pid: Some(pid) }) = *atual else {
            return false;
        };

        // `/T` leva os filhos (o DISM cria um). O retorno é o CÓDIGO DE SAÍDA: "conseguiu nascer" era verdadeiro também
        // com "Acesso negado", e o DISM terminava com sucesso sob o rótulo `Cancelada`.
        let morreu = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|saida| saida.status.success())
            .unwrap_or(false);

        if morreu {
            self.cancelar_pedido.store(true, Ordering::SeqCst);
        }

        morreu
    }

    /// BLOQUEIA: quem chama é `#[tauri::command(async)]`, fora da thread da interface.
    pub fn rodar<F>(
        &self,
        programa: &str,
        args: &[&str],
        ao_progredir: F,
    ) -> Result<Desfecho, String>
    where
        F: FnMut(Andamento) + Send + 'static,
    {
        // Verificação e reserva sob o MESMO lock: em dois passos, duas chamadas passariam juntas.
        {
            let Ok(mut atual) = self.atual.lock() else {
                return Err("o estado da tarefa está corrompido".to_string());
            };

            if atual.is_some() {
                return Ok(Desfecho::NaoComecou {
                    motivo: "Já existe uma tarefa em andamento.".into(),
                });
            }

            *atual = Some(Estado { pid: None });
        }

        // A reserva é devolvida por saída de escopo: o `?` do `wait()` escapava das atribuições à mão e prendia o
        // executor pelo resto da sessão.
        let _reserva = Reserva { dono: self };

        self.cancelar_pedido.store(false, Ordering::SeqCst);

        let mut filho = Command::new(programa)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("não consegui iniciar `{}`: {}", programa, e))?;

        if let Ok(mut atual) = self.atual.lock() {
            *atual = Some(Estado {
                pid: Some(filho.id()),
            });
        }

        // O stderr precisa de leitor: o cano do Windows tem buffer pequeno e um DISM trava esperando dreno. E é nele
        // que mora o motivo da falha.
        let ao_progredir = Arc::new(Mutex::new(ao_progredir));

        let leitor_stderr = filho.stderr.take().map(|saida| {
            let callback = ao_progredir.clone();
            thread::spawn(move || drenar(saida, Origem::Erro, &callback))
        });

        // Lida ENQUANTO roda: no fim, o cano encheria e travaria o processo.
        if let Some(saida) = filho.stdout.take() {
            drenar(saida, Origem::Saida, &ao_progredir);
        }

        let status = filho.wait().map_err(|e| format!("o processo sumiu: {}", e))?;

        // Sem o `join`, a última linha de stderr podia não chegar antes do desfecho.
        if let Some(leitor_stderr) = leitor_stderr {
            let _ = leitor_stderr.join();
        }

        if self.cancelar_pedido.load(Ordering::SeqCst) {
            return Ok(Desfecho::Cancelada);
        }

        Ok(Desfecho::Terminou {
            codigo: status.code().unwrap_or(-1),
        })
    }
}

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
    fn a_origem_da_linha_e_a_do_cano_de_onde_ela_veio() {
        let tarefa = TarefaLonga::nova();
        let colhidas = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let dentro = colhidas.clone();

        tarefa
            .rodar(
                "cmd",
                &["/c", "echo do_stdout&&echo do_stderr 1>&2"],
                move |a| dentro.lock().unwrap().push(a),
            )
            .expect("a tarefa não chegou a rodar");

        let linhas = colhidas.lock().unwrap().clone();
        let da_saida = linhas
            .iter()
            .find(|a| a.linha == "do_stdout")
            .expect("a linha do stdout não chegou");
        let do_erro = linhas
            .iter()
            .find(|a| a.linha == "do_stderr")
            .expect("a linha do stderr não chegou");

        assert_eq!(da_saida.origem, Origem::Saida);
        assert_eq!(do_erro.origem, Origem::Erro);
    }

    #[test]
    fn uma_de_cada_vez() {
        // Prende o comportamento real (`rodar` recusando a segunda chamada), e não um getter que existia só para o teste.
        let tarefa = Arc::new(TarefaLonga::nova());
        let dentro = tarefa.clone();
        let (comecou_envia, comecou_recebe) = std::sync::mpsc::channel();

        let primeira = std::thread::spawn(move || {
            dentro.rodar(
                "cmd",
                // O `ping` segura o processo vivo até a segunda chamada.
                &["/c", "echo comecei&ping -n 3 127.0.0.1 >nul"],
                move |_| {
                    let _ = comecou_envia.send(());
                },
            )
        });

        comecou_recebe
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("a primeira tarefa não chegou a começar");

        let segunda = tarefa.rodar("cmd", &["/c", "echo nunca deveria rodar"], |_| {});
        assert!(
            matches!(segunda, Ok(Desfecho::NaoComecou { .. })),
            "a segunda chamada devia ser recusada enquanto a primeira roda: {:?}",
            segunda
        );

        let resultado_primeira = primeira.join().expect("a primeira thread entrou em pânico");
        assert!(
            matches!(resultado_primeira, Ok(Desfecho::Terminou { .. })),
            "a primeira tarefa não terminou normalmente: {:?}",
            resultado_primeira
        );
    }

    /// Prazo em vez de travar o CI se o stderr não for drenado. Folgado de propósito: sozinho leva ~15 s, e 20 s
    /// reprovava por disputa de CPU.
    #[test]
    fn stderr_nao_trava_a_tarefa() {
        let tarefa = std::sync::Arc::new(TarefaLonga::nova());
        let dentro = tarefa.clone();
        let (envia, recebe) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            // Mais que o buffer do cano (~4 KB).
            let resultado = dentro.rodar(
                "cmd",
                &["/c", "for /l %i in (1,1,5000) do @echo linha%i 1>&2"],
                |_| {},
            );
            let _ = envia.send(resultado);
        });

        let resultado = recebe
            .recv_timeout(std::time::Duration::from_secs(90))
            .expect("a tarefa travou — o stderr não está sendo drenado");

        assert!(matches!(resultado, Ok(Desfecho::Terminou { codigo: 0 })));
    }

    fn drenado(bytes: &[u8]) -> Vec<String> {
        let colhidas = Arc::new(Mutex::new(Vec::new()));
        let dentro = colhidas.clone();
        let callback = Mutex::new(move |a: Andamento| {
            dentro.lock().unwrap().push(a.linha);
        });

        drenar(std::io::Cursor::new(bytes.to_vec()), Origem::Saida, &callback);

        let saida = colhidas.lock().unwrap().clone();
        saida
    }

    #[test]
    fn byte_invalido_nao_interrompe_a_drenagem() {
        let bytes = b"comecou\nservi\xE7o quebrado\nterminou\n";
        let linhas = drenado(bytes);

        assert_eq!(linhas.len(), 3, "truncou na sequencia invalida: {:?}", linhas);
        assert_eq!(linhas[0], "comecou");
        assert_eq!(
            linhas[2], "terminou",
            "a ultima linha nao chegou: {:?}",
            linhas
        );
    }

    #[test]
    fn retorno_de_carro_tambem_quebra_linha() {
        let linhas = drenado(b"20%\r40%\r100%\r\nPronto\n");

        assert_eq!(linhas, vec!["20%", "40%", "100%", "Pronto"]);
    }

    #[test]
    fn nul_do_utf16_nao_vira_buraco_no_texto() {
        let linhas = drenado(b"o\0k\0\n\0");
        assert_eq!(linhas, vec!["ok"]);
    }

    #[test]
    fn saida_sem_quebra_no_fim_ainda_e_entregue() {
        assert_eq!(
            drenado(b"A operacao foi concluida"),
            vec!["A operacao foi concluida"]
        );
    }

    /// De ponta a ponta: se a drenagem parar nos bytes inválidos, o `cmd` trava e o prazo estoura.
    #[test]
    fn processo_com_saida_invalida_termina_e_nao_trunca() {
        let dir = std::env::temp_dir().join("otimiza_drenagem");
        let _ = std::fs::create_dir_all(&dir);
        let arquivo = dir.join(format!("saida_{}.txt", std::process::id()));

        let mut conteudo: Vec<u8> = Vec::new();
        conteudo.extend_from_slice(b"primeira\n");
        conteudo.extend_from_slice(&[0xE7, 0xE3, 0xF5]); /* "cao" em CP-850 */
        conteudo.extend_from_slice(b"\n");
        for i in 0..5000 {
            conteudo.extend_from_slice(format!("linha{}\n", i).as_bytes());
        }
        conteudo.extend_from_slice(b"ultima\n");
        std::fs::write(&arquivo, &conteudo).expect("nao consegui escrever o arquivo de teste");

        let tarefa = Arc::new(TarefaLonga::nova());
        let dentro = tarefa.clone();
        let colhidas = Arc::new(Mutex::new(Vec::new()));
        let colhidas_dentro = colhidas.clone();
        let caminho = arquivo.to_string_lossy().to_string();
        let (envia, recebe) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let resultado = dentro.rodar("cmd", &["/c", "type", &caminho], move |a| {
                colhidas_dentro.lock().unwrap().push(a.linha);
            });
            let _ = envia.send(resultado);
        });

        let resultado = recebe
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("a tarefa travou — a drenagem parou no byte invalido");

        let _ = std::fs::remove_file(&arquivo);

        assert!(
            matches!(resultado, Ok(Desfecho::Terminou { codigo: 0 })),
            "desfecho: {:?}",
            resultado
        );

        let linhas = colhidas.lock().unwrap().clone();
        assert!(
            linhas.iter().any(|l| l == "ultima"),
            "truncou antes do fim: {:?} linhas, ultima {:?}",
            linhas.len(),
            linhas.last()
        );
    }

    #[test]
    fn a_reserva_volta_mesmo_quando_o_programa_nao_existe() {
        let tarefa = TarefaLonga::nova();
        let erro = tarefa.rodar("programa_que_nao_existe_no_windows", &[], |_| {});
        assert!(erro.is_err(), "esperava falha ao iniciar: {:?}", erro);

        // A prova é uma segunda chamada CONSEGUIR rodar, não ler um campo interno.
        let depois = tarefa.rodar("cmd", &["/c", "echo ok"], |_| {});
        assert!(
            matches!(depois, Ok(Desfecho::Terminou { codigo: 0 })),
            "a reserva ficou presa depois do erro: {:?}",
            depois
        );
    }

    #[test]
    fn panico_no_callback_devolve_a_reserva_e_limpa_o_veneno() {
        let tarefa = Arc::new(TarefaLonga::nova());
        let dentro = tarefa.clone();

        // Pânico esperado: silencia o relator para não sujar a saída.
        let relator = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let panico = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = dentro.rodar("cmd", &["/c", "echo estoura"], |_| {
                panic!("panico de teste dentro do callback");
            });
        }));
        std::panic::set_hook(relator);

        assert!(panico.is_err(), "o panico de teste nao aconteceu");
        let depois = tarefa.rodar("cmd", &["/c", "echo depois"], |_| {});
        assert!(
            matches!(depois, Ok(Desfecho::Terminou { codigo: 0 })),
            "o executor nao se recuperou: {:?}",
            depois
        );
    }

    #[test]
    fn cancelar_sem_nada_rodando_nao_levanta_a_bandeira() {
        let tarefa = TarefaLonga::nova();
        assert!(!tarefa.cancelar(), "disse que cancelou sem nada rodando");

        let desfecho = tarefa.rodar("cmd", &["/c", "echo ok"], |_| {});
        assert!(
            matches!(desfecho, Ok(Desfecho::Terminou { codigo: 0 })),
            "um cancelamento que nao aconteceu contaminou o desfecho: {:?}",
            desfecho
        );
    }
}
