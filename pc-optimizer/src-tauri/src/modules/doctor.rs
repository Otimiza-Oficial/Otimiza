// Diagnóstico da partida (MASTER-PLAN, item 2): "por que o meu jogo roda assim?", respondido com a última partida
// que a medição automática já gravou (as janelas do PresentMon) e os limites de FPS escondidos (`tetos`). No máximo
// três causas, cada uma com o número que a sustenta, a confiança e o que ajuda e o que NÃO ajuda.
//
// Regras: sem leitura não há causa (vira lacuna); uma janela só nunca dá confiança alta; "espera" sem limite
// encontrado é hipótese, não causa. A tela decide pelo enum (`TipoDeCausa`, `Confianca`, `Ajuste`), nunca pelo
// texto. Nada aqui escreve no Windows.

use serde::Serialize;

use crate::modules::medicoes::{self, MedicaoAutomatica, TipoDeEvento};
use crate::modules::windows::presentmon::{Apresentacao, GargaloProvavel, Resumo};
use crate::modules::windows::tetos::Teto;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TipoDeCausa {
    /// O processador monta os quadros no limite; a placa sobra.
    Processador,
    /// A placa de vídeo trabalha o quadro inteiro.
    PlacaDeVideo,
    /// O FPS ficou colado na taxa do monitor: V-Sync ou limite na taxa. Mais FPS aqui não aparece na tela.
    PresoNaTaxaDoMonitor,
    /// Nenhum dos dois no limite e um teto de FPS encontrado (ou suspeito).
    LimiteDeFps,
    /// A placa freou por calor ou energia durante as janelas.
    CalorOuEnergia,
    /// Os trancos coincidem com o disco ocupado: carregamento, não FPS médio.
    DiscoNosTrancos,
    /// O Windows copia cada quadro antes de mostrar: mais atraso até a tela.
    QuadroCopiado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Confianca {
    Alta,
    Media,
    /// Sinal fraco ou indireto: aparece, mas não entra na receita automática.
    Hipotese,
}

/// Ajustes conhecidos, pelo que atacam. A receita (item 3) é quem escolhe e prova.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Ajuste {
    PerfilGraficoMaisLeve,
    MenosCargaNoProcessador,
    PlanoDeEnergiaOtimiza,
    FecharProgramasPesados,
    TirarLimiteDeFps,
    ConferirRefrigeracao,
    JogoNoSsd,
    TelaCheiaOuSemBordaComFlip,
    MaisResolucaoOuQualidade,
    BaixarResolucao,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Causa {
    pub tipo: TipoDeCausa,
    pub confianca: Confianca,
    /// Os números da partida, já em frase curta: "processador ocupado em 97% de cada quadro, placa em 61%".
    pub evidencia: String,
    pub ajuda: Vec<Ajuste>,
    pub nao_ajuda: Vec<Ajuste>,
    /// Janelas que sustentam a causa, de `janelas_lidas`.
    pub janelas: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diagnostico {
    pub jogo: Option<String>,
    /// Janelas da partida medidas pelo PresentMon (as outras não dizem o gargalo).
    pub janelas_lidas: usize,
    pub causas: Vec<Causa>,
    /// O que não deu para ler, em frase: faltar leitura não é "está tudo bem".
    pub lacunas: Vec<String>,
}

pub const CAUSAS_NA_TELA: usize = 3;
/// A mesma regra de `frames::trancos_com_disco`: 40% dos trancos com o disco ocupado.
const DISCO_NOS_TRANCOS_PCT: f64 = 40.0;
/// Menos que isso e a proporção não diz nada.
const TRANCOS_MINIMOS: usize = 10;

fn pct(parte: Option<f64>, quadro: f64) -> Option<f64> {
    parte.filter(|_| quadro > 0.0).map(|p| (p / quadro * 100.0).min(100.0))
}

fn confianca_pela_quantidade(de_acordo: usize, lidas: usize) -> Option<Confianca> {
    // Maioria das janelas; uma só nunca é "alta".
    if de_acordo == 0 || de_acordo * 2 <= lidas {
        None
    } else if de_acordo >= 2 {
        Some(Confianca::Alta)
    } else {
        Some(Confianca::Media)
    }
}

fn nome_do_teto(t: &Teto) -> String {
    match t {
        Teto::LimiteGlobalNvidia { fps } => format!("limite de {} FPS no painel da NVIDIA", fps),
        Teto::VsyncForcadoNvidia => "V-Sync forçado no painel da NVIDIA".to_string(),
        Teto::Rtss => "RivaTuner (RTSS) aberto, que costuma limitar o FPS".to_string(),
        Teto::LimiteNoJogo { fps, .. } => format!("limite de {} FPS na configuração do jogo", fps),
        Teto::VsyncNoJogo { .. } => "V-Sync ligado na configuração do jogo".to_string(),
        Teto::RobloxLimitado { fps: Some(fps), .. } => format!("limite de {} FPS no Roblox", fps),
        Teto::RobloxLimitado { fps: None, .. } => "o limite padrão do Roblox".to_string(),
    }
}

/// O número do teto, quando ele tem um.
fn fps_do_teto(t: &Teto) -> Option<u32> {
    match t {
        Teto::LimiteGlobalNvidia { fps } | Teto::LimiteNoJogo { fps, .. } => Some(*fps),
        Teto::RobloxLimitado { fps, .. } => *fps,
        _ => None,
    }
}

/// Um limite um pouco abaixo da taxa (o Reflex e o limite recomendado para VRR ficam uns 5% abaixo) é saudável:
/// entra como "no máximo que a tela mostra", e nunca como "tire o limite".
const PERTO_DA_TAXA_ABAIXO: f64 = 0.94;
const PERTO_DA_TAXA_ACIMA: f64 = 1.02;
/// O FPS medido bate com o número do teto.
const BATE_COM_O_TETO: f64 = 0.03;

/// **Pura.** O teto explica ESTE jogo? Os do painel da NVIDIA e o RTSS valem para todos; os gravados na configuração
/// de um jogo só para ele (comparado pelo nome, sem maiúscula nem espaço: "Satisfactory" no executável
/// "FactoryGame-Win64-Shipping.exe" não casa, e aí fica de fora em vez de acusar à toa).
fn teto_vale_para(t: &Teto, executavel: &str) -> bool {
    let exe = executavel.to_lowercase();
    let casa = |nome: &str| {
        let nome: String = nome.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
        !nome.is_empty() && exe.chars().filter(|c| c.is_alphanumeric()).collect::<String>().contains(&nome)
    };
    match t {
        Teto::LimiteGlobalNvidia { .. } | Teto::VsyncForcadoNvidia | Teto::Rtss => true,
        Teto::LimiteNoJogo { jogo, .. } | Teto::VsyncNoJogo { jogo, .. } => casa(jogo),
        Teto::RobloxLimitado { .. } => exe.contains("roblox"),
    }
}

/// **Pura.** `janelas`: as da última partida (ver `relatoriodapartida::partidas`). `tetos`: `None` quando não se
/// conseguiu procurar.
pub fn diagnosticar(janelas: &[MedicaoAutomatica], tetos: Option<&[Teto]>, monitor_hz: Option<u32>) -> Diagnostico {
    let mut lacunas = Vec::new();
    let pm: Vec<&Resumo> = janelas.iter().filter_map(|m| m.presentmon.as_ref()).collect();
    let lidas = pm.len();
    let jogo = janelas.last().map(|m| m.jogo.clone());
    if janelas.is_empty() {
        lacunas.push("Ainda não há partida medida: jogue alguns minutos com o Otimiza aberto.".to_string());
    } else if lidas == 0 {
        lacunas.push("A última partida foi medida pelo canal antigo, que não separa processador de placa.".to_string());
    }

    let mut causas: Vec<Causa> = Vec::new();
    let conta = |g: GargaloProvavel| pm.iter().filter(|r| r.gargalo == g).count();
    let media = |f: &dyn Fn(&Resumo) -> Option<f64>| {
        let v: Vec<f64> = pm.iter().filter_map(|r| f(r)).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    };
    let cpu = media(&|r| pct(r.cpu_ocupada_media_ms, r.quadro_medio_ms));
    let gpu = media(&|r| pct(r.gpu_ocupada_media_ms, r.quadro_medio_ms));
    let ocupacao = || match (cpu, gpu) {
        (Some(c), Some(g)) => format!("processador ocupado em {:.0}% de cada quadro, placa de vídeo em {:.0}%", c, g),
        _ => "pelo PresentMon".to_string(),
    };

    if let Some(conf) = confianca_pela_quantidade(conta(GargaloProvavel::Cpu), lidas) {
        causas.push(Causa {
            tipo: TipoDeCausa::Processador,
            confianca: conf,
            evidencia: format!("Em {} de {} janelas: {}.", conta(GargaloProvavel::Cpu), lidas, ocupacao()),
            ajuda: vec![Ajuste::MenosCargaNoProcessador, Ajuste::PlanoDeEnergiaOtimiza, Ajuste::FecharProgramasPesados],
            nao_ajuda: vec![Ajuste::BaixarResolucao],
            janelas: conta(GargaloProvavel::Cpu),
        });
    }
    if let Some(conf) = confianca_pela_quantidade(conta(GargaloProvavel::Gpu), lidas) {
        causas.push(Causa {
            tipo: TipoDeCausa::PlacaDeVideo,
            confianca: conf,
            evidencia: format!("Em {} de {} janelas: {}.", conta(GargaloProvavel::Gpu), lidas, ocupacao()),
            ajuda: vec![Ajuste::PerfilGraficoMaisLeve, Ajuste::BaixarResolucao],
            nao_ajuda: vec![Ajuste::FecharProgramasPesados, Ajuste::MenosCargaNoProcessador],
            janelas: conta(GargaloProvavel::Gpu),
        });
    }
    let esperas = conta(GargaloProvavel::Espera);
    // Colado na taxa do monitor (2% de folga): é o próprio V-Sync ou limite na taxa, e o que o cliente vê já é o máximo
    // que a tela mostra. Vem antes do "limite de FPS": aqui tirar o limite não mostra quadro a mais.
    if monitor_hz.is_none() && !janelas.is_empty() {
        lacunas.push("Não deu para ler a taxa do monitor.".to_string());
    }
    let na_taxa = monitor_hz.filter(|hz| *hz > 0).map(|hz| {
        let (baixo, alto) = (hz as f64 * PERTO_DA_TAXA_ABAIXO, hz as f64 * PERTO_DA_TAXA_ACIMA);
        pm.iter()
            .filter(|r| r.gargalo == GargaloProvavel::Espera && r.fps_do_jogo_medio >= baixo && r.fps_do_jogo_medio <= alto)
            .count()
    });
    let preso_na_taxa = matches!(na_taxa, Some(n) if n * 2 > lidas && n > 0);
    if preso_na_taxa {
        let n = na_taxa.unwrap_or(0);
        causas.push(Causa {
            tipo: TipoDeCausa::PresoNaTaxaDoMonitor,
            confianca: if n >= 2 { Confianca::Alta } else { Confianca::Media },
            evidencia: format!(
                "Em {} de {} janelas o jogo ficou na taxa do monitor ({} Hz) ou logo abaixo dela, com o processador e a placa \
                 folgados ({}). A tela já mostra o máximo que consegue: otimizar aqui não aparece como FPS a mais.",
                n,
                lidas,
                monitor_hz.unwrap_or(0),
                ocupacao()
            ),
            ajuda: vec![],
            nao_ajuda: vec![Ajuste::PlanoDeEnergiaOtimiza, Ajuste::PerfilGraficoMaisLeve, Ajuste::FecharProgramasPesados],
            janelas: n,
        });
    }
    if !preso_na_taxa && esperas * 2 > lidas && esperas > 0 {
        let deste_jogo: Option<Vec<&Teto>> =
            tetos.map(|t| t.iter().filter(|x| teto_vale_para(x, jogo.as_deref().unwrap_or(""))).collect());
        let batem = |fps: u32| {
            pm.iter()
                .filter(|r| r.gargalo == GargaloProvavel::Espera && (r.fps_do_jogo_medio - fps as f64).abs() <= fps as f64 * BATE_COM_O_TETO)
                .count()
        };
        let (confianca, evidencia) = match deste_jogo.as_deref() {
            Some(t) if !t.is_empty() => {
                let com_numero = t.iter().filter_map(|x| fps_do_teto(x).map(|f| (x, f))).max_by_key(|(_, f)| batem(*f));
                let nomes = t.iter().map(|x| nome_do_teto(x)).collect::<Vec<_>>().join(" e ");
                match com_numero {
                    Some((_, fps)) if batem(fps) * 2 > lidas => (
                        if batem(fps) >= 2 { Confianca::Alta } else { Confianca::Media },
                        format!(
                            "Em {} de {} janelas o jogo ficou em {} FPS com o processador e a placa folgados, o mesmo número \
                             do limite encontrado ({}).",
                            batem(fps),
                            lidas,
                            fps,
                            nomes
                        ),
                    ),
                    _ => (
                        Confianca::Media,
                        format!(
                            "Em {} de {} janelas nem o processador nem a placa estavam no limite, e há {}; o FPS medido não \
                             confirma que é ele.",
                            esperas, lidas, nomes
                        ),
                    ),
                }
            }
            Some(_) => (
                Confianca::Hipotese,
                format!(
                    "Em {} de {} janelas nem o processador nem a placa estavam no limite, e nenhum limite conhecido foi \
                     encontrado: pode ser V-Sync ou limite do próprio jogo.",
                    esperas, lidas
                ),
            ),
            None => {
                lacunas.push("Não deu para procurar limites de FPS escondidos.".to_string());
                (
                    Confianca::Hipotese,
                    format!("Em {} de {} janelas nem o processador nem a placa estavam no limite.", esperas, lidas),
                )
            }
        };
        causas.push(Causa {
            tipo: TipoDeCausa::LimiteDeFps,
            confianca,
            evidencia,
            ajuda: vec![Ajuste::TirarLimiteDeFps],
            nao_ajuda: vec![Ajuste::PlanoDeEnergiaOtimiza, Ajuste::FecharProgramasPesados, Ajuste::PerfilGraficoMaisLeve],
            janelas: esperas,
        });
    }

    let lidos: Vec<&crate::modules::medicoes::EventosDaPartida> =
        janelas.iter().filter_map(|m| m.eventos.as_ref()).filter(|e| e.freios_lidos).collect();
    let com_freio = janelas
        .iter()
        .filter(|m| m.presentmon.as_ref().is_some_and(|r| r.gargalo == GargaloProvavel::Gpu))
        .filter_map(|m| m.eventos.as_ref())
        .filter(|e| e.freios_lidos)
        .filter(|e| {
            e.freando_no_inicio
                || e.lista.iter().any(|ev| matches!(ev.tipo, TipoDeEvento::FreioTermico | TipoDeEvento::FreioDeHardware))
        })
        .count();
    if com_freio > 0 {
        causas.push(Causa {
            tipo: TipoDeCausa::CalorOuEnergia,
            // O bit de hardware da NVML não separa calor de energia nem de troca de clock: no máximo "provável".
            confianca: Confianca::Media,
            evidencia: format!(
                "Com a placa de vídeo no limite, ela freou por calor ou energia em {} de {} janelas lidas.",
                com_freio,
                lidos.len()
            ),
            ajuda: vec![Ajuste::ConferirRefrigeracao],
            nao_ajuda: vec![Ajuste::PerfilGraficoMaisLeve],
            janelas: com_freio,
        });
    } else if lidos.is_empty() && !janelas.is_empty() {
        // Sem `eventos` é medição de antes do registro do freio; com eventos e sem leitura, a placa não informa.
        lacunas.push(if janelas.iter().all(|m| m.eventos.is_none()) {
            "Estas partidas foram medidas antes de o Otimiza registrar o freio da placa: a próxima partida já traz.".to_string()
        } else {
            "Freio da placa não lido (só placas NVIDIA informam).".to_string()
        });
    }

    let (trancos, com_disco) = janelas.iter().fold((0usize, 0.0f64), |(t, d), m| match (m.trancos_medidos, m.trancos_com_disco_pct) {
        (Some(n), Some(p)) => (t + n, d + p * n as f64 / 100.0),
        _ => (t, d),
    });
    let trancos_sem_disco = janelas
        .iter()
        .any(|m| m.trancos_medidos.is_none() && m.eventos.as_ref().is_some_and(|e| e.trancos_com_instante > 0));
    if trancos_sem_disco {
        lacunas.push("Parte dos trancos não foi cruzada com o disco nesta partida.".to_string());
    }
    if trancos >= TRANCOS_MINIMOS && com_disco / trancos as f64 * 100.0 >= DISCO_NOS_TRANCOS_PCT {
        causas.push(Causa {
            tipo: TipoDeCausa::DiscoNosTrancos,
            confianca: Confianca::Media,
            evidencia: format!(
                "{:.0} de {} trancos aconteceram com o disco ocupado: é carregamento (textura, shader, mapa), não falta de FPS.",
                com_disco, trancos
            ),
            ajuda: vec![Ajuste::JogoNoSsd],
            nao_ajuda: vec![Ajuste::PerfilGraficoMaisLeve, Ajuste::PlanoDeEnergiaOtimiza],
            janelas: janelas.iter().filter(|m| m.trancos_medidos.is_some()).count(),
        });
    }

    let copiadas = pm.iter().filter(|r| r.apresentacao == Apresentacao::Copiada).count();
    let com_gerador = janelas.iter().any(|m| !medicoes::comparavel(m));
    if copiadas * 2 > lidas && copiadas > 0 && !com_gerador {
        causas.push(Causa {
            tipo: TipoDeCausa::QuadroCopiado,
            confianca: Confianca::Media,
            evidencia: format!("Em {} de {} janelas o Windows copiou cada quadro antes de mostrar (mais atraso até a tela).", copiadas, lidas),
            ajuda: vec![Ajuste::TelaCheiaOuSemBordaComFlip],
            nao_ajuda: vec![Ajuste::PerfilGraficoMaisLeve],
            janelas: copiadas,
        });
    }

    // Mais forte primeiro: confiança, depois quantas janelas sustentam.
    causas.sort_by(|a, b| a.confianca.cmp(&b.confianca).then(b.janelas.cmp(&a.janelas)));
    causas.truncate(CAUSAS_NA_TELA);
    Diagnostico { jogo, janelas_lidas: lidas, causas, lacunas }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::medicoes::EventosDaPartida;

    fn resumo(gargalo: GargaloProvavel, cpu_ms: f64, gpu_ms: f64) -> Resumo {
        Resumo {
            segundos: 20.0,
            quadros_do_jogo: 2000,
            quadros_gerados: 0,
            quadros_descartados: 0,
            fps_do_jogo_medio: 100.0,
            fps_do_jogo_mediana: 100.0,
            fps_exibido: 100.0,
            low_1pct: Some(50.0),
            low_01pct: None,
            quadro_medio_ms: 10.0,
            quadro_p95_ms: 12.0,
            quadro_p99_ms: 15.0,
            quadro_desvio_ms: 1.0,
            engasgos_por_minuto: 1.0,
            modo_de_apresentacao: "Hardware: Independent Flip".to_string(),
            apresentacao: Apresentacao::Direta,
            api: Default::default(),
            cpu_ocupada_media_ms: Some(cpu_ms),
            gpu_ocupada_media_ms: Some(gpu_ms),
            gargalo,
            ate_a_tela_media_ms: None,
            geradores: Vec::new(),
            trancos_qpc: Vec::new(),
        }
    }

    fn janela(r: Option<Resumo>) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: "FiveM_b3258_GTAProcess.exe".to_string(),
            quando: 0,
            fps: 100.0,
            low_1pct: 50.0,
            engasgos_por_minuto: 1.0,
            segundos: 20.0,
            confiavel: true,
            mudancas_aplicadas: 0,
            ambiente: None,
            frametime_medio_ms: None,
            frametime_p95_ms: None,
            frametime_p99_ms: None,
            placa: None,
            cpu_uso_pct: None,
            gpu_uso_pct: None,
            trancos_com_disco_pct: None,
            trancos_medidos: None,
            governador: None,
            geracao: None,
            presentmon: r,
            quadros: None,
            configuracao_do_jogo: None,
            versao_do_formato: 1,
            eventos: None,
        }
    }

    #[test]
    fn processador_no_limite_em_todas_as_janelas_e_causa_alta() {
        let js = [janela(Some(resumo(GargaloProvavel::Cpu, 9.7, 6.1))), janela(Some(resumo(GargaloProvavel::Cpu, 9.5, 6.0)))];
        let d = diagnosticar(&js, Some(&[]), None);
        assert_eq!(d.causas[0].tipo, TipoDeCausa::Processador);
        assert_eq!(d.causas[0].confianca, Confianca::Alta);
        assert!(d.causas[0].evidencia.contains("96%"), "{}", d.causas[0].evidencia);
        assert!(d.causas[0].nao_ajuda.contains(&Ajuste::BaixarResolucao));
    }

    #[test]
    fn uma_janela_so_nunca_e_alta() {
        let d = diagnosticar(&[janela(Some(resumo(GargaloProvavel::Gpu, 5.0, 9.8)))], Some(&[]), None);
        assert_eq!((d.causas[0].tipo, d.causas[0].confianca), (TipoDeCausa::PlacaDeVideo, Confianca::Media));
    }

    #[test]
    fn empate_nao_vira_causa() {
        let js = [janela(Some(resumo(GargaloProvavel::Cpu, 9.7, 6.0))), janela(Some(resumo(GargaloProvavel::Gpu, 6.0, 9.7)))];
        assert!(diagnosticar(&js, Some(&[]), None).causas.is_empty());
    }

    #[test]
    fn espera_so_e_causa_alta_com_limite_encontrado() {
        let js = [janela(Some(resumo(GargaloProvavel::Espera, 4.0, 5.0))), janela(Some(resumo(GargaloProvavel::Espera, 4.0, 5.0)))];
        let com = diagnosticar(&js, Some(&[Teto::LimiteGlobalNvidia { fps: 100 }]), None);
        assert_eq!((com.causas[0].tipo, com.causas[0].confianca), (TipoDeCausa::LimiteDeFps, Confianca::Alta));
        assert!(com.causas[0].evidencia.contains("100 FPS"), "{}", com.causas[0].evidencia);
        let nao_bate = diagnosticar(&js, Some(&[Teto::LimiteGlobalNvidia { fps: 60 }]), None);
        assert_eq!(nao_bate.causas[0].confianca, Confianca::Media, "limite de 60 com o jogo a 100 não é o que segura");
        let de_outro_jogo = diagnosticar(&js, Some(&[Teto::RobloxLimitado { fps: Some(60), arquivo: "x".into() }]), None);
        assert_eq!(de_outro_jogo.causas[0].confianca, Confianca::Hipotese, "limite do Roblox não explica o FiveM");
        let rtss = diagnosticar(&js, Some(&[Teto::Rtss]), None);
        assert_eq!(rtss.causas[0].confianca, Confianca::Media, "RTSS aberto sem número não é evidência forte");
        let uma = diagnosticar(&js[..1], Some(&[Teto::LimiteGlobalNvidia { fps: 100 }]), None);
        assert_eq!(uma.causas[0].confianca, Confianca::Media, "uma janela só nunca é alta");
        let sem = diagnosticar(&js, Some(&[]), None);
        assert_eq!(sem.causas[0].confianca, Confianca::Hipotese);
        let sem_leitura = diagnosticar(&js, None, None);
        assert!(!sem_leitura.lacunas.is_empty(), "não ter procurado é lacuna");
    }

    #[test]
    fn fps_colado_na_taxa_do_monitor_diz_que_nao_ha_o_que_ganhar_na_tela() {
        let mut r = resumo(GargaloProvavel::Espera, 1.3, 1.9);
        r.fps_do_jogo_medio = 180.0;
        r.quadro_medio_ms = 5.56;
        let js = [janela(Some(r.clone())), janela(Some(r))];
        let d = diagnosticar(&js, Some(&[]), Some(180));
        assert_eq!((d.causas[0].tipo, d.causas[0].confianca), (TipoDeCausa::PresoNaTaxaDoMonitor, Confianca::Alta));
        assert!(!d.causas.iter().any(|c| c.tipo == TipoDeCausa::LimiteDeFps), "não repete como limite a tirar");
        assert!(d.causas[0].nao_ajuda.contains(&Ajuste::PlanoDeEnergiaOtimiza));
        assert!(d.causas[0].ajuda.is_empty(), "não há o que fazer pelo FPS: nada de 'sobe a qualidade'");
        let longe = diagnosticar(&js, Some(&[]), Some(240));
        assert_eq!(longe.causas[0].tipo, TipoDeCausa::LimiteDeFps, "180 num monitor de 240 Hz é outro limite");
        let reflex = diagnosticar(&js, Some(&[]), Some(190));
        assert_eq!(reflex.causas[0].tipo, TipoDeCausa::PresoNaTaxaDoMonitor, "5% abaixo da taxa (Reflex, VRR) não vira 'tire o limite'");
    }

    #[test]
    fn sem_partida_ou_so_canal_antigo_vira_lacuna_e_nao_causa() {
        let vazio = diagnosticar(&[], Some(&[]), None);
        assert!(vazio.causas.is_empty() && !vazio.lacunas.is_empty());
        let antigo = diagnosticar(&[janela(None)], Some(&[]), None);
        assert!(antigo.causas.is_empty() && antigo.lacunas.iter().any(|l| l.contains("canal antigo")));
    }

    #[test]
    fn freio_e_disco_entram_com_a_evidencia() {
        let e = EventosDaPartida { lista: vec![], freios_lidos: true, freando_no_inicio: true, trancos_com_instante: 30 };
        let j = MedicaoAutomatica {
            eventos: Some(e),
            trancos_medidos: Some(30),
            trancos_com_disco_pct: Some(60.0),
            ..janela(Some(resumo(GargaloProvavel::Gpu, 5.0, 9.8)))
        };
        let d = diagnosticar(&[j], Some(&[]), None);
        let tipos: Vec<TipoDeCausa> = d.causas.iter().map(|c| c.tipo).collect();
        assert!(tipos.contains(&TipoDeCausa::CalorOuEnergia) && tipos.contains(&TipoDeCausa::DiscoNosTrancos), "{tipos:?}");
    }

    #[test]
    fn com_gerador_de_quadros_nao_manda_para_tela_cheia() {
        let mut r = resumo(GargaloProvavel::Gpu, 5.0, 9.8);
        r.apresentacao = Apresentacao::Copiada;
        let j = MedicaoAutomatica { geracao: Some(crate::modules::medicoes::GeracaoNaPartida::LosslessScaling), ..janela(Some(r)) };
        assert!(!diagnosticar(&[j], Some(&[]), None).causas.iter().any(|c| c.tipo == TipoDeCausa::QuadroCopiado));
    }

    #[test]
    fn freio_com_o_jogo_preso_em_limite_nao_e_causa() {
        let e = EventosDaPartida { lista: vec![], freios_lidos: true, freando_no_inicio: true, trancos_com_instante: 0 };
        let j = MedicaoAutomatica { eventos: Some(e), ..janela(Some(resumo(GargaloProvavel::Espera, 3.0, 4.0))) };
        assert!(!diagnosticar(&[j], Some(&[]), None).causas.iter().any(|c| c.tipo == TipoDeCausa::CalorOuEnergia));
    }

    #[test]
    fn no_maximo_tres_causas_e_a_mais_forte_primeiro() {
        let e = EventosDaPartida { lista: vec![], freios_lidos: true, freando_no_inicio: true, trancos_com_instante: 30 };
        let mut r = resumo(GargaloProvavel::Cpu, 9.7, 6.0);
        r.apresentacao = Apresentacao::Copiada;
        let j = || MedicaoAutomatica { eventos: Some(e.clone()), trancos_medidos: Some(30), trancos_com_disco_pct: Some(60.0), ..janela(Some(r.clone())) };
        let d = diagnosticar(&[j(), j()], Some(&[]), None);
        assert_eq!(d.causas.len(), CAUSAS_NA_TELA);
        assert!(d.causas.windows(2).all(|p| p[0].confianca <= p[1].confianca));
    }

    /// Laboratório: o diagnóstico com as partidas REAIS desta máquina. `cargo test --release --lib doctor_real -- --ignored --nocapture`
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "laboratório: lê as medições e os limites desta máquina"]
    fn doctor_real() {
        let medicoes = crate::modules::medicoes::ler().expect("medicoes.json");
        let jogo = medicoes.iter().max_by_key(|m| m.quando).map(|m| m.jogo.clone()).expect("alguma partida");
        let ultima: Vec<MedicaoAutomatica> = crate::modules::relatoriodapartida::partidas(&jogo, &medicoes).pop().unwrap().into_iter().cloned().collect();
        let t = crate::modules::windows::tetos::procurar();
        println!("monitor: {:?} Hz, tetos: {:?}", t.monitor_hz, t.tetos);
        println!("{:#?}", diagnosticar(&ultima, Some(&t.tetos), t.monitor_hz));
    }
}
