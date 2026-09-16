// ---------------------------------------------------------------------------
// GERADOR DE QUADROS DO OTIMIZA
//
// A geração de quadros do próprio Otimiza, feita POR FORA do jogo — o mesmo
// caminho que o Lossless Scaling usa, e por isso seguro com anticheat:
//
//   1. captura a área do jogo pela Duplicação de Área de Trabalho (DXGI);
//   2. estima o movimento entre os dois últimos quadros reais, na GPU;
//   3. monta os quadros intermediários e os apresenta numa janela por cima do
//      jogo — que não recebe clique, não rouba foco e fica fora de captura;
//   4. obedece a agenda de `ritmo.rs`: gerados no meio, real no fim.
//
// O QUE ISTO FAZ E O QUE NÃO FAZ, SEM ENFEITE: aumenta os quadros EXIBIDOS
// (2×, 3×, 4×). Não aumenta os quadros que o jogo desenha, e acrescenta atraso
// — (M−1)/M de um quadro real, mais o atraso da captura. O laboratório mede
// os dois lados.
//
// Exige o jogo em JANELA ou JANELA SEM BORDAS: tela cheia exclusiva não passa
// pela composição do Windows e não pode ser capturada assim.
// ---------------------------------------------------------------------------

pub mod gpu;
pub mod ritmo;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::DirectComposition::{DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual};
use windows::Win32::Graphics::Direct3D11::{ID3D11RenderTargetView, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::{ClientToScreen, MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use gpu::Gpu;
use ritmo::{Apresentacao, Contadores, Fase, Intervalo, Retangulo};

// ================================================================== estado

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Situacao {
    Parado,
    ProcurandoJanela,
    Gerando,
    /// O jogo está aberto, mas nenhum quadro novo chega à captura: tela cheia
    /// exclusiva, minimizado ou parado.
    SemQuadros,
    Erro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estado {
    pub situacao: Situacao,
    pub processo: String,
    pub multiplicador: u8,
    pub area: Option<Retangulo>,
    pub fps_reais: f64,
    pub fps_apresentados: f64,
    pub contadores: Contadores,
    pub erro: Option<String>,
    /// PID deste processo: é por ele que o medidor de quadros conta o que o
    /// gerador apresenta.
    pub pid_do_gerador: u32,
}

impl Default for Estado {
    fn default() -> Self {
        Estado {
            situacao: Situacao::Parado,
            processo: String::new(),
            multiplicador: 2,
            area: None,
            fps_reais: 0.0,
            fps_apresentados: 0.0,
            contadores: Contadores::default(),
            erro: None,
            pid_do_gerador: std::process::id(),
        }
    }
}

static ESTADO: Mutex<Option<Estado>> = Mutex::new(None);

/// Diagnóstico: grava os próximos quadros apresentados como BMP nesta pasta.
static CAPTURA_DE_DIAGNOSTICO: Mutex<Option<(std::path::PathBuf, u32)>> = Mutex::new(None);

pub fn gravar_proximos_quadros(pasta: std::path::PathBuf, quantos: u32) {
    if let Ok(mut g) = CAPTURA_DE_DIAGNOSTICO.lock() {
        *g = Some((pasta, quantos));
    }
}

fn gravar_bmp(gpu: &Gpu, textura: &ID3D11Texture2D, caminho: &std::path::Path) -> Result<(), String> {
    use windows::Win32::Graphics::Direct3D11::*;
    unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        textura.GetDesc(&mut desc);
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.MiscFlags = 0;
        let mut copia = None;
        gpu.device.CreateTexture2D(&desc, None, Some(&mut copia)).map_err(|e| e.message().to_string())?;
        let copia = copia.ok_or("cópia")?;
        gpu.ctx.CopyResource(&copia, textura);
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        gpu.ctx.Map(&copia, 0, D3D11_MAP_READ, 0, Some(&mut m)).map_err(|e| e.message().to_string())?;
        let (l, a) = (desc.Width as usize, desc.Height as usize);
        let mut dados = Vec::with_capacity(54 + l * a * 4);
        let tamanho = (54 + l * a * 4) as u32;
        dados.extend_from_slice(b"BM");
        dados.extend_from_slice(&tamanho.to_le_bytes());
        dados.extend_from_slice(&[0, 0, 0, 0]);
        dados.extend_from_slice(&54u32.to_le_bytes());
        dados.extend_from_slice(&40u32.to_le_bytes());
        dados.extend_from_slice(&(l as i32).to_le_bytes());
        dados.extend_from_slice(&(-(a as i32)).to_le_bytes());
        dados.extend_from_slice(&1u16.to_le_bytes());
        dados.extend_from_slice(&32u16.to_le_bytes());
        dados.extend_from_slice(&[0u8; 24]);
        for y in 0..a {
            let linha = std::slice::from_raw_parts((m.pData as *const u8).add(y * m.RowPitch as usize), l * 4);
            dados.extend_from_slice(linha);
        }
        gpu.ctx.Unmap(&copia, 0);
        std::fs::write(caminho, dados).map_err(|e| e.to_string())
    }
}
static EM_EXECUCAO: Mutex<Option<(Arc<AtomicBool>, std::thread::JoinHandle<()>)>> = Mutex::new(None);

fn publicar(f: impl FnOnce(&mut Estado)) {
    if let Ok(mut e) = ESTADO.lock() {
        f(e.get_or_insert_with(Estado::default));
    }
}

pub fn estado() -> Estado {
    ESTADO.lock().ok().and_then(|e| e.clone()).unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuracao {
    pub processo: String,
    pub multiplicador: u8,
    /// Só para diagnóstico: deixa a janela do gerador aparecer em capturas.
    #[serde(default)]
    pub visivel_em_captura: bool,
}

pub fn ligar(config: Configuracao) -> Result<Estado, String> {
    desligar();
    if config.processo.trim().is_empty() {
        return Err("Diga qual é o jogo.".to_string());
    }
    let parar = Arc::new(AtomicBool::new(false));
    let p2 = parar.clone();
    publicar(|e| {
        *e = Estado {
            situacao: Situacao::ProcurandoJanela,
            processo: config.processo.clone(),
            multiplicador: config.multiplicador.clamp(2, 4),
            ..Estado::default()
        }
    });
    let linha = std::thread::Builder::new()
        .name("otimiza-gerador-de-quadros".into())
        .spawn(move || {
            if let Err(e) = executar(&config, &p2) {
                crate::utils::Logger::warn(&format!("gerador de quadros parou: {}", e));
                publicar(|s| {
                    s.situacao = Situacao::Erro;
                    s.erro = Some(e);
                });
            } else {
                publicar(|s| s.situacao = Situacao::Parado);
            }
        })
        .map_err(|e| e.to_string())?;
    if let Ok(mut g) = EM_EXECUCAO.lock() {
        *g = Some((parar, linha));
    }
    std::thread::sleep(Duration::from_millis(400));
    Ok(estado())
}

pub fn desligar() {
    let em = EM_EXECUCAO.lock().ok().and_then(|mut g| g.take());
    if let Some((parar, linha)) = em {
        parar.store(true, Ordering::SeqCst);
        let _ = linha.join();
    }
    publicar(|e| {
        e.situacao = Situacao::Parado;
        e.fps_apresentados = 0.0;
        e.fps_reais = 0.0;
    });
}

// ============================================================ a janela do jogo

struct Busca {
    pid: u32,
    melhor: Option<(HWND, i64)>,
}

unsafe extern "system" fn ao_enumerar(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let busca = &mut *(lparam.0 as *mut Busca);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == busca.pid && IsWindowVisible(hwnd).as_bool() {
        let mut r = RECT::default();
        if GetClientRect(hwnd, &mut r).is_ok() {
            let area = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
            if busca.melhor.map_or(true, |(_, a)| area > a) {
                busca.melhor = Some((hwnd, area));
            }
        }
    }
    true.into()
}

fn janela_do_processo(pid: u32) -> Option<HWND> {
    let mut busca = Busca { pid, melhor: None };
    unsafe {
        let _ = EnumWindows(Some(ao_enumerar), LPARAM(&mut busca as *mut Busca as isize));
    }
    busca.melhor.filter(|(_, a)| *a > 320 * 200).map(|(h, _)| h)
}

fn area_do_cliente(hwnd: HWND) -> Option<Retangulo> {
    unsafe {
        let mut r = RECT::default();
        GetClientRect(hwnd, &mut r).ok()?;
        let mut origem = POINT { x: 0, y: 0 };
        if !ClientToScreen(hwnd, &mut origem).as_bool() {
            return None;
        }
        Some(Retangulo { x: origem.x, y: origem.y, largura: r.right - r.left, altura: r.bottom - r.top })
    }
}

// ======================================================= a saída do monitor

struct Saida {
    adaptador: IDXGIAdapter1,
    saida: IDXGIOutput1,
    monitor: Retangulo,
}

fn saida_do_monitor(hwnd: HWND) -> Result<Saida, String> {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let fabrica: IDXGIFactory1 = CreateDXGIFactory1().map_err(|e| e.message().to_string())?;
        let mut i = 0;
        while let Ok(adaptador) = fabrica.EnumAdapters1(i) {
            let mut j = 0;
            while let Ok(saida) = adaptador.EnumOutputs(j) {
                if let Ok(desc) = saida.GetDesc() {
                    if desc.Monitor == monitor {
                        let r = desc.DesktopCoordinates;
                        return Ok(Saida {
                            adaptador,
                            saida: saida.cast().map_err(|e| e.message().to_string())?,
                            monitor: Retangulo { x: r.left, y: r.top, largura: r.right - r.left, altura: r.bottom - r.top },
                        });
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }
    Err("Não achei a saída de vídeo do monitor onde o jogo está.".to_string())
}

// =========================================================== a sobreposição

const CLASSE: PCWSTR = windows::core::w!("OtimizaGeradorDeQuadros");

unsafe extern "system" fn procedimento(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

struct Sobreposicao {
    hwnd: HWND,
    _dcomp: IDCompositionDevice,
    _alvo: IDCompositionTarget,
    _visual: IDCompositionVisual,
    swap: IDXGISwapChain1,
    rtv: Option<ID3D11RenderTargetView>,
    largura: u32,
    altura: u32,
}

impl Sobreposicao {
    fn nova(gpu: &Gpu, area: Retangulo, visivel_em_captura: bool) -> Result<Sobreposicao, String> {
        unsafe {
            let instancia = GetModuleHandleW(None).map_err(|e| e.message().to_string())?;
            let classe = WNDCLASSW {
                lpfnWndProc: Some(procedimento),
                hInstance: instancia.into(),
                lpszClassName: CLASSE,
                ..Default::default()
            };
            RegisterClassW(&classe);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP,
                CLASSE,
                windows::core::w!("Otimiza — geração de quadros"),
                WS_POPUP,
                area.x,
                area.y,
                area.largura,
                area.altura,
                None,
                None,
                Some(instancia.into()),
                None,
            )
            .map_err(|e| format!("a janela do gerador não abriu: {}", e.message()))?;

            let _ = SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), 255, LWA_ALPHA);
            if !visivel_em_captura {
                // Fora da captura: sem isto, o gerador capturaria o próprio
                // quadro gerado e interpolaria em cima dele.
                let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
            }

            let dxgi = gpu.dxgi_device()?;
            let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi).map_err(|e| e.message().to_string())?;
            let alvo = dcomp.CreateTargetForHwnd(hwnd, true).map_err(|e| e.message().to_string())?;
            let visual = dcomp.CreateVisual().map_err(|e| e.message().to_string())?;

            let adaptador: IDXGIAdapter = dxgi.GetAdapter().map_err(|e| e.message().to_string())?;
            let fabrica: IDXGIFactory2 = adaptador.GetParent().map_err(|e| e.message().to_string())?;
            let swap = fabrica
                .CreateSwapChainForComposition(
                    &gpu.device,
                    &DXGI_SWAP_CHAIN_DESC1 {
                        Width: area.largura as u32,
                        Height: area.altura as u32,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                        BufferCount: 2,
                        Scaling: DXGI_SCALING_STRETCH,
                        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
                        ..Default::default()
                    },
                    None,
                )
                .map_err(|e| format!("swap chain: {}", e.message()))?;
            visual.SetContent(&swap).map_err(|e| e.message().to_string())?;
            alvo.SetRoot(&visual).map_err(|e| e.message().to_string())?;
            dcomp.Commit().map_err(|e| e.message().to_string())?;

            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

            let mut s = Sobreposicao {
                hwnd,
                _dcomp: dcomp,
                _alvo: alvo,
                _visual: visual,
                swap,
                rtv: None,
                largura: area.largura as u32,
                altura: area.altura as u32,
            };
            s.criar_rtv(gpu)?;
            Ok(s)
        }
    }

    fn criar_rtv(&mut self, gpu: &Gpu) -> Result<(), String> {
        unsafe {
            let fundo: ID3D11Texture2D = self.swap.GetBuffer(0).map_err(|e| e.message().to_string())?;
            let mut rtv = None;
            gpu.device.CreateRenderTargetView(&fundo, None, Some(&mut rtv)).map_err(|e| e.message().to_string())?;
            self.rtv = rtv;
        }
        Ok(())
    }

    fn apresentar(&self, gpu: &Gpu, t: Option<f32>) {
        if let Some(rtv) = &self.rtv {
            gpu.compor(rtv, self.largura, self.altura, t);
            if let Ok(mut g) = CAPTURA_DE_DIAGNOSTICO.lock() {
                if let Some((pasta, restantes)) = g.as_mut() {
                    if *restantes > 0 {
                        let nome = match t {
                            Some(t) => format!("quadro_{:02}_gerado_{:.2}.bmp", *restantes, t),
                            None => format!("quadro_{:02}_real.bmp", *restantes),
                        };
                        if let Ok(fundo) = unsafe { self.swap.GetBuffer::<ID3D11Texture2D>(0) } {
                            let _ = gravar_bmp(gpu, &fundo, &pasta.join(nome));
                        }
                        *restantes -= 1;
                    }
                }
            }
            unsafe {
                let _ = self.swap.Present(0, DXGI_PRESENT(0));
            }
        }
    }
}

impl Drop for Sobreposicao {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

/// Devolve `true` quando o atalho de desligar foi apertado.
fn bombear_mensagens() -> bool {
    let mut desligar = false;
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_HOTKEY && msg.wParam.0 == ATALHO as usize {
                desligar = true;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    desligar
}

/// Ctrl+Alt+G desliga o gerador de qualquer lugar, inclusive de dentro do jogo.
const ATALHO: i32 = 0x4F47;

// ================================================================ o laço

fn segundos(inicio: Instant) -> f64 {
    inicio.elapsed().as_secs_f64()
}

fn executar(config: &Configuracao, parar: &AtomicBool) -> Result<(), String> {
    use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
    use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT};
    unsafe {
        timeBeginPeriod(1);
        let _ = RegisterHotKey(None, ATALHO, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, b'G' as u32);
    }
    let r = executar_interno(config, parar);
    unsafe {
        let _ = UnregisterHotKey(None, ATALHO);
        timeEndPeriod(1);
    }
    r
}

fn executar_interno(config: &Configuracao, parar: &AtomicBool) -> Result<(), String> {
    let multiplicador = config.multiplicador.clamp(2, 4);
    // Taxa do monitor principal: o teto de quadros que vale a pena gerar.
    let hz = super::display::monitores().into_iter().find(|m| m.principal).map(|m| m.hz_atual).unwrap_or(0);

    // Procura o jogo por até 30 segundos.
    let limite = Instant::now() + Duration::from_secs(30);
    let (pid, _nome) = loop {
        if parar.load(Ordering::Relaxed) {
            return Ok(());
        }
        if let Some(achado) = super::frames::encontrar_processo(&config.processo) {
            break achado;
        }
        if Instant::now() > limite {
            return Err(format!("Não encontrei `{}` aberto.", config.processo));
        }
        std::thread::sleep(Duration::from_millis(500));
    };

    let hwnd = janela_do_processo(pid).ok_or("O jogo está aberto, mas sem janela visível. Coloque em janela sem bordas.")?;
    let cliente = area_do_cliente(hwnd).ok_or("Não consegui ler a área da janela do jogo.")?;
    let saida = saida_do_monitor(hwnd)?;
    let recorte = ritmo::recorte(cliente, saida.monitor).ok_or("A janela do jogo está fora do monitor ou pequena demais.")?;

    let mut gpu = Gpu::novo(&saida.adaptador)?;
    gpu.preparar(recorte.largura as u32, recorte.altura as u32)?;

    let duplicacao: IDXGIOutputDuplication =
        unsafe { saida.saida.DuplicateOutput(&gpu.device) }.map_err(|e| format!("a captura da tela não abriu: {}", e.message()))?;

    let area_na_tela = Retangulo { x: saida.monitor.x + recorte.x, y: saida.monitor.y + recorte.y, ..recorte };
    let sobreposicao = Sobreposicao::nova(&gpu, area_na_tela, config.visivel_em_captura)?;

    publicar(|e| {
        e.situacao = Situacao::Gerando;
        e.area = Some(area_na_tela);
        e.multiplicador = multiplicador;
        e.erro = None;
    });

    let inicio = Instant::now();
    let mut intervalo = Intervalo::default();
    let mut agenda: std::collections::VecDeque<Apresentacao> = Default::default();
    let mut contadores = Contadores::default();
    let mut janela_de_contagem = (segundos(inicio), 0u64, 0u64);
    let mut ultima_checagem = Instant::now();
    let mut ultimo_real = Instant::now();

    let mut escondida = false;
    let mut cabe_atual = multiplicador;
    let mut pedidos_de_troca = 0u32;
    while !parar.load(Ordering::Relaxed) {
        if bombear_mensagens() {
            return Ok(());
        }

        // NUNCA UMA IMAGEM CONGELADA POR CIMA DO JOGO. Sem quadro real por
        // meio segundo (tela de carregamento travada, tela cheia exclusiva,
        // captura parada), a sobreposição some e o jogo aparece por baixo.
        let parado = ultimo_real.elapsed() > Duration::from_millis(500);
        if parado != escondida {
            unsafe {
                let _ = ShowWindow(sobreposicao.hwnd, if parado { SW_HIDE } else { SW_SHOWNOACTIVATE });
            }
            escondida = parado;
        }

        // A janela do jogo mudou de lugar ou fechou: para, e a tela diz.
        if ultima_checagem.elapsed() > Duration::from_millis(500) {
            ultima_checagem = Instant::now();
            if unsafe { !IsWindow(Some(hwnd)).as_bool() } {
                return Err("O jogo fechou.".to_string());
            }
            if area_do_cliente(hwnd) != Some(cliente) {
                return Err("A janela do jogo mudou de tamanho ou de lugar. Ligue o gerador de novo.".to_string());
            }
            let agora = segundos(inicio);
            let dt = agora - janela_de_contagem.0;
            if dt > 0.0 {
                let reais = contadores.reais - janela_de_contagem.1;
                let apresentados = (contadores.reais + contadores.gerados) - janela_de_contagem.2;
                publicar(|e| {
                    e.fps_reais = (reais as f64 / dt * 10.0).round() / 10.0;
                    e.fps_apresentados = (apresentados as f64 / dt * 10.0).round() / 10.0;
                    e.contadores = contadores;
                    e.situacao = if ultimo_real.elapsed() > Duration::from_secs(2) { Situacao::SemQuadros } else { Situacao::Gerando };
                });
            }
            janela_de_contagem = (agora, contadores.reais, contadores.reais + contadores.gerados);
        }

        // Espera o próximo quadro real só até a próxima apresentação marcada.
        let espera_ms = agenda
            .front()
            .map(|a| ((a.instante - segundos(inicio)) * 1000.0).floor().max(0.0) as u32)
            .unwrap_or(8)
            .min(8);

        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut recurso: Option<IDXGIResource> = None;
        match unsafe { duplicacao.AcquireNextFrame(espera_ms, &mut info, &mut recurso) } {
            Ok(()) => {
                if info.LastPresentTime != 0 {
                    if let Some(tex) = recurso.as_ref().and_then(|r| r.cast::<ID3D11Texture2D>().ok()) {
                        let chegada = segundos(inicio);
                        let medido = Instant::now();
                        gpu.receber(&tex, recorte);
                        intervalo.registrar(chegada);
                        contadores.reais += 1;
                        ultimo_real = Instant::now();

                        contadores.descartados += agenda.len() as u64;
                        agenda.clear();
                        if gpu.tem_par() && intervalo.pode_gerar() {
                            gpu.estimar();
                            gpu.esperar();
                            let custo = medido.elapsed().as_secs_f64() * 1000.0;
                            contadores.custo_ms = if contadores.custo_ms == 0.0 { custo } else { contadores.custo_ms * 0.9 + custo * 0.1 };
                            // A agenda começa quando a GPU terminou, e reparte só o
                            // tempo que sobra até o próximo quadro real.
                            let pronto = segundos(inicio);
                            let janela = intervalo.estimado.unwrap_or(0.0) - (pronto - chegada);
                            let quer = ritmo::multiplicador_que_cabe(multiplicador, intervalo.estimado.unwrap_or(0.0), hz);
                            // Histerese: o jogo oscilando perto do limite (87–92 FPS num
                            // monitor de 180 Hz) não pode ficar ligando e desligando a
                            // geração a cada quadro — isso é engasgo. Troca só depois de
                            // meio segundo pedindo a mesma coisa.
                            if quer == cabe_atual {
                                pedidos_de_troca = 0;
                            } else {
                                pedidos_de_troca += 1;
                                if pedidos_de_troca as f64 * intervalo.estimado.unwrap_or(0.016) > 0.5 {
                                    cabe_atual = quer;
                                    pedidos_de_troca = 0;
                                }
                            }
                            let cabe = cabe_atual;
                            agenda.extend(ritmo::agenda(pronto, janela.max(0.0), cabe));
                        } else {
                            agenda.push_back(Apresentacao { instante: chegada, fase: Fase::Real });
                        }
                    }
                }
                unsafe {
                    let _ = duplicacao.ReleaseFrame();
                }
            }
            Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => {}
            Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => {
                return Err("A captura foi interrompida pelo Windows (troca de modo de vídeo, UAC ou tela cheia exclusiva). Ligue de novo.".to_string());
            }
            Err(e) => return Err(format!("captura: {}", e.message())),
        }

        // Apresenta o que venceu. Se mais de um venceu, só o último vai à tela.
        let agora = segundos(inicio);
        let mut vencido = None;
        while agenda.front().is_some_and(|a| a.instante <= agora + 0.0005) {
            if vencido.is_some() {
                contadores.descartados += 1;
            }
            vencido = agenda.pop_front();
        }
        if let Some(a) = vencido {
            match a.fase {
                Fase::Gerado(t) => {
                    sobreposicao.apresentar(&gpu, Some(t));
                    contadores.gerados += 1;
                }
                Fase::Real => sobreposicao.apresentar(&gpu, None),
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Liga o gerador sobre o `jogo_de_teste` já aberto, grava quadros e
    /// imprime o estado. Roda com:
    /// `cargo test --lib -- --ignored geracao::tests::sobre_o_jogo_de_teste --nocapture`
    #[test]
    #[ignore]
    fn sobre_o_jogo_de_teste() {
        let pasta = std::path::PathBuf::from(std::env::var("OTIMIZA_FG_PASTA").unwrap_or_else(|_| ".".into()));
        let mult: u8 = std::env::var("OTIMIZA_FG_MULT").ok().and_then(|v| v.parse().ok()).unwrap_or(2);
        let e = ligar(Configuracao { processo: std::env::var("OTIMIZA_FG_PROCESSO").unwrap_or_else(|_| "jogo_de_teste".into()), multiplicador: mult, visivel_em_captura: std::env::var_os("OTIMIZA_FG_VISIVEL").is_some() }).unwrap();
        println!("inicio: {:?}", e);
        std::thread::sleep(Duration::from_secs(3));
        gravar_proximos_quadros(pasta, 6);
        for _ in 0..5 {
            std::thread::sleep(Duration::from_secs(1));
            let e = estado();
            println!("{:?} reais={} apresentados={} contadores={:?} erro={:?}", e.situacao, e.fps_reais, e.fps_apresentados, e.contadores, e.erro);
        }
        desligar();
    }
}
