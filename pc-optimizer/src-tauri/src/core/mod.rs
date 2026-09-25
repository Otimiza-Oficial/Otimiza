// Detecção da plataforma e as regras de medição comuns: estatística, saúde dos quadros e a origem de cada número.

pub mod estatistica;
pub mod fluidez;
pub mod janela;
#[cfg(windows)]
pub mod pdh;
pub mod sensores;
pub mod platform;
pub mod telemetria;
pub mod travadas;

pub use platform::PlatformDetector;
