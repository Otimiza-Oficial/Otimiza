// Perfis de otimização
//
// O catálogo tem 35 otimizações. Para quem entende, é poder de escolha; para o
// cliente que abriu o programa pela primeira vez, é uma lista intimidante em que
// ele vai marcar tudo — inclusive o que não serve para o uso dele, e inclusive o
// que troca segurança por FPS.
//
// A resposta óbvia seria um botão "otimizar tudo", que é o que o mercado faz. A
// resposta honesta é perguntar PARA QUE serve este PC, porque a resposta muda o
// que vale a pena. Desligar a indexação de busca é excelente num PC de jogos e
// péssimo num PC de escritório, onde a pessoa procura arquivo o dia inteiro.
//
// Um perfil aqui é uma recomendação, não um pacote fechado: ele marca os itens
// na lista e a pessoa continua vendo, e podendo desmarcar, cada um deles.
//
// REGRA QUE NÃO SE QUEBRA: nenhum perfil inclui otimização que troca segurança
// por desempenho. Desligar as proteções da CPU rende FPS de verdade, e por isso
// mesmo não pode entrar num botão que a pessoa aperta sem ler. Quem quiser essa
// troca marca na mão, lendo o aviso. Existe um teste garantindo isto.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// O que este perfil deliberadamente NÃO faz, e por quê. Todo perfil tem
    /// uma renúncia; esconder isso seria vender pacote fechado como se fosse
    /// escolha informada.
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
            "background_apps_off",
            "disable_widgets",
            "disable_copilot",
            "disable_telemetry",
            "telemetry_policy",
            "stop_sponsored_apps",
            "start_menu_web_search_off",
            "disable_startup_delay",
            "disable_xbox_services",
            "disable_gamedvr",
            "delivery_optimization_off",
            "disable_reserved_storage",
            "power_high_performance",
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
            "gpu_hardware_scheduling",
            "system_responsiveness_gaming",
            "mmcss_games",
            "mouse_precision_off",
            "foreground_priority",
            "disable_core_parking",
            "processor_min_state",
            "disable_power_throttling",
            "power_high_performance",
            "gpu_msi_mode",
            "network_low_latency",
            "nic_power_saving_off",
            "remove_forced_hpet",
            "clear_boot_limits",
            "disable_xbox_services",
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
            "background_apps_off",
            "disable_widgets",
            "disable_telemetry",
            "telemetry_policy",
            "stop_sponsored_apps",
            "start_menu_web_search_off",
            "disable_startup_delay",
            "disable_gamedvr",
            "disable_xbox_services",
            "delivery_optimization_off",
            "power_high_performance",
        ],
    },
    ProfileInfo {
        id: "privacidade",
        name: "Privacidade",
        description:
            "Corta a coleta de dados e a propaganda embutida no sistema. O ganho \
             de desempenho é pequeno e honestamente secundário — o motivo aqui é \
             outro.",
        tradeoff:
            "Ganho de velocidade quase nulo. Se o seu problema é PC lento, este \
             não é o perfil que resolve.",
        optimization_ids: &[
            "disable_telemetry",
            "telemetry_policy",
            "stop_sponsored_apps",
            "start_menu_web_search_off",
            "background_apps_off",
            "disable_widgets",
            "delivery_optimization_off",
        ],
    },
];

/// Perfil pelo id. A interface trabalha com a lista inteira, mas a busca por id
/// existe para quando o motor precisar validar um perfil vindo de fora.
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
        // Um id com erro de digitação some em silêncio: o perfil marca uma
        // otimização a menos e ninguém percebe até o cliente reclamar que não
        // melhorou. Este teste é o que impede isso.
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
        // A regra que dá nome ao produto. Desligar as proteções da CPU rende
        // FPS de verdade — e por isso mesmo não pode entrar num botão que a
        // pessoa aperta sem ler o aviso.
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
        // Perfil marca caixas numa lista; apagar não tem volta e não pode
        // acontecer por causa de um clique em "PC fraco".
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

        // A promessa está escrita na renúncia do perfil; o teste garante que a
        // promessa e a lista não se separem com o tempo.
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

        // O público que motivou o produto. Se o perfil dele não tiver o que
        // libera memória, ele não serve para nada.
        for essencial in ["background_apps_off", "visual_effects_performance"] {
            assert!(fraco.optimization_ids.contains(&essencial));
        }

        // E não deve carregar ajuste que só faz sentido em jogo.
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
