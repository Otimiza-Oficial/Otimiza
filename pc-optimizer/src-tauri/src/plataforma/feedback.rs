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
//
// Erro NOSSO não trava o cliente (visto em 21/11/2026: a API do bot desligada respondia 503 e o cliente que já tinha
// mandado o feedback ficava com as correções pausadas). Se o "Já mandei" esbarra em rede ou servidor fora do ar, o
// pedido fica PROVISÓRIO: as correções seguem liberadas e o programa confere sozinho a cada 15 minutos. A resposta de
// verdade do bot decide (achou: liberado; não achou: volta o pedido). O provisório vale 7 dias, abre uma vez só e
// não se renova enquanto o bot não responder: bloquear o endereço no firewall não vira passe livre.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 10 minutos de uso com a licença ativa (decisão do dono).
pub const MINUTOS_LIVRES: u64 = 10;
pub const SEGUNDOS_LIVRES: u64 = MINUTOS_LIVRES * 60;

/// O canal de feedbacks do servidor do Otimiza. Só informativo aqui: quem confere é o bot.
pub const CANAL_ID: &str = "1563224882731290786";

/// Por quanto tempo um atestado bom vale sem rechecar.
pub const VALIDADE_DIAS: i64 = 7;

/// Rechecagem de fundo: no máximo uma por dia, para não pesar no bot nem na abertura.
pub const RECHECAR_HORAS: i64 = 24;

/// Com o pedido provisório, o programa volta a perguntar ao bot neste intervalo, em segundo plano.
pub const RECHECAR_PROVISORIO_MIN: i64 = 15;

/// Por quanto tempo o "Já mandei" vale sem o bot confirmar (o mesmo prazo de um atestado sem rede).
pub const PROVISORIO_DIAS: i64 = VALIDADE_DIAS;

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
    /// Quando o cliente disse "Já mandei" e o NOSSO servidor não respondeu (RFC 3339). Só a resposta do bot limpa.
    pub provisorio_desde: Option<String>,
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
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "SituacaoFeedback"))]
pub enum Situacao {
    /// Sem licença (o portão de compra cuida) ou sem `api` publicada.
    NaoSeAplica,
    Contando {
        restante_segundos: u64,
    },
    Liberado {
        verificado_em: String,
    },
    /// O cliente disse "Já mandei" e o nosso servidor não respondeu: correções liberadas enquanto conferimos sozinhos.
    Provisorio {
        desde: String,
    },
    /// `apagado`: havia feedback e o bot avisou que a mensagem sumiu.
    Exigido {
        apagado: bool,
    },
}

