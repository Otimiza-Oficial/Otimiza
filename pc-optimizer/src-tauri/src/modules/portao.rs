// O portão "nunca menos FPS" (2.9)
//
// A REGRA DO DONO, com as palavras dele: "quero que isso não diminua o meu
// FPS ou dos clientes". Um ajuste pode render numa máquina e custar em outra
// — foi o incidente da 2.1.0. Então todo ajuste de configuração de JOGO que o
// Otimiza aplica fica em observação:
//
//   1. no momento em que é aplicado, o jogo entra na vigília;
//   2. as medições automáticas das partidas (`medicoes.rs`) se acumulam de
//      um lado (antes) e do outro (depois);
//   3. com pelo menos 3 de cada lado, a regra comum do produto
//      (`modules::repeticoes`: intervalos de 95% que não se tocam) compara
//      FPS médio e 1% low;
//   4. se algum PIOROU com os intervalos separados E em pelo menos 5%, o ajuste é desfeito
//      sozinho, e a pessoa é avisada com os números;
//   5. se melhorou, ou ficou igual, a vigília termina e o resultado fica
//      guardado para a tela.
//
// Por que "além do ruído E 5%": partidas diferentes (servidor cheio, mapa
// outro) variam muito. O teste estatístico impede que uma noite pesada seja
// lida como piora; o piso de 5% impede que uma diferença real, porém
// irrelevante, desfaça um ajuste que o cliente quis.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::modules::repeticoes::{comparar, resumir, Diferenca};
use crate::modules::medicoes::MedicaoAutomatica;

/// Medições de cada lado para decidir.
pub const MINIMO_POR_LADO: usize = 3;
/// Quantas medições de cada lado entram na conta (as mais próximas do ajuste).
pub const MAXIMO_POR_LADO: usize = 6;
/// Piora mínima, em %, para desfazer.
pub const PIORA_MINIMA_PCT: f64 = 5.0;

/// Um ajuste em observação.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vigiado {
    /// Id no histórico (o que o desfazer usa).
    pub id: String,
    /// Nome para a tela.
    pub nome: String,
    /// Começo do nome do processo do jogo, em minúsculas (`fivem_`,
    /// `fortniteclient-win64-shipping`...). Casa com `MedicaoAutomatica.jogo`.
    pub processo: String,
    pub aplicado_em: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Veredito {
    /// Ainda faltam medições.
    Aguardando { antes: usize, depois: usize },
    /// Piorou: desfazer.
    Desfazer,
    Melhorou,
    SemMudanca,
}

/// As médias dos dois lados de uma métrica, e o que a regra concluiu.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Lados {
    pub media_base: f64,
    pub media_candidato: f64,
    pub diferenca: Diferenca,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Avaliacao {
    pub veredito: Veredito,
    pub fps: Option<Lados>,
    pub low_1pct: Option<Lados>,
}

fn lados(id: &str, antes: &[f64], depois: &[f64]) -> Option<Lados> {
    let (a, d) = (resumir(id, antes)?, resumir(id, depois)?);
    Some(Lados { media_base: a.media, media_candidato: d.media, diferenca: comparar(&a, &d) })
}

fn e_do_jogo(m: &MedicaoAutomatica, processo: &str) -> bool {
    m.jogo.to_lowercase().starts_with(processo)
}

/// A decisão. **Função pura.**
pub fn avaliar(v: &Vigiado, medicoes: &[MedicaoAutomatica]) -> Avaliacao {
    let mut antes: Vec<&MedicaoAutomatica> =
        medicoes.iter().filter(|m| e_do_jogo(m, &v.processo) && m.quando < v.aplicado_em).collect();
    let mut depois: Vec<&MedicaoAutomatica> =
        medicoes.iter().filter(|m| e_do_jogo(m, &v.processo) && m.quando > v.aplicado_em).collect();
    antes.sort_by_key(|m| std::cmp::Reverse(m.quando));
    depois.sort_by_key(|m| m.quando);
    antes.truncate(MAXIMO_POR_LADO);
    depois.truncate(MAXIMO_POR_LADO);

    if antes.len() < MINIMO_POR_LADO || depois.len() < MINIMO_POR_LADO {
        return Avaliacao { veredito: Veredito::Aguardando { antes: antes.len(), depois: depois.len() }, fps: None, low_1pct: None };
    }

    let fps = lados(
        "fps.average",
        &antes.iter().map(|m| m.fps).collect::<Vec<_>>(),
        &depois.iter().map(|m| m.fps).collect::<Vec<_>>(),
    );
    // 1% low só das medições com amostra suficiente para ele valer.
    let a1: Vec<f64> = antes.iter().filter(|m| m.confiavel).map(|m| m.low_1pct).collect();
    let d1: Vec<f64> = depois.iter().filter(|m| m.confiavel).map(|m| m.low_1pct).collect();
    let low = lados("fps.low_1pct", &a1, &d1);

    // FPS e 1% low: maior é melhor nos dois.
    let piorou = |c: &Option<Lados>| {
        c.as_ref().is_some_and(|c| matches!(c.diferenca, Diferenca::Real { delta, pct: Some(p), .. } if delta < 0.0 && -p >= PIORA_MINIMA_PCT))
    };
    let melhorou = |c: &Option<Lados>| c.as_ref().is_some_and(|c| matches!(c.diferenca, Diferenca::Real { delta, .. } if delta > 0.0));

    let veredito = if piorou(&fps) || piorou(&low) {
        Veredito::Desfazer
    } else if melhorou(&fps) || melhorou(&low) {
        Veredito::Melhorou
    } else {
        Veredito::SemMudanca
    };
    Avaliacao { veredito, fps, low_1pct: low }
}

