// Estatística de medição — o motor comum da 2.9
//
// POR QUE ISTO EXISTE
//
// Até a 2.7 o produto tinha quatro jeitos de comparar antes e depois
// (`benchmark.rs`, `prova.rs`, `experimento.rs`, `motorenergia.rs`), cada um
// com a própria regra de "isso é ganho ou é ruído". Duas medições de 30 s no
// mesmo PC, sem mudar nada, variam alguns por cento — e um produto que chama
// essa variação de "+4% de FPS" está anunciando ruído.
//
// Aqui fica UMA regra, com teste, que todos passam a usar:
//
//   1. Nunca comparar uma rodada contra uma rodada. Comparar CONJUNTOS de
//      rodadas, intercaladas (A B B A ...), para o aquecimento da máquina não
//      favorecer quem foi medido por último.
//   2. O ganho só é "medido" quando a diferença passa do ruído com 95% de
//      confiança (teste t de Welch, que não supõe variâncias iguais).
//   3. Com poucas rodadas, o veredito é "inconclusivo" — nunca um número.
//
// Tudo aqui é função pura.

use serde::{Deserialize, Serialize};

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

/// Valor crítico bicaudal da distribuição t para 95%, por graus de liberdade.
///
/// Tabela padrão; acima de 30 graus usa o valor da normal (1,96), que é onde a
/// diferença vira irrelevante para o número de rodadas que o produto faz.
fn t_critico_95(gl: f64) -> f64 {
    const TABELA: [f64; 30] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179,
        2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064,
        2.060, 2.056, 2.052, 2.048, 2.045, 2.042,
    ];
    if !gl.is_finite() || gl < 1.0 {
        return TABELA[0];
    }
    let i = gl.floor() as usize;
    if i > 30 {
        1.96
    } else {
        TABELA[i - 1]
    }
}

/// O que a comparação concluiu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Conclusao {
    /// A diferença passou do ruído, a favor do candidato.
    MelhoraMedida,
    /// A diferença passou do ruído, contra o candidato.
    PioraMedida,
    /// Rodadas suficientes, e a diferença ficou dentro do ruído.
    ProvavelmenteSemMudanca,
    /// Poucas rodadas, ou rodadas que discordam demais entre si.
    Inconclusivo,
}

/// Resultado de comparar as rodadas de base (A) com as do candidato (B).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparacao {
    pub media_base: f64,
    pub media_candidato: f64,
    /// (B - A) / A, em %. Positivo = B maior.
    pub diferenca_pct: f64,
    /// Meia-largura do intervalo de 95% da diferença, em % da base.
    pub margem_pct: f64,
    pub rodadas_base: usize,
    pub rodadas_candidato: usize,
    pub conclusao: Conclusao,
}

/// Mínimo de rodadas de cada lado para arriscar uma conclusão.
pub const RODADAS_MINIMAS: usize = 3;

/// Compara duas amostras de rodadas.
///
/// `maior_e_melhor`: verdadeiro para FPS e 1% low, falso para tempo de quadro,
/// P99 e contagem de engasgos.
pub fn comparar(base: &[f64], candidato: &[f64], maior_e_melhor: bool) -> Option<Comparacao> {
    let ma = media(base)?;
    let mb = media(candidato)?;
    if ma.abs() < f64::EPSILON {
        return None;
    }
    let diferenca_pct = (mb - ma) / ma.abs() * 100.0;

    let poucas = base.len() < RODADAS_MINIMAS || candidato.len() < RODADAS_MINIMAS;
    let (margem_pct, conclusao) = match (desvio(base), desvio(candidato)) {
        (Some(sa), Some(sb)) if !poucas => {
            let (na, nb) = (base.len() as f64, candidato.len() as f64);
            let (va, vb) = (sa * sa / na, sb * sb / nb);
            let erro = (va + vb).sqrt();
            // Graus de liberdade de Welch–Satterthwaite.
            let gl = if erro > 0.0 {
                (va + vb).powi(2) / (va * va / (na - 1.0) + vb * vb / (nb - 1.0))
            } else {
                na + nb - 2.0
            };
            let margem = t_critico_95(gl) * erro / ma.abs() * 100.0;
            let conclusao = if diferenca_pct.abs() <= margem {
                Conclusao::ProvavelmenteSemMudanca
            } else if (diferenca_pct > 0.0) == maior_e_melhor {
                Conclusao::MelhoraMedida
            } else {
                Conclusao::PioraMedida
            };
            (margem, conclusao)
        }
        _ => (f64::NAN, Conclusao::Inconclusivo),
    };

    Some(Comparacao {
        media_base: ma,
        media_candidato: mb,
        diferenca_pct,
        margem_pct,
        rodadas_base: base.len(),
        rodadas_candidato: candidato.len(),
        conclusao,
    })
}

