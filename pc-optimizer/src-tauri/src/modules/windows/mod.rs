// Otimizador do Windows
//
// Aplica e desfaz as otimizações do catálogo. Duas regras governam este módulo:
//
// 1. Nenhuma mudança é feita sem antes gravar o estado anterior no ChangeLog.
// 2. Se uma ação falhar no meio de uma otimização, as ações já aplicadas são
//    desfeitas antes de reportar o erro — o sistema nunca fica pela metade.

pub mod bloatware;
pub mod boot;
pub mod browsers;
pub mod catalog;
pub mod cleanup;
pub mod conflicts;
pub mod devices;
pub mod diskspace;
pub mod firmware;
pub mod fivem;
pub mod foldermap;
pub mod frames;
pub mod hardware;
pub mod health;
pub mod memory;
pub mod network;
pub mod power;
pub mod processes;
pub mod profiles;
pub mod registry;
pub mod restore;
pub mod services;
pub mod servicesaudit;
pub mod shell;
pub mod startup;
pub mod tasks;
pub mod thermal;

use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeLog, ChangeRecord, PreviousValue};
use crate::modules::optimizer::{BatchStep, OptimizationInfo, OptimizationOutcome, OptimizationState};
use crate::modules::safety::SafetyValidator;
use catalog::{Action, OptimizationSpec, RegValue};

/// Caminho das interfaces de rede. Cada subchave é o GUID de um adaptador.
const TCPIP_INTERFACES: &str = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";

/// Situação de uma ação isolada dentro de uma otimização.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionState {
    /// O sistema já está no estado desejado.
    Satisfied,
    /// Precisa ser aplicada.
    Pending,
    /// Não faz sentido nesta máquina (serviço inexistente, chave sem suporte).
    NotApplicable,
}

pub struct WindowsOptimizer;

impl WindowsOptimizer {
    pub fn new() -> Self {
        WindowsOptimizer
    }

    /// Lista o catálogo com a situação real de cada otimização nesta máquina.
    pub fn list(&self, log: &ChangeLog) -> Vec<OptimizationInfo> {
        catalog::CATALOG
            .iter()
            .map(|spec| {
                let state = self.inspect(spec, log);
                spec.to_info(state, self.detail(spec), pesa_nesta_maquina(spec))
            })
            .collect()
    }

    /// Descobre a situação de uma otimização olhando o sistema, não um arquivo.
    ///
    /// Sem isso o programa ofereceria "otimizar" coisas que o PC já tem — o
    /// truque clássico de quem cobra por serviço que não executou.
    fn inspect(&self, spec: &OptimizationSpec, log: &ChangeLog) -> OptimizationState {
        if log.is_applied(spec.id) {
            return OptimizationState::Applied;
        }

        // A máquina decide antes do catálogo: uma otimização que faria mal a
        // este hardware nem chega a ser oferecida.
        if !meets_requirement(spec) {
            return OptimizationState::Unavailable;
        }

        let states: Vec<ActionState> = spec.actions.iter().map(|a| self.inspect_action(a)).collect();

        // Se nenhuma ação pode rodar aqui, a otimização não serve para esta máquina.
        if states.iter().all(|s| *s == ActionState::NotApplicable) {
            return OptimizationState::Unavailable;
        }

        // Já satisfeita em todo lugar que se aplica: o PC já estava assim.
        if states
            .iter()
            .all(|s| matches!(s, ActionState::Satisfied | ActionState::NotApplicable))
        {
            return OptimizationState::AlreadyOptimal;
        }

        OptimizationState::Available
    }

