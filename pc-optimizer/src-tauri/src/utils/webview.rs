// A janela do Otimiza é uma página do WebView2. Quando o processo que desenha a página cai (driver de vídeo, memória,
// antivírus), o WebView2 deixa a janela em branco ou congelada e ninguém fica sabendo: o log não tinha uma linha.
// Aqui cada falha vira linha no log, e a página é recarregada quando dá; quando o próprio navegador interno morre, a
// janela não tem mais como se recuperar e o programa reabre.

#[cfg(target_os = "windows")]
use tauri::Manager;

#[cfg(target_os = "windows")]
pub fn vigiar(janela: &tauri::WebviewWindow) {
    let handle = janela.app_handle().clone();
    let resultado = janela.with_webview(move |pw| {
        if let Err(e) = unsafe { instalar(pw.controller(), handle) } {
            super::Logger::warn(&format!("vigia do WebView2 não ligou: {}", e));
        }
    });
    if let Err(e) = resultado {
        super::Logger::warn(&format!("vigia do WebView2 não ligou: {}", e));
    }
}

#[cfg(target_os = "windows")]
static RECARGAS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// O que fazer com cada tipo de falha. Pura, para o teste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reacao {
    /// O processo da página caiu ou parou de responder: recarregar recria.
    Recarregar,
    /// O navegador interno morreu: a janela não volta sem reabrir o programa.
    Reabrir,
    /// Processo auxiliar (placa de vídeo, utilitário): o WebView2 refaz sozinho.
    SoAnotar,
}

pub fn reacao(tipo: i32) -> Reacao {
    // Valores de COREWEBVIEW2_PROCESS_FAILED_KIND.
    match tipo {
        0 => Reacao::Reabrir,                // BROWSER_PROCESS_EXITED
        1 | 2 | 3 => Reacao::Recarregar,     // RENDER_PROCESS_EXITED, RENDER_PROCESS_UNRESPONSIVE, FRAME_RENDER_PROCESS_EXITED
        _ => Reacao::SoAnotar,
    }
}

#[cfg(target_os = "windows")]
unsafe fn instalar(
    controlador: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller,
    handle: tauri::AppHandle,
) -> windows::core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_PROCESS_FAILED_KIND;
    use webview2_com::ProcessFailedEventHandler;

    let nucleo = controlador.CoreWebView2()?;
    let mut token = 0i64;
    nucleo.add_ProcessFailed(
        &ProcessFailedEventHandler::create(Box::new(move |webview, argumentos| {
            let mut tipo = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
            if let Some(a) = argumentos {
                let _ = a.ProcessFailedKind(&mut tipo);
            }
            let r = reacao(tipo.0);
            super::Logger::error(&format!("WebView2: processo falhou (tipo {}); reação: {:?}", tipo.0, r));
            match r {
                // Três por sessão: página que cai toda vez que abre recarregaria sem parar.
                Reacao::Recarregar if RECARGAS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 3 => {
                    if let Some(w) = webview {
                        let _ = w.Reload();
                    }
                }
                // No modo seguro não reabre: reabrir em laço é pior que a janela parada, e a pessoa fecha.
                Reacao::Reabrir if !super::diagnostico::modo_seguro() => {
                    super::diagnostico::sair_por_falha();
                    // `request_restart`, não `restart`: passa pelo fechamento normal (devoluções do `RunEvent::Exit`).
                    handle.request_restart();
                }
                Reacao::Recarregar | Reacao::Reabrir => {}
                Reacao::SoAnotar => {}
            }
            Ok(())
        })),
        &mut token,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_falha_tem_a_reacao_que_a_recupera() {
        assert_eq!(reacao(0), Reacao::Reabrir);
        assert_eq!(reacao(1), Reacao::Recarregar);
        assert_eq!(reacao(2), Reacao::Recarregar);
        assert_eq!(reacao(3), Reacao::Recarregar);
        assert_eq!(reacao(4), Reacao::SoAnotar);
        assert_eq!(reacao(99), Reacao::SoAnotar);
    }
}
