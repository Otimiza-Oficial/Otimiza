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
use crate::modules::windows::essenciais::Checagem;
#[cfg(target_os = "windows")]
use crate::modules::windows::nvdriver::PainelDoDriver;
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
use crate::modules::windows::framegen::{Artefato, Deteccao, Intensidade, Perfil, ResultadoDaRodada, Rodada, Tecnologia};
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

// `tokio::sync::Mutex`: os guards atravessam `.await`, e um `std::sync::MutexGuard` não é Send.
pub struct AppState {
    pub monitor: Mutex<PerformanceMonitor>,
    pub changes: Mutex<ChangeLog>,
    /// Mantido: uso de CPU por processo só existe comparando duas leituras.
    #[cfg(target_os = "windows")]
    pub processes: Mutex<crate::modules::windows::processes::ProcessMonitor>,
    /// Sem `Mutex`: `TarefaLonga` guarda o próprio estado e impede dois reparos juntos.
    #[cfg(target_os = "windows")]
    pub reparo: crate::modules::windows::tarefa_longa::TarefaLonga,
    /// Autoriza (ou não) agendar o `chkdsk`. `std::sync::Mutex`: os comandos de reparo são síncronos marcados
    /// `(async)`, e um `blocking_lock()` do tokio dentro do runtime entraria em pânico.
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

#[tauri::command]
pub async fn get_performance_metrics(state: State<'_, AppState>) -> Result<PerformanceMetrics, String> {
    let mut monitor = state.monitor.lock().await;
    monitor.collect_metrics().await
}

/// SÓ LÊ. O instalado vem do REGISTRO (três chaves de desinstalação), não do winget, que falta justamente nas
/// máquinas que mais precisam.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn catalogo_de_programas() -> Result<ProgramasNaTela, String> {
    use crate::modules::programas;
    use crate::modules::windows::{conflicts, winget};

    let (instalados, winget) = tokio::task::spawn_blocking(|| {
        (conflicts::programas_instalados(), winget::disponibilidade())
    })
    .await
    .map_err(|e| format!("a leitura de programas não terminou: {e}"))?;

    // Falha na leitura vira desconhecido em todos, não lista vazia: senão o técnico instalaria por cima.
    let lidos = instalados.as_deref().ok();

    Ok(ProgramasNaTela {
        programas: programas::montar(lidos),
        winget,
        lacuna: instalados.err(),
    })
}

#[cfg(target_os = "windows")]
#[derive(Debug, Serialize)]
pub struct ProgramasNaTela {
    pub programas: Vec<crate::modules::programas::NaLista>,
    pub winget: crate::modules::windows::winget::Disponibilidade,
    pub lacuna: Option<String>,
}

/// O pacote do winget sai do CATÁLOGO pelo id curto, nunca da tela: o que instalar não fica fora do nosso controle.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn instalar_programa(id: String) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    use crate::modules::programas;
    use crate::modules::windows::winget;

    let programa = programas::por_id(&id).ok_or("programa fora do catálogo")?;
    let pacote = programa.winget;

    tokio::task::spawn_blocking(move || winget::instalar(pacote))
        .await
        .map_err(|e| format!("a instalação não terminou: {e}"))?
}

/// Separado de apagar: medir precisa ser repetível sem consequência. Alvo não medido sai AUSENTE, não zero (que
/// diria pasta vazia).
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn medir_limpeza() -> Result<LimpezaNaTela, String> {
    use crate::modules::limpeza;
    use crate::modules::windows::limpar;

    let alvos = tokio::task::spawn_blocking(limpar::medir)
        .await
        .map_err(|e| format!("a medição não terminou: {e}"))?;

    Ok(LimpezaNaTela {
        marcados: limpeza::marcados_por_padrao(),
        alvos,
    })
}

#[cfg(target_os = "windows")]
#[derive(Debug, Serialize)]
pub struct LimpezaNaTela {
    pub alvos: Vec<crate::modules::limpeza::AlvoMedido>,
    /// Nada que contenha arquivo do cliente vem marcado.
    pub marcados: Vec<String>,
}

/// A ÚNICA OPERAÇÃO SEM DESFAZER: recebe a lista EXPLÍCITA marcada item a item. Id fora do catálogo é recusado;
/// caminho vindo da tela nunca.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn limpar_alvos(ids: Vec<String>) -> Result<Vec<LimparResultado>, String> {
    crate::modules::licenca::exigir()?;

    use crate::modules::windows::limpar;

    tokio::task::spawn_blocking(move || {
        ids.iter().map(|id| limpar::apagar(id)).collect()
    })
    .await
    .map_err(|e| format!("a limpeza não terminou: {e}"))
}

#[cfg(target_os = "windows")]
pub type LimparResultado = crate::modules::windows::limpar::Resultado;

/// SÓ LÊ. A classe de cada núcleo vem do Windows, não de tabela de modelos. Processador uniforme: NÃO HÁ O QUE
/// FAZER, escrito (um botão ali seria um jeito de piorar).
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn nucleos_da_maquina() -> Result<NucleosNaTela, String> {
    use crate::modules::nucleos;
    use crate::modules::windows::{afinidade, deteccao, topologia};

    let (topologia, jogo) =
        tokio::task::spawn_blocking(|| (topologia::ler(), deteccao::procurar()))
            .await
            .map_err(|e| format!("a leitura dos núcleos não terminou: {e}"))?;

    let topologia = topologia.ok_or(
        "o Windows não informou a lista de núcleos desta máquina. Sem ela o produto não oferece \
         prender o jogo em núcleo nenhum — prender no núcleo errado é pior que não prender.",
    )?;

    // A afinidade atual diz se o jogo JÁ está preso, por exemplo nos núcleos de eficiência por outro programa.
    let (jogo_nome, jogo_pid, jogo_mascara, jogo_exe) = match &jogo {
        Some(j) => {
            let mascara = afinidade::ler(j.pid).ok().map(|(processo, _)| processo);
            (Some(j.nome.clone()), Some(j.pid), mascara, Some(j.executavel.clone()))
        }
        None => (None, None, None, None),
    };

    Ok(NucleosNaTela {
        conselho: nucleos::conselho(&topologia),
        // Daqui, não da tela: duas versões da conta de físicos discordariam.
        fisicos: topologia.fisicos(),
        mascara_de_desempenho: nucleos::mascara_de_desempenho(&topologia).map(|m| m.to_string()),
        topologia,
        jogo_nome,
        jogo_pid,
        jogo_mascara: jogo_mascara.map(|m| m.to_string()),
        jogo_exe,
    })
}

#[cfg(target_os = "windows")]
#[derive(Debug, Serialize)]
pub struct NucleosNaTela {
    pub topologia: crate::modules::nucleos::Topologia,
    pub conselho: crate::modules::nucleos::Conselho,
    pub fisicos: usize,
    /// TEXTO: uma máscara de 64 bits perde os bits altos no número do JavaScript.
    pub mascara_de_desempenho: Option<String>,
    pub jogo_nome: Option<String>,
    pub jogo_pid: Option<u32>,
    pub jogo_mascara: Option<String>,
    pub jogo_exe: Option<String>,
}

/// NÃO entra no histórico: afinidade morre com o processo, e uma linha lá ofereceria para sempre desfazer um
/// processo que não existe. O desfazer é o botão ao lado, ou fechar o jogo.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn prender_jogo_nos_nucleos(pid: u32, prender: bool) -> Result<String, String> {
    crate::modules::licenca::exigir()?;

    use crate::modules::nucleos;
    use crate::modules::windows::{afinidade, topologia};

    tokio::task::spawn_blocking(move || {
        if !prender {
            afinidade::soltar(pid)?;
            return Ok("O jogo voltou a poder usar todos os núcleos.".to_string());
        }

        let t = topologia::ler().ok_or("o Windows não informou a lista de núcleos.")?;

        // Processador uniforme não tem núcleo melhor: prender só tira máquina.
        let mascara = nucleos::mascara_de_desempenho(&t).ok_or(
            "este processador tem todos os núcleos iguais: prender o jogo em parte deles só \
             reduziria o que a máquina entrega.",
        )?;

        afinidade::escrever(pid, mascara)?;

        Ok(format!(
            "O jogo está preso nos {} núcleos de desempenho. Isso vale para o processo aberto e \
             some quando o jogo fechar.",
            nucleos::indices_de(mascara).len()
        ))
    })
    .await
    .map_err(|e| format!("a mudança não terminou: {e}"))?
}

/// Auto CPU Set (2.9): mede o jogo em todos os núcleos e só nos de desempenho, alternando, e fica com o que rendeu.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn cpuset_testar(pid: u32, executavel: String) -> Result<crate::modules::windows::cpuset::ResultadoCpuSet, String> {
    crate::modules::licenca::exigir()?;
    tokio::task::spawn_blocking(move || crate::modules::windows::cpuset::testar(pid, &executavel))
        .await
        .map_err(|e| format!("o teste não terminou: {e}"))?
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn cpuset_resultados() -> Result<Vec<crate::modules::windows::cpuset::ResultadoCpuSet>, String> {
    Ok(crate::modules::windows::cpuset::ler().into_values().collect())
}

/// `LIVRES`: só tira.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn cpuset_esquecer(executavel: String) -> Result<(), String> {
    crate::modules::windows::cpuset::esquecer(&executavel)
}

/// NÃO mede mira nem olha dentro do jogo: duas chaves do registro do usuário. `intervalos_us` vêm da tela; lista
/// curta dá taxa ausente, nunca zero.
#[cfg(target_os = "windows")]
#[tauri::command]
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

