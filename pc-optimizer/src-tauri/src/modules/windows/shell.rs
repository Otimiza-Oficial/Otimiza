// Execução de comandos do sistema no Windows
//
// Todo comando roda com CREATE_NO_WINDOW: sem esse flag, cada `sc`, `powercfg`
// ou `netsh` abriria um console preto piscando na tela do usuário.
//
// E TODO COMANDO TEM PRAZO.
//
// Até a 1.9, `run` esperava o programa terminar para sempre, e a sessão do
// PowerShell lia a resposta sem limite. Um `powercfg` ou um `sc` que travasse
// prendia o "Otimizar agora" inteiro, com a tela parada em "Aplicando…" e sem
// rastro de qual passo tinha parado. No PC do dono, em 10 e 11/09/2026, o clique
// no "Otimizar agora" foi seguido de "os programas não abriam" — e não havia
// como provar nem descartar um comando pendurado.
//
// Estourado o prazo, o processo é encerrado, a chamada vira erro com o nome do
// comando, e a linha vai para o registro em arquivo (`utils::logger`). O motor
// de otimização trata esse erro como qualquer outra falha: desfaz o que o item
// já tinha feito.

use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Quanto um programa do sistema tem para terminar, quando quem chama não diz.
///
/// `sc`, `powercfg`, `bcdedit`, `fsutil` e `ipconfig` respondem em menos de um
/// segundo numa máquina saudável. Um minuto é folga de sobra para máquina lenta
/// — e é finito, que é o que importa.
pub const PRAZO_PADRAO: Duration = Duration::from_secs(60);

/// Quanto um script do PowerShell tem para terminar, quando quem chama não diz.
///
/// Maior que o dos programas porque o PowerShell faz consulta de WMI e de log de
/// eventos, que num PC fraco com o disco ocupado passam de dez segundos. Quem
/// sabe que vai demorar mais — ponto de restauração, esvaziar a lixeira — passa
/// o próprio prazo.
pub const PRAZO_DO_POWERSHELL: Duration = Duration::from_secs(120);

pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Executa um programa do sistema e captura a saída, com o `PRAZO_PADRAO`.
pub fn run(program: &str, args: &[&str]) -> Result<CommandOutput, String> {
    run_com_prazo(program, args, PRAZO_PADRAO)
}

/// Executa um programa do sistema e captura a saída, desistindo em `prazo`.
pub fn run_com_prazo(
    program: &str,
    args: &[&str],
    prazo: Duration,
) -> Result<CommandOutput, String> {
    let rotulo = if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    };

    executar_com_rotulo(program, args, &resumir(&rotulo), prazo)
}

fn executar_com_rotulo(
    program: &str,
    args: &[&str],
    rotulo: &str,
    prazo: Duration,
) -> Result<CommandOutput, String> {
    let mut filho = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run `{}`: {}", program, e))?;

    esperar_com_prazo(&mut filho, rotulo, prazo)
}

/// Sobe um programa do sistema e devolve o processo VIVO, com a saída ligada
/// num cano — sem esperar ele terminar.
///
/// Serve a quem precisa do processo na mão, como a análise do DISM, que passa o
/// processo para `esperar_com_prazo` e confere no teste que ele morreu.
///
/// O `CREATE_NO_WINDOW` mora aqui pelo mesmo motivo de sempre: um console preto
/// piscando na tela do cliente.
pub fn spawn_capturando(program: &str, args: &[&str]) -> Result<std::process::Child, String> {
    Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn `{}`: {}", program, e))
}

