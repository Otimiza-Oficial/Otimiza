// Ajustes de driver NVIDIA pela NVAPI (DRS). Só entra opção com RESTAURAÇÃO DE PADRÃO da própria NVAPI
// (`id_do_padrao`; há teste que recusa entrada sem ela). AMD fica de fora e a tela diz "ainda não cobrimos sua
// placa". A `nvapi64.dll` é carregada em execução (`LoadLibraryW`): ligada na compilação, o Otimiza não abriria sem
// placa NVIDIA. Antes de escrever, pergunta ao driver o nome do número (`NvAPI_DRS_GetSettingNameFromId`) e confere
// com a tabela: número errado escreveria em outra opção. Na dúvida, não escreve.

use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::sync::OnceLock;

/// TRÊS casos: sem placa NVIDIA (permanente) e DLL que não responde (pede driver novo) têm conselhos opostos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nvapi {
    Disponivel,
    SemPlacaNvidia,
    NaoCarregou,
}

pub fn nota_do_estado(estado: Nvapi) -> String {
    match estado {
        Nvapi::Disponivel => "Encontrei o driver da NVIDIA. Os ajustes abaixo são aplicados no \
             perfil global do driver, e cada um volta ao padrão de fábrica pelo \
             \"Desfazer\"."
            .to_string(),
        // Sem a palavra "driver": mandar o dono de uma Radeon atualizar o driver da NVIDIA seria o conselho errado.
        Nvapi::SemPlacaNvidia => "Estes ajustes são só para placas NVIDIA, e ainda não cobrimos \
             placas AMD nem Intel. Não é problema no seu PC: é recurso que \
             ainda não chegou."
            .to_string(),
        // A `nvapi64.dll` só existe onde o driver da NVIDIA foi instalado: o que falhou tem conserto.
        Nvapi::NaoCarregou => "Encontrei uma placa NVIDIA, mas o driver não respondeu ao Otimiza. \
             Atualize o driver da NVIDIA pelo GeForce Experience ou pelo site \
             da NVIDIA e tente de novo. Nada foi alterado."
            .to_string(),
    }
}

/// PURA: prova os três casos sem placa (a esteira não tem GPU). Sem o arquivo é sem placa NVIDIA; com o arquivo e
/// sem resposta é driver quebrado ou velho.
pub fn classificar(dll_presente: bool, api_respondeu: bool, placas: u32) -> Nvapi {
    if !dll_presente {
        return Nvapi::SemPlacaNvidia;
    }

    if !api_respondeu {
        return Nvapi::NaoCarregou;
    }

    // Driver deixado para trás depois de a placa sair da máquina.
    if placas == 0 {
        return Nvapi::SemPlacaNvidia;
    }

    Nvapi::Disponivel
}

#[derive(Debug, Clone, Copy)]
pub struct Opcao {
    pub id: &'static str,
    pub titulo: &'static str,
    pub explicacao: &'static str,
    /// `Option` porque decide se a opção pode existir: sem restauração, `None`, e
    /// `toda_opcao_conhecida_tem_como_restaurar_o_padrao` a recusa.
    pub id_do_padrao: Option<u32>,
    /// A trava contra número errado na tabela (ver o cabeçalho).
    pub nome_esperado: &'static str,
    pub valor_otimizado: u32,
    /// "Desempenho máximo" global mantém a placa acordada na área de trabalho; V-Sync desligado global quebra o
    /// G-SYNC/VRR de todo jogo. Só por jogo (2.9), e o V-Sync nem lá sozinho.
    pub so_por_jogo: bool,
}

/// Números e valores do `NvApiDriverSettings.h` público, conferidos contra o nome do driver antes de escrever.
pub static OPCOES: &[Opcao] = &[
    Opcao {
        id: "energia",
        titulo: "Gerenciamento de energia: preferir desempenho máximo",
        explicacao: "Impede a placa de baixar o clock entre um quadro e outro. \
                     Gasta mais energia e esquenta um pouco mais; em troca some \
                     o engasgo de quando a placa demora a acordar.",
        id_do_padrao: Some(0x1057_EB71),
        nome_esperado: "power management",
        // PREFERRED_PSTATE_PREFER_MAX
        valor_otimizado: 0x0000_0001,
        so_por_jogo: true,
    },
    Opcao {
        id: "latencia",
        titulo: "Modo de baixa latência: um quadro pré-renderizado",
        explicacao: "Encurta a fila de quadros que o processador prepara adiantado. \
                     Menos atraso entre o movimento do mouse e a tela; em jogo mal \
                     otimizado pode custar alguns quadros por segundo.",
        id_do_padrao: Some(0x007B_A09E),
        nome_esperado: "pre-rendered",
        valor_otimizado: 0x0000_0001,
        so_por_jogo: false,
    },
    Opcao {
        id: "textura",
        titulo: "Filtragem de textura: desempenho",
        explicacao: "Deixa a placa caprichar menos no filtro das texturas. Rende \
                     quadros e, em movimento, quase não dá para ver a diferença.",
        id_do_padrao: Some(0x00CE_2691),
        nome_esperado: "texture filtering",
        // QUALITY_ENHANCEMENTS_PERFORMANCE (0x0A). "Alto desempenho" é 0x14, que piora mais a imagem: o título diz o que
        // o valor faz.
        valor_otimizado: 0x0000_000A,
        so_por_jogo: false,
    },
    Opcao {
        id: "vsync",
        titulo: "Sincronização vertical: desligada",
        explicacao: "Solta os quadros assim que ficam prontos, sem esperar o monitor. \
                     Derruba bastante o atraso; em troca a imagem pode partir ao \
                     meio em cena de movimento rápido.",
        id_do_padrao: Some(0x00A8_79CF),
        nome_esperado: "vertical sync",
        // VSYNCMODE_FORCEOFF
        valor_otimizado: 0x0841_6747,
        so_por_jogo: true,
    },
    Opcao {
        id: "cache_shader",
        titulo: "Cache de shader: ligado",
        explicacao: "Guarda em disco os shaders já compilados. Corta o engasgo dos \
                     primeiros minutos de jogo, que é o mais visível de todos.",
        id_do_padrao: Some(0x0019_8FFF),
        nome_esperado: "shader cache",
        // PS_SHADERDISKCACHE_ON
        valor_otimizado: 0x0000_0001,
        so_por_jogo: false,
    },
    Opcao {
        id: "cache_shader_tamanho",
        titulo: "Tamanho do cache de shader: sem limite",
        explicacao: "Com o cache cheio, o driver apaga shaders antigos e o jogo volta a compilar e engasgar onde já não engasgava. Sem limite, o cache ocupa mais espaço no disco do sistema.",
        id_do_padrao: Some(0x00AC_8497),
        nome_esperado: "shader disk cache maximum size",
        // PS_SHADERDISKCACHE_MAX_SIZE_MAX
        valor_otimizado: 0xFFFF_FFFF,
        so_por_jogo: false,
    },
];

/// `None` para nome desconhecido: o primeiro portão de toda escrita.
pub fn opcao_por_id(id: &str) -> Option<&'static Opcao> {
    OPCOES.iter().find(|opcao| opcao.id == id)
}

/// Fora de `OPCOES`: aquela lista vai para o perfil global, e um limite global prenderia área de trabalho,
/// navegador e todo jogo. `FRL_FPS_ID`: 0 a 1023, 0 é desligado (padrão de fábrica).
pub static LIMITADOR: Opcao = Opcao {
    id: "limitador",
    titulo: "Limite de quadros por segundo",
    explicacao: "Segura o jogo num número fixo de quadros por segundo. Baixa o FPS \
                 médio por definição — o Otimiza nunca aplica sozinho; serve para quem \
                 quer a placa mais fria ou um número estável.",
    id_do_padrao: Some(0x1083_5002),
    nome_esperado: "frame rate limiter",
    valor_otimizado: 0,
    so_por_jogo: true,
};

/// `FRL_FPS_MAX` no `NvApiDriverSettings.h`.
pub const LIMITE_MAXIMO: u32 = 1023;

