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

pub mod atualizacao;
pub mod baseline;
pub mod benchmark;
pub mod changelog;
pub mod convite;
pub mod jitter;
pub mod latencia;
pub mod licenca;
pub mod maquina;
pub mod medicoes;
pub mod pontuacao;
pub mod regressao;
pub mod optimizer;
pub mod orquestrador;
pub mod preferences;
pub mod prova;
pub mod repeticoes;
pub mod report;
pub mod safety;
pub mod streaming;
pub mod transacao;
pub mod vram;
pub mod monitor;
pub mod gargalo;
pub mod telemetry;

#[cfg(target_os = "windows")]
pub mod windows;

pub use monitor::{PerformanceMonitor, PerformanceMetrics};
