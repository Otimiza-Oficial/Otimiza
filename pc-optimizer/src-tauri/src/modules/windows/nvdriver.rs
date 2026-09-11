// Ajustes de driver NVIDIA pela NVAPI
//
// Este é o único módulo do pilar que ESCREVE, e é o último a entrar de
// propósito: os cinco anteriores só leem, então se alguma coisa tivesse
// quebrado antes, nada teria sido alterado na máquina de ninguém.
//
// O QUE DECIDIU ESTE PILAR, no veredito da investigação anterior:
//
//     "A NVAPI tem um subsistema de configuração (DRS) feito exatamente para
//     isto, oficial e documentado, e — o que decide o pilar para este produto
//     — com chamada para RESTAURAR O PADRÃO de uma opção. Reversível de
//     verdade, não 'reversível se a gente anotar direitinho'."
//
// É por isso que uma opção SEM restauração de padrão não pode entrar no
// catálogo: ela quebraria a regra que sustenta o produto inteiro — toda
// mudança reversível, com o valor anterior registrado. O campo `id_do_padrao`
// carrega esse nome porque é a chamada de restauração que dá à opção o direito
// de existir aqui, e há um teste que recusa qualquer entrada sem ele.
//
// AMD FICA DE FORA, E A TELA DIZ ISSO. Também é recomendação literal do
// veredito: "entrar com a NVIDIA e DIZER NA TELA que a AMD ainda não é
// coberta, em vez de fazer meia coisa nas duas". Cliente com Radeon lê "ainda
// não cobrimos sua placa"; não uma tela vazia, que ele leria como programa
// quebrado.
//
// POR QUE A DLL É CARREGADA EM TEMPO DE EXECUÇÃO
//
// A `nvapi64.dll` acompanha o driver da NVIDIA — mesma escolha do `nvidia-smi`
// no `rbar.rs`, e pela mesma razão: não acrescentar nada ao instalador. Ligar
// contra ela em tempo de compilação faria o Otimiza SE RECUSAR A ABRIR em toda
// máquina sem placa NVIDIA, que é a maioria. Então a carga é por
// `LoadLibraryW`, e o fracasso dela é um estado do produto, não um erro do
// programa.
//
// A TRAVA QUE PROTEGE DE UM NÚMERO ERRADO NA TABELA
//
// Cada ajuste da DRS é endereçado por um número. Um número errado escreveria
// em OUTRA opção do driver — e o cliente veria mudar algo que não pediu. Por
// isso, antes de qualquer escrita, este módulo pergunta ao próprio driver como
// aquele número se chama (`NvAPI_DRS_GetSettingNameFromId`) e confere contra o
// nome esperado da tabela. Não batendo, ou não dando para perguntar, a escrita
// é RECUSADA. Fecha por padrão: a dúvida vira "não fiz", nunca "fiz assim
// mesmo".

use crate::modules::changelog::ChangeRecord;
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::sync::OnceLock;

// -------------------------------------------------------------- o estado

/// Em que pé está a NVAPI nesta máquina.
///
/// São TRÊS casos, e não dois, porque "não deu" tem causas diferentes com
/// conselhos diferentes: não ter placa NVIDIA é permanente e não tem conserto;
/// a DLL não responder é passageiro e pede driver novo. Colapsar os dois daria
/// o conselho errado para metade dos clientes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nvapi {
    Disponivel,
    SemPlacaNvidia,
    NaoCarregou,
}

/// A frase que o cliente lê.
pub fn nota_do_estado(estado: Nvapi) -> String {
    match estado {
        Nvapi::Disponivel => "Encontrei o driver da NVIDIA. Os ajustes abaixo são aplicados no \
             perfil global do driver, e cada um volta ao padrão de fábrica pelo \
             \"Desfazer\"."
            .to_string(),
        // SEM A PALAVRA "DRIVER", DE PROPÓSITO. Mandar o dono de uma Radeon
        // atualizar o driver da NVIDIA o faria perder tempo e concluir que o
        // produto não sabe do que fala. O que ele precisa ler é que a placa
        // dele ainda não é coberta — e que isso não é defeito do PC.
        Nvapi::SemPlacaNvidia => "Estes ajustes são só para placas NVIDIA, e ainda não cobrimos \
             placas AMD nem Intel. Não é problema no seu PC: é recurso que \
             ainda não chegou."
            .to_string(),
        // Aqui a placa é NVIDIA — a `nvapi64.dll` só existe onde o driver dela
        // foi instalado. O que falhou foi falar com ela, e isso tem conserto.
        Nvapi::NaoCarregou => "Encontrei uma placa NVIDIA, mas o driver não respondeu ao Otimiza. \
             Atualize o driver da NVIDIA pelo GeForce Experience ou pelo site \
             da NVIDIA e tente de novo. Nada foi alterado."
            .to_string(),
    }
}

