// OTIMIZA REDE: DE QUEM É CADA TRECHO DA ROTA. IP → ASN e nome da operadora pelo IPinfo Lite (CC BY-SA 4.0, uso
// comercial com atribuição, grátis e sem limite de consultas; `docs/planos/APIS-E-PARCERIAS.md`). A atribuição vai junto
// de todo resultado (`ATRIBUICAO`) para a tela mostrar onde o nome aparecer.
//
// Endereço privado (a sua casa) e CGNAT (100.64.0.0/10, a rede interna da operadora) não são consultados: não têm dono
// público, e mandar o endereço da casa da pessoa para fora não serve para nada. Só endereços públicos de roteadores do
// caminho e do servidor saem.
//
// A chave (token) do IPinfo: primeiro um arquivo próprio na pasta de dados do app
// (`%APPDATA%\pc-optimizer\rede\ipinfo_token.txt`), depois a variável `OTIMIZA_IPINFO_TOKEN`, depois o valor da
// compilação. Fora das Preferências de propósito: `set_preferences` grava o objeto inteiro que a tela devolve, e uma
// tela que não conhece o campo apagaria a chave. Sem chave, a rota funciona e o trecho fica sem nome.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Exigida pela licença CC BY-SA 4.0 do IPinfo Lite.
pub const ATRIBUICAO: &str = "IP address data is powered by IPinfo";
pub const ATRIBUICAO_URL: &str = "https://ipinfo.io";

const PRAZO_SEGUNDOS: u64 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operadora {
    pub asn: Option<u32>,
    /// O nome como está no registro ("TELEFONICA BRASIL S.A").
    pub nome_registrado: String,
    /// O nome que a pessoa conhece ("Vivo").
    pub nome: String,
    pub dominio: Option<String>,
    pub pais: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "estado", rename_all = "snake_case")]
pub enum Consulta {
    Encontrada {
        operadora: Operadora,
        atribuicao: &'static str,
    },
    /// Casa, CGNAT, loopback: não se consulta.
    EnderecoNaoPublico,
    /// Sem chave do IPinfo configurada.
    SemChave,
    Falhou {
        motivo: String,
    },
}

/// **Pura.** 100.64.0.0/10: o endereço da rede interna da operadora (CGNAT), não da internet.
pub fn e_cgnat(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 100 && (o[1] & 0xC0) == 64
        }
        IpAddr::V6(_) => false,
    }
}

/// **Pura.** Privado da casa (10/8, 172.16/12, 192.168/16, fc00::/7) ou link-local.
pub fn e_privado(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            let s = v6.segments()[0];
            (s & 0xFE00) == 0xFC00 || (s & 0xFFC0) == 0xFE80
        }
    }
}

/// **Pura.** Só endereço com dono público é consultado.
pub fn e_publico(ip: &IpAddr) -> bool {
    let especial = match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                || v4.octets()[0] == 0
                || v4.octets()[0] >= 240
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || v6.is_multicast(),
    };
    !especial && !e_privado(ip) && !e_cgnat(ip)
}

/// **Pura.** O nome que a pessoa conhece. Primeiro pelo número do sistema autônomo (estável), depois pelo nome
/// registrado; sem correspondência, o nome registrado sem "S.A."/"LTDA".
pub fn nome_popular(asn: Option<u32>, registrado: &str) -> String {
    let por_asn = match asn {
        Some(26599 | 18881) => Some("Vivo"),
        Some(28573 | 4230) => Some("Claro"),
        Some(26615) => Some("TIM"),
        Some(7738) => Some("Oi"),
        Some(16735) => Some("Algar"),
        Some(28126) => Some("Brisanet"),
        _ => None,
    };
    if let Some(n) = por_asn {
        return n.to_string();
    }
    let r = registrado.to_lowercase();
    const PELO_NOME: [(&str, &str); 14] = [
        ("telefonica brasil", "Vivo"),
        ("telefônica brasil", "Vivo"),
        ("global village telecom", "Vivo"),
        ("claro", "Claro"),
        ("net servicos", "Claro"),
        ("embratel", "Claro"),
        ("tim s/a", "TIM"),
        ("tim s.a", "TIM"),
        ("telemar", "Oi"),
        ("brasil telecom", "Oi"),
        ("algar", "Algar"),
        ("brisanet", "Brisanet"),
        ("unifique", "Unifique"),
        ("ligga", "Ligga"),
    ];
    if let Some((_, n)) = PELO_NOME.iter().find(|(k, _)| r.contains(k)) {
        return n.to_string();
    }
    limpar(registrado)
}

