// A parte Direct3D 11 do gerador. `Gpu` nunca sai da linha de execução dele; as texturas "anterior" e "atual"
// alternam de papel a cada quadro real, para não copiar imagem inteira à toa.

use windows::core::{Interface, PCSTR};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::Fxc::{D3DCompile, D3DCOMPILE_OPTIMIZATION_LEVEL3};
use windows::Win32::Graphics::Direct3D::{
    ID3DBlob, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::IDXGIAdapter1;

use super::ritmo::{niveis, Giro, Retangulo};

const FONTE: &str = include_str!("shaders.hlsl");

/// `OTIMIZA_FG_INVERTER` inverte as cores do quadro real, para provar que a sobreposição está na tela.
fn inverter_para_diagnostico() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var_os("OTIMIZA_FG_INVERTER").is_some())
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Parametros {
    texel_destino: [f32; 2],
    texel_origem: [f32; 2],
    tamanho_origem: [i32; 2],
    tamanho_aux: [i32; 2],
    t: f32,
    lambda: f32,
    limiar_erro: f32,
    faixa_erro: f32,
    /// Preenchido por `parametros` em todo passe: 1 na área de trabalho HDR.
    hdr: f32,
    /// Só no passe `Entrada`: o giro do monitor (`Giro::codigo`).
    rotacao: f32,
    /// Só no passe `Busca`: penalidade por pixel longe da previsão e vantagem da mediana dos pais.
    ajuste_fluxo: [f32; 2],
    /// `DisPedaco`: x = 1 testa como partida o movimento dos pedaços vizinhos (a propagação espacial do DIS).
    extra: [f32; 4],
}

pub struct Alvo {
    pub textura: ID3D11Texture2D,
    pub rtv: Option<ID3D11RenderTargetView>,
    pub srv: ID3D11ShaderResourceView,
    pub uav: Option<ID3D11UnorderedAccessView>,
    pub largura: u32,
    pub altura: u32,
}

struct Passes {
    vs: ID3D11VertexShader,
    luma: ID3D11PixelShader,
    global: ID3D11PixelShader,
    grossa: ID3D11PixelShader,
    fina: ID3D11PixelShader,
    suave: ID3D11PixelShader,
    escolha: ID3D11PixelShader,
    mascara: ID3D11PixelShader,
    final_: ID3D11PixelShader,
    final_suave: ID3D11PixelShader,
    copia: ID3D11PixelShader,
    entrada: ID3D11PixelShader,
    diferenca: ID3D11PixelShader,
    camera: ID3D11PixelShader,
    camera_min: ID3D11PixelShader,
    luma_meia: ID3D11PixelShader,
    busca: ID3D11ComputeShader,
    amplia: ID3D11PixelShader,
    gradiente: ID3D11PixelShader,
    dis_pedaco: ID3D11ComputeShader,
    dis_denso: ID3D11PixelShader,
    final_denso: ID3D11PixelShader,
    consistencia: ID3D11PixelShader,
    final_bidirecional: ID3D11PixelShader,
    easu: ID3D11PixelShader,
    rcas: ID3D11PixelShader,
    cursor: ID3D11PixelShader,
}

/// Escala: o quadro composto no tamanho do jogo (`origem`, ainda codificado) e ampliado pelo `Easu` (`ampliado`, do
/// tamanho da área do encaixe), antes do `Rcas` escrever na swap chain.
struct Ampliacao {
    origem: Alvo,
    ampliado: Alvo,
}

/// Nitidez do `Rcas`, em "stops" do FSR (0 = a máxima). 0,2 é o padrão dos exemplos da AMD.
const NITIDEZ_DA_ESCALA: f32 = 0.2;

struct Trabalho {
    largura: u32,
    altura: u32,
    hdr: bool,
    giro: Giro,
    /// HDR ou monitor girado: o recorte da área de trabalho em FP16 linear, antes de `Codificar` levar para `imagem`.
    bruto: Option<Alvo>,
    imagem: [Alvo; 2],
    /// O quadro capturado antes de entrar no par: só entra se for diferente do atual.
    candidato: Alvo,
    cand_q: Alvo,
    cand_s: Alvo,
    dif: Alvo,
    dif_leitura: ID3D11Texture2D,
    niveis_dif: u32,
    luma_q: [Alvo; 2],
    luma_s: [Alvo; 2],
    luma_g: [Alvo; 2],
    vet_global: Alvo,
    /// Movimento da câmera: a nota de cada candidato (2R+1)² e o vencedor (1×1).
    camera_notas: Alvo,
    camera: Alvo,
    vet_grosso: Alvo,
    vet_fino: Alvo,
    vet_suave: Alvo,
    /// A luma em 1/2 e 1/8, para a pirâmide do fluxo óptico andar de 2 em 2 (1/16 → 1/8 → 1/4 → 1/2).
    luma_m: [Alvo; 2],
    luma_o: [Alvo; 2],
    /// O fluxo de cada nível (um vetor por bloco 8×8 do nível), do 1/16 ao 1/2.
    fluxo: [Alvo; 4],
    /// O fluxo na grade de 1/8, antes da mediana.
    vet_amplo: Alvo,
    /// DIS, do 1/16 ao 1/2: luma com gradiente (dos dois reais), os vetores dos pedaços e o campo denso de cada nível.
    luma_grad: [[Alvo; 2]; 4],
    pedacos: [Alvo; 4],
    denso: [Alvo; 4],
    /// Os campos de ida (pixels de A) e de volta (pixels de B) em 1/16 e 1/8, e a consistência entre eles em 1/8.
    pedacos_ida: [Alvo; 2],
    denso_ida: [Alvo; 2],
    pedacos_volta: [Alvo; 2],
    denso_volta: [Alvo; 2],
    consistencia: Alvo,
    /// Refeito a cada quadro gerado, porque depende de `t`.
    escolhido: Alvo,
    mascara: [Alvo; 2],
    mascara_atual: usize,
    nota_leitura: ID3D11Texture2D,
    niveis_escolhido: u32,
    consistencia_leitura: ID3D11Texture2D,
    niveis_consistencia: u32,
}

pub struct Gpu {
    pub device: ID3D11Device,
    pub ctx: ID3D11DeviceContext,
    passes: Passes,
    amostrador: ID3D11SamplerState,
    buffer: ID3D11Buffer,
    trabalho: Option<Trabalho>,
    atual: usize,
    pub reais: u64,
    /// A composição sem blocos (`FinalSuave`), só para comparar (`OTIMIZA_FG_FINAL=suave`): nos quadros do FiveM ela
    /// tirou os quadradinhos mas deixou o cenário ondulado (SSIM 0,641 contra 0,663 da por blocos).
    pub suave: bool,
    /// O fluxo óptico por blocos no método do AMD (`Busca`), em teste (`OTIMIZA_FG_BUSCA=blocos`). Nos quadros gravados
    /// do FiveM acertou melhor o cenário (banco, quiosques) mas rasgou o personagem parado, e a média ficou abaixo da
    /// busca Grossa/Fina (SSIM 0,655 contra 0,663) custando o dobro na placa: por isso não é o padrão.
    pub busca_por_blocos: bool,
    /// O fluxo denso por busca inversa (`DisPedaco`/`DisDenso`/`FinalDenso`), o PADRÃO. Nos quadros gravados do FiveM:
    /// SSIM 0,664 contra 0,663 da Grossa/Fina, mesmo custo na placa (1,3 ms), o cenário sem os quadradinhos e o rosto
    /// do personagem inteiro. `OTIMIZA_FG_BUSCA=antiga` (ou `blocos`) volta às outras, só para comparar.
    pub dis: bool,
    /// Oclusão pelos vetores de ida e volta e mediana dinâmica (`FinalBidirecional`). `OTIMIZA_FG_OCLUSAO=0` desliga, só
    /// para comparar.
    pub oclusao: bool,
    /// Até que nível a busca principal desce (3 = 1/4 da imagem; 2 = 1/8, o modo leve). Trocável com o gerador ligado:
    /// cada quadro real refaz o campo inteiro.
    pub niveis_dis: usize,
    /// Só escala, sem geração: o `receber` só copia a imagem (nem luma nem fluxo).
    pub sem_geracao: bool,
    ampliacao: Option<Ampliacao>,
    /// Durante a composição no alvo da escala: os passes não passam pela `Sair` (no HDR a imagem segue codificada
    /// até o `Rcas`).
    saida_codificada: std::cell::Cell<bool>,
    /// A forma do cursor, para desenhar ampliado por cima do jogo na escala, e a mistura pelo alfa dela.
    forma_do_cursor: Option<Alvo>,
    mistura: ID3D11BlendState,
}

/// O modo leve (o "Performance" do Lossless Scaling): a busca principal para em 1/8, sem o passe de 1/4, o mais caro
/// dela. A oclusão fica (sem ela a troca de cena também sairia). Para placa sem folga, ou quando a guarda vê o jogo
/// perder FPS.
pub const NIVEIS_DO_MODO_LEVE: usize = 2;

/// Penalidade por pixel longe da previsão e vantagem da mediana dos pais, na busca do fluxo (diferença somada do bloco
/// 8×8, luma de 0 a 1). `OTIMIZA_FG_AJUSTE=penalidade,mediana` troca, só para medir.
fn ajuste_do_fluxo() -> [f32; 2] {
    static V: std::sync::OnceLock<[f32; 2]> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        let padrao = [0.02, 0.1];
        std::env::var("OTIMIZA_FG_AJUSTE")
            .ok()
            .and_then(|v| {
                let mut p = v.split(',').map(|x| x.trim().parse::<f32>());
                Some([p.next()?.ok()?, p.next()?.ok()?])
            })
            .unwrap_or(padrao)
    })
}

/// Quantos pedaços 8×8, a cada 4 pixels, cobrem um lado de `n` pixels (igual a `DisPedaco` no shader).
fn pedacos(n: u32) -> u32 {
    (n.saturating_sub(8)).div_ceil(4) + 1
}

/// Amortecimento, iterações e folga do parado no refino do DIS (erro ao quadrado somado no pedaço 8×8).
/// `OTIMIZA_FG_DIS=amortecimento,iteracoes,folga` troca, só para medir.
fn ajuste_do_dis() -> (f32, f32, f32) {
    static V: std::sync::OnceLock<(f32, f32, f32)> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        std::env::var("OTIMIZA_FG_DIS")
            .ok()
            .and_then(|v| {
                let mut p = v.split(',').map(|x| x.trim().parse::<f32>());
                Some((p.next()?.ok()?, p.next()?.ok()?, p.next()?.ok()?))
            })
            .unwrap_or((0.05, 6.0, 0.2))
    })
}

/// A propagação espacial do DIS (OpenCV `dis_flow.cpp`, Apache-2.0, a ideia reescrita para a placa): cada pedaço testa
/// como partida também o movimento dos quatro vizinhos no nível de cima, antes do refino. Conserta o pedaço que
/// começaria no vale errado — borda de objeto, carro cruzando a tela. `OTIMIZA_FG_PROPAGA=0` desliga, só para medir.
fn propagar() -> bool {
    std::env::var("OTIMIZA_FG_PROPAGA")
        .map(|v| v != "0")
        .unwrap_or(true)
}

/// Até que nível o DIS desce: 4 = 1/2 da imagem, 3 = 1/4. `OTIMIZA_FG_DIS_NIVEIS` troca, só para medir.
fn niveis_do_dis() -> usize {
    static V: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        std::env::var("OTIMIZA_FG_DIS_NIVEIS")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|n| (1..=4).contains(n))
            .unwrap_or(3)
    })
}

/// Raio da busca inteira antes do refino, por nível (1/16, 1/8, 1/4, 1/2). Só nos níveis de cima, onde o movimento
/// ainda é grande para o refino. `OTIMIZA_FG_DIS_RAIO=2,2,0,0` troca, só para medir.
fn raios_do_dis() -> [f32; 4] {
    static V: std::sync::OnceLock<[f32; 4]> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        let mut r = [2.0, 2.0, 0.0, 0.0];
        if let Ok(v) = std::env::var("OTIMIZA_FG_DIS_RAIO") {
            for (i, x) in v.split(',').take(4).enumerate() {
                if let Ok(x) = x.trim().parse() {
                    r[i] = x;
                }
            }
        }
        r
    })
}

fn erro(contexto: &str, e: windows::core::Error) -> String {
    format!("{}: {}", contexto, e.message())
}

