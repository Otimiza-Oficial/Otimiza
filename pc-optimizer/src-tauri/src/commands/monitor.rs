// Medição contínua: métricas, monitoramento, baseline, histórico de desempenho e o diagnóstico ao vivo.
// É o que a tela lê o tempo todo; nada aqui altera o computador.

use super::*;

#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn get_performance_metrics(
    state: State<'_, AppState>,
) -> Result<PerformanceMetrics, String> {
    let mut monitor = state.monitor.lock().await;
    monitor.collect_metrics().await
}

/// NÃO mede mira nem olha dentro do jogo: duas chaves do registro do usuário. `intervalos_us` vêm da tela; lista
/// curta dá taxa ausente, nunca zero.
#[cfg(target_os = "windows")]
#[tauri::command(async)]
pub fn caminho_do_mouse(intervalos_us: Vec<u64>) -> CaminhoNaTela {
    use crate::modules::mouse;

    // Da janela DESTE aplicativo: nenhum gancho global, nada que um anticheat precise vigiar.
    let taxa = mouse::taxa_de_varredura(&intervalos_us);
    let caminho = mouse::desta_maquina(taxa);

    CaminhoNaTela {
        achados: caminho.achados_na_tela(),
        caminho,
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Serialize)]
pub struct CaminhoNaTela {
    pub caminho: crate::modules::mouse::Caminho,
    pub achados: Vec<crate::modules::mouse::AchadoNaTela>,
}

/// NÃO APLICA NADA: devolve um plano com o nome que `apply_game_profile` aceita, as razões medidas e o que não foi
/// verificado.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn plano_de_renderizacao(state: State<'_, AppState>) -> Result<PlanoNaTela, String> {
    use crate::modules::windows::{configjogo, discodojogo};
    use crate::modules::{orquestrador, streaming};

    let metricas = {
        let mut monitor = state.monitor.lock().await;
        monitor.collect_metrics().await?
    };

    let relatorio = tokio::task::spawn_blocking(discodojogo::analisar)
        .await
        .map_err(|e| format!("a leitura dos discos não terminou: {e}"))?;
    let config = tokio::task::spawn_blocking(configjogo::analyze)
        .await
        .map_err(|e| format!("a leitura da configuração do jogo não terminou: {e}"))?;

    let midia = discodojogo::em_disco_mecanico(&relatorio.jogos)
        .first()
        .map(|o| streaming::Midia::from(o.midia));

    let analise = streaming::analisar(&metricas.telemetry, midia);
    let plano = orquestrador::planejar(
        &metricas.gargalo,
        &metricas.vram,
        &analise,
        config.arquivo.is_some(),
    );

    Ok(PlanoNaTela {
        // Daqui e não da tela: uma segunda tabela de nomes sairia do lugar calada.
        perfil_para_aplicar: match plano.decisao {
            orquestrador::Decisao::Aplicar(p) => Some(p.nome().to_string()),
            _ => None,
        },
        plano,
        jogo: config.jogo,
    })
}

#[cfg(target_os = "windows")]
#[derive(Debug, Serialize)]
pub struct PlanoNaTela {
    pub plano: crate::modules::orquestrador::Plano,
    pub perfil_para_aplicar: Option<String>,
    pub jogo: String,
}

/// Comando, não campo da coleta: descobrir o disco de cada jogo custa sistema de arquivos e WMI, e não muda de
/// segundo em segundo. Entra o PRIMEIRO jogo em disco mecânico: é o único que muda a conclusão.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn laboratorio_de_streaming(
    state: State<'_, AppState>,
) -> Result<LaboratorioNaTela, String> {
    use crate::modules::streaming;
    use crate::modules::windows::discodojogo;

    let metricas = {
        let mut monitor = state.monitor.lock().await;
        monitor.collect_metrics().await?
    };

    let relatorio = tokio::task::spawn_blocking(discodojogo::analisar)
        .await
        .map_err(|e| format!("a leitura dos discos não terminou: {e}"))?;

    let em_mecanico = discodojogo::em_disco_mecanico(&relatorio.jogos);
    let jogo = em_mecanico.first().map(|o| (*o).clone());
    let midia = jogo.as_ref().map(|o| streaming::Midia::from(o.midia));

    Ok(LaboratorioNaTela {
        analise: streaming::analisar(&metricas.telemetry, midia),
        jogo,
        lacunas: relatorio.lacunas,
    })
}

