// Governador de segundo plano do modo jogo. Programa em segundo plano que está USANDO processador (medido, não
// por lista) recebe prioridade abaixo do normal e EcoQoS: continua rodando, só para de brigar com o jogo. Nada é
// congelado nem fechado, e tudo volta quando o jogo fecha. Nunca toca: o jogo e a pasta dele, voz, transmissão,
// áudio, anticheats, sistema e o Otimiza. A lista do que foi mexido fica em disco (conferida pelo horário de
// início do processo). Não age sem prova: passa pelo portão "nunca menos FPS" (`modules::portao`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 3% de uma máquina de 8 núcleos é um quarto de núcleo ocupado sem parar.
const USO_MINIMO_DA_MAQUINA: f64 = 0.03;

const PROTEGIDOS: &[&str] = &[
    "discord", "teamspeak", "ts3client", "mumble", "obs64", "obs32", "obs", "streamlabs", "xsplit",
    "nvidia broadcast", "voicemeeter", "spotify", "audiodg",
    "vgc", "vgtray", "easyanticheat", "beservice", "battleye", "faceit", "eac_launcher", "ricochet",
    "system", "registry", "smss", "csrss", "wininit", "winlogon", "services", "lsass", "svchost", "dwm",
    "explorer", "fontdrvhost", "sihost", "ctfmon", "searchhost", "startmenuexperiencehost",
    "textinputhost", "runtimebroker", "msmpeng", "securityhealthservice", "taskmgr",
    "pc-optimizer", "otimiza",
];

#[derive(Debug, Clone)]
pub struct Candidato {
    pub pid: u32,
    pub nome: String,
    pub caminho: Option<PathBuf>,
    pub uso: f64,
}

pub fn protegido(nome: &str) -> bool {
    let n = nome.to_lowercase();
    let n = n.trim_end_matches(".exe");
    PROTEGIDOS.iter().any(|p| n == *p || n.starts_with(&format!("{}_", p)) || n.starts_with(&format!("{} ", p)) || (p.len() >= 5 && n.starts_with(p)))
}

/// O FiveM roda em vários processos: fica de fora tudo da pasta e do prefixo de nome do jogo.
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

/// Chrome, Edge e o Windows ligam o EcoQoS sozinhos: o estado é LIDO antes e devolvido como estava.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Eco {
    /// Sem saber o que havia (no Windows 10 a leitura não existe), não se escreve.
    NaoMexido,
    Anterior { control: u32, state: u32 },
    /// Gravado por versão que não lia o estado: devolve sem controle, como ela devolvia.
    Legado,
}

