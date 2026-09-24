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

use super::governador::{Devolucao, Governador, Modo, Passada};
use super::power;
use crate::modules::changelog::{ChangeLog, ChangeRecord};
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

/// O governador da sessão de jogo aberta. `Some` enquanto o modo está ligado.
static GOVERNADOR: std::sync::Mutex<Option<Governador>> = std::sync::Mutex::new(None);

/// O governador, mesmo com o mutex envenenado.
///
/// Envenenar é um pânico no meio de uma passada. A lista do que foi acalmado
/// continua lá dentro e continua valendo — e é justamente ela que o desligar
/// precisa para devolver. Tratar o veneno como "não há governador" fazia a
/// tela dizer "Modo jogo desligado." com programas ainda em prioridade baixa.
fn trava() -> std::sync::MutexGuard<'static, Option<Governador>> {
    GOVERNADOR.lock().unwrap_or_else(|envenenado| envenenado.into_inner())
}

/// A chave de um jogo no formato do portão e das medições: o começo do nome
/// do processo, em minúsculas. **Função pura.**
///
/// FiveM e RedM trocam de nome a cada compilação (`FiveM_b3258_GTAProcess`),
/// então a chave deles é o pedaço fixo do catálogo; os outros, o nome sem
/// `.exe`.
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

/// Pergunta ao portão como o governador age na partida que começa. Devolve o
/// governador da sessão e, quando há, o aviso para a tela.
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
    // Antes de qualquer outra coisa: sem ler o que uma sessão anterior deixou
    // acalmado, ele não mexe em nada — nem anota a partida como "agindo".
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

    // Sem a vigília gravada não há comparação nem veredito: não age.
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

/// Uma passada do governador para o jogo aberto, criando o da sessão quando
/// ela começa. Devolve as frases para a tela (vazia quando nada mudou).
fn passar(guarda: &mut Option<Governador>, jogo: &str, pid: u32) -> Vec<String> {
    let mut frases = Vec::new();

    // Outro jogo no lugar do anterior sem o vigia ter visto o fechamento:
    // devolve o que era da sessão velha antes de começar a nova.
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
    // "Nada disputando" só na primeira passada da sessão: nas seguintes,
    // silêncio é o normal e não precisa de aviso.
    if let Some(frase) = mensagem_da_passada(jogo, g.modo(), &feito, sessao_nova) {
        frases.push(frase);
    }
    frases
}

/// A frase de uma passada. **Função pura.**
///
/// "Nada disputando" e "o Windows não deixou" são coisas diferentes, e a tela
/// precisa saber qual das duas aconteceu: a segunda é um programa que continua
/// brigando com o jogo, e quase sempre tem solução (abrir o Otimiza como
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

/// **Função pura.**
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

/// Liga o modo (2.9): o governador de segundo plano.
///
/// O QUE SAIU, E POR QUÊ. Até a 2.7 o modo jogo ligava o plano "Alto
/// Desempenho" e punha o jogo em prioridade alta. As duas coisas são receita
/// universal — o mesmo número em toda máquina, sem medir — e a 2.9 tirou
/// ambas: a energia do jogo agora é do motor adaptativo (perfil MEDIDO por
/// jogo, aba Energia), e prioridade alta cega não mostrou ganho. O que fica é
/// o que ataca uma causa real de engasgo: programa em segundo plano
/// disputando processador com o jogo. Ver `governador.rs` — e ele mesmo só
/// age com o aval do portão "nunca menos FPS".
///
/// `log` continua na assinatura: quem atualizou de uma versão antiga pode
/// ter o plano de energia do modo antigo no histórico, e `desativar` devolve.
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

/// Na abertura do Otimiza: devolve o que uma sessão anterior deixou acalmado.
///
/// Pela trava do governador, para não correr com uma sessão de jogo que
/// começa ao mesmo tempo: as duas leem e gravam o mesmo governador.json.
/// Se a sessão já começou, ela carregou as sobras e devolve quando terminar.
pub fn recuperar_na_abertura() -> Result<Devolucao, String> {
    let guarda = trava();
    if guarda.is_some() {
        return Ok(Devolucao::default());
    }
    super::governador::recuperar_na_abertura()
}

/// O que o governador está fazendo agora no jogo `executavel`, para marcar a
/// medição automática. `None` quando ele não está nesse jogo.
pub fn governador_na_partida(executavel: &str) -> Option<crate::modules::portao::GovernadorNaPartida> {
    let guarda = trava();
    let g = guarda.as_ref()?;
    let processo = g.processo()?;
    if !executavel.to_lowercase().starts_with(processo) {
        return None;
    }
    g.participacao()
}