fn limpar(nome: &str) -> String {
    let mut s = nome.trim().trim_end_matches(['.', ',']).to_string();
    loop {
        let antes = s.len();
        for sufixo in [
            " S.A", " S/A", " SA", " LTDA", " Ltda", " ltda", " EIRELI", " Inc", " LLC", " Ltd",
            " ME", ",",
        ] {
            if s.len() > sufixo.len() && s.ends_with(sufixo) {
                s.truncate(s.len() - sufixo.len());
                s = s
                    .trim_end()
                    .trim_end_matches(['.', ',', '-'])
                    .trim_end()
                    .to_string();
            }
        }
        if s.len() == antes {
            break;
        }
    }
    if s.is_empty() {
        nome.trim().to_string()
    } else {
        s
    }
}

#[derive(Deserialize)]
struct RespostaLite {
    asn: Option<String>,
    as_name: Option<String>,
    as_domain: Option<String>,
    country_code: Option<String>,
}

/// **Pura.** A resposta do `api.ipinfo.io/lite/{ip}`. Endereço sem dono (ex.: anycast sem registro) volta sem `asn`.
pub fn ler_resposta(json: &str) -> Result<Operadora, String> {
    let r: RespostaLite =
        serde_json::from_str(json).map_err(|e| format!("resposta ilegível: {}", e))?;
    let registrado = r
        .as_name
        .filter(|s| !s.trim().is_empty())
        .ok_or("o IPinfo não conhece o dono deste endereço")?;
    let asn = r.asn.as_deref().and_then(|a| {
        a.trim()
            .trim_start_matches(['A', 'S', 'a', 's'])
            .parse()
            .ok()
    });
    Ok(Operadora {
        nome: nome_popular(asn, &registrado),
        asn,
        nome_registrado: registrado,
        dominio: r.as_domain.filter(|s| !s.is_empty()),
        pais: r.country_code.filter(|s| !s.is_empty()),
    })
}

fn arquivo_da_chave() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("rede")
        .join("ipinfo_token.txt")
}

/// **Pura.** A primeira chave não vazia, na ordem: arquivo, variável de ambiente, compilação.
pub fn escolher_chave(
    arquivo: Option<String>,
    ambiente: Option<String>,
    compilacao: Option<&str>,
) -> Option<String> {
    [arquivo, ambiente, compilacao.map(str::to_string)]
        .into_iter()
        .flatten()
        .map(|s| s.trim().to_string())
        .find(|s| !s.is_empty())
}

pub fn chave() -> Option<String> {
    escolher_chave(
        std::fs::read_to_string(arquivo_da_chave()).ok(),
        std::env::var("OTIMIZA_IPINFO_TOKEN").ok(),
        option_env!("OTIMIZA_IPINFO_TOKEN"),
    )
}

/// Só na memória e só o que deu certo: o dono de um endereço não muda durante a sessão.
fn cache() -> &'static Mutex<HashMap<IpAddr, Operadora>> {
    static C: OnceLock<Mutex<HashMap<IpAddr, Operadora>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn encontrada(operadora: Operadora) -> Consulta {
    Consulta::Encontrada {
        operadora,
        atribuicao: ATRIBUICAO,
    }
}

/// De quem é este endereço.
pub async fn consultar(ip: IpAddr) -> Consulta {
    if !e_publico(&ip) {
        return Consulta::EnderecoNaoPublico;
    }
    if let Some(o) = cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&ip)
        .cloned()
    {
        return encontrada(o);
    }
    let Some(chave) = chave() else {
        return Consulta::SemChave;
    };
    match buscar(ip, &chave).await {
        Ok(o) => {
            cache()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(ip, o.clone());
            encontrada(o)
        }
        Err(motivo) => Consulta::Falhou { motivo },
    }
}

