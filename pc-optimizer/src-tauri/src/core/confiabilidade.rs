// De onde veio cada número
//
// Regra da 2.9: nenhuma métrica chega na tela sem dizer se foi MEDIDA, CALCULADA
// a partir de medidas, ESTIMADA, ou se o produto NÃO SABE. Estimativa nunca
// vira medição no caminho, e "não sei" nunca vira zero.
//
// Exemplos do que cai em cada caso nesta máquina:
// - Medido: FPS por ETW, uso da GPU pelo contador do Windows, VRAM pelo DXGI.
// - Derivado: clock efetivo (frequência × % de desempenho do contador),
//   índice de fluidez (fórmula sobre 1% low, P99 e engasgos).
// - Estimado: temperatura da zona térmica ACPI, que muitas placas-mãe
//   preenchem com valor fixo.
// - Desconhecido: temperatura e potência do pacote da CPU — exigem ler
//   registradores do processador por um driver de kernel, e o Otimiza não
//   instala driver.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confiabilidade {
    Medido,
    Derivado,
    Estimado,
    Desconhecido,
}

/// Um valor com a origem dele. `valor` é `None` exatamente quando a origem é
/// `Desconhecido` — o construtor garante isso.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Medida {
    pub valor: Option<f64>,
    pub unidade: &'static str,
    pub confiabilidade: Confiabilidade,
    /// De onde o número saiu, em palavras que o modo Expert mostra
    /// ("contador GPU Engine", "DXGI QueryVideoMemoryInfo"...).
    pub fonte: &'static str,
}

impl Medida {
    pub fn medida(valor: f64, unidade: &'static str, fonte: &'static str) -> Self {
        Self::com(valor, unidade, fonte, Confiabilidade::Medido)
    }
    pub fn derivada(valor: f64, unidade: &'static str, fonte: &'static str) -> Self {
        Self::com(valor, unidade, fonte, Confiabilidade::Derivado)
    }
    pub fn estimada(valor: f64, unidade: &'static str, fonte: &'static str) -> Self {
        Self::com(valor, unidade, fonte, Confiabilidade::Estimado)
    }
    pub fn desconhecida(unidade: &'static str, fonte: &'static str) -> Self {
        Medida { valor: None, unidade, confiabilidade: Confiabilidade::Desconhecido, fonte }
    }

    /// Valor não finito (NaN, infinito) vira desconhecido: não existe
    /// "medido: NaN".
    fn com(valor: f64, unidade: &'static str, fonte: &'static str, c: Confiabilidade) -> Self {
        if valor.is_finite() {
            Medida { valor: Some(valor), unidade, confiabilidade: c, fonte }
        } else {
            Self::desconhecida(unidade, fonte)
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nao_existe_medido_sem_valor() {
        let m = Medida::medida(f64::NAN, "%", "teste");
        assert_eq!(m.confiabilidade, Confiabilidade::Desconhecido);
        assert_eq!(m.valor, None);
        let d = Medida::desconhecida("°C", "teste");
        assert_eq!(d.valor, None);
        let ok = Medida::derivada(4400.0, "MHz", "teste");
        assert_eq!(ok.valor, Some(4400.0));
        assert_eq!(ok.confiabilidade, Confiabilidade::Derivado);
    }
}
