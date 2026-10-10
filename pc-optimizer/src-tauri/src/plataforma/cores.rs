// Cores do monitor por jogo: saturação, brilho, contraste, gama e matiz. Três caminhos, do melhor para o mais simples:
//
// - AMD: o ADLX oficial (`amddriver::escrever_cor`), com saturação, matiz, brilho e contraste do próprio driver.
// - NVIDIA: a vibração digital do NVAPI (`NvAPI_GetDVCInfoEx` / `NvAPI_SetDVCLevelEx`). Os números dessas funções
//   não estão no cabeçalho público do NVAPI: o interruptor `vibracao_nvidia` desliga (provados numa GTX 1650).
// - Qualquer placa: a rampa de gama do Windows (`SetDeviceGammaRamp`), que faz brilho, contraste e gama. Saturação
//   não dá por ela: a tela diz isso.
//
// O mais importante é DEVOLVER: os valores de antes vão para `cores-originais.json` ANTES de qualquer mudança, e a
// abertura do Otimiza devolve o que estiver lá (o Otimiza pode ter caído com o monitor saturado).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Na escala da tela: saturação 0 a 100 (50 é o normal), brilho e contraste -40 a 40, gama 70 a 150 (100 é o normal),
/// matiz -30 a 30 graus.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ajuste {
    pub saturacao: i32,
    pub brilho: i32,
    pub contraste: i32,
    pub gama: i32,
    pub matiz: i32,
}

pub const NORMAL: Ajuste = Ajuste {
    saturacao: 50,
    brilho: 0,
    contraste: 0,
    gama: 100,
    matiz: 0,
};

impl Ajuste {
    /// Pura: dentro dos limites da tela.
    pub fn limitado(self) -> Ajuste {
        Ajuste {
            saturacao: self.saturacao.clamp(0, 100),
            brilho: self.brilho.clamp(-40, 40),
            contraste: self.contraste.clamp(-40, 40),
            gama: self.gama.clamp(70, 150),
            matiz: self.matiz.clamp(-30, 30),
        }
    }

