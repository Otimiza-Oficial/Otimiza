// "Este recurso funciona aqui?" num lugar só. A tela nunca decide sozinha se mostra um botão: pergunta aqui, e recebe
// o estado e o motivo em português. Regras só do que está documentado pelo fabricante; o que depende de prova em
// hardware real sai como `Parcial` com o motivo, nunca como `Suportado` por otimismo.

use serde::{Deserialize, Serialize};

use super::hardware::{Fabricante, Familia};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Recurso {
    /// A geração do próprio jogo (DLSS, FSR ou XeSS com geração de quadros).
    FrameBoostDoJogo,
    /// NVIDIA Smooth Motion, no driver.
    FrameBoostSmoothMotion,
    /// AMD Fluid Motion Frames, no driver.
    FrameBoostAfmf,
    /// O gerador do Otimiza, por fora do jogo.
    FrameBoostOtimiza,
    BaixaLatenciaDoDriver,
    LimitadorDoDriver,
    MiraNaTela,
    MiraNaConfiguracaoDoJogo,
    CoresDoMonitor,
}

impl Recurso {
    pub fn nome(self, fabricante: Fabricante) -> &'static str {
        match self {
            Recurso::FrameBoostDoJogo => "Geração de quadros do jogo",
            Recurso::FrameBoostSmoothMotion => "NVIDIA Smooth Motion",
            Recurso::FrameBoostAfmf => "AMD Fluid Motion Frames",
            Recurso::FrameBoostOtimiza => "Gerador do Otimiza",
            Recurso::BaixaLatenciaDoDriver => match fabricante {
                Fabricante::Amd => "Radeon Anti-Lag",
                Fabricante::Nvidia => "Modo de baixa latência",
                _ => "Baixa latência",
            },
            Recurso::LimitadorDoDriver => match fabricante {
                Fabricante::Amd => "Frame Rate Target Control",
                _ => "Limitador de FPS do driver",
            },
            Recurso::MiraNaTela => "Mira na tela",
            Recurso::MiraNaConfiguracaoDoJogo => "Mira do próprio jogo",
            Recurso::CoresDoMonitor => match fabricante {
                Fabricante::Nvidia => "Vibração digital",
                _ => "Saturação",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModoDeTela {
    TelaCheia,
    SemBordas,
    Janela,
    Desconhecido,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Api {
    DirectX12,
    DirectX11,
    DirectX9,
    Vulkan,
    OpenGl,
    Desconhecida,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "estado", content = "motivo")]
pub enum Suporte {
    Suportado,
    /// Funciona com condição ou ainda sem prova em hardware real; o motivo diz qual.
    Parcial(String),
    NaoSuportado(String),
    PrecisaAtualizar(String),
}

impl Suporte {
    pub fn usavel(&self) -> bool {
        matches!(self, Suporte::Suportado | Suporte::Parcial(_))
    }
}

/// Tudo o que as regras precisam saber. Montado por quem chama (hardware em cache, jogo detectado, anticheat).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contexto {
    pub fabricante: Fabricante,
    pub familia: Familia,
    pub hz_do_monitor: Option<u32>,
    pub modo_de_tela: ModoDeTela,
    pub api: Api,
    /// O motivo do anticheat para não sobrepor janela (`anticheat::permite(SobreporJanela)`), se houver.
    pub sobreposicao_barrada: Option<String>,
    /// O jogo aceita janela por cima, mas com ressalva (servidor de RP, anticheat que não documenta): vira `Parcial`.
    pub sobreposicao_aviso: Option<String>,
    pub jogo_tem_geracao_propria: bool,
    pub jogo_tem_mira_configuravel: bool,
    /// O ADLX (AMD) ou o IGCL (Intel) carregou. `None`: ainda não se tentou.
    pub biblioteca_do_driver: Option<bool>,
    /// O interruptor experimental `smooth_motion_drs` está ligado: o Otimiza liga o Smooth Motion no perfil do jogo.
    pub smooth_motion_pelo_otimiza: bool,
}

impl Contexto {
    pub fn so_hardware(fabricante: Fabricante, familia: Familia) -> Self {
        Contexto {
            fabricante,
            familia,
            hz_do_monitor: None,
            modo_de_tela: ModoDeTela::Desconhecido,
            api: Api::Desconhecida,
            sobreposicao_barrada: None,
            sobreposicao_aviso: None,
            jogo_tem_geracao_propria: false,
            jogo_tem_mira_configuravel: false,
            biblioteca_do_driver: None,
            smooth_motion_pelo_otimiza: false,
        }
    }
}

fn so_directx_moderno(api: Api, quem: &str) -> Option<Suporte> {
    match api {
        Api::DirectX9 | Api::OpenGl => Some(Suporte::NaoSuportado(format!(
            "{} só funciona em jogos DirectX 11, DirectX 12 ou Vulkan.",
            quem
        ))),
        _ => None,
    }
}

fn biblioteca_ausente(c: &Contexto) -> Option<Suporte> {
    (c.biblioteca_do_driver == Some(false)).then(|| {
        Suporte::PrecisaAtualizar(format!(
            "O driver da {} não respondeu ao Otimiza. Atualize o driver e abra o Otimiza de novo.",
            c.fabricante.nome()
        ))
    })
}

/// Pura.
pub fn avaliar(recurso: Recurso, c: &Contexto) -> Suporte {
    use Suporte::*;

    match recurso {
        Recurso::FrameBoostDoJogo => {
            if c.jogo_tem_geracao_propria {
                Suportado
            } else {
                NaoSuportado("Este jogo não tem geração de quadros própria.".to_string())
            }
        }

        Recurso::FrameBoostSmoothMotion => {
            if c.fabricante != Fabricante::Nvidia {
                return NaoSuportado("Smooth Motion é das placas NVIDIA.".to_string());
            }
            if !matches!(c.familia, Familia::Rtx40 | Familia::Rtx50) {
                return NaoSuportado("Smooth Motion só existe nas RTX 40 e RTX 50.".to_string());
            }
            if let Some(nao) = so_directx_moderno(c.api, "O Smooth Motion") {
                return nao;
            }
            if c.smooth_motion_pelo_otimiza {
                return Parcial(
                    "Experimental: o Otimiza liga no perfil do jogo no driver, com um ajuste que a NVIDIA não                      documenta. Vale quando o jogo abrir de novo, e se desfaz em um clique."
                        .to_string(),
                );
            }
            // Não há ajuste público para ligar por fora: o Otimiza mostra onde ligar no NVIDIA App e mede.
            Parcial("Liga no NVIDIA App, no perfil do jogo. O Otimiza mostra onde e mede o resultado.".to_string())
        }

        Recurso::FrameBoostAfmf => {
            if c.fabricante != Fabricante::Amd {
                return NaoSuportado("Fluid Motion Frames é das placas AMD.".to_string());
            }
            if !matches!(c.familia, Familia::Rx6000 | Familia::Rx7000 | Familia::Rx9000) {
                return NaoSuportado("Fluid Motion Frames só existe a partir das RX 6000.".to_string());
            }
            if let Some(nao) = biblioteca_ausente(c) {
                return nao;
            }
            if let Some(nao) = so_directx_moderno(c.api, "O Fluid Motion Frames") {
                return nao;
            }
            match c.modo_de_tela {
                ModoDeTela::Janela => Parcial(
                    "Em janela o AMD Fluid Motion Frames não gera. Use tela cheia ou sem bordas.".to_string(),
                ),
                _ => Suportado,
            }
        }

        Recurso::FrameBoostOtimiza => {
            if let Some(motivo) = &c.sobreposicao_barrada {
                return NaoSuportado(motivo.clone());
            }
            match c.modo_de_tela {
                // Tela cheia exclusiva não passa pela composição do Windows, e a captura não vê o jogo.
                ModoDeTela::TelaCheia => Parcial(
                    "Precisa do jogo em janela ou sem bordas. O Otimiza troca o modo e desfaz ao desligar.".to_string(),
                ),
                _ => match &c.sobreposicao_aviso {
                    Some(aviso) => Parcial(aviso.clone()),
                    None => Suportado,
                },
            }
        }

        Recurso::BaixaLatenciaDoDriver => match c.fabricante {
            Fabricante::Nvidia => match c.api {
                // Documentado pela NVIDIA: em DirectX 12 e Vulkan o jogo controla a fila; só o Reflex do jogo ajuda.
                Api::DirectX12 | Api::Vulkan => Parcial(
                    "Neste jogo quem controla o atraso é o próprio jogo. Se ele tiver NVIDIA Reflex, ligue no menu."
                        .to_string(),
                ),
                _ => Suportado,
            },
            Fabricante::Amd => biblioteca_ausente(c).unwrap_or(Suportado),
            Fabricante::Intel => biblioteca_ausente(c)
                .unwrap_or_else(|| Parcial("Ainda sem prova numa placa Intel.".to_string())),
            Fabricante::Desconhecido => NaoSuportado("Não identifiquei a placa de vídeo.".to_string()),
        },

        Recurso::LimitadorDoDriver => match c.fabricante {
            Fabricante::Nvidia => Suportado,
            Fabricante::Amd | Fabricante::Intel => biblioteca_ausente(c).unwrap_or(Suportado),
            Fabricante::Desconhecido => NaoSuportado("Não identifiquei a placa de vídeo.".to_string()),
        },

        Recurso::MiraNaTela => {
            if let Some(motivo) = &c.sobreposicao_barrada {
                return NaoSuportado(motivo.clone());
            }
            if c.jogo_tem_mira_configuravel {
                return Parcial("Este jogo tem mira própria configurável: o Otimiza grava nela, sem sobreposição.".to_string());
            }
            match &c.sobreposicao_aviso {
                Some(aviso) => Parcial(aviso.clone()),
                None => Suportado,
            }
        }

        Recurso::MiraNaConfiguracaoDoJogo => {
            if c.jogo_tem_mira_configuravel {
                Suportado
            } else {
                NaoSuportado("Este jogo não deixa configurar a mira por arquivo.".to_string())
            }
        }

        Recurso::CoresDoMonitor => match c.fabricante {
            Fabricante::Amd => biblioteca_ausente(c).unwrap_or(Suportado),
            // No IGCL a cor passa pela transformação de pixels, ainda sem prova numa placa Intel.
            Fabricante::Intel => {
                Parcial("Saturação nas placas Intel em teste: por enquanto, brilho, contraste e gama do Windows.".to_string())
            }
            // Vibração digital pelo NVAPI (provada numa GTX 1650) e brilho, contraste e gama pela rampa. Só o matiz falta,
            // e a tela desliga esse controle.
            Fabricante::Nvidia => biblioteca_ausente(c).unwrap_or(Suportado),
            Fabricante::Desconhecido => {
                Parcial("Sem o driver da placa, só brilho, contraste e gama do Windows.".to_string())
            }
        },
    }
}

/// O Frame Boost automático: a geração do jogo é a melhor (usa os vetores de movimento dele), depois a do driver,
/// e por último a do Otimiza. Nunca devolve algo que `avaliar` recusou.
pub fn melhor_frame_boost(c: &Contexto) -> Option<(Recurso, Suporte)> {
    [
        Recurso::FrameBoostDoJogo,
        Recurso::FrameBoostSmoothMotion,
        Recurso::FrameBoostAfmf,
        Recurso::FrameBoostOtimiza,
    ]
    .into_iter()
    .map(|r| (r, avaliar(r, c)))
    .find(|(_, s)| matches!(s, Suporte::Suportado))
    .or_else(|| {
        [
            Recurso::FrameBoostSmoothMotion,
            Recurso::FrameBoostAfmf,
            Recurso::FrameBoostOtimiza,
        ]
        .into_iter()
        .map(|r| (r, avaliar(r, c)))
        .find(|(_, s)| s.usavel())
    })
}

/// O multiplicador mais alto que faz sentido: nunca acima do monitor e, nos geradores de driver, só 2×.
pub fn multiplicadores(recurso: Recurso, fps_nativo: Option<f64>, hz: Option<u32>) -> Vec<u8> {
    let maximo_do_gerador = match recurso {
        Recurso::FrameBoostSmoothMotion | Recurso::FrameBoostAfmf => 2,
        Recurso::FrameBoostOtimiza => 4,
        _ => return Vec::new(),
    };

    (2..=maximo_do_gerador)
        .filter(|m| match (fps_nativo, hz) {
            // Mais quadros que o monitor mostra é trabalho jogado fora; 5% de folga para a variação do nativo.
            (Some(n), Some(h)) if n > 0.0 => n * (*m as f64) <= h as f64 * 1.05 || *m == 2,
            _ => true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(f: Fabricante, fam: Familia) -> Contexto {
        Contexto {
            api: Api::DirectX11,
            modo_de_tela: ModoDeTela::SemBordas,
            ..Contexto::so_hardware(f, fam)
        }
    }

    #[test]
    fn smooth_motion_so_em_rtx_40_e_50_e_nunca_como_suportado_por_fora() {
        assert!(matches!(
            avaliar(
                Recurso::FrameBoostSmoothMotion,
                &ctx(Fabricante::Nvidia, Familia::Rtx40)
            ),
            Suporte::Parcial(_)
        ));
        assert!(matches!(
            avaliar(
                Recurso::FrameBoostSmoothMotion,
                &ctx(Fabricante::Nvidia, Familia::Rtx30)
            ),
            Suporte::NaoSuportado(_)
        ));
        assert!(matches!(
            avaliar(
                Recurso::FrameBoostSmoothMotion,
                &ctx(Fabricante::Amd, Familia::Rx7000)
            ),
            Suporte::NaoSuportado(_)
        ));
    }

    #[test]
    fn afmf_a_partir_da_rx_6000_e_nao_em_janela() {
        assert_eq!(
            avaliar(
                Recurso::FrameBoostAfmf,
                &ctx(Fabricante::Amd, Familia::Rx6000)
            ),
            Suporte::Suportado
        );
        assert!(matches!(
            avaliar(
                Recurso::FrameBoostAfmf,
                &ctx(Fabricante::Amd, Familia::RxAnterior)
            ),
            Suporte::NaoSuportado(_)
        ));
        let em_janela = Contexto {
            modo_de_tela: ModoDeTela::Janela,
            ..ctx(Fabricante::Amd, Familia::Rx7000)
        };
        assert!(matches!(
            avaliar(Recurso::FrameBoostAfmf, &em_janela),
            Suporte::Parcial(_)
        ));
    }

    #[test]
    fn driver_amd_sem_adlx_pede_atualizacao() {
        let sem = Contexto {
            biblioteca_do_driver: Some(false),
            ..ctx(Fabricante::Amd, Familia::Rx6000)
        };
        assert!(matches!(
            avaliar(Recurso::FrameBoostAfmf, &sem),
            Suporte::PrecisaAtualizar(_)
        ));
        assert!(matches!(
            avaliar(Recurso::BaixaLatenciaDoDriver, &sem),
            Suporte::PrecisaAtualizar(_)
        ));
        assert!(matches!(
            avaliar(Recurso::CoresDoMonitor, &sem),
            Suporte::PrecisaAtualizar(_)
        ));
    }

    #[test]
    fn jogo_dx9_e_opengl_nao_tem_geracao_de_driver() {
        for api in [Api::DirectX9, Api::OpenGl] {
            let c = Contexto {
                api,
                ..ctx(Fabricante::Amd, Familia::Rx7000)
            };
            assert!(matches!(
                avaliar(Recurso::FrameBoostAfmf, &c),
                Suporte::NaoSuportado(_)
            ));
        }
    }

    #[test]
    fn anticheat_que_barra_sobreposicao_tira_gerador_e_mira() {
        let c = Contexto {
            sobreposicao_barrada: Some("Vanguard aberto".to_string()),
            ..ctx(Fabricante::Nvidia, Familia::Rtx30)
        };
        assert_eq!(
            avaliar(Recurso::FrameBoostOtimiza, &c),
            Suporte::NaoSuportado("Vanguard aberto".to_string())
        );
        assert_eq!(
            avaliar(Recurso::MiraNaTela, &c),
            Suporte::NaoSuportado("Vanguard aberto".to_string())
        );
        assert_eq!(
            melhor_frame_boost(&c),
            None,
            "RTX 30 sem gerador próprio e sem sobreposição: nada a oferecer"
        );
    }

    #[test]
    fn auto_prefere_a_geracao_do_jogo_depois_o_driver_depois_o_otimiza() {
        let com_jogo = Contexto {
            jogo_tem_geracao_propria: true,
            ..ctx(Fabricante::Amd, Familia::Rx7000)
        };
        assert_eq!(
            melhor_frame_boost(&com_jogo).unwrap().0,
            Recurso::FrameBoostDoJogo
        );
        assert_eq!(
            melhor_frame_boost(&ctx(Fabricante::Amd, Familia::Rx7000))
                .unwrap()
                .0,
            Recurso::FrameBoostAfmf
        );
        assert_eq!(
            melhor_frame_boost(&ctx(Fabricante::Nvidia, Familia::Rtx30))
                .unwrap()
                .0,
            Recurso::FrameBoostOtimiza
        );
    }

    #[test]
    fn rtx_40_cai_no_otimiza_suportado_antes_do_smooth_motion_parcial() {
        // O suportado vence o parcial: o Smooth Motion depende da pessoa ligar no NVIDIA App.
        assert_eq!(
            melhor_frame_boost(&ctx(Fabricante::Nvidia, Familia::Rtx40))
                .unwrap()
                .0,
            Recurso::FrameBoostOtimiza
        );
    }

    #[test]
    fn tela_cheia_exclusiva_deixa_o_gerador_do_otimiza_parcial() {
        let c = Contexto {
            modo_de_tela: ModoDeTela::TelaCheia,
            ..ctx(Fabricante::Intel, Familia::Arc)
        };
        assert!(matches!(
            avaliar(Recurso::FrameBoostOtimiza, &c),
            Suporte::Parcial(_)
        ));
    }

    #[test]
    fn aviso_de_sobreposicao_deixa_mira_e_gerador_parciais_com_o_motivo() {
        let c = Contexto {
            sobreposicao_aviso: Some("servidor de RP".to_string()),
            ..ctx(Fabricante::Nvidia, Familia::Rtx30)
        };
        assert_eq!(
            avaliar(Recurso::MiraNaTela, &c),
            Suporte::Parcial("servidor de RP".to_string())
        );
        assert_eq!(
            avaliar(Recurso::FrameBoostOtimiza, &c),
            Suporte::Parcial("servidor de RP".to_string())
        );
    }

    #[test]
    fn baixa_latencia_da_nvidia_em_dx12_e_parcial() {
        let c = Contexto {
            api: Api::DirectX12,
            ..ctx(Fabricante::Nvidia, Familia::Rtx40)
        };
        assert!(matches!(
            avaliar(Recurso::BaixaLatenciaDoDriver, &c),
            Suporte::Parcial(_)
        ));
        assert_eq!(
            avaliar(
                Recurso::BaixaLatenciaDoDriver,
                &ctx(Fabricante::Nvidia, Familia::Rtx40)
            ),
            Suporte::Suportado
        );
    }

    #[test]
    fn nome_do_recurso_muda_com_a_marca() {
        assert_eq!(
            Recurso::BaixaLatenciaDoDriver.nome(Fabricante::Amd),
            "Radeon Anti-Lag"
        );
        assert_eq!(
            Recurso::CoresDoMonitor.nome(Fabricante::Nvidia),
            "Vibração digital"
        );
        assert_eq!(Recurso::CoresDoMonitor.nome(Fabricante::Amd), "Saturação");
    }

    #[test]
    fn mira_vai_para_o_jogo_quando_ele_deixa() {
        let c = Contexto {
            jogo_tem_mira_configuravel: true,
            ..ctx(Fabricante::Nvidia, Familia::Rtx40)
        };
        assert_eq!(
            avaliar(Recurso::MiraNaConfiguracaoDoJogo, &c),
            Suporte::Suportado
        );
        assert!(matches!(
            avaliar(Recurso::MiraNaTela, &c),
            Suporte::Parcial(_)
        ));
    }

    #[test]
    fn multiplicador_nunca_passa_do_monitor_nem_do_gerador() {
        assert_eq!(
            multiplicadores(Recurso::FrameBoostAfmf, Some(52.0), Some(240)),
            vec![2]
        );
        assert_eq!(
            multiplicadores(Recurso::FrameBoostOtimiza, Some(52.0), Some(240)),
            vec![2, 3, 4]
        );
        assert_eq!(
            multiplicadores(Recurso::FrameBoostOtimiza, Some(52.0), Some(144)),
            vec![2]
        );
        assert_eq!(
            multiplicadores(Recurso::FrameBoostOtimiza, Some(40.0), Some(144)),
            vec![2, 3]
        );
        assert!(multiplicadores(Recurso::CoresDoMonitor, None, None).is_empty());
    }

    #[test]
    fn cores_por_marca() {
        assert_eq!(
            avaliar(
                Recurso::CoresDoMonitor,
                &ctx(Fabricante::Amd, Familia::Rx6000)
            ),
            Suporte::Suportado
        );
        assert!(matches!(
            avaliar(
                Recurso::CoresDoMonitor,
                &ctx(Fabricante::Intel, Familia::Arc)
            ),
            Suporte::Parcial(_)
        ));
        // Vibração digital provada numa GTX 1650: vale para a NVIDIA.
        assert_eq!(
            avaliar(
                Recurso::CoresDoMonitor,
                &ctx(Fabricante::Nvidia, Familia::Rtx40)
            ),
            Suporte::Suportado
        );
        assert!(matches!(
            avaliar(
                Recurso::CoresDoMonitor,
                &ctx(Fabricante::Desconhecido, Familia::Desconhecida)
            ),
            Suporte::Parcial(_)
        ));
    }
}
