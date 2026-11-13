// Prova e diagnóstico: laboratório, medir antes/depois, prova guardada, medições automáticas, protocolo de grupos
// e os relatórios da partida. É o que sustenta o "melhorou" com número, e não com promessa.

use super::*;

/// Roda sozinho ao abrir: só o barato, e UMA frase com o número que a sustenta.
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

/// "O que mudou?" por jogo (`modules::linhadotempo`): partidas, trocas em volta da máquina e mudanças do Otimiza.
/// Só leitura.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn o_que_mudou(
    state: State<'_, AppState>,
) -> Result<Vec<crate::modules::linhadotempo::LinhaDoJogo>, String> {
    let aplicadas: Vec<(u64, String)> = state
        .changes
        .lock()
        .await
        .applied()
        .iter()
        .map(|a| (a.timestamp, a.name.clone()))
        .collect();
    tokio::task::spawn_blocking(move || {
        crate::modules::medicoes::ler().map(|m| crate::modules::linhadotempo::todos(&m, &aplicadas))
    })
    .await
    .map_err(|e| format!("Falha ao montar a linha do tempo: {}", e))?
}

/// "Por que meu jogo roda assim?" (`modules::doctor`): a última partida medida e os limites de FPS escondidos.
/// Só leitura, `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn diagnostico_da_partida() -> Result<crate::modules::doctor::Diagnostico, String> {
    tokio::task::spawn_blocking(|| crate::modules::medicoes::ler().map(|m| diagnosticar_agora(&m)))
        .await
        .map_err(|e| format!("Falha ao diagnosticar a partida: {}", e))?
}

/// O diagnóstico da última partida, com os limites de FPS em cache.
fn diagnosticar_agora(
    medicoes: &[crate::modules::medicoes::MedicaoAutomatica],
) -> crate::modules::doctor::Diagnostico {
    {
        let ultima: Vec<crate::modules::medicoes::MedicaoAutomatica> = medicoes
            .iter()
            .max_by_key(|m| m.quando)
            .map(|m| m.jogo.clone())
            .and_then(|jogo| {
                crate::modules::relatoriodapartida::partidas(&jogo, medicoes)
                    .pop()
                    .map(|p| p.into_iter().cloned().collect())
            })
            .unwrap_or_default();
        #[cfg(target_os = "windows")]
        let (tetos, monitor_hz) = {
            // Em cache por dez minutos: `procurar` varre jogos, lê a NVAPI e pergunta ao WMI (PowerShell) pelos
            // monitores, e o diagnóstico roda a cada medição, com o jogo aberto. Limite de FPS muda só quando alguém mexe.
            static GUARDADO: std::sync::Mutex<
                Option<(
                    std::time::Instant,
                    crate::modules::windows::tetos::Relatorio,
                )>,
            > = std::sync::Mutex::new(None);
            let mut g = GUARDADO.lock().unwrap_or_else(|e| e.into_inner());
            let valido = g
                .as_ref()
                .is_some_and(|(q, _)| q.elapsed() < std::time::Duration::from_secs(600));
            if !valido {
                *g = Some((
                    std::time::Instant::now(),
                    crate::modules::windows::tetos::procurar(),
                ));
            }
            let r = &g.as_ref().expect("acabou de ser preenchido").1;
            (Some(r.tetos.clone()), r.monitor_hz)
        };
        #[cfg(not(target_os = "windows"))]
        let (tetos, monitor_hz): (
            Option<Vec<crate::modules::windows::tetos::Teto>>,
            Option<u32>,
        ) = (None, None);
        crate::modules::doctor::diagnosticar(&ultima, tetos.as_deref(), monitor_hz)
    }
}

