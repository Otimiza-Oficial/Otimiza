// O FPS BASE DO FRAME BOOST. O gerador multiplica o ritmo do jogo: base que oscila (60, 75, 52…) vira oscilação
// multiplicada, e no Auto a tela recebe 2 quadros por real num momento e 3 no outro (o que o olho vê como judder). Com a
// base travada numa fração do monitor (60 num de 180 Hz), cada real ganha sempre o mesmo número de gerados.
//
// O teto vai pelo driver, no perfil que o driver USA para o jogo (achado pelo caminho: o FiveM tem perfil próprio), e só
// vale ao reabrir o jogo (`nvdriver::laboratorio_do_limite`). A base vem da última medição de 10 s ou mais do jogo: a do
// Jogo liso (gravada aqui) ou a automática.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::modules::windows::presentmon::Apresentacao;

/// Uma semana: depois disso o jogo, o driver ou a configuração podem ter mudado.
pub const VALIDADE_S: u64 = 7 * 24 * 3600;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseMedida {
    /// FPS do jogo, mediana, medido pelo PresentMon.
    pub mediana: f64,
    pub apresentacao: Apresentacao,
    /// Segundos desde 1970.
    pub quando: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BaseTravada {
    pub fps: u32,
    /// Quadros por real que fecham o monitor (60 em 180 Hz → 3).
    pub multiplicador: u8,
    /// O teto está no perfil que o driver usa E o jogo abriu depois dele: está valendo nesta partida.
    pub valendo: bool,
}

/// Pura: o teto do FPS base para o Frame Boost. A mesma folga para o processador do Jogo liso (abaixo da mediana o
/// jogo segura sempre), na maior fração inteira do monitor que cabe. Sem teto se o jogo já passa dos Hz (aí o Frame
/// Boost não tem o que gerar) ou fica abaixo do jogável.
///
/// Nunca abaixo de `hz / MULTIPLICADOR_MAXIMO_DA_BASE`: no FiveM do dono (10/10), com 48 fps de mediana numa cena
/// pesada, a conta saiu 36 (180/5). Base de 36 é atraso de 28 ms por quadro real, e 4× só enche 144 dos 180 Hz. Com a
/// base baixa assim, o Frame Boost não fica liso de jeito nenhum: melhor não travar e dizer.
pub fn teto_da_base(mediana: f64, hz: u32) -> Option<u32> {
    if hz == 0 || mediana <= 0.0 || mediana >= hz as f64 {
        return None;
    }
    let teto = super::ritmo_estavel::teto_estavel(hz, mediana * super::jogo_liso::FOLGA_DO_PROCESSADOR)?;
    (teto as f64 >= hz as f64 / MULTIPLICADOR_MAXIMO_DA_BASE as f64 - 0.5).then_some(teto)
}

/// Quantos quadros por real o teto da base pode exigir para fechar os Hz (o mesmo teto de `multiplicador_para_o_monitor`).
pub const MULTIPLICADOR_MAXIMO_DA_BASE: u32 = 4;

/// Pura: a medição ainda serve?
pub fn recente(b: &BaseMedida, agora: u64) -> bool {
    b.quando + VALIDADE_S >= agora
}

/// Pura: a medição mostra o jogo solto? A feita depois de um teto que acabou de ser desfeito mediu o teto, não o jogo
/// (no FiveM do dono, 27 fps medidos sob o teto de 36): refazer a conta com ela travaria a base mais baixo ainda.
pub fn livre_do_teto(b: &BaseMedida, teto_desfeito_gravado_em: Option<u64>) -> bool {
    teto_desfeito_gravado_em.map_or(true, |t| b.quando < t)
}

/// Pura: o teto está valendo nesta partida? Gravado no perfil que o driver usa (`no_driver`) antes de o jogo abrir.
pub fn valendo(fps: u32, no_driver: Option<u32>, gravado_em: u64, jogo_abriu_em: Option<u64>) -> bool {
    no_driver == Some(fps) && jogo_abriu_em.is_some_and(|a| a > gravado_em)
}

fn arquivo() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("base-do-jogo.json")
}

