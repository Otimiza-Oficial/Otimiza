// Catálogo de otimizações do Windows
//
// Cada entrada declara o que muda, o ganho REAL esperado e se é reversível.
// A honestidade aqui é o produto: um ganho descrito como "pequeno" vale mais que
// um número inventado que o cliente não consegue medir.
//
// Nada neste catálogo desativa Windows Update, antivírus, firewall ou serviços de
// núcleo — as três coisas que "otimizadores" de má qualidade quebram.

use serde::{Deserialize, Serialize};
use crate::modules::optimizer::{
    Category, ExpectedGain, OQuePodeCustar, OptimizationInfo, OptimizationState, RiscoDeFps,
};

/// Valor a ser escrito no registro.
#[derive(Debug, Clone)]
pub enum RegValue {
    Dword(u32),
    Text(&'static str),
    /// Valor binário bruto.
    ///
    /// Existe por causa da `UserPreferencesMask`, que é onde o Windows guarda,
    /// bit a bit, quais efeitos visuais estão ligados. Sem escrever binário só
    /// dá para mudar o rótulo "melhor desempenho" e deixar os efeitos rodando.
    Binary(&'static [u8]),
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
    /// Cria, configura, ativa e CONFERE o plano de energia OTIMIZA.
    ///
    /// SUBSTITUIU TRÊS AÇÕES, e a troca não é de arrumação: `HighPerformancePowerPlan`
    /// ativava um GUID fixo que não existe em toda máquina, e as duas
    /// `PowerSetting` escreviam DENTRO DO PLANO DO CLIENTE — o que fazia o
    /// desfazer depender de ter lido e regravado cada valor certo. O plano
    /// próprio não toca no plano de ninguém, e desfazer é reativar o anterior.
    /// Ver o cabeçalho de `planoenergia.rs`.
    PlanoOtimiza,
    /// Desativa o algoritmo de Nagle em cada interface de rede ativa.
    /// Precisa enumerar as interfaces em tempo de execução — os GUIDs mudam de PC para PC.
    DisableNagle,
    /// Desliga a hibernação, liberando do disco um arquivo do tamanho da RAM.
    DisableHibernation,
    // `PowerSetting` MORAVA AQUI, e saiu com a migração para o plano próprio.
    //
    // Ela escrevia o ajuste dentro do plano ATIVO do cliente, e por isso o
    // desfazer precisava ter lido o valor anterior de cada um e conseguir
    // gravá-lo de volta. `ChangeRecord::PowerSetting` continua existindo de
    // propósito: há `changes.json` em máquina de cliente gravado por versões
    // anteriores, e essas mudanças precisam continuar podendo ser desfeitas.
    /// Liga ou desliga a compressão de memória do Windows.
    MemoryCompression { enabled: bool },
    /// Remove limites de núcleos e memória gravados na configuração de boot.
    ClearBootLimits,
    /// Liga interrupções por mensagem (MSI) nas placas de vídeo encontradas.
    GpuMsiMode,
    /// Impede o Windows de desligar a placa de rede para economizar energia.
    NicPowerSaving,
    /// Liga ou desliga o Armazenamento Reservado do Windows.
    ReservedStorage { enabled: bool },
    /// Remove o relógio de plataforma forçado na configuração de boot.
    RemoveForcedPlatformClock,
    /// Impede o hipervisor de subir no boot.
    ///
    /// Anda junto com o desligamento do VBS, e não por capricho: o VBS roda em
    /// cima do hipervisor. Zerar só as chaves de registro tira a proteção e
    /// deixa o hipervisor sendo carregado — o cliente paga o preço em segurança
    /// e não recebe o desempenho que a otimização prometeu.
    DisableHypervisor,
    /// Desliga Filtragem de Teclas, Teclas de Aderência e Teclas Alternadas.
    ///
    /// Ação própria, e não três escritas de registro, porque o estado mora num
    /// campo de bits: só o bit 0 pode mudar, senão as preferências do usuário
    /// sobre atalho, som e aviso vão junto.
    AccessibilityKeysOff,
}

/// Condição de hardware em que uma otimização pesa MUITO mais que a média.
///
/// Não é promessa de milagre: é reconhecer que desligar efeito visual muda pouco
/// num PC forte e muda muito num PC de 4 GB. O produto usa isso para dizer ao
/// cliente o que vale a pena na máquina dele, em vez de entregar a mesma lista
/// para todo mundo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boost {
    /// Pouca memória RAM.
    LowRam,
    /// Disco mecânico, onde qualquer leitura extra custa caro.
    MechanicalDisk,
    /// Poucos núcleos, onde cada processo de fundo disputa espaço de verdade.
    FewCores,
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
    /// Exige que o driver de vídeo declare pelo menos esta versão de WDDM,
    /// no formato do registro: 2700 é WDDM 2.7.
    ///
    /// POR QUE ISTO EXISTE. O agendamento de GPU por hardware era escrito em
    /// qualquer máquina. A escrita dá certo sempre — é um DWORD num caminho que
    /// existe em todo Windows —, e a releitura devolve o valor gravado, então o
    /// produto marcava a otimização como APLICADA. Só que abaixo de WDDM 2.7 o
    /// Windows simplesmente ignora a chave: o cliente reiniciava o PC por nada e
    /// a lista dizia que estava tudo certo.
    ///
    /// É o defeito da casa — confiar na escrita em vez de conferir o efeito —
    /// num lugar onde nem reler o valor resolve, porque o valor entra e não vale.
    /// A única saída honesta é perguntar antes se esta máquina consegue.
    MinWddm(u32),
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
            // A FRASE PRECISA SER VERDADE NOS DOIS CASOS que chegam aqui: o
            // driver é antigo, ou a versão não pôde ser lida. Dizer "o seu
            // driver é anterior ao WDDM 2.7" seria afirmar o que não foi
            // verificado quando a leitura é que falhou.
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
    /// O que o cliente realmente deve esperar. Aparece na interface sem maquiagem.
    pub honest_effect: &'static str,
    pub category: Category,
    pub expected_gain: ExpectedGain,
    /// Se este ajuste pode DERRUBAR o FPS em alguma máquina, e em qual caso.
    /// Quando pode, `entra_no_lote` o exclui do "Otimizar agora".
    pub risco_de_fps: RiscoDeFps,
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
    /// Em que tipo de máquina esta otimização pesa muito mais que a média.
    /// Vazio significa que o ganho não depende do porte do hardware.
    pub highlight_when: &'static [Boost],
    pub actions: &'static [Action],
}

/// Itens que nunca entram num lote — nem no "Otimizar agora", nem num perfil —,
/// mesmo sendo reversíveis e sem troca de segurança. Continuam disponíveis um a
/// um, com o aviso na tela.
///
/// `background_apps_off` saiu do lote na 2.0. Ele corta a execução em segundo
/// plano de TODO aplicativo instalado pela Loja, inclusive os que o cliente usa
/// o dia inteiro. É o tipo de efeito que só aparece dias depois, sem ninguém
/// ligar ao clique — e decidir isso é do dono do PC, item por item.
pub const FORA_DO_LOTE: &[&str] = &["background_apps_off"];

// ─── Classes da auditoria 2.9 ────────────────────────────────────────────
//
// "Menos ajustes, e melhores": o ajuste certo PARA ESTE COMPUTADOR, não a
// lista mais longa. Cada item do catálogo é de uma de três classes
// (`docs/AUDITORIA-2.9.md`, seção 2):
//
// - **Essencial**: vale em qualquer máquina, entra no "Otimizar agora".
// - **Condicional**: só faz diferença quando a máquina tem um problema que dá
//   para MEDIR (gravação do Game Bar ligada, pouco espaço, pouca memória, PC
//   fraco). Só aparece — e só entra no lote — quando a condição foi medida
//   aqui. Os de "só se pedir" aparecem sempre, mas nunca entram em lote.
// - **Expert**: pode render numa máquina e custar em outra, ou troca algo que
//   a pessoa precisa entender. Só aparece no modo Expert, e nunca em lote.
//
// As listas são curtas e têm trava: id errado aqui não classifica nada, em
// silêncio, e o teste `toda_classe_aponta_para_um_item_do_catalogo` pega.

/// O que precisa ser verdade nesta máquina para um item condicional valer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condicao {
    /// A gravação em segundo plano do Game Bar está ligada.
    GameDvrLigado,
    /// PC fraco: até 8 GB de RAM ou até 4 núcleos lógicos. Os ajustes de área
    /// de trabalho só são sentidos aqui.
    PcFraco,
    /// Menos de 20 GB livres no disco do Windows.
    PoucoEspaco,
    /// Até 16 GB de RAM: processo de fundo disputa memória com o jogo.
    MemoriaApertada,
    /// Nada que dê para medir decide por você. Aparece, mas só entra se a
    /// pessoa escolher o item — nunca num lote.
    SoSePedir,
}

