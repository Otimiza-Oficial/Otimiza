// O governador de segundo plano — o que o modo jogo faz desde a 2.9
//
// O PROBLEMA QUE ELE ATACA
//
// Engasgo no jogo muitas vezes não é o jogo: é um atualizador, uma
// sincronização de nuvem, um navegador com vinte abas ou um launcher baixando
// patch, disputando processador com o jogo na hora errada. Isso aparece como
// 1% low ruim e travada, não como FPS médio baixo.
//
// O QUE ELE FAZ — E O QUE NÃO FAZ
//
// Enquanto o jogo roda, todo programa em segundo plano que está USANDO
// processador de verdade (medido, não por lista) recebe:
//
//   1. prioridade "abaixo do normal" (SetPriorityClass);
//   2. EcoQoS (SetProcessInformation com ProcessPowerThrottling /
//      PROCESS_POWER_THROTTLING_EXECUTION_SPEED), que o Windows documenta
//      como "o processo prefere eficiência a desempenho": no Windows 11 ele
//      vai para frequência baixa e, em CPU híbrida, para os núcleos E.
//
// O programa continua rodando — só para de brigar com o jogo. NADA é
// congelado (o congelamento saiu do produto na 2.0 e tem trava contra a
// volta), nada é fechado, e quando o jogo fecha tudo volta ao que era.
//
// E O QUE ELE NUNCA TOCA: o próprio jogo e os processos da pasta dele, voz
// (Discord, TeamSpeak), transmissão (OBS), áudio, anticheats, processos do
// sistema e o próprio Otimiza. Prioridade do JOGO não é mexida: prioridade
// alta cega saiu na 2.9.
//
// Se o Otimiza morrer no meio, a lista do que foi mexido fica em disco e a
// próxima abertura devolve (conferindo que é o mesmo processo pelo horário
// em que ele começou, para não mexer em outro que herdou o mesmo número).
//
// E ELE NÃO AGE SEM PROVA. Acalmar algo de que o jogo depende fora da pasta
// dele (overlay do launcher, `steamwebhelper`) pode custar FPS. Por isso cada
// jogo passa pelo portão "nunca menos FPS" (`modules::portao`): partidas de
// comparação, em que ele só anota quem acalmaria, se alternam com partidas em
// que ele age, até haver medições dos dois lados. Se o FPS médio ou o 1% pior
// caírem (intervalos separados e pelo menos 5%), ele devolve tudo e fica
// parado naquele jogo. Sem medição automática, ele não age.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Uso de processador (fração da máquina inteira) a partir do qual um
/// programa em segundo plano está disputando com o jogo. 3% de uma máquina de
/// 8 núcleos é um quarto de núcleo ocupado sem parar.
const USO_MINIMO_DA_MAQUINA: f64 = 0.03;

/// Nomes que nunca são mexidos (comparação sem extensão, minúsculas, por
/// começo do nome).
const PROTEGIDOS: &[&str] = &[
    // voz, transmissão e áudio
    "discord", "teamspeak", "ts3client", "mumble", "obs64", "obs32", "obs", "streamlabs", "xsplit",
    "nvidia broadcast", "voicemeeter", "spotify", "audiodg",
    // anticheats
    "vgc", "vgtray", "easyanticheat", "beservice", "battleye", "faceit", "eac_launcher", "ricochet",
    // sistema e shell
    "system", "registry", "smss", "csrss", "wininit", "winlogon", "services", "lsass", "svchost", "dwm",
    "explorer", "fontdrvhost", "sihost", "ctfmon", "searchhost", "startmenuexperiencehost",
    "textinputhost", "runtimebroker", "msmpeng", "securityhealthservice", "taskmgr",
    // o próprio Otimiza
    "pc-optimizer", "otimiza",
];

/// Um processo como a regra vê. **Sem nada do Windows**, para ser testável.
#[derive(Debug, Clone)]
pub struct Candidato {
    pub pid: u32,
    pub nome: String,
    pub caminho: Option<PathBuf>,
    /// Fração da máquina inteira (0–1).
    pub uso: f64,
}

