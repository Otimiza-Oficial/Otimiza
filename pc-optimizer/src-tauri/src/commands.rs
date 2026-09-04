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
use crate::modules::windows::citizenfx::CitizenFxReport;
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
    /// Sem `Mutex`: `TarefaLonga` já guarda o próprio estado internamente, e
    /// é o que impede duas ferramentas de reparo de rodar ao mesmo tempo.
    #[cfg(target_os = "windows")]
    pub reparo: crate::modules::windows::tarefa_longa::TarefaLonga,
    /// O que se sabe sobre o disco desta máquina nesta sessão — é isto que
    /// autoriza (ou não) agendar o `chkdsk`.
    ///
    /// `std::sync::Mutex`, e não o `tokio::sync::Mutex` dos vizinhos: os
    /// comandos de reparo são funções SÍNCRONAS marcadas `(async)`, então
    /// nenhum guarda daqui atravessa um `.await` — e um `blocking_lock()` do
    /// tokio chamado de dentro do runtime entraria em pânico.
    #[cfg(target_os = "windows")]
    pub disco: std::sync::Mutex<crate::modules::windows::reparo::EstadoDoDisco>,
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

/// Comando: mede perda de pacote, jitter de rede e tempo de resposta contra o
/// servidor em que o cliente está jogando agora.
///
/// O alvo sai das conexões ativas do processo do jogo — ver
/// `modules::windows::rede::servidor_do_jogo`. Não descobrir o servidor não é
/// erro: é o próprio resultado, escrito na nota (regra 1 do módulo).
///
/// Fica em `LIVRES`: é leitura, e é justamente o diagnóstico que evita o
/// cliente concluir, errado, que o produto o enganou quando otimiza o PC e a
/// travada continua — porque a travada era de rede, não de FPS.
#[tauri::command]
pub async fn medir_perda_de_pacote() -> Result<crate::modules::windows::rede::MedidaDeRede, String>
{
    #[cfg(target_os = "windows")]
    {
        // São até vinte pings sequenciais; leva alguns segundos.
        tokio::task::spawn_blocking(crate::modules::windows::rede::medir_agora)
            .await
            .map_err(|e| format!("Falha ao medir a perda de pacote: {}", e))
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

/// Comando: lê o `CitizenFX.ini` e mostra o que está em `PoolSizesIncrease`.
///
/// SÓ LEITURA. Não escreve nada, e não sugere aumentar nada — só mostra o que
/// já está configurado, quando há algo. Ver `modules::windows::citizenfx`
/// para o porquê: aumentar pool sem evidência de estouro no registro do
/// FiveM é o "aplique e torça" que o produto recusa, e ninguém viu esse
/// registro ainda.
///
/// Fica em `LIVRES`: é leitura, e nem exige o cliente ter licença para saber
/// o que já está configurado na máquina dele.
#[tauri::command]
pub async fn analyze_citizenfx() -> Result<CitizenFxReport, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::citizenfx::analyze())
    }

    #[cfg(not(target_os = "windows"))]
    {
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

/// Comando: os monitores, para a tela desenhá-los.
///
/// `display::monitores()` já existia e só alimentava o veredito. A tela nunca
/// via a lista — então o cliente lia "seu monitor está em 60 Hz e aceita 180"
/// sem nunca ver QUAL monitor, numa máquina com dois.
///
/// Fica em `LIVRES`: é leitura.
#[tauri::command]
pub async fn monitores() -> Result<Vec<crate::modules::windows::display::Monitor>, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::display::monitores())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: a memória instalada, para a tela desenhá-la slot a slot.
///
/// Fica em `LIVRES`: é leitura, e é justamente o diagnóstico que faz o cliente
/// entender por que o PC dele trava — sem pagar nada para descobrir.
#[tauri::command]
pub async fn memoria_instalada(
) -> Result<crate::modules::windows::firmware::MemoriaInstalada, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::firmware::memoria_instalada())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// A placa de vídeo desta máquina, para a tela desenhar.
#[derive(serde::Serialize)]
pub struct PlacaDeVideo {
    /// `nvidia`, `amd`, `intel` ou `desconhecida`. É o que escolhe a cor.
    pub marca: String,
    pub nome: Option<String>,
    pub driver: Option<String>,
    pub driver_data: Option<String>,
    pub driver_dias: Option<i64>,
    pub vram_gb: f64,
}

/// Deduz o fabricante pelo nome que o Windows dá à placa.
///
/// Pelo NOME e não por identificador de fornecedor no PCI: o nome é o que já
/// está lido e disponível de graça, e é o mesmo texto que o cliente vê no
/// Gerenciador de Dispositivos — então quando erra, ele consegue perceber que
/// errou. Um número de fornecedor certo mas invisível não daria a ele essa
/// chance.
///
/// Na dúvida devolve `desconhecida`, e a tela pergunta em vez de chutar.
pub fn marca_da_placa(nome: &str) -> &'static str {
    let n = nome.to_lowercase();

    if n.contains("nvidia") || n.contains("geforce") || n.contains("quadro") || n.contains("rtx") {
        "nvidia"
    } else if n.contains("amd") || n.contains("radeon") || n.contains("ati ") {
        "amd"
    } else if n.contains("intel") || n.contains("arc ") || n.contains("iris") {
        "intel"
    } else {
        "desconhecida"
    }
}

