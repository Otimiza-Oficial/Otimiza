//! Provas de ponta a ponta da licença, só em compilação de teste: o único arquivo do produto que toca em chave
//! privada, sorteada na hora. Fica fora de `licenca.rs` porque lá uma guarda reprova o build se `SigningKey`
//! aparecer. Prova: a chave desta máquina ativa; um caractere trocado é recusado; chave de outro ID não ativa.

use super::*;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};

fn par() -> (SigningKey, VerifyingKey) {
    let mut semente = [0u8; 32];
    getrandom::getrandom(&mut semente).expect("sortear semente");

    let privada = SigningKey::from_bytes(&semente);
    let publica = privada.verifying_key();

    (privada, publica)
}

/// Igual a `examples/gerar_chave.rs`; a guarda do fim do arquivo confere que os dois não divergem.
fn emitir(privada: &SigningKey, maquina: &str, expira: Option<&str>) -> String {
    let dados = serde_json::json!({
        "maquina": maquina,
        "comprador": "Fulano de Teste",
        "emitida": "2026-01-01",
        "expira": expira,
    });

    let corpo = serde_json::to_vec(&dados).expect("serializar");
    let assinatura = privada.sign(&corpo);

    let url = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    format!("{}.{}", url.encode(&corpo), url.encode(assinatura.to_bytes()))
}

#[test]
fn chave_emitida_para_esta_maquina_e_aceita() {
    let (privada, publica) = par();
    let chave = emitir(&privada, "OTZ-WPYY-0J4F-77AB", None);

    let dados = conferir_com(publica, &chave, "OTZ-WPYY-0J4F-77AB", "2026-08-29")
        .expect("a chave emitida para esta máquina tinha que valer");

    assert_eq!(dados.maquina, "OTZ-WPYY-0J4F-77AB");
    assert_eq!(dados.comprador, "Fulano de Teste");
    assert_eq!(dados.expira, None, "sem prazo é vitalícia");
}

#[test]
fn um_caractere_trocado_derruba_a_chave() {
    // Se este passar por acidente, todo o resto é teatro: qualquer um escreveria a própria licença.
    let (privada, publica) = par();
    let boa = emitir(&privada, "OTZ-WPYY-0J4F-77AB", None);

    let (dados, assinatura) = boa.split_once('.').unwrap();

    let mut mexida: Vec<char> = dados.chars().collect();
    mexida[10] = if mexida[10] == 'A' { 'B' } else { 'A' };
    let com_dados_mexidos = format!("{}.{}", mexida.iter().collect::<String>(), assinatura);

    assert!(
        matches!(
            conferir_com(publica, &com_dados_mexidos, "OTZ-WPYY-0J4F-77AB", "2026-08-29"),
            Err(Recusa::AssinaturaInvalida) | Err(Recusa::Malformada)
        ),
        "dados alterados passaram na conferência"
    );

    let mut mexida: Vec<char> = assinatura.chars().collect();
    let ultimo = mexida.len() - 1;
    mexida[ultimo] = if mexida[ultimo] == 'A' { 'B' } else { 'A' };
    let com_assinatura_mexida = format!("{}.{}", dados, mexida.iter().collect::<String>());

    assert!(
        matches!(
            conferir_com(publica, &com_assinatura_mexida, "OTZ-WPYY-0J4F-77AB", "2026-08-29"),
            Err(Recusa::AssinaturaInvalida) | Err(Recusa::Malformada)
        ),
        "assinatura alterada passou na conferência"
    );

    // A original continua valendo; senão o teste acima não prova nada.
    assert!(conferir_com(publica, &boa, "OTZ-WPYY-0J4F-77AB", "2026-08-29").is_ok());
}

#[test]
fn chave_assinada_por_outro_par_nao_vale() {
    let (privada_do_impostor, _) = par();
    let (_, publica_do_otimiza) = par();

    let forjada = emitir(&privada_do_impostor, "OTZ-WPYY-0J4F-77AB", None);

    assert_eq!(
        conferir_com(publica_do_otimiza, &forjada, "OTZ-WPYY-0J4F-77AB", "2026-08-29").unwrap_err(),
        Recusa::AssinaturaInvalida
    );
}

