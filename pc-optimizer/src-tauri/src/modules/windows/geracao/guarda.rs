// A GUARDA DE FPS: o gerador usa a mesma placa que o jogo, e nunca pode tirar quadro real dele. De tempos em
// tempos a geração pausa e o FPS DO JOGO é medido com e sem ela, colados no tempo; a decisão usa a média de várias
// comparações. Perdeu mais que a tolerância: o gerador desliga e diz por quê.
//
// A contagem vem de fora (`contar(de, ate)`), por janela de tempo: pelo PresentMon no processo do jogo, os quadros
// chegam com atraso (o ETW entrega em lotes), por isso a conta só é feita `ATRASO_DA_CONTAGEM` depois de a janela
// sem geração fechar. Sem PresentMon, conta a captura da tela, como antes.

pub const JANELA: f64 = 1.5;
pub const INTERVALO_INICIAL: f64 = 5.0;
/// Cada conferência pausa o Frame Boost por `JANELA` segundos, e a pessoa vê o tranco (o FPS do jogo sozinho por um
/// instante). Com 30 s, e a conferência que falhava repetindo a cada 13 s para sempre, o log de uma sessão do FiveM
/// mostrou a sobreposição escondida 15% do tempo. Depois das três primeiras, uma a cada 5 minutos.
pub const INTERVALO_DE_ROTINA: f64 = 300.0;
pub const MINIMO_DE_AMOSTRAS: usize = 3;
pub const RAZAO_MINIMA: f64 = 0.96;
/// Medido nesta máquina (PresentMon 2.6.0 no dwm, `presentmon::tests::vivo_conta_o_dwm`): os quadros chegam de 3,1 a
/// 3,9 s depois de apresentados (lote do ETW mais o que o PresentMon segura para calcular as métricas). Cinco dão folga.
pub const ATRASO_DA_CONTAGEM: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Fase {
    Gerando {
        proxima: f64,
    },
    MedindoComGeracao {
        desde: f64,
        ate: f64,
    },
    MedindoSemGeracao {
        com: (f64, f64),
        ate: f64,
    },
    /// Geração de volta; esperando a contagem das duas janelas chegar.
    Contando {
        com: (f64, f64),
        sem: (f64, f64),
        pronto: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acao {
    Nada,
    Pausar,
    Retomar,
    Desligar,
}

#[derive(Debug, Clone)]
pub struct Guarda {
    fase: Fase,
    pub razoes: Vec<f64>,
    /// Conferências descartadas seguidas (jogo parado, fonte sem contagem): cada uma dobra a espera da próxima.
    falhas: u32,
}

impl Guarda {
    pub fn nova(agora: f64) -> Guarda {
        Guarda {
            fase: Fase::Gerando {
                proxima: agora + INTERVALO_INICIAL,
            },
            razoes: Vec::new(),
            falhas: 0,
        }
    }

    pub fn pausada(&self) -> bool {
        matches!(self.fase, Fase::MedindoSemGeracao { .. })
    }

    pub fn razao_media(&self) -> Option<f64> {
        (!self.razoes.is_empty())
            .then(|| self.razoes.iter().sum::<f64>() / self.razoes.len() as f64)
    }

    /// `contar(de, ate)`: quadros do jogo no intervalo, no mesmo relógio de `agora`. `None`: a fonte parou de contar
    /// (a comparação é descartada, nunca vira zero).
    pub fn passo(&mut self, agora: f64, contar: impl Fn(f64, f64) -> Option<u64>) -> Acao {
        match self.fase {
            Fase::Gerando { proxima } if agora >= proxima => {
                self.fase = Fase::MedindoComGeracao {
                    desde: agora,
                    ate: agora + JANELA,
                };
                Acao::Nada
            }
            Fase::MedindoComGeracao { desde, ate } if agora >= ate => {
                self.fase = Fase::MedindoSemGeracao {
                    com: (desde, agora),
                    ate: agora + JANELA,
                };
                Acao::Pausar
            }
            Fase::MedindoSemGeracao { com, ate } if agora >= ate => {
                let desde = com.1;
                self.fase = Fase::Contando {
                    com,
                    sem: (desde, agora),
                    pronto: agora + ATRASO_DA_CONTAGEM,
                };
                Acao::Retomar
            }
            Fase::Contando { com, sem, pronto } if agora >= pronto => {
                let fps = |(a, b): (f64, f64)| contar(a, b).map(|n| n as f64 / (b - a).max(1e-3));
                // Jogo parado (menu, carregamento) não serve de comparação.
                let valeu = match (fps(com), fps(sem)) {
                    (Some(fps_com), Some(fps_sem)) if fps_sem >= 15.0 && fps_com >= 15.0 => {
                        self.razoes.push(fps_com / fps_sem);
                        true
                    }
                    _ => false,
                };
                self.falhas = if valeu { 0 } else { (self.falhas + 1).min(10) };
                let intervalo = if self.razoes.len() < MINIMO_DE_AMOSTRAS {
                    (INTERVALO_INICIAL * 2f64.powi(self.falhas as i32)).min(INTERVALO_DE_ROTINA)
                } else {
                    INTERVALO_DE_ROTINA
                };
                self.fase = Fase::Gerando {
                    proxima: agora + intervalo,
                };
                if self.razoes.len() >= MINIMO_DE_AMOSTRAS
                    && self.razao_media().is_some_and(|r| r < RAZAO_MINIMA)
                {
                    Acao::Desligar
                } else {
                    Acao::Nada
                }
            }
            _ => Acao::Nada,
        }
    }
}

/// Os instantes dos quadros do jogo, para `contar(de, ate)`. Guarda só os últimos segundos.
#[derive(Debug, Default)]
pub struct LinhaDoTempo {
    instantes: std::collections::VecDeque<f64>,
}

impl LinhaDoTempo {
    pub const GUARDA_S: f64 = 15.0;

    pub fn registrar(&mut self, instante: f64) {
        self.instantes.push_back(instante);
        while self
            .instantes
            .front()
            .is_some_and(|t| instante - *t > Self::GUARDA_S)
        {
            self.instantes.pop_front();
        }
    }

    pub fn contar(&self, de: f64, ate: f64) -> u64 {
        self.instantes
            .iter()
            .filter(|t| **t >= de && **t < ate)
            .count() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O jogo roda a `fps_com` gerando e `fps_sem` pausado; `atraso` simula o ETW entregando depois.
    fn simular(fps_com: f64, fps_sem: f64, segundos: f64, atraso: f64) -> (Guarda, Vec<Acao>) {
        let mut g = Guarda::nova(0.0);
        let mut linha = LinhaDoTempo::default();
        let mut pendentes: std::collections::VecDeque<f64> = Default::default();
        let mut acoes = Vec::new();
        let passo = 0.001;
        let mut t = 0.0;
        let mut proximo_quadro = 0.0;
        while t < segundos {
            if t >= proximo_quadro {
                pendentes.push_back(t);
                proximo_quadro = t + 1.0 / if g.pausada() { fps_sem } else { fps_com };
            }
            while pendentes.front().is_some_and(|q| t - *q >= atraso) {
                linha.registrar(pendentes.pop_front().unwrap());
            }
            let a = g.passo(t, |de, ate| Some(linha.contar(de, ate)));
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
        let (g, acoes) = simular(70.0, 70.0, 120.0, 0.0);
        assert!(!acoes.contains(&Acao::Desligar));
        assert!(acoes.iter().filter(|a| **a == Acao::Pausar).count() >= MINIMO_DE_AMOSTRAS);
        assert!((g.razao_media().unwrap() - 1.0).abs() < 0.03);
    }

    #[test]
    fn perda_real_desliga_em_poucas_rodadas() {
        let (g, acoes) = simular(62.0, 70.0, 120.0, 0.0);
        assert_eq!(acoes.last(), Some(&Acao::Desligar));
        assert_eq!(g.razoes.len(), MINIMO_DE_AMOSTRAS);
    }

    /// Com o atraso do ETW, a conta imediata veria a janela sem geração quase vazia e desligaria à toa.
    #[test]
    fn a_contagem_atrasada_nao_vira_perda_falsa() {
        let (g, acoes) = simular(70.0, 70.0, 120.0, 1.0);
        assert!(!acoes.contains(&Acao::Desligar), "{:?}", g.razoes);
        assert!((g.razao_media().unwrap() - 1.0).abs() < 0.03);

        let (_, acoes) = simular(60.0, 70.0, 120.0, 1.0);
        assert_eq!(acoes.last(), Some(&Acao::Desligar));

        // O pior atraso medido no PresentMon real.
        let (g, acoes) = simular(70.0, 70.0, 120.0, 3.9);
        assert!(!acoes.contains(&Acao::Desligar), "{:?}", g.razoes);
        let (_, acoes) = simular(60.0, 70.0, 120.0, 3.9);
        assert_eq!(acoes.last(), Some(&Acao::Desligar));
    }

    #[test]
    fn perda_pequena_dentro_da_tolerancia_nao_desliga() {
        let (_, acoes) = simular(68.5, 70.0, 120.0, 0.0);
        assert!(!acoes.contains(&Acao::Desligar));
    }

    #[test]
    fn jogo_parado_nao_conta() {
        let (g, acoes) = simular(5.0, 5.0, 60.0, 0.0);
        assert!(g.razoes.is_empty());
        assert!(!acoes.contains(&Acao::Desligar));
    }

    #[test]
    fn fonte_que_parou_nao_vira_zero() {
        let mut g = Guarda::nova(0.0);
        let mut t = 0.0;
        while t < 60.0 {
            assert_ne!(g.passo(t, |_, _| None), Acao::Desligar);
            t += 0.01;
        }
        assert!(g.razoes.is_empty());
    }

    /// O log do FiveM: a conferência que não valia (fonte sem contagem) repetia a cada 13 s, cada vez com o tranco.
    #[test]
    fn conferencia_que_falha_espera_cada_vez_mais() {
        let mut g = Guarda::nova(0.0);
        let mut pausas = 0;
        let mut t = 0.0;
        while t < 600.0 {
            if g.passo(t, |_, _| None) == Acao::Pausar {
                pausas += 1;
            }
            t += 0.01;
        }
        // 5, 10, 20, 40, 80, 160, 300…: umas 6 em 10 minutos, contra ~45 antes.
        assert!(pausas <= 7, "{} pausas", pausas);
    }

    #[test]
    fn depois_de_confirmar_quase_nao_pausa() {
        let (_, acoes) = simular(70.0, 70.0, 600.0, 1.0);
        let pausas = acoes.iter().filter(|a| **a == Acao::Pausar).count();
        assert!(
            pausas <= MINIMO_DE_AMOSTRAS + 2,
            "{} pausas em 10 min",
            pausas
        );
    }

    #[test]
    fn decide_antes_de_quarenta_segundos() {
        let (_, acoes) = simular(50.0, 70.0, 40.0, 1.0);
        assert_eq!(acoes.last(), Some(&Acao::Desligar));
    }

    #[test]
    fn a_linha_do_tempo_esquece_o_velho() {
        let mut l = LinhaDoTempo::default();
        for i in 0..100 {
            l.registrar(i as f64 * 0.5);
        }
        assert_eq!(l.contar(0.0, 10.0), 0, "mais de 15 s atrás já saiu");
        assert_eq!(l.contar(45.0, 50.0), 10);
    }
}