/// Espera um processo terminar por um prazo — e, se o prazo estourar, ENCERRA o
/// processo em vez de deixá-lo rodando sozinho.
///
/// Recebe o `Child` emprestado (e não por valor) de propósito: quem chamou
/// continua dono do processo e pode conferir, no teste, que ele de fato morreu.
///
/// Os canos são lidos em threads à parte por dois motivos: ler um cano até o fim
/// bloqueia até o processo fechá-lo, e um processo que escreve muito sem ninguém
/// ler trava com o cano cheio. Encerrar o processo fecha os canos, a leitura
/// termina e as threads morrem junto.
///
/// Sai sempre com o processo esperado, inclusive depois de encerrá-lo: sem o
/// `wait`, o processo morto fica como zumbi até o Otimiza fechar.
pub fn esperar_com_prazo(
    filho: &mut Child,
    rotulo: &str,
    prazo: Duration,
) -> Result<CommandOutput, String> {
    let limite = Instant::now() + prazo;
    let (tx, rx) = mpsc::channel::<(bool, Vec<u8>)>();

    ler_em_segundo_plano(filho.stdout.take(), true, tx.clone());
    ler_em_segundo_plano(filho.stderr.take(), false, tx);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    for _ in 0..2 {
        match rx.recv_timeout(limite.saturating_duration_since(Instant::now())) {
            Ok((true, bruto)) => stdout = bruto,
            Ok((false, bruto)) => stderr = bruto,
            Err(mpsc::RecvTimeoutError::Timeout) => return Err(encerrar(filho, rotulo, prazo)),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = filho.kill();
                let _ = filho.wait();
                return Err(format!("perdi a saída de `{}` no meio da leitura", rotulo));
            }
        }
    }

    // Os dois canos fecharam, o que quase sempre quer dizer que o processo
    // saiu. "Quase": um programa pode fechar a saída e seguir vivo, e um `wait`
    // sem prazo aqui devolveria exatamente o defeito que esta função conserta.
    let status = loop {
        match filho.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < limite => std::thread::sleep(Duration::from_millis(5)),
            Ok(None) => return Err(encerrar(filho, rotulo, prazo)),
            Err(e) => {
                let _ = filho.kill();
                let _ = filho.wait();
                return Err(format!("não consegui acompanhar `{}`: {}", rotulo, e));
            }
        }
    };

    Ok(CommandOutput {
        success: status.success(),
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
    })
}

fn ler_em_segundo_plano<R: Read + Send + 'static>(
    cano: Option<R>,
    e_saida: bool,
    tx: mpsc::Sender<(bool, Vec<u8>)>,
) {
    let Some(mut cano) = cano else {
        // Cano não canalizado (o `stderr` de `spawn_capturando`): não há o que
        // ler, e não vale uma thread para descobrir isso.
        let _ = tx.send((e_saida, Vec::new()));
        return;
    };

    std::thread::spawn(move || {
        let mut bruto = Vec::new();
        let _ = cano.read_to_end(&mut bruto);
        let _ = tx.send((e_saida, bruto));
    });
}

/// Encerra o processo que estourou o prazo, anota no registro e devolve a
/// mensagem de erro.
fn encerrar(filho: &mut Child, rotulo: &str, prazo: Duration) -> String {
    let _ = filho.kill();
    let _ = filho.wait();

    let mensagem = format!(
        "`{}` não terminou em {} e foi encerrado",
        rotulo,
        descrever_prazo(prazo)
    );
    crate::utils::Logger::warn(&mensagem);
    mensagem
}

fn descrever_prazo(prazo: Duration) -> String {
    if prazo.as_millis() < 1000 {
        format!("{} ms", prazo.as_millis())
    } else {
        format!("{} s", prazo.as_secs())
    }
}

/// Um rótulo curto para erro e registro: o script de um PowerShell pode ter
/// centenas de caracteres e várias linhas. Corta por caractere, não por byte —
/// cortar no meio de um "ç" derrubaria o programa.
fn resumir(texto: &str) -> String {
    const MAXIMO: usize = 160;

    let limpo = texto.split_whitespace().collect::<Vec<_>>().join(" ");

    if limpo.chars().count() <= MAXIMO {
        limpo
    } else {
        format!("{}…", limpo.chars().take(MAXIMO).collect::<String>())
    }
}

