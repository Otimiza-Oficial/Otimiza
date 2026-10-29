// Atestado online da licença. A chave offline prova que alguém pagou, e só: não sabe se foi revogada, nem se está em
// cem máquinas com o `MachineGuid` copiado, nem se o executável que pergunta é o publicado. O bot devolve a quem pede
// com chave e máquina um atestado assinado com a MESMA privada e prazo curto; sem rede, o atestado guardado segura até
// vencer. O servidor é quem decide: recusa chave revogada, e vê a mesma chave chegando de impressões diferentes.
//
// Desligado na compilação sem `OTIMIZA_API_LICENCA`: aí o programa não fala com servidor nenhum e nada muda para o
// cliente. Ligado, renova e obedece revogação; exigir o atestado (sem ele, trancado) é a segunda etapa, `EXIGE_ATESTADO`.

use serde::{Deserialize, Serialize};

/// Segunda etapa: só depois de o bot responder em produção e de a primeira etapa mostrar que os clientes renovam.
pub const EXIGE_ATESTADO: bool = false;

/// Renova quando falta menos que isto para vencer: o bot dá 30 dias, então uma semana sem rede não tranca ninguém.
const RENOVAR_COM_DIAS: i64 = 23;

const TIPO: &str = "licenca-online";

/// O endereço da API do bot, fixado na compilação (o mesmo do `NEXT_PUBLIC_OTIMIZA_API` do site).
pub fn endereco() -> Option<&'static str> {
    endereco_de(option_env!("OTIMIZA_API_LICENCA"))
}

/// **Pura**: só `https://` liga (a chave do cliente não viaja em texto aberto).
pub fn endereco_de(bruto: Option<&str>) -> Option<&str> {
    bruto.map(str::trim).filter(|s| s.starts_with("https://"))
}

/// `{"tipo":"licenca-online","maquina","licenca","emitido","valido_ate"}`, assinado como a licença.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Atestado {
    pub tipo: String,
    pub maquina: String,
    /// SHA-256 da chave: o atestado de uma licença não vale para outra na mesma máquina.
    pub licenca: String,
    /// A data do servidor. O prazo é conferido pelo relógio do PC, que quem quer burlar atrasa: o atestado limita o
    /// repasse e a revogação, não quem mexe no próprio relógio.
    pub emitido: String,
    pub valido_ate: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Falta {
    Ausente,
    Invalido,
    Vencido { em: String },
    Revogada { motivo: String },
}

impl Falta {
    pub fn explicacao(&self) -> String {
        match self {
            Falta::Ausente | Falta::Invalido => "Esta licença precisa ser confirmada pela internet. Conecte o \
                 computador e abra o Otimiza de novo; depois disso ele funciona semanas sem rede."
                .to_string(),
            Falta::Vencido { em } => format!(
                "A confirmação desta licença venceu em {}. Conecte o computador à internet e abra o \
                 Otimiza de novo para renovar.",
                em
            ),
            Falta::Revogada { motivo } => format!(
                "Esta licença foi desativada ({}). Se você comprou e acha que é engano, fale no suporte com o \
                 código desta máquina.",
                motivo
            ),
        }
    }
}

/// A identidade da licença no atestado e na revogação: espaço de cópia não muda a chave.
pub fn impressao_da_licenca(chave: &str) -> String {
    let limpa: String = chave.chars().filter(|c| !c.is_whitespace()).collect();
    super::licenca::hex_sha256(limpa.as_bytes())
}

/// **Pura.** Assinatura antes de tudo; depois tipo, máquina, licença e prazo.
pub fn conferir(
    publica: ed25519_dalek::VerifyingKey,
    texto: &str,
    chave: &str,
    maquina: &str,
    hoje: &str,
) -> Result<Atestado, Falta> {
    let bytes = super::licenca::conferir_assinatura(publica, texto).map_err(|_| Falta::Invalido)?;
    let a: Atestado = serde_json::from_slice(&bytes).map_err(|_| Falta::Invalido)?;

    if a.tipo != TIPO || a.maquina != maquina || a.licenca != impressao_da_licenca(chave) {
        return Err(Falta::Invalido);
    }

    if hoje > a.valido_ate.as_str() {
        return Err(Falta::Vencido {
            em: a.valido_ate.clone(),
        });
    }

    Ok(a)
}

/// **Pura.** O que a política pede desta licença, já conferida offline. Revogação vale com o online ligado, mesmo antes
/// de o atestado ser exigido: é o que tira do ar uma chave que vazou.
pub fn politica(
    publica: ed25519_dalek::VerifyingKey,
    ligado: bool,
    exige: bool,
    guardada: &super::licenca::Guardada,
    maquina: &str,
    hoje: &str,
) -> Result<(), Falta> {
    if !ligado {
        return Ok(());
    }

    if let Some(r) = &guardada.revogada {
        if r.licenca == impressao_da_licenca(&guardada.chave) {
            return Err(Falta::Revogada {
                motivo: r.motivo.clone(),
            });
        }
    }

    if !exige {
        return Ok(());
    }

    let texto = guardada.atestado.as_deref().ok_or(Falta::Ausente)?;
    conferir(publica, texto, &guardada.chave, maquina, hoje).map(|_| ())
}