/// Mora aqui e a tela recebe pronto: formato mudado de um lado só faria o "Desfazer" procurar id inexistente.
pub fn id_no_historico(opcao: &str) -> String {
    format!("driver_nvidia:{}", opcao)
}

#[derive(Debug, Clone, Serialize)]
pub struct AjusteNaTela {
    pub id: &'static str,
    pub titulo: &'static str,
    pub explicacao: &'static str,
    pub historico: String,
    /// Se o OTIMIZA aplicou e não desfez, não leitura do driver: o que o cliente pôs à mão aparece como não aplicado,
    /// e o "Aplicar" guarda o valor dele.
    pub aplicado: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LimiteNaTela {
    pub executavel: String,
    pub fps: u32,
    pub historico: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PainelDoDriver {
    pub estado: Nvapi,
    pub nota: String,
    pub ajustes: Vec<AjusteNaTela>,
    pub limites: Vec<LimiteNaTela>,
}

/// Só leitura: carrega a DLL e conta as placas, sem sessão da DRS.
pub fn painel(aplicado: impl Fn(&str) -> bool, limites: Vec<LimiteNaTela>) -> PainelDoDriver {
    let estado = estado();

    PainelDoDriver {
        estado,
        nota: nota_do_estado(estado),
        limites,
        ajustes: OPCOES
            .iter()
            .filter(|opcao| !opcao.so_por_jogo || aplicado(&id_no_historico(opcao.id)))
            .map(|opcao| {
                let historico = id_no_historico(opcao.id);
                AjusteNaTela {
                    id: opcao.id,
                    titulo: opcao.titulo,
                    explicacao: opcao.explicacao,
                    aplicado: aplicado(&historico),
                    historico,
                }
            })
            .collect(),
    }
}

pub const ANTERIOR_PADRAO: &str = "padrao";

/// PURA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanoDeDesfazer {
    RestaurarPadrao,
    /// O cliente tinha outro valor: devolver o padrão APAGARIA a escolha dele.
    Escrever(u32),
}

pub fn codificar_anterior(era_o_padrao: bool, valor: u32) -> String {
    if era_o_padrao {
        ANTERIOR_PADRAO.to_string()
    } else {
        valor.to_string()
    }
}

/// Texto ilegível cai em `RestaurarPadrao`: o padrão é estado conhecido, e deixar como o Otimiza pôs seria um
/// desfazer que não desfaz.
pub fn plano_de_desfazer(valor_anterior: &str) -> PlanoDeDesfazer {
    match valor_anterior.trim().parse::<u32>() {
        Ok(valor) => PlanoDeDesfazer::Escrever(valor),
        Err(_) => PlanoDeDesfazer::RestaurarPadrao,
    }
}

pub fn registro(opcao: &str, valor_anterior: String) -> ChangeRecord {
    ChangeRecord::DriverNvidia {
        opcao: opcao.to_string(),
        valor_anterior,
    }
}

// A NVAPI exporta só `nvapi_QueryInterface`, que traduz cada número no ponteiro da função (a forma documentada).
const ID_INITIALIZE: u32 = 0x0150_E828;
const ID_ENUM_PHYSICAL_GPUS: u32 = 0xE5AC_921F;
const ID_DRS_CREATE_SESSION: u32 = 0x0694_D52E;
const ID_DRS_DESTROY_SESSION: u32 = 0xDAD9_CFF8;
const ID_DRS_LOAD_SETTINGS: u32 = 0x375D_BD6B;
const ID_DRS_SAVE_SETTINGS: u32 = 0xFCBC_7E14;
const ID_DRS_GET_BASE_PROFILE: u32 = 0xDA84_66A0;
const ID_DRS_SET_SETTING: u32 = 0x577D_D202;
const ID_DRS_GET_SETTING: u32 = 0x73BF_8338;
const ID_DRS_RESTORE_DEFAULT: u32 = 0x53F0_381E;
const ID_DRS_GET_SETTING_NAME: u32 = 0xD61C_BE6E;

// Do perfil por jogo: conferidos em três tabelas independentes (wiki do nvapi.net, lista do jNizM, `FunctionId.cs`
// do NvAPIWrapper). Não há portão de nome para função: um número errado chamaria OUTRA função.
const ID_DRS_FIND_PROFILE_BY_NAME: u32 = 0x7E4A_9A0B;
const ID_DRS_CREATE_PROFILE: u32 = 0xCC17_6068;
const ID_DRS_DELETE_PROFILE: u32 = 0x1709_3206;
const ID_DRS_FIND_APPLICATION_BY_NAME: u32 = 0xEEE5_66B2;
const ID_DRS_CREATE_APPLICATION: u32 = 0x4347_A9DE;

/// `NVAPI_OK`. Todo erro é negativo.
const NVAPI_OK: i32 = 0;

/// Escrever num ajuste de outro tipo como se fosse número é a segunda forma de estragar o driver.
const NVDRS_DWORD_TYPE: u32 = 0;

/// União em C; os quatro primeiros bytes são o número, e todos os ajustes do catálogo são número.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct ValorNvdrs {
    numero: u32,
    /// Existe para o tamanho da estrutura bater; nunca é lido.
    _resto: [u8; 4096],
}

/// O `NVDRS_SETTING` v1. O tamanho vai nos 16 bits de baixo de `versao` e o driver RECUSA se não bater: um
/// desalinhamento vira código de erro, não corrupção. Há teste do tamanho.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct NvdrsSetting {
    versao: u32,
    nome: [u16; 2048],
    id: u32,
    tipo: u32,
    local: u32,
    e_predefinido_agora: u32,
    predefinido_valido: u32,
    valor_predefinido: ValorNvdrs,
    valor_atual: ValorNvdrs,
}

const TAMANHO_NVDRS_SETTING: usize = 12320;

const NVDRS_SETTING_VER1: u32 = (TAMANHO_NVDRS_SETTING as u32) | (1 << 16);

impl NvdrsSetting {
    fn zerada() -> Self {
        // Campo não zerado faria o driver ler lixo como pedido.
        let mut ajuste: NvdrsSetting = unsafe { std::mem::zeroed() };
        ajuste.versao = NVDRS_SETTING_VER1;
        ajuste
    }
}

/// O `NVDRS_APPLICATION_V3`, na ordem da documentação: versão, predefinido, quatro `NvAPI_UnicodeString` (2048
/// UTF-16) e um `NvU32` de bits. O tamanho vai na versão, com teste.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct NvdrsApplicationV3 {
    versao: u32,
    e_predefinido: u32,
    nome_do_app: [u16; 2048],
    nome_amigavel: [u16; 2048],
    lancador: [u16; 2048],
    arquivo_na_pasta: [u16; 2048],
    bits: u32,
}

const TAMANHO_NVDRS_APPLICATION_V3: usize = 16396;

const NVDRS_APPLICATION_VER_V3: u32 = (TAMANHO_NVDRS_APPLICATION_V3 as u32) | (3 << 16);

