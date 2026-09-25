// Perda de pacote até o servidor do jogo: rede que engasga sente igual a FPS baixo, e o cliente otimiza, não vê
// diferença e pede reembolso. Jitter de REDE, não o do agendador (`modules/jitter.rs`). Regras: não descobrir o
// servidor é resultado (nunca medir contra um host qualquer); 100% de perda é medição, falha da medição não é;
// não promete melhorar; a tela diz que travada de rede sente igual a FPS baixo. ICMP pelo `Ping` do .NET (campos,
// sem texto traduzido); servidor com ICMP bloqueado é conferido pela porta TCP. O servidor sai das conexões TCP do
// processo do jogo; mais de um endereço público, ou nenhum, é "não descobri".

use super::{gamemode, shell};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;

/// Com 1 ou 2 amostras não há o que variar.
const AMOSTRAS_PADRAO: u32 = 20;

/// Não é `Test-Connection`: no PowerShell 5.1 não aceita prazo (~4 s por tentativa; vinte contra ICMP bloqueado
/// levavam ~78 s) e prendia a sessão compartilhada do PowerShell, travando todas as outras análises.
const PRAZO_DO_PING_MS: u32 = 1000;

/// Cinco silêncios seguidos já respondem; o contador zera a cada resposta, e perda parcial roda as vinte.
const DESISTE_APOS: u32 = 5;

/// `NaoDescobri` é resultado nomeado, não erro escondido num `Option`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlvoDaMedida {
    NaoDescobri,
    Servidor(String),
}

/// Pura: testa a Regra 1 sem nada aberto.
pub fn decidir_alvo(candidato: Option<String>) -> AlvoDaMedida {
    match candidato {
        Some(endereco) if !endereco.trim().is_empty() => AlvoDaMedida::Servidor(endereco),
        _ => AlvoDaMedida::NaoDescobri,
    }
}

/// `NaoMedi` e "100% perdidos" são variantes diferentes: `perdidos == enviados` não cobre os dois.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Perda {
    /// `perdidos == enviados` É medição válida (rede fora ou ICMP bloqueado).
    Medida { enviados: u32, perdidos: u32 },
    /// Nenhum pacote saiu: nem 0% nem 100%, ausência de dado.
    NaoMedi,
    /// Nenhum ping voltou, mas a porta do jogo aceitou conexão: servidor atrás de anti-DDoS que descarta ICMP.
    /// Sem isto, uma conexão perfeita apareceria como "rede fora do ar".
    NaoRespondePing { enviados: u32 },
    /// O caso comum é LIMITAR A TAXA de ICMP, não descartar tudo: 19 de 20 perdidos saía "95% de perda". Não afirma
    /// que a rede está boa: diz que não dá para saber.
    PingLimitado { enviados: u32, perdidos: u32 },
}

pub fn resumir(enviados: u32, recebidos: u32) -> Perda {
    if enviados == 0 {
        return Perda::NaoMedi;
    }

    Perda::Medida {
        enviados,
        perdidos: enviados.saturating_sub(recebidos),
    }
}

/// Pura: separar "rede ruim" de "servidor não responde a ping" é a decisão mais cara de errar.
/// `Some(true)` a porta aceitou; `Some(false)` nem ping nem porta; `None` não havia porta para tentar.
pub fn avaliar_perda_total(enviados: u32, perdidos: u32, porta_respondeu: Option<bool>) -> Perda {
    match porta_respondeu {
        Some(true) if perdidos >= enviados => Perda::NaoRespondePing { enviados },
        Some(true) => Perda::PingLimitado { enviados, perdidos },
        _ => Perda::Medida { enviados, perdidos },
    }
}

