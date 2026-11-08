// Sistema: inicialização, serviços, tarefas agendadas, apps da loja, ponto de restauração, limpeza, arquivo de
// paginação, elevação e as preferências do próprio programa (fundos, flags, erros da tela).

use super::*;

// Existem em todas as plataformas para uma interface só; fora do Windows falham com mensagem clara.

/// Para avisar antes de o usuário tentar aplicar e falhar.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::registry::is_elevated()
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[tauri::command(async)]
pub fn get_preferences() -> Preferences {
    Preferences::load()
}

#[tauri::command(async)]
pub fn set_preferences(preferences: Preferences) -> Result<Preferences, String> {
    preferences.save()?;
    // Valores fora da faixa são corrigidos: a interface reflete o gravado, não o pedido.
    Ok(Preferences::load())
}

// A Lixeira saiu do liberador na 2.9: é item da Limpeza do sistema, desmarcado.

#[tauri::command]
pub async fn set_automatic_pagefile() -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::memory::set_automatic_pagefile)
            .await
            .map_err(|e| format!("Falha ao alterar a paginação: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: só mede.
#[tauri::command(async)]
/// `pagina_ms`: quanto a própria página levou, da navegação ao primeiro desenho (`performance.now()`). O resto até o
/// total é o WebView2 nascendo: a divisão diz onde cortar.
pub fn abertura_pronta(pagina_ms: Option<f64>) -> Option<u64> {
    let ms = crate::modules::abertura::marcar_pronta();
    if let Some(ms) = ms {
        match pagina_ms {
            Some(p) => crate::utils::Logger::info(&format!(
                "abertura: {} ms (página: {} ms da navegação ao primeiro desenho)",
                ms,
                p.round() as u64
            )),
            None => crate::utils::Logger::info(&format!("abertura: {} ms", ms)),
        }
    }
    ms
}

#[tauri::command]
pub async fn list_scheduled_tasks() -> Result<Vec<ScheduledTask>, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::tasks::listar_de_terceiros)
            .await
            .map_err(|e| format!("Falha ao listar tarefas: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Id próprio no histórico: "Desfazer tudo" alcança.
#[tauri::command]
pub async fn set_scheduled_task(
    path: String,
    name: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new()
            .set_scheduled_task(&path, &name, enabled, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, name, enabled, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command]
pub async fn list_third_party_services() -> Result<Vec<ServiceEntry>, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::servicesaudit::listar_de_terceiros)
            .await
            .map_err(|e| format!("Falha ao listar serviços: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command]
pub async fn set_service_start(
    name: String,
    automatic: bool,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new()
            .set_service_start(&name, automatic, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (name, automatic, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command]
pub async fn analyze_bloatware() -> Result<BloatReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::bloatware::analyze)
            .await
            .map_err(|e| format!("Falha ao examinar programas: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Só apps da Loja (voltam pela Loja); programa comum vai pela tela oficial do Windows.
#[tauri::command]
pub async fn remove_store_app(package: String) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            crate::modules::windows::bloatware::remover_app_da_loja(&package)
        })
        .await
        .map_err(|e| format!("Falha ao remover: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = package;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// O desinstalador do fabricante faz perguntas: imitá-lo arriscaria instalação pela metade.
#[tauri::command(async)]
pub fn open_apps_settings() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::shell::run_checked(
            "cmd",
            &["/c", "start", "", "ms-settings:appsfeatures"],
        )?;
        Ok("Tela de aplicativos do Windows aberta.".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn restore_status() -> Result<RestoreStatus, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::restore::status)
            .await
            .map_err(|e| format!("Restore status failed: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Pode levar dezenas de segundos.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn create_restore_point() -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(|| {
            crate::modules::windows::restore::create("Otimiza - ponto manual")
        })
        .await
        .map_err(|e| format!("Restore point failed: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command]
pub async fn enable_system_protection() -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::restore::enable_protection)
            .await
            .map_err(|e| format!("Enable protection failed: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn list_startup() -> Result<Vec<StartupEntry>, String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::startup::entries()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Escreve onde o Gerenciador de Tarefas escreve, com o anterior no histórico.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn set_startup_enabled(
    hive: String,
    name: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        let outcome = crate::modules::windows::WindowsOptimizer::new()
            .set_startup(&hive, &name, enabled, &mut log)?;

        state.processes.lock().await.refresh_startup();

        Ok(outcome)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (hive, name, enabled, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Um processo não se eleva sozinho: abre outro e o Windows pergunta; recusado, nada acontece. Na versão final a
/// mensagem fica vazia, porque este processo encerra.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
/// Genérico no runtime só para o contrato tipado (`contrato.rs`) montar com o `MockRuntime`; no programa é o Wry.
pub fn relaunch_as_admin<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if crate::modules::windows::registry::is_elevated() {
            return Ok(String::new());
        }

        let executable = std::env::current_exe()
            .map_err(|e| format!("Não foi possível localizar o programa: {}", e))?;

        // Aspas simples escapadas dobrando, regra do PowerShell.
        let path = executable.to_string_lossy().replace('\'', "''");
        let script = format!("Start-Process -FilePath '{}' -Verb RunAs", path);

        // `Start-Process -Verb RunAs` só volta quando a pessoa responde ao aviso.
        crate::modules::windows::shell::run_checked_com_prazo(
            "powershell",
            &["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script],
            std::time::Duration::from_secs(600),
        )
        .map_err(|_| "Você recusou a permissão de administrador. Nada foi alterado.".to_string())?;

        // Em desenvolvimento, encerrar derrubaria o servidor do Vite hospedado pelo `tauri dev`: esconde a janela.
        if cfg!(debug_assertions) {
            for (_, window) in app.webview_windows() {
                let _ = window.hide();
            }

            crate::utils::Logger::info(
                "Janela oculta: o processo segue vivo só para manter o servidor de \
                 desenvolvimento. Feche este terminal para encerrar tudo.",
            );

            return Ok(String::new());
        }

        // Na versão final, dois abertos disputariam o mesmo arquivo de histórico.
        app.exit(0);
        Ok(String::new())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Quanto cada limpeza libera agora (temporários e relatórios de erro, só arquivo com mais de 2 dias). `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn limpeza_medir() -> Vec<crate::plataforma::limpeza::Item> {
    crate::plataforma::limpeza::medir()
}

/// Apaga os arquivos velhos de um item da limpeza; o que estiver em uso fica. `EXIGEM_LICENCA`. Sem volta: o cartão
/// mostra o tamanho antes e só limpa pasta temporária ou de relatório (`limpeza::pasta_permitida`).
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn limpeza_executar(id: String) -> Result<crate::plataforma::limpeza::Resultado, String> {
    crate::modules::licenca::exigir()?;
    crate::plataforma::limpeza::limpar(&id)
}

/// As imagens do topo do Início que a pessoa escolheu (só neste PC). `LIVRES`: preferência de tela.
#[tauri::command(async)]
pub fn fundos_listar() -> Vec<String> {
    crate::plataforma::fundos::listar()
}

/// `LIVRES`: guarda uma imagem escolhida na pasta de dados do Otimiza; não mexe no Windows.
#[tauri::command(async)]
pub fn fundos_adicionar(imagem: String) -> Result<usize, String> {
    crate::plataforma::fundos::adicionar(&imagem)
}

/// `LIVRES`: apaga só as imagens que a pessoa pôs; o topo volta à arte da Steam.
#[tauri::command(async)]
pub fn fundos_limpar() -> Result<(), String> {
    crate::plataforma::fundos::limpar()
}

/// Erro de JavaScript da tela vai para o `otimiza.log`, o arquivo que o cliente manda ao suporte. `LIVRES`.
#[tauri::command(async)]
pub fn anotar_erro_da_tela(texto: String) -> Result<(), String> {
    let curto: String = texto.chars().take(2000).collect();
    crate::utils::Logger::warn(&format!("tela: {}", curto));
    Ok(())
}

/// `LIVRES`: só lê.
#[tauri::command(async)]
pub fn flags_ler() -> Result<crate::plataforma::flags::Flags, String> {
    Ok(crate::plataforma::flags::Flags::load())
}

/// `LIVRES`: liga e desliga telas do próprio Otimiza, não mexe no computador.
#[tauri::command(async)]
pub fn flags_gravar(flags: crate::plataforma::flags::Flags) -> Result<(), String> {
    flags.save()
}