/// O jogo vai junto: "o disco está lento" sem dizer qual jogo não se confere.
#[cfg(target_os = "windows")]
#[derive(Debug, Serialize)]
pub struct LaboratorioNaTela {
    pub analise: crate::modules::streaming::Analise,
    pub jogo: Option<crate::modules::windows::discodojogo::OndeMora>,
    pub lacunas: Vec<String>,
}

/// O perfil vem de fora: não é detectável, e rotular errado autorizaria comparação indevida (ver `baseline.rs`).
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn capturar_baseline(
    perfil: crate::modules::baseline::Perfil,
    state: State<'_, AppState>,
) -> Result<crate::modules::baseline::Baseline, String> {
    use crate::modules::baseline;

    let aplicadas = state.changes.lock().await.applied().len();

    let metricas = {
        let mut monitor = state.monitor.lock().await;
        monitor.collect_metrics().await?
    };

    let quando = crate::modules::changelog::now_timestamp();
    let retrato = baseline::Baseline::novo(
        quando,
        perfil,
        baseline::identidade_desta_maquina(aplicadas),
        metricas.telemetry,
    );

    baseline::guardar(retrato.clone())?;

    // O histórico DEPOIS do baseline, e falhar nele não derruba a captura: o retrato já está guardado.
    if let Err(erro) = anotar_no_historico(&retrato) {
        eprintln!("histórico de desempenho não foi atualizado: {erro}");
    }

    Ok(retrato)
}

/// Só `METRICAS_GUARDADAS`: as cinquenta encheriam o teto em nove capturas.
#[cfg(target_os = "windows")]
fn anotar_no_historico(retrato: &crate::modules::baseline::Baseline) -> Result<(), String> {
    use crate::modules::historico;

    let mut h = historico::ler()?;

    for resumo in &retrato.incerteza {
        if historico::METRICAS_GUARDADAS.contains(&resumo.id.as_str()) {
            h.anotar(
                retrato.quando,
                retrato.identidade.clone(),
                historico::Evento::Medicao(resumo.clone()),
            );
        }
    }

    historico::guardar(&h)
}

/// As mudanças vêm do `changelog` na hora: duas listas do mesmo fato discordam.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn historico_de_desempenho(
    state: State<'_, AppState>,
) -> Result<HistoricoNaTela, String> {
    use crate::modules::historico;

    let aplicadas = state.changes.lock().await.applied().to_vec();
    let h = historico::ler()?.com_mudancas(&aplicadas);

    // Sentido pela tabela, não palpite. Menos de duas medições não entra.
    let regressoes = historico::METRICAS_GUARDADAS
        .iter()
        .filter_map(|id| {
            let sentido = historico::maior_e_melhor(id)?;
            h.regressao(id, sentido)
        })
        .collect();

    Ok(HistoricoNaTela {
        historico: h,
        regressoes,
    })
}

#[cfg(target_os = "windows")]
#[derive(Debug, Serialize)]
pub struct HistoricoNaTela {
    pub historico: crate::modules::historico::Historico,
    pub regressoes: Vec<crate::modules::historico::Regressao>,
}

/// Chamado ANTES de `capturar_baseline_repetido`: benchmark que prende a máquina sem avisar é cancelado no meio.
#[tauri::command(async)]
pub fn protocolo_do_perfil(perfil: crate::modules::baseline::Perfil) -> ProtocoloNaTela {
    let p = crate::modules::repeticoes::protocolo(perfil);

    ProtocoloNaTela {
        // Contas aqui: `execucoes` inclui a descartada e `duracao` depende dela.
        execucoes: p.execucoes(),
        duracao_estimada_s: p.duracao_estimada_s(),
        protocolo: p,
    }
}

