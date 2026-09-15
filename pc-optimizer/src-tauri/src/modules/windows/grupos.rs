// Os grupos de ajuste, para poder testar um de cada vez
//
// POR QUE ISTO EXISTE: um cliente aplicou tudo de uma vez, o FPS caiu pela
// metade, e nem ele nem eu tínhamos como saber QUAL dos vinte ajustes fez isso.
// A investigação levou dias e terminou comigo lendo a árvore de registro do
// Windows à mão.
//
// Com o catálogo dividido em grupos, a pergunta muda de "o Otimiza piorou meu
// PC?" para "qual destes nove grupos piorou meu PC?" — e essa segunda pergunta
// se responde com quatro medições em vez de uma investigação.
//
// ─────────────────────────────────────────────────────────────────────────
// A DIVISÃO NÃO É POR CATEGORIA DA TELA, e isso é deliberado
//
// As categorias (`Category::Gaming`, `System`, `Network`) existem para o cliente
// ENCONTRAR um ajuste. Os grupos existem para ISOLAR uma causa, e as duas coisas
// pedem cortes diferentes: o modo MSI da placa e o agendamento por hardware são
// os dois "Gaming" na tela, mas mexem em camadas completamente diferentes e
// falham de jeitos diferentes.
//
// O corte aqui é por ONDE O AJUSTE TOCA. Dois ajustes ficam no mesmo grupo
// quando uma falha de um seria confundida com a falha do outro.
//
// ─────────────────────────────────────────────────────────────────────────
// A REGRA QUE MUDA O QUE É POSSÍVEL: REINÍCIO
//
// Um grupo que exige reiniciar NÃO PODE ser testado na mesma sessão. O antes é
// medido, a máquina reinicia, e o depois é outra sessão — outra ocupação de
// memória, outros programas abertos, outro lugar do mapa. Comparar isso e
// reverter sozinho seria reverter em cima de ruído.
//
// E O RESULTADO DESSA CONTA É DESCONFORTÁVEL, então ele fica escrito aqui em
// vez de escondido: **só três dos nove grupos dispensam reinício — F, G e I —,
// e os três são justamente os que menos mexem em FPS.** Energia, agendamento,
// vídeo e memória, que são os que pesam, todos exigem reiniciar.
//
// Ou seja: o rollback automático, do jeito que o pedido descreve, cobre pouco.
// O que cobre muito é o resto do protocolo — testar UM GRUPO DE CADA VEZ em vez
// de vinte ajustes de uma vez. Essa parte funciona em todos os nove, e é ela
// que transforma "o Otimiza piorou meu PC" em "o grupo C piorou meu PC".
//
// Eu descobri isso escrevendo um teste que AFIRMAVA que o grupo de energia
// cabia numa sessão. Ele falhou. Ver
// `so_tres_grupos_cabem_numa_sessao_e_o_produto_sabe_quais`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Grupo {
    /// A: o plano de energia inteiro.
    Energia,
    /// B: quem ganha o processador — prioridade, MMCSS, fatia de fundo.
    Agendamento,
    /// C: a placa de vídeo — agendamento por hardware, interrupções.
    Video,
    /// D: memória do sistema.
    Memoria,
    /// E: rede.
    Rede,
    /// F: serviços e coisas rodando de fundo.
    Fundo,
    /// G: efeitos visuais do Windows.
    Visual,
    /// H: configuração de inicialização — HPET, limites de boot, VBS.
    Boot,
    /// I: privacidade e higiene. Não promete quadro, e está aqui para poder ser
    /// descartado como suspeito em vez de ficar misturado com o resto.
    Higiene,
}

