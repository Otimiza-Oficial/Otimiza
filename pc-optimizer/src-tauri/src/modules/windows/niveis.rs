// Seguro, Competitivo, Experimental: O QUE VOCÊ ACEITA TROCAR (os perfis dizem para que serve a máquina).
// Encaixados: subir de nível nunca tira nada. Nenhum troca segurança por desempenho, nem o Experimental. O
// Experimental não é aplicar tudo: é a porta do protocolo A/B, um grupo de cada vez, medido (a 2.1.0 foi aplicar
// tudo).

use serde::{Deserialize, Serialize};

use super::catalog::{entra_no_lote, OptimizationSpec, CATALOG, FORA_DO_LOTE};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nivel {
    Seguro,
    Competitivo,
    Experimental,
}

impl Nivel {
    pub const TODOS: &'static [Nivel] = &[Nivel::Seguro, Nivel::Competitivo, Nivel::Experimental];

    pub fn nome(self) -> &'static str {
        match self {
            Nivel::Seguro => "Seguro",
            Nivel::Competitivo => "Completo",
            Nivel::Experimental => "Experimental",
        }
    }

    pub fn promessa(self) -> &'static str {
        match self {
            Nivel::Seguro => {
                "Só o que não pode piorar nada. Nenhum ajuste daqui tem histórico de custar \
                 quadro em máquina nenhuma, nenhum mexe em segurança, e todos voltam atrás. \
                 É o mínimo do botão \"Otimizar agora\"."
            }
            Nivel::Competitivo => {
                "Tudo do Seguro, mais os ajustes dos pacotes de otimização que são documentados \
                 e voltam atrás: o jogo com prioridade no processador e no agendador de \
                 multimídia, o timer fino valendo para o sistema todo, telemetria, aplicativos \
                 em segundo plano, Edge, widgets e manutenção automática parados, e o Windows \
                 respondendo sem as esperas de fábrica. Nada que possa custar quadro e nada que \
                 tire uma função do Windows. É o padrão do \"Otimizar agora\"."
            }
            Nivel::Experimental => {
                "Tudo do Avançado, mais os ajustes que rendem numa máquina e custam \
                 quadro em outra. Eles não vêm com promessa: vêm com um teste. O jeito de \
                 usar este nível é um grupo de cada vez, medindo antes e depois — e foi \
                 aplicar tudo isso de uma vez que derrubou o FPS de um cliente na 2.1.0."
            }
        }
    }

    pub fn exigencia(self) -> &'static str {
        match self {
            Nivel::Seguro => "Nada. Aplique e siga.",
            Nivel::Competitivo => {
                "Reiniciar o PC: parte dos ajustes só vale depois. Tudo volta atrás num clique, \
                 e a prova no jogo mostra se rendeu nesta máquina."
            }
            Nivel::Experimental => {
                "Medir. Sem medição dos dois lados, este nível é um chute com passos extras \
                 — e um chute que já custou FPS de cliente. Use pelo painel de grupos, que \
                 aplica um por vez e compara."
            }
        }
    }
}

/// O que o Completo acrescenta dos "só se pedir" (3.5, a pedido do dono: o "Otimizar agora" mexia em quase nada). A
/// regra para entrar: documentado, volta atrás, `RiscoDeFps::Nenhum`, sem troca de segurança e SEM tirar função do
/// Windows. Fica de fora o que custa uma função a alguém (Bluetooth, impressora, Hyper-V, Xbox, notificações,
/// localização, hibernação), o que só serve em parte das máquinas sem medida aqui (Prefetch e indexação: só em SSD;
/// efeitos visuais e transparência: mudam o que a pessoa vê) e o que pode custar quadro (esses moram no Experimental).
/// `o_completo_so_acrescenta_o_que_nao_custa_nada` confere a regra contra o catálogo.
pub const NO_COMPLETO: &[&str] = &[
    // O jogo na frente: prioridade no processador e no agendador de multimídia, e o timer fino.
    "system_responsiveness_gaming",
    "foreground_priority",
    "mmcss_games",
    "timer_global_win11",
    // O que roda por trás e disputa disco, rede e processador com o jogo.
    "disable_telemetry",
    "telemetry_policy",
    "background_apps_off",
    "edge_background_off",
    "disable_widgets",
    "store_auto_download_off",
    "maps_auto_update_off",
    "error_reporting_off",
    "manutencao_automatica_fora",
    "servicos_agrupados",
    // SysMain só em SSD: a exigência do item (`SsdSystemDrive`) o deixa de fora em HD.
    "disable_sysmain",
    // O Windows sem as esperas de fábrica.
    "windows_responde_rapido",
    "start_menu_web_search_off",
    "disable_startup_delay",
    // Estabilidade e o driver que a pessoa escolheu.
    "sem_inicializacao_rapida",
    "drivers_fora_do_windows_update",
    "remote_assistance_off",
];

