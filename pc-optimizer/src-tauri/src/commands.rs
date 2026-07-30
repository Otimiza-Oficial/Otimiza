// Tauri Commands
// IPC commands exposed to the frontend

use crate::core::PlatformDetector;
use crate::modules::benchmark::{
    self, BaselineResult, BaselineStore, Benchmark, BenchmarkComparison, BenchmarkSnapshot,
};
use crate::modules::changelog::ChangeLog;
use crate::modules::preferences::Preferences;
#[cfg(target_os = "windows")]
use crate::modules::windows::firmware::FirmwareReport;
use crate::modules::optimizer::{OptimizationInfo, OptimizationOutcome};
#[cfg(target_os = "windows")]
use crate::modules::windows::processes::ProcessImpact;
#[cfg(target_os = "windows")]
use crate::modules::windows::startup::StartupEntry;
#[cfg(target_os = "windows")]
use crate::modules::windows::restore::RestoreStatus;
#[cfg(target_os = "windows")]
use crate::modules::windows::diskspace::{CleanOutcome, DiskReport};
#[cfg(target_os = "windows")]
use crate::modules::windows::memory::MemoryReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::conflicts::ConflictReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::tasks::ScheduledTask;
#[cfg(target_os = "windows")]
use crate::modules::windows::bloatware::BloatReport;
#[cfg(target_os = "windows")]
use crate::modules::optimizer::BatchStep;
use tauri::{Emitter, Manager};
use crate::modules::{DiagnosticEngine, DiagnosticReport, PerformanceMonitor, PerformanceMetrics};

#[cfg(not(target_os = "windows"))]
const UNSUPPORTED_PLATFORM: &str =
    "Otimizações ainda não implementadas para este sistema operacional.";
use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

// Estado global do aplicativo.
// Usa tokio::sync::Mutex porque os guards atravessam pontos de `.await`
// dentro dos comandos — um std::sync::MutexGuard não é Send e faria
// o future do comando falhar em compilar.
pub struct AppState {
    pub monitor: Mutex<PerformanceMonitor>,
    pub changes: Mutex<ChangeLog>,
    /// Mantido entre chamadas: o uso de CPU por processo só existe comparando
    /// duas leituras consecutivas. Recriar o monitor a cada chamada devolveria
    /// sempre zero.
    #[cfg(target_os = "windows")]
    pub processes: Mutex<crate::modules::windows::processes::ProcessMonitor>,
}

#[derive(Serialize)]
pub struct PlatformInfoResponse {
    pub platform: String,
    pub os_type: String,
    pub arch: String,
    pub version: String,
}

/// Comando: Obter informações da plataforma
#[tauri::command]
pub fn get_platform_info() -> Result<PlatformInfoResponse, String> {
    let info = PlatformDetector::get_info();
    Ok(PlatformInfoResponse {
        platform: format!("{:?}", info.platform),
        os_type: info.os_type,
        arch: info.arch,
        version: info.version,
    })
}

/// Comando: Executar diagnóstico completo
#[tauri::command]
pub async fn run_diagnostic() -> Result<DiagnosticReport, String> {
    let mut diagnostic_engine = DiagnosticEngine::new();
    diagnostic_engine.run_full_diagnostic().await
}

/// Comando: Obter métricas de performance em tempo real
#[tauri::command]
pub async fn get_performance_metrics(state: State<'_, AppState>) -> Result<PerformanceMetrics, String> {
    let mut monitor = state.monitor.lock().await;
    monitor.collect_metrics().await
}

/// Comando: Iniciar monitoramento contínuo
#[tauri::command]
pub async fn start_monitoring(state: State<'_, AppState>) -> Result<String, String> {
    let mut monitor = state.monitor.lock().await;
    monitor.start_monitoring();
    Ok("Monitoring started".to_string())
}

/// Comando: Parar monitoramento
#[tauri::command]
pub async fn stop_monitoring(state: State<'_, AppState>) -> Result<String, String> {
    let mut monitor = state.monitor.lock().await;
    monitor.stop_monitoring();
    Ok("Monitoring stopped".to_string())
}

// ---------------------------------------------------------------------------
// Medição de desempenho
//
// O benchmark ocupa a CPU por vários segundos, então roda em `spawn_blocking`:
// executá-lo direto no runtime async travaria a interface e, pior, distorceria a
// própria medição.
// ---------------------------------------------------------------------------