/// O portão reprovou o governador em `processo`: se ele está agindo nesse
/// jogo agora, devolve tudo e para. As próximas partidas já nascem paradas
/// (`portao::rodada_do_governador`).
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

/// Desliga o modo: devolve prioridade e EcoQoS de todo programa acalmado, e
/// — para quem veio de versão antiga — o plano de energia do modo antigo.
///
/// Programa que o Windows não deixou devolver NÃO vira sucesso: a resposta é
/// erro, com o nome dele, e ele segue anotado em disco para a próxima
/// tentativa.
pub fn desativar(log: &mut ChangeLog) -> Result<String, String> {
    // Sem governador na memória ainda pode haver sobra anotada em disco de
    // uma devolução que falhou antes: `novo` a carrega, e ela é tentada aqui.
    // A trava fica segura até o fim da devolução: a devolução da abertura
    // (ecuperar_na_abertura) mexe no mesmo arquivo.
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

/// **Função pura.**
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

/// O botão "Zerar o modo jogo": para quando a anotação do governador estragou
/// e ele ficou parado de vez. Com o modo ligado, a lista em uso é a da
/// partida aberta, e zerar perderia o caminho de volta.
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

/// Situação atual, para a interface.
pub fn status(log: &ChangeLog) -> GameModeStatus {
    status_com(log, jogo_aberto())
}

/// O status com o jogo já lido: uma leitura só dos processos, e o teste
/// passa o jogo que quiser sem depender do que está aberto na máquina.
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

    // SEM `unwrap_or` AQUI, E DOI DOS DOIS LADOS.
    //
    // Desde a 1.8 o `registry::read` distingue "não existe" de "não consegui
    // ler". Assumir `AbsentKey` diante de uma leitura falha estraga as duas
    // pontas desta função:
    //
    //   - no `ativar`, o histórico guarda "não existia", e o desfazer APAGA a
    //     prioridade que o cliente tinha em vez de devolvê-la;
    //   - no `desativar`, o estrago é IMEDIATO — a linha logo abaixo restaura
    //     `anterior`, então uma leitura falha vira uma escrita errada agora,
    //     sem esperar desfazer nenhum.
    //
    // Falhar aqui não custa nada ao cliente: ainda não escrevemos.
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

/// Nome do executável do jogo em execução, quando há um.
///
/// É preciso pegar o nome real porque o processo do FiveM carrega o número da
/// compilação: `FiveM_b3570_GTAProcess.exe`. Isso tem uma consequência que a
/// interface precisa dizer — quando o FiveM atualiza, o nome muda e o ajuste
/// precisa ser aplicado de novo.
/// Garante a extensão no nome curto vindo do `sysinfo`.
///
/// O NOME SEM `.exe` PERDIA A MAIORIA DOS JOGOS DO CATÁLOGO, e em silêncio.
///
/// Das doze entradas de `JOGOS`, dez casam por igualdade exata e todas as dez
/// têm `.exe` na chave — `gta5.exe`, `valorant-win64-shipping.exe`,
/// `r5apex.exe`. Só FiveM e RedM casam por pedaço, e por isso eram os únicos
/// que sobreviviam ao nome curto.
///
/// A reserva sem extensão só é usada quando o caminho do processo não pode ser
/// lido — e isso não é raro nem aleatório: é o comportamento típico de jogo com
/// anticheat, que é exatamente Valorant, Fortnite e Apex. Ou seja, o caminho de
/// reserva falhava justamente nos jogos em que ele era a única saída.
///
/// Acrescentar a extensão aqui é um palpite, e vale dizer que é: existe
/// executável sem extensão no Windows — o próprio FiveM instala
/// `FiveM_ChromeBrowser` assim. Mas esta reserva só roda quando o caminho não
/// pôde ser lido, e nesse caso não há como saber o nome real de qualquer jeito.
/// O palpite acerta os dez jogos de casamento exato do catálogo e erra num
/// binário sem extensão cujo caminho também não abre — que é um caso que já
/// estava perdido antes.
fn com_extensao_exe(nome: String) -> String {
    if nome.to_lowercase().ends_with(".exe") {
        nome
    } else {
        format!("{}.exe", nome)
    }
}

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

/// Entre (caminho, nome curto) de cada processo, o nome de arquivo do
/// primeiro que é jogo conhecido. Separada da varredura para ser testada sem
/// depender do que está aberto na máquina.
fn escolher_executavel(
    processos: impl IntoIterator<Item = (Option<std::path::PathBuf>, String)>,
) -> Option<String> {
    processos
        .into_iter()
        .filter_map(|(exe, nome_curto)| {
            // O nome curto do `sysinfo` às vezes vem sem extensão — na máquina
            // de desenvolvimento, `FiveM_DumpServer`. O nome do arquivo no
            // caminho real é a fonte confiável, e é ele que a chave do IFEO
            // precisa: sem `.exe`, a gravação é recusada e o caminho automático
            // do modo jogo esbarraria num "nome de executável inválido" para um
            // jogo legítimo. O nome curto fica como reserva para o processo cujo
            // caminho não podemos ler.
            let do_caminho = exe
                .as_deref()
                .and_then(|caminho| caminho.file_name())
                .map(|arquivo| arquivo.to_string_lossy().to_string());

            do_caminho.or_else(|| Some(com_extensao_exe(nome_curto)))
        })
        .find(|nome| nome_do_jogo(nome).is_some())
}

/// Um passo do vigia: liga quando o jogo abre, desliga quando ele fecha.
///
/// Devolve a mensagem quando alguma coisa mudou, e `None` quando não havia o
/// que fazer — assim a interface só é avisada quando há novidade.
pub fn passo(log: &mut ChangeLog) -> Option<String> {
    passo_com(log, jogo_aberto_com_pid())
}

/// O que um passo do vigia faz, decidido só por jogo aberto e modo ligado.
/// Pura, para a regra ser testada sem processo nem histórico reais.
#[derive(Debug, PartialEq, Eq)]
enum Vigia {
    /// Nada mudou: o vigia roda a cada poucos segundos e não pode agir à toa.
    Nada,
    /// O jogo abriu com o modo desligado.
    Ligar,
    /// O jogo segue aberto com o modo ligado: nova passada do governador.
    NovaPassada,
    /// O jogo fechou com o modo ligado.
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

/// O passo com o jogo já detectado, para o teste injetar o jogo.
fn passo_com(log: &mut ChangeLog, aberto: Option<(String, u32)>) -> Option<String> {
    match (o_que_o_vigia_faz(aberto.is_some(), modo_ligado(log)), aberto) {
        (Vigia::NovaPassada, Some((nome, pid))) => {
            // Um atualizador pode começar a baixar no meio da partida.
            let frases = passar(&mut trava(), &nome, pid);
            (!frases.is_empty()).then(|| frases.join(" "))
        }
        (Vigia::Ligar, Some((nome, _))) => {
            // NADA É CONGELADO AQUI DESDE A 2.0.
            //
            // Até a 1.9 este era o ponto em que o vigia suspendia Discord,
            // navegador e afins. Foi a opção que fez programa do cliente parar
            // de abrir — o Explorador na 1.1.1, a Steam na 1.1.2 — e saiu do
            // produto. O modo jogo agora é só o que o nome promete: plano de
            // energia e prioridade para o jogo.
            let feito = ativar(log).ok()?;

            Some(format!("{} aberto. {}", nome, feito.join(" ")))
        }
        (Vigia::Desligar, _) => {
            // A ordem importa: devolver os programas ANTES de desfazer o resto.
            //
            // Nada é congelado desde a 2.0, então isto só age para quem
            // atualizou de uma versão antiga no meio de uma partida. Com o
            // registro vazio não faz nada — e continua aqui porque um Discord
            // preso pelo Otimiza velho é exatamente o defeito que a remoção
            // existe para acabar.
            let devolvidos = super::suspend::retomar_tudo().unwrap_or_default();
            // Falha ao devolver vai para a tela como está: nunca some.
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
    }

    #[test]
    fn o_modo_jogo_nao_usa_mais_receita_universal() {
        // 2.9: nem plano "Alto Desempenho" fixo, nem prioridade alta cega no
        // jogo. A energia é do motor adaptativo; o modo jogo é o governador.
        let producao = include_str!("gamemode.rs").split("#[cfg(test)]").next().unwrap();
        for proibido in ["garantir_alto_desempenho", "fn priorizar_pid", "PriorityClass = 'High'"] {
            assert!(!producao.contains(proibido), "`{}` voltou ao modo jogo", proibido);
        }
        assert!(producao.contains("governador"), "o modo jogo precisa usar o governador");
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

    /// O que sai daqui e um NOME DE ARQUIVO, e nao um caminho.
    ///
    /// A versao anterior deste teste exigia que o nome terminasse em `.exe`, e
    /// isso e falso: o FiveM instala `FiveM_ChromeBrowser` SEM extensao nenhuma,
    /// e o arquivo existe assim em disco. O teste passava so porque nenhuma
    /// maquina de teste tinha o FiveM aberto — quando teve, ele reprovou, e
    /// estava certo em reprovar: quem estava errado era a afirmacao.
    ///
    /// Depois disso ele ainda lia os processos desta maquina e so conferia com
    /// jogo aberto: sem jogo, nao afirmava nada. Agora os processos sao
    /// simulados, e a varredura real fica no teste ignorado abaixo.
    ///
    /// O que vale de verdade e que o valor seja um nome de arquivo. Caminho
    /// completo aqui quebraria a chave do IFEO e o casamento com o catalogo.
    #[test]
    fn nome_do_executavel_do_jogo() {
        use std::path::PathBuf;

        // Caminho legivel: o nome vem do arquivo, nunca o caminho inteiro, e
        // o nome curto sem extensao do `sysinfo` nao e o que sai.
        let nome = escolher_executavel([
            (Some(PathBuf::from(r"C:\Windows\notepad.exe")), "notepad.exe".to_string()),
            (Some(PathBuf::from(r"C:\Steam\steamapps\common\cs2\cs2.exe")), "cs2".to_string()),
        ]);
        assert_eq!(nome.as_deref(), Some("cs2.exe"));

        // Caminho ilegivel: o nome curto, com `.exe` quando faltar.
        assert_eq!(escolher_executavel([(None, "cs2".to_string())]).as_deref(), Some("cs2.exe"));
        assert_eq!(
            escolher_executavel([(None, "RobloxPlayerBeta.exe".to_string())]).as_deref(),
            Some("RobloxPlayerBeta.exe")
        );

        // Sem jogo, nada.
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
        // ATÉ A 2.8 ISTO ERA UMA RECUSA, e a 2.9 mudou de propósito: o modo
        // jogo agora é o governador, que pode estar ligado sem entrada nenhuma
        // no histórico. Desligar precisa funcionar nesse caso — recusar
        // deixaria programas acalmados sem caminho de volta.
        //
        // O teste antigo lia o histórico DESTA máquina e só conferia quando o
        // modo não estava aplicado. No PC de quem desenvolve estava, a
        // conferência nunca rodava, e o teste ficou verde por acaso enquanto
        // afirmava o comportamento velho. Na esteira, com histórico vazio,
        // ele rodou — e derrubou a publicação da 2.9.0.
        //
        // Histórico em memória: o resultado não depende de máquina nenhuma.
        let mut log = ChangeLog::em_memoria();

        let resposta = desativar(&mut log).expect("desligar sem ter ligado não pode falhar");

        assert!(resposta.starts_with("Modo jogo desligado"), "{resposta}");
        assert!(!resposta.contains("plano de energia"), "não havia plano para devolver: {resposta}");
        assert!(!log.is_applied(ID));
    }

    #[test]
    fn o_vigia_so_age_quando_algo_muda() {
        // Sem jogo aberto e com o modo desligado, um passo do vigia não pode
        // mexer em nada: ele roda a cada poucos segundos, e um passo que age à
        // toa mexeria na máquina o tempo todo.
        //
        // Desde a 2.9 "ligado" é o governador OU o histórico (`modo_ligado`).
        // A regra recebe o ligado já resolvido e as quatro combinações são
        // conferidas, sem depender do que está aberto nesta máquina.
        assert_eq!(o_que_o_vigia_faz(false, false), Vigia::Nada);
        assert_eq!(o_que_o_vigia_faz(true, false), Vigia::Ligar);
        assert_eq!(o_que_o_vigia_faz(true, true), Vigia::NovaPassada);
        assert_eq!(o_que_o_vigia_faz(false, true), Vigia::Desligar);
    }

    #[test]
    fn o_vigia_nao_faz_nada_quando_nao_ha_mudanca() {
        // O teste antigo lia o histórico e os processos DESTA máquina e só
        // conferia sem jogo aberto e sem modo aplicado — com um jogo aberto,
        // não afirmava nada. É o padrão do teste que derrubou a 2.9.0.
        //
        // Histórico em memória e jogo injetado como ausente. Nenhum teste
        // deste binário liga o governador (só `ativar` e o passo COM jogo o
        // fazem), então o modo está desligado e o passo tem de ser nada.
        let mut log = ChangeLog::em_memoria();

        assert!(passo_com(&mut log, None).is_none());
        assert!(!log.is_applied(ID));
    }

    #[test]
    fn o_status_mostra_o_jogo_que_recebeu() {
        // Uma leitura só do jogo, passada adiante: o antigo
        // `detecta_jogo_nesta_maquina` chamava `jogo_aberto()` duas vezes e
        // falhava se um jogo abrisse ou fechasse entre as leituras.
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
        // Um pânico no meio de uma passada envenena o mutex. Antes, o desligar
        // lia isso como "não há governador" e dizia "Modo jogo desligado." com
        // programas ainda acalmados.
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
