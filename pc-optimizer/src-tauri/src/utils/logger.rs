// Logger utility
// Centralized logging system
//
// NA 2.0 O REGISTRO PASSOU A IR PARA UM ARQUIVO.
//
// Até a 1.9 isto só fazia `println!`, e num programa de janela no Windows a
// saída padrão não vai a lugar nenhum. Quando o "Otimizar agora" foi seguido de
// "os programas não abriam" no PC do dono, duas vezes, não havia como saber
// qual passo rodou, quanto demorou nem onde parou. O histórico de desfazer não
// ajuda nisso: ele só guarda o que deu certo.
//
// Agora cada linha vai também para `%APPDATA%\pc-optimizer\otimiza.log`, aberta,
// escrita e fechada na hora — um travamento no passo seguinte não leva a linha
// junto. O arquivo gira em 2 MB e guarda um anterior (`otimiza.log.1`).

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

/// Tamanho a partir do qual o arquivo gira.
const LIMITE_DO_ARQUIVO: u64 = 2 * 1024 * 1024;

/// Uma escrita por vez: duas threads anotando juntas podiam girar o arquivo
/// duas vezes ou intercalar pedaços de linha.
static ESCRITA: Mutex<()> = Mutex::new(());

pub struct Logger;

impl Logger {
    pub fn log(level: LogLevel, message: &str) {
        // Com milissegundos: dois passos do lote costumam cair no mesmo
        // segundo, e a ordem e a duração entre eles é o que se quer ler.
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let linha = format!("[{}] [{}] {}", timestamp, level, message);

        println!("{}", linha);

        if let Some(caminho) = Self::caminho() {
            // Não conseguir anotar não pode derrubar o que estava sendo anotado.
            let _ = anotar_em(&caminho, &linha, LIMITE_DO_ARQUIVO);
        }
    }

    /// Onde o registro mora.
    ///
    /// `None` nos testes: um teste que anotasse no `%APPDATA%` de quem roda
    /// misturaria linha de teste com o registro real da máquina.
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

/// Acrescenta uma linha ao arquivo, girando antes se ele já passou de `limite`.
///
/// Abre e fecha a cada linha de propósito. Manter o arquivo aberto seria mais
/// rápido, mas o que se quer daqui é justamente a última linha antes de um
/// travamento — e ela precisa já estar com o Windows quando o travamento vier.
fn anotar_em(caminho: &Path, linha: &str, limite: u64) -> std::io::Result<()> {
    let _vez = ESCRITA.lock().unwrap_or_else(|envenenado| envenenado.into_inner());

    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta)?;
    }

    if std::fs::metadata(caminho).map(|m| m.len() >= limite).unwrap_or(false) {
        // No Windows o `rename` substitui o anterior que já existir.
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
        // Lido ANTES da segunda linha: é a garantia de que um travamento logo
        // depois de "começou" ainda deixa o "começou" no disco.
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

        // O atual nunca cresce muito além do limite, e a linha mais recente
        // está nele — girar não pode perder justamente a última.
        assert!(atual.len() < 64 + 40, "o arquivo não girou: {} bytes", atual.len());
        assert!(atual.contains("linha 09"), "a última linha sumiu: {:?}", atual);
        assert!(!anterior.is_empty(), "o arquivo anterior não foi guardado");
    }

    #[test]
    fn os_testes_nao_anotam_no_registro_real() {
        assert!(Logger::caminho().is_none());
    }
}
