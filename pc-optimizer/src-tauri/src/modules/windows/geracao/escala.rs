// ESCALA: o jogo renderiza menor que o monitor e o gerador amplia (FSR 1). Aqui só a conta, sem placa de vídeo: o
// tamanho de render de cada nível, onde a imagem ampliada cai na tela (com barras quando a proporção não é a do
// monitor) e a conversão do cursor entre a tela e a janela do jogo.
//
// Os níveis têm os nomes e as proporções do FSR, que o jogador já conhece do menu dos jogos.

use serde::{Deserialize, Serialize};

use super::ritmo::Retangulo;

/// Menos que isso por eixo não é jogo, é janela minimizada ou erro de leitura.
pub const MINIMO_PX: i32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nivel {
    UltraQualidade,
    Qualidade,
    Equilibrado,
    Desempenho,
}

impl Nivel {
    pub const TODOS: [Nivel; 4] = [
        Nivel::UltraQualidade,
        Nivel::Qualidade,
        Nivel::Equilibrado,
        Nivel::Desempenho,
    ];

    /// Quanto a tela é maior que o render, por eixo.
    pub fn fator(self) -> f64 {
        match self {
            Nivel::UltraQualidade => 1.3,
            Nivel::Qualidade => 1.5,
            Nivel::Equilibrado => 1.7,
            Nivel::Desempenho => 2.0,
        }
    }
}

/// Pura: o tamanho em que o jogo deve renderizar para o nível, num destino de `largura`×`altura`.
pub fn resolucao_de_render(nivel: Nivel, largura: i32, altura: i32) -> (i32, i32) {
    let f = nivel.fator();
    (
        ((largura as f64 / f).round() as i32).max(MINIMO_PX),
        ((altura as f64 / f).round() as i32).max(MINIMO_PX),
    )
}

/// Pura: onde a imagem do jogo ampliada cai dentro de `tela`, mantendo a proporção do jogo e centrada (o que sobra
/// vira barra). `None` com o jogo pequeno demais para ser jogo.
pub fn encaixe(jogo_largura: i32, jogo_altura: i32, tela: Retangulo) -> Option<Retangulo> {
    if jogo_largura < MINIMO_PX || jogo_altura < MINIMO_PX || tela.largura <= 0 || tela.altura <= 0
    {
        return None;
    }
    let escala =
        (tela.largura as f64 / jogo_largura as f64).min(tela.altura as f64 / jogo_altura as f64);
    let largura = ((jogo_largura as f64 * escala).round() as i32).min(tela.largura);
    let altura = ((jogo_altura as f64 * escala).round() as i32).min(tela.altura);
    Some(Retangulo {
        x: tela.x + (tela.largura - largura) / 2,
        y: tela.y + (tela.altura - altura) / 2,
        largura,
        altura,
    })
}

/// Pura: um ponto da tela (dentro da imagem ampliada `destino`) no ponto correspondente da janela do jogo. `None` em
/// cima das barras.
pub fn tela_para_jogo(x: i32, y: i32, destino: Retangulo, jogo: Retangulo) -> Option<(i32, i32)> {
    if destino.largura <= 0
        || destino.altura <= 0
        || x < destino.x
        || y < destino.y
        || x >= destino.x + destino.largura
        || y >= destino.y + destino.altura
    {
        return None;
    }
    let u = (x - destino.x) as f64 / destino.largura as f64;
    let v = (y - destino.y) as f64 / destino.altura as f64;
    Some((
        jogo.x + (u * jogo.largura as f64).floor() as i32,
        jogo.y + (v * jogo.altura as f64).floor() as i32,
    ))
}

/// Pura: um ponto da janela do jogo no ponto da tela onde ele aparece ampliado (para desenhar o cursor).
pub fn jogo_para_tela(x: i32, y: i32, jogo: Retangulo, destino: Retangulo) -> (i32, i32) {
    let u = (x - jogo.x) as f64 / jogo.largura.max(1) as f64;
    let v = (y - jogo.y) as f64 / jogo.altura.max(1) as f64;
    (
        destino.x + (u * destino.largura as f64).round() as i32,
        destino.y + (v * destino.altura as f64).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, largura: i32, altura: i32) -> Retangulo {
        Retangulo {
            x,
            y,
            largura,
            altura,
        }
    }

    #[test]
    fn niveis_do_fsr_em_1080p() {
        assert_eq!(
            resolucao_de_render(Nivel::UltraQualidade, 1920, 1080),
            (1477, 831)
        );
        assert_eq!(
            resolucao_de_render(Nivel::Qualidade, 1920, 1080),
            (1280, 720)
        );
        assert_eq!(
            resolucao_de_render(Nivel::Equilibrado, 1920, 1080),
            (1129, 635)
        );
        assert_eq!(
            resolucao_de_render(Nivel::Desempenho, 1920, 1080),
            (960, 540)
        );
    }

    #[test]
    fn os_niveis_sobem_de_ganho_na_ordem() {
        let pixels: Vec<i32> = Nivel::TODOS
            .iter()
            .map(|n| {
                let (l, a) = resolucao_de_render(*n, 2560, 1440);
                l * a
            })
            .collect();
        assert!(pixels.windows(2).all(|p| p[0] > p[1]));
    }

    #[test]
    fn mesma_proporcao_enche_a_tela() {
        assert_eq!(
            encaixe(1280, 720, r(0, 0, 1920, 1080)),
            Some(r(0, 0, 1920, 1080))
        );
    }

    #[test]
    fn jogo_4_por_3_ganha_barras_dos_lados() {
        let e = encaixe(1024, 768, r(0, 0, 1920, 1080)).unwrap();
        assert_eq!(e, r(240, 0, 1440, 1080));
    }

    #[test]
    fn monitor_ultrawide_com_jogo_16_por_9() {
        let e = encaixe(1280, 720, r(0, 0, 3440, 1440)).unwrap();
        assert_eq!(e.altura, 1440);
        assert_eq!(e.largura, 2560);
        assert_eq!(e.x, 440);
    }

    #[test]
    fn segundo_monitor_conserva_a_origem() {
        let e = encaixe(1280, 720, r(1920, 0, 1920, 1080)).unwrap();
        assert_eq!(e.x, 1920);
    }

    #[test]
    fn janela_minimizada_nao_encaixa() {
        assert_eq!(encaixe(0, 0, r(0, 0, 1920, 1080)), None);
        assert_eq!(encaixe(32, 32, r(0, 0, 1920, 1080)), None);
    }

    #[test]
    fn cursor_vai_e_volta() {
        let jogo = r(100, 50, 1280, 720);
        let destino = r(0, 0, 1920, 1080);
        assert_eq!(tela_para_jogo(960, 540, destino, jogo), Some((740, 410)));
        assert_eq!(jogo_para_tela(740, 410, jogo, destino), (960, 540));
        assert_eq!(tela_para_jogo(0, 0, destino, jogo), Some((100, 50)));
    }

    #[test]
    fn cursor_na_barra_nao_vai_para_o_jogo() {
        let destino = r(240, 0, 1440, 1080);
        let jogo = r(0, 0, 1024, 768);
        assert_eq!(tela_para_jogo(100, 500, destino, jogo), None);
        assert_eq!(tela_para_jogo(1680, 500, destino, jogo), None);
    }
}
