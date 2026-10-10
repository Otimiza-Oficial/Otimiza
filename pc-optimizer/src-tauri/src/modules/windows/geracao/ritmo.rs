// O RITMO do gerador: QUANDO mostrar cada quadro. Na hora errada, o gerado vira "judder". Para multiplicador M
// e intervalo d, os gerados saem em a + k·d/M e o real por último: o real atrasa (M-1)/M de um intervalo, o
// atraso que o laboratório declara como ESTIMADO. Real que chega antes do fim descarta o resto: atraso nunca
// vira fila.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fase {
    Gerado(f32),
    Real,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Apresentacao {
    pub instante: f64,
    pub fase: Fase,
}

pub fn agenda(chegada: f64, intervalo: f64, multiplicador: u8) -> Vec<Apresentacao> {
    let m = multiplicador.clamp(1, 8) as usize;
    if m == 1 || intervalo <= 0.0 {
        return vec![Apresentacao {
            instante: chegada,
            fase: Fase::Real,
        }];
    }
    let passo = intervalo / m as f64;
    let mut v: Vec<Apresentacao> = (1..m)
        .map(|k| Apresentacao {
            instante: chegada + (k - 1) as f64 * passo,
            fase: Fase::Gerado(k as f32 / m as f32),
        })
        .collect();
    v.push(Apresentacao {
        instante: chegada + (m - 1) as f64 * passo,
        fase: Fase::Real,
    });
    v
}

/// MODO AUTOMÁTICO: quantos quadros (gerados + o real) cabem neste intervalo para encher o monitor. Com o jogo a 48 fps
/// num monitor de 180 Hz cabem 3,75 por quadro real: o multiplicador fixo parava em 3× (144 na tela); aqui a sobra se
/// acumula e um quadro real em cada quatro ganha 4, e a tela chega aos 180. `acumulado` guarda a fração de um para o outro.
pub fn quadros_no_automatico(acumulado: &mut f64, intervalo: f64, hz: u32, maximo: u8) -> u8 {
    if hz == 0 || intervalo <= 0.0 {
        return 1;
    }
    *acumulado += hz as f64 * intervalo;
    let m = (acumulado.floor().max(1.0) as u8).min(maximo.max(1));
    *acumulado = (*acumulado - m as f64).clamp(-1.0, 1.0);
    m
}

/// O que chegou à tela, pelas estatísticas da swap chain (`GetFrameStatistics`): `PresentCount` é o último quadro
/// exibido e `SyncRefreshCount` a atualização do monitor em que ele entrou. Atualização sem quadro novo é o micro-tranco
/// que o olho vê (o quadro anterior fica duas vezes na tela).
#[derive(Debug, Default, Clone)]
pub struct Exibicao {
    ultimo: Option<(u32, u32)>,
    pub exibidos: u64,
    pub atualizacoes: u64,
}

impl Exibicao {
    pub fn amostra(&mut self, quadro: u32, atualizacao: u32) {
        if let Some((q, a)) = self.ultimo {
            if quadro != q && atualizacao > a {
                self.exibidos += 1;
                // Até meio segundo de atualizações entre dois quadros conta; mais que isso é pausa, não tranco.
                self.atualizacoes += (atualizacao - a).min(90) as u64;
            }
            if quadro == q && atualizacao == a {
                return;
            }
        }
        self.ultimo = Some((quadro, atualizacao));
    }

    /// Escondida, a sobreposição não exibe nada: recomeça a contagem sem contar o buraco como tranco.
    pub fn perdeu_o_fio(&mut self) {
        self.ultimo = None;
    }

    pub fn repetidas_pct(&self) -> f64 {
        if self.atualizacoes == 0 {
            return 0.0;
        }
        (self.atualizacoes.saturating_sub(self.exibidos)) as f64 / self.atualizacoes as f64 * 100.0
    }

    pub fn zerar(&mut self) {
        self.exibidos = 0;
        self.atualizacoes = 0;
    }
}

