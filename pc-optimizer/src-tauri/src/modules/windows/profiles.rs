// Perfis: PARA QUE serve este PC muda o que vale a pena. Um perfil marca itens na lista, que a pessoa continua
// vendo e podendo desmarcar. Nenhum perfil inclui otimização que troca segurança por desempenho (há teste).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// O que o perfil NÃO faz, e por quê: esconder a renúncia seria vender pacote fechado.
    pub tradeoff: &'static str,
    pub optimization_ids: &'static [&'static str],
}

pub const PROFILES: &[ProfileInfo] = &[
    ProfileInfo {
        id: "pc_fraco",
        name: "PC fraco",
        description:
            "Para a máquina de 4 a 8 GB que trava ao abrir o navegador. Tudo aqui \
             ataca a mesma coisa: parar de gastar memória e disco com o que roda \
             sozinho em segundo plano, para sobrar máquina para o que você abriu.",
        tradeoff:
            "Deixa a interface mais seca: sem transparência, sem animação, sem \
             Widgets. Em PC fraco isso é o que devolve resposta ao clique — mas é \
             uma troca de aparência por velocidade, e você vai notar.",
        optimization_ids: &[
            "visual_effects_performance",
            "disable_transparency",
            "disable_widgets",
            "stop_sponsored_apps",
            "disable_gamedvr",
            "disable_reserved_storage",
            "plano_otimiza",
        ],
    },
    ProfileInfo {
        id: "jogos",
        name: "Jogos",
        description:
            "Foco em quadro estável e resposta do mouse, não em número de FPS \
             médio. Travada de meio segundo estraga mais partida que dez quadros \
             a menos, e é nela que estes ajustes mexem.",
        tradeoff:
            "Mantém o PC em desempenho máximo o tempo todo: gasta mais energia e \
             esquenta mais. Em notebook na bateria, isso encurta a autonomia de \
             forma bem visível.",
        optimization_ids: &[
            "disable_gamedvr",
            // `gpu_hardware_scheduling` saiu daqui depois da 2.1.0: pode custar quadro, e perfil é lote. Ver `RiscoDeFps`.
            "mouse_precision_off",
            "plano_otimiza",
            "remove_forced_hpet",
            "clear_boot_limits",
        ],
    },
    ProfileInfo {
        id: "trabalho",
        name: "Trabalho",
        description:
            "Para quem passa o dia em planilha, navegador e reunião. Tira o que \
             consome máquina em segundo plano e não mexe em nada que você use \
             para trabalhar.",
        tradeoff:
            "De propósito, NÃO desliga a indexação de busca nem a hibernação. As \
             duas rendem desempenho, mas quem procura arquivo o dia inteiro e \
             fecha o notebook sem salvar sente a falta muito mais do que sente o \
             ganho.",
        optimization_ids: &[
            "visual_effects_performance",
            "disable_widgets",
            "stop_sponsored_apps",
            "disable_gamedvr",
            "plano_otimiza",
        ],
    },
];

#[allow(dead_code)]
pub fn find(id: &str) -> Option<&'static ProfileInfo> {
    PROFILES.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::optimizer::Category;
    use crate::modules::windows::catalog::CATALOG;
    use std::collections::HashSet;

    #[test]
    fn todo_id_de_perfil_existe_no_catalogo() {
        // Um id com erro de digitação some em silêncio: o perfil marca uma otimização a menos.
        let conhecidos: HashSet<&str> = CATALOG.iter().map(|o| o.id).collect();

        for perfil in PROFILES {
            for id in perfil.optimization_ids {
                assert!(
                    conhecidos.contains(id),
                    "o perfil `{}` aponta para `{}`, que não existe no catálogo",
                    perfil.id,
                    id
                );
            }
        }
    }

    #[test]
    fn nenhum_perfil_troca_seguranca_por_desempenho() {
        for perfil in PROFILES {
            for id in perfil.optimization_ids {
                let spec = CATALOG.iter().find(|o| o.id == *id).unwrap();

                assert!(
                    !spec.security_tradeoff,
                    "o perfil `{}` inclui `{}`, que troca segurança por desempenho",
                    perfil.id,
                    id
                );
            }
        }
    }

    #[test]
    fn nenhum_perfil_apaga_arquivo() {
        for perfil in PROFILES {
            for id in perfil.optimization_ids {
                let spec = CATALOG.iter().find(|o| o.id == *id).unwrap();

                assert!(
                    spec.reversible,
                    "o perfil `{}` inclui `{}`, que não tem como ser desfeito",
                    perfil.id,
                    id
                );
            }
        }
    }

    #[test]
    fn nenhum_perfil_cita_item_que_fica_fora_do_lote() {
        // O perfil aplica pelo mesmo lote, que pula `catalog::FORA_DO_LOTE`: a prévia contaria um item que o clique
        // não aplica.
        for perfil in PROFILES {
            for id in perfil.optimization_ids {
                let spec = CATALOG.iter().find(|o| o.id == *id).unwrap();

                // Condicional entra quando a condição for medida; Expert e "só se pedir", nunca.
                assert!(
                    crate::modules::windows::catalog::entra_no_lote_se(spec, |_| true),
                    "o perfil `{}` cita `{}`, que o lote não aplica",
                    perfil.id,
                    id
                );
            }
        }
    }

    #[test]
    fn perfil_nao_repete_a_mesma_otimizacao() {
        for perfil in PROFILES {
            let unicos: HashSet<&&str> = perfil.optimization_ids.iter().collect();
            assert_eq!(
                unicos.len(),
                perfil.optimization_ids.len(),
                "o perfil `{}` repete alguma otimização",
                perfil.id
            );
        }
    }

    #[test]
    fn perfil_de_trabalho_nao_atrapalha_quem_trabalha() {
        let trabalho = find("trabalho").unwrap();

        for proibida in ["disable_search_indexing", "disable_hibernation"] {
            assert!(
                !trabalho.optimization_ids.contains(&proibida),
                "`{}` quebra a promessa escrita no perfil Trabalho",
                proibida
            );
        }
    }

    #[test]
    fn perfil_de_pc_fraco_ataca_memoria_e_segundo_plano() {
        let fraco = find("pc_fraco").unwrap();

        for essencial in ["disable_widgets", "visual_effects_performance"] {
            assert!(fraco.optimization_ids.contains(&essencial));
        }

        assert!(!fraco.optimization_ids.contains(&"mouse_precision_off"));
    }

    #[test]
    fn perfil_de_jogos_e_majoritariamente_de_jogos() {
        let jogos = find("jogos").unwrap();

        let de_jogo = jogos
            .optimization_ids
            .iter()
            .filter(|id| {
                CATALOG
                    .iter()
                    .find(|o| o.id == **id)
                    .map(|o| o.category == Category::Gaming)
                    .unwrap_or(false)
            })
            .count();

        assert!(
            de_jogo * 2 > jogos.optimization_ids.len(),
            "só {} de {} itens do perfil Jogos são de jogos",
            de_jogo,
            jogos.optimization_ids.len()
        );
    }

    #[test]
    fn todo_perfil_declara_o_que_abre_mao() {
        for perfil in PROFILES {
            assert!(
                perfil.tradeoff.len() > 40,
                "o perfil `{}` não diz o que deixa de fazer",
                perfil.id
            );
            assert!(!perfil.optimization_ids.is_empty());
        }
    }

    #[test]
    fn id_desconhecido_nao_devolve_perfil() {
        assert!(find("nao_existe").is_none());
        assert!(find("").is_none());
    }
}
