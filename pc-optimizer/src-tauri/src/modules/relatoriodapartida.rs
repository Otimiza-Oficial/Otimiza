// Relatório da partida (MASTER-PLAN, item 7): junta as janelas que a medição automática já gravou numa partida (a
// primeira três minutos depois de o jogo abrir, depois uma a cada vinte) e os eventos delas. Não mede nada a mais
// durante o jogo: é montado depois, na hora de mostrar, do que já está em `medicoes.json`.
//
// Não afirma queda. A comparação com as partidas anteriores sai como FAIXA (a mediana e os extremos), porque janelas
// de vinte segundos em lugares diferentes do mapa não provam nada sozinhas; queda com prova é trabalho de `deriva`.

use serde::Serialize;

use crate::modules::medicoes::{self, MedicaoAutomatica, TipoDeEvento, SEGUNDOS_ENTRE_MEDICOES};
use crate::modules::windows::presentmon::GargaloProvavel;

/// Janelas mais afastadas que isto são de partidas diferentes. Duas vezes o intervalo: uma medição que falhou no meio
/// não grava janela, e a partida não pode partir em duas por isso. O jogo fechado e aberto de novo antes disso conta
/// como a mesma partida: a medição não guarda o processo.
pub const INTERVALO_ENTRE_PARTIDAS_S: u64 = 2 * SEGUNDOS_ENTRE_MEDICOES + 5 * 60;

/// Partidas anteriores usadas na faixa.
pub const PARTIDAS_NA_FAIXA: usize = 5;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Faixa {
    pub partidas: usize,
    pub mediana_fps: f64,
    pub menor_fps: f64,
    pub maior_fps: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Relatorio {
    pub jogo: String,
    pub inicio: u64,
    pub fim: u64,
    /// Janelas medidas nesta partida, e quantas entram na conta (sem gerador de quadros).
    pub janelas: usize,
    pub janelas_comparaveis: usize,
    /// Média do FPS das janelas comparáveis. `None` sem nenhuma comparável.
    pub fps_medio: Option<f64>,
    /// O pior 1% entre as janelas comparáveis e com amostra suficiente (`confiavel`): um mínimo sobre janela curta
    /// seria ruído virando o número da partida.
    pub pior_low_1pct: Option<f64>,
    /// O gargalo da maioria das janelas medidas pelo PresentMon; empate não afirma nenhum.
    pub gargalo: Option<GargaloProvavel>,
    /// Trancos com instante, somados nas `janelas_com_trancos` que os leram. `None`: nenhuma leu.
    pub trancos: Option<usize>,
    pub janelas_com_trancos: usize,
    /// Com o disco ocupado em volta, pela conta da própria medição (antes do corte da lista de eventos). `None` quando
    /// alguma dessas janelas não cruzou com o disco.
    pub trancos_com_disco: Option<usize>,
    /// Freios da placa que começaram, somados nas `janelas_com_freio_lido`. `None`: a NVML não leu em nenhuma (AMD,
    /// Intel).
    pub freios: Option<usize>,
    pub janelas_com_freio_lido: usize,
    /// Alguma janela já começou com a placa freando: "não freou" seria mentira.
    pub ja_freava: bool,
    /// As partidas anteriores deste jogo, só janelas comparáveis. `None` com menos de duas.
    pub anteriores: Option<Faixa>,
}

/// **Pura.** As partidas de um jogo, cada uma com as suas janelas, em ordem.
pub fn partidas<'a>(jogo: &str, medicoes: &'a [MedicaoAutomatica]) -> Vec<Vec<&'a MedicaoAutomatica>> {
    let mut do_jogo: Vec<&MedicaoAutomatica> = medicoes.iter().filter(|m| m.jogo.eq_ignore_ascii_case(jogo)).collect();
    do_jogo.sort_by_key(|m| m.quando);
    let mut grupos: Vec<Vec<&MedicaoAutomatica>> = Vec::new();
    for m in do_jogo {
        match grupos.last_mut() {
            Some(g) if m.quando.saturating_sub(g.last().map(|u| u.quando).unwrap_or(0)) <= INTERVALO_ENTRE_PARTIDAS_S => g.push(m),
            _ => grupos.push(vec![m]),
        }
    }
    grupos
}

