// As réguas do laboratório do Frame Boost (1.0.7, Onda 1). Puras e só no processador: recebem quadros BGRA (4 bytes por
// pixel) e devolvem números comparáveis entre versões. O gabarito é o próprio jogo: grava-se a 120–144 FPS, gera-se o
// quadro do meio a partir do 1º e do 3º, e compara-se com o 2º de verdade (`gpu::qualidade`).
//
// O que o cliente vê e cada régua mede:
// - borrão e fantasma no que se mexe → `fantasma` (borda que aparece no gerado e não existe no real);
// - a mira e a HUD tremendo → `erro_na_mascara` sobre os pixels parados (`mascara_de_parados`);
// - piscar → `cintilacao` (o gerado foge da mistura dos dois reais mais do que o real foge);
// - tranco → `ritmo` sobre os intervalos entre quadros exibidos.
// A linha de base de toda comparação é "repetir o quadro real": gerar só vale onde ganha dela.

/// Luminância (0–255) de um pixel BGRA.
fn luma(px: &[u8], i: usize) -> f64 {
    0.114 * px[i * 4] as f64 + 0.587 * px[i * 4 + 1] as f64 + 0.299 * px[i * 4 + 2] as f64
}

/// PSNR em dB sobre os três canais de cor. Idênticos: 99 (o teto, para a média não virar infinito).
pub fn psnr(a: &[u8], b: &[u8]) -> f64 {
    let (mut soma, mut n) = (0.0f64, 0usize);
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        for c in 0..3 {
            let d = pa[c] as f64 - pb[c] as f64;
            soma += d * d;
            n += 1;
        }
    }
    if n == 0 || soma == 0.0 {
        return 99.0;
    }
    (10.0 * (255.0f64 * 255.0 / (soma / n as f64)).log10()).min(99.0)
}

/// SSIM da luminância em janelas de 8×8 (média das janelas). 1 = idênticos.
pub fn ssim(a: &[u8], b: &[u8], largura: u32, altura: u32) -> f64 {
    let (l, h) = (largura as usize, altura as usize);
    let (c1, c2) = ((0.01f64 * 255.0).powi(2), (0.03f64 * 255.0).powi(2));
    let (mut soma, mut janelas) = (0.0, 0usize);
    for by in (0..h.saturating_sub(7)).step_by(8) {
        for bx in (0..l.saturating_sub(7)).step_by(8) {
            let (mut ma, mut mb) = (0.0, 0.0);
            for y in by..by + 8 {
                for x in bx..bx + 8 {
                    ma += luma(a, y * l + x);
                    mb += luma(b, y * l + x);
                }
            }
            ma /= 64.0;
            mb /= 64.0;
            let (mut va, mut vb, mut cov) = (0.0, 0.0, 0.0);
            for y in by..by + 8 {
                for x in bx..bx + 8 {
                    let (da, db) = (luma(a, y * l + x) - ma, luma(b, y * l + x) - mb);
                    va += da * da;
                    vb += db * db;
                    cov += da * db;
                }
            }
            va /= 63.0;
            vb /= 63.0;
            cov /= 63.0;
            soma += ((2.0 * ma * mb + c1) * (2.0 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2));
            janelas += 1;
        }
    }
    if janelas == 0 {
        1.0
    } else {
        soma / janelas as f64
    }
}

/// Os pixels que ficaram parados nos dois reais (diferença de luminância até `tolerancia`) enquanto a cena em volta
/// mexeu: a HUD e a mira. `true` = parado.
pub fn mascara_de_parados(a: &[u8], b: &[u8], tolerancia: f64) -> Vec<bool> {
    (0..a.len() / 4).map(|i| (luma(a, i) - luma(b, i)).abs() <= tolerancia).collect()
}

