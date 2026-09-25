// Otimizador do Windows: aplica e desfaz o catálogo. Nenhuma mudança sem gravar antes o estado anterior; ação
// que falha no meio desfaz as já aplicadas antes de reportar.

pub mod acessibilidade;
pub mod achados;
pub mod afinidade;
pub mod anticheat;
pub mod bloatware;
pub mod boot;
pub mod bottleneck;
pub mod browsers;
pub mod cabecalho;
pub mod catalog;
pub mod citizenfx;
pub mod configjogo;
pub mod conflicts;
pub mod deteccao;
pub mod devices;
pub mod diskspace;
pub mod cpuset;
pub mod registro;
pub mod nvml;
pub mod display;
pub mod essenciais;
pub mod exhaustion;
pub mod firmware;
pub mod fivem;
pub mod frames;
pub mod motorenergia;
pub mod motorenergia_maquina;
pub mod framegen;
pub mod geracao;
pub mod gamemode;
pub mod gpupref;
pub mod governador;
pub mod hardware;
pub mod bios;
pub mod eventoshw;
pub mod fichabios;
pub mod janelas;
pub mod apo;
pub mod x3d;
pub mod causas;
pub mod contadousuario;
pub mod conflitos;
pub mod cpugeracao;
pub mod discodojogo;
pub mod jogos;
pub mod labcompat;
pub mod health;
pub mod memory;
pub mod network;
pub mod nvdriver;
pub mod planoenergia;
pub mod experimento;
pub mod grupos;
pub mod naofazemos;
pub mod niveis;
pub mod pcie;
pub mod placa;
pub mod power;
pub mod pressao;
pub mod processes;
pub mod profiles;
pub mod prontojogo;
pub mod rbar;
pub mod readiness;
pub mod rede;
pub mod registry;
pub mod restore;
pub mod services;
pub mod servicesaudit;
pub mod sessao;
pub mod shaders;
pub mod shell;
pub mod startup;
pub mod suporte;
pub mod suspend;
pub mod sysparams;
pub mod tasks;
pub mod tetos;
pub mod thermal;
pub mod unreal;
pub mod topologia;
pub mod veredito;

use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeLog, ChangeRecord, PreviousValue};
use crate::modules::optimizer::{
    ActionResult, ActionStatus, BatchStep, OptimizationInfo, OptimizationOutcome, OptimizationState,
};
use crate::modules::safety::SafetyValidator;
use catalog::{Action, OptimizationSpec, RegValue};

const TCPIP_INTERFACES: &str = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionState {
    Satisfied,
    Pending,
    NotApplicable,
    /// NÃO DEU PARA LER. Caía em `NotApplicable`, e a tela dizia "não se aplica" sobre algo que ninguém verificou
    /// (ACL negada, imagem modificada, domínio). Em `NotApplicable` o produto SABE que não se aplica.
    Desconhecido,
}

pub struct WindowsOptimizer;

impl WindowsOptimizer {
    pub fn new() -> Self {
        WindowsOptimizer
    }

    pub fn list(&self, log: &ChangeLog) -> Vec<OptimizationInfo> {
        catalog::CATALOG
            .iter()
            .filter(|spec| !catalog::retirado(spec.id) || log.is_applied(spec.id))
            // Condicional com a condição medida e falsa some; fica se já aplicado (para desfazer) ou se não foi possível
            // medir (esconder afirmaria o que ninguém viu).
            .filter(|spec| match catalog::classe(spec.id) {
                catalog::Classe::Condicional(c) => condicao_atendida_sem_esperar(c) != Some(false) || log.is_applied(spec.id),
                _ => true,
            })
            .map(|spec| {
                let state = self.inspect(spec, log);

                // Rótulo sem motivo não ajuda: diz as três causas prováveis, a menos que o `detail` tenha algo mais específico.
                let detail = self.detail(spec).or_else(|| {
                    (state == OptimizationState::Unknown).then(|| {
                        "Não foi possível ler o estado atual desta configuração. \
                         Costuma ser permissão negada, Windows modificado ou política \
                         de domínio — o item fica fora do \"Otimizar agora\" por isso."
                            .to_string()
                    })
                });

                spec.to_info(state, detail, pesa_nesta_maquina(spec))
            })
            .collect()
    }

    /// Pura. A ORDEM É A REGRA: pendente vence desconhecido (uma leitura alheia não esconde uma otimização real); o
    /// desconhecido que sobra não vira "já está bom" nem "não se aplica".
    fn compor(states: &[ActionState]) -> OptimizationState {
        if states.iter().any(|s| *s == ActionState::Pending) {
            return OptimizationState::Available;
        }

        if states.iter().any(|s| *s == ActionState::Desconhecido) {
            return OptimizationState::Unknown;
        }

        if states.iter().all(|s| *s == ActionState::NotApplicable) {
            return OptimizationState::Unavailable;
        }

        OptimizationState::AlreadyOptimal
    }

    /// Pelo sistema, não por arquivo: senão ofereceria "otimizar" o que o PC já tem.
    fn inspect(&self, spec: &OptimizationSpec, log: &ChangeLog) -> OptimizationState {
        if log.is_applied(spec.id) {
            return OptimizationState::Applied;
        }

        // Otimização que faria mal a este hardware nem é oferecida.
        if !meets_requirement(spec) {
            return OptimizationState::Unavailable;
        }

        let states: Vec<ActionState> = spec.actions.iter().map(|a| self.inspect_action(a)).collect();

        Self::compor(&states)
    }

    fn detail(&self, spec: &OptimizationSpec) -> Option<String> {
        if let Some(requirement) = spec.requirement {
            if !meets_requirement(spec) {
                return Some(motivo_da_recusa(requirement));
            }
        }

        // O VBS é o único item em que o registro não é a verdade: `EnableVirtualizationBasedSecurity = 0` entra e relê 0,
        // mas o VBS pode seguir rodando (domínio, UEFI, Integridade de Memória religada). Pergunta a
        // `firmware::vbs_running` (`Win32_DeviceGuard`, número). Por id, porque a primeira ação é uma escrita comum.
        if spec.id == "disable_vbs" {
            return Some(
                match firmware::vbs_running() {
                    Some(true) => "Agora: VBS ligado e em execução nesta máquina.",
                    Some(false) => "Agora: VBS não está em execução nesta máquina.",
                    // Não saber não vira "está desligado".
                    None => "Não foi possível ler se o VBS está em execução nesta máquina.",
                }
                .to_string(),
            );
        }

        match spec.actions.first()? {
            // Sem elevação nem se LÊ: o item aparece porque não foi possível conferir, e o detalhe diz isso.
            Action::ReservedStorage { .. }
            | Action::RemoveForcedPlatformClock
            | Action::ClearBootLimits
                if !registry::is_elevated() =>
            {
                Some("Só dá para conferir o estado atual como administrador.".to_string())
            }

            // Elevado e ainda "acesso negado" (Windows 11 Pro): disponível porque não se conferiu, e escrito.
            Action::ReservedStorage { .. }
                if power::estado_do_armazenamento_reservado()
                    == power::EstadoReservado::NaoVerificavel =>
            {
                Some(
                    "O Windows recusou informar o estado atual, mesmo como administrador. \
                     Não sabemos se há espaço reservado nesta máquina."
                        .to_string(),
                )
            }

            _ => None,
        }
    }

    fn inspect_action(&self, action: &Action) -> ActionState {
        match action {
            Action::Registry {
                hive,
                path,
                name,
                value,
            } => {
                let current = registry::read(hive, path, name);
                let target = match value {
                    RegValue::Dword(v) => PreviousValue::Dword(*v),
                    RegValue::Binary(v) => PreviousValue::Binary(v.to_vec()),
                    RegValue::Text(v) => PreviousValue::Text(v.to_string()),
                };

                match current {
                    Ok(current) if current == target => ActionState::Satisfied,
                    Ok(_) => ActionState::Pending,
                    Err(_) => ActionState::Desconhecido,
                }
            }

            Action::DisableService { name } => {
                match services::exists(name) {
                    Some(true) => {}
                    // Instalações variam: isso o produto SABE.
                    Some(false) => return ActionState::NotApplicable,
                    // ACL negando a chave do serviço: "não se aplica" sumiria com o item com a frase errada.
                    None => return ActionState::Desconhecido,
                }

                match services::query_start_type(name) {
                    Ok(start_type) if start_type == "disabled" => ActionState::Satisfied,
                    Ok(_) => ActionState::Pending,
                    Err(_) => ActionState::Desconhecido,
                }
            }

            // Contra o plano de alto desempenho QUE EXISTE AQUI, não o GUID fixo, senão ficaria eternamente pendente.
            Action::PlanoOtimiza => match planoenergia::onde_esta_o_plano() {
                planoenergia::EstadoDoPlano::Ativo => ActionState::Satisfied,
                planoenergia::EstadoDoPlano::ExisteEnaoEstaAtivo
                | planoenergia::EstadoDoPlano::NaoExiste => ActionState::Pending,
                // Não ler NÃO é "não aplicado": seria pedir para refazer o que talvez já esteja feito.
                planoenergia::EstadoDoPlano::NaoConsegui => ActionState::Desconhecido,
            },

            Action::DisableNagle => match registry::subkeys("HKLM", TCPIP_INTERFACES) {
                Ok(interfaces) if !interfaces.is_empty() => {
                    let all_set = interfaces.iter().all(|interface| {
                        let path = format!("{}\\{}", TCPIP_INTERFACES, interface);
                        ["TcpAckFrequency", "TCPNoDelay"].iter().all(|name| {
                            matches!(
                                registry::read("HKLM", &path, name),
                                Ok(PreviousValue::Dword(1))
                            )
                        })
                    });

                    if all_set {
                        ActionState::Satisfied
                    } else {
                        ActionState::Pending
                    }
                }
                Ok(_) => ActionState::NotApplicable,
                // A lista ilegível virava "não se aplica" num PC com placa de rede.
                Err(_) => ActionState::Desconhecido,
            },

            Action::DisableHibernation => {
                match power::hibernation_enabled() {
                    Some(true) => ActionState::Pending,
                    Some(false) => ActionState::Satisfied,
                    // Não ler não vira "já está desativada" (espaço liberado sem olhar).
                    None => ActionState::Desconhecido,
                }
            }

            Action::MemoryCompression { enabled } => {
                // A RAM já foi checada em `meets_requirement`.
                match power::memory_compression_enabled() {
                    Some(current) if current == *enabled => ActionState::Satisfied,
                    Some(_) => ActionState::Pending,
                    // A compressão existe em todo Windows 10 e 11: `Get-MMAgent` calado não é "não se aplica".
                    None => ActionState::Desconhecido,
                }
            }

            // Estas três só respondem com elevação: sem ela, oferece e diz que a conferência exige administrador.
            Action::ClearBootLimits => {
                if !registry::is_elevated() {
                    return ActionState::Pending;
                }

                if firmware::boot_limits().is_empty() {
                    ActionState::Satisfied
                } else {
                    ActionState::Pending
                }
            }

            Action::GpuMsiMode => match devices::msi_ja_ativo() {
                Some(true) => ActionState::Satisfied,
                Some(false) => ActionState::Pending,
                // Chutar qual dispositivo é a GPU seria mexer em interrupção alheia.
                None => ActionState::NotApplicable,
            },

            Action::NicPowerSaving => match devices::economia_de_energia_da_rede_desligada() {
                Some(true) => ActionState::Satisfied,
                Some(false) => ActionState::Pending,
                None => ActionState::NotApplicable,
            },

            Action::ReservedStorage { enabled } => {
                if !registry::is_elevated() {
                    return ActionState::Pending;
                }

                use power::EstadoReservado;

                match power::estado_do_armazenamento_reservado() {
                    EstadoReservado::Ligado if *enabled => ActionState::Satisfied,
                    EstadoReservado::Desligado if !*enabled => ActionState::Satisfied,
                    EstadoReservado::Ligado | EstadoReservado::Desligado => ActionState::Pending,
                    EstadoReservado::SemRecurso => ActionState::NotApplicable,
                    // Não sabemos, e "não sabemos" não vira "não se aplica" nem `Pending` (que afirmaria haver o que aplicar).
                    EstadoReservado::NaoVerificavel => ActionState::Desconhecido,
                }
            }

            // Só quando alguma está ligada: no padrão do Windows a linha nem aparece.
            Action::AccessibilityKeysOff => {
                if acessibilidade::ligadas().is_empty() {
                    ActionState::Satisfied
                } else {
                    ActionState::Pending
                }
            }

            Action::JanelasOtimizadas => match janelas::ligada() {
                Ok(None) => ActionState::NotApplicable,
                Ok(Some(true)) => ActionState::Satisfied,
                Ok(Some(false)) => ActionState::Pending,
                Err(_) => ActionState::Desconhecido,
            },

            Action::DisableHypervisor => {
                if !registry::is_elevated() {
                    return ActionState::Pending;
                }

                match power::hypervisor_launch_type() {
                    Some(tipo) if tipo == "off" => ActionState::Satisfied,
                    Some(_) => ActionState::Pending,
                    // `None` é "não conseguimos ler", não "desligado".
                    None => ActionState::Desconhecido,
                }
            }

            Action::RemoveForcedPlatformClock => {
                if !registry::is_elevated() {
                    return ActionState::Pending;
                }

                if firmware::forced_platform_clock().is_some() {
                    ActionState::Pending
                } else {
                    ActionState::Satisfied
                }
            }
        }
    }

    /// Deixa rastro no `utils::logger`: início, cada ação antes de executar, e o fim, inclusive a falha que se
    /// desfez sozinha.
    pub fn apply(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        let inicio = std::time::Instant::now();
        crate::utils::Logger::info(&format!("aplicar `{}`: começou", id));

        let resultado = self.aplicar_sem_registro(id, log);
        anotar_fim("aplicar", id, inicio, &resultado);
        resultado
    }

