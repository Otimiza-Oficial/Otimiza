// VIGIA DE ESTABILIDADE (Plano de refino, E2 e E3). Regra do dono: otimização que trava, congela ou derruba o PC é
// otimização ruim, e o Otimiza não faz. O cliente não liga "o PC desligou sozinho ontem" ao ajuste de anteontem; o
// Otimiza liga. Na abertura, lê no registro do Windows os desligamentos inesperados (Kernel-Power 41), as telas azuis
// (BugCheck 1001), o driver de vídeo reiniciado (Display 4101) e os erros de hardware (WHEA), e cruza com o que ele
// mesmo aplicou nas 48 horas antes de cada um.
//
// Só desfaz quando o PC ESTAVA ESTÁVEL antes: nenhuma dessas quedas nos 30 dias anteriores ao primeiro ajuste
// suspeito. Num PC que já caía, culpar o último ajuste seria chute, e a tela diz isso em vez de desfazer. Cada queda é
// tratada uma vez só (`estabilidade.json` guarda até onde já se olhou). O ajuste que pede reiniciar (E3) cai na
// mesma janela: aplicado, reiniciou e voltou com tela azul, é desfeito na abertura seguinte.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Até quanto tempo depois de um ajuste uma queda conta contra ele.
pub const JANELA_S: i64 = 48 * 3600;
/// O PC precisa ter ficado este tempo sem quedas antes do ajuste para a culpa ser dele.
pub const ESTAVEL_ANTES_S: i64 = 30 * 24 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(specta::Type))]
pub enum Queda {
    Desligamento,
    TelaAzul,
    VideoReiniciou,
    Hardware,
}