    pub fn e_normal(&self) -> bool {
        *self == NORMAL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Caminho {
    Amd,
    Nvidia,
    Gama,
}

/// O que cada caminho sabe mudar. A tela desliga o controle que não vale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Suporta {
    pub saturacao: bool,
    pub brilho: bool,
    pub contraste: bool,
    pub gama: bool,
    pub matiz: bool,
}

pub fn suporta(c: Caminho) -> Suporta {
    match c {
        Caminho::Amd => Suporta {
            saturacao: true,
            brilho: true,
            contraste: true,
            gama: true,
            matiz: true,
        },
        // A vibração digital só satura; brilho, contraste e gama vêm da rampa do Windows ao lado.
        Caminho::Nvidia => Suporta {
            saturacao: true,
            brilho: true,
            contraste: true,
            gama: true,
            matiz: false,
        },
        Caminho::Gama => Suporta {
            saturacao: false,
            brilho: true,
            contraste: true,
            gama: true,
            matiz: false,
        },
    }
}

/// Pura: a rampa de gama (256 níveis por canal, 0 a 65535) para brilho, contraste e gama. O Windows recusa rampa que
/// se afasta demais da identidade, então os limites da tela são estreitos de propósito.
pub fn rampa(a: &Ajuste) -> [[u16; 256]; 3] {
    let a = a.limitado();
    let b = a.brilho as f64 / 100.0;
    let k = 1.0 + a.contraste as f64 / 100.0;
    let g = 100.0 / a.gama as f64;
    let mut canal = [0u16; 256];
    for (i, v) in canal.iter_mut().enumerate() {
        let x = i as f64 / 255.0;
        let y = ((x - 0.5) * k + 0.5 + b).clamp(0.0, 1.0).powf(g);
        *v = (y * 65535.0).round() as u16;
    }
    [canal, canal, canal]
}

/// Pura: o nível da vibração digital a partir da saturação da tela. 50 na tela é o padrão do driver; abaixo tira cor
/// até o mínimo, acima satura até o máximo. Numa GTX 1650 o driver diz 0 a 100 com padrão 50; placas antigas dizem 0 a
/// 63 com padrão 0, e aí abaixo de 50 fica no padrão.
pub fn nivel_nvidia(saturacao: i32, minimo: i32, padrao: i32, maximo: i32) -> i32 {
    let s = saturacao.clamp(0, 100) as f64;
    let n = if s <= 50.0 {
        minimo as f64 + (padrao - minimo) as f64 * s / 50.0
    } else {
        padrao as f64 + (maximo - padrao) as f64 * (s - 50.0) / 50.0
    };
    (n.round() as i32).clamp(minimo.min(padrao), maximo.max(padrao))
}

/// Pura: o inverso de `nivel_nvidia`.
pub fn na_escala_da_tela(nivel: i32, minimo: i32, padrao: i32, maximo: i32) -> i32 {
    let n = nivel as f64;
    let s = if n <= padrao as f64 {
        if padrao > minimo {
            (n - minimo as f64) / (padrao - minimo) as f64 * 50.0
        } else {
            50.0
        }
    } else if maximo > padrao {
        50.0 + (n - padrao as f64) / (maximo - padrao) as f64 * 50.0
    } else {
        50.0
    };
    s.round().clamp(0.0, 100.0) as i32
}

/// Pura: da escala da tela para a faixa que o driver AMD disse aceitar. O meio da faixa é o normal do driver
/// (saturação e contraste 100 de 0 a 200, brilho 0 de -100 a 100).
pub fn na_faixa_amd(
    valor_da_tela: i32,
    normal_da_tela: i32,
    amplitude_da_tela: i32,
    minimo: i32,
    maximo: i32,
) -> i32 {
    let meio = (minimo + maximo) as f64 / 2.0;
    let metade = (maximo - minimo) as f64 / 2.0;
    let fracao = (valor_da_tela - normal_da_tela) as f64 / amplitude_da_tela as f64;
    (meio + fracao * metade)
        .round()
        .clamp(minimo as f64, maximo as f64) as i32
}

/// O que estava antes, para devolver. Cada campo só existe se o Otimiza mudou aquilo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Originais {
    /// Rampa de cada monitor, pelo nome do dispositivo (`\\.\DISPLAY1`).
    pub rampas: BTreeMap<String, Vec<u16>>,
    /// Nível da vibração digital de cada saída NVIDIA, pelo índice.
    pub nvidia: BTreeMap<u32, i32>,
    /// Valor de cada cor AMD, por (monitor, cor).
    pub amd: Vec<(usize, String, i32)>,
}

impl Originais {
    pub fn vazio(&self) -> bool {
        self.rampas.is_empty() && self.nvidia.is_empty() && self.amd.is_empty()
    }
}

/// O ajuste salvo de cada jogo (pela chave do catálogo) e se ele liga sozinho quando o jogo abre.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Perfis {
    pub por_jogo: BTreeMap<String, Ajuste>,
    pub so_com_jogo: bool,
}

fn pasta() -> PathBuf {
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("pc-optimizer")
}

fn ler<T: for<'de> Deserialize<'de> + Default>(c: &Path) -> T {
    fs::read_to_string(c)
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

fn gravar<T: Serialize>(c: &Path, v: &T) -> Result<(), String> {
    if let Some(p) = c.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    let tmp = crate::modules::changelog::caminho_temporario(c);
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, c).map_err(|e| e.to_string())
}

pub fn perfis() -> Perfis {
    ler(&pasta().join("cores-perfis.json"))
}

pub fn gravar_perfis(p: &Perfis) -> Result<(), String> {
    gravar(&pasta().join("cores-perfis.json"), p)
}

fn caminho_dos_originais() -> PathBuf {
    pasta().join("cores-originais.json")
}

#[derive(Debug, Clone, Serialize)]
pub struct Estado {
    pub caminho: Caminho,
    pub suporta: Suporta,
    pub aplicado: bool,
    pub perfis: Perfis,
    pub saturacao_atual: Option<i32>,
    pub nota: String,
}

#[cfg(target_os = "windows")]
mod sistema {
    use super::*;
    use std::ffi::c_void;
    use std::sync::{Mutex, OnceLock};

