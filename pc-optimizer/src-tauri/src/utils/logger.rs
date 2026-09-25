// Registro em arquivo: `%APPDATA%\pc-optimizer\otimiza.log`, aberto, escrito e fechado a cada linha, para
// que um travamento no passo seguinte não leve a linha junto. Gira em 2 MB e guarda um anterior.

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

const LIMITE_DO_ARQUIVO: u64 = 2 * 1024 * 1024;

/// Uma escrita por vez: duas threads podiam girar o arquivo duas vezes ou intercalar pedaços de linha.
static ESCRITA: Mutex<()> = Mutex::new(());

pub struct Logger;

impl Logger {
    pub fn log(level: LogLevel, message: &str) {
        // Com milissegundos: dois passos do lote costumam cair no mesmo segundo.
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let linha = format!("[{}] [{}] {}", timestamp, level, message);

        println!("{}", linha);

        if let Some(caminho) = Self::caminho() {
            // Não conseguir anotar não pode derrubar o que estava sendo anotado.
            let _ = anotar_em(&caminho, &linha, LIMITE_DO_ARQUIVO);
        }
    }

    /// `None` nos testes: linha de teste não se mistura com o registro real da máquina.
    pub fn caminho() -> Option<PathBuf> {
        if cfg!(test) {
            return None;
        }

        let base = std::env::var("APPDATA").ok()?;
        Some(PathBuf::from(base).join("pc-optimizer").join("otimiza.log"))
    }

    pub fn info(message: &str) {
        Self::log(LogLevel::Info, message);
    }

    pub fn warn(message: &str) {
        Self::log(LogLevel::Warn, message);
    }

    pub fn error(message: &str) {
        Self::log(LogLevel::Error, message);
    }
}

/// Abre e fecha a cada linha de propósito: a última linha antes de um travamento precisa já estar no disco.
fn anotar_em(caminho: &Path, linha: &str, limite: u64) -> std::io::Result<()> {
    let _vez = ESCRITA.lock().unwrap_or_else(|envenenado| envenenado.into_inner());

    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta)?;
    }

    if std::fs::metadata(caminho).map(|m| m.len() >= limite).unwrap_or(false) {
        std::fs::rename(caminho, caminho.with_extension("log.1"))?;
    }

    let mut arquivo = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(caminho)?;

    writeln!(arquivo, "{}", linha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pasta_de_teste(nome: &str) -> PathBuf {
        let unico = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        std::env::temp_dir().join(format!("otimiza-logger-{}-{}-{}", nome, std::process::id(), unico))
    }

    #[test]
    fn cada_linha_chega_ao_arquivo_na_hora_e_em_ordem() {
        let pasta = pasta_de_teste("ordem");
        let caminho = pasta.join("otimiza.log");

        anotar_em(&caminho, "aplicar `x`: começou", LIMITE_DO_ARQUIVO).unwrap();
        // Lido ANTES da segunda linha: um travamento logo depois de "começou" ainda deixa o "começou" no disco.
        let depois_da_primeira = std::fs::read_to_string(&caminho).unwrap();
        anotar_em(&caminho, "aplicar `x`: terminou", LIMITE_DO_ARQUIVO).unwrap();
        let conteudo = std::fs::read_to_string(&caminho).unwrap();
        let _ = std::fs::remove_dir_all(&pasta);

        assert_eq!(depois_da_primeira.trim(), "aplicar `x`: começou");
        assert_eq!(
            conteudo.lines().collect::<Vec<_>>(),
            vec!["aplicar `x`: começou", "aplicar `x`: terminou"]
        );
    }

    #[test]
    fn o_arquivo_gira_quando_passa_do_limite_e_guarda_o_anterior() {
        let pasta = pasta_de_teste("giro");
        let caminho = pasta.join("otimiza.log");

        for numero in 0..10 {
            anotar_em(&caminho, &format!("linha {:02} com algum texto", numero), 64).unwrap();
        }

        let atual = std::fs::read_to_string(&caminho).unwrap();
        let anterior = std::fs::read_to_string(caminho.with_extension("log.1")).unwrap();
        let _ = std::fs::remove_dir_all(&pasta);

        // Girar não pode perder justamente a linha mais recente.
        assert!(atual.len() < 64 + 40, "o arquivo não girou: {} bytes", atual.len());
        assert!(atual.contains("linha 09"), "a última linha sumiu: {:?}", atual);
        assert!(!anterior.is_empty(), "o arquivo anterior não foi guardado");
    }

    #[test]
    fn os_testes_nao_anotam_no_registro_real() {
        assert!(Logger::caminho().is_none());
    }
}
