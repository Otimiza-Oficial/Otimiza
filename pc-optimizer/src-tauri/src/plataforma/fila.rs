// OTIMIZA REDE: O PING SOB CARGA (Plano de refino, R1). "O ping sobe quando alguém usa a internet" quase sempre é
// fila: com a linha cheia, o modem guarda pacotes demais antes de mandar (bufferbloat), e o pacote do jogo espera na
// fila junto com o download. Nenhum ajuste do PC muda a fila do modem; o que muda é saber que é isso e onde se resolve
// (SQM/limite no roteador, o envio de fundo segurado durante o jogo).
//
// O teste: ping para 1.1.1.1 parado, depois baixando e depois enviando um arquivo grande do servidor de teste público
// da Cloudflare (speed.cloudflare.com), 8 s cada. A mediana de cada fase diz quanto a fila acrescenta. Só número
// medido: fase que não conseguiu encher a linha (sem rede, servidor fora) diz isso em vez de virar zero.

use serde::Serialize;

/// Quanto o ping pode subir com a linha cheia antes de se sentir no jogo.
pub const BOA_ATE_MS: f64 = 30.0;
pub const ATENCAO_ATE_MS: f64 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "ClasseDaFila"))]
pub enum Classe {
    /// A fila quase não cresce: jogar com alguém baixando não pesa.
    Boa,
    /// Cresce o bastante para sentir em jogo competitivo.
    Atencao,
    /// O ping dispara com a linha cheia: é a causa do "ping pula".
    Ruim,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "FaseDaFila"))]
pub struct Fase {
    /// Mediana do ping nesta fase, ou `None` se nenhum eco voltou.
    pub ping_ms: Option<f64>,
    /// Pacotes sem resposta, em %.
    pub perda_pct: f64,
    /// Velocidade que a fase conseguiu (Mb/s). `None` na fase parada ou quando a carga não andou.
    pub mbps: Option<f64>,
    /// Quanto subiu em relação ao parado, e a classe disso. `None` na fase parada ou sem número.
    pub a_mais_ms: Option<f64>,
    pub classe: Option<Classe>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct FilaMedida {
    pub parado: Fase,
    pub baixando: Fase,
    pub enviando: Fase,
    /// A frase do resultado, com o que fazer.
    pub frase: String,
}

/// **Pura.** A mediana em ms dos ecos que voltaram.
pub fn mediana(ecos: &[Option<u32>]) -> Option<f64> {
    let mut v: Vec<u32> = ecos.iter().flatten().copied().collect();
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    let n = v.len();
    Some(if n % 2 == 1 { v[n / 2] as f64 } else { (v[n / 2 - 1] as f64 + v[n / 2] as f64) / 2.0 })
}

/// **Pura.** Perda em % (eco sem resposta).
pub fn perda(ecos: &[Option<u32>]) -> f64 {
    if ecos.is_empty() {
        return 0.0;
    }
    100.0 * ecos.iter().filter(|e| e.is_none()).count() as f64 / ecos.len() as f64
}

/// **Pura.** Quanto o ping subiu com a carga vira uma classe.
pub fn classificar(a_mais_ms: f64) -> Classe {
    if a_mais_ms <= BOA_ATE_MS {
        Classe::Boa
    } else if a_mais_ms <= ATENCAO_ATE_MS {
        Classe::Atencao
    } else {
        Classe::Ruim
    }
}

/// **Pura.** Monta a fase com carga em relação à parada.
pub fn fase_com_carga(parado_ms: Option<f64>, ecos: &[Option<u32>], mbps: Option<f64>) -> Fase {
    let ping = mediana(ecos);
    let a_mais = match (parado_ms, ping) {
        (Some(p), Some(c)) if mbps.is_some_and(|m| m > 0.5) => Some((c - p).max(0.0)),
        _ => None,
    };
    Fase { ping_ms: ping, perda_pct: perda(ecos), mbps, a_mais_ms: a_mais, classe: a_mais.map(classificar) }
}

/// **Pura.** A frase final, pela pior das duas fases.
pub fn frase(baixando: &Fase, enviando: &Fase) -> String {
    let n = |v: f64| format!("{:.0}", v);
    let pior = [baixando, enviando]
        .into_iter()
        .filter_map(|f| f.a_mais_ms.map(|a| (a, f)))
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let Some((a_mais, _)) = pior else {
        return "Não deu para encher a linha (sem conexão com o servidor de teste): o teste não mediu a fila.".to_string();
    };
    let envio_pior = enviando.a_mais_ms.unwrap_or(0.0) >= baixando.a_mais_ms.unwrap_or(0.0);
    match classificar(a_mais) {
        Classe::Boa => format!(
            "A sua internet aguenta carga: o ping sobe no máximo {} ms com a linha cheia. Jogar com alguém baixando não pesa.",
            n(a_mais)
        ),
        Classe::Atencao => format!(
            "Com a linha cheia o ping sobe {} ms{}. Dá para sentir em jogo competitivo: segure os downloads e o envio de \
             fundo durante a partida (Durante o jogo, na tela inicial).",
            n(a_mais),
            if envio_pior { " enviando" } else { " baixando" }
        ),
        Classe::Ruim => format!(
            "É fila no modem (bufferbloat): com a linha cheia o ping sobe {} ms{}. Nenhum ajuste do PC resolve isso; o que \
             resolve é ligar o SQM (\"QoS inteligente\") no roteador, ou limitar a velocidade a uns 90% do plano. No PC, \
             segure o envio de fundo durante o jogo.",
            n(a_mais),
            if envio_pior { " enviando" } else { " baixando" }
        ),
    }
}

#[cfg(windows)]
mod medicao {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::{Duration, Instant};