impl Condicao {
    /// Por que o item aparece, na tela.
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

/// Só no modo Expert, nunca em lote, sempre com antes e depois.
pub const EXPERT: &[&str] = &[
    // Depende de placa e driver; já causou engasgo em driver antigo.
    "gpu_hardware_scheduling",
    // A maioria dos drivers atuais já usa MSI; só age se estiver desligado.
    "gpu_msi_mode",
    // Ganho real em parte dos jogos presos na CPU, com custo de segurança real.
    "disable_vbs",
    // A busca do Windows fica lenta; ganho só em disco mecânico.
    "disable_search_indexing",
];

/// Itens que só valem quando a máquina tem o problema que eles resolvem.
pub const CONDICIONAIS: &[(&str, Condicao)] = &[
    ("disable_gamedvr", Condicao::GameDvrLigado),
    ("visual_effects_performance", Condicao::PcFraco),
    ("disable_transparency", Condicao::PcFraco),
    ("disable_hibernation", Condicao::PoucoEspaco),
    ("disable_reserved_storage", Condicao::PoucoEspaco),
    ("disable_widgets", Condicao::MemoriaApertada),
    ("edge_background_off", Condicao::MemoriaApertada),
    // Boot, não jogo.
    ("disable_startup_delay", Condicao::SoSePedir),
    // Real quando a placa dorme e perde pacote — o Otimiza ainda não mede
    // perda de pacote, então não decide sozinho.
    ("nic_power_saving_off", Condicao::SoSePedir),
    ("delivery_optimization_off", Condicao::SoSePedir),
    // Os apps da Loja ficam desatualizados.
    ("store_auto_download_off", Condicao::SoSePedir),
    // Perde o registro da falha quando um jogo cai.
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

/// Itens RETIRADOS na 2.9: não mudam FPS, 1% low, engasgo, atraso,
/// carregamento nem responsividade, e não protegem a máquina.
///
/// A regra da 2.9 é "as mudanças certas para aquele computador, não a lista
/// mais longa". Um item que não muda nada que o cliente sinta só ocupa lugar
/// na tela e dá a impressão de que o produto "fez 40 coisas". Dois deles (UAC
/// e Firewall) ainda tiravam proteção em troca de nada.
///
/// Eles ficam no catálogo SÓ para o desfazer: quem aplicou numa versão antiga
/// continua vendo o item na lista, com o botão de desfazer, até desfazer. Nada
/// aqui pode ser aplicado de novo — nem um a um, nem em lote, nem por perfil.
/// O motivo de cada um mora em `naofazemos.rs`.
pub const RETIRADOS: &[&str] = &[
    // Segunda rodada (auditoria da 2.9, docs/AUDITORIA-2.9.md):
    // - SystemResponsiveness/NetworkThrottlingIndex, Win32PrioritySeparation e
    //   as prioridades MMCSS de "Games": sem ganho reproduzível em jogo;
    // - SysMain e compressão de memória: o Windows gerencia; com pouca RAM,
    //   desligar a compressão piora;
    // - PowerThrottlingOff: desliga o EcoQoS no sistema inteiro, que é
    //   justamente o que o modo jogo (governador) usa nos programas de fundo;
    // - notificações: o Windows 11 já silencia durante o jogo;
    // - limpezas de temporários e do cache do Windows Update: duplicadas da
    //   tela Limpeza do sistema.
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

/// Se o item foi retirado do produto (ver `RETIRADOS`).
pub fn retirado(id: &str) -> bool {
    RETIRADOS.contains(&id)
}

/// Se um item pode ser aplicado por um lote, sem a pessoa escolher item a item.
///
/// As quatro exclusões moram juntas aqui para o motor e os testes lerem a mesma
/// regra: o que não volta, o que troca segurança por desempenho, o que está em
/// `FORA_DO_LOTE`, e — desde o incidente da 2.1.0 — **o que pode custar FPS**.
///
/// A quarta é a mais importante das quatro, e a mais cara de aprender. Um
/// cliente aplicou tudo que o produto oferece e o FPS dele caiu pela metade.
/// O produto é vendido para quem olha o contador de quadros: um lote que pode
/// derrubar esse número, sem a pessoa ter escolhido a troca, não é otimização
/// — é uma aposta feita no lugar dela.
///
/// O item não some: continua no catálogo, item a item, com o caso escrito em
/// `RiscoDeFps::PodeCustar` aparecendo na tela. A diferença é quem decide.
pub fn entra_no_lote(spec: &OptimizationSpec) -> bool {
    entra_no_lote_se(spec, |_| false)
}

/// `entra_no_lote`, mas com a resposta de cada condição medida nesta
/// máquina. Condicional só entra com a condição atendida; Expert e "só se
/// pedir" nunca entram. **Pura**: quem mede é o chamador.
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

/// Subgrupo "Gerenciamento de energia do processador" do Windows.
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
        // WDDM 2.7 é o piso do agendamento por hardware. Ver `Requirement::MinWddm`.
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
            // Esta é a linha que faz a otimização valer. `VisualFXSetting` é só
            // o rótulo que a tela de Sistema mostra; os efeitos de verdade
            // moram nos bits desta máscara, e é ela que o Painel de Controle
            // grava ao escolher "melhor desempenho". Sem ela, o produto trocava
            // o rótulo e o Windows seguia animando tudo — medido na máquina do
            // dono: `VisualFXSetting = 2` com a máscara `9E ...`, ou seja,
            // rótulo dizendo uma coisa e sistema fazendo outra.
            //
            // `90 12 03 80 10 00 00 00` é exatamente o que o Windows escreve
            // nessa escolha: animação de janela, sombra, deslizar de menu e
            // arrastar conteúdo desligados; suavização de fonte mantida.
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
            // Engasgo e não FPS médio: sem a compressão o Windows vai ao
            // disco, e o sintoma disso é travada — a média mal se move.
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
        // FICA NO LOTE, e isso foi reconsiderado de propósito.
        //
        // Na primeira passada depois do incidente eu marquei este ajuste como
        // "pode custar FPS" junto com os outros dois. Revendo: não existe
        // evidência de que o modo MSI derrube quadro. O que existe é relato de
        // instabilidade com driver antigo — outro problema, com outro nome.
        //
        // Classificar por medo, sem evidência, é exatamente o erro que criou o
        // incidente: eu afirmei o significado de um valor sem conferir. Marcar
        // um ajuste de risco sem prova é o mesmo vício virado do avesso, e
        // esvazia o aviso dos que têm risco de verdade.
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
            // Sem esta linha o hipervisor continua subindo no boot, e o custo de
            // desempenho que a otimização promete devolver fica onde estava.
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
    // ======================================================================
    // O CATÁLOGO DO MERCADO
    //
    // Itens que todo otimizador concorrente oferece. Entram porque o cliente
    // compara lista com lista, e sair perdendo numa comparação de catálogo
    // custa venda mesmo quando o catálogo do outro é feito de nada.
    //
    // A condição é a que o produto já tinha: cada um entra com a classificação
    // honesta. A maioria é `NoGain` — higiene e privacidade, não desempenho —
    // e a tela conta quantos são, em voz alta, antes de o cliente clicar.
    // ======================================================================

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

    // ======================================================================
    // OS DOIS QUE ENTRARAM NO LUGAR DOS QUE SAÍRAM
    //
    // Saíram daqui "desligar o UAC" e "desligar o firewall": ganho declarado
    // nulo, custo real em segurança. Estes dois entraram sob a régua contrária
    // — cada um mexe numa chave documentada pelo dono dela, faz uma coisa que
    // dá para perceber, volta atrás, e traz o preço escrito.
    //
    // Nenhum dos dois promete quadro por segundo, e os textos dizem isso na
    // primeira linha. Trocar dois itens que mentiam alto por dois que dizem
    // "isto não é FPS" é a troca que este produto precisa fazer.
    // ======================================================================

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

    // ======================================================================
    // POR QUE NÃO EXISTE "DESLIGAR O WINDOWS DEFENDER" NESTA LISTA
    //
    // Não é escrúpulo: é que não funciona, e um botão que não funciona é pior
    // que a ausência dele.
    //
    // Desde o Windows 10 versão 1903, a Proteção contra Adulteração vem ligada
    // de fábrica. Com ela ligada, o Windows IGNORA a chave `DisableAntiSpyware`,
    // recusa parar o serviço `WinDefend` e reverte sozinho o que for escrito.
    // Só o próprio usuário desliga isso, à mão, na janela de Segurança do
    // Windows — nenhum programa consegue por ele.
    //
    // Ou seja: todo concorrente que oferece "desativar Defender" ou escreve
    // chave que o Windows descarta, ou avisa em letra miúda que o cliente
    // precisa desligar a Proteção antes. No primeiro caso o botão mente; no
    // segundo, quem faz o trabalho é o cliente.
    //
    // Este produto não tem um botão que finge. Se um dia a Proteção contra
    // Adulteração puder ser lida de forma confiável, o caminho honesto é um
    // aviso explicando o que fazer — não um interruptor.
    // ======================================================================

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
];

/// Busca uma otimização pelo identificador.
pub fn find(id: &str) -> Option<&'static OptimizationSpec> {
    CATALOG.iter().find(|spec| spec.id == id)
}

#[cfg(test)]
mod tests_1_6 {
    use super::*;