fn compilar(entrada: &str, alvo: &str) -> Result<ID3DBlob, String> {
    let mut codigo: Option<ID3DBlob> = None;
    let mut mensagens: Option<ID3DBlob> = None;
    let entrada_c = format!("{}\0", entrada);
    let alvo_c = format!("{}\0", alvo);
    let r = unsafe {
        D3DCompile(
            FONTE.as_ptr() as *const _,
            FONTE.len(),
            PCSTR(b"geracao.hlsl\0".as_ptr()),
            None,
            None,
            PCSTR(entrada_c.as_ptr()),
            PCSTR(alvo_c.as_ptr()),
            D3DCOMPILE_OPTIMIZATION_LEVEL3,
            0,
            &mut codigo,
            Some(&mut mensagens),
        )
    };
    if let Err(e) = r {
        let detalhe = mensagens
            .map(|m| unsafe {
                String::from_utf8_lossy(std::slice::from_raw_parts(
                    m.GetBufferPointer() as *const u8,
                    m.GetBufferSize(),
                ))
                .to_string()
            })
            .unwrap_or_default();
        return Err(format!(
            "shader {} não compilou: {} {}",
            entrada,
            e.message(),
            detalhe
        ));
    }
    codigo.ok_or_else(|| format!("shader {} sem código", entrada))
}

pub(super) fn meio_float(h: u16) -> f32 {
    let sinal = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let expoente = ((h >> 10) & 0x1f) as i32;
    let fracao = (h & 0x3ff) as f32;
    sinal
        * match expoente {
            0 => fracao / 1024.0 * 2f32.powi(-14),
            31 => f32::INFINITY,
            e => (1.0 + fracao / 1024.0) * 2f32.powi(e - 15),
        }
}

fn bytes(b: &ID3DBlob) -> &[u8] {
    unsafe { std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize()) }
}

impl Gpu {
    pub fn novo(adaptador: &IDXGIAdapter1) -> Result<Gpu, String> {
        let mut device = None;
        let mut ctx = None;
        unsafe {
            D3D11CreateDevice(
                adaptador,
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut ctx),
            )
            .map_err(|e| erro("a placa não criou o dispositivo Direct3D 11", e))?;
        }
        let device: ID3D11Device = device.ok_or("sem dispositivo")?;
        let ctx: ID3D11DeviceContext = ctx.ok_or("sem contexto")?;

        let ps = |nome: &str| -> Result<ID3D11PixelShader, String> {
            let b = compilar(nome, "ps_5_0")?;
            let mut s = None;
            unsafe { device.CreatePixelShader(bytes(&b), None, Some(&mut s)) }
                .map_err(|e| erro(nome, e))?;
            s.ok_or_else(|| nome.to_string())
        };
        let vs_b = compilar("VS", "vs_5_0")?;
        let mut vs = None;
        unsafe { device.CreateVertexShader(bytes(&vs_b), None, Some(&mut vs)) }
            .map_err(|e| erro("VS", e))?;

        let passes = Passes {
            vs: vs.ok_or("VS")?,
            luma: ps("Luma")?,
            global: ps("Global")?,
            grossa: ps("Grossa")?,
            fina: ps("Fina")?,
            suave: ps("Suave")?,
            escolha: ps("Escolha")?,
            mascara: ps("Mascara")?,
            final_: ps("Final")?,
            final_suave: ps("FinalSuave")?,
            copia: ps("Copia")?,
            entrada: ps("Entrada")?,
            diferenca: ps("Diferenca")?,
            camera: ps("Camera")?,
            camera_min: ps("CameraMin")?,
            luma_meia: ps("LumaMeia")?,
            busca: {
                let b = compilar("Busca", "cs_5_0")?;
                let mut s = None;
                unsafe { device.CreateComputeShader(bytes(&b), None, Some(&mut s)) }
                    .map_err(|e| erro("Busca", e))?;
                s.ok_or("Busca")?
            },
            amplia: ps("Amplia")?,
            gradiente: ps("Gradiente")?,
            dis_pedaco: {
                let b = compilar("DisPedaco", "cs_5_0")?;
                let mut s = None;
                unsafe { device.CreateComputeShader(bytes(&b), None, Some(&mut s)) }
                    .map_err(|e| erro("DisPedaco", e))?;
                s.ok_or("DisPedaco")?
            },
            dis_denso: ps("DisDenso")?,
            final_denso: ps("FinalDenso")?,
            consistencia: ps("Consistencia")?,
            final_bidirecional: ps("FinalBidirecional")?,
            easu: ps("Easu")?,
            rcas: ps("Rcas")?,
            cursor: ps("Cursor")?,
        };

        // Alfa pré-multiplicado: cor + destino × (1 − alfa).
        let mut mistura = None;
        unsafe {
            let mut desc = D3D11_BLEND_DESC::default();
            desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: true.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            device
                .CreateBlendState(&desc, Some(&mut mistura))
                .map_err(|e| erro("mistura do cursor", e))?;
        }

        let mut amostrador = None;
        unsafe {
            device
                .CreateSamplerState(
                    &D3D11_SAMPLER_DESC {
                        Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                        AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                        AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                        AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                        MaxLOD: f32::MAX,
                        ..Default::default()
                    },
                    Some(&mut amostrador),
                )
                .map_err(|e| erro("amostrador", e))?;
        }

