// A parte Direct3D 11 do gerador: texturas de trabalho, passes e composição.
//
// Um objeto `Gpu` vive dentro da linha de execução do gerador e nunca sai
// dela. As texturas do quadro "anterior" e "atual" alternam de papel a cada
// quadro real (índice `atual`), para não copiar imagem inteira à toa.

use windows::core::{Interface, PCSTR};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::Fxc::{D3DCompile, D3DCOMPILE_OPTIMIZATION_LEVEL3};
use windows::Win32::Graphics::Direct3D::{ID3DBlob, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::IDXGIAdapter1;

use super::ritmo::{niveis, Retangulo};

const FONTE: &str = include_str!("shaders.hlsl");

/// Diagnóstico: `OTIMIZA_FG_INVERTER` inverte as cores do quadro real, para
/// provar que a sobreposição está na tela. Lido uma vez.
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
}

pub struct Alvo {
    pub textura: ID3D11Texture2D,
    pub rtv: Option<ID3D11RenderTargetView>,
    pub srv: ID3D11ShaderResourceView,
    pub largura: u32,
    pub altura: u32,
}

struct Passes {
    vs: ID3D11VertexShader,
    luma: ID3D11PixelShader,
    grossa: ID3D11PixelShader,
    fina: ID3D11PixelShader,
    suave: ID3D11PixelShader,
    escolha: ID3D11PixelShader,
    final_: ID3D11PixelShader,
    copia: ID3D11PixelShader,
}

struct Trabalho {
    largura: u32,
    altura: u32,
    imagem: [Alvo; 2],
    luma_q: [Alvo; 2],
    luma_s: [Alvo; 2],
    vet_grosso: Alvo,
    vet_fino: Alvo,
    vet_suave: Alvo,
    /// Vetor escolhido e discordância, em meia resolução. Refeito a cada
    /// quadro gerado, porque depende de `t`.
    escolhido: Alvo,
}

