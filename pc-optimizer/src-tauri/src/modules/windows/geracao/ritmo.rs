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

/// Quanto o quadro REAL espera pela vez da tela nos modos fixos. Ele sempre sai: é o quadro do jogo.
pub const ESPERA_DO_REAL_MS: u32 = 50;

/// Quanto o GERADO espera pela vez. Com 0, no 2× a 144 Hz com o jogo a 72 FPS (o passo da agenda igual a uma
/// atualização da tela), só 76% dos gerados saíam; com 2 ms, todos (`o_2x_no_limite_do_monitor_mostra_os_gerados`).
/// O gerador pede `timeBeginPeriod(1)`: a espera não passa disso.
pub const ESPERA_DO_GERADO_MS: u32 = 2;

/// MODOS FIXOS (2×–4×), com um quadro na fila do Windows, no máximo: o gerado espera um pouco pela vez e, sem ela, é
/// descartado (esperar mais atrasaria o real seguinte). O real espera a vez e sai de todo jeito.
/// `esperar(ms, sai_de_todo_jeito)`: `true` quando a fila tem lugar.
pub fn vez_no_modo_fixo(fase: Fase, esperar: impl FnOnce(u32, bool) -> bool) -> bool {
    match fase {
        Fase::Gerado(_) => esperar(ESPERA_DO_GERADO_MS, false),
        Fase::Real => {
            esperar(ESPERA_DO_REAL_MS, true);
            true
        }
    }
}

/// A vez da tela é um semáforo do Windows: cada quadro que sai da fila devolve uma vaga. Um `Present` feito sem ter
/// tomado a vaga (o real que passou da espera) deixa uma vaga a mais quando sai, e a fila voltaria a aceitar dois.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Divida {
    /// As vagas a devolver.
    pub vagas: u32,
    /// Pagou alguma desde a última espera que deu certo. Se a espera seguinte estoura, a dívida era falsa (a vaga
    /// tinha se perdido: `Present` ocluído, quadro que o Windows descartou), e não se cobra mais nada: senão, cada
    /// real pagaria a vaga do anterior e esperaria 50 ms, e a tela ficaria em 20 FPS para sempre.
    pub pagou: bool,
}

