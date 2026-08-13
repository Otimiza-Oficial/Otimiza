// Modo jogo automático
//
// O resto do produto espera o técnico clicar. Este módulo faz sozinho: percebe
// que um jogo abriu, aplica o que ajuda, e — a parte que quase ninguém faz —
// DESFAZ quando o jogo fecha.
//
// POR QUE DESFAZER É O PONTO INTEIRO
//
// Deixar a máquina em desempenho máximo o tempo todo é o que os "modos turbo"
// do mercado fazem, e tem preço: em notebook, gasta bateria e esquenta o dia
// inteiro por causa de duas horas de jogo à noite. Aplicar só enquanto o jogo
// está aberto entrega o mesmo ganho na hora que importa, sem cobrar o resto do
// dia.
//
// TRÊS REGRAS QUE ESTE MÓDULO NÃO QUEBRA
//
// 1. É desligado por padrão, e só liga se o usuário mandar. Um programa que
//    muda a configuração do sistema sozinho, sem avisar, é exatamente o que
//    este produto critica nos outros — mesmo quando a mudança é boa.
//
// 2. Tudo passa pelo histórico. Se o Otimiza fechar no meio, morrer ou o PC
//    desligar na tomada, a mudança continua registrada e o "Desfazer tudo"
//    devolve. Nada fica preso sem registro.
//
// 3. Não mata processo nenhum. "Fechar programas desnecessários" soa bem e é a
//    forma mais fácil de fazer alguém perder o trabalho não salvo. O que pesa
//    em segundo plano já está listado nas abas Sistema e Painel, com nome, para
//    a pessoa decidir.

use super::power;
use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeLog, ChangeRecord};
use serde::{Deserialize, Serialize};

/// Identificador do registro no histórico.
///
/// Fixo: se o programa morrer com o modo ligado, é por este id que a próxima
/// execução encontra o que ficou aplicado.
const ID: &str = "gamemode:energia";

/// Um jogo que o Otimiza sabe chamar pelo nome.
pub struct Jogo {
    /// Nome do executável, em minúsculas.
    pub chave: &'static str,
    pub nome: &'static str,
    /// Verdadeiro só quando o executável carrega o número da compilação no meio
    /// do nome e não há como comparar por igualdade.
    ///
    /// É a exceção, não a regra: comparação por pedaço casa com programa que
    /// não tem nada a ver. Ver o teste `nome_parecido_nao_e_jogo`.
    pub por_pedaco: bool,
}

/// Jogos reconhecidos pelo nome.
///
/// ATENÇÃO — esta lista NÃO é mais o gatilho do modo jogo, e sim o catálogo de
/// nome bonito. Até a versão 0.13 ela era a única forma de o produto saber que
/// um jogo abriu, o que deixava de fora tudo que não fosse GTA: das cinco
/// entradas de então, três eram da mesma família.
///
/// Um jogo fora desta lista continua sendo reconhecido pelos sinais medidos
/// (uso do motor 3D, janela em primeiro plano) — só não tem nome próprio.
pub const JOGOS: &[Jogo] = &[
    // Os dois da Cfx.re não têm nome fixo: o processo é
    // `FiveM_b3570_GTAProcess.exe` e muda a cada compilação. As chaves incluem
    // o sublinhado para não casar com o lançador (`FiveM.exe`), que não é o
    // processo que desenha.
    Jogo { chave: "fivem_", nome: "FiveM", por_pedaco: true },
    Jogo { chave: "redm_", nome: "RedM", por_pedaco: true },

    Jogo { chave: "gta5.exe", nome: "GTA V", por_pedaco: false },
    Jogo { chave: "cs2.exe", nome: "Counter-Strike 2", por_pedaco: false },
    Jogo { chave: "valorant-win64-shipping.exe", nome: "Valorant", por_pedaco: false },
    Jogo { chave: "fortniteclient-win64-shipping.exe", nome: "Fortnite", por_pedaco: false },
    Jogo { chave: "robloxplayerbeta.exe", nome: "Roblox", por_pedaco: false },
    Jogo { chave: "minecraft.windows.exe", nome: "Minecraft", por_pedaco: false },
    Jogo { chave: "league of legends.exe", nome: "League of Legends", por_pedaco: false },
    Jogo { chave: "rocketleague.exe", nome: "Rocket League", por_pedaco: false },
    Jogo { chave: "r5apex.exe", nome: "Apex Legends", por_pedaco: false },
    Jogo { chave: "rainbowsix.exe", nome: "Rainbow Six Siege", por_pedaco: false },
    Jogo { chave: "tslgame.exe", nome: "PUBG", por_pedaco: false },
    Jogo { chave: "dota2.exe", nome: "Dota 2", por_pedaco: false },
    Jogo { chave: "csgo.exe", nome: "CS:GO", por_pedaco: false },
];