/// Decide o estado a partir dos três fatos que a máquina fornece. PURA — é o
/// que permite provar os três casos sem placa de vídeo nenhuma, inclusive na
/// esteira, que roda em runner sem GPU.
///
/// A ORDEM IMPORTA. A `nvapi64.dll` só é instalada pelo driver da NVIDIA:
/// máquina sem o arquivo é máquina sem placa NVIDIA, e o conselho certo é
/// "ainda não cobrimos", não "atualize o driver". O arquivo EXISTIR e mesmo
/// assim não responder é a outra história — aí sim o driver está quebrado ou
/// velho demais.
pub fn classificar(dll_presente: bool, api_respondeu: bool, placas: u32) -> Nvapi {
    if !dll_presente {
        return Nvapi::SemPlacaNvidia;
    }

    if !api_respondeu {
        return Nvapi::NaoCarregou;
    }

    // Driver instalado e falando, mas nenhuma GPU enumerada: acontece com
    // driver deixado para trás depois de a placa sair da máquina.
    if placas == 0 {
        return Nvapi::SemPlacaNvidia;
    }

    Nvapi::Disponivel
}

// -------------------------------------------------------------- o catálogo

/// Um ajuste do driver que o produto sabe aplicar E desfazer.
#[derive(Debug, Clone, Copy)]
pub struct Opcao {
    /// Identificador interno, o que vai gravado no histórico de mudanças.
    pub id: &'static str,
    /// O que o cliente lê no botão.
    pub titulo: &'static str,
    /// Por que mexer nisto, em uma frase honesta.
    pub explicacao: &'static str,
    /// O número da opção na DRS — e, por isso mesmo, o número que a chamada de
    /// restauração de padrão recebe.
    ///
    /// É `Option` porque é ele quem decide se a opção pode existir: um ajuste
    /// que soubéssemos escrever mas não restaurar entraria aqui como `None`, e
    /// o teste `toda_opcao_conhecida_tem_como_restaurar_o_padrao` o recusaria
    /// antes de chegar em máquina de cliente.
    pub id_do_padrao: Option<u32>,
    /// Pedaço do nome que o próprio driver dá a esse número, em minúsculas.
    /// A trava contra um número errado na tabela — ver o cabeçalho.
    pub nome_esperado: &'static str,
    /// O valor que o Otimiza escreve.
    pub valor_otimizado: u32,
}

/// Os cinco ajustes. Números e valores vêm do `NvApiDriverSettings.h` público
/// da NVIDIA, e cada um é conferido contra o nome que o driver devolve antes
/// de qualquer escrita.
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
    },
    Opcao {
        id: "textura",
        titulo: "Filtragem de textura: desempenho",
        explicacao: "Deixa a placa caprichar menos no filtro das texturas. Rende \
                     quadros e, em movimento, quase não dá para ver a diferença.",
        id_do_padrao: Some(0x00CE_2691),
        nome_esperado: "texture filtering",
        // QUALITY_ENHANCEMENTS_PERFORMANCE. Até a 2.0 o título dizia "alto
        // desempenho", que no `NvApiDriverSettings.h` é outro valor (0x14, que
        // piora mais a imagem). O valor gravado continua o de "desempenho" —
        // o que a explicação promete —, e o título passou a dizer o mesmo.
        valor_otimizado: 0x0000_000A,
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
    },
];

/// Acha um ajuste pelo identificador. `None` para nome desconhecido — e é o
/// primeiro portão de toda função que escreve.
pub fn opcao_por_id(id: &str) -> Option<&'static Opcao> {
    OPCOES.iter().find(|opcao| opcao.id == id)
}

// ---------------------------------------------------------------- a tela

/// O id com que um ajuste entra no histórico de mudanças.
///
/// Mora aqui, e a tela recebe pronto: se o formato mudasse só de um lado, o
/// botão "Desfazer" procuraria um id que não existe.
pub fn id_no_historico(opcao: &str) -> String {
    format!("driver_nvidia:{}", opcao)
}

