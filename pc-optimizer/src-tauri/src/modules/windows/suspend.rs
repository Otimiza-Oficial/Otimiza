// Suspender o que está em segundo plano durante o jogo
//
// POR QUE ISTO EXISTE, E POR QUE NÃO É "FECHAR PROGRAMAS"
//
// A queixa que originou este módulo: com o FiveM aberto, o PC inteiro trava —
// o jogo, o Discord, o navegador, tudo junto. Isso não é FPS baixo; FPS baixo
// trava o jogo e deixa o resto fluido. Travar tudo junto é memória acabando.
//
// Todo "otimizador" do mercado responde a isso MATANDO processos. O Otimiza
// não faz isso e não vai fazer: matar o Discord no meio de uma conversa, ou o
// navegador com quinze abas de trabalho, é a forma mais rápida de o cliente
// perder coisa que não dá para recuperar. Já recusamos essa ideia quatro vezes
// neste projeto, e a recusa continua.
//
// Suspender é diferente. O processo para de consumir CPU e suas páginas viram
// candidatas preferenciais a sair da memória física para a paginação — que é
// exatamente o que queremos, porque a RAM liberada vai para o jogo. Quando o
// jogo fecha, o processo volta do ponto em que estava. Nada se perde.
//
// O RISCO, E COMO ELE É COBERTO
//
// Se o Otimiza morrer com processos suspensos, eles ficam suspensos até o
// cliente reiniciar o PC — e ele não vai saber por quê. Um Discord congelado
// para sempre é um defeito pior do que o problema que viemos resolver.
//
// Por isso os PIDs suspensos vão para disco ANTES de a primeira thread ser
// suspensa, e `retomar_pendentes()` roda na abertura do programa, antes de
// qualquer outra coisa. Queda de energia, travamento, fechamento à força: em
// todos os casos a próxima abertura devolve os processos.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

/// Programas que faz sentido suspender durante o jogo.
///
/// A lista é explícita e curta de propósito. Suspender "tudo que não é o jogo"
/// é como se congela o áudio, o antivírus ou o próprio Windows — e a diferença
/// entre um otimizador e um problema é justamente esta lista.
///
/// Os nomes são comparados em minúsculas, por conter.
pub const SUSPENSIVEIS: &[(&str, &str)] = &[
    ("discord.exe", "Discord"),
    ("chrome.exe", "Google Chrome"),
    ("msedge.exe", "Microsoft Edge"),
    ("firefox.exe", "Firefox"),
    ("opera.exe", "Opera"),
    ("brave.exe", "Brave"),
    ("arc.exe", "Arc"),
    ("steam.exe", "Steam"),
    ("steamwebhelper.exe", "Steam (navegador interno)"),
    ("epicgameslauncher.exe", "Epic Games"),
    ("spotify.exe", "Spotify"),
    ("slack.exe", "Slack"),
    ("teams.exe", "Microsoft Teams"),
    ("whatsapp.exe", "WhatsApp"),
    ("telegram.exe", "Telegram"),
];

/// Nunca, em hipótese alguma.
///
/// Não é uma lista de "melhor não": é uma lista de coisas que, suspensas,
/// quebram o PC enquanto o jogo roda. O áudio para. O antivírus deixa de
/// proteger. O explorador de arquivos congela a barra de tarefas. E suspender
/// o próprio Otimiza deixaria os processos suspensos para sempre, porque quem
/// os devolve é ele.
pub const NUNCA_SUSPENDER: &[&str] = &[
    // Anticheat. Já não entrariam por inclusão, mas a lista de proibidos é o
    // que o próximo mantenedor lê — e suspender um anticheat é a forma mais
    // rápida de fazer um cliente perder a conta.
    "vgc.exe",
    "vgtray.exe",
    "easyanticheat.exe",
    "easyanticheat_eos.exe",
    "beservice.exe",
    "bedaisy.exe",
    "faceitclient.exe",
    "faceitservice.exe",
    // O próprio Otimiza e o motor da interface.
    "pc-optimizer.exe",
    "otimiza.exe",
    // Núcleo do Windows.
    "explorer.exe",
    "dwm.exe",
    "csrss.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
    "smss.exe",
    "wininit.exe",
    "svchost.exe",
    "system",
    "registry",
    "fontdrvhost.exe",
    "sihost.exe",
    "ctfmon.exe",
    "taskhostw.exe",
    "shellexperiencehost.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    // Áudio. Suspender qualquer um destes corta o som do jogo.
    "audiodg.exe",
    "rtkngui64.exe",
    "realtekaudiouniversalservice.exe",
    // Antivírus e segurança. Suspender é desligar a proteção sem avisar.
    "msmpeng.exe",
    "nissrv.exe",
    "securityhealthservice.exe",
    "securityhealthsystray.exe",
    "avp.exe",
    "avastui.exe",
    "avgui.exe",
    "bdagent.exe",
    "mbamservice.exe",
    "mbam.exe",
    "ekrn.exe",
    "egui.exe",
    "norton.exe",
    "ns.exe",
    // Vídeo. O painel do driver participa da apresentação de quadros.
    "nvcontainer.exe",
    "nvdisplay.container.exe",
    "amddvr.exe",
    "radeonsoftware.exe",
    "igfxem.exe",
];