/// No ritmo do monitor: o `t` do quadro que mostra o instante `no_jogo` entre os reais que apareceram em `a` e `b`.
/// `None`: `b` já passou, é a vez do próprio real (e, se o próximo atrasou, ele fica mais uma atualização).
pub fn t_no_tempo(no_jogo: f64, a: f64, b: f64) -> Option<f32> {
    if b <= a || no_jogo >= b - 1e-4 {
        return None;
    }
    Some(((no_jogo - a) / (b - a)).clamp(0.0, 0.999) as f32)
}

/// Os carimbos dos reais sem o degrau do monitor. O Windows carimba cada quadro do jogo na atualização da tela em que
/// ele entrou: um jogo a 50 fps num monitor de 180 Hz chega com 16,7 e 22,2 ms alternados, quando anda a 20 ms. Usado
/// cru, esse degrau virava solavanco no meio dos quadros gerados. Aqui cada carimbo é a previsão pelo ritmo médio,
/// puxada um pouco para o medido (sem nunca se afastar dele mais que meio intervalo).
#[derive(Debug, Clone, Default)]
pub struct CarimboSuave {
    ultimo: Option<f64>,
    intervalo: Option<f64>,
}

impl CarimboSuave {
    pub fn suavizar(&mut self, medido: f64) -> f64 {
        let novo = match (self.ultimo, self.intervalo) {
            (Some(u), Some(i)) if medido > u && medido - u < 0.25 => {
                let previsto = u + i;
                let v = previsto + 0.25 * (medido - previsto);
                v.clamp(medido - i * 0.5, medido + i * 0.5).max(u + 1e-4)
            }
            _ => medido,
        };
        if let Some(u) = self.ultimo {
            let d = medido - u;
            if (0.0..0.25).contains(&d) {
                self.intervalo = Some(self.intervalo.map_or(d, |i| i * 0.85 + d * 0.15));
            } else {
                // Pausa ou troca de cena: recomeça do medido.
                self.intervalo = None;
            }
        }
        self.ultimo = Some(novo);
        novo
    }
}

/// O atraso entre o jogo e a tela no ritmo do monitor: o intervalo entre os reais mais o tempo até o par ficar pronto
/// (captura e estimativa), em média, e 2 ms de folga para o quadro que chega um pouco atrasado.
#[derive(Debug, Clone, Default)]
pub struct AtrasoDaExibicao {
    intervalo: Option<f64>,
    preparo: Option<f64>,
}

impl AtrasoDaExibicao {
    pub const FOLGA: f64 = 0.005;

    pub fn registrar(&mut self, intervalo: f64, preparo: f64) {
        let media = |m: Option<f64>, v: f64| Some(m.map_or(v, |m| m * 0.9 + v * 0.1));
        if (0.0..0.2).contains(&intervalo) {
            self.intervalo = media(self.intervalo, intervalo);
        }
        if (0.0..0.1).contains(&preparo) {
            self.preparo = media(self.preparo, preparo);
        }
    }

    pub fn segundos(&self) -> f64 {
        self.intervalo.unwrap_or(0.02) + self.preparo.unwrap_or(0.005) + Self::FOLGA
    }
}

/// Gerado além da taxa do monitor não aparece: vira descarte ou rasgo e custa placa à toa.
pub fn multiplicador_que_cabe(pedido: u8, intervalo: f64, hz: u32) -> u8 {
    if hz == 0 || intervalo <= 0.0 {
        return pedido.max(1);
    }
    let cabe = (hz as f64 * intervalo * 1.03).floor() as u8;
    pedido.min(cabe).max(1)
}

pub const FRACAO_RUIM_MAXIMA: f32 = 0.06;

pub fn quadro_aprovado(fracao_ruim: Option<f32>) -> bool {
    fracao_ruim.is_some_and(|f| f.is_finite() && f <= FRACAO_RUIM_MAXIMA)
}

/// Média exponencial com rejeição de extremos: uma tela de carregamento de 2 s não pode virar "0,5 FPS".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intervalo {
    pub estimado: Option<f64>,
    pub ultimo_real: Option<f64>,
}

impl Default for Intervalo {
    fn default() -> Self {
        Intervalo {
            estimado: None,
            ultimo_real: None,
        }
    }
}

/// Abaixo de 20 FPS reais não se gera: a interpolação vira borrão e o atraso dobra.
pub const INTERVALO_MAXIMO: f64 = 1.0 / 20.0;
pub const INTERVALO_MINIMO: f64 = 1.0 / 500.0;

