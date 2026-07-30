// Catálogo de otimizações do Windows
//
// Cada entrada declara o que muda, o ganho REAL esperado e se é reversível.
// A honestidade aqui é o produto: um ganho descrito como "pequeno" vale mais que
// um número inventado que o cliente não consegue medir.
//
// Nada neste catálogo desativa Windows Update, antivírus, firewall ou serviços de
// núcleo — as três coisas que "otimizadores" de má qualidade quebram.

use crate::modules::optimizer::{Category, ExpectedGain, OptimizationInfo, OptimizationState};

/// Valor a ser escrito no registro.
#[derive(Debug, Clone)]
pub enum RegValue {
    Dword(u32),
    Text(&'static str),
}

/// Uma ação concreta que a otimização executa no sistema.
#[derive(Debug, Clone)]
pub enum Action {
    Registry {
        hive: &'static str,
        path: &'static str,
        name: &'static str,
        value: RegValue,
    },
    /// Desativa um serviço e o para. O tipo de inicialização anterior é preservado.
    DisableService { name: &'static str },
    /// Ativa o plano de energia Alto Desempenho, guardando o plano anterior.
    HighPerformancePowerPlan,
    /// Desativa o algoritmo de Nagle em cada interface de rede ativa.
    /// Precisa enumerar as interfaces em tempo de execução — os GUIDs mudam de PC para PC.
    DisableNagle,
    /// Desliga a hibernação, liberando do disco um arquivo do tamanho da RAM.
    DisableHibernation,
    /// Altera um ajuste fino do plano de energia ativo (estacionamento de núcleos,
    /// estado mínimo do processador). São as opções que o Windows esconde do painel.
    PowerSetting {
        subgroup: &'static str,
        setting: &'static str,
        value: u32,
    },
    /// Liga ou desliga a compressão de memória do Windows.
    MemoryCompression { enabled: bool },
    /// Remove limites de núcleos e memória gravados na configuração de boot.
    ClearBootLimits,
    /// Apaga arquivos temporários. A única ação irreversível do catálogo.
    CleanTempFiles,
}

/// Condição de hardware para uma otimização fazer sentido.
///
/// Existe para o produto poder dizer "isto não serve para a SUA máquina" — a
/// diferença entre ler o hardware e despejar uma lista de tweaks igual para todos.
#[derive(Debug, Clone, Copy)]
pub enum Requirement {
    /// Só ajuda em SSD. Em disco mecânico faz mal.
    SsdSystemDrive,
    /// Só ajuda a partir desta quantidade de memória.
    MinRamGb(f64),
}

impl Requirement {
    /// Motivo mostrado ao usuário quando a máquina dele não atende à condição.
    pub fn unmet_reason(&self) -> &'static str {
        match self {
            Requirement::SsdSystemDrive => {
                "Não oferecemos: seu disco de sistema não é SSD, e aqui isso deixaria o PC mais lento."
            }
            Requirement::MinRamGb(_) => {
                "Não oferecemos: sua memória RAM é pouca para isso, e aplicar pioraria o desempenho."
            }
        }
    }
}

pub struct OptimizationSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// O que o cliente realmente deve esperar. Aparece na interface sem maquiagem.
    pub honest_effect: &'static str,
    pub category: Category,
    pub expected_gain: ExpectedGain,
    pub requires_admin: bool,
    pub requires_restart: bool,
    /// Quase todas as otimizações são reversíveis por construção, porque cada ação
    /// registra o estado anterior. A exceção é apagar arquivo, que não volta.
    pub reversible: bool,
    /// Condição de hardware. `None` significa que serve para qualquer máquina.
    pub requirement: Option<Requirement>,
    /// Troca segurança por desempenho. Nunca entra no "Otimizar agora": abrir mão
    /// de proteção é decisão consciente do dono do PC, não efeito colateral de um
    /// clique genérico.
    pub security_tradeoff: bool,
    pub actions: &'static [Action],
}

