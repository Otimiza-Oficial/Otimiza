// O pedido de feedback (regra do dono, 3.3): quem pagou usa 10 minutos à vontade; depois disso as CORREÇÕES pausam até
// haver um feedback dele no canal de feedbacks do Discord. Medir e ler o PC continuam livres, e desfazer nunca pausa.
//
// Quem lê o canal é o bot, não o programa: um token de bot aqui dentro vazaria no primeiro instalador aberto. O bot
// responde `GET {api}/v1/feedback/{código da máquina}` com um ATESTADO assinado pela mesma chave da licença:
// `{"tipo":"feedback","maquina","ok","mensagem_id","verificado_em"}`. O programa confere a assinatura antes de ler
// qualquer campo. `ok:false` quer dizer que a mensagem foi apagada, e o pedido volta.
//
// Sem rede o atestado bom vale 7 dias: ficar offline um fim de semana não pode trancar quem já mandou o feedback.
// Sem `api` publicada em `.github/convite.json` o pedido fica DESLIGADO: não haveria como liberar ninguém.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 10 minutos de uso com a licença ativa (decisão do dono).
pub const MINUTOS_LIVRES: u64 = 10;
pub const SEGUNDOS_LIVRES: u64 = MINUTOS_LIVRES * 60;

/// O canal de feedbacks do servidor do Otimiza. Só informativo aqui: quem confere é o bot.
pub const CANAL_ID: &str = "1543436859529629707";

/// Por quanto tempo um atestado bom vale sem rechecar.
pub const VALIDADE_DIAS: i64 = 7;

/// Rechecagem de fundo: no máximo uma por dia, para não pesar no bot nem na abertura.
pub const RECHECAR_HORAS: i64 = 24;

/// Tolerância para relógio adiantado no bot ou atrasado aqui.
const FUTURO_TOLERADO_MIN: i64 = 10;

const TIMEOUT_SEGUNDOS: u64 = 5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atestado {
    pub tipo: String,
    pub maquina: String,
    pub ok: bool,
    #[serde(default)]
    pub mensagem_id: Option<String>,
    pub verificado_em: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecusaDoAtestado {
    Assinatura,
    NaoEFeedback,
    OutraMaquina,
    DataInvalida,
    Vencido,
}

impl RecusaDoAtestado {
    pub fn explicacao(&self) -> &'static str {
        match self {
            RecusaDoAtestado::Assinatura => {
                "A resposta do servidor não foi assinada pelo Otimiza. Tente de novo em alguns minutos."
            }
            RecusaDoAtestado::NaoEFeedback => "A resposta do servidor não é sobre feedback.",
            RecusaDoAtestado::OutraMaquina => "A resposta do servidor é sobre outro computador.",
            RecusaDoAtestado::DataInvalida => {
                "A data da resposta não confere. Veja se o relógio do Windows está certo."
            }
            RecusaDoAtestado::Vencido => "A última confirmação venceu e precisa ser renovada.",
        }
    }
}

/// Pura: assinatura, tipo, máquina e prazo. A assinatura vem primeiro, antes de qualquer campo ser lido.
pub fn conferir_atestado_com(
    publica: ed25519_dalek::VerifyingKey,
    texto: &str,
    maquina: &str,
    agora: DateTime<Utc>,
) -> Result<Atestado, RecusaDoAtestado> {
    let bytes = crate::modules::licenca::conferir_assinatura(publica, texto)
        .map_err(|_| RecusaDoAtestado::Assinatura)?;

    let atestado: Atestado =
        serde_json::from_slice(&bytes).map_err(|_| RecusaDoAtestado::NaoEFeedback)?;

    // Sem isto uma chave de licença (mesmo par) passaria por atestado.
    if atestado.tipo != "feedback" {
        return Err(RecusaDoAtestado::NaoEFeedback);
    }

    if atestado.maquina != maquina {
        return Err(RecusaDoAtestado::OutraMaquina);
    }

    let quando = DateTime::parse_from_rfc3339(&atestado.verificado_em)
        .map_err(|_| RecusaDoAtestado::DataInvalida)?
        .with_timezone(&Utc);

    if quando > agora + Duration::minutes(FUTURO_TOLERADO_MIN) {
        return Err(RecusaDoAtestado::DataInvalida);
    }

    if agora - quando > Duration::days(VALIDADE_DIAS) {
        return Err(RecusaDoAtestado::Vencido);
    }

    Ok(atestado)
}

pub fn conferir_atestado(
    texto: &str,
    maquina: &str,
    agora: DateTime<Utc>,
) -> Result<Atestado, RecusaDoAtestado> {
    let publica =
        crate::modules::licenca::chave_publica().map_err(|_| RecusaDoAtestado::Assinatura)?;
    conferir_atestado_com(publica, texto, maquina, agora)
}