impl Intervalo {
    pub fn registrar(&mut self, chegada: f64) -> Option<f64> {
        let medido = self.ultimo_real.map(|u| chegada - u);
        self.ultimo_real = Some(chegada);
        let d = medido?;
        if !(INTERVALO_MINIMO..=INTERVALO_MAXIMO).contains(&d) {
            if d > INTERVALO_MAXIMO {
                self.estimado = None;
            }
            return Some(d);
        }
        self.estimado = Some(match self.estimado {
            None => d,
            // Mudança grande de ritmo segue rápido; oscilação pequena é suavizada.
            Some(e) if (d - e).abs() > e * 0.5 => d,
            Some(e) => e * 0.8 + d * 0.2,
        });
        Some(d)
    }

    pub fn pode_gerar(&self) -> bool {
        self.estimado.is_some()
    }
}

/// O monitor girado nas configurações de vídeo (monitor em pé). A captura vem na orientação do PAINEL, não da área de
/// trabalho: a área do jogo precisa ser convertida e a imagem girada de volta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Giro {
    #[default]
    Nenhum,
    G90,
    G180,
    G270,
}

impl Giro {
    /// `DXGI_MODE_ROTATION`: 1 identidade, 2 = 90°, 3 = 180°, 4 = 270° (0 = não informado).
    pub fn de_dxgi(v: i32) -> Giro {
        match v {
            2 => Giro::G90,
            3 => Giro::G180,
            4 => Giro::G270,
            _ => Giro::Nenhum,
        }
    }

    /// O número que o shader `Entrada` entende.
    pub fn codigo(self) -> f32 {
        match self {
            Giro::Nenhum => 0.0,
            Giro::G90 => 1.0,
            Giro::G180 => 2.0,
            Giro::G270 => 3.0,
        }
    }

