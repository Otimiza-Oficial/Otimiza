// Os grupos de ajuste, para testar um de cada vez: "qual destes grupos piorou meu PC?" se responde com quatro
// medições. O corte é por ONDE O AJUSTE TOCA, não pela categoria da tela. Grupo que exige reiniciar não pode ser
// revertido sozinho (o depois é outra sessão). O que cobre todos é testar UM GRUPO DE CADA VEZ. Aposentado não
// entra em grupo (só existe para o desfazer), e grupo que ficou sem item sai de `TODOS` mantendo a letra.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Grupo {
    Energia,
    Agendamento,
    Video,
    Memoria,
    Rede,
    Fundo,
    Visual,
    Boot,
    /// Não promete quadro: está aqui para poder ser descartado como suspeito.
    Higiene,
}

impl Grupo {
    /// B (Agendamento), D (Memória) e G (Visual) ficaram vazios com os cortes da 2.9 e da A3.2.
    pub const TODOS: &'static [Grupo] = &[
        Grupo::Energia,
        Grupo::Video,
        Grupo::Rede,
        Grupo::Fundo,
        Grupo::Boot,
        Grupo::Higiene,
    ];

    /// "O grupo C piorou aqui" cabe numa mensagem.
    pub fn letra(self) -> char {
        match self {
            Grupo::Energia => 'A',
            Grupo::Agendamento => 'B',
            Grupo::Video => 'C',
            Grupo::Memoria => 'D',
            Grupo::Rede => 'E',
            Grupo::Fundo => 'F',
            Grupo::Visual => 'G',
            Grupo::Boot => 'H',
            Grupo::Higiene => 'I',
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Grupo::Energia => "Energia",
            Grupo::Agendamento => "Quem ganha o processador",
            Grupo::Video => "Placa de vídeo",
            Grupo::Memoria => "Memória",
            Grupo::Rede => "Rede",
            Grupo::Fundo => "O que roda de fundo",
            Grupo::Visual => "Efeitos visuais",
            Grupo::Boot => "Configuração de inicialização",
            Grupo::Higiene => "Privacidade e higiene",
        }
    }

    pub fn descricao(self) -> &'static str {
        match self {
            Grupo::Energia => {
                "O plano de energia: frequência mínima do processador, preferência entre \
                 energia e desempenho e estacionamento de núcleos. É o grupo com o maior \
                 efeito medido na maioria das máquinas."
            }
            Grupo::Agendamento => {
                "Como o Windows reparte o processador entre o jogo e o resto: prioridade \
                 da janela em uso, perfil de multimídia, e a fatia reservada para tarefas \
                 de fundo. Pesa muito mais em processador de poucos núcleos."
            }
            Grupo::Video => {
                "O caminho até a placa de vídeo: quem gerencia a fila de trabalho dela e \
                 como ela avisa o processador. É o grupo com mais variação entre máquinas \
                 — há PC em que rende e PC em que custa quadro."
            }
            Grupo::Memoria => {
                "Compressão de memória. Troca processador por memória, e a troca só vale \
                 quando sobra memória DURANTE o jogo."
            }
            Grupo::Rede => {
                "A economia de energia da placa de rede, que faz a placa cochilar e perder \
                 pacote. Mexe em ping e em variação de ping, não em FPS."
            }
            Grupo::Fundo => {
                "As teclas de acessibilidade que o Windows liga sozinho por atalho \
                 (Filtragem de Teclas, Teclas de Aderência). Mexe no teclado responder na \
                 hora, não em FPS."
            }
            Grupo::Visual => {
                "Animação, transparência e sombra das janelas do Windows. Melhora a \
                 sensação de resposta da área de trabalho; dentro do jogo não muda nada."
            }
            Grupo::Boot => {
                "O que é decidido quando a máquina liga: temporizador, limites de núcleo e \
                 memória, virtualização de segurança. Exige reiniciar para valer."
            }
            Grupo::Higiene => {
                "A aceleração do mouse. Não promete quadro nenhum: muda a mira, e está separado justamente para poder ser descartado como \
                 suspeito quando alguma coisa piorar."
            }
        }
    }
}

/// Lista explícita: deduzir da categoria poria um ajuste novo no grupo errado, e o A/B apontaria o culpado
/// errado. Há trava exigindo que todo item de lote apareça aqui.
pub fn grupo_de(id: &str) -> Option<Grupo> {
    Some(match id {
        "plano_otimiza" => Grupo::Energia,
        "disable_power_throttling" => Grupo::Energia,
        "disable_hibernation" => Grupo::Energia,

        "system_responsiveness_gaming" => Grupo::Agendamento,
        "foreground_priority" => Grupo::Agendamento,
        "mmcss_games" => Grupo::Agendamento,

        "gpu_hardware_scheduling" => Grupo::Video,
        "gpu_msi_mode" => Grupo::Video,
        "disable_gamedvr" => Grupo::Video,
        "windowed_game_optimizations" => Grupo::Video,

        "disable_memory_compression" => Grupo::Memoria,

        "network_low_latency" => Grupo::Rede,
        "nic_power_saving_off" => Grupo::Rede,

        "disable_sysmain" => Grupo::Fundo,
        "disable_xbox_services" => Grupo::Fundo,
        "disable_search_indexing" => Grupo::Fundo,
        "background_apps_off" => Grupo::Fundo,
        "delivery_optimization_off" => Grupo::Fundo,
        "disable_startup_delay" => Grupo::Fundo,
        "disable_widgets" => Grupo::Fundo,
        "disable_copilot" => Grupo::Fundo,
        "stop_sponsored_apps" => Grupo::Fundo,
        "store_auto_download_off" => Grupo::Fundo,
        "maps_auto_update_off" => Grupo::Fundo,
        "remote_assistance_off" => Grupo::Fundo,
        "accessibility_keys_off" => Grupo::Fundo,
        "edge_background_off" => Grupo::Fundo,
        "error_reporting_off" => Grupo::Fundo,

        "visual_effects_performance" => Grupo::Visual,
        "disable_transparency" => Grupo::Visual,

        "clear_boot_limits" => Grupo::Boot,
        "remove_forced_hpet" => Grupo::Boot,
        // `disable_vbs` NÃO está aqui: troca segurança por desempenho, e o protocolo aplica grupos INTEIROS.

        "disable_telemetry" => Grupo::Higiene,
        "telemetry_policy" => Grupo::Higiene,
        "notifications_off" => Grupo::Higiene,
        "settings_sync_off" => Grupo::Higiene,
        "start_menu_web_search_off" => Grupo::Higiene,
        "mouse_precision_off" => Grupo::Higiene,
        "disable_reserved_storage" => Grupo::Higiene,

        "disable_vbs" => return None,

        _ => return None,
    })
}

