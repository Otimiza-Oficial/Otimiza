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
            commands::revert_optimization,
            commands::optimize_now,
            commands::revert_all_optimizations,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            
            utils::Logger::info("PC Performance Optimizer iniciado");

            // REDE DE SEGURANÇA DA SUSPENSÃO — antes de qualquer outra coisa.
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