/// O nome do jogo, se este executável for um jogo conhecido.
///
/// Função pura, separada da varredura de processos para poder ser testada sem
/// depender do que está aberto na máquina.
pub fn nome_do_jogo(executavel: &str) -> Option<&'static str> {
    let nome = executavel.trim().to_lowercase();

    if nome.is_empty() {
        return None;
    }

    JOGOS
        .iter()
        .find(|jogo| {
            if jogo.por_pedaco {
                nome.contains(jogo.chave)
            } else {
                nome == jogo.chave
            }
        })
        .map(|jogo| jogo.nome)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameModeStatus {
    /// Um jogo conhecido está aberto agora.
    pub game_running: bool,
    /// Nome visível do jogo detectado.
    pub game: Option<String>,
    /// O modo está aplicado neste momento.
    pub active: bool,
    /// O que foi feito, em português, para a interface mostrar.
    pub applied: Vec<String>,
}

/// Procura um jogo conhecido entre os processos.
pub fn jogo_aberto() -> Option<String> {
    jogo_aberto_com_pid().map(|(nome, _)| nome)
}

/// O jogo aberto e o identificador do processo dele.
///
/// O PID é o que permite dar prioridade ao jogo certo. Até a versão 0.13 a
/// prioridade era pedida a `fivem::priorizar_jogo()`, que procurava o processo
/// por um filtro literal `FiveM*GTAProcess*` — então o modo jogo detectava
/// Counter-Strike, chamava aquela função, e ela respondia "o jogo não está
/// aberto" com o jogo aberto na frente do cliente.
pub fn jogo_aberto_com_pid() -> Option<(String, u32)> {
    // PRIMEIRO os sinais medidos: janela cobrindo o monitor, motor 3D em uso,
    // tempo de vida. É o que reconhece jogo que ninguém cadastrou — Palworld,
    // um indie que saiu ontem, um emulador.
    if let Some(detectado) = super::deteccao::procurar() {
        return Some((detectado.nome, detectado.pid));
    }

    // DEPOIS a lista de nomes, como rede de apoio. Ela pega o caso em que o
    // jogo está aberto mas não em primeiro plano — o cliente deu alt-tab para
    // olhar o Discord —, situação em que os sinais de janela não fecham e o
    // modo jogo não deveria desligar por isso.
    use sysinfo::System;

    let mut sistema = System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for (pid, processo) in sistema.processes() {
        let executavel = processo.name().to_string_lossy().to_string();

        if let Some(nome) = nome_do_jogo(&executavel) {
            return Some((nome.to_string(), pid.as_u32()));
        }
    }

    None
}