/// NÃO é `entra_no_lote`: o lote exclui o que pode custar quadro; o A/B existe justamente para medir esses.
fn testavel(spec: &super::catalog::OptimizationSpec) -> bool {
    spec.reversible && !spec.security_tradeoff && !super::catalog::retirado(spec.id)
}

pub fn itens_do_grupo(grupo: Grupo) -> Vec<&'static str> {
    super::catalog::CATALOG
        .iter()
        .filter(|spec| testavel(spec))
        .filter(|spec| grupo_de(spec.id) == Some(grupo))
        .map(|spec| spec.id)
        .collect()
}

pub fn exige_reinicio(grupo: Grupo) -> bool {
    super::catalog::CATALOG
        .iter()
        .filter(|spec| testavel(spec))
        .filter(|spec| grupo_de(spec.id) == Some(grupo))
        .any(|spec| spec.requires_restart)
}

/// Só quando o teste cabe numa sessão: reverter comparando duas sessões é reverter em cima de ruído.
pub fn pode_reverter_sozinho(grupo: Grupo) -> bool {
    !exige_reinicio(grupo) && !itens_do_grupo(grupo).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::windows::catalog::CATALOG;

    /// Ajuste reversível fora de grupo é um ajuste que o A/B nunca testaria.

    #[test]
    fn os_grupos_que_cabem_numa_sessao_sao_conhecidos() {
        let cabem: Vec<char> = Grupo::TODOS
            .iter()
            .filter(|g| pode_reverter_sozinho(**g))
            .map(|g| g.letra())
            .collect();

        assert_eq!(
            cabem,
            vec!['A', 'E', 'F', 'I'],
            "a lista de grupos que dispensam reinício mudou; se foi de propósito, \
             atualize este teste E o texto que explica o protocolo ao cliente"
        );

        for g in [Grupo::Video, Grupo::Boot] {
            assert!(
                exige_reinicio(g),
                "o grupo {} deixou de exigir reinício — vale reconferir, porque isso \
                 mudaria o que o protocolo consegue reverter sozinho",
                g.letra()
            );
        }
    }

    #[test]
    fn todo_item_testavel_tem_grupo() {
        for spec in CATALOG {
            if !testavel(spec) {
                continue;
            }

            assert!(
                grupo_de(spec.id).is_some(),
                "`{}` entra no lote e não tem grupo — o A/B nunca o testaria",
                spec.id
            );
        }
    }

    #[test]
    fn aposentado_nao_entra_em_grupo() {
        for grupo in Grupo::TODOS {
            for id in itens_do_grupo(*grupo) {
                assert!(!crate::modules::windows::catalog::retirado(id), "`{id}` foi aposentado e está no grupo");
            }
        }
    }

    #[test]
    fn todo_id_com_grupo_existe_no_catalogo() {
        for grupo in Grupo::TODOS {
            for id in itens_do_grupo(*grupo) {
                assert!(
                    CATALOG.iter().any(|s| s.id == id),
                    "`{id}` está num grupo e não existe no catálogo"
                );
            }
        }
    }

    #[test]
    fn o_que_troca_seguranca_fica_fora_dos_grupos() {
        for spec in CATALOG {
            if spec.security_tradeoff {
                assert!(
                    grupo_de(spec.id).is_none(),
                    "`{}` troca segurança e entrou num grupo",
                    spec.id
                );
            }
        }
    }

    #[test]
    fn as_letras_sao_diferentes() {
        let mut letras: Vec<char> = Grupo::TODOS.iter().map(|g| g.letra()).collect();
        let antes = letras.len();

        letras.sort();
        letras.dedup();

        assert_eq!(letras.len(), antes, "duas letras repetidas");
        assert_eq!(antes, 6, "a tela fala em seis grupos");
    }

    #[test]
    fn nenhum_grupo_esta_vazio() {
        for grupo in Grupo::TODOS {
            assert!(
                !itens_do_grupo(*grupo).is_empty(),
                "o grupo {} ({}) não tem nenhum item",
                grupo.letra(),
                grupo.nome()
            );
        }
    }

    /// Os dois ajustes que mais variam entre máquinas, e ambos exigem reiniciar: quem mudar isto, pare e pense.
    #[test]
    fn o_grupo_de_video_nao_se_reverte_sozinho() {
        assert!(exige_reinicio(Grupo::Video));
        assert!(!pode_reverter_sozinho(Grupo::Video));
    }

    #[test]
    fn todo_grupo_se_descreve() {
        for grupo in Grupo::TODOS {
            assert!(
                grupo.descricao().len() >= 80,
                "o grupo {} não se explica",
                grupo.letra()
            );
            assert!(!grupo.nome().trim().is_empty());
        }
    }
}
