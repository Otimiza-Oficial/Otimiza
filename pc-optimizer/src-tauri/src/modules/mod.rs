// Modules - Optimization and Diagnostic Modules
// Contains platform-specific optimizers and diagnostic engine

pub mod benchmark;
pub mod changelog;
pub mod diagnostic;
pub mod jitter;
pub mod optimizer;
pub mod preferences;
pub mod report;
pub mod safety;
pub mod monitor;

#[cfg(target_os = "windows")]
pub mod windows;

pub use diagnostic::{DiagnosticEngine, DiagnosticReport};
pub use monitor::{PerformanceMonitor, PerformanceMetrics};