    /// `VisualFXSetting = 2` é só o RÓTULO de "melhor desempenho". Quem governa
    /// as animações é a `UserPreferencesMask`, e o Painel de Controle escreve as
    /// duas coisas. Na máquina do dono o resultado era `VisualFXSetting = 2` com
    /// a máscara `9E ...`: rótulo dizendo uma coisa, sistema fazendo outra.
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

    /// Arrastar janela redesenhando todo o conteúdo é um dos efeitos mais caros
    /// em PC fraco, e é justamente o que "melhor desempenho" desliga.
    #[test]
    fn efeitos_visuais_desligam_o_arrasto_de_janela_cheia() {
        let spec = find("visual_effects_performance").expect("otimização deveria existir");

        assert!(spec.actions.iter().any(|acao| matches!(
            acao,
            Action::Registry { name, value: RegValue::Text("0"), .. }
                if *name == "DragFullWindows"
        )));
    }

    /// O VBS roda EM CIMA do hipervisor. Zerar as duas chaves do DeviceGuard
    /// desliga a camada de segurança, mas se o hipervisor continuar sendo
    /// carregado no boot o custo de desempenho continua exatamente onde estava
    /// — e é justamente esse custo que a otimização promete devolver.
    ///
    /// Sem `hypervisorlaunchtype off`, o cliente abre mão da proteção das senhas
    /// do Windows, que é o que o aviso vermelho desta otimização anuncia, e
    /// recebe menos do que foi prometido. Aqui a conta é pior que em qualquer
    /// outro item: esta é a única que cobra em segurança.
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