fn media(v: &[f64]) -> Option<f64> {
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

fn fps_da_partida(janelas: &[&MedicaoAutomatica]) -> Option<f64> {
    media(&janelas.iter().filter(|m| medicoes::comparavel(m)).map(|m| m.fps).collect::<Vec<_>>())
}

/// **Pura.** Só com um gargalo à frente dos outros; empate vira "nenhum ficou claro".
fn gargalo_da_maioria(contagem: &[(GargaloProvavel, usize)]) -> Option<GargaloProvavel> {
    let maior = contagem.iter().map(|(_, n)| *n).max()?;
    let lideres: Vec<GargaloProvavel> = contagem.iter().filter(|(_, n)| *n == maior).map(|(g, _)| *g).collect();
    Some(if lideres.len() == 1 { lideres[0] } else { GargaloProvavel::NaoDeuParaSaber })
}

/// **Pura.** O relatório da partida mais recente do jogo que teve a medição mais recente.
pub fn ultima(medicoes: &[MedicaoAutomatica]) -> Option<Relatorio> {
    let jogo = medicoes.iter().max_by_key(|m| m.quando)?.jogo.clone();
    let todas = partidas(&jogo, medicoes);
    let (esta, antes) = todas.split_last()?;

    let comparaveis: Vec<&&MedicaoAutomatica> = esta.iter().filter(|m| medicoes::comparavel(m)).collect();
    let mut contagem: Vec<(GargaloProvavel, usize)> = Vec::new();
    for g in esta.iter().filter_map(|m| m.presentmon.as_ref().map(|r| r.gargalo)) {
        match contagem.iter_mut().find(|(x, _)| *x == g) {
            Some((_, n)) => *n += 1,
            None => contagem.push((g, 1)),
        }
    }

    let janelas_com_eventos: Vec<&&MedicaoAutomatica> = esta.iter().filter(|m| m.eventos.is_some()).collect();
    let com_eventos: Vec<&medicoes::EventosDaPartida> = janelas_com_eventos.iter().filter_map(|m| m.eventos.as_ref()).collect();
    let trancos = (!com_eventos.is_empty()).then(|| com_eventos.iter().map(|e| e.trancos_com_instante).sum());
    // A lista de eventos é cortada em 50; a proporção com o disco que a medição gravou é sobre todos.
    let trancos_com_disco = if janelas_com_eventos.is_empty() {
        None
    } else {
        janelas_com_eventos
            .iter()
            .map(|m| match (m.trancos_com_disco_pct, m.trancos_medidos) {
                (Some(pct), Some(total)) => Some((pct * total as f64 / 100.0).round() as usize),
                (_, Some(0)) => Some(0),
                _ if m.eventos.as_ref().is_some_and(|e| e.trancos_com_instante == 0) => Some(0),
                _ => None,
            })
            .sum::<Option<usize>>()
    };
    let lidos: Vec<&&medicoes::EventosDaPartida> = com_eventos.iter().filter(|e| e.freios_lidos).collect();
    let ja_freava = lidos.iter().any(|e| e.freando_no_inicio);
    let freios = (!lidos.is_empty()).then(|| {
        lidos
            .iter()
            .flat_map(|e| e.lista.iter())
            .filter(|ev| matches!(ev.tipo, TipoDeEvento::FreioTermico | TipoDeEvento::FreioDeHardware))
            .count()
    });

    let mut anteriores: Vec<f64> = antes.iter().rev().filter_map(|p| fps_da_partida(p)).take(PARTIDAS_NA_FAIXA).collect();
    anteriores.sort_by(f64::total_cmp);
    let faixa = (anteriores.len() >= 2).then(|| Faixa {
        partidas: anteriores.len(),
        mediana_fps: if anteriores.len() % 2 == 1 {
            anteriores[anteriores.len() / 2]
        } else {
            (anteriores[anteriores.len() / 2 - 1] + anteriores[anteriores.len() / 2]) / 2.0
        },
        menor_fps: anteriores[0],
        maior_fps: anteriores[anteriores.len() - 1],
    });

    Some(Relatorio {
        jogo: esta.last()?.jogo.clone(),
        inicio: esta.first()?.quando,
        fim: esta.last()?.quando,
        janelas: esta.len(),
        janelas_comparaveis: comparaveis.len(),
        fps_medio: fps_da_partida(esta),
        pior_low_1pct: comparaveis.iter().filter(|m| m.confiavel).map(|m| m.low_1pct).min_by(f64::total_cmp),
        gargalo: gargalo_da_maioria(&contagem),
        trancos,
        janelas_com_trancos: com_eventos.len(),
        trancos_com_disco,
        freios,
        janelas_com_freio_lido: lidos.len(),
        ja_freava,
        anteriores: faixa,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::medicoes::{Evento, EventosDaPartida, GeracaoNaPartida};

    fn janela(jogo: &str, quando: u64, fps: f64) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: jogo.to_string(),
            quando,
            fps,
            low_1pct: fps / 2.0,
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
            presentmon: None,
            quadros: None,
            configuracao_do_jogo: None,
            versao_do_formato: 1,
            eventos: None,
        }
    }

    const P: u64 = INTERVALO_ENTRE_PARTIDAS_S + 1;

    #[test]
    fn janelas_proximas_sao_a_mesma_partida_e_afastadas_nao() {
        let ms = [janela("FiveM.exe", 0, 90.0), janela("FiveM.exe", 1200, 91.0), janela("FiveM.exe", 1200 + P, 92.0)];
        let p = partidas("FiveM.exe", &ms);
        assert_eq!(p.iter().map(|g| g.len()).collect::<Vec<_>>(), vec![2, 1]);
    }

    #[test]
    fn a_ultima_partida_junta_as_janelas_e_ignora_as_com_gerador_na_conta() {
        let ms = [
            janela("FiveM.exe", 0, 100.0),
            MedicaoAutomatica { geracao: Some(GeracaoNaPartida::LosslessScaling), ..janela("FiveM.exe", 1200, 60.0) },
            janela("FiveM.exe", 2400, 80.0),
        ];
        let r = ultima(&ms).unwrap();
        assert_eq!((r.janelas, r.janelas_comparaveis), (3, 2));
        assert_eq!(r.fps_medio, Some(90.0));
        assert_eq!(r.pior_low_1pct, Some(40.0));
        assert_eq!(r.anteriores, None, "sem partida anterior não há faixa");
    }

    #[test]
    fn sem_eventos_lidos_nao_vira_zero_trancos() {
        let r = ultima(&[janela("FiveM.exe", 0, 90.0)]).unwrap();
        assert_eq!((r.trancos, r.freios), (None, None));
    }

    #[test]
    fn freio_so_conta_quando_a_nvml_leu() {
        let eventos = |freios_lidos| EventosDaPartida {
            lista: vec![
                Evento { segundo: 3.0, tipo: TipoDeEvento::Tranco { com_disco: true } },
                Evento { segundo: 9.0, tipo: TipoDeEvento::FreioTermico },
            ],
            freios_lidos,
            freando_no_inicio: false,
            trancos_com_instante: 4,
        };
        let com_disco = |m: MedicaoAutomatica| MedicaoAutomatica { trancos_com_disco_pct: Some(25.0), trancos_medidos: Some(4), ..m };
        let r = ultima(&[com_disco(MedicaoAutomatica { eventos: Some(eventos(true)), ..janela("FiveM.exe", 0, 90.0) })]).unwrap();
        assert_eq!((r.trancos, r.trancos_com_disco, r.freios), (Some(4), Some(1), Some(1)));
        let r = ultima(&[MedicaoAutomatica { eventos: Some(eventos(false)), ..janela("FiveM.exe", 0, 90.0) }]).unwrap();
        assert_eq!(r.freios, None, "sem leitura da NVML não se diz 'nenhum freio'");
    }

    #[test]
    fn quem_ja_freava_no_comeco_nao_aparece_como_sem_freio() {
        let e = EventosDaPartida { lista: vec![], freios_lidos: true, freando_no_inicio: true, trancos_com_instante: 0 };
        let r = ultima(&[MedicaoAutomatica { eventos: Some(e), ..janela("FiveM.exe", 0, 90.0) }]).unwrap();
        assert_eq!(r.freios, Some(0));
        assert!(r.ja_freava);
    }

    #[test]
    fn o_disco_conta_todos_os_trancos_e_nao_so_os_da_lista_cortada() {
        let e = EventosDaPartida { lista: vec![], freios_lidos: false, freando_no_inicio: false, trancos_com_instante: 200 };
        let m = MedicaoAutomatica { eventos: Some(e), trancos_com_disco_pct: Some(100.0), trancos_medidos: Some(200), ..janela("FiveM.exe", 0, 90.0) };
        assert_eq!(ultima(&[m]).unwrap().trancos_com_disco, Some(200));
    }

    #[test]
    fn o_1_pct_de_janela_curta_nao_vira_o_da_partida() {
        let curta = MedicaoAutomatica { confiavel: false, low_1pct: 5.0, ..janela("FiveM.exe", 1200, 90.0) };
        let r = ultima(&[janela("FiveM.exe", 0, 90.0), curta]).unwrap();
        assert_eq!(r.pior_low_1pct, Some(45.0));
    }

    #[test]
    fn empate_no_gargalo_nao_afirma_nenhum() {
        assert_eq!(gargalo_da_maioria(&[(GargaloProvavel::Cpu, 1), (GargaloProvavel::Gpu, 1)]), Some(GargaloProvavel::NaoDeuParaSaber));
        assert_eq!(gargalo_da_maioria(&[(GargaloProvavel::Cpu, 2), (GargaloProvavel::Gpu, 1)]), Some(GargaloProvavel::Cpu));
    }

    #[test]
    fn uma_medicao_que_falhou_no_meio_nao_parte_a_partida() {
        let ms = [janela("FiveM.exe", 0, 90.0), janela("FiveM.exe", 2 * SEGUNDOS_ENTRE_MEDICOES + 60, 91.0)];
        assert_eq!(partidas("FiveM.exe", &ms).len(), 1);
    }

    #[test]
    fn a_faixa_vem_das_partidas_anteriores_do_mesmo_jogo() {
        let ms = [
            janela("FiveM.exe", 0, 100.0),
            janela("FiveM.exe", P, 110.0),
            janela("FiveM.exe", 2 * P, 120.0),
            janela("GTA5.exe", 2 * P + 10, 50.0),
            janela("FiveM.exe", 3 * P, 90.0),
        ];
        let r = ultima(&ms).unwrap();
        assert_eq!(r.jogo, "FiveM.exe");
        assert_eq!(r.anteriores, Some(Faixa { partidas: 3, mediana_fps: 110.0, menor_fps: 100.0, maior_fps: 120.0 }));
    }
}
