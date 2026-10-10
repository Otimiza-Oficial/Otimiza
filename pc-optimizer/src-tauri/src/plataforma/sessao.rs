// O jogo aberto agora, para a tela inicial virar a tela do jogo. Usa a detecção que já existe (`deteccao::procurar`:
// janela cobrindo o monitor, motor 3D, tempo aberto) e acrescenta o perfil, o que a pasta do jogo traz e o que funciona
// nele. Avisa a tela só quando o jogo muda: a tela não fica perguntando.

use serde::Serialize;

use super::compat::{self, Contexto};
use super::jogos::{self, PerfilDoJogo, Sobreposicao};
use super::RecursoAvaliado;

#[derive(Debug, Clone, Serialize)]
pub struct JogoAberto {
    pub pid: u32,
    pub executavel: String,
    pub nome: String,
    pub perfil: PerfilDoJogo,
    pub no_catalogo: bool,
    /// "DLSS Frame Generation", "FSR Frame Generation"... achados na pasta do jogo.
    pub geracao_propria: Vec<&'static str>,
    pub recursos: Vec<RecursoAvaliado>,
    pub frame_boost_auto: Option<RecursoAvaliado>,
}

/// Pura: o contexto de compatibilidade de um jogo neste hardware. `barrada_agora` é o anticheat ABERTO agora
/// (`anticheat::permite`), que vale mais que o catálogo.
pub fn contexto(
    hw: &super::hardware::PerfilDeHardware,
    perfil: &PerfilDoJogo,
    geracao_propria: &[&str],
    barrada_agora: Option<String>,
) -> Contexto {
    let (barrada, aviso) = match (barrada_agora, perfil.sobreposicao) {
        (Some(motivo), _) => (Some(motivo), None),
        (None, Sobreposicao::Barrada(m)) => (Some(m.to_string()), None),
        (None, Sobreposicao::Desaconselhada(m)) => (None, Some(m.to_string())),
        (None, Sobreposicao::Segura) => (None, None),
    };

    Contexto {
        hz_do_monitor: hw.tela_principal().map(|t| t.hz_atual),
        api: perfil.api,
        sobreposicao_barrada: barrada,
        sobreposicao_aviso: aviso,
        jogo_tem_geracao_propria: !geracao_propria.is_empty(),
        jogo_tem_mira_configuravel: perfil.mira_no_jogo,
        smooth_motion_pelo_otimiza: super::flags::Flags::load().smooth_motion_drs,
        ..Contexto::so_hardware(hw.placa.fabricante, hw.placa.familia)
    }
}

pub fn montar(
    pid: u32,
    executavel: String,
    nome_detectado: String,
    caminho: Option<&std::path::Path>,
    hw: &super::hardware::PerfilDeHardware,
    barrada_agora: Option<String>,
) -> JogoAberto {
    let achado = jogos::identificar(&executavel);
    let perfil = achado.cloned().unwrap_or_else(jogos::generico);
    let geracao = caminho.map(jogos::geracao_nos_arquivos).unwrap_or_default();
    let c = contexto(hw, &perfil, &geracao, barrada_agora);
    let fab = hw.placa.fabricante;
    let avaliado = |r: compat::Recurso, s: compat::Suporte| RecursoAvaliado {
        recurso: r,
        nome: r.nome(fab),
        suporte: s,
    };

    let recursos = [
        compat::Recurso::FrameBoostDoJogo,
        compat::Recurso::FrameBoostSmoothMotion,
        compat::Recurso::FrameBoostAfmf,
        compat::Recurso::FrameBoostOtimiza,
        compat::Recurso::BaixaLatenciaDoDriver,
        compat::Recurso::MiraNaTela,
        compat::Recurso::MiraNaConfiguracaoDoJogo,
        compat::Recurso::CoresDoMonitor,
    ]
    .into_iter()
    .map(|r| avaliado(r, compat::avaliar(r, &c)))
    .collect();

    JogoAberto {
        pid,
        nome: if achado.is_some() {
            perfil.nome.to_string()
        } else {
            nome_detectado
        },
        executavel,
        no_catalogo: achado.is_some(),
        frame_boost_auto: compat::melhor_frame_boost(&c).map(|(r, s)| avaliado(r, s)),
        geracao_propria: geracao,
        perfil,
        recursos,
    }
}

