// O CURSOR NA ESCALA. Com o jogo ampliado, o cursor do Windows continua na posição da janela pequena. Aqui: a forma
// do cursor que a Duplicação de Área de Trabalho entrega (ela não grava o cursor na imagem) em BGRA para a placa
// desenhar ampliada, o mouse preso na janela do jogo (`ClipCursor`) e o cursor do sistema escondido enquanto a
// sobreposição está na tela. As duas guardas devolvem o que estava ao sair, até quando o gerador para por erro.

use windows::Win32::Foundation::RECT;
use windows::Win32::UI::Magnification::{MagInitialize, MagShowSystemCursor, MagUninitialize};
use windows::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetClipCursor, GetCursorInfo, CURSORINFO, CURSOR_SHOWING,
};

use super::ritmo::Retangulo;

/// Os tipos de `DXGI_OUTDUPL_POINTER_SHAPE_TYPE`.
pub const MONOCROMATICO: u32 = 1;
pub const COLORIDO: u32 = 2;
pub const COLORIDO_COM_MASCARA: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forma {
    pub largura: u32,
    pub altura: u32,
    /// BGRA, alfa direto (não pré-multiplicado).
    pub px: Vec<u8>,
    /// O ponto quente, a partir do canto da forma.
    pub ponta: (i32, i32),
}

/// Pura: a forma da captura em BGRA. Os pixels que INVERTEM a tela (o cursor de texto é quase todo assim) saem
/// brancos: a sobreposição não lê o que está por baixo, e a maioria dos jogos é escura.
pub fn forma_em_bgra(
    tipo: u32,
    largura: u32,
    altura: u32,
    passo: u32,
    ponta: (i32, i32),
    dados: &[u8],
) -> Option<Forma> {
    let (l, passo) = (largura as usize, passo as usize);
    let mut px = Vec::with_capacity(l * altura as usize * 4);
    match tipo {
        MONOCROMATICO => {
            // AND nas primeiras `altura/2` linhas, XOR nas seguintes; 1 bit por pixel, o mais alto primeiro.
            let a = altura as usize / 2;
            if dados.len() < passo * a * 2 {
                return None;
            }
            let bit = |linha: usize, x: usize| (dados[linha * passo + x / 8] >> (7 - x % 8)) & 1;
            for y in 0..a {
                for x in 0..l {
                    let cor = match (bit(y, x), bit(y + a, x)) {
                        (0, 0) => [0, 0, 0, 255],
                        (0, _) => [255, 255, 255, 255],
                        (_, 0) => [0, 0, 0, 0],
                        _ => [255, 255, 255, 255],
                    };
                    px.extend_from_slice(&cor);
                }
            }
            Some(Forma {
                largura,
                altura: a as u32,
                px,
                ponta,
            })
        }
        COLORIDO | COLORIDO_COM_MASCARA => {
            if dados.len() < passo * altura as usize {
                return None;
            }
            for y in 0..altura as usize {
                for x in 0..l {
                    let i = y * passo + x * 4;
                    let (b, g, r, alfa) = (dados[i], dados[i + 1], dados[i + 2], dados[i + 3]);
                    let cor = if tipo == COLORIDO {
                        [b, g, r, alfa]
                    } else if alfa == 0 {
                        // Com máscara, alfa 0 = troca pelo RGB.
                        [b, g, r, 255]
                    } else if (b, g, r) == (0, 0, 0) {
                        // Alfa cheio = XOR com a tela; XOR com preto não muda nada.
                        [0, 0, 0, 0]
                    } else {
                        [255, 255, 255, 255]
                    };
                    px.extend_from_slice(&cor);
                }
            }
            Some(Forma {
                largura,
                altura,
                px,
                ponta,
            })
        }
        _ => None,
    }
}