/// Prefixo obrigatório de todo script do PowerShell.
///
/// Sem isto, o PowerShell escreve a saída na página de código do console — CP850
/// num Windows em português — e não em UTF-8. O resultado é que todo nome com
/// acento chega corrompido: "Serviço do Brave Update" vira "Servi?o do Brave
/// Update". Como quase tudo que este produto lê do sistema é nome escolhido por
/// terceiros (serviço, tarefa agendada, programa instalado), o estrago aparecia
/// em lista, em painel e no relatório entregue ao cliente.
///
/// Uma linha resolve na origem, e resolver na origem é melhor que adivinhar a
/// página de código na hora de decodificar — ela muda com o idioma do Windows.
const FORCAR_UTF8: &str = "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;";

/// Executa um script do PowerShell com a saída em UTF-8, com o
/// `PRAZO_DO_POWERSHELL`.
///
/// É por aqui que todo PowerShell do projeto passa. Chamar `run("powershell", …)`
/// direto funciona, mas devolve acento quebrado — ver `FORCAR_UTF8`.
pub fn powershell(script: &str) -> Result<CommandOutput, String> {
    powershell_com_prazo(script, PRAZO_DO_POWERSHELL)
}

/// Igual a `powershell`, desistindo em `prazo`.
pub fn powershell_com_prazo(script: &str, prazo: Duration) -> Result<CommandOutput, String> {
    // Tenta a sessão viva; se ela não estiver disponível por qualquer motivo,
    // cai para o processo de uma vez só, que sempre funciona.
    //
    // O prazo estourado NÃO cai para o avulso. O script já rodou o prazo inteiro
    // e travou; rodá-lo de novo dobraria a espera, e num script que escreve no
    // sistema repetiria a escrita.
    if let Some(resposta) = sessao::executar(script, prazo) {
        return resposta;
    }

    powershell_avulso(script, prazo)
}

/// Um processo do PowerShell por chamada. O caminho de reserva.
fn powershell_avulso(script: &str, prazo: Duration) -> Result<CommandOutput, String> {
    let completo = format!("{} {}", FORCAR_UTF8, script);
    let rotulo = resumir(&format!("powershell: {}", script));

    executar_com_rotulo("powershell", &["-NoProfile", "-Command", &completo], &rotulo, prazo)
}

/// Uma sessão do PowerShell viva, reaproveitada entre consultas.
///
/// POR QUE ISTO EXISTE
///
/// Medido nesta máquina: abrir um `powershell.exe` VAZIO — um processo que só
/// executa `1` e sai — custa **2,26 segundos**. Não é a consulta que é cara: é
/// o processo. Os módulos que fazem uma única chamada custavam exatamente
/// isso, e o diagnóstico inicial abria dez processos.
///
/// Vinte e dois dos trinta e um segundos de abertura eram só o Windows subindo
/// o PowerShell, dez vezes.
///
/// A alternativa óbvia era juntar as consultas num script gigante, o que
/// obrigaria a reescrever dez módulos. Manter UM processo vivo paga o custo uma
/// vez e não pede mudança em nenhum chamador: `powershell()` continua com a
/// mesma assinatura, e quem chama nem sabe que a sessão existe.
///
/// COMO SE SABE ONDE ACABA UMA RESPOSTA
///
/// O processo lê comandos da entrada padrão e nunca termina, então não há
/// código de saída nem fim de arquivo para esperar. Depois de cada script a
/// sessão imprime uma marca com um número que só aquela consulta conhece, e a
/// leitura para ali. A marca carrega também se o script deu erro, que é o que
/// `success` significa no caminho de reserva.
///
/// QUANDO A SESSÃO NÃO SERVE
///
/// Se ela morrer ou não subir, `executar` devolve `None` e a chamada segue pelo
/// processo avulso. Um diagnóstico lento é muito melhor que um diagnóstico que
/// não acontece. Se ela TRAVAR, o prazo estoura, a sessão é derrubada e a
/// chamada volta como erro.
mod sessao {
    use super::{CommandOutput, CREATE_NO_WINDOW, FORCAR_UTF8};
    use std::io::{BufRead, BufReader, Write};
    use std::os::windows::process::CommandExt;
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{mpsc, Mutex};
    use std::time::{Duration, Instant};

