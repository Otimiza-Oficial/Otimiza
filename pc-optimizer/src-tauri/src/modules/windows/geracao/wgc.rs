// A captura pela Windows.Graphics.Capture (WGC), a reserva da Duplicação de Área de Trabalho (1.0.7).
//
// O gerador captura o monitor pela Duplicação (DXGI): é a que entrega o retângulo que mudou e o carimbo de composição.
// Em parte das máquinas ela nem começa (notebook híbrido com o monitor na placa dedicada: `DXGI_ERROR_UNSUPPORTED`;
// alguns drivers): aí o Frame Boost dava erro e não ligava. A WGC captura o mesmo monitor por outro caminho do Windows
// (é a do Magpie e a opção do Lossless Scaling), então entra SÓ nesses casos: quem hoje já usa o Frame Boost não muda.
//
// Monitor inteiro, como a Duplicação: a área do jogo continua nas mesmas coordenadas. O quadro chega quando a tela muda;
// o "mudou dentro do jogo" sai da comparação na placa (`Gpu::igual_ao_atual`), já que a WGC não diz o que mudou.

use windows::core::Interface;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

pub struct Wgc {
    pool: Direct3D11CaptureFramePool,
    sessao: GraphicsCaptureSession,
    _item: GraphicsCaptureItem,
}

/// Um quadro da WGC: a textura (vale enquanto o quadro vive) e o instante em que o Windows o compôs, em segundos do
/// mesmo relógio da Duplicação (QPC).
pub struct QuadroWgc {
    pub textura: ID3D11Texture2D,
    pub composto_s: f64,
    _quadro: windows::Graphics::Capture::Direct3D11CaptureFrame,
}

fn erro(o_que: &str, e: windows::core::Error) -> String {
    format!("captura do Windows ({}): {}", o_que, e.message())
}

impl Wgc {
    /// Começa a capturar o monitor, no mesmo dispositivo do gerador (a textura vem pronta para ele).
    pub fn do_monitor(device: &ID3D11Device, monitor: HMONITOR) -> Result<Wgc, String> {
        if !GraphicsCaptureSession::IsSupported().unwrap_or(false) {
            return Err("este Windows não tem a captura moderna (Windows.Graphics.Capture)".to_string());
        }
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
            .map_err(|e| erro("interop", e))?;
        let item: GraphicsCaptureItem =
            unsafe { interop.CreateForMonitor(monitor) }.map_err(|e| erro("monitor", e))?;
        let dxgi: IDXGIDevice = device.cast().map_err(|e| erro("dispositivo", e))?;
        let direto: IDirect3DDevice = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }
            .and_then(|i| i.cast())
            .map_err(|e| erro("dispositivo WinRT", e))?;
        let tamanho = item.Size().map_err(|e| erro("tamanho", e))?;
        // Sem a moldura amarela (Windows 11) e sem o cursor dentro da imagem (o do sistema fica por cima, como na
        // Duplicação). Tirar a moldura pede permissão ao Windows; recusada ("acesso negado" ao começar), tudo é refeito
        // com a moldura (fila nova: a da tentativa recusada não serve mais). A captura vale mais que a moldura.
        let comecar = |sem_moldura: bool| -> windows::core::Result<(Direct3D11CaptureFramePool, GraphicsCaptureSession)> {
            let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                &direto,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                tamanho,
            )?;
            let sessao = pool.CreateCaptureSession(&item)?;
            if sem_moldura {
                sessao.SetIsBorderRequired(false)?;
            }
            let _ = sessao.SetIsCursorCaptureEnabled(false);
            if let Err(e) = sessao.StartCapture() {
                let _ = sessao.Close();
                let _ = pool.Close();
                return Err(e);
            }
            Ok((pool, sessao))
        };
        let (pool, sessao) = match comecar(true) {
            Ok(x) => x,
            Err(_) => comecar(false).map_err(|e| erro("começar", e))?,
        };
        Ok(Wgc { pool, sessao, _item: item })
    }

    /// O quadro mais novo que chegou, esperando até `espera_ms`. `None`: nada novo nesse tempo.
    pub fn proximo(&self, espera_ms: u32) -> Option<QuadroWgc> {
        let fim = std::time::Instant::now() + std::time::Duration::from_millis(espera_ms as u64);
        loop {
            // Fica com o mais novo: dois quadros na fila, o de trás já é velho.
            let mut ultimo = None;
            while let Ok(q) = self.pool.TryGetNextFrame() {
                ultimo = Some(q);
            }
            if let Some(quadro) = ultimo {
                let textura = quadro
                    .Surface()
                    .ok()
                    .and_then(|s| s.cast::<IDirect3DDxgiInterfaceAccess>().ok())
                    .and_then(|a| unsafe { a.GetInterface::<ID3D11Texture2D>() }.ok())?;
                // `SystemRelativeTime`: o QPC em unidades de 100 ns.
                let composto_s = quadro.SystemRelativeTime().map(|t| t.Duration as f64 / 1e7).unwrap_or(0.0);
                return Some(QuadroWgc { textura, composto_s, _quadro: quadro });
            }
            if std::time::Instant::now() >= fim {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

impl Drop for Wgc {
    fn drop(&mut self) {
        let _ = self.sessao.Close();
        let _ = self.pool.Close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

    /// Na máquina de verdade: a WGC começa no monitor principal, no dispositivo do gerador, e entrega um quadro.
    /// No PC do dono ela é RECUSADA (E_ACCESSDENIED no StartCapture, já medido no Laboratório E1, docs/app/PROGRESS.md:
    /// o Windows modificado "SnyX OS" bloqueia a WGC); num Windows original deve passar. Por isso ela só entra quando a
    /// Duplicação nem começa, e falhando as duas vale o erro da Duplicação.
    #[test]
    #[ignore = "lê esta máquina"]
    fn captura_o_monitor_desta_maquina() {
        let fabrica: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.unwrap();
        let adaptador = unsafe { fabrica.EnumAdapters1(0) }.unwrap();
        let saida = unsafe { adaptador.EnumOutputs(0) }.unwrap();
        let monitor = unsafe { saida.GetDesc() }.unwrap().Monitor;
        let gpu = super::super::gpu::Gpu::novo(&adaptador).unwrap();
        let w = Wgc::do_monitor(&gpu.device, monitor).expect("a WGC começa");
        // Algo precisa mudar na tela para chegar quadro: o primeiro vem logo após começar.
        let q = w.proximo(2000).expect("um quadro em até 2 s");
        let mut desc = Default::default();
        unsafe { q.textura.GetDesc(&mut desc) };
        println!("WGC: {}x{} formato {:?}, composto em {:.3} s", desc.Width, desc.Height, desc.Format, q.composto_s);
        assert!(desc.Width > 0 && desc.Height > 0);
        assert!(q.composto_s > 0.0, "o carimbo vem do relógio do sistema");
    }
}
