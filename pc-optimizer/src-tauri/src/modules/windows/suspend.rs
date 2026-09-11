// Devolver o que versões antigas deixaram suspenso
//
// ESTE MÓDULO NÃO CONGELA MAIS NADA, E O MOTIVO FICA ESCRITO AQUI.
//
// Até a 1.9, o modo jogo automático suspendia Discord, navegador e afins
// durante a partida, para devolver memória ao jogo. A ideia era melhor que a
// do mercado — suspender em vez de matar, nada se perdia —, e mesmo assim foi
// a opção que mais machucou cliente:
//
//   - 1.1.1: um programa suspenso não responde ao aviso de desligamento, o
//     Windows não descarrega o perfil direito, e no login seguinte o
//     Explorador e a barra de tarefas não abriam nada;
//   - 1.1.2: a Steam congelada não abria jogo, não baixava, não respondia;
//   - e o relatório de suporte (`suporte.rs`) nasceu de um cliente pagante
//     dizendo que os programas não abriam mais.
//
// Cada incidente ganhou uma rede de segurança, e as redes funcionavam. Mas um
// produto que precisa de quatro redes para não quebrar o PC de quem comprou
// está pagando caro demais por memória que o cliente nem vê. Na 2.0 a opção
// saiu inteira: não existe mais caminho no produto que suspenda uma thread. A
// trava `o_produto_nao_congela_mais_nenhum_programa`, nos testes abaixo,
// reprova o build se esse caminho voltar.
//
// O QUE FICOU, E POR QUÊ
//
// Só a metade que DEVOLVE. Quem atualiza de uma versão antiga pode chegar com
// programas registrados como suspensos em disco — o Otimiza velho morreu no
// meio de uma partida, faltou energia. `retomar_pendentes()` roda na primeira
// abertura, antes de tudo, e devolve esses programas. As outras redes (fechar
// o Otimiza, fim de sessão do Windows, prazo de dez minutos) ficam também:
// com o registro vazio, que é o caso de todo mundo daqui para frente, nenhuma
// delas faz nada.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

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

    /// Grava o registro. Só os testes gravam: desde a 2.0 o produto não
    /// suspende nada, então só LÊ o que versões antigas deixaram.
    #[cfg(test)]
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

// --------------------------------------------------------------- Windows API

#[cfg(target_os = "windows")]
mod api {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::Threading::{
        OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
    };

    /// Retoma uma vez cada thread de um processo.
    ///
    /// O Windows não tem uma chamada pública que retome o processo inteiro:
    /// a forma suportada é percorrer as threads, uma a uma.
    ///
    /// Devolve quantas threads responderam.
    fn retomar_cada_thread(pid: u32) -> u32 {
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
                            let resultado = ResumeThread(handle);

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

    pub fn retomar(pid: u32) -> u32 {
        // Retomar é chamado repetidamente até a thread destravar: cada
        // `SuspendThread` incrementa um contador, e a thread só volta a rodar
        // quando ele zera. Suspender duas vezes e retomar uma deixaria o
        // processo congelado — e é exatamente o defeito que este módulo não
        // pode ter.
        let mut atingidas = 0;

        for _ in 0..8 {
            let n = retomar_cada_thread(pid);
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
    pub fn retomar(_pid: u32) -> u32 {
        0
    }
}

// ------------------------------------------------------------------- ações

/// Serializa as devoluções.
///
/// Até a 1.9 esta tranca existia para o congelamento e a devolução nunca
/// correrem ao mesmo tempo. O congelamento saiu, mas a devolução ainda pode
/// ser disparada de três lugares — o gancho de fim de sessão do Windows, o
/// fechamento do Otimiza e o vigia de seis segundos —, e dois deles lendo e
/// apagando o mesmo arquivo juntos continua sendo uma corrida.
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

    // Tranca com prazo — ver `TRANCA` e `PRAZO_TRANCA`. Roda no vigia de seis
    // segundos, então o bloqueio nunca é o caso comum; é a rede para quando o
    // fim de sessão do Windows cair no mesmo instante.
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
    fn devolver_nunca_vira_matar() {
        // Este módulo só existe para devolver programas do jeito que estavam.
        // Matar faz o cliente perder o que não salvou. Se alguém trocar uma
        // coisa pela outra numa pressa futura, o teste quebra antes de virar
        // release.
        let fonte = include_str!("suspend.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();

        for proibido in [
            "TerminateProcess",
            "Stop-Process",
            "taskkill",
            "TerminateThread",
            "EmptyWorkingSet",
        ] {
            assert!(
                !producao.contains(proibido),
                "`{}` apareceu no módulo que devolve programas",
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

    /// NENHUM CAMINHO DO PRODUTO SUSPENDE UMA THREAD.
    ///
    /// A 2.0 tirou o congelamento porque ele fazia programa do cliente parar de
    /// abrir. Esta trava garante que ele não volta por um atalho: varre o
    /// código inteiro — não só este arquivo — atrás das chamadas que suspendem
    /// processo no Windows, e reprova o build se achar alguma.
    ///
    /// Os comentários são descartados antes de procurar, porque a história do
    /// que foi removido precisa poder continuar escrita.
    #[test]
    fn o_produto_nao_congela_mais_nenhum_programa() {
        fn arquivos(dir: &Path, achados: &mut Vec<PathBuf>) {
            let Ok(entradas) = std::fs::read_dir(dir) else {
                return;
            };

            for entrada in entradas.flatten() {
                let caminho = entrada.path();

                if caminho.is_dir() {
                    arquivos(&caminho, achados);
                } else if caminho.extension().and_then(|e| e.to_str()) == Some("rs") {
                    achados.push(caminho);
                }
            }
        }

        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut todos = Vec::new();
        arquivos(&raiz, &mut todos);

        assert!(
            todos.len() > 40,
            "a varredura achou só {} arquivos — o caminho provavelmente está errado",
            todos.len()
        );

        // Montados em pedaços para esta própria lista não casar consigo mesma.
        let proibidos = [
            ["Suspend", "Thread"].concat(),
            ["NtSuspend", "Process"].concat(),
            ["DebugActive", "Process"].concat(),
            ["suspender", "_fundo"].concat(),
        ];

        let mut culpados = Vec::new();

        for arquivo in &todos {
            let Ok(conteudo) = std::fs::read_to_string(arquivo) else {
                continue;
            };

            let sem_comentarios: String = conteudo
                .lines()
                .map(|l| l.split("//").next().unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\n");

            for proibido in &proibidos {
                if sem_comentarios.contains(proibido.as_str()) {
                    culpados.push(format!("{} em {}", proibido, arquivo.display()));
                }
            }
        }

        assert!(
            culpados.is_empty(),
            "o congelamento de programas voltou ao produto: {:?}. Ele saiu na 2.0 porque fazia programa do cliente parar de abrir. Não é para voltar.",
            culpados
        );
    }
}
