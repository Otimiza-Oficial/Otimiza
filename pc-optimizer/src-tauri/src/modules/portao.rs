// O portão "nunca menos FPS" (regra do dono). Todo ajuste de jogo fica em vigília: com pelo menos 3 partidas
// medidas de cada lado, compara FPS médio e 1% low pela regra de `modules::repeticoes`. Piorou com os intervalos
// separados E pelo menos 5%: desfaz sozinho e avisa com os números. O teste estatístico impede que uma noite
// pesada vire piora; o piso de 5% impede desfazer por uma diferença real, porém irrelevante.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::modules::repeticoes::{comparar, resumir, Diferenca};
use crate::modules::medicoes::MedicaoAutomatica;

pub const MINIMO_POR_LADO: usize = 3;
/// Os mais próximos do ajuste.
pub const MAXIMO_POR_LADO: usize = 6;
pub const PIORA_MINIMA_PCT: f64 = 5.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vigiado {
    pub id: String,
    pub nome: String,
    /// Casa com `MedicaoAutomatica.jogo` (`fivem_`, `fortniteclient-win64-shipping`...).
    pub processo: String,
    pub aplicado_em: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Veredito {
    Aguardando { antes: usize, depois: usize },
    Desfazer,
    Melhorou,
    SemMudanca,
}

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

pub fn avaliar(v: &Vigiado, medicoes: &[MedicaoAutomatica]) -> Avaliacao {
    let mut antes: Vec<&MedicaoAutomatica> =
        medicoes.iter().filter(|m| e_do_jogo(m, &v.processo) && m.quando < v.aplicado_em).collect();
    let mut depois: Vec<&MedicaoAutomatica> =
        medicoes.iter().filter(|m| e_do_jogo(m, &v.processo) && m.quando > v.aplicado_em).collect();
    antes.sort_by_key(|m| std::cmp::Reverse(m.quando));
    depois.sort_by_key(|m| m.quando);
    antes.truncate(MAXIMO_POR_LADO);
    depois.truncate(MAXIMO_POR_LADO);
    decidir(&antes, &depois)
}

/// Serve ao ajuste de jogo (antes/depois de uma data) e ao governador (partidas sem/com ele).
fn decidir(antes: &[&MedicaoAutomatica], depois: &[&MedicaoAutomatica]) -> Avaliacao {
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

// O governador passa pela mesma regra. Sem data de "antes", os lados são PARTIDAS: `Acalmou` (agiu) e
// `Absteve` (partida de comparação, com programa para acalmar; sem candidato mediria o programa, não o
// governador). As partidas se alternam até a comparação fechar. Piorou: devolve e fica parado naquele jogo.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernadorNaPartida {
    Acalmou,
    Absteve,
}

/// Estado que mudou no meio da medição não serve a nenhum dos lados.
pub fn marcar(inicio: Option<GovernadorNaPartida>, fim: Option<GovernadorNaPartida>) -> Option<GovernadorNaPartida> {
    if inicio == fim {
        inicio
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernadorNoJogo {
    pub processo: String,
    pub nome: String,
    pub desde: u64,
    #[serde(default)]
    pub decidido: Option<Decidido>,
    #[serde(default)]
    pub sessoes_agindo: Vec<u64>,
}

/// Partida curta demais ou medição que falha não pode virar governador agindo sem prova indefinidamente.
pub const SESSOES_AGINDO_SEM_MEDICAO: usize = 3;
const SESSOES_GUARDADAS: usize = 10;

impl GovernadorNoJogo {
    pub fn como_vigiado(&self) -> Vigiado {
        Vigiado {
            id: format!("governador:{}", self.processo),
            nome: format!("governador do modo jogo em {}", self.nome),
            processo: self.processo.clone(),
            aplicado_em: self.desde,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Rodada {
    Agir,
    Comparar,
    Reprovado(Decidido),
    /// Sem medição não há como provar que ele não tira FPS.
    SemMedicao(String),
}

pub fn lados_do_governador<'a>(
    processo: &str,
    medicoes: &'a [MedicaoAutomatica],
) -> (Vec<&'a MedicaoAutomatica>, Vec<&'a MedicaoAutomatica>) {
    let lado = |qual: GovernadorNaPartida| {
        let mut v: Vec<&MedicaoAutomatica> =
            medicoes.iter().filter(|m| e_do_jogo(m, processo) && m.governador == Some(qual)).collect();
        v.sort_by_key(|m| std::cmp::Reverse(m.quando));
        v.truncate(MAXIMO_POR_LADO);
        v
    };
    (lado(GovernadorNaPartida::Absteve), lado(GovernadorNaPartida::Acalmou))
}

pub fn avaliar_governador(processo: &str, medicoes: &[MedicaoAutomatica]) -> Avaliacao {
    let (sem, com) = lados_do_governador(processo, medicoes);
    decidir(&sem, &com)
}

/// `medicoes` `None` (arquivo ilegível) ou `estado` `None` (portao.json ilegível, o veredito pode estar lá):
/// ele não age. `medindo` = medição automática ligada e possível.
pub fn rodada_do_governador(
    estado: Option<&Estado>,
    processo: &str,
    medicoes: Option<&[MedicaoAutomatica]>,
    medindo: bool,
) -> Rodada {
    let Some(estado) = estado else {
        return Rodada::SemMedicao("não consegui ler o registro do portão, onde fica o veredito dele".into());
    };
    let vigilia = estado.governador.iter().find(|g| g.processo == processo);
    if let Some(d) = vigilia.and_then(|g| g.decidido.as_ref()) {
        return if d.veredito == Veredito::Desfazer { Rodada::Reprovado(d.clone()) } else { Rodada::Agir };
    }
    if !medindo {
        return Rodada::SemMedicao(
            "a medição automática de quadros está desligada ou sem administrador, e sem medir não dá para provar que ele não tira FPS".into(),
        );
    }
    let Some(medicoes) = medicoes else {
        return Rodada::SemMedicao("não consegui ler as medições das partidas anteriores para conferir o efeito dele".into());
    };
    let (sem, com) = lados_do_governador(processo, medicoes);
    // Uma partida de comparação medida zera a conta: prova que a medição voltou a funcionar.
    let ultima_marca = medicoes
        .iter()
        .filter(|m| e_do_jogo(m, processo) && m.governador.is_some())
        .map(|m| m.quando)
        .max()
        .unwrap_or(0);
    let agindo_sem_medicao = vigilia.map(|g| g.sessoes_agindo.iter().filter(|&&t| t > ultima_marca).count()).unwrap_or(0);
    // Empate começa pela comparação: o lado sem governador é a referência.
    if sem.len() <= com.len() || agindo_sem_medicao >= SESSOES_AGINDO_SEM_MEDICAO {
        Rodada::Comparar
    } else {
        Rodada::Agir
    }
}

pub fn frase_do_governador(d: &Decidido) -> String {
    let numeros = match (d.fps_antes, d.fps_depois) {
        (Some(a), Some(b)) => format!(" (FPS médio {:.0} sem ele, {:.0} com ele", a, b),
        _ => String::from(" ("),
    };
    let low = match (d.low_antes, d.low_depois) {
        (Some(a), Some(b)) => format!("; 1% pior {:.0} sem, {:.0} com)", a, b),
        _ => String::from(")"),
    };
    let numeros = if numeros == " (" && low == ")" { String::new() } else { format!("{}{}", numeros, low) };
    match d.veredito {
        Veredito::Desfazer => match &d.erro {
            None => format!(
                "O {} foi desligado sozinho: nas partidas medidas o jogo rodou pior com ele{}. Os programas voltaram ao normal.",
                d.vigiado.nome, numeros
            ),
            Some(e) => format!(
                "O {} derrubou o FPS{} e foi desligado, mas não consegui devolver tudo: {}",
                d.vigiado.nome, numeros, e
            ),
        },
        Veredito::Melhorou => format!("O {} passou na comparação: o jogo rodou melhor com ele{}.", d.vigiado.nome, numeros),
        _ => format!("O {} passou na comparação: sem diferença além do ruído{}.", d.vigiado.nome, numeros),
    }
}

/// Erro sem gravar quando o arquivo existe e não se lê: gravar por cima apagaria vigílias e vereditos.
pub fn observar_governador(processo: &str, nome: &str, agora: u64, agindo: bool) -> Result<(), String> {
    let mut e = ler_estrito()?;
    let i = match e.governador.iter().position(|g| g.processo == processo) {
        Some(i) => i,
        None => {
            e.governador.push(GovernadorNoJogo {
                processo: processo.into(),
                nome: nome.into(),
                desde: agora,
                decidido: None,
                sessoes_agindo: Vec::new(),
            });
            e.governador.len() - 1
        }
    };
    if agindo {
        let s = &mut e.governador[i].sessoes_agindo;
        s.push(agora);
        let excesso = s.len().saturating_sub(SESSOES_GUARDADAS);
        s.drain(..excesso);
    }
    gravar(&e).map_err(|erro| format!("não gravei a vigília do governador em {}: {}", nome, erro))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Estado {
    pub vigiados: Vec<Vigiado>,
    pub decididos: Vec<Decidido>,
    #[serde(default)]
    pub governador: Vec<GovernadorNoJogo>,
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
    pub erro: Option<String>,
}

fn arquivo() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("pc-optimizer").join("portao.json"))
}

/// Distingue "não existe" de "existe e não se lê": quem vai GRAVAR depois usa esta.
pub fn ler_estrito() -> Result<Estado, String> {
    let a = arquivo().ok_or("APPDATA ausente")?;
    match std::fs::read_to_string(&a) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| format!("portao.json ilegível: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Estado::default()),
        Err(e) => Err(format!("portao.json: {}", e)),
    }
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

/// Mantém o horário ORIGINAL: o "antes" é antes do primeiro ajuste.
pub fn vigiar(id: &str, nome: &str, processo: &str, agora: u64) {
    let mut e = ler();
    if !e.vigiados.iter().any(|v| v.id == id) {
        e.vigiados.push(Vigiado { id: id.into(), nome: nome.into(), processo: processo.to_lowercase(), aplicado_em: agora });
    }
    if let Err(erro) = gravar(&e) {
        crate::utils::Logger::warn(&format!("portão: não gravei a vigília de {}: {}", id, erro));
    }
}

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
            placa: None,
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
            governador: None,
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

    fn g(quando: u64, fps: f64, lado: GovernadorNaPartida) -> MedicaoAutomatica {
        MedicaoAutomatica { governador: Some(lado), ..m("FiveM_b3258_GTAProcess.exe", quando, fps, fps * 0.6) }
    }

    fn estado_com(decidido: Option<Decidido>) -> Estado {
        Estado {
            governador: vec![GovernadorNoJogo { processo: "fivem_".into(), nome: "FiveM".into(), desde: 1, decidido, sessoes_agindo: Vec::new() }],
            ..Default::default()
        }
    }

    #[test]
    fn governador_que_derruba_fps_e_reprovado() {
        use GovernadorNaPartida::*;
        let mut ms = Vec::new();
        for (i, f) in [140.0, 142.0, 139.0].iter().enumerate() {
            ms.push(g(10 + i as u64, *f, Absteve));
        }
        for (i, f) in [120.0, 121.0, 119.0].iter().enumerate() {
            ms.push(g(20 + i as u64, *f, Acalmou));
        }
        let a = avaliar_governador("fivem_", &ms);
        assert_eq!(a.veredito, Veredito::Desfazer);
        assert_eq!(a.fps.as_ref().map(|l| l.media_base.round()), Some(140.0), "a base é a partida SEM ele");
    }

    #[test]
    fn governador_sem_efeito_segue() {
        use GovernadorNaPartida::*;
        let mut ms = Vec::new();
        for (i, f) in [140.0, 142.0, 139.0, 141.0].iter().enumerate() {
            ms.push(g(10 + i as u64, *f, Absteve));
            ms.push(g(50 + i as u64, *f + 0.5, Acalmou));
        }
        assert_ne!(avaliar_governador("fivem_", &ms).veredito, Veredito::Desfazer);
    }

    #[test]
    fn medicao_sem_marca_nao_entra_em_lado_nenhum() {
        let ms: Vec<MedicaoAutomatica> = (0..8).map(|i| m("FiveM_b3258_GTAProcess.exe", i, 100.0, 60.0)).collect();
        assert_eq!(avaliar_governador("fivem_", &ms).veredito, Veredito::Aguardando { antes: 0, depois: 0 });
    }

    #[test]
    fn marca_que_muda_no_meio_nao_vale() {
        use GovernadorNaPartida::*;
        assert_eq!(marcar(Some(Acalmou), Some(Acalmou)), Some(Acalmou));
        assert_eq!(marcar(None, Some(Acalmou)), None);
        assert_eq!(marcar(Some(Absteve), Some(Acalmou)), None);
    }

    #[test]
    fn rodadas_alternam_para_encher_o_lado_menor() {
        use GovernadorNaPartida::*;
        let vazio = Estado::default();
        assert!(matches!(rodada_do_governador(Some(&vazio), "fivem_", Some(&[]), true), Rodada::Comparar), "começa pela referência");
        let ms = vec![g(1, 100.0, Absteve)];
        assert!(matches!(rodada_do_governador(Some(&vazio), "fivem_", Some(&ms), true), Rodada::Agir));
        let ms = vec![g(1, 100.0, Absteve), g(2, 100.0, Acalmou)];
        assert!(matches!(rodada_do_governador(Some(&vazio), "fivem_", Some(&ms), true), Rodada::Comparar));
    }

    #[test]
    fn sem_medicao_o_governador_nao_age() {
        let vazio = Estado::default();
        assert!(matches!(rodada_do_governador(Some(&vazio), "fivem_", Some(&[]), false), Rodada::SemMedicao(_)));
        assert!(matches!(rodada_do_governador(Some(&vazio), "fivem_", None, true), Rodada::SemMedicao(_)));
    }

    #[test]
    fn reprovado_fica_parado_e_aprovado_age() {
        let d = |veredito| Decidido {
            vigiado: estado_com(None).governador[0].como_vigiado(),
            veredito,
            quando: 9,
            fps_antes: Some(140.0),
            fps_depois: Some(120.0),
            low_antes: None,
            low_depois: None,
            erro: None,
        };
        let reprovado = estado_com(Some(d(Veredito::Desfazer)));
        assert!(matches!(rodada_do_governador(Some(&reprovado), "fivem_", Some(&[]), true), Rodada::Reprovado(_)));
        assert!(frase_do_governador(&d(Veredito::Desfazer)).contains("140 sem ele, 120 com ele"));
        let aprovado = estado_com(Some(d(Veredito::SemMudanca)));
        assert!(matches!(rodada_do_governador(Some(&aprovado), "fivem_", Some(&[]), true), Rodada::Agir));
    }

    #[test]
    fn estado_antigo_sem_governador_continua_legivel() {
        let e: Estado = serde_json::from_str(r#"{"vigiados":[],"decididos":[]}"#).unwrap();
        assert!(e.governador.is_empty());
    }

    #[test]
    fn portao_ilegivel_nao_solta_o_governador() {
        assert!(matches!(rodada_do_governador(None, "fivem_", Some(&[]), true), Rodada::SemMedicao(_)));
    }

    #[test]
    fn agir_sem_medicao_que_conte_tem_limite() {
        use GovernadorNaPartida::*;
        let ms = vec![g(100, 140.0, Absteve)];
        let com_sessoes = |sessoes: Vec<u64>| Estado {
            governador: vec![GovernadorNoJogo {
                processo: "fivem_".into(),
                nome: "FiveM".into(),
                desde: 1,
                decidido: None,
                sessoes_agindo: sessoes,
            }],
            ..Default::default()
        };
        assert!(matches!(rodada_do_governador(Some(&com_sessoes(vec![200, 300])), "fivem_", Some(&ms), true), Rodada::Agir));
        let travado = com_sessoes(vec![200, 300, 400]);
        assert!(matches!(rodada_do_governador(Some(&travado), "fivem_", Some(&ms), true), Rodada::Comparar));
        let ms2 = vec![g(100, 140.0, Absteve), g(500, 139.0, Absteve)];
        assert!(matches!(rodada_do_governador(Some(&travado), "fivem_", Some(&ms2), true), Rodada::Agir));
        assert!(matches!(rodada_do_governador(Some(&com_sessoes(vec![10, 20, 30])), "fivem_", Some(&ms), true), Rodada::Agir));
    }
}
