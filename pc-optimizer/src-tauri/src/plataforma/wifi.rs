// OTIMIZA REDE: POR ONDE O PC SAI E COMO ESTÁ O WI-FI. Só lê. Primeiro a placa por onde sai a rota (cabo, Wi-Fi ou
// placa virtual de VPN/máquina virtual); no Wi-Fi, a API nativa (wlanapi), em campos e não no texto traduzido do
// `netsh`: sinal em dBm, banda e canal pela frequência do ponto de acesso, padrão (Wi-Fi 4/5/6/7), velocidade do
// enlace e quantas redes vizinhas disputam o mesmo canal.
//
// No Windows 11 24H2 listar redes Wi-Fi conta como localização: sem "Permitir que aplicativos da área de trabalho
// acessem sua localização", a wlanapi responde ERROR_ACCESS_DENIED. Isso é um estado nomeado (`SemPermissaoDeLocalizacao`),
// não "Wi-Fi sem vizinhos": a lista vazia de verdade é outro estado (`ListaVazia`). Nenhum dos dois vira número falso.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Banda {
    #[serde(rename = "2,4 GHz")]
    G2_4,
    #[serde(rename = "5 GHz")]
    G5,
    #[serde(rename = "6 GHz")]
    G6,
}

impl Banda {
    pub fn nome(self) -> &'static str {
        match self {
            Banda::G2_4 => "2,4 GHz",
            Banda::G5 => "5 GHz",
            Banda::G6 => "6 GHz",
        }
    }
}

/// **Pura.** Banda e canal pela frequência central que a wlanapi entrega (em kHz). Fora das faixas: `None`.
pub fn banda_e_canal(khz: u32) -> Option<(Banda, u32)> {
    let mhz = (khz + 500) / 1000;
    match mhz {
        2412..=2472 => Some((Banda::G2_4, (mhz - 2407) / 5)),
        2484 => Some((Banda::G2_4, 14)),
        5160..=5885 => Some((Banda::G5, (mhz - 5000) / 5)),
        // O canal 2 do 6 GHz é o único fora da grade de 5 em 5 a partir de 5950.
        5935 => Some((Banda::G6, 2)),
        5955..=7115 => Some((Banda::G6, (mhz - 5950) / 5)),
        _ => None,
    }
}

/// **Pura.** Duas redes disputam o ar? Em 2,4 GHz os canais têm 20 MHz e ficam a 5 MHz um do outro: menos de 5 canais
/// de distância se sobrepõem (1, 6 e 11 são os únicos três que não se tocam). Em 5 e 6 GHz conta só o mesmo canal
/// (largura de 40/80/160 MHz não aparece na lista; o número fica por baixo, nunca por cima).
pub fn disputam(a: (Banda, u32), b: (Banda, u32)) -> bool {
    a.0 == b.0
        && match a.0 {
            Banda::G2_4 => a.1.abs_diff(b.1) < 5,
            _ => a.1 == b.1,
        }
}

/// Uma rede ouvida pela placa (um ponto de acesso, pelo BSSID).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vizinho {
    pub bssid: [u8; 6],
    pub ssid: String,
    pub khz: u32,
    pub rssi_dbm: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Vizinhanca {
    /// Outras redes exatamente no seu canal.
    pub mesmo_canal: u32,
    /// Outras redes em canal sobreposto ao seu (só existe em 2,4 GHz).
    pub sobrepostas: u32,
    /// Tudo que a placa ouviu, em qualquer banda, fora o seu ponto de acesso.
    pub total_ouvidas: u32,
}

impl Vizinhanca {
    /// Quem disputa o ar com você: mesmo canal mais os sobrepostos.
    pub fn disputando(&self) -> u32 {
        self.mesmo_canal + self.sobrepostas
    }
}

/// **Pura.** Conta os vizinhos do seu canal. O seu próprio ponto de acesso (o BSSID conectado) fica fora; os outros
/// rádios do mesmo roteador e os nós de uma malha (mesh) no mesmo canal contam, porque disputam o ar do mesmo jeito.
pub fn contar_vizinhos(meu_bssid: [u8; 6], meu: (Banda, u32), redes: &[Vizinho]) -> Vizinhanca {
    let mut v = Vizinhanca::default();
    for r in redes.iter().filter(|r| r.bssid != meu_bssid) {
        v.total_ouvidas += 1;
        let Some(dele) = banda_e_canal(r.khz) else {
            continue;
        };
        if dele == meu {
            v.mesmo_canal += 1;
        } else if disputam(dele, meu) {
            v.sobrepostas += 1;
        }
    }
    v
}

