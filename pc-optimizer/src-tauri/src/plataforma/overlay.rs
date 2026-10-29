// FPS por cima do jogo, opcional: Desligado (padrão), Mínimo (só o FPS) ou Detalhado (FPS, 1% low e tempo de quadro).
//
// Mesma regra do resto do produto: só número MEDIDO. O FPS do jogo vem do PresentMon ao vivo (precisa do Otimiza
// como administrador); sem ele, o overlay diz isso em vez de inventar. Com o Frame Boost do Otimiza ligado, mostra o
// FPS do PRÓPRIO jogo, como o `cl_drawfps` (decisão do dono, 08/10); o que chegou à tela fica no painel do Frame Boost.
//
// A janela é do Otimiza, por fora do jogo: não injeta nada. Não pega foco
// e não recebe clique; aparece em print, como o contador do jogo. Some quando o jogo sai da frente. Anticheat que barra
// sobreposição barra também.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Modo {
    #[default]
    Desligado,
    Minimo,
    Detalhado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Canto {
    #[default]
    SuperiorEsquerdo,
    SuperiorDireito,
    InferiorEsquerdo,
    InferiorDireito,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub modo: Modo,
    pub canto: Canto,
    /// Jogos com ressalva (servidor de RP, anticheat que não documenta) em que a pessoa já disse "mostrar mesmo assim":
    /// só nesses o overlay liga sozinho quando o jogo abre.
    pub confirmados: Vec<String>,
    /// Abrir o contador junto com o Frame Boost, mesmo com o modo Desligado. Desligado por padrão desde 08/10/2026: o dono
    /// não quer contador do Otimiza por cima do jogo com o Frame Boost (o do jogo serve); fica para quem pedir.
    pub com_frame_boost: bool,
    /// Versão do arquivo: a 2 desliga o `com_frame_boost` que as anteriores gravavam ligado por padrão. Ausente = 0 (o
    /// padrão do struct é a versão atual).
    #[serde(default)]
    pub versao: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            modo: Modo::Desligado,
            canto: Canto::SuperiorEsquerdo,
            confirmados: Vec::new(),
            com_frame_boost: false,
            versao: VERSAO_DA_CONFIG,
        }
    }
}

/// O que a janela mostra.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Leitura {
    pub modo: Modo,
    /// "FPS" (do jogo) ou "FPS na tela" (com o Frame Boost).
    pub rotulo: &'static str,
    pub fps: Option<f64>,
    pub low_1pct: Option<f64>,
    pub quadro_ms: Option<f64>,
    /// Multiplicador do Frame Boost do Otimiza, quando ele gera quadros.
    pub boost: Option<u8>,
    pub nota: Option<String>,
    /// Com a escala do Frame Boost: o nível ("Qualidade").
    #[serde(default)]
    pub escala: Option<&'static str>,
    /// Gerando quadros: o que chegou à tela (medido pelo PresentMon, ou o apresentado pelo gerador).
    #[serde(default)]
    pub na_tela: Option<f64>,
}

pub const MARGEM: i32 = 12;

/// Pura: o modo da janela. Pelo Frame Boost, o escolhido em Ajustes, ou o Mínimo se lá estiver Desligado.
/// `no_lugar_do_fivem`: com o contador do Otimiza no lugar do `cl_drawfps`, ele é sempre o Mínimo, no estilo do FiveM (o
/// Detalhado, um cartão escuro, ficava no canto onde a pessoa espera o número do jogo).
pub fn modo_efetivo(escolhido: Modo, pelo_frame_boost: bool, no_lugar_do_fivem: bool) -> Modo {
    match (escolhido, pelo_frame_boost) {
        (Modo::Desligado, false) => Modo::Desligado,
        (Modo::Desligado, true) => Modo::Minimo,
        _ if no_lugar_do_fivem => Modo::Minimo,
        (m, _) => m,
    }
}

