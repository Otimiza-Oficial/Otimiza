// Ajustes de driver AMD pelo ADLX, a biblioteca oficial da AMD que vem com o Adrenalin (`amdadlx64.dll`). O mesmo
// cuidado do `nvdriver.rs`: a DLL é carregada em execução (sem placa AMD o Otimiza abre igual), só entra ajuste com
// leitura do valor anterior, e o Desfazer devolve exatamente o que estava.
//
// As posições das funções nas tabelas (vtables) foram copiadas dos cabeçalhos C oficiais do ADLX SDK 2.0
// (GPUOpen-LibrariesAndSDKs/ADLX, SDK/Include). Uma posição errada chamaria OUTRA função: elas ficam em constantes
// com o nome do cabeçalho, e não se acrescenta função sem conferir lá.

use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Adlx {
    Disponivel,
    SemPlacaAmd,
    /// A DLL existe e não respondeu: driver velho ou quebrado. Conselho oposto ao de "sem placa".
    NaoCarregou,
}

pub fn nota_do_estado(estado: Adlx) -> &'static str {
    match estado {
        Adlx::Disponivel => "Encontrei o driver da AMD. Cada ajuste volta ao que era pelo Desfazer.",
        Adlx::SemPlacaAmd => "Estes ajustes são para placas AMD Radeon.",
        Adlx::NaoCarregou => {
            "Encontrei uma placa AMD, mas o driver não respondeu ao Otimiza. Atualize o AMD Software: \
             Adrenalin Edition e tente de novo. Nada foi alterado."
        }
    }
}

/// O que o Otimiza sabe mudar no driver da AMD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opcao {
    /// Radeon Anti-Lag: menos atraso quando a placa está no limite.
    AntiLag,
    /// AMD Fluid Motion Frames: geração de quadros do driver.
    Afmf,
    /// Frame Rate Target Control: limite de FPS global do driver.
    LimiteDeFps(i32),
    /// Radeon Super Resolution: a escala do driver para jogos em tela cheia numa resolução menor (3.5).
    Rsr,
}

impl Opcao {
    pub fn id(self) -> String {
        match self {
            Opcao::AntiLag => "amd-antilag".to_string(),
            Opcao::Afmf => "amd-afmf".to_string(),
            Opcao::LimiteDeFps(_) => "amd-frtc".to_string(),
            Opcao::Rsr => "amd-rsr".to_string(),
        }
    }

    pub fn titulo(self) -> &'static str {
        match self {
            Opcao::AntiLag => "Radeon Anti-Lag",
            Opcao::Afmf => "AMD Fluid Motion Frames",
            Opcao::LimiteDeFps(_) => "Frame Rate Target Control",
            Opcao::Rsr => "Radeon Super Resolution",
        }
    }

    /// `amd-antilag`, `amd-afmf`, `amd-rsr`, `amd-frtc:141`.
    pub fn ler(texto: &str) -> Option<Opcao> {
        match texto.trim() {
            "amd-antilag" => Some(Opcao::AntiLag),
            "amd-afmf" => Some(Opcao::Afmf),
            "amd-rsr" => Some(Opcao::Rsr),
            outro => {
                let fps: i32 = outro.strip_prefix("amd-frtc:")?.parse().ok()?;
                (20..=1000)
                    .contains(&fps)
                    .then_some(Opcao::LimiteDeFps(fps))
            }
        }
    }
}

/// O estado anterior, em texto para o histórico: "1"/"0" para ligado e desligado; "1:141" para o limitador (ligado e
/// o número que estava).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anterior {
    Chave(bool),
    Limite { ligado: bool, fps: i32 },
}

pub fn codificar(a: Anterior) -> String {
    match a {
        Anterior::Chave(b) => (if b { "1" } else { "0" }).to_string(),
        Anterior::Limite { ligado, fps } => format!("{}:{}", if ligado { 1 } else { 0 }, fps),
    }
}