/// Um ajuste do catálogo como a tela o vê.
#[derive(Debug, Clone, Serialize)]
pub struct AjusteNaTela {
    pub id: &'static str,
    pub titulo: &'static str,
    pub explicacao: &'static str,
    /// O id no histórico, para o "Desfazer" deste ajuste.
    pub historico: String,
    /// Se o OTIMIZA aplicou e ainda não desfez. Não é leitura do driver: um
    /// ajuste que o cliente pôs à mão no painel da NVIDIA aparece como não
    /// aplicado, e o "Aplicar" guarda o valor dele para o desfazer devolver.
    pub aplicado: bool,
}

/// O painel de ajustes do driver: em que pé está a NVAPI e o que dá para fazer.
#[derive(Debug, Clone, Serialize)]
pub struct PainelDoDriver {
    pub estado: Nvapi,
    pub nota: String,
    pub ajustes: Vec<AjusteNaTela>,
}

/// Monta o painel. Só leitura: carrega a DLL e conta as placas, sem abrir
/// sessão de configuração do driver.
pub fn painel(aplicado: impl Fn(&str) -> bool) -> PainelDoDriver {
    let estado = estado();

    PainelDoDriver {
        estado,
        nota: nota_do_estado(estado),
        ajustes: OPCOES
            .iter()
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

// ------------------------------------------------- o valor anterior

/// O que fica gravado no histórico quando o valor anterior ERA o padrão de
/// fábrica do driver.
pub const ANTERIOR_PADRAO: &str = "padrao";

/// Como desfazer, decidido só a partir do que foi gravado no histórico. PURA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanoDeDesfazer {
    /// Chama a restauração de padrão da NVAPI — a chamada que decidiu o pilar.
    RestaurarPadrao,
    /// O cliente já tinha mexido nessa opção antes do Otimiza, e o valor dele
    /// não era o padrão. Devolver o padrão aqui seria APAGAR uma escolha dele,
    /// e "desfazer" viraria "mudar de novo".
    Escrever(u32),
}

/// Escreve o valor anterior do jeito que o histórico guarda.
pub fn codificar_anterior(era_o_padrao: bool, valor: u32) -> String {
    if era_o_padrao {
        ANTERIOR_PADRAO.to_string()
    } else {
        valor.to_string()
    }
}

/// Lê de volta o que `codificar_anterior` gravou.
///
/// Texto que não dá para entender cai em `RestaurarPadrao` de propósito: o
/// padrão do driver é um estado conhecido e definido, e é infinitamente melhor
/// do que deixar o ajuste como o Otimiza o pôs. Um "desfazer" que não desfaz é
/// o pior defeito possível neste produto.
pub fn plano_de_desfazer(valor_anterior: &str) -> PlanoDeDesfazer {
    match valor_anterior.trim().parse::<u32>() {
        Ok(valor) => PlanoDeDesfazer::Escrever(valor),
        Err(_) => PlanoDeDesfazer::RestaurarPadrao,
    }
}

/// Monta o registro que o histórico guarda, para o "Desfazer tudo" alcançar.
pub fn registro(opcao: &str, valor_anterior: String) -> ChangeRecord {
    ChangeRecord::DriverNvidia {
        opcao: opcao.to_string(),
        valor_anterior,
    }
}

// -------------------------------------------------------- a NVAPI de verdade

// Os números das funções da NVAPI. A biblioteca não exporta as funções pelo
// nome: exporta UM símbolo, `nvapi_QueryInterface`, que traduz cada número
// destes no ponteiro da função. É assim que a NVIDIA mantém compatibilidade
// entre versões de driver, e é a forma documentada de chamá-la.
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

/// `NVAPI_OK`. Todo erro da NVAPI é negativo.
const NVAPI_OK: i32 = 0;

/// Tipo do ajuste. Os cinco do catálogo são DWORD, e escrever num ajuste de
/// outro tipo como se fosse número é a segunda forma de estragar o driver.
const NVDRS_DWORD_TYPE: u32 = 0;

/// Um valor da DRS. É uma união em C — binário, texto ou número — e os quatro
/// primeiros bytes são o número. Todos os cinco ajustes do catálogo são
/// número, então é por `numero` que se lê e se escreve.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct ValorNvdrs {
    numero: u32,
    /// O resto da união (o corpo do valor binário). Existe para o tamanho da
    /// estrutura bater com o que o driver espera, e nunca é lido.
    _resto: [u8; 4096],
}