/// Pura: o tamanho da janela em pixels.
pub fn tamanho(modo: Modo) -> (i32, i32) {
    match modo {
        Modo::Detalhado => (220, 104),
        // Do tamanho do contador do próprio jogo, no mesmo canto: fica por cima dele.
        _ => (118, 32),
    }
}

/// Pura: onde a janela fica, no canto pedido da área do jogo.
pub fn posicao(area: (i32, i32, i32, i32), canto: Canto, tamanho: (i32, i32)) -> (i32, i32) {
    let (x, y, w, h) = area;
    let (lw, lh) = tamanho;
    match canto {
        Canto::SuperiorEsquerdo => (x + MARGEM, y + MARGEM),
        Canto::SuperiorDireito => (x + w - lw - MARGEM, y + MARGEM),
        Canto::InferiorEsquerdo => (x + MARGEM, y + h - lh - MARGEM),
        Canto::InferiorDireito => (x + w - lw - MARGEM, y + h - lh - MARGEM),
    }
}

/// Pura: dos instantes (em segundos) dos quadros do jogo, o FPS do último segundo, o tempo médio de quadro desse
/// segundo e o 1% low dos últimos 10 s. O 1% low segue a regra do produto (`fluidez::AMOSTRA_PARA_1PCT`): com menos
/// de mil quadros não existe, para não virar número com cara de medido.
pub fn estatisticas(instantes: &[f64], agora: f64) -> (Option<f64>, Option<f64>, Option<f64>) {
    let ultimo_segundo: Vec<f64> = instantes
        .iter()
        .copied()
        .filter(|t| *t > agora - 1.0 && *t <= agora)
        .collect();
    let fps = (ultimo_segundo.len() >= 2).then(|| ultimo_segundo.len() as f64);
    let quadro_ms = (ultimo_segundo.len() >= 2).then(|| {
        (ultimo_segundo[ultimo_segundo.len() - 1] - ultimo_segundo[0]) * 1000.0
            / (ultimo_segundo.len() - 1) as f64
    });
    let janela: Vec<f64> = instantes
        .iter()
        .copied()
        .filter(|t| *t > agora - 10.0 && *t <= agora)
        .collect();
    let mut intervalos: Vec<f64> = janela
        .windows(2)
        .map(|w| (w[1] - w[0]) * 1000.0)
        .filter(|d| *d > 0.0)
        .collect();
    let low = (intervalos.len() >= crate::core::fluidez::AMOSTRA_PARA_1PCT).then(|| {
        intervalos.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let n = (intervalos.len() / 100).max(1);
        1000.0 / (intervalos[..n].iter().sum::<f64>() / n as f64)
    });
    (fps, quadro_ms, low)
}

/// O número do contador, firme como o `cl_drawfps`. A medida de meio segundo pula (a captura chega irregular) e às vezes
/// falha por um instante (o PresentMon entrega em lotes, o Frame Boost pausa para conferir): sem isto o contador ia de
/// 140 a 90 e a zero e voltava. Média móvel, que acompanha rápido quando a mudança se confirma, e o último número bom
/// segurado por até 3 s quando a medida some. Nunca mostra um número que não veio de uma medida.
#[derive(Debug, Clone, Default)]
pub struct Suavizador {
    valor: Option<f64>,
    ultima_boa: f64,
    /// Leituras seguidas longe (mais de 20%) do valor mostrado, na mesma direção.
    longe: i32,
}

impl Suavizador {
    pub const SEGURA_S: f64 = 3.0;

