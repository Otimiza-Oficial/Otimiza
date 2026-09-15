// Otimizador do Windows
//
// Aplica e desfaz as otimizações do catálogo. Duas regras governam este módulo:
//
// 1. Nenhuma mudança é feita sem antes gravar o estado anterior no ChangeLog.
// 2. Se uma ação falhar no meio de uma otimização, as ações já aplicadas são
//    desfeitas antes de reportar o erro — o sistema nunca fica pela metade.

pub mod acessibilidade;
pub mod achados;
pub mod anticheat;
pub mod bloatware;
pub mod boot;
pub mod bottleneck;
pub mod browsers;
pub mod cabecalho;
pub mod catalog;
pub mod cbslog;
pub mod citizenfx;
pub mod cleanup;
pub mod configjogo;
pub mod conflicts;
pub mod deteccao;
pub mod devices;
pub mod diskspace;
pub mod display;
pub mod essenciais;
pub mod exhaustion;
pub mod firmware;
pub mod fivem;
pub mod foldermap;
pub mod frames;
pub mod gamemode;
pub mod gpupref;
pub mod hardware;
pub mod discodojogo;
pub mod jogos;
pub mod labcompat;
pub mod health;
pub mod memory;
pub mod network;
pub mod nvdriver;
pub mod planoenergia;
pub mod pcie;
pub mod power;
pub mod pressao;
pub mod processes;
pub mod profiles;
pub mod rbar;
pub mod readiness;
pub mod rede;
pub mod registry;
pub mod reparo;
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
pub mod tarefa_longa;
pub mod tasks;
pub mod thermal;
pub mod veredito;