/// O que fica no disco. O contador não é segredo: zerá-lo à mão só adia o pedido por mais dez minutos.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Guardado {
    pub uso_segundos: u64,
    /// O texto assinado, como veio do bot: conferido de novo a cada leitura, nunca confiado por estar no disco.
    pub atestado: Option<String>,
    /// Última vez que perguntamos ao bot (RFC 3339).
    pub checado_em: Option<String>,
    /// A última `api` publicada que conhecemos: sem ela o pedido não liga.
    pub api: Option<String>,
    pub canal: Option<String>,
}

impl Guardado {
    fn path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("feedback.json")
    }

    pub fn load() -> Self {
        Self::ler(&Self::path())
    }

    fn ler(caminho: &Path) -> Self {
        fs::read_to_string(caminho)
            .ok()
            .and_then(|bruto| serde_json::from_str(&bruto).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        Self::gravar(&Self::path(), self)
    }

    fn gravar(caminho: &Path, g: &Guardado) -> Result<(), String> {
        if let Some(pasta) = caminho.parent() {
            fs::create_dir_all(pasta).map_err(|e| format!("Falha ao criar a pasta: {}", e))?;
        }

        let json = serde_json::to_string_pretty(g).map_err(|e| e.to_string())?;
        let temporario = crate::modules::changelog::caminho_temporario(caminho);
        fs::write(&temporario, json).map_err(|e| e.to_string())?;
        fs::rename(&temporario, caminho).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "situacao")]
pub enum Situacao {
    /// Sem licença (o portão de compra cuida) ou sem `api` publicada.
    NaoSeAplica,
    Contando {
        restante_segundos: u64,
    },
    Liberado {
        verificado_em: String,
    },
    /// `apagado`: havia feedback e o bot avisou que a mensagem sumiu.
    Exigido {
        apagado: bool,
    },
}

/// Pura. A ordem importa: atestado bom libera mesmo dentro dos dez minutos; atestado de mensagem apagada só pesa
/// depois deles (quem apagou e reinstalou ainda tem os minutos de quem nunca mandou).
pub fn decidir(
    licenca_ativa: bool,
    pedido_ligado: bool,
    uso_segundos: u64,
    atestado: Option<&Atestado>,
) -> Situacao {
    if !licenca_ativa || !pedido_ligado {
        return Situacao::NaoSeAplica;
    }

    if let Some(a) = atestado.filter(|a| a.ok) {
        return Situacao::Liberado {
            verificado_em: a.verificado_em.clone(),
        };
    }

    if uso_segundos < SEGUNDOS_LIVRES {
        return Situacao::Contando {
            restante_segundos: SEGUNDOS_LIVRES - uso_segundos,
        };
    }

    Situacao::Exigido {
        apagado: atestado.is_some_and(|a| !a.ok),
    }
}

fn maquina() -> String {
    crate::modules::maquina::identidade().id
}

/// O atestado guardado, conferido de novo agora. Vencido ou adulterado é como não existir.
fn atestado_valido(g: &Guardado, maquina: &str, agora: DateTime<Utc>) -> Option<Atestado> {
    conferir_atestado(g.atestado.as_deref()?, maquina, agora).ok()
}

pub fn situacao() -> Situacao {
    let g = Guardado::load();
    let quem = maquina();
    let atestado = atestado_valido(&g, &quem, Utc::now());

    decidir(
        crate::modules::licenca::liberado(),
        g.api.is_some(),
        g.uso_segundos,
        atestado.as_ref(),
    )
}

/// Chamada por `licenca::exigir` depois da licença: a mesma guarda dos comandos que alteram o PC.
pub fn exigir() -> Result<(), String> {
    match situacao() {
        Situacao::Exigido { .. } => Err(
            "As correções estão pausadas até chegar o seu feedback no canal de feedbacks do Discord. \
             Medir e ver o diagnóstico continuam livres, e desfazer o que já foi aplicado também."
                .to_string(),
        ),
        _ => Ok(()),
    }
}

/// Soma o tempo de uso. Só conta com licença ativa e enquanto ainda não liberou: depois disso o número não importa.
pub fn somar_uso(segundos: u64) -> Option<Situacao> {
    if !crate::modules::licenca::liberado() {
        return None;
    }

    let mut g = Guardado::load();
    let quem = maquina();
    let agora = Utc::now();
    let antes = decidir(
        true,
        g.api.is_some(),
        g.uso_segundos,
        atestado_valido(&g, &quem, agora).as_ref(),
    );

    if matches!(antes, Situacao::Contando { .. }) || g.api.is_none() {
        g.uso_segundos = g.uso_segundos.saturating_add(segundos);
        let _ = g.save();
    }

    let depois = decidir(
        true,
        g.api.is_some(),
        g.uso_segundos,
        atestado_valido(&g, &quem, agora).as_ref(),
    );
    Some(depois)
}