/// Comando: a placa de vídeo, para o painel que a desenha.
///
/// Junta o que três módulos já sabiam separados — o nome vem de `shaders`, que
/// já lia driver e data para decidir se o cache estava obsoleto, e a memória vem
/// de `bottleneck`, que já a lia do registro porque o valor do WMI satura em
/// 4 GB e mentiria justamente na faixa que interessa.
///
/// Fica em `LIVRES`: é leitura.
#[tauri::command]
pub async fn placa_de_video() -> Result<PlacaDeVideo, String> {
    #[cfg(target_os = "windows")]
    {
        let s = crate::modules::windows::shaders::analyze();
        let nome = s.gpu.clone();

        Ok(PlacaDeVideo {
            marca: nome
                .as_deref()
                .map(marca_da_placa)
                .unwrap_or("desconhecida")
                .to_string(),
            nome,
            driver: s.driver_version.clone(),
            driver_data: s.driver_date.clone(),
            driver_dias: s.driver_age_days,
            vram_gb: crate::modules::windows::bottleneck::vram_total_gb(),
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: lê a configuração do jogo instalado e diz o que está pesando.
///
/// Só lê. Fica em `LIVRES` pelo mesmo motivo que todo o diagnóstico fica: o
/// cliente pode descobrir de graça que o MSAA dele custa 40% dos quadros. É
/// essa descoberta que faz ele querer a chave.
#[tauri::command]
pub async fn analyze_game_config(
) -> Result<crate::modules::windows::configjogo::ConfigJogoReport, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::configjogo::analyze())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando: mede o jogo agora e guarda como o "antes".
///
/// O jogo precisa estar ABERTO — é o contrário de `apply_game_profile`, que
/// exige o jogo fechado. Não é contradição: mede-se o que está rodando, e
/// escreve-se no arquivo de quem não está.
///
/// Fica em `LIVRES`: medir é diagnóstico, e o diagnóstico do produto é livre.
/// Um cliente sem chave pode medir o próprio PC — e é justamente isso que faz
/// ele querer a chave.
#[tauri::command]
pub async fn medir_antes(process: String, seconds: u64) -> Result<crate::modules::prova::Prova, String> {
    let medicao = measure_frames(process, seconds).await?;

    let prova = crate::modules::prova::Prova {
        jogo: medicao.process,
        quando: crate::modules::changelog::now_timestamp(),
        fps: medicao.fps,
        low_1pct: medicao.low_1pct,
        engasgos_por_minuto: medicao.engasgos_por_minuto,
        segundos: medicao.seconds,
        confiavel: medicao.detalhe_confiavel,
    };

    crate::modules::prova::guardar(&prova)?;
    Ok(prova)
}

/// Comando: mede de novo e compara com o "antes".
#[tauri::command]
pub async fn medir_depois(
    process: String,
    seconds: u64,
) -> Result<crate::modules::prova::Comparacao, String> {
    use crate::modules::prova;

    let antes = prova::guardada().ok_or(
        "Não há medição inicial. Meça com o jogo aberto ANTES de aplicar as mudanças.",
    )?;

    let medicao = measure_frames(process, seconds).await?;

    let depois = prova::Prova {
        jogo: medicao.process,
        quando: crate::modules::changelog::now_timestamp(),
        fps: medicao.fps,
        low_1pct: medicao.low_1pct,
        engasgos_por_minuto: medicao.engasgos_por_minuto,
        segundos: medicao.seconds,
        confiavel: medicao.detalhe_confiavel,
    };

    Ok(prova::comparar(&antes, &depois))
}

/// Comando: a medição do "antes" que estiver guardada.
#[tauri::command]
pub fn prova_guardada() -> Option<crate::modules::prova::Prova> {
    crate::modules::prova::guardada()
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

// =========================================================== REPARO
//
// OS QUATRO COMANDOS DESTA SECAO LEVAM O ATRIBUTO `async` NA MARCA DE COMANDO
// DO TAURI, E ISSO NAO E DECORACAO. (A marca nao aparece escrita neste
// comentario de proposito: a guarda do fim do arquivo conta comandos
// procurando por ela no proprio fonte, e contaria um fantasma.)
//
// Sem o atributo, o Tauri classifica a funcao como `ExecutionContext::Blocking`
// (tauri-macros-2.6.3, `src/command/wrapper.rs`: o contexto so vira `Async` se
// `function.sig.asyncness.is_some()` ou se o atributo estiver escrito), e o
// corpo roda EM LINHA dentro do manipulador de invoke — que o wry chama de
// forma sincrona a partir do callback de IPC, na thread do laco de eventos.
// Um `DISM` de trinta minutos ali para de repintar a janela, o Windows marca
// "Nao Responde", o `app.emit("reparo-andamento")` enfileira trabalho para o
// mesmo laco travado (o painel de andamento fica vazio) e — o pior — o
// `reparo_cancelar`, que tambem e uma mensagem de IPC, so consegue executar
// depois que a tarefa que ele deveria interromper ja terminou. O botao
// Interromper existiria sem funcionar, e a unica saida do cliente seria matar
// o programa no meio de uma escrita do DISM: exatamente o que
// `cancelar_e_seguro: false` existe para evitar.
//
// O atributo e valido numa funcao SINCRONA que recebe `State<'_, AppState>`:
// a restricao do Tauri contra referencias na entrada (`wrapper.rs`, o bloco
// `async_command_check`) so e emitida quando `asyncness.is_some()`. Com a
// funcao sincrona e o atributo presente, o Tauri gera o corpo assincrono e
// chama a funcao dentro dele — o que o proprio macro rotula `sync_threadpool`.

/// Monta a trava do disco a partir de um diagnóstico de verdade.
///
/// Só existe no Windows: `HealthReport` e `DiscoSaudavel` vêm de
/// `modules::windows`, que nem compila fora dele.
#[cfg(target_os = "windows")]
fn disco_saudavel_agora() -> crate::modules::windows::reparo::DiscoSaudavel {
    let relatorio = crate::modules::windows::health::analyze();
    crate::modules::windows::reparo::DiscoSaudavel::a_partir_do_relatorio(&relatorio)
}

/// Uma ferramenta de reparo oferecida nesta máquina, com tudo que a tela
/// precisa para avisar ANTES do clique — duração típica, se cancelar é
/// seguro e os avisos de segurança — lido de `Receita`
/// (`modules::windows::reparo`) e nunca reescrito à mão na tela.
///
/// NÃO leva `programa` nem `args`: são detalhe de execução, não informação
/// que o cliente precisa ver, e expô-los sem necessidade só aumentaria a
/// superfície que alguém poderia tentar forjar numa chamada direta.
/// `titulo`/`descricao` também ficam de fora de propósito — continuam sendo
/// texto de apresentação escrito pela própria tela, e não um FATO DE
/// SEGURANÇA como o aviso do `/ResetBase`; só o que muda o risco de um clique
/// tem que ter dono único no backend.
#[derive(Serialize)]
pub struct FerramentaDeReparo {
    /// O mesmo nome que `reparo_executar` espera no campo `ferramenta`.
    pub nome: String,
    pub minutos_tipicos: (u32, u32),
    pub cancelar_e_seguro: bool,
    pub aviso: Option<String>,
    /// Só `true` em `LimparWinSxS`: só ela oferece o interruptor do
    /// `/ResetBase`.
    pub oferece_reset_base: bool,
    /// O aviso de que ligar o `/ResetBase` tira a capacidade de desinstalar
    /// atualizações já aplicadas. Vem de `Receita` com `resetar_base: true`,
    /// e não de um texto próprio da tela — é a mesma garantia de fonte única
    /// que o aviso comum acima já tem.
    pub aviso_reset_base: Option<String>,
}

#[cfg(target_os = "windows")]
fn descrever_ferramenta(
    f: crate::modules::windows::reparo::Ferramenta,
    nome: &str,
) -> FerramentaDeReparo {
    use crate::modules::windows::reparo::{self, Ferramenta};

    let r = reparo::receita(&f);

    let (oferece_reset_base, aviso_reset_base) = match f {
        Ferramenta::LimparWinSxS { .. } => {
            let com_reset = reparo::receita(&Ferramenta::LimparWinSxS { resetar_base: true });
            (true, com_reset.aviso.map(str::to_string))
        }
        _ => (false, None),
    };

    FerramentaDeReparo {
        nome: nome.to_string(),
        minutos_tipicos: r.minutos_tipicos,
        cancelar_e_seguro: r.cancelar_e_seguro,
        aviso: r.aviso.map(str::to_string),
        oferece_reset_base,
        aviso_reset_base,
    }
}

/// O que dá para oferecer nesta máquina.
///
/// Fica em `LIVRES`: é leitura, e é o diagnóstico que mostra ao cliente que o
/// problema dele existe antes de qualquer cobrança.
///
/// O disco NÃO chega como parâmetro da tela — o comando lê o `HealthReport`
/// aqui dentro e monta o próprio `DiscoSaudavel`. Um `bool` vindo do
/// frontend reabriria exatamente o buraco que `DiscoSaudavel` foi criado
/// para fechar: a mesma regra do fluxo de compra, em que só um código de
/// cupom viaja do cliente e é o servidor sozinho quem decide o preço.
#[tauri::command(async)]
pub fn reparo_disponivel(state: State<'_, AppState>) -> Vec<FerramentaDeReparo> {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::reparo::{self, Ferramenta};

        let mut lista = vec![
            descrever_ferramenta(Ferramenta::VerificarArquivos, "VerificarArquivos"),
            descrever_ferramenta(Ferramenta::RepararImagem, "RepararImagem"),
            descrever_ferramenta(Ferramenta::VerificarDisco, "VerificarDisco"),
            descrever_ferramenta(Ferramenta::AnalisarWinSxS, "AnalisarWinSxS"),
            descrever_ferramenta(
                Ferramenta::LimparWinSxS { resetar_base: false },
                "LimparWinSxS",
            ),
        ];

        let medicao = estado_do_disco(&state);

        // DUAS PROVAS, E NÃO UMA. `DiscoSaudavel` diz que o disco aguenta a
        // operação; `EstadoDoDisco` diz que houve MEDIÇÃO que a justifique.
        // Faltando qualquer uma, o botão não existe — e é assim que ele deixa
        // de ser um clique disponível para quem tem um NTFS limpo.
        let disco = disco_saudavel_agora();
        if reparo::consertar_disco_e_permitido(&disco) && medicao.autoriza_consertar() {
            lista.push(descrever_ferramenta(Ferramenta::ConsertarDisco, "ConsertarDisco"));
        }

        if medicao.tem_conserto_agendado() {
            lista.push(descrever_ferramenta(
                Ferramenta::DesmarcarConsertoDoDisco,
                "DesmarcarConsertoDoDisco",
            ));
        }

        lista
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Vec::new()
    }
}

/// Lê o que se sabe do disco nesta sessão.
///
/// Tranca envenenada devolve `SemVerificacao`, que é o estado que NÃO autoriza
/// nada: aqui "não sei" fecha a porta, como em todo o resto deste caminho.
#[cfg(target_os = "windows")]
fn estado_do_disco(state: &State<'_, AppState>) -> crate::modules::windows::reparo::EstadoDoDisco {
    state.disco.lock().map(|d| *d).unwrap_or_default()
}

/// O tom com que a tela deve colorir `UltimoResultadoReparo` — "ok",
/// "atencao" ou "erro" na borda do IPC.
///
/// Existe para a tela nunca precisar decidir isso sozinha. A versão anterior
/// devolvia só a frase, e a tela escolhia a cor comparando o INÍCIO do texto
/// (`resultado.startsWith("Corrigiu ")`) — só que a frase de
/// `CorrigiuEmParte` ("Corrigiu 2 arquivo(s), mas 1 continua...") também
/// começa com "Corrigiu ", e a tela pintava de verde um resultado que o
/// próprio comentário deste arquivo já dizia que NÃO é sucesso. A cor agora
/// nasce de `ResultadoSfc::severidade()` — do dado estruturado, não da
/// prosa — então essa colisão de vocabulário deixou de ser possível.
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TomResultado {
    Ok,
    Atencao,
    Erro,
}

#[cfg(target_os = "windows")]
impl From<crate::modules::windows::cbslog::Severidade> for TomResultado {
    fn from(s: crate::modules::windows::cbslog::Severidade) -> Self {
        use crate::modules::windows::cbslog::Severidade;

        match s {
            Severidade::Ok => TomResultado::Ok,
            Severidade::Atencao => TomResultado::Atencao,
            Severidade::Erro => TomResultado::Erro,
        }
    }
}

/// O mesmo princípio, agora para o desfecho de uma EXECUÇÃO de reparo — este
/// é o defeito da vez: a tela decidia a cor comparando a frase formatada
/// (`desfecho === "Terminou."`), e `CorrigiuEmParte` já provou que prosa e
/// cor divergem. O tom nasce aqui, da variante de `Desfecho`, antes de a
/// frase existir.
#[cfg(target_os = "windows")]
impl From<&crate::modules::windows::tarefa_longa::Desfecho> for TomResultado {
    fn from(d: &crate::modules::windows::tarefa_longa::Desfecho) -> Self {
        use crate::modules::windows::tarefa_longa::Desfecho;

        match d {
            Desfecho::Terminou { codigo: 0 } => TomResultado::Ok,
            // Código diferente de zero é falha do programa — mesmo tom que
            // "não consegui verificar" já usa em `UltimoResultadoReparo`.
            Desfecho::Terminou { codigo: _ } => TomResultado::Erro,
            // Cancelar foi uma escolha do cliente, não uma falha: nem verde
            // (nada terminou) nem vermelho (ninguém errou).
            Desfecho::Cancelada => TomResultado::Atencao,
            Desfecho::NaoComecou { .. } => TomResultado::Erro,
        }
    }
}

/// O desfecho de `reparo_executar`, com o tom já decidido — mesma forma de
/// `UltimoResultadoReparo`, pelo mesmo motivo: a tela lê `tom`, nunca
/// compara a prosa de `texto`.
#[derive(Serialize)]
pub struct DesfechoReparo {
    pub tom: TomResultado,
    pub texto: String,
}

/// O resultado da última verificação de arquivos de sistema, com o tom já
/// decidido — a tela lê `tom`, nunca a prosa de `texto`.
#[derive(Serialize)]
pub struct UltimoResultadoReparo {
    pub tom: TomResultado,
    pub texto: String,
}

/// O resultado da última verificação de arquivos de sistema.
///
/// Fica em `LIVRES`: é leitura de um registro que o Windows já escreveu.
#[tauri::command(async)]
pub fn reparo_ultimo_resultado() -> UltimoResultadoReparo {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::cbslog::{self, ResultadoSfc};

        // O `unwrap_or_default()` que estava aqui ENGOLIA o erro de leitura e
        // entregava uma string vazia ao `interpretar`, que respondia "o
        // registro do Windows não trouxe nenhuma verificação". Num programa
        // aberto sem elevação — o CBS.log só abre como administrador — essa
        // frase nomeia a causa errada: não é que o Windows não verificou, é
        // que nós não conseguimos ler. O cliente então clicava em Executar e
        // recebia um vermelho seco "Terminou com o código 1".
        let resultado = match std::fs::read_to_string(cbslog::caminho_do_log()) {
            Ok(conteudo) => cbslog::interpretar(&conteudo),
            Err(e) => ResultadoSfc::NaoSei {
                motivo: match e.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        "o registro do Windows só abre com permissão de administrador".to_string()
                    }
                    std::io::ErrorKind::NotFound => {
                        "o registro do Windows ainda não existe nesta máquina".to_string()
                    }
                    _ => format!("não consegui abrir o registro do Windows ({})", e),
                },
            },
        };
        let tom = TomResultado::from(resultado.severidade());

        let texto = match resultado {
            // Este é o resultado mais comum, e é um resultado BOM. A tela diz
            // isso com todas as letras, sem inventar benefício — mesma regra
            // do `prova.rs`, que se recusa a chamar ruído de ganho.
            ResultadoSfc::SemCorrupcao => "Nenhuma corrupção encontrada.".into(),
            ResultadoSfc::Corrigiu { quantos } => {
                format!("Corrigiu {} arquivo(s) corrompido(s).", quantos)
            }
            // Misto: parte foi consertada, parte não. Isto NÃO é sucesso —
            // ainda sobra corrupção na máquina, e dizer só "corrigiu" faria o
            // cliente achar que o problema acabou quando não acabou. O tom
            // (`Atencao`, não `Ok`) já carrega essa distinção sozinho.
            ResultadoSfc::CorrigiuEmParte {
                corrigidos,
                restantes,
            } => format!(
                "Corrigiu {} arquivo(s), mas {} continuam corrompidos e sem \
                 conserto. O próximo passo é reparar a imagem do Windows.",
                corrigidos, restantes
            ),
            ResultadoSfc::NaoConseguiu { quantos } => format!(
                "Encontrou {} arquivo(s) corrompido(s) e não conseguiu corrigir. \
                 O próximo passo é reparar a imagem do Windows.",
                quantos
            ),
            // Corrigiu tudo que deu para nomear, mas o registro também
            // tinha linha de falha sem nome legível — não dá para garantir
            // que não sobrou corrupção nelas. O tom (`Atencao`, não `Ok`)
            // já carrega essa ressalva sozinho.
            ResultadoSfc::CorrigiuComRessalva {
                quantos,
                linhas_ilegiveis,
            } => format!(
                "Corrigiu {} arquivo(s) corrompido(s), mas o registro do Windows tinha \
                 {} linha(s) de falha que não deram para identificar — não dá para \
                 garantir que não sobrou corrupção nelas. O próximo passo é rodar a \
                 verificação de novo: se a corrupção que sobrou já foi consertada, a \
                 próxima passagem do `sfc` escreve um registro limpo e legível; se não \
                 foi, ela aparece de novo, desta vez nomeada.",
                quantos, linhas_ilegiveis
            ),
            ResultadoSfc::NaoSei { motivo } => format!("Não consegui conferir: {}.", motivo),
        };

        UltimoResultadoReparo { tom, texto }
    }

    #[cfg(not(target_os = "windows"))]
    {
        UltimoResultadoReparo {
            tom: TomResultado::Atencao,
            texto: "Disponível apenas no Windows.".to_string(),
        }
    }
}

