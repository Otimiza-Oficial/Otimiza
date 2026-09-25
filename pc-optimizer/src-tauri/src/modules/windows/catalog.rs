// Catálogo de otimizações: o que muda, o ganho REAL esperado e se é reversível. Nada desativa Windows Update,
// antivírus, firewall ou serviços de núcleo.

use serde::{Deserialize, Serialize};
use crate::modules::optimizer::{
    Category, ExpectedGain, OQuePodeCustar, OptimizationInfo, OptimizationState, RiscoDeFps,
};

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
    DisableService { name: &'static str },
    /// Plano próprio: `HighPerformancePowerPlan` ativava um GUID que não existe em toda máquina, e `PowerSetting`
    /// escrevia DENTRO do plano do cliente. Desfazer é reativar o anterior (ver `planoenergia.rs`).
    PlanoOtimiza,
    /// Os GUIDs das interfaces mudam de PC para PC.
    DisableNagle,
    DisableHibernation,
    // `ChangeRecord::PowerSetting` continua: há `changes.json` de versões anteriores a desfazer.
    MemoryCompression { enabled: bool },
    ClearBootLimits,
    GpuMsiMode,
    NicPowerSaving,
    ReservedStorage { enabled: bool },
    RemoveForcedPlatformClock,
    /// Anda com o desligamento do VBS, que roda em cima do hipervisor: sem isto o cliente paga em segurança e não
    /// recebe o desempenho.
    DisableHypervisor,
    /// O estado é um campo de bits: só o bit 0 muda, senão atalho, som e aviso iriam junto.
    AccessibilityKeysOff,
    /// O valor é um texto com várias escolhas gráficas: só este pedaço muda (`janelas.rs`).
    JanelasOtimizadas,
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
/// `background_apps_off`: corta todo app da Loja, efeito que aparece dias depois. `windowed_game_optimizations`
/// (3.0): muda a entrega de quadros e não foi medido aqui.
pub const FORA_DO_LOTE: &[&str] = &["background_apps_off", "windowed_game_optimizations"];

// Classes da auditoria 2.9 (`docs/auditorias/AUDITORIA-2.9.md`): Essencial (qualquer máquina, entra no lote);
// Condicional (só com a condição MEDIDA aqui; os "só se pedir" aparecem e nunca entram em lote); Expert (só no
// modo Expert, nunca em lote). `toda_classe_aponta_para_um_item_do_catalogo` pega id errado.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condicao {
    GameDvrLigado,
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
    "disable_search_indexing",
];