pub fn protegido(nome: &str) -> bool {
    let n = nome.to_lowercase();
    let n = n.trim_end_matches(".exe");
    PROTEGIDOS.iter().any(|p| n == *p || n.starts_with(&format!("{}_", p)) || n.starts_with(&format!("{} ", p)) || (p.len() >= 5 && n.starts_with(p)))
}

/// Quem o governador deve acalmar. **Função pura.**
///
/// Fica de fora: o jogo, tudo da pasta do jogo (o FiveM roda em vários
/// processos), tudo com o mesmo prefixo de nome do jogo, os protegidos, e
/// quem não está usando processador de verdade.
pub fn escolher(candidatos: &[Candidato], jogo_pid: u32, jogo_caminho: Option<&Path>, meu_pid: u32) -> Vec<u32> {
    let pasta_do_jogo = jogo_caminho.and_then(|c| c.parent());
    let prefixo_do_jogo = jogo_caminho
        .and_then(|c| c.file_stem())
        .map(|s| s.to_string_lossy().to_lowercase())
        .map(|s| s.split(['_', '-', '.']).next().unwrap_or("").to_string())
        .filter(|s| s.len() >= 3);
    candidatos
        .iter()
        .filter(|c| c.pid != jogo_pid && c.pid != meu_pid && c.pid > 4)
        .filter(|c| c.uso >= USO_MINIMO_DA_MAQUINA)
        .filter(|c| !protegido(&c.nome))
        .filter(|c| match (pasta_do_jogo, &c.caminho) {
            (Some(p), Some(cam)) => !cam.starts_with(p),
            _ => true,
        })
        .filter(|c| match &prefixo_do_jogo {
            Some(p) => !c.nome.to_lowercase().starts_with(p.as_str()),
            None => true,
        })
        .map(|c| c.pid)
        .collect()
}

/// O que estava no EcoQoS do processo antes de o governador mexer.
///
/// Chrome, Edge e o próprio Windows ligam o EcoQoS em processos por conta
/// própria. Devolver "sem controle" (ControlMask 0) para todo mundo apagaria
/// essa escolha: por isso o estado é LIDO antes, e devolvido como estava.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Eco {
    /// O governador não mexeu no EcoQoS: o Windows não deixou ler o estado
    /// anterior (no Windows 10 a leitura não existe) ou recusou a mudança.
    /// Sem saber o que havia, não se escreve.
    NaoMexido,
    /// O estado lido antes, para devolver exatamente.
    Anterior { control: u32, state: u32 },
    /// Registro gravado por uma versão que não lia o estado anterior. Devolve
    /// como ela devolvia (sem controle), que é o melhor que se sabe.
    Legado,
}

fn eco_legado() -> Eco {
    Eco::Legado
}

/// O que foi mexido num processo, para devolver.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Acalmado {
    pub pid: u32,
    pub nome: String,
    /// Segundos desde 1970 em que o processo começou: confirma que é o mesmo.
    pub inicio: u64,
    pub prioridade_anterior: u32,
    #[serde(default = "eco_legado")]
    pub ecoqos: Eco,
}

/// Um processo que o governador quis acalmar e o Windows não deixou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recusa {
    pub nome: String,
    /// Código do Windows (`GetLastError`). 5 é acesso negado: quase sempre um
    /// programa rodando como administrador com o Otimiza sem administrador.
    pub codigo: u32,
}

/// O que uma passada fez de novo, para a tela.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Passada {
    /// Passaram a rodar em modo econômico agora.
    pub acalmados: Vec<String>,
    /// Estavam disputando e o Windows recusou.
    pub recusados: Vec<Recusa>,
    /// Partida de comparação: estavam disputando e ficaram como estavam.
    pub em_espera: Vec<String>,
}

/// Como o governador age nesta sessão de jogo (decidido pelo portão).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Modo {
    /// Acalma quem disputa.
    Agir,
    /// Partida de comparação: vê quem acalmaria e não mexe.
    Comparar,
    /// Não faz nada: reprovado neste jogo, ou sem medição para conferir. É o
    /// padrão: um governador criado sem passar pelo portão não mexe em nada.
    #[default]
    Parado,
}