    // ------------------------------------------------------------ rampa de gama do Windows

    fn dc_do_monitor(dispositivo: &str) -> Option<windows::Win32::Graphics::Gdi::HDC> {
        use windows::core::PCWSTR;
        use windows::Win32::Graphics::Gdi::CreateDCW;
        let nome: Vec<u16> = dispositivo
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let dc = unsafe { CreateDCW(PCWSTR::null(), PCWSTR(nome.as_ptr()), PCWSTR::null(), None) };
        (!dc.is_invalid()).then_some(dc)
    }

    fn com_dc<T>(
        dispositivo: &str,
        f: impl FnOnce(windows::Win32::Graphics::Gdi::HDC) -> T,
    ) -> Option<T> {
        let dc = dc_do_monitor(dispositivo)?;
        let r = f(dc);
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::DeleteDC(dc);
        }
        Some(r)
    }

    pub fn ler_rampa(dispositivo: &str) -> Option<Vec<u16>> {
        use windows::Win32::UI::ColorSystem::GetDeviceGammaRamp;
        com_dc(dispositivo, |dc| {
            let mut r = [0u16; 768];
            unsafe { GetDeviceGammaRamp(dc, r.as_mut_ptr() as *mut c_void).as_bool() }
                .then(|| r.to_vec())
        })
        .flatten()
    }

    pub fn escrever_rampa(dispositivo: &str, r: &[u16]) -> bool {
        use windows::Win32::UI::ColorSystem::SetDeviceGammaRamp;
        if r.len() != 768 {
            return false;
        }
        com_dc(dispositivo, |dc| unsafe {
            SetDeviceGammaRamp(dc, r.as_ptr() as *const c_void).as_bool()
        })
        .unwrap_or(false)
    }

    pub fn monitores() -> Vec<String> {
        crate::modules::windows::display::monitores_sem_nomes()
            .into_iter()
            .map(|m| m.dispositivo)
            .collect()
    }

    // ------------------------------------------------------------ vibração digital da NVIDIA

    // Números do `nvapi_QueryInterface` para estas funções, conferidos em duas tabelas independentes (o `FunctionId`
    // do NvAPIWrapper e a lista do jNizM). NÃO estão no cabeçalho público: por isso o interruptor `vibracao_nvidia`.
    const ID_INITIALIZE: u32 = 0x0150_E828;
    const ID_ENUM_DISPLAY: u32 = 0x9ABD_D40D;
    const ID_GET_DVC_EX: u32 = 0x0E45_002D;
    const ID_SET_DVC_EX: u32 = 0x4A82_C2B1;

    /// NV_DISPLAY_DVC_INFO_EX, versão 1: versão, atual, mínimo, máximo, padrão.
    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy)]
    pub struct DvcEx {
        versao: u32,
        pub atual: i32,
        pub minimo: i32,
        pub maximo: i32,
        pub padrao: i32,
    }

    pub const VERSAO_DVC_EX: u32 = std::mem::size_of::<DvcEx>() as u32 | (1 << 16);

    type FnInit = unsafe extern "C" fn() -> i32;
    type FnEnumDisplay = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
    type FnGetDvcEx = unsafe extern "C" fn(*mut c_void, u32, *mut DvcEx) -> i32;
    type FnSetDvcEx = unsafe extern "C" fn(*mut c_void, u32, *mut DvcEx) -> i32;

    pub struct Nvapi {
        enum_display: FnEnumDisplay,
        get: FnGetDvcEx,
        set: FnSetDvcEx,
    }
    unsafe impl Send for Nvapi {}
    unsafe impl Sync for Nvapi {}

    pub fn nvapi() -> Option<&'static Nvapi> {
        static N: OnceLock<Option<Nvapi>> = OnceLock::new();
        N.get_or_init(|| unsafe {
            use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
            let nome: Vec<u16> = "nvapi64.dll\0".encode_utf16().collect();
            let m = LoadLibraryW(nome.as_ptr());
            if m.is_null() {
                return None;
            }
            let q = GetProcAddress(m, c"nvapi_QueryInterface".as_ptr() as *const u8)?;
            let q: unsafe extern "C" fn(u32) -> *mut c_void = std::mem::transmute(q);
            let pegar = |id: u32| {
                let p = q(id);
                (!p.is_null()).then_some(p)
            };
            let init: FnInit = std::mem::transmute(pegar(ID_INITIALIZE)?);
            if init() != 0 {
                return None;
            }
            Some(Nvapi {
                enum_display: std::mem::transmute(pegar(ID_ENUM_DISPLAY)?),
                get: std::mem::transmute(pegar(ID_GET_DVC_EX)?),
                set: std::mem::transmute(pegar(ID_SET_DVC_EX)?),
            })
        })
        .as_ref()
    }

    fn tela_nvidia(n: &Nvapi, i: u32) -> Option<*mut c_void> {
        let mut h: *mut c_void = std::ptr::null_mut();
        (unsafe { (n.enum_display)(i, &mut h) } == 0 && !h.is_null()).then_some(h)
    }

    pub fn ler_dvc(i: u32) -> Option<DvcEx> {
        let n = nvapi()?;
        let h = tela_nvidia(n, i)?;
        let mut d = DvcEx {
            versao: VERSAO_DVC_EX,
            ..Default::default()
        };
        (unsafe { (n.get)(h, 0, &mut d) } == 0).then_some(d)
    }

    pub fn escrever_dvc(i: u32, nivel: i32) -> bool {
        let Some(n) = nvapi() else { return false };
        let Some(h) = tela_nvidia(n, i) else {
            return false;
        };
        let mut d = DvcEx {
            versao: VERSAO_DVC_EX,
            atual: nivel,
            ..Default::default()
        };
        unsafe { (n.set)(h, 0, &mut d) == 0 }
    }

    pub fn telas_nvidia() -> Vec<u32> {
        (0..8u32).filter(|i| ler_dvc(*i).is_some()).collect()
    }

    // ------------------------------------------------------------ aplicar e devolver

    static TRAVA: Mutex<()> = Mutex::new(());

    fn vibracao_ligada() -> bool {
        crate::plataforma::flags::Flags::load().vibracao_nvidia
    }

    /// Pelo que o driver responde agora, não pelo nome da placa: quem lê a vibração digital é NVIDIA, quem abre o
    /// ADLX é AMD, o resto vai pela rampa.
    pub fn caminho() -> Caminho {
        use crate::modules::windows::amddriver::{self, Adlx};
        if vibracao_ligada() && !telas_nvidia().is_empty() {
            Caminho::Nvidia
        } else if amddriver::estado() == Adlx::Disponivel
            && amddriver::cores().map(|c| !c.is_empty()).unwrap_or(false)
        {
            Caminho::Amd
        } else {
            Caminho::Gama
        }
    }

    /// A saturação de agora, na escala da tela, para a tela começar do valor real (quem já usa 100% no painel da
    /// NVIDIA não pode ser jogado para 50 sem pedir).
    pub fn saturacao_atual(c: Caminho) -> Option<i32> {
        match c {
            Caminho::Nvidia => {
                let d = ler_dvc(*telas_nvidia().first()?)?;
                Some(na_escala_da_tela(d.atual, d.minimo, d.padrao, d.maximo))
            }
            Caminho::Amd => {
                let f = crate::modules::windows::amddriver::cores()
                    .ok()?
                    .first()?
                    .saturacao?;
                let meio = (f.minimo + f.maximo) as f64 / 2.0;
                let metade = ((f.maximo - f.minimo) as f64 / 2.0).max(1.0);
                Some(
                    (50.0 + (f.atual as f64 - meio) / metade * 50.0)
                        .round()
                        .clamp(0.0, 100.0) as i32,
                )
            }
            Caminho::Gama => None,
        }
    }

    /// Guarda os originais (se ainda não estão guardados) e aplica. Guardar primeiro: se o Otimiza cair no meio, a
    /// próxima abertura devolve.
    pub fn aplicar(a: Ajuste) -> Result<Caminho, String> {
        let _g = TRAVA.lock().map_err(|_| "as cores travaram".to_string())?;
        let a = a.limitado();
        let c = caminho();
        let mut orig: Originais = ler(&caminho_dos_originais());

        match c {
            Caminho::Amd => {
                use crate::modules::windows::amddriver::{self, Cor};
                let telas = amddriver::cores()?;
                for (m, t) in telas.iter().enumerate() {
                    for (cor, faixa, valor_tela, normal, amplitude) in [
                        (Cor::Saturacao, t.saturacao, a.saturacao, 50, 50),
                        (Cor::Brilho, t.brilho, a.brilho, 0, 40),
                        (Cor::Contraste, t.contraste, a.contraste, 0, 40),
                    ] {
                        let Some(f) = faixa else { continue };
                        let chave = format!("{:?}", cor);
                        if !orig.amd.iter().any(|(mm, cc, _)| *mm == m && *cc == chave) {
                            orig.amd.push((m, chave, f.atual));
                            gravar(&caminho_dos_originais(), &orig)?;
                        }
                        let v = na_faixa_amd(valor_tela, normal, amplitude, f.minimo, f.maximo);
                        amddriver::escrever_cor(m, cor, v)?;
                    }
                    if let Some(f) = t.matiz {
                        let chave = "Matiz".to_string();
                        if !orig.amd.iter().any(|(mm, cc, _)| *mm == m && *cc == chave) {
                            orig.amd.push((m, chave, f.atual));
                            gravar(&caminho_dos_originais(), &orig)?;
                        }
                        // Matiz em graus; a faixa do driver pode ser 0..359 ou -180..180.
                        let v = if f.minimo >= 0 {
                            (a.matiz + 360) % 360
                        } else {
                            a.matiz
                        };
                        amddriver::escrever_cor(m, Cor::Matiz, v)?;
                    }
                }
                // O gama não existe no ADLX de cor: vai pela rampa.
                aplicar_rampa(
                    &Ajuste {
                        brilho: 0,
                        contraste: 0,
                        ..a
                    },
                    &mut orig,
                )?;
            }
            Caminho::Nvidia => {
                for i in telas_nvidia() {
                    let Some(d) = ler_dvc(i) else { continue };
                    if !orig.nvidia.contains_key(&i) {
                        orig.nvidia.insert(i, d.atual);
                        gravar(&caminho_dos_originais(), &orig)?;
                    }
                    if !escrever_dvc(i, nivel_nvidia(a.saturacao, d.minimo, d.padrao, d.maximo)) {
                        return Err("O driver da NVIDIA recusou a vibração digital.".to_string());
                    }
                }
                aplicar_rampa(&a, &mut orig)?;
            }
            Caminho::Gama => aplicar_rampa(&a, &mut orig)?,
        }
        Ok(c)
    }

    fn aplicar_rampa(a: &Ajuste, orig: &mut Originais) -> Result<(), String> {
        let r = rampa(a);
        let plana: Vec<u16> = r.iter().flatten().copied().collect();
        for m in monitores() {
            if !orig.rampas.contains_key(&m) {
                let Some(atual) = ler_rampa(&m) else { continue };
                orig.rampas.insert(m.clone(), atual);
                gravar(&caminho_dos_originais(), orig)?;
            }
            if !escrever_rampa(&m, &plana) {
                return Err("O Windows recusou a rampa de gama deste monitor (alguns monitores e drivers bloqueiam).".to_string());
            }
        }
        Ok(())
    }

    /// Devolve tudo o que está em `cores-originais.json` e apaga o arquivo. Chamada ao fechar o jogo, no botão
    /// "Voltar ao normal" e na abertura do Otimiza.
    pub fn restaurar() -> Result<(), String> {
        let _g = TRAVA.lock().map_err(|_| "as cores travaram".to_string())?;
        let orig: Originais = ler(&caminho_dos_originais());
        if orig.vazio() {
            return Ok(());
        }
        let mut falhas = Vec::new();
        for (m, r) in &orig.rampas {
            if !escrever_rampa(m, r) {
                falhas.push(format!("rampa de {}", m));
            }
        }
        for (i, nivel) in &orig.nvidia {
            if !escrever_dvc(*i, *nivel) {
                falhas.push(format!("vibração da saída {}", i));
            }
        }
        for (m, cor, v) in &orig.amd {
            use crate::modules::windows::amddriver::{self, Cor};
            let c = match cor.as_str() {
                "Saturacao" => Cor::Saturacao,
                "Brilho" => Cor::Brilho,
                "Contraste" => Cor::Contraste,
                "Matiz" => Cor::Matiz,
                _ => continue,
            };
            if amddriver::escrever_cor(*m, c, *v).is_err() {
                falhas.push(format!("{} do monitor {}", cor, m));
            }
        }
        if falhas.is_empty() {
            let _ = fs::remove_file(caminho_dos_originais());
            Ok(())
        } else {
            Err(format!("Não consegui devolver: {}", falhas.join(", ")))
        }
    }

    pub fn aplicado() -> bool {
        !ler::<Originais>(&caminho_dos_originais()).vazio()
    }
}

