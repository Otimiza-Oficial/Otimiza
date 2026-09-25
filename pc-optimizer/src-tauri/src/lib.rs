mod ci_coverage;

mod core;
mod modules;
mod utils;
mod commands;

use commands::AppState;
use modules::changelog::ChangeLog;
use modules::PerformanceMonitor;
use tauri::Manager;
use tokio::sync::Mutex;

/// Frações, não pixels: nenhum número fixo serve para 1366x768 e para 4K.
const FRACAO_DA_LARGURA: f64 = 0.66;
const FRACAO_DA_ALTURA: f64 = 0.75;

/// Igual ao `minWidth`/`minHeight` do tauri.conf.json: abaixo disso a tabela rola na horizontal.
const LARGURA_MINIMA: f64 = 900.0;
const ALTURA_MINIMA: f64 = 640.0;

/// O mínimo entra DEPOIS da fração e ANTES do teto da tela: numa tela menor que o mínimo, a janela é a tela.
fn tamanho_que_cabe(largura_util: f64, altura_util: f64) -> (f64, f64) {
    let largura = (largura_util * FRACAO_DA_LARGURA)
        .max(LARGURA_MINIMA)
        .min(largura_util);

    let altura = (altura_util * FRACAO_DA_ALTURA)
        .max(ALTURA_MINIMA)
        .min(altura_util);

    (largura, altura)
}