/// Liga o modo: plano de alto desempenho e prioridade para o jogo.
///
/// O plano de energia entra no histórico; a prioridade não, porque ela some
/// sozinha quando o processo do jogo fecha e não há o que reverter.
pub fn ativar(log: &mut ChangeLog) -> Result<Vec<String>, String> {
    if log.is_applied(ID) {
        return Err("O modo jogo já está aplicado.".to_string());
    }

    let mut feito = Vec::new();

    // Plano de energia: o único ajuste do modo que muda o sistema e precisa
    // voltar depois.
    //
    // A consulta ao anticheat aqui sempre autoriza, e a chamada existe de
    // propósito: plano de energia é configuração da máquina e não encosta em
    // processo nenhum. Deixar a decisão escrita no mesmo lugar das outras
    // impede que alguém, no futuro, endureça a política sem perceber que este
    // caminho existe — ou afrouxe achando que ele nunca foi avaliado.
    if let Some(recusa) = super::anticheat::permite(
        super::anticheat::Acao::PlanoDeEnergia,
        &super::anticheat::detectar_agora(),
    )
    .motivo()
    {
        return Err(recusa.to_string());
    }

    let anterior = power::active_scheme()?;

    if anterior != power::HIGH_PERFORMANCE_GUID {
        power::ensure_high_performance_exists()?;
        power::set_active_scheme(power::HIGH_PERFORMANCE_GUID)?;

        log.record(AppliedOptimization {
            optimization_id: ID.to_string(),
            name: "Modo jogo: plano de energia".to_string(),
            timestamp: now_timestamp(),
            changes: vec![ChangeRecord::PowerPlan { previous_guid: anterior }],
        })?;

        feito.push("Plano de alto desempenho ligado.".to_string());
    }

    // Prioridade é por sessão e some com o processo. Falhar aqui não derruba o
    // modo: o plano de energia já vale por si.
    match jogo_aberto_com_pid() {
        Some((nome, pid)) => {
            // Mudar a prioridade abre um handle NO PROCESSO DO JOGO — é a coisa
            // mais visível que o Otimiza faz para um anticheat. E o ganho é
            // pequeno: prioridade alta só muda alguma coisa quando há disputa
            // real de processador. Trocar risco de banimento por isso seria um
            // mau negócio para o cliente.
            let presencas = super::anticheat::detectar_agora();
            let permissao =
                super::anticheat::permite(super::anticheat::Acao::PrioridadeNoJogo, &presencas);

            match permissao.motivo() {
                Some(recusa) => feito.push(recusa.to_string()),
                None => match priorizar_pid(pid) {
                    Ok(_) => feito.push(format!("{} em prioridade alta no processador.", nome)),
                    Err(motivo) => feito.push(format!("Prioridade não aplicada: {}", motivo)),
                },
            }
        }
        None => feito.push(
            "Prioridade não aplicada: o jogo fechou entre a detecção e o ajuste.".to_string(),
        ),
    }

    Ok(feito)
}

/// Onde está, no disco, o executável com este nome — se ele estiver rodando.
///
/// Serve à trava do IFEO: o nome sozinho não diz nada sobre a origem do
/// programa, e é o caminho que separa o jogo instalado pela Steam de um
/// arquivo qualquer que alguém batizou com o mesmo nome.
fn caminho_do_executavel(nome: &str) -> Option<std::path::PathBuf> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

    let mut sistema = System::new();
    sistema.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );

    sistema
        .processes()
        .values()
        .find(|p| p.name().to_string_lossy().to_lowercase() == nome)
        .and_then(|p| p.exe().map(std::path::PathBuf::from))
}

/// Põe um processo em prioridade alta, pelo identificador.
///
/// Alta, e nunca tempo real: prioridade de tempo real põe o processo acima do
/// próprio Windows, e um jogo travado nessa faixa deixa a máquina sem teclado e
/// sem mouse. É a mesma recusa que `definir_prioridade_persistente` já faz.
///
/// Vale só para a sessão: some quando o processo fecha, então não há o que
/// registrar no histórico de mudanças.
pub fn priorizar_pid(pid: u32) -> Result<(), String> {
    // O PID entra formatado como número, nunca como texto vindo de fora — é o
    // que impede alguém de fazer o script executar outra coisa.
    let script = format!(
        "$p = Get-Process -Id {} -ErrorAction SilentlyContinue; \
         if ($p) {{ $p.PriorityClass = 'High'; 1 }} else {{ 0 }}",
        pid
    );

    let saida = super::shell::powershell(&script)?;

    if saida.stdout.trim() == "1" {
        Ok(())
    } else {
        Err("Reabra o Otimiza como administrador para ajustar a prioridade do jogo.".to_string())
    }
}