/// Roda uma ferramenta de reparo, transmitindo o andamento pelo evento
/// `reparo-andamento`.
///
/// Exige licença: é correção, como todas as outras.
///
/// O parâmetro chama `resetbase`, uma palavra só, e não `reset_base` — a
/// tela chamaria isto de `resetbase` (sem separador) ou dependeria da
/// conversão automática de nome que o Tauri faz entre camelCase no
/// JavaScript e snake_case no Rust. A conversão provavelmente está certa,
/// mas o `/ResetBase` é consequente demais (custa a capacidade de
/// desinstalar atualizações já aplicadas, sem volta) para valer a pena
/// alguém ter que raciocinar sobre ela. Uma palavra só fecha essa dúvida de
/// vez — a mesma lógica que fez `DiscoSaudavel` virar tipo em vez de `bool`.
#[tauri::command(async)]
pub fn reparo_executar(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    ferramenta: String,
    resetbase: bool,
) -> Result<DesfechoReparo, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::reparo::{self, Ferramenta};
        use crate::modules::windows::tarefa_longa::Desfecho;
        use tauri::Emitter;

        let escolhida = match ferramenta.as_str() {
            "VerificarArquivos" => Ferramenta::VerificarArquivos,
            "RepararImagem" => Ferramenta::RepararImagem,
            "VerificarDisco" => Ferramenta::VerificarDisco,
            "ConsertarDisco" => {
                // AS DUAS TRAVAS SÃO CONFERIDAS AQUI TAMBÉM, e não só na tela
                // que chamou `reparo_disponivel`: a tela pode ser contornada,
                // esta chamada não — tudo é lido de novo, na hora, e nada do
                // que veio do cliente é levado em conta.
                let disco = disco_saudavel_agora();
                if !reparo::consertar_disco_e_permitido(&disco) {
                    return Err(
                        "O disco desta máquina não está em condições para isso. \
                         Consertar a estrutura num disco que já falha costuma \
                         terminar de estragá-lo."
                            .into(),
                    );
                }

                if !estado_do_disco(&state).autoriza_consertar() {
                    return Err(
                        "Nada foi encontrado no disco para consertar. Rode antes \
                         \"Verificar o disco\": sem achado, não há motivo para \
                         reiniciar a sua máquina."
                            .into(),
                    );
                }

                Ferramenta::ConsertarDisco
            }
            "DesmarcarConsertoDoDisco" => {
                if !estado_do_disco(&state).tem_conserto_agendado() {
                    return Err("Não há conserto de disco agendado para desmarcar.".into());
                }
                Ferramenta::DesmarcarConsertoDoDisco
            }
            "AnalisarWinSxS" => Ferramenta::AnalisarWinSxS,
            "LimparWinSxS" => Ferramenta::LimparWinSxS {
                resetar_base: resetbase,
            },
            outra => return Err(format!("não conheço a ferramenta `{}`", outra)),
        };

        // O `ConsertarDisco` sozinho pode mentir: se uma sessão anterior
        // desmarcou um conserto (`DesmarcarConsertoDoDisco`, `chkntfs /X`),
        // o volume fica EXCLUÍDO do boot check para sempre — não só naquela
        // vez. Sem reincluir agora, o `fsutil dirty set` abaixo sai 0, o
        // retorno diz "agendado", e o `autochk` pula o volume no próximo
        // boot mesmo assim. `receita_reinclusao_do_disco` (reparo.rs) desfaz
        // isso, e quem sequencia as duas chamadas é este executor — não
        // `reparo.rs`, que só descreve receitas, e não `Receita`, que carrega
        // um programa só. Rodar a reinclusão sempre, mesmo sem `/X` anterior,
        // é seguro: ela só restaura o padrão do Windows.
        if escolhida == Ferramenta::ConsertarDisco {
            let reinclusao = reparo::receita_reinclusao_do_disco();
            let args_reinclusao: Vec<&str> =
                reinclusao.args.iter().map(|s| s.as_str()).collect();
            let app_reinclusao = app.clone();
            let desfecho_reinclusao =
                state
                    .reparo
                    .rodar(reinclusao.programa, &args_reinclusao, move |a| {
                        let _ = app_reinclusao.emit("reparo-andamento", &a);
                    })?;

            // Falhar aqui e seguir para o `fsutil` mesmo assim seria recriar
            // a mentira que esta reinclusão existe para fechar: o bit sujo
            // marcado (o `fsutil` quase sempre funciona) enquanto o volume
            // continua fora do boot check porque a reinclusão não pegou. O
            // cliente não pode ouvir "agendado" nesse caso — não seria
            // verdade.
            if !reparo::reinclusao_deu_certo(&desfecho_reinclusao) {
                return Err(
                    "Não consegui preparar o disco para o conserto (a \
                     reinclusão no boot check falhou). Nada foi agendado."
                        .into(),
                );
            }
        }

        let r = reparo::receita(&escolhida);
        let args: Vec<&str> = r.args.iter().map(|s| s.as_str()).collect();

        let desfecho = state.reparo.rodar(r.programa, &args, move |a| {
            let _ = app.emit("reparo-andamento", &a);
        })?;

        // O QUE ACABOU DE ACONTECER É A ÚNICA FONTE DA PRÓXIMA OFERTA.
        // Registrado a partir do desfecho real, e não de um `true` posto à mão
        // depois de um clique: um `/scan` cancelado, ou que devolveu "não
        // consegui verificar", apaga a autorização em vez de deixá-la de pé.
        if let Ok(mut atual) = state.disco.lock() {
            *atual = atual.apos_execucao(&escolhida, &desfecho);
        }

        // O tom é lido da VARIANTE, antes de `desfecho` ser consumido pelo
        // `match` que monta a frase — é a mesma ordem de
        // `reparo_ultimo_resultado`, e existe pelo mesmo motivo: se o tom
        // nascesse depois, olhando para `texto`, seria a prosa decidindo de
        // novo, só que num lugar mais difícil de notar.
        let tom = TomResultado::from(&desfecho);
        let texto = match desfecho {
            Desfecho::Terminou { codigo: 0 } => "Terminou.".into(),
            Desfecho::Terminou { codigo } => format!("Terminou com o código {}.", codigo),
            Desfecho::Cancelada => "Interrompida por você.".into(),
            Desfecho::NaoComecou { motivo } => motivo,
        };

        Ok(DesfechoReparo { tom, texto })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state, ferramenta, resetbase);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Interrompe a ferramenta de reparo em andamento.