/// Comando: Mede o desempenho atual e grava como ponto de partida.
/// O baseline vai para disco, então otimizações que exigem reiniciar o PC
/// continuam mensuráveis depois do boot.
#[tauri::command]
pub async fn measure_baseline() -> Result<BaselineResult, String> {
    let snapshot = tokio::task::spawn_blocking(|| Benchmark::new().run())
        .await
        .map_err(|e| format!("Benchmark failed: {}", e))?;

    BaselineStore::save(&snapshot)?;
    Ok(BaselineResult::from(snapshot))
}

/// Comando: Devolve o baseline gravado, se existir.
#[tauri::command]
pub fn get_baseline() -> Option<BenchmarkSnapshot> {
    BaselineStore::load()
}

/// Comando: Mede de novo e compara com o baseline.
#[tauri::command]
pub async fn measure_and_compare() -> Result<BenchmarkComparison, String> {
    let before = BaselineStore::load().ok_or(
        "Nenhuma medição inicial encontrada. Meça o desempenho antes de otimizar.",
    )?;

    let after = tokio::task::spawn_blocking(|| Benchmark::new().run())
        .await
        .map_err(|e| format!("Benchmark failed: {}", e))?;

    Ok(benchmark::compare(&before, &after))
}

// ---------------------------------------------------------------------------
// Otimizações
//
// Os comandos abaixo existem em todas as plataformas para manter uma única
// interface, mas hoje só o Windows tem catálogo implementado. Em outros sistemas
// eles falham com uma mensagem clara em vez de fingir que otimizaram algo.
// ---------------------------------------------------------------------------

/// Comando: O programa está rodando como administrador?
/// A interface usa isso para avisar antes que o usuário tente aplicar algo e falhe.
#[tauri::command]
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

#[derive(Serialize)]
pub struct HardwareProfileResponse {
    /// "SSD", "HD mecânico" ou "não identificado".
    pub storage: String,
    pub total_ram_gb: f64,
    pub logical_cores: usize,
}

