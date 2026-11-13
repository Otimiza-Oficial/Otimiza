// O RISCO DE CADA AJUSTE, POR ESCRITO (Plano de refino, E4). Para cada item vivo do catálogo: o que pode dar errado, em
// que máquina, e qual leitura do Otimiza prova que deu certo (ou desfaz). Um item novo sem linha aqui reprova o build
// (`todo_item_vivo_tem_o_risco_escrito`): risco não escrito é risco que ninguém testou.
//
// As provas são as que o produto já roda: o portão (FPS, 1% pior e trancos da partida antes e depois), o vigia de
// estabilidade (queda, tela azul, driver de vídeo reiniciado), a guarda térmica (meia hora depois) e a releitura do
// valor no Windows logo depois de gravar.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prova {
    /// A partida antes e depois (`modules::portao`): piorou, desfaz.
    Portao,
    /// Queda, tela azul ou driver reiniciado depois do ajuste (`estabilidade`): desfaz.
    Estabilidade,
    /// Temperatura e freio da placa e do processador na meia hora seguinte (`guarda_termica`).
    Termica,
    /// Só a releitura do valor: o ajuste não mexe em FPS, mexe em comportamento que a pessoa vê.
    Leitura,
}

pub struct Risco {
    pub id: &'static str,
    pub o_que_pode_dar_errado: &'static str,
    pub onde: &'static str,
    pub provas: &'static [Prova],
}

use Prova::*;

