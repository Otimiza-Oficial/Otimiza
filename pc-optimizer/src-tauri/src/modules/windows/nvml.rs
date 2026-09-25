// Sensores da placa NVIDIA EM PROCESSO (`nvml.dll`), para amostrar durante a partida: abrir o `nvidia-smi` a
// cada 200 ms viraria a carga que se está medindo. Só leituras.

#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::sync::OnceLock;

use crate::core::sensores::AmostraGpu;

const OK: u32 = 0;
const SENSOR_GPU: u32 = 0;
const CLOCK_GRAFICO: u32 = 0;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Utilizacao {
    gpu: u32,
    memoria: u32,
}

struct Nvml {
    dispositivo: *mut c_void,
    temperatura: unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> u32,
    clock: unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> u32,
    potencia: unsafe extern "C" fn(*mut c_void, *mut u32) -> u32,
    limite_de_potencia: unsafe extern "C" fn(*mut c_void, *mut u32) -> u32,
    utilizacao: unsafe extern "C" fn(*mut c_void, *mut Utilizacao) -> u32,
    motivos: Option<unsafe extern "C" fn(*mut c_void, *mut u64) -> u32>,
}

// Uma thread por vez; o ponteiro do dispositivo é estável enquanto a biblioteca está carregada.
unsafe impl Send for Nvml {}
unsafe impl Sync for Nvml {}

fn nvml() -> Option<&'static Nvml> {
    static NVML: OnceLock<Option<Nvml>> = OnceLock::new();
    NVML.get_or_init(carregar).as_ref()
}

/// Não é descarregada: o `nvmlInit` cria estado no driver, e soltar a DLL com ele de pé é mais arriscado.
fn carregar() -> Option<Nvml> {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let nome: Vec<u16> = "nvml.dll\0".encode_utf16().collect();
    let modulo = unsafe { LoadLibraryW(nome.as_ptr()) };
    // Sem o arquivo não há driver NVIDIA: é AMD ou Intel, e não um erro.
    if modulo.is_null() {
        return None;
    }

    let buscar = |simbolo: &std::ffi::CStr| unsafe { GetProcAddress(modulo, simbolo.as_ptr() as *const u8) };

    let iniciar: unsafe extern "C" fn() -> u32 = unsafe { std::mem::transmute(buscar(c"nvmlInit_v2")?) };
    if unsafe { iniciar() } != OK {
        return None;
    }

    let por_indice: unsafe extern "C" fn(u32, *mut *mut c_void) -> u32 =
        unsafe { std::mem::transmute(buscar(c"nvmlDeviceGetHandleByIndex_v2")?) };

    let mut dispositivo: *mut c_void = std::ptr::null_mut();
    if unsafe { por_indice(0, &mut dispositivo) } != OK || dispositivo.is_null() {
        return None;
    }

    Some(Nvml {
        dispositivo,
        temperatura: unsafe { std::mem::transmute(buscar(c"nvmlDeviceGetTemperature")?) },
        clock: unsafe { std::mem::transmute(buscar(c"nvmlDeviceGetClockInfo")?) },
        potencia: unsafe { std::mem::transmute(buscar(c"nvmlDeviceGetPowerUsage")?) },
        limite_de_potencia: unsafe { std::mem::transmute(buscar(c"nvmlDeviceGetEnforcedPowerLimit")?) },
        utilizacao: unsafe { std::mem::transmute(buscar(c"nvmlDeviceGetUtilizationRates")?) },
        // "ThrottleReasons" virou "EventReasons" em drivers novos; tenta os dois.
        motivos: buscar(c"nvmlDeviceGetCurrentClocksThrottleReasons")
            .or_else(|| buscar(c"nvmlDeviceGetCurrentClocksEventReasons"))
            .map(|p| unsafe { std::mem::transmute(p) }),
    })
}

/// Medido aqui: o motivo do clock segurado custa ~11 ms, o resto menos de 1 ms. Dentro da janela ele é
/// perguntado a cada segundo, para não empurrar o carimbo do disco que correlaciona travadas.
pub fn amostrar_com_motivos(incluir_motivos: bool) -> Option<AmostraGpu> {
    let n = nvml()?;
    let ler = |f: unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> u32, arg: u32| {
        let mut v = 0u32;
        (unsafe { f(n.dispositivo, arg, &mut v) } == OK).then_some(v)
    };
    let mut miliwatts = 0u32;
    let potencia = (unsafe { (n.potencia)(n.dispositivo, &mut miliwatts) } == OK).then(|| miliwatts as f64 / 1000.0);
    let mut uso = Utilizacao::default();
    let uso_pct = (unsafe { (n.utilizacao)(n.dispositivo, &mut uso) } == OK).then_some(uso.gpu);
    let motivos = incluir_motivos.then_some(n.motivos).flatten().and_then(|f| {
        let mut bits = 0u64;
        (unsafe { f(n.dispositivo, &mut bits) } == OK).then_some(bits)
    });

    Some(AmostraGpu {
        temperatura_c: ler(n.temperatura, SENSOR_GPU),
        clock_mhz: ler(n.clock, CLOCK_GRAFICO),
        potencia_w: potencia,
        uso_pct,
        motivos,
    })
}

/// Lido uma vez: não muda no meio da partida.
pub fn limite_de_potencia_w() -> Option<f64> {
    let n = nvml()?;
    let mut miliwatts = 0u32;
    (unsafe { (n.limite_de_potencia)(n.dispositivo, &mut miliwatts) } == OK).then(|| miliwatts as f64 / 1000.0)
}

#[cfg(test)]
mod nesta_maquina {
}

#[cfg(test)]
mod custo {
}
