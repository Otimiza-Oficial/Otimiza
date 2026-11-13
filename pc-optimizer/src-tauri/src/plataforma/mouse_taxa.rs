// A TAXA REAL DO MOUSE (Plano 1.1, Onda 3b). O Windows não publica quantas vezes por segundo o mouse manda relato; a
// caixa diz "1000 Hz", e o mouse pode estar em 125 por causa do programa do fabricante, da porta ou do sem-fio.
//
// Mede contando a entrada bruta (Raw Input) numa janela só de mensagens, apenas durante os segundos em que a pessoa
// pede, mexendo o mouse. Não é gancho global (SetWindowsHookEx): é o mesmo canal que os jogos leem, e só guarda o
// INSTANTE de cada relato e de qual mouse ele veio — nunca posição, clique ou tecla.
//
// A conta é a mediana dos intervalos entre relatos do mouse que mais mandou (um relato atrasado dobraria a média).

use serde::Serialize;

/// Pausa da mão: intervalo acima disso não é a taxa, é o mouse parado.
const PAUSA_US: u64 = 50_000;

/// As taxas que os mouses de verdade usam.
const NOMINAIS: [u32; 7] = [125, 250, 500, 1000, 2000, 4000, 8000];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct TaxaDoMouse {
    /// Pela mediana dos intervalos. `None` com relatos de menos (a mão parou, ou não mexeu).
    pub taxa_hz: Option<f64>,
    /// A taxa padrão mais perto (125, 250, 500, 1000…), quando a medida cai a menos de 15% dela.
    pub nominal: Option<u32>,
    pub relatos: u32,
    /// Intervalos a menos de 25% da mediana: abaixo de ~80%, o mouse (ou o sem-fio) não segura a taxa.
    pub estavel_pct: Option<f64>,
    pub frase: String,
}

/// **Pura.** Dos relatos (`dispositivo`, instante em QPC) para os intervalos em microssegundos do mouse que mais
/// mandou. Pausas saem.
pub fn intervalos(relatos: &[(isize, i64)], frequencia_qpc: i64) -> Vec<u64> {
    if frequencia_qpc <= 0 {
        return Vec::new();
    }
    let mut contagem: std::collections::HashMap<isize, usize> = std::collections::HashMap::new();
    for (d, _) in relatos {
        *contagem.entry(*d).or_default() += 1;
    }
    let Some((principal, _)) = contagem.into_iter().max_by_key(|(_, n)| *n) else {
        return Vec::new();
    };
    let instantes: Vec<i64> = relatos
        .iter()
        .filter(|(d, _)| *d == principal)
        .map(|(_, t)| *t)
        .collect();
    instantes
        .windows(2)
        .filter_map(|j| {
            let us = ((j[1] - j[0]) as i128 * 1_000_000 / frequencia_qpc as i128) as i64;
            (us > 0 && (us as u64) < PAUSA_US).then_some(us as u64)
        })
        .collect()
}

/// **Pura.** Taxa, taxa padrão, estabilidade e a frase.
pub fn avaliar(intervalos_us: &[u64]) -> TaxaDoMouse {
    let relatos = intervalos_us.len() as u32;
    let Some(taxa) = crate::modules::mouse::taxa_de_varredura(intervalos_us) else {
        return TaxaDoMouse {
            taxa_hz: None,
            nominal: None,
            relatos,
            estavel_pct: None,
            frase: "Poucos relatos do mouse: mexa sem parar, em círculos, até a medida terminar.".to_string(),
        };
    };
    let nominal = NOMINAIS
        .iter()
        .copied()
        .min_by(|a, b| (*a as f64 - taxa).abs().total_cmp(&(*b as f64 - taxa).abs()))
        .filter(|n| (*n as f64 - taxa).abs() <= *n as f64 * 0.15);
    let mediana_us = 1_000_000.0 / taxa;
    let dentro = intervalos_us
        .iter()
        .filter(|i| (**i as f64 - mediana_us).abs() <= mediana_us * 0.25)
        .count();
    let estavel = dentro as f64 / intervalos_us.len() as f64 * 100.0;

    let base = match nominal {
        Some(n) => format!("Seu mouse está mandando {} relatos por segundo ({} Hz).", n, n),
        None => format!("Seu mouse está mandando cerca de {:.0} relatos por segundo.", taxa),
    };
    let complemento = if taxa < 400.0 {
        " É pouco para jogo de mira: no programa do mouse, procure \"polling rate\" ou \"taxa de relatório\" e suba para 1000 Hz."
    } else if estavel < 80.0 {
        " A taxa oscila: no sem-fio, aproxime o receptor do mouse (extensor USB na mesa) e use uma porta USB de trás do PC."
    } else {
        " Está bom para jogo."
    };
    TaxaDoMouse {
        taxa_hz: Some(taxa),
        nominal,
        relatos,
        estavel_pct: Some(estavel),
        frase: format!("{}{}", base, complemento),
    }
}

#[cfg(target_os = "windows")]
pub mod maquina {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
    use windows_sys::Win32::UI::Input::{
        GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
        RIDEV_INPUTSINK, RIDEV_REMOVE, RID_INPUT, RIM_TYPEMOUSE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, HWND_MESSAGE, MSG,
        PM_REMOVE, QS_ALLINPUT, WM_INPUT,
    };

    fn agora() -> i64 {
        let mut t = 0i64;
        unsafe { QueryPerformanceCounter(&mut t) };
        t
    }