/// Pura. A ordem importa: atestado bom libera mesmo dentro dos dez minutos; o provisório vale por cima de um atestado
/// de mensagem apagada (o cliente mandou de novo e o servidor caiu); atestado de mensagem apagada só pesa depois dos
/// minutos livres (quem apagou e reinstalou ainda tem os minutos de quem nunca mandou).
pub fn decidir(
    licenca_ativa: bool,
    pedido_ligado: bool,
    uso_segundos: u64,
    atestado: Option<&Atestado>,
    provisorio: Option<&str>,
) -> Situacao {
    if !licenca_ativa || !pedido_ligado {
        return Situacao::NaoSeAplica;
    }

    if let Some(a) = atestado.filter(|a| a.ok) {
        return Situacao::Liberado {
            verificado_em: a.verificado_em.clone(),
        };
    }

    if let Some(desde) = provisorio {
        return Situacao::Provisorio {
            desde: desde.to_string(),
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

/// Pura: o provisório guardado, se ainda vale agora. Data quebrada, no futuro ou vencida não vale.
pub fn provisorio_valido(g: &Guardado, agora: DateTime<Utc>) -> Option<&str> {
    let texto = g.provisorio_desde.as_deref()?;
    let desde = DateTime::parse_from_rfc3339(texto).ok()?.with_timezone(&Utc);

    if desde > agora + Duration::minutes(FUTURO_TOLERADO_MIN)
        || agora - desde > Duration::days(PROVISORIO_DIAS)
    {
        return None;
    }

    Some(texto)
}

pub fn situacao() -> Situacao {
    let g = Guardado::load();
    let quem = maquina();
    let agora = Utc::now();
    let atestado = atestado_valido(&g, &quem, agora);

    decidir(
        crate::modules::licenca::liberado(),
        g.api.is_some(),
        g.uso_segundos,
        atestado.as_ref(),
        provisorio_valido(&g, agora),
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
        provisorio_valido(&g, agora),
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
        provisorio_valido(&g, agora),
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

/// 404 só é "sem feedback" quando é o BOT que diz (`{"erro":"sem_feedback"}`). A hospedagem também responde 404 com
/// a página dela quando o endereço da API não está no ar (visto em 10/11/2026: o cliente mandava o feedback, o bot
/// reagia com ✅, e o Otimiza dizia "ainda não achamos a sua mensagem").
pub fn interpretar(status: u16, corpo: String) -> Resposta {
    match status {
        200 => Resposta::Atestado(corpo.trim().to_string()),
        404 if corpo.contains("sem_feedback") => Resposta::NaoEncontrado,
        404 => Resposta::SemRede("o servidor de feedback está fora do ar".to_string()),
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
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "EstadoFeedback"))]
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
                g.provisorio_desde = None;
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
            g.provisorio_desde = None;
            let _ = g.save();
            Some(
                "Ainda não achamos a sua mensagem no canal. Ela precisa ter pelo menos 30 caracteres e um print \
                 mostrando o FPS. Pode levar até um minuto para o bot ver."
                    .to_string(),
            )
        }
        Resposta::SemRede(motivo) => {
            aceitar_provisorio(&mut g, agora);
            let _ = g.save();
            Some(aviso_sem_rede(&motivo, provisorio_valido(&g, agora).is_some()))
        }
    };

    estado(aviso)
}

/// Pura: o "Já mandei" (ou a rechecagem) que esbarrou no nosso servidor. Abre o provisório uma vez só: se já existe,
/// valendo ou vencido, não renova. Marca a tentativa em `checado_em` para a rechecagem de fundo esperar o intervalo.
pub fn aceitar_provisorio(g: &mut Guardado, agora: DateTime<Utc>) {
    if g.provisorio_desde.is_none() {
        g.provisorio_desde = Some(agora.to_rfc3339());
    }
    g.checado_em = Some(agora.to_rfc3339());
}

/// A frase quando o servidor não respondeu: deixa claro que o problema é nosso, não do cliente.
pub fn aviso_sem_rede(motivo: &str, liberado: bool) -> String {
    if liberado {
        format!(
            "O problema é do nosso lado ({}), não seu. As correções continuam liberadas e o Otimiza confere o seu \
             feedback sozinho assim que o servidor voltar.",
            motivo
        )
    } else {
        format!(
            "O problema é do nosso lado ({}), não seu. Faz dias que o servidor não confirma o seu feedback: tente de \
             novo mais tarde ou chame a gente no Discord.",
            motivo
        )
    }
}

/// Na abertura: só pergunta se já passou dos minutos livres e se a última pergunta tem mais de um dia. Com o pedido
/// provisório, pergunta a cada 15 minutos, mesmo dentro dos minutos livres: é o "conferimos sozinhos" prometido.
pub fn precisa_rechecar(g: &Guardado, agora: DateTime<Utc>) -> bool {
    let provisorio = g.provisorio_desde.is_some();

    if g.api.is_none() || (g.uso_segundos < SEGUNDOS_LIVRES && !provisorio) {
        return false;
    }

    let intervalo = if provisorio {
        Duration::minutes(RECHECAR_PROVISORIO_MIN)
    } else {
        Duration::hours(RECHECAR_HORAS)
    };

    match g
        .checado_em
        .as_deref()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
    {
        Some(quando) => agora - quando.with_timezone(&Utc) > intervalo,
        None => true,
    }
}

pub async fn rechecar_na_abertura() {
    atualizar_configuracao().await;

    if precisa_rechecar(&Guardado::load(), Utc::now()) {
        let _ = verificar().await;
    }
}

/// A cada 30 s, junto com a contagem de uso: só pergunta ao bot com o pedido provisório e o intervalo vencido.
pub async fn rechecar_provisorio() {
    let g = Guardado::load();

    if g.provisorio_desde.is_some() && precisa_rechecar(&g, Utc::now()) {
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
        assert_eq!(decidir(false, true, 99_999, None, None), Situacao::NaoSeAplica);
    }

    #[test]
    fn sem_api_publicada_ninguem_fica_trancado() {
        assert_eq!(decidir(true, false, 99_999, None, None), Situacao::NaoSeAplica);
    }

    #[test]
    fn os_dez_minutos_contam_antes_do_pedido() {
        assert_eq!(
            decidir(true, true, 0, None, None),
            Situacao::Contando {
                restante_segundos: 600
            }
        );
        assert_eq!(
            decidir(true, true, 599, None, None),
            Situacao::Contando {
                restante_segundos: 1
            }
        );
        assert_eq!(
            decidir(true, true, 600, None, None),
            Situacao::Exigido { apagado: false }
        );
    }

    #[test]
    fn atestado_bom_libera_mesmo_dentro_dos_dez_minutos() {
        assert!(matches!(
            decidir(true, true, 10, Some(&atestado(true)), None),
            Situacao::Liberado { .. }
        ));
        assert!(matches!(
            decidir(true, true, 9_000, Some(&atestado(true)), None),
            Situacao::Liberado { .. }
        ));
    }

    #[test]
    fn mensagem_apagada_volta_a_pedir_e_diz_por_que() {
        assert_eq!(
            decidir(true, true, 900, Some(&atestado(false)), None),
            Situacao::Exigido { apagado: true }
        );
    }

    #[test]
    fn mensagem_apagada_ainda_respeita_os_minutos_livres() {
        assert!(matches!(
            decidir(true, true, 100, Some(&atestado(false)), None),
            Situacao::Contando { .. }
        ));
    }

    #[test]
    fn interpreta_as_respostas_do_bot() {
        assert_eq!(
            interpretar(200, " abc.def \n".to_string()),
            Resposta::Atestado("abc.def".to_string())
        );
        assert_eq!(interpretar(404, r#"{"erro":"sem_feedback"}"#.to_string()), Resposta::NaoEncontrado);
        // A página de "site não encontrado" da hospedagem também é 404: não é "sem feedback".
        assert!(matches!(
            interpretar(404, "<!doctype html><title>Square Cloud | This site couldn't be found.</title>".to_string()),
            Resposta::SemRede(_)
        ));
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

    fn instante(t: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(t).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn servidor_fora_do_ar_nao_trava_quem_disse_ja_mandei() {
        // 21/11/2026: a API do bot desligada respondia 503 e o cliente ficava com as correções pausadas.
        assert!(matches!(interpretar(503, String::new()), Resposta::SemRede(_)));

        let agora = instante("2026-11-21T12:00:00Z");
        let mut g = Guardado {
            uso_segundos: 900,
            api: Some("https://bot.exemplo.com".to_string()),
            ..Guardado::default()
        };
        assert_eq!(
            decidir(true, true, g.uso_segundos, None, provisorio_valido(&g, agora)),
            Situacao::Exigido { apagado: false }
        );

        aceitar_provisorio(&mut g, agora);
        assert!(matches!(
            decidir(true, true, g.uso_segundos, None, provisorio_valido(&g, agora)),
            Situacao::Provisorio { .. }
        ));
    }

    #[test]
    fn provisorio_vale_por_cima_de_mensagem_apagada_mas_atestado_bom_manda() {
        let p = Some("2026-11-21T12:00:00Z");
        assert!(matches!(
            decidir(true, true, 900, Some(&atestado(false)), p),
            Situacao::Provisorio { .. }
        ));
        assert!(matches!(
            decidir(true, true, 900, Some(&atestado(true)), p),
            Situacao::Liberado { .. }
        ));
        assert_eq!(decidir(false, true, 900, None, p), Situacao::NaoSeAplica);
    }

    #[test]
    fn provisorio_vence_em_sete_dias_e_nao_se_renova() {
        let inicio = instante("2026-11-21T12:00:00Z");
        let mut g = Guardado::default();
        aceitar_provisorio(&mut g, inicio);
        assert!(provisorio_valido(&g, inicio + Duration::days(6)).is_some());

        let depois = inicio + Duration::days(8);
        assert!(provisorio_valido(&g, depois).is_none());

        // Outro "Já mandei" com o servidor ainda fora não reabre: só a resposta do bot limpa o provisório.
        aceitar_provisorio(&mut g, depois);
        assert_eq!(g.provisorio_desde.as_deref(), Some(inicio.to_rfc3339().as_str()));
        assert!(provisorio_valido(&g, depois).is_none());
        assert_eq!(
            decidir(true, true, 900, None, provisorio_valido(&g, depois)),
            Situacao::Exigido { apagado: false }
        );
    }

    #[test]
    fn provisorio_com_data_quebrada_ou_no_futuro_nao_vale() {
        let agora = instante("2026-11-21T12:00:00Z");
        let quebrado = Guardado {
            provisorio_desde: Some("ontem".to_string()),
            ..Guardado::default()
        };
        let futuro = Guardado {
            provisorio_desde: Some("2026-11-22T12:00:00Z".to_string()),
            ..Guardado::default()
        };
        assert!(provisorio_valido(&quebrado, agora).is_none());
        assert!(provisorio_valido(&futuro, agora).is_none());
    }

    #[test]
    fn com_provisorio_a_rechecagem_de_fundo_e_a_cada_quinze_minutos() {
        let agora = instante("2026-11-21T12:00:00Z");
        let g = Guardado {
            uso_segundos: 100,
            api: Some("https://bot.exemplo.com".to_string()),
            provisorio_desde: Some("2026-11-21T11:00:00Z".to_string()),
            checado_em: Some("2026-11-21T11:50:00Z".to_string()),
            ..Guardado::default()
        };
        assert!(!precisa_rechecar(&g, agora), "10 minutos atrás: ainda não");
        assert!(precisa_rechecar(
            &Guardado {
                checado_em: Some("2026-11-21T11:40:00Z".to_string()),
                ..g.clone()
            },
            agora
        ));
    }

    #[test]
    fn a_frase_diz_que_o_problema_e_nosso() {
        let liberado = aviso_sem_rede("o servidor respondeu 503", true);
        assert!(liberado.contains("nosso lado") && liberado.contains("não seu") && liberado.contains("503"));
        assert!(liberado.contains("continuam liberadas"));
        assert!(aviso_sem_rede("x", false).contains("nosso lado"));
    }

    #[test]
    fn guardado_antigo_sem_provisorio_continua_lendo() {
        let antigo = r#"{"uso_segundos":700,"atestado":null,"checado_em":null,"api":"https://a.b","canal":null}"#;
        let g: Guardado = serde_json::from_str(antigo).unwrap();
        assert_eq!(g.provisorio_desde, None);
        assert_eq!(g.uso_segundos, 700);
    }

    #[test]
    fn o_canal_e_o_que_o_dono_pediu() {
        // O #feedbacks de hoje (o 1543436859529629707 não existe mais, conferido no Discord em 10/11/2026).
        assert_eq!(CANAL_ID, "1563224882731290786");
        assert_eq!(SEGUNDOS_LIVRES, 600);
    }
}

#[cfg(test)]
#[path = "feedback_prova.rs"]
mod prova;