/// Feita dos predicados que já existem (`entra_no_lote`, `FORA_DO_LOTE`, `RiscoDeFps`) e da lista do Completo, que
/// tem a própria trava contra o catálogo.
pub fn pertence(spec: &OptimizationSpec, nivel: Nivel) -> bool {
    // Aposentado só existe para o desfazer: o Experimental mostrava até os retirados da 2.9.
    if spec.security_tradeoff || !spec.reversible || super::catalog::retirado(spec.id) {
        return false;
    }

    match nivel {
        Nivel::Seguro => entra_no_lote(spec),
        Nivel::Competitivo => {
            entra_no_lote(spec)
                || FORA_DO_LOTE.contains(&spec.id)
                || (NO_COMPLETO.contains(&spec.id) && !spec.risco_de_fps.pode_custar())
        }
        Nivel::Experimental => true,
    }
}

/// O que o lote do "Otimizar agora" leva A MAIS no nível pedido, além do que já entra sozinho. Os condicionais medidos
/// (Game DVR, Modo de Jogo) continuam dependendo da condição; o Experimental nunca vai de uma vez.
pub fn acrescenta_ao_lote(spec: &OptimizationSpec, nivel: Nivel) -> bool {
    use super::catalog::{classe, Classe, Condicao};
    let medido = matches!(classe(spec.id), Classe::Condicional(c) if c != Condicao::SoSePedir);
    aplica_de_uma_vez(nivel) && pertence(spec, nivel) && !medido
}

pub fn itens_do_nivel(nivel: Nivel) -> Vec<&'static str> {
    CATALOG
        .iter()
        .filter(|spec| pertence(spec, nivel))
        .map(|spec| spec.id)
        .collect()
}

/// A diferença para o anterior é o que ajuda a decidir, não a lista inteira.
pub fn acrescenta(nivel: Nivel) -> Vec<&'static str> {
    let anterior = match nivel {
        Nivel::Seguro => return itens_do_nivel(Nivel::Seguro),
        Nivel::Competitivo => Nivel::Seguro,
        Nivel::Experimental => Nivel::Competitivo,
    };

    let de_baixo = itens_do_nivel(anterior);

    itens_do_nivel(nivel)
        .into_iter()
        .filter(|id| !de_baixo.contains(id))
        .collect()
}

