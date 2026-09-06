// Fazer o ajuste valer AGORA
//
// O defeito que este módulo existe para consertar: gravar no registro não muda
// o comportamento do Windows na sessão aberta. As preferências de `HKCU\Control
// Panel` são lidas no logon e mantidas em memória; escrever a chave e parar por
// aí deixa o valor certo no registro e o comportamento errado na tela.
//
// Duas otimizações prometiam efeito imediato e não entregavam:
//
//   Desativar aceleração do mouse  — a aceleração continuava até o próximo logon
//   Efeitos visuais para desempenho — as animações continuavam rodando
//
// As duas declaram `requires_restart: false`, então a promessa era explícita.
//
// A forma suportada de avisar o Windows é `SystemParametersInfoW`, que atualiza
// a cópia em memória e, com `SPIF_SENDCHANGE`, avisa os programas abertos. É o
// que o Painel de Controle faz ao clicar em Aplicar.
//
// A função é de SINCRONIZAÇÃO, não de aplicação: ela lê o que está no registro
// e empurra para a sessão. Assim serve tanto ao aplicar quanto ao desfazer —
// nos dois casos o que se quer é a sessão refletindo o registro, e não há como
// os dois caminhos divergirem.

/// Se uma escrita no registro mexeu numa preferência que o Windows mantém em
/// memória, e portanto exige aviso para valer na sessão aberta.
///
/// Só `HKCU\Control Panel`: é ali que moram as preferências do usuário lidas no
/// logon. Chave de máquina e política não passam por aqui — são lidas na hora
/// pelo componente que as usa, ou dependem de reinício, e nesses casos o
/// catálogo já declara `requires_restart`.
pub fn precisa_sincronizar_interface(hive: &str, path: &str) -> bool {
    hive.eq_ignore_ascii_case("HKCU")
        && path
            .trim_start_matches('\\')
            .to_lowercase()
            .starts_with("control panel")
}

/// O valor lido, ou o padrão do Windows quando não havia valor.
///
/// A distinção é onde estava um defeito: ausência não é zero. Para
/// `DragFullWindows` o padrão é 1 e para `MenuShowDelay` é 400 — assumir zero
/// desligaria o arraste de janela e zeraria o atraso do menu sem ninguém pedir.
/// Pior ao DESFAZER: a reversão devolve o valor a ausente, e a sincronização
/// seguinte empurraria zero para a sessão. Zero escolhido pelo usuário é escolha
/// legítima e vence o padrão; ausência não.
pub fn valor_ou_padrao(lido: Option<i32>, padrao: i32) -> i32 {
    lido.unwrap_or(padrao)
}

/// O que dizer ao cliente quando o ajuste está gravado mas o shell ainda não o
/// leu.
///
/// Algumas políticas — Widgets, Copilot, sugestões da busca — são lidas pelo
/// Explorer ao iniciar. A chave fica certa na hora e a barra de tarefas continua
/// igual, e essas otimizações declaram `requires_restart: false`. O cliente
/// aplicava, não via nada mudar e concluía que o produto não funcionou.
///
/// Aqui a resposta é dizer a verdade, não reiniciar o Explorer por conta
/// própria: derrubar o Explorador fecha as pastas que a pessoa deixou abertas, e
/// fazer isso sem avisar, no meio de um lote, é pior que esperar. Reiniciar o PC
/// inteiro por causa de um botão da barra também seria cobrar caro demais.
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

/// Empurra para a sessão aberta as preferências que estão no registro.
///
/// Idempotente de propósito: chamar duas vezes não faz mal, e chamar sem que
/// nada tenha mudado também não. Falha aqui não invalida a otimização — o valor
/// já está gravado e vale no próximo logon —, então o erro é engolido e a vida
/// segue, em vez de derrubar uma otimização que funcionou.
#[cfg(target_os = "windows")]
pub fn sincronizar_interface() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPIF_SENDCHANGE, SPI_SETDRAGFULLWINDOWS, SPI_SETMENUSHOWDELAY,
        SPI_SETMOUSE, SPI_SETUIEFFECTS,
    };

    // Os padrões são os do Windows, e estão aqui escritos porque ausência de
    // valor significa "o Windows usa o dele" — não zero.
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

        // Relê a `UserPreferencesMask` inteira, que é onde moram animação de
        // janela, sombra e deslizar de menu.
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
        // Sem o aviso, a aceleração do mouse continuava ligada até o próximo
        // logon — numa otimização que promete efeito imediato.
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
        // Nota que aparece onde não precisa vira ruído, e ruído numa ferramenta
        // de sistema ensina o cliente a ignorar aviso.
        assert!(nota_de_ativacao(
            "HKCU",
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize",
            "StartupDelayInMSec"
        )
        .is_none());
    }
}