    /// Conta os relatos de movimento por `segundos` (2 a 10). Roda numa thread própria: a fila de mensagens é dela.
    pub fn medir(segundos: u32) -> Result<super::TaxaDoMouse, String> {
        let segundos = segundos.clamp(2, 10);
        let mut freq = 0i64;
        unsafe { QueryPerformanceFrequency(&mut freq) };

        let classe: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let janela: HWND = unsafe {
            CreateWindowExW(
                0,
                classe.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        if janela.is_null() {
            return Err("O Windows não deixou abrir a janela de medida.".to_string());
        }
        // Página 1 (genérico de mesa), uso 2 (mouse). INPUTSINK: a janela de medida nunca fica em primeiro plano.
        let mut dispositivo = RAWINPUTDEVICE { usUsagePage: 1, usUsage: 2, dwFlags: RIDEV_INPUTSINK, hwndTarget: janela };
        let tam = std::mem::size_of::<RAWINPUTDEVICE>() as u32;
        if unsafe { RegisterRawInputDevices(&dispositivo, 1, tam) } == 0 {
            unsafe { DestroyWindow(janela) };
            return Err("O Windows não deixou ler a entrada do mouse.".to_string());
        }

        let mut relatos: Vec<(isize, i64)> = Vec::with_capacity(segundos as usize * 8000);
        let fim = agora() + freq * segundos as i64;
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        loop {
            let resta_ms = ((fim - agora()) * 1000 / freq.max(1)).max(0) as u32;
            if resta_ms == 0 {
                break;
            }
            unsafe { MsgWaitForMultipleObjects(0, std::ptr::null(), 0, resta_ms, QS_ALLINPUT) };
            while unsafe { PeekMessageW(&mut msg, janela, 0, 0, PM_REMOVE) } != 0 {
                if msg.message == WM_INPUT {
                    let instante = agora();
                    let mut dado: RAWINPUT = unsafe { std::mem::zeroed() };
                    let mut tamanho = std::mem::size_of::<RAWINPUT>() as u32;
                    let lidos = unsafe {
                        GetRawInputData(
                            msg.lParam as HRAWINPUT,
                            RID_INPUT,
                            &mut dado as *mut RAWINPUT as *mut _,
                            &mut tamanho,
                            std::mem::size_of::<RAWINPUTHEADER>() as u32,
                        )
                    };
                    if lidos != u32::MAX && dado.header.dwType == RIM_TYPEMOUSE {
                        let m = unsafe { dado.data.mouse };
                        // Só movimento: o relato de botão parado não diz a taxa.
                        if m.lLastX != 0 || m.lLastY != 0 {
                            relatos.push((dado.header.hDevice as isize, instante));
                        }
                    }
                }
                unsafe { DispatchMessageW(&msg) };
            }
        }

        dispositivo.dwFlags = RIDEV_REMOVE;
        dispositivo.hwndTarget = std::ptr::null_mut();
        unsafe {
            RegisterRawInputDevices(&dispositivo, 1, tam);
            DestroyWindow(janela);
        }
        Ok(super::avaliar(&super::intervalos(&relatos, freq)))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Relatos a cada `passo_us`, no relógio de 10 MHz (o QPC comum).
    fn serie(dispositivo: isize, n: usize, passo_us: i64) -> Vec<(isize, i64)> {
        (0..n as i64).map(|i| (dispositivo, i * passo_us * 10)).collect()
    }

    #[test]
    fn mouse_a_1000_hz_da_1000_e_estavel() {
        let t = avaliar(&intervalos(&serie(7, 600, 1000), 10_000_000));
        assert_eq!(t.nominal, Some(1000));
        assert!(t.estavel_pct.unwrap() > 99.0);
        assert!(t.frase.contains("1000 Hz") && t.frase.contains("bom"), "{}", t.frase);
    }

    #[test]
    fn mouse_a_125_hz_manda_subir_a_taxa() {
        let t = avaliar(&intervalos(&serie(7, 400, 8000), 10_000_000));
        assert_eq!(t.nominal, Some(125));
        assert!(t.frase.contains("1000 Hz") && t.frase.contains("programa do mouse"), "{}", t.frase);
    }

    #[test]
    fn so_conta_o_mouse_que_mais_mandou_e_tira_as_pausas() {
        let mut r = serie(1, 500, 2000); // 500 Hz
        r.extend(serie(2, 50, 1000).into_iter().map(|(d, t)| (d, t + 3)));
        // Uma pausa de meio segundo no meio do mouse principal.
        let pausa: Vec<(isize, i64)> = serie(1, 300, 2000).into_iter().map(|(d, t)| (d, t + 15_000_000)).collect();
        r.extend(pausa);
        r.sort_by_key(|(_, t)| *t);
        let iv = intervalos(&r, 10_000_000);
        assert!(iv.iter().all(|i| *i < PAUSA_US));
        assert_eq!(avaliar(&iv).nominal, Some(500));
    }

    #[test]
    fn taxa_que_oscila_e_dita() {
        // Metade a 1 ms, metade a 3 ms: mediana cai num dos lados, e metade fica longe dela.
        let iv: Vec<u64> = (0..400).map(|i| if i % 2 == 0 { 1000 } else { 3000 }).collect();
        let t = avaliar(&iv);
        assert!(t.estavel_pct.unwrap() < 80.0);
    }

    /// Lê a máquina: `cargo test --lib mede_nesta_maquina -- --ignored --nocapture` (mexa o mouse por 3 s).
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn mede_nesta_maquina() {
        let t = maquina::medir(3).expect("a medida tem de abrir e fechar sem erro");
        println!("{t:?}");
    }

    #[test]
    fn poucos_relatos_pede_para_mexer_e_nao_inventa_taxa() {
        let t = avaliar(&intervalos(&serie(1, 30, 1000), 10_000_000));
        assert_eq!(t.taxa_hz, None);
        assert!(t.frase.contains("mexa"));
    }
}
