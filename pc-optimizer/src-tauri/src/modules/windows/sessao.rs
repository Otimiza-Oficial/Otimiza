// Devolução ao encerrar a sessão do Windows
//
// POR QUE ISTO EXISTE
//
// `suspend.rs` já cobre "o Otimiza morreu": `retomar_pendentes` roda na
// próxima abertura e devolve tudo. Mas entre a morte do Otimiza e essa
// próxima abertura pode acontecer um desligamento ou um logoff — e uma
// thread suspensa não responde à mensagem de fim de sessão do Windows. O
// Windows não consegue descarregar a colmeia de registro do usuário
// (UsrClass.dat, que guarda os registros COM do shell), e na sessão seguinte
// o Explorer não abre e os atalhos da barra de tarefas não fazem nada. Foi
// isto que aconteceu na máquina que originou este módulo: dezesseis
// processos suspensos, nunca devolvidos, e setenta eventos 1512/1542 no
// mesmo minuto do logoff seguinte.
//
// POR QUE É JANELA, E NÃO CONSOLE
//
// O Windows tem dois mecanismos para avisar um processo de que a sessão está
// terminando: `SetConsoleCtrlHandler` (para processo de console) e as
// mensagens `WM_QUERYENDSESSION`/`WM_ENDSESSION` (para processo com janela).
// O Otimiza é uma aplicação de janela — carrega `user32.dll` para desenhar a
// interface —, e o próprio Windows documenta que um processo assim não é
// tratado como console para fins de logoff/desligamento: o retorno do
// `HandlerRoutine` de `SetConsoleCtrlHandler` simplesmente não é chamado
// para `CTRL_LOGOFF_EVENT`/`CTRL_SHUTDOWN_EVENT` num processo que carregou
// `user32.dll`. A forma documentada e correta para este tipo de processo é a
// mensagem de janela.
//
// POR QUE UM SUBCLASSE PRÓPRIO, E NÃO O EVENTO DO TAURI
//
// A versão do `tao` (a biblioteca de janelas por trás do Tauri nesta versão
// do projeto) que examinamos NÃO processa `WM_QUERYENDSESSION` — o próprio
// código-fonte dela tem a chamada comentada, com uma nota dizendo que o
// mecanismo ainda não foi implementado. E o tratamento que ela dá a
// `WM_ENDSESSION` só encerra o laço de eventos internamente; não chega a
// gerar um `RunEvent` que o `run()` do Tauri exponha para o código do
// produto. Depender do evento do Tauri aqui seria depender de um caminho que
// não existe na versão que este projeto usa.
//
// A saída é gravar nosso PRÓPRIO subclasse na janela, usando a mesma API que
// o `tao` já usa por baixo (`SetWindowSubclass`/`DefSubclassProc`, do
// `comctl32`) — ela foi desenhada justamente para permitir vários donos na
// mesma janela, cada um encadeando para o próximo. Não é um controle
// paralelo por fora do Tauri: é o mecanismo suportado para se somar ao que
// já está lá, sem substituir nada.
//
// POR QUE RESPONDER NA CONSULTA, E NÃO SÓ NO AVISO FINAL
//
// `WM_QUERYENDSESSION` chega primeiro, perguntando "pode encerrar?"; só
// depois de todo mundo responder que sim o Windows manda `WM_ENDSESSION`
// avisando que vai acontecer de verdade. O orçamento de tempo antes de o
// Windows considerar o processo travado e matá-lo à força é curto — e conta
// a partir da primeira mensagem. Devolver os processos já na consulta usa o
// tempo inteiro disponível, em vez de gastar parte dele esperando a segunda
// mensagem chegar. Devolver de novo no aviso final não tem custo: retomar
// processo que já está rodando não faz nada (ver `suspend::api::retomar`).
//
// Se o desligamento for cancelado por outro programa depois da consulta, o
// pior caso é o Otimiza ter devolvido processos cedo demais durante uma
// sessão que continuou — inofensivo pelo mesmo motivo, e infinitamente
// melhor do que a alternativa de não devolver nunca.

#[cfg(target_os = "windows")]
mod api {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{WM_ENDSESSION, WM_QUERYENDSESSION};

    /// Identificador do nosso subclasse nesta janela.
    ///
    /// `SetWindowSubclass` distingue subclasses pelo par (função, id); o
    /// valor em si não precisa significar nada, só ser estável e não colidir
    /// com outro subclasse desta mesma janela — e só o `tao` está na mesma
    /// janela, com o seu próprio id interno.
    const ID_SUBCLASSE: usize = 0x4F54_4D5A; // "OTMZ" em hex

    /// Roda na thread da interface, exatamente quando o Windows avisa que a
    /// sessão está terminando (ver o comentário do módulo para o porquê das
    /// duas mensagens).
    ///
    /// PRECISA ser rápida: isto acontece dentro do orçamento de tempo que o
    /// Windows dá antes de considerar o processo travado. `retomar_tudo` só
    /// percorre threads já suspensas do próprio processo — sem PowerShell,
    /// sem rede, sem disco além de um JSON pequeno — e por isso cabe aqui.
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

        // Sempre encadeia para o próximo dono da janela — é o contrato do
        // subclasse do comctl32, e é o que deixa o `tao` continuar recebendo
        // as mensagens dele normalmente.
        DefSubclassProc(janela, mensagem, wparam, lparam)
    }

    /// Registra o subclasse na janela principal.
    ///
    /// `janela` chega como ponteiro cru porque o tipo `HWND` do Tauri vem do
    /// crate `windows`, e o deste módulo vem do `windows-sys` — crates
    /// diferentes, mesmo `*mut c_void` por baixo (é como o Win32 representa
    /// um identificador de janela). Converter pelo valor cru evita depender
    /// dos dois crates ao mesmo tempo só por causa de um tipo que já é
    /// idêntico.
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

/// Liga a devolução por fim de sessão nesta janela.
///
/// Chamado uma vez, na configuração do aplicativo (`lib.rs`), com o `HWND` da
/// janela principal. Falhar aqui não é fatal para o programa — só significa
/// que esta rede de segurança específica não está de pé, e as outras
/// (devolução ao fechar o Otimiza, devolução na próxima abertura, prazo
/// máximo) continuam cobrindo o cliente.
pub fn instalar(janela: *mut std::ffi::c_void) -> bool {
    api::instalar(janela)
}

#[cfg(test)]
mod tests {
    #[test]
    fn nao_mata_processo() {
        // A mesma trava dos outros módulos deste conjunto: este arquivo
        // decide QUANDO devolver, nunca decide matar. Quem mata é sempre
        // proibido — a suspensão em si já garante isso em `suspend.rs`, e
        // este teste tranca que ninguém introduza um atalho aqui.
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
        // Não há janela de verdade numa esteira sem sessão gráfica. Chamar
        // com um ponteiro nulo não pode gerar pânico nem violação de
        // acesso — só devolver "não instalou", que é a resposta honesta.
        assert!(!super::instalar(std::ptr::null_mut()));
    }
}
