pub mod abertura;
pub mod atualizacao;
pub mod autoajuste;
pub mod baseline;
pub mod benchmark;
pub mod changelog;
pub mod convite;
pub mod deriva;
pub mod historico;
pub mod jitter;
pub mod latencia;
pub mod licenca;
pub mod maquina;
pub mod medicoes;
pub mod pontuacao;
pub mod portao;
pub mod regressao;
pub mod mouse;
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
pub mod nucleos;
pub mod monitor;
pub mod gargalo;
pub mod telemetry;

#[cfg(target_os = "windows")]
pub mod windows;

pub use monitor::{PerformanceMonitor, PerformanceMetrics};