use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeLog, ChangeRecord, PreviousValue};
use crate::modules::optimizer::{
    ActionResult, ActionStatus, BatchStep, OptimizationInfo, OptimizationOutcome, OptimizationState,
};
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
    /// NÃO DEU PARA LER o estado atual.
    ///
    /// Este estado existe porque as leituras que falhavam caíam em
    /// `NotApplicable`, e a tela então dizia ao cliente "não se aplica a esta
    /// máquina" — uma afirmação sobre o computador dele que ninguém verificou.
    /// O caminho mais provável disso é justamente a máquina do cliente: chave
    /// de registro com permissão negada, imagem modificada, política de
    /// domínio. Ou seja, o produto ficava mais calado exatamente onde ele
    /// precisa falar.
    ///
    /// `registry::read` já separa "não existe" (que volta `Ok`) de "não
    /// consegui ler" (que volta `Err`) desde a 1.8. Quem achatava os dois era
    /// esta camada.
    ///
    /// Não confundir com `NotApplicable`: lá o produto SABE que não se aplica.
    Desconhecido,
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

                // UM RÓTULO SEM MOTIVO NÃO AJUDA NINGUÉM. "Não deu para
                // verificar" sozinho deixa o cliente sem saber se o problema é
                // dele, do produto, ou do Windows — e é essa a frase que ele vai
                // colar no suporte. A causa quase sempre é uma destas três, e
                // dizer quais são já encurta a conversa pela metade.
                //
                // Só quando o `detail` do item não tem nada mais específico a
                // dizer: medida concreta vence explicação genérica.
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

    /// Junta o estado das ações no estado da otimização.
    ///
    /// Função pura, separada do `inspect` de propósito: é aqui que mora a regra
    /// de o que o cliente vê, e ela precisa de teste sem depender de máquina.
    ///
    /// A ORDEM DAS PERGUNTAS É A REGRA. Uma ação pendente vence o
    /// desconhecimento — se sabemos que há trabalho a fazer, esconder o item
    /// atrás de "não deu para verificar" tiraria do cliente uma otimização real
    /// por causa de uma leitura alheia que falhou. Mas quando o que sobra é
    /// desconhecimento, ele não pode virar nem "já está bom" nem "não se aplica":
    /// as duas são afirmações sobre o PC do cliente que ninguém verificou.
    fn compor(states: &[ActionState]) -> OptimizationState {
        // Sabemos que há o que fazer. Isto vem primeiro.
        if states.iter().any(|s| *s == ActionState::Pending) {
            return OptimizationState::Available;
        }

        // Alguma leitura falhou, e nada acima provou que há trabalho.
        if states.iter().any(|s| *s == ActionState::Desconhecido) {
            return OptimizationState::Unknown;
        }

        // Nenhuma ação pode rodar aqui: a otimização não serve para esta máquina.
        if states.iter().all(|s| *s == ActionState::NotApplicable) {
            return OptimizationState::Unavailable;
        }

        // Satisfeita em todo lugar que se aplica: o PC já estava assim.
        OptimizationState::AlreadyOptimal
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

        Self::compor(&states)
    }

    /// Informação medida agora, quando a otimização tem um número ou um motivo
    /// concreto a mostrar sobre ESTA máquina.
    fn detail(&self, spec: &OptimizationSpec) -> Option<String> {
        if let Some(requirement) = spec.requirement {
            if !meets_requirement(spec) {
                return Some(motivo_da_recusa(requirement));
            }
        }

        // O VBS É O ÚNICO ITEM ONDE A CHAVE DE REGISTRO NÃO É A VERDADE.
        //
        // `EnableVirtualizationBasedSecurity = 0` entra sempre, e a releitura
        // devolve 0 — então a lista marcava a otimização como aplicada. Só que
        // quem manda é o que está RODANDO, e o VBS pode continuar de pé depois
        // do reinício: por política de domínio, por bloqueio em UEFI, ou porque
        // a Integridade de Memória foi religada. O cliente reiniciava o PC,
        // perdia o Hyper-V e o WSL, não ganhava o FPS, e a tela dizia "feito".
        //
        // A resposta verdadeira já existia no produto — `firmware::vbs_running`
        // lê `Win32_DeviceGuard`, que devolve NÚMERO e não texto traduzido — e
        // era usada só no diagnóstico. Aqui ela custa uma chamada ao PowerShell
        // por atualização da lista, e vale: é a diferença entre mostrar o
        // registro e mostrar a máquina.
        //
        // Por id e não por ação porque a primeira ação deste item é uma escrita
        // de registro comum, indistinguível das outras.
        if spec.id == "disable_vbs" {
            return Some(
                match firmware::vbs_running() {
                    Some(true) => "Agora: VBS ligado e em execução nesta máquina.",
                    Some(false) => "Agora: VBS não está em execução nesta máquina.",
                    // Não saber não pode virar "está desligado": é justamente o
                    // silêncio que faria o cliente reiniciar por nada.
                    None => "Não foi possível ler se o VBS está em execução nesta máquina.",
                }
                .to_string(),
            );
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

            // Elevado e ainda assim sem resposta. Medido no Windows 11 Pro da
            // máquina de desenvolvimento: o comando existe e devolve "acesso
            // negado". O item aparece como disponível porque não foi possível
            // conferir — e isso precisa estar escrito, senão vira promessa de
            // conserto para um problema que talvez nem exista.
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
                    // O Windows realmente não tem este serviço. Instalações
                    // variam, e isso o produto SABE.
                    Some(false) => return ActionState::NotApplicable,
                    // A chave do serviço não pôde ser lida — ACL de domínio,
                    // endurecimento, antivírus. Dizer "não se aplica" faria a
                    // otimização sumir da lista com a frase errada.
                    None => return ActionState::Desconhecido,
                }

                match services::query_start_type(name) {
                    Ok(start_type) if start_type == "disabled" => ActionState::Satisfied,
                    Ok(_) => ActionState::Pending,
                    Err(_) => ActionState::Desconhecido,
                }
            }

            // Comparado com o plano de alto desempenho QUE EXISTE NESTA
            // MÁQUINA, e não com o GUID fixo: onde o Windows não traz o Alto
            // Desempenho, o produto usa uma cópia com GUID próprio, e comparar
            // com o fixo deixaria a otimização eternamente "pendente" mesmo
            // depois de aplicada.
            Action::PlanoOtimiza => match planoenergia::onde_esta_o_plano() {
                planoenergia::EstadoDoPlano::Ativo => ActionState::Satisfied,
                planoenergia::EstadoDoPlano::ExisteEnaoEstaAtivo
                | planoenergia::EstadoDoPlano::NaoExiste => ActionState::Pending,
                // Não conseguir ler NÃO É "não está aplicado": oferecer aplicar
                // de novo aqui seria pedir ao cliente que refizesse algo que
                // talvez já esteja feito.
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
                // Sem nenhuma interface listada, não há placa de rede em que
                // mexer — isso o produto SABE.
                Ok(_) => ActionState::NotApplicable,
                // A lista não pôde ser lida. Antes caía junto com o caso acima e
                // virava "não se aplica a esta máquina", sobre um PC que tem
                // placa de rede como qualquer outro.
                Err(_) => ActionState::Desconhecido,
            },

            Action::DisableHibernation => {
                match power::hibernation_enabled() {
                    Some(true) => ActionState::Pending,
                    Some(false) => ActionState::Satisfied,
                    // Não ler não pode virar "já está desativada": seria dizer
                    // ao cliente que o espaço já foi liberado sem ter olhado.
                    None => ActionState::Desconhecido,
                }
            }

            Action::MemoryCompression { enabled } => {
                // A condição de RAM já foi checada em `meets_requirement`; aqui
                // só resta comparar o estado atual com o desejado.
                match power::memory_compression_enabled() {
                    Some(current) if current == *enabled => ActionState::Satisfied,
                    Some(_) => ActionState::Pending,
                    // O `Get-MMAgent` não respondeu. A compressão de memória
                    // existe em todo Windows 10 e 11 — dizer "não se aplica a
                    // esta máquina" era inventar uma limitação que ela não tem.
                    None => ActionState::Desconhecido,
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

                use power::EstadoReservado;

                match power::estado_do_armazenamento_reservado() {
                    EstadoReservado::Ligado if *enabled => ActionState::Satisfied,
                    EstadoReservado::Desligado if !*enabled => ActionState::Satisfied,
                    EstadoReservado::Ligado | EstadoReservado::Desligado => ActionState::Pending,
                    // O comando respondeu e não trouxe estado: este Windows não
                    // tem o recurso.
                    EstadoReservado::SemRecurso => ActionState::NotApplicable,
                    // O comando não respondeu. Não sabemos — e "não sabemos"
                    // nunca pode virar "não se aplica a esta máquina".
                    //
                    // Era `Pending` porque `Desconhecido` não existia, e
                    // `Pending` ao menos não escondia o item. Agora há o estado
                    // certo: `Pending` afirmava que HÁ o que aplicar, o que
                    // também não tinha sido verificado.
                    EstadoReservado::NaoVerificavel => ActionState::Desconhecido,
                }
            }

            // Só é oferecida quando alguma das três está de fato ligada. Numa
            // máquina no padrão do Windows — que é a maioria — esta linha nem
            // aparece, em vez de virar mais um item para inflar a lista.
            Action::AccessibilityKeysOff => {
                if acessibilidade::ligadas().is_empty() {
                    ActionState::Satisfied
                } else {
                    ActionState::Pending
                }
            }

            // Mesma regra dos outros itens que dependem do `bcdedit`: sem
            // elevação a leitura não acontece, e não se afirma o que não foi
            // verificado.
            Action::DisableHypervisor => {
                if !registry::is_elevated() {
                    return ActionState::Pending;
                }

                match power::hypervisor_launch_type() {
                    Some(tipo) if tipo == "off" => ActionState::Satisfied,
                    Some(_) => ActionState::Pending,
                    // O `bcdedit` não respondeu, ou a linha não estava lá. O
                    // próprio `hypervisor_launch_type` documenta que `None` é
                    // "não conseguimos ler" e não "desligado" — esta camada é
                    // que transformava isso em "não se aplica".
                    None => ActionState::Desconhecido,
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
    ///
    /// Cada aplicação deixa rastro no registro em arquivo (`utils::logger`):
    /// quando começou, cada ação antes de executá-la, e como terminou — inclusive
    /// a falha que se desfez sozinha, que o histórico de desfazer não guarda.
    pub fn apply(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        let inicio = std::time::Instant::now();
        crate::utils::Logger::info(&format!("aplicar `{}`: começou", id));

        let resultado = self.aplicar_sem_registro(id, log);
        anotar_fim("aplicar", id, inicio, &resultado);
        resultado
    }

    fn aplicar_sem_registro(&self, id: &str, log: &mut ChangeLog) -> Result<OptimizationOutcome, String> {
        let spec = catalog::find(id).ok_or_else(|| format!("Unknown optimization: {}", id))?;

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

        for (numero, action) in spec.actions.iter().enumerate() {
            // Antes de executar, e não depois: se esta ação travar, a última
            // linha do registro diz qual foi.
            crate::utils::Logger::info(&format!(
                "aplicar `{}`: ação {}/{} — {:?}",
                spec.id,
                numero + 1,
                total_de_acoes,
                action
            ));

            // O DETALHE É CRIADO AQUI E EMPRESTADO AO RAMO, em vez de cada ramo
            // montar o seu. Nome, relógio e estado padrão saem de um lugar só; o
            // ramo preenche apenas o que ele é o único a saber — o valor de
            // antes, o que foi pedido, o que ficou, e a saída do comando.
            //
            // O estado nasce `Verified` porque, depois da releitura obrigatória
            // que todo ramo de escrita agora faz, um `Ok` significa exatamente
            // isso. Os ramos que sabem mais — já estava bom, não existe aqui —
            // corrigem antes de sair.
            let relogio = std::time::Instant::now();
            let mut detalhe = ActionResult {
                name: nome_da_acao(action),
                status: ActionStatus::Verified,
                ..Default::default()
            };

            let resultado = self.execute(action, &mut changes, &mut detalhe);
            detalhe.duration_ms = relogio.elapsed().as_millis() as u64;

            // DEPOIS DO RAMO, E NÃO DENTRO DELE. O ramo sabe o que aconteceu
            // com o ajuste; quem manda na máquina é outra pergunta, e a mesma
            // para todos os ramos. Repeti-la em cada um seria dezessete cópias
            // da mesma regra.
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
                    // `VerificationFailed` é diferente de `Failed`, e quem sabe
                    // qual dos dois é o ramo — ele classifica antes de devolver
                    // o erro. Só quem não disse nada vira `Failed`.
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
            // DERIVADO DAS AÇÕES, e não `true` cravado. Chegar até aqui já
            // significa que nada devolveu erro — mas o `success` passa a sair
            // do mesmo lugar que o cliente vê ação por ação, e não de uma
            // afirmação paralela que pode divergir no próximo conserto.
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

    /// Desfaz uma otimização, restaurando cada valor ao estado anterior.
    ///
    /// Funciona também para o que não está no catálogo — como as entradas de
    /// inicialização desligadas pelo usuário. O histórico guarda o suficiente para
    /// reverter qualquer coisa que a gente tenha mexido, e é ele quem manda aqui.
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
            ..Default::default()
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

    /// Fixa a prioridade alta de um jogo, valendo em toda abertura.
    ///
    /// Entra no histórico, então "Desfazer tudo" remove o ajuste junto com o
    /// resto — inclusive se o programa fechar no meio.
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
                "`{}` passa a abrir sempre em prioridade alta. Atenção: quando o jogo                  atualizar, o nome do executável muda e este ajuste precisa ser aplicado de                  novo — o Otimiza avisa quando isso acontecer.",
                executable
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// Escolhe qual placa de vídeo um jogo deve usar.
    ///
    /// Entra no histórico com id próprio por jogo, então "Desfazer tudo"
    /// devolve a preferência que existia antes — inclusive a ausência dela.
    /// Coloca um monitor na maior taxa que ele aceita na resolução atual.
    ///
    /// É a única ação do produto que muda o que a tela mostra na hora, e por
    /// isso é a que mais precisa de caminho de volta: entra no histórico com a
    /// frequência anterior, e o "Desfazer" a devolve.
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
            // Reaplicar por cima perderia o valor original: o histórico
            // guardaria como "anterior" aquilo que nós mesmos escrevemos.
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
            // A honestidade que o achado já dizia, repetida no momento em que
            // ela mais importa: o cliente acabou de clicar e vai olhar o
            // contador de FPS esperando um número maior.
            message: format!(
                "{} passou de {} para {} Hz. O jogo fica visivelmente mais suave, e o                  contador de FPS continua onde estava — a taxa do monitor não cria                  quadros, ela deixa de segurar os que a placa já entrega.",
                alvo.descricao, anterior, maximo
            ),
            requires_restart: false,
            changes_count: 1,
            changes: vec![described],
            ..Default::default()
        })
    }

    /// Aplica um ajuste do driver NVIDIA no perfil global e registra o valor que
    /// existia antes.
    ///
    /// Até a 2.0 o `nvdriver.rs` tinha o desfazer e não tinha quem chamasse o
    /// fazer. Esta é a porta.
    ///
    /// O histórico é gravado logo depois da escrita no driver. Se ele não
    /// gravar, o ajuste é desfeito na hora: mudança no driver sem registro é
    /// mudança sem caminho de volta pelo Otimiza.
    pub fn aplicar_ajuste_nvidia(
        &self,
        opcao: &str,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let alvo = nvdriver::opcao_por_id(opcao)
            .ok_or_else(|| format!("não conheço o ajuste de driver \"{}\".", opcao))?;
        let id = nvdriver::id_no_historico(opcao);

        if log.is_applied(&id) {
            // Reaplicar por cima gravaria como "anterior" o valor que nós
            // mesmos escrevemos, e o desfazer devolveria o nosso.
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

    /// Limita os quadros por segundo de um jogo, no perfil do executável dele no
    /// driver da NVIDIA.
    ///
    /// NUNCA NO PERFIL GLOBAL: um limite global prenderia a área de trabalho e
    /// todo outro jogo no mesmo número. Trocar o número de um jogo já limitado
    /// desfaz o limite anterior primeiro — senão o histórico guardaria como
    /// "anterior" o limite que nós mesmos pusemos.
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

    pub fn set_gpu_preference(
        &self,
        caminho: &str,
        preferencia: gpupref::Preferencia,
        log: &mut ChangeLog,
    ) -> Result<OptimizationOutcome, String> {
        let id = format!("placa:{}", caminho.to_lowercase());

        if log.is_applied(&id) {
            // Reaplicar por cima perderia o valor original: o histórico
            // guardaria como "anterior" aquilo que nós mesmos escrevemos.
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
            ..Default::default()
        })
    }

    /// Volta ao tipo de início padrão do Windows cada serviço essencial que está
    /// desativado (`essenciais::ESSENCIAIS`).
    ///
    /// Cada chamada entra no histórico com um id próprio, marcado com o instante:
    /// um Windows modificado pode ter os serviços desligados de novo por fora, e
    /// cada religada precisa poder ser desfeita sozinha, sem apagar a anterior.
    ///
    /// Falha num serviço não impede os outros. Cada um é independente, e o que
    /// foi religado entra no histórico mesmo quando outro falhou — senão ficaria
    /// mudado sem caminho de volta.
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
            // A frase importa: o usuário precisa entender que não quebrou nada.
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

    /// Aplica um lote do que é seguro aplicar sem o usuário escolher item a item.
    ///
    /// `only` restringe a lista: `Some(ids)` é o caminho dos perfis, que aplicam
    /// só o que recomendam, e `None` é o "Otimizar agora", que pega tudo que
    /// está pendente.
    ///
    /// Três exclusões deliberadas, em `catalog::entra_no_lote`, e elas vêm
    /// DEPOIS do filtro de ids — um perfil não pode arrastar nenhuma das três só
    /// porque citou o id:
    /// - o que não é reversível (apagar arquivo nunca acontece por um clique genérico)
    /// - o que troca segurança por desempenho
    /// - o que está em `catalog::FORA_DO_LOTE`, cujo efeito o cliente só sente dias depois
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
            .filter(|spec| catalog::entra_no_lote(spec))
            // SÓ `Available`, E ISSO AGORA DEIXA `Unknown` DE FORA DE PROPÓSITO.
            //
            // O "Otimizar agora" é o botão que o cliente aperta sem ler item a
            // item, e por isso ele só leva o que o produto CONFERIU que falta.
            // Item cujo estado não pôde ser lido continua na lista, com a frase
            // dizendo o porquê, para a pessoa decidir — mas não entra num lote
            // que ninguém revisou.
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

    /// Executa uma ação e acumula os registros necessários para desfazê-la.
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

                // O QUE HAVIA ANTES, lido antes de escrever. É o campo que
                // responde "o Otimiza mudou o quê, exatamente?" sem o cliente
                // ter que confiar na nossa palavra.
                let antes = registry::read(hive, path, name);
                detalhe.before_value = antes.as_ref().ok().map(descrever_valor);
                detalhe.expected_value = Some(descrever_alvo(value));

                // JÁ ESTAVA NO ALVO: não escreve e não registra.
                //
                // Todos os outros ramos já faziam isso, e este não — ele
                // gravava o valor por cima do mesmo valor e devolvia
                // `Verified`, dizendo ao cliente que mudou o que não mudou. Foi
                // visto num resultado real: `antes=0 esperado=0 depois=0` com
                // estado "conferido".
                //
                // Também evita uma linha inútil no histórico: um desfazer que
                // reescreve o mesmo número não desfaz nada, e ocupa espaço na
                // lista do cliente como se desfizesse.
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

                // ANTES DE CONFERIR, E NÃO DEPOIS. Se a conferência reprovar, o
                // valor JÁ ESTÁ GRAVADO no PC do cliente — e é este registro que
                // faz a reversão automática do `apply` conseguir devolvê-lo.
                changes.push(ChangeRecord::RegistryValue {
                    hive: hive.to_string(),
                    path: path.to_string(),
                    name: name.to_string(),
                    previous,
                });

                // NÃO CONFIE QUE FUNCIONOU: RELÊ.
                //
                // A escrita retornar `Ok` só diz que o Windows aceitou o pedido,
                // não que o valor ficou. Em máquina gerenciada por política de
                // domínio, o valor volta sozinho; em processo de 32 bits sobre
                // Windows de 64, a escrita cai no espelho `WOW6432Node` e o
                // sistema continua lendo a chave verdadeira. Nos dois casos o
                // produto dizia "aplicado" sobre um PC que não mudou.
                //
                // É a mesma regra que o plano de energia já seguia, agora no
                // caminho por onde passa a maior parte do catálogo.
                let relido = registry::read(hive, path, name);
                detalhe.after_value = relido.as_ref().ok().map(descrever_valor);

                match conferir_escrita(value, &relido) {
                    Confirmacao::Igual => {}
                    Confirmacao::Diferente(lido) => {
                        // `VerificationFailed`, e não `Failed`: o comando foi
                        // aceito. A diferença diz ao atendimento que o problema
                        // não está no Otimiza.
                        detalhe.status = ActionStatus::VerificationFailed;

                        return Err(format!(
                            "O Windows aceitou a gravação de `{}`, mas ao reler o valor é {}. \
                             Costuma ser política de domínio ou outro programa reescrevendo a \
                             chave. Nada ficou aplicado.",
                            name, lido
                        ))
                    }
                    // Gravou e não deu para reler. Não é falha — desfazer aqui
                    // seria descartar uma mudança que provavelmente valeu — mas
                    // também não pode passar como confirmado.
                    Confirmacao::NaoDeuParaLer => {
                        detalhe.status = ActionStatus::NotConfirmed;
                        nao_confirmado = Some(format!(
                            "`{}` foi gravado, mas não foi possível reler para confirmar.",
                            name
                        ));
                    }
                }

                // Gravar não basta: as preferências de `HKCU\Control Panel`
                // ficam em memória desde o logon, e sem avisar o Windows a
                // otimização valeria só no próximo — numa tela que promete
                // efeito imediato.
                if sysparams::precisa_sincronizar_interface(hive, path) {
                    sysparams::sincronizar_interface();
                }

                // Quando o shell só relê a chave ao iniciar, o cliente precisa
                // saber disso — senão aplica, não vê nada mudar na barra de
                // tarefas e conclui que o produto não funcionou.
                // As duas notas podem existir juntas, e nenhuma pode engolir a
                // outra: uma diz que o efeito só aparece depois de reiniciar o
                // shell, a outra diz que não deu para confirmar a gravação.
                let ativacao = sysparams::nota_de_ativacao(hive, path, name).map(|n| n.to_string());

                Ok(match (nao_confirmado, ativacao) {
                    (Some(a), Some(b)) => Some(format!("{} {}", a, b)),
                    (Some(unica), None) | (None, Some(unica)) => Some(unica),
                    (None, None) => None,
                })
            }

            Action::DisableService { name } => {
                // Segunda barreira, além dos testes do catálogo: nem uma alteração
                // futura no catálogo consegue desativar um serviço crítico.
                let validation = SafetyValidator::new().validate_operation("service_disable", name);
                if !validation.valid {
                    return Err(format!("Serviço crítico bloqueado: {}", name));
                }

                // Um serviço ausente não é falha: instalações do Windows variam.
                match services::exists(name) {
                    Some(true) => {}
                    Some(false) => {
                        detalhe.status = ActionStatus::Unsupported;
                        detalhe.unsupported_reason =
                            Some(format!("O serviço {} não existe neste Windows.", name));
                        return Ok(None);
                    }
                    // Sem conseguir ler a chave do serviço não dá para saber o
                    // tipo de inicialização atual, e sem isso não há caminho de
                    // volta — a mesma regra do hipervisor e da hibernação.
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

                // Já desativado: nada a fazer e nada a registrar.
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

                // NÃO CONFIE QUE FUNCIONOU. `sc config` devolver 0 diz que o
                // Gerenciador de Serviços aceitou o pedido; em máquina com
                // política de domínio, ou com o serviço trancado pelo próprio
                // Windows, o tipo volta ao que era.
                let agora = services::query_start_type(name).ok();
                detalhe.after_value.clone_from(&agora);

                let nota = exigir_confirmacao(
                    conferir(&"disabled".to_string(), agora),
                    &format!("o serviço {}", name),
                )
                .inspect_err(|_| detalhe.status = ActionStatus::VerificationFailed)?;

                // Parar o serviço é o que libera recursos agora; a falha em parar
                // não invalida a otimização, que já vale a partir do próximo boot.
                if let Err(error) = services::stop(name) {
                    crate::utils::Logger::warn(&format!("serviço {} não parou agora: {}", name, error));
                }
                Ok(nota)
            }

            Action::PlanoOtimiza => {
                let relatorio = planoenergia::montar(false, false)?;

                if !relatorio.plano_ativo {
                    return Err(
                        "O plano OTIMIZA foi montado mas o Windows não o deixou ativo. \
                         Nada foi mudado no seu plano de energia."
                            .to_string(),
                    );
                }

                let Some(anterior) = relatorio.guid_anterior.clone() else {
                    // Sem saber qual era o plano de antes não há caminho de
                    // volta, e aplicar sem volta é o que este produto não faz.
                    return Err(
                        "Não foi possível ler qual plano de energia estava ativo antes. \
                         O plano OTIMIZA não foi ativado."
                            .to_string(),
                    );
                };

                // O plano OTIMIZA já era o ativo: os ajustes podem ter sido
                // conferidos ou corrigidos, mas não houve troca de plano para
                // desfazer. Gravar `previous_guid` igual ao nosso faria o
                // "Desfazer" reativar o próprio plano que ele deveria remover.
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

                // SEM SABER O ESTADO ANTERIOR NÃO SE MEXE. É a mesma regra do
                // hipervisor: não há como prometer a volta do que não foi lido,
                // e o histórico guardaria um "antes" inventado.
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

                // `HibernateEnabled` muda na hora, então dá para conferir agora
                // — e aqui a conferência tem um valor extra: quando a hibernação
                // não desliga, o `hiberfil.sys` continua ocupando o disco, e o
                // cliente ia atrás do espaço que a tela prometeu.
                // `Option` direto, e não embrulhado num `Some`: era esse embrulho
                // que fazia a conferência validar a si mesma. Com a leitura
                // quebrada devolvendo `false`, o "depois" batia com o alvo e o
                // produto dava por conferido o que nunca leu. Agora `None` cai
                // em `NaoDeuParaLer`, que é a verdade.
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

                // O ajuste mais profundo do catálogo escreve numa chave de
                // dispositivo que o Windows pode reescrever ao reenumerar o
                // hardware. O efeito só vale depois do reinício, mas o valor é
                // legível agora — e é o valor que estamos prometendo.
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

                // A MESMA distinção que a inspeção passou a fazer. Sem ela aqui,
                // a inspeção dizia "não sabemos, então oferecemos" e a aplicação
                // respondia "este Windows não tem o recurso" — afirmando, na
                // hora de agir, exatamente o que se acabou de admitir não saber.
                //
                // O ciclo real contra a máquina pegou esta contradição: o item
                // virou disponível pela correção da inspeção e falhou aqui com a
                // mensagem antiga.
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

                // O Windows recusa mexer no Armazenamento Reservado quando há
                // atualização em andamento — e nem sempre pela via do erro. Sem
                // reler, a tela prometia vários GB de volta que não voltaram.
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

                // O vetor vai por referência: se a segunda chave falhar, a
                // primeira — já gravada — continua no histórico e a reversão
                // automática a desfaz.
                acessibilidade::desligar(changes)?;

                // O Windows guarda estas preferências em memória desde o logon,
                // como as do mouse. Sem o aviso, o teclado continuaria atrasando
                // até o próximo logon — numa otimização que promete efeito
                // imediato.
                sysparams::sincronizar_interface();

                Ok(Some(format!("{} desligada(s).", ligadas.join(", "))))
            }

            Action::DisableHypervisor => {
                let Some(anterior) = power::hypervisor_launch_type() else {
                    // Sem conseguir ler o estado atual não há como prometer a
                    // volta, e mexer sem poder reverter está fora de questão.
                    //
                    // NEM "já estava bom" NEM falha: não sabemos. Marcar como
                    // aplicada diria que o hipervisor está desligado sobre uma
                    // leitura que não aconteceu.
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

                // Reaproveita o registro dos limites de boot: ele já sabe
                // devolver um valor do `bcdedit` ao que estava.
                changes.push(ChangeRecord::BootLimits {
                    removed: vec![("hypervisorlaunchtype".to_string(), anterior)],
                });

                // O VALOR É LEGÍVEL NA HORA, mesmo o efeito só valendo no boot.
                // Em máquina com VBS imposto por política ou trancado em UEFI, o
                // `bcdedit` devolve 0 e o valor não fica — e era esse o caso em
                // que o cliente reiniciava, perdia o Hyper-V e não ganhava nada.
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
                    // Nenhum relógio forçado: a máquina já está saudável neste
                    // ponto, e é isso que o resultado precisa dizer.
                    detalhe.status = ActionStatus::AlreadyOptimized;
                    return Ok(None);
                };

                detalhe.before_value = Some(valor.clone());
                detalhe.expected_value = Some("ausente".to_string());

                shell::run_checked("bcdedit", &["/deletevalue", "{current}", "useplatformclock"])?;

                // Reaproveita o registro de limites de boot: a reversão dele já
                // sabe devolver um valor do bcdedit ao que estava.
                changes.push(ChangeRecord::BootLimits {
                    removed: vec![("useplatformclock".to_string(), valor.clone())],
                });

                // Apagar tem que ter apagado: `forced_platform_clock` devolve
                // `None` quando a linha não está mais lá, que é o alvo aqui.
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

                        // Esta ação escreve direto, sem passar pelo ramo
                        // `Action::Registry`, então precisa da mesma conferência
                        // por conta própria.
                        //
                        // O ERRO AQUI DERRUBA A OTIMIZAÇÃO INTEIRA, de propósito:
                        // Nagle meio desligado — numa placa sim e na outra não —
                        // é pior do que não mexer, porque a latência passa a
                        // depender de qual placa o Windows escolher.
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
        Some(Requirement::MinWddm(minimo)) => {
            hardware::alcanca_wddm(hardware::wddm_version(), minimo)
        }
    }
}

/// Por que uma otimização não é oferecida nesta máquina.
///
/// ENCONTRADO NA MÁQUINA, e não num teste. Este computador roda uma imagem
/// modificada onde o provedor WMI de armazenamento foi removido: tanto
/// `Get-PhysicalDisk` quanto `Get-Partition -DriveLetter C` voltam vazios.
/// `hardware::detect_system_storage` faz a coisa certa e devolve `Unknown` —
/// mas a frase de recusa dizia, textualmente, **"seu disco de sistema não é
/// SSD"**. Uma afirmação sobre o computador do cliente tirada de uma leitura
/// que falhou, na tela, a caminho da decisão de compra dele.
///
/// A RECUSA CONTINUA, e é o lado certo: desligar o SysMain num disco mecânico
/// piora a máquina, e sem saber o tipo do disco não dá para correr esse risco. O
/// que muda é a frase — ela passa a dizer a verdade sobre o que aconteceu.
pub fn motivo_da_recusa(requirement: catalog::Requirement) -> String {
    use catalog::Requirement;
    use hardware::StorageKind;

    match requirement {
        Requirement::SsdSystemDrive => match hardware::profile().system_storage {
            StorageKind::Hdd => "Não oferecemos: seu disco de sistema é mecânico, e aqui isso \
                                 deixaria o PC mais lento."
                .to_string(),
            // O caso desta máquina, e de qualquer Windows "lite" que tenha
            // tirado o provedor de armazenamento.
            StorageKind::Unknown => "Não oferecemos: não foi possível ler se o disco de sistema é \
                                     SSD ou mecânico nesta máquina. Em disco mecânico este ajuste \
                                     piora o PC, e sem saber o tipo não dá para arriscar."
                .to_string(),
            // Chegar aqui significaria que o requisito foi atendido e a
            // otimização foi recusada mesmo assim.
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

/// Quem manda nesta máquina além do dono dela.
///
/// Existe porque duas causas muito comuns de "não funcionou no PC do cliente"
/// não são defeito do produto nem do Windows — são de quem administra a máquina
/// e de quem montou a imagem. Sem separá-las, as duas chegavam ao atendimento
/// como "falhou", e o atendimento procurava no lugar errado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Governanca {
    /// Há política de grupo APLICADA — não apenas a chave existindo.
    pub com_politica_de_grupo: bool,
    /// A imagem do Windows foi montada por terceiros E tem serviço essencial
    /// desativado.
    pub imagem_de_terceiros: bool,
}

/// Lida uma vez por execução: são leituras de registro baratas, mas o motor
/// consulta isto uma vez por AÇÃO, e são dezenas por lote.
static GOVERNANCA: std::sync::OnceLock<Governanca> = std::sync::OnceLock::new();

pub fn governanca() -> Governanca {
    *GOVERNANCA.get_or_init(|| {
        let checagem = essenciais::checar();

        Governanca {
            com_politica_de_grupo: ha_politica_aplicada(),
            // FABRICANTE DECLARADO NÃO BASTA: todo PC de marca declara um. O
            // sinal é o fabricante JUNTO de serviço essencial desligado, que é
            // imagem modificada e não Windows de fábrica. Mesma regra do
            // relatório de compatibilidade, e de propósito: duas definições da
            // mesma coisa divergiriam.
            imagem_de_terceiros: checagem.fabricante.is_some() && checagem.desativados > 0,
        }
    })
}

/// Há GPO aplicada nesta máquina?
///
/// A CHAVE EXISTIR NÃO É SINAL, e conferir isso valeu: nesta máquina, sem
/// domínio e sem política nenhuma, `Group Policy\History` EXISTE e está vazia.
/// Usar a existência teria acusado política de grupo em todo computador do
/// mundo. O sinal é ter subchave — cada uma é um objeto de política aplicado.
fn ha_politica_aplicada() -> bool {
    registry::subkeys(
        "HKLM",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Group Policy\History",
    )
    .map(|gpos| !gpos.is_empty())
    .unwrap_or(false)
}

/// Dá nome à causa quando ela é de quem administra a máquina, e não do produto.
///
/// Função pura, separada da leitura: é uma regra de produto, e regra de produto
/// precisa de teste que não dependa de uma máquina com domínio.
///
/// SÓ REFINA, NUNCA INVENTA. Um estado que já é conclusivo — aplicado, já estava
/// bom, falhou de vez — não vira outra coisa por causa do ambiente.
pub fn refinar_status(status: ActionStatus, g: Governanca) -> ActionStatus {
    match status {
        // O comando foi aceito e o valor não ficou, numa máquina com GPO. É a
        // assinatura de política sobrescrevendo, e nomear isso poupa o cliente
        // de procurar defeito no Otimiza — não há.
        ActionStatus::VerificationFailed if g.com_politica_de_grupo => {
            ActionStatus::BlockedByPolicy
        }
        // O recurso não está aqui, e a imagem foi montada por terceiros. O
        // Windows TEM o recurso; esta instalação é que não.
        ActionStatus::Unsupported if g.imagem_de_terceiros => ActionStatus::UserOrOemManaged,
        outro => outro,
    }
}

/// Escreve, no resultado, a frase que o estado refinado passou a merecer.
///
/// Um estado novo sem frase nova não serve para nada: quem lê o relatório vê
/// `BlockedByPolicy` e continua sem saber o que fazer. A frase diz com quem
/// falar — e ela nomeia a IMAGEM quando há uma, porque "Team AntiLag / SnyX OS"
/// é uma informação que o cliente reconhece e que o atendimento pode pesquisar.
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

/// O nome de uma ação no resultado padronizado.
///
/// É PARA SER LIDO POR UMA PESSOA no atendimento, e por isso não é o `{:?}` do
/// enum: `Registry { hive: "HKLM", path: "SYSTEM\\…", name: "HwSchMode", … }`
/// tem a informação toda e é ilegível. O caminho e os valores já viajam nos
/// campos próprios do `ActionResult`.
pub fn nome_da_acao(action: &Action) -> String {
    match action {
        // Para o registro, o NOME DO VALOR é o que identifica a ação — é ele
        // que se pesquisa quando se quer saber o que aquela chave faz.
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
        Action::CleanTempFiles => "arquivos temporários".to_string(),
        Action::CleanUpdateCache => "instaladores de atualização".to_string(),
        Action::AccessibilityKeysOff => "teclas de acessibilidade".to_string(),
        Action::DisableHypervisor => "hipervisor no boot".to_string(),
        Action::RemoveForcedPlatformClock => "relógio de plataforma forçado".to_string(),
    }
}

/// A otimização só mostra efeito depois de sair e entrar na conta?
///
/// DERIVADO, e não um campo novo no catálogo. `sysparams::nota_de_ativacao` já
/// sabe quais chaves o shell só relê ao iniciar — é a mesma tabela que escreve
/// a frase mostrada ao cliente. Declarar de novo no catálogo criaria uma
/// segunda fonte, e duas fontes divergem no primeiro conserto.
pub fn exige_logoff(spec: &catalog::OptimizationSpec) -> bool {
    spec.actions.iter().any(|action| match action {
        Action::Registry {
            hive, path, name, ..
        } => sysparams::nota_de_ativacao(hive, path, name).is_some(),
        _ => false,
    })
}

/// O que a releitura de uma escrita de registro diz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirmacao {
    /// O valor lido é o que pedimos. Só aqui a otimização está provada.
    Igual,
    /// O Windows aceitou a gravação e o valor é OUTRO.
    Diferente(String),
    /// Não deu para reler. Não é falha, e não é confirmação.
    NaoDeuParaLer,
}

/// Confere uma escrita de registro relendo o valor.
///
/// POR QUE ISTO NÃO É PARANOIA. `set_dword` devolver `Ok` diz que o Windows
/// aceitou o pedido, não que o valor ficou. Dois casos reais, os dois na
/// máquina do cliente e nenhum na de desenvolvimento:
///
/// - política de domínio reescrevendo a chave logo depois;
/// - processo de 32 bits sobre Windows de 64, onde a escrita cai no espelho
///   `WOW6432Node` e o sistema continua lendo a chave verdadeira.
///
/// Nos dois o produto dizia "aplicado" sobre um PC que não mudou — que é
/// exatamente a queixa que abriu este trabalho.
///
/// Função pura, separada da execução para poder ser testada.
pub fn conferir_escrita(alvo: &RegValue, lido: &Result<PreviousValue, String>) -> Confirmacao {
    let Ok(atual) = lido else {
        return Confirmacao::NaoDeuParaLer;
    };

    let igual = match (alvo, atual) {
        (RegValue::Dword(esperado), PreviousValue::Dword(v)) => v == esperado,
        (RegValue::Text(esperado), PreviousValue::Text(v)) => v == esperado,
        (RegValue::Binary(esperado), PreviousValue::Binary(v)) => v.as_slice() == *esperado,
        // TIPO DIFERENTE É VALOR DIFERENTE. Gravamos DWORD e lemos texto quando
        // a chave é de um tipo que o sistema impõe — o número entrou como outra
        // coisa, e o Windows não vai lê-lo como nós queríamos.
        _ => false,
    };

    if igual {
        Confirmacao::Igual
    } else {
        Confirmacao::Diferente(descrever_valor(atual))
    }
}

/// A mesma conferência da escrita de registro, para tudo que NÃO é registro.
///
/// Serviços, `bcdedit`, MSI da placa, hibernação e compressão de memória
/// passavam pelo código de saída do comando e nada mais. `sc config` devolver 0
/// diz que o Gerenciador de Serviços aceitou o pedido — e numa máquina com
/// política de domínio, ou com o serviço trancado pelo próprio Windows, o tipo
/// de inicialização volta ao que era. O produto anotava no histórico uma
/// mudança que não existia, e o cliente depois mandava desfazer algo que nunca
/// foi feito.
///
/// `None` em `lido` é "não deu para reler", e não "diferente": ver
/// `exigir_confirmacao`.
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

/// O que fazer com o resultado de `conferir`, em uma regra só.
///
/// Existe para que as nove ações não-registro não escrevam nove versões
/// ligeiramente diferentes da mesma decisão — que é como uma delas acaba
/// tratando "não consegui ler" como sucesso.
pub fn exigir_confirmacao(c: Confirmacao, o_que: &str) -> Result<Option<String>, String> {
    match c {
        Confirmacao::Igual => Ok(None),
        Confirmacao::Diferente(atual) => Err(format!(
            "O Windows aceitou o comando, mas ao reler {} continua {}. Nada ficou aplicado.",
            o_que, atual
        )),
        // Não desfaz: a mudança provavelmente valeu, e descartá-la por causa de
        // uma leitura que falhou seria trocar um erro por outro. Mas também não
        // passa calado.
        Confirmacao::NaoDeuParaLer => Ok(Some(format!(
            "{} foi alterado, mas não foi possível reler para confirmar.",
            o_que
        ))),
    }
}

/// O valor que a otimização vai gravar, na mesma forma do que foi lido.
///
/// Mesma função de impressão dos dois lados de propósito: o cliente compara
/// "antes" com "esperado" olhando, e duas formatações diferentes para o mesmo
/// número fazem parecer que mudou quando não mudou.
pub fn descrever_alvo(valor: &RegValue) -> String {
    match valor {
        RegValue::Dword(v) => v.to_string(),
        RegValue::Text(v) => format!("\"{}\"", v),
        RegValue::Binary(bytes) => format!("{} byte(s)", bytes.len()),
    }
}

/// Como o valor lido aparece na mensagem de erro. Curto: ele vai para uma frase
/// que o cliente lê na tela, não para um despejo de memória.
fn descrever_valor(valor: &PreviousValue) -> String {
    match valor {
        PreviousValue::Dword(v) => v.to_string(),
        PreviousValue::Text(v) => format!("\"{}\"", v),
        PreviousValue::Binary(bytes) => format!("{} byte(s)", bytes.len()),
        PreviousValue::Absent => "inexistente".to_string(),
        PreviousValue::AbsentKey => "inexistente (a chave sumiu)".to_string(),
    }
}

/// A linha que o registro ao vivo mostra depois de montar o plano.
///
/// Ela diz os quatro números porque um "pronto" sozinho seria o que este
/// produto acusa nos concorrentes. "Já estava bom" não é enfeite: num PC que já
/// usava um plano de desempenho ele é a resposta inteira, e o cliente merece
/// saber disso em vez de achar que comprou um ganho que não houve.
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

/// A linha de fim de uma aplicação ou de um desfazer, com a duração.
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

/// Desfaz uma lista de mudanças na ordem inversa em que foram aplicadas.
/// Tenta reverter todas mesmo se alguma falhar, e devolve as falhas acumuladas.
fn revert_changes(changes: &[ChangeRecord]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    // Desfazer também precisa valer na hora. Sem isto, "Desfazer" devolveria o
    // registro e deixaria a sessão com o comportamento que o cliente pediu para
    // remover — o mesmo defeito, espelhado.
    let mut sincronizar_interface = false;

    for change in changes.iter().rev() {
        // Antes, como na aplicação: um desfazer que trava deixa dito qual foi.
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

            // Pelo `planoenergia`, e não pelo `set_active_scheme` cru, por duas
            // razões: ele CONFERE relendo qual plano ficou ativo — o `powercfg`
            // devolve zero e não é prova —, e apaga o plano OTIMIZA depois de a
            // volta estar confirmada, para não deixar plano nosso parado na
            // máquina de quem desfez.
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

            // O ÚNICO RAMO QUE VOLTA POR UMA CHAMADA DO FABRICANTE.
            //
            // Todos os outros aqui reescrevem um valor que o Otimiza anotou.
            // Este pergunta à própria NVIDIA qual era o padrão de fábrica e
            // volta para ele — a diferença entre "reversível de verdade" e
            // "reversível se a gente anotar direitinho", que é o que decidiu
            // este pilar. O caso em que o cliente já tinha uma escolha própria
            // no ajuste continua voltando escrito, e quem separa os dois é o
            // `nvdriver::desfazer`.
            ChangeRecord::DriverNvidia {
                opcao,
                valor_anterior,
            } => nvdriver::desfazer(opcao, valor_anterior),

            // O limite de um jogo. Perfil criado pelo Otimiza é apagado
            // inteiro; perfil que já existia só tem o limite devolvido — quem
            // separa os dois é o `nvdriver::desfazer_limite`.
            ChangeRecord::LimiteNvidia {
                executavel,
                perfil_criado,
                valor_anterior,
                ..
            } => nvdriver::desfazer_limite(executavel, *perfil_criado, valor_anterior),

            // O arquivo do jogo volta INTEIRO ao que era.
            //
            // Sem `anterior`, o arquivo não existia antes de o Otimiza mexer, e
            // desfazer é apagá-lo. Apagar um arquivo que já não está lá não é
            // falha: o estado desejado — ele não existir — já é o estado atual.
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

    // Depois de restaurar tudo, e uma vez só: a sincronização lê o registro já
    // devolvido ao estado original.
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

    // ------------------------------------------ o que a leitura falha vira

    use ActionState::{Desconhecido, NotApplicable, Pending, Satisfied};

    // ------------------------------------ provar que a escrita ficou de pé

    // ------------------------------------------ o resultado padronizado

    /// Aplica uma otimização de registro REAL e imprime o resultado
    /// padronizado, depois desfaz.
    ///
    /// `disable_startup_delay` é a escolhida por ser em `HKCU`, invisível,
    /// instantânea e reversível — ela muda um atraso de alguns segundos ao
    /// entrar na conta, e nada mais.
    ///
    /// `#[ignore]`: escreve no registro da máquina que roda o teste.
    ///
    ///   cargo test --lib resultado_padronizado -- --ignored --nocapture
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

        // Devolve a máquina ao que estava, tenha a aplicação dado certo ou não.
        if aplicar.is_ok() {
            let desfazer = otimizador.revert(id, &mut log);
            println!("desfazer: {:?}", desfazer.map(|r| r.message));
        }
    }

    // ----------------------------------- quem manda na máquina além do dono

    const SEM_GOVERNANCA: Governanca = Governanca {
        com_politica_de_grupo: false,
        imagem_de_terceiros: false,
    };

    /// O catálogo INTEIRO visto por esta máquina, item a item. SÓ LÊ.
    ///
    /// É o mais perto que dá para chegar da pergunta "o que aconteceria no PC
    /// do cliente" sem aplicar nada. Cada item aparece com o estado que a lista
    /// mostraria e com o detalhe medido — e o que interessa não são os
    /// `Available`, é tudo o que NÃO é: `Unavailable` diz que o produto se
    /// recusa a oferecer, e `Unknown` diz que ele não conseguiu nem olhar.
    ///
    ///   cargo test --lib catalogo_visto_por -- --ignored --nocapture
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

    /// O que o produto conclui sobre QUEM MANDA nesta máquina. Só lê.
    ///
    ///   cargo test --lib governanca_desta -- --ignored --nocapture
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
        // ACUSAR POLÍTICA SEM POLÍTICA mandaria o cliente falar com um
        // administrador que não existe — e faria o produto parecer que sabe de
        // algo que não sabe.
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
        // "Este Windows não tem" e "quem montou o seu Windows tirou" são duas
        // frases muito diferentes para o cliente: a segunda explica por que o
        // mesmo PC, com um Windows normal, se comportaria de outro jeito.
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
        // Só dá nome à causa; não muda o que aconteceu. Um ajuste aplicado numa
        // máquina com GPO continua aplicado.
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
        // O motor e o relatório de laboratório classificam com o MESMO
        // vocabulário. Se um dos dois perder um termo, o relatório do cliente
        // deixa de se agrupar com o do laboratório — que é a razão de o
        // relatório existir.
        //
        // `partial` e `requires restart/logoff` ficam de fora aqui de propósito:
        // o primeiro é do conjunto, não da ação, e o segundo é CAMPO no
        // resultado, porque uma ação pode ter sido aplicada E exigir reinício.
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
        // Não é problema do cliente nem do produto, e pintar de vermelho manda
        // procurar defeito onde não há. É a mesma regra que o plano de energia
        // já seguia, agora no vocabulário comum.
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
        // O `{:?}` do enum tem a informação toda e é ilegível. O caminho e os
        // valores viajam nos campos próprios do resultado.
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
        // Trava de forma: uma ação nova sem nome sairia com o nome de outra, ou
        // vazia, justamente no relatório que o cliente manda quando algo falha.
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
        // A tabela que sabe quais chaves o shell só relê ao iniciar é a do
        // `sysparams`, e ela já escreve a frase mostrada ao cliente. Se algum
        // item do catálogo mexe numa dessas chaves, o resultado precisa dizer
        // que exige logoff — sem ninguém ter declarado isso à mão.
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
        // O cliente compara as duas colunas olhando. Duas formatações
        // diferentes para o mesmo número fazem parecer que mudou quando não
        // mudou.
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
        // `None` é "não deu para reler". Se virasse `Diferente`, o produto
        // desfaria uma mudança que provavelmente valeu, por causa de uma
        // leitura que falhou.
        assert_eq!(conferir(&true, None::<bool>), Confirmacao::NaoDeuParaLer);
    }

    #[test]
    fn a_regra_da_confirmacao_e_uma_so() {
        // Nove ações usam esta função. Se cada uma escrevesse a própria versão
        // da decisão, uma delas acabaria tratando "não consegui ler" como
        // sucesso — que é exatamente o defeito que o produto passou três
        // versões consertando em outros lugares.
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
        // TRAVA DE FORMA, e não de ocorrência — o mesmo recurso da guarda de
        // prosa do `commands.rs`. Ela procura, no corpo do `execute`, ramos que
        // terminam em `Ok(None)` logo depois de empilhar uma mudança no
        // histórico: gravar e sair sem reler é exatamente o defeito que este
        // trabalho fechou, e é o que a ação número dez vai fazer por descuido.
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

            // PARA OS DOIS LADOS, E NÃO SÓ PARA A FRENTE.
            //
            // Conferir ANTES de empilhar é um padrão válido — e no ramo do plano
            // de energia é o único correto: não se grava registro de desfazer
            // para uma troca que ainda não se provou que aconteceu. Olhando só
            // adiante, esta trava acusou esse ramo de não conferir, e foi um
            // edit sem relação que deslocou as linhas e revelou a fragilidade.
            //
            // A janela é uma aproximação do ramo do `match`; achar a fronteira
            // exata por texto seria mais frágil do que o problema que resolve.
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
                // O plano de energia confere relendo qual plano ficou ativo,
                // dentro do `montar`. Ver `planoenergia::montar`.
                || janela.contains("plano_ativo");

            if !confere {
                // O trecho, e não só a contagem: um teste que diz "há 1
                // problema" e não diz onde custa a mesma busca toda vez.
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
        // O CASO DA MÁQUINA GERENCIADA: a gravação volta `Ok`, a política de
        // domínio reescreve a chave, e o produto dizia "aplicado" sobre um PC
        // que não mudou.
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Ok(PreviousValue::Dword(1))),
            Confirmacao::Diferente("1".to_string())
        );
    }

    #[test]
    fn valor_que_sumiu_depois_da_escrita_nao_passa() {
        // O caso do espelho `WOW6432Node`: escrevemos num lugar e o sistema lê
        // outro, onde continua não havendo nada.
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
        // O número entrou como texto: o Windows não vai lê-lo como nós
        // queríamos, e "2" não é 2.
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Ok(PreviousValue::Text("2".into()))),
            Confirmacao::Diferente("\"2\"".to_string())
        );
    }

    #[test]
    fn nao_conseguir_reler_nao_e_falha_nem_confirmacao() {
        // Desfazer aqui descartaria uma mudança que provavelmente valeu; dar
        // por confirmado afirmaria o que não foi lido. O terceiro estado existe
        // para não ter que escolher entre os dois erros.
        assert_eq!(
            conferir_escrita(&RegValue::Dword(2), &Err("acesso negado".into())),
            Confirmacao::NaoDeuParaLer
        );
    }

    /// A conferência contra o REGISTRO DE VERDADE, e não contra um valor
    /// montado à mão.
    ///
    /// O que os testes puros acima não cobrem é o encontro das duas pontas: o
    /// tipo que `set_dword` grava precisa ser o mesmo que `read` devolve, senão
    /// a conferência reprovaria TODA otimização de registro do catálogo por um
    /// detalhe de tipo — um estrago bem maior que o defeito que ela conserta.
    ///
    /// Escreve numa chave de rascunho nossa, confere, e apaga a chave inteira.
    /// `#[ignore]`: toca no registro da máquina que roda o teste.
    ///
    ///   cargo test --lib conferencia_contra_o_registro -- --ignored --nocapture
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
        // O DEFEITO QUE ESTE ESTADO VEIO CONSERTAR. Antes, `Err` na leitura do
        // registro caía em `NotApplicable`, e o cliente lia "não se aplica a
        // esta máquina" — uma afirmação sobre o PC dele que ninguém verificou.
        assert_eq!(
            WindowsOptimizer::compor(&[Desconhecido]),
            OptimizationState::Unknown
        );
    }

    #[test]
    fn leitura_que_falhou_nao_vira_ja_esta_bom() {
        // A mentira mais cara das duas: dizer "seu PC já está assim" faz o
        // cliente PARAR DE PROCURAR.
        assert_eq!(
            WindowsOptimizer::compor(&[Satisfied, Desconhecido]),
            OptimizationState::Unknown
        );
    }

    #[test]
    fn saber_que_ha_o_que_fazer_vence_o_desconhecimento() {
        // O outro lado do erro: esconder atrás de "não deu para verificar" uma
        // otimização real, por causa de uma leitura alheia que falhou.
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
        // Trava de não-regressão: o quarto estado não pode ter mudado o que o
        // produto já respondia certo.
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
        // O "Otimizar agora" filtra por `Available`. Este teste existe para que
        // alguém que um dia afrouxe esse filtro para incluir `Unknown` tenha
        // que encarar a decisão: seria aplicar, sem revisão, o que o produto
        // não conseguiu ler.
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

    /// O ramo do driver NVIDIA existe e chama o `nvdriver` DE VERDADE.
    ///
    /// NENHUM TESTE DESTA SUÍTE PODE ESCREVER NO DRIVER: a máquina que roda os
    /// testes é a do dono, e a esteira roda em runner sem placa NVIDIA. Então a
    /// prova é feita com um ajuste que não existe no catálogo — o `nvdriver`
    /// recusa pelo nome antes de abrir qualquer sessão da NVAPI, e o erro sobe
    /// por este ramo.
    ///
    /// O QUE ISSO PEGA: um ramo que devolvesse `Ok(())` sem chamar nada — o
    /// "desfazer" que não desfaz, o pior defeito possível neste produto — e um
    /// ramo ligado no módulo errado. O que não pega é o desfazer com um ajuste
    /// de verdade: esse é o Passo 5 do plano, na máquina, com o Painel de
    /// Controle da NVIDIA aberto.
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

    /// Desfazer a configuração de um jogo tem que devolver o arquivo BYTE A BYTE.
    ///
    /// É o teste que sustenta o produto passar a escrever no jogo. Enquanto ele
    /// só mexia no registro, cada mudança era um par de chave e valor com dono
    /// conhecido; um arquivo de configuração é do jogo, e devolver "quase" o que
    /// era deixaria o cliente com um arquivo que não é nem o de antes nem o de
    /// agora.
    #[test]
    fn desfazer_a_configuracao_do_jogo_devolve_o_arquivo_inteiro() {
        let dir = std::env::temp_dir().join(format!("otz-conf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("criar pasta de teste");
        let alvo = dir.join("settings.xml");

        // Um arquivo com acento, aspas e quebra de linha do Windows: se a volta
        // passar por alguma conversão de texto, é aqui que aparece.
        let original = "<Settings>\r\n  <MSAA value=\"4\" />\r\n  <!-- resolução -->\r\n</Settings>\r\n";
        std::fs::write(&alvo, original).expect("escrever o original");

        let registro = ChangeRecord::GameConfig {
            caminho: alvo.to_string_lossy().to_string(),
            anterior: Some(original.to_string()),
            jogo: "FiveM".to_string(),
        };

        // O "jogo" é alterado, como o Pilar 1 fará.
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

    /// Arquivo que não existia antes: desfazer é apagá-lo, e apagar duas vezes
    /// não é falha.
    ///
    /// O segundo caso importa porque `revert_all` pode passar pelo mesmo
    /// registro depois de uma reversão parcial que já tinha limpado o arquivo.
    /// Tratar "já não está lá" como erro faria a tela acusar falha de uma
    /// reversão que deu certo.
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
        let mut recusadas = 0;

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

            let meio = match executar(primeiro, &mut log) {
                Ok(resultado) => resultado,

                // RECUSA HONESTA NÃO É FALHA DO CICLO.
                //
                // Duas coisas acontecem nesta máquina, e nas duas o produto age
                // certo: o Windows nega informar o Armazenamento Reservado, e
                // nega a escrita na política dos Widgets — as duas mesmo com o
                // programa elevado. Em ambas o Otimiza explica e NÃO altera
                // nada, que é exatamente a regra que este ciclo existe para
                // proteger.
                //
                // Tratar por categoria, e não por lista de exceções: lista de
                // ids envelhece e vira teste que ignora tudo que incomoda. O
                // critério é o contrato da mensagem — se o produto recusou
                // agir, ele precisa dizer que nada foi alterado.
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

    /// Se o erro é o produto recusando agir, e dizendo que não alterou nada.
    ///
    /// É o CONTRATO das mensagens de recusa, e existe como função para poder
    /// ser testado: um `contains` solto dentro do ciclo viraria uma peneira
    /// invisível, que passa a aceitar falha de verdade no dia em que alguém
    /// escrever a frase errada.
    ///
    /// A frase é obrigatória porque é ela que separa "não fiz, e o sistema está
    /// intacto" de "falhei no meio". O ciclo pode tolerar a primeira; a segunda
    /// é exatamente o que ele existe para pegar.
    fn e_recusa_honesta(erro: &str) -> bool {
        erro.to_lowercase().contains("nada foi alterado")
    }

    #[test]
    fn recusa_honesta_exige_dizer_que_nada_mudou() {
        // As duas recusas reais desta máquina, com o texto que o produto
        // realmente emite.
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
        // O caso que o ciclo existe para pegar: algo quebrou DEPOIS de mexer.
        // Sem a frase, não é recusa — é falha, e tem que derrubar o teste.
        assert!(!e_recusa_honesta("Falha ao reverter `X`: acesso negado"));
        assert!(!e_recusa_honesta("Este ajuste não existe neste Windows"));
        assert!(!e_recusa_honesta(""));
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