// ------------------------------------------------------------ o arquivo

/// Vigiados em aberto e os resultados já decididos.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Estado {
    pub vigiados: Vec<Vigiado>,
    pub decididos: Vec<Decidido>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decidido {
    pub vigiado: Vigiado,
    pub veredito: Veredito,
    pub quando: u64,
    pub fps_antes: Option<f64>,
    pub fps_depois: Option<f64>,
    pub low_antes: Option<f64>,
    pub low_depois: Option<f64>,
    /// Quando o veredito foi desfazer e o desfazer falhou.
    pub erro: Option<String>,
}

fn arquivo() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("pc-optimizer").join("portao.json"))
}

pub fn ler() -> Estado {
    arquivo()
        .and_then(|a| std::fs::read_to_string(a).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn gravar(e: &Estado) -> Result<(), String> {
    let a = arquivo().ok_or("APPDATA ausente")?;
    if let Some(p) = a.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(e).map_err(|e| e.to_string())?;
    let tmp = a.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &a).map_err(|e| e.to_string())
}

/// Põe um ajuste em vigília (substitui um vigiado anterior do mesmo id, e
/// mantém o horário ORIGINAL: o "antes" é antes do primeiro ajuste).
pub fn vigiar(id: &str, nome: &str, processo: &str, agora: u64) {
    let mut e = ler();
    if !e.vigiados.iter().any(|v| v.id == id) {
        e.vigiados.push(Vigiado { id: id.into(), nome: nome.into(), processo: processo.to_lowercase(), aplicado_em: agora });
    }
    if let Err(erro) = gravar(&e) {
        crate::utils::Logger::warn(&format!("portão: não gravei a vigília de {}: {}", id, erro));
    }
}

/// Tira da vigília (o ajuste foi desfeito na mão).
pub fn esquecer(id: &str) {
    let mut e = ler();
    let antes = e.vigiados.len();
    e.vigiados.retain(|v| v.id != id);
    if e.vigiados.len() != antes {
        let _ = gravar(&e);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn m(jogo: &str, quando: u64, fps: f64, low: f64) -> MedicaoAutomatica {
        MedicaoAutomatica {
            jogo: jogo.into(),
            quando,
            fps,
            low_1pct: low,
            engasgos_por_minuto: 0.0,
            segundos: 30.0,
            confiavel: true,
            mudancas_aplicadas: 0,
            frametime_medio_ms: None,
            frametime_p95_ms: None,
            frametime_p99_ms: None,
            cpu_uso_pct: None,
            gpu_uso_pct: None,
            trancos_com_disco_pct: None,
            trancos_medidos: None,
            ambiente: None,
        }
    }

    fn vig() -> Vigiado {
        Vigiado { id: "config_unreal_x".into(), nome: "X".into(), processo: "x-win64".into(), aplicado_em: 1000 }
    }

    #[test]
    fn espera_ter_medicoes_dos_dois_lados() {
        let ms = vec![m("X-Win64-Shipping.exe", 10, 100.0, 60.0), m("X-Win64-Shipping.exe", 2000, 90.0, 50.0)];
        assert_eq!(avaliar(&vig(), &ms).veredito, Veredito::Aguardando { antes: 1, depois: 1 });
    }

    #[test]
    fn piora_clara_desfaz() {
        let mut ms = Vec::new();
        for (i, f) in [100.0, 102.0, 98.0, 101.0].iter().enumerate() {
            ms.push(m("X-Win64-Shipping.exe", 100 + i as u64, *f, 60.0 + i as f64));
        }
        for (i, f) in [85.0, 86.0, 84.0, 85.5].iter().enumerate() {
            ms.push(m("X-Win64-Shipping.exe", 2000 + i as u64, *f, 50.0 + i as f64));
        }
        assert_eq!(avaliar(&vig(), &ms).veredito, Veredito::Desfazer);
    }

    #[test]
    fn melhora_clara_confirma() {
        let mut ms = Vec::new();
        for (i, f) in [100.0, 102.0, 98.0].iter().enumerate() {
            ms.push(m("X-Win64-Shipping.exe", 100 + i as u64, *f, 60.0));
        }
        for (i, f) in [130.0, 128.0, 131.0].iter().enumerate() {
            ms.push(m("X-Win64-Shipping.exe", 2000 + i as u64, *f, 60.0 + i as f64));
        }
        assert_eq!(avaliar(&vig(), &ms).veredito, Veredito::Melhorou);
    }

    #[test]
    fn noite_pesada_nao_vira_piora() {
        // Partidas muito diferentes entre si: a média caiu 8%, mas dentro do
        // ruído. Não desfaz.
        let mut ms = Vec::new();
        for (i, f) in [100.0, 140.0, 70.0, 120.0].iter().enumerate() {
            ms.push(m("X-Win64-Shipping.exe", 100 + i as u64, *f, 50.0 + (i * 10) as f64));
        }
        for (i, f) in [95.0, 125.0, 65.0, 110.0].iter().enumerate() {
            ms.push(m("X-Win64-Shipping.exe", 2000 + i as u64, *f, 48.0 + (i * 10) as f64));
        }
        assert_ne!(avaliar(&vig(), &ms).veredito, Veredito::Desfazer);
    }

    #[test]
    fn so_conta_o_proprio_jogo() {
        let mut ms = Vec::new();
        for i in 0..4 {
            ms.push(m("Outro.exe", 100 + i, 200.0, 100.0));
            ms.push(m("Outro.exe", 2000 + i, 50.0, 20.0));
        }
        assert!(matches!(avaliar(&vig(), &ms).veredito, Veredito::Aguardando { .. }));
    }
}
