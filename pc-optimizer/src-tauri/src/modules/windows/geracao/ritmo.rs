// ---------------------------------------------------------------------------
// O RITMO DO GERADOR DE QUADROS — a parte que decide QUANDO mostrar cada quadro
//
// Um quadro gerado mostrado na hora errada é pior que quadro nenhum: a imagem
// anda, para, anda — o "judder" que faz a geração parecer engasgo. Por isso a
// agenda é função pura e testada, separada da GPU.
//
// Como funciona, para multiplicador M e intervalo real estimado d:
//
//   chega o quadro real N (instante a)
//     a + 0·d/M   → gerado t = 1/M   (entre N-1 e N)
//     a + 1·d/M   → gerado t = 2/M
//     ...
//     a + (M-1)·d/M → o próprio quadro real N
//
// O quadro real sai atrasado (M-1)/M de um intervalo. É o preço da
// interpolação, e é exatamente o atraso que o laboratório declara como
// ESTIMADO. Se o próximo quadro real chega antes de a agenda terminar, o que
// sobrou é descartado: atraso acumulado nunca vira fila.
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fase {
    /// Quadro interpolado na posição `t` entre o real anterior (0) e o atual (1).
    Gerado(f32),
    /// O quadro real atual, sem interpolação.
    Real,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Apresentacao {
    /// Segundos no relógio de alta resolução.
    pub instante: f64,
    pub fase: Fase,
}

/// A agenda de apresentações para um quadro real que acabou de chegar.
pub fn agenda(chegada: f64, intervalo: f64, multiplicador: u8) -> Vec<Apresentacao> {
    let m = multiplicador.clamp(1, 8) as usize;
    if m == 1 || intervalo <= 0.0 {
        return vec![Apresentacao { instante: chegada, fase: Fase::Real }];
    }
    let passo = intervalo / m as f64;
    let mut v: Vec<Apresentacao> = (1..m)
        .map(|k| Apresentacao { instante: chegada + (k - 1) as f64 * passo, fase: Fase::Gerado(k as f32 / m as f32) })
        .collect();
    v.push(Apresentacao { instante: chegada + (m - 1) as f64 * passo, fase: Fase::Real });
    v
}

/// O multiplicador que cabe na tela.
///
/// Quadro gerado além da taxa do monitor não aparece: vira descarte ou rasgo e
/// custa placa de vídeo à toa. Com o jogo a 96 FPS num monitor de 180 Hz, 2×
/// daria 192 — então fica 1× (só o real) até o jogo cair abaixo de 90.
pub fn multiplicador_que_cabe(pedido: u8, intervalo: f64, hz: u32) -> u8 {
    if hz == 0 || intervalo <= 0.0 {
        return pedido.max(1);
    }
    let cabe = (hz as f64 * intervalo * 1.03).floor() as u8;
    pedido.min(cabe).max(1)
}

/// Acima desta fração de imagem ruim, o quadro gerado não sai.
pub const FRACAO_RUIM_MAXIMA: f32 = 0.10;

/// O quadro gerado pode ir à tela?
pub fn quadro_aprovado(fracao_ruim: Option<f32>) -> bool {
    fracao_ruim.is_some_and(|f| f.is_finite() && f <= FRACAO_RUIM_MAXIMA)
}

/// Estimativa do intervalo entre quadros reais.
///
/// Média exponencial com rejeição de extremos: uma tela de carregamento de
/// dois segundos não pode virar "o jogo roda a 0,5 FPS" e espalhar quadros
/// gerados por dois segundos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intervalo {
    pub estimado: Option<f64>,
    pub ultimo_real: Option<f64>,
}

impl Default for Intervalo {
    fn default() -> Self {
        Intervalo { estimado: None, ultimo_real: None }
    }
}

/// Acima disto (menos de 20 FPS reais) não se gera nada: a interpolação entre
/// quadros tão distantes vira borrão, e o atraso dobra.
pub const INTERVALO_MAXIMO: f64 = 1.0 / 20.0;
/// Abaixo disto (mais de 500 FPS) é duplicata ou ruído de captura.
pub const INTERVALO_MINIMO: f64 = 1.0 / 500.0;

impl Intervalo {
    /// Registra a chegada de um quadro real. Devolve o intervalo medido.
    pub fn registrar(&mut self, chegada: f64) -> Option<f64> {
        let medido = self.ultimo_real.map(|u| chegada - u);
        self.ultimo_real = Some(chegada);
        let d = medido?;
        if !(INTERVALO_MINIMO..=INTERVALO_MAXIMO).contains(&d) {
            // Pausa longa zera a estimativa: ao voltar, recomeça do zero.
            if d > INTERVALO_MAXIMO {
                self.estimado = None;
            }
            return Some(d);
        }
        self.estimado = Some(match self.estimado {
            None => d,
            // Mudança grande de ritmo (entrou numa área pesada) segue rápido;
            // oscilação pequena é suavizada.
            Some(e) if (d - e).abs() > e * 0.5 => d,
            Some(e) => e * 0.8 + d * 0.2,
        });
        Some(d)
    }

    pub fn pode_gerar(&self) -> bool {
        self.estimado.is_some()
    }
}

/// Retângulo em pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retangulo {
    pub x: i32,
    pub y: i32,
    pub largura: i32,
    pub altura: i32,
}