///
/// Fica em `LIVRES` DE PROPÓSITO, pelo mesmo motivo que `revert` fica: uma
/// licença que vence no meio de um `DISM` de vinte minutos não pode deixar o
/// cliente preso nele.
#[tauri::command(async)]
pub fn reparo_cancelar(state: State<'_, AppState>) -> bool {
    #[cfg(target_os = "windows")]
    {
        state.reparo.cancelar()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        false
    }
}

// ============================================================ SUSPENSÃO

/// O que está congelado agora.
///
/// Fica em `LIVRES`: é leitura, e é a informação que faltava. Um cliente
/// abriu o gerenciador de tarefas, viu "Steam — Suspenso" (depois "Discord",
/// depois "Chrome"), e escreveu para o dono dizendo que o PC dele tinha
/// ficado "mt bugado" — porque a tela do Otimiza não mostrava nada disso e
/// não tinha botão nenhum para desfazer. Ele estava certo em concluir que
/// algo estava errado: o defeito não era ter suspendido, era não ter
/// avisado. Este comando é o aviso.
#[tauri::command]
pub fn congelados_agora() -> Vec<crate::modules::windows::suspend::Suspenso> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::suspend::congelados()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

/// Devolve todos os congelados, agora — sem esperar o jogo fechar.
///
/// Fica em `LIVRES` pelo mesmo motivo que `revert_optimization` e
/// `reparo_cancelar` ficam: desfazer o que o próprio produto fez nunca pode
/// depender de licença. Antes deste comando, socorrer o cliente do
/// incidente acima exigiu um script de PowerShell escrito à mão para
/// alguém que JÁ TINHA o produto instalado — e que já tinha, portanto,
/// tudo que precisava para se ajudar sozinho, se a tela tivesse oferecido.
/// Esta é a função que substitui aquele script.
#[tauri::command]
pub fn descongelar_agora() -> Result<usize, String> {
    #[cfg(target_os = "windows")]
    {
        let devolvidos = crate::modules::windows::suspend::retomar_tudo()?;
        Ok(devolvidos.len())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(0)
    }
}

