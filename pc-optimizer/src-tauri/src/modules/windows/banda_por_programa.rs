// QUEM ESTÁ USANDO A INTERNET AGORA, POR PROGRAMA (Plano de refino, R3). "O OneDrive está enviando 8 Mb/s" no lugar de
// "sua internet está ruim". Pelo rastreamento de eventos do Windows (o provedor Microsoft-Windows-Kernel-Network, o
// mesmo do Monitor de Recursos): cada envio e recebimento TCP e UDP traz o processo e o tamanho. Cinco segundos numa
// sessão própria, que para sozinha; só lê. Exige administrador (sessão de rastreamento do núcleo).

#![cfg(target_os = "windows")]

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, ERROR_SUCCESS};
use windows_sys::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW, CONTROLTRACE_HANDLE,
    EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW,
    EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE, PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_REAL_TIME,
    TRACE_LEVEL_INFORMATION, WNODE_FLAG_TRACED_GUID,
};

/// Microsoft-Windows-Kernel-Network, GUID publicado.
const PROVEDOR_REDE: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x7DD42A49,
    data2: 0x5329,
    data3: 0x4832,
    data4: [0x8D, 0xFD, 0x43, 0xD9, 0x79, 0x15, 0x3A, 0x88],
};
/// IPv4 (0x10) e IPv6 (0x20).
const PALAVRAS_IP: u64 = 0x10 | 0x20;
const NOME_SESSAO: &str = "Otimiza-Rede";

/// **Pura.** Os eventos de envio e recebimento (TCP e UDP, IPv4 e IPv6) do manifesto do provedor. `Some(true)` envio.
pub fn direcao(id: u16) -> Option<bool> {
    match id {
        10 | 26 | 42 | 58 => Some(true),
        11 | 27 | 43 | 59 => Some(false),
        _ => None,
    }
}

static MEDINDO: AtomicBool = AtomicBool::new(false);
/// pid → (bytes enviados, bytes recebidos).
static POR_PID: Mutex<Option<HashMap<u32, (u64, u64)>>> = Mutex::new(None);

unsafe extern "system" fn ao_receber_evento(registro: *mut EVENT_RECORD) {
    if registro.is_null() {
        return;
    }
    let r = &*registro;
    let Some(envio) = direcao(r.EventHeader.EventDescriptor.Id) else {
        return;
    };
    // Os dois primeiros campos de todos esses eventos: PID e tamanho (u32 cada). O `ProcessId` do cabeçalho costuma
    // ser o do System, não o do programa.
    if r.UserDataLength < 8 || r.UserData.is_null() {
        return;
    }
    let dados = r.UserData as *const u8;
    let pid = u32::from_le_bytes(std::ptr::read_unaligned(dados as *const [u8; 4]));
    let tamanho = u32::from_le_bytes(std::ptr::read_unaligned(dados.add(4) as *const [u8; 4])) as u64;
    if let Ok(mut g) = POR_PID.try_lock() {
        if let Some(mapa) = g.as_mut() {
            let e = mapa.entry(pid).or_insert((0, 0));
            if envio {
                e.0 += tamanho;
            } else {
                e.1 += tamanho;
            }
        }
    }
}

fn para_utf16(texto: &str) -> Vec<u16> {
    texto.encode_utf16().chain(std::iter::once(0)).collect()
}