#[test]
fn chave_de_outra_maquina_nao_ativa_aqui() {
    let (privada, publica) = par();
    let do_vizinho = emitir(&privada, "OTZ-AAAA-BBBB-CCCC", None);

    let recusa = conferir_com(publica, &do_vizinho, "OTZ-WPYY-0J4F-77AB", "2026-08-29").unwrap_err();

    assert_eq!(
        recusa,
        Recusa::OutraMaquina {
            emitida_para: "OTZ-AAAA-BBBB-CCCC".to_string()
        }
    );

    assert!(recusa.explicacao().contains("Discord"));
}

#[test]
fn a_licenca_com_prazo_vence_no_dia_seguinte() {
    let (privada, publica) = par();
    let chave = emitir(&privada, "OTZ-WPYY-0J4F-77AB", Some("2026-12-31"));

    // No dia do vencimento ainda vale: cortar no dia seria cobrar um dia a menos do que foi vendido.
    assert!(conferir_com(publica, &chave, "OTZ-WPYY-0J4F-77AB", "2026-12-31").is_ok());

    assert_eq!(
        conferir_com(publica, &chave, "OTZ-WPYY-0J4F-77AB", "2027-01-01").unwrap_err(),
        Recusa::Expirada {
            em: "2026-12-31".to_string()
        }
    );
}

#[test]
fn maquina_nao_identificada_nunca_libera() {
    // Sem série de placa nem MachineGuid (máquina virtual), o padrão é trancar, não liberar.
    let (privada, publica) = par();
    let chave = emitir(&privada, "OTZ-WPYY-0J4F-77AB", None);

    assert_eq!(
        conferir_com(publica, &chave, "", "2026-08-29").unwrap_err(),
        Recusa::MaquinaDesconhecida
    );
}

/// Par e licença emitidos pelo `bot/otimiza-licenca.cjs`, fixos de propósito: prova que a chave saída do bot, em
/// JavaScript, abre o produto em Rust. Duas implementações do mesmo formato divergem sem ninguém perceber, e o
/// prejuízo é o cliente pagar e a chave não abrir. Este par nunca abriu nada.
const PUBLICA_DO_BOT: &str = "0g5yK2+hntwcBt/QTqD1gtkWJD9YNpG5DhNnZednrTk=";

const LICENCA_DO_BOT: &str = "eyJtYXF1aW5hIjoiT1RaLVRFU1QtQjBUMC0wMDAxIiwiY29tcHJhZG9yIjoiUHJvdmEgZGUgY29tcGF0aWJpbGlkYWRlIiwiZW1pdGlkYSI6IjIwMjYtMDEtMDEiLCJleHBpcmEiOm51bGx9.MKtQhvO4EKP_onlDTG68381zNsIOiPF2wdKB4FrFJlVtliYfWBYUGFmZiHam_CHTVI5aSzn690ioD0Q6RjUrCQ";

#[test]
fn a_chave_emitida_pelo_bot_abre_o_produto() {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(PUBLICA_DO_BOT)
        .expect("a pública do bot é base64");

    let publica = VerifyingKey::from_bytes(&bytes.try_into().expect("32 bytes"))
        .expect("chave pública válida");

    let dados = conferir_com(
        publica,
        LICENCA_DO_BOT,
        "OTZ-TEST-B0T0-0001",
        "2026-08-29",
    )
    .expect("o Rust precisa aceitar o que o bot em JavaScript assinou");

    assert_eq!(dados.maquina, "OTZ-TEST-B0T0-0001");
    assert_eq!(dados.comprador, "Prova de compatibilidade");
    assert_eq!(dados.emitida, "2026-01-01");
    assert_eq!(dados.expira, None);
}

#[test]
fn a_chave_do_bot_tambem_e_presa_a_uma_maquina() {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(PUBLICA_DO_BOT)
        .unwrap();
    let publica = VerifyingKey::from_bytes(&bytes.try_into().unwrap()).unwrap();

    assert!(matches!(
        conferir_com(publica, LICENCA_DO_BOT, "OTZ-WPYY-0J4F-77AB", "2026-08-29"),
        Err(Recusa::OutraMaquina { .. })
    ));
}

#[test]
fn o_emissor_de_verdade_monta_a_chave_do_mesmo_jeito() {
    let emissor = include_str!("../../examples/gerar_chave.rs");

    for parte in [
        "URL_SAFE_NO_PAD",
        "\"maquina\": maquina",
        "\"comprador\": comprador",
        "\"emitida\"",
        "\"expira\"",
        "{}.{}",
    ] {
        assert!(
            emissor.contains(parte),
            "o emissor não tem mais `{}` — o formato da chave mudou e este \
             arquivo precisa acompanhar",
            parte
        );
    }
}
