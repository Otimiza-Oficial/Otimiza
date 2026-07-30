// Tarefas agendadas
//
// O Windows executa dezenas de tarefas em segundo plano em horários que ninguém
// escolheu. As da Microsoft costumam ser leves e necessárias; as que chegam com
// programas de terceiros — atualizadores de Adobe, Google, Office, utilitários
// de fabricante — acordam sozinhas, sobem processo e somem, o dia inteiro.
//
// Num PC fraco isso é parte grande do "por que meu PC está lento sem eu abrir
// nada". Este módulo lista as de terceiros, permite desligar e — como tudo aqui
// — guarda o estado anterior para conseguir voltar atrás.

use super::shell;
use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub name: String,
    /// Pasta da tarefa na árvore do Agendador, terminando em barra invertida.
    pub path: String,
    pub author: String,
    pub enabled: bool,
    /// Se é da própria Microsoft. Elas ficam de fora da lista por padrão:
    /// desligar tarefa do sistema é como desligar serviço do sistema.
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

/// Se a tarefa pertence à Microsoft.
///
/// O caminho é o critério, não o autor: autor vem em branco ou traduzido em
/// parte das tarefas, enquanto o caminho `\Microsoft\` é estrutura fixa do
/// Windows em qualquer idioma.
pub fn e_da_microsoft(caminho: &str) -> bool {
    caminho.to_lowercase().starts_with("\\microsoft\\")
}

/// Lista as tarefas agendadas da máquina.
///
/// `Get-ScheduledTask` devolve nomes de propriedade em inglês em qualquer idioma
/// do Windows, ao contrário do `schtasks`, cujos cabeçalhos são traduzidos.
pub fn listar() -> Vec<ScheduledTask> {
    let script = "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-ScheduledTask | \
                  Select-Object TaskName,TaskPath,Author,@{n='State';e={$_.State.ToString()}})";

    let saida = match shell::run("powershell", &["-NoProfile", "-Command", script]) {
        Ok(o) if o.success && !o.stdout.trim().is_empty() => o.stdout,
        _ => return Vec::new(),
    };

    let brutas: Vec<RawTask> = match serde_json::from_str(&saida) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut tarefas: Vec<ScheduledTask> = brutas
        .into_iter()
        .filter_map(|t| {
            let name = t.task_name?;
            let path = t.task_path.unwrap_or_else(|| "\\".to_string());

            Some(ScheduledTask {
                microsoft: e_da_microsoft(&path),
                // "Disabled" é o único estado que interessa distinguir; os demais
                // (Ready, Running) significam habilitada.
                enabled: t.state.as_deref() != Some("Disabled"),
                author: t.author.unwrap_or_default(),
                name,
                path,
            })
        })
        .collect();

    // Ligadas primeiro: são as que ainda custam alguma coisa.
    tarefas.sort_by(|a, b| {
        b.enabled
            .cmp(&a.enabled)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    tarefas
}

/// Só as de terceiros — as que chegaram com programas instalados.
pub fn listar_de_terceiros() -> Vec<ScheduledTask> {
    listar().into_iter().filter(|t| !t.microsoft).collect()
}

/// Liga ou desliga uma tarefa, devolvendo o registro para o histórico.
pub fn definir_estado(path: &str, name: &str, ligar: bool) -> Result<ChangeRecord, String> {
    if !super::registry::is_elevated() {
        return Err("Mexer em tarefas agendadas exige executar como administrador.".to_string());
    }

    // Recusamos tarefas do Windows: desligar tarefa do sistema é da mesma
    // família de desligar serviço do sistema, e não é o que este produto faz.
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

    shell::run_checked("powershell", &["-NoProfile", "-Command", &script])
        .map_err(|e| format!("Não foi possível alterar `{}`: {}", name, e))?;

    Ok(ChangeRecord::ScheduledTask {
        path: path.to_string(),
        name: name.to_string(),
        // O que guardamos é o estado ANTERIOR, que é o oposto do que acabamos
        // de fazer — é ele que a reversão precisa restaurar.
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
        // A barreira principal: tarefa do sistema não pode chegar à tela onde
        // existe um botão de desligar.
        for tarefa in listar_de_terceiros() {
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

        // A recusa por ser do Windows precisa vir ANTES da checagem de
        // administrador, senão um PC elevado desligaria tarefa do sistema.
        assert!(erro.contains("Windows") || erro.contains("administrador"));
    }

    #[test]
    fn lista_esta_maquina() {
        let todas = listar();
        let terceiros = listar_de_terceiros();

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

        // Ligadas vêm primeiro: são as que ainda custam alguma coisa.
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