    /// A TRAVA QUE SUBSTITUIU O TESTE DO UAC.
    ///
    /// Havia aqui um teste conferindo que o aviso de "desligar o UAC" dizia
    /// que aquilo quebra aplicativo da Loja da Microsoft. O ajuste saiu do
    /// catálogo — ele não rendia quadro nenhum e cobrava segurança —, e junto
    /// com ele saiu "desligar o firewall", pelo mesmo motivo.
    ///
    /// Apagar o teste e não pôr nada no lugar deixaria a porta aberta para os
    /// dois voltarem por descuido, numa revisão futura em que alguém repita a
    /// lista de um concorrente. Então a regra virou geral: ajuste que troca
    /// segurança PRECISA entregar alguma coisa. `NoGain` com `security_tradeoff`
    /// é, por definição, custo sem contrapartida.
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
        // Tirar do catálogo sem explicar troca um botão ruim por um buraco: o
        // cliente que vem de outro programa procura, não acha, e conclui que
        // falta função. Estes dois ids são o contrato de que a explicação
        // continua existindo.
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
        // Qualquer nova otimização irreversível precisa ser decisão consciente:
        // este teste falha e obriga a revisão. As duas que existem apagam
        // arquivo, e arquivo apagado não volta — nenhuma outra pode entrar aqui
        // sem alguém decidir que ela merece.
        let mut irreversible: Vec<&str> = CATALOG
            .iter()
            .filter(|spec| !spec.reversible)
            .map(|spec| spec.id)
            .collect();
        irreversible.sort();

