// Modo jogo automático: percebe o jogo, aplica e DESFAZ quando ele fecha (modo turbo o dia inteiro gasta bateria
// e esquenta). Desligado por padrão; tudo passa pelo histórico; não mata processo nenhum.

use super::governador::{Devolucao, Governador, Modo, Passada};
use super::power;
use crate::modules::changelog::{ChangeLog, ChangeRecord};
use serde::{Deserialize, Serialize};

/// Fixo: se o programa morrer com o modo ligado, é por ele que a próxima execução acha o que ficou.
const ID: &str = "gamemode:energia";

pub struct Jogo {
    pub chave: &'static str,
    pub nome: &'static str,
    /// A exceção: comparação por pedaço casa com programa sem relação (ver `nome_parecido_nao_e_jogo`).
    pub por_pedaco: bool,
}

/// O catálogo de NOME BONITO, não o gatilho: jogo fora dela é reconhecido pelos sinais medidos.
pub const JOGOS: &[Jogo] = &[
    // O processo da Cfx.re muda de nome a cada compilação (`FiveM_b3570_GTAProcess.exe`). O sublinhado evita casar
    // com o lançador (`FiveM.exe`), que não desenha.
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

/// Pura: testável sem depender do que está aberto.
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
    pub game_running: bool,
    pub game: Option<String>,
    pub active: bool,
    pub applied: Vec<String>,
}

pub fn jogo_aberto() -> Option<String> {
    jogo_aberto_com_pid().map(|(nome, _)| nome)
}

pub fn jogo_aberto_com_pid() -> Option<(String, u32)> {
    // PRIMEIRO os sinais medidos: reconhecem o jogo que ninguém cadastrou.
    if let Some(detectado) = super::deteccao::procurar() {
        return Some((detectado.nome, detectado.pid));
    }

    // DEPOIS a lista, como apoio: com alt-tab os sinais de janela não fecham, e o modo não deve desligar por isso.
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

static GOVERNADOR: std::sync::Mutex<Option<Governador>> = std::sync::Mutex::new(None);

/// Mesmo envenenado: a lista do que foi acalmado continua valendo e é o que o desligar precisa. Tratar o veneno
/// como "sem governador" dizia "desligado" com programas ainda em prioridade baixa.
fn trava() -> std::sync::MutexGuard<'static, Option<Governador>> {
    GOVERNADOR.lock().unwrap_or_else(|envenenado| envenenado.into_inner())
}

/// **Pura.** FiveM e RedM trocam de nome a cada compilação: a chave deles é o pedaço fixo do catálogo.
pub fn chave_do_processo(executavel: &str) -> String {
    let nome = executavel.trim().to_lowercase();
    if let Some(jogo) = JOGOS.iter().find(|j| j.por_pedaco && nome.contains(j.chave)) {
        return jogo.chave.to_string();
    }
    nome.strip_suffix(".exe").unwrap_or(&nome).to_string()
}

fn executavel_do_pid(pid: u32) -> Option<String> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sistema = System::new();
    let alvo = [Pid::from_u32(pid)];
    sistema.refresh_processes_specifics(ProcessesToUpdate::Some(&alvo), true, ProcessRefreshKind::nothing().with_exe(sysinfo::UpdateKind::OnlyIfNotSet));
    let p = sistema.process(alvo[0])?;
    let do_caminho = p.exe().and_then(|c| c.file_name()).map(|a| a.to_string_lossy().to_string());
    do_caminho.or_else(|| Some(p.name().to_string_lossy().to_string()))
}

