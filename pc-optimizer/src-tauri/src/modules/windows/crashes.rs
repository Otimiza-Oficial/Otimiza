// Os crashes do jogo, lidos dos relatórios que o próprio FiveM grava (`%LOCALAPPDATA%\FiveM\FiveM.app\crashes` e o
// `logs\CitizenFX_log_*.log` da sessão), cruzados com o que o Otimiza mudou antes de cada um. Cliente reclamando que
// "depois da otimização o jogo crasha" e produto sem saber responder era o pior dos mundos: aqui a resposta vem com
// a assinatura do crash, a memória no momento, e se aquilo já acontecia antes de o Otimiza mexer. Só lê.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Janela em que uma mudança do Otimiza é listada como "veio antes deste crash".
pub const JANELA_ANTES_S: u64 = 48 * 3600;

/// Memória acima disso no instante do crash: o Windows estava sem folga.
pub const MEMORIA_NO_LIMITE_PCT: u32 = 90;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Crash {
    /// Segundos desde 1970 (quando o FiveM gravou o relatório).
    pub quando: u64,
    /// "An error at GTA5_b3258.exe+14B7EA2" ou "fragInstGta Pool Full, Size == 1600".
    pub erro: Option<String>,
    /// "bacon-vegan-carbon": o mesmo defeito tem o mesmo hash em qualquer PC.
    pub hash: Option<String>,
    pub memoria_pct: Option<u32>,
    pub paginacao_livre_mb: Option<u64>,
    pub veiculo: Option<String>,
    /// Minutos de jogo naquela sessão.
    pub minutos_de_jogo: Option<u64>,
    /// Ids do histórico aplicados nas 48 h antes.
    #[serde(default)]
    pub mudancas_antes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pista {
    /// O mesmo hash em dois ou mais crashes: um defeito que se repete no mesmo ponto do jogo.
    MesmoDefeito,
    /// Um limite do motor do jogo (pool) estourou: depende do servidor, não do Windows.
    PoolCheio,
    /// Memória ou paginação no limite no instante do crash.
    MemoriaNoLimite,
    /// O mesmo veículo em dois ou mais crashes: item adicionado pelo servidor.
    MesmoVeiculo,
    /// Crash anterior à primeira mudança do Otimiza nesta máquina.
    JaAcontecia,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Leitura {
    pub pista: Pista,
    pub frase: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Relatorio {
    /// Do mais novo para o mais velho.
    pub crashes: Vec<Crash>,
    pub leituras: Vec<Leitura>,
    /// Primeira mudança do Otimiza registrada nesta máquina.
    pub primeira_mudanca: Option<u64>,
    pub antes_do_otimiza: usize,
    pub depois_do_otimiza: usize,
    /// Pasta de crashes do FiveM não existe: não há o que ler (e não é "zero crashes").
    pub sem_fivem: bool,
}

fn numero_entre_parenteses(linha: &str) -> Option<u64> {
    let dentro = linha.split('(').nth(1)?.split(')').next()?;
    dentro.trim().trim_end_matches('%').trim_end_matches("Mb").trim().parse().ok()
}

/// Pura: o `.dmp.gamelog` que o FiveM grava junto do crash.
pub fn ler_gamelog(texto: &str) -> Crash {
    let mut c = Crash::default();
    for linha in texto.lines() {
        let l = linha.trim();
        if l.starts_with("Memory Load") {
            c.memoria_pct = numero_entre_parenteses(l).map(|v| v as u32);
        } else if l.starts_with("PageFile Free") {
            c.paginacao_livre_mb = numero_entre_parenteses(l);
        } else if let Some(v) = l.strip_prefix("Current Vehicle :") {
            let v = v.trim();
            if !v.is_empty() && v != "None" {
                c.veiculo = Some(v.to_string());
            }
        } else if l.starts_with("System Time (") {
            // "System Time (0h 13m 47s)": minutos desde que o jogo abriu.
            let dentro = l.split('(').nth(1).and_then(|s| s.split(')').next()).unwrap_or("");
            let mut min = 0u64;
            for parte in dentro.split_whitespace() {
                if let Some(h) = parte.strip_suffix('h').and_then(|n| n.parse::<u64>().ok()) {
                    min += h * 60;
                } else if let Some(m) = parte.strip_suffix('m').and_then(|n| n.parse::<u64>().ok()) {
                    min += m;
                }
            }
            c.minutos_de_jogo = Some(min);
        }
    }
    c
}

/// Pura: o bloco "Process crash captured" do `CitizenFX_log`. O erro original vence o genérico.
pub fn ler_dialogo(log: &str) -> (Option<String>, Option<String>) {
    let Some(i) = log.rfind("Process crash captured") else { return (None, None) };
    let bloco: Vec<&str> = log[i..].lines().skip(1).take(15).collect();
    let conteudo = |l: &str| l.split("/ ").nth(1).map(|s| s.trim().to_string());

    let mut erro = None;
    let mut hash = None;
    for l in &bloco {
        let Some(t) = conteudo(l) else { continue };
        if let Some(h) = t.strip_prefix("Legacy crash hash:") {
            hash = Some(h.trim().to_string());
        } else if let Some(o) = t.strip_prefix("Original error:") {
            erro = Some(o.trim().to_string());
        } else if erro.is_none() && t.starts_with("An error at ") {
            erro = Some(t.split(" caused").next().unwrap_or(&t).to_string());
        }
    }
    (erro, hash)
}

/// Pura: a leitura em frases, pelas pistas que os próprios relatórios dão.
pub fn ler(crashes: &[Crash], primeira_mudanca: Option<u64>) -> Vec<Leitura> {
    use std::collections::HashMap;
    let mut leituras = Vec::new();

    let mut por_hash: HashMap<&str, usize> = HashMap::new();
    for h in crashes.iter().filter_map(|c| c.hash.as_deref()) {
        *por_hash.entry(h).or_insert(0) += 1;
    }
    let mut repetidos: Vec<(&str, usize)> = por_hash.into_iter().filter(|(_, n)| *n >= 2).collect();
    repetidos.sort_by(|a, b| b.1.cmp(&a.1));
    for (h, n) in repetidos {
        leituras.push(Leitura {
            pista: Pista::MesmoDefeito,
            frase: format!(
                "{n} crashes com a mesma assinatura ({h}). O mesmo defeito se repetindo costuma ser do jogo ou de um \
                 item do servidor naquele ponto, não do Windows; a assinatura é o que o suporte do FiveM pede."
            ),
        });
    }

    let pools = crashes.iter().filter(|c| c.erro.as_deref().is_some_and(|e| e.contains("Pool Full"))).count();
    if pools > 0 {
        leituras.push(Leitura {
            pista: Pista::PoolCheio,
            frase: format!(
                "{pools} crash(es) por limite do jogo estourado (\"Pool Full\"): o servidor criou mais objetos do que o \
                 jogo comporta. Isso não depende do Windows nem do Otimiza."
            ),
        });
    }

    let memoria = crashes
        .iter()
        .filter(|c| c.memoria_pct.is_some_and(|m| m >= MEMORIA_NO_LIMITE_PCT) || c.paginacao_livre_mb.is_some_and(|p| p < 1024))
        .count();
    if memoria > 0 {
        leituras.push(Leitura {
            pista: Pista::MemoriaNoLimite,
            frase: format!(
                "Em {memoria} crash(es) a memória estava no limite (acima de {MEMORIA_NO_LIMITE_PCT}% ou menos de 1 GB de \
                 paginação livre). Fechar navegador e Discord antes de jogar dá folga; com 8 GB, o FiveM cheio vive \
                 perto disso."
            ),
        });
    }

    let mut por_veiculo: HashMap<&str, usize> = HashMap::new();
    for v in crashes.iter().filter_map(|c| c.veiculo.as_deref()) {
        *por_veiculo.entry(v).or_insert(0) += 1;
    }
    if let Some((v, n)) = por_veiculo.into_iter().filter(|(_, n)| *n >= 2).max_by_key(|(_, n)| *n) {
        leituras.push(Leitura {
            pista: Pista::MesmoVeiculo,
            frase: format!(
                "{n} crashes dentro do mesmo veículo ({v}). Se ele for um veículo do servidor, é o principal suspeito: \
                 vale avisar a equipe do servidor."
            ),
        });
    }

    if let Some(p) = primeira_mudanca {
        let antes = crashes.iter().filter(|c| c.quando < p).count();
        if antes > 0 {
            leituras.push(Leitura {
                pista: Pista::JaAcontecia,
                frase: format!(
                    "{antes} crash(es) aconteceram antes da primeira mudança do Otimiza nesta máquina: já existiam sem ele."
                ),
            });
        }
    }

    leituras
}

/// Pura: o que o Otimiza aplicou nas `JANELA_ANTES_S` antes de cada crash.
pub fn cruzar(crashes: &mut [Crash], mudancas: &[(u64, String)]) {
    for c in crashes.iter_mut() {
        c.mudancas_antes = mudancas
            .iter()
            .filter(|(t, _)| *t <= c.quando && c.quando - *t <= JANELA_ANTES_S)
            .map(|(_, id)| id.clone())
            .collect();
    }
}

fn pasta_do_fivem() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    Some(PathBuf::from(base).join("FiveM").join("FiveM.app"))
}

fn segundos(p: &Path) -> Option<u64> {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// O log da sessão que caiu termina no mesmo instante do crash (o FiveM fecha o arquivo junto).
fn log_da_sessao(logs: &[(u64, PathBuf)], quando: u64) -> Option<&PathBuf> {
    logs.iter().filter(|(t, _)| t.abs_diff(quando) <= 30).min_by_key(|(t, _)| t.abs_diff(quando)).map(|(_, p)| p)
}

/// `mudancas`: (instante, id) de cada item do histórico.
pub fn relatorio(mudancas: &[(u64, String)]) -> Relatorio {
    let Some(raiz) = pasta_do_fivem().filter(|p| p.join("crashes").is_dir()) else {
        return Relatorio { sem_fivem: true, ..Default::default() };
    };

    let logs: Vec<(u64, PathBuf)> = std::fs::read_dir(raiz.join("logs"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "log"))
        .filter_map(|p| segundos(&p).map(|t| (t, p)))
        .collect();

    let mut crashes: Vec<Crash> = std::fs::read_dir(raiz.join("crashes"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".dmp.gamelog"))
        .filter_map(|p| {
            let quando = segundos(&p)?;
            let mut c = ler_gamelog(&std::fs::read_to_string(&p).ok()?);
            c.quando = quando;
            if let Some(texto) = log_da_sessao(&logs, quando).and_then(|l| std::fs::read(l).ok()) {
                let (erro, hash) = ler_dialogo(&String::from_utf8_lossy(&texto));
                c.erro = erro;
                c.hash = hash;
            }
            Some(c)
        })
        .collect();
    crashes.sort_by(|a, b| b.quando.cmp(&a.quando));

    let primeira_mudanca = mudancas.iter().map(|(t, _)| *t).min();
    cruzar(&mut crashes, mudancas);

    let antes = primeira_mudanca.map_or(0, |p| crashes.iter().filter(|c| c.quando < p).count());
    Relatorio {
        leituras: ler(&crashes, primeira_mudanca),
        antes_do_otimiza: antes,
        depois_do_otimiza: crashes.len() - antes,
        crashes,
        primeira_mudanca,
        sem_fivem: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trecho real de um `.dmp.gamelog` do FiveM b3258 (crash de 26/09, 15:47, na máquina do dono).
    const GAMELOG: &str = "###### DEBUG INFO ######

============ SYSTEM INFO ============
Game State : Game Running
System Time (0h 13m 47s)
System Memory :
  Memory Load       : (        85%)
  Physical Total    : (      8089Mb)
  PageFile Total    : (     22425Mb)
  PageFile Free     : (      3073Mb)
Current Mission : None
Current Vehicle : SG 1150
";

    const LOG_B3258: &str = "[    636719] [    DumpServer]                  272/ Process crash captured. Crash dialog content:
[    636719] [    DumpServer]                  272/ An error at GTA5_b3258.exe+14B7EA2 caused FiveM to stop working. A crash report is being uploaded to the FiveM developers.
[    636719] [    DumpServer]                  272/ Legacy crash hash: bacon-vegan-carbon
";

    const LOG_POOL: &str = "[    852578] [    DumpServer]                 7800/ Process crash captured. Crash dialog content:
[    852578] [    DumpServer]                 7800/ Recursive-recursive error: fragInstGta Pool Full, Size == 1600
[    852578] [    DumpServer]                 7800/ Recursive error: fragInstGta Pool Full, Size == 1600
[    852578] [    DumpServer]                 7800/ Original error: fragInstGta Pool Full, Size == 1600
";

    #[test]
    fn o_gamelog_da_memoria_veiculo_e_tempo_de_jogo() {
        let c = ler_gamelog(GAMELOG);
        assert_eq!(c.memoria_pct, Some(85));
        assert_eq!(c.paginacao_livre_mb, Some(3073));
        assert_eq!(c.veiculo.as_deref(), Some("SG 1150"));
        assert_eq!(c.minutos_de_jogo, Some(13));
    }

    #[test]
    fn sem_veiculo_nao_vira_veiculo() {
        let c = ler_gamelog("Current Vehicle : None\n");
        assert_eq!(c.veiculo, None);
    }

    #[test]
    fn o_dialogo_da_o_erro_e_o_hash() {
        let (erro, hash) = ler_dialogo(LOG_B3258);
        assert_eq!(erro.as_deref(), Some("An error at GTA5_b3258.exe+14B7EA2"));
        assert_eq!(hash.as_deref(), Some("bacon-vegan-carbon"));

        let (erro, hash) = ler_dialogo(LOG_POOL);
        assert_eq!(erro.as_deref(), Some("fragInstGta Pool Full, Size == 1600"));
        assert_eq!(hash, None);

        assert_eq!(ler_dialogo("sessão sem crash"), (None, None));
    }

    fn crash(quando: u64, hash: Option<&str>, erro: Option<&str>, memoria: u32, veiculo: Option<&str>) -> Crash {
        Crash {
            quando,
            erro: erro.map(Into::into),
            hash: hash.map(Into::into),
            memoria_pct: Some(memoria),
            paginacao_livre_mb: Some(3000),
            veiculo: veiculo.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn as_pistas_saem_dos_relatorios() {
        let lista = vec![
            crash(100, Some("bacon-vegan-carbon"), None, 85, Some("SG 1150")),
            crash(200, Some("bacon-vegan-carbon"), None, 84, Some("SG 1150")),
            crash(300, None, Some("fragInstGta Pool Full, Size == 1600"), 96, None),
            crash(10, None, None, 50, None),
        ];
        let pistas: Vec<Pista> = ler(&lista, Some(50)).into_iter().map(|l| l.pista).collect();
        assert!(pistas.contains(&Pista::MesmoDefeito));
        assert!(pistas.contains(&Pista::PoolCheio));
        assert!(pistas.contains(&Pista::MemoriaNoLimite));
        assert!(pistas.contains(&Pista::MesmoVeiculo));
        assert!(pistas.contains(&Pista::JaAcontecia));
    }

    #[test]
    fn crash_isolado_nao_inventa_pista() {
        let lista = vec![crash(100, Some("um-so"), None, 60, Some("Adder"))];
        assert!(ler(&lista, Some(10)).is_empty());
    }

    #[test]
    fn so_entra_a_mudanca_das_48h_antes() {
        let mut lista = vec![crash(JANELA_ANTES_S + 1000, None, None, 50, None)];
        let mudancas = vec![
            (500, "velha".to_string()),
            (1500, "dentro".to_string()),
            (JANELA_ANTES_S + 2000, "depois".to_string()),
        ];
        cruzar(&mut lista, &mudancas);
        assert_eq!(lista[0].mudancas_antes, vec!["dentro".to_string()]);
    }

    /// Lê os crashes DESTA máquina: `cargo test --lib -- --ignored --nocapture crashes_desta_maquina`.
    #[test]
    #[ignore]
    fn crashes_desta_maquina() {
        let r = relatorio(&[]);
        for c in &r.crashes {
            println!("{} {:?} {:?} mem {:?} veiculo {:?}", c.quando, c.erro, c.hash, c.memoria_pct, c.veiculo);
        }
        for l in &r.leituras {
            println!("{:?}: {}", l.pista, l.frase);
        }
    }

    #[test]
    fn o_log_da_sessao_e_o_que_fechou_junto() {
        let logs = vec![(1000, PathBuf::from("a.log")), (5000, PathBuf::from("b.log")), (5010, PathBuf::from("c.log"))];
        assert_eq!(log_da_sessao(&logs, 5012), Some(&PathBuf::from("c.log")));
        assert_eq!(log_da_sessao(&logs, 3000), None);
    }
}