fn propriedades(nome: &[u16]) -> Vec<u8> {
    let tamanho_struct = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
    let total = tamanho_struct + nome.len() * 2;
    let mut buffer = vec![0u8; total];
    unsafe {
        let props = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
        (*props).Wnode.BufferSize = total as u32;
        (*props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        (*props).Wnode.ClientContext = 1;
        (*props).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        (*props).LoggerNameOffset = tamanho_struct as u32;
        std::ptr::copy_nonoverlapping(nome.as_ptr(), buffer.as_mut_ptr().add(tamanho_struct) as *mut u16, nome.len());
    }
    buffer
}

fn parar() {
    let nome = para_utf16(NOME_SESSAO);
    let mut props = propriedades(&nome);
    unsafe {
        ControlTraceW(
            CONTROLTRACE_HANDLE { Value: 0 },
            nome.as_ptr(),
            props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES,
            EVENT_TRACE_CONTROL_STOP,
        );
    }
}

/// Bytes por pid durante `segundos`. Bloqueia.
pub fn contar(segundos: u64) -> Result<HashMap<u32, (u64, u64)>, String> {
    if MEDINDO.swap(true, Ordering::SeqCst) {
        return Err("Já estou medindo a rede: espere terminar.".to_string());
    }
    struct Solta;
    impl Drop for Solta {
        fn drop(&mut self) {
            MEDINDO.store(false, Ordering::SeqCst);
        }
    }
    let _solta = Solta;

    parar();
    let nome = para_utf16(NOME_SESSAO);
    let mut props = propriedades(&nome);
    let mut sessao = CONTROLTRACE_HANDLE { Value: 0 };
    let inicio = unsafe { StartTraceW(&mut sessao, nome.as_ptr(), props.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES) };
    if inicio != ERROR_SUCCESS {
        return Err(if inicio == ERROR_ALREADY_EXISTS {
            "Uma medição de rede anterior ficou presa: reinicie o PC e tente de novo.".to_string()
        } else if inicio == 5 {
            "Para ver quem usa a internet, abra o Otimiza como administrador.".to_string()
        } else {
            format!("O Windows não deixou medir a rede (código {}).", inicio)
        });
    }
    let habilitou = unsafe {
        EnableTraceEx2(
            sessao,
            &PROVEDOR_REDE,
            EVENT_CONTROL_CODE_ENABLE_PROVIDER,
            TRACE_LEVEL_INFORMATION as u8,
            PALAVRAS_IP,
            0,
            0,
            std::ptr::null(),
        )
    };
    if habilitou != ERROR_SUCCESS {
        parar();
        return Err(format!("O Windows não deixou escutar a rede (código {}).", habilitou));
    }
    if let Ok(mut g) = POR_PID.lock() {
        *g = Some(HashMap::new());
    }
    let coletor = std::thread::spawn(move || {
        let mut arquivo: EVENT_TRACE_LOGFILEW = unsafe { std::mem::zeroed() };
        let nome = para_utf16(NOME_SESSAO);
        arquivo.LoggerName = nome.as_ptr() as *mut u16;
        arquivo.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
        arquivo.Anonymous2.EventRecordCallback = Some(ao_receber_evento);
        let consumidor = unsafe { OpenTraceW(&mut arquivo) };
        if consumidor.Value == u64::MAX {
            return Err(unsafe { GetLastError() });
        }
        unsafe {
            ProcessTrace(&consumidor, 1, std::ptr::null(), std::ptr::null());
            CloseTrace(consumidor);
        }
        Ok(())
    });
    std::thread::sleep(std::time::Duration::from_secs(segundos));
    parar();
    let saida = coletor.join();
    let mapa = POR_PID.lock().ok().and_then(|mut g| g.take()).unwrap_or_default();
    if let Ok(Err(codigo)) = saida {
        return Err(format!("Não consegui ler os eventos de rede (código {}).", codigo));
    }
    Ok(mapa)
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct ProgramaNaRede {
    pub programa: String,
    pub envio_mbps: f64,
    pub recebe_mbps: f64,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct QuemUsaARede {
    pub segundos: u64,
    pub programas: Vec<ProgramaNaRede>,
    pub frase: String,
}

/// Abaixo disto (Mb/s, envio + recebimento) o programa não aparece: é o fundo normal de qualquer PC.
pub const MINIMO_MBPS: f64 = 0.05;

/// **Pura.** Por programa (pids do mesmo nome somados), em Mb/s, do que mais usa para o que menos usa.
pub fn por_programa(bytes: &HashMap<u32, (u64, u64)>, nomes: &HashMap<u32, String>, segundos: f64) -> Vec<ProgramaNaRede> {
    let mut soma: HashMap<String, (u64, u64)> = HashMap::new();
    for (pid, (env, rec)) in bytes {
        let nome = nomes.get(pid).cloned().unwrap_or_else(|| if *pid == 4 { "Windows (sistema)".to_string() } else { format!("processo {}", pid) });
        let e = soma.entry(nome).or_insert((0, 0));
        e.0 += env;
        e.1 += rec;
    }
    let mbps = |b: u64| b as f64 * 8.0 / segundos.max(0.1) / 1_000_000.0;
    let mut v: Vec<ProgramaNaRede> = soma
        .into_iter()
        .map(|(programa, (e, r))| ProgramaNaRede { programa, envio_mbps: mbps(e), recebe_mbps: mbps(r) })
        .filter(|p| p.envio_mbps + p.recebe_mbps >= MINIMO_MBPS)
        .collect();
    v.sort_by(|a, b| {
        (b.envio_mbps + b.recebe_mbps).partial_cmp(&(a.envio_mbps + a.recebe_mbps)).unwrap_or(std::cmp::Ordering::Equal)
    });
    v.truncate(8);
    v
}

/// **Pura.** A frase: quem mais pesa, e se é envio (o que mais sobe o ping).
pub fn frase(programas: &[ProgramaNaRede]) -> String {
    let n = |v: f64| format!("{:.1}", v).replace('.', ",");
    match programas.first() {
        None => "Nada está usando a internet agora: se o ping está ruim, o problema não é um programa deste PC.".to_string(),
        Some(p) if p.envio_mbps >= 1.0 && p.envio_mbps >= p.recebe_mbps => format!(
            "O {} está enviando {} Mb/s: envio enche a fila do modem e é o que mais sobe o ping. Pause ele durante o jogo.",
            p.programa,
            n(p.envio_mbps)
        ),
        Some(p) if p.recebe_mbps >= 5.0 => format!(
            "O {} está baixando {} Mb/s: com a linha cheia, o ping pode subir.",
            p.programa,
            n(p.recebe_mbps)
        ),
        Some(p) => format!(
            "Quem mais usa é o {}, com {} Mb/s: pouco para pesar no jogo.",
            p.programa,
            n(p.envio_mbps + p.recebe_mbps)
        ),
    }
}

/// Mede `segundos` e dá nome aos processos.
pub fn medir(segundos: u64) -> Result<QuemUsaARede, String> {
    let bytes = contar(segundos)?;
    let mut sistema = sysinfo::System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let nomes: HashMap<u32, String> = bytes
        .keys()
        .filter_map(|pid| {
            sistema
                .process(sysinfo::Pid::from_u32(*pid))
                .map(|p| (*pid, p.name().to_string_lossy().to_string()))
        })
        .collect();
    let programas = por_programa(&bytes, &nomes, segundos as f64);
    let frase = frase(&programas);
    Ok(QuemUsaARede { segundos, programas, frase })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_os_eventos_de_envio_e_recebimento() {
        assert_eq!(direcao(10), Some(true));
        assert_eq!(direcao(43), Some(false));
        assert_eq!(direcao(12), None, "conexão, não dado");
    }

    #[test]
    fn soma_por_nome_e_ordena_pelo_total() {
        let bytes = HashMap::from([(100, (5_000_000, 0)), (101, (1_250_000, 0)), (200, (0, 2_500_000)), (300, (10, 10))]);
        let nomes = HashMap::from([(100, "OneDrive.exe".to_string()), (101, "OneDrive.exe".to_string()), (200, "steam.exe".to_string())]);
        let v = por_programa(&bytes, &nomes, 5.0);
        assert_eq!(v[0].programa, "OneDrive.exe");
        assert!((v[0].envio_mbps - 10.0).abs() < 0.01, "{v:?}");
        assert_eq!(v[1].programa, "steam.exe");
        assert_eq!(v.len(), 2, "o de 20 bytes fica de fora");
    }

    #[test]
    fn a_frase_aponta_o_envio() {
        let v = vec![ProgramaNaRede { programa: "OneDrive.exe".into(), envio_mbps: 8.2, recebe_mbps: 0.1 }];
        assert!(frase(&v).contains("enviando 8,2 Mb/s"));
        assert!(frase(&[]).contains("Nada está usando"));
    }

    /// Mede de verdade (administrador): `cargo test -- --ignored quem_usa_de_verdade --nocapture`.
    #[test]
    #[ignore]
    fn quem_usa_de_verdade() {
        let r = medir(5).expect("precisa de administrador");
        println!("{:#?}", r);
    }
}
