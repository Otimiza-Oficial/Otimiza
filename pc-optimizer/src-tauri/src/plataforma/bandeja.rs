// O Otimiza na bandeja do Windows (perto do relógio). O contador no lugar do `cl_drawfps` e o Frame Boost são janelas
// do próprio Otimiza: fechar o programa no meio do jogo apagava o FPS da tela (o do FiveM já estava desligado). Com um
// jogo aberto, fechar a janela só a esconde; o ícone da bandeja reabre ou sai de vez, e quando o jogo fecha o Otimiza
// que ficou só na bandeja sai sozinho, para não ficar rodando esquecido.

use std::sync::atomic::{AtomicBool, Ordering};

static FECHADO_COM_JOGO: AtomicBool = AtomicBool::new(false);

/// Pura: fechar a janela vira "esconder na bandeja"? Só com um jogo aberto.
pub fn esconde_ao_fechar(jogo_aberto: bool) -> bool {
    jogo_aberto
}

/// A pessoa fechou a janela durante o jogo e ela ficou só na bandeja.
pub fn fechado_com_jogo() -> bool {
    FECHADO_COM_JOGO.load(Ordering::SeqCst)
}

#[cfg(target_os = "windows")]
fn mostrar(app: &tauri::AppHandle) {
    use tauri::Manager;
    FECHADO_COM_JOGO.store(false, Ordering::SeqCst);
    if let Some(j) = app.get_webview_window("main") {
        let _ = j.show();
        let _ = j.unminimize();
        let _ = j.set_focus();
    }
}

/// O ícone, criado uma vez na abertura.
#[cfg(target_os = "windows")]
pub fn criar(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let abrir = MenuItem::with_id(app, "abrir", "Abrir o Otimiza", true, None::<&str>)?;
    let sair = MenuItem::with_id(
        app,
        "sair",
        "Sair (o FPS na tela e o Frame Boost desligam)",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&abrir, &sair])?;
    let mut b = TrayIconBuilder::with_id("otimiza")
        .tooltip("Otimiza")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "abrir" => mostrar(app),
            "sair" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|icone, e| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                mostrar(icone.app_handle());
            }
        });
    if let Some(i) = app.default_window_icon() {
        b = b.icon(i.clone());
    }
    b.build(app)?;
    Ok(())
}

/// No "fechar" da janela principal: com jogo aberto, esconde em vez de fechar.
#[cfg(target_os = "windows")]
pub fn ao_pedir_fechar(janela: &tauri::Window, api: &tauri::CloseRequestApi) {
    if janela.label() != "main" || !esconde_ao_fechar(super::sessao::atual().is_some()) {
        return;
    }
    api.prevent_close();
    let _ = janela.hide();
    if !FECHADO_COM_JOGO.swap(true, Ordering::SeqCst) {
        crate::utils::Logger::info(
            "janela fechada com o jogo aberto: o Otimiza fica na bandeja até o jogo fechar",
        );
    }
}

/// Uma cópia só do Otimiza. Com ele escondido na bandeja, o atalho abria outra cópia (com outro contador por cima do
/// jogo); agora a segunda traz a primeira para a frente e sai. `true` = já havia uma aberta.
#[cfg(target_os = "windows")]
pub fn ja_aberto_traz_para_frente() -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };
    let largo = |s: &str| {
        s.encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    };
    let nome = largo("Local\\OtimizaConsoleDeDesempenho");
    unsafe {
        // O mutex fica aberto até o processo acabar (o Windows fecha o identificador).
        let m = CreateMutexW(std::ptr::null(), 0, nome.as_ptr());
        if m.is_null() || GetLastError() != ERROR_ALREADY_EXISTS {
            return false;
        }
        let titulo = largo("Otimiza — Console de Desempenho");
        let janela = FindWindowW(std::ptr::null(), titulo.as_ptr());
        if !janela.is_null() {
            ShowWindow(janela, SW_SHOW);
            ShowWindow(janela, SW_RESTORE);
            SetForegroundWindow(janela);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_esconde_com_jogo_aberto() {
        assert!(esconde_ao_fechar(true));
        assert!(!esconde_ao_fechar(false), "sem jogo, fechar fecha");
    }
}