        // 2.9: as duas limpezas saíram do catálogo (moram na Limpeza do sistema,
        // que mostra o que se perde antes). O catálogo inteiro tem desfazer.
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
        // A tentação de um catálogo pago é crescer. Cada item novo faz a lista
        // parecer maior que a do concorrente, e o jeito silencioso de crescer é
        // acrescentar ajuste de higiene com texto que soa como desempenho.
        //
        // Um item marcado `NoGain` precisa dizer, no texto que o cliente lê,
        // que não muda desempenho. Sem isto o nível vira uma etiqueta interna
        // que ninguém fora do código enxerga.
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
        // O erro contrário: classificar como `NoGain` alguma coisa que a
        // própria descrição diz que acelera. Aí o produto está escondendo um
        // ganho real, e o cliente deixa de aplicar o que funcionaria.
        for spec in CATALOG.iter().filter(|s| s.expected_gain == ExpectedGain::NoGain) {
            let texto = format!("{} {}", spec.description, spec.honest_effect).to_lowercase();

            assert!(
                !texto.contains("mais fps") && !texto.contains("aumenta o fps"),
                "`{}` promete FPS e está classificado como sem ganho",
                spec.id
            );
        }
    }

    // ─── O lote automático e o que não pode entrar nele ──────────────────

    /// A trava central do incidente da 2.1.0.
    ///
    /// O cliente clicou no botão grande, aplicou tudo, e o FPS caiu pela
    /// metade. Depois disso a regra virou código: o lote automático só aplica
    /// o que não pode custar quadro. Se alguém marcar um item como
    /// `PodeCustar` e ele continuar entrando no lote, a compilação para aqui.
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