/// A receita (`modules::receita`): o ajuste de cada causa, se já está aplicado, como se prova, e o que já piorou
/// este jogo nesta máquina. Só leitura, `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn receita_da_partida(
    state: State<'_, AppState>,
) -> Result<crate::modules::receita::Receita, String> {
    use crate::modules::changelog::ChangeRecord;
    use crate::modules::receita::{chave_do_jogo, Aplicados, PerfilDoJogo};
    let (plano, perfis) = {
        let log = state.changes.lock().await;
        let perfis: Vec<(PerfilDoJogo, String)> = log
            .applied()
            .iter()
            .filter_map(|a| {
                let perfil = PerfilDoJogo::do_id(&a.optimization_id)?;
                let jogo = a.changes.iter().find_map(|m| match m {
                    ChangeRecord::GameConfig { jogo, .. } => Some(chave_do_jogo(jogo)),
                    _ => None,
                })?;
                Some((perfil, jogo))
            })
            .collect();
        (log.is_applied("plano_otimiza"), perfis)
    };
    tokio::task::spawn_blocking(move || {
        let medicoes = crate::modules::medicoes::ler()?;
        let diagnostico = diagnosticar_agora(&medicoes);
        // Estrito: registro ilegível NÃO vira "nada foi provado", que ofereceria de novo o que já piorou.
        let (memoria, memoria_ilegivel) = match crate::modules::portao::ler_estrito() {
            Ok(estado) => (
                crate::modules::receita::lembrancas(
                    &estado,
                    crate::modules::provaalternada::guardada().as_ref(),
                    &medicoes,
                ),
                None,
            ),
            Err(e) => (Vec::new(), Some(e)),
        };
        #[cfg(target_os = "windows")]
        let agora = Some(crate::modules::deriva::Ambiente::agora());
        #[cfg(not(target_os = "windows"))]
        let agora = None;
        let aplicados = Aplicados {
            plano,
            modo_jogo: crate::modules::preferences::Preferences::load().auto_game_mode,
            perfis,
        };
        Ok(crate::modules::receita::montar(
            diagnostico,
            memoria,
            memoria_ilegivel,
            &aplicados,
            &agora,
        ))
    })
    .await
    .map_err(|e| format!("Falha ao montar a receita: {}", e))?
}

/// O texto da última prova no jogo para colar no Discord (`modules::compartilhar`). Só leitura, `LIVRES`; a recusa
/// volta como erro, já com a frase para a tela.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn resultado_para_compartilhar() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        let prova = crate::modules::provaalternada::guardada();
        let data = prova
            .as_ref()
            .and_then(|p| chrono::DateTime::from_timestamp(p.quando as i64, 0))
            .map(|d| {
                d.with_timezone(&chrono::Local)
                    .format("%d/%m/%Y")
                    .to_string()
            })
            .unwrap_or_default();
        #[cfg(target_os = "windows")]
        let (cpu, gpu) = (
            crate::modules::windows::registry::read_text(
                "HKLM",
                r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                "ProcessorNameString",
            )
            .ok()
            .flatten(),
            crate::core::telemetria::placa_principal().map(|(nome, _)| nome),
        );
        #[cfg(not(target_os = "windows"))]
        let (cpu, gpu): (Option<String>, Option<String>) = (None, None);
        crate::modules::compartilhar::texto(prova.as_ref(), (cpu.as_deref(), gpu.as_deref()), &data)
            .map_err(|r| r.frase().to_string())
    })
    .await
    .map_err(|e| format!("Falha ao montar o texto: {}", e))?
}

/// O perfil do PC para a primeira abertura (`modules::perfil`): o que esta máquina pode esperar, sem porcentagem.
/// Só leitura, `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn perfil_do_pc() -> Result<crate::modules::perfil::Perfil, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(|| {
            crate::modules::perfil::montar(&crate::modules::perfil::ler())
        })
        .await
        .map_err(|e| format!("Falha ao ler o perfil do PC: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// A última partida inteira (`modules::relatoriodapartida`), das janelas já medidas. Só leitura.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn relatorio_da_ultima_partida(
) -> Result<Option<crate::modules::relatoriodapartida::Relatorio>, String> {
    tokio::task::spawn_blocking(|| {
        crate::modules::medicoes::ler().map(|m| {
            crate::modules::relatoriodapartida::ultima(&m).map(|mut r| {
                r.rede = crate::plataforma::rede_partida::da_partida(
                    &crate::plataforma::rede_partida::ler(),
                    &r.jogo,
                    r.inicio,
                    r.fim,
                );
                r
            })
        })
    })
    .await
    .map_err(|e| format!("Falha ao montar o relatório da partida: {}", e))?
}

#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn quedas_de_desempenho() -> Result<Vec<crate::modules::deriva::Deriva>, String> {
    tokio::task::spawn_blocking(|| {
        crate::modules::medicoes::ler_para_comparar().map(|m| crate::modules::deriva::procurar(&m))
    })
    .await
    .map_err(|e| format!("Falha ao ler as medições: {}", e))?
}

#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn pronto_para_jogar() -> Result<crate::modules::windows::prontojogo::Prontidao, String> {
    tokio::task::spawn_blocking(crate::modules::windows::prontojogo::verificar)
        .await
        .map_err(|e| format!("Falha na verificação: {}", e))
}

/// O teste do Lab em andamento e a comparação dele. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn lab_estado() -> Result<crate::plataforma::lab::Resultado, String> {
    Ok(crate::plataforma::lab::comparar(
        &crate::plataforma::lab::atual(),
    ))
}