    pub(super) struct Viva {
        processo: Child,
        entrada: ChildStdin,
        /// As linhas que o PowerShell escreve, lidas por uma thread à parte.
        ///
        /// Até a 1.9 a leitura era direto do cano, com `read_line`, que bloqueia
        /// sem limite: um script travado prendia quem chamou para sempre. Pelo
        /// canal, a espera tem prazo.
        linhas: mpsc::Receiver<String>,
    }

    impl Viva {
        pub(super) fn encerrar(&mut self) {
            // Matar o processo fecha o cano; a thread de leitura recebe o fim de
            // arquivo e termina sozinha.
            let _ = self.processo.kill();
            let _ = self.processo.wait();
        }
    }

    /// O que aconteceu com um script mandado à sessão.
    pub(super) enum Resposta {
        Respondeu(CommandOutput),
        /// A sessão caiu antes da marca. O script pode ser repetido pelo avulso.
        Morreu,
        /// O prazo acabou sem a marca. O script NÃO deve ser repetido.
        Estourou,
    }

    static SESSAO: Mutex<Option<Viva>> = Mutex::new(None);
    static CONTADOR: AtomicU64 = AtomicU64::new(0);

    /// Uma sessão que morreu ou travou no meio de uma resposta não é
    /// reaproveitável, e insistir nela transformaria um diagnóstico lento num
    /// que não termina.
    static DESISTIMOS: AtomicBool = AtomicBool::new(false);

