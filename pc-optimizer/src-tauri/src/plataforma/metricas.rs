// O modelo único de desempenho da 3.3. Cada número carrega a ORIGEM: medido, estimado ou desconhecido. A tela nunca
// recebe um f64 solto, e por isso não tem como mostrar estimativa como se fosse medição.
//
// Separação que não pode se perder: FPS NATIVO (o que o jogo desenhou) e FPS NA TELA (o que chegou ao monitor, gerados
// incluídos). Otimização compara nativo com nativo. O Frame Boost mostra o que chegou à tela, e esse número só existe
// se foi medido.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origem {
    Medido,
    Estimado,
    Desconhecido,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Valor {
    pub valor: Option<f64>,
    pub origem: Origem,
}

impl Valor {
    pub fn medido(v: f64) -> Self {
        Valor {
            valor: Some(v).filter(|x| x.is_finite()),
            origem: Origem::Medido,
        }
    }

    pub fn estimado(v: f64) -> Self {
        Valor {
            valor: Some(v).filter(|x| x.is_finite()),
            origem: Origem::Estimado,
        }
    }

    pub fn desconhecido() -> Self {
        Valor {
            valor: None,
            origem: Origem::Desconhecido,
        }
    }

    /// `None` vira desconhecido, nunca zero.
    pub fn medido_ou_nada(v: Option<f64>) -> Self {
        v.map(Valor::medido).unwrap_or_else(Valor::desconhecido)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metricas {
    pub fps_nativo: Valor,
    pub fps_na_tela: Valor,
    pub fps_gerado: Valor,
    pub frametime_ms: Valor,
    pub low_1pct: Valor,
    pub low_01pct: Valor,
    pub p95_ms: Valor,
    pub p99_ms: Valor,
    /// Do quadro pronto até a tela, pelo PresentMon. NÃO é a latência de clique a pixel (essa só existe com os
    /// marcadores do Reflex no jogo): por isso sai sempre como ESTIMADO.
    pub latencia_ms: Valor,
    pub hz_do_monitor: Valor,
    pub engasgos_por_minuto: Valor,
    /// Quem gerou os quadros extras, quando se identificou ("AMD AFMF", "Otimiza"...).
    pub geradores: Vec<String>,
}

impl Metricas {
    pub fn vazia() -> Self {
        Metricas {
            fps_nativo: Valor::desconhecido(),
            fps_na_tela: Valor::desconhecido(),
            fps_gerado: Valor::desconhecido(),
            frametime_ms: Valor::desconhecido(),
            low_1pct: Valor::desconhecido(),
            low_01pct: Valor::desconhecido(),
            p95_ms: Valor::desconhecido(),
            p99_ms: Valor::desconhecido(),
            latencia_ms: Valor::desconhecido(),
            hz_do_monitor: Valor::desconhecido(),
            engasgos_por_minuto: Valor::desconhecido(),
            geradores: Vec::new(),
        }
    }

    /// Há geração de quadros acontecendo de verdade: mais quadros na tela do que o jogo desenhou, por margem que não
    /// é ruído de medição.
    pub fn com_geracao(&self) -> bool {
        match (self.fps_nativo.valor, self.fps_na_tela.valor) {
            (Some(n), Some(t)) if self.fps_na_tela.origem == Origem::Medido => t > n * 1.15,
            _ => false,
        }
    }

    /// Mais quadros chegando ao monitor do que ele atualiza por segundo: parte deles aparece só como pedaço de tela
    /// (tearing). A tela avisa, em vez de cortar o número (cortar seria inventar outro).
    pub fn excede_o_monitor(&self) -> bool {
        match (self.fps_na_tela.valor, self.hz_do_monitor.valor) {
            (Some(t), Some(hz)) => t > hz * 1.02,
            _ => false,
        }
    }
}

/// O número grande da tela. Com o Frame Boost ligado é o FPS NA TELA, e só se tiver sido medido (decisão do dono: o
/// nativo não aparece nessa tela). Sem medição, desconhecido: a tela mostra "medindo…", nunca nativo × multiplicador.
pub fn numero_principal(m: &Metricas, frame_boost_ligado: bool) -> Valor {
    if !frame_boost_ligado {
        return m.fps_nativo;
    }

    if m.fps_na_tela.origem == Origem::Medido {
        m.fps_na_tela
    } else {
        Valor::desconhecido()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Faixa {
    pub minimo: f64,
    pub maximo: f64,
    pub origem: Origem,
}

/// Quanto o Frame Boost deve entregar ANTES de ligar. Sempre faixa e sempre ESTIMADO. Gerar custa GPU: o jogo perde
/// de 3% a 15% do nativo (a guarda do gerador desliga acima de 4% medidos no nosso, mas drivers variam). O teto é o
/// monitor, porque o gerador nunca apresenta acima do Hz (`geracao::ritmo`).
pub fn estimar_frame_boost(nativo: f64, multiplicador: u8, hz: Option<f64>) -> Option<Faixa> {
    if !nativo.is_finite() || nativo <= 0.0 || !(2..=4).contains(&multiplicador) {
        return None;
    }

    let teto = hz
        .filter(|h| h.is_finite() && *h > 0.0)
        .unwrap_or(f64::INFINITY);
    let m = multiplicador as f64;
    let minimo = (nativo * 0.85 * m).min(teto);
    let maximo = (nativo * 0.97 * m).min(teto);

    Some(Faixa {
        minimo: minimo.floor(),
        maximo: maximo.ceil(),
        origem: Origem::Estimado,
    })
}

/// Ganho percentual entre duas medições. Só entre MEDIDOS: estimativa contra medição não é ganho.
pub fn ganho_pct(antes: Valor, depois: Valor) -> Option<f64> {
    if antes.origem != Origem::Medido || depois.origem != Origem::Medido {
        return None;
    }

    match (antes.valor, depois.valor) {
        (Some(a), Some(d)) if a > 0.0 => Some((d - a) / a * 100.0),
        _ => None,
    }
}

/// Da medição do PresentMon para o modelo. `hz` vem do monitor onde o jogo está (não do principal).
#[cfg(target_os = "windows")]
pub fn de_resumo(r: &crate::modules::windows::presentmon::Resumo, hz: Option<f64>) -> Metricas {
    let gerado = (r.fps_exibido - r.fps_do_jogo_medio).max(0.0);

    Metricas {
        fps_nativo: Valor::medido(r.fps_do_jogo_medio),
        fps_na_tela: Valor::medido(r.fps_exibido),
        fps_gerado: if r.quadros_gerados > 0 || gerado > r.fps_do_jogo_medio * 0.15 {
            Valor::medido(gerado)
        } else {
            Valor::medido(0.0)
        },
        frametime_ms: Valor::medido(r.quadro_medio_ms),
        low_1pct: Valor::medido_ou_nada(r.low_1pct),
        low_01pct: Valor::medido_ou_nada(r.low_01pct),
        p95_ms: Valor::medido(r.quadro_p95_ms),
        p99_ms: Valor::medido(r.quadro_p99_ms),
        latencia_ms: r
            .ate_a_tela_media_ms
            .map(Valor::estimado)
            .unwrap_or_else(Valor::desconhecido),
        hz_do_monitor: hz.map(Valor::medido).unwrap_or_else(Valor::desconhecido),
        engasgos_por_minuto: Valor::medido(r.engasgos_por_minuto),
        geradores: r.geradores.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn com(nativo: f64, tela: Option<f64>, hz: Option<f64>) -> Metricas {
        Metricas {
            fps_nativo: Valor::medido(nativo),
            fps_na_tela: Valor::medido_ou_nada(tela),
            hz_do_monitor: Valor::medido_ou_nada(hz),
            ..Metricas::vazia()
        }
    }

    #[test]
    fn frame_boost_ligado_mostra_o_fps_na_tela_e_nao_o_nativo() {
        let m = com(52.0, Some(204.0), Some(240.0));

        let principal = numero_principal(&m, true);
        assert_eq!(principal.valor, Some(204.0));
        assert_eq!(principal.origem, Origem::Medido);
    }

    #[test]
    fn frame_boost_sem_medicao_nunca_vira_nativo_vezes_multiplicador() {
        let m = com(52.0, None, Some(240.0));

        let principal = numero_principal(&m, true);
        assert_eq!(
            principal.valor, None,
            "sem medir, a tela mostra 'medindo', não 208"
        );
        assert_eq!(principal.origem, Origem::Desconhecido);
    }

    #[test]
    fn frame_boost_com_valor_estimado_nao_sobe_para_o_numero_principal() {
        let mut m = com(52.0, None, None);
        m.fps_na_tela = Valor::estimado(200.0);

        assert_eq!(numero_principal(&m, true).valor, None);
    }

    #[test]
    fn sem_frame_boost_o_numero_principal_e_o_nativo() {
        let m = com(67.0, Some(67.0), Some(144.0));
        assert_eq!(numero_principal(&m, false).valor, Some(67.0));
    }

    #[test]
    fn estimativa_e_faixa_estimada_e_respeita_o_monitor() {
        let f = estimar_frame_boost(52.0, 4, Some(240.0)).unwrap();
        assert_eq!(f.origem, Origem::Estimado);
        assert!(f.minimo < f.maximo);
        assert!(f.maximo <= 240.0);

        let em_60hz = estimar_frame_boost(52.0, 4, Some(60.0)).unwrap();
        assert_eq!(
            em_60hz.maximo, 60.0,
            "num monitor de 60 Hz não existe 204 na tela"
        );
    }

    #[test]
    fn estimativa_recusa_entrada_sem_sentido() {
        assert!(estimar_frame_boost(0.0, 2, None).is_none());
        assert!(estimar_frame_boost(f64::NAN, 2, None).is_none());
        assert!(estimar_frame_boost(60.0, 1, None).is_none());
        assert!(estimar_frame_boost(60.0, 5, None).is_none());
    }

    #[test]
    fn ganho_so_entre_medidos() {
        assert_eq!(
            ganho_pct(Valor::medido(52.0), Valor::medido(67.0)).map(|g| g.round()),
            Some(29.0)
        );
        assert_eq!(ganho_pct(Valor::medido(52.0), Valor::estimado(67.0)), None);
        assert_eq!(ganho_pct(Valor::desconhecido(), Valor::medido(67.0)), None);
        assert_eq!(ganho_pct(Valor::medido(0.0), Valor::medido(67.0)), None);
    }

    #[test]
    fn geracao_so_conta_com_margem_acima_do_ruido() {
        assert!(com(52.0, Some(104.0), None).com_geracao());
        assert!(!com(60.0, Some(62.0), None).com_geracao());
    }

    #[test]
    fn acima_do_monitor_e_avisado_e_nao_cortado() {
        let m = com(100.0, Some(204.0), Some(60.0));
        assert!(m.excede_o_monitor());
        assert_eq!(numero_principal(&m, true).valor, Some(204.0));
    }

    #[test]
    fn numero_infinito_vira_desconhecido() {
        assert_eq!(Valor::medido(f64::INFINITY).valor, None);
    }
}