/// Antes de esperar, toma de volta as vagas que já sobraram (sem esperar por elas). `tentar(ms)`: tenta tomar uma
/// vaga. Devolve se a vaga desta espera veio.
pub fn esperar_a_vez_pagando(
    divida: &mut Divida,
    ms: u32,
    sai_de_todo_jeito: bool,
    mut tentar: impl FnMut(u32) -> bool,
) -> bool {
    while divida.vagas > 0 && tentar(0) {
        divida.vagas -= 1;
        divida.pagou = true;
    }
    let veio = tentar(ms);
    if veio {
        divida.pagou = false;
    } else if sai_de_todo_jeito {
        if divida.pagou {
            *divida = Divida::default();
        } else {
            divida.vagas = (divida.vagas + 1).min(8);
        }
    }
    veio
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

/// O menor alvo que vale: abaixo disto a geração não compensa o atraso que acrescenta.
pub const ALVO_MINIMO: u32 = 60;

/// **Pura.** Para onde o automático enche: o alvo escolhido (nunca acima dos Hz nem abaixo de `ALVO_MINIMO`), os Hz, ou
/// o dobro deles passando do limite do monitor.
pub fn mira_do_automatico(hz: u32, sem_limite: bool, alvo: Option<u32>) -> u32 {
    if sem_limite {
        return hz * 2;
    }
    match alvo.filter(|a| *a > 0) {
        Some(a) if hz > 0 => a.clamp(ALVO_MINIMO.min(hz), hz),
        _ => hz,
    }
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

/// No ritmo do monitor: quantas imagens novas a tela recebeu por quadro real do jogo, para a tela, o log e a guarda
/// (o Auto não tem multiplicador fixo; antes a tela dizia "agora 1×" enquanto gerava). `imagens`: as que foram para a
/// tela (geradas e o real na primeira vez). A imagem de t≈0 já É o real: somar os reais de novo dava 1 a mais (revisão
/// final da 1.0.9). `None` sem quadro real na janela.
pub fn multiplicador_medido(reais: u64, imagens: u64) -> Option<f64> {
    (reais > 0).then(|| imagens.max(reais) as f64 / reais as f64)
}

/// No ritmo do monitor, o que esta atualização da tela recebe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NaAtualizacao {
    /// Um quadro gerado no instante `t` entre os dois reais.
    Gerado(f32),
    /// O real mais novo, pela primeira vez.
    Real,
    /// Nada de novo: a tela repete o que já mostra, sem gastar placa com outro `Present`.
    Repetir,
}

/// Pura: o que mostrar nesta atualização. `t`: o de `t_no_tempo`. `ja_mostrados`: imagens deste intervalo entre
/// reais que já foram para a tela. `teto`: o multiplicador fixo (2×, 3×, 4×) ou `None` no Auto. `meio_passo`: meia
/// atualização da tela em unidades de `t` (período ÷ 2 ÷ intervalo do jogo). `redesenhar`: repetir precisa de outro
/// `Present` (escala com o cursor ampliado).
///
/// No ritmo do monitor o real quase nunca sai sozinho: a imagem de `t` perto de 0 é ele (o quadro `a` do par). Por
/// isso o fixo conta TODAS as imagens do intervalo (2× = duas por real) e põe a k-ésima na atualização mais perto de
/// `t = k/m`. Gastar o gerado na primeira atualização deixava o 2× com uma imagem por real (revisão da 1.0.9).
pub fn na_atualizacao(
    t: Option<f32>,
    real_ja_mostrado: bool,
    ja_mostrados: u8,
    teto: Option<u8>,
    meio_passo: f32,
    redesenhar: bool,
) -> NaAtualizacao {
    match t {
        Some(t) => {
            let cabe = teto.is_none_or(|m| {
                let m = m.max(1);
                ja_mostrados < m && t + meio_passo >= ja_mostrados as f32 / m as f32
            });
            if cabe || redesenhar {
                NaAtualizacao::Gerado(t)
            } else {
                NaAtualizacao::Repetir
            }
        }
        None if !real_ja_mostrado || redesenhar => NaAtualizacao::Real,
        None => NaAtualizacao::Repetir,
    }
}

/// Pura: só vale gerar se cabe pelo menos um quadro a mais entre dois reais. Com o jogo nos Hz ou acima (FiveM a 150
/// num monitor de 144), gerar só custava placa e somava um intervalo de atraso, sem nenhuma imagem a mais.
pub fn vale_gerar(hz: u32, intervalo_do_jogo: Option<f64>) -> bool {
    intervalo_do_jogo.is_some_and(|i| hz as f64 * i > 1.1)
}

/// A FAIXA EM QUE VALE GERAR, COM FOLGA PARA NÃO FICAR TROCANDO (1.0.15). `vale_gerar` decidia a cada real com o
/// limite de 1,1 atualização por real: no log do sócio (FiveM com a base solta, 115 a 138 FPS num monitor de 144 Hz),
/// cada queda do jogo ao girar a câmera passava do limite e o gerador enfiava um gerado aqui e ali, entre reais que
/// chegam em qualquer fase da atualização. A tela medida (PresentMon) mostrou o preço: nas janelas sem gerados, 144
/// quadros novos/s, 0,3% das atualizações repetidas e 0 engasgos; nas com 30 a 60 gerados/s, 10 a 20 engasgos e 10 a
/// 18% repetidas: as microtravadas "toda hora que viro". Um gerado só deixa o ritmo regular quando cabem pelo menos
/// ~1,6 atualização por real (base até 90 FPS em 144 Hz; a base travada em 72 dá 2,0). Abaixo disso, mostrar só os
/// reais é mais liso, e a sobreposição sai da frente.
pub const FAIXA_ENTRA: f64 = 1.6;
/// Gerando, só para abaixo disto: a folga entre 1,4 e 1,6 evita ligar e desligar a cada oscilação do jogo.
pub const FAIXA_SAI: f64 = 1.4;
/// Quanto tempo o jogo precisa ficar do outro lado da faixa para trocar (sair é mais rápido: metade).
pub const FAIXA_FIRME: f64 = 1.0;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FaixaDeGerar {
    pub gerando: bool,
    trocar_desde: Option<f64>,
}

impl FaixaDeGerar {
    /// Pura: com o intervalo estimado do jogo, se gera nesta volta. Sem intervalo (jogo parado, menu), para na hora.
    pub fn passo(&mut self, hz: u32, intervalo_do_jogo: Option<f64>, agora: f64) -> bool {
        let razao = intervalo_do_jogo.filter(|_| hz > 0).map(|i| hz as f64 * i);
        let quer_trocar = match (self.gerando, razao) {
            (true, Some(r)) => r < FAIXA_SAI,
            (true, None) => true,
            (false, Some(r)) => r >= FAIXA_ENTRA,
            (false, None) => false,
        };
        if !quer_trocar {
            self.trocar_desde = None;
            return self.gerando;
        }
        let desde = *self.trocar_desde.get_or_insert(agora);
        let firme = if self.gerando { FAIXA_FIRME * 0.5 } else { FAIXA_FIRME };
        if razao.is_none() || agora - desde >= firme {
            self.gerando = !self.gerando;
            self.trocar_desde = None;
        }
        self.gerando
    }
}

/// UMA IMAGEM POR ATUALIZAÇÃO, ANCORADA NA ATUALIZAÇÃO DO MONITOR SEM BLOQUEAR NELA.
///
/// - 1.0.8: o laço só esperava a vez da fila, que voltava rápido: 180-274 imagens/s num monitor de 144 Hz e a placa a
///   70-80% desenhando o que nunca aparecia.
/// - 1.0.9: `WaitForVBlank` antes de cada imagem: placa a 40%, mas os engasgos foram de ~2/s para 4-10/s no FiveM. A
///   imagem que perdia o prazo ainda esperava a atualização seguinte inteira (duas perdas somadas).
/// - 1.0.10: o Windows diz quando foi a última atualização e o período (`DwmGetCompositionTimingInfo`); a imagem é
///   feita para a próxima, começando `MARGEM_ANTES_DA_ATUALIZACAO` antes dela, e o laço nunca dorme além disso. Quem
///   perde o prazo sai na hora (cai na seguinte) em vez de esperar mais uma.
pub const MARGEM_ANTES_DA_ATUALIZACAO: f64 = 0.003;

/// Pura: a próxima atualização da tela que ainda dá tempo de pegar: a primeira `ultima + k·período` depois de
/// `agora + margem/2` (`k` pode ser 0 ou negativo: medido nesta máquina, o Windows às vezes dá a atualização que AINDA
/// vai acontecer). Mais perto que meia margem, não dá para compor a tempo: fica a seguinte.
pub fn proxima_atualizacao(ultima: f64, periodo: f64, agora: f64) -> f64 {
    if periodo <= 0.0 {
        return agora;
    }
    let limite = agora + MARGEM_ANTES_DA_ATUALIZACAO * 0.5;
    let k = ((limite - ultima) / periodo).floor() + 1.0;
    ultima + k * periodo
}

/// Pura: a reserva sem o tempo do Windows. Prazo ABSOLUTO a cada 3/4 de atualização (contar a partir de "agora"
/// escorregava para perto do período e batia de frente com a atualização). Atrasado, recomeça de agora.
pub fn proximo_prazo(prazo: f64, agora: f64, intervalo: f64) -> f64 {
    let p = prazo + intervalo;
    if p < agora { agora + intervalo } else { p }
}

/// Pura: o teto de onde a guarda desce, pelo que o gerador faz de verdade. Um 4× com base 60 em 144 Hz já faz ~2,4
/// imagens por real: baixar para 3× não aliviaria nada.
pub fn teto_efetivo(teto: u8, medido: Option<f64>) -> u8 {
    match medido {
        // 2,03 medido é 2 com ruído: subir para 3 daria um degrau que não alivia nada.
        Some(m) if m.is_finite() && m >= 1.0 => teto.min((m - 0.1).ceil().max(1.0) as u8),
        _ => teto,
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

/// O que chegou à tela numa janela, pelo instante em que cada quadro da sobreposição APARECEU (PresentMon).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RitmoNaTela {
    pub por_segundo: f64,
    /// Atualizações do monitor que repetiram o quadro anterior, em % (com 2× dentro dos Hz, o ideal é perto de 0).
    pub repetidas_pct: f64,
    /// Buracos de 3 atualizações ou mais sem quadro novo (o tranco que se sente), fora pausas de meio segundo.
    pub engasgos: u32,
}

/// Pura: o ritmo na tela a partir dos instantes (s, em ordem) em que os quadros apareceram. `None` sem quadros que
/// bastem ou sem os Hz.
pub fn ritmo_na_tela(instantes: &[f64], periodo: f64) -> Option<RitmoNaTela> {
    if instantes.len() < 10 || periodo <= 0.0 {
        return None;
    }
    let (mut atualizacoes, mut repetidas, mut engasgos, mut tempo) = (0u64, 0u64, 0u32, 0.0);
    for par in instantes.windows(2) {
        let d = par[1] - par[0];
        if !(0.0..0.5).contains(&d) {
            continue;
        }
        let n = (d / periodo).round().max(1.0) as u64;
        atualizacoes += n;
        repetidas += n - 1;
        if n >= 3 {
            engasgos += 1;
        }
        tempo += d;
    }
    (atualizacoes > 0 && tempo > 0.0).then(|| RitmoNaTela {
        por_segundo: atualizacoes.saturating_sub(repetidas) as f64 / tempo,
        repetidas_pct: repetidas as f64 / atualizacoes as f64 * 100.0,
        engasgos,
    })
}

/// Média de uma medida em ms (fila até a tela), sem os extremos de um engasgo.
pub fn media_da_fila(media: Option<f64>, novas: &[f64]) -> Option<f64> {
    novas
        .iter()
        .filter(|v| (0.0..200.0).contains(*v))
        .fold(media, |m, v| Some(m.map_or(*v, |m| m * 0.95 + v * 0.05)))
}

/// O atraso que o Frame Boost acrescenta, em ms: do quadro do jogo composto pelo Windows (perto da atualização em que
/// ele apareceria) até a sobreposição chegar à tela. `ate_o_present` pelo relógio do gerador, mais a fila inteira da
/// sobreposição até a tela, pelo PresentMon. Continua ESTIMADO: são duas médias, e não o mesmo quadro casado.
pub fn atraso_acrescentado(ate_o_present: Option<f64>, fila_da_sobreposicao: Option<f64>) -> Option<f64> {
    ate_o_present.map(|a| a + fila_da_sobreposicao.unwrap_or(0.0))
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
#[cfg_attr(test, derive(specta::Type))]
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
#[cfg_attr(test, derive(specta::Type))]
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
    fn o_alvo_fica_entre_o_minimo_e_os_hz() {
        assert_eq!(mira_do_automatico(180, false, Some(120)), 120);
        assert_eq!(mira_do_automatico(180, false, Some(500)), 180);
        assert_eq!(mira_do_automatico(180, false, Some(30)), ALVO_MINIMO);
        assert_eq!(mira_do_automatico(180, false, None), 180);
        assert_eq!(mira_do_automatico(180, true, Some(120)), 360);
        // A 120 num de 180 com o jogo a 48: 2,5 por quadro real, e a tela chega aos 120.
        let mut acc = 0.0;
        let soma: u32 = (0..48).map(|_| quadros_no_automatico(&mut acc, 1.0 / 48.0, 120, 6) as u32).sum();
        assert!((118..=122).contains(&soma), "{soma}");
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

    /// Simulação do ritmo do monitor como o laço faz: reais chegando a cada intervalo, uma decisão por atualização,
    /// `no_jogo` atrás do agora pelo atraso da exibição (intervalo + preparo + folga). Devolve imagens novas por
    /// segundo.
    fn novos_por_segundo_no_ritmo(hz: f64, base: f64, teto: Option<u8>) -> f64 {
        let (p, i) = (1.0 / hz, 1.0 / base);
        let atraso = i + 0.005 + AtrasoDaExibicao::FOLGA;
        let (mut a, mut b): (Option<f64>, Option<f64>) = (None, None);
        let (mut proximo_real, mut ja, mut real_ja, mut novos) = (0.0013, 0u8, true, 0u32);
        let segundos = 10.0;
        for k in 0..(hz * segundos) as u32 {
            let agora = k as f64 * p + 0.0004;
            while proximo_real <= agora {
                a = b;
                b = Some(proximo_real);
                proximo_real += i;
                ja = 0;
                real_ja = false;
            }
            let par = if vale_gerar(hz as u32, Some(i)) { a.zip(b) } else { None };
            let t = par.and_then(|(a, b)| t_no_tempo(agora + p - atraso, a, b));
            match na_atualizacao(t, real_ja, ja, teto, (p / (2.0 * i)) as f32, false) {
                NaAtualizacao::Gerado(_) => {
                    novos += 1;
                    ja += 1;
                }
                NaAtualizacao::Real => {
                    if !real_ja {
                        novos += 1;
                    }
                    real_ja = true;
                }
                NaAtualizacao::Repetir => {}
            }
        }
        novos as f64 / segundos
    }

    #[test]
    fn no_ritmo_do_monitor_o_2x_entrega_o_dobro_e_o_auto_enche_a_tela() {
        // O log do sócio (1.0.8, pela agenda): 2× com base 60-72 em 144 Hz mostrava 80-90 novos/s.
        let n = novos_por_segundo_no_ritmo(144.0, 60.0, Some(2));
        assert!((110.0..=125.0).contains(&n), "2× base 60: {n}");
        let n = novos_por_segundo_no_ritmo(144.0, 72.0, Some(2));
        assert!(n >= 130.0, "2× base 72: {n}");
        let n = novos_por_segundo_no_ritmo(144.0, 60.0, None);
        assert!(n >= 135.0, "Auto base 60: {n}");
        let n = novos_por_segundo_no_ritmo(144.0, 40.0, Some(3));
        assert!((110.0..=125.0).contains(&n), "3× base 40: {n}");
        // Jogo acima dos Hz: não gera nada, e a tela mostra os reais.
        let n = novos_por_segundo_no_ritmo(144.0, 150.0, Some(2));
        assert!(n >= 130.0, "base 150: {n}");
    }

    #[test]
    fn no_ritmo_do_monitor_o_fixo_tem_teto_e_nada_se_redesenha_a_toa() {
        use NaAtualizacao::*;
        let meio = 0.2;
        // Auto: gera em toda atualização em que o real ainda não chegou.
        assert_eq!(na_atualizacao(Some(0.4), false, 5, None, meio, false), Gerado(0.4));
        // 2× fixo: a 1ª imagem logo, a 2ª perto de t = 0,5, e a 3ª não cabe.
        assert_eq!(na_atualizacao(Some(0.03), false, 0, Some(2), meio, false), Gerado(0.03));
        assert_eq!(na_atualizacao(Some(0.2), false, 1, Some(2), meio, false), Repetir, "cedo demais para a 2ª");
        assert_eq!(na_atualizacao(Some(0.45), false, 1, Some(2), meio, false), Gerado(0.45));
        assert_eq!(na_atualizacao(Some(0.86), false, 2, Some(2), meio, false), Repetir);
        // O real aparece uma vez; nas atualizações seguintes, sem quadro novo do jogo, não se redesenha.
        assert_eq!(na_atualizacao(None, false, 1, Some(2), meio, false), Real);
        assert_eq!(na_atualizacao(None, true, 2, None, meio, false), Repetir);
        // Escala com o cursor ampliado: redesenha para o cursor andar, mesmo com o teto batendo.
        assert_eq!(na_atualizacao(None, true, 2, None, meio, true), Real);
        assert_eq!(na_atualizacao(Some(0.86), false, 2, Some(2), meio, true), Gerado(0.86));
    }

    #[test]
    fn so_gera_com_folga_entre_dois_reais() {
        assert!(vale_gerar(144, Some(1.0 / 72.0)));
        assert!(!vale_gerar(144, Some(1.0 / 150.0)), "acima dos Hz");
        assert!(!vale_gerar(144, Some(1.0 / 140.0)), "perto dos Hz: não cabe um quadro a mais");
        assert!(!vale_gerar(60, Some(1.0 / 60.0)));
        assert!(!vale_gerar(144, None));
    }

    #[test]
    fn a_faixa_do_socio_nao_gera_perto_dos_hz() {
        // O log do sócio (2026-11-21, FiveM em 144 Hz, base solta): 115 a 138 FPS. Nunca gera.
        let mut f = FaixaDeGerar::default();
        let mut agora = 0.0;
        for fps in [138.0, 125.0, 115.0, 130.0, 119.0, 136.0].iter().cycle().take(600) {
            agora += 1.0 / fps;
            assert!(!f.passo(144, Some(1.0 / fps), agora), "{} FPS em 144 Hz não pode gerar", fps);
        }
    }

    #[test]
    fn a_faixa_gera_com_a_base_travada_e_tem_folga() {
        let mut f = FaixaDeGerar::default();
        // Base travada em 72 (144 Hz): entra depois de 1 s firme.
        assert!(!f.passo(144, Some(1.0 / 72.0), 0.0));
        assert!(!f.passo(144, Some(1.0 / 72.0), 0.5));
        assert!(f.passo(144, Some(1.0 / 72.0), 1.0));
        // Oscila para 100 FPS (1,44 atualização por real): dentro da folga, segue gerando.
        assert!(f.passo(144, Some(1.0 / 100.0), 1.1));
        assert!(f.passo(144, Some(1.0 / 100.0), 3.0));
        // Sobe para 130 FPS: sai depois de meio segundo.
        assert!(f.passo(144, Some(1.0 / 130.0), 3.1));
        assert!(!f.passo(144, Some(1.0 / 130.0), 3.7));
        // Um pico curto acima da faixa não liga.
        assert!(!f.passo(144, Some(1.0 / 72.0), 4.0));
        assert!(!f.passo(144, Some(1.0 / 130.0), 4.3));
        assert!(!f.passo(144, Some(1.0 / 72.0), 4.4));
        assert!(!f.passo(144, Some(1.0 / 72.0), 5.3));
        assert!(f.passo(144, Some(1.0 / 72.0), 5.4));
        // Jogo parado: para na hora.
        assert!(!f.passo(144, None, 5.5));
    }

    #[test]
    fn a_proxima_atualizacao_vem_do_windows_e_nunca_fica_para_tras() {
        let p = 1.0 / 144.0;
        // Última atualização em 10,000 s; agora 10,002: a próxima é 10,000 + p.
        assert!((proxima_atualizacao(10.0, p, 10.002) - (10.0 + p)).abs() < 1e-12);
        // O tempo do Windows atrasado (agora já passou duas atualizações): pula para a que vem.
        assert!((proxima_atualizacao(10.0, p, 10.0 + 2.5 * p) - (10.0 + 3.0 * p)).abs() < 1e-9);
        // Exatamente numa atualização: a próxima, não a mesma.
        assert!((proxima_atualizacao(10.0, p, 10.0) - (10.0 + p)).abs() < 1e-12);
        // O Windows deu a atualização que ainda vai acontecer, com folga: é ela (antes pulava uma, um período de atraso).
        assert!((proxima_atualizacao(10.0, p, 9.996) - 10.0).abs() < 1e-12);
        // Perto demais para compor a tempo: a seguinte.
        assert!((proxima_atualizacao(10.0, p, 9.9995) - (10.0 + p)).abs() < 1e-12);
        assert_eq!(proxima_atualizacao(10.0, 0.0, 11.0), 11.0);
    }

    #[test]
    fn a_reserva_pelo_relogio_nao_escorrega() {
        let i = 0.75 / 144.0;
        assert!((proximo_prazo(1.0, 1.001, i) - (1.0 + i)).abs() < 1e-12);
        assert!((proximo_prazo(1.0, 1.05, i) - (1.05 + i)).abs() < 1e-12, "atrasado: recomeça de agora");
    }

    #[test]
    fn a_guarda_desce_do_que_o_gerador_faz_de_verdade() {
        assert_eq!(teto_efetivo(4, Some(2.4)), 3);
        assert_eq!(teto_efetivo(6, Some(2.4)), 3);
        assert_eq!(teto_efetivo(2, Some(2.4)), 2);
        assert_eq!(teto_efetivo(4, None), 4);
        assert_eq!(teto_efetivo(4, Some(2.03)), 2, "ruído não sobe um degrau");
    }

    #[test]
    fn o_auto_diz_quantos_quadros_por_real_chegaram() {
        // GTX 1650, 144 Hz, FiveM a 61: 144 imagens novas por segundo (uma por atualização) = 2,4× (a tela dizia 1×).
        assert!((multiplicador_medido(61, 144).unwrap() - 2.36).abs() < 0.01);
        assert_eq!(multiplicador_medido(0, 144), None);
        assert_eq!(multiplicador_medido(60, 30), Some(1.0), "nunca menos que 1×");
    }

    #[test]
    fn o_atraso_soma_a_fila_inteira_da_sobreposicao() {
        // A fila do jogo acontece ANTES da composição de onde a conta parte: não se desconta.
        assert_eq!(atraso_acrescentado(Some(9.0), Some(6.0)), Some(15.0));
        assert_eq!(atraso_acrescentado(Some(9.0), None), Some(9.0), "sem PresentMon, como antes");
        assert_eq!(atraso_acrescentado(None, Some(6.0)), None);
    }

    #[test]
    fn a_vaga_de_um_present_sem_vez_e_devolvida() {
        let mut divida = Divida::default();
        // O real passa da espera e sai de todo jeito: fica devendo.
        assert!(!esperar_a_vez_pagando(&mut divida, 50, true, |_| false));
        assert_eq!(divida.vagas, 1);
        // Os dois quadros saem da fila: duas vagas no semáforo (uma a mais que o certo).
        let mut vagas = 2u32;
        let mut tentar = |_ms: u32| {
            if vagas > 0 {
                vagas -= 1;
                true
            } else {
                false
            }
        };
        assert!(esperar_a_vez_pagando(&mut divida, 2, false, &mut tentar));
        assert_eq!(divida.vagas, 0);
        // Sem vaga sobrando: a fila não aceita um segundo quadro.
        assert!(!esperar_a_vez_pagando(&mut divida, 2, false, &mut tentar));
        assert_eq!(divida.vagas, 0, "o gerado sem vez não sai, e não deve nada");
    }

    #[test]
    fn uma_vaga_perdida_nao_prende_a_tela_em_20_fps() {
        // O semáforo e a fila do Windows, com a vaga perdida: nada livre e nada na fila.
        let (mut vagas, mut fila) = (0u32, 0u32);
        let mut divida = Divida::default();
        let mut gerados_que_sairam = Vec::new();
        for volta in 0..8 {
            for fase in [Fase::Real, Fase::Gerado(0.5)] {
                let vez = vez_no_modo_fixo(fase, |ms, sai| {
                    esperar_a_vez_pagando(&mut divida, ms, sai, |_| {
                        if vagas > 0 {
                            vagas -= 1;
                            true
                        } else {
                            false
                        }
                    })
                });
                if vez {
                    fila += 1;
                }
                if let Fase::Gerado(_) = fase {
                    gerados_que_sairam.push((volta, vez));
                }
                // Uma atualização da tela: o quadro da fila sai e devolve a vaga.
                if fila > 0 {
                    fila -= 1;
                    vagas += 1;
                }
            }
        }
        assert!(
            gerados_que_sairam.iter().skip(3).all(|(_, saiu)| *saiu),
            "depois de se acertar, todo gerado sai: {:?}",
            gerados_que_sairam
        );
    }

    #[test]
    fn o_ritmo_na_tela_conta_repetidas_e_engasgos() {
        let p = 1.0 / 144.0;
        // Um quadro por atualização: liso.
        let liso: Vec<f64> = (0..144).map(|i| i as f64 * p).collect();
        let r = ritmo_na_tela(&liso, p).unwrap();
        assert!((r.por_segundo - 144.0).abs() < 1.0 && r.repetidas_pct < 0.1 && r.engasgos == 0, "{r:?}");
        // Um em cada dois repetido (o 2× virando 1×): metade das atualizações repetidas.
        let metade: Vec<f64> = (0..72).map(|i| i as f64 * 2.0 * p).collect();
        let r = ritmo_na_tela(&metade, p).unwrap();
        assert!((r.repetidas_pct - 50.0).abs() < 1.0 && r.engasgos == 0, "{r:?}");
        // Um buraco de 4 atualizações: um engasgo.
        let mut buraco = liso.clone();
        buraco.drain(50..53);
        assert_eq!(ritmo_na_tela(&buraco, p).unwrap().engasgos, 1);
        // Pausa longa (menu, jogo atrás) não vira engasgo.
        let mut pausa = liso.clone();
        for t in pausa.iter_mut().skip(70) {
            *t += 2.0;
        }
        assert_eq!(ritmo_na_tela(&pausa, p).unwrap().engasgos, 0);
        assert_eq!(ritmo_na_tela(&liso[..5], p), None);
    }

    #[test]
    fn a_media_da_fila_ignora_o_absurdo() {
        assert_eq!(media_da_fila(None, &[]), None);
        assert_eq!(media_da_fila(None, &[10.0]), Some(10.0));
        assert_eq!(media_da_fila(Some(10.0), &[500.0, -1.0]), Some(10.0));
        assert!((media_da_fila(Some(10.0), &[30.0]).unwrap() - 11.0).abs() < 1e-9);
    }

    /// Simulação do 2× fixo no limite do monitor: a fila de 1 quadro esvazia na atualização seguinte ao `Present`.
    /// Devolve a fração dos gerados que chegou à tela.
    fn gerados_na_tela(hz: f64, fps: f64, variacao: f64, espera_do_gerado_ms: f64) -> f64 {
        let periodo = 1.0 / hz;
        let intervalo = 1.0 / fps;
        let mut ultimo_present = -1.0f64;
        let (mut gerados, mut mostrados) = (0u32, 0u32);
        let mut semente = 12345u64;
        for i in 0..2000 {
            semente = semente.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let ruido = ((semente >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 2.0 * variacao;
            let chegada = i as f64 * intervalo + ruido;
            for a in agenda(chegada, intervalo, 2) {
                let livre = (ultimo_present / periodo).floor() * periodo + periodo;
                let t = a.instante.max(ultimo_present);
                let vez = vez_no_modo_fixo(a.fase, |ms, _| {
                    let espera = match a.fase {
                        Fase::Gerado(_) => espera_do_gerado_ms,
                        Fase::Real => ms as f64,
                    } / 1000.0;
                    livre <= t + espera
                });
                if let Fase::Gerado(_) = a.fase {
                    gerados += 1;
                    if vez {
                        mostrados += 1;
                    }
                }
                if vez {
                    ultimo_present = t.max(livre);
                }
            }
        }
        mostrados as f64 / gerados as f64
    }

    #[test]
    fn o_2x_no_limite_do_monitor_mostra_os_gerados() {
        let espera = ESPERA_DO_GERADO_MS as f64;
        for fps in [60.0, 70.0, 72.0] {
            let f = gerados_na_tela(144.0, fps, 0.0005, espera);
            assert!(f >= 0.99, "144 Hz, {fps} FPS: {:.1}% dos gerados", f * 100.0);
        }
        // Acima de Hz/2 o 2× já não cabe inteiro (`multiplicador_que_cabe` tolera 3%): ainda assim, quase todos.
        assert!(gerados_na_tela(144.0, 74.0, 0.0005, espera) >= 0.95);
        // Sem espera, o caso de 72 FPS perdia um quarto: a trava de que a espera importa.
        assert!(gerados_na_tela(144.0, 72.0, 0.0005, 0.0) < 0.9);
    }

    #[test]
    fn no_modo_fixo_o_gerado_espera_pouco_e_o_real_sempre_sai() {
        let mut pedido = None;
        assert!(!vez_no_modo_fixo(Fase::Gerado(0.5), |ms, sai| {
            pedido = Some((ms, sai));
            false
        }));
        assert_eq!(pedido, Some((ESPERA_DO_GERADO_MS, false)), "o gerado quase não segura o laço");
        assert!(vez_no_modo_fixo(Fase::Gerado(0.5), |_, _| true));
        assert!(vez_no_modo_fixo(Fase::Real, |ms, sai| {
            pedido = Some((ms, sai));
            false
        }));
        assert_eq!(pedido, Some((ESPERA_DO_REAL_MS, true)));
    }

    #[test]
    fn niveis_arredondam_para_cima() {
        assert_eq!(niveis(1920, 1080), ((480, 270), (240, 135), (120, 68)));
    }
}
