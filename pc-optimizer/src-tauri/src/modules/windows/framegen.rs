// Laboratório de geração de quadros. O quadro gerado é INTERPOLADO: a tela fica mais fluida, o jogo não responde
// mais rápido (o real mais novo espera o gerado). Separa RENDERIZADOS (resposta) de EXIBIDOS (fluidez), cada número
// MEDIDO, ESTIMADO (base dita) ou DESCONHECIDO. Não liga geração em jogo nenhum, não injeta nada, não grava ajuste
// de driver fora do cabeçalho público (as travas no fim seguram).

use serde::{Deserialize, Serialize};

use super::bottleneck::Limite;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fabricante {
    Nvidia,
    Amd,
    Intel,
    Desconhecido,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arquitetura {
    /// Blackwell. DLSS 4 com geração múltipla (até 4x) e Smooth Motion.
    Rtx50,
    /// Ada Lovelace. DLSS 3 com geração de quadros 2x.
    Rtx40,
    Rtx30,
    Rtx20,
    /// GTX 16, GTX 10 e anteriores. Sem núcleo dedicado para DLSS.
    GtxOuAnterior,
    Rx9000,
    Rx7000,
    Rx6000,
    RxAnterior,
    IntelArc,
    IntelIntegrada,
    Desconhecida,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placa {
    pub nome: String,
    pub fabricante: Fabricante,
    pub arquitetura: Arquitetura,
    pub driver: Option<String>,
}

/// Número de modelo depois de um marcador (`rtx 4070` → 4070).
fn numero_depois(nome: &str, marcador: &str) -> Option<u32> {
    let resto = &nome[nome.find(marcador)? + marcador.len()..];
    let digitos: String = resto
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digitos.parse().ok()
}

fn classificar_uma(nome: &str) -> (Fabricante, Arquitetura) {
    let n = nome.to_lowercase();

    if n.contains("nvidia") || n.contains("geforce") || n.contains("rtx") || n.contains("gtx") {
        let arquitetura = match numero_depois(&n, "rtx") {
            // Placas de notebook e de mesa usam a mesma numeração de série.
            Some(m) if (5000..6000).contains(&m) => Arquitetura::Rtx50,
            Some(m) if (4000..5000).contains(&m) => Arquitetura::Rtx40,
            Some(m) if (3000..4000).contains(&m) => Arquitetura::Rtx30,
            Some(m) if (2000..3000).contains(&m) => Arquitetura::Rtx20,
            // "RTX A4000" e afins: estação de trabalho, sem série para mapear.
            Some(_) | None if n.contains("rtx") => Arquitetura::Desconhecida,
            _ if n.contains("gtx") || n.contains("gt ") || n.contains("mx") => {
                Arquitetura::GtxOuAnterior
            }
            _ => Arquitetura::Desconhecida,
        };
        return (Fabricante::Nvidia, arquitetura);
    }

    if n.contains("radeon") || n.contains("amd") {
        let arquitetura = match numero_depois(&n, "rx") {
            Some(m) if (9000..10000).contains(&m) => Arquitetura::Rx9000,
            Some(m) if (7000..8000).contains(&m) => Arquitetura::Rx7000,
            Some(m) if (6000..7000).contains(&m) => Arquitetura::Rx6000,
            Some(_) => Arquitetura::RxAnterior,
            None => Arquitetura::Desconhecida,
        };
        return (Fabricante::Amd, arquitetura);
    }

    if n.contains("intel") {
        let arquitetura = if n.contains("arc") {
            Arquitetura::IntelArc
        } else {
            Arquitetura::IntelIntegrada
        };
        return (Fabricante::Intel, arquitetura);
    }

    (Fabricante::Desconhecido, Arquitetura::Desconhecida)
}

/// Notebook com duas placas chega como "Intel(R) UHD Graphics + NVIDIA ... Laptop GPU": decide a dedicada.
pub fn classificar_placa(nome: &str, driver: Option<String>) -> Placa {
    let prioridade = |f: Fabricante| match f {
        Fabricante::Nvidia | Fabricante::Amd => 2,
        Fabricante::Intel => 1,
        Fabricante::Desconhecido => 0,
    };

    let (escolhida, fabricante, arquitetura) = nome
        .split(" + ")
        .map(|parte| {
            let (f, a) = classificar_uma(parte);
            (parte.trim(), f, a)
        })
        // Intel Arc é dedicada: sobe para o mesmo nível de NVIDIA e AMD.
        .max_by_key(|(_, f, a)| {
            if *a == Arquitetura::IntelArc {
                2
            } else {
                prioridade(*f)
            }
        })
        .unwrap_or((nome, Fabricante::Desconhecido, Arquitetura::Desconhecida));

    Placa {
        nome: escolhida.to_string(),
        fabricante,
        arquitetura,
        driver,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tecnologia {
    /// NVIDIA DLSS Frame Generation (DLSS 3 e 4). Dentro do jogo.
    DlssFg,
    /// AMD FSR 3 Frame Generation. Dentro do jogo, em qualquer fabricante.
    FsrFg,
    /// NVIDIA Smooth Motion. No driver, para jogos sem geração própria.
    SmoothMotion,
    /// AMD Fluid Motion Frames. No driver (Adrenalin).
    Afmf,
    /// Lossless Scaling. Programa à parte, pago, vendido na Steam.
    LosslessScaling,
    /// O gerador do próprio Otimiza (`geracao/`). Por fora do jogo, como o
    /// Lossless Scaling: captura, estima movimento na GPU e apresenta por cima.
    Otimiza,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tipo {
    /// O jogo gera. Os quadros gerados saem pela mesma entrega do jogo.
    Nativa,
    /// O driver gera, depois que o jogo entregou. O Windows só vê os reais.
    Driver,
    /// Outro programa gera, na janela dele. Os dois ritmos são mensuráveis.
    Externa,
}

pub fn tipo_de(tecnologia: Tecnologia) -> Tipo {
    match tecnologia {
        Tecnologia::DlssFg | Tecnologia::FsrFg => Tipo::Nativa,
        Tecnologia::SmoothMotion | Tecnologia::Afmf => Tipo::Driver,
        Tecnologia::LosslessScaling | Tecnologia::Otimiza => Tipo::Externa,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disponibilidade {
    /// A placa suporta; falta o jogo oferecer a opção no menu.
    DependeDoJogo,
    /// A placa suporta; depende da versão do driver instalada.
    DependeDoDriver,
    /// Instalado nesta máquina.
    Instalado,
    /// Vem dentro do Otimiza.
    Integrado,
    /// Não foi encontrado na pasta padrão. Pode estar em outra biblioteca.
    NaoEncontrado,
    /// A placa não tem suporte oficial.
    NaoSuportada,
    /// O fabricante não confirma suporte para esta placa. Pode funcionar.
    SemConfirmacao,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcaoDeGeracao {
    pub tecnologia: Tecnologia,
    pub tipo: Tipo,
    pub disponibilidade: Disponibilidade,
    pub multiplicadores: Vec<u8>,
    pub o_jogo_oferece: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JogoConhecido {
    /// FiveM roda sobre a base Legacy do GTA V: sem geração própria, e
    /// quase sempre preso no processador em servidor cheio.
    FiveM,
    /// GTA V Enhanced (2025) traz DLSS e FSR com geração de quadros.
    GtaVEnhanced,
    /// GTA V Legacy: sem geração própria.
    GtaVLegacy,
}

pub fn reconhecer_jogo(processo: &str) -> Option<JogoConhecido> {
    let p = processo.to_lowercase();
    // FiveM primeiro: o processo dele também tem "gta" no nome.
    if p.contains("fivem") {
        Some(JogoConhecido::FiveM)
    } else if p.contains("gta5_enhanced") {
        Some(JogoConhecido::GtaVEnhanced)
    } else if p == "gta5.exe" || p == "gta5" {
        Some(JogoConhecido::GtaVLegacy)
    } else {
        None
    }
}

fn geracoes_nativas(jogo: JogoConhecido) -> &'static [Tecnologia] {
    match jogo {
        JogoConhecido::GtaVEnhanced => &[Tecnologia::DlssFg, Tecnologia::FsrFg],
        JogoConhecido::FiveM | JogoConhecido::GtaVLegacy => &[],
    }
}

/// `lossless_instalado` é `None` quando não houve como procurar.
pub fn catalogo(
    placa: &Placa,
    jogo: Option<JogoConhecido>,
    lossless_instalado: Option<bool>,
) -> Vec<OpcaoDeGeracao> {
    use Arquitetura::*;
    let a = placa.arquitetura;

    let oferece = |t: Tecnologia| jogo.map(|j| geracoes_nativas(j).contains(&t));

    let dlss = match a {
        Rtx50 => (Disponibilidade::DependeDoJogo, vec![2, 3, 4]),
        Rtx40 => (Disponibilidade::DependeDoJogo, vec![2]),
        _ => (Disponibilidade::NaoSuportada, vec![]),
    };

    // A AMD recomenda FSR 3 com geração a partir de RX 6000 e RTX 20; abaixo
    // disso roda em muitos casos, sem garantia.
    let fsr = match a {
        Rtx50 | Rtx40 | Rtx30 | Rtx20 | Rx9000 | Rx7000 | Rx6000 | IntelArc => {
            (Disponibilidade::DependeDoJogo, vec![2])
        }
        _ => (Disponibilidade::SemConfirmacao, vec![2]),
    };

    // Smooth Motion nasceu na RTX 50; a RTX 40 recebeu depois, em driver mais
    // novo. Sem ler a versão exata, o produto não afirma nenhuma das duas.
    let smooth = match a {
        Rtx50 | Rtx40 => (Disponibilidade::DependeDoDriver, vec![2]),
        _ => (Disponibilidade::NaoSuportada, vec![]),
    };

    let afmf = match a {
        Rx9000 | Rx7000 | Rx6000 => (Disponibilidade::DependeDoDriver, vec![2]),
        _ => (Disponibilidade::NaoSuportada, vec![]),
    };

    let lossless = match lossless_instalado {
        Some(true) => Disponibilidade::Instalado,
        _ => Disponibilidade::NaoEncontrado,
    };

    vec![
        OpcaoDeGeracao {
            tecnologia: Tecnologia::DlssFg,
            tipo: Tipo::Nativa,
            disponibilidade: dlss.0,
            multiplicadores: dlss.1,
            o_jogo_oferece: oferece(Tecnologia::DlssFg),
        },
        OpcaoDeGeracao {
            tecnologia: Tecnologia::FsrFg,
            tipo: Tipo::Nativa,
            disponibilidade: fsr.0,
            multiplicadores: fsr.1,
            o_jogo_oferece: oferece(Tecnologia::FsrFg),
        },
        OpcaoDeGeracao {
            tecnologia: Tecnologia::SmoothMotion,
            tipo: Tipo::Driver,
            disponibilidade: smooth.0,
            multiplicadores: smooth.1,
            o_jogo_oferece: None,
        },
        OpcaoDeGeracao {
            tecnologia: Tecnologia::Afmf,
            tipo: Tipo::Driver,
            disponibilidade: afmf.0,
            multiplicadores: afmf.1,
            o_jogo_oferece: None,
        },
        OpcaoDeGeracao {
            tecnologia: Tecnologia::Otimiza,
            tipo: Tipo::Externa,
            disponibilidade: Disponibilidade::Integrado,
            multiplicadores: vec![2, 3, 4],
            o_jogo_oferece: None,
        },
        OpcaoDeGeracao {
            tecnologia: Tecnologia::LosslessScaling,
            tipo: Tipo::Externa,
            disponibilidade: lossless,
            multiplicadores: vec![2, 3, 4],
            o_jogo_oferece: None,
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaseDaEstimativa {
    /// Quadros medidos vezes o multiplicador que a pessoa escolheu.
    MedidoVezesMultiplicador,
    /// Quadros medidos divididos pelo multiplicador (geração dentro do jogo:
    /// os gerados passam pela mesma entrega dos reais).
    MedidoDivididoPeloMultiplicador,
    /// Um quadro dura 1000 / FPS milissegundos.
    IntervaloDoQuadro,
    /// Interpolar exige segurar o quadro real mais novo por cerca de um
    /// intervalo de quadro renderizado.
    UmQuadroRetido,
    /// Sem geração ligada, não há atraso de geração.
    SemGeracao,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotivoDesconhecido {
    /// Medir atraso de ponta a ponta exige sensor na tela (ou Reflex
    /// Analyzer). Software sozinho não vê.
    ExigeSensor,
    /// O gerador externo não entregou quadros durante a medição.
    GeradorSemQuadros,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "origem")]
pub enum Valor {
    Medido { valor: f64 },
    Estimado { valor: f64, base: BaseDaEstimativa },
    Desconhecido { motivo: MotivoDesconhecido },
}

impl Valor {
    pub fn numero(&self) -> Option<f64> {
        match self {
            Valor::Medido { valor } | Valor::Estimado { valor, .. } => Some(*valor),
            Valor::Desconhecido { .. } => None,
        }
    }
}

fn arredondar(x: f64, casas: i32) -> f64 {
    let f = 10f64.powi(casas);
    (x * f).round() / f
}

/// Abaixo disto os percentis viram meia dúzia de quadros.
const AMOSTRAS_MINIMAS: usize = 300;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ritmo {
    pub amostras: usize,
    pub media_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub low_1pct_fps: f64,
    /// Média do 0,1% pior, em FPS. É o engasgo raro que o 1% ainda dilui.
    pub low_01pct_fps: f64,
    /// Diferença média entre um intervalo e o seguinte. É o "tremido" que
    /// o olho sente mesmo com a média alta.
    pub oscilacao_ms: f64,
    pub picos_por_minuto: f64,
    /// 0 a 100. Cem é todo quadro com a mesma duração.
    pub consistencia: f64,
    pub confiavel: bool,
}

fn percentil(ordenados: &[f64], p: f64) -> f64 {
    if ordenados.is_empty() {
        return 0.0;
    }
    let posto = ((p / 100.0) * ordenados.len() as f64).ceil() as usize;
    ordenados[posto.clamp(1, ordenados.len()) - 1]
}

pub fn ritmo(intervalos_ms: &[f64]) -> Ritmo {
    let validos: Vec<f64> = intervalos_ms
        .iter()
        .copied()
        .filter(|ms| ms.is_finite() && *ms > 0.0)
        .collect();

    if validos.is_empty() {
        return Ritmo {
            amostras: 0,
            media_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            low_1pct_fps: 0.0,
            low_01pct_fps: 0.0,
            oscilacao_ms: 0.0,
            picos_por_minuto: 0.0,
            consistencia: 0.0,
            confiavel: false,
        };
    }

    let n = validos.len();
    let soma: f64 = validos.iter().sum();
    let media = soma / n as f64;

    let mut ordenados = validos.clone();
    ordenados.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mediana = ordenados[n / 2];

    // Mesma definição de `frames::estatistica`: média do 1% pior, não P99.
    let quantos = (n / 100).max(1);
    let media_piores = ordenados[n - quantos..].iter().sum::<f64>() / quantos as f64;
    let quantos_01 = (n / 1000).max(1);
    let media_piores_01 = ordenados[n - quantos_01..].iter().sum::<f64>() / quantos_01 as f64;

    let oscilacao = if n > 1 {
        validos.windows(2).map(|p| (p[1] - p[0]).abs()).sum::<f64>() / (n - 1) as f64
    } else {
        0.0
    };

    // Pico é o quadro que o olho percebe: bem mais longo que o normal DAQUELA
    // partida, e mais longo em absoluto — 1 ms a mais a 500 FPS não se vê.
    let limiar = (mediana * 2.0).max(mediana + 4.0);
    let picos = validos.iter().filter(|ms| **ms > limiar).count();
    let minutos = soma / 60_000.0;

    let consistencia = if media > 0.0 {
        (100.0 * (1.0 - (oscilacao / media).min(1.0))).max(0.0)
    } else {
        0.0
    };

    Ritmo {
        amostras: n,
        media_ms: arredondar(media, 2),
        p95_ms: arredondar(percentil(&ordenados, 95.0), 2),
        p99_ms: arredondar(percentil(&ordenados, 99.0), 2),
        low_1pct_fps: arredondar(if media_piores > 0.0 { 1000.0 / media_piores } else { 0.0 }, 1),
        low_01pct_fps: arredondar(if media_piores_01 > 0.0 { 1000.0 / media_piores_01 } else { 0.0 }, 1),
        oscilacao_ms: arredondar(oscilacao, 2),
        picos_por_minuto: arredondar(if minutos > 0.0 { picos as f64 / minutos } else { 0.0 }, 1),
        consistencia: arredondar(consistencia, 0),
        confiavel: n >= AMOSTRAS_MINIMAS,
    }
}

fn ritmo_irregular(r: &Ritmo) -> bool {
    r.confiavel && (r.consistencia < 70.0 || r.picos_por_minuto > 10.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Prontidao {
    NaoRecomendada,
    Marginal,
    Boa,
    Excelente,
    /// O jogo já entrega o que o monitor mostra. Gerar mais não aparece.
    Desnecessaria,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotivoDaProntidao {
    /// Menos de 30 quadros reais: o gerado vira borrão e o atraso dobra.
    BaseMuitoBaixa,
    /// Entre 30 e 45: funciona, com artefato visível e atraso sentido.
    BaseBaixa,
    /// Entre 45 e 60: o fabricante considera aceitável.
    BaseAceitavel,
    /// 60 ou mais: a faixa em que a geração foi pensada para operar.
    BaseIdeal,
    /// O ritmo já é irregular: a geração copia a irregularidade.
    RitmoIrregular,
    JaNoLimiteDoMonitor,
    /// O processador é o teto. A geração não depende dele, então é um dos
    /// poucos casos em que ela sobe a fluidez — sem mudar a resposta.
    LimitadoPeloProcessador,
    AmostraCurta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeituraDeProntidao {
    pub nivel: Prontidao,
    pub motivos: Vec<MotivoDaProntidao>,
}

fn rebaixar(p: Prontidao) -> Prontidao {
    match p {
        Prontidao::Excelente => Prontidao::Boa,
        Prontidao::Boa => Prontidao::Marginal,
        outro => outro,
    }
}

/// Pelos quadros REAIS medidos com a geração desligada.
pub fn prontidao(
    fps_base: f64,
    hz: u32,
    ritmo_base: &Ritmo,
    limite: Option<Limite>,
) -> LeituraDeProntidao {
    let mut motivos = Vec::new();

    if hz > 0 && fps_base >= hz as f64 * 0.95 {
        motivos.push(MotivoDaProntidao::JaNoLimiteDoMonitor);
        return LeituraDeProntidao { nivel: Prontidao::Desnecessaria, motivos };
    }

    let (mut nivel, motivo) = if fps_base < 30.0 {
        (Prontidao::NaoRecomendada, MotivoDaProntidao::BaseMuitoBaixa)
    } else if fps_base < 45.0 {
        (Prontidao::Marginal, MotivoDaProntidao::BaseBaixa)
    } else if fps_base < 60.0 {
        (Prontidao::Boa, MotivoDaProntidao::BaseAceitavel)
    } else {
        (Prontidao::Excelente, MotivoDaProntidao::BaseIdeal)
    };
    motivos.push(motivo);

    if !ritmo_base.confiavel {
        motivos.push(MotivoDaProntidao::AmostraCurta);
    } else if ritmo_irregular(ritmo_base) {
        motivos.push(MotivoDaProntidao::RitmoIrregular);
        // Marginal com ritmo ruim desce para "não recomendada"; o piso não muda.
        nivel = if nivel == Prontidao::Marginal {
            Prontidao::NaoRecomendada
        } else {
            rebaixar(nivel)
        };
    }

    if matches!(limite, Some(Limite::CpuUmNucleo) | Some(Limite::CpuTodos)) {
        motivos.push(MotivoDaProntidao::LimitadoPeloProcessador);
    }

    LeituraDeProntidao { nivel, motivos }
}

pub fn cpu_limitada(limite: Limite) -> bool {
    matches!(limite, Limite::CpuUmNucleo | Limite::CpuTodos)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Latencia {
    /// Quanto dura um quadro real. Piso do que o jogo consegue responder.
    pub renderizacao_ms: Valor,
    /// NUNCA negativo: geração de quadros não reduz atraso.
    pub acrescimo_da_geracao_ms: Valor,
    pub jogo_ms: Valor,
    pub tela_ms: Valor,
}

pub fn latencia(fps_renderizado: Option<f64>, geracao_ligada: bool) -> Latencia {
    let quadro = fps_renderizado.filter(|f| *f > 0.0).map(|f| 1000.0 / f);

    let renderizacao_ms = match quadro {
        Some(ms) => Valor::Estimado { valor: arredondar(ms, 1), base: BaseDaEstimativa::IntervaloDoQuadro },
        None => Valor::Desconhecido { motivo: MotivoDesconhecido::ExigeSensor },
    };

    let acrescimo_da_geracao_ms = match (geracao_ligada, quadro) {
        (false, _) => Valor::Estimado { valor: 0.0, base: BaseDaEstimativa::SemGeracao },
        (true, Some(ms)) => Valor::Estimado { valor: arredondar(ms, 1), base: BaseDaEstimativa::UmQuadroRetido },
        (true, None) => Valor::Desconhecido { motivo: MotivoDesconhecido::ExigeSensor },
    };

    Latencia {
        renderizacao_ms,
        acrescimo_da_geracao_ms,
        jogo_ms: Valor::Desconhecido { motivo: MotivoDesconhecido::ExigeSensor },
        tela_ms: Valor::Desconhecido { motivo: MotivoDesconhecido::ExigeSensor },
    }
}

#[derive(Debug, Clone)]
pub struct Contagem {
    pub fps: f64,
    pub intervalos_ms: Vec<f64>,
}

/// Quantos intervalos viajam para o gráfico. O resto fica no Rust.
const PONTOS_DO_GRAFICO: usize = 240;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rodada {
    /// `None` é a geração desligada.
    pub tecnologia: Option<Tecnologia>,
    pub multiplicador: u8,
    pub fps_renderizado: Valor,
    pub fps_exibido: Valor,
    pub ritmo_renderizado: Ritmo,
    pub ritmo_exibido: Option<Ritmo>,
    pub latencia: Latencia,
    /// Exibidos ÷ renderizados, só na geração externa: com limite ou vsync no gerador, chega menos que o escolhido.
    pub multiplicador_medido: Option<f64>,
    pub amostra_ms: Vec<f64>,
    pub segundos: f64,
}

/// Monta a rodada respeitando o que cada tipo de geração deixa medir.
///
/// | geração  | renderizado             | exibido                         |
/// |----------|-------------------------|---------------------------------|
/// | nenhuma  | medido                  | medido (é o mesmo)              |
/// | driver   | medido                  | ESTIMADO: medido × multiplicador |
/// | externa  | medido (processo jogo)  | medido (processo do gerador)     |
/// | nativa   | ESTIMADO: medido ÷ mult | medido                           |
pub fn montar_rodada(
    tecnologia: Option<Tecnologia>,
    multiplicador: u8,
    jogo: &Contagem,
    gerador: Option<&Contagem>,
    segundos: f64,
) -> Rodada {
    let mult = if tecnologia.is_some() { multiplicador.clamp(2, 4) } else { 1 };
    let medido = |f: f64| Valor::Medido { valor: arredondar(f, 1) };
    let ritmo_jogo = ritmo(&jogo.intervalos_ms);

    let (fps_renderizado, fps_exibido, ritmo_renderizado, ritmo_exibido) = match tecnologia.map(tipo_de) {
        None => (medido(jogo.fps), medido(jogo.fps), ritmo_jogo.clone(), Some(ritmo_jogo)),
        Some(Tipo::Driver) => (
            medido(jogo.fps),
            Valor::Estimado {
                valor: arredondar(jogo.fps * mult as f64, 1),
                base: BaseDaEstimativa::MedidoVezesMultiplicador,
            },
            ritmo_jogo,
            None,
        ),
        Some(Tipo::Externa) => match gerador {
            Some(g) => (medido(jogo.fps), medido(g.fps), ritmo_jogo, Some(ritmo(&g.intervalos_ms))),
            None => (
                medido(jogo.fps),
                Valor::Desconhecido { motivo: MotivoDesconhecido::GeradorSemQuadros },
                ritmo_jogo,
                None,
            ),
        },
        Some(Tipo::Nativa) => {
            // O ritmo medido é o da tela; o dos quadros reais não é separável.
            let renderizado = Valor::Estimado {
                valor: arredondar(jogo.fps / mult as f64, 1),
                base: BaseDaEstimativa::MedidoDivididoPeloMultiplicador,
            };
            (renderizado, medido(jogo.fps), ritmo_jogo.clone(), Some(ritmo_jogo))
        }
    };

    let latencia = latencia(fps_renderizado.numero(), tecnologia.is_some());
    let multiplicador_medido = match (&fps_renderizado, &fps_exibido) {
        (Valor::Medido { valor: r }, Valor::Medido { valor: e }) if tecnologia.is_some() && *r > 0.0 => {
            Some(arredondar(e / r, 2))
        }
        _ => None,
    };

    Rodada {
        tecnologia,
        multiplicador: mult,
        fps_renderizado,
        fps_exibido,
        ritmo_renderizado,
        ritmo_exibido,
        latencia,
        multiplicador_medido,
        amostra_ms: jogo.intervalos_ms.iter().take(PONTOS_DO_GRAFICO).map(|ms| arredondar(*ms, 2)).collect(),
        segundos: arredondar(segundos, 1),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perfil {
    /// Resposta acima de tudo. Quadro exibido não vale ponto nenhum.
    Competitivo,
    Equilibrado,
    MaximaFluidez,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pontuacao {
    pub total: f64,
    pub fluidez: f64,
    pub resposta: f64,
    pub estabilidade: f64,
}

fn pesos(perfil: Perfil) -> (f64, f64, f64) {
    match perfil {
        Perfil::Competitivo => (0.0, 0.7, 0.3),
        Perfil::Equilibrado => (0.35, 0.35, 0.3),
        Perfil::MaximaFluidez => (0.6, 0.1, 0.3),
    }
}

/// Atraso estimado desta rodada: um quadro real, mais o que a geração segura.
fn atraso_estimado(r: &Rodada) -> Option<f64> {
    Some(r.latencia.renderizacao_ms.numero()? + r.latencia.acrescimo_da_geracao_ms.numero()?)
}

/// O ritmo que representa o que a pessoa VÊ.
fn ritmo_visto(r: &Rodada) -> &Ritmo {
    r.ritmo_exibido.as_ref().unwrap_or(&r.ritmo_renderizado)
}

/// A pontuação de geração de quadros. Não é FPS: é fluidez, resposta e
/// estabilidade, pesadas pelo que a pessoa disse que quer.
pub fn pontuar(r: &Rodada, perfil: Perfil, hz: u32) -> Pontuacao {
    let tela = if hz > 0 { hz as f64 } else { 60.0 };
    let exibido = r.fps_exibido.numero().or(r.fps_renderizado.numero()).unwrap_or(0.0);
    let fluidez = (exibido / tela * 100.0).clamp(0.0, 100.0);

    let resposta = atraso_estimado(r)
        .map(|ms| (100.0 - (ms - 5.0) * 2.4).clamp(0.0, 100.0))
        .unwrap_or(0.0);

    let visto = ritmo_visto(r);
    let estabilidade = (visto.consistencia - (visto.picos_por_minuto * 2.0).min(30.0)).clamp(0.0, 100.0);

    let (pf, pr, pe) = pesos(perfil);
    Pontuacao {
        total: arredondar(fluidez * pf + resposta * pr + estabilidade * pe, 0),
        fluidez: arredondar(fluidez, 0),
        resposta: arredondar(resposta, 0),
        estabilidade: arredondar(estabilidade, 0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Artefato {
    /// Rastro atrás do que se move rápido.
    Fantasmas,
    /// Mira, minimapa e textos tremendo ou duplicando.
    InterfaceTremida,
    /// Contorno de personagem ou carro "rasgando" ao girar a câmera.
    BordasQuebradas,
    /// A tela parece lisa, mas o controle parece atrasado.
    RespostaPesada,
    Borrado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intensidade {
    Nenhuma,
    Leve,
    Forte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NivelDeArtefato {
    Limpo,
    Aceitavel,
    Incomodo,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NotaDeArtefatos {
    pub nota: f64,
    pub nivel: NivelDeArtefato,
}

/// O teste é da pessoa, não do Otimiza: nenhuma leitura automática de imagem
/// é confiável o bastante para julgar artefato de interpolação, e fingir que
/// é seria o número inventado que este produto se recusa a mostrar.
pub fn avaliar_artefatos(respostas: &[(Artefato, Intensidade)]) -> NotaDeArtefatos {
    let peso = |a: Artefato| match a {
        // Em FiveM e jogo competitivo a interface é onde o olho mora.
        Artefato::InterfaceTremida | Artefato::RespostaPesada => 1.5,
        _ => 1.0,
    };

    let desconto: f64 = respostas
        .iter()
        .map(|(a, i)| {
            peso(*a)
                * match i {
                    Intensidade::Nenhuma => 0.0,
                    Intensidade::Leve => 8.0,
                    Intensidade::Forte => 25.0,
                }
        })
        .sum();

    let nota = (100.0 - desconto).clamp(0.0, 100.0);
    let algum_forte = respostas.iter().any(|(_, i)| *i == Intensidade::Forte);

    let nivel = if algum_forte || nota < 60.0 {
        NivelDeArtefato::Incomodo
    } else if nota < 100.0 {
        NivelDeArtefato::Aceitavel
    } else {
        NivelDeArtefato::Limpo
    };

    NotaDeArtefatos { nota: arredondar(nota, 0), nivel }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decisao {
    Manter,
    Desligar,
    Inconclusivo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotivoDaDecisao {
    AmostraCurta,
    /// O custo de gerar tirou quadros reais demais.
    RenderizadoCaiu,
    RitmoPiorou,
    /// O atraso acrescentado passou do aceitável (10 ms no perfil de resposta,
    /// 20 ms nos outros).
    AtrasoAcrescentadoAlto,
    ArtefatosIncomodos,
    PontuacaoSubiu,
    PontuacaoCaiu,
    DiferencaPequena,
    ExibidoDesconhecido,
    /// A tela recebeu mais quadros do que consegue mostrar.
    PassouDoMonitor,
    /// A base desligada, medida de novo, mudou demais: a cena não se repetiu.
    CenaNaoRepetivel,
    /// O multiplicador que chegou à tela não é o escolhido.
    MultiplicadorNaoConfere,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparacao {
    pub perfil: Perfil,
    pub pontos_desligado: Pontuacao,
    pub pontos_ligado: Pontuacao,
    pub exibido_pct: Option<f64>,
    pub renderizado_pct: Option<f64>,
    pub atraso_acrescentado_ms: Option<f64>,
    pub decisao: Decisao,
    pub motivos: Vec<MotivoDaDecisao>,
    /// 0–100. Cai com amostra curta, cena que não se repete e diferença
    /// pequena. Uma decisão com 40% de confiança é dita como tal.
    pub confianca_pct: f64,
}

/// Quanto a base desligada mudou entre a primeira medição e a conferência.
pub fn deriva_da_cena(primeira: &Rodada, conferencia: &Rodada) -> Option<f64> {
    variacao(primeira.fps_renderizado.numero(), conferencia.fps_renderizado.numero())
}

/// O multiplicador medido bate com o escolhido (tolerância de 0,25)?
pub fn multiplicador_confere(r: &Rodada) -> Option<bool> {
    r.multiplicador_medido.map(|m| (m - r.multiplicador as f64).abs() <= 0.25)
}

pub const DERIVA_MAXIMA_PCT: f64 = 5.0;

fn confianca(motivos: &[MotivoDaDecisao], deriva_pct: Option<f64>) -> f64 {
    let mut c: f64 = 90.0;
    if motivos.contains(&MotivoDaDecisao::AmostraCurta) {
        c -= 45.0;
    }
    if motivos.contains(&MotivoDaDecisao::DiferencaPequena) {
        c -= 15.0;
    }
    if motivos.contains(&MotivoDaDecisao::MultiplicadorNaoConfere) {
        c -= 10.0;
    }
    match deriva_pct.map(f64::abs) {
        Some(d) if d > DERIVA_MAXIMA_PCT => c -= (15.0 + (d - DERIVA_MAXIMA_PCT) * 5.0).min(50.0),
        Some(_) => c += 5.0,
        None => c -= 10.0,
    }
    arredondar(c.clamp(5.0, 98.0), 0)
}

fn variacao(antes: Option<f64>, depois: Option<f64>) -> Option<f64> {
    let (a, d) = (antes?, depois?);
    (a > 0.0).then(|| arredondar((d - a) / a * 100.0, 1))
}

pub const QUEDA_MAXIMA_COMPETITIVO_PCT: f64 = 5.0;
pub const QUEDA_MAXIMA_PCT: f64 = 20.0;
pub const ATRASO_MAXIMO_COMPETITIVO_MS: f64 = 10.0;
/// Vale para TODO perfil desde a 2.9: acima disto o controle pesa para qualquer jogador.
pub const ATRASO_MAXIMO_MS: f64 = 20.0;

/// Liga contra desliga, com a mesma régua para os dois lados.
pub fn comparar(
    desligado: &Rodada,
    ligado: &Rodada,
    perfil: Perfil,
    hz: u32,
    artefatos: Option<NivelDeArtefato>,
    deriva_pct: Option<f64>,
) -> Comparacao {
    let pontos_desligado = pontuar(desligado, perfil, hz);
    let pontos_ligado = pontuar(ligado, perfil, hz);

    let exibido_pct = variacao(desligado.fps_exibido.numero(), ligado.fps_exibido.numero());
    let renderizado_pct = variacao(desligado.fps_renderizado.numero(), ligado.fps_renderizado.numero());
    let atraso_acrescentado_ms = match (atraso_estimado(desligado), atraso_estimado(ligado)) {
        (Some(a), Some(d)) => Some(arredondar(d - a, 1)),
        _ => None,
    };

    let mut motivos = Vec::new();
    let mut contra = false;

    if !desligado.ritmo_renderizado.confiavel || !ligado.ritmo_renderizado.confiavel {
        motivos.push(MotivoDaDecisao::AmostraCurta);
        return Comparacao {
            perfil,
            pontos_desligado,
            pontos_ligado,
            exibido_pct,
            renderizado_pct,
            atraso_acrescentado_ms,
            decisao: Decisao::Inconclusivo,
            confianca_pct: confianca(&motivos, deriva_pct),
            motivos,
        };
    }

    if ligado.fps_exibido.numero().is_none() {
        motivos.push(MotivoDaDecisao::ExibidoDesconhecido);
    }

    if deriva_pct.is_some_and(|d| d.abs() > DERIVA_MAXIMA_PCT) {
        motivos.push(MotivoDaDecisao::CenaNaoRepetivel);
    }

    if multiplicador_confere(ligado) == Some(false) {
        motivos.push(MotivoDaDecisao::MultiplicadorNaoConfere);
    }

    let queda_maxima = if perfil == Perfil::Competitivo {
        QUEDA_MAXIMA_COMPETITIVO_PCT
    } else {
        QUEDA_MAXIMA_PCT
    };
    if renderizado_pct.is_some_and(|v| -v > queda_maxima) {
        motivos.push(MotivoDaDecisao::RenderizadoCaiu);
        contra = true;
    }

    let tolerancia_ritmo = if perfil == Perfil::Competitivo { 5.0 } else { 10.0 };
    if ritmo_visto(ligado).consistencia + tolerancia_ritmo < ritmo_visto(desligado).consistencia {
        motivos.push(MotivoDaDecisao::RitmoPiorou);
        contra = true;
    }

    let limite_de_atraso = if perfil == Perfil::Competitivo { ATRASO_MAXIMO_COMPETITIVO_MS } else { ATRASO_MAXIMO_MS };
    if atraso_acrescentado_ms.is_some_and(|ms| ms > limite_de_atraso) {
        motivos.push(MotivoDaDecisao::AtrasoAcrescentadoAlto);
        contra = true;
    }

    if artefatos == Some(NivelDeArtefato::Incomodo) {
        motivos.push(MotivoDaDecisao::ArtefatosIncomodos);
        contra = true;
    }

    if hz > 0 && ligado.fps_exibido.numero().is_some_and(|f| f > hz as f64 * 1.02) {
        // Não decide sozinho: o conserto é limitar, não desligar.
        motivos.push(MotivoDaDecisao::PassouDoMonitor);
    }

    let diferenca = pontos_ligado.total - pontos_desligado.total;
    let decisao = if contra {
        Decisao::Desligar
    } else if diferenca > 3.0 {
        motivos.push(MotivoDaDecisao::PontuacaoSubiu);
        Decisao::Manter
    } else if diferenca < -3.0 {
        motivos.push(MotivoDaDecisao::PontuacaoCaiu);
        Decisao::Desligar
    } else {
        motivos.push(MotivoDaDecisao::DiferencaPequena);
        Decisao::Inconclusivo
    };

    Comparacao {
        perfil,
        pontos_desligado,
        pontos_ligado,
        exibido_pct,
        renderizado_pct,
        atraso_acrescentado_ms,
        decisao,
        confianca_pct: confianca(&motivos, deriva_pct),
        motivos,
    }
}

/// O modo inteligente: entre as rodadas testadas, a que ganhou do desligado
/// com a maior pontuação. `None` é "fica desligado" — e é uma resposta, não
/// uma falha.
pub fn melhor_rodada(
    desligado: &Rodada,
    testadas: &[Rodada],
    perfil: Perfil,
    hz: u32,
) -> Option<usize> {
    testadas
        .iter()
        .enumerate()
        .map(|(i, r)| (i, comparar(desligado, r, perfil, hz, None, None)))
        .filter(|(_, c)| c.decisao == Decisao::Manter)
        .max_by(|(_, a), (_, b)| {
            a.pontos_ligado
                .total
                .partial_cmp(&b.pontos_ligado.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Casamento {
    /// Mais quadros do que a tela mostra: os excedentes viram rasgo ou fila.
    AcimaDoMonitor,
    NoLimite,
    AbaixoDoMonitor,
    MonitorDesconhecido,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AjusteDeTela {
    pub casamento: Casamento,
    /// Limite de quadros RENDERIZADOS sugerido: com o multiplicador, o exibido
    /// fica logo abaixo da taxa do monitor, dentro da faixa de VRR.
    pub limite_sugerido: Option<u32>,
}

/// `(Hz − folga) ÷ multiplicador`, com folga de 3% e no mínimo 3 quadros.
///
/// A folga mantém o exibido abaixo do teto do monitor, onde G-SYNC e
/// FreeSync continuam agindo. Sem ela, qualquer oscilação passa do teto.
pub fn limite_sugerido(hz: u32, multiplicador: u8) -> Option<u32> {
    if hz == 0 || multiplicador == 0 {
        return None;
    }
    let folga = ((hz as f64 * 0.03).ceil() as u32).max(3);
    let alvo = hz.saturating_sub(folga) / multiplicador as u32;
    (alvo >= 20).then_some(alvo)
}

pub fn ajuste_de_tela(fps_exibido: Option<f64>, hz: u32, multiplicador: u8) -> AjusteDeTela {
    let casamento = match (fps_exibido, hz) {
        (_, 0) | (None, _) => Casamento::MonitorDesconhecido,
        (Some(f), h) if f > h as f64 * 1.02 => Casamento::AcimaDoMonitor,
        (Some(f), h) if f >= h as f64 * 0.95 => Casamento::NoLimite,
        _ => Casamento::AbaixoDoMonitor,
    };
    AjusteDeTela { casamento, limite_sugerido: limite_sugerido(hz, multiplicador.max(1)) }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tela {
    pub largura: u32,
    pub altura: u32,
    pub hz_atual: u32,
    pub hz_maximo: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deteccao {
    pub placa: Placa,
    pub tela: Option<Tela>,
    pub jogo: Option<JogoConhecido>,
    pub processo: Option<String>,
    pub lossless_scaling: Option<bool>,
    pub opcoes: Vec<OpcaoDeGeracao>,
}

pub const PROCESSO_LOSSLESS: &str = "losslessscaling";

/// Só a biblioteca padrão (outro parser de `libraryfolders.vdf` não se paga). Não achar é `NaoEncontrado`, não
/// "não tem".
#[cfg(target_os = "windows")]
fn procurar_lossless() -> Option<bool> {
    if super::frames::encontrar_processo(PROCESSO_LOSSLESS).is_some() {
        return Some(true);
    }
    let base = std::env::var("ProgramFiles(x86)").ok()?;
    let pasta = std::path::Path::new(&base)
        .join("Steam")
        .join("steamapps")
        .join("common")
        .join("Lossless Scaling");
    Some(pasta.is_dir())
}

#[cfg(target_os = "windows")]
fn ler_placa() -> Placa {
    let script = "Get-CimInstance Win32_VideoController | Where-Object { $_.AdapterRAM -gt 0 } | \
                  ForEach-Object { \"$($_.Name)|$($_.DriverVersion)\" }";

    let linhas: Vec<(String, String)> = super::shell::powershell(script)
        .ok()
        .filter(|s| s.success)
        .map(|s| {
            s.stdout
                .lines()
                .filter_map(|l| l.trim().split_once('|'))
                .map(|(n, d)| (n.trim().to_string(), d.trim().to_string()))
                .collect()
        })
        .unwrap_or_default();

    if linhas.is_empty() {
        return classificar_placa("", None);
    }

    let nomes = linhas.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(" + ");
    let mut placa = classificar_placa(&nomes, None);
    placa.driver = linhas
        .iter()
        .find(|(n, _)| *n == placa.nome)
        .map(|(_, d)| d.clone())
        .filter(|d| !d.is_empty());
    placa
}

#[cfg(target_os = "windows")]
pub fn detectar(processo: Option<&str>) -> Deteccao {
    let placa = ler_placa();

    let tela = super::display::monitores()
        .into_iter()
        .find(|m| m.principal)
        .map(|m| Tela {
            largura: m.largura,
            altura: m.altura,
            hz_atual: m.hz_atual,
            hz_maximo: m.hz_maximo(),
        });

    let achado = processo
        .filter(|p| !p.trim().is_empty())
        .and_then(super::frames::encontrar_processo);
    let jogo = achado
        .as_ref()
        .and_then(|(_, nome)| reconhecer_jogo(nome))
        .or_else(|| processo.and_then(reconhecer_jogo));

    let lossless_scaling = procurar_lossless();
    let opcoes = catalogo(&placa, jogo, lossless_scaling);

    Deteccao {
        placa,
        tela,
        jogo,
        processo: achado.map(|(_, nome)| nome),
        lossless_scaling,
        opcoes,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gargalo {
    pub limite: Limite,
    pub cpu_total: f64,
    pub cpu_max_core: f64,
    pub gpu_percent: f64,
    pub vram_used_mb: f64,
    pub vram_total_mb: Option<f64>,
    pub cpu_limitada: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultadoDaRodada {
    pub rodada: Rodada,
    /// Só na rodada desligada: é ela que diz se vale tentar.
    pub prontidao: Option<LeituraDeProntidao>,
    pub gargalo: Option<Gargalo>,
    pub tela: AjusteDeTela,
}

/// Bloqueia pelo tempo pedido. Processador e placa são amostrados AO MESMO TEMPO: depois seria outra cena.
#[cfg(target_os = "windows")]
pub fn medir_rodada(
    processo: &str,
    segundos: u64,
    tecnologia: Option<Tecnologia>,
    multiplicador: u8,
    hz: u32,
) -> Result<ResultadoDaRodada, String> {
    use super::frames;

    let (pid, nome) = frames::encontrar_processo(processo).ok_or_else(|| {
        format!("Não encontrei nenhum processo com `{}` no nome. Abra o jogo antes de medir.", processo)
    })?;

    let gerador = match tecnologia {
        Some(Tecnologia::LosslessScaling) => Some(frames::encontrar_processo(PROCESSO_LOSSLESS).ok_or_else(|| {
            "O Lossless Scaling não está aberto. Abra, ligue a geração na janela do jogo e meça de novo."
                .to_string()
        })?),
        // O gerador do Otimiza apresenta pelo próprio processo do Otimiza.
        Some(Tecnologia::Otimiza) => {
            let estado = super::geracao::estado();
            if estado.situacao != super::geracao::Situacao::Gerando {
                return Err("O gerador do Otimiza não está ligado. Ligue no painel acima e meça de novo.".to_string());
            }
            Some((estado.pid_do_gerador, "otimiza".to_string()))
        }
        _ => None,
    };

    let amostrador = std::thread::spawn(move || super::bottleneck::analisar(segundos));
    let (jogo, ger) = frames::medir_par(pid, &nome, gerador, segundos)?;
    let gargalo = amostrador.join().ok().map(|b| Gargalo {
        cpu_limitada: cpu_limitada(b.limite),
        limite: b.limite,
        cpu_total: b.cpu_total,
        cpu_max_core: b.cpu_max_core,
        gpu_percent: b.gpu_percent,
        vram_used_mb: b.vram_used_mb,
        vram_total_mb: b.vram_total_mb,
    });

    let contagem = |m: &frames::MedicaoCrua| Contagem { fps: m.resumo.fps, intervalos_ms: m.intervalos_ms.clone() };
    let rodada = montar_rodada(
        tecnologia,
        multiplicador,
        &contagem(&jogo),
        ger.as_ref().map(contagem).as_ref(),
        jogo.resumo.seconds,
    );

    let prontidao = tecnologia.is_none().then(|| {
        prontidao(
            rodada.fps_renderizado.numero().unwrap_or(0.0),
            hz,
            &rodada.ritmo_renderizado,
            gargalo.as_ref().map(|g| g.limite),
        )
    });
    let tela = ajuste_de_tela(rodada.fps_exibido.numero(), hz, rodada.multiplicador);

    Ok(ResultadoDaRodada { rodada, prontidao, gargalo, tela })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estavel(ms: f64, n: usize) -> Vec<f64> {
        vec![ms; n]
    }

    fn contagem(fps: f64, n: usize) -> Contagem {
        Contagem { fps, intervalos_ms: estavel(1000.0 / fps, n) }
    }

    fn rodada_off(fps: f64) -> Rodada {
        montar_rodada(None, 1, &contagem(fps, 1000), None, 20.0)
    }

    #[test]
    fn classifica_as_series_nvidia() {
        assert_eq!(classificar_placa("NVIDIA GeForce RTX 5070 Ti", None).arquitetura, Arquitetura::Rtx50);
        assert_eq!(classificar_placa("NVIDIA GeForce RTX 4060 Laptop GPU", None).arquitetura, Arquitetura::Rtx40);
        assert_eq!(classificar_placa("NVIDIA GeForce RTX 3060", None).arquitetura, Arquitetura::Rtx30);
        assert_eq!(classificar_placa("NVIDIA GeForce GTX 1650", None).arquitetura, Arquitetura::GtxOuAnterior);
        assert_eq!(classificar_placa("NVIDIA RTX A4000", None).arquitetura, Arquitetura::Desconhecida);
    }

    #[test]
    fn classifica_amd_e_intel() {
        assert_eq!(classificar_placa("AMD Radeon RX 9070 XT", None).arquitetura, Arquitetura::Rx9000);
        assert_eq!(classificar_placa("AMD Radeon RX 6600", None).arquitetura, Arquitetura::Rx6000);
        assert_eq!(classificar_placa("AMD Radeon RX 580 2048SP", None).arquitetura, Arquitetura::RxAnterior);
        assert_eq!(classificar_placa("Intel(R) Arc(TM) A770", None).arquitetura, Arquitetura::IntelArc);
        assert_eq!(classificar_placa("Intel(R) UHD Graphics 630", None).fabricante, Fabricante::Intel);
    }

    #[test]
    fn em_notebook_a_dedicada_decide() {
        let p = classificar_placa("Intel(R) UHD Graphics + NVIDIA GeForce RTX 4060 Laptop GPU", None);
        assert_eq!(p.arquitetura, Arquitetura::Rtx40);
        assert_eq!(p.nome, "NVIDIA GeForce RTX 4060 Laptop GPU");
    }

    fn opcao(lista: &[OpcaoDeGeracao], t: Tecnologia) -> OpcaoDeGeracao {
        lista.iter().find(|o| o.tecnologia == t).cloned().expect("toda tecnologia aparece")
    }

    #[test]
    fn gtx_nao_tem_dlss_nem_smooth_motion() {
        let placa = classificar_placa("NVIDIA GeForce GTX 1650", None);
        let lista = catalogo(&placa, Some(JogoConhecido::FiveM), Some(false));
        assert_eq!(opcao(&lista, Tecnologia::DlssFg).disponibilidade, Disponibilidade::NaoSuportada);
        assert_eq!(opcao(&lista, Tecnologia::SmoothMotion).disponibilidade, Disponibilidade::NaoSuportada);
        assert_eq!(opcao(&lista, Tecnologia::LosslessScaling).disponibilidade, Disponibilidade::NaoEncontrado);
        assert!(opcao(&lista, Tecnologia::DlssFg).multiplicadores.is_empty());
    }

    #[test]
    fn so_a_rtx_50_tem_geracao_multipla() {
        let rtx50 = catalogo(&classificar_placa("RTX 5080", None), None, None);
        let rtx40 = catalogo(&classificar_placa("RTX 4080", None), None, None);
        assert_eq!(opcao(&rtx50, Tecnologia::DlssFg).multiplicadores, vec![2, 3, 4]);
        assert_eq!(opcao(&rtx40, Tecnologia::DlssFg).multiplicadores, vec![2]);
    }

    #[test]
    fn afmf_so_em_radeon_recente() {
        let rx = catalogo(&classificar_placa("AMD Radeon RX 7800 XT", None), None, None);
        let rtx = catalogo(&classificar_placa("RTX 4070", None), None, None);
        assert_eq!(opcao(&rx, Tecnologia::Afmf).disponibilidade, Disponibilidade::DependeDoDriver);
        assert_eq!(opcao(&rtx, Tecnologia::Afmf).disponibilidade, Disponibilidade::NaoSuportada);
    }

    #[test]
    fn classificacao_de_cada_tecnologia() {
        assert_eq!(tipo_de(Tecnologia::DlssFg), Tipo::Nativa);
        assert_eq!(tipo_de(Tecnologia::FsrFg), Tipo::Nativa);
        assert_eq!(tipo_de(Tecnologia::SmoothMotion), Tipo::Driver);
        assert_eq!(tipo_de(Tecnologia::Afmf), Tipo::Driver);
        assert_eq!(tipo_de(Tecnologia::LosslessScaling), Tipo::Externa);
        assert_eq!(tipo_de(Tecnologia::Otimiza), Tipo::Externa);
    }

    #[test]
    fn o_gerador_do_otimiza_existe_em_qualquer_placa() {
        for nome in ["NVIDIA GeForce GTX 1650", "AMD Radeon RX 580", "Intel(R) UHD Graphics 630", ""] {
            let lista = catalogo(&classificar_placa(nome, None), Some(JogoConhecido::FiveM), None);
            let o = opcao(&lista, Tecnologia::Otimiza);
            assert_eq!(o.disponibilidade, Disponibilidade::Integrado);
            assert_eq!(o.multiplicadores, vec![2, 3, 4]);
        }
    }

    #[test]
    fn fivem_nao_tem_geracao_propria_e_o_enhanced_tem() {
        assert_eq!(reconhecer_jogo("FiveM_b3258_GTAProcess.exe"), Some(JogoConhecido::FiveM));
        assert_eq!(reconhecer_jogo("GTA5_Enhanced.exe"), Some(JogoConhecido::GtaVEnhanced));
        let placa = classificar_placa("RTX 4070", None);
        let fivem = catalogo(&placa, Some(JogoConhecido::FiveM), None);
        let enhanced = catalogo(&placa, Some(JogoConhecido::GtaVEnhanced), None);
        assert_eq!(opcao(&fivem, Tecnologia::DlssFg).o_jogo_oferece, Some(false));
        assert_eq!(opcao(&enhanced, Tecnologia::DlssFg).o_jogo_oferece, Some(true));
        assert_eq!(opcao(&placa_sem_jogo(), Tecnologia::DlssFg).o_jogo_oferece, None);
    }

    fn placa_sem_jogo() -> Vec<OpcaoDeGeracao> {
        catalogo(&classificar_placa("RTX 4070", None), None, None)
    }

    #[test]
    fn ritmo_constante_e_totalmente_consistente() {
        let r = ritmo(&estavel(16.67, 1000));
        assert_eq!(r.consistencia, 100.0);
        assert_eq!(r.picos_por_minuto, 0.0);
        assert_eq!(r.p99_ms, 16.67);
        assert!(r.confiavel);
    }

    #[test]
    fn percentis_pegam_a_cauda() {
        let mut v = estavel(10.0, 990);
        v.extend(estavel(40.0, 10));
        let r = ritmo(&v);
        assert_eq!(r.p95_ms, 10.0);
        assert_eq!(r.p99_ms, 10.0);
        assert!(r.picos_por_minuto > 0.0);
        let mut w = estavel(10.0, 980);
        w.extend(estavel(40.0, 20));
        assert_eq!(ritmo(&w).p99_ms, 40.0);
    }

    #[test]
    fn alternar_quadros_derruba_a_consistencia_mesmo_com_a_mesma_media() {
        let alternado: Vec<f64> = (0..1000).map(|i| if i % 2 == 0 { 8.0 } else { 25.0 }).collect();
        let r = ritmo(&alternado);
        assert!(r.consistencia < 30.0, "consistência {}", r.consistencia);
    }

    #[test]
    fn pouca_amostra_nao_e_confiavel() {
        assert!(!ritmo(&estavel(16.0, 100)).confiavel);
        assert_eq!(ritmo(&[]).amostras, 0);
    }

    #[test]
    fn faixas_de_prontidao_pelo_fps_real() {
        let r = ritmo(&estavel(20.0, 1000));
        assert_eq!(prontidao(25.0, 144, &r, None).nivel, Prontidao::NaoRecomendada);
        assert_eq!(prontidao(40.0, 144, &r, None).nivel, Prontidao::Marginal);
        assert_eq!(prontidao(50.0, 144, &r, None).nivel, Prontidao::Boa);
        assert_eq!(prontidao(90.0, 144, &r, None).nivel, Prontidao::Excelente);
    }

    #[test]
    fn quem_ja_esta_no_limite_do_monitor_nao_precisa() {
        let r = ritmo(&estavel(6.0, 1000));
        let leitura = prontidao(175.0, 180, &r, None);
        assert_eq!(leitura.nivel, Prontidao::Desnecessaria);
    }

    #[test]
    fn ritmo_irregular_rebaixa_um_nivel() {
        let irregular: Vec<f64> = (0..1000).map(|i| if i % 2 == 0 { 8.0 } else { 25.0 }).collect();
        let r = ritmo(&irregular);
        let leitura = prontidao(90.0, 144, &r, None);
        assert_eq!(leitura.nivel, Prontidao::Boa);
        assert!(leitura.motivos.contains(&MotivoDaProntidao::RitmoIrregular));
        assert_eq!(prontidao(40.0, 144, &r, None).nivel, Prontidao::NaoRecomendada);
        assert_eq!(prontidao(20.0, 144, &r, None).nivel, Prontidao::NaoRecomendada);
    }

    #[test]
    fn processador_no_limite_aparece_como_motivo() {
        let r = ritmo(&estavel(16.0, 1000));
        let leitura = prontidao(60.0, 144, &r, Some(Limite::CpuUmNucleo));
        assert!(leitura.motivos.contains(&MotivoDaProntidao::LimitadoPeloProcessador));
        assert!(!prontidao(60.0, 144, &r, Some(Limite::Gpu)).motivos.contains(&MotivoDaProntidao::LimitadoPeloProcessador));
    }

    #[test]
    fn desligado_mede_os_dois_e_nao_acrescenta_atraso() {
        let r = rodada_off(60.0);
        assert!(matches!(r.fps_renderizado, Valor::Medido { .. }));
        assert!(matches!(r.fps_exibido, Valor::Medido { .. }));
        assert_eq!(r.latencia.acrescimo_da_geracao_ms.numero(), Some(0.0));
    }

    #[test]
    fn geracao_no_driver_estima_o_exibido() {
        let r = montar_rodada(Some(Tecnologia::SmoothMotion), 2, &contagem(60.0, 1000), None, 20.0);
        assert!(matches!(r.fps_renderizado, Valor::Medido { valor } if valor == 60.0));
        assert!(matches!(
            r.fps_exibido,
            Valor::Estimado { valor, base: BaseDaEstimativa::MedidoVezesMultiplicador } if valor == 120.0
        ));
    }

    #[test]
    fn geracao_nativa_estima_o_renderizado() {
        let r = montar_rodada(Some(Tecnologia::DlssFg), 2, &contagem(120.0, 1000), None, 20.0);
        assert!(matches!(r.fps_exibido, Valor::Medido { valor } if valor == 120.0));
        assert!(matches!(
            r.fps_renderizado,
            Valor::Estimado { valor, base: BaseDaEstimativa::MedidoDivididoPeloMultiplicador } if valor == 60.0
        ));
    }

    #[test]
    fn geracao_externa_mede_os_dois_processos() {
        let r = montar_rodada(
            Some(Tecnologia::LosslessScaling),
            3,
            &contagem(55.0, 1000),
            Some(&contagem(160.0, 3000)),
            20.0,
        );
        assert!(matches!(r.fps_renderizado, Valor::Medido { valor } if valor == 55.0));
        assert!(matches!(r.fps_exibido, Valor::Medido { valor } if valor == 160.0));
        assert!(r.ritmo_exibido.is_some());
    }

    #[test]
    fn gerador_externo_parado_nao_vira_zero() {
        let r = montar_rodada(Some(Tecnologia::LosslessScaling), 2, &contagem(55.0, 1000), None, 20.0);
        assert!(matches!(r.fps_exibido, Valor::Desconhecido { motivo: MotivoDesconhecido::GeradorSemQuadros }));
    }

    #[test]
    fn geracao_nunca_reduz_o_atraso() {
        for tecnologia in [Tecnologia::DlssFg, Tecnologia::FsrFg, Tecnologia::SmoothMotion, Tecnologia::Afmf, Tecnologia::LosslessScaling, Tecnologia::Otimiza] {
            for mult in 2..=4 {
                let r = montar_rodada(Some(tecnologia), mult, &contagem(60.0, 1000), Some(&contagem(180.0, 3000)), 20.0);
                let acrescimo = r.latencia.acrescimo_da_geracao_ms.numero().unwrap_or(0.0);
                assert!(acrescimo >= 0.0);
                assert!(matches!(r.latencia.jogo_ms, Valor::Desconhecido { .. }));
                assert!(matches!(r.latencia.tela_ms, Valor::Desconhecido { .. }));
            }
        }
    }

    #[test]
    fn multiplicador_fora_da_faixa_e_contido() {
        let r = montar_rodada(Some(Tecnologia::SmoothMotion), 9, &contagem(60.0, 1000), None, 20.0);
        assert_eq!(r.multiplicador, 4);
        assert_eq!(rodada_off(60.0).multiplicador, 1);
    }

    #[test]
    fn no_competitivo_quadro_exibido_nao_vale_ponto() {
        let off = rodada_off(60.0);
        let dobrado = montar_rodada(Some(Tecnologia::SmoothMotion), 2, &contagem(60.0, 1000), None, 20.0);
        let a = pontuar(&off, Perfil::Competitivo, 144);
        let b = pontuar(&dobrado, Perfil::Competitivo, 144);
        assert!(b.total < a.total, "competitivo: {} deveria ser menor que {}", b.total, a.total);
        assert!(pontuar(&dobrado, Perfil::MaximaFluidez, 144).total > pontuar(&off, Perfil::MaximaFluidez, 144).total);
    }

    #[test]
    fn competitivo_desliga_quando_o_atraso_sobe() {
        let off = rodada_off(60.0);
        let ligado = montar_rodada(Some(Tecnologia::SmoothMotion), 2, &contagem(60.0, 1000), None, 20.0);
        let c = comparar(&off, &ligado, Perfil::Competitivo, 144, None, None);
        assert_eq!(c.decisao, Decisao::Desligar);
        assert!(c.motivos.contains(&MotivoDaDecisao::AtrasoAcrescentadoAlto));
    }

    #[test]
    fn maxima_fluidez_mantem_quando_dobra_sem_perder_quadro_real() {
        let off = rodada_off(60.0);
        let ligado = montar_rodada(Some(Tecnologia::SmoothMotion), 2, &contagem(59.0, 1000), None, 20.0);
        let c = comparar(&off, &ligado, Perfil::MaximaFluidez, 144, None, None);
        assert_eq!(c.decisao, Decisao::Manter);
        assert!(c.exibido_pct.unwrap() > 90.0);
    }

    #[test]
    fn perda_grande_de_quadro_real_desliga_em_qualquer_perfil() {
        let off = rodada_off(60.0);
        let ligado = montar_rodada(Some(Tecnologia::LosslessScaling), 2, &contagem(40.0, 1000), Some(&contagem(80.0, 2000)), 20.0);
        for perfil in [Perfil::Competitivo, Perfil::Equilibrado, Perfil::MaximaFluidez] {
            let c = comparar(&off, &ligado, perfil, 144, None, None);
            assert_eq!(c.decisao, Decisao::Desligar, "{:?}", perfil);
            assert!(c.motivos.contains(&MotivoDaDecisao::RenderizadoCaiu));
        }
    }

    #[test]
    fn artefato_incomodo_desliga() {
        let off = rodada_off(60.0);
        let ligado = montar_rodada(Some(Tecnologia::SmoothMotion), 2, &contagem(60.0, 1000), None, 20.0);
        let c = comparar(&off, &ligado, Perfil::MaximaFluidez, 144, Some(NivelDeArtefato::Incomodo), None);
        assert_eq!(c.decisao, Decisao::Desligar);
    }

    #[test]
    fn amostra_curta_e_inconclusiva() {
        let off = montar_rodada(None, 1, &contagem(60.0, 50), None, 1.0);
        let ligado = montar_rodada(Some(Tecnologia::SmoothMotion), 2, &contagem(60.0, 50), None, 1.0);
        let c = comparar(&off, &ligado, Perfil::Equilibrado, 144, None, None);
        assert_eq!(c.decisao, Decisao::Inconclusivo);
        assert_eq!(c.motivos, vec![MotivoDaDecisao::AmostraCurta]);
    }

    #[test]
    fn exibido_acima_do_monitor_e_avisado() {
        let off = rodada_off(100.0);
        let ligado = montar_rodada(Some(Tecnologia::DlssFg), 2, &contagem(200.0, 2000), None, 20.0);
        let c = comparar(&off, &ligado, Perfil::MaximaFluidez, 144, None, None);
        assert!(c.motivos.contains(&MotivoDaDecisao::PassouDoMonitor));
    }

    #[test]
    fn cena_que_nao_se_repete_derruba_a_confianca() {
        let off = rodada_off(60.0);
        let ligado = montar_rodada(Some(Tecnologia::SmoothMotion), 2, &contagem(59.0, 1000), None, 20.0);
        let estavel = comparar(&off, &ligado, Perfil::MaximaFluidez, 144, None, Some(1.0));
        let instavel = comparar(&off, &ligado, Perfil::MaximaFluidez, 144, None, Some(-18.0));
        assert!(instavel.motivos.contains(&MotivoDaDecisao::CenaNaoRepetivel));
        assert!(instavel.confianca_pct + 30.0 <= estavel.confianca_pct);
        assert_eq!(deriva_da_cena(&rodada_off(60.0), &rodada_off(51.0)), Some(-15.0));
    }

    #[test]
    fn multiplicador_real_do_gerador_externo_e_conferido() {
        let certo = montar_rodada(Some(Tecnologia::LosslessScaling), 2, &contagem(60.0, 1000), Some(&contagem(119.0, 2000)), 20.0);
        let errado = montar_rodada(Some(Tecnologia::LosslessScaling), 3, &contagem(60.0, 1000), Some(&contagem(120.0, 2000)), 20.0);
        assert_eq!(multiplicador_confere(&certo), Some(true));
        assert_eq!(multiplicador_confere(&errado), Some(false));
        let driver = montar_rodada(Some(Tecnologia::SmoothMotion), 2, &contagem(60.0, 1000), None, 20.0);
        assert_eq!(multiplicador_confere(&driver), None);
        let off = rodada_off(60.0);
        assert!(comparar(&off, &errado, Perfil::MaximaFluidez, 144, None, None).motivos.contains(&MotivoDaDecisao::MultiplicadorNaoConfere));
    }

    #[test]
    fn low_01_pega_o_engasgo_raro() {
        let mut v = estavel(10.0, 2000);
        v[500] = 200.0;
        v[1500] = 200.0;
        let r = ritmo(&v);
        assert_eq!(r.low_01pct_fps, 5.0);
        assert!(r.low_1pct_fps > r.low_01pct_fps);
    }

    #[test]
    fn modo_inteligente_escolhe_a_melhor_ou_nenhuma() {
        let off = rodada_off(60.0);
        let x2 = montar_rodada(Some(Tecnologia::LosslessScaling), 2, &contagem(58.0, 1000), Some(&contagem(116.0, 2000)), 20.0);
        let x3 = montar_rodada(Some(Tecnologia::LosslessScaling), 3, &contagem(57.0, 1000), Some(&contagem(140.0, 2800)), 20.0);
        let ruim = montar_rodada(Some(Tecnologia::LosslessScaling), 4, &contagem(35.0, 1000), Some(&contagem(140.0, 2800)), 20.0);
        let testadas = vec![x2, x3, ruim];
        assert_eq!(melhor_rodada(&off, &testadas, Perfil::MaximaFluidez, 144), Some(1));
        assert_eq!(melhor_rodada(&off, &testadas, Perfil::Competitivo, 144), None);
    }

    #[test]
    fn artefatos_forte_e_incomodo_nenhum_e_limpo() {
        assert_eq!(avaliar_artefatos(&[(Artefato::Fantasmas, Intensidade::Nenhuma)]).nivel, NivelDeArtefato::Limpo);
        assert_eq!(avaliar_artefatos(&[(Artefato::Fantasmas, Intensidade::Leve)]).nivel, NivelDeArtefato::Aceitavel);
        assert_eq!(avaliar_artefatos(&[(Artefato::Borrado, Intensidade::Forte)]).nivel, NivelDeArtefato::Incomodo);
        assert!(
            avaliar_artefatos(&[(Artefato::InterfaceTremida, Intensidade::Leve)]).nota
                < avaliar_artefatos(&[(Artefato::Borrado, Intensidade::Leve)]).nota
        );
    }

    #[test]
    fn limite_sugerido_fica_abaixo_do_monitor_depois_de_multiplicar() {
        assert_eq!(limite_sugerido(144, 1), Some(139));
        assert_eq!(limite_sugerido(144, 2), Some(69));
        assert_eq!(limite_sugerido(180, 2), Some(87));
        assert_eq!(limite_sugerido(60, 4), None);
        assert_eq!(limite_sugerido(0, 2), None);
        for hz in [60, 75, 120, 144, 165, 180, 240, 360] {
            for m in 1..=4u8 {
                if let Some(l) = limite_sugerido(hz, m) {
                    assert!(l * m as u32 <= hz - 3, "{hz} Hz × {m}");
                }
            }
        }
    }

    #[test]
    fn casamento_com_a_tela() {
        assert_eq!(ajuste_de_tela(Some(200.0), 144, 2).casamento, Casamento::AcimaDoMonitor);
        assert_eq!(ajuste_de_tela(Some(140.0), 144, 2).casamento, Casamento::NoLimite);
        assert_eq!(ajuste_de_tela(Some(90.0), 144, 2).casamento, Casamento::AbaixoDoMonitor);
        assert_eq!(ajuste_de_tela(None, 144, 2).casamento, Casamento::MonitorDesconhecido);
    }

    fn fonte_sem_testes() -> String {
        let fonte = include_str!("framegen.rs");
        fonte.split("#[cfg(test)]").next().unwrap_or_default().to_string()
    }

    #[test]
    fn o_laboratorio_nao_escreve_no_driver_nem_no_jogo() {
        // Ligar geração é da pessoa: se alguém chamar escrita daqui, a trava reprova.
        let fonte = fonte_sem_testes();
        for proibido in [
            "nvdriver::aplicar",
            "WriteProcessMemory",
            "CreateRemoteThread",
            "OpenProcess",
            "configjogo::aplicar",
            "registry::set",
        ] {
            assert!(!fonte.contains(proibido), "framegen.rs não pode usar `{}`", proibido);
        }
    }
}