#[cfg(target_os = "windows")]
pub use sistema::{aplicado, aplicar, caminho, restaurar, saturacao_atual};

#[cfg(target_os = "windows")]
pub fn estado() -> Estado {
    let c = caminho();
    Estado {
        caminho: c,
        suporta: suporta(c),
        aplicado: aplicado(),
        perfis: perfis(),
        saturacao_atual: saturacao_atual(c),
        nota: match c {
            Caminho::Amd => {
                "Pelo driver da sua Radeon: saturação, matiz, brilho e contraste.".to_string()
            }
            Caminho::Nvidia => {
                "Vibração digital pelo driver da NVIDIA, e brilho, contraste e gama pelo Windows."
                    .to_string()
            }
            Caminho::Gama => {
                "Brilho, contraste e gama pelo Windows. Saturação precisa do driver da placa."
                    .to_string()
            }
        },
    }
}

/// Quando o jogo abre: aplica o perfil dele, se a pessoa pediu "só com o jogo aberto".
#[cfg(target_os = "windows")]
pub fn ao_abrir(id_do_jogo: &str) {
    let p = perfis();
    if !p.so_com_jogo {
        return;
    }
    if let Some(a) = p.por_jogo.get(id_do_jogo).filter(|a| !a.e_normal()) {
        if let Err(e) = aplicar(*a) {
            crate::utils::Logger::warn(&format!("cores do jogo {}: {}", id_do_jogo, e));
        }
    }
}