/// Uma rodada do Lab. `LIVRES`: medir não muda o computador.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn lab_medir(
    processo: String,
    lado: crate::plataforma::lab::Lado,
    segundos: u32,
) -> Result<crate::plataforma::lab::Resultado, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            crate::plataforma::lab::medir_rodada(&processo, lado, segundos)
                .map(|t| crate::plataforma::lab::comparar(&t))
        })
        .await
        .map_err(|e| format!("Falha ao medir: {}", e))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (processo, lado, segundos);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Recomeça o teste do Lab. `LIVRES`: apaga só as rodadas do Lab.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn lab_zerar() -> Result<(), String> {
    crate::plataforma::lab::zerar()
}

/// Jogo ABERTO (o contrário de `apply_game_profile`): mede-se o que roda, escreve-se no de quem não roda. `LIVRES`.
#[tauri::command]
pub async fn medir_antes(
    process: String,
    seconds: u64,
) -> Result<crate::modules::prova::Prova, String> {
    let prova = medir_para_prova(process, seconds).await?;
    crate::modules::prova::guardar(&prova)?;
    Ok(prova)
}

/// Uma medição da prova: PresentMon (canal antigo de reserva), com a geração de quadros e a impressão digital da
/// configuração gráfica do jogo no momento, para a comparação separar otimização de qualidade e de quadro gerado.
async fn medir_para_prova(
    process: String,
    seconds: u64,
) -> Result<crate::modules::prova::Prova, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            use crate::modules::windows::{configjogo, frames, presentmon};

            let (pid, nome) = frames::encontrar_processo(&process).ok_or_else(|| {
                format!(
                    "Não encontrei nenhum processo com `{}` no nome. Abra o jogo antes de medir.",
                    process
                )
            })?;
            let segundos = seconds.clamp(3, 30);

            let (medicao, resumo) = match presentmon::medir_como_antes(pid, &nome, segundos as u32)
            {
                Ok((crua, resumo)) => (crua.resumo, Some(resumo)),
                Err(erro) => {
                    crate::utils::Logger::warn(&format!(
                        "prova: PresentMon não mediu ({}); medindo pelo canal antigo",
                        erro
                    ));
                    (frames::medir(pid, &nome, segundos)?, None)
                }
            };

            Ok(crate::modules::prova::Prova {
                jogo: medicao.process,
                quando: crate::modules::changelog::now_timestamp(),
                fps: medicao.fps,
                low_1pct: medicao.low_1pct,
                engasgos_por_minuto: medicao.engasgos_por_minuto,
                segundos: medicao.seconds,
                confiavel: medicao.detalhe_confiavel,
                geracao: Some(crate::modules::medicoes::geracao_agora(resumo.as_ref())),
                presentmon: resumo,
                configuracao_do_jogo: configjogo::impressao_da_configuracao(&nome),
            })
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

#[tauri::command]
pub async fn medir_depois(
    process: String,
    seconds: u64,
) -> Result<crate::modules::prova::Comparacao, String> {
    use crate::modules::prova;

    let antes = prova::guardada()
        .ok_or("Não há medição inicial. Meça com o jogo aberto ANTES de aplicar as mudanças.")?;

    let depois = medir_para_prova(process, seconds).await?;

    Ok(prova::comparar(&antes, &depois))
}

