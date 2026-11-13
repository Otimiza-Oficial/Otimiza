// Turbo: ligar, provar, compartilhar e desligar, com os ajudantes do governador e da base que só ele usa.

use super::*;

/// O Turbo do jogo aberto: o perfil lembrado, se o jogo é competitivo e o que o Turbo fez. `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn turbo_painel() -> Result<crate::plataforma::turbo::Painel, String> {
    #[cfg(target_os = "windows")]
    return Ok(crate::plataforma::turbo::painel());
    #[cfg(not(target_os = "windows"))]
    Err("O Turbo só existe no Windows.".to_string())
}

/// `EXIGEM_LICENCA`: mede o jogo aberto (10 s) ou usa a receita lembrada dele (`lembrado`), aplica e lembra. O FPS
/// base é travado como no Frame Boost (o `travar_base` não trava no só escala).
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn turbo_ligar(
    lembrado: Option<bool>,
    state: State<'_, AppState>,
) -> Result<crate::plataforma::turbo::Painel, String> {
    crate::modules::licenca::exigir()?;
    let app = crate::aplicativo()?;
    #[cfg(target_os = "windows")]
    {
        use crate::plataforma::{historico, sessao, turbo};
        let app_da_prova = app.clone();
        let (id, aplicada) = tokio::task::spawn_blocking(move || -> Result<(String, turbo::Receita), String> {
            let jogo = sessao::atual().ok_or("Abra o jogo primeiro: o Turbo mede o jogo aberto.")?;
            let (id, _) = historico::id_do_jogo(&jogo.executavel);
            if turbo::em_prova() || turbo::prova_sozinha_pendente() {
                return Err("A prova do Turbo está rodando: espere ela terminar.".to_string());
            }
            // Outra prova alternada (a do plano de energia) roda agora: ligar mudaria o lado dela, e a do Turbo não
            // teria a vez para conferir esta receita.
            if crate::modules::provaalternada::em_andamento() {
                return Err("Uma prova está rodando: espere ela terminar para ligar o Turbo.".to_string());
            }
            let anterior = turbo::perfil(&id);
            let reprovada = anterior
                .as_ref()
                .filter(|p| p.prova == Some(turbo::Desfecho::Piorou))
                .map(|p| p.receita.clone());
            let guardado = lembrado.unwrap_or(false).then_some(anterior).flatten();
            if guardado.as_ref().is_some_and(|p| p.prova == Some(turbo::Desfecho::Piorou)) {
                return Err(
                    "Na prova, esta receita deixou o jogo pior neste PC: o Turbo não a liga de novo. Toque em \
                     Medir de novo para uma receita nova."
                        .to_string(),
                );
            }
            // Com a receita lembrada, o perfil fica como está (inclusive o desfecho da prova).
            let lembrada = guardado.is_some();
            let (receita, fps_antes) = match guardado {
                Some(p) => (p.receita, p.fps_antes),
                None => turbo::medir()?,
            };
            // A medida nova deu a mesma receita que a prova reprovou: não liga, e a reprovação fica guardada.
            if reprovada.as_ref().is_some_and(|r| turbo::mesma_acao(r, &receita)) {
                return Err(
                    "A medida de agora deu a mesma receita que a prova reprovou neste PC, então o Turbo não a liga. Ela pode mudar com outro trecho do jogo, outra resolução ou outro driver."
                        .to_string(),
                );
            }
            turbo::aplicar(&app, &receita, fps_antes);
            let aplicada = receita.clone();
            if !lembrada && turbo::vale_guardar(receita.gargalo) {
                let perfil = turbo::Perfil {
                    receita,
                    fps_antes,
                    medido_em: crate::modules::changelog::now_timestamp(),
                    prova: None,
                };
                if let Err(e) = turbo::lembrar(&id, perfil) {
                    crate::utils::Logger::warn(&format!("turbo: não guardei a receita de {}: {}", id, e));
                }
            }
            Ok((id, aplicada))
        })
        .await
        .map_err(|e| e.to_string())??;
        let mut log = state.changes.lock().await;
        let trava = sessao_do_turbo().map(|exe| (crate::modules::windows::nvdriver::id_do_limite(&exe), exe));
        let ja_travada = trava.as_ref().is_some_and(|(id, _)| log.is_applied(id));
        crate::plataforma::frameboost::travar_base(&mut log);
        if let Some((_, exe)) = trava.filter(|(id, _)| !ja_travada && log.is_applied(id)) {
            turbo::marcar_base(Some(&exe));
        }
        if turbo::governador_na_receita(&aplicada) {
            ligar_governador_do_turbo(&mut log);
        }
        drop(log);
        // Primeira vez desta receita neste jogo: a prova roda sozinha, e o fim dela avisa a tela (`turbo:prova_fim`).
        // Só a receita que acabou de ligar: a lembrada de outra medida (esta deu "nada no limite") não é provada nem
        // religada.
        let desta_vez = turbo::perfil(&id)
            .filter(|p| turbo::mesma_acao(&p.receita, &aplicada))
            .is_some_and(|p| turbo::prova_sozinha(&p));
        if desta_vez && turbo::reservar_prova_sozinha() {
            tauri::async_runtime::spawn(async move {
                if let Err(e) = provar_o_turbo(app_da_prova, true).await {
                    crate::utils::Logger::warn(&format!("turbo: a prova automática não terminou: {}", e));
                }
            });
        }
        return Ok(turbo::painel());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, lembrado, state);
        Err("O Turbo só existe no Windows.".to_string())
    }
}

