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
    /// Gravado na mão (Boost → limite do driver): nem o Frame Boost nem o Turbo mexem nele.
    Pessoa,
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

/// **Pura.** A frase de quando o Frame Boost quer travar a base de um jogo que a pessoa travou na mão.
pub fn teto_da_pessoa(fps: u32) -> String {
    format!(
        "Você travou este jogo em {} FPS: o Frame Boost não troca o seu teto. Para ele travar a base, desfaça o seu \
         teto pelo Histórico.",
        fps
    )
}

/// **Pura.** O que fazer com o teto deste jogo. `atual`: o registro que vale agora (já conferido contra o histórico).
/// `frame_boost_no_jogo`: o Frame Boost está ligado neste jogo ou volta sozinho na próxima partida.
pub fn decidir(atual: Option<&Registro>, pedido: Pedido, frame_boost_no_jogo: bool) -> Decisao {
    match pedido {
        Pedido::JogoLiso(_) if frame_boost_no_jogo => Decisao::Recusar(FRAME_BOOST_CUIDANDO.to_string()),
        Pedido::JogoLiso(fps) => Decisao::Gravar(Registro { origem: Origem::JogoLiso, fps, antes: None }),
        Pedido::FrameBoost(_) if atual.is_some_and(|r| r.origem == Origem::Pessoa) => {
            Decisao::Recusar(teto_da_pessoa(atual.map_or(0, |r| r.fps)))
        }
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
    crate::utils::arquivo::escrever_json_ou_avisar(&p, &t);
}

/// **Pura.** Quem pôs o teto que o driver usa, em palavras para a tela. `registro`: o do Otimiza para este jogo;
/// `no_driver`: o teto lido no perfil que o driver usa (0 = sem teto).
pub fn quem_pos(registro: Option<&Registro>, no_driver: Option<u32>) -> Option<String> {
    let fps = no_driver.filter(|f| *f > 0)?;
    Some(match registro {
        Some(r) if r.fps == fps && r.origem == Origem::FrameBoost => format!(
            "Teto valendo: {} FPS, posto pelo Frame Boost (a base que ele multiplica). Sai quando o Frame Boost sair do jogo.",
            fps
        ),
        Some(r) if r.fps == fps && r.origem == Origem::Pessoa => {
            format!("Teto valendo: {} FPS, posto por você. Desfaz pelo Histórico.", fps)
        }
        Some(r) if r.fps == fps => format!("Teto valendo: {} FPS, posto pelo Jogo liso. Desfaz pelo Histórico.", fps),
        _ => format!(
            "Teto valendo: {} FPS, posto fora do Jogo liso e do Frame Boost (pelo app da NVIDIA ou por uma \
             versão antiga do Otimiza). Desfaz pelo Histórico ou no app da NVIDIA.",
            fps
        ),
    })
}

/// **Pura.** Um teto gravado por versão antiga (o Jogo liso da 1.0.3, o Frame Boost e o Turbo até a 1.0.5) que tira mais
/// que `PERDA_MAXIMA` de um jogo que roda perto dos Hz: o FiveM de 130 fps em 144 Hz preso em 72. O da pessoa nunca.
/// `fps_antes`: o FPS de costume do jogo ANTES de o teto ser gravado (depois dele, a medição só mede o teto).
///
/// Sem dono (gravado antes da 1.0.5, quando o "Limitar FPS" manual também não deixava dono), só sai o teto que é uma
/// fração dos Hz (metade, um terço, um quarto): é a marca da conta do Otimiza. Um 60 posto à mão num monitor de 144 fica.
pub fn teto_antigo_pesado(teto: u32, fps_antes: Option<f64>, hz: u32, origem: Option<Origem>) -> bool {
    let Some(antes) = fps_antes else { return false };
    let do_otimiza = match origem {
        Some(Origem::Pessoa) => false,
        Some(_) => true,
        None => hz > 0 && (2..=4).any(|d| (hz as f64 / d as f64).round() as u32 == teto),
    };
    do_otimiza
        && hz > 0
        && antes >= hz as f64 * super::base_do_jogo::PERTO_DO_MONITOR
        && (teto as f64) < antes * (1.0 - super::jogo_liso::PERDA_MAXIMA)
}

/// Ao fechar o jogo: desfaz o teto antigo pesado (`teto_antigo_pesado`), para a próxima partida abrir solta. Antes a
/// pessoa precisava achar o "limitado a 72 FPS" no Histórico. Anota o intervalo em que ele valeu: a medição feita
/// nele não vira a base do Frame Boost (senão a conta travaria ainda mais baixo).
#[cfg(target_os = "windows")]
pub fn revisar_teto_antigo(executavel: &str, log: &mut crate::modules::changelog::ChangeLog) {
    let Some((teto, gravado_em)) = super::base_do_jogo::gravado(log, executavel) else {
        return;
    };
    let registro = atual(executavel, log);
    // O Frame Boost guardou o teto do Jogo liso que existia antes dele: desfazer aqui apagaria esse teto da pessoa. Fica
    // para o caminho normal (`FimDoFrameBoost`), que devolve o de antes.
    if registro.as_ref().is_some_and(|r| r.antes.is_some()) {
        return;
    }
    let origem = registro.map(|r| r.origem);
    let hz = crate::modules::windows::display::monitores_sem_nomes()
        .iter()
        .find(|m| m.principal)
        .map(|m| m.hz_atual)
        .unwrap_or(0);
    let (id_do_jogo, _) = super::historico::id_do_jogo(executavel);
    // Só as medições comparáveis e do medidor atual: o canal antigo contava em dobro no FiveM, e partida com gerador
    // de outro programa conta quadro gerado; um "antes" inflado desfaria o teto legítimo da base do Frame Boost.
    let antes: Vec<f64> = crate::modules::medicoes::ler_para_comparar()
        .unwrap_or_default()
        .iter()
        .filter(|m| m.confiavel && m.quando < gravado_em && super::historico::id_do_jogo(&m.jogo).0 == id_do_jogo)
        .map(|m| m.fps)
        .collect();
    let fps_antes = super::jogo_liso::fps_de_costume(&antes);
    if !teto_antigo_pesado(teto, fps_antes, hz, origem) {
        return;
    }
    let id = crate::modules::windows::nvdriver::id_do_limite(executavel);
    match crate::modules::windows::WindowsOptimizer::new().revert(&id, log) {
        Ok(_) => {
            guardar(executavel, None);
            let agora = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            super::base_do_jogo::anotar_teto_desfeito(executavel, gravado_em, agora);
            crate::utils::Logger::info(&format!(
                "teto do jogo: teto antigo de {} fps em {} desfeito (o jogo rodava a {:.0} fps num monitor de {} Hz)",
                teto,
                executavel,
                fps_antes.unwrap_or_default(),
                hz
            ));
        }
        Err(e) => crate::utils::Logger::warn(&format!("teto do jogo: não desfiz o teto antigo de {}: {}", executavel, e)),
    }
}

/// O registro do Otimiza para este jogo, sem conferir no histórico (só para a tela).
pub fn registro(executavel: &str) -> Option<Registro> {
    todos().remove(&executavel.to_lowercase())
}

/// A pessoa gravou o teto na mão: fica com dono `Pessoa`. O fechar do jogo, o Desligar Turbo e o Destravar não o tiram,
/// e o Frame Boost não o troca (antes o registro era apagado, e "sem registro" era tratado como teto antigo do Otimiza).
pub fn da_pessoa(executavel: &str, fps: u32) {
    guardar(executavel, Some(Registro { origem: Origem::Pessoa, fps, antes: None }));
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
    // Só escala não trava a base (o mesmo critério do `travar_base`): o Jogo liso fica livre.
    let so_escala = e.escala.is_some() && !e.automatico && e.multiplicador <= 1;
    e.processo.eq_ignore_ascii_case(executavel) && super::frameboost::ativo_de(&e).is_some() && !so_escala
}

/// O gerador do Otimiza rodou de fato neste jogo nesta partida (pelo menos o mínimo do resumo da sessão).
#[cfg(target_os = "windows")]
fn gerou_nesta_partida(executavel: &str) -> bool {
    let e = crate::modules::windows::geracao::estado();
    e.processo.eq_ignore_ascii_case(executavel) && e.sessao.resumo().is_some()
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
            // A fila de 1 quadro do driver vai junto com a base do Frame Boost (o atraso que ele acrescenta é menor
            // com o jogo sem quadros enfileirados). Falhar aqui não impede o teto: só vai para o log.
            if r.origem == Origem::FrameBoost {
                if let Err(e) = crate::modules::windows::WindowsOptimizer::new().fila_nvidia(
                    executavel,
                    caminho.or(aberto.as_deref()),
                    log,
                ) {
                    crate::utils::Logger::warn(&format!("teto do jogo: não pus a fila de 1 quadro em {}: {}", executavel, e));
                }
            }
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
    // O Frame Boost saiu do jogo: a fila sai junto (devolvida no perfil, que fica).
    if matches!(pedido, Pedido::FimDoFrameBoost) && !matches!(decisao, Decisao::Recusar(_)) {
        soltar_fila(executavel, log);
    }
    Ok(decisao)
}

/// Devolve a fila de 1 quadro deste jogo, se o Otimiza a pôs.
#[cfg(target_os = "windows")]
pub fn soltar_fila(executavel: &str, log: &mut crate::modules::changelog::ChangeLog) {
    let id = crate::modules::windows::nvdriver::id_da_fila(executavel);
    if log.is_applied(&id) {
        if let Err(e) = crate::modules::windows::WindowsOptimizer::new().revert(&id, log) {
            crate::utils::Logger::warn(&format!("teto do jogo: não devolvi a fila de {}: {}", executavel, e));
        }
    }
}

/// **Pura.** O teto da base do Frame Boost fica no driver depois que o jogo fecha? Só se a partida abriu pelo "Jogar", o
/// Frame Boost volta nele (escolha lembrada ou receita do Turbo que gera) E o gerador rodou de fato nesta partida. Quem
/// abre o jogo direto voltaria preso na base, sem quadro gerado; e um "Jogar" em que o Frame Boost não ligou (licença
/// vencida, modo seguro, anticheat) prenderia o jogo na base partida após partida.
pub fn manter_ao_fechar(pelo_jogar: bool, volta_no_jogar: bool, gerou_nesta_partida: bool) -> bool {
    pelo_jogar && volta_no_jogar && gerou_nesta_partida
}

/// O jogo fechou: se o teto é do Frame Boost e ele não volta na próxima partida, o teto sai (ou volta o do Jogo liso).
/// Sem isso, o jogo abria da próxima vez preso na base do Frame Boost, sem quadro gerado nenhum.
#[cfg(target_os = "windows")]
pub fn ao_fechar_o_jogo(executavel: &str, pelo_jogar: bool, log: &mut crate::modules::changelog::ChangeLog) {
    let (id, _) = super::historico::id_do_jogo(executavel);
    if manter_ao_fechar(pelo_jogar, volta_sozinho(&id), gerou_nesta_partida(executavel)) {
        return;
    }
    if let Err(e) = aplicar(executavel, None, Pedido::FimDoFrameBoost, log) {
        crate::utils::Logger::warn(&format!("teto do jogo: não soltei o teto do Frame Boost de {}: {}", executavel, e));
    }
}

/// Solta o teto da base do Frame Boost deste jogo (Destravar, Desligar Turbo, prova que piorou), devolvendo o do Jogo
/// liso que existia antes. Teto do Jogo liso ou da pessoa fica. `legado`: quem chama sabe que um teto SEM registro é da
/// base (gravado antes da 1.0.5): o Turbo, só pela marca dele (`turbo::base_do_turbo`); o Destravar, porque a pessoa
/// pediu.
#[cfg(target_os = "windows")]
pub fn soltar_base(executavel: &str, legado: bool, log: &mut crate::modules::changelog::ChangeLog) -> Result<(), String> {
    let sem_registro = atual(executavel, log).is_none();
    if aplicar(executavel, None, Pedido::FimDoFrameBoost, log)? == Decisao::Nada && sem_registro && legado {
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
    fn teto_antigo_pesado_sai_e_o_da_pessoa_fica() {
        // O FiveM do sócio: 130 fps de costume, monitor de 144 Hz, preso em 72 por versão antiga.
        assert!(teto_antigo_pesado(72, Some(130.0), 144, None), "teto antigo sem registro (1.0.3/1.0.4)");
        assert!(!teto_antigo_pesado(60, Some(130.0), 144, None), "sem dono e fora das frações: posto à mão, fica");
        assert!(teto_antigo_pesado(48, Some(130.0), 144, None), "144/3, a conta do Otimiza");
        assert!(teto_antigo_pesado(72, Some(130.0), 144, Some(Origem::FrameBoost)));
        assert!(!teto_antigo_pesado(72, Some(130.0), 144, Some(Origem::Pessoa)), "o da pessoa nunca");
        assert!(!teto_antigo_pesado(120, Some(130.0), 144, None), "tira menos de 15%: fica");
        assert!(!teto_antigo_pesado(48, Some(70.0), 144, None), "jogo longe dos Hz: é a base do Frame Boost, fica");
        assert!(!teto_antigo_pesado(72, None, 144, None), "sem medição de antes, não chuta");
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
    fn a_tela_diz_quem_pos_o_teto() {
        let fb = r(Origem::FrameBoost, 72, None);
        assert!(quem_pos(Some(&fb), Some(72)).unwrap().contains("Frame Boost"));
        assert!(quem_pos(Some(&r(Origem::JogoLiso, 111, None)), Some(111)).unwrap().contains("Jogo liso"));
        assert!(quem_pos(Some(&fb), Some(60)).unwrap().contains("fora"), "o driver tem outro número: não é nosso");
        assert!(quem_pos(None, Some(72)).unwrap().contains("fora"));
        assert_eq!(quem_pos(Some(&fb), Some(0)), None, "sem teto no driver");
        assert_eq!(quem_pos(None, None), None, "sem leitura, sem frase inventada");
    }

    #[test]
    fn so_o_jogar_mantem_o_teto_da_base_para_a_proxima_partida() {
        assert!(manter_ao_fechar(true, true, true));
        assert!(!manter_ao_fechar(false, true, true), "aberto direto: o Frame Boost não religa, o teto não pode ficar");
        assert!(!manter_ao_fechar(true, false, true), "nada lembrado: o Frame Boost não volta");
        assert!(!manter_ao_fechar(true, true, false), "Jogar em que o Frame Boost não ligou (licença vencida): solta");
    }

    #[test]
    fn o_teto_da_pessoa_ninguem_troca_nem_solta() {
        let p = r(Origem::Pessoa, 90, None);
        match decidir(Some(&p), Pedido::FrameBoost(72), false) {
            Decisao::Recusar(m) => assert!(m.contains("90") && m.contains("Histórico"), "{}", m),
            d => panic!("{:?}", d),
        }
        assert_eq!(decidir(Some(&p), Pedido::FimDoFrameBoost, false), Decisao::Nada);
        assert!(quem_pos(Some(&p), Some(90)).unwrap().contains("por você"));
        // O Jogo liso é a pessoa pedindo de novo: grava.
        assert_eq!(decidir(Some(&p), Pedido::JogoLiso(111), false), Decisao::Gravar(r(Origem::JogoLiso, 111, None)));
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
