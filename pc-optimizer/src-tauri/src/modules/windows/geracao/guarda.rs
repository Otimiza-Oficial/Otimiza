// A GUARDA DE FPS: o gerador usa a mesma placa que o jogo, e nunca pode tirar quadro real dele. De tempos em
// tempos a geração pausa e o FPS DO JOGO é medido com e sem ela, colados no tempo; a decisão usa a média de várias
// comparações. Perdeu mais que a tolerância: o gerador desliga e diz por quê.
//
// A contagem vem de fora (`contar(de, ate)`), por janela de tempo: pelo PresentMon no processo do jogo, os quadros
// chegam com atraso (o ETW entrega em lotes), por isso a conta só é feita `ATRASO_DA_CONTAGEM` depois de a janela
// sem geração fechar. Sem PresentMon, conta a captura da tela, como antes.

/// 1 s com e 1 s sem (1.0.7; era 1,5 s): o tranco da conferência é o tempo sem o Frame Boost, e um segundo a 60 fps já
/// são 60 quadros por lado. A decisão continua pela média de várias comparações.
pub const JANELA: f64 = 1.0;
pub const INTERVALO_INICIAL: f64 = 5.0;
/// Cada conferência pausa o Frame Boost por `JANELA` segundos, e a pessoa vê o tranco (o FPS do jogo sozinho por um
/// instante). Com 30 s, e a conferência que falhava repetindo a cada 13 s para sempre, o log de uma sessão do FiveM
/// mostrou a sobreposição escondida 15% do tempo. Depois das três primeiras, uma a cada 5 minutos.
pub const INTERVALO_DE_ROTINA: f64 = 300.0;
/// Com as três últimas comparações sem perda real (o jogo manteve 98% do FPS ou mais), o gerador já provou que não tira
/// FPS nesta partida: a pausa de conferência (o tranco de 1,5 s) passa a cada 15 min. Vale nos dois modos: no leve, a
/// régua de desligar (75%) não é folga; perder 20% e espaçar a conferência seria vigiar menos justo quando tira mais.
pub const INTERVALO_DE_ROTINA_FOLGADO: f64 = 900.0;
pub const SEM_PERDA_REAL: f64 = 0.98;

/// **Pura.** O intervalo entre conferências depois das três primeiras.
pub fn intervalo_de_rotina(razoes: &[f64], _minima: f64) -> f64 {
    let ultimas = &razoes[razoes.len().saturating_sub(MINIMO_DE_AMOSTRAS)..];
    if ultimas.len() == MINIMO_DE_AMOSTRAS && ultimas.iter().all(|r| *r >= SEM_PERDA_REAL) {
        INTERVALO_DE_ROTINA_FOLGADO
    } else {
        INTERVALO_DE_ROTINA
    }
}
pub const MINIMO_DE_AMOSTRAS: usize = 3;
pub const RAZAO_MINIMA: f64 = 0.96;
/// Já no modo leve, a régua para desligar de vez. Com 0,96 nas duas, o log do dono mostrou o gerador desligando com o
/// jogo mantendo 95,5% do FPS: a tela perdia o 2× inteiro (uns 80% de quadros a mais) por 4,5% do jogo. Com 0,90, um
/// cliente com a placa no limite via o Frame Boost sumir minutos depois de ligar e o jogo "voltar ao normal": com 2×,
/// 80% do jogo ainda é 1,6× na tela. Só abaixo de 75% (a tela perde metade do ganho e o mouse pesa) desliga.
pub const RAZAO_MINIMA_LEVE: f64 = 0.75;
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
    /// Abaixo desta razão média, desliga: `RAZAO_MINIMA`, ou `RAZAO_MINIMA_LEVE` depois do modo leve.
    minima: f64,
}

impl Guarda {
    pub fn nova(agora: f64) -> Guarda {
        Guarda {
            fase: Fase::Gerando {
                proxima: agora + INTERVALO_INICIAL,
            },
            razoes: Vec::new(),
            falhas: 0,
            minima: RAZAO_MINIMA,
        }
    }

