// O aviso de versão nova dentro do programa, para quem não está no Discord. O programa PERGUNTA ao GitHub (não
// há porta aberta para ser avisado). A comparação é pura e testável; a consulta é separada, para o teste da
// regra não depender de rede.

use serde::Serialize;

const REPOSITORIO: &str = "Otimiza-Oficial/Otimiza";

/// Curto: acontece na abertura, e internet ruim não pode prender o cliente esperando um aviso que não pediu.
const TIMEOUT_SEGUNDOS: u64 = 10;

/// Três variantes, não um bool: "não sei" (consulta falhou ou ilegível) não pode virar "está tudo em dia".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Comparacao {
    /// Instalada igual ou MAIOR (compilação local).
    NaoHaNova,
    HaVersaoNova,
    NaoSei,
}

/// Aceita o prefixo `v`. Vazia ou com pedaço não numérico é `None`, que vira `NaoSei`.
fn normalizar(versao: &str) -> Option<Vec<u32>> {
    let sem_prefixo = versao.trim().trim_start_matches(['v', 'V']);

    if sem_prefixo.is_empty() {
        return None;
    }

    let partes: Vec<u32> = sem_prefixo
        .split('.')
        .map(|pedaco| pedaco.trim().parse::<u32>())
        .collect::<Result<_, _>>()
        .ok()?;

    if partes.is_empty() {
        None
    } else {
        Some(partes)
    }
}

/// PELO VALOR: como texto, "1.10.0" viria antes de "1.9.0", e o aviso pararia justo na 1.10.
pub fn comparar(instalada: &str, publicada: &str) -> Comparacao {
    let (Some(mut numeros_instalada), Some(mut numeros_publicada)) =
        (normalizar(instalada), normalizar(publicada))
    else {
        return Comparacao::NaoSei;
    };

    // "1.5" contra "1.5.0": completa com zero, para a ausência não ser lida como menor.
    let tamanho = numeros_instalada.len().max(numeros_publicada.len());
    numeros_instalada.resize(tamanho, 0);
    numeros_publicada.resize(tamanho, 0);

    if numeros_publicada > numeros_instalada {
        Comparacao::HaVersaoNova
    } else {
        Comparacao::NaoHaNova
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UltimaVersao {
    pub versao: String,
    pub pagina: Option<String>,
}

/// `None` quando falha: falha de rede é silêncio, não "está tudo em dia". Anônima, sem servidor nosso no meio.
pub async fn consultar_ultima() -> Option<UltimaVersao> {
    let url = format!("https://api.github.com/repos/{REPOSITORIO}/releases/latest");

    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SEGUNDOS))
        // O GitHub exige User-Agent em toda chamada; sem ele a resposta é 403.
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    let resposta = cliente
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;

    if !resposta.status().is_success() {
        return None;
    }

    let corpo: serde_json::Value = resposta.json().await.ok()?;
    let versao = corpo.get("tag_name").and_then(|v| v.as_str())?.trim();

    if versao.is_empty() {
        return None;
    }

    Some(UltimaVersao {
        versao: versao.to_string(),
        pagina: corpo
            .get("html_url")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_avisa_quando_a_publicada_e_maior() {
        assert_eq!(comparar("1.5.0", "1.5.0"), Comparacao::NaoHaNova);
        assert_eq!(comparar("1.5.0", "1.6.0"), Comparacao::HaVersaoNova);
        assert_eq!(comparar("1.5.0", "v1.6.0"), Comparacao::HaVersaoNova);

        assert_eq!(comparar("1.9.0", "1.10.0"), Comparacao::HaVersaoNova);

        assert_eq!(comparar("1.6.0", "1.5.0"), Comparacao::NaoHaNova);
    }

    #[test]
    fn resposta_ilegivel_nao_vira_ha_versao_nova() {
        assert_eq!(comparar("1.5.0", ""), Comparacao::NaoSei);
        assert_eq!(comparar("1.5.0", "sei lá"), Comparacao::NaoSei);
    }
}