    fn aplicar_sem_registro(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        let spec = catalog::find(id).ok_or_else(|| format!("Unknown optimization: {}", id))?;

        if catalog::retirado(id) && !log.is_applied(id) {
            return Err(format!(
                "`{}` foi retirado do Otimiza na 2.9: não muda FPS nem fluidez. \
                 Se estiver aplicado, ainda dá para desfazer.",
                spec.name
            ));
        }

        if log.is_applied(id) {
            return Ok(OptimizationOutcome {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                success: true,
                applied: true,
                message: "Otimização já estava aplicada.".to_string(),
                ..Default::default()
            });
        }

        if spec.requires_admin && !registry::is_elevated() {
            return Err(format!(
                "`{}` exige executar o programa como administrador.",
                spec.name
            ));
        }

        let inicio = std::time::Instant::now();
        let mut changes: Vec<ChangeRecord> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        let mut acoes: Vec<ActionResult> = Vec::new();
        let total_de_acoes = spec.actions.len();

        // `None` enquanto nada foi mexido.
        let mut diario: Option<crate::modules::transacao::Diario> = None;

        for (numero, action) in spec.actions.iter().enumerate() {
            // Antes de executar: se travar, a última linha do log diz qual.
            crate::utils::Logger::info(&format!(
                "aplicar `{}`: ação {}/{} — {:?}",
                spec.id,
                numero + 1,
                total_de_acoes,
                action
            ));

            // Nome, relógio e estado padrão saem daqui; o ramo preenche o que só ele sabe. Nasce `Verified` porque todo
            // ramo de escrita relê; os que sabem mais corrigem.
            let relogio = std::time::Instant::now();
            let mut detalhe = ActionResult {
                name: nome_da_acao(action),
                status: ActionStatus::Verified,
                ..Default::default()
            };

            let resultado = self.execute(action, &mut changes, &mut detalhe);
            detalhe.duration_ms = relogio.elapsed().as_millis() as u64;

            // Reescrito a cada ação: gravado antes do laço estaria vazio. Um `sync_all` por ação some ao lado das escritas.
            if !changes.is_empty() {
                diario = Some(crate::modules::transacao::abrir(
                    &crate::modules::transacao::Pendencia::nova(
                        spec.id,
                        spec.name,
                        crate::modules::transacao::Intencao::Aplicar,
                        now_timestamp(),
                        changes.clone(),
                    ),
                )?);
            }

            // Depois do ramo: quem manda na máquina é a mesma pergunta para todos (dezessete cópias seriam demais).
            detalhe.status = refinar_status(detalhe.status, governanca());

            match resultado {
                Ok(nota) => {
                    if let Some(note) = nota {
                        if detalhe.message.is_empty() {
                            detalhe.message.clone_from(&note);
                        }
                        notes.push(note);
                    }

                    explicar_governanca(&mut detalhe);
                    acoes.push(detalhe);
                }
                Err(error) => {
                    // Só quem não classificou vira `Failed`; `VerificationFailed` é decisão do ramo.
                    if detalhe.status == ActionStatus::Verified {
                        detalhe.status = ActionStatus::Failed;
                    }

                    detalhe.message.clone_from(&error);
                    explicar_governanca(&mut detalhe);
                    acoes.push(detalhe);

                    crate::utils::Logger::warn(&format!(
                        "aplicar `{}`: ação {}/{} falhou ({}); desfazendo {} mudança(s) já feitas",
                        spec.id,
                        numero + 1,
                        total_de_acoes,
                        error,
                        changes.len()
                    ));

                    // Se a própria reversão falhar vai para o log: é o único caso de PC em estado intermediário.
                    if let Err(failures) = revert_changes(&changes) {
                        crate::utils::Logger::error(&format!(
                            "reversão parcial de `{}` falhou: {}",
                            spec.id,
                            failures.join("; ")
                        ));
                    }

                    // Falha TRATADA fecha o diário: a máquina já voltou, e a pendência faria oferecer desfazer o desfeito.
                    if let Some(d) = diario.take() {
                        if let Err(erro) = d.concluir() {
                            crate::utils::Logger::warn(&format!(
                                "não consegui fechar o diário de `{}`: {}",
                                spec.id, erro
                            ));
                        }
                    }

                    return Err(format!("{}: {}", spec.name, error));
                }
            }
        }

        let changes_count = changes.len();
        let described: Vec<String> = changes.iter().map(|change| change.describe()).collect();

        log.record(AppliedOptimization {
            optimization_id: spec.id.to_string(),
            name: spec.name.to_string(),
            timestamp: now_timestamp(),
            changes,
        })?;

        // DEPOIS do `record`: o diário cobre a janela entre mexer no sistema e o histórico saber.
        if let Some(d) = diario.take() {
            d.concluir()?;
        }

        Ok(OptimizationOutcome {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            // Derivado das ações, não `true` cravado, para não divergir do que o cliente vê.
            success: acoes.iter().all(|a| a.status.deu_certo()),
            applied: true,
            message: success_message(spec, &notes),
            requires_restart: spec.requires_restart,
            requires_logoff: exige_logoff(spec),
            duration_ms: inicio.elapsed().as_millis() as u64,
            changes_count,
            changes: described,
            actions: acoes,
        })
    }

    /// Serve também ao que não está no catálogo (inicialização desligada pelo usuário): quem manda é o histórico.
    pub fn revert(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        let inicio = std::time::Instant::now();
        crate::utils::Logger::info(&format!("desfazer `{}`: começou", id));

        let resultado = self.desfazer_sem_registro(id, log);
        anotar_fim("desfazer", id, inicio, &resultado);
        resultado
    }

    fn desfazer_sem_registro(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        let spec = catalog::find(id);

        if let Some(spec) = spec {
            if !spec.reversible {
                return Err(format!(
                    "`{}` não pode ser desfeita — os arquivos foram apagados de verdade.",
                    spec.name
                ));
            }
        }

        let entry = match log.take(id)? {
            Some(entry) => entry,
            None => {
                let name = spec.map(|s| s.name.to_string()).unwrap_or_else(|| id.to_string());
                return Ok(OptimizationOutcome {
                    id: id.to_string(),
                    name,
                    success: true,
                    applied: false,
                    message: "Não estava aplicada.".to_string(),
                    ..Default::default()
                });
            }
        };

        let name = spec
            .map(|s| s.name.to_string())
            .unwrap_or_else(|| entry.name.clone());
        let requires_restart = spec.map(|s| s.requires_restart).unwrap_or(false);

        let changes_count = entry.changes.len();
        let described: Vec<String> = entry.changes.iter().map(|change| change.describe()).collect();

        // `log.take` já tirou e gravou o registro: daqui ao fim o valor anterior só existe na memória. O `if let Err`
        // cobre a reversão falhar; o diário cobre o processo morrer.
        let diario = crate::modules::transacao::abrir(&crate::modules::transacao::Pendencia::nova(
            id,
            &name,
            crate::modules::transacao::Intencao::Reverter,
            crate::modules::changelog::now_timestamp(),
            entry.changes.clone(),
        ))?;

        if let Err(errors) = revert_changes(&entry.changes) {
            // O registro volta ao histórico para tentar de novo.
            log.record(entry)?;
            diario.concluir()?;
            return Err(format!("Falha ao reverter `{}`: {}", name, errors.join("; ")));
        }

        diario.concluir()?;

        Ok(OptimizationOutcome {
            id: id.to_string(),
            name: name.clone(),
            success: true,
            applied: false,
            message: format!("`{}` foi desfeita.", name),
            requires_restart,
            changes_count,
            changes: described,
            ..Default::default()
        })
    }

