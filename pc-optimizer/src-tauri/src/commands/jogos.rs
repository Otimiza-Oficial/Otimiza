// Jogos: o jogo aberto, a biblioteca, capas, FiveM/CitizenFX, crashes e o perfil de configuração do jogo,
// inclusive o pedido de perfil feito pelo Turbo (que aplica pelo mesmo `aplicar_perfil_do_jogo`).

use super::*;

#[tauri::command(async)]
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

/// O jogo aberto agora, se houver (o evento `jogo:sessao` avisa quando muda). `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn jogo_aberto() -> Result<Option<crate::plataforma::sessao::JogoAberto>, String> {
    Ok(crate::plataforma::sessao::atual())
}

/// Os jogos que o Otimiza conhece pelo nome, com o que cada um aceita. `LIVRES`.
#[tauri::command(async)]
pub fn catalogo_de_jogos() -> Result<Vec<crate::plataforma::jogos::PerfilDoJogo>, String> {
    Ok(crate::plataforma::jogos::CATALOGO.to_vec())
}

/// Os jogos instalados neste PC (Steam, Epic e instaladores de jogo), para a prateleira da aba Jogos. `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn jogos_instalados() -> Vec<crate::modules::windows::jogos::JogoDaLoja> {
    crate::modules::windows::jogos::para_a_tela(&crate::modules::windows::jogos::varrer())
}

/// "Jogar": abre o jogo pelo Explorer (sem herdar o administrador do Otimiza) e guarda o pedido; quando ele abrir, o
/// vigia de sessão liga a receita que o Turbo lembra. `LIVRES`: abrir um jogo não muda nada no PC (a receita, que muda,
/// passa pela licença no vigia).
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn jogar(nome: String) -> Result<crate::plataforma::partida::Lancamento, String> {
    #[cfg(target_os = "windows")]
    return crate::plataforma::partida::lancar(&nome);
    #[cfg(not(target_os = "windows"))]
    {
        let _ = nome;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// A biblioteca: cada jogo já medido, com a última partida e o ganho medido. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn biblioteca_de_jogos() -> Result<Vec<crate::plataforma::historico::ResumoDoJogo>, String> {
    crate::plataforma::historico::biblioteca()
}

/// A capa do jogo pela Steam deste PC (`data:` URL). `None`: sem Steam ou sem arte; a tela desenha uma. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn capa_do_jogo(appid: u32) -> Result<Option<String>, String> {
    Ok(crate::plataforma::capas::capa(appid))
}

