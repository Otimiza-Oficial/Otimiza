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
    let completo = format!("{} {}", FORCAR_UTF8, script);
    run("powershell", &["-NoProfile", "-Command", &completo])
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
