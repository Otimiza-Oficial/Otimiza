// A saúde dos quadros: a cauda que a média de FPS esconde (percentis, 1% e 0,1% piores, quadros muito acima do
// normal). Número sem amostra não sai: vira `None`, e a tela diz "amostra curta". O 1% low é o mesmo de
// `frames::estatistica`, com teste de equivalência lá.

use serde::{Deserialize, Serialize};

use super::estatistica::{coeficiente_de_variacao, desvio, mediana, percentil};

pub const AMOSTRA_PARA_1PCT: usize = 2_000;
/// Com menos que isso o "0,1% pior" seria um ou dois quadros: sorte, não medida.
pub const AMOSTRA_PARA_01PCT: usize = 10_000;

/// RAZÃO sobre a mediana E ATRASO ABSOLUTO: só a razão acusaria todo quadro a 400 FPS; só o absoluto nunca
/// acusaria nada a 30 FPS. Os ms seguem o quadro de 60 Hz (16,7 ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Gravidade {
    Micro,
    Perceptivel,
    Severo,
    Extremo,
}

pub fn gravidade(intervalo_ms: f64, mediana_ms: f64) -> Option<Gravidade> {
    const EXTREMO_MS: f64 = 100.0;
    if intervalo_ms >= EXTREMO_MS {
        return Some(Gravidade::Extremo);
    }
    if mediana_ms <= 0.0 {
        return None;
    }
    let razao = intervalo_ms / mediana_ms;
    let a_mais = intervalo_ms - mediana_ms;
    if razao >= 3.0 && a_mais >= 16.7 {
        Some(Gravidade::Severo)
    } else if razao >= 2.0 && a_mais >= 8.0 {
        Some(Gravidade::Perceptivel)
    } else if razao >= 1.5 && a_mais >= 4.0 {
        Some(Gravidade::Micro)
    } else {
        None
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContagemDeEngasgos {
    pub micro: u32,
    pub perceptivel: u32,
    pub severo: u32,
    pub extremo: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaudeDosQuadros {
    pub quadros: usize,
    pub duracao_s: f64,
    pub fps_medio: f64,
    pub frametime_mediano_ms: f64,
    pub frametime_desvio_ms: Option<f64>,
    pub frametime_cv: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub low_1pct_fps: Option<f64>,
    pub low_01pct_fps: Option<f64>,
    pub engasgos: ContagemDeEngasgos,
    pub engasgos_graves_por_minuto: f64,
    /// DERIVADO dos números acima (`indice_de_fluidez`), e nunca aparece sem eles do lado.
    pub indice_de_fluidez: Option<f64>,
}

fn low(ordenados_desc: &[f64], fracao: f64) -> Option<f64> {
    let n = ((ordenados_desc.len() as f64) * fracao).floor().max(1.0) as usize;
    let piores = &ordenados_desc[..n.min(ordenados_desc.len())];
    let media = piores.iter().sum::<f64>() / piores.len() as f64;
    (media > 0.0).then(|| 1000.0 / media)
}

pub fn analisar(intervalos_ms: &[f64]) -> Option<SaudeDosQuadros> {
    let validos: Vec<f64> = intervalos_ms.iter().copied().filter(|x| x.is_finite() && *x > 0.0).collect();
    if validos.is_empty() {
        return None;
    }
    let n = validos.len();
    let duracao_ms: f64 = validos.iter().sum();
    let med = mediana(&validos)?;

    let mut desc = validos.clone();
    desc.sort_by(|a, b| b.total_cmp(a));

    let tem_1pct = n >= AMOSTRA_PARA_1PCT;
    let tem_01pct = n >= AMOSTRA_PARA_01PCT;

    let mut engasgos = ContagemDeEngasgos::default();
    for &x in &validos {
        match gravidade(x, med) {
            Some(Gravidade::Micro) => engasgos.micro += 1,
            Some(Gravidade::Perceptivel) => engasgos.perceptivel += 1,
            Some(Gravidade::Severo) => engasgos.severo += 1,
            Some(Gravidade::Extremo) => engasgos.extremo += 1,
            None => {}
        }
    }
    let minutos = duracao_ms / 60_000.0;
    let graves_por_minuto = if minutos > 0.0 {
        (engasgos.severo + engasgos.extremo) as f64 / minutos
    } else {
        0.0
    };

    let fps_medio = n as f64 / (duracao_ms / 1000.0);
    let p99 = if tem_1pct { percentil(&validos, 99.0) } else { None };
    let low_1 = if tem_1pct { low(&desc, 0.01) } else { None };

    let mut saude = SaudeDosQuadros {
        quadros: n,
        duracao_s: duracao_ms / 1000.0,
        fps_medio,
        frametime_mediano_ms: med,
        frametime_desvio_ms: desvio(&validos),
        frametime_cv: coeficiente_de_variacao(&validos),
        p95_ms: if tem_1pct { percentil(&validos, 95.0) } else { None },
        p99_ms: p99,
        low_1pct_fps: low_1,
        low_01pct_fps: if tem_01pct { low(&desc, 0.001) } else { None },
        engasgos,
        engasgos_graves_por_minuto: graves_por_minuto,
        indice_de_fluidez: None,
    };
    saude.indice_de_fluidez = indice_de_fluidez(&saude);
    Some(saude)
}

/// 40% consistência (1% low ÷ média), 40% cauda (mediana ÷ P99), 20% travadas (exp(-g/4)). Sem 1% low e P99,
/// sem índice.
pub fn indice_de_fluidez(s: &SaudeDosQuadros) -> Option<f64> {
    let low1 = s.low_1pct_fps?;
    let p99 = s.p99_ms?;
    if s.fps_medio <= 0.0 || p99 <= 0.0 {
        return None;
    }
    let consistencia = (low1 / s.fps_medio).clamp(0.0, 1.0);
    let cauda = (s.frametime_mediano_ms / p99).clamp(0.0, 1.0);
    let travadas = (-s.engasgos_graves_por_minuto / 4.0).exp();
    let indice = 100.0 * (0.4 * consistencia + 0.4 * cauda + 0.2 * travadas);
    Some((indice * 10.0).round() / 10.0)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn constante(ms: f64, n: usize) -> Vec<f64> {
        vec![ms; n]
    }

    #[test]
    fn jogo_perfeitamente_liso() {
        let s = analisar(&constante(10.0, 3_000)).unwrap();
        assert!((s.fps_medio - 100.0).abs() < 1e-9);
        assert_eq!(s.low_1pct_fps, Some(100.0));
        assert_eq!(s.p99_ms, Some(10.0));
        assert_eq!(s.engasgos, ContagemDeEngasgos::default());
        assert_eq!(s.indice_de_fluidez, Some(100.0));
        assert_eq!(s.low_01pct_fps, None);
    }

    #[test]
    fn amostra_curta_nao_inventa_cauda() {
        let s = analisar(&constante(10.0, 500)).unwrap();
        assert_eq!(s.low_1pct_fps, None);
        assert_eq!(s.p99_ms, None);
        assert_eq!(s.indice_de_fluidez, None);
        assert!((s.fps_medio - 100.0).abs() < 1e-9);
    }

    #[test]
    fn mesma_media_fluidez_diferente() {
        let liso = constante(10.0, 12_000);
        let mut travando = constante(9.8, 12_000);
        for i in (0..12_000).step_by(200) {
            travando[i] = 40.0;
        }
        let a = analisar(&liso).unwrap();
        let b = analisar(&travando).unwrap();
        assert!((a.fps_medio - b.fps_medio).abs() / a.fps_medio < 0.03);
        assert!(b.engasgos.severo == 60, "{:?}", b.engasgos);
        assert!(b.indice_de_fluidez.unwrap() < a.indice_de_fluidez.unwrap() - 20.0);
        assert!(b.low_01pct_fps.unwrap() < 30.0);
    }

    #[test]
    fn gravidade_combina_razao_e_atraso() {
        assert_eq!(gravidade(5.0, 2.5), None);
        assert_eq!(gravidade(26.0, 16.7), Some(Gravidade::Micro));
        assert_eq!(gravidade(34.0, 16.7), Some(Gravidade::Perceptivel));
        assert_eq!(gravidade(51.0, 16.7), Some(Gravidade::Severo));
        assert_eq!(gravidade(150.0, 16.7), Some(Gravidade::Extremo));
    }

    #[test]
    fn intervalos_invalidos_sao_ignorados() {
        let s = analisar(&[10.0, f64::NAN, -1.0, 0.0, 10.0]).unwrap();
        assert_eq!(s.quadros, 2);
        assert!(analisar(&[]).is_none());
    }
}
