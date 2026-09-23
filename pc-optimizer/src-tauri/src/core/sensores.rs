// O que os sensores da placa dizem sobre UMA partida
//
// A leitura mora em `modules::windows::nvml` (que carrega a `nvml.dll`);
// aqui fica só a conta, pura e testável sem placa nenhuma: juntar as amostras
// da janela medida e dizer o que segurou a placa, se é que algo segurou.
//
// A separação é a mesma do resto do núcleo: quem toca a máquina fica em
// `modules/windows`, quem decide fica aqui — e a decisão tem teste que roda
// em qualquer máquina, inclusive sem NVIDIA.

use serde::{Deserialize, Serialize};

/// Bits de motivo que a NVML publica (`nvmlClocksThrottleReason*`). Ficam
/// aqui porque é esta conta que os interpreta.
pub const MOTIVO_TETO_DE_ENERGIA: u64 = 0x0000_0004;
pub const MOTIVO_FREIO_DE_HARDWARE: u64 = 0x0000_0008;
pub const MOTIVO_TERMICO_SW: u64 = 0x0000_0020;
pub const MOTIVO_TERMICO_HW: u64 = 0x0000_0040;
pub const MOTIVO_FREIO_DE_ENERGIA_HW: u64 = 0x0000_0080;

/// Uma leitura instantânea. Campo que não veio é `None` — nunca zero.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AmostraGpu {
    pub temperatura_c: Option<u32>,
    pub clock_mhz: Option<u32>,
    pub potencia_w: Option<f64>,
    pub uso_pct: Option<u32>,
    /// Bits crus do motivo do clock estar segurado, como a NVML publica.
    pub motivos: Option<u64>,
}

// ─────────────────────────────── o resumo da janela (puro) ───────────────

/// O que a placa fez durante a partida medida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumoGpu {
    pub amostras: usize,
    pub temperatura_max_c: Option<u32>,
    pub temperatura_media_c: Option<f64>,
    pub clock_medio_mhz: Option<f64>,
    pub potencia_media_w: Option<f64>,
    pub limite_w: Option<f64>,
    pub uso_medio_pct: Option<f64>,
    /// Percentual do tempo medido em que o driver disse estar segurando o
    /// clock por CADA motivo.
    pub pct_termico: f64,
    pub pct_teto_de_energia: f64,
    pub pct_freio_de_hardware: f64,
    /// Falso quando o driver não publica os motivos (placa ou driver antigo).
    pub motivos_lidos: bool,
}

/// A partir de quanto tempo segurado por temperatura isso vira um achado.
///
/// Cinco por cento da partida. Abaixo disso é o pico de um instante — que toda
/// placa tem — e acusar seria mandar a pessoa limpar um cooler que está bom.
pub const TERMICO_ALTO_PCT: f64 = 5.0;

/// A partir de quanto o teto de energia deixa de ser normal e vira achado.
///
/// Placa em carga máxima batendo no teto de energia é o funcionamento normal
/// dela — o boost sobe até o limite e para ali. Só vira conversa quando é a
/// maior parte da partida E o clock está bem abaixo do que a placa faz.
pub const TETO_DOMINANTE_PCT: f64 = 60.0;

/// **Pura.** Junta as amostras da janela medida.
pub fn resumir(amostras: &[AmostraGpu], limite_w: Option<f64>) -> Option<ResumoGpu> {
    if amostras.is_empty() {
        return None;
    }
    let media = |v: Vec<f64>| (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64);
    let com_motivo = amostras.iter().filter_map(|a| a.motivos).count();
    let pct = |bits: u64| {
        if com_motivo == 0 {
            return 0.0;
        }
        let quantas = amostras.iter().filter_map(|a| a.motivos).filter(|m| m & bits != 0).count();
        quantas as f64 / com_motivo as f64 * 100.0
    };

    Some(ResumoGpu {
        amostras: amostras.len(),
        temperatura_max_c: amostras.iter().filter_map(|a| a.temperatura_c).max(),
        temperatura_media_c: media(amostras.iter().filter_map(|a| a.temperatura_c).map(f64::from).collect()),
        clock_medio_mhz: media(amostras.iter().filter_map(|a| a.clock_mhz).map(f64::from).collect()),
        potencia_media_w: media(amostras.iter().filter_map(|a| a.potencia_w).collect()),
        limite_w,
        uso_medio_pct: media(amostras.iter().filter_map(|a| a.uso_pct).map(f64::from).collect()),
        pct_termico: pct(MOTIVO_TERMICO_SW | MOTIVO_TERMICO_HW),
        pct_teto_de_energia: pct(MOTIVO_TETO_DE_ENERGIA),
        pct_freio_de_hardware: pct(MOTIVO_FREIO_DE_HARDWARE | MOTIVO_FREIO_DE_ENERGIA_HW),
        motivos_lidos: com_motivo > 0,
    })
}

