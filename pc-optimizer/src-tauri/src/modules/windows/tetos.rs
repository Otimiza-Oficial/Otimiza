// Limites de FPS escondidos (2.9)
//
// Um jogo preso em 60 num monitor de 144 não está limitado pela placa: está
// limitado por um número escrito em algum lugar que ninguém lembra de ter
// mexido. É o ganho mais barato que existe — devolve o que a máquina já
// entregava — e por isso é a alavanca nº 4 do plano.
//
// Onde o teto pode morar, e o que este módulo olha:
//
//   - perfil GLOBAL do driver NVIDIA: limitador de FPS e V-Sync forçado
//     (lido pela NVAPI, sem escrever — `nvdriver::tetos_no_perfil_global`);
//   - RTSS (RivaTuner) aberto: é o limitador mais comum que vem junto com o
//     MSI Afterburner;
//   - arquivo de configuração de jogo em Unreal: `FrameRateLimit` abaixo da
//     taxa do monitor, ou `bUseVSync=True`.
//
// O Otimiza NUNCA põe teto (regra da 2.9). Aqui ele só encontra e explica.
// Tirar é decisão da pessoa — às vezes o teto foi escolhido de propósito.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo")]
pub enum Teto {
    LimiteGlobalNvidia { fps: u32 },
    VsyncForcadoNvidia,
    Rtss,
    LimiteNoJogo { jogo: String, fps: u32, arquivo: String },
    VsyncNoJogo { jogo: String, arquivo: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct Relatorio {
    pub monitor_hz: Option<u32>,
    pub tetos: Vec<Teto>,
    /// O que não deu para ler (ex.: sem placa NVIDIA não há o que ler lá, e
    /// isso NÃO entra aqui; entra quando havia e a leitura falhou).
    pub lacunas: Vec<String>,
}

/// Lê `FrameRateLimit` e `bUseVSync` de um GameUserSettings.ini. **Pura.**
/// Devolve (limite em FPS, 0 = sem limite; vsync ligado).
pub fn ler_unreal(texto: &str) -> (Option<u32>, bool) {
    let mut limite = None;
    let mut vsync = false;
    for linha in texto.lines() {
        let t = linha.trim();
        if let Some((k, v)) = t.split_once('=') {
            let (k, v) = (k.trim(), v.trim());
            if k.eq_ignore_ascii_case("FrameRateLimit") {
                limite = v.parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0).map(|x| x.round() as u32);
            } else if k.eq_ignore_ascii_case("bUseVSync") {
                vsync = v.eq_ignore_ascii_case("true");
            }
        }
    }
    (limite, vsync)
}

/// Um limite é teto quando fica abaixo da taxa do monitor. Sem saber o
/// monitor, qualquer limite acima de zero conta.
pub fn prende(limite: u32, monitor_hz: Option<u32>) -> bool {
    limite > 0 && monitor_hz.map(|hz| limite + 1 < hz).unwrap_or(true)
}

pub fn procurar() -> Relatorio {
    let monitor_hz = super::display::monitores().iter().map(|m| m.hz_atual).max();
    let mut tetos = Vec::new();
    let mut lacunas = Vec::new();

    if matches!(super::nvdriver::estado(), super::nvdriver::Nvapi::Disponivel) {
        match super::nvdriver::tetos_no_perfil_global() {
            Ok(t) => {
                if prende(t.limite_global_fps, monitor_hz) {
                    tetos.push(Teto::LimiteGlobalNvidia { fps: t.limite_global_fps });
                }
                if t.vsync_forcado {
                    tetos.push(Teto::VsyncForcadoNvidia);
                }
            }
            Err(e) => lacunas.push(format!("Driver NVIDIA: {}", e)),
        }
    }

    if super::frames::encontrar_processo("RTSS").is_some() {
        tetos.push(Teto::Rtss);
    }

    for j in super::jogos::varrer().jogos {
        let Some(exe) = j.executavel else { continue };
        let Some(arquivo) = super::unreal::config_do_jogo(&exe) else { continue };
        let Ok(texto) = std::fs::read_to_string(&arquivo) else { continue };
        let (limite, vsync) = ler_unreal(&texto);
        let arquivo = arquivo.to_string_lossy().to_string();
        if let Some(fps) = limite.filter(|l| prende(*l, monitor_hz)) {
            tetos.push(Teto::LimiteNoJogo { jogo: j.nome.clone(), fps, arquivo: arquivo.clone() });
        }
        if vsync {
            tetos.push(Teto::VsyncNoJogo { jogo: j.nome, arquivo });
        }
    }

    Relatorio { monitor_hz, tetos, lacunas }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_o_limite_do_unreal() {
        assert_eq!(ler_unreal("[x]\nFrameRateLimit=60.000000\nbUseVSync=False\n"), (Some(60), false));
        assert_eq!(ler_unreal("FrameRateLimit=0.000000\r\nbUseVSync=True\r\n"), (Some(0), true));
        assert_eq!(ler_unreal("nada aqui"), (None, false));
    }

    #[test]
    fn so_e_teto_abaixo_do_monitor() {
        assert!(prende(60, Some(144)));
        assert!(!prende(144, Some(144)));
        assert!(!prende(143, Some(144)), "143 num de 144 é o limite recomendado para VRR, não teto");
        assert!(!prende(0, Some(144)));
        assert!(prende(120, None));
    }

    #[test]
    fn vsync_que_prende() {
        use crate::modules::windows::nvdriver::vsync_prende;
        assert!(vsync_prende(0x4781_4940));
        assert!(!vsync_prende(0x6092_5292), "controlado pelo aplicativo não prende");
        assert!(!vsync_prende(0x0841_6747), "desligado não prende");
    }

    #[test]
    #[ignore = "lê o driver e os jogos desta máquina"]
    fn procura_nesta_maquina() {
        println!("{:#?}", procurar());
    }

    #[test]
    fn o_modulo_nunca_escreve_teto() {
        let fonte = include_str!("tetos.rs");
        let producao = fonte.split("#[cfg(test)]").next().unwrap();
        for proibido in ["aplicar_limite", "escrever_valor", "fs::write"] {
            assert!(!producao.contains(proibido), "`{}` em tetos.rs", proibido);
        }
    }
}