/// O resultado de devolver uma lista.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Devolucao {
    pub devolvidos: usize,
    /// Fecharam (ou o número passou a outro processo): não há o que devolver.
    pub sumidos: usize,
    /// O Windows recusou devolver. Ficam anotados em disco para nova tentativa.
    pub falharam: Vec<Acalmado>,
    /// A anotação em disco falhou (ou o arquivo anterior não se lê): quem
    /// falhou pode não ser tentado de novo, e a frase não pode dizer que é.
    pub nao_anotado: Option<String>,
}

fn arquivo() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("pc-optimizer").join("governador.json"))
}

fn gravar(lista: &[Acalmado]) -> Result<(), String> {
    let a = arquivo().ok_or("APPDATA ausente")?;
    let resultado = if lista.is_empty() {
        match std::fs::remove_file(&a) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    } else {
        if let Some(p) = a.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        serde_json::to_string(lista).map_err(|e| e.to_string()).and_then(|json| std::fs::write(&a, json).map_err(|e| e.to_string()))
    };
    if let Err(e) = &resultado {
        crate::utils::Logger::warn(&format!("governador: não gravei a lista do que está acalmado: {}", e));
    }
    resultado
}

/// O que ficou anotado em disco. Arquivo ausente é lista vazia; arquivo que
/// existe e não se lê é ERRO — não "nada acalmado".
fn ler() -> Result<Vec<Acalmado>, String> {
    let Some(a) = arquivo() else { return Ok(Vec::new()) };
    match std::fs::read_to_string(&a) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| format!("governador.json ilegível: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("governador.json: {}", e)),
    }
}

// ------------------------------------------------------------------ Windows

#[cfg(windows)]
mod sys {
    use super::Eco;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::System::Threading::{
        GetPriorityClass, GetProcessInformation, OpenProcess, ProcessPowerThrottling, SetPriorityClass,
        SetProcessInformation, BELOW_NORMAL_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SET_INFORMATION,
    };

