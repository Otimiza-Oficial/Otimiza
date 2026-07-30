// Medidor de engasgos
//
// Esta é a medida que falta em todo otimizador de PC: a travada em si.
//
// Ninguém reclama de "média de FPS baixa". As pessoas reclamam que o jogo
// *engasga* — trava por uma fração de segundo no meio da partida. Isso não
// aparece em média nenhuma: um congelamento de 40 ms derruba a suavidade e
// quase não mexe na média de 60 quadros por segundo.
//
// O que engasga um PC é o agendador do Windows demorar para devolver a CPU a um
// processo. Dá para medir isso diretamente: peça para dormir 1 ms, muitas vezes,
// e veja quanto o sistema realmente demorou para te acordar. O atraso é o que o
// jogador sente como travada.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Quantidade de esperas medidas. Cada uma custa ~1 ms mais o atraso.
const SAMPLES: usize = 1500;
/// Duração pedida em cada espera.
const REQUESTED: Duration = Duration::from_millis(1);
/// A partir deste atraso a pausa é longa o suficiente para ser percebida como
/// engasgo num jogo a 60 quadros por segundo (um quadro dura ~16,7 ms).
const HITCH_THRESHOLD_MS: f64 = 16.7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitterReport {
    /// Atraso típico do agendador, em milissegundos.
    pub median_delay_ms: f64,
    /// Atraso no pior 1% das esperas. É este número que vira travada visível.
    pub p99_delay_ms: f64,
    /// Engasgos por minuto: esperas que passaram da duração de um quadro.
    pub hitches_per_minute: f64,
}

/// Executa a medição. Leva cerca de 3 segundos.
pub fn measure() -> JitterReport {
    let mut delays_ms = Vec::with_capacity(SAMPLES);
    let started = Instant::now();

    for _ in 0..SAMPLES {
        let before = Instant::now();
        std::thread::sleep(REQUESTED);
        let slept = before.elapsed();

        // O atraso é o excesso sobre o que foi pedido. Nunca é negativo:
        // o sistema pode demorar mais, jamais menos.
        let delay = slept.saturating_sub(REQUESTED);
        delays_ms.push(delay.as_secs_f64() * 1000.0);
    }

    let elapsed_minutes = started.elapsed().as_secs_f64() / 60.0;
    let hitches = delays_ms
        .iter()
        .filter(|delay| **delay >= HITCH_THRESHOLD_MS)
        .count();

    let hitches_per_minute = if elapsed_minutes > 0.0 {
        hitches as f64 / elapsed_minutes
    } else {
        0.0
    };

    JitterReport {
        median_delay_ms: percentile(&mut delays_ms, 50.0),
        p99_delay_ms: percentile(&mut delays_ms, 99.0),
        hitches_per_minute,
    }
}

/// Percentil por ordenação. A amostra é pequena o bastante para o custo ser
/// irrelevante, e ordenar evita as aproximações de um cálculo incremental.
fn percentile(values: &mut [f64], percent: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let rank = (percent / 100.0) * (values.len() - 1) as f64;
    let low = rank.floor() as usize;
    let high = rank.ceil() as usize;

    if low == high {
        return values[low];
    }

    // Interpolação entre os dois vizinhos, para o percentil não pular degraus
    // quando a amostra é pequena.
    let weight = rank - low as f64;
    values[low] * (1.0 - weight) + values[high] * weight
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_of_known_series() {
        let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&mut values, 50.0), 3.0);
        assert_eq!(percentile(&mut values, 0.0), 1.0);
        assert_eq!(percentile(&mut values, 100.0), 5.0);
    }

    #[test]
    fn percentile_interpolates_between_neighbours() {
        let mut values = vec![0.0, 10.0];
        assert_eq!(percentile(&mut values, 50.0), 5.0);
    }

    #[test]
    fn percentile_of_empty_series_is_zero() {
        assert_eq!(percentile(&mut [], 99.0), 0.0);
    }

    #[test]
    fn measures_real_scheduler_delay() {
        let report = measure();
        println!("{:?}", report);

        // O agendador nunca acorda antes da hora, então o atraso é sempre >= 0.
        assert!(report.median_delay_ms >= 0.0);
        // O pior caso não pode ser melhor que o típico.
        assert!(report.p99_delay_ms >= report.median_delay_ms);
        assert!(report.hitches_per_minute >= 0.0);
    }
}
