// JOGO LISO. O botão que resolve o que o cliente SENTE: o jogo travando, mesmo com FPS alto. Mede o jogo, escolhe o teto
// de FPS que o ritmo fica constante (`ritmo_estavel`), aplica pelo driver e prova o resultado com número (trancos/min e
// 1% low, antes × depois). Medido no FiveM do dono: 45 fps de mediana mas 74 trancos/min — o teto em 45 corta a oscilação.
//
// O teto do driver só vale na PRÓXIMA abertura do jogo (medido em `nvdriver::laboratorio_do_limite`): por isso, aplicado
// o teto, o jogo precisa ser reaberto. A prova do "depois" é feita na próxima partida.

use serde::Serialize;

use crate::modules::windows::presentmon::{Apresentacao, GargaloProvavel};

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "PlanoJogoLiso"))]
pub struct Plano {
    /// FPS que o jogo entrega (mediana medida agora).
    pub fps_agora: f64,
    /// O FPS de costume deste jogo nesta máquina (mediana das últimas medições automáticas); `None` com poucas.
    pub fps_tipico: Option<f64>,
    /// Quanto o teto tira do FPS de referência (o maior entre o de agora e o de costume), em %. 0 sem teto.
    pub perda_pct: u32,
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
    /// Quem pôs o teto que o driver usa agora (Frame Boost, Jogo liso ou fora do Otimiza), em palavras.
    pub dono_do_teto: Option<String>,
    /// Como o quadro chegou à tela na medição, e o que fazer se for o caminho caro.
    pub tela: Option<Tela>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "TelaJogoLiso"))]
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

/// O máximo que o teto pode tirar do FPS de referência. Num cliente (GTX 1650, 144 Hz) a fração do monitor abaixo de
/// 85% da mediana era 72, metade do monitor, e o FiveM, que costuma rodar de 105 a 160, foi travado em 72: "piorou
/// muito, o frame tá dropando". A fração do monitor só vale quando cabe nesta régua.
pub const PERDA_MAXIMA: f64 = 0.15;

/// Medições automáticas que contam para o FPS de costume (as mais recentes), e o mínimo para confiar nelas.
pub const MEDICOES_DO_COSTUME: usize = 10;
pub const MINIMO_PARA_COSTUME: usize = 3;

