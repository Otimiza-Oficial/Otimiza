//! Emissor de licença do Otimiza — a ferramenta do dono.
//!
//! ISTO NUNCA ENTRA NO INSTALADOR.
//!
//! Está em `examples/` de propósito: `cargo build --release` não compila
//! exemplos. Esta é a única peça do projeto que toca na chave privada, e ela
//! precisa ficar do lado de cá da cerca.
//!
//! COMO USAR
//!
//! Uma vez na vida, para criar o par de chaves:
//!
//!     cargo run --example gerar_chave -- novo-par
//!
//! Isso imprime duas linhas. A PÚBLICA vai para a constante `CHAVE_PUBLICA` em
//! `src/modules/licenca.rs`. A PRIVADA você guarda — e ela nunca entra no
//! repositório, nunca é colada em conversa, nunca sobe para lugar nenhum além
//! do segredo do bot do Discord.
//!
//! Depois, a cada venda:
//!
//!     cargo run --example gerar_chave -- emitir <PRIVADA> OTZ-XXXX-XXXX-XXXX "Nome" [AAAA-MM-DD]
//!
//! O último argumento é a validade, e é opcional: sem ele a licença é
//! vitalícia.
//!
//! POR QUE PERDER A CHAVE PRIVADA É GRAVE
//!
//! Não dá para recuperá-la a partir da pública — é essa impossibilidade que faz
//! o sistema funcionar. Perdendo, o único caminho é gerar um par novo, publicar
//! uma versão do programa com a pública nova, e reemitir a licença de TODOS os
//! clientes. Guarde em dois lugares.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("novo-par") => novo_par(),
        Some("emitir") => emitir(&args[1..]),
        _ => {
            eprintln!("{}", AJUDA);
            std::process::exit(2);
        }
    }
}

const AJUDA: &str = "\
Emissor de licença do Otimiza

  novo-par
      Cria o par de chaves. Roda UMA vez na vida do produto.

  emitir <PRIVADA> <MAQUINA> <COMPRADOR> [VALIDADE]
      Emite uma licença para uma máquina.
      MAQUINA   é o código OTZ-XXXX-XXXX-XXXX que o cliente manda.
      VALIDADE  em AAAA-MM-DD. Sem ela, a licença é vitalícia.
";

fn novo_par() {
    let mut semente = [0u8; 32];

    if getrandom::getrandom(&mut semente).is_err() {
        eprintln!("Não foi possível sortear a chave com segurança. Nada foi gerado.");
        std::process::exit(1);
    }

    let privada = SigningKey::from_bytes(&semente);
    let publica = privada.verifying_key();

    let padrao = base64::engine::general_purpose::STANDARD;

    println!("PUBLICA  (vai para CHAVE_PUBLICA em licenca.rs):");
    println!("{}", padrao.encode(publica.to_bytes()));
    println!();
    println!("PRIVADA  (guarde; nunca versione, nunca cole em conversa):");
    println!("{}", padrao.encode(privada.to_bytes()));
    println!();
    println!("Perder a privada obriga a reemitir a licença de todos os clientes.");
}

fn emitir(args: &[String]) {
    let [privada, maquina, comprador, resto @ ..] = args else {
        eprintln!("{}", AJUDA);
        std::process::exit(2);
    };

    let padrao = base64::engine::general_purpose::STANDARD;

    let bytes = padrao
        .decode(privada.trim())
        .ok()
        .and_then(|b| <[u8; 32]>::try_from(b).ok());

    let Some(bytes) = bytes else {
        eprintln!("A chave privada não parece válida. Ela tem 32 bytes em base64.");
        std::process::exit(1);
    };

    if !maquina.starts_with("OTZ-") {
        eprintln!(
            "`{}` não parece um código de máquina. Ele tem a forma OTZ-XXXX-XXXX-XXXX e o \
             cliente copia da própria tela do Otimiza.",
            maquina
        );
        std::process::exit(1);
    }

    let assinante = SigningKey::from_bytes(&bytes);

    // A ordem dos campos aqui precisa bater com a struct `Dados` de
    // `licenca.rs`. Como os dois lados usam serde sobre JSON com os mesmos
    // nomes, o que importa é o NOME de cada campo, não a ordem.
    let dados = serde_json::json!({
        "maquina": maquina,
        "comprador": comprador,
        "emitida": chrono::Local::now().format("%Y-%m-%d").to_string(),
        "expira": resto.first().map(|v| v.trim().to_string()),
    });

    let corpo = serde_json::to_vec(&dados).expect("serializar a licença");
    let assinatura = assinante.sign(&corpo);

    let url = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    println!("{}.{}", url.encode(&corpo), url.encode(assinatura.to_bytes()));
}
