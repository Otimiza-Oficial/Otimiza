// OTIMIZA REDE: ONDE ESTÁ O PROBLEMA. Rota salto a salto até o servidor, no estilo do MTR: eco ICMP com TTL crescente
// (`IcmpSendEcho2`/`Icmp6SendEcho2`, campos e não o texto traduzido do `tracert`), dez rodadas de um segundo, uma thread por
// salto, cada salto numa fase própria do segundo (rajada perde eco: ver `medir`). Cerca de 10 s no total.
//
// Saindo por VPN (placa virtual), os endereços privados do começo são o túnel, não a casa, e o primeiro endereço
// público é a saída da VPN, não a operadora da pessoa: esses saltos ganham o trecho `Tunel` e o resto não é chamado
// de operadora.
//
// Cada salto ganha um trecho: a sua casa (endereços privados do começo da rota), a sua operadora (CGNAT, e o mesmo
// sistema autônomo do primeiro endereço público, pelo `operadora`), o caminho (outro dono) e o servidor. A regra do MTR
// para não culpar quem não tem culpa: roteador do meio que "perde" e o seguinte não é roteador poupando resposta de
// ping, não perda. O problema começa no primeiro salto a partir do qual a perda (ou a variação) segue até o fim.
//
// Não diminui o ping: diz com número em que trecho ele nasce. Regra do `rede.rs` mantida: sem servidor conhecido não
// mede contra um endereço qualquer.

use super::operadora;
use serde::Serialize;
use std::collections::BTreeMap;
use std::net::IpAddr;

/// Rodadas de um segundo: cada salto recebe esse tanto de pacotes.
pub const RODADAS: u32 = 10;
pub const MAX_SALTOS: u8 = 30;
const PRAZO_MS: u32 = 1000;

/// Perda no fim da rota a partir da qual há o que procurar (1 de 10 já conta: no jogo, um pacote perdido é um tranco).
const PERDA_MINIMA: f64 = 10.0;
/// Jitter no fim da rota a partir do qual se sente no jogo.
const JITTER_MINIMO_MS: f64 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub enum TipoResposta {
    /// Um roteador do meio avisou que o pacote expirou nele.
    Salto,
    /// O próprio destino respondeu.
    Destino,
    /// Um roteador respondeu "destino inalcançável".
    Inalcancavel,
    /// Nada voltou no prazo.
    Silencio,
}

/// Uma sonda: um pacote com um TTL numa rodada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resposta {
    pub ttl: u8,
    pub endereco: Option<IpAddr>,
    pub ms: Option<u32>,
    pub tipo: TipoResposta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "TrechoDaRota"))]
#[serde(rename_all = "snake_case")]
pub enum Trecho {
    Casa,
    /// Dentro do túnel da VPN: a VPN ou a conexão por baixo dela, sem como separar daqui.
    Tunel,
    Operadora,
    /// Fora da casa, sem dono conhecido (sem a consulta de operadora): operadora ou caminho.
    Internet,
    Caminho,
    Servidor,
    /// O roteador nunca respondeu.
    SemResposta,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "SaltoDaRota"))]
pub struct Salto {
    pub numero: u8,
    pub endereco: Option<IpAddr>,
    pub enviados: u32,
    pub respondidos: u32,
    pub perda_pct: f64,
    pub melhor_ms: Option<u32>,
    pub mediana_ms: Option<u32>,
    pub pior_ms: Option<u32>,
    /// Média da diferença entre respostas seguidas.
    pub jitter_ms: Option<f64>,
    pub trecho: Trecho,
    pub asn: Option<u32>,
    pub operadora: Option<String>,
}

/// **Pura.** Junta as sondas por salto. A rota termina no primeiro salto em que o destino respondeu; sem destino,
/// no último que respondeu alguma vez (o resto é silêncio de quem não responde ping).
pub fn agregar(respostas: &[Resposta]) -> Vec<Salto> {
    let fim_destino = respostas
        .iter()
        .filter(|r| r.tipo == TipoResposta::Destino)
        .map(|r| r.ttl)
        .min();
    let mut por_ttl: BTreeMap<u8, Vec<&Resposta>> = BTreeMap::new();
    for r in respostas {
        if fim_destino.is_none_or(|f| r.ttl <= f) {
            por_ttl.entry(r.ttl).or_default().push(r);
        }
    }
    let mut saltos: Vec<Salto> = por_ttl
        .into_iter()
        .map(|(ttl, rs)| {
            let enviados = rs.len() as u32;
            let tempos: Vec<u32> = rs.iter().filter_map(|r| r.ms).collect();
            let respondidos = tempos.len() as u32;
            let mut ordenados = tempos.clone();
            ordenados.sort_unstable();
            let jitter = (tempos.len() >= 2).then(|| {
                let soma: u64 = tempos.windows(2).map(|w| w[0].abs_diff(w[1]) as u64).sum();
                soma as f64 / (tempos.len() - 1) as f64
            });
            Salto {
                numero: ttl,
                endereco: mais_frequente(rs.iter().filter_map(|r| r.endereco)),
                enviados,
                respondidos,
                perda_pct: if enviados == 0 {
                    0.0
                } else {
                    100.0 * (enviados - respondidos) as f64 / enviados as f64
                },
                melhor_ms: ordenados.first().copied(),
                mediana_ms: ordenados.get(ordenados.len() / 2).copied(),
                pior_ms: ordenados.last().copied(),
                jitter_ms: jitter,
                trecho: Trecho::SemResposta,
                asn: None,
                operadora: None,
            }
        })
        .collect();
    if fim_destino.is_none() {
        while saltos.last().is_some_and(|s| s.respondidos == 0) {
            saltos.pop();
        }
    }
    saltos
}

