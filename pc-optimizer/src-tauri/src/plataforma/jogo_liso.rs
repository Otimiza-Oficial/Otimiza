// JOGO LISO. O botão que resolve o que o cliente SENTE: o jogo travando, mesmo com FPS alto. Mede o jogo, escolhe o teto
// de FPS que o ritmo fica constante (`ritmo_estavel`), aplica pelo driver e prova o resultado com número (trancos/min e
// 1% low, antes × depois). Medido no FiveM do dono: 45 fps de mediana mas 74 trancos/min — o teto em 45 corta a oscilação.
//
// O teto do driver só vale na PRÓXIMA abertura do jogo (medido em `nvdriver::laboratorio_do_limite`): por isso, aplicado
// o teto, o jogo precisa ser reaberto. A prova do "depois" é feita na próxima partida.

use serde::Serialize;

use crate::modules::windows::presentmon::{Apresentacao, GargaloProvavel};

#[derive(Debug, Clone, Serialize)]
pub struct Plano {
    /// FPS que o jogo entrega (mediana medida agora).
    pub fps_agora: f64,
    pub hz: u32,
    /// O teto escolhido, ou `None` se o jogo já roda liso / passa do monitor.
    pub teto: Option<u32>,
    pub multiplicador_do_boost: u8,
    /// Trancos por minuto agora (a dor que o teto ataca).
    pub trancos_por_minuto: f64,
    pub motivo: String,
    /// O teto que o driver USA neste jogo agora (lido pelo caminho do executável aberto); `None` sem leitura.
    pub teto_no_driver: Option<u32>,
    /// Um teto gravado pelo Otimiza que não chega ao jogo.
    pub aviso_do_teto: Option<String>,
    /// Como o quadro chegou à tela na medição, e o que fazer se for o caminho caro.
    pub tela: Option<Tela>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tela {
    pub apresentacao: Apresentacao,
    /// O texto do PresentMon, ex. "Composed: Copy with GPU GDI".
    pub modo: String,
    /// Programas com janela sempre por cima no monitor do jogo.
    pub por_cima: Vec<String>,
    pub aviso: Option<String>,
}

/// Pura: o aviso da tela. Medido no FiveM do dono, no mesmo lugar do mapa: em janela sem bordas o Windows copiava cada
/// quadro ("Composed: Copy with GPU GDI") e o jogo dava 61 fps, com 16,2 ms de processador por quadro; em tela cheia
/// (direto ao monitor) 85 fps, com 11,5 ms. Com o processador no limite, a cópia vira FPS perdido.
pub fn aviso_de_tela(apresentacao: Apresentacao, gargalo: GargaloProvavel, por_cima: &[String]) -> Option<String> {
    let culpados = (!por_cima.is_empty()).then(|| por_cima.join(", "));
    match apresentacao {
        Apresentacao::Copiada => {
            let custo = if gargalo == GargaloProvavel::Cpu {
                " Isso gasta processador, que é o que falta neste jogo: no FiveM medido, 61 fps em janela e 85 em tela cheia."
            } else {
                " Em tela cheia o quadro vai direto ao monitor, com menos atraso."
            };
            let mut t = format!(
                "O jogo está em janela e o Windows copia cada quadro antes de mostrar.{} Use tela cheia na resolução do monitor.",
                custo
            );
            if let Some(c) = culpados {
                t.push_str(&format!(
                    " Antes, feche {}: fica sempre por cima do jogo e, em tela cheia, faz o jogo minimizar sozinho.",
                    c
                ));
            }
            Some(t)
        }
        _ => culpados.map(|c| {
            format!(
                "Se o jogo minimiza sozinho em tela cheia, é {}: fica sempre por cima do jogo. Feche antes de jogar.",
                c
            )
        }),
    }
}

/// Pura: o plano a partir da medição. Separada para testar sem o driver nem o PresentMon.
/// Quanto do FPS mediano o teto usa. Travar NA mediana (o primeiro teste do dono, 60 fps, "não mudou nada") só corta os
/// picos para cima: a CPU segue a 100% metade do tempo e os trancos (scripts, carregamento de cenário) continuam. Abaixo
/// dela sobra ~15% de processador para absorver o pico, e é isso que tira o tranco.
pub const FOLGA_DO_PROCESSADOR: f64 = 0.85;

/// Abaixo disto por minuto o jogo já está liso (a régua do `fluidez`: quadro acima de 2,5× a média).
pub const JA_LISO_TRANCOS: f64 = 2.0;

pub fn planejar(fps_mediana: f64, hz: u32, trancos_por_minuto: f64) -> Plano {
    // Jogo que já passa dos Hz trava no próprio monitor; abaixo deles, com a folga para o processador.
    let base = if hz > 0 && fps_mediana >= hz as f64 {
        fps_mediana
    } else {
        fps_mediana * FOLGA_DO_PROCESSADOR
    };
    // Já liso (medido no dono, com a Energia de jogo: 6 → 0 trancos/min a 80 fps): travar abaixo só tiraria FPS. Acima
    // dos Hz o teto nos Hz continua: quadro que o monitor não mostra é só calor e atraso.
    let ja_liso = trancos_por_minuto < JA_LISO_TRANCOS && !(hz > 0 && fps_mediana >= hz as f64);
    let teto = if ja_liso {
        None
    } else {
        super::ritmo_estavel::teto_estavel(hz, base)
    };
    let multiplicador = teto
        .map(|t| super::ritmo_estavel::multiplicador_para_o_monitor(t, hz))
        .unwrap_or(1);
    let motivo = match teto {
        Some(t) if trancos_por_minuto >= 10.0 => format!(
            "O jogo entrega {:.0} fps mas trava {:.0} vezes por minuto: travar em {} deixa o ritmo constante (o que o olho \
             sente como liso).",
            fps_mediana, trancos_por_minuto, t
        ),
        Some(t) => format!(
            "Travar o jogo em {} deixa o ritmo mais constante num monitor de {} Hz.",
            t, hz
        ),
        None if ja_liso => format!(
            "O jogo já está liso: {:.0} fps e {:.0} trancos por minuto. Travar agora só tiraria FPS — deixe como está.",
            fps_mediana, trancos_por_minuto
        ),
        None => "O jogo já roda num ritmo que fecha o monitor: travar não ajudaria.".to_string(),
    };
    Plano {
        fps_agora: fps_mediana,
        hz,
        teto,
        multiplicador_do_boost: multiplicador,
        trancos_por_minuto,
        motivo,
        teto_no_driver: None,
        aviso_do_teto: None,
        tela: None,
    }
}

/// Pura: o teto que o Otimiza gravou (`gravado`, pelo histórico) chega ao jogo? `no_driver` é o teto do perfil que o
/// driver usa para o executável aberto. Medido no FiveM do dono: 60 gravado num perfil pelo nome, 0 no perfil que o
/// FiveM cria com o próprio caminho, e o jogo a 80 fps — o "Travar em 60" nunca valeu.
pub fn aviso_do_teto(gravado: Option<u32>, no_driver: Option<u32>) -> Option<String> {
    let gravado = gravado?;
    let no_driver = no_driver?;
    (no_driver != gravado).then(|| {
        format!(
            "O teto de {} fps que o Otimiza gravou antes não chegava ao jogo: ele usa outro perfil no driver{}. \
             Travar de novo grava no perfil certo.",
            gravado,
            if no_driver == 0 {
                String::new()
            } else {
                format!(", que trava em {}", no_driver)
            }
        )
    })
}

/// Mede o jogo aberto (segundos de PresentMon) e monta o plano. Não aplica nada.
#[cfg(target_os = "windows")]
pub fn medir(processo: &str, segundos: u32) -> Result<Plano, String> {
    use crate::modules::windows::{frames, presentmon};
    let (pid, nome) = frames::encontrar_processo(processo)
        .ok_or_else(|| format!("Abra o {} e entre na partida antes de medir.", processo))?;
    let (_, r) = presentmon::medir_como_antes(pid, &nome, segundos.clamp(10, 60))?;
    let hz = super::frameboost::hz_do_jogo(pid).unwrap_or(0);
    // A base do Frame Boost: a medição mais confiável do jogo é esta (20 s de PresentMon).
    super::base_do_jogo::gravar(
        processo,
        super::base_do_jogo::BaseMedida {
            mediana: r.fps_do_jogo_mediana,
            apresentacao: r.apresentacao,
            quando: crate::modules::changelog::now_timestamp(),
        },
    );
    let mut plano = planejar(r.fps_do_jogo_mediana, hz, r.engasgos_por_minuto);
    plano.teto_no_driver = teto_no_driver(processo);
    plano.aviso_do_teto = aviso_do_teto(teto_gravado(processo), plano.teto_no_driver);
    let por_cima = super::por_cima::agora(pid, &nome);
    plano.tela = Some(Tela {
        apresentacao: r.apresentacao,
        modo: r.modo_de_apresentacao.clone(),
        aviso: aviso_de_tela(r.apresentacao, r.gargalo, &por_cima),
        por_cima,
    });
    Ok(plano)
}

/// O teto do perfil que o driver usa para o jogo aberto, achado pelo caminho dele. Só NVIDIA; `None` sem leitura.
#[cfg(target_os = "windows")]
pub fn teto_no_driver(executavel: &str) -> Option<u32> {
    let caminho = crate::modules::windows::frames::caminho_do_processo(executavel)?;
    crate::modules::windows::nvdriver::perfil_em_uso(&caminho)
        .ok()
        .map(|p| p.map_or(0, |p| p.teto))
}

/// O teto que o histórico diz que o Otimiza gravou para este executável.
#[cfg(target_os = "windows")]
pub fn teto_gravado(executavel: &str) -> Option<u32> {
    use crate::modules::changelog::{ChangeLog, ChangeRecord};
    let id = crate::modules::windows::nvdriver::id_do_limite(executavel);
    ChangeLog::load()
        .applied()
        .iter()
        .filter(|e| e.optimization_id == id)
        .flat_map(|e| e.changes.iter())
        .find_map(|c| match c {
            ChangeRecord::LimiteNvidia { fps, .. } => Some(*fps),
            _ => None,
        })
}

/// Aplica o teto do plano pelo driver (guardando o anterior, no `ChangeLog`), para a PRÓXIMA abertura do jogo. O
/// executável é o nome do processo aberto (ex. `FiveM_b3258_GTAProcess.exe`).
#[cfg(target_os = "windows")]
pub async fn aplicar(
    executavel: &str,
    plano: &Plano,
    log: &mut crate::modules::changelog::ChangeLog,
) -> Result<(), String> {
    let Some(teto) = plano.teto else {
        return Err(
            "Não há teto a aplicar: o jogo já roda num ritmo que fecha o monitor.".to_string(),
        );
    };
    let caminho = crate::modules::windows::frames::caminho_do_processo(executavel);
    crate::modules::windows::WindowsOptimizer::new().limitar_fps_nvidia(
        executavel,
        caminho.as_deref(),
        teto,
        log,
    )?;
    crate::utils::Logger::info(&format!(
        "jogo liso: teto de {} fps no perfil de {} (vale ao reabrir o jogo)",
        teto, executavel
    ));
    Ok(())
}

/// O jogo está preso no processador? (A última medição automática deste jogo diz que a CPU foi o gargalo.) Nesse caso
/// tirar o limite de FPS do jogo (`configjogo::Perfil::SemTeto`, que o "Otimizar" fazia) só aumenta a oscilação do ritmo:
/// a dor é o tempo de quadro irregular, não um teto baixo. Medido no FiveM do dono (gargalo Cpu, 99% de CPU).
#[cfg(target_os = "windows")]
pub fn preso_no_processador(executavel: &str) -> bool {
    use crate::modules::windows::presentmon::GargaloProvavel;
    let (id, _) = super::historico::id_do_jogo(executavel);
    let Ok(medicoes) = crate::modules::medicoes::ler() else {
        // Sem histórico: o nome do jogo decide (FiveM e GTA V são sempre presos no processador neste tipo de PC).
        return nome_de_jogo_preso(executavel);
    };
    medicoes
        .iter()
        .rev()
        .find(|m| super::historico::id_do_jogo(&m.jogo).0 == id)
        .and_then(|m| m.presentmon.as_ref())
        .map(|r| r.gargalo == GargaloProvavel::Cpu)
        .unwrap_or_else(|| nome_de_jogo_preso(executavel))
}

/// Pura: o nome é de um jogo que, por natureza, pesa no processador (mundo aberto com muitos jogadores).
pub fn nome_de_jogo_preso(executavel: &str) -> bool {
    let e = executavel.to_lowercase();
    ["fivem", "gta5", "gta_v", "rdr2", "cfx"]
        .iter()
        .any(|k| e.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fivem_e_gta_sao_presos_no_processador() {
        assert!(nome_de_jogo_preso("FiveM_b3258_GTAProcess.exe"));
        assert!(nome_de_jogo_preso("GTA5.exe"));
        assert!(!nome_de_jogo_preso("valorant.exe"));
    }

    #[test]
    fn plano_do_fivem_do_dono() {
        // A medição do dono que deu "não mudou nada": mediana 60, e o teto saiu 60 (na média, sem folga). Agora: 45.
        let p = planejar(60.0, 180, 74.0);
        assert_eq!(
            p.teto,
            Some(45),
            "abaixo da mediana, com folga para o processador"
        );
        assert_eq!(p.multiplicador_do_boost, 4);
        assert!(p.motivo.contains("74"));
    }

    #[test]
    fn jogo_ja_liso_nao_perde_fps_para_um_teto() {
        // O print do dono com a Energia de jogo ligada: 80 fps, 0 trancos/min — o cartão sugeria travar em 60.
        let p = planejar(80.0, 180, 0.0);
        assert_eq!(p.teto, None);
        assert!(p.motivo.contains("já está liso"));
        assert_eq!(planejar(68.0, 180, 6.0).teto, Some(45));
    }

    #[test]
    fn teto_gravado_que_nao_chega_ao_jogo_vira_aviso() {
        // O FiveM do dono: 60 gravado pelo nome, 0 no perfil que o driver usa.
        let a = aviso_do_teto(Some(60), Some(0)).expect("aviso");
        assert!(a.contains("60") && a.contains("não chegava"), "{}", a);
        assert_eq!(aviso_do_teto(Some(60), Some(60)), None, "chegou: nada a dizer");
        assert_eq!(aviso_do_teto(None, Some(0)), None, "nada gravado");
        assert_eq!(aviso_do_teto(Some(60), None), None, "sem leitura, sem aviso inventado");
        assert!(aviso_do_teto(Some(60), Some(90)).unwrap().contains("90"));
    }

    #[test]
    fn quadro_copiado_pede_tela_cheia_e_cita_o_que_fica_por_cima() {
        let dock = vec!["dock_64.exe".to_string()];
        let a = aviso_de_tela(Apresentacao::Copiada, GargaloProvavel::Cpu, &dock).unwrap();
        assert!(a.contains("tela cheia") && a.contains("61") && a.contains("85"), "{}", a);
        assert!(a.contains("dock_64.exe"), "{}", a);
        let a = aviso_de_tela(Apresentacao::Copiada, GargaloProvavel::Gpu, &[]).unwrap();
        assert!(!a.contains("61"), "sem o processador no limite, não promete FPS: {}", a);
        assert!(!a.contains("feche"));
    }

    #[test]
    fn direto_na_tela_so_avisa_se_algo_fica_por_cima() {
        assert_eq!(aviso_de_tela(Apresentacao::Direta, GargaloProvavel::Cpu, &[]), None);
        let a = aviso_de_tela(Apresentacao::Direta, GargaloProvavel::Cpu, &["dock_64.exe".to_string()]).unwrap();
        assert!(a.contains("minimiza") && a.contains("dock_64.exe"), "{}", a);
    }

    #[test]
    fn jogo_rapido_nao_recebe_teto() {
        let p = planejar(190.0, 180, 2.0);
        assert_eq!(p.teto, Some(180), "trava nos Hz quando passa deles");
        let p = planejar(300.0, 180, 1.0);
        assert_eq!(p.teto, Some(180));
    }
}