/// O cursor do sistema agora: a ponta (o ponto quente) na tela, se o jogo o mostra. Lido do Windows (`GetCursorInfo`) e
/// NÃO da Duplicação: com o cursor do sistema escondido pela Lupa (o nosso `CursorDoSistema`), a Duplicação pode
/// dar o ponteiro como invisível, e o cursor ampliado sumia junto (1.0.15, "meu mouse está sumindo dentro do FiveM").
/// O `GetCursorInfo` segue o que o JOGO pediu (`ShowCursor`/`SetCursor`), que é o que decide se há cursor.
pub fn no_sistema() -> Option<(i32, i32)> {
    let mut ci = CURSORINFO {
        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetCursorInfo(&mut ci) }.ok()?;
    visivel(ci.flags.0, !ci.hCursor.is_invalid()).then_some((ci.ptScreenPos.x, ci.ptScreenPos.y))
}

/// Pura: o jogo mostra cursor (o Windows diz "mostrando" e há uma forma escolhida; `SetCursor(NULL)` some).
pub fn visivel(flags: u32, tem_forma: bool) -> bool {
    flags & CURSOR_SHOWING.0 != 0 && tem_forma
}

/// Pura: onde a forma começa (o canto, como a Duplicação dá) na tela do monitor `origem`, pela ponta em coordenadas da
/// área de trabalho.
pub fn canto_no_monitor(ponta_na_tela: (i32, i32), ponta_da_forma: (i32, i32), origem: (i32, i32)) -> (i32, i32) {
    (
        ponta_na_tela.0 - ponta_da_forma.0 - origem.0,
        ponta_na_tela.1 - ponta_da_forma.1 - origem.1,
    )
}

/// Quando a linha do gerador não termina a tempo (presa na placa ou na troca de modo de vídeo), as guardas dela não
/// rodam: o cursor ficava escondido e preso no recorte da janela pequena até o Otimiza fechar. No log do sócio
/// (2026-11-21 16:18): "refazendo (o Windows trocou o modo de vídeo)" com a escala ligada e, 6 s depois, "a linha não
/// terminou em 5 s". Chamado por quem desliga, de outra linha: mostra o cursor e solta o recorte.
pub fn devolver_de_fora() {
    devolver_na_abertura();
    unsafe {
        let _ = ClipCursor(None);
    }
}

/// O mouse preso na área do jogo enquanto a sobreposição está na tela. Devolve o recorte que havia antes (o jogo pode
/// ter o dele), ao soltar e ao sair.
pub struct MousePreso {
    anterior: Option<RECT>,
}

impl MousePreso {
    pub fn novo() -> MousePreso {
        MousePreso { anterior: None }
    }

    /// Prende (de novo: trocar de janela no Windows desfaz o recorte).
    pub fn prender(&mut self, jogo_na_tela: Retangulo) {
        unsafe {
            if self.anterior.is_none() {
                let mut r = RECT::default();
                if GetClipCursor(&mut r).is_ok() {
                    self.anterior = Some(r);
                }
            }
            let r = RECT {
                left: jogo_na_tela.x,
                top: jogo_na_tela.y,
                right: jogo_na_tela.x + jogo_na_tela.largura,
                bottom: jogo_na_tela.y + jogo_na_tela.altura,
            };
            let _ = ClipCursor(Some(&r));
        }
    }

    pub fn soltar(&mut self) {
        if let Some(r) = self.anterior.take() {
            unsafe {
                let _ = ClipCursor(Some(&r));
            }
        }
    }
}

impl Drop for MousePreso {
    fn drop(&mut self) {
        self.soltar();
    }
}

/// O cursor do sistema escondido (API de Lupa do Windows), só enquanto o cursor ampliado está na tela. Vale para a
/// linha de execução que inicializou: a do gerador.
pub struct CursorDoSistema {
    iniciado: bool,
    escondido: bool,
}

impl CursorDoSistema {
    pub fn novo() -> CursorDoSistema {
        CursorDoSistema {
            iniciado: unsafe { MagInitialize().as_bool() },
            escondido: false,
        }
    }