fn mais_frequente(enderecos: impl Iterator<Item = IpAddr>) -> Option<IpAddr> {
    let mut contagem: Vec<(IpAddr, u32)> = Vec::new();
    for e in enderecos {
        match contagem.iter_mut().find(|(x, _)| *x == e) {
            Some((_, n)) => *n += 1,
            None => contagem.push((e, 1)),
        }
    }
    // Empate: o primeiro visto.
    contagem
        .into_iter()
        .fold(None, |melhor: Option<(IpAddr, u32)>, (e, n)| match melhor {
            Some((_, m)) if m >= n => melhor,
            _ => Some((e, n)),
        })
        .map(|(e, _)| e)
}

/// **Pura.** Trecho de cada salto. Casa: privados enquanto a rota não saiu de casa. Operadora: CGNAT, privados depois
/// de sair de casa (rede interna da operadora) e o mesmo dono (ASN) do primeiro endereço público. Caminho: outro dono.
/// Sem o dono do primeiro público, o que vem depois dele fica `Internet` (não dá para separar operadora de caminho).
pub fn classificar(saltos: &mut [Salto], destino: IpAddr) {
    classificar_via(saltos, destino, false)
}

/// **Pura.** Como `classificar`; `via_tunel` quando a rota sai por placa virtual (VPN): os privados do começo são o
/// túnel, e nenhum salto público é chamado de operadora da pessoa (ela fica escondida dentro do túnel).
pub fn classificar_via(saltos: &mut [Salto], destino: IpAddr, via_tunel: bool) {
    let mut em_casa = true;
    // Na VPN, a "operadora" que aparece é a da saída da VPN: tratada como já fora dela.
    let mut viu_publico = via_tunel;
    let mut asn_da_operadora: Option<u32> = None;
    let mut saiu_da_operadora = false;
    for s in saltos.iter_mut() {
        let Some(ip) = s.endereco else {
            s.trecho = Trecho::SemResposta;
            continue;
        };
        s.trecho = if ip == destino {
            Trecho::Servidor
        } else if em_casa && operadora::e_privado(&ip) {
            if via_tunel {
                Trecho::Tunel
            } else {
                Trecho::Casa
            }
        } else if operadora::e_cgnat(&ip) || operadora::e_privado(&ip) {
            em_casa = false;
            if saiu_da_operadora {
                Trecho::Caminho
            } else if via_tunel {
                Trecho::Internet
            } else {
                Trecho::Operadora
            }
        } else if !viu_publico {
            em_casa = false;
            viu_publico = true;
            asn_da_operadora = s.asn;
            Trecho::Operadora
        } else {
            match (asn_da_operadora, s.asn) {
                (Some(op), Some(a)) if op == a && !saiu_da_operadora => Trecho::Operadora,
                (Some(_), Some(_)) => {
                    saiu_da_operadora = true;
                    Trecho::Caminho
                }
                (Some(_), None) if saiu_da_operadora => Trecho::Caminho,
                _ => Trecho::Internet,
            }
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "ProblemaDaRota"))]
#[serde(rename_all = "snake_case")]
pub enum Problema {
    Perda,
    Variacao,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "VereditoDaRota"))]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Veredito {
    /// Chegou ao servidor sem perda nem variação que se sinta.
    Limpa,
    Problema {
        problema: Problema,
        trecho: Trecho,
        salto: u8,
        /// Perda (%) ou jitter (ms) no fim da rota.
        tamanho: f64,
        operadora: Option<String>,
    },
    /// O servidor não responde ping e, até o último salto que respondeu, nada aparece.
    ServidorNaoResponde { ultimo_salto: u8 },
    /// Nenhum salto respondeu.
    NaoMediu,
}

/// **Pura.** Índice do primeiro salto a partir do qual `valor` segue acima de `limiar` até o fim.
fn inicio_persistente(
    validos: &[&Salto],
    valor: impl Fn(&Salto) -> Option<f64>,
    limiar: f64,
) -> Option<usize> {
    let mut inicio = None;
    for (i, s) in validos.iter().enumerate().rev() {
        match valor(s) {
            Some(v) if v >= limiar => inicio = Some(i),
            Some(_) => break,
            // Sem número (uma resposta só): não interrompe nem conta.
            None => {}
        }
    }
    inicio
}