/// Pura. Texto ilegível vira `None`: desfazer às cegas escreveria um valor que ninguém escolheu.
pub fn decodificar(texto: &str) -> Option<Anterior> {
    match texto.trim() {
        "1" => Some(Anterior::Chave(true)),
        "0" => Some(Anterior::Chave(false)),
        outro => {
            let (l, f) = outro.split_once(':')?;
            let ligado = match l {
                "1" => true,
                "0" => false,
                _ => return None,
            };
            Some(Anterior::Limite {
                ligado,
                fps: f.parse().ok()?,
            })
        }
    }
}

pub fn registro(opcao: Opcao, anterior: Anterior) -> ChangeRecord {
    ChangeRecord::DriverAmd {
        opcao: opcao.id(),
        valor_anterior: codificar(anterior),
    }
}

/// Pura. Com APU e placa dedicada juntas, a dedicada é a do jogo. Tipo do ADLX: 0 indefinido, 1 integrada, 2 dedicada.
pub fn escolher_placa(tipos: &[i32]) -> Option<usize> {
    tipos
        .iter()
        .position(|t| *t == 2)
        .or_else(|| (!tipos.is_empty()).then_some(0))
}

/// O que o driver respondeu agora. `None` em cada campo: o recurso não existe nesta placa ou neste driver.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ajustes {
    pub anti_lag: Option<bool>,
    pub afmf: Option<bool>,
    #[serde(default)]
    pub rsr: Option<bool>,
    pub limite_ligado: Option<bool>,
    pub limite_fps: Option<i32>,
    pub limite_faixa: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sensores {
    pub uso_pct: Option<f64>,
    pub temperatura_c: Option<f64>,
    pub ponto_quente_c: Option<f64>,
    pub clock_mhz: Option<i32>,
    pub potencia_w: Option<f64>,
    pub vram_usada_mb: Option<i32>,
}