/// O que mudou entre duas olhadas. Pura: o laço só emite quando há o que dizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mudanca {
    Nada,
    Abriu(u32),
    Fechou,
    Trocou(u32),
}

pub fn comparar(antes: Option<u32>, agora: Option<u32>) -> Mudanca {
    match (antes, agora) {
        (None, None) => Mudanca::Nada,
        (Some(a), Some(b)) if a == b => Mudanca::Nada,
        (None, Some(b)) => Mudanca::Abriu(b),
        (Some(_), None) => Mudanca::Fechou,
        (Some(_), Some(b)) => Mudanca::Trocou(b),
    }
}

/// O processo ainda existe? (Fechou de verdade, e não só saiu da frente.)
#[cfg(target_os = "windows")]
fn processo_vivo(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessesToUpdate, System};
    let mut s = System::new();
    let alvo = Pid::from_u32(pid);
    s.refresh_processes(ProcessesToUpdate::Some(&[alvo]), true);
    s.process(alvo).is_some()
}

/// Pura: entre os processos do catálogo com janela, o de janela maior (o FiveM tem vários `FiveM_*`; só o que desenha
/// o jogo tem janela grande).
pub fn escolher_do_catalogo(candidatos: &[(u32, String, i64)]) -> Option<(u32, String)> {
    candidatos
        .iter()
        .filter(|(_, exe, area)| *area > 320 * 200 && jogos::identificar(exe).is_some())
        .max_by_key(|(_, _, area)| *area)
        .map(|(pid, exe, _)| (*pid, exe.clone()))
}

/// Um jogo do catálogo rodando com janela visível, mesmo sem estar na frente.
#[cfg(target_os = "windows")]
fn do_catalogo_aberto() -> Option<crate::modules::windows::deteccao::JogoDetectado> {
    use crate::modules::windows::geracao::{area_do_cliente, janela_do_processo};
    use sysinfo::{ProcessesToUpdate, System};
    let mut s = System::new();
    s.refresh_processes(ProcessesToUpdate::All, true);
    let mut caminhos = std::collections::HashMap::new();
    let candidatos: Vec<(u32, String, i64)> = s
        .processes()
        .iter()
        .filter_map(|(pid, p)| {
            let exe = p.name().to_string_lossy().to_string();
            jogos::identificar(&exe)?;
            let pid = pid.as_u32();
            let area = janela_do_processo(pid).and_then(area_do_cliente)?;
            caminhos.insert(pid, p.exe().map(|e| e.to_path_buf()));
            Some((pid, exe, area.largura as i64 * area.altura as i64))
        })
        .collect();
    let (pid, executavel) = escolher_do_catalogo(&candidatos)?;
    Some(crate::modules::windows::deteccao::JogoDetectado {
        pid,
        nome: jogos::identificar(&executavel)
            .map(|p| p.nome.to_string())
            .unwrap_or_default(),
        caminho: caminhos.remove(&pid).flatten(),
        executavel,
        conhecido: true,
        sinais: Vec::new(),
    })
}

/// Só na compilação de desenvolvimento: `OTIMIZA_JOGO_DE_TESTE=jogo_de_teste` faz o `examples/jogo_de_teste` contar como
/// jogo aberto (ele usa 1% do 3D, abaixo do mínimo da detecção). O instalador nunca tem isto.
#[cfg(target_os = "windows")]
fn jogo_de_teste() -> Option<crate::modules::windows::deteccao::JogoDetectado> {
    #[cfg(debug_assertions)]
    {
        let nome = std::env::var("OTIMIZA_JOGO_DE_TESTE").ok()?;
        let (pid, executavel) = crate::modules::windows::frames::encontrar_processo(&nome)?;
        Some(crate::modules::windows::deteccao::JogoDetectado {
            pid,
            nome: "Jogo de teste".to_string(),
            executavel,
            caminho: None,
            conhecido: false,
            sinais: Vec::new(),
        })
    }
    #[cfg(not(debug_assertions))]
    None
}

