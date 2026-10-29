//! Provas do atestado online e das travas da chave anterior, com par sorteado na hora: nada aqui depende da chave de
//! produção, da rede nem da máquina de quem roda.

use super::*;
use crate::modules::licenca::{conferir_anterior, resumo_da_maquina, Guardada, Recusa};
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

const M: &str = "OTZ-WPYY-0J4F-77AB";
const CHAVE: &str = "eyJsaWNlbmNhIjoxfQ.YXNzaW5hdHVyYQ";

fn par() -> (SigningKey, VerifyingKey) {
    let mut semente = [0u8; 32];
    getrandom::getrandom(&mut semente).expect("sortear semente");
    let privada = SigningKey::from_bytes(&semente);
    let publica = privada.verifying_key();
    (privada, publica)
}

fn assinar(privada: &SigningKey, dados: serde_json::Value) -> String {
    let corpo = serde_json::to_vec(&dados).unwrap();
    let url = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!(
        "{}.{}",
        url.encode(&corpo),
        url.encode(privada.sign(&corpo).to_bytes())
    )
}

/// Igual ao `emitirAtestado` do `bot/otimiza-licenca.cjs`.
fn atestado(privada: &SigningKey, maquina: &str, chave: &str, emitido: &str, ate: &str) -> String {
    assinar(
        privada,
        serde_json::json!({
            "tipo": "licenca-online",
            "maquina": maquina,
            "licenca": impressao_da_licenca(chave),
            "emitido": emitido,
            "valido_ate": ate,
        }),
    )
}

fn licenca(privada: &SigningKey, maquina: &str, emitida: &str) -> String {
    assinar(
        privada,
        serde_json::json!({
            "maquina": maquina,
            "comprador": "Fulano de Teste",
            "emitida": emitida,
            "expira": null,
        }),
    )
}

fn guardada(atestado: Option<String>) -> Guardada {
    Guardada {
        chave: CHAVE.to_string(),
        atestado,
        revogada: None,
    }
}

#[test]
fn atestado_em_dia_vale() {
    let (privada, publica) = par();
    let a = atestado(&privada, M, CHAVE, "2026-10-01", "2026-10-31");
    assert!(conferir(publica, &a, CHAVE, M, "2026-10-29").is_ok());
}

#[test]
fn atestado_vencido_nao_vale() {
    let (privada, publica) = par();
    let a = atestado(&privada, M, CHAVE, "2026-10-01", "2026-10-31");
    assert_eq!(
        conferir(publica, &a, CHAVE, M, "2026-11-01").unwrap_err(),
        Falta::Vencido {
            em: "2026-10-31".to_string()
        }
    );
}

#[test]
fn atestado_de_outra_maquina_ou_outra_licenca_nao_vale() {
    let (privada, publica) = par();
    let de_outra = atestado(
        &privada,
        "OTZ-AAAA-BBBB-CCCC",
        CHAVE,
        "2026-10-01",
        "2026-10-31",
    );
    assert_eq!(
        conferir(publica, &de_outra, CHAVE, M, "2026-10-10").unwrap_err(),
        Falta::Invalido
    );

    let de_outra_chave = atestado(&privada, M, "outra.chave", "2026-10-01", "2026-10-31");
    assert_eq!(
        conferir(publica, &de_outra_chave, CHAVE, M, "2026-10-10").unwrap_err(),
        Falta::Invalido
    );
}

#[test]
fn atestado_assinado_por_outro_par_nao_vale() {
    let (_, publica) = par();
    let (estranha, _) = par();
    let a = atestado(&estranha, M, CHAVE, "2026-10-01", "2026-10-31");
    assert_eq!(
        conferir(publica, &a, CHAVE, M, "2026-10-10").unwrap_err(),
        Falta::Invalido
    );
}

#[test]
fn a_licenca_nao_passa_por_atestado_nem_o_contrario() {
    // Mesmo par assina os dois: o `tipo` e os campos obrigatórios separam.
    let (privada, publica) = par();
    let l = licenca(&privada, M, "2026-10-01");
    assert_eq!(
        conferir(publica, &l, &l, M, "2026-10-10").unwrap_err(),
        Falta::Invalido
    );

    let a = atestado(&privada, M, CHAVE, "2026-10-01", "2026-10-31");
    assert!(crate::modules::licenca::conferir_com(publica, &a, M, "2026-10-10").is_err());
}

#[test]
fn espaco_de_copia_nao_muda_a_impressao() {
    assert_eq!(
        impressao_da_licenca("abc.def"),
        impressao_da_licenca("  abc\n.def ")
    );
}

#[test]
fn desligado_nada_muda_para_o_cliente() {
    let (_, publica) = par();
    let mut g = guardada(None);
    g.revogada = Some(Revogacao {
        licenca: impressao_da_licenca(CHAVE),
        motivo: "teste".to_string(),
    });
    assert!(politica(publica, false, true, &g, M, "2026-10-10").is_ok());
}

#[test]
fn revogada_tranca_mesmo_sem_exigir_atestado() {
    let (_, publica) = par();
    let mut g = guardada(None);
    g.revogada = Some(Revogacao {
        licenca: impressao_da_licenca(CHAVE),
        motivo: "chave repassada".to_string(),
    });
    assert!(matches!(
        politica(publica, true, false, &g, M, "2026-10-10"),
        Err(Falta::Revogada { .. })
    ));
}

#[test]
fn revogacao_de_outra_chave_nao_tranca_a_nova() {
    let (_, publica) = par();
    let mut g = guardada(None);
    g.revogada = Some(Revogacao {
        licenca: impressao_da_licenca("chave.antiga"),
        motivo: "trocada".to_string(),
    });
    assert!(politica(publica, true, false, &g, M, "2026-10-10").is_ok());
}