fn eco_legado() -> Eco {
    Eco::Legado
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Acalmado {
    pub pid: u32,
    pub nome: String,
    /// Confirma que é o mesmo processo, e não outro que herdou o número.
    pub inicio: u64,
    pub prioridade_anterior: u32,
    #[serde(default = "eco_legado")]
    pub ecoqos: Eco,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recusa {
    pub nome: String,
    /// 5 é acesso negado: quase sempre um programa como administrador e o Otimiza sem.
    pub codigo: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Passada {
    pub acalmados: Vec<String>,
    pub recusados: Vec<Recusa>,
    pub em_espera: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Modo {
    Agir,
    Comparar,
    /// O padrão: governador que não passou pelo portão não mexe em nada.
    #[default]
    Parado,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Devolucao {
    pub devolvidos: usize,
    pub sumidos: usize,
    pub falharam: Vec<Acalmado>,
    /// Quem falhou pode não ser tentado de novo, e a frase não pode dizer que é.
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

/// Existe e não se lê é ERRO, não "nada acalmado".
fn ler() -> Result<Vec<Acalmado>, String> {
    let Some(a) = arquivo() else { return Ok(Vec::new()) };
    match std::fs::read_to_string(&a) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| format!("governador.json ilegível: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("governador.json: {}", e)),
    }
}

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

    /// `None` quando o Windows não responde (no Windows 10 esta leitura não existe).
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

    pub fn acalmar(pid: u32) -> Result<(u32, Eco), u32> {
        let h = abrir(pid)?;
        let anterior = unsafe { GetPriorityClass(h.0) };
        if anterior == 0 {
            return Err(erro());
        }
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

#[derive(Debug, Default)]
pub struct Governador {
    acalmados: Vec<Acalmado>,
    pendentes: Vec<Acalmado>,
    sistema: Option<sysinfo::System>,
    modo: Modo,
    processo: Option<String>,
    jogo_pid: u32,
    viu_candidatos: bool,
    avisados: Vec<u32>,
    /// Enquanto não se lê, não acalma nem grava por cima: pode haver programas de uma sessão anterior ainda em
    /// prioridade baixa na lista.
    anotacao_ilegivel: Option<String>,
}

impl Governador {
    pub fn novo(modo: Modo, processo: Option<String>, jogo_pid: u32) -> Self {
        match ler() {
            Ok(pendentes) => Governador { modo, processo, jogo_pid, pendentes, ..Default::default() },
            Err(e) => {
                crate::utils::Logger::warn(&format!("governador: {}", e));
                Governador { modo: Modo::Parado, processo, jogo_pid, anotacao_ilegivel: Some(e), ..Default::default() }
            }
        }
    }

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
            // Só a volta depois de o Otimiza morrer no meio fica sem a lista; o resto é devolvido ao fechar o jogo.
            let _ = self.anotar();
            crate::utils::Logger::info(&format!("governador: acalmados {}", feito.acalmados.join(", ")));
        }
        if !feito.recusados.is_empty() {
            let nomes: Vec<String> = feito.recusados.iter().map(|r| format!("{} (erro {})", r.nome, r.codigo)).collect();
            crate::utils::Logger::warn(&format!("governador: o Windows recusou acalmar {}", nomes.join(", ")));
        }
        feito
    }

    /// O que o Windows recusar fica anotado para nova tentativa: "não consegui" nunca vira "devolvido".
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

    #[cfg(windows)]
    pub fn parar(&mut self) -> Devolucao {
        self.modo = Modo::Parado;
        self.devolver_tudo()
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Zerado {
    NadaAnotado,
    Devolvidos { devolvidos: usize, falharam: Vec<String> },
    Ilegivel { guardado_em: Option<PathBuf> },
}

#[cfg(windows)]
pub fn zerar_anotacao() -> Result<Zerado, String> {
    let a = arquivo().ok_or("APPDATA ausente")?;
    zerar_em(&a, devolver_lista, crate::modules::changelog::now_timestamp())
}

/// Zerar é esquecer a lista: quem falha ao devolver volta ao normal quando o processo reabrir.
fn zerar_em(a: &Path, devolver: impl Fn(&[Acalmado]) -> Devolucao, agora: u64) -> Result<Zerado, String> {
    let texto = match std::fs::read_to_string(a) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Zerado::NadaAnotado),
        Err(e) => return Err(format!("governador.json: {}", e)),
    };
    match serde_json::from_str::<Vec<Acalmado>>(&texto) {
        Ok(lista) => {
            let d = devolver(&lista);
            std::fs::remove_file(a).map_err(|e| format!("não consegui apagar governador.json: {}", e))?;
            if lista.is_empty() {
                return Ok(Zerado::NadaAnotado);
            }
            Ok(Zerado::Devolvidos { devolvidos: d.devolvidos, falharam: d.falharam.into_iter().map(|x| x.nome).collect() })
        }
        Err(_) => {
            let guardado = a.with_file_name(format!("governador.ilegivel-{}.json", agora));
            if std::fs::rename(a, &guardado).is_ok() {
                return Ok(Zerado::Ilegivel { guardado_em: Some(guardado) });
            }
            std::fs::remove_file(a).map_err(|e| format!("não consegui tirar governador.json do caminho: {}", e))?;
            Ok(Zerado::Ilegivel { guardado_em: None })
        }
    }
}

pub fn frase_do_zerar(z: &Zerado) -> String {
    match z {
        Zerado::NadaAnotado => "Não havia nada anotado: o modo jogo já estava livre.".to_string(),
        Zerado::Devolvidos { devolvidos, falharam } if falharam.is_empty() => {
            format!("Modo jogo zerado. {} programa(s) voltaram ao normal.", devolvidos)
        }
        Zerado::Devolvidos { devolvidos, falharam } => format!(
            "Modo jogo zerado. {} programa(s) voltaram ao normal; {} o Windows não deixou devolver agora e volta(m) ao normal quando for(em) reaberto(s).",
            devolvidos,
            falharam.join(", ")
        ),
        Zerado::Ilegivel { .. } => "Modo jogo zerado. A anotação estava estragada e foi guardada à parte. Algum programa que tenha ficado em modo econômico volta ao normal quando for reaberto, ou quando o PC reiniciar.".to_string(),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pasta_de_teste(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("otimiza_zerar_{}_{}", nome, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn acalmado(nome: &str) -> Acalmado {
        Acalmado { pid: 1, nome: nome.into(), inicio: 0, prioridade_anterior: 32, ecoqos: Eco::NaoMexido }
    }

    #[test]
    fn zerar_sem_arquivo_nao_faz_nada() {
        let p = pasta_de_teste("vazio");
        let z = zerar_em(&p.join("governador.json"), |_| panic!("não devolve nada"), 1).unwrap();
        assert_eq!(z, Zerado::NadaAnotado);
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn zerar_arquivo_estragado_guarda_a_parte_e_libera() {
        let p = pasta_de_teste("estragado");
        let a = p.join("governador.json");
        std::fs::write(&a, "{isto não é json").unwrap();
        let z = zerar_em(&a, |_| panic!("não devolve nada"), 42).unwrap();
        let guardado = p.join("governador.ilegivel-42.json");
        assert_eq!(z, Zerado::Ilegivel { guardado_em: Some(guardado.clone()) });
        assert!(!a.exists(), "o caminho precisa ficar livre, senão o governador segue parado");
        assert_eq!(std::fs::read_to_string(&guardado).unwrap(), "{isto não é json");
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn zerar_lista_legivel_devolve_e_apaga_mesmo_com_falha() {
        let p = pasta_de_teste("legivel");
        let a = p.join("governador.json");
        std::fs::write(&a, serde_json::to_string(&vec![acalmado("chrome.exe"), acalmado("OneDrive.exe")]).unwrap()).unwrap();
        let z = zerar_em(
            &a,
            |lista| Devolucao { devolvidos: 1, falharam: vec![lista[1].clone()], ..Default::default() },
            1,
        )
        .unwrap();
        assert_eq!(z, Zerado::Devolvidos { devolvidos: 1, falharam: vec!["OneDrive.exe".into()] });
        assert!(!a.exists());
        assert!(frase_do_zerar(&z).contains("OneDrive.exe"));
        let _ = std::fs::remove_dir_all(&p);
    }

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
        let ligou = sys::ligar_ecoqos_por_conta_propria(pid);
        let antes = sys::ecoqos_de(pid);
        println!("ligou sozinho: {}, estado antes: {:?}", ligou, antes);
        let (prioridade, eco) = sys::acalmar(pid).expect("acalmar o próprio filho");
        match antes {
            Some((control, state)) => assert_eq!(eco, Eco::Anterior { control, state }),
            None => assert_eq!(eco, Eco::NaoMexido),
        }
        assert!(sys::devolver(pid, prioridade, eco));
        assert_eq!(sys::ecoqos_de(pid), antes, "o EcoQoS tem que voltar ao que o processo tinha, não a zero");
        let _ = filho.kill();
    }

    #[test]
    fn registro_antigo_sem_ecoqos_devolve_como_antes() {
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
        // Em pedaços para a trava de `suspend.rs`, que varre os fontes por esses nomes, não acusar este teste.
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
