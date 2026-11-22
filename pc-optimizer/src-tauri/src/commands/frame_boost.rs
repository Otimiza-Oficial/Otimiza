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
            alvo_fps: None,
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
#[cfg_attr(test, specta::specta)]
pub fn frame_boost_painel() -> Result<crate::plataforma::frameboost::Painel, String> {
    #[cfg(target_os = "windows")]
    return Ok(crate::plataforma::frameboost::painel());
    #[cfg(not(target_os = "windows"))]
    Err("O Frame Boost só existe no Windows.".to_string())
}

/// `EXIGEM_LICENCA`: liga o gerador do Otimiza no jogo aberto (o anticheat é conferido antes) e trava o FPS base
/// (`base_do_jogo`), no histórico de mudanças do app.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn frame_boost_ligar(
    multiplicador: u8,
    sem_limite: Option<bool>,
    escala: Option<crate::modules::windows::geracao::escala::Nivel>,
    alvo: Option<u32>,
    leve: Option<bool>,
    state: State<'_, AppState>,
) -> Result<crate::plataforma::frameboost::Painel, String> {
    crate::modules::licenca::exigir()?;
    let app = crate::aplicativo()?;
    #[cfg(target_os = "windows")]
    {
        // A trava do histórico fica presa enquanto liga e vai junto para a thread: o Smooth Motion, o AFMF e o portão da
        // base gravam no histórico, e só quem segura a trava grava (1.0.12, um dono só).
        let guarda = state.changes.clone().lock_owned().await;
        let (mut log, ligado) = tokio::task::spawn_blocking(move || {
            let mut log = guarda;
            let r = crate::plataforma::frameboost::ligar(
                &app,
                multiplicador,
                sem_limite.unwrap_or(false),
                escala,
                alvo,
                leve.unwrap_or(false),
                &mut log,
            );
            (log, r)
        })
        .await
        .map_err(|e| e.to_string())?;
        ligado?;
        crate::plataforma::frameboost::travar_base(&mut log);
        drop(log);
        // O Auto sem escala vigia o jogo: perto dos Hz, trava a base na metade sozinho (1.0.15, teste 2).
        if multiplicador == 0 && escala.is_none() && !sem_limite.unwrap_or(false) {
            vigiar_o_auto(state.changes.clone());
        }
        return Ok(crate::plataforma::frameboost::painel());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, multiplicador, sem_limite, alvo, state);
        Err("O Frame Boost só existe no Windows.".to_string())
    }
}

/// O mesmo pedaço do quadro do jogo (antes de ampliar) e da imagem ampliada pelo FSR, para a tela comparar.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct ComparacaoNaTela {
    /// `data:image/bmp;base64,...`
    pub jogo: String,
    pub tela: String,
    pub jogo_px: [u32; 2],
    pub tela_px: [u32; 2],
}

/// A nitidez da escala em "stops" do FSR (0 = a máxima, 2 = a mais suave). Vale no próximo quadro, ligado ou não.
/// `LIVRES`: só mexe na imagem que o próprio Otimiza desenha.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn escala_nitidez(stops: f32) -> f32 {
    #[cfg(target_os = "windows")]
    return crate::modules::windows::geracao::gpu::definir_nitidez(stops);
    #[cfg(not(target_os = "windows"))]
    stops
}

/// A nitidez do `Rcas` também sem escala (ligada por padrão desde a 1.0.15): `None` só lê. Vale no próximo quadro,
/// ligado ou não. `LIVRES`: só mexe na imagem que o próprio Otimiza desenha.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn nitidez_sem_escala(ligada: Option<bool>) -> bool {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::geracao::gpu;
        return match ligada {
            Some(v) => gpu::definir_nitidez_sem_escala(v),
            None => gpu::nitidez_sem_escala(),
        };
    }
    #[cfg(not(target_os = "windows"))]
    ligada.unwrap_or(true)
}

/// Pede a comparação da escala: o próximo quadro real com o jogo na frente. `LIVRES`: só lê a tela.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
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
#[cfg_attr(test, specta::specta)]
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
#[cfg_attr(test, specta::specta)]
pub async fn frame_boost_destravar_base(
    state: State<'_, AppState>,
) -> Result<crate::plataforma::frameboost::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        let jogo = crate::plataforma::sessao::atual().ok_or("Abra o jogo primeiro.")?;
        let mut log = state.changes.lock().await;
        crate::plataforma::teto_do_jogo::soltar_base(&jogo.executavel, true, &mut log)?;
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

/// `EXIGEM_LICENCA`: volta a travar o FPS base do jogo aberto (desfaz o Destravar). Ligado, trava agora; desligado, o
/// próximo ligar passa pelo portão de novo (1.0.14: antes só aparecia ligado, e com a base solta não ligava).
#[tauri::command]
#[cfg_attr(test, specta::specta)]
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

