// Tarefas que demoram minutos, e não milissegundos
//
// Todo comando do Otimiza responde em milissegundos, e a interface espera a
// resposta. Um `DISM` de vinte minutos nesse formato congela a janela.
//
// Este módulo não sabe o que é reparo. Ele roda um processo, entrega as linhas
// conforme elas saem, e aceita ser interrompido — e é isso que o torna útil
// também para a limpeza do WinSxS e para o que vier depois.

use std::io::{BufReader, Read};
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

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

/// Devolve a reserva quando `rodar` sai — por retorno, por `?` ou por pânico.
///
/// A reserva é o que impede duas ferramentas de reparo de rodar ao mesmo
/// tempo. Uma reserva que sobrevive à tarefa não é um vazamento discreto: ela
/// desliga a aba de reparo pelo resto da sessão. Por isso a devolução não pode
/// ficar por conta de alguém lembrar de escrevê-la em cada caminho de saída.
struct Reserva<'a> {
    dono: &'a TarefaLonga,
}

impl Drop for Reserva<'_> {
    fn drop(&mut self) {
        // Se a saída foi um pânico, a tranca fica envenenada. `ocupada()`
        // passa a responder OCUPADO, que é a resposta certa para "não sei" —
        // mas responder isso PARA SEMPRE seria trocar um defeito por outro.
        // Aqui a reserva é limpa mesmo com veneno e o veneno é retirado em
        // seguida: o estado que ele protege é um `Option<Estado>` que acabou
        // de ser zerado, e não sobra nada pela metade para contaminar a
        // próxima tarefa.
        match self.dono.atual.lock() {
            Ok(mut atual) => *atual = None,
            Err(envenenada) => {
                *envenenada.into_inner() = None;
                self.dono.atual.clear_poison();
            }
        }
    }
}

