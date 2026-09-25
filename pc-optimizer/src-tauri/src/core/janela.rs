// A janela do Mapa traduzida para a `Telemetry` que `modules::gargalo` lê: as duas telas dão a mesma resposta
// para os mesmos números. Cada métrica é a MEDIANA da janela: um pico de meio segundo não vira veredito.

use crate::core::estatistica::mediana;
use crate::core::fluidez::SaudeDosQuadros;
use crate::core::telemetria::Amostra;
use crate::core::travadas::{Investigacao, Suspeito};
use crate::modules::telemetry::{id_do_nucleo, Metric, Telemetry, Unit};

fn med(amostras: &[Amostra], f: impl Fn(&Amostra) -> Option<f64>) -> Option<f64> {
    mediana(&amostras.iter().filter_map(f).collect::<Vec<_>>())
}

/// `hz`: taxa do monitor principal, quando conhecida.
pub fn para_telemetria(
    amostras: &[Amostra],
    ram_total_mb: Option<f64>,
    vram_total_mb: Option<f64>,
    saude: Option<&SaudeDosQuadros>,
    travadas: Option<&Investigacao>,
    hz: Option<u32>,
    agora: u64,
) -> Telemetry {
    let fonte = format!("pdh, mediana de {} leituras", amostras.len());
    let mut t = Telemetry::new(agora, Some(500));
    let mut medida = |t: &mut Telemetry, id: &str, v: Option<f64>, u: Unit| {
        if let Some(v) = v {
            t.set(id, Metric::measured(v, u, fonte.clone()));
        }
    };

    let cpu = med(amostras, |a| a.cpu_total_pct);
    medida(&mut t, "cpu.usage.overall", cpu, Unit::Percent);
    // Índices contíguos: o classificador para no primeiro que faltar.
    let n = amostras.iter().map(|a| a.nucleos_pct.len()).max().unwrap_or(0);
    for i in 0..n {
        let v = med(amostras, |a| a.nucleos_pct.get(i).copied());
        if v.is_none() {
            break;
        }
        if let Some(v) = v {
            t.set_series(id_do_nucleo(i), Metric::measured(v, Unit::Percent, fonte.clone()));
        }
    }
    // "Ligada" se ligada na maior parte da janela.
    let flag = |bit: u64| -> Option<f64> {
        let lidas: Vec<bool> = amostras.iter().filter_map(|a| a.limite_flags.map(|f| f & bit != 0)).collect();
        (!lidas.is_empty()).then(|| (lidas.iter().filter(|b| **b).count() * 2 > lidas.len()) as u8 as f64)
    };
    medida(&mut t, "cpu.throttling.thermal", flag(1), Unit::Boolean);
    medida(&mut t, "cpu.throttling.power", flag(2), Unit::Boolean);

    let gpu = med(amostras, |a| a.gpu_pct);
    medida(&mut t, "gpu.usage", gpu, Unit::Percent);
    let vram_usada = med(amostras, |a| a.vram_usada_mb).map(|m| m / 1024.0);
    medida(&mut t, "vram.used", vram_usada, Unit::Gigabytes);
    medida(&mut t, "vram.total", vram_total_mb.map(|m| m / 1024.0), Unit::Gigabytes);
    if let (Some(u), Some(tot)) = (vram_usada, vram_total_mb.map(|m| m / 1024.0)) {
        if tot > 0.0 {
            medida(&mut t, "vram.usage", Some(u / tot * 100.0), Unit::Percent);
        }
    }
    medida(&mut t, "vram.shared_used", med(amostras, |a| a.vram_compartilhada_mb).map(|m| m / 1024.0), Unit::Gigabytes);
    let ram = match ram_total_mb.filter(|t| *t > 0.0) {
        Some(total) => med(amostras, |a| a.ram_disponivel_mb).map(|livre| (1.0 - livre / total) * 100.0),
        None => None,
    };
    medida(&mut t, "ram.usage", ram, Unit::Percent);
    medida(&mut t, "storage.busy", med(amostras, |a| a.disco_ocupado_pct), Unit::Percent);
    medida(&mut t, "display.refresh", hz.map(|h| h as f64), Unit::Hertz);

    if let Some(s) = saude {
        t.set("fps.average", Metric::measured(s.fps_medio, Unit::Fps, "etw, na mesma janela"));
        if let Some(l) = s.low_1pct_fps {
            t.set("fps.low_1pct", Metric::measured(l, Unit::Fps, "etw, na mesma janela"));
        }
        t.set(
            "frametime.stutters_per_minute",
            Metric::measured(s.engasgos_graves_por_minuto, Unit::Count, "etw, na mesma janela"),
        );
        // O par CPU e placa DURANTE a partida separa limite fora do hardware de gargalo de peça.
        medida(&mut t, "match.cpu_usage", cpu, Unit::Percent);
        medida(&mut t, "match.gpu_usage", gpu, Unit::Percent);
    }
    if let Some(inv) = travadas.filter(|i| !i.travadas.is_empty()) {
        let com_disco = inv.pistas.iter().find(|p| p.suspeito == Suspeito::Disco).map(|p| p.travadas).unwrap_or(0);
        t.set(
            "frametime.stutters_with_disk",
            Metric::measured(com_disco as f64 / inv.travadas.len() as f64 * 100.0, Unit::Percent, "detetive de travadas"),
        );
    }
    t.finish(0)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::modules::gargalo::{classificar_com, Classe};

    fn amostra(nucleos: Vec<f64>, gpu: f64) -> Amostra {
        Amostra {
            cpu_total_pct: Some(nucleos.iter().sum::<f64>() / nucleos.len() as f64),
            cpu_nucleo_max_pct: nucleos.iter().copied().reduce(f64::max),
            nucleos_pct: nucleos,
            gpu_pct: Some(gpu),
            vram_usada_mb: Some(1500.0),
            vram_compartilhada_mb: Some(100.0),
            commit_pct: Some(40.0),
            disco_ocupado_pct: Some(5.0),
            ram_disponivel_mb: Some(8000.0),
            limite_flags: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn um_nucleo_no_talo_vira_cpu_um_nucleo_no_classificador_da_2_8() {
        let janela: Vec<Amostra> = (0..40).map(|_| amostra(vec![99.0, 20.0, 15.0, 10.0, 12.0, 8.0, 9.0, 11.0], 55.0)).collect();
        let t = para_telemetria(&janela, Some(16384.0), Some(4096.0), None, None, Some(144), 0);
        let d = classificar_com(&t, &crate::modules::vram::avaliar(&t, &Default::default()));
        assert!(d.achados.iter().any(|a| a.classe == Classe::CpuUmNucleo), "{:?}", d.achados);
    }

    #[test]
    fn placa_no_limite() {
        let janela: Vec<Amostra> = (0..40).map(|_| amostra(vec![50.0; 8], 99.0)).collect();
        let t = para_telemetria(&janela, Some(16384.0), Some(4096.0), None, None, None, 0);
        let d = classificar_com(&t, &crate::modules::vram::avaliar(&t, &Default::default()));
        assert!(d.achados.iter().any(|a| a.classe == Classe::Gpu), "{:?}", d.achados);
    }
}