/// Um ajuste de cor do monitor: valor atual e faixa que o driver aceita.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Faixa {
    pub atual: i32,
    pub minimo: i32,
    pub maximo: i32,
    pub passo: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Cores {
    pub monitor: String,
    pub saturacao: Option<Faixa>,
    pub matiz: Option<Faixa>,
    pub brilho: Option<Faixa>,
    pub contraste: Option<Faixa>,
    pub temperatura: Option<Faixa>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cor {
    Saturacao,
    Matiz,
    Brilho,
    Contraste,
    Temperatura,
}

/// Pura: o valor pedido dentro da faixa e no passo do driver. Fora da faixa o driver recusaria, e a tela ficaria
/// mostrando um número que não foi aplicado.
pub fn ajustar_a_faixa(valor: i32, f: &Faixa) -> i32 {
    let v = valor.clamp(f.minimo, f.maximo);
    if f.passo > 1 {
        f.minimo + ((v - f.minimo) / f.passo) * f.passo
    } else {
        v
    }
}

#[cfg(target_os = "windows")]
mod ffi {
    //! A conversa com o ADLX. Cada interface é um ponteiro para uma tabela de funções (o "vtable" do C); a posição de
    //! cada função está nas constantes abaixo, copiadas dos cabeçalhos.

    use super::*;
    use std::ffi::{c_char, c_void};
    use std::ptr::null_mut;
    use std::sync::Mutex;

    // ADLXDefines.h: ADLX_RESULT. OK, ALREADY_ENABLED e ALREADY_INITIALIZED contam como sucesso (ADLX_SUCCEEDED).
    fn deu_certo(r: i32) -> bool {
        matches!(r, 0 | 1 | 2)
    }

    // ADLXVersion.h 2.0.0.125 e ADLX_MAKE_FULL_VER.
    const NOSSA_VERSAO: u64 = (2u64 << 48) | (0u64 << 32) | (0u64 << 16) | 125u64;

    // ISystem.h, IADLXSystemVtbl (sem Acquire/Release: o sistema é único).
    const SISTEMA_GET_GPUS: usize = 1;
    const SISTEMA_GET_DISPLAYS_SERVICES: usize = 3;
    const SISTEMA_GET_3D_SETTINGS_SERVICES: usize = 7;
    const SISTEMA_GET_PERFORMANCE_MONITORING_SERVICES: usize = 9;

    // IADLXInterfaceVtbl: todas as outras interfaces começam assim.
    const RELEASE: usize = 1;
    const QUERY_INTERFACE: usize = 2;

    // ICollections.h / ISystem.h: IADLXGPUList e IADLXDisplayList.
    const LISTA_SIZE: usize = 3;
    const LISTA_AT_TIPADO: usize = 11; // At_GPUList / At_DisplayList

    // ISystem.h, IADLXGPUVtbl.
    const GPU_TYPE: usize = 5;

    // I3DSettings.h, IADLX3DSettingsServicesVtbl; I3DSettings1.h, IADLX3DSettingsServices1Vtbl.
    const SERVICOS3D_GET_ANTI_LAG: usize = 3;
    const SERVICOS3D_GET_FRTC: usize = 9;
    // GetRadeonSuperResolution: global, SEM a placa no argumento (o único dos serviços 3D assim).
    const SERVICOS3D_GET_RSR: usize = 14;
    const SERVICOS3D1_GET_AFMF: usize = 17;

    // IADLX3DAntiLag, IADLX3DAMDFluidMotionFrames e IADLX3DRadeonSuperResolution: IsSupported, IsEnabled, SetEnabled.
    const CHAVE_IS_SUPPORTED: usize = 3;
    const CHAVE_IS_ENABLED: usize = 4;
    const CHAVE_SET_ENABLED: usize = 5;

    // IADLX3DFrameRateTargetControl.
    const FRTC_GET_FPS_RANGE: usize = 5;
    const FRTC_GET_FPS: usize = 6;
    const FRTC_SET_ENABLED: usize = 7;
    const FRTC_SET_FPS: usize = 8;

    // IDisplays.h, IADLXDisplayServicesVtbl e IADLXDisplayVtbl.
    const DISPLAYS_GET_DISPLAYS: usize = 4;
    const DISPLAYS_GET_CUSTOM_COLOR: usize = 16;
    const DISPLAY_NAME: usize = 6;

    // IDisplaySettings.h, IADLXCustomColorVtbl: para cada cor, IsSupported, GetRange, Get, Set, a partir de 3.
    fn base_da_cor(c: Cor) -> usize {
        3 + 4 * match c {
            Cor::Matiz => 0,
            Cor::Saturacao => 1,
            Cor::Brilho => 2,
            Cor::Contraste => 3,
            Cor::Temperatura => 4,
        }
    }

    // IPerformanceMonitoring.h.
    const DESEMPENHO_GET_CURRENT_GPU_METRICS: usize = 18;
    const METRICA_USO: usize = 4;
    const METRICA_CLOCK: usize = 5;
    const METRICA_TEMPERATURA: usize = 7;
    const METRICA_PONTO_QUENTE: usize = 8;
    const METRICA_POTENCIA: usize = 9;
    const METRICA_VRAM: usize = 12;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct IntRange {
        min: i32,
        max: i32,
        passo: i32,
    }

    unsafe fn funcao(objeto: *mut c_void, posicao: usize) -> *const c_void {
        let tabela = *(objeto as *const *const *const c_void);
        *tabela.add(posicao)
    }

    /// Chama a função `posicao` da interface com a assinatura dada. Todas recebem o próprio objeto primeiro.
    macro_rules! chamar {
        ($obj:expr, $pos:expr, ($($tipo:ty),*) -> $ret:ty $(, $arg:expr)*) => {{
            let f: unsafe extern "system" fn(*mut c_void $(, $tipo)*) -> $ret = std::mem::transmute(funcao($obj, $pos));
            f($obj $(, $arg)*)
        }};
    }

    /// Interface com dono: o `Release` do ADLX sai no `Drop`, nunca esquecido num caminho de erro.
    struct Interface(*mut c_void);

    impl Drop for Interface {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    chamar!(self.0, RELEASE, () -> i32);
                }
            }
        }
    }

    unsafe fn pegar(origem: *mut c_void, posicao: usize) -> Option<Interface> {
        let mut saida: *mut c_void = null_mut();
        let r = chamar!(origem, posicao, (*mut *mut c_void) -> i32, &mut saida);
        (deu_certo(r) && !saida.is_null()).then(|| Interface(saida))
    }

    unsafe fn pegar_com_gpu(
        origem: *mut c_void,
        posicao: usize,
        gpu: *mut c_void,
    ) -> Option<Interface> {
        let mut saida: *mut c_void = null_mut();
        let r = chamar!(origem, posicao, (*mut c_void, *mut *mut c_void) -> i32, gpu, &mut saida);
        (deu_certo(r) && !saida.is_null()).then(|| Interface(saida))
    }

    unsafe fn item(lista: *mut c_void, i: u32) -> Option<Interface> {
        let mut saida: *mut c_void = null_mut();
        let r = chamar!(lista, LISTA_AT_TIPADO, (u32, *mut *mut c_void) -> i32, i, &mut saida);
        (deu_certo(r) && !saida.is_null()).then(|| Interface(saida))
    }

    unsafe fn ler_bool(obj: *mut c_void, posicao: usize) -> Option<bool> {
        let mut v: u8 = 0;
        deu_certo(chamar!(obj, posicao, (*mut u8) -> i32, &mut v)).then_some(v != 0)
    }

    unsafe fn ler_i32(obj: *mut c_void, posicao: usize) -> Option<i32> {
        let mut v: i32 = 0;
        deu_certo(chamar!(obj, posicao, (*mut i32) -> i32, &mut v)).then_some(v)
    }

    unsafe fn ler_f64(obj: *mut c_void, posicao: usize) -> Option<f64> {
        let mut v: f64 = 0.0;
        deu_certo(chamar!(obj, posicao, (*mut f64) -> i32, &mut v))
            .then_some(v)
            .filter(|x| x.is_finite())
    }

    unsafe fn ler_faixa(obj: *mut c_void, posicao: usize) -> Option<IntRange> {
        let mut v = IntRange::default();
        deu_certo(chamar!(obj, posicao, (*mut IntRange) -> i32, &mut v)).then_some(v)
    }

    struct Sessao {
        sistema: *mut c_void,
        gpu: Interface,
        tres_d: Interface,
    }

    // O ADLX é usado só por trás do `Mutex` abaixo, uma chamada por vez.
    unsafe impl Send for Sessao {}

    static SESSAO: Mutex<Option<Result<Sessao, Adlx>>> = Mutex::new(None);

    /// A DLL nunca é descarregada nem o ADLX terminado: o driver guarda estado desde o `ADLXInitialize`, e objetos
    /// pendurados depois do `Terminate` apontam para memória inválida (ADLX_ORPHAN_OBJECTS).
    fn abrir() -> Result<Sessao, Adlx> {
        use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

        let nome: Vec<u16> = "amdadlx64.dll\0".encode_utf16().collect();
        let modulo = unsafe { LoadLibraryW(nome.as_ptr()) };
        if modulo.is_null() {
            return Err(Adlx::SemPlacaAmd);
        }

        unsafe {
            let iniciar = GetProcAddress(modulo, c"ADLXInitialize".as_ptr() as *const u8)
                .ok_or(Adlx::NaoCarregou)?;
            let iniciar: unsafe extern "C" fn(u64, *mut *mut c_void) -> i32 =
                std::mem::transmute(iniciar);

            // Pede a versão menor entre a nossa e a do driver: um Adrenalin mais velho recusaria a nossa (ADLX_BAD_VER),
            // e as tabelas só crescem no fim, então as posições que usamos valem nas duas.
            let versao = match GetProcAddress(modulo, c"ADLXQueryFullVersion".as_ptr() as *const u8)
            {
                Some(consulta) => {
                    let consulta: unsafe extern "C" fn(*mut u64) -> i32 =
                        std::mem::transmute(consulta);
                    let mut do_driver: u64 = 0;
                    if deu_certo(consulta(&mut do_driver)) && do_driver > 0 {
                        do_driver.min(NOSSA_VERSAO)
                    } else {
                        NOSSA_VERSAO
                    }
                }
                None => NOSSA_VERSAO,
            };

            let mut sistema: *mut c_void = null_mut();
            if !deu_certo(iniciar(versao, &mut sistema)) || sistema.is_null() {
                return Err(Adlx::NaoCarregou);
            }

            let lista = pegar(sistema, SISTEMA_GET_GPUS).ok_or(Adlx::NaoCarregou)?;
            let total = chamar!(lista.0, LISTA_SIZE, () -> u32);
            let placas: Vec<Interface> = (0..total).filter_map(|i| item(lista.0, i)).collect();
            if placas.is_empty() {
                return Err(Adlx::SemPlacaAmd);
            }

            let tipos: Vec<i32> = placas
                .iter()
                .map(|p| ler_i32(p.0, GPU_TYPE).unwrap_or(0))
                .collect();
            let escolhida = escolher_placa(&tipos).ok_or(Adlx::SemPlacaAmd)?;
            let gpu = placas.into_iter().nth(escolhida).ok_or(Adlx::SemPlacaAmd)?;

            let tres_d =
                pegar(sistema, SISTEMA_GET_3D_SETTINGS_SERVICES).ok_or(Adlx::NaoCarregou)?;

            Ok(Sessao {
                sistema,
                gpu,
                tres_d,
            })
        }
    }

    fn com_sessao<T>(f: impl FnOnce(&Sessao) -> Result<T, String>) -> Result<T, String> {
        let mut guarda = SESSAO
            .lock()
            .map_err(|_| "o acesso ao driver AMD travou".to_string())?;
        if guarda.is_none() {
            *guarda = Some(abrir());
        }
        match guarda.as_ref().expect("acabou de ser preenchida") {
            Ok(s) => f(s),
            Err(e) => Err(nota_do_estado(*e).to_string()),
        }
    }

    pub fn estado() -> Adlx {
        let mut guarda = match SESSAO.lock() {
            Ok(g) => g,
            Err(_) => return Adlx::NaoCarregou,
        };
        if guarda.is_none() {
            *guarda = Some(abrir());
        }
        match guarda.as_ref() {
            Some(Ok(_)) => Adlx::Disponivel,
            Some(Err(e)) => *e,
            None => Adlx::NaoCarregou,
        }
    }

    unsafe fn afmf(s: &Sessao) -> Option<Interface> {
        let id: Vec<u16> = "IADLX3DSettingsServices1\0".encode_utf16().collect();
        let mut servicos1: *mut c_void = null_mut();
        let r = chamar!(s.tres_d.0, QUERY_INTERFACE, (*const u16, *mut *mut c_void) -> i32, id.as_ptr(), &mut servicos1);
        if !deu_certo(r) || servicos1.is_null() {
            return None;
        }
        let servicos1 = Interface(servicos1);
        pegar(servicos1.0, SERVICOS3D1_GET_AFMF)
    }

    unsafe fn interface_da_opcao(s: &Sessao, o: Opcao) -> Option<Interface> {
        match o {
            Opcao::AntiLag => pegar_com_gpu(s.tres_d.0, SERVICOS3D_GET_ANTI_LAG, s.gpu.0),
            Opcao::LimiteDeFps(_) => pegar_com_gpu(s.tres_d.0, SERVICOS3D_GET_FRTC, s.gpu.0),
            Opcao::Afmf => afmf(s),
            Opcao::Rsr => pegar(s.tres_d.0, SERVICOS3D_GET_RSR),
        }
    }

    unsafe fn suportado(i: &Interface) -> bool {
        ler_bool(i.0, CHAVE_IS_SUPPORTED).unwrap_or(false)
    }

    pub fn ler_ajustes() -> Result<Ajustes, String> {
        com_sessao(|s| unsafe {
            let mut a = Ajustes::default();
            if let Some(i) = interface_da_opcao(s, Opcao::AntiLag).filter(|i| suportado(i)) {
                a.anti_lag = ler_bool(i.0, CHAVE_IS_ENABLED);
            }
            if let Some(i) = interface_da_opcao(s, Opcao::Afmf).filter(|i| suportado(i)) {
                a.afmf = ler_bool(i.0, CHAVE_IS_ENABLED);
            }
            if let Some(i) = interface_da_opcao(s, Opcao::Rsr).filter(|i| suportado(i)) {
                a.rsr = ler_bool(i.0, CHAVE_IS_ENABLED);
            }
            if let Some(i) = interface_da_opcao(s, Opcao::LimiteDeFps(0)).filter(|i| suportado(i)) {
                a.limite_ligado = ler_bool(i.0, CHAVE_IS_ENABLED);
                a.limite_fps = ler_i32(i.0, FRTC_GET_FPS);
                a.limite_faixa = ler_faixa(i.0, FRTC_GET_FPS_RANGE).map(|r| (r.min, r.max));
            }
            Ok(a)
        })
    }

    /// Liga a opção e devolve como estava, para o histórico.
    pub fn aplicar(o: Opcao) -> Result<Anterior, String> {
        com_sessao(|s| unsafe {
            let i = interface_da_opcao(s, o)
                .filter(|i| suportado(i))
                .ok_or_else(|| format!("{} não existe nesta placa ou neste driver.", o.titulo()))?;

            let ligado = ler_bool(i.0, CHAVE_IS_ENABLED).ok_or_else(|| {
                format!(
                    "O driver não disse se {} estava ligado. Nada foi alterado.",
                    o.titulo()
                )
            })?;

            match o {
                Opcao::AntiLag | Opcao::Afmf | Opcao::Rsr => {
                    if !deu_certo(chamar!(i.0, CHAVE_SET_ENABLED, (u8) -> i32, 1u8)) {
                        return Err(format!("O driver recusou ligar {}.", o.titulo()));
                    }
                    Ok(Anterior::Chave(ligado))
                }
                Opcao::LimiteDeFps(fps) => {
                    let antes = ler_i32(i.0, FRTC_GET_FPS).ok_or_else(|| {
                        "O driver não disse qual limite estava. Nada foi alterado.".to_string()
                    })?;
                    let fps = match ler_faixa(i.0, FRTC_GET_FPS_RANGE) {
                        Some(r) => fps.clamp(r.min, r.max),
                        None => fps,
                    };
                    if !deu_certo(chamar!(i.0, FRTC_SET_FPS, (i32) -> i32, fps))
                        || !deu_certo(chamar!(i.0, FRTC_SET_ENABLED, (u8) -> i32, 1u8))
                    {
                        // Meio aplicado não pode ficar: volta o número antigo antes de falhar.
                        let _ = chamar!(i.0, FRTC_SET_FPS, (i32) -> i32, antes);
                        return Err("O driver recusou o limite de FPS.".to_string());
                    }
                    Ok(Anterior::Limite { ligado, fps: antes })
                }
            }
        })
    }

    pub fn desfazer(opcao_id: &str, anterior: &str) -> Result<(), String> {
        let anterior = decodificar(anterior).ok_or_else(|| {
            format!(
                "o valor anterior de {} está ilegível no histórico",
                opcao_id
            )
        })?;
        let o = match opcao_id {
            "amd-antilag" => Opcao::AntiLag,
            "amd-afmf" => Opcao::Afmf,
            "amd-rsr" => Opcao::Rsr,
            "amd-frtc" => Opcao::LimiteDeFps(0),
            outro => return Err(format!("não conheço o ajuste AMD \"{}\"", outro)),
        };

        com_sessao(|s| unsafe {
            let i = interface_da_opcao(s, o)
                .ok_or_else(|| format!("{} sumiu do driver", o.titulo()))?;
            let ok = match anterior {
                Anterior::Chave(b) => {
                    deu_certo(chamar!(i.0, CHAVE_SET_ENABLED, (u8) -> i32, b as u8))
                }
                Anterior::Limite { ligado, fps } => {
                    deu_certo(chamar!(i.0, FRTC_SET_FPS, (i32) -> i32, fps))
                        && deu_certo(chamar!(i.0, FRTC_SET_ENABLED, (u8) -> i32, ligado as u8))
                }
            };
            ok.then_some(())
                .ok_or_else(|| format!("o driver recusou devolver {}", o.titulo()))
        })
    }

    pub fn sensores() -> Result<Sensores, String> {
        com_sessao(|s| unsafe {
            let desempenho = pegar(s.sistema, SISTEMA_GET_PERFORMANCE_MONITORING_SERVICES)
                .ok_or_else(|| "o driver não deu os sensores".to_string())?;
            let m = pegar_com_gpu(desempenho.0, DESEMPENHO_GET_CURRENT_GPU_METRICS, s.gpu.0)
                .ok_or_else(|| "o driver não deu os sensores".to_string())?;
            Ok(Sensores {
                uso_pct: ler_f64(m.0, METRICA_USO),
                temperatura_c: ler_f64(m.0, METRICA_TEMPERATURA),
                ponto_quente_c: ler_f64(m.0, METRICA_PONTO_QUENTE),
                clock_mhz: ler_i32(m.0, METRICA_CLOCK),
                potencia_w: ler_f64(m.0, METRICA_POTENCIA),
                vram_usada_mb: ler_i32(m.0, METRICA_VRAM),
            })
        })
    }

    unsafe fn monitores(s: &Sessao) -> Option<(Interface, Vec<Interface>)> {
        let servicos = pegar(s.sistema, SISTEMA_GET_DISPLAYS_SERVICES)?;
        let lista = pegar(servicos.0, DISPLAYS_GET_DISPLAYS)?;
        let total = chamar!(lista.0, LISTA_SIZE, () -> u32);
        let telas = (0..total).filter_map(|i| item(lista.0, i)).collect();
        Some((servicos, telas))
    }

    unsafe fn ler_cor(cc: *mut c_void, c: Cor) -> Option<Faixa> {
        let b = base_da_cor(c);
        if !ler_bool(cc, b)? {
            return None;
        }
        let r = ler_faixa(cc, b + 1)?;
        let atual = ler_i32(cc, b + 2)?;
        Some(Faixa {
            atual,
            minimo: r.min,
            maximo: r.max,
            passo: r.passo.max(1),
        })
    }

    pub fn cores() -> Result<Vec<Cores>, String> {
        com_sessao(|s| unsafe {
            let (servicos, telas) =
                monitores(s).ok_or_else(|| "o driver não listou os monitores".to_string())?;
            Ok(telas
                .iter()
                .filter_map(|t| {
                    let cc = pegar_com_gpu(servicos.0, DISPLAYS_GET_CUSTOM_COLOR, t.0)?;
                    let mut nome: *const c_char = std::ptr::null();
                    let _ = chamar!(t.0, DISPLAY_NAME, (*mut *const c_char) -> i32, &mut nome);
                    let monitor = if nome.is_null() {
                        "Monitor".to_string()
                    } else {
                        std::ffi::CStr::from_ptr(nome)
                            .to_string_lossy()
                            .into_owned()
                    };
                    Some(Cores {
                        monitor,
                        saturacao: ler_cor(cc.0, Cor::Saturacao),
                        matiz: ler_cor(cc.0, Cor::Matiz),
                        brilho: ler_cor(cc.0, Cor::Brilho),
                        contraste: ler_cor(cc.0, Cor::Contraste),
                        temperatura: ler_cor(cc.0, Cor::Temperatura),
                    })
                })
                .collect())
        })
    }

    /// Escreve uma cor num monitor (índice da lista do ADLX) e devolve o valor que estava.
    pub fn escrever_cor(monitor: usize, c: Cor, valor: i32) -> Result<i32, String> {
        com_sessao(|s| unsafe {
            let (servicos, telas) =
                monitores(s).ok_or_else(|| "o driver não listou os monitores".to_string())?;
            let t = telas
                .get(monitor)
                .ok_or_else(|| "esse monitor não está mais ligado".to_string())?;
            let cc = pegar_com_gpu(servicos.0, DISPLAYS_GET_CUSTOM_COLOR, t.0)
                .ok_or_else(|| "o driver não deu as cores deste monitor".to_string())?;
            let faixa = ler_cor(cc.0, c)
                .ok_or_else(|| "este monitor não aceita esse ajuste".to_string())?;
            let v = ajustar_a_faixa(valor, &faixa);
            if !deu_certo(chamar!(cc.0, base_da_cor(c) + 3, (i32) -> i32, v)) {
                return Err("o driver recusou a cor".to_string());
            }
            Ok(faixa.atual)
        })
    }
}

