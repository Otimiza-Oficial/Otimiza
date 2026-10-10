// O QUE FICA POR CIMA DO JOGO. Em tela cheia exclusiva, uma janela de outro programa fixada "sempre por cima" no monitor
// do jogo derruba a tela cheia, e o jogo minimiza sozinho. Medido no FiveM do dono: o MyDockFinder (`dock_64.exe`) deixa
// uma barra de 1920×24 no topo e uma faixa de 1 pixel embaixo; o jogo minimizava de 0,2 a 9 s depois de voltar à frente,
// e fechando o Dock parou. Só lê as janelas: nada é fechado sem a pessoa.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub largura: i32,
    pub altura: i32,
}

impl Area {
    fn cruza(&self, outra: &Area) -> bool {
        self.largura > 0
            && self.altura > 0
            && outra.largura > 0
            && outra.altura > 0
            && self.x < outra.x + outra.largura
            && outra.x < self.x + self.largura
            && self.y < outra.y + outra.altura
            && outra.y < self.y + self.altura
    }
}

/// Uma janela visível e "sempre por cima", como o Windows a lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JanelaVista {
    pub pid: u32,
    pub processo: String,
    pub area: Area,
}

/// Do Windows e do próprio jogo: a barra de tarefas e o menu Iniciar convivem com a tela cheia.
const DO_SISTEMA: [&str; 7] = [
    "explorer.exe",
    "shellexperiencehost.exe",
    "startmenuexperiencehost.exe",
    "searchhost.exe",
    "textinputhost.exe",
    "dwm.exe",
    "pc-optimizer.exe",
];

/// Pura: os programas (sem repetir, na ordem em que aparecem) com janela sempre por cima no monitor do jogo, fora o
/// próprio jogo, o que ele abre (mesmo prefixo, ex. `FiveM_ChromeBrowser`) e o Windows.
pub fn sobre_o_jogo(janelas: &[JanelaVista], monitor: Area, pid_do_jogo: u32, executavel: &str) -> Vec<String> {
    let prefixo_do_jogo = executavel
        .to_lowercase()
        .split(['_', '.', ' '])
        .next()
        .unwrap_or_default()
        .to_string();
    let mut nomes: Vec<String> = Vec::new();
    for j in janelas {
        let nome = j.processo.to_lowercase();
        let do_jogo = j.pid == pid_do_jogo
            || (prefixo_do_jogo.len() >= 3 && nome.starts_with(&prefixo_do_jogo));
        if do_jogo || DO_SISTEMA.contains(&nome.as_str()) || !j.area.cruza(&monitor) {
            continue;
        }
        if !nomes.iter().any(|n| n.eq_ignore_ascii_case(&j.processo)) {
            nomes.push(j.processo.clone());
        }
    }
    nomes
}

/// As janelas visíveis "sempre por cima" agora, com o processo de cada uma.
#[cfg(target_os = "windows")]
pub fn listar() -> Vec<JanelaVista> {
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible,
        GWL_EXSTYLE, WS_EX_TOPMOST,
    };

    unsafe extern "system" fn ao_enumerar(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let v = &mut *(lparam.0 as *mut Vec<(u32, Area)>);
        let topo = (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0) != 0;
        if topo && IsWindowVisible(hwnd).as_bool() {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let mut r = RECT::default();
            if GetWindowRect(hwnd, &mut r).is_ok() {
                v.push((
                    pid,
                    Area {
                        x: r.left,
                        y: r.top,
                        largura: r.right - r.left,
                        altura: r.bottom - r.top,
                    },
                ));
            }
        }
        true.into()
    }

    let mut cruas: Vec<(u32, Area)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(ao_enumerar), LPARAM(&mut cruas as *mut _ as isize));
    }
    let mut sistema = sysinfo::System::new();
    sistema.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    cruas
        .into_iter()
        .map(|(pid, area)| JanelaVista {
            pid,
            processo: sistema
                .process(sysinfo::Pid::from_u32(pid))
                .map(|p| p.name().to_string_lossy().into_owned())
                .unwrap_or_default(),
            area,
        })
        .collect()
}