    const ALVO: IpAddr = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
    const ENTRE_ECOS: Duration = Duration::from_millis(250);
    const FASE: Duration = Duration::from_secs(8);

    /// Ecos a cada 250 ms durante `quanto`, numa thread (a chamada ICMP é síncrona).
    async fn ecos_por(quanto: Duration) -> Vec<Option<u32>> {
        tokio::task::spawn_blocking(move || {
            let fim = Instant::now() + quanto;
            let mut v = Vec::new();
            while Instant::now() < fim {
                let r = crate::plataforma::rota::sondar(ALVO, 64, 1000);
                v.push(r.ms.filter(|_| matches!(r.tipo, crate::plataforma::rota::TipoResposta::Destino)));
                std::thread::sleep(ENTRE_ECOS);
            }
            v
        })
        .await
        .unwrap_or_default()
    }

    fn cliente() -> Option<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(FASE + Duration::from_secs(4))
            .user_agent(concat!("Otimiza/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok()
    }

    /// Baixa até acabar a fase e devolve os Mb/s.
    async fn baixar(cliente: &reqwest::Client) -> Option<f64> {
        let inicio = Instant::now();
        let mut r = cliente.get("https://speed.cloudflare.com/__down?bytes=250000000").send().await.ok()?;
        let mut bytes = 0u64;
        while inicio.elapsed() < FASE {
            match tokio::time::timeout(Duration::from_secs(2), r.chunk()).await {
                Ok(Ok(Some(c))) => bytes += c.len() as u64,
                _ => break,
            }
        }
        let s = inicio.elapsed().as_secs_f64();
        (s > 0.0 && bytes > 0).then(|| bytes as f64 * 8.0 / s / 1_000_000.0)
    }

    /// Envia em pedaços até acabar a fase e devolve os Mb/s.
    async fn enviar(cliente: &reqwest::Client) -> Option<f64> {
        let inicio = Instant::now();
        let pedaco = vec![0u8; 4_000_000];
        let mut bytes = 0u64;
        while inicio.elapsed() < FASE {
            let falta = FASE.saturating_sub(inicio.elapsed());
            let pedido = cliente.post("https://speed.cloudflare.com/__up").body(pedaco.clone()).send();
            match tokio::time::timeout(falta + Duration::from_secs(1), pedido).await {
                Ok(Ok(r)) if r.status().is_success() => bytes += pedaco.len() as u64,
                _ => break,
            }
        }
        let s = inicio.elapsed().as_secs_f64();
        (s > 0.0 && bytes > 0).then(|| bytes as f64 * 8.0 / s / 1_000_000.0)
    }

    pub async fn medir() -> Result<FilaMedida, String> {
        let cliente = cliente().ok_or("não consegui preparar a conexão")?;
        let parado = ecos_por(Duration::from_secs(5)).await;
        let parado_ms = mediana(&parado);
        if parado_ms.is_none() {
            return Err("O 1.1.1.1 não respondeu ao ping: sem internet, ou a rede bloqueia ping. O teste precisa dos dois.".to_string());
        }
        let (ecos_baixando, mbps_baixando) = tokio::join!(ecos_por(FASE), baixar(&cliente));
        let (ecos_enviando, mbps_enviando) = tokio::join!(ecos_por(FASE), enviar(&cliente));
        let baixando = fase_com_carga(parado_ms, &ecos_baixando, mbps_baixando);
        let enviando = fase_com_carga(parado_ms, &ecos_enviando, mbps_enviando);
        let frase = frase(&baixando, &enviando);
        Ok(FilaMedida {
            parado: Fase { ping_ms: parado_ms, perda_pct: perda(&parado), mbps: None, a_mais_ms: None, classe: None },
            baixando,
            enviando,
            frase,
        })
    }
}

#[cfg(windows)]
pub use medicao::medir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mediana_e_perda_ignoram_o_eco_perdido() {
        let e = [Some(20), None, Some(10), Some(30)];
        assert_eq!(mediana(&e), Some(20.0));
        assert_eq!(perda(&e), 25.0);
        assert_eq!(mediana(&[None, None]), None);
        assert_eq!(mediana(&[Some(10), Some(20)]), Some(15.0));
    }

    #[test]
    fn a_classe_sai_de_quanto_o_ping_sobe() {
        assert_eq!(classificar(12.0), Classe::Boa);
        assert_eq!(classificar(60.0), Classe::Atencao);
        assert_eq!(classificar(250.0), Classe::Ruim);
    }

    #[test]
    fn carga_que_nao_andou_nao_vira_numero() {
        // Sem velocidade medida, a fase não encheu a linha: nada de "subiu 0 ms".
        let f = fase_com_carga(Some(20.0), &[Some(21), Some(22)], None);
        assert_eq!(f.a_mais_ms, None);
        assert_eq!(f.classe, None);
        let g = fase_com_carga(Some(20.0), &[Some(220), Some(240)], Some(80.0));
        assert_eq!(g.a_mais_ms, Some(210.0));
        assert_eq!(g.classe, Some(Classe::Ruim));
    }

    #[test]
    fn a_frase_aponta_o_roteador_quando_e_fila() {
        let boa = fase_com_carga(Some(20.0), &[Some(25)], Some(80.0));
        let ruim = fase_com_carga(Some(20.0), &[Some(300)], Some(20.0));
        let f = frase(&boa, &ruim);
        assert!(f.contains("bufferbloat") && f.contains("roteador") && f.contains("enviando"), "{f}");
        let sem = fase_com_carga(Some(20.0), &[], None);
        assert!(frase(&sem, &sem).contains("não mediu"));
    }
}
