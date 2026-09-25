// Devolve os processos suspensos por versões antigas quando a sessão do Windows termina. Thread suspensa não
// responde ao fim de sessão, o Windows não descarrega o UsrClass.dat, e na sessão seguinte o Explorer não abre
// (visto: 16 processos suspensos e 70 eventos 1512/1542 no logoff). Processo com janela não recebe o
// `CTRL_LOGOFF_EVENT`, e o `tao` desta versão não trata `WM_QUERYENDSESSION`: por isso um subclasse próprio
// (`SetWindowSubclass`, que encadeia com o do `tao`). Devolve já na consulta, para usar o prazo inteiro.

#[cfg(target_os = "windows")]
mod api {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{WM_ENDSESSION, WM_QUERYENDSESSION};

    /// Estável e sem colidir com o id interno do `tao`.
    const ID_SUBCLASSE: usize = 0x4F54_4D5A; // "OTMZ" em hex

    /// PRECISA ser rápida (prazo do Windows): `retomar_tudo` só percorre threads do próprio processo e um JSON pequeno.
    unsafe extern "system" fn subclasse(
        janela: HWND,
        mensagem: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id_subclasse: usize,
        _dados: usize,
    ) -> LRESULT {
        if mensagem == WM_QUERYENDSESSION || mensagem == WM_ENDSESSION {
            let _ = super::super::suspend::retomar_tudo();
        }

        // Sempre encadeia: é o que deixa o `tao` receber as mensagens dele.
        DefSubclassProc(janela, mensagem, wparam, lparam)
    }

    /// Ponteiro cru: o `HWND` do Tauri vem do crate `windows` e o daqui do `windows-sys`, mesmo `*mut c_void`.
    pub fn instalar(janela: *mut std::ffi::c_void) -> bool {
        let alvo: HWND = janela;
        unsafe { SetWindowSubclass(alvo, Some(subclasse), ID_SUBCLASSE, 0) != 0 }
    }
}

#[cfg(not(target_os = "windows"))]
mod api {
    pub fn instalar(_janela: *mut std::ffi::c_void) -> bool {
        false
    }
}

/// Falhar aqui não é fatal: as outras redes (ao fechar, na próxima abertura, prazo máximo) continuam.
pub fn instalar(janela: *mut std::ffi::c_void) -> bool {
    api::instalar(janela)
}

#[cfg(test)]
mod tests {
    #[test]
    fn nao_mata_processo() {
        // Este arquivo decide QUANDO devolver, nunca mata.
        let fonte = include_str!("sessao.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();

        for proibido in ["TerminateProcess", "Stop-Process", "taskkill", "ExitProcess"] {
            assert!(
                !producao.contains(proibido),
                "`{}` apareceu no módulo de fim de sessão",
                proibido
            );
        }
    }

    #[test]
    fn instalar_em_janela_nula_nao_estoura() {
        // Sem sessão gráfica na esteira: ponteiro nulo devolve "não instalou", sem pânico.
        assert!(!super::instalar(std::ptr::null_mut()));
    }
}
