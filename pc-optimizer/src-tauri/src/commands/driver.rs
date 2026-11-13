// Driver de vídeo: ajustes da NVIDIA, AMD e Intel, a preferência de GPU por programa e o cache de shaders,
// que é do driver e não do jogo.

use super::*;

#[tauri::command]
#[cfg_attr(test, specta::specta)]
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

#[tauri::command]
#[cfg_attr(test, specta::specta)]
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

#[tauri::command]
#[cfg_attr(test, specta::specta)]
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

/// Em notebook com duas placas é o maior ganho que o produto dá, sem administrador nem reinício.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
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
        crate::modules::windows::WindowsOptimizer::new().set_gpu_preference(
            &caminho,
            preferencia,
            &mut log,
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (caminho, desempenho, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Só leitura. Sem placa NVIDIA, a frase que diz isso.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn ajustes_do_driver_nvidia(
    state: State<'_, AppState>,
) -> Result<PainelDoDriver, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::changelog::ChangeRecord;
        use crate::modules::windows::nvdriver::LimiteNaTela;

        // A trava do histórico é solta antes do driver: carregar a DLL não prende quem quer aplicar outra coisa.
        let (aplicados, limites) = {
            let log = state.changes.lock().await;

            let aplicados: Vec<String> = log
                .applied()
                .iter()
                .map(|entrada| entrada.optimization_id.clone())
                .collect();

            let limites: Vec<LimiteNaTela> = log
                .applied()
                .iter()
                .flat_map(|entrada| {
                    entrada
                        .changes
                        .iter()
                        .filter_map(move |mudanca| match mudanca {
                            ChangeRecord::LimiteNvidia {
                                executavel, fps, ..
                            } => Some(LimiteNaTela {
                                executavel: executavel.clone(),
                                fps: *fps,
                                historico: entrada.optimization_id.clone(),
                            }),
                            _ => None,
                        })
                })
                .collect();

            (aplicados, limites)
        };

        tokio::task::spawn_blocking(move || {
            crate::modules::windows::nvdriver::painel(
                |id| aplicados.iter().any(|a| a == id),
                limites,
            )
        })
        .await
        .map_err(|e| format!("Falha ao ler o driver da NVIDIA: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `EXIGEM_LICENCA`. O desfazer é o `revert_optimization`, com o id que o painel recebe pronto.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn aplicar_ajuste_nvidia(
    opcao: String,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new().aplicar_ajuste_nvidia(&opcao, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (opcao, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct PainelAmd {
    pub estado: String,
    pub nota: String,
    pub ajustes: Option<crate::modules::windows::amddriver::Ajustes>,
    pub sensores: Option<crate::modules::windows::amddriver::Sensores>,
    /// Ids que o OTIMIZA aplicou e não desfez: o que a pessoa ligou à mão não aparece aqui.
    pub aplicados: Vec<String>,
}

/// O que o driver AMD tem e como está. `LIVRES`: só lê.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn driver_amd(state: State<'_, AppState>) -> Result<PainelAmd, String> {
    let aplicados = aplicados_com_prefixo(&state, "amd-").await;
    tokio::task::spawn_blocking(move || {
        use crate::modules::windows::amddriver;
        let estado = amddriver::estado();
        let disponivel = estado == amddriver::Adlx::Disponivel;
        PainelAmd {
            estado: format!("{:?}", estado),
            nota: amddriver::nota_do_estado(estado).to_string(),
            ajustes: disponivel.then(amddriver::ler_ajustes).and_then(|r| r.ok()),
            sensores: disponivel.then(amddriver::sensores).and_then(|r| r.ok()),
            aplicados,
        }
    })
    .await
    .map_err(|e| format!("Falha ao ler o driver AMD: {}", e))
}

/// `EXIGEM_LICENCA`. `opcao`: `amd-antilag`, `amd-afmf`, `amd-rsr` ou `amd-frtc:<fps>`. O desfazer é o `revert_optimization`.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn aplicar_ajuste_amd(
    opcao: String,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    let mut log = state.changes.lock().await;
    crate::modules::windows::WindowsOptimizer::new().aplicar_ajuste_amd(&opcao, &mut log)
}

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct PainelIntel {
    pub estado: String,
    pub nota: String,
    pub ajustes: Option<crate::modules::windows::inteldriver::Ajustes>,
    pub aplicados: Vec<String>,
}

async fn aplicados_com_prefixo(state: &State<'_, AppState>, prefixo: &str) -> Vec<String> {
    state
        .changes
        .lock()
        .await
        .applied()
        .iter()
        .map(|a| a.optimization_id.clone())
        .filter(|id| id.starts_with(prefixo))
        .collect()
}

/// O que o driver Intel tem e como está. `LIVRES`: só lê.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn driver_intel(state: State<'_, AppState>) -> Result<PainelIntel, String> {
    let aplicados = aplicados_com_prefixo(&state, "intel-").await;
    tokio::task::spawn_blocking(move || {
        use crate::modules::windows::inteldriver;
        let estado = inteldriver::estado();
        PainelIntel {
            estado: format!("{:?}", estado),
            nota: inteldriver::nota_do_estado(estado).to_string(),
            ajustes: (estado == inteldriver::Igcl::Disponivel)
                .then(inteldriver::ler_ajustes)
                .and_then(|r| r.ok()),
            aplicados,
        }
    })
    .await
    .map_err(|e| format!("Falha ao ler o driver Intel: {}", e))
}

/// `EXIGEM_LICENCA`. `opcao`: `intel-latencia` ou `intel-limite:<fps>`. O desfazer é o `revert_optimization`.
#[cfg(target_os = "windows")]
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn aplicar_ajuste_intel(
    opcao: String,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    let mut log = state.changes.lock().await;
    crate::modules::windows::WindowsOptimizer::new().aplicar_ajuste_intel(&opcao, &mut log)
}

/// `EXIGEM_LICENCA`. O desfazer é o `revert_optimization`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn limitar_fps_nvidia(
    executavel: String,
    fps: u32,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        let caminho = crate::modules::windows::frames::caminho_do_processo(&executavel);
        crate::modules::windows::WindowsOptimizer::new().limitar_fps_nvidia(
            &executavel,
            caminho.as_deref(),
            fps,
            &mut log,
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (executavel, fps, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}