    /// Pura: a nova leitura (`None` ou zero = sem medida agora) no instante `agora` (segundos) → o número a mostrar.
    pub fn passo(&mut self, medida: Option<f64>, agora: f64) -> Option<f64> {
        match medida.filter(|v| *v >= 1.0) {
            Some(v) => {
                let novo = match self.valor {
                    None => v,
                    Some(atual) => {
                        let rel = (v - atual) / atual.max(1.0);
                        self.longe = if rel.abs() > 0.2 {
                            if self.longe.signum() == rel.signum() as i32 {
                                self.longe + rel.signum() as i32
                            } else {
                                rel.signum() as i32
                            }
                        } else {
                            0
                        };
                        // Mudança confirmada (duas leituras seguidas longe, para o mesmo lado): vai quase direto.
                        let alfa = if self.longe.abs() >= 2 { 0.75 } else { 0.3 };
                        atual + alfa * (v - atual)
                    }
                };
                self.valor = Some(novo);
                self.ultima_boa = agora;
            }
            None => {
                if agora - self.ultima_boa > Self::SEGURA_S {
                    self.valor = None;
                    self.longe = 0;
                }
            }
        }
        self.valor.map(|v| v.round())
    }
}

/// Pura: a leitura da janela: o FPS do jogo, com ou sem o Frame Boost.
pub fn leitura_de(
    modo: Modo,
    gerador: &crate::modules::windows::geracao::Estado,
    jogo: Option<(Option<f64>, Option<f64>, Option<f64>)>,
) -> Leitura {
    use crate::modules::windows::geracao::Situacao;
    // Com o Frame Boost, o contador mostra o FPS do PRÓPRIO JOGO, como o `cl_drawfps` (decisão do dono, 08/10/2026: o
    // que vende é o jogo ficar liso, não um número maior). Sem o PresentMon, o FPS do jogo que o gerador capturou.
    if gerador.situacao == Situacao::Gerando {
        let (fps, quadro_ms, low) = jogo.unwrap_or((None, None, None));
        let fps = fps.or((gerador.fps_reais > 0.0).then_some(gerador.fps_reais));
        if fps.is_some() {
            // Só escala não gera: o que chega à tela é o próprio jogo, e "1×" não diria nada.
            let gera = gerador.automatico || gerador.multiplicador > 1;
            return Leitura {
                modo,
                rotulo: "FPS",
                fps,
                low_1pct: low,
                quadro_ms,
                boost: gera.then_some(gerador.multiplicador),
                nota: None,
                escala: gerador.escala.map(|n| n.nome()),
                na_tela: gera
                    .then(|| {
                        gerador
                            .fps_exibidos
                            .or((gerador.fps_apresentados > 0.0).then_some(gerador.fps_apresentados))
                    })
                    .flatten(),
            };
        }
    }
    match jogo {
        Some((fps, quadro_ms, low)) => Leitura {
            modo,
            rotulo: "FPS",
            fps,
            low_1pct: low,
            quadro_ms,
            boost: None,
            nota: None,
            escala: None,
            na_tela: None,
        },
        None => Leitura {
            modo,
            rotulo: "FPS",
            fps: None,
            low_1pct: None,
            quadro_ms: None,
            boost: None,
            nota: Some("Abra o Otimiza como administrador para medir".to_string()),
            escala: None,
            na_tela: None,
        },
    }
}

fn caminho() -> PathBuf {
    std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("overlay.json")
}

const VERSAO_DA_CONFIG: u32 = 2;

fn ler_em(c: &Path) -> Config {
    fs::read_to_string(c)
        .ok()
        .and_then(|b| serde_json::from_str::<Config>(&b).ok())
        .map(migrar)
        .unwrap_or_default()
}

/// Pura: o arquivo de antes da versão 2 tinha o contador com o Frame Boost ligado por padrão; desliga uma vez.
fn migrar(mut c: Config) -> Config {
    if c.versao < VERSAO_DA_CONFIG {
        c.com_frame_boost = false;
        c.versao = VERSAO_DA_CONFIG;
    }
    c
}

pub fn config() -> Config {
    ler_em(&caminho())
}

