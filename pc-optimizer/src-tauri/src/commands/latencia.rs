// Latência: o Caçador de DPC (`modules::windows::dpc`), qual driver segura o processador. Só mede: nada é alterado.

#[cfg(not(target_os = "windows"))]
use super::UNSUPPORTED_PLATFORM;
use crate::modules::windows::dpc::ResultadoDaCaptura;

/// Captura por `segundos` (5 a 60; sem valor, 30) e devolve o relatório. Sem administrador, o resultado diz isso em
/// vez de falhar. `LIVRES`: medir não depende de licença.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn dpc_capturar(segundos: Option<u64>) -> ResultadoDaCaptura {
    let segundos = segundos.unwrap_or(crate::modules::windows::dpc::SEGUNDOS_PADRAO);

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || crate::modules::windows::dpc::capturar(segundos))
            .await
            .unwrap_or_else(|e| ResultadoDaCaptura::Falhou {
                etapa: "captura".to_string(),
                codigo: 0,
                texto: e.to_string(),
            })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = segundos;
        ResultadoDaCaptura::NaoSuportado {
            texto: UNSUPPORTED_PLATFORM.to_string(),
        }
    }
}

/// Termina a captura em andamento já, com o que mediu até agora. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn dpc_cancelar() {
    #[cfg(target_os = "windows")]
    crate::modules::windows::dpc::cancelar();
}

/// A taxa real do mouse: conta a entrada bruta por `segundos` (2 a 10) enquanto a pessoa mexe o mouse. Só o instante
/// de cada relato, nunca posição ou clique; nada fica ligado depois. `LIVRES`: não muda o PC.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn mouse_taxa(segundos: u32) -> Result<crate::plataforma::mouse_taxa::TaxaDoMouse, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(move || crate::plataforma::mouse_taxa::maquina::medir(segundos))
            .await
            .map_err(|_| "A medida do mouse não terminou.".to_string())?
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = segundos;
        Err(super::UNSUPPORTED_PLATFORM.to_string())
    }
}
