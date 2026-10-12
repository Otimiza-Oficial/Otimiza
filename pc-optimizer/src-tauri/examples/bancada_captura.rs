//! Bancada de captura (laboratório E1): a Duplicação de Área de Trabalho (a captura do Frame Boost hoje) contra a
//! Windows.Graphics.Capture, no MESMO jogo de teste, com os mesmos números.
//!
//! O jogo de teste com `OTIMIZA_TESTE_MARCADOR=<arquivo>` escreve o número de cada quadro em 20 quadradinhos no canto
//! e guarda "número instante" de cada Present. Esta bancada lê o número em cada quadro capturado e conta, de verdade:
//! quantos quadros do jogo a captura viu, quantos perdeu, quantos repetiu, o intervalo entre quadros novos e o atraso
//! do Present do jogo até a captura chegar aqui. Não precisa de administrador.
//!
//!     cargo run --release --example bancada_captura -- <dxgi|wgc> <segundos> <arquivo do marcador do jogo>

#[cfg(windows)]
fn main() {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use windows::core::{w, Interface};
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Direct3D::*;
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::Win32::Graphics::Dxgi::Common::*;
    use windows::Win32::Graphics::Dxgi::*;
    use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    let args: Vec<String> = std::env::args().collect();
    let metodo = args.get(1).map(String::as_str).unwrap_or("dxgi").to_string();
    let segundos: f64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(10.0);
    let arquivo_do_jogo = args.get(3).cloned();

    unsafe {
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
        windows::Win32::Media::timeBeginPeriod(1);
    }
    let agora_qpc = || {
        let mut q = 0i64;
        unsafe {
            let _ = QueryPerformanceCounter(&mut q);
        }
        q
    };
    let mut freq = 0i64;
    unsafe {
        let _ = QueryPerformanceFrequency(&mut freq);
    }

    let hwnd = unsafe { FindWindowW(w!("JogoDeTeste"), None) }.expect("abra o jogo de teste antes");
    let mut origem = POINT::default();
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut origem);
    }

    // A placa e a saída do monitor do jogo (a Duplicação exige o dispositivo na placa do monitor).
    let fabrica: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.unwrap();
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut achado: Option<(IDXGIAdapter1, IDXGIOutput1, RECT)> = None;
    'placas: for i in 0.. {
        let Ok(adaptador) = (unsafe { fabrica.EnumAdapters1(i) }) else { break };
        for j in 0.. {
            let Ok(saida) = (unsafe { adaptador.EnumOutputs(j) }) else { break };
            let desc = unsafe { saida.GetDesc() }.unwrap();
            if desc.Monitor == monitor {
                achado = Some((adaptador.clone(), saida.cast().unwrap(), desc.DesktopCoordinates));
                break 'placas;
            }
        }
    }
    let (adaptador, saida, area_do_monitor) = achado.expect("o monitor do jogo não apareceu no DXGI");

    let mut device = None;
    let mut ctx = None;
    unsafe {
        D3D11CreateDevice(
            &adaptador,
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut ctx),
        )
        .unwrap();
    }
    let device: ID3D11Device = device.unwrap();
    let ctx: ID3D11DeviceContext = ctx.unwrap();

    // Faixa do marcador copiada para a CPU: 20 quadradinhos de 8 px.
    let criar_copia = |formato: DXGI_FORMAT| {
        let mut t = None;
        unsafe {
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: 160,
                        Height: 8,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: formato,
                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        Usage: D3D11_USAGE_STAGING,
                        BindFlags: 0,
                        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                        MiscFlags: 0,
                    },
                    None,
                    Some(&mut t),
                )
                .unwrap();
        }
        t.unwrap()
    };
    let mut copias: Vec<(DXGI_FORMAT, ID3D11Texture2D)> = Vec::new();
    // Lê o número do quadro da textura capturada, com o marcador começando em (x, y).
    let mut ler_numero = |textura: &ID3D11Texture2D, x: u32, y: u32| -> Option<u32> {
        let mut d = D3D11_TEXTURE2D_DESC::default();
        unsafe { textura.GetDesc(&mut d) };
        if x + 160 > d.Width || y + 8 > d.Height {
            return None;
        }
        let copia = match copias.iter().find(|(f, _)| *f == d.Format) {
            Some((_, c)) => c.clone(),
            None => {
                let c = criar_copia(d.Format);
                copias.push((d.Format, c.clone()));
                c
            }
        };
        unsafe {
            ctx.CopySubresourceRegion(
                &copia,
                0,
                0,
                0,
                0,
                textura,
                0,
                Some(&D3D11_BOX { left: x, top: y, front: 0, right: x + 160, bottom: y + 8, back: 1 }),
            );
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            ctx.Map(&copia, 0, D3D11_MAP_READ, 0, Some(&mut m)).ok()?;
            let linha = std::slice::from_raw_parts((m.pData as *const u8).add(4 * m.RowPitch as usize), m.RowPitch as usize);
            let mut n = 0u32;
            for bit in 0..20 {
                let px = bit * 8 + 4;
                let claro = if d.Format == DXGI_FORMAT_R16G16B16A16_FLOAT {
                    let i = px * 8;
                    u16::from_le_bytes([linha[i], linha[i + 1]]) > 0x3000
                } else {
                    let i = px * 4;
                    (linha[i] as u32 + linha[i + 1] as u32 + linha[i + 2] as u32) > 3 * 128
                };
                if claro {
                    n |= 1 << bit;
                }
            }
            ctx.Unmap(&copia, 0);
            Some(n)
        }
    };

    // (número lido, instante em que chegou aqui, em QPC)
    let mut vistos: Vec<(u32, i64)> = Vec::new();
    // `BANCADA_HORA=tela`: a hora de cada quadro é quando a imagem foi para a tela (`LastPresentTime` da Duplicação),
    // e não quando a captura chegou aqui. É a hora certa para medir tranco (E2).
    let hora_da_tela = std::env::var("BANCADA_HORA").is_ok_and(|v| v == "tela");
    let inicio = Instant::now();
    let tempos_de_cpu = || {
        let (mut c, mut s, mut k, mut u) = (0u64, 0u64, 0u64, 0u64);
        unsafe {
            windows_sys::Win32::System::Threading::GetProcessTimes(
                windows_sys::Win32::System::Threading::GetCurrentProcess(),
                &mut c as *mut u64 as _,
                &mut s as *mut u64 as _,
                &mut k as *mut u64 as _,
                &mut u as *mut u64 as _,
            );
        }
        (k + u) as f64 / 10_000.0
    };
    let cpu_antes = tempos_de_cpu();

    match metodo.as_str() {
        "dxgi" => {
            // Como o Frame Boost hoje: `DuplicateOutput1` pedindo FP16 ou BGRA.
            let o5: IDXGIOutput5 = saida.cast().unwrap();
            let dup = unsafe {
                o5.DuplicateOutput1(&device, 0, &[DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_B8G8R8A8_UNORM])
            }
            .unwrap();
            let x = (origem.x - area_do_monitor.left) as u32;
            let y = (origem.y - area_do_monitor.top) as u32;
            while inicio.elapsed().as_secs_f64() < segundos {
                let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut recurso: Option<IDXGIResource> = None;
                match unsafe { dup.AcquireNextFrame(100, &mut info, &mut recurso) } {
                    Ok(()) => {
                        let chegou = agora_qpc();
                        if info.LastPresentTime != 0 {
                            if let Some(t) = recurso.and_then(|r| r.cast::<ID3D11Texture2D>().ok()) {
                                if let Some(n) = ler_numero(&t, x, y) {
                                    vistos.push((n, if hora_da_tela { info.LastPresentTime } else { chegou }));
                                }
                            }
                        }
                        unsafe {
                            let _ = dup.ReleaseFrame();
                        }
                    }
                    Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => {}
                    Err(e) => panic!("a duplicação parou: {}", e.message()),
                }
            }
        }
        "wgc" => {
            use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
            use windows::Graphics::DirectX::DirectXPixelFormat;
            use windows::Win32::System::WinRT::Direct3D11::{
                CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
            };
            use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

            // A WinRT quer o COM do processo iniciado (multithread, como o Tauri faz).
            eprintln!(
                "RoInitialize: {:?}",
                unsafe { windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED) }
            );
            // Sem borda amarela: no Windows 11 o programa pede antes (concedido sozinho a programas de área de trabalho).
            let acesso = windows::Graphics::Capture::GraphicsCaptureAccess::RequestAccessAsync(
                windows::Graphics::Capture::GraphicsCaptureAccessKind::Borderless,
            )
            .and_then(|a| a.get());
            eprintln!("acesso sem borda: {:?}", acesso);
            let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().unwrap();
            // `BANCADA_ALVO=monitor`: captura o monitor do jogo em vez da janela (o marcador fica na posição da janela).
            let pelo_monitor = std::env::var("BANCADA_ALVO").is_ok_and(|v| v == "monitor");
            let item: GraphicsCaptureItem = if pelo_monitor {
                unsafe { interop.CreateForMonitor(monitor) }.unwrap()
            } else {
                unsafe { interop.CreateForWindow(hwnd) }.unwrap()
            };
            let (mx, my) = if pelo_monitor {
                ((origem.x - area_do_monitor.left) as u32, (origem.y - area_do_monitor.top) as u32)
            } else {
                (0, 0)
            };
            let dxgi: IDXGIDevice = device.cast().unwrap();
            let d3d: windows::Graphics::DirectX::Direct3D11::IDirect3DDevice =
                unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }.unwrap().cast().unwrap();
            // Fila de 1: sempre o quadro mais novo (o "Queue Target 0" do Lossless Scaling).
            let fila: i32 = std::env::var("BANCADA_FILA").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
            let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                &d3d,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                fila,
                item.Size().unwrap(),
            )
            .unwrap();
            let (tx, rx) = mpsc::channel::<()>();
            pool.FrameArrived(&windows::Foundation::TypedEventHandler::new(move |_, _| {
                let _ = tx.send(());
                Ok(())
            }))
            .unwrap();
            let sessao = pool.CreateCaptureSession(&item).unwrap();
            // `BANCADA_AJUSTES`: quais ajustes da sessão ligar (b = sem borda, c = sem cursor, i = sem intervalo mínimo).
            let ajustes = std::env::var("BANCADA_AJUSTES").unwrap_or_else(|_| "bci".into());
            if ajustes.contains('b') {
                eprintln!("sem borda: {:?}", sessao.SetIsBorderRequired(false));
            }
            if ajustes.contains('c') {
                let _ = sessao.SetIsCursorCaptureEnabled(false);
            }
            if ajustes.contains('i') {
                // Sem intervalo mínimo: todo quadro que o Windows compuser.
                eprintln!(
                    "intervalo mínimo: {:?}",
                    sessao.SetMinUpdateInterval(windows::Foundation::TimeSpan { Duration: 0 })
                );
            }
            sessao.StartCapture().unwrap();
            while inicio.elapsed().as_secs_f64() < segundos {
                if rx.recv_timeout(Duration::from_millis(100)).is_err() {
                    continue;
                }
                while let Ok(quadro) = pool.TryGetNextFrame() {
                    let chegou = agora_qpc();
                    let textura: ID3D11Texture2D = unsafe {
                        quadro.Surface().unwrap().cast::<IDirect3DDxgiInterfaceAccess>().unwrap().GetInterface()
                    }
                    .unwrap();
                    if let Some(n) = ler_numero(&textura, mx, my) {
                        vistos.push((n, chegou));
                    }
                    let _ = quadro.Close();
                }
            }
            let _ = sessao.Close();
            let _ = pool.Close();
        }
        outro => panic!("método desconhecido: {} (use dxgi ou wgc)", outro),
    }
    let cpu_ms = tempos_de_cpu() - cpu_antes;

    // O que o jogo apresentou no mesmo período (número → instante do Present).
    let presentes: std::collections::HashMap<u32, i64> = arquivo_do_jogo
        .and_then(|c| std::fs::read_to_string(c).ok())
        .map(|t| {
            t.lines()
                .filter_map(|l| {
                    let mut p = l.split_whitespace();
                    Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?))
                })
                .collect()
        })
        .unwrap_or_default();

    let ms = |q: i64| q as f64 * 1000.0 / freq as f64;
    if std::env::var_os("BANCADA_DEPURAR").is_some() {
        let mostra: Vec<String> = vistos.iter().take(40).map(|(n, _)| n.to_string()).collect();
        eprintln!("lidos: {}", mostra.join(" "));
        let mut chaves: Vec<&u32> = presentes.keys().collect();
        chaves.sort();
        eprintln!("no arquivo: {} números, de {:?} a {:?}", chaves.len(), chaves.first(), chaves.last());
    }
    let validos: Vec<(u32, i64)> = vistos.iter().copied().filter(|(n, _)| presentes.contains_key(n)).collect();
    let lixo = vistos.len() - validos.len();
    let mut novos: Vec<(u32, i64)> = Vec::new();
    let mut repetidos = 0usize;
    for (n, t) in &validos {
        if novos.last().is_some_and(|(u, _)| u >= n) {
            repetidos += 1;
        } else {
            novos.push((*n, *t));
        }
    }
    let (primeiro, ultimo) = (novos.first().map(|x| x.0).unwrap_or(0), novos.last().map(|x| x.0).unwrap_or(0));
    let do_jogo = presentes.keys().filter(|n| **n >= primeiro && **n <= ultimo).count();
    let perdidos = do_jogo.saturating_sub(novos.len());
    let duracao_s = novos.last().zip(novos.first()).map(|(u, p)| ms(u.1 - p.1) / 1000.0).unwrap_or(0.0).max(1e-3);
    let mut intervalos: Vec<f64> = novos.windows(2).map(|w| ms(w[1].1 - w[0].1)).collect();
    // Tranco: intervalos acima de 1,5× o ideal (o quadro chegou uma atualização depois, ou um sumiu no meio).
    let ideal_ms = 1000.0 * duracao_s / do_jogo.max(1) as f64;
    let atrasados = intervalos.iter().filter(|i| **i > 1.5 * ideal_ms).count();
    let mut atrasos: Vec<f64> = novos.iter().map(|(n, t)| ms(t - presentes[n])).collect();
    let pct = |v: &mut Vec<f64>, p: f64| -> f64 {
        if v.is_empty() {
            return f64::NAN;
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[((v.len() - 1) as f64 * p).round() as usize]
    };
    println!(
        "{} | jogo {:.1} fps | capturou {:.1}/s novos ({} quadros) | perdidos {} ({:.1}%) | repetidos {} | lixo {} | intervalo p50 {:.2} ms p99 {:.2} ms max {:.2} ms | atraso p50 {:.2} ms p95 {:.2} ms | CPU {:.3} ms por quadro | tranco {:.1}%",
        metodo,
        do_jogo as f64 / duracao_s,
        novos.len() as f64 / duracao_s,
        novos.len(),
        perdidos,
        100.0 * perdidos as f64 / do_jogo.max(1) as f64,
        repetidos,
        lixo,
        pct(&mut intervalos, 0.5),
        pct(&mut intervalos, 0.99),
        pct(&mut intervalos, 1.0),
        pct(&mut atrasos, 0.5),
        pct(&mut atrasos, 0.95),
        cpu_ms / vistos.len().max(1) as f64,
        100.0 * atrasados as f64 / intervalos.len().max(1) as f64
    );
    unsafe {
        windows::Win32::Media::timeEndPeriod(1);
    }
}

#[cfg(not(windows))]
fn main() {}