// ========================================================== ATENDIMENTO

/// O relatório que o cliente cola no atendimento — versão, Windows, RAM,
/// congelados, mudanças aplicadas, disco e térmico, e o que não deu para ler.
///
/// Fica em `LIVRES`: é leitura, e é o diagnóstico que o produto já dá de
/// graça — inclusive porque é justamente esta a informação que faltou
/// quando um cliente com o produto JÁ INSTALADO precisou de um script de
/// PowerShell escrito à mão para alguém entender o que estava acontecendo
/// na máquina dele. Ver `modules::windows::suporte` para o porquê de cada
/// regra que o texto obedece.
/// SÍNCRONO NÃO: este comando chama `health::analyze()` e
/// `thermal::analyze()`, que o resto deste arquivo tira do runtime de
/// propósito — o primeiro conversa com o disco via WMI e a tela etiqueta os
/// dois com "~5 s" cada. Somados ao `Get-CimInstance` do sistema e ao resto,
/// dão 12 a 15 segundos NA THREAD PRINCIPAL: janela sem redesenhar, sem
/// arrastar, e o "Otimiza não está respondendo" do Windows. É exatamente a
/// queixa que o relatório de congelados existe para investigar, causada pelo
/// botão que investiga.
#[tauri::command(async)]
pub fn relatorio_de_suporte() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::suporte::montar(
            &crate::modules::windows::suporte::gerar(),
        ))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

