// Os sensores da placa NVIDIA, EM PROCESSO, para amostrar durante a partida
//
// POR QUE NÃO O `nvidia-smi`
//
// O `sensoresgpu.rs` lê pelo `nvidia-smi`, e para uma leitura sob demanda isso
// está certo: é o cliente oficial, vem com o driver, e custar 100 ms uma vez
// não atrapalha ninguém. Mas amostrar DURANTE o jogo é outra coisa — abrir um
// processo a cada 200 ms enquanto se mede quadros seria virar a carga que se
// está medindo.
//
// A `nvml.dll` é a mesma biblioteca que o `nvidia-smi` usa por dentro, e ela
// acompanha o driver (`System32\nvml.dll`). Carregada uma vez, cada leitura é
// uma chamada de função.
//
// O QUE ISTO FECHA
//
// A pergunta que o produto não conseguia responder: "a placa estava limitada
// por temperatura DURANTE a partida?". Antes, a temperatura era lida depois,
// com o jogo fechado e a placa já fria — que é justamente quando ela não diz
// nada. Agora a resposta vem do intervalo medido, com o percentual do tempo em
// que o próprio driver disse que estava segurando o clock.
//
// NADA AQUI ESCREVE. São leituras.

#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::sync::OnceLock;

use crate::core::sensores::AmostraGpu;

/// `NVML_SUCCESS`.
const OK: u32 = 0;
/// `NVML_TEMPERATURE_GPU`.
const SENSOR_GPU: u32 = 0;
/// `NVML_CLOCK_GRAPHICS`.
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

// A NVML é usada de uma thread por vez (a que amostra), e o ponteiro do
// dispositivo é estável enquanto a biblioteca está carregada.
unsafe impl Send for Nvml {}
unsafe impl Sync for Nvml {}

fn nvml() -> Option<&'static Nvml> {
    static NVML: OnceLock<Option<Nvml>> = OnceLock::new();
    NVML.get_or_init(carregar).as_ref()
}

/// Carrega a `nvml.dll` e resolve o que este módulo usa.
///
/// Como a NVAPI, a biblioteca não é descarregada: o `nvmlInit` cria estado no
/// driver, e devolver a DLL com esse estado de pé é mais arriscado do que
/// segurar o identificador até o Otimiza fechar.
fn carregar() -> Option<Nvml> {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let nome: Vec<u16> = "nvml.dll\0".encode_utf16().collect();
    let modulo = unsafe { LoadLibraryW(nome.as_ptr()) };
    // Sem o arquivo não há driver NVIDIA: é o cliente com AMD ou Intel, e não
    // um erro.
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

    // A primeira placa. Máquina com duas NVIDIA é rara o bastante para não
    // valer um seletor que ninguém usaria; o nome aparece no painel da placa.
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
        // O nome mudou de "ThrottleReasons" para "EventReasons" em drivers
        // novos, e o antigo continua exportado por compatibilidade. Tenta os
        // dois: sem nenhum, o resto das leituras continua valendo.
        motivos: buscar(c"nvmlDeviceGetCurrentClocksThrottleReasons")
            .or_else(|| buscar(c"nvmlDeviceGetCurrentClocksEventReasons"))
            .map(|p| unsafe { std::mem::transmute(p) }),
    })
}

/// Lê os sensores agora. `None` quando não há placa NVIDIA aqui.
pub fn amostrar() -> Option<AmostraGpu> {
    amostrar_com_motivos(true)
}

/// A MESMA LEITURA, COM A PERGUNTA CARA OPCIONAL.
///
/// Medido nesta máquina, 100 chamadas de cada: temperatura 0,5 µs, clock
/// 0,9 ms, potência 0,5 ms, utilização 0,5 ms — e o motivo do clock estar
/// segurado, **11 ms**. Ele é o dado mais importante e o mais caro.
///
/// Dentro da janela de medição isso não pode acontecer a cada 200 ms: o mesmo
/// laço carimba o disco para correlacionar travadas, e 11 ms de atraso por
/// volta empurrariam esse carimbo. Então o motivo é perguntado a cada um
/// segundo, e o resto continua a cada volta. O resumo já conta a porcentagem
/// sobre as amostras que TÊM motivo, então misturar as duas cadências não
/// distorce a conta.
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

/// O limite de potência que o driver está impondo, em watts. Lido uma vez: ele
/// não muda no meio da partida.
pub fn limite_de_potencia_w() -> Option<f64> {
    let n = nvml()?;
    let mut miliwatts = 0u32;
    (unsafe { (n.limite_de_potencia)(n.dispositivo, &mut miliwatts) } == OK).then(|| miliwatts as f64 / 1000.0)
}

#[cfg(test)]
mod nesta_maquina {
    /// Lê os sensores desta placa por 2 segundos:
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
    /// Quanto custa uma amostra. Ela roda dentro da janela de medição de
    /// quadros: se custasse muito, viraria a carga que se está medindo.
    #[test]
    #[ignore]
    fn quanto_custa_uma_amostra() {
        let _ = super::amostrar();

        // A cadência que o vigia usa: motivo a cada 5 voltas.
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

        // Qual das chamadas custa? Sem isto, "otimizar" seria chute.
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