/// Desliga o modo, devolvendo o plano de energia que existia antes.
pub fn desativar(log: &mut ChangeLog) -> Result<String, String> {
    let Some(registro) = log.take(ID)? else {
        return Err("O modo jogo não está aplicado.".to_string());
    };

    for mudanca in &registro.changes {
        if let ChangeRecord::PowerPlan { previous_guid } = mudanca {
            power::set_active_scheme(previous_guid)?;
        }
    }

    Ok("Modo jogo desligado. O plano de energia voltou ao que era antes.".to_string())
}

/// Situação atual, para a interface.
pub fn status(log: &ChangeLog) -> GameModeStatus {
    let jogo = jogo_aberto();

    GameModeStatus {
        game_running: jogo.is_some(),
        game: jogo.map(|g| g.to_string()),
        active: log.is_applied(ID),
        applied: Vec::new(),
    }
}

// ------------------------------------------------- prioridade persistente

/// Onde o Windows guarda ajustes por executável.
const IFEO: &str =
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";

/// Prioridade alta. NUNCA 4, que é tempo real.
///
/// A escala aqui não é a mesma da API de processos: 1 é baixa, 2 normal, 3
/// alta, 4 tempo real, 5 abaixo do normal, 6 acima do normal. Tempo real
/// colocaria o jogo acima do próprio sistema operacional, incluindo o que cuida
/// de som, mouse e teclado, e o resultado prático é travar a máquina inteira.
const PRIORIDADE_ALTA: u32 = 3;

/// Ajusta a prioridade de um executável de forma permanente.
///
/// A prioridade dada ao processo em execução some quando ele fecha, e isso está
/// documentado em `fivem::priorizar_jogo`. Aqui ela passa a valer em toda
/// abertura, porque o Windows lê este ajuste ao criar o processo.
///
/// A ARMADILHA DE SEGURANÇA DESTE LUGAR
///
/// A mesma chave aceita um valor chamado `Debugger`, e quem escreve ali faz o
/// Windows abrir OUTRO programa no lugar do que foi pedido. É um mecanismo
/// clássico de sequestro de execução, usado por malware há décadas.
///
/// Por isso este código só escreve `CpuPriorityClass`, só dentro de
/// `PerfOptions`, e só para executável cujo nome bate com a lista de jogos
/// conhecidos. O comando é exposto por IPC: aceitar nome de arquivo vindo de
/// fora transformaria o Otimiza na ferramenta de sequestro.
pub fn definir_prioridade_persistente(
    executavel: &str,
    ativar: bool,
) -> Result<ChangeRecord, String> {
    if !super::registry::is_elevated() {
        return Err("Fixar a prioridade de um jogo exige executar como administrador.".to_string());
    }

    let nome = executavel.to_lowercase();

    // Nome de arquivo, e nada além disso. Barra ou dois-pontos aqui seria
    // tentativa de escrever fora do lugar previsto.
    if nome.contains(['\\', '/', ':']) || !nome.ends_with(".exe") {
        return Err("Nome de executável inválido.".to_string());
    }

    // A TRAVA, E POR QUE ELA MUDOU
    //
    // Até a versão 0.13 quem autorizava esta escrita era a lista de nomes de
    // jogo. Com a detecção genérica, a lista deixou de ser a fonte da verdade —
    // e usar o detector no lugar dela seria pior ainda: detector é heurística,
    // e heurística não pode virar autoridade de segurança numa chave que serve
    // para sequestrar a execução de programas.
    //
    // A trava agora é o CAMINHO: o executável precisa estar dentro de uma
    // biblioteca de jogo de verdade, declarada pela Steam ou pela Epic. Um
    // `sethc.exe` ou um `cmd.exe` nunca vai estar.
    let biblioteca = super::jogos::varrer();
    let dentro = caminho_do_executavel(&nome)
        .map(|caminho| super::jogos::dentro_de_biblioteca(&caminho, &biblioteca.raizes))
        .unwrap_or(false);

    if !dentro && nome_do_jogo(&nome).is_none() {
        return Err(format!(
            "`{}` não está numa pasta de jogo instalado nem na lista de jogos conhecidos do \
             Otimiza. Esta chave do registro é um mecanismo conhecido de sequestro de \
             execução, e por isso só aceita executável cuja origem o programa consegue \
             confirmar.",
            executavel
        ));
    }

    // Esta escrita deixa marca PERMANENTE no registro, na mesma chave usada
    // por programas que sequestram a execução de outros. Um anticheat de
    // núcleo tem todo o direito de estranhar — e ao contrário da prioridade de
    // sessão, aqui não adianta esperar o jogo fechar: a marca continua lá
    // quando ele abrir.
    let presencas = super::anticheat::detectar_agora();
    if let Some(recusa) =
        super::anticheat::permite(super::anticheat::Acao::EscreverIfeo, &presencas).motivo()
    {
        return Err(recusa.to_string());
    }

    let caminho = format!("{}\\{}\\PerfOptions", IFEO, executavel);
    let anterior = super::registry::read("HKLM", &caminho, "CpuPriorityClass")
        .unwrap_or(crate::modules::changelog::PreviousValue::AbsentKey);

    if ativar {
        super::registry::set_dword("HKLM", &caminho, "CpuPriorityClass", PRIORIDADE_ALTA)?;
    } else {
        super::registry::restore("HKLM", &caminho, "CpuPriorityClass", &anterior)?;
    }

    Ok(ChangeRecord::RegistryValue {
        hive: "HKLM".to_string(),
        path: caminho,
        name: "CpuPriorityClass".to_string(),
        previous: anterior,
    })
}

