// Otimizações do Windows: catálogo, níveis, aplicar/desfazer, o botão "Otimizar", recuperação pendente e
// o relatório das alterações. É o caminho que grava no registro, então o desfazer mora junto do aplicar.

use super::*;

/// `None` é o normal; `Some` traz os valores anteriores de uma operação interrompida. NÃO conserta sozinho: seria
/// decidir pelo cliente com base num arquivo que já provou que algo deu errado.
#[tauri::command(async)]
pub fn recuperacao_pendente() -> Result<Option<PendenciaNaTela>, String> {
    Ok(
        crate::modules::transacao::pendente()?.map(|p| PendenciaNaTela {
            // A frase aqui e não na tela: duas versões da explicação discordariam.
            explicacao: p.explicacao(),
            pendencia: p,
        }),
    )
}

#[derive(Debug, Serialize)]
pub struct PendenciaNaTela {
    pub pendencia: crate::modules::transacao::Pendencia,
    pub explicacao: String,
}

/// Só quando o cliente pede (ver `recuperacao_pendente`). Devolve quantas mudanças desfez.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn concluir_recuperacao(state: State<'_, AppState>) -> Result<usize, String> {
    let Some(pendencia) = crate::modules::transacao::pendente()? else {
        return Err("Não há operação pela metade para terminar.".to_string());
    };

    let mut log = state.changes.lock().await;
    crate::modules::windows::concluir_recuperacao(&pendencia, &mut log)
}

/// O nome não disfarça: DESCARTAR, não resolver.
#[tauri::command(async)]
pub fn descartar_pendencia() -> Result<(), String> {
    crate::modules::transacao::descartar()
}

