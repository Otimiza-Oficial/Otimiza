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
use crate::modules::windows::foldermap::FolderMap;
#[cfg(target_os = "windows")]
use crate::modules::windows::health::HealthReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::boot::BootReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::browsers::{BrowserReport, CleanOutcome as BrowserCleanOutcome};
#[cfg(target_os = "windows")]
use crate::modules::windows::thermal::ThermalReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::fivem::{FiveMReport, CleanOutcome as FiveMCleanOutcome};
#[cfg(target_os = "windows")]
use crate::modules::windows::network::NetworkReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::frames::FrameMeasurement;
#[cfg(target_os = "windows")]
use crate::modules::windows::gamemode::GameModeStatus;
#[cfg(target_os = "windows")]
use crate::modules::windows::bottleneck::BottleneckReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::shaders::{ShaderReport, CleanOutcome as ShaderCleanOutcome};
#[cfg(target_os = "windows")]
use crate::modules::windows::readiness::ReadinessReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::veredito::Veredito;
#[cfg(target_os = "windows")]
use crate::modules::windows::gpupref::GpuPrefReport;
#[cfg(target_os = "windows")]
use crate::modules::windows::tasks::ScheduledTask;
#[cfg(target_os = "windows")]
use crate::modules::windows::servicesaudit::ServiceEntry;
#[cfg(target_os = "windows")]
use crate::modules::windows::bloatware::BloatReport;
#[cfg(target_os = "windows")]
use crate::modules::optimizer::BatchStep;
use tauri::{Emitter, Manager};
use crate::modules::{PerformanceMonitor, PerformanceMetrics};

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
    pub cpu_name: String,
    pub gpu_name: String,
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
            cpu_name: hardware.cpu_name.clone(),
            gpu_name: hardware.gpu_name.clone(),
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
    crate::modules::licenca::exigir()?;

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
    crate::modules::licenca::exigir()?;

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

/// Comando: Perfis de otimização recomendados por tipo de uso.
///
/// Perfil aqui é sugestão que marca caixas na lista, não pacote fechado: a
/// pessoa continua vendo e podendo desmarcar cada item.
#[tauri::command]
pub fn list_profiles() -> Vec<crate::modules::windows::profiles::ProfileInfo> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::profiles::PROFILES.to_vec()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

