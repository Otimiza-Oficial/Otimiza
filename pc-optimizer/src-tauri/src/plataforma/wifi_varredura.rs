// WI-FI SEM VARREDURA DURANTE O JOGO. Conectado, a placa Wi-Fi ainda sai do canal a cada ~60 s para procurar outras
// redes (varredura em segundo plano); em muitas placas isso é um pico de ping a cada minuto. Enquanto o jogo está
// aberto, o Otimiza pede ao Windows para não varrer; quando ele fecha, devolve.
//
// O pedido vale enquanto o cliente da wlanapi que o fez está aberto ("a varredura fica desligada se ao menos um cliente
// a desligar") e o Windows o desfaz sozinho se a placa desconectar. Por isso não há nada para devolver no disco: se o
// Otimiza cair, o cliente fecha com o processo e a varredura volta. O disco guarda só a escolha (ligado ou não).
// Mesmo modelo da energia de jogo (`energia_jogo`): o vigia do `lib.rs` chama `passo` quando o jogo muda.
// Só age no Wi-Fi; no cabo não há o que desligar.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "EstadoWifiVarredura"))]
pub struct Estado {
    pub ligado: bool,
}

/// Para a tela.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Painel {
    pub ligado: bool,
    /// O jogo durante o qual a varredura está desligada agora.
    pub agindo_em: Option<String>,
    /// Por que não vai agir aqui (no cabo, por exemplo). Não impede de ligar: vale quando a pessoa estiver no Wi-Fi.
    pub observacao: Option<String>,
}

fn arquivo() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("rede")
        .join("wifi_sem_varredura.json")
}

/// Sem escolha gravada: desligada.
pub fn ler() -> Estado {
    std::fs::read_to_string(arquivo())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn gravar(e: &Estado) -> Result<(), String> {
    let a = arquivo();
    if let Some(p) = a.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let texto = serde_json::to_string_pretty(e).map_err(|e| e.to_string())?;
    std::fs::write(a, texto)
        .map_err(|e| format!("Não foi possível gravar a escolha do Wi-Fi: {}", e))
}

/// O que fazer num passo do vigia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acao {
    Desligar,
    Devolver,
    Nada,
}

/// **Pura.** Jogo aberto, escolha ligada e nada aplicado: desliga. Jogo fechado (ou escolha desligada) com algo
/// aplicado: devolve.
pub fn decidir(ligado: bool, jogo_aberto: bool, aplicado: bool) -> Acao {
    match (ligado && jogo_aberto, aplicado) {
        (true, false) => Acao::Desligar,
        (false, true) => Acao::Devolver,
        _ => Acao::Nada,
    }
}

#[cfg(windows)]
pub use nativo::{definir, painel, passo};

#[cfg(windows)]
mod nativo {
    use super::*;
    use crate::plataforma::wifi::{self, Cliente, TipoDePlaca};
    use std::sync::Mutex;
    use windows_sys::core::GUID;
    use windows_sys::Win32::NetworkManagement::WiFi::{
        wlan_intf_opcode_background_scan_enabled, WlanSetInterface,
    };

    /// O cliente aberto que segura a varredura desligada. HANDLE é ponteiro (não `Send`): guardado o `Cliente` dentro
    /// do `Mutex` com `Send` declarado abaixo.
    struct Ativo {
        cliente: Cliente,
        interface: GUID,
        jogo: String,
    }

    // SAFETY: o handle da wlanapi pode ser usado de qualquer thread; o acesso é serializado pelo Mutex.
    unsafe impl Send for Ativo {}

    static ATIVO: Mutex<Option<Ativo>> = Mutex::new(None);

    fn ativo() -> std::sync::MutexGuard<'static, Option<Ativo>> {
        ATIVO.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn varredura(c: &Cliente, g: &GUID, ligada: bool) -> u32 {
        let valor: i32 = ligada as i32;
        // SAFETY: BOOL de 4 bytes, como a documentação pede para este opcode.
        unsafe {
            WlanSetInterface(
                c.0,
                g,
                wlan_intf_opcode_background_scan_enabled,
                std::mem::size_of::<i32>() as u32,
                &valor as *const i32 as *const _,
                std::ptr::null(),
            )
        }
    }