/// `EXIGEM_LICENCA`: a prova A-B do Turbo no jogo aberto (uns 5 minutos), com o passo a passo em `turbo:prova`. Piorou:
/// o Turbo fica desligado e a receita não volta neste jogo. Senão, a receita inteira volta e o FPS base é travado como
/// no Frame Boost.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn turbo_provar() -> Result<crate::modules::provaalternada::Resultado, String> {
    crate::modules::licenca::exigir()?;
    let app = crate::aplicativo()?;
    #[cfg(target_os = "windows")]
    return provar_o_turbo(app, false).await;
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("O Turbo só existe no Windows.".to_string())
    }
}

/// A prova, pedida no botão ou sozinha na primeira vez: os passos vão em `turbo:prova` e o fim, com o erro se houver,
/// em `turbo:prova_fim` (a tela que não pediu a prova também redesenha).
#[cfg(target_os = "windows")]
async fn provar_o_turbo(
    app: tauri::AppHandle,
    sozinha: bool,
) -> Result<crate::modules::provaalternada::Resultado, String> {
    use tauri::{Emitter, Manager};
    let (emissor, na_prova) = (app.clone(), app.clone());
    let r = tokio::task::spawn_blocking(move || {
        crate::plataforma::turbo::provar(&na_prova, &|passo| {
            let _ = emissor.emit("turbo:prova", passo);
        })
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    if let Ok(prova) = &r {
        let estado = app.state::<AppState>();
        let mut log = estado.changes.lock().await;
        if prova.desfecho != crate::modules::provaalternada::Desfecho::Piorou {
            crate::plataforma::frameboost::travar_base(&mut log);
        } else {
            soltar_governador_do_turbo(&mut log);
            // Piorou: o Turbo saiu, e a trava do FPS base que ele pôs (vale ao reabrir o jogo) sai junto.
            let trava = crate::modules::windows::nvdriver::id_do_limite(&prova.jogo);
            if log.is_applied(&trava) {
                if let Err(e) = crate::modules::windows::WindowsOptimizer::new().revert(&trava, &mut log) {
                    crate::utils::Logger::warn(&format!("turbo: não soltei o FPS base depois da prova: {}", e));
                }
            }
            crate::plataforma::turbo::marcar_base(None);
        }
    }
    // A flag sai antes do aviso: a tela que redesenha ao receber o fim já lê o painel sem a prova.
    if sozinha {
        crate::plataforma::turbo::liberar_prova_sozinha();
    }
    let _ = app.emit("turbo:prova_fim", r.as_ref().err());
    // Depois do aviso: a prova aparece na hora, e os núcleos rápidos (~1 min) entram nos passos quando terminarem.
    if let Ok(prova) = &r {
        let prova = prova.clone();
        tauri::async_runtime::spawn_blocking(move || testar_nucleos_depois_da_prova(&prova));
    }
    r
}

/// A última prova do Turbo. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn turbo_prova_guardada() -> Option<crate::modules::provaalternada::Resultado> {
    crate::plataforma::turbo::prova_guardada()
}

/// O texto da última prova do Turbo para colar no Discord: só de prova com comparação justa e margem. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn turbo_compartilhar() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        let prova = crate::plataforma::turbo::prova_guardada();
        let receita = crate::plataforma::sessao::atual()
            .map(|j| crate::plataforma::historico::id_do_jogo(&j.executavel).0)
            .or_else(|| {
                prova
                    .as_ref()
                    .map(|p| crate::plataforma::historico::id_do_jogo(&p.jogo).0)
            })
            .and_then(|id| crate::plataforma::turbo::perfil(&id))
            .and_then(|p| crate::plataforma::turbo::receita_da_prova(&crate::plataforma::turbo::conforme_os_ajustes(p.receita)))
            .map(|r| crate::plataforma::turbo::descrever(&r))
            .unwrap_or_else(|| "Turbo".to_string());
        let data = prova
            .as_ref()
            .and_then(|p| chrono::DateTime::from_timestamp(p.quando as i64, 0))
            .map(|d| d.with_timezone(&chrono::Local).format("%d/%m/%Y").to_string())
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
        crate::modules::compartilhar::texto_do_turbo(
            prova.as_ref(),
            &receita,
            (cpu.as_deref(), gpu.as_deref()),
            &data,
        )
        .map_err(|r| match r {
            crate::modules::compartilhar::Recusa::SemProva => {
                "Ainda não há prova do Turbo neste PC. Toque em Provar no cartão do Turbo.".to_string()
            }
            outra => outra.frase().to_string(),
        })
    })
    .await
    .map_err(|e| format!("Falha ao montar o texto: {}", e))?
}

