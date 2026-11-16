// O TETO DO JOGO: um dono só para o limite de FPS que o Otimiza grava no driver NVIDIA. O Frame Boost (a base que ele
// dobra) e o Jogo liso (o ritmo constante) gravavam o MESMO limite (`limite_nvidia:<exe>`), e quem gravava por último
// vencia. Dois defeitos chegaram a cliente por isso:
// - o Jogo liso travou um FiveM de 140 FPS em 72 por cima do que o Frame Boost esperava;
// - o teto da base do Frame Boost ficava no driver depois que ele desligava (pela pessoa ou pela guarda): o jogo voltava
//   preso em 72, sem nenhum quadro gerado, e a pessoa via "o Frame Boost para de funcionar e o jogo não fica liso".
//
// Aqui cada teto tem dono (`Origem`). Com o Frame Boost cuidando do jogo, vale o teto dele e o Jogo liso só mostra o que
// faria. Quando o Frame Boost sai do jogo, o teto dele sai junto, e o teto do Jogo liso que existia antes volta.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origem {
    FrameBoost,
    JogoLiso,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registro {
    pub origem: Origem,
    pub fps: u32,
    /// O teto do Jogo liso que o Frame Boost substituiu: volta quando o Frame Boost sai do jogo.
    #[serde(default)]
    pub antes: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pedido {
    JogoLiso(u32),
    FrameBoost(u32),
    /// O Frame Boost não vai mais cuidar deste jogo (desligou e não volta sozinho).
    FimDoFrameBoost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decisao {
    Gravar(Registro),
    Soltar,
    Recusar(String),
    Nada,
}

/// A frase da recusa: o Jogo liso com o Frame Boost cuidando do jogo.
pub const FRAME_BOOST_CUIDANDO: &str = "O Frame Boost está cuidando do ritmo deste jogo: ele trava o FPS base numa fração \
     do monitor e completa o resto com quadros gerados. Para usar o Jogo liso, desligue o Frame Boost neste jogo.";

/// **Pura.** O que fazer com o teto deste jogo. `atual`: o registro que vale agora (já conferido contra o histórico).
/// `frame_boost_no_jogo`: o Frame Boost está ligado neste jogo ou volta sozinho na próxima partida.
pub fn decidir(atual: Option<&Registro>, pedido: Pedido, frame_boost_no_jogo: bool) -> Decisao {
    match pedido {
        Pedido::JogoLiso(_) if frame_boost_no_jogo => Decisao::Recusar(FRAME_BOOST_CUIDANDO.to_string()),
        Pedido::JogoLiso(fps) => Decisao::Gravar(Registro { origem: Origem::JogoLiso, fps, antes: None }),
        Pedido::FrameBoost(fps) => {
            let antes = match atual {
                Some(r) if r.origem == Origem::JogoLiso => Some(r.fps),
                Some(r) => r.antes,
                None => None,
            };
            Decisao::Gravar(Registro { origem: Origem::FrameBoost, fps, antes })
        }
        Pedido::FimDoFrameBoost => match atual {
            Some(r) if r.origem == Origem::FrameBoost => match r.antes {
                Some(fps) => Decisao::Gravar(Registro { origem: Origem::JogoLiso, fps, antes: None }),
                None => Decisao::Soltar,
            },
            _ => Decisao::Nada,
        },
    }
}

/// **Pura.** Uma receita do Turbo que liga o gerador (Auto ou 2× em diante) e não foi reprovada na prova.
pub fn turbo_gera(p: &super::turbo::Perfil) -> bool {
    p.receita.multiplicador.is_some_and(|m| m != 1) && p.prova != Some(super::turbo::Desfecho::Piorou)
}

fn arquivo() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("pc-optimizer")
        .join("teto-do-jogo.json")
}

fn todos() -> BTreeMap<String, Registro> {
    fs::read_to_string(arquivo())
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .unwrap_or_default()
}

fn guardar(executavel: &str, registro: Option<Registro>) {
    let mut t = todos();
    match registro {
        Some(r) => t.insert(executavel.to_lowercase(), r),
        None => t.remove(&executavel.to_lowercase()),
    };
    let p = arquivo();
    if let Some(d) = p.parent() {
        let _ = fs::create_dir_all(d);
    }
    if let Ok(json) = serde_json::to_string_pretty(&t) {
        let _ = fs::write(p, json);
    }
}

/// A pessoa gravou o teto na mão: ele não é do Frame Boost nem do Jogo liso, e o fechar do jogo não o tira.
pub fn da_pessoa(executavel: &str) {
    guardar(executavel, None);
}

/// O registro deste jogo, se o teto dele ainda está aplicado no histórico. Teto desfeito por fora (Histórico,
/// "Restaurar tudo") apaga o registro: dono de um teto que não existe mais travaria a decisão seguinte.
#[cfg(target_os = "windows")]
pub fn atual(executavel: &str, log: &crate::modules::changelog::ChangeLog) -> Option<Registro> {
    let r = todos().remove(&executavel.to_lowercase())?;
    if log.is_applied(&crate::modules::windows::nvdriver::id_do_limite(executavel)) {
        Some(r)
    } else {
        guardar(executavel, None);
        None
    }
}

/// O Frame Boost está gerando neste jogo AGORA. Só agora: com uma escolha lembrada (ou receita do Turbo) que o traz de
/// volta no "Jogar", o teto do Jogo liso pode ser gravado; quando o Frame Boost voltar, ele o guarda em `antes` e devolve
/// ao sair. Recusar por causa da escolha lembrada prendia a pessoa sem saída (desligar o Frame Boost não a apagava).
#[cfg(target_os = "windows")]
pub fn frame_boost_no_jogo(executavel: &str) -> bool {
    let e = crate::modules::windows::geracao::estado();
    e.processo.eq_ignore_ascii_case(executavel) && super::frameboost::ativo_de(&e).is_some()
}

#[cfg(target_os = "windows")]
fn volta_sozinho(id_do_jogo: &str) -> bool {
    super::fb_por_jogo::escolha(id_do_jogo).is_some() || super::turbo::perfil(id_do_jogo).is_some_and(|p| turbo_gera(&p))
}

/// Executa o pedido pela única porta do teto. Devolve a decisão tomada (a `Recusar` traz a frase para a tela).
/// `caminho`: o do executável, quando o jogo já fechou (o FiveM tem perfil próprio no driver, achado pelo caminho);
/// `None` procura no processo aberto.
#[cfg(target_os = "windows")]
pub fn aplicar(
    executavel: &str,
    caminho: Option<&str>,
    pedido: Pedido,
    log: &mut crate::modules::changelog::ChangeLog,
) -> Result<Decisao, String> {
    let fb = matches!(pedido, Pedido::JogoLiso(_)) && frame_boost_no_jogo(executavel);
    let decisao = decidir(atual(executavel, log).as_ref(), pedido, fb);
    match &decisao {
        Decisao::Gravar(r) => {
            let aberto = caminho.is_none().then(|| crate::modules::windows::frames::caminho_do_processo(executavel)).flatten();
            crate::modules::windows::WindowsOptimizer::new().limitar_fps_nvidia(
                executavel,
                caminho.or(aberto.as_deref()),
                r.fps,
                log,
            )?;
            guardar(executavel, Some(r.clone()));
            crate::utils::Logger::info(&format!(
                "teto do jogo: {} fps em {} pelo {:?}{}",
                r.fps,
                executavel,
                r.origem,
                r.antes.map(|a| format!(" (guardado o {} do Jogo liso)", a)).unwrap_or_default()
            ));
        }
        Decisao::Soltar => {
            let id = crate::modules::windows::nvdriver::id_do_limite(executavel);
            if log.is_applied(&id) {
                crate::modules::windows::WindowsOptimizer::new().revert(&id, log)?;
            }
            guardar(executavel, None);
            crate::utils::Logger::info(&format!(
                "teto do jogo: o teto do Frame Boost saiu de {} junto com ele",
                executavel
            ));
        }
        Decisao::Recusar(_) | Decisao::Nada => {}
    }
    Ok(decisao)
}

/// **Pura.** O teto da base do Frame Boost fica no driver depois que o jogo fecha? Só se a partida abriu pelo "Jogar" e
/// o Frame Boost volta nele (escolha lembrada ou receita do Turbo que gera): o "Jogar" é o único caminho que o religa.
/// Quem abre o jogo direto voltaria preso na base, sem quadro gerado.
pub fn manter_ao_fechar(pelo_jogar: bool, volta_no_jogar: bool) -> bool {
    pelo_jogar && volta_no_jogar
}

/// O jogo fechou: se o teto é do Frame Boost e ele não volta na próxima partida, o teto sai (ou volta o do Jogo liso).
/// Sem isso, o jogo abria da próxima vez preso na base do Frame Boost, sem quadro gerado nenhum.
#[cfg(target_os = "windows")]
pub fn ao_fechar_o_jogo(executavel: &str, pelo_jogar: bool, log: &mut crate::modules::changelog::ChangeLog) {
    let (id, _) = super::historico::id_do_jogo(executavel);
    if manter_ao_fechar(pelo_jogar, volta_sozinho(&id)) {
        return;
    }
    if let Err(e) = aplicar(executavel, None, Pedido::FimDoFrameBoost, log) {
        crate::utils::Logger::warn(&format!("teto do jogo: não soltei o teto do Frame Boost de {}: {}", executavel, e));
    }
}

/// Solta o teto da base do Frame Boost deste jogo (Destravar, Desligar Turbo, prova que piorou), devolvendo o do Jogo
/// liso que existia antes. Teto sem registro (gravado antes da 1.0.5) é revertido como antes; teto do Jogo liso ou da
/// pessoa fica.
#[cfg(target_os = "windows")]
pub fn soltar_base(executavel: &str, log: &mut crate::modules::changelog::ChangeLog) -> Result<(), String> {
    let sem_registro = atual(executavel, log).is_none();
    if aplicar(executavel, None, Pedido::FimDoFrameBoost, log)? == Decisao::Nada && sem_registro {
        let id = crate::modules::windows::nvdriver::id_do_limite(executavel);
        if log.is_applied(&id) {
            crate::modules::windows::WindowsOptimizer::new().revert(&id, log)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(origem: Origem, fps: u32, antes: Option<u32>) -> Registro {
        Registro { origem, fps, antes }
    }

    #[test]
    fn com_o_frame_boost_no_jogo_o_jogo_liso_so_mostra() {
        // O caso de 15/11: o Jogo liso gravando por cima. Agora, com o Frame Boost cuidando do jogo, recusa e diz por quê.
        let fb = r(Origem::FrameBoost, 72, None);
        match decidir(Some(&fb), Pedido::JogoLiso(111), true) {
            Decisao::Recusar(m) => assert!(m.contains("Frame Boost"), "{}", m),
            d => panic!("{:?}", d),
        }
        assert_eq!(
            decidir(None, Pedido::JogoLiso(111), false),
            Decisao::Gravar(r(Origem::JogoLiso, 111, None))
        );
    }

    #[test]
    fn o_frame_boost_guarda_o_teto_do_jogo_liso_e_devolve_ao_sair() {
        let liso = r(Origem::JogoLiso, 111, None);
        let Decisao::Gravar(fb) = decidir(Some(&liso), Pedido::FrameBoost(72), false) else { panic!() };
        assert_eq!(fb, r(Origem::FrameBoost, 72, Some(111)));
        // Recalcular a base mantém o teto de antes guardado.
        let Decisao::Gravar(fb2) = decidir(Some(&fb), Pedido::FrameBoost(48), false) else { panic!() };
        assert_eq!(fb2.antes, Some(111));
        assert_eq!(
            decidir(Some(&fb2), Pedido::FimDoFrameBoost, false),
            Decisao::Gravar(r(Origem::JogoLiso, 111, None))
        );
    }

    #[test]
    fn o_teto_do_frame_boost_sai_junto_com_ele() {
        // O jogo voltava preso em 72 sem quadro gerado: sem teto de antes, o fim do Frame Boost solta o driver.
        let fb = r(Origem::FrameBoost, 72, None);
        assert_eq!(decidir(Some(&fb), Pedido::FimDoFrameBoost, false), Decisao::Soltar);
    }

    #[test]
    fn so_o_jogar_mantem_o_teto_da_base_para_a_proxima_partida() {
        assert!(manter_ao_fechar(true, true));
        assert!(!manter_ao_fechar(false, true), "aberto direto: o Frame Boost não religa, o teto não pode ficar");
        assert!(!manter_ao_fechar(true, false), "nada lembrado: o Frame Boost não volta");
    }

    #[test]
    fn o_fim_do_frame_boost_nunca_mexe_no_teto_dos_outros() {
        let liso = r(Origem::JogoLiso, 111, None);
        assert_eq!(decidir(Some(&liso), Pedido::FimDoFrameBoost, false), Decisao::Nada);
        assert_eq!(decidir(None, Pedido::FimDoFrameBoost, false), Decisao::Nada);
    }

    #[test]
    fn so_receita_do_turbo_que_gera_e_nao_piorou_traz_o_frame_boost_de_volta() {
        use super::super::turbo::{Desfecho, Gargalo, Perfil, Receita};
        let p = |m: Option<u8>, prova: Option<Desfecho>| Perfil {
            receita: Receita {
                gargalo: Gargalo::Processador,
                escala: None,
                multiplicador: m,
                energia: false,
                memoria: false,
                explicacao: String::new(),
            },
            fps_antes: None,
            medido_em: 0,
            prova,
        };
        assert!(turbo_gera(&p(Some(2), None)));
        assert!(turbo_gera(&p(Some(0), None)), "Auto gera");
        assert!(!turbo_gera(&p(Some(1), None)), "só escala");
        assert!(!turbo_gera(&p(None, None)));
        assert!(!turbo_gera(&p(Some(2), Some(Desfecho::Piorou))));
    }
}