    pub fn esconder(&mut self, esconder: bool) {
        if !self.iniciado || esconder == self.escondido {
            return;
        }
        if unsafe { MagShowSystemCursor(!esconder) }.as_bool() {
            self.escondido = esconder;
        }
    }
}

impl Drop for CursorDoSistema {
    fn drop(&mut self) {
        self.esconder(false);
        if self.iniciado {
            unsafe {
                let _ = MagUninitialize();
            }
        }
    }
}

/// Na abertura do Otimiza: se a sessão anterior morreu com o cursor escondido, ele volta. Mostrar um cursor que já
/// está à vista não muda nada.
pub fn devolver_na_abertura() {
    let mut c = CursorDoSistema::novo();
    c.escondido = true;
    c.esconder(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monocromatico_tem_as_quatro_combinacoes() {
        // 8x2 visível (4 linhas: 2 de AND, 2 de XOR), passo de 1 byte.
        // Linha 0: AND 0000_1111, XOR 0101_0101 → preto, branco, preto, branco, transparente, inverte...
        let dados = [0b0000_1111, 0xFF, 0b0101_0101, 0x00];
        let f = forma_em_bgra(MONOCROMATICO, 8, 4, 1, (0, 0), &dados).unwrap();
        assert_eq!((f.largura, f.altura), (8, 2));
        let px = |x: usize, y: usize| &f.px[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4];
        assert_eq!(px(0, 0), [0, 0, 0, 255]);
        assert_eq!(px(1, 0), [255, 255, 255, 255]);
        assert_eq!(px(4, 0), [0, 0, 0, 0]);
        assert_eq!(px(5, 0), [255, 255, 255, 255]);
        // Linha 1: AND tudo 1, XOR tudo 0 → transparente.
        assert!((0..8).all(|x| px(x, 1)[3] == 0));
    }

    #[test]
    fn colorido_passa_direto_respeitando_o_passo() {
        // 1x2 com passo de 8 bytes (sobra no fim da linha).
        let dados = [1, 2, 3, 128, 9, 9, 9, 9, 4, 5, 6, 255, 9, 9, 9, 9];
        let f = forma_em_bgra(COLORIDO, 1, 2, 8, (3, 4), &dados).unwrap();
        assert_eq!(f.px, vec![1, 2, 3, 128, 4, 5, 6, 255]);
        assert_eq!(f.ponta, (3, 4));
    }

    #[test]
    fn com_mascara_troca_ou_inverte() {
        let dados = [10, 20, 30, 0, 0, 0, 0, 255, 50, 50, 50, 255];
        let f = forma_em_bgra(COLORIDO_COM_MASCARA, 3, 1, 12, (0, 0), &dados).unwrap();
        assert_eq!(&f.px[0..4], [10, 20, 30, 255]);
        assert_eq!(&f.px[4..8], [0, 0, 0, 0]);
        assert_eq!(&f.px[8..12], [255, 255, 255, 255]);
    }

    #[test]
    fn cursor_some_so_quando_o_jogo_esconde() {
        assert!(visivel(CURSOR_SHOWING.0, true));
        assert!(!visivel(0, true), "ShowCursor(FALSE) do jogo");
        assert!(!visivel(CURSOR_SHOWING.0, false), "SetCursor(NULL) do jogo");
    }

    #[test]
    fn a_ponta_vira_o_canto_no_monitor() {
        // Segundo monitor à direita (origem 1920,0), ponta da seta em (5,3).
        assert_eq!(canto_no_monitor((2020, 500), (5, 3), (1920, 0)), (95, 497));
    }

    #[test]
    fn dados_curtos_nao_viram_forma() {
        assert_eq!(forma_em_bgra(COLORIDO, 32, 32, 128, (0, 0), &[0; 16]), None);
        assert_eq!(forma_em_bgra(MONOCROMATICO, 32, 64, 4, (0, 0), &[0; 16]), None);
        assert_eq!(forma_em_bgra(99, 1, 1, 4, (0, 0), &[0; 4]), None);
    }
}
