pub mod arquivo;
pub mod diagnostico;
pub mod logger;
pub mod webview;

pub use logger::Logger;

/// O resultado de uma tarefa de fundo (`spawn_blocking(..).await`). Um panic lá dentro perdia só aquela volta do laço,
/// mas sumia sem rastro (`let _ =`): agora fica no log com o nome do laço (1.0.7).
pub fn avisar_panico<T>(laco: &str, resultado: Result<T, tokio::task::JoinError>) {
    if let Err(e) = resultado {
        if e.is_panic() {
            let msg = e
                .into_panic()
                .downcast::<String>()
                .map(|s| *s)
                .or_else(|p| p.downcast::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|_| "sem mensagem".to_string());
            crate::utils::Logger::error(&format!("laço de fundo `{}`: panic ({}); segue na próxima volta", laco, msg));
        } else {
            crate::utils::Logger::warn(&format!("laço de fundo `{}`: tarefa cancelada ({})", laco, e));
        }
    }
}
