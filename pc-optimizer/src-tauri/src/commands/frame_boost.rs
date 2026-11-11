// Frame Boost: geração de quadros (framegen e o gerador), a escala, o Jogo liso e a memória para o jogo.

use super::*;

/// `LIVRES`: saber o que a máquina suporta não altera nada.
#[tauri::command]
pub async fn framegen_detectar(processo: Option<String>) -> Result<Deteccao, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            crate::modules::windows::framegen::detectar(processo.as_deref())
        })
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

#[tauri::command(async)]
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
        comparacao: framegen::comparar(
            &desligado,
            &ligado,
            perfil,
            hz,
            nota.map(|n| n.nivel),
            deriva_pct,
        ),
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
        // A janela do gerador fica por cima do jogo: com anticheat que caça sobreposição aberto, nem liga.
        use crate::modules::windows::anticheat;
        if let Some(recusa) = anticheat::permite(
            anticheat::Acao::SobreporJanela,
            &anticheat::detectar_agora(),
        )
        .motivo()
        {
            return Err(recusa.to_string());
        }

        crate::modules::windows::geracao::ligar(crate::modules::windows::geracao::Configuracao {
            processo,
            multiplicador,
            visivel_em_captura: false,
            automatico: false,
            sem_limite: false,
            leve: false,
            escala: None,
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

#[tauri::command(async)]
pub fn gerador_estado() -> crate::modules::windows::geracao::Estado {
    crate::modules::windows::geracao::estado()
}

#[tauri::command(async)]
pub fn framegen_melhor(
    desligado: Rodada,
    testadas: Vec<Rodada>,
    perfil: Perfil,
    hz: u32,
) -> Option<usize> {
    crate::modules::windows::framegen::melhor_rodada(&desligado, &testadas, perfil, hz)
}

#[derive(serde::Serialize)]
pub struct FramegenComparacao {
    pub comparacao: crate::modules::windows::framegen::Comparacao,
    pub artefatos: Option<crate::modules::windows::framegen::NotaDeArtefatos>,
}

/// O Frame Boost do jogo aberto: o gerador que o automático escolhe, os multiplicadores que cabem no monitor dele, a
/// estimativa (ESTIMADA) antes de ligar e o FPS na tela (MEDIDO) depois. `LIVRES`: só lê.
#[tauri::command(async)]
pub fn frame_boost_painel() -> Result<crate::plataforma::frameboost::Painel, String> {
    #[cfg(target_os = "windows")]
    return Ok(crate::plataforma::frameboost::painel());
    #[cfg(not(target_os = "windows"))]
    Err("O Frame Boost só existe no Windows.".to_string())
}

/// `EXIGEM_LICENCA`: liga o gerador do Otimiza no jogo aberto (o anticheat é conferido antes) e trava o FPS base
/// (`base_do_jogo`), no histórico de mudanças do app.
#[tauri::command]
pub async fn frame_boost_ligar(
    app: tauri::AppHandle,
    multiplicador: u8,
    sem_limite: Option<bool>,
    escala: Option<crate::modules::windows::geracao::escala::Nivel>,
    state: State<'_, AppState>,
) -> Result<crate::plataforma::frameboost::Painel, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || {
            crate::plataforma::frameboost::ligar(
                &app,
                multiplicador,
                sem_limite.unwrap_or(false),
                escala,
            )
        })
        .await
        .map_err(|e| e.to_string())??;
        let mut log = state.changes.lock().await;
        crate::plataforma::frameboost::travar_base(&mut log);
        drop(log);
        return Ok(crate::plataforma::frameboost::painel());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, multiplicador, sem_limite, state);
        Err("O Frame Boost só existe no Windows.".to_string())
    }
}

/// O mesmo pedaço do quadro do jogo (antes de ampliar) e da imagem ampliada pelo FSR, para a tela comparar.
#[derive(Debug, Clone, Serialize)]
pub struct ComparacaoNaTela {
    /// `data:image/bmp;base64,...`
    pub jogo: String,
    pub tela: String,
    pub jogo_px: [u32; 2],
    pub tela_px: [u32; 2],
}

/// Pede a comparação da escala: o próximo quadro real com o jogo na frente. `LIVRES`: só lê a tela.
#[tauri::command(async)]
pub fn escala_pedir_comparacao() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::geracao;
        let e = geracao::estado();
        if e.escala.is_none() || e.situacao != geracao::Situacao::Gerando {
            return Err("Ligue o Frame Boost com escala para comparar.".to_string());
        }
        geracao::pedir_comparacao();
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    Err("A escala só existe no Windows.".to_string())
}

/// A comparação pedida, se o gerador já a tirou. `LIVRES`.
#[tauri::command(async)]
pub fn escala_comparacao() -> Option<ComparacaoNaTela> {
    #[cfg(target_os = "windows")]
    {
        use base64::Engine;
        let c = crate::modules::windows::geracao::tomar_comparacao()?;
        let url = |b: &[u8]| {
            format!(
                "data:image/bmp;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(b)
            )
        };
        return Some(ComparacaoNaTela {
            jogo: url(&c.jogo_bmp),
            tela: url(&c.tela_bmp),
            jogo_px: [c.jogo.0, c.jogo.1],
            tela_px: [c.tela.0, c.tela.1],
        });
    }
    #[cfg(not(target_os = "windows"))]
    None
}

