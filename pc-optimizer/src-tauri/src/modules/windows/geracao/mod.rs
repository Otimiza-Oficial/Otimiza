// Gerador de quadros POR FORA do jogo (como o Lossless Scaling, seguro com anticheat): captura pela Duplicação
// de Área de Trabalho, estima o movimento na GPU e apresenta numa janela por cima, fora de captura. Aumenta os
// quadros EXIBIDOS, não os que o jogo desenha, e acrescenta atraso. Funciona em janela, sem bordas e na tela cheia
// dos jogos de hoje, que o Windows faz por composição (Independent Flip): medido em 10/10 com o jogo de teste em tela
// cheia a 60 fps, a sobreposição por cima não tirou o jogo do modo nem lhe tirou FPS. Na tela cheia exclusiva antiga
// (o jogo troca o modo de vídeo do monitor), nada fica por cima: o gerador percebe as trocas e desliga.

pub mod escala;
pub mod gpu;
pub mod guarda;
pub mod resumo;
pub mod ritmo;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct3D11::{ID3D11RenderTargetView, ID3D11Texture2D};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::{ClientToScreen, MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use gpu::Gpu;
use ritmo::{Apresentacao, Contadores, Fase, Intervalo, Retangulo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Situacao {
    Parado,
    ProcurandoJanela,
    Gerando,
    SemQuadros,
    Erro,
    DesligadoPorPerdaDeFps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estado {
    pub situacao: Situacao,
    pub processo: String,
    pub multiplicador: u8,
    /// Ligado no automático: `multiplicador` é o do último quadro real (varia, 3×, 4×…).
    #[serde(default)]
    pub automatico: bool,
    /// Ligado passando do limite do monitor (`Configuracao::sem_limite`).
    #[serde(default)]
    pub sem_limite: bool,
    pub area: Option<Retangulo>,
    pub fps_reais: f64,
    pub fps_apresentados: f64,
    pub contadores: Contadores,
    pub erro: Option<String>,
    pub razao_fps: Option<f64>,
    /// Do instante em que o Windows compôs o quadro do jogo até o gerador mostrá-lo: o atraso que ele acrescenta.
    pub atraso_ms: Option<f64>,
    pub pid_do_gerador: u32,
    /// Hz do monitor ONDE O JOGO ESTÁ: o teto do que dá para mostrar.
    #[serde(default)]
    pub hz: u32,
    /// A área de trabalho é HDR e o gerador trabalha em FP16.
    #[serde(default)]
    pub hdr: bool,
    /// A placa que captura e gera (a do monitor).
    #[serde(default)]
    pub placa: Option<String>,
    /// Notebook híbrido com o monitor na placa integrada: o gerador roda nela, e a guarda vigia mais de perto.
    #[serde(default)]
    pub integrada: bool,
    /// Quantas vezes o gerador se refez sozinho (janela redimensionada, movida, minimizada, troca de modo de vídeo).
    #[serde(default)]
    pub refeitas: u32,
    /// O tempo de placa que o gerador ocupou nos últimos 10 s, em ms por segundo (MEDIDO pelos carimbos da placa,
    /// `gpu::Cronometro`). 100 ms/s = 10% da placa tirada do jogo.
    #[serde(default)]
    pub placa_ms_por_s: Option<f64>,
    /// Quadros por segundo que CHEGARAM À TELA, pelo PresentMon no processo do Otimiza (`DisplayedTime`). `None` sem
    /// PresentMon (sem administrador): aí o painel mostra `fps_apresentados` como estimativa.
    #[serde(default)]
    pub fps_exibidos: Option<f64>,
    /// No modo leve: começou assim (placa sem folga) ou a guarda aliviou depois de ver o jogo perder FPS.
    #[serde(default)]
    pub leve: bool,
    /// Quantas comparações com e sem o gerador valem na `razao_fps` (a guarda decide a partir de
    /// `guarda::MINIMO_DE_AMOSTRAS`).
    #[serde(default)]
    pub comparacoes: u32,
    /// Desde que ligou, só o tempo gerando e à vista: o "antes × depois" do painel quando desliga.
    #[serde(default)]
    pub sessao: resumo::Acumulado,
}

impl Default for Estado {
    fn default() -> Self {
        Estado {
            situacao: Situacao::Parado,
            processo: String::new(),
            multiplicador: 2,
            automatico: false,
            sem_limite: false,
            area: None,
            fps_reais: 0.0,
            fps_apresentados: 0.0,
            contadores: Contadores::default(),
            erro: None,
            razao_fps: None,
            atraso_ms: None,
            pid_do_gerador: std::process::id(),
            hz: 0,
            hdr: false,
            placa: None,
            integrada: false,
            refeitas: 0,
            placa_ms_por_s: None,
            fps_exibidos: None,
            leve: false,
            comparacoes: 0,
            sessao: resumo::Acumulado::default(),
        }
    }
}

static ESTADO: Mutex<Option<Estado>> = Mutex::new(None);

static CAPTURA_DE_DIAGNOSTICO: Mutex<Option<(std::path::PathBuf, u32)>> = Mutex::new(None);

#[cfg(test)]
pub fn gravar_proximos_quadros(pasta: std::path::PathBuf, quantos: u32) {
    if let Ok(mut g) = CAPTURA_DE_DIAGNOSTICO.lock() {
        *g = Some((pasta, quantos));
    }
}

fn gravar_bmp(
    gpu: &Gpu,
    textura: &ID3D11Texture2D,
    caminho: &std::path::Path,
) -> Result<(), String> {
    use windows::Win32::Graphics::Direct3D11::*;
    unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        textura.GetDesc(&mut desc);
        // HDR: FP16 linear, convertido para 8 bits com gama 2,2, para comparar com o mesmo quadro no SDR.
        let fp16 = desc.Format == DXGI_FORMAT_R16G16B16A16_FLOAT;
        if !fp16 && desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
            return Err("formato sem diagnóstico".to_string());
        }
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.MiscFlags = 0;
        let mut copia = None;
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut copia))
            .map_err(|e| e.message().to_string())?;
        let copia = copia.ok_or("cópia")?;
        gpu.ctx.CopyResource(&copia, textura);
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        gpu.ctx
            .Map(&copia, 0, D3D11_MAP_READ, 0, Some(&mut m))
            .map_err(|e| e.message().to_string())?;
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
            let base = (m.pData as *const u8).add(y * m.RowPitch as usize);
            if fp16 {
                // Tabela: a conversão pixel a pixel atrasava o quadro gerado seguinte e ele nunca era gravado.
                static TABELA: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
                let tabela = TABELA.get_or_init(|| {
                    (0..=u16::MAX)
                        .map(|v| {
                            (gpu::meio_float(v).clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0).round()
                                as u8
                        })
                        .collect()
                });
                let linha = std::slice::from_raw_parts(base as *const u16, l * 4);
                for px in linha.chunks_exact(4) {
                    let c = |v: u16| tabela[v as usize];
                    dados.extend_from_slice(&[c(px[2]), c(px[1]), c(px[0]), 255]);
                }
            } else {
                dados.extend_from_slice(std::slice::from_raw_parts(base, l * 4));
            }
        }
        gpu.ctx.Unmap(&copia, 0);
        std::fs::write(caminho, dados).map_err(|e| e.to_string())
    }
}
static EM_EXECUCAO: Mutex<Option<(Arc<AtomicBool>, std::thread::JoinHandle<()>)>> =
    Mutex::new(None);

fn publicar(f: impl FnOnce(&mut Estado)) {
    if let Ok(mut e) = ESTADO.lock() {
        f(e.get_or_insert_with(Estado::default));
    }
}

pub fn estado() -> Estado {
    ESTADO
        .lock()
        .ok()
        .and_then(|e| e.clone())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuracao {
    pub processo: String,
    pub multiplicador: u8,
    #[serde(default)]
    pub visivel_em_captura: bool,
    /// Automático: enche o monitor (`ritmo::quadros_no_automatico`), com `multiplicador` como teto.
    #[serde(default)]
    pub automatico: bool,
    /// Passar do limite do monitor: o multiplicador não é cortado pelos Hz, e o automático mira o dobro deles. O monitor
    /// continua mostrando no máximo os Hz dele: acima disso é como um jogo sem limite de FPS (o quadro é gerado e
    /// apresentado, e o Windows mostra o mais novo a cada atualização da tela).
    #[serde(default)]
    pub sem_limite: bool,
    /// Começa no modo leve (`gpu::NIVEIS_DO_MODO_LEVE`): placa sem folga.
    #[serde(default)]
    pub leve: bool,
}

/// O que fazer quando a guarda vê o jogo perder FPS. Pura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReacaoAPerda {
    /// Primeiro alivia a placa e confere de novo, do zero.
    FicarLeve,
    /// Já estava leve e o jogo ainda perde: desliga.
    Desligar,
}

pub fn reacao_a_perda(ja_leve: bool) -> ReacaoAPerda {
    if ja_leve {
        ReacaoAPerda::Desligar
    } else {
        ReacaoAPerda::FicarLeve
    }
}

/// O teto do automático: com o jogo a 30 fps num monitor de 180 Hz, seis quadros por real.
pub const MAXIMO_DO_AUTOMATICO: u8 = 6;

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
            multiplicador: config.multiplicador.clamp(2, MAXIMO_DO_AUTOMATICO),
            automatico: config.automatico,
            sem_limite: config.sem_limite,
            leve: config.leve,
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
                publicar(|s| {
                    if s.situacao != Situacao::DesligadoPorPerdaDeFps {
                        s.situacao = Situacao::Parado;
                    }
                });
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