    /// Informação medida agora, quando a otimização tem um número ou um motivo
    /// concreto a mostrar sobre ESTA máquina.
    fn detail(&self, spec: &OptimizationSpec) -> Option<String> {
        if let Some(requirement) = spec.requirement {
            if !meets_requirement(spec) {
                return Some(requirement.unmet_reason().to_string());
            }
        }

        match spec.actions.first()? {
            Action::CleanTempFiles => {
                let bytes = cleanup::estimate();
                Some(format!("{} para liberar", cleanup::format_size(bytes)))
            }
            Action::CleanUpdateCache => {
                let bytes = cleanup::estimate_update_cache();
                Some(format!("{} para liberar", cleanup::format_size(bytes)))
            }

            // Sem elevação não conseguimos sequer LER estas configurações. Dizer
            // isso é obrigatório: o usuário precisa saber que o item aparece
            // como disponível porque não foi possível conferir, não porque
            // sabemos que falta aplicar.
            Action::ReservedStorage { .. }
            | Action::RemoveForcedPlatformClock
            | Action::ClearBootLimits
                if !registry::is_elevated() =>
            {
                Some("Só dá para conferir o estado atual como administrador.".to_string())
            }

            _ => None,
        }
    }

    /// Verifica se uma ação já está satisfeita, sem alterar nada.
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
                    RegValue::Text(v) => PreviousValue::Text(v.to_string()),
                };

                match current {
                    Ok(current) if current == target => ActionState::Satisfied,
                    Ok(_) => ActionState::Pending,
                    Err(_) => ActionState::NotApplicable,
                }
            }

            Action::DisableService { name } => {
                if !services::exists(name) {
                    return ActionState::NotApplicable;
                }

                match services::query_start_type(name) {
                    Ok(start_type) if start_type == "disabled" => ActionState::Satisfied,
                    Ok(_) => ActionState::Pending,
                    Err(_) => ActionState::NotApplicable,
                }
            }

            Action::HighPerformancePowerPlan => match power::active_scheme() {
                Ok(guid) if guid == power::HIGH_PERFORMANCE_GUID => ActionState::Satisfied,
                Ok(_) => ActionState::Pending,
                Err(_) => ActionState::NotApplicable,
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
                _ => ActionState::NotApplicable,
            },

            Action::DisableHibernation => {
                if power::hibernation_enabled() {
                    ActionState::Pending
                } else {
                    ActionState::Satisfied
                }
            }

            Action::PowerSetting {
                subgroup,
                setting,
                value,
            } => match power::active_scheme() {
                Ok(scheme) => match power::read_power_setting(&scheme, subgroup, setting) {
                    Ok(PreviousValue::Dword(current)) if current == *value => ActionState::Satisfied,
                    Ok(_) => ActionState::Pending,
                    Err(_) => ActionState::NotApplicable,
                },
                Err(_) => ActionState::NotApplicable,
            },

            Action::MemoryCompression { enabled } => {
                // A condição de RAM já foi checada em `meets_requirement`; aqui
                // só resta comparar o estado atual com o desejado.
                match power::memory_compression_enabled() {
                    Some(current) if current == *enabled => ActionState::Satisfied,
                    Some(_) => ActionState::Pending,
                    None => ActionState::NotApplicable,
                }
            }

            // As três verificações abaixo dependem de comandos que o Windows só
            // responde com elevação. Sem ela, a leitura volta vazia — e concluir
            // "está tudo certo" a partir de uma leitura que não aconteceu seria
            // afirmar o que não foi verificado. Nesses casos oferecemos o item e
            // dizemos, no detalhe, que a conferência exige administrador.
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
                // Sem placa reconhecida, não há o que ajustar — e chutar qual
                // dispositivo é a GPU seria mexer em interrupção alheia.
                None => ActionState::NotApplicable,
            },

            Action::NicPowerSaving => match devices::economia_de_energia_da_rede_desligada() {
                Some(true) => ActionState::Satisfied,
                Some(false) => ActionState::Pending,
                None => ActionState::NotApplicable,
            },

            // Só faz sentido oferecer a limpeza se houver algo a limpar.
            Action::CleanTempFiles => {
                if cleanup::estimate() > 0 {
                    ActionState::Pending
                } else {
                    ActionState::Satisfied
                }
            }

            Action::CleanUpdateCache => {
                if cleanup::estimate_update_cache() > 0 {
                    ActionState::Pending
                } else {
                    ActionState::Satisfied
                }
            }

            Action::ReservedStorage { enabled } => {
                if !registry::is_elevated() {
                    return ActionState::Pending;
                }

                match power::reserved_storage_enabled() {
                    Some(current) if current == *enabled => ActionState::Satisfied,
                    Some(_) => ActionState::Pending,
                    // Elevados e ainda sem resposta: este Windows não tem o recurso.
                    None => ActionState::NotApplicable,
                }
            }

            // Só aparece como disponível se alguém realmente forçou o relógio.
            // Num PC saudável esta linha nunca é oferecida.
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

    /// Aplica uma otimização, registrando tudo o que for alterado.
    pub fn apply(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        let spec = catalog::find(id).ok_or_else(|| format!("Unknown optimization: {}", id))?;

        if log.is_applied(id) {
            return Ok(OptimizationOutcome {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                success: true,
                applied: true,
                message: "Otimização já estava aplicada.".to_string(),
                requires_restart: false,
                changes_count: 0,
                changes: Vec::new(),
            });
        }

        if spec.requires_admin && !registry::is_elevated() {
            return Err(format!(
                "`{}` exige executar o programa como administrador.",
                spec.name
            ));
        }

        let mut changes: Vec<ChangeRecord> = Vec::new();
        let mut notes: Vec<String> = Vec::new();

        for action in spec.actions {
            match self.execute(action, &mut changes) {
                Ok(Some(note)) => notes.push(note),
                Ok(None) => {}
                Err(error) => {
                    // Desfaz o que já foi aplicado para não deixar o sistema num estado misto.
                    // Se a própria reversão falhar, isso precisa ir para o log: é a
                    // única situação em que o PC pode ficar num estado intermediário.
                    if let Err(failures) = revert_changes(&changes) {
                        crate::utils::Logger::error(&format!(
                            "reversão parcial de `{}` falhou: {}",
                            spec.id,
                            failures.join("; ")
                        ));
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

        Ok(OptimizationOutcome {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            success: true,
            applied: true,
            message: success_message(spec, &notes),
            requires_restart: spec.requires_restart,
            changes_count,
            changes: described,
        })
    }

    /// Desfaz uma otimização, restaurando cada valor ao estado anterior.
    ///
    /// Funciona também para o que não está no catálogo — como as entradas de
    /// inicialização desligadas pelo usuário. O histórico guarda o suficiente para
    /// reverter qualquer coisa que a gente tenha mexido, e é ele quem manda aqui.
    pub fn revert(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
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
                    requires_restart: false,
                    changes_count: 0,
                    changes: Vec::new(),
                });
            }
        };

        let name = spec
            .map(|s| s.name.to_string())
            .unwrap_or_else(|| entry.name.clone());
        let requires_restart = spec.map(|s| s.requires_restart).unwrap_or(false);

        let changes_count = entry.changes.len();
        let described: Vec<String> = entry.changes.iter().map(|change| change.describe()).collect();

        if let Err(errors) = revert_changes(&entry.changes) {
            // A reversão falhou: o registro volta ao histórico para que o usuário
            // possa tentar de novo em vez de perder o estado original.
            log.record(entry)?;
            return Err(format!("Falha ao reverter `{}`: {}", name, errors.join("; ")));
        }

        Ok(OptimizationOutcome {
            id: id.to_string(),
            name: name.clone(),
            success: true,
            applied: false,
            message: format!("`{}` foi desfeita.", name),
            requires_restart,
            changes_count,
            changes: described,
        })
    }

    /// Liga ou desliga uma tarefa agendada de terceiros.
    ///
    /// Segue o mesmo desenho da inicialização: desligar grava no histórico com
    /// id próprio, ligar de volta desfaz esse registro. Assim "Desfazer tudo"
    /// devolve também as tarefas ao estado em que estavam.
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
        })
    }

    /// Troca o servidor de DNS de um adaptador.
    ///
    /// Entra no histórico com id próprio, então "Desfazer tudo" devolve o DNS
    /// original junto com o resto. Voltar para automático desfaz o registro em
    /// vez de criar um segundo.
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
        })
    }

    /// Leva um serviço de terceiro para Manual, ou devolve para Automático.
    ///
    /// Segue o mesmo desenho das tarefas agendadas: o id próprio faz a mudança
    /// entrar no "Desfazer tudo" junto com o resto, e voltar para Automático
    /// desfaz o registro em vez de criar um segundo.
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
            // A frase importa: o usuário precisa entender que não quebrou nada.
            message: format!(
                "`{}` não sobe mais sozinho no boot. Ele ainda sobe quando o programa pedir.",
                name
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
        })
    }

    /// Liga ou desliga um programa de inicialização.
    ///
    /// Desligar grava no histórico com um id próprio, então "Desfazer tudo"
    /// devolve a inicialização ao estado original junto com o resto. Ligar de novo
    /// desfaz esse registro, restaurando exatamente o valor que existia antes.
    pub fn set_startup(
        &self,
        hive: &str,
        name: &str,
        enabled: bool,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        // Entradas de HKLM valem para todos os usuários da máquina.
        if hive.eq_ignore_ascii_case("HKLM") && !registry::is_elevated() {
            return Err(format!(
                "`{}` vale para todos os usuários do PC e exige executar como administrador.",
                name
            ));
        }

        let id = startup_change_id(hive, name);

        if enabled {
            // Se fomos nós que desligamos, reverter restaura o valor exato.
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
        })
    }

    /// Aplica um lote do que é seguro aplicar sem o usuário escolher item a item.
    ///
    /// `only` restringe a lista: `Some(ids)` é o caminho dos perfis, que aplicam
    /// só o que recomendam, e `None` é o "Otimizar agora", que pega tudo que
    /// está pendente.
    ///
    /// Duas exclusões deliberadas, e elas vêm DEPOIS do filtro de ids — um
    /// perfil não pode arrastar nenhuma das duas só porque citou o id:
    /// - o que não é reversível (apagar arquivo nunca acontece por um clique genérico)
    /// - o que troca segurança por desempenho
    ///
    /// Já aplicado ou já padrão da máquina também fica de fora, pela inspeção.
    ///
    /// A falha de uma otimização não interrompe as demais — cada uma é independente
    /// e já se desfez sozinha antes de reportar o erro.
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
            .filter(|spec| spec.reversible)
            .filter(|spec| !spec.security_tradeoff)
            .filter(|spec| self.inspect(spec, log) == OptimizationState::Available)
            .collect();

        let total = pending.len();

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

    /// Desfaz tudo o que foi aplicado, devolvendo o PC ao estado original.
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

    /// Executa uma ação e acumula os registros necessários para desfazê-la.
    fn execute(
        &self,
        action: &Action,
        changes: &mut Vec<ChangeRecord>,
    ) -> Result<Option<String>, String> {
        match action {
            Action::Registry {
                hive,
                path,
                name,
                value,
            } => {
                let previous = match value {
                    RegValue::Dword(v) => registry::set_dword(hive, path, name, *v)?,
                    RegValue::Text(v) => registry::set_string(hive, path, name, v)?,
                };

                changes.push(ChangeRecord::RegistryValue {
                    hive: hive.to_string(),
                    path: path.to_string(),
                    name: name.to_string(),
                    previous,
                });
                Ok(None)
            }

            Action::DisableService { name } => {
                // Segunda barreira, além dos testes do catálogo: nem uma alteração
                // futura no catálogo consegue desativar um serviço crítico.
                let validation = SafetyValidator::new().validate_operation("service_disable", name);
                if !validation.valid {
                    return Err(format!("Serviço crítico bloqueado: {}", name));
                }

                // Um serviço ausente não é falha: instalações do Windows variam.
                if !services::exists(name) {
                    return Ok(None);
                }

                let previous = services::query_start_type(name)?;

                // Já desativado: nada a fazer e nada a registrar.
                if previous == "disabled" {
                    return Ok(None);
                }

                services::set_start_type(name, "disabled")?;
                changes.push(ChangeRecord::ServiceStartType {
                    service: name.to_string(),
                    previous,
                });

                // Parar o serviço é o que libera recursos agora; a falha em parar
                // não invalida a otimização, que já vale a partir do próximo boot.
                if let Err(error) = services::stop(name) {
                    crate::utils::Logger::warn(&format!("serviço {} não parou agora: {}", name, error));
                }
                Ok(None)
            }

            Action::HighPerformancePowerPlan => {
                let previous = power::active_scheme()?;

                if previous == power::HIGH_PERFORMANCE_GUID {
                    return Ok(None);
                }

                power::ensure_high_performance_exists()?;
                power::set_active_scheme(power::HIGH_PERFORMANCE_GUID)?;

                changes.push(ChangeRecord::PowerPlan {
                    previous_guid: previous,
                });
                Ok(None)
            }

            Action::DisableHibernation => {
                let previously_enabled = power::hibernation_enabled();

                if !previously_enabled {
                    return Ok(None);
                }

                power::set_hibernation(false)?;
                changes.push(ChangeRecord::Hibernation { previously_enabled });
                Ok(Some("Arquivo de hibernação removido.".to_string()))
            }

            Action::PowerSetting {
                subgroup,
                setting,
                value,
            } => {
                let scheme = power::active_scheme()?;
                let previous = power::read_power_setting(&scheme, subgroup, setting)?;

                if previous == PreviousValue::Dword(*value) {
                    return Ok(None);
                }

                power::set_power_setting(&scheme, subgroup, setting, *value)?;
                changes.push(ChangeRecord::PowerSetting {
                    scheme,
                    subgroup: subgroup.to_string(),
                    setting: setting.to_string(),
                    previous,
                });
                Ok(None)
            }

            Action::MemoryCompression { enabled } => {
                let previously_enabled = power::memory_compression_enabled()
                    .ok_or("Não foi possível ler o estado da compressão de memória.")?;

                if previously_enabled == *enabled {
                    return Ok(None);
                }

                power::set_memory_compression(*enabled)?;
                changes.push(ChangeRecord::MemoryCompression { previously_enabled });
                Ok(None)
            }

            Action::ClearBootLimits => {
                let removed = firmware::boot_limits();

                if removed.is_empty() {
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
                changes.extend(devices::ativar_msi()?);
                Ok(None)
            }

            Action::NicPowerSaving => {
                changes.extend(devices::desligar_economia_de_energia_da_rede()?);
                Ok(None)
            }

            Action::ReservedStorage { enabled } => {
                let anterior = power::reserved_storage_enabled()
                    .ok_or("Este Windows não tem Armazenamento Reservado.")?;

                if anterior == *enabled {
                    return Ok(None);
                }

                power::set_reserved_storage(*enabled)?;
                changes.push(ChangeRecord::ReservedStorage {
                    previously_enabled: anterior,
                });
                Ok(Some("Espaço reservado devolvido ao disco.".to_string()))
            }

            Action::RemoveForcedPlatformClock => {
                let Some(valor) = firmware::forced_platform_clock() else {
                    return Ok(None);
                };

                shell::run_checked("bcdedit", &["/deletevalue", "{current}", "useplatformclock"])?;

                // Reaproveita o registro de limites de boot: a reversão dele já
                // sabe devolver um valor do bcdedit ao que estava.
                changes.push(ChangeRecord::BootLimits {
                    removed: vec![("useplatformclock".to_string(), valor.clone())],
                });

                Ok(Some(format!(
                    "Relógio de plataforma forçado removido (estava em {}).",
                    valor
                )))
            }

            Action::CleanUpdateCache => {
                let result = cleanup::run_update_cache()?;

                Ok(Some(format!(
                    "{} liberados dos instaladores de atualização.",
                    cleanup::format_size(result.bytes_freed)
                )))
            }

            Action::CleanTempFiles => {
                let result = cleanup::run();

                // Nada é registrado no ChangeLog: arquivo apagado não volta, e
                // fingir que volta seria pior que admitir que não.
                let mut note = format!(
                    "{} liberados em {} itens.",
                    cleanup::format_size(result.bytes_freed),
                    result.files_removed
                );

                if result.files_skipped > 0 {
                    note.push_str(&format!(
                        " {} itens em uso foram pulados.",
                        result.files_skipped
                    ));
                }

                Ok(Some(note))
            }

            Action::DisableNagle => {
                let interfaces = registry::subkeys("HKLM", TCPIP_INTERFACES)?;

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
                    }
                }

                Ok(None)
            }
        }
    }
}

