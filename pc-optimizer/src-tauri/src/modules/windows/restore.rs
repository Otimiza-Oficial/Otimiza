// Pontos de restauração do Windows
//
// Nosso histórico de mudanças já desfaz item por item, com o valor exato que
// existia antes. O ponto de restauração é a rede de segurança de baixo dela: se
// algo der errado de um jeito que não previmos, o cliente volta o Windows inteiro.
//
// Duas armadilhas que este módulo trata de frente:
//
// 1. A Proteção do Sistema vem DESLIGADA em muitas instalações do Windows 10 e 11.
//    Pedir um ponto de restauração nessas máquinas falha silenciosamente — e um
//    produto que anuncia "criamos um ponto de restauração" sem verificar está
//    vendendo uma segurança que não existe.
// 2. O Windows recusa criar mais de um ponto a cada 24 horas. A recusa também é
//    silenciosa.
//
// Por isso não confiamos no comando: contamos os pontos antes e depois e
// verificamos se um novo apareceu de verdade. É independente do idioma do Windows.

use super::shell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorePoint {
    pub sequence: u32,
    pub description: String,
    /// Data no formato do Windows (AAAAMMDDhhmmss), já legível.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreStatus {
    /// Se conseguimos criar um ponto de restauração agora.
    pub available: bool,
    /// Explicação em português do estado atual.
    pub message: String,
    pub points: Vec<RestorePoint>,
}

/// Consulta o WMI devolvendo JSON. Nomes de propriedade do WMI são estáveis em
/// qualquer idioma, ao contrário do texto formatado dos comandos.
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

/// Pontos de restauração existentes, do mais recente para o mais antigo.
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

/// Converte a data do WMI (AAAAMMDDhhmmss...) para algo legível.
/// Exposto para teste: formato de data é onde parsing costuma quebrar calado.
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

/// Cria um ponto de restauração e CONFIRMA que ele existe.
///
/// Devolve a descrição do que aconteceu, sempre verdadeira: nunca afirma que
/// criou sem ter conferido.
pub fn create(description: &str) -> Result<String, String> {
    if !super::registry::is_elevated() {
        return Err("Criar ponto de restauração exige executar como administrador.".to_string());
    }

    let before = list();
    let highest_before = before.first().map(|point| point.sequence).unwrap_or(0);

    // O comando pode demorar; o Windows tira um instantâneo do volume.
    let script = format!(
        "Checkpoint-Computer -Description '{}' -RestorePointType MODIFY_SETTINGS",
        description.replace('\'', "''")
    );
    let _ = shell::powershell(&script);

    let after = list();
    let highest_after = after.first().map(|point| point.sequence).unwrap_or(0);

    if highest_after > highest_before {
        return Ok(format!(
            "Ponto de restauração criado ({} no total).",
            after.len()
        ));
    }

    // Nada apareceu. As duas causas prováveis, ditas com clareza em vez de um
    // "falhou" genérico que não ajuda ninguém.
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

/// Liga a Proteção do Sistema no disco do Windows.
///
/// Consome espaço em disco para guardar os instantâneos — por isso é ação
/// explícita do usuário, nunca automática.
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

/// Estado atual, para a interface mostrar sem prometer nada.
///
/// Sem administrador o Windows nega a leitura da lista. Uma lista vazia nessa
/// situação NÃO prova que a Proteção do Sistema está desligada — prova apenas que
/// não conseguimos olhar. Afirmar o primeiro seria exatamente o tipo de chute
/// disfarçado de diagnóstico que este produto existe para não fazer.
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
    fn leaves_unexpected_date_untouched() {
        // Melhor mostrar o valor cru que inventar uma data errada.
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

        // A consistência importa: dizer "disponível" sem ponto nenhum seria
        // prometer uma segurança que não existe.
        assert_eq!(status.available, !status.points.is_empty());

        // Sem elevação, a mensagem precisa falar de permissão — e nunca afirmar
        // que a Proteção do Sistema está desligada, porque não temos como saber.
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