async fn buscar(ip: IpAddr, chave: &str) -> Result<Operadora, String> {
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(PRAZO_SEGUNDOS))
        .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    // A chave no cabeçalho, não na URL: URL acaba em log.
    let resposta = cliente
        .get(format!("https://api.ipinfo.io/lite/{}", ip))
        .bearer_auth(chave)
        .send()
        .await
        .map_err(|e| format!("sem resposta do IPinfo: {}", e))?;
    let status = resposta.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("o IPinfo recusou a chave".into());
    }
    if status.as_u16() == 429 {
        return Err("o IPinfo pediu para esperar".into());
    }
    if !status.is_success() {
        return Err(format!("o IPinfo respondeu {}", status.as_u16()));
    }
    let texto = resposta.text().await.map_err(|e| e.to_string())?;
    ler_resposta(&texto)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn casa_e_cgnat_nao_saem_do_pc() {
        for s in [
            "192.168.0.1",
            "10.0.0.1",
            "172.20.1.1",
            "100.64.0.1",
            "100.127.255.254",
            "169.254.1.1",
            "127.0.0.1",
            "fe80::1",
            "fd00::1",
            "::1",
        ] {
            assert!(!e_publico(&ip(s)), "{}", s);
        }
        for s in [
            "8.8.8.8",
            "177.79.1.1",
            "100.63.255.255",
            "100.128.0.1",
            "2804:14c::1",
        ] {
            assert!(e_publico(&ip(s)), "{}", s);
        }
        assert!(e_cgnat(&ip("100.100.1.1")));
        assert!(!e_cgnat(&ip("100.128.1.1")));
        assert!(e_privado(&ip("192.168.1.1")));
        assert!(!e_privado(&ip("100.64.0.1")), "CGNAT não é a casa");
    }

    #[test]
    fn le_a_resposta_do_lite() {
        let json = r#"{"ip":"177.79.1.1","asn":"AS26599","as_name":"TELEFONICA BRASIL S.A","as_domain":"telefonica.com.br","country_code":"BR","country":"Brazil","continent_code":"SA","continent":"South America"}"#;
        let o = ler_resposta(json).unwrap();
        assert_eq!(o.asn, Some(26599));
        assert_eq!(o.nome, "Vivo");
        assert_eq!(o.nome_registrado, "TELEFONICA BRASIL S.A");
        assert_eq!(o.pais.as_deref(), Some("BR"));
        assert_eq!(o.dominio.as_deref(), Some("telefonica.com.br"));
    }

    #[test]
    fn endereco_sem_dono_e_erro_e_nao_operadora_vazia() {
        assert!(ler_resposta(r#"{"ip":"1.2.3.4"}"#).is_err());
        assert!(ler_resposta(r#"{"ip":"1.2.3.4","as_name":""}"#).is_err());
        assert!(ler_resposta("<html>").is_err());
    }

    #[test]
    fn nomes_que_a_pessoa_conhece() {
        assert_eq!(nome_popular(Some(26599), "qualquer"), "Vivo");
        assert_eq!(
            nome_popular(Some(28573), "Claro NXT Telecomunicacoes Ltda"),
            "Claro"
        );
        assert_eq!(nome_popular(Some(26615), "TIM S/A"), "TIM");
        assert_eq!(nome_popular(None, "TELEMAR NORTE LESTE S.A."), "Oi");
        assert_eq!(nome_popular(None, "Claro S.A."), "Claro");
        assert_eq!(nome_popular(Some(1), "Algar Telecom S/A"), "Algar");
    }

    #[test]
    fn sem_correspondencia_fica_o_registrado_sem_sufixo() {
        assert_eq!(nome_popular(Some(13335), "Cloudflare, Inc."), "Cloudflare");
        assert_eq!(
            nome_popular(None, "Provedor Exemplo Internet LTDA"),
            "Provedor Exemplo Internet"
        );
        assert_eq!(nome_popular(None, "Google LLC"), "Google");
        assert_eq!(nome_popular(None, "LTDA"), "LTDA", "nunca fica vazio");
    }

    #[test]
    fn a_chave_do_arquivo_vence_a_variavel_que_vence_a_compilacao() {
        assert_eq!(
            escolher_chave(Some("a\n".into()), Some("b".into()), Some("c")),
            Some("a".into())
        );
        assert_eq!(
            escolher_chave(Some("  ".into()), Some("b".into()), Some("c")),
            Some("b".into())
        );
        assert_eq!(escolher_chave(None, None, Some("c")), Some("c".into()));
        assert_eq!(escolher_chave(None, Some("".into()), None), None);
    }

    #[test]
    fn a_atribuicao_e_a_que_a_licenca_pede() {
        assert_eq!(ATRIBUICAO, "IP address data is powered by IPinfo");
    }

    #[tokio::test]
    async fn endereco_da_casa_nunca_e_consultado() {
        assert_eq!(
            consultar(ip("192.168.0.1")).await,
            Consulta::EnderecoNaoPublico
        );
        assert_eq!(
            consultar(ip("100.64.1.1")).await,
            Consulta::EnderecoNaoPublico
        );
    }
}

#[cfg(test)]
mod na_maquina {
    /// Consulta de verdade (precisa de chave): `cargo test --lib -- --ignored operadora::na_maquina`.
    #[tokio::test]
    #[ignore]
    async fn consulta_real() {
        println!("{:#?}", super::consultar("8.8.8.8".parse().unwrap()).await);
    }
}
