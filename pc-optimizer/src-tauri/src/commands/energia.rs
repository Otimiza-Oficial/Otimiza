// Energia: o painel e o motor de planos de energia, a energia por jogo e o plano do Otimiza (simular,
// aplicar, vistoriar, reparar). Os tetos escondidos entram aqui porque quase todos são de energia.

use super::*;

#[derive(serde::Serialize)]
#[cfg_attr(test, derive(specta::Type))]
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
#[cfg_attr(test, specta::specta)]
pub async fn energia_painel() -> Result<PainelDeEnergia, String> {
    tokio::task::spawn_blocking(|| {
        use crate::modules::windows::{
            motorenergia as motor, motorenergia_maquina as maquina, planoenergia,
        };

        let impressao = maquina::impressao();
        let base = maquina::enumerar_base()?;
        let bateria = maquina::bateria_de_candidatos(&impressao, &base);
        let ativo = impressao.plano_ativo.clone().unwrap_or_default();
        let ppm_atual = maquina::enumerar_plano(&ativo)
            .map(|e| {
                e.configuracoes
                    .into_iter()
                    .filter(|c| {
                        c.alias.as_deref().is_some_and(|a| {
                            let sem_classe = a.trim_end_matches('1');
                            motor::apelidos::DA_TELA.iter().any(|t| {
                                t.eq_ignore_ascii_case(a) || t.eq_ignore_ascii_case(sem_classe)
                            })
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let plano_otimiza_ativo = impressao
            .plano_ativo_nome
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(planoenergia::NOME_DO_PLANO));

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
    #[cfg(target_os = "windows")]
    if crate::modules::provaalternada::em_andamento() {
        return Err(
            "A prova do Otimizar está rodando e troca o plano de energia. Espere ela terminar."
                .to_string(),
        );
    }

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
        let mut medicao = maquina::medir_atual(
            &candidato.id,
            processo.as_deref(),
            segundos.clamp(10, 60),
            repeticoes,
        )?;
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
        crate::modules::windows::motorenergia_maquina::medir_atual(
            "atual",
            processo.as_deref(),
            segundos.clamp(10, 60),
            repeticoes,
        )
    })
    .await
    .map_err(|e| format!("Falha ao medir: {}", e))?
}

#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn tetos_escondidos() -> Result<crate::modules::windows::tetos::Relatorio, String> {
    tokio::task::spawn_blocking(crate::modules::windows::tetos::procurar)
        .await
        .map_err(|e| format!("Falha ao procurar limites: {}", e))
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
            .map(|(i, p)| {
                motor::gerar(
                    &impressao,
                    &base,
                    &format!("refino-{}", i + 1),
                    motor::Papel::Refino,
                    p,
                )
            })
            .collect())
    })
    .await
    .map_err(|e| format!("Falha ao gerar o refino: {}", e))?
}

#[tauri::command(async)]
pub fn energia_escolher(
    resultados: Vec<crate::modules::windows::motorenergia::ResultadoDoCandidato>,
    base: String,
) -> Option<crate::modules::windows::motorenergia::Escolha> {
    use crate::modules::windows::motorenergia as motor;
    // A tela mede a base de novo no meio da bateria; a escolha compara cada candidato já sem a deriva.
    motor::escolher(&motor::corrigir_deriva(&resultados, &base), &base)
}

#[tauri::command]
pub async fn energia_aplicar(
    parametros: crate::modules::windows::motorenergia::Parametros,
) -> Result<crate::modules::windows::motorenergia_maquina::Aplicacao, String> {
    crate::modules::licenca::exigir()?;

    tokio::task::spawn_blocking(move || {
        let r = crate::modules::windows::motorenergia_maquina::aplicar_parametros(
            "escolhido",
            parametros,
        );
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
pub async fn energia_restaurar_anterior(
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::motorenergia_maquina::Restauracao, String> {
    let otimiza = state.changes.lock().await.is_applied("plano_otimiza");
    tokio::task::spawn_blocking(move || {
        crate::modules::windows::motorenergia_maquina::restaurar_anterior(otimiza)
    })
    .await
    .map_err(|e| format!("Falha ao restaurar: {}", e))?
}

#[tauri::command]
pub async fn energia_restaurar_windows(
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::motorenergia_maquina::Restauracao, String> {
    let r = tokio::task::spawn_blocking(
        crate::modules::windows::motorenergia_maquina::restaurar_padrao_windows,
    )
    .await
    .map_err(|e| format!("Falha ao restaurar: {}", e))??;
    // O plano OTIMIZA foi apagado junto: o histórico não pode seguir dizendo que ele está aplicado.
    if r.plano_otimiza_apagado {
        if let Err(e) = state.changes.lock().await.take("plano_otimiza") {
            crate::utils::Logger::warn(&format!(
                "o plano OTIMIZA foi apagado, mas o histórico não gravou: {}",
                e
            ));
        }
    }
    Ok(r)
}

#[tauri::command(async)]
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

#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn energia_remover_perfil_de_jogo(
    executavel: String,
) -> Result<Vec<crate::modules::windows::motorenergia_maquina::PerfilDeJogo>, String> {
    crate::modules::windows::motorenergia_maquina::remover_perfil_de_jogo(&executavel)
}

/// `EXIGEM_LICENCA`: ligado, troca o plano sozinho.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn energia_modo_dinamico(ligado: bool) -> Result<bool, String> {
    crate::modules::licenca::exigir()?;
    crate::modules::windows::motorenergia_maquina::definir_dinamico(ligado).map(|d| d.ligado)
}

/// Energia de jogo: o que muda no processador com o jogo aberto e se está ligada. `LIVRES`: só lê.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn energia_de_jogo() -> Result<crate::plataforma::energia_jogo::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::plataforma::energia_jogo::painel())
    }
    #[cfg(not(target_os = "windows"))]
    Err(UNSUPPORTED_PLATFORM.to_string())
}

/// Liga ou desliga a energia de jogo. Desligar devolve na hora o que estava escrito. `EXIGEM_LICENCA`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn energia_de_jogo_definir(
    ligado: bool,
) -> Result<crate::plataforma::energia_jogo::Painel, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        crate::plataforma::energia_jogo::definir(ligado)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = ligado;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// `LIVRES`: responde "por que não funcionou?" antes de pagar.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn diagnostico_de_energia(
) -> Result<crate::modules::windows::planoenergia::Diagnostico, String> {
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
#[cfg_attr(test, specta::specta)]
pub async fn simular_plano_otimiza(
) -> Result<crate::modules::windows::planoenergia::RelatorioDoPlano, String> {
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
#[cfg_attr(test, specta::specta)]
pub async fn aplicar_plano_otimiza(
    state: State<'_, AppState>,
) -> Result<crate::modules::windows::planoenergia::RelatorioDoPlano, String> {
    crate::modules::licenca::exigir()?;

    #[cfg(target_os = "windows")]
    {
        use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeRecord};

        let relatorio = crate::modules::windows::planoenergia::montar(false)?;

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
#[cfg_attr(test, specta::specta)]
pub async fn vistoriar_plano_otimiza(
) -> Result<crate::modules::windows::planoenergia::Vistoria, String> {
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
#[cfg_attr(test, specta::specta)]
pub async fn reparar_plano_otimiza(
) -> Result<crate::modules::windows::planoenergia::RelatorioDoPlano, String> {
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