/// Precisa pedir outro? Sem atestado, inválido, ou a menos de `RENOVAR_COM_DIAS` do fim.
pub fn precisa_renovar(
    publica: ed25519_dalek::VerifyingKey,
    guardada: &super::licenca::Guardada,
    maquina: &str,
    hoje: chrono::NaiveDate,
) -> bool {
    let Some(texto) = guardada.atestado.as_deref() else {
        return true;
    };
    let hoje_txt = hoje.format("%Y-%m-%d").to_string();
    match conferir(publica, texto, &guardada.chave, maquina, &hoje_txt) {
        Ok(a) => chrono::NaiveDate::parse_from_str(&a.valido_ate, "%Y-%m-%d")
            .map(|fim| (fim - hoje).num_days() < RENOVAR_COM_DIAS)
            .unwrap_or(true),
        Err(_) => true,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Revogacao {
    pub licenca: String,
    pub motivo: String,
}

#[derive(Deserialize)]
struct RevogacaoAssinada {
    tipo: String,
    maquina: String,
    licenca: String,
    motivo: String,
}

/// **Pura.** A revogação tranca quem pagou, então só vale ASSINADA (`{"tipo":"revogacao","maquina","licenca",
/// "motivo"}`): um defeito do servidor ou de um proxy na frente dele não vira trava no cliente.
pub fn conferir_revogacao(
    publica: ed25519_dalek::VerifyingKey,
    texto: &str,
    chave: &str,
    maquina: &str,
) -> Option<Revogacao> {
    let bytes = super::licenca::conferir_assinatura(publica, texto).ok()?;
    let r: RevogacaoAssinada = serde_json::from_slice(&bytes).ok()?;
    let licenca = impressao_da_licenca(chave);
    (r.tipo == "revogacao" && r.maquina == maquina && r.licenca == licenca).then(|| Revogacao {
        licenca,
        motivo: r.motivo,
    })
}

#[derive(Serialize)]
struct Pedido<'a> {
    chave: &'a str,
    maquina: &'a str,
    versao: &'a str,
    /// SHA-256 do executável que pergunta: o servidor compara com os publicados, e um binário remendado aparece.
    executavel: Option<String>,
    /// De onde veio o código da máquina: `windows` é o `MachineGuid`, que se copia com um `.reg`.
    fonte: &'a str,
}

#[derive(Deserialize)]
struct Resposta {
    atestado: Option<String>,
    /// A revogação assinada (`emitirRevogacao` no bot).
    revogacao: Option<String>,
}

fn sha256_do_executavel() -> Option<String> {
    let caminho = std::env::current_exe().ok()?;
    let bytes = std::fs::read(caminho).ok()?;
    Some(super::licenca::hex_sha256(&bytes))
}

/// Pede um atestado novo quando precisa. Falha de rede é silêncio: o guardado continua valendo até vencer.
pub async fn renovar_se_preciso() {
    let Some(base) = endereco() else { return };
    let Ok(publica) = super::licenca::chave_publica() else {
        return;
    };

    let super::licenca::Leitura::Ok(guardada) = super::licenca::Guardada::load() else {
        return;
    };
    if guardada.chave.trim().is_empty() {
        return;
    }

    let quem = super::maquina::identidade();
    let hoje = chrono::Local::now().date_naive();
    let hoje_txt = hoje.format("%Y-%m-%d").to_string();

    // Chave que nem offline vale não vai ao servidor.
    if super::licenca::conferir(&guardada.chave, &quem.id, &hoje_txt).is_err() {
        return;
    }
    if !precisa_renovar(publica, &guardada, &quem.id, hoje) {
        return;
    }

    let executavel = tokio::task::spawn_blocking(sha256_do_executavel)
        .await
        .ok()
        .flatten();

    let pedido = Pedido {
        chave: guardada.chave.trim(),
        maquina: &quem.id,
        versao: env!("CARGO_PKG_VERSION"),
        executavel,
        fonte: match quem.fonte {
            super::maquina::Fonte::PlacaMae => "placa",
            super::maquina::Fonte::Windows => "windows",
            super::maquina::Fonte::Nenhuma => "nenhuma",
        },
    };

    let Ok(cliente) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return;
    };

    let Ok(resposta) = cliente
        .post(format!(
            "{}/v1/licencas/atestado",
            base.trim_end_matches('/')
        ))
        .json(&pedido)
        .send()
        .await
    else {
        return;
    };

    let status = resposta.status();
    let Ok(corpo) = resposta.json::<Resposta>().await else {
        return;
    };

    // Relida: o cliente pode ter colado outra chave enquanto a rede respondia, e gravar a cópia de antes a desfaria.
    let super::licenca::Leitura::Ok(mut nova) = super::licenca::Guardada::load() else {
        return;
    };
    if nova.chave.trim() != guardada.chave.trim() {
        return;
    }

    if let Some(r) = corpo
        .revogacao
        .as_deref()
        .and_then(|t| conferir_revogacao(publica, t, &guardada.chave, &quem.id))
    {
        nova.atestado = None;
        nova.revogada = Some(r);
    } else if let Some(texto) = corpo.atestado.filter(|_| status.is_success()) {
        // Conferido antes de gravar: resposta forjada no caminho não entra no disco.
        if conferir(publica, &texto, &guardada.chave, &quem.id, &hoje_txt).is_err() {
            crate::utils::Logger::warn("licença online: o atestado recebido não confere; ignorado");
            return;
        }
        nova.atestado = Some(texto);
        nova.revogada = None;
    } else {
        return;
    }

    if let Err(e) = nova.save() {
        crate::utils::Logger::warn(&format!("licença online: não consegui gravar: {}", e));
    }
}

#[cfg(test)]
#[path = "licenca_online_prova.rs"]
mod prova;
