// Detetive de travadas (2.9)
//
// "O jogo trava de vez em quando" é a queixa mais comum e a mais difícil de
// atacar, porque na hora que a pessoa abre um otimizador a travada já passou.
// Este módulo junta, NA MESMA JANELA DE TEMPO, cada quadro que demorou demais
// e o que a máquina estava fazendo naquele meio segundo: disco, paginação,
// memória de vídeo, o núcleo mais ocupado, a placa, e os programas em segundo
// plano que mais usavam processador.
//
// O QUE ELE AFIRMA, E O QUE NÃO AFIRMA
//
// A telemetria é amostrada a cada meio segundo; uma travada dura dezenas de
// milissegundos. Então o que se tem é COINCIDÊNCIA NO TEMPO, não causa
// provada. A tela diz "provável" e mostra a força da coincidência:
//
//   - Alta: o recurso saltou muito acima do normal daquela partida na mesma
//     amostra da travada, e isso se repete em pelo menos metade das travadas.
//   - Média: o salto aconteceu, mas em menos da metade.
//
// Travada sem nenhum salto fica "sem causa visível" — que também é
// informação: costuma ser compilação de shader ou o próprio jogo, e nenhum
// ajuste de Windows resolve.
//
// Função pura: recebe intervalos e amostras, devolve a explicação.

use serde::Serialize;

use super::estatistica::mediana;
use super::fluidez::{gravidade, Gravidade};
use super::telemetria::Amostra;

/// Uma travada, no tempo da medição.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Travada {
    /// Milissegundos desde o primeiro quadro medido.
    pub instante_ms: f64,
    pub duracao_ms: f64,
    pub gravidade: Gravidade,
}

