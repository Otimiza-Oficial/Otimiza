// Frame Boost: um lugar só para escolher o gerador de quadros, ligar, desligar e dizer o número da tela.
//
// A regra de exibição (decisão do dono): com o Frame Boost ligado, a tela mostra SÓ o FPS que chegou à tela, MEDIDO
// (no gerador do Otimiza, o contador de quadros apresentados). O nativo continua sendo medido e gravado, mas não
// aparece no Frame Boost. Antes de ligar, só faixa e sempre ESTIMADO. Nunca nativo × multiplicador como medição.

use serde::Serialize;

use super::compat::{self, Recurso};
use super::metricas::{self, Faixa, Valor};
use super::sessao::JogoAberto;
use super::RecursoAvaliado;
use crate::modules::windows::geracao::{self, Situacao};

#[derive(Debug, Clone, Serialize)]
pub struct Estimativa {
    pub multiplicador: u8,
    pub faixa: Faixa,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ativo {
    pub recurso: Recurso,
    pub multiplicador: u8,
    /// No automático, `multiplicador` é o do último quadro real.
    pub automatico: bool,
    /// Passando do limite do monitor: o número pode passar dos Hz (o monitor mostra até os Hz dele).
    pub sem_limite: bool,
    /// O que chegou à tela. MEDIDO enquanto gera; desconhecido enquanto procura a janela.
    pub fps_na_tela: Valor,
    pub situacao: Situacao,
    /// Hz do monitor onde o jogo está: o teto do número.
    pub hz: u32,
    pub hdr: bool,
    pub integrada: bool,
    pub refeitas: u32,
    /// Do quadro composto pelo Windows até o gerador mostrar. ESTIMADO: sem marcadores do jogo não dá para medir o
    /// atraso do clique até a tela.
    pub atraso_ms: Valor,
    /// AFMF ou Smooth Motion: ligado no driver, número pelo PresentMon.
    pub pelo_driver: bool,
    /// Só para os de driver: a medição viu o driver gerar quadros?
    pub geracao_vista: Option<bool>,
    pub nota: Option<String>,
    /// Quanto da placa o gerador do Otimiza ocupou nos últimos 10 s, em % do tempo (MEDIDO pelos carimbos da placa).
    #[serde(default)]
    pub placa_pct: Option<f64>,
    /// No modo leve (placa sem folga, ou a guarda aliviou).
    #[serde(default)]
    pub leve: bool,
    /// FPS do jogo agora, com o gerador ligado (o que ele capturou de quadros reais).
    #[serde(default)]
    pub fps_do_jogo: Option<f64>,
    /// Quanto do FPS o jogo mantém com o gerador ligado, em %, MEDIDO pela guarda (com e sem ele, colados no tempo).
    #[serde(default)]
    pub manteve_pct: Option<f64>,
    #[serde(default)]
    pub comparacoes: u32,
    /// Com escala: o nível, o tamanho em que o jogo renderiza e o da imagem ampliada na tela.
    #[serde(default)]
    pub escala: Option<geracao::escala::Nivel>,
    #[serde(default)]
    pub jogo_px: Option<[i32; 2]>,
    #[serde(default)]
    pub tela_px: Option<[i32; 2]>,
}

/// O "antes × depois" da última vez que o gerador do Otimiza ficou ligado (só com 30 s ou mais gerando).
#[derive(Debug, Clone, Serialize)]
pub struct UltimaSessao {
    pub jogo: String,
    pub minutos: f64,
    pub fps_do_jogo: f64,
    pub fps_na_tela: Valor,
    pub manteve_pct: Option<f64>,
    pub comparacoes: u32,
    pub desligou_por_perda: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Painel {
    pub jogo: Option<String>,
    /// O que o automático escolhe: o do jogo, depois o do driver, depois o do Otimiza.
    pub recomendado: Option<RecursoAvaliado>,
    pub opcoes: Vec<RecursoAvaliado>,
    /// Só os que cabem no monitor (2× sempre).
    pub multiplicadores: Vec<u8>,
    pub hz: Option<u32>,
    pub estimativas: Vec<Estimativa>,
    pub ativo: Option<Ativo>,
    /// Por que o gerador desligou sozinho, para a tela dizer.
    pub aviso: Option<String>,
    /// O teto do FPS base que o Otimiza gravou para este jogo (`base_do_jogo`).
    pub base: Option<super::base_do_jogo::BaseTravada>,
    /// O que aconteceu com o FPS base ao ligar (travou, já estava, falta medir).
    pub nota_da_base: Option<String>,
    /// A pessoa destravou o FPS base deste jogo: o Frame Boost não trava de novo sozinho.
    #[serde(default)]
    pub base_solta: bool,
    /// Gerador desligado: como foi a última vez.
    pub ultima_sessao: Option<UltimaSessao>,
    /// O monitor onde o jogo está e, para cada nível de escala, a resolução em que o jogo deve rodar.
    #[serde(default)]
    pub monitor: Option<[i32; 2]>,
    #[serde(default)]
    pub escalas: Vec<geracao::escala::Opcao>,
}

/// A frase da última vez que o Frame Boost ligou, para o painel repetir enquanto ele estiver ligado.
#[cfg(target_os = "windows")]
static NOTA_DA_BASE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(target_os = "windows")]
pub fn anotar_base(nota: Option<String>) {
    if let Ok(mut n) = NOTA_DA_BASE.lock() {
        *n = nota;
    }
}

/// Pura: o número da tela, só gerando de verdade. MEDIDO é o que o PresentMon viu chegar à tela (`fps_exibidos`); sem
/// ele, o que o gerador apresentou, como ESTIMADO (apresentado não é exibido: quadro descartado, dois na mesma
/// atualização).
pub fn fps_na_tela(e: &geracao::Estado) -> Valor {
    if e.situacao != Situacao::Gerando {
        return Valor::desconhecido();
    }
    match e.fps_exibidos {
        Some(f) if f > 0.0 => Valor::medido(f),
        _ if e.fps_apresentados > 0.0 => Valor::estimado(e.fps_apresentados),
        _ => Valor::desconhecido(),
    }
}

/// Pura: a razão da guarda em %, com uma casa.
fn manteve_pct(e: &geracao::Estado) -> Option<f64> {
    e.razao_fps
        .filter(|_| e.comparacoes > 0)
        .map(|r| (r * 1000.0).round() / 10.0)
}

/// Pura: com o gerador parado, o resumo da última vez que ele ligou.
pub fn ultima_sessao_de(e: &geracao::Estado) -> Option<UltimaSessao> {
    if ativo_de(e).is_some() || e.processo.is_empty() {
        return None;
    }
    let r = e.sessao.resumo()?;
    Some(UltimaSessao {
        jogo: e.processo.clone(),
        minutos: (r.segundos / 6.0).round() / 10.0,
        fps_do_jogo: r.fps_do_jogo.round(),
        fps_na_tela: if r.na_tela_medido {
            Valor::medido(r.fps_na_tela.round())
        } else {
            Valor::estimado(r.fps_na_tela.round())
        },
        manteve_pct: manteve_pct(e),
        comparacoes: e.comparacoes,
        desligou_por_perda: e.situacao == Situacao::DesligadoPorPerdaDeFps,
    })
}

/// Pura.
pub fn ativo_de(e: &geracao::Estado) -> Option<Ativo> {
    matches!(
        e.situacao,
        Situacao::Gerando | Situacao::ProcurandoJanela | Situacao::SemQuadros
    )
    .then(|| Ativo {
        recurso: Recurso::FrameBoostOtimiza,
        multiplicador: e.multiplicador,
        automatico: e.automatico,
        sem_limite: e.sem_limite,
        fps_na_tela: fps_na_tela(e),
        situacao: e.situacao,
        hz: e.hz,
        hdr: e.hdr,
        integrada: e.integrada,
        refeitas: e.refeitas,
        atraso_ms: e
            .atraso_ms
            .map(Valor::estimado)
            .unwrap_or_else(Valor::desconhecido),
        pelo_driver: false,
        geracao_vista: None,
        nota: None,
        placa_pct: e.placa_ms_por_s.map(|ms| (ms / 10.0 * 10.0).round() / 10.0),
        leve: e.leve,
        fps_do_jogo: (e.situacao == Situacao::Gerando && e.fps_reais > 0.0).then_some(e.fps_reais.round()),
        manteve_pct: manteve_pct(e),
        comparacoes: e.comparacoes,
        escala: e.escala,
        jogo_px: e.escala.and(e.area).map(|a| [a.largura, a.altura]),
        tela_px: e.ampliada.map(|a| [a.largura, a.altura]),
    })
}

/// Pura: o motivo de o gerador ter parado sozinho.
pub fn aviso_de(e: &geracao::Estado) -> Option<String> {
    match e.situacao {
        Situacao::DesligadoPorPerdaDeFps => Some(
            "O Frame Boost desligou sozinho: o jogo perdeu FPS de verdade com ele ligado. Nesta placa, neste jogo, \
             jogar sem ele rende mais."
                .to_string(),
        ),
        Situacao::Erro => e.erro.clone(),
        _ => None,
    }
}

/// Uma medição do PresentMon no jogo, para os geradores de driver (AFMF, Smooth Motion): eles não têm contador
/// próprio que o Otimiza leia. Vale para o processo medido.
#[derive(Debug, Clone, PartialEq)]
pub struct Medicao {
    pub pid: u32,
    pub fps_exibido: f64,
    pub fps_do_jogo: f64,
    pub quadros_gerados: usize,
    pub geradores: Vec<String>,
}

/// Pura: o driver gerou quadros nesta medição? O AFMF se identifica no PresentMon; o Smooth Motion não, e aí vale a
/// tela recebendo bem mais quadros que o jogo desenha.
pub fn geracao_vista(m: &Medicao) -> bool {
    m.quadros_gerados > 0 || !m.geradores.is_empty() || m.fps_exibido > m.fps_do_jogo * 1.3
}

/// Pura: o Frame Boost ligado no driver. O número só existe depois de medir.
pub fn ativo_do_driver(recurso: Recurso, medicao: Option<&Medicao>) -> Ativo {
    let vista = medicao.map(geracao_vista);
    Ativo {
        recurso,
        multiplicador: 2,
        automatico: false,
        sem_limite: false,
        fps_na_tela: medicao
            .map(|m| Valor::medido(m.fps_exibido))
            .unwrap_or_else(Valor::desconhecido),
        situacao: Situacao::Gerando,
        hz: 0,
        hdr: false,
        integrada: false,
        refeitas: 0,
        atraso_ms: Valor::desconhecido(),
        pelo_driver: true,
        placa_pct: None,
        leve: false,
        fps_do_jogo: None,
        manteve_pct: None,
        comparacoes: 0,
        escala: None,
        jogo_px: None,
        tela_px: None,
        geracao_vista: vista,
        nota: Some(match vista {
            None => "Ligado no driver. Toque em \u{201c}Medir agora\u{201d} com o jogo em movimento para ver o FPS na tela.".to_string(),
            Some(true) => "Medido pelo PresentMon com o jogo aberto.".to_string(),
            Some(false) => "Na medição o driver não gerou quadros: o jogo precisa estar em tela cheia ou sem bordas, e às \
                            vezes precisa ser fechado e aberto de novo."
                .to_string(),
        }),
    }
}

/// Pura: o painel inteiro a partir do jogo aberto, do último FPS nativo medido dele, do Hz do monitor dele, do
/// gerador do Otimiza e do Frame Boost ligado no driver (se houver).
pub fn painel_de(
    jogo: Option<&JogoAberto>,
    nativo: Option<f64>,
    hz: Option<u32>,
    gerador: &geracao::Estado,
    pelo_driver: Option<Ativo>,
) -> Painel {
    let opcoes: Vec<RecursoAvaliado> = jogo
        .map(|j| {
            j.recursos
                .iter()
                .filter(|r| {
                    matches!(
                        r.recurso,
                        Recurso::FrameBoostDoJogo
                            | Recurso::FrameBoostSmoothMotion
                            | Recurso::FrameBoostAfmf
                            | Recurso::FrameBoostOtimiza
                    )
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let recomendado = jogo.and_then(|j| j.frame_boost_auto.clone());
    let multiplicadores = recomendado
        .as_ref()
        .map(|r| compat::multiplicadores(r.recurso, nativo, hz))
        .unwrap_or_default();
    let estimativas = nativo
        .map(|n| {
            multiplicadores
                .iter()
                .filter_map(|m| {
                    metricas::estimar_frame_boost(n, *m, hz.map(|h| h as f64)).map(|faixa| {
                        Estimativa {
                            multiplicador: *m,
                            faixa,
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let ativo = ativo_de(gerador).or_else(|| {
        pelo_driver.map(|mut a| {
            a.hz = hz.unwrap_or(0);
            a
        })
    });
    Painel {
        jogo: jogo.map(|j| j.nome.clone()),
        recomendado,
        opcoes,
        multiplicadores,
        hz,
        estimativas,
        ativo,
        aviso: aviso_de(gerador),
        base: None,
        nota_da_base: None,
        base_solta: false,
        ultima_sessao: ultima_sessao_de(gerador),
        monitor: None,
        escalas: Vec::new(),
    }
}

/// O "multiplicador" do automático, no pedido da tela.
pub const AUTOMATICO: u8 = 0;

/// Pura: o multiplicador pedido, se couber; senão o maior que cabe.
pub fn multiplicador_valido(pedido: u8, cabem: &[u8]) -> Option<u8> {
    if cabem.contains(&pedido) {
        Some(pedido)
    } else {
        cabem
            .iter()
            .copied()
            .filter(|m| *m <= pedido)
            .max()
            .or_else(|| cabem.first().copied())
    }
}

/// O último FPS nativo medido deste jogo: base da estimativa, nunca exibido no Frame Boost ligado.
fn nativo_do_jogo(j: &JogoAberto) -> Option<f64> {
    let (id, _) = super::historico::id_do_jogo(&j.executavel);
    super::historico::biblioteca()
        .ok()?
        .into_iter()
        .find(|r| r.id == id)?
        .ultima
        .map(|p| p.fps_nativo)
}

/// O Hz do monitor onde a janela do jogo está (o principal pode ser outro).
#[cfg(target_os = "windows")]
pub fn hz_do_jogo(pid: u32) -> Option<u32> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
    };
    let monitores = crate::modules::windows::display::monitores_sem_nomes();
    let lista: Vec<(&str, bool, u32)> = monitores
        .iter()
        .map(|m| (m.dispositivo.as_str(), m.principal, m.hz_atual))
        .collect();
    let dispositivo = geracao::janela_do_processo(pid).and_then(|hwnd| unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        GetMonitorInfoW(
            monitor,
            &mut info as *mut MONITORINFOEXW as *mut MONITORINFO,
        )
        .as_bool()
        .then(|| {
            let fim = info
                .szDevice
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(info.szDevice.len());
            String::from_utf16_lossy(&info.szDevice[..fim])
        })
    });
    let hz = geracao::hz_escolhido(&lista, dispositivo.as_deref().unwrap_or(""));
    (hz > 0).then_some(hz)
}

/// O AFMF é global no driver; o Smooth Motion é do perfil do jogo.
pub const ID_DO_AFMF: &str = "amd-afmf";

#[cfg(target_os = "windows")]
static MEDICAO: std::sync::Mutex<Option<Medicao>> = std::sync::Mutex::new(None);

/// Qual gerador de driver o Otimiza ligou para este jogo (pelo histórico de mudanças).
#[cfg(target_os = "windows")]
fn ligado_no_driver(jogo: &JogoAberto) -> Option<Recurso> {
    let log = crate::modules::changelog::ChangeLog::load();
    let sm = crate::modules::windows::nvdriver::id_do_smooth_motion(&jogo.executavel);
    if log.is_applied(&sm) {
        Some(Recurso::FrameBoostSmoothMotion)
    } else if log.is_applied(ID_DO_AFMF) {
        Some(Recurso::FrameBoostAfmf)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
pub fn painel() -> Painel {
    let jogo = super::sessao::atual();
    let gerador = geracao::estado();
    let hz = if gerador.hz > 0 && ativo_de(&gerador).is_some() {
        Some(gerador.hz)
    } else {
        jogo.as_ref().and_then(|j| hz_do_jogo(j.pid))
    };
    let nativo = jogo.as_ref().and_then(nativo_do_jogo);
    let pelo_driver = jogo.as_ref().and_then(|j| {
        let recurso = ligado_no_driver(j)?;
        let medicao = MEDICAO
            .lock()
            .ok()
            .and_then(|m| m.clone())
            .filter(|m| m.pid == j.pid);
        Some(ativo_do_driver(recurso, medicao.as_ref()))
    });
    let mut p = painel_de(jogo.as_ref(), nativo, hz, &gerador, pelo_driver);
    if p.aviso.is_none() && jogo.is_some() {
        p.aviso = aviso_de_folga(memoria_disponivel_gb(), uso_da_placa_pct());
    }
    if let (Some(j), Some(hz)) = (jogo.as_ref(), hz) {
        p.base = super::base_do_jogo::travada(j.pid, &j.executavel, hz);
    }
    p.base_solta = jogo
        .as_ref()
        .is_some_and(|j| super::base_do_jogo::solta(&j.executavel));
    if p.ativo.is_some() {
        p.nota_da_base = NOTA_DA_BASE.lock().ok().and_then(|n| n.clone());
    }
    if let Some((l, a)) = jogo.as_ref().and_then(|j| monitor_do_jogo(j.pid)) {
        p.monitor = Some([l, a]);
        p.escalas = geracao::escala::opcoes(l, a);
    }
    p
}

/// O tamanho, em pixels, do monitor onde a janela do jogo está.
#[cfg(target_os = "windows")]
fn monitor_do_jogo(pid: u32) -> Option<(i32, i32)> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    let hwnd = geracao::janela_do_processo(pid)?;
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info).as_bool().then(|| {
            let r = info.rcMonitor;
            (r.right - r.left, r.bottom - r.top)
        })
    }
}

/// Trava o FPS base do jogo aberto para o Frame Boost do Otimiza (só ele: o do driver não tem o ritmo do Otimiza).
#[cfg(target_os = "windows")]
pub fn travar_base(log: &mut crate::modules::changelog::ChangeLog) {
    let e = geracao::estado();
    // Só escala: travar o FPS do jogo tiraria justamente o FPS real que a escala dá.
    let so_escala = e.escala.is_some() && !e.automatico && e.multiplicador <= 1;
    let gerando = ativo_de(&e).is_some() && !so_escala;
    let Some(j) = super::sessao::atual().filter(|_| gerando) else {
        return;
    };
    let nota = match hz_do_jogo(j.pid) {
        Some(hz) => super::base_do_jogo::travar(&j.executavel, hz, log)
            .unwrap_or_else(|e| format!("Não consegui travar o FPS base: {}", e)),
        None => "Não consegui ler os Hz do monitor do jogo para travar o FPS base.".to_string(),
    };
    // A 3.3.0 ligava a otimização de janela do Windows para o jogo aqui; medida sem efeito (o jogo continuou na cópia,
    // `gpupref`), ela sai, e o que foi gravado volta como estava.
    if let Some(caminho) = crate::modules::windows::frames::caminho_do_processo(&j.executavel) {
        match crate::modules::windows::WindowsOptimizer::new().desfazer_janela_do_jogo(&caminho, log) {
            Ok(true) => crate::utils::Logger::info(&format!(
                "frame boost: otimização de janela de {} desfeita (medida sem efeito)",
                j.executavel
            )),
            Ok(false) => {}
            Err(e) => crate::utils::Logger::warn(&format!("frame boost: desfazer a otimização de janela: {}", e)),
        }
    }
    anotar_base(Some(nota));
}

/// Pura: com a placa a partir de 85% ocupada pelo jogo, o gerador começa no modo leve.
pub fn comecar_leve(placa_pct: Option<u32>) -> bool {
    placa_pct.is_some_and(|p| p >= 85)
}

/// Pura: o PC tem folga para o Frame Boost agora? O gerador usa a mesma placa e memória que o jogo: com a placa quase
/// cheia ou a memória no limite, ele atrasa os próprios quadros e o resultado é tranco (o aviso do Lossless Scaling, e o
/// que o dono viu no FiveM com 0,24 GB livres). Nunca bloqueia: só diz antes de a pessoa ligar.
pub fn aviso_de_folga(disponivel_gb: Option<f64>, placa_pct: Option<u32>) -> Option<String> {
    if disponivel_gb.is_some_and(|d| d < 0.7) {
        return Some(format!(
            "A memória do PC está no limite agora ({:.1} GB livres): o jogo já trava buscando memória no disco, e o Frame \
             Boost ocupa mais. Feche o que estiver aberto além do jogo (navegador, Discord) antes de ligar.",
            disponivel_gb.unwrap_or(0.0)
        ));
    }
    if placa_pct.is_some_and(|p| p >= 90) {
        return Some(format!(
            "A placa de vídeo está {}% ocupada pelo jogo: sem folga, o Frame Boost atrasa os quadros e dá tranco. Baixe \
             a qualidade ou a resolução do jogo antes de ligar.",
            placa_pct.unwrap_or(0)
        ));
    }
    None
}

fn memoria_disponivel_gb() -> Option<f64> {
    let mut s = sysinfo::System::new();
    s.refresh_memory();
    let d = s.available_memory() as f64;
    (d > 0.0).then(|| d / 1_073_741_824.0)
}

fn uso_da_placa_pct() -> Option<u32> {
    crate::modules::windows::nvml::amostrar_com_motivos(false).and_then(|a| a.uso_pct)
}

/// Liga o Frame Boost que o automático escolheu. O do jogo não é do Otimiza ligar (vai no menu de vídeo dele); o
/// Smooth Motion só com o interruptor experimental; o AFMF e o gerador do Otimiza, sempre.
#[cfg(target_os = "windows")]
pub fn ligar(
    app: &tauri::AppHandle,
    multiplicador: u8,
    sem_limite: bool,
    escala: Option<geracao::escala::Nivel>,
) -> Result<Painel, String> {
    let jogo =
        super::sessao::atual().ok_or("Abra o jogo primeiro: o Frame Boost liga no jogo aberto.")?;
    let recomendado = jogo
        .frame_boost_auto
        .clone()
        .ok_or("Nenhum gerador de quadros funciona neste jogo com esta placa.")?;
    if let Some(m) = compat_motivo(&recomendado) {
        return Err(m);
    }
    let otimizador = crate::modules::windows::WindowsOptimizer::new();
    match recomendado.recurso {
        Recurso::FrameBoostDoJogo => Err(format!(
            "{}: ligue a geração de quadros no menu de vídeo do jogo. É a melhor, porque usa o movimento do próprio jogo.",
            recomendado.nome
        )),
        Recurso::FrameBoostSmoothMotion => {
            if !super::flags::Flags::load().smooth_motion_drs {
                return Err(format!(
                    "{}: ligue no app da NVIDIA, em Gráficos, no perfil do jogo. O Otimiza mede o resultado.",
                    recomendado.nome
                ));
            }
            let mut log = crate::modules::changelog::ChangeLog::load();
            otimizador.ligar_smooth_motion(&jogo.executavel, &mut log)?;
            Ok(painel())
        }
        Recurso::FrameBoostAfmf => {
            let mut log = crate::modules::changelog::ChangeLog::load();
            if !log.is_applied(ID_DO_AFMF) {
                otimizador.aplicar_ajuste_amd(ID_DO_AFMF, &mut log)?;
            }
            Ok(painel())
        }
        Recurso::FrameBoostOtimiza => {
            let hz = hz_do_jogo(jogo.pid);
            let nativo = nativo_do_jogo(&jogo);
            // Passando do limite do monitor, os Hz não cortam o multiplicador (até 6×).
            let cabem = if sem_limite {
                (2..=geracao::MAXIMO_DO_AUTOMATICO).collect()
            } else {
                compat::multiplicadores(Recurso::FrameBoostOtimiza, nativo, hz)
            };
            // 0 = automático: enche o monitor, quantos quadros couberem.
            // 1 com escala: só escala, sem gerar quadros.
            let so_escala = escala.is_some() && multiplicador == 1;
            let automatico = multiplicador == AUTOMATICO;
            let m = if so_escala {
                1
            } else if automatico {
                geracao::MAXIMO_DO_AUTOMATICO
            } else {
                multiplicador_valido(multiplicador, &cabem).unwrap_or(2)
            };

            // A janela do gerador fica por cima do jogo: com anticheat que caça sobreposição aberto, nem liga.
            use crate::modules::windows::anticheat;
            if let Some(recusa) = anticheat::permite(
                anticheat::Acao::SobreporJanela,
                &anticheat::detectar_agora(),
            )
            .motivo()
            {
                return Err(recusa.to_string());
            }

            geracao::ligar(geracao::Configuracao {
                processo: jogo.executavel.clone(),
                multiplicador: m,
                visivel_em_captura: false,
                automatico,
                sem_limite,
                // Placa já quase cheia com o jogo: começa leve, em vez de esperar a guarda ver o jogo perder FPS.
                leve: comecar_leve(uso_da_placa_pct()),
                escala,
            })?;
            // O contador do jogo nunca vê os quadros gerados: o FPS na tela aparece no canto do jogo, pelo Otimiza.
            if let Err(e) = super::overlay::com_frame_boost(app, jogo.pid, jogo.nome.clone()) {
                crate::utils::Logger::warn(&format!("FPS na tela com o Frame Boost: {}", e));
            }
            Ok(painel())
        }
        _ => Err("Este gerador não liga pelo Otimiza.".to_string()),
    }
}

fn compat_motivo(r: &RecursoAvaliado) -> Option<String> {
    (!r.suporte.usavel()).then(|| match &r.suporte {
        compat::Suporte::NaoSuportado(m) | compat::Suporte::PrecisaAtualizar(m) => m.clone(),
        _ => "Este gerador não funciona aqui.".to_string(),
    })
}

/// Desliga tudo o que o Frame Boost ligou: o gerador do Otimiza e, pelo histórico, o AFMF e o Smooth Motion do jogo.
#[cfg(target_os = "windows")]
pub fn desligar() -> Result<Painel, String> {
    geracao::desligar();
    anotar_base(None);
    if let Some(j) = super::sessao::atual() {
        let mut log = crate::modules::changelog::ChangeLog::load();
        let otimizador = crate::modules::windows::WindowsOptimizer::new();
        let sm = crate::modules::windows::nvdriver::id_do_smooth_motion(&j.executavel);
        for id in [sm.as_str(), ID_DO_AFMF] {
            if log.is_applied(id) {
                otimizador.revert(id, &mut log)?;
            }
        }
    }
    if let Ok(mut m) = MEDICAO.lock() {
        *m = None;
    }
    Ok(painel())
}

/// Seis segundos de PresentMon no jogo: o FPS que chegou à tela com o gerador do driver, MEDIDO.
#[cfg(target_os = "windows")]
pub fn medir() -> Result<Painel, String> {
    use crate::modules::windows::presentmon;
    let jogo = super::sessao::atual().ok_or("Abra o jogo primeiro.")?;
    let quadros = presentmon::capturar(jogo.pid, 6)?;
    let r = presentmon::resumir(&quadros)?;
    if let Ok(mut m) = MEDICAO.lock() {
        *m = Some(Medicao {
            pid: jogo.pid,
            fps_exibido: r.fps_exibido,
            fps_do_jogo: r.fps_do_jogo_medio,
            quadros_gerados: r.quadros_gerados,
            geradores: r.geradores.clone(),
        });
    }
    Ok(painel())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placa_quase_cheia_comeca_leve() {
        assert!(comecar_leve(Some(85)));
        assert!(comecar_leve(Some(97)));
        assert!(!comecar_leve(Some(77)), "o FiveM do dono: placa a 77%");
        assert!(!comecar_leve(None), "sem leitura, modo normal");
    }

    #[test]
    fn avisa_quando_o_pc_nao_tem_folga() {
        assert!(aviso_de_folga(Some(0.24), Some(60))
            .unwrap()
            .contains("memória"));
        assert!(aviso_de_folga(Some(3.0), Some(95)).unwrap().contains("95%"));
        assert_eq!(aviso_de_folga(Some(3.0), Some(70)), None);
        assert_eq!(
            aviso_de_folga(None, None),
            None,
            "sem leitura, nada de aviso inventado"
        );
    }
    use crate::plataforma::metricas::Origem;

    fn gerando(apresentados: f64) -> geracao::Estado {
        geracao::Estado {
            situacao: Situacao::Gerando,
            fps_reais: 52.0,
            fps_apresentados: apresentados,
            multiplicador: 4,
            hz: 240,
            atraso_ms: Some(9.0),
            ..Default::default()
        }
    }

    fn medicao(exibido: f64, jogo: f64, gerados: usize) -> Medicao {
        Medicao {
            pid: 1,
            fps_exibido: exibido,
            fps_do_jogo: jogo,
            quadros_gerados: gerados,
            geradores: Vec::new(),
        }
    }

    #[test]
    fn afmf_ligado_sem_medir_nao_tem_numero() {
        let a = ativo_do_driver(Recurso::FrameBoostAfmf, None);
        assert_eq!(
            a.fps_na_tela.origem,
            Origem::Desconhecido,
            "nunca nativo × 2"
        );
        assert!(a.pelo_driver);
        assert!(a.nota.unwrap().contains("Medir agora"));
    }

    #[test]
    fn afmf_medido_mostra_o_exibido_do_presentmon() {
        let a = ativo_do_driver(Recurso::FrameBoostAfmf, Some(&medicao(118.0, 60.0, 340)));
        assert_eq!(a.fps_na_tela.valor, Some(118.0));
        assert_eq!(a.fps_na_tela.origem, Origem::Medido);
        assert_eq!(a.geracao_vista, Some(true));
    }

    #[test]
    fn smooth_motion_que_nao_se_identifica_conta_pela_tela_recebendo_mais() {
        assert!(
            geracao_vista(&medicao(110.0, 58.0, 0)),
            "não se identifica, mas a tela recebe quase o dobro"
        );
        assert!(!geracao_vista(&medicao(60.0, 59.0, 0)), "nada gerado");
    }

    #[test]
    fn driver_ligado_sem_gerar_explica_a_tela_cheia() {
        let a = ativo_do_driver(Recurso::FrameBoostAfmf, Some(&medicao(60.0, 60.0, 0)));
        assert_eq!(a.geracao_vista, Some(false));
        assert!(a.nota.unwrap().contains("tela cheia"));
    }

    #[test]
    fn ligado_mostra_o_exibido_medido_e_nao_o_nativo() {
        let mut e = gerando(204.0);
        e.fps_exibidos = Some(178.0);
        let v = fps_na_tela(&e);
        assert_eq!(v.valor, Some(178.0), "o que chegou à tela, não o apresentado (204) nem o nativo (52)");
        assert_eq!(v.origem, Origem::Medido);
        // Sem o PresentMon (sem administrador): o apresentado, marcado como estimativa.
        e.fps_exibidos = None;
        assert_eq!(fps_na_tela(&e).origem, Origem::Estimado);
        assert_eq!(fps_na_tela(&e).valor, Some(204.0));
        let a = ativo_de(&e).expect("ativo");
        assert_eq!(a.fps_na_tela.valor, Some(204.0));
        assert_eq!(a.atraso_ms.origem, Origem::Estimado, "atraso é estimativa");
    }

    #[test]
    fn procurando_a_janela_nao_inventa_numero() {
        let e = geracao::Estado {
            situacao: Situacao::ProcurandoJanela,
            fps_apresentados: 120.0,
            ..Default::default()
        };
        assert_eq!(fps_na_tela(&e).origem, Origem::Desconhecido);
        assert!(ativo_de(&e).is_some(), "continua ligado enquanto se refaz");
    }

    #[test]
    fn parado_nao_e_ativo_e_perda_de_fps_vira_aviso() {
        assert!(ativo_de(&geracao::Estado::default()).is_none());
        let e = geracao::Estado {
            situacao: Situacao::DesligadoPorPerdaDeFps,
            ..Default::default()
        };
        assert!(ativo_de(&e).is_none());
        assert!(aviso_de(&e).unwrap().contains("perdeu FPS"));
    }

    #[test]
    fn ligado_mostra_o_fps_do_jogo_e_o_que_ele_manteve_so_depois_de_comparar() {
        let mut e = gerando(180.0);
        let a = ativo_de(&e).unwrap();
        assert_eq!(a.fps_do_jogo, Some(52.0));
        assert_eq!(a.manteve_pct, None, "sem comparação, nada de porcentagem");
        e.razao_fps = Some(0.9837);
        e.comparacoes = 3;
        assert_eq!(ativo_de(&e).unwrap().manteve_pct, Some(98.4));
    }

    #[test]
    fn desligado_mostra_a_ultima_sessao_e_ligado_nao() {
        let mut e = gerando(180.0);
        e.processo = "FiveM_b3258_GTAProcess.exe".to_string();
        e.sessao.somar(120.0, 60.0, 178.0, Some(171.0));
        e.razao_fps = Some(0.98);
        e.comparacoes = 4;
        assert!(ultima_sessao_de(&e).is_none(), "ligado, o painel mostra o vivo");
        e.situacao = Situacao::Parado;
        let u = ultima_sessao_de(&e).unwrap();
        assert_eq!(u.minutos, 2.0);
        assert_eq!(u.fps_do_jogo, 60.0);
        assert_eq!(u.fps_na_tela.valor, Some(171.0));
        assert_eq!(u.fps_na_tela.origem, metricas::Origem::Medido);
        assert_eq!(u.manteve_pct, Some(98.0));
        assert!(!u.desligou_por_perda);
        e.situacao = Situacao::DesligadoPorPerdaDeFps;
        assert!(ultima_sessao_de(&e).unwrap().desligou_por_perda);
    }

    #[test]
    fn ligou_e_desligou_nao_tem_ultima_sessao() {
        let mut e = geracao::Estado {
            processo: "FiveM_b3258_GTAProcess.exe".to_string(),
            ..Default::default()
        };
        e.sessao.somar(10.0, 60.0, 178.0, None);
        assert!(ultima_sessao_de(&e).is_none());
    }

    #[test]
    fn multiplicador_pedido_que_nao_cabe_cai_para_o_maior_que_cabe() {
        assert_eq!(multiplicador_valido(3, &[2, 3, 4]), Some(3));
        assert_eq!(multiplicador_valido(4, &[2, 3]), Some(3));
        assert_eq!(multiplicador_valido(4, &[2]), Some(2));
        assert_eq!(multiplicador_valido(2, &[]), None);
    }

    #[test]
    fn sem_jogo_o_painel_fica_vazio_e_sem_estimativa() {
        let p = painel_de(
            None,
            Some(60.0),
            Some(144),
            &geracao::Estado::default(),
            None,
        );
        assert!(p.recomendado.is_none());
        assert!(p.multiplicadores.is_empty());
        assert!(p.estimativas.is_empty());
    }

    #[test]
    fn estimativa_e_faixa_estimada_e_nunca_passa_do_monitor() {
        let e = metricas::estimar_frame_boost(52.0, 4, Some(180.0)).unwrap();
        assert_eq!(e.origem, Origem::Estimado);
        assert!(e.maximo <= 180.0);
    }
}