fn todas() -> BTreeMap<String, BaseMedida> {
    fs::read_to_string(arquivo())
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

/// Guarda a medição (chamada pelo Jogo liso).
pub fn gravar(executavel: &str, base: BaseMedida) {
    let mut t = todas();
    t.insert(executavel.to_lowercase(), base);
    let p = arquivo();
    if let Some(d) = p.parent() {
        let _ = fs::create_dir_all(d);
    }
    if let Ok(json) = serde_json::to_string_pretty(&t) {
        let _ = fs::write(p, json);
    }
}

fn arquivo_das_soltas() -> PathBuf {
    arquivo().with_file_name("base-solta.json")
}

fn soltas() -> Vec<String> {
    fs::read_to_string(arquivo_das_soltas())
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

/// Pura: a lista com o jogo marcado (ou desmarcado), sem repetir.
pub fn com_a_solta(mut lista: Vec<String>, executavel: &str, solta: bool) -> Vec<String> {
    let nome = executavel.to_lowercase();
    lista.retain(|e| *e != nome);
    if solta {
        lista.push(nome);
    }
    lista
}

/// A pessoa destravou o FPS base deste jogo: o Frame Boost não trava de novo sozinho ao ligar.
pub fn solta(executavel: &str) -> bool {
    soltas().contains(&executavel.to_lowercase())
}

/// Marca (Destravar) ou desmarca (Travar de novo) o jogo.
pub fn soltar(executavel: &str, solta: bool) {
    let p = arquivo_das_soltas();
    if let Some(d) = p.parent() {
        let _ = fs::create_dir_all(d);
    }
    if let Ok(json) = serde_json::to_string_pretty(&com_a_solta(soltas(), executavel, solta)) {
        let _ = fs::write(p, json);
    }
}

/// A medição mais nova de 10 s ou mais deste jogo: a do Jogo liso ou a automática.
pub fn ler(executavel: &str) -> Option<BaseMedida> {
    let do_liso = todas().remove(&executavel.to_lowercase());
    let automatica = crate::modules::medicoes::ler().ok().and_then(|ms| {
        ms.into_iter()
            .rev()
            .filter(|m| m.jogo.eq_ignore_ascii_case(executavel) && m.segundos >= 10.0)
            .find_map(|m| {
                let r = m.presentmon?;
                (r.fps_do_jogo_mediana > 0.0).then_some(BaseMedida {
                    mediana: r.fps_do_jogo_mediana,
                    apresentacao: r.apresentacao,
                    quando: m.quando,
                })
            })
    });
    match (do_liso, automatica) {
        (Some(a), Some(b)) => Some(if a.quando >= b.quando { a } else { b }),
        (a, b) => a.or(b),
    }
}

fn agora() -> u64 {
    crate::modules::changelog::now_timestamp()
}

/// Quando o processo do jogo abriu (segundos desde 1970).
#[cfg(target_os = "windows")]
fn jogo_abriu_em(pid: u32) -> Option<u64> {
    let mut s = sysinfo::System::new();
    let p = sysinfo::Pid::from_u32(pid);
    s.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[p]), true);
    s.process(p).map(|x| x.start_time())
}

/// O teto que o Otimiza gravou para este jogo e quando (do histórico).
#[cfg(target_os = "windows")]
fn gravado(log: &crate::modules::changelog::ChangeLog, executavel: &str) -> Option<(u32, u64)> {
    use crate::modules::changelog::ChangeRecord;
    let id = crate::modules::windows::nvdriver::id_do_limite(executavel);
    log.applied()
        .iter()
        .filter(|e| e.optimization_id == id)
        .find_map(|e| {
            e.changes.iter().find_map(|c| match c {
                ChangeRecord::LimiteNvidia { fps, .. } => Some((*fps, e.timestamp)),
                _ => None,
            })
        })
}

/// O teto do FPS base deste jogo agora, se o Otimiza gravou um.
#[cfg(target_os = "windows")]
pub fn travada(pid: u32, executavel: &str, hz: u32) -> Option<BaseTravada> {
    let log = crate::modules::changelog::ChangeLog::load();
    let (fps, quando) = gravado(&log, executavel)?;
    Some(BaseTravada {
        fps,
        multiplicador: super::ritmo_estavel::multiplicador_para_o_monitor(fps, hz),
        valendo: valendo(
            fps,
            super::jogo_liso::teto_no_driver(executavel),
            quando,
            jogo_abriu_em(pid),
        ),
    })
}

