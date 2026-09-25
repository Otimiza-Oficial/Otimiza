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
    nome: unsafe extern "C" fn(*mut c_void, *mut u8, u32) -> u32,
    clock_maximo: unsafe extern "C" fn(*mut c_void, u32, *mut u32) -> u32,
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
        nome: unsafe { std::mem::transmute(buscar(c"nvmlDeviceGetName")?) },
        clock_maximo: unsafe { std::mem::transmute(buscar(c"nvmlDeviceGetMaxClockInfo")?) },
        // "ThrottleReasons" virou "EventReasons" em drivers novos; tenta os dois.
        motivos: buscar(c"nvmlDeviceGetCurrentClocksThrottleReasons")
            .or_else(|| buscar(c"nvmlDeviceGetCurrentClocksEventReasons"))
            .map(|p| unsafe { std::mem::transmute(p) }),
    })
}

pub fn amostrar() -> Option<AmostraGpu> {
    amostrar_com_motivos(true)
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

pub fn nome_da_placa() -> Option<String> {
    let n = nvml()?;
    // 96 bytes é o tamanho que a NVML documenta para o nome.
    let mut buffer = [0u8; 96];
    if unsafe { (n.nome)(n.dispositivo, buffer.as_mut_ptr(), buffer.len() as u32) } != OK {
        return None;
    }
    let fim = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
    String::from_utf8(buffer[..fim].to_vec()).ok().filter(|s| !s.is_empty())
}

pub fn clock_maximo_mhz() -> Option<u32> {
    let n = nvml()?;
    let mut v = 0u32;
    (unsafe { (n.clock_maximo)(n.dispositivo, CLOCK_GRAFICO, &mut v) } == OK).then_some(v)
}

/// Lido uma vez: não muda no meio da partida.
pub fn limite_de_potencia_w() -> Option<f64> {
    let n = nvml()?;
    let mut miliwatts = 0u32;
    (unsafe { (n.limite_de_potencia)(n.dispositivo, &mut miliwatts) } == OK).then(|| miliwatts as f64 / 1000.0)
}

#[cfg(test)]
mod nesta_maquina {
    /// `cargo test --lib -- --ignored sensores_ao_vivo --nocapture`.
    #[test]
    #[ignore]
    fn sensores_ao_vivo() {
        let limite = super::limite_de_potencia_w();
        let mut amostras = Vec::new();
        for _ in 0..10 {
            if let Some(a) = super::amostrar() {
                amostras.push(a);
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        println!("limite de potência: {limite:?}");
        println!("primeira amostra: {:?}", amostras.first());
        let r = crate::core::sensores::resumir(&amostras, limite).expect("resumo");
        println!("{r:#?}");
        println!("veredito: {:?}", crate::core::sensores::julgar(&r));
    }
}

#[cfg(test)]
mod custo {
    /// Roda dentro da janela de medição de quadros: se custasse muito, viraria a carga medida.
    #[test]
    #[ignore]
    fn quanto_custa_uma_amostra() {
        let _ = super::amostrar();

        let t = std::time::Instant::now();
        for volta in 1..=100u32 {
            let _ = super::amostrar_com_motivos(volta % 5 == 1);
        }
        println!("cadência do vigia (1 motivo a cada 5): {:?} cada", t.elapsed() / 100);

        let t = std::time::Instant::now();
        for _ in 0..100 {
            let _ = super::amostrar_com_motivos(false);
        }
        println!("sem motivo: {:?} cada", t.elapsed() / 100);

        let inicio = std::time::Instant::now();
        for _ in 0..100 {
            let _ = super::amostrar();
        }
        println!("100 amostras: {:?} ({:?} cada)", inicio.elapsed(), inicio.elapsed() / 100);

        let n = super::nvml().expect("nvml");
        let medir = |nome: &str, f: &dyn Fn()| {
            let t = std::time::Instant::now();
            for _ in 0..100 {
                f();
            }
            println!("  {nome}: {:?} cada", t.elapsed() / 100);
        };
        medir("temperatura", &|| {
            let mut v = 0u32;
            unsafe { (n.temperatura)(n.dispositivo, super::SENSOR_GPU, &mut v) };
        });
        medir("clock", &|| {
            let mut v = 0u32;
            unsafe { (n.clock)(n.dispositivo, super::CLOCK_GRAFICO, &mut v) };
        });
        medir("potencia", &|| {
            let mut v = 0u32;
            unsafe { (n.potencia)(n.dispositivo, &mut v) };
        });
        medir("utilizacao", &|| {
            let mut u = super::Utilizacao::default();
            unsafe { (n.utilizacao)(n.dispositivo, &mut u) };
        });
        medir("motivos", &|| {
            let mut bits = 0u64;
            if let Some(f) = n.motivos {
                unsafe { f(n.dispositivo, &mut bits) };
            }
        });
    }
}