    struct Alca(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Alca {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    fn erro() -> u32 {
        unsafe { GetLastError() }
    }

    fn abrir(pid: u32) -> Result<Alca, u32> {
        let h = unsafe { OpenProcess(PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if h.is_null() {
            Err(erro())
        } else {
            Ok(Alca(h))
        }
    }

    /// O EcoQoS como está agora. `None` quando o Windows não responde — no
    /// Windows 10 esta leitura não existe.
    fn ler_ecoqos(h: &Alca) -> Option<(u32, u32)> {
        let mut estado = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: 0,
            StateMask: 0,
        };
        let ok = unsafe {
            GetProcessInformation(
                h.0,
                ProcessPowerThrottling,
                &mut estado as *mut _ as *mut core::ffi::c_void,
                std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
            ) != 0
        };
        ok.then_some((estado.ControlMask, estado.StateMask))
    }

    fn escrever_ecoqos(h: &Alca, control: u32, state: u32) -> bool {
        let estado = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: control,
            StateMask: state,
        };
        unsafe {
            SetProcessInformation(
                h.0,
                ProcessPowerThrottling,
                &estado as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
            ) != 0
        }
    }

    /// Acalma. Devolve a prioridade anterior e o que havia no EcoQoS, ou o
    /// código do Windows quando ele negou.
    pub fn acalmar(pid: u32) -> Result<(u32, Eco), u32> {
        let h = abrir(pid)?;
        let anterior = unsafe { GetPriorityClass(h.0) };
        if anterior == 0 {
            return Err(erro());
        }
        // Quem já está abaixo do normal (ou ocioso) continua como está.
        let ja_baixo = anterior == BELOW_NORMAL_PRIORITY_CLASS || anterior == 0x40;
        if !ja_baixo && unsafe { SetPriorityClass(h.0, BELOW_NORMAL_PRIORITY_CLASS) } == 0 {
            return Err(erro());
        }
        let eco = match ler_ecoqos(&h) {
            Some((control, state))
                if escrever_ecoqos(&h, PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_EXECUTION_SPEED) =>
            {
                Eco::Anterior { control, state }
            }
            _ => Eco::NaoMexido,
        };
        Ok((anterior, eco))
    }

    pub fn devolver(pid: u32, prioridade: u32, eco: Eco) -> bool {
        let Ok(h) = abrir(pid) else { return false };
        let a = unsafe { SetPriorityClass(h.0, prioridade) } != 0;
        let b = match eco {
            Eco::NaoMexido => true,
            Eco::Anterior { control, state } => escrever_ecoqos(&h, control, state),
            Eco::Legado => escrever_ecoqos(&h, 0, 0),
        };
        a && b
    }

    #[cfg(test)]
    pub fn ecoqos_de(pid: u32) -> Option<(u32, u32)> {
        ler_ecoqos(&abrir(pid).ok()?)
    }

    #[cfg(test)]
    pub fn ligar_ecoqos_por_conta_propria(pid: u32) -> bool {
        abrir(pid).is_ok_and(|h| escrever_ecoqos(&h, PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_EXECUTION_SPEED))
    }
}

/// Estado da sessão de jogo.
#[derive(Debug, Default)]
pub struct Governador {
    acalmados: Vec<Acalmado>,
    /// Sobras de uma devolução que o Windows recusou: continuam anotadas em
    /// disco e são tentadas de novo ao devolver.
    pendentes: Vec<Acalmado>,
    sistema: Option<sysinfo::System>,
    modo: Modo,
    /// Chave do jogo desta sessão (formato de `portao::Vigiado::processo`).
    processo: Option<String>,
    jogo_pid: u32,
    /// Na partida de comparação: houve quem acalmar em algum momento.
    viu_candidatos: bool,
    /// Já avisados nesta sessão (recusa ou espera), para não repetir.
    avisados: Vec<u32>,
    /// O governador.json existe e não se lê. Enquanto for assim ele não
    /// acalma nada e não grava por cima: a lista de uma sessão anterior pode
    /// estar lá dentro, com programas ainda em prioridade baixa.
    anotacao_ilegivel: Option<String>,
}

impl Governador {
    /// Um governador para a sessão que começa, com as sobras de devoluções
    /// anteriores que falharam.
    ///
    /// Com o governador.json ilegível ele nasce parado (ver
    /// notacao_ilegivel).
    pub fn novo(modo: Modo, processo: Option<String>, jogo_pid: u32) -> Self {
        match ler() {
            Ok(pendentes) => Governador { modo, processo, jogo_pid, pendentes, ..Default::default() },
            Err(e) => {
                crate::utils::Logger::warn(&format!("governador: {}", e));
                Governador { modo: Modo::Parado, processo, jogo_pid, anotacao_ilegivel: Some(e), ..Default::default() }
            }
        }
    }

    /// Por que ele não consegue anotar, quando não consegue.
    pub fn anotacao_ilegivel(&self) -> Option<&str> {
        self.anotacao_ilegivel.as_deref()
    }

    pub fn acalmados(&self) -> &[Acalmado] {
        &self.acalmados
    }

    pub fn modo(&self) -> Modo {
        self.modo
    }

    pub fn jogo_pid(&self) -> u32 {
        self.jogo_pid
    }

    pub fn processo(&self) -> Option<&str> {
        self.processo.as_deref()
    }

    /// O que ele está fazendo AGORA, para marcar a medição.
    pub fn participacao(&self) -> Option<crate::modules::portao::GovernadorNaPartida> {
        use crate::modules::portao::GovernadorNaPartida;
        match self.modo {
            Modo::Agir if !self.acalmados.is_empty() => Some(GovernadorNaPartida::Acalmou),
            Modo::Comparar if self.viu_candidatos => Some(GovernadorNaPartida::Absteve),
            _ => None,
        }
    }

    fn anotar(&self) -> Result<(), String> {
        if let Some(e) = &self.anotacao_ilegivel {
            return Err(e.clone());
        }
        let mut tudo = self.pendentes.clone();
        tudo.extend(self.acalmados.iter().cloned());
        gravar(&tudo)
    }

    /// Uma passada com o jogo aberto: mede quem está usando processador e
    /// acalma os que disputam (ou, na partida de comparação, só anota).
    /// Chamado a cada olhada do vigia (~6 s).
    #[cfg(windows)]
    pub fn passada(&mut self, jogo_pid: u32) -> Passada {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
        if self.modo == Modo::Parado {
            return Passada::default();
        }
        let primeira = self.sistema.is_none();
        let s = self.sistema.get_or_insert_with(System::new);
        let atualizar = |s: &mut System| {
            s.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_exe(sysinfo::UpdateKind::OnlyIfNotSet));
        };
        atualizar(s);
        if primeira {
            // Uso de CPU precisa de duas leituras separadas no tempo.
            std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.max(std::time::Duration::from_millis(500)));
            atualizar(s);
        }
        let nucleos = num_cpus::get().max(1) as f64;
        let candidatos: Vec<Candidato> = s
            .processes()
            .iter()
            .map(|(pid, p)| Candidato {
                pid: pid.as_u32(),
                nome: p.name().to_string_lossy().to_string(),
                caminho: p.exe().map(PathBuf::from),
                uso: p.cpu_usage() as f64 / 100.0 / nucleos,
            })
            .collect();
        let jogo_caminho = s.process(sysinfo::Pid::from_u32(jogo_pid)).and_then(|p| p.exe()).map(PathBuf::from);
        let escolhidos = escolher(&candidatos, jogo_pid, jogo_caminho.as_deref(), std::process::id());
        let mut feito = Passada::default();
        for pid in escolhidos {
            if self.acalmados.iter().any(|a| a.pid == pid) {
                continue;
            }
            let Some(p) = s.process(sysinfo::Pid::from_u32(pid)) else { continue };
            let nome = p.name().to_string_lossy().to_string();
            let novo_aviso = !self.avisados.contains(&pid);
            if self.modo == Modo::Comparar {
                self.viu_candidatos = true;
                if novo_aviso {
                    self.avisados.push(pid);
                    feito.em_espera.push(nome);
                }
                continue;
            }
            match sys::acalmar(pid) {
                Ok((anterior, ecoqos)) => {
                    feito.acalmados.push(nome.clone());
                    self.acalmados.push(Acalmado { pid, nome, inicio: p.start_time(), prioridade_anterior: anterior, ecoqos });
                }
                Err(codigo) => {
                    if novo_aviso {
                        self.avisados.push(pid);
                        feito.recusados.push(Recusa { nome, codigo });
                    }
                }
            }
        }
        if !feito.acalmados.is_empty() {
            // Falha aqui fica no log: o que foi acalmado continua na memória e
            // é devolvido quando o jogo fechar; só a volta depois de o Otimiza
            // morrer no meio fica sem a lista.
            let _ = self.anotar();
            crate::utils::Logger::info(&format!("governador: acalmados {}", feito.acalmados.join(", ")));
        }
        if !feito.recusados.is_empty() {
            let nomes: Vec<String> = feito.recusados.iter().map(|r| format!("{} (erro {})", r.nome, r.codigo)).collect();
            crate::utils::Logger::warn(&format!("governador: o Windows recusou acalmar {}", nomes.join(", ")));
        }
        feito
    }

