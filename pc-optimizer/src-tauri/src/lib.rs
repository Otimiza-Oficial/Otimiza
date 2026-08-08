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
            commands::run_diagnostic,
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
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
