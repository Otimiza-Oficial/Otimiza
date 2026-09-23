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

/// O que foi mexido num processo, para devolver.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Acalmado {
    pub pid: u32,
    pub nome: String,
    /// Segundos desde 1970 em que o processo começou: confirma que é o mesmo.
    pub inicio: u64,
    pub prioridade_anterior: u32,
}

fn arquivo() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("pc-optimizer").join("governador.json"))
}

fn gravar(lista: &[Acalmado]) {
    let Some(a) = arquivo() else { return };
    if lista.is_empty() {
        let _ = std::fs::remove_file(&a);
        return;
    }
    if let Some(p) = a.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    if let Ok(json) = serde_json::to_string(lista) {
        let _ = std::fs::write(&a, json);
    }
}

fn ler() -> Vec<Acalmado> {
    arquivo()
        .and_then(|a| std::fs::read_to_string(a).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

// ------------------------------------------------------------------ Windows

#[cfg(windows)]
mod sys {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetPriorityClass, OpenProcess, ProcessPowerThrottling, SetPriorityClass, SetProcessInformation,
        BELOW_NORMAL_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SET_INFORMATION,
    };

    struct Alca(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Alca {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    fn abrir(pid: u32) -> Option<Alca> {
        let h = unsafe { OpenProcess(PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        (!h.is_null()).then_some(Alca(h))
    }

    fn ecoqos(h: &Alca, ligar: bool) -> bool {
        // Ligar: controla a velocidade e pede eficiência. Devolver: tira o
        // controle (ControlMask 0), e o Windows volta a decidir sozinho.
        let estado = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: if ligar { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
            StateMask: if ligar { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
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

    /// Acalma. Devolve a prioridade anterior, ou `None` se o Windows negou.
    pub fn acalmar(pid: u32) -> Option<u32> {
        let h = abrir(pid)?;
        let anterior = unsafe { GetPriorityClass(h.0) };
        if anterior == 0 {
            return None;
        }
        // Quem já está abaixo do normal (ou ocioso) continua como está.
        if anterior == BELOW_NORMAL_PRIORITY_CLASS || anterior == 0x40 {
            ecoqos(&h, true);
            return Some(anterior);
        }
        if unsafe { SetPriorityClass(h.0, BELOW_NORMAL_PRIORITY_CLASS) } == 0 {
            return None;
        }
        ecoqos(&h, true);
        Some(anterior)
    }

    pub fn devolver(pid: u32, prioridade: u32) -> bool {
        let Some(h) = abrir(pid) else { return false };
        let a = unsafe { SetPriorityClass(h.0, prioridade) } != 0;
        let b = ecoqos(&h, false);
        a && b
    }
}

/// Estado da sessão de jogo.
#[derive(Debug, Default)]
pub struct Governador {
    acalmados: Vec<Acalmado>,
    sistema: Option<sysinfo::System>,
}

impl Governador {
    pub fn acalmados(&self) -> &[Acalmado] {
        &self.acalmados
    }

    /// Uma passada com o jogo aberto: mede quem está usando processador e
    /// acalma os que disputam. Chamado a cada olhada do vigia (~6 s).
    #[cfg(windows)]
    pub fn passada(&mut self, jogo_pid: u32) -> Vec<String> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
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
        let mut novos = Vec::new();
        for pid in escolhidos {
            if self.acalmados.iter().any(|a| a.pid == pid) {
                continue;
            }
            let Some(p) = s.process(sysinfo::Pid::from_u32(pid)) else { continue };
            if let Some(anterior) = sys::acalmar(pid) {
                let nome = p.name().to_string_lossy().to_string();
                novos.push(nome.clone());
                self.acalmados.push(Acalmado { pid, nome, inicio: p.start_time(), prioridade_anterior: anterior });
            }
        }
        if !novos.is_empty() {
            gravar(&self.acalmados);
            crate::utils::Logger::info(&format!("governador: acalmados {}", novos.join(", ")));
        }
        novos
    }

    /// O jogo fechou: devolve tudo.
    #[cfg(windows)]
    pub fn devolver_tudo(&mut self) -> usize {
        let n = devolver_lista(&self.acalmados);
        self.acalmados.clear();
        self.sistema = None;
        gravar(&[]);
        n
    }
}

/// Devolve uma lista, conferindo que cada processo ainda é o mesmo.
#[cfg(windows)]
fn devolver_lista(lista: &[Acalmado]) -> usize {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut s = System::new();
    s.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    lista
        .iter()
        .filter(|a| {
            s.process(sysinfo::Pid::from_u32(a.pid)).is_some_and(|p| p.start_time() == a.inicio)
                && sys::devolver(a.pid, a.prioridade_anterior)
        })
        .count()
}

/// Na abertura: devolve o que uma sessão anterior deixou acalmado (o Otimiza
/// morreu com o jogo aberto).
#[cfg(windows)]
pub fn recuperar_na_abertura() -> usize {
    let lista = ler();
    if lista.is_empty() {
        return 0;
    }
    let n = devolver_lista(&lista);
    gravar(&[]);
    n
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
        let mut g = Governador::default();
        let novos = g.passada(0);
        println!("acalmados: {:?}", novos);
        assert!(g.acalmados().iter().any(|a| a.pid == pid), "o processo que queima CPU tinha que ser acalmado");
        assert_eq!(prioridade(pid), 0x4000, "abaixo do normal");
        let devolvidos = g.devolver_tudo();
        assert!(devolvidos >= 1);
        assert_eq!(prioridade(pid), 0x20, "voltou ao normal");
        let _ = filho.kill();
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
