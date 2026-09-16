//! Um "jogo" mínimo para testar o gerador de quadros sem abrir jogo nenhum.
//!
//! Janela Direct3D 11 de 1280×720 a 60 FPS: um xadrez que rola na horizontal
//! (a câmera andando), um quadrado que atravessa a tela (objeto rápido) e uma
//! barra parada no canto (interface). É exatamente o que testa a estimativa de
//! movimento: fundo em movimento uniforme, objeto com movimento próprio e
//! elemento fixo que não pode tremer.
//!
//!     cargo run --example jogo_de_teste -- [fps] [largura] [altura]

#[cfg(windows)]
fn main() {
    use std::time::{Duration, Instant};
    use windows::core::{w, Interface};
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Direct3D::*;
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::Win32::Graphics::Dxgi::Common::*;
    use windows::Win32::Graphics::Dxgi::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        if m == WM_DESTROY {
            PostQuitMessage(0);
            return LRESULT(0);
        }
        DefWindowProcW(h, m, w, l)
    }

    let fps: f64 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(60.0);
    let largura: i32 = std::env::args().nth(2).and_then(|a| a.parse().ok()).unwrap_or(1280);
    let altura: i32 = std::env::args().nth(3).and_then(|a| a.parse().ok()).unwrap_or(720);

    unsafe {
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let inst = GetModuleHandleW(None).unwrap();
        RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(proc_), hInstance: inst.into(), lpszClassName: w!("JogoDeTeste"), ..Default::default() });
        let mut r = RECT { left: 0, top: 0, right: largura, bottom: altura };
        let _ = AdjustWindowRect(&mut r, WS_POPUP, false);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("JogoDeTeste"),
            w!("Jogo de teste do Otimiza"),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            r.right - r.left,
            r.bottom - r.top,
            None,
            None,
            Some(inst.into()),
            None,
        )
        .unwrap();

        let desc = DXGI_SWAP_CHAIN_DESC {
            BufferDesc: DXGI_MODE_DESC { Width: largura as u32, Height: altura as u32, Format: DXGI_FORMAT_B8G8R8A8_UNORM, ..Default::default() },
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            OutputWindow: hwnd,
            Windowed: true.into(),
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            ..Default::default()
        };
        let mut swap = None;
        let mut dev = None;
        let mut ctx = None;
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&desc),
            Some(&mut swap),
            Some(&mut dev),
            None,
            Some(&mut ctx),
        )
        .unwrap();
        let swap: IDXGISwapChain = swap.unwrap();
        let dev: ID3D11Device = dev.unwrap();
        let ctx: ID3D11DeviceContext1 = ctx.unwrap().cast().unwrap();
        let fundo: ID3D11Texture2D = swap.GetBuffer(0).unwrap();
        let mut rtv = None;
        dev.CreateRenderTargetView(&fundo, None, Some(&mut rtv)).unwrap();
        let rtv = rtv.unwrap();

        let mut semente: u32 = 12345;
        let mut sorteio = |max: u32| {
            semente = semente.wrapping_mul(1103515245).wrapping_add(12345);
            (semente >> 8) % max
        };
        let cenario: Vec<(i32, i32, i32, i32, f32)> = (0..140)
            .map(|_| (sorteio(2560) as i32, sorteio(560) as i32, 30 + sorteio(170) as i32, 20 + sorteio(140) as i32, sorteio(1000) as f32 / 1000.0))
            .collect();

        let inicio = Instant::now();
        let quadro = Duration::from_secs_f64(1.0 / fps);
        let mut proximo = Instant::now();
        let mut msg = MSG::default();
        loop {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let t = inicio.elapsed().as_secs_f64();
            ctx.ClearRenderTargetView(&rtv, &[0.08, 0.1, 0.14, 1.0]);

            // "Cenário" irregular rolando a 240 px/s: blocos de tamanhos e tons
            // sorteados uma vez, repetidos a cada 2560 px.
            let desloc = t * 240.0;
            for (i, (bx, by, bl, ba, tom)) in cenario.iter().enumerate() {
                let x = (*bx as f64 - desloc).rem_euclid(2560.0) as i32 - 640;
                let cor = [0.25 + tom * 0.6, 0.3 + tom * 0.5, 0.35 + (1.0 - tom) * 0.4, 1.0];
                let _ = i;
                ctx.ClearView(&rtv, &cor, Some(&[RECT { left: x, top: *by, right: x + bl, bottom: by + ba }]));
            }

            // Faixa com textura repetida (grade), o caso difícil da busca.
            let passo = (t * 240.0) as i32 % 40;
            let listras: Vec<RECT> = (-1..34).map(|k| RECT { left: k * 40 - passo, top: 560, right: k * 40 - passo + 20, bottom: 600 }).collect();
            ctx.ClearView(&rtv, &[0.9, 0.9, 0.2, 1.0], Some(&listras));

            // Objeto rápido: 900 px/s na horizontal, ida e volta.
            let ciclo = (t * 900.0) % 2160.0;
            let x = if ciclo < 1080.0 { ciclo } else { 2160.0 - ciclo } as i32;
            ctx.ClearView(&rtv, &[0.95, 0.35, 0.15, 1.0], Some(&[RECT { left: x, top: 300, right: x + 200, bottom: 460 }]));

            // Interface parada.
            ctx.ClearView(&rtv, &[1.0, 1.0, 1.0, 1.0], Some(&[RECT { left: 40, top: 640, right: 420, bottom: 680 }]));

            let _ = swap.Present(0, DXGI_PRESENT(0));

            proximo += quadro;
            let agora = Instant::now();
            if proximo > agora {
                std::thread::sleep(proximo - agora);
            } else {
                proximo = agora;
            }
        }
    }
}

#[cfg(not(windows))]
fn main() {}