/// **Pura.** O veredito pela regra do MTR: o problema é o que persiste até o fim da rota.
pub fn veredito(saltos: &[Salto], chegou: bool) -> Veredito {
    let validos: Vec<&Salto> = saltos.iter().filter(|s| s.respondidos > 0).collect();
    let Some(fim) = validos.last() else {
        return Veredito::NaoMediu;
    };
    let culpado = |i: usize, problema: Problema, tamanho: f64| -> Option<Veredito> {
        // Sem chegar ao servidor, um único salto no fim não basta: pode ser só o último roteador poupando resposta.
        if !chegou && i + 1 == validos.len() {
            return None;
        }
        let s = validos[i];
        Some(Veredito::Problema {
            problema,
            trecho: s.trecho,
            salto: s.numero,
            tamanho,
            operadora: s.operadora.clone(),
        })
    };
    if fim.perda_pct >= PERDA_MINIMA {
        if let Some(v) = inicio_persistente(&validos, |s| Some(s.perda_pct), fim.perda_pct / 2.0)
            .and_then(|i| culpado(i, Problema::Perda, fim.perda_pct))
        {
            return v;
        }
    }
    if let Some(j) = fim.jitter_ms.filter(|j| *j >= JITTER_MINIMO_MS) {
        if let Some(v) = inicio_persistente(&validos, |s| s.jitter_ms, j / 2.0)
            .and_then(|i| culpado(i, Problema::Variacao, j))
        {
            return v;
        }
    }
    if chegou {
        Veredito::Limpa
    } else {
        Veredito::ServidorNaoResponde {
            ultimo_salto: fim.numero,
        }
    }
}

fn onde(trecho: Trecho, salto: u8, operadora: Option<&str>) -> String {
    match trecho {
        Trecho::Casa => format!(
            "na sua casa (salto {}): roteador ou Wi-Fi. Teste no cabo ou perto do roteador; reiniciar o roteador também vale",
            salto
        ),
        Trecho::Tunel => format!(
            "dentro do túnel da VPN (salto {}): pode ser a VPN ou a sua conexão por baixo dela. Desligue a VPN e meça de novo para separar",
            salto
        ),
        Trecho::Operadora => format!(
            "na sua operadora{} (salto {}): não é o seu PC nem a sua casa. Vale reclamar com ela mostrando este resultado",
            operadora.map(|o| format!(" ({})", o)).unwrap_or_default(),
            salto
        ),
        Trecho::Internet => format!(
            "fora da sua casa, no salto {} (na operadora ou depois dela): não é o seu PC nem a sua casa",
            salto
        ),
        Trecho::Caminho => format!(
            "no caminho entre a sua operadora e o servidor{} (salto {}): fora da sua casa e da sua operadora; costuma passar sozinho",
            operadora.map(|o| format!(", na rede {}", o)).unwrap_or_default(),
            salto
        ),
        Trecho::Servidor => "no próprio servidor: a rota até ele está limpa".to_string(),
        Trecho::SemResposta => format!("a partir do salto {}", salto),
    }
}

/// **Pura.** A frase do veredito.
pub fn frase(v: &Veredito) -> String {
    match v {
        Veredito::Limpa => {
            "Rota limpa até o servidor: nenhum trecho perde pacote nem varia a ponto de se sentir no jogo.".to_string()
        }
        Veredito::Problema {
            problema: Problema::Perda,
            trecho,
            salto,
            tamanho,
            operadora,
        } => format!(
            "{:.0}% dos pacotes se perdem, e a perda começa {}.",
            tamanho,
            onde(*trecho, *salto, operadora.as_deref())
        ),
        Veredito::Problema {
            problema: Problema::Variacao,
            trecho,
            salto,
            tamanho,
            operadora,
        } => format!(
            "O ping varia {:.0} ms de um pacote para o outro, e a variação começa {}.",
            tamanho,
            onde(*trecho, *salto, operadora.as_deref())
        ),
        Veredito::ServidorNaoResponde { ultimo_salto } => format!(
            "O servidor não responde ping (comum em servidor de jogo com proteção contra ataque). Até o último salto que respondeu ({}), a rota está limpa.",
            ultimo_salto
        ),
        Veredito::NaoMediu => {
            "Nenhum salto respondeu: sem rede agora, ou o ping está bloqueado no seu roteador ou firewall.".to_string()
        }
    }
}

fn lista_de_saltos(n: &[String]) -> String {
    match n {
        [um] => format!("O salto {} mostra", um),
        [inicio @ .., ultimo] => format!("Os saltos {} e {} mostram", inicio.join(", "), ultimo),
        [] => String::new(),
    }
}

