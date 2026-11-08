// DISCORD RICH PRESENCE (Plano 1.1, Onda 4). Com a opção ligada (Ajustes; desligada por padrão), o perfil do Discord
// mostra "Jogando <jogo> · com o Otimiza", com o Oti e o tempo de partida. Fala com o Discord aberto no próprio PC pelo
// canal local dele (`\\.\pipe\discord-ipc-N`): nenhuma senha, token ou servidor no meio, e nada sai além do nome do jogo.
//
// Protocolo: cada mensagem é `op` (u32 LE) + tamanho (u32 LE) + JSON. Abre com HANDSHAKE (op 0, `client_id`), depois
// FRAME (op 1) com `SET_ACTIVITY`; atividade `null` limpa. O ID da aplicação (público) entra na compilação por
// `OTIMIZA_DISCORD_APP_ID`; sem ele, a opção diz que não está disponível nesta versão.

use serde_json::{json, Value};

pub const OP_HANDSHAKE: u32 = 0;
pub const OP_FRAME: u32 = 1;
pub const OP_FECHAR: u32 = 2;

/// A imagem grande: a arte "oti" enviada em Rich Presence › Art Assets da aplicação no portal do Discord (sem ela, o
/// Discord mostra a atividade sem imagem).
const IMAGEM: &str = "oti";

pub fn id_da_aplicacao() -> Option<&'static str> {
    option_env!("OTIMIZA_DISCORD_APP_ID")
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

/// **Pura.** Uma mensagem no formato do canal.
pub fn quadro(op: u32, corpo: &Value) -> Vec<u8> {
    let texto = corpo.to_string();
    let mut v = Vec::with_capacity(8 + texto.len());
    v.extend_from_slice(&op.to_le_bytes());
    v.extend_from_slice(&(texto.len() as u32).to_le_bytes());
    v.extend_from_slice(texto.as_bytes());
    v
}

/// **Pura.** O cabeçalho de uma resposta: (op, tamanho do corpo).
pub fn cabecalho(b: &[u8; 8]) -> (u32, usize) {
    (
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize,
    )
}

/// **Pura.** O pedido de atividade. `jogo = None` limpa.
pub fn pedido_de_atividade(pid: u32, jogo: Option<&str>, inicio_s: u64, nonce: u64) -> Value {
    let atividade = jogo.map(|j| {
        json!({
            "details": format!("Jogando {j}"),
            "state": "com o Otimiza",
            "timestamps": { "start": inicio_s },
            "assets": { "large_image": IMAGEM, "large_text": "Oti, o mascote do Otimiza" },
        })
    });
    json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": pid, "activity": atividade },
        "nonce": nonce.to_string(),
    })
}