    /// A guarda depois do modo leve: confere de novo do zero, com a régua de `RAZAO_MINIMA_LEVE`.
    pub fn nova_leve(agora: f64) -> Guarda {
        Guarda {
            minima: RAZAO_MINIMA_LEVE,
            ..Guarda::nova(agora)
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
                    intervalo_de_rotina(&self.razoes, self.minima)
                };
                self.fase = Fase::Gerando {
                    proxima: agora + intervalo,
                };
                if self.razoes.len() >= MINIMO_DE_AMOSTRAS
                    && self.razao_media().is_some_and(|r| r < self.minima)
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

    /// Os instantes de `de` até `ate` (exclusive), em ordem.
    pub fn entre(&self, de: f64, ate: f64) -> Vec<f64> {
        self.instantes.iter().copied().filter(|t| *t >= de && *t < ate).collect()
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

    #[test]
    fn conferencia_espaca_quando_o_gerador_ja_provou_folga() {
        assert_eq!(intervalo_de_rotina(&[1.0, 0.99, 1.01], RAZAO_MINIMA_LEVE), INTERVALO_DE_ROTINA_FOLGADO);
        assert_eq!(intervalo_de_rotina(&[1.0, 1.0, 0.97], RAZAO_MINIMA), INTERVALO_DE_ROTINA, "perto da régua: 5 min");
        assert_eq!(
            intervalo_de_rotina(&[0.80, 0.80, 0.80], RAZAO_MINIMA_LEVE),
            INTERVALO_DE_ROTINA,
            "no leve, perder 20% não é folga: continua a cada 5 min"
        );
        assert_eq!(intervalo_de_rotina(&[1.0, 1.0], RAZAO_MINIMA_LEVE), INTERVALO_DE_ROTINA, "menos de três: 5 min");
        // Só as três últimas contam: uma comparação ruim antiga não prende em 5 min para sempre.
        assert_eq!(intervalo_de_rotina(&[0.7, 1.0, 1.0, 1.0], RAZAO_MINIMA_LEVE), INTERVALO_DE_ROTINA_FOLGADO);
    }

    /// O jogo roda a `fps_com` gerando e `fps_sem` pausado; `atraso` simula o ETW entregando depois.
    fn simular(fps_com: f64, fps_sem: f64, segundos: f64, atraso: f64) -> (Guarda, Vec<Acao>) {
        simular_com(Guarda::nova(0.0), fps_com, fps_sem, segundos, atraso)
    }

    fn simular_com(mut g: Guarda, fps_com: f64, fps_sem: f64, segundos: f64, atraso: f64) -> (Guarda, Vec<Acao>) {
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
    fn no_modo_leve_perder_pouco_nao_desliga_mas_perder_muito_sim() {
        // 92% do FPS: antes desligava (o log do dono mostrou até com 95,5%); no modo leve, o gerador fica.
        let (_, acoes) = simular_com(Guarda::nova_leve(0.0), 64.4, 70.0, 120.0, 0.0);
        assert!(!acoes.contains(&Acao::Desligar), "{:?}", acoes);
        // Fora do modo leve, a mesma perda ainda pede o modo leve primeiro.
        let (_, acoes) = simular(64.4, 70.0, 120.0, 0.0);
        assert!(acoes.contains(&Acao::Desligar));
        // 82% do FPS (placa no limite): com 2× a tela ainda ganha 1,6×; no modo leve, o gerador fica.
        let (_, acoes) = simular_com(Guarda::nova_leve(0.0), 57.4, 70.0, 600.0, 0.0);
        assert!(!acoes.contains(&Acao::Desligar), "{:?}", acoes);
        // 65% do FPS: o jogo sente de verdade; no modo leve, desliga.
        let (_, acoes) = simular_com(Guarda::nova_leve(0.0), 45.5, 70.0, 120.0, 0.0);
        assert!(acoes.contains(&Acao::Desligar));
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
