// Sensores da placa de vídeo NVIDIA (2.9)
//
// Fecha o "limitado por temperatura" do lado da placa. O processador já tinha
// o `thermal.rs`; a placa não tinha nada, e é ela que esquenta primeiro num PC
// de jogo com pouco fluxo de ar.
//
// POR QUE `nvidia-smi`, E NÃO A NVML DIRETO
//
// O `nvidia-smi` é o cliente oficial da NVML, vem junto com o driver e mora em
// `System32` — o `pcie.rs` já o usa. Ele devolve o MOTIVO de a placa estar
// segurando o clock, do jeito que o próprio driver decide: limite térmico por
// software, desaceleração térmica de hardware, teto de energia e freio de
// hardware. Não é dependência nova e não carrega DLL nenhuma no processo.
//
// A REGRA DE HONESTIDADE DESTE MÓDULO
//
// Placa em carga máxima batendo no teto de energia é o COMPORTAMENTO NORMAL de
// qualquer placa moderna: o boost sobe até o limite de potência e para ali.
// Chamar isso de problema faria a pessoa trocar fonte à toa. Só temperatura e
// freio de hardware viram alerta; teto de energia é informação.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeituraGpu {
    pub nome: String,
    pub temperatura_c: Option<f64>,
    pub clock_mhz: Option<f64>,
    pub clock_max_mhz: Option<f64>,
    pub potencia_w: Option<f64>,
    pub limite_potencia_w: Option<f64>,
    pub uso_pct: Option<f64>,
    pub termico: bool,
    pub teto_de_energia: bool,
    pub freio_de_hardware: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado")]