/// O que dizer sobre a placa nesta partida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum LimiteDaPlaca {
    /// O driver segurou o clock por temperatura em parte relevante da partida.
    Temperatura { pct: f64, temperatura_max_c: Option<u32> },
    /// Freio de hardware: fonte, conector de energia ou proteção da placa.
    FreioDeHardware { pct: f64 },
    /// No teto de energia a maior parte do tempo. É o normal de uma placa em
    /// carga — aparece como informação, não como defeito.
    TetoDeEnergia { pct: f64 },
    /// Nada segurou a placa além do normal.
    Livre,
    /// O driver não publica os motivos nesta máquina.
    NaoDeuParaLer,
}

/// **Pura.** A ordem importa: temperatura e freio são problema; teto de
/// energia é informação, e só quando dominou a partida.
pub fn julgar(r: &ResumoGpu) -> LimiteDaPlaca {
    if !r.motivos_lidos {
        return LimiteDaPlaca::NaoDeuParaLer;
    }
    if r.pct_termico >= TERMICO_ALTO_PCT {
        return LimiteDaPlaca::Temperatura { pct: r.pct_termico, temperatura_max_c: r.temperatura_max_c };
    }
    if r.pct_freio_de_hardware >= TERMICO_ALTO_PCT {
        return LimiteDaPlaca::FreioDeHardware { pct: r.pct_freio_de_hardware };
    }
    if r.pct_teto_de_energia >= TETO_DOMINANTE_PCT {
        return LimiteDaPlaca::TetoDeEnergia { pct: r.pct_teto_de_energia };
    }
    LimiteDaPlaca::Livre
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(temp: u32, clock: u32, motivos: u64) -> AmostraGpu {
        AmostraGpu {
            temperatura_c: Some(temp),
            clock_mhz: Some(clock),
            potencia_w: Some(60.0),
            uso_pct: Some(98),
            motivos: Some(motivos),
        }
    }

    #[test]
    fn sem_amostra_nao_ha_resumo() {
        assert!(resumir(&[], Some(75.0)).is_none());
    }

    #[test]
    fn junta_maximo_e_media() {
        let r = resumir(&[a(60, 1800, 0), a(72, 1700, 0)], Some(75.0)).unwrap();
        assert_eq!(r.temperatura_max_c, Some(72));
        assert_eq!(r.temperatura_media_c, Some(66.0));
        assert_eq!(r.clock_medio_mhz, Some(1750.0));
        assert_eq!(r.limite_w, Some(75.0));
        assert_eq!(r.pct_termico, 0.0);
        assert!(r.motivos_lidos);
    }

    #[test]
    fn conta_o_tempo_de_cada_motivo() {
        // 4 amostras: 1 térmica, 3 no teto de energia.
        let r = resumir(
            &[a(80, 1500, MOTIVO_TERMICO_HW), a(70, 1800, MOTIVO_TETO_DE_ENERGIA), a(70, 1800, MOTIVO_TETO_DE_ENERGIA), a(70, 1800, MOTIVO_TETO_DE_ENERGIA)],
            None,
        )
        .unwrap();
        assert_eq!(r.pct_termico, 25.0);
        assert_eq!(r.pct_teto_de_energia, 75.0);
    }

    #[test]
    fn temperatura_vence_teto_de_energia() {
        let r = resumir(
            &[a(83, 1400, MOTIVO_TERMICO_SW | MOTIVO_TETO_DE_ENERGIA), a(70, 1800, MOTIVO_TETO_DE_ENERGIA)],
            None,
        )
        .unwrap();
        assert_eq!(julgar(&r), LimiteDaPlaca::Temperatura { pct: 50.0, temperatura_max_c: Some(83) });
    }

    #[test]
    fn pico_termico_de_um_instante_nao_vira_achado() {
        // 1 em 50 amostras = 2%, abaixo do piso: toda placa tem esse pico.
        let mut amostras = vec![a(70, 1800, 0); 49];
        amostras.push(a(79, 1700, MOTIVO_TERMICO_HW));
        let r = resumir(&amostras, None).unwrap();
        assert!(r.pct_termico < TERMICO_ALTO_PCT);
        assert_eq!(julgar(&r), LimiteDaPlaca::Livre);
    }

    #[test]
    fn teto_de_energia_so_aparece_quando_domina() {
        let poucas = resumir(&[a(70, 1800, MOTIVO_TETO_DE_ENERGIA), a(70, 1800, 0), a(70, 1800, 0)], None).unwrap();
        assert_eq!(julgar(&poucas), LimiteDaPlaca::Livre);

        let muitas = resumir(&[a(70, 1800, MOTIVO_TETO_DE_ENERGIA); 4], None).unwrap();
        assert_eq!(julgar(&muitas), LimiteDaPlaca::TetoDeEnergia { pct: 100.0 });
    }

    #[test]
    fn driver_que_nao_publica_motivo_nao_vira_placa_livre() {
        let sem = AmostraGpu { temperatura_c: Some(70), clock_mhz: Some(1800), potencia_w: None, uso_pct: None, motivos: None };
        let r = resumir(&[sem, sem], None).unwrap();
        assert!(!r.motivos_lidos);
        assert_eq!(julgar(&r), LimiteDaPlaca::NaoDeuParaLer);
        // O que deu para ler continua valendo.
        assert_eq!(r.temperatura_max_c, Some(70));
    }
}