/// Acha as travadas perceptíveis ou piores. O instante é a soma dos
/// intervalos até ali (o relógio dos próprios quadros).
pub fn localizar(intervalos_ms: &[f64]) -> Vec<Travada> {
    let validos: Vec<f64> = intervalos_ms.iter().copied().filter(|x| x.is_finite() && *x > 0.0).collect();
    let Some(med) = mediana(&validos) else { return Vec::new() };
    let mut t = 0.0;
    let mut v = Vec::new();
    for &x in &validos {
        t += x;
        if let Some(g) = gravidade(x, med) {
            if g >= Gravidade::Perceptivel {
                v.push(Travada { instante_ms: t, duracao_ms: x, gravidade: g });
            }
        }
    }
    v
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(tag = "tipo")]
pub enum Suspeito {
    /// O disco demorou para responder.
    Disco,
    /// O Windows leu memória do disco (falta de RAM).
    Paginacao,
    /// A memória de vídeo encheu ou transbordou para a RAM.
    Vram,
    /// Um núcleo do processador bateu no teto.
    ThreadPrincipal,
    /// Um programa em segundo plano disparou o uso de processador.
    SegundoPlano { processo: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Forca {
    Media,
    Alta,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pista {
    pub suspeito: Suspeito,
    pub forca: Forca,
    /// Em quantas das travadas este suspeito estava presente.
    pub travadas: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Investigacao {
    pub travadas: Vec<Travada>,
    pub pistas: Vec<Pista>,
    /// Travadas sem nenhum suspeito na mesma amostra.
    pub sem_causa_visivel: usize,
}

/// Quanto acima do normal (mediana da janela) um valor precisa estar para
/// contar como salto, e o mínimo absoluto — sem o mínimo, um disco a 1 ms
/// que foi para 3 ms seria "triplicou".
const SALTO: f64 = 3.0;
const DISCO_MIN_MS: f64 = 20.0;
const PAGINAS_MIN: f64 = 200.0;
const VRAM_CHEIA: f64 = 0.95;
const VRAM_TRANSBORDO_MB: f64 = 100.0;
const NUCLEO_TETO: f64 = 95.0;
const PROCESSO_MIN: f64 = 0.08;

fn salto(valor: Option<f64>, normal: Option<f64>, minimo: f64) -> bool {
    match (valor, normal) {
        (Some(v), Some(n)) => v >= minimo && v >= n.max(f64::EPSILON) * SALTO,
        (Some(v), None) => v >= minimo,
        _ => false,
    }
}

/// A amostra que cobre o instante (a primeira com `instante_ms` >= t, e a
/// anterior a ela — meio segundo para cada lado).
fn vizinhas(amostras: &[Amostra], inicio_ms: u64, t: f64) -> Vec<&Amostra> {
    let alvo = inicio_ms as f64 + t;
    let i = amostras.iter().position(|a| a.instante_ms as f64 >= alvo).unwrap_or(amostras.len());
    let mut v = Vec::new();
    if i > 0 {
        v.push(&amostras[i - 1]);
    }
    if i < amostras.len() {
        v.push(&amostras[i]);
    }
    v
}

/// Investiga. `inicio_ms` é o instante (no relógio das amostras) em que a
/// medição de quadros começou; `vram_total_mb` quando conhecido.
pub fn investigar(intervalos_ms: &[f64], amostras: &[Amostra], inicio_ms: u64, vram_total_mb: Option<f64>) -> Investigacao {
    let travadas = localizar(intervalos_ms);
    let med = |f: &dyn Fn(&Amostra) -> Option<f64>| mediana(&amostras.iter().filter_map(f).collect::<Vec<_>>());
    let disco_n = med(&|a| a.disco_latencia_ms);
    let pag_n = med(&|a| a.paginas_lidas_s);
    let comp_n = med(&|a| a.vram_compartilhada_mb);
    let nucleo_n = med(&|a| a.cpu_nucleo_max_pct);

    // Uso normal de cada processo na janela, para saber quem disparou.
    let mut normal_proc: std::collections::HashMap<&str, Vec<f64>> = Default::default();
    for a in amostras {
        for p in &a.processos {
            normal_proc.entry(p.nome.as_str()).or_default().push(p.cpu);
        }
    }
    let n_amostras = amostras.len().max(1);
    let normal_de = |nome: &str| -> f64 {
        // Ausente numa amostra = 0 naquela amostra.
        let mut v = normal_proc.get(nome).cloned().unwrap_or_default();
        v.resize(n_amostras, 0.0);
        mediana(&v).unwrap_or(0.0)
    };

    let mut contagem: std::collections::BTreeMap<Suspeito, usize> = Default::default();
    let mut sem_causa = 0;
    for t in &travadas {
        let viz = vizinhas(amostras, inicio_ms, t.instante_ms);
        let mut suspeitos = std::collections::BTreeSet::new();
        for a in viz {
            if salto(a.disco_latencia_ms, disco_n, DISCO_MIN_MS) {
                suspeitos.insert(Suspeito::Disco);
            }
            if salto(a.paginas_lidas_s, pag_n, PAGINAS_MIN) {
                suspeitos.insert(Suspeito::Paginacao);
            }
            let cheia = match (a.vram_usada_mb, vram_total_mb) {
                (Some(u), Some(t)) if t > 0.0 => u / t >= VRAM_CHEIA,
                _ => false,
            };
            let transbordou = match (a.vram_compartilhada_mb, comp_n) {
                (Some(c), Some(n)) => c - n >= VRAM_TRANSBORDO_MB,
                _ => false,
            };
            if cheia || transbordou {
                suspeitos.insert(Suspeito::Vram);
            }
            if a.cpu_nucleo_max_pct.is_some_and(|n| n >= NUCLEO_TETO) && nucleo_n.is_some_and(|n| n < NUCLEO_TETO - 10.0) {
                suspeitos.insert(Suspeito::ThreadPrincipal);
            }
            for p in &a.processos {
                if p.cpu >= PROCESSO_MIN && p.cpu >= normal_de(&p.nome).max(0.005) * SALTO {
                    suspeitos.insert(Suspeito::SegundoPlano { processo: p.nome.clone() });
                }
            }
        }
        if suspeitos.is_empty() {
            sem_causa += 1;
        }
        for s in suspeitos {
            *contagem.entry(s).or_default() += 1;
        }
    }

    let total = travadas.len().max(1);
    let mut pistas: Vec<Pista> = contagem
        .into_iter()
        .map(|(suspeito, n)| Pista { suspeito, forca: if n * 2 >= total { Forca::Alta } else { Forca::Media }, travadas: n })
        .collect();
    pistas.sort_by(|a, b| b.travadas.cmp(&a.travadas));
    Investigacao { travadas, pistas, sem_causa_visivel: sem_causa }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::core::telemetria::ProcessoNaAmostra;

    fn base(ms: u64) -> Amostra {
        Amostra {
            instante_ms: ms,
            cpu_nucleo_max_pct: Some(60.0),
            disco_latencia_ms: Some(1.0),
            paginas_lidas_s: Some(5.0),
            vram_usada_mb: Some(2000.0),
            vram_compartilhada_mb: Some(50.0),
            ..Default::default()
        }
    }

    /// 20 s a 10 ms por quadro, com travadas de 60 ms nos instantes pedidos.
    fn quadros_com_travadas(em_ms: &[f64]) -> Vec<f64> {
        let mut v = Vec::new();
        let mut t = 0.0;
        let mut i = 0;
        while t < 20_000.0 {
            let x = if i < em_ms.len() && t + 10.0 >= em_ms[i] {
                i += 1;
                60.0
            } else {
                10.0
            };
            t += x;
            v.push(x);
        }
        v
    }

    #[test]
    fn acha_as_travadas_no_tempo() {
        let q = quadros_com_travadas(&[5_000.0, 12_000.0]);
        let t = localizar(&q);
        assert_eq!(t.len(), 2);
        assert!((t[0].instante_ms - 5_060.0).abs() < 30.0, "{:?}", t[0]);
        assert_eq!(t[0].gravidade, Gravidade::Severo);
    }

    #[test]
    fn disco_que_salta_na_hora_da_travada() {
        let q = quadros_com_travadas(&[5_000.0, 12_000.0]);
        let amostras: Vec<Amostra> = (0..=40)
            .map(|i| {
                let ms = i * 500;
                let mut a = base(ms);
                if ms == 5_500 || ms == 12_500 {
                    a.disco_latencia_ms = Some(80.0);
                }
                a
            })
            .collect();
        let inv = investigar(&q, &amostras, 0, Some(4096.0));
        assert_eq!(inv.pistas[0].suspeito, Suspeito::Disco);
        assert_eq!(inv.pistas[0].forca, Forca::Alta);
        assert_eq!(inv.sem_causa_visivel, 0);
    }

    #[test]
    fn programa_em_segundo_plano_que_dispara() {
        let q = quadros_com_travadas(&[8_000.0]);
        let amostras: Vec<Amostra> = (0..=40)
            .map(|i| {
                let ms = i * 500;
                let mut a = base(ms);
                a.processos = vec![ProcessoNaAmostra { nome: "OneDrive.exe".into(), cpu: if ms == 8_500 { 0.25 } else { 0.01 } }];
                a
            })
            .collect();
        let inv = investigar(&q, &amostras, 0, None);
        assert_eq!(inv.pistas[0].suspeito, Suspeito::SegundoPlano { processo: "OneDrive.exe".into() });
    }

    #[test]
    fn travada_sem_salto_fica_sem_causa() {
        let q = quadros_com_travadas(&[3_000.0, 9_000.0, 15_000.0]);
        let amostras: Vec<Amostra> = (0..=40).map(|i| base(i * 500)).collect();
        let inv = investigar(&q, &amostras, 0, Some(4096.0));
        assert!(inv.pistas.is_empty());
        assert_eq!(inv.sem_causa_visivel, 3);
    }

    #[test]
    fn disco_sempre_lento_nao_e_salto() {
        // Um HD que vive a 30 ms não "saltou" na hora da travada.
        let q = quadros_com_travadas(&[5_000.0]);
        let amostras: Vec<Amostra> = (0..=40).map(|i| Amostra { disco_latencia_ms: Some(30.0), ..base(i * 500) }).collect();
        let inv = investigar(&q, &amostras, 0, None);
        assert!(!inv.pistas.iter().any(|p| p.suspeito == Suspeito::Disco));
    }
}
