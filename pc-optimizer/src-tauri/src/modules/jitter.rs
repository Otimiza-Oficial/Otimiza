// Engasgo: quanto o agendador do Windows demora para devolver a CPU. Pede para dormir 1 ms muitas vezes e mede
// o atraso; é o que o jogador sente como travada, e a média de FPS esconde.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const SAMPLES: usize = 1500;
const REQUESTED: Duration = Duration::from_millis(1);
/// Um quadro a 60 FPS dura ~16,7 ms: atraso acima disso é engasgo visível.
const HITCH_THRESHOLD_MS: f64 = 16.7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitterReport {
    pub median_delay_ms: f64,
    pub p99_delay_ms: f64,
    pub hitches_per_minute: f64,
}

pub fn measure() -> JitterReport {
    let mut delays_ms = Vec::with_capacity(SAMPLES);
    let started = Instant::now();

    for _ in 0..SAMPLES {
        let before = Instant::now();
        std::thread::sleep(REQUESTED);
        let slept = before.elapsed();

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

        assert!(report.median_delay_ms >= 0.0);
        assert!(report.p99_delay_ms >= report.median_delay_ms);
        assert!(report.hitches_per_minute >= 0.0);
    }
}
