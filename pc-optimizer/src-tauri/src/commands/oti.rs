// O Oti com IA (Plano 1.1): a pergunta que as respostas prontas da tela não entendem vai ao servidor do Otimiza, que
// fala com a API do Claude. Daqui saem a licença e o código da máquina (o servidor confere e conta o limite do dia), a
// pergunta, os FATOS que a tela juntou (números medidos e nomes de peças) e as últimas trocas. Nada de arquivo, nome de
// usuário ou caminho de disco. A resposta nunca muda o PC: no máximo, botões que abrem uma área do app.
//
// Desligado na compilação sem `OTIMIZA_API_LICENCA` (o mesmo endereço do atestado): aí responde que não está disponível.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct TrocaComOti {
    /// "pessoa" ou "oti".
    pub papel: String,
    pub texto: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct BotaoDoOti {
    pub rotulo: String,
    /// Uma área do app (`turbo`, `rede`…): o servidor só devolve as que existem.
    pub area: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct RespostaDoOti {
    pub resposta: String,
    pub humor: String,
    pub acoes: Vec<BotaoDoOti>,
    /// Perguntas que ainda cabem hoje nesta licença.
    pub restantes: Option<i64>,
}

#[derive(Serialize)]
struct Pedido<'a> {
    chave: &'a str,
    maquina: &'a str,
    pergunta: &'a str,
    fatos: &'a serde_json::Value,
    historico: &'a [TrocaComOti],
}

#[derive(Deserialize)]
struct Erro {
    erro: Option<String>,
    limite: Option<i64>,
}

/// **Pura**: o código de erro do servidor vira a frase que a pessoa lê.
pub fn frase_do_erro(status: u16, erro: Option<&str>, limite: Option<i64>) -> String {
    match (status, erro) {
        (429, Some("limite_do_dia")) => format!(
            "Você já fez as {} perguntas de hoje para a IA. Amanhã tem mais; até lá, as respostas prontas continuam.",
            limite.unwrap_or(20)
        ),
        (429, _) => "Muitas perguntas seguidas. Espere um minuto.".to_string(),
        (403, Some("licenca_revogada")) => "Esta licença foi desativada. Fale com o suporte.".to_string(),
        (403, _) => "A IA do Oti é para quem tem licença. Ative a sua nos Ajustes.".to_string(),
        (503, _) => "A IA do Oti não está ligada agora.".to_string(),
        _ => "Não consegui falar com a IA agora. Tente de novo daqui a pouco.".to_string(),
    }
}

/// `LIVRES`: não muda nada no PC (a licença é conferida no servidor, que recusa quem não tem).
#[tauri::command]
#[cfg_attr(test, specta::specta)]
pub async fn oti_perguntar(
    pergunta: String,
    // Os fatos em JSON (texto): conferido aqui, e nada além de 8 KB sai do PC.
    fatos: String,
    historico: Vec<TrocaComOti>,
) -> Result<RespostaDoOti, String> {
    let fatos: serde_json::Value = if fatos.len() <= 8192 {
        serde_json::from_str(&fatos).unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Null
    };
    let pergunta = pergunta.trim().to_string();
    if pergunta.chars().count() < 2 || pergunta.chars().count() > 500 {
        return Err("A pergunta precisa ter entre 2 e 500 letras.".to_string());
    }
    let Some(base) = crate::modules::licenca_online::endereco() else {
        return Err(frase_do_erro(503, None, None));
    };
    let crate::modules::licenca::Leitura::Ok(guardada) = crate::modules::licenca::Guardada::load() else {
        return Err(frase_do_erro(403, None, None));
    };
    if guardada.chave.trim().is_empty() {
        return Err(frase_do_erro(403, None, None));
    }
    let maquina = crate::modules::maquina::identidade().id;

    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| frase_do_erro(0, None, None))?;

    let historico: Vec<TrocaComOti> = historico.into_iter().rev().take(6).rev().collect();
    let resposta = cliente
        .post(format!("{}/v1/oti/pergunta", base.trim_end_matches('/')))
        .json(&Pedido {
            chave: guardada.chave.trim(),
            maquina: &maquina,
            pergunta: &pergunta,
            fatos: &fatos,
            historico: &historico,
        })
        .send()
        .await
        .map_err(|_| frase_do_erro(0, None, None))?;

    let status = resposta.status();
    if status.is_success() {
        return resposta
            .json::<RespostaDoOti>()
            .await
            .map_err(|_| frase_do_erro(0, None, None));
    }
    let erro = resposta.json::<Erro>().await.ok();
    Err(frase_do_erro(
        status.as_u16(),
        erro.as_ref().and_then(|e| e.erro.as_deref()),
        erro.as_ref().and_then(|e| e.limite),
    ))
}

#[cfg(test)]
mod testes {
    use super::frase_do_erro;

    #[test]
    fn cada_erro_do_servidor_vira_uma_frase_que_diz_o_que_fazer() {
        assert!(frase_do_erro(429, Some("limite_do_dia"), Some(20)).contains("20 perguntas"));
        assert!(frase_do_erro(403, Some("licenca_revogada"), None).contains("desativada"));
        assert!(frase_do_erro(403, Some("licenca_desconhecida"), None).contains("Ative"));
        assert!(frase_do_erro(503, None, None).contains("não está ligada"));
        assert!(frase_do_erro(502, None, None).contains("Tente de novo"));
    }
}