impl Grupo {
    pub const TODOS: &'static [Grupo] = &[
        Grupo::Energia,
        Grupo::Agendamento,
        Grupo::Video,
        Grupo::Memoria,
        Grupo::Rede,
        Grupo::Fundo,
        Grupo::Visual,
        Grupo::Boot,
        Grupo::Higiene,
    ];

    /// A letra do protocolo, para o cliente e o atendimento falarem a mesma
    /// língua: "o grupo C piorou aqui" cabe numa mensagem.
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

    /// O que este grupo mexe, em uma frase, e o que esperar dele.
    pub fn descricao(self) -> &'static str {
        match self {
            Grupo::Energia => {
                "O plano de energia: frequência mínima do processador, preferência entre \
                 energia e desempenho, estacionamento de núcleos, e o que dorme sozinho. \
                 É o grupo com o maior efeito medido na maioria das máquinas."
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
                "Atraso de rede: agrupamento de pacotes e economia de energia da placa de \
                 rede. Mexe em ping e em variação de ping, não em FPS."
            }
            Grupo::Fundo => {
                "Serviços e programas que rodam sem ninguém pedir. Libera processador e \
                 disco, e o efeito aparece mais em máquina apertada."
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
                "Coleta de dados, sugestões, aplicativos instalados sozinhos. Não promete \
                 quadro nenhum, e está separado justamente para poder ser descartado como \
                 suspeito quando alguma coisa piorar."
            }
        }
    }
}

/// A que grupo um ajuste pertence.
///
/// A LISTA É EXPLÍCITA e não deduzida da categoria de propósito: dedução
/// silenciosa é como um ajuste novo acabaria num grupo errado sem ninguém
/// perceber, e aí o teste A/B apontaria o culpado errado. Há trava exigindo que
/// todo item do catálogo que entra em lote apareça aqui.
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

        "visual_effects_performance" => Grupo::Visual,
        "disable_transparency" => Grupo::Visual,

        "clear_boot_limits" => Grupo::Boot,
        "remove_forced_hpet" => Grupo::Boot,
        // `disable_vbs` mora no Boot por natureza e NÃO está aqui: ele troca
        // segurança por desempenho, e a regra abaixo vale para os três que
        // fazem isso. O protocolo aplica grupos INTEIROS — um grupo com
        // "desligar a virtualização de segurança" dentro desligaria isso na
        // máquina de quem pediu só um teste de FPS.

        "disable_telemetry" => Grupo::Higiene,
        "telemetry_policy" => Grupo::Higiene,
        "notifications_off" => Grupo::Higiene,
        "settings_sync_off" => Grupo::Higiene,
        "start_menu_web_search_off" => Grupo::Higiene,
        "mouse_precision_off" => Grupo::Higiene,
        "clean_temp_files" => Grupo::Higiene,
        "clean_update_cache" => Grupo::Higiene,
        "disable_reserved_storage" => Grupo::Higiene,

        "uac_off" | "firewall_off" | "disable_vbs" => return None,

        _ => return None,
    })
}

/// Pode ser testado pelo protocolo: volta atrás e não troca segurança.
///
/// NÃO é `entra_no_lote`, e a diferença é o ponto inteiro do protocolo. O lote
/// automático exclui o que pode custar quadro — porque ninguém deve apostar o
/// FPS de alguém num clique genérico. O teste A/B faz o contrário: ele existe
/// JUSTAMENTE para descobrir, com medição, se aquele ajuste rende NESTA máquina.
///
/// Eu escrevi este filtro como `entra_no_lote` na primeira versão, e o teste
/// de grupo vazio pegou: o grupo Memória ficou sem nenhum item, porque o único
/// ajuste dele é um dos que saíram do lote. O protocolo teria nascido cego para
/// exatamente os ajustes que ele foi feito para julgar.
fn testavel(spec: &super::catalog::OptimizationSpec) -> bool {
    spec.reversible && !spec.security_tradeoff
}

/// Os identificadores de um grupo, na ordem do catálogo.
pub fn itens_do_grupo(grupo: Grupo) -> Vec<&'static str> {
    super::catalog::CATALOG
        .iter()
        .filter(|spec| testavel(spec))
        .filter(|spec| grupo_de(spec.id) == Some(grupo))
        .map(|spec| spec.id)
        .collect()
}