impl OptimizationSpec {
    pub fn to_info(&self, state: OptimizationState, detail: Option<String>) -> OptimizationInfo {
        OptimizationInfo {
            id: self.id.to_string(),
            name: self.name.to_string(),
            description: self.description.to_string(),
            honest_effect: self.honest_effect.to_string(),
            category: self.category,
            expected_gain: self.expected_gain,
            requires_admin: self.requires_admin,
            requires_restart: self.requires_restart,
            reversible: self.reversible,
            security_tradeoff: self.security_tradeoff,
            state,
            detail,
        }
    }
}

/// Subgrupo "Gerenciamento de energia do processador" do Windows.
pub const SUB_PROCESSOR: &str = "54533251-82be-4824-96c1-47b60b740d00";
/// Mínimo de núcleos que o Windows mantém acordados (estacionamento de núcleos).
pub const CPMINCORES: &str = "0cc5b647-c1df-4637-891a-dec35c318583";
/// Estado mínimo do processador, em porcentagem.
pub const PROCTHROTTLEMIN: &str = "893dee8e-2bef-41e0-89c6-b55d0929964c";

pub static CATALOG: &[OptimizationSpec] = &[
    OptimizationSpec {
        id: "power_high_performance",
        name: "Plano de energia Alto Desempenho",
        description: "Impede que o Windows reduza a frequência da CPU durante jogos e cargas leves.",
        honest_effect: "Ganho real e consistente em notebooks e em PCs que estão no plano Equilibrado. Se o PC já está em Alto Desempenho, o ganho é zero.",
        category: Category::System,
        expected_gain: ExpectedGain::Measurable,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        actions: &[Action::HighPerformancePowerPlan],
    },
    OptimizationSpec {
        id: "disable_gamedvr",
        name: "Desativar gravação em segundo plano (Game DVR)",
        description: "Desliga a gravação automática da Xbox Game Bar, que roda durante todo jogo.",
        honest_effect: "Costuma render de 2 a 8 FPS em placas de vídeo mais fracas. Em PCs potentes o ganho é pequeno.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Measurable,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        honest_effect: "Reduz latência e ajuda quando a CPU é o gargalo. Exige GPU e driver compatíveis e reinício. Em algumas máquinas o efeito é nulo.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Situational,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        actions: &[
            Action::Registry {
                hive: "HKCU",
                path: r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
                name: "VisualFXSetting",
                value: RegValue::Dword(2),
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
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        actions: &[Action::DisableNagle],
    },
    OptimizationSpec {
        id: "mouse_precision_off",
        name: "Desativar aceleração do mouse",
        description: "Remove a \"precisão aprimorada do ponteiro\", que muda a distância percorrida conforme a velocidade do movimento.",
        honest_effect: "Não muda FPS. Deixa a mira consistente: o mesmo movimento de mão passa a percorrer sempre a mesma distância. Quem está acostumado com a aceleração vai estranhar nos primeiros dias.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Responsiveness,
        requires_admin: false,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: Some(Requirement::SsdSystemDrive),
        security_tradeoff: false,
        actions: &[Action::DisableService { name: "SysMain" }],
    },
    OptimizationSpec {
        id: "disable_hibernation",
        name: "Desativar hibernação",
        description: "Remove o arquivo de hibernação, que ocupa no disco o mesmo tanto da sua memória RAM.",
        honest_effect: "Libera vários GB de disco imediatamente. Em troca, você perde a hibernação e a Inicialização Rápida — o PC vai ligar alguns segundos mais devagar. Reversível.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        actions: &[Action::DisableHibernation],
    },
    OptimizationSpec {
        id: "disable_core_parking",
        name: "Desligar estacionamento de núcleos",
        description: "Impede o Windows de colocar núcleos da CPU para dormir e ter que acordá-los quando a carga chega.",
        honest_effect: "Ataca engasgo, não FPS médio. Acordar um núcleo estacionado custa milissegundos, e é isso que vira travada no meio da partida. Mede-se em 'Travada no pior caso' — se lá não mudar, aqui não teve efeito. Em notebook a bateria dura menos.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Measurable,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        actions: &[Action::PowerSetting {
            subgroup: SUB_PROCESSOR,
            setting: CPMINCORES,
            value: 100,
        }],
    },
    OptimizationSpec {
        id: "processor_min_state",
        name: "Estado mínimo do processador em 100%",
        description: "Trava a frequência da CPU no máximo em vez de deixá-la cair e subir conforme a carga.",
        honest_effect: "Elimina o atraso de a CPU ter que 'acordar' de uma frequência baixa. Aumenta consumo e temperatura. Em notebook na bateria, não recomendamos.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Measurable,
        requires_admin: true,
        requires_restart: false,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        actions: &[Action::PowerSetting {
            subgroup: SUB_PROCESSOR,
            setting: PROCTHROTTLEMIN,
            value: 100,
        }],
    },
    OptimizationSpec {
        id: "disable_power_throttling",
        name: "Desligar limitação de energia por processo",
        description: "Impede o Windows de reduzir a velocidade de programas que ele julga estarem em segundo plano.",
        honest_effect: "O Windows às vezes classifica errado e limita justamente o que você está usando. Desligar corrige esses casos. Em notebook, gasta mais bateria.",
        category: Category::System,
        expected_gain: ExpectedGain::Situational,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
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
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: Some(Requirement::MinRamGb(12.0)),
        security_tradeoff: false,
        actions: &[Action::MemoryCompression { enabled: false }],
    },
    OptimizationSpec {
        id: "clear_boot_limits",
        name: "Liberar limites de inicialização",
        description: "Remove restrições de número de núcleos e de memória gravadas na configuração de boot do Windows.",
        honest_effect: "Se esses limites existem, seu PC está usando de propósito menos processador ou menos memória do que tem — e o ganho ao remover é enorme. Se não existem, esta opção nem aparece como disponível.",
        category: Category::System,
        expected_gain: ExpectedGain::Measurable,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: false,
        actions: &[Action::ClearBootLimits],
    },
    OptimizationSpec {
        id: "disable_vbs",
        name: "Desligar virtualização de segurança (VBS)",
        description: "Desativa a camada de virtualização que o Windows usa para isolar credenciais e validar código do núcleo.",
        honest_effect: "ATENÇÃO, ISTO REDUZ A SEGURANÇA DO SEU PC. O VBS protege suas senhas do Windows contra roubo por programa malicioso e barra exploração do núcleo do sistema. Desligar rende FPS de verdade, mais em processadores de 8ª a 10ª geração, e é reversível — mas é uma troca de proteção por desempenho, não almoço grátis. Se você usa Hyper-V, WSL ou Sandbox, eles param de funcionar.",
        category: Category::Gaming,
        expected_gain: ExpectedGain::Measurable,
        requires_admin: true,
        requires_restart: true,
        reversible: true,
        requirement: None,
        security_tradeoff: true,
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
        ],
    },
    OptimizationSpec {
        id: "clean_temp_files",
        name: "Limpar arquivos temporários",
        description: "Apaga o conteúdo das pastas de temporários do Windows e do seu usuário.",
        honest_effect: "Libera espaço em disco. Não aumenta FPS. É a ÚNICA operação do programa que não pode ser desfeita — arquivo apagado não volta. Arquivos em uso são pulados.",
        category: Category::System,
        expected_gain: ExpectedGain::Responsiveness,
        requires_admin: false,
        requires_restart: false,
        reversible: false,
        requirement: None,
        security_tradeoff: false,
        actions: &[Action::CleanTempFiles],
    },
];

/// Busca uma otimização pelo identificador.
pub fn find(id: &str) -> Option<&'static OptimizationSpec> {
    CATALOG.iter().find(|spec| spec.id == id)
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

    #[test]
    fn no_optimization_touches_windows_update_or_defender() {
        // Estes são exatamente os serviços que otimizadores ruins quebram.
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
        // Qualquer nova otimização irreversível precisa ser uma decisão consciente:
        // este teste falha e obriga a revisão.
        let irreversible: Vec<&str> = CATALOG
            .iter()
            .filter(|spec| !spec.reversible)
            .map(|spec| spec.id)
            .collect();

        assert_eq!(irreversible, vec!["clean_temp_files"]);
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
    fn find_returns_known_optimization() {
        assert!(find("power_high_performance").is_some());
        assert!(find("does_not_exist").is_none());
    }
}