    /// Desligar grava com id próprio; ligar desfaz esse registro. "Desfazer tudo" alcança.
    pub fn set_scheduled_task(
        &self,
        path: &str,
        name: &str,
        enabled: bool,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let id = format!("task:{}{}", path, name);

        if enabled {
            if log.is_applied(&id) {
                return self.revert(&id, log);
            }

            let change = tasks::definir_estado(path, name, true)?;
            return Ok(OptimizationOutcome {
                id,
                name: name.to_string(),
                success: true,
                applied: false,
                message: format!("`{}` volta a ser executada pelo agendador.", name),
                requires_restart: false,
                changes_count: 1,
                changes: vec![change.describe()],
            ..Default::default()
            });
        }

        let change = tasks::definir_estado(path, name, false)?;
        let described = change.describe();

        log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("{} (tarefa agendada)", name),
            timestamp: now_timestamp(),
            changes: vec![change],
        })?;

        Ok(OptimizationOutcome {
            id,
            name: name.to_string(),
            success: true,
            applied: true,
            message: format!("`{}` não é mais executada pelo agendador.", name),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// No histórico: "Desfazer tudo" remove, inclusive se o programa fechar no meio.
    pub fn set_persistent_priority(
        &self,
        executable: &str,
        enable: bool,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let id = format!("prioridade:{}", executable.to_lowercase());

        if !enable {
            if log.is_applied(&id) {
                return self.revert(&id, log);
            }

            return Err("Este jogo não tem prioridade fixada.".to_string());
        }

        if log.is_applied(&id) {
            return Err("Este jogo já está com prioridade fixada.".to_string());
        }

        let change = gamemode::definir_prioridade_persistente(executable, true)?;
        let described = change.describe();

        log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("{} em prioridade alta permanente", executable),
            timestamp: now_timestamp(),
            changes: vec![change],
        })?;

        Ok(OptimizationOutcome {
            id,
            name: executable.to_string(),
            success: true,
            applied: true,
            message: format!(
                "`{}` passa a abrir sempre em prioridade alta. Atenção: quando o jogo atualizar, o nome do executável muda e este ajuste precisa ser aplicado de novo — o Otimiza avisa quando isso acontecer.",
                executable
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// A única ação que muda a tela na hora: entra no histórico com a frequência anterior.
    pub fn set_max_refresh_rate(
        &self,
        dispositivo: &str,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let id = format!("monitor:{}", dispositivo.to_lowercase());

        let alvo = display::monitores()
            .into_iter()
            .find(|m| m.dispositivo == dispositivo)
            .ok_or_else(|| {
                format!(
                    "Não encontrei o monitor `{}`. Ele pode ter sido desconectado.",
                    dispositivo
                )
            })?;

        let maximo = alvo.hz_maximo();

        if !alvo.abaixo_do_maximo() {
            return Err(format!(
                "{} já está em {} Hz, que é o máximo desta resolução. Nada a fazer.",
                alvo.descricao, alvo.hz_atual
            ));
        }

        if log.is_applied(&id) {
            // Reaplicar por cima guardaria como "anterior" o que nós escrevemos.
            self.revert(&id, log)?;
        }

        let anterior = display::aplicar_hz(dispositivo, maximo)?;

        let change = ChangeRecord::RefreshRate {
            device: dispositivo.to_string(),
            previous_hz: anterior,
        };
        let described = change.describe();

        log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("{} em {} Hz", alvo.descricao, maximo),
            timestamp: now_timestamp(),
            changes: vec![change],
        })?;

        Ok(OptimizationOutcome {
            id,
            name: alvo.descricao.clone(),
            success: true,
            applied: true,
            // O cliente acabou de clicar e vai olhar o contador esperando mais FPS.
            message: format!(
                "{} passou de {} para {} Hz. O jogo fica visivelmente mais suave, e o contador de FPS continua onde estava — a taxa do monitor não cria quadros, ela deixa de segurar os que a placa já entrega.",
                alvo.descricao, anterior, maximo
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// Histórico gravado logo depois da escrita no driver; se não gravar, desfaz na hora (mudança sem caminho de
    /// volta).
    pub fn aplicar_ajuste_nvidia(
        &self,
        opcao: &str,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let alvo = nvdriver::opcao_por_id(opcao)
            .ok_or_else(|| format!("não conheço o ajuste de driver \"{}\".", opcao))?;
        let id = nvdriver::id_no_historico(opcao);

        if log.is_applied(&id) {
            return Ok(OptimizationOutcome {
                id,
                name: alvo.titulo.to_string(),
                success: true,
                applied: true,
                message: "Este ajuste já está aplicado pelo Otimiza.".to_string(),
                requires_restart: false,
                changes_count: 0,
                changes: Vec::new(),
            ..Default::default()
            });
        }

        crate::utils::Logger::info(&format!("driver NVIDIA `{}`: aplicando", opcao));
        let anterior = nvdriver::aplicar(opcao)?;

        let change = nvdriver::registro(opcao, anterior.clone());
        let described = change.describe();

        if let Err(erro) = log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("{} (driver NVIDIA)", alvo.titulo),
            timestamp: now_timestamp(),
            changes: vec![change],
        }) {
            crate::utils::Logger::warn(&format!(
                "driver NVIDIA `{}`: histórico não gravou ({}); desfazendo",
                opcao, erro
            ));
            if let Err(falhas) = revert_changes(&[nvdriver::registro(opcao, anterior)]) {
                return Err(format!(
                    "{} E não consegui devolver o ajuste ao que era: {}",
                    erro,
                    falhas.join("; ")
                ));
            }
            return Err(erro);
        }

        Ok(OptimizationOutcome {
            id,
            name: alvo.titulo.to_string(),
            success: true,
            applied: true,
            message: format!(
                "\"{}\" aplicado no perfil global do driver. Vale na próxima vez que o jogo \
                 abrir, e o Desfazer devolve o que existia antes.",
                alvo.titulo
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// NUNCA no perfil global (prenderia área de trabalho e todo jogo). Trocar o número de um jogo já limitado desfaz o
    /// anterior primeiro.
    pub fn limitar_fps_nvidia(
        &self,
        executavel: &str,
        fps: u32,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let id = nvdriver::id_do_limite(executavel);

        if log.is_applied(&id) {
            self.revert(&id, log)?;
        }

        crate::utils::Logger::info(&format!(
            "limite NVIDIA `{}`: {} FPS",
            executavel, fps
        ));
        let feito = nvdriver::aplicar_limite(executavel, fps)?;

        let change = ChangeRecord::LimiteNvidia {
            executavel: executavel.to_string(),
            fps,
            perfil_criado: feito.perfil_criado,
            valor_anterior: feito.valor_anterior,
        };
        let described = change.describe();

        if let Err(erro) = log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("{} limitado a {} FPS (driver NVIDIA)", executavel, fps),
            timestamp: now_timestamp(),
            changes: vec![change.clone()],
        }) {
            crate::utils::Logger::warn(&format!(
                "limite NVIDIA `{}`: histórico não gravou ({}); desfazendo",
                executavel, erro
            ));
            if let Err(falhas) = revert_changes(&[change]) {
                return Err(format!(
                    "{} E não consegui tirar o limite que acabei de pôr: {}",
                    erro,
                    falhas.join("; ")
                ));
            }
            return Err(erro);
        }

        Ok(OptimizationOutcome {
            id,
            name: executavel.to_string(),
            success: true,
            applied: true,
            message: format!(
                "`{}` limitado a {} quadros por segundo, no perfil dele no driver. Vale na \
                 próxima vez que o jogo abrir.",
                executavel, fps
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// Id próprio por jogo no histórico: "Desfazer tudo" devolve a preferência anterior, inclusive a ausência dela.
    pub fn set_gpu_preference(
        &self,
        caminho: &str,
        preferencia: gpupref::Preferencia,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let id = format!("placa:{}", caminho.to_lowercase());

        if log.is_applied(&id) {
            self.revert(&id, log)?;
        }

        let arquivo = std::path::Path::new(caminho);
        let change = gpupref::definir(arquivo, preferencia)?;
        let described = change.describe();

        let nome_curto = arquivo
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| caminho.to_string());

        log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("{} na {}", nome_curto, preferencia.nome()),
            timestamp: now_timestamp(),
            changes: vec![change],
        })?;

        Ok(OptimizationOutcome {
            id,
            name: nome_curto.clone(),
            success: true,
            applied: true,
            message: format!(
                "`{}` passa a usar a {}. Vale na próxima vez que o jogo abrir — se ele \
                 estiver aberto agora, feche e abra de novo.",
                nome_curto,
                preferencia.nome()
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// Voltar para automático desfaz o registro em vez de criar um segundo.
    pub fn set_dns(
        &self,
        guid: &str,
        servers: &str,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let id = format!("dns:{}", guid);

        if servers.trim().is_empty() {
            if log.is_applied(&id) {
                return self.revert(&id, log);
            }

            return Err("Este adaptador já usa o DNS que veio do roteador.".to_string());
        }

        let change = network::definir_dns(guid, servers)?;
        let described = change.describe();

        log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("DNS do adaptador ({})", servers),
            timestamp: now_timestamp(),
            changes: vec![change],
        })?;

        Ok(OptimizationOutcome {
            id,
            name: "DNS".to_string(),
            success: true,
            applied: true,
            message: format!(
                "DNS trocado para {}. Isso acelera achar o endereço dos sites; não muda o \
                 ping dentro do jogo.",
                servers
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// Id com o instante: um Windows modificado religa os desligados por fora, e cada religada se desfaz sozinha. Falha
    /// num não impede os outros, e o religado entra no histórico mesmo assim.
    pub fn religar_essenciais(&self, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        if !registry::is_elevated() {
            return Err(
                "Religar serviços do Windows exige executar o Otimiza como administrador."
                    .to_string(),
            );
        }

        let id = format!("religar_essenciais:{}", now_timestamp());
        let mut changes: Vec<ChangeRecord> = Vec::new();
        let mut falhas: Vec<String> = Vec::new();

        for essencial in essenciais::ESSENCIAIS {
            if essenciais::inicio_de(essencial.servico) != essenciais::Inicio::Desativado {
                continue;
            }

            crate::utils::Logger::info(&format!(
                "religar `{}`: de desativado para {}",
                essencial.servico, essencial.padrao
            ));

            match services::set_start_type(essencial.servico, essencial.padrao) {
                Ok(()) => changes.push(ChangeRecord::ServiceStartType {
                    service: essencial.servico.to_string(),
                    previous: "disabled".to_string(),
                }),
                Err(erro) => {
                    crate::utils::Logger::warn(&format!(
                        "religar `{}` falhou: {}",
                        essencial.servico, erro
                    ));
                    falhas.push(format!("{}: {}", essencial.servico, erro));
                }
            }
        }

        let religados = changes.len();

        if religados == 0 {
            if !falhas.is_empty() {
                return Err(format!(
                    "Nenhum serviço essencial foi religado. {}",
                    falhas.join(" · ")
                ));
            }

            return Ok(OptimizationOutcome {
                id,
                name: "Serviços essenciais do Windows".to_string(),
                success: true,
                applied: false,
                message: "Nenhum serviço essencial estava desativado.".to_string(),
                requires_restart: false,
                changes_count: 0,
                changes: Vec::new(),
            ..Default::default()
            });
        }

        let described: Vec<String> = changes.iter().map(|change| change.describe()).collect();

        log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: "Serviços essenciais do Windows religados".to_string(),
            timestamp: now_timestamp(),
            changes,
        })?;

        let message = if falhas.is_empty() {
            format!(
                "{} serviço(s) essencial(is) religado(s). Reinicie o PC para o Windows subir com eles.",
                religados
            )
        } else {
            format!(
                "{} religado(s); {} não: {}. Reinicie o PC para os religados valerem.",
                religados,
                falhas.len(),
                falhas.join(" · ")
            )
        };

        Ok(OptimizationOutcome {
            id,
            name: "Serviços essenciais do Windows".to_string(),
            success: falhas.is_empty(),
            applied: true,
            message,
            requires_restart: true,
            changes_count: religados,
            changes: described,
            ..Default::default()
        })
    }

    /// Voltar para Automático desfaz o registro em vez de criar um segundo.
    pub fn set_service_start(
        &self,
        name: &str,
        automatic: bool,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let id = format!("service:{}", name);

        if automatic {
            if log.is_applied(&id) {
                return self.revert(&id, log);
            }

            let change = servicesaudit::definir_inicio(name, true)?;
            return Ok(OptimizationOutcome {
                id,
                name: name.to_string(),
                success: true,
                applied: false,
                message: format!("`{}` volta a subir junto com o Windows.", name),
                requires_restart: false,
                changes_count: 1,
                changes: vec![change.describe()],
            ..Default::default()
            });
        }

        let change = servicesaudit::definir_inicio(name, false)?;
        let described = change.describe();

        log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("{} (serviço em Manual)", name),
            timestamp: now_timestamp(),
            changes: vec![change],
        })?;

        Ok(OptimizationOutcome {
            id,
            name: name.to_string(),
            success: true,
            applied: true,
            message: format!(
                "`{}` não sobe mais sozinho no boot. Ele ainda sobe quando o programa pedir.",
                name
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// Ligar de novo restaura exatamente o valor anterior.
    pub fn set_startup(
        &self,
        hive: &str,
        name: &str,
        enabled: bool,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        if hive.eq_ignore_ascii_case("HKLM") && !registry::is_elevated() {
            return Err(format!(
                "`{}` vale para todos os usuários do PC e exige executar como administrador.",
                name
            ));
        }

        let id = startup_change_id(hive, name);

        if enabled {
            if log.is_applied(&id) {
                return self.revert(&id, log);
            }

            let change = startup::set_enabled(hive, name, true)?;
            return Ok(OptimizationOutcome {
                id,
                name: name.to_string(),
                success: true,
                applied: false,
                message: format!("`{}` volta a iniciar com o Windows.", name),
                requires_restart: false,
                changes_count: 1,
                changes: vec![change.describe()],
                ..Default::default()
            });
        }

        let change = startup::set_enabled(hive, name, false)?;
        let described = change.describe();

        log.record(AppliedOptimization {
            optimization_id: id.clone(),
            name: format!("{} (inicialização)", name),
            timestamp: now_timestamp(),
            changes: vec![change],
        })?;

        Ok(OptimizationOutcome {
            id,
            name: name.to_string(),
            success: true,
            applied: true,
            message: format!("`{}` não sobe mais com o Windows.", name),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// `Some(ids)` é o caminho dos perfis; `None` é o "Otimizar agora". As exclusões de `catalog::entra_no_lote` vêm
    /// DEPOIS do filtro de ids: um perfil não arrasta o irreversível, a troca de segurança nem `FORA_DO_LOTE`. Uma
    /// falha não interrompe as outras.
    pub fn apply_selection<F>(
        &self,
        only: Option<&[String]>,
        log: &mut ChangeLog,
        mut on_step: F,
    ) -> Vec<OptimizationOutcome>
    where
        F: FnMut(BatchStep),
    {
        let pending: Vec<&OptimizationSpec> = catalog::CATALOG
            .iter()
            .filter(|spec| match only {
                Some(ids) => ids.iter().any(|id| id == spec.id),
                None => true,
            })
            .filter(|spec| catalog::entra_no_lote_se(spec, |c| condicao_atendida(c) == Some(true)))
            // SÓ `Available`: o botão que ninguém revisa leva só o que o produto CONFERIU que falta. `Unknown` fica na lista
            // para a pessoa decidir.
            .filter(|spec| self.inspect(spec, log) == OptimizationState::Available)
            .collect();

        let total = pending.len();

        crate::utils::Logger::info(&format!(
            "lote{}: {} item(ns) pendente(s): {}",
            if only.is_some() { " de perfil" } else { " do Otimizar agora" },
            total,
            pending.iter().map(|spec| spec.id).collect::<Vec<_>>().join(", ")
        ));

        pending
            .iter()
            .enumerate()
            .map(|(index, spec)| {
                on_step(BatchStep {
                    index: index + 1,
                    total,
                    name: spec.name.to_string(),
                    stage: "started",
                    message: String::new(),
                    changes: Vec::new(),
                    success: true,
                });

                let outcome = match self.apply(spec.id, log) {
                    Ok(outcome) => outcome,
                    Err(error) => OptimizationOutcome::failed(spec.id, spec.name, error),
                };

                on_step(BatchStep {
                    index: index + 1,
                    total,
                    name: spec.name.to_string(),
                    stage: "finished",
                    message: outcome.message.clone(),
                    changes: outcome.changes.clone(),
                    success: outcome.success,
                });

                outcome
            })
            .collect()
    }

    pub fn revert_all<F>(&self, log: &mut ChangeLog, mut on_step: F) -> Vec<OptimizationOutcome>
    where
        F: FnMut(BatchStep),
    {
        let applied: Vec<(String, String)> = log
            .applied()
            .iter()
            .map(|entry| (entry.optimization_id.clone(), entry.name.clone()))
            .collect();

        let total = applied.len();

        crate::utils::Logger::info(&format!(
            "desfazer tudo: {} item(ns): {}",
            total,
            applied.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>().join(", ")
        ));

        applied
            .iter()
            .enumerate()
            .map(|(index, (id, name))| {
                on_step(BatchStep {
                    index: index + 1,
                    total,
                    name: name.clone(),
                    stage: "started",
                    message: String::new(),
                    changes: Vec::new(),
                    success: true,
                });

                let outcome = match self.revert(id, log) {
                    Ok(outcome) => outcome,
                    Err(error) => OptimizationOutcome::failed(id, name, error),
                };

                on_step(BatchStep {
                    index: index + 1,
                    total,
                    name: name.clone(),
                    stage: "finished",
                    message: outcome.message.clone(),
                    changes: outcome.changes.clone(),
                    success: outcome.success,
                });

                outcome
            })
            .collect()
    }

    fn execute(
        &self,
        action: &Action,
        changes: &mut Vec<ChangeRecord>,
        detalhe: &mut ActionResult,
    ) -> Result<Option<String>, String> {
        match action {
            Action::Registry {
                hive,
                path,
                name,
                value,
            } => {
                let mut nao_confirmado: Option<String> = None;

                // O antes, lido antes de escrever.
                let antes = registry::read(hive, path, name);
                detalhe.before_value = antes.as_ref().ok().map(descrever_valor);
                detalhe.expected_value = Some(descrever_alvo(value));

                // Já no alvo: não escreve nem registra (gravava por cima e devolvia `Verified`, `antes=0 esperado=0 depois=0`).
                if matches!(conferir_escrita(value, &antes), Confirmacao::Igual) {
                    detalhe.status = ActionStatus::AlreadyOptimized;
                    detalhe.after_value.clone_from(&detalhe.before_value);
                    return Ok(None);
                }

                let previous = match value {
                    RegValue::Dword(v) => registry::set_dword(hive, path, name, *v)?,
                    RegValue::Binary(v) => registry::set_binary(hive, path, name, v)?,
                    RegValue::Text(v) => registry::set_string(hive, path, name, v)?,
                };

                // ANTES de conferir: se reprovar, o valor já está gravado, e este registro permite a reversão automática.
                changes.push(ChangeRecord::RegistryValue {
                    hive: hive.to_string(),
                    path: path.to_string(),
                    name: name.to_string(),
                    previous,
                });

                // Relê: `Ok` só diz que o Windows aceitou. Domínio reescreve a chave; em 32 bits sobre 64 a escrita cai no
                // `WOW6432Node`.
                let relido = registry::read(hive, path, name);
                detalhe.after_value = relido.as_ref().ok().map(descrever_valor);

                match conferir_escrita(value, &relido) {
                    Confirmacao::Igual => {}
                    Confirmacao::Diferente(lido) => {
                        // Aceito e não ficou: o problema não está no Otimiza.
                        detalhe.status = ActionStatus::VerificationFailed;

                        return Err(format!(
                            "O Windows aceitou a gravação de `{}`, mas ao reler o valor é {}. \
                             Costuma ser política de domínio ou outro programa reescrevendo a \
                             chave. Nada ficou aplicado.",
                            name, lido
                        ))
                    }
                    // Não é falha (desfazer descartaria uma mudança que provavelmente valeu) nem confirmação.
                    Confirmacao::NaoDeuParaLer => {
                        detalhe.status = ActionStatus::NotConfirmed;
                        nao_confirmado = Some(format!(
                            "`{}` foi gravado, mas não foi possível reler para confirmar.",
                            name
                        ));
                    }
                }

                // `HKCU\Control Panel` fica em memória desde o logon: sem avisar o Windows, valeria só no próximo.
                if sysparams::precisa_sincronizar_interface(hive, path) {
                    sysparams::sincronizar_interface();
                }

                // As duas notas coexistem: reiniciar o shell e não ter confirmado a gravação.
                let ativacao = sysparams::nota_de_ativacao(hive, path, name).map(|n| n.to_string());

                Ok(match (nao_confirmado, ativacao) {
                    (Some(a), Some(b)) => Some(format!("{} {}", a, b)),
                    (Some(unica), None) | (None, Some(unica)) => Some(unica),
                    (None, None) => None,
                })
            }

            Action::DisableService { name } => {
                // Segunda barreira: nem uma mudança futura no catálogo desativa um serviço crítico.
                let validation = SafetyValidator::new().validate_operation("service_disable", name);
                if !validation.valid {
                    return Err(format!("Serviço crítico bloqueado: {}", name));
                }

                match services::exists(name) {
                    Some(true) => {}
                    Some(false) => {
                        detalhe.status = ActionStatus::Unsupported;
                        detalhe.unsupported_reason =
                            Some(format!("O serviço {} não existe neste Windows.", name));
                        return Ok(None);
                    }
                    // Sem o tipo de inicialização atual não há volta.
                    None => {
                        detalhe.status = ActionStatus::Skipped;
                        detalhe.message = format!(
                            "Não foi possível ler a configuração do serviço {} nesta máquina.",
                            name
                        );
                        return Ok(None);
                    }
                }

                let previous = services::query_start_type(name)?;
                detalhe.before_value = Some(previous.clone());
                detalhe.expected_value = Some("disabled".to_string());

                if previous == "disabled" {
                    detalhe.status = ActionStatus::AlreadyOptimized;
                    detalhe.after_value = Some(previous);
                    return Ok(None);
                }

                services::set_start_type(name, "disabled")?;
                changes.push(ChangeRecord::ServiceStartType {
                    service: name.to_string(),
                    previous,
                });

                // `sc config` devolver 0 não prova: com domínio ou serviço trancado, o tipo volta.
                let agora = services::query_start_type(name).ok();
                detalhe.after_value.clone_from(&agora);

                let nota = exigir_confirmacao(
                    conferir(&"disabled".to_string(), agora),
                    &format!("o serviço {}", name),
                )
                .inspect_err(|_| detalhe.status = ActionStatus::VerificationFailed)?;

                // Falhar ao parar não invalida: vale do próximo boot.
                if let Err(error) = services::stop(name) {
                    crate::utils::Logger::warn(&format!("serviço {} não parou agora: {}", name, error));
                }
                Ok(nota)
            }

            Action::PlanoOtimiza => {
                let relatorio = planoenergia::montar(false)?;

                if !relatorio.plano_ativo {
                    return Err(
                        "O plano OTIMIZA foi montado mas o Windows não o deixou ativo. \
                         Nada foi mudado no seu plano de energia."
                            .to_string(),
                    );
                }

                let Some(anterior) = relatorio.guid_anterior.clone() else {
                    return Err(
                        "Não foi possível ler qual plano de energia estava ativo antes. \
                         O plano OTIMIZA não foi ativado."
                            .to_string(),
                    );
                };

                // O OTIMIZA já era o ativo: `previous_guid` igual ao nosso faria o desfazer reativar o próprio plano a remover.
                if anterior.eq_ignore_ascii_case(
                    relatorio.guid_do_plano.as_deref().unwrap_or_default(),
                ) {
                    return Ok(Some(nota_do_plano(&relatorio)));
                }

                changes.push(ChangeRecord::PowerPlan {
                    previous_guid: anterior,
                });

                Ok(Some(nota_do_plano(&relatorio)))
            }

            Action::DisableHibernation => {
                detalhe.expected_value = Some("false".to_string());

                let Some(previously_enabled) = power::hibernation_enabled() else {
                    detalhe.status = ActionStatus::Skipped;
                    detalhe.message =
                        "Não foi possível ler se a hibernação está ligada nesta máquina."
                            .to_string();
                    return Ok(None);
                };

                detalhe.before_value = Some(previously_enabled.to_string());

                if !previously_enabled {
                    detalhe.status = ActionStatus::AlreadyOptimized;
                    detalhe.after_value = Some("false".to_string());
                    return Ok(None);
                }

                power::set_hibernation(false)?;
                changes.push(ChangeRecord::Hibernation { previously_enabled });

                // Muda na hora e dá para conferir: sem desligar, o `hiberfil.sys` fica. `Option` direto: um `Some` embrulhando o
                // `false` da leitura quebrada validava a si mesmo; `None` cai em `NaoDeuParaLer`.
                let depois = power::hibernation_enabled();
                detalhe.after_value = depois.map(|v| v.to_string());

                match exigir_confirmacao(conferir(&false, depois), "a hibernação")? {
                    Some(ressalva) => Ok(Some(ressalva)),
                    None => Ok(Some("Arquivo de hibernação removido.".to_string())),
                }
            }

            Action::MemoryCompression { enabled } => {
                let previously_enabled = power::memory_compression_enabled()
                    .ok_or("Não foi possível ler o estado da compressão de memória.")?;

                if previously_enabled == *enabled {
detalhe.status = ActionStatus::AlreadyOptimized;
                    return Ok(None);
                }

                power::set_memory_compression(*enabled)?;
                changes.push(ChangeRecord::MemoryCompression { previously_enabled });

                exigir_confirmacao(
                    conferir(enabled, power::memory_compression_enabled()),
                    "a compressão de memória",
                )
            }

            Action::ClearBootLimits => {
                let removed = firmware::boot_limits();

                if removed.is_empty() {
detalhe.status = ActionStatus::AlreadyOptimized;
                    return Ok(None);
                }

                for (key, _) in &removed {
                    shell::run_checked("bcdedit", &["/deletevalue", "{current}", key])?;
                }

                let count = removed.len();
                changes.push(ChangeRecord::BootLimits { removed });
                Ok(Some(format!("{} limite(s) removido(s).", count)))
            }

            Action::GpuMsiMode => {
                devices::ativar_msi(changes)?;

                // O Windows pode reescrever a chave ao reenumerar o hardware; o valor é legível agora.
                exigir_confirmacao(
                    conferir(&true, devices::msi_ja_ativo()),
                    "o modo MSI da placa de vídeo",
                )
            }

            Action::NicPowerSaving => {
                devices::desligar_economia_de_energia_da_rede(changes)?;

                exigir_confirmacao(
                    conferir(&true, devices::economia_de_energia_da_rede_desligada()),
                    "a economia de energia da placa de rede",
                )
            }

            Action::ReservedStorage { enabled } => {
                use power::EstadoReservado;

                // A mesma distinção da inspeção: senão a inspeção diria "não sabemos" e a aplicação "não tem o recurso".
                let anterior = match power::estado_do_armazenamento_reservado() {
                    EstadoReservado::Ligado => true,
                    EstadoReservado::Desligado => false,
                    EstadoReservado::SemRecurso => {
                        return Err("Este Windows não tem Armazenamento Reservado.".to_string())
                    }
                    EstadoReservado::NaoVerificavel => {
                        return Err(
                            "O Windows recusou informar se há espaço reservado, mesmo com o \
                             Otimiza como administrador. Sem saber o estado atual não há como \
                             desfazer depois, então nada foi alterado."
                                .to_string(),
                        )
                    }
                };

                if anterior == *enabled {
detalhe.status = ActionStatus::AlreadyOptimized;
                    return Ok(None);
                }

                power::set_reserved_storage(*enabled)?;
                changes.push(ChangeRecord::ReservedStorage {
                    previously_enabled: anterior,
                });

                // Com atualização em andamento o Windows recusa, nem sempre por erro.
                let agora = match power::estado_do_armazenamento_reservado() {
                    EstadoReservado::Ligado => Some(true),
                    EstadoReservado::Desligado => Some(false),
                    EstadoReservado::SemRecurso | EstadoReservado::NaoVerificavel => None,
                };

                match exigir_confirmacao(
                    conferir(enabled, agora),
                    "o Armazenamento Reservado",
                )? {
                    Some(ressalva) => Ok(Some(ressalva)),
                    None => Ok(Some("Espaço reservado devolvido ao disco.".to_string())),
                }
            }

            Action::AccessibilityKeysOff => {
                let ligadas: Vec<&str> = acessibilidade::ligadas();

                if ligadas.is_empty() {
detalhe.status = ActionStatus::AlreadyOptimized;
                    return Ok(None);
                }

                // Por referência: se a segunda chave falhar, a primeira continua no histórico e a reversão a desfaz.
                acessibilidade::desligar(changes)?;

                // Em memória desde o logon, como as do mouse.
                sysparams::sincronizar_interface();

                Ok(Some(format!("{} desligada(s).", ligadas.join(", "))))
            }

            Action::JanelasOtimizadas => {
                match janelas::ligada()? {
                    None => {
                        detalhe.status = ActionStatus::Skipped;
                        detalhe.message = "Esta opção só existe no Windows 11.".to_string();
                        return Ok(None);
                    }
                    Some(true) => {
                        detalhe.status = ActionStatus::AlreadyOptimized;
                        return Ok(None);
                    }
                    Some(false) => {}
                }
                changes.push(janelas::ligar()?);
                Ok(Some("Ligada. Vale a partir da próxima vez que o jogo abrir.".to_string()))
            }

            Action::DisableHypervisor => {
                let Some(anterior) = power::hypervisor_launch_type() else {
                    // Sem o estado atual não há volta, e marcar aplicada afirmaria o hipervisor desligado sem ler.
                    detalhe.status = ActionStatus::Skipped;
                    detalhe.message =
                        "Não foi possível ler como o hipervisor sobe no boot.".to_string();
                    return Ok(None);
                };

                detalhe.before_value = Some(anterior.clone());
                detalhe.expected_value = Some("off".to_string());

                if anterior == "off" {
detalhe.status = ActionStatus::AlreadyOptimized;
                    return Ok(None);
                }

                shell::run_checked("bcdedit", &["/set", "{current}", "hypervisorlaunchtype", "off"])?;

                changes.push(ChangeRecord::BootLimits {
                    removed: vec![("hypervisorlaunchtype".to_string(), anterior)],
                });

                // Com VBS imposto por política ou UEFI o `bcdedit` devolve 0 e o valor não fica.
                match exigir_confirmacao(
                    conferir(&"off".to_string(), power::hypervisor_launch_type()),
                    "o hipervisor no boot",
                )? {
                    Some(ressalva) => Ok(Some(ressalva)),
                    None => Ok(Some(
                        "Hipervisor desligado no boot — vale depois de reiniciar.".to_string(),
                    )),
                }
            }

            Action::RemoveForcedPlatformClock => {
                let Some(valor) = firmware::forced_platform_clock() else {
                    detalhe.status = ActionStatus::AlreadyOptimized;
                    return Ok(None);
                };

                detalhe.before_value = Some(valor.clone());
                detalhe.expected_value = Some("ausente".to_string());

                shell::run_checked("bcdedit", &["/deletevalue", "{current}", "useplatformclock"])?;

                changes.push(ChangeRecord::BootLimits {
                    removed: vec![("useplatformclock".to_string(), valor.clone())],
                });

                match firmware::forced_platform_clock() {
                    None => Ok(Some(format!(
                        "Relógio de plataforma forçado removido (estava em {}).",
                        valor
                    ))),
                    Some(ainda) => Err(format!(
                        "O Windows aceitou o comando, mas o relógio de plataforma continua \
                         forçado em {}. Nada ficou aplicado.",
                        ainda
                    )),
                }
            }

            Action::DisableNagle => {
                let interfaces = registry::subkeys("HKLM", TCPIP_INTERFACES)?;

                let mut sem_confirmar = 0usize;

                for interface in interfaces {
                    let path = format!("{}\\{}", TCPIP_INTERFACES, interface);

                    for name in ["TcpAckFrequency", "TCPNoDelay"] {
                        let previous = registry::set_dword("HKLM", &path, name, 1)?;
                        changes.push(ChangeRecord::RegistryValue {
                            hive: "HKLM".to_string(),
                            path: path.clone(),
                            name: name.to_string(),
                            previous,
                        });

                        // Escreve direto, sem o ramo `Action::Registry`: confere por conta própria. O erro derruba a otimização: Nagle
                        // meio desligado deixa a latência dependendo da placa que o Windows escolher.
                        match conferir_escrita(
                            &RegValue::Dword(1),
                            &registry::read("HKLM", &path, name),
                        ) {
                            Confirmacao::Igual => {}
                            Confirmacao::Diferente(lido) => {
                                return Err(format!(
                                    "O Windows aceitou gravar `{}` nesta placa de rede, mas ao \
                                     reler o valor é {}. Nada ficou aplicado.",
                                    name, lido
                                ))
                            }
                            Confirmacao::NaoDeuParaLer => sem_confirmar += 1,
                        }
                    }
                }

                Ok((sem_confirmar > 0).then(|| {
                    format!(
                        "{} valor(es) foram gravados, mas não foi possível reler para confirmar.",
                        sem_confirmar
                    )
                }))
            }
        }
    }
}

impl Default for WindowsOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// O prefixo evita colisão entre um programa "SysMain" e a otimização de mesmo nome.
fn startup_change_id(hive: &str, name: &str) -> String {
    format!("startup:{}:{}", hive.to_uppercase(), name)
}

/// A condição de um item condicional, MEDIDA nesta máquina. `None` quando não deu para medir: nem "não se aplica"
/// nem lote.
type CacheDasCondicoes = std::sync::Mutex<Vec<(catalog::Condicao, std::time::Instant, Option<bool>)>>;

fn cache_das_condicoes() -> &'static CacheDasCondicoes {
    static LEMBRADO: std::sync::OnceLock<CacheDasCondicoes> = std::sync::OnceLock::new();
    LEMBRADO.get_or_init(Default::default)
}

const VALIDADE_DA_CONDICAO: std::time::Duration = std::time::Duration::from_secs(30);

fn condicao_lembrada(c: catalog::Condicao) -> Option<Option<bool>> {
    let cache = cache_das_condicoes().lock().ok()?;
    cache
        .iter()
        .find(|(qual, _, _)| *qual == c)
        .filter(|(_, quando, _)| quando.elapsed() < VALIDADE_DA_CONDICAO)
        .map(|(_, _, valor)| *valor)
}

pub fn condicao_atendida(c: catalog::Condicao) -> Option<bool> {
    // Meio minuto: o espaço em disco custou 850 ms por chamada, uma por item condicional.
    if let Some(valor) = condicao_lembrada(c) {
        return valor;
    }
    let valor = medir_condicao(c);
    guardar_condicao(c, valor);
    valor
}

fn guardar_condicao(c: catalog::Condicao, valor: Option<bool>) {
    if let Some(cache) = cache_das_condicoes().lock().ok().as_mut() {
        cache.retain(|(qual, _, _)| *qual != c);
        cache.push((c, std::time::Instant::now(), valor));
    }
}

/// Sem esperar: a primeira leitura de espaço custou 3,4 s. Sem resposta, `None` (o item CONTINUA aparecendo) e
/// mede numa thread à parte. Nunca esconde por pressa.
pub fn condicao_atendida_sem_esperar(c: catalog::Condicao) -> Option<bool> {
    if let Some(valor) = condicao_lembrada(c) {
        return valor;
    }
    std::thread::spawn(move || {
        let valor = medir_condicao(c);
        guardar_condicao(c, valor);
    });
    None
}

pub fn aquecer_condicoes() {
    for c in [
        catalog::Condicao::GameDvrLigado,
        catalog::Condicao::PcFraco,
        catalog::Condicao::MemoriaApertada,
        catalog::Condicao::PoucoEspaco,
    ] {
        let valor = medir_condicao(c);
        guardar_condicao(c, valor);
    }
}

fn medir_condicao(c: catalog::Condicao) -> Option<bool> {
    use catalog::Condicao;
    let perfil = hardware::profile();
    match c {
        Condicao::GameDvrLigado => gamedvr_ligado(
            registry::read("HKCU", r"System\GameConfigStore", "GameDVR_Enabled").ok(),
            registry::read("HKCU", r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled").ok(),
        ),
        Condicao::PcFraco => Some(perfil.total_ram_gb <= 8.5 || perfil.logical_cores <= 4),
        Condicao::MemoriaApertada => Some(perfil.total_ram_gb > 0.0 && perfil.total_ram_gb <= 16.5),
        Condicao::PoucoEspaco => diskspace::disk_usage().map(|(_, livre)| livre < 20 * 1024 * 1024 * 1024),
        Condicao::SoSePedir => Some(false),
    }
}

/// **Pura.** `GameDVR_Enabled` ausente é o padrão, ligado; `AppCaptureEnabled = 1` também liga.
fn gamedvr_ligado(dvr: Option<PreviousValue>, captura: Option<PreviousValue>) -> Option<bool> {
    let ligado = |v: &PreviousValue| match v {
        PreviousValue::Dword(n) => Some(*n != 0),
        PreviousValue::Absent | PreviousValue::AbsentKey => None,
        _ => Some(true),
    };
    let dvr = dvr?;
    let captura = captura?;
    Some(ligado(&dvr).unwrap_or(true) || ligado(&captura).unwrap_or(false))
}

fn pesa_nesta_maquina(spec: &OptimizationSpec) -> bool {
    use catalog::Boost;
    use hardware::StorageKind;

    let perfil = hardware::profile();

    spec.highlight_when.iter().any(|condicao| match condicao {
        // Abaixo de 8 GB o Windows já comprime e pagina em uso comum.
        Boost::LowRam => perfil.total_ram_gb <= 8.5,
        Boost::MechanicalDisk => perfil.system_storage == StorageKind::Hdd,
        Boost::FewCores => perfil.logical_cores <= 4,
    })
}

/// Disco desconhecido não atende: não se arrisca piorar o PC por palpite.
fn meets_requirement(spec: &OptimizationSpec) -> bool {
    use catalog::Requirement;
    use hardware::StorageKind;

    match spec.requirement {
        None => true,
        Some(Requirement::SsdSystemDrive) => {
            hardware::profile().system_storage == StorageKind::Ssd
        }
        Some(Requirement::MinRamGb(minimum)) => hardware::profile().total_ram_gb >= minimum,
        Some(Requirement::MinWddm(minimo)) => {
            hardware::alcanca_wddm(hardware::wddm_version(), minimo)
        }
    }
}

/// Numa imagem sem o provedor WMI de armazenamento o disco é `Unknown`, e a frase dizia "seu disco não é SSD". A
/// recusa continua (SysMain em HD piora); a frase diz o que aconteceu.
pub fn motivo_da_recusa(requirement: catalog::Requirement) -> String {
    use catalog::Requirement;
    use hardware::StorageKind;

    match requirement {
        Requirement::SsdSystemDrive => match hardware::profile().system_storage {
            StorageKind::Hdd => "Não oferecemos: seu disco de sistema é mecânico, e aqui isso \
                                 deixaria o PC mais lento."
                .to_string(),
            StorageKind::Unknown => "Não oferecemos: não foi possível ler se o disco de sistema é \
                                     SSD ou mecânico nesta máquina. Em disco mecânico este ajuste \
                                     piora o PC, e sem saber o tipo não dá para arriscar."
                .to_string(),
            StorageKind::Ssd => "Não oferecemos, e o motivo não pôde ser determinado.".to_string(),
        },
        Requirement::MinRamGb(minimo) => format!(
            "Não oferecemos: esta máquina tem {:.0} GB de memória e este ajuste só ajuda a partir \
             de {:.0} GB. Abaixo disso, aplicar piora o desempenho.",
            hardware::profile().total_ram_gb,
            minimo
        ),
        Requirement::MinWddm(_) => requirement.unmet_reason().to_string(),
    }
}

/// Política de grupo e imagem de terceiros chegavam ao atendimento como "falhou".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Governanca {
    pub com_politica_de_grupo: bool,
    pub imagem_de_terceiros: bool,
}

/// Consultado uma vez por AÇÃO, dezenas por lote.
static GOVERNANCA: std::sync::OnceLock<Governanca> = std::sync::OnceLock::new();

pub fn governanca() -> Governanca {
    *GOVERNANCA.get_or_init(|| {
        let checagem = essenciais::checar();

        Governanca {
            com_politica_de_grupo: ha_politica_aplicada(),
            // Todo PC de marca declara fabricante: o sinal é ele JUNTO de serviço essencial desligado. A mesma definição do
            // relatório de compatibilidade.
            imagem_de_terceiros: checagem.fabricante.is_some() && checagem.desativados > 0,
        }
    })
}

/// A chave existir não é sinal (existe vazia sem domínio): o sinal é ter subchave.
fn ha_politica_aplicada() -> bool {
    registry::subkeys(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Group Policy\History",
    )
    .map(|gpos| !gpos.is_empty())
    .unwrap_or(false)
}

/// Pura. SÓ REFINA, NUNCA INVENTA: estado conclusivo não muda por causa do ambiente.
pub fn refinar_status(status: ActionStatus, g: Governanca) -> ActionStatus {
    match status {
        ActionStatus::VerificationFailed if g.com_politica_de_grupo => {
            ActionStatus::BlockedByPolicy
        }
        ActionStatus::Unsupported if g.imagem_de_terceiros => ActionStatus::UserOrOemManaged,
        outro => outro,
    }
}

/// Estado novo sem frase nova não serve: nomeia a IMAGEM quando há uma ("Team AntiLag / SnyX OS" se pesquisa).
fn explicar_governanca(detalhe: &mut ActionResult) {
    match detalhe.status {
        ActionStatus::BlockedByPolicy => {
            let aviso = "Esta máquina tem política de grupo aplicada, e é ela que costuma \
                         reescrever este valor. Quem pode mudar isso é quem administra o \
                         computador.";

            if detalhe.message.is_empty() {
                detalhe.message = aviso.to_string();
            } else {
                detalhe.message.push(' ');
                detalhe.message.push_str(aviso);
            }
        }
        ActionStatus::UserOrOemManaged => {
            let checagem = essenciais::checar();

            let quem = match (checagem.fabricante.as_deref(), checagem.modelo.as_deref()) {
                (Some(f), Some(m)) => format!("{} / {}", f, m),
                (Some(f), None) => f.to_string(),
                _ => "outra pessoa".to_string(),
            };

            detalhe.unsupported_reason = Some(format!(
                "Este Windows foi montado por {} e vem sem esta parte. Num Windows de fábrica \
                 ela existe — não é limitação do seu computador.",
                quem
            ));
        }
        _ => {}
    }
}

/// Para ler no atendimento, não o `{:?}` do enum; caminho e valores viajam nos campos do `ActionResult`.
pub fn nome_da_acao(action: &Action) -> String {
    match action {
        Action::Registry { hive, name, .. } => format!("registro {}\\…\\{}", hive, name),
        Action::DisableService { name } => format!("serviço {}", name),
        Action::PlanoOtimiza => "plano de energia OTIMIZA".to_string(),
        Action::DisableNagle => "algoritmo de Nagle nas placas de rede".to_string(),
        Action::DisableHibernation => "hibernação".to_string(),
        Action::MemoryCompression { .. } => "compressão de memória".to_string(),
        Action::GpuMsiMode => "modo MSI da placa de vídeo".to_string(),
        Action::NicPowerSaving => "economia de energia da placa de rede".to_string(),
        Action::ClearBootLimits => "limites de inicialização".to_string(),
        Action::ReservedStorage { .. } => "Armazenamento Reservado".to_string(),
        Action::AccessibilityKeysOff => "teclas de acessibilidade".to_string(),
        Action::JanelasOtimizadas => "otimizações para jogos em janela".to_string(),
        Action::DisableHypervisor => "hipervisor no boot".to_string(),
        Action::RemoveForcedPlatformClock => "relógio de plataforma forçado".to_string(),
    }
}

/// Derivado de `sysparams::nota_de_ativacao`, não campo novo no catálogo: duas fontes divergem.
pub fn exige_logoff(spec: &catalog::OptimizationSpec) -> bool {
    spec.actions.iter().any(|action| match action {
        Action::Registry {
            hive, path, name, ..
        } => sysparams::nota_de_ativacao(hive, path, name).is_some(),
        _ => false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirmacao {
    Igual,
    Diferente(String),
    NaoDeuParaLer,
}

/// Pura. `set_dword` `Ok` não prova (domínio reescreve; `WOW6432Node`).
pub fn conferir_escrita(alvo: &RegValue, lido: &Result<PreviousValue, String>) -> Confirmacao {
    let Ok(atual) = lido else {
        return Confirmacao::NaoDeuParaLer;
    };

    let igual = match (alvo, atual) {
        (RegValue::Dword(esperado), PreviousValue::Dword(v)) => v == esperado,
        (RegValue::Text(esperado), PreviousValue::Text(v)) => v == esperado,
        (RegValue::Binary(esperado), PreviousValue::Binary(v)) => v.as_slice() == *esperado,
        // Tipo diferente é valor diferente: o Windows não leria como queríamos.
        _ => false,
    };

    if igual {
        Confirmacao::Igual
    } else {
        Confirmacao::Diferente(descrever_valor(atual))
    }
}

/// Para o que NÃO é registro (serviços, `bcdedit`, MSI, hibernação, compressão): `sc config` 0 não prova, e o
/// histórico anotava mudança inexistente. `None` em `lido` é "não deu para reler" (ver `exigir_confirmacao`).
pub fn conferir<T>(esperado: &T, lido: Option<T>) -> Confirmacao
where
    T: PartialEq + std::fmt::Debug,
{
    match lido {
        None => Confirmacao::NaoDeuParaLer,
        Some(atual) if atual == *esperado => Confirmacao::Igual,
        Some(atual) => Confirmacao::Diferente(format!("{:?}", atual)),
    }
}

/// Uma regra só para nove ações, senão uma delas acaba tratando "não li" como sucesso.
pub fn exigir_confirmacao(c: Confirmacao, o_que: &str) -> Result<Option<String>, String> {
    match c {
        Confirmacao::Igual => Ok(None),
        Confirmacao::Diferente(atual) => Err(format!(
            "O Windows aceitou o comando, mas ao reler {} continua {}. Nada ficou aplicado.",
            o_que, atual
        )),
        // Não desfaz (provavelmente valeu), mas não passa calado.
        Confirmacao::NaoDeuParaLer => Ok(Some(format!(
            "{} foi alterado, mas não foi possível reler para confirmar.",
            o_que
        ))),
    }
}

/// Mesma impressão dos dois lados: formatações diferentes fariam parecer que mudou.
pub fn descrever_alvo(valor: &RegValue) -> String {
    match valor {
        RegValue::Dword(v) => v.to_string(),
        RegValue::Text(v) => format!("\"{}\"", v),
        RegValue::Binary(bytes) => format!("{} byte(s)", bytes.len()),
    }
}

fn descrever_valor(valor: &PreviousValue) -> String {
    match valor {
        PreviousValue::Dword(v) => v.to_string(),
        PreviousValue::Text(v) => format!("\"{}\"", v),
        PreviousValue::Binary(bytes) => format!("{} byte(s)", bytes.len()),
        PreviousValue::Absent => "inexistente".to_string(),
        PreviousValue::AbsentKey => "inexistente (a chave sumiu)".to_string(),
    }
}

/// "Já estava bom" é a resposta inteira num PC que já usava plano de desempenho.
pub fn nota_do_plano(r: &planoenergia::RelatorioDoPlano) -> String {
    let mut partes = vec![format!("{} ajuste(s) aplicados e conferidos", r.aplicados)];

    if r.ja_estavam_bons > 0 {
        partes.push(format!("{} já estavam bons", r.ja_estavam_bons));
    }

    if r.nao_suportados > 0 {
        partes.push(format!(
            "{} não existem neste Windows",
            r.nao_suportados
        ));
    }

    if r.falhas > 0 {
        partes.push(format!("{} não entraram", r.falhas));
    }

    format!("Plano OTIMIZA ativo: {}.", partes.join(", "))
}

fn anotar_fim(
    verbo: &str,
    id: &str,
    inicio: std::time::Instant,
    resultado: &Result<OptimizationOutcome, String>,
) {
    let ms = inicio.elapsed().as_millis();

    match resultado {
        Ok(outcome) => crate::utils::Logger::info(&format!(
            "{} `{}`: terminou em {} ms ({} mudança(s))",
            verbo, id, ms, outcome.changes_count
        )),
        Err(erro) => crate::utils::Logger::warn(&format!(
            "{} `{}`: falhou em {} ms — {}",
            verbo, id, ms, erro
        )),
    }
}

/// O portão "nunca menos FPS": avalia cada ajuste em observação contra as medições automáticas e DESFAZ o que
/// piorou. Devolve o decidido agora, para a tela.
pub fn decidir_portao(log: &mut ChangeLog) -> Vec<crate::modules::portao::Decidido> {
    use crate::modules::portao::{self, Decidido, Veredito};
    let mut estado = portao::ler();
    let governador_em_aberto = estado.governador.iter().any(|g| g.decidido.is_none());
    if estado.vigiados.is_empty() && !governador_em_aberto {
        return Vec::new();
    }
    let Ok(medicoes) = crate::modules::medicoes::ler() else { return Vec::new() };
    let agora = crate::modules::changelog::now_timestamp();
    let mut decididos = Vec::new();
    let mut ficam = Vec::new();
    for v in std::mem::take(&mut estado.vigiados) {
        if !log.is_applied(&v.id) {
            continue;
        }
        let a = portao::avaliar(&v, &medicoes);
        if matches!(a.veredito, Veredito::Aguardando { .. }) {
            ficam.push(v);
            continue;
        }
        let erro = if a.veredito == Veredito::Desfazer {
            match WindowsOptimizer::new().revert(&v.id, log) {
                Ok(r) if r.success => None,
                Ok(r) => Some(r.message),
                Err(e) => Some(e),
            }
        } else {
            None
        };
        decididos.push(Decidido {
            fps_antes: a.fps.as_ref().map(|c| c.media_base),
            fps_depois: a.fps.as_ref().map(|c| c.media_candidato),
            low_antes: a.low_1pct.as_ref().map(|c| c.media_base),
            low_depois: a.low_1pct.as_ref().map(|c| c.media_candidato),
            vigiado: v,
            veredito: a.veredito,
            quando: agora,
            erro,
        });
    }
    for g in estado.governador.iter_mut().filter(|g| g.decidido.is_none()) {
        let a = portao::avaliar_governador(&g.processo, &medicoes);
        if matches!(a.veredito, Veredito::Aguardando { .. }) {
            continue;
        }
        let erro = if a.veredito == Veredito::Desfazer {
            gamemode::reprovar_governador(&g.processo).err()
        } else {
            None
        };
        let d = Decidido {
            fps_antes: a.fps.as_ref().map(|c| c.media_base),
            fps_depois: a.fps.as_ref().map(|c| c.media_candidato),
            low_antes: a.low_1pct.as_ref().map(|c| c.media_base),
            low_depois: a.low_1pct.as_ref().map(|c| c.media_candidato),
            vigiado: g.como_vigiado(),
            veredito: a.veredito,
            quando: agora,
            erro,
        };
        g.decidido = Some(d.clone());
        decididos.push(d);
    }
    estado.vigiados = ficam;
    estado.decididos.extend(decididos.iter().cloned());
    let excesso = estado.decididos.len().saturating_sub(50);
    estado.decididos.drain(..excesso);
    if let Err(e) = portao::gravar(&estado) {
        crate::utils::Logger::warn(&format!("portão: não gravei: {}", e));
    }
    decididos
}

/// Termina uma operação interrompida devolvendo os anteriores do diário, aplicando ou desfazendo: o diário guarda
/// o caminho de volta, não as ações que faltavam. Reverter o já revertido é inofensivo. O histórico é limpo no fim,
/// senão diria aplicada uma otimização desfeita.
#[cfg(target_os = "windows")]
pub fn concluir_recuperacao(
    pendencia: &crate::modules::transacao::Pendencia,
    log: &mut ChangeLog,
) -> Result<usize, String> {
    revert_changes(&pendencia.mudancas)
        .map_err(|erros| format!("não consegui terminar o serviço: {}", erros.join("; ")))?;

    log.take(&pendencia.id)?;
    crate::modules::transacao::descartar()?;

    Ok(pendencia.mudancas.len())
}

/// Desfaz na ordem inversa da aplicação. Tenta todas mesmo se alguma falhar, e devolve as falhas acumuladas.
fn revert_changes(changes: &[ChangeRecord]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    // Desfazer também precisa valer na hora.
    let mut sincronizar_interface = false;

    for change in changes.iter().rev() {
        crate::utils::Logger::info(&format!("desfazendo: {}", change.describe()));

        let result = match change {
            ChangeRecord::RegistryValue {
                hive,
                path,
                name,
                previous,
            } => {
                if sysparams::precisa_sincronizar_interface(hive, path) {
                    sincronizar_interface = true;
                }

                registry::restore(hive, path, name, previous)
            }

            ChangeRecord::ServiceStartType { service, previous } => {
                services::set_start_type(service, previous)
            }

            // Pelo `planoenergia`: confere relendo o plano ativo e apaga o OTIMIZA depois da volta confirmada.
            ChangeRecord::PowerPlan { previous_guid } => planoenergia::desfazer(previous_guid),

            ChangeRecord::Hibernation { previously_enabled } => {
                power::set_hibernation(*previously_enabled)
            }

            ChangeRecord::PowerSetting {
                scheme,
                subgroup,
                setting,
                previous,
                previous_dc,
            } => power::restore_power_setting(
                scheme,
                subgroup,
                setting,
                previous,
                previous_dc.as_ref(),
            ),

            ChangeRecord::MemoryCompression { previously_enabled } => {
                power::set_memory_compression(*previously_enabled)
            }

            ChangeRecord::ReservedStorage { previously_enabled } => {
                power::set_reserved_storage(*previously_enabled)
            }

            ChangeRecord::ScheduledTask {
                path,
                name,
                previously_enabled,
            } => tasks::definir_estado(path, name, *previously_enabled).map(|_| ()),

            ChangeRecord::RefreshRate {
                device,
                previous_hz,
            } => display::aplicar_hz(device, *previous_hz).map(|_| ()),

            ChangeRecord::BootLimits { removed } => {
                let mut failures = Vec::new();

                for (key, value) in removed {
                    if let Err(error) = shell::run_checked("bcdedit", &["/set", "{current}", key, value]) {
                        failures.push(error);
                    }
                }

                if failures.is_empty() {
                    Ok(())
                } else {
                    Err(failures.join("; "))
                }
            }

            // O único ramo que volta por uma chamada do fabricante; o cliente com escolha própria volta escrito
            // (`nvdriver::desfazer` separa).
            ChangeRecord::DriverNvidia {
                opcao,
                valor_anterior,
            } => nvdriver::desfazer(opcao, valor_anterior),

            ChangeRecord::LimiteNvidia {
                executavel,
                perfil_criado,
                valor_anterior,
                ..
            } => nvdriver::desfazer_limite(executavel, *perfil_criado, valor_anterior),

            ChangeRecord::PerfilNvidia { executavel, perfil_criado, anteriores, .. } => {
                nvdriver::desfazer_perfil_do_jogo(executavel, *perfil_criado, anteriores)
            }

            // Sem `anterior` o arquivo não existia: desfazer é apagar, e já não estar lá não é falha.
            ChangeRecord::GameConfig {
                caminho, anterior, ..
            } => match anterior {
                Some(conteudo) => std::fs::write(caminho, conteudo)
                    .map_err(|e| format!("não consegui devolver {}: {}", caminho, e)),
                None => match std::fs::remove_file(caminho) {
                    Ok(()) => Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(format!("não consegui apagar {}: {}", caminho, e)),
                },
            },
        };

        if let Err(error) = result {
            crate::utils::Logger::warn(&format!("não consegui desfazer: {}", error));
            errors.push(error);
        }
    }

    // Uma vez só, depois de restaurar tudo.
    if sincronizar_interface {
        sysparams::sincronizar_interface();
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::changelog::PreviousValue;

    const STARTUP_DELAY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize";

    use ActionState::{Desconhecido, NotApplicable, Pending, Satisfied};

    /// `disable_startup_delay`: HKCU, invisível, instantâneo e reversível. Escreve no registro.
    /// `cargo test --lib resultado_padronizado -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn resultado_padronizado_de_uma_otimizacao_real() {
        use crate::modules::changelog::ChangeLog;

        let otimizador = WindowsOptimizer::new();
        let mut log = ChangeLog::load();
        let id = "disable_startup_delay";

        let aplicar = otimizador.apply(id, &mut log);

        match &aplicar {
            Ok(r) => {
                println!("\n=== {} ===", r.name);
                println!(
                    "sucesso={} aplicada={} reinício={} logoff={} {} ms",
                    r.success, r.applied, r.requires_restart, r.requires_logoff, r.duration_ms
                );

                for a in &r.actions {
                    println!(
                        "  [{:?}] {} | antes={:?} esperado={:?} depois={:?} | saída={:?} | {} ms",
                        a.status,
                        a.name,
                        a.before_value,
                        a.expected_value,
                        a.after_value,
                        a.exit_code,
                        a.duration_ms
                    );

                    if let Some(motivo) = &a.unsupported_reason {
                        println!("      motivo: {}", motivo);
                    }
                }

                assert!(!r.actions.is_empty(), "o resultado veio sem ações");
            }
            Err(e) => println!("FALHOU: {}", e),
        }

        if aplicar.is_ok() {
            let desfazer = otimizador.revert(id, &mut log);
            println!("desfazer: {:?}", desfazer.map(|r| r.message));
        }
    }

    const SEM_GOVERNANCA: Governanca = Governanca {
        com_politica_de_grupo: false,
        imagem_de_terceiros: false,
    };

    /// O catálogo inteiro visto por esta máquina, SÓ LÊ: interessa o que não é `Available`.
    /// `cargo test --lib catalogo_visto_por -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn catalogo_visto_por_esta_maquina() {
        use crate::modules::changelog::ChangeLog;

        let otimizador = WindowsOptimizer::new();
        let log = ChangeLog::load();
        let lista = otimizador.list(&log);

        let mut contagem = std::collections::BTreeMap::new();

        for item in &lista {
            *contagem.entry(format!("{:?}", item.state)).or_insert(0) += 1;
        }

        println!("\n=== {} otimizações ===", lista.len());
        for (estado, quantas) in &contagem {
            println!("{:<16} {}", estado, quantas);
        }

        println!("\n--- o que NÃO está disponível aqui ---");
        for item in &lista {
            if matches!(
                item.state,
                OptimizationState::Available | OptimizationState::Applied
            ) {
                continue;
            }

            println!(
                "[{:?}] {}\n    {}",
                item.state,
                item.name,
                item.detail.as_deref().unwrap_or("(sem detalhe)")
            );
        }
    }

    /// `cargo test --lib governanca_desta -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn governanca_desta_maquina() {
        let g = governanca();
        let checagem = essenciais::checar();

        println!("\npolítica de grupo aplicada .. {}", g.com_politica_de_grupo);
        println!("imagem de terceiros ......... {}", g.imagem_de_terceiros);
        println!("fabricante declarado ........ {:?}", checagem.fabricante);
        println!("modelo declarado ............ {:?}", checagem.modelo);
        println!("essenciais desativados ...... {}", checagem.desativados);

        let mut exemplo = ActionResult {
            name: "serviço de exemplo".to_string(),
            status: refinar_status(ActionStatus::Unsupported, g),
            ..Default::default()
        };

        explicar_governanca(&mut exemplo);

        println!("\num `Unsupported` nesta máquina vira: {:?}", exemplo.status);
        println!("motivo: {:?}", exemplo.unsupported_reason);
    }

    #[test]
    fn sem_politica_na_maquina_nao_se_acusa_politica() {
        assert_eq!(
            refinar_status(ActionStatus::VerificationFailed, SEM_GOVERNANCA),
            ActionStatus::VerificationFailed
        );
    }

    #[test]
    fn com_politica_o_valor_que_nao_ficou_ganha_nome() {
        let com_gpo = Governanca {
            com_politica_de_grupo: true,
            ..SEM_GOVERNANCA
        };

        assert_eq!(
            refinar_status(ActionStatus::VerificationFailed, com_gpo),
            ActionStatus::BlockedByPolicy
        );
    }

    #[test]
    fn imagem_de_terceiros_separa_o_windows_da_instalacao() {
        let com_imagem = Governanca {
            imagem_de_terceiros: true,
            ..SEM_GOVERNANCA
        };

        assert_eq!(
            refinar_status(ActionStatus::Unsupported, com_imagem),
            ActionStatus::UserOrOemManaged
        );
    }

    #[test]
    fn o_refinamento_nunca_mexe_num_estado_conclusivo() {
        let tudo = Governanca {
            com_politica_de_grupo: true,
            imagem_de_terceiros: true,
        };

        for estado in [
            ActionStatus::Verified,
            ActionStatus::AlreadyOptimized,
            ActionStatus::Failed,
            ActionStatus::NotConfirmed,
            ActionStatus::Skipped,
        ] {
            assert_eq!(
                refinar_status(estado, tudo),
                estado,
                "o refinamento mexeu num estado conclusivo: {:?}",
                estado
            );
        }
    }

    #[test]
    fn os_sete_termos_do_protocolo_existem_no_motor() {
        // O motor e o lab usam o MESMO vocabulário. `partial` é do conjunto e `requires restart/logoff` é campo (aplicada
        // E exige reinício).
        let por_acao = [
            ActionStatus::Verified,
            ActionStatus::Unsupported,
            ActionStatus::BlockedByPolicy,
            ActionStatus::Failed,
            ActionStatus::UserOrOemManaged,
        ];

        assert_eq!(por_acao.len(), 5);
        assert!(ActionStatus::UserOrOemManaged.deu_certo());
        assert!(!ActionStatus::BlockedByPolicy.deu_certo());
    }

    #[test]
    fn ajuste_que_este_windows_nao_tem_nao_e_vermelho() {
        assert!(ActionStatus::Unsupported.deu_certo());
        assert!(ActionStatus::AlreadyOptimized.deu_certo());
        assert!(ActionStatus::Verified.deu_certo());
        assert!(ActionStatus::NotConfirmed.deu_certo());
        assert!(ActionStatus::Skipped.deu_certo());

        assert!(!ActionStatus::Failed.deu_certo());
        assert!(!ActionStatus::VerificationFailed.deu_certo());
    }

    #[test]
    fn o_nome_da_acao_e_para_uma_pessoa_ler() {
        let nome = nome_da_acao(&Action::Registry {
            hive: "HKLM",
            path: r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            name: "HwSchMode",
            value: RegValue::Dword(2),
        });

        assert!(nome.contains("HwSchMode"), "{}", nome);
        assert!(!nome.contains("RegValue"), "{}", nome);
        assert!(!nome.contains("CurrentControlSet"), "{}", nome);

        assert_eq!(
            nome_da_acao(&Action::DisableService { name: "SysMain" }),
            "serviço SysMain"
        );
    }

    #[test]
    fn todo_item_do_catalogo_tem_nome_legivel_em_cada_acao() {
        for spec in catalog::CATALOG {
            for action in spec.actions {
                let nome = nome_da_acao(action);

                assert!(
                    nome.len() > 4 && !nome.contains('{'),
                    "a ação de `{}` não tem nome legível: `{}`",
                    spec.id,
                    nome
                );
            }
        }
    }

    #[test]
    fn o_logoff_e_derivado_e_nao_declarado() {
        let com_logoff: Vec<&str> = catalog::CATALOG
            .iter()
            .filter(|spec| exige_logoff(spec))
            .map(|spec| spec.id)
            .collect();

        assert!(
            !com_logoff.is_empty(),
            "nenhum item do catálogo exige logoff — ou a derivação quebrou, ou \
             as chaves do `sysparams::nota_de_ativacao` saíram do catálogo"
        );
    }

    #[test]
    fn antes_e_esperado_sao_impressos_do_mesmo_jeito() {
        assert_eq!(
            descrever_alvo(&RegValue::Dword(2)),
            descrever_valor(&PreviousValue::Dword(2))
        );
        assert_eq!(
            descrever_alvo(&RegValue::Text("0".into())),
            descrever_valor(&PreviousValue::Text("0".into()))
        );
    }

    #[test]
    fn conferir_separa_diferente_de_nao_lido() {
        assert_eq!(conferir(&true, Some(true)), Confirmacao::Igual);
        assert_eq!(
            conferir(&"disabled".to_string(), Some("auto".to_string())),
            Confirmacao::Diferente("\"auto\"".to_string())
        );
        assert_eq!(conferir(&true, None::<bool>), Confirmacao::NaoDeuParaLer);
    }

    #[test]
    fn a_regra_da_confirmacao_e_uma_so() {
        assert_eq!(exigir_confirmacao(Confirmacao::Igual, "o serviço X"), Ok(None));

        let falhou = exigir_confirmacao(Confirmacao::Diferente("\"auto\"".into()), "o serviço X");
        assert!(falhou.is_err());
        assert!(falhou.unwrap_err().contains("Nada ficou aplicado"));

        let ressalva = exigir_confirmacao(Confirmacao::NaoDeuParaLer, "o serviço X");
        assert!(ressalva.clone().is_ok());
        assert!(ressalva.unwrap().unwrap().contains("não foi possível reler"));
    }

    #[test]
    fn nenhuma_acao_que_escreve_devolve_sucesso_sem_conferir() {
        // Trava de FORMA: ramos do `execute` que terminam em `Ok(None)` logo depois de empilhar no histórico, sem reler.
        let fonte = include_str!("mod.rs");

        let Some(corpo) = fonte.split("fn execute(").nth(1) else {
            panic!("não achei o `execute` para varrer");
        };

        let corpo = corpo.split("\n    fn ").next().unwrap_or(corpo);

        let mut suspeitos = Vec::new();
        let linhas: Vec<&str> = corpo.lines().collect();

        for (i, linha) in linhas.iter().enumerate() {
            if !linha.trim().starts_with("changes.push(") {
                continue;
            }

            // Para os dois lados: conferir ANTES de empilhar é válido (e o único correto no plano de energia). A janela
            // aproxima o ramo do `match`.
            let inicio = i.saturating_sub(30);
            let janela = linhas[inicio..]
                .iter()
                .take(70)
                .map(|l| l.trim())
                .collect::<Vec<_>>()
                .join(" ");

            let confere = janela.contains("exigir_confirmacao")
                || janela.contains("conferir_escrita")
                || janela.contains("forced_platform_clock()")
                || janela.contains("plano_ativo");

            if !confere {
                suspeitos.push(linha.trim().to_string());
            }
        }

        assert!(
            suspeitos.is_empty(),
            "há escrita(s) no `execute` que não releem o estado depois de gravar. \
             Toda mudança precisa ser conferida — ver `exigir_confirmacao`.\n{}",
            suspeitos.join("\n")
        );
    }

    #[test]
    fn escrita_conferida_e_igual_esta_provada() {
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Ok(PreviousValue::Dword(2))),
            Confirmacao::Igual
        );
        assert_eq!(
            conferir_escrita(
                &RegValue::Text("0".into()),
                &Ok(PreviousValue::Text("0".into()))
            ),
            Confirmacao::Igual
        );
        assert_eq!(
            conferir_escrita(
                &RegValue::Binary(&[3, 0, 0, 0]),
                &Ok(PreviousValue::Binary(vec![3, 0, 0, 0]))
            ),
            Confirmacao::Igual
        );
    }

    #[test]
    fn o_windows_aceitar_nao_e_o_valor_ter_ficado() {
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Ok(PreviousValue::Dword(1))),
            Confirmacao::Diferente("1".to_string())
        );
    }

    #[test]
    fn valor_que_sumiu_depois_da_escrita_nao_passa() {
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Ok(PreviousValue::Absent)),
            Confirmacao::Diferente("inexistente".to_string())
        );
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Ok(PreviousValue::AbsentKey)),
            Confirmacao::Diferente("inexistente (a chave sumiu)".to_string())
        );
    }

    #[test]
    fn tipo_diferente_e_valor_diferente() {
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Ok(PreviousValue::Text("2".into()))),
            Confirmacao::Diferente("\"2\"".to_string())
        );
    }

    #[test]
    fn nao_conseguir_reler_nao_e_falha_nem_confirmacao() {
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Err("acesso negado".into())),
            Confirmacao::NaoDeuParaLer
        );
    }

    /// Contra o REGISTRO DE VERDADE: o tipo gravado por `set_dword` precisa ser o que `read` devolve, senão toda
    /// otimização de registro reprovaria. Chave de rascunho apagada no fim.
    /// `cargo test --lib conferencia_contra_o_registro -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn conferencia_contra_o_registro_de_verdade() {
        const CAMINHO: &str = r"Software\Otimiza\rascunho-de-teste";

        for (alvo, rotulo) in [
            (RegValue::Dword(2), "dword"),
            (RegValue::Text("0".into()), "texto"),
            (RegValue::Binary(&[3, 0, 0, 0]), "binário"),
        ] {
            let anterior = match &alvo {
                RegValue::Dword(v) => registry::set_dword("HKCU", CAMINHO, "valor", *v),
                RegValue::Text(v) => registry::set_string("HKCU", CAMINHO, "valor", v),
                RegValue::Binary(v) => registry::set_binary("HKCU", CAMINHO, "valor", v),
            }
            .expect("gravar no rascunho");

            let conferido = conferir_escrita(&alvo, &registry::read("HKCU", CAMINHO, "valor"));

            println!("{:<8} -> {:?}", rotulo, conferido);

            registry::restore("HKCU", CAMINHO, "valor", &anterior).expect("limpar o rascunho");

            assert_eq!(
                conferido,
                Confirmacao::Igual,
                "o tipo gravado e o tipo lido não batem para {}",
                rotulo
            );
        }
    }

    #[test]
    fn leitura_que_falhou_nao_vira_nao_se_aplica() {
        assert_eq!(
            WindowsOptimizer::compor(&[Desconhecido]),
            OptimizationState::Unknown
        );
    }

    #[test]
    fn leitura_que_falhou_nao_vira_ja_esta_bom() {
        assert_eq!(
            WindowsOptimizer::compor(&[Satisfied, Desconhecido]),
            OptimizationState::Unknown
        );
    }

    #[test]
    fn saber_que_ha_o_que_fazer_vence_o_desconhecimento() {
        assert_eq!(
            WindowsOptimizer::compor(&[Pending, Desconhecido]),
            OptimizationState::Available
        );
        assert_eq!(
            WindowsOptimizer::compor(&[Desconhecido, Pending, Satisfied]),
            OptimizationState::Available
        );
    }

    #[test]
    fn os_estados_conhecidos_continuam_como_eram() {
        assert_eq!(
            WindowsOptimizer::compor(&[NotApplicable, NotApplicable]),
            OptimizationState::Unavailable
        );
        assert_eq!(
            WindowsOptimizer::compor(&[Satisfied, NotApplicable]),
            OptimizationState::AlreadyOptimal
        );
        assert_eq!(
            WindowsOptimizer::compor(&[Satisfied]),
            OptimizationState::AlreadyOptimal
        );
        assert_eq!(
            WindowsOptimizer::compor(&[Pending, Satisfied]),
            OptimizationState::Available
        );
    }

    #[test]
    fn o_lote_automatico_nunca_leva_o_que_nao_foi_conferido() {
        // Afrouxar o filtro para `Unknown` aplicaria, sem revisão, o que não se leu.
        for estados in [
            vec![Desconhecido],
            vec![Satisfied, Desconhecido],
            vec![Desconhecido, NotApplicable],
        ] {
            assert_ne!(
                WindowsOptimizer::compor(&estados),
                OptimizationState::Available,
                "estado desconhecido não pode entrar no lote automático"
            );
        }
    }

    /// Nenhum teste pode escrever no driver: um ajuste inexistente é recusado pelo nome antes da sessão, e o erro sobe
    /// por este ramo. Pega o ramo que devolvesse `Ok(())` sem chamar nada.
    #[test]
    fn desfazer_um_ajuste_de_driver_inexistente_reclama_em_vez_de_fingir() {
        let registro = ChangeRecord::DriverNvidia {
            opcao: "ajuste-que-nao-existe".to_string(),
            valor_anterior: nvdriver::ANTERIOR_PADRAO.to_string(),
        };

        let erros = revert_changes(&[registro])
            .expect_err("um ajuste desconhecido não pode passar por desfeito");

        assert_eq!(erros.len(), 1);
        assert!(
            erros[0].contains("ajuste-que-nao-existe"),
            "o erro precisa dizer QUAL ajuste: {}",
            erros[0]
        );
    }

    /// O arquivo do jogo volta BYTE A BYTE.
    #[test]
    fn desfazer_a_configuracao_do_jogo_devolve_o_arquivo_inteiro() {
        let dir = std::env::temp_dir().join(format!("otz-conf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("criar pasta de teste");
        let alvo = dir.join("settings.xml");

        let original = "<Settings>\r\n  <MSAA value=\"4\" />\r\n  <!-- resolução -->\r\n</Settings>\r\n";
        std::fs::write(&alvo, original).expect("escrever o original");

        let registro = ChangeRecord::GameConfig {
            caminho: alvo.to_string_lossy().to_string(),
            anterior: Some(original.to_string()),
            jogo: "FiveM".to_string(),
        };

        std::fs::write(&alvo, "<Settings><MSAA value=\"0\" /></Settings>").expect("alterar");
        assert_ne!(std::fs::read_to_string(&alvo).unwrap(), original);

        revert_changes(&[registro]).expect("a reversão não podia falhar");

        assert_eq!(
            std::fs::read_to_string(&alvo).unwrap(),
            original,
            "o arquivo voltou diferente do que era"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `revert_all` pode passar pelo mesmo registro depois de uma reversão parcial que já limpou o arquivo.
    #[test]
    fn desfazer_apaga_o_arquivo_que_o_otimiza_criou() {
        let dir = std::env::temp_dir().join(format!("otz-conf-novo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("criar pasta de teste");
        let alvo = dir.join("settings.xml");

        std::fs::write(&alvo, "<Settings/>").expect("criar o arquivo");

        let registro = ChangeRecord::GameConfig {
            caminho: alvo.to_string_lossy().to_string(),
            anterior: None,
            jogo: "FiveM".to_string(),
        };

        revert_changes(std::slice::from_ref(&registro)).expect("a reversão não podia falhar");
        assert!(!alvo.exists(), "o arquivo continuou existindo");

        revert_changes(std::slice::from_ref(&registro)).expect("desfazer duas vezes não é falha");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn inspects_every_optimization_against_this_machine() {
        let optimizer = WindowsOptimizer::new();
        let log = ChangeLog::load();

        for info in optimizer.list(&log) {
            println!("{:<45} {:?} {:?}", info.name, info.state, info.detail);
        }

        let esperado = catalog::CATALOG
            .iter()
            .filter(|spec| !catalog::retirado(spec.id) || log.is_applied(spec.id))
            .filter(|spec| match catalog::classe(spec.id) {
                catalog::Classe::Condicional(c) => condicao_atendida(c) != Some(false) || log.is_applied(spec.id),
                _ => true,
            })
            .count();
        assert_eq!(optimizer.list(&log).len(), esperado);
    }

    #[test]
    fn o_game_dvr_ligado_e_lido_como_o_windows_decide() {
        use PreviousValue::*;
        assert_eq!(gamedvr_ligado(Some(Absent), Some(Absent)), Some(true));
        assert_eq!(gamedvr_ligado(Some(Dword(0)), Some(Dword(0))), Some(false));
        assert_eq!(gamedvr_ligado(Some(Dword(0)), Some(Absent)), Some(false));
        assert_eq!(gamedvr_ligado(Some(Dword(0)), Some(Dword(1))), Some(true));
        assert_eq!(gamedvr_ligado(None, Some(Dword(0))), None);
    }

    #[test]
    fn item_retirado_nao_pode_ser_aplicado_nem_entrar_em_lote() {
        let optimizer = WindowsOptimizer::new();
        let mut log = ChangeLog::em_memoria();
        for id in catalog::RETIRADOS {
            let spec = catalog::find(id).expect("retirado continua no catálogo para o desfazer");
            assert!(!catalog::entra_no_lote(spec), "`{}` entrou no lote", id);
            assert!(optimizer.apply(id, &mut log).is_err(), "`{}` foi aplicado", id);
        }
    }

    #[test]
    fn lote_nunca_inclui_operacao_sem_volta() {
        let optimizer = WindowsOptimizer::new();
        let log = ChangeLog::load();

        let batch: Vec<&str> = catalog::CATALOG
            .iter()
            .filter(|spec| spec.reversible)
            .filter(|spec| optimizer.inspect(spec, &log) == OptimizationState::Available)
            .map(|spec| spec.id)
            .collect();

        assert!(batch.iter().all(|id| catalog::find(id).is_some_and(|s| s.reversible)));
    }

    #[test]
    fn nao_afirma_estar_otimizado_o_que_nao_conseguiu_conferir() {
        if registry::is_elevated() {
            return;
        }

        let optimizer = WindowsOptimizer::new();
        let log = ChangeLog::load();

        for id in ["remove_forced_hpet", "clear_boot_limits"] {
            let spec = catalog::find(id).expect("otimização deveria existir");

            if log.is_applied(id) {
                continue;
            }

            assert_ne!(
                optimizer.inspect(spec, &log),
                OptimizationState::AlreadyOptimal,
                "{} afirma estar otimizado sem ter conseguido conferir",
                id
            );

            let detalhe = optimizer.detail(spec).unwrap_or_default();
            assert!(
                detalhe.contains("administrador"),
                "{} não explica que a conferência exige administrador",
                id
            );
        }
    }

    #[test]
    fn lote_nunca_troca_seguranca_por_desempenho() {
        let optimizer = WindowsOptimizer::new();
        let log = ChangeLog::load();

        let batch: Vec<&str> = catalog::CATALOG
            .iter()
            .filter(|spec| spec.reversible)
            .filter(|spec| !spec.security_tradeoff)
            .filter(|spec| optimizer.inspect(spec, &log) == OptimizationState::Available)
            .map(|spec| spec.id)
            .collect();

        for spec in catalog::CATALOG.iter().filter(|spec| spec.security_tradeoff) {
            assert!(
                !batch.contains(&spec.id),
                "{} reduz segurança e não pode entrar no lote automático",
                spec.id
            );
        }
    }

    #[test]
    fn security_tradeoffs_warn_loudly() {
        for spec in catalog::CATALOG.iter().filter(|spec| spec.security_tradeoff) {
            assert!(
                spec.honest_effect.contains("SEGURANÇA")
                    || spec.honest_effect.contains("ATENÇÃO"),
                "{} não avisa que reduz segurança",
                spec.id
            );
        }
    }

    /// Byte-exato: o Windows guarda a data do desligamento nos bytes 4 a 11.
    /// `cargo test --lib -- --ignored --nocapture real_startup_cycle`
    #[test]
    #[ignore]
    fn real_startup_cycle_restores_exact_bytes() {
        const APPROVED: &str =
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

        let optimizer = WindowsOptimizer::new();
        let mut log = ChangeLog::load();

        let entry = startup::entries()
            .expect("as chaves de inicialização desta máquina precisam ser legíveis")
            .into_iter()
            .find(|entry| entry.hive == "HKCU")
            .expect("nenhuma entrada de inicialização em HKCU");

        println!("alvo: {} (ligado: {})", entry.name, entry.enabled);

        let original = registry::read("HKCU", APPROVED, &entry.name).unwrap();
        println!("bytes originais: {:?}", original);

        optimizer
            .set_startup("HKCU", &entry.name, false, &mut log)
            .expect("desligar falhou");

        match registry::read("HKCU", APPROVED, &entry.name).unwrap() {
            PreviousValue::Binary(bytes) => assert_eq!(
                bytes.first(),
                Some(&3u8),
                "o Windows não vai considerar isto desligado"
            ),
            other => panic!("esperado valor binário, veio {:?}", other),
        }

        optimizer
            .set_startup("HKCU", &entry.name, true, &mut log)
            .expect("religar falhou");

        let restored = registry::read("HKCU", APPROVED, &entry.name).unwrap();
        assert_eq!(restored, original, "os bytes não voltaram idênticos");
        assert!(!log.is_applied(&startup_change_id("HKCU", &entry.name)));

        println!("bytes após o ciclo: {:?}", restored);
    }

    /// Exige elevação: `cargo test --lib -- --ignored --nocapture real_admin_optimizations`
    #[test]
    #[ignore]
    fn real_admin_optimizations_apply_and_revert() {
        assert!(
            registry::is_elevated(),
            "este teste precisa de uma sessão como administrador"
        );

        let optimizer = WindowsOptimizer::new();
        let mut log = ChangeLog::load();

        // `AlreadyOptimal` fica de fora: testá-la desconfiguraria a máquina de quem roda.
        let alvos: Vec<&OptimizationSpec> = catalog::CATALOG
            .iter()
            .filter(|spec| spec.requires_admin && spec.reversible && !spec.security_tradeoff)
            .filter(|spec| {
                matches!(
                    optimizer.inspect(spec, &log),
                    OptimizationState::Available | OptimizationState::Applied
                )
            })
            .collect();

        println!("otimizações a testar: {}", alvos.len());
        let mut testadas = 0;
        let mut recusadas = 0;

        for spec in alvos {
            let estado_inicial = optimizer.inspect(spec, &log);

            let (primeiro, segundo, esperado_no_meio) = match estado_inicial {
                OptimizationState::Applied => ("desfazer", "aplicar", OptimizationState::Available),
                _ => ("aplicar", "desfazer", OptimizationState::Applied),
            };

            let executar = |acao: &str, log: &mut ChangeLog| match acao {
                "aplicar" => optimizer.apply(spec.id, log),
                _ => optimizer.revert(spec.id, log),
            };

            let meio = match executar(primeiro, &mut log) {
                Ok(resultado) => resultado,

                // Recusa honesta não é falha do ciclo (aqui: Armazenamento Reservado e política dos Widgets negados mesmo
                // elevado). Por categoria, não por lista de ids: se recusou, precisa dizer que nada foi alterado.
                Err(erro) if e_recusa_honesta(&erro) => {
                    println!("  {} → recusado sem alterar nada: {}", spec.id, erro);
                    recusadas += 1;
                    continue;
                }

                Err(erro) => panic!("{} `{}` falhou: {}", primeiro, spec.id, erro),
            };

            println!(
                "  {} → {} ({} mudança(s)){}",
                spec.id,
                primeiro,
                meio.changes_count,
                if meio.changes.is_empty() {
                    String::new()
                } else {
                    format!(": {}", meio.changes.join(" | "))
                }
            );

            let estado_meio = optimizer.inspect(spec, &log);
            assert_eq!(
                estado_meio, esperado_no_meio,
                "{} ficou em {:?} depois de {}",
                spec.id, estado_meio, primeiro
            );

            executar(segundo, &mut log)
                .unwrap_or_else(|erro| panic!("{} `{}` falhou: {}", segundo, spec.id, erro));

            assert_eq!(
                optimizer.inspect(spec, &log),
                estado_inicial,
                "{} não voltou ao estado em que estava antes do teste",
                spec.id
            );

            println!("  {} → {}, sistema no estado original", spec.id, segundo);
            testadas += 1;
        }

        assert!(
            testadas > 0,
            "nenhuma otimização de administrador estava disponível para testar"
        );
        println!(
            "ciclo completo em {} otimização(ões); {} recusadas sem alterar nada",
            testadas, recusadas
        );
    }

    /// O CONTRATO da recusa, testável: um `contains` solto aceitaria falha de verdade no dia de uma frase errada.
    fn e_recusa_honesta(erro: &str) -> bool {
        erro.to_lowercase().contains("nada foi alterado")
    }

    #[test]
    fn recusa_honesta_exige_dizer_que_nada_mudou() {
        assert!(e_recusa_honesta(
            "O Windows recusou informar se há espaço reservado, mesmo com o Otimiza como \
             administrador. Sem saber o estado atual não há como desfazer depois, então nada \
             foi alterado."
        ));

        assert!(e_recusa_honesta(
            "O Windows negou a escrita em HKLM\\SOFTWARE\\Policies\\Microsoft\\Dsh mesmo com o \
             Otimiza aberto como administrador. Isso normalmente é antivírus ou uma proteção de \
             política bloqueando a alteração — não é falta de permissão sua. Nada foi alterado."
        ));
    }

    #[test]
    fn falha_no_meio_nao_passa_por_recusa() {
        assert!(!e_recusa_honesta("Falha ao reverter `X`: acesso negado"));
        assert!(!e_recusa_honesta("Este ajuste não existe neste Windows"));
        assert!(!e_recusa_honesta(""));
    }

    /// ~20 s. `cargo test --release --lib -- --ignored --nocapture real_full_cycle`
    #[test]
    #[ignore]
    fn real_full_cycle_with_measurement() {
        use crate::modules::benchmark::{compare, Benchmark};

        let optimizer = WindowsOptimizer::new();
        let mut log = ChangeLog::load();
        let id = "disable_startup_delay";

        let before = Benchmark::new().run();
        println!("ANTES:  {:?}", before);

        let applied = optimizer.apply(id, &mut log).expect("aplicar falhou");
        println!("APLICADO: {}", applied.message);

        let after = Benchmark::new().run();
        println!("DEPOIS: {:?}", after);

        let comparison = compare(&before, &after);
        println!("\n=== RESUMO: {} ===", comparison.summary);
        for metric in &comparison.metrics {
            println!(
                "{:<38} {:>10.1} -> {:>10.1} {:<7} {:>7.1}%  {:?}",
                metric.label,
                metric.before,
                metric.after,
                metric.unit,
                metric.change_percent,
                metric.verdict
            );
        }

        optimizer.revert(id, &mut log).expect("desfazer falhou");
        assert!(!log.is_applied(id));
    }

    /// `disable_startup_delay`, HKCU e sem administrador.
    /// `cargo test --lib -- --ignored --nocapture real_apply_and_revert`
    #[test]
    #[ignore]
    fn real_apply_and_revert_cycle_restores_the_system() {
        let optimizer = WindowsOptimizer::new();
        let mut log = ChangeLog::load();
        let id = "disable_startup_delay";

        let original = registry::read("HKCU", STARTUP_DELAY_PATH, "StartupDelayInMSec")
            .expect("leitura inicial falhou");
        println!("estado original: {:?}", original);

        let applied = optimizer.apply(id, &mut log).expect("aplicar falhou");
        println!("aplicado: {} ({} mudanças)", applied.message, applied.changes_count);

        assert!(applied.applied);
        assert!(log.is_applied(id), "otimização não foi registrada no histórico");
        assert_eq!(
            registry::read("HKCU", STARTUP_DELAY_PATH, "StartupDelayInMSec").unwrap(),
            PreviousValue::Dword(0),
            "o valor não foi realmente escrito no registro"
        );

        let reverted = optimizer.revert(id, &mut log).expect("desfazer falhou");
        println!("revertido: {}", reverted.message);

        assert!(!reverted.applied);
        assert!(!log.is_applied(id), "histórico ainda marca a otimização como aplicada");
        assert_eq!(
            registry::read("HKCU", STARTUP_DELAY_PATH, "StartupDelayInMSec").unwrap(),
            original,
            "o registro não voltou ao estado original"
        );
    }
}

fn success_message(spec: &OptimizationSpec, notes: &[String]) -> String {
    let mut message = format!("`{}` aplicada.", spec.name);

    if !notes.is_empty() {
        message.push(' ');
        message.push_str(&notes.join(" "));
    }

    if spec.requires_restart {
        message.push_str(" Reinicie o PC para o efeito valer.");
    }

    message
}

#[cfg(test)]
mod custo_das_condicoes {
    /// `cargo test --lib -- --ignored quanto_custa_listar --nocapture`
    #[test]
    #[ignore]
    fn quanto_custa_listar() {
        use std::time::Instant;
        for k in 0..4 {
            let u = Instant::now();
            let v = super::condicao_atendida(super::catalog::Condicao::PoucoEspaco);
            println!("  chamada {k}: {:?} = {v:?}", u.elapsed());
        }
        let t = Instant::now();
        let log = super::ChangeLog::load();
        let n = super::WindowsOptimizer::new().list(&log).len();
        println!("list() com {n} itens: {:?}", t.elapsed());
    }
}