    pub(super) fn abrir() -> Option<Viva> {
        let mut processo = Command::new("powershell")
            .args(["-NoProfile", "-NoLogo", "-NonInteractive", "-Command", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ok()?;

        let entrada = processo.stdin.take()?;
        let mut saida = BufReader::new(processo.stdout.take()?);
        let (tx, linhas) = mpsc::channel();

        std::thread::spawn(move || loop {
            let mut linha = String::new();

            match saida.read_line(&mut linha) {
                // Fim de arquivo ou erro: a sessão morreu. Soltar o canal é o
                // aviso — quem espera recebe `Disconnected` na hora.
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx.send(linha).is_err() {
                        break;
                    }
                }
            }
        });

        let mut viva = Viva {
            processo,
            entrada,
            linhas,
        };

        // A codificação é acertada UMA vez, na abertura — no processo avulso
        // ela era reenviada em cada chamada.
        viva.entrada.write_all(FORCAR_UTF8.as_bytes()).ok()?;
        viva.entrada.write_all(b"\n").ok()?;
        viva.entrada.flush().ok()?;

        Some(viva)
    }

    /// Roda o script na sessão viva.
    ///
    /// `None` significa "use o caminho avulso". `Some(Err)` é o prazo estourado,
    /// que não se repete pelo avulso.
    pub fn executar(script: &str, prazo: Duration) -> Option<Result<CommandOutput, String>> {
        if DESISTIMOS.load(Ordering::Relaxed) {
            return None;
        }

        // SCRIPT COM ACENTO NÃO PASSA POR AQUI.
        //
        // A codificação tem dois lados, e a sessão só resolve um. A SAÍDA vem
        // certa: `[Console]::OutputEncoding` é acertado na abertura, e os bytes
        // de "Ação" chegam como UTF-8 válido — foi medido.
        //
        // A ENTRADA não. O PowerShell lê a entrada padrão usando a página de
        // código do console, e não há como acertar isso de dentro do próprio
        // fluxo: quando a primeira linha chega, ele já leu com a página errada.
        // Um script contendo "Ação" chegava lá dentro como "A├º├úo", e o erro
        // acontecia ANTES de o script rodar.
        //
        // O processo avulso não sofre disso, porque ali o script viaja como
        // argumento da linha de comando e não pela entrada padrão.
        //
        // Então a regra é simples e não depende de auditar os scripts de hoje:
        // qualquer coisa fora do ASCII vai pelo caminho lento. Custa a
        // lentidão de um processo nos poucos casos em que isso acontece, e
        // remove por construção uma classe inteira de corrupção silenciosa.
        if !script.is_ascii() {
            return None;
        }

        let mut guarda = SESSAO.lock().ok()?;

        if guarda.is_none() {
            *guarda = abrir();
        }

        let viva = guarda.as_mut()?;

        match conversar(viva, script, prazo) {
            Resposta::Respondeu(saida) => Some(Ok(saida)),
            Resposta::Morreu => {
                derrubar(&mut guarda);
                None
            }
            Resposta::Estourou => {
                derrubar(&mut guarda);

                let mensagem = format!(
                    "`powershell: {}` não terminou em {} e foi encerrado",
                    super::resumir(script),
                    super::descrever_prazo(prazo)
                );
                crate::utils::Logger::warn(&mensagem);
                Some(Err(mensagem))
            }
        }
    }

    /// Manda um script a uma sessão e espera a marca de fim até o prazo.
    ///
    /// Separada de `executar` para o teste poder travar uma sessão PRÓPRIA: a
    /// global, derrubada, faria os outros testes caírem no avulso no meio.
    pub(super) fn conversar(viva: &mut Viva, script: &str, prazo: Duration) -> Resposta {
        let limite = Instant::now() + prazo;
        let marca = format!(
            "<<<OTIMIZA-FIM-{}>>>",
            CONTADOR.fetch_add(1, Ordering::Relaxed)
        );

        // `$global:LASTEXITCODE` não serve: nem todo script chama programa
        // externo. O que interessa é se o script LANÇOU erro, e é isso que o
        // `try/catch` captura.
        let bloco = format!(
            "$ErrorActionPreference='Continue'; $__ok=$true; try {{ {} }} catch {{ $__ok=$false }}; Write-Output ('{}' + $__ok)\n",
            script, marca
        );

        if viva.entrada.write_all(bloco.as_bytes()).is_err() || viva.entrada.flush().is_err() {
            return Resposta::Morreu;
        }

        let mut coletado = String::new();

        loop {
            let linha = match viva
                .linhas
                .recv_timeout(limite.saturating_duration_since(Instant::now()))
            {
                Ok(linha) => linha,
                Err(mpsc::RecvTimeoutError::Timeout) => return Resposta::Estourou,
                // A thread de leitura soltou o canal: fim de arquivo sem a
                // marca, a sessão morreu no meio.
                Err(mpsc::RecvTimeoutError::Disconnected) => return Resposta::Morreu,
            };

            if let Some(resto) = linha.trim_end().strip_prefix(marca.as_str()) {
                return Resposta::Respondeu(CommandOutput {
                    success: !resto.trim().eq_ignore_ascii_case("False"),
                    stdout: coletado,
                    stderr: String::new(),
                });
            }

            coletado.push_str(&linha);
        }
    }

    fn derrubar(guarda: &mut Option<Viva>) {
        if let Some(mut viva) = guarda.take() {
            viva.encerrar();
        }

        // Uma sessão que caiu costuma cair de novo. Desistir de vez custa a
        // lentidão do caminho avulso; insistir custa uma falha por consulta.
        DESISTIMOS.store(true, Ordering::Relaxed);
    }
}

/// Igual a `powershell`, mas devolve `Err` quando o script falha.
pub fn powershell_checked(script: &str) -> Result<String, String> {
    powershell_checked_com_prazo(script, PRAZO_DO_POWERSHELL)
}

/// Igual a `powershell_checked`, desistindo em `prazo`.
pub fn powershell_checked_com_prazo(script: &str, prazo: Duration) -> Result<String, String> {
    let saida = powershell_com_prazo(script, prazo)?;

    if saida.success {
        Ok(saida.stdout)
    } else {
        let detalhe = if saida.stderr.trim().is_empty() {
            saida.stdout.trim().to_string()
        } else {
            saida.stderr.trim().to_string()
        };
        Err(detalhe)
    }
}

/// Executa e falha com erro descritivo se o comando retornar código diferente de zero.
pub fn run_checked(program: &str, args: &[&str]) -> Result<String, String> {
    run_checked_com_prazo(program, args, PRAZO_PADRAO)
}

/// Igual a `run_checked`, desistindo em `prazo`.
pub fn run_checked_com_prazo(
    program: &str,
    args: &[&str],
    prazo: Duration,
) -> Result<String, String> {
    let output = run_com_prazo(program, args, prazo)?;

    if output.success {
        Ok(output.stdout)
    } else {
        let detail = if output.stderr.trim().is_empty() {
            output.stdout.trim().to_string()
        } else {
            output.stderr.trim().to_string()
        };
        Err(format!("`{} {}` failed: {}", program, args.join(" "), detail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saida_do_powershell_chega_com_acento_intacto() {
        // O texto tem ç, ã e á de propósito: são os que quebram em CP850, a
        // página de código padrão do console num Windows em português.
        let saida = powershell("Write-Output 'Serviço de Configuração Básica'")
            .expect("o PowerShell precisa rodar");

        assert!(
            saida.stdout.contains("Serviço de Configuração Básica"),
            "acento corrompido na saída: {:?}",
            saida.stdout.trim()
        );
        // O caractere de substituição é o sintoma exato do erro que isto corrige.
        assert!(!saida.stdout.contains('\u{FFFD}'));
    }

    #[test]
    fn json_com_acento_sobrevive_a_desserializacao() {
        // O caminho real do produto: PowerShell devolve JSON, o serde lê. Se a
        // codificação estiver errada, o JSON chega com bytes inválidos.
        let saida = powershell(
            "ConvertTo-Json -Compress -InputObject @{ nome = 'Ação de Manutenção' }",
        )
        .expect("o PowerShell precisa rodar");

        let valor: serde_json::Value =
            serde_json::from_str(saida.stdout.trim()).expect("JSON válido");

        assert_eq!(valor["nome"], "Ação de Manutenção");
    }

    #[test]
    fn script_que_falha_devolve_erro() {
        let erro = powershell_checked("throw 'falhou de proposito'");
        assert!(erro.is_err());
    }

    #[test]
    fn a_sessao_viva_recusa_script_com_acento() {
        // A armadilha que este teste tranca é silenciosa: um script com acento
        // passando pela sessão não FALHA, ele devolve o resultado errado.
        //
        // O PowerShell lê a entrada padrão na página de código do console, e
        // "Ação" chega lá dentro como "A├º├úo" — antes de o script rodar. O
        // processo avulso não sofre disso porque o script viaja na linha de
        // comando.
        assert!(super::sessao::executar("Write-Output 'Ação'", PRAZO_DO_POWERSHELL).is_none());

        // E o caminho completo continua entregando o texto certo, porque cai
        // no avulso sozinho.
        let saida = powershell("Write-Output 'Ação de Manutenção'").unwrap();
        assert!(saida.stdout.contains("Ação de Manutenção"), "veio: {}", saida.stdout);
    }

    #[test]
    fn a_sessao_viva_devolve_o_mesmo_que_o_processo_avulso() {
        // O ganho de velocidade não vale nada se a resposta mudar. Um script
        // ASCII precisa dar exatamente o mesmo resultado pelos dois caminhos.
        let script = "ConvertTo-Json -Compress -InputObject ([ordered]@{ a = 1; b = 'dois' })";

        let pela_sessao = super::sessao::executar(script, PRAZO_DO_POWERSHELL)
            .expect("script ASCII usa a sessão")
            .expect("a sessão respondeu no prazo");
        let avulso = super::powershell_avulso(script, PRAZO_DO_POWERSHELL).unwrap();

        assert_eq!(pela_sessao.stdout.trim(), avulso.stdout.trim());
        assert_eq!(pela_sessao.success, avulso.success);
    }

    #[test]
    fn script_que_lanca_erro_e_reportado_como_falha_pela_sessao() {
        // Sem isto, a sessão diria "deu certo" para tudo, e quem chama deixaria
        // de perceber a diferença entre "não há dado" e "a consulta quebrou" —
        // que é a distinção que este produto inteiro se apoia.
        let saida = super::sessao::executar("throw 'quebrou'", PRAZO_DO_POWERSHELL)
            .expect("script ASCII usa a sessão")
            .expect("a sessão respondeu no prazo");
        assert!(!saida.success);
    }

    #[test]
    fn a_sessao_que_trava_e_encerrada_no_prazo() {
        // O DEFEITO: a sessão lia a resposta com `read_line` sem limite. Um
        // script que travasse prendia quem chamou para sempre — no "Otimizar
        // agora", a tela parada em "Aplicando…" sem fim.
        //
        // Sessão própria, e não a global: estourar o prazo derruba a sessão, e
        // derrubar a global faria os outros testes deste arquivo caírem no
        // caminho avulso no meio da execução.
        let mut viva = super::sessao::abrir().expect("o PowerShell sobe");

        // Antes de travar, a sessão própria responde normalmente — senão o
        // estouro abaixo poderia ser só uma sessão que nunca funcionou.
        let normal = super::sessao::conversar(&mut viva, "Write-Output 'ok'", PRAZO_DO_POWERSHELL);
        let respondeu = matches!(
            &normal,
            super::sessao::Resposta::Respondeu(saida) if saida.stdout.trim() == "ok"
        );

        let inicio = Instant::now();
        let travada = super::sessao::conversar(
            &mut viva,
            "Start-Sleep -Seconds 60",
            Duration::from_millis(1500),
        );
        let esperou = inicio.elapsed();
        viva.encerrar();

        assert!(respondeu, "a sessão própria não respondeu nem ao script simples");
        assert!(
            matches!(travada, super::sessao::Resposta::Estourou),
            "um script de 60 s com prazo de 1,5 s não foi dado como estourado"
        );
        assert!(
            esperou < Duration::from_secs(10),
            "não desistiu no prazo: esperou {:?}",
            esperou
        );
    }

    #[test]
    fn desistir_de_esperar_mata_o_processo_em_vez_de_deixar_rodando() {
        // O DEFEITO: o `recv_timeout` devolvia o controle, mas o `Dism.exe`
        // seguia até o fim — de 1 a 5 minutos de disco e CPU numa máquina que,
        // por definição, é o "PC fraco" que este produto existe para ajudar. E
        // como cada clique em "Limpar" refazia a varredura, eles empilhavam.
        //
        // (O teste morava em `diskspace.rs`. Subiu para cá junto com a função,
        // que desde a 2.0 serve todo comando do produto e não só o DISM.)
        //
        // `ping -n 30` no lugar do DISM: um processo que demora muito mais que
        // o prazo, sem precisar de administrador nem mexer no sistema.
        let mut filho = spawn_capturando("ping", &["-n", "30", "127.0.0.1"])
            .expect("o ping do Windows sobe");

        let inicio = Instant::now();
        let saida = esperar_com_prazo(&mut filho, "ping", Duration::from_millis(300));

        assert!(saida.is_err(), "o prazo estourou; não podia vir saída");
        assert!(
            inicio.elapsed() < Duration::from_secs(10),
            "não desistiu no prazo: esperou {:?}",
            inicio.elapsed()
        );

        // A prova: o processo precisa estar MORTO agora. Sem o `kill`, ele
        // continuaria vivo aqui pelos ~30 s do ping.
        let mut morreu = false;
        for _ in 0..100 {
            if matches!(filho.try_wait(), Ok(Some(_))) {
                morreu = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = filho.kill();

        assert!(
            morreu,
            "desistimos de esperar e o processo continuou rodando sozinho"
        );
    }

    #[test]
    fn um_comando_rapido_devolve_saida_erro_e_codigo_como_antes() {
        // `run` passou a ter prazo, e isso não pode ter custado o que ele já
        // entregava: a saída, o erro e se o programa terminou bem. Os módulos
        // de serviço leem o código 1056/1062 da saída do `sc` — perder a saída
        // aqui faria "serviço já parado" virar falha.
        let saida = run("cmd", &["/c", "echo saida& echo erro 1>&2& exit 3"])
            .expect("o cmd do Windows roda");

        assert!(!saida.success, "saiu com código 3 e veio como sucesso");
        assert!(saida.stdout.contains("saida"), "stdout: {:?}", saida.stdout);
        assert!(saida.stderr.contains("erro"), "stderr: {:?}", saida.stderr);
    }

    #[test]
    fn run_com_prazo_encerra_o_programa_que_nao_termina() {
        let inicio = Instant::now();
        let erro = run_com_prazo("ping", &["-n", "30", "127.0.0.1"], Duration::from_millis(500))
            .err()
            .expect("um ping de 30 s com prazo de meio segundo precisa falhar");

        assert!(inicio.elapsed() < Duration::from_secs(10));
        // O erro diz QUAL comando parou — é o que vai para o registro e o que
        // permite saber, depois, onde o lote travou.
        assert!(erro.contains("ping -n 30"), "o erro não diz o comando: {}", erro);
    }

    #[test]
    fn rotulo_longo_e_cortado_sem_quebrar_acento() {
        let longo = "ç".repeat(400);
        let rotulo = resumir(&longo);

        assert_eq!(rotulo.chars().count(), 161);
        assert!(rotulo.ends_with('…'));
        assert_eq!(resumir("  Get-Thing \n  -Flag  "), "Get-Thing -Flag");
    }

    #[test]
    fn ninguem_chama_o_powershell_por_fora_do_helper() {
        // Chamar `run("powershell", …)` direto compila e funciona — e devolve
        // acento quebrado, silenciosamente. O erro só aparece na tela do
        // cliente, num nome de serviço ou de programa. Uma trava é mais barata
        // que descobrir isso de novo daqui a seis meses.
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut infratores = Vec::new();

        fn varrer(dir: &std::path::Path, achados: &mut Vec<String>) {
            let Ok(entradas) = std::fs::read_dir(dir) else { return };

            for entrada in entradas.flatten() {
                let caminho = entrada.path();

                if caminho.is_dir() {
                    varrer(&caminho, achados);
                    continue;
                }

                let nome = caminho.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !nome.ends_with(".rs") || nome == "shell.rs" {
                    continue;
                }

                let Ok(conteudo) = std::fs::read_to_string(&caminho) else { continue };

                for (numero, linha) in conteudo.lines().enumerate() {
                    // A única exceção legítima: reabrir o programa como
                    // administrador. Não lê saída nenhuma, então codificação
                    // não se aplica, e ela precisa de `-WindowStyle Hidden`.
                    if linha.contains("\"powershell\"") && !conteudo.contains("Start-Process -FilePath") {
                        achados.push(format!("{}:{}", nome, numero + 1));
                    }
                }
            }
        }

        varrer(&raiz, &mut infratores);

        assert!(
            infratores.is_empty(),
            "estes pontos chamam o PowerShell sem forçar UTF-8 e vão devolver \
             acento quebrado — use shell::powershell(…): {:?}",
            infratores
        );
    }
}
