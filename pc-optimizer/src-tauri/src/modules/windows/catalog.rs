// Catálogo de otimizações: o que muda, o ganho REAL esperado e se é reversível. Nada desativa Windows Update,
// antivírus, firewall ou serviços de núcleo.

use crate::modules::optimizer::{
    Category, ExpectedGain, OQuePodeCustar, OptimizationInfo, OptimizationState, RiscoDeFps,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum RegValue {
    Dword(u32),
    Text(&'static str),
    /// Para a `UserPreferencesMask`: os efeitos visuais ficam bit a bit ali, e sem ela só mudaria o rótulo.
    Binary(&'static [u8]),
}

#[derive(Debug, Clone)]
pub enum Action {
    Registry {
        hive: &'static str,
        path: &'static str,
        name: &'static str,
        value: RegValue,
    },
    DisableService {
        name: &'static str,
    },
    /// Plano próprio: `HighPerformancePowerPlan` ativava um GUID que não existe em toda máquina, e `PowerSetting`
    /// escrevia DENTRO do plano do cliente. Desfazer é reativar o anterior (ver `planoenergia.rs`).
    PlanoOtimiza,
    /// Os GUIDs das interfaces mudam de PC para PC.
    DisableNagle,
    DisableHibernation,
    // `ChangeRecord::PowerSetting` continua: há `changes.json` de versões anteriores a desfazer.
    MemoryCompression {
        enabled: bool,
    },
    ClearBootLimits,
    GpuMsiMode,
    NicPowerSaving,
    ReservedStorage {
        enabled: bool,
    },
    RemoveForcedPlatformClock,
    /// Anda com o desligamento do VBS, que roda em cima do hipervisor: sem isto o cliente paga em segurança e não
    /// recebe o desempenho.
    DisableHypervisor,
    /// O estado é um campo de bits: só o bit 0 muda, senão atalho, som e aviso iriam junto.
    AccessibilityKeysOff,
    /// O valor é um texto com várias escolhas gráficas: só este pedaço muda (`janelas.rs`).
    JanelasOtimizadas,
    /// `DisableDynamicPstate` na chave de classe de cada placa NVIDIA: a pasta (`0000`, `0001`) muda de PC para PC.
    ClockMaximoDaPlaca,
    /// Um elemento do `bcdedit` no `{current}`. Só os nomes de `pacote::BOOT_PERMITIDOS`: o desfazer passa o nome
    /// guardado no histórico para o `bcdedit`.
    BootValor {
        nome: &'static str,
        valor: &'static str,
    },
    /// Um parâmetro do `Set-NetTCPSetting` no modelo "Internet". Só os de `pacote::TCP_PERMITIDOS`.
    RedeTcp {
        parametro: &'static str,
        valor: &'static str,
    },
    /// Um parâmetro do `Set-NetOffloadGlobalSetting`. Só os de `pacote::OFFLOAD_PERMITIDOS`.
    RedeOffload {
        parametro: &'static str,
        valor: &'static str,
    },
    /// Um recurso de cada placa de rede ("Lso" ou "ChecksumOffload"), desligado onde estava ligado.
    RedeAdaptador {
        recurso: &'static str,
    },
}

/// Desligar efeito visual muda pouco num PC forte e muito num de 4 GB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boost {
    LowRam,
    MechanicalDisk,
    FewCores,
}

#[derive(Debug, Clone, Copy)]
pub enum Requirement {
    SsdSystemDrive,
    // Sem item que use desde 11/10 (a compressão de memória saiu da trava a pedido do dono); a checagem fica.
    #[allow(dead_code)]
    MinRamGb(f64),
    /// `2700` é WDDM 2.7. Abaixo disso o Windows ignora o agendamento por hardware: a escrita dá certo, a releitura
    /// confere, e o cliente reiniciava por nada. Só perguntar antes resolve.
    MinWddm(u32),
}

impl Requirement {
    pub fn unmet_reason(&self) -> &'static str {
        match self {
            Requirement::SsdSystemDrive => {
                "Não oferecemos: seu disco de sistema não é SSD, e aqui isso deixaria o PC mais lento."
            }
            Requirement::MinRamGb(_) => {
                "Não oferecemos: sua memória RAM é pouca para isso, e aplicar pioraria o desempenho."
            }
            // Verdade nos dois casos: driver antigo ou versão ilegível.
            Requirement::MinWddm(_) => {
                "Não oferecemos: não deu para confirmar que o driver de vídeo desta máquina é WDDM 2.7 ou mais novo. Abaixo disso o Windows ignora este ajuste — aplicar marcaria como feito o que não teria efeito nenhum."
            }
        }
    }
}

pub struct OptimizationSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub honest_effect: &'static str,
    pub category: Category,
    pub expected_gain: ExpectedGain,
    /// Quando pode, `entra_no_lote` o exclui do "Otimizar agora".
    pub risco_de_fps: RiscoDeFps,
    pub requires_admin: bool,
    pub requires_restart: bool,
    /// A exceção é apagar arquivo, que não volta.
    pub reversible: bool,
    pub requirement: Option<Requirement>,
    /// Nunca no "Otimizar agora": abrir mão de proteção é decisão consciente.
    pub security_tradeoff: bool,
    pub highlight_when: &'static [Boost],
    pub actions: &'static [Action],
}

/// Nunca em lote, mesmo reversíveis e sem troca de segurança; continuam um a um, com aviso.
/// `windowed_game_optimizations` (3.0): muda a entrega de quadros; só entra quando o PresentMon mostrar o ganho.
pub const FORA_DO_LOTE: &[&str] = &["windowed_game_optimizations"];

// Classes da auditoria 2.9 (`docs/auditorias/AUDITORIA-2.9.md`): Essencial (qualquer máquina, entra no lote);
// Condicional (só com a condição MEDIDA aqui; os "só se pedir" aparecem e nunca entram em lote); Expert (só no
// modo Expert, nunca em lote). `toda_classe_aponta_para_um_item_do_catalogo` pega id errado.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condicao {
    GameDvrLigado,
    /// `AutoGameModeEnabled = 0`: alguém (ou outro otimizador) desligou o Modo de Jogo, que o Windows traz ligado.
    ModoDeJogoDesligado,
    /// Os ajustes de área de trabalho só são sentidos aqui.
    PcFraco,
    PoucoEspaco,
    MemoriaApertada,
    /// Aparece, mas só entra se a pessoa escolher o item.
    SoSePedir,
}