/// Uma volta do laço: `Some(novo)` só quando o jogo mudou (abriu, fechou ou trocou). Montar lê a pasta do jogo e o
/// anticheat, então só se monta na mudança.
#[cfg(target_os = "windows")]
pub fn volta(pid_anterior: Option<u32>) -> Option<Option<JogoAberto>> {
    use crate::modules::windows::{anticheat, deteccao};

    let d = deteccao::procurar().or_else(jogo_de_teste);
    // O jogo continua aberto enquanto o processo dele existir, mesmo atrás do Otimiza: a detecção só olha o primeiro
    // plano, e o Alt+Tab para clicar em Frame Boost "fechava" o jogo (o botão ficava cinza). Sem jogo nenhum, um do
    // catálogo rodando com janela conta também (o Otimiza pode ter aberto depois do jogo).
    let d = match (pid_anterior, d) {
        (_, Some(d)) => Some(d),
        (Some(p), None) if processo_vivo(p) => return None,
        (Some(_), None) => None,
        (None, None) => do_catalogo_aberto(),
    };
    match comparar(pid_anterior, d.as_ref().map(|d| d.pid)) {
        Mudanca::Nada => None,
        Mudanca::Fechou => {
            guardar(None);
            Some(None)
        }
        Mudanca::Abriu(_) | Mudanca::Trocou(_) => {
            let d = d?;
            let barrada = anticheat::permite(
                anticheat::Acao::SobreporJanela,
                &anticheat::detectar_agora(),
            )
            .motivo()
            .map(str::to_string);
            let j = montar(
                d.pid,
                d.executavel,
                d.nome,
                d.caminho.as_deref(),
                &super::hardware::perfil(),
                barrada,
            );
            guardar(Some(j.clone()));
            Some(Some(j))
        }
    }
}

static ATUAL: std::sync::Mutex<Option<JogoAberto>> = std::sync::Mutex::new(None);

pub fn atual() -> Option<JogoAberto> {
    ATUAL.lock().ok().and_then(|g| g.clone())
}

