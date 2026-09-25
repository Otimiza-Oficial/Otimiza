// Devolve o que versões antigas deixaram suspenso. Este módulo NÃO congela mais nada: até a 1.9 o modo jogo
// suspendia Discord e navegador, e foi o que mais machucou cliente (Explorador sem abrir no login seguinte, Steam
// congelada). Saiu na 2.0, e `o_produto_nao_congela_mais_nenhum_programa` reprova o build se voltar. Ficou só a
// metade que devolve: `retomar_pendentes()` na abertura e as outras redes, que com registro vazio não fazem nada.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Suspenso {
    pub pid: u32,
    pub nome: String,
    pub visivel: String,
    /// Assinatura: o Windows recicla PIDs, e sem isto se retomaria um processo que nunca foi suspenso.
    pub inicio: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registro {
    pub suspensos: Vec<Suspenso>,
    /// `serde(default)`: registro de versão anterior não tem o campo e vale como "há muito tempo", sem travar a leitura.
    #[serde(default)]
    pub quando: u64,
}

impl Registro {
    /// Os testes não passam por aqui: usam `_de`/`_em` com arquivo próprio (ver `caminho_de_teste`).
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

    /// Só os testes gravam: desde a 2.0 o produto só LÊ o que versões antigas deixaram.
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
        // Ausente é o estado desejado. Perguntar `exists()` antes abriria uma janela entre pergunta e resposta.
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Não foi possível limpar o registro: {}", e)),
        }
    }
}

#[cfg(target_os = "windows")]
mod api {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::Threading::{
        OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
    };

    /// Não há chamada pública que retome o processo inteiro: percorre as threads.
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
        // Cada `SuspendThread` incrementa um contador: retomar uma vez só deixaria congelado quem foi suspenso duas.
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

/// Três caminhos disparam a devolução (fim de sessão, fechamento, vigia) e dois lendo e apagando o mesmo arquivo
/// juntos é uma corrida.
static TRANCA: Mutex<()> = Mutex::new(());

/// O fim de sessão roda sob o orçamento curto do Windows: travar o logoff de todo cliente é pior que a corrida
/// rara sem tranca. Tenta um pouco e segue sem ela.
const PRAZO_TRANCA: Duration = Duration::from_millis(300);

/// `None` por prazo ou por tranca envenenada: nos dois casos quem chama segue sem ela.
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

/// Relógio quebrado vira zero: a rede por prazo age cedo demais, nunca tarde demais.
fn agora_epoch() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Pura: é o ponto onde um descuido mexeria num processo novo com o PID reciclado.
fn ainda_e_o_mesmo_processo(suspenso: &Suspenso, vivos: &HashSet<(u32, u64)>) -> bool {
    vivos.contains(&(suspenso.pid, suspenso.inicio))
}

/// A mesma conferência de PID de `retomar_pendentes`.
pub fn retomar_tudo() -> Result<Vec<Suspenso>, String> {
    let registro = Registro::load();

    // Chamado no fim de sessão e no fechamento: NUNCA espera indefinidamente.
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

fn tempo_esgotado(quando: u64, agora: u64, limite_segundos: u64) -> bool {
    agora.saturating_sub(quando) >= limite_segundos
}

/// Rede por PRAZO, para o Otimiza que CONTINUA rodando com os dois registros fora de sincronia. Depende só de
/// haver suspensos e não haver jogo agora (reconferido a cada chamada). Dez minutos: não compete com o caminho
/// normal e não deixa um Discord congelado a tarde inteira.
pub const PRAZO_MAXIMO_SEGUNDOS: u64 = 10 * 60;

pub fn retomar_se_expirado(limite_segundos: u64) -> Vec<Suspenso> {
    retomar_se_expirado_com(
        &Registro::path(),
        limite_segundos,
        agora_epoch(),
        super::gamemode::jogo_aberto().is_some(),
    )
}

/// Costura só para `a_fiacao_publica_usa_o_jogo_de_verdade`: o detector de jogo é o real, só o arquivo muda.
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

/// Na abertura, antes de tudo: a rede para Otimiza fechado à força, travado ou sem energia com suspensos.
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

    /// Pasta própria por teste e por processo: com o caminho único em %APPDATA%, testes paralelos se apagavam e
    /// reprovavam publicação sem defeito, e rodar a suíte com suspensos reais apagava a rede de segurança.
    fn caminho_de_teste(nome: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("otimiza-suspend-{}-{}", std::process::id(), nome))
            .join("suspensos.json")
    }

    #[test]
    fn devolver_nunca_vira_matar() {
        // Devolver, nunca matar: matar faz perder o que não foi salvo.
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
        let caminho = caminho_de_teste("retomar-sem-nada");

        Registro::limpar_em(&caminho).expect("limpar o registro");
        assert!(retomar_pendentes_em(&caminho).is_empty());
    }

    #[test]
    fn o_registro_sobrevive_a_ida_e_volta_do_disco() {
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
        let suspenso = Suspenso {
            pid: 4242,
            nome: "discord.exe".to_string(),
            visivel: "Discord".to_string(),
            inicio: 133_000_000,
        };

        let mut vivos = HashSet::new();
        vivos.insert((4242u32, 999_000_000u64));
        assert!(
            !ainda_e_o_mesmo_processo(&suspenso, &vivos),
            "aceitou um PID reciclado como se fosse o processo suspenso"
        );

        let mut vivos = HashSet::new();
        vivos.insert((4242u32, 133_000_000u64));
        assert!(
            ainda_e_o_mesmo_processo(&suspenso, &vivos),
            "recusou o próprio processo que suspendemos"
        );

        let vivos = HashSet::new();
        assert!(!ainda_e_o_mesmo_processo(&suspenso, &vivos));
    }

    #[test]
    fn tempo_esgotado_respeita_o_limite() {
        assert!(!tempo_esgotado(1_000, 1_000 + 599, 600));
        assert!(tempo_esgotado(1_000, 1_000 + 600, 600));
        assert!(tempo_esgotado(1_000, 1_000 + 700, 600));
        // `saturating_sub`: relógio para trás não pode estourar.
        assert!(!tempo_esgotado(2_000, 1_000, 600));
    }

    #[test]
    fn retomar_se_expirado_nao_mexe_com_jogo_rodando() {
        // Por mais longa que seja a partida, o prazo nunca vence com jogo aberto.
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
        // Os testes acima escolhem o booleano do jogo à mão; este chama a mesma linha da função pública, com o detector
        // real e sem sessão gráfica (`jogo_aberto()` = `None`). Confere o ESTADO DO ARQUIVO: com a fiação certa o portão
        // passa e limpa; invertida, o registro vencido fica.
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
            // Máquina de desenvolvimento pode ter jogo aberto: aí o teste não afirma nada.
            Registro::limpar_em(&caminho).expect("limpar o registro");
            return;
        }

        retomar_se_expirado_no_caminho(&caminho, 0);

        assert!(
            Registro::load_de(&caminho).suspensos.is_empty(),
            "o registro vencido continuou no arquivo mesmo sem jogo nenhum rodando — \
             confira se `retomar_se_expirado` ainda passa `jogo_aberto().is_some()`, e não \
             `.is_none()` nem os argumentos fora de ordem"
        );

        Registro::limpar_em(&caminho).expect("limpar o registro");
    }

    /// NENHUM CAMINHO DO PRODUTO SUSPENDE UMA THREAD: varre o código inteiro atrás das chamadas e reprova o build.
    /// Comentários são descartados antes, para a história poder continuar escrita.
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

        // Em pedaços para a lista não casar consigo mesma.
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
