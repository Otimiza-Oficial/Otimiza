// Núcleo
//
// Detecção da plataforma, e — desde a 2.9 — as regras de medição que todos os
// módulos compartilham: estatística (comparar rodadas sem confundir ruído com
// ganho), saúde dos quadros (a cauda que a média esconde) e a origem de cada
// número (o contrato é `modules::telemetry`, da 2.8).

pub mod estatistica;
pub mod fluidez;
pub mod janela;
#[cfg(windows)]
pub mod pdh;
pub mod platform;
pub mod telemetria;
pub mod travadas;

pub use platform::PlatformDetector;
