// Perfil do PC na primeira abertura (MASTER-PLAN, item 10): antes de otimizar qualquer coisa, dizer o que ESTA máquina
// pode esperar, com o número de cada leitura. Nada de porcentagem de ganho: ninguém mediu nada ainda. Só regras sobre
// o hardware, e a primeira delas é a que mais engana o cliente: FPS acima da taxa do monitor não aparece na tela.
//
// Só lê. Leitura que falhou não vira limitação nem "tudo certo": fica de fora e aparece em `nao_lido`.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Disco {
    Ssd,
    Hd,
    Desconhecido,
}

/// O que foi lido da máquina. `None`: não deu para ler.
#[derive(Debug, Clone, PartialEq)]
pub struct Entrada {
    pub ram_gb: Option<f64>,
    pub nucleos_logicos: Option<usize>,
    pub vram_gb: Option<f64>,
    pub disco_do_sistema: Disco,
    pub notebook: Option<bool>,
    /// Do monitor principal.
    pub monitor_hz: Option<u32>,
    pub monitor_hz_maximo: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo")]
pub enum Limitacao {
    /// O monitor está abaixo do que aceita: é FPS visível de graça, o Otimiza ajusta (Sistema → Monitor).
    MonitorAbaixoDoMaximo { atual: u32, maximo: u32 },
    /// Monitor de até 75 Hz: FPS acima disso não aparece na tela.
    MonitorLimitaATela { hz: u32 },
    PoucaMemoria { gb: f64 },
    PoucosNucleos { nucleos: usize },
    PoucaMemoriaDeVideo { gb: f64 },
    /// Só vídeo integrado (menos de 1 GB dedicado): a placa usa a memória do sistema.
    VideoIntegrado,
    SistemaEmHd,
    /// Notebook: energia e calor limitam, e na bateria o Windows corta desempenho.
    Notebook,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Perfil {
    /// Da mais importante para a menos.
    pub limitacoes: Vec<Limitacao>,
    /// O que não deu para ler, em nome curto ("memória de vídeo", "monitor"...).
    pub nao_lido: Vec<String>,
}

/// Abaixo disso o FiveM e jogos atuais trocam memória com o disco em partida.
pub const MEMORIA_MINIMA_GB: f64 = 12.0;
pub const NUCLEOS_MINIMOS: usize = 6;
pub const VRAM_MINIMA_GB: f64 = 4.0;
/// Abaixo disso de memória DEDICADA é vídeo integrado, e o número não diz o que o jogo tem.
pub const VRAM_DE_PLACA_DEDICADA_GB: f64 = 1.0;
pub const MONITOR_QUE_LIMITA_HZ: u32 = 75;

/// **Pura.**
pub fn montar(e: &Entrada) -> Perfil {
    let mut limitacoes = Vec::new();
    let mut nao_lido = Vec::new();

    match (e.monitor_hz, e.monitor_hz_maximo) {
        // O mesmo limiar da sugestão de taxa do monitor (`display`): 60 → 75 não vale o aviso.
        (Some(atual), Some(maximo)) if maximo >= atual + crate::modules::windows::display::DIFERENCA_QUE_IMPORTA => {
            limitacoes.push(Limitacao::MonitorAbaixoDoMaximo { atual, maximo })
        }
        (Some(hz), _) if hz <= MONITOR_QUE_LIMITA_HZ => limitacoes.push(Limitacao::MonitorLimitaATela { hz }),
        (Some(_), _) => {}
        (None, _) => nao_lido.push("monitor".to_string()),
    }
    match e.ram_gb {
        // Arredonda para o que o cliente conhece: 7,9 GB utilizáveis são "8 GB".
        Some(gb) if gb.round() < MEMORIA_MINIMA_GB => limitacoes.push(Limitacao::PoucaMemoria { gb: gb.round() }),
        Some(_) => {}
        None => nao_lido.push("memória".to_string()),
    }
    match e.nucleos_logicos {
        Some(n) if n < NUCLEOS_MINIMOS => limitacoes.push(Limitacao::PoucosNucleos { nucleos: n }),
        Some(_) => {}
        None => nao_lido.push("processador".to_string()),
    }
    match e.vram_gb {
        Some(gb) if gb < VRAM_DE_PLACA_DEDICADA_GB => limitacoes.push(Limitacao::VideoIntegrado),
        // Arredondada: 3,9 GB utilizáveis de uma placa de 4 GB são "4 GB".
        Some(gb) if gb.round() < VRAM_MINIMA_GB => limitacoes.push(Limitacao::PoucaMemoriaDeVideo { gb: gb.round() }),
        Some(_) => {}
        None => nao_lido.push("memória de vídeo".to_string()),
    }
    match e.disco_do_sistema {
        Disco::Hd => limitacoes.push(Limitacao::SistemaEmHd),
        Disco::Ssd => {}
        Disco::Desconhecido => nao_lido.push("disco".to_string()),
    }
    match e.notebook {
        Some(true) => limitacoes.push(Limitacao::Notebook),
        Some(false) => {}
        None => nao_lido.push("tipo de computador".to_string()),
    }
    Perfil { limitacoes, nao_lido }
}

/// Lê a máquina: memória, núcleos e disco (já lidos pelo Otimiza), memória de vídeo, bateria e o monitor principal.
#[cfg(target_os = "windows")]
pub fn ler() -> Entrada {
    use crate::modules::windows::{display, hardware};
    let h = hardware::profile();
    let principal = display::monitores_sem_nomes().into_iter().find(|m| m.principal);
    Entrada {
        ram_gb: (h.total_ram_gb > 0.0).then_some(h.total_ram_gb),
        nucleos_logicos: (h.logical_cores > 0).then_some(h.logical_cores),
        vram_gb: crate::core::telemetria::memoria_da_placa_principal_gb(),
        disco_do_sistema: match h.system_storage {
            hardware::StorageKind::Ssd => Disco::Ssd,
            hardware::StorageKind::Hdd => Disco::Hd,
            hardware::StorageKind::Unknown => Disco::Desconhecido,
        },
        notebook: tem_tampa(),
        monitor_hz: principal.as_ref().map(|m| m.hz_atual).filter(|hz| *hz > 1),
        monitor_hz_maximo: principal.as_ref().map(|m| m.hz_maximo()).filter(|hz| *hz > 1),
    }
}

/// Notebook pela TAMPA (`LidPresent`), não pela bateria: nobreak USB aparece ao Windows como bateria do sistema, e
/// notebook sem a bateria encaixada não tem nenhuma. Chamada que falha é "não sei".
#[cfg(target_os = "windows")]
fn tem_tampa() -> Option<bool> {
    use windows_sys::Win32::System::Power::{GetPwrCapabilities, SYSTEM_POWER_CAPABILITIES};
    let mut c: SYSTEM_POWER_CAPABILITIES = unsafe { std::mem::zeroed() };
    if unsafe { GetPwrCapabilities(&mut c) } == 0 {
        return None;
    }
    Some(c.LidPresent != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pc_bom() -> Entrada {
        Entrada {
            ram_gb: Some(31.9),
            nucleos_logicos: Some(12),
            vram_gb: Some(8.0),
            disco_do_sistema: Disco::Ssd,
            notebook: Some(false),
            monitor_hz: Some(180),
            monitor_hz_maximo: Some(180),
        }
    }

    #[test]
    fn pc_bom_nao_tem_limitacao_nem_leitura_faltando() {
        assert_eq!(montar(&pc_bom()), Perfil { limitacoes: vec![], nao_lido: vec![] });
    }

    #[test]
    fn monitor_abaixo_do_que_aceita_vem_primeiro() {
        let p = montar(&Entrada { monitor_hz: Some(60), monitor_hz_maximo: Some(144), ram_gb: Some(7.9), ..pc_bom() });
        assert_eq!(p.limitacoes[0], Limitacao::MonitorAbaixoDoMaximo { atual: 60, maximo: 144 });
        assert_eq!(p.limitacoes[1], Limitacao::PoucaMemoria { gb: 8.0 });
    }

    #[test]
    fn diferenca_pequena_de_taxa_nao_vira_aviso() {
        let p = montar(&Entrada { monitor_hz: Some(165), monitor_hz_maximo: Some(170), ..pc_bom() });
        assert!(p.limitacoes.is_empty());
    }

    #[test]
    fn monitor_de_60_no_maximo_limita_a_tela() {
        let p = montar(&Entrada { monitor_hz: Some(60), monitor_hz_maximo: Some(60), ..pc_bom() });
        assert_eq!(p.limitacoes, vec![Limitacao::MonitorLimitaATela { hz: 60 }]);
    }

    #[test]
    fn o_que_nao_se_leu_nao_vira_limitacao_nem_tudo_certo() {
        let p = montar(&Entrada {
            ram_gb: None,
            vram_gb: None,
            disco_do_sistema: Disco::Desconhecido,
            notebook: None,
            monitor_hz: None,
            monitor_hz_maximo: None,
            ..pc_bom()
        });
        assert!(p.limitacoes.is_empty());
        assert_eq!(p.nao_lido.len(), 5);
    }

    #[test]
    fn video_integrado_nao_vira_numero_de_memoria() {
        let p = montar(&Entrada { vram_gb: Some(0.5), ..pc_bom() });
        assert_eq!(p.limitacoes, vec![Limitacao::VideoIntegrado]);
        let quase_4 = montar(&Entrada { vram_gb: Some(3.9), ..pc_bom() });
        assert!(quase_4.limitacoes.is_empty(), "3,9 GB utilizáveis de uma placa de 4 GB não é pouca memória");
    }

    #[test]
    fn maquina_de_entrada_junta_as_limitacoes() {
        let p = montar(&Entrada {
            ram_gb: Some(8.0),
            nucleos_logicos: Some(4),
            vram_gb: Some(2.0),
            disco_do_sistema: Disco::Hd,
            notebook: Some(true),
            ..pc_bom()
        });
        assert_eq!(p.limitacoes.len(), 5);
    }

    /// Laboratório: o perfil DESTA máquina. `cargo test --release --lib perfil_real -- --ignored --nocapture`
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "laboratório: lê esta máquina"]
    fn perfil_real() {
        let e = ler();
        println!("{:?}", e);
        println!("{:#?}", montar(&e));
    }
}