/// O que o bot respondeu, já interpretado.
#[derive(Debug, Clone, PartialEq)]
pub enum Resposta {
    /// Corpo de um 200: o texto assinado.
    Atestado(String),
    /// 404: nenhum feedback desta máquina no canal.
    NaoEncontrado,
    SemRede(String),
}

pub fn interpretar(status: u16, corpo: String) -> Resposta {
    match status {
        200 => Resposta::Atestado(corpo.trim().to_string()),
        404 => Resposta::NaoEncontrado,
        outro => Resposta::SemRede(format!("o servidor respondeu {}", outro)),
    }
}

async fn perguntar_ao_bot(api: &str, maquina: &str) -> Resposta {
    let Ok(cliente) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SEGUNDOS))
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return Resposta::SemRede("não consegui preparar a conexão".to_string());
    };

    // Só o código da máquina viaja: é o mesmo que o cliente já mandou ao bot ao comprar.
    let url = format!("{}/v1/feedback/{}", api, maquina);

    match cliente.get(url).send().await {
        Ok(r) => {
            let status = r.status().as_u16();
            interpretar(status, r.text().await.unwrap_or_default())
        }
        Err(_) => Resposta::SemRede("sem conexão com o servidor".to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estado {
    #[serde(flatten)]
    pub situacao: Situacao,
    pub maquina: String,
    pub canal: Option<String>,
    pub canal_id: String,
    /// Frase da última verificação pedida pelo cliente, quando houve.
    pub aviso: Option<String>,
}

pub fn estado(aviso: Option<String>) -> Estado {
    let g = Guardado::load();

    Estado {
        situacao: situacao(),
        maquina: maquina(),
        canal: g.canal.clone(),
        canal_id: CANAL_ID.to_string(),
        aviso,
    }
}

/// Atualiza `api` e `canal` a partir do arquivo publicado. Falha de rede mantém o que já se conhecia.
pub async fn atualizar_configuracao() {
    let Some(publicado) = crate::modules::convite::consultar_publicado().await else {
        return;
    };

    let mut g = Guardado::load();
    let api = publicado.api();
    let canal = publicado.canal_feedback();

    if g.api != api || g.canal != canal {
        g.api = api;
        g.canal = canal;
        let _ = g.save();
    }
}

/// "Já enviei" ou a rechecagem diária. Devolve o estado e uma frase para a tela.
pub async fn verificar() -> Estado {
    let mut g = Guardado::load();

    let Some(api) = g.api.clone() else {
        return estado(None);
    };

    let quem = maquina();
    let agora = Utc::now();

    let aviso = match perguntar_ao_bot(&api, &quem).await {
        Resposta::Atestado(texto) => match conferir_atestado(&texto, &quem, agora) {
            Ok(a) => {
                g.atestado = Some(texto);
                g.checado_em = Some(agora.to_rfc3339());
                let _ = g.save();

                if a.ok {
                    None
                } else {
                    Some("A sua mensagem não está mais no canal. Mande o feedback de novo para liberar.".to_string())
                }
            }
            Err(recusa) => Some(recusa.explicacao().to_string()),
        },
        Resposta::NaoEncontrado => {
            g.checado_em = Some(agora.to_rfc3339());
            let _ = g.save();
            Some(
                "Ainda não achamos a sua mensagem no canal. Ela precisa ter pelo menos 30 caracteres e um print \
                 mostrando o FPS. Pode levar até um minuto para o bot ver."
                    .to_string(),
            )
        }
        Resposta::SemRede(motivo) => Some(format!(
            "Não deu para conferir agora ({}). Tente de novo.",
            motivo
        )),
    };

    estado(aviso)
}

/// Na abertura: só pergunta se já passou dos minutos livres e se a última pergunta tem mais de um dia.
pub fn precisa_rechecar(g: &Guardado, agora: DateTime<Utc>) -> bool {
    if g.api.is_none() || g.uso_segundos < SEGUNDOS_LIVRES {
        return false;
    }

    match g
        .checado_em
        .as_deref()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
    {
        Some(quando) => agora - quando.with_timezone(&Utc) > Duration::hours(RECHECAR_HORAS),
        None => true,
    }
}

pub async fn rechecar_na_abertura() {
    atualizar_configuracao().await;

    if precisa_rechecar(&Guardado::load(), Utc::now()) {
        let _ = verificar().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atestado(ok: bool) -> Atestado {
        Atestado {
            tipo: "feedback".to_string(),
            maquina: "OTZ-AAAA-BBBB-CCCC".to_string(),
            ok,
            mensagem_id: Some("1".to_string()),
            verificado_em: "2026-10-05T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn sem_licenca_o_pedido_nao_se_aplica() {
        assert_eq!(decidir(false, true, 99_999, None), Situacao::NaoSeAplica);
    }

    #[test]
    fn sem_api_publicada_ninguem_fica_trancado() {
        assert_eq!(decidir(true, false, 99_999, None), Situacao::NaoSeAplica);
    }

    #[test]
    fn os_dez_minutos_contam_antes_do_pedido() {
        assert_eq!(
            decidir(true, true, 0, None),
            Situacao::Contando {
                restante_segundos: 600
            }
        );
        assert_eq!(
            decidir(true, true, 599, None),
            Situacao::Contando {
                restante_segundos: 1
            }
        );
        assert_eq!(
            decidir(true, true, 600, None),
            Situacao::Exigido { apagado: false }
        );
    }

    #[test]
    fn atestado_bom_libera_mesmo_dentro_dos_dez_minutos() {
        assert!(matches!(
            decidir(true, true, 10, Some(&atestado(true))),
            Situacao::Liberado { .. }
        ));
        assert!(matches!(
            decidir(true, true, 9_000, Some(&atestado(true))),
            Situacao::Liberado { .. }
        ));
    }

    #[test]
    fn mensagem_apagada_volta_a_pedir_e_diz_por_que() {
        assert_eq!(
            decidir(true, true, 900, Some(&atestado(false))),
            Situacao::Exigido { apagado: true }
        );
    }

    #[test]
    fn mensagem_apagada_ainda_respeita_os_minutos_livres() {
        assert!(matches!(
            decidir(true, true, 100, Some(&atestado(false))),
            Situacao::Contando { .. }
        ));
    }

    #[test]
    fn interpreta_as_respostas_do_bot() {
        assert_eq!(
            interpretar(200, " abc.def \n".to_string()),
            Resposta::Atestado("abc.def".to_string())
        );
        assert_eq!(interpretar(404, String::new()), Resposta::NaoEncontrado);
        assert!(matches!(
            interpretar(500, String::new()),
            Resposta::SemRede(_)
        ));
    }

    #[test]
    fn rechecagem_so_depois_dos_minutos_e_uma_vez_por_dia() {
        let agora = DateTime::parse_from_rfc3339("2026-10-05T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let base = Guardado {
            api: Some("https://bot.exemplo.com".to_string()),
            ..Guardado::default()
        };

        assert!(!precisa_rechecar(
            &Guardado {
                uso_segundos: 100,
                ..base.clone()
            },
            agora
        ));
        assert!(precisa_rechecar(
            &Guardado {
                uso_segundos: 600,
                ..base.clone()
            },
            agora
        ));
        assert!(!precisa_rechecar(
            &Guardado {
                uso_segundos: 600,
                checado_em: Some("2026-10-05T01:00:00Z".to_string()),
                ..base.clone()
            },
            agora
        ));
        assert!(precisa_rechecar(
            &Guardado {
                uso_segundos: 600,
                checado_em: Some("2026-10-03T01:00:00Z".to_string()),
                ..base.clone()
            },
            agora
        ));
        assert!(!precisa_rechecar(
            &Guardado {
                uso_segundos: 600,
                ..Guardado::default()
            },
            agora
        ));
    }

    #[test]
    fn arquivo_corrompido_vira_padrao_e_nao_libera_nada() {
        let dir = std::env::temp_dir().join("otimiza-feedback-corrompido");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let caminho = dir.join("feedback.json");
        fs::write(&caminho, "{quebrado").unwrap();

        assert_eq!(Guardado::ler(&caminho), Guardado::default());
    }

    #[test]
    fn gravar_e_ler_devolve_o_mesmo() {
        let dir = std::env::temp_dir().join("otimiza-feedback-ida-e-volta");
        let _ = fs::remove_dir_all(&dir);
        let caminho = dir.join("feedback.json");
        let g = Guardado {
            uso_segundos: 42,
            api: Some("https://a.b".to_string()),
            ..Guardado::default()
        };

        Guardado::gravar(&caminho, &g).unwrap();
        assert_eq!(Guardado::ler(&caminho), g);
    }

    #[test]
    fn o_canal_e_o_que_o_dono_pediu() {
        assert_eq!(CANAL_ID, "1543436859529629707");
        assert_eq!(SEGUNDOS_LIVRES, 600);
    }
}

#[cfg(test)]
#[path = "feedback_prova.rs"]
mod prova;
