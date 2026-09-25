// Tarefas agendadas de terceiros (atualizadores e utilitários de fabricante): listar, desligar e voltar atrás.
// As da Microsoft ficam de fora, como os serviços do sistema.

use super::shell;
use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub name: String,
    pub path: String,
    pub author: String,
    pub enabled: bool,
    pub microsoft: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTask {
    task_name: Option<String>,
    task_path: Option<String>,
    author: Option<String>,
    state: Option<String>,
}

/// O caminho `\Microsoft\` é o critério, e não o autor, que vem em branco ou traduzido em parte das tarefas.
pub fn e_da_microsoft(caminho: &str) -> bool {
    caminho.to_lowercase().starts_with("\\microsoft\\")
}

/// `Get-ScheduledTask` devolve propriedades em inglês em qualquer idioma. `Err` quando o Agendador não
/// responde: lista vazia diria "nenhuma tarefa de terceiros" com o serviço parado (imagens "lite" do Windows).
pub fn listar() -> Result<Vec<ScheduledTask>, String> {
    let script = "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-ScheduledTask -ErrorAction Stop | \
                  Select-Object TaskName,TaskPath,Author,@{n='State';e={$_.State.ToString()}})";

    let brutas: Vec<RawTask> =
        shell::json_da_saida(shell::powershell(script), "as tarefas agendadas")?;

    let mut tarefas: Vec<ScheduledTask> = brutas
        .into_iter()
        .filter_map(|t| {
            let name = t.task_name?;
            let path = t.task_path.unwrap_or_else(|| "\\".to_string());

            Some(ScheduledTask {
                microsoft: e_da_microsoft(&path),
                enabled: t.state.as_deref() != Some("Disabled"),
                author: t.author.unwrap_or_default(),
                name,
                path,
            })
        })
        .collect();

    tarefas.sort_by(|a, b| {
        b.enabled
            .cmp(&a.enabled)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(tarefas)
}

pub fn listar_de_terceiros() -> Result<Vec<ScheduledTask>, String> {
    Ok(listar()?.into_iter().filter(|t| !t.microsoft).collect())
}

pub fn definir_estado(path: &str, name: &str, ligar: bool) -> Result<ChangeRecord, String> {
    if !super::registry::is_elevated() {
        return Err("Mexer em tarefas agendadas exige executar como administrador.".to_string());
    }

    if e_da_microsoft(path) {
        return Err(format!(
            "`{}` é uma tarefa do próprio Windows e não é mexida pelo Otimiza.",
            name
        ));
    }

    let comando = if ligar { "Enable-ScheduledTask" } else { "Disable-ScheduledTask" };
    let script = format!(
        "{} -TaskPath '{}' -TaskName '{}' -ErrorAction Stop | Out-Null",
        comando,
        path.replace('\'', "''"),
        name.replace('\'', "''")
    );

    shell::powershell_checked(&script)
        .map_err(|e| format!("Não foi possível alterar `{}`: {}", name, e))?;

    Ok(ChangeRecord::ScheduledTask {
        path: path.to_string(),
        name: name.to_string(),
        // O que se guarda é o estado ANTERIOR, que é o que a reversão restaura.
        previously_enabled: !ligar,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminho_identifica_tarefa_do_windows() {
        assert!(e_da_microsoft("\\Microsoft\\Windows\\Defrag\\"));
        assert!(e_da_microsoft("\\microsoft\\windows\\"));
        assert!(!e_da_microsoft("\\GoogleUpdateTaskMachineUA\\"));
        assert!(!e_da_microsoft("\\"));
    }

    #[test]
    fn lista_de_terceiros_nao_traz_tarefa_do_windows() {
        // A barreira principal: tarefa do sistema não pode chegar à tela onde existe um botão de desligar.
        let tarefas = match listar_de_terceiros() {
            Ok(tarefas) => tarefas,
            Err(erro) => {
                println!("o Agendador não respondeu nesta máquina: {}", erro);
                return;
            }
        };

        for tarefa in tarefas {
            assert!(
                !e_da_microsoft(&tarefa.path),
                "tarefa do Windows entrou na lista de terceiros: {}{}",
                tarefa.path,
                tarefa.name
            );
        }
    }

    #[test]
    fn recusa_desligar_tarefa_do_windows() {
        let erro = definir_estado("\\Microsoft\\Windows\\Defrag\\", "ScheduledDefrag", false)
            .expect_err("deveria recusar tarefa do Windows");

        // A recusa por ser do Windows vem ANTES da checagem de administrador, senão um PC elevado desligaria tarefa
        // do sistema.
        assert!(erro.contains("Windows") || erro.contains("administrador"));
    }

    #[test]
    fn lista_esta_maquina() {
        let (todas, terceiros) = match (listar(), listar_de_terceiros()) {
            (Ok(todas), Ok(terceiros)) => (todas, terceiros),
            (Err(erro), _) | (_, Err(erro)) => {
                println!("o Agendador não respondeu nesta máquina: {}", erro);
                return;
            }
        };

        println!(
            "{} tarefas no total, {} de terceiros",
            todas.len(),
            terceiros.len()
        );
        for t in terceiros.iter().take(12) {
            println!(
                "  [{}] {}{}  (autor: {})",
                if t.enabled { "ligada" } else { "desligada" },
                t.path,
                t.name,
                t.author
            );
        }

        let estados: Vec<bool> = terceiros.iter().map(|t| t.enabled).collect();
        let primeira_desligada = estados.iter().position(|e| !e);
        if let Some(i) = primeira_desligada {
            assert!(
                estados[i..].iter().all(|e| !e),
                "a lista não está com as ligadas na frente"
            );
        }
    }
}
