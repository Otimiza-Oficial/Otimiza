// Núcleo
//
// Detecção da plataforma, e — desde a 2.9 — as regras de medição que todos os
// módulos compartilham: estatística (comparar rodadas sem confundir ruído com
// ganho), saúde dos quadros (a cauda que a média esconde) e a origem de cada
// número (medido, derivado, estimado ou desconhecido).

pub mod confiabilidade;
pub mod estatistica;
pub mod fluidez;
pub mod gargalo;
#[cfg(windows)]
pub mod pdh;
pub mod platform;
pub mod telemetria;
pub mod travadas;

pub use platform::PlatformDetector;