/// **Pura.** Roteador do meio com perda que não segue até o fim: explica, para ninguém se assustar com ele.
pub fn observacao(saltos: &[Salto], v: &Veredito) -> Option<String> {
    let culpado = match v {
        Veredito::Problema {
            problema: Problema::Perda,
            salto,
            ..
        } => *salto,
        _ => u8::MAX,
    };
    let fim = saltos.iter().rfind(|s| s.respondidos > 0)?.numero;
    let enganosos: Vec<String> = saltos
        .iter()
        .filter(|s| {
            s.respondidos > 0 && s.numero < fim && s.numero < culpado && s.perda_pct >= PERDA_MINIMA
        })
        .map(|s| s.numero.to_string())
        .collect();
    (!enganosos.is_empty()).then(|| {
        format!(
            "{} perda que não continua nos seguintes: é o roteador poupando resposta de ping, não perda.",
            lista_de_saltos(&enganosos)
        )
    })
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Rota {
    pub destino: IpAddr,
    pub chegou: bool,
    pub saltos: Vec<Salto>,
    pub veredito: Veredito,
    pub frase: String,
    pub observacao: Option<String>,
    /// Presente quando algum nome de operadora veio do IPinfo (a licença pede a atribuição).
    pub atribuicao: Option<&'static str>,
}

/// **Pura.** Monta o resultado depois das sondas e da consulta de operadora.
pub fn montar(
    destino: IpAddr,
    respostas: &[Resposta],
    donos: &[(IpAddr, operadora::Operadora)],
) -> Rota {
    montar_via(destino, respostas, donos, false)
}

/// **Pura.** Como `montar`, sabendo se a rota sai por VPN.
pub fn montar_via(
    destino: IpAddr,
    respostas: &[Resposta],
    donos: &[(IpAddr, operadora::Operadora)],
    via_tunel: bool,
) -> Rota {
    let mut saltos = agregar(respostas);
    for s in saltos.iter_mut() {
        if let Some((_, o)) = s
            .endereco
            .and_then(|e| donos.iter().find(|(ip, _)| *ip == e))
        {
            s.asn = o.asn;
            s.operadora = Some(o.nome.clone());
        }
    }
    classificar_via(&mut saltos, destino, via_tunel);
    let chegou = respostas.iter().any(|r| r.tipo == TipoResposta::Destino);
    let v = veredito(&saltos, chegou);
    Rota {
        destino,
        chegou,
        frase: frase(&v),
        observacao: observacao(&saltos, &v),
        atribuicao: saltos
            .iter()
            .any(|s| s.operadora.is_some())
            .then_some(operadora::ATRIBUICAO),
        saltos,
        veredito: v,
    }
}

#[cfg(windows)]
pub use nativo::{diagnosticar, diagnosticar_servidor_do_jogo, sondar};

#[cfg(windows)]
mod nativo {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        Icmp6CreateFile, Icmp6SendEcho2, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho2,
        ICMPV6_ECHO_REPLY_LH, ICMP_ECHO_REPLY, IP_DEST_HOST_UNREACHABLE, IP_DEST_NET_UNREACHABLE,
        IP_DEST_PORT_UNREACHABLE, IP_OPTION_INFORMATION, IP_SUCCESS, IP_TTL_EXPIRED_TRANSIT,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET6, SOCKADDR_IN6};

    const DADOS: [u8; 32] = [0x4F; 32];

    fn tipo(status: u32) -> TipoResposta {
        match status {
            IP_SUCCESS => TipoResposta::Destino,
            // Mesmo código para o "hop limit exceeded" do IPv6.
            IP_TTL_EXPIRED_TRANSIT => TipoResposta::Salto,
            IP_DEST_NET_UNREACHABLE | IP_DEST_HOST_UNREACHABLE | IP_DEST_PORT_UNREACHABLE => {
                TipoResposta::Inalcancavel
            }
            _ => TipoResposta::Silencio,
        }
    }

    struct Icmp(HANDLE);
    impl Drop for Icmp {
        fn drop(&mut self) {
            // SAFETY: handle válido de IcmpCreateFile/Icmp6CreateFile.
            unsafe {
                IcmpCloseHandle(self.0);
            }
        }
    }

    fn abrir(h: HANDLE) -> Option<Icmp> {
        (!h.is_null() && h != INVALID_HANDLE_VALUE).then_some(Icmp(h))
    }

    fn silencio(ttl: u8) -> Resposta {
        Resposta {
            ttl,
            endereco: None,
            ms: None,
            tipo: TipoResposta::Silencio,
        }
    }

    /// Um pacote com este TTL. Chamada síncrona: volta em até `prazo_ms`.
    pub fn sondar(destino: IpAddr, ttl: u8, prazo_ms: u32) -> Resposta {
        // u64: alinhamento da estrutura de resposta. Folga para a resposta, os dados e a mensagem de erro do ICMP.
        let mut buf = [0u64; 128];
        let tamanho = std::mem::size_of_val(&buf) as u32;
        // SAFETY: as estruturas de entrada estão zeradas e preenchidas; a resposta cabe no buffer informado.
        unsafe {
            let mut opcoes: IP_OPTION_INFORMATION = std::mem::zeroed();
            opcoes.Ttl = ttl;
            let (status, endereco, ms) = match destino {
                IpAddr::V4(v4) => {
                    let Some(h) = abrir(IcmpCreateFile()) else {
                        return silencio(ttl);
                    };
                    IcmpSendEcho2(
                        h.0,
                        std::ptr::null_mut(),
                        None,
                        std::ptr::null(),
                        u32::from_ne_bytes(v4.octets()),
                        DADOS.as_ptr() as *const _,
                        DADOS.len() as u16,
                        &opcoes,
                        buf.as_mut_ptr() as *mut _,
                        tamanho,
                        prazo_ms,
                    );
                    // O retorno pode ser zero mesmo com a resposta de "TTL expirou" no buffer: decide o buffer.
                    let r = &*(buf.as_ptr() as *const ICMP_ECHO_REPLY);
                    if r.Address == 0 {
                        return silencio(ttl);
                    }
                    (
                        r.Status,
                        IpAddr::V4(Ipv4Addr::from(r.Address.to_ne_bytes())),
                        r.RoundTripTime,
                    )
                }
                IpAddr::V6(v6) => {
                    let Some(h) = abrir(Icmp6CreateFile()) else {
                        return silencio(ttl);
                    };
                    let mut origem: SOCKADDR_IN6 = std::mem::zeroed();
                    origem.sin6_family = AF_INET6;
                    let mut alvo: SOCKADDR_IN6 = std::mem::zeroed();
                    alvo.sin6_family = AF_INET6;
                    alvo.sin6_addr.u.Byte = v6.octets();
                    Icmp6SendEcho2(
                        h.0,
                        std::ptr::null_mut(),
                        None,
                        std::ptr::null(),
                        &origem,
                        &alvo,
                        DADOS.as_ptr() as *const _,
                        DADOS.len() as u16,
                        &opcoes,
                        buf.as_mut_ptr() as *mut _,
                        tamanho,
                        prazo_ms,
                    );
                    let r = buf.as_ptr() as *const ICMPV6_ECHO_REPLY_LH;
                    // Estrutura empacotada: leitura sem supor alinhamento.
                    let palavras: [u16; 8] =
                        std::ptr::addr_of!((*r).Address.sin6_addr).read_unaligned();
                    let mut bytes = [0u8; 16];
                    for (i, w) in palavras.iter().enumerate() {
                        bytes[2 * i..2 * i + 2].copy_from_slice(&w.to_ne_bytes());
                    }
                    if bytes == [0; 16] {
                        return silencio(ttl);
                    }
                    let status = std::ptr::addr_of!((*r).Status).read_unaligned();
                    let ms = std::ptr::addr_of!((*r).RoundTripTime).read_unaligned();
                    (status, IpAddr::V6(Ipv6Addr::from(bytes)), ms)
                }
            };
            let t = tipo(status);
            if t == TipoResposta::Silencio {
                return silencio(ttl);
            }
            Resposta {
                ttl,
                endereco: Some(endereco),
                ms: Some(ms),
                tipo: t,
            }
        }
    }

    /// As sondas: uma thread por salto, cada uma com `rodadas` pacotes, um por segundo, numa fase própria dentro do
    /// segundo (salto k sai em (k-1)/L do segundo, L = saltos até o destino). Achado o destino, os saltos além dele
    /// param e a fase dos outros se espalha pelos que sobraram.
    ///
    /// Por que espalhar: medido no PC do dono (2026-11, saída por VPN WireGuard, destino 1.1.1.1), um a um os pacotes
    /// voltavam todos (~145 ms, 0 perda), mas a rajada com todos os saltos no mesmo instante perdia de 5 em 8 até 7 em 8
    /// ecos do destino e criava jitter falso de ~120 ms (rajada de seis com 20 ms entre eles: ainda 7 em 8 perdidos).
    /// Dois pacotes simultâneos voltavam sempre: o que pesa é a rajada, provavelmente um limite de taxa de ICMP no
    /// caminho. Espalhados pelo segundo, ficam no ritmo da varredura sequencial, que não perdia.
    pub fn medir(destino: IpAddr, rodadas: u32, max_saltos: u8) -> Vec<Resposta> {
        use std::sync::atomic::{AtomicU8, Ordering::Relaxed};
        let limite = AtomicU8::new(max_saltos.max(1));
        let inicio = Instant::now();
        std::thread::scope(|s| {
            let fios: Vec<_> = (1..=max_saltos)
                .map(|ttl| {
                    let limite = &limite;
                    s.spawn(move || {
                        let mut minhas = Vec::with_capacity(rodadas as usize);
                        for rodada in 0..rodadas {
                            let l = limite.load(Relaxed);
                            if ttl > l {
                                break;
                            }
                            let fase = (ttl as u64 - 1) * 1000 / l as u64;
                            let quando = Duration::from_millis(rodada as u64 * 1000 + fase);
                            if let Some(espera) = quando.checked_sub(inicio.elapsed()) {
                                std::thread::sleep(espera);
                            }
                            if ttl > limite.load(Relaxed) {
                                break;
                            }
                            let r = sondar(destino, ttl, PRAZO_MS);
                            if r.tipo == TipoResposta::Destino {
                                limite.fetch_min(ttl, Relaxed);
                            }
                            minhas.push(r);
                        }
                        minhas
                    })
                })
                .collect();
            fios.into_iter()
                .filter_map(|f| f.join().ok())
                .flatten()
                .collect()
        })
    }

    /// Rota até `destino` com o dono de cada trecho. ~10 s.
    pub async fn diagnosticar(destino: IpAddr) -> Rota {
        let respostas = tokio::task::spawn_blocking(move || medir(destino, RODADAS, MAX_SALTOS))
            .await
            .unwrap_or_default();
        let mut publicos: Vec<IpAddr> = respostas
            .iter()
            .filter_map(|r| r.endereco)
            .filter(operadora::e_publico)
            .collect();
        publicos.sort();
        publicos.dedup();
        // Todas ao mesmo tempo: são no máximo uns 20 endereços, e cada consulta tem prazo próprio.
        let consultas: Vec<_> = publicos
            .into_iter()
            .map(|ip| tokio::spawn(async move { (ip, operadora::consultar(ip).await) }))
            .collect();
        let mut donos = Vec::new();
        for c in consultas {
            if let Ok((ip, operadora::Consulta::Encontrada { operadora, .. })) = c.await {
                donos.push((ip, operadora));
            }
        }
        let via_tunel = tokio::task::spawn_blocking(move || {
            crate::plataforma::wifi::placa_da_rota(destino)
                .is_some_and(|p| p.tipo == crate::plataforma::wifi::TipoDePlaca::Virtual)
        })
        .await
        .unwrap_or(false);
        montar_via(destino, &respostas, &donos, via_tunel)
    }

    /// Rota até o servidor do jogo aberto. Sem servidor descoberto, não mede (nunca contra um endereço qualquer).
    pub async fn diagnosticar_servidor_do_jogo() -> Result<Rota, String> {
        let servidor = tokio::task::spawn_blocking(crate::modules::windows::rede::servidor_do_jogo)
            .await
            .ok()
            .flatten()
            .ok_or("Não descobri o servidor do jogo: abra o jogo e entre numa partida antes de medir a rota.")?;
        let ip: IpAddr = servidor.parse().map_err(|_| {
            format!(
                "O servidor do jogo veio num formato que não é endereço: {}",
                servidor
            )
        })?;
        Ok(diagnosticar(ip).await)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn resp(ttl: u8, end: &str, ms: Option<u32>, tipo: TipoResposta) -> Resposta {
        Resposta {
            ttl,
            endereco: ms.map(|_| ip(end)),
            ms,
            tipo: if ms.is_none() {
                TipoResposta::Silencio
            } else {
                tipo
            },
        }
    }

    /// Uma rota de casa até o servidor; `perda(ttl, rodada)` e `tempo(ttl, rodada)` desenham o cenário.
    fn cenario(
        enderecos: &[&str],
        perda: impl Fn(u8, u32) -> bool,
        tempo: impl Fn(u8, u32) -> u32,
    ) -> Vec<Resposta> {
        let n = enderecos.len() as u8;
        let mut v = Vec::new();
        for rodada in 0..RODADAS {
            for ttl in 1..=n {
                let tipo = if ttl == n {
                    TipoResposta::Destino
                } else {
                    TipoResposta::Salto
                };
                let ms = (!perda(ttl, rodada)).then(|| tempo(ttl, rodada));
                v.push(resp(ttl, enderecos[ttl as usize - 1], ms, tipo));
            }
        }
        v
    }

    const ROTA: [&str; 6] = [
        "192.168.0.1",  // 1 casa
        "100.64.10.1",  // 2 CGNAT da operadora
        "177.79.1.1",   // 3 primeiro público (Vivo)
        "177.79.2.1",   // 4 Vivo
        "4.69.1.1",     // 5 trânsito
        "131.196.1.10", // 6 servidor
    ];

    fn donos() -> Vec<(IpAddr, operadora::Operadora)> {
        let o = |asn, nome: &str| operadora::Operadora {
            asn: Some(asn),
            nome_registrado: nome.into(),
            nome: operadora::nome_popular(Some(asn), nome),
            dominio: None,
            pais: Some("BR".into()),
        };
        vec![
            (ip("177.79.1.1"), o(26599, "TELEFONICA BRASIL S.A")),
            (ip("177.79.2.1"), o(26599, "TELEFONICA BRASIL S.A")),
            (ip("4.69.1.1"), o(3356, "Level 3 Parent, LLC")),
            (ip("131.196.1.10"), o(64500, "Hospedagem do Servidor LTDA")),
        ]
    }

    fn destino() -> IpAddr {
        ip(ROTA[5])
    }

    #[test]
    fn agrega_por_salto_com_perda_mediana_e_jitter() {
        let r = cenario(
            &ROTA,
            |ttl, rod| ttl == 2 && rod < 3,
            |ttl, rod| 10 * ttl as u32 + (rod % 2) * 4,
        );
        let s = agregar(&r);
        assert_eq!(s.len(), 6);
        assert_eq!(s[1].enviados, 10);
        assert_eq!(s[1].respondidos, 7);
        assert!((s[1].perda_pct - 30.0).abs() < 1e-9);
        assert_eq!(s[0].melhor_ms, Some(10));
        assert_eq!(s[0].pior_ms, Some(14));
        assert_eq!(s[0].jitter_ms, Some(4.0));
        assert_eq!(s[5].endereco, Some(destino()));
    }

    #[test]
    fn a_rota_termina_no_primeiro_salto_em_que_o_destino_respondeu() {
        let mut r = cenario(&ROTA, |_, _| false, |_, _| 20);
        // Rodada que sondou além: o destino responde de novo no TTL 7 e 8.
        r.push(resp(7, ROTA[5], Some(20), TipoResposta::Destino));
        r.push(resp(8, ROTA[5], Some(20), TipoResposta::Destino));
        assert_eq!(agregar(&r).len(), 6);
    }

    #[test]
    fn sem_destino_o_silencio_do_fim_e_cortado() {
        let mut r: Vec<Resposta> = cenario(&ROTA[..4], |_, _| false, |_, _| 20)
            .into_iter()
            .map(|mut x| {
                x.tipo = TipoResposta::Salto;
                x
            })
            .collect();
        for ttl in 5..=30 {
            r.push(resp(ttl, "", None, TipoResposta::Silencio));
        }
        let s = agregar(&r);
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn trechos_casa_operadora_caminho_servidor() {
        let rota = montar(
            destino(),
            &cenario(&ROTA, |_, _| false, |_, _| 20),
            &donos(),
        );
        let t: Vec<Trecho> = rota.saltos.iter().map(|s| s.trecho).collect();
        assert_eq!(
            t,
            vec![
                Trecho::Casa,
                Trecho::Operadora,
                Trecho::Operadora,
                Trecho::Operadora,
                Trecho::Caminho,
                Trecho::Servidor
            ]
        );
        assert_eq!(rota.saltos[2].operadora.as_deref(), Some("Vivo"));
        assert_eq!(rota.atribuicao, Some(operadora::ATRIBUICAO));
    }

    #[test]
    fn sem_dono_conhecido_o_que_vem_depois_do_primeiro_publico_e_internet() {
        let rota = montar(destino(), &cenario(&ROTA, |_, _| false, |_, _| 20), &[]);
        let t: Vec<Trecho> = rota.saltos.iter().map(|s| s.trecho).collect();
        assert_eq!(
            t[2],
            Trecho::Operadora,
            "o primeiro público é a borda da operadora"
        );
        assert_eq!(t[3], Trecho::Internet);
        assert_eq!(t[4], Trecho::Internet);
        assert_eq!(t[5], Trecho::Servidor);
        assert_eq!(rota.atribuicao, None, "sem nome do IPinfo, sem atribuição");
    }

    #[test]
    fn rota_limpa() {
        let rota = montar(
            destino(),
            &cenario(&ROTA, |_, _| false, |t, _| 5 * t as u32),
            &donos(),
        );
        assert_eq!(rota.veredito, Veredito::Limpa);
        assert!(rota.frase.starts_with("Rota limpa"));
        assert!(rota.observacao.is_none());
    }

    #[test]
    fn roteador_do_meio_que_poupa_resposta_nao_e_culpado() {
        // O salto 4 "perde" 60%, mas o 5 e o servidor não perdem nada.
        let r = cenario(&ROTA, |ttl, rod| ttl == 4 && rod < 6, |_, _| 20);
        let rota = montar(destino(), &r, &donos());
        assert_eq!(rota.veredito, Veredito::Limpa);
        assert!(rota
            .observacao
            .unwrap()
            .starts_with("O salto 4 mostra perda"));
    }

    #[test]
    fn perda_que_nasce_na_operadora_e_segue_ate_o_fim() {
        let r = cenario(&ROTA, |ttl, rod| ttl >= 3 && rod % 5 == 0, |_, _| 20);
        let rota = montar(destino(), &r, &donos());
        match &rota.veredito {
            Veredito::Problema {
                problema,
                trecho,
                salto,
                operadora,
                ..
            } => {
                assert_eq!(*problema, Problema::Perda);
                assert_eq!(*trecho, Trecho::Operadora);
                assert_eq!(*salto, 3);
                assert_eq!(operadora.as_deref(), Some("Vivo"));
            }
            v => panic!("{:?}", v),
        }
        assert!(
            rota.frase.contains("na sua operadora (Vivo) (salto 3)"),
            "{}",
            rota.frase
        );
        assert!(rota.frase.starts_with("20% dos pacotes"));
    }

    #[test]
    fn variacao_que_nasce_no_wifi_de_casa() {
        // Wi-Fi ruim: o primeiro salto já varia 0/60 ms, e tudo depois carrega isso.
        let r = cenario(
            &ROTA,
            |_, _| false,
            |ttl, rod| 5 * ttl as u32 + if rod % 2 == 0 { 0 } else { 60 },
        );
        let rota = montar(destino(), &r, &donos());
        match &rota.veredito {
            Veredito::Problema {
                problema,
                trecho,
                salto,
                ..
            } => {
                assert_eq!(*problema, Problema::Variacao);
                assert_eq!(*trecho, Trecho::Casa);
                assert_eq!(*salto, 1);
            }
            v => panic!("{:?}", v),
        }
        assert!(rota.frase.contains("na sua casa (salto 1)"));
    }

    #[test]
    fn perda_so_no_servidor() {
        let r = cenario(&ROTA, |ttl, rod| ttl == 6 && rod < 3, |_, _| 20);
        let rota = montar(destino(), &r, &donos());
        assert!(matches!(
            rota.veredito,
            Veredito::Problema {
                trecho: Trecho::Servidor,
                salto: 6,
                ..
            }
        ));
        assert!(rota.frase.contains("no próprio servidor"));
    }

    #[test]
    fn servidor_que_nao_responde_ping_nao_vira_perda_total() {
        // O servidor nunca responde: a rota vai até o salto 5 e para.
        let r: Vec<Resposta> = cenario(&ROTA, |ttl, _| ttl == 6, |_, _| 20);
        let rota = montar(destino(), &r, &donos());
        assert!(!rota.chegou);
        assert_eq!(rota.saltos.len(), 5);
        assert_eq!(
            rota.veredito,
            Veredito::ServidorNaoResponde { ultimo_salto: 5 }
        );
        assert!(rota.frase.contains("não responde ping"));
    }

    #[test]
    fn sem_chegar_um_unico_salto_no_fim_com_perda_nao_e_culpado() {
        let r: Vec<Resposta> = cenario(
            &ROTA,
            |ttl, rod| ttl == 6 || (ttl == 5 && rod < 5),
            |_, _| 20,
        );
        let rota = montar(destino(), &r, &donos());
        assert_eq!(
            rota.veredito,
            Veredito::ServidorNaoResponde { ultimo_salto: 5 }
        );
    }

    #[test]
    fn nada_respondeu_e_nao_mediu() {
        let r: Vec<Resposta> = (1..=30)
            .map(|t| resp(t, "", None, TipoResposta::Silencio))
            .collect();
        let rota = montar(destino(), &r, &[]);
        assert_eq!(rota.veredito, Veredito::NaoMediu);
        assert!(rota.saltos.is_empty());
    }

    #[test]
    fn salto_sem_resposta_no_meio_nao_interrompe_a_analise() {
        let mut r = cenario(&ROTA, |ttl, rod| ttl >= 3 && rod % 5 == 0, |_, _| 20);
        // O salto 4 nunca responde (roteador que não fala ICMP).
        for x in r.iter_mut().filter(|x| x.ttl == 4) {
            *x = resp(4, "", None, TipoResposta::Silencio);
        }
        let rota = montar(destino(), &r, &donos());
        assert_eq!(rota.saltos[3].trecho, Trecho::SemResposta);
        assert!(matches!(rota.veredito, Veredito::Problema { salto: 3, .. }));
    }

    #[test]
    fn na_vpn_o_comeco_e_tunel_e_a_saida_da_vpn_nao_vira_operadora() {
        let rota = montar_via(
            destino(),
            &cenario(&ROTA, |_, _| false, |_, _| 20),
            &donos(),
            true,
        );
        let t: Vec<Trecho> = rota.saltos.iter().map(|s| s.trecho).collect();
        assert_eq!(t[0], Trecho::Tunel);
        assert!(!t.contains(&Trecho::Casa));
        assert!(!t.contains(&Trecho::Operadora), "{:?}", t);
        assert_eq!(t[5], Trecho::Servidor);
    }

    #[test]
    fn variacao_que_nasce_no_tunel_manda_desligar_a_vpn_e_medir_de_novo() {
        let r = cenario(
            &ROTA,
            |_, _| false,
            |ttl, rod| 5 * ttl as u32 + if rod % 2 == 0 { 0 } else { 60 },
        );
        let rota = montar_via(destino(), &r, &donos(), true);
        assert!(matches!(
            rota.veredito,
            Veredito::Problema {
                trecho: Trecho::Tunel,
                salto: 1,
                ..
            }
        ));
        assert!(rota.frase.contains("túnel da VPN"));
        assert!(!rota.frase.contains("sua casa"));
    }

    #[test]
    fn mais_frequente_e_estavel_no_empate() {
        let a = ip("1.1.1.1");
        let b = ip("2.2.2.2");
        assert_eq!(mais_frequente([a, b, b].into_iter()), Some(b));
        assert_eq!(mais_frequente([a, b].into_iter()), Some(a));
        assert_eq!(mais_frequente(std::iter::empty()), None);
    }

    #[test]
    fn nenhuma_frase_promete_diminuir_o_ping() {
        for v in [
            Veredito::Limpa,
            Veredito::NaoMediu,
            Veredito::ServidorNaoResponde { ultimo_salto: 3 },
            Veredito::Problema {
                problema: Problema::Perda,
                trecho: Trecho::Casa,
                salto: 1,
                tamanho: 20.0,
                operadora: None,
            },
        ] {
            let f = frase(&v).to_lowercase();
            assert!(
                !f.contains("vamos diminuir") && !f.contains("ping menor"),
                "{}",
                f
            );
        }
    }
}

#[cfg(all(test, windows))]
mod na_maquina {
    /// Mede de verdade (~10 s, só ICMP): `cargo test --lib -- --ignored rota::na_maquina`.
    #[tokio::test]
    #[ignore]
    async fn rota_real_ate_um_resolvedor_publico() {
        let r = super::diagnosticar("1.1.1.1".parse().unwrap()).await;
        for s in &r.saltos {
            println!(
                "{:>2} {:<40} perda {:>3.0}% mediana {:?} jitter {:?} {:?} {:?}",
                s.numero,
                s.endereco.map(|e| e.to_string()).unwrap_or("*".into()),
                s.perda_pct,
                s.mediana_ms,
                s.jitter_ms,
                s.trecho,
                s.operadora
            );
        }
        println!("{}\n{:?}", r.frase, r.observacao);
        assert!(!r.saltos.is_empty(), "nenhum salto respondeu");
    }
}