/// Desliga o que o Turbo ligou. `LIVRES`: desfazer nunca depende de licença.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn turbo_desligar(state: State<'_, AppState>) -> Result<crate::plataforma::turbo::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        if crate::plataforma::turbo::em_prova() || crate::plataforma::turbo::prova_sozinha_pendente() {
            return Err("A prova do Turbo está rodando: espere ela terminar. Se o jogo piorar, ela desliga o Turbo sozinha.".to_string());
        }
        // O teste dos núcleos rápidos (depois da prova) mede com o Turbo ligado: desligar no meio mudaria o FPS entre as
        // fases e o teste guardaria uma regra pelo motivo errado.
        if crate::modules::provaalternada::em_andamento() {
            return Err("O Turbo está medindo os núcleos rápidos (cerca de 1 minuto): espere terminar para desligar.".to_string());
        }
        tokio::task::spawn_blocking(crate::plataforma::turbo::desligar)
            .await
            .map_err(|e| e.to_string())??;
        let mut log = state.changes.lock().await;
        soltar_governador_do_turbo(&mut log);
        soltar_base_do_turbo(&mut log);
        drop(log);
        return Ok(crate::plataforma::turbo::painel());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err("O Turbo só existe no Windows.".to_string())
    }
}

/// O governador do modo jogo como parte do Turbo, só quando a pessoa não usa o modo jogo automático (aí o governador é
/// dela, e o Turbo não liga nem desliga). O vigia do `lib.rs` roda enquanto `turbo::governa()` e o devolve quando o
/// jogo fecha.
#[cfg(target_os = "windows")]
fn ligar_governador_do_turbo(log: &mut crate::modules::changelog::ChangeLog) {
    use crate::plataforma::turbo;
    // Ligado pela pessoa (automático ou à mão): é dela, e o Turbo não o assume nem o desliga depois.
    if crate::modules::preferences::Preferences::load().auto_game_mode
        || turbo::governa()
        || crate::modules::windows::gamemode::governador_ligado(log)
    {
        return;
    }
    match crate::modules::windows::gamemode::ativar(log) {
        Ok(frases) => {
            turbo::marcar_governo(true);
            turbo::anotar_passo(format!("Segundo plano: {}", frases.join(" ")));
        }
        Err(e) => turbo::anotar_passo(format!("Segundo plano: {}", e)),
    }
}

/// O executável do jogo aberto, para o teto da base.
#[cfg(target_os = "windows")]
fn sessao_do_turbo() -> Option<String> {
    crate::plataforma::sessao::atual().map(|j| j.executavel)
}

