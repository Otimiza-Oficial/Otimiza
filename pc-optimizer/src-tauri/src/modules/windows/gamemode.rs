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

use super::{fivem, power};
use crate::modules::changelog::{now_timestamp, AppliedOptimization, ChangeLog, ChangeRecord};
use serde::{Deserialize, Serialize};

/// Identificador do registro no histórico.
///
/// Fixo: se o programa morrer com o modo ligado, é por este id que a próxima
/// execução encontra o que ficou aplicado.
const ID: &str = "gamemode:energia";

/// Jogos reconhecidos.
///
/// Pedaço do nome do processo e nome visível. A lista é curta e explícita — um
/// detector genérico de "parece um jogo" erraria, e errar aqui significa mexer
/// na energia da máquina porque alguém abriu um programa qualquer.
pub const JOGOS: &[(&str, &str)] = &[
    ("gtaprocess", "FiveM"),
    ("redm", "RedM"),
    ("gta5", "GTA V"),
    ("cs2", "Counter-Strike 2"),
    ("valorant", "Valorant"),
];

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
///
/// A comparação é por pedaço do nome porque o processo do FiveM carrega o
/// número da compilação: `FiveM_b3570_GTAProcess.exe`.
pub fn jogo_aberto() -> Option<&'static str> {
    use sysinfo::System;

    let mut sistema = System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for processo in sistema.processes().values() {
        let nome = processo.name().to_string_lossy().to_lowercase();

        if let Some((_, visivel)) = JOGOS.iter().find(|(chave, _)| nome.contains(chave)) {
            return Some(visivel);
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
    match fivem::priorizar_jogo() {
        Ok(_) => feito.push("Jogo em prioridade alta no processador.".to_string()),
        Err(motivo) => feito.push(format!("Prioridade não aplicada: {}", motivo)),
    }

    Ok(feito)
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

    if !JOGOS.iter().any(|(chave, _)| nome.contains(chave)) {
        return Err(format!(
            "`{}` não está na lista de jogos conhecidos do Otimiza. Esta chave do registro é \
             um mecanismo conhecido de sequestro de execução, e por isso só aceita executável \
             que o programa reconhece.",
            executavel
        ));
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
        .find(|nome| {
            let minusculo = nome.to_lowercase();
            JOGOS.iter().any(|(chave, _)| minusculo.contains(chave))
        })
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
        // Um detector genérico de "parece um jogo" erraria, e errar aqui
        // significa mexer na energia da máquina porque alguém abriu um
        // programa qualquer.
        assert!(JOGOS.iter().any(|(chave, _)| *chave == "gtaprocess"));

        for (chave, visivel) in JOGOS {
            assert!(!chave.is_empty() && !visivel.is_empty());
            assert_eq!(
                *chave,
                chave.to_lowercase(),
                "`{}` precisa estar em minúsculas para casar com o nome do processo",
                chave
            );
        }
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

            assert!(
                erro.contains("não está na lista")
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