/// O pedaço da imagem do monitor que é a área do jogo.
///
/// `cliente` e `monitor` em coordenadas da área de trabalho. Devolve o recorte
/// em coordenadas do monitor, já limitado a ele. `None` quando o jogo está
/// fora do monitor ou pequeno demais para valer a pena.
pub fn recorte(cliente: Retangulo, monitor: Retangulo) -> Option<Retangulo> {
    let x0 = cliente.x.max(monitor.x);
    let y0 = cliente.y.max(monitor.y);
    let x1 = (cliente.x + cliente.largura).min(monitor.x + monitor.largura);
    let y1 = (cliente.y + cliente.altura).min(monitor.y + monitor.altura);
    let (largura, altura) = (x1 - x0, y1 - y0);
    if largura < 320 || altura < 200 {
        return None;
    }
    Some(Retangulo { x: x0 - monitor.x, y: y0 - monitor.y, largura, altura })
}

/// Tamanhos das texturas de trabalho: 1/4 e 1/16 da imagem (mínimo 1 pixel).
pub fn niveis(largura: i32, altura: i32) -> ((u32, u32), (u32, u32), (u32, u32)) {
    let d = |v: i32, f: i32| ((v + f - 1) / f).max(1) as u32;
    ((d(largura, 4), d(altura, 4)), (d(largura, 8), d(altura, 8)), (d(largura, 16), d(altura, 16)))
}

/// Contadores do gerador, para a tela e para o laboratório.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Contadores {
    pub reais: u64,
    pub gerados: u64,
    pub descartados: u64,
    /// Quadros reais em que a geração foi recusada pela nota (movimento que
    /// não dá para interpolar sem inventar).
    #[serde(default)]
    pub recusados: u64,
    /// Tempo médio de GPU por quadro gerado, em ms (do envio ao fim da cópia).
    pub custo_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agenda_2x_mostra_gerado_na_chegada_e_real_na_metade() {
        let a = agenda(10.0, 1.0 / 60.0, 2);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0], Apresentacao { instante: 10.0, fase: Fase::Gerado(0.5) });
        assert_eq!(a[1].fase, Fase::Real);
        assert!((a[1].instante - (10.0 + 1.0 / 120.0)).abs() < 1e-9);
    }

    #[test]
    fn agenda_3x_espacamento_uniforme() {
        let d = 0.03;
        let a = agenda(0.0, d, 3);
        let fases: Vec<Fase> = a.iter().map(|x| x.fase).collect();
        assert_eq!(fases, vec![Fase::Gerado(1.0 / 3.0), Fase::Gerado(2.0 / 3.0), Fase::Real]);
        assert!((a[1].instante - 0.01).abs() < 1e-9);
        assert!((a[2].instante - 0.02).abs() < 1e-9);
    }

    #[test]
    fn multiplicador_1_so_repassa_o_real() {
        assert_eq!(agenda(5.0, 0.016, 1), vec![Apresentacao { instante: 5.0, fase: Fase::Real }]);
        assert_eq!(agenda(5.0, 0.0, 3).len(), 1);
    }

    #[test]
    fn intervalo_suaviza_e_segue_mudanca_grande() {
        let mut i = Intervalo::default();
        assert_eq!(i.registrar(0.0), None);
        i.registrar(1.0 / 60.0);
        assert!((i.estimado.unwrap() - 1.0 / 60.0).abs() < 1e-9);
        // Oscilação pequena: suavizada.
        i.registrar(1.0 / 60.0 + 0.018);
        let e = i.estimado.unwrap();
        assert!(e > 1.0 / 60.0 && e < 0.018);
        // Queda para 30 FPS: segue na hora.
        let agora = i.ultimo_real.unwrap();
        i.registrar(agora + 1.0 / 30.0);
        assert!((i.estimado.unwrap() - 1.0 / 30.0).abs() < 1e-9);
    }

    #[test]
    fn pausa_longa_zera_e_nao_gera() {
        let mut i = Intervalo::default();
        i.registrar(0.0);
        i.registrar(0.016);
        assert!(i.pode_gerar());
        i.registrar(2.0);
        assert!(!i.pode_gerar());
    }

    #[test]
    fn recorte_limita_ao_monitor() {
        let monitor = Retangulo { x: 1920, y: 0, largura: 1920, altura: 1080 };
        let jogo = Retangulo { x: 1900, y: -10, largura: 1300, altura: 800 };
        assert_eq!(recorte(jogo, monitor), Some(Retangulo { x: 0, y: 0, largura: 1280, altura: 790 }));
        assert_eq!(recorte(Retangulo { x: 0, y: 0, largura: 800, altura: 600 }, monitor), None);
    }

    #[test]
    fn multiplicador_nunca_passa_do_monitor() {
        assert_eq!(multiplicador_que_cabe(2, 1.0 / 60.0, 180), 2);
        assert_eq!(multiplicador_que_cabe(4, 1.0 / 60.0, 180), 3);
        assert_eq!(multiplicador_que_cabe(2, 1.0 / 96.0, 180), 1);
        assert_eq!(multiplicador_que_cabe(3, 1.0 / 30.0, 144), 3);
        assert_eq!(multiplicador_que_cabe(2, 1.0 / 90.0, 180), 2);
        assert_eq!(multiplicador_que_cabe(2, 1.0 / 60.0, 0), 2);
    }

    #[test]
    fn quadro_com_muita_imagem_ruim_nao_sai() {
        assert!(quadro_aprovado(Some(0.02)));
        assert!(quadro_aprovado(Some(0.10)));
        assert!(!quadro_aprovado(Some(0.25)));
        assert!(!quadro_aprovado(None));
        assert!(!quadro_aprovado(Some(f32::NAN)));
    }

    #[test]
    fn niveis_arredondam_para_cima() {
        assert_eq!(niveis(1920, 1080), ((480, 270), (240, 135), (120, 68)));
    }
}
