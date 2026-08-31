// Módulos do Otimiza
//
// O antigo `diagnostic.rs` foi removido nesta versão. Ele calculava uma "nota
// de saúde" de 0 a 100 que não consultava nenhum dos módulos de medição de
// verdade, escrevia em inglês para cliente brasileiro, e ficava lado a lado com
// um veredito honesto na mesma tela. Manter os dois era o pior desfecho
// possível: o cliente fotografa a nota, não o parágrafo.
//
// Quem responde "o que há de errado com este PC" agora é
// `windows::veredito`, que elege uma frase a partir dos módulos que medem.

pub mod benchmark;
pub mod changelog;
pub mod jitter;
pub mod licenca;
pub mod maquina;
pub mod optimizer;
pub mod preferences;
pub mod prova;
pub mod report;
pub mod safety;
pub mod monitor;

#[cfg(target_os = "windows")]
pub mod windows;

pub use monitor::{PerformanceMonitor, PerformanceMetrics};
