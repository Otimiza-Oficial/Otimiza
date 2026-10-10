//! Provas do atestado com um par de chaves sorteado na hora, no mesmo formato que o bot (`otimiza-licenca.cjs`,
//! `atestarFeedback`) emite. Só em compilação de teste.

use super::*;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

fn par() -> (SigningKey, VerifyingKey) {
    let mut semente = [0u8; 32];
    getrandom::getrandom(&mut semente).expect("sortear semente");
    let privada = SigningKey::from_bytes(&semente);
    let publica = privada.verifying_key();
    (privada, publica)
}

fn assinar(privada: &SigningKey, corpo: serde_json::Value) -> String {
    let bytes = serde_json::to_vec(&corpo).unwrap();
    let url = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!(
        "{}.{}",
        url.encode(&bytes),
        url.encode(privada.sign(&bytes).to_bytes())
    )
}

fn agora() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-10-05T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn feedback(maquina: &str, ok: bool, quando: &str) -> serde_json::Value {
    serde_json::json!({
        "tipo": "feedback",
        "maquina": maquina,
        "ok": ok,
        "mensagem_id": "1300000000000000000",
        "verificado_em": quando,
    })
}

const MAQ: &str = "OTZ-WPYY-0J4F-77AB";

#[test]
fn atestado_do_bot_para_esta_maquina_e_aceito() {
    let (privada, publica) = par();
    let texto = assinar(&privada, feedback(MAQ, true, "2026-10-05T11:59:00Z"));

    let a = conferir_atestado_com(publica, &texto, MAQ, agora()).expect("tinha que valer");
    assert!(a.ok);
}

#[test]
fn atestado_de_outra_maquina_e_recusado() {
    let (privada, publica) = par();
    let texto = assinar(
        &privada,
        feedback("OTZ-ZZZZ-ZZZZ-ZZZZ", true, "2026-10-05T11:59:00Z"),
    );

    assert_eq!(
        conferir_atestado_com(publica, &texto, MAQ, agora()).unwrap_err(),
        RecusaDoAtestado::OutraMaquina
    );
}

#[test]
fn atestado_assinado_por_outra_chave_e_recusado() {
    let (privada, _) = par();
    let (_, outra_publica) = par();
    let texto = assinar(&privada, feedback(MAQ, true, "2026-10-05T11:59:00Z"));

    assert_eq!(
        conferir_atestado_com(outra_publica, &texto, MAQ, agora()).unwrap_err(),
        RecusaDoAtestado::Assinatura
    );
}

#[test]
fn trocar_ok_false_por_true_derruba_a_assinatura() {
    let (privada, publica) = par();
    let texto = assinar(&privada, feedback(MAQ, false, "2026-10-05T11:59:00Z"));
    let (dados, assinatura) = texto.split_once('.').unwrap();

    let url = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let mexido = String::from_utf8(url.decode(dados).unwrap())
        .unwrap()
        .replace("false", "true");
    let forjado = format!("{}.{}", url.encode(mexido), assinatura);

    assert_eq!(
        conferir_atestado_com(publica, &forjado, MAQ, agora()).unwrap_err(),
        RecusaDoAtestado::Assinatura
    );
}

#[test]
fn uma_chave_de_licenca_nao_passa_por_atestado() {
    // Mesmo par de chaves: sem conferir o `tipo`, a própria licença liberaria o feedback.
    let (privada, publica) = par();
    let licenca = assinar(
        &privada,
        serde_json::json!({"maquina": MAQ, "comprador": "x", "emitida": "2026-10-01", "expira": null}),
    );

    assert_eq!(
        conferir_atestado_com(publica, &licenca, MAQ, agora()).unwrap_err(),
        RecusaDoAtestado::NaoEFeedback
    );
}

#[test]
fn atestado_mais_velho_que_a_validade_vence() {
    let (privada, publica) = par();
    let velho = assinar(&privada, feedback(MAQ, true, "2026-09-27T12:00:00Z"));
    let dentro = assinar(&privada, feedback(MAQ, true, "2026-09-29T12:00:00Z"));

    assert_eq!(
        conferir_atestado_com(publica, &velho, MAQ, agora()).unwrap_err(),
        RecusaDoAtestado::Vencido
    );
    assert!(conferir_atestado_com(publica, &dentro, MAQ, agora()).is_ok());
}

#[test]
fn atestado_do_futuro_e_recusado() {
    let (privada, publica) = par();
    let futuro = assinar(&privada, feedback(MAQ, true, "2026-10-06T12:00:00Z"));
    let quase = assinar(&privada, feedback(MAQ, true, "2026-10-05T12:05:00Z"));

    assert_eq!(
        conferir_atestado_com(publica, &futuro, MAQ, agora()).unwrap_err(),
        RecusaDoAtestado::DataInvalida
    );
    assert!(
        conferir_atestado_com(publica, &quase, MAQ, agora()).is_ok(),
        "relógio alguns minutos adiantado passa"
    );
}

#[test]
fn data_sem_fuso_e_recusada() {
    let (privada, publica) = par();
    let texto = assinar(&privada, feedback(MAQ, true, "2026-10-05 11:00"));

    assert_eq!(
        conferir_atestado_com(publica, &texto, MAQ, agora()).unwrap_err(),
        RecusaDoAtestado::DataInvalida
    );
}

/// Emitido em 10/10/2026 pelo `atestarFeedback` do bot de verdade, com a privada do `.env` dele (lida no script, nunca
/// impressa), para uma máquina inventada. A pública de produção do Otimiza tem que aceitar.
const ATESTADO_DO_BOT: &str = "eyJ0aXBvIjoiZmVlZGJhY2siLCJtYXF1aW5hIjoiT1RaLVdFQkItMDAwMi1CQkJCIiwib2siOnRydWUsIm1lbnNhZ2VtX2lkIjoiNzcwMDAwMDAwMDAwMDAwMDAxIiwidmVyaWZpY2Fkb19lbSI6IjIwMjYtMTAtMTBUMTI6MDA6MDBaIn0.cadeCABCnSSXl5RDORBM8z4FxwBElsN6zyGlkkwuyGVNAF0xGEPTQktdKz8RrYovUcRb_y4aFjIKojEWnU1KCQ";

#[test]
fn o_atestado_do_bot_de_verdade_confere_com_a_publica_de_producao() {
    let agora = DateTime::parse_from_rfc3339("2026-10-10T13:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let a = conferir_atestado(ATESTADO_DO_BOT, "OTZ-WEBB-0002-BBBB", agora)
        .expect("o atestado do bot tinha que valer");
    assert!(a.ok);
    assert!(matches!(
        conferir_atestado(ATESTADO_DO_BOT, "OTZ-AAAA-BBBB-CCCC", agora),
        Err(RecusaDoAtestado::OutraMaquina)
    ));
    let depois = agora + Duration::days(8);
    assert!(matches!(
        conferir_atestado(ATESTADO_DO_BOT, "OTZ-WEBB-0002-BBBB", depois),
        Err(RecusaDoAtestado::Vencido)
    ));
}