/// Qual lado mede em cada posição de uma bateria intercalada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lado {
    Base,
    Candidato,
}

/// A ordem das rodadas: A B B A, repetido.
///
/// Por que não A B A B: a máquina esquenta e o cache de shader enche ao longo
/// da bateria, e isso é uma deriva quase linear. Em A B A B o candidato é
/// sempre medido "um passo depois" da base e herda a deriva inteira; em
/// A B B A cada par se espelha e a deriva linear se cancela.
pub fn ordem_intercalada(pares: usize) -> Vec<Lado> {
    (0..pares)
        .flat_map(|i| {
            if i % 2 == 0 {
                [Lado::Base, Lado::Candidato]
            } else {
                [Lado::Candidato, Lado::Base]
            }
        })
        .collect()
}

/// Se uma rodada serve para comparação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Validade {
    Valida,
    /// Serve, com ressalva escrita na tela (amostra curta, cena instável).
    Questionavel,
    /// Não entra em nenhuma conta nem em nenhuma frase de ganho.
    Invalida,
}

/// Validade de uma bateria pelo quanto as rodadas do mesmo lado concordam.
///
/// Acima de 5% de variação entre rodadas iguais, a diferença entre lados que
/// o produto consegue afirmar fica grande demais para servir de decisão;
/// acima de 15%, a cena mudou de uma rodada para outra e nada ali é
/// comparável.
pub fn validade_por_variacao(rodadas: &[f64]) -> Validade {
    const CV_QUESTIONAVEL: f64 = 0.05;
    const CV_INVALIDO: f64 = 0.15;
    match coeficiente_de_variacao(rodadas) {
        None => Validade::Questionavel,
        Some(cv) if cv > CV_INVALIDO => Validade::Invalida,
        Some(cv) if cv > CV_QUESTIONAVEL => Validade::Questionavel,
        Some(_) => Validade::Valida,
    }
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

    #[test]
    fn ruido_nao_vira_ganho() {
        // Mesmo PC, nada mudou: rodadas oscilando ±3%.
        let a = [100.0, 103.0, 97.0, 101.0];
        let b = [102.0, 98.0, 104.0, 99.0];
        let c = comparar(&a, &b, true).unwrap();
        assert_eq!(c.conclusao, Conclusao::ProvavelmenteSemMudanca, "{:?}", c);
    }

    #[test]
    fn ganho_claro_e_medido() {
        let a = [100.0, 101.0, 99.0, 100.0];
        let b = [112.0, 111.0, 113.0, 112.0];
        let c = comparar(&a, &b, true).unwrap();
        assert_eq!(c.conclusao, Conclusao::MelhoraMedida);
        assert!((c.diferenca_pct - 12.0).abs() < 0.01);
    }

    #[test]
    fn menor_e_melhor_para_tempo_de_quadro() {
        let a = [20.0, 20.5, 19.5, 20.0];
        let b = [15.0, 15.2, 14.8, 15.0];
        assert_eq!(comparar(&a, &b, false).unwrap().conclusao, Conclusao::MelhoraMedida);
        assert_eq!(comparar(&a, &b, true).unwrap().conclusao, Conclusao::PioraMedida);
    }

    #[test]
    fn uma_rodada_de_cada_lado_nunca_conclui() {
        let c = comparar(&[100.0], &[150.0], true).unwrap();
        assert_eq!(c.conclusao, Conclusao::Inconclusivo);
        let c = comparar(&[100.0, 101.0], &[150.0, 151.0], true).unwrap();
        assert_eq!(c.conclusao, Conclusao::Inconclusivo);
    }

    #[test]
    fn ordem_espelhada_cancela_deriva_linear() {
        let ordem = ordem_intercalada(4);
        assert_eq!(ordem.len(), 8);
        // Deriva linear: a rodada i mede i a mais, e os dois lados são iguais.
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for (i, lado) in ordem.iter().enumerate() {
            let valor = 100.0 + i as f64;
            match lado {
                Lado::Base => a.push(valor),
                Lado::Candidato => b.push(valor),
            }
        }
        assert_eq!(media(&a), media(&b), "a deriva favoreceu um lado");
    }

    #[test]
    fn validade_pela_concordancia_das_rodadas() {
        assert_eq!(validade_por_variacao(&[100.0, 101.0, 99.0]), Validade::Valida);
        assert_eq!(validade_por_variacao(&[100.0, 110.0, 92.0]), Validade::Questionavel);
        assert_eq!(validade_por_variacao(&[100.0, 140.0, 70.0]), Validade::Invalida);
        assert_eq!(validade_por_variacao(&[100.0]), Validade::Questionavel);
    }
}
