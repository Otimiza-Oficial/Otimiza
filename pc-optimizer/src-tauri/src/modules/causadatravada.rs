// Causa da travada, motor V2 (docs/planos/PIP-ENGENHARIA.md, seção 8). A V1 (`core::travadas`) contava em quantas
// travadas um sinal estava ligado: disco ocupado em 9 de 14 travadas parecia forte mesmo com o disco ocupado 60% do
// tempo. Aqui cada hipótese é comparada com a TAXA DE BASE do próprio sinal na captura: sob a hipótese nula
// (travada independente do sinal), as travadas caem em amostras com o sinal ligado na mesma proporção do tempo em que
// ele fica ligado. Valor-p exato da binomial, corrigido pelo número de hipóteses. Evidência contra rebaixa; com poucas
// travadas, o motor se abstém. Nada aqui afirma CAUSA: o máximo sem intervir é associação, e quem promove a causa é o
// teste ativo (`testedatravada`).
//
// Pura: recebe quadros e amostras no mesmo relógio (QPC) e devolve a investigação. Sem Windows, sem arquivo.

use serde::{Deserialize, Serialize};

use crate::core::estatistica::mediana;
use crate::core::telemetria::Amostra;

/// Um quadro DO JOGO na cadeia principal, pelo PresentMon. Serializável: uma captura gravada roda de novo no motor
/// (replay offline), sem o jogo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuadroDoJogo {
    pub qpc: i64,
    pub intervalo_ms: f64,
    /// `CPUBusy` e `GPUBusy` do PresentMon: dizem de que lado o quadro atrasou.
    pub cpu_ms: Option<f64>,
    pub gpu_ms: Option<f64>,
}

/// Quadros em volta (centrado) para a mediana local: trecho pesado do mapa é FPS baixo, não travada.
pub const JANELA_LOCAL: usize = 61;
pub const FATOR_DE_TRAVADA: f64 = 2.0;
/// A 180 FPS, dobrar de 5,5 para 11 ms não se sente; a 60 FPS, de 16,7 para 33 ms sim.
pub const EXCESSO_MINIMO_MS: f64 = 8.0;
/// Quadros ruins colados são um episódio só: a unidade do teste precisa ser independente.
pub const EPISODIO_JUNTA_MS: f64 = 250.0;
/// Abaixo disso o motor não aponta nada.
pub const MINIMO_DE_EPISODIOS: usize = 5;
pub const QUADROS_MINIMOS: usize = 1000;
/// Lado do quadro: o tempo de CPU (ou de GPU) do pior quadro passou disso vezes a mediana local.
const LADO_FATOR: f64 = 1.5;

// Os limiares de sinal são os mesmos da V1, para a comparação entre as duas medir só o método.
const SALTO: f64 = 3.0;
const DISCO_MIN_MS: f64 = 20.0;
const PAGINAS_MIN: f64 = 200.0;
const VRAM_CHEIA: f64 = 0.95;
const VRAM_TRANSBORDO_MB: f64 = 100.0;
const NUCLEO_TETO: f64 = 95.0;
const PROCESSO_MIN: f64 = 0.08;
/// Máquina praticamente cheia: acima disso, qualquer coisa a mais tira tempo do jogo.
const PROCESSADOR_CHEIO_PCT: f64 = 90.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severidade {
    Leve,
    Moderada,
    Severa,
    Extrema,
}

