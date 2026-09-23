// Contas básicas de medição
//
// Média, mediana, desvio e percentil, puros, para quem descreve UMA série
// (a saúde dos quadros, o detetive de travadas, a janela do Mapa).
//
// COMPARAR duas séries — "melhorou ou é ruído?" — NÃO mora aqui: a regra do
// produto para isso é `modules::repeticoes` (intervalos de 95% que não se
// tocam), publicada na 2.8. Até a junção com a 2.8 havia aqui um teste de
// Welch em paralelo; dois critérios para a mesma pergunta davam respostas
// diferentes para os mesmos números, e ele saiu.

/// Média aritmética. `None` para lista vazia.
pub fn media(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    Some(xs.iter().sum::<f64>() / xs.len() as f64)
}

/// Mediana. `None` para lista vazia.
pub fn mediana(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    let mut v = xs.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    Some(if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 })
}

/// Desvio padrão AMOSTRAL (divide por n-1). `None` com menos de 2 valores.
pub fn desvio(xs: &[f64]) -> Option<f64> {
    if xs.len() < 2 {
        return None;
    }
    let m = media(xs)?;
    let soma: f64 = xs.iter().map(|x| (x - m).powi(2)).sum();
    Some((soma / (xs.len() - 1) as f64).sqrt())
}

/// Coeficiente de variação: desvio / média. Mede o quanto as rodadas
/// concordam entre si, independente da escala.
pub fn coeficiente_de_variacao(xs: &[f64]) -> Option<f64> {
    let m = media(xs)?;
    if m.abs() < f64::EPSILON {
        return None;
    }
    Some(desvio(xs)? / m.abs())
}

/// Percentil pelo método do posto mais próximo (o mesmo das ferramentas de
/// análise de quadros). `p` em [0, 100].
pub fn percentil(xs: &[f64], p: f64) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    let mut v = xs.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let p = p.clamp(0.0, 100.0);
    let posto = ((p / 100.0) * v.len() as f64).ceil() as usize;
    Some(v[posto.clamp(1, v.len()) - 1])
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn contas_basicas() {
        let xs = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert_eq!(media(&xs), Some(5.0));
        assert_eq!(mediana(&xs), Some(4.5));
        let d = desvio(&xs).unwrap();
        assert!((d - 2.138).abs() < 0.001, "{}", d);
        assert_eq!(media(&[]), None);
        assert_eq!(desvio(&[1.0]), None);
    }

    #[test]
    fn percentil_pelo_posto_mais_proximo() {
        let xs: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        assert_eq!(percentil(&xs, 99.0), Some(99.0));
        assert_eq!(percentil(&xs, 95.0), Some(95.0));
        assert_eq!(percentil(&xs, 100.0), Some(100.0));
        assert_eq!(percentil(&xs, 0.0), Some(1.0));
    }

}