#[cfg(target_os = "windows")]
pub mod maquina {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicBool, Ordering};

    static LIGADO: AtomicBool = AtomicBool::new(false);

    pub fn definir(ligado: bool) {
        LIGADO.store(ligado, Ordering::SeqCst);
    }

    struct Conexao {
        canal: std::fs::File,
        nonce: u64,
    }

    impl Conexao {
        fn abrir(id: &str) -> Option<Conexao> {
            for n in 0..10 {
                let caminho = format!(r"\\.\pipe\discord-ipc-{n}");
                let Ok(canal) = std::fs::OpenOptions::new().read(true).write(true).open(&caminho) else {
                    continue;
                };
                let mut c = Conexao { canal, nonce: 0 };
                if c.enviar(OP_HANDSHAKE, &json!({ "v": 1, "client_id": id })).is_ok() && c.ler().is_some() {
                    return Some(c);
                }
            }
            None
        }

        fn enviar(&mut self, op: u32, corpo: &Value) -> std::io::Result<()> {
            self.canal.write_all(&quadro(op, corpo))
        }

        fn ler(&mut self) -> Option<Value> {
            let mut b = [0u8; 8];
            self.canal.read_exact(&mut b).ok()?;
            let (_, tamanho) = cabecalho(&b);
            if tamanho > 1 << 20 {
                return None;
            }
            let mut corpo = vec![0u8; tamanho];
            self.canal.read_exact(&mut corpo).ok()?;
            serde_json::from_slice(&corpo).ok()
        }

        fn atividade(&mut self, jogo: Option<&str>, inicio: u64) -> bool {
            self.nonce += 1;
            let p = pedido_de_atividade(std::process::id(), jogo, inicio, self.nonce);
            self.enviar(OP_FRAME, &p).is_ok() && self.ler().is_some()
        }
    }

    /// A vigia: a cada 15 s confere o jogo aberto e a opção. Sem Discord aberto, tenta de novo na volta seguinte.
    pub fn vigiar() {
        let Some(id) = id_da_aplicacao() else { return };
        LIGADO.store(crate::modules::preferences::Preferences::load().discord_presenca, Ordering::SeqCst);
        std::thread::Builder::new()
            .name("discord-presenca".into())
            .spawn(move || {
                let mut conexao: Option<Conexao> = None;
                // O que está publicado agora: (jogo, início).
                let mut publicado: Option<(String, u64)> = None;
                loop {
                    let jogo = LIGADO
                        .load(Ordering::SeqCst)
                        .then(super::super::sessao::atual)
                        .flatten()
                        .map(|j| j.nome);
                    let mudou = jogo.as_deref() != publicado.as_ref().map(|(j, _)| j.as_str());
                    if mudou && (jogo.is_some() || publicado.is_some()) {
                        if conexao.is_none() {
                            conexao = Conexao::abrir(id);
                        }
                        if let Some(c) = conexao.as_mut() {
                            let inicio = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            if c.atividade(jogo.as_deref(), inicio) {
                                publicado = jogo.map(|j| (j, inicio));
                            } else {
                                // Discord fechou ou reiniciou: reabre na próxima volta.
                                conexao = None;
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(15));
                }
            })
            .ok();
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Com o Discord aberto: o aperto de mão com um ID inválido tem de voltar recusado, no formato certo.
    /// `cargo test --lib o_discord_responde -- --ignored --nocapture`
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn o_discord_responde_ao_aperto_de_mao() {
        use std::io::{Read, Write};
        let mut canal = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(r"\\.\pipe\discord-ipc-0")
            .expect("Discord aberto");
        canal.write_all(&quadro(OP_HANDSHAKE, &json!({ "v": 1, "client_id": "1" }))).unwrap();
        let mut b = [0u8; 8];
        canal.read_exact(&mut b).unwrap();
        let (op, n) = cabecalho(&b);
        let mut corpo = vec![0u8; n];
        canal.read_exact(&mut corpo).unwrap();
        let v: Value = serde_json::from_slice(&corpo).unwrap();
        println!("op {op}: {v}");
        assert!(op == OP_FECHAR || op == OP_FRAME);
    }

    #[test]
    fn o_quadro_tem_op_tamanho_e_json() {
        let q = quadro(OP_HANDSHAKE, &json!({ "v": 1, "client_id": "123" }));
        let (op, n) = cabecalho(q[..8].try_into().unwrap());
        assert_eq!(op, 0);
        assert_eq!(n, q.len() - 8);
        let corpo: Value = serde_json::from_slice(&q[8..]).unwrap();
        assert_eq!(corpo["client_id"], "123");
    }

    #[test]
    fn a_atividade_diz_o_jogo_e_o_inicio_e_limpar_manda_nulo() {
        let p = pedido_de_atividade(42, Some("FiveM"), 1_700_000_000, 7);
        assert_eq!(p["cmd"], "SET_ACTIVITY");
        assert_eq!(p["args"]["pid"], 42);
        assert_eq!(p["args"]["activity"]["details"], "Jogando FiveM");
        assert_eq!(p["args"]["activity"]["timestamps"]["start"], 1_700_000_000u64);
        assert_eq!(p["nonce"], "7");
        assert!(pedido_de_atividade(42, None, 0, 8)["args"]["activity"].is_null());
    }
}
