// A mira na tela: uma janela pequena, transparente e que não aceita clique, no centro da janela do jogo. Não toca no
// processo do jogo (nem handle, nem injeção): só pergunta ao Windows onde a janela dele está e se ela está na frente.
//
// Some quando o jogo sai da frente (a mira não fica no Discord) e, se a pessoa quiser, enquanto o botão direito está
// apertado (a mira do jogo aparece ao mirar). Tela cheia EXCLUSIVA não mostra janela nenhuma por cima: a tela diz isso.
//
// No CS2 a mira certa é a do próprio jogo: `comandos_cs2` gera os comandos para colar no console, sem sobreposição.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mira {
    pub estilo: String,
    pub cor: String,
    pub tamanho: f64,
    pub espessura: f64,
    pub abertura: f64,
    pub opacidade: f64,
    pub contorno: bool,
    #[serde(default)]
    pub esconder_ao_mirar: bool,
}

const ESTILOS: &[&str] = &["ponto", "cruz", "cruz-ponto", "circulo", "t"];

/// Pura: o que a tela mandou, dentro dos limites. A janela da mira recebe só valores conhecidos.
pub fn validar(m: &Mira) -> Result<Mira, String> {
    if !ESTILOS.contains(&m.estilo.as_str()) {
        return Err(format!("estilo de mira desconhecido: {}", m.estilo));
    }
    let cor_ok = m.cor.len() == 7
        && m.cor.starts_with('#')
        && m.cor[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !cor_ok {
        return Err("a cor da mira precisa ser #rrggbb".to_string());
    }
    let n = |v: f64, min: f64, max: f64| {
        if v.is_finite() {
            v.clamp(min, max)
        } else {
            min
        }
    };
    Ok(Mira {
        estilo: m.estilo.clone(),
        cor: m.cor.to_lowercase(),
        tamanho: n(m.tamanho, 1.0, 40.0),
        espessura: n(m.espessura, 0.5, 8.0),
        abertura: n(m.abertura, 0.0, 20.0),
        opacidade: n(m.opacidade, 0.1, 1.0),
        contorno: m.contorno,
        esconder_ao_mirar: m.esconder_ao_mirar,
    })
}

/// Pura: a mesma mira em comandos do console do CS2 (`cl_crosshair*`). A escala do CS2 é outra: o tamanho e a
/// espessura dele valem mais ou menos metade dos pixels daqui, e a abertura começa em -2 no padrão do jogo.
pub fn comandos_cs2(m: &Mira) -> String {
    let (r, g, b) = (
        u8::from_str_radix(&m.cor[1..3], 16).unwrap_or(255),
        u8::from_str_radix(&m.cor[3..5], 16).unwrap_or(255),
        u8::from_str_radix(&m.cor[5..7], 16).unwrap_or(255),
    );
    let ponto = matches!(m.estilo.as_str(), "ponto" | "cruz-ponto" | "circulo");
    let sem_linhas = m.estilo == "ponto";
    let f = |v: f64| {
        format!("{:.1}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    };
    [
        "cl_crosshairstyle 4".to_string(),
        format!(
            "cl_crosshairsize {}",
            if sem_linhas {
                "0".to_string()
            } else {
                f(m.tamanho / 2.0)
            }
        ),
        format!("cl_crosshairthickness {}", f(m.espessura / 2.0)),
        format!("cl_crosshairgap {}", f(m.abertura - 4.0)),
        format!("cl_crosshairdot {}", ponto as u8),
        format!("cl_crosshair_t {}", (m.estilo == "t") as u8),
        format!("cl_crosshair_drawoutline {}", m.contorno as u8),
        "cl_crosshair_outlinethickness 1".to_string(),
        "cl_crosshaircolor 5".to_string(),
        format!("cl_crosshaircolor_r {}", r),
        format!("cl_crosshaircolor_g {}", g),
        format!("cl_crosshaircolor_b {}", b),
        "cl_crosshairusealpha 1".to_string(),
        format!("cl_crosshairalpha {}", (m.opacidade * 255.0).round() as u32),
    ]
    .join("; ")
}

/// Pura: onde a janela da mira fica para o centro dela cair no centro da área do jogo.
pub fn posicao(area: (i32, i32, i32, i32), lado: i32) -> (i32, i32) {
    let (x, y, w, h) = area;
    (x + w / 2 - lado / 2, y + h / 2 - lado / 2)
}

/// Texto JSON da mira em base64 URL-safe para ir no endereço da página da mira (`mira.html#...`).
pub fn no_endereco(m: &Mira) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(m).unwrap_or_default())
}

#[derive(Debug, Clone, Serialize)]
pub struct Estado {
    pub ligada: bool,
    pub jogo: Option<String>,
}

#[cfg(target_os = "windows")]
mod janela {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use tauri::Manager;

    pub const ROTULO: &str = "mira";
    const LADO: i32 = 220;

    struct Ativa {
        jogo: String,
        parar: Arc<AtomicBool>,
        mira: Arc<Mutex<Mira>>,
    }

    static ATIVA: Mutex<Option<Ativa>> = Mutex::new(None);

    pub fn estado() -> Estado {
        let g = ATIVA.lock().ok();
        let a = g.as_ref().and_then(|g| g.as_ref());
        Estado {
            ligada: a.is_some(),
            jogo: a.map(|a| a.jogo.clone()),
        }
    }

    pub fn desligar(app: &tauri::AppHandle) {
        if let Ok(mut g) = ATIVA.lock() {
            if let Some(a) = g.take() {
                a.parar.store(true, Ordering::SeqCst);
            }
        }
        if let Some(w) = app.get_webview_window(ROTULO) {
            let _ = w.close();
        }
    }

    pub fn atualizar(app: &tauri::AppHandle, m: Mira) -> Result<(), String> {
        let g = ATIVA.lock().map_err(|_| "a mira travou".to_string())?;
        let Some(a) = g.as_ref() else { return Ok(()) };
        if let Ok(mut atual) = a.mira.lock() {
            *atual = m.clone();
        }
        if let Some(w) = app.get_webview_window(ROTULO) {
            let json = serde_json::to_string(&m).map_err(|e| e.to_string())?;
            let _ = w.eval(&format!("window.mira && window.mira({})", json));
        }
        Ok(())
    }

    fn botao_direito_apertado() -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_RBUTTON};
        unsafe { (GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16 & 0x8000) != 0 }
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

    pub fn ligar(app: &tauri::AppHandle, pid: u32, jogo: String, m: Mira) -> Result<(), String> {
        desligar(app);

        let w = tauri::WebviewWindowBuilder::new(
            app,
            ROTULO,
            tauri::WebviewUrl::App(format!("mira.html#{}", no_endereco(&m)).into()),
        )
        .title("Mira do Otimiza")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .shadow(false)
        .visible(false)
        .inner_size(LADO as f64, LADO as f64)
        .build()
        .map_err(|e| format!("Não consegui abrir a janela da mira: {}", e))?;
        // Clique passa direto para o jogo: a mira nunca rouba o mouse.
        w.set_ignore_cursor_events(true)
            .map_err(|e| e.to_string())?;
        let _ = w.set_size(tauri::PhysicalSize::new(LADO as u32, LADO as u32));

        let parar = Arc::new(AtomicBool::new(false));
        let mira = Arc::new(Mutex::new(m));
        if let Ok(mut g) = ATIVA.lock() {
            *g = Some(Ativa {
                jogo,
                parar: parar.clone(),
                mira: mira.clone(),
            });
        }

        let app = app.clone();
        std::thread::spawn(move || {
            use crate::modules::windows::geracao::{area_do_cliente, janela_do_processo};
            let mut visivel = false;
            let mut onde = (i32::MIN, i32::MIN);
            let mut sem_janela = 0u32;
            // 30 ms: a mira acompanha a janela e o botão direito sem pesar (GetAsyncKeyState e GetClientRect custam
            // microssegundos).
            while !parar.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(30));
                let Some(w) = app.get_webview_window(ROTULO) else {
                    break;
                };

                let Some(area) = janela_do_processo(pid)
                    .and_then(area_do_cliente)
                    .filter(|a| a.largura > 0 && a.altura > 0)
                else {
                    // Sem janela é jogo minimizado (Alt+Tab) ou fechado. Esconde; desliga só quando a sessão do jogo,
                    // que confere o processo a cada 4 s, disser que ele fechou de verdade.
                    if visivel {
                        let _ = w.hide();
                        visivel = false;
                    }
                    sem_janela += 1;
                    if sem_janela % 33 == 0
                        && crate::plataforma::sessao::atual().map(|j| j.pid) != Some(pid)
                    {
                        break;
                    }
                    continue;
                };
                sem_janela = 0;

                let pos = posicao((area.x, area.y, area.largura, area.altura), LADO);
                if pos != onde {
                    let _ = w.set_position(tauri::PhysicalPosition::new(pos.0, pos.1));
                    onde = pos;
                }

                let esconder = mira.lock().map(|m| m.esconder_ao_mirar).unwrap_or(false);
                let mostrar = pid_na_frente() == Some(pid)
                    && !(esconder && botao_direito_apertado())
                    && !crate::modules::windows::geracao::tela_cheia_exclusiva();
                if mostrar != visivel {
                    let _ = if mostrar { w.show() } else { w.hide() };
                    visivel = mostrar;
                }
            }
            if let Some(w) = app.get_webview_window(ROTULO) {
                let _ = w.close();
            }
            if let Ok(mut g) = ATIVA.lock() {
                if g.as_ref().is_some_and(|a| Arc::ptr_eq(&a.parar, &parar)) {
                    *g = None;
                }
            }
        });
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use janela::{atualizar, desligar, estado, ligar};

