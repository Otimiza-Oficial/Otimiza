// Dois AJUSTES do próprio Otimiza que se anulam, se somam mal ou só fazem sentido juntos (dois PROGRAMAS
// brigando é `conflicts.rs`). Entra só com o MECANISMO escrito, não "podem interferir"; há trava para isso. Só
// avisa: a decisão continua de quem usa.

use serde::{Deserialize, Serialize};

use super::grupos::{grupo_de, Grupo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tipo {
    /// O último a rodar ganha: o resultado depende da ORDEM.
    MesmoLugar,
    SeAnulam,
    DependeDoOutro,
    JuntosCustamCaro,
}

impl Tipo {
    pub fn rotulo(self) -> &'static str {
        match self {
            Tipo::MesmoLugar => "escrevem no mesmo lugar",
            Tipo::SeAnulam => "um anula o outro",
            Tipo::DependeDoOutro => "um depende do outro",
            Tipo::JuntosCustamCaro => "juntos custam caro",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conflito {
    pub id: &'static str,
    pub um: &'static str,
    pub outro: &'static str,
    pub tipo: Tipo,
    pub mecanismo: &'static str,
    /// Nunca "escolha um": qual escolher, e por quê.
    pub conselho: &'static str,
}

pub static CONHECIDOS: &[Conflito] = &[
    Conflito {
        id: "hpet_e_boot",
        um: "remove_forced_hpet",
        outro: "clear_boot_limits",
        tipo: Tipo::MesmoLugar,
        mecanismo: "Os dois escrevem na configuração de inicialização pelo `bcdedit`, e os \
                    dois exigem reiniciar. Aplicados na mesma sessão, o resultado final \
                    depende de qual rodou por último — e como os dois só valem depois do \
                    reinício, uma medição feita antes dele mede a máquina de antes, sem \
                    nenhum dos dois.",
        conselho: "Não é problema aplicar os dois: eles escrevem valores diferentes e não \
                   se sobrepõem. O cuidado é com a MEDIÇÃO — reinicie antes de medir, senão \
                   o teste do grupo H vai comparar duas vezes a mesma configuração.",
    },
    Conflito {
        id: "compressao_e_sysmain",
        um: "disable_memory_compression",
        outro: "disable_sysmain",
        tipo: Tipo::JuntosCustamCaro,
        mecanismo: "Os dois tiram muletas de memória ao mesmo tempo. Sem a compressão, o \
                    que não cabe vai para o disco; sem o SysMain, o Windows para de \
                    adiantar leitura do que costuma ser usado. Cada um sozinho é uma troca \
                    defensável em máquina com folga — juntos, numa máquina apertada, \
                    somam-se e o sintoma é engasgo de carregamento no meio da partida.",
        conselho: "Com 16 GB ou menos, aplique no máximo um dos dois e meça. Se o jogo está \
                   num disco mecânico, NÃO desligue o SysMain: é justamente nesse disco que \
                   ele adianta leitura.",
    },
    Conflito {
        id: "prioridade_e_mmcss",
        um: "foreground_priority",
        outro: "mmcss_games",
        tipo: Tipo::MesmoLugar,
        mecanismo: "Os dois mexem em quem ganha processador, por caminhos diferentes: o \
                    primeiro muda a fatia da janela em primeiro plano, e o segundo eleva a \
                    prioridade das tarefas de jogo pelo agendador de multimídia. Aplicados \
                    juntos, o efeito de um esconde o do outro numa medição — e se o FPS \
                    piorar, não dá para saber qual dos dois foi.",
        conselho: "Os dois estão no MESMO grupo do protocolo (B) justamente por isso: eles \
                   são testados juntos ou nenhum, e o veredito é do par. Testar um de cada \
                   vez aqui daria uma resposta que não se confirma.",
    },
    Conflito {
        id: "hags_e_msi",
        um: "gpu_hardware_scheduling",
        outro: "gpu_msi_mode",
        tipo: Tipo::MesmoLugar,
        mecanismo: "Os dois mudam como o trabalho chega à placa de vídeo — um passa a fila \
                    para a própria GPU gerenciar, o outro muda como ela avisa o processador \
                    de que terminou. Os dois exigem reiniciar e os dois são dos que mais \
                    variam entre máquinas. Aplicados juntos e o FPS caindo, o culpado é um \
                    dos dois e não há como saber qual sem desfazer um.",
        conselho: "Estão no mesmo grupo (C) e são testados juntos. Se o grupo C piorar, \
                   desfaça o agendamento por hardware primeiro: ele é o que tem o histórico \
                   de custar quadro em combinação de placa e driver.",
    },
    Conflito {
        id: "plano_e_throttling",
        um: "plano_otimiza",
        outro: "disable_power_throttling",
        tipo: Tipo::DependeDoOutro,
        mecanismo: "A limitação de energia por processo age sobre programas que o Windows \
                    considera de segundo plano. Com o plano OTIMIZA ativo — estado mínimo \
                    em 100% e preferência de desempenho no máximo —, boa parte do que ela \
                    faria já não acontece. Desligá-la sem o plano ativo tem efeito bem \
                    maior do que com ele.",
        conselho: "Aplique o plano primeiro e meça. Se o grupo A já resolveu, a limitação \
                   por processo vira um ajuste sem efeito visível nesta máquina — e saber \
                   disso é melhor que aplicar e achar que ajudou.",
    },
];

pub fn entre(ids: &[String]) -> Vec<&'static Conflito> {
    CONHECIDOS
        .iter()
        .filter(|c| {
            ids.iter().any(|id| id == c.um) && ids.iter().any(|id| id == c.outro)
        })
        .collect()
}

/// Os que atravessam grupos quebram o protocolo A/B: o teste do segundo mede o efeito do primeiro, e o
/// veredito sai trocado. Dentro do mesmo grupo não: o grupo é aplicado inteiro.
pub fn atravessam_grupos() -> Vec<&'static Conflito> {
    CONHECIDOS
        .iter()
        .filter(|c| {
            let (a, b) = (grupo_de(c.um), grupo_de(c.outro));
            a.is_some() && b.is_some() && a != b
        })
        .collect()
}

pub fn grupos_ligados(conflito: &Conflito) -> Option<(Grupo, Grupo)> {
    let a = grupo_de(conflito.um)?;
    let b = grupo_de(conflito.outro)?;

    (a != b).then_some((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::windows::catalog::CATALOG;

    /// Id que não existe é um aviso que nunca aparece, e a ausência de aviso parece paz.
    #[test]
    fn todo_conflito_aponta_para_ajustes_que_existem() {
        for c in CONHECIDOS {
            for id in [c.um, c.outro] {
                assert!(
                    CATALOG.iter().any(|s| s.id == id),
                    "o conflito `{}` cita `{id}`, que não existe no catálogo",
                    c.id
                );
            }
        }
    }

    #[test]
    fn todo_conflito_diz_o_mecanismo_e_nao_um_palpite() {
        for c in CONHECIDOS {
            assert!(
                c.mecanismo.len() >= 180,
                "`{}` não explica o mecanismo, só rotula",
                c.id
            );

            let baixo = c.mecanismo.to_lowercase();
            assert!(
                !baixo.contains("podem interferir") && !baixo.contains("pode interferir"),
                "`{}` usa a frase que se escreve quando não se sabe o mecanismo",
                c.id
            );
        }
    }

    #[test]
    fn todo_conselho_diz_qual_escolher_e_por_que() {
        for c in CONHECIDOS {
            assert!(c.conselho.len() >= 100, "`{}` sem conselho de verdade", c.id);

            let baixo = c.conselho.to_lowercase();
            assert!(
                !baixo.starts_with("escolha um"),
                "`{}` devolve a decisão sem ajudar a tomá-la",
                c.id
            );
        }
    }

    #[test]
    fn nenhum_conflito_e_de_um_ajuste_com_ele_mesmo() {
        for c in CONHECIDOS {
            assert_ne!(c.um, c.outro, "`{}` é um ajuste brigando consigo", c.id);
        }
    }

    #[test]
    fn nenhum_par_esta_repetido() {
        let mut pares: Vec<(&str, &str)> = CONHECIDOS
            .iter()
            .map(|c| {
                if c.um < c.outro {
                    (c.um, c.outro)
                } else {
                    (c.outro, c.um)
                }
            })
            .collect();

        let antes = pares.len();
        pares.sort();
        pares.dedup();

        assert_eq!(pares.len(), antes, "há par repetido, com dois conselhos diferentes");
    }

    #[test]
    fn acha_o_conflito_numa_selecao() {
        let selecao = vec![
            "disable_memory_compression".to_string(),
            "disable_sysmain".to_string(),
            "disable_transparency".to_string(),
        ];

        let achados = entre(&selecao);

        assert_eq!(achados.len(), 1);
        assert_eq!(achados[0].id, "compressao_e_sysmain");
    }

    /// Um dos dois sozinho não é conflito: alarme falso gasta a confiança do aviso de verdade.
    #[test]
    fn um_dos_dois_sozinho_nao_e_conflito() {
        let so_um = vec!["disable_memory_compression".to_string()];
        assert!(entre(&so_um).is_empty());
    }

    #[test]
    fn selecao_vazia_nao_inventa_conflito() {
        assert!(entre(&[]).is_empty());
    }

    /// Se esta lista mudar, o texto que explica o protocolo ao cliente muda junto.
    #[test]
    fn so_um_conflito_atravessa_grupos() {
        let ids: Vec<&str> = atravessam_grupos().iter().map(|c| c.id).collect();

        assert_eq!(
            ids,
            vec!["compressao_e_sysmain"],
            "a lista de conflitos que atravessam grupos mudou; se foi de propósito, o \
             texto que explica o protocolo ao cliente precisa mudar junto"
        );
    }

    #[test]
    fn o_par_de_energia_esta_no_mesmo_grupo() {
        let par = CONHECIDOS
            .iter()
            .find(|c| c.id == "plano_e_throttling")
            .expect("continua na lista");

        assert_eq!(grupo_de(par.um), Some(Grupo::Energia));
        assert_eq!(grupo_de(par.outro), Some(Grupo::Energia));
        assert!(grupos_ligados(par).is_none());
    }

    #[test]
    fn conflito_dentro_do_mesmo_grupo_nao_atravessa() {
        let dentro = CONHECIDOS
            .iter()
            .find(|c| c.id == "hags_e_msi")
            .expect("o par da placa de vídeo continua na lista");

        assert!(grupos_ligados(dentro).is_none(), "os dois são do grupo C");
        assert_eq!(grupo_de(dentro.um), Some(Grupo::Video));
        assert_eq!(grupo_de(dentro.outro), Some(Grupo::Video));
    }

    #[test]
    fn cada_tipo_tem_rotulo_proprio() {
        let mut rotulos = [
            Tipo::MesmoLugar.rotulo(),
            Tipo::SeAnulam.rotulo(),
            Tipo::DependeDoOutro.rotulo(),
            Tipo::JuntosCustamCaro.rotulo(),
        ]
        .to_vec();

        let antes = rotulos.len();
        rotulos.sort();
        rotulos.dedup();

        assert_eq!(rotulos.len(), antes);
    }
}
