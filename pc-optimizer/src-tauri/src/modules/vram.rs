// Pressão de memória de vídeo. Porcentagem de dedicada NÃO é pressão: o driver é um cache e não devolve o que
// não faz falta, então 7,5 de 8 GB é normal. Pressão é a dedicada no fim E derramando para a memória do sistema
// acima do PISO (o menor derramamento desta máquina em repouso; piso não observado é "não sei", nunca zero).
// Integrada tem estado próprio: usar memória do sistema é o projeto dela.

use serde::{Deserialize, Serialize};

use super::telemetry::Telemetry;

/// Generoso porque não produz veredito sozinho: só habilita a segunda pergunta.
pub const DEDICADA_NO_FIM_PCT: f64 = 90.0;

/// Abaixo de 200 MB é uma aba a mais no navegador, não transbordo da placa.
pub const TRANSBORDO_QUE_CONTA_GB: f64 = 0.20;

pub const PLACA_EM_REPOUSO_PCT: f64 = 30.0;

/// Uma leitura só pode pegar um programa recém-fechado cuja memória o driver ainda não devolveu.
pub const AMOSTRAS_PARA_PISO: usize = 5;

/// Integradas chamam um pedaço simbólico de dedicado; nenhuma dedicada de verdade tem menos de meio giga.
pub const SEM_DEDICADA_ATE_GB: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Estado {
    NaoAvaliado,
    PlacaIntegrada,
    Folgada,
    /// O estado normal depois de um tempo de jogo; dito ao cliente, evita o ajuste desnecessário.
    CacheCheio,
    Transbordando,
    /// Outra coisa usando memória compartilhada (captura, navegador): o módulo não atribui ao jogo.
    DerramaSemPressao,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conselho {
    /// O que derramou acima do piso é exatamente o que não coube.
    pub liberar_gb: f64,
    pub texto: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Analise {
    pub estado: Estado,
    pub dedicada_pct: Option<f64>,
    pub folga_gb: Option<f64>,
    pub derramado_gb: Option<f64>,
    pub piso_gb: Option<f64>,
    pub falta: Vec<String>,
    pub explicacao: String,
    pub conselho: Option<Conselho>,
}

/// Medida desta máquina, não configuração: não há valor padrão que sirva.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Piso {
    minimo_gb: Option<f64>,
    amostras: usize,
}

impl Piso {
    /// Leitura com a placa carregada não pode baixar o piso: o transbordo passaria a ser medido contra ele mesmo.
    pub fn observar(&mut self, compartilhada_gb: Option<f64>, gpu_pct: Option<f64>) {
        let (Some(gb), Some(uso)) = (compartilhada_gb, gpu_pct) else {
            return;
        };

        if !gb.is_finite() || gb < 0.0 || uso > PLACA_EM_REPOUSO_PCT {
            return;
        }

        self.amostras += 1;
        self.minimo_gb = Some(match self.minimo_gb {
            Some(m) => m.min(gb),
            None => gb,
        });
    }

    pub fn valor(&self) -> Option<f64> {
        if self.amostras >= AMOSTRAS_PARA_PISO {
            self.minimo_gb
        } else {
            None
        }
    }

    pub fn faltam(&self) -> usize {
        AMOSTRAS_PARA_PISO.saturating_sub(self.amostras)
    }
}

pub fn avaliar(t: &Telemetry, piso: &Piso) -> Analise {
    let usada = t.value("vram.used");
    let total = t.value("vram.total");
    let compartilhada = t.value("vram.shared_used");
    let piso_gb = piso.valor();

    let mut falta = Vec::new();

    // Integrada primeiro: senão sairia acusada de transbordo pelo próprio projeto.
    if total.is_some_and(|tot| tot <= SEM_DEDICADA_ATE_GB) {
        return Analise {
            estado: Estado::PlacaIntegrada,
            dedicada_pct: None,
            folga_gb: None,
            derramado_gb: None,
            piso_gb,
            falta,
            explicacao: "Esta máquina usa vídeo integrado: a memória da placa é a memória do \
                 sistema, por projeto. A conta de transbordo não se aplica aqui — o que limita \
                 é a RAM total e a velocidade dela."
                .to_string(),
            conselho: None,
        };
    }

    let (Some(usada), Some(total)) = (usada, total) else {
        if usada.is_none() {
            falta.push("vram.used".to_string());
        }
        if total.is_none() {
            falta.push("vram.total".to_string());
        }
        return Analise {
            estado: Estado::NaoAvaliado,
            dedicada_pct: None,
            folga_gb: None,
            derramado_gb: None,
            piso_gb,
            falta,
            explicacao: "Sem o uso e o total da memória da placa não há o que avaliar.".to_string(),
            conselho: None,
        };
    };

    let pct = usada / total * 100.0;
    let folga = (total - usada).max(0.0);
    let no_fim = pct >= DEDICADA_NO_FIM_PCT;

    let derramado = match (compartilhada, piso_gb) {
        (Some(agora), Some(p)) => Some((agora - p).max(0.0)),
        _ => {
            if compartilhada.is_none() {
                falta.push("vram.shared_used".to_string());
            } else {
                falta.push(format!(
                    "piso da memória compartilhada: faltam {} leituras com a placa em repouso",
                    piso.faltam()
                ));
            }
            None
        }
    };

    let transbordou = derramado.is_some_and(|d| d >= TRANSBORDO_QUE_CONTA_GB);

    let (estado, explicacao, conselho) = match (no_fim, derramado.filter(|_| transbordou)) {
        (true, Some(d)) => (
            Estado::Transbordando,
            format!(
                "A memória da placa acabou: {usada:.1} de {total:.1} GB em uso, e {d:.1} GB de \
                 textura foram parar na memória do sistema. O que está do outro lado do PCIe é \
                 lido muito mais devagar, e é daí que vem o engasgo ao entrar numa área nova."
            ),
            Some(Conselho {
                liberar_gb: d,
                texto: format!(
                    "Precisa deixar de caber na placa cerca de {d:.1} GB — é exatamente o que \
                     derramou. Textura e distância de visão são o que ocupa mais; resolução de \
                     tela quase não entra nesta conta."
                ),
            }),
        ),
        (false, Some(d)) => (
            Estado::DerramaSemPressao,
            format!(
                "Há {d:.1} GB em memória compartilhada, mas a memória da placa NÃO está no fim \
                 ({pct:.0}% de {total:.1} GB). Isto não é o jogo transbordando: é outro programa \
                 usando memória de vídeo pelo sistema — captura de tela, navegador com aceleração \
                 ou uma segunda placa."
            ),
            None,
        ),
        (true, None) => (
            Estado::CacheCheio,
            format!(
                "A memória da placa está em {pct:.0}%, e isso não é um problema. O driver não \
                 devolve textura que já carregou enquanto ninguém precisa do espaço — placa cheia \
                 depois de um tempo de jogo é o estado normal. Sem derramamento para a memória do \
                 sistema, não há o que ajustar aqui."
            ),
            None,
        ),
        (false, None) => (
            Estado::Folgada,
            format!("Memória da placa com folga: {folga:.1} GB livres de {total:.1} GB."),
            None,
        ),
    };

    Analise {
        estado,
        dedicada_pct: Some(pct),
        folga_gb: Some(folga),
        derramado_gb: derramado,
        piso_gb,
        falta,
        explicacao,
        conselho,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::telemetry::{Metric, Unit};

    fn com(pares: &[(&str, f64)]) -> Telemetry {
        let mut t = Telemetry::new(0, None);
        for (id, v) in pares {
            let unidade = match *id {
                "vram.usage" | "gpu.usage" => Unit::Percent,
                _ => Unit::Gigabytes,
            };
            t.set(id, Metric::measured(*v, unidade, "teste"));
        }
        t.finish(0)
    }

    fn piso_de(gb: f64) -> Piso {
        let mut p = Piso::default();
        for _ in 0..AMOSTRAS_PARA_PISO {
            p.observar(Some(gb), Some(5.0));
        }
        p
    }

    /// A regressão que originou o módulo: 94% sem derramar é cache cheio, não "baixe a textura".
    #[test]
    fn dedicada_cheia_sem_derramar_nao_e_problema() {
        let t = com(&[
            ("vram.used", 7.5),
            ("vram.total", 8.0),
            ("vram.shared_used", 0.4),
        ]);

        let a = avaliar(&t, &piso_de(0.4));

        assert_eq!(a.estado, Estado::CacheCheio);
        assert!(a.conselho.is_none(), "não há o que ajustar");
        assert!(a.explicacao.contains("normal"));
    }

    #[test]
    fn dedicada_cheia_derramando_e_transbordo() {
        let t = com(&[
            ("vram.used", 7.8),
            ("vram.total", 8.0),
            ("vram.shared_used", 1.9),
        ]);

        let a = avaliar(&t, &piso_de(0.4));

        assert_eq!(a.estado, Estado::Transbordando);
        let c = a.conselho.expect("conselho");
        assert!((c.liberar_gb - 1.5).abs() < 1e-9, "{}", c.liberar_gb);
    }

    #[test]
    fn derramamento_igual_ao_piso_nao_conta() {
        let t = com(&[
            ("vram.used", 7.9),
            ("vram.total", 8.0),
            ("vram.shared_used", 1.6),
        ]);

        let a = avaliar(&t, &piso_de(1.6));

        assert_eq!(a.estado, Estado::CacheCheio);
        assert_eq!(a.derramado_gb, Some(0.0));
    }

    #[test]
    fn sem_piso_nao_inventa_transbordo() {
        let t = com(&[
            ("vram.used", 7.8),
            ("vram.total", 8.0),
            ("vram.shared_used", 3.0),
        ]);

        let a = avaliar(&t, &Piso::default());

        assert_eq!(a.estado, Estado::CacheCheio);
        assert_eq!(a.derramado_gb, None);
        assert!(
            a.falta.iter().any(|f| f.contains("piso")),
            "a ausência do piso tem de ser declarada: {:?}",
            a.falta
        );
    }

    #[test]
    fn derramar_com_dedicada_folgada_nao_e_o_jogo() {
        let t = com(&[
            ("vram.used", 3.0),
            ("vram.total", 8.0),
            ("vram.shared_used", 2.0),
        ]);

        let a = avaliar(&t, &piso_de(0.3));

        assert_eq!(a.estado, Estado::DerramaSemPressao);
        assert!(a.conselho.is_none());
    }

    #[test]
    fn integrada_nao_e_acusada_de_transbordo() {
        let t = com(&[
            ("vram.used", 0.1),
            ("vram.total", 0.128),
            ("vram.shared_used", 6.0),
        ]);

        let a = avaliar(&t, &piso_de(1.0));

        assert_eq!(a.estado, Estado::PlacaIntegrada);
        assert!(a.conselho.is_none());
    }

    #[test]
    fn sem_medida_nao_avalia() {
        let a = avaliar(&Telemetry::new(0, None).finish(0), &Piso::default());

        assert_eq!(a.estado, Estado::NaoAvaliado);
        assert!(a.falta.contains(&"vram.used".to_string()));
        assert!(a.falta.contains(&"vram.total".to_string()));
    }

    #[test]
    fn piso_ignora_leitura_com_a_placa_ocupada() {
        let mut p = Piso::default();
        for _ in 0..AMOSTRAS_PARA_PISO {
            p.observar(Some(2.0), Some(5.0));
        }
        p.observar(Some(0.1), Some(97.0));

        assert_eq!(p.valor(), Some(2.0));
    }

    #[test]
    fn piso_exige_repeticao_antes_de_valer() {
        let mut p = Piso::default();
        p.observar(Some(0.5), Some(2.0));

        assert_eq!(p.valor(), None);
        assert_eq!(p.faltam(), AMOSTRAS_PARA_PISO - 1);
    }
}