pub enum EstadoGpu {
    /// O driver está segurando o clock por temperatura.
    Temperatura,
    /// Freio de hardware: sinal da fonte, do conector de energia ou proteção
    /// térmica da própria placa.
    FreioDeHardware,
    /// Em carga, no teto de energia: normal.
    TetoDeEnergiaNormal,
    /// Em carga e nada segurando.
    Livre,
    /// Sem carga agora: os limites só aparecem com jogo aberto.
    SemCarga,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum SensoresGpu {
    Lido { leitura: LeituraGpu, estado: EstadoGpu },
    NaoDeuParaLer { motivo: String },
}

const CAMPOS: &str = "name,temperature.gpu,clocks.gr,clocks.max.gr,power.draw,power.limit,utilization.gpu,\
clocks_throttle_reasons.hw_thermal_slowdown,clocks_throttle_reasons.sw_thermal_slowdown,\
clocks_throttle_reasons.sw_power_cap,clocks_throttle_reasons.hw_slowdown,\
clocks_throttle_reasons.hw_power_brake_slowdown";

fn numero(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn ativo(s: &str) -> bool {
    s.trim().eq_ignore_ascii_case("active")
}

/// **Pura.** A primeira placa da saída CSV do `nvidia-smi`.
pub fn ler_linha(saida: &str) -> Option<LeituraGpu> {
    let linha = saida.lines().find(|l| !l.trim().is_empty())?;
    let c: Vec<&str> = linha.split(',').collect();
    if c.len() < 12 {
        return None;
    }
    Some(LeituraGpu {
        nome: c[0].trim().to_string(),
        temperatura_c: numero(c[1]),
        clock_mhz: numero(c[2]),
        clock_max_mhz: numero(c[3]),
        potencia_w: numero(c[4]),
        limite_potencia_w: numero(c[5]),
        uso_pct: numero(c[6]),
        termico: ativo(c[7]) || ativo(c[8]),
        teto_de_energia: ativo(c[9]),
        freio_de_hardware: ativo(c[10]) || ativo(c[11]),
    })
}

/// **Pura.** Temperatura e freio valem com ou sem carga — são o driver dizendo
/// que segurou. O resto só se julga com a placa trabalhando.
pub fn julgar(l: &LeituraGpu) -> EstadoGpu {
    if l.termico {
        return EstadoGpu::Temperatura;
    }
    if l.freio_de_hardware {
        return EstadoGpu::FreioDeHardware;
    }
    match l.uso_pct {
        Some(u) if u >= 50.0 => {
            if l.teto_de_energia {
                EstadoGpu::TetoDeEnergiaNormal
            } else {
                EstadoGpu::Livre
            }
        }
        _ => EstadoGpu::SemCarga,
    }
}

#[cfg(target_os = "windows")]
pub fn ler() -> SensoresGpu {
    let saida = super::shell::run("nvidia-smi", &[&format!("--query-gpu={CAMPOS}"), "--format=csv,noheader,nounits"]);
    match saida {
        Ok(s) if s.success => match ler_linha(&s.stdout) {
            Some(leitura) => {
                let estado = julgar(&leitura);
                SensoresGpu::Lido { leitura, estado }
            }
            None => SensoresGpu::NaoDeuParaLer { motivo: "o driver da NVIDIA respondeu, mas sem os sensores esperados.".into() },
        },
        _ => SensoresGpu::NaoDeuParaLer {
            motivo: "a leitura usa o nvidia-smi, que vem com o driver da NVIDIA e não respondeu aqui. Em placa AMD ou Intel ainda não há leitura.".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Saída real da GTX 1650 do dono, parada na área de trabalho.
    const PARADA: &str = "NVIDIA GeForce GTX 1650, 35, 1485, 2175, 15.63, 75.00, 0, Not Active, Not Active, Not Active, Not Active, Not Active\n";

    #[test]
    fn le_a_saida_real_da_gtx_1650() {
        let l = ler_linha(PARADA).expect("leu");
        assert_eq!(l.nome, "NVIDIA GeForce GTX 1650");
        assert_eq!(l.temperatura_c, Some(35.0));
        assert_eq!(l.limite_potencia_w, Some(75.0));
        assert!(!l.termico && !l.teto_de_energia && !l.freio_de_hardware);
        assert_eq!(julgar(&l), EstadoGpu::SemCarga);
    }

    #[test]
    fn campo_nao_disponivel_vira_nada_e_nao_zero() {
        let l = ler_linha("GPU, [N/A], 1000, 2000, [N/A], [N/A], 90, Not Active, Not Active, Not Active, Not Active, Not Active").unwrap();
        assert_eq!(l.temperatura_c, None);
        assert_eq!(l.potencia_w, None);
        assert_eq!(julgar(&l), EstadoGpu::Livre);
    }

    #[test]
    fn teto_de_energia_em_carga_nao_e_alerta() {
        let l = ler_linha("GPU, 70, 1800, 2000, 75, 75, 99, Not Active, Not Active, Active, Not Active, Not Active").unwrap();
        assert_eq!(julgar(&l), EstadoGpu::TetoDeEnergiaNormal);
    }

    #[test]
    fn temperatura_e_freio_valem_mesmo_sem_carga() {
        let t = ler_linha("GPU, 90, 900, 2000, 40, 75, 10, Not Active, Active, Not Active, Not Active, Not Active").unwrap();
        assert_eq!(julgar(&t), EstadoGpu::Temperatura);
        let f = ler_linha("GPU, 60, 600, 2000, 40, 75, 95, Not Active, Not Active, Not Active, Active, Not Active").unwrap();
        assert_eq!(julgar(&f), EstadoGpu::FreioDeHardware);
    }

    #[test]
    fn saida_curta_nao_vira_leitura() {
        assert!(ler_linha("GPU, 35").is_none());
        assert!(ler_linha("").is_none());
    }
}

#[cfg(test)]
mod nesta_maquina {
    /// Só leitura: `cargo test -- --ignored sensores_desta_placa --nocapture`.
    #[test]
    #[ignore]
    fn sensores_desta_placa() {
        println!("{:#?}", super::ler());
    }
}