/// SEM ESTADO NO BACKEND: a sessão vem da tela e volta; fechar o programa encerra a sessão, e o aplicado segue no
/// histórico. Não aplica nem desfaz: devolve o passo.
#[tauri::command]
pub fn passo_do_autoajuste(
    sessao: crate::modules::autoajuste::Sessao,
) -> crate::modules::autoajuste::Passo {
    crate::modules::autoajuste::proximo_passo(&sessao)
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
#[tauri::command]
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

/// `None` é o normal; `Some` traz os valores anteriores de uma operação interrompida. NÃO conserta sozinho: seria
/// decidir pelo cliente com base num arquivo que já provou que algo deu errado.
#[tauri::command]
pub fn recuperacao_pendente() -> Result<Option<PendenciaNaTela>, String> {
    Ok(crate::modules::transacao::pendente()?.map(|p| PendenciaNaTela {
        // A frase aqui e não na tela: duas versões da explicação discordariam.
        explicacao: p.explicacao(),
        pendencia: p,
    }))
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
#[tauri::command]
pub fn descartar_pendencia() -> Result<(), String> {
    crate::modules::transacao::descartar()
}

#[tauri::command]
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

#[tauri::command]
pub fn get_baseline() -> Option<BenchmarkSnapshot> {
    BaselineStore::load()
}

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

// Existem em todas as plataformas para uma interface só; fora do Windows falham com mensagem clara.

/// Para avisar antes de o usuário tentar aplicar e falhar.
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
    pub storage: String,
    pub total_ram_gb: f64,
    pub logical_cores: usize,
    pub cpu_name: String,
    pub gpu_name: String,
}

/// Permite recusar o que faria mal a este PC.
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

#[tauri::command]
pub fn get_preferences() -> Preferences {
    Preferences::load()
}

#[tauri::command]
pub fn set_preferences(preferences: Preferences) -> Result<Preferences, String> {
    preferences.save()?;
    // Valores fora da faixa são corrigidos: a interface reflete o gravado, não o pedido.
    Ok(Preferences::load())
}

/// Não apaga nada.
#[tauri::command]
pub async fn scan_disk_space() -> Result<DiskReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::diskspace::scan)
            .await
            .map_err(|e| format!("Falha na varredura: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

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

// A Lixeira saiu do liberador na 2.9: é item da Limpeza do sistema, desmarcado.

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

/// `LIVRES`: **não escreve absolutamente nada** (uma ferramenta capaz de alterar firmware deixaria a máquina sem
/// ligar). Memória abaixo do nominal e Resizable BAR vêm de quem já mede: remedir daria dois números.
#[tauri::command]
pub fn passo_a_passo_da_bios() -> BiosNaTela {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::{bios, firmware, planoenergia, rbar};

        let leitura = bios::ler();

        let memoria_abaixo = firmware::analyze_memory_ou_lacuna()
            .map(|achados| achados.iter().any(|a| a.id == "memory_xmp_off"))
            .unwrap_or(false);

        let rebar_desligado = matches!(
            rbar::analyze().estado,
            rbar::EstadoDoRbar::DesligadoESuportado
        );

        let amd = matches!(
            planoenergia::detectar().fabricante_da_cpu,
            planoenergia::FabricanteDaCpu::Amd
        );

        BiosNaTela {
            passos: bios::montar(&leitura, memoria_abaixo, rebar_desligado, amd),
            leitura,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        BiosNaTela::default()
    }
}

/// `LIVRES`: só mede.
#[tauri::command]
pub fn abertura_pronta() -> Option<u64> {
    let ms = crate::modules::abertura::marcar_pronta();
    if let Some(ms) = ms {
        crate::utils::Logger::info(&format!("abertura: {} ms", ms));
    }
    ms
}

/// `LIVRES`: só lê.
#[tauri::command]
pub async fn ficha_da_bios() -> Result<crate::modules::windows::fichabios::Ficha, String> {
    tokio::task::spawn_blocking(crate::modules::windows::fichabios::ler)
        .await
        .map_err(|e| format!("Falha ao ler o firmware: {}", e))
}

/// `LIVRES`: grava a ficha num arquivo da Área de Trabalho, com nome escolhido aqui. O texto vem da tela (é o que
/// ela mostra), limitado em tamanho.
#[tauri::command]
pub fn salvar_ficha_da_bios(texto: String) -> Result<String, String> {
    const LIMITE: usize = 64 * 1024;
    if texto.trim().is_empty() || texto.len() > LIMITE {
        return Err("A ficha está vazia ou grande demais para salvar.".to_string());
    }
    crate::modules::report::salvar_texto("ficha da BIOS", &texto)
}

/// `LIVRES`: não altera configuração; a tela confirma antes.
#[tauri::command]
pub fn reiniciar_na_bios() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::fichabios::reiniciar_na_bios()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BiosNaTela {
    pub leitura: crate::modules::windows::bios::Leitura,
    pub passos: Vec<crate::modules::windows::bios::Passo>,
}

/// `LIVRES`: definição montada do catálogo; aplicar é `optimize_now`. Perfis respondem "para que você usa"; níveis,
/// "o que você aceita trocar": perguntas independentes.
#[tauri::command]
pub fn niveis_de_otimizacao() -> Vec<NivelNaTela> {
    use crate::modules::windows::niveis::{acrescenta, aplica_de_uma_vez, itens_do_nivel, Nivel};

    Nivel::TODOS
        .iter()
        .map(|nivel| NivelNaTela {
            id: format!("{nivel:?}"),
            nome: nivel.nome().to_string(),
            promessa: nivel.promessa().to_string(),
            exigencia: nivel.exigencia().to_string(),
            itens: itens_do_nivel(*nivel).iter().map(|s| s.to_string()).collect(),
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

/// Não "o que dá para apagar", mas "cadê o meu disco".
#[tauri::command]
pub async fn map_folders() -> Result<FolderMap, String> {
    #[cfg(target_os = "windows")]
    {
        // Centenas de milhares de arquivos: fora do runtime async.
        tokio::task::spawn_blocking(|| {
            use crate::modules::windows::foldermap;
            foldermap::mapear_o_disco(12)
        })
        .await
        .map_err(|e| format!("Falha ao mapear pastas: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: o `rbar` só lê, e só a BIOS liga. `(async)`: chama `nvidia-smi`, processo externo.
#[tauri::command(async)]
pub fn analyze_rbar() -> Result<crate::modules::windows::rbar::RelatorioDoRbar, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::rbar::analyze())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
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

/// (2.9, modo Expert). `LIVRES`.
#[tauri::command]
pub async fn diagnostico_dpc() -> Result<crate::modules::windows::dpc::DiagnosticoDpc, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(|| crate::modules::windows::dpc::medir(10))
            .await
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// (2.9, modo Expert). `LIVRES`.
#[tauri::command]
pub async fn msi_dispositivos() -> Result<Vec<crate::modules::windows::devices::DispositivoMsi>, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::devices::msi_por_dispositivo)
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: só lê o histórico.
#[tauri::command]
pub async fn exportar_alteracoes(state: State<'_, AppState>) -> Result<String, String> {
    let log = state.changes.lock().await;
    let csv = crate::modules::report::csv_das_alteracoes(&log, valor_novo_da_alteracao);
    crate::modules::report::salvar_csv(&csv)
}

#[cfg(target_os = "windows")]
fn valor_novo_da_alteracao(id: &str, c: &crate::modules::changelog::ChangeRecord) -> Option<String> {
    use crate::modules::changelog::ChangeRecord;
    use crate::modules::windows::catalog::{self, Action, RegValue};
    match c {
        ChangeRecord::RegistryValue { path, name, .. } => catalog::find(id)?.actions.iter().find_map(|a| match a {
            Action::Registry { path: p, name: n, value, .. } if p.eq_ignore_ascii_case(path) && n.eq_ignore_ascii_case(name) => Some(match value {
                RegValue::Dword(v) => v.to_string(),
                RegValue::Text(t) => t.to_string(),
                RegValue::Binary(b) => b.iter().map(|x| format!("{:02X}", x)).collect::<Vec<_>>().join(" "),
            }),
            _ => None,
        }),
        ChangeRecord::LimiteNvidia { fps, .. } => Some(format!("{} FPS", fps)),
        ChangeRecord::PerfilNvidia { perfil, .. } => Some(format!("perfil {}", perfil)),
        ChangeRecord::Hibernation { .. } => Some("desligada".into()),
        ChangeRecord::MemoryCompression { .. } => Some("desligada".into()),
        ChangeRecord::ReservedStorage { .. } => Some("desligado".into()),
        ChangeRecord::ScheduledTask { previously_enabled, .. } => {
            Some(if *previously_enabled { "desligada" } else { "ligada" }.into())
        }
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn valor_novo_da_alteracao(_: &str, _: &crate::modules::changelog::ChangeRecord) -> Option<String> {
    None
}

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

/// Em notebook com duas placas é o maior ganho que o produto dá, sem administrador nem reinício.
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

/// Só leitura. Sem placa NVIDIA, a frase que diz isso.
#[tauri::command]
pub async fn ajustes_do_driver_nvidia(state: State<'_, AppState>) -> Result<PainelDoDriver, String> {
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
                    entrada.changes.iter().filter_map(move |mudanca| match mudanca {
                        ChangeRecord::LimiteNvidia { executavel, fps, .. } => Some(LimiteNaTela {
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
            crate::modules::windows::nvdriver::painel(|id| aplicados.iter().any(|a| a == id), limites)
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

/// `EXIGEM_LICENCA`. O desfazer é o `revert_optimization`.
#[tauri::command]
pub async fn limitar_fps_nvidia(
    executavel: String,
    fps: u32,
    state: State<'_, AppState>,
) -> Result<OptimizationOutcome, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::modules::windows::WindowsOptimizer::new().limitar_fps_nvidia(&executavel, fps, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (executavel, fps, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

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
pub fn running_game_executable() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // O detector por sinais primeiro: a lista de nomes devolvia `FiveM_ChromeBrowser` (subprocesso do navegador) em
        // vez do jogo, por ordem de enumeração. A lista fica de reserva para o jogo aberto fora do primeiro plano.
        crate::modules::windows::deteccao::procurar()
            .map(|jogo| jogo.executavel)
            .or_else(crate::modules::windows::gamemode::executavel_do_jogo)
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
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
        crate::modules::windows::WindowsOptimizer::new()
            .set_persistent_priority(&executable, enable, &mut log)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (executable, enable, state);
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

#[tauri::command]
pub async fn analyze_network() -> Result<NetworkReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::network::analyze)
            .await
            .map_err(|e| format!("Falha ao medir a rede: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

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

/// O alvo sai das conexões do jogo (`rede::servidor_do_jogo`); não descobrir é o resultado. `LIVRES`: é o
/// diagnóstico que evita culpar o produto por travada de rede.
#[tauri::command]
pub async fn medir_perda_de_pacote() -> Result<crate::modules::windows::rede::MedidaDeRede, String>
{
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::rede::medir_agora)
            .await
            .map_err(|e| format!("Falha ao medir a perda de pacote: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
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

/// `LIVRES`: saber o que a máquina suporta não altera nada.
#[tauri::command]
pub async fn framegen_detectar(processo: Option<String>) -> Result<Deteccao, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || crate::modules::windows::framegen::detectar(processo.as_deref()))
            .await
            .map_err(|e| format!("Falha ao detectar a geração de quadros: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = processo;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// De fora, como `measure_frames`; quem liga a geração é a pessoa. `LIVRES`.
#[tauri::command]
pub async fn framegen_medir(
    processo: String,
    segundos: u64,
    tecnologia: Option<Tecnologia>,
    multiplicador: u8,
    hz: u32,
) -> Result<ResultadoDaRodada, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            crate::modules::windows::framegen::medir_rodada(
                &processo,
                segundos.clamp(5, 60),
                tecnologia,
                multiplicador,
                hz,
            )
        })
        .await
        .map_err(|e| format!("Falha ao medir a rodada: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (processo, segundos, tecnologia, multiplicador, hz);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command]
pub fn framegen_comparar(
    desligado: Rodada,
    ligado: Rodada,
    perfil: Perfil,
    hz: u32,
    artefatos: Vec<(Artefato, Intensidade)>,
    deriva_pct: Option<f64>,
) -> FramegenComparacao {
    use crate::modules::windows::framegen;

    let nota = (!artefatos.is_empty()).then(|| framegen::avaliar_artefatos(&artefatos));
    FramegenComparacao {
        comparacao: framegen::comparar(&desligado, &ligado, perfil, hz, nota.map(|n| n.nivel), deriva_pct),
        artefatos: nota,
    }
}

/// `EXIGEM_LICENCA` por ser recurso do produto; nada é gravado, e desligar some com a sobreposição.
#[tauri::command]
pub async fn gerador_ligar(
    processo: String,
    multiplicador: u8,
) -> Result<crate::modules::windows::geracao::Estado, String> {
    crate::modules::licenca::exigir()?;

    tokio::task::spawn_blocking(move || {
        crate::modules::windows::geracao::ligar(crate::modules::windows::geracao::Configuracao {
            processo,
            multiplicador,
            visivel_em_captura: false,
        })
    })
    .await
    .map_err(|e| format!("Falha ao ligar o gerador: {}", e))?
}

#[tauri::command]
pub async fn gerador_desligar() -> crate::modules::windows::geracao::Estado {
    let _ = tokio::task::spawn_blocking(crate::modules::windows::geracao::desligar).await;
    crate::modules::windows::geracao::estado()
}

#[tauri::command]
pub fn gerador_estado() -> crate::modules::windows::geracao::Estado {
    crate::modules::windows::geracao::estado()
}

#[tauri::command]
pub fn framegen_melhor(desligado: Rodada, testadas: Vec<Rodada>, perfil: Perfil, hz: u32) -> Option<usize> {
    crate::modules::windows::framegen::melhor_rodada(&desligado, &testadas, perfil, hz)
}

#[derive(serde::Serialize)]
pub struct FramegenComparacao {
    pub comparacao: crate::modules::windows::framegen::Comparacao,
    pub artefatos: Option<crate::modules::windows::framegen::NotaDeArtefatos>,
}

#[derive(serde::Serialize)]
pub struct PainelDeEnergia {
    pub impressao: crate::modules::windows::motorenergia::Impressao,
    pub bateria: crate::modules::windows::motorenergia_maquina::Bateria,
    pub ppm_atual: Vec<crate::modules::windows::motorenergia::Configuracao>,
    pub plano_otimiza_ativo: bool,
    pub backup: Option<crate::modules::windows::motorenergia_maquina::Backup>,
    pub perfis_de_jogo: Vec<crate::modules::windows::motorenergia_maquina::PerfilDeJogo>,
    pub dinamico: bool,
    pub elevado: bool,
    /// Normalmente por falta de administrador.
    pub teste_interrompido: bool,
}

#[tauri::command]
pub async fn energia_painel() -> Result<PainelDeEnergia, String> {
    tokio::task::spawn_blocking(|| {
        use crate::modules::windows::{motorenergia as motor, motorenergia_maquina as maquina, planoenergia};

        let impressao = maquina::impressao();
        let base = maquina::enumerar_base()?;
        let bateria = maquina::bateria_de_candidatos(&impressao, &base);
        let ativo = impressao.plano_ativo.clone().unwrap_or_default();
        let ppm_atual = maquina::enumerar_plano(&ativo)
            .map(|e| {
                e.configuracoes
                    .into_iter()
                    .filter(|c| c.alias.as_deref().is_some_and(|a| {
                        let sem_classe = a.trim_end_matches('1');
                        motor::apelidos::DA_TELA.iter().any(|t| t.eq_ignore_ascii_case(a) || t.eq_ignore_ascii_case(sem_classe))
                    }))
                    .collect()
            })
            .unwrap_or_default();
        let plano_otimiza_ativo = impressao.plano_ativo_nome.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(planoenergia::NOME_DO_PLANO));

        Ok(PainelDeEnergia {
            impressao,
            bateria,
            ppm_atual,
            plano_otimiza_ativo,
            backup: maquina::ler_backup(),
            perfis_de_jogo: maquina::perfis_de_jogo(),
            dinamico: maquina::dinamico().ligado,
            elevado: crate::modules::windows::registry::is_elevated(),
            teste_interrompido: maquina::teste_foi_interrompido(),
        })
    })
    .await
    .map_err(|e| format!("Falha ao ler a máquina: {}", e))?
}

/// `EXIGEM_LICENCA`: escreve no plano OTIMIZA; o do cliente não é tocado, e o backup vem antes.
#[tauri::command]
pub async fn energia_testar_candidato(
    candidato: crate::modules::windows::motorenergia::Candidato,
    processo: Option<String>,
    segundos: u64,
    repeticoes: u32,
) -> Result<crate::modules::windows::motorenergia_maquina::MedicaoDoCandidato, String> {
    crate::modules::licenca::exigir()?;

    tokio::task::spawn_blocking(move || {
        use crate::modules::windows::motorenergia_maquina as maquina;

        let impressao = maquina::impressao();
        let base = maquina::enumerar_base()?;
        let mut controlados = maquina::bateria_de_candidatos(&impressao, &base).controlados;
        for m in &candidato.mudancas {
            if !controlados.contains(&m.alias) {
                controlados.push(m.alias.clone());
            }
        }
        maquina::marcar_teste_em_andamento(true);
        let aplicacao = maquina::aplicar(&candidato, &controlados)?;
        let mut medicao = maquina::medir_atual(&candidato.id, processo.as_deref(), segundos.clamp(10, 60), repeticoes)?;
        medicao.aplicacao = Some(aplicacao);
        Ok(medicao)
    })
    .await
    .map_err(|e| format!("Falha ao testar o candidato: {}", e))?
}

#[tauri::command]
pub async fn energia_medir_atual(
    processo: Option<String>,
    segundos: u64,
    repeticoes: u32,
) -> Result<crate::modules::windows::motorenergia_maquina::MedicaoDoCandidato, String> {
    tokio::task::spawn_blocking(move || {
        crate::modules::windows::motorenergia_maquina::medir_atual("atual", processo.as_deref(), segundos.clamp(10, 60), repeticoes)
    })
    .await
    .map_err(|e| format!("Falha ao medir: {}", e))?
}

#[tauri::command]
pub async fn tetos_escondidos() -> Result<crate::modules::windows::tetos::Relatorio, String> {
    tokio::task::spawn_blocking(crate::modules::windows::tetos::procurar)
        .await
        .map_err(|e| format!("Falha ao procurar limites: {}", e))
}
#[tauri::command]
pub async fn quedas_de_desempenho() -> Result<Vec<crate::modules::deriva::Deriva>, String> {
    tokio::task::spawn_blocking(|| crate::modules::medicoes::ler().map(|m| crate::modules::deriva::procurar(&m)))
        .await
        .map_err(|e| format!("Falha ao ler as medições: {}", e))?
}

#[tauri::command]
pub async fn pronto_para_jogar() -> Result<crate::modules::windows::prontojogo::Prontidao, String> {
    tokio::task::spawn_blocking(crate::modules::windows::prontojogo::verificar)
        .await
        .map_err(|e| format!("Falha na verificação: {}", e))
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
pub async fn diagnostico_ao_vivo(segundos: u64, state: State<'_, AppState>) -> Result<DiagnosticoAoVivo, String> {
    let segundos = segundos.clamp(5, 60);
    // Sem o piso o transbordo de VRAM não se distingue do normal.
    let piso = state.monitor.lock().await.piso_de_vram();
    tokio::task::spawn_blocking(move || diagnostico_na_janela(segundos, piso))
        .await
        .map_err(|e| format!("Falha no diagnóstico: {}", e))?
}

pub fn diagnostico_na_janela(segundos: u64, piso: crate::modules::vram::Piso) -> Result<DiagnosticoAoVivo, String> {
    {
        use crate::core::telemetria;

        let mut coletor = telemetria::Coletor::novo()
            .ok_or("Os contadores de desempenho do Windows não abriram nesta máquina.")?;
        let jogo = crate::modules::windows::gamemode::jogo_aberto_com_pid();
        coletor.acompanhar_processos(jogo.as_ref().map(|(_, pid)| *pid));

        // O instante de início amarra o relógio dos quadros ao das amostras.
        let inicio_dos_quadros = coletor.decorrido_ms();
        let medicao = jogo.clone().map(|(nome, pid)| {
            std::thread::spawn(move || crate::modules::windows::frames::medir_par(pid, &nome, None, segundos))
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

        let (saude, intervalos, quadros_erro) = match medicao.map(|h| h.join()) {
            None => (None, Vec::new(), None),
            Some(Ok(Ok((m, _)))) => (m.resumo.saude, m.intervalos_ms, None),
            Some(Ok(Err(e))) => (None, Vec::new(), Some(e)),
            Some(Err(_)) => (None, Vec::new(), Some("A medição de quadros parou no meio.".to_string())),
        };
        let vram_total = coletor.placa().map(|p| p.vram_total_mb);
        let travadas = (!intervalos.is_empty())
            .then(|| crate::core::travadas::investigar(&intervalos, &amostras, inicio_dos_quadros, vram_total));

        let ram_total_mb = {
            let mut s = sysinfo::System::new();
            s.refresh_memory();
            Some(s.total_memory() as f64 / 1_048_576.0)
        };
        let hz = crate::modules::windows::display::monitores().iter().find(|m| m.principal).map(|m| m.hz_atual);
        let agora = crate::modules::changelog::now_timestamp();
        let t = crate::core::janela::para_telemetria(&amostras, ram_total_mb, vram_total, saude.as_ref(), travadas.as_ref(), hz, agora);
        let gargalo = crate::modules::gargalo::classificar_com(&t, &crate::modules::vram::avaliar(&t, &piso));

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
#[tauri::command]
pub async fn energia_vizinhos(
    parametros: crate::modules::windows::motorenergia::Parametros,
) -> Result<Vec<crate::modules::windows::motorenergia::Candidato>, String> {
    tokio::task::spawn_blocking(move || {
        use crate::modules::windows::{motorenergia as motor, motorenergia_maquina as maquina};
        let impressao = maquina::impressao();
        let base = maquina::enumerar_base()?;
        Ok(motor::vizinhos_do_vencedor(parametros, &impressao)
            .into_iter()
            .enumerate()
            .map(|(i, p)| motor::gerar(&impressao, &base, &format!("refino-{}", i + 1), motor::Papel::Refino, p))
            .collect())
    })
    .await
    .map_err(|e| format!("Falha ao gerar o refino: {}", e))?
}

#[tauri::command]
pub fn energia_escolher(
    resultados: Vec<crate::modules::windows::motorenergia::ResultadoDoCandidato>,
    base: String,
) -> Option<crate::modules::windows::motorenergia::Escolha> {
    crate::modules::windows::motorenergia::escolher(&resultados, &base)
}

#[tauri::command]
pub async fn energia_aplicar(
    parametros: crate::modules::windows::motorenergia::Parametros,
) -> Result<crate::modules::windows::motorenergia_maquina::Aplicacao, String> {
    crate::modules::licenca::exigir()?;

    tokio::task::spawn_blocking(move || {
        let r = crate::modules::windows::motorenergia_maquina::aplicar_parametros("escolhido", parametros);
        if r.is_ok() {
            crate::modules::windows::motorenergia_maquina::marcar_teste_em_andamento(false);
        }
        r
    })
        .await
        .map_err(|e| format!("Falha ao aplicar: {}", e))?
}

/// `LIVRES`: desfazer nunca depende de licença válida.
#[tauri::command]
pub async fn energia_restaurar_anterior() -> Result<crate::modules::windows::motorenergia_maquina::Restauracao, String> {
    tokio::task::spawn_blocking(crate::modules::windows::motorenergia_maquina::restaurar_anterior)
        .await
        .map_err(|e| format!("Falha ao restaurar: {}", e))?
}

#[tauri::command]
pub async fn energia_restaurar_windows() -> Result<crate::modules::windows::motorenergia_maquina::Restauracao, String> {
    tokio::task::spawn_blocking(crate::modules::windows::motorenergia_maquina::restaurar_padrao_windows)
        .await
        .map_err(|e| format!("Falha ao restaurar: {}", e))?
}

#[tauri::command]
pub fn energia_salvar_perfil_de_jogo(
    executavel: String,
    parametros: crate::modules::windows::motorenergia::Parametros,
    escolha: Option<crate::modules::windows::motorenergia::Escolha>,
) -> Result<Vec<crate::modules::windows::motorenergia_maquina::PerfilDeJogo>, String> {
    crate::modules::windows::motorenergia_maquina::salvar_perfil_de_jogo(
        crate::modules::windows::motorenergia_maquina::PerfilDeJogo {
            executavel,
            parametros,
            escolha,
            quando: crate::modules::changelog::now_timestamp(),
        },
    )
}

#[tauri::command]
pub fn energia_remover_perfil_de_jogo(
    executavel: String,
) -> Result<Vec<crate::modules::windows::motorenergia_maquina::PerfilDeJogo>, String> {
    crate::modules::windows::motorenergia_maquina::remover_perfil_de_jogo(&executavel)
}

/// `EXIGEM_LICENCA`: ligado, troca o plano sozinho.
#[tauri::command]
pub fn energia_modo_dinamico(ligado: bool) -> Result<bool, String> {
    crate::modules::licenca::exigir()?;
    crate::modules::windows::motorenergia_maquina::definir_dinamico(ligado).map(|d| d.ligado)
}

/// SÓ LEITURA e não sugere aumentar nada (ver `modules::windows::citizenfx`).
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

#[tauri::command]
pub async fn analyze_fivem() -> Result<FiveMReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::fivem::analyze)
            .await
            .map_err(|e| format!("Falha ao ler a instalação do FiveM: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Recusa pasta protegida e jogo aberto. Sem volta.
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

#[tauri::command]
pub async fn analyze_browsers() -> Result<BrowserReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::browsers::analyze)
            .await
            .map_err(|e| format!("Falha ao ler os navegadores: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Sem volta, recusa com o navegador aberto; dado de aplicativo nunca é tocado.
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

#[tauri::command]
pub async fn analyze_thermal() -> Result<ThermalReport, String> {
    #[cfg(target_os = "windows")]
    {
        // WMI passa de um segundo: fora do runtime. O processador e a placa no mesmo diagnóstico (2.9).
        tokio::task::spawn_blocking(|| {
            let mut r = crate::modules::windows::thermal::analyze();
            r.placa = Some(crate::modules::windows::sensoresgpu::ler());
            r
        })
        .await
        .map_err(|e| format!("Falha ao medir o processador: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command]
pub async fn analyze_health() -> Result<HealthReport, String> {
    #[cfg(target_os = "windows")]
    {
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

#[tauri::command]
pub async fn analyze_conflicts() -> Result<ConflictReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::conflicts::analyze)
            .await
            .map_err(|e| format!("Falha ao procurar conflitos: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[tauri::command]
pub async fn list_scheduled_tasks() -> Result<Vec<ScheduledTask>, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::tasks::listar_de_terceiros)
            .await
            .map_err(|e| format!("Falha ao listar tarefas: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Id próprio no histórico: "Desfazer tudo" alcança.
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

#[tauri::command]
pub async fn list_third_party_services() -> Result<Vec<ServiceEntry>, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::servicesaudit::listar_de_terceiros)
            .await
            .map_err(|e| format!("Falha ao listar serviços: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

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

#[tauri::command]
pub async fn analyze_bloatware() -> Result<BloatReport, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(crate::modules::windows::bloatware::analyze)
            .await
            .map_err(|e| format!("Falha ao examinar programas: {}", e))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Só apps da Loja (voltam pela Loja); programa comum vai pela tela oficial do Windows.
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

/// O desinstalador do fabricante faz perguntas: imitá-lo arriscaria instalação pela metade.
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

/// Pode levar dezenas de segundos.
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

#[tauri::command]
pub fn list_startup() -> Result<Vec<StartupEntry>, String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::startup::entries()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Escreve onde o Gerenciador de Tarefas escreve, com o anterior no histórico.
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

        state.processes.lock().await.refresh_startup();

        Ok(outcome)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (hive, name, enabled, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Não escreve na BIOS. ~12 s de carga sustentada: `spawn_blocking`.
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

/// Um processo não se eleva sozinho: abre outro e o Windows pergunta; recusado, nada acontece. Na versão final a
/// mensagem fica vazia, porque este processo encerra.
#[tauri::command]
pub fn relaunch_as_admin(app: tauri::AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if crate::modules::windows::registry::is_elevated() {
            return Ok(String::new());
        }

        let executable = std::env::current_exe()
            .map_err(|e| format!("Não foi possível localizar o programa: {}", e))?;

        // Aspas simples escapadas dobrando, regra do PowerShell.
        let path = executable.to_string_lossy().replace('\'', "''");
        let script = format!("Start-Process -FilePath '{}' -Verb RunAs", path);

        // `Start-Process -Verb RunAs` só volta quando a pessoa responde ao aviso.
        crate::modules::windows::shell::run_checked_com_prazo(
            "powershell",
            &["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script],
            std::time::Duration::from_secs(600),
        )
        .map_err(|_| {
            "Você recusou a permissão de administrador. Nada foi alterado.".to_string()
        })?;

        // Em desenvolvimento, encerrar derrubaria o servidor do Vite hospedado pelo `tauri dev`: esconde a janela.
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

        // Na versão final, dois abertos disputariam o mesmo arquivo de histórico.
        app.exit(0);
        Ok(String::new())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// O convite embutido vem da tela (é o único em `main.ts`): duplicá-lo aqui faria dois valores discordarem.
/// `LIVRES`: quem não ativou é quem mais precisa do suporte.
#[tauri::command]
pub async fn convite_do_discord(embutido: String) -> Result<String, String> {
    Ok(crate::modules::convite::consultar()
        .await
        .unwrap_or(embutido))
}

/// Vazio por não haver nada e vazio por não saber são diferentes. `LIVRES`.
#[tauri::command]
pub async fn estado_do_historico(
    state: State<'_, AppState>,
) -> Result<crate::modules::changelog::LeituraDoHistorico, String> {
    let log = state.changes.lock().await;

    Ok(log.leitura().clone())
}

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

/// Sem a lista, o cliente lia "seu monitor está em 60 Hz" sem saber QUAL. `LIVRES`.
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

/// `LIVRES`.
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

#[derive(serde::Serialize)]
pub struct PlacaDeVideo {
    pub marca: String,
    pub nome: Option<String>,
    pub driver: Option<String>,
    pub driver_data: Option<String>,
    pub driver_dias: Option<i64>,
    pub driver_origem: Option<String>,
    pub driver_generico: bool,
    pub vram_gb: f64,
}

/// Pelo NOME, o mesmo texto do Gerenciador de Dispositivos: quando erra, o cliente percebe. Na dúvida,
/// `desconhecida`.
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

/// O nome de `shaders`, a memória de `bottleneck` (o WMI satura em 4 GB). `LIVRES`.
#[tauri::command]
pub async fn placa_de_video() -> Result<PlacaDeVideo, String> {
    #[cfg(target_os = "windows")]
    {
        let s = crate::modules::windows::shaders::analyze();
        let nome = s.gpu.clone();
        let origem = crate::modules::windows::shaders::provedor_do_driver();

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
            driver_generico: origem
                .as_deref()
                .is_some_and(|p| crate::modules::windows::shaders::driver_generico(p, s.gpu.as_deref())),
            driver_origem: origem,
            vram_gb: crate::modules::windows::bottleneck::vram_total_gb(),
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: descobrir de graça que o MSAA custa 40% é o que faz querer a chave.
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

/// Jogo ABERTO (o contrário de `apply_game_profile`): mede-se o que roda, escreve-se no de quem não roda. `LIVRES`.
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

#[tauri::command]
pub fn prova_guardada() -> Option<crate::modules::prova::Prova> {
    crate::modules::prova::guardada()
}

/// `Err` quando existe e não se lê.
#[tauri::command]
pub fn medicoes_automaticas() -> Result<Vec<crate::modules::medicoes::MedicaoAutomatica>, String> {
    crate::modules::medicoes::ler()
}

/// Vinte e um ajustes são por CONTA: elevado por outra, iriam para um perfil que ninguém usa. `LIVRES`.
#[tauri::command]
pub fn conta_que_esta_rodando() -> ContaDoUsuario {
    #[cfg(target_os = "windows")]
    {
        let conta = crate::modules::windows::contadousuario::verificar();
        let quantos = crate::modules::windows::contadousuario::quantos_sao_por_conta();

        ContaDoUsuario {
            explicacao: crate::modules::windows::contadousuario::explicar(&conta, quantos),
            ajustes_por_conta: quantos,
            conta,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        ContaDoUsuario::default()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContaDoUsuario {
    #[cfg(target_os = "windows")]
    pub conta: crate::modules::windows::contadousuario::Conta,
    pub ajustes_por_conta: usize,
    pub explicacao: String,
}

#[cfg(not(target_os = "windows"))]
impl Default for ContaDoUsuario {
    fn default() -> Self {
        ContaDoUsuario {
            ajustes_por_conta: 0,
            explicacao: "Só no Windows.".to_string(),
        }
    }
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

/// `LIVRES`. 60% no 1% pior e 40% na média: ninguém sente média.
#[tauri::command]
pub fn nota_do_jogo() -> Result<crate::modules::pontuacao::Nota, String> {
    let medicoes = crate::modules::medicoes::ler()?;

    // A mais recente CONFIÁVEL: amostra curta faria a nota piscar.
    let Some(m) = medicoes.iter().rev().find(|m| m.confiavel) else {
        return Ok(crate::modules::pontuacao::Nota::SemAmostra);
    };

    Ok(crate::modules::pontuacao::calcular(m.fps, m.low_1pct, m.confiavel))
}

/// `LIVRES`. Aplicar é `optimize_now`; desfazer é `revert`. Por jogo: FPS de jogos diferentes não se compara;
/// sem jogo, o mais medido.
#[tauri::command]
pub async fn protocolo_de_grupos(
    state: State<'_, AppState>,
    jogo: Option<String>,
) -> Result<Vec<crate::modules::windows::experimento::Experimento>, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::{experimento, grupos::Grupo};

        let medicoes = crate::modules::medicoes::ler()?;

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
                let aplicado =
                    !itens.is_empty() && itens.iter().all(|id| log.is_applied(id));

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

/// `LIVRES`. O cliente compara com vídeos de "50 tweaks" sem saber que metade não faz nada.
#[tauri::command]
pub fn o_que_nao_fazemos() -> Vec<crate::modules::windows::naofazemos::NaoFazemos> {
    crate::modules::windows::naofazemos::LISTA.to_vec()
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn o_que_o_otimiza_altera() -> Vec<crate::modules::windows::registro::Alteracao> {
    crate::modules::windows::registro::todas()
}

/// `LIVRES`. Com a regressão medida: se o Otimiza pode ser a causa, é o PRIMEIRO suspeito.
#[tauri::command]
pub async fn por_que_o_fps_esta_baixo(
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::causas::Investigacao, String> {
    #[cfg(target_os = "windows")]
    {
        // Falhar a regressão não impede a investigação.
        let piorou = match crate::modules::medicoes::ler() {
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

/// `LIVRES`: com SSD pequeno e HD grande o jogo quase sempre está no HD.
#[tauri::command]
pub fn onde_os_jogos_moram(
) -> Result<crate::modules::windows::discodojogo::Relatorio, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::discodojogo::analisar())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Só no Windows.".to_string())
    }
}

/// Antes contra depois, pelo histórico do vigia (na 2.1.0 os dois números estavam no disco e ninguém comparou).
/// `Err` quando não se lê: lista vazia afirmaria que nenhum jogo piorou.
#[tauri::command]
pub async fn conferir_o_proprio_trabalho(
    state: State<'_, AppState>,
) -> Result<Vec<crate::modules::regressao::Veredito>, String> {
    let medicoes = crate::modules::medicoes::ler()?;
    let aplicadas = state.changes.lock().await.applied().len();

    Ok(crate::modules::regressao::todos(&medicoes, aplicadas))
}

/// `LIVRES`: chave por chave antes de decidir.
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

/// A única escrita num arquivo do cliente: guarda o arquivo INTEIRO no histórico. `EXIGEM_LICENCA`.
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

        // Nada mudou, nada é registrado.
        if feito.mudou.is_empty() {
            return Ok(feito.mudou);
        }

        let mut log = state.changes.lock().await;

        // Nunca menos FPS: em observação (`modules::portao`). FiveM é "FiveM_b3258_GTAProcess.exe"; GTA, "GTA5.exe".
        let processo = if feito.jogo.to_lowercase().contains("fivem") { "fivem_" } else { "gta5" };
        crate::modules::portao::vigiar(&format!("config_jogo_{}", perfil), &feito.jogo, processo, now_timestamp());

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

#[tauri::command]
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
) -> Result<Vec<OptimizationOutcome>, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;

        // Rede de segurança antes de mudar: falhar não bloqueia, mas o cliente sabe.
        if Preferences::load().restore_point_before_batch {
            let inicio = std::time::Instant::now();
            crate::utils::Logger::info("ponto de restauração antes do lote: começou");

            let (message, success) = match crate::modules::windows::restore::create(
                "Otimiza - antes de otimizar",
            ) {
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
        let mut resultados = crate::modules::windows::WindowsOptimizer::new().apply_selection(
            only.as_deref(),
            &mut log,
            |step| {
                let _ = app.emit("optimize:step", step);
            },
        );

        // O teto de FPS do jogo no mesmo clique: a configuração do jogo move FPS em dezenas, e quem aperta o botão não
        // vai até a aba Jogos. SÓ o `SemTeto` (VSync e limite, preço visual zero); os que mudam a cara do jogo são
        // escolha do dono. Com `only` de um perfil, nada é acrescentado.
        if only.is_none() {
            if let Some(resultado) = aplicar_teto_do_jogo(&app, &mut log).await {
                resultados.push(resultado);
            }
        }

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
    use crate::modules::changelog::{AppliedOptimization, ChangeRecord, now_timestamp};
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
                    format!("{} — mas o histórico não pôde ser gravado: {}", resumo, erro),
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

/// Desfaz todas as otimizações aplicadas.
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

/// `id` porque é o nome que o botão do diagnóstico manda; aqui é o dispositivo (`\.\DISPLAY1`).
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

// Os quatro comandos de reparo levam o atributo `async` do Tauri. Sem ele o Tauri os classifica como Blocking
// (tauri-macros-2.6.3, `src/command/wrapper.rs`) e o corpo roda na thread do laço de eventos: um DISM de trinta
// minutos congela a janela e o `reparo_cancelar` só executaria depois do fim. Numa função SÍNCRONA com
// `State<'_, AppState>` o atributo é válido (`sync_threadpool`). (A marca não aparece escrita aqui: a guarda do fim
// do arquivo conta comandos por ela no fonte.)

/// Só no Windows: `HealthReport` e `DiscoSaudavel` vêm de `modules::windows`.
#[cfg(target_os = "windows")]
fn disco_saudavel_agora() -> crate::modules::windows::reparo::DiscoSaudavel {
    let relatorio = crate::modules::windows::health::analyze();
    crate::modules::windows::reparo::DiscoSaudavel::a_partir_do_relatorio(&relatorio)
}

/// Lido de `Receita`, nunca reescrito na tela. Sem `programa` nem `args`: detalhe de execução, e mais superfície
/// para forjar numa chamada direta.
#[derive(Serialize)]
pub struct FerramentaDeReparo {
    pub nome: String,
    pub minutos_tipicos: (u32, u32),
    pub cancelar_e_seguro: bool,
    pub aviso: Option<String>,
    pub oferece_reset_base: bool,
    /// Da `Receita` com `resetar_base: true`: fonte única do aviso.
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

/// `LIVRES`. O disco NÃO vem da tela: o comando lê o `HealthReport` e monta o `DiscoSaudavel`; um `bool` do
/// frontend reabriria o buraco.
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

        // DUAS PROVAS: `DiscoSaudavel` (aguenta) e `EstadoDoDisco` (houve medição). Faltando uma, o botão não existe.
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

/// Tranca envenenada devolve `SemVerificacao`, que não autoriza nada.
#[cfg(target_os = "windows")]
fn estado_do_disco(state: &State<'_, AppState>) -> crate::modules::windows::reparo::EstadoDoDisco {
    state.disco.lock().map(|d| *d).unwrap_or_default()
}

/// Da `ResultadoSfc::severidade()`: a tela escolhia a cor com `startsWith("Corrigiu ")` e pintava
/// `CorrigiuEmParte` de verde.
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

/// Da variante de `Desfecho`, antes da frase: a tela comparava `desfecho === "Terminou."`.
#[cfg(target_os = "windows")]
impl From<&crate::modules::windows::tarefa_longa::Desfecho> for TomResultado {
    fn from(d: &crate::modules::windows::tarefa_longa::Desfecho) -> Self {
        use crate::modules::windows::tarefa_longa::Desfecho;

        match d {
            Desfecho::Terminou { codigo: 0 } => TomResultado::Ok,
            Desfecho::Terminou { codigo: _ } => TomResultado::Erro,
            // Cancelar foi escolha: nem verde nem vermelho.
            Desfecho::Cancelada => TomResultado::Atencao,
            Desfecho::NaoComecou { .. } => TomResultado::Erro,
        }
    }
}

/// A tela lê `tom`, nunca a prosa de `texto`.
#[derive(Serialize)]
pub struct DesfechoReparo {
    pub tom: TomResultado,
    pub texto: String,
}

/// A tela lê `tom`, nunca a prosa de `texto`.
#[derive(Serialize)]
pub struct UltimoResultadoReparo {
    pub tom: TomResultado,
    pub texto: String,
}

/// `LIVRES`.
#[tauri::command(async)]
pub fn reparo_ultimo_resultado() -> UltimoResultadoReparo {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::cbslog::{self, ResultadoSfc};

        // Sem `unwrap_or_default()`: o CBS.log só abre como administrador, e a string vazia virava "o Windows não
        // verificou" em vez de "não conseguimos ler".
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
            // O resultado mais comum, e BOM.
            ResultadoSfc::SemCorrupcao => "Nenhuma corrupção encontrada.".into(),
            ResultadoSfc::Corrigiu { quantos } => {
                format!("Corrigiu {} arquivo(s) corrompido(s).", quantos)
            }
            // Misto NÃO é sucesso: o tom `Atencao` carrega isso.
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
            // Linhas de falha sem nome legível: `Atencao`, não `Ok`.
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

/// Exige licença. O parâmetro é `resetbase`, uma palavra só: o `/ResetBase` é caro demais (sem volta) para depender
/// da conversão camelCase/snake_case do Tauri.
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
                // As duas travas conferidas de novo aqui: a tela pode ser contornada, esta chamada não.
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

        // Reincluir antes: um `chkntfs /X` antigo deixa o volume fora do boot check para sempre, e o `fsutil dirty set`
        // sairia 0 com o `autochk` pulando o volume. Quem sequencia é este executor; rodar sempre é seguro.
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

            // Seguir para o `fsutil` com a reinclusão falha recriaria a mentira: "agendado" sem boot check.
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

        // Do desfecho real: `/scan` cancelado ou "não consegui verificar" apaga a autorização.
        if let Ok(mut atual) = state.disco.lock() {
            *atual = atual.apos_execucao(&escolhida, &desfecho);
        }

        // O tom é lido da VARIANTE antes do `match` consumir `desfecho`.
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

/// `LIVRES` como o `revert`: licença vencida no meio de um DISM não pode prender o cliente nele.
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

/// `LIVRES` (faltou quando um cliente precisou de script de PowerShell à mão; ver `modules::windows::suporte`). NÃO
/// síncrono: `health::analyze()` e `thermal::analyze()` somam 12 a 15 s, e na thread principal congelariam a
/// janela.
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

/// A tela decide só por `comparacao`, nunca por `versao_publicada` (texto para exibir).
#[derive(Debug, Clone, Serialize)]
pub struct AvisoDeVersao {
    pub comparacao: crate::modules::atualizacao::Comparacao,
    pub versao_publicada: Option<String>,
    pub pagina: Option<String>,
}

/// `LIVRES`: só pergunta ao GitHub; não instala nada. A versão instalada é `CARGO_PKG_VERSION`.
#[tauri::command]
pub async fn versao_mais_nova() -> AvisoDeVersao {
    let instalada = env!("CARGO_PKG_VERSION");

    match crate::modules::atualizacao::consultar_ultima().await {
        Some(ultima) => AvisoDeVersao {
            comparacao: crate::modules::atualizacao::comparar(instalada, &ultima.versao),
            versao_publicada: Some(ultima.versao),
            pagina: ultima.pagina,
        },
        // Falha de rede é silêncio: `NaoSei` não mostra nada.
        None => AvisoDeVersao {
            comparacao: crate::modules::atualizacao::Comparacao::NaoSei,
            versao_publicada: None,
            pagina: None,
        },
    }
}

/// Livre: alimenta a tela de compra.
#[tauri::command]
pub fn licenca_estado() -> crate::modules::licenca::Estado {
    crate::modules::licenca::estado()
}

#[tauri::command]
pub fn licenca_ativar(chave: String) -> Result<crate::modules::licenca::Estado, String> {
    crate::modules::licenca::ativar(&chave)?;
    Ok(crate::modules::licenca::estado())
}

/// `LIVRES`: responde "por que não funcionou?" antes de pagar.
#[tauri::command]
pub async fn diagnostico_de_energia() -> Result<
    crate::modules::windows::planoenergia::Diagnostico,
    String,
> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::planoenergia::diagnosticar())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: quem não comprou é quem precisa ver.
#[tauri::command]
pub async fn simular_plano_otimiza() -> Result<
    crate::modules::windows::planoenergia::RelatorioDoPlano,
    String,
> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::planoenergia::montar(true)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `EXIGEM_LICENCA`. UMA linha no histórico: o plano do cliente não é tocado, desfazer é reativar o anterior.
#[tauri::command]
pub async fn aplicar_plano_otimiza(
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::planoenergia::RelatorioDoPlano, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        use crate::modules::changelog::{AppliedOptimization, ChangeRecord, now_timestamp};

        let relatorio =
            crate::modules::windows::planoenergia::montar(false)?;

        // Só registra se o plano ficou ativo: registro de troca que não houve não desfaz nada.
        if relatorio.plano_ativo {
            if let Some(anterior) = relatorio.guid_anterior.clone() {
                let mut log = state.changes.lock().await;

                log.record(AppliedOptimization {
                    optimization_id: "plano_otimiza".to_string(),
                    name: "Plano de energia OTIMIZA".to_string(),
                    timestamp: now_timestamp(),
                    changes: vec![ChangeRecord::PowerPlan {
                        previous_guid: anterior,
                    }],
                })?;
            }
        }

        Ok(relatorio)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: a pergunta de quem "aplicou e depois voltou tudo".
#[tauri::command]
pub async fn vistoriar_plano_otimiza() -> Result<
    crate::modules::windows::planoenergia::Vistoria,
    String,
> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::planoenergia::vistoriar())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `EXIGEM_LICENCA`. NÃO mexe no histórico: o anterior guardado continua certo, e uma linha nova daria dois
/// "desfazer" para uma troca só.
#[tauri::command]
pub async fn reparar_plano_otimiza() -> Result<crate::modules::windows::planoenergia::RelatorioDoPlano, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::planoenergia::reparar()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: é de quem ainda não comprou, ou reclamou, que vem o dado das máquinas que o projeto não tem.
#[tauri::command]
pub async fn relatorio_de_compatibilidade() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        crate::modules::windows::labcompat::gerar()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Comando novo sem classificação REPROVA O BUILD: a falha provável não é quebrar a assinatura, é esquecer a
/// linha da guarda.
#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "mede esta máquina por 6 s"]
    fn diagnostico_ao_vivo_nesta_maquina() {
        let d = super::diagnostico_na_janela(6, Default::default()).expect("diagnóstico");
        println!("placa {:?} jogo {:?} erro {:?}", d.placa, d.jogo, d.quadros_erro);
        println!("saude {:#?}", d.saude);
        println!("gargalo {:#?}", d.gargalo);
        println!("{}", serde_json::to_string(&d).unwrap().len());
        assert!(d.amostras.len() >= 10);
    }

    /// O botão genérico NUNCA muda a aparência do jogo: no lote, só o teto de quadros sai do arquivo. Trava de FORMA
    /// sobre o corpo da função.
    #[test]
    fn o_lote_automatico_so_tira_o_teto_do_jogo() {
        let fonte = include_str!("commands.rs");

        let Some(corpo) = fonte.split("async fn aplicar_teto_do_jogo").nth(1) else {
            panic!("não achei a função que aplica o perfil no lote");
        };

        // Até o fim da função: a próxima declaração de item no nível do arquivo.
        let corpo = corpo.split("\n/// ").next().unwrap_or(corpo);

        assert!(
            corpo.contains("Perfil::SemTeto"),
            "o lote deixou de usar o perfil sem custo visual"
        );

        for proibido in ["Perfil::Equilibrado", "Perfil::Competitivo"] {
            assert!(
                !corpo.contains(proibido),
                "o lote automático passou a aplicar `{}`, que muda a aparência do \
                 jogo sem o cliente ter escolhido",
                proibido
            );
        }

        assert!(
            corpo.contains("anterior"),
            "o lote parou de guardar o arquivo anterior do jogo"
        );
    }

    /// O nome que o orquestrador devolve é o que a tela manda para `apply_game_profile`: tabelas divergentes dariam
    /// "perfil desconhecido" depois de recomendar.
    #[cfg(target_os = "windows")]
    #[test]
    fn o_plano_fala_a_lingua_de_quem_aplica() {
        use crate::modules::orquestrador::Perfil;

        for p in Perfil::TODOS {
            super::perfil_por_nome(p.nome())
                .unwrap_or_else(|e| panic!("o perfil {:?} saiu do lugar: {e}", p));
        }
    }

    /// Rodam sem licença: leitura, medição e o desfazer (licença vencida não pode deixar o PC sem volta).
    const LIVRES: &[&str] = &[
        "zerar_modo_jogo",
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
        "capturar_baseline",
        "capturar_baseline_repetido",
        "protocolo_do_perfil",
        "laboratorio_de_streaming",
        "plano_de_renderizacao",
        "passo_do_autoajuste",
        "historico_de_desempenho",
        "caminho_do_mouse",
        "catalogo_de_programas",
        "medir_limpeza",
        "nucleos_da_maquina",
        "recuperacao_pendente",
        "descartar_pendencia",
        "concluir_recuperacao",
        "comparar_com_baseline",
        "start_monitoring",
        "stop_monitoring",
        "measure_baseline",
        "get_baseline",
        "measure_and_compare",
        "is_elevated",
        "diagnostico_de_energia",
        "simular_plano_otimiza",
        "relatorio_de_compatibilidade",
        "vistoriar_plano_otimiza",
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
        "framegen_detectar",
        "framegen_medir",
        "framegen_comparar",
        "framegen_melhor",
        "gerador_desligar",
        "gerador_estado",
        "energia_painel",
        "energia_vizinhos",
        "diagnostico_ao_vivo",
        "tetos_escondidos",
        "pronto_para_jogar",
        "energia_medir_atual",
        "energia_escolher",
        "energia_restaurar_anterior",
        "energia_restaurar_windows",
        "energia_salvar_perfil_de_jogo",
        "energia_remover_perfil_de_jogo",
        "analyze_fivem",
        "analyze_citizenfx",
        "analyze_browsers",
        "analyze_boot",
        "analyze_thermal",
        "export_report",
        "exportar_alteracoes",
        "salvar_ficha_da_bios",
        "msi_dispositivos",
        "diagnostico_dpc",
        "cpuset_resultados",
        "cpuset_esquecer",
        "map_folders",
        "analyze_rbar",
        "list_profiles",
        "list_third_party_services",
        "list_scheduled_tasks",
        "scan_disk_space",
        "analyze_memory",
        "restore_status",
        "list_startup",
        "list_optimizations",
        "estado_do_historico",
        "convite_do_discord",
        "revert_optimization",
        "revert_all_optimizations",
        "licenca_estado",
        "licenca_ativar",
        "reparo_disponivel",
        "reparo_ultimo_resultado",
        "reparo_cancelar",
        "relatorio_de_suporte",
        "versao_mais_nova",
        "checar_essenciais",
        "ajustes_do_driver_nvidia",
        "medicoes_automaticas",
        "conferir_o_proprio_trabalho",
        "onde_os_jogos_moram",
        "por_que_o_fps_esta_baixo",
        "o_que_nao_fazemos",
        "o_que_o_otimiza_altera",
        "protocolo_de_grupos",
        "nota_do_jogo",
        "conflitos_entre_ajustes",
        "conta_que_esta_rodando",
        "niveis_de_otimizacao",
        "passo_a_passo_da_bios",
        "ficha_da_bios",
        "abertura_pronta",
        "quedas_de_desempenho",
        "reiniciar_na_bios",
    ];

    /// Alteram o computador. Sem licença, recusam.
    const EXIGEM_LICENCA: &[&str] = &[
        "instalar_programa",
        "limpar_alvos",
        "prender_jogo_nos_nucleos",
        "gerador_ligar",
        "cpuset_testar",
        "energia_testar_candidato",
        "energia_aplicar",
        "energia_modo_dinamico",
        "clean_disk_category",
        "aplicar_plano_otimiza",
        "reparar_plano_otimiza",
        "set_automatic_pagefile",
        "clean_shader_cache",
        "set_gpu_preference",
        "fix_readiness",
        "set_persistent_priority",
        "set_game_mode",
        "set_dns",
        "flush_dns",
        "clean_fivem",
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
        "religar_essenciais",
        "aplicar_ajuste_nvidia",
        "limitar_fps_nvidia",
    ];

    /// Só a parte de produção: os testes contêm as palavras procuradas.
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

    fn comandos() -> Vec<&'static str> {
        let marca = concat!("#[tauri::", "command");

        producao()
            .split(marca)
            .skip(1)
            // A marca aceita argumento: os de reparo são `(async)`, e sem isto ficariam invisíveis à guarda.
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

    /// Chutar pintaria a placa de verde para quem tem Radeon.
    #[test]
    fn a_marca_da_placa_sai_do_nome() {
        for (nome, esperado) in [
            ("NVIDIA GeForce GTX 1650", "nvidia"),
            ("NVIDIA GeForce RTX 4070 Ti", "nvidia"),
            ("AMD Radeon RX 6600", "amd"),
            ("Radeon(TM) Graphics", "amd"),
            ("Intel(R) UHD Graphics 630", "intel"),
            ("Intel(R) Arc(TM) A750", "intel"),
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

            // A guarda é a PRIMEIRA coisa do corpo: depois já se mexeu no computador de quem não pagou.
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
        // Diagnóstico com licença esvaziaria a tela de compra.
        let fonte = producao();

        for nome in LIVRES {
            let Some(inicio) = fonte.find(&format!("fn {}(", nome)) else {
                continue;
            };

            // O fim é a primeira linha só com `}`: o arquivo é CRLF, e "\n}\n" nunca casaria.
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
        // Diagnóstico livre, correção paga.
        assert!(LIVRES.contains(&"reparo_disponivel"));
        assert!(LIVRES.contains(&"reparo_ultimo_resultado"));
        assert!(EXIGEM_LICENCA.contains(&"reparo_executar"));

        assert!(LIVRES.contains(&"reparo_cancelar"));
    }

    /// Com `decorations: false` a barra é nosso HTML chamando comandos do Tauri, e no Tauri 2 comando não declarado
    /// em `capabilities/default.json` falha CALADO (o duplo-clique usa `internal_toggle_maximize`, outra permissão).
    #[test]
    fn a_barra_da_janela_tem_todas_as_permissoes_que_usa() {
        let permissoes = include_str!("../capabilities/default.json");

        for comando in [
            "core:window:allow-close",
            "core:window:allow-minimize",
            "core:window:allow-toggle-maximize",
            "core:window:allow-start-dragging",
            "core:window:allow-internal-toggle-maximize",
            "core:window:allow-is-maximized",
        ] {
            assert!(
                permissoes.contains(comando),
                "a barra da janela usa `{}` e a permissão não está declarada;                  o botão vai falhar sem dizer nada",
                comando
            );
        }
    }

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

    /// Literais de string com o texto ANTES deles, onde mora o operador. Em `char`, não byte: há acento.
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

    /// Prosa do backend: tem espaço ou termina em pontuação. Rótulo que a tela inventou é uma palavra só.
    fn parece_prosa_do_backend(literal: &str) -> bool {
        if literal.trim().is_empty() {
            return false;
        }
        literal.contains(' ') || literal.trim_end().ends_with(['.', '!', '?'])
    }

    /// Igualdade, prefixo, substring, posição ou `case`: o repertório que já causou o defeito três vezes.
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

    /// Tira comentários antes (senão a guarda reprova por CITAÇÃO do antipadrão), com o mesmo estado de aspas de
    /// `literais_com_contexto` para não confundir `"https://"`. Mantém as quebras de linha para os números baterem.
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

    /// Compartilhada pela guarda real (`main.ts` do disco) e pelo teste com fontes sintéticas.
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
        // O defeito voltou três vezes (`Corrigiu` escondendo, `CorrigiuEmParte` verde por prefixo, desfecho por
        // igualdade): procura a FORMA, literal com cara de prosa comparado por operador de decisão.
        // `CARGO_MANIFEST_DIR` aponta sempre para `src-tauri`; o `main.ts` está em `../src/`.
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
        // Canário da própria guarda.
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

    /// Comentário citando o antipadrão não reprova; o mesmo texto como código dispara.
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

    #[test]
    fn remover_comentarios_preserva_a_contagem_de_linhas() {
        let fonte = "linha1\n/* bloco\nde duas\nlinhas */\nlinha5";
        let limpo = remover_comentarios(fonte);

        assert_eq!(limpo.lines().count(), fonte.lines().count());
    }

    /// Extrai PEÇAS do `main.ts`, transpila com o TypeScript do projeto e RODA no Node: a guarda por leitura já foi
    /// burlada com variável intermediária. Cada peça vai até o primeiro `}` na coluna 0; a marca inclui `async` quando
    /// existe. Dublês de `element`, `setStatus`, `text` e `invoke`.
    fn roda_a_tela(pecas: &[&str], retorno: &str, chamadas: &str) -> serde_json::Value {
        let raiz = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let main_ts = raiz.join("src").join("main.ts");
        let typescript = raiz.join("node_modules").join("typescript");
        assert!(
            typescript.is_dir(),
            "esta prova roda a tela de verdade e precisa do TypeScript do projeto: \
             rode `npm ci` em pc-optimizer ({:?} não existe)",
            typescript
        );

        // O `(` evita achar `renderFolderMap` antes de `renderFolder`.
        let modelo = r#"
const fs = require("fs");
const ts = require(process.argv[3]);

const fonte = fs.readFileSync(process.argv[2], "utf8");

function extrai(marca) {
  const depois = fonte.split(marca)[1];
  if (depois === undefined) throw new Error(marca + " nao existe no main.ts");
  const corpo = [];
  for (const linha of depois.split(/\r?\n/)) {
    corpo.push(linha);
    if (linha.startsWith("}")) return marca + corpo.join("\n") + "\n";
  }
  throw new Error(marca + " nao fecha em coluna 0");
}

const trecho = __PECAS__.map(extrai).join("\n");
const js = ts.transpileModule(trecho, { compilerOptions: { target: "ES2020" } }).outputText;

const escapeHtml = (v) =>
  String(v).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

// Dublês do DOM e do backend. `registro` é tudo que a tela tentou fazer.
const registro = { status: [], html: null, texto: {}, botao: { disabled: false } };
let resposta = null;
let erro = null;
const element = (id) => {
  if (id === "analyze-rbar") return registro.botao;
  return { set innerHTML(v) { registro.html = v; } };
};
const setStatus = (id, mensagem, tom) => registro.status.push({ id, mensagem, tom });
const text = (id, valor) => { registro.texto[id] = valor; };
const invoke = async () => { if (erro !== null) throw erro; return resposta; };

const render = new Function(
  "escapeHtml", "element", "setStatus", "text", "invoke",
  js + "\nreturn (__RETORNO__);"
)(escapeHtml, element, setStatus, text, invoke);

//__CHAMADAS__
"#;

        let lista = serde_json::to_string(pecas).expect("as peças viram um array JS");
        let laboratorio = modelo
            .replace("__PECAS__", &lista)
            .replace("__RETORNO__", retorno)
            .replace("//__CHAMADAS__", chamadas);

        let apelido: String = retorno
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        let script = std::env::temp_dir().join(format!("otimiza_tela_{}.cjs", apelido));
        std::fs::write(&script, laboratorio).expect("escreve o laboratório no temporário");

        let saida = std::process::Command::new("node")
            .arg(&script)
            .arg(&main_ts)
            .arg(&typescript)
            .output()
            .expect("esta prova roda a tela de verdade e precisa do Node no PATH");
        let _ = std::fs::remove_file(&script);

        assert!(
            saida.status.success(),
            "não deu para rodar `{}`: {}",
            retorno,
            String::from_utf8_lossy(&saida.stderr)
        );

        serde_json::from_slice(&saida.stdout).expect("o laboratório imprime JSON")
    }

    #[test]
    fn a_tela_do_mapa_decide_por_natureza_e_nao_por_texto() {
        // Exige `natureza.tipo`, não só `natureza`, que passaria aparecendo num comentário.
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("src")
            .join("main.ts");
        let ts = std::fs::read_to_string(&caminho).expect("main.ts");

        assert!(
            ts.contains("natureza.tipo"),
            "a tela do mapa precisa decidir pelo campo `natureza.tipo`"
        );
    }

    #[test]
    fn o_comando_do_rbar_esta_registrado() {
        // Comando fora do `generate_handler!` só falha na máquina do cliente.
        let lib = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("lib.rs"),
        )
        .expect("lib.rs");
        assert!(
            lib.contains("analyze_rbar"),
            "analyze_rbar fora do generate_handler!"
        );
    }

    /// As três linhas só diferem na `natureza`: qualquer diferença no HTML vem dela.
    #[test]
    fn a_tela_do_mapa_so_oferece_limpar_o_que_e_limpavel() {
        let telas = roda_a_tela(
            &["function renderFolder("],
            "renderFolder",
            r#"
const base = {
  name: "Steam",
  path: "C:\\Program Files (x86)\\Steam",
  bytes: 131000000000,
  formatted: "122.0 GB",
  percent: 40,
  explanation: "",
  partial: false,
};

console.log(
  JSON.stringify({
    podeLimpar: render(Object.assign({}, base, { natureza: { tipo: "PodeLimpar" } })),
    soOWindowsLimpa: render(Object.assign({}, base, { natureza: { tipo: "SoOWindowsLimpa" } })),
    seu: render(Object.assign({}, base, { natureza: { tipo: "Seu" } })),
    naoSei: render(Object.assign({}, base, { natureza: { tipo: "NaoSei" } })),
  })
);
"#,
        );

        let pode_limpar = telas["podeLimpar"].as_str().expect("html do limpável");
        let outro_lugar = telas["soOWindowsLimpa"]
            .as_str()
            .expect("html do que só o Windows limpa");
        let seu = telas["seu"].as_str().expect("html do arquivo do cliente");
        let nao_sei = telas["naoSei"].as_str().expect("html do ilegível");

        // Sobre a POSSIBILIDADE de limpar, não a tag `<button>`: `data-mapa-limpar` no `<article>` inteiro passou nos
        // testes. A marca é o que o ouvinte procura.
        for (nome, html) in [
            ("do arquivo do cliente", seu),
            ("da pasta que não deu para ler", nao_sei),
            ("da pasta que só o Windows limpa", outro_lugar),
        ] {
            assert!(
                !html.contains("<button"),
                "a linha {} ganhou botão:\n{}",
                nome,
                html
            );
            assert!(
                !html.contains("data-mapa-limpar"),
                "a linha {} carrega a marca `data-mapa-limpar`, que é o que o \
                 ouvinte do mapa procura — com ela no HTML a linha vira \
                 clicável para limpeza, tenha ou não uma tag `<button>`:\n{}",
                nome,
                html
            );
        }
        assert!(
            pode_limpar.contains("<button") && pode_limpar.contains("data-mapa-limpar"),
            "a pasta limpável perdeu o botão:\n{}",
            pode_limpar
        );

        // `SoOWindowsLimpa` não pode oferecer "Limpar no liberador".
        assert_ne!(
            outro_lugar, pode_limpar,
            "a pasta que o liberador não limpa sai igual à que ele limpa"
        );
        assert!(
            outro_lugar.contains("Limpeza de Disco"),
            "a linha que perdeu o botão precisa dizer ONDE a limpeza acontece, \
             senão o cliente só vê a recusa:\n{}",
            outro_lugar
        );
        assert!(
            !outro_lugar.contains("seu arquivo"),
            "sobra do sistema não é arquivo do cliente — esconderia 24 GB atrás \
             do rótulo errado:\n{}",
            outro_lugar
        );

        assert_ne!(
            seu, nao_sei,
            "a tela pinta IGUAL a pasta do cliente e a pasta que não deu para \
             ler — o cliente não tem como saber a diferença"
        );
        assert!(
            nao_sei.contains("não consegui ler"),
            "a pasta ilegível não diz que é ilegível:\n{}",
            nao_sei
        );
        assert!(
            seu.contains("Steam") && seu.contains("Program Files"),
            "a linha do cliente precisa mostrar o caminho, que é para o que ela \
             serve:\n{}",
            seu
        );
    }

    /// `NaoSei` não sai com o selo verde.
    #[test]
    fn a_tela_do_rbar_separa_os_quatro_estados() {
        let telas = roda_a_tela(
            &["const NA_TELA_DO_RBAR", "function renderRbar("],
            "renderRbar",
            r#"
const base = { modelo: "NVIDIA GeForce RTX 3060", nota: "nota do backend" };

console.log(
  JSON.stringify({
    ligado: render(Object.assign({}, base, { estado: "Ligado" })),
    desligadoESuportado: render(Object.assign({}, base, { estado: "DesligadoESuportado" })),
    desligadoSemSuporte: render(Object.assign({}, base, { estado: "DesligadoSemSuporte" })),
    naoSei: render(Object.assign({}, base, { estado: "NaoSei" })),
  })
);
"#,
        );

        let ligado = telas["ligado"].as_str().expect("html do ligado");
        let suportado = telas["desligadoESuportado"]
            .as_str()
            .expect("html do suportado");
        let sem_suporte = telas["desligadoSemSuporte"]
            .as_str()
            .expect("html do sem suporte");
        let nao_sei = telas["naoSei"].as_str().expect("html do não sei");

        let todos = [ligado, suportado, sem_suporte, nao_sei];
        for (i, a) in todos.iter().enumerate() {
            for b in todos.iter().skip(i + 1) {
                assert_ne!(a, b, "dois estados do rBAR saem idênticos na tela");
            }
        }

        assert!(
            ligado.contains(r#"data-severity="Ok""#),
            "rBAR ligado é assunto resolvido:\n{}",
            ligado
        );
        assert!(
            !suportado.contains(r#"data-severity="Ok""#),
            "rBAR desligado numa placa que aceita saiu como assunto resolvido:\n{}",
            suportado
        );

        assert!(
            !nao_sei.contains(r#"data-severity="Ok""#),
            "a placa que não deu para verificar saiu com o selo verde:\n{}",
            nao_sei
        );
        assert!(
            nao_sei.contains("não consegui verificar"),
            "a tela não diz que não conseguiu verificar:\n{}",
            nao_sei
        );

        assert!(
            sem_suporte.contains("não tem"),
            "a placa sem o recurso precisa dizer isso:\n{}",
            sem_suporte
        );
    }

    /// `analyzeRbar` decidia o tom por uma SEGUNDA tabela: `NaoSei` como "ok" pintaria de verde o "ainda não cobrimos
    /// placas AMD" com a suíte passando. Roda o `analyzeRbar` real e olha o `setStatus`.
    #[test]
    fn a_faixa_de_status_do_rbar_nao_pinta_de_verde_o_que_ninguem_mediu() {
        let faixas = roda_a_tela(
            &[
                "const NA_TELA_DO_RBAR",
                "function tomDoRbar(",
                "function renderRbar(",
                "async function analyzeRbar(",
            ],
            "analyzeRbar",
            r#"
const base = { modelo: "AMD Radeon RX 6600", nota: "nota do backend" };

async function tom(estado) {
  registro.status.length = 0;
  resposta = Object.assign({}, base, { estado: estado });
  await render();
  // A faixa recebe duas mensagens: "Lendo a placa de vídeo…" no começo e o
  // veredito no fim. O que o cliente fica olhando é a última.
  const ultima = registro.status[registro.status.length - 1];
  return { tom: ultima.tom, mensagem: ultima.mensagem, card: registro.html };
}

(async () => {
  console.log(
    JSON.stringify({
      ligado: await tom("Ligado"),
      desligadoESuportado: await tom("DesligadoESuportado"),
      desligadoSemSuporte: await tom("DesligadoSemSuporte"),
      naoSei: await tom("NaoSei"),
    })
  );
})();
"#,
        );

        let tom_de = |chave: &str| -> String {
            faixas[chave]["tom"]
                .as_str()
                .unwrap_or_else(|| panic!("a faixa de `{}` não recebeu tom", chave))
                .to_string()
        };

        assert_ne!(
            tom_de("naoSei"),
            "ok",
            "a faixa de status pintou de verde uma placa que ninguém conseguiu \
             verificar — o cliente lê \"não consegui verificar\" em cor de \
             assunto resolvido"
        );

        assert_ne!(
            tom_de("desligadoESuportado"),
            "ok",
            "a faixa deu por resolvido um rBAR desligado numa placa que aceita"
        );

        // Senão o teste passaria com a faixa sempre amarela.
        assert_eq!(
            tom_de("ligado"),
            "ok",
            "rBAR ligado é assunto resolvido e a faixa precisa dizer isso"
        );
        assert_eq!(
            tom_de("desligadoSemSuporte"),
            "ok",
            "esta placa não tem o recurso: não há nada pendente para o cliente"
        );

        // Faixa e card da MESMA tabela, conferidos par a par.
        for chave in [
            "ligado",
            "desligadoESuportado",
            "desligadoSemSuporte",
            "naoSei",
        ] {
            let card_verde = faixas[chave]["card"]
                .as_str()
                .unwrap_or_else(|| panic!("`{}` não renderizou card", chave))
                .contains(r#"data-severity="Ok""#);
            assert_eq!(
                card_verde,
                tom_de(chave) == "ok",
                "em `{}` o card e a faixa de status discordam de cor — quem olha \
                 só a cor está olhando a faixa",
                chave
            );
        }

        assert_eq!(
            faixas["naoSei"]["mensagem"].as_str(),
            Some("nota do backend"),
            "a faixa parou de mostrar a nota que o backend escreveu"
        );
    }
}
