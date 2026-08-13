// O que o Windows já registrou
//
// Este é o módulo com a evidência mais forte do produto inteiro, e o mais
// barato de construir: o Windows já anotou tudo, e ninguém nunca leu.
//
// Quando a memória acaba, o Windows grava o evento 2004 do
// Resource-Exhaustion-Detector — com o commit do sistema, a memória física, e
// o NOME e o tamanho dos três processos que mais seguravam memória naquele
// instante. Quando um programa para de responder, grava o evento 1002 com o
// nome do executável e a hora.
//
// É a diferença entre o produto dizer "você precisa de mais memória" e dizer
// "em 16/07 às 21:22 o Windows registrou falta de memória nesta máquina;
// claude.exe segurava 10,5 GB". A segunda frase o cliente confere sozinho no
// Visualizador de Eventos, e é por isso que ela vale.
//
// ONDE O DADO MORA, E POR QUE ISSO IMPORTA
//
// O 2004 NÃO usa `EventData` como quase todo evento do Windows: usa `UserData`,
// com um bloco `MemoryExhaustionInfo` em namespace próprio. Ler `EventData`
// nele devolve vazio — foi a primeira coisa que tentei.
//
// E, como em `boot.rs` e `thermal.rs`, nada aqui lê a mensagem renderizada do
// evento: ela é traduzida pelo idioma do Windows. Os nomes dos elementos XML
// (`SystemCommitCharge`, `CommitCharge`, `AppName`) são fixos em inglês em
// qualquer instalação, e é neles que este módulo se apoia.

use super::achados::{FindingSeverity, FixLocation};
use super::shell;
use serde::{Deserialize, Serialize};

/// Um processo que estava segurando memória quando o Windows desistiu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Culpado {
    pub nome: String,
    pub gb: f64,
}

/// Um registro de "a memória acabou", feito pelo próprio Windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Esgotamento {
    /// Quando, em formato ordenável.
    pub quando: String,
    pub commit_usado_gb: f64,
    pub commit_limite_gb: f64,
    pub ram_fisica_gb: f64,
    /// Os maiores consumidores no instante do esgotamento, do maior para o
    /// menor. O Windows guarda seis lugares e costuma preencher três.
    pub culpados: Vec<Culpado>,
}

/// Um programa que parou de responder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Travamento {
    pub quando: String,
    pub programa: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsgotamentoFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsgotamentoReport {
    pub esgotamentos: Vec<Esgotamento>,
    pub travamentos: Vec<Travamento>,
    pub dias_observados: u32,
    pub findings: Vec<EsgotamentoFinding>,
    /// Preenchido quando o log não pôde ser lido. Vira lacuna visível na tela,
    /// nunca silêncio — silêncio aqui seria indistinguível de "nunca aconteceu".
    pub erro: Option<String>,
}

const BYTES_EM_GB: f64 = 1_073_741_824.0;
const MS_POR_DIA: u64 = 86_400_000;

/// Quantos dias de histórico o produto olha por padrão.
///
/// Um mês é o suficiente para pegar a rotina sem trazer de volta um episódio
/// isolado de um ano atrás e vendê-lo como problema atual.
pub const DIAS_PADRAO: u32 = 30;

// ------------------------------------------------------------------- leitura