/// Um retângulo no centro da tela, com `fracao` da área (0,03 = 3%): a mira.
pub fn mascara_do_centro(largura: u32, altura: u32, fracao: f64) -> Vec<bool> {
    let (l, h) = (largura as f64, altura as f64);
    let lado_l = (l * fracao.sqrt()).round().max(1.0);
    let lado_h = (h * fracao.sqrt()).round().max(1.0);
    let (x0, y0) = ((l - lado_l) / 2.0, (h - lado_h) / 2.0);
    (0..(largura * altura) as usize)
        .map(|i| {
            let (x, y) = ((i % largura as usize) as f64, (i / largura as usize) as f64);
            x >= x0 && x < x0 + lado_l && y >= y0 && y < y0 + lado_h
        })
        .collect()
}

/// Erro médio absoluto da luminância (0–255) só nos pixels da máscara. Sem pixel na máscara: 0.
pub fn erro_na_mascara(gerado: &[u8], real: &[u8], mascara: &[bool]) -> f64 {
    let (mut soma, mut n) = (0.0, 0usize);
    for (i, dentro) in mascara.iter().enumerate() {
        if *dentro {
            soma += (luma(gerado, i) - luma(real, i)).abs();
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        soma / n as f64
    }
}

/// Gradiente horizontal + vertical da luminância em cada pixel (borda).
fn bordas(px: &[u8], largura: u32, altura: u32) -> Vec<f64> {
    let (l, h) = (largura as usize, altura as usize);
    let mut g = vec![0.0; l * h];
    for y in 1..h.saturating_sub(1) {
        for x in 1..l.saturating_sub(1) {
            let i = y * l + x;
            g[i] = (luma(px, i + 1) - luma(px, i - 1)).abs() + (luma(px, i + l) - luma(px, i - l)).abs();
        }
    }
    g
}

/// O fantasma: borda que o gerado tem e o real não tem (o contorno dobrado em volta do que se mexe), em média por pixel.
/// 0 = nenhuma borda inventada.
pub fn fantasma(gerado: &[u8], real: &[u8], largura: u32, altura: u32) -> f64 {
    let (bg, br) = (bordas(gerado, largura, altura), bordas(real, largura, altura));
    if bg.is_empty() {
        return 0.0;
    }
    bg.iter().zip(&br).map(|(g, r)| (g - r).max(0.0)).sum::<f64>() / bg.len() as f64
}

/// A cintilação: quanto o gerado se afasta da mistura dos dois reais além do que o real do meio se afasta dela. Positivo
/// = o gerado inventa variação que o jogo não tem (pisca).
pub fn cintilacao(gerado: &[u8], real: &[u8], antes: &[u8], depois: &[u8]) -> f64 {
    let n = gerado.len() / 4;
    if n == 0 {
        return 0.0;
    }
    let (mut g, mut r) = (0.0, 0.0);
    for i in 0..n {
        let media = (luma(antes, i) + luma(depois, i)) / 2.0;
        g += (luma(gerado, i) - media).abs();
        r += (luma(real, i) - media).abs();
    }
    (g - r) / n as f64
}

/// O ritmo dos quadros exibidos, pelos intervalos entre eles (ms).
#[derive(Debug, Clone, PartialEq)]
pub struct Ritmo {
    pub p50_ms: f64,
    pub p99_ms: f64,
    /// Fração dos intervalos acima de 1,5× a mediana: o tranco que o olho vê.
    pub trancos: f64,
}

pub fn ritmo(intervalos_ms: &[f64]) -> Option<Ritmo> {
    let mut v: Vec<f64> = intervalos_ms.iter().copied().filter(|x| x.is_finite() && *x > 0.0).collect();
    if v.len() < 2 {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let em = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    let p50 = em(0.5);
    let trancos = v.iter().filter(|x| **x > 1.5 * p50).count() as f64 / v.len() as f64;
    Some(Ritmo { p50_ms: p50, p99_ms: em(0.99), trancos })
}

/// O laudo de um quadro gerado contra o real do meio, com a linha de base "repetir o real mais próximo".
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Laudo {
    pub ssim: f64,
    pub psnr: f64,
    pub ssim_repetir: f64,
    pub erro_hud: f64,
    pub erro_hud_repetir: f64,
    pub erro_mira: f64,
    pub erro_mira_repetir: f64,
    pub fantasma: f64,
    pub cintilacao: f64,
}

/// Pura: o laudo completo. `antes` e `depois` são os reais; `real` o do meio; `gerado` o do Frame Boost.
pub fn laudo(gerado: &[u8], real: &[u8], antes: &[u8], depois: &[u8], largura: u32, altura: u32) -> Laudo {
    let parados = mascara_de_parados(antes, depois, 2.0);
    let mira = mascara_do_centro(largura, altura, 0.03);
    // "Repetir": o real mais próximo no tempo (o de antes, no quadro do meio).
    Laudo {
        ssim: ssim(gerado, real, largura, altura),
        psnr: psnr(gerado, real),
        ssim_repetir: ssim(antes, real, largura, altura),
        erro_hud: erro_na_mascara(gerado, real, &parados),
        erro_hud_repetir: erro_na_mascara(antes, real, &parados),
        erro_mira: erro_na_mascara(gerado, real, &mira),
        erro_mira_repetir: erro_na_mascara(antes, real, &mira),
        fantasma: fantasma(gerado, real, largura, altura),
        cintilacao: cintilacao(gerado, real, antes, depois),
    }
}

#[cfg(test)]
pub mod cenas {
    //! Cenas sintéticas BGRA para o laboratório: as mesmas que o WARP roda nos shaders reais.

    /// Fundo com textura (para o fluxo ter o que seguir), um quadrado claro em `x_quadrado` e, se `mira`, uma cruz
    /// branca fixa no centro. `deslocamento_fundo` faz panorâmica.
    /// Ruído de valor suave e determinístico, em duas escalas (60–190).
    fn textura(x: i32, y: i32) -> u8 {
        fn hash(x: i32, y: i32) -> f64 {
            let h = (x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263)).wrapping_mul(1_274_126_177);
            ((h >> 8) & 0xff) as f64 / 255.0
        }
        fn suave(x: i32, y: i32, passo: i32) -> f64 {
            let (gx, gy) = (x.div_euclid(passo), y.div_euclid(passo));
            let (fx, fy) = (x.rem_euclid(passo) as f64 / passo as f64, y.rem_euclid(passo) as f64 / passo as f64);
            let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
            let a = hash(gx, gy) * (1.0 - sx) + hash(gx + 1, gy) * sx;
            let b = hash(gx, gy + 1) * (1.0 - sx) + hash(gx + 1, gy + 1) * sx;
            a * (1.0 - sy) + b * sy
        }
        (60.0 + 90.0 * suave(x, y, 23) + 40.0 * suave(x + 1000, y + 1000, 7)) as u8
    }

    pub fn quadro(largura: u32, altura: u32, x_quadrado: i32, deslocamento_fundo: i32, mira: bool) -> Vec<u8> {
        let (l, h) = (largura as i32, altura as i32);
        let mut px = vec![0u8; (largura * altura * 4) as usize];
        for y in 0..h {
            for x in 0..l {
                let i = ((y * l + x) * 4) as usize;
                let fx = x + deslocamento_fundo;
                // Textura sem repetição (manchas suaves de tamanhos diferentes): um xadrez se repete a cada 16 px e
                // engana qualquer fluxo óptico, o jogo não.
                let v = textura(fx, y);
                px[i] = v;
                px[i + 1] = v;
                px[i + 2] = v;
                px[i + 3] = 255;
                let no_quadrado = x >= x_quadrado && x < x_quadrado + 24 && y >= h / 2 - 12 && y < h / 2 + 12;
                if no_quadrado {
                    px[i] = 40;
                    px[i + 1] = 200;
                    px[i + 2] = 230;
                }
                let (cx, cy) = (l / 2, h / 2);
                if mira && (((x - cx).abs() <= 6 && (y - cy).abs() <= 1) || ((y - cy).abs() <= 6 && (x - cx).abs() <= 1)) {
                    px[i] = 255;
                    px[i + 1] = 255;
                    px[i + 2] = 255;
                }
            }
        }
        px
    }
}

#[cfg(test)]
mod tests {
    use super::cenas::quadro;
    use super::*;

    const L: u32 = 160;
    const H: u32 = 90;

    #[test]
    fn identicos_dao_o_teto() {
        let a = quadro(L, H, 40, 0, true);
        assert_eq!(psnr(&a, &a), 99.0);
        assert!((ssim(&a, &a, L, H) - 1.0).abs() < 1e-9);
        assert_eq!(fantasma(&a, &a, L, H), 0.0);
        assert_eq!(erro_na_mascara(&a, &a, &mascara_do_centro(L, H, 0.03)), 0.0);
    }

    #[test]
    fn o_quadro_do_meio_certo_ganha_de_repetir() {
        // Quadrado andando 20 px por quadro sobre fundo parado: o real do meio está em 60.
        let (antes, meio, depois) = (quadro(L, H, 40, 0, true), quadro(L, H, 60, 0, true), quadro(L, H, 80, 0, true));
        let certo = laudo(&meio, &meio, &antes, &depois, L, H);
        assert!(certo.ssim > certo.ssim_repetir, "o gerado perfeito ganha de repetir: {certo:?}");
        // A mistura dos dois reais cria o fantasma (dois quadrados meio transparentes).
        let mistura: Vec<u8> = antes.iter().zip(&depois).map(|(a, b)| ((*a as u16 + *b as u16) / 2) as u8).collect();
        let misturado = laudo(&mistura, &meio, &antes, &depois, L, H);
        assert!(misturado.fantasma > certo.fantasma, "a mistura inventa borda: {misturado:?}");
    }

    #[test]
    fn a_mira_tremida_aparece_no_erro_da_mira() {
        // Panorâmica do fundo com a mira fixa: um gerado que arrasta a mira junto com o fundo erra no centro.
        let (antes, meio, depois) = (quadro(L, H, 40, 0, true), quadro(L, H, 40, 6, true), quadro(L, H, 40, 12, true));
        let arrastou = quadro(L, H, 40, 6, false); // a mira sumiu (foi levada com o fundo)
        let ruim = laudo(&arrastou, &meio, &antes, &depois, L, H);
        let bom = laudo(&meio, &meio, &antes, &depois, L, H);
        assert!(ruim.erro_mira > 10.0 && bom.erro_mira == 0.0, "{} × {}", ruim.erro_mira, bom.erro_mira);
        // A mira fica parada nos dois reais: a máscara de parados a acha.
        let parados = mascara_de_parados(&antes, &depois, 2.0);
        assert!(parados[((H / 2) * L + L / 2) as usize], "o centro da mira é parado");
    }

    #[test]
    fn corte_de_cena_derruba_tudo() {
        let a = quadro(L, H, 40, 0, true);
        let preto = vec![0u8; a.len()];
        assert!(ssim(&a, &preto, L, H) < 0.2);
        assert!(psnr(&a, &preto) < 15.0);
    }

    #[test]
    fn ritmo_conta_os_trancos() {
        let liso = vec![6.94; 100];
        let r = ritmo(&liso).unwrap();
        assert!((r.p50_ms - 6.94).abs() < 1e-9 && r.trancos == 0.0);
        let mut com_tranco = vec![6.94; 98];
        com_tranco.extend([13.9, 20.8]);
        let r = ritmo(&com_tranco).unwrap();
        assert!((r.trancos - 0.02).abs() < 1e-9, "{r:?}");
        assert!(r.p99_ms > 13.0);
        assert!(ritmo(&[6.9]).is_none());
    }

    #[test]
    fn mascara_do_centro_tem_a_area_pedida() {
        let m = mascara_do_centro(1000, 1000, 0.03);
        let area = m.iter().filter(|x| **x).count() as f64 / 1e6;
        assert!((area - 0.03).abs() < 0.002, "{area}");
    }
}