/// O teto do FPS base que o Turbo pôs (`turbo::base_do_turbo`) sai junto com ele.
#[cfg(target_os = "windows")]
fn soltar_base_do_turbo(log: &mut crate::modules::changelog::ChangeLog) {
    let Some(exe) = crate::plataforma::turbo::base_do_turbo() else { return };
    let id = crate::modules::windows::nvdriver::id_do_limite(&exe);
    if log.is_applied(&id) {
        if let Err(e) = crate::modules::windows::WindowsOptimizer::new().revert(&id, log) {
            crate::utils::Logger::warn(&format!("turbo: não soltei o FPS base: {}", e));
            return;
        }
    }
    crate::plataforma::turbo::marcar_base(None);
}

#[cfg(target_os = "windows")]
fn soltar_governador_do_turbo(log: &mut crate::modules::changelog::ChangeLog) {
    use crate::plataforma::turbo;
    if !turbo::governa() {
        return;
    }
    turbo::marcar_governo(false);
    match crate::modules::windows::gamemode::desativar(log) {
        Ok(t) => crate::utils::Logger::info(&format!("turbo: segundo plano devolvido: {}", t)),
        Err(e) => crate::utils::Logger::warn(&format!("turbo: não devolvi o segundo plano: {}", e)),
    }
}

/// Depois da prova, com o processador no limite: os núcleos rápidos (Auto CPU Set), medidos em ~1 min e só mantidos se
/// renderem. Com a vez da prova alternada, para a medição automática não cair no meio.
#[cfg(target_os = "windows")]
fn testar_nucleos_depois_da_prova(prova: &crate::modules::provaalternada::Resultado) {
    use crate::modules::windows::cpuset;
    use crate::plataforma::{historico, sessao, turbo};
    // O jogo de agora tem de ser o da prova (ele pode ter trocado nesse meio-tempo).
    let Some(jogo) = sessao::atual().filter(|j| j.executavel.eq_ignore_ascii_case(&prova.jogo)) else { return };
    let (id, _) = historico::id_do_jogo(&jogo.executavel);
    let Some(perfil) = turbo::perfil(&id) else { return };
    let ja = match cpuset::ler_conferido() {
        Ok(regras) => regras.contains_key(&jogo.executavel.to_lowercase()),
        Err(e) => {
            crate::utils::Logger::warn(&format!("turbo: núcleos rápidos não testados: {}", e));
            return;
        }
    };
    if !turbo::testar_nucleos(&perfil.receita, prova.desfecho, ja) {
        return;
    }
    let Some(_vez) = crate::modules::provaalternada::reservar() else { return };
    let frase = match cpuset::testar(jogo.pid, &jogo.executavel) {
        Ok(r) if r.escolha == cpuset::Escolha::SoDesempenho => format!(
            "Núcleos rápidos: o jogo ficou só neles (medido: {:.0} → {:.0} fps, 1% piores {:.0} → {:.0}).",
            r.fps_todos, r.fps_desempenho, r.low_todos, r.low_desempenho
        ),
        Ok(_) => "Núcleos rápidos: medido, não rendeu neste jogo; o jogo segue em todos os núcleos.".to_string(),
        Err(e) => format!("Núcleos rápidos: {}", e),
    };
    crate::utils::Logger::info(&format!("turbo: {}", frase));
    turbo::anotar_passo(frase);
}

/// "Menor atraso" ou "Mais FPS" para o jogo aberto: com o menor atraso, o Turbo não liga geração de quadros. `LIVRES`:
/// é uma preferência no arquivo do Turbo, não mexe no Windows.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn turbo_atraso(menor: bool) -> Result<crate::plataforma::turbo::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        let jogo = crate::plataforma::sessao::atual().ok_or("Abra o jogo primeiro.")?;
        let (id, _) = crate::plataforma::historico::id_do_jogo(&jogo.executavel);
        crate::plataforma::turbo::escolher_atraso(&id, menor)?;
        return Ok(crate::plataforma::turbo::painel());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = menor;
        Err("O Turbo só existe no Windows.".to_string())
    }
}

/// Esquece a receita lembrada do jogo aberto: o próximo Turbo mede de novo. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn turbo_esquecer() -> Result<crate::plataforma::turbo::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        let jogo = crate::plataforma::sessao::atual().ok_or("Abra o jogo primeiro.")?;
        let (id, _) = crate::plataforma::historico::id_do_jogo(&jogo.executavel);
        crate::plataforma::turbo::esquecer(&id)?;
        return Ok(crate::plataforma::turbo::painel());
    }
    #[cfg(not(target_os = "windows"))]
    Err("O Turbo só existe no Windows.".to_string())
}