#[cfg(target_os = "windows")]
pub use ffi::{aplicar, cores, desfazer, escrever_cor, estado, ler_ajustes, sensores};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcoes_vao_e_voltam_pelo_texto() {
        for o in [Opcao::AntiLag, Opcao::Afmf, Opcao::Rsr] {
            assert_eq!(Opcao::ler(&o.id()), Some(o));
        }
        assert_eq!(Opcao::ler("amd-frtc:141"), Some(Opcao::LimiteDeFps(141)));
        assert_eq!(Opcao::ler("amd-frtc:5"), None, "limite absurdo é recusado");
        assert_eq!(Opcao::ler("amd-frtc:abc"), None);
        assert_eq!(Opcao::ler("nvidia-qualquer"), None);
    }

    #[test]
    fn anterior_vai_e_volta_pelo_texto() {
        for a in [
            Anterior::Chave(true),
            Anterior::Chave(false),
            Anterior::Limite {
                ligado: true,
                fps: 141,
            },
            Anterior::Limite {
                ligado: false,
                fps: 200,
            },
        ] {
            assert_eq!(decodificar(&codificar(a)), Some(a));
        }
    }

    #[test]
    fn anterior_ilegivel_nao_vira_desfazer_as_cegas() {
        for ruim in ["", "sim", "2:100", "1:", ":100", "1:abc"] {
            assert_eq!(decodificar(ruim), None, "{:?}", ruim);
        }
    }

    #[test]
    fn com_apu_e_dedicada_escolhe_a_dedicada() {
        assert_eq!(escolher_placa(&[1, 2]), Some(1));
        assert_eq!(escolher_placa(&[2, 1]), Some(0));
        assert_eq!(escolher_placa(&[1]), Some(0), "só a integrada: é ela");
        assert_eq!(escolher_placa(&[]), None);
    }

    #[test]
    fn cor_pedida_cai_dentro_da_faixa_e_do_passo() {
        let f = Faixa {
            atual: 100,
            minimo: 0,
            maximo: 200,
            passo: 1,
        };
        assert_eq!(ajustar_a_faixa(250, &f), 200);
        assert_eq!(ajustar_a_faixa(-3, &f), 0);
        let com_passo = Faixa {
            atual: 0,
            minimo: -30,
            maximo: 30,
            passo: 5,
        };
        assert_eq!(ajustar_a_faixa(12, &com_passo), 10);
        assert_eq!(ajustar_a_faixa(-12, &com_passo), -15);
    }

    #[test]
    fn registro_vai_para_o_historico_com_id_e_anterior() {
        match registro(
            Opcao::LimiteDeFps(141),
            Anterior::Limite {
                ligado: false,
                fps: 200,
            },
        ) {
            ChangeRecord::DriverAmd {
                opcao,
                valor_anterior,
            } => {
                assert_eq!(opcao, "amd-frtc");
                assert_eq!(valor_anterior, "0:200");
            }
            outro => panic!("registro errado: {:?}", outro),
        }
    }

    #[test]
    fn so_aviso_de_driver_quando_a_placa_e_amd() {
        assert!(!nota_do_estado(Adlx::SemPlacaAmd).contains("Atualize"));
        assert!(nota_do_estado(Adlx::NaoCarregou).contains("Adrenalin"));
    }

    #[test]
    fn as_posicoes_das_tabelas_batem_com_os_cabecalhos() {
        // Guarda contra edição distraída: cada número aqui foi conferido no ADLX SDK 2.0.
        let fonte = include_str!("amddriver.rs");
        for (nome, valor) in [
            ("SISTEMA_GET_GPUS: usize = 1", 1),
            ("SISTEMA_GET_3D_SETTINGS_SERVICES: usize = 7", 7),
            ("SERVICOS3D_GET_FRTC: usize = 9", 9),
            ("SERVICOS3D_GET_RSR: usize = 14", 14),
            ("SERVICOS3D1_GET_AFMF: usize = 17", 17),
            ("DISPLAYS_GET_CUSTOM_COLOR: usize = 16", 16),
            ("DESEMPENHO_GET_CURRENT_GPU_METRICS: usize = 18", 18),
        ] {
            assert!(
                fonte.contains(nome),
                "a posição {} mudou: confira no cabeçalho do ADLX",
                valor
            );
        }
    }

    #[test]
    #[ignore = "precisa de uma placa AMD com o Adrenalin"]
    #[cfg(target_os = "windows")]
    fn le_o_driver_amd_desta_maquina() {
        println!("estado: {:?}", estado());
        println!("ajustes: {:?}", ler_ajustes());
        println!("sensores: {:?}", sensores());
        println!("cores: {:?}", cores());
    }
}