impl NvdrsApplicationV3 {
    fn zerada() -> Self {
        let mut aplicativo: NvdrsApplicationV3 = unsafe { std::mem::zeroed() };
        aplicativo.versao = NVDRS_APPLICATION_VER_V3;
        aplicativo
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct NvdrsProfileV1 {
    versao: u32,
    nome: [u16; 2048],
    suporte_de_gpu: u32,
    e_predefinido: u32,
    quantos_apps: u32,
    quantos_ajustes: u32,
}

const TAMANHO_NVDRS_PROFILE_V1: usize = 4116;

const NVDRS_PROFILE_VER1: u32 = (TAMANHO_NVDRS_PROFILE_V1 as u32) | (1 << 16);

impl NvdrsProfileV1 {
    fn zerado() -> Self {
        let mut perfil: NvdrsProfileV1 = unsafe { std::mem::zeroed() };
        perfil.versao = NVDRS_PROFILE_VER1;
        perfil
    }
}

/// 2048 unidades UTF-16 com o zero final. `None` quando não cabe: cortar amarraria o limite a outro arquivo.
fn utf16_fixo(texto: &str) -> Option<[u16; 2048]> {
    let unidades: Vec<u16> = texto.encode_utf16().collect();

    if unidades.len() >= 2048 {
        return None;
    }

    let mut fixo = [0u16; 2048];
    fixo[..unidades.len()].copy_from_slice(&unidades);
    Some(fixo)
}

type FnInit = unsafe extern "C" fn() -> i32;
type FnEnumGpus = unsafe extern "C" fn(*mut *mut c_void, *mut u32) -> i32;
type FnSessao = unsafe extern "C" fn(*mut *mut c_void) -> i32;
type FnComSessao = unsafe extern "C" fn(*mut c_void) -> i32;
type FnPerfilBase = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;
type FnGetSetting = unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut NvdrsSetting) -> i32;
type FnSetSetting = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut NvdrsSetting) -> i32;
type FnRestaurar = unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32;
type FnNomeDoId = unsafe extern "C" fn(u32, *mut [u16; 2048]) -> i32;
type FnAcharApp =
    unsafe extern "C" fn(*mut c_void, *const u16, *mut *mut c_void, *mut NvdrsApplicationV3) -> i32;
type FnAcharPerfil = unsafe extern "C" fn(*mut c_void, *const u16, *mut *mut c_void) -> i32;
type FnCriarPerfil = unsafe extern "C" fn(*mut c_void, *mut NvdrsProfileV1, *mut *mut c_void) -> i32;
type FnCriarApp = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut NvdrsApplicationV3) -> i32;
type FnApagarPerfil = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;

/// À parte e `Option`: driver sem elas continua servindo os ajustes globais.
struct ApiDePerfil {
    achar_app: FnAcharApp,
    achar_perfil: FnAcharPerfil,
    criar_perfil: FnCriarPerfil,
    criar_app: FnCriarApp,
    apagar_perfil: FnApagarPerfil,
}

struct Api {
    enum_gpus: FnEnumGpus,
    criar_sessao: FnSessao,
    destruir_sessao: FnComSessao,
    carregar_ajustes: FnComSessao,
    salvar_ajustes: FnComSessao,
    perfil_base: FnPerfilBase,
    ler_ajuste: FnGetSetting,
    escrever_ajuste: FnSetSetting,
    restaurar_ajuste: FnRestaurar,
    /// Sem ela não há como conferir os números: NENHUMA escrita acontece.
    nome_do_id: Option<FnNomeDoId>,
    perfis: Option<ApiDePerfil>,
}

/// A DLL NÃO é descarregada: o driver guarda estado desde o `NvAPI_Initialize`.
fn carregar_api() -> Result<Api, Nvapi> {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let nome: Vec<u16> = "nvapi64.dll\0".encode_utf16().collect();
    let modulo = unsafe { LoadLibraryW(nome.as_ptr()) };

    // Sem o arquivo não há driver NVIDIA: é o cliente com AMD, não erro.
    if modulo.is_null() {
        return Err(Nvapi::SemPlacaNvidia);
    }

    let consulta = unsafe { GetProcAddress(modulo, c"nvapi_QueryInterface".as_ptr() as *const u8) }
        .ok_or(Nvapi::NaoCarregou)?;

    let consulta: unsafe extern "C" fn(u32) -> *mut c_void =
        unsafe { std::mem::transmute(consulta) };

    let buscar = |id: u32| -> Option<*mut c_void> {
        let ponteiro = unsafe { consulta(id) };
        if ponteiro.is_null() {
            None
        } else {
            Some(ponteiro)
        }
    };

    let inicializar: FnInit =
        unsafe { std::mem::transmute(buscar(ID_INITIALIZE).ok_or(Nvapi::NaoCarregou)?) };

    if unsafe { inicializar() } != NVAPI_OK {
        return Err(Nvapi::NaoCarregou);
    }

    Ok(Api {
        enum_gpus: unsafe {
            std::mem::transmute(buscar(ID_ENUM_PHYSICAL_GPUS).ok_or(Nvapi::NaoCarregou)?)
        },
        criar_sessao: unsafe {
            std::mem::transmute(buscar(ID_DRS_CREATE_SESSION).ok_or(Nvapi::NaoCarregou)?)
        },
        destruir_sessao: unsafe {
            std::mem::transmute(buscar(ID_DRS_DESTROY_SESSION).ok_or(Nvapi::NaoCarregou)?)
        },
        carregar_ajustes: unsafe {
            std::mem::transmute(buscar(ID_DRS_LOAD_SETTINGS).ok_or(Nvapi::NaoCarregou)?)
        },
        salvar_ajustes: unsafe {
            std::mem::transmute(buscar(ID_DRS_SAVE_SETTINGS).ok_or(Nvapi::NaoCarregou)?)
        },
        perfil_base: unsafe {
            std::mem::transmute(buscar(ID_DRS_GET_BASE_PROFILE).ok_or(Nvapi::NaoCarregou)?)
        },
        ler_ajuste: unsafe {
            std::mem::transmute(buscar(ID_DRS_GET_SETTING).ok_or(Nvapi::NaoCarregou)?)
        },
        escrever_ajuste: unsafe {
            std::mem::transmute(buscar(ID_DRS_SET_SETTING).ok_or(Nvapi::NaoCarregou)?)
        },
        restaurar_ajuste: unsafe {
            std::mem::transmute(buscar(ID_DRS_RESTORE_DEFAULT).ok_or(Nvapi::NaoCarregou)?)
        },
        nome_do_id: buscar(ID_DRS_GET_SETTING_NAME)
            .map(|ponteiro| unsafe { std::mem::transmute::<*mut c_void, FnNomeDoId>(ponteiro) }),
        perfis: (|| {
            Some(ApiDePerfil {
                achar_app: unsafe {
                    std::mem::transmute::<*mut c_void, FnAcharApp>(buscar(
                        ID_DRS_FIND_APPLICATION_BY_NAME,
                    )?)
                },
                achar_perfil: unsafe {
                    std::mem::transmute::<*mut c_void, FnAcharPerfil>(buscar(
                        ID_DRS_FIND_PROFILE_BY_NAME,
                    )?)
                },
                criar_perfil: unsafe {
                    std::mem::transmute::<*mut c_void, FnCriarPerfil>(buscar(ID_DRS_CREATE_PROFILE)?)
                },
                criar_app: unsafe {
                    std::mem::transmute::<*mut c_void, FnCriarApp>(buscar(
                        ID_DRS_CREATE_APPLICATION,
                    )?)
                },
                apagar_perfil: unsafe {
                    std::mem::transmute::<*mut c_void, FnApagarPerfil>(buscar(ID_DRS_DELETE_PROFILE)?)
                },
            })
        })(),
    })
}

fn api() -> Result<&'static Api, Nvapi> {
    static CACHE: OnceLock<Result<Api, Nvapi>> = OnceLock::new();
    CACHE.get_or_init(carregar_api).as_ref().map_err(|erro| *erro)
}

pub fn estado() -> Nvapi {
    let api = match api() {
        Ok(api) => api,
        // O erro da carga é o veredito: DLL ausente `SemPlacaNvidia`, DLL muda `NaoCarregou`.
        Err(estado) => return estado,
    };

    let mut placas: [*mut c_void; 64] = [std::ptr::null_mut(); 64];
    let mut quantas: u32 = 0;
    let situacao = unsafe { (api.enum_gpus)(placas.as_mut_ptr(), &mut quantas) };

    classificar(true, situacao == NVAPI_OK, quantas)
}

fn nome_do_ajuste(api: &Api, id: u32) -> Option<String> {
    let funcao = api.nome_do_id?;
    let mut destino: [u16; 2048] = [0; 2048];

    if unsafe { funcao(id, &mut destino) } != NVAPI_OK {
        return None;
    }

    let fim = destino.iter().position(|c| *c == 0).unwrap_or(destino.len());
    Some(String::from_utf16_lossy(&destino[..fim]).to_lowercase())
}