/// O Experimental não: aplicar junto dá um número e nenhuma informação sobre o culpado.
pub fn aplica_de_uma_vez(nivel: Nivel) -> bool {
    !matches!(nivel, Nivel::Experimental)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_nivel_contem_o_anterior() {
        let seguro = itens_do_nivel(Nivel::Seguro);
        let competitivo = itens_do_nivel(Nivel::Competitivo);
        let experimental = itens_do_nivel(Nivel::Experimental);

        for id in &seguro {
            assert!(
                competitivo.contains(id),
                "`{id}` está no Seguro e não no Competitivo"
            );
        }

        for id in &competitivo {
            assert!(
                experimental.contains(id),
                "`{id}` está no Competitivo e não no Experimental"
            );
        }
    }

    #[test]
    fn cada_nivel_acrescenta_alguma_coisa() {
        assert!(
            !acrescenta(Nivel::Competitivo).is_empty(),
            "o Competitivo é igual ao Seguro"
        );
        assert!(
            !acrescenta(Nivel::Experimental).is_empty(),
            "o Experimental é igual ao Competitivo"
        );
    }

    #[test]
    fn nenhum_nivel_troca_seguranca_por_desempenho() {
        for nivel in Nivel::TODOS {
            for id in itens_do_nivel(*nivel) {
                let spec = CATALOG.iter().find(|s| s.id == id).unwrap();

                assert!(
                    !spec.security_tradeoff,
                    "`{}` troca segurança e entrou no nível {}",
                    id,
                    nivel.nome()
                );
            }
        }
    }

    /// Se os dois se separarem, "Seguro" e o botão entregam coisas diferentes.
    #[test]
    fn o_seguro_e_exatamente_o_que_o_botao_grande_aplica() {
        let do_lote: Vec<&str> = CATALOG
            .iter()
            .filter(|s| entra_no_lote(s))
            .map(|s| s.id)
            .collect();

        assert_eq!(itens_do_nivel(Nivel::Seguro), do_lote);
    }

    #[test]
    fn nada_que_pode_custar_fps_entra_abaixo_do_experimental() {
        for nivel in [Nivel::Seguro, Nivel::Competitivo] {
            for id in itens_do_nivel(nivel) {
                let spec = CATALOG.iter().find(|s| s.id == id).unwrap();

                assert!(
                    !spec.risco_de_fps.pode_custar(),
                    "`{}` pode custar FPS e entrou no nível {}",
                    id,
                    nivel.nome()
                );
            }
        }
    }

    #[test]
    fn o_experimental_e_onde_moram_os_que_podem_custar_fps() {
        let experimental = itens_do_nivel(Nivel::Experimental);

        let arriscados: Vec<&str> = CATALOG
            .iter()
            .filter(|s| s.risco_de_fps.pode_custar() && !super::super::catalog::retirado(s.id))
            .map(|s| s.id)
            .collect();

        assert!(
            !arriscados.is_empty(),
            "o catálogo não tem nenhum ajuste de risco?"
        );

        for id in arriscados {
            assert!(
                experimental.contains(&id),
                "`{id}` não está no Experimental"
            );
        }
    }

    #[test]
    fn o_experimental_nao_se_aplica_de_uma_vez() {
        assert!(aplica_de_uma_vez(Nivel::Seguro));
        assert!(aplica_de_uma_vez(Nivel::Competitivo));
        assert!(!aplica_de_uma_vez(Nivel::Experimental));
    }

    #[test]
    fn todo_nivel_promete_e_exige_por_escrito() {
        for nivel in Nivel::TODOS {
            assert!(
                nivel.promessa().len() >= 120,
                "{} sem promessa",
                nivel.nome()
            );
            assert!(
                !nivel.exigencia().trim().is_empty(),
                "{} sem exigência",
                nivel.nome()
            );
        }
    }

    #[test]
    fn a_exigencia_do_experimental_e_medir() {
        let e = Nivel::Experimental.exigencia();

        assert!(
            e.contains("Medir"),
            "o Experimental precisa exigir medição: {e}"
        );
        assert!(e.contains("um por vez") || e.contains("grupo"));
    }

    #[test]
    fn o_completo_so_acrescenta_o_que_nao_custa_nada() {
        use super::super::catalog::{classe, Classe, Condicao};
        for id in NO_COMPLETO {
            let spec = CATALOG
                .iter()
                .find(|s| s.id == *id)
                .unwrap_or_else(|| panic!("`{id}` não está no catálogo"));
            assert!(spec.reversible, "{id} não volta atrás");
            assert!(!spec.security_tradeoff, "{id} troca segurança");
            assert!(!spec.risco_de_fps.pode_custar(), "{id} pode custar FPS");
            assert_eq!(
                classe(id),
                Classe::Condicional(Condicao::SoSePedir),
                "{id}: a lista é dos \"só se pedir\""
            );
            assert!(itens_do_nivel(Nivel::Competitivo).contains(id), "{id} ficou fora do Completo");
            assert!(acrescenta_ao_lote(spec, Nivel::Competitivo), "{id} não iria no lote do Completo");
            assert!(!acrescenta_ao_lote(spec, Nivel::Seguro), "{id} entraria no Seguro");
        }
    }

    #[test]
    fn o_completo_nao_tira_funcao_do_windows() {
        for fora in [
            "servicos_bluetooth_fora",
            "servicos_impressora_fora",
            "servicos_hyperv_fora",
            "disable_xbox_services",
            "notifications_off",
            "servicos_pouco_usados",
            "disable_hibernation",
            "prefetch_desligado",
        ] {
            assert!(!NO_COMPLETO.contains(&fora), "{fora} tira uma função do Windows de alguém");
        }
    }

    #[test]
    fn o_experimental_nunca_vai_no_lote() {
        for spec in CATALOG {
            assert!(!acrescenta_ao_lote(spec, Nivel::Experimental), "{} iria de uma vez", spec.id);
        }
    }

    #[test]
    fn o_competitivo_acrescenta_o_que_muda_comportamento() {
        let novos = acrescenta(Nivel::Competitivo);

        assert!(
            !novos.is_empty(),
            "o Competitivo precisa acrescentar alguma coisa ao Seguro"
        );
        for nivel in [Nivel::Seguro, Nivel::Competitivo, Nivel::Experimental] {
            for id in itens_do_nivel(nivel) {
                assert!(
                    !super::super::catalog::retirado(id),
                    "{id} foi aposentado e ainda aparece num nível"
                );
            }
        }
    }
}