impl Condicao {
    pub fn quando(self) -> &'static str {
        match self {
            Condicao::GameDvrLigado => "Aparece porque a gravação em segundo plano do Game Bar está ligada nesta máquina.",
            Condicao::ModoDeJogoDesligado => "Aparece porque o Modo de Jogo do Windows está desligado nesta máquina — o Windows o traz ligado.",
            Condicao::PcFraco => "Aparece porque esta máquina tem até 8 GB de memória ou até 4 núcleos — é onde a área de trabalho mais pesa.",
            Condicao::PoucoEspaco => "Aparece porque o disco do Windows tem menos de 20 GB livres.",
            Condicao::MemoriaApertada => "Aparece porque esta máquina tem até 16 GB de memória, e processo de fundo disputa memória com o jogo.",
            Condicao::SoSePedir => "Só se você quiser: nada medido nesta máquina decide isto por você, então nunca entra no \"Otimizar agora\".",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classe {
    Essencial,
    Condicional(Condicao),
    Expert,
}

pub const EXPERT: &[&str] = &[
    "gpu_hardware_scheduling",
    "gpu_msi_mode",
    "disable_vbs",
    // Troca de segurança como o VBS (do pacote do dono, 11/10).
    "mitigacoes_spectre_fora",
];

pub const CONDICIONAIS: &[(&str, Condicao)] = &[
    ("disable_gamedvr", Condicao::GameDvrLigado),
    ("windows_game_mode_on", Condicao::ModoDeJogoDesligado),
    // A pedido, nunca no lote: mexem no que a pessoa sente na mão e no olho, não em FPS.
    ("visual_effects_performance", Condicao::SoSePedir),
    ("keyboard_fast_repeat", Condicao::SoSePedir),
    ("mouse_pointer_1to1", Condicao::SoSePedir),
    // Real quando a placa dorme e perde pacote, e a perda não é medida aqui: não decide sozinho.
    ("nic_power_saving_off", Condicao::SoSePedir),
    // Do pacote do dono (11/10): documentados e reversíveis, mas sem medida aqui ainda. Só entram no lote depois de
    // a medição no FiveM mostrar ganho.
    ("timer_global_win11", Condicao::SoSePedir),
    ("drivers_fora_do_windows_update", Condicao::SoSePedir),
    ("sem_inicializacao_rapida", Condicao::SoSePedir),
    ("servicos_agrupados", Condicao::SoSePedir),
    ("manutencao_automatica_fora", Condicao::SoSePedir),
    ("mpo_desligado", Condicao::SoSePedir),
    // Voltaram dos RETIRADOS a pedido do dono (11/10): estão nos dois pacotes que lhe deram FPS. Cada um com o efeito
    // escrito e o desfazer; nenhum entra sozinho no "Otimizar agora".
    ("system_responsiveness_gaming", Condicao::SoSePedir),
    ("foreground_priority", Condicao::SoSePedir),
    ("mmcss_games", Condicao::SoSePedir),
    ("disable_power_throttling", Condicao::SoSePedir),
    ("disable_sysmain", Condicao::SoSePedir),
    ("disable_memory_compression", Condicao::SoSePedir),
    ("disable_xbox_services", Condicao::SoSePedir),
    ("notifications_off", Condicao::SoSePedir),
    ("maps_auto_update_off", Condicao::SoSePedir),
    ("remote_assistance_off", Condicao::SoSePedir),
    ("disable_telemetry", Condicao::SoSePedir),
    ("telemetry_policy", Condicao::SoSePedir),
    ("start_menu_web_search_off", Condicao::SoSePedir),
    ("disable_transparency", Condicao::SoSePedir),
    ("disable_startup_delay", Condicao::SoSePedir),
    ("disable_hibernation", Condicao::SoSePedir),
    ("background_apps_off", Condicao::SoSePedir),
    ("disable_search_indexing", Condicao::SoSePedir),
    ("disable_widgets", Condicao::SoSePedir),
    ("store_auto_download_off", Condicao::SoSePedir),
    ("error_reporting_off", Condicao::SoSePedir),
    ("edge_background_off", Condicao::SoSePedir),
    // O resto dos dois pacotes, novo no catálogo.
    ("fila_de_entrada_curta", Condicao::SoSePedir),
    ("windows_responde_rapido", Condicao::SoSePedir),
    ("prefetch_desligado", Condicao::SoSePedir),
    ("privacidade_do_pacote", Condicao::SoSePedir),
    ("servicos_bluetooth_fora", Condicao::SoSePedir),
    ("servicos_impressora_fora", Condicao::SoSePedir),
    ("servicos_hyperv_fora", Condicao::SoSePedir),
    ("servicos_pouco_usados", Condicao::SoSePedir),
    ("servicos_de_empresa_fora", Condicao::SoSePedir),
    ("placa_no_clock_maximo", Condicao::SoSePedir),
    ("tick_do_pacote", Condicao::SoSePedir),
    ("tcp_do_pacote", Condicao::SoSePedir),
    ("compartilhamento_do_pacote", Condicao::SoSePedir),
    ("bufferbloat_do_pacote", Condicao::SoSePedir),
    ("tela_cheia_sem_otimizacao", Condicao::SoSePedir),
];

pub fn classe(id: &str) -> Classe {
    if EXPERT.contains(&id) {
        return Classe::Expert;
    }
    match CONDICIONAIS.iter().find(|(i, _)| *i == id) {
        Some((_, c)) => Classe::Condicional(*c),
        None => Classe::Essencial,
    }
}

/// RETIRADOS na 2.9: não mudam nada que o cliente sinta (UAC e Firewall ainda tiravam proteção). Ficam SÓ para o
/// desfazer de quem aplicou antes; nada aqui se aplica de novo. Os motivos moram em `naofazemos.rs`.
///
/// Em 11/10/2026 o dono trouxe de volta tudo o que estava nos dois pacotes que lhe deram FPS (`CONDICIONAIS`, "só se
/// pedir"). Ficam aqui só os que nenhum dos dois pacotes faz.
pub const RETIRADOS: &[&str] = &[
    "network_low_latency",
    "settings_sync_off",
    "disable_copilot",
    // Terceira rodada (3.1): liberar disco não muda o jogo, fora a pedido do dono.
    "disable_reserved_storage",
    "delivery_optimization_off",
    "stop_sponsored_apps",
];

pub fn retirado(id: &str) -> bool {
    RETIRADOS.contains(&id)
}

/// As quatro exclusões juntas: não volta, troca segurança, `FORA_DO_LOTE` e, desde a 2.1.0 (FPS de um cliente
/// caiu pela metade), **pode custar FPS**. O item continua item a item, com o caso de `RiscoDeFps::PodeCustar` na
/// tela.
pub fn entra_no_lote(spec: &OptimizationSpec) -> bool {
    entra_no_lote_se(spec, |_| false)
}

/// Condicional só entra com a condição atendida; Expert e "só se pedir" nunca. **Pura**: quem mede é o chamador.
pub fn entra_no_lote_se(spec: &OptimizationSpec, atendida: impl Fn(Condicao) -> bool) -> bool {
    let pela_classe = match classe(spec.id) {
        Classe::Essencial => true,
        Classe::Expert | Classe::Condicional(Condicao::SoSePedir) => false,
        Classe::Condicional(c) => atendida(c),
    };
    pela_classe
        && spec.reversible
        && !spec.security_tradeoff
        && !spec.risco_de_fps.pode_custar()
        && !FORA_DO_LOTE.contains(&spec.id)
        && !retirado(spec.id)
}

impl OptimizationSpec {
    pub fn to_info(
        &self,
        state: OptimizationState,
        detail: Option<String>,
        recommended: bool,
    ) -> OptimizationInfo {
        OptimizationInfo {
            id: self.id.to_string(),
            name: self.name.to_string(),
            description: self.description.to_string(),
            honest_effect: self.honest_effect.to_string(),
            category: self.category,
            expected_gain: self.expected_gain,
            risco_de_fps: (&self.risco_de_fps).into(),
            requires_admin: self.requires_admin,
            requires_restart: self.requires_restart,
            reversible: self.reversible,
            security_tradeoff: self.security_tradeoff,
            retirado: retirado(self.id),
            expert: classe(self.id) == Classe::Expert,
            condicao: match classe(self.id) {
                Classe::Condicional(c) => Some(c.quando().to_string()),
                _ => None,
            },
            recommended,
            state,
            detail,
        }
    }
}

pub static CATALOG: &[OptimizationSpec] = &[
    OptimizationSpec {
        id: "plano_otimiza",
        name: "Plano de energia OTIMIZA",
        description: "Cria um plano de energia próprio para este computador, configurado pelo hardware que foi encontrado nele, e o ativa.",
        honest_effect: "Ganho real e consistente em PCs no plano Equilibrado, e nenhum ganho em PC que já estava num plano de desempenho — o relatório diz qual dos dois é o seu, ajuste por ajuste. Em notebook, os valores agressivos valem só na tomada: na bateria o plano fica no padrão do Windows de propósito. Ajuste que não existe neste Windows é listado como tal, e não conta como falha.",
        category: Category::System,
        expected_gain: ExpectedGain::Measurable,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::PlanoOtimiza],
    },
    OptimizationSpec {
        id: "disable_gamedvr",
        name: "Desativar gravação em segundo plano (Game DVR)",
        description: "Desliga a gravação automática da Xbox Game Bar, que roda durante todo jogo.",
        honest_effect: "Costuma render de 2 a 8 FPS em placas de vídeo mais fracas. Em PCs potentes o ganho é pequeno.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Measurable,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"System\GameConfigStore",
                name: "GameDVR_Enabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
                name: "AppCaptureEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\GameDVR",
                name: "AllowGameDVR",
                value: RegValue::Dword(0),
            },
        ],
    },
    // Medido no PC do dono (08/10/2026): `AutoGameModeEnabled = 0`. O Modo de Jogo dá ao jogo a prioridade de
    // agendamento e segura atualizações do Windows e notificações durante a partida; num processador de 4 núcleos
    // dividido com navegador e Discord é a diferença entre o jogo esperar ou não. Volta ao padrão do Windows, não
    // inventa nada: por isso `RiscoDeFps::Nenhum`.
    OptimizationSpec {
        id: "windows_game_mode_on",
        name: "Ligar o Modo de Jogo do Windows",
        description: "Religa o Modo de Jogo, que o Windows traz ligado e estava desligado neste PC.",
        honest_effect: "Com o jogo aberto, o Windows dá prioridade a ele sobre os programas de fundo e segura atualizações e notificações. Ajuda mais em processador de 4 núcleos dividido com navegador e Discord; em PC folgado quase não muda nada.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\GameBar",
                name: "AutoGameModeEnabled",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\GameBar",
                name: "AllowAutoGameMode",
                value: RegValue::Dword(1),
            },
        ],
    },
    OptimizationSpec {
        id: "gpu_hardware_scheduling",
        name: "Agendamento de GPU por hardware",
        description: "Deixa a própria GPU gerenciar sua fila de tarefas, em vez da CPU.",
        honest_effect: "Reduz latência e ajuda quando a CPU é o gargalo. Exige reinício, e só é oferecido onde o driver de vídeo declara WDDM 2.7 ou mais novo — abaixo disso o Windows ignora a chave e o reinício seria por nada. Em parte das máquinas o efeito é nulo, e em algumas combinações de placa e driver ele CUSTA quadros. Por isso saiu do botão automático: ligue, reinicie e meça.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::custa(
            OQuePodeCustar::FpsMedio,
            "Depende da combinação de placa de vídeo e versão de driver. Há máquinas em \
             que ele rende e máquinas em que ele tira quadros, e não existe como saber \
             qual é a sua sem medir antes e depois.",
        ),
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: Some(Requirement::MinWddm(2700)),
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            name: "HwSchMode",
            value: RegValue::Dword(2),
        }],
    },
    OptimizationSpec {
        id: "system_responsiveness_gaming",
        name: "Prioridade do sistema para jogos",
        description: "Reduz a fatia de CPU reservada para tarefas de fundo e remove o limite de tráfego de rede em jogos.",
        honest_effect: "Usa os valores dos pacotes de otimização mais conhecidos: SystemResponsiveness 0 e NetworkThrottlingIndex desligado. Só vale para programa que se registra no agendador multimídia do Windows, e o ganho em jogo é pequeno e difícil de medir sozinho; ajuda mais em PC com poucos núcleos. Reversível; vale ao reiniciar.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::FewCores],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
                name: "SystemResponsiveness",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
                name: "NetworkThrottlingIndex",
                value: RegValue::Dword(0xFFFF_FFFF),
            },
        ],
    },
    OptimizationSpec {
        id: "disable_telemetry",
        name: "Desativar coleta de dados de diagnóstico",
        description: "Desliga os serviços DiagTrack e dmwappushservice, que enviam telemetria para a Microsoft.",
        honest_effect: "Libera CPU, disco e rede em segundo plano. Não aumenta FPS diretamente; reduz engasgos aleatórios. Não afeta o Windows Update.",
        category: Category::Privacy,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam, Boost::FewCores, Boost::MechanicalDisk],
        actions: &[
            Action::DisableService { name: "DiagTrack" },
            Action::DisableService { name: "dmwappushservice" },
        ],
    },
    OptimizationSpec {
        id: "visual_effects_performance",
        name: "Efeitos visuais para desempenho",
        description: "Desliga animações e sombras do Windows, mantendo as fontes suavizadas.",
        honest_effect: "Não muda FPS em jogos. Deixa a navegação do Windows visivelmente mais rápida em PCs fracos e com HDD.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam, Boost::FewCores, Boost::MechanicalDisk],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
                name: "VisualFXSetting",
                value: RegValue::Dword(2),
            },
            // `VisualFXSetting` é só o rótulo; os efeitos moram nesta máscara (a máquina do dono tinha rótulo 2 com máscara
            // `9E ...`). `90 12 03 80 10 00 00 00` é o que o Windows escreve em "melhor desempenho": animações, sombra, menu e
            // arrastar conteúdo desligados, suavização de fonte mantida.
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Desktop",
                name: "UserPreferencesMask",
                value: RegValue::Binary(&[0x90, 0x12, 0x03, 0x80, 0x10, 0x00, 0x00, 0x00]),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Desktop",
                name: "DragFullWindows",
                value: RegValue::Text("0"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Desktop\WindowMetrics",
                name: "MinAnimate",
                value: RegValue::Text("0"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Desktop",
                name: "MenuShowDelay",
                value: RegValue::Text("0"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
                name: "TaskbarAnimations",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\DWM",
                name: "EnableAeroPeek",
                value: RegValue::Dword(0),
            },
        ],
    },
    OptimizationSpec {
        id: "disable_startup_delay",
        name: "Remover atraso de inicialização",
        description: "Elimina o atraso artificial que o Windows aplica aos programas de inicialização.",
        honest_effect: "O desktop fica utilizável alguns segundos antes. Não reduz o tempo total de boot.",
        category: Category::Startup,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::MechanicalDisk],
        actions: &[Action::Registry {
            hive: "HKCU",
            path: r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize",
            name: "StartupDelayInMSec",
            value: RegValue::Dword(0),
        }],
    },
    OptimizationSpec {
        id: "network_low_latency",
        name: "Rede de baixa latência (desativar Nagle)",
        description: "Faz o Windows enviar pacotes pequenos imediatamente, em vez de agrupá-los.",
        honest_effect: "Pode reduzir alguns milissegundos de ping em jogos competitivos. Não aumenta a velocidade da internet nem resolve conexão ruim.",
        category: Category::Network,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::DisableNagle],
    },
    OptimizationSpec {
        id: "mouse_precision_off",
        name: "Desativar aceleração do mouse",
        description: "Remove a \"precisão aprimorada do ponteiro\", que muda a distância percorrida conforme a velocidade do movimento.",
        honest_effect: "Não muda FPS. Deixa a mira consistente: o mesmo movimento de mão passa a percorrer sempre a mesma distância. Quem está acostumado com a aceleração vai estranhar nos primeiros dias.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Mouse",
                name: "MouseSpeed",
                value: RegValue::Text("0"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Mouse",
                name: "MouseThreshold1",
                value: RegValue::Text("0"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Mouse",
                name: "MouseThreshold2",
                value: RegValue::Text("0"),
            },
        ],
    },
    // A "correção de precisão" (a ideia do MarkC Mouse Fix): com a aceleração desligada (`mouse_precision_off`), a
    // velocidade 10 de 20 (o 6º risquinho de 11 no Painel) é a única em que o Windows não multiplica nem pula
    // contagem do sensor — 1 contagem do mouse vira 1 pixel. Acima dela pula pixel; abaixo, joga contagem fora.
    OptimizationSpec {
        id: "mouse_pointer_1to1",
        name: "Ponteiro 1:1 (correção de precisão)",
        description: "Põe a velocidade do ponteiro do Windows no 6º risquinho, em que cada contagem do sensor vira exatamente um pixel.",
        honest_effect: "Não muda FPS e não mira por você. Em jogo que lê o mouse direto (CS2, Valorant) não muda nada; no FiveM, no menu e em jogo que usa o ponteiro do Windows, o movimento fica exato, sem pular pixel. Se o ponteiro ficar lento ou rápido demais, ajuste o DPI no programa do mouse em vez da velocidade do Windows.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKCU",
            path: r"Control Panel\Mouse",
            name: "MouseSensitivity",
            value: RegValue::Text("10"),
        }],
    },
    // Padrão do Windows: atraso 1 (~500 ms) e velocidade 31. O mínimo de atraso faz a tecla segurada repetir logo.
    OptimizationSpec {
        id: "keyboard_fast_repeat",
        name: "Teclado sem demora de repetição",
        description: "Põe no mínimo a espera antes de a tecla segurada começar a repetir, e no máximo a velocidade da repetição.",
        honest_effect: "Não muda FPS nem o primeiro toque, que já é imediato. Muda a tecla SEGURADA: no chat, no console e em menus ela repete na hora, sem a pausa de meio segundo. Vale depois de sair e entrar de novo no Windows.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Keyboard",
                name: "KeyboardDelay",
                value: RegValue::Text("0"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Keyboard",
                name: "KeyboardSpeed",
                value: RegValue::Text("31"),
            },
        ],
    },
    OptimizationSpec {
        id: "foreground_priority",
        name: "Prioridade para o programa em primeiro plano",
        description: "Dá fatias de CPU maiores para a janela que está em uso, em vez de dividir igualmente com o que roda atrás.",
        honest_effect: "Ganho pequeno, mais perceptível em PCs de 2 e 4 núcleos. Em processadores modernos com muitos núcleos a diferença é quase nula.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::FewCores],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\CurrentControlSet\Control\PriorityControl",
            name: "Win32PrioritySeparation",
            value: RegValue::Dword(0x26),
        }],
    },
    OptimizationSpec {
        id: "disable_xbox_services",
        name: "Desativar serviços do Xbox",
        description: "Desliga os serviços de conta, save na nuvem e rede do Xbox.",
        honest_effect: "Libera memória e processos de fundo. NÃO faça se você usa Game Pass, jogos da Microsoft Store ou controle de Xbox — eles param de funcionar. É reversível.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam],
        actions: &[
            Action::DisableService { name: "XblAuthManager" },
            Action::DisableService { name: "XblGameSave" },
            Action::DisableService { name: "XboxNetApiSvc" },
        ],
    },
    OptimizationSpec {
        id: "disable_sysmain",
        name: "Desativar SysMain (SuperFetch)",
        description: "Para o serviço que carrega programas na memória antecipadamente, prevendo o que você vai abrir.",
        honest_effect: "Em SSD o ganho de previsão é irrelevante e o serviço só consome disco e CPU. Em HD mecânico ele AJUDA — se seu PC ainda tem HD, não aplique.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: Some(Requirement::SsdSystemDrive),
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::DisableService { name: "SysMain" }],
    },
    OptimizationSpec {
        id: "disable_hibernation",
        name: "Desativar hibernação",
        description: "Remove o arquivo de hibernação, que ocupa no disco o mesmo tanto da sua memória RAM.",
        honest_effect: "Libera vários GB de disco imediatamente. Em troca, você perde a hibernação e a Inicialização Rápida — o PC vai ligar alguns segundos mais devagar. Reversível.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::DisableHibernation],
    },
    OptimizationSpec {
        id: "disable_power_throttling",
        name: "Desligar limitação de energia por processo",
        description: "Impede o Windows de reduzir a velocidade de programas que ele julga estarem em segundo plano.",
        honest_effect: "O Windows às vezes classifica errado e limita justamente o que você está usando. Desligar corrige esses casos. Em notebook, gasta mais bateria.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling",
            name: "PowerThrottlingOff",
            value: RegValue::Dword(1),
        }],
    },
    OptimizationSpec {
        id: "disable_memory_compression",
        name: "Desligar compressão de memória",
        description: "Para de comprimir páginas de memória para caber mais RAM, gastando CPU para isso.",
        honest_effect: "Só vale com RAM sobrando: troca uso de CPU por uso de memória. Com 16 GB ou mais costuma render. Com 8 GB pode PIORAR: se a memória acabar durante o jogo, o Windows vai ao disco e o jogo engasga. Se depois de aplicar aparecer travadinha, desfaça.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::custa(
            // Engasgo e não FPS médio: sem a compressão o Windows vai ao disco.
            OQuePodeCustar::Engasgo,
            "O piso de 12 GB não basta. Quem joga FiveM com navegador e Discord abertos \
             enche 16 GB, e sem a compressão o Windows passa a ir ao disco — que é \
             engasgo e queda de quadro, não ganho. Só vale se a memória sobrar DURANTE \
             o jogo, e isso se vê medindo. Com 8 GB o risco é maior.",
        ),
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::MemoryCompression { enabled: false }],
    },
    OptimizationSpec {
        id: "gpu_msi_mode",
        name: "Interrupções diretas da placa de vídeo (MSI)",
        description: "Muda como a placa de vídeo avisa o processador de que terminou uma tarefa: em vez de disputar uma fila compartilhada com outros dispositivos, cada aviso vai direto.",
        honest_effect: "Ataca engasgo e latência, não FPS médio. É o ajuste mais profundo do catálogo e quase nenhum concorrente faz, porque exige achar a placa no registro. Em raríssimos casos de driver antigo pode causar instabilidade — é reversível e exige reiniciar.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        // Fica no lote: não há evidência de que o MSI derrube quadro (o relato é de instabilidade com driver antigo).
        // Marcar risco sem prova esvazia o aviso dos que têm risco de verdade.
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::GpuMsiMode],
    },
    OptimizationSpec {
        id: "nic_power_saving_off",
        name: "Impedir que a placa de rede durma",
        description: "Desliga a economia de energia da placa de rede física.",
        honest_effect: "O Windows desliga a placa quando acha que está ociosa, e o primeiro pacote depois disso atrasa. É uma das causas reais de pico de ping no meio da partida. Em notebook na bateria, gasta um pouco mais.",
        category: Category::Network,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::NicPowerSaving],
    },
    OptimizationSpec {
        id: "background_apps_off",
        name: "Desligar aplicativos rodando em segundo plano",
        description: "Impede que aplicativos da Microsoft Store continuem executando quando você não está usando.",
        honest_effect: "Libera CPU, memória e rede de forma contínua. Em troca, os aplicativos instalados pela Store param de rodar com a janela fechada: deixam de dar notificação e de atualizar sozinhos — Correio, Calendário e Fotos são os mais afetados, e o WhatsApp instalado pela Store pode parar de avisar mensagem nova. Não mexe em programas comuns instalados fora da Store. Por isso não entra no \"Otimizar agora\": só é aplicado escolhendo este item.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam, Boost::FewCores],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
                name: "GlobalUserDisabled",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\Search",
                name: "BackgroundAppGlobalToggle",
                value: RegValue::Dword(0),
            },
        ],
    },
    OptimizationSpec {
        id: "start_menu_web_search_off",
        name: "Tirar a busca na internet do menu Iniciar",
        description: "Faz o menu Iniciar procurar só no seu PC, em vez de consultar a internet a cada letra digitada.",
        honest_effect: "Uma das mudanças mais perceptíveis do dia a dia: o menu Iniciar passa a responder na hora, em vez de esperar resposta da internet para mostrar o programa que já está instalado. Não muda FPS.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\Search",
                name: "BingSearchEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\Explorer",
                name: "DisableSearchBoxSuggestions",
                value: RegValue::Dword(1),
            },
        ],
    },
    OptimizationSpec {
        id: "delivery_optimization_off",
        name: "Parar de compartilhar atualizações com estranhos",
        description: "Desliga o envio de partes das atualizações do Windows para outros computadores pela sua internet.",
        honest_effect: "O Windows usa a SUA banda de subida para distribuir atualizações a outras máquinas na internet. Desligar libera essa banda — o que aparece como ping mais estável em jogo e upload mais livre em transmissão. Você continua recebendo todas as atualizações normalmente.",
        category: Category::Network,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization",
            name: "DODownloadMode",
            value: RegValue::Dword(0),
        }],
    },
    OptimizationSpec {
        id: "clear_boot_limits",
        name: "Liberar limites de inicialização",
        description: "Remove restrições de número de núcleos e de memória gravadas na configuração de boot do Windows.",
        honest_effect: "Se esses limites existem, seu PC está usando de propósito menos processador ou menos memória do que tem — e o ganho ao remover é enorme. Se não existem, esta opção nem aparece como disponível.",
        category: Category::System,
        expected_gain: ExpectedGain::Measurable,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::ClearBootLimits],
    },
    // Do pacote de otimização que o dono usou (11/10, `docs/app/ANALISE-PACOTES-DE-OTIMIZACAO.md`): os documentados e
    // reversíveis. Entram "só se pedir" (`CONDICIONAIS`) até a medição no FiveM decidir o lote.
    OptimizationSpec {
        id: "timer_global_win11",
        name: "Timer do Windows 11 como no Windows 10",
        description: "Faz o pedido de timer de 1 ms de um programa valer para o sistema inteiro, como era até o Windows 10.",
        honest_effect: "No Windows 11, o pedido de timer fino de um jogo só vale enquanto a janela dele está visível. Com isto, volta a valer para o sistema todo. Ajuda jogo e programa que dependem desse ritmo (limitador de FPS, áudio); custa um pouco mais de energia. No Windows 10 não muda nada.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel",
            name: "GlobalTimerResolutionRequests",
            value: RegValue::Dword(1),
        }],
    },
    OptimizationSpec {
        id: "drivers_fora_do_windows_update",
        name: "Windows Update sem trocar os drivers",
        description: "Impede o Windows Update de instalar drivers sozinho, inclusive o da placa de vídeo.",
        honest_effect: "Por si só não muda desempenho: evita que o Windows troque o driver de vídeo que você escolheu por um mais velho ou genérico no meio da semana, o que zera as configurações da placa. Em troca, driver novo de qualquer peça passa a ser por sua conta (site do fabricante ou app da NVIDIA/AMD).",
        category: Category::System,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate",
                name: "ExcludeWUDriversInQualityUpdate",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\DriverSearching",
                name: "SearchOrderConfig",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\DriverSearching",
                name: "SearchOrderConfig",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\DriverSearching",
                name: "DontSearchWindowsUpdate",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Device Metadata",
                name: "PreventDeviceMetadataFromNetwork",
                value: RegValue::Dword(1),
            },
        ],
    },
    OptimizationSpec {
        id: "sem_inicializacao_rapida",
        name: "Desligar a Inicialização Rápida",
        description: "O desligar passa a desligar de verdade, sem guardar o estado do sistema para o próximo boot.",
        honest_effect: "Com a Inicialização Rápida, \"Desligar\" guarda o núcleo do Windows em disco e o religa como estava, inclusive driver travado ou com vazamento de memória. Desligada, todo boot começa limpo; o PC demora alguns segundos a mais para ligar. Com o jogo aberto, não muda desempenho.",
        category: Category::Startup,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\CurrentControlSet\Control\Session Manager\Power",
            name: "HiberbootEnabled",
            value: RegValue::Dword(0),
        }],
    },
    OptimizationSpec {
        id: "servicos_agrupados",
        name: "Agrupar os serviços do Windows",
        description: "Os serviços do Windows voltam a dividir poucos processos, em vez de um processo para cada.",
        honest_effect: "Desde 2017, em PC com mais de 3,5 GB o Windows separa cada serviço num processo próprio (svchost), o que custa memória — uns 100 a 300 MB a mais. Agrupados, gastam menos memória; em troca, se um serviço travar, leva junto os outros do mesmo grupo. Faz diferença em PC de 8 GB rodando jogo pesado; em 16 GB ou mais, quase nada.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam],
        // Acima da memória do PC o Windows agrupa; o máximo vale para qualquer quantidade de memória.
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\CurrentControlSet\Control",
            name: "SvcHostSplitThresholdInKB",
            value: RegValue::Dword(0xFFFF_FFFF),
        }],
    },
    OptimizationSpec {
        id: "manutencao_automatica_fora",
        name: "Desligar a Manutenção Automática do Windows",
        description: "O Windows para de rodar sozinho a manutenção agendada (verificações, limpeza, otimização de disco).",
        honest_effect: "A Manutenção Automática começa quando o PC parece ocioso e pode estar rodando quando você abre o jogo, disputando disco e processador nos primeiros minutos. Desligada, ela não entra mais sozinha — inclusive a otimização semanal do SSD (TRIM), que você pode rodar à mão em \"Otimizar unidades\" de vez em quando. O Sensor de Armazenamento (limpeza automática de arquivos) também para.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Schedule\Maintenance",
                name: "MaintenanceDisabled",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\StorageSense",
                name: "AllowStorageSenseGlobal",
                value: RegValue::Dword(0),
            },
        ],
    },
    OptimizationSpec {
        id: "mpo_desligado",
        name: "Desligar as sobreposições de vários planos (MPO)",
        description: "O Windows para de usar os planos extras da placa de vídeo para mostrar janelas e vídeos.",
        honest_effect: "É a correção que a própria NVIDIA indica para tela piscando, tremor e travadinha em jogo com navegador ou Discord aberto em outro monitor. Onde não há esse problema, não muda nada ou custa um pouco: o Windows passa a compor mais coisa ele mesmo. Atenção: no Windows 11 24H2 ou mais novo a captura do Frame Boost depende do MPO — se usar o Frame Boost, meça os dois jeitos.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::custa(
            OQuePodeCustar::Engasgo,
            "Onde o MPO funciona bem, o Windows passa a compor sozinho o que ia direto para a \
             tela. No Windows 11 24H2 ou mais novo, a captura do Frame Boost fica menos confiável \
             sem ele.",
        ),
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SOFTWARE\Microsoft\Windows\Dwm",
            name: "OverlayTestMode",
            value: RegValue::Dword(5),
        }],
    },
    // Do pacote do dono (11/10), o restante dos dois pacotes. Todos "só se pedir".
    OptimizationSpec {
        id: "fila_de_entrada_curta",
        name: "Fila curta do mouse e do teclado",
        description: "Diminui para 30 a fila em que o Windows guarda os movimentos do mouse e as teclas antes de entregar ao programa.",
        honest_effect: "É o valor dos dois pacotes (o Windows usa 100). Não muda FPS. Quem usa diz que o mouse fica mais \"seco\"; não há medida pública que mostre diferença de atraso. Com mouse de taxa muito alta (4000 ou 8000 Hz) a fila curta pode perder movimento: se o ponteiro pular, desfaça. Vale ao reiniciar.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\mouclass\Parameters",
                name: "MouseDataQueueSize",
                value: RegValue::Dword(30),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\kbdclass\Parameters",
                name: "KeyboardDataQueueSize",
                value: RegValue::Dword(30),
            },
        ],
    },
    // `LowLevelHooksTimeout` (4096) do pacote "d" fica de fora: aumenta o tempo que um programa travado pode segurar
    // o mouse e o teclado. É o contrário do que o pacote quer.
    OptimizationSpec {
        id: "windows_responde_rapido",
        name: "Menus, dicas e desligamento sem espera",
        description: "Menus abrem sem atraso, a dica do mouse aparece na hora e o Windows espera menos por programa travado ao desligar.",
        honest_effect: "Não muda FPS. Muda o Windows: menu sem os 400 ms de espera, dica sem meio segundo de espera, e no desligar o Windows espera 5 s (e não 20) por programa e 2 s por serviço, e fecha sozinho o que travou. O PREÇO: programa com trabalho não salvo é fechado no desligamento em vez de perguntar. Parte vale depois de sair e entrar no Windows; o desligamento, depois de reiniciar.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Desktop",
                name: "MenuShowDelay",
                value: RegValue::Text("0"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Mouse",
                name: "MouseHoverTime",
                value: RegValue::Text("10"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Desktop",
                name: "WaitToKillAppTimeout",
                value: RegValue::Text("5000"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Desktop",
                name: "HungAppTimeout",
                value: RegValue::Text("4000"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\Desktop",
                name: "AutoEndTasks",
                value: RegValue::Text("1"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Control",
                name: "WaitToKillServiceTimeout",
                value: RegValue::Text("2000"),
            },
        ],
    },
    OptimizationSpec {
        id: "prefetch_desligado",
        name: "Desligar o Prefetch",
        description: "O Windows para de anotar o que cada programa lê ao abrir para adiantar a leitura na próxima vez.",
        honest_effect: "Está no guia. Com o jogo aberto não muda desempenho: o Prefetch só trabalha ao abrir programa e no boot. Em SSD a diferença é pequena; em HD programas e jogos passam a abrir mais devagar. Vale ao reiniciar.",
        category: Category::System,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management\PrefetchParameters",
                name: "EnablePrefetcher",
                value: RegValue::Dword(0),
            },
        ],
    },
    // `graphicsCaptureWithoutBorder` negado fica de fora: impede capturar janela sem a borda amarela, que o Frame Boost
    // e programas de gravação podem usar.
    OptimizationSpec {
        id: "privacidade_do_pacote",
        name: "Permissões de privacidade do guia",
        description: "Nega aos aplicativos da loja localização, contatos, agenda, e-mail, arquivos e diagnóstico, e desliga a coleta de digitação, as dicas e o histórico de atividades.",
        honest_effect: "Não muda desempenho: é a parte de privacidade do guia. Câmera e microfone continuam liberados. O PREÇO: aplicativos da loja que dependem dessas permissões (Clima com a sua cidade, Mapas, Pessoas, Correio, Calendário) param de ver esses dados até você liberar de novo em Configurações > Privacidade.",
        category: Category::Privacy,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\userNotificationListener",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\userAccountInformation",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\contacts",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\appointments",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\phoneCall",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\phoneCallHistory",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\email",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\userDataTasks",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\chat",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\radios",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\appDiagnostics",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\documentsLibrary",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\downloadsFolder",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\musicLibrary",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\picturesLibrary",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\videosLibrary",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\broadFileSystemAccess",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\bluetoothSync",
                name: "Value",
                value: RegValue::Text("Deny"),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Speech_OneCore\Settings\VoiceActivation\UserPreferenceForAllApps",
                name: "AgentActivationEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Control Panel\International\User Profile",
                name: "HttpAcceptLanguageOptOut",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Policies\Microsoft\Windows\EdgeUI",
                name: "DisableMFUTracking",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\EdgeUI",
                name: "DisableMFUTracking",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\InputPersonalization",
                name: "RestrictImplicitInkCollection",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\InputPersonalization",
                name: "RestrictImplicitTextCollection",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\InputPersonalization\TrainedDataStore",
                name: "HarvestContacts",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Siuf\Rules",
                name: "NumberOfSIUFInPeriod",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\System",
                name: "PublishUserActivities",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\SearchSettings",
                name: "IsAADCloudSearchEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\SearchSettings",
                name: "IsMSACloudSearchEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\SearchSettings",
                name: "IsDynamicSearchBoxEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\SmartActionPlatform\SmartClipboard",
                name: "Disabled",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\input\Settings",
                name: "InsightsEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager",
                name: "SubscribedContent-338389Enabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement",
                name: "ScoobeSystemSettingEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer",
                name: "AllowOnlineTips",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\SQMClient\Windows",
                name: "CEIPEnable",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\SQMClient\Windows",
                name: "CEIPEnable",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
                name: "POWERSHELL_TELEMETRY_OPTOUT",
                value: RegValue::Text("1"),
            },
        ],
    },
    OptimizationSpec {
        id: "servicos_bluetooth_fora",
        name: "Desativar os serviços de Bluetooth",
        description: "Desliga o serviço de suporte ao Bluetooth e o de áudio Bluetooth.",
        honest_effect: "Só para quem NÃO usa Bluetooth: fone, controle, caixa de som e mouse sem fio por Bluetooth param de conectar. Mouse e teclado com receptor USB não são afetados. Libera pouco: os dois serviços quase não gastam com nada conectado. Reversível.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::DisableService { name: "bthserv" },
            Action::DisableService { name: "BTAGService" },
        ],
    },
    OptimizationSpec {
        id: "servicos_impressora_fora",
        name: "Desativar impressora, scanner e fax",
        description: "Desliga o spooler de impressão, os avisos de impressora, a aquisição de imagens (scanner) e o fax.",
        honest_effect: "Só para quem NÃO imprime nem digitaliza: a impressora some do PC, inclusive \"Imprimir em PDF\". Libera pouca memória. Reversível.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::DisableService { name: "Spooler" },
            Action::DisableService { name: "PrintNotify" },
            Action::DisableService { name: "stisvc" },
            Action::DisableService { name: "Fax" },
        ],
    },
    OptimizationSpec {
        id: "servicos_hyperv_fora",
        name: "Desativar os serviços de máquina virtual (Hyper-V)",
        description: "Desliga o serviço de host do Hyper-V e os serviços de integração usados dentro de máquinas virtuais.",
        honest_effect: "Só para quem NÃO usa máquina virtual, WSL, Docker ou emulador de Android com Hyper-V. Não desliga o Isolamento de Núcleo (isso é outro item). Fora de máquina virtual os serviços de integração nem rodam, então o ganho é pequeno. Reversível.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::DisableService { name: "HvHost" },
            Action::DisableService { name: "vmickvpexchange" },
            Action::DisableService { name: "vmicguestinterface" },
            Action::DisableService { name: "vmicshutdown" },
            Action::DisableService { name: "vmicheartbeat" },
            Action::DisableService { name: "vmicvmsession" },
            Action::DisableService { name: "vmicrdv" },
            Action::DisableService { name: "vmictimesync" },
            Action::DisableService { name: "vmicvss" },
        ],
    },
    OptimizationSpec {
        id: "servicos_pouco_usados",
        name: "Desativar serviços que quase ninguém usa",
        description: "Desliga serviços que os dois pacotes desligam: localização, telefone, áudio de rede (QWAVE), leitor biométrico, cache de fontes, assistente de compatibilidade, diagnóstico e outros.",
        honest_effect: "Cada um gasta pouco; juntos são menos processos acordando com o PC. O PREÇO, item por item: a localização para (Clima, Mapas, \"Encontrar meu dispositivo\"); a digital e o rosto do Windows Hello param; o assistente de compatibilidade deixa de corrigir programa antigo; o solucionador de problemas de rede para; o roteador móvel (hotspot) e o compartilhamento de internet param; fontes podem demorar um pouco mais na primeira abertura. Reversível.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::DisableService { name: "lfsvc" },
            Action::DisableService { name: "PhoneSvc" },
            Action::DisableService { name: "QWAVE" },
            Action::DisableService { name: "GraphicsPerfSvc" },
            Action::DisableService { name: "PcaSvc" },
            Action::DisableService { name: "Wecsvc" },
            Action::DisableService { name: "WbioSrvc" },
            Action::DisableService { name: "FontCache" },
            Action::DisableService { name: "FontCache3.0.0.0" },
            Action::DisableService { name: "MapsBroker" },
            Action::DisableService { name: "RetailDemo" },
            Action::DisableService { name: "RemoteRegistry" },
            Action::DisableService { name: "WMPNetworkSvc" },
            Action::DisableService { name: "AJRouter" },
            Action::DisableService { name: "WpcMonSvc" },
            Action::DisableService { name: "diagsvc" },
            Action::DisableService { name: "DPS" },
            Action::DisableService { name: "WdiServiceHost" },
            Action::DisableService { name: "SEMgrSvc" },
            Action::DisableService { name: "icssvc" },
            Action::DisableService { name: "SharedAccess" },
            Action::DisableService { name: "diagnosticshub.standardcollector.service" },
            Action::DisableService { name: "DialogBlockingService" },
            Action::DisableService { name: "EntAppSvc" },
            Action::DisableService { name: "AppVClient" },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Policies\Microsoft\Windows\AppCompat",
                name: "DisablePCA",
                value: RegValue::Dword(1),
            },
        ],
    },
    OptimizationSpec {
        id: "servicos_de_empresa_fora",
        name: "Desativar serviços de empresa (cartão inteligente, área de trabalho remota, domínio)",
        description: "Desliga cartão inteligente, Área de Trabalho Remota, Acesso Remoto, logon em domínio, arquivos offline e o agente SSH.",
        honest_effect: "Não muda desempenho: são serviços que num PC de casa ficam parados. Só para PC pessoal: em PC de empresa, de escola ou que você acessa de longe pela Área de Trabalho Remota, isto quebra o login e o acesso. Reversível.",
        category: Category::System,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::DisableService { name: "CertPropSvc" },
            Action::DisableService { name: "SCardSvr" },
            Action::DisableService { name: "ScDeviceEnum" },
            Action::DisableService { name: "SCPolicySvc" },
            Action::DisableService { name: "TermService" },
            Action::DisableService { name: "SessionEnv" },
            Action::DisableService { name: "RemoteAccess" },
            Action::DisableService { name: "Netlogon" },
            Action::DisableService { name: "CscService" },
            Action::DisableService { name: "ssh-agent" },
        ],
    },
    OptimizationSpec {
        id: "placa_no_clock_maximo",
        name: "Placa de vídeo NVIDIA sempre no clock máximo",
        description: "Desliga a troca dinâmica de estado de energia (P-State) da placa NVIDIA, que fica no clock de jogo o tempo todo.",
        honest_effect: "Do guia (DisableDynamicPstate). Tira a demora da placa para subir de clock quando a cena pesa de repente, que aparece como travadinha no começo da luta ou ao virar a câmera. O PREÇO: a placa gasta mais energia e esquenta mais parada na área de trabalho, e a ventoinha pode girar mais. Só placas NVIDIA. Vale ao reiniciar.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::custa(
            OQuePodeCustar::Boost,
            "Placa sempre quente chega à partida com menos folga de temperatura: em notebook e em \
             gabinete abafado ela pode baixar o clock no meio da partida.",
        ),
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::ClockMaximoDaPlaca,
        ],
    },
    OptimizationSpec {
        id: "tick_do_pacote",
        name: "Relógio do Windows sem economia (tick fixo)",
        description: "Desliga o tick dinâmico, usa o tick da plataforma e o sincronismo antigo do contador do processador, como os pacotes fazem na configuração de boot.",
        honest_effect: "Dos dois pacotes (bcdedit: disabledynamictick, useplatformtick e tscsyncpolicy legacy). O Windows para de espaçar o relógio interno para economizar energia. Há quem sinta o jogo mais regular; há PCs em que o tick da plataforma deixa tudo menos regular. Não há medida pública que decida, então meça o seu. Em notebook gasta mais bateria. Vale ao reiniciar.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::custa(
            OQuePodeCustar::Engasgo,
            "Em parte dos PCs o tick da plataforma é mais grosso que o do processador, e o jogo \
             fica menos regular. Meça antes e depois.",
        ),
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::BootValor { nome: "disabledynamictick", valor: "yes" },
            Action::BootValor { nome: "useplatformtick", valor: "yes" },
            Action::BootValor { nome: "tscsyncpolicy", valor: "legacy" },
        ],
    },
    OptimizationSpec {
        id: "tcp_do_pacote",
        name: "Parâmetros de TCP do pacote",
        description: "Mais portas de saída, porta liberada mais cedo depois de fechar a conexão, TTL 64 e a ordem de resolução de nomes dos pacotes.",
        honest_effect: "Não muda desempenho de jogo nem o ping: o ping é a distância até o servidor. Ajuda só quem abre muitas conexões ao mesmo tempo (servidor, torrent). Reversível; vale ao reiniciar.",
        category: Category::Network,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
                name: "TcpTimedWaitDelay",
                value: RegValue::Dword(30),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
                name: "MaxUserPort",
                value: RegValue::Dword(65534),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
                name: "DefaultTTL",
                value: RegValue::Dword(64),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\Tcpip\ServiceProvider",
                name: "LocalPriority",
                value: RegValue::Dword(4),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\Tcpip\ServiceProvider",
                name: "HostsPriority",
                value: RegValue::Dword(5),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\Tcpip\ServiceProvider",
                name: "DnsPriority",
                value: RegValue::Dword(6),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\Tcpip\ServiceProvider",
                name: "NetbtPriority",
                value: RegValue::Dword(7),
            },
        ],
    },
    OptimizationSpec {
        id: "compartilhamento_do_pacote",
        name: "Compartilhamento de arquivos do pacote",
        description: "Ajusta o servidor de compartilhamento de pastas do Windows como o guia: sem desconexão automática, sem oplocks e sem espera em conflito de arquivo.",
        honest_effect: "Não muda desempenho de jogo: só vale se este PC compartilha pastas na rede local. Sem oplocks, quem abre arquivo da sua pasta pela rede fica um pouco mais lento. Reversível; vale ao reiniciar.",
        category: Category::Network,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters",
                name: "autodisconnect",
                value: RegValue::Dword(0xFFFF_FFFF),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters",
                name: "Size",
                value: RegValue::Dword(3),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters",
                name: "EnableOplocks",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters",
                name: "IRPStackSize",
                value: RegValue::Dword(32),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters",
                name: "SharingViolationDelay",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters",
                name: "SharingViolationRetries",
                value: RegValue::Dword(0),
            },
        ],
    },
    // O `Chimney` do guia fica de fora: o Windows 10 e 11 não têm mais o recurso. O MTU 1500 também: já é o padrão.
    OptimizationSpec {
        id: "bufferbloat_do_pacote",
        name: "Rede do guia \"Bufferbloat\"",
        description: "Aplica o conjunto de TCP do guia: controle de congestionamento CTCP, retransmissão mais curta, ECN e timestamps, sem ajuste automático de janela, e sem RSS, RSC, LSO e checksum na placa de rede.",
        honest_effect: "Ataca o bufferbloat (ping que sobe quando alguém usa a internet ao mesmo tempo), e o efeito depende do roteador e da operadora: teste num site de bufferbloat antes e depois. O PREÇO: sem ajuste automático de janela o download fica mais lento, e sem RSS e sem os offloads o trabalho da placa de rede passa para o processador. Em jogo preso no processador, como o FiveM, isso pode custar quadro. Ao aplicar e ao desfazer a placa de rede reinicia e a internet cai por alguns segundos: não aplique no meio da partida. Reversível.",
        category: Category::Network,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::custa(
            OQuePodeCustar::FpsMedio,
            "Sem RSS e sem os offloads o processador faz o trabalho da placa de rede. Em jogo \
             preso no processador e com muito tráfego (FiveM cheio), isso tira tempo do jogo.",
        ),
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::RedeTcp { parametro: "CongestionProvider", valor: "CTCP" },
            Action::RedeTcp { parametro: "InitialRto", valor: "2000" },
            Action::RedeTcp { parametro: "MinRto", valor: "300" },
            Action::RedeTcp { parametro: "MaxSynRetransmissions", valor: "2" },
            Action::RedeTcp { parametro: "NonSackRttResiliency", valor: "Disabled" },
            Action::RedeTcp { parametro: "Timestamps", valor: "Enabled" },
            Action::RedeTcp { parametro: "EcnCapability", valor: "Enabled" },
            Action::RedeTcp { parametro: "ScalingHeuristics", valor: "Disabled" },
            Action::RedeTcp { parametro: "AutoTuningLevelLocal", valor: "Disabled" },
            Action::RedeOffload { parametro: "ReceiveSideScaling", valor: "Disabled" },
            Action::RedeOffload { parametro: "ReceiveSegmentCoalescing", valor: "Disabled" },
            Action::RedeAdaptador { recurso: "Lso" },
            Action::RedeAdaptador { recurso: "ChecksumOffload" },
        ],
    },
    // Só os valores gerais: `Parents` e `Children` do .reg são os jogos do PC de quem o fez.
    OptimizationSpec {
        id: "tela_cheia_sem_otimizacao",
        name: "Desligar as otimizações de tela cheia do Windows",
        description: "Os valores do \"Disabilitar Game DVR.reg\": o Windows deixa de trocar a tela cheia exclusiva do jogo pela tela cheia otimizada dele.",
        honest_effect: "Em jogo com tela cheia exclusiva de verdade, a imagem vai direto para o monitor, sem passar pelo compositor do Windows. No FiveM do dono a tela cheia exclusiva deu 85 fps contra 61 da janela sem bordas, mas isso é a opção do jogo; esta chave só impede o Windows de \"converter\". Em jogo que já roda sem bordas não muda nada. O PREÇO: Alt+Tab fica mais lento e sobreposições (Discord, Game Bar) podem não aparecer. Vale ao abrir o jogo de novo.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"System\GameConfigStore",
                name: "GameDVR_FSEBehavior",
                value: RegValue::Dword(2),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"System\GameConfigStore",
                name: "GameDVR_FSEBehaviorMode",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"System\GameConfigStore",
                name: "GameDVR_HonorUserFSEBehaviorMode",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"System\GameConfigStore",
                name: "GameDVR_DXGIHonorFSEWindowsCompatible",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"System\GameConfigStore",
                name: "GameDVR_EFSEFeatureFlags",
                value: RegValue::Dword(0),
            },
        ],
    },
    OptimizationSpec {
        id: "mitigacoes_spectre_fora",
        name: "Desligar as proteções contra Spectre e Meltdown",
        description: "Tira as proteções do Windows contra as falhas Spectre (variante 2) e Meltdown dos processadores.",
        honest_effect: "ATENÇÃO, ISTO REDUZ A SEGURANÇA DO SEU PC. Essas proteções custam desempenho a cada vez que um programa chama o Windows, e o custo é maior nos Intel da 6ª à 10ª geração (como i3-10100F) e em jogo que faz muita chamada ao sistema, como o FiveM. Desligadas, um programa malicioso rodando no seu PC consegue ler memória de outros programas (senhas, chaves). Reversível; vale ao reiniciar. Só faça se você não roda programa de origem duvidosa.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Measurable,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: true,
        highlight_when: &[],
        // Microsoft KB4072698: 3 + máscara 3 desliga as duas (variante 2 e Meltdown).
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management",
                name: "FeatureSettingsOverride",
                value: RegValue::Dword(3),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management",
                name: "FeatureSettingsOverrideMask",
                value: RegValue::Dword(3),
            },
        ],
    },
    OptimizationSpec {
        id: "disable_vbs",
        name: "Desligar virtualização de segurança (VBS)",
        description: "Desativa a camada de virtualização que o Windows usa para isolar credenciais e validar código do núcleo.",
        honest_effect: "ATENÇÃO, ISTO REDUZ A SEGURANÇA DO SEU PC. O VBS protege suas senhas do Windows contra roubo por programa malicioso e barra exploração do núcleo do sistema. Desligar rende FPS de verdade, mais em processadores de 8ª a 10ª geração, e é reversível — mas é uma troca de proteção por desempenho, não almoço grátis. Se você usa Hyper-V, WSL ou Sandbox, eles param de funcionar.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Measurable,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: true,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Control\DeviceGuard",
                name: "EnableVirtualizationBasedSecurity",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
                name: "Enabled",
                value: RegValue::Dword(0),
            },
            // Sem esta linha o hipervisor continua subindo e o desempenho prometido não volta.
            Action::DisableHypervisor,
        ],
    },
    OptimizationSpec {
        id: "disable_search_indexing",
        name: "Desligar a indexação de busca do Windows",
        description: "Para o serviço que fica lendo seus arquivos em segundo plano para montar o índice de busca.",
        honest_effect: "Em PC fraco e em disco mecânico é das mudanças que mais aliviam, porque o indexador lê disco e gasta CPU sem hora marcada. Em troca, procurar arquivo pelo Explorador fica lento — ele passa a varrer as pastas na hora. Se você usa muito a busca do Windows, não vale.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::MechanicalDisk, Boost::FewCores],
        actions: &[Action::DisableService { name: "WSearch" }],
    },
    OptimizationSpec {
        id: "disable_transparency",
        name: "Desligar a transparência das janelas",
        description: "Remove o efeito de vidro fosco do menu Iniciar, da barra de tarefas e das janelas.",
        honest_effect: "O efeito é desenhado pela placa de vídeo a cada quadro. Em vídeo integrado e em PC fraco isso aparece: menus e janelas abrem sem arrastar. Em PC com placa dedicada, o ganho é próximo de zero.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam, Boost::FewCores],
        actions: &[Action::Registry {
            hive: "HKCU",
            path: r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            name: "EnableTransparency",
            value: RegValue::Dword(0),
        }],
    },
    OptimizationSpec {
        id: "disable_reserved_storage",
        name: "Liberar o Armazenamento Reservado",
        description: "Devolve os gigabytes que o Windows reserva no disco só para instalar atualizações futuras.",
        honest_effect: "Costuma devolver de 7 a 10 GB de disco imediatamente. Não aumenta FPS: o ganho é espaço, e num SSD pequeno e cheio isso muda a vida do PC. As atualizações continuam funcionando — o Windows passa a usar o espaço livre comum em vez de um pedaço separado.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam, Boost::MechanicalDisk],
        actions: &[Action::ReservedStorage { enabled: false }],
    },
    OptimizationSpec {
        id: "remove_forced_hpet",
        name: "Remover relógio de plataforma forçado (HPET)",
        description: "Desfaz a configuração que obriga o Windows a usar um temporizador de hardware mais lento.",
        honest_effect: "Isto não é um ajuste, é um conserto. Forçar o HPET é uma das dicas de FPS mais repetidas da internet e uma das mais erradas: o Windows já escolhe o melhor temporizador sozinho, e forçar costuma causar engasgo. Se a opção não estiver no seu PC, esta linha nem aparece como disponível. Reversível: guardamos o valor que estava lá.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Measurable,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::RemoveForcedPlatformClock],
    },
    OptimizationSpec {
        id: "mmcss_games",
        name: "Perfil de multimídia para jogos",
        description: "Ajusta a prioridade que o Windows dá a jogos na fila de processador, de vídeo e de disco.",
        honest_effect: "Complementa a prioridade do sistema para jogos, que sozinha só reserva menos CPU para o fundo. Ganho pequeno e difícil de isolar, mais perceptível em PC com poucos núcleos. Exige reiniciar.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::FewCores],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
                name: "GPU Priority",
                value: RegValue::Dword(8),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
                name: "Priority",
                value: RegValue::Dword(6),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
                name: "Scheduling Category",
                value: RegValue::Text("High"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
                name: "SFIO Priority",
                value: RegValue::Text("High"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
                name: "Affinity",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
                name: "Background Only",
                value: RegValue::Text("False"),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
                name: "Clock Rate",
                value: RegValue::Dword(10000),
            },
        ],
    },
    OptimizationSpec {
        id: "stop_sponsored_apps",
        name: "Impedir o Windows de instalar aplicativos sozinho",
        description: "Desliga a instalação automática de jogos e aplicativos patrocinados que o Windows coloca no menu Iniciar.",
        honest_effect: "O Windows instala aplicativos patrocinados sem pedir, e eles voltam depois de cada atualização grande. Esta é a única maneira de a limpeza durar: sem ela, o que você desinstalar hoje reaparece. Também tira as sugestões do menu Iniciar.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam, Boost::MechanicalDisk],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager",
                name: "SilentInstalledAppsEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager",
                name: "PreInstalledAppsEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager",
                name: "OemPreInstalledAppsEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager",
                name: "SubscribedContent-338388Enabled",
                value: RegValue::Dword(0),
            },
        ],
    },
    OptimizationSpec {
        id: "disable_widgets",
        name: "Desligar os Widgets da barra de tarefas",
        description: "Remove o painel de notícias e clima que fica ao lado do menu Iniciar no Windows 11.",
        honest_effect: "O painel mantém um processo próprio carregando conteúdo da internet em segundo plano, e reaparece sozinho ao passar o mouse. Em PC com pouca memória o alívio é perceptível. Em Windows 10 sem o recurso, não muda nada.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam, Boost::FewCores],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SOFTWARE\Policies\Microsoft\Dsh",
            name: "AllowNewsAndInterests",
            value: RegValue::Dword(0),
        }],
    },
    OptimizationSpec {
        id: "disable_copilot",
        name: "Desligar o Copilot do Windows",
        description: "Remove o assistente de inteligência artificial da barra de tarefas do Windows 11.",
        honest_effect: "Libera memória e o processo que fica pronto em segundo plano. Se você usa o Copilot, não aplique: o ganho não compensa perder uma ferramenta que você usa de verdade.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::LowRam],
        actions: &[Action::Registry {
            hive: "HKCU",
            path: r"Software\Policies\Microsoft\Windows\WindowsCopilot",
            name: "TurnOffWindowsCopilot",
            value: RegValue::Dword(1),
        }],
    },
    OptimizationSpec {
        id: "telemetry_policy",
        name: "Fixar a coleta de dados no mínimo",
        description: "Grava na política do sistema o nível mínimo de envio de dados de diagnóstico.",
        honest_effect: "Sozinho não muda desempenho. O valor está em fazer a desativação da telemetria DURAR: sem a política, o serviço volta a ser reativado em atualizações grandes do Windows. É o cinto que segura o outro ajuste.",
        category: Category::Privacy,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
            name: "AllowTelemetry",
            value: RegValue::Dword(0),
        }],
    },
    // O catálogo do mercado: entra porque o cliente compara lista com lista, com classificação honesta (a maioria
    // `NoGain`, e a tela conta quantos antes do clique).

    OptimizationSpec {
        id: "notifications_off",
        name: "Desligar notificações do Windows",
        description: "Impede que avisos do sistema e de aplicativos apareçam no canto da tela.",
        honest_effect: "Não muda FPS e não libera memória. O que resolve é a notificação que rouba o foco durante um jogo em tela cheia sem bordas — que causa engasgo, mas não é perda de desempenho contínua. A central de notificações some da barra de tarefas. Os avisos de segurança do Windows continuam aparecendo.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\PushNotifications",
                name: "ToastEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\Explorer",
                name: "DisableNotificationCenter",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\CurrentVersion\PushNotifications",
                name: "NoCloudApplicationNotification",
                value: RegValue::Dword(1),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel",
                name: "Enabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\Windows.SystemToast.CapabilityAccess",
                name: "Enabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\Windows.SystemToast.StartupApp",
                name: "Enabled",
                value: RegValue::Dword(0),
            },
        ],
    },
    OptimizationSpec {
        id: "store_auto_download_off",
        name: "Desligar atualização automática da Microsoft Store",
        description: "A Store baixa e instala atualização de aplicativo sozinha, em segundo plano.",
        honest_effect: "Não muda FPS. Pesa de verdade em disco mecânico e em internet lenta, onde uma atualização baixando no meio da partida disputa o disco com o jogo. Em SSD com internet boa a diferença é imperceptível.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[Boost::MechanicalDisk],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SOFTWARE\Policies\Microsoft\WindowsStore",
            name: "AutoDownload",
            value: RegValue::Dword(2),
        }],
    },
    OptimizationSpec {
        id: "maps_auto_update_off",
        name: "Desligar atualização automática de mapas",
        description: "O aplicativo Mapas do Windows baixa atualização de mapa em segundo plano.",
        honest_effect: "Praticamente ninguém usa o Mapas do Windows. Se você nunca abriu esse aplicativo, não há mapa nenhum sendo baixado e desligar isto não muda absolutamente nada. Está aqui porque o mercado oferece.",
        category: Category::System,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\Maps",
            name: "AutoUpdateEnabled",
            value: RegValue::Dword(0),
        }],
    },
    OptimizationSpec {
        id: "settings_sync_off",
        name: "Desligar sincronização de configurações",
        description: "Impede o Windows de copiar tema, senha e preferências para a conta Microsoft.",
        honest_effect: "Não muda desempenho. É privacidade: as suas configurações param de subir para a nuvem. Em conta local isto já não fazia nada.",
        category: Category::Privacy,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\SettingSync",
                name: "DisableSettingSync",
                value: RegValue::Dword(2),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Windows\SettingSync",
                name: "DisableSettingSyncUserOverride",
                value: RegValue::Dword(1),
            },
        ],
    },
    OptimizationSpec {
        id: "remote_assistance_off",
        name: "Desligar Assistência Remota",
        description: "Impede que alguém peça acesso remoto a este PC pelo recurso nativo do Windows.",
        honest_effect: "Não muda desempenho. É segurança: fecha um caminho de acesso remoto que quase ninguém usa e que golpista de suporte falso usa. O acesso remoto do TeamViewer, AnyDesk e afins não é afetado.",
        category: Category::Privacy,
        expected_gain: ExpectedGain::NoGain,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\CurrentControlSet\Control\Remote Assistance",
            name: "fAllowToGetHelp",
            value: RegValue::Dword(0),
        }],
    },

    // No lugar de "desligar UAC" e "desligar firewall" (ganho nulo, custo em segurança): chaves documentadas, efeito
    // perceptível, reversíveis, preço escrito. Nenhum promete FPS.

    OptimizationSpec {
        id: "error_reporting_off",
        name: "Desligar o Relatório de Erros do Windows",
        description: "Impede o Windows de abrir o coletor de falhas (WerFault) quando um programa fecha sozinho.",
        honest_effect: "Não dá FPS — o coletor só existe depois que alguma coisa já quebrou. O que ele muda é o que acontece NA HORA em que o jogo fecha sozinho: hoje o Windows sobe o WerFault, que segura a janela travada enquanto copia a memória do processo para o disco e tenta enviar o relatório. Com jogo grande isso é gigabyte de escrita e dezenas de segundos de PC parado, e é por isso que a queda parece muito pior do que foi. Desligado, o jogo simplesmente fecha e você volta para a área de trabalho. O PREÇO: você perde o registro daquela falha. O Histórico de Confiabilidade do Windows para de anotar, e se um dia precisar descobrir POR QUE aquele jogo cai, a informação não vai estar lá — e aí o caminho é ligar de volta e reproduzir a queda. Junto sai o Fault Tolerant Heap, que o Windows liga sozinho num programa que caiu várias vezes e que deixa esse programa um pouco mais lento.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\Windows\Windows Error Reporting",
                name: "Disabled",
                value: RegValue::Dword(1),
            },
            Action::DisableService { name: "WerSvc" },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Microsoft\FTH",
                name: "Enabled",
                value: RegValue::Dword(0),
            },
        ],
    },

    OptimizationSpec {
        id: "edge_background_off",
        name: "Impedir os navegadores de ficar abertos por trás e de carregar no boot",
        description: "Desliga o modo em segundo plano e a pré-carga no boot do Edge e do Chrome, e a atualização do Firefox com ele fechado.",
        honest_effect: "O Edge tem dois comportamentos ligados de fábrica que quase ninguém sabe que existem: ele se carrega sozinho durante o boot para abrir mais rápido depois, e continua com processos vivos depois que você fecha a última janela. Juntos costumam segurar algumas centenas de megabytes e um punhado de processos que você não pediu. Desligando os dois, esses processos somem quando você fecha o navegador. NÃO ESPERE FPS: memória liberada só vira quadro em máquina que estava realmente sem memória, e em PC com folga a diferença no jogo é nenhuma. O que muda de verdade é o boot e o que fica rodando enquanto você joga. O PREÇO, e ele aparece na cara: isto é escrito como política de máquina, que é a única forma que o Edge lê do registro — então o navegador vai passar a mostrar \"gerenciado pela sua organização\" nas configurações dele. Não é vírus e não é bloqueio; é o Edge dizendo que a configuração veio de fora. Desfazendo aqui, o aviso some. O mesmo vale para o Chrome. O Firefox passa a se atualizar só aberto.",
        category: Category::Startup,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        highlight_when: &[Boost::LowRam],
        security_tradeoff: false,
        actions: &[
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Edge",
                name: "StartupBoostEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Edge",
                name: "BackgroundModeEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Google\Chrome",
                name: "StartupBoostEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Google\Chrome",
                name: "BackgroundModeEnabled",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Mozilla\Firefox",
                name: "BackgroundAppUpdate",
                value: RegValue::Dword(0),
            },
        ],
    },

    // Sem "desligar o Windows Defender": desde o 1903 a Proteção contra Adulteração ignora `DisableAntiSpyware` e
    // reverte sozinha; só o usuário a desliga à mão. O botão mentiria. O caminho honesto seria um aviso.

    OptimizationSpec {
        id: "accessibility_keys_off",
        name: "Desligar teclas de acessibilidade acionadas sem querer",
        description: "Desliga a Filtragem de Teclas e as Teclas de Aderência, que o Windows liga sozinho por atalho de teclado.",
        honest_effect: "Só aparece se alguma delas estiver LIGADA na sua máquina — e quando está, o teclado atrasa de verdade: a Filtragem de Teclas chega a ignorar toques por um segundo inteiro. Ela liga sozinha ao segurar o Shift por oito segundos, o que acontece jogando sem ninguém perceber. Não muda FPS: muda o teclado responder na hora. Se você USA esses recursos por necessidade, não aplique — eles existem por um bom motivo.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::AccessibilityKeysOff],
    },
    OptimizationSpec {
        id: "windowed_game_optimizations",
        name: "Otimizações para jogos em janela",
        description: "Liga a opção do Windows 11 que tira o atraso dos jogos em tela cheia sem borda.",
        honest_effect: "Vale para jogos DirectX 10 e 11 em janela ou em tela cheia sem borda, o modo em que o FiveM costuma rodar. O ganho é menos atraso entre o comando e a imagem, e o VRR (G-Sync, FreeSync) passa a funcionar nesse modo. Não é um ajuste para aumentar o FPS, e o Otimiza não mediu o efeito dele na sua máquina: aplique um a um e compare. Jogo em tela cheia exclusiva e em DirectX 12 já usa esse caminho. Só existe no Windows 11. Se algum jogo passar a piscar, desfaça.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Responsiveness,
        risco_de_fps: RiscoDeFps::Nenhum,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        highlight_when: &[],
        actions: &[Action::JanelasOtimizadas],
    },
];