    /// Marcar o risco sem dizer QUANDO ele acontece não serve para nada: o
    /// cliente fica com um aviso que não o ajuda a decidir, e o suporte fica
    /// com um "pode variar" para explicar. O texto vai para a tela como está.
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

    /// Os dois que saíram do lote depois do incidente. O teste existe para que
    /// tirá-los da lista seja uma decisão consciente, com este nome falhando,
    /// e não um efeito colateral de mexer noutra coisa.
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
        // Na 2.0 este item saiu do "Otimizar agora" e dos perfis: ele corta
        // aplicativo da Loja que o cliente usa o dia inteiro, e o efeito só
        // aparece dias depois, sem ligação com o clique.
        let item = find("background_apps_off").expect("o item continua existindo, um a um");

        assert!(!entra_no_lote(item), "`background_apps_off` voltou a entrar no lote");
        // Sair do lote não é sair do produto: ele continua reversível e
        // aplicável sozinho.
        assert!(item.reversible);
        assert!(
            item.honest_effect.contains("não entra no \"Otimizar agora\""),
            "o texto não diz ao cliente que o item fica fora do lote"
        );
    }

    #[test]
    fn todo_id_fora_do_lote_existe_no_catalogo() {
        // Um id escrito errado na lista não exclui nada, e em silêncio.
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