#[derive(Debug, Deserialize)]
struct RawCulpado {
    nome: Option<String>,
    bytes: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawEsgotamento {
    when: Option<String>,
    commit: Option<f64>,
    limite: Option<f64>,
    fisica: Option<f64>,
    processos: Option<Vec<RawCulpado>>,
}

#[derive(Debug, Deserialize)]
struct RawTravamento {
    when: Option<String>,
    programa: Option<String>,
}

fn gb(bytes: Option<f64>) -> f64 {
    (bytes.unwrap_or(0.0) / BYTES_EM_GB * 10.0).round() / 10.0
}

/// Os registros de memória esgotada, direto do log do Windows.
///
/// O XPath usa `local-name()` porque o bloco vem em namespace próprio: sem
/// isso, todo `SelectSingleNode` volta nulo e o módulo relataria "nunca
/// aconteceu" numa máquina que esgotou memória três vezes na mesma noite.
pub fn esgotamentos(dias: u32) -> Result<Vec<Esgotamento>, String> {
    let script = format!(
        "$e = Get-WinEvent -LogName System -FilterXPath \
           \"*[System[Provider[@Name='Microsoft-Windows-Resource-Exhaustion-Detector'] \
             and EventID=2004 and TimeCreated[timediff(@SystemTime) <= {}]]]\" \
           -MaxEvents 30 -ErrorAction Stop; \
         ConvertTo-Json -Compress -Depth 5 -InputObject @($e | ForEach-Object {{ \
           $x = [xml]$_.ToXml(); \
           $texto = {{ param($no, $nome) \
             $f = $no.SelectSingleNode(\"*[local-name()='$nome']\"); \
             if ($f) {{ [double]$f.InnerText }} else {{ 0 }} }}; \
           $si = $x.SelectSingleNode(\"//*[local-name()='SystemInfo']\"); \
           [ordered]@{{ \
             when      = $_.TimeCreated.ToString('s'); \
             commit    = (& $texto $si 'SystemCommitCharge'); \
             limite    = (& $texto $si 'SystemCommitLimit'); \
             fisica    = (& $texto $si 'PhysicalMemorySize'); \
             processos = @($x.SelectNodes(\"//*[local-name()='ProcessInfo']/*\") | ForEach-Object {{ \
                 [ordered]@{{ nome  = $_.SelectSingleNode(\"*[local-name()='Name']\").InnerText; \
                              bytes = (& $texto $_ 'CommitCharge') }} }}) }} }})",
        dias as u64 * MS_POR_DIA
    );

    let saida = shell::powershell(&script)
        .map_err(|e| format!("Não foi possível ler o registro de eventos: {}", e))?;

    if !saida.success {
        return Err(
            "O registro de eventos do Windows não pôde ser lido nesta máquina.".to_string(),
        );
    }

    // Sem nenhum evento, o PowerShell devolve vazio. Isso é boa notícia, e não
    // erro: significa que a memória não acabou no período.
    if saida.stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    let brutos: Vec<RawEsgotamento> = serde_json::from_str(saida.stdout.trim())
        .map_err(|e| format!("Registro de eventos em formato inesperado: {}", e))?;

    Ok(brutos
        .into_iter()
        .map(|b| {
            let mut culpados: Vec<Culpado> = b
                .processos
                .unwrap_or_default()
                .into_iter()
                // O Windows reserva seis lugares e preenche os que couberem; os
                // vazios vêm com nome em branco e zero byte.
                .filter_map(|p| {
                    let nome = p.nome.unwrap_or_default().trim().to_string();
                    let tamanho = gb(p.bytes);

                    if nome.is_empty() || tamanho <= 0.0 {
                        None
                    } else {
                        Some(Culpado { nome, gb: tamanho })
                    }
                })
                .collect();

            culpados.sort_by(|a, b| b.gb.total_cmp(&a.gb));

            Esgotamento {
                quando: b.when.unwrap_or_default(),
                commit_usado_gb: gb(b.commit),
                commit_limite_gb: gb(b.limite),
                ram_fisica_gb: gb(b.fisica),
                culpados,
            }
        })
        .collect())
}

/// Programas que pararam de responder.
///
/// O filtro por provedor é obrigatório: o identificador 1002 é reaproveitado
/// por outros componentes do Windows (o Winlogon, por exemplo), e sem ele a
/// lista viria com eventos que não têm nada a ver com programa travado.
pub fn travamentos(dias: u32) -> Result<Vec<Travamento>, String> {
    let script = format!(
        "$e = Get-WinEvent -LogName Application -FilterXPath \
           \"*[System[Provider[@Name='Application Hang'] and EventID=1002 \
             and TimeCreated[timediff(@SystemTime) <= {}]]]\" \
           -MaxEvents 40 -ErrorAction Stop; \
         ConvertTo-Json -Compress -Depth 3 -InputObject @($e | ForEach-Object {{ \
           $d = ([xml]$_.ToXml()).Event.EventData.Data; \
           [ordered]@{{ \
             when     = $_.TimeCreated.ToString('s'); \
             programa = ($d | Where-Object {{ $_.Name -eq 'AppName' }}).'#text' }} }})",
        dias as u64 * MS_POR_DIA
    );

    let saida = shell::powershell(&script)
        .map_err(|e| format!("Não foi possível ler o registro de eventos: {}", e))?;

    if !saida.success {
        return Err("O registro de aplicativos do Windows não pôde ser lido.".to_string());
    }

    if saida.stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    let brutos: Vec<RawTravamento> = serde_json::from_str(saida.stdout.trim())
        .map_err(|e| format!("Registro de eventos em formato inesperado: {}", e))?;

    Ok(brutos
        .into_iter()
        .filter_map(|b| {
            let programa = b.programa.unwrap_or_default().trim().to_string();

            if programa.is_empty() {
                None
            } else {
                Some(Travamento {
                    quando: b.when.unwrap_or_default(),
                    programa,
                })
            }
        })
        .collect())
}

// --------------------------------------------------------------- diagnóstico

/// Só a data, sem o horário com segundos, para caber numa frase.
fn dia_e_hora(iso: &str) -> String {
    // O formato é "2026-07-16T21:21:59". Vira "16/07 às 21:21".
    let (data, hora) = match iso.split_once('T') {
        Some(par) => par,
        None => return iso.to_string(),
    };

    let partes: Vec<&str> = data.split('-').collect();
    let hora_curta = hora.get(0..5).unwrap_or(hora);

    if partes.len() == 3 {
        format!("{}/{} às {}", partes[2], partes[1], hora_curta)
    } else {
        format!("{} às {}", data, hora_curta)
    }
}

/// Regras de diagnóstico, puras, testáveis sem tocar em log de evento.
pub fn diagnosticar(
    esgotamentos: &[Esgotamento],
    travamentos: &[Travamento],
    dias: u32,
) -> Vec<EsgotamentoFinding> {
    let mut findings = Vec::new();

    if let Some(ultimo) = esgotamentos.first() {
        // O próprio Windows declarou o esgotamento. Não existe falso positivo
        // aqui, e por isso este achado não precisa de corroboração de ninguém:
        // é a única evidência do produto que dispensa interpretação.
        let culpados = if ultimo.culpados.is_empty() {
            String::new()
        } else {
            let lista: Vec<String> = ultimo
                .culpados
                .iter()
                .take(3)
                .map(|c| format!("{} com {:.1} GB", c.nome, c.gb))
                .collect();

            format!(" Segurando memória no momento: {}.", lista.join(", "))
        };

        findings.push(EsgotamentoFinding {
            id: "windows_registrou_esgotamento".to_string(),
            title: "O Windows já registrou falta de memória nesta máquina".to_string(),
            measured: format!(
                "{} nos últimos {} dias — o mais recente em {}, com {:.1} GB \
                 comprometidos para {:.1} GB de memória física.{}",
                if esgotamentos.len() == 1 {
                    "1 registro".to_string()
                } else {
                    format!("{} registros", esgotamentos.len())
                },
                dias,
                dia_e_hora(&ultimo.quando),
                ultimo.commit_usado_gb,
                ultimo.ram_fisica_gb,
                culpados
            ),
            advice: "Isto não é uma estimativa nossa: é o próprio Windows dizendo que \
                     ficou sem memória. Você pode conferir no Visualizador de Eventos, \
                     no registro do sistema, evento 2004. Quando isso acontece, tudo \
                     para junto esperando o disco — e nenhum ajuste de software cria \
                     memória. Rodar menos coisa ao mesmo tempo alivia; acrescentar um \
                     pente resolve."
                .to_string(),
            severity: FindingSeverity::Critical,
            fix_location: FixLocation::Hardware,
        });
    }

    if !travamentos.is_empty() {
        // Agrupa por programa: dez travamentos do mesmo jogo é uma informação,
        // dez linhas iguais na tela é ruído.
        let mut por_programa: Vec<(String, usize, String)> = Vec::new();

        for t in travamentos {
            match por_programa.iter_mut().find(|(nome, _, _)| *nome == t.programa) {
                Some((_, quantos, _)) => *quantos += 1,
                None => por_programa.push((t.programa.clone(), 1, t.quando.clone())),
            }
        }

        por_programa.sort_by(|a, b| b.1.cmp(&a.1));

        let lista: Vec<String> = por_programa
            .iter()
            .take(3)
            .map(|(nome, quantos, quando)| {
                if *quantos == 1 {
                    format!("{} em {}", nome, dia_e_hora(quando))
                } else {
                    format!("{} ({}x, último em {})", nome, quantos, dia_e_hora(quando))
                }
            })
            .collect();

        findings.push(EsgotamentoFinding {
            id: "programas_travaram".to_string(),
            title: "Programas que pararam de responder".to_string(),
            measured: format!("Nos últimos {} dias: {}.", dias, lista.join("; ")),
            // Um programa parar de responder tem muitas causas, e a honestidade
            // aqui é não escolher uma. O que este achado faz é dar a data, para
            // que ela seja comparada com a do esgotamento acima.
            advice: "Travar sozinho tem várias causas possíveis — falta de memória, \
                     disco lento, defeito do próprio programa. Se a data bater com a \
                     de um registro de falta de memória, a causa provável é essa."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::None,
        });
    }

    findings
}

/// Lê os dois logs e diagnostica.
pub fn analyze() -> EsgotamentoReport {
    analyze_dias(DIAS_PADRAO)
}

pub fn analyze_dias(dias: u32) -> EsgotamentoReport {
    let (lista_esgotamentos, erro_esgotamento) = match esgotamentos(dias) {
        Ok(v) => (v, None),
        Err(e) => (Vec::new(), Some(e)),
    };

    // O log de aplicativos falhar não pode apagar o de sistema, que é o
    // importante. Cada um responde por si.
    let lista_travamentos = travamentos(dias).unwrap_or_default();

    let findings = diagnosticar(&lista_esgotamentos, &lista_travamentos, dias);

    EsgotamentoReport {
        esgotamentos: lista_esgotamentos,
        travamentos: lista_travamentos,
        dias_observados: dias,
        findings,
        erro: erro_esgotamento,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esgotamento_exemplo() -> Esgotamento {
        // Os números reais da máquina do dono, evento de 16/07/2026.
        Esgotamento {
            quando: "2026-07-16T21:21:59".to_string(),
            commit_usado_gb: 31.7,
            commit_limite_gb: 31.9,
            ram_fisica_gb: 7.9,
            culpados: vec![
                Culpado { nome: "claude.exe".to_string(), gb: 9.8 },
                Culpado { nome: "HuntinBuddies-Win64-Shipping.exe".to_string(), gb: 4.4 },
                Culpado { nome: "Arc.exe".to_string(), gb: 2.2 },
            ],
        }
    }

    #[test]
    fn esgotamento_registrado_pelo_windows_e_critico_e_nomeia_os_culpados() {
        // O que dá força a este achado é ele ser conferível: o cliente abre o
        // Visualizador de Eventos e vê a mesma coisa.
        let f = diagnosticar(&[esgotamento_exemplo()], &[], 30);
        let achado = &f[0];

        assert_eq!(achado.id, "windows_registrou_esgotamento");
        assert_eq!(achado.severity, FindingSeverity::Critical);
        assert_eq!(achado.fix_location, FixLocation::Hardware);
        assert!(achado.measured.contains("claude.exe com 9.8 GB"));
        assert!(achado.measured.contains("16/07 às 21:21"));
        assert!(achado.advice.contains("evento 2004"));
    }

    #[test]
    fn maquina_sem_registro_nao_ganha_achado_nenhum() {
        // Não achar nada é o resultado esperado numa máquina saudável, e não
        // pode virar aviso genérico.
        assert!(diagnosticar(&[], &[], 30).is_empty());
    }

    #[test]
    fn travamentos_do_mesmo_programa_viram_uma_linha_com_a_contagem() {
        let travamentos = vec![
            Travamento {
                quando: "2026-08-11T14:13:46".to_string(),
                programa: "FiveM_b3258_GTAProcess.exe".to_string(),
            },
            Travamento {
                quando: "2026-08-09T20:02:10".to_string(),
                programa: "FiveM_b3258_GTAProcess.exe".to_string(),
            },
            Travamento {
                quando: "2026-07-16T21:26:00".to_string(),
                programa: "Discord.exe".to_string(),
            },
        ];

        let f = diagnosticar(&[], &travamentos, 30);
        let achado = f.iter().find(|f| f.id == "programas_travaram").unwrap();

        assert!(achado.measured.contains("FiveM_b3258_GTAProcess.exe (2x"));
        assert!(achado.measured.contains("Discord.exe em 16/07"));
    }

    #[test]
    fn travamento_nao_afirma_causa_que_nao_pode_provar() {
        // Um programa travar tem muitas causas. Escolher uma sem prova seria
        // exatamente o que este produto critica nos outros.
        let f = diagnosticar(
            &[],
            &[Travamento {
                quando: "2026-08-11T14:13:46".to_string(),
                programa: "FiveM_b3258_GTAProcess.exe".to_string(),
            }],
            30,
        );

        assert_eq!(f[0].severity, FindingSeverity::Important);
        assert!(f[0].advice.contains("várias causas possíveis"));
    }

    #[test]
    fn data_vira_texto_legivel() {
        assert_eq!(dia_e_hora("2026-07-16T21:21:59"), "16/07 às 21:21");
        // Formato inesperado não pode derrubar nem inventar data.
        assert_eq!(dia_e_hora("sem data"), "sem data");
    }

    #[test]
    fn le_o_registro_desta_maquina() {
        let r = analyze();

        println!("\n  {} dias observados", r.dias_observados);
        if let Some(e) = &r.erro {
            println!("  não deu para ler: {}", e);
        }
        for e in r.esgotamentos.iter().take(3) {
            let quem: Vec<String> = e
                .culpados
                .iter()
                .map(|c| format!("{} {:.1} GB", c.nome, c.gb))
                .collect();
            println!(
                "  esgotamento em {} · {:.1} GB comprometidos · {}",
                e.quando,
                e.commit_usado_gb,
                quem.join(", ")
            );
        }
        for t in r.travamentos.iter().take(5) {
            println!("  travou: {} em {}", t.programa, t.quando);
        }
        for f in &r.findings {
            println!("  [{:?}] {}", f.severity, f.measured);
        }
        println!();

        // Não dá para exigir achado: uma máquina saudável não tem nenhum. O que
        // dá para exigir é que a leitura não devolva erro e silêncio ao mesmo
        // tempo — isso seria o produto não sabendo se sabe.
        assert!(
            r.erro.is_none() || r.esgotamentos.is_empty(),
            "não pode relatar erro e dado ao mesmo tempo"
        );
    }
}