/// Um processo que o Otimiza suspendeu, e precisa devolver.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Suspenso {
    pub pid: u32,
    pub nome: String,
    /// Nome apresentável, para a interface poder dizer o que fez.
    pub visivel: String,
    /// Quando o processo começou. Serve de assinatura: um PID é reciclado pelo
    /// Windows, e sem isto o Otimiza poderia "retomar" um processo novo que
    /// nunca suspendeu — mexendo num programa que não é o dele.
    pub inicio: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registro {
    pub suspensos: Vec<Suspenso>,
    /// Quando este registro foi gravado, em segundos desde a época Unix.
    ///
    /// `#[serde(default)]` porque um registro gravado por uma versão anterior
    /// do Otimiza não tem este campo — e um registro antigo sem data precisa
    /// ser tratado como "há muito tempo", não travar a leitura. É o que
    /// permite à rede de segurança por prazo (ver `retomar_se_expirado`)
    /// saber que algo ficou suspenso tempo demais mesmo sem jogo nenhum
    /// rodando.
    #[serde(default)]
    pub quando: u64,
}

impl Registro {
    /// Onde o registro fica na máquina do cliente.
    ///
    /// Caminho único, porque o produto tem um registro só. Os testes não
    /// passam por aqui: cada um chama as variantes `_de`/`_em` com um arquivo
    /// próprio. Ver `caminho_de_teste`, na seção de testes, para o porquê.
    fn path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        base.join("pc-optimizer").join("suspensos.json")
    }

    pub fn load() -> Self {
        Self::load_de(&Self::path())
    }

    fn load_de(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Grava o registro.
    ///
    /// Chamado ANTES de suspender, nunca depois: se o programa morresse entre
    /// suspender e gravar, o processo ficaria congelado sem ninguém sabendo.
    /// Gravar antes pode, no pior caso, mandar retomar algo que não chegou a
    /// ser suspenso — e retomar um processo que já roda não faz nada.
    pub fn save(&self) -> Result<(), String> {
        self.save_em(&Self::path())
    }

    fn save_em(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Não foi possível criar a pasta de dados: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Não foi possível gravar o registro: {}", e))?;

        fs::write(path, json).map_err(|e| format!("Não foi possível gravar o registro: {}", e))
    }

    pub fn limpar() -> Result<(), String> {
        Self::limpar_em(&Self::path())
    }

    fn limpar_em(path: &Path) -> Result<(), String> {
        // Registro ausente é o estado que se queria alcançar, não um erro.
        // Perguntar `exists()` antes de apagar abriria uma janela entre a
        // pergunta e a resposta; deixar o próprio apagar responder não abre.
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Não foi possível limpar o registro: {}", e)),
        }
    }
}

/// Lançadores de loja.
///
/// Separados do resto porque o anticheat conversa com eles durante a partida:
/// suspender a Steam com Counter-Strike aberto derruba a sessão do VAC, e
/// suspender o lançador da Epic com Fortnite aberto atrapalha o Easy
/// Anti-Cheat. Fora de partida, suspendê-los é seguro e devolve memória — que
/// é por que eles continuam na lista de suspensíveis.
pub const LANCADORES: &[&str] = &[
    "steam.exe",
    "steamwebhelper.exe",
    "epicgameslauncher.exe",
    "riotclientservices.exe",
    "battle.net.exe",
    "eadesktop.exe",
    "upc.exe",
];

pub fn e_lancador(nome: &str) -> bool {
    let minusculo = nome.trim().to_lowercase();
    LANCADORES.iter().any(|l| minusculo == *l)
}

/// Decide se um processo pode ser suspenso.
///
/// Função pura, separada da varredura, para poder ser testada sem suspender
/// nada de verdade. É a peça mais perigosa do módulo: um engano aqui congela
/// o áudio ou o antivírus da máquina de um cliente.
pub fn pode_suspender(nome: &str) -> Option<&'static str> {
    let minusculo = nome.trim().to_lowercase();

    if minusculo.is_empty() {
        return None;
    }

    // A proibição vem primeiro e vence sempre, inclusive se alguém acrescentar
    // o mesmo nome nas duas listas por engano.
    if NUNCA_SUSPENDER.iter().any(|p| minusculo == *p) {
        return None;
    }

    // O jogo em execução nunca é suspenso — seria o oposto do objetivo.
    if super::gamemode::nome_do_jogo(&minusculo).is_some() {
        return None;
    }

    SUSPENSIVEIS
        .iter()
        .find(|(exe, _)| minusculo == *exe)
        .map(|(_, visivel)| *visivel)
}

// --------------------------------------------------------------- Windows API

#[cfg(target_os = "windows")]
mod api {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::Threading::{
        OpenThread, ResumeThread, SuspendThread, THREAD_SUSPEND_RESUME,
    };

    /// Aplica uma operação a todas as threads de um processo.
    ///
    /// O Windows não tem `SuspendProcess` público. A forma suportada é
    /// percorrer as threads e suspender uma a uma — é o que os depuradores
    /// fazem. `NtSuspendProcess` existe, mas é interna e sem contrato: um
    /// produto vendido não se apoia em API que a Microsoft pode mudar sem
    /// aviso numa atualização.
    ///
    /// Devolve quantas threads responderam.
    fn para_cada_thread(pid: u32, suspender: bool) -> u32 {
        let mut atingidas = 0;

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);

            if snapshot == INVALID_HANDLE_VALUE {
                return 0;
            }

            let mut entrada: THREADENTRY32 = std::mem::zeroed();
            entrada.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;