    pub fn troca_lados(self) -> bool {
        matches!(self, Giro::G90 | Giro::G270)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retangulo {
    pub x: i32,
    pub y: i32,
    pub largura: i32,
    pub altura: i32,
}

/// Coordenadas da área de trabalho; devolve o recorte no monitor. `None` fora dele ou pequeno demais.
/// Pura: a área do jogo (relativa ao monitor, na orientação da área de trabalho) nas coordenadas da textura capturada.
/// `largura`/`altura` são as do monitor como a área de trabalho o vê. Mesmas fórmulas do exemplo de Duplicação de
/// Área de Trabalho da Microsoft, ao contrário.
pub fn na_textura(r: Retangulo, giro: Giro, largura: i32, altura: i32) -> Retangulo {
    match giro {
        Giro::Nenhum => r,
        Giro::G90 => Retangulo {
            x: r.y,
            y: largura - (r.x + r.largura),
            largura: r.altura,
            altura: r.largura,
        },
        Giro::G180 => Retangulo {
            x: largura - (r.x + r.largura),
            y: altura - (r.y + r.altura),
            largura: r.largura,
            altura: r.altura,
        },
        Giro::G270 => Retangulo {
            x: altura - (r.y + r.altura),
            y: r.x,
            largura: r.altura,
            altura: r.largura,
        },
    }
}

pub fn recorte(cliente: Retangulo, monitor: Retangulo) -> Option<Retangulo> {
    let x0 = cliente.x.max(monitor.x);
    let y0 = cliente.y.max(monitor.y);
    let x1 = (cliente.x + cliente.largura).min(monitor.x + monitor.largura);
    let y1 = (cliente.y + cliente.altura).min(monitor.y + monitor.altura);
    let (largura, altura) = (x1 - x0, y1 - y0);
    if largura < 320 || altura < 200 {
        return None;
    }
    Some(Retangulo {
        x: x0 - monitor.x,
        y: y0 - monitor.y,
        largura,
        altura,
    })
}

pub fn niveis(largura: i32, altura: i32) -> ((u32, u32), (u32, u32), (u32, u32)) {
    let d = |v: i32, f: i32| ((v + f - 1) / f).max(1) as u32;
    (
        (d(largura, 4), d(altura, 4)),
        (d(largura, 8), d(altura, 8)),
        (d(largura, 16), d(altura, 16)),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Contadores {
    pub reais: u64,
    pub gerados: u64,
    pub descartados: u64,
    #[serde(default)]
    pub recusados: u64,
    pub custo_ms: f64,
    /// Quadros capturados iguais ao último real (jogo parado, ou a tela recomposta pela própria sobreposição).
    #[serde(default)]
    pub iguais: u64,
    /// Quadros capturados em que nada mudou dentro da janela do jogo (outra coisa mudou no monitor).
    #[serde(default)]
    pub fora_do_jogo: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_tempo_cada_atualizacao_mostra_o_seu_instante() {
        // Reais a 45 fps (22,2 ms), mostrados com o atraso de um intervalo: as atualizações de 180 Hz andam por igual.
        let (a, b) = (1.0, 1.0 + 1.0 / 45.0);
        let p = 1.0 / 180.0;
        let ts: Vec<Option<f32>> = (0..4).map(|k| t_no_tempo(a + k as f64 * p, a, b)).collect();
        let passos: Vec<f32> = ts.windows(2).filter_map(|w| Some(w[1]? - w[0]?)).collect();
        assert!(passos.iter().all(|d| (d - 0.25).abs() < 0.01), "{:?}", ts);
        assert_eq!(t_no_tempo(b, a, b), None, "chegou a vez do real");
        assert_eq!(
            t_no_tempo(a - 0.01, a, b),
            Some(0.0),
            "antes do par, o começo dele"
        );
    }

    #[test]
    fn o_carimbo_suave_tira_o_degrau_do_monitor() {
        // Jogo a 50 fps carimbado na grade de 180 Hz: 3 ou 4 atualizações por quadro.
        let p = 1.0 / 180.0;
        let mut c = CarimboSuave::default();
        let mut suaves = Vec::new();
        for k in 0..60 {
            let real = k as f64 * 0.02;
            let carimbo = (real / p).ceil() * p;
            suaves.push(c.suavizar(carimbo));
        }
        let passos: Vec<f64> = suaves[20..]
            .windows(2)
            .map(|w| (w[1] - w[0]) * 1000.0)
            .collect();
        let pior = passos.iter().map(|d| (d - 20.0).abs()).fold(0.0, f64::max);
        assert!(pior < 2.0, "passos de {:?} ms", passos);
    }

    #[test]
    fn o_atraso_e_o_intervalo_mais_o_preparo() {
        let mut d = AtrasoDaExibicao::default();
        for _ in 0..50 {
            d.registrar(0.022, 0.006);
        }
        assert!(
            (d.segundos() - (0.028 + AtrasoDaExibicao::FOLGA)).abs() < 0.0005,
            "{}",
            d.segundos()
        );
    }

    #[test]
    fn conta_as_atualizacoes_sem_quadro_novo() {
        let mut e = Exibicao::default();
        // Um quadro por atualização, depois um que ficou duas.
        e.amostra(1, 100);
        e.amostra(2, 101);
        e.amostra(3, 102);
        e.amostra(4, 104);
        assert_eq!(e.exibidos, 3);
        assert_eq!(e.atualizacoes, 4);
        assert!((e.repetidas_pct() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn o_automatico_enche_o_monitor() {
        // 48 fps num monitor de 180 Hz: 3,75 por quadro real, na média.
        let mut acc = 0.0;
        let total: u32 = (0..48)
            .map(|_| quadros_no_automatico(&mut acc, 1.0 / 48.0, 180, 6) as u32)
            .sum();
        assert!(
            (178..=181).contains(&total),
            "{} quadros por segundo",
            total
        );
        // Jogo mais rápido que o monitor: só o real.
        let mut acc = 0.0;
        assert_eq!(quadros_no_automatico(&mut acc, 1.0 / 200.0, 180, 6), 1);
        // 25 fps em 180 Hz pediria 7,2: para no máximo.
        let mut acc = 0.0;
        assert_eq!(quadros_no_automatico(&mut acc, 1.0 / 25.0, 180, 6), 6);
    }

    #[test]
    fn area_do_jogo_no_monitor_em_pe_vira_a_area_na_textura_do_painel() {
        // Monitor 1080x1920 em pé (90°); a textura capturada é 1920x1080.
        let jogo = Retangulo {
            x: 0,
            y: 300,
            largura: 1080,
            altura: 608,
        };
        assert_eq!(
            na_textura(jogo, Giro::G90, 1080, 1920),
            Retangulo {
                x: 300,
                y: 0,
                largura: 608,
                altura: 1080
            }
        );
        let canto = Retangulo {
            x: 100,
            y: 200,
            largura: 300,
            altura: 400,
        };
        let t = na_textura(canto, Giro::G90, 1080, 1920);
        assert!(t.x >= 0 && t.y >= 0 && t.x + t.largura <= 1920 && t.y + t.altura <= 1080);
        assert_eq!(
            na_textura(canto, Giro::G180, 1920, 1080),
            Retangulo {
                x: 1520,
                y: 480,
                largura: 300,
                altura: 400
            }
        );
        assert_eq!(
            na_textura(canto, Giro::G270, 1080, 1920),
            Retangulo {
                x: 1320,
                y: 100,
                largura: 400,
                altura: 300
            }
        );
        assert_eq!(na_textura(canto, Giro::Nenhum, 1920, 1080), canto);
        assert_eq!(Giro::de_dxgi(2), Giro::G90);
        assert_eq!(Giro::de_dxgi(0), Giro::Nenhum);
    }

    #[test]
    fn agenda_2x_mostra_gerado_na_chegada_e_real_na_metade() {
        let a = agenda(10.0, 1.0 / 60.0, 2);
        assert_eq!(a.len(), 2);
        assert_eq!(
            a[0],
            Apresentacao {
                instante: 10.0,
                fase: Fase::Gerado(0.5)
            }
        );
        assert_eq!(a[1].fase, Fase::Real);
        assert!((a[1].instante - (10.0 + 1.0 / 120.0)).abs() < 1e-9);
    }

    #[test]
    fn agenda_3x_espacamento_uniforme() {
        let d = 0.03;
        let a = agenda(0.0, d, 3);
        let fases: Vec<Fase> = a.iter().map(|x| x.fase).collect();
        assert_eq!(
            fases,
            vec![Fase::Gerado(1.0 / 3.0), Fase::Gerado(2.0 / 3.0), Fase::Real]
        );
        assert!((a[1].instante - 0.01).abs() < 1e-9);
        assert!((a[2].instante - 0.02).abs() < 1e-9);
    }

    #[test]
    fn multiplicador_1_so_repassa_o_real() {
        assert_eq!(
            agenda(5.0, 0.016, 1),
            vec![Apresentacao {
                instante: 5.0,
                fase: Fase::Real
            }]
        );
        assert_eq!(agenda(5.0, 0.0, 3).len(), 1);
    }

    #[test]
    fn intervalo_suaviza_e_segue_mudanca_grande() {
        let mut i = Intervalo::default();
        assert_eq!(i.registrar(0.0), None);
        i.registrar(1.0 / 60.0);
        assert!((i.estimado.unwrap() - 1.0 / 60.0).abs() < 1e-9);
        i.registrar(1.0 / 60.0 + 0.018);
        let e = i.estimado.unwrap();
        assert!(e > 1.0 / 60.0 && e < 0.018);
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
        let monitor = Retangulo {
            x: 1920,
            y: 0,
            largura: 1920,
            altura: 1080,
        };
        let jogo = Retangulo {
            x: 1900,
            y: -10,
            largura: 1300,
            altura: 800,
        };
        assert_eq!(
            recorte(jogo, monitor),
            Some(Retangulo {
                x: 0,
                y: 0,
                largura: 1280,
                altura: 790
            })
        );
        assert_eq!(
            recorte(
                Retangulo {
                    x: 0,
                    y: 0,
                    largura: 800,
                    altura: 600
                },
                monitor
            ),
            None
        );
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
        assert!(quadro_aprovado(Some(0.06)));
        assert!(!quadro_aprovado(Some(0.08)));
        assert!(!quadro_aprovado(Some(0.25)));
        assert!(!quadro_aprovado(None));
        assert!(!quadro_aprovado(Some(f32::NAN)));
    }

    #[test]
    fn niveis_arredondam_para_cima() {
        assert_eq!(niveis(1920, 1080), ((480, 270), (240, 135), (120, 68)));
    }
}
