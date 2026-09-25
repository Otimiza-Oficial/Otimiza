// Fazer o ajuste valer AGORA: as preferências de `HKCU\Control Panel` são lidas no logon, e gravar o registro
// não muda a sessão aberta. `SystemParametersInfoW` com `SPIF_SENDCHANGE` avisa o Windows, como o Painel de
// Controle. É sincronização: lê o registro e empurra, e serve igual ao aplicar e ao desfazer.

/// Só `HKCU\Control Panel`: é ali que moram as preferências lidas no logon.
pub fn precisa_sincronizar_interface(hive: &str, path: &str) -> bool {
    hive.eq_ignore_ascii_case("HKCU")
        && path
            .trim_start_matches('\\')
            .to_lowercase()
            .starts_with("control panel")
}

/// Ausência não é zero: o padrão de `DragFullWindows` é 1 e o de `MenuShowDelay` é 400. Ao desfazer, a
/// reversão devolve o valor a ausente, e empurrar zero desligaria o que ninguém pediu.
pub fn valor_ou_padrao(lido: Option<i32>, padrao: i32) -> i32 {
    lido.unwrap_or(padrao)
}

/// Widgets, Copilot e sugestões da busca são lidos pelo Explorer ao iniciar. Diz-se a verdade em vez de
/// reiniciar o Explorer por conta própria, o que fecharia as pastas abertas no meio de um lote.
pub fn nota_de_ativacao(hive: &str, path: &str, name: &str) -> Option<&'static str> {
    const PELO_EXPLORADOR: &str =
        "O ajuste já está gravado; a barra de tarefas mostra a mudança depois de reiniciar o \
         Explorador do Windows (pelo Gerenciador de Tarefas) ou de entrar de novo na conta.";

    let path = path.to_lowercase();
    let combina = |colmeia: &str, trecho: &str, valor: &str| {
        hive.eq_ignore_ascii_case(colmeia)
            && path.ends_with(trecho)
            && name.eq_ignore_ascii_case(valor)
    };

    if combina("HKLM", r"policies\microsoft\dsh", "AllowNewsAndInterests")
        || combina(
            "HKCU",
            r"policies\microsoft\windows\windowscopilot",
            "TurnOffWindowsCopilot",
        )
        || combina(
            "HKLM",
            r"policies\microsoft\windows\explorer",
            "DisableSearchBoxSuggestions",
        )
    {
        return Some(PELO_EXPLORADOR);
    }

    None
}

/// Idempotente. Falha aqui não invalida a otimização: o valor já está gravado e vale no próximo logon.
#[cfg(target_os = "windows")]
pub fn sincronizar_interface() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPIF_SENDCHANGE, SPI_SETDRAGFULLWINDOWS, SPI_SETMENUSHOWDELAY,
        SPI_SETMOUSE, SPI_SETUIEFFECTS,
    };

    // Os padrões do Windows: ausência significa "o Windows usa o dele", não zero.
    let mouse: [i32; 3] = [
        ler_numero(r"Control Panel\Mouse", "MouseSpeed", 1),
        ler_numero(r"Control Panel\Mouse", "MouseThreshold1", 6),
        ler_numero(r"Control Panel\Mouse", "MouseThreshold2", 10),
    ];

    let atraso_menu = ler_numero(r"Control Panel\Desktop", "MenuShowDelay", 400).max(0) as u32;
    let arrasto = ler_numero(r"Control Panel\Desktop", "DragFullWindows", 1) != 0;

    unsafe {
        SystemParametersInfoW(
            SPI_SETMOUSE,
            0,
            mouse.as_ptr() as *mut core::ffi::c_void,
            SPIF_SENDCHANGE,
        );

        SystemParametersInfoW(
            SPI_SETMENUSHOWDELAY,
            atraso_menu,
            core::ptr::null_mut(),
            SPIF_SENDCHANGE,
        );

        SystemParametersInfoW(
            SPI_SETDRAGFULLWINDOWS,
            u32::from(arrasto),
            core::ptr::null_mut(),
            SPIF_SENDCHANGE,
        );

        SystemParametersInfoW(SPI_SETUIEFFECTS, 0, core::ptr::null_mut(), SPIF_SENDCHANGE);
    }
}

#[cfg(target_os = "windows")]
fn ler_numero(path: &str, name: &str, padrao: i32) -> i32 {
    use crate::modules::changelog::PreviousValue;

    let lido = match super::registry::read("HKCU", path, name) {
        Ok(PreviousValue::Text(texto)) => texto.trim().parse().ok(),
        Ok(PreviousValue::Dword(valor)) => Some(valor as i32),
        _ => None,
    };

    valor_ou_padrao(lido, padrao)
}

#[cfg(not(target_os = "windows"))]
pub fn sincronizar_interface() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferencia_do_mouse_precisa_de_aviso() {
        assert!(precisa_sincronizar_interface("HKCU", r"Control Panel\Mouse"));
    }

    #[test]
    fn preferencia_de_area_de_trabalho_precisa_de_aviso() {
        assert!(precisa_sincronizar_interface("HKCU", r"Control Panel\Desktop"));
        assert!(precisa_sincronizar_interface(
            "HKCU",
            r"Control Panel\Desktop\WindowMetrics"
        ));
    }

    #[test]
    fn o_nome_da_colmeia_nao_diferencia_maiuscula() {
        assert!(precisa_sincronizar_interface("hkcu", r"control panel\mouse"));
    }

    #[test]
    fn chave_de_maquina_nao_passa_por_aqui() {
        assert!(!precisa_sincronizar_interface(
            "HKLM",
            r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers"
        ));
    }

    #[test]
    fn outra_chave_do_usuario_nao_passa_por_aqui() {
        assert!(!precisa_sincronizar_interface(
            "HKCU",
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize"
        ));
    }

    #[test]
    fn valor_ausente_cai_no_padrao_do_windows_e_nao_em_zero() {
        assert_eq!(valor_ou_padrao(None, 1), 1);
        assert_eq!(valor_ou_padrao(None, 400), 400);
    }

    #[test]
    fn valor_lido_vence_o_padrao_inclusive_quando_e_zero() {
        assert_eq!(valor_ou_padrao(Some(0), 400), 0);
        assert_eq!(valor_ou_padrao(Some(250), 400), 250);
    }

    #[test]
    fn widgets_e_copilot_avisam_que_a_barra_so_muda_depois() {
        assert!(nota_de_ativacao(
            "HKLM",
            r"SOFTWARE\Policies\Microsoft\Dsh",
            "AllowNewsAndInterests"
        )
        .is_some());
        assert!(nota_de_ativacao(
            "HKCU",
            r"Software\Policies\Microsoft\Windows\WindowsCopilot",
            "TurnOffWindowsCopilot"
        )
        .is_some());
    }

    #[test]
    fn chave_sem_cache_do_shell_nao_ganha_nota() {
        // Nota onde não precisa ensina o cliente a ignorar aviso.
        assert!(nota_de_ativacao(
            "HKCU",
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize",
            "StartupDelayInMSec"
        )
        .is_none());
    }
}