#[cfg(test)]
mod tests {
    use super::*;

    fn mira() -> Mira {
        Mira {
            estilo: "cruz-ponto".to_string(),
            cor: "#00E5FF".to_string(),
            tamanho: 10.0,
            espessura: 2.0,
            abertura: 4.0,
            opacidade: 1.0,
            contorno: true,
            esconder_ao_mirar: false,
        }
    }

    #[test]
    fn valida_e_aperta_os_limites() {
        let mut m = mira();
        m.tamanho = 999.0;
        m.opacidade = f64::NAN;
        let v = validar(&m).unwrap();
        assert_eq!(v.tamanho, 40.0);
        assert_eq!(v.opacidade, 0.1);
        assert_eq!(v.cor, "#00e5ff");
    }

    #[test]
    fn recusa_estilo_e_cor_desconhecidos() {
        assert!(validar(&Mira {
            estilo: "<script>".to_string(),
            ..mira()
        })
        .is_err());
        for cor in ["red", "#12345", "#gggggg", "#1234567", "javascript:"] {
            assert!(
                validar(&Mira {
                    cor: cor.to_string(),
                    ..mira()
                })
                .is_err(),
                "{}",
                cor
            );
        }
    }

    #[test]
    fn comandos_do_cs2_levam_a_cor_e_o_ponto() {
        let c = comandos_cs2(&validar(&mira()).unwrap());
        assert!(c.contains("cl_crosshaircolor_r 0"));
        assert!(c.contains("cl_crosshaircolor_g 229"));
        assert!(c.contains("cl_crosshaircolor_b 255"));
        assert!(c.contains("cl_crosshairdot 1"));
        assert!(c.contains("cl_crosshairsize 5"));
        assert!(c.contains("cl_crosshairgap 0"));
        assert!(c.contains("cl_crosshairalpha 255"));
    }

    #[test]
    fn mira_de_ponto_no_cs2_nao_tem_linhas() {
        let c = comandos_cs2(&Mira {
            estilo: "ponto".to_string(),
            ..mira()
        });
        assert!(c.contains("cl_crosshairsize 0"));
        assert!(c.contains("cl_crosshairdot 1"));
    }

    #[test]
    fn centro_da_mira_no_centro_do_jogo() {
        assert_eq!(posicao((0, 0, 1920, 1080), 220), (850, 430));
        assert_eq!(
            posicao((1920, 0, 2560, 1440), 220),
            (1920 + 1170, 610),
            "segundo monitor"
        );
    }

    #[test]
    fn endereco_vai_e_volta() {
        use base64::Engine;
        let m = validar(&mira()).unwrap();
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(no_endereco(&m))
            .unwrap();
        assert_eq!(serde_json::from_slice::<Mira>(&bytes).unwrap(), m);
    }
}
