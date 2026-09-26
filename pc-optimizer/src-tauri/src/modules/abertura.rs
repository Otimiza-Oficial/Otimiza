// Quanto o Otimiza leva para abrir, do começo do processo até a tela pronta. Vai para o log e para o relatório de suporte.

use std::sync::OnceLock;
use std::time::Instant;

static INICIO: OnceLock<Instant> = OnceLock::new();
static PRONTA_MS: OnceLock<u64> = OnceLock::new();

pub fn marcar_inicio() {
    let _ = INICIO.set(Instant::now());
}

/// Só a primeira vez conta: recarregar a tela não é abrir o programa.
pub fn marcar_pronta() -> Option<u64> {
    let inicio = INICIO.get()?;
    Some(*PRONTA_MS.get_or_init(|| inicio.elapsed().as_millis() as u64))
}

/// Milissegundos desde o começo do processo, para marcar as etapas da abertura no log.
pub fn desde_o_inicio() -> Option<u64> {
    INICIO.get().map(|i| i.elapsed().as_millis() as u64)
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

/// Quanto custa, nesta máquina, o que roda antes da janela: `cargo test --lib -- --ignored --nocapture custo_antes_da_janela`.
#[cfg(all(test, target_os = "windows"))]
mod custo {
    #[test]
    #[ignore]
    fn custo_antes_da_janela() {
        let t = std::time::Instant::now();
        let _ = crate::modules::changelog::ChangeLog::load();
        println!("ChangeLog::load: {} ms", t.elapsed().as_millis());
        let t = std::time::Instant::now();
        let _ = crate::modules::PerformanceMonitor::new();
        println!("PerformanceMonitor::new: {} ms", t.elapsed().as_millis());
        let t = std::time::Instant::now();
        let _ = crate::modules::windows::processes::ProcessMonitor::new();
        println!("ProcessMonitor::new: {} ms", t.elapsed().as_millis());
        let t = std::time::Instant::now();
        crate::utils::diagnostico::abrir_sessao();
        println!("abrir_sessao: {} ms", t.elapsed().as_millis());
    }
}