        let mut buffer = None;
        unsafe {
            device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: std::mem::size_of::<Parametros>().next_multiple_of(16) as u32,
                        Usage: D3D11_USAGE_DYNAMIC,
                        BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                        CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
                        ..Default::default()
                    },
                    None,
                    Some(&mut buffer),
                )
                .map_err(|e| erro("buffer de parâmetros", e))?;
        }

        Ok(Gpu {
            device,
            ctx,
            passes,
            amostrador: amostrador.ok_or("amostrador")?,
            buffer: buffer.ok_or("buffer")?,
            trabalho: None,
            atual: 0,
            reais: 0,
            suave: std::env::var("OTIMIZA_FG_FINAL").is_ok_and(|v| v == "suave"),
            busca_por_blocos: std::env::var("OTIMIZA_FG_BUSCA").is_ok_and(|v| v == "blocos"),
            dis: !std::env::var("OTIMIZA_FG_BUSCA").is_ok_and(|v| v == "antiga" || v == "blocos"),
            oclusao: !std::env::var("OTIMIZA_FG_OCLUSAO").is_ok_and(|v| v == "0"),
            niveis_dis: niveis_do_dis(),
            sem_geracao: false,
            ampliacao: None,
            saida_codificada: std::cell::Cell::new(false),
            forma_do_cursor: None,
            mistura: mistura.ok_or("mistura")?,
        })
    }

    /// Liga ou desliga o modo leve (`NIVEIS_DO_MODO_LEVE`).
    pub fn modo_leve(&mut self, leve: bool) {
        self.niveis_dis = if leve {
            NIVEIS_DO_MODO_LEVE.min(niveis_do_dis())
        } else {
            niveis_do_dis()
        };
    }

    pub fn leve(&self) -> bool {
        self.niveis_dis < niveis_do_dis()
    }

    fn alvo(
        &self,
        largura: u32,
        altura: u32,
        formato: DXGI_FORMAT,
        render: bool,
    ) -> Result<Alvo, String> {
        let mut bind = D3D11_BIND_SHADER_RESOURCE.0 as u32;
        if render {
            bind |= D3D11_BIND_RENDER_TARGET.0 as u32;
        }
        let desc = D3D11_TEXTURE2D_DESC {
            Width: largura,
            Height: altura,
            MipLevels: 1,
            ArraySize: 1,
            Format: formato,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: bind,
            ..Default::default()
        };
        let mut textura = None;
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut textura)) }
            .map_err(|e| erro("textura", e))?;
        let textura: ID3D11Texture2D = textura.ok_or("textura")?;
        let mut srv = None;
        unsafe {
            self.device
                .CreateShaderResourceView(&textura, None, Some(&mut srv))
        }
        .map_err(|e| erro("srv", e))?;
        let rtv = if render {
            let mut r = None;
            unsafe {
                self.device
                    .CreateRenderTargetView(&textura, None, Some(&mut r))
            }
            .map_err(|e| erro("rtv", e))?;
            r
        } else {
            None
        };
        Ok(Alvo {
            textura,
            rtv,
            srv: srv.ok_or("srv")?,
            uav: None,
            largura,
            altura,
        })
    }

    fn alvo_com_mips(
        &self,
        largura: u32,
        altura: u32,
        formato: DXGI_FORMAT,
    ) -> Result<Alvo, String> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: largura,
            Height: altura,
            MipLevels: 0,
            ArraySize: 1,
            Format: formato,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
            MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
            ..Default::default()
        };
        let mut textura = None;
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut textura)) }
            .map_err(|e| erro("textura com mips", e))?;
        let textura: ID3D11Texture2D = textura.ok_or("textura")?;
        let mut srv = None;
        unsafe {
            self.device
                .CreateShaderResourceView(&textura, None, Some(&mut srv))
        }
        .map_err(|e| erro("srv", e))?;
        let mut rtv = None;
        unsafe {
            self.device
                .CreateRenderTargetView(&textura, None, Some(&mut rtv))
        }
        .map_err(|e| erro("rtv", e))?;
        Ok(Alvo {
            textura,
            rtv,
            srv: srv.ok_or("srv")?,
            uav: None,
            largura,
            altura,
        })
    }

    /// Textura que um passe de computação escreve (`RWTexture2D`) e os outros leem.
    fn alvo_uav(&self, largura: u32, altura: u32, formato: DXGI_FORMAT) -> Result<Alvo, String> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: largura,
            Height: altura,
            MipLevels: 1,
            ArraySize: 1,
            Format: formato,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_UNORDERED_ACCESS.0) as u32,
            ..Default::default()
        };
        let mut textura = None;
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut textura)) }
            .map_err(|e| erro("textura do fluxo", e))?;
        let textura: ID3D11Texture2D = textura.ok_or("textura")?;
        let mut srv = None;
        unsafe {
            self.device
                .CreateShaderResourceView(&textura, None, Some(&mut srv))
        }
        .map_err(|e| erro("srv", e))?;
        let mut uav = None;
        unsafe {
            self.device
                .CreateUnorderedAccessView(&textura, None, Some(&mut uav))
        }
        .map_err(|e| erro("uav", e))?;
        Ok(Alvo {
            textura,
            rtv: None,
            srv: srv.ok_or("srv")?,
            uav,
            largura,
            altura,
        })
    }

    fn leitura_1x1(&self, formato: DXGI_FORMAT) -> Result<ID3D11Texture2D, String> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: 1,
            Height: 1,
            MipLevels: 1,
            ArraySize: 1,
            Format: formato,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            ..Default::default()
        };
        let mut textura = None;
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut textura)) }
            .map_err(|e| erro("leitura", e))?;
        textura.ok_or_else(|| "leitura".to_string())
    }

    /// Fração da imagem em que os dois reais discordam muito (t = 0,5). Giro rápido, troca de cena e explosão
    /// dão nota alta, e o quadro gerado não sai.
    #[cfg(test)]
    pub fn fracao_ruim(&self) -> Option<f32> {
        let tr = self.trabalho.as_ref()?;
        if !self.tem_par() {
            return None;
        }
        let (a, b) = (1 - self.atual, self.atual);
        self.passe(
            &self.passes.escolha,
            tr.escolhido.rtv.as_ref().unwrap(),
            tr.escolhido.largura,
            tr.escolhido.altura,
            &[
                Some(tr.imagem[a].srv.clone()),
                Some(tr.imagem[b].srv.clone()),
                Some(tr.vet_suave.srv.clone()),
            ],
            Parametros {
                texel_destino: [1.0 / tr.largura as f32, 1.0 / tr.altura as f32],
                tamanho_aux: [tr.vet_suave.largura as i32, tr.vet_suave.altura as i32],
                t: 0.5,
                ..Default::default()
            },
        );
        unsafe {
            self.ctx.GenerateMips(&tr.escolhido.srv);
            let ultimo = tr.niveis_escolhido.saturating_sub(1);
            self.ctx.CopySubresourceRegion(
                &tr.nota_leitura,
                0,
                0,
                0,
                0,
                &tr.escolhido.textura,
                ultimo,
                None,
            );
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            self.ctx
                .Map(&tr.nota_leitura, 0, D3D11_MAP_READ, 0, Some(&mut m))
                .ok()?;
            let bytes = std::slice::from_raw_parts(m.pData as *const u8, 8);
            let ruim = meio_float(u16::from_le_bytes([bytes[6], bytes[7]]));
            self.ctx.Unmap(&tr.nota_leitura, 0);
            Some(ruim)
        }
    }

    /// `hdr`: a área de trabalho vem em FP16 linear (`DuplicateOutput1` numa tela HDR). A imagem guardada passa a ser
    /// FP16 codificada com gama, e a saída volta a linear.
    /// `giro`: o monitor está girado e a captura vem na orientação do painel; a imagem é girada de volta na entrada.
    pub fn preparar(
        &mut self,
        largura: u32,
        altura: u32,
        hdr: bool,
        giro: Giro,
    ) -> Result<(), String> {
        if self.trabalho.as_ref().is_some_and(|t| {
            t.largura == largura && t.altura == altura && t.hdr == hdr && t.giro == giro
        }) {
            return Ok(());
        }
        let ((ql, qa), (fl, fa), (sl, sa)) = niveis(largura as i32, altura as i32);
        let (gl, ga) = (sl.div_ceil(4).max(1), sa.div_ceil(4).max(1));
        let (ml, ma) = (largura.div_ceil(2), altura.div_ceil(2));
        let luma = DXGI_FORMAT_R16_FLOAT;
        let vet = DXGI_FORMAT_R16G16B16A16_FLOAT;
        let formato = if hdr {
            DXGI_FORMAT_R16G16B16A16_FLOAT
        } else {
            DXGI_FORMAT_B8G8R8A8_UNORM
        };
        // Com HDR ou monitor girado a imagem passa pelo passe `Entrada` e precisa ser alvo; sem, a cópia é direta.
        let com_entrada = hdr || giro != Giro::Nenhum;
        let (bl, ba) = if giro.troca_lados() {
            (altura, largura)
        } else {
            (largura, altura)
        };
        self.trabalho = Some(Trabalho {
            largura,
            altura,
            hdr,
            giro,
            bruto: if com_entrada {
                Some(self.alvo(bl, ba, formato, false)?)
            } else {
                None
            },
            imagem: [
                self.alvo(largura, altura, formato, com_entrada)?,
                self.alvo(largura, altura, formato, com_entrada)?,
            ],
            candidato: self.alvo(largura, altura, formato, com_entrada)?,
            cand_q: self.alvo(ql, qa, luma, true)?,
            cand_s: self.alvo(sl, sa, luma, true)?,
            // Em 1/4: em 1/16 um giro de câmera de 3 px vira 0,2 px e some na média (o quadro andando parecia repetido).
            dif: self.alvo_com_mips(ql, qa, luma)?,
            dif_leitura: self.leitura_1x1(luma)?,
            niveis_dif: 32 - ql.max(qa).leading_zeros(),
            luma_q: [
                self.alvo(ql, qa, luma, true)?,
                self.alvo(ql, qa, luma, true)?,
            ],
            luma_s: [
                self.alvo(sl, sa, luma, true)?,
                self.alvo(sl, sa, luma, true)?,
            ],
            luma_g: [
                self.alvo(gl, ga, luma, true)?,
                self.alvo(gl, ga, luma, true)?,
            ],
            vet_global: self.alvo(gl, ga, vet, true)?,
            camera_notas: self.alvo(25, 25, vet, true)?,
            camera: self.alvo(1, 1, vet, true)?,
            vet_grosso: self.alvo(sl, sa, vet, true)?,
            vet_fino: self.alvo(fl, fa, vet, true)?,
            vet_suave: self.alvo(fl, fa, vet, true)?,
            vet_amplo: self.alvo(fl, fa, vet, true)?,
            luma_grad: {
                let par = |l: u32, a: u32| -> Result<[Alvo; 2], String> {
                    Ok([self.alvo(l, a, vet, true)?, self.alvo(l, a, vet, true)?])
                };
                [par(sl, sa)?, par(fl, fa)?, par(ql, qa)?, par(ml, ma)?]
            },
            pedacos: [
                self.alvo_uav(pedacos(sl), pedacos(sa), vet)?,
                self.alvo_uav(pedacos(fl), pedacos(fa), vet)?,
                self.alvo_uav(pedacos(ql), pedacos(qa), vet)?,
                self.alvo_uav(pedacos(ml), pedacos(ma), vet)?,
            ],
            denso: [
                self.alvo(sl, sa, vet, true)?,
                self.alvo(fl, fa, vet, true)?,
                self.alvo(ql, qa, vet, true)?,
                self.alvo(ml, ma, vet, true)?,
            ],
            pedacos_ida: [
                self.alvo_uav(pedacos(sl), pedacos(sa), vet)?,
                self.alvo_uav(pedacos(fl), pedacos(fa), vet)?,
            ],
            denso_ida: [self.alvo(sl, sa, vet, true)?, self.alvo(fl, fa, vet, true)?],
            pedacos_volta: [
                self.alvo_uav(pedacos(sl), pedacos(sa), vet)?,
                self.alvo_uav(pedacos(fl), pedacos(fa), vet)?,
            ],
            denso_volta: [self.alvo(sl, sa, vet, true)?, self.alvo(fl, fa, vet, true)?],
            consistencia: self.alvo_com_mips(fl, fa, vet)?,
            consistencia_leitura: self.leitura_1x1(vet)?,
            niveis_consistencia: 32 - fl.max(fa).leading_zeros(),
            luma_m: [
                self.alvo(ml, ma, luma, true)?,
                self.alvo(ml, ma, luma, true)?,
            ],
            luma_o: [
                self.alvo(fl, fa, luma, true)?,
                self.alvo(fl, fa, luma, true)?,
            ],
            fluxo: [
                self.alvo_uav(sl.div_ceil(8), sa.div_ceil(8), vet)?,
                self.alvo_uav(fl.div_ceil(8), fa.div_ceil(8), vet)?,
                self.alvo_uav(ql.div_ceil(8), qa.div_ceil(8), vet)?,
                self.alvo_uav(ml.div_ceil(8), ma.div_ceil(8), vet)?,
            ],
            escolhido: self.alvo_com_mips((largura + 1) / 2, (altura + 1) / 2, vet)?,
            nota_leitura: self.leitura_1x1(vet)?,
            mascara: [
                self.alvo((largura + 1) / 2, (altura + 1) / 2, luma, true)?,
                self.alvo((largura + 1) / 2, (altura + 1) / 2, luma, true)?,
            ],
            mascara_atual: 0,
            niveis_escolhido: 32 - ((largura + 1) / 2).max((altura + 1) / 2).leading_zeros(),
        });
        self.reais = 0;
        Ok(())
    }

    fn parametros(&self, mut p: Parametros) {
        p.hdr = if self.trabalho.as_ref().is_some_and(|t| t.hdr) && !self.saida_codificada.get() {
            1.0
        } else {
            0.0
        };
        unsafe {
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            if self
                .ctx
                .Map(&self.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m))
                .is_ok()
            {
                std::ptr::copy_nonoverlapping(
                    &p as *const Parametros as *const u8,
                    m.pData as *mut u8,
                    std::mem::size_of::<Parametros>(),
                );
                self.ctx.Unmap(&self.buffer, 0);
            }
        }
    }

    fn passe(
        &self,
        ps: &ID3D11PixelShader,
        rtv: &ID3D11RenderTargetView,
        largura: u32,
        altura: u32,
        entradas: &[Option<ID3D11ShaderResourceView>],
        p: Parametros,
    ) {
        let area = Retangulo {
            x: 0,
            y: 0,
            largura: largura as i32,
            altura: altura as i32,
        };
        self.passe_na_area(ps, rtv, area, entradas, p);
    }

    /// Um passe de tela cheia que só desenha em `area` do alvo (o `SV_Position` continua contando do canto do alvo).
    fn passe_na_area(
        &self,
        ps: &ID3D11PixelShader,
        rtv: &ID3D11RenderTargetView,
        area: Retangulo,
        entradas: &[Option<ID3D11ShaderResourceView>],
        p: Parametros,
    ) {
        self.parametros(p);
        unsafe {
            self.ctx
                .OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
            self.ctx.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: area.x as f32,
                TopLeftY: area.y as f32,
                Width: area.largura as f32,
                Height: area.altura as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
            self.ctx.IASetPrimitiveTopology(
                windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            );
            self.ctx.IASetInputLayout(None);
            self.ctx.VSSetShader(&self.passes.vs, None);
            self.ctx.PSSetShader(ps, None);
            self.ctx.PSSetShaderResources(0, Some(entradas));
            self.ctx
                .PSSetSamplers(0, Some(&[Some(self.amostrador.clone())]));
            self.ctx
                .PSSetConstantBuffers(0, Some(&[Some(self.buffer.clone())]));
            self.ctx.Draw(3, 0);
            // A mesma textura vira alvo no passe seguinte.
            self.ctx
                .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
            self.ctx.OMSetRenderTargets(None, None);
        }
    }

    /// Um passe de computação: `grupos` grupos de 8×8, escrevendo em `saida`.
    fn calcular(
        &self,
        cs: &ID3D11ComputeShader,
        saida: &Alvo,
        grupos: (u32, u32),
        entradas: &[Option<ID3D11ShaderResourceView>],
        p: Parametros,
    ) {
        self.parametros(p);
        unsafe {
            self.ctx.CSSetShader(cs, None);
            self.ctx.CSSetShaderResources(0, Some(entradas));
            self.ctx
                .CSSetConstantBuffers(0, Some(&[Some(self.buffer.clone())]));
            let uav = [saida.uav.clone()];
            self.ctx
                .CSSetUnorderedAccessViews(0, 1, Some(uav.as_ptr()), None);
            self.ctx.Dispatch(grupos.0, grupos.1, 1);
            self.ctx
                .CSSetShaderResources(0, Some(&[None, None, None, None]));
            let nenhum: [Option<ID3D11UnorderedAccessView>; 1] = [None];
            self.ctx
                .CSSetUnorderedAccessViews(0, 1, Some(nenhum.as_ptr()), None);
        }
    }

    /// O recorte da captura em `img`: cópia direta, ou pelo passe `Entrada` (HDR, monitor girado).
    fn copiar(
        &self,
        t: &Trabalho,
        img: &Alvo,
        area_de_trabalho: &ID3D11Texture2D,
        recorte: Retangulo,
    ) {
        let caixa = D3D11_BOX {
            left: recorte.x as u32,
            top: recorte.y as u32,
            front: 0,
            right: (recorte.x + recorte.largura) as u32,
            bottom: (recorte.y + recorte.altura) as u32,
            back: 1,
        };
        match &t.bruto {
            None => unsafe {
                self.ctx.CopySubresourceRegion(
                    &img.textura,
                    0,
                    0,
                    0,
                    0,
                    area_de_trabalho,
                    0,
                    Some(&caixa),
                );
            },
            Some(bruto) => {
                unsafe {
                    self.ctx.CopySubresourceRegion(
                        &bruto.textura,
                        0,
                        0,
                        0,
                        0,
                        area_de_trabalho,
                        0,
                        Some(&caixa),
                    );
                }
                self.passe(
                    &self.passes.entrada,
                    img.rtv.as_ref().unwrap(),
                    img.largura,
                    img.altura,
                    &[Some(bruto.srv.clone()), None, None],
                    Parametros {
                        tamanho_origem: [img.largura as i32, img.altura as i32],
                        rotacao: t.giro.codigo(),
                        ..Default::default()
                    },
                );
            }
        }
    }

    /// A luma de `img` em 1/4 (`q`) e 1/16 (`s`).
    fn luma_em(&self, img: &Alvo, q: &Alvo, s: &Alvo) {
        self.passe(
            &self.passes.luma,
            q.rtv.as_ref().unwrap(),
            q.largura,
            q.altura,
            &[Some(img.srv.clone()), None, None],
            Parametros {
                texel_origem: [1.0 / img.largura as f32, 1.0 / img.altura as f32],
                tamanho_aux: [0, 0],
                ..Default::default()
            },
        );
        self.passe(
            &self.passes.luma,
            s.rtv.as_ref().unwrap(),
            s.largura,
            s.altura,
            &[Some(q.srv.clone()), None, None],
            Parametros {
                texel_origem: [1.0 / q.largura as f32, 1.0 / q.altura as f32],
                tamanho_aux: [1, 1],
                ..Default::default()
            },
        );
    }

    /// O quadro capturado é igual ao último real (média da diferença de luma em 1/16 quase zero)? Num monitor girado
    /// o Windows entrega como "novo" a tela recomposta pela própria sobreposição; contar isso como quadro do jogo
    /// prendia o FPS no Hz do monitor e o gerador em 1×.
    pub fn igual_ao_atual(&self, area_de_trabalho: &ID3D11Texture2D, recorte: Retangulo) -> bool {
        // Só a cópia exata conta como repetida (a tela recomposta sem mudança dá a mesma luma, bit a bit): qualquer
        // mudança de verdade, até um dígito do HUD, passa disso.
        const LIMIAR: f32 = 0.000002;
        let Some(t) = self.trabalho.as_ref() else {
            return false;
        };
        if self.reais == 0 {
            return false;
        }
        self.copiar(t, &t.candidato, area_de_trabalho, recorte);
        self.luma_em(&t.candidato, &t.cand_q, &t.cand_s);
        self.passe(
            &self.passes.diferenca,
            t.dif.rtv.as_ref().unwrap(),
            t.dif.largura,
            t.dif.altura,
            &[
                Some(t.cand_q.srv.clone()),
                Some(t.luma_q[self.atual].srv.clone()),
                None,
            ],
            Parametros::default(),
        );
        unsafe {
            self.ctx.GenerateMips(&t.dif.srv);
            self.ctx.CopySubresourceRegion(
                &t.dif_leitura,
                0,
                0,
                0,
                0,
                &t.dif.textura,
                t.niveis_dif.saturating_sub(1),
                None,
            );
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            if self
                .ctx
                .Map(&t.dif_leitura, 0, D3D11_MAP_READ, 0, Some(&mut m))
                .is_err()
            {
                return false;
            }
            let v = meio_float(*(m.pData as *const u16));
            self.ctx.Unmap(&t.dif_leitura, 0);
            v < LIMIAR
        }
    }

    /// `recorte` nas coordenadas da TEXTURA capturada (`ritmo::na_textura`).
    pub fn receber(&mut self, area_de_trabalho: &ID3D11Texture2D, recorte: Retangulo) {
        let Some(t) = self.trabalho.as_ref() else {
            return;
        };
        let novo = if self.reais == 0 {
            self.atual
        } else {
            1 - self.atual
        };
        let img = &t.imagem[novo];
        self.copiar(t, img, area_de_trabalho, recorte);
        if self.sem_geracao {
            self.atual = novo;
            self.reais += 1;
            return;
        }
        let q = &t.luma_q[novo];
        let s = &t.luma_s[novo];
        self.luma_em(img, q, s);
        let g = &t.luma_g[novo];
        self.passe(
            &self.passes.luma,
            g.rtv.as_ref().unwrap(),
            g.largura,
            g.altura,
            &[Some(s.srv.clone()), None, None],
            Parametros {
                texel_origem: [1.0 / s.largura as f32, 1.0 / s.altura as f32],
                tamanho_aux: [1, 1],
                ..Default::default()
            },
        );
        // A luma em 1/2 (da imagem) e em 1/8 (da de 1/4), para a pirâmide do fluxo.
        if self.busca_por_blocos || self.dis {
            let m = &t.luma_m[novo];
            // O 1/2 só entra na busca por blocos e no DIS com 4 níveis.
            if self.busca_por_blocos || self.niveis_dis == 4 {
                self.passe(
                    &self.passes.luma_meia,
                    m.rtv.as_ref().unwrap(),
                    m.largura,
                    m.altura,
                    &[Some(img.srv.clone()), None, None],
                    Parametros {
                        texel_origem: [1.0 / img.largura as f32, 1.0 / img.altura as f32],
                        ..Default::default()
                    },
                );
            }
            let o = &t.luma_o[novo];
            self.passe(
                &self.passes.luma_meia,
                o.rtv.as_ref().unwrap(),
                o.largura,
                o.altura,
                &[Some(q.srv.clone()), None, None],
                Parametros {
                    texel_origem: [1.0 / q.largura as f32, 1.0 / q.altura as f32],
                    tamanho_aux: [1, 1],
                    ..Default::default()
                },
            );
        }
        if self.dis {
            let lumas = [
                &t.luma_s[novo],
                &t.luma_o[novo],
                &t.luma_q[novo],
                &t.luma_m[novo],
            ];
            for (luma, par) in lumas.iter().zip(t.luma_grad.iter()).take(self.niveis_dis) {
                let g = &par[novo];
                self.passe(
                    &self.passes.gradiente,
                    g.rtv.as_ref().unwrap(),
                    g.largura,
                    g.altura,
                    &[Some(luma.srv.clone()), None, None],
                    Parametros {
                        tamanho_origem: [luma.largura as i32, luma.altura as i32],
                        ..Default::default()
                    },
                );
            }
        }
        if self.reais >= 1 {
            let anterior = 1 - novo;
            let (de, para) = (t.mascara_atual, 1 - t.mascara_atual);
            let alvo = &t.mascara[para];
            self.passe(
                &self.passes.mascara,
                alvo.rtv.as_ref().unwrap(),
                alvo.largura,
                alvo.altura,
                &[
                    Some(t.imagem[anterior].srv.clone()),
                    Some(t.imagem[novo].srv.clone()),
                    Some(t.mascara[de].srv.clone()),
                ],
                Parametros {
                    texel_destino: [1.0 / t.largura as f32, 1.0 / t.altura as f32],
                    ..Default::default()
                },
            );
            if let Some(tr) = self.trabalho.as_mut() {
                tr.mascara_atual = para;
            }
        }
        self.atual = novo;
        self.reais += 1;
    }

    pub fn tem_par(&self) -> bool {
        self.reais >= 2
    }

    pub fn estimar(&self) {
        let Some(t) = self.trabalho.as_ref() else {
            return;
        };
        if !self.tem_par() {
            return;
        }
        let (a, b) = (1 - self.atual, self.atual);
        let lambda = 0.02;

        // O movimento da câmera, um vetor para a imagem toda (ver `Camera` no shader).
        let s16 = &t.luma_s[a];
        self.passe(
            &self.passes.camera,
            t.camera_notas.rtv.as_ref().unwrap(),
            t.camera_notas.largura,
            t.camera_notas.altura,
            &[
                Some(t.luma_s[a].srv.clone()),
                Some(t.luma_s[b].srv.clone()),
                None,
            ],
            Parametros {
                tamanho_origem: [s16.largura as i32, s16.altura as i32],
                ..Default::default()
            },
        );
        self.passe(
            &self.passes.camera_min,
            t.camera.rtv.as_ref().unwrap(),
            1,
            1,
            &[Some(t.camera_notas.srv.clone()), None, None],
            Parametros::default(),
        );

        if self.busca_por_blocos {
            self.fluxo_otico(t, a, b);
            return;
        }
        if self.dis {
            self.fluxo_denso(t, a, b);
            return;
        }

        let s = &t.luma_s[a];
        self.passe(
            &self.passes.grossa,
            t.vet_grosso.rtv.as_ref().unwrap(),
            t.vet_grosso.largura,
            t.vet_grosso.altura,
            &[
                Some(t.luma_s[a].srv.clone()),
                Some(t.luma_s[b].srv.clone()),
                Some(t.camera.srv.clone()),
            ],
            Parametros {
                tamanho_origem: [s.largura as i32, s.altura as i32],
                tamanho_aux: [1, 1],
                lambda,
                ..Default::default()
            },
        );

        let q = &t.luma_q[a];
        self.passe(
            &self.passes.fina,
            t.vet_fino.rtv.as_ref().unwrap(),
            t.vet_fino.largura,
            t.vet_fino.altura,
            &[
                Some(t.luma_q[a].srv.clone()),
                Some(t.luma_q[b].srv.clone()),
                Some(t.vet_grosso.srv.clone()),
            ],
            Parametros {
                tamanho_origem: [q.largura as i32, q.altura as i32],
                tamanho_aux: [t.vet_grosso.largura as i32, t.vet_grosso.altura as i32],
                lambda,
                ..Default::default()
            },
        );

        self.passe(
            &self.passes.suave,
            t.vet_suave.rtv.as_ref().unwrap(),
            t.vet_suave.largura,
            t.vet_suave.altura,
            &[Some(t.vet_fino.srv.clone()), None, None],
            Parametros {
                tamanho_origem: [t.vet_fino.largura as i32, t.vet_fino.altura as i32],
                ..Default::default()
            },
        );
    }

    /// O fluxo óptico em pirâmide (ver `Busca` no shader): cada nível parte do de cima (o primeiro, da câmera) e
    /// busca ±8 pixels do nível em volta; o último (1/2) passa pela mediana vetorial e vai para a grade de 1/8.
    fn fluxo_otico(&self, t: &Trabalho, a: usize, b: usize) {
        let niveis: [(&[Alvo; 2], f32); 4] = [
            (&t.luma_s, 16.0),
            (&t.luma_o, 8.0),
            (&t.luma_q, 4.0),
            (&t.luma_m, 2.0),
        ];
        for (i, (luma, escala)) in niveis.iter().enumerate() {
            let cima = if i == 0 { &t.camera } else { &t.fluxo[i - 1] };
            let saida = &t.fluxo[i];
            self.calcular(
                &self.passes.busca,
                saida,
                (saida.largura, saida.altura),
                &[
                    Some(luma[a].srv.clone()),
                    Some(luma[b].srv.clone()),
                    Some(cima.srv.clone()),
                ],
                Parametros {
                    tamanho_origem: [luma[a].largura as i32, luma[a].altura as i32],
                    tamanho_aux: [cima.largura as i32, cima.altura as i32],
                    lambda: *escala,
                    limiar_erro: if i == 3 { 1.0 } else { 0.0 },
                    ajuste_fluxo: ajuste_do_fluxo(),
                    ..Default::default()
                },
            );
        }
        let ultimo = &t.fluxo[3];
        self.passe(
            &self.passes.amplia,
            t.vet_amplo.rtv.as_ref().unwrap(),
            t.vet_amplo.largura,
            t.vet_amplo.altura,
            &[
                Some(ultimo.srv.clone()),
                Some(t.luma_m[a].srv.clone()),
                Some(t.luma_m[b].srv.clone()),
            ],
            Parametros {
                texel_origem: [
                    1.0 / t.luma_m[a].largura as f32,
                    1.0 / t.luma_m[a].altura as f32,
                ],
                tamanho_origem: [t.luma_m[a].largura as i32, t.luma_m[a].altura as i32],
                tamanho_aux: [ultimo.largura as i32, ultimo.altura as i32],
                ..Default::default()
            },
        );
        self.passe(
            &self.passes.suave,
            t.vet_suave.rtv.as_ref().unwrap(),
            t.vet_suave.largura,
            t.vet_suave.altura,
            &[Some(t.vet_amplo.srv.clone()), None, None],
            Parametros {
                tamanho_origem: [t.vet_amplo.largura as i32, t.vet_amplo.altura as i32],
                ..Default::default()
            },
        );
    }

    /// O fluxo denso por busca inversa (ver `DisPedaco` no shader), do 1/16 ao 1/2: o campo do meio e, em 1/8, os de ida e
    /// de volta, que dão a oclusão (`Consistencia`).
    fn fluxo_denso(&self, t: &Trabalho, a: usize, b: usize) {
        self.dis_em(t, a, b, 0, self.niveis_dis, &t.pedacos, &t.denso);
        if self.oclusao {
            self.dis_em(t, a, b, 1, 2, &t.pedacos_ida, &t.denso_ida);
            self.dis_em(t, a, b, 2, 2, &t.pedacos_volta, &t.denso_volta);
            let c = &t.consistencia;
            self.passe(
                &self.passes.consistencia,
                c.rtv.as_ref().unwrap(),
                c.largura,
                c.altura,
                &[
                    Some(t.denso_ida[1].srv.clone()),
                    Some(t.denso_volta[1].srv.clone()),
                    None,
                ],
                Parametros {
                    texel_destino: [1.0 / t.largura as f32, 1.0 / t.altura as f32],
                    tamanho_origem: [c.largura as i32, c.altura as i32],
                    ..Default::default()
                },
            );
            unsafe { self.ctx.GenerateMips(&c.srv) };
        }
    }

    /// Só para calibrar a troca de cena: a fração de A com par em B e a de B com par em A (o 1×1 da consistência).
    #[cfg(test)]
    pub fn casamento(&self) -> Option<(f32, f32)> {
        let tr = self.trabalho.as_ref()?;
        if !self.tem_par() || !self.oclusao {
            return None;
        }
        unsafe {
            self.ctx.CopySubresourceRegion(
                &tr.consistencia_leitura,
                0,
                0,
                0,
                0,
                &tr.consistencia.textura,
                tr.niveis_consistencia.saturating_sub(1),
                None,
            );
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            self.ctx
                .Map(&tr.consistencia_leitura, 0, D3D11_MAP_READ, 0, Some(&mut m))
                .ok()?;
            let bytes = std::slice::from_raw_parts(m.pData as *const u8, 8);
            let x = meio_float(u16::from_le_bytes([bytes[0], bytes[1]]));
            let y = meio_float(u16::from_le_bytes([bytes[2], bytes[3]]));
            self.ctx.Unmap(&tr.consistencia_leitura, 0);
            Some((x, y))
        }
    }

    /// Os `niveis` primeiros níveis do DIS numa `direcao` (0 simétrica, 1 de ida, 2 de volta), em `pedacos`/`denso`.
    #[allow(clippy::too_many_arguments)]
    fn dis_em(
        &self,
        t: &Trabalho,
        a: usize,
        b: usize,
        direcao: u8,
        niveis: usize,
        pedacos_por_nivel: &[Alvo],
        denso_por_nivel: &[Alvo],
    ) {
        let (amortecimento, iteracoes, folga) = ajuste_do_dis();
        let escalas = [16.0, 8.0, 4.0, 2.0];
        for i in 0..niveis {
            let (ga, gb) = (&t.luma_grad[i][a], &t.luma_grad[i][b]);
            let cima = if i == 0 {
                &t.camera
            } else {
                &denso_por_nivel[i - 1]
            };
            let texel = [1.0 / ga.largura as f32, 1.0 / ga.altura as f32];
            let tamanho = [ga.largura as i32, ga.altura as i32];
            let pedacos = &pedacos_por_nivel[i];
            self.calcular(
                &self.passes.dis_pedaco,
                pedacos,
                // Um grupo por pedaço.
                (pedacos.largura, pedacos.altura),
                &[
                    Some(ga.srv.clone()),
                    Some(gb.srv.clone()),
                    Some(cima.srv.clone()),
                    Some(t.camera.srv.clone()),
                ],
                Parametros {
                    texel_origem: texel,
                    tamanho_origem: tamanho,
                    lambda: escalas[i],
                    limiar_erro: raios_do_dis()[i],
                    faixa_erro: iteracoes,
                    ajuste_fluxo: [amortecimento, folga],
                    rotacao: direcao as f32,
                    extra: [if propagar() && i > 0 { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
                    ..Default::default()
                },
            );
            let denso = &denso_por_nivel[i];
            self.passe(
                &self.passes.dis_denso,
                denso.rtv.as_ref().unwrap(),
                denso.largura,
                denso.altura,
                &[
                    Some(ga.srv.clone()),
                    Some(gb.srv.clone()),
                    Some(pedacos.srv.clone()),
                ],
                Parametros {
                    texel_origem: texel,
                    tamanho_origem: tamanho,
                    tamanho_aux: [pedacos.largura as i32, pedacos.altura as i32],
                    lambda: escalas[i],
                    rotacao: direcao as f32,
                    ..Default::default()
                },
            );
        }
    }

    pub fn compor(
        &self,
        destino: &ID3D11RenderTargetView,
        largura: u32,
        altura: u32,
        t_gerado: Option<f32>,
    ) {
        let Some(tr) = self.trabalho.as_ref() else {
            return;
        };
        let (a, b) = (1 - self.atual, self.atual);
        let texel = [1.0 / largura as f32, 1.0 / altura as f32];
        match t_gerado {
            Some(t) if self.tem_par() && self.dis => {
                self.passe(
                    if self.oclusao {
                        &self.passes.final_bidirecional
                    } else {
                        &self.passes.final_denso
                    },
                    destino,
                    largura,
                    altura,
                    &[
                        Some(tr.imagem[a].srv.clone()),
                        Some(tr.imagem[b].srv.clone()),
                        Some(tr.denso[self.niveis_dis - 1].srv.clone()),
                        Some(tr.mascara[tr.mascara_atual].srv.clone()),
                        Some(tr.consistencia.srv.clone()),
                    ],
                    Parametros {
                        texel_destino: texel,
                        t,
                        limiar_erro: 0.04,
                        faixa_erro: 0.06,
                        ..Default::default()
                    },
                );
            }
            Some(t) if self.tem_par() && self.suave => {
                self.passe(
                    &self.passes.final_suave,
                    destino,
                    largura,
                    altura,
                    &[
                        Some(tr.imagem[a].srv.clone()),
                        Some(tr.imagem[b].srv.clone()),
                        Some(tr.vet_suave.srv.clone()),
                        Some(tr.mascara[tr.mascara_atual].srv.clone()),
                    ],
                    Parametros {
                        texel_destino: texel,
                        t,
                        limiar_erro: 0.04,
                        faixa_erro: 0.06,
                        ..Default::default()
                    },
                );
            }
            Some(t) if self.tem_par() => {
                self.passe(
                    &self.passes.escolha,
                    tr.escolhido.rtv.as_ref().unwrap(),
                    tr.escolhido.largura,
                    tr.escolhido.altura,
                    &[
                        Some(tr.imagem[a].srv.clone()),
                        Some(tr.imagem[b].srv.clone()),
                        Some(tr.vet_suave.srv.clone()),
                    ],
                    Parametros {
                        texel_destino: texel,
                        tamanho_aux: [tr.vet_suave.largura as i32, tr.vet_suave.altura as i32],
                        t,
                        ..Default::default()
                    },
                );
                self.passe(
                    &self.passes.final_,
                    destino,
                    largura,
                    altura,
                    &[
                        Some(tr.imagem[a].srv.clone()),
                        Some(tr.imagem[b].srv.clone()),
                        Some(tr.escolhido.srv.clone()),
                        Some(tr.mascara[tr.mascara_atual].srv.clone()),
                    ],
                    Parametros {
                        texel_destino: texel,
                        t,
                        limiar_erro: 0.04,
                        faixa_erro: 0.06,
                        ajuste_fluxo: if std::env::var_os("OTIMIZA_FG_VER_VETORES").is_some() {
                            [-1.0, 0.0]
                        } else {
                            [0.0, 0.0]
                        },
                        ..Default::default()
                    },
                );
            }
            _ => self.passe(
                &self.passes.copia,
                destino,
                largura,
                altura,
                &[None, Some(tr.imagem[b].srv.clone()), None],
                Parametros {
                    texel_destino: texel,
                    lambda: if inverter_para_diagnostico() {
                        -1.0
                    } else {
                        0.0
                    },
                    ..Default::default()
                },
            ),
        }
    }

    /// Escala: os alvos do quadro no tamanho do jogo e do quadro ampliado (`largura`×`altura`, a área do encaixe).
    /// Depois de `preparar`.
    pub fn preparar_ampliacao(&mut self, largura: u32, altura: u32) -> Result<(), String> {
        let Some(t) = self.trabalho.as_ref() else {
            return Err("escala antes de preparar o gerador".to_string());
        };
        let formato = if t.hdr {
            DXGI_FORMAT_R16G16B16A16_FLOAT
        } else {
            DXGI_FORMAT_B8G8R8A8_UNORM
        };
        let (tl, ta) = (t.largura, t.altura);
        if self.ampliacao.as_ref().is_some_and(|a| {
            (a.origem.largura, a.origem.altura, a.ampliado.largura, a.ampliado.altura)
                == (tl, ta, largura, altura)
        }) {
            return Ok(());
        }
        self.ampliacao = Some(Ampliacao {
            origem: self.alvo(tl, ta, formato, true)?,
            ampliado: self.alvo(largura, altura, formato, true)?,
        });
        Ok(())
    }

    /// `compor`, ampliado pelo FSR 1 para `area` do destino (o resto fica preto: as barras do encaixe). Sem
    /// `preparar_ampliacao`, compõe direto, como antes.
    pub fn compor_ampliado(
        &self,
        destino: &ID3D11RenderTargetView,
        area: Retangulo,
        t_gerado: Option<f32>,
    ) {
        let (Some(amp), Some(tr)) = (self.ampliacao.as_ref(), self.trabalho.as_ref()) else {
            self.compor(destino, area.largura as u32, area.altura as u32, t_gerado);
            return;
        };
        self.saida_codificada.set(true);
        self.compor(
            amp.origem.rtv.as_ref().unwrap(),
            tr.largura,
            tr.altura,
            t_gerado,
        );
        self.passe(
            &self.passes.easu,
            amp.ampliado.rtv.as_ref().unwrap(),
            amp.ampliado.largura,
            amp.ampliado.altura,
            &[Some(amp.origem.srv.clone())],
            Parametros {
                texel_destino: [
                    1.0 / amp.ampliado.largura as f32,
                    1.0 / amp.ampliado.altura as f32,
                ],
                tamanho_origem: [tr.largura as i32, tr.altura as i32],
                ..Default::default()
            },
        );
        self.saida_codificada.set(false);
        unsafe { self.ctx.ClearRenderTargetView(destino, &[0.0, 0.0, 0.0, 1.0]) };
        self.passe_na_area(
            &self.passes.rcas,
            destino,
            area,
            &[Some(amp.ampliado.srv.clone())],
            Parametros {
                tamanho_origem: [amp.ampliado.largura as i32, amp.ampliado.altura as i32],
                lambda: (-NITIDEZ_DA_ESCALA).exp2(),
                extra: [area.x as f32, area.y as f32, 0.0, 0.0],
                ..Default::default()
            },
        );
    }

    /// Escala: o último quadro composto no tamanho do jogo, antes de ampliar (no HDR, ainda codificado com gama).
    pub fn textura_da_origem(&self) -> Option<&ID3D11Texture2D> {
        self.ampliacao.as_ref().map(|a| &a.origem.textura)
    }

    /// Troca a forma do cursor (a captura só manda quando ela muda).
    pub fn definir_cursor(&mut self, forma: &super::cursor::Forma) -> Result<(), String> {
        // Pré-multiplica: o filtro linear da ampliação não puxa o preto dos pixels transparentes para a borda.
        let px: Vec<u8> = forma
            .px
            .chunks_exact(4)
            .flat_map(|p| {
                let a = p[3] as u32;
                let m = |c: u8| ((c as u32 * a + 127) / 255) as u8;
                [m(p[0]), m(p[1]), m(p[2]), p[3]]
            })
            .collect();
        let desc = D3D11_TEXTURE2D_DESC {
            Width: forma.largura,
            Height: forma.altura,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            ..Default::default()
        };
        let mut textura = None;
        unsafe {
            self.device.CreateTexture2D(
                &desc,
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: px.as_ptr() as *const _,
                    SysMemPitch: forma.largura * 4,
                    SysMemSlicePitch: 0,
                }),
                Some(&mut textura),
            )
        }
        .map_err(|e| erro("forma do cursor", e))?;
        let textura: ID3D11Texture2D = textura.ok_or("forma do cursor")?;
        let mut srv = None;
        unsafe {
            self.device
                .CreateShaderResourceView(&textura, None, Some(&mut srv))
        }
        .map_err(|e| erro("srv do cursor", e))?;
        self.forma_do_cursor = Some(Alvo {
            textura,
            rtv: None,
            srv: srv.ok_or("srv do cursor")?,
            uav: None,
            largura: forma.largura,
            altura: forma.altura,
        });
        Ok(())
    }

    /// O cursor ampliado em `onde` do destino, por cima do que já está lá.
    pub fn desenhar_cursor(&self, destino: &ID3D11RenderTargetView, onde: Retangulo) {
        let Some(forma) = self.forma_do_cursor.as_ref() else {
            return;
        };
        unsafe { self.ctx.OMSetBlendState(&self.mistura, None, 0xffff_ffff) };
        self.passe_na_area(
            &self.passes.cursor,
            destino,
            onde,
            &[Some(forma.srv.clone())],
            Parametros {
                extra: [
                    onde.x as f32,
                    onde.y as f32,
                    onde.largura as f32,
                    onde.altura as f32,
                ],
                ..Default::default()
            },
        );
        unsafe { self.ctx.OMSetBlendState(None, None, 0xffff_ffff) };
    }

    pub fn dxgi_device(&self) -> Result<windows::Win32::Graphics::Dxgi::IDXGIDevice, String> {
        self.device.cast().map_err(|e| erro("dxgi device", e))
    }
}