#[derive(Debug, Serialize)]
pub struct ProtocoloNaTela {
    pub protocolo: crate::modules::repeticoes::Protocolo,
    pub execucoes: usize,
    pub duracao_estimada_s: u64,
}

/// Várias coletas entregam o número E o quanto balança nesta máquina (ver `repeticoes.rs`). O protocolo sai do
/// perfil de carga, com a duração para a tela avisar antes.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn capturar_baseline_repetido(
    perfil: crate::modules::baseline::Perfil,
    state: State<'_, AppState>,
) -> Result<crate::modules::baseline::Baseline, String> {
    use crate::modules::{baseline, repeticoes};
    use std::collections::BTreeMap;

    let protocolo = repeticoes::protocolo(perfil);
    let aplicadas = state.changes.lock().await.applied().len();

    let mut series: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut ultima = None;

    for volta in 0..protocolo.execucoes() {
        let metricas = {
            let mut monitor = state.monitor.lock().await;
            monitor.collect_metrics().await?
        };

        // A primeira (cache frio) entra no relatório como descartada, em vez de sumir.
        let aquecimento = protocolo.descarta_primeira && volta == 0;

        if !aquecimento {
            for (id, m) in &metricas.telemetry.metrics {
                if let Some(v) = m.value {
                    series.entry(id.clone()).or_default().push(v);
                }
            }
        }

        ultima = Some(metricas);

        if volta + 1 < protocolo.execucoes() {
            tokio::time::sleep(std::time::Duration::from_secs(
                protocolo.segundos_por_repeticao,
            ))
            .await;
        }
    }

    let metricas = ultima.ok_or("nenhuma repetição foi executada")?;

    let incerteza: Vec<repeticoes::Resumo> = series
        .iter()
        .filter_map(|(id, amostras)| repeticoes::resumir(id, amostras))
        .collect();

    let retrato = baseline::Baseline::novo(
        crate::modules::changelog::now_timestamp(),
        perfil,
        baseline::identidade_desta_maquina(aplicadas),
        metricas.telemetry,
    )
    .com_incerteza(incerteza);

    baseline::guardar(retrato.clone())?;
    Ok(retrato)
}

/// `Err` explicado quando não dá para comparar: vale mais que uma tabela sem sentido.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn comparar_com_baseline(
    perfil: crate::modules::baseline::Perfil,
    state: State<'_, AppState>,
) -> Result<crate::modules::baseline::Comparacao, String> {
    use crate::modules::baseline;

    let antes = baseline::ler()?
        .into_iter()
        .find(|b| b.perfil == perfil)
        .ok_or_else(|| {
            format!(
                "Não há retrato guardado com {}. Guarde o antes primeiro.",
                perfil.nome()
            )
        })?;

    let aplicadas = state.changes.lock().await.applied().len();

    let metricas = {
        let mut monitor = state.monitor.lock().await;
        monitor.collect_metrics().await?
    };

    let agora = baseline::Baseline::novo(
        crate::modules::changelog::now_timestamp(),
        perfil,
        baseline::identidade_desta_maquina(aplicadas),
        metricas.telemetry,
    );

    baseline::comparar(&antes, &agora).map_err(|recusa| recusa.explicacao())
}

#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn start_monitoring(state: State<'_, AppState>) -> Result<String, String> {
    let mut monitor = state.monitor.lock().await;
    monitor.start_monitoring();
    Ok("Monitoring started".to_string())
}

#[tauri::command]
pub async fn stop_monitoring(state: State<'_, AppState>) -> Result<String, String> {
    let mut monitor = state.monitor.lock().await;
    monitor.stop_monitoring();
    Ok("Monitoring stopped".to_string())
}

// O benchmark ocupa a CPU por segundos: `spawn_blocking`, senão trava a interface e distorce a medição.

