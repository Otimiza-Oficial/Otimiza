mod ci_coverage;
// PC Performance Optimizer - Main Library
// Tauri + Rust implementation

mod core;
mod modules;
mod utils;
mod commands;

use commands::AppState;
use modules::changelog::ChangeLog;
use modules::PerformanceMonitor;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            monitor: Mutex::new(PerformanceMonitor::new()),
            changes: Mutex::new(ChangeLog::load()),
            #[cfg(target_os = "windows")]
            processes: Mutex::new(modules::windows::processes::ProcessMonitor::new()),
            #[cfg(target_os = "windows")]
            reparo: modules::windows::tarefa_longa::TarefaLonga::nova(),
            #[cfg(target_os = "windows")]
            disco: std::sync::Mutex::new(Default::default()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_platform_info,
            commands::get_performance_metrics,
            commands::start_monitoring,
            commands::stop_monitoring,
            commands::measure_baseline,
            commands::get_baseline,
            commands::measure_and_compare,
            commands::is_elevated,
            commands::relaunch_as_admin,
            commands::get_hardware_profile,
            commands::analyze_firmware,
            commands::top_processes,
            commands::get_preferences,
            commands::set_preferences,
            commands::analyze_bloatware,
            commands::remove_store_app,
            commands::open_apps_settings,
            commands::analyze_conflicts,
            commands::analyze_health,
            commands::analyze_shaders,
            commands::clean_shader_cache,
            commands::analyze_readiness,
            commands::diagnostico_rapido,
            commands::analyze_gpu_preference,
            commands::set_gpu_preference,
            commands::fix_readiness,
            commands::running_game_executable,
            commands::set_persistent_priority,
            commands::analyze_bottleneck,
            commands::game_mode_status,
            commands::set_game_mode,
            commands::analyze_network,
            commands::set_dns,
            commands::flush_dns,
            commands::measure_frames,
            commands::analyze_fivem,
            commands::clean_fivem,
            commands::prioritize_fivem,
            commands::analyze_browsers,
            commands::clean_browser_cache,
            commands::analyze_boot,
            commands::analyze_thermal,
            commands::export_report,
            commands::map_folders,
            commands::list_profiles,
            commands::list_third_party_services,
            commands::set_service_start,
            commands::list_scheduled_tasks,
            commands::set_scheduled_task,
            commands::scan_disk_space,
            commands::clean_disk_category,
            commands::empty_recycle_bin,
            commands::analyze_memory,
            commands::set_automatic_pagefile,
            commands::restore_status,
            commands::create_restore_point,
            commands::enable_system_protection,
            commands::list_startup,
            commands::set_startup_enabled,
            commands::list_optimizations,
            commands::apply_optimization,
            commands::placa_de_video,
            commands::memoria_instalada,
            commands::monitores,
            commands::analyze_game_config,
            commands::medir_antes,
            commands::medir_depois,
            commands::prova_guardada,
            commands::preview_game_profile,
            commands::apply_game_profile,
            commands::revert_optimization,
            commands::optimize_now,
            commands::revert_all_optimizations,
            commands::set_max_refresh_rate,
            commands::licenca_estado,
            commands::licenca_ativar,
            commands::reparo_disponivel,
            commands::reparo_ultimo_resultado,
            commands::reparo_executar,
            commands::reparo_cancelar,
            commands::congelados_agora,
            commands::descongelar_agora,
            commands::relatorio_de_suporte,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            utils::Logger::info("PC Performance Optimizer iniciado");

            // TERCEIRA REDE DE SEGURANÇA DA SUSPENSÃO: fim de sessão do Windows.
            //
            // As outras duas (mais abaixo, e `retomar_pendentes` acima) cobrem
            // "o Otimiza não está mais rodando". Esta cobre o intervalo entre o
            // Otimiza morrer e o cliente desligar ou fazer logoff: uma thread
            // suspensa não responde à mensagem de fim de sessão, o Windows não
            // descarrega a colmeia de registro do usuário, e o Explorer para de
            // abrir na sessão seguinte. Foi exatamente essa cadeia que originou
            // este conserto — ver o comentário de `modules::windows::sessao`
            // para a investigação completa de por que é janela e não console.
            #[cfg(target_os = "windows")]
            {
                if let Some(janela) = app.get_webview_window("main") {
                    match janela.hwnd() {
                        Ok(hwnd) => {
                            if !modules::windows::sessao::instalar(hwnd.0) {
                                utils::Logger::info(
                                    "Não consegui ligar a devolução por fim de sessão do \
                                     Windows — as outras redes de segurança continuam de pé.",
                                );
                            }
                        }
                        Err(e) => utils::Logger::info(&format!(
                            "Não consegui obter a janela para a devolução por fim de sessão: {}",
                            e
                        )),
                    }
                }
            }

            // PRIMEIRA REDE DE SEGURANÇA DA SUSPENSÃO — antes de qualquer outra
            // coisa.
            //
            // O modo jogo suspende Discord, navegador e afins para devolver
            // memória ao jogo, e os devolve quando o jogo fecha. Se o Otimiza
            // morrer no meio disso — travamento, fechamento à força, queda de
            // energia — esses programas ficariam congelados até o cliente
            // reiniciar o PC, sem qualquer pista do motivo.
            //
            // Os identificadores vão para disco ANTES de a primeira thread ser
            // suspensa. Esta chamada é o outro lado dessa garantia, e por isso
            // roda de forma síncrona, na frente de tudo: um Discord congelado
            // por nossa causa é um defeito pior do que o que viemos resolver.
            #[cfg(target_os = "windows")]
            {
                let devolvidos = modules::windows::suspend::retomar_pendentes();

                if !devolvidos.is_empty() {
                    let nomes: Vec<&str> =
                        devolvidos.iter().map(|s| s.visivel.as_str()).collect();
                    utils::Logger::info(&format!(
                        "Devolvi programas que tinham ficado pausados: {}",
                        nomes.join(", ")
                    ));
                }
            }

            // Vigia do modo jogo.
            //
            // Roda sempre, mas só age quando a preferência está ligada — e ela
            // vem desligada de fábrica. A preferência é lida a cada volta, e
            // não uma vez só: assim ligar e desligar na tela vale na hora, sem
            // reiniciar o programa.
            //
            // Seis segundos é de propósito. Mais rápido que isso gasta CPU do
            // próprio otimizador para vigiar, o que num PC fraco é o oposto do
            // trabalho; mais devagar e o jogo já está carregando quando o modo
            // entra.
            #[cfg(target_os = "windows")]
            {
                let handle = app.handle().clone();

                tauri::async_runtime::spawn(async move {
                    use tauri::{Emitter, Manager};

                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(6)).await;

                        // A amostragem de pressão roda SEMPRE, independente do
                        // modo jogo, porque a pergunta que ela responde é sobre
                        // a rotina da máquina e não sobre o jogo. É de graça:
                        // uma leitura de memória em memória, sub-milissegundo.
                        // Nunca PowerShell aqui — a cada seis segundos isso
                        // seria o próprio otimizador pesando no PC do cliente.
                        modules::windows::pressao::amostrar();

                        // QUARTA REDE DE SEGURANÇA DA SUSPENSÃO: prazo máximo.
                        //
                        // Roda ANTES da checagem da preferência, de propósito: o
                        // cliente pode ter desligado o modo jogo automático
                        // depois que algo já ficou suspenso, e mesmo assim os
                        // programas precisam voltar. `retomar_se_expirado` só
                        // faz alguma coisa quando há suspenso pendente E não há
                        // jogo algum rodando agora — nunca interrompe uma
                        // partida em andamento, por mais longa que seja. Ver a
                        // justificativa do prazo em `suspend::PRAZO_MAXIMO_SEGUNDOS`.
                        let expirados = modules::windows::suspend::retomar_se_expirado(
                            modules::windows::suspend::PRAZO_MAXIMO_SEGUNDOS,
                        );
                        if !expirados.is_empty() {
                            let nomes: Vec<&str> =
                                expirados.iter().map(|s| s.visivel.as_str()).collect();
                            let mensagem = format!(
                                "Devolvi programas que ficaram suspensos além do prazo, sem \
                                 jogo nenhum rodando: {}",
                                nomes.join(", ")
                            );
                            utils::Logger::info(&mensagem);

                            // A tela só sabe que algo foi devolvido através deste
                            // evento — é o mesmo que o vigia normal emite ao
                            // suspender e ao devolver. Sem ele aqui, o bloco de
                            // congelados continuaria mostrando um programa que já
                            // voltou até algum OUTRO evento forçar a atualização:
                            // a própria tela mentindo sobre o estado que ela existe
                            // para mostrar direito. Só entra neste `if` — emitir a
                            // cada seis segundos sem nada ter mudado faria a tela
                            // recarregar à toa o tempo todo.
                            let _ = handle.emit("gamemode:changed", mensagem);
                        }

                        if !modules::preferences::Preferences::load().auto_game_mode {
                            continue;
                        }

                        let estado = handle.state::<commands::AppState>();
                        let mut log = estado.changes.lock().await;

                        // `passo` não faz nada quando não há mudança, então o
                        // caso comum desta volta é não tocar em nada.
                        if let Some(mensagem) =
                            modules::windows::gamemode::passo(&mut log)
                        {
                            utils::Logger::info(&format!("Modo jogo: {}", mensagem));
                            let _ = handle.emit("gamemode:changed", mensagem);
                        }
                    }
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            // SEGUNDA REDE DE SEGURANÇA DA SUSPENSÃO: fechar o Otimiza.
            //
            // `ExitRequested` cobre o caminho comum — o cliente fechou a
            // janela, ou pediu para sair pelo tray — e `Exit` cobre o
            // instante final do laço de eventos, de propósito redundante com
            // o de cima: `retomar_tudo` devolver um processo que já está
            // rodando não faz nada (ver o comentário em
            // `suspend::api::retomar`), então chamar duas vezes não tem
            // custo, e cobre o caso de o primeiro evento não chegar a rodar
            // por algum motivo do próprio Tauri.
            #[cfg(target_os = "windows")]
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                let devolvidos = modules::windows::suspend::retomar_tudo().unwrap_or_default();

                if !devolvidos.is_empty() {
                    let nomes: Vec<&str> =
                        devolvidos.iter().map(|s| s.visivel.as_str()).collect();
                    utils::Logger::info(&format!(
                        "Devolvi programas suspensos antes de fechar: {}",
                        nomes.join(", ")
                    ));
                }
            }
        });
}