/// PURA, para a trava ser provada sem placa. FECHA POR PADRÃO: sem conseguir perguntar, a resposta é NÃO.
fn veredito_do_nome(nome: Option<&str>, opcao: &Opcao, id: u32) -> Result<(), String> {
    match nome {
        Some(nome) if nome.contains(opcao.nome_esperado) => Ok(()),
        Some(nome) => Err(format!(
            "recusei mexer em \"{}\": o driver chama o ajuste {:#010X} de \"{}\", e eu esperava \
             algo com \"{}\". Escrever assim mudaria uma opção que você não pediu.",
            opcao.titulo, id, nome, opcao.nome_esperado
        )),
        None => Err(format!(
            "recusei mexer em \"{}\": este driver não soube me dizer o nome do ajuste {:#010X}, \
             e sem conferir isso eu não escrevo.",
            opcao.titulo, id
        )),
    }
}

fn conferir_o_numero(api: &Api, opcao: &Opcao, id: u32) -> Result<(), String> {
    veredito_do_nome(nome_do_ajuste(api, id).as_deref(), opcao, id)
}

fn na_sessao<T>(
    trabalho: impl FnOnce(&Api, *mut c_void, *mut c_void) -> Result<T, String>,
) -> Result<T, String> {
    na_sessao_da_drs(move |api, sessao| {
        let mut perfil: *mut c_void = std::ptr::null_mut();
        if unsafe { (api.perfil_base)(sessao, &mut perfil) } != NVAPI_OK {
            return Err("não consegui abrir o perfil global do driver da NVIDIA.".to_string());
        }

        trabalho(api, sessao, perfil)
    })
}

/// A sessão é sempre destruída, e trabalho que falha NÃO é salvo: criar perfil, ligar o executável e escrever o
/// limite nunca fica pela metade.
fn na_sessao_da_drs<T>(
    trabalho: impl FnOnce(&Api, *mut c_void) -> Result<T, String>,
) -> Result<T, String> {
    let api = api().map_err(nota_do_estado)?;

    let mut sessao: *mut c_void = std::ptr::null_mut();
    if unsafe { (api.criar_sessao)(&mut sessao) } != NVAPI_OK {
        return Err("não consegui abrir a configuração do driver da NVIDIA.".to_string());
    }

    let resultado = (|| {
        if unsafe { (api.carregar_ajustes)(sessao) } != NVAPI_OK {
            return Err("não consegui ler a configuração atual do driver da NVIDIA.".to_string());
        }

        let valor = trabalho(api, sessao)?;

        // Sem salvar, tudo some com a sessão, inclusive o desfazer, que "funcionaria" sem mudar nada.
        if unsafe { (api.salvar_ajustes)(sessao) } != NVAPI_OK {
            return Err(
                "mudei a configuração mas o driver não deixou salvar. Rode o Otimiza como \
                 administrador."
                    .to_string(),
            );
        }

        Ok(valor)
    })();

    unsafe { (api.destruir_sessao)(sessao) };
    resultado
}

fn na_sessao_de_leitura<T>(
    trabalho: impl FnOnce(&Api, *mut c_void, *mut c_void) -> Result<T, String>,
) -> Result<T, String> {
    let api = api().map_err(nota_do_estado)?;
    let mut sessao: *mut c_void = std::ptr::null_mut();
    if unsafe { (api.criar_sessao)(&mut sessao) } != NVAPI_OK {
        return Err("não consegui abrir a configuração do driver da NVIDIA.".to_string());
    }
    let resultado = (|| {
        if unsafe { (api.carregar_ajustes)(sessao) } != NVAPI_OK {
            return Err("não consegui ler a configuração atual do driver da NVIDIA.".to_string());
        }
        let mut perfil: *mut c_void = std::ptr::null_mut();
        if unsafe { (api.perfil_base)(sessao, &mut perfil) } != NVAPI_OK {
            return Err("não consegui abrir o perfil global do driver da NVIDIA.".to_string());
        }
        trabalho(api, sessao, perfil)
    })();
    unsafe { (api.destruir_sessao)(sessao) };
    resultado
}

/// Os que PRENDEM o FPS: ligado à força e intervalos de 2, 3 e 4. "Controlado pelo aplicativo" (0x60925292),
/// "desligado" e "rápido" não prendem.
pub fn vsync_prende(valor: u32) -> bool {
    const FORCEON: u32 = 0x4781_4940;
    const FLIPINTERVAL2: u32 = 0x3261_0244;
    const FLIPINTERVAL3: u32 = 0x7127_1021;
    const FLIPINTERVAL4: u32 = 0x1324_5256;
    matches!(valor, FORCEON | FLIPINTERVAL2 | FLIPINTERVAL3 | FLIPINTERVAL4)
}

#[derive(Debug, Clone, Serialize)]
pub struct TetosDoDriver {
    pub limite_global_fps: u32,
    pub vsync_forcado: bool,
}

pub fn tetos_no_perfil_global() -> Result<TetosDoDriver, String> {
    na_sessao_de_leitura(|api, sessao, perfil| {
        let limite_id = LIMITADOR.id_do_padrao.expect("o limitador tem número");
        let vsync_id = opcao_por_id("vsync").and_then(|o| o.id_do_padrao).expect("vsync tem número");
        let (_, limite) = ler_valor(api, sessao, perfil, limite_id);
        let (padrao_vsync, vsync) = ler_valor(api, sessao, perfil, vsync_id);
        Ok(TetosDoDriver {
            limite_global_fps: limite,
            vsync_forcado: !padrao_vsync && vsync_prende(vsync),
        })
    })
}

/// PURA, para ser provada sem placa. Ajuste ausente do perfil é ajuste que ninguém tocou: "está no padrão", não
/// "não sei". Senão o desfazer escreveria um zero que nunca existiu.
fn interpretar_leitura(leitura_deu_certo: bool, e_predefinido: u32, valor: u32) -> (bool, u32) {
    if !leitura_deu_certo {
        return (true, 0);
    }

    // `e_predefinido` é o driver dizendo que é o de fábrica: melhor que comparar números.
    (e_predefinido != 0, valor)
}

fn ler_valor(api: &Api, sessao: *mut c_void, perfil: *mut c_void, id: u32) -> (bool, u32) {
    let mut ajuste = NvdrsSetting::zerada();
    let situacao = unsafe { (api.ler_ajuste)(sessao, perfil, id, &mut ajuste) };

    interpretar_leitura(
        situacao == NVAPI_OK,
        ajuste.e_predefinido_agora,
        ajuste.valor_atual.numero,
    )
}

fn escrever_valor_cru(
    api: &Api,
    sessao: *mut c_void,
    perfil: *mut c_void,
    id: u32,
    valor: u32,
) -> Result<(), String> {
    let mut ajuste = NvdrsSetting::zerada();
    ajuste.id = id;
    ajuste.tipo = NVDRS_DWORD_TYPE;
    ajuste.valor_atual.numero = valor;

    if unsafe { (api.escrever_ajuste)(sessao, perfil, &mut ajuste) } != NVAPI_OK {
        return Err(format!(
            "o driver da NVIDIA recusou o ajuste {:#010X}. Nada foi alterado.",
            id
        ));
    }

    Ok(())
}

fn numero_de(opcao: &str) -> Result<&'static Opcao, String> {
    let alvo =
        opcao_por_id(opcao).ok_or_else(|| format!("não conheço o ajuste de driver \"{}\".", opcao))?;

    if alvo.id_do_padrao.is_none() {
        return Err(format!(
            "o ajuste \"{}\" não tem como voltar ao padrão, então o Otimiza não o aplica.",
            alvo.titulo
        ));
    }

    Ok(alvo)
}

