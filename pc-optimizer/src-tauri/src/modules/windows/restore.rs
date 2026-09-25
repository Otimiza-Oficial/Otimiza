// Pontos de restauração, a rede de baixo do histórico. A Proteção do Sistema vem desligada em muitas máquinas e o
// Windows recusa mais de um ponto a cada 24 h, os dois em silêncio: por isso os pontos são contados antes e
// depois, em vez de confiar no comando.

use super::shell;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Normalmente de 15 s a um minuto; três minutos cobrem HD ocupado. Depois disso o lote segue sem o ponto.
const PRAZO_DO_PONTO: Duration = Duration::from_secs(180);

/// Imagens "lite" costumam vir com os dois desativados: pedir o ponto assim só deixa o cliente esperando.
const SERVICOS_DO_PONTO: [(&str, &str); 2] = [
    ("VSS", "Cópia de Sombra de Volume"),
    ("swprv", "Provedor de Cópia de Sombra de Software da Microsoft"),
];

fn mensagem_de_servicos_desativados(nomes: &[&str]) -> String {
    let sujeito = match nomes {
        [um] => format!("o serviço \"{}\" está desativado", um),
        [inicio @ .., ultimo] => format!(
            "os serviços {} e \"{}\" estão desativados",
            inicio
                .iter()
                .map(|nome| format!("\"{}\"", nome))
                .collect::<Vec<_>>()
                .join(", "),
            ultimo
        ),
        [] => return String::new(),
    };

    format!(
        "Ponto de restauração não criado: {} neste Windows, e sem isso o Windows \
         não cria ponto de restauração. O Otimiza seguiu sem ele; suas otimizações \
         continuam reversíveis pelo histórico do Otimiza.",
        sujeito
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorePoint {
    pub sequence: u32,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreStatus {
    pub available: bool,
    pub message: String,
    pub points: Vec<RestorePoint>,
}

/// Nomes de propriedade do WMI são iguais em qualquer idioma, ao contrário do texto dos comandos.
fn query(script: &str) -> Option<String> {
    let output = shell::powershell(script).ok()?;

    if output.success {
        Some(output.stdout)
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawPoint {
    sequence_number: Option<u32>,
    description: Option<String>,
    creation_time: Option<String>,
}

pub fn list() -> Vec<RestorePoint> {
    let script = "ConvertTo-Json -Compress -Depth 2 -InputObject @(Get-CimInstance \
                  -Namespace root/default -ClassName SystemRestore -ErrorAction SilentlyContinue | \
                  Select-Object SequenceNumber,Description,CreationTime)";

    let raw: Vec<RawPoint> = query(script)
        .filter(|json| !json.trim().is_empty())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();

    let mut points: Vec<RestorePoint> = raw
        .into_iter()
        .filter_map(|point| {
            Some(RestorePoint {
                sequence: point.sequence_number?,
                description: point.description.unwrap_or_default(),
                created_at: format_wmi_date(point.creation_time.as_deref().unwrap_or("")),
            })
        })
        .collect();

    points.sort_by(|a, b| b.sequence.cmp(&a.sequence));
    points
}

pub fn format_wmi_date(raw: &str) -> String {
    if raw.len() < 14 || !raw[..14].chars().all(|c| c.is_ascii_digit()) {
        return raw.to_string();
    }

    format!(
        "{}/{}/{} {}:{}",
        &raw[6..8],   // dia
        &raw[4..6],   // mês
        &raw[0..4],   // ano
        &raw[8..10],  // hora
        &raw[10..12], // minuto
    )
}

/// Nunca afirma que criou sem ter conferido.
pub fn create(description: &str) -> Result<String, String> {
    if !super::registry::is_elevated() {
        return Err("Criar ponto de restauração exige executar como administrador.".to_string());
    }

    // Serviço ilegível NÃO conta como desativado: tenta e confere.
    let desativados: Vec<&str> = SERVICOS_DO_PONTO
        .iter()
        .filter(|(servico, _)| {
            matches!(super::services::query_start_type(servico).as_deref(), Ok("disabled"))
        })
        .map(|(_, nome)| *nome)
        .collect();

    if !desativados.is_empty() {
        return Err(mensagem_de_servicos_desativados(&desativados));
    }

    let before = list();
    let highest_before = before.first().map(|point| point.sequence).unwrap_or(0);

    let script = format!(
        "Checkpoint-Computer -Description '{}' -RestorePointType MODIFY_SETTINGS",
        description.replace('\'', "''")
    );
    let pedido = shell::powershell_com_prazo(&script, PRAZO_DO_PONTO);

    let after = list();
    let highest_after = after.first().map(|point| point.sequence).unwrap_or(0);

    // Conferido ANTES de ver se estourou o prazo: o ponto pode ter saído logo depois de o Otimiza desistir.
    if highest_after > highest_before {
        return Ok(format!(
            "Ponto de restauração criado ({} no total).",
            after.len()
        ));
    }

    if let Err(erro) = pedido {
        return Err(format!(
            "Ponto de restauração não confirmado: o pedido ao Windows não terminou \
             ({}). O Otimiza seguiu sem ele; suas otimizações continuam reversíveis \
             pelo histórico do Otimiza.",
            erro
        ));
    }

    if before.is_empty() {
        Err("Não foi possível criar: a Proteção do Sistema está desligada neste PC. \
             Ative em \"Criar um ponto de restauração\" nas configurações do Windows, \
             ou use o botão abaixo."
            .to_string())
    } else {
        Err("Não foi possível criar: o Windows só permite um ponto de restauração a \
             cada 24 horas, e já existe um recente. Suas otimizações continuam \
             reversíveis pelo histórico do Otimiza."
            .to_string())
    }
}

/// Consome disco: ação explícita do usuário, nunca automática.
pub fn enable_protection() -> Result<String, String> {
    if !super::registry::is_elevated() {
        return Err("Ativar a Proteção do Sistema exige executar como administrador.".to_string());
    }

    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());

    shell::powershell_checked(&format!("Enable-ComputerRestore -Drive '{}\\'", system_drive))?;

    Ok(format!(
        "Proteção do Sistema ativada em {}. Agora dá para criar pontos de restauração.",
        system_drive
    ))
}

/// Sem administrador a lista é negada: vazia aí NÃO prova Proteção desligada, só que não se conseguiu olhar.
pub fn status() -> RestoreStatus {
    if !super::registry::is_elevated() {
        return RestoreStatus {
            available: false,
            message: "O Windows não deixa consultar pontos de restauração sem \
                      administrador. Reabra como administrador para ver e criar."
                .to_string(),
            points: Vec::new(),
        };
    }

    let points = list();

    if points.is_empty() {
        return RestoreStatus {
            available: false,
            message: "Nenhum ponto de restauração neste PC: a Proteção do Sistema está \
                      desligada, que é o padrão em muitas instalações do Windows."
                .to_string(),
            points,
        };
    }

    RestoreStatus {
        available: true,
        message: format!("{} ponto(s) de restauração disponível(is).", points.len()),
        points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_wmi_date() {
        assert_eq!(format_wmi_date("20260729143005.000000-180"), "29/07/2026 14:30");
    }

    #[test]
    fn servico_desativado_e_dito_pelo_nome_e_no_plural_certo() {
        let um = mensagem_de_servicos_desativados(&["Cópia de Sombra de Volume"]);
        assert!(um.contains("o serviço \"Cópia de Sombra de Volume\" está desativado"), "{}", um);

        let dois = mensagem_de_servicos_desativados(&["A", "B"]);
        assert!(dois.contains("os serviços \"A\" e \"B\" estão desativados"), "{}", dois);

        assert!(um.contains("continuam reversíveis"));
    }

    #[test]
    fn leaves_unexpected_date_untouched() {
        assert_eq!(format_wmi_date("sem-data"), "sem-data");
        assert_eq!(format_wmi_date(""), "");
    }

    #[test]
    fn reads_restore_status_of_this_machine() {
        let status = status();
        println!("disponível: {} — {}", status.available, status.message);

        for point in status.points.iter().take(5) {
            println!("  #{} {} · {}", point.sequence, point.created_at, point.description);
        }

        assert_eq!(status.available, !status.points.is_empty());

        if !super::super::registry::is_elevated() {
            assert!(
                status.message.contains("administrador"),
                "mensagem esconde o motivo real: {}",
                status.message
            );
            assert!(
                !status.message.contains("está desligada"),
                "afirma o que não foi verificado: {}",
                status.message
            );
        }
    }
}
