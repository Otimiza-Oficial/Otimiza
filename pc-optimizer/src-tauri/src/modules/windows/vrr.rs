// O que o MONITOR anuncia sobre taxa variável (VRR: G-SYNC Compatível, FreeSync, Adaptive-Sync), lido do EDID que o
// Windows guarda no registro. Só lê. Serve para uma decisão: o limite de FPS "para G-Sync" só é oferecido quando o
// monitor anuncia VRR, porque sem ele o limite só tira quadro. Anunciar não é estar ligado: ligar é no painel da placa.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vrr {
    /// O bloco da AMD (FreeSync) está no EDID: o monitor faz taxa variável.
    Anunciado,
    /// Faixa larga no descritor de limites (ex.: 48–144 Hz). Quase sempre é VRR, mas o EDID não jura.
    Provavel,
    /// Faixa estreita ou sem faixa: taxa fixa.
    NaoAnunciado,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorVrr {
    /// Nome gravado no próprio monitor (descritor 0xFC), quando há.
    pub nome: Option<String>,
    /// Faixa vertical do descritor de limites (0xFD).
    pub min_hz: Option<u32>,
    pub max_hz: Option<u32>,
    pub vrr: Vrr,
}

/// Bloco de dados do fabricante AMD no CTA-861 (OUI 00-00-1A, gravado ao contrário).
const OUI_AMD: [u8; 3] = [0x1A, 0x00, 0x00];

/// Pura. `None` se não for EDID.
pub fn ler_edid(b: &[u8]) -> Option<MonitorVrr> {
    if b.len() < 128 || b[0..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
        return None;
    }

    let mut nome = None;
    let mut faixa = None;
    for inicio in [54usize, 72, 90, 108] {
        let d = &b[inicio..inicio + 18];
        if d[0] != 0 || d[1] != 0 || d[2] != 0 {
            continue;
        }
        match d[3] {
            0xFC => {
                let texto: String = d[5..18]
                    .iter()
                    .take_while(|c| **c != 0x0A)
                    .map(|c| *c as char)
                    .collect::<String>()
                    .trim()
                    .to_string();
                if !texto.is_empty() {
                    nome = Some(texto);
                }
            }
            0xFD => {
                // EDID 1.4: bit 0 e bit 1 do byte 4 somam 255 ao mínimo e ao máximo vertical.
                let min = d[5] as u32 + if d[4] & 0x01 != 0 { 255 } else { 0 };
                let max = d[6] as u32 + if d[4] & 0x02 != 0 { 255 } else { 0 };
                faixa = Some((min, max));
            }
            _ => {}
        }
    }

    let freesync = (1..=b[126] as usize)
        .filter_map(|i| b.get(i * 128..i * 128 + 128))
        .filter(|ext| ext[0] == 0x02)
        .any(|ext| bloco_amd(ext));

    let vrr = if freesync {
        Vrr::Anunciado
    } else {
        match faixa {
            Some((min, max)) if max >= 90 && max.saturating_sub(min) >= 40 => Vrr::Provavel,
            _ => Vrr::NaoAnunciado,
        }
    };

    Some(MonitorVrr { nome, min_hz: faixa.map(|f| f.0), max_hz: faixa.map(|f| f.1), vrr })
}

/// Percorre a coleção de blocos de dados do CTA procurando o bloco do fabricante (tag 3) com o OUI da AMD.
fn bloco_amd(ext: &[u8]) -> bool {
    let fim = (ext[2] as usize).min(127);
    let mut i = 4;
    while i < fim {
        let cabeca = ext[i];
        let tag = cabeca >> 5;
        let tamanho = (cabeca & 0x1F) as usize;
        if tag == 3 && tamanho >= 3 && ext.get(i + 1..i + 4) == Some(&OUI_AMD[..]) {
            return true;
        }
        i += 1 + tamanho;
    }
    false
}

/// Pura: o modelo no id de dispositivo do monitor ("MONITOR\AOCB401\{4d36e96e-...}\0003" → "AOCB401").
pub fn modelo_do_id(id: &str) -> Option<String> {
    let mut partes = id.split('\\');
    partes.next().filter(|p| p.eq_ignore_ascii_case("MONITOR"))?;
    partes.next().filter(|m| !m.is_empty()).map(|m| m.to_uppercase())
}