pub(crate) fn janela_do_processo(pid: u32) -> Option<HWND> {
    let mut busca = Busca { pid, melhor: None };
    unsafe {
        let _ = EnumWindows(Some(ao_enumerar), LPARAM(&mut busca as *mut Busca as isize));
    }
    busca.melhor.filter(|(_, a)| *a > 320 * 200).map(|(h, _)| h)
}

pub(crate) fn area_do_cliente(hwnd: HWND) -> Option<Retangulo> {
    unsafe {
        let mut r = RECT::default();
        GetClientRect(hwnd, &mut r).ok()?;
        let mut origem = POINT { x: 0, y: 0 };
        if !ClientToScreen(hwnd, &mut origem).as_bool() {
            return None;
        }
        Some(Retangulo {
            x: origem.x,
            y: origem.y,
            largura: r.right - r.left,
            altura: r.bottom - r.top,
        })
    }
}

struct Saida {
    adaptador: IDXGIAdapter1,
    saida: IDXGIOutput1,
    monitor: Retangulo,
    /// `\\.\DISPLAY1`: o mesmo nome de `display::monitores`, para o Hz ser o DESTE monitor.
    dispositivo: String,
    placa: String,
    integrada: bool,
}

fn texto_de(w: &[u16]) -> String {
    let fim = w.iter().position(|c| *c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..fim]).trim().to_string()
}

/// Pura: a placa do monitor é a integrada de um notebook híbrido: pouca memória própria e outra placa com bem mais.
fn e_integrada(dedicada: u64, outras: &[u64]) -> bool {
    const GB: u64 = 1 << 30;
    dedicada < GB
        && outras
            .iter()
            .any(|o| *o >= GB && *o > dedicada.saturating_mul(2))
}

/// Pura: o Hz do monitor pelo nome do dispositivo; sem achar, o do principal; sem nada, 0 (sem teto).
pub(crate) fn hz_escolhido(monitores: &[(&str, bool, u32)], dispositivo: &str) -> u32 {
    monitores
        .iter()
        .find(|m| m.0.eq_ignore_ascii_case(dispositivo))
        .or_else(|| monitores.iter().find(|m| m.1))
        .map(|m| m.2)
        .unwrap_or(0)
}

fn hz_do_monitor(dispositivo: &str) -> u32 {
    if std::env::var_os("OTIMIZA_FG_SEM_TETO").is_some() {
        return 0;
    }
    let ms = super::display::monitores_sem_nomes();
    let lista: Vec<(&str, bool, u32)> = ms
        .iter()
        .map(|m| (m.dispositivo.as_str(), m.principal, m.hz_atual))
        .collect();
    hz_escolhido(&lista, dispositivo)
}

/// Pura: o motivo da captura recusada, em palavras de quem joga.
fn explicar_captura(codigo: windows::core::HRESULT, mensagem: &str) -> String {
    use windows::Win32::Foundation::E_ACCESSDENIED;
    if codigo == DXGI_ERROR_UNSUPPORTED {
        "Neste computador o monitor está ligado numa placa e o Otimiza abriu na outra, e o Windows não deixa capturar assim. Em Configurações > Sistema > Tela > Elementos gráficos, deixe o Otimiza em “Economia de energia” e abra de novo."
            .to_string()
    } else if codigo == DXGI_ERROR_NOT_CURRENTLY_AVAILABLE {
        "Outros programas já capturam esta tela no limite do Windows (OBS, Discord, gravadores). Feche um deles e ligue de novo."
            .to_string()
    } else if codigo == E_ACCESSDENIED {
        "O Windows está numa tela protegida (UAC, bloqueio ou Ctrl+Alt+Del). Volte para a área de trabalho e ligue de novo."
            .to_string()
    } else {
        format!("a captura da tela não abriu: {}", mensagem)
    }
}

/// A captura no formato da área de trabalho: FP16 numa tela HDR (`DuplicateOutput1`), BGRA no resto. Sem o
/// `IDXGIOutput5` (Windows antigo) ou recusado, a captura antiga, que é sempre BGRA. O formato de verdade vem do
/// primeiro quadro (`formato_da_captura`): o `ModeDesc` da duplicação é o do modo de vídeo, não o da textura.
fn duplicar(saida: &Saida, gpu: &Gpu) -> Result<IDXGIOutputDuplication, String> {
    // Diagnóstico: `OTIMIZA_FG_FP16` força o caminho HDR numa tela SDR, para provar o FP16 sem monitor HDR.
    let so_fp16 = std::env::var_os("OTIMIZA_FG_FP16").is_some();
    // Diagnóstico: `OTIMIZA_FG_CAPTURA_ANTIGA` pula o `DuplicateOutput1`, para comparar as duas capturas.
    let antiga = std::env::var_os("OTIMIZA_FG_CAPTURA_ANTIGA").is_some();
    if let Some(o5) = saida.saida.cast::<IDXGIOutput5>().ok().filter(|_| !antiga) {
        let formatos: &[DXGI_FORMAT] = if so_fp16 {
            &[DXGI_FORMAT_R16G16B16A16_FLOAT]
        } else {
            &[DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_B8G8R8A8_UNORM]
        };
        match unsafe { o5.DuplicateOutput1(&gpu.device, 0, formatos) } {
            Ok(d) => return Ok(d),
            Err(e)
                if e.code() == DXGI_ERROR_NOT_CURRENTLY_AVAILABLE
                    || e.code() == DXGI_ERROR_UNSUPPORTED && saida.integrada =>
            {
                return Err(explicar_captura(e.code(), &e.message()));
            }
            Err(e) => crate::utils::Logger::info(&format!(
                "gerador: DuplicateOutput1 recusou ({}); vai pela captura antiga",
                e.message()
            )),
        }
    }
    unsafe { saida.saida.DuplicateOutput(&gpu.device) }
        .map_err(|e| explicar_captura(e.code(), &e.message()))
}

/// Pura: algum retângulo que mudou encosta na área do jogo (coordenadas do monitor).
fn toca(retangulos: &[RECT], area: Retangulo) -> bool {
    retangulos.iter().any(|r| {
        r.left < area.x + area.largura
            && r.right > area.x
            && r.top < area.y + area.altura
            && r.bottom > area.y
    })
}

/// O quadro capturado mudou alguma coisa na área do jogo? Sem os metadados (o Windows não mandou), na dúvida conta.
fn mudou_no_jogo(
    d: &IDXGIOutputDuplication,
    info: &DXGI_OUTDUPL_FRAME_INFO,
    area: Retangulo,
) -> bool {
    let total = info.TotalMetadataBufferSize as usize;
    if total == 0 {
        return true;
    }
    let mut sujos = vec![RECT::default(); total / std::mem::size_of::<RECT>() + 1];
    let mut usado = 0u32;
    let tamanho = (sujos.len() * std::mem::size_of::<RECT>()) as u32;
    if unsafe { d.GetFrameDirtyRects(tamanho, sujos.as_mut_ptr(), &mut usado) }.is_err() {
        return true;
    }
    sujos.truncate(usado as usize / std::mem::size_of::<RECT>());
    if toca(&sujos, area) {
        return true;
    }
    let mut movidos = vec![
        DXGI_OUTDUPL_MOVE_RECT::default();
        total / std::mem::size_of::<DXGI_OUTDUPL_MOVE_RECT>() + 1
    ];
    let tamanho = (movidos.len() * std::mem::size_of::<DXGI_OUTDUPL_MOVE_RECT>()) as u32;
    if unsafe { d.GetFrameMoveRects(tamanho, movidos.as_mut_ptr(), &mut usado) }.is_err() {
        return true;
    }
    movidos.truncate(usado as usize / std::mem::size_of::<DXGI_OUTDUPL_MOVE_RECT>());
    let destinos: Vec<RECT> = movidos.iter().map(|m| m.DestinationRect).collect();
    toca(&destinos, area)
}

fn formato_da_textura(t: &ID3D11Texture2D) -> DXGI_FORMAT {
    let mut d = windows::Win32::Graphics::Direct3D11::D3D11_TEXTURE2D_DESC::default();
    unsafe { t.GetDesc(&mut d) };
    d.Format
}

/// O formato que a captura entrega. O PRIMEIRO quadro não serve: pedindo FP16, o Windows manda a imagem inicial em
/// BGRA e só depois passa a FP16 (visto numa GTX 1650). Vale quando dois quadros seguidos concordam; sem quadros
/// (tela parada), o último visto.
fn formato_da_captura(d: &IDXGIOutputDuplication) -> Result<Option<DXGI_FORMAT>, Fim> {
    let mut ultimo: Option<DXGI_FORMAT> = None;
    for _ in 0..20 {
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut recurso: Option<IDXGIResource> = None;
        match unsafe { d.AcquireNextFrame(100, &mut info, &mut recurso) } {
            Ok(()) => {
                let formato = recurso
                    .as_ref()
                    .and_then(|r| r.cast::<ID3D11Texture2D>().ok())
                    .map(|t| formato_da_textura(&t));
                unsafe {
                    let _ = d.ReleaseFrame();
                }
                if let Some(f) = formato {
                    if ultimo == Some(f) {
                        return Ok(Some(f));
                    }
                    ultimo = Some(f);
                }
            }
            Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => {
                return Err(Fim::Refazer("o Windows trocou o modo de vídeo"));
            }
            Err(_) => {}
        }
    }
    Ok(ultimo)
}

