// ---------------------------------------------------------------------------
// A GUARDA DE FPS — o gerador nunca pode tirar quadro real do jogo
//
// O gerador usa a mesma placa de vídeo que o jogo. Numa placa no limite, os
// ~2 ms dele por quadro podem virar quadro a menos para o jogo — e aí a pessoa
// perde FPS de verdade para ganhar imagem inventada. A regra do produto é o
// contrário: nunca diminuir o FPS do jogo.
//
// COMO A GUARDA CONFERE, sem palpite: de tempos em tempos a geração PAUSA por
// um instante, e o FPS real do jogo é medido com e sem ela, colados no tempo.
// Uma comparação isolada é ruído (a cena muda), então a decisão usa a média de
// várias. Perdeu mais que a tolerância: o gerador se desliga e diz por quê.
//
// Função pura, com o relógio e o contador de quadros entrando por parâmetro.
// ---------------------------------------------------------------------------

/// Duração de cada janela de medição (com e sem geração).
pub const JANELA: f64 = 1.5;
/// Intervalo entre comparações nas primeiras rodadas: decide rápido.
pub const INTERVALO_INICIAL: f64 = 5.0;
/// Depois de decidir que está tudo bem, confere de vez em quando.
pub const INTERVALO_DE_ROTINA: f64 = 30.0;
/// Quantas comparações antes de poder decidir.
pub const MINIMO_DE_AMOSTRAS: usize = 3;
/// Razão FPS com geração ÷ sem geração abaixo da qual o gerador desliga.
pub const RAZAO_MINIMA: f64 = 0.96;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Fase {
    /// Gerando; a próxima comparação começa em `proxima`.
    Gerando { proxima: f64 },
    /// Medindo a janela COM geração, que termina em `ate`.
    MedindoComGeracao { desde: f64, reais_no_inicio: u64, ate: f64 },
    /// Geração pausada, medindo SEM ela até `ate`.
    MedindoSemGeracao { fps_com: f64, desde: f64, reais_no_inicio: u64, ate: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acao {
    Nada,
    Pausar,
    Retomar,
    /// Perda de FPS confirmada: desligar a geração.
    Desligar,
}

#[derive(Debug, Clone)]
pub struct Guarda {
    fase: Fase,
    /// Razões FPS com ÷ sem geração, uma por comparação.
    pub razoes: Vec<f64>,
}

impl Guarda {
    pub fn nova(agora: f64) -> Guarda {
        Guarda { fase: Fase::Gerando { proxima: agora + INTERVALO_INICIAL }, razoes: Vec::new() }
    }

    /// A geração deve estar pausada neste instante?
    pub fn pausada(&self) -> bool {
        matches!(self.fase, Fase::MedindoSemGeracao { .. })
    }

    /// Média das razões medidas, quando já há alguma.
    pub fn razao_media(&self) -> Option<f64> {
        (!self.razoes.is_empty()).then(|| self.razoes.iter().sum::<f64>() / self.razoes.len() as f64)
    }

    pub fn passo(&mut self, agora: f64, reais: u64) -> Acao {
        match self.fase {
            Fase::Gerando { proxima } if agora >= proxima => {
                self.fase = Fase::MedindoComGeracao { desde: agora, reais_no_inicio: reais, ate: agora + JANELA };
                Acao::Nada
            }
            Fase::MedindoComGeracao { desde, reais_no_inicio, ate } if agora >= ate => {
                let fps_com = (reais - reais_no_inicio) as f64 / (agora - desde).max(1e-3);
                self.fase = Fase::MedindoSemGeracao { fps_com, desde: agora, reais_no_inicio: reais, ate: agora + JANELA };
                Acao::Pausar
            }
            Fase::MedindoSemGeracao { fps_com, desde, reais_no_inicio, ate } if agora >= ate => {
                let fps_sem = (reais - reais_no_inicio) as f64 / (agora - desde).max(1e-3);
                // Jogo parado (menu, carregamento) não serve de comparação.
                if fps_sem >= 15.0 && fps_com >= 15.0 {
                    self.razoes.push(fps_com / fps_sem);
                }
                let intervalo = if self.razoes.len() < MINIMO_DE_AMOSTRAS { INTERVALO_INICIAL } else { INTERVALO_DE_ROTINA };
                self.fase = Fase::Gerando { proxima: agora + intervalo };
                if self.razoes.len() >= MINIMO_DE_AMOSTRAS && self.razao_media().is_some_and(|r| r < RAZAO_MINIMA) {
                    Acao::Desligar
                } else {
                    Acao::Retomar
                }
            }
            _ => Acao::Nada,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simula `segundos` com o jogo a `fps_com` gerando e `fps_sem` pausado.
    fn simular(fps_com: f64, fps_sem: f64, segundos: f64) -> (Guarda, Vec<Acao>) {
        let mut g = Guarda::nova(0.0);
        let mut reais = 0.0f64;
        let mut acoes = Vec::new();
        let passo = 0.01;
        let mut t = 0.0;
        while t < segundos {
            reais += passo * if g.pausada() { fps_sem } else { fps_com };
            let a = g.passo(t, reais as u64);
            if a != Acao::Nada {
                acoes.push(a);
            }
            if a == Acao::Desligar {
                break;
            }
            t += passo;
        }
        (g, acoes)
    }

    #[test]
    fn sem_perda_nao_desliga_e_pausa_so_por_instantes() {
        let (g, acoes) = simular(70.0, 70.0, 120.0);
        assert!(!acoes.contains(&Acao::Desligar));
        assert!(acoes.iter().filter(|a| **a == Acao::Pausar).count() >= MINIMO_DE_AMOSTRAS);
        assert!((g.razao_media().unwrap() - 1.0).abs() < 0.03);
    }

    #[test]
    fn perda_real_desliga_em_poucas_rodadas() {
        let (g, acoes) = simular(62.0, 70.0, 120.0);
        assert_eq!(acoes.last(), Some(&Acao::Desligar));
        assert_eq!(g.razoes.len(), MINIMO_DE_AMOSTRAS);
    }

    #[test]
    fn perda_pequena_dentro_da_tolerancia_nao_desliga() {
        let (_, acoes) = simular(68.5, 70.0, 120.0);
        assert!(!acoes.contains(&Acao::Desligar));
    }

    #[test]
    fn jogo_parado_nao_conta() {
        let (g, acoes) = simular(5.0, 5.0, 60.0);
        assert!(g.razoes.is_empty());
        assert!(!acoes.contains(&Acao::Desligar));
    }

    #[test]
    fn decide_antes_de_trinta_segundos() {
        let (_, acoes) = simular(50.0, 70.0, 30.0);
        assert_eq!(acoes.last(), Some(&Acao::Desligar));
    }
}