fn governador_para(jogo: &str, pid: u32) -> (Governador, Option<String>) {
    use crate::modules::portao::{self, Rodada};

    let Some(processo) = executavel_do_pid(pid).map(|e| chave_do_processo(&e)).filter(|p| !p.is_empty()) else {
        return (
            Governador::novo(Modo::Parado, None, pid),
            Some(format!(
                "O governador fica parado: não consegui identificar o executável de {} para medir o efeito dele.",
                jogo
            )),
        );
    };
    // Sem ler o que uma sessão anterior deixou acalmado, não mexe em nada nem anota a partida como "agindo".
    let sondagem = Governador::novo(Modo::Parado, Some(processo.clone()), pid);
    if let Some(e) = sondagem.anotacao_ilegivel() {
        let aviso = format!("O governador fica parado: {}. Sem conseguir ler o que uma sessão anterior acalmou, ele não mexe em nada.", e);
        return (sondagem, Some(aviso));
    }
    let medindo = crate::modules::preferences::Preferences::load().medir_quadros_sozinho && super::registry::is_elevated();
    let medicoes = crate::modules::medicoes::ler().ok();
    let estado = portao::ler_estrito();
    let rodada = portao::rodada_do_governador(estado.as_ref().ok(), &processo, medicoes.as_deref(), medindo);
    let agora = crate::modules::changelog::now_timestamp();

    let vigiar = |agindo: bool| {
        portao::observar_governador(&processo, jogo, agora, agindo)
            .map_err(|e| (Modo::Parado, Some(format!("O governador fica parado: {}.", e))))
    };
    let (modo, aviso) = match rodada {
        Rodada::Agir => match vigiar(true) {
            Ok(()) => (Modo::Agir, None),
            Err(parado) => parado,
        },
        Rodada::Comparar => match vigiar(false) {
            Err(parado) => parado,
            Ok(()) => (
                Modo::Comparar,
                Some(format!(
                    "Partida de comparação: nesta partida o governador não mexe em nada, para o Otimiza medir {} sem ele e conferir que ele não tira FPS.",
                    jogo
                )),
            ),
        },
        Rodada::Reprovado(d) => (Modo::Parado, Some(format!("Governador parado. {}", portao::frase_do_governador(&d)))),
        Rodada::SemMedicao(motivo) => (Modo::Parado, Some(format!("O governador fica parado: {}.", motivo))),
    };
    (Governador::novo(modo, Some(processo), pid), aviso)
}

fn passar(guarda: &mut Option<Governador>, jogo: &str, pid: u32) -> Vec<String> {
    let mut frases = Vec::new();

    // Outro jogo sem o fechamento visto: devolve a sessão velha antes de começar a nova.
    if guarda.as_ref().is_some_and(|g| g.jogo_pid() != pid) {
        if let Some(mut velho) = guarda.take() {
            let d = velho.devolver_tudo();
            if !d.falharam.is_empty() {
                frases.push(frase_de_falha(&d));
            }
        }
    }
    let sessao_nova = guarda.is_none();
    if sessao_nova {
        let (novo, aviso) = governador_para(jogo, pid);
        frases.extend(aviso);
        *guarda = Some(novo);
    }
    let Some(g) = guarda.as_mut() else { return frases };
    let feito = g.passada(pid);
    if let Some(frase) = mensagem_da_passada(jogo, g.modo(), &feito, sessao_nova) {
        frases.push(frase);
    }
    frases
}

/// **Pura.** "Nada disputando" e "o Windows não deixou" são diferentes: o segundo tem solução (abrir como
/// administrador).
pub fn mensagem_da_passada(jogo: &str, modo: Modo, p: &Passada, dizer_nada: bool) -> Option<String> {
    let mut partes = Vec::new();
    if !p.acalmados.is_empty() {
        partes.push(format!(
            "{} programa(s) em segundo plano passaram a rodar em modo econômico enquanto {} está aberto: {}.",
            p.acalmados.len(),
            jogo,
            p.acalmados.join(", ")
        ));
    }
    if !p.em_espera.is_empty() {
        partes.push(format!(
            "Disputando processador com {}, deixados como estão nesta partida de comparação: {}.",
            jogo,
            p.em_espera.join(", ")
        ));
    }
    if !p.recusados.is_empty() {
        let nomes: Vec<&str> = p.recusados.iter().map(|r| r.nome.as_str()).collect();
        let causa = if p.recusados.iter().all(|r| r.codigo == 5) {
            "acesso negado — costuma ser programa rodando como administrador com o Otimiza sem administrador".to_string()
        } else {
            let mut codigos: Vec<String> = p.recusados.iter().map(|r| r.codigo.to_string()).collect();
            codigos.dedup();
            format!("erro do Windows {}", codigos.join(", "))
        };
        partes.push(format!(
            "{} programa(s) disputam processador com {}, mas o Windows não deixou o Otimiza acalmá-los ({}): {}.",
            nomes.len(),
            jogo,
            causa,
            nomes.join(", ")
        ));
    }
    if partes.is_empty() && dizer_nada && modo != Modo::Parado {
        partes.push(format!("Nada em segundo plano está disputando processador com {} agora.", jogo));
    }
    (!partes.is_empty()).then(|| partes.join(" "))
}