    /// O jogo fechou, o modo desligou ou o portão reprovou: devolve tudo. O
    /// que o Windows recusar devolver fica anotado em disco para a próxima
    /// tentativa — "não consegui" nunca vira "devolvido".
    #[cfg(windows)]
    pub fn devolver_tudo(&mut self) -> Devolucao {
        let mut lista = std::mem::take(&mut self.pendentes);
        lista.append(&mut self.acalmados);
        let mut d = devolver_lista(&lista);
        self.pendentes = d.falharam.clone();
        self.sistema = None;
        self.viu_candidatos = false;
        d.nao_anotado = self.anotar().err();
        d
    }

    /// O portão reprovou o governador neste jogo: devolve e para.
    #[cfg(windows)]
    pub fn parar(&mut self) -> Devolucao {
        self.modo = Modo::Parado;
        self.devolver_tudo()
    }
}

/// Devolve uma lista, conferindo que cada processo ainda é o mesmo.
#[cfg(windows)]
fn devolver_lista(lista: &[Acalmado]) -> Devolucao {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut s = System::new();
    s.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let mut d = Devolucao::default();
    for a in lista {
        let mesmo = s.process(sysinfo::Pid::from_u32(a.pid)).is_some_and(|p| p.start_time() == a.inicio);
        if !mesmo {
            d.sumidos += 1;
        } else if sys::devolver(a.pid, a.prioridade_anterior, a.ecoqos) {
            d.devolvidos += 1;
        } else {
            d.falharam.push(a.clone());
        }
    }
    if !d.falharam.is_empty() {
        let nomes: Vec<&str> = d.falharam.iter().map(|a| a.nome.as_str()).collect();
        crate::utils::Logger::warn(&format!("governador: não consegui devolver {}", nomes.join(", ")));
    }
    d
}

