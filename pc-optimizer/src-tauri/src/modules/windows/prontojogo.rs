// Pronto para jogar? Poucos segundos antes de abrir o jogo: monitor abaixo da taxa, limite de FPS escondido,
// programa pesando agora, memória já apertada e modo jogo desligado. Cada item é MEDIDO agora; nada é escrito.

use serde::Serialize;

const PROGRAMA_PESADO: f64 = 0.05;
const RAM_LIVRE_MINIMA_MB: f64 = 1_500.0;
const COMMIT_ALTO_PCT: f64 = 85.0;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "tipo")]
pub enum Item {
    MonitorAbaixo { hz_atual: u32, hz_maximo: u32, monitor: String },
    LimitesEscondidos { quantos: usize },
    SegundoPlanoPesado { programas: Vec<(String, f64)> },
    MemoriaApertada { livre_mb: f64, commit_pct: Option<f64>, maiores: Vec<(String, f64)> },
    ModoJogoDesligado,
}

#[derive(Debug, Clone, Serialize)]
pub struct Prontidao {
    pub pronto: bool,
    pub itens: Vec<Item>,
    pub conferido: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Leitura {
    pub monitores: Vec<(String, u32, u32)>,
    pub limites: usize,
    pub processos_cpu: Vec<(String, f64)>,
    pub ram_livre_mb: Option<f64>,
    pub commit_pct: Option<f64>,
    pub maiores_em_memoria: Vec<(String, f64)>,
    pub modo_jogo_ligado: bool,
}

pub fn avaliar(l: &Leitura) -> Prontidao {
    let mut itens = Vec::new();
    let mut conferido = Vec::new();

    for (nome, atual, maximo) in &l.monitores {
        if atual < maximo {
            itens.push(Item::MonitorAbaixo { hz_atual: *atual, hz_maximo: *maximo, monitor: nome.clone() });
        } else {
            conferido.push(format!("{} a {} Hz", nome, atual));
        }
    }
    if l.limites > 0 {
        itens.push(Item::LimitesEscondidos { quantos: l.limites });
    } else {
        conferido.push("nenhum limite de FPS escondido".into());
    }
    let pesados: Vec<(String, f64)> = l
        .processos_cpu
        .iter()
        .filter(|(n, c)| *c >= PROGRAMA_PESADO && !super::governador::protegido(n))
        .cloned()
        .collect();
    if pesados.is_empty() {
        conferido.push("nada pesando em segundo plano".into());
    } else {
        itens.push(Item::SegundoPlanoPesado { programas: pesados });
    }
    let apertada = l.ram_livre_mb.is_some_and(|m| m < RAM_LIVRE_MINIMA_MB) || l.commit_pct.is_some_and(|c| c >= COMMIT_ALTO_PCT);
    match (apertada, l.ram_livre_mb) {
        (true, livre) => itens.push(Item::MemoriaApertada {
            livre_mb: livre.unwrap_or(0.0),
            commit_pct: l.commit_pct,
            maiores: l.maiores_em_memoria.clone(),
        }),
        (false, Some(livre)) => conferido.push(format!("{:.1} GB de memória livre", livre / 1024.0)),
        (false, None) => {}
    }
    if l.modo_jogo_ligado {
        conferido.push("modo jogo ligado".into());
    } else {
        itens.push(Item::ModoJogoDesligado);
    }
    let pronto = itens.iter().all(|i| matches!(i, Item::ModoJogoDesligado));
    Prontidao { pronto, itens, conferido }
}

#[cfg(windows)]
pub fn verificar() -> Prontidao {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

    let monitores = super::display::monitores()
        .iter()
        .map(|m| (m.descricao.clone(), m.hz_atual, m.hz_maximo()))
        .collect();
    let limites = super::tetos::procurar().tetos.len();

    let mut s = System::new();
    let cpu_e_memoria = ProcessRefreshKind::nothing().with_cpu().with_memory();
    s.refresh_processes_specifics(ProcessesToUpdate::All, true, cpu_e_memoria);
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.max(std::time::Duration::from_millis(1000)));
    s.refresh_processes_specifics(ProcessesToUpdate::All, true, cpu_e_memoria);
    s.refresh_memory();
    let nucleos = num_cpus::get().max(1) as f64;
    let eu = std::process::id();