/// Trava o FPS base para o Frame Boost, se ainda não houver um teto que chegue ao jogo. Devolve a frase da tela.
/// Teto que já está no perfil certo (do Jogo liso ou de uma vez anterior) fica como está: refazer a conta com a base
/// já travada baixaria o teto a cada partida.
#[cfg(target_os = "windows")]
pub fn travar(
    executavel: &str,
    hz: u32,
    log: &mut crate::modules::changelog::ChangeLog,
) -> Result<String, String> {
    if crate::modules::windows::nvdriver::estado() != crate::modules::windows::nvdriver::Nvapi::Disponivel {
        return Ok("O FPS base só trava pelo driver da NVIDIA.".to_string());
    }
    if solta(executavel) {
        return Ok("FPS base solto: você destravou. Para o Frame Boost voltar a travar, toque em Travar de novo.".to_string());
    }
    let no_driver = super::jogo_liso::teto_no_driver(executavel);
    let mut nota_de_antes = String::new();
    let mut desfeito_em = None;
    if let Some((fps, quando)) = gravado(log, executavel) {
        // Teto baixo demais para o Frame Boost (o 36 que a regra antiga travou no FiveM do dono): desfaz e refaz a conta.
        if (fps as f64) < hz as f64 / MULTIPLICADOR_MAXIMO_DA_BASE as f64 - 0.5 {
            let id = crate::modules::windows::nvdriver::id_do_limite(executavel);
            crate::modules::windows::WindowsOptimizer::new().revert(&id, log)?;
            crate::utils::Logger::info(&format!(
                "frame boost: teto de {} fps de {} desfeito (abaixo de {} Hz ÷ {})",
                fps, executavel, hz, MULTIPLICADOR_MAXIMO_DA_BASE
            ));
            nota_de_antes = format!("O teto de {} fps de antes foi desfeito (baixo demais para o Frame Boost). ", fps);
            desfeito_em = Some(quando);
        } else if no_driver == Some(fps) {
            return Ok(format!("FPS base travado em {} (já estava).", fps));
        }
    }
    let Some(base) = ler(executavel).filter(|b| recente(b, agora()) && livre_do_teto(b, desfeito_em)) else {
        return Ok(format!(
            "{}Para travar o FPS base, meça o jogo no Jogo liso (20 s) com o jogo reaberto: o Otimiza escolhe o teto pela medição.",
            nota_de_antes
        ));
    };
    let Some(teto) = teto_da_base(base.mediana, hz) else {
        return Ok(nota_de_antes + &if base.mediana >= hz as f64 {
            format!(
                "O jogo entrega {:.0} fps num monitor de {} Hz: já passa do monitor, não há o que gerar.",
                base.mediana, hz
            )
        } else {
            format!(
                "FPS base não travado: o jogo entrega {:.0} fps, e para fechar os {} Hz sem tranco ele precisa segurar pelo menos {} (no máximo {}× por quadro real). Com a base abaixo disso, o Frame Boost oscila.",
                base.mediana,
                hz,
                (hz as f64 / MULTIPLICADOR_MAXIMO_DA_BASE as f64).round(),
                MULTIPLICADOR_MAXIMO_DA_BASE
            )
        });
    };
    let caminho = crate::modules::windows::frames::caminho_do_processo(executavel);
    crate::modules::windows::WindowsOptimizer::new().limitar_fps_nvidia(
        executavel,
        caminho.as_deref(),
        teto,
        log,
    )?;
    crate::utils::Logger::info(&format!(
        "frame boost: FPS base de {} travado em {} (mediana medida {:.0}, {} Hz)",
        executavel, teto, base.mediana, hz
    ));
    Ok(format!(
        "{}FPS base travado em {} ({}× fecham os {} Hz). Vale quando você reabrir o jogo.",
        nota_de_antes,
        teto,
        super::ritmo_estavel::multiplicador_para_o_monitor(teto, hz),
        hz
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teto_da_base_divide_o_monitor() {
        // O FiveM do dono: 89 fps de mediana em tela cheia, 64 sem bordas, monitor de 180 Hz.
        assert_eq!(teto_da_base(89.0, 180), Some(60), "180/3");
        assert_eq!(teto_da_base(64.0, 180), Some(45), "180/4");
        // A cena pesada do FiveM do dono (10/10): 48 de mediana dava 36 (180/5). Base baixa assim não trava.
        assert_eq!(teto_da_base(48.0, 180), None, "36 exigiria 5× e um atraso de 28 ms por real");
        assert_eq!(teto_da_base(50.0, 144), Some(36), "144/4: ainda dentro de 4×");
        assert_eq!(teto_da_base(200.0, 180), None, "passa do monitor: nada a gerar");
        assert_eq!(teto_da_base(60.0, 0), None, "sem Hz, sem conta");
        assert_eq!(teto_da_base(20.0, 180), None, "abaixo do jogável");
    }

    /// Reparo no PC do dono (10/10): desfaz, pelo histórico do próprio Otimiza, o teto baixo que a regra antiga travou no
    /// FiveM (36 fps) e a otimização de janela medida sem efeito, e lê o teto que ficou no perfil que o driver usa.
    /// Com o Otimiza fechado: `cargo test --lib base_do_jogo::tests::desfaz_o_teto_baixo_do_fivem -- --ignored --nocapture`.
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore]
    fn desfaz_o_teto_baixo_do_fivem() {
        use crate::modules::changelog::ChangeLog;
        use crate::modules::windows::{nvdriver, WindowsOptimizer};
        let exe = "FiveM_b3258_GTAProcess.exe";
        let caminho = std::env::var("LOCALAPPDATA").unwrap()
            + r"\FiveM\FiveM.app\data\cache\subprocess\FiveM_b3258_GTAProcess.exe";
        let mut log = ChangeLog::load();
        let id = nvdriver::id_do_limite(exe);
        println!("teto gravado antes: {:?}", gravado(&log, exe));
        if log.is_applied(&id) {
            println!("desfazer teto: {:?}", WindowsOptimizer::new().revert(&id, &mut log).map(|_| "ok"));
        }
        println!(
            "otimização de janela desfeita: {:?}",
            WindowsOptimizer::new().desfazer_janela_do_jogo(&caminho, &mut log)
        );
        println!("teto gravado depois: {:?}", gravado(&ChangeLog::load(), exe));
        println!("perfil que o driver usa: {:?}", nvdriver::perfil_em_uso(&caminho));
    }

    #[test]
    fn destravar_marca_o_jogo_uma_vez_e_travar_desmarca() {
        let l = com_a_solta(vec![], "FiveM_b3258_GTAProcess.exe", true);
        assert_eq!(l, vec!["fivem_b3258_gtaprocess.exe".to_string()]);
        let l = com_a_solta(l, "fivem_b3258_GTAPROCESS.exe", true);
        assert_eq!(l.len(), 1, "não repete");
        let l = com_a_solta(l, "FiveM_b3258_GTAProcess.exe", false);
        assert!(l.is_empty());
    }

    #[test]
    fn medicao_velha_nao_serve() {
        let b = BaseMedida {
            mediana: 80.0,
            apresentacao: Apresentacao::Direta,
            quando: 1_000,
        };
        assert!(recente(&b, 1_000 + VALIDADE_S));
        assert!(!recente(&b, 1_001 + VALIDADE_S));
    }

    #[test]
    fn medicao_sob_o_teto_desfeito_nao_serve() {
        // O caso do dono (10/10): teto de 36 gravado em 1791654844, base de 27 medida em 1791655324.
        let sob_o_teto = BaseMedida {
            mediana: 27.25,
            apresentacao: Apresentacao::Desconhecida,
            quando: 1_791_655_324,
        };
        assert!(!livre_do_teto(&sob_o_teto, Some(1_791_654_844)));
        assert!(livre_do_teto(&sob_o_teto, None), "sem teto desfeito, a medição vale");
        let de_antes = BaseMedida { quando: 1_791_600_000, ..sob_o_teto };
        assert!(livre_do_teto(&de_antes, Some(1_791_654_844)), "medida antes do teto, o jogo estava solto");
    }

    #[test]
    fn so_vale_no_perfil_certo_e_com_o_jogo_aberto_depois() {
        assert!(valendo(60, Some(60), 100, Some(200)));
        assert!(!valendo(60, Some(60), 100, Some(50)), "o jogo já estava aberto quando gravou");
        assert!(!valendo(60, Some(0), 100, Some(200)), "gravado num perfil que o driver não usa");
        assert!(!valendo(60, None, 100, Some(200)), "sem leitura do driver");
    }
}