pub struct Gpu {
    pub device: ID3D11Device,
    pub ctx: ID3D11DeviceContext,
    passes: Passes,
    amostrador: ID3D11SamplerState,
    buffer: ID3D11Buffer,
    trabalho: Option<Trabalho>,
    /// Índice do quadro real mais novo em `imagem`, `luma_q`, `luma_s`.
    atual: usize,
    /// Quantos quadros reais já chegaram desde a criação dos recursos.
    pub reais: u64,
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
                String::from_utf8_lossy(std::slice::from_raw_parts(m.GetBufferPointer() as *const u8, m.GetBufferSize())).to_string()
            })
            .unwrap_or_default();
        return Err(format!("shader {} não compilou: {} {}", entrada, e.message(), detalhe));
    }
    codigo.ok_or_else(|| format!("shader {} sem código", entrada))
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
            unsafe { device.CreatePixelShader(bytes(&b), None, Some(&mut s)) }.map_err(|e| erro(nome, e))?;
            s.ok_or_else(|| nome.to_string())
        };
        let vs_b = compilar("VS", "vs_5_0")?;
        let mut vs = None;
        unsafe { device.CreateVertexShader(bytes(&vs_b), None, Some(&mut vs)) }.map_err(|e| erro("VS", e))?;

        let passes = Passes {
            vs: vs.ok_or("VS")?,
            luma: ps("Luma")?,
            grossa: ps("Grossa")?,
            fina: ps("Fina")?,
            suave: ps("Suave")?,
            escolha: ps("Escolha")?,
            final_: ps("Final")?,
            copia: ps("Copia")?,
        };

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
        })
    }

    fn alvo(&self, largura: u32, altura: u32, formato: DXGI_FORMAT, render: bool) -> Result<Alvo, String> {
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
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: bind,
            ..Default::default()
        };
        let mut textura = None;
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut textura)) }.map_err(|e| erro("textura", e))?;
        let textura: ID3D11Texture2D = textura.ok_or("textura")?;
        let mut srv = None;
        unsafe { self.device.CreateShaderResourceView(&textura, None, Some(&mut srv)) }.map_err(|e| erro("srv", e))?;
        let rtv = if render {
            let mut r = None;
            unsafe { self.device.CreateRenderTargetView(&textura, None, Some(&mut r)) }.map_err(|e| erro("rtv", e))?;
            r
        } else {
            None
        };
        Ok(Alvo { textura, rtv, srv: srv.ok_or("srv")?, largura, altura })
    }

    /// Cria (ou recria) as texturas para uma área de jogo deste tamanho.
    pub fn preparar(&mut self, largura: u32, altura: u32) -> Result<(), String> {
        if self.trabalho.as_ref().is_some_and(|t| t.largura == largura && t.altura == altura) {
            return Ok(());
        }
        let ((ql, qa), (fl, fa), (sl, sa)) = niveis(largura as i32, altura as i32);
        let luma = DXGI_FORMAT_R16_FLOAT;
        let vet = DXGI_FORMAT_R16G16B16A16_FLOAT;
        self.trabalho = Some(Trabalho {
            largura,
            altura,
            imagem: [
                self.alvo(largura, altura, DXGI_FORMAT_B8G8R8A8_UNORM, false)?,
                self.alvo(largura, altura, DXGI_FORMAT_B8G8R8A8_UNORM, false)?,
            ],
            luma_q: [self.alvo(ql, qa, luma, true)?, self.alvo(ql, qa, luma, true)?],
            luma_s: [self.alvo(sl, sa, luma, true)?, self.alvo(sl, sa, luma, true)?],
            vet_grosso: self.alvo(sl, sa, vet, true)?,
            vet_fino: self.alvo(fl, fa, vet, true)?,
            vet_suave: self.alvo(fl, fa, vet, true)?,
            escolhido: self.alvo((largura + 1) / 2, (altura + 1) / 2, vet, true)?,
        });
        self.reais = 0;
        Ok(())
    }

    fn parametros(&self, p: Parametros) {
        unsafe {
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            if self.ctx.Map(&self.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m)).is_ok() {
                std::ptr::copy_nonoverlapping(&p as *const Parametros as *const u8, m.pData as *mut u8, std::mem::size_of::<Parametros>());
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
        entradas: &[Option<ID3D11ShaderResourceView>; 3],
        p: Parametros,
    ) {
        self.parametros(p);
        unsafe {
            self.ctx.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
            self.ctx.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: largura as f32,
                Height: altura as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
            self.ctx.IASetPrimitiveTopology(windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.ctx.IASetInputLayout(None);
            self.ctx.VSSetShader(&self.passes.vs, None);
            self.ctx.PSSetShader(ps, None);
            self.ctx.PSSetShaderResources(0, Some(entradas));
            self.ctx.PSSetSamplers(0, Some(&[Some(self.amostrador.clone())]));
            self.ctx.PSSetConstantBuffers(0, Some(&[Some(self.buffer.clone())]));
            self.ctx.Draw(3, 0);
            // Desliga as entradas: a mesma textura vira alvo no passe seguinte.
            self.ctx.PSSetShaderResources(0, Some(&[None, None, None]));
            self.ctx.OMSetRenderTargets(None, None);
        }
    }

    /// Um quadro real chegou: copia o recorte e calcula a luma dele.
    pub fn receber(&mut self, area_de_trabalho: &ID3D11Texture2D, recorte: Retangulo) {
        let Some(t) = self.trabalho.as_ref() else { return };
        let novo = if self.reais == 0 { self.atual } else { 1 - self.atual };
        unsafe {
            self.ctx.CopySubresourceRegion(
                &t.imagem[novo].textura,
                0,
                0,
                0,
                0,
                area_de_trabalho,
                0,
                Some(&D3D11_BOX {
                    left: recorte.x as u32,
                    top: recorte.y as u32,
                    front: 0,
                    right: (recorte.x + recorte.largura) as u32,
                    bottom: (recorte.y + recorte.altura) as u32,
                    back: 1,
                }),
            );
        }
        let img = &t.imagem[novo];
        let q = &t.luma_q[novo];
        let s = &t.luma_s[novo];
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
        self.atual = novo;
        self.reais += 1;
    }

    /// Espera a GPU terminar o que foi enviado. Faz o custo medido ser o da
    /// placa, e não só o do envio — e garante que o quadro gerado já existe
    /// quando a agenda mandar mostrá-lo.
    pub fn esperar(&self) {
        unsafe {
            let mut consulta = None;
            if self.device.CreateQuery(&D3D11_QUERY_DESC { Query: D3D11_QUERY_EVENT, MiscFlags: 0 }, Some(&mut consulta)).is_err() {
                return;
            }
            let Some(consulta) = consulta else { return };
            self.ctx.End(&consulta);
            let limite = std::time::Instant::now() + std::time::Duration::from_millis(100);
            let mut pronto: i32 = 0;
            while std::time::Instant::now() < limite {
                if self.ctx.GetData(&consulta, Some(&mut pronto as *mut i32 as *mut _), 4, 0).is_ok() && pronto != 0 {
                    break;
                }
                std::thread::yield_now();
            }
        }
    }

    pub fn tem_par(&self) -> bool {
        self.reais >= 2
    }

    /// Campo de movimento entre o real anterior e o atual.
    pub fn estimar(&self) {
        let Some(t) = self.trabalho.as_ref() else { return };
        if !self.tem_par() {
            return;
        }
        let (a, b) = (1 - self.atual, self.atual);
        let lambda = 0.02;

        let s = &t.luma_s[a];
        self.passe(
            &self.passes.grossa,
            t.vet_grosso.rtv.as_ref().unwrap(),
            t.vet_grosso.largura,
            t.vet_grosso.altura,
            &[Some(t.luma_s[a].srv.clone()), Some(t.luma_s[b].srv.clone()), None],
            Parametros { tamanho_origem: [s.largura as i32, s.altura as i32], lambda, ..Default::default() },
        );

        let q = &t.luma_q[a];
        self.passe(
            &self.passes.fina,
            t.vet_fino.rtv.as_ref().unwrap(),
            t.vet_fino.largura,
            t.vet_fino.altura,
            &[Some(t.luma_q[a].srv.clone()), Some(t.luma_q[b].srv.clone()), Some(t.vet_grosso.srv.clone())],
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
            Parametros { tamanho_origem: [t.vet_fino.largura as i32, t.vet_fino.altura as i32], ..Default::default() },
        );
    }

    /// Desenha no alvo final: `Some(t)` gera o quadro em `t`; `None` copia o real.
    pub fn compor(&self, destino: &ID3D11RenderTargetView, largura: u32, altura: u32, t_gerado: Option<f32>) {
        let Some(tr) = self.trabalho.as_ref() else { return };
        let (a, b) = (1 - self.atual, self.atual);
        let texel = [1.0 / largura as f32, 1.0 / altura as f32];
        match t_gerado {
            Some(t) if self.tem_par() => {
                self.passe(
                    &self.passes.escolha,
                    tr.escolhido.rtv.as_ref().unwrap(),
                    tr.escolhido.largura,
                    tr.escolhido.altura,
                    &[Some(tr.imagem[a].srv.clone()), Some(tr.imagem[b].srv.clone()), Some(tr.vet_suave.srv.clone())],
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
                    &[Some(tr.imagem[a].srv.clone()), Some(tr.imagem[b].srv.clone()), Some(tr.escolhido.srv.clone())],
                    Parametros { texel_destino: texel, t, limiar_erro: 0.05, faixa_erro: 0.12, ..Default::default() },
                );
            }
            _ => self.passe(
                &self.passes.copia,
                destino,
                largura,
                altura,
                &[None, Some(tr.imagem[b].srv.clone()), None],
                Parametros { texel_destino: texel, lambda: if inverter_para_diagnostico() { -1.0 } else { 0.0 }, ..Default::default() },
            ),
        }
    }

    pub fn dxgi_device(&self) -> Result<windows::Win32::Graphics::Dxgi::IDXGIDevice, String> {
        self.device.cast().map_err(|e| erro("dxgi device", e))
    }
}