/// Na abertura: devolve o que uma sessão anterior deixou acalmado (o Otimiza
/// morreu com o jogo aberto, ou o Windows recusou devolver). O que falhar de
/// novo continua anotado.
#[cfg(windows)]
pub fn recuperar_na_abertura() -> Result<Devolucao, String> {
    let lista = ler()?;
    if lista.is_empty() {
        return Ok(Devolucao::default());
    }
    let mut d = devolver_lista(&lista);
    d.nao_anotado = gravar(&d.falharam).err();
    Ok(d)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn c(pid: u32, nome: &str, caminho: &str, uso: f64) -> Candidato {
        Candidato { pid, nome: nome.into(), caminho: Some(PathBuf::from(caminho)), uso }
    }

    #[test]
    fn acalma_so_quem_disputa_processador() {
        let jogo = Path::new(r"C:\Users\U\AppData\Local\FiveM\FiveM.app\data\cache\subprocess\FiveM_b3258_GTAProcess.exe");
        let lista = vec![
            c(100, "FiveM_b3258_GTAProcess.exe", jogo.to_str().unwrap(), 0.40),
            c(101, "FiveM_ChromeBrowser", r"C:\Users\U\AppData\Local\FiveM\FiveM.app\data\cache\subprocess\FiveM_ChromeBrowser", 0.05),
            c(200, "chrome.exe", r"C:\Program Files\Google\Chrome\chrome.exe", 0.08),
            c(201, "OneDrive.exe", r"C:\Users\U\AppData\Local\Microsoft\OneDrive\OneDrive.exe", 0.05),
            c(202, "Discord.exe", r"C:\Users\U\AppData\Local\Discord\Discord.exe", 0.06),
            c(203, "svchost.exe", r"C:\Windows\System32\svchost.exe", 0.10),
            c(204, "notepad.exe", r"C:\Windows\notepad.exe", 0.001),
            c(205, "vgc.exe", r"C:\Program Files\Riot Vanguard\vgc.exe", 0.05),
            c(999, "pc-optimizer.exe", r"C:\Otimiza\pc-optimizer.exe", 0.2),
        ];
        let mut e = escolher(&lista, 100, Some(jogo), 999);
        e.sort();
        assert_eq!(e, vec![200, 201]);
    }

    #[test]
    #[ignore = "mexe na prioridade de processos reais desta máquina e devolve"]
    fn acalma_e_devolve_um_processo_real() {
        use windows_sys::Win32::System::Threading::{GetPriorityClass, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        let prioridade = |pid: u32| unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            let p = GetPriorityClass(h);
            windows_sys::Win32::Foundation::CloseHandle(h);
            p
        };
        let mut filho = std::process::Command::new("cmd")
            .args(["/C", "for /L %i in () do @rem"])
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));
        let pid = filho.id();
        assert_eq!(prioridade(pid), 0x20, "começa em normal");
        let mut g = Governador::novo(Modo::Agir, None, 0);
        let novos = g.passada(0);
        println!("acalmados: {:?}", novos);
        assert!(g.acalmados().iter().any(|a| a.pid == pid), "o processo que queima CPU tinha que ser acalmado");
        assert_eq!(prioridade(pid), 0x4000, "abaixo do normal");
        let devolucao = g.devolver_tudo();
        assert!(devolucao.devolvidos >= 1 && devolucao.falharam.is_empty(), "{:?}", devolucao);
        assert_eq!(prioridade(pid), 0x20, "voltou ao normal");
        let _ = filho.kill();
    }

    /// Só mexe num processo filho criado pelo próprio teste.
    #[test]
    #[ignore = "abre um processo filho e mexe na prioridade e no EcoQoS dele"]
    fn ecoqos_volta_ao_que_o_processo_tinha() {
        let mut filho = std::process::Command::new("cmd").args(["/C", "for /L %i in () do @rem"]).spawn().unwrap();
        let pid = filho.id();
        // O processo liga o EcoQoS por conta própria, como o Chrome faz.
        let ligou = sys::ligar_ecoqos_por_conta_propria(pid);
        let antes = sys::ecoqos_de(pid);
        println!("ligou sozinho: {}, estado antes: {:?}", ligou, antes);
        let (prioridade, eco) = sys::acalmar(pid).expect("acalmar o próprio filho");
        match antes {
            Some((control, state)) => assert_eq!(eco, Eco::Anterior { control, state }),
            // Windows sem a leitura (Windows 10): não se escreve o que não se leu.
            None => assert_eq!(eco, Eco::NaoMexido),
        }
        assert!(sys::devolver(pid, prioridade, eco));
        assert_eq!(sys::ecoqos_de(pid), antes, "o EcoQoS tem que voltar ao que o processo tinha, não a zero");
        let _ = filho.kill();
    }

    #[test]
    fn registro_antigo_sem_ecoqos_devolve_como_antes() {
        // governador.json gravado antes de o estado anterior ser lido.
        let antigo = r#"[{"pid":10,"nome":"a.exe","inicio":5,"prioridade_anterior":32}]"#;
        let lista: Vec<Acalmado> = serde_json::from_str(antigo).unwrap();
        assert_eq!(lista[0].ecoqos, Eco::Legado);
    }

    #[test]
    fn sem_passar_pelo_portao_nao_mexe() {
        let g = Governador::default();
        assert_eq!(g.modo(), Modo::Parado);
        assert_eq!(g.participacao(), None);
    }

    #[test]
    fn participacao_marca_os_dois_lados() {
        use crate::modules::portao::GovernadorNaPartida;
        let mut g = Governador { modo: Modo::Agir, ..Default::default() };
        assert_eq!(g.participacao(), None, "agindo sem nada acalmado não é lado nenhum");
        g.acalmados.push(Acalmado { pid: 9, nome: "x".into(), inicio: 1, prioridade_anterior: 32, ecoqos: Eco::NaoMexido });
        assert_eq!(g.participacao(), Some(GovernadorNaPartida::Acalmou));
        let mut c = Governador { modo: Modo::Comparar, ..Default::default() };
        assert_eq!(c.participacao(), None, "comparação sem ninguém disputando não é lado nenhum");
        c.viu_candidatos = true;
        assert_eq!(c.participacao(), Some(GovernadorNaPartida::Absteve));
    }

    #[test]
    fn protegidos_por_nome() {
        for n in ["Discord.exe", "obs64.exe", "audiodg.exe", "EasyAntiCheat.exe", "BEService.exe", "explorer.exe", "svchost.exe"] {
            assert!(protegido(n), "{}", n);
        }
        for n in ["chrome.exe", "OneDrive.exe", "EpicGamesLauncher.exe", "steamwebhelper.exe"] {
            assert!(!protegido(n), "{}", n);
        }
    }

    #[test]
    fn o_governador_nao_congela_nem_fecha() {
        let fonte = include_str!("governador.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();
        // Montados em pedaços para a trava de `suspend.rs`, que varre os
        // fontes procurando esses nomes, não acusar este próprio teste.
        let proibidos = [
            ["Suspend", "Thread"].concat(),
            ["NtSuspend", "Process"].concat(),
            ["Terminate", "Process"].concat(),
            ["task", "kill"].concat(),
            ["Stop-", "Process"].concat(),
            ["HIGH_", "PRIORITY"].concat(),
            ["REAL", "TIME"].concat(),
        ];
        for proibido in &proibidos {
            assert!(!producao.contains(proibido), "`{}` apareceu no governador", proibido);
        }
    }
}
