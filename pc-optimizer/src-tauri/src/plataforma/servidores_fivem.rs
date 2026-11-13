// OS SERVIDORES DO FIVEM PELO NÚMERO (Plano de refino, R8). Antes de entrar, a pessoa cola os códigos (`cfx.re/join/…`)
// ou os endereços dos servidores que joga, e o Otimiza mede o ping de cada um e ordena. Escolher pelo número, não pelo
// nome: o mesmo servidor "BR" pode estar em São Paulo ou em Miami.
//
// O código vira endereço pela lista pública do FiveM (`servers-frontend.fivem.net/api/servers/single/<código>`, o
// `Data.connectEndPoints`). O ping é o tempo da conexão TCP na porta do servidor (o FXServer atende TCP e UDP na mesma
// porta): a maioria não responde ping de ICMP, e o aperto de mão TCP leva uma ida e volta. Servidor atrás do proxy da
// Cfx.re mede o proxy, e a tela diz isso.

use serde::Serialize;

/// Porta padrão do FXServer.
pub const PORTA_PADRAO: u16 = 30120;
/// Servidores por medida: mais que isso a pessoa não compara.
pub const MAXIMO: usize = 12;
const TENTATIVAS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entrada {
    /// O código do `cfx.re/join/<código>`.
    Codigo(String),
    /// Endereço ou nome, com a porta.
    Endereco { host: String, porta: u16 },
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct ServidorMedido {
    /// O que a pessoa colou.
    pub entrada: String,
    /// O nome do servidor na lista do FiveM, sem os códigos de cor. `None` para endereço colado.
    pub nome: Option<String>,
    pub endereco: Option<String>,
    pub ping_ms: Option<f64>,
    /// Tentativas sem resposta, em %.
    pub falhas_pct: f64,
    pub por_proxy: bool,
    pub erro: Option<String>,
}

/// **Pura.** O que a pessoa colou vira código ou endereço. Linhas vazias e lixo ficam de fora.
pub fn ler_entrada(bruto: &str) -> Option<Entrada> {
    let t = bruto.trim().trim_end_matches('/');
    if t.is_empty() {
        return None;
    }
    let sem_esquema = t.trim_start_matches("https://").trim_start_matches("http://");
    if let Some(c) = sem_esquema.strip_prefix("cfx.re/join/") {
        return codigo_valido(c).then(|| Entrada::Codigo(c.to_lowercase()));
    }
    let sem_connect = sem_esquema.trim_start_matches("connect ").trim();
    let (host, porta) = match sem_connect.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => (h, p.parse::<u16>().ok()?),
        _ => (sem_connect, PORTA_PADRAO),
    };
    let parece_host = host.contains('.') && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if parece_host {
        return Some(Entrada::Endereco { host: host.to_lowercase(), porta });
    }
    // Só o código, sem o `cfx.re/join/`.
    codigo_valido(sem_connect).then(|| Entrada::Codigo(sem_connect.to_lowercase()))
}

fn codigo_valido(c: &str) -> bool {
    (4..=10).contains(&c.len()) && c.chars().all(|ch| ch.is_ascii_alphanumeric())
}