            if Thread32First(snapshot, &mut entrada) != 0 {
                loop {
                    if entrada.th32OwnerProcessID == pid {
                        let handle = OpenThread(THREAD_SUSPEND_RESUME, 0, entrada.th32ThreadID);

                        if !handle.is_null() {
                            let resultado = if suspender {
                                SuspendThread(handle)
                            } else {
                                ResumeThread(handle)
                            };

                            // -1 (0xFFFFFFFF) sinaliza falha.
                            if resultado != u32::MAX {
                                atingidas += 1;
                            }

                            CloseHandle(handle);
                        }
                    }

                    entrada.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
                    if Thread32Next(snapshot, &mut entrada) == 0 {
                        break;
                    }
                }
            }

            CloseHandle(snapshot);
        }

        atingidas
    }

    pub fn suspender(pid: u32) -> u32 {
        para_cada_thread(pid, true)
    }

    pub fn retomar(pid: u32) -> u32 {
        // Retomar é chamado repetidamente até a thread destravar: cada
        // `SuspendThread` incrementa um contador, e a thread só volta a rodar
        // quando ele zera. Suspender duas vezes e retomar uma deixaria o
        // processo congelado — e é exatamente o defeito que este módulo não
        // pode ter.
        let mut atingidas = 0;

        for _ in 0..8 {
            let n = para_cada_thread(pid, false);
            if n == 0 {
                break;
            }
            atingidas = atingidas.max(n);
        }

        atingidas
    }
}

#[cfg(not(target_os = "windows"))]
mod api {
    pub fn suspender(_pid: u32) -> u32 {
        0
    }
    pub fn retomar(_pid: u32) -> u32 {
        0
    }
}

// ------------------------------------------------------------------- ações

/// Serializa suspender e devolver.
///
/// `suspender_fundo` grava o registro em disco ANTES de suspender e só depois
/// percorre os alvos suspendendo um a um — de propósito, para nunca ter um
/// processo suspenso sem registro (ver o comentário no topo do arquivo).
/// Antes deste conserto, quem devolvia (`retomar_tudo`) só corria depois que
/// o jogo fechava ou na abertura seguinte do Otimiza — nunca no meio desse
/// laço. Agora que `retomar_tudo` também pode ser disparado pelo gancho de
/// fim de sessão do Windows e pelo fechamento do Otimiza, as duas coisas
/// podem correr ao mesmo tempo, em threads diferentes: o laço de devolução lê
/// o registro, retoma o que já foi suspenso até ali (inofensivo) e APAGA o
/// arquivo — e o laço de suspensão, ainda no meio, suspende o restante sem
/// registro nenhum. É o mesmo defeito que este conserto inteiro existe para
/// fechar, só que numa janela de milissegundos em vez de minutos.
///
/// A tranca não guarda nada além do direito de andar sozinho.
static TRANCA: Mutex<()> = Mutex::new(());

/// Tempo máximo que um caminho de devolução espera pela tranca antes de
/// seguir sem ela.
///
/// O gancho de fim de sessão roda na thread da janela, sob o orçamento curto
/// que o Windows dá antes de considerar o processo travado e matá-lo. Ficar
/// preso esperando uma tranca que a thread de fundo está segurando é pior do
/// que devolver sem ela: o cenário sem tranca é a corrida rara descrita
/// acima, que na pior das hipóteses deixa ALGO suspenso — o defeito que este
/// conserto já reduziu de "sempre que o Otimiza morre" para "raríssimo, só
/// com coincidência de milissegundos". Travar o desligamento do cliente, em
/// troca, afeta TODO cliente que desligar com qualquer coisa suspensa,
/// corrida ou não. Por isso: tenta por um tempo curto, e se não conseguir,
/// segue em frente sem a tranca — retomar sem ela é pior do que com ela, mas
/// é muito melhor do que segurar o logoff.
const PRAZO_TRANCA: Duration = Duration::from_millis(300);

