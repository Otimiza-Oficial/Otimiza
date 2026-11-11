// Rede: a rota salto a salto com o dono de cada trecho (`plataforma::rota`), a ligação Wi-Fi (`plataforma::wifi`) e a
// varredura de redes segurada durante o jogo (`plataforma::wifi_varredura`).

#[cfg(not(target_os = "windows"))]
use super::UNSUPPORTED_PLATFORM;
#[cfg(target_os = "windows")]
use crate::plataforma::{rota, wifi, wifi_varredura};

/// Endereço ou nome. Nome resolvido aqui: a rota é de um endereço só, e a tela mostra qual.
#[cfg(target_os = "windows")]
async fn resolver(destino: &str) -> Result<std::net::IpAddr, String> {
    let destino = destino.trim();
    if let Ok(ip) = destino.parse() {
        return Ok(ip);
    }
    tokio::net::lookup_host((destino, 0))
        .await
        .ok()
        .and_then(|mut l| l.next())
        .map(|s| s.ip())
        .ok_or_else(|| format!("Não achei o endereço de \"{}\".", destino))
}

/// Rota até `destino` (endereço ou nome): 10 rodadas por salto, espalhadas pelo segundo. Leva uns 10 s. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn rede_rota(destino: String) -> Result<crate::plataforma::rota::Rota, String> {
    #[cfg(target_os = "windows")]
    {
        let r = rota::diagnosticar(resolver(&destino).await?).await;
        crate::plataforma::rota_historico::guardar(&r, false);
        Ok(r)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = destino;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Ping com a internet ocupada (bufferbloat): parado, baixando e enviando, uns 21 s. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn rede_fila() -> Result<crate::plataforma::fila::FilaMedida, String> {
    #[cfg(target_os = "windows")]
    {
        crate::plataforma::fila::medir().await
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// O ping dos servidores do FiveM que a pessoa colou (R8), do menor para o maior. Uns 2 s. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn fivem_servidores_ping(
    entradas: Vec<String>,
) -> Result<Vec<crate::plataforma::servidores_fivem::ServidorMedido>, String> {
    crate::plataforma::servidores_fivem::medir(entradas).await
}

/// A placa de rede com prova (R2): testa cada propriedade no ping e mantém só o que melhorou. Uns 2 a 4 minutos, com
/// a internet caindo alguns segundos a cada troca. `EXIGEM_LICENCA`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn rede_provar_placa(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::plataforma::placa_prova::ProvaDaPlaca, String> {
    crate::modules::licenca::exigir()?;
    #[cfg(target_os = "windows")]
    {
        // A trava do histórico fica presa o teste inteiro: nada mais grava no meio. O teste grava no do disco (o mesmo
        // conteúdo), e a memória é relida no fim.
        let mut log = state.changes.lock().await;
        let r = tokio::task::spawn_blocking(|| {
            let mut do_disco = crate::modules::changelog::ChangeLog::load();
            crate::plataforma::placa_prova::provar(&mut do_disco)
        })
        .await
        .map_err(|e| e.to_string())?;
        *log = crate::modules::changelog::ChangeLog::load();
        r
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Rota até o servidor da partida aberta. Sem partida, recusa dizendo o que fazer. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn rede_rota_do_jogo() -> Result<crate::plataforma::rota::Rota, String> {
    #[cfg(target_os = "windows")]
    {
        let r = rota::diagnosticar_servidor_do_jogo().await?;
        crate::plataforma::rota_historico::guardar(&r, true);
        Ok(r)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct HistoricoDaRede {
    /// As últimas medidas, a mais recente primeiro.
    pub recentes: Vec<crate::plataforma::rota_historico::RegistroDaRota>,
    pub resumo: crate::plataforma::rota_historico::ResumoDaRede,
    /// Texto pronto para o chamado da operadora (só quando o problema esteve dentro da rede dela).
    pub relatorio: Option<String>,
}

/// As rotas já medidas: quando a conexão piora e o relatório para a operadora. Só lê. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn rede_historico() -> Result<HistoricoDaRede, String> {
    use crate::plataforma::rota_historico as h;
    let todos = h::ler()?;
    Ok(HistoricoDaRede {
        recentes: todos.iter().rev().take(30).cloned().collect(),
        resumo: h::resumir(&todos),
        relatorio: h::relatorio(&todos, h::data_local),
    })
}

/// Por onde o PC sai para a internet (cabo, Wi-Fi, VPN) e, no Wi-Fi, sinal, banda, canal e vizinhos. `LIVRES`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn rede_wifi() -> Result<crate::plataforma::wifi::Diagnostico, String> {
    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(|| wifi::diagnosticar(None))
            .await
            .map_err(|e| e.to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// A escolha "Wi-Fi sem varredura durante o jogo" e se está agindo agora. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn rede_varredura_wifi() -> Result<crate::plataforma::wifi_varredura::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(wifi_varredura::painel())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Liga: com o jogo aberto, age na hora.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn rede_varredura_wifi_ligar() -> Result<crate::plataforma::wifi_varredura::Painel, String> {
    crate::modules::licenca::exigir()?;
    definir_varredura(true)
}

/// Desliga e devolve a varredura antes de responder. `LIVRES`: é desfazer.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn rede_varredura_wifi_desligar() -> Result<crate::plataforma::wifi_varredura::Painel, String> {
    definir_varredura(false)
}

fn definir_varredura(ligado: bool) -> Result<crate::plataforma::wifi_varredura::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        wifi_varredura::definir(ligado)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = ligado;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// O controle de banda do segundo plano: se está ligado, em que jogo está agindo e o que o impede. `LIVRES`.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn banda_painel() -> Result<crate::plataforma::banda::Painel, String> {
    #[cfg(target_os = "windows")]
    {
        Ok(crate::plataforma::banda::painel())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}

/// Liga. Com o jogo aberto, limita na hora; sem administrador, o resultado diz isso em vez de falhar.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn banda_ligar() -> Result<crate::plataforma::banda::Resultado, String> {
    crate::modules::licenca::exigir()?;
    definir_banda(true)
}

/// Desliga e devolve as políticas e o limite do Update antes de responder. `LIVRES`: é desfazer.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn banda_desligar() -> Result<crate::plataforma::banda::Resultado, String> {
    definir_banda(false)
}

fn definir_banda(ligado: bool) -> Result<crate::plataforma::banda::Resultado, String> {
    #[cfg(target_os = "windows")]
    {
        crate::plataforma::banda::definir(ligado)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = ligado;
        Err(UNSUPPORTED_PLATFORM.to_string())
    }
}