fn severidade(excesso_ms: f64) -> Severidade {
    if excesso_ms > 100.0 {
        Severidade::Extrema
    } else if excesso_ms > 33.0 {
        Severidade::Severa
    } else if excesso_ms > 16.0 {
        Severidade::Moderada
    } else {
        Severidade::Leve
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lado {
    Processador,
    Placa,
    Indefinido,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Episodio {
    /// O pior quadro do episódio (`CPUStartQPC` dele).
    pub qpc: i64,
    /// O trecho em que o atraso aconteceu: do começo do primeiro quadro ruim ao fim do último. O `CPUStartQPC` do
    /// PresentMon marca o COMEÇO do quadro; o atraso está dentro dele, e pode cruzar para a amostra seguinte.
    pub inicio_qpc: i64,
    pub fim_qpc: i64,
    pub pior_ms: f64,
    pub excesso_ms: f64,
    pub severidade: Severidade,
    pub quadros: usize,
    pub lado: Lado,
}

/// Mediana móvel centrada, com a janela cortada nas pontas. Buffer ordenado: O(n·janela), sem ordenar a cada quadro.
pub fn medianas_locais(v: &[f64], janela: usize) -> Vec<f64> {
    let meia = janela / 2;
    let mut ordenado: Vec<f64> = Vec::with_capacity(janela + 1);
    let inserir = |o: &mut Vec<f64>, x: f64| {
        let i = o.partition_point(|y| y.total_cmp(&x).is_lt());
        o.insert(i, x);
    };
    for &x in v.iter().take(meia + 1) {
        inserir(&mut ordenado, x);
    }
    let mut saida = Vec::with_capacity(v.len());
    for i in 0..v.len() {
        if i > 0 {
            if let Some(&novo) = v.get(i + meia) {
                inserir(&mut ordenado, novo);
            }
            if i > meia {
                let velho = v[i - meia - 1];
                let j = ordenado.partition_point(|y| y.total_cmp(&velho).is_lt());
                ordenado.remove(j);
            }
        }
        let n = ordenado.len();
        saida.push(if n % 2 == 1 { ordenado[n / 2] } else { (ordenado[n / 2 - 1] + ordenado[n / 2]) / 2.0 });
    }
    saida
}

/// Quadro ruim: ≥ 2× a mediana local E ≥ mediana local + 8 ms. Colados a menos de 250 ms viram um episódio.
pub fn detectar_episodios(quadros: &[QuadroDoJogo], frequencia: i64) -> Vec<Episodio> {
    if quadros.is_empty() || frequencia <= 0 {
        return Vec::new();
    }
    let intervalos: Vec<f64> = quadros.iter().map(|q| q.intervalo_ms).collect();
    let locais = medianas_locais(&intervalos, JANELA_LOCAL);
    let com_lado = quadros.iter().all(|q| q.cpu_ms.is_some() && q.gpu_ms.is_some());
    let (cpu_locais, gpu_locais) = if com_lado {
        let c: Vec<f64> = quadros.iter().map(|q| q.cpu_ms.unwrap_or(0.0)).collect();
        let g: Vec<f64> = quadros.iter().map(|q| q.gpu_ms.unwrap_or(0.0)).collect();
        (medianas_locais(&c, JANELA_LOCAL), medianas_locais(&g, JANELA_LOCAL))
    } else {
        (Vec::new(), Vec::new())
    };
    let junta = (EPISODIO_JUNTA_MS / 1000.0 * frequencia as f64) as i64;
    let fim_de = |q: &QuadroDoJogo| q.qpc + (q.intervalo_ms / 1000.0 * frequencia as f64) as i64;

    let mut episodios: Vec<Episodio> = Vec::new();
    let mut ultimo_qpc: Option<i64> = None;
    for (i, q) in quadros.iter().enumerate() {
        let local = locais[i];
        let excesso = q.intervalo_ms - local;
        if !(q.intervalo_ms >= FATOR_DE_TRAVADA * local && excesso >= EXCESSO_MINIMO_MS) {
            continue;
        }
        let lado = if com_lado {
            let cpu = q.cpu_ms.unwrap_or(0.0) / cpu_locais[i].max(0.01);
            let gpu = q.gpu_ms.unwrap_or(0.0) / gpu_locais[i].max(0.01);
            match (cpu >= LADO_FATOR, gpu >= LADO_FATOR) {
                (true, false) => Lado::Processador,
                (false, true) => Lado::Placa,
                _ => Lado::Indefinido,
            }
        } else {
            Lado::Indefinido
        };
        let colado = ultimo_qpc.is_some_and(|u| q.qpc - u <= junta);
        ultimo_qpc = Some(q.qpc);
        if colado {
            let e = episodios.last_mut().expect("colado implica um episódio aberto");
            e.quadros += 1;
            e.fim_qpc = e.fim_qpc.max(fim_de(q));
            if excesso > e.excesso_ms {
                e.qpc = q.qpc;
                e.pior_ms = q.intervalo_ms;
                e.excesso_ms = excesso;
                e.severidade = severidade(excesso);
                e.lado = lado;
            }
        } else {
            episodios.push(Episodio {
                qpc: q.qpc,
                inicio_qpc: q.qpc,
                fim_qpc: fim_de(q),
                pior_ms: q.intervalo_ms,
                excesso_ms: excesso,
                severidade: severidade(excesso),
                quadros: 1,
                lado,
            });
        }
    }
    episodios
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Hipotese {
    Processo { nome: String },
    Disco,
    Paginacao,
    Vram,
    NucleoSaturado,
    /// A máquina inteira ocupada (≥ 90%): o jogo disputa com tudo, não com um programa só. Achado no laboratório de
    /// 28/09/2026: as travadas acompanhavam a carga total, e nenhum processo sozinho.
    ProcessadorCheio,
}

impl Hipotese {
    /// "com {sinal}".
    pub fn sinal(&self) -> String {
        match self {
            Hipotese::Processo { nome } => format!("o {} usando o processador", nome),
            Hipotese::Disco => "o disco respondendo devagar".to_string(),
            Hipotese::Paginacao => "o Windows lendo memória do disco (falta de RAM)".to_string(),
            Hipotese::Vram => "a memória de vídeo cheia".to_string(),
            Hipotese::NucleoSaturado => "um núcleo do processador no limite".to_string(),
            Hipotese::ProcessadorCheio => "o processador inteiro ocupado".to_string(),
        }
    }

    /// O atraso que esta hipótese causa aparece no tempo de processador do quadro.
    fn do_lado_do_processador(&self) -> bool {
        matches!(self, Hipotese::Processo { .. } | Hipotese::NucleoSaturado | Hipotese::ProcessadorCheio)
    }
}

/// Ordem crescente de força.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Confianca {
    /// O contador não respondeu: nunca vira "ausente".
    NaoMedido,
    /// Poucas travadas para qualquer conclusão.
    Inconclusiva,
    SemEvidencia,
    Moderada,
    Forte,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Teste {
    /// Acalmar o processo (prioridade abaixo do normal + EcoQoS) em janelas alternadas: o teste ativo do slice.
    AcalmarProcesso { processo: String },
    Recomendacao { texto: String },
    Nenhum,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Avaliacao {
    pub hipotese: Hipotese,
    pub confianca: Confianca,
    /// k: episódios com o sinal ligado.
    pub episodios_com_sinal: usize,
    /// n: episódios dentro da cobertura das amostras.
    pub episodios_cobertos: usize,
    /// p0: fração das amostras com o sinal ligado.
    pub taxa_de_base: f64,
    pub lift: Option<f64>,
    pub p_corrigido: Option<f64>,
    pub a_favor: Vec<String>,
    pub contra: Vec<String>,
    pub teste: Teste,
    /// Resultado de um teste ativo anterior desta hipótese neste jogo (memória, inclusive negativa).
    pub ja_testado: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Severidades {
    pub leve: usize,
    pub moderada: usize,
    pub severa: usize,
    pub extrema: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Investigacao {
    pub segundos: f64,
    pub quadros: usize,
    pub fps_mediano: Option<f64>,
    pub episodios: Vec<Episodio>,
    pub por_minuto: f64,
    pub severidades: Severidades,
    /// Com ≥ 1.000 quadros.
    pub p99_ms: Option<f64>,
    /// Com ≥ 10.000 quadros.
    pub p999_ms: Option<f64>,
    pub lado_processador: usize,
    pub lado_placa: usize,
    /// Da mais forte para a mais fraca; `NaoMedido` no fim.
    pub avaliacoes: Vec<Avaliacao>,
    /// Episódios cobertos sem nenhuma hipótese Moderada ou Forte ligada.
    pub sem_causa: usize,
    /// `Some`: o motor não aponta nada, e diz por quê.
    pub abstencao: Option<String>,
    pub conclusao: String,
}

/// ln(k!) para k em 0..=n.
fn ln_fatoriais(n: usize) -> Vec<f64> {
    let mut v = Vec::with_capacity(n + 1);
    let mut acc = 0.0;
    v.push(0.0);
    for i in 1..=n {
        acc += (i as f64).ln();
        v.push(acc);
    }
    v
}

/// P(X ≥ k) para X ~ Binomial(n, p), exata (soma em escala log).
pub fn cauda_binomial(n: usize, k: usize, p: f64) -> f64 {
    if k == 0 {
        return 1.0;
    }
    if k > n || p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let lf = ln_fatoriais(n);
    let (lp, lq) = (p.ln(), (1.0 - p).ln());
    (k..=n)
        .map(|i| (lf[n] - lf[i] - lf[n - i] + i as f64 * lp + (n - i) as f64 * lq).exp())
        .sum::<f64>()
        .min(1.0)
}

fn percentil(ordenados: &[f64], p: f64) -> f64 {
    let i = ((p / 100.0) * ordenados.len() as f64).ceil() as usize;
    ordenados[i.clamp(1, ordenados.len()) - 1]
}

fn salto(valor: Option<f64>, normal: Option<f64>, minimo: f64) -> Option<bool> {
    let v = valor?;
    Some(match normal {
        Some(n) => v >= minimo && v >= n.max(f64::EPSILON) * SALTO,
        None => v >= minimo,
    })
}

/// Sinal de cada hipótese em cada amostra. `None`: não medido naquela amostra.
fn sinais(amostras: &[(i64, Amostra)], vram_total_mb: Option<f64>) -> Vec<(Hipotese, Vec<Option<bool>>)> {
    let med = |f: &dyn Fn(&Amostra) -> Option<f64>| mediana(&amostras.iter().filter_map(|(_, a)| f(a)).collect::<Vec<_>>());
    let disco_n = med(&|a| a.disco_latencia_ms);
    let pag_n = med(&|a| a.paginas_lidas_s);
    let comp_n = med(&|a| a.vram_compartilhada_mb);
    let nucleo_n = med(&|a| a.cpu_nucleo_max_pct);

    let mut v: Vec<(Hipotese, Vec<Option<bool>>)> = vec![
        (Hipotese::Disco, amostras.iter().map(|(_, a)| salto(a.disco_latencia_ms, disco_n, DISCO_MIN_MS)).collect()),
        (Hipotese::Paginacao, amostras.iter().map(|(_, a)| salto(a.paginas_lidas_s, pag_n, PAGINAS_MIN)).collect()),
        (
            Hipotese::Vram,
            amostras
                .iter()
                .map(|(_, a)| {
                    let cheia = match (a.vram_usada_mb, vram_total_mb) {
                        (Some(u), Some(t)) if t > 0.0 => Some(u / t >= VRAM_CHEIA),
                        _ => None,
                    };
                    let transbordo = match (a.vram_compartilhada_mb, comp_n) {
                        (Some(c), Some(n)) => Some(c - n >= VRAM_TRANSBORDO_MB),
                        _ => None,
                    };
                    match (cheia, transbordo) {
                        (None, None) => None,
                        (c, t) => Some(c.unwrap_or(false) || t.unwrap_or(false)),
                    }
                })
                .collect(),
        ),
        (
            Hipotese::NucleoSaturado,
            amostras
                .iter()
                .map(|(_, a)| match (a.cpu_nucleo_max_pct, nucleo_n) {
                    (Some(n), Some(normal)) => Some(n >= NUCLEO_TETO && normal < NUCLEO_TETO - 10.0),
                    _ => None,
                })
                .collect(),
        ),
        (
            Hipotese::ProcessadorCheio,
            amostras.iter().map(|(_, a)| a.cpu_total_pct.map(|c| c >= PROCESSADOR_CHEIO_PCT)).collect(),
        ),
    ];

    // Processos: ausente numa amostra = 0 naquela amostra (a coleta guarda só os maiores).
    let acompanhou = amostras.iter().any(|(_, a)| !a.processos.is_empty());
    if acompanhou {
        let mut nomes: Vec<String> = amostras.iter().flat_map(|(_, a)| a.processos.iter().map(|p| p.nome.clone())).collect();
        nomes.sort();
        nomes.dedup();
        for nome in nomes {
            let serie: Vec<f64> = amostras
                .iter()
                .map(|(_, a)| a.processos.iter().find(|p| p.nome == nome).map(|p| p.cpu).unwrap_or(0.0))
                .collect();
            let normal = mediana(&serie).unwrap_or(0.0).max(0.005);
            let ativo: Vec<Option<bool>> = serie.iter().map(|c| Some(*c >= PROCESSO_MIN && *c >= normal * SALTO)).collect();
            if ativo.iter().any(|a| *a == Some(true)) {
                v.push((Hipotese::Processo { nome }, ativo));
            }
        }
    }
    v
}

/// As amostras cujo intervalo `(c[j-1], c[j]]` toca o trecho `[inicio, fim]` (o contador de taxa mede o intervalo
/// entre duas leituras). `None`: o trecho sai da cobertura das amostras.
fn amostras_que_tocam(carimbos: &[i64], inicio: i64, fim: i64) -> Option<std::ops::RangeInclusive<usize>> {
    let (Some(primeiro), Some(ultimo)) = (carimbos.first(), carimbos.last()) else { return None };
    if carimbos.len() < 2 || inicio < *primeiro || fim > *ultimo {
        return None;
    }
    let a = carimbos.partition_point(|c| *c < inicio).max(1);
    let b = carimbos.partition_point(|c| *c < fim).max(a).min(carimbos.len() - 1);
    Some(a..=b)
}

/// Sob a hipótese nula (a travada cai em qualquer lugar, sem relação com o sinal): a chance de um trecho de duração
/// `d` tocar algum intervalo com o sinal ligado. Mede o conjunto de começos possíveis que tocam, sobre o total.
fn chance_de_tocar_ao_acaso(carimbos: &[i64], ligado: &[bool], d: i64) -> f64 {
    let (c0, cm) = (carimbos[0], *carimbos.last().expect("não vazio"));
    let fim_do_espaco = cm - d;
    if fim_do_espaco <= c0 {
        return 1.0;
    }
    // Trechos ligados contínuos (a, b]; um começo t toca se t ∈ (a - d, b).
    let mut faixas: Vec<(i64, i64)> = Vec::new();
    for j in 1..carimbos.len() {
        if !ligado[j] {
            continue;
        }
        let (a, b) = ((carimbos[j - 1] - d).max(c0), carimbos[j].min(fim_do_espaco));
        if b <= a {
            continue;
        }
        match faixas.last_mut() {
            Some(u) if a <= u.1 => u.1 = u.1.max(b),
            _ => faixas.push((a, b)),
        }
    }
    let tocado: i64 = faixas.iter().map(|(a, b)| b - a).sum();
    (tocado as f64 / (fim_do_espaco - c0) as f64).clamp(0.0, 1.0)
}

/// P(K ≥ k) com K = soma de Bernoullis independentes de chances `p` (Poisson-binomial), exata por programação dinâmica.
pub fn cauda_poisson_binomial(p: &[f64], k: usize) -> f64 {
    if k == 0 {
        return 1.0;
    }
    if k > p.len() {
        return 0.0;
    }
    let mut dist = vec![0.0f64; p.len() + 1];
    dist[0] = 1.0;
    for (i, &pi) in p.iter().enumerate() {
        for j in (0..=i + 1).rev() {
            let de_cima = if j > 0 { dist[j - 1] * pi } else { 0.0 };
            dist[j] = dist[j] * (1.0 - pi) + de_cima;
        }
    }
    dist[k..].iter().sum::<f64>().clamp(0.0, 1.0)
}

fn recomendacao(h: &Hipotese, protegido: &dyn Fn(&str) -> bool) -> Teste {
    match h {
        Hipotese::Processo { nome } if protegido(nome) => Teste::Recomendacao {
            texto: format!(
                "O {} é um programa que o Otimiza não mexe (voz, transmissão, sistema ou anticheat). Se der, feche-o \
                 antes de jogar e investigue de novo: se as travadas sumirem, era ele.",
                nome
            ),
        },
        Hipotese::Processo { nome } => Teste::AcalmarProcesso { processo: nome.clone() },
        Hipotese::Disco => Teste::Recomendacao {
            texto: "Travadas junto de disco lento: jogo num HD, antivírus varrendo ou disco cheio. Investigue de novo \
                    com o antivírus pausado; mover o jogo para um SSD é o que resolve."
                .to_string(),
        },
        Hipotese::Paginacao => Teste::Recomendacao {
            texto: "Faltou memória: o Windows buscou dados no disco na hora das travadas. Feche o navegador e outros \
                    programas antes de jogar e investigue de novo."
                .to_string(),
        },
        Hipotese::Vram => Teste::Recomendacao {
            texto: "A memória de vídeo encheu. Baixe a qualidade de textura no jogo e investigue de novo.".to_string(),
        },
        Hipotese::ProcessadorCheio => Teste::Recomendacao {
            texto: "O processador inteiro estava ocupado nas travadas: não é um programa só, é a soma. Feche o que \
                    não precisa (navegador, atualizações, compilações, gravação) e investigue de novo: se as travadas \
                    caírem, era isso."
                .to_string(),
        },
        Hipotese::NucleoSaturado => Teste::Recomendacao {
            texto: "Um núcleo no limite enquanto os outros sobram: é o próprio jogo. Nenhum ajuste de Windows resolve; \
                    menos distância de visão e menos gente em volta aliviam."
                .to_string(),
        },
    }
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

/// **Pura.** `amostras`: `(carimbo QPC, amostra)` em ordem. `protegido`: programas que o Otimiza não mexe.
pub fn investigar(
    quadros: &[QuadroDoJogo],
    amostras: &[(i64, Amostra)],
    frequencia: i64,
    vram_total_mb: Option<f64>,
    protegido: &dyn Fn(&str) -> bool,
) -> Investigacao {
    let segundos = match (quadros.first(), quadros.last()) {
        (Some(a), Some(b)) if frequencia > 0 => (b.qpc - a.qpc) as f64 / frequencia as f64,
        _ => 0.0,
    };
    let mut intervalos: Vec<f64> = quadros.iter().map(|q| q.intervalo_ms).collect();
    intervalos.sort_by(f64::total_cmp);
    let fps_mediano = mediana(&intervalos).filter(|m| *m > 0.0).map(|m| 1000.0 / m);
    let p99_ms = (intervalos.len() >= 1_000).then(|| percentil(&intervalos, 99.0));
    let p999_ms = (intervalos.len() >= 10_000).then(|| percentil(&intervalos, 99.9));

    let episodios = detectar_episodios(quadros, frequencia);
    let mut severidades = Severidades::default();
    for e in &episodios {
        match e.severidade {
            Severidade::Leve => severidades.leve += 1,
            Severidade::Moderada => severidades.moderada += 1,
            Severidade::Severa => severidades.severa += 1,
            Severidade::Extrema => severidades.extrema += 1,
        }
    }
    let lado_processador = episodios.iter().filter(|e| e.lado == Lado::Processador).count();
    let lado_placa = episodios.iter().filter(|e| e.lado == Lado::Placa).count();
    let por_minuto = if segundos > 0.0 { episodios.len() as f64 * 60.0 / segundos } else { 0.0 };

    let carimbos: Vec<i64> = amostras.iter().map(|(q, _)| *q).collect();
    let cobertura: Vec<Option<std::ops::RangeInclusive<usize>>> =
        episodios.iter().map(|e| amostras_que_tocam(&carimbos, e.inicio_qpc, e.fim_qpc)).collect();
    let cobertos = cobertura.iter().filter(|c| c.is_some()).count();

    let abstencao = if quadros.len() < QUADROS_MINIMOS {
        Some(format!("Só {} quadros medidos: pouco para dizer qualquer coisa sobre travadas.", quadros.len()))
    } else if episodios.len() >= MINIMO_DE_EPISODIOS && cobertos < MINIMO_DE_EPISODIOS {
        Some(format!(
            "{} travadas em {:.0} s, mas os contadores do Windows não cobriram esse trecho: não dá para apontar causa \
             sem saber o que a máquina fazia na hora.",
            episodios.len(),
            segundos
        ))
    } else if cobertos < MINIMO_DE_EPISODIOS {
        Some(format!(
            "{} travada(s) em {:.0} s: poucas para apontar uma causa. Isso é bom sinal; se o jogo trava mais em outro \
             lugar, investigue lá.",
            episodios.len(),
            segundos
        ))
    } else {
        None
    };

    let todos = sinais(amostras, vram_total_mb);
    // Bonferroni pelo número de hipóteses medidas: com 10 processos, uma associação "por sorte" apareceria à toa.
    let medidas = todos.iter().filter(|(_, s)| s.iter().skip(1).any(|x| x.is_some())).count().max(1);

    let mut avaliacoes: Vec<Avaliacao> = todos
        .iter()
        .map(|(h, serie)| {
            // Taxa de base: amostras (a partir da segunda, as que têm intervalo) com o sinal ligado.
            let medidos: Vec<bool> = serie.iter().skip(1).filter_map(|x| *x).collect();
            if medidos.is_empty() {
                return Avaliacao {
                    hipotese: h.clone(),
                    confianca: Confianca::NaoMedido,
                    episodios_com_sinal: 0,
                    episodios_cobertos: 0,
                    taxa_de_base: 0.0,
                    lift: None,
                    p_corrigido: None,
                    a_favor: Vec::new(),
                    contra: vec![format!("Não deu para medir {} nesta máquina.", h.sinal())],
                    teste: Teste::Nenhum,
                    ja_testado: None,
                };
            }
            // Fração do tempo com o sinal ligado (para a frase) e, por travada, a chance de tocar o sinal ao acaso.
            let fracao_do_tempo = medidos.iter().filter(|x| **x).count() as f64 / medidos.len() as f64;
            let ligado: Vec<bool> = serie.iter().map(|x| *x == Some(true)).collect();
            let mut n = 0usize;
            let mut k = 0usize;
            let mut chances: Vec<f64> = Vec::new();
            let (mut k_cpu, mut k_gpu) = (0usize, 0usize);
            for (e, c) in episodios.iter().zip(&cobertura) {
                let Some(faixa) = c else { continue };
                if !faixa.clone().any(|j| serie[j].is_some()) {
                    continue;
                }
                n += 1;
                chances.push(chance_de_tocar_ao_acaso(&carimbos, &ligado, e.fim_qpc - e.inicio_qpc));
                if faixa.clone().any(|j| serie[j] == Some(true)) {
                    k += 1;
                    match e.lado {
                        Lado::Processador => k_cpu += 1,
                        Lado::Placa => k_gpu += 1,
                        Lado::Indefinido => {}
                    }
                }
            }
            // p0: a fração de travadas que tocaria o sinal ao acaso (média das chances, que crescem com a duração).
            let p0 = if chances.is_empty() { fracao_do_tempo } else { chances.iter().sum::<f64>() / chances.len() as f64 };
            let p = cauda_poisson_binomial(&chances, k);
            let p_corrigido = (p * medidas as f64).min(1.0);
            let lift = (p0 > 0.0 && n > 0).then(|| (k as f64 / n as f64) / p0);

            let mut a_favor = Vec::new();
            let mut contra = Vec::new();
            if n > 0 {
                a_favor.push(format!(
                    "{} de {} travadas aconteceram com {}; ao acaso seriam umas {:.0} ({} delas: o sinal fica ligado em {} \
                     do tempo).",
                    k,
                    n,
                    h.sinal(),
                    p0 * n as f64,
                    pct(p0),
                    pct(fracao_do_tempo)
                ));
            }
            if p0 > 0.5 {
                contra.push(format!("O sinal fica ligado em {} do tempo: coincidir com travada é esperado.", pct(p0)));
            }
            if h.do_lado_do_processador() && k_gpu > k_cpu {
                contra.push(format!(
                    "{} das travadas com o sinal ligado foram do lado da placa de vídeo, não do processador.",
                    k_gpu
                ));
            }
            if n > 0 && k > 0 && (k as f64) < 0.25 * n as f64 {
                contra.push(format!("Explica no máximo {} de {} travadas.", k, n));
            }

            let base = if abstencao.is_some() {
                Confianca::Inconclusiva
            } else if p_corrigido < 0.01 && lift.is_some_and(|l| l >= 2.0) && k >= 3 {
                Confianca::Forte
            } else if p_corrigido < 0.05 && lift.is_some_and(|l| l >= 1.5) && k >= 3 {
                Confianca::Moderada
            } else {
                Confianca::SemEvidencia
            };
            let confianca = match (base, contra.is_empty()) {
                (Confianca::Forte, false) => Confianca::Moderada,
                (Confianca::Moderada, false) => Confianca::SemEvidencia,
                (c, _) => c,
            };
            let teste = if confianca >= Confianca::Moderada { recomendacao(h, protegido) } else { Teste::Nenhum };
            Avaliacao {
                hipotese: h.clone(),
                confianca,
                episodios_com_sinal: k,
                episodios_cobertos: n,
                taxa_de_base: p0,
                lift,
                p_corrigido: Some(p_corrigido),
                a_favor,
                contra,
                teste,
                ja_testado: None,
            }
        })
        .collect();
    avaliacoes.sort_by(|a, b| {
        b.confianca.cmp(&a.confianca).then(b.episodios_com_sinal.cmp(&a.episodios_com_sinal)).then(a.hipotese.cmp(&b.hipotese))
    });

    // Sem causa: episódio coberto em que nenhuma hipótese Moderada/Forte estava ligada.
    let fortes: Vec<&Vec<Option<bool>>> = todos
        .iter()
        .filter(|(h, _)| avaliacoes.iter().any(|a| &a.hipotese == h && a.confianca >= Confianca::Moderada))
        .map(|(_, s)| s)
        .collect();
    let sem_causa = cobertura
        .iter()
        .flatten()
        .filter(|faixa| !fortes.iter().any(|s| (**faixa).clone().any(|j| s[j] == Some(true))))
        .count();

    let conclusao = if let Some(a) = &abstencao {
        a.clone()
    } else if let Some(melhor) = avaliacoes.first().filter(|a| a.confianca >= Confianca::Moderada) {
        let forca = if melhor.confianca == Confianca::Forte { "forte" } else { "moderada" };
        format!(
            "Associação {} com {}: {} de {} travadas aconteceram com ele ligado, contra {} esperado ao acaso. \
             Associação ainda não é causa: {}",
            forca,
            melhor.hipotese.sinal(),
            melhor.episodios_com_sinal,
            melhor.episodios_cobertos,
            pct(melhor.taxa_de_base),
            match &melhor.teste {
                Teste::AcalmarProcesso { .. } => "o teste ativo mostra se acalmar esse programa diminui as travadas.",
                _ => "a recomendação abaixo é o teste.",
            }
        )
    } else {
        let placa = if lado_placa * 2 > episodios.len() {
            " A maioria das travadas atrasou do lado da placa de vídeo: costuma ser o próprio jogo (compilação de \
             shader, carregamento de textura)."
        } else {
            ""
        };
        format!(
            "Causa ainda não determinada: nenhum sinal medido aparece mais nas travadas do que fora delas.{}",
            placa
        )
    };

    Investigacao {
        segundos,
        quadros: quadros.len(),
        fps_mediano,
        episodios,
        por_minuto,
        severidades,
        p99_ms,
        p999_ms,
        lado_processador,
        lado_placa,
        avaliacoes,
        sem_causa,
        abstencao,
        conclusao,
    }
}

#[cfg(test)]
pub(crate) mod simulacao {
    //! Gerador determinístico de sessões sintéticas com a verdade conhecida, para medir o motor como classificador.
    use super::*;
    use crate::core::telemetria::ProcessoNaAmostra;

    pub const FREQ: i64 = 10_000_000;

    /// xorshift64*: determinístico, sem dependência.
    pub struct Sorteio(pub u64);
    impl Sorteio {
        pub fn u(&mut self) -> f64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            ((self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64) / ((1u64 << 53) as f64)
        }
        pub fn normal(&mut self) -> f64 {
            let (a, b) = (self.u().max(1e-12), self.u());
            (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos()
        }
        pub fn exp(&mut self, taxa: f64) -> f64 {
            -self.u().max(1e-12).ln() / taxa
        }
    }

    /// Quadros de `ms_base` com ruído de 8%, com travadas (quadro de 45–90 ms, lado processador) nos instantes dados (s).
    pub fn quadros(s: &mut Sorteio, segundos: f64, ms_base: f64, travadas_s: &[f64]) -> Vec<QuadroDoJogo> {
        let mut v = Vec::new();
        let mut t = 0.0f64;
        let mut i = 0;
        while t < segundos * 1000.0 {
            let travada = i < travadas_s.len() && t >= travadas_s[i] * 1000.0;
            let ms = if travada {
                i += 1;
                45.0 + 45.0 * s.u()
            } else {
                (ms_base * (1.0 + 0.08 * s.normal())).max(1.0)
            };
            let inicio = t;
            t += ms;
            v.push(QuadroDoJogo {
                qpc: (inicio * FREQ as f64 / 1000.0) as i64,
                intervalo_ms: ms,
                cpu_ms: Some(if travada { ms * 0.9 } else { ms_base * 0.6 }),
                gpu_ms: Some(ms_base * 0.7),
            });
        }
        v
    }

    /// Amostras de 500 ms; `ativo[i]`: o processo "Rajada.exe" disparou naquele intervalo.
    pub fn amostras(ativo: &[bool]) -> Vec<(i64, Amostra)> {
        ativo
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let ms = i as u64 * 500;
                (
                    (ms as i64) * FREQ / 1000,
                    Amostra {
                        instante_ms: ms,
                        cpu_nucleo_max_pct: Some(60.0),
                        disco_latencia_ms: Some(1.0),
                        paginas_lidas_s: Some(5.0),
                        vram_usada_mb: Some(2000.0),
                        vram_compartilhada_mb: Some(50.0),
                        processos: vec![
                            ProcessoNaAmostra { nome: "Rajada.exe".into(), cpu: if *a { 0.25 } else { 0.01 } },
                            ProcessoNaAmostra { nome: "Discord.exe".into(), cpu: 0.02 },
                        ],
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    /// Sessão de `segundos`: rajadas cobrindo `fracao_ativa` do tempo. `causal`: probabilidade de uma rajada gerar
    /// travada; `fundo_por_min`: travadas independentes de tudo.
    pub fn sessao(s: &mut Sorteio, segundos: f64, fracao_ativa: f64, causal: f64, fundo_por_min: f64) -> (Vec<QuadroDoJogo>, Vec<(i64, Amostra)>) {
        let n = (segundos * 2.0) as usize + 1;
        let ativo: Vec<bool> = (0..n).map(|i| i > 0 && s.u() < fracao_ativa).collect();
        let mut travadas: Vec<f64> = Vec::new();
        for (i, a) in ativo.iter().enumerate() {
            if *a && s.u() < causal {
                // Dentro do intervalo (i-1, i] da amostra i.
                travadas.push((i as f64 - 1.0) * 0.5 + 0.05 + 0.4 * s.u());
            }
        }
        let mut t = s.exp(fundo_por_min / 60.0);
        while t < segundos {
            travadas.push(t);
            t += s.exp(fundo_por_min / 60.0);
        }
        travadas.sort_by(f64::total_cmp);
        (quadros(s, segundos, 11.0, &travadas), amostras(&ativo))
    }
}

#[cfg(test)]
mod tests {
    use super::simulacao::*;
    use super::*;

    fn nada_protegido(_: &str) -> bool {
        false
    }

    #[test]
    fn mediana_local_igual_a_ingenua() {
        let mut s = Sorteio(7);
        let v: Vec<f64> = (0..500).map(|_| s.u() * 20.0).collect();
        let rapida = medianas_locais(&v, JANELA_LOCAL);
        let meia = JANELA_LOCAL / 2;
        for i in [0usize, 1, 30, 31, 250, 468, 469, 499] {
            let (a, b) = (i.saturating_sub(meia), (i + meia).min(v.len() - 1));
            let ingenua = mediana(&v[a..=b].to_vec()).unwrap();
            assert!((rapida[i] - ingenua).abs() < 1e-12, "i={} {} vs {}", i, rapida[i], ingenua);
        }
    }

    #[test]
    fn poisson_binomial_com_chances_iguais_e_a_binomial() {
        let p = vec![0.3; 12];
        for k in 0..=12 {
            assert!((cauda_poisson_binomial(&p, k) - cauda_binomial(12, k, 0.3)).abs() < 1e-12, "k={}", k);
        }
        assert!((cauda_poisson_binomial(&[0.5, 0.2], 2) - 0.1).abs() < 1e-12);
    }

    #[test]
    fn travada_longa_tem_mais_chance_de_tocar_o_sinal_ao_acaso() {
        // 10 amostras de 500 ms; sinal ligado em uma só (a quinta).
        let c: Vec<i64> = (0..=10).map(|i| i * 5_000_000).collect();
        let mut l = vec![false; 11];
        l[5] = true;
        let curta = chance_de_tocar_ao_acaso(&c, &l, 10_000);
        let longa = chance_de_tocar_ao_acaso(&c, &l, 3_000_000);
        assert!((curta - 0.1).abs() < 0.01, "{}", curta);
        assert!(longa > 0.15 && longa > curta, "{}", longa);
    }

    #[test]
    fn atraso_que_cruza_para_a_amostra_seguinte_conta() {
        // Quadro começa no fim da amostra 3 (sinal desligado) e o atraso cai na 4 (sinal ligado).
        let c: Vec<i64> = (0..=10).map(|i| i * 5_000_000).collect();
        assert_eq!(amostras_que_tocam(&c, 19_900_000, 22_000_000), Some(4..=5));
        assert_eq!(amostras_que_tocam(&c, 19_000_000, 19_500_000), Some(4..=4));
        assert_eq!(amostras_que_tocam(&c, -1, 10), None, "antes da primeira amostra: fora da cobertura");
    }

    #[test]
    fn cauda_binomial_confere_com_valores_conhecidos() {
        // P(X ≥ 8 | n=10, p=0,5) = 56/1024.
        assert!((cauda_binomial(10, 8, 0.5) - 56.0 / 1024.0).abs() < 1e-12);
        assert_eq!(cauda_binomial(10, 0, 0.3), 1.0);
        assert_eq!(cauda_binomial(10, 11, 0.3), 0.0);
        assert_eq!(cauda_binomial(10, 3, 0.0), 0.0);
    }

    #[test]
    fn trecho_pesado_nao_e_travada_mas_pico_e() {
        // 10 s a 5 ms e 10 s a 20 ms (cena pesada): a mediana global chamaria os 20 ms de travada.
        let mut v = Vec::new();
        let mut t = 0.0;
        for i in 0..2500 {
            let ms = if i < 2000 { 5.0 } else { 20.0 };
            t += ms;
            v.push(QuadroDoJogo { qpc: (t * 10_000.0) as i64, intervalo_ms: ms, cpu_ms: None, gpu_ms: None });
        }
        assert!(detectar_episodios(&v, FREQ).is_empty());
        v[2200].intervalo_ms = 70.0;
        let e = detectar_episodios(&v, FREQ);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].severidade, Severidade::Severa);
        assert_eq!(e[0].lado, Lado::Indefinido, "sem CPUBusy/GPUBusy não se afirma o lado");
    }

    #[test]
    fn quadros_ruins_colados_sao_um_episodio() {
        let mut s = Sorteio(3);
        let mut q = quadros(&mut s, 5.0, 10.0, &[]);
        for i in 200..204 {
            q[i].intervalo_ms = 50.0;
        }
        let e = detectar_episodios(&q, FREQ);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].quadros, 4);
    }

    #[test]
    fn a_180_fps_dobrar_nao_e_travada_perceptivel() {
        let mut s = Sorteio(5);
        let mut q = quadros(&mut s, 10.0, 5.5, &[]);
        q[500].intervalo_ms = 12.0;
        assert!(detectar_episodios(&q, FREQ).is_empty());
    }

    #[test]
    fn causa_plantada_e_apontada() {
        let mut s = Sorteio(11);
        let (q, a) = sessao(&mut s, 120.0, 0.15, 0.6, 1.0);
        let inv = investigar(&q, &a, FREQ, Some(8192.0), &nada_protegido);
        let melhor = &inv.avaliacoes[0];
        assert_eq!(melhor.hipotese, Hipotese::Processo { nome: "Rajada.exe".into() }, "{:#?}", inv.avaliacoes);
        assert!(melhor.confianca >= Confianca::Moderada);
        assert_eq!(melhor.teste, Teste::AcalmarProcesso { processo: "Rajada.exe".into() });
        assert!(inv.conclusao.contains("não é causa"), "{}", inv.conclusao);
    }

    #[test]
    fn programa_protegido_vira_recomendacao_nao_teste() {
        let mut s = Sorteio(11);
        let (q, a) = sessao(&mut s, 120.0, 0.15, 0.6, 1.0);
        let inv = investigar(&q, &a, FREQ, None, &|n: &str| n == "Rajada.exe");
        assert!(matches!(inv.avaliacoes[0].teste, Teste::Recomendacao { .. }));
    }

    #[test]
    fn sinal_sempre_ligado_nao_e_associacao() {
        // O caso que a V1 erra: o programa dispara 45% do tempo e as travadas são independentes dele. (Acima de
        // metade do tempo, a mediana do próprio programa vira o "normal" e o sinal nem dispara, nas duas versões.)
        let mut s = Sorteio(21);
        let (q, a) = sessao(&mut s, 120.0, 0.45, 0.0, 10.0);
        let inv = investigar(&q, &a, FREQ, None, &nada_protegido);
        let rajada = inv.avaliacoes.iter().find(|x| x.hipotese == Hipotese::Processo { nome: "Rajada.exe".into() }).unwrap();
        assert!(rajada.confianca <= Confianca::SemEvidencia, "{:#?}", rajada);
        assert!(rajada.lift.is_some_and(|l| l < 1.5), "{:#?}", rajada);
        assert!(inv.conclusao.starts_with("Causa ainda não determinada"), "{}", inv.conclusao);

        let v1 = v1_na_mesma_sessao(&q, &a);
        assert!(
            v1.pistas.iter().any(|p| matches!(&p.suspeito, crate::core::travadas::Suspeito::SegundoPlano { processo } if processo == "Rajada.exe")
                && p.forca == crate::core::travadas::Forca::Alta),
            "a V1 chama de forte o que é coincidência: {:?}",
            v1.pistas
        );
    }

    pub(crate) fn v1_na_mesma_sessao(q: &[QuadroDoJogo], a: &[(i64, Amostra)]) -> crate::core::travadas::Investigacao {
        let intervalos: Vec<f64> = q.iter().map(|x| x.intervalo_ms).collect();
        let amostras: Vec<Amostra> = a.iter().map(|(_, x)| x.clone()).collect();
        crate::core::travadas::investigar(&intervalos, &amostras, 0, None)
    }

    #[test]
    fn poucas_travadas_abstem() {
        let mut s = Sorteio(2);
        let q = quadros(&mut s, 60.0, 11.0, &[10.0, 30.0, 50.0]);
        let a = amostras(&vec![false; 121]);
        let inv = investigar(&q, &a, FREQ, None, &nada_protegido);
        assert_eq!(inv.episodios.len(), 3);
        assert!(inv.abstencao.as_deref().is_some_and(|m| m.contains("poucas")), "{:?}", inv.abstencao);
        assert!(inv.avaliacoes.iter().all(|x| x.confianca <= Confianca::Inconclusiva));

        // Muitas travadas sem contador cobrindo: "não deu para medir", nunca "bom sinal".
        let q = quadros(&mut s, 60.0, 11.0, &[5.0, 12.0, 20.0, 31.0, 44.0, 52.0]);
        let inv = investigar(&q, &[], FREQ, None, &nada_protegido);
        let m = inv.abstencao.unwrap();
        assert!(m.contains("não cobriram") && !m.contains("bom sinal"), "{}", m);
        let mut s = Sorteio(3);
        let (q, a) = sessao(&mut s, 30.0, 0.2, 0.0, 4.0);
        let inv = investigar(&q[..900], &a, FREQ, None, &nada_protegido);
        assert!(inv.abstencao.as_deref().is_some_and(|m| m.contains("quadros")));
    }

    #[test]
    fn contador_que_nao_respondeu_e_nao_medido() {
        let mut s = Sorteio(4);
        let (q, mut a) = sessao(&mut s, 120.0, 0.15, 0.6, 1.0);
        for (_, x) in a.iter_mut() {
            x.disco_latencia_ms = None;
        }
        let inv = investigar(&q, &a, FREQ, None, &nada_protegido);
        let disco = inv.avaliacoes.iter().find(|x| x.hipotese == Hipotese::Disco).unwrap();
        assert_eq!(disco.confianca, Confianca::NaoMedido);
        assert_eq!(inv.avaliacoes.last().unwrap().confianca, Confianca::NaoMedido);
    }

    #[test]
    fn travada_do_lado_da_placa_pesa_contra_processo() {
        let mut s = Sorteio(11);
        let (mut q, a) = sessao(&mut s, 120.0, 0.15, 0.6, 1.0);
        for x in q.iter_mut().filter(|x| x.intervalo_ms > 40.0) {
            x.cpu_ms = Some(6.6);
            x.gpu_ms = Some(x.intervalo_ms * 0.9);
        }
        let inv = investigar(&q, &a, FREQ, None, &nada_protegido);
        let rajada = inv.avaliacoes.iter().find(|x| x.hipotese == Hipotese::Processo { nome: "Rajada.exe".into() }).unwrap();
        assert!(rajada.contra.iter().any(|c| c.contains("placa")), "{:?}", rajada.contra);
        assert!(inv.lado_placa > inv.lado_processador);
    }

    /// Critérios de aceite 1 e 2 (PIP-ENGENHARIA, seção 11), e a V1 medida nos mesmos dados.
    #[test]
    fn aceite_deteccao_e_falso_positivo_em_simulacao() {
        let sims = 200;
        let (mut acertos, mut falsos, mut falsos_v1) = (0, 0, 0);
        for semente in 0..sims {
            let mut s = Sorteio(1000 + semente);
            let (q, a) = sessao(&mut s, 120.0, 0.15, 0.6, 1.0);
            let inv = investigar(&q, &a, FREQ, None, &nada_protegido);
            if inv.avaliacoes.first().is_some_and(|x| {
                x.hipotese == Hipotese::Processo { nome: "Rajada.exe".into() } && x.confianca >= Confianca::Moderada
            }) {
                acertos += 1;
            }

            // Nula: rajadas 30% do tempo, travadas independentes.
            let mut s = Sorteio(5000 + semente);
            let (q, a) = sessao(&mut s, 120.0, 0.3, 0.0, 8.0);
            let inv = investigar(&q, &a, FREQ, None, &nada_protegido);
            if inv.avaliacoes.iter().any(|x| x.confianca >= Confianca::Moderada) {
                falsos += 1;
            }
            let v1 = v1_na_mesma_sessao(&q, &a);
            if v1.pistas.iter().any(|p| p.forca == crate::core::travadas::Forca::Alta) {
                falsos_v1 += 1;
            }
        }
        let (taxa_acerto, taxa_falso, taxa_falso_v1) =
            (acertos as f64 / sims as f64, falsos as f64 / sims as f64, falsos_v1 as f64 / sims as f64);
        println!("V2: acerto {:.3}, falso positivo {:.3} · V1: falso positivo {:.3}", taxa_acerto, taxa_falso, taxa_falso_v1);
        assert!(taxa_acerto >= 0.90, "acerto {}", taxa_acerto);
        assert!(taxa_falso <= 0.05, "falso positivo {}", taxa_falso);
    }
}