/// Tenta segurar `TRANCA` até `prazo`, tentando a cada 10ms.
///
/// Devolve `None` tanto por prazo esgotado quanto por tranca envenenada
/// (alguém entrou em pânico segurando-a): nos dois casos a resposta certa
/// para quem chama é a mesma — seguir sem a tranca —, e distinguir os dois
/// motivos não mudaria nada.
fn tentar_travar(prazo: Duration) -> Option<MutexGuard<'static, ()>> {
    let comeco = Instant::now();

    loop {
        match TRANCA.try_lock() {
            Ok(guarda) => return Some(guarda),
            Err(TryLockError::Poisoned(_)) => return None,
            Err(TryLockError::WouldBlock) => {
                if comeco.elapsed() >= prazo {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

/// Suspende o que estiver rodando em segundo plano e puder ser suspenso.
///
/// Devolve o que foi suspenso, para a interface poder dizer o que fez. Mudança
/// silenciosa no sistema é exatamente o que este produto critica nos outros.
pub fn suspender_fundo() -> Result<Vec<Suspenso>, String> {
    use super::anticheat::{self, Acao};

    let candidatos = super::processes::listar_para_suspensao();

    // ANTICHEAT PRIMEIRO, ANTES DE QUALQUER THREAD PARAR.
    //
    // Suspender thread de processo alheio é primitiva clássica de trapaça.
    // Enquanto a lista de jogos tinha cinco nomes e três eram GTA, isso quase
    // nunca encostava num anticheat. Com Valorant, Fortnite e PUBG na lista,
    // encosta — e um cliente banido por causa do Otimiza é pior do que um
    // cliente com o PC travando: travamento se conserta, conta não volta.
    let nomes: Vec<String> = candidatos.iter().map(|(_, nome, _)| nome.clone()).collect();
    let presencas = anticheat::detectar(&nomes);

    let permissao = anticheat::permite(Acao::SuspenderFundo, &presencas);
    if let Some(motivo) = permissao.motivo() {
        return Err(motivo.to_string());
    }

    // O lançador da loja tem regra própria: o anticheat conversa com ele
    // durante a partida, então suspendê-lo derruba a sessão mesmo quando
    // suspender o resto é seguro.
    let pode_lancador = anticheat::permite(Acao::SuspenderLancador, &presencas).pode();

    let alvos: Vec<Suspenso> = candidatos
        .into_iter()
        .filter_map(|(pid, nome, inicio)| {
            if !pode_lancador && e_lancador(&nome) {
                return None;
            }

            pode_suspender(&nome).map(|visivel| Suspenso {
                pid,
                nome,
                visivel: visivel.to_string(),
                inicio,
            })
        })
        .collect();

    if alvos.is_empty() {
        return Ok(Vec::new());
    }

    // Tranca o intervalo inteiro entre gravar e suspender — ver o comentário
    // de `TRANCA`. Bloqueia sem prazo, e de propósito: quem chama está numa
    // thread de fundo, não na thread da janela sob orçamento do Windows, e
    // encurtar essa espera é o que reabriria a corrida que esta tranca
    // existe para fechar. Uma tranca envenenada (chamador anterior entrou em
    // pânico no meio) não pode travar a suspensão para sempre — por isso
    // `unwrap_or_else` recupera a tranca em vez de propagar o pânico alheio.
    let _guarda = TRANCA.lock().unwrap_or_else(|env| env.into_inner());

    // GRAVA ANTES DE SUSPENDER. Ver a explicação em `Registro::save`.
    let mut registro = Registro::load();
    for alvo in &alvos {
        if !registro.suspensos.iter().any(|s| s.pid == alvo.pid) {
            registro.suspensos.push(alvo.clone());
        }
    }
    // Marca agora como o instante da suspensão. É o relógio que a rede de
    // segurança por prazo usa para saber que algo ficou suspenso tempo
    // demais — ver `retomar_se_expirado`.
    registro.quando = agora_epoch();
    registro.save()?;

    let mut feitos = Vec::new();

    for alvo in alvos {
        if api::suspender(alvo.pid) > 0 {
            feitos.push(alvo);
        }
    }

    Ok(feitos)
}

/// Segundos desde a época Unix, agora.
///
/// `UNIX_EPOCH` é sempre anterior a `now()`, então o `unwrap_or` só entraria
/// com relógio do sistema quebrado — situação em que "zero" é uma resposta
/// segura: no pior caso a rede de segurança por prazo age cedo demais, nunca
/// tarde demais.
fn agora_epoch() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// O PID ainda é o mesmo processo que o Otimiza suspendeu?
///
/// Separada da chamada ao Windows de propósito: é lógica pura, testável sem
/// sistema real, e é exatamente o ponto onde um descuido devolveria memória
/// de um programa que nunca suspendemos. O Windows recicla PIDs — sem esta
/// conferência, `retomar_tudo` (e qualquer outro caminho de devolução)
/// poderia "retomar" um processo novo que por acaso nasceu com o mesmo
/// número de um Discord que já tinha fechado.
fn ainda_e_o_mesmo_processo(suspenso: &Suspenso, vivos: &HashSet<(u32, u64)>) -> bool {
    vivos.contains(&(suspenso.pid, suspenso.inicio))
}

/// Devolve tudo que o Otimiza suspendeu.
///
/// Usa a MESMA conferência de PID que `retomar_pendentes`: até esta correção,
/// este era o único dos dois caminhos de devolução que confiava só no
/// número do PID, e o Windows recicla PIDs constantemente. Um cliente que
/// abrisse um programa novo bem na hora em que o jogo fechasse corria o
/// risco de o Otimiza mexer num processo que nunca suspendeu.
pub fn retomar_tudo() -> Result<Vec<Suspenso>, String> {
    let registro = Registro::load();

    // Tranca com prazo — ver `TRANCA` e `PRAZO_TRANCA`. Esta é a função
    // chamada pelo gancho de fim de sessão do Windows e pelo fechamento do
    // Otimiza, então NUNCA pode esperar indefinidamente.
    let _guarda = tentar_travar(PRAZO_TRANCA);

    let vivos: HashSet<(u32, u64)> = super::processes::listar_para_suspensao()
        .into_iter()
        .map(|(pid, _, inicio)| (pid, inicio))
        .collect();

    let mut devolvidos = Vec::new();

    for suspenso in registro.suspensos {
        if ainda_e_o_mesmo_processo(&suspenso, &vivos) && api::retomar(suspenso.pid) > 0 {
            devolvidos.push(suspenso);
        }
    }

    Registro::limpar()?;
    Ok(devolvidos)
}

/// O registro está parado tempo demais sem nenhum jogo por perto?
///
/// Função pura — o relógio e a resposta do detector de jogo entram como
/// parâmetro — para não depender de sistema real nos testes.
fn tempo_esgotado(quando: u64, agora: u64, limite_segundos: u64) -> bool {
    agora.saturating_sub(quando) >= limite_segundos
}

/// Rede de segurança por PRAZO: nada fica suspenso indefinidamente.
///
/// As outras duas redes de segurança (`retomar_pendentes` na abertura do
/// programa, e a devolução ao fechar o Otimiza ou ao encerrar a sessão do
/// Windows) cobrem "o programa não está mais rodando". Esta cobre o caso em
/// que o Otimiza CONTINUA rodando, mas por alguma falha de estado — o
/// registro de mudanças e o registro de suspensão são dois arquivos
/// separados, e podem, em tese, sair de sincronia — o modo jogo não percebe
/// que deveria devolver os processos.
///
/// Por isso a decisão de agir aqui não depende do estado do modo jogo, só de
/// dois fatos observáveis: há processos suspensos, e não há jogo nenhum
/// rodando agora. Enquanto um jogo estiver aberto, o prazo nunca vence — por
/// mais longa que seja a partida — porque a pergunta "há jogo agora" é
/// checada de novo a cada chamada.
///
/// Dez minutos: tempo generoso para não competir com o caminho normal (que
/// devolve na hora que o jogo fecha, a cada passo de seis segundos do vigia)
/// e ainda assim curto o suficiente para o cliente não conviver com um
/// Discord congelado por uma tarde inteira quando alguma coisa deu errado.
pub const PRAZO_MAXIMO_SEGUNDOS: u64 = 10 * 60;

pub fn retomar_se_expirado(limite_segundos: u64) -> Vec<Suspenso> {
    retomar_se_expirado_com(
        &Registro::path(),
        limite_segundos,
        agora_epoch(),
        super::gamemode::jogo_aberto().is_some(),
    )
}

/// A MESMA fiação de `retomar_se_expirado`, com o caminho do registro
/// trocável.
///
/// Existe só para o teste `a_fiacao_publica_usa_o_jogo_de_verdade`. Sem esta
/// costura, testar a fiação de verdade — a linha `jogo_aberto().is_some()`,
/// e não um booleano escolhido à mão como todo teste de `retomar_se_expirado_com`
/// já faz — exigiria escrever no `suspensos.json` do PRODUTO, e este módulo
/// não toca nele nos testes (ver `caminho_de_teste`, na seção de testes, para
/// o porquê). O detector de jogo continua sendo o de verdade: só o arquivo
/// muda.
#[cfg(test)]
fn retomar_se_expirado_no_caminho(caminho: &Path, limite_segundos: u64) -> Vec<Suspenso> {
    retomar_se_expirado_com(
        caminho,
        limite_segundos,
        agora_epoch(),
        super::gamemode::jogo_aberto().is_some(),
    )
}

fn retomar_se_expirado_com(
    caminho: &Path,
    limite_segundos: u64,
    agora: u64,
    jogo_rodando: bool,
) -> Vec<Suspenso> {
    let registro = Registro::load_de(caminho);

    if registro.suspensos.is_empty() || jogo_rodando {
        return Vec::new();
    }

    if !tempo_esgotado(registro.quando, agora, limite_segundos) {
        return Vec::new();
    }

    // Tranca com prazo — ver `TRANCA` e `PRAZO_TRANCA`. Roda no mesmo vigia
    // de seis segundos que já chama `suspender_fundo`/`retomar_tudo`, então
    // o bloqueio nunca é o caso comum; é só a rede de segurança para o dia em
    // que essas chamadas migrarem de thread.
    let _guarda = tentar_travar(PRAZO_TRANCA);

    let vivos: HashSet<(u32, u64)> = super::processes::listar_para_suspensao()
        .into_iter()
        .map(|(pid, _, inicio)| (pid, inicio))
        .collect();

    let mut devolvidos = Vec::new();

    for suspenso in registro.suspensos {
        if ainda_e_o_mesmo_processo(&suspenso, &vivos) && api::retomar(suspenso.pid) > 0 {
            devolvidos.push(suspenso);
        }
    }

    let _ = Registro::limpar_em(caminho);
    devolvidos
}

/// Roda na abertura do programa, antes de qualquer outra coisa.
///
/// É a rede de segurança do módulo inteiro: se o Otimiza foi fechado à força,
/// travou ou perdeu energia com processos suspensos, é aqui que eles voltam.
///
/// A conferência do instante de início existe porque o Windows recicla PIDs:
/// sem ela, o Otimiza poderia "retomar" um processo novo que nunca suspendeu.
/// Retomar um processo que já roda é inofensivo, mas mexer num programa que
/// não é o nosso não é uma coisa que este produto faça.
pub fn retomar_pendentes() -> Vec<Suspenso> {
    retomar_pendentes_em(&Registro::path())
}

fn retomar_pendentes_em(caminho: &Path) -> Vec<Suspenso> {
    let registro = Registro::load_de(caminho);

    if registro.suspensos.is_empty() {
        return Vec::new();
    }

    let vivos: HashSet<(u32, u64)> = super::processes::listar_para_suspensao()
        .into_iter()
        .map(|(pid, _, inicio)| (pid, inicio))
        .collect();

    let mut devolvidos = Vec::new();

    for suspenso in registro.suspensos {
        if ainda_e_o_mesmo_processo(&suspenso, &vivos) && api::retomar(suspenso.pid) > 0 {
            devolvidos.push(suspenso);
        }
    }

    let _ = Registro::limpar_em(caminho);
    devolvidos
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um registro só deste teste, em pasta própria.
    ///
    /// O `cargo test` roda os testes em threads paralelas do MESMO processo, e
    /// o registro do produto é um caminho único em %APPDATA%. Enquanto os
    /// testes usavam esse caminho, um apagava o arquivo no exato intervalo em
    /// que o outro gravava e relia — e a suíte reprovava uma publicação sem
    /// existir defeito no produto.
    ///
    /// O nome do processo entra no caminho porque duas execuções de `cargo
    /// test` ao mesmo tempo na mesma máquina disputariam a pasta de novo.
    ///
    /// De quebra, teste nenhum encosta mais no registro real: rodar a suíte
    /// com o Otimiza aberto e processos suspensos apagava a rede de segurança
    /// que devolve esses processos.
    fn caminho_de_teste(nome: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("otimiza-suspend-{}-{}", std::process::id(), nome))
            .join("suspensos.json")
    }

    #[test]
    fn o_discord_pode_ser_suspenso_e_o_audio_nunca() {
        assert_eq!(pode_suspender("Discord.exe"), Some("Discord"));
        assert_eq!(pode_suspender("chrome.exe"), Some("Google Chrome"));

        // Suspender o áudio corta o som do jogo — o oposto do objetivo.
        assert_eq!(pode_suspender("audiodg.exe"), None);
        // Suspender o antivírus é desligar a proteção sem avisar o cliente.
        assert_eq!(pode_suspender("MsMpEng.exe"), None);
        // Suspender o explorador congela a barra de tarefas.
        assert_eq!(pode_suspender("explorer.exe"), None);
    }

    #[test]
    fn o_otimiza_nunca_suspende_a_si_mesmo() {
        // Quem devolve os processos é ele. Suspender a si mesmo deixaria tudo
        // congelado até o cliente reiniciar o PC.
        assert_eq!(pode_suspender("pc-optimizer.exe"), None);
        assert_eq!(pode_suspender("Otimiza.exe"), None);
    }

    #[test]
    fn o_jogo_em_execucao_nunca_e_suspenso() {
        for jogo in super::super::gamemode::JOGOS {
            // As chaves por pedaço não são nome de arquivo — carregam o número
            // da compilação no meio. Monta-se um nome plausível para elas.
            let nome = if jogo.por_pedaco {
                format!("{}b0000_GTAProcess.exe", jogo.chave)
            } else {
                jogo.chave.to_string()
            };

            assert_eq!(
                pode_suspender(&nome),
                None,
                "{} não pode ser suspenso: é o jogo",
                nome
            );
        }
    }

    #[test]
    fn programa_desconhecido_nao_e_suspenso() {
        // A lista é permissiva por inclusão, não por exclusão. Suspender "tudo
        // que não reconheço" é como se congela o programa de trabalho do
        // cliente, ou o driver de um periférico que ninguém previu.
        assert_eq!(pode_suspender("ProgramaDoCliente.exe"), None);
        assert_eq!(pode_suspender("algum_servico_qualquer.exe"), None);
        assert_eq!(pode_suspender(""), None);
    }

    #[test]
    fn a_proibicao_vence_a_permissao() {
        // Se alguém acrescentar um nome nas duas listas por engano, o
        // resultado seguro precisa ser "não suspende".
        for proibido in NUNCA_SUSPENDER {
            assert_eq!(
                pode_suspender(proibido),
                None,
                "{} está na lista de proibidos e mesmo assim passou",
                proibido
            );
        }
    }

    #[test]
    fn nenhum_suspensivel_esta_na_lista_de_proibidos() {
        // Guarda contra contradição na própria configuração do módulo.
        for (exe, visivel) in SUSPENSIVEIS {
            assert!(
                !NUNCA_SUSPENDER.contains(exe),
                "{} ({}) está nas duas listas",
                exe,
                visivel
            );
        }
    }

    #[test]
    fn suspender_nunca_vira_matar() {
        // A distinção entre este módulo e um "otimizador" qualquer é esta
        // linha. Suspender devolve o programa como estava; matar faz o cliente
        // perder o que não salvou. Se alguém trocar uma coisa pela outra numa
        // pressa futura, o teste quebra antes de virar release.
        let fonte = include_str!("suspend.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();

        // A lista cobre só o que MATA. `NtSuspendProcess` fica de fora de
        // propósito: ela suspende, não mata, e o comentário que explica por que
        // não a usamos é documentação que precisa continuar no arquivo.
        for proibido in [
            "TerminateProcess",
            "Stop-Process",
            "taskkill",
            "TerminateThread",
            "EmptyWorkingSet",
        ] {
            assert!(
                !producao.contains(proibido),
                "`{}` apareceu na suspensão",
                proibido
            );
        }
    }

    #[test]
    fn retomar_sem_nada_pendente_nao_faz_nada() {
        // O caminho que roda em toda abertura do programa. Não pode explodir
        // nem mexer em processo nenhum quando o registro está vazio.
        let caminho = caminho_de_teste("retomar-sem-nada");

        Registro::limpar_em(&caminho).expect("limpar o registro");
        assert!(retomar_pendentes_em(&caminho).is_empty());
    }

    /// Suspende e devolve um processo de verdade.
    ///
    /// Marcado como ignorado porque cria um processo real: numa esteira de
    /// integração isso é frágil, e um teste que falha por motivo alheio ensina
    /// a equipe a ignorar teste vermelho. Rodar na mão com:
    ///
    /// ```text
    /// cargo test --lib -- suspende_e_devolve_um_processo_de_verdade --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    #[cfg(target_os = "windows")]
    fn suspende_e_devolve_um_processo_de_verdade() {
        use std::process::{Command, Stdio};

        // `ping` e não `powershell`: o alvo precisa ficar vivo uns trinta
        // segundos sem ler a entrada padrão, e existe em qualquer Windows.
        // (Chamar o PowerShell direto aqui também faria a guarda de UTF-8 do
        // projeto reprovar este arquivo, e com razão.)
        let mut filho = Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(Stdio::null())
            .spawn()
            .expect("criar o processo de teste");

        let pid = filho.id();
        std::thread::sleep(std::time::Duration::from_millis(400));

        let estado = |quando: &str| -> String {
            let saida = super::super::shell::powershell(&format!(
                "(Get-Process -Id {} -ErrorAction SilentlyContinue).Threads \
                 | Group-Object WaitReason | ForEach-Object {{ \
                   \"$($_.Name)=$($_.Count)\" }}",
                pid
            ))
            .expect("consultar as threads");

            let texto = saida.stdout.trim().to_string();
            println!("  {}: {}", quando, texto.replace('\n', " "));
            texto
        };

        estado("antes");

        let threads = api::suspender(pid);
        assert!(threads > 0, "nenhuma thread foi suspensa");
        let depois = estado("suspenso");

        // `Suspended` é a razão de espera que o Windows atribui a uma thread
        // parada por SuspendThread. Se não aparecer, o módulo não está fazendo
        // o que promete — e o cliente teria um Discord pausado à toa.
        assert!(
            depois.contains("Suspended"),
            "o processo deveria estar suspenso, e está: {}",
            depois
        );

        let devolvidas = api::retomar(pid);
        assert!(devolvidas > 0, "nenhuma thread foi devolvida");
        let voltou = estado("devolvido");

        assert!(
            !voltou.contains("Suspended"),
            "o processo continuou suspenso depois de retomar — este é o pior \
             defeito possível neste módulo: {}",
            voltou
        );

        let _ = filho.kill();
        let _ = filho.wait();
    }

    #[test]
    fn o_registro_sobrevive_a_ida_e_volta_do_disco() {
        // É este arquivo que impede um Discord de ficar congelado para sempre
        // se o Otimiza morrer no meio.
        let registro = Registro {
            suspensos: vec![Suspenso {
                pid: 4242,
                nome: "discord.exe".to_string(),
                visivel: "Discord".to_string(),
                inicio: 133_000_000,
            }],
            quando: 1_700_000_000,
        };

        let caminho = caminho_de_teste("ida-e-volta");

        registro.save_em(&caminho).expect("gravar o registro");
        let lido = Registro::load_de(&caminho);
        assert_eq!(lido.suspensos, registro.suspensos);
        assert_eq!(lido.quando, registro.quando);

        Registro::limpar_em(&caminho).expect("limpar o registro");
        assert!(Registro::load_de(&caminho).suspensos.is_empty());
    }

    #[test]
    fn registro_antigo_sem_data_carrega_como_zero() {
        // Um registro gravado por uma versão anterior do Otimiza não tem o
        // campo `quando`. Sem `#[serde(default)]` a leitura falharia inteira
        // — inclusive para os PIDs suspensos, que são o que realmente
        // importa devolver.
        let caminho = caminho_de_teste("registro-sem-data");

        if let Some(dir) = caminho.parent() {
            std::fs::create_dir_all(dir).expect("criar pasta de teste");
        }
        std::fs::write(&caminho, r#"{"suspensos":[]}"#).expect("gravar registro antigo");

        let lido = Registro::load_de(&caminho);
        assert_eq!(lido.quando, 0);

        Registro::limpar_em(&caminho).expect("limpar o registro");
    }

    #[test]
    fn retomar_tudo_recusa_pid_reciclado() {
        // O defeito que esta trava fecha: `retomar_tudo` confiava só no PID.
        // Se o Windows reciclou o número para um processo novo — o cliente
        // abriu outro programa bem na hora em que o jogo fechou —, o Otimiza
        // não pode mexer nele. É a mesma conferência que `retomar_pendentes`
        // já fazia; agora as duas passam pela mesma função.
        let suspenso = Suspenso {
            pid: 4242,
            nome: "discord.exe".to_string(),
            visivel: "Discord".to_string(),
            inicio: 133_000_000,
        };

        // PID igual, mas o processo vivo agora começou em outro instante:
        // não é o mesmo Discord que suspendemos.
        let mut vivos = HashSet::new();
        vivos.insert((4242u32, 999_000_000u64));
        assert!(
            !ainda_e_o_mesmo_processo(&suspenso, &vivos),
            "aceitou um PID reciclado como se fosse o processo suspenso"
        );

        // PID e instante de início batem: é o mesmo processo, e a devolução
        // pode prosseguir.
        let mut vivos = HashSet::new();
        vivos.insert((4242u32, 133_000_000u64));
        assert!(
            ainda_e_o_mesmo_processo(&suspenso, &vivos),
            "recusou o próprio processo que suspendemos"
        );

        // PID nem sequer está mais na lista de processos vivos.
        let vivos = HashSet::new();
        assert!(!ainda_e_o_mesmo_processo(&suspenso, &vivos));
    }

    #[test]
    fn tempo_esgotado_respeita_o_limite() {
        // Antes do limite, nada de forçar devolução.
        assert!(!tempo_esgotado(1_000, 1_000 + 599, 600));
        // No limite exato e depois dele, sim.
        assert!(tempo_esgotado(1_000, 1_000 + 600, 600));
        assert!(tempo_esgotado(1_000, 1_000 + 700, 600));
        // Relógio andando para trás (registro do futuro, relógio ajustado)
        // não pode subtrair estourando: `saturating_sub` é o que impede
        // isso de virar um número gigante por overflow.
        assert!(!tempo_esgotado(2_000, 1_000, 600));
    }

    #[test]
    fn retomar_se_expirado_nao_mexe_com_jogo_rodando() {
        // A garantia mais importante desta rede de segurança: por mais longa
        // que seja a partida, o prazo nunca vence enquanto há jogo aberto.
        let caminho = caminho_de_teste("prazo-com-jogo");

        let registro = Registro {
            suspensos: vec![Suspenso {
                pid: 4242,
                nome: "discord.exe".to_string(),
                visivel: "Discord".to_string(),
                inicio: 133_000_000,
            }],
            quando: 0,
        };
        registro.save_em(&caminho).expect("gravar o registro");

        // Prazo estourado há muito tempo (quando=0, agora=um milhão de
        // segundos depois), mas com jogo_rodando=true nada pode acontecer.
        let devolvidos = retomar_se_expirado_com(&caminho, 600, 1_000_000, true);
        assert!(devolvidos.is_empty());
        assert!(
            !Registro::load_de(&caminho).suspensos.is_empty(),
            "o registro foi apagado mesmo com um jogo rodando"
        );

        Registro::limpar_em(&caminho).expect("limpar o registro");
    }

    #[test]
    fn retomar_se_expirado_nao_mexe_antes_do_prazo() {
        let caminho = caminho_de_teste("prazo-ainda-nao");

        let registro = Registro {
            suspensos: vec![Suspenso {
                pid: 4242,
                nome: "discord.exe".to_string(),
                visivel: "Discord".to_string(),
                inicio: 133_000_000,
            }],
            quando: 1_000,
        };
        registro.save_em(&caminho).expect("gravar o registro");

        // Sem jogo rodando, mas ainda dentro do prazo: não é hora de agir.
        let devolvidos = retomar_se_expirado_com(&caminho, 600, 1_000 + 100, false);
        assert!(devolvidos.is_empty());
        assert!(!Registro::load_de(&caminho).suspensos.is_empty());

        Registro::limpar_em(&caminho).expect("limpar o registro");
    }

    #[test]
    fn retomar_se_expirado_sem_nada_suspenso_nao_faz_nada() {
        let caminho = caminho_de_teste("prazo-vazio");
        Registro::limpar_em(&caminho).expect("limpar o registro");

        assert!(retomar_se_expirado_com(&caminho, 600, 1_000_000, false).is_empty());
    }

    #[test]
    fn a_fiacao_publica_usa_o_jogo_de_verdade() {
        // Todo teste acima passa pelo `_com`, com o booleano do jogo
        // escolhido à mão — nenhum deles pegaria uma inversão na fiação real
        // de `retomar_se_expirado` (`.is_none()` no lugar de `.is_some()`,
        // ou os argumentos trocados). Este teste chama a MESMA linha que a
        // função pública chama — só o caminho do arquivo é de teste, o
        // detector de jogo é o de verdade.
        //
        // A conferência é o ESTADO DO ARQUIVO depois da chamada, não a lista
        // devolvida: o PID gravado abaixo é inventado, então nenhum processo
        // vivo de verdade vai bater com ele, e `ainda_e_o_mesmo_processo`
        // vai recusar — como deve ser, esse é o assunto de outro teste. O
        // que esta conferência isola é só o portão de entrada da função:
        // com `jogo_rodando` errado, `retomar_se_expirado_com` devolve cedo
        // (ver o primeiro `if` do corpo dela) e NUNCA chega a limpar o
        // arquivo; com `jogo_rodando` certo, ela passa do portão e limpa.
        //
        // A esteira roda sem sessão gráfica: não há jogo para detectar, e
        // `jogo_aberto()` de verdade devolve `None` — a mesma suposição que
        // `gamemode::tests::detecta_jogo_nesta_maquina` já faz. Com a fiação
        // certa, "nenhum jogo" vira `jogo_rodando = false`, passa o portão, e
        // o arquivo é limpo; com a fiação invertida, o mesmo "nenhum jogo"
        // viraria `jogo_rodando = true`, o portão barraria, e o arquivo
        // continuaria com o registro vencido — é essa inversão que este
        // teste tranca.
        let caminho = caminho_de_teste("fiacao-prazo");

        let registro = Registro {
            suspensos: vec![Suspenso {
                pid: 4242,
                nome: "discord.exe".to_string(),
                visivel: "Discord".to_string(),
                inicio: 133_000_000,
            }],
            quando: 0,
        };
        registro.save_em(&caminho).expect("gravar o registro");

        if super::super::gamemode::jogo_aberto().is_some() {
            // Máquina de quem desenvolve pode ter um jogo de verdade aberto.
            // Sem controle sobre isso o teste não pode afirmar nada — e não
            // deve reprovar por um motivo alheio à fiação.
            Registro::limpar_em(&caminho).expect("limpar o registro");
            return;
        }

        // Prazo zero: com `quando = 0`, qualquer relógio atual já esgotou.
        retomar_se_expirado_no_caminho(&caminho, 0);

        assert!(
            Registro::load_de(&caminho).suspensos.is_empty(),
            "o registro vencido continuou no arquivo mesmo sem jogo nenhum rodando — \
             confira se `retomar_se_expirado` ainda passa `jogo_aberto().is_some()`, e não \
             `.is_none()` nem os argumentos fora de ordem"
        );

        Registro::limpar_em(&caminho).expect("limpar o registro");
    }
}
