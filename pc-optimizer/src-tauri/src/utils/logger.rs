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

/// **Pura.** Tira o nome da pasta do usuário dos caminhos (`C:\Users\<nome>`): o log vai para o suporte pelo Discord,
/// e o nome do usuário do Windows costuma ser o nome da pessoa.
pub fn anonimizar(texto: &str, usuario: &str) -> String {
    let usuario = usuario.trim();
    if usuario.is_empty() {
        return texto.to_string();
    }
    let mut saida = texto.to_string();
    for separador in ["\\", "/"] {
        for pasta in ["Users", "users", "USERS"] {
            let para = format!("{pasta}{separador}<usuario>");
            // O Windows não diferencia maiúsculas: "snyx" e "SnyX" são a mesma pasta.
            for nome in [usuario.to_string(), usuario.to_lowercase(), usuario.to_uppercase()] {
                saida = saida.replace(&format!("{pasta}{separador}{nome}"), &para);
            }
        }
    }
    saida
}

/// Junta o registro anterior (`otimiza.log.1`, se houver) e o atual num arquivo só, anonimizado, para o suporte.
pub fn juntar_para_suporte(log: &Path, destino: &Path, usuario: &str) -> std::io::Result<()> {
    let _vez = ESCRITA
        .lock()
        .unwrap_or_else(|envenenado| envenenado.into_inner());
    let mut texto = String::new();
    if let Ok(anterior) = std::fs::read(log.with_extension("log.1")) {
        texto.push_str(&String::from_utf8_lossy(&anterior));
    }
    texto.push_str(&String::from_utf8_lossy(&std::fs::read(log)?));
    std::fs::write(destino, anonimizar(&texto, usuario))
}

/// Abre e fecha a cada linha de propósito: a última linha antes de um travamento precisa já estar no disco.
fn anotar_em(caminho: &Path, linha: &str, limite: u64) -> std::io::Result<()> {
    let _vez = ESCRITA
        .lock()
        .unwrap_or_else(|envenenado| envenenado.into_inner());

    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta)?;
    }

    if std::fs::metadata(caminho)
        .map(|m| m.len() >= limite)
        .unwrap_or(false)
    {
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

    #[test]
    fn o_log_para_o_suporte_sai_sem_o_nome_do_usuario() {
        let t = r"lendo C:\Users\SnyX\AppData\Local\x e c:\users\snyx\Downloads, e C:/Users/SnyX/y";
        let a = anonimizar(t, "SnyX");
        assert!(!a.to_lowercase().contains("snyx"), "{}", a);
        assert!(a.contains(r"Users\<usuario>\AppData"), "{}", a);
        assert_eq!(anonimizar("nada aqui", ""), "nada aqui");
    }

    #[test]
    fn junta_o_anterior_e_o_atual_em_ordem() {
        let pasta = pasta_de_teste("suporte");
        std::fs::create_dir_all(&pasta).unwrap();
        let log = pasta.join("otimiza.log");
        std::fs::write(log.with_extension("log.1"), "antigo C:\\Users\\Ana\\x\n").unwrap();
        std::fs::write(&log, "novo\n").unwrap();
        let destino = pasta.join("saida.txt");
        juntar_para_suporte(&log, &destino, "Ana").unwrap();
        let s = std::fs::read_to_string(&destino).unwrap();
        assert!(s.starts_with("antigo") && s.ends_with("novo\n") && !s.contains("Ana"), "{}", s);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    fn pasta_de_teste(nome: &str) -> PathBuf {
        let unico = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        std::env::temp_dir().join(format!(
            "otimiza-logger-{}-{}-{}",
            nome,
            std::process::id(),
            unico
        ))
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
            anotar_em(
                &caminho,
                &format!("linha {:02} com algum texto", numero),
                64,
            )
            .unwrap();
        }

        let atual = std::fs::read_to_string(&caminho).unwrap();
        let anterior = std::fs::read_to_string(caminho.with_extension("log.1")).unwrap();
        let _ = std::fs::remove_dir_all(&pasta);

        // Girar não pode perder justamente a linha mais recente.
        assert!(
            atual.len() < 64 + 40,
            "o arquivo não girou: {} bytes",
            atual.len()
        );
        assert!(
            atual.contains("linha 09"),
            "a última linha sumiu: {:?}",
            atual
        );
        assert!(!anterior.is_empty(), "o arquivo anterior não foi guardado");
    }

    #[test]
    fn os_testes_nao_anotam_no_registro_real() {
        assert!(Logger::caminho().is_none());
    }
}
