// Licença e conta: estado e ativação da licença, a conta que está rodando, feedback, convite do Discord,
// versão e aviso de versão nova, e o relatório de suporte.

use super::*;

/// O convite embutido vem da tela (é o único em `PedidoDeFeedback.svelte`): duplicá-lo aqui faria dois valores discordarem.
/// `LIVRES`: quem não ativou é quem mais precisa do suporte.
#[tauri::command]
pub async fn convite_do_discord(embutido: String) -> Result<String, String> {
    Ok(crate::modules::convite::consultar()
        .await
        .unwrap_or(embutido))
}

/// Onde está o pedido de feedback: contando, liberado ou exigido. `LIVRES`: só lê.
#[tauri::command(async)]
pub fn feedback_estado() -> Result<crate::plataforma::feedback::Estado, String> {
    Ok(crate::plataforma::feedback::estado(None))
}

/// "Já enviei": pergunta ao bot se o feedback desta máquina está no canal. `LIVRES`: quem está com as correções
/// pausadas é justamente quem precisa chamar.
#[tauri::command]
pub async fn feedback_verificar() -> Result<crate::plataforma::feedback::Estado, String> {
    crate::plataforma::feedback::atualizar_configuracao().await;
    Ok(crate::plataforma::feedback::verificar().await)
}

/// Vinte e um ajustes são por CONTA: elevado por outra, iriam para um perfil que ninguém usa. `LIVRES`.
#[tauri::command(async)]
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
#[cfg_attr(test, derive(specta::Type))]
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

/// `LIVRES` (faltou quando um cliente precisou de script de PowerShell à mão; ver `modules::windows::suporte`). NÃO
/// síncrono: `health::analyze()` e `thermal::analyze()` somam 12 a 15 s, e na thread principal congelariam a
/// janela.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
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
#[cfg_attr(test, derive(specta::Type))]
pub struct AvisoDeVersao {
    pub comparacao: crate::modules::atualizacao::Comparacao,
    pub versao_publicada: Option<String>,
    /// O que a tela escreve: "1.0" para a tag `v4.0.0`.
    pub versao_publicada_exibida: Option<String>,
    pub pagina: Option<String>,
}

/// `LIVRES`: só pergunta ao GitHub; não instala nada. A versão instalada é `CARGO_PKG_VERSION`.
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn versao_mais_nova() -> AvisoDeVersao {
    let instalada = env!("CARGO_PKG_VERSION");

    match crate::modules::atualizacao::consultar_ultima().await {
        Some(ultima) => AvisoDeVersao {
            comparacao: crate::modules::atualizacao::comparar(instalada, &ultima.versao),
            versao_publicada_exibida: crate::modules::atualizacao::de_exibicao(&ultima.versao),
            versao_publicada: Some(ultima.versao),
            pagina: ultima.pagina,
        },
        // Falha de rede é silêncio: `NaoSei` não mostra nada.
        None => AvisoDeVersao {
            comparacao: crate::modules::atualizacao::Comparacao::NaoSei,
            versao_publicada: None,
            versao_publicada_exibida: None,
            pagina: None,
        },
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct VersaoDoApp {
    /// Semver do executável: é o que o atualizador e o instalador comparam.
    pub interna: String,
    /// O que o cliente vê ("1.0").
    pub exibida: String,
    /// Compilado pela esteira oficial (com o endereço da API da licença). Falso: compilação local, sem a conferência
    /// online da licença; a tela diz isso para ninguém mandar esse instalador a um cliente por engano (Plano de refino,
    /// 1.1).
    pub oficial: bool,
}

/// `LIVRES`: só lê a versão compilada no executável.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn versao_do_app() -> VersaoDoApp {
    VersaoDoApp {
        interna: env!("CARGO_PKG_VERSION").to_string(),
        exibida: crate::modules::atualizacao::versao_de_exibicao(),
        oficial: crate::modules::licenca_online::compilacao_oficial(),
    }
}

/// Livre: alimenta a tela de compra.
#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn licenca_estado() -> crate::modules::licenca::Estado {
    crate::modules::licenca::estado()
}

#[tauri::command(async)]
#[cfg_attr(test, specta::specta)]
pub fn licenca_ativar(chave: String) -> Result<crate::modules::licenca::Estado, String> {
    crate::modules::licenca::ativar(&chave)?;
    Ok(crate::modules::licenca::estado())
}