impl Queda {
    pub fn frase(self) -> &'static str {
        match self {
            Queda::Desligamento => "o PC desligou sem desligar normalmente",
            Queda::TelaAzul => "o Windows deu tela azul",
            Queda::VideoReiniciou => "o driver de vídeo parou e reiniciou",
            Queda::Hardware => "o Windows registrou um erro de hardware",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Ocorrencia {
    /// Segundos desde 1970.
    pub quando: i64,
    pub queda: Queda,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(specta::Type))]
pub struct Aplicado {
    pub id: String,
    pub nome: String,
    pub quando: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Julgamento {
    /// O PC estava estável e caiu depois destes ajustes: desfazer.
    Desfazer { queda: Ocorrencia, ajustes: Vec<Aplicado> },
    /// Caiu depois de ajustes, mas já caía antes deles: não dá para culpar, só avisar.
    JaCaiaAntes { queda: Ocorrencia, ajustes: Vec<Aplicado> },
}

/// **Pura.** O provedor e o número do evento viram o tipo de queda.
pub fn queda_do_evento(provedor: &str, id: u32, nivel: u32) -> Option<Queda> {
    match (provedor, id) {
        ("Microsoft-Windows-Kernel-Power", 41) => Some(Queda::Desligamento),
        ("Microsoft-Windows-WER-SystemErrorReporting", 1001) | ("BugCheck", 1001) => Some(Queda::TelaAzul),
        ("Display", 4101) => Some(Queda::VideoReiniciou),
        ("Microsoft-Windows-WHEA-Logger", _) if nivel <= 2 => Some(Queda::Hardware),
        _ => None,
    }
}

/// **Pura.** Para cada queda ainda não olhada (depois de `olhado_ate`), os ajustes aplicados na janela antes dela.
/// Desfazer só se não houve queda nenhuma nos 30 dias antes do primeiro desses ajustes.
pub fn julgar(quedas: &[Ocorrencia], aplicados: &[Aplicado], olhado_ate: i64) -> Vec<Julgamento> {
    let mut ordenadas: Vec<&Ocorrencia> = quedas.iter().filter(|q| q.quando > olhado_ate).collect();
    ordenadas.sort_by_key(|q| q.quando);
    let mut ja_culpados: Vec<String> = Vec::new();
    let mut saida = Vec::new();
    for q in ordenadas {
        let suspeitos: Vec<Aplicado> = aplicados
            .iter()
            .filter(|a| a.quando < q.quando && q.quando - a.quando <= JANELA_S)
            .filter(|a| !ja_culpados.contains(&a.id))
            .cloned()
            .collect();
        if suspeitos.is_empty() {
            continue;
        }
        let primeiro = suspeitos.iter().map(|a| a.quando).min().unwrap_or(q.quando);
        let caia_antes = quedas
            .iter()
            .any(|x| x.quando < primeiro && primeiro - x.quando <= ESTAVEL_ANTES_S);
        ja_culpados.extend(suspeitos.iter().map(|a| a.id.clone()));
        saida.push(if caia_antes {
            Julgamento::JaCaiaAntes { queda: q.clone(), ajustes: suspeitos }
        } else {
            Julgamento::Desfazer { queda: q.clone(), ajustes: suspeitos }
        });
    }
    saida
}

/// "2026-11-10T20:31:50" (hora local, como o `Get-WinEvent` devolve) → segundos desde 1970.
pub fn epoch_local(s: &str) -> Option<i64> {
    use chrono::{Local, NaiveDateTime, TimeZone};
    let n = NaiveDateTime::parse_from_str(s.get(..19)?, "%Y-%m-%dT%H:%M:%S").ok()?;
    Local.from_local_datetime(&n).earliest().map(|d| d.timestamp())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Memoria {
    /// Quedas até este instante já foram julgadas.
    olhado_ate: i64,
}

fn arquivo() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|a| PathBuf::from(a).join("pc-optimizer").join("estabilidade.json"))
}

fn ler_memoria() -> Memoria {
    arquivo()
        .and_then(|a| std::fs::read_to_string(a).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn gravar_memoria(m: &Memoria) {
    if let (Some(a), Ok(json)) = (arquivo(), serde_json::to_string(m)) {
        let _ = std::fs::create_dir_all(a.parent().unwrap_or(&a));
        let _ = std::fs::write(a, json);
    }
}

#[derive(Debug, Clone, Deserialize)]
struct EventoCru {
    quando: String,
    provedor: String,
    id: u32,
    nivel: u32,
}

/// As quedas dos últimos 32 dias (30 de estabilidade e a janela de 48 horas).
#[cfg(windows)]
pub fn ler_quedas() -> Result<Vec<Ocorrencia>, String> {
    let script = "try { $e = Get-WinEvent -LogName System -FilterXPath \
        \"*[System[(Provider[@Name='Microsoft-Windows-Kernel-Power'] and EventID=41) or \
          (Provider[@Name='Microsoft-Windows-WER-SystemErrorReporting'] and EventID=1001) or \
          (Provider[@Name='BugCheck'] and EventID=1001) or \
          (Provider[@Name='Display'] and EventID=4101) or \
          (Provider[@Name='Microsoft-Windows-WHEA-Logger'] and (Level=1 or Level=2))] and \
          System[TimeCreated[timediff(@SystemTime) <= 2764800000]]]\" -MaxEvents 500 -ErrorAction Stop } \
        catch { if ($_.FullyQualifiedErrorId -like 'NoMatchingEventsFound*') { $e = @() } else { throw } }; \
        ConvertTo-Json -Compress -InputObject @($e | ForEach-Object { [ordered]@{ \
          quando = $_.TimeCreated.ToString('s'); provedor = $_.ProviderName; id = [int]$_.Id; nivel = [int]$_.Level } })";
    let saida = super::shell::powershell(script).map_err(|e| format!("registro de eventos: {}", e))?;
    if !saida.success {
        return Err("o registro de eventos do Windows não pôde ser lido".to_string());
    }
    let texto = saida.stdout.trim();
    if texto.is_empty() {
        return Ok(Vec::new());
    }
    let crus: Vec<EventoCru> =
        serde_json::from_str(texto).map_err(|e| format!("registro de eventos em formato inesperado: {}", e))?;
    Ok(crus
        .iter()
        .filter_map(|e| Some(Ocorrencia { quando: epoch_local(&e.quando)?, queda: queda_do_evento(&e.provedor, e.id, e.nivel)? }))
        .collect())
}

/// O que o vigia fez nesta abertura, para o log e para a tela (`estabilidade:aviso`).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(specta::Type))]
#[cfg_attr(test, serde(rename = "RelatoDeEstabilidade"))]
pub struct Relato {
    pub frase: String,
    pub desfeitos: Vec<String>,
    pub falhas: Vec<String>,
}

/// Na abertura: julga as quedas novas e desfaz o que o PC estável não aguentou. Não roda no modo seguro do Otimiza.
#[cfg(windows)]
pub fn conferir(log: &mut crate::modules::changelog::ChangeLog) -> Vec<Relato> {
    let memoria = ler_memoria();
    let quedas = match ler_quedas() {
        Ok(q) => q,
        Err(e) => {
            crate::utils::Logger::warn(&format!("estabilidade: {}", e));
            return Vec::new();
        }
    };
    let aplicados: Vec<Aplicado> = log
        .applied()
        .iter()
        .map(|a| Aplicado { id: a.optimization_id.clone(), nome: a.name.clone(), quando: a.timestamp as i64 })
        .collect();
    let julgamentos = julgar(&quedas, &aplicados, memoria.olhado_ate);
    let mais_nova = quedas.iter().map(|q| q.quando).max().unwrap_or(memoria.olhado_ate);
    let mut relatos = Vec::new();
    for j in julgamentos {
        match j {
            Julgamento::Desfazer { queda, ajustes } => {
                let mut desfeitos = Vec::new();
                let mut falhas = Vec::new();
                for a in &ajustes {
                    match super::WindowsOptimizer::new().revert(&a.id, log) {
                        Ok(r) if r.success => desfeitos.push(a.nome.clone()),
                        Ok(r) => falhas.push(format!("{}: {}", a.nome, r.message)),
                        Err(e) => falhas.push(format!("{}: {}", a.nome, e)),
                    }
                }
                relatos.push(Relato {
                    frase: format!(
                        "Depois de ajustes do Otimiza, {}. O PC estava estável antes, então desfiz o que foi aplicado nas 48 horas antes disso.",
                        queda.queda.frase()
                    ),
                    desfeitos,
                    falhas,
                });
            }
            Julgamento::JaCaiaAntes { queda, ajustes } => {
                relatos.push(Relato {
                    frase: format!(
                        "{} depois de ajustes do Otimiza, mas isso já acontecia neste PC antes deles: não dá para culpar um ajuste. Os ajustes ficaram.",
                        primeira_maiuscula(queda.queda.frase())
                    ),
                    desfeitos: Vec::new(),
                    falhas: Vec::new(),
                });
                let _ = ajustes;
            }
        }
    }
    gravar_memoria(&Memoria { olhado_ate: mais_nova.max(memoria.olhado_ate) });
    relatos
}

fn primeira_maiuscula(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|p| p.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIA: i64 = 24 * 3600;
    const T0: i64 = 1_790_000_000;

    fn ajuste(id: &str, quando: i64) -> Aplicado {
        Aplicado { id: id.into(), nome: id.into(), quando }
    }
    fn queda(quando: i64, q: Queda) -> Ocorrencia {
        Ocorrencia { quando, queda: q }
    }

    #[test]
    fn pc_estavel_que_cai_depois_do_ajuste_desfaz_o_ajuste() {
        let j = julgar(&[queda(T0 + 3600 * 5, Queda::TelaAzul)], &[ajuste("timer", T0)], 0);
        assert!(matches!(&j[..], [Julgamento::Desfazer { ajustes, .. }] if ajustes[0].id == "timer"));
    }

    #[test]
    fn pc_que_ja_caia_antes_nao_culpa_o_ajuste() {
        let quedas = [queda(T0 - 10 * DIA, Queda::Desligamento), queda(T0 + 3600, Queda::Desligamento)];
        let j = julgar(&quedas, &[ajuste("timer", T0)], 0);
        assert!(matches!(&j[..], [Julgamento::JaCaiaAntes { .. }]));
    }

    #[test]
    fn queda_longe_do_ajuste_nao_conta() {
        let j = julgar(&[queda(T0 + 3 * DIA, Queda::TelaAzul)], &[ajuste("timer", T0)], 0);
        assert!(j.is_empty());
    }

    #[test]
    fn queda_antes_do_ajuste_nao_culpa_o_ajuste() {
        let j = julgar(&[queda(T0 - 3600, Queda::TelaAzul)], &[ajuste("timer", T0)], 0);
        assert!(j.is_empty());
    }

    #[test]
    fn queda_ja_olhada_nao_desfaz_de_novo() {
        let j = julgar(&[queda(T0 + 3600, Queda::TelaAzul)], &[ajuste("timer", T0)], T0 + 3600);
        assert!(j.is_empty());
    }

    #[test]
    fn duas_quedas_seguidas_culpam_o_ajuste_uma_vez_so() {
        let quedas = [queda(T0 + 3600, Queda::TelaAzul), queda(T0 + 7200, Queda::Desligamento)];
        let j = julgar(&quedas, &[ajuste("timer", T0)], 0);
        assert_eq!(j.len(), 1);
    }

    #[test]
    fn os_eventos_certos_viram_queda() {
        assert_eq!(queda_do_evento("Microsoft-Windows-Kernel-Power", 41, 1), Some(Queda::Desligamento));
        assert_eq!(queda_do_evento("Microsoft-Windows-WER-SystemErrorReporting", 1001, 2), Some(Queda::TelaAzul));
        assert_eq!(queda_do_evento("Display", 4101, 3), Some(Queda::VideoReiniciou));
        assert_eq!(queda_do_evento("Microsoft-Windows-WHEA-Logger", 18, 2), Some(Queda::Hardware));
        // Erro de hardware corrigido (aviso) não é queda.
        assert_eq!(queda_do_evento("Microsoft-Windows-WHEA-Logger", 19, 3), None);
        assert_eq!(queda_do_evento("Microsoft-Windows-Kernel-Power", 42, 4), None);
    }

    #[test]
    fn a_hora_do_registro_vira_epoch() {
        let a = epoch_local("2026-11-10T20:31:50").unwrap();
        let b = epoch_local("2026-11-10T20:31:51.1234567").unwrap();
        assert_eq!(b - a, 1);
        assert!(epoch_local("ontem").is_none());
    }

    /// Lê as quedas reais desta máquina (manual: `cargo test -- --ignored le_as_quedas_nesta_maquina --nocapture`).
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn le_as_quedas_nesta_maquina() {
        let q = ler_quedas().expect("a consulta ao registro de eventos falhou");
        println!("{} quedas nos últimos 32 dias: {:?}", q.len(), q.iter().take(10).collect::<Vec<_>>());
    }
}