/// **Pura.** O FPS de costume: a mediana das medições (as mais recentes no fim). `None` com menos de
/// `MINIMO_PARA_COSTUME`.
pub fn fps_de_costume(fps: &[f64]) -> Option<f64> {
    let mut v: Vec<f64> = fps
        .iter()
        .rev()
        .filter(|x| x.is_finite() && **x > 0.0)
        .take(MEDICOES_DO_COSTUME)
        .copied()
        .collect();
    if v.len() < MINIMO_PARA_COSTUME {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    Some(if v.len() % 2 == 0 { (v[m - 1] + v[m]) / 2.0 } else { v[m] })
}

/// Quanto abaixo dos Hz o teto fica num monitor de taxa variável (o limite que a NVIDIA e a AMD recomendam).
pub const ABAIXO_DOS_HZ_COM_VRR: u32 = 3;

/// **Pura.** Num monitor com G-Sync/FreeSync, o teto nos próprios Hz faz o jogo bater no topo e sair da taxa variável
/// (vira sincronia vertical com atraso, ou rasgo): o teto fica 3 abaixo (141 num de 144). Abaixo dos Hz nada muda.
pub fn com_vrr(mut plano: Plano, vrr: bool) -> Plano {
    if !vrr || plano.hz < 60 || plano.teto != Some(plano.hz) {
        return plano;
    }
    let novo = plano.hz - ABAIXO_DOS_HZ_COM_VRR;
    plano.teto = Some(novo);
    plano.motivo = format!(
        "O jogo passa dos {} Hz do monitor: travar em {} (3 abaixo dos Hz, para o G-Sync/FreeSync continuar valendo) não \
         tira nada que você veja, e tira o calor e o atraso dos quadros que o monitor joga fora.",
        plano.hz, novo
    );
    plano
}

/// Abaixo disto por minuto o jogo já está liso (a régua do `fluidez`: quadro acima de 2,5× a média).
pub const JA_LISO_TRANCOS: f64 = 2.0;

/// `fps_tipico`: o FPS de costume do jogo (`fps_de_costume`). Uma medição de 20 s num momento ruim (cidade cheia,
/// cenário carregando) não pode virar o teto da noite inteira: a referência é o maior dos dois.
pub fn planejar(fps_mediana: f64, fps_tipico: Option<f64>, hz: u32, trancos_por_minuto: f64) -> Plano {
    let referencia = fps_tipico.map_or(fps_mediana, |t| t.max(fps_mediana));
    let passa_do_monitor = hz > 0 && referencia >= hz as f64;
    // Jogo que já passa dos Hz trava no próprio monitor; abaixo deles, com a folga para o processador.
    let base = if passa_do_monitor {
        referencia
    } else {
        referencia * FOLGA_DO_PROCESSADOR
    };
    // Já liso (medido no dono, com a Energia de jogo: 6 → 0 trancos/min a 80 fps): travar abaixo só tiraria FPS. Acima
    // dos Hz o teto nos Hz continua: quadro que o monitor não mostra é só calor e atraso.
    let ja_liso = trancos_por_minuto < JA_LISO_TRANCOS && !passa_do_monitor;
    let piso = (referencia * (1.0 - PERDA_MAXIMA)).floor();
    let teto = if ja_liso || base < 24.0 {
        None
    } else if passa_do_monitor {
        super::ritmo_estavel::teto_estavel(hz, base)
    } else {
        // A fração do monitor (o ritmo que fecha com os Hz) só se não tirar mais que `PERDA_MAXIMA`; senão, a folga.
        let fracao = super::ritmo_estavel::teto_estavel(hz, base).filter(|f| *f as f64 >= piso);
        Some(fracao.unwrap_or_else(|| base.round() as u32))
    };
    let perda_pct = teto
        .map(|t| ((1.0 - t as f64 / referencia) * 100.0).round().max(0.0) as u32)
        .unwrap_or(0);
    let multiplicador = teto
        .map(|t| super::ritmo_estavel::multiplicador_para_o_monitor(t, hz))
        .unwrap_or(1);
    let motivo = match teto {
        Some(t) if passa_do_monitor => format!(
            "O jogo passa dos {} Hz do monitor: travar em {} não tira nada que você veja, e tira o calor e o atraso dos \
             quadros que o monitor joga fora.",
            hz, t
        ),
        Some(t) if trancos_por_minuto >= 10.0 => format!(
            "O jogo entrega {:.0} fps mas trava {:.0} vezes por minuto: travar em {} deixa o ritmo constante (o que o olho \
             sente como liso), tirando no máximo {}% do FPS nos melhores momentos.",
            referencia, trancos_por_minuto, t, perda_pct
        ),
        Some(t) => format!(
            "Travar o jogo em {} deixa o ritmo mais constante, tirando no máximo {}% do FPS nos melhores momentos.",
            t, perda_pct
        ),
        None if ja_liso => format!(
            "O jogo já está liso: {:.0} fps e {:.0} trancos por minuto. Travar agora só tiraria FPS — deixe como está.",
            fps_mediana, trancos_por_minuto
        ),
        None => "O jogo já roda num ritmo que fecha o monitor: travar não ajudaria.".to_string(),
    };
    Plano {
        fps_agora: fps_mediana,
        fps_tipico,
        perda_pct,
        hz,
        teto,
        multiplicador_do_boost: multiplicador,
        trancos_por_minuto,
        motivo,
        teto_no_driver: None,
        aviso_do_teto: None,
        dono_do_teto: None,
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
    let vrr = crate::modules::windows::vrr::monitores().iter().any(|m| {
        m.max_hz.is_some_and(|max| max.abs_diff(hz) <= 2)
            && matches!(m.vrr, crate::modules::windows::vrr::Vrr::Anunciado | crate::modules::windows::vrr::Vrr::Provavel)
    });
    let mut plano = com_vrr(planejar(r.fps_do_jogo_mediana, fps_tipico(processo), hz, r.engasgos_por_minuto), vrr);
    // Com o Frame Boost cuidando do jogo, o teto é dele (`teto_do_jogo`): a tela mostra a medição e diz por que não trava.
    if plano.teto.is_some() && super::teto_do_jogo::frame_boost_no_jogo(processo) {
        plano.teto = None;
        plano.perda_pct = 0;
        plano.motivo = super::teto_do_jogo::FRAME_BOOST_CUIDANDO.to_string();
    }
    plano.teto_no_driver = teto_no_driver(processo);
    plano.aviso_do_teto = aviso_do_teto(teto_gravado(processo), plano.teto_no_driver);
    plano.dono_do_teto = super::teto_do_jogo::quem_pos(
        super::teto_do_jogo::registro(processo).as_ref(),
        plano.teto_no_driver,
    );
    let por_cima = super::por_cima::agora(pid, &nome);
    plano.tela = Some(Tela {
        apresentacao: r.apresentacao,
        modo: r.modo_de_apresentacao.clone(),
        aviso: aviso_de_tela(r.apresentacao, r.gargalo, &por_cima),
        por_cima,
    });
    Ok(plano)
}

/// O FPS de costume deste jogo, das medições automáticas (todas as versões do FiveM contam como o mesmo jogo).
#[cfg(target_os = "windows")]
pub fn fps_tipico(executavel: &str) -> Option<f64> {
    let (id, _) = super::historico::id_do_jogo(executavel);
    // Só as comparáveis e do medidor atual: o canal antigo contava em dobro no FiveM, e partida com gerador de outro
    // programa conta quadro gerado como FPS do jogo.
    let medicoes = crate::modules::medicoes::ler_para_comparar().ok()?;
    let fps: Vec<f64> = medicoes
        .iter()
        .filter(|m| m.confiavel && super::historico::id_do_jogo(&m.jogo).0 == id)
        .map(|m| m.fps)
        .collect();
    fps_de_costume(&fps)
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
    use crate::modules::changelog::ChangeRecord;
    let id = crate::modules::windows::nvdriver::id_do_limite(executavel);
    crate::modules::changelog::foto()
        .applied()
        .iter()
        .filter(|e| e.optimization_id == id)
        .flat_map(|e| e.changes.iter())
        .find_map(|c| match c {
            ChangeRecord::LimiteNvidia { fps, .. } => Some(*fps),
            _ => None,
        })
}

/// O teto gravado depois da partida (cartão da partida, "Deixar liso"): o que vai para a tela.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
pub struct LisoGravado {
    pub teto: Option<u32>,
    pub motivo: String,
}

/// A taxa do monitor principal, da ficha de hardware (o jogo já fechou: não há processo para perguntar).
pub fn hz_principal() -> u32 {
    let p = super::hardware::perfil();
    p.telas
        .iter()
        .find(|t| t.principal)
        .or(p.telas.first())
        .map(|t| t.hz_atual)
        .unwrap_or(0)
}

/// Plano de Partida de respeito, fase 3: o teto calculado pelo FPS e pelos trancos da partida que acabou, gravado no
/// perfil do jogo no driver para a próxima abertura. Partida já lisa não ganha teto (`planejar` decide).
#[cfg(target_os = "windows")]
pub fn gravar_depois_da_partida(
    executavel: &str,
    caminho: Option<&str>,
    fps: f64,
    trancos_por_minuto: f64,
    log: &mut crate::modules::changelog::ChangeLog,
) -> Result<LisoGravado, String> {
    crate::modules::licenca::exigir()?;
    let plano = planejar(fps, fps_tipico(executavel), hz_principal(), trancos_por_minuto);
    let Some(teto) = plano.teto else {
        return Ok(LisoGravado { teto: None, motivo: plano.motivo });
    };
    if let super::teto_do_jogo::Decisao::Recusar(motivo) =
        super::teto_do_jogo::aplicar(executavel, caminho, super::teto_do_jogo::Pedido::JogoLiso(teto), log)?
    {
        return Ok(LisoGravado { teto: None, motivo });
    }
    crate::utils::Logger::info(&format!(
        "jogo liso depois da partida: teto de {} fps no perfil de {} ({:.0} fps, {:.1} trancos/min)",
        teto, executavel, fps, trancos_por_minuto
    ));
    Ok(LisoGravado {
        teto: Some(teto),
        motivo: format!("{} Vale na próxima vez que o jogo abrir.", plano.motivo),
    })
}

/// **Pura.** O teto que a tela mostrou pode ser gravado? Antes, "Travar em X" media o jogo de novo e gravava o que
/// saísse dessa segunda medição, que podia ser outro número. Agora grava o que a pessoa viu, se for jogável e não cortar
/// mais que `PERDA_MAXIMA` do FPS de costume (com 2 fps de folga do arredondamento). Teto nos Hz do monitor sempre vale.
pub fn teto_aceitavel(teto: u32, fps_tipico: Option<f64>, hz: u32) -> Result<(), String> {
    if !(24..=1000).contains(&teto) {
        return Err(format!("Teto de {} fps fora do que dá para jogar.", teto));
    }
    if hz > 0 && teto >= hz {
        return Ok(());
    }
    if let Some(t) = fps_tipico {
        let piso = (t * (1.0 - PERDA_MAXIMA)).floor() - 2.0;
        if (teto as f64) < piso {
            return Err(format!(
                "Travar em {} tiraria demais: este jogo costuma rodar a {:.0} fps aqui. Meça de novo.",
                teto, t
            ));
        }
    }
    Ok(())
}

/// Aplica o teto do plano pelo driver (guardando o anterior, no `ChangeLog`), para a PRÓXIMA abertura do jogo. O
/// executável é o nome do processo aberto (ex. `FiveM_b3258_GTAProcess.exe`).
#[cfg(target_os = "windows")]
pub async fn aplicar(
    executavel: &str,
    teto: u32,
    hz: u32,
    log: &mut crate::modules::changelog::ChangeLog,
) -> Result<(), String> {
    teto_aceitavel(teto, fps_tipico(executavel), hz)?;
    if let super::teto_do_jogo::Decisao::Recusar(motivo) =
        super::teto_do_jogo::aplicar(executavel, None, super::teto_do_jogo::Pedido::JogoLiso(teto), log)?
    {
        return Err(motivo);
    }
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
        // Depois: 45 tirava 25%; o teto nunca tira mais que `PERDA_MAXIMA`, então fica na folga exata (51).
        let p = planejar(60.0, None, 180, 74.0);
        assert_eq!(p.teto, Some(51), "abaixo da mediana, com folga para o processador");
        assert_eq!(p.multiplicador_do_boost, 3);
        assert!(p.motivo.contains("74"));
        assert_eq!(p.perda_pct, 15);
    }

    #[test]
    fn jogo_ja_liso_nao_perde_fps_para_um_teto() {
        // O print do dono com a Energia de jogo ligada: 80 fps, 0 trancos/min — o cartão sugeria travar em 60.
        let p = planejar(80.0, None, 180, 0.0);
        assert_eq!(p.teto, None);
        assert!(p.motivo.contains("já está liso"));
        assert_eq!(planejar(68.0, None, 180, 6.0).teto, Some(58));
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
        let p = planejar(190.0, None, 180, 2.0);
        assert_eq!(p.teto, Some(180), "trava nos Hz quando passa deles");
        let p = planejar(300.0, None, 180, 1.0);
        assert_eq!(p.teto, Some(180));
    }

    #[test]
    fn o_fivem_do_cliente_nao_e_mais_travado_na_metade_do_monitor() {
        // 15/11, GTX 1650 a 144 Hz: o FiveM rodava de 105 a 159 e o Jogo liso gravou 72.
        for fps in [85.0, 100.0, 120.0, 140.0] {
            let p = planejar(fps, None, 144, 12.0);
            let t = p.teto.expect("com trancos, há teto");
            assert!(t as f64 >= fps * 0.85 - 1.0, "{} fps virou teto {}", fps, t);
            assert!(p.perda_pct <= 15, "{}", p.perda_pct);
        }
        // A fração do monitor continua quando cabe: 85 fps → 72 (metade de 144) tira 15%.
        assert_eq!(planejar(85.0, None, 144, 12.0).teto, Some(72));
    }

    #[test]
    fn um_momento_ruim_nao_vira_o_teto_da_noite() {
        // Medido numa queda (85 fps) num jogo que costuma rodar a 130: a referência é o costume.
        let p = planejar(85.0, Some(130.0), 144, 12.0);
        assert_eq!(p.teto, Some(111));
        assert_eq!(p.fps_tipico, Some(130.0));
        // Costume abaixo do de agora: vale o de agora.
        assert_eq!(planejar(130.0, Some(90.0), 144, 12.0).teto, Some(111));
    }

    #[test]
    fn com_vrr_o_teto_fica_abaixo_dos_hz() {
        let acima = planejar(170.0, None, 144, 12.0);
        assert_eq!(acima.teto, Some(144));
        let p = com_vrr(acima.clone(), true);
        assert_eq!(p.teto, Some(141), "3 abaixo: o G-Sync/FreeSync continua valendo");
        assert!(p.motivo.contains("141"));
        assert_eq!(com_vrr(acima, false).teto, Some(144), "sem taxa variável, nos Hz");
        let abaixo = planejar(130.0, None, 144, 20.0);
        assert_eq!(com_vrr(abaixo.clone(), true).teto, abaixo.teto, "abaixo dos Hz nada muda");
    }

    #[test]
    fn costume_e_a_mediana_das_ultimas_medicoes() {
        assert_eq!(fps_de_costume(&[100.0, 140.0]), None, "poucas");
        assert_eq!(fps_de_costume(&[105.0, 91.0, 86.0, 143.0, 111.0]), Some(105.0));
        assert_eq!(fps_de_costume(&[100.0, 0.0, f64::NAN, 120.0, 140.0]), Some(120.0));
        let muitas: Vec<f64> = (0..30).map(|i| if i < 20 { 10.0 } else { 100.0 }).collect();
        assert_eq!(fps_de_costume(&muitas), Some(100.0), "só as 10 mais recentes");
    }

    #[test]
    fn o_teto_gravado_e_o_que_a_tela_mostrou_e_nunca_corta_demais() {
        assert!(teto_aceitavel(111, Some(130.0), 144).is_ok());
        assert!(teto_aceitavel(72, Some(130.0), 144).is_err(), "72 num jogo de 130");
        assert!(teto_aceitavel(72, None, 144).is_ok(), "sem costume, vale o que foi medido agora");
        assert!(teto_aceitavel(10, None, 144).is_err());
        assert!(teto_aceitavel(144, Some(300.0), 144).is_ok(), "teto nos Hz do monitor");
    }
}