fn frase_de_falha(d: &Devolucao) -> String {
    let mut partes = Vec::new();
    if !d.falharam.is_empty() {
        let nomes: Vec<&str> = d.falharam.iter().map(|a| a.nome.as_str()).collect();
        partes.push(format!("O Windows não deixou devolver ao normal: {}. Continuam em prioridade baixa.", nomes.join(", ")));
    }
    match (&d.nao_anotado, d.falharam.is_empty()) {
        (None, false) => partes.push(
            "Estão anotados, e o Otimiza tenta de novo quando o próximo jogo fechar e na próxima abertura.".to_string(),
        ),
        (Some(e), _) => partes.push(format!(
            "E não consegui ler ou gravar a lista do que foi acalmado ({}): o que estiver nela pode não voltar sozinho — reiniciar o PC devolve tudo.",
            e
        )),
        (None, true) => {}
    }
    partes.join(" ")
}

/// Liga o governador de segundo plano, sob o portão "nunca menos FPS" (ver `governador.rs`). Plano "Alto
/// Desempenho" e prioridade alta cega saíram na 2.9 (receita universal). `log` fica: quem veio de versão antiga
/// pode ter o plano antigo no histórico.
pub fn ativar(_log: &mut ChangeLog) -> Result<Vec<String>, String> {
    let (nome, pid) = jogo_aberto_com_pid()
        .ok_or("Nenhum jogo aberto agora. O modo age enquanto um jogo está rodando.")?;
    let mut guarda = trava();
    let mut frases = passar(&mut guarda, &nome, pid);
    if frases.is_empty() {
        frases.push(format!("Modo jogo ligado para {}.", nome));
    }
    Ok(frases)
}

/// Pela trava do governador: uma sessão começando ao mesmo tempo lê e grava o mesmo governador.json.
pub fn recuperar_na_abertura() -> Result<Devolucao, String> {
    let guarda = trava();
    if guarda.is_some() {
        return Ok(Devolucao::default());
    }
    super::governador::recuperar_na_abertura()
}

pub fn governador_na_partida(executavel: &str) -> Option<crate::modules::portao::GovernadorNaPartida> {
    let guarda = trava();
    let g = guarda.as_ref()?;
    let processo = g.processo()?;
    if !executavel.to_lowercase().starts_with(processo) {
        return None;
    }
    g.participacao()
}

/// As próximas partidas já nascem paradas (`portao::rodada_do_governador`).
pub fn reprovar_governador(processo: &str) -> Result<(), String> {
    let mut guarda = trava();
    let Some(g) = guarda.as_mut().filter(|g| g.processo() == Some(processo)) else {
        return Ok(());
    };
    let d = g.parar();
    if d.falharam.is_empty() {
        Ok(())
    } else {
        Err(frase_de_falha(&d))
    }
}

/// Para a trava do IFEO: o caminho separa o jogo instalado de um arquivo batizado com o mesmo nome.
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

/// O que o Windows não deixar devolver NÃO vira sucesso: erro com o nome, e segue anotado em disco.
pub fn desativar(log: &mut ChangeLog) -> Result<String, String> {
    // Sem governador na memória pode haver sobra em disco (`novo` a carrega). A trava fica até o fim: a devolução da
    // abertura (`recuperar_na_abertura`) mexe no mesmo arquivo.
    let devolucao = {
        let mut guarda = trava();
        let mut governador = guarda.take().unwrap_or_else(|| Governador::novo(Modo::Parado, None, 0));
        governador.devolver_tudo()
    };

    let mut plano = false;
    if let Some(registro) = log.take(ID)? {
        for mudanca in &registro.changes {
            if let ChangeRecord::PowerPlan { previous_guid } = mudanca {
                power::set_active_scheme(previous_guid)?;
                plano = true;
            }
        }
    }

    frase_do_desligar(&devolucao, plano)
}

