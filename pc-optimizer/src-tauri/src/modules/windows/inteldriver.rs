// Ajustes de driver Intel (Arc e integradas) pelo IGCL, a biblioteca de controle oficial da Intel que vem com o driver
// (`ControlLib.dll`, na System32). O mesmo contrato do `nvdriver.rs` e do `amddriver.rs`: carregada em execução, só
// entra ajuste com leitura do anterior, e o Desfazer devolve o que estava.
//
// As estruturas foram copiadas de `igcl_api.h` (intel/drivers.gpu.control-library, versão 1.1). Elas têm tamanho e
// versão no começo: o driver confere os dois, então um campo fora de ordem vira recusa, não corrupção.
//
// Cores do monitor pela Intel ficam de fora por enquanto: no IGCL elas passam pela transformação de pixels (matrizes e
// tabelas), que precisa de prova numa placa Intel antes de entrar.

use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Igcl {
    Disponivel,
    SemPlacaIntel,
    NaoCarregou,
}

pub fn nota_do_estado(estado: Igcl) -> &'static str {
    match estado {
        Igcl::Disponivel => "Encontrei o driver da Intel. Cada ajuste volta ao que era pelo Desfazer.",
        Igcl::SemPlacaIntel => "Estes ajustes são para placas de vídeo Intel.",
        Igcl::NaoCarregou => {
            "Encontrei uma placa Intel, mas o driver não respondeu ao Otimiza. Atualize o driver da Intel \
             (Intel Graphics Software) e tente de novo. Nada foi alterado."
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opcao {
    BaixaLatencia,
    LimiteDeFps(i32),
}

impl Opcao {
    pub fn id(self) -> &'static str {
        match self {
            Opcao::BaixaLatencia => "intel-latencia",
            Opcao::LimiteDeFps(_) => "intel-limite",
        }
    }

    pub fn titulo(self) -> &'static str {
        match self {
            Opcao::BaixaLatencia => "Baixa latência (Intel)",
            Opcao::LimiteDeFps(_) => "Limitador de FPS (Intel)",
        }
    }

    pub fn ler(texto: &str) -> Option<Opcao> {
        match texto.trim() {
            "intel-latencia" => Some(Opcao::BaixaLatencia),
            outro => {
                let fps: i32 = outro.strip_prefix("intel-limite:")?.parse().ok()?;
                (20..=1000)
                    .contains(&fps)
                    .then_some(Opcao::LimiteDeFps(fps))
            }
        }
    }
}

/// "e:1" (modo de baixa latência que estava: 0 desligado, 1 ligado, 2 ligado com boost); "l:1:141" (limitador ligado
/// e o número).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anterior {
    Modo(u32),
    Limite { ligado: bool, fps: i32 },
}

pub fn codificar(a: Anterior) -> String {
    match a {
        Anterior::Modo(m) => format!("e:{}", m),
        Anterior::Limite { ligado, fps } => format!("l:{}:{}", ligado as u8, fps),
    }
}

pub fn decodificar(texto: &str) -> Option<Anterior> {
    let partes: Vec<&str> = texto.trim().split(':').collect();
    match partes.as_slice() {
        ["e", m] => m.parse().ok().filter(|m| *m <= 2).map(Anterior::Modo),
        ["l", l, f] => {
            let ligado = match *l {
                "1" => true,
                "0" => false,
                _ => return None,
            };
            Some(Anterior::Limite {
                ligado,
                fps: f.parse().ok()?,
            })
        }
        _ => None,
    }
}

