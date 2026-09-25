// Quanto o Otimiza leva para abrir (3.0)
//
// Do começo do processo até a tela estar pronta para uso. O número vai para o
// log e para o relatório de suporte: abertura lenta passa a ter número, em vez
// de "demorou um pouco". Medido uma vez por execução.

use std::sync::OnceLock;
use std::time::Instant;

static INICIO: OnceLock<Instant> = OnceLock::new();
static PRONTA_MS: OnceLock<u64> = OnceLock::new();

/// Chamado na primeira linha de `run()`.
pub fn marcar_inicio() {
    let _ = INICIO.set(Instant::now());
}

/// A tela avisou que está pronta. Só a primeira vez conta: recarregar a tela
/// não é abrir o programa.
pub fn marcar_pronta() -> Option<u64> {
    let inicio = INICIO.get()?;
    Some(*PRONTA_MS.get_or_init(|| inicio.elapsed().as_millis() as u64))
}

pub fn medida() -> Option<u64> {
    PRONTA_MS.get().copied()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_primeira_marca_vale_e_as_seguintes_nao_mudam() {
        marcar_inicio();
        let primeira = marcar_pronta().expect("com início marcado, há medida");
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(marcar_pronta(), Some(primeira));
        assert_eq!(medida(), Some(primeira));
    }
}