fn frase_do_desligar(d: &Devolucao, plano: bool) -> Result<String, String> {
    let mut partes = Vec::new();
    if d.devolvidos > 0 {
        partes.push(format!("{} programa(s) voltaram ao normal.", d.devolvidos));
    }
    if plano {
        partes.push("O plano de energia voltou ao de antes.".to_string());
    }
    if !d.falharam.is_empty() || d.nao_anotado.is_some() {
        partes.push(frase_de_falha(d));
        return Err(format!("Modo jogo desligado só em parte. {}", partes.join(" ")));
    }
    Ok(if partes.is_empty() { "Modo jogo desligado.".to_string() } else { format!("Modo jogo desligado. {}", partes.join(" ")) })
}

/// Para quando a anotação estragou e ele ficou parado de vez. Com o modo ligado, zerar perderia o caminho de volta.
#[cfg(windows)]
pub fn zerar(log: &ChangeLog) -> Result<String, String> {
    if modo_ligado(log) {
        return Err("Desligue o modo jogo antes de zerar.".to_string());
    }
    let mut guarda = trava();
    *guarda = None;
    super::governador::zerar_anotacao().map(|z| super::governador::frase_do_zerar(&z))
}

fn modo_ligado(log: &ChangeLog) -> bool {
    trava().is_some() || log.is_applied(ID)
}

pub fn status(log: &ChangeLog) -> GameModeStatus {
    status_com(log, jogo_aberto())
}

/// Uma leitura só dos processos; o teste passa o jogo que quiser.
fn status_com(log: &ChangeLog, jogo: Option<String>) -> GameModeStatus {
    GameModeStatus {
        game_running: jogo.is_some(),
        game: jogo.map(|g| g.to_string()),
        active: modo_ligado(log),
        applied: trava()
            .as_ref()
            .map(|g| g.acalmados().iter().map(|a| a.nome.clone()).collect())
            .unwrap_or_default(),
    }
}

const IFEO: &str =
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";

/// NUNCA 4, que é tempo real (escala do IFEO: 1 baixa, 2 normal, 3 alta, 4 tempo real, 5 abaixo, 6 acima): acima
/// do próprio sistema, trava som, mouse e teclado.
const PRIORIDADE_ALTA: u32 = 3;