/// **Pura.** Sem a lista de redes, a wlanapi ainda dá a qualidade 0–100; a conversão documentada é linear entre
/// −100 dBm (0) e −50 dBm (100).
pub fn dbm_pela_qualidade(qualidade: u32) -> i32 {
    (qualidade.min(100) as i32) / 2 - 100
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Sinal {
    Forte,
    Bom,
    Fraco,
    MuitoFraco,
}

/// **Pura.** Régua usual para tempo real: até −60 dBm sobra; −70 começa a perder pacote em hora de pico; abaixo de −80
/// a placa vive retransmitindo.
pub fn avaliar_sinal(rssi_dbm: i32) -> Sinal {
    match rssi_dbm {
        r if r >= -60 => Sinal::Forte,
        r if r >= -70 => Sinal::Bom,
        r if r >= -80 => Sinal::Fraco,
        _ => Sinal::MuitoFraco,
    }
}

/// **Pura.** O nome comercial do padrão pelo tipo de camada física da wlanapi (`DOT11_PHY_TYPE`).
pub fn padrao(phy: i32, banda: Option<Banda>) -> Option<&'static str> {
    match phy {
        7 => Some("Wi-Fi 4"),
        8 => Some("Wi-Fi 5"),
        10 if banda == Some(Banda::G6) => Some("Wi-Fi 6E"),
        10 => Some("Wi-Fi 6"),
        11 => Some("Wi-Fi 7"),
        _ => None,
    }
}

/// A conexão Wi-Fi atual, lida da wlanapi.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConexaoWifi {
    pub ssid: String,
    pub banda: Option<Banda>,
    pub canal: Option<u32>,
    pub rssi_dbm: i32,
    pub sinal: Sinal,
    /// Qualidade 0–100 que o próprio Windows mostra.
    pub qualidade: u32,
    pub padrao: Option<&'static str>,
    /// Velocidade do enlace, em Mb/s (o que o rádio negociou, não a internet).
    pub recepcao_mbps: u32,
    pub envio_mbps: u32,
}

