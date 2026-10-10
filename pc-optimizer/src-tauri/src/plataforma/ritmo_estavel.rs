// RITMO ESTÁVEL. A reclamação do dono ("o jogo trava, mesmo com FPS alto") e a medição do próprio Otimiza no FiveM dele
// (`medicoes.json`: 45 fps de mediana, mas 74 trancos/min e P99 de 72 ms) são o mesmo problema: o FPS oscila (40–76) e
// cada queda é um tranco. A regra do Lossless Scaling e do SpecialK: travar o FPS num valor que o jogo SEGURA sempre, e
// que divida certo o monitor. Ritmo constante é o que o olho sente como "liso", não o número médio.
//
// O teto é aplicado pelo driver (`nvdriver::aplicar_limite`), e medido aqui: só vale com o jogo aberto DEPOIS de aplicar
// (teste `laboratorio_do_limite`). Por isso o Modo PC fraco aplica o teto e pede para reabrir o jogo.

/// O FPS que o jogo segura: o percentil `p` (0–100) de baixo das amostras por segundo. Com o jogo a 40–76, o P5 é ~41:
/// é o que ele entrega quase sempre, e não a média (que inclui os picos que ele não sustenta).
pub fn fps_sustentado(amostras_por_segundo: &[f64], p: f64) -> Option<f64> {
    let mut v: Vec<f64> = amostras_por_segundo
        .iter()
        .copied()
        .filter(|x| *x > 0.0)
        .collect();
    if v.len() < 3 {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let i = ((p / 100.0) * (v.len() - 1) as f64).round() as usize;
    Some(v[i.min(v.len() - 1)])
}

/// O teto que deixa o ritmo liso: a maior fração inteira do monitor (Hz, Hz/2, Hz/3…) que fica em cima ou logo abaixo do
/// FPS sustentado, com 2% de folga para o teto não ser alto demais. Num monitor de 180 Hz com FPS sustentado 45 → 45
/// (180/4). Num de 60 Hz com FPS 48 → 60 não cabe, 30 (60/2) cabe. Sempre ≥ um mínimo jogável.
pub fn teto_estavel(hz: u32, fps_sustentado: f64) -> Option<u32> {
    const MIN_JOGAVEL: u32 = 24;
    if hz == 0 || fps_sustentado < MIN_JOGAVEL as f64 {
        return None;
    }
    let alvo = fps_sustentado * 1.02;
    // As frações do monitor, da mais alta para a mais baixa; a primeira que cabe no alvo.
    let teto = (1..=8)
        .map(|d| hz as f64 / d as f64)
        .filter(|f| *f <= alvo && *f >= MIN_JOGAVEL as f64)
        .fold(None, |acc: Option<f64>, f| {
            Some(acc.map_or(f, |a| a.max(f)))
        });
    teto.map(|f| f.round() as u32)
}

/// O multiplicador inteiro do Frame Boost que, sobre o teto, fecha os Hz do monitor sem passar: 45 em 180 Hz → 4×. Nunca
/// acima de 4 (a GTX 1650 não acompanha mais que isso) nem abaixo de 1.
pub fn multiplicador_para_o_monitor(teto: u32, hz: u32) -> u8 {
    if teto == 0 {
        return 1;
    }
    ((hz / teto).clamp(1, 4)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_sustentado_e_o_que_o_jogo_entrega_quase_sempre() {
        // FiveM do dono: oscila, com picos. O P5 fica perto do piso, não da média.
        let a = [41.0, 44.0, 76.0, 50.0, 40.0, 68.0, 45.0, 42.0, 55.0, 43.0];
        let p5 = fps_sustentado(&a, 5.0).unwrap();
        assert!((40.0..=42.0).contains(&p5), "{}", p5);
        assert_eq!(
            fps_sustentado(&[60.0, 60.0], 5.0),
            None,
            "poucas amostras, sem palpite"
        );
    }

    #[test]
    fn teto_e_a_maior_fracao_do_monitor_que_cabe() {
        assert_eq!(teto_estavel(180, 45.0), Some(45), "180/4");
        assert_eq!(teto_estavel(180, 90.0), Some(90), "180/2");
        assert_eq!(
            teto_estavel(180, 200.0),
            Some(180),
            "o jogo passa do monitor: trava nos Hz"
        );
        assert_eq!(
            teto_estavel(60, 48.0),
            Some(30),
            "60 não cabe em 48, 30 cabe"
        );
        assert_eq!(teto_estavel(144, 50.0), Some(48), "144/3");
        assert_eq!(
            teto_estavel(180, 10.0),
            None,
            "abaixo do jogável, não trava"
        );
    }

    #[test]
    fn multiplicador_fecha_os_hz() {
        assert_eq!(multiplicador_para_o_monitor(45, 180), 4);
        assert_eq!(multiplicador_para_o_monitor(90, 180), 2);
        assert_eq!(
            multiplicador_para_o_monitor(30, 180),
            4,
            "180/30=6, mas o teto é 4×"
        );
        assert_eq!(multiplicador_para_o_monitor(180, 180), 1);
    }
}
