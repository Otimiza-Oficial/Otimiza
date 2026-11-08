// CS2 GAME STATE INTEGRATION (Plano 1.1, Onda 4). O próprio CS2 conta ao Otimiza em que pé a partida está — mapa,
// fase, se a pessoa está viva ou assistindo — pelo canal oficial da Valve: um arquivo `gamestate_integration_*.cfg` na
// pasta `cfg` do jogo faz o CS2 mandar POSTs JSON para um endereço local. Não lê memória, não injeta nada: é o mesmo
// recurso que os overlays de torneio usam, permitido pela Valve.
//
// Para quê: medir FPS só quando importa (rodada valendo e a pessoa viva, não o menu a 400 FPS nem o espectador), e o
// Oti saber quando NÃO falar (no meio da rodada).
//
// O arquivo leva um código aleatório (`auth.token`): o servidor local só aceita POST com ele. Escuta só em 127.0.0.1.
// Desligar apaga o arquivo; nada mais no jogo é tocado.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const PORTA: u16 = 47_823;
const NOME_DO_ARQUIVO: &str = "gamestate_integration_otimiza.cfg";
/// Sem notícia do CS2 há mais que isso, o estado não vale (o CS2 manda batida a cada 10 s).
pub const VALIDADE_S: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum MomentoDoCs2 {
    /// Menu principal (sem mapa).
    Menu,
    /// Aquecimento, intervalo, fim de partida ou congelamento do começo da rodada.
    Pausa,
    /// Rodada valendo e a pessoa viva: o momento que a medida quer.
    Jogando,
    /// Morto ou assistindo outro jogador.
    Assistindo,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct EstadoDoCs2 {
    pub momento: MomentoDoCs2,
    pub mapa: Option<String>,
    /// "competitive", "casual", "deathmatch"…
    pub modo: Option<String>,
    pub rodada: Option<u32>,
    /// Segundos desde o último aviso do CS2.
    pub ha_s: u64,
}

/// **Pura.** O JSON que o CS2 manda vira o momento. Campos ausentes são normais (o CS2 só manda o que mudou nos
/// "added"/"previously", mas o corpo traz o estado inteiro).
pub fn ler_estado(corpo: &serde_json::Value) -> (MomentoDoCs2, Option<String>, Option<String>, Option<u32>) {
    let texto = |v: &serde_json::Value| v.as_str().map(String::from);
    let mapa = &corpo["map"];
    if mapa.is_null() {
        return (MomentoDoCs2::Menu, None, None, None);
    }
    let nome = texto(&mapa["name"]);
    let modo = texto(&mapa["mode"]);
    let rodada = mapa["round"].as_u64().map(|r| r as u32 + 1);
    let fase_do_mapa = mapa["phase"].as_str().unwrap_or("");
    let fase_da_rodada = corpo["round"]["phase"].as_str().unwrap_or("");

    // Assistindo: o jogador do corpo não é quem joga neste PC (o `provider`).
    let eu = corpo["provider"]["steamid"].as_str();
    let jogador = corpo["player"]["steamid"].as_str();
    let assistindo = matches!((eu, jogador), (Some(a), Some(b)) if a != b);
    let vida = corpo["player"]["state"]["health"].as_u64();

    let momento = if fase_do_mapa != "live" || fase_da_rodada == "freezetime" {
        MomentoDoCs2::Pausa
    } else if assistindo || vida == Some(0) {
        MomentoDoCs2::Assistindo
    } else {
        MomentoDoCs2::Jogando
    };
    (momento, nome, modo, rodada)
}

/// **Pura.** O texto do arquivo de configuração.
pub fn arquivo_de_configuracao(token: &str) -> String {
    format!(
        "\"Otimiza\"\r\n{{\r\n\
         \t\"uri\"       \"http://127.0.0.1:{PORTA}/cs2\"\r\n\
         \t\"timeout\"   \"1.0\"\r\n\
         \t\"buffer\"    \"0.1\"\r\n\
         \t\"throttle\"  \"0.5\"\r\n\
         \t\"heartbeat\" \"10.0\"\r\n\
         \t\"auth\"\r\n\t{{\r\n\t\t\"token\" \"{token}\"\r\n\t}}\r\n\
         \t\"data\"\r\n\t{{\r\n\
         \t\t\"provider\"     \"1\"\r\n\
         \t\t\"map\"          \"1\"\r\n\
         \t\t\"round\"        \"1\"\r\n\
         \t\t\"player_id\"    \"1\"\r\n\
         \t\t\"player_state\" \"1\"\r\n\
         \t}}\r\n}}\r\n"
    )
}

/// **Pura.** O código que está num arquivo nosso (para o servidor saber qual aceitar).
pub fn token_do_arquivo(texto: &str) -> Option<String> {
    let depois = texto.split("\"token\"").nth(1)?;
    let inicio = depois.find('"')? + 1;
    let resto = &depois[inicio..];
    let fim = resto.find('"')?;
    let t = &resto[..fim];
    (t.len() >= 16 && t.chars().all(|c| c.is_ascii_hexdigit())).then(|| t.to_string())
}

/// **Pura.** Só aceita o POST com o nosso código.
pub fn autorizado(corpo: &serde_json::Value, token: &str) -> bool {
    !token.is_empty() && corpo["auth"]["token"].as_str() == Some(token)
}

pub fn caminho_do_arquivo(pasta_do_jogo: &Path) -> PathBuf {
    pasta_do_jogo.join("game").join("csgo").join("cfg").join(NOME_DO_ARQUIVO)
}

struct Recebido {
    momento: MomentoDoCs2,
    mapa: Option<String>,
    modo: Option<String>,
    rodada: Option<u32>,
    quando: std::time::Instant,
}

static ULTIMO: Mutex<Option<Recebido>> = Mutex::new(None);
static TOKEN: Mutex<String> = Mutex::new(String::new());

fn guardar(corpo: &serde_json::Value) {
    let (momento, mapa, modo, rodada) = ler_estado(corpo);
    if let Ok(mut u) = ULTIMO.lock() {
        *u = Some(Recebido { momento, mapa, modo, rodada, quando: std::time::Instant::now() });
    }
}

/// O estado de agora, se o CS2 deu notícia nos últimos `VALIDADE_S` segundos.
pub fn estado() -> Option<EstadoDoCs2> {
    let u = ULTIMO.lock().ok()?;
    let r = u.as_ref()?;
    let ha = r.quando.elapsed().as_secs();
    (ha <= VALIDADE_S).then(|| EstadoDoCs2 {
        momento: r.momento,
        mapa: r.mapa.clone(),
        modo: r.modo.clone(),
        rodada: r.rodada,
        ha_s: ha,
    })
}

/// Para a medição automática: `Some(false)` quando o CS2 diz que agora não é jogo valendo; `None` sem notícia (aí
/// a medição segue a regra de sempre).
pub fn vale_medir_agora() -> Option<bool> {
    estado().map(|e| e.momento == MomentoDoCs2::Jogando)
}

#[cfg(target_os = "windows")]
pub mod maquina {
    use super::*;

    /// A pasta do CS2 pela biblioteca da Steam (appid 730).
    pub fn pasta_do_cs2() -> Option<PathBuf> {
        crate::modules::windows::jogos::varrer()
            .jogos
            .into_iter()
            .find(|j| j.appid == Some(730))
            .map(|j| j.pasta)
    }

    fn novo_token() -> Result<String, String> {
        let mut b = [0u8; 16];
        getrandom::getrandom(&mut b).map_err(|_| "Não consegui gerar o código da integração.".to_string())?;
        Ok(b.iter().map(|x| format!("{x:02x}")).collect())
    }

    /// Lê o código do arquivo já escrito (o servidor sobe antes de a pessoa ligar de novo).
    pub fn carregar_token() {
        if let Some(t) = pasta_do_cs2()
            .and_then(|p| std::fs::read_to_string(caminho_do_arquivo(&p)).ok())
            .and_then(|t| token_do_arquivo(&t))
        {
            if let Ok(mut g) = TOKEN.lock() {
                *g = t;
            }
        }
    }

    pub fn ligar() -> Result<PathBuf, String> {
        let pasta = pasta_do_cs2().ok_or("Não achei o Counter-Strike 2 na biblioteca da Steam.")?;
        let arquivo = caminho_do_arquivo(&pasta);
        let cfg = arquivo.parent().ok_or("Pasta do CS2 inesperada.")?;
        if !cfg.exists() {
            return Err(format!("A pasta de configuração do CS2 não existe: {}", cfg.display()));
        }
        let token = novo_token()?;
        std::fs::write(&arquivo, arquivo_de_configuracao(&token))
            .map_err(|e| format!("Não consegui escrever na pasta do CS2: {e}"))?;
        if let Ok(mut g) = TOKEN.lock() {
            *g = token;
        }
        Ok(arquivo)
    }

    pub fn desligar() -> Result<(), String> {
        if let Some(p) = pasta_do_cs2() {
            let a = caminho_do_arquivo(&p);
            if a.exists() {
                std::fs::remove_file(&a).map_err(|e| format!("Não consegui apagar {}: {e}", a.display()))?;
            }
        }
        if let Ok(mut g) = TOKEN.lock() {
            g.clear();
        }
        if let Ok(mut u) = ULTIMO.lock() {
            *u = None;
        }
        Ok(())
    }

    pub fn ligado() -> (bool, Option<PathBuf>) {
        match pasta_do_cs2() {
            Some(p) => (caminho_do_arquivo(&p).exists(), Some(p)),
            None => (false, None),
        }
    }

    /// O servidor local que recebe os POSTs do CS2. Sobe uma vez na abertura; sem arquivo ligado, nenhum POST chega.
    pub fn escutar() {
        carregar_token();
        tauri::async_runtime::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let Ok(ouvinte) = tokio::net::TcpListener::bind(("127.0.0.1", PORTA)).await else {
                return; // Porta ocupada (outro Otimiza aberto): sem integração, sem erro na tela.
            };
            loop {
                let Ok((mut conexao, _)) = ouvinte.accept().await else { continue };
                tokio::spawn(async move {
                    let mut bruto = Vec::with_capacity(4096);
                    let mut pedaco = [0u8; 4096];
                    // Cabeçalho + corpo pelo Content-Length; no máximo 64 KB (o estado do CS2 tem uns 2 KB).
                    let corpo = loop {
                        let Ok(Ok(n)) =
                            tokio::time::timeout(std::time::Duration::from_secs(2), conexao.read(&mut pedaco)).await
                        else {
                            return;
                        };
                        if n == 0 || bruto.len() > 65_536 {
                            return;
                        }
                        bruto.extend_from_slice(&pedaco[..n]);
                        if let Some(fim) = bruto.windows(4).position(|w| w == b"\r\n\r\n") {
                            let cabecalho = String::from_utf8_lossy(&bruto[..fim]).to_ascii_lowercase();
                            let tamanho = cabecalho
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length:"))
                                .and_then(|v| v.trim().parse::<usize>().ok())
                                .unwrap_or(0);
                            if bruto.len() >= fim + 4 + tamanho {
                                break bruto[fim + 4..fim + 4 + tamanho].to_vec();
                            }
                        }
                    };
                    let _ = conexao.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&corpo) {
                        let token = TOKEN.lock().map(|t| t.clone()).unwrap_or_default();
                        if autorizado(&json, &token) {
                            guardar(&json);
                        }
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    fn corpo(fase_mapa: &str, fase_rodada: &str, vida: u64, eu: &str, jogador: &str) -> serde_json::Value {
        json!({
            "provider": { "steamid": eu },
            "map": { "name": "de_mirage", "mode": "competitive", "phase": fase_mapa, "round": 4 },
            "round": { "phase": fase_rodada },
            "player": { "steamid": jogador, "state": { "health": vida } },
            "auth": { "token": "abc" }
        })
    }

    #[test]
    fn rodada_valendo_e_vivo_e_jogando() {
        let (m, mapa, modo, rodada) = ler_estado(&corpo("live", "live", 100, "1", "1"));
        assert_eq!(m, MomentoDoCs2::Jogando);
        assert_eq!(mapa.as_deref(), Some("de_mirage"));
        assert_eq!(modo.as_deref(), Some("competitive"));
        assert_eq!(rodada, Some(5));
    }

    #[test]
    fn morto_ou_assistindo_outro_nao_e_jogando() {
        assert_eq!(ler_estado(&corpo("live", "live", 0, "1", "1")).0, MomentoDoCs2::Assistindo);
        assert_eq!(ler_estado(&corpo("live", "live", 100, "1", "2")).0, MomentoDoCs2::Assistindo);
    }

    #[test]
    fn aquecimento_congelamento_e_menu() {
        assert_eq!(ler_estado(&corpo("warmup", "live", 100, "1", "1")).0, MomentoDoCs2::Pausa);
        assert_eq!(ler_estado(&corpo("live", "freezetime", 100, "1", "1")).0, MomentoDoCs2::Pausa);
        assert_eq!(ler_estado(&corpo("gameover", "over", 100, "1", "1")).0, MomentoDoCs2::Pausa);
        assert_eq!(ler_estado(&json!({ "provider": { "steamid": "1" } })).0, MomentoDoCs2::Menu);
    }

    #[test]
    fn o_arquivo_leva_o_codigo_e_o_codigo_volta_de_la() {
        let t = "0123456789abcdef0123456789abcdef";
        let a = arquivo_de_configuracao(t);
        assert!(a.contains(&format!("http://127.0.0.1:{PORTA}/cs2")));
        assert!(a.contains("\"player_state\" \"1\""));
        assert_eq!(token_do_arquivo(&a).as_deref(), Some(t));
        assert_eq!(a.matches('{').count(), a.matches('}').count());
        assert_eq!(token_do_arquivo("\"token\" \"curto\""), None);
    }

    #[test]
    fn so_aceita_o_post_com_o_codigo() {
        assert!(autorizado(&json!({ "auth": { "token": "abc" } }), "abc"));
        assert!(!autorizado(&json!({ "auth": { "token": "x" } }), "abc"));
        assert!(!autorizado(&json!({}), "abc"));
        assert!(!autorizado(&json!({ "auth": { "token": "" } }), ""), "sem código ligado, nada passa");
    }

    /// Sobe o servidor de verdade na porta e manda POSTs como o CS2 manda.
    /// `cargo test --lib o_servidor_recebe -- --ignored`
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn o_servidor_recebe_o_post_do_cs2_e_recusa_sem_codigo() {
        maquina::escutar();
        *TOKEN.lock().unwrap() = "abc".into();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let c = reqwest::Client::new();
            let url = format!("http://127.0.0.1:{PORTA}/cs2");
            let mut sem = corpo("live", "live", 100, "1", "1");
            sem["auth"]["token"] = json!("errado");
            c.post(&url).json(&sem).send().await.unwrap();
            assert!(estado().is_none(), "sem o código não guarda nada");
            let r = c.post(&url).json(&corpo("live", "live", 100, "1", "1")).send().await.unwrap();
            assert_eq!(r.status(), 200);
        });
        let e = estado().expect("o POST com o código vira estado");
        assert_eq!(e.momento, MomentoDoCs2::Jogando);
        assert_eq!(vale_medir_agora(), Some(true));
    }

    #[test]
    fn o_arquivo_fica_na_pasta_cfg_do_jogo() {
        let p = caminho_do_arquivo(Path::new("D:/Steam/steamapps/common/Counter-Strike Global Offensive"));
        assert!(p.ends_with("game/csgo/cfg/gamestate_integration_otimiza.cfg"));
    }
}