/// **Pura.** O nome do FiveM vem com códigos de cor (`^1`, `^7`) e às vezes com espaços repetidos.
pub fn limpar_nome(n: &str) -> String {
    let mut s = String::new();
    let mut chars = n.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '^' && chars.peek().is_some_and(|d| d.is_ascii_digit()) {
            chars.next();
            continue;
        }
        s.push(c);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// **Pura.** O primeiro ponto de conexão da lista: `ip:porta`, ou a URL do proxy da Cfx.re (porta 443).
pub fn ponto_de_conexao(dados: &serde_json::Value) -> Option<(String, u16, bool)> {
    let pontos = dados.get("Data")?.get("connectEndPoints")?.as_array()?;
    let primeiro = pontos.iter().filter_map(|p| p.as_str()).next()?;
    if primeiro.starts_with("https://") || primeiro.starts_with("http://") {
        let host = primeiro.split("://").nth(1)?.split(['/', ':']).next()?.to_string();
        return (!host.is_empty()).then_some((host, 443, true));
    }
    match ler_entrada(primeiro)? {
        Entrada::Endereco { host, porta } => Some((host, porta, false)),
        Entrada::Codigo(_) => None,
    }
}

/// **Pura.** Mediana dos tempos que voltaram e a % de falhas.
pub fn resumir(tempos: &[Option<f64>]) -> (Option<f64>, f64) {
    let mut v: Vec<f64> = tempos.iter().flatten().copied().collect();
    let falhas = if tempos.is_empty() { 0.0 } else { 100.0 * (tempos.len() - v.len()) as f64 / tempos.len() as f64 };
    if v.is_empty() {
        return (None, falhas);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    (Some(v[v.len() / 2]), falhas)
}

/// **Pura.** Medidos primeiro pelo ping; os sem número, no fim, na ordem colada.
pub fn ordenar(v: &mut [ServidorMedido]) {
    v.sort_by(|a, b| match (a.ping_ms, b.ping_ms) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}

async fn tempo_de_conexao(host: &str, porta: u16) -> Option<f64> {
    use std::time::{Duration, Instant};
    let alvo = tokio::net::lookup_host((host, porta)).await.ok()?.next()?;
    let inicio = Instant::now();
    match tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(alvo)).await {
        Ok(Ok(_)) => Some(inicio.elapsed().as_secs_f64() * 1000.0),
        _ => None,
    }
}

async fn medir_um(cliente: &reqwest::Client, bruto: String) -> ServidorMedido {
    let mut m = ServidorMedido {
        entrada: bruto.trim().to_string(),
        nome: None,
        endereco: None,
        ping_ms: None,
        falhas_pct: 0.0,
        por_proxy: false,
        erro: None,
    };
    let (host, porta) = match ler_entrada(&bruto) {
        None => {
            m.erro = Some("Não reconheci: cole o link cfx.re/join/… ou o endereço (ip:porta).".to_string());
            return m;
        }
        Some(Entrada::Endereco { host, porta }) => (host, porta),
        Some(Entrada::Codigo(c)) => {
            let url = format!("https://servers-frontend.fivem.net/api/servers/single/{}", c);
            let dados = match cliente.get(url).send().await {
                Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.ok(),
                Ok(r) if r.status().as_u16() == 404 => {
                    m.erro = Some("Esse código não está na lista do FiveM agora (servidor fora do ar ou código errado).".to_string());
                    return m;
                }
                _ => None,
            };
            let Some(dados) = dados else {
                m.erro = Some("A lista do FiveM não respondeu. Tente de novo, ou cole o endereço (ip:porta).".to_string());
                return m;
            };
            m.nome = dados
                .get("Data")
                .and_then(|d| d.get("hostname"))
                .and_then(|h| h.as_str())
                .map(limpar_nome)
                .filter(|n| !n.is_empty());
            match ponto_de_conexao(&dados) {
                Some((h, p, proxy)) => {
                    m.por_proxy = proxy;
                    (h, p)
                }
                None => {
                    m.erro = Some("A lista do FiveM não deu o endereço desse servidor.".to_string());
                    return m;
                }
            }
        }
    };
    m.endereco = Some(format!("{}:{}", host, porta));
    let mut tempos = Vec::with_capacity(TENTATIVAS);
    for _ in 0..TENTATIVAS {
        tempos.push(tempo_de_conexao(&host, porta).await);
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
    let (ping, falhas) = resumir(&tempos);
    m.ping_ms = ping;
    m.falhas_pct = falhas;
    if ping.is_none() {
        m.erro = Some("O servidor não aceitou conexão (fora do ar, ou a porta não é a do jogo).".to_string());
    }
    m
}

/// Mede todos ao mesmo tempo (cada um faz 5 conexões em sequência) e devolve do menor ping para o maior.
pub async fn medir(entradas: Vec<String>) -> Result<Vec<ServidorMedido>, String> {
    let entradas: Vec<String> = entradas.into_iter().filter(|e| !e.trim().is_empty()).take(MAXIMO).collect();
    if entradas.is_empty() {
        return Err("Cole pelo menos um servidor.".to_string());
    }
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let tarefas: Vec<_> = entradas
        .into_iter()
        .map(|e| {
            let c = cliente.clone();
            tokio::spawn(async move { medir_um(&c, e).await })
        })
        .collect();
    let mut v = Vec::with_capacity(tarefas.len());
    for t in tarefas {
        if let Ok(m) = t.await {
            v.push(m);
        }
    }
    ordenar(&mut v);
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_o_link_o_codigo_e_o_endereco() {
        assert_eq!(ler_entrada("https://cfx.re/join/Ab12cd"), Some(Entrada::Codigo("ab12cd".into())));
        assert_eq!(ler_entrada("cfx.re/join/ab12cd/"), Some(Entrada::Codigo("ab12cd".into())));
        assert_eq!(ler_entrada("ab12cd"), Some(Entrada::Codigo("ab12cd".into())));
        assert_eq!(
            ler_entrada("connect 45.10.20.30:30125"),
            Some(Entrada::Endereco { host: "45.10.20.30".into(), porta: 30125 })
        );
        assert_eq!(
            ler_entrada("play.servidor.com.br"),
            Some(Entrada::Endereco { host: "play.servidor.com.br".into(), porta: PORTA_PADRAO })
        );
        assert_eq!(ler_entrada("   "), None);
        assert_eq!(ler_entrada("isso não é servidor"), None);
    }

    #[test]
    fn o_nome_perde_os_codigos_de_cor() {
        assert_eq!(limpar_nome("^1Cidade ^7Alta  ^2RP"), "Cidade Alta RP");
    }

    #[test]
    fn o_ponto_de_conexao_sai_da_lista_e_o_proxy_e_marcado() {
        let direto = serde_json::json!({"Data": {"connectEndPoints": ["45.10.20.30:30120"]}});
        assert_eq!(ponto_de_conexao(&direto), Some(("45.10.20.30".into(), 30120, false)));
        let proxy = serde_json::json!({"Data": {"connectEndPoints": ["https://abc-xyz.users.cfx.re/"]}});
        assert_eq!(ponto_de_conexao(&proxy), Some(("abc-xyz.users.cfx.re".into(), 443, true)));
        assert_eq!(ponto_de_conexao(&serde_json::json!({})), None);
    }

    #[test]
    fn resume_e_ordena_pelo_ping() {
        assert_eq!(resumir(&[Some(30.0), None, Some(10.0), Some(20.0)]), (Some(20.0), 25.0));
        let s = |e: &str, p: Option<f64>| ServidorMedido {
            entrada: e.into(),
            nome: None,
            endereco: None,
            ping_ms: p,
            falhas_pct: 0.0,
            por_proxy: false,
            erro: None,
        };
        let mut v = vec![s("a", None), s("b", Some(90.0)), s("c", Some(12.0))];
        ordenar(&mut v);
        assert_eq!(v.iter().map(|x| x.entrada.as_str()).collect::<Vec<_>>(), ["c", "b", "a"]);
    }
}