/// Nome do executável do jogo em execução, quando há um.
///
/// É preciso pegar o nome real porque o processo do FiveM carrega o número da
/// compilação: `FiveM_b3570_GTAProcess.exe`. Isso tem uma consequência que a
/// interface precisa dizer — quando o FiveM atualiza, o nome muda e o ajuste
/// precisa ser aplicado de novo.
pub fn executavel_do_jogo() -> Option<String> {
    use sysinfo::System;

    let mut sistema = System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    sistema
        .processes()
        .values()
        .map(|p| p.name().to_string_lossy().to_string())
        .find(|nome| nome_do_jogo(nome).is_some())
}

/// Um passo do vigia: liga quando o jogo abre, desliga quando ele fecha.
///
/// Devolve a mensagem quando alguma coisa mudou, e `None` quando não havia o
/// que fazer — assim a interface só é avisada quando há novidade.
pub fn passo(log: &mut ChangeLog) -> Option<String> {
    let jogo = jogo_aberto();
    let aplicado = log.is_applied(ID);

    match (jogo, aplicado) {
        (Some(nome), false) => {
            let mut feito = ativar(log).ok()?;

            // Suspender o segundo plano é o que devolve memória ao jogo — e é
            // a razão de este vigia existir numa máquina que trava por falta
            // de RAM. Falhar aqui não pode impedir o resto do modo jogo.
            if let Ok(suspensos) = super::suspend::suspender_fundo() {
                if !suspensos.is_empty() {
                    let nomes: Vec<&str> =
                        suspensos.iter().map(|s| s.visivel.as_str()).collect();
                    feito.push(format!(
                        "Pausei {} — voltam quando o jogo fechar.",
                        nomes.join(", ")
                    ));
                }
            }

            Some(format!("{} aberto. {}", nome, feito.join(" ")))
        }
        (None, true) => {
            // A ordem importa: devolver os programas ANTES de desfazer o resto.
            // Se algo falhar no meio, o cliente prefere ter o Discord de volta
            // com o plano de energia errado do que o contrário.
            let devolvidos = super::suspend::retomar_tudo().unwrap_or_default();
            let texto = desativar(log).ok()?;

            if devolvidos.is_empty() {
                Some(texto)
            } else {
                let nomes: Vec<&str> = devolvidos.iter().map(|s| s.visivel.as_str()).collect();
                Some(format!("{} {} de volta.", texto, nomes.join(", ")))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nao_mata_processo() {
        // "Fechar programas desnecessários" soa bem e é a forma mais fácil de
        // fazer alguém perder trabalho não salvo. O que pesa já está listado
        // nas abas Sistema e Painel, com nome, para a pessoa decidir.
        let fonte = include_str!("gamemode.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();

        for proibido in ["TerminateProcess", "Stop-Process", "taskkill", "EmptyWorkingSet"] {
            assert!(
                !producao.contains(proibido),
                "`{}` apareceu no modo jogo",
                proibido
            );
        }
    }

    #[test]
    fn a_lista_de_jogos_e_explicita() {
        for jogo in JOGOS {
            assert!(!jogo.chave.is_empty() && !jogo.nome.is_empty());
            assert_eq!(
                jogo.chave,
                jogo.chave.to_lowercase(),
                "`{}` precisa estar em minúsculas para casar com o nome do processo",
                jogo.chave
            );

            // Comparação por pedaço é a exceção perigosa: ela casa com
            // qualquer programa que contenha aquele texto no nome. Só é
            // aceitável quando o executável carrega número de compilação, e
            // nesses casos a chave termina em sublinhado justamente para não
            // casar com o lançador nem com nome solto.
            if jogo.por_pedaco {
                assert!(
                    jogo.chave.ends_with('_'),
                    "`{}` compara por pedaço sem terminar em sublinhado — cedo demais para casar",
                    jogo.chave
                );
            } else {
                assert!(
                    jogo.chave.ends_with(".exe"),
                    "`{}` compara por igualdade e precisa ser o nome completo do executável",
                    jogo.chave
                );
            }
        }
    }

    #[test]
    fn nome_parecido_nao_e_jogo() {
        // O defeito que este teste tranca: até a versão 0.13 a comparação era
        // `nome.contains(chave)` com a chave `cs2`, então QUALQUER programa com
        // "cs2" no nome ligava o modo jogo — e ligar o modo jogo muda o plano
        // de energia da máquina do cliente.
        for impostor in [
            "docs2pdf.exe",
            "nvcs2.exe",
            "redmine.exe",
            "redmond-sync.exe",
            "gta5-mod-manager-installer.exe",
            "fivem.exe", // o lançador não é o processo que desenha
        ] {
            assert_eq!(
                nome_do_jogo(impostor),
                None,
                "`{}` não é jogo e foi reconhecido como tal",
                impostor
            );
        }
    }

    #[test]
    fn reconhece_os_jogos_de_verdade() {
        assert_eq!(nome_do_jogo("cs2.exe"), Some("Counter-Strike 2"));
        assert_eq!(nome_do_jogo("CS2.exe"), Some("Counter-Strike 2"));
        assert_eq!(
            nome_do_jogo("FortniteClient-Win64-Shipping.exe"),
            Some("Fortnite")
        );
        assert_eq!(nome_do_jogo("RobloxPlayerBeta.exe"), Some("Roblox"));

        // Os dois da Cfx.re carregam o número da compilação no meio do nome, e
        // precisam continuar sendo distinguidos um do outro.
        assert_eq!(nome_do_jogo("FiveM_b3570_GTAProcess.exe"), Some("FiveM"));
        assert_eq!(nome_do_jogo("RedM_b1491_GTAProcess.exe"), Some("RedM"));

        assert_eq!(nome_do_jogo(""), None);
        assert_eq!(nome_do_jogo("   "), None);
    }

    #[test]
    fn a_prioridade_nao_e_mais_exclusiva_do_fivem() {
        // O defeito: `ativar()` pedia a prioridade a `fivem::priorizar_jogo()`,
        // cujo filtro era o literal `FiveM*GTAProcess*`. O modo jogo detectava
        // Counter-Strike, chamava aquela função, e ela respondia "o jogo não
        // está aberto" — com o jogo aberto na frente do cliente.
        // A verificação é pela IMPORTAÇÃO, e não pelo texto: o comentário que
        // documenta o defeito precisa continuar citando o nome da função
        // antiga, senão daqui a um ano ninguém entende por que este teste
        // existe.
        let producao = include_str!("gamemode.rs").split("#[cfg(test)]").next().unwrap();

        assert!(
            !producao.contains("use super::{fivem"),
            "o modo jogo voltou a importar o módulo do FiveM"
        );
        assert!(
            producao.contains("fn priorizar_pid"),
            "a prioridade precisa ser dada pelo identificador do processo detectado"
        );
    }

    #[test]
    fn a_recusa_por_origem_esta_escrita_no_texto_da_mensagem() {
        // Conferência que NÃO depende de privilégio, e por isso roda igual na
        // máquina de quem desenvolve e na esteira.
        //
        // O teste acima só alcança a recusa por origem quando roda elevado; se
        // a mensagem mudar de novo, ele passa verde localmente e quebra no
        // release — que foi exatamente o que aconteceu na versão 0.14.0.
        let producao = include_str!("gamemode.rs").split("#[cfg(test)]").next().unwrap();

        assert!(
            producao.contains("pasta de jogo instalado"),
            "a mensagem de recusa do IFEO mudou sem o teste acompanhar"
        );
        assert!(
            producao.contains("dentro_de_biblioteca"),
            "a trava do IFEO precisa conferir a pasta de origem do executável"
        );
    }

    #[test]
    fn nunca_escreve_tempo_real_nem_depurador() {
        // As duas travas deste módulo. Tempo real põe o jogo acima do sistema
        // operacional; `Debugger` na mesma chave faz o Windows abrir outro
        // programa no lugar do pedido, que é sequestro de execução.
        let producao = include_str!("gamemode.rs").split("#[cfg(test)]").next().unwrap();

        assert_eq!(PRIORIDADE_ALTA, 3);
        assert!(!producao.contains("PRIORIDADE_ALTA: u32 = 4"));
        assert!(
            !producao.contains("\"Debugger\""),
            "escrita de Debugger no IFEO nunca pode entrar aqui"
        );
    }

    #[test]
    fn executavel_de_fora_da_lista_e_recusado() {
        // O comando é exposto por IPC. Aceitar nome arbitrário transformaria o
        // Otimiza na ferramenta de sequestro.
        for tentativa in [
            "notepad.exe",
            r"..\..\malicioso.exe",
            r"C:\jogos\gtaprocess.exe",
            "gtaprocess",
        ] {
            let erro = definir_prioridade_persistente(tentativa, true).unwrap_err();

            // A recusa por ORIGEM ("não está numa pasta de jogo instalado") é a
            // que vale: ela só é exercitada com privilégio de administrador,
            // porque sem ele a função para antes, no primeiro if. Foi assim que
            // este teste passou verde na máquina de quem desenvolve e quebrou
            // na esteira, que roda elevada — o teste conferia o caminho fácil.
            assert!(
                erro.contains("pasta de jogo instalado")
                    || erro.contains("inválido")
                    || erro.contains("administrador"),
                "recusa inesperada para `{}`: {}",
                tentativa,
                erro
            );
        }
    }

    #[test]
    fn nome_do_executavel_do_jogo() {
        let nome = executavel_do_jogo();
        println!("executável do jogo agora: {:?}", nome);

        if let Some(n) = nome {
            assert!(n.to_lowercase().ends_with(".exe"));
        }
    }

    #[test]
    fn desligar_sem_ter_ligado_e_recusado() {
        let mut log = ChangeLog::load();

        if !log.is_applied(ID) {
            let erro = desativar(&mut log).unwrap_err();
            assert!(erro.contains("não está aplicado"));
        }
    }

    #[test]
    fn o_vigia_nao_faz_nada_quando_nao_ha_mudanca() {
        let mut log = ChangeLog::load();

        let jogo = jogo_aberto();
        let aplicado = log.is_applied(ID);

        // Sem jogo aberto e sem modo aplicado, um passo do vigia não pode
        // mexer em nada: ele rodaria a cada poucos segundos, e um passo que
        // age à toa mexeria na energia da máquina o tempo todo.
        if jogo.is_none() && !aplicado {
            assert!(passo(&mut log).is_none());
            assert!(!log.is_applied(ID));
        }
    }

    #[test]
    fn detecta_jogo_nesta_maquina() {
        let jogo = jogo_aberto();
        println!("jogo aberto agora: {:?}", jogo);

        let s = status(&ChangeLog::load());
        assert_eq!(s.game_running, jogo.is_some());
    }
}