/// O que deu para ler do Wi-Fi. Cada "não deu" tem nome: a tela explica em vez de mostrar zero.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "estado", rename_all = "snake_case")]
pub enum LeituraWifi {
    Lida {
        conexao: ConexaoWifi,
        /// `None` quando o seu ponto de acesso não apareceu na lista (sem frequência, sem canal para comparar).
        vizinhanca: Option<Vizinhanca>,
    },
    /// Windows 11 24H2: listar redes exige a permissão de localização para apps da área de trabalho.
    SemPermissaoDeLocalizacao,
    /// Conectado, mas a lista de redes veio vazia (placa no meio de uma varredura, driver que não informa).
    ListaVazia { conexao: Option<ConexaoWifi> },
    /// O serviço do Wi-Fi (WLAN AutoConfig) não está rodando ou a placa não está na wlanapi.
    NaoLi { motivo: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TipoDePlaca {
    Cabo,
    WiFi,
    /// VPN, máquina virtual, túnel: ping e rota medidos são os do túnel.
    Virtual,
    Outra,
}

/// **Pura.** Pelo tipo de interface (IANA) e pelo nome da placa. Placa virtual vence: um adaptador do Hyper-V se
/// declara Ethernet, e um cliente de VPN se declara Ethernet ou túnel.
pub fn tipo_de_placa(if_type: u32, descricao: &str) -> TipoDePlaca {
    const VIRTUAIS: [&str; 16] = [
        "hyper-v",
        "virtual",
        "vmware",
        "virtualbox",
        "tap-windows",
        "tap adapter",
        "wireguard",
        "wintun",
        "vpn",
        "tunnel",
        "zerotier",
        "tailscale",
        "radmin",
        "hamachi",
        "openvpn",
        "loopback",
    ];
    let d = descricao.to_lowercase();
    // 23 = PPP, 53 = virtual proprietária, 131 = túnel.
    if matches!(if_type, 23 | 53 | 131) || VIRTUAIS.iter().any(|v| d.contains(v)) {
        return TipoDePlaca::Virtual;
    }
    match if_type {
        71 => TipoDePlaca::WiFi,
        6 => TipoDePlaca::Cabo,
        _ => TipoDePlaca::Outra,
    }
}

/// Por onde a rota sai agora.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Ligacao {
    Cabo {
        placa: String,
        velocidade_mbps: Option<u64>,
    },
    WiFi {
        placa: String,
        leitura: LeituraWifi,
    },
    Virtual {
        placa: String,
    },
    Outra {
        placa: String,
    },
    /// Nenhuma rota para a internet.
    SemRede,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Gravidade {
    Bom,
    Info,
    Atencao,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Achado {
    pub gravidade: Gravidade,
    pub frase: String,
}

fn achado(gravidade: Gravidade, frase: impl Into<String>) -> Achado {
    Achado {
        gravidade,
        frase: frase.into(),
    }
}

/// A partir de quantas redes no canal o Wi-Fi conta como lotado.
const CANAL_LOTADO: u32 = 4;

/// **Pura.** O que dizer, em português simples. Nunca promete ping menor: diz onde está o que dá para mudar.
pub fn achados(l: &Ligacao) -> Vec<Achado> {
    match l {
        Ligacao::SemRede => vec![achado(
            Gravidade::Atencao,
            "Sem rota para a internet agora: nenhuma placa de rede está ligada a uma rede.",
        )],
        Ligacao::Cabo {
            velocidade_mbps: Some(v),
            ..
        } if *v > 0 && *v <= 100 => vec![achado(
            Gravidade::Atencao,
            format!(
                "Seu cabo negociou {} Mb: cabo, porta ou roteador com defeito (um cabo bom negocia 1000). Não aumenta o ping, mas limita downloads e pode ser sinal de cabo com fio rompido.",
                v
            ),
        )],
        Ligacao::Cabo { .. } => vec![achado(
            Gravidade::Bom,
            "Você está no cabo: a ligação mais estável que existe para jogar.",
        )],
        Ligacao::Virtual { placa } => vec![achado(
            Gravidade::Info,
            format!(
                "A internet sai por uma placa virtual ({}), de VPN ou máquina virtual. O ping e a rota medidos aqui são os do túnel, não os da sua conexão.",
                placa
            ),
        )],
        Ligacao::Outra { placa } => vec![achado(
            Gravidade::Info,
            format!("A internet sai por {}, que não é cabo nem Wi-Fi comum.", placa),
        )],
        Ligacao::WiFi { leitura, .. } => achados_do_wifi(leitura),
    }
}

fn achados_do_wifi(leitura: &LeituraWifi) -> Vec<Achado> {
    match leitura {
        LeituraWifi::SemPermissaoDeLocalizacao => vec![achado(
            Gravidade::Info,
            "Você está no Wi-Fi, mas o Windows não deixa o Otimiza ver as redes: ligue \"Permitir que aplicativos da área de trabalho acessem sua localização\" em Configurações > Privacidade e segurança > Localização. O Otimiza não usa a sua localização; o Windows trata a lista de redes Wi-Fi como se fosse.",
        )],
        LeituraWifi::ListaVazia { conexao } => {
            let mut v = vec![achado(
                Gravidade::Info,
                "Você está no Wi-Fi, mas o Windows devolveu a lista de redes vazia: sem banda, canal nem vizinhos para avaliar agora. Tente de novo em alguns segundos.",
            )];
            if let Some(c) = conexao {
                v.extend(achado_do_sinal(c));
            }
            v
        }
        LeituraWifi::NaoLi { motivo } => vec![achado(
            Gravidade::Info,
            format!("Você está no Wi-Fi, mas não consegui ler os detalhes dele: {}.", motivo),
        )],
        LeituraWifi::Lida {
            conexao,
            vizinhanca,
        } => achados_da_conexao(conexao, vizinhanca.as_ref()),
    }
}

fn achado_do_sinal(c: &ConexaoWifi) -> Option<Achado> {
    matches!(c.sinal, Sinal::Fraco | Sinal::MuitoFraco).then(|| {
        achado(
            Gravidade::Atencao,
            format!(
                "Sinal fraco ({} dBm): a placa retransmite pacote, e cada retransmissão é um pico de ping. Aproxime o PC do roteador ou use cabo.",
                c.rssi_dbm
            ),
        )
    })
}

fn achados_da_conexao(c: &ConexaoWifi, viz: Option<&Vizinhanca>) -> Vec<Achado> {
    let fraco = matches!(c.sinal, Sinal::Fraco | Sinal::MuitoFraco);
    let disputando = viz.map(|v| v.disputando()).unwrap_or(0);
    let lotado = disputando >= CANAL_LOTADO;
    let em_2_4 = c.banda == Some(Banda::G2_4);

    let mut problemas: Vec<String> = Vec::new();
    if fraco {
        problemas.push(format!("sinal fraco ({} dBm)", c.rssi_dbm));
    }
    if lotado {
        problemas.push(format!("{} redes no seu canal", disputando));
    }

    if problemas.is_empty() {
        let onde = match (c.banda, c.canal) {
            (Some(b), Some(ch)) => format!(" em {}, canal {}", b.nome(), ch),
            (Some(b), None) => format!(" em {}", b.nome()),
            _ => String::new(),
        };
        let mut v = vec![achado(
            Gravidade::Bom,
            format!(
                "Wi-Fi{} com sinal bom ({} dBm) e canal sem disputa forte.",
                onde, c.rssi_dbm
            ),
        )];
        if em_2_4 {
            v.push(achado(
                Gravidade::Info,
                "Ainda é 2,4 GHz, a faixa que micro-ondas, Bluetooth e os vizinhos mais usam: se o roteador tiver 5 GHz, o ping varia menos lá.",
            ));
        }
        return v;
    }

    let lista = problemas.join(" e ");
    let conselho = if em_2_4 {
        "use 5 GHz ou cabo"
    } else if fraco {
        "aproxime o PC do roteador ou use cabo"
    } else {
        "troque o canal do roteador ou use cabo"
    };
    let frase = if em_2_4 {
        format!("2,4 GHz com {}: {}.", lista, conselho)
    } else {
        format!("{}: {}.", maiuscula(&lista), conselho)
    };
    vec![achado(Gravidade::Atencao, frase)]
}

fn maiuscula(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(p) => p.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Diagnóstico completo: por onde sai e o que dizer.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostico {
    pub ligacao: Ligacao,
    pub achados: Vec<Achado>,
}

#[cfg(windows)]
pub use nativo::{diagnosticar, placa_da_rota};

#[cfg(windows)]
mod nativo {
    use super::*;
    use std::net::IpAddr;
    use windows_sys::core::GUID;
    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_BUFFER_OVERFLOW, HANDLE};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GetBestInterfaceEx, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
        GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::NetworkManagement::WiFi::*;
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
    };

    /// A placa por onde o Windows mandaria um pacote para `destino`.
    #[derive(Debug, Clone)]
    pub struct PlacaDaRota {
        pub nome: String,
        pub descricao: String,
        pub tipo: TipoDePlaca,
        /// O GUID da placa, `{XXXXXXXX-...}`: é o mesmo da interface na wlanapi.
        pub guid: String,
        pub velocidade_mbps: Option<u64>,
    }

    fn texto(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        // SAFETY: o Windows entrega texto terminado em zero dentro do buffer que está vivo.
        unsafe {
            let mut n = 0;
            while *p.add(n) != 0 {
                n += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
        }
    }

    fn texto_ansi(p: *const u8) -> String {
        if p.is_null() {
            return String::new();
        }
        // SAFETY: idem, ANSI terminado em zero.
        unsafe {
            std::ffi::CStr::from_ptr(p as *const i8)
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Índice da interface pela tabela de rotas: nenhum pacote sai.
    fn indice_da_rota(destino: IpAddr) -> Option<u32> {
        let mut indice = 0u32;
        // SAFETY: estruturas zeradas e preenchidas antes da chamada; o Windows só lê.
        let r = unsafe {
            match destino {
                IpAddr::V4(v4) => {
                    let mut s: SOCKADDR_IN = std::mem::zeroed();
                    s.sin_family = AF_INET;
                    s.sin_addr.S_un.S_addr = u32::from_ne_bytes(v4.octets());
                    GetBestInterfaceEx(&s as *const _ as *const SOCKADDR, &mut indice)
                }
                IpAddr::V6(v6) => {
                    let mut s: SOCKADDR_IN6 = std::mem::zeroed();
                    s.sin6_family = AF_INET6;
                    s.sin6_addr.u.Byte = v6.octets();
                    GetBestInterfaceEx(&s as *const _ as *const SOCKADDR, &mut indice)
                }
            }
        };
        (r == 0).then_some(indice)
    }

    pub fn placa_da_rota(destino: IpAddr) -> Option<PlacaDaRota> {
        let indice = indice_da_rota(destino)?;
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut tamanho: u32 = 16 * 1024;
        for _ in 0..3 {
            // u64 para o alinhamento da estrutura.
            let mut buf = vec![0u64; (tamanho as usize).div_ceil(8)];
            let p = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
            // SAFETY: buffer do tamanho informado; a lista encadeada aponta para dentro dele.
            let r = unsafe {
                GetAdaptersAddresses(AF_UNSPEC as u32, flags, std::ptr::null(), p, &mut tamanho)
            };
            if r == ERROR_BUFFER_OVERFLOW {
                continue;
            }
            if r != 0 {
                return None;
            }
            let mut atual = p;
            while !atual.is_null() {
                // SAFETY: nó da lista dentro de `buf`, que segue vivo.
                let a = unsafe { &*atual };
                let if_index = unsafe { a.Anonymous1.Anonymous.IfIndex };
                if if_index == indice || a.Ipv6IfIndex == indice {
                    let descricao = texto(a.Description);
                    let velocidade = a.TransmitLinkSpeed.max(a.ReceiveLinkSpeed);
                    return Some(PlacaDaRota {
                        nome: texto(a.FriendlyName),
                        tipo: tipo_de_placa(a.IfType, &descricao),
                        descricao,
                        guid: texto_ansi(a.AdapterName as *const u8),
                        // u64::MAX é "não sei" na API.
                        velocidade_mbps: (velocidade > 0 && velocidade != u64::MAX)
                            .then(|| velocidade / 1_000_000),
                    });
                }
                atual = a.Next;
            }
            return None;
        }
        None
    }

    pub(crate) fn guid_texto(g: &GUID) -> String {
        format!(
            "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
            g.data1,
            g.data2,
            g.data3,
            g.data4[0],
            g.data4[1],
            g.data4[2],
            g.data4[3],
            g.data4[4],
            g.data4[5],
            g.data4[6],
            g.data4[7]
        )
    }

    /// Cliente da wlanapi; fecha sozinho.
    pub(crate) struct Cliente(pub(crate) HANDLE);

    impl Cliente {
        pub(crate) fn abrir() -> Result<Self, u32> {
            let mut versao = 0u32;
            let mut h: HANDLE = std::ptr::null_mut();
            // SAFETY: saídas válidas; versão 2 = Vista em diante.
            let r = unsafe { WlanOpenHandle(2, std::ptr::null(), &mut versao, &mut h) };
            if r == 0 {
                Ok(Cliente(h))
            } else {
                Err(r)
            }
        }

        /// A interface da wlanapi com este GUID de placa.
        pub(crate) fn interface(&self, guid_da_placa: &str) -> Option<GUID> {
            let mut lista: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
            // SAFETY: a lista é liberada logo abaixo com WlanFreeMemory.
            unsafe {
                if WlanEnumInterfaces(self.0, std::ptr::null(), &mut lista) != 0 || lista.is_null()
                {
                    return None;
                }
                let n = (*lista).dwNumberOfItems as usize;
                let itens = std::slice::from_raw_parts(
                    std::ptr::addr_of!((*lista).InterfaceInfo) as *const WLAN_INTERFACE_INFO,
                    n,
                );
                let achado = itens
                    .iter()
                    .map(|i| i.InterfaceGuid)
                    .find(|g| guid_texto(g).eq_ignore_ascii_case(guid_da_placa));
                WlanFreeMemory(lista as *const _);
                achado
            }
        }
    }

    impl Drop for Cliente {
        fn drop(&mut self) {
            // SAFETY: handle aberto por WlanOpenHandle.
            unsafe {
                WlanCloseHandle(self.0, std::ptr::null());
            }
        }
    }

    fn ssid(s: &DOT11_SSID) -> String {
        let n = (s.uSSIDLength as usize).min(s.ucSSID.len());
        String::from_utf8_lossy(&s.ucSSID[..n]).into_owned()
    }

    struct Atual {
        ssid: String,
        bssid: [u8; 6],
        phy: i32,
        qualidade: u32,
        rx_kbps: u32,
        tx_kbps: u32,
    }

    fn conexao_atual(c: &Cliente, g: &GUID) -> Result<Atual, u32> {
        let mut tamanho = 0u32;
        let mut dados: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: a wlanapi aloca `dados`; liberado com WlanFreeMemory.
        unsafe {
            let r = WlanQueryInterface(
                c.0,
                g,
                wlan_intf_opcode_current_connection,
                std::ptr::null(),
                &mut tamanho,
                &mut dados,
                std::ptr::null_mut(),
            );
            if r != 0 || dados.is_null() {
                return Err(r);
            }
            let a = &(*(dados as *const WLAN_CONNECTION_ATTRIBUTES)).wlanAssociationAttributes;
            let atual = Atual {
                ssid: ssid(&a.dot11Ssid),
                bssid: a.dot11Bssid,
                phy: a.dot11PhyType,
                qualidade: a.wlanSignalQuality,
                rx_kbps: a.ulRxRate,
                tx_kbps: a.ulTxRate,
            };
            WlanFreeMemory(dados);
            Ok(atual)
        }
    }

    fn redes(c: &Cliente, g: &GUID) -> Result<Vec<Vizinho>, u32> {
        let mut lista: *mut WLAN_BSS_LIST = std::ptr::null_mut();
        // SAFETY: idem; os itens são copiados antes de liberar.
        unsafe {
            let r = WlanGetNetworkBssList(
                c.0,
                g,
                std::ptr::null(),
                dot11_BSS_type_any,
                0,
                std::ptr::null(),
                &mut lista,
            );
            if r != 0 || lista.is_null() {
                return Err(r);
            }
            let n = (*lista).dwNumberOfItems as usize;
            let itens = std::slice::from_raw_parts(
                std::ptr::addr_of!((*lista).wlanBssEntries) as *const WLAN_BSS_ENTRY,
                n,
            );
            let v = itens
                .iter()
                .map(|e| Vizinho {
                    bssid: e.dot11Bssid,
                    ssid: ssid(&e.dot11Ssid),
                    khz: e.ulChCenterFrequency,
                    rssi_dbm: e.lRssi,
                })
                .collect();
            WlanFreeMemory(lista as *const _);
            Ok(v)
        }
    }

    fn montar(a: &Atual, minha: Option<&Vizinho>) -> ConexaoWifi {
        let canal = minha.and_then(|m| banda_e_canal(m.khz));
        let rssi = minha
            .map(|m| m.rssi_dbm)
            .unwrap_or_else(|| dbm_pela_qualidade(a.qualidade));
        ConexaoWifi {
            ssid: a.ssid.clone(),
            banda: canal.map(|c| c.0),
            canal: canal.map(|c| c.1),
            rssi_dbm: rssi,
            sinal: avaliar_sinal(rssi),
            qualidade: a.qualidade,
            padrao: padrao(a.phy, canal.map(|c| c.0)),
            recepcao_mbps: a.rx_kbps / 1000,
            envio_mbps: a.tx_kbps / 1000,
        }
    }

    pub(crate) fn ler_wifi(guid_da_placa: &str) -> LeituraWifi {
        let cliente = match Cliente::abrir() {
            Ok(c) => c,
            Err(e) => {
                return LeituraWifi::NaoLi {
                    motivo: format!("o serviço do Wi-Fi do Windows não respondeu (erro {})", e),
                }
            }
        };
        let Some(g) = cliente.interface(guid_da_placa) else {
            return LeituraWifi::NaoLi {
                motivo: "a placa não aparece na lista de placas Wi-Fi do Windows".into(),
            };
        };
        let atual = match conexao_atual(&cliente, &g) {
            Ok(a) => a,
            Err(ERROR_ACCESS_DENIED) => return LeituraWifi::SemPermissaoDeLocalizacao,
            Err(e) => {
                return LeituraWifi::NaoLi {
                    motivo: format!("o Windows recusou ler a conexão atual (erro {})", e),
                }
            }
        };
        // Sem a permissão, alguns builds devolvem a conexão com SSID e BSSID zerados em vez de recusar.
        if atual.ssid.is_empty() && atual.bssid == [0; 6] {
            return LeituraWifi::SemPermissaoDeLocalizacao;
        }
        let lista = match redes(&cliente, &g) {
            Ok(l) => l,
            Err(ERROR_ACCESS_DENIED) => return LeituraWifi::SemPermissaoDeLocalizacao,
            Err(_) => Vec::new(),
        };
        if lista.is_empty() {
            return LeituraWifi::ListaVazia {
                conexao: Some(montar(&atual, None)),
            };
        }
        let minha = lista.iter().find(|r| r.bssid == atual.bssid);
        let conexao = montar(&atual, minha);
        let vizinhanca = match (conexao.banda, conexao.canal) {
            (Some(b), Some(ch)) => Some(contar_vizinhos(atual.bssid, (b, ch), &lista)),
            _ => None,
        };
        LeituraWifi::Lida {
            conexao,
            vizinhanca,
        }
    }

    /// Um endereço público qualquer só para consultar a tabela de rotas (nenhum pacote sai).
    const REFERENCIA: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1));

    pub fn ligacao(destino: Option<IpAddr>) -> Ligacao {
        let Some(p) = placa_da_rota(destino.unwrap_or(REFERENCIA)) else {
            return Ligacao::SemRede;
        };
        let placa = if p.descricao.is_empty() {
            p.nome.clone()
        } else {
            p.descricao.clone()
        };
        match p.tipo {
            TipoDePlaca::Cabo => Ligacao::Cabo {
                placa,
                velocidade_mbps: p.velocidade_mbps,
            },
            TipoDePlaca::WiFi => Ligacao::WiFi {
                leitura: ler_wifi(&p.guid),
                placa,
            },
            TipoDePlaca::Virtual => Ligacao::Virtual { placa },
            TipoDePlaca::Outra => Ligacao::Outra { placa },
        }
    }

    /// Por onde sai a rota para `destino` (ou para a internet) e o que dizer dela.
    pub fn diagnosticar(destino: Option<IpAddr>) -> Diagnostico {
        let ligacao = ligacao(destino);
        Diagnostico {
            achados: achados(&ligacao),
            ligacao,
        }
    }
}

#[cfg(windows)]
pub(crate) use nativo::Cliente;

#[cfg(test)]
mod testes {
    use super::*;

    fn rede(bssid: u8, mhz: u32) -> Vizinho {
        Vizinho {
            bssid: [bssid; 6],
            ssid: format!("rede{}", bssid),
            khz: mhz * 1000,
            rssi_dbm: -70,
        }
    }

    fn conexao(banda: Banda, canal: u32, rssi: i32) -> ConexaoWifi {
        ConexaoWifi {
            ssid: "casa".into(),
            banda: Some(banda),
            canal: Some(canal),
            rssi_dbm: rssi,
            sinal: avaliar_sinal(rssi),
            qualidade: 50,
            padrao: Some("Wi-Fi 5"),
            recepcao_mbps: 144,
            envio_mbps: 144,
        }
    }

    fn wifi(leitura: LeituraWifi) -> Ligacao {
        Ligacao::WiFi {
            placa: "Intel Wi-Fi 6 AX201".into(),
            leitura,
        }
    }

    #[test]
    fn banda_e_canal_pela_frequencia() {
        assert_eq!(banda_e_canal(2_412_000), Some((Banda::G2_4, 1)));
        assert_eq!(banda_e_canal(2_437_000), Some((Banda::G2_4, 6)));
        assert_eq!(banda_e_canal(2_462_000), Some((Banda::G2_4, 11)));
        assert_eq!(banda_e_canal(2_484_000), Some((Banda::G2_4, 14)));
        assert_eq!(banda_e_canal(5_180_000), Some((Banda::G5, 36)));
        assert_eq!(banda_e_canal(5_745_000), Some((Banda::G5, 149)));
        assert_eq!(banda_e_canal(5_955_000), Some((Banda::G6, 1)));
        assert_eq!(banda_e_canal(5_935_000), Some((Banda::G6, 2)));
        assert_eq!(banda_e_canal(6_115_000), Some((Banda::G6, 33)));
        assert_eq!(banda_e_canal(0), None);
        assert_eq!(banda_e_canal(900_000), None);
    }

    #[test]
    fn frequencia_quebrada_em_khz_arredonda() {
        assert_eq!(banda_e_canal(2_436_999), Some((Banda::G2_4, 6)));
    }

    #[test]
    fn em_2_4_ghz_canais_proximos_se_sobrepoem_e_1_6_11_nao() {
        let c = |n| (Banda::G2_4, n);
        assert!(disputam(c(6), c(6)));
        assert!(disputam(c(6), c(4)));
        assert!(disputam(c(6), c(10)));
        assert!(!disputam(c(1), c(6)));
        assert!(!disputam(c(6), c(11)));
        assert!(!disputam(c(1), c(11)));
    }

    #[test]
    fn em_5_ghz_so_o_mesmo_canal_disputa_e_banda_diferente_nunca() {
        assert!(disputam((Banda::G5, 36), (Banda::G5, 36)));
        assert!(!disputam((Banda::G5, 36), (Banda::G5, 40)));
        assert!(!disputam((Banda::G5, 6), (Banda::G2_4, 6)));
        assert!(!disputam((Banda::G6, 1), (Banda::G5, 36)));
    }

    #[test]
    fn conta_vizinhos_sem_contar_o_proprio_ponto_de_acesso() {
        let redes = vec![
            rede(1, 2437), // eu, canal 6
            rede(2, 2437), // mesmo canal
            rede(3, 2437), // mesmo canal
            rede(4, 2427), // canal 4: sobrepõe
            rede(5, 2412), // canal 1: não sobrepõe
            rede(6, 5180), // 5 GHz: outra banda
        ];
        let v = contar_vizinhos([1; 6], (Banda::G2_4, 6), &redes);
        assert_eq!(v.mesmo_canal, 2);
        assert_eq!(v.sobrepostas, 1);
        assert_eq!(v.total_ouvidas, 5);
        assert_eq!(v.disputando(), 3);
    }

    #[test]
    fn vizinho_com_frequencia_desconhecida_conta_como_ouvido_e_nao_como_disputa() {
        let redes = vec![rede(2, 0)];
        let v = contar_vizinhos([1; 6], (Banda::G2_4, 6), &redes);
        assert_eq!((v.total_ouvidas, v.disputando()), (1, 0));
    }

    #[test]
    fn qualidade_vira_dbm_pela_regra_do_windows() {
        assert_eq!(dbm_pela_qualidade(0), -100);
        assert_eq!(dbm_pela_qualidade(100), -50);
        assert_eq!(dbm_pela_qualidade(60), -70);
        assert_eq!(dbm_pela_qualidade(250), -50);
    }

    #[test]
    fn regua_do_sinal() {
        assert_eq!(avaliar_sinal(-45), Sinal::Forte);
        assert_eq!(avaliar_sinal(-60), Sinal::Forte);
        assert_eq!(avaliar_sinal(-65), Sinal::Bom);
        assert_eq!(avaliar_sinal(-75), Sinal::Fraco);
        assert_eq!(avaliar_sinal(-85), Sinal::MuitoFraco);
    }

    #[test]
    fn padrao_comercial_pela_camada_fisica() {
        assert_eq!(padrao(7, Some(Banda::G2_4)), Some("Wi-Fi 4"));
        assert_eq!(padrao(8, Some(Banda::G5)), Some("Wi-Fi 5"));
        assert_eq!(padrao(10, Some(Banda::G5)), Some("Wi-Fi 6"));
        assert_eq!(padrao(10, Some(Banda::G6)), Some("Wi-Fi 6E"));
        assert_eq!(padrao(11, None), Some("Wi-Fi 7"));
        assert_eq!(padrao(4, None), None);
    }

    #[test]
    fn placa_virtual_vence_o_tipo_que_ela_declara() {
        assert_eq!(
            tipo_de_placa(6, "Realtek PCIe GbE Family Controller"),
            TipoDePlaca::Cabo
        );
        assert_eq!(
            tipo_de_placa(71, "Intel(R) Wi-Fi 6 AX201 160MHz"),
            TipoDePlaca::WiFi
        );
        assert_eq!(
            tipo_de_placa(6, "Hyper-V Virtual Ethernet Adapter"),
            TipoDePlaca::Virtual
        );
        assert_eq!(
            tipo_de_placa(6, "TAP-Windows Adapter V9"),
            TipoDePlaca::Virtual
        );
        assert_eq!(tipo_de_placa(53, "WireGuard Tunnel"), TipoDePlaca::Virtual);
        assert_eq!(tipo_de_placa(131, "Qualquer"), TipoDePlaca::Virtual);
        assert_eq!(tipo_de_placa(243, "Modem celular"), TipoDePlaca::Outra);
    }

    #[test]
    fn a_frase_do_plano_2_4_ghz_sinal_fraco_e_canal_lotado() {
        let l = wifi(LeituraWifi::Lida {
            conexao: conexao(Banda::G2_4, 6, -78),
            vizinhanca: Some(Vizinhanca {
                mesmo_canal: 6,
                sobrepostas: 3,
                total_ouvidas: 14,
            }),
        });
        let a = achados(&l);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].gravidade, Gravidade::Atencao);
        assert_eq!(
            a[0].frase,
            "2,4 GHz com sinal fraco (-78 dBm) e 9 redes no seu canal: use 5 GHz ou cabo."
        );
    }

    #[test]
    fn em_5_ghz_com_sinal_fraco_o_conselho_e_aproximar() {
        let l = wifi(LeituraWifi::Lida {
            conexao: conexao(Banda::G5, 36, -79),
            vizinhanca: Some(Vizinhanca::default()),
        });
        let a = achados(&l);
        assert_eq!(
            a[0].frase,
            "Sinal fraco (-79 dBm): aproxime o PC do roteador ou use cabo."
        );
    }

    #[test]
    fn em_5_ghz_lotado_o_conselho_e_trocar_o_canal() {
        let l = wifi(LeituraWifi::Lida {
            conexao: conexao(Banda::G5, 36, -55),
            vizinhanca: Some(Vizinhanca {
                mesmo_canal: 5,
                sobrepostas: 0,
                total_ouvidas: 9,
            }),
        });
        assert_eq!(
            achados(&l)[0].frase,
            "5 redes no seu canal: troque o canal do roteador ou use cabo."
        );
    }

    #[test]
    fn wifi_bom_e_elogiado_e_2_4_limpo_so_ganha_uma_dica() {
        let bom = wifi(LeituraWifi::Lida {
            conexao: conexao(Banda::G5, 36, -52),
            vizinhanca: Some(Vizinhanca::default()),
        });
        let a = achados(&bom);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].gravidade, Gravidade::Bom);
        assert!(a[0].frase.contains("5 GHz, canal 36"));