fn saida_do_monitor(hwnd: HWND) -> Result<Saida, String> {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let fabrica: IDXGIFactory1 = CreateDXGIFactory1().map_err(|e| e.message().to_string())?;
        // A memória própria de cada placa de verdade (sem a "Microsoft Basic Render"), para saber se o monitor está na
        // integrada de um notebook.
        let mut memorias: Vec<(windows::Win32::Foundation::LUID, u64)> = Vec::new();
        let mut k = 0;
        while let Ok(a) = fabrica.EnumAdapters1(k) {
            if let Ok(d) = a.GetDesc1() {
                if d.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) == 0 {
                    memorias.push((d.AdapterLuid, d.DedicatedVideoMemory as u64));
                }
            }
            k += 1;
        }
        let mut i = 0;
        while let Ok(adaptador) = fabrica.EnumAdapters1(i) {
            let mut j = 0;
            while let Ok(saida) = adaptador.EnumOutputs(j) {
                if let Ok(desc) = saida.GetDesc() {
                    if desc.Monitor == monitor {
                        let r = desc.DesktopCoordinates;
                        let (placa, integrada) = match adaptador.GetDesc1() {
                            Ok(d) => {
                                let outras: Vec<u64> = memorias
                                    .iter()
                                    .filter(|(l, _)| {
                                        l.LowPart != d.AdapterLuid.LowPart
                                            || l.HighPart != d.AdapterLuid.HighPart
                                    })
                                    .map(|(_, m)| *m)
                                    .collect();
                                (
                                    texto_de(&d.Description),
                                    e_integrada(d.DedicatedVideoMemory as u64, &outras),
                                )
                            }
                            Err(_) => (String::new(), false),
                        };
                        return Ok(Saida {
                            dispositivo: texto_de(&desc.DeviceName),
                            placa,
                            integrada,
                            adaptador,
                            saida: saida.cast().map_err(|e| e.message().to_string())?,
                            monitor: Retangulo {
                                x: r.left,
                                y: r.top,
                                largura: r.right - r.left,
                                altura: r.bottom - r.top,
                            },
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
    /// No ritmo do monitor: o objeto que avisa a vez da tela (latência máxima de 1 quadro) e `Present(1)`.
    vez_da_tela: Option<windows::Win32::Foundation::HANDLE>,
}

impl Sobreposicao {
    fn nova(
        gpu: &Gpu,
        area: Retangulo,
        visivel_em_captura: bool,
        hdr: bool,
        pelo_monitor: bool,
    ) -> Result<Sobreposicao, String> {
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
                WS_EX_TOPMOST
                    | WS_EX_TRANSPARENT
                    | WS_EX_LAYERED
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOREDIRECTIONBITMAP,
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

            let _ = SetLayeredWindowAttributes(
                hwnd,
                windows::Win32::Foundation::COLORREF(0),
                255,
                LWA_ALPHA,
            );
            if !visivel_em_captura {
                // Sem isto o gerador capturaria o próprio quadro gerado e interpolaria em cima dele.
                let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
            }

            let dxgi = gpu.dxgi_device()?;
            let dcomp: IDCompositionDevice =
                DCompositionCreateDevice(&dxgi).map_err(|e| e.message().to_string())?;
            let alvo = dcomp
                .CreateTargetForHwnd(hwnd, true)
                .map_err(|e| e.message().to_string())?;
            let visual = dcomp.CreateVisual().map_err(|e| e.message().to_string())?;

            let adaptador: IDXGIAdapter = dxgi.GetAdapter().map_err(|e| e.message().to_string())?;
            let fabrica: IDXGIFactory2 =
                adaptador.GetParent().map_err(|e| e.message().to_string())?;
            let swap = fabrica
                .CreateSwapChainForComposition(
                    &gpu.device,
                    &DXGI_SWAP_CHAIN_DESC1 {
                        Width: area.largura as u32,
                        Height: area.altura as u32,
                        Format: if hdr {
                            DXGI_FORMAT_R16G16B16A16_FLOAT
                        } else {
                            DXGI_FORMAT_B8G8R8A8_UNORM
                        },
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                        BufferCount: 2,
                        Scaling: DXGI_SCALING_STRETCH,
                        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
                        Flags: if pelo_monitor {
                            DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as u32
                        } else {
                            0
                        },
                        ..Default::default()
                    },
                    None,
                )
                .map_err(|e| format!("swap chain: {}", e.message()))?;
            if hdr {
                // scRGB linear: o mesmo espaço da área de trabalho HDR, sem o Windows achatar o brilho.
                let s3: IDXGISwapChain3 = swap.cast().map_err(|e| e.message().to_string())?;
                s3.SetColorSpace1(DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709)
                    .map_err(|e| format!("espaço de cor HDR: {}", e.message()))?;
            }
            // Um quadro na fila, no máximo: cada atualização do monitor recebe o quadro feito para ela, e não um de
            // três atualizações atrás.
            let vez_da_tela = if pelo_monitor {
                let s2: IDXGISwapChain2 = swap.cast().map_err(|e| e.message().to_string())?;
                s2.SetMaximumFrameLatency(1)
                    .map_err(|e| format!("latência da swap chain: {}", e.message()))?;
                Some(s2.GetFrameLatencyWaitableObject())
            } else {
                None
            };
            visual
                .SetContent(&swap)
                .map_err(|e| e.message().to_string())?;
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
                vez_da_tela,
            };
            s.criar_rtv(gpu)?;
            Ok(s)
        }
    }

    fn criar_rtv(&mut self, gpu: &Gpu) -> Result<(), String> {
        unsafe {
            let fundo: ID3D11Texture2D = self
                .swap
                .GetBuffer(0)
                .map_err(|e| e.message().to_string())?;
            let mut rtv = None;
            gpu.device
                .CreateRenderTargetView(&fundo, None, Some(&mut rtv))
                .map_err(|e| e.message().to_string())?;
            self.rtv = rtv;
        }
        Ok(())
    }

    fn apresentar(&self, gpu: &Gpu, t: Option<f32>, cronometro: Option<&mut gpu::Cronometro>) {
        if let Some(rtv) = &self.rtv {
            match cronometro {
                Some(c) => c.medir(gpu, || gpu.compor(rtv, self.largura, self.altura, t)),
                None => gpu.compor(rtv, self.largura, self.altura, t),
            }
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
                let intervalo = if self.vez_da_tela.is_some() { 1 } else { 0 };
                let _ = self.swap.Present(intervalo, DXGI_PRESENT(0));
            }
        }
    }
}

impl Sobreposicao {
    /// No ritmo do monitor: espera a vez da próxima atualização da tela (no máximo `ms`).
    fn esperar_a_vez(&self, ms: u32) {
        if let Some(h) = self.vez_da_tela {
            unsafe {
                windows_sys::Win32::System::Threading::WaitForSingleObjectEx(h.0 as _, ms, 1);
            }
        }
    }
}

impl Drop for Sobreposicao {
    fn drop(&mut self) {
        unsafe {
            if let Some(h) = self.vez_da_tela.take() {
                let _ = windows::Win32::Foundation::CloseHandle(h);
            }
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

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

/// O que dizer quando o jogo está na tela cheia EXCLUSIVA antiga: nesse modo ele toma o monitor, e qualquer janela por
/// cima o derruba do modo; ele volta, troca o modo de vídeo e perde FPS (`trocas_demais`). A tela cheia de hoje (do
/// Windows, por composição) não troca o modo e aguenta o Frame Boost. Vale para qualquer resolução: a saída é o modo Sem
/// bordas, e a resolução esticada vem do Windows.
pub const TELA_CHEIA_EXCLUSIVA: &str = "O jogo saiu e voltou da tela cheia várias vezes com o Frame Boost por cima: é a tela cheia exclusiva antiga, em que nenhuma janela pode ficar por cima dele. No jogo, use Sem bordas. Para jogar esticado (por exemplo 1100x1080): ponha essa resolução no Windows (Configurações → Sistema → Tela) com a escala \"Tela inteira\" no Painel de Controle NVIDIA, e o jogo em Sem bordas usa essa resolução.";

/// O aplicativo da frente está em tela cheia Direct3D exclusiva? (`SHQueryUserNotificationState`, documentada.)
pub(crate) fn tela_cheia_exclusiva() -> bool {
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
    unsafe { SHQueryUserNotificationState() }.is_ok_and(|s| s == QUNS_RUNNING_D3D_FULL_SCREEN)
}

/// Pura: três trocas de modo de vídeo em 90 s com o gerador ligado = o jogo entrando e saindo da tela cheia exclusiva.
pub(crate) fn trocas_demais(trocas: &[Instant], agora: Instant) -> bool {
    trocas
        .iter()
        .filter(|t| agora.duration_since(**t) < Duration::from_secs(90))
        .count()
        >= 3
}

/// A janela em primeiro plano é do jogo? (A sobreposição do gerador nunca fica com o foco: ela não ativa.)
fn jogo_na_frente(pid: u32) -> bool {
    // Laboratório (só desenvolvimento): `OTIMIZA_FG_IGNORAR_FRENTE` mede o custo sem roubar o foco de quem usa o PC.
    if cfg!(debug_assertions) && std::env::var_os("OTIMIZA_FG_IGNORAR_FRENTE").is_some() {
        return true;
    }
    let mut na_frente = 0u32;
    unsafe {
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut na_frente));
    }
    na_frente == pid
}

/// Ctrl+Alt+G desliga o gerador de qualquer lugar, inclusive de dentro do jogo.
const ATALHO: i32 = 0x4F47;

fn segundos(inicio: Instant) -> f64 {
    inicio.elapsed().as_secs_f64()
}

fn executar(config: &Configuracao, parar: &AtomicBool) -> Result<(), String> {
    use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    };
    unsafe {
        timeBeginPeriod(1);
        let _ = RegisterHotKey(
            None,
            ATALHO,
            MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
            b'G' as u32,
        );
    }
    let r = executar_interno(config, parar);
    unsafe {
        let _ = UnregisterHotKey(None, ATALHO);
        timeEndPeriod(1);
    }
    r
}

fn executar_interno(config: &Configuracao, parar: &AtomicBool) -> Result<(), String> {
    let multiplicador = if config.automatico {
        MAXIMO_DO_AUTOMATICO
    } else if config.sem_limite {
        config.multiplicador.clamp(2, MAXIMO_DO_AUTOMATICO)
    } else {
        config.multiplicador.clamp(2, 4)
    };

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

    // Janela redimensionada, movida para outro monitor, minimizada, ou o Windows trocou o modo de vídeo: o gerador se
    // refaz sozinho em vez de desligar. Seis refeitas seguidas que não duram nem 5 s = a janela não para quieta.
    let mut refeitas = 0u32;
    let mut trocas_de_modo: Vec<Instant> = Vec::new();
    let mut seguidas = 0u32;
    loop {
        let Some(hwnd) = esperar_janela(config, parar, pid)? else {
            return Ok(());
        };
        let comeco = Instant::now();
        match sessao(config, parar, hwnd, multiplicador)? {
            Fim::Parou => return Ok(()),
            Fim::Refazer(motivo) => {
                if motivo == "o Windows trocou o modo de vídeo" {
                    trocas_de_modo.push(Instant::now());
                    if trocas_demais(&trocas_de_modo, Instant::now()) {
                        return Err(TELA_CHEIA_EXCLUSIVA.to_string());
                    }
                }
                seguidas = if comeco.elapsed() < Duration::from_secs(5) {
                    seguidas + 1
                } else {
                    0
                };
                if seguidas >= 6 {
                    return Err(format!(
                        "O gerador não conseguiu se firmar na janela do jogo ({}). Deixe o jogo em janela sem bordas e ligue de novo.",
                        motivo
                    ));
                }
                refeitas += 1;
                crate::utils::Logger::info(&format!("gerador: refazendo ({})", motivo));
                publicar(|e| {
                    e.situacao = Situacao::ProcurandoJanela;
                    e.refeitas = refeitas;
                    e.fps_apresentados = 0.0;
                });
            }
        }
    }
}

enum Fim {
    Parou,
    Refazer(&'static str),
}

/// Espera a janela do jogo existir, não estar minimizada e ficar do mesmo tamanho e no mesmo lugar por meio segundo
/// (arrastando a borda, ela muda a cada quadro). `None` = pediram para parar.
fn esperar_janela(
    config: &Configuracao,
    parar: &AtomicBool,
    pid: u32,
) -> Result<Option<HWND>, String> {
    let mut anterior: Option<(HWND, Retangulo)> = None;
    let mut iguais = 0;
    let mut ultima_vida = Instant::now();
    loop {
        if parar.load(Ordering::Relaxed) || bombear_mensagens() {
            return Ok(None);
        }
        if ultima_vida.elapsed() > Duration::from_secs(2) {
            ultima_vida = Instant::now();
            if !super::frames::encontrar_processo(&config.processo).is_some_and(|(p, _)| p == pid) {
                return Err("O jogo fechou.".to_string());
            }
        }
        let atual = janela_do_processo(pid)
            .filter(|h| unsafe { !IsIconic(*h).as_bool() })
            .and_then(|h| area_do_cliente(h).map(|a| (h, a)));
        match atual {
            Some((h, a)) if anterior == Some((h, a)) => {
                iguais += 1;
                if iguais >= 3 {
                    return Ok(Some(h));
                }
            }
            outro => {
                anterior = outro;
                iguais = 0;
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

fn sessao(
    config: &Configuracao,
    parar: &AtomicBool,
    hwnd: HWND,
    multiplicador: u8,
) -> Result<Fim, String> {
    let pid = {
        let mut p = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut p)) };
        p
    };
    let Some(cliente) = area_do_cliente(hwnd) else {
        return Ok(Fim::Refazer("a janela sumiu"));
    };
    let saida = saida_do_monitor(hwnd)?;
    let Some(recorte) = ritmo::recorte(cliente, saida.monitor) else {
        return Ok(Fim::Refazer(
            "a janela está fora do monitor ou pequena demais",
        ));
    };
    let hz = hz_do_monitor(&saida.dispositivo);

    let mut gpu = Gpu::novo(&saida.adaptador)?;
    // Refeito (janela mudou) depois de a guarda aliviar: continua leve.
    gpu.modo_leve(config.leve || estado().leve);
    let duplicacao = duplicar(&saida, &gpu)?;
    let formato = match formato_da_captura(&duplicacao) {
        Ok(f) => f.unwrap_or(DXGI_FORMAT_B8G8R8A8_UNORM),
        Err(fim) => return Ok(fim),
    };
    let hdr = formato == DXGI_FORMAT_R16G16B16A16_FLOAT;
    if !hdr && formato != DXGI_FORMAT_B8G8R8A8_UNORM {
        return Err(format!(
            "a captura da tela veio num formato que o gerador não conhece ({:?})",
            formato
        ));
    }
    // Monitor girado (em pé): a captura vem na orientação do painel. A área do jogo é convertida para a textura, e a
    // imagem, girada de volta na entrada.
    let giro = ritmo::Giro::de_dxgi(unsafe { duplicacao.GetDesc() }.Rotation.0);
    let na_textura = ritmo::na_textura(recorte, giro, saida.monitor.largura, saida.monitor.altura);
    gpu.preparar(recorte.largura as u32, recorte.altura as u32, hdr, giro)?;
    crate::utils::Logger::info(&format!(
        "gerador: {} em {} ({} Hz{}{}{}), {}x{}",
        saida.placa,
        saida.dispositivo,
        hz,
        if hdr { ", HDR" } else { "" },
        if giro != ritmo::Giro::Nenhum {
            format!(", girado {:?}", giro)
        } else {
            String::new()
        },
        if saida.integrada {
            ", placa integrada"
        } else {
            ""
        },
        recorte.largura,
        recorte.altura
    ));

    let area_na_tela = Retangulo {
        x: saida.monitor.x + recorte.x,
        y: saida.monitor.y + recorte.y,
        ..recorte
    };
    // O Auto dentro dos Hz anda no ritmo do monitor: um quadro por atualização da tela, cada um feito para o instante em
    // que aparece. Pelo cronômetro (o 2×/3×/4× e o "passar do limite"), às vezes caíam dois quadros na mesma
    // atualização e nenhum na seguinte: o micro-tranco.
    let pelo_monitor = config.automatico && !config.sem_limite && hz > 0;
    let sobreposicao = Sobreposicao::nova(
        &gpu,
        area_na_tela,
        config.visivel_em_captura,
        hdr,
        pelo_monitor,
    )?;
    let periodo_da_tela = if hz > 0 { 1.0 / hz as f64 } else { 0.0 };
    // O par que está sendo mostrado, pelo instante em que cada real apareceu na tela do jogo (`LastPresentTime`), e o
    // atraso da exibição: o intervalo do jogo mais o que o gerador leva para ter o par pronto (médias).
    let mut par_no_monitor: Option<(f64, f64)> = None;
    let mut ultimo_carimbo: Option<f64> = None;
    let mut carimbo_suave = ritmo::CarimboSuave::default();
    let mut atraso_da_exibicao = ritmo::AtrasoDaExibicao::default();
    let mut real_ja_mostrado = true;

    publicar(|e| {
        e.situacao = Situacao::Gerando;
        e.area = Some(area_na_tela);
        e.multiplicador = multiplicador;
        e.erro = None;
        e.hz = hz;
        e.hdr = hdr;
        e.placa = Some(saida.placa.clone());
        e.integrada = saida.integrada;
    });

    let inicio = Instant::now();
    let mut intervalo = Intervalo::default();
    let mut agenda: std::collections::VecDeque<Apresentacao> = Default::default();
    let mut contadores = Contadores::default();
    let mut relatorio = (Instant::now(), Contadores::default(), Duration::ZERO);
    let mut janela_de_contagem = (segundos(inicio), 0u64, 0u64);
    let mut ultima_checagem = Instant::now();
    let mut cabe_atual = multiplicador;
    // Automático: a fração de quadro que sobra de um real para o outro, e quantos coube no último.
    let mut acumulado_do_automatico = 0.0;
    let mut ultimo_do_automatico = 1u8;
    let mut pedidos_de_troca = 0u32;
    let mut ultimo_real = Instant::now();

    let mut escondida = false;
    let mut escondida_desde: Option<Instant> = None;
    let mut tempo_escondida = Duration::ZERO;
    // Escondida sem quadro novo, pela guarda e com o jogo fora da frente.
    let mut escondida_por = [Duration::ZERO; 3];
    // No ritmo do monitor: atualizações da tela e as que repetiram o quadro (o real seguinte do jogo atrasou).
    let mut exibicao = ritmo::Exibicao::default();
    let mut ultimo_laco = Instant::now();
    // O relógio do carimbo de composição do quadro.
    let (qpc_inicio, qpc_frequencia) = unsafe {
        use windows::Win32::System::Performance::{
            QueryPerformanceCounter, QueryPerformanceFrequency,
        };
        let mut f = 0i64;
        let mut c = 0i64;
        let _ = QueryPerformanceFrequency(&mut f);
        let _ = QueryPerformanceCounter(&mut c);
        (
            c as f64 / f.max(1) as f64 - segundos(inicio),
            f.max(1) as f64,
        )
    };
    let mut composto_do_real: Option<f64> = None;
    let mut atraso_medio: Option<f64> = None;
    let mut guarda = guarda::Guarda::nova(0.0);
    // A guarda conta os quadros do PRÓPRIO jogo pelo PresentMon; sem ele, a captura da tela (presa ao Hz do monitor),
    // como antes. Com ele, a pausa da guarda é o jogo puro: sem sobreposição e sem captura.
    let mut vivo = match super::presentmon::Vivo::iniciar(pid) {
        Ok(v) => Some(v),
        Err(e) => {
            crate::utils::Logger::warn(&format!(
                "gerador: a guarda conta pela captura (PresentMon: {})",
                e
            ));
            None
        }
    };
    let mut quadros_do_jogo = guarda::LinhaDoTempo::default();
    let mut quadros_capturados = guarda::LinhaDoTempo::default();
    // Quando o PresentMon saiu: janela que começou antes disso não é contada por nenhuma das duas fontes (metade de cada
    // uma inflaria a razão e a guarda nunca desligaria).
    let mut troca_de_fonte: Option<f64> = None;
    let mut ultima_conferencia_do_vivo = 0.0f64;
    let mut cronometro = gpu::Cronometro::novo(&gpu);
    // O que chegou à tela, contado pelo PresentMon no próprio Otimiza (a sobreposição é deste processo).
    let mut na_tela_vivo = super::presentmon::Vivo::iniciar_exibidos(std::process::id(), "OtimizaNaTela").ok();
    let mut quadros_na_tela = guarda::LinhaDoTempo::default();
    while !parar.load(Ordering::Relaxed) {
        if bombear_mensagens() {
            return Ok(Fim::Parou);
        }
        if let Some(c) = cronometro.as_mut() {
            c.colher(&gpu);
        }
        if let Some(v) = na_tela_vivo.as_mut() {
            match v.novos() {
                Some(lote) => {
                    for qpc in lote {
                        quadros_na_tela.registrar(qpc as f64 / qpc_frequencia - qpc_inicio);
                    }
                }
                None => na_tela_vivo = None,
            }
        }

        if let Some(v) = vivo.as_mut() {
            match v.novos() {
                Some(lote) => {
                    for qpc in lote {
                        quadros_do_jogo.registrar(qpc as f64 / qpc_frequencia - qpc_inicio);
                    }
                }
                None => {
                    crate::utils::Logger::warn(
                        "gerador: o PresentMon parou; a guarda volta a contar pela captura",
                    );
                    vivo = None;
                    troca_de_fonte = Some(segundos(inicio));
                }
            }
        }
        // PresentMon vivo mas mudo (outro processo apresenta, eventos perdidos): a captura vê o jogo andando e ele não.
        let agora_do_laco = segundos(inicio);
        if vivo.is_some()
            && agora_do_laco > guarda::ATRASO_DA_CONTAGEM + 6.0
            && agora_do_laco - ultima_conferencia_do_vivo > 5.0
        {
            ultima_conferencia_do_vivo = agora_do_laco;
            let ate = agora_do_laco - guarda::ATRASO_DA_CONTAGEM;
            let de = ate - 4.0;
            if !guarda.pausada()
                && quadros_do_jogo.contar(de, ate) == 0
                && quadros_capturados.contar(de, ate) >= 30
            {
                crate::utils::Logger::warn("gerador: o PresentMon não vê quadros do jogo; a guarda volta a contar pela captura");
                vivo = None;
                troca_de_fonte = Some(agora_do_laco);
            }
        }
        let pausa_completa = vivo.is_some() && guarda.pausada();

        // NUNCA UMA IMAGEM CONGELADA POR CIMA DO JOGO: meio segundo sem quadro real e a sobreposição some. Na pausa
        // completa da guarda também: o que aparece é o jogo, sem nada do Otimiza no caminho.
        // E só com o jogo na frente: ligado pelo Otimiza com o jogo atrás, a sobreposição não cobre o próprio Otimiza.
        let sem_quadro = ultimo_real.elapsed() > Duration::from_millis(500);
        let fora_da_frente = !jogo_na_frente(pid);
        let parado = sem_quadro || pausa_completa || fora_da_frente;
        // Por que ficou escondida (para o log de 10 s): o primeiro motivo que vale, nesta ordem.
        let dt_do_laco = ultimo_laco.elapsed();
        ultimo_laco = Instant::now();
        if parado {
            let i = if fora_da_frente {
                2
            } else if pausa_completa {
                1
            } else {
                0
            };
            escondida_por[i] += dt_do_laco;
        }
        if parado != escondida {
            match (parado, escondida_desde.take()) {
                (true, _) => escondida_desde = Some(Instant::now()),
                (false, Some(d)) => tempo_escondida += d.elapsed(),
                _ => {}
            }
            unsafe {
                let _ = ShowWindow(
                    sobreposicao.hwnd,
                    if parado { SW_HIDE } else { SW_SHOWNOACTIVATE },
                );
            }
            escondida = parado;
        }

        // Para o diagnóstico: a cada 10 s, no otimiza.log, o que chegou e o que virou quadro na tela.
        if relatorio.0.elapsed() >= Duration::from_secs(10) {
            let escondida_total =
                tempo_escondida + escondida_desde.map(|d| d.elapsed()).unwrap_or_default();
            let (desde, antes, escondida_antes) = relatorio;
            let segs = desde.elapsed().as_secs_f64().max(0.001);
            // A placa que o gerador ocupou (medida): ms por segundo, % do tempo e por quadro real do jogo.
            let placa = cronometro.as_mut().map(|c| (c.tirar(), c.sem_marca));
            let reais_na_janela = (contadores.reais - antes.reais).max(1) as f64;
            let placa_no_log = match placa {
                Some(((ms, n), sem)) if n > 0 => {
                    let por_s = ms / segs;
                    publicar(|e| e.placa_ms_por_s = Some((por_s * 10.0).round() / 10.0));
                    format!(
                        "placa {:.0} ms/s ({:.1}% do tempo), {:.2} ms por real{}",
                        por_s,
                        por_s / 10.0,
                        ms / reais_na_janela,
                        if sem > 0 { format!(", {} sem medir", sem) } else { String::new() }
                    )
                }
                _ => "placa sem medida".to_string(),
            };
            crate::utils::Logger::info(&format!(
"gerador: 10 s · {:.0} reais/s · {:.0} gerados/s · {:.0} iguais/s · {:.0} fora do jogo/s · {} recusados · escondida {:.0}% (sem quadro {:.1} s, guarda {:.1} s, jogo atrás {:.1} s) · {}× · custo {:.1} ms · {} · na tela {:.0}/s, {:.0}% das atualizações repetidas",
                (contadores.reais - antes.reais) as f64 / segs,
                (contadores.gerados - antes.gerados) as f64 / segs,
                (contadores.iguais - antes.iguais) as f64 / segs,
                (contadores.fora_do_jogo - antes.fora_do_jogo) as f64 / segs,
                contadores.recusados - antes.recusados,
                (escondida_total - escondida_antes).as_secs_f64() / segs * 100.0,
                escondida_por[0].as_secs_f64(),
                escondida_por[1].as_secs_f64(),
                escondida_por[2].as_secs_f64(),
                cabe_atual,
                contadores.custo_ms,
                placa_no_log,
                exibicao.exibidos as f64 / segs,
                exibicao.repetidas_pct()
            ));
            exibicao.zerar();
            relatorio = (Instant::now(), contadores, escondida_total);
            escondida_por = [Duration::ZERO; 3];
        }

        if ultima_checagem.elapsed() > Duration::from_millis(500) {
            ultima_checagem = Instant::now();
            if unsafe { !IsWindow(Some(hwnd)).as_bool() } {
                return Err("O jogo fechou.".to_string());
            }
            if unsafe { IsIconic(hwnd).as_bool() } {
                return Ok(Fim::Refazer("o jogo foi minimizado"));
            }
            if area_do_cliente(hwnd) != Some(cliente) {
                return Ok(Fim::Refazer("a janela mudou de tamanho ou de lugar"));
            }
            let agora = segundos(inicio);
            let dt = agora - janela_de_contagem.0;
            if dt > 0.0 {
                let reais = contadores.reais - janela_de_contagem.1;
                // Pelo FPS de meio segundo, e não quadro a quadro: a captura chega irregular e nunca estabilizaria. Troca só
                // depois de duas janelas pedindo a mesma coisa.
                let fps_janela = reais as f64 / dt;
                if fps_janela > 1.0 {
                    let quer = if config.sem_limite {
                        multiplicador
                    } else {
                        ritmo::multiplicador_que_cabe(multiplicador, 1.0 / fps_janela, hz)
                    };
                    if quer == cabe_atual {
                        pedidos_de_troca = 0;
                    } else {
                        pedidos_de_troca += 1;
                        if pedidos_de_troca >= 2 {
                            cabe_atual = quer;
                            pedidos_de_troca = 0;
                        }
                    }
                }
                let apresentados = (contadores.reais + contadores.gerados) - janela_de_contagem.2;
                // Os 2 s que o PresentMon já entregou (ele entrega de 3 a 4 s depois).
                let exibidos = na_tela_vivo.as_ref().and_then(|_| {
                    let ate = agora - guarda::ATRASO_DA_CONTAGEM;
                    (ate > 2.0).then(|| {
                        (quadros_na_tela.contar(ate - 2.0, ate) as f64 / 2.0 * 10.0).round() / 10.0
                    })
                });
                let gerando = ultimo_real.elapsed() <= Duration::from_secs(2);
                let comparacoes = guarda.razoes.len() as u32;
                publicar(|e| {
                    e.fps_reais = (reais as f64 / dt * 10.0).round() / 10.0;
                    e.fps_apresentados = (apresentados as f64 / dt * 10.0).round() / 10.0;
                    if gerando && !escondida {
                        e.sessao.somar(dt, reais as f64 / dt, apresentados as f64 / dt, exibidos);
                    }
                    e.comparacoes = comparacoes;
                    e.contadores = contadores;
                    e.multiplicador = if config.automatico {
                        ultimo_do_automatico
                    } else {
                        cabe_atual
                    };
                    e.razao_fps = guarda.razao_media();
                    e.atraso_ms = atraso_medio.map(|a| (a * 10.0).round() / 10.0);
                    e.fps_exibidos = exibidos;
                    e.situacao = if gerando {
                        Situacao::Gerando
                    } else {
                        Situacao::SemQuadros
                    };
                });
            }
            janela_de_contagem = (
                agora,
                contadores.reais,
                contadores.reais + contadores.gerados,
            );
        }

        let espera_ms = if pelo_monitor {
            // A espera é pela vez da tela (abaixo); a captura só confere se chegou quadro.
            0
        } else {
            agenda
                .front()
                .map(|a| ((a.instante - segundos(inicio)) * 1000.0).floor().max(0.0) as u32)
                .unwrap_or(8)
                .min(8)
        };

        let pelo_jogo = vivo.is_some();
        let acao = guarda.passo(segundos(inicio), |de, ate| {
            if pelo_jogo {
                Some(quadros_do_jogo.contar(de, ate))
            } else if troca_de_fonte.is_some_and(|t| de < t) {
                None
            } else {
                Some(quadros_capturados.contar(de, ate))
            }
        });
        match acao {
            guarda::Acao::Desligar => {
                let razao = guarda.razao_media();
                match reacao_a_perda(gpu.leve()) {
                    // No FiveM do dono a guarda desligou o gerador 7 vezes (o jogo perdendo de 5% a 35%): antes de
                    // desistir, alivia a placa e confere de novo do zero (a média de antes condenaria o modo leve).
                    ReacaoAPerda::FicarLeve => {
                        gpu.modo_leve(true);
                        guarda = guarda::Guarda::nova(segundos(inicio));
                        crate::utils::Logger::warn(&format!(
                            "gerador: o jogo perdia FPS (razão {:?}); modo leve, conferindo de novo",
                            razao
                        ));
                        publicar(|e| {
                            e.leve = true;
                            e.razao_fps = None;
                        });
                    }
                    ReacaoAPerda::Desligar => {
                        crate::utils::Logger::warn(&format!(
                            "gerador desligado pela guarda: razão de FPS {:?} (já no modo leve)",
                            razao
                        ));
                        publicar(|e| {
                            e.situacao = Situacao::DesligadoPorPerdaDeFps;
                            e.razao_fps = razao;
                            e.fps_apresentados = 0.0;
                        });
                        return Ok(Fim::Parou);
                    }
                }
            }
            guarda::Acao::Pausar => {
                contadores.descartados += agenda.len() as u64;
                agenda.clear();
            }
            _ => {}
        }

        // Pausa completa: nem captura nem apresenta; a janela do jogo aparece sozinha por baixo.
        if pausa_completa {
            std::thread::sleep(Duration::from_millis(8));
            continue;
        }

        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut recurso: Option<IDXGIResource> = None;
        match unsafe { duplicacao.AcquireNextFrame(espera_ms, &mut info, &mut recurso) } {
            Ok(()) => {
                // Só conta o que mudou DENTRO da janela do jogo: a cena do Otimiza, um vídeo no navegador ou o relógio
                // da barra de tarefas no mesmo monitor inflavam o FPS do jogo, e o multiplicador caía para 1×.
                if info.LastPresentTime != 0 && {
                    let no_jogo = mudou_no_jogo(&duplicacao, &info, na_textura);
                    if !no_jogo {
                        contadores.fora_do_jogo += 1;
                    }
                    no_jogo
                } {
                    // Igual ao último real não é quadro novo do jogo (num monitor girado, a tela recomposta pela própria
                    // sobreposição chega assim). Formato diferente passa, para a trava de HDR abaixo refazer.
                    if let Some(tex) = recurso
                        .as_ref()
                        .and_then(|r| r.cast::<ID3D11Texture2D>().ok())
                        .filter(|t| {
                            if formato_da_textura(t) != formato {
                                return true;
                            }
                            // Só em monitor girado: é lá que a tela recomposta pela própria sobreposição chega como
                            // quadro "novo". Fora dele, a comparação só custaria placa.
                            if giro == ritmo::Giro::Nenhum {
                                return true;
                            }
                            let igual = gpu.igual_ao_atual(t, na_textura);
                            if igual {
                                contadores.iguais += 1;
                            }
                            !igual
                        })
                    {
                        let chegada = segundos(inicio);
                        let medido = Instant::now();
                        composto_do_real =
                            Some(info.LastPresentTime as f64 / qpc_frequencia - qpc_inicio);
                        // A pessoa ligou ou desligou o HDR do Windows com o gerador ligado: copiar entre formatos
                        // diferentes daria lixo na tela.
                        if formato_da_textura(&tex) != formato {
                            unsafe {
                                let _ = duplicacao.ReleaseFrame();
                            }
                            return Ok(Fim::Refazer("o formato da tela mudou (HDR)"));
                        }
                        gpu.receber(&tex, na_textura);
                        intervalo.registrar(chegada);
                        quadros_capturados.registrar(chegada);
                        contadores.reais += 1;
                        ultimo_real = Instant::now();

                        contadores.descartados += agenda.len() as u64;
                        agenda.clear();
                        if pelo_monitor {
                            par_no_monitor = None;
                            real_ja_mostrado = false;
                            // Sem carimbo (raro), vale a chegada.
                            let carimbo = carimbo_suave
                                .suavizar(composto_do_real.filter(|c| *c > 0.0).unwrap_or(chegada));
                            let anterior = ultimo_carimbo.replace(carimbo);
                            if gpu.tem_par() && intervalo.pode_gerar() && !guarda.pausada() {
                                match cronometro.as_mut() {
                                    Some(c) => c.medir(&gpu, || gpu.estimar()),
                                    None => gpu.estimar(),
                                }
                                if let Some(a) = anterior.filter(|a| carimbo > *a) {
                                    par_no_monitor = Some((a, carimbo));
                                    atraso_da_exibicao
                                        .registrar(carimbo - a, segundos(inicio) - carimbo);
                                }
                            }
                        } else
                        // A tela já recebe o que o monitor mostra: não gasta placa à toa.
                        if gpu.tem_par()
                            && intervalo.pode_gerar()
                            && !guarda.pausada()
                            && cabe_atual > 1
                        {
                            // SEM ESPERAR A PLACA. Antes, a nota do quadro (um passe a mais em meia resolução e uma
                            // leitura de volta) e a espera pelo fim do trabalho paravam o gerador até 28 ms por quadro
                            // e faziam a placa trocar de contexto com o jogo o tempo todo: o jogo perdia ~30% do FPS.
                            // Movimento que não dá para interpolar (troca de cena, giro rapidíssimo) é decidido pixel a
                            // pixel no `Final`: onde os dois reais discordam demais, sai o real mais próximo.
                            match cronometro.as_mut() {
                                Some(c) => c.medir(&gpu, || gpu.estimar()),
                                None => gpu.estimar(),
                            }
                            // O que custou para o processador mandar o trabalho (a placa trabalha depois, em paralelo).
                            let custo = medido.elapsed().as_secs_f64() * 1000.0;
                            contadores.custo_ms = if contadores.custo_ms == 0.0 {
                                custo
                            } else {
                                contadores.custo_ms * 0.9 + custo * 0.1
                            };
                            let pronto = segundos(inicio);
                            let janela = intervalo.estimado.unwrap_or(0.0) - (pronto - chegada);
                            let cabe = if config.automatico {
                                let m = ritmo::quadros_no_automatico(
                                    &mut acumulado_do_automatico,
                                    intervalo.estimado.unwrap_or(0.0),
                                    if config.sem_limite { hz * 2 } else { hz },
                                    multiplicador,
                                );
                                ultimo_do_automatico = m;
                                m
                            } else {
                                cabe_atual
                            };
                            agenda.extend(ritmo::agenda(pronto, janela.max(0.0), cabe));
                        } else {
                            agenda.push_back(Apresentacao {
                                instante: chegada,
                                fase: Fase::Real,
                            });
                        }
                    }
                }
                unsafe {
                    let _ = duplicacao.ReleaseFrame();
                }
            }
            Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => {}
            // Troca de resolução, UAC, tela cheia exclusiva: o Windows invalida a captura. Refaz quando a tela voltar.
            Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => {
                return Ok(Fim::Refazer("o Windows trocou o modo de vídeo"));
            }
            Err(e) => return Err(format!("captura: {}", e.message())),
        }

        if pelo_monitor {
            // A vez da tela: um quadro por atualização do monitor.
            sobreposicao.esperar_a_vez(50);
            let agora = segundos(inicio);
            // O instante do jogo que esta atualização mostra: a hora em que ela aparece, menos o atraso.
            let no_jogo = agora + periodo_da_tela - atraso_da_exibicao.segundos();
            let t = par_no_monitor
                .filter(|_| !guarda.pausada())
                .map(|(a, b)| ritmo::t_no_tempo(no_jogo, a, b));
            match t {
                Some(Some(t)) => {
                    sobreposicao.apresentar(&gpu, Some(t), cronometro.as_mut());
                    contadores.gerados += 1;
                    exibicao.exibidos += 1;
                }
                _ => {
                    // Já mostrado: o próximo real do jogo ainda não chegou, e o mesmo quadro fica mais uma atualização.
                    if !real_ja_mostrado {
                        exibicao.exibidos += 1;
                    }
                    sobreposicao.apresentar(&gpu, None, cronometro.as_mut());
                    real_ja_mostrado = true;
                    if let Some(composto) = composto_do_real.take() {
                        let atraso = (segundos(inicio) - composto) * 1000.0;
                        if (0.0..200.0).contains(&atraso) {
                            atraso_medio =
                                Some(atraso_medio.map_or(atraso, |m| m * 0.9 + atraso * 0.1));
                        }
                    }
                }
            }
            exibicao.atualizacoes += 1;
            continue;
        }

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
                    sobreposicao.apresentar(&gpu, Some(t), cronometro.as_mut());
                    contadores.gerados += 1;
                }
                Fase::Real => {
                    sobreposicao.apresentar(&gpu, None, cronometro.as_mut());
                    if let Some(composto) = composto_do_real.take() {
                        let atraso = (segundos(inicio) - composto) * 1000.0;
                        if (0.0..200.0).contains(&atraso) {
                            atraso_medio =
                                Some(atraso_medio.map_or(atraso, |m| m * 0.9 + atraso * 0.1));
                        }
                    }
                }
            }
        }
    }

    Ok(Fim::Parou)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test --lib -- --ignored geracao::tests::sobre_o_jogo_de_teste --nocapture`
    #[test]
    #[ignore]
    fn sobre_o_jogo_de_teste() {
        // Como o app: o Tauri declara DPI por monitor, e sem isso o `DuplicateOutput1` (HDR) recusa.
        unsafe {
            let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
                windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
        }
        let pasta = std::path::PathBuf::from(
            std::env::var("OTIMIZA_FG_PASTA").unwrap_or_else(|_| ".".into()),
        );
        let mult: u8 = std::env::var("OTIMIZA_FG_MULT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let e = ligar(Configuracao {
            processo: std::env::var("OTIMIZA_FG_PROCESSO")
                .unwrap_or_else(|_| "jogo_de_teste".into()),
            multiplicador: mult,
            visivel_em_captura: std::env::var_os("OTIMIZA_FG_VISIVEL").is_some(),
            // OTIMIZA_FG_MULT=0: o automático. OTIMIZA_FG_SEM_LIMITE: passa do limite do monitor.
            automatico: mult == 0,
            sem_limite: std::env::var_os("OTIMIZA_FG_SEM_LIMITE").is_some(),
            leve: std::env::var_os("OTIMIZA_FG_LEVE").is_some(),
        })
        .unwrap();
        println!("inicio: {:?}", e);
        std::thread::sleep(Duration::from_secs(2));
        // Sem `OTIMIZA_FG_PASTA`, não grava imagens (gravar pesa e falsearia a medição de custo).
        let gravar = std::env::var_os("OTIMIZA_FG_PASTA").is_some();
        for n in 0..(if gravar { 12 } else { 0 }) {
            let sub = pasta.join(format!("{:02}", n));
            let _ = std::fs::create_dir_all(&sub);
            gravar_proximos_quadros(sub, 2);
            std::thread::sleep(Duration::from_millis(400));
        }
        let extra: u64 = std::env::var("OTIMIZA_FG_SEGUNDOS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        for _ in 0..extra {
            std::thread::sleep(Duration::from_secs(1));
            let e = estado();
            println!(
                "{:?} reais={} apresentados={} razao={:?} atraso_ms={:?} hz={} hdr={} refeitas={} placa={:?} placa_ms_por_s={:?} contadores={:?} erro={:?}",
                e.situacao,
                e.fps_reais,
                e.fps_apresentados,
                e.razao_fps,
                e.atraso_ms,
                e.hz,
                e.hdr,
                e.refeitas,
                e.placa,
                e.placa_ms_por_s,
                e.contadores,
                e.erro
            );
        }
        desligar();
    }
}

#[cfg(test)]
mod tests_da_guarda_leve {
    use super::*;

    #[test]
    fn a_primeira_perda_alivia_e_a_segunda_desliga() {
        assert_eq!(reacao_a_perda(false), ReacaoAPerda::FicarLeve);
        assert_eq!(reacao_a_perda(true), ReacaoAPerda::Desligar);
    }
}

#[cfg(test)]
mod tests_puros {
    use super::*;

    #[test]
    fn hz_vem_do_monitor_do_jogo_e_nao_do_principal() {
        let ms = [(r"\\.\DISPLAY1", true, 60), (r"\\.\DISPLAY2", false, 180)];
        assert_eq!(hz_escolhido(&ms, r"\\.\DISPLAY2"), 180);
        assert_eq!(hz_escolhido(&ms, r"\\.\display2"), 180);
        assert_eq!(
            hz_escolhido(&ms, r"\\.\DISPLAY9"),
            60,
            "sem achar, o principal"
        );
        assert_eq!(hz_escolhido(&[], "x"), 0, "sem monitor nenhum, sem teto");
    }

    #[test]
    fn integrada_de_notebook_hibrido() {
        const MB: u64 = 1 << 20;
        assert!(
            e_integrada(128 * MB, &[4096 * MB]),
            "Intel UHD + RTX de 4 GB"
        );
        assert!(
            !e_integrada(4096 * MB, &[128 * MB]),
            "o monitor na dedicada"
        );
        assert!(!e_integrada(512 * MB, &[]), "só a integrada: nada a avisar");
        assert!(!e_integrada(2048 * MB, &[8192 * MB]), "duas dedicadas");
    }

    #[test]
    fn captura_recusada_explica_o_motivo() {
        assert!(explicar_captura(DXGI_ERROR_UNSUPPORTED, "").contains("Economia de energia"));
        assert!(explicar_captura(DXGI_ERROR_NOT_CURRENTLY_AVAILABLE, "").contains("OBS"));
        assert!(explicar_captura(windows::Win32::Foundation::E_ACCESSDENIED, "").contains("UAC"));
        assert!(explicar_captura(windows::Win32::Foundation::E_FAIL, "falhou").ends_with("falhou"));
    }

    #[test]
    fn tres_trocas_de_modo_em_90_s_e_tela_cheia_exclusiva() {
        let agora = Instant::now();
        let s = |x: u64| agora - Duration::from_secs(x);
        assert!(!trocas_demais(&[s(10), s(5)], agora));
        assert!(trocas_demais(&[s(60), s(30), s(1)], agora));
        assert!(
            !trocas_demais(&[s(300), s(200), s(1)], agora),
            "espaçadas: alt+tab normal"
        );
        assert!(TELA_CHEIA_EXCLUSIVA.contains("Sem bordas"));
    }

    #[test]
    fn so_conta_o_que_mudou_dentro_do_jogo() {
        let jogo = Retangulo {
            x: 100,
            y: 100,
            largura: 1280,
            altura: 720,
        };
        let r = |l, t, rr, b| RECT {
            left: l,
            top: t,
            right: rr,
            bottom: b,
        };
        assert!(toca(&[r(500, 400, 520, 420)], jogo), "dentro");
        assert!(toca(&[r(0, 0, 150, 150)], jogo), "encostando na borda");
        assert!(
            !toca(&[r(1500, 0, 1900, 1000)], jogo),
            "a janela do Otimiza ao lado não conta"
        );
        assert!(
            !toca(&[r(0, 1040, 1920, 1080)], jogo),
            "a barra de tarefas não conta"
        );
        assert!(!toca(&[], jogo));
    }

    #[test]
    fn nome_do_dispositivo_sem_zeros() {
        let mut w = [0u16; 32];
        for (i, c) in r"\\.\DISPLAY1".encode_utf16().enumerate() {
            w[i] = c;
        }
        assert_eq!(texto_de(&w), r"\\.\DISPLAY1");
    }
}

#[cfg(test)]
mod teste_do_limite {
    /// `OTIMIZA_FG_EXE=... OTIMIZA_FG_LIMITE=90` aplica; `OTIMIZA_FG_LIMITE=0` desfaz.
    #[test]
    #[ignore]
    fn limite_no_driver() {
        use crate::modules::changelog::ChangeLog;
        let exe = std::env::var("OTIMIZA_FG_EXE").expect("OTIMIZA_FG_EXE");
        let fps: u32 = std::env::var("OTIMIZA_FG_LIMITE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let mut log = ChangeLog::load();
        let otimizador = crate::modules::windows::WindowsOptimizer::new();
        if fps == 0 {
            let id = crate::modules::windows::nvdriver::id_do_limite(&exe);
            println!("desfazer: {:?}", otimizador.revert(&id, &mut log));
        } else {
            println!(
                "aplicar: {:?}",
                otimizador.limitar_fps_nvidia(&exe, None, fps, &mut log)
            );
        }
    }
}

#[cfg(test)]
mod teste_do_monitor {
    #[test]
    #[ignore]
    fn monitor_principal() {
        for m in super::super::display::monitores() {
            println!(
                "{} principal={} {}x{} {}Hz",
                m.dispositivo, m.principal, m.largura, m.altura, m.hz_atual
            );
        }
    }
}

#[cfg(test)]
mod gravador_de_quadros {
    use super::*;

    /// Uma textura já em memória do sistema (STAGING) direto para BMP.
    pub fn salvar_bmp(
        gpu: &Gpu,
        t: &ID3D11Texture2D,
        l: usize,
        a: usize,
        caminho: &std::path::Path,
    ) {
        use windows::Win32::Graphics::Direct3D11::*;
        unsafe {
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            gpu.ctx.Map(t, 0, D3D11_MAP_READ, 0, Some(&mut m)).unwrap();
            let mut dados = Vec::with_capacity(54 + l * a * 4);
            dados.extend_from_slice(b"BM");
            dados.extend_from_slice(&((54 + l * a * 4) as u32).to_le_bytes());
            dados.extend_from_slice(&[0, 0, 0, 0]);
            dados.extend_from_slice(&54u32.to_le_bytes());
            dados.extend_from_slice(&40u32.to_le_bytes());
            dados.extend_from_slice(&(l as i32).to_le_bytes());
            dados.extend_from_slice(&(-(a as i32)).to_le_bytes());
            dados.extend_from_slice(&1u16.to_le_bytes());
            dados.extend_from_slice(&32u16.to_le_bytes());
            dados.extend_from_slice(&[0u8; 24]);
            for y in 0..a {
                dados.extend_from_slice(std::slice::from_raw_parts(
                    (m.pData as *const u8).add(y * m.RowPitch as usize),
                    l * 4,
                ));
            }
            gpu.ctx.Unmap(t, 0);
            std::fs::write(caminho, dados).unwrap();
        }
    }

    /// Grava quadros CONSECUTIVOS do jogo (só os que mudaram dentro da janela dele), recortados, para o teste de
    /// qualidade do gerador: de N e N+2 ele cria o do meio, comparado com o N+1 de verdade. Espera o jogo vir para a
    /// frente se mexendo. `OTIMIZA_FG_PROCESSO`, `OTIMIZA_FG_PASTA`, `OTIMIZA_FG_QUADROS` (60).
    #[test]
    #[ignore]
    fn grava_quadros_do_jogo() {
        use windows::Win32::Graphics::Direct3D11::*;
        unsafe {
            let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
                windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
        }
        let processo = std::env::var("OTIMIZA_FG_PROCESSO").expect("OTIMIZA_FG_PROCESSO");
        let pasta =
            std::path::PathBuf::from(std::env::var("OTIMIZA_FG_PASTA").expect("OTIMIZA_FG_PASTA"));
        let quantos: usize = std::env::var("OTIMIZA_FG_QUADROS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        let _ = std::fs::create_dir_all(&pasta);
        let (pid, _) = super::super::frames::encontrar_processo(&processo).expect("jogo aberto");
        let limite = Instant::now() + Duration::from_secs(3600);
        // Espera o jogo na frente.
        while !jogo_na_frente(pid) {
            assert!(
                Instant::now() < limite,
                "o jogo não veio para a frente em 1 h"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
        std::thread::sleep(Duration::from_millis(1500));
        let hwnd = janela_do_processo(pid).expect("janela");
        let cliente = area_do_cliente(hwnd).expect("área");
        let saida = saida_do_monitor(hwnd).unwrap();
        let recorte = ritmo::recorte(cliente, saida.monitor).expect("recorte");
        let gpu = Gpu::novo(&saida.adaptador).unwrap();
        let dup = duplicar(&saida, &gpu).unwrap();
        println!(
            "gravando {} quadros de {:?} ({}x{})",
            quantos, processo, recorte.largura, recorte.altura
        );
        // Em texturas que o processador lê (memória do sistema): gravar no disco no meio perderia quadros.
        let nova = || -> ID3D11Texture2D {
            let mut t = None;
            unsafe {
                gpu.device
                    .CreateTexture2D(
                        &D3D11_TEXTURE2D_DESC {
                            Width: recorte.largura as u32,
                            Height: recorte.altura as u32,
                            MipLevels: 1,
                            ArraySize: 1,
                            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            SampleDesc: DXGI_SAMPLE_DESC {
                                Count: 1,
                                Quality: 0,
                            },
                            Usage: D3D11_USAGE_STAGING,
                            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                            ..Default::default()
                        },
                        None,
                        Some(&mut t),
                    )
                    .unwrap();
            }
            t.unwrap()
        };
        let guardados: Vec<ID3D11Texture2D> = (0..quantos).map(|_| nova()).collect();
        let freq = {
            let mut f = 0i64;
            unsafe {
                let _ = windows::Win32::System::Performance::QueryPerformanceFrequency(&mut f);
            }
            f as f64
        };
        let mut instantes: Vec<f64> = Vec::new();
        while instantes.len() < quantos && Instant::now() < limite {
            let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut r: Option<IDXGIResource> = None;
            if unsafe { dup.AcquireNextFrame(100, &mut info, &mut r) }.is_err() {
                continue;
            }
            // Só o jogo: se outra janela vier para a frente, joga fora o que tinha e recomeça (nunca grava outra tela,
            // e a sequência precisa ser contínua).
            if !jogo_na_frente(pid) {
                instantes.clear();
                unsafe {
                    let _ = dup.ReleaseFrame();
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            if info.LastPresentTime != 0 && mudou_no_jogo(&dup, &info, recorte) {
                if let Some(t) = r.and_then(|r| r.cast::<ID3D11Texture2D>().ok()) {
                    if formato_da_textura(&t) == DXGI_FORMAT_B8G8R8A8_UNORM {
                        unsafe {
                            gpu.ctx.CopySubresourceRegion(
                                &guardados[instantes.len()],
                                0,
                                0,
                                0,
                                0,
                                &t,
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
                        instantes.push(info.LastPresentTime as f64 / freq);
                    }
                }
            }
            unsafe {
                let _ = dup.ReleaseFrame();
            }
        }
        assert!(jogo_na_frente(pid), "o jogo saiu da frente: nada gravado");
        let mut tempos = String::new();
        for (n, t) in guardados.iter().take(instantes.len()).enumerate() {
            salvar_bmp(
                &gpu,
                t,
                recorte.largura as usize,
                recorte.altura as usize,
                &pasta.join(format!("{:03}.bmp", n)),
            );
            tempos.push_str(&format!(
                "{:03} {:.4}
",
                n, instantes[n]
            ));
        }
        let quadros: Vec<(f64, ())> = instantes.iter().map(|t| (*t, ())).collect();
        std::fs::write(pasta.join("tempos.txt"), tempos).unwrap();
        let dt: Vec<f64> = quadros
            .windows(2)
            .map(|w| (w[1].0 - w[0].0) * 1000.0)
            .collect();
        println!(
            "gravados {} · intervalo médio {:.1} ms",
            quadros.len(),
            dt.iter().sum::<f64>() / dt.len().max(1) as f64
        );
    }
}

#[cfg(test)]
mod teste_da_captura_crua {
    use super::*;

    /// Grava a textura que a Duplicação entrega para o monitor do jogo de teste, sem o gerador, e diz o giro.
    #[test]
    #[ignore]
    fn grava_a_textura_crua() {
        unsafe {
            let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
                windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
        }
        let pasta = std::path::PathBuf::from(
            std::env::var("OTIMIZA_FG_PASTA").unwrap_or_else(|_| ".".into()),
        );
        let (pid, _) =
            super::super::frames::encontrar_processo("jogo_de_teste").expect("jogo aberto");
        let hwnd = janela_do_processo(pid).expect("janela");
        let saida = saida_do_monitor(hwnd).unwrap();
        let gpu = Gpu::novo(&saida.adaptador).unwrap();
        let dup = duplicar(&saida, &gpu).unwrap();
        let desc = unsafe { dup.GetDesc() };
        println!(
            "monitor {:?} giro {:?} modo {}x{}",
            saida.monitor, desc.Rotation, desc.ModeDesc.Width, desc.ModeDesc.Height
        );
        for n in 0..5 {
            let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut r: Option<IDXGIResource> = None;
            if unsafe { dup.AcquireNextFrame(200, &mut info, &mut r) }.is_ok() {
                if let Some(t) = r.and_then(|r| r.cast::<ID3D11Texture2D>().ok()) {
                    let mut d =
                        windows::Win32::Graphics::Direct3D11::D3D11_TEXTURE2D_DESC::default();
                    unsafe { t.GetDesc(&mut d) };
                    println!("textura {}x{} formato {:?}", d.Width, d.Height, d.Format);
                    if n == 4 {
                        println!("{:?}", gravar_bmp(&gpu, &t, &pasta.join("crua.bmp")));
                    }
                }
                unsafe {
                    let _ = dup.ReleaseFrame();
                }
            }
        }
    }
}

#[cfg(test)]
mod teste_sem_gerador {
    use super::*;

    /// A base "desligado" para comparar com o gerador ligado.
    #[test]
    #[ignore]
    fn quadros_do_jogo_sem_gerador() {
        let processo =
            std::env::var("OTIMIZA_FG_PROCESSO").unwrap_or_else(|_| "jogo_de_teste".into());
        let (pid, _) = super::super::frames::encontrar_processo(&processo).expect("jogo aberto");
        let hwnd = janela_do_processo(pid).expect("janela");
        let saida = saida_do_monitor(hwnd).unwrap();
        let gpu = Gpu::novo(&saida.adaptador).unwrap();
        let dup = unsafe { saida.saida.DuplicateOutput(&gpu.device) }.unwrap();
        for segundo in 0..6 {
            let inicio = Instant::now();
            let mut quadros = 0;
            while inicio.elapsed() < Duration::from_secs(1) {
                let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut r: Option<IDXGIResource> = None;
                if unsafe { dup.AcquireNextFrame(50, &mut info, &mut r) }.is_ok() {
                    if info.LastPresentTime != 0 {
                        quadros += 1;
                    }
                    unsafe {
                        let _ = dup.ReleaseFrame();
                    }
                }
            }
            println!("sem gerador, segundo {}: {} quadros", segundo + 1, quadros);
        }
    }
}