pub const RISCOS: &[Risco] = &[
    Risco { id: "plano_otimiza", o_que_pode_dar_errado: "Mais consumo e calor com o processador mais acordado.", onde: "Notebook na bateria e gabinete com pouca ventilação.", provas: &[Portao, Termica] },
    Risco { id: "disable_gamedvr", o_que_pode_dar_errado: "Some a gravação dos últimos minutos pela Game Bar.", onde: "Quem usa Win+Alt+G para gravar jogada.", provas: &[Leitura] },
    Risco { id: "windows_game_mode_on", o_que_pode_dar_errado: "Em alguns jogos antigos o Modo de Jogo já deu tranco.", onde: "Windows 10 antigo (antes de 2004).", provas: &[Portao] },
    Risco { id: "gpu_hardware_scheduling", o_que_pode_dar_errado: "Tranco ou tela preta com driver de vídeo antigo; só vale depois de reiniciar.", onde: "Placa NVIDIA antes da série 10, AMD antes da RX 5000, driver desatualizado.", provas: &[Portao, Estabilidade] },
    Risco { id: "system_responsiveness_gaming", o_que_pode_dar_errado: "Áudio picotando com o jogo pesado (o áudio perde fatia reservada).", onde: "Processador de 4 núcleos ou menos com programa de áudio aberto.", provas: &[Portao] },
    Risco { id: "visual_effects_performance", o_que_pode_dar_errado: "O Windows fica sem animações; não muda o jogo.", onde: "Qualquer PC: é gosto.", provas: &[Leitura] },
    Risco { id: "disable_startup_delay", o_que_pode_dar_errado: "Tudo que abre com o Windows abre junto, e o começo fica mais disputado.", onde: "PC com disco rígido (HD) e muitos programas na inicialização.", provas: &[Leitura] },
    Risco { id: "mouse_precision_off", o_que_pode_dar_errado: "O ponteiro parece mais lento no Windows até acostumar.", onde: "Mouse de baixa resolução (DPI) em monitor grande.", provas: &[Leitura] },
    Risco { id: "mouse_pointer_1to1", o_que_pode_dar_errado: "A velocidade do ponteiro muda; o jogo com entrada bruta não sente.", onde: "Quem tinha a velocidade ajustada de propósito.", provas: &[Leitura] },
    Risco { id: "keyboard_fast_repeat", o_que_pode_dar_errado: "Tecla segurada repete rápido demais ao digitar.", onde: "Quem digita devagar ou usa o teclado para texto longo.", provas: &[Leitura] },
    Risco { id: "foreground_priority", o_que_pode_dar_errado: "O que roda atrás (gravação, transmissão) ganha menos processador.", onde: "Quem transmite pelo OBS no mesmo PC.", provas: &[Portao] },
    Risco { id: "disable_hibernation", o_que_pode_dar_errado: "Some a hibernação; a bateria descarregada desliga sem salvar o estado.", onde: "Notebook.", provas: &[Leitura] },
    Risco { id: "disable_power_throttling", o_que_pode_dar_errado: "Programa de fundo gasta mais bateria.", onde: "Notebook na bateria.", provas: &[Portao, Termica] },
    Risco { id: "disable_memory_compression", o_que_pode_dar_errado: "Com pouca memória, o Windows passa a usar mais o disco (arquivo de paginação).", onde: "PC com 8 GB ou menos.", provas: &[Portao] },
    Risco { id: "gpu_msi_mode", o_que_pode_dar_errado: "Placa ou driver que não aceita MSI pode dar tela preta no boot.", onde: "Placa de vídeo muito antiga ou driver de terceiro.", provas: &[Estabilidade, Portao] },
    Risco { id: "nic_power_saving_off", o_que_pode_dar_errado: "Um pouco mais de consumo na placa de rede.", onde: "Notebook na bateria.", provas: &[Leitura] },
    Risco { id: "background_apps_off", o_que_pode_dar_errado: "App da Loja deixa de avisar em segundo plano (Telefone, Correio).", onde: "Quem usa apps da Loja para mensagem.", provas: &[Leitura] },
    Risco { id: "start_menu_web_search_off", o_que_pode_dar_errado: "O Iniciar não mostra resultado da internet.", onde: "Qualquer PC: é gosto.", provas: &[Leitura] },
    Risco { id: "clear_boot_limits", o_que_pode_dar_errado: "Nenhum em PC comum: só tira limite que alguém gravou; vale depois de reiniciar.", onde: "PC em que o limite foi posto de propósito (teste de hardware).", provas: &[Estabilidade] },
    Risco { id: "timer_global_win11", o_que_pode_dar_errado: "Um pouco mais de consumo parado, porque o relógio de 1 ms vale para todos.", onde: "Notebook na bateria.", provas: &[Portao, Termica] },
    Risco { id: "drivers_fora_do_windows_update", o_que_pode_dar_errado: "Driver de aparelho novo não chega sozinho; a pessoa instala pelo fabricante.", onde: "PC que recebe aparelho novo com frequência.", provas: &[Leitura] },
    Risco { id: "sem_inicializacao_rapida", o_que_pode_dar_errado: "Ligar o PC demora alguns segundos a mais.", onde: "PC com disco rígido (HD).", provas: &[Leitura] },
    Risco { id: "servicos_agrupados", o_que_pode_dar_errado: "Um serviço que trava leva os do mesmo grupo junto.", onde: "PC com driver ou programa que derruba serviço com frequência.", provas: &[Estabilidade] },
    Risco { id: "manutencao_automatica_fora", o_que_pode_dar_errado: "A otimização agendada do disco e as verificações não rodam sozinhas.", onde: "PC com disco rígido (HD), que precisa da desfragmentação.", provas: &[Leitura] },
    Risco { id: "mpo_desligado", o_que_pode_dar_errado: "Vídeo e janela gastam um pouco mais de placa; em alguns, o HDR muda de brilho.", onde: "Monitor HDR e notebook com placa integrada.", provas: &[Portao, Estabilidade] },
    Risco { id: "fila_de_entrada_curta", o_que_pode_dar_errado: "Com o PC travado, tecla e movimento podem se perder em vez de esperar.", onde: "Mouse de taxa muito alta (4000 Hz ou mais) em processador fraco.", provas: &[Portao] },
    Risco { id: "windows_responde_rapido", o_que_pode_dar_errado: "Ao desligar, programa que demora a salvar é fechado mais cedo.", onde: "Quem desliga com editor ou jogo salvando aberto.", provas: &[Leitura] },
    Risco { id: "prefetch_desligado", o_que_pode_dar_errado: "Programas e jogos abrem mais devagar.", onde: "PC com disco rígido (HD).", provas: &[Portao] },
    Risco { id: "privacidade_do_pacote", o_que_pode_dar_errado: "App da Loja que precisa de localização ou arquivos para de funcionar até liberar.", onde: "Quem usa Mapas, Fotos ou Correio.", provas: &[Leitura] },
    Risco { id: "placa_no_clock_maximo", o_que_pode_dar_errado: "A placa esquenta e gasta mais parada, e o ventilador gira mais.", onde: "Notebook e placa com refrigeração fraca.", provas: &[Termica, Portao] },
    Risco { id: "tcp_do_pacote", o_que_pode_dar_errado: "Em rede de empresa com regra própria, conexão pode estranhar o TTL.", onde: "PC em rede corporativa.", provas: &[Leitura] },
    Risco { id: "compartilhamento_do_pacote", o_que_pode_dar_errado: "Arquivo aberto por dois PCs na rede pode conflitar sem aviso.", onde: "Quem compartilha pasta com outra máquina.", provas: &[Leitura] },
    Risco { id: "bufferbloat_do_pacote", o_que_pode_dar_errado: "Download mais lento, a placa de rede passa trabalho para o processador e a internet cai alguns segundos ao aplicar.", onde: "Jogo preso no processador com muito tráfego (FiveM cheio) e internet rápida.", provas: &[Portao] },
    Risco { id: "tela_cheia_sem_otimizacao", o_que_pode_dar_errado: "Alt+Tab mais lento e a sobreposição (Discord, Steam) pode sumir em tela cheia.", onde: "Jogo em tela cheia exclusiva com sobreposição aberta.", provas: &[Portao] },
    Risco { id: "mitigacoes_spectre_fora", o_que_pode_dar_errado: "O PC fica exposto a uma classe de ataque pelo processador.", onde: "PC compartilhado ou que roda programa de origem duvidosa.", provas: &[Portao, Estabilidade] },
    Risco { id: "disable_vbs", o_que_pode_dar_errado: "Somem o isolamento de credenciais e a integridade de memória; anticheat que exige VBS recusa.", onde: "Jogo com anticheat que pede integridade de memória e PC de trabalho.", provas: &[Portao, Estabilidade] },
    Risco { id: "disable_transparency", o_que_pode_dar_errado: "Menu e barra ficam opacos; não muda o jogo.", onde: "Qualquer PC: é gosto.", provas: &[Leitura] },
    Risco { id: "remove_forced_hpet", o_que_pode_dar_errado: "Nenhum em PC comum: volta ao relógio padrão do Windows; vale depois de reiniciar.", onde: "PC em que alguém forçou o HPET de propósito para um programa antigo.", provas: &[Estabilidade, Portao] },
    Risco { id: "mmcss_games", o_que_pode_dar_errado: "Áudio e gravação disputam mais com o jogo.", onde: "Quem grava ou transmite no mesmo PC.", provas: &[Portao] },
    Risco { id: "disable_widgets", o_que_pode_dar_errado: "Some o painel de notícias e clima.", onde: "Qualquer PC: é gosto.", provas: &[Leitura] },
    Risco { id: "telemetry_policy", o_que_pode_dar_errado: "O Windows Insider e parte do diagnóstico pedem nível maior e reclamam.", onde: "PC no programa Insider.", provas: &[Leitura] },
    Risco { id: "notifications_off", o_que_pode_dar_errado: "Aviso de app e do sistema deixa de aparecer, inclusive o que importa.", onde: "Quem depende de aviso no canto da tela.", provas: &[Leitura] },
    Risco { id: "store_auto_download_off", o_que_pode_dar_errado: "Apps da Loja ficam desatualizados até a pessoa atualizar.", onde: "Quem usa muitos apps da Loja.", provas: &[Leitura] },
    Risco { id: "maps_auto_update_off", o_que_pode_dar_errado: "Mapas offline ficam desatualizados.", onde: "Quem usa o app Mapas offline.", provas: &[Leitura] },
    Risco { id: "remote_assistance_off", o_que_pode_dar_errado: "O suporte pela Assistência Remota do Windows deixa de conectar.", onde: "PC que recebe suporte por esse recurso.", provas: &[Leitura] },
    Risco { id: "edge_background_off", o_que_pode_dar_errado: "O navegador abre um pouco mais devagar e não avisa com ele fechado.", onde: "Quem usa notificação do navegador.", provas: &[Leitura] },
    Risco { id: "accessibility_keys_off", o_que_pode_dar_errado: "O atalho das teclas de acessibilidade para de funcionar.", onde: "Quem precisa das Teclas de Aderência.", provas: &[Leitura] },
    Risco { id: "windowed_game_optimizations", o_que_pode_dar_errado: "Captura de tela antiga pode sair preta em alguns jogos.", onde: "Programa de gravação antigo.", provas: &[Portao] },
];