/// Comando: Perfil de hardware desta máquina.
/// É o que permite ao produto recusar otimizações que fariam mal a este PC.
#[tauri::command]
pub fn get_hardware_profile() -> Result<HardwareProfileResponse, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::hardware::{profile, StorageKind};

        let hardware = profile();
        Ok(HardwareProfileResponse {
            storage: match hardware.system_storage {
                StorageKind::Ssd => "SSD".to_string(),
                StorageKind::Hdd => "HD mecânico".to_string(),
                StorageKind::Unknown => "não identificado".to_string(),
            },
            total_ram_gb: hardware.total_ram_gb,
            logical_cores: hardware.logical_cores,
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Os processos que mais pesam no PC agora.
///
/// Responde a pergunta que o cliente realmente faz — "o que está deixando meu PC
/// lento?" — apontando o culpado pelo nome e dizendo se ele volta no próximo boot.
#[tauri::command]
pub async fn top_processes(state: State<'_, AppState>) -> Result<Vec<ProcessImpact>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut monitor = state.processes.lock().await;
        Ok(monitor.top(8))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Preferências gravadas.
#[tauri::command]
pub fn get_preferences() -> Preferences {
    Preferences::load()
}

/// Comando: Grava as preferências.
#[tauri::command]
pub fn set_preferences(preferences: Preferences) -> Result<Preferences, String> {
    preferences.save()?;
    // Devolve o que ficou de fato gravado: valores fora da faixa são corrigidos
    // na gravação, e a interface precisa refletir o valor real, não o pedido.
    Ok(Preferences::load())
}

// ---------------------------------------------------------------------------
// Espaço em disco
//
// Num PC fraco, disco cheio é o problema que mais se disfarça de "PC lento".
// ---------------------------------------------------------------------------

/// Comando: Varre o disco por categoria de espaço recuperável. Não apaga nada.
#[tauri::command]
pub async fn scan_disk_space() -> Result<DiskReport, String> {
    #[cfg(target_os = "windows")]
    {
        // A varredura percorre pastas grandes; fora do runtime para não travar
        // a interface enquanto soma.
        tokio::task::spawn_blocking(crate::modules::windows::diskspace::scan)
            .await
            .map_err(|e| format!("Falha na varredura: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Limpa uma categoria de espaço.
#[tauri::command]
pub async fn clean_disk_category(id: String) -> Result<CleanOutcome, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || crate::modules::windows::diskspace::clean(&id))
            .await
            .map_err(|e| format!("Falha na limpeza: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = id;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Esvazia a Lixeira.
#[tauri::command]
pub async fn empty_recycle_bin() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::diskspace::empty_recycle_bin)
            .await
            .map_err(|e| format!("Falha ao esvaziar a Lixeira: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// Memória e paginação
// ---------------------------------------------------------------------------

/// Comando: Diagnostica memória e arquivo de paginação.
#[tauri::command]
pub async fn analyze_memory() -> Result<MemoryReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::memory::analyze)
            .await
            .map_err(|e| format!("Falha na análise de memória: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Devolve ao Windows o gerenciamento do arquivo de paginação.
#[tauri::command]
pub async fn set_automatic_pagefile() -> Result<String, String> {
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

// ---------------------------------------------------------------------------
// Conflitos entre programas e tarefas agendadas
// ---------------------------------------------------------------------------

/// Comando: Procura programas que brigam entre si.
#[tauri::command]
pub async fn analyze_conflicts() -> Result<ConflictReport, String> {
    #[cfg(target_os = "windows")]
    {
        // Percorre o registro de programas instalados e a lista de processos;
        // fora do runtime para não travar a interface.
        tokio::task::spawn_blocking(crate::modules::windows::conflicts::analyze)
            .await
            .map_err(|e| format!("Falha ao procurar conflitos: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Tarefas agendadas de terceiros.
#[tauri::command]
pub async fn list_scheduled_tasks() -> Result<Vec<ScheduledTask>, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::tasks::listar_de_terceiros)
            .await
            .map_err(|e| format!("Falha ao listar tarefas: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Liga ou desliga uma tarefa agendada.
///
/// Entra no histórico com id próprio, então "Desfazer tudo" também devolve as
/// tarefas ao estado original.
#[tauri::command]
pub async fn set_scheduled_task(
    path: String,
    name: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
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

// ---------------------------------------------------------------------------
// Programas de fábrica
// ---------------------------------------------------------------------------

/// Comando: Procura utilitário de fabricante, antivírus em teste e app da Loja
/// pré-instalado.
#[tauri::command]
pub async fn analyze_bloatware() -> Result<BloatReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::bloatware::analyze)
            .await
            .map_err(|e| format!("Falha ao examinar programas: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Remove um aplicativo da Microsoft Store.
///
/// Só aplicativos da Loja: eles voltam pela Loja quando o usuário quiser.
/// Programa comum nunca é desinstalado por nós — para esses, abrimos a tela
/// oficial do Windows.
#[tauri::command]
pub async fn remove_store_app(package: String) -> Result<String, String> {
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

/// Comando: Abre a tela de programas instalados do Windows.
///
/// Desinstalar programa comum é feito pelo desinstalador do próprio fabricante,
/// que costuma fazer perguntas. Levar o usuário até a tela oficial é mais seguro
/// que imitar esse processo e arriscar deixar instalação pela metade.
#[tauri::command]
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

/// Comando: Estado dos pontos de restauração do Windows.
#[tauri::command]
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

/// Comando: Cria um ponto de restauração agora.
/// Pode levar dezenas de segundos: o Windows tira um instantâneo do volume.
#[tauri::command]
pub async fn create_restore_point() -> Result<String, String> {
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

/// Comando: Liga a Proteção do Sistema no disco do Windows.
#[tauri::command]
pub async fn enable_system_protection() -> Result<String, String> {
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

/// Comando: Programas que sobem com o Windows.
#[tauri::command]
pub fn list_startup() -> Result<Vec<StartupEntry>, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::startup::entries())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Liga ou desliga um programa de inicialização.
///
/// Não remove a entrada do cliente: escreve no mesmo lugar que o Gerenciador de
/// Tarefas do Windows escreve, e o valor anterior vai para o histórico.
#[tauri::command]
pub async fn set_startup_enabled(
    hive: String,
    name: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        let outcome = crate::modules::windows::WindowsOptimizer::new()
            .set_startup(&hive, &name, enabled, &mut log)?;

        // O monitor de processos marca quem sobe no boot; a lista dele precisa
        // refletir a mudança na próxima leitura.
        state.processes.lock().await.refresh_startup();

        Ok(outcome)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (hive, name, enabled, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Analisa firmware e hardware.
///
/// Não escreve nada na BIOS — em placa de consumo isso não é possível com
/// segurança. Lê o que a BIOS e o hardware estão fazendo com o desempenho e
/// aponta onde se resolve: software, BIOS ou troca de peça.
///
/// Leva ~12 segundos por causa da medição de carga sustentada, então roda em
/// `spawn_blocking` para não travar a interface.
#[tauri::command]
pub async fn analyze_firmware() -> Result<FirmwareReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::firmware::analyze)
            .await
            .map_err(|e| format!("Firmware analysis failed: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Reabre o programa como administrador.
///
/// Um processo não consegue ganhar privilégios sozinho no Windows: é preciso
/// iniciar um novo processo e deixar o próprio sistema pedir a autorização ao
/// usuário. Se ele recusar no aviso do Windows, nada acontece e o programa
/// continua rodando normalmente com acesso limitado.
///
/// Devolve a mensagem a ser mostrada ao usuário. Em versão final ela fica vazia,
/// porque este processo encerra e o elevado assume sem que haja o que explicar.
#[tauri::command]
pub fn relaunch_as_admin(app: tauri::AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if crate::modules::windows::registry::is_elevated() {
            return Ok(String::new());
        }

        let executable = std::env::current_exe()
            .map_err(|e| format!("Não foi possível localizar o programa: {}", e))?;

        // Aspas simples são escapadas dobrando, conforme a regra do PowerShell.
        let path = executable.to_string_lossy().replace('\'', "''");
        let script = format!("Start-Process -FilePath '{}' -Verb RunAs", path);

        crate::modules::windows::shell::run_checked(
            "powershell",
            &["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script],
        )
        .map_err(|_| {
            "Você recusou a permissão de administrador. Nada foi alterado.".to_string()
        })?;

        // Em desenvolvimento, este processo é filho do `tauri dev`, que também
        // hospeda o servidor do Vite. Encerrá-lo derruba o servidor junto, e a
        // janela elevada abriria numa página de erro por não achar o localhost.
        //
        // A saída é esconder a janela em vez de encerrar o processo: fica só uma
        // janela na tela, como o usuário espera, e o processo continua vivo em
        // segundo plano apenas para manter o servidor no ar.
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

        // Na versão final os arquivos vão embutidos no programa: o processo
        // elevado se basta, e manter os dois abertos só confundiria o usuário e
        // faria os dois disputarem o mesmo arquivo de histórico.
        app.exit(0);
        Ok(String::new())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Lista o catálogo de otimizações com o estado atual de cada uma
#[tauri::command]
pub async fn list_optimizations(
    state: State<'_, AppState>,
) -> Result<Vec<OptimizationInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        let log = state.changes.lock().await;
        Ok(crate::modules::windows::WindowsOptimizer::new().list(&log))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Ok(Vec::new())
    }
}

/// Comando: Aplica uma otimização específica
#[tauri::command]
pub async fn apply_optimization(
    id: String,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new().apply(&id, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (id, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Desfaz uma otimização específica
#[tauri::command]
pub async fn revert_optimization(
    id: String,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new().revert(&id, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (id, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: "Otimizar agora" — aplica todo o catálogo de uma vez
#[tauri::command]
pub async fn optimize_now(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<OptimizationOutcome>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;

        // Rede de segurança antes de qualquer mudança. Não bloqueia o lote se
        // falhar — nosso histórico já reverte item por item — mas o cliente é
        // informado do que aconteceu de verdade, inclusive quando não deu.
        if Preferences::load().restore_point_before_batch {
            let (message, success) = match crate::modules::windows::restore::create(
                "Otimiza - antes de otimizar",
            ) {
                Ok(message) => (message, true),
                Err(error) => (error, false),
            };

            let _ = app.emit(
                "optimize:step",
                BatchStep {
                    index: 0,
                    total: 0,
                    name: "Ponto de restauração do Windows".to_string(),
                    stage: "finished",
                    message,
                    changes: Vec::new(),
                    success,
                },
            );
        }

        // Cada passo é emitido na hora que acontece: a interface mostra o que
        // está sendo mexido, em vez de uma barra de progresso sem informação.
        Ok(crate::modules::windows::WindowsOptimizer::new().apply_all(&mut log, |step| {
            let _ = app.emit("optimize:step", step);
        }))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Desfaz todas as otimizações aplicadas
#[tauri::command]
pub async fn revert_all_optimizations(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<OptimizationOutcome>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;

        Ok(crate::modules::windows::WindowsOptimizer::new().revert_all(&mut log, |step| {
            let _ = app.emit("optimize:step", step);
        }))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}