#[test]
fn exigido_sem_atestado_tranca_e_com_atestado_abre() {
    let (privada, publica) = par();
    assert_eq!(
        politica(publica, true, true, &guardada(None), M, "2026-10-10").unwrap_err(),
        Falta::Ausente
    );

    let a = atestado(&privada, M, CHAVE, "2026-10-01", "2026-10-31");
    assert!(politica(publica, true, true, &guardada(Some(a)), M, "2026-10-10").is_ok());
}

#[test]
fn renova_perto_do_fim_e_nao_renova_no_comeco() {
    let (privada, publica) = par();
    let dia = |t: &str| chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap();

    assert!(precisa_renovar(
        publica,
        &guardada(None),
        M,
        dia("2026-10-10")
    ));

    let a = atestado(&privada, M, CHAVE, "2026-10-01", "2026-10-31");
    let g = guardada(Some(a));
    assert!(!precisa_renovar(publica, &g, M, dia("2026-10-02")));
    assert!(precisa_renovar(publica, &g, M, dia("2026-10-20")));
}

#[test]
fn quem_faltou_na_lista_e_mandado_ao_suporte() {
    let frase = Recusa::ChaveAntiga.explicacao();
    assert!(frase.contains("suporte") && frase.contains("código desta máquina"));
}

#[test]
fn toda_falta_explica_o_que_fazer() {
    assert!(Falta::Ausente.explicacao().contains("internet"));
    assert!(Falta::Vencido {
        em: "2026-10-31".to_string()
    }
    .explicacao()
    .contains("2026-10-31"));
    assert!(Falta::Revogada {
        motivo: "x".to_string()
    }
    .explicacao()
    .contains("suporte"));
}

#[test]
fn so_https_liga_o_online() {
    assert_eq!(endereco_de(None), None);
    assert_eq!(endereco_de(Some("")), None);
    assert_eq!(endereco_de(Some("http://api.exemplo")), None);
    assert_eq!(
        endereco_de(Some("  https://api.exemplo ")),
        Some("https://api.exemplo")
    );
}

fn revogacao(privada: &SigningKey, maquina: &str, chave: &str) -> String {
    assinar(
        privada,
        serde_json::json!({
            "tipo": "revogacao",
            "maquina": maquina,
            "licenca": impressao_da_licenca(chave),
            "motivo": "chave repassada",
        }),
    )
}

#[test]
fn revogacao_so_vale_assinada_e_desta_licenca() {
    let (privada, publica) = par();
    let (estranha, _) = par();

    let boa = revogacao(&privada, M, CHAVE);
    assert_eq!(
        conferir_revogacao(publica, &boa, CHAVE, M).map(|r| r.motivo),
        Some("chave repassada".to_string())
    );

    assert!(conferir_revogacao(publica, &revogacao(&estranha, M, CHAVE), CHAVE, M).is_none());
    assert!(conferir_revogacao(
        publica,
        &revogacao(&privada, "OTZ-AAAA-BBBB-CCCC", CHAVE),
        CHAVE,
        M
    )
    .is_none());
    assert!(
        conferir_revogacao(publica, &revogacao(&privada, M, "outra.chave"), CHAVE, M).is_none()
    );

    // Um atestado não passa por revogação.
    let a = atestado(&privada, M, CHAVE, "2026-10-01", "2026-10-31");
    assert!(conferir_revogacao(publica, &a, CHAVE, M).is_none());
}

#[test]
fn chave_anterior_so_vale_ate_a_troca() {
    let (privada, publica) = par();

    let antiga = licenca(&privada, M, "2026-09-20");
    assert!(conferir_anterior(publica, &[], &antiga, M, "2026-10-29").is_ok());

    // Venda da noite de 04/10 em Brasília: o bot carimba em UTC, dia 05.
    let da_noite = licenca(&privada, M, "2026-10-05");
    assert!(conferir_anterior(publica, &[], &da_noite, M, "2026-10-29").is_ok());

    // Quem ficou com a privada perdida assina a data de hoje: recusada.
    let forjada = licenca(&privada, M, "2026-10-20");
    assert_eq!(
        conferir_anterior(publica, &[], &forjada, M, "2026-10-29").unwrap_err(),
        Recusa::ChaveAntiga
    );
}

#[test]
fn com_a_lista_preenchida_so_as_maquinas_dela_passam() {
    let (privada, publica) = par();
    let lista_texto = resumo_da_maquina(M);
    let lista = [lista_texto.as_str()];

    let da_lista = licenca(&privada, M, "2026-09-20");
    assert!(conferir_anterior(publica, &lista, &da_lista, M, "2026-10-29").is_ok());

    // Data antiga, assinatura boa, máquina fora da lista: é o que um gerador com a privada perdida produz.
    let outra = "OTZ-AAAA-BBBB-CCCC";
    let fora = licenca(&privada, outra, "2026-09-20");
    assert_eq!(
        conferir_anterior(publica, &lista, &fora, outra, "2026-10-29").unwrap_err(),
        Recusa::ChaveAntiga
    );
}

#[test]
fn o_resumo_da_maquina_bate_com_o_do_bot() {
    // Mesmo valor que `resumoDaMaquina("OTZ-WPYY-0J4F-77AB")` no `otimiza-licenca.cjs` (sha256 de Node).
    assert_eq!(
        resumo_da_maquina(" otz-wpyy-0j4f-77ab "),
        "6a9086ff684d2accb7605ed1762b7ac78b351f0191d016a71831fce0e24a29c3"
    );
    assert_eq!(
        crate::modules::licenca::hex_sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