/// O TEMPO DE PLACA DO GERADOR, AO VIVO. O `custo_ms` do log media só o processador mandando o trabalho; a placa faz
/// depois, em paralelo com o jogo, e é ela que o jogo perde (no FiveM do dono a guarda desligou o gerador 7 vezes, com
/// o jogo perdendo de 5% a 35%). Carimbos de tempo da placa em volta de cada trabalho, lidos SEM ESPERAR: uma roda de
/// marcas; a que ainda não terminou fica para a próxima volta do laço (esperar a placa parava o jogo, `mod.rs`).
pub struct Cronometro {
    livres: Vec<Marca>,
    pendentes: std::collections::VecDeque<Marca>,
    /// Soma desde a última `tirar`, em ms de placa.
    soma_ms: f64,
    medidas: u64,
    /// Trabalhos que ficaram sem medir (todas as marcas ocupadas).
    pub sem_marca: u64,
}

struct Marca {
    disjunto: ID3D11Query,
    inicio: ID3D11Query,
    fim: ID3D11Query,
}

/// Quantos trabalhos podem estar em voo na placa ao mesmo tempo (estimar + até 6 composições por real).
const MARCAS_EM_VOO: usize = 16;

impl Cronometro {
    pub fn novo(gpu: &Gpu) -> Option<Cronometro> {
        let q = |tipo| -> Option<ID3D11Query> {
            let mut r = None;
            unsafe {
                gpu.device
                    .CreateQuery(
                        &D3D11_QUERY_DESC {
                            Query: tipo,
                            MiscFlags: 0,
                        },
                        Some(&mut r),
                    )
                    .ok()?;
            }
            r
        };
        let livres = (0..MARCAS_EM_VOO)
            .map(|_| {
                Some(Marca {
                    disjunto: q(D3D11_QUERY_TIMESTAMP_DISJOINT)?,
                    inicio: q(D3D11_QUERY_TIMESTAMP)?,
                    fim: q(D3D11_QUERY_TIMESTAMP)?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Cronometro {
            livres,
            pendentes: Default::default(),
            soma_ms: 0.0,
            medidas: 0,
            sem_marca: 0,
        })
    }

    /// Faz o trabalho entre dois carimbos da placa. Sem marca livre, faz sem medir (nunca segura o gerador).
    pub fn medir<R>(&mut self, gpu: &Gpu, trabalho: impl FnOnce() -> R) -> R {
        let Some(m) = self.livres.pop() else {
            self.sem_marca += 1;
            return trabalho();
        };
        unsafe {
            gpu.ctx.Begin(&m.disjunto);
            gpu.ctx.End(&m.inicio);
        }
        let r = trabalho();
        unsafe {
            gpu.ctx.End(&m.fim);
            gpu.ctx.End(&m.disjunto);
        }
        self.pendentes.push_back(m);
        r
    }

    /// Lê as marcas que a placa já terminou, sem esperar e sem forçar a fila (`DONOTFLUSH`).
    pub fn colher(&mut self, gpu: &Gpu) {
        const SEM_FORCAR: u32 = D3D11_ASYNC_GETDATA_DONOTFLUSH.0 as u32;
        while let Some(m) = self.pendentes.front() {
            let mut dj = D3D11_QUERY_DATA_TIMESTAMP_DISJOINT::default();
            // O `windows` trata S_FALSE ("ainda não") como sucesso sem escrever: o sentinela diz o que não chegou.
            let (mut t0, mut t1) = (u64::MAX, u64::MAX);
            let pronto = unsafe {
                gpu.ctx
                    .GetData(
                        &m.disjunto,
                        Some(&mut dj as *mut _ as *mut _),
                        std::mem::size_of_val(&dj) as u32,
                        SEM_FORCAR,
                    )
                    .is_ok_and(|_| dj.Frequency > 0)
                    && gpu
                        .ctx
                        .GetData(&m.inicio, Some(&mut t0 as *mut _ as *mut _), 8, SEM_FORCAR)
                        .is_ok()
                    && gpu
                        .ctx
                        .GetData(&m.fim, Some(&mut t1 as *mut _ as *mut _), 8, SEM_FORCAR)
                        .is_ok()
            };
            if !pronto || t0 == u64::MAX || t1 == u64::MAX {
                break;
            }
            if !dj.Disjoint.as_bool() && t1 >= t0 {
                self.soma_ms += (t1 - t0) as f64 * 1000.0 / dj.Frequency as f64;
                self.medidas += 1;
            }
            let m = self.pendentes.pop_front().expect("a frente existe");
            self.livres.push(m);
        }
    }

    /// O tempo de placa somado e quantos trabalhos, desde a última vez; zera.
    pub fn tirar(&mut self) -> (f64, u64) {
        let r = (self.soma_ms, self.medidas);
        self.soma_ms = 0.0;
        self.medidas = 0;
        r
    }
}

#[cfg(test)]
mod qualidade {
    //! O teste de qualidade do gerador, com quadros gravados do jogo (`gravador_de_quadros`): de N e N+2 o gerador
    //! cria o do meio, comparado pixel a pixel com o N+1 de verdade. Também mede o tempo de cada etapa na placa.
    //! `OTIMIZA_FG_QUADROS_DIR=<pasta> cargo test --lib qualidade -- --ignored --nocapture`
    use super::*;
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

    pub fn ler_bmp(caminho: &std::path::Path) -> (u32, u32, Vec<u8>) {
        let b = std::fs::read(caminho).unwrap();
        let l = i32::from_le_bytes(b[18..22].try_into().unwrap()) as u32;
        let a = i32::from_le_bytes(b[22..26].try_into().unwrap()).unsigned_abs();
        (l, a, b[54..54 + (l * a * 4) as usize].to_vec())
    }

    fn textura(gpu: &Gpu, l: u32, a: u32, px: &[u8]) -> ID3D11Texture2D {
        let mut t = None;
        unsafe {
            gpu.device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: l,
                        Height: a,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: px.as_ptr() as *const _,
                        SysMemPitch: l * 4,
                        SysMemSlicePitch: 0,
                    }),
                    Some(&mut t),
                )
                .unwrap();
        }
        t.unwrap()
    }