pub fn guardar(j: Option<JogoAberto>) {
    if let Ok(mut g) = ATUAL.lock() {
        *g = j;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Com um jogo do catálogo aberto atrás de outras janelas: `cargo test --lib acha_o_jogo_aberto_agora -- --ignored --nocapture`.
    #[test]
    #[ignore]
    #[cfg(target_os = "windows")]
    fn acha_o_jogo_aberto_agora() {
        let primeiro = volta(None);
        println!(
            "sem jogo antes: {:?}",
            primeiro
                .as_ref()
                .map(|j| j.as_ref().map(|j| (&j.nome, j.pid, &j.executavel)))
        );
        let pid = primeiro.flatten().map(|j| j.pid);
        println!(
            "na volta seguinte (nada muda enquanto o processo existir): {:?}",
            volta(pid).is_none()
        );
    }

    #[test]
    fn do_catalogo_pega_o_processo_do_fivem_que_desenha() {
        let c = vec![
            (10, "FiveM_ChromeBrowser".to_string(), 0),
            (11, "FiveM_b3258_GTAProcess.exe".to_string(), 1920 * 1080),
            (12, "FiveM_ROSLauncher".to_string(), 400 * 300),
            (13, "Discord.exe".to_string(), 1280 * 720),
        ];
        assert_eq!(
            escolher_do_catalogo(&c),
            Some((11, "FiveM_b3258_GTAProcess.exe".to_string()))
        );
        assert_eq!(
            escolher_do_catalogo(&[(13, "Discord.exe".to_string(), 1280 * 720)]),
            None,
            "fora do catálogo não"
        );
        assert_eq!(
            escolher_do_catalogo(&[(14, "cs2.exe".to_string(), 100 * 100)]),
            None,
            "janela minúscula não"
        );
    }
    use crate::plataforma::hardware::{Fabricante, Familia, PerfilDeHardware, Placa, Tela, Vrr};

    fn hw(f: Fabricante, fam: Familia) -> PerfilDeHardware {
        PerfilDeHardware {
            processador: "x".to_string(),
            nucleos_logicos: 8,
            memoria_gb: 16.0,
            disco_do_sistema: "SSD".to_string(),
            placa: Placa {
                nome: "x".to_string(),
                fabricante: f,
                familia: fam,
                driver: None,
            },
            telas: vec![Tela {
                nome: "m".to_string(),
                dispositivo: "d".to_string(),
                principal: true,
                largura: 1920,
                altura: 1080,
                hz_atual: 144,
                hz_maximo: 144,
                vrr: Vrr::Anunciado,
            }],
            windows_build: None,
            lido_em: 0,
        }
    }

    #[test]
    fn fivem_em_radeon_escolhe_afmf_e_avisa_da_mira() {
        let j = montar(
            1,
            "FiveM_b3258_GTAProcess.exe".to_string(),
            "x".to_string(),
            None,
            &hw(Fabricante::Amd, Familia::Rx6000),
            None,
        );
        assert_eq!(j.nome, "FiveM");
        assert!(j.no_catalogo);
        assert_eq!(
            j.frame_boost_auto.unwrap().recurso,
            compat::Recurso::FrameBoostAfmf
        );
        let mira = j
            .recursos
            .iter()
            .find(|r| r.recurso == compat::Recurso::MiraNaTela)
            .unwrap();
        assert!(
            matches!(mira.suporte, compat::Suporte::Parcial(_)),
            "servidor de RP pode proibir"
        );
    }

    #[test]
    fn valorant_nunca_recebe_janela_por_cima() {
        let j = montar(
            1,
            "VALORANT-Win64-Shipping.exe".to_string(),
            "x".to_string(),
            None,
            &hw(Fabricante::Nvidia, Familia::Rtx30),
            None,
        );
        for r in &j.recursos {
            if matches!(
                r.recurso,
                compat::Recurso::MiraNaTela | compat::Recurso::FrameBoostOtimiza
            ) {
                assert!(
                    matches!(r.suporte, compat::Suporte::NaoSuportado(_)),
                    "{:?}",
                    r.recurso
                );
            }
        }
        assert!(
            j.frame_boost_auto.is_none(),
            "RTX 30 no Valorant: nada de Frame Boost"
        );
    }

    #[test]
    fn anticheat_aberto_agora_vence_o_catalogo() {
        let perfil = jogos::identificar("cs2.exe").unwrap();
        let c = contexto(
            &hw(Fabricante::Nvidia, Familia::Rtx40),
            perfil,
            &[],
            Some("FACEIT aberto".to_string()),
        );
        assert_eq!(c.sobreposicao_barrada.as_deref(), Some("FACEIT aberto"));
    }

    #[test]
    fn geracao_da_pasta_vira_frame_boost_do_jogo() {
        let perfil = jogos::identificar("cyberpunk2077.exe").unwrap();
        let c = contexto(
            &hw(Fabricante::Nvidia, Familia::Rtx40),
            perfil,
            &["DLSS Frame Generation"],
            None,
        );
        assert_eq!(
            compat::melhor_frame_boost(&c).unwrap().0,
            compat::Recurso::FrameBoostDoJogo
        );
    }

    #[test]
    fn jogo_fora_do_catalogo_guarda_o_nome_detectado() {
        let j = montar(
            7,
            "jogoindie.exe".to_string(),
            "Jogo Indie".to_string(),
            None,
            &hw(Fabricante::Intel, Familia::Arc),
            None,
        );
        assert_eq!(j.nome, "Jogo Indie");
        assert!(!j.no_catalogo);
    }

    #[test]
    fn so_avisa_quando_o_jogo_muda() {
        assert_eq!(comparar(None, None), Mudanca::Nada);
        assert_eq!(comparar(Some(5), Some(5)), Mudanca::Nada);
        assert_eq!(comparar(None, Some(5)), Mudanca::Abriu(5));
        assert_eq!(comparar(Some(5), None), Mudanca::Fechou);
        assert_eq!(comparar(Some(5), Some(9)), Mudanca::Trocou(9));
    }
}

#[cfg(test)]
mod tests_json {
    use super::*;
    use crate::plataforma::hardware::{Fabricante, Familia, PerfilDeHardware, Placa};

    #[test]
    fn jogo_aberto_vira_json_para_a_tela() {
        let hw = PerfilDeHardware {
            processador: "x".to_string(),
            nucleos_logicos: 8,
            memoria_gb: 8.0,
            disco_do_sistema: "SSD".to_string(),
            placa: Placa {
                nome: "GTX 1650".to_string(),
                fabricante: Fabricante::Nvidia,
                familia: Familia::GtxOuAnterior,
                driver: None,
            },
            telas: Vec::new(),
            windows_build: None,
            lido_em: 0,
        };
        for exe in ["jogo_de_teste.exe", "FiveM_b3258_GTAProcess.exe"] {
            let j = montar(
                1,
                exe.to_string(),
                "Jogo de teste".to_string(),
                None,
                &hw,
                None,
            );
            let json = serde_json::to_string(&Some(j)).expect("a sessão precisa virar JSON");
            assert!(json.contains("\"pid\":1"), "{}", json);
        }
    }
}