    /// A interface Wi-Fi por onde sai a rota agora, ou por que não há.
    fn interface_da_rota() -> Result<GUID, String> {
        let referencia = std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1));
        let Some(p) = wifi::placa_da_rota(referencia) else {
            return Err("sem rota para a internet agora".into());
        };
        if p.tipo != TipoDePlaca::WiFi {
            return Err("você não está no Wi-Fi: a varredura não pesa no cabo".into());
        }
        let c = Cliente::abrir()
            .map_err(|e| format!("o serviço do Wi-Fi não respondeu (erro {})", e))?;
        c.interface(&p.guid)
            .ok_or_else(|| "a placa não aparece na lista de placas Wi-Fi do Windows".to_string())
    }

    pub fn painel() -> Painel {
        let e = ler();
        let agindo_em = ativo().as_ref().map(|a| a.jogo.clone());
        let observacao = if agindo_em.is_some() {
            None
        } else {
            interface_da_rota().err()
        };
        Painel {
            ligado: e.ligado,
            agindo_em,
            observacao,
        }
    }

    pub fn definir(ligado: bool) -> Result<Painel, String> {
        let mut e = ler();
        e.ligado = ligado;
        gravar(&e)?;
        if !ligado {
            devolver()?;
        } else if let Some(j) = crate::plataforma::sessao::atual() {
            // O vigia só age quando o jogo muda: ligado com ele já aberto, entra agora.
            entrar(&j.nome)?;
        }
        Ok(painel())
    }

    /// Com o jogo aberto e no Wi-Fi: desliga a varredura e segura o cliente aberto. Fora do Wi-Fi ou desconectado:
    /// nada (não é erro).
    pub fn entrar(jogo: &str) -> Result<Option<String>, String> {
        let mut a = ativo();
        if !ler().ligado || a.is_some() {
            return Ok(None);
        }
        let Ok(interface) = interface_da_rota() else {
            return Ok(None);
        };
        let cliente = Cliente::abrir()
            .map_err(|e| format!("o serviço do Wi-Fi não respondeu (erro {})", e))?;
        match varredura(&cliente, &interface, false) {
            0 => {}
            // ERROR_INVALID_STATE: a placa não está conectada; o Windows só aceita desligar conectado.
            5023 => return Ok(None),
            e => {
                return Err(format!(
                    "o Windows recusou desligar a varredura do Wi-Fi (erro {})",
                    e
                ))
            }
        }
        *a = Some(Ativo {
            cliente,
            interface,
            jogo: jogo.to_string(),
        });
        Ok(Some(
            "a placa Wi-Fi não sai mais do canal para procurar redes enquanto o jogo está aberto."
                .to_string(),
        ))
    }

    /// Religa a varredura e fecha o cliente. Chamado quando o jogo fecha e ao desligar.
    pub fn devolver() -> Result<bool, String> {
        let Some(a) = ativo().take() else {
            return Ok(false);
        };
        // Fechar o cliente já basta; religar antes é o pedido explícito. Desconectada, a varredura já voltou.
        let r = varredura(&a.cliente, &a.interface, true);
        drop(a);
        match r {
            0 | 5023 => Ok(true),
            e => Err(format!(
                "o Windows respondeu {} ao religar, mas o cliente foi fechado e a varredura volta",
                e
            )),
        }
    }

    /// Um passo do vigia: desliga com o jogo aberto, devolve com ele fechado.
    pub fn passo(jogo_aberto: Option<&str>) -> Option<String> {
        let aplicado = ativo().is_some();
        match (
            decidir(ler().ligado, jogo_aberto.is_some(), aplicado),
            jogo_aberto,
        ) {
            (Acao::Desligar, Some(jogo)) => match entrar(jogo) {
                Ok(Some(frase)) => Some(format!("Wi-Fi sem varredura durante {}: {}", jogo, frase)),
                Ok(None) => None,
                Err(e) => Some(format!("Wi-Fi sem varredura: {}", e)),
            },
            (Acao::Devolver, _) => match devolver() {
                Ok(_) => {
                    Some("Wi-Fi sem varredura: a busca de redes voltou ao normal.".to_string())
                }
                Err(e) => Some(format!("Wi-Fi sem varredura: {}", e)),
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_desliga_com_a_escolha_ligada_e_o_jogo_aberto() {
        assert_eq!(decidir(true, true, false), Acao::Desligar);
        assert_eq!(decidir(false, true, false), Acao::Nada);
        assert_eq!(decidir(true, false, false), Acao::Nada);
    }

    #[test]
    fn devolve_quando_o_jogo_fecha_ou_a_escolha_e_desligada() {
        assert_eq!(decidir(true, false, true), Acao::Devolver);
        assert_eq!(decidir(false, true, true), Acao::Devolver);
        assert_eq!(
            decidir(true, true, true),
            Acao::Nada,
            "já aplicado: não repete"
        );
    }

    #[test]
    fn sem_escolha_gravada_vem_desligada() {
        assert!(!Estado::default().ligado);
        let e: Estado = serde_json::from_str(r#"{"ligado":true}"#).unwrap();
        assert!(e.ligado);
    }
}

#[cfg(all(test, windows))]
mod na_maquina {
    /// Só lê: `cargo test --lib -- --ignored wifi_varredura::na_maquina`.
    #[test]
    #[ignore]
    fn painel_real_sem_escrever() {
        println!("{:#?}", super::painel());
    }
}
