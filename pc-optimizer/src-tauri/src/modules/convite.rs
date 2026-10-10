// O convite do Discord, lido na hora do clique de um arquivo público do repositório: convite vence, e assim
// quem já instalou não fica com link morto. No clique e não na abertura, para não pesar em quem nunca pede
// suporte. É um GET anônimo: nenhum dado da máquina viaja.
//
// O mesmo arquivo publica, desde a 3.3, onde o bot responde sobre o feedback (`api`) e o link do canal de feedbacks.
// Trocar o servidor do bot não pode exigir publicar versão nova.

use serde::Deserialize;

/// Editável sem publicar versão nova: trocar o convite não pode exigir atualizar o programa.
const ENDERECO: &str =
    "https://raw.githubusercontent.com/Otimiza-Oficial/Otimiza/main/.github/convite.json";

/// Prazo curto: há alguém com o dedo no botão.
const TIMEOUT_SEGUNDOS: u64 = 3;

#[derive(Debug, Deserialize, Default)]
pub struct Publicado {
    #[serde(default)]
    discord: Option<String>,
    /// Onde o bot responde se o feedback desta máquina está no canal (`plataforma::feedback`). Ausente: o pedido de
    /// feedback fica desligado, porque não haveria como liberar quem já mandou.
    #[serde(default)]
    api: Option<String>,
    /// Link direto ao canal de feedbacks (`https://discord.com/channels/<servidor>/<canal>`).
    #[serde(default)]
    canal_feedback: Option<String>,
}

impl Publicado {
    pub fn ler(json: &str) -> Option<Publicado> {
        serde_json::from_str(json).ok()
    }

    pub fn api(&self) -> Option<String> {
        self.api
            .as_deref()
            .filter(|a| parece_api(a))
            .map(|a| a.trim_end_matches('/').to_string())
    }

    pub fn canal_feedback(&self) -> Option<String> {
        self.canal_feedback
            .as_deref()
            .filter(|c| parece_canal(c))
            .map(str::to_string)
    }
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

/// Só HTTPS, um host com ponto e caminho simples: o valor vem da rede e vira o endereço de um pedido.
pub fn parece_api(endereco: &str) -> bool {
    let Some(resto) = endereco.strip_prefix("https://") else {
        return false;
    };
    let host = resto.split('/').next().unwrap_or("");

    endereco.len() <= 200
        && host.contains('.')
        && resto
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':'))
}

/// `https://discord.com/channels/<números>/<números>`, nada além.
pub fn parece_canal(endereco: &str) -> bool {
    let Some(resto) = endereco.strip_prefix("https://discord.com/channels/") else {
        return false;
    };
    let partes: Vec<&str> = resto.split('/').collect();

    partes.len() == 2
        && partes
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 20 && p.chars().all(|c| c.is_ascii_digit()))
}

pub fn ler_publicado(json: &str) -> Option<String> {
    let endereco = Publicado::ler(json)?.discord?;

    parece_convite(&endereco).then_some(endereco)
}

/// `None` em qualquer tropeço: quem chama usa o convite embutido.
pub async fn consultar() -> Option<String> {
    ler_publicado(&baixar().await?)
}

/// O arquivo inteiro, para quem precisa de outro campo além do convite.
pub async fn consultar_publicado() -> Option<Publicado> {
    Publicado::ler(&baixar().await?)
}

async fn baixar() -> Option<String> {
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

    resposta.text().await.ok()
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
            "https://discord.gg/",        // sem código
            "https://discord.gg/abc/def", // caminho a mais
            "https://discord.gg/abc?x=1", // consulta pendurada
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
    fn a_api_do_feedback_so_vale_em_https_com_host_de_verdade() {
        assert!(parece_api("https://otimiza.squareweb.app"));
        assert!(parece_api("https://bot.exemplo.com:8443/base"));

        for ruim in [
            "http://otimiza.squareweb.app",
            "https://localhost",
            "https://user@exemplo.com",
            "https://exemplo.com/?x=1",
            "https://exemplo.com/#a",
            "",
        ] {
            assert!(
                !parece_api(ruim),
                "{:?} nao pode virar endereco de API",
                ruim
            );
        }
    }

    #[test]
    fn o_canal_do_feedback_so_vale_como_link_de_canal_do_discord() {
        assert!(parece_canal(
            "https://discord.com/channels/123456789012345678/1543436859529629707"
        ));

        for ruim in [
            "https://discord.com/channels/123/abc",
            "https://discord.com/channels/123",
            "https://discord.com/channels/1/2/3",
            "https://exemplo.com/channels/1/2",
            "discord://channels/1/2",
        ] {
            assert!(
                !parece_canal(ruim),
                "{:?} nao pode virar link do canal",
                ruim
            );
        }
    }

    #[test]
    fn le_api_e_canal_do_json_publicado_e_tira_a_barra_final() {
        let p = Publicado::ler(
            r#"{"discord":"https://discord.gg/x","api":"https://bot.exemplo.com/","canal_feedback":"https://discord.com/channels/1/2"}"#,
        )
        .unwrap();

        assert_eq!(p.api().as_deref(), Some("https://bot.exemplo.com"));
        assert_eq!(
            p.canal_feedback().as_deref(),
            Some("https://discord.com/channels/1/2")
        );
    }

    #[test]
    fn sem_api_publicada_nao_ha_api() {
        assert_eq!(
            Publicado::ler(r#"{"discord":"https://discord.gg/x"}"#)
                .unwrap()
                .api(),
            None
        );
        assert_eq!(
            Publicado::ler(r#"{"api":"http://inseguro.com"}"#)
                .unwrap()
                .api(),
            None
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
            assert_eq!(
                ler_publicado(ruim),
                None,
                "{:?} deveria cair na reserva",
                ruim
            );
        }
    }

    #[test]
    fn o_convite_publicado_e_aceito_pelo_programa() {
        // O arquivo que o GitHub serve a todo Otimiza instalado: um valor fora da regra desligaria o pedido de
        // feedback em silêncio (ou abriria o navegador do cliente num endereço errado).
        let p = Publicado::ler(include_str!("../../../../.github/convite.json")).expect("JSON válido");
        assert!(parece_convite(p.discord.as_deref().unwrap_or("")), "convite do Discord");
        assert_eq!(p.api().as_deref(), Some("https://otimiza-api.squareweb.app"));
        assert!(p.canal_feedback().is_some(), "link do canal de feedbacks");
    }
}