// ========================================================= ATUALIZAÇÃO

/// O que a tela precisa para decidir se mostra a faixa de versão nova.
///
/// `comparacao` é a única coisa que a tela decide a partir de — nunca
/// `versao_publicada`, que é texto solto para exibir, não para comparar. Ver
/// a guarda `a_tela_nao_decide_cor_comparando_texto_do_backend`, que reprova
/// o build se `main.ts` comparar por igualdade ou substring um texto que
/// parece prosa.
#[derive(Debug, Clone, Serialize)]
pub struct AvisoDeVersao {
    pub comparacao: crate::modules::atualizacao::Comparacao,
    pub versao_publicada: Option<String>,
    pub pagina: Option<String>,
}

/// Pergunta ao GitHub se existe versão publicada mais nova que a instalada.
///
/// Fica em `LIVRES`: é leitura — pergunta ao GitHub e não altera nada neste
/// computador. Não instala nada sozinho: a resposta é só a faixa que o
/// cliente vê e, se quiser, clica para baixar por conta própria.
///
/// A versão instalada vem de `CARGO_PKG_VERSION`, o mesmo número que já
/// alimenta `relatorio_de_suporte` — não é hardcoded aqui de novo.
#[tauri::command]
pub async fn versao_mais_nova() -> AvisoDeVersao {
    let instalada = env!("CARGO_PKG_VERSION");

    match crate::modules::atualizacao::consultar_ultima().await {
        Some(ultima) => AvisoDeVersao {
            comparacao: crate::modules::atualizacao::comparar(instalada, &ultima.versao),
            versao_publicada: Some(ultima.versao),
            pagina: ultima.pagina,
        },
        // Falha de rede é silêncio, não alarme: `NaoSei` faz a tela não
        // mostrar nada, em vez de arriscar um "atualize" sem versão nenhuma
        // por trás.
        None => AvisoDeVersao {
            comparacao: crate::modules::atualizacao::Comparacao::NaoSei,
            versao_publicada: None,
            pagina: None,
        },
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
        "placa_de_video",
        "memoria_instalada",
        "monitores",
        "analyze_game_config",
        "preview_game_profile",
        "medir_antes",
        "medir_depois",
        "prova_guardada",
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
        "medir_perda_de_pacote",
        "measure_frames",
        "analyze_fivem",
        "analyze_citizenfx",
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
        "reparo_disponivel",
        "reparo_ultimo_resultado",
        "reparo_cancelar",
        "congelados_agora",
        "descongelar_agora",
        "relatorio_de_suporte",
        "versao_mais_nova",
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
        "reparo_executar",
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
        let marca = concat!("#[tauri::", "command");

        producao()
            .split(marca)
            .skip(1)
            // A marca aceita argumento: os comandos de reparo sao
            // `(async)` para nao rodarem na thread da interface. Procurar
            // so pela forma sem argumento deixaria os quatro invisiveis
            // para a guarda — que existe justamente para nenhum comando
            // escapar da classificacao.
            .filter(|bloco| bloco.starts_with(']') || bloco.starts_with("(async)]"))
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

    /// A marca sai do nome que o Windows dá à placa.
    ///
    /// Na dúvida, `desconhecida` — e a tela pergunta em vez de chutar. Chutar
    /// aqui pintaria a placa de verde para quem tem uma Radeon, que é o tipo de
    /// erro que faz o cliente duvidar de todo o resto que a tela afirma.
    #[test]
    fn a_marca_da_placa_sai_do_nome() {
        for (nome, esperado) in [
            ("NVIDIA GeForce GTX 1650", "nvidia"),
            ("NVIDIA GeForce RTX 4070 Ti", "nvidia"),
            ("AMD Radeon RX 6600", "amd"),
            ("Radeon(TM) Graphics", "amd"),
            ("Intel(R) UHD Graphics 630", "intel"),
            ("Intel(R) Arc(TM) A750", "intel"),
            // Uma placa que nenhum dos três nomes cobre não vira chute.
            ("Microsoft Basic Display Adapter", "desconhecida"),
            ("", "desconhecida"),
        ] {
            assert_eq!(
                super::marca_da_placa(nome),
                esperado,
                "`{}` foi classificada errado",
                nome
            );
        }
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

    #[test]
    fn verificar_e_livre_e_consertar_pede_licenca() {
        // A regra da casa: diagnóstico livre, correção paga. Se a verificação
        // passar a exigir licença, o cliente não consegue nem descobrir que o
        // problema dele existe — e é justamente esse achado que vende.
        assert!(LIVRES.contains(&"reparo_disponivel"));
        assert!(LIVRES.contains(&"reparo_ultimo_resultado"));
        assert!(EXIGEM_LICENCA.contains(&"reparo_executar"));

        // Cancelar é livre DE PROPÓSITO, pelo mesmo motivo que `revert` é: uma
        // licença vencida no meio de um DISM não pode prender a pessoa nele.
        assert!(LIVRES.contains(&"reparo_cancelar"));
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

    /// O tom de `DesfechoReparo` nasce da VARIANTE de `Desfecho`, nunca da
    /// frase formatada — é a mesma regra de `UltimoResultadoReparo`, agora
    /// no vizinho que ainda não tinha essa proteção.
    #[cfg(target_os = "windows")]
    #[test]
    fn o_tom_do_desfecho_vem_da_variante_nao_da_prosa() {
        use crate::modules::windows::tarefa_longa::Desfecho;

        assert!(matches!(
            super::TomResultado::from(&Desfecho::Terminou { codigo: 0 }),
            super::TomResultado::Ok
        ));
        assert!(matches!(
            super::TomResultado::from(&Desfecho::Terminou { codigo: 1 }),
            super::TomResultado::Erro
        ));
        assert!(matches!(
            super::TomResultado::from(&Desfecho::Cancelada),
            super::TomResultado::Atencao
        ));
        assert!(matches!(
            super::TomResultado::from(&Desfecho::NaoComecou {
                motivo: "Já existe uma tarefa em andamento.".into()
            }),
            super::TomResultado::Erro
        ));
    }

    // =============================== A TELA NÃO DECIDE COR COMPARANDO PROSA

    /// Extrai, de uma linha de TypeScript, cada literal de string (aspas
    /// simples, duplas ou template sem interpolação) junto com o texto que
    /// vem ANTES dela — é nesse texto anterior que mora o operador de
    /// comparação que denuncia o defeito.
    ///
    /// Trabalha em `char`, não em byte: este arquivo tem acento
    /// ("não", "código"), e indexar por byte cortaria um caractere
    /// multibyte ao meio.
    fn literais_com_contexto(linha: &str) -> Vec<(String, String)> {
        let chars: Vec<char> = linha.chars().collect();
        let mut achados = Vec::new();
        let mut i = 0;

        while i < chars.len() {
            let abre = chars[i];
            if abre == '"' || abre == '\'' || abre == '`' {
                let contexto: String = chars[..i].iter().collect();
                let mut j = i + 1;
                let mut conteudo = String::new();

                while j < chars.len() {
                    if chars[j] == '\\' && j + 1 < chars.len() {
                        j += 2;
                        continue;
                    }
                    if chars[j] == abre {
                        break;
                    }
                    conteudo.push(chars[j]);
                    j += 1;
                }

                let resto: String = chars.get(j + 1..).unwrap_or(&[]).iter().collect();
                achados.push((contexto, conteudo, resto));
                i = j + 1;
            } else {
                i += 1;
            }
        }

        achados
            .into_iter()
            .map(|(contexto, conteudo, _resto)| (contexto, conteudo))
            .collect()
    }

    /// A impressão digital de prosa do backend: tem espaço, ou termina em
    /// pontuação de frase. Um rótulo que a PRÓPRIA tela inventou — um id de
    /// aba, o nome de uma ferramenta, um `data-state` — é uma palavra só,
    /// sem espaço e sem ponto final: "Applied", "VerificarArquivos",
    /// "localhost". "Terminou.", "Corrigiu ", "Interrompida por você." não
    /// são: nasceram como frase, no backend, para gente ler — não para a
    /// tela comparar.
    fn parece_prosa_do_backend(literal: &str) -> bool {
        if literal.trim().is_empty() {
            return false;
        }
        literal.contains(' ') || literal.trim_end().ends_with(['.', '!', '?'])
    }

    /// O contexto termina com um operador que decide alguma coisa a partir
    /// do valor: igualdade, prefixo, substring, posição, ou um `case` de
    /// `switch`. Estes são os únicos jeitos que este arquivo tem de tomar
    /// uma decisão comparando uma string — e é exatamente o repertório que
    /// já causou o defeito três vezes (igualdade exata, `startsWith`,
    /// prefixo por acidente).
    fn termina_em_operador_de_decisao(contexto: &str) -> bool {
        let c = contexto.trim_end();
        c.ends_with("===")
            || c.ends_with("!==")
            || c.ends_with(".startsWith(")
            || c.ends_with(".endsWith(")
            || c.ends_with(".includes(")
            || c.ends_with(".indexOf(")
            || c.trim_start().ends_with("case")
    }

    /// Tira `//` até o fim da linha e `/* ... */` (mesmo cruzando linha) do
    /// texto, ANTES da varredura de literais.
    ///
    /// Sem isto a guarda reprova por CITAÇÃO, não por defeito: esta base
    /// explica no comentário justamente os defeitos que já consertou — e
    /// `desfecho === "Terminou."`, o antipadrão desta própria tarefa, é
    /// exatamente o tipo de frase que um comentário de "antes era assim"
    /// cita entre aspas normais. Achar essa citação não prova nada sobre o
    /// código.
    ///
    /// Caminha o MESMO estado de aspas que `literais_com_contexto` usa —
    /// comentário é o terceiro estado da mesma máquina — para não confundir
    /// `//` ou `/*` que apareçam DENTRO de uma string (`"https://..."`) com
    /// o início de um comentário de verdade.
    ///
    /// Cada caractere de comentário vira espaço, mas toda quebra de linha do
    /// original sobrevive: é o que mantém os números de linha que a guarda
    /// relata batendo com o arquivo de verdade, mesmo depois da limpeza.
    fn remover_comentarios(fonte: &str) -> String {
        let chars: Vec<char> = fonte.chars().collect();
        let mut saida = String::with_capacity(chars.len());
        let mut i = 0;
        let mut aspas: Option<char> = None;

        while i < chars.len() {
            let c = chars[i];

            if let Some(abre) = aspas {
                saida.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    saida.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == abre {
                    aspas = None;
                }
                i += 1;
                continue;
            }

            if c == '"' || c == '\'' || c == '`' {
                aspas = Some(c);
                saida.push(c);
                i += 1;
                continue;
            }

            if c == '/' && chars.get(i + 1) == Some(&'/') {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }

            if c == '/' && chars.get(i + 1) == Some(&'*') {
                i += 2;
                while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                    if chars[i] == '\n' {
                        saida.push('\n');
                    }
                    i += 1;
                }
                i = (i + 2).min(chars.len());
                continue;
            }

            saida.push(c);
            i += 1;
        }

        saida
    }

    /// A varredura completa: tira comentário, depois procura literal de
    /// prosa comparado por operador de decisão. Compartilhada pela guarda de
    /// verdade (que lê `main.ts` do disco) e pelo teste que prova, com
    /// fontes sintéticas, que comentário não dispara e código equivalente
    /// dispara.
    fn achados_de_prosa_comparada(fonte: &str) -> Vec<String> {
        let sem_comentarios = remover_comentarios(fonte);
        let mut achados = Vec::new();

        for (numero, linha) in sem_comentarios.lines().enumerate() {
            for (contexto, literal) in literais_com_contexto(linha) {
                if termina_em_operador_de_decisao(&contexto) && parece_prosa_do_backend(&literal)
                {
                    achados.push(format!(
                        "linha {}: `{}\"{}\"…`",
                        numero + 1,
                        contexto.trim_start(),
                        literal
                    ));
                }
            }
        }

        achados
    }

    #[test]
    fn a_tela_nao_decide_cor_comparando_texto_do_backend() {
        // ESTE DEFEITO JÁ VOLTOU TRÊS VEZES:
        //   1. `Corrigiu` escondendo arquivos não reparados
        //   2. a tela pintando `CorrigiuEmParte` de verde por prefixo
        //   3. o desfecho da execução, por igualdade exata
        // Da terceira vez vira regra, não conserto.
        //
        // A guarda não procura as três strings de hoje — isso só provaria
        // que ninguém vai escrever "desfecho === \"Terminou.\"" de novo,
        // e a quarta vez viria com outro nome de variável. Ela procura a
        // FORMA do defeito: um literal que parece PROSA (tem espaço, ou
        // termina em pontuação de frase — ninguém escreve um id de aba ou
        // um nome de ferramenta assim) comparado por igualdade, prefixo,
        // substring, posição, ou `case`. Um id, um nome de aba, um
        // `custom_id` — a própria tela inventou essas palavras, e elas não
        // têm espaço nem ponto final, então não acionam a guarda.
        //
        // `../src/main.ts` relativo ao diretório de trabalho do teste
        // funciona hoje, mas depende de onde `cargo test` é chamado —
        // `CARGO_MANIFEST_DIR` não depende disso: aponta sempre para
        // `src-tauri`, e o `main.ts` mora um nível acima, em `src/`.
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("src")
            .join("main.ts");
        let fonte = std::fs::read_to_string(&caminho)
            .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", caminho, e));

        let achados = achados_de_prosa_comparada(&fonte);

        assert!(
            achados.is_empty(),
            "a tela voltou a decidir por texto do backend:\n{}",
            achados.join("\n")
        );
    }

    #[test]
    fn a_guarda_da_prosa_reconhece_o_defeito_original() {
        // Canário da própria guarda: se `parece_prosa_do_backend` ou
        // `termina_em_operador_de_decisao` regredirem a ponto de não
        // reconhecer mais o defeito que motivou esta tarefa, este teste
        // acusa antes que o guarda de verdade fique cego.
        assert!(parece_prosa_do_backend("Terminou."));
        assert!(parece_prosa_do_backend("Interrompida por você."));
        assert!(parece_prosa_do_backend("Corrigiu "));
        assert!(!parece_prosa_do_backend("Applied"));
        assert!(!parece_prosa_do_backend("VerificarArquivos"));

        assert!(termina_em_operador_de_decisao("desfecho === "));
        assert!(termina_em_operador_de_decisao("resultado.startsWith("));
        assert!(termina_em_operador_de_decisao("    case "));
        assert!(!termina_em_operador_de_decisao("const titulo = "));
    }

    /// O conserto do fix round 1: um comentário CITANDO o antipadrão antigo
    /// entre aspas normais (não entre crases — o defeito que fez o próprio
    /// `main.ts:4312` passar por acidente) não pode reprovar a guarda, mas o
    /// mesmo texto fora de comentário, como código de verdade, precisa
    /// continuar disparando. Sem este par, um "conserto" na guarda que só
    /// afrouxasse a heurística de prosa passaria despercebido.
    #[test]
    fn comentario_que_cita_prosa_do_backend_nao_dispara_mas_o_codigo_equivalente_dispara() {
        let comentado = "// antes era: desfecho === \"Terminou.\"\n\
             // devolve \"Corrigiu 2 arquivos.\" quando conserta em parte\n\
             const x = 1;\n";
        assert!(
            achados_de_prosa_comparada(comentado).is_empty(),
            "um comentário citando a prosa antiga não pode reprovar a guarda"
        );

        let codigo = "const cor = desfecho === \"Terminou.\" ? \"ok\" : \"error\";\n";
        assert!(
            !achados_de_prosa_comparada(codigo).is_empty(),
            "o mesmo texto, fora de comentário, precisa continuar disparando"
        );
    }

    /// `//` e `/*` dentro de uma string não abrem comentário — sem isso uma
    /// URL como `"https://..."` perderia metade dela, apagada como se fosse
    /// comentário.
    #[test]
    fn remover_comentarios_nao_confunde_barra_dentro_de_string() {
        let fonte =
            "const url = \"https://exemplo.com/caminho\"; // comentario de verdade\nconst y = 2;";
        let limpo = remover_comentarios(fonte);

        assert!(
            limpo.contains("https://exemplo.com/caminho"),
            "apagou parte da string, tratando a barra dela como comentário: {:?}",
            limpo
        );
        assert!(
            !limpo.contains("comentario de verdade"),
            "não tirou o comentário de linha de verdade: {:?}",
            limpo
        );
    }

    /// A limpeza troca comentário por espaço, mas as quebras de linha do
    /// arquivo original têm que sobreviver — inclusive as que estão DENTRO
    /// de um bloco `/* */` de várias linhas — porque são elas que mantêm o
    /// número de linha que a guarda relata batendo com o arquivo de
    /// verdade.
    #[test]
    fn remover_comentarios_preserva_a_contagem_de_linhas() {
        let fonte = "linha1\n/* bloco\nde duas\nlinhas */\nlinha5";
        let limpo = remover_comentarios(fonte);

        assert_eq!(limpo.lines().count(), fonte.lines().count());
    }
}