    let mut cpu: std::collections::HashMap<String, f64> = Default::default();
    let mut mem: std::collections::HashMap<String, f64> = Default::default();
    for (pid, p) in s.processes() {
        if pid.as_u32() == eu || pid.as_u32() <= 4 {
            continue;
        }
        let nome = p.name().to_string_lossy().to_string();
        *cpu.entry(nome.clone()).or_default() += p.cpu_usage() as f64 / 100.0 / nucleos;
        *mem.entry(nome).or_default() += p.memory() as f64 / 1_048_576.0;
    }
    let mut processos_cpu: Vec<(String, f64)> = cpu.into_iter().collect();
    processos_cpu.sort_by(|a, b| b.1.total_cmp(&a.1));
    processos_cpu.truncate(5);
    let mut maiores: Vec<(String, f64)> = mem.into_iter().filter(|(n, _)| !super::governador::protegido(n)).collect();
    maiores.sort_by(|a, b| b.1.total_cmp(&a.1));
    maiores.truncate(3);

    let total = s.total_memory() as f64;
    let commit = crate::core::pdh::Consulta::nova().and_then(|q| {
        let c = q.adicionar(r"\Memory\% Committed Bytes In Use");
        q.coletar();
        q.valor(c)
    });

    avaliar(&Leitura {
        monitores,
        limites,
        processos_cpu,
        ram_livre_mb: (total > 0.0).then(|| s.available_memory() as f64 / 1_048_576.0),
        commit_pct: commit,
        maiores_em_memoria: maiores,
        modo_jogo_ligado: crate::modules::preferences::Preferences::load().auto_game_mode,
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn tudo_certo() -> Leitura {
        Leitura {
            monitores: vec![("Monitor".into(), 180, 180)],
            limites: 0,
            processos_cpu: vec![("chrome.exe".into(), 0.01)],
            ram_livre_mb: Some(6_000.0),
            commit_pct: Some(40.0),
            maiores_em_memoria: vec![],
            modo_jogo_ligado: true,
        }
    }

    #[test]
    #[ignore = "lê esta máquina"]
    fn verifica_esta_maquina() {
        println!("{:#?}", verificar());
    }

    #[test]
    fn maquina_pronta() {
        let p = avaliar(&tudo_certo());
        assert!(p.pronto);
        assert!(p.itens.is_empty());
        assert!(p.conferido.iter().any(|c| c.contains("180 Hz")));
    }

    #[test]
    fn monitor_a_60_nao_esta_pronto() {
        let l = Leitura { monitores: vec![("Monitor".into(), 60, 144)], ..tudo_certo() };
        let p = avaliar(&l);
        assert!(!p.pronto);
        assert!(matches!(p.itens[0], Item::MonitorAbaixo { hz_atual: 60, hz_maximo: 144, .. }));
    }

    #[test]
    fn programa_pesado_e_memoria_apertada() {
        let l = Leitura {
            processos_cpu: vec![("OneDrive.exe".into(), 0.12), ("Discord.exe".into(), 0.2)],
            ram_livre_mb: Some(900.0),
            ..tudo_certo()
        };
        let p = avaliar(&l);
        assert!(!p.pronto);
        let programas = p.itens.iter().find_map(|i| match i {
            Item::SegundoPlanoPesado { programas } => Some(programas.clone()),
            _ => None,
        });
        // Discord é voz: protegido, não entra na lista.
        assert_eq!(programas.unwrap(), vec![("OneDrive.exe".to_string(), 0.12)]);
        assert!(p.itens.iter().any(|i| matches!(i, Item::MemoriaApertada { .. })));
    }

    #[test]
    fn modo_jogo_desligado_e_so_recomendacao() {
        let l = Leitura { modo_jogo_ligado: false, ..tudo_certo() };
        let p = avaliar(&l);
        assert!(p.pronto);
        assert_eq!(p.itens, vec![Item::ModoJogoDesligado]);
    }
}
