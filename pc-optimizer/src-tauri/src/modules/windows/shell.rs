// Execução de comandos do sistema no Windows
//
// Todo comando roda com CREATE_NO_WINDOW: sem esse flag, cada `sc`, `powercfg`
// ou `netsh` abriria um console preto piscando na tela do usuário.

use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Executa um programa do sistema e captura a saída.
pub fn run(program: &str, args: &[&str]) -> Result<CommandOutput, String> {
    let output = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run `{}`: {}", program, e))?;

    Ok(CommandOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
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

/// Executa um script do PowerShell com a saída em UTF-8.
///
/// É por aqui que todo PowerShell do projeto passa. Chamar `run("powershell", …)`
/// direto funciona, mas devolve acento quebrado — ver `FORCAR_UTF8`.
pub fn powershell(script: &str) -> Result<CommandOutput, String> {
    // Tenta a sessão viva; se ela não estiver disponível por qualquer motivo,
    // cai para o processo de uma vez só, que sempre funciona.
    if let Some(saida) = sessao::executar(script) {
        return Ok(saida);
    }

    powershell_avulso(script)
}

/// Um processo do PowerShell por chamada. O caminho de reserva.
fn powershell_avulso(script: &str) -> Result<CommandOutput, String> {
    let completo = format!("{} {}", FORCAR_UTF8, script);
    run("powershell", &["-NoProfile", "-Command", &completo])
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
/// Se ela morrer, travar ou não subir, `executar` devolve `None` e a chamada
/// segue pelo processo avulso. Um diagnóstico lento é muito melhor que um
/// diagnóstico que não acontece.
mod sessao {
    use super::{CommandOutput, CREATE_NO_WINDOW, FORCAR_UTF8};
    use std::io::{BufRead, BufReader, Write};
    use std::os::windows::process::CommandExt;
    use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Mutex;

    struct Viva {
        processo: Child,
        entrada: ChildStdin,
        saida: BufReader<ChildStdout>,
    }

    static SESSAO: Mutex<Option<Viva>> = Mutex::new(None);
    static CONTADOR: AtomicU64 = AtomicU64::new(0);

    /// Uma sessão que morreu no meio de uma resposta não é reaproveitável, e
    /// insistir nela transformaria um diagnóstico lento num que não termina.
    static DESISTIMOS: AtomicBool = AtomicBool::new(false);

    fn abrir() -> Option<Viva> {
        let mut processo = Command::new("powershell")
            .args(["-NoProfile", "-NoLogo", "-NonInteractive", "-Command", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ok()?;

        let entrada = processo.stdin.take()?;
        let saida = BufReader::new(processo.stdout.take()?);

        let mut viva = Viva {
            processo,
            entrada,
            saida,
        };

        // A codificação é acertada UMA vez, na abertura — no processo avulso
        // ela era reenviada em cada chamada.
        viva.entrada.write_all(FORCAR_UTF8.as_bytes()).ok()?;
        viva.entrada.write_all(b"
").ok()?;
        viva.entrada.flush().ok()?;

        Some(viva)
    }

    /// Roda o script na sessão viva. `None` significa "use o caminho avulso".
    pub fn executar(script: &str) -> Option<CommandOutput> {
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
        let marca = format!(
            "<<<OTIMIZA-FIM-{}>>>",
            CONTADOR.fetch_add(1, Ordering::Relaxed)
        );

        // `$global:LASTEXITCODE` não serve: nem todo script chama programa
        // externo. O que interessa é se o script LANÇOU erro, e é isso que o
        // `try/catch` captura.
        let bloco = format!(
            "$ErrorActionPreference='Continue'; $__ok=$true;              try {{ {} }} catch {{ $__ok=$false }};              Write-Output ('{}' + $__ok)
",
            script, marca
        );

        if viva.entrada.write_all(bloco.as_bytes()).is_err() || viva.entrada.flush().is_err() {
            derrubar(&mut guarda);
            return None;
        }

        let mut coletado = String::new();
        let sucesso;

        loop {
            let mut linha = String::new();

            match viva.saida.read_line(&mut linha) {
                // Fim de arquivo sem a marca: a sessão morreu no meio.
                Ok(0) => {
                    derrubar(&mut guarda);
                    return None;
                }
                Ok(_) => {}
                Err(_) => {
                    derrubar(&mut guarda);
                    return None;
                }
            }

            if let Some(resto) = linha.trim_end().strip_prefix(marca.as_str()) {
                sucesso = !resto.trim().eq_ignore_ascii_case("False");
                break;
            }

            coletado.push_str(&linha);
        }

        Some(CommandOutput {
            success: sucesso,
            stdout: coletado,
            stderr: String::new(),
        })
    }

    fn derrubar(guarda: &mut Option<Viva>) {
        if let Some(mut viva) = guarda.take() {
            let _ = viva.processo.kill();
            let _ = viva.processo.wait();
        }

        // Uma sessão que caiu costuma cair de novo. Desistir de vez custa a
        // lentidão do caminho avulso; insistir custa uma falha por consulta.
        DESISTIMOS.store(true, Ordering::Relaxed);
    }
}

/// Igual a `powershell`, mas devolve `Err` quando o script falha.
pub fn powershell_checked(script: &str) -> Result<String, String> {
    let saida = powershell(script)?;

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
    let output = run(program, args)?;

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
        assert!(super::sessao::executar("Write-Output 'Ação'").is_none());

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

        let pela_sessao = super::sessao::executar(script).expect("script ASCII usa a sessão");
        let avulso = super::powershell_avulso(script).unwrap();

        assert_eq!(pela_sessao.stdout.trim(), avulso.stdout.trim());
        assert_eq!(pela_sessao.success, avulso.success);
    }

    #[test]
    fn script_que_lanca_erro_e_reportado_como_falha_pela_sessao() {
        // Sem isto, a sessão diria "deu certo" para tudo, e quem chama deixaria
        // de perceber a diferença entre "não há dado" e "a consulta quebrou" —
        // que é a distinção que este produto inteiro se apoia.
        let saida = super::sessao::executar("throw 'quebrou'").expect("script ASCII usa a sessão");
        assert!(!saida.success);
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