/// O monitor onde está a janela do jogo (o mais próximo dela; minimizada, o principal).
#[cfg(target_os = "windows")]
pub fn monitor_do_jogo(pid: u32) -> Option<Area> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        MONITOR_DEFAULTTOPRIMARY,
    };
    unsafe {
        let monitor = match crate::modules::windows::geracao::janela_do_processo(pid) {
            Some(hwnd) => MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
            None => MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY),
        };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info).as_bool().then(|| Area {
            x: info.rcMonitor.left,
            y: info.rcMonitor.top,
            largura: info.rcMonitor.right - info.rcMonitor.left,
            altura: info.rcMonitor.bottom - info.rcMonitor.top,
        })
    }
}

/// Os programas por cima do jogo aberto agora.
#[cfg(target_os = "windows")]
pub fn agora(pid_do_jogo: u32, executavel: &str) -> Vec<String> {
    match monitor_do_jogo(pid_do_jogo) {
        Some(monitor) => sobre_o_jogo(&listar(), monitor, pid_do_jogo, executavel),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRINCIPAL: Area = Area {
        x: 0,
        y: 0,
        largura: 1920,
        altura: 1080,
    };

    fn janela(pid: u32, processo: &str, x: i32, y: i32, largura: i32, altura: i32) -> JanelaVista {
        JanelaVista {
            pid,
            processo: processo.to_string(),
            area: Area {
                x,
                y,
                largura,
                altura,
            },
        }
    }

    #[test]
    fn o_dock_do_dono_aparece_uma_vez() {
        // O que o Windows listou no PC do dono com o FiveM aberto (21:30 de 09/10).
        let vistas = [
            janela(5860, "explorer.exe", 0, 1032, 1920, 48),
            janela(10488, "dock_64.exe", 760, 1056, 400, 12),
            janela(10488, "dock_64.exe", -32000, -32000, 20, 20),
            janela(10488, "dock_64.exe", 0, 1079, 1920, 1),
            janela(10488, "dock_64.exe", 0, 0, 1920, 24),
            janela(5860, "explorer.exe", 1920, 1872, 1080, 48),
        ];
        assert_eq!(
            sobre_o_jogo(&vistas, PRINCIPAL, 7548, "FiveM_b3258_GTAProcess.exe"),
            vec!["dock_64.exe".to_string()]
        );
    }

    #[test]
    fn o_jogo_o_que_ele_abre_e_o_windows_nao_contam() {
        let vistas = [
            janela(7548, "FiveM_b3258_GTAProcess.exe", 0, 0, 1920, 1080),
            janela(13220, "FiveM_ChromeBrowser", 0, 0, 1920, 1080),
            janela(1, "pc-optimizer.exe", 10, 10, 200, 40),
            janela(2, "TextInputHost.exe", 0, 1000, 400, 80),
        ];
        assert!(sobre_o_jogo(&vistas, PRINCIPAL, 7548, "FiveM_b3258_GTAProcess.exe").is_empty());
    }

    /// Com o jogo aberto: `cargo test --lib por_cima::tests::ao_vivo -- --ignored --nocapture`. Só lê.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn ao_vivo() {
        let exe = std::env::var("OTIMIZA_JOGO").unwrap_or("FiveM_b3258_GTAProcess.exe".into());
        let (pid, _) = crate::modules::windows::frames::encontrar_processo(&exe).expect("jogo aberto");
        println!("monitor do jogo: {:?}", monitor_do_jogo(pid));
        for j in listar() {
            println!("  {:?}", j);
        }
        println!("por cima do jogo: {:?}", agora(pid, &exe));
    }

    #[test]
    fn so_conta_o_monitor_do_jogo() {
        // A janela no segundo monitor (à direita) não cobre o jogo.
        let vistas = [janela(9, "outro.exe", 1920, 0, 1080, 24)];
        assert!(sobre_o_jogo(&vistas, PRINCIPAL, 1, "jogo.exe").is_empty());
        let vistas = [janela(9, "outro.exe", 1900, 0, 100, 24)];
        assert_eq!(sobre_o_jogo(&vistas, PRINCIPAL, 1, "jogo.exe"), vec!["outro.exe"]);
    }
}
