// Limpeza de arquivos temporários
//
// Única operação do catálogo que NÃO é reversível — arquivo apagado não volta.
// Por isso ela: só toca em pastas de temporários, nunca falha o lote inteiro por
// causa de um arquivo travado, e informa quantos MB realmente liberou.

use std::fs;
use std::path::{Path, PathBuf};

pub struct CleanupResult {
    pub bytes_freed: u64,
    pub files_removed: usize,
    /// Arquivos em uso por programas abertos. Não é erro: são pulados.
    pub files_skipped: usize,
}

/// Pastas de temporários seguras para limpar.
/// Nada fora daqui é tocado — sem "limpeza de registro", sem apagar downloads,
/// sem mexer em pasta de usuário.
fn temp_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();

    if let Ok(temp) = std::env::var("TEMP") {
        directories.push(PathBuf::from(temp));
    }

    if let Ok(windir) = std::env::var("SystemRoot") {
        directories.push(PathBuf::from(windir).join("Temp"));
    }

    directories
}

/// Soma o tamanho do que pode ser limpo, sem apagar nada.
/// Usado para mostrar o ganho antes do usuário decidir.
pub fn estimate() -> u64 {
    temp_directories()
        .iter()
        .filter(|dir| dir.exists())
        .map(|dir| directory_size(dir))
        .sum()
}

fn directory_size(dir: &Path) -> u64 {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| match entry.metadata() {
            Ok(metadata) if metadata.is_dir() => directory_size(&entry.path()),
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        })
        .sum()
}

/// Apaga o conteúdo das pastas de temporários.
/// Arquivos em uso são pulados em silêncio: travar a limpeza porque o Chrome está
/// com um arquivo aberto seria pior que deixar esse arquivo para trás.
pub fn run() -> CleanupResult {
    let mut result = CleanupResult {
        bytes_freed: 0,
        files_removed: 0,
        files_skipped: 0,
    };

    for directory in temp_directories().iter().filter(|dir| dir.exists()) {
        clean_directory(directory, &mut result);
    }

    result
}

fn clean_directory(dir: &Path, result: &mut CleanupResult) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                result.files_skipped += 1;
                continue;
            }
        };

        if metadata.is_dir() {
            // O tamanho é medido antes de remover, senão não há o que somar depois.
            let size = directory_size(&path);
            match fs::remove_dir_all(&path) {
                Ok(()) => {
                    result.bytes_freed += size;
                    result.files_removed += 1;
                }
                Err(_) => result.files_skipped += 1,
            }
        } else {
            match fs::remove_file(&path) {
                Ok(()) => {
                    result.bytes_freed += metadata.len();
                    result.files_removed += 1;
                }
                Err(_) => result.files_skipped += 1,
            }
        }
    }
}

/// Pasta onde o Windows guarda os instaladores já usados das atualizações.
///
/// Depois que uma atualização é instalada, o instalador continua ali ocupando
/// espaço. Em PC com SSD pequeno isso vira gigabytes parados — e é a limpeza que
/// mais devolve espaço sem risco nenhum.
fn update_cache_dir() -> Option<PathBuf> {
    let windows = std::env::var("SystemRoot").ok()?;
    Some(PathBuf::from(windows).join("SoftwareDistribution").join("Download"))
}

pub fn estimate_update_cache() -> u64 {
    update_cache_dir()
        .filter(|dir| dir.exists())
        .map(|dir| directory_size(&dir))
        .unwrap_or(0)
}

/// Limpa o cache de atualizações do Windows.
///
/// Os serviços de atualização precisam parar antes: apagar com eles rodando
/// deixaria arquivos travados para trás e, pior, poderia confundir uma
/// atualização em andamento. Eles voltam ao final, sempre — mesmo se a limpeza
/// falhar no meio.
pub fn run_update_cache() -> Result<CleanupResult, String> {
    let dir = update_cache_dir().ok_or("Não foi possível localizar a pasta do Windows.")?;

    if !dir.exists() {
        return Ok(CleanupResult {
            bytes_freed: 0,
            files_removed: 0,
            files_skipped: 0,
        });
    }

    // Guardar o estado anterior de cada serviço evita deixar desligado o que já
    // estava desligado por decisão do usuário.
    let servicos = ["wuauserv", "bits"];
    let mut estavam_rodando = Vec::new();

    for servico in servicos {
        let rodando = super::services::is_running(servico);
        estavam_rodando.push(rodando);

        if rodando {
            let _ = super::services::stop(servico);
        }
    }

    let mut result = CleanupResult {
        bytes_freed: 0,
        files_removed: 0,
        files_skipped: 0,
    };
    clean_directory(&dir, &mut result);

    for (servico, estava_rodando) in servicos.iter().zip(estavam_rodando) {
        if estava_rodando {
            let _ = super::services::start(servico);
        }
    }

    Ok(result)
}

/// Formata bytes para exibição ao usuário.
pub fn format_size(bytes: u64) -> String {
    const GB: f64 = 1_073_741_824.0;
    const MB: f64 = 1_048_576.0;

    let bytes = bytes as f64;

    if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else {
        format!("{:.0} MB", bytes / MB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_temp_directories_are_targeted() {
        let directories = temp_directories();

        assert!(!directories.is_empty(), "nenhuma pasta de temporários encontrada");
        for directory in &directories {
            let path = directory.to_string_lossy().to_lowercase();
            assert!(
                path.contains("temp"),
                "pasta fora do escopo de temporários: {}",
                path
            );
        }
    }

    #[test]
    fn formats_sizes_for_humans() {
        assert_eq!(format_size(524_288_000), "500 MB");
        assert_eq!(format_size(2_147_483_648), "2.0 GB");
    }

    #[test]
    fn estimate_reads_without_deleting() {
        // A estimativa precisa ser segura de chamar a qualquer momento: ela roda
        // toda vez que a lista de otimizações é carregada.
        let before = estimate();
        let after = estimate();
        assert_eq!(before > 0, after > 0);
    }
}