/// `EXIGEM_LICENCA`: troca o plano de energia ativo durante o teste e desfaz o do Otimiza se ele piorar o jogo.
/// O histórico fica livre durante as rodadas (só lido no começo, escrito no fim): minutos com ele preso travariam a tela.
#[tauri::command]
pub async fn provar_o_otimizar(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    process: String,
    seconds: u64,
) -> Result<crate::modules::provaalternada::Resultado, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        use crate::modules::provaalternada::{self as prova, Desfecho};

        if prova::em_andamento() {
            return Err("Já há uma prova rodando.".to_string());
        }
        let planos = prova::planos(&*state.changes.lock().await)?;
        let (pid, nome) = crate::modules::windows::frames::encontrar_processo(&process)
            .ok_or_else(|| {
                format!(
                    "Não encontrei nenhum processo com `{}` no nome. Abra o jogo antes de provar.",
                    process
                )
            })?;

        let contra_o_equilibrado = planos.contra_o_equilibrado;
        let nome_da_thread = nome.clone();
        let planos_da_volta = planos.clone();
        let rodadas = tokio::task::spawn_blocking(move || {
            prova::executar(pid, &nome_da_thread, seconds, &planos, &|passo| {
                let _ = app.emit("prova-alternada:passo", passo);
            })
        })
        .await
        .map_err(|e| format!("Falha na prova: {}", e))??;

        let segundos = seconds.clamp(
            prova::SEGUNDOS_POR_RODADA_MIN,
            prova::SEGUNDOS_POR_RODADA_MAX,
        );
        let mut resultado = prova::decidir(
            &nome,
            crate::modules::changelog::now_timestamp(),
            segundos,
            rodadas,
            contra_o_equilibrado,
        );

        if resultado.desfecho == Desfecho::Piorou {
            let mut log = state.changes.lock().await;
            if let Err(e) = prova::voltar_ao_de_antes(&planos_da_volta, &mut log) {
                prova::marcar_falha_ao_desfazer(&mut resultado, e);
            }
        }

        crate::utils::Logger::info(&format!(
            "prova alternada em {}: {:?}, ficou com o Otimiza: {}",
            resultado.jogo, resultado.desfecho, resultado.ficou_com_otimiza
        ));
        if let Err(e) = prova::guardar(&resultado) {
            crate::utils::Logger::warn(&format!("prova alternada não foi guardada: {}", e));
        }
        Ok(resultado)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state, process, seconds);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn prova_alternada_guardada() -> Option<crate::modules::provaalternada::Resultado> {
    crate::modules::provaalternada::guardada()
}

#[tauri::command(async)]
pub fn prova_guardada() -> Option<crate::modules::prova::Prova> {
    crate::modules::prova::guardada()
}

/// `Err` quando existe e não se lê.
#[tauri::command(async)]
pub fn medicoes_automaticas() -> Result<Vec<crate::modules::medicoes::MedicaoAutomatica>, String> {
    crate::modules::medicoes::ler()
}

/// `LIVRES`. Aplicar é `optimize_now`; desfazer é `revert`. Por jogo: FPS de jogos diferentes não se compara;
/// sem jogo, o mais medido.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn protocolo_de_grupos(
    state: State<'_, AppState>,
    jogo: Option<String>,
) -> Result<Vec<crate::modules::windows::experimento::Experimento>, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::{experimento, grupos::Grupo};

        let medicoes = crate::modules::medicoes::ler_para_comparar()?;

        let Some(jogo) = jogo.or_else(|| jogo_mais_medido(&medicoes)) else {
            return Err(
                "Ainda não há nenhuma medição de jogo nesta máquina. O Otimiza mede sozinho \
                 depois de alguns minutos de partida — abra o jogo e jogue um pouco."
                    .to_string(),
            );
        };

        let log = state.changes.lock().await;
        let aplicadas = log.applied().len();

        Ok(Grupo::TODOS
            .iter()
            .map(|grupo| {
                // TODO o grupo no histórico: metade aplicada mediria uma mistura.
                let itens = crate::modules::windows::grupos::itens_do_grupo(*grupo);
                let aplicado = !itens.is_empty() && itens.iter().all(|id| log.is_applied(id));

                experimento::montar(*grupo, &jogo, &medicoes, aplicadas, aplicado)
            })
            .collect())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (state, jogo);
        Err("Só no Windows.".to_string())
    }
}

/// Mais medições, não a mais recente: o protocolo precisa de amostra dos dois lados.
#[cfg(target_os = "windows")]
fn jogo_mais_medido(medicoes: &[crate::modules::medicoes::MedicaoAutomatica]) -> Option<String> {
    let mut contagem: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for m in medicoes {
        *contagem.entry(m.jogo.as_str()).or_insert(0) += 1;
    }

    contagem
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(jogo, _)| jogo.to_string())
}

/// `LIVRES`. Com a regressão medida: se o Otimiza pode ser a causa, é o PRIMEIRO suspeito.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn por_que_o_fps_esta_baixo(
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::causas::Investigacao, String> {
    #[cfg(target_os = "windows")]
    {
        // Falhar a regressão não impede a investigação.
        let piorou = match crate::modules::medicoes::ler_para_comparar() {
            Ok(medicoes) => {
                let aplicadas = state.changes.lock().await.applied().len();
                let vereditos = crate::modules::regressao::todos(&medicoes, aplicadas);

                crate::modules::regressao::pior_regressao(&vereditos)
                    .and_then(|v| v.variacao_fps_pct.map(|pct| (v.jogo.clone(), pct)))
            }
            Err(_) => None,
        };

        Ok(crate::modules::windows::causas::investigar(piorou))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Só no Windows.".to_string())
    }
}