        let limpo_2_4 = wifi(LeituraWifi::Lida {
            conexao: conexao(Banda::G2_4, 1, -50),
            vizinhanca: Some(Vizinhanca {
                mesmo_canal: 1,
                sobrepostas: 0,
                total_ouvidas: 3,
            }),
        });
        let a = achados(&limpo_2_4);
        assert_eq!(a.len(), 2);
        assert_eq!(a[1].gravidade, Gravidade::Info);
        assert!(a.iter().all(|x| x.gravidade != Gravidade::Atencao));
    }

    #[test]
    fn sem_permissao_de_localizacao_e_estado_proprio_e_ensina_onde_ligar() {
        let a = achados(&wifi(LeituraWifi::SemPermissaoDeLocalizacao));
        assert_eq!(a.len(), 1);
        assert!(a[0].frase.contains("Localização"));
        assert!(a[0].frase.contains("não usa a sua localização"));
        assert!(!a[0].frase.contains("redes no seu canal"));
    }

    #[test]
    fn lista_vazia_nao_vira_canal_sem_vizinhos() {
        let a = achados(&wifi(LeituraWifi::ListaVazia { conexao: None }));
        assert_eq!(a.len(), 1);
        assert!(a[0].frase.contains("vazia"));
        assert!(a.iter().all(|x| x.gravidade != Gravidade::Bom));

        let mut c = conexao(Banda::G5, 36, -82);
        c.banda = None;
        c.canal = None;
        let a = achados(&wifi(LeituraWifi::ListaVazia { conexao: Some(c) }));
        assert_eq!(a.len(), 2, "o sinal fraco ainda é dito");
        assert!(a[1].frase.contains("-82 dBm"));
    }

    #[test]
    fn cabo_que_negociou_100_mb_e_avisado_sem_prometer_ping() {
        let a = achados(&Ligacao::Cabo {
            placa: "Realtek".into(),
            velocidade_mbps: Some(100),
        });
        assert_eq!(a[0].gravidade, Gravidade::Atencao);
        assert!(a[0].frase.starts_with("Seu cabo negociou 100 Mb"));
        assert!(a[0].frase.contains("Não aumenta o ping"));

        let bom = achados(&Ligacao::Cabo {
            placa: "Realtek".into(),
            velocidade_mbps: Some(1000),
        });
        assert_eq!(bom[0].gravidade, Gravidade::Bom);
        let sem_leitura = achados(&Ligacao::Cabo {
            placa: "Realtek".into(),
            velocidade_mbps: None,
        });
        assert_eq!(sem_leitura[0].gravidade, Gravidade::Bom);
    }

    #[test]
    fn placa_virtual_avisa_que_a_medida_e_do_tunel() {
        let a = achados(&Ligacao::Virtual {
            placa: "WireGuard Tunnel".into(),
        });
        assert!(a[0].frase.contains("túnel"));
        assert!(achados(&Ligacao::SemRede)[0].frase.contains("Sem rota"));
    }

    #[test]
    fn nenhuma_frase_promete_diminuir_o_ping() {
        let casos = [
            wifi(LeituraWifi::SemPermissaoDeLocalizacao),
            wifi(LeituraWifi::Lida {
                conexao: conexao(Banda::G2_4, 6, -85),
                vizinhanca: Some(Vizinhanca {
                    mesmo_canal: 9,
                    sobrepostas: 0,
                    total_ouvidas: 9,
                }),
            }),
            Ligacao::Cabo {
                placa: "x".into(),
                velocidade_mbps: Some(10),
            },
        ];
        for l in casos {
            for a in achados(&l) {
                let f = a.frase.to_lowercase();
                assert!(
                    !f.contains("diminui o ping") && !f.contains("ping menor"),
                    "{}",
                    a.frase
                );
            }
        }
    }
}

#[cfg(all(test, windows))]
mod na_maquina {
    /// Só lê: `cargo test --lib -- --ignored wifi::na_maquina`.
    #[test]
    #[ignore]
    fn diagnostico_real() {
        let d = super::diagnosticar(None);
        println!("{:#?}", d);
        assert!(!d.achados.is_empty());
    }
}