/// Este grupo exige reiniciar?
///
/// Quando exige, o antes e o depois caem em sessões diferentes, e comparar as
/// duas é comparar duas tardes diferentes. Ver o cabeçalho.
pub fn exige_reinicio(grupo: Grupo) -> bool {
    super::catalog::CATALOG
        .iter()
        .filter(|spec| testavel(spec))
        .filter(|spec| grupo_de(spec.id) == Some(grupo))
        .any(|spec| spec.requires_restart)
}

/// Este grupo pode ser desfeito SOZINHO quando a medição piorar?
///
/// Só quando o teste cabe numa sessão. Reverter em cima de uma comparação entre
/// duas sessões diferentes é reverter em cima de ruído — e desfazer sozinho o
/// trabalho que a pessoa pediu, sem ela por perto, é pior que não desfazer.
pub fn pode_reverter_sozinho(grupo: Grupo) -> bool {
    !exige_reinicio(grupo) && !itens_do_grupo(grupo).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::windows::catalog::CATALOG;

    /// A TRAVA CENTRAL. Um ajuste reversível que não está em grupo nenhum é um
    /// ajuste que o protocolo A/B nunca testaria — e que continuaria podendo
    /// derrubar o FPS sem ninguém conseguir isolar.

    /// QUAIS GRUPOS CABEM NUMA SESSÃO — e a resposta é desconfortável.
    ///
    /// Eu tinha escrito um teste afirmando que o grupo de energia cabe. Ele
    /// falhou: `disable_power_throttling` exige reiniciar, e ele é de energia
    /// por natureza. Rodando a conta em todos os nove, só três cabem — e os
    /// três são justamente os que menos mexem em FPS.
    ///
    /// Isso NÃO é limitação da implementação. É o que o dado permite afirmar, e
    /// o produto prefere dizer isso a fingir que reverte sozinho em cima de uma
    /// comparação entre duas tardes diferentes.
    #[test]
    fn so_tres_grupos_cabem_numa_sessao_e_o_produto_sabe_quais() {
        let cabem: Vec<char> = Grupo::TODOS
            .iter()
            .filter(|g| pode_reverter_sozinho(**g))
            .map(|g| g.letra())
            .collect();

        assert_eq!(
            cabem,
            vec!['F', 'G', 'I'],
            "a lista de grupos que dispensam reinício mudou; se foi de propósito, \
             atualize este teste E o texto que explica o protocolo ao cliente"
        );

        // E os quatro que mais pesam em FPS estão TODOS fora dela.
        for g in [Grupo::Energia, Grupo::Agendamento, Grupo::Video, Grupo::Memoria] {
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

    /// O contrário: um id em `grupo_de` que não existe mais no catálogo é
    /// lixo que faz o grupo parecer maior do que é.
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

    /// Trocar segurança por desempenho nunca pode entrar num grupo: o protocolo
    /// aplica grupos inteiros, e um grupo com "desligar o firewall" dentro
    /// desligaria o firewall de alguém que pediu um teste de FPS.
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
    fn as_nove_letras_sao_diferentes() {
        let mut letras: Vec<char> = Grupo::TODOS.iter().map(|g| g.letra()).collect();
        let antes = letras.len();

        letras.sort();
        letras.dedup();

        assert_eq!(letras.len(), antes, "duas letras repetidas");
        assert_eq!(antes, 9, "o protocolo fala em nove grupos");
    }

    /// Nenhum grupo pode ficar vazio: um grupo sem itens aparece na tela como
    /// uma etapa do teste que não faz nada, e o cliente espera por nada.
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

    /// O grupo de vídeo tem os dois ajustes que mais variam entre máquinas, e
    /// os dois exigem reiniciar — então ele NÃO pode ser revertido sozinho.
    /// Este teste existe para que alguém que mude isso pare aqui e pense.
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