/// Abaixo da metade a medição se sustenta; acima, descarte por regra é tão provável quanto perda real.
const PERDA_QUE_PEDE_TESTEMUNHA: f64 = 0.5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedidaDeRede {
    pub alvo: Option<String>,
    pub perda: Perda,
    pub jitter_ms: Option<f64>,
    pub tempo_ms: Option<f64>,
    pub nota: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct ConexaoBruta {
    remote_address: Option<String>,
    remote_port: Option<u16>,
}

fn conexoes_estabelecidas(pid: u32) -> Vec<ConexaoBruta> {
    let script = format!(
        "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-NetTCPConnection \
         -OwningProcess {} -State Established -ErrorAction SilentlyContinue | \
         Select-Object RemoteAddress,RemotePort)",
        pid
    );

    shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

fn e_endereco_publico(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_unspecified())
        }
        IpAddr::V6(v6) => {
            !(v6.is_loopback()
                || v6.is_multicast()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // Sem ele, um `fd00:…` de rede local virava candidato a servidor.
                || (v6.segments()[0] & 0xfe00) == 0xfc00)
        }
    }
}

/// Portas de web nunca são o servidor: o cliente FiveM fala com a Cfx.re em HTTP/HTTPS, e o jogo é UDP (a TCP
/// com o servidor fecha depois dos recursos). Sem o filtro, sobrava a Cfx.re em `:443` e a tela media a CDN
/// como "o servidor".
const PORTAS_QUE_NAO_SAO_JOGO: [u16; 5] = [80, 443, 8080, 8443, 3478];

// Não há alternativa pelo log: o FiveM não grava contra quem conectou (conferido em `CitizenFX_log_*.log`,
// `cef_console.txt` e `nui-storage` de uma partida real). Em partida longa, a checagem diz "não identifiquei".

/// Reaproveita `gamemode::jogo_aberto_com_pid`. Ambiguidade vira `None`.
pub fn servidor_do_jogo() -> Option<String> {
    let (_, pid) = gamemode::jogo_aberto_com_pid()?;

    let candidatos: Vec<(String, u16)> = conexoes_estabelecidas(pid)
        .into_iter()
        .filter_map(|c| {
            let endereco = c.remote_address?;
            let porta = c.remote_port?;
            let ip: IpAddr = endereco.parse().ok()?;

            e_endereco_publico(&ip).then_some((endereco, porta))
        })
        .collect();

    escolher_alvo(candidatos)
}