/// O `NVDRS_SETTING` da NVAPI, versão 1.
///
/// O TAMANHO É PARTE DO CONTRATO: o campo `versao` carrega o tamanho da
/// estrutura nos 16 bits de baixo, e o driver RECUSA a chamada se não bater.
/// Isso é uma sorte enorme — um desalinhamento aqui seria corrupção de memória
/// silenciosa; do jeito que a NVIDIA desenhou, vira um código de erro. Ainda
/// assim há um teste conferindo o tamanho, para o engano aparecer na esteira e
/// não na máquina do cliente.
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

/// Tamanho da `NVDRS_SETTING` v1, em bytes. Conferido por teste.
const TAMANHO_NVDRS_SETTING: usize = 12320;

/// `MAKE_NVAPI_VERSION(NVDRS_SETTING_V1, 1)`: o tamanho, com a versão nos bits
/// altos.
const NVDRS_SETTING_VER1: u32 = (TAMANHO_NVDRS_SETTING as u32) | (1 << 16);

impl NvdrsSetting {
    fn zerada() -> Self {
        // Toda a estrutura em zero, menos a versão. Campo não zerado faria o
        // driver ler lixo como se fosse pedido.
        let mut ajuste: NvdrsSetting = unsafe { std::mem::zeroed() };
        ajuste.versao = NVDRS_SETTING_VER1;
        ajuste
    }
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

/// Os ponteiros de função resolvidos uma vez só.
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
    /// Pode faltar em driver antigo. Sem ela não há como conferir os números da
    /// tabela, e por isso NENHUMA escrita acontece — ver o cabeçalho.
    nome_do_id: Option<FnNomeDoId>,
}

/// Carrega a `nvapi64.dll` e resolve o que este módulo usa.
///
/// A biblioteca NÃO é descarregada depois. É de propósito: o driver guarda
/// estado próprio a partir do `NvAPI_Initialize`, e devolver a DLL enquanto
/// esse estado existe é mais arriscado do que segurar um identificador até o
/// Otimiza fechar.
fn carregar_api() -> Result<Api, Nvapi> {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let nome: Vec<u16> = "nvapi64.dll\0".encode_utf16().collect();
    let modulo = unsafe { LoadLibraryW(nome.as_ptr()) };

    // Sem o arquivo não há driver NVIDIA instalado — logo, não há placa NVIDIA
    // para cuidar. Não é erro: é o cliente com AMD.
    if modulo.is_null() {
        return Err(Nvapi::SemPlacaNvidia);
    }

    let consulta = unsafe { GetProcAddress(modulo, c"nvapi_QueryInterface".as_ptr() as *const u8) }
        .ok_or(Nvapi::NaoCarregou)?;

    // O único símbolo exportado pela NVAPI: traduz um número de função no
    // ponteiro dela.
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

    // A DLL existe mas não entrega o básico: driver quebrado ou velho demais.
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
    })
}

fn api() -> Result<&'static Api, Nvapi> {
    static CACHE: OnceLock<Result<Api, Nvapi>> = OnceLock::new();
    CACHE.get_or_init(carregar_api).as_ref().map_err(|erro| *erro)
}

/// Em que pé está a NVAPI nesta máquina.
pub fn estado() -> Nvapi {
    let api = match api() {
        Ok(api) => api,
        // O erro da carga JÁ É o veredito: DLL ausente vira `SemPlacaNvidia`,
        // DLL muda vira `NaoCarregou`.
        Err(estado) => return estado,
    };

    // `NVAPI_MAX_PHYSICAL_GPUS` é 64.
    let mut placas: [*mut c_void; 64] = [std::ptr::null_mut(); 64];
    let mut quantas: u32 = 0;
    let situacao = unsafe { (api.enum_gpus)(placas.as_mut_ptr(), &mut quantas) };

    classificar(true, situacao == NVAPI_OK, quantas)
}

/// O nome que o driver dá a um número de ajuste, em minúsculas.
///
/// É a trava do cabeçalho: se o número da tabela estiver errado, o nome não
/// bate e a escrita é recusada.
fn nome_do_ajuste(api: &Api, id: u32) -> Option<String> {
    let funcao = api.nome_do_id?;
    let mut destino: [u16; 2048] = [0; 2048];

    if unsafe { funcao(id, &mut destino) } != NVAPI_OK {
        return None;
    }

    let fim = destino.iter().position(|c| *c == 0).unwrap_or(destino.len());
    Some(String::from_utf16_lossy(&destino[..fim]).to_lowercase())
}