pub fn de(id: &str) -> Option<&'static Risco> {
    RISCOS.iter().find(|r| r.id == id)
}

/// **Pura.** A frase da tela: só para o que mexe em FPS, estabilidade ou temperatura.
pub fn cuidado(id: &str) -> Option<String> {
    let r = de(id)?;
    r.provas
        .iter()
        .any(|p| *p != Prova::Leitura)
        .then(|| format!("{} Mais provável em: {}", r.o_que_pode_dar_errado, r.onde.trim_end_matches('.')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_item_vivo_tem_o_risco_escrito() {
        let faltando: Vec<&str> = super::super::catalog::CATALOG
            .iter()
            .map(|s| s.id)
            .filter(|id| !super::super::catalog::RETIRADOS.contains(id))
            .filter(|id| de(id).is_none())
            .collect();
        assert!(faltando.is_empty(), "item sem risco escrito em `riscos.rs`: {faltando:?}");
    }

    #[test]
    fn o_cuidado_so_aparece_no_que_mexe_em_fps() {
        assert!(cuidado("gpu_msi_mode").is_some_and(|c| c.contains("Mais provável em")));
        assert_eq!(cuidado("disable_widgets"), None);
        assert_eq!(cuidado("nao_existe"), None);
    }

    #[test]
    fn cada_risco_diz_o_que_onde_e_como_se_prova() {
        for r in RISCOS {
            assert!(r.o_que_pode_dar_errado.len() >= 15 && r.onde.len() >= 8, "{}", r.id);
            assert!(!r.provas.is_empty(), "{}", r.id);
            assert!(super::super::catalog::find(r.id).is_some(), "{} não está no catálogo", r.id);
        }
    }
}