impl Default for WindowsOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Identificador de uma entrada de inicialização no histórico.
///
/// O prefixo separa essas entradas dos ids do catálogo, então nunca há colisão
/// entre um programa chamado "SysMain" e a otimização de mesmo nome.
fn startup_change_id(hive: &str, name: &str) -> String {
    format!("startup:{}:{}", hive.to_uppercase(), name)
}

/// Se esta otimização pesa muito mais nesta máquina do que na média.
///
/// É o que permite dizer ao dono de um PC de 4 GB quais ajustes valem a pena
/// para ELE, em vez de entregar a mesma lista de vinte itens para todo mundo e
/// deixar a pessoa adivinhar.
fn pesa_nesta_maquina(spec: &OptimizationSpec) -> bool {
    use catalog::Boost;
    use hardware::StorageKind;

    let perfil = hardware::profile();

    spec.highlight_when.iter().any(|condicao| match condicao {
        // 8 GB é a fronteira prática: abaixo disso o Windows já começa a
        // comprimir memória e a paginar em uso comum.
        Boost::LowRam => perfil.total_ram_gb <= 8.5,
        Boost::MechanicalDisk => perfil.system_storage == StorageKind::Hdd,
        Boost::FewCores => perfil.logical_cores <= 4,
    })
}

/// Se esta máquina atende à condição de hardware da otimização.
///
/// Quando o tipo do disco é desconhecido, a resposta é "não atende": preferimos
/// não oferecer a arriscar deixar o PC do cliente pior por um palpite.
fn meets_requirement(spec: &OptimizationSpec) -> bool {
    use catalog::Requirement;
    use hardware::StorageKind;

    match spec.requirement {
        None => true,
        Some(Requirement::SsdSystemDrive) => {
            hardware::profile().system_storage == StorageKind::Ssd
        }
        Some(Requirement::MinRamGb(minimum)) => hardware::profile().total_ram_gb >= minimum,
    }
}