pub fn gravar(c: &Config) -> Result<(), String> {
    let p = caminho();
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let tmp = crate::modules::changelog::caminho_temporario(&p);
    fs::write(
        &tmp,
        serde_json::to_string_pretty(c).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

/// Pura: o overlay pode aparecer neste jogo? `Err("CONFIRMAR:...")` = a tela pergunta e chama de novo confirmando.
pub fn pode_mostrar(
    sobreposicao: crate::plataforma::jogos::Sobreposicao,
    barrada_agora: Option<&str>,
    confirmado: bool,
) -> Result<(), String> {
    use crate::plataforma::jogos::Sobreposicao;
    if let Some(m) = barrada_agora {
        return Err(m.to_string());
    }
    match sobreposicao {
        Sobreposicao::Barrada(m) => Err(m.to_string()),
        Sobreposicao::Desaconselhada(m) if !confirmado => Err(format!("CONFIRMAR:{}", m)),
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Estado {
    pub config: Config,
    pub ligado: bool,
    pub jogo: Option<String>,
}

#[cfg(target_os = "windows")]
mod janela {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use tauri::Manager;

    pub const ROTULO: &str = "overlay";

    struct Ativo {
        jogo: String,
        parar: Arc<AtomicBool>,
    }

    static ATIVO: Mutex<Option<Ativo>> = Mutex::new(None);

    pub fn estado() -> Estado {
        let g = ATIVO.lock().ok();
        let a = g.as_ref().and_then(|g| g.as_ref());
        Estado {
            config: config(),
            ligado: a.is_some(),
            jogo: a.map(|a| a.jogo.clone()),
        }
    }

    pub fn desligar(app: &tauri::AppHandle) {
        if let Ok(mut g) = ATIVO.lock() {
            if let Some(a) = g.take() {
                a.parar.store(true, Ordering::SeqCst);
            }
        }
        if let Some(w) = app.get_webview_window(ROTULO) {
            let _ = w.close();
        }
    }

    fn pid_na_frente() -> Option<u32> {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        unsafe {
            let h = GetForegroundWindow();
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, Some(&mut pid));
            (pid != 0).then_some(pid)
        }
    }

    /// Por cima de todas as janelas "sempre no topo", inclusive a do Frame Boost, sem pegar o foco.
    fn subir(h: windows::Win32::Foundation::HWND) {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        unsafe {
            let _ = SetWindowPos(
                h,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    fn qpc_em_segundos() -> impl Fn(i64) -> f64 {
        use windows::Win32::System::Performance::QueryPerformanceFrequency;
        let mut f = 0i64;
        unsafe {
            let _ = QueryPerformanceFrequency(&mut f);
        }
        let f = f.max(1) as f64;
        move |q| q as f64 / f
    }

    fn agora_qpc() -> i64 {
        let mut c = 0i64;
        unsafe {
            let _ = windows::Win32::System::Performance::QueryPerformanceCounter(&mut c);
        }
        c
    }

    pub fn ligar(app: &tauri::AppHandle, pid: u32, jogo: String) -> Result<(), String> {
        abrir(app, pid, jogo, false)
    }

    /// O Frame Boost do Otimiza ligou: mostra o FPS na tela no jogo (se a pessoa não desligou "com o Frame Boost"), e
    /// tira quando o Frame Boost desligar. Não mexe em overlay que já estava na tela.
    pub fn com_frame_boost(app: &tauri::AppHandle, pid: u32, jogo: String) -> Result<(), String> {
        if !config().com_frame_boost || estado().ligado {
            return Ok(());
        }
        abrir(app, pid, jogo, true)
    }

    fn abrir(
        app: &tauri::AppHandle,
        pid: u32,
        jogo: String,
        pelo_frame_boost: bool,
    ) -> Result<(), String> {
        desligar(app);
        let modo = modo_efetivo(
            config().modo,
            pelo_frame_boost,
            crate::plataforma::contador_fivem::estado().ligado,
        );
        if modo == Modo::Desligado {
            return Ok(());
        }
        let (lw, lh) = tamanho(modo);
        let w = tauri::WebviewWindowBuilder::new(
            app,
            ROTULO,
            tauri::WebviewUrl::App("overlay.html".into()),
        )
        .title("FPS do Otimiza")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .shadow(false)
        .visible(false)
        .inner_size(lw as f64, lh as f64)
        .build()
        .map_err(|e| format!("Não consegui abrir o FPS por cima do jogo: {}", e))?;
        w.set_ignore_cursor_events(true)
            .map_err(|e| e.to_string())?;
        let _ = w.set_size(tauri::PhysicalSize::new(lw as u32, lh as u32));
        // NA captura, de propósito: o print do jogo (o feedback do Discord pede um com o FPS) precisa mostrar o número.
        // Fica sempre acima da janela do Frame Boost (`subir`), então o gerador não o desenha duas vezes.
        let hwnd = w
            .hwnd()
            .ok()
            // Como número: o HWND não atravessa linhas de execução.
            .map(|h| h.0 as isize);
        let parar = Arc::new(AtomicBool::new(false));
        if let Ok(mut g) = ATIVO.lock() {
            *g = Some(Ativo {
                jogo,
                parar: parar.clone(),
            });
        }

        let app = app.clone();
        let aberto_em = std::time::Instant::now();
        std::thread::spawn(move || {
            use crate::modules::windows::geracao::{
                self, area_do_cliente, janela_do_processo, Situacao,
            };
            use crate::modules::windows::presentmon::Vivo;
            let em_segundos = qpc_em_segundos();
            let mut vivo: Option<Vivo> = None;
            let mut tentou_em: Option<std::time::Instant> = None;
            let mut instantes: std::collections::VecDeque<f64> = Default::default();
            let mut visivel = false;
            let mut onde = (i32::MIN, i32::MIN);
            let mut tamanho_atual = (lw, lh);
            let mut modo_atual = modo;
            let mut sem_janela = 0u32;
            let mut ultima_subida = std::time::Instant::now() - std::time::Duration::from_secs(5);
            let mut ultima_leitura = std::time::Instant::now() - std::time::Duration::from_secs(1);
            let mut suavizador = Suavizador::default();
            while !parar.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let Some(w) = app.get_webview_window(ROTULO) else {
                    break;
                };

                // O PresentMon ao vivo, com sessão própria. Sem administrador falha: tenta de novo a cada 30 s.
                if vivo.is_none()
                    && tentou_em.is_none_or(|t| t.elapsed() > std::time::Duration::from_secs(30))
                {
                    tentou_em = Some(std::time::Instant::now());
                    vivo = Vivo::iniciar_na_sessao(pid, "OtimizaOverlay").ok();
                }
                if let Some(v) = vivo.as_mut() {
                    match v.novos() {
                        Some(lote) => instantes.extend(lote.into_iter().map(&em_segundos)),
                        None => vivo = None,
                    }
                }
                let agora = em_segundos(agora_qpc());
                while instantes.front().is_some_and(|t| *t < agora - 10.0) {
                    instantes.pop_front();
                }

                let Some(area) = janela_do_processo(pid)
                    .and_then(area_do_cliente)
                    .filter(|a| a.largura > 0 && a.altura > 0)
                else {
                    if visivel {
                        let _ = w.hide();
                        visivel = false;
                    }
                    sem_janela += 1;
                    if sem_janela % 40 == 0
                        && crate::plataforma::sessao::atual().map(|j| j.pid) != Some(pid)
                    {
                        break;
                    }
                    continue;
                };
                sem_janela = 0;

                let c = config();
                let modo = modo_efetivo(
                    c.modo,
                    pelo_frame_boost,
                    crate::plataforma::contador_fivem::estado().ligado,
                );
                if modo == Modo::Desligado || (pelo_frame_boost && !c.com_frame_boost) {
                    break;
                }
                // Aberto pelo Frame Boost: sai junto com ele (3 s de folga para o gerador achar a janela).
                if pelo_frame_boost
                    && aberto_em.elapsed() > std::time::Duration::from_secs(3)
                    && !matches!(
                        geracao::estado().situacao,
                        Situacao::Gerando | Situacao::ProcurandoJanela | Situacao::SemQuadros
                    )
                {
                    break;
                }
                if modo != modo_atual {
                    modo_atual = modo;
                    tamanho_atual = tamanho(modo);
                    let _ = w.set_size(tauri::PhysicalSize::new(
                        tamanho_atual.0 as u32,
                        tamanho_atual.1 as u32,
                    ));
                    onde = (i32::MIN, i32::MIN);
                }
                let pos = posicao(
                    (area.x, area.y, area.largura, area.altura),
                    c.canto,
                    tamanho_atual,
                );
                if pos != onde {
                    let _ = w.set_position(tauri::PhysicalPosition::new(pos.0, pos.1));
                    onde = pos;
                }
                // Em tela cheia exclusiva, aparecer por cima derrubaria o jogo do modo: fica escondido.
                let mostrar = pid_na_frente() == Some(pid) && !geracao::tela_cheia_exclusiva();
                if mostrar != visivel {
                    let _ = if mostrar { w.show() } else { w.hide() };
                    visivel = mostrar;
                }
                if visivel && ultima_subida.elapsed() >= std::time::Duration::from_secs(1) {
                    ultima_subida = std::time::Instant::now();
                    if let Some(h) = hwnd {
                        subir(windows::Win32::Foundation::HWND(h as _));
                    }
                }

                if visivel && ultima_leitura.elapsed() >= std::time::Duration::from_millis(500) {
                    ultima_leitura = std::time::Instant::now();
                    let lista: Vec<f64> = instantes.iter().copied().collect();
                    let jogo = vivo.as_ref().map(|_| estatisticas(&lista, agora));
                    let mut l = leitura_de(modo_atual, &geracao::estado(), jogo);
                    l.fps = suavizador.passo(l.fps, agora);
                    if l.fps.is_some() {
                        l.nota = None;
                    }
                    if let Ok(json) = serde_json::to_string(&l) {
                        let _ = w.eval(&format!("window.fps && window.fps({})", json));
                    }
                }
            }
            if let Some(w) = app.get_webview_window(ROTULO) {
                let _ = w.close();
            }
            if let Ok(mut g) = ATIVO.lock() {
                if g.as_ref().is_some_and(|a| Arc::ptr_eq(&a.parar, &parar)) {
                    *g = None;
                }
            }
        });
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use janela::{com_frame_boost, desligar, estado, ligar};

/// Quando um jogo abre: liga sozinho se o modo não for Desligado e o jogo deixar (anticheat; ressalva só se a pessoa
/// já confirmou para este jogo).
#[cfg(target_os = "windows")]
pub fn ao_abrir(app: &tauri::AppHandle, jogo: &crate::plataforma::sessao::JogoAberto) {
    let c = config();
    if c.modo == Modo::Desligado || !crate::plataforma::flags::Flags::load().overlay {
        return;
    }
    // O modo se grava sem licença (`overlay_gravar` é preferência), e o arquivo se edita à mão: ligar sozinho é o
    // `overlay_ligar` sem ninguém clicar, então pede o mesmo que ele.
    if crate::modules::licenca::exigir().is_err() {
        return;
    }
    let barrada = {
        use crate::modules::windows::anticheat;
        anticheat::permite(
            anticheat::Acao::SobreporJanela,
            &anticheat::detectar_agora(),
        )
        .motivo()
        .map(str::to_string)
    };
    let confirmado = c
        .confirmados
        .iter()
        .any(|x| x == jogo.perfil.id || x.eq_ignore_ascii_case(&jogo.executavel));
    match pode_mostrar(jogo.perfil.sobreposicao, barrada.as_deref(), confirmado) {
        Ok(()) => {
            if let Err(e) = ligar(app, jogo.pid, jogo.nome.clone()) {
                crate::utils::Logger::warn(&format!("FPS por cima do jogo: {}", e));
            }
        }
        Err(m) => crate::utils::Logger::info(&format!(
            "FPS por cima do jogo não ligou em {}: {}",
            jogo.nome, m
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::windows::geracao::{Estado as EstadoDoGerador, Situacao};
    use crate::plataforma::jogos::Sobreposicao;

    #[test]
    fn fps_do_ultimo_segundo_e_tempo_de_quadro() {
        let quadros: Vec<f64> = (0..600).map(|i| i as f64 / 60.0).collect(); // 60 FPS por 10 s
        let (fps, ms, low) = estatisticas(&quadros, 9.99);
        assert_eq!(fps, Some(60.0));
        assert!((ms.unwrap() - 16.667).abs() < 0.1);
        assert_eq!(low, None, "600 quadros: sem 1% low (precisa de mil)");
    }

    #[test]
    fn um_porcento_low_pega_os_trancos() {
        // 144 FPS por 10 s com 15 quadros de 50 ms no meio.
        let mut t = 0.0;
        let mut quadros = Vec::new();
        for i in 0..1440 {
            t += if i % 96 == 0 { 0.050 } else { 1.0 / 144.0 };
            quadros.push(t);
        }
        let (_, _, low) = estatisticas(&quadros, t);
        let low = low.expect("mais de mil quadros");
        assert!(low < 40.0, "o 1% low reflete os trancos: {}", low);
    }

    #[test]
    fn o_contador_nao_pula_nem_cai_a_zero_num_instante() {
        let mut s = Suavizador::default();
        let mut t = 0.0;
        let mut mostrado = Vec::new();
        // 140 com o ruído da medida de meio segundo, um buraco de 1 s (pausa, lote do PresentMon) e 140 de novo.
        for m in [
            140.0, 128.0, 151.0, 136.0, 144.0, 0.0, 0.0, 132.0, 147.0, 139.0,
        ] {
            mostrado.push(s.passo(Some(m), t).unwrap());
            t += 0.5;
        }
        assert!(
            mostrado.iter().all(|v| (125.0..=152.0).contains(v)),
            "{:?}",
            mostrado
        );
        let saltos = mostrado
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f64::max);
        assert!(
            saltos <= 8.0,
            "pulou {} de uma leitura para a outra: {:?}",
            saltos,
            mostrado
        );
    }

    #[test]
    fn o_contador_acompanha_quando_a_mudanca_e_de_verdade() {
        let mut s = Suavizador::default();
        let mut t = 0.0;
        for _ in 0..6 {
            s.passo(Some(60.0), t);
            t += 0.5;
        }
        // Ligou o Frame Boost: 60 → 120. Em 2 s o contador já mostra perto de 120.
        let mut v = 0.0;
        for _ in 0..4 {
            v = s.passo(Some(120.0), t).unwrap();
            t += 0.5;
        }
        assert!(v >= 112.0, "{}", v);
    }

    #[test]
    fn sem_medida_por_muito_tempo_o_contador_admite() {
        let mut s = Suavizador::default();
        s.passo(Some(90.0), 0.0);
        assert_eq!(s.passo(None, 2.0), Some(90.0), "segura um instante");
        assert_eq!(s.passo(None, 4.0), None, "não segura para sempre");
    }

    #[test]
    fn sem_quadros_nao_tem_numero() {
        assert_eq!(estatisticas(&[], 5.0), (None, None, None));
        assert_eq!(estatisticas(&[4.5], 5.0).0, None);
    }

    #[test]
    fn so_escala_mostra_o_nivel_e_nao_um_multiplicador() {
        let g = EstadoDoGerador {
            situacao: Situacao::Gerando,
            fps_reais: 88.0,
            fps_apresentados: 88.0,
            multiplicador: 1,
            escala: Some(crate::modules::windows::geracao::escala::Nivel::Qualidade),
            ..Default::default()
        };
        let l = leitura_de(Modo::Detalhado, &g, Some((Some(87.0), None, None)));
        assert_eq!(l.fps, Some(87.0));
        assert_eq!(l.boost, None, "sem geração não há multiplicador");
        assert_eq!(l.escala, Some("Qualidade"));
        assert_eq!(l.na_tela, None, "na tela é o próprio jogo");
    }

    #[test]
    fn com_frame_boost_mostra_o_fps_do_jogo() {
        let g = EstadoDoGerador {
            situacao: Situacao::Gerando,
            fps_reais: 52.0,
            fps_apresentados: 204.0,
            multiplicador: 4,
            hz: 180,
            ..Default::default()
        };
        let l = leitura_de(
            Modo::Detalhado,
            &g,
            Some((Some(51.0), Some(19.2), Some(41.0))),
        );
        assert_eq!(
            l.fps,
            Some(51.0),
            "o do jogo, pelo PresentMon, e não os quadros na tela"
        );
        assert_eq!(l.rotulo, "FPS");
        assert_eq!(l.low_1pct, Some(41.0));
        assert_eq!(l.boost, Some(4));
        // Sem PresentMon: o FPS do jogo que o gerador capturou.
        let l = leitura_de(Modo::Minimo, &g, None);
        assert_eq!(l.fps, Some(52.0));
    }

    #[test]
    fn sem_presentmon_diz_o_motivo_em_vez_de_inventar() {
        let l = leitura_de(Modo::Minimo, &EstadoDoGerador::default(), None);
        assert_eq!(l.fps, None);
        assert!(l.nota.unwrap().contains("administrador"));
    }

    #[test]
    fn cantos_ficam_dentro_da_area_do_jogo() {
        let area = (100, 50, 1280, 720);
        let t = tamanho(Modo::Detalhado);
        assert_eq!(posicao(area, Canto::SuperiorEsquerdo, t), (112, 62));
        assert!(
            tamanho(Modo::Minimo).0 <= 120,
            "o mínimo cobre o contador do jogo sem tampar o resto"
        );
        assert_eq!(
            posicao(area, Canto::InferiorDireito, t),
            (100 + 1280 - 220 - 12, 50 + 720 - 104 - 12)
        );
    }

    #[test]
    fn anticheat_barra_e_ressalva_pede_confirmacao() {
        assert!(pode_mostrar(Sobreposicao::Segura, Some("Vanguard aberto"), true).is_err());
        assert!(pode_mostrar(Sobreposicao::Barrada("FACEIT"), None, true).is_err());
        assert!(
            pode_mostrar(Sobreposicao::Desaconselhada("RP"), None, false)
                .unwrap_err()
                .starts_with("CONFIRMAR:")
        );
        assert!(pode_mostrar(Sobreposicao::Desaconselhada("RP"), None, true).is_ok());
        assert!(pode_mostrar(Sobreposicao::Segura, None, false).is_ok());
    }

    #[test]
    fn config_padrao_nao_abre_contador_com_o_frame_boost() {
        assert_eq!(Config::default().modo, Modo::Desligado);
        assert!(!Config::default().com_frame_boost);
        // O arquivo antigo (sem versão, com o padrão ligado) passa a desligado uma vez.
        let antigo: Config =
            serde_json::from_str(r#"{"modo":"Detalhado","com_frame_boost":true}"#).unwrap();
        assert!(!migrar(antigo).com_frame_boost);
        // Quem religar depois (versão 2) fica com o que escolheu.
        let novo: Config =
            serde_json::from_str(r#"{"modo":"Minimo","com_frame_boost":true,"versao":2}"#).unwrap();
        assert!(migrar(novo).com_frame_boost);
    }

    #[test]
    fn pelo_frame_boost_o_desligado_vira_minimo() {
        assert_eq!(modo_efetivo(Modo::Desligado, true, false), Modo::Minimo);
        assert_eq!(modo_efetivo(Modo::Detalhado, true, false), Modo::Detalhado);
        assert_eq!(modo_efetivo(Modo::Desligado, false, false), Modo::Desligado);
        // No lugar do cl_drawfps: sempre o estilo do FiveM, mesmo com o Detalhado escolhido.
        assert_eq!(modo_efetivo(Modo::Detalhado, true, true), Modo::Minimo);
        assert_eq!(modo_efetivo(Modo::Detalhado, false, true), Modo::Minimo);
        assert_eq!(modo_efetivo(Modo::Desligado, false, true), Modo::Desligado);
    }
}