/// `LIVRES`: definição montada do catálogo; aplicar é `optimize_now`. Perfis respondem "para que você usa"; níveis,
/// "o que você aceita trocar": perguntas independentes.
#[tauri::command(async)]
pub fn niveis_de_otimizacao() -> Vec<NivelNaTela> {
    use crate::modules::windows::niveis::{acrescenta, aplica_de_uma_vez, itens_do_nivel, Nivel};

    Nivel::TODOS
        .iter()
        .map(|nivel| NivelNaTela {
            id: format!("{nivel:?}"),
            nome: nivel.nome().to_string(),
            promessa: nivel.promessa().to_string(),
            exigencia: nivel.exigencia().to_string(),
            itens: itens_do_nivel(*nivel)
                .iter()
                .map(|s| s.to_string())
                .collect(),
            acrescenta: acrescenta(*nivel).iter().map(|s| s.to_string()).collect(),
            aplica_de_uma_vez: aplica_de_uma_vez(*nivel),
        })
        .collect()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NivelNaTela {
    pub id: String,
    pub nome: String,
    pub promessa: String,
    /// Parte do contrato, não nota de rodapé.
    pub exigencia: String,
    pub itens: Vec<String>,
    pub acrescenta: Vec<String>,
    /// Falso no Experimental: aplicar tudo de uma vez derrubou o FPS de um cliente.
    pub aplica_de_uma_vez: bool,
}

/// Sugestão que marca caixas, não pacote fechado.
#[tauri::command(async)]
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

/// Cada análise é independente: seção que falha sai dita como indisponível. Mapa de pastas e varredura de espaço
/// ficam fora: quase um minuto cada, com o cliente esperando.
#[cfg(target_os = "windows")]
fn coletar_para_relatorio() -> crate::modules::report::ReportData {
    use crate::modules::windows;

    crate::modules::report::ReportData {
        boot: Some(windows::boot::analyze()),
        thermal: Some(windows::thermal::analyze()),
        health: Some(windows::health::analyze()),
        memory: Some(windows::memory::analyze()),
        browsers: Some(windows::browsers::analyze()),
        // Ilegível vira lacuna escrita; como lista vazia, a seção sumia e o laudo afirmava por omissão.
        startup: windows::startup::entries(),
        // O mesmo veredito da tela: papel e programa não discordam.
        veredito: Some(windows::veredito::diagnostico_rapido()),
    }
}

#[cfg(not(target_os = "windows"))]
fn coletar_para_relatorio() -> crate::modules::report::ReportData {
    crate::modules::report::ReportData::default()
}

/// A comparação vem da interface: refazer o benchmark mediria outro momento.
#[tauri::command]
pub async fn export_report(
    state: State<'_, AppState>,
    comparison: Option<BenchmarkComparison>,
) -> Result<crate::modules::report::ReportSaved, String> {
    // WMI e log de eventos passam de dez segundos: fora do runtime.
    let dados = tokio::task::spawn_blocking(coletar_para_relatorio)
        .await
        .map_err(|e| format!("Falha ao levantar os dados da maquina: {}", e))?;

    let changes = state.changes.lock().await;
    crate::modules::report::save(&changes, comparison.as_ref(), &dados)
}

/// `LIVRES`: só lê o histórico.
#[tauri::command]
pub async fn exportar_alteracoes(state: State<'_, AppState>) -> Result<String, String> {
    let log = state.changes.lock().await;
    let csv = crate::modules::report::csv_das_alteracoes(&log, valor_novo_da_alteracao);
    crate::modules::report::salvar_csv(&csv)
}

#[cfg(target_os = "windows")]
fn valor_novo_da_alteracao(
    id: &str,
    c: &crate::modules::changelog::ChangeRecord,
) -> Option<String> {
    use crate::modules::changelog::ChangeRecord;
    use crate::modules::windows::catalog::{self, Action, RegValue};
    match c {
        ChangeRecord::RegistryValue { path, name, .. } => {
            catalog::find(id)?.actions.iter().find_map(|a| match a {
                Action::Registry {
                    path: p,
                    name: n,
                    value,
                    ..
                } if p.eq_ignore_ascii_case(path) && n.eq_ignore_ascii_case(name) => {
                    Some(match value {
                        RegValue::Dword(v) => v.to_string(),
                        RegValue::Text(t) => t.to_string(),
                        RegValue::Binary(b) => b
                            .iter()
                            .map(|x| format!("{:02X}", x))
                            .collect::<Vec<_>>()
                            .join(" "),
                    })
                }
                _ => None,
            })
        }
        ChangeRecord::LimiteNvidia { fps, .. } => Some(format!("{} FPS", fps)),
        ChangeRecord::PerfilNvidia { perfil, .. } => Some(format!("perfil {}", perfil)),
        ChangeRecord::Hibernation { .. } => Some("desligada".into()),
        ChangeRecord::MemoryCompression { .. } => Some("desligada".into()),
        ChangeRecord::ReservedStorage { .. } => Some("desligado".into()),
        ChangeRecord::ScheduledTask {
            previously_enabled, ..
        } => Some(
            if *previously_enabled {
                "desligada"
            } else {
                "ligada"
            }
            .into(),
        ),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn valor_novo_da_alteracao(_: &str, _: &crate::modules::changelog::ChangeRecord) -> Option<String> {
    None
}

#[tauri::command]
pub async fn fix_readiness(id: String) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            use crate::modules::windows::readiness;

            match id.as_str() {
                "trim" => readiness::ligar_trim(),
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

#[tauri::command]
pub async fn set_persistent_priority(
    executable: String,
    enable: bool,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    // RETIRADO NA 2.9 (prioridade cega, e a escrita mais visível a um anticheat): quem fixou antes ainda remove.
    if enable {
        return Err("Fixar prioridade foi retirado na 2.9: não mostrava ganho medido. Ainda dá para remover o que foi fixado antes.".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new().set_persistent_priority(
            &executable,
            enable,
            &mut log,
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (executable, enable, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

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

#[tauri::command]
pub async fn set_game_mode(active: bool, state: State<'_, AppState>) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::gamemode;

        let mut log = state.changes.lock().await;
        // A escolha à mão vale por cima do Turbo: daqui em diante o governador é da pessoa (ligado ou desligado).
        crate::plataforma::turbo::marcar_governo(false);

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

/// `LIVRES`: só devolve e esquece.
#[tauri::command]
pub async fn zerar_modo_jogo(state: State<'_, AppState>) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let log = state.changes.lock().await;
        crate::modules::windows::gamemode::zerar(&log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Vazio por não haver nada e vazio por não saber são diferentes. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn estado_do_historico(
    state: State<'_, AppState>,
) -> Result<crate::modules::changelog::LeituraDoHistorico, String> {
    let log = state.changes.lock().await;

    Ok(log.leitura().clone())
}

/// A linha do tempo da área Histórico: cada registro, do mais novo ao mais antigo, com o valor de antes como foi
/// GRAVADO (o `describe` de cada mudança) e quando. A `leitura` vem junto: lista vazia com histórico ilegível não é
/// "nada mudou". `LIVRES`: só lê.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn historico_de_mudancas(state: State<'_, AppState>) -> Result<HistoricoNaTela, String> {
    let log = state.changes.lock().await;

    Ok(linha_do_tempo(&log))
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct HistoricoNaTela {
    pub leitura: crate::modules::changelog::LeituraDoHistorico,
    /// Do mais novo ao mais antigo.
    pub mudancas: Vec<MudancaNaTela>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct MudancaNaTela {
    /// O que o `revert_optimization` recebe.
    pub id: String,
    pub nome: String,
    /// Segundos desde 1970, como o histórico grava.
    pub quando: u64,
    /// Uma linha por valor mexido, com o anterior gravado ("registro · Chave\Valor (antes: 1)").
    pub o_que_mudou: Vec<String>,
    /// `None`: o item não está no catálogo (energia, jogo, driver) e não dá para afirmar; a tela não chuta.
    pub precisa_de_admin: Option<bool>,
    pub precisa_reiniciar: Option<bool>,
    /// `false` só para o que apagou arquivo de verdade: o desfazer recusa, e a tela não oferece o botão.
    pub desfazivel: bool,
}

/// **Pura**, tirando o catálogo, que é estático.
fn linha_do_tempo(log: &ChangeLog) -> HistoricoNaTela {
    let mudancas = log
        .applied()
        .iter()
        .rev()
        .map(|entrada| {
            #[cfg(target_os = "windows")]
            let (admin, reiniciar, desfazivel) = {
                let spec = crate::modules::windows::catalog::find(&entrada.optimization_id);
                (
                    spec.map(|s| s.requires_admin),
                    spec.map(|s| s.requires_restart),
                    spec.map(|s| s.reversible).unwrap_or(true),
                )
            };
            #[cfg(not(target_os = "windows"))]
            let (admin, reiniciar, desfazivel) = (None, None, true);

            MudancaNaTela {
                id: entrada.optimization_id.clone(),
                nome: entrada.name.clone(),
                quando: entrada.timestamp,
                o_que_mudou: entrada.changes.iter().map(|c| c.describe()).collect(),
                precisa_de_admin: admin,
                precisa_reiniciar: reiniciar,
                desfazivel,
            }
        })
        .collect();

    HistoricoNaTela {
        leitura: log.leitura().clone(),
        mudancas,
    }
}

#[tauri::command]
#[cfg_attr(test, specta::specta)]
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

#[tauri::command]
#[cfg_attr(test, specta::specta)]
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

/// Guardião (`modules::windows::guardiao`): relê cada ajuste do catálogo que o histórico diz estar aplicado.
/// Só leitura, `LIVRES`.
#[tauri::command]
pub async fn vistoriar_mudancas(
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::guardiao::Vistoria, String> {
    #[cfg(target_os = "windows")]
    {
        // Copia a lista e solta o histórico: a leitura passa por bcdedit, powercfg e PowerShell.
        let aplicadas: Vec<(String, String)> = state
            .changes
            .lock()
            .await
            .applied()
            .iter()
            .map(|a| (a.optimization_id.clone(), a.name.clone()))
            .collect();
        tokio::task::spawn_blocking(move || {
            crate::modules::windows::WindowsOptimizer::new().vistoriar(&aplicadas)
        })
        .await
        .map_err(|e| format!("Falha ao conferir as mudanças: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Desfaz e aplica de novo o que não está mais como o Otimiza deixou. `EXIGEM_LICENCA`, como aplicar.
#[tauri::command]
pub async fn refazer_mudanca(
    id: String,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new().refazer(&id, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (id, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`. Quantas sessões seguidas caíram e se as tarefas automáticas estão paradas por isso.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModoSeguro {
    pub ativo: bool,
    pub quedas_seguidas: u32,
}

#[tauri::command(async)]
pub fn modo_seguro() -> ModoSeguro {
    ModoSeguro {
        ativo: crate::utils::diagnostico::modo_seguro(),
        quedas_seguidas: crate::utils::diagnostico::quedas_seguidas(),
    }
}

/// `LIVRES`: só religa o que o próprio Otimiza parou.
#[tauri::command(async)]
pub fn sair_do_modo_seguro() {
    crate::utils::diagnostico::sair_do_modo_seguro();
    crate::utils::Logger::info("modo seguro: desligado pela pessoa; tarefas automáticas religadas");
}

/// `LIVRES`: não impede nada (há casos legítimos). Diferente de `detect_conflicts`, que é entre PROGRAMAS.
#[tauri::command]
pub async fn conflitos_entre_ajustes(
    state: State<'_, AppState>,
) -> Result<Vec<crate::modules::windows::conflitos::Conflito>, String> {
    let aplicados: Vec<String> = state
        .changes
        .lock()
        .await
        .applied()
        .iter()
        .map(|a| a.optimization_id.clone())
        .collect();

    Ok(crate::modules::windows::conflitos::entre(&aplicados)
        .into_iter()
        .cloned()
        .collect())
}

/// `LIVRES`. O cliente compara com vídeos de "50 tweaks" sem saber que metade não faz nada.
#[tauri::command(async)]
pub fn o_que_nao_fazemos() -> Vec<crate::modules::windows::naofazemos::NaoFazemos> {
    crate::modules::windows::naofazemos::LISTA.to_vec()
}

#[cfg(target_os = "windows")]
#[tauri::command(async)]
pub fn o_que_o_otimiza_altera() -> Vec<crate::modules::windows::registro::Alteracao> {
    crate::modules::windows::registro::todas()
}

/// Antes contra depois, pelo histórico do vigia (na 2.1.0 os dois números estavam no disco e ninguém comparou).
/// `Err` quando não se lê: lista vazia afirmaria que nenhum jogo piorou.
#[tauri::command]
pub async fn conferir_o_proprio_trabalho(
    state: State<'_, AppState>,
) -> Result<Vec<crate::modules::regressao::Veredito>, String> {
    let medicoes = crate::modules::medicoes::ler_para_comparar()?;
    let aplicadas = state.changes.lock().await.applied().len();

    Ok(crate::modules::regressao::todos(&medicoes, aplicadas))
}

#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn revert_optimization(
    id: String,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        let resultado = crate::modules::windows::WindowsOptimizer::new().revert(&id, &mut log);
        if resultado.as_ref().is_ok_and(|r| r.success) {
            crate::modules::portao::esquecer(&id);
        }
        resultado
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (id, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `only` restringe a uma lista de ids (perfis); os filtros de segurança valem igual.
#[tauri::command]
pub async fn optimize_now(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    only: Option<Vec<String>>,
    nivel: Option<crate::modules::windows::niveis::Nivel>,
) -> Result<Vec<OptimizationOutcome>, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;

        // Rede de segurança antes de mudar: falhar não bloqueia, mas o cliente sabe.
        if Preferences::load().restore_point_before_batch {
            let inicio = std::time::Instant::now();
            crate::utils::Logger::info("ponto de restauração antes do lote: começou");

            let (message, success) =
                match crate::modules::windows::restore::create("Otimiza - antes de otimizar") {
                    Ok(message) => (message, true),
                    Err(error) => (error, false),
                };

            crate::utils::Logger::info(&format!(
                "ponto de restauração antes do lote: {} em {} ms — {}",
                if success { "criado" } else { "não criado" },
                inicio.elapsed().as_millis(),
                message
            ));

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

        // Cada passo emitido na hora, não uma barra de progresso.
        let resultados = crate::modules::windows::WindowsOptimizer::new().apply_selection(
            only.as_deref(),
            nivel,
            &mut log,
            |step| {
                let _ = app.emit("optimize:step", step);
            },
        );

        // O teto de FPS do jogo no mesmo clique: a configuração do jogo move FPS em dezenas, e quem aperta o botão não
        // O "Otimizar" NÃO tira mais o limite de FPS do jogo. Medido no FiveM do dono (preso no processador, 99% de CPU,
        // gargalo na CPU): sem limite o tempo de quadro oscila mais, e é a oscilação — não um teto baixo — que o cliente
        // sente como tranco. Quem quer ritmo liso usa o "Jogo liso", que TRAVA o FPS num valor estável. `aplicar_teto_do_jogo`
        // fica só para o desfazer de quem já recebeu o SemTeto em versões anteriores.
        let _ = aplicar_teto_do_jogo;

        Ok(resultados)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state, only);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `None` sem nada a dizer. Jogo aberto não é falha do lote (ele reescreve ao sair): o cliente precisa saber, mas
/// o lote de Windows foi bem.
#[cfg(target_os = "windows")]
async fn aplicar_teto_do_jogo(
    app: &tauri::AppHandle,
    log: &mut crate::modules::changelog::ChangeLog,
) -> Option<OptimizationOutcome> {
    use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeRecord};
    use crate::modules::optimizer::{ActionResult, ActionStatus};
    use crate::modules::windows::configjogo::{self, Perfil};

    const ID: &str = "config_jogo_sem_teto";
    const NOME: &str = "Limite de quadros do jogo";

    if log.is_applied(ID) {
        return None;
    }

    let inicio = std::time::Instant::now();

    let (mensagem, sucesso, mudou) = match configjogo::aplicar_perfil(Perfil::SemTeto) {
        Ok(feito) if feito.mudou.is_empty() => {
            return None;
        }
        Ok(feito) => {
            let resumo = format!("{}: {}", feito.jogo, feito.mudou.join(", "));

            // O arquivo inteiro: "Desfazer tudo" devolve byte a byte.
            let gravou = log.record(AppliedOptimization {
                optimization_id: ID.to_string(),
                name: NOME.to_string(),
                timestamp: now_timestamp(),
                changes: vec![ChangeRecord::GameConfig {
                    caminho: feito.arquivo.to_string_lossy().to_string(),
                    anterior: Some(feito.anterior),
                    jogo: feito.jogo.clone(),
                }],
            });

            match gravou {
                Ok(()) => (resumo, true, feito.mudou),
                Err(erro) => (
                    format!(
                        "{} — mas o histórico não pôde ser gravado: {}",
                        resumo, erro
                    ),
                    false,
                    feito.mudou,
                ),
            }
        }
        Err(erro) => (erro, false, Vec::new()),
    };

    let _ = app.emit(
        "optimize:step",
        BatchStep {
            index: 0,
            total: 0,
            name: NOME.to_string(),
            stage: "finished",
            message: mensagem.clone(),
            changes: mudou.clone(),
            success: sucesso,
        },
    );

    Some(OptimizationOutcome {
        success: sucesso,
        applied: sucesso,
        message: mensagem.clone(),
        duration_ms: inicio.elapsed().as_millis() as u64,
        changes_count: mudou.len(),
        changes: mudou.clone(),
        actions: vec![ActionResult {
            name: "configuração do jogo".to_string(),
            status: if sucesso {
                ActionStatus::Verified
            } else {
                ActionStatus::Failed
            },
            message: mensagem.clone(),
            after_value: Some(mudou.join(", ")),
            ..Default::default()
        }],
        ..OptimizationOutcome::novo(ID, NOME, mensagem)
    })
}

/// "Desfazer tudo": o histórico E o que tem estado próprio (motor de energia, perfis por jogo, regras de núcleos,
/// modos automáticos). Ver `modules::windows::restaurar`. `LIVRES`: desfazer nunca depende de licença.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
/// Genérico no runtime só para o contrato tipado (`contrato.rs`) montar com o `MockRuntime`; no programa é o Wry.
pub async fn restaurar_tudo<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::restaurar::Relatorio, String> {
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;

        crate::modules::windows::restaurar::restaurar_tudo(&mut log, |step| {
            let _ = app.emit("optimize:step", step);
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Só leitura, antes do "Otimizar agora" e dos perfis: com esses serviços desligados, programa trava com ou sem
/// otimização. Ver `essenciais.rs`.
#[tauri::command]
pub async fn checar_essenciais() -> Result<Checagem, String> {
    #[cfg(target_os = "windows")]
    {
        let checagem = tokio::task::spawn_blocking(crate::modules::windows::essenciais::checar)
            .await
            .map_err(|e| format!("Falha ao conferir os serviços essenciais: {}", e))?;

        // No log também, logo acima do lote.
        let desativados: Vec<&str> = checagem
            .servicos
            .iter()
            .filter(|s| s.inicio == crate::modules::windows::essenciais::Inicio::Desativado)
            .map(|s| s.servico)
            .collect();
        crate::utils::Logger::info(&format!(
            "essenciais: {} desativado(s){}{} — fabricante {:?}, modelo {:?}",
            desativados.len(),
            if desativados.is_empty() { "" } else { ": " },
            desativados.join(", "),
            checagem.fabricante,
            checagem.modelo
        ));

        Ok(checagem)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `EXIGEM_LICENCA`. O "Desfazer" os devolve a desligados.
#[tauri::command]
pub async fn religar_essenciais(state: State<'_, AppState>) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new().religar_essenciais(&mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::changelog::{AppliedOptimization, ChangeRecord, LeituraDoHistorico, PreviousValue};

    #[test]
    fn a_linha_do_tempo_vem_do_mais_novo_com_o_valor_de_antes_gravado() {
        let mut log = ChangeLog::em_memoria();
        for (id, quando) in [("ajuste_antigo_de_teste", 100), ("ajuste_novo_de_teste", 200)] {
            log.record(AppliedOptimization {
                optimization_id: id.to_string(),
                name: format!("Ajuste {}", id),
                timestamp: quando,
                changes: vec![ChangeRecord::RegistryValue {
                    hive: "HKCU".into(),
                    path: r"Software\Teste".into(),
                    name: "Valor".into(),
                    previous: PreviousValue::Dword(7),
                }],
            })
            .expect("grava no temporário");
        }

        let tela = linha_do_tempo(&log);

        assert_eq!(tela.leitura, LeituraDoHistorico::Ok);
        let ids: Vec<&str> = tela.mudancas.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["ajuste_novo_de_teste", "ajuste_antigo_de_teste"]);
        assert_eq!(tela.mudancas[0].quando, 200);
        assert_eq!(tela.mudancas[0].o_que_mudou, [r"registro · Teste\Valor (antes: 7)"]);
        // Fora do catálogo: a tela não ganha um "precisa de administrador" chutado.
        assert_eq!(tela.mudancas[0].precisa_de_admin, None);
        assert_eq!(tela.mudancas[0].precisa_reiniciar, None);
        assert!(tela.mudancas[0].desfazivel);
    }
}
