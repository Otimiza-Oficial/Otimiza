//! As provas de ponta a ponta da licença.
//!
//! Este arquivo existe SÓ em compilação de teste (`#[cfg(test)]` na declaração
//! dele em `licenca.rs`), e é o único do lado do produto que toca em chave
//! privada. A chave que ele usa é sorteada na hora, vive alguns milissegundos
//! em memória e não é a de ninguém.
//!
//! Está separado de `licenca.rs` porque lá existe uma guarda que reprova o
//! build se a palavra `SigningKey` aparecer. Essa guarda é boa e não vai sair;
//! o que muda é onde a prova mora.
//!
//! O que aqui se prova são os itens 1, 2 e 3 do plano:
//!
//!   1. Uma chave emitida para esta máquina ativa.
//!   2. A MESMA chave com um caractere trocado é recusada — é o teste que
//!      prova que a assinatura está sendo conferida de verdade, e não apenas
//!      que o texto tem o formato certo.
//!   3. Uma chave emitida para outro ID não ativa aqui. É o teste do
//!      "uma chave, um PC".

use super::*;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};

/// Sorteia um par de chaves para o teste.
fn par() -> (SigningKey, VerifyingKey) {
    let mut semente = [0u8; 32];
    getrandom::getrandom(&mut semente).expect("sortear semente");

    let privada = SigningKey::from_bytes(&semente);
    let publica = privada.verifying_key();

    (privada, publica)
}

/// Emite uma licença do mesmo jeito que `examples/gerar_chave.rs` emite.
///
/// Se os dois formatos divergirem um dia, estes testes param de valer como
/// prova. Por isso existe, no fim do arquivo, uma guarda que confere que o
/// emissor de verdade continua montando a chave da mesma forma.
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

// ------------------------------------------------------------ 1. ela ativa

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

// ---------------------------------------- 2. um caractere trocado derruba

#[test]
fn um_caractere_trocado_derruba_a_chave() {
    // O teste mais importante do arquivo. Se ele passar por acidente — porque
    // a conferência não está acontecendo —, todo o resto é teatro: qualquer
    // pessoa escreveria a própria licença num editor de texto.
    let (privada, publica) = par();
    let boa = emitir(&privada, "OTZ-WPYY-0J4F-77AB", None);

    let (dados, assinatura) = boa.split_once('.').unwrap();

    // Mexe nos DADOS, mantendo a assinatura. É a tentativa óbvia: pegar a
    // chave de alguém e trocar o ID da máquina para o seu.
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

    // E mexe na ASSINATURA, mantendo os dados.
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

    // A original continua valendo — senão o teste acima não prova nada.
    assert!(conferir_com(publica, &boa, "OTZ-WPYY-0J4F-77AB", "2026-08-29").is_ok());
}

#[test]
fn chave_assinada_por_outro_par_nao_vale() {
    // O caso de quem monta o próprio emissor: o formato está certo, a
    // assinatura é uma assinatura de verdade — só que de outra chave.
    let (privada_do_impostor, _) = par();
    let (_, publica_do_otimiza) = par();

    let forjada = emitir(&privada_do_impostor, "OTZ-WPYY-0J4F-77AB", None);

    assert_eq!(
        conferir_com(publica_do_otimiza, &forjada, "OTZ-WPYY-0J4F-77AB", "2026-08-29").unwrap_err(),
        Recusa::AssinaturaInvalida
    );
}

// ------------------------------------------- 3. uma chave, um computador

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

    // E a mensagem precisa dizer o que fazer, não só que deu errado.
    assert!(recusa.explicacao().contains("Discord"));
}

// ---------------------------------------------------------------- prazo

#[test]
fn a_licenca_com_prazo_vence_no_dia_seguinte() {
    let (privada, publica) = par();
    let chave = emitir(&privada, "OTZ-WPYY-0J4F-77AB", Some("2026-12-31"));

    // No próprio dia do vencimento ainda vale. Cortar no dia seria cobrar um
    // dia a menos do que foi vendido.
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
    // Em máquina virtual pode não haver série de placa nem MachineGuid. O
    // padrão nesse caso é trancar, não liberar.
    let (privada, publica) = par();
    let chave = emitir(&privada, "OTZ-WPYY-0J4F-77AB", None);

    assert_eq!(
        conferir_com(publica, &chave, "", "2026-08-29").unwrap_err(),
        Recusa::MaquinaDesconhecida
    );
}

// ---------------------------------------------------------------- guarda

#[test]
fn o_emissor_de_verdade_monta_a_chave_do_mesmo_jeito() {
    // Estes testes só provam alguma coisa enquanto a `emitir` daqui e a de
    // `examples/gerar_chave.rs` produzirem o mesmo formato. Se alguém mudar o
    // emissor e esquecer deste arquivo, as provas acima continuariam passando
    // enquanto o produto real deixaria de aceitar as chaves emitidas.
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