/// A arte larga do jogo (a da Steam deste PC), para o topo do Início. `LIVRES`: só lê.
#[tauri::command(async)]
pub fn capa_larga_do_jogo(appid: u32) -> Result<Option<String>, String> {
    Ok(crate::plataforma::capas::capa_larga(appid))
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

/// `LIVRES`: só lê os relatórios do FiveM e o histórico.
#[tauri::command]
pub async fn crashes_do_jogo(
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::crashes::Relatorio, String> {
    #[cfg(target_os = "windows")]
    {
        let mudancas: Vec<(u64, String)> = state
            .changes
            .lock()
            .await
            .applied()
            .iter()
            .map(|a| (a.timestamp, a.optimization_id.clone()))
            .collect();
        tokio::task::spawn_blocking(move || crate::modules::windows::crashes::relatorio(&mudancas))
            .await
            .map_err(|e| format!("Falha ao ler os crashes: {}", e))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`. 60% no 1% pior e 40% na média: ninguém sente média.
#[tauri::command(async)]
pub fn nota_do_jogo() -> Result<crate::modules::pontuacao::Nota, String> {
    let medicoes = crate::modules::medicoes::ler()?;

    // A mais recente CONFIÁVEL: amostra curta faria a nota piscar.
    let Some(m) = medicoes.iter().rev().find(|m| m.confiavel) else {
        return Ok(crate::modules::pontuacao::Nota::SemAmostra);
    };

    Ok(crate::modules::pontuacao::calcular(
        m.fps,
        m.low_1pct,
        m.confiavel,
    ))
}

/// `LIVRES`: com SSD pequeno e HD grande o jogo quase sempre está no HD.
#[tauri::command(async)]
pub fn onde_os_jogos_moram() -> Result<crate::modules::windows::discodojogo::Relatorio, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::modules::windows::discodojogo::analisar())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Só no Windows.".to_string())
    }
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

        Ok(
            configjogo::prever(&conteudo, escolhido, configjogo::manter_janela())
                .into_iter()
                .map(|(chave, atual, novo, custo)| (chave, atual, novo, custo.to_string()))
                .collect(),
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = perfil;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// A única escrita num arquivo do cliente: guarda o arquivo INTEIRO no histórico. `EXIGEM_LICENCA`.
#[cfg(target_os = "windows")]
// `pub(super)`: o teste `o_plano_fala_a_lingua_de_quem_aplica`, em `commands/mod.rs`, confere os nomes por aqui.
pub(super) fn perfil_por_nome(
    nome: &str,
) -> Result<crate::modules::windows::configjogo::Perfil, String> {
    use crate::modules::windows::configjogo::Perfil;

    match nome {
        "sem_teto" => Ok(Perfil::SemTeto),
        "equilibrado" => Ok(Perfil::Equilibrado),
        "competitivo" => Ok(Perfil::Competitivo),
        outro => Err(format!("perfil desconhecido: {}", outro)),
    }
}

/// Um jogo da configuração por motor, com o que o Otimiza já aplicou nele.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigDoJogoNaTela {
    #[serde(flatten)]
    pub jogo: crate::modules::windows::configmotor::JogoNaTela,
    pub historico: String,
    pub aplicado: bool,
}

/// A configuração por motor (Fortnite, CS2, Apex): o arquivo achado e o que o perfil mudaria. `LIVRES`: só lê.
#[tauri::command]
pub async fn config_motor_painel(
    state: State<'_, AppState>,
) -> Result<Vec<ConfigDoJogoNaTela>, String> {
    #[cfg(target_os = "windows")]
    {
        let jogos = tokio::task::spawn_blocking(crate::modules::windows::configmotor::painel)
            .await
            .map_err(|e| e.to_string())?;
        let log = state.changes.lock().await;
        return Ok(jogos
            .into_iter()
            .map(|jogo| {
                let historico = format!("config_motor_{}", jogo.id);
                ConfigDoJogoNaTela {
                    aplicado: log.is_applied(&historico),
                    historico,
                    jogo,
                }
            })
            .collect());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err("A configuração de jogo só existe no Windows.".to_string())
    }
}

/// `EXIGEM_LICENCA`: escreve o perfil leve no arquivo de configuração do jogo (fechado). O arquivo inteiro de antes
/// vai para o histórico: o `revert_optimization` devolve byte a byte.
#[tauri::command]
pub async fn config_motor_aplicar(
    jogo: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeRecord};
        let id = jogo.clone();
        let feito = tokio::task::spawn_blocking(move || {
            crate::modules::windows::configmotor::aplicar(&id)
        })
        .await
        .map_err(|e| e.to_string())??;
        if feito.mudou.is_empty() {
            return Ok(Vec::new());
        }
        let historico = format!("config_motor_{}", jogo);
        let mut log = state.changes.lock().await;
        let gravado = log.record(AppliedOptimization {
            optimization_id: historico,
            name: format!("Configuração do {} · perfil leve", feito.jogo),
            timestamp: now_timestamp(),
            changes: vec![ChangeRecord::GameConfig {
                caminho: feito.arquivo.to_string_lossy().to_string(),
                anterior: Some(feito.anterior.clone()),
                jogo: feito.jogo.to_string(),
            }],
        });
        // Sem a cópia no histórico não há desfazer: o arquivo volta como estava antes de devolver o erro.
        if let Err(e) = gravado {
            return Err(match std::fs::write(&feito.arquivo, &feito.anterior) {
                Ok(()) => format!("Não consegui guardar o desfazer ({}); o arquivo do jogo voltou como estava.", e),
                Err(e2) => format!(
                    "Não consegui guardar o desfazer ({}) nem devolver o arquivo ({}): {} ficou com o perfil leve e                      sem desfazer pelo Otimiza. Para voltar, apague o arquivo com o jogo fechado: o jogo cria outro                      com o padrão dele.",
                    e,
                    e2,
                    feito.arquivo.display()
                ),
            });
        }
        return Ok(feito.mudou);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (jogo, state);
        Err("A configuração de jogo só existe no Windows.".to_string())
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
        let mut log = state.changes.lock().await;
        return aplicar_perfil_do_jogo(&perfil, &mut log);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (perfil, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// O perfil gráfico do GTA/FiveM, com o arquivo inteiro de antes no histórico e o portão "nunca menos FPS". Pelo botão
/// da tela clássica e pelo pedido do Turbo (aplicado quando o jogo fecha).
#[cfg(target_os = "windows")]
fn aplicar_perfil_do_jogo(
    perfil: &str,
    log: &mut crate::modules::changelog::ChangeLog,
) -> Result<Vec<String>, String> {
    {
        use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeRecord};
        use crate::modules::windows::configjogo;

        let escolhido = perfil_por_nome(perfil)?;
        let feito = configjogo::aplicar_perfil(escolhido)?;

        let aviso = feito
            .tela_cheia_suprimida
            .then(|| configjogo::AVISO_JANELA.to_string());

        // Nada mudou, nada é registrado.
        if feito.mudou.is_empty() {
            return Ok(aviso.into_iter().collect());
        }

        // Nunca menos FPS: em observação (`modules::portao`). FiveM é "FiveM_b3258_GTAProcess.exe"; GTA, "GTA5.exe".
        let processo = if feito.jogo.to_lowercase().contains("fivem") {
            "fivem_"
        } else {
            "gta5"
        };
        crate::modules::portao::vigiar(
            &format!("config_jogo_{}", perfil),
            &feito.jogo,
            processo,
            now_timestamp(),
        );

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

        Ok(feito.mudou.into_iter().chain(aviso).collect())
    }
}

/// O perfil gráfico do jogo pelo cartão do Turbo: se ele é oferecido para o jogo aberto, o pedido pendente e o que o
/// histórico diz que já está aplicado. `LIVRES`: só lê.
#[derive(Debug, Clone, Serialize)]
pub struct PerfilDoJogoNoTurbo {
    pub oferece: bool,
    pub pendente: Option<String>,
    pub aplicado: Option<String>,
}

#[tauri::command]
pub async fn turbo_perfil_do_jogo(state: State<'_, AppState>) -> Result<PerfilDoJogoNoTurbo, String> {
    #[cfg(target_os = "windows")]
    {
        use crate::plataforma::{historico, sessao, turbo};
        let oferece = sessao::atual()
            .map(|j| historico::id_do_jogo(&j.executavel).0)
            .is_some_and(|id| turbo::oferece_perfil_do_jogo(&id));
        let log = state.changes.lock().await;
        let aplicado = ["competitivo", "equilibrado", "sem_teto"]
            .into_iter()
            .find(|p| log.is_applied(&format!("config_jogo_{}", p)))
            .map(str::to_string);
        return Ok(PerfilDoJogoNoTurbo { oferece, pendente: turbo::perfil_do_jogo_pendente(), aplicado });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err("O Turbo só existe no Windows.".to_string())
    }
}

/// `EXIGEM_LICENCA`: pede o perfil gráfico do jogo (aplicado quando ele fechar; `None` cancela o pedido).
#[tauri::command]
pub async fn turbo_perfil_do_jogo_pedir(
    perfil: Option<String>,
    state: State<'_, AppState>,
) -> Result<PerfilDoJogoNoTurbo, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    crate::plataforma::turbo::pedir_perfil_do_jogo(perfil.as_deref())?;
    #[cfg(not(target_os = "windows"))]
    let _ = &perfil;
    turbo_perfil_do_jogo(state).await
}

/// O laço da sessão: com um perfil pedido no Turbo e o jogo fechado, aplica e tira o pedido. `None` sem nada a fazer.
#[cfg(target_os = "windows")]
pub async fn aplicar_perfil_pendente(app: &tauri::AppHandle) -> Option<String> {
    use tauri::Manager;
    let perfil = crate::plataforma::turbo::perfil_do_jogo_pendente()?;
    let aberto = tokio::task::spawn_blocking(|| crate::modules::windows::fivem::processos_abertos().0)
        .await
        .unwrap_or(true);
    if aberto {
        return None;
    }
    let estado = app.state::<AppState>();
    let mut log = estado.changes.lock().await;
    let frase = match aplicar_perfil_do_jogo(&perfil, &mut log) {
        Ok(mudou) if mudou.is_empty() => format!("perfil {} do jogo: já estava assim", perfil),
        Ok(mudou) => format!("perfil {} do jogo aplicado: {}", perfil, mudou.join("; ")),
        // Sem o arquivo ou com erro de escrita: o pedido sai mesmo assim, para não tentar a cada 4 s para sempre.
        Err(e) => format!("perfil {} do jogo não aplicado: {}", perfil, e),
    };
    let _ = crate::plataforma::turbo::pedir_perfil_do_jogo(None);
    Some(frase)
}

#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct PainelDoCs2 {
    pub instalado: bool,
    /// O arquivo da integração está na pasta do CS2.
    pub ligado: bool,
    /// O que o CS2 contou nos últimos 30 s (com o jogo aberto e a integração ligada).
    pub estado: Option<crate::plataforma::cs2::EstadoDoCs2>,
}

#[cfg(target_os = "windows")]
fn painel_do_cs2() -> PainelDoCs2 {
    let (ligado, pasta) = crate::plataforma::cs2::maquina::ligado();
    PainelDoCs2 { instalado: pasta.is_some(), ligado, estado: crate::plataforma::cs2::estado() }
}

/// A integração oficial do CS2 (Game State Integration): instalada, ligada e o momento da partida. `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn cs2_painel() -> Result<PainelDoCs2, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(painel_do_cs2())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Escreve `gamestate_integration_otimiza.cfg` na pasta `cfg` do CS2 (vale ao abrir o jogo de novo). `EXIGEM_LICENCA`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn cs2_ligar() -> Result<PainelDoCs2, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        crate::plataforma::cs2::maquina::ligar()?;
        Ok(painel_do_cs2())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Apaga o arquivo da integração. `LIVRES`: é desfazer.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn cs2_desligar() -> Result<PainelDoCs2, String> {
    #[cfg(target_os = "windows")]
    {
        crate::plataforma::cs2::maquina::desligar()?;
        Ok(painel_do_cs2())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}