/// O VIGIA DO AUTO (1.0.15, teste 2). No log do sócio, o Auto com a base solta ficou 96 de 99 janelas de 10 s fora da
/// frente (FiveM a 84-124 FPS em 144 Hz): ligado e sem fazer nada. Com o jogo perto dos Hz por `PERTO_FIRME_S` seguidos,
/// o Auto trava a base na metade dos Hz sozinho, uma vez por ligada, e a tela explica (nota da base). Para quando o
/// gerador para ou sai do Auto; desiste depois de 10 minutos.
#[cfg(target_os = "windows")]
fn vigiar_o_auto(changes: std::sync::Arc<tokio::sync::Mutex<crate::modules::changelog::ChangeLog>>) {
    use crate::modules::windows::geracao::{self, Situacao};
    tauri::async_runtime::spawn(async move {
        let mut perto_desde: Option<std::time::Instant> = None;
        for _ in 0..300 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let e = geracao::estado();
            if e.situacao == Situacao::Parado || !e.automatico {
                return;
            }
            let perto = e.perto_dos_hz && e.situacao == Situacao::Gerando;
            let ha_s = if perto {
                perto_desde.get_or_insert_with(std::time::Instant::now).elapsed().as_secs_f64()
            } else {
                perto_desde = None;
                0.0
            };
            if !crate::plataforma::frameboost::auto_trava_na_metade(true, e.escala.is_some(), ha_s, e.fps_reais, e.hz) {
                continue;
            }
            let guarda = changes.clone().lock_owned().await;
            let r = tokio::task::spawn_blocking(move || {
                let mut log = guarda;
                crate::plataforma::frameboost::travar_na_metade(&mut log)
            })
            .await;
            match r {
                Ok(Ok(nota)) => crate::utils::Logger::info(&format!("frame boost: o Auto travou a base sozinho: {}", nota)),
                Ok(Err(motivo)) => {
                    crate::utils::Logger::warn(&format!("frame boost: o Auto não travou a base: {}", motivo));
                    crate::plataforma::frameboost::anotar_base(Some(format!(
                        "O jogo está perto dos Hz e o Auto não tem onde gerar sem tranco. Para ele encher o monitor liso, o Otimiza travaria o FPS base na metade dos Hz, mas: {}",
                        motivo
                    )));
                }
                Err(e) => crate::utils::Logger::warn(&format!("frame boost: o vigia do Auto: {}", e)),
            }
            return;
        }
    });
}

/// `EXIGEM_LICENCA`: trava o FPS base do jogo aberto em exatamente metade dos Hz (72 em 144), o "Travar em 72" da tela
/// quando o Frame Boost sai da frente com o jogo perto dos Hz. Vale ao reabrir o jogo.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn frame_boost_travar_na_metade(
    state: State<'_, AppState>,
) -> Result<crate::plataforma::frameboost::Painel, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        let guarda = state.changes.clone().lock_owned().await;
        tokio::task::spawn_blocking(move || {
            let mut log = guarda;
            crate::plataforma::frameboost::travar_na_metade(&mut log)
        })
        .await
        .map_err(|e| e.to_string())??;
        return Ok(crate::plataforma::frameboost::painel());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err("O Frame Boost só existe no Windows.".to_string())
    }
}

/// `LIVRES`: desligar nunca depende de licença. O desligar desfaz o Smooth Motion e o AFMF no histórico: a trava do
/// histórico fica presa enquanto isso e vai junto para a thread.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn frame_boost_desligar(
    state: State<'_, AppState>,
) -> Result<crate::plataforma::frameboost::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        let guarda = state.changes.clone().lock_owned().await;
        return tokio::task::spawn_blocking(move || {
            let mut log = guarda;
            crate::plataforma::frameboost::desligar(&mut log)
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err("O Frame Boost só existe no Windows.".to_string())
    }
}

/// Seis segundos de PresentMon no jogo: o FPS na tela com AFMF ou Smooth Motion, medido. `LIVRES`: só mede.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn frame_boost_medir() -> Result<crate::plataforma::frameboost::Painel, String> {
    #[cfg(target_os = "windows")]
    return crate::plataforma::frameboost::medir();
    #[cfg(not(target_os = "windows"))]
    Err("O Frame Boost só existe no Windows.".to_string())
}

/// Mede o jogo aberto e devolve o plano de ritmo estável (o teto que deixa o jogo liso). `LIVRES`: só mede.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
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

/// Grava no perfil do jogo o teto que a medição mostrou (guardando o anterior). Vale ao reabrir o jogo. Grava o número
/// que a pessoa viu: antes media de novo e gravava o resultado da segunda medição. `EXIGEM_LICENCA`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub async fn jogo_liso_aplicar(
    teto: u32,
    hz: u32,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        let jogo =
            crate::plataforma::sessao::atual().ok_or("Abra o jogo e entre na partida antes.")?;
        let mut log = state.changes.lock().await;
        crate::plataforma::jogo_liso::aplicar(&jogo.executavel, teto, hz, &mut log).await?;
        Ok(teto)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (teto, hz, state);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Quem ocupa a memória agora (o jogo aberto nunca entra). `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
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
#[cfg_attr(test, specta::specta)]
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