/// Desfaz uma lista de mudanças na ordem inversa em que foram aplicadas.
/// Tenta reverter todas mesmo se alguma falhar, e devolve as falhas acumuladas.
fn revert_changes(changes: &[ChangeRecord]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    for change in changes.iter().rev() {
        let result = match change {
            ChangeRecord::RegistryValue {
                hive,
                path,
                name,
                previous,
            } => registry::restore(hive, path, name, previous),

            ChangeRecord::ServiceStartType { service, previous } => {
                services::set_start_type(service, previous)
            }

            ChangeRecord::PowerPlan { previous_guid } => power::set_active_scheme(previous_guid),

            ChangeRecord::Hibernation { previously_enabled } => {
                power::set_hibernation(*previously_enabled)
            }

            ChangeRecord::PowerSetting {
                scheme,
                subgroup,
                setting,
                previous,
            } => power::restore_power_setting(scheme, subgroup, setting, previous),

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
        };

        if let Err(error) = result {
            errors.push(error);
        }
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

    /// A inspeção lê o sistema real: nenhuma otimização pode aparecer com estado
    /// errado por causa de exceção não tratada.
    #[test]
    fn inspects_every_optimization_against_this_machine() {
        let optimizer = WindowsOptimizer::new();
        let log = ChangeLog::load();

        for info in optimizer.list(&log) {
            println!("{:<45} {:?} {:?}", info.name, info.state, info.detail);
        }

        assert_eq!(optimizer.list(&log).len(), catalog::CATALOG.len());
    }

    /// "Otimizar Agora" nunca pode apagar arquivos do cliente sem ele escolher isso.
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

        assert!(!batch.contains(&"clean_temp_files"));
    }

    /// Sem elevação, o Windows nega a leitura de algumas configurações. Nesses
    /// casos o produto não pode dizer "já otimizado" — isso seria afirmar o que
    /// não foi verificado, que é exatamente o que ele existe para não fazer.
    #[test]
    fn nao_afirma_estar_otimizado_o_que_nao_conseguiu_conferir() {
        // Com elevação a leitura funciona e a regra não se aplica.
        if registry::is_elevated() {
            return;
        }

        let optimizer = WindowsOptimizer::new();
        let log = ChangeLog::load();

        for id in ["disable_reserved_storage", "remove_forced_hpet", "clear_boot_limits"] {
            let spec = catalog::find(id).expect("otimização deveria existir");

            // Já aplicada por nós é outra história: aí o histórico é a prova.
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

    /// "Otimizar Agora" nunca pode abrir mão de segurança por conta própria.
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

    /// Toda otimização que reduz segurança precisa gritar isso no texto que o
    /// cliente lê, não esconder numa etiqueta.
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

    /// Ciclo real de uma entrada de inicialização: desliga, confere, religa e
    /// confere que os bytes voltaram EXATAMENTE como estavam.
    ///
    /// Byte-exato importa: o Windows guarda a data/hora do desligamento nos bytes
    /// 4 a 11. Restaurar "equivalente" deixaria rastro nosso no registro do
    /// cliente. Restaurar idêntico não deixa nenhum.
    ///
    /// `cargo test --lib -- --ignored --nocapture real_startup_cycle`
    #[test]
    #[ignore]
    fn real_startup_cycle_restores_exact_bytes() {
        const APPROVED: &str =
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

        let optimizer = WindowsOptimizer::new();
        let mut log = ChangeLog::load();

        // Usa a primeira entrada de HKCU: não exige administrador e vale só para
        // este usuário.
        let entry = startup::entries()
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

    /// Ciclo real de TODAS as otimizações que exigem administrador.
    ///
    /// Cada uma é aplicada, conferida contra o sistema e desfeita, e no fim o
    /// estado precisa estar idêntico ao do começo. É o teste que faltava: até
    /// aqui só o que roda sem elevação tinha sido executado de verdade.
    ///
    /// Exige sessão elevada:
    /// `cargo test --lib -- --ignored --nocapture real_admin_optimizations`
    #[test]
    #[ignore]
    fn real_admin_optimizations_apply_and_revert() {
        assert!(
            registry::is_elevated(),
            "este teste precisa de uma sessão como administrador"
        );

        let optimizer = WindowsOptimizer::new();
        let mut log = ChangeLog::load();

        // Só as que exigem elevação, são reversíveis e não trocam segurança por
        // desempenho. `AlreadyOptimal` fica de fora: testá-la exigiria
        // desconfigurar a máquina de quem está rodando o teste.
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

        for spec in alvos {
            let estado_inicial = optimizer.inspect(spec, &log);

            // Cada uma percorre o ciclo completo e termina exatamente no estado em
            // que começou. Quem já está aplicada percorre o caminho inverso —
            // mesmos códigos, ordem trocada — em vez de ficar sem cobertura.
            let (primeiro, segundo, esperado_no_meio) = match estado_inicial {
                OptimizationState::Applied => ("desfazer", "aplicar", OptimizationState::Available),
                _ => ("aplicar", "desfazer", OptimizationState::Applied),
            };

            let executar = |acao: &str, log: &mut ChangeLog| match acao {
                "aplicar" => optimizer.apply(spec.id, log),
                _ => optimizer.revert(spec.id, log),
            };

            let meio = executar(primeiro, &mut log)
                .unwrap_or_else(|erro| panic!("{} `{}` falhou: {}", primeiro, spec.id, erro));

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
        println!("ciclo completo em {} otimização(ões)", testadas);
    }

    /// Fluxo completo do produto: medir → otimizar → medir de novo → comparar → desfazer.
    ///
    /// Valida o encadeamento inteiro contra o sistema real. Leva ~20 segundos.
    /// `cargo test --release --lib -- --ignored --nocapture real_full_cycle`
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

    /// Ciclo real contra o registro do Windows: aplica, confere, desfaz e confere
    /// que o sistema voltou EXATAMENTE ao estado anterior.
    ///
    /// Marcado como `ignore` porque altera o sistema de verdade. Rode com:
    /// `cargo test --lib -- --ignored --nocapture real_apply_and_revert`
    ///
    /// Usa `disable_startup_delay`: fica em HKCU, não exige administrador e é
    /// totalmente reversível — a escolha certa para validar o mecanismo.
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
