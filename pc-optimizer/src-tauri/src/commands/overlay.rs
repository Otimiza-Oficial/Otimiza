// O que se desenha por cima do jogo ou da tela: mira, cores, overlay, contador do FiveM e o FOV do FiveM.

/// Liga a mira na tela por cima do jogo aberto. `EXIGEM_LICENCA`: recurso do produto; não grava nada no PC e some
/// ao desligar. Recusa com anticheat que barra sobreposição; com ressalva (servidor de RP), só com `confirmado`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn mira_ligar(
    mira: crate::plataforma::mira::Mira,
    confirmado: bool,
) -> Result<crate::plataforma::mira::Estado, String> {
    crate::modules::licenca::exigir()?;
    let app = crate::aplicativo()?;

    #[cfg(target_os = "windows")]
    {
        use crate::plataforma::jogos::Sobreposicao;
        let mira = crate::plataforma::mira::validar(&mira)?;
        let jogo = crate::plataforma::sessao::atual()
            .ok_or("Abra o jogo primeiro: a mira fica no centro da janela dele.")?;

        let barrada = tokio::task::spawn_blocking(|| {
            use crate::modules::windows::anticheat;
            anticheat::permite(
                anticheat::Acao::SobreporJanela,
                &anticheat::detectar_agora(),
            )
            .motivo()
            .map(str::to_string)
        })
        .await
        .map_err(|e| e.to_string())?;
        if let Some(motivo) = barrada {
            return Err(motivo);
        }
        match jogo.perfil.sobreposicao {
            Sobreposicao::Barrada(motivo) => return Err(motivo.to_string()),
            // A tela pergunta e chama de novo com `confirmado`.
            Sobreposicao::Desaconselhada(motivo) if !confirmado => {
                return Err(format!("CONFIRMAR:{}", motivo))
            }
            _ => {}
        }

        crate::plataforma::mira::ligar(&app, jogo.pid, jogo.nome.clone(), mira)?;
        Ok(crate::plataforma::mira::estado())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, mira, confirmado);
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Troca o desenho da mira já ligada. `LIVRES`: só redesenha a janela do Otimiza.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn mira_atualizar(
    mira: crate::plataforma::mira::Mira,
) -> Result<(), String> {
    let app = crate::aplicativo()?;
    let mira = crate::plataforma::mira::validar(&mira)?;
    #[cfg(target_os = "windows")]
    return crate::plataforma::mira::atualizar(&app, mira);
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, mira);
        Ok(())
    }
}

/// `LIVRES`: desligar nunca depende de licença.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn mira_desligar() -> Result<(), String> {
    let app = crate::aplicativo()?;
    #[cfg(target_os = "windows")]
    crate::plataforma::mira::desligar(&app);
    #[cfg(not(target_os = "windows"))]
    let _ = app;
    Ok(())
}

/// `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn mira_estado() -> Result<crate::plataforma::mira::Estado, String> {
    #[cfg(target_os = "windows")]
    return Ok(crate::plataforma::mira::estado());
    #[cfg(not(target_os = "windows"))]
    Ok(crate::plataforma::mira::Estado {
        ligada: false,
        jogo: None,
    })
}

/// Os comandos de console do CS2 para a mesma mira. `LIVRES`: só monta texto.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn mira_cs2(mira: crate::plataforma::mira::Mira) -> Result<String, String> {
    Ok(crate::plataforma::mira::comandos_cs2(
        &crate::plataforma::mira::validar(&mira)?,
    ))
}

/// Por qual caminho as cores vão (driver AMD, vibração NVIDIA ou rampa do Windows), o que cada um sabe mudar e os
/// perfis por jogo. `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn cores_estado() -> Result<crate::plataforma::cores::Estado, String> {
    #[cfg(target_os = "windows")]
    return Ok(crate::plataforma::cores::estado());
    #[cfg(not(target_os = "windows"))]
    Err("Cores do monitor só existem no Windows.".to_string())
}

/// Aplica no monitor agora. Guarda o valor de antes primeiro. `EXIGEM_LICENCA`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn cores_aplicar(
    ajuste: crate::plataforma::cores::Ajuste,
) -> Result<crate::plataforma::cores::Caminho, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    return crate::plataforma::cores::aplicar(ajuste);
    #[cfg(not(target_os = "windows"))]
    {
        let _ = ajuste;
        Err("Cores do monitor só existem no Windows.".to_string())
    }
}

/// Devolve as cores de antes. `LIVRES`: voltar ao normal nunca depende de licença.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn cores_restaurar() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return crate::plataforma::cores::restaurar();
    #[cfg(not(target_os = "windows"))]
    Ok(())
}

/// Grava o ajuste de um jogo e se ele liga sozinho com o jogo aberto. `LIVRES`: grava só a preferência; quem mexe no
/// monitor é `cores_aplicar` ou a abertura do jogo, que confere a licença.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn cores_salvar_perfil(
    jogo: Option<String>,
    ajuste: Option<crate::plataforma::cores::Ajuste>,
    so_com_jogo: bool,
) -> Result<crate::plataforma::cores::Perfis, String> {
    let mut p = crate::plataforma::cores::perfis();
    if let Some(jogo) = jogo.filter(|j| !j.trim().is_empty()) {
        let jogo: String = jogo.chars().take(120).collect();
        match ajuste.map(|a| a.limitado()) {
            Some(a) if !a.e_normal() => {
                p.por_jogo.insert(jogo, a);
            }
            _ => {
                p.por_jogo.remove(&jogo);
            }
        }
    }
    p.so_com_jogo = so_com_jogo;
    crate::plataforma::cores::gravar_perfis(&p)?;
    Ok(p)
}

