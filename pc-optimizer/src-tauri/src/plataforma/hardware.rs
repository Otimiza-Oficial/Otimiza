// O retrato do PC numa peça só, para a tela e para a compatibilidade. Não lê nada de novo: junta o que
// `modules::windows::{hardware, framegen, display, vrr}` já sabem ler. Ler custa (PowerShell e WMI, centenas de
// milissegundos), então fica em memória por 10 minutos e em disco para a abertura seguinte mostrar algo na hora.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const VALIDADE_EM_MEMORIA: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fabricante {
    Nvidia,
    Amd,
    Intel,
    Desconhecido,
}

impl Fabricante {
    pub fn nome(self) -> &'static str {
        match self {
            Fabricante::Nvidia => "NVIDIA",
            Fabricante::Amd => "AMD",
            Fabricante::Intel => "Intel",
            Fabricante::Desconhecido => "placa não identificada",
        }
    }
}

/// A família decide o que o driver oferece (Smooth Motion só em RTX 40/50, AFMF só em RX 6000 em diante).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Familia {
    Rtx50,
    Rtx40,
    Rtx30,
    Rtx20,
    GtxOuAnterior,
    Rx9000,
    Rx7000,
    Rx6000,
    RxAnterior,
    Arc,
    IntelIntegrada,
    Desconhecida,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placa {
    pub nome: String,
    pub fabricante: Fabricante,
    pub familia: Familia,
    pub driver: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vrr {
    Anunciado,
    Provavel,
    NaoAnunciado,
    Desconhecido,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tela {
    pub nome: String,
    pub dispositivo: String,
    pub principal: bool,
    pub largura: u32,
    pub altura: u32,
    pub hz_atual: u32,
    pub hz_maximo: u32,
    pub vrr: Vrr,
}

impl Tela {
    /// O monitor aguenta mais do que o Windows está usando: a oportunidade mais comum e mais barata de corrigir.
    pub fn abaixo_do_maximo(&self) -> bool {
        self.hz_maximo > self.hz_atual + 5
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerfilDeHardware {
    pub processador: String,
    pub nucleos_logicos: usize,
    pub memoria_gb: f64,
    pub disco_do_sistema: String,
    pub placa: Placa,
    pub telas: Vec<Tela>,
    pub windows_build: Option<u32>,
    /// Segundos desde 1970 da leitura. A tela diz "lido há X" em vez de fingir que é ao vivo.
    pub lido_em: u64,
}

impl PerfilDeHardware {
    pub fn tela_principal(&self) -> Option<&Tela> {
        self.telas
            .iter()
            .find(|t| t.principal)
            .or_else(|| self.telas.first())
    }
}

/// Junta o VRR (lido do EDID, com o nome gravado no monitor) às telas ligadas. Pelo nome quando dá; com um monitor
/// de cada lado, o par é óbvio; o resto fica desconhecido em vez de chutado.
pub fn juntar_vrr(telas: &mut [Tela], vrr: &[(Option<String>, Vrr)]) {
    for tela in telas.iter_mut() {
        let nome = tela.nome.to_lowercase();
        let achado = vrr.iter().find(|(n, _)| {
            n.as_deref()
                .map(|n| {
                    let n = n.trim().to_lowercase();
                    !n.is_empty() && (nome.contains(&n) || n.contains(&nome))
                })
                .unwrap_or(false)
        });
        if let Some((_, v)) = achado {
            tela.vrr = *v;
        }
    }

    if telas.len() == 1 && vrr.len() == 1 && telas[0].vrr == Vrr::Desconhecido {
        telas[0].vrr = vrr[0].1;
    }
}

/// Versão do driver NVIDIA como a pessoa conhece ("581.42") a partir da do Windows ("32.0.15.8142"): os cinco últimos
/// dígitos. As outras marcas mostram a do Windows mesmo.
pub fn versao_amigavel(fabricante: Fabricante, driver: &str) -> String {
    if fabricante != Fabricante::Nvidia {
        return driver.to_string();
    }

    let digitos: String = driver.chars().filter(|c| c.is_ascii_digit()).collect();
    if digitos.len() < 5 {
        return driver.to_string();
    }

    let ultimos = &digitos[digitos.len() - 5..];
    format!("{}.{}", &ultimos[..3], &ultimos[3..])
}

fn agora() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(target_os = "windows")]
fn ler_agora() -> PerfilDeHardware {
    use crate::modules::windows::{display, framegen, hardware, vrr};

    let base = hardware::profile();
    let p = framegen::ler_placa();

    let fabricante = match p.fabricante {
        framegen::Fabricante::Nvidia => Fabricante::Nvidia,
        framegen::Fabricante::Amd => Fabricante::Amd,
        framegen::Fabricante::Intel => Fabricante::Intel,
        framegen::Fabricante::Desconhecido => Fabricante::Desconhecido,
    };
    let familia = match p.arquitetura {
        framegen::Arquitetura::Rtx50 => Familia::Rtx50,
        framegen::Arquitetura::Rtx40 => Familia::Rtx40,
        framegen::Arquitetura::Rtx30 => Familia::Rtx30,
        framegen::Arquitetura::Rtx20 => Familia::Rtx20,
        framegen::Arquitetura::GtxOuAnterior => Familia::GtxOuAnterior,
        framegen::Arquitetura::Rx9000 => Familia::Rx9000,
        framegen::Arquitetura::Rx7000 => Familia::Rx7000,
        framegen::Arquitetura::Rx6000 => Familia::Rx6000,
        framegen::Arquitetura::RxAnterior => Familia::RxAnterior,
        framegen::Arquitetura::IntelArc => Familia::Arc,
        framegen::Arquitetura::IntelIntegrada => Familia::IntelIntegrada,
        framegen::Arquitetura::Desconhecida => Familia::Desconhecida,
    };

    let mut telas: Vec<Tela> = display::monitores()
        .into_iter()
        .map(|m| Tela {
            nome: m.descricao.clone(),
            dispositivo: m.dispositivo.clone(),
            principal: m.principal,
            largura: m.largura,
            altura: m.altura,
            hz_atual: m.hz_atual,
            hz_maximo: m.hz_disponiveis.iter().copied().max().unwrap_or(m.hz_atual),
            vrr: Vrr::Desconhecido,
        })
        .collect();

    let edids: Vec<(Option<String>, Vrr)> = vrr::monitores()
        .into_iter()
        .map(|m| {
            let v = match m.vrr {
                vrr::Vrr::Anunciado => Vrr::Anunciado,
                vrr::Vrr::Provavel => Vrr::Provavel,
                vrr::Vrr::NaoAnunciado => Vrr::NaoAnunciado,
            };
            (m.nome, v)
        })
        .collect();
    juntar_vrr(&mut telas, &edids);

    let windows_build = crate::modules::windows::registry::read_text(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuildNumber",
    )
    .ok()
    .flatten()
    .and_then(|b| b.trim().parse().ok());

    PerfilDeHardware {
        processador: base.cpu_name.clone(),
        nucleos_logicos: base.logical_cores,
        memoria_gb: (base.total_ram_gb * 10.0).round() / 10.0,
        disco_do_sistema: match base.system_storage {
            hardware::StorageKind::Ssd => "SSD",
            hardware::StorageKind::Hdd => "HD",
            hardware::StorageKind::Unknown => "não identificado",
        }
        .to_string(),
        placa: Placa {
            driver: p.driver.as_deref().map(|d| versao_amigavel(fabricante, d)),
            nome: p.nome,
            fabricante,
            familia,
        },
        telas,
        windows_build,
        lido_em: agora(),
    }
}

#[cfg(not(target_os = "windows"))]
fn ler_agora() -> PerfilDeHardware {
    PerfilDeHardware {
        processador: String::new(),
        nucleos_logicos: num_cpus::get(),
        memoria_gb: 0.0,
        disco_do_sistema: "não identificado".to_string(),
        placa: Placa {
            nome: String::new(),
            fabricante: Fabricante::Desconhecido,
            familia: Familia::Desconhecida,
            driver: None,
        },
        telas: Vec::new(),
        windows_build: None,
        lido_em: agora(),
    }
}

static MEMORIA: Mutex<Option<(Instant, PerfilDeHardware)>> = Mutex::new(None);

fn caminho() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("pc-optimizer").join("hardware.json")
}

fn ler_disco(c: &Path) -> Option<PerfilDeHardware> {
    serde_json::from_str(&fs::read_to_string(c).ok()?).ok()
}

fn gravar_disco(c: &Path, p: &PerfilDeHardware) {
    let Ok(json) = serde_json::to_string_pretty(p) else {
        return;
    };
    if let Some(pasta) = c.parent() {
        let _ = fs::create_dir_all(pasta);
    }
    let temporario = crate::modules::changelog::caminho_temporario(c);
    if fs::write(&temporario, json).is_ok() {
        let _ = fs::rename(&temporario, c);
    }
}

/// Lê de novo, sempre. Para "Atualizar", troca de monitor ou de driver.
pub fn atualizar() -> PerfilDeHardware {
    let novo = ler_agora();
    gravar_disco(&caminho(), &novo);
    if let Ok(mut m) = MEMORIA.lock() {
        *m = Some((Instant::now(), novo.clone()));
    }
    novo
}

/// O de memória se tiver menos de 10 minutos; senão lê.
pub fn perfil() -> PerfilDeHardware {
    if let Ok(m) = MEMORIA.lock() {
        if let Some((quando, p)) = m.as_ref() {
            if quando.elapsed() < VALIDADE_EM_MEMORIA {
                return p.clone();
            }
        }
    }
    atualizar()
}

/// Para a abertura: o último gravado, sem esperar o PowerShell. `None` na primeira vez da vida.
pub fn ultimo_conhecido() -> Option<PerfilDeHardware> {
    if let Ok(m) = MEMORIA.lock() {
        if let Some((_, p)) = m.as_ref() {
            return Some(p.clone());
        }
    }
    ler_disco(&caminho())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tela(nome: &str) -> Tela {
        Tela {
            nome: nome.to_string(),
            dispositivo: r"\\.\DISPLAY1".to_string(),
            principal: true,
            largura: 1920,
            altura: 1080,
            hz_atual: 60,
            hz_maximo: 144,
            vrr: Vrr::Desconhecido,
        }
    }

    #[test]
    fn versao_do_driver_nvidia_sai_como_a_pessoa_conhece() {
        assert_eq!(
            versao_amigavel(Fabricante::Nvidia, "32.0.15.8142"),
            "581.42"
        );
        assert_eq!(
            versao_amigavel(Fabricante::Nvidia, "31.0.15.5222"),
            "552.22"
        );
        assert_eq!(
            versao_amigavel(Fabricante::Amd, "32.0.21025.1001"),
            "32.0.21025.1001"
        );
        assert_eq!(versao_amigavel(Fabricante::Nvidia, "abc"), "abc");
    }

    #[test]
    fn vrr_casa_pelo_nome_do_monitor() {
        let mut telas = vec![tela("LG ULTRAGEAR (27GN800)"), tela("Dell P2419H")];
        juntar_vrr(
            &mut telas,
            &[
                (Some("LG ULTRAGEAR".to_string()), Vrr::Anunciado),
                (None, Vrr::NaoAnunciado),
            ],
        );

        assert_eq!(telas[0].vrr, Vrr::Anunciado);
        assert_eq!(telas[1].vrr, Vrr::Desconhecido, "sem nome não se chuta");
    }

    #[test]
    fn um_monitor_de_cada_lado_e_o_mesmo() {
        let mut telas = vec![tela("Monitor PnP genérico")];
        juntar_vrr(&mut telas, &[(Some("VG27A".to_string()), Vrr::Provavel)]);
        assert_eq!(telas[0].vrr, Vrr::Provavel);
    }

    #[test]
    fn monitor_abaixo_do_maximo_e_oportunidade() {
        assert!(tela("x").abaixo_do_maximo());
        let mut cheio = tela("x");
        cheio.hz_atual = 144;
        assert!(!cheio.abaixo_do_maximo());
        cheio.hz_atual = 143;
        assert!(
            !cheio.abaixo_do_maximo(),
            "143,9 Hz contado como 143 não é problema"
        );
    }

    #[test]
    fn disco_corrompido_nao_vira_perfil() {
        let dir = std::env::temp_dir().join("otimiza-hw-corrompido");
        let _ = fs::create_dir_all(&dir);
        let c = dir.join("hardware.json");
        fs::write(&c, "{ruim").unwrap();
        assert!(ler_disco(&c).is_none());
    }

    #[test]
    fn gravar_e_ler_o_perfil_devolve_o_mesmo() {
        let dir = std::env::temp_dir().join("otimiza-hw-ida-e-volta");
        let _ = fs::remove_dir_all(&dir);
        let c = dir.join("hardware.json");
        let p = PerfilDeHardware {
            processador: "Ryzen 5 5600".to_string(),
            nucleos_logicos: 12,
            memoria_gb: 16.0,
            disco_do_sistema: "SSD".to_string(),
            placa: Placa {
                nome: "RX 6600".to_string(),
                fabricante: Fabricante::Amd,
                familia: Familia::Rx6000,
                driver: None,
            },
            telas: vec![tela("x")],
            windows_build: Some(26100),
            lido_em: 1,
        };
        gravar_disco(&c, &p);
        assert_eq!(ler_disco(&c), Some(p));
    }

    #[test]
    #[ignore = "lê o hardware desta máquina"]
    fn mostra_esta_maquina() {
        let p = atualizar();
        println!("{:#?}", p);
        assert!(p.nucleos_logicos > 0);
    }
}