/// Lê, guarda o anterior, e só então escreve.
pub fn aplicar(opcao: &str) -> Result<String, String> {
    let alvo = numero_de(opcao)?;
    if alvo.so_por_jogo {
        return Err(format!(
            "\"{}\" não é mais aplicado para todos os jogos: use o perfil NVIDIA do jogo, na ficha dele na Biblioteca.",
            alvo.titulo
        ));
    }
    let id = alvo.id_do_padrao.expect("numero_de já garantiu");

    na_sessao(move |api, sessao, perfil| {
        conferir_o_numero(api, alvo, id)?;

        let (era_o_padrao, anterior) = ler_valor(api, sessao, perfil, id);
        escrever_valor_cru(api, sessao, perfil, id, alvo.valor_otimizado)?;

        Ok(codificar_anterior(era_o_padrao, anterior))
    })
}

/// A própria NVIDIA diz qual é o padrão e volta para ele.
pub fn restaurar_padrao(opcao: &str) -> Result<(), String> {
    let alvo = numero_de(opcao)?;
    let id = alvo.id_do_padrao.expect("numero_de já garantiu");

    na_sessao(move |api, sessao, perfil| {
        conferir_o_numero(api, alvo, id)?;

        if unsafe { (api.restaurar_ajuste)(sessao, perfil, id) } != NVAPI_OK {
            return Err(format!(
                "não consegui devolver \"{}\" ao padrão do driver.",
                alvo.titulo
            ));
        }

        Ok(())
    })
}

/// Chamado por `revert_changes()`. Anterior de fábrica: restaura a NVIDIA; escolha do cliente: volta a DELE.
pub fn desfazer(opcao: &str, valor_anterior: &str) -> Result<(), String> {
    match plano_de_desfazer(valor_anterior) {
        PlanoDeDesfazer::RestaurarPadrao => restaurar_padrao(opcao),
        PlanoDeDesfazer::Escrever(valor) => {
            let alvo = numero_de(opcao)?;
            let id = alvo.id_do_padrao.expect("numero_de já garantiu");

            na_sessao(move |api, sessao, perfil| {
                conferir_o_numero(api, alvo, id)?;
                escrever_valor_cru(api, sessao, perfil, id, valor)
            })
        }
    }
}

/// Em minúsculas: dois registros para o mesmo jogo deixariam um desfazer sem par.
pub fn id_do_limite(executavel: &str) -> String {
    format!("limite_nvidia:{}", executavel.to_lowercase())
}

/// É por este nome que o desfazer acha o perfil: nenhum outro perfil é apagado.
pub fn nome_do_perfil_do_otimiza(executavel: &str) -> String {
    format!("Otimiza - {}", executavel)
}