/// O veredito sobre o nome, separado da chamada que o obtém. PURA — e é assim
/// que a trava do cabeçalho fica provável sem placa de vídeo.
///
/// FECHA POR PADRÃO. Sem conseguir perguntar (`None`), a resposta é NÃO: um
/// driver que não sabe dizer o nome de um número também não nos dá como saber
/// se aquele número é o que pensamos, e escrever no escuro é justamente o que
/// esta função existe para impedir.
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

/// Confere o número da tabela contra o nome que o driver reporta.
fn conferir_o_numero(api: &Api, opcao: &Opcao, id: u32) -> Result<(), String> {
    veredito_do_nome(nome_do_ajuste(api, id).as_deref(), opcao, id)
}

/// Abre a sessão da DRS, faz o trabalho no perfil global e salva.
///
/// Toda escrita passa por aqui para a sessão ser sempre destruída — inclusive
/// quando o trabalho falha no meio.
fn na_sessao<T>(
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

        let valor = trabalho(api, sessao, perfil)?;

        // SALVAR É O QUE TORNA A MUDANÇA REAL. Sem esta chamada tudo acontece
        // só dentro da sessão e some quando ela é destruída — inclusive o
        // desfazer, que passaria a "funcionar" sem mudar nada.
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

/// Traduz o que a NVAPI devolveu numa leitura em `(era_o_padrao, valor)`.
/// PURA — e separada de `ler_valor` justamente para poder ser provada sem
/// placa de vídeo, que é a única coisa que nenhum teste desta suíte tem.
///
/// AJUSTE QUE NÃO ESTÁ NO PERFIL É AJUSTE QUE NINGUÉM TOCOU. A NVAPI responde
/// com erro quando o ajuste nunca foi gravado no perfil global, e a leitura
/// certa disso é "está no padrão de fábrica" — não "não sei". A diferença
/// aparece no desfazer: no primeiro caso o Otimiza chama a restauração de
/// padrão; no segundo escreveria um zero que nunca existiu, e o cliente ficaria
/// com uma configuração que não é nem a dele nem a de fábrica.
fn interpretar_leitura(leitura_deu_certo: bool, e_predefinido: u32, valor: u32) -> (bool, u32) {
    if !leitura_deu_certo {
        return (true, 0);
    }

    // `e_predefinido` é o próprio driver dizendo que o valor de agora é o de
    // fábrica. Confiar nele é melhor do que comparar números na mão.
    (e_predefinido != 0, valor)
}

/// Lê o valor de um ajuste no perfil global.
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

/// O ajuste do catálogo, ou o erro que explica por que ele não pode ser
/// mexido. Primeiro portão de tudo que escreve.
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

/// Aplica um ajuste e DEVOLVE O VALOR ANTERIOR, para o histórico.
///
/// A ordem é a de sempre neste produto: lê o que existe, guarda, e só então
/// escreve. Sem esse registro não existe "desfazer" honesto — apenas a
/// promessa dele.
pub fn aplicar(opcao: &str) -> Result<String, String> {
    let alvo = numero_de(opcao)?;
    let id = alvo.id_do_padrao.expect("numero_de já garantiu");

    na_sessao(move |api, sessao, perfil| {
        conferir_o_numero(api, alvo, id)?;

        let (era_o_padrao, anterior) = ler_valor(api, sessao, perfil, id);
        escrever_valor_cru(api, sessao, perfil, id, alvo.valor_otimizado)?;

        Ok(codificar_anterior(era_o_padrao, anterior))
    })
}

/// Devolve um ajuste ao padrão de fábrica do driver.
///
/// É A CHAMADA QUE DECIDIU O PILAR. Não é o Otimiza reescrevendo um número que
/// anotou: é a própria NVIDIA dizendo qual é o padrão e voltando para ele.
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

/// Desfaz o que `aplicar` fez, a partir do que ficou no histórico.
///
/// É o ramo que o `revert_changes()` chama. Duas saídas, e a diferença entre
/// elas é o que separa este pilar de um "desfazer" de mentira: se o valor de
/// antes era o padrão de fábrica, quem restaura é a NVIDIA; se o cliente já
/// tinha uma escolha própria ali, é a escolha DELE que volta, não o padrão.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_nvapi_o_produto_diz_que_nao_cobre_em_vez_de_tela_vazia() {
        // A RECOMENDACAO LITERAL DO VEREDITO DA INVESTIGACAO ANTERIOR:
        // "entrar com a NVIDIA e DIZER NA TELA que a AMD ainda nao e coberta, em
        // vez de fazer meia coisa nas duas".
        //
        // Cliente com AMD lendo "ainda nao cobrimos sua placa" entende. Vendo tela
        // vazia, acha que o programa quebrou.
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
        // Dois "nao deu" com causas diferentes e conselhos diferentes: um pede
        // atualizar o driver, o outro diz que a placa nao e coberta. Colapsar os
        // dois daria o conselho errado para metade dos casos.
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
        // O QUE DECIDIU ESTE PILAR, segundo o veredito: a NVAPI tem chamada para
        // restaurar o padrao de uma opcao. "Reversivel de verdade, nao reversivel
        // se a gente anotar direitinho."
        //
        // Uma opcao sem restauracao nao pode entrar no catalogo: quebraria a regra
        // que sustenta o produto inteiro.
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
        // As tres notas precisam ser tres frases distintas. Duas iguais fariam a
        // tela mentir sobre qual dos casos aconteceu.
        let disponivel = nota_do_estado(Nvapi::Disponivel);
        assert!(!disponivel.trim().is_empty());
        assert_ne!(disponivel, nota_do_estado(Nvapi::SemPlacaNvidia));
        assert_ne!(disponivel, nota_do_estado(Nvapi::NaoCarregou));

        // E a nota da placa nao coberta NAO pode mandar mexer no driver: e o
        // conselho de outro caso, e faria o dono de uma Radeon perder tempo.
        assert!(!nota_do_estado(Nvapi::SemPlacaNvidia)
            .to_lowercase()
            .contains("driver"));
    }

    #[test]
    fn dll_ausente_e_placa_amd_e_nao_driver_quebrado() {
        // O conselho errado aqui custa caro: mandar o dono de uma Radeon
        // atualizar o driver da NVIDIA. A `nvapi64.dll` so existe onde o driver
        // da NVIDIA foi instalado, entao arquivo ausente e placa nao coberta.
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
        // Driver deixado para tras depois de a placa sair da maquina. Oferecer os
        // ajustes aqui daria erro na cara do cliente na hora de aplicar.
        assert_eq!(classificar(true, true, 0), Nvapi::SemPlacaNvidia);
        assert_eq!(classificar(true, true, 1), Nvapi::Disponivel);
        assert_eq!(classificar(true, true, 3), Nvapi::Disponivel);
    }

    #[test]
    fn o_catalogo_tem_as_cinco_e_nenhuma_repetida() {
        assert_eq!(OPCOES.len(), 5, "o pilar prometeu cinco ajustes");

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

        // Dois ajustes com o MESMO numero seriam dois botoes escrevendo na mesma
        // opcao do driver -- e um "desfazer" desfaria o do outro.
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
        // O painel lê a NVAPI desta máquina, mas a marca de "aplicado" vem só
        // do histórico — é ela que decide se o botão diz "Aplicar" ou
        // "Desfazer", e um "Desfazer" sem registro não teria o que devolver.
        let vsync = id_no_historico("vsync");
        let painel = painel(|id| id == vsync);

        assert_eq!(painel.ajustes.len(), OPCOES.len());
        for ajuste in &painel.ajustes {
            assert_eq!(ajuste.historico, id_no_historico(ajuste.id));
            assert_eq!(ajuste.aplicado, ajuste.id == "vsync", "{}", ajuste.id);
        }
        assert!(!painel.nota.trim().is_empty());
    }

    #[test]
    fn a_textura_grava_o_valor_que_o_titulo_diz() {
        // No `NvApiDriverSettings.h`: PERFORMANCE = 0x0A, HIGHPERFORMANCE = 0x14.
        // O título dizia "alto desempenho" sobre o valor de "desempenho" — o
        // cliente lia um nível e o driver recebia outro.
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
        // Este teste roda na maquina do dono, QUE TEM PLACA NVIDIA. Se o portao
        // do nome desconhecido nao viesse antes de tudo, ele abriria uma sessao
        // da DRS de verdade. Nenhum teste desta suite pode escrever no driver.
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
        // O historico guarda TEXTO. Se a ida e a volta nao baterem, o "desfazer"
        // devolve outro numero -- e o cliente fica com uma terceira configuracao,
        // que nao e nem a dele nem a nossa.
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

        // Zero NAO e "padrao": e valor legitimo de varios ajustes da NVIDIA.
        // Confundir os dois faria o desfazer restaurar o padrao onde o cliente
        // tinha escolhido zero.
        assert_eq!(
            plano_de_desfazer(&codificar_anterior(false, 0)),
            PlanoDeDesfazer::Escrever(0)
        );
    }

    #[test]
    fn historico_ilegivel_cai_no_padrao_do_driver_e_nao_no_silencio() {
        // Texto que nao da para entender e um estado que nao deveria existir.
        // Mas se existir, deixar o ajuste como o Otimiza o pos e o pior desfecho
        // possivel: o padrao do driver e um estado conhecido e definido.
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
                // E o que foi gravado tem que ser lido de volta como escrita, e
                // nao como restauracao de padrao.
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
        // O TAMANHO E PARTE DO CONTRATO: o campo `versao` carrega o tamanho nos
        // 16 bits de baixo e o driver recusa a chamada se nao bater. Este teste
        // faz o engano aparecer na esteira, e nao na maquina do cliente.
        assert_eq!(
            std::mem::size_of::<NvdrsSetting>(),
            TAMANHO_NVDRS_SETTING,
            "o layout da NVDRS_SETTING mudou"
        );
        assert_eq!(std::mem::size_of::<ValorNvdrs>(), 4100);
        assert_eq!(NVDRS_SETTING_VER1, 0x0001_3020);

        // E a estrutura zerada precisa sair com a versao preenchida e o resto em
        // zero -- campo com lixo vira pedido que ninguem fez.
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

    /// LE O ESTADO DESTA MAQUINA. Nao escreve nada: `estado()` so carrega a
    /// biblioteca, inicializa e conta as placas.
    #[test]
    fn le_o_estado_desta_maquina() {
        let estado = estado();
        println!("[{:?}] {}", estado, nota_do_estado(estado));

        // O que vale em qualquer maquina, com placa NVIDIA ou sem: a frase
        // acompanha o estado e nunca sai vazia.
        assert!(!nota_do_estado(estado).trim().is_empty());
    }

    /// A PROVA DE QUE OS NUMEROS DA TABELA SAO OS CERTOS.
    ///
    /// Na maquina do dono, com placa NVIDIA, este teste pergunta ao driver como
    /// ele chama cada numero da tabela e confere contra o nome esperado. Um
    /// numero trocado escreveria em outra opcao do driver, e nenhum teste de
    /// logica pegaria isso.
    ///
    /// Na esteira, que roda em runner sem placa NVIDIA, nao ha a quem perguntar
    /// e o teste passa sem afirmar nada -- de proposito: e verificacao de campo,
    /// nao requisito de compilacao.
    #[test]
    fn os_numeros_da_tabela_batem_com_o_que_o_driver_chama() {
        let Ok(api) = api() else {
            println!("sem NVAPI nesta maquina — nada a conferir");
            return;
        };

        for opcao in OPCOES {
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
                // NUMERO QUE O DRIVER NAO CONHECE E NUMERO ERRADO. Sem esta
                // exigencia, um digito trocado na tabela passaria batido: a
                // trava do `conferir_o_numero` recusaria a escrita em
                // silencio e o botao simplesmente nunca funcionaria, sem
                // ninguem saber por que.
                None => panic!(
                    "{}: este driver nao conhece o ajuste {:#010X} -- o numero da tabela \n                     esta errado, ou nao existe mais nesta versao do driver",
                    opcao.id, id
                ),
            }
        }
    }

    /// Lê o CÓDIGO deste próprio arquivo — só a parte de fora de `mod tests` —
    /// para os canários de texto-fonte abaixo. Mesmo truque do `rbar.rs`:
    /// cortar em `#[cfg(test)]` evita que o `.contains` se ache a si mesmo.
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

    // A LOGICA DE `classificar` ESTA PROVADA ACIMA -- mas com `bool`s escritos
    // a mao no teste. Isso nao amarra o que `estado()` REALMENTE passa: trocar
    // a chamada por `classificar(true, true, 1)` deixaria a suite inteira verde
    // e o produto diria "Disponivel" em toda maquina do planeta, inclusive nas
    // com Radeon.
    //
    // Provar isso de verdade exigiria injetar a NVAPI como parametro -- costura
    // que so existiria para o teste, num modulo cujo unico ponto de entrada real
    // e a DLL do driver. O canario de texto-fonte fecha o fio pelo lado barato,
    // como o `rbar.rs` ja faz nesta mesma branch.
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
        // A NVAPI responde com erro quando o ajuste nunca foi gravado no perfil
        // global -- e isso quer dizer "esta no padrao", nao "nao sei". Lido como
        // "nao sei", o desfazer escreveria um zero que nunca existiu, e o
        // cliente ficaria com uma configuracao que nao e nem a dele nem a de
        // fabrica. E o caso MAIS COMUM de todos: perfil limpo, cliente que
        // nunca abriu o Painel de Controle da NVIDIA.
        assert_eq!(interpretar_leitura(false, 0, 0), (true, 0));
        assert_eq!(
            interpretar_leitura(false, 0, 12345),
            (true, 0),
            "leitura que falhou nao tem valor para aproveitar"
        );

        // Leitura boa: quem decide se e o padrao e o driver.
        assert_eq!(interpretar_leitura(true, 1, 7), (true, 7));
        assert_eq!(interpretar_leitura(true, 0, 7), (false, 7));

        // E o valor zero de um ajuste que o cliente escolheu NAO pode virar
        // "padrao": zero e valor legitimo em varios ajustes da NVIDIA.
        assert_eq!(interpretar_leitura(true, 0, 0), (false, 0));

        // A ida e volta pelo historico precisa preservar os dois casos.
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

    // A leitura de verdade precisa passar o que a NVAPI devolveu, e nao
    // constantes: `interpretar_leitura(true, 1, 0)` ali dentro faria todo
    // ajuste parecer estar no padrao, e o desfazer restauraria o padrao por
    // cima da escolha do cliente. Nenhum teste de logica pega isso -- a suite
    // nao tem como chamar `ler_valor`, que precisa de uma sessao de verdade.
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
        // A UNICA COISA entre um numero errado na tabela e uma escrita na opcao
        // errada do driver. Aqui ela e provada sem placa de video nenhuma.
        let vsync = opcao_por_id("vsync").expect("o catalogo tem vsync");

        // O caso bom: o nome do driver contem o que a tabela espera.
        assert!(veredito_do_nome(Some("vertical sync"), vsync, 0x00A8_79CF).is_ok());

        // Numero errado apontando para OUTRA opcao conhecida do driver. E o
        // caso perigoso: a escrita daria certo, na opcao errada.
        let erro = veredito_do_nome(Some("shader cache"), vsync, 0x0019_8FFF)
            .expect_err("nome que nao bate precisa recusar");
        assert!(erro.contains("shader cache"), "{}", erro);
        assert!(erro.contains("vertical sync"), "{}", erro);

        // E o caso em que nao da para perguntar: FECHA POR PADRAO.
        let erro = veredito_do_nome(None, vsync, 0x00A8_79CF)
            .expect_err("sem saber o nome, a resposta e nao");
        assert!(!erro.trim().is_empty());

        // A comparacao e por conteudo, e nao por igualdade: o driver acrescenta
        // sufixos ("texture filtering - quality") e a tabela guarda so o miolo.
        let textura = opcao_por_id("textura").expect("o catalogo tem textura");
        assert!(veredito_do_nome(Some("texture filtering - quality"), textura, 0).is_ok());
    }

    // E o portao precisa perguntar AO DRIVER, e nao a uma constante: um
    // `veredito_do_nome(Some(opcao.nome_esperado), ...)` ali dentro passaria em
    // todo teste puro acima e nao conferiria coisa nenhuma.
    #[test]
    fn o_portao_pergunta_o_nome_ao_driver() {
        let fonte = codigo_fonte_deste_arquivo();

        assert!(
            fonte.contains("veredito_do_nome(nome_do_ajuste(api, id).as_deref(), opcao, id)"),
            "`conferir_o_numero` precisa julgar o nome que o DRIVER devolveu"
        );
    }

    // O PORTAO DO NOME e a unica coisa entre um numero errado na tabela e uma
    // escrita na opcao errada do driver. Ele nao pode sumir de nenhum dos tres
    // caminhos que escrevem, e teste de logica nao pega isso: a suite nao pode
    // chamar nenhum deles com um ajuste de verdade.
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

    // E o salvamento: sem ele, tudo acontece so dentro da sessao e some quando
    // ela e destruida. O "desfazer" passaria em todo teste e nao mudaria nada
    // na maquina do cliente -- exatamente o defeito que este produto nao pode
    // ter. Nenhum teste desta suite pode salvar de verdade para provar isso.
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