/// Quando o jogo fecha: devolve as cores de antes.
#[cfg(target_os = "windows")]
pub fn ao_fechar() {
    if perfis().so_com_jogo {
        if let Err(e) = restaurar() {
            crate::utils::Logger::warn(&format!("cores ao fechar o jogo: {}", e));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rampa_normal_e_a_identidade() {
        let r = rampa(&NORMAL);
        assert_eq!(r[0][0], 0);
        assert_eq!(r[0][255], 65535);
        assert_eq!(r[0][128], ((128.0 / 255.0) * 65535.0_f64).round() as u16);
    }

    #[test]
    fn rampa_mais_brilho_sobe_e_para_no_teto() {
        let r = rampa(&Ajuste {
            brilho: 40,
            ..NORMAL
        });
        assert!(r[0][128] > rampa(&NORMAL)[0][128]);
        assert_eq!(r[0][255], 65535, "o branco para no teto");
        assert!(
            r[0].windows(2).all(|w| w[0] <= w[1]),
            "a rampa precisa subir sempre"
        );
    }

    #[test]
    fn gama_acima_de_100_clareia_os_escuros() {
        let claro = rampa(&Ajuste {
            gama: 130,
            ..NORMAL
        });
        assert!(
            claro[0][40] > rampa(&NORMAL)[0][40],
            "o competitivo clareia as sombras"
        );
    }

    #[test]
    fn nvidia_segue_a_faixa_do_driver() {
        // GTX 1650 (lido nesta máquina): 0 a 100, padrão 50.
        assert_eq!(nivel_nvidia(50, 0, 50, 100), 50);
        assert_eq!(nivel_nvidia(0, 0, 50, 100), 0);
        assert_eq!(nivel_nvidia(75, 0, 50, 100), 75);
        assert_eq!(nivel_nvidia(100, 0, 50, 100), 100);
        // Placas antigas: 0 a 63, padrão 0. Abaixo do normal não há para onde ir.
        assert_eq!(nivel_nvidia(20, 0, 0, 63), 0);
        assert_eq!(nivel_nvidia(100, 0, 0, 63), 63);
    }

    #[test]
    fn escala_da_tela_e_o_inverso_do_nivel() {
        for s in [0, 10, 50, 75, 100] {
            assert_eq!(
                na_escala_da_tela(nivel_nvidia(s, 0, 50, 100), 0, 50, 100),
                s
            );
        }
        assert_eq!(
            na_escala_da_tela(100, 0, 50, 100),
            100,
            "quem já usa 100% no painel da NVIDIA aparece em 100"
        );
        assert_eq!(
            na_escala_da_tela(0, 0, 0, 63),
            50,
            "padrão das placas antigas é o normal"
        );
    }

    #[test]
    fn amd_mapeia_para_o_meio_da_faixa_do_driver() {
        assert_eq!(na_faixa_amd(50, 50, 50, 0, 200), 100, "saturação normal");
        assert_eq!(na_faixa_amd(100, 50, 50, 0, 200), 200);
        assert_eq!(na_faixa_amd(0, 50, 50, 0, 200), 0);
        assert_eq!(na_faixa_amd(20, 0, 40, -100, 100), 50, "brilho +20 de 40");
        assert_eq!(
            na_faixa_amd(999, 0, 40, -100, 100),
            100,
            "fora da faixa fica no limite"
        );
    }

    #[test]
    fn ajuste_fora_dos_limites_e_apertado() {
        let a = Ajuste {
            saturacao: 300,
            brilho: -99,
            contraste: 99,
            gama: 10,
            matiz: 90,
        }
        .limitado();
        assert_eq!(
            a,
            Ajuste {
                saturacao: 100,
                brilho: -40,
                contraste: 40,
                gama: 70,
                matiz: 30
            }
        );
    }

    #[test]
    fn gama_nao_promete_saturacao() {
        assert!(!suporta(Caminho::Gama).saturacao);
        assert!(!suporta(Caminho::Nvidia).matiz);
        assert!(suporta(Caminho::Amd).matiz);
    }

    #[test]
    fn originais_vazios_nao_devolvem_nada() {
        assert!(Originais::default().vazio());
        let mut o = Originais::default();
        o.nvidia.insert(0, 0);
        assert!(!o.vazio());
    }

    #[test]
    #[ignore = "lê a vibração digital e a rampa desta máquina (só leitura)"]
    #[cfg(target_os = "windows")]
    fn le_as_cores_desta_maquina() {
        println!("versão DVC_EX: {:#x}", sistema::VERSAO_DVC_EX);
        println!("NVAPI: {}", sistema::nvapi().is_some());
        for i in 0..4 {
            println!("saída {}: {:?}", i, sistema::ler_dvc(i));
        }
        for m in sistema::monitores() {
            let r = sistema::ler_rampa(&m);
            println!(
                "{}: rampa {}",
                m,
                r.map(|r| format!("{} {} {}", r[0], r[128], r[255]))
                    .unwrap_or("—".to_string())
            );
        }
    }

    #[test]
    #[ignore = "escreve na vibração digital e na rampa desta máquina O MESMO valor que já está lá (nada muda na tela)"]
    #[cfg(target_os = "windows")]
    fn escreve_o_mesmo_valor_de_volta() {
        for i in sistema::telas_nvidia() {
            let antes = sistema::ler_dvc(i).expect("leu");
            assert!(
                sistema::escrever_dvc(i, antes.atual),
                "o driver aceitou a escrita na saída {}",
                i
            );
            assert_eq!(sistema::ler_dvc(i).expect("leu").atual, antes.atual);
        }
        for m in sistema::monitores() {
            let Some(r) = sistema::ler_rampa(&m) else {
                continue;
            };
            assert!(
                sistema::escrever_rampa(&m, &r),
                "o Windows aceitou a rampa de {}",
                m
            );
        }
    }
}