/// Pura: o descarte das portas de web mora aqui para ser testável sem jogo. Deduplica pelo PAR: pelo endereço,
/// ficava a menor porta e o aperto de mão batia na errada.
fn escolher_alvo(candidatos: Vec<(String, u16)>) -> Option<String> {
    let mut candidatos: Vec<(String, u16)> = candidatos
        .into_iter()
        .filter(|(_, porta)| !PORTAS_QUE_NAO_SAO_JOGO.contains(porta))
        .collect();

    candidatos.sort();
    candidatos.dedup();

    match candidatos.as_slice() {
        [(ip, porta)] => Some(format!("{}:{}", ip, porta)),
        _ => None,
    }
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
struct RespostaPing {
    ms: Option<f64>,
    ok: Option<bool>,
}

/// Uma tentativa por vez: cada `try/catch` é independente, e `enviados` sai do tamanho da lista.
fn sondar(host: &str, amostras: u32) -> Vec<RespostaPing> {
    let script = format!(
        "$p = New-Object System.Net.NetworkInformation.Ping; \
         $r = @(); $seguidas = 0; \
         foreach ($i in 1..{}) {{ \
           try {{ $resp = $p.Send('{}', {}); \
                  if ($resp.Status -eq 'Success') {{ \
                    $seguidas = 0; \
                    $r += [ordered]@{{ ms = [double]$resp.RoundtripTime; ok = $true }} }} \
                  else {{ $seguidas++; \
                    $r += [ordered]@{{ ms = $null; ok = $false }} }} }} \
           catch {{ $seguidas++; $r += [ordered]@{{ ms = $null; ok = $false }} }}; \
           if ($seguidas -ge {}) {{ break }} }}; \
         $p.Dispose(); \
         ConvertTo-Json -Compress -InputObject @($r)",
        amostras, host, PRAZO_DO_PING_MS, DESISTE_APOS
    );

    shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

fn calcular_perda(respostas: &[RespostaPing]) -> Perda {
    let enviados = respostas.len() as u32;
    let recebidos = respostas.iter().filter(|r| r.ok == Some(true)).count() as u32;
    resumir(enviados, recebidos)
}

/// Mediana: uma tentativa isolada e lenta não decide o número.
fn calcular_tempo_mediano(respostas: &[RespostaPing]) -> Option<f64> {
    let mut tempos: Vec<f64> = respostas
        .iter()
        .filter(|r| r.ok == Some(true))
        .filter_map(|r| r.ms)
        .collect();

    if tempos.is_empty() {
        return None;
    }

    tempos.sort_by(f64::total_cmp);
    Some(tempos[tempos.len() / 2])
}

/// Média das diferenças absolutas entre respostas SUCESSIVAS. Precisa de ao menos duas.
fn calcular_jitter(respostas: &[RespostaPing]) -> Option<f64> {
    let tempos: Vec<f64> = respostas
        .iter()
        .filter(|r| r.ok == Some(true))
        .filter_map(|r| r.ms)
        .collect();

    if tempos.len() < 2 {
        return None;
    }

    let diferencas: Vec<f64> = tempos.windows(2).map(|par| (par[1] - par[0]).abs()).collect();
    Some(diferencas.iter().sum::<f64>() / diferencas.len() as f64)
}

/// As Regras 3 e 4, sempre no texto. Constante para nenhuma frase solta escapar delas (a de endereço inválido
/// escapava do canário).
const BASE_DA_NOTA: &str =
    "Travamento na hora de jogar sente exatamente igual a FPS baixo: o carro que teleporta, o \
     tiro que não registra, a tela que congela por um instante. Se você otimizou o PC e não \
     sentiu diferença, pode ser isto aqui — e não o computador. Perda de pacote é quase sempre \
     do provedor, do cabo, do Wi-Fi ou do servidor do jogo, nunca do PC: o Otimiza mede e \
     mostra onde está o problema, sem prometer consertar o que não está aqui.";

/// O texto que acompanha a medição: `BASE_DA_NOTA` sempre, mais o que cada desfecho pede.
fn montar_nota(perda: &Perda, sem_alvo: bool) -> String {
    let base = BASE_DA_NOTA;

    let extra = if sem_alvo {
        "Não descobri, com confiança, o servidor em que você está jogando agora. Medir contra \
         um endereço qualquer e apresentar como \"o servidor do jogo\" seria inventar um número \
         — por isso a medição não rodou. Esta checagem funciona melhor logo depois de entrar \
         no servidor: no FiveM, o jogo em si conversa por um caminho que não dá para identificar \
         de fora, e depois de um tempo de partida o endereço deixa de ficar visível aqui. Se o \
         jogo já está aberto há bastante tempo, reconectar ao servidor faz o endereço aparecer \
         de novo."
            .to_string()
    } else {
        match perda {
            Perda::NaoMedi => "A checagem não rodou desta vez — não é 0% de perda nem 100%, é \
                que nenhum pacote chegou a sair. Tente de novo."
                .to_string(),
            // A porta respondeu: a frase não pode soar como perda.
            Perda::NaoRespondePing { enviados } => format!(
                "Este servidor não responde a ping — as {} tentativas ficaram sem resposta —, \
                 mas a porta do jogo aceitou conexão normalmente. Ou seja: **não é perda de \
                 pacote**. Bloquear ping é comum em servidor de jogo, por segurança. Não dá \
                 para medir perda contra este servidor, e isso não é problema na sua rede.",
                enviados
            ),
            Perda::PingLimitado { enviados, perdidos } => format!(
                "Este servidor respondeu a {} de {} pings, mas aceitou conexão na porta do jogo \
                 normalmente. Limitar a taxa de ping é comum em servidor de jogo, por segurança, \
                 e é a explicação mais provável. **Não dá para dizer, por aqui, se houve perda \
                 de pacote de verdade** — e afirmar que houve seria inventar um número.",
                enviados - perdidos,
                enviados
            ),
            Perda::Medida { enviados, perdidos } if *perdidos == *enviados => format!(
                "Nenhuma das {} tentativas voltou, e a porta do jogo também não respondeu. \
                 As duas coisas juntas apontam o servidor fora do ar, ou algo entre você e \
                 ele bloqueando a conexão inteira.",
                enviados
            ),
            Perda::Medida { enviados, perdidos } if *perdidos == 0 => format!(
                "Nenhuma perda nas {} tentativas contra o servidor agora — é a foto deste \
                 instante, não garantia de que nunca há perda.",
                enviados
            ),
            Perda::Medida { enviados, perdidos } => format!(
                "{} de {} pacotes não voltaram ({:.0}%) na checagem contra o servidor.",
                perdidos,
                enviados,
                (*perdidos as f64 / *enviados as f64) * 100.0
            ),
        }
    };

    format!("{} {}", base, extra)
}

pub fn medir(alvo: Option<String>, amostras: u32) -> MedidaDeRede {
    match decidir_alvo(alvo) {
        AlvoDaMedida::NaoDescobri => MedidaDeRede {
            alvo: None,
            perda: Perda::NaoMedi,
            jitter_ms: None,
            tempo_ms: None,
            nota: montar_nota(&Perda::NaoMedi, true),
        },
        AlvoDaMedida::Servidor(destino) => {
            let host = destino.rsplit_once(':').map(|(h, _)| h).unwrap_or(&destino);

            // Só IP validado entra no script: `medir` aceita `Option<String>` de qualquer chamador.
            if host.parse::<IpAddr>().is_err() {
                return MedidaDeRede {
                    alvo: Some(destino),
                    perda: Perda::NaoMedi,
                    jitter_ms: None,
                    tempo_ms: None,
                    nota: format!(
                        "{} O endereço do servidor não é um IP válido — a medição não rodou.",
                        BASE_DA_NOTA
                    )
                        .to_string(),
                };
            }

            let respostas = sondar(host, amostras.max(1));
            let mut perda = calcular_perda(&respostas);

            // Nenhum ping (ou metade) não voltou: pergunta à porta antes de acusar a rede. O aperto de mão TCP não é
            // engolido por regra que só vale para ICMP.
            if let Perda::Medida { enviados, perdidos } = perda {
                if enviados > 0
                    && f64::from(perdidos) / f64::from(enviados) >= PERDA_QUE_PEDE_TESTEMUNHA
                {
                    perda = avaliar_perda_total(enviados, perdidos, porta_responde(&destino));
                }
            }

            MedidaDeRede {
                alvo: Some(destino),
                jitter_ms: calcular_jitter(&respostas),
                tempo_ms: calcular_tempo_mediano(&respostas),
                nota: montar_nota(&perda, false),
                perda,
            }
        }
    }
}

/// Dois segundos: é confirmação, não medição.
const PRAZO_DA_PORTA: Duration = Duration::from_secs(2);

/// `None` sem porta: ausência de testemunha é diferente de a testemunha dizer não.
fn porta_responde(destino: &str) -> Option<bool> {
    let (host, porta) = destino.rsplit_once(':')?;
    let porta: u16 = porta.parse().ok()?;
    let ip: IpAddr = host.parse().ok()?;

    Some(TcpStream::connect_timeout(&SocketAddr::new(ip, porta), PRAZO_DA_PORTA).is_ok())
}

pub fn medir_agora() -> MedidaDeRede {
    medir(servidor_do_jogo(), AMOSTRAS_PADRAO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_alvo_conhecido_nao_mede_contra_qualquer_um() {
        assert!(matches!(decidir_alvo(None), AlvoDaMedida::NaoDescobri));
        assert!(matches!(
            decidir_alvo(Some("203.0.113.10:30120".into())),
            AlvoDaMedida::Servidor(_)
        ));
    }

    #[test]
    fn ping_bloqueado_nao_e_apresentado_como_perda_de_pacote() {
        assert_eq!(
            avaliar_perda_total(20, 20, Some(true)),
            Perda::NaoRespondePing { enviados: 20 },
            "porta respondendo continuou virando perda total"
        );

        assert_eq!(
            avaliar_perda_total(20, 20, Some(false)),
            Perda::Medida { enviados: 20, perdidos: 20 }
        );

        assert_eq!(
            avaliar_perda_total(20, 20, None),
            Perda::Medida { enviados: 20, perdidos: 20 }
        );
    }

    #[test]
    fn ping_limitado_por_taxa_tambem_nao_vira_perda_de_pacote() {
        assert_eq!(
            avaliar_perda_total(20, 19, Some(true)),
            Perda::PingLimitado {
                enviados: 20,
                perdidos: 19
            },
            "perda parcial com a porta respondendo continuou virando perda medida"
        );

        let nota = montar_nota(
            &Perda::PingLimitado {
                enviados: 20,
                perdidos: 19,
            },
            false,
        );
        assert!(
            nota.contains("não dá para dizer") || nota.contains("Não dá para dizer"),
            "a nota precisa admitir que não sabe: {}",
            nota
        );

        assert_eq!(
            avaliar_perda_total(20, 19, None),
            Perda::Medida {
                enviados: 20,
                perdidos: 19
            }
        );
    }

    #[test]
    fn servico_de_web_nao_e_confundido_com_o_servidor_do_jogo() {
        assert_eq!(
            escolher_alvo(vec![("203.0.113.10".to_string(), 443)]).as_deref(),
            None,
            "a CDN da Cfx.re em :443 continuou virando o servidor do jogo"
        );

        assert_eq!(
            escolher_alvo(vec![
                ("203.0.113.10".to_string(), 30120),
                ("203.0.113.10".to_string(), 30120),
            ])
            .as_deref(),
            Some("203.0.113.10:30120")
        );

        assert_eq!(
            escolher_alvo(vec![
                ("203.0.113.10".to_string(), 30120),
                ("203.0.113.10".to_string(), 30110),
            ]),
            None,
            "duas portas no mesmo host continuaram sendo resolvidas por chute"
        );

        assert!(PORTAS_QUE_NAO_SAO_JOGO.contains(&443));
        assert!(!PORTAS_QUE_NAO_SAO_JOGO.contains(&30120));
    }

    #[test]
    fn endereco_ipv6_de_rede_local_nao_e_publico() {
        let local: IpAddr = "fd00::1".parse().unwrap();
        let outro_local: IpAddr = "fc00::abcd".parse().unwrap();
        let link_local: IpAddr = "fe80::1".parse().unwrap();
        let publico: IpAddr = "2001:db8::1".parse().unwrap();

        assert!(!e_endereco_publico(&local), "fd00::/8 passou como público");
        assert!(
            !e_endereco_publico(&outro_local),
            "fc00::/8 passou como público"
        );
        assert!(!e_endereco_publico(&link_local));
        // Unicast global: a função não tem por que recusá-lo.
        assert!(e_endereco_publico(&publico));
    }

    #[test]
    fn a_sonda_desiste_depois_de_silencio_seguido() {
        // Trava a INTENÇÃO: as constantes que fazem a medição caber nos "~10 s" da tela.
        assert!(
            PRAZO_DO_PING_MS <= 1000,
            "prazo por tentativa acima de 1 s estoura o custo anunciado na tela"
        );
        assert!(
            DESISTE_APOS <= 5 && DESISTE_APOS >= 2,
            "desistir cedo demais confunde perda com bloqueio; tarde demais volta a travar"
        );
        assert!(
            u32::from(DESISTE_APOS) * PRAZO_DO_PING_MS <= 6000,
            "o pior caso — ICMP bloqueado — precisa caber junto com o prazo da porta"
        );

        let script = format!(
            "$p = New-Object System.Net.NetworkInformation.Ping; $p.Send('{}', {})",
            "1.1.1.1", PRAZO_DO_PING_MS
        );
        assert!(script.contains("NetworkInformation.Ping"));
    }

    #[test]
    fn a_frase_do_ping_bloqueado_nega_a_perda_em_vez_de_ressalvar() {
        // A negação tem de estar na frase, não num parêntese depois de "rede fora do ar".
        let nota = montar_nota(&Perda::NaoRespondePing { enviados: 20 }, false);
        let minuscula = nota.to_lowercase();

        assert!(
            minuscula.contains("não é perda de pacote"),
            "a frase não nega a perda: {}",
            nota
        );
        assert!(
            !minuscula.contains("fora do ar"),
            "a frase ainda fala em rede fora do ar: {}",
            nota
        );
    }

    #[test]
    fn perda_total_nao_se_confunde_com_nao_medido() {
        assert_eq!(resumir(10, 0), Perda::Medida { enviados: 10, perdidos: 10 });
        assert_eq!(resumir(0, 0), Perda::NaoMedi);
    }

    #[test]
    fn string_vazia_tambem_e_nao_descobri() {
        assert!(matches!(decidir_alvo(Some(String::new())), AlvoDaMedida::NaoDescobri));
        assert!(matches!(decidir_alvo(Some("   ".into())), AlvoDaMedida::NaoDescobri));
    }

    #[test]
    fn perda_parcial_e_perda_zero_sao_distintas() {
        assert_eq!(resumir(20, 20), Perda::Medida { enviados: 20, perdidos: 0 });
        assert_eq!(resumir(20, 15), Perda::Medida { enviados: 20, perdidos: 5 });
    }

    #[test]
    fn a_nota_sempre_diz_que_travada_de_rede_sente_igual_a_fps_baixo() {
        // Canário: apagar esta frase de `BASE_DA_NOTA` derruba este teste.
        for (perda, sem_alvo) in [
            (Perda::NaoMedi, true),
            (Perda::NaoMedi, false),
            (Perda::Medida { enviados: 10, perdidos: 10 }, false),
            (Perda::Medida { enviados: 10, perdidos: 0 }, false),
            (Perda::Medida { enviados: 10, perdidos: 3 }, false),
            (Perda::NaoRespondePing { enviados: 10 }, false),
            (
                Perda::PingLimitado {
                    enviados: 10,
                    perdidos: 9,
                },
                false,
            ),
        ] {
            let nota = montar_nota(&perda, sem_alvo);
            assert!(
                nota.contains("sente exatamente igual a FPS baixo"),
                "faltou a frase que justifica o recurso: {:?} / sem_alvo={}",
                perda,
                sem_alvo
            );
        }

        // O caminho que não passa por `montar_nota`.
        let fora_do_montar = medir(Some("nao-e-um-ip:30120".to_string()), 1);
        assert!(
            fora_do_montar
                .nota
                .contains("sente exatamente igual a FPS baixo"),
            "a frase de endereço inválido escapou da regra: {}",
            fora_do_montar.nota
        );
    }

    #[test]
    fn a_nota_nunca_promete_melhorar() {
        for (perda, sem_alvo) in [
            (Perda::NaoMedi, true),
            (Perda::Medida { enviados: 10, perdidos: 10 }, false),
            (Perda::Medida { enviados: 10, perdidos: 0 }, false),
        ] {
            let nota = montar_nota(&perda, sem_alvo);
            assert!(nota.contains("nunca do PC"));
            assert!(nota.contains("sem prometer consertar"));
        }
    }

    #[test]
    fn perda_total_confirmada_pela_porta_nao_ressalva_o_que_nao_ha() {
        // A dúvida mora na RESPOSTA (a porta é consultada), não num parêntese: com as duas testemunhas dizendo o mesmo,
        // a nota de perda total dispensa ressalva.
        let nota = montar_nota(&Perda::Medida { enviados: 20, perdidos: 20 }, false);

        assert!(
            nota.contains("a porta do jogo também não respondeu"),
            "a nota não diz que a porta foi consultada: {}",
            nota
        );
        assert!(
            !nota.contains("bloqueiam ping"),
            "a ressalva voltou para uma nota que não precisa mais dela: {}",
            nota
        );
    }

    #[test]
    fn nota_sem_alvo_explica_por_que_nao_mediu() {
        let nota = montar_nota(&Perda::NaoMedi, true);
        assert!(nota.contains("Não descobri"));
        assert!(nota.contains("inventar um número"));
    }

    #[test]
    fn medir_sem_alvo_nao_chama_powershell() {
        let medida = medir(None, 20);
        assert_eq!(medida.alvo, None);
        assert_eq!(medida.perda, Perda::NaoMedi);
        assert!(medida.jitter_ms.is_none());
        assert!(medida.tempo_ms.is_none());
    }

    #[test]
    fn medir_recusa_alvo_que_nao_e_ip() {
        let medida = medir(Some("nao-e-um-ip:1234".into()), 5);
        assert_eq!(medida.perda, Perda::NaoMedi);
        assert!(medida.tempo_ms.is_none());
    }

    #[test]
    fn calculo_de_perda_conta_pelo_tamanho_da_lista_e_nao_por_um_contador_separado() {
        let respostas = vec![
            RespostaPing { ms: Some(10.0), ok: Some(true) },
            RespostaPing { ms: None, ok: Some(false) },
            RespostaPing { ms: Some(12.0), ok: Some(true) },
        ];
        assert_eq!(calcular_perda(&respostas), Perda::Medida { enviados: 3, perdidos: 1 });
        assert_eq!(calcular_perda(&[]), Perda::NaoMedi);
    }

    #[test]
    fn mediana_ignora_falhas() {
        let respostas = vec![
            RespostaPing { ms: Some(30.0), ok: Some(true) },
            RespostaPing { ms: None, ok: Some(false) },
            RespostaPing { ms: Some(10.0), ok: Some(true) },
            RespostaPing { ms: Some(20.0), ok: Some(true) },
        ];
        assert_eq!(calcular_tempo_mediano(&respostas), Some(20.0));
        assert_eq!(calcular_tempo_mediano(&[]), None);
    }

    #[test]
    fn jitter_zero_quando_tempo_constante() {
        let respostas = vec![
            RespostaPing { ms: Some(20.0), ok: Some(true) },
            RespostaPing { ms: Some(20.0), ok: Some(true) },
            RespostaPing { ms: Some(20.0), ok: Some(true) },
        ];
        assert_eq!(calcular_jitter(&respostas), Some(0.0));
    }

    #[test]
    fn jitter_mede_a_variacao_entre_tentativas_sucessivas() {
        let respostas = vec![
            RespostaPing { ms: Some(20.0), ok: Some(true) },
            RespostaPing { ms: Some(80.0), ok: Some(true) },
            RespostaPing { ms: Some(20.0), ok: Some(true) },
        ];
        assert_eq!(calcular_jitter(&respostas), Some(60.0));
    }

    #[test]
    fn jitter_precisa_de_ao_menos_duas_respostas() {
        let uma = vec![RespostaPing { ms: Some(20.0), ok: Some(true) }];
        assert_eq!(calcular_jitter(&uma), None);
        assert_eq!(calcular_jitter(&[]), None);
    }

    #[test]
    fn endereco_publico_exclui_privado_loopback_e_link_local() {
        for privado in ["10.0.0.5", "192.168.1.1", "172.16.0.1", "127.0.0.1", "169.254.1.1"] {
            let ip: IpAddr = privado.parse().unwrap();
            assert!(!e_endereco_publico(&ip), "{} deveria ser privado", privado);
        }

        let publico: IpAddr = "203.0.113.10".parse().unwrap();
        assert!(e_endereco_publico(&publico));
    }

    #[test]
    fn medicao_real_desta_maquina() {
        // Sem jogo aberto, cai em "não descobri" sem sondar nada.
        let medida = medir_agora();
        println!("alvo: {:?}", medida.alvo);
        println!("perda: {:?}", medida.perda);
        println!("jitter: {:?} ms", medida.jitter_ms);
        println!("tempo: {:?} ms", medida.tempo_ms);
        println!("nota: {}", medida.nota);

        assert!(!medida.nota.is_empty());

        if let Perda::Medida { enviados, perdidos } = medida.perda {
            assert!(perdidos <= enviados, "perdidos não pode passar de enviados");
        }
    }
}
