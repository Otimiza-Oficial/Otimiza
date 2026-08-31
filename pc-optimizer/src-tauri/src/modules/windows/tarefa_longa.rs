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

/// O que existe enquanto a tarefa está reservada.
///
/// `pid` nasce vazio: a reserva acontece ANTES de o processo existir, para
/// fechar a janela entre "ninguém está rodando" e "o processo foi criado".
/// É essa reserva — não o PID, que só chega depois do `spawn` — que faz uma
/// segunda chamada concorrente ser recusada.
struct Estado {
    pid: Option<u32>,
}

pub struct TarefaLonga {
    /// `Some` significa reservado — com ou sem processo ainda rodando.
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

    pub fn ocupada(&self) -> bool {
        self.atual.lock().map(|a| a.is_some()).unwrap_or(false)
    }

    /// Pede para a tarefa parar. Devolve `false` se não havia nada rodando.
    pub fn cancelar(&self) -> bool {
        let Ok(atual) = self.atual.lock() else {
            return false;
        };

        // Reservado mas sem PID ainda: o `spawn` está no meio, e não há
        // processo para matar.
        let Some(Estado { pid: Some(pid) }) = *atual else {
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
        // A verificação e a reserva acontecem sob o MESMO lock, antes de
        // qualquer processo existir. Checar e só depois reservar em dois
        // passos deixaria uma brecha: duas chamadas concorrentes passariam
        // pela checagem antes de qualquer uma reservar, e as duas rodariam.
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

        self.cancelar_pedido.store(false, Ordering::SeqCst);

        let mut filho = match Command::new(programa)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
        {
            Ok(filho) => filho,
            Err(e) => {
                // Sem isto, uma falha ao iniciar deixaria a reserva presa
                // para sempre — o executor ficaria "ocupado" e nenhum reparo
                // futuro conseguiria rodar.
                if let Ok(mut atual) = self.atual.lock() {
                    *atual = None;
                }
                return Err(format!("não consegui iniciar `{}`: {}", programa, e));
            }
        };

        if let Ok(mut atual) = self.atual.lock() {
            *atual = Some(Estado {
                pid: Some(filho.id()),
            });
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
