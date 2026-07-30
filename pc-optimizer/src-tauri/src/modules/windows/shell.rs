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