/// O FPS por cima do jogo: modo, canto e se está na tela agora. `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn overlay_estado() -> Result<crate::plataforma::overlay::Estado, String> {
    #[cfg(target_os = "windows")]
    return Ok(crate::plataforma::overlay::estado());
    #[cfg(not(target_os = "windows"))]
    Err(UNSUPPORTED_PLATFORM.to_string())
}

/// Grava o modo e o canto. Desligado tira da tela na hora. `LIVRES`: é preferência do Otimiza; quem põe a janela
/// por cima do jogo é `overlay_ligar` ou a abertura do jogo, que conferem o anticheat.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn overlay_gravar(
    modo: crate::plataforma::overlay::Modo,
    canto: crate::plataforma::overlay::Canto,
    com_frame_boost: Option<bool>,
) -> Result<crate::plataforma::overlay::Estado, String> {
    let mut c = crate::plataforma::overlay::config();
    c.modo = modo;
    c.canto = canto;
    if let Some(v) = com_frame_boost {
        c.com_frame_boost = v;
    }
    crate::plataforma::overlay::gravar(&c)?;
    #[cfg(target_os = "windows")]
    {
        if modo == crate::plataforma::overlay::Modo::Desligado {
            crate::plataforma::overlay::desligar(&crate::aplicativo()?);
        }
        return Ok(crate::plataforma::overlay::estado());
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Mostra agora no jogo aberto. `EXIGEM_LICENCA`: recurso do produto. Anticheat que barra sobreposição recusa; com
/// ressalva, só com `confirmado` (e o jogo entra na lista dos que ligam sozinhos).
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn overlay_ligar(
    confirmado: bool,
) -> Result<crate::plataforma::overlay::Estado, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        use crate::plataforma::overlay;
        let jogo = crate::plataforma::sessao::atual()
            .ok_or("Abra o jogo primeiro: o FPS fica no canto da janela dele.")?;
        let barrada = {
            use crate::modules::windows::anticheat;
            anticheat::permite(
                anticheat::Acao::SobreporJanela,
                &anticheat::detectar_agora(),
            )
            .motivo()
            .map(str::to_string)
        };
        overlay::pode_mostrar(jogo.perfil.sobreposicao, barrada.as_deref(), confirmado)?;
        let mut c = overlay::config();
        if c.modo == overlay::Modo::Desligado {
            c.modo = overlay::Modo::Minimo;
        }
        if confirmado && !c.confirmados.iter().any(|x| x == jogo.perfil.id) {
            c.confirmados.push(jogo.perfil.id.to_string());
        }
        overlay::gravar(&c)?;
        overlay::ligar(&crate::aplicativo()?, jogo.pid, jogo.nome.clone())?;
        return Ok(overlay::estado());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = confirmado;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// O contador do Otimiza no lugar do do FiveM (`cl_drawFPS` no fivem.cfg). `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn contador_fivem_estado() -> Result<crate::plataforma::contador_fivem::Estado, String> {
    Ok(crate::plataforma::contador_fivem::estado())
}

/// Desliga o `cl_drawFPS` no fivem.cfg (agora, ou quando o FiveM fechar) e deixa o FPS do Otimiza sempre no canto do
/// jogo. `EXIGEM_LICENCA`: recurso do produto.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn contador_fivem_ligar() -> Result<crate::plataforma::contador_fivem::Estado, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        let mut c = crate::plataforma::overlay::config();
        if c.modo == crate::plataforma::overlay::Modo::Desligado {
            c.modo = crate::plataforma::overlay::Modo::Minimo;
            crate::plataforma::overlay::gravar(&c)?;
        }
        return crate::plataforma::contador_fivem::pedir(true);
    }
    #[cfg(not(target_os = "windows"))]
    Err(UNSUPPORTED_PLATFORM.to_string())
}

/// Devolve o `cl_drawFPS` de antes. `LIVRES`: desfazer nunca depende de licença.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn contador_fivem_desligar() -> Result<crate::plataforma::contador_fivem::Estado, String> {
    #[cfg(target_os = "windows")]
    return crate::plataforma::contador_fivem::pedir(false);
    #[cfg(not(target_os = "windows"))]
    Err(UNSUPPORTED_PLATFORM.to_string())
}

/// O FOV de primeira pessoa do FiveM (`profile_fpsFieldOfView` no fivem.cfg). `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn fov_fivem() -> Result<crate::plataforma::fov_fivem::Painel, String> {
    #[cfg(target_os = "windows")]
    return Ok(crate::plataforma::fov_fivem::painel());
    #[cfg(not(target_os = "windows"))]
    Err(UNSUPPORTED_PLATFORM.to_string())
}

/// Grava a posição do FOV (agora, ou quando o FiveM fechar), guardando a de antes. `EXIGEM_LICENCA`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn fov_fivem_definir(posicao: u8) -> Result<crate::plataforma::fov_fivem::Painel, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    return crate::plataforma::fov_fivem::pedir(Some(posicao));
    #[cfg(not(target_os = "windows"))]
    {
        let _ = posicao;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Devolve o FOV de antes. `LIVRES`: desfazer nunca depende de licença.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn fov_fivem_devolver() -> Result<crate::plataforma::fov_fivem::Painel, String> {
    #[cfg(target_os = "windows")]
    return crate::plataforma::fov_fivem::pedir(None);
    #[cfg(not(target_os = "windows"))]
    Err(UNSUPPORTED_PLATFORM.to_string())
}