/// Lê a saída de um processo em BYTES, e nunca em UTF-8 estrito.
///
/// `BufRead::lines()` devolve `Err(InvalidData)` na primeira sequência que não
/// é UTF-8 válido, e `map_while(Result::ok)` ENCERRA o iterador ali. Num
/// Windows em português isso acontece no primeiro `ç`: o `chkdsk` escreve
/// texto traduzido na página de código do console (CP-850) e o `sfc` escreve
/// a saída canalizada em UTF-16. O laço terminava, ninguém drenava mais o
/// cano, o filho travava na primeira escrita que não coubesse no buffer, e o
/// `wait()` nunca voltava — a mesma classe de travamento que a leitura do
/// stderr já tinha evitado, entrando pela porta da frente.
///
/// Este projeto já tinha aprendido a lição e escrito o motivo: ver
/// `shell.rs` (`FORCAR_UTF8` e o bloco da sessão viva), que decodifica com
/// `from_utf8_lossy` justamente porque adivinhar a página de código do
/// console é impossível — ela muda com o idioma do Windows. Um caminho de
/// execução novo não pode reabrir isso.
///
/// A quebra é em `\r` E em `\n`. O `sfc` e o `DISM` desenham a porcentagem
/// com retorno de carro, redesenhando a MESMA linha: quebrando só em `\n`, o
/// "fica parado em 20%" — o número que a especificação diz ser o que impede o
/// cliente de desistir — nunca chegaria à tela.
fn drenar<L, F>(saida: L, numero: &AtomicUsize, ao_progredir: &Mutex<F>)
where
    L: Read,
    F: FnMut(Andamento),
{
    let mut leitor = BufReader::new(saida);
    let mut pendente: Vec<u8> = Vec::new();
    let mut bloco = [0u8; 4096];

    let entregar = |pedaco: &[u8]| {
        let linha = decodificar(pedaco);

        // Pedaço vazio não vira linha. Quebrando no retorno de carro E na
        // quebra de linha, todo par "CR LF" produz um pedaço vazio entre os
        // dois — sem este descarte, a tela receberia uma linha em branco a
        // cada linha de verdade.
        if linha.is_empty() {
            return;
        }
        let numero = numero.fetch_add(1, Ordering::SeqCst) + 1;
        if let Ok(mut callback) = ao_progredir.lock() {
            callback(Andamento { linha, numero });
        }
    };

    loop {
        let lidos = match leitor.read(&mut bloco) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            // Erro DE CANO, não de conteúdo: o duto fechou. Não há como
            // continuar lendo, e insistir seria um laço infinito.
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

    // O que sobrou sem quebra no fim ainda é saída, e costuma ser a última
    // linha do `DISM` — a que diz se deu certo.
    entregar(&pendente);
}

/// Decodifica um pedaço de saída sem nunca falhar.
///
/// Os NUL vêm do UTF-16 do `sfc`: nele um "A" viaja como `41 00`, e o `00` é
/// UTF-8 perfeitamente válido (U+0000), então `from_utf8_lossy` o preserva e
/// a tela receberia texto com um buraco entre cada letra. Texto de console
/// nunca tem NUL legítimo, então descartá-los deixa a saída ASCII do `sfc`
/// legível sem estragar a saída CP-850 ou UTF-8 de ninguém. Não é uma
/// decodificação de UTF-16 completa — acento em UTF-16 continua chegando
/// substituído — e não precisa ser: o veredito do `sfc` vem do CBS.log, e
/// está saída serve de sinal de vida.
fn decodificar(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|c| *c != '\0')
        .collect::<String>()
        .trim_end()
        .to_string()
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

    /// Se há tarefa reservada. Uma tranca envenenada responde OCUPADO.
    ///
    /// Envenenada significa que alguma thread entrou em pânico segurando o
    /// estado: não dá para saber se sobrou processo rodando. Responder
    /// "está livre" aí seria a mesma troca que este produto proíbe em todo
    /// lugar — "não sei" virando "está tudo bem" — e o preço dela aqui é
    /// deixar um segundo `DISM` nascer por cima do primeiro.
    pub fn ocupada(&self) -> bool {
        match self.atual.lock() {
            Ok(atual) => atual.is_some(),
            Err(_) => true,
        }
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

        // `taskkill /T` leva junto os processos filhos. O `DISM` cria um, e
        // matar só o pai deixaria o filho segurando os arquivos.
        //
        // O RETORNO É O CÓDIGO DE SAÍDA, NÃO O NASCIMENTO DO TASKKILL.
        // `output().is_ok()` só dizia que o `taskkill` conseguiu ser criado —
        // era verdadeiro também quando ele saía com 1 e "Acesso negado"
        // (matar filho elevado a partir de programa não elevado, falha
        // parcial do `/T`). E a bandeira de cancelamento era levantada ANTES
        // da morte: o `DISM` terminava inteiro, com sucesso, e o desfecho
        // ainda saía `Cancelada`. O cliente era informado com certeza de que o
        // reparo tinha sido interrompido justamente quando ele terminou.
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

    /// Roda o programa até o fim, chamando `ao_progredir` a cada linha.
    ///
    /// BLOQUEIA a thread que chamou. Quem chama é um comando do Tauri
    /// declarado `#[tauri::command(async)]`, que por isso já roda fora da
    /// thread da interface.
    pub fn rodar<F>(
        &self,
        programa: &str,
        args: &[&str],
        ao_progredir: F,
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

        // A PARTIR DAQUI A RESERVA É DEVOLVIDA POR SAÍDA DE ESCOPO.
        //
        // Antes eram três atribuições `*atual = None` colocadas à mão, e
        // faltava uma: o `?` do `filho.wait()` voltava sem passar por
        // nenhuma delas. Um erro do `wait` prendia o executor pelo resto da
        // sessão — todo reparo seguinte respondia "Já existe uma tarefa em
        // andamento". Um pânico no laço de leitura ou no callback fazia o
        // mesmo, e ainda envenenava a tranca. A guarda cobre os três
        // caminhos porque não depende de ninguém lembrar dela.
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

        // O `stderr` é canalizado (`Stdio::piped()`) e não pode ficar sem
        // leitor: o cano do Windows tem um buffer pequeno, e um `DISM` que
        // escreve o bastante ali trava para sempre esperando alguém drenar —
        // o mesmo travamento que o comentário abaixo descreve para o stdout,
        // só que no duto que ninguém olhava. Descartar essa saída não é
        // opção: é nela que mora o motivo de um `DISM` falhar ("precisa de
        // internet" vs. "a imagem está corrompida"), e sem esse texto a
        // falha fica muda para o cliente. A solução é uma thread dedicada,
        // lendo o stderr e alimentando o MESMO callback — a UI não distingue
        // de onde veio a linha, só precisa vê-la.
        let ao_progredir = Arc::new(Mutex::new(ao_progredir));
        let numero = Arc::new(AtomicUsize::new(0));

        let leitor_stderr = filho.stderr.take().map(|saida| {
            let callback = ao_progredir.clone();
            let numero = numero.clone();
            thread::spawn(move || drenar(saida, &numero, &callback))
        });

        // A saída é lida ENQUANTO o processo roda. Guardar para ler no fim
        // seria o mesmo que não ter andamento nenhum — e pior, encheria o cano
        // do sistema até o processo travar esperando alguém ler.
        if let Some(saida) = filho.stdout.take() {
            drenar(saida, &numero, &ao_progredir);
        }

        let status = filho.wait().map_err(|e| format!("o processo sumiu: {}", e))?;

        // O processo já terminou, então o stderr dele já fechou — a thread
        // sai do laço sozinha. Ainda assim é preciso esperar por ela: sem o
        // `join`, uma linha de stderr que chegou por último podia nunca ser
        // entregue antes de `rodar` devolver o desfecho.
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
    fn uma_de_cada_vez() {
        // Duas ferramentas de reparo ao mesmo tempo disputam os mesmos
        // arquivos, e o resultado é imprevisível para as duas.
        let tarefa = TarefaLonga::nova();
        assert!(!tarefa.ocupada(), "nasceu ocupada");
    }

    /// Um `stderr` canalizado e nunca lido enche o buffer do cano do Windows
    /// e trava o processo filho para sempre — exatamente a classe de
    /// travamento que o comentário acima do laço do stdout já evitava lá,
    /// só que no duto vizinho. Roda `rodar` numa thread à parte e usa um
    /// prazo: se o `stderr` não for drenado, o `recv_timeout` estoura antes
    /// da tarefa terminar, e o teste falha em vez de travar o CI para sempre.
    #[test]
    fn stderr_nao_trava_a_tarefa() {
        let tarefa = std::sync::Arc::new(TarefaLonga::nova());
        let dentro = tarefa.clone();
        let (envia, recebe) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            // Mais que o buffer do cano (uns 4 KB): sem drenar, o `cmd`
            // trava na primeira escrita que não coube.
            let resultado = dentro.rodar(
                "cmd",
                &["/c", "for /l %i in (1,1,5000) do @echo linha%i 1>&2"],
                |_| {},
            );
            let _ = envia.send(resultado);
        });

        let resultado = recebe
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("a tarefa travou — o stderr não está sendo drenado");

        assert!(matches!(resultado, Ok(Desfecho::Terminou { codigo: 0 })));
    }

    /// Junta o que `drenar` entregou, para os testes de decodificação.
    fn drenado(bytes: &[u8]) -> Vec<String> {
        let colhidas = Arc::new(Mutex::new(Vec::new()));
        let dentro = colhidas.clone();
        let numero = AtomicUsize::new(0);
        let callback = Mutex::new(move |a: Andamento| {
            dentro.lock().unwrap().push(a.linha);
        });

        drenar(std::io::Cursor::new(bytes.to_vec()), &numero, &callback);

        let saida = colhidas.lock().unwrap().clone();
        saida
    }

    #[test]
    fn byte_invalido_nao_interrompe_a_drenagem() {
        // Num Windows em português o primeiro "c cedilha" do `chkdsk` chega em
        // CP-850 (0xE7), que não e UTF-8 válido. Com `lines()` + `map_while`, o
        // iterador ENCERRAVA ali: ninguém drenava mais o cano, o filho travava
        // na primeira escrita que não coubesse, e o `wait()` nunca voltava.
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
        // O `sfc` e o `DISM` redesenham a MESMA linha com retorno de carro. Só
        // quebrando em `\n`, o "20%... 40%... 100%" — o número que impede o
        // cliente de desistir no meio — chegaria como uma linha só, no fim.
        let linhas = drenado(b"20%\r40%\r100%\r\nPronto\n");

        assert_eq!(linhas, vec!["20%", "40%", "100%", "Pronto"]);
    }

    #[test]
    fn nul_do_utf16_nao_vira_buraco_no_texto() {
        // O `sfc` escreve a saída canalizada em UTF-16: "ok" viaja como
        // `6F 00 6B 00`. O `00` e UTF-8 válido, então sobreviveria a
        // decodificação e a tela mostraria letra e buraco alternados.
        let linhas = drenado(b"o\0k\0\n\0");
        assert_eq!(linhas, vec!["ok"]);
    }

    #[test]
    fn saida_sem_quebra_no_fim_ainda_e_entregue() {
        // A última linha do `DISM` — a que diz se deu certo — costuma chegar
        // sem quebra depois dela.
        assert_eq!(
            drenado(b"A operacao foi concluida"),
            vec!["A operacao foi concluida"]
        );
    }

    /// A prova de ponta a ponta do mesmo defeito: um processo DE VERDADE que
    /// escreve bytes inválidos no meio e continua escrevendo depois. Se a
    /// drenagem parar na sequência inválida, o `cmd` trava esperando alguém
    /// ler o resto e o prazo estoura — em vez de o CI ficar preso para sempre.
    #[test]
    fn processo_com_saida_invalida_termina_e_nao_trunca() {
        let dir = std::env::temp_dir().join("otimiza_drenagem");
        let _ = std::fs::create_dir_all(&dir);
        let arquivo = dir.join(format!("saida_{}.txt", std::process::id()));

        let mut conteudo: Vec<u8> = Vec::new();
        conteudo.extend_from_slice(b"primeira\n");
        conteudo.extend_from_slice(&[0xE7, 0xE3, 0xF5]); // "cao" em CP-850
        conteudo.extend_from_slice(b"\n");
        // Mais que o buffer do cano: se a drenagem parar acima, o `type`
        // bloqueia aqui e a tarefa nunca termina.
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
        // Sem a guarda `Drop`, qualquer saída por erro deixava a reserva
        // presa: todo reparo seguinte respondia "Já existe uma tarefa em
        // andamento" pelo resto da sessão.
        let tarefa = TarefaLonga::nova();
        let erro = tarefa.rodar("programa_que_nao_existe_no_windows", &[], |_| {});

        assert!(erro.is_err(), "esperava falha ao iniciar: {:?}", erro);
        assert!(!tarefa.ocupada(), "a reserva ficou presa depois do erro");
    }

    #[test]
    fn panico_no_callback_devolve_a_reserva_e_limpa_o_veneno() {
        // Um pânico dentro do callback de emissão envenenava a tranca. A
        // partir dali `ocupada()` respondia OCIOSO (por causa de um
        // `unwrap_or(false)`) enquanto `rodar` respondia ocupado para sempre:
        // um "não sei" respondido como "está tudo bem" dentro do executor.
        let tarefa = Arc::new(TarefaLonga::nova());
        let dentro = tarefa.clone();

        // O pânico do callback e esperado: sem silenciar o relator, ele suja a
        // saída do teste com um rastro de pilha que não e falha nenhuma.
        let relator = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let panico = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = dentro.rodar("cmd", &["/c", "echo estoura"], |_| {
                panic!("panico de teste dentro do callback");
            });
        }));
        std::panic::set_hook(relator);

        assert!(panico.is_err(), "o panico de teste nao aconteceu");
        assert!(!tarefa.ocupada(), "a reserva sobreviveu ao panico");

        // E o executor volta a funcionar, em vez de ficar ocupado para sempre.
        let depois = tarefa.rodar("cmd", &["/c", "echo depois"], |_| {});
        assert!(
            matches!(depois, Ok(Desfecho::Terminou { codigo: 0 })),
            "o executor nao se recuperou: {:?}",
            depois
        );
    }

    #[test]
    fn cancelar_sem_nada_rodando_nao_levanta_a_bandeira() {
        // A bandeira era gravada ANTES da morte, e o retorno era só "o
        // taskkill conseguiu nascer". Uma morte que falha ("Acesso negado" ao
        // matar filho elevado) deixava a bandeira de pé, o `DISM` terminava
        // inteiro com sucesso, e o desfecho ainda saía `Cancelada` — o cliente
        // informado com certeza de que o reparo foi interrompido quando ele
        // terminou.
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