/// Vale em toda abertura: o Windows lê ao criar o processo. A mesma chave aceita `Debugger`, que faz o Windows
/// abrir OUTRO programa (sequestro de execução): só `CpuPriorityClass`, só em `PerfOptions`, e o nome vindo por IPC
/// é conferido.
pub fn definir_prioridade_persistente(
    executavel: &str,
    ativar: bool,
) -> Result<ChangeRecord, String> {
    if !super::registry::is_elevated() {
        return Err("Fixar a prioridade de um jogo exige executar como administrador.".to_string());
    }

    let nome = executavel.to_lowercase();

    // Barra ou dois-pontos seria escrever fora do lugar previsto.
    if nome.contains(['\\', '/', ':']) || !nome.ends_with(".exe") {
        return Err("Nome de executável inválido.".to_string());
    }

    // A trava é o CAMINHO dentro de uma biblioteca de jogo (Steam, Epic): heurística de detecção não pode ser
    // autoridade de segurança nesta chave.
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

    // Marca PERMANENTE na chave usada para sequestro: um anticheat de núcleo pode estranhar mesmo com o jogo fechado.
    let presencas = super::anticheat::detectar_agora();
    if let Some(recusa) =
        super::anticheat::permite(super::anticheat::Acao::EscreverIfeo, &presencas).motivo()
    {
        return Err(recusa.to_string());
    }

    let caminho = format!("{}\\{}\\PerfOptions", IFEO, executavel);

    // SEM `unwrap_or`: com `AbsentKey` diante de leitura falha, o `ativar` guardaria "não existia" (o desfazer
    // apagaria a prioridade do cliente) e o `desativar` escreveria errado na hora.
    let anterior = super::registry::read("HKLM", &caminho, "CpuPriorityClass")?;

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

/// Só na reserva, quando o caminho não se lê (típico de jogo com anticheat: Valorant, Fortnite, Apex): sem `.exe`,
/// dez dos doze jogos do catálogo, que casam por igualdade, sumiam. É palpite (existe `FiveM_ChromeBrowser` sem
/// extensão), mas esse caso já estava perdido.
fn com_extensao_exe(nome: String) -> String {
    if nome.to_lowercase().ends_with(".exe") {
        nome
    } else {
        format!("{}.exe", nome)
    }
}

/// O nome real do executável do jogo aberto: o do FiveM carrega o número da compilação, e quando ele atualiza o
/// ajuste por executável precisa ser aplicado de novo.
pub fn executavel_do_jogo() -> Option<String> {
    use sysinfo::System;

    let mut sistema = System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    escolher_executavel(sistema.processes().values().map(|p| {
        (
            p.exe().map(std::path::PathBuf::from),
            p.name().to_string_lossy().to_string(),
        )
    }))
}

/// Separada da varredura para ser testada.
fn escolher_executavel(
    processos: impl IntoIterator<Item = (Option<std::path::PathBuf>, String)>,
) -> Option<String> {
    processos
        .into_iter()
        .filter_map(|(exe, nome_curto)| {
            // O nome do arquivo no caminho real é o que a chave do IFEO precisa (o nome curto vem sem `.exe`); o curto é
            // reserva.
            let do_caminho = exe
                .as_deref()
                .and_then(|caminho| caminho.file_name())
                .map(|arquivo| arquivo.to_string_lossy().to_string());

            do_caminho.or_else(|| Some(com_extensao_exe(nome_curto)))
        })
        .find(|nome| nome_do_jogo(nome).is_some())
}

/// `None` sem mudança: a interface só é avisada quando há novidade.
pub fn passo(log: &mut ChangeLog) -> Option<String> {
    passo_com(log, jogo_aberto_com_pid())
}

/// Pura, testável sem processo nem histórico reais.
#[derive(Debug, PartialEq, Eq)]
enum Vigia {
    Nada,
    Ligar,
    NovaPassada,
    Desligar,
}

fn o_que_o_vigia_faz(jogo_aberto: bool, ligado: bool) -> Vigia {
    match (jogo_aberto, ligado) {
        (false, false) => Vigia::Nada,
        (true, false) => Vigia::Ligar,
        (true, true) => Vigia::NovaPassada,
        (false, true) => Vigia::Desligar,
    }
}

fn passo_com(log: &mut ChangeLog, aberto: Option<(String, u32)>) -> Option<String> {
    match (o_que_o_vigia_faz(aberto.is_some(), modo_ligado(log)), aberto) {
        (Vigia::NovaPassada, Some((nome, pid))) => {
            let frases = passar(&mut trava(), &nome, pid);
            (!frases.is_empty()).then(|| frases.join(" "))
        }
        (Vigia::Ligar, Some((nome, _))) => {
            // NADA É CONGELADO AQUI DESDE A 2.0.
            let feito = ativar(log).ok()?;

            Some(format!("{} aberto. {}", nome, feito.join(" ")))
        }
        (Vigia::Desligar, _) => {
            // Devolver os programas ANTES de desfazer o resto. Só age para quem veio de versão antiga.
            let devolvidos = super::suspend::retomar_tudo().unwrap_or_default();
            let texto = desativar(log).unwrap_or_else(|erro| erro);

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

            // Só com número de compilação no nome, e a chave termina em sublinhado para não casar com o lançador.
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
        // Até a 0.13, `contains("cs2")` ligava o modo jogo para qualquer programa com "cs2" no nome.
        for impostor in [
            "docs2pdf.exe",
            "nvcs2.exe",
            "redmine.exe",
            "redmond-sync.exe",
            "gta5-mod-manager-installer.exe",
            "fivem.exe", /* o lançador não é o processo que desenha */
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

        assert_eq!(nome_do_jogo("FiveM_b3570_GTAProcess.exe"), Some("FiveM"));
        assert_eq!(nome_do_jogo("RedM_b1491_GTAProcess.exe"), Some("RedM"));

        assert_eq!(nome_do_jogo(""), None);
        assert_eq!(nome_do_jogo("   "), None);
    }

    #[test]
    fn a_prioridade_nao_e_mais_exclusiva_do_fivem() {
        // `fivem::priorizar_jogo()` procurava `FiveM*GTAProcess*` e dizia "o jogo não está aberto" com o CS aberto.
        // Conferido pela IMPORTAÇÃO, não pelo texto, para os comentários poderem citar a função antiga.
        let producao = include_str!("gamemode.rs").split("#[cfg(test)]").next().unwrap();

        assert!(
            !producao.contains("use super::{fivem"),
            "o modo jogo voltou a importar o módulo do FiveM"
        );
    }

    #[test]
    fn o_modo_jogo_nao_usa_mais_receita_universal() {
        let producao = include_str!("gamemode.rs").split("#[cfg(test)]").next().unwrap();
        for proibido in ["garantir_alto_desempenho", "fn priorizar_pid", "PriorityClass = 'High'"] {
            assert!(!producao.contains(proibido), "`{}` voltou ao modo jogo", proibido);
        }
        assert!(producao.contains("governador"), "o modo jogo precisa usar o governador");
    }

    #[test]
    fn a_recusa_por_origem_esta_escrita_no_texto_da_mensagem() {
        // Não depende de privilégio: o teste acima só chega à recusa por origem elevado (verde local, quebrou na 0.14.0).
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
        for tentativa in [
            "notepad.exe",
            r"..\..\malicioso.exe",
            r"C:\jogos\gtaprocess.exe",
            "gtaprocess",
        ] {
            let erro = definir_prioridade_persistente(tentativa, true).unwrap_err();

            // Sem administrador a função para no primeiro if: a recusa por origem só roda elevada, como na esteira.
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

    /// Um NOME DE ARQUIVO, não um caminho, e nem sempre com `.exe` (`FiveM_ChromeBrowser` existe assim). Processos
    /// simulados; a varredura real fica no teste ignorado.
    #[test]
    fn nome_do_executavel_do_jogo() {
        use std::path::PathBuf;

        let nome = escolher_executavel([
            (Some(PathBuf::from(r"C:\Windows\notepad.exe")), "notepad.exe".to_string()),
            (Some(PathBuf::from(r"C:\Steam\steamapps\common\cs2\cs2.exe")), "cs2".to_string()),
        ]);
        assert_eq!(nome.as_deref(), Some("cs2.exe"));

        assert_eq!(escolher_executavel([(None, "cs2".to_string())]).as_deref(), Some("cs2.exe"));
        assert_eq!(
            escolher_executavel([(None, "RobloxPlayerBeta.exe".to_string())]).as_deref(),
            Some("RobloxPlayerBeta.exe")
        );

        assert_eq!(escolher_executavel([(None, "notepad.exe".to_string())]), None);
        assert_eq!(escolher_executavel(Vec::new()), None);
    }

    #[test]
    #[ignore = "lê esta máquina"]
    fn nome_do_executavel_do_jogo_nesta_maquina() {
        let nome = executavel_do_jogo();
        println!("executável do jogo agora: {:?}", nome);

        if let Some(n) = nome {
            assert!(!n.trim().is_empty(), "nome vazio nao serve para nada");
            assert!(!n.contains('\\') && !n.contains('/'), "veio um caminho: {:?}", n);
        }
    }

    #[test]
    fn desligar_sem_ter_ligado_nao_falha_e_nao_mexe_em_nada() {
        // Até a 2.8 era recusa: o governador pode estar ligado sem entrada no histórico, e recusar deixaria programas
        // acalmados sem volta. O teste antigo lia o histórico desta máquina e derrubou a publicação da 2.9.0.
        let mut log = ChangeLog::em_memoria();

        let resposta = desativar(&mut log).expect("desligar sem ter ligado não pode falhar");

        assert!(resposta.starts_with("Modo jogo desligado"), "{resposta}");
        assert!(!resposta.contains("plano de energia"), "não havia plano para devolver: {resposta}");
        assert!(!log.is_applied(ID));
    }

    #[test]
    fn o_vigia_so_age_quando_algo_muda() {
        // "Ligado" é governador OU histórico (`modo_ligado`); as quatro combinações sem depender da máquina.
        assert_eq!(o_que_o_vigia_faz(false, false), Vigia::Nada);
        assert_eq!(o_que_o_vigia_faz(true, false), Vigia::Ligar);
        assert_eq!(o_que_o_vigia_faz(true, true), Vigia::NovaPassada);
        assert_eq!(o_que_o_vigia_faz(false, true), Vigia::Desligar);
    }

    #[test]
    fn o_vigia_nao_faz_nada_quando_nao_ha_mudanca() {
        // Histórico em memória e jogo ausente injetado: nenhum teste liga o governador, então o passo é nada.
        let mut log = ChangeLog::em_memoria();

        assert!(passo_com(&mut log, None).is_none());
        assert!(!log.is_applied(ID));
    }

    #[test]
    fn o_status_mostra_o_jogo_que_recebeu() {
        // Uma leitura só: com duas, um jogo abrindo entre elas quebrava o teste.
        let log = ChangeLog::em_memoria();

        let s = status_com(&log, Some("Counter-Strike 2".to_string()));
        assert!(s.game_running);
        assert_eq!(s.game.as_deref(), Some("Counter-Strike 2"));

        let s = status_com(&log, None);
        assert!(!s.game_running);
        assert_eq!(s.game, None);
        assert!(!s.active);
    }

    #[test]
    #[ignore = "lê esta máquina"]
    fn detecta_jogo_nesta_maquina() {
        let s = status(&ChangeLog::em_memoria());
        println!("jogo aberto agora: {:?}", s.game);
        assert_eq!(s.game_running, s.game.is_some());
    }

    #[test]
    fn chave_casa_com_o_nome_que_a_medicao_grava() {
        assert_eq!(chave_do_processo("FiveM_b3258_GTAProcess.exe"), "fivem_");
        assert_eq!(chave_do_processo("cs2.exe"), "cs2");
        assert_eq!(chave_do_processo("Palworld-Win64-Shipping.exe"), "palworld-win64-shipping");
    }

    fn recusa(nome: &str, codigo: u32) -> super::super::governador::Recusa {
        super::super::governador::Recusa { nome: nome.into(), codigo }
    }

    #[test]
    fn nao_havia_e_nao_consegui_sao_frases_diferentes() {
        let nada = mensagem_da_passada("FiveM", Modo::Agir, &Passada::default(), true).unwrap();
        assert!(nada.contains("Nada em segundo plano"));

        let recusado = Passada { recusados: vec![recusa("OneDrive.exe", 5)], ..Default::default() };
        let frase = mensagem_da_passada("FiveM", Modo::Agir, &recusado, true).unwrap();
        assert!(!frase.contains("Nada"), "recusa não pode virar \"nada disputando\": {}", frase);
        assert!(frase.contains("OneDrive.exe") && frase.contains("administrador"), "{}", frase);

        let outro = Passada { recusados: vec![recusa("x.exe", 87)], ..Default::default() };
        assert!(mensagem_da_passada("FiveM", Modo::Agir, &outro, false).unwrap().contains("erro do Windows 87"));
    }

    #[test]
    fn silencio_depois_da_primeira_passada_e_parado_sem_frase() {
        assert_eq!(mensagem_da_passada("FiveM", Modo::Agir, &Passada::default(), false), None);
        assert_eq!(mensagem_da_passada("FiveM", Modo::Parado, &Passada::default(), true), None);
    }

    #[test]
    fn desligar_com_devolucao_falha_nao_e_sucesso() {
        use super::super::governador::{Acalmado, Eco};
        let falhou = Devolucao {
            devolvidos: 2,
            sumidos: 0,
            falharam: vec![Acalmado { pid: 7, nome: "chrome.exe".into(), inicio: 1, prioridade_anterior: 32, ecoqos: Eco::NaoMexido }],
            nao_anotado: None,
        };
        let erro = frase_do_desligar(&falhou, false).unwrap_err();
        assert!(erro.contains("só em parte") && erro.contains("chrome.exe"), "{}", erro);

        assert_eq!(frase_do_desligar(&Devolucao::default(), false).unwrap(), "Modo jogo desligado.");
        let ok = Devolucao { devolvidos: 3, ..Default::default() };
        assert!(frase_do_desligar(&ok, true).unwrap().contains("3 programa(s) voltaram ao normal. O plano de energia"));
    }

    #[test]
    fn mutex_envenenado_nao_esconde_o_governador() {
        let _ = std::thread::spawn(|| {
            let mut g = GOVERNADOR.lock().unwrap_or_else(|e| e.into_inner());
            *g = Some(Governador::default());
            panic!("envenena de propósito");
        })
        .join();
        assert!(GOVERNADOR.is_poisoned());
        assert!(trava().take().is_some(), "o governador continua lá dentro para ser devolvido");
        GOVERNADOR.clear_poison();
    }

    #[test]
    fn lista_que_nao_se_le_nem_grava_nao_vira_desligado() {
        let d = Devolucao { nao_anotado: Some("governador.json ilegível".into()), ..Default::default() };
        let erro = frase_do_desligar(&d, false).unwrap_err();
        assert!(erro.contains("governador.json ilegível") && !erro.contains("Estão anotados"), "{}", erro);
    }
}