pub fn registro(opcao: Opcao, anterior: Anterior) -> ChangeRecord {
    ChangeRecord::DriverIntel {
        opcao: opcao.id().to_string(),
        valor_anterior: codificar(anterior),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ajustes {
    /// 0 desligado, 1 ligado, 2 ligado com boost.
    pub baixa_latencia: Option<u32>,
    pub limite_ligado: Option<bool>,
    pub limite_fps: Option<i32>,
}

#[cfg(target_os = "windows")]
mod ffi {
    use super::*;
    use std::ffi::c_void;
    use std::ptr::null_mut;
    use std::sync::Mutex;

    const SUCESSO: u32 = 0;

    // CTL_MAKE_VERSION(1, 1).
    const VERSAO_DO_APP: u32 = (1 << 16) | 1;

    // ctl_property_value_type_t.
    const TIPO_INT32: i32 = 2;
    const TIPO_ENUM: i32 = 4;

    // ctl_3d_feature_t.
    const RECURSO_LIMITE_DE_QUADROS: i32 = 2;
    const RECURSO_BAIXA_LATENCIA: i32 = 16;

    #[repr(C)]
    #[derive(Default)]
    struct IdDoApp {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    /// ctl_init_args_t.
    #[repr(C)]
    #[derive(Default)]
    struct ArgsDeInicio {
        tamanho: u32,
        versao: u8,
        versao_do_app: u32,
        flags: u32,
        versao_suportada: u32,
        id_do_app: IdDoApp,
    }

    /// ctl_property_int_t e ctl_property_enum_t dentro de ctl_property_t (união de 8 bytes, alinhada a 4).
    #[repr(C)]
    #[derive(Clone, Copy)]
    union Valor {
        inteiro: Inteiro,
        enumerado: u32,
        _bruto: [u32; 2],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Inteiro {
        ligado: bool,
        valor: i32,
    }

    /// ctl_3d_feature_getset_t.
    #[repr(C)]
    struct Recurso3d {
        tamanho: u32,
        versao: u8,
        recurso: i32,
        nome_do_app: *mut u8,
        tamanho_do_nome: i8,
        escrever: bool,
        tipo: i32,
        valor: Valor,
        tamanho_custom: i32,
        custom: *mut c_void,
    }

    // A ordem e o tamanho do driver: conferidos no teste do fim (56 bytes em 64 bits).
    pub(super) const TAMANHO_RECURSO3D: usize = std::mem::size_of::<Recurso3d>();
    pub(super) const TAMANHO_INICIO: usize = std::mem::size_of::<ArgsDeInicio>();

    type FnInit = unsafe extern "C" fn(*mut ArgsDeInicio, *mut *mut c_void) -> u32;
    type FnEnumerar = unsafe extern "C" fn(*mut c_void, *mut u32, *mut *mut c_void) -> u32;
    type FnRecurso3d = unsafe extern "C" fn(*mut c_void, *mut Recurso3d) -> u32;

    struct Sessao {
        placa: *mut c_void,
        recurso3d: FnRecurso3d,
    }
    unsafe impl Send for Sessao {}

    static SESSAO: Mutex<Option<Result<Sessao, Igcl>>> = Mutex::new(None);

    fn abrir() -> Result<Sessao, Igcl> {
        use windows_sys::Win32::System::LibraryLoader::{
            GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32,
        };

        // Só da System32, como o wrapper oficial: um ControlLib.dll na pasta do programa não é carregado.
        let nome: Vec<u16> = "ControlLib.dll\0".encode_utf16().collect();
        let modulo = unsafe {
            LoadLibraryExW(
                nome.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if modulo.is_null() {
            return Err(Igcl::SemPlacaIntel);
        }

        unsafe {
            let buscar = |n: &std::ffi::CStr| GetProcAddress(modulo, n.as_ptr() as *const u8);
            let iniciar: FnInit = std::mem::transmute(buscar(c"ctlInit").ok_or(Igcl::NaoCarregou)?);
            let enumerar: FnEnumerar =
                std::mem::transmute(buscar(c"ctlEnumerateDevices").ok_or(Igcl::NaoCarregou)?);
            let recurso3d: FnRecurso3d =
                std::mem::transmute(buscar(c"ctlGetSet3DFeature").ok_or(Igcl::NaoCarregou)?);

            let mut args = ArgsDeInicio {
                tamanho: TAMANHO_INICIO as u32,
                versao: 0,
                versao_do_app: VERSAO_DO_APP,
                ..Default::default()
            };
            let mut api: *mut c_void = null_mut();
            if iniciar(&mut args, &mut api) != SUCESSO || api.is_null() {
                return Err(Igcl::NaoCarregou);
            }

            let mut total: u32 = 0;
            if enumerar(api, &mut total, null_mut()) != SUCESSO || total == 0 {
                return Err(Igcl::SemPlacaIntel);
            }
            let mut placas: Vec<*mut c_void> = vec![null_mut(); total as usize];
            if enumerar(api, &mut total, placas.as_mut_ptr()) != SUCESSO {
                return Err(Igcl::NaoCarregou);
            }

            // A primeira da lista. Com Arc e integrada juntas, o ajuste global vale para a que o Windows usa no jogo;
            // escolher entre elas pede as propriedades do adaptador, que ainda não foram provadas numa máquina Intel.
            let placa = placas
                .into_iter()
                .find(|p| !p.is_null())
                .ok_or(Igcl::SemPlacaIntel)?;
            Ok(Sessao { placa, recurso3d })
        }
    }

    fn com_sessao<T>(f: impl FnOnce(&Sessao) -> Result<T, String>) -> Result<T, String> {
        let mut g = SESSAO
            .lock()
            .map_err(|_| "o acesso ao driver Intel travou".to_string())?;
        if g.is_none() {
            *g = Some(abrir());
        }
        match g.as_ref().expect("acabou de ser preenchida") {
            Ok(s) => f(s),
            Err(e) => Err(nota_do_estado(*e).to_string()),
        }
    }

    pub fn estado() -> Igcl {
        let mut g = match SESSAO.lock() {
            Ok(g) => g,
            Err(_) => return Igcl::NaoCarregou,
        };
        if g.is_none() {
            *g = Some(abrir());
        }
        match g.as_ref() {
            Some(Ok(_)) => Igcl::Disponivel,
            Some(Err(e)) => *e,
            None => Igcl::NaoCarregou,
        }
    }

    /// Lê ou escreve um recurso no perfil GLOBAL (nome de aplicativo vazio, como o cabeçalho pede).
    unsafe fn pedir(
        s: &Sessao,
        recurso: i32,
        tipo: i32,
        escrever: bool,
        valor: Valor,
    ) -> Option<Valor> {
        let mut vazio = [0u8; 1];
        let mut r = Recurso3d {
            tamanho: TAMANHO_RECURSO3D as u32,
            versao: 0,
            recurso,
            nome_do_app: vazio.as_mut_ptr(),
            tamanho_do_nome: 0,
            escrever,
            tipo,
            valor,
            tamanho_custom: 0,
            custom: null_mut(),
        };
        ((s.recurso3d)(s.placa, &mut r) == SUCESSO).then_some(r.valor)
    }

    unsafe fn ler_modo(s: &Sessao) -> Option<u32> {
        pedir(
            s,
            RECURSO_BAIXA_LATENCIA,
            TIPO_ENUM,
            false,
            Valor { _bruto: [0; 2] },
        )
        .map(|v| v.enumerado)
    }

    unsafe fn ler_limite(s: &Sessao) -> Option<Inteiro> {
        pedir(
            s,
            RECURSO_LIMITE_DE_QUADROS,
            TIPO_INT32,
            false,
            Valor { _bruto: [0; 2] },
        )
        .map(|v| v.inteiro)
    }

    unsafe fn escrever_modo(s: &Sessao, m: u32) -> bool {
        let mut v = Valor { _bruto: [0; 2] };
        v.enumerado = m;
        pedir(s, RECURSO_BAIXA_LATENCIA, TIPO_ENUM, true, v).is_some()
    }

    unsafe fn escrever_limite(s: &Sessao, ligado: bool, fps: i32) -> bool {
        let v = Valor {
            inteiro: Inteiro { ligado, valor: fps },
        };
        pedir(s, RECURSO_LIMITE_DE_QUADROS, TIPO_INT32, true, v).is_some()
    }

    pub fn ler_ajustes() -> Result<Ajustes, String> {
        com_sessao(|s| unsafe {
            let limite = ler_limite(s);
            Ok(Ajustes {
                baixa_latencia: ler_modo(s),
                limite_ligado: limite.map(|l| l.ligado),
                limite_fps: limite.map(|l| l.valor),
            })
        })
    }

    pub fn aplicar(o: Opcao) -> Result<Anterior, String> {
        com_sessao(|s| unsafe {
            match o {
                Opcao::BaixaLatencia => {
                    let antes = ler_modo(s).ok_or_else(|| {
                        "O driver Intel não disse como estava a baixa latência. Nada foi alterado.".to_string()
                    })?;
                    if antes != 0 {
                        return Ok(Anterior::Modo(antes));
                    }
                    escrever_modo(s, 1)
                        .then_some(Anterior::Modo(antes))
                        .ok_or_else(|| "O driver Intel recusou ligar a baixa latência.".to_string())
                }
                Opcao::LimiteDeFps(fps) => {
                    let antes = ler_limite(s).ok_or_else(|| {
                        "O driver Intel não disse qual limite estava. Nada foi alterado."
                            .to_string()
                    })?;
                    escrever_limite(s, true, fps)
                        .then_some(Anterior::Limite {
                            ligado: antes.ligado,
                            fps: antes.valor,
                        })
                        .ok_or_else(|| "O driver Intel recusou o limite de FPS.".to_string())
                }
            }
        })
    }

    pub fn desfazer(opcao_id: &str, anterior: &str) -> Result<(), String> {
        let a = decodificar(anterior).ok_or_else(|| {
            format!(
                "o valor anterior de {} está ilegível no histórico",
                opcao_id
            )
        })?;
        com_sessao(|s| unsafe {
            let ok = match (opcao_id, a) {
                ("intel-latencia", Anterior::Modo(m)) => escrever_modo(s, m),
                ("intel-limite", Anterior::Limite { ligado, fps }) => {
                    escrever_limite(s, ligado, fps)
                }
                _ => {
                    return Err(format!(
                        "o histórico de {} não confere com o ajuste",
                        opcao_id
                    ))
                }
            };
            ok.then_some(())
                .ok_or_else(|| format!("o driver Intel recusou devolver {}", opcao_id))
        })
    }
}

#[cfg(target_os = "windows")]
pub use ffi::{aplicar, desfazer, estado, ler_ajustes};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcoes_e_anteriores_vao_e_voltam() {
        assert_eq!(Opcao::ler("intel-latencia"), Some(Opcao::BaixaLatencia));
        assert_eq!(
            Opcao::ler("intel-limite:141"),
            Some(Opcao::LimiteDeFps(141))
        );
        assert_eq!(Opcao::ler("intel-limite:3"), None);
        for a in [
            Anterior::Modo(0),
            Anterior::Modo(2),
            Anterior::Limite {
                ligado: true,
                fps: 141,
            },
            Anterior::Limite {
                ligado: false,
                fps: 0,
            },
        ] {
            assert_eq!(decodificar(&codificar(a)), Some(a));
        }
    }

    #[test]
    fn anterior_ilegivel_nao_desfaz_as_cegas() {
        for ruim in ["", "e:9", "e:", "l:2:10", "l:1", "x:1"] {
            assert_eq!(decodificar(ruim), None, "{:?}", ruim);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn estruturas_tem_o_tamanho_do_cabecalho() {
        // ctl_3d_feature_getset_t: u32, u8, enum, ponteiro, i8, bool, enum, união de 8, i32, ponteiro.
        assert_eq!(ffi::TAMANHO_RECURSO3D, 56);
        // ctl_init_args_t: u32, u8, u32, u32, u32 e o id de 16 bytes.
        assert_eq!(ffi::TAMANHO_INICIO, 36);
    }

    #[test]
    fn registro_vai_para_o_historico() {
        match registro(Opcao::BaixaLatencia, Anterior::Modo(0)) {
            ChangeRecord::DriverIntel {
                opcao,
                valor_anterior,
            } => {
                assert_eq!(opcao, "intel-latencia");
                assert_eq!(valor_anterior, "e:0");
            }
            outro => panic!("{:?}", outro),
        }
    }

    #[test]
    #[ignore = "precisa de uma placa Intel com o driver"]
    #[cfg(target_os = "windows")]
    fn le_o_driver_intel_desta_maquina() {
        println!("estado: {:?}", estado());
        println!("ajustes: {:?}", ler_ajustes());
    }
}