    fn ler_de_volta(gpu: &Gpu, t: &ID3D11Texture2D, l: u32, a: u32) -> Vec<u8> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { t.GetDesc(&mut desc) };
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.MiscFlags = 0;
        let mut c = None;
        unsafe {
            gpu.device
                .CreateTexture2D(&desc, None, Some(&mut c))
                .unwrap();
            let c = c.unwrap();
            gpu.ctx.CopyResource(&c, t);
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            gpu.ctx.Map(&c, 0, D3D11_MAP_READ, 0, Some(&mut m)).unwrap();
            let mut v = Vec::with_capacity((l * a * 4) as usize);
            for y in 0..a as usize {
                v.extend_from_slice(std::slice::from_raw_parts(
                    (m.pData as *const u8).add(y * m.RowPitch as usize),
                    (l * 4) as usize,
                ));
            }
            gpu.ctx.Unmap(&c, 0);
            v
        }
    }

    /// Os vetores (xy) de uma textura R16G16B16A16_FLOAT.
    fn ler_vetores(gpu: &Gpu, t: &ID3D11Texture2D, l: u32, a: u32) -> Vec<(f32, f32)> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { t.GetDesc(&mut desc) };
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.MiscFlags = 0;
        let mut c = None;
        unsafe {
            gpu.device
                .CreateTexture2D(&desc, None, Some(&mut c))
                .unwrap();
            let c = c.unwrap();
            gpu.ctx.CopyResource(&c, t);
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            gpu.ctx.Map(&c, 0, D3D11_MAP_READ, 0, Some(&mut m)).unwrap();
            let mut v = Vec::new();
            for y in 0..a as usize {
                let linha = std::slice::from_raw_parts(
                    (m.pData as *const u8).add(y * m.RowPitch as usize) as *const u16,
                    l as usize * 4,
                );
                for px in linha.chunks_exact(4) {
                    v.push((meio_float(px[0]), meio_float(px[1])));
                }
            }
            gpu.ctx.Unmap(&c, 0);
            v
        }
    }

    /// O deslocamento global entre dois quadros (em px da imagem), por força bruta numa versão 1/8 da luma. Pura.
    pub fn deslocamento_global(a: &[u8], b: &[u8], l: u32, al: u32) -> (i32, i32) {
        let (w, h) = ((l / 8) as i32, (al / 8) as i32);
        let reduz = |p: &[u8]| -> Vec<f32> {
            let mut v = vec![0f32; (w * h) as usize];
            for y in 0..h {
                for x in 0..w {
                    let i = (((y * 8) as u32 * l + (x * 8) as u32) * 4) as usize;
                    v[(y * w + x) as usize] =
                        0.114 * p[i] as f32 + 0.587 * p[i + 1] as f32 + 0.299 * p[i + 2] as f32;
                }
            }
            v
        };
        let (ra, rb) = (reduz(a), reduz(b));
        let mut melhor = (f32::MAX, (0, 0));
        for dy in -12..=12 {
            for dx in -40..=40 {
                let mut soma = 0f32;
                let mut n = 0;
                for y in (h / 4)..(h * 3 / 4) {
                    for x in (w / 4)..(w * 3 / 4) {
                        let (x2, y2) = (x + dx, y + dy);
                        if x2 >= 0 && y2 >= 0 && x2 < w && y2 < h {
                            soma += (ra[(y * w + x) as usize] - rb[(y2 * w + x2) as usize]).abs();
                            n += 1;
                        }
                    }
                }
                let m = soma / n.max(1) as f32;
                if m < melhor.0 {
                    melhor = (m, (dx * 8, dy * 8));
                }
            }
        }
        melhor.1
    }

    /// SSIM da luma em blocos 8×8 (média dos blocos). Diferente do PSNR, cai com imagem dobrada (fantasma). Pura.
    pub fn ssim(a: &[u8], b: &[u8], l: u32, al: u32) -> f64 {
        let y = |p: &[u8], i: usize| {
            0.114 * p[i] as f64 + 0.587 * p[i + 1] as f64 + 0.299 * p[i + 2] as f64
        };
        let (c1, c2) = ((0.01f64 * 255.0).powi(2), (0.03f64 * 255.0).powi(2));
        let (mut soma, mut n) = (0.0, 0);
        for by in (0..al as usize - 8).step_by(8) {
            for bx in (0..l as usize - 8).step_by(8) {
                let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0, 0.0, 0.0);
                for yy in 0..8 {
                    for xx in 0..8 {
                        let i = ((by + yy) * l as usize + bx + xx) * 4;
                        let (va, vb) = (y(a, i), y(b, i));
                        sa += va;
                        sb += vb;
                        saa += va * va;
                        sbb += vb * vb;
                        sab += va * vb;
                    }
                }
                let k = 64.0;
                let (ma, mb) = (sa / k, sb / k);
                let (va, vb, cov) = (saa / k - ma * ma, sbb / k - mb * mb, sab / k - ma * mb);
                soma += ((2.0 * ma * mb + c1) * (2.0 * cov + c2))
                    / ((ma * ma + mb * mb + c1) * (va + vb + c2));
                n += 1;
            }
        }
        soma / n.max(1) as f64
    }

    /// PSNR da luma, em dB. Pura.
    pub fn psnr(a: &[u8], b: &[u8]) -> f64 {
        let mut soma = 0.0f64;
        let mut n = 0usize;
        for (p, q) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
            let y = |c: &[u8]| 0.114 * c[0] as f64 + 0.587 * c[1] as f64 + 0.299 * c[2] as f64;
            let d = y(p) - y(q);
            soma += d * d;
            n += 1;
        }
        let mse = soma / n.max(1) as f64;
        if mse <= 1e-9 {
            99.0
        } else {
            10.0 * (255.0f64 * 255.0 / mse).log10()
        }
    }

    pub fn salvar_bmp(caminho: &std::path::Path, l: u32, a: u32, px: &[u8]) {
        let mut d = Vec::with_capacity(54 + px.len());
        d.extend_from_slice(b"BM");
        d.extend_from_slice(&((54 + px.len()) as u32).to_le_bytes());
        d.extend_from_slice(&[0, 0, 0, 0]);
        d.extend_from_slice(&54u32.to_le_bytes());
        d.extend_from_slice(&40u32.to_le_bytes());
        d.extend_from_slice(&(l as i32).to_le_bytes());
        d.extend_from_slice(&(-(a as i32)).to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&32u16.to_le_bytes());
        d.extend_from_slice(&[0u8; 24]);
        d.extend_from_slice(px);
        std::fs::write(caminho, d).unwrap();
    }

    struct Relogio {
        disjunto: ID3D11Query,
        marcas: Vec<ID3D11Query>,
    }

    impl Relogio {
        fn novo(gpu: &Gpu, n: usize) -> Relogio {
            let q = |tipo| {
                let mut r = None;
                unsafe {
                    gpu.device
                        .CreateQuery(
                            &D3D11_QUERY_DESC {
                                Query: tipo,
                                MiscFlags: 0,
                            },
                            Some(&mut r),
                        )
                        .unwrap()
                };
                r.unwrap()
            };
            Relogio {
                disjunto: q(D3D11_QUERY_TIMESTAMP_DISJOINT),
                marcas: (0..n).map(|_| q(D3D11_QUERY_TIMESTAMP)).collect(),
            }
        }

        /// Os tempos entre as marcas, em ms (espera a placa terminar).
        fn ler(&self, gpu: &Gpu) -> Vec<f64> {
            unsafe {
                let mut dj = D3D11_QUERY_DATA_TIMESTAMP_DISJOINT::default();
                while gpu
                    .ctx
                    .GetData(
                        &self.disjunto,
                        Some(&mut dj as *mut _ as *mut _),
                        std::mem::size_of_val(&dj) as u32,
                        0,
                    )
                    .is_err()
                    || dj.Frequency == 0
                {
                    std::thread::yield_now();
                }
                let t: Vec<u64> = self
                    .marcas
                    .iter()
                    .map(|q| {
                        let mut v = 0u64;
                        while gpu
                            .ctx
                            .GetData(q, Some(&mut v as *mut _ as *mut _), 8, 0)
                            .is_err()
                        {
                            std::thread::yield_now();
                        }
                        v
                    })
                    .collect();
                t.windows(2)
                    .map(|w| (w[1].saturating_sub(w[0])) as f64 * 1000.0 / dj.Frequency as f64)
                    .collect()
            }
        }
    }

    /// Uma cena com bordas finas (anéis, listras diagonais, retângulos), em cinza, `l`×`a`, com 4×4 amostras por pixel:
    /// a mesma cena em qualquer resolução, como o jogo renderizaria.
    fn cena_sintetica(l: u32, a: u32) -> Vec<u8> {
        let valor = |u: f64, v: f64| -> f64 {
            if u < 0.5 {
                let r = ((u - 0.25).powi(2) + ((v - 0.5) * 9.0 / 16.0).powi(2)).sqrt();
                if (r * 80.0).fract() < 0.5 { 0.9 } else { 0.1 }
            } else if v < 0.5 {
                if ((u + v) * 24.0).fract() < 0.5 { 0.85 } else { 0.15 }
            } else {
                let dentro = (u * 10.0).fract() < 0.6 && (v * 7.0).fract() < 0.6;
                if dentro { 0.8 } else { 0.2 }
            }
        };
        let mut px = Vec::with_capacity((l * a * 4) as usize);
        for y in 0..a {
            for x in 0..l {
                let mut s = 0.0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let u = (x as f64 + (sx as f64 + 0.5) / 4.0) / l as f64;
                        let v = (y as f64 + (sy as f64 + 0.5) / 4.0) / a as f64;
                        s += valor(u, v);
                    }
                }
                let c = (s / 16.0 * 255.0).round() as u8;
                px.extend_from_slice(&[c, c, c, 255]);
            }
        }
        px
    }

    /// Ampliação bilinear na CPU, com o mesmo mapeamento de centro de pixel do `Easu`. Pura.
    fn bilinear(px: &[u8], l: u32, a: u32, nl: u32, na: u32) -> Vec<u8> {
        let ler = |x: i64, y: i64| -> f64 {
            let x = x.clamp(0, l as i64 - 1) as usize;
            let y = y.clamp(0, a as i64 - 1) as usize;
            px[(y * l as usize + x) * 4] as f64
        };
        let mut v = Vec::with_capacity((nl * na * 4) as usize);
        for y in 0..na {
            for x in 0..nl {
                let fx = (x as f64 + 0.5) * l as f64 / nl as f64 - 0.5;
                let fy = (y as f64 + 0.5) * a as f64 / na as f64 - 0.5;
                let (x0, y0) = (fx.floor(), fy.floor());
                let (tx, ty) = (fx - x0, fy - y0);
                let (x0, y0) = (x0 as i64, y0 as i64);
                let c = ler(x0, y0) * (1.0 - tx) * (1.0 - ty)
                    + ler(x0 + 1, y0) * tx * (1.0 - ty)
                    + ler(x0, y0 + 1) * (1.0 - tx) * ty
                    + ler(x0 + 1, y0 + 1) * tx * ty;
                let c = c.round() as u8;
                v.extend_from_slice(&[c, c, c, 255]);
            }
        }
        v
    }

    fn alvo_bgra(gpu: &Gpu, l: u32, a: u32) -> (ID3D11Texture2D, ID3D11RenderTargetView) {
        let mut t = None;
        let mut rtv = None;
        unsafe {
            gpu.device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: l,
                        Height: a,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
                        ..Default::default()
                    },
                    None,
                    Some(&mut t),
                )
                .unwrap();
            let t: ID3D11Texture2D = t.unwrap();
            gpu.device
                .CreateRenderTargetView(&t, None, Some(&mut rtv))
                .unwrap();
            (t, rtv.unwrap())
        }
    }

    /// A ESCALA NA PLACA. A cena renderizada em 640×360 e ampliada para 960×540 (o nível Qualidade) tem de ficar mais
    /// perto da mesma cena renderizada em 960×540 pelo FSR 1 do que pelo bilinear; com barras, o que fica fora do
    /// encaixe sai preto. `OTIMIZA_FG_QUADROS_SAIDA=<pasta>` grava as imagens para olhar.
    /// `cargo test --lib -- --ignored escala_na_placa --nocapture`
    #[test]
    #[ignore]
    fn escala_na_placa() {
        let (bl, ba) = (640u32, 360u32);
        let (nl, na) = (960u32, 540u32);
        let baixa = cena_sintetica(bl, ba);
        let nativa = cena_sintetica(nl, na);

        let fabrica: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.unwrap();
        let adaptador = unsafe { fabrica.EnumAdapters1(0) }.unwrap();
        let mut gpu = Gpu::novo(&adaptador).unwrap();
        gpu.sem_geracao = true;
        gpu.preparar(bl, ba, false, Giro::Nenhum).unwrap();
        gpu.preparar_ampliacao(nl, na).unwrap();
        let entrada = textura(&gpu, bl, ba, &baixa);
        gpu.receber(
            &entrada,
            Retangulo {
                x: 0,
                y: 0,
                largura: bl as i32,
                altura: ba as i32,
            },
        );

        let (saida, rtv) = alvo_bgra(&gpu, nl, na);
        gpu.compor_ampliado(
            &rtv,
            Retangulo {
                x: 0,
                y: 0,
                largura: nl as i32,
                altura: na as i32,
            },
            None,
        );
        let fsr = ler_de_volta(&gpu, &saida, nl, na);
        let lin = bilinear(&baixa, bl, ba, nl, na);
        let (s_fsr, s_lin) = (ssim(&fsr, &nativa, nl, na), ssim(&lin, &nativa, nl, na));
        let (p_fsr, p_lin) = (psnr(&fsr, &nativa), psnr(&lin, &nativa));
        println!(
            "escala 640x360 -> 960x540: FSR SSIM {:.4} PSNR {:.2} dB · bilinear SSIM {:.4} PSNR {:.2} dB",
            s_fsr, p_fsr, s_lin, p_lin
        );
        if let Ok(pasta) = std::env::var("OTIMIZA_FG_QUADROS_SAIDA") {
            let pasta = std::path::PathBuf::from(pasta);
            salvar_bmp(&pasta.join("escala_fsr.bmp"), nl, na, &fsr);
            salvar_bmp(&pasta.join("escala_bilinear.bmp"), nl, na, &lin);
            salvar_bmp(&pasta.join("escala_nativa.bmp"), nl, na, &nativa);
        }
        assert!(s_fsr > s_lin, "o FSR tem de vencer o bilinear");

        // O comparador lê o quadro do jogo antes de ampliar: tem de ser a própria imagem do jogo.
        let (ol, oa, origem) =
            super::super::ler_bgra(&gpu, gpu.textura_da_origem().unwrap(), true).unwrap();
        assert_eq!((ol, oa), (bl, ba));
        let diferente = origem
            .iter()
            .zip(baixa.iter())
            .filter(|(a, b)| a.abs_diff(**b) > 1)
            .count();
        assert_eq!(diferente, 0, "o quadro de origem do comparador é o do jogo");

        // Barras: a mesma imagem num alvo mais largo, encaixada no meio.
        let (ll, la) = (nl + 140, na);
        gpu.preparar_ampliacao(nl, na).unwrap();
        let (larga, rtv_larga) = alvo_bgra(&gpu, ll, la);
        gpu.compor_ampliado(
            &rtv_larga,
            Retangulo {
                x: 70,
                y: 0,
                largura: nl as i32,
                altura: na as i32,
            },
            None,
        );
        let px = ler_de_volta(&gpu, &larga, ll, la);
        let em = |x: u32, y: u32| px[((y * ll + x) * 4) as usize];
        for y in [0, la / 2, la - 1] {
            assert_eq!(em(10, y), 0, "barra da esquerda");
            assert_eq!(em(ll - 10, y), 0, "barra da direita");
        }
        let dentro: Vec<u8> = (0..nl)
            .map(|x| em(70 + x, la / 2))
            .collect();
        let igual_ao_sem_barra: Vec<u8> = (0..nl)
            .map(|x| fsr[(((la / 2) * nl + x) * 4) as usize])
            .collect();
        assert_eq!(dentro, igual_ao_sem_barra, "a área do encaixe é a mesma imagem");
    }

    /// O CURSOR AMPLIADO NA PLACA. Uma forma 4×4 (metade de cima vermelha opaca, de baixo transparente) desenhada 8×
    /// maior por cima da imagem cinza: o vermelho cobre, o transparente deixa a imagem como estava.
    /// `cargo test --lib -- --ignored cursor_na_placa --nocapture`
    #[test]
    #[ignore]
    fn cursor_na_placa() {
        let (l, a) = (128u32, 128u32);
        let cinza = vec![128u8; (l * a * 4) as usize];
        let fabrica: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.unwrap();
        let adaptador = unsafe { fabrica.EnumAdapters1(0) }.unwrap();
        let mut gpu = Gpu::novo(&adaptador).unwrap();
        gpu.sem_geracao = true;
        gpu.preparar(l, a, false, Giro::Nenhum).unwrap();
        gpu.preparar_ampliacao(l, a).unwrap();
        let entrada = textura(&gpu, l, a, &cinza);
        let tudo = Retangulo {
            x: 0,
            y: 0,
            largura: l as i32,
            altura: a as i32,
        };
        gpu.receber(&entrada, tudo);

        let mut px = Vec::new();
        for y in 0..4 {
            for _ in 0..4 {
                px.extend_from_slice(if y < 2 { &[0, 0, 255, 255] } else { &[0, 0, 0, 0] });
            }
        }
        gpu.definir_cursor(&super::super::cursor::Forma {
            largura: 4,
            altura: 4,
            px,
            ponta: (0, 0),
        })
        .unwrap();

        let (saida, rtv) = alvo_bgra(&gpu, l, a);
        gpu.compor_ampliado(&rtv, tudo, None);
        let onde = Retangulo {
            x: 40,
            y: 40,
            largura: 32,
            altura: 32,
        };
        gpu.desenhar_cursor(&rtv, onde);
        let v = ler_de_volta(&gpu, &saida, l, a);
        let em = |x: u32, y: u32| {
            let i = ((y * l + x) * 4) as usize;
            [v[i], v[i + 1], v[i + 2]]
        };
        assert_eq!(em(50, 45), [0, 0, 255], "metade de cima do cursor: vermelho");
        let baixo = em(50, 66);
        assert!(baixo.iter().all(|c| c.abs_diff(128) <= 2), "metade de baixo: a imagem por baixo ({:?})", baixo);
        let fora = em(10, 10);
        assert!(fora.iter().all(|c| c.abs_diff(128) <= 2), "fora do cursor: intacta ({:?})", fora);
    }

    /// Calibra a troca de cena: pares seguidos do FiveM gravado × o mesmo quadro contra ele espelhado (conteúdo
    /// parecido, nada no lugar — o que um corte de câmera faz). `OTIMIZA_FG_QUADROS_DIR=... cargo test --lib -- --ignored
    /// casamento_nos_quadros_gravados --nocapture`.
    #[test]
    #[ignore]
    fn casamento_nos_quadros_gravados() {
        let pasta = std::path::PathBuf::from(
            std::env::var("OTIMIZA_FG_QUADROS_DIR").expect("OTIMIZA_FG_QUADROS_DIR"),
        );
        let mut arquivos: Vec<_> = std::fs::read_dir(&pasta)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "bmp"))
            .collect();
        arquivos.sort();
        let fabrica: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.unwrap();
        let adaptador = unsafe { fabrica.EnumAdapters1(0) }.unwrap();
        let mut gpu = Gpu::novo(&adaptador).unwrap();
        let (l, a, _) = ler_bmp(&arquivos[0]);
        gpu.preparar(l, a, false, Giro::Nenhum).unwrap();
        let tudo = Retangulo {
            x: 0,
            y: 0,
            largura: l as i32,
            altura: a as i32,
        };
        let espelhar = |px: &[u8]| -> Vec<u8> {
            let mut v = px.to_vec();
            for linha in v.chunks_mut(l as usize * 4) {
                let n = linha.len() / 4;
                for i in 0..n / 2 {
                    for c in 0..4 {
                        linha.swap(i * 4 + c, (n - 1 - i) * 4 + c);
                    }
                }
            }
            v
        };
        let (mut seguidos, mut cortes) = (Vec::new(), Vec::new());
        for i in (0..arquivos.len() - 1).step_by(2) {
            let (_, _, p0) = ler_bmp(&arquivos[i]);
            let (_, _, p1) = ler_bmp(&arquivos[i + 1]);
            gpu.reais = 0;
            gpu.receber(&textura(&gpu, l, a, &p0), tudo);
            gpu.receber(&textura(&gpu, l, a, &p1), tudo);
            gpu.estimar();
            let (x, y) = gpu.casamento().unwrap();
            seguidos.push(x.min(y));
            gpu.reais = 0;
            gpu.receber(&textura(&gpu, l, a, &p0), tudo);
            gpu.receber(&textura(&gpu, l, a, &espelhar(&p1)), tudo);
            gpu.estimar();
            let (x, y) = gpu.casamento().unwrap();
            cortes.push(x.min(y));
        }
        // O corte de verdade: um quadro do jogo seguido de uma imagem que não tem nada dele (`OTIMIZA_FG_CORTE_DIR`).
        let mut outras = Vec::new();
        if let Ok(d) = std::env::var("OTIMIZA_FG_CORTE_DIR") {
            let mut v: Vec<_> = std::fs::read_dir(d)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "bmp"))
                .collect();
            v.sort();
            for (k, outra) in v.iter().enumerate() {
                let (_, _, p0) = ler_bmp(&arquivos[k * 7 % arquivos.len()]);
                let (_, _, p1) = ler_bmp(outra);
                gpu.reais = 0;
                gpu.receber(&textura(&gpu, l, a, &p0), tudo);
                gpu.receber(&textura(&gpu, l, a, &p1), tudo);
                gpu.estimar();
                let (x, y) = gpu.casamento().unwrap();
                outras.push(x.min(y));
                // No corte, o quadro gerado em t = 0,3 tem de ser o real de antes, não uma mistura das duas cenas.
                let destino = gpu.alvo(l, a, DXGI_FORMAT_B8G8R8A8_UNORM, true).unwrap();
                gpu.compor(destino.rtv.as_ref().unwrap(), l, a, Some(0.3));
                let gerado = ler_de_volta(&gpu, &destino.textura, l, a);
                println!(
                    "corte {}: gerado × real de antes = {:.1} dB",
                    k,
                    psnr(&gerado, &p0)
                );
            }
        }
        if !outras.is_empty() {
            outras.sort_by(|a, b| a.partial_cmp(b).unwrap());
            println!("corte de verdade = {:?}", outras);
        }
        let resumo = |v: &mut Vec<f32>| {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            (v[0], v[v.len() / 2], v[v.len() - 1])
        };
        println!("seguidos (min, mediana, max) = {:?}", resumo(&mut seguidos));
        println!("cortes   (min, mediana, max) = {:?}", resumo(&mut cortes));
    }

    #[test]
    #[ignore]
    fn qualidade_nos_quadros_gravados() {
        let pasta = std::path::PathBuf::from(
            std::env::var("OTIMIZA_FG_QUADROS_DIR").expect("OTIMIZA_FG_QUADROS_DIR"),
        );
        let mut arquivos: Vec<_> = std::fs::read_dir(&pasta)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension().is_some_and(|x| x == "bmp")
                    && !p.file_name().unwrap().to_string_lossy().starts_with('_')
            })
            .collect();
        arquivos.sort();
        let fabrica: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.unwrap();
        let adaptador = unsafe { fabrica.EnumAdapters1(0) }.unwrap();
        let mut gpu = Gpu::novo(&adaptador).unwrap();
        let (l, a, _) = ler_bmp(&arquivos[0]);
        gpu.preparar(l, a, false, Giro::Nenhum).unwrap();
        let destino = gpu.alvo(l, a, DXGI_FORMAT_B8G8R8A8_UNORM, true).unwrap();
        let tudo = Retangulo {
            x: 0,
            y: 0,
            largura: l as i32,
            altura: a as i32,
        };
        let (mut g, mut rep, mut med, mut n) = (0.0, 0.0, 0.0, 0);
        let mut oraculo_soma = 0.0;
        let (mut ssim_g, mut ssim_m, mut ssim_o, mut ssim_r) = (0.0, 0.0, 0.0, 0.0);
        let mut tempos = vec![0.0f64; 4];
        let mut notas = Vec::new();
        let salvar = std::env::var("OTIMIZA_FG_SALVAR")
            .ok()
            .and_then(|v| v.parse::<usize>().ok());
        // O instante real de cada quadro: a gravação não sai em intervalos iguais, e o do meio nem sempre está em t = 0,5.
        let instantes: Vec<f64> = std::fs::read_to_string(pasta.join("tempos.txt"))
            .map(|t| {
                t.lines()
                    .filter_map(|l| l.split_whitespace().nth(1)?.parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        let mut i = 0;
        while i + 2 < arquivos.len() {
            let tm = if instantes.len() > i + 2 && instantes[i + 2] > instantes[i] {
                ((instantes[i + 1] - instantes[i]) / (instantes[i + 2] - instantes[i])) as f32
            } else {
                0.5
            };
            let (_, _, p0) = ler_bmp(&arquivos[i]);
            let (_, _, p1) = ler_bmp(&arquivos[i + 1]);
            let (_, _, p2) = ler_bmp(&arquivos[i + 2]);
            let (t0, t2) = (textura(&gpu, l, a, &p0), textura(&gpu, l, a, &p2));
            gpu.reais = 0;
            gpu.receber(&t0, tudo);
            let rel = Relogio::novo(&gpu, 5);
            unsafe {
                gpu.ctx.Begin(&rel.disjunto);
                gpu.ctx.End(&rel.marcas[0]);
            }
            gpu.receber(&t2, tudo);
            unsafe { gpu.ctx.End(&rel.marcas[1]) };
            gpu.estimar();
            unsafe { gpu.ctx.End(&rel.marcas[2]) };
            let nota = gpu.fracao_ruim();
            unsafe { gpu.ctx.End(&rel.marcas[3]) };
            gpu.compor(destino.rtv.as_ref().unwrap(), l, a, Some(tm));
            unsafe {
                gpu.ctx.End(&rel.marcas[4]);
                gpu.ctx.End(&rel.disjunto);
            }
            for (k, ms) in rel.ler(&gpu).into_iter().enumerate() {
                tempos[k] += ms;
            }
            let gerado = ler_de_volta(&gpu, &destino.textura, l, a);
            let media: Vec<u8> = p0
                .iter()
                .zip(&p2)
                .map(|(x, y)| (*x as f32 * (1.0 - tm) + *y as f32 * tm).round() as u8)
                .collect();
            let (pg, pr, pm) = (psnr(&gerado, &p1), psnr(&p0, &p1), psnr(&media, &p1));
            // Teto do modelo "câmera": desloca pelo movimento global REAL (força bruta na CPU) e mistura.
            let (vx, vy) = deslocamento_global(&p0, &p2, l, a);
            let oraculo: Vec<u8> = {
                let mut o = vec![0u8; p0.len()];
                let (ax, ay) = (
                    (vx as f32 * tm).round() as i32,
                    (vy as f32 * tm).round() as i32,
                );
                let (bx, by) = (
                    (vx as f32 * (1.0 - tm)).round() as i32,
                    (vy as f32 * (1.0 - tm)).round() as i32,
                );
                for y in 0..a as i32 {
                    for x in 0..l as i32 {
                        let pa = ((y - ay).clamp(0, a as i32 - 1) * l as i32
                            + (x - ax).clamp(0, l as i32 - 1))
                            as usize
                            * 4;
                        let pb = ((y + by).clamp(0, a as i32 - 1) * l as i32
                            + (x + bx).clamp(0, l as i32 - 1))
                            as usize
                            * 4;
                        let d = (y * l as i32 + x) as usize * 4;
                        for c in 0..4 {
                            o[d + c] = (p0[pa + c] as f32 * (1.0 - tm) + p2[pb + c] as f32 * tm)
                                .round() as u8;
                        }
                    }
                }
                o
            };
            let po = psnr(&oraculo, &p1);
            ssim_g += ssim(&gerado, &p1, l, a);
            ssim_m += ssim(&media, &p1, l, a);
            ssim_o += ssim(&oraculo, &p1, l, a);
            ssim_r += ssim(&p0, &p1, l, a);
            oraculo_soma += po;
            println!("    câmera real ({}, {}) → {:.2} dB", vx, vy, po);
            println!(
                "{:02}: t {:.2} · gerado {:.2} dB · repetir {:.2} · misturar {:.2} · nota {:?}",
                i, tm, pg, pr, pm, nota
            );
            if salvar == Some(i) {
                // Os vetores da placa (1/8) contra o deslocamento global real, achado na força bruta na CPU.
                let tr = gpu.trabalho.as_ref().unwrap();
                for (nome, alvo) in [
                    ("camera", &tr.camera),
                    ("grosso", &tr.vet_grosso),
                    ("fino", &tr.vet_fino),
                    ("suave", &tr.vet_suave),
                ] {
                    let v = ler_vetores(&gpu, &alvo.textura, alvo.largura, alvo.altura);
                    let mut xs: Vec<f32> = v.iter().map(|p| p.0).collect();
                    let mut ys: Vec<f32> = v.iter().map(|p| p.1).collect();
                    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let zeros = v.iter().filter(|p| p.0 == 0.0 && p.1 == 0.0).count();
                    println!(
                        "  vetores {} ({}x{}): mediana ({:.1}, {:.1}) · p10 x {:.1} · p90 x {:.1} · zerados {:.0}%",
                        nome, alvo.largura, alvo.altura, xs[xs.len() / 2], ys[ys.len() / 2], xs[xs.len() / 10], xs[xs.len() * 9 / 10],
                        zeros as f64 * 100.0 / v.len() as f64
                    );
                }
                println!(
                    "  deslocamento global real (CPU): {:?}",
                    deslocamento_global(&p0, &p2, l, a)
                );
                salvar_bmp(&pasta.join(format!("_gerado_{:02}.bmp", i)), l, a, &gerado);
            }
            g += pg;
            rep += pr;
            med += pm;
            notas.push(nota.unwrap_or(1.0));
            n += 1;
            i += 2;
        }
        let n = n as f64;
        println!(
            "MÉDIA: gerado {:.2} dB · câmera real {:.2} dB · repetir {:.2} dB · misturar {:.2} dB · nota média {:.3}",
            g / n,
            oraculo_soma / n,
            rep / n,
            med / n,
            notas.iter().sum::<f32>() / notas.len() as f32
        );
        println!(
            "SSIM: gerado {:.4} · câmera real {:.4} · repetir {:.4} · misturar {:.4}",
            ssim_g / n,
            ssim_o / n,
            ssim_r / n,
            ssim_m / n
        );
        println!(
            "PLACA por quadro real: receber {:.2} ms · estimar {:.2} ms · nota {:.2} ms · compor 1 gerado {:.2} ms",
            tempos[0] / n,
            tempos[1] / n,
            tempos[2] / n,
            tempos[3] / n
        );
    }
}
