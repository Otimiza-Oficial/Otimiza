// O convite do Discord, lido na hora do clique de um arquivo público do repositório: convite vence, e assim
// quem já instalou não fica com link morto. No clique e não na abertura, para não pesar em quem nunca pede
// suporte. É um GET anônimo: nenhum dado da máquina viaja.

use serde::Deserialize;

/// Editável sem publicar versão nova: trocar o convite não pode exigir atualizar o programa.
const ENDERECO: &str =
    "https://raw.githubusercontent.com/Otimiza-Oficial/Otimiza/main/.github/convite.json";

/// Prazo curto: há alguém com o dedo no botão.
const TIMEOUT_SEGUNDOS: u64 = 3;

#[derive(Debug, Deserialize)]
struct Publicado {
    discord: Option<String>,
}

/// O valor vem da rede e abre o navegador do cliente: só `https://discord.gg/<código>`, sem outro domínio nem
/// caminho a mais.
pub fn parece_convite(endereco: &str) -> bool {
    let Some(codigo) = endereco.strip_prefix("https://discord.gg/") else {
        return false;
    };

    !codigo.is_empty()
        && codigo.len() <= 32
        && codigo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn ler_publicado(json: &str) -> Option<String> {
    let publicado: Publicado = serde_json::from_str(json).ok()?;
    let endereco = publicado.discord?;

    parece_convite(&endereco).then_some(endereco)
}

/// `None` em qualquer tropeço: quem chama usa o convite embutido.
pub async fn consultar() -> Option<String> {
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SEGUNDOS))
        // O User-Agent diz o programa e a versão, nada da máquina.
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    let resposta = cliente.get(ENDERECO).send().await.ok()?;

    if !resposta.status().is_success() {
        return None;
    }

    ler_publicado(&resposta.text().await.ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_um_convite_de_verdade() {
        assert!(parece_convite("https://discord.gg/ultimus"));
        assert!(parece_convite("https://discord.gg/otimiza-oficial"));
        assert!(parece_convite("https://discord.gg/a_b-C9"));
    }

    #[test]
    fn recusa_qualquer_coisa_que_nao_seja_convite() {
        for estranho in [
            "http://discord.gg/abc",              // sem TLS
            "https://discord.gg.exemplo.com/abc", // domínio parecido
            "https://exemplo.com/discord.gg/abc",
            "https://discord.gg/",                // sem código
            "https://discord.gg/abc/def",         // caminho a mais
            "https://discord.gg/abc?x=1",         // consulta pendurada
            "javascript:alert(1)",
            "",
        ] {
            assert!(
                !parece_convite(estranho),
                "{:?} nao pode ser aceito como convite",
                estranho
            );
        }
    }

    #[test]
    fn le_o_convite_do_json_publicado() {
        let json = r#"{"discord": "https://discord.gg/otimiza"}"#;

        assert_eq!(
            ler_publicado(json),
            Some("https://discord.gg/otimiza".to_string())
        );
    }

    #[test]
    fn json_estranho_cai_na_reserva() {
        for ruim in [
            "",
            "não é json",
            "{}",
            r#"{"discord": null}"#,
            r#"{"discord": ""}"#,
            r#"{"discord": "https://exemplo.com/entrar"}"#,
            r#"{"outro_campo": "https://discord.gg/abc"}"#,
        ] {
            assert_eq!(ler_publicado(ruim), None, "{:?} deveria cair na reserva", ruim);
        }
    }
}