pub const CONDICIONAIS: &[(&str, Condicao)] = &[
    ("disable_gamedvr", Condicao::GameDvrLigado),
    ("visual_effects_performance", Condicao::PcFraco),
    ("disable_transparency", Condicao::PcFraco),
    ("disable_hibernation", Condicao::PoucoEspaco),
    ("disable_reserved_storage", Condicao::PoucoEspaco),
    ("disable_widgets", Condicao::MemoriaApertada),
    ("edge_background_off", Condicao::MemoriaApertada),
    ("disable_startup_delay", Condicao::SoSePedir),
    // Real quando a placa dorme e perde pacote, e a perda não é medida aqui: não decide sozinho.
    ("nic_power_saving_off", Condicao::SoSePedir),
    ("delivery_optimization_off", Condicao::SoSePedir),
    ("store_auto_download_off", Condicao::SoSePedir),
    ("error_reporting_off", Condicao::SoSePedir),
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
pub const RETIRADOS: &[&str] = &[
    // Segunda rodada (AUDITORIA-2.9): SystemResponsiveness, Win32PrioritySeparation e MMCSS de "Games" sem ganho
    // reproduzível; SysMain e compressão o Windows gerencia; PowerThrottlingOff desliga o EcoQoS que o governador usa;
    // notificações o Windows 11 já silencia; limpezas duplicavam a Limpeza do sistema.
    "system_responsiveness_gaming",
    "foreground_priority",
    "disable_sysmain",
    "disable_power_throttling",
    "disable_memory_compression",
    "mmcss_games",
    "notifications_off",
    "network_low_latency",
    "disable_xbox_services",
    "maps_auto_update_off",
    "settings_sync_off",
    "remote_assistance_off",
    "disable_telemetry",
    "telemetry_policy",
    "start_menu_web_search_off",
    "disable_copilot",
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
        honest_effect: "Ganho pequeno e difícil de medir isoladamente. Ajuda mais em PCs com poucos núcleos.",
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
                value: RegValue::Dword(10),
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
        honest_effect: "Só vale com RAM sobrando: troca uso de CPU por uso de memória. Com 16 GB ou mais costuma render; com 8 GB ou menos PIORA, e por isso nem oferecemos nessas máquinas.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        risco_de_fps: RiscoDeFps::custa(
            // Engasgo e não FPS médio: sem a compressão o Windows vai ao disco.
            OQuePodeCustar::Engasgo,
            "O piso de 12 GB não basta. Quem joga FiveM com navegador e Discord abertos \
             enche 16 GB, e sem a compressão o Windows passa a ir ao disco — que é \
             engasgo e queda de quadro, não ganho. Só vale se a memória sobrar DURANTE \
             o jogo, e isso se vê medindo.",
        ),
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: Some(Requirement::MinRamGb(12.0)),
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
        honest_effect: "Não muda FPS e não libera memória. O que resolve é a notificação que rouba o foco durante um jogo em tela cheia sem bordas — que causa engasgo, mas não é perda de desempenho contínua.",
        category: Category::System,
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
            path: r"Software\Microsoft\Windows\CurrentVersion\PushNotifications",
            name: "ToastEnabled",
            value: RegValue::Dword(0),
        }],
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
        honest_effect: "Não dá FPS — o coletor só existe depois que alguma coisa já quebrou. O que ele muda é o que acontece NA HORA em que o jogo fecha sozinho: hoje o Windows sobe o WerFault, que segura a janela travada enquanto copia a memória do processo para o disco e tenta enviar o relatório. Com jogo grande isso é gigabyte de escrita e dezenas de segundos de PC parado, e é por isso que a queda parece muito pior do que foi. Desligado, o jogo simplesmente fecha e você volta para a área de trabalho. O PREÇO: você perde o registro daquela falha. O Histórico de Confiabilidade do Windows para de anotar, e se um dia precisar descobrir POR QUE aquele jogo cai, a informação não vai estar lá — e aí o caminho é ligar de volta e reproduzir a queda.",
        category: Category::System,
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
            path: r"SOFTWARE\Microsoft\Windows\Windows Error Reporting",
            name: "Disabled",
            value: RegValue::Dword(1),
        }],
    },

    OptimizationSpec {
        id: "edge_background_off",
        name: "Impedir o Edge de ficar aberto por trás e de carregar no boot",
        description: "Desliga as duas configurações que mantêm o Microsoft Edge em execução com a janela fechada e o carregam junto com o Windows.",
        honest_effect: "O Edge tem dois comportamentos ligados de fábrica que quase ninguém sabe que existem: ele se carrega sozinho durante o boot para abrir mais rápido depois, e continua com processos vivos depois que você fecha a última janela. Juntos costumam segurar algumas centenas de megabytes e um punhado de processos que você não pediu. Desligando os dois, esses processos somem quando você fecha o navegador. NÃO ESPERE FPS: memória liberada só vira quadro em máquina que estava realmente sem memória, e em PC com folga a diferença no jogo é nenhuma. O que muda de verdade é o boot e o que fica rodando enquanto você joga. O PREÇO, e ele aparece na cara: isto é escrito como política de máquina, que é a única forma que o Edge lê do registro — então o navegador vai passar a mostrar \"gerenciado pela sua organização\" nas configurações dele. Não é vírus e não é bloqueio; é o Edge dizendo que a configuração veio de fora. Desfazendo aqui, o aviso some.",
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
                name: "StartupBoost",
                value: RegValue::Dword(0),
            },
            Action::Registry {
                hive: "HKLM",
                path: r"SOFTWARE\Policies\Microsoft\Edge",
                name: "BackgroundModeEnabled",
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
        assert_eq!(ids.len(), CATALOG.len(), "ids duplicados tornam o rollback ambíguo");
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
        let forbidden = ["wuauserv", "BITS", "WinDefend", "mpssvc", "SecurityHealthService"];

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
        for spec in CATALOG.iter().filter(|s| s.expected_gain == ExpectedGain::NoGain) {
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
        for spec in CATALOG.iter().filter(|s| s.expected_gain == ExpectedGain::NoGain) {
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

        assert!(!entra_no_lote(item), "`background_apps_off` voltou a entrar no lote");
        assert!(item.reversible);
        assert!(
            item.honest_effect.contains("não entra no \"Otimizar agora\""),
            "o texto não diz ao cliente que o item fica fora do lote"
        );
    }

    #[test]
    fn todo_id_fora_do_lote_existe_no_catalogo() {
        // Um id errado na lista não exclui nada, em silêncio.
        for id in FORA_DO_LOTE {
            assert!(find(id).is_some(), "`{}` está em FORA_DO_LOTE e não existe no catálogo", id);
        }
    }

    #[test]
    fn o_lote_continua_sem_irreversivel_e_sem_troca_de_seguranca() {
        for spec in CATALOG.iter().filter(|s| !s.reversible || s.security_tradeoff) {
            assert!(!entra_no_lote(spec), "`{}` entraria no lote", spec.id);
        }
    }

    #[test]
    fn toda_classe_aponta_para_um_item_do_catalogo() {
        for id in EXPERT.iter().chain(CONDICIONAIS.iter().map(|(i, _)| i)) {
            let spec = find(id).unwrap_or_else(|| panic!("`{id}` classificado e fora do catálogo"));
            assert!(!retirado(id), "`{id}` foi retirado e ainda está classificado");
            let _ = spec;
        }
        for id in EXPERT {
            assert!(!CONDICIONAIS.iter().any(|(i, _)| i == id), "`{id}` em duas classes");
        }
    }

    #[test]
    fn expert_e_so_se_pedir_nunca_entram_em_lote_nem_com_tudo_atendido() {
        for spec in CATALOG {
            let nunca = matches!(classe(spec.id), Classe::Expert | Classe::Condicional(Condicao::SoSePedir));
            if nunca {
                assert!(!entra_no_lote_se(spec, |_| true), "`{}` entraria num lote", spec.id);
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