/// Falhar aqui não derruba a abertura: vale o tamanho conservador do tauri.conf.json.
fn ajustar_a_janela_a_tela(janela: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = janela.current_monitor() else {
        return;
    };

    let escala = monitor.scale_factor();
    let fisico = monitor.size();

    // Pixels lógicos: a única unidade que faz sentido com escala de 125% ou 150%.
    let largura_util = fisico.width as f64 / escala;
    let altura_util = fisico.height as f64 / escala;

    let (largura, altura) = tamanho_que_cabe(largura_util, altura_util);

    let _ = janela.set_size(tauri::LogicalSize::new(largura, altura));
    let _ = janela.center();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    modules::abertura::marcar_inicio();
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
            #[cfg(target_os = "windows")]
            commands::capturar_baseline,
            #[cfg(target_os = "windows")]
            commands::capturar_baseline_repetido,
            commands::protocolo_do_perfil,
            #[cfg(target_os = "windows")]
            commands::laboratorio_de_streaming,
            #[cfg(target_os = "windows")]
            commands::plano_de_renderizacao,
            commands::passo_do_autoajuste,
            #[cfg(target_os = "windows")]
            commands::historico_de_desempenho,
            #[cfg(target_os = "windows")]
            commands::caminho_do_mouse,
            #[cfg(target_os = "windows")]
            commands::catalogo_de_programas,
            #[cfg(target_os = "windows")]
            commands::medir_limpeza,
            #[cfg(target_os = "windows")]
            commands::nucleos_da_maquina,
            #[cfg(target_os = "windows")]
            commands::prender_jogo_nos_nucleos,
            #[cfg(target_os = "windows")]
            commands::limpar_alvos,
            #[cfg(target_os = "windows")]
            commands::instalar_programa,
            #[cfg(target_os = "windows")]
            commands::comparar_com_baseline,
            commands::recuperacao_pendente,
            #[cfg(target_os = "windows")]
            commands::concluir_recuperacao,
            commands::descartar_pendencia,
            commands::start_monitoring,
            commands::stop_monitoring,
            commands::measure_baseline,
            commands::get_baseline,
            commands::measure_and_compare,
            commands::is_elevated,
            commands::diagnostico_de_energia,
            commands::simular_plano_otimiza,
            commands::relatorio_de_compatibilidade,
            commands::vistoriar_plano_otimiza,
            commands::reparar_plano_otimiza,
            commands::aplicar_plano_otimiza,
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
            commands::zerar_modo_jogo,
            commands::analyze_network,
            commands::set_dns,
            commands::flush_dns,
            commands::medir_perda_de_pacote,
            commands::measure_frames,
            commands::framegen_detectar,
            commands::framegen_medir,
            commands::framegen_comparar,
            commands::framegen_melhor,
            commands::gerador_ligar,
            commands::gerador_desligar,
            commands::gerador_estado,
            commands::energia_painel,
            commands::energia_vizinhos,
            commands::diagnostico_ao_vivo,
            commands::tetos_escondidos,
            commands::pronto_para_jogar,
            commands::energia_medir_atual,
            commands::energia_escolher,
            commands::energia_restaurar_anterior,
            commands::energia_restaurar_windows,
            commands::energia_salvar_perfil_de_jogo,
            commands::energia_remover_perfil_de_jogo,
            commands::energia_testar_candidato,
            commands::energia_aplicar,
            commands::energia_modo_dinamico,
            commands::analyze_fivem,
            commands::clean_fivem,
            commands::analyze_citizenfx,
            commands::analyze_browsers,
            commands::clean_browser_cache,
            commands::analyze_boot,
            commands::analyze_thermal,
            commands::export_report,
            commands::exportar_alteracoes,
            commands::msi_dispositivos,
            commands::diagnostico_dpc,
            commands::cpuset_testar,
            commands::cpuset_resultados,
            commands::cpuset_esquecer,
            commands::map_folders,
            commands::analyze_rbar,
            commands::list_profiles,
            commands::list_third_party_services,
            commands::set_service_start,
            commands::list_scheduled_tasks,
            commands::set_scheduled_task,
            commands::scan_disk_space,
            commands::clean_disk_category,
            commands::analyze_memory,
            commands::set_automatic_pagefile,
            commands::restore_status,
            commands::create_restore_point,
            commands::enable_system_protection,
            commands::list_startup,
            commands::set_startup_enabled,
            commands::list_optimizations,
            commands::estado_do_historico,
            commands::convite_do_discord,
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
            commands::checar_essenciais,
            commands::religar_essenciais,
            commands::ajustes_do_driver_nvidia,
            commands::aplicar_ajuste_nvidia,
            commands::limitar_fps_nvidia,
            commands::medicoes_automaticas,
            commands::conferir_o_proprio_trabalho,
            commands::onde_os_jogos_moram,
            commands::por_que_o_fps_esta_baixo,
            commands::o_que_nao_fazemos,
            commands::o_que_o_otimiza_altera,
            commands::protocolo_de_grupos,
            commands::nota_do_jogo,
            commands::conflitos_entre_ajustes,
            commands::conta_que_esta_rodando,
            commands::niveis_de_otimizacao,
            commands::passo_a_passo_da_bios,
            commands::ficha_da_bios,
            commands::abertura_pronta,
            commands::quedas_de_desempenho,
            commands::reiniciar_na_bios,
            commands::set_max_refresh_rate,
            commands::licenca_estado,
            commands::licenca_ativar,
            commands::reparo_disponivel,
            commands::reparo_ultimo_resultado,
            commands::reparo_executar,
            commands::reparo_cancelar,
            commands::relatorio_de_suporte,
            commands::versao_mais_nova,
        ])
        .setup(|app| {
            // `otimiza.log` começa dizendo em que máquina: é o arquivo que o cliente manda. Em thread própria (PowerShell e
            // `powercfg`), fora do orçamento da abertura.
            #[cfg(target_os = "windows")]
            modules::windows::cabecalho::anotar_em_segundo_plano();

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            // A janela fixa em 1440x900 nascia maior que a tela num notebook de 1366x768. Mede a área ÚTIL (sem a barra
            // de tarefas) e toma dois terços da largura e três quartos da altura.
            if let Some(janela) = app.get_webview_window("main") {
                ajustar_a_janela_a_tela(&janela);
            }

            utils::Logger::info("PC Performance Optimizer iniciado");

            // Rede da suspensão (versões antigas): fim de sessão. Thread suspensa não responde ao fim de sessão, a colmeia
            // do usuário não descarrega e o Explorer não abre na sessão seguinte (ver `modules::windows::sessao`).
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

            // Rede da suspensão, antes de tudo: o congelamento saiu na 2.0, mas quem atualiza pode chegar com programas
            // registrados como suspensos. Com o registro vazio, não faz nada.
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

            // Lê a preferência a cada volta, para ligar e desligar valer na hora. Seis segundos: mais rápido gasta CPU do
            // próprio otimizador; mais devagar o jogo já carregou.
            #[cfg(target_os = "windows")]
            {
                let handle = app.handle().clone();

                tauri::async_runtime::spawn(async move {
                    use tauri::{Emitter, Manager};

                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(6)).await;

                        // Sempre, independente do modo jogo: leitura em memória, sub-milissegundo. Nunca PowerShell aqui.
                        modules::windows::pressao::amostrar();

                        // Rede da suspensão por prazo, ANTES da preferência: o modo pode ter sido desligado com algo ainda suspenso.
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

                            // Só com mudança: emitir a cada seis segundos recarregaria a tela à toa.
                            let _ = handle.emit("gamemode:changed", mensagem);
                        }

                        if !modules::preferences::Preferences::load().auto_game_mode {
                            continue;
                        }

                        let estado = handle.state::<commands::AppState>();
                        let mut log = estado.changes.lock().await;

                        if let Some(mensagem) =
                            modules::windows::gamemode::passo(&mut log)
                        {
                            utils::Logger::info(&format!("Modo jogo: {}", mensagem));
                            let _ = handle.emit("gamemode:changed", mensagem);
                        }
                    }
                });
            }

            // O Otimiza fechou com jogo aberto e programas em modo econômico: devolve antes de tudo.
            #[cfg(windows)]
            tauri::async_runtime::spawn(async {
                match tokio::task::spawn_blocking(modules::windows::gamemode::recuperar_na_abertura).await {
                    Ok(Ok(d)) => {
                        if d.devolvidos > 0 {
                            utils::Logger::info(&format!("governador: {} programa(s) devolvidos ao normal na abertura", d.devolvidos));
                        }
                        if !d.falharam.is_empty() {
                            let nomes: Vec<&str> = d.falharam.iter().map(|a| a.nome.as_str()).collect();
                            utils::Logger::warn(&format!(
                                "governador: na abertura o Windows não deixou devolver {}; seguem anotados",
                                nomes.join(", ")
                            ));
                        }
                    }
                    Ok(Err(e)) => utils::Logger::warn(&format!("governador: não consegui ler o que ficou acalmado: {}", e)),
                    Err(e) => utils::Logger::warn(&format!("governador: a devolução na abertura caiu: {}", e)),
                }
            });

            #[cfg(target_os = "windows")]
            tauri::async_runtime::spawn(async {
                if let Ok(Some(r)) = tokio::task::spawn_blocking(modules::windows::motorenergia_maquina::recuperar_teste_interrompido).await {
                    utils::Logger::info(&format!("motor de energia: teste interrompido desfeito na abertura: {:?}", r.map(|x| x.plano_ativo)));
                }
            });

            // Olha a cada três segundos só os executáveis com perfil salvo; só age com o modo ligado e elevado.
            #[cfg(target_os = "windows")]
            {
                let handle = app.handle().clone();

                tauri::async_runtime::spawn(async move {
                    use tauri::Emitter;
                    let mut vigia = modules::windows::motorenergia_maquina::Vigia::default();

                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        let (volta, evento) = tokio::task::spawn_blocking(move || {
                            let evento = modules::windows::motorenergia_maquina::olhar(&mut vigia);
                            (vigia, evento)
                        })
                        .await
                        .unwrap_or_default();
                        vigia = volta;

                        if let Some(evento) = evento {
                            utils::Logger::info(&format!("motor de energia, modo dinâmico: {:?}", evento));
                            let _ = handle.emit("energia:dinamico", evento);
                        }
                    }
                });
            }

            // O espaço livre do disco custa segundos na primeira leitura: aquecido aqui, a listagem já o encontra.
            #[cfg(target_os = "windows")]
            tauri::async_runtime::spawn(async {
                let _ = tokio::task::spawn_blocking(modules::windows::aquecer_condicoes).await;
            });

            // Afinidade morre com o processo: reaplica a cada abertura do jogo, só onde foi MEDIDO e rendeu.
            #[cfg(target_os = "windows")]
            tauri::async_runtime::spawn(async {
                let mut ja = std::collections::HashSet::new();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    let (volta, aplicados) = tokio::task::spawn_blocking(move || {
                        let a = modules::windows::cpuset::reaplicar(&mut ja);
                        (ja, a)
                    })
                    .await
                    .unwrap_or_default();
                    ja = volta;
                    for jogo in aplicados {
                        utils::Logger::info(&format!("auto cpu set: {jogo} nos núcleos de desempenho"));
                    }
                }
            });

            // Vigia à parte: detectar pelo PowerShell e escutar vinte segundos não podem atrasar a volta de seis segundos.
            // Ver `modules::medicoes`.
            #[cfg(target_os = "windows")]
            {
                let handle = app.handle().clone();

                tauri::async_runtime::spawn(async move {
                    use modules::medicoes::{self, Acompanhamento, MedicaoAutomatica};
                    use tauri::{Emitter, Manager};

                    let mut acompanhamento = Acompanhamento::default();

                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(
                            medicoes::SEGUNDOS_ENTRE_OLHADAS,
                        ))
                        .await;

                        if !modules::preferences::Preferences::load().medir_quadros_sozinho
                            || !modules::windows::registry::is_elevated()
                        {
                            acompanhamento = Acompanhamento::default();
                            continue;
                        }

                        let jogo = tokio::task::spawn_blocking(modules::windows::deteccao::procurar)
                            .await
                            .ok()
                            .flatten();
                        let agora = modules::changelog::now_timestamp();

                        if !acompanhamento.observar(jogo.as_ref().map(|j| j.pid), agora) {
                            continue;
                        }
                        acompanhamento.tentado(agora);

                        let Some(jogo) = jogo else { continue };

                        let mudancas_aplicadas = handle
                            .state::<commands::AppState>()
                            .changes
                            .lock()
                            .await
                            .applied()
                            .len();

                        let executavel = jogo.executavel.clone();

                        // Em paralelo, na mesma janela: medir depois compararia dois momentos da partida.
                        let parar = std::sync::Arc::new(
                            std::sync::atomic::AtomicBool::new(false),
                        );
                        let parar_amostrador = parar.clone();
                        let amostrador = std::thread::spawn(move || {
                            modules::windows::motorenergia_maquina::amostrar_enquanto(
                                parar_amostrador,
                                std::time::Duration::from_millis(500),
                            )
                        });

                        // Por contador: o WMI passa de um segundo e abriria PowerShell dentro da medição.
                        let parar_placa = parar.clone();
                        let placa = std::thread::spawn(move || {
                            use std::sync::atomic::Ordering;
                            let contadores = modules::windows::placa::Contadores::novo()?;
                            let mut gpu: Vec<f64> = Vec::new();
                            // Pela NVML em processo: o nvidia-smi a cada 200 ms viraria a carga que se mede.
                            let limite_w = modules::windows::nvml::limite_de_potencia_w();
                            let mut sensores: Vec<crate::core::sensores::AmostraGpu> = Vec::new();
                            // O motivo custa 11 ms e atrasaria o carimbo do disco: a cada cinco voltas (um segundo).
                            let mut volta: u32 = 0;
                            // Carimbada no mesmo relógio dos quadros, para cruzar com cada tranco.
                            let mut disco: Vec<(i64, f64)> = Vec::new();

                            // 200 ms: a janela de correlação é de 300 ms para cada lado.
                            while !parar_placa.load(Ordering::Relaxed) {
                                std::thread::sleep(std::time::Duration::from_millis(200));
                                let a = contadores.coletar();

                                if let Some(pct) = a.gpu_pct {
                                    gpu.push(pct);
                                }
                                volta += 1;
                                if let Some(amostra) =
                                    modules::windows::nvml::amostrar_com_motivos(volta % 5 == 1)
                                {
                                    sensores.push(amostra);
                                }
                                if let (Some(quando), Some(pct)) = (
                                    modules::windows::frames::agora_qpc(),
                                    a.disco_ocupado_pct,
                                ) {
                                    disco.push((quando, pct));
                                }
                            }

                            // Lista vazia viraria zero, "placa parada".
                            let media = (!gpu.is_empty())
                                .then(|| gpu.iter().sum::<f64>() / gpu.len() as f64);

                            Some((media, disco, crate::core::sensores::resumir(&sensores, limite_w)))
                        });

                        // O estado do governador precisa ser o mesmo no começo e no fim (`modules::portao`).
                        let governador_no_inicio =
                            modules::windows::gamemode::governador_na_partida(&executavel);

                        let medido = tokio::task::spawn_blocking(move || {
                            modules::windows::frames::medir_par(
                                jogo.pid,
                                &jogo.executavel,
                                None,
                                medicoes::SEGUNDOS_DE_MEDICAO,
                            )
                        })
                        .await;

                        parar.store(true, std::sync::atomic::Ordering::Relaxed);
                        let governador_na_medicao = modules::portao::marcar(
                            governador_no_inicio,
                            modules::windows::gamemode::governador_na_partida(&executavel),
                        );
                        let cpu_uso_pct = amostrador
                            .join()
                            .ok()
                            .and_then(|amostras| {
                                modules::windows::motorenergia::resumir_cpu(&amostras)
                            })
                            .and_then(|resumo| resumo.uso_medio_pct);
                        let (gpu_uso_pct, disco_da_janela, sensores_da_placa) = match placa.join().ok().flatten() {
                            Some((media, disco, sensores)) => (media, disco, sensores),
                            None => (None, Vec::new(), None),
                        };

                        match medido.map(|r| r.map(|(principal, _)| principal)) {
                            Ok(Ok(crua)) => {
                                let m = crua.resumo;
                                let (medio, p95, p99) =
                                    match modules::windows::frames::percentis(&crua.intervalos_ms) {
                                        Some((a, b, c)) => (Some(a), Some(b), Some(c)),
                                        None => (None, None, None),
                                    };

                                // 300 ms para cada lado; 40% é quando o disco deixa de estar de passagem e passa a trabalhar.
                                let correlacao = modules::windows::frames::frequencia_qpc()
                                    .and_then(|hz| {
                                        modules::windows::frames::trancos_com_disco(
                                            &crua.trancos_qpc,
                                            &disco_da_janela,
                                            hz,
                                            300.0,
                                            40.0,
                                        )
                                    });

                                let trancos_com_disco_pct = correlacao
                                    .map(|(com, total)| com as f64 / total as f64 * 100.0);

                                let registro = MedicaoAutomatica {
                                    jogo: m.process,
                                    quando: agora,
                                    fps: m.fps,
                                    low_1pct: m.low_1pct,
                                    engasgos_por_minuto: m.engasgos_por_minuto,
                                    segundos: m.seconds,
                                    placa: sensores_da_placa,
                                    confiavel: m.detalhe_confiavel,
                                    mudancas_aplicadas,
                                    ambiente: Some(modules::deriva::Ambiente {
                                        driver: core::telemetria::versao_do_driver(),
                                        windows: core::telemetria::build_do_windows(),
                                    }),
                                    frametime_medio_ms: medio,
                                    frametime_p95_ms: p95,
                                    frametime_p99_ms: p99,
                                    cpu_uso_pct,
                                    gpu_uso_pct,
                                    trancos_com_disco_pct,
                                    trancos_medidos: correlacao.map(|(_, total)| total),
                                    governador: governador_na_medicao,
                                };

                                match medicoes::registrar(registro) {
                                    Ok(()) => {
                                        utils::Logger::info(&format!(
                                            "medição automática de {}: {:.0} FPS, 1% piores {:.0}",
                                            executavel, m.fps, m.low_1pct
                                        ));
                                        let _ = handle.emit("prova:automatica", ());

                                        let decididos = {
                                            let estado = handle.state::<commands::AppState>();
                                            let mut log = estado.changes.lock().await;
                                            modules::windows::decidir_portao(&mut log)
                                        };
                                        for d in decididos {
                                            utils::Logger::info(&format!("portão: {} → {:?}", d.vigiado.nome, d.veredito));
                                            if d.vigiado.id.starts_with("governador:") {
                                                let _ = handle.emit("gamemode:changed", modules::portao::frase_do_governador(&d));
                                            }
                                            let _ = handle.emit("portao:decidido", d);
                                        }
                                    }
                                    Err(erro) => utils::Logger::warn(&format!(
                                        "medição automática de {} feita, mas não gravada: {}",
                                        executavel, erro
                                    )),
                                }
                            }
                            Ok(Err(motivo)) => utils::Logger::info(&format!(
                                "medição automática de {} não aconteceu: {}",
                                executavel, motivo
                            )),
                            Err(erro) => utils::Logger::warn(&format!(
                                "medição automática de {} caiu: {}",
                                executavel, erro
                            )),
                        }
                    }
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            // Rede da suspensão ao fechar. `Exit` é redundante com `ExitRequested` de propósito: retomar o que já roda não
            // custa nada.
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

#[cfg(test)]
mod tests_da_janela {
    use super::*;

    /// O notebook de 1366x768 em que a janela fixa nascia maior que a tela.
    #[test]
    fn a_janela_nunca_nasce_maior_que_a_tela() {
        for (largura_tela, altura_tela) in [
            (1366.0, 768.0),
            (1280.0, 720.0),
            (1920.0, 1080.0),
            (2560.0, 1440.0),
            (3840.0, 2160.0),
            (1024.0, 600.0),
        ] {
            let (largura, altura) = tamanho_que_cabe(largura_tela, altura_tela);

            assert!(
                largura <= largura_tela,
                "{}x{}: a janela saiu com {} de largura e nao cabe",
                largura_tela, altura_tela, largura
            );

            assert!(
                altura <= altura_tela,
                "{}x{}: a janela saiu com {} de altura e nao cabe",
                largura_tela, altura_tela, altura
            );
        }
    }

    #[test]
    fn em_monitor_grande_sobra_area_de_trabalho_atras() {
        let (largura, altura) = tamanho_que_cabe(1920.0, 1080.0);

        assert!(
            largura < 1920.0 * 0.8,
            "em 1920 de largura a janela ficou com {}, que ainda e quase tudo",
            largura
        );

        assert!(altura < 1080.0 * 0.85, "altura de {} ainda e quase tudo", altura);
    }

    #[test]
    fn em_tela_normal_a_janela_respeita_o_minimo_util() {
        let (largura, altura) = tamanho_que_cabe(1920.0, 1080.0);

        assert!(largura >= LARGURA_MINIMA);
        assert!(altura >= ALTURA_MINIMA);
    }

    /// Numa tela menor que o mínimo, a resposta é a tela inteira.
    #[test]
    fn tela_menor_que_o_minimo_devolve_a_propria_tela() {
        let (largura, altura) = tamanho_que_cabe(800.0, 600.0);

        assert_eq!(largura, 800.0);
        assert_eq!(altura, 600.0);
    }
}