/// Modelos dos monitores ligados à área de trabalho agora, pelo próprio Windows.
#[cfg(target_os = "windows")]
fn modelos_ligados() -> Vec<String> {
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP};

    fn texto(bruto: &[u16]) -> String {
        let fim = bruto.iter().position(|c| *c == 0).unwrap_or(bruto.len());
        String::from_utf16_lossy(&bruto[..fim])
    }

    let mut modelos = Vec::new();
    unsafe {
        for indice in 0..16u32 {
            let mut adaptador: DISPLAY_DEVICEW = std::mem::zeroed();
            adaptador.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            if EnumDisplayDevicesW(std::ptr::null(), indice, &mut adaptador, 0) == 0 {
                break;
            }
            if adaptador.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0 {
                continue;
            }
            for m in 0..4u32 {
                let mut monitor: DISPLAY_DEVICEW = std::mem::zeroed();
                monitor.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(adaptador.DeviceName.as_ptr(), m, &mut monitor, 0) == 0 {
                    break;
                }
                if let Some(modelo) = modelo_do_id(&texto(&monitor.DeviceID)) {
                    if !modelos.contains(&modelo) {
                        modelos.push(modelo);
                    }
                }
            }
        }
    }
    modelos
}

/// Um por modelo ligado agora. O registro guarda um EDID por porta em que o monitor já esteve: o de mesmo modelo é o
/// mesmo monitor (ou um igual), e basta o primeiro que se lê.
#[cfg(target_os = "windows")]
pub fn monitores() -> Vec<MonitorVrr> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let Ok(display) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Enum\DISPLAY", KEY_READ)
    else {
        return Vec::new();
    };
    modelos_ligados()
        .into_iter()
        .filter_map(|modelo| {
            let chave_modelo = display.open_subkey_with_flags(&modelo, KEY_READ).ok()?;
            chave_modelo.enum_keys().flatten().find_map(|instancia| {
                let parametros = chave_modelo.open_subkey_with_flags(format!(r"{}\Device Parameters", instancia), KEY_READ).ok()?;
                ler_edid(&parametros.get_raw_value("EDID").ok()?.bytes)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edid(faixa: Option<(u8, u8)>, nome: &str, amd: bool) -> Vec<u8> {
        let mut b = vec![0u8; if amd { 256 } else { 128 }];
        b[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
        if let Some((min, max)) = faixa {
            b[72..77].copy_from_slice(&[0, 0, 0, 0xFD, 0]);
            b[77] = min;
            b[78] = max;
        }
        b[90..95].copy_from_slice(&[0, 0, 0, 0xFC, 0]);
        for (i, c) in nome.bytes().chain(std::iter::once(0x0A)).enumerate().take(13) {
            b[95 + i] = c;
        }
        if amd {
            b[126] = 1;
            let ext = &mut b[128..256];
            ext[0] = 0x02;
            ext[1] = 0x03;
            // Bloco do fabricante: tag 3, tamanho 8, OUI da AMD e o resto da carga do FreeSync.
            ext[4..13].copy_from_slice(&[(3 << 5) | 8, 0x1A, 0x00, 0x00, 0x01, 0x01, 48, 144, 0x00]);
            ext[2] = 13;
        }
        b
    }

    #[test]
    fn freesync_no_edid_e_anunciado() {
        let m = ler_edid(&edid(Some((48, 144)), "AOC 24G2", true)).unwrap();
        assert_eq!(m.vrr, Vrr::Anunciado);
        assert_eq!(m.nome.as_deref(), Some("AOC 24G2"));
        assert_eq!((m.min_hz, m.max_hz), (Some(48), Some(144)));
    }

    #[test]
    fn faixa_larga_sem_bloco_amd_e_provavel() {
        assert_eq!(ler_edid(&edid(Some((48, 144)), "X", false)).unwrap().vrr, Vrr::Provavel);
    }

    #[test]
    fn monitor_de_60_fixo_nao_anuncia() {
        assert_eq!(ler_edid(&edid(Some((56, 76)), "Y", false)).unwrap().vrr, Vrr::NaoAnunciado);
        assert_eq!(ler_edid(&edid(None, "Z", false)).unwrap().vrr, Vrr::NaoAnunciado);
    }

    #[test]
    fn o_modelo_sai_do_id_do_monitor() {
        assert_eq!(modelo_do_id(r"MONITOR\AOCB401\{4d36e96e-e325-11ce-bfc1-08002be10318}\0003").as_deref(), Some("AOCB401"));
        assert_eq!(modelo_do_id(r"PCI\VEN_10DE"), None);
    }

    #[test]
    fn lixo_nao_e_edid() {
        assert_eq!(ler_edid(&[0u8; 128]), None);
        assert_eq!(ler_edid(&[0u8; 10]), None);
    }

    /// Lê os monitores DESTA máquina: `cargo test --lib -- --ignored --nocapture vrr_desta_maquina`.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn vrr_desta_maquina() {
        for m in monitores() {
            println!("{:?}", m);
        }
    }
}
