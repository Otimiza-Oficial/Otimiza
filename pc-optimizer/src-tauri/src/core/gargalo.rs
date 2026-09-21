// Classificador de gargalo — vários ao mesmo tempo, com a evidência junto
//
// O `bottleneck.rs` antigo escolhia UM limite a partir de médias. Na prática
// um jogo pode estar preso na thread principal E com a VRAM estourando E com
// o disco engasgando no mesmo minuto. E "CPU a 40%" não quer dizer CPU
// folgada: pode ser um núcleo a 100% com os outros parados — o caso mais
// comum de FPS baixo em jogo de mundo aberto e servidor cheio, e o motivo de
// "troquei a placa de vídeo e nada mudou".
//
// Aqui cada gargalo é testado sozinho sobre a JANELA de amostras (não sobre a
// média), e só aparece com a fração do tempo em que a condição valeu. Um
// pico de um segundo numa janela de trinta não vira diagnóstico.
//
// Função pura. Os limites são constantes nomeadas logo abaixo.

use serde::{Deserialize, Serialize};

use super::estatistica::mediana;
use super::telemetria::Amostra;

/// Núcleo considerado "no talo". Não é 100 porque o contador oscila e o
/// escalonador migra a thread entre núcleos.
const NUCLEO_CHEIO: f64 = 90.0;
/// GPU considerada o limite.
const GPU_CHEIA: f64 = 95.0;
/// Abaixo disto a GPU está esperando alguém.
const GPU_ESPERANDO: f64 = 85.0;
/// VRAM ocupada que já força o driver a despejar recurso para a RAM.
const VRAM_CRITICA: f64 = 0.95;
const VRAM_ALTA: f64 = 0.88;
/// RAM livre abaixo da qual o Windows começa a tirar página dos programas.
const RAM_LIVRE_CRITICA_MB: f64 = 700.0;
const COMMIT_CRITICO_PCT: f64 = 90.0;
/// Páginas lidas do disco por segundo, sustentadas, durante o jogo.
const PAGINACAO_ALTA: f64 = 400.0;
/// Latência média de disco que já aparece como engasgo de carregamento.
const DISCO_LENTO_MS: f64 = 25.0;
/// Clock efetivo abaixo desta fração do nominal, com carga: o processador
/// está segurando a frequência (temperatura ou limite de potência).
const CLOCK_SEGURADO: f64 = 0.85;
/// Fração da janela para cada nível de confiança.
const FRACAO_ALTA: f64 = 0.6;
const FRACAO_MEDIA: f64 = 0.3;
/// Taxas típicas de limite de FPS (V-Sync, limitadores, monitores).
const TETOS_COMUNS: &[f64] = &[30.0, 60.0, 75.0, 90.0, 100.0, 120.0, 144.0, 165.0, 180.0, 200.0, 240.0, 360.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gargalo {
    /// Um núcleo no talo e a GPU esperando: a thread principal do jogo.
    ThreadPrincipal,
    /// Todos os núcleos ocupados.
    CpuInteira,
    Gpu,
    /// A memória de vídeo acabou ou está acabando.
    Vram,
    /// RAM acabando / paginação.
    Memoria,
    Disco,
    /// O processador está abaixo do clock nominal com carga.
    ClockSegurado,
    /// O FPS está preso num teto (V-Sync, limitador) com CPU e GPU folgadas.
    LimiteDeFps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Confianca {
    Media,
    Alta,
}

/// Um gargalo encontrado, com os números que o justificam.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Achado {
    pub gargalo: Gargalo,
    pub confianca: Confianca,
    /// Fração da janela em que a condição valeu (0–1).
    pub fracao: f64,
    /// Os números que a tela mostra do lado do diagnóstico.
    pub evidencia: Vec<(&'static str, f64)>,
}

/// O que o FPS estava fazendo durante a janela, quando havia jogo medido.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuadrosDaJanela {
    pub fps_medio: f64,
    /// Desvio do tempo de quadro ÷ média. Teto de FPS é quase uma reta.
    pub frametime_cv: f64,
}

pub struct Contexto {
    pub vram_total_mb: Option<f64>,
    pub quadros: Option<QuadrosDaJanela>,
}

fn fracao(amostras: &[Amostra], condicao: impl Fn(&Amostra) -> Option<bool>) -> Option<f64> {
    let avaliadas: Vec<bool> = amostras.iter().filter_map(&condicao).collect();
    if avaliadas.is_empty() {
        return None;
    }
    Some(avaliadas.iter().filter(|b| **b).count() as f64 / avaliadas.len() as f64)
}

fn med(amostras: &[Amostra], campo: impl Fn(&Amostra) -> Option<f64>) -> Option<f64> {
    let v: Vec<f64> = amostras.iter().filter_map(campo).collect();
    mediana(&v)
}

fn nivel(f: f64) -> Option<Confianca> {
    if f >= FRACAO_ALTA {
        Some(Confianca::Alta)
    } else if f >= FRACAO_MEDIA {
        Some(Confianca::Media)
    } else {
        None
    }
}

/// Classifica a janela. Lista vazia = nenhum gargalo identificado (o que
/// pode ser "sobra de máquina" ou "o limite é o próprio jogo" — a tela diz
/// as duas possibilidades, não escolhe uma).
pub fn classificar(amostras: &[Amostra], ctx: &Contexto) -> Vec<Achado> {
    let mut achados = Vec::new();
    let mut empurrar = |gargalo: Gargalo, f: Option<f64>, evidencia: Vec<(&'static str, Option<f64>)>| {
        if let Some(f) = f {
            if let Some(confianca) = nivel(f) {
                achados.push(Achado {
                    gargalo,
                    confianca,
                    fracao: (f * 100.0).round() / 100.0,
                    evidencia: evidencia.into_iter().filter_map(|(k, v)| v.map(|v| (k, (v * 10.0).round() / 10.0))).collect(),
                });
            }
        }
    };

    let cpu = med(amostras, |a| a.cpu_total_pct);
    let nucleo = med(amostras, |a| a.cpu_nucleo_max_pct);
    let gpu = med(amostras, |a| a.gpu_pct);

    // Thread principal: um núcleo cheio, a CPU inteira NÃO, e a GPU esperando.
    empurrar(
        Gargalo::ThreadPrincipal,
        fracao(amostras, |a| {
            Some(a.cpu_nucleo_max_pct? >= NUCLEO_CHEIO && a.cpu_total_pct? < NUCLEO_CHEIO && a.gpu_pct? < GPU_ESPERANDO)
        }),
        vec![("cpu_total_pct", cpu), ("nucleo_max_pct", nucleo), ("gpu_pct", gpu)],
    );
    empurrar(
        Gargalo::CpuInteira,
        fracao(amostras, |a| Some(a.cpu_total_pct? >= NUCLEO_CHEIO)),
        vec![("cpu_total_pct", cpu), ("gpu_pct", gpu)],
    );
    empurrar(Gargalo::Gpu, fracao(amostras, |a| Some(a.gpu_pct? >= GPU_CHEIA)), vec![("gpu_pct", gpu)]);

    if let Some(total) = ctx.vram_total_mb.filter(|t| *t > 0.0) {
        let usada = med(amostras, |a| a.vram_usada_mb);
        let critica = fracao(amostras, |a| Some(a.vram_usada_mb? / total >= VRAM_CRITICA));
        let alta_e_transbordando = fracao(amostras, |a| {
            Some(a.vram_usada_mb? / total >= VRAM_ALTA && a.vram_compartilhada_mb? > 300.0)
        });
        let f = match (critica, alta_e_transbordando) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        empurrar(
            Gargalo::Vram,
            f,
            vec![("vram_usada_mb", usada), ("vram_total_mb", Some(total)), ("vram_compartilhada_mb", med(amostras, |a| a.vram_compartilhada_mb))],
        );
    }

    empurrar(
        Gargalo::Memoria,
        fracao(amostras, |a| {
            let livre = a.ram_disponivel_mb.map(|m| m < RAM_LIVRE_CRITICA_MB);
            let commit = a.commit_pct.map(|c| c >= COMMIT_CRITICO_PCT);
            let paginando = a.paginas_lidas_s.map(|p| p >= PAGINACAO_ALTA);
            match (livre, commit, paginando) {
                (None, None, None) => None,
                (l, c, p) => Some(l.unwrap_or(false) || c.unwrap_or(false) || p.unwrap_or(false)),
            }
        }),
        vec![
            ("ram_disponivel_mb", med(amostras, |a| a.ram_disponivel_mb)),
            ("commit_pct", med(amostras, |a| a.commit_pct)),
            ("paginas_lidas_s", med(amostras, |a| a.paginas_lidas_s)),
        ],
    );
    empurrar(
        Gargalo::Disco,
        fracao(amostras, |a| Some(a.disco_latencia_ms? >= DISCO_LENTO_MS && a.disco_ocupado_pct.unwrap_or(100.0) >= 50.0)),
        vec![("disco_latencia_ms", med(amostras, |a| a.disco_latencia_ms)), ("disco_ocupado_pct", med(amostras, |a| a.disco_ocupado_pct))],
    );
    empurrar(
        Gargalo::ClockSegurado,
        fracao(amostras, |a| {
            if a.cpu_nucleo_max_pct? < 80.0 {
                return None; // sem carga, clock baixo é economia, não limite
            }
            Some(a.clock_efetivo_mhz? / a.clock_nominal_mhz? < CLOCK_SEGURADO)
        }),
        vec![("clock_efetivo_mhz", med(amostras, |a| a.clock_efetivo_mhz)), ("clock_nominal_mhz", med(amostras, |a| a.clock_nominal_mhz))],
    );

    // Teto de FPS: precisa do jogo medido. FPS colado num valor comum, quase
    // sem variação, e nem CPU nem GPU no limite.
    if let Some(q) = ctx.quadros {
        let perto_de_teto = TETOS_COMUNS.iter().any(|t| (q.fps_medio - t).abs() / t < 0.03);
        let folgado = gpu.map(|g| g < GPU_ESPERANDO).unwrap_or(false) && nucleo.map(|n| n < NUCLEO_CHEIO).unwrap_or(false);
        if perto_de_teto && q.frametime_cv < 0.08 && folgado {
            empurrar(
                Gargalo::LimiteDeFps,
                Some(1.0),
                vec![("fps_medio", Some(q.fps_medio)), ("gpu_pct", gpu), ("nucleo_max_pct", nucleo)],
            );
        }
    }

    // Quem aparece primeiro na tela: alta confiança antes, e dentro dela, a
    // ordem do enum (thread principal é o caso mais comum e mais mal-entendido).
    achados.sort_by(|a, b| b.confianca.cmp(&a.confianca));
    achados
}

#[cfg(test)]
mod testes {
    use super::*;

    fn janela(n: usize, f: impl Fn(usize) -> Amostra) -> Vec<Amostra> {
        (0..n).map(f).collect()
    }

    fn base() -> Amostra {
        Amostra {
            cpu_total_pct: Some(30.0),
            cpu_nucleo_max_pct: Some(50.0),
            gpu_pct: Some(60.0),
            vram_usada_mb: Some(1500.0),
            vram_compartilhada_mb: Some(50.0),
            ram_disponivel_mb: Some(6000.0),
            commit_pct: Some(40.0),
            paginas_lidas_s: Some(5.0),
            disco_latencia_ms: Some(1.0),
            disco_ocupado_pct: Some(5.0),
            clock_efetivo_mhz: Some(4200.0),
            clock_nominal_mhz: Some(3600.0),
            ..Default::default()
        }
    }

    fn ctx() -> Contexto {
        Contexto { vram_total_mb: Some(4096.0), quadros: None }
    }

    fn tipos(v: &[Achado]) -> Vec<Gargalo> {
        v.iter().map(|a| a.gargalo).collect()
    }

    #[test]
    fn cpu_a_40_pct_com_um_nucleo_no_talo_e_thread_principal() {
        let j = janela(30, |_| Amostra { cpu_total_pct: Some(40.0), cpu_nucleo_max_pct: Some(98.0), gpu_pct: Some(55.0), ..base() });
        let r = classificar(&j, &ctx());
        assert_eq!(tipos(&r), vec![Gargalo::ThreadPrincipal]);
        assert_eq!(r[0].confianca, Confianca::Alta);
    }

    #[test]
    fn gpu_no_limite_nao_e_thread_principal() {
        let j = janela(30, |_| Amostra { cpu_nucleo_max_pct: Some(95.0), gpu_pct: Some(99.0), ..base() });
        assert_eq!(tipos(&classificar(&j, &ctx())), vec![Gargalo::Gpu]);
    }

    #[test]
    fn varios_ao_mesmo_tempo() {
        let j = janela(30, |_| Amostra {
            gpu_pct: Some(99.0),
            vram_usada_mb: Some(4000.0),
            ram_disponivel_mb: Some(400.0),
            ..base()
        });
        let r = tipos(&classificar(&j, &ctx()));
        assert!(r.contains(&Gargalo::Gpu) && r.contains(&Gargalo::Vram) && r.contains(&Gargalo::Memoria), "{:?}", r);
    }

    #[test]
    fn pico_curto_nao_vira_diagnostico() {
        let j = janela(30, |i| if i < 3 { Amostra { gpu_pct: Some(100.0), ..base() } } else { base() });
        assert!(classificar(&j, &ctx()).is_empty());
    }

    #[test]
    fn metade_do_tempo_e_confianca_media() {
        let j = janela(30, |i| if i % 2 == 0 { Amostra { gpu_pct: Some(99.0), ..base() } } else { base() });
        let r = classificar(&j, &ctx());
        assert_eq!(r[0].gargalo, Gargalo::Gpu);
        assert_eq!(r[0].confianca, Confianca::Media);
    }

    #[test]
    fn clock_abaixo_do_nominal_so_conta_com_carga() {
        let parado = janela(30, |_| Amostra { clock_efetivo_mhz: Some(1200.0), ..base() });
        assert!(classificar(&parado, &ctx()).is_empty());
        let segurado = janela(30, |_| Amostra { clock_efetivo_mhz: Some(2400.0), cpu_nucleo_max_pct: Some(85.0), ..base() });
        assert_eq!(tipos(&classificar(&segurado, &ctx())), vec![Gargalo::ClockSegurado]);
    }

    #[test]
    fn teto_de_fps_com_maquina_folgada() {
        let j = janela(30, |_| base());
        let c = Contexto { vram_total_mb: Some(4096.0), quadros: Some(QuadrosDaJanela { fps_medio: 59.9, frametime_cv: 0.02 }) };
        assert_eq!(tipos(&classificar(&j, &c)), vec![Gargalo::LimiteDeFps]);
        // Mesmo FPS, mas com a GPU cheia: não é teto, é a placa.
        let j = janela(30, |_| Amostra { gpu_pct: Some(99.0), ..base() });
        assert_eq!(tipos(&classificar(&j, &c)), vec![Gargalo::Gpu]);
    }

    #[test]
    fn contador_ausente_nao_acusa_nada() {
        let j = janela(30, |_| Amostra::default());
        assert!(classificar(&j, &ctx()).is_empty());
    }
}