/// Comando: Mapa das maiores pastas do perfil do usuário.
///
/// Responde a pergunta que vem antes do liberador de espaço: não "o que dá para
/// apagar", mas "cadê o meu disco".
#[tauri::command]
pub async fn map_folders() -> Result<FolderMap, String> {
    #[cfg(target_os = "windows")]
    {
        // Percorre centenas de milhares de arquivos: fora do runtime async,
        // senão trava a interface inteira durante a varredura.
        tokio::task::spawn_blocking(|| {
            use crate::modules::windows::foldermap;
            foldermap::mapear(&foldermap::perfil_do_usuario(), 12)
        })
        .await
        .map_err(|e| format!("Falha ao mapear pastas: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// Relatório de atendimento
// ---------------------------------------------------------------------------

/// Levanta o estado da máquina para o relatório.
///
/// Cada análise é independente e nenhuma derruba as outras: se a leitura do
/// boot falhar, o documento sai sem essa seção e diz que ela não estava
/// disponível — em vez de o relatório inteiro deixar de existir.
///
/// O mapa de pastas e a varredura de espaço ficam de fora de propósito: levam
/// quase um minuto cada, e o relatório é gerado com o cliente esperando.
#[cfg(target_os = "windows")]
fn coletar_para_relatorio() -> crate::modules::report::ReportData {
    use crate::modules::windows;

    crate::modules::report::ReportData {
        boot: Some(windows::boot::analyze()),
        thermal: Some(windows::thermal::analyze()),
        health: Some(windows::health::analyze()),
        memory: Some(windows::memory::analyze()),
        browsers: Some(windows::browsers::analyze()),
        startup: windows::startup::entries(),
        // O mesmo veredito que a tela mostra, para que o papel e o programa
        // não possam discordar sobre a mesma máquina.
        veredito: Some(windows::veredito::diagnostico_rapido()),
    }
}

#[cfg(not(target_os = "windows"))]
fn coletar_para_relatorio() -> crate::modules::report::ReportData {
    crate::modules::report::ReportData::default()
}

/// Comando: Gera o relatório entregável e grava na Área de Trabalho.
///
/// A comparação vem da interface porque ela já tem o resultado da última
/// medição em mãos. Refazer o benchmark aqui custaria vários segundos e, pior,
/// mediria um momento diferente daquele que o usuário está vendo na tela.
#[tauri::command]
pub async fn export_report(
    state: State<'_, AppState>,
    comparison: Option<BenchmarkComparison>,
) -> Result<crate::modules::report::ReportSaved, String> {
    // O levantamento roda fora do runtime: são várias consultas ao WMI e ao
    // log de eventos, e juntas passam de dez segundos. Feito aqui dentro,
    // travaria a interface enquanto o técnico espera.
    let dados = tokio::task::spawn_blocking(coletar_para_relatorio)
        .await
        .map_err(|e| format!("Falha ao levantar os dados da maquina: {}", e))?;

    let changes = state.changes.lock().await;
    crate::modules::report::save(&changes, comparison.as_ref(), &dados)
}

// ---------------------------------------------------------------------------
// Cache de shader, prontidão e prioridade permanente
// ---------------------------------------------------------------------------

/// Comando: Cache de shader e idade do driver de vídeo.
#[tauri::command]
pub async fn analyze_shaders() -> Result<ShaderReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::shaders::analyze)
            .await
            .map_err(|e| format!("Falha ao ler o cache de shader: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Apaga um cache de shader.
#[tauri::command]
pub async fn clean_shader_cache(id: String) -> Result<ShaderCleanOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || crate::modules::windows::shaders::limpar(&id))
            .await
            .map_err(|e| format!("Falha ao limpar: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = id;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: O veredito da máquina.
///
/// É o diagnóstico que roda sozinho ao abrir o programa, antes de qualquer
/// botão de otimizar. Recolhe só o que é barato e devolve UMA frase com o
/// número que a sustenta — o resto dos diagnósticos continua sob demanda.
///
/// Existe porque o produto foi testado em máquina que travava e disse que
/// estava tudo bem: os achados estavam corretos, mas espalhados por cinco abas,
/// e nenhum deles era o veredito.
#[tauri::command]
pub async fn diagnostico_rapido() -> Result<Veredito, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::veredito::diagnostico_rapido)
            .await
            .map_err(|e| format!("Falha ao diagnosticar a máquina: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Qual placa de vídeo cada jogo usa.
#[tauri::command]
pub async fn analyze_gpu_preference() -> Result<GpuPrefReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::gpupref::analyze)
            .await
            .map_err(|e| format!("Falha ao ler as placas de vídeo: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Fixa qual placa de vídeo um jogo deve usar.
///
/// Em notebook com duas placas, é o maior ganho de FPS que este produto tem
/// para dar — e não exige administrador nem reiniciar o PC.
#[tauri::command]
pub async fn set_gpu_preference(
    caminho: String,
    desempenho: bool,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::gpupref::Preferencia;

        let preferencia = if desempenho {
            Preferencia::Desempenho
        } else {
            Preferencia::Automatica
        };

        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new()
            .set_gpu_preference(&caminho, preferencia, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (caminho, desempenho, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Condições que atrapalham antes de otimizar.
#[tauri::command]
pub async fn analyze_readiness() -> Result<ReadinessReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::readiness::analyze)
            .await
            .map_err(|e| format!("Falha ao verificar o sistema: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Corrige um item de prontidão que o Otimiza sabe resolver.
#[tauri::command]
pub async fn fix_readiness(id: String) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            use crate::modules::windows::readiness;

            match id.as_str() {
                "trim" => readiness::ligar_trim(),
                "plano_maximo" => readiness::criar_plano_maximo(),
                outro => Err(format!("`{}` não é corrigível pelo Otimiza.", outro)),
            }
        })
        .await
        .map_err(|e| format!("Falha ao corrigir: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = id;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Executável do jogo aberto agora, para fixar a prioridade dele.
#[tauri::command]
pub fn running_game_executable() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::gamemode::executavel_do_jogo()
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Comando: Fixa ou remove a prioridade alta permanente de um jogo.
#[tauri::command]
pub async fn set_persistent_priority(
    executable: String,
    enable: bool,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new()
            .set_persistent_priority(&executable, enable, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (executable, enable, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// Analisador de gargalo
// ---------------------------------------------------------------------------

/// Comando: Descobre qual recurso está limitando o desempenho.
///
/// Não otimiza nada — só explica. É a resposta para "por que meu FPS é baixo",
/// e a resposta honesta quase nunca é "falta otimizar".
#[tauri::command]
pub async fn analyze_bottleneck(seconds: u64) -> Result<BottleneckReport, String> {
    #[cfg(target_os = "windows")]
    {
        // Amostra contadores em laço pelo tempo pedido: bloqueia, então sai do
        // runtime. Entre 4 e 30 segundos — menos não dá amostra suficiente,
        // mais é o técnico parado olhando a tela.
        tokio::task::spawn_blocking(move || {
            crate::modules::windows::bottleneck::analisar(seconds.clamp(4, 30))
        })
        .await
        .map_err(|e| format!("Falha ao analisar o gargalo: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = seconds;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// Modo jogo
// ---------------------------------------------------------------------------

/// Comando: Situação do modo jogo.
#[tauri::command]
pub async fn game_mode_status(state: State<'_, AppState>) -> Result<GameModeStatus, String> {
    #[cfg(target_os = "windows")]
    {
        let log = state.changes.lock().await;
        Ok(crate::modules::windows::gamemode::status(&log))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Liga ou desliga o modo jogo na mão.
#[tauri::command]
pub async fn set_game_mode(
    active: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::gamemode;

        let mut log = state.changes.lock().await;

        if active {
            gamemode::ativar(&mut log).map(|feito| feito.join(" "))
        } else {
            gamemode::desativar(&mut log)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (active, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// Rede e quadros
// ---------------------------------------------------------------------------

/// Comando: Mede os resolvedores de DNS e mostra o que a máquina usa.
#[tauri::command]
pub async fn analyze_network() -> Result<NetworkReport, String> {
    #[cfg(target_os = "windows")]
    {
        // São várias consultas de DNS cronometradas; leva alguns segundos.
        tokio::task::spawn_blocking(crate::modules::windows::network::analyze)
            .await
            .map_err(|e| format!("Falha ao medir a rede: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Troca o DNS de um adaptador, com registro para reversão.
#[tauri::command]
pub async fn set_dns(
    guid: String,
    servers: String,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new().set_dns(&guid, &servers, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (guid, servers, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Limpa o cache de resolução de nomes.
#[tauri::command]
pub async fn flush_dns() -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::network::limpar_cache_dns)
            .await
            .map_err(|e| format!("Falha ao limpar: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Conta os quadros que um jogo está entregando.
///
/// Mede de fora, escutando o canal de eventos do Windows — nada é injetado no
/// processo do jogo.
#[tauri::command]
pub async fn measure_frames(process: String, seconds: u64) -> Result<FrameMeasurement, String> {
    #[cfg(target_os = "windows")]
    {
        // A medição bloqueia pelo tempo pedido: precisa sair do runtime.
        tokio::task::spawn_blocking(move || {
            use crate::modules::windows::frames;

            let (pid, nome) = frames::encontrar_processo(&process).ok_or_else(|| {
                format!(
                    "Não encontrei nenhum processo com `{}` no nome. Abra o jogo antes de medir.",
                    process
                )
            })?;

            // Entre 3 e 30 segundos: menos que isso não estabiliza, mais que
            // isso é o técnico parado olhando a tela.
            frames::medir(pid, &nome, seconds.clamp(3, 30))
        })
        .await
        .map_err(|e| format!("Falha ao medir os quadros: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (process, seconds);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// FiveM
// ---------------------------------------------------------------------------

/// Comando: Levantamento da instalação do FiveM.
#[tauri::command]
pub async fn analyze_fivem() -> Result<FiveMReport, String> {
    #[cfg(target_os = "windows")]
    {
        // A pasta de cache tem dezenas de milhares de arquivos: somar tudo
        // leva segundos e não pode travar a interface.
        tokio::task::spawn_blocking(crate::modules::windows::fivem::analyze)
            .await
            .map_err(|e| format!("Falha ao ler a instalação do FiveM: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Apaga uma pasta descartável do FiveM.
///
/// Recusa pasta protegida e recusa com o jogo aberto. Não tem volta.
#[tauri::command]
pub async fn clean_fivem(id: String) -> Result<FiveMCleanOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || crate::modules::windows::fivem::limpar(&id))
            .await
            .map_err(|e| format!("Falha ao limpar: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = id;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Prioridade alta no processador para o jogo.
#[tauri::command]
pub async fn prioritize_fivem() -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::fivem::priorizar_jogo)
            .await
            .map_err(|e| format!("Falha ao ajustar a prioridade: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// Navegador
// ---------------------------------------------------------------------------

/// Comando: O que o navegador está consumindo, e o que dá para recuperar.
#[tauri::command]
pub async fn analyze_browsers() -> Result<BrowserReport, String> {
    #[cfg(target_os = "windows")]
    {
        // Percorre as pastas de perfil somando tamanho: fora do runtime, senão
        // trava a interface durante a varredura.
        tokio::task::spawn_blocking(crate::modules::windows::browsers::analyze)
            .await
            .map_err(|e| format!("Falha ao ler os navegadores: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Limpa o cache descartável de um navegador.
///
/// Não tem volta, e recusa se o navegador estiver aberto. Dado de aplicativo —
/// IndexedDB e afins — nunca é tocado.
#[tauri::command]
pub async fn clean_browser_cache(executable: String) -> Result<BrowserCleanOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            crate::modules::windows::browsers::limpar_cache(&executable)
        })
        .await
        .map_err(|e| format!("Falha ao limpar: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = executable;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// Inicialização e limitação do processador
// ---------------------------------------------------------------------------

/// Comando: Quanto o PC demora para ligar, e quem atrasa.
///
/// É a medição que o cliente percebe. Ajuste de registro rende pouco que se
/// sinta; boot que cai de dois minutos para quarenta segundos, todo mundo nota.
#[tauri::command]
pub async fn analyze_boot() -> Result<BootReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::boot::analyze)
            .await
            .map_err(|e| format!("Falha ao ler o tempo de inicialização: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Por que o processador não está entregando tudo.
#[tauri::command]
pub async fn analyze_thermal() -> Result<ThermalReport, String> {
    #[cfg(target_os = "windows")]
    {
        // Amostra contadores e varre o log térmico; fora do runtime porque a
        // consulta WMI custa mais de um segundo.
        tokio::task::spawn_blocking(crate::modules::windows::thermal::analyze)
            .await
            .map_err(|e| format!("Falha ao medir o processador: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ---------------------------------------------------------------------------
// Saúde do hardware
// ---------------------------------------------------------------------------

/// Comando: Lê a saúde física do disco e da bateria.
///
/// É a checagem que evita o pior desperdício de tempo do técnico: otimizar por
/// uma tarde uma máquina cujo problema é peça morrendo.
#[tauri::command]
pub async fn analyze_health() -> Result<HealthReport, String> {
    #[cfg(target_os = "windows")]
    {
        // Consulta WMI de armazenamento e bateria; fora do runtime porque
        // `Get-StorageReliabilityCounter` conversa com o disco e demora.
        tokio::task::spawn_blocking(crate::modules::windows::health::analyze)
            .await
            .map_err(|e| format!("Falha ao ler a saúde do hardware: {}", e))
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

/// Comando: Serviços deixados por programas instalados.
#[tauri::command]
pub async fn list_third_party_services() -> Result<Vec<ServiceEntry>, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::servicesaudit::listar_de_terceiros)
            .await
            .map_err(|e| format!("Falha ao listar serviços: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: Leva um serviço para Manual, ou devolve para Automático.
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

/// Comando: Liga a Proteção do Sistema no disco do Windows.
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
    crate::modules::licenca::exigir()?;

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
    crate::modules::licenca::exigir()?;

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

/// Comando: mostra o que um perfil MUDARIA na configuração do jogo.
///
/// Não escreve nada. Existe para a tela poder listar chave por chave, com o
/// valor de agora, o valor novo e o que se perde — antes de o cliente decidir.
///
/// Fica em `LIVRES`: é leitura, e o diagnóstico do produto é livre.
#[tauri::command]
pub async fn preview_game_profile(
    perfil: String,
) -> Result<Vec<(String, String, String, String)>, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::configjogo;

        let escolhido = perfil_por_nome(&perfil)?;
        let relatorio = configjogo::analyze();

        let Some(caminho) = relatorio.arquivo else {
            return Err("Não encontrei a configuração de nenhum jogo conhecido.".to_string());
        };

        let conteudo = std::fs::read_to_string(&caminho)
            .map_err(|e| format!("não consegui ler {}: {}", caminho.display(), e))?;

        Ok(configjogo::prever(&conteudo, escolhido)
            .into_iter()
            .map(|(chave, atual, novo, custo)| (chave, atual, novo, custo.to_string()))
            .collect())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = perfil;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: aplica um perfil na configuração do jogo.
///
/// É A ÚNICA COISA DO PRODUTO QUE ESCREVE NUM ARQUIVO DO CLIENTE, e por isso
/// registra no histórico de desfazer como qualquer outra otimização — só que
/// guardando o arquivo INTEIRO em vez de um valor de registro.
///
/// Vai para `EXIGEM_LICENCA`: altera o computador.
#[cfg(target_os = "windows")]
fn perfil_por_nome(nome: &str) -> Result<crate::modules::windows::configjogo::Perfil, String> {
    use crate::modules::windows::configjogo::Perfil;

    match nome {
        "sem_teto" => Ok(Perfil::SemTeto),
        "equilibrado" => Ok(Perfil::Equilibrado),
        "competitivo" => Ok(Perfil::Competitivo),
        outro => Err(format!("perfil desconhecido: {}", outro)),
    }
}

#[tauri::command]
pub async fn apply_game_profile(
    perfil: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        use crate::modules::changelog::{AppliedOptimization, ChangeRecord, now_timestamp};
        use crate::modules::windows::configjogo;

        let escolhido = perfil_por_nome(&perfil)?;
        let feito = configjogo::aplicar_perfil(escolhido)?;

        // NADA MUDOU, NADA É REGISTRADO.
        //
        // Gravar um registro de desfazer para uma mudança que não houve daria
        // ao cliente um item no histórico que não desfaz nada — e a sensação de
        // que algo foi mexido quando não foi.
        if feito.mudou.is_empty() {
            return Ok(feito.mudou);
        }

        let mut log = state.changes.lock().await;

        log.record(AppliedOptimization {
            optimization_id: format!("config_jogo_{}", perfil),
            name: format!("Configuração do {} · perfil {}", feito.jogo, perfil),
            timestamp: now_timestamp(),
            changes: vec![ChangeRecord::GameConfig {
                caminho: feito.arquivo.to_string_lossy().to_string(),
                anterior: Some(feito.anterior),
                jogo: feito.jogo,
            }],
        })?;

        Ok(feito.mudou)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (perfil, state);
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

/// Comando: "Otimizar agora" — aplica todo o catálogo de uma vez.
///
/// `only` restringe o lote a uma lista de ids: é como um perfil aplica só o que
/// recomenda. Os filtros de segurança do motor valem igual nos dois casos.
#[tauri::command]
pub async fn optimize_now(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    only: Option<Vec<String>>,
) -> Result<Vec<OptimizationOutcome>, String> {
    crate::modules::licenca::exigir()?;

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
        Ok(crate::modules::windows::WindowsOptimizer::new().apply_selection(
            only.as_deref(),
            &mut log,
            |step| {
                let _ = app.emit("optimize:step", step);
            },
        ))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state, only);
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

/// Coloca um monitor na maior taxa de atualização que ele aceita.
///
/// O parâmetro chama-se `id` porque é assim que o botão do diagnóstico manda o
/// argumento — um só, sempre com esse nome. Aqui ele é o dispositivo, no
/// formato `\.\DISPLAY1`.
#[tauri::command]
pub async fn set_max_refresh_rate(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;

        crate::modules::windows::WindowsOptimizer::new()
            .set_max_refresh_rate(&id, &mut log)
            .map(|resultado| resultado.message)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (id, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// =========================================================== LICENÇA

/// O estado da licença desta máquina.
///
/// Livre de propósito: é o comando que alimenta a tela de compra, e uma tela
/// de compra que precisa de licença para abrir não faria sentido nenhum.
#[tauri::command]
pub fn licenca_estado() -> crate::modules::licenca::Estado {
    crate::modules::licenca::estado()
}

/// Ativa uma chave. Confere ANTES de gravar.
#[tauri::command]
pub fn licenca_ativar(chave: String) -> Result<crate::modules::licenca::Estado, String> {
    crate::modules::licenca::ativar(&chave)?;
    Ok(crate::modules::licenca::estado())
}

// ==================================================== A GUARDA DA GUARDA

/// Confere que todo comando está classificado e que os que alteram o sistema
/// pedem licença.
///
/// Existe porque a falha mais provável deste sistema não é alguém quebrar a
/// assinatura — é alguém acrescentar um comando novo daqui a seis meses e
/// esquecer a linha da guarda. Um comando esquecido é uma porta aberta que
/// ninguém percebe, e revisão de código não pega isso de forma confiável.
///
/// Com este teste, comando novo sem classificação REPROVA O BUILD.
#[cfg(test)]
mod tests {
    /// Rodam sem licença: leitura, medição, e o desfazer.
    ///
    /// O desfazer está aqui de propósito. Se a licença vencer, o cliente
    /// precisa conseguir voltar o PC dele ao que era. Trancar o `revert`
    /// deixaria a máquina alterada sem caminho de volta pela nossa tela.
    const LIVRES: &[&str] = &[
        "preview_game_profile",
        "get_platform_info",
        "get_performance_metrics",
        "start_monitoring",
        "stop_monitoring",
        "measure_baseline",
        "get_baseline",
        "measure_and_compare",
        "is_elevated",
        "relaunch_as_admin",
        "get_hardware_profile",
        "analyze_firmware",
        "top_processes",
        "get_preferences",
        "set_preferences",
        "analyze_bloatware",
        "open_apps_settings",
        "analyze_conflicts",
        "analyze_health",
        "analyze_shaders",
        "analyze_readiness",
        "diagnostico_rapido",
        "analyze_gpu_preference",
        "running_game_executable",
        "analyze_bottleneck",
        "game_mode_status",
        "analyze_network",
        "measure_frames",
        "analyze_fivem",
        "analyze_browsers",
        "analyze_boot",
        "analyze_thermal",
        "export_report",
        "map_folders",
        "list_profiles",
        "list_third_party_services",
        "list_scheduled_tasks",
        "scan_disk_space",
        "analyze_memory",
        "restore_status",
        "list_startup",
        "list_optimizations",
        "revert_optimization",
        "revert_all_optimizations",
        "licenca_estado",
        "licenca_ativar",
    ];

    /// Alteram o computador. Sem licença, recusam.
    const EXIGEM_LICENCA: &[&str] = &[
        "clean_disk_category",
        "empty_recycle_bin",
        "set_automatic_pagefile",
        "clean_shader_cache",
        "set_gpu_preference",
        "fix_readiness",
        "set_persistent_priority",
        "set_game_mode",
        "set_dns",
        "flush_dns",
        "clean_fivem",
        "prioritize_fivem",
        "clean_browser_cache",
        "set_scheduled_task",
        "set_service_start",
        "remove_store_app",
        "create_restore_point",
        "enable_system_protection",
        "set_startup_enabled",
        "apply_game_profile",
        "apply_optimization",
        "optimize_now",
        "set_max_refresh_rate",
    ];

    /// Só a parte de produção do arquivo. O código de teste também contém as
    /// palavras que estamos procurando, e olhar o arquivo inteiro faria a
    /// guarda se encontrar sozinha.
    fn producao() -> &'static str {
        let fonte = include_str!("commands.rs");

        let corte = fonte
            .split("#[cfg(test)]")
            .next()
            .expect("split devolve ao menos um pedaço");

        assert!(
            corte.len() < fonte.len(),
            "não achei onde a produção termina; a guarda estaria olhando o \
             arquivo errado"
        );

        corte
    }

    /// Os nomes de todos os comandos, lidos do próprio fonte.
    fn comandos() -> Vec<&'static str> {
        let marca = concat!("#[tauri::", "command]");

        producao()
            .split(marca)
            .skip(1)
            .map(|bloco| {
                let assinatura = bloco
                    .lines()
                    .find(|l| l.contains("pub fn ") || l.contains("pub async fn "))
                    .expect("todo comando tem uma assinatura logo abaixo da marca");

                assinatura
                    .split("fn ")
                    .nth(1)
                    .and_then(|resto| resto.split('(').next())
                    .expect("nome do comando")
                    .trim()
            })
            .collect()
    }

    #[test]
    fn nenhum_comando_fica_sem_classificacao() {
        for nome in comandos() {
            let livre = LIVRES.contains(&nome);
            let paga = EXIGEM_LICENCA.contains(&nome);

            assert!(
                livre || paga,
                "o comando `{}` não está classificado. Decida: ele LÊ o \
                 computador (vai para LIVRES) ou ALTERA (vai para \
                 EXIGEM_LICENCA e ganha a linha `licenca::exigir()?`)?",
                nome
            );

            assert!(
                !(livre && paga),
                "`{}` está nas duas listas",
                nome
            );
        }
    }

    #[test]
    fn quem_altera_o_computador_pede_licenca() {
        let fonte = producao();

        for nome in EXIGEM_LICENCA {
            let inicio = fonte
                .find(&format!("fn {}(", nome))
                .unwrap_or_else(|| panic!("comando `{}` sumiu do arquivo", nome));

            let corpo = &fonte[inicio..];
            let abre = corpo.find('{').expect("corpo do comando");

            // A guarda precisa ser a PRIMEIRA coisa do corpo. Depois de
            // qualquer trabalho já é tarde: o comando já teria começado a
            // mexer no computador de quem não pagou.
            let primeiras = corpo[abre + 1..]
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("");

            assert!(
                primeiras.contains("licenca::exigir()?"),
                "`{}` altera o computador e não confere a licença na primeira \
                 linha. Achei: `{}`",
                nome,
                primeiras.trim()
            );
        }
    }

    #[test]
    fn quem_so_le_nao_pede_licenca() {
        // O outro lado do erro. Se o diagnóstico passar a exigir licença, a
        // tela de compra fica vazia — e o argumento de venda do produto é
        // justamente mostrar o problema real da máquina antes de cobrar.
        let fonte = producao();

        for nome in LIVRES {
            let Some(inicio) = fonte.find(&format!("fn {}(", nome)) else {
                continue;
            };

            // O fim da função é a primeira linha que é só `}`. Procurar por
            // "\n}\n" direto NÃO serve: este arquivo tem quebra de linha do
            // Windows, e a busca nunca acha — a fatia iria até o fim do
            // arquivo e o teste acusaria todo mundo de exigir licença.
            let corpo: String = fonte[inicio..]
                .lines()
                .take_while(|l| l.trim_end() != "}")
                .collect::<Vec<_>>()
                .join("\n");

            assert!(
                !corpo.contains("licenca::exigir()?"),
                "`{}` está na lista dos livres e mesmo assim exige licença",
                nome
            );
        }
    }

    #[test]
    fn as_duas_listas_cobrem_todos_os_comandos() {
        let achados = comandos().len();
        let classificados = LIVRES.len() + EXIGEM_LICENCA.len();

        assert_eq!(
            achados, classificados,
            "{} comandos no arquivo, {} nas listas",
            achados, classificados
        );
    }

    /// A BARRA DA JANELA É DESENHADA POR NÓS, E ISSO TEM UM PREÇO EM PERMISSÃO.
    ///
    /// Com `decorations: false` o Windows não desenha mais fechar, minimizar e
    /// maximizar: quem faz isso é o nosso HTML, chamando comandos do Tauri. E
    /// no Tauri 2 todo comando precisa estar declarado em
    /// `capabilities/default.json` — o que não está declarado falha CALADO, sem
    /// erro na tela.
    ///
    /// Foi exatamente assim que o duplo-clique para maximizar nasceu quebrado:
    /// ele fala por um comando separado (`internal_toggle_maximize`), e a
    /// permissão dele não é a mesma do botão de maximizar.
    ///
    /// Um botão de janela que não faz nada é o tipo de defeito que ninguém
    /// reporta e todo mundo sente. Esta guarda existe para ele não voltar.
    #[test]
    fn a_barra_da_janela_tem_todas_as_permissoes_que_usa() {
        let permissoes = include_str!("../capabilities/default.json");

        for comando in [
            // Os três botões.
            "core:window:allow-close",
            "core:window:allow-minimize",
            "core:window:allow-toggle-maximize",
            // Arrastar a barra, e o duplo-clique nela.
            "core:window:allow-start-dragging",
            "core:window:allow-internal-toggle-maximize",
            // Saber se está maximizada, para tirar o canto arredondado.
            "core:window:allow-is-maximized",
        ] {
            assert!(
                permissoes.contains(comando),
                "a barra da janela usa `{}` e a permissão não está declarada;                  o botão vai falhar sem dizer nada",
                comando
            );
        }
    }
}