pub fn find(id: &str) -> Option<&'static OptimizationSpec> {
    CATALOG.iter().find(|spec| spec.id == id)
}

#[cfg(test)]
mod tests_1_6 {
    use super::*;

    #[test]
    fn efeitos_visuais_escrevem_a_mascara_que_realmente_governa() {
        let spec = find("visual_effects_performance").expect("otimização deveria existir");

        assert!(
            spec.actions.iter().any(|acao| matches!(
                acao,
                Action::Registry { name, value: RegValue::Binary(_), .. }
                    if *name == "UserPreferencesMask"
            )),
            "sem UserPreferencesMask a otimização muda o rótulo e não o comportamento"
        );
    }

    #[test]
    fn efeitos_visuais_desligam_o_arrasto_de_janela_cheia() {
        let spec = find("visual_effects_performance").expect("otimização deveria existir");

        assert!(spec.actions.iter().any(|acao| matches!(
            acao,
            Action::Registry { name, value: RegValue::Text("0"), .. }
                if *name == "DragFullWindows"
        )));
    }

    #[test]
    fn desligar_vbs_tambem_impede_o_hipervisor_de_subir() {
        let spec = find("disable_vbs").expect("otimização deveria existir");

        assert!(
            spec.actions
                .iter()
                .any(|acao| matches!(acao, Action::DisableHypervisor)),
            "sem desligar o hipervisor, o cliente perde a proteção e não ganha o desempenho"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_ids_are_unique() {
        let ids: HashSet<&str> = CATALOG.iter().map(|spec| spec.id).collect();
        assert_eq!(
            ids.len(),
            CATALOG.len(),
            "ids duplicados tornam o rollback ambíguo"
        );
    }

    #[test]
    fn every_optimization_has_an_honest_effect() {
        for spec in CATALOG {
            assert!(
                !spec.honest_effect.trim().is_empty(),
                "{} não descreve o ganho real",
                spec.id
            );
        }
    }

    /// Ajuste que troca segurança PRECISA entregar algo: `NoGain` com `security_tradeoff` é custo sem contrapartida
    /// (substituiu o teste do UAC, para os dois não voltarem por descuido).
    #[test]
    fn nenhum_ajuste_cobra_seguranca_sem_entregar_nada() {
        for spec in CATALOG {
            assert!(
                !(spec.security_tradeoff && spec.expected_gain == ExpectedGain::NoGain),
                "`{}` pede segurança em troca de ganho declarado nulo — isso é recusa, \
                 e o lugar dela é `naofazemos.rs`",
                spec.id
            );
        }
    }

    #[test]
    fn o_que_saiu_do_catalogo_esta_explicado_na_lista_de_recusas() {
        // Estes ids são o contrato de que a explicação continua existindo.
        use super::super::naofazemos::LISTA;

        for id in ["uac_desligado", "firewall_desligado"] {
            assert!(
                LISTA.iter().any(|n| n.id == id),
                "`{id}` saiu do catálogo e ninguém explica por quê"
            );
        }
    }

    #[test]
    fn no_optimization_touches_windows_update_or_defender() {
        let forbidden = [
            "wuauserv",
            "BITS",
            "WinDefend",
            "mpssvc",
            "SecurityHealthService",
        ];

        for spec in CATALOG {
            for action in spec.actions {
                if let Action::DisableService { name } = action {
                    assert!(
                        !forbidden.iter().any(|f| f.eq_ignore_ascii_case(name)),
                        "{} desativa serviço proibido: {}",
                        spec.id,
                        name
                    );
                }
            }
        }
    }

    #[test]
    fn only_file_deletion_is_irreversible() {
        // Irreversível novo precisa de decisão consciente: este teste falha.
        let mut irreversible: Vec<&str> = CATALOG
            .iter()
            .filter(|spec| !spec.reversible)
            .map(|spec| spec.id)
            .collect();
        irreversible.sort();

        assert!(irreversible.is_empty(), "{:?}", irreversible);
    }

    #[test]
    fn irreversible_optimizations_say_so_to_the_user() {
        for spec in CATALOG.iter().filter(|spec| !spec.reversible) {
            let text = spec.honest_effect.to_lowercase();
            assert!(
                text.contains("não pode ser desfeita") || text.contains("não volta"),
                "{} não avisa que é irreversível",
                spec.id
            );
        }
    }

    #[test]
    fn quem_nao_promete_ganho_diz_isso_na_cara() {
        // `NoGain` precisa dizer, no texto que o cliente lê, que não muda desempenho.
        for spec in CATALOG
            .iter()
            .filter(|s| s.expected_gain == ExpectedGain::NoGain)
        {
            let texto = spec.honest_effect.to_lowercase();

            assert!(
                texto.contains("não muda desempenho")
                    || texto.contains("não muda absolutamente nada")
                    || texto.contains("não ganha um quadro"),
                "`{}` está marcado como sem ganho e não diz isso ao cliente: {:?}",
                spec.id,
                spec.honest_effect
            );
        }
    }

    #[test]
    fn nada_que_promete_fps_entra_como_sem_ganho() {
        for spec in CATALOG
            .iter()
            .filter(|s| s.expected_gain == ExpectedGain::NoGain)
        {
            let texto = format!("{} {}", spec.description, spec.honest_effect).to_lowercase();

            assert!(
                !texto.contains("mais fps") && !texto.contains("aumenta o fps"),
                "`{}` promete FPS e está classificado como sem ganho",
                spec.id
            );
        }
    }

    /// A trava da 2.1.0: `PodeCustar` entrando no lote para a compilação.
    #[test]
    fn nada_que_pode_custar_fps_entra_no_lote_automatico() {
        for spec in CATALOG {
            if spec.risco_de_fps.pode_custar() {
                assert!(
                    !entra_no_lote(spec),
                    "`{}` pode custar FPS e ainda assim entra no \"Otimizar agora\". \
                     Foi exatamente assim que a 2.1.0 derrubou o FPS de um cliente.",
                    spec.id
                );
            }
        }
    }

    /// Risco sem dizer QUANDO não ajuda a decidir.
    #[test]
    fn todo_risco_de_fps_diz_em_que_caso_ele_custa() {
        for spec in CATALOG {
            if let RiscoDeFps::PodeCustar { quando, .. } = &spec.risco_de_fps {
                assert!(
                    quando.len() >= 80,
                    "`{}`: o risco precisa dizer em que caso ele custa quadro, \
                     não só que pode",
                    spec.id
                );
            }
        }
    }

    /// Tirá-los da lista precisa ser decisão consciente.
    #[test]
    fn os_ajustes_do_incidente_continuam_fora_do_lote() {
        for id in ["gpu_hardware_scheduling", "disable_memory_compression"] {
            let spec = CATALOG
                .iter()
                .find(|s| s.id == id)
                .unwrap_or_else(|| panic!("`{id}` sumiu do catálogo"));

            assert!(
                spec.risco_de_fps.pode_custar(),
                "`{id}` voltou a ser tratado como se não pudesse custar FPS"
            );
            assert!(!entra_no_lote(spec), "`{id}` voltou para o lote automático");
        }
    }

    #[test]
    fn aplicativos_em_segundo_plano_nao_entram_no_lote() {
        let item = find("background_apps_off").expect("o item continua existindo, um a um");

        assert!(
            !entra_no_lote(item),
            "`background_apps_off` voltou a entrar no lote"
        );
        assert!(item.reversible);
        assert!(
            item.honest_effect
                .contains("não entra no \"Otimizar agora\""),
            "o texto não diz ao cliente que o item fica fora do lote"
        );
    }

    #[test]
    fn todo_id_fora_do_lote_existe_no_catalogo() {
        // Um id errado na lista não exclui nada, em silêncio.
        for id in FORA_DO_LOTE {
            assert!(
                find(id).is_some(),
                "`{}` está em FORA_DO_LOTE e não existe no catálogo",
                id
            );
        }
    }

    #[test]
    fn o_lote_continua_sem_irreversivel_e_sem_troca_de_seguranca() {
        for spec in CATALOG
            .iter()
            .filter(|s| !s.reversible || s.security_tradeoff)
        {
            assert!(!entra_no_lote(spec), "`{}` entraria no lote", spec.id);
        }
    }

    #[test]
    fn toda_classe_aponta_para_um_item_do_catalogo() {
        for id in EXPERT.iter().chain(CONDICIONAIS.iter().map(|(i, _)| i)) {
            let spec = find(id).unwrap_or_else(|| panic!("`{id}` classificado e fora do catálogo"));
            assert!(
                !retirado(id),
                "`{id}` foi retirado e ainda está classificado"
            );
            let _ = spec;
        }
        for id in EXPERT {
            assert!(
                !CONDICIONAIS.iter().any(|(i, _)| i == id),
                "`{id}` em duas classes"
            );
        }
    }

    #[test]
    fn expert_e_so_se_pedir_nunca_entram_em_lote_nem_com_tudo_atendido() {
        for spec in CATALOG {
            let nunca = matches!(
                classe(spec.id),
                Classe::Expert | Classe::Condicional(Condicao::SoSePedir)
            );
            if nunca {
                assert!(
                    !entra_no_lote_se(spec, |_| true),
                    "`{}` entraria num lote",
                    spec.id
                );
            }
        }
    }

    #[test]
    fn condicional_so_entra_no_lote_com_a_condicao_medida() {
        let dvr = find("disable_gamedvr").unwrap();
        assert!(!entra_no_lote(dvr), "sem medir, o Game DVR não pode entrar");
        assert!(entra_no_lote_se(dvr, |c| c == Condicao::GameDvrLigado));
        assert!(!entra_no_lote_se(dvr, |c| c != Condicao::GameDvrLigado));
    }

    #[test]
    fn toda_condicao_explica_por_que_o_item_aparece() {
        for (_, c) in CONDICIONAIS {
            assert!(c.quando().len() >= 40, "{:?} sem explicação", c);
        }
    }

    #[test]
    fn find_returns_known_optimization() {
        assert!(find("plano_otimiza").is_some());
        assert!(find("does_not_exist").is_none());
    }
}