/// Caminho, pasta ou nome sem `.exe` são recusados antes de qualquer chamada ao driver.
pub fn executavel_valido(nome: &str) -> Result<(), String> {
    let valido = !nome.is_empty()
        && nome.trim() == nome
        && nome.chars().count() <= 260
        && nome.to_lowercase().ends_with(".exe")
        && !nome.contains(['\\', '/', ':', '"', '*', '?', '<', '>', '|']);

    if valido {
        Ok(())
    } else {
        Err(format!(
            "\"{}\" não é o nome do arquivo de um jogo. O limite fica amarrado ao nome do \
             executável, como FiveM_GTAProcess.exe.",
            nome
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimiteAplicado {
    /// O desfazer apaga esse perfil inteiro.
    pub perfil_criado: bool,
    pub valor_anterior: String,
}

fn perfil_do_executavel(
    perfis: &ApiDePerfil,
    sessao: *mut c_void,
    nome_do_app: &[u16; 2048],
) -> Option<*mut c_void> {
    let mut perfil: *mut c_void = std::ptr::null_mut();
    let mut aplicativo = NvdrsApplicationV3::zerada();

    let achou = unsafe {
        (perfis.achar_app)(sessao, nome_do_app.as_ptr(), &mut perfil, &mut aplicativo)
    } == NVAPI_OK;

    (achou && !perfil.is_null()).then_some(perfil)
}

/// No perfil já amarrado ao executável (guardando o anterior) ou num perfil novo com o nome do Otimiza. Falha no
/// meio desfaz tudo (sessão não salva). Os portões vêm antes de abrir a sessão.
pub fn aplicar_limite(executavel: &str, fps: u32) -> Result<LimiteAplicado, String> {
    executavel_valido(executavel)?;

    if fps == 0 || fps > LIMITE_MAXIMO {
        return Err(format!(
            "o limite precisa ficar entre 1 e {} quadros por segundo.",
            LIMITE_MAXIMO
        ));
    }

    let id = LIMITADOR.id_do_padrao.expect("o limitador tem número");
    let nome_do_app = utf16_fixo(executavel)
        .ok_or_else(|| "o nome do executável é longo demais para o driver.".to_string())?;
    let nome_do_perfil = utf16_fixo(&nome_do_perfil_do_otimiza(executavel))
        .ok_or_else(|| "o nome do executável é longo demais para o driver.".to_string())?;

    na_sessao_da_drs(move |api, sessao| {
        conferir_o_numero(api, &LIMITADOR, id)?;

        let perfis = api.perfis.as_ref().ok_or_else(|| {
            "este driver não oferece perfil por jogo, então o limite não foi aplicado.".to_string()
        })?;

        let (perfil, perfil_criado) = match perfil_do_executavel(perfis, sessao, &nome_do_app) {
            Some(perfil) => (perfil, false),
            None => {
                let mut dados = NvdrsProfileV1::zerado();
                dados.nome = nome_do_perfil;

                let mut perfil: *mut c_void = std::ptr::null_mut();
                if unsafe { (perfis.criar_perfil)(sessao, &mut dados, &mut perfil) } != NVAPI_OK {
                    return Err(
                        "o driver da NVIDIA não deixou criar o perfil deste jogo. Nada foi alterado."
                            .to_string(),
                    );
                }

                let mut aplicativo = NvdrsApplicationV3::zerada();
                aplicativo.nome_do_app = nome_do_app;

                if unsafe { (perfis.criar_app)(sessao, perfil, &mut aplicativo) } != NVAPI_OK {
                    return Err(
                        "o driver da NVIDIA não deixou ligar o jogo ao perfil. Nada foi alterado."
                            .to_string(),
                    );
                }

                (perfil, true)
            }
        };

        let (era_o_padrao, anterior) = ler_valor(api, sessao, perfil, id);
        escrever_valor_cru(api, sessao, perfil, id, fps)?;

        Ok(LimiteAplicado {
            perfil_criado,
            valor_anterior: codificar_anterior(era_o_padrao, anterior),
        })
    })
}

// Aplicar saiu na 3.0 com a Biblioteca; o desfazer fica para quem aplicou antes.

pub fn desfazer_perfil_do_jogo(executavel: &str, perfil_criado: bool, anteriores: &[(String, String)]) -> Result<(), String> {
    executavel_valido(executavel)?;
    let nome_do_app = utf16_fixo(executavel).ok_or_else(|| "o nome do executável é longo demais para o driver.".to_string())?;
    let nome_do_perfil = utf16_fixo(&nome_do_perfil_do_otimiza(executavel))
        .ok_or_else(|| "o nome do executável é longo demais para o driver.".to_string())?;
    let anteriores = anteriores.to_vec();
    na_sessao_da_drs(move |api, sessao| {
        let perfis = api.perfis.as_ref().ok_or_else(|| "este driver não oferece perfil por jogo.".to_string())?;
        if perfil_criado {
            let mut perfil: *mut c_void = std::ptr::null_mut();
            let achou = unsafe { (perfis.achar_perfil)(sessao, nome_do_perfil.as_ptr(), &mut perfil) } == NVAPI_OK;
            if achou && !perfil.is_null() && unsafe { (perfis.apagar_perfil)(sessao, perfil) } != NVAPI_OK {
                return Err(format!("não consegui apagar o perfil \"{}\" do driver da NVIDIA.", nome_do_perfil_do_otimiza(executavel)));
            }
            return Ok(());
        }
        let Some(perfil) = perfil_do_executavel(perfis, sessao, &nome_do_app) else { return Ok(()) };
        for (opcao, anterior) in &anteriores {
            let o = numero_de(opcao)?;
            let id = o.id_do_padrao.expect("numero_de já garantiu");
            match plano_de_desfazer(anterior) {
                PlanoDeDesfazer::RestaurarPadrao => {
                    if unsafe { (api.restaurar_ajuste)(sessao, perfil, id) } != NVAPI_OK {
                        return Err(format!("não consegui devolver \"{}\" ao padrão neste jogo.", o.titulo));
                    }
                }
                PlanoDeDesfazer::Escrever(valor) => escrever_valor_cru(api, sessao, perfil, id, valor)?,
            }
        }
        Ok(())
    })
}

/// Perfil do Otimiza é achado PELO NOME e apagado; perfil que já existia tem só o limite devolvido. Perfil que
/// sumiu por fora não é falha: o estado desejado já é o atual.
pub fn desfazer_limite(
    executavel: &str,
    perfil_criado: bool,
    valor_anterior: &str,
) -> Result<(), String> {
    executavel_valido(executavel)?;

    let id = LIMITADOR.id_do_padrao.expect("o limitador tem número");
    let nome_do_app = utf16_fixo(executavel)
        .ok_or_else(|| "o nome do executável é longo demais para o driver.".to_string())?;
    let nome_do_perfil = utf16_fixo(&nome_do_perfil_do_otimiza(executavel))
        .ok_or_else(|| "o nome do executável é longo demais para o driver.".to_string())?;
    let plano = plano_de_desfazer(valor_anterior);

    na_sessao_da_drs(move |api, sessao| {
        conferir_o_numero(api, &LIMITADOR, id)?;

        let perfis = api.perfis.as_ref().ok_or_else(|| {
            "este driver não oferece perfil por jogo, então não consegui desfazer o limite."
                .to_string()
        })?;

        if perfil_criado {
            let mut perfil: *mut c_void = std::ptr::null_mut();
            let achou = unsafe {
                (perfis.achar_perfil)(sessao, nome_do_perfil.as_ptr(), &mut perfil)
            } == NVAPI_OK;

            if !achou || perfil.is_null() {
                return Ok(());
            }

            if unsafe { (perfis.apagar_perfil)(sessao, perfil) } != NVAPI_OK {
                return Err(format!(
                    "não consegui apagar o perfil \"{}\" do driver da NVIDIA.",
                    nome_do_perfil_do_otimiza(executavel)
                ));
            }

            return Ok(());
        }

        let Some(perfil) = perfil_do_executavel(perfis, sessao, &nome_do_app) else {
            return Ok(());
        };

        match plano {
            PlanoDeDesfazer::RestaurarPadrao => {
                if unsafe { (api.restaurar_ajuste)(sessao, perfil, id) } != NVAPI_OK {
                    return Err(format!(
                        "não consegui devolver o limite de \"{}\" ao padrão do driver.",
                        executavel
                    ));
                }
                Ok(())
            }
            PlanoDeDesfazer::Escrever(valor) => escrever_valor_cru(api, sessao, perfil, id, valor),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_nvapi_o_produto_diz_que_nao_cobre_em_vez_de_tela_vazia() {
        let nota = nota_do_estado(Nvapi::SemPlacaNvidia);

        assert!(
            nota.to_lowercase().contains("amd") || nota.to_lowercase().contains("não cobrimos"),
            "a tela precisa dizer que a placa nao e coberta: {}",
            nota
        );
        assert!(!nota.is_empty());
    }

    #[test]
    fn nao_carregar_a_dll_e_diferente_de_nao_ter_placa_nvidia() {
        assert_ne!(
            nota_do_estado(Nvapi::NaoCarregou),
            nota_do_estado(Nvapi::SemPlacaNvidia)
        );
        assert!(nota_do_estado(Nvapi::NaoCarregou)
            .to_lowercase()
            .contains("driver"));
    }

    #[test]
    fn toda_opcao_conhecida_tem_como_restaurar_o_padrao() {
        for opcao in OPCOES {
            assert!(
                opcao.id_do_padrao.is_some(),
                "{} nao tem como voltar ao padrao e nao pode entrar",
                opcao.id
            );
        }
    }

    #[test]
    fn a_nota_do_estado_disponivel_tambem_fala_alguma_coisa() {
        let disponivel = nota_do_estado(Nvapi::Disponivel);
        assert!(!disponivel.trim().is_empty());
        assert_ne!(disponivel, nota_do_estado(Nvapi::SemPlacaNvidia));
        assert_ne!(disponivel, nota_do_estado(Nvapi::NaoCarregou));

        assert!(!nota_do_estado(Nvapi::SemPlacaNvidia)
            .to_lowercase()
            .contains("driver"));
    }

    #[test]
    fn dll_ausente_e_placa_amd_e_nao_driver_quebrado() {
        assert_eq!(classificar(false, false, 0), Nvapi::SemPlacaNvidia);
        assert_eq!(
            classificar(false, true, 4),
            Nvapi::SemPlacaNvidia,
            "sem o arquivo nada mais importa"
        );
    }

    #[test]
    fn dll_presente_que_nao_responde_e_problema_de_driver() {
        assert_eq!(classificar(true, false, 0), Nvapi::NaoCarregou);
        assert_eq!(classificar(true, false, 1), Nvapi::NaoCarregou);
    }

    #[test]
    fn driver_falando_mas_sem_nenhuma_placa_nao_e_disponivel() {
        assert_eq!(classificar(true, true, 0), Nvapi::SemPlacaNvidia);
        assert_eq!(classificar(true, true, 1), Nvapi::Disponivel);
        assert_eq!(classificar(true, true, 3), Nvapi::Disponivel);
    }

    #[test]
    fn o_catalogo_tem_as_seis_e_nenhuma_repetida() {
        assert_eq!(OPCOES.len(), 6, "o pilar prometeu seis ajustes");

        for opcao in OPCOES {
            assert!(!opcao.id.trim().is_empty());
            assert!(!opcao.titulo.trim().is_empty(), "{}", opcao.id);
            assert!(!opcao.explicacao.trim().is_empty(), "{}", opcao.id);
            assert!(!opcao.nome_esperado.trim().is_empty(), "{}", opcao.id);
            assert_eq!(
                opcao.nome_esperado.to_lowercase(),
                opcao.nome_esperado,
                "{}: o nome esperado e comparado em minusculas",
                opcao.id
            );
            assert!(opcao_por_id(opcao.id).is_some(), "{}", opcao.id);
        }

        // Mesmo número em dois ajustes: um "desfazer" desfaria o do outro.
        for (i, a) in OPCOES.iter().enumerate() {
            for b in OPCOES.iter().skip(i + 1) {
                assert_ne!(a.id, b.id, "identificador repetido: {}", a.id);
                assert_ne!(
                    a.id_do_padrao, b.id_do_padrao,
                    "{} e {} apontam para o mesmo ajuste do driver",
                    a.id, b.id
                );
            }
        }
    }

    #[test]
    fn o_painel_marca_como_aplicado_so_o_que_esta_no_historico() {
        let vsync = id_no_historico("vsync");
        let painel = painel(|id| id == vsync, Vec::new());

        let esperados = OPCOES.iter().filter(|o| !o.so_por_jogo || o.id == "vsync").count();
        assert_eq!(painel.ajustes.len(), esperados);
        assert!(painel.ajustes.iter().all(|a| a.id != "energia"));
        for ajuste in &painel.ajustes {
            assert_eq!(ajuste.historico, id_no_historico(ajuste.id));
            assert_eq!(ajuste.aplicado, ajuste.id == "vsync", "{}", ajuste.id);
        }
        assert!(!painel.nota.trim().is_empty());
    }

    #[test]
    fn energia_e_vsync_nao_se_aplicam_mais_no_global() {
        for id in ["energia", "vsync"] {
            let erro = aplicar(id).expect_err(id);
            assert!(erro.contains("perfil NVIDIA do jogo"), "{}: {}", id, erro);
        }
    }

    #[test]
    fn as_estruturas_de_perfil_tem_o_tamanho_que_o_driver_espera() {
        assert_eq!(
            std::mem::size_of::<NvdrsApplicationV3>(),
            TAMANHO_NVDRS_APPLICATION_V3,
            "o layout da NVDRS_APPLICATION_V3 mudou"
        );
        assert_eq!(NVDRS_APPLICATION_VER_V3, 0x0003_400C);

        assert_eq!(
            std::mem::size_of::<NvdrsProfileV1>(),
            TAMANHO_NVDRS_PROFILE_V1,
            "o layout da NVDRS_PROFILE_V1 mudou"
        );
        assert_eq!(NVDRS_PROFILE_VER1, 0x0001_1014);

        let aplicativo = NvdrsApplicationV3::zerada();
        assert_eq!(aplicativo.versao, NVDRS_APPLICATION_VER_V3);
        assert_eq!(aplicativo.e_predefinido, 0);
        assert_eq!(aplicativo.nome_do_app[0], 0);
        assert_eq!(aplicativo.bits, 0);

        let perfil = NvdrsProfileV1::zerado();
        assert_eq!(perfil.versao, NVDRS_PROFILE_VER1);
        assert_eq!(perfil.nome[0], 0);
        assert_eq!(perfil.suporte_de_gpu, 0);
    }

    #[test]
    fn o_nome_vai_ao_driver_inteiro_ou_nao_vai() {
        let fixo = utf16_fixo("FiveM_GTAProcess.exe").expect("cabe");
        let fim = fixo.iter().position(|c| *c == 0).unwrap();
        assert_eq!(String::from_utf16_lossy(&fixo[..fim]), "FiveM_GTAProcess.exe");

        assert!(utf16_fixo(&"a".repeat(2048)).is_none());
        assert!(utf16_fixo(&"a".repeat(2047)).is_some());
    }

    #[test]
    fn so_nome_de_executavel_vira_perfil() {
        assert!(executavel_valido("FiveM_b3258_GTAProcess.exe").is_ok());
        assert!(executavel_valido("cs2.EXE").is_ok());

        assert!(executavel_valido("").is_err());
        assert!(executavel_valido("FiveM").is_err());
        assert!(executavel_valido(r"C:\Jogos\FiveM.exe").is_err());
        assert!(executavel_valido("pasta/jogo.exe").is_err());
        assert!(executavel_valido(" jogo.exe").is_err());

        assert_eq!(id_do_limite("FiveM.EXE"), id_do_limite("fivem.exe"));
        assert_eq!(nome_do_perfil_do_otimiza("cs2.exe"), "Otimiza - cs2.exe");
    }

    #[test]
    fn limite_invalido_nao_chega_perto_do_driver() {
        // A máquina do dono TEM NVIDIA: os portões precisam vir antes da sessão, ou o teste abriria a DRS.
        assert!(aplicar_limite("FiveM_GTAProcess.exe", 0).is_err());
        assert!(aplicar_limite("FiveM_GTAProcess.exe", LIMITE_MAXIMO + 1).is_err());
        assert!(aplicar_limite(r"C:\Jogos\FiveM.exe", 60).is_err());
        assert!(aplicar_limite("FiveM", 60).is_err());
        assert!(desfazer_limite("", true, ANTERIOR_PADRAO).is_err());
    }

    #[test]
    fn os_portoes_do_limitador_vem_antes_da_sessao() {
        // Prende a ORDEM: validação dentro da sessão deixaria um caso não testado chegar ao driver.
        let fonte = codigo_fonte_deste_arquivo();
        let corpo = fonte
            .split("pub fn aplicar_limite")
            .nth(1)
            .expect("aplicar_limite existe");

        let validacao = corpo.find("executavel_valido(executavel)?;").expect("valida o executável");
        let faixa = corpo.find("fps > LIMITE_MAXIMO").expect("valida a faixa");
        let sessao = corpo.find("na_sessao_da_drs(").expect("abre a sessão");

        assert!(validacao < sessao && faixa < sessao);
    }

    #[test]
    fn o_limitador_confere_o_numero_nos_dois_caminhos() {
        let fonte = codigo_fonte_deste_arquivo();

        assert_eq!(
            fonte.matches("conferir_o_numero(api, &LIMITADOR, id)?;").count(),
            2,
            "aplicar_limite e desfazer_limite precisam conferir o número do limitador antes de mexer"
        );
    }

    #[test]
    fn a_textura_grava_o_valor_que_o_titulo_diz() {
        let textura = opcao_por_id("textura").expect("o ajuste de textura existe");

        assert_eq!(textura.valor_otimizado, 0x0000_000A);
        assert!(
            !textura.titulo.to_lowercase().contains("alto"),
            "o título promete um nível que não é o gravado: {}",
            textura.titulo
        );
    }

    #[test]
    fn ajuste_desconhecido_nao_chega_perto_do_driver() {
        // Nenhum teste desta suíte pode escrever no driver (a máquina do dono tem NVIDIA).
        assert!(opcao_por_id("ajuste-que-nao-existe").is_none());

        let erro = numero_de("ajuste-que-nao-existe").unwrap_err();
        assert!(erro.contains("ajuste-que-nao-existe"), "{}", erro);

        assert!(aplicar("ajuste-que-nao-existe").is_err());
        assert!(restaurar_padrao("ajuste-que-nao-existe").is_err());
        assert!(desfazer("ajuste-que-nao-existe", ANTERIOR_PADRAO).is_err());
        assert!(desfazer("ajuste-que-nao-existe", "7").is_err());
    }

    #[test]
    fn o_valor_anterior_sobrevive_a_ida_e_volta_do_historico() {
        assert_eq!(codificar_anterior(true, 999), ANTERIOR_PADRAO);
        assert_eq!(codificar_anterior(false, 0x0841_6747), "138504007");

        assert_eq!(
            plano_de_desfazer(&codificar_anterior(true, 999)),
            PlanoDeDesfazer::RestaurarPadrao
        );
        assert_eq!(
            plano_de_desfazer(&codificar_anterior(false, 42)),
            PlanoDeDesfazer::Escrever(42)
        );

        // Zero NÃO é "padrão": é valor legítimo em vários ajustes.
        assert_eq!(
            plano_de_desfazer(&codificar_anterior(false, 0)),
            PlanoDeDesfazer::Escrever(0)
        );
    }

    #[test]
    fn historico_ilegivel_cai_no_padrao_do_driver_e_nao_no_silencio() {
        for lixo in [
            "",
            "   ",
            "padrao",
            "padrão",
            "-1",
            "abc",
            "3.5",
            "99999999999999",
        ] {
            assert_eq!(
                plano_de_desfazer(lixo),
                PlanoDeDesfazer::RestaurarPadrao,
                "entrada {:?}",
                lixo
            );
        }
    }

    #[test]
    fn o_registro_do_historico_leva_o_que_o_desfazer_precisa() {
        let gravado = registro("vsync", codificar_anterior(false, 7));

        match gravado {
            ChangeRecord::DriverNvidia {
                opcao,
                valor_anterior,
            } => {
                assert_eq!(opcao, "vsync");
                assert_eq!(valor_anterior, "7");
                assert_eq!(
                    plano_de_desfazer(&valor_anterior),
                    PlanoDeDesfazer::Escrever(7)
                );
            }
            outro => panic!("registro errado: {:?}", outro),
        }
    }

    #[test]
    fn a_estrutura_da_nvapi_tem_o_tamanho_que_o_driver_espera() {
        assert_eq!(
            std::mem::size_of::<NvdrsSetting>(),
            TAMANHO_NVDRS_SETTING,
            "o layout da NVDRS_SETTING mudou"
        );
        assert_eq!(std::mem::size_of::<ValorNvdrs>(), 4100);
        assert_eq!(NVDRS_SETTING_VER1, 0x0001_3020);

        let zerada = NvdrsSetting::zerada();
        assert_eq!(zerada.versao, NVDRS_SETTING_VER1);
        assert_eq!(zerada.id, 0);
        assert_eq!(zerada.tipo, 0);
        assert_eq!(zerada.local, 0);
        assert_eq!(zerada.valor_atual.numero, 0);
        assert_eq!(zerada.valor_predefinido.numero, 0);
        assert_eq!(zerada.e_predefinido_agora, 0);
        assert_eq!(zerada.predefinido_valido, 0);
        assert_eq!(zerada.nome[0], 0);
    }

    /// Não escreve: `estado()` só carrega, inicializa e conta as placas.
    #[test]
    fn le_o_estado_desta_maquina() {
        let estado = estado();
        println!("[{:?}] {}", estado, nota_do_estado(estado));

        assert!(!nota_do_estado(estado).trim().is_empty());
    }

    /// A PROVA DOS NÚMEROS: na máquina do dono, pergunta ao driver o nome de cada um. Sem NVIDIA (a esteira) passa sem
    /// afirmar nada: verificação de campo.
    #[test]
    fn os_numeros_da_tabela_batem_com_o_que_o_driver_chama() {
        let Ok(api) = api() else {
            println!("sem NVAPI nesta maquina — nada a conferir");
            return;
        };

        for opcao in OPCOES.iter().chain(std::iter::once(&LIMITADOR)) {
            let id = opcao.id_do_padrao.expect("o catalogo ja garante");
            match nome_do_ajuste(api, id) {
                Some(nome) => {
                    println!(
                        "{:#010X} -> {:?} (esperado conter {:?})",
                        id, nome, opcao.nome_esperado
                    );
                    assert!(
                        nome.contains(opcao.nome_esperado),
                        "{}: o driver chama {:#010X} de {:?}, e a tabela espera {:?}",
                        opcao.id,
                        id,
                        nome,
                        opcao.nome_esperado
                    );
                }
                // Número que o driver não conhece é número errado: sem isto o botão nunca funcionaria, calado.
                None => panic!(
                    "{}: este driver nao conhece o ajuste {:#010X} -- o numero da tabela \n                     esta errado, ou nao existe mais nesta versao do driver",
                    opcao.id, id
                ),
            }
        }
    }

    /// Só fora de `mod tests`, para o `.contains` não se achar.
    fn codigo_fonte_deste_arquivo() -> String {
        let caminho = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("modules")
            .join("windows")
            .join("nvdriver.rs");
        let fonte = std::fs::read_to_string(&caminho)
            .unwrap_or_else(|e| panic!("não consegui ler {:?}: {}", caminho, e));
        fonte
            .split_once("#[cfg(test)]")
            .map(|(codigo, _testes)| codigo.to_string())
            .unwrap_or(fonte)
    }

    // Trocar a chamada por `classificar(true, true, 1)` diria "Disponível" até com Radeon, com a suíte verde. O
    // canário fecha o fio sem injetar a NVAPI.
    #[test]
    fn estado_classifica_com_o_que_a_maquina_devolveu() {
        let fonte = codigo_fonte_deste_arquivo();

        assert!(
            fonte.contains("classificar(true, situacao == NVAPI_OK, quantas)"),
            "`estado()` precisa classificar com o resultado REAL da enumeracao de \
             placas -- qualquer constante no lugar faz o produto oferecer ajustes \
             de NVIDIA em maquina com Radeon"
        );
    }

    #[test]
    fn ajuste_ausente_do_perfil_conta_como_padrao_de_fabrica() {
        // O caso MAIS COMUM: perfil limpo, cliente que nunca abriu o painel da NVIDIA.
        assert_eq!(interpretar_leitura(false, 0, 0), (true, 0));
        assert_eq!(
            interpretar_leitura(false, 0, 12345),
            (true, 0),
            "leitura que falhou nao tem valor para aproveitar"
        );

        assert_eq!(interpretar_leitura(true, 1, 7), (true, 7));
        assert_eq!(interpretar_leitura(true, 0, 7), (false, 7));

        assert_eq!(interpretar_leitura(true, 0, 0), (false, 0));

        let (era_padrao, valor) = interpretar_leitura(true, 0, 0);
        assert_eq!(
            plano_de_desfazer(&codificar_anterior(era_padrao, valor)),
            PlanoDeDesfazer::Escrever(0)
        );
        let (era_padrao, valor) = interpretar_leitura(false, 0, 0);
        assert_eq!(
            plano_de_desfazer(&codificar_anterior(era_padrao, valor)),
            PlanoDeDesfazer::RestaurarPadrao
        );
    }

    // `interpretar_leitura(true, 1, 0)` ali faria tudo parecer padrão e o desfazer passaria por cima da escolha do
    // cliente; `ler_valor` exige sessão real.
    #[test]
    fn a_leitura_real_interpreta_o_que_a_nvapi_devolveu() {
        let fonte = codigo_fonte_deste_arquivo();

        assert!(
            fonte.contains("situacao == NVAPI_OK,")
                && fonte.contains("ajuste.e_predefinido_agora,")
                && fonte.contains("ajuste.valor_atual.numero,"),
            "`ler_valor` precisa interpretar os valores que a NVAPI escreveu na              estrutura, e nao constantes"
        );
    }

    #[test]
    fn o_portao_do_nome_recusa_o_que_nao_bate_e_o_que_nao_da_para_conferir() {
        let vsync = opcao_por_id("vsync").expect("o catalogo tem vsync");

        assert!(veredito_do_nome(Some("vertical sync"), vsync, 0x00A8_79CF).is_ok());

        // O caso perigoso: número errado apontando para OUTRA opção conhecida; a escrita daria certo, no lugar errado.
        let erro = veredito_do_nome(Some("shader cache"), vsync, 0x0019_8FFF)
            .expect_err("nome que nao bate precisa recusar");
        assert!(erro.contains("shader cache"), "{}", erro);
        assert!(erro.contains("vertical sync"), "{}", erro);

        let erro = veredito_do_nome(None, vsync, 0x00A8_79CF)
            .expect_err("sem saber o nome, a resposta e nao");
        assert!(!erro.trim().is_empty());

        // Por conteúdo: o driver acrescenta sufixos ("texture filtering - quality").
        let textura = opcao_por_id("textura").expect("o catalogo tem textura");
        assert!(veredito_do_nome(Some("texture filtering - quality"), textura, 0).is_ok());
    }

    // O portão pergunta AO DRIVER, não a uma constante.
    #[test]
    fn o_portao_pergunta_o_nome_ao_driver() {
        let fonte = codigo_fonte_deste_arquivo();

        assert!(
            fonte.contains("veredito_do_nome(nome_do_ajuste(api, id).as_deref(), opcao, id)"),
            "`conferir_o_numero` precisa julgar o nome que o DRIVER devolveu"
        );
    }

    // O portão do nome não pode sumir de nenhum dos três caminhos que escrevem.
    #[test]
    fn os_tres_caminhos_de_escrita_conferem_o_numero_antes() {
        let fonte = codigo_fonte_deste_arquivo();

        let confs = fonte.matches("conferir_o_numero(api, alvo, id)?;").count();
        assert_eq!(
            confs, 3,
            "`aplicar`, `restaurar_padrao` e o ramo de escrita do `desfazer` \
             precisam conferir o numero antes de mexer; achei {} chamadas",
            confs
        );
    }

    // E o salvamento: sem ele o desfazer passaria em todo teste sem mudar nada.
    #[test]
    fn a_sessao_salva_antes_de_ser_destruida() {
        let fonte = codigo_fonte_deste_arquivo();

        assert!(
            fonte.contains("(api.salvar_ajustes)(sessao)"),
            "`na_sessao` precisa salvar antes de fechar a sessao"
        );
        assert!(
            fonte.contains("(api.destruir_sessao)(sessao)"),
            "`na_sessao` precisa destruir a sessao mesmo quando o trabalho falha"
        );

        let salvar = fonte.find("(api.salvar_ajustes)(sessao)").unwrap();
        let destruir = fonte.find("(api.destruir_sessao)(sessao)").unwrap();
        assert!(
            salvar < destruir,
            "salvar precisa vir antes de destruir a sessao"
        );
    }
}