/// Tira o teto do FPS base do jogo aberto (vale ao reabrir o jogo). `LIVRES`: desfazer nunca depende de licença.
#[tauri::command]
pub async fn frame_boost_destravar_base(
    state: State<'_, AppState>,
) -> Result<crate::plataforma::frameboost::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        let jogo = crate::plataforma::sessao::atual().ok_or("Abra o jogo primeiro.")?;
        let id = crate::modules::windows::nvdriver::id_do_limite(&jogo.executavel);
        let mut log = state.changes.lock().await;
        if log.is_applied(&id) {
            crate::modules::windows::WindowsOptimizer::new().revert(&id, &mut log)?;
        }
        drop(log);
        // A escolha fica: ligar o Frame Boost de novo não trava outra vez sozinho.
        crate::plataforma::base_do_jogo::soltar(&jogo.executavel, true);
        crate::plataforma::frameboost::anotar_base(Some(
            "FPS base destravado: vale quando você reabrir o jogo.".to_string(),
        ));
        return Ok(crate::plataforma::frameboost::painel());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err("O Frame Boost só existe no Windows.".to_string())
    }
}

/// `EXIGEM_LICENCA`: volta a travar o FPS base do jogo aberto (desfaz o Destravar), com o Frame Boost ligado.
#[tauri::command]
pub async fn frame_boost_travar_base(
    state: State<'_, AppState>,
) -> Result<crate::plataforma::frameboost::Painel, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        let jogo = crate::plataforma::sessao::atual().ok_or("Abra o jogo primeiro.")?;
        crate::plataforma::base_do_jogo::soltar(&jogo.executavel, false);
        let mut log = state.changes.lock().await;
        crate::plataforma::frameboost::travar_base(&mut log);
        drop(log);
        return Ok(crate::plataforma::frameboost::painel());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err("O Frame Boost só existe no Windows.".to_string())
    }
}

/// `LIVRES`: desligar nunca depende de licença.
#[tauri::command(async)]
pub fn frame_boost_desligar() -> Result<crate::plataforma::frameboost::Painel, String> {
    #[cfg(target_os = "windows")]
    return crate::plataforma::frameboost::desligar();
    #[cfg(not(target_os = "windows"))]
    Err("O Frame Boost só existe no Windows.".to_string())
}

/// Seis segundos de PresentMon no jogo: o FPS na tela com AFMF ou Smooth Motion, medido. `LIVRES`: só mede.
#[tauri::command(async)]
pub fn frame_boost_medir() -> Result<crate::plataforma::frameboost::Painel, String> {
    #[cfg(target_os = "windows")]
    return crate::plataforma::frameboost::medir();
    #[cfg(not(target_os = "windows"))]
    Err("O Frame Boost só existe no Windows.".to_string())
}

/// Mede o jogo aberto e devolve o plano de ritmo estável (o teto que deixa o jogo liso). `LIVRES`: só mede.
#[tauri::command(async)]
pub fn jogo_liso_medir() -> Result<crate::plataforma::jogo_liso::Plano, String> {
    #[cfg(target_os = "windows")]
    {
        let jogo = crate::plataforma::sessao::atual()
            .ok_or("Abra o jogo e entre na partida antes de medir.")?;
        crate::plataforma::jogo_liso::medir(&jogo.executavel, 20)
    }
    #[cfg(not(target_os = "windows"))]
    Err(UNSUPPORTED_PLATFORM.to_string())
}

/// "Deixar liso" no cartão da partida: o teto pela partida que acabou, sem o jogo aberto. `EXIGEM_LICENCA`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn jogo_liso_depois_da_partida(
    executavel: String,
    caminho: Option<String>,
    fps: f64,
    trancos_por_minuto: f64,
    state: State<'_, AppState>,
) -> Result<crate::plataforma::jogo_liso::LisoGravado, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        let mut log = state.changes.lock().await;
        crate::plataforma::jogo_liso::gravar_depois_da_partida(&executavel, caminho.as_deref(), fps, trancos_por_minuto, &mut log)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (executavel, caminho, fps, trancos_por_minuto, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Aplica o teto de FPS estável no perfil do jogo (guardando o anterior). Vale ao reabrir o jogo. `EXIGEM_LICENCA`.
#[tauri::command(async)]
pub async fn jogo_liso_aplicar(
    state: State<'_, AppState>,
) -> Result<crate::plataforma::jogo_liso::Plano, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        let jogo =
            crate::plataforma::sessao::atual().ok_or("Abra o jogo e entre na partida antes.")?;
        let plano = crate::plataforma::jogo_liso::medir(&jogo.executavel, 20)?;
        let mut log = state.changes.lock().await;
        crate::plataforma::jogo_liso::aplicar(&jogo.executavel, &plano, &mut log).await?;
        Ok(plano)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Quem ocupa a memória agora (o jogo aberto nunca entra). `LIVRES`: só lê.
#[tauri::command(async)]
pub fn memoria_para_o_jogo() -> Result<crate::plataforma::memoria_jogo::Memoria, String> {
    #[cfg(target_os = "windows")]
    {
        let jogo = crate::plataforma::sessao::atual().map(|j| j.executavel);
        Ok(crate::plataforma::memoria_jogo::agora(jogo.as_deref()))
    }
    #[cfg(not(target_os = "windows"))]
    Err(UNSUPPORTED_PLATFORM.to_string())
}

/// Fecha o programa que a pessoa escolheu na lista. `EXIGEM_LICENCA`.
#[tauri::command(async)]
pub fn memoria_fechar(nome: String) -> Result<crate::plataforma::memoria_jogo::Memoria, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        crate::plataforma::memoria_jogo::fechar(&nome)?;
        let jogo = crate::plataforma::sessao::atual().map(|j| j.executavel);
        Ok(crate::plataforma::memoria_jogo::agora(jogo.as_deref()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = nome;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}