/// Em disco: o que exige reiniciar continua mensurável.
#[tauri::command]
pub async fn measure_baseline() -> Result<BaselineResult, String> {
    let snapshot = tokio::task::spawn_blocking(|| Benchmark::new().run())
        .await
        .map_err(|e| format!("Benchmark failed: {}", e))?;

    BaselineStore::save(&snapshot)?;
    Ok(BaselineResult::from(snapshot))
}

#[tauri::command(async)]
pub fn get_baseline() -> Option<BenchmarkSnapshot> {
    BaselineStore::load()
}

#[tauri::command]
pub async fn measure_and_compare() -> Result<BenchmarkComparison, String> {
    let before = BaselineStore::load()
        .ok_or("Nenhuma medição inicial encontrada. Meça o desempenho antes de otimizar.")?;

    let after = tokio::task::spawn_blocking(|| Benchmark::new().run())
        .await
        .map_err(|e| format!("Benchmark failed: {}", e))?;

    Ok(benchmark::compare(&before, &after))
}

/// Aponta o culpado pelo nome e se ele volta no próximo boot.
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

/// Só explica, não otimiza.
#[tauri::command]
pub async fn analyze_bottleneck(seconds: u64) -> Result<BottleneckReport, String> {
    #[cfg(target_os = "windows")]
    {
        // Bloqueia: sai do runtime. Entre 4 e 30 s.
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

/// Mede de fora, pelo canal de eventos do Windows.
#[tauri::command]
pub async fn measure_frames(process: String, seconds: u64) -> Result<FrameMeasurement, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            use crate::modules::windows::frames;

            let (pid, nome) = frames::encontrar_processo(&process).ok_or_else(|| {
                format!(
                    "Não encontrei nenhum processo com `{}` no nome. Abra o jogo antes de medir.",
                    process
                )
            })?;

            // Entre 3 e 30 s.
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

/// O classificador (`core::gargalo`) aponta os gargalos sustentados na janela, com a evidência.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagnosticoAoVivo {
    pub placa: Option<crate::core::telemetria::Placa>,
    pub jogo: Option<String>,
    pub amostras: Vec<crate::core::telemetria::Amostra>,
    pub saude: Option<crate::core::fluidez::SaudeDosQuadros>,
    pub quadros_erro: Option<String>,
    pub gargalo: crate::modules::gargalo::Diagnostico,
    pub travadas: Option<crate::core::travadas::Investigacao>,
    /// `None` sem placa NVIDIA.
    pub sensores_da_placa: Option<crate::core::sensores::ResumoGpu>,
}

#[tauri::command]
pub async fn diagnostico_ao_vivo(
    segundos: u64,
    state: State<'_, AppState>,
) -> Result<DiagnosticoAoVivo, String> {
    let segundos = segundos.clamp(5, 60);
    // Sem o piso o transbordo de VRAM não se distingue do normal.
    let piso = state.monitor.lock().await.piso_de_vram();
    tokio::task::spawn_blocking(move || diagnostico_na_janela(segundos, piso))
        .await
        .map_err(|e| format!("Falha no diagnóstico: {}", e))?
}

pub fn diagnostico_na_janela(
    segundos: u64,
    piso: crate::modules::vram::Piso,
) -> Result<DiagnosticoAoVivo, String> {
    {
        use crate::core::telemetria;

        let mut coletor = telemetria::Coletor::novo()
            .ok_or("Os contadores de desempenho do Windows não abriram nesta máquina.")?;
        let jogo = crate::modules::windows::gamemode::jogo_aberto_com_pid();
        coletor.acompanhar_processos(jogo.as_ref().map(|(_, pid)| *pid));

        // A âncora: o mesmo instante no relógio das amostras (ms do coletor) e no dos quadros (QPC).
        let ancora_ms = coletor.decorrido_ms();
        let ancora_qpc = crate::modules::windows::frames::agora_qpc();
        let frequencia = crate::modules::windows::frames::frequencia_qpc();
        // PresentMon, nunca o canal antigo: no FiveM ele conta o dobro dos quadros (medido em 28/09/2026).
        let medicao = jogo.clone().map(|(nome, pid)| {
            std::thread::spawn(move || -> Result<(crate::modules::windows::frames::MedicaoCrua, Option<i64>), String> {
                let crus = crate::modules::windows::presentmon::capturar(pid, segundos as u32)?;
                let q = crate::modules::windows::investigacao::quadros_do_jogo(&crus);
                let (Some(primeiro), Some(ultimo), Some(f)) = (q.first(), q.last(), frequencia) else {
                    return Err("o PresentMon não viu nenhum quadro do jogo".to_string());
                };
                let decorrido = (ultimo.qpc - primeiro.qpc) as f64 / f as f64;
                let intervalos: Vec<f64> = q.iter().map(|x| x.intervalo_ms).collect();
                // O relógio da V1 soma intervalos a partir do início: o início é o primeiro quadro menos o intervalo dele.
                let inicio_qpc = primeiro.qpc - (primeiro.intervalo_ms / 1000.0 * f as f64) as i64;
                Ok((crate::modules::windows::frames::montar(&nome, pid, q.len() as u64, decorrido, intervalos, Vec::new()), Some(inicio_qpc)))
            })
        });

        let mut amostras = Vec::new();
        // Na MESMA janela: temperatura lida depois é de placa fria. O motivo do clock custa 11 ms: a cada duas voltas.
        let limite_w = crate::modules::windows::nvml::limite_de_potencia_w();
        let mut sensores: Vec<crate::core::sensores::AmostraGpu> = Vec::new();
        let mut volta: u32 = 0;
        let fim = std::time::Instant::now() + std::time::Duration::from_secs(segundos);
        while std::time::Instant::now() < fim {
            std::thread::sleep(std::time::Duration::from_millis(500));
            amostras.push(coletor.amostra());
            volta += 1;
            if let Some(a) = crate::modules::windows::nvml::amostrar_com_motivos(volta % 2 == 1) {
                sensores.push(a);
            }
        }

        let (saude, intervalos, quadros_erro, inicio_qpc) = match medicao.map(|h| h.join()) {
            None => (None, Vec::new(), None, None),
            Some(Ok(Ok((m, inicio)))) => (m.resumo.saude, m.intervalos_ms, None, inicio),
            Some(Ok(Err(e))) => (None, Vec::new(), Some(e), None),
            Some(Err(_)) => (
                None,
                Vec::new(),
                Some("A medição de quadros parou no meio.".to_string()),
                None,
            ),
        };
        let inicio_dos_quadros = match (inicio_qpc, ancora_qpc, frequencia) {
            (Some(i), Some(a), Some(f)) if f > 0 => {
                (ancora_ms as i64 + (i - a) * 1000 / f).max(0) as u64
            }
            _ => ancora_ms,
        };
        let vram_total = coletor.placa().map(|p| p.vram_total_mb);
        let travadas = (!intervalos.is_empty()).then(|| {
            crate::core::travadas::investigar(
                &intervalos,
                &amostras,
                inicio_dos_quadros,
                vram_total,
            )
        });

        let ram_total_mb = {
            let mut s = sysinfo::System::new();
            s.refresh_memory();
            Some(s.total_memory() as f64 / 1_048_576.0)
        };
        let hz = crate::modules::windows::display::monitores()
            .iter()
            .find(|m| m.principal)
            .map(|m| m.hz_atual);
        let agora = crate::modules::changelog::now_timestamp();
        let t = crate::core::janela::para_telemetria(
            &amostras,
            ram_total_mb,
            vram_total,
            saude.as_ref(),
            travadas.as_ref(),
            hz,
            agora,
        );
        let gargalo =
            crate::modules::gargalo::classificar_com(&t, &crate::modules::vram::avaliar(&t, &piso));

        Ok(DiagnosticoAoVivo {
            placa: coletor.placa().cloned(),
            jogo: jogo.map(|(n, _)| n),
            amostras,
            saude,
            quadros_erro,
            gargalo,
            travadas,
            sensores_da_placa: crate::core::sensores::resumir(&sensores, limite_w),
        })
    }
}
