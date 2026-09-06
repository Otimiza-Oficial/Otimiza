// Saúde do hardware
//
// Existe um caso que nenhum otimizador do mercado detecta e que explica boa
// parte dos "otimizei e não melhorou": o disco está morrendo.
//
// Um SSD no fim da vida ou um HD com setores realocados fica lento de um jeito
// que nenhum ajuste de software conserta. O técnico limpa, otimiza, mede — e o
// PC continua ruim, porque o problema é físico. Sem essa leitura, ele perde a
// tarde e a confiança do cliente.
//
// O mesmo vale para bateria de notebook: abaixo de certo desgaste o Windows
// passa a limitar o processador para o PC não desligar sozinho, e o dono jura
// que "o notebook ficou lento do nada".

use super::shell;
use serde::{Deserialize, Serialize};

pub use super::firmware::{FindingSeverity, FixLocation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthFinding {
    pub id: String,
    pub title: String,
    pub measured: String,
    pub advice: String,
    pub severity: FindingSeverity,
    pub fix_location: FixLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub findings: Vec<HealthFinding>,
    /// Se faltou permissão para ler algo. A interface avisa em vez de deixar
    /// parecer que a checagem foi completa.
    pub needs_admin: bool,
}

fn powershell(script: &str) -> Option<String> {
    let output = shell::powershell(script).ok()?;

    if output.success && !output.stdout.trim().is_empty() {
        Some(output.stdout)
    } else {
        None
    }
}

// ---------------------------------------------------------------------- disco

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawDisk {
    friendly_name: Option<String>,
    media_type: Option<String>,
    health_status: Option<String>,
    size_gb: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawReliability {
    device_id: Option<String>,
    /// Percentual de vida já consumido do SSD. 100 = fim da vida projetada.
    wear: Option<u32>,
    temperature: Option<u32>,
    read_errors_total: Option<u64>,
    write_errors_total: Option<u64>,
    power_on_hours: Option<u64>,
}

/// O que sabemos sobre a saúde do disco desta máquina.
///
/// Os quatro casos existem porque `unwrap_or_default()` juntava dois deles num
/// só. "Perguntei e não há disco" e "não consegui perguntar" viravam a mesma
/// lista vazia — e a tela afirmava, sobre a segunda, a explicação da primeira:
/// *"acontece em máquinas virtuais e em alguns controladores de disco antigos"*,
/// marcada como tudo certo. Inventar a causa de uma leitura que nunca aconteceu
/// é o oposto do que este produto existe para fazer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SituacaoDoDisco {
    /// Os contadores foram lidos. Segue a análise.
    Normal,
    /// Há disco, os contadores exigem elevação e não a temos.
    PrecisaDeAdministrador,
    /// Não deu para ler, e não é falta de permissão — já estamos elevados, ou a
    /// própria enumeração de discos falhou.
    NaoVerificavel,
    /// Perguntamos, o Windows respondeu, e não há disco a reportar. Aqui a
    /// explicação da máquina virtual é legítima.
    SemDadosDeVerdade,
}

/// A regra pura, separada da leitura para poder ser testada sem hardware.
///
/// `None` significa "não consegui perguntar"; `Some(0)`, "perguntei e não há".
pub fn situacao_do_disco(
    discos: Option<usize>,
    contadores: Option<usize>,
    elevado: bool,
) -> SituacaoDoDisco {
    let Some(discos) = discos else {
        return SituacaoDoDisco::NaoVerificavel;
    };

    if discos == 0 {
        return SituacaoDoDisco::SemDadosDeVerdade;
    }

    match contadores {
        Some(n) if n > 0 => SituacaoDoDisco::Normal,
        // Há disco e o contador não veio. Sem elevação isso se resolve; com
        // ela, não sabemos por quê — e não saber precisa ser dito.
        _ if !elevado => SituacaoDoDisco::PrecisaDeAdministrador,
        _ => SituacaoDoDisco::NaoVerificavel,
    }
}

fn discos_lidos() -> Option<Vec<RawDisk>> {
    let script = "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-PhysicalDisk \
                  -ErrorAction SilentlyContinue | Select-Object FriendlyName,MediaType,HealthStatus,\
                  @{n='SizeGb';e={[math]::Round($_.Size/1GB,0)}})";

    // `None` aqui é "não consegui perguntar" — e não pode virar lista vazia,
    // que é a resposta de "perguntei e não há". Ver `situacao_do_disco`.
    powershell(script).and_then(|json| serde_json::from_str(&json).ok())
}

fn confiabilidade() -> Option<Vec<RawReliability>> {
    let script = "ConvertTo-Json -Compress -Depth 3 -InputObject @(Get-PhysicalDisk \
                  -ErrorAction SilentlyContinue | Get-StorageReliabilityCounter \
                  -ErrorAction SilentlyContinue | Select-Object DeviceId,Wear,Temperature,\
                  ReadErrorsTotal,WriteErrorsTotal,PowerOnHours)";

    powershell(script).and_then(|json| serde_json::from_str(&json).ok())
}

/// Traduz o estado do disco relatado pelo Windows.
/// Os valores são em inglês em qualquer idioma do sistema.
pub fn avaliar_estado(health_status: &str) -> Option<(FindingSeverity, &'static str)> {
    match health_status {
        "Healthy" => Some((FindingSeverity::Ok, "O Windows não relata problema neste disco.")),
        "Warning" => Some((
            FindingSeverity::Critical,
            "O Windows já registrou erro neste disco. Faça backup do que importa HOJE — \
             disco em aviso costuma piorar rápido, e nenhum ajuste de software recupera.",
        )),
        "Unhealthy" => Some((
            FindingSeverity::Critical,
            "O Windows considera este disco com falha. Copie seus arquivos agora e troque \
             o disco. Otimização nenhuma resolve, e continuar usando arrisca perder tudo.",
        )),
        _ => None,
    }
}

/// Avalia o desgaste de um SSD.
///
/// `Wear` é a porcentagem da vida projetada já consumida. Exposto para teste
/// porque as faixas decidem o que o cliente vai ouvir sobre trocar peça.
pub fn avaliar_desgaste(wear: u32) -> (FindingSeverity, String) {
    if wear >= 90 {
        (
            FindingSeverity::Critical,
            format!(
                "{}% da vida útil consumida. O SSD está no fim e a velocidade cai muito \
                 nesse estágio. Faça backup e planeje a troca — nenhum software recupera \
                 célula gasta.",
                wear
            ),
        )
    } else if wear >= 70 {
        (
            FindingSeverity::Important,
            format!(
                "{}% da vida útil consumida. Ainda funciona, mas vale começar a planejar a \
                 troca e manter backup em dia.",
                wear
            ),
        )
    } else {
        (
            FindingSeverity::Ok,
            format!("{}% da vida útil consumida. O disco está com folga.", wear),
        )
    }
}

/// O que se sabe sobre os erros acumulados de um disco.
///
/// TRÊS RESPOSTAS, E NÃO DUAS. `Some(0)` é "medi e deu zero"; `None` é "não
/// consegui medir". O código antigo somava com `unwrap_or(0)` e os dois viravam
/// a mesma coisa — um SSD que não publica o contador (ou uma consulta sem
/// administrador) recebia atestado de saúde que ninguém tinha medido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrosDoDisco {
    Nenhum,
    Contados(u64),
    NaoSei,
}

pub fn avaliar_erros(leitura: Option<u64>, gravacao: Option<u64>) -> ErrosDoDisco {
    // Um lado ausente já impede saber o total: somar o que se tem com zero
    // seria inventar a metade que falta.
    let (Some(l), Some(g)) = (leitura, gravacao) else {
        return ErrosDoDisco::NaoSei;
    };

    match l + g {
        0 => ErrosDoDisco::Nenhum,
        total => ErrosDoDisco::Contados(total),
    }
}

/// Constrói os achados de um único disco a partir do contador de
/// confiabilidade já lido.
///
/// Extraída de `analisar_discos` para que a COSTURA entre `avaliar_erros` e o
/// achado que ela produz seja testável sem hardware — sem esta função, um
/// teste só provava que o ajudante puro decidia certo, e nada garantia que
/// `analisar_discos` de fato usasse a decisão dele em vez de, por exemplo,
/// voltar a somar com `unwrap_or(0)`.
fn achados_de_confiabilidade(contador: &RawReliability) -> Vec<HealthFinding> {
    let mut findings = Vec::new();
    let id = contador.device_id.clone().unwrap_or_default();

    if let Some(wear) = contador.wear {
        let (severidade, conselho) = avaliar_desgaste(wear);
        findings.push(HealthFinding {
            id: format!("disk_wear_{}", id),
            title: format!("Desgaste do disco {}", id),
            measured: format!("{}% da vida projetada consumida.", wear),
            advice: conselho,
            severity: severidade,
            fix_location: if severidade == FindingSeverity::Ok {
                FixLocation::None
            } else {
                FixLocation::Hardware
            },
        });
    }

    match avaliar_erros(contador.read_errors_total, contador.write_errors_total) {
        ErrosDoDisco::Contados(erros) => {
            findings.push(HealthFinding {
                id: format!("disk_errors_{}", id),
                title: format!("Erros de leitura ou gravação no disco {}", id),
                measured: format!("{} erros acumulados desde a fabricação.", erros),
                advice: "Erro de leitura faz o Windows tentar de novo, e é isso que aparece \
                         como travada de segundos. Faça backup e considere a troca."
                    .to_string(),
                severity: FindingSeverity::Critical,
                fix_location: FixLocation::Hardware,
            });
        }
        ErrosDoDisco::Nenhum => {}
        // Contador ausente não é o mesmo que contador zerado. Se chegamos aqui
        // é porque `Get-StorageReliabilityCounter` respondeu para este disco —
        // a ausência total já foi interceptada antes, em `analisar_discos`,
        // pelo `contadores.is_empty()` que liga `needs_admin`. O que acontece
        // aqui é mais estreito: este SSD específico não publica
        // ReadErrorsTotal/WriteErrorsTotal, algo comum no mercado. O achado
        // avisa a lacuna em vez de preenchê-la com "está tudo bem".
        ErrosDoDisco::NaoSei => {
            findings.push(HealthFinding {
                id: format!("disk_errors_naosei_{}", id),
                title: format!("Erros de leitura ou gravação no disco {}", id),
                measured: "Não foi possível ler o contador de erros deste disco.".to_string(),
                advice: "Isso não é o mesmo que o disco estar sem erro — apenas não deu \
                         para confirmar por aqui. Pode faltar permissão de administrador, \
                         ou o fabricante simplesmente não publica esse contador."
                    .to_string(),
                severity: FindingSeverity::Ok,
                fix_location: FixLocation::None,
            });
        }
    }

    // Acima de 60 °C um SSD começa a reduzir a própria velocidade para
    // não se danificar.
    if let Some(temp) = contador.temperature.filter(|t| *t >= 60) {
        findings.push(HealthFinding {
            id: format!("disk_temp_{}", id),
            title: format!("Disco {} quente", id),
            measured: format!("{} °C.", temp),
            advice: "Acima de 60 °C o disco reduz a própria velocidade para se proteger. \
                     Verifique a ventilação do gabinete ou do notebook."
                .to_string(),
            severity: FindingSeverity::Important,
            fix_location: FixLocation::Hardware,
        });
    }

    if let Some(horas) = contador.power_on_hours.filter(|h| *h > 0) {
        findings.push(HealthFinding {
            id: format!("disk_hours_{}", id),
            title: format!("Tempo de uso do disco {}", id),
            measured: format!("{} horas ligado ({:.1} anos de uso contínuo).", horas, horas as f64 / 8760.0),
            advice: String::new(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::None,
        });
    }

    findings
}

fn analisar_discos(findings: &mut Vec<HealthFinding>) -> bool {
    let lidos = discos_lidos();
    // `None` continua distinguível de lista vazia pelo `quantos`: a fatia serve
    // só para percorrer o que veio, e a decisão usa a contagem original.
    let quantos = lidos.as_ref().map(Vec::len);
    let lista: &[RawDisk] = lidos.as_deref().unwrap_or(&[]);

    for (indice, disco) in lista.iter().enumerate() {
        let nome = disco
            .friendly_name
            .clone()
            .unwrap_or_else(|| format!("Disco {}", indice));
        let tipo = disco.media_type.clone().unwrap_or_default();
        let tamanho = disco.size_gb.unwrap_or(0.0);

        if let Some((severidade, conselho)) = disco
            .health_status
            .as_deref()
            .and_then(avaliar_estado)
        {
            findings.push(HealthFinding {
                id: format!("disk_status_{}", indice),
                title: format!("Disco: {}", nome),
                measured: format!("{} de {:.0} GB, estado relatado pelo Windows.", tipo, tamanho),
                advice: conselho.to_string(),
                severity: severidade,
                fix_location: if severidade == FindingSeverity::Ok {
                    FixLocation::None
                } else {
                    FixLocation::Hardware
                },
            });
        }
    }

    // O contador de confiabilidade — desgaste, erros, horas ligadas — exige
    // elevação. Sem ela, não temos como saber; e não saber precisa ser dito.
    let contadores = confiabilidade();

    match situacao_do_disco(
        quantos,
        contadores.as_ref().map(Vec::len),
        super::registry::is_elevated(),
    ) {
        SituacaoDoDisco::Normal => {
            for contador in contadores.iter().flatten() {
                findings.extend(achados_de_confiabilidade(contador));
            }
            false
        }

        // O caminho que já existia: a tela pede administrador.
        SituacaoDoDisco::PrecisaDeAdministrador => true,

        // O caminho que faltava. Antes daqui a função voltava calada e a tela
        // ficava tranquilizadora sobre um disco que ninguém mediu.
        SituacaoDoDisco::NaoVerificavel => {
            findings.push(HealthFinding {
                id: "disk_nao_verificado".to_string(),
                title: "Saúde do disco não verificada".to_string(),
                measured: "O Windows não respondeu à consulta de desgaste e erros do disco, \
                           mesmo com o Otimiza aberto como administrador."
                    .to_string(),
                advice: "Não sabemos o estado deste disco — isto não é o mesmo que dizer que \
                         ele está bem. Costuma acontecer com controladores RAID e com adaptadores \
                         USB, que escondem os contadores do disco do Windows."
                    .to_string(),
                severity: FindingSeverity::Important,
                fix_location: FixLocation::None,
            });
            false
        }

        // Perguntamos e o Windows disse que não há disco a reportar. O achado
        // genérico de "sem dados" continua servindo, e é ele que explica o caso
        // da máquina virtual — agora só onde a explicação é verdadeira.
        SituacaoDoDisco::SemDadosDeVerdade => false,
    }
}

// -------------------------------------------------------------------- bateria

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct RawBattery {
    designed: Option<u64>,
    full_charged: Option<u64>,
}

fn bateria() -> Option<RawBattery> {
    let script = "$d = (Get-CimInstance -Namespace root\\wmi -ClassName BatteryStaticData \
                  -ErrorAction SilentlyContinue | Select-Object -First 1).DesignedCapacity; \
                  $f = (Get-CimInstance -Namespace root\\wmi -ClassName BatteryFullChargedCapacity \
                  -ErrorAction SilentlyContinue | Select-Object -First 1).FullChargedCapacity; \
                  if ($d -and $f) { ConvertTo-Json -Compress -InputObject ([ordered]@{ \
                    Designed = $d; FullCharged = $f }) }";

    powershell(script).and_then(|json| serde_json::from_str(&json).ok())
}

/// Saúde da bateria em porcentagem da capacidade de fábrica.
pub fn avaliar_bateria(designed: u64, full_charged: u64) -> Option<(f64, FindingSeverity, String)> {
    if designed == 0 {
        return None;
    }

    let saude = full_charged as f64 / designed as f64 * 100.0;

    let (severidade, conselho) = if saude < 50.0 {
        (
            FindingSeverity::Critical,
            "Abaixo de metade da capacidade de fábrica. Nesse estágio o Windows costuma \
             limitar o processador para o notebook não desligar sozinho — e é isso que o \
             dono sente como \"ficou lento do nada\". Trocar a bateria devolve o desempenho, \
             e nenhum ajuste de software substitui."
                .to_string(),
        )
    } else if saude < 70.0 {
        (
            FindingSeverity::Important,
            "Desgaste já perceptível na autonomia. Ainda não costuma limitar o processador, \
             mas vale acompanhar."
                .to_string(),
        )
    } else {
        (
            FindingSeverity::Ok,
            "Bateria em boa condição para a idade.".to_string(),
        )
    };

    Some((saude, severidade, conselho))
}

fn analisar_bateria(findings: &mut Vec<HealthFinding>) {
    // Sem bateria é desktop: não há o que avaliar, e inventar um achado seria
    // encher a tela de informação que não existe.
    let Some(b) = bateria() else { return };

    let (Some(designed), Some(full)) = (b.designed, b.full_charged) else {
        return;
    };

    if let Some((saude, severidade, conselho)) = avaliar_bateria(designed, full) {
        findings.push(HealthFinding {
            id: "battery_health".to_string(),
            title: "Saúde da bateria".to_string(),
            measured: format!(
                "{:.0}% da capacidade de fábrica ({} de {} mWh).",
                saude, full, designed
            ),
            advice: conselho,
            severity: severidade,
            fix_location: if severidade == FindingSeverity::Ok {
                FixLocation::None
            } else {
                FixLocation::Hardware
            },
        });
    }
}

/// Análise completa de saúde do hardware.
pub fn analyze() -> HealthReport {
    let mut findings = Vec::new();

    let needs_admin = analisar_discos(&mut findings);
    analisar_bateria(&mut findings);

    if findings.is_empty() && !needs_admin {
        findings.push(HealthFinding {
            id: "no_data".to_string(),
            title: "Sem dados de saúde disponíveis".to_string(),
            measured: "O Windows não expôs informação de saúde para este hardware.".to_string(),
            advice: "Acontece em máquinas virtuais e em alguns controladores de disco antigos."
                .to_string(),
            severity: FindingSeverity::Ok,
            fix_location: FixLocation::None,
        });
    }

    findings.sort_by_key(|f| match f.severity {
        FindingSeverity::Critical => 0,
        FindingSeverity::Important => 1,
        FindingSeverity::Ok => 2,
    });

    HealthReport {
        findings,
        needs_admin,
    }
}

#[cfg(test)]
mod tests_1_7 {
    use super::*;

    #[test]
    fn contador_ausente_sem_elevacao_pede_administrador() {
        // Comportamento que já existia e continua valendo.
        assert_eq!(
            situacao_do_disco(Some(1), Some(0), false),
            SituacaoDoDisco::PrecisaDeAdministrador
        );
    }

    #[test]
    fn contador_ausente_mesmo_elevado_nao_pode_virar_silencio() {
        // O DEFEITO. `faltou_permissao = !is_elevated()` dava `false` quando o
        // programa ESTAVA elevado, e a função voltava calada: nem achado de
        // desgaste, nem aviso. A tela ficava tranquilizadora sobre um disco que
        // ninguém conseguiu medir.
        assert_eq!(
            situacao_do_disco(Some(1), Some(0), true),
            SituacaoDoDisco::NaoVerificavel
        );
    }

    #[test]
    fn consulta_que_falhou_nao_e_maquina_sem_disco() {
        // A raiz: `unwrap_or_default()` juntava "perguntei e não há disco" com
        // "não consegui perguntar". Com os dois iguais, a tela afirmava
        // "acontece em máquinas virtuais e controladores antigos" — inventando
        // a causa de uma leitura que nunca aconteceu.
        assert_eq!(
            situacao_do_disco(None, None, true),
            SituacaoDoDisco::NaoVerificavel
        );
    }

    #[test]
    fn maquina_que_realmente_nao_reporta_disco_continua_sendo_isso() {
        // Perguntamos, o Windows respondeu, e não havia disco para reportar.
        // Aqui a explicação da máquina virtual é legítima.
        assert_eq!(
            situacao_do_disco(Some(0), Some(0), true),
            SituacaoDoDisco::SemDadosDeVerdade
        );
    }

    #[test]
    fn com_contador_lido_a_analise_segue_normal() {
        assert_eq!(
            situacao_do_disco(Some(2), Some(2), true),
            SituacaoDoDisco::Normal
        );
        assert_eq!(
            situacao_do_disco(Some(2), Some(2), false),
            SituacaoDoDisco::Normal
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disco_em_aviso_manda_fazer_backup() {
        // O achado mais importante que este módulo pode dar. A mensagem precisa
        // mandar salvar os arquivos, não sugerir otimização.
        let (severidade, conselho) = avaliar_estado("Warning").unwrap();

        assert_eq!(severidade, FindingSeverity::Critical);
        assert!(conselho.to_lowercase().contains("backup"));
    }

    #[test]
    fn disco_com_falha_manda_trocar() {
        let (severidade, conselho) = avaliar_estado("Unhealthy").unwrap();
        assert_eq!(severidade, FindingSeverity::Critical);
        assert!(conselho.to_lowercase().contains("troque"));
    }

    #[test]
    fn disco_saudavel_nao_vira_alarme() {
        let (severidade, _) = avaliar_estado("Healthy").unwrap();
        assert_eq!(severidade, FindingSeverity::Ok);
    }

    #[test]
    fn estado_desconhecido_nao_e_inventado() {
        assert!(avaliar_estado("").is_none());
        assert!(avaliar_estado("Sei la").is_none());
    }

    #[test]
    fn desgaste_de_ssd_tem_tres_faixas() {
        assert_eq!(avaliar_desgaste(10).0, FindingSeverity::Ok);
        assert_eq!(avaliar_desgaste(75).0, FindingSeverity::Important);
        assert_eq!(avaliar_desgaste(95).0, FindingSeverity::Critical);

        // No fim da vida, a mensagem tem que ser sobre trocar, não sobre ajustar.
        assert!(avaliar_desgaste(95).1.to_lowercase().contains("backup"));
    }

    #[test]
    fn bateria_muito_gasta_explica_a_lentidao() {
        let (saude, severidade, conselho) = avaliar_bateria(50000, 20000).unwrap();

        assert!((saude - 40.0).abs() < 0.1);
        assert_eq!(severidade, FindingSeverity::Critical);
        // A ligação entre bateria gasta e PC lento é o que o dono não sabe.
        assert!(conselho.contains("limitar o processador"));
    }

    #[test]
    fn bateria_boa_nao_alarma() {
        let (saude, severidade, _) = avaliar_bateria(50000, 46000).unwrap();
        assert!(saude > 90.0);
        assert_eq!(severidade, FindingSeverity::Ok);
    }

    #[test]
    fn capacidade_de_fabrica_zerada_nao_vira_divisao_por_zero() {
        assert!(avaliar_bateria(0, 1000).is_none());
    }

    #[test]
    fn contador_de_erro_ausente_nao_vira_disco_sem_erro() {
        // Muitos SSDs não publicam o contador, e a consulta falha sem
        // administrador. Nos dois casos o valor chegava como zero e NENHUM achado
        // era emitido — o disco recebia atestado de saúde que ninguém mediu.
        let sem_leitura = avaliar_erros(None, None);
        assert!(
            matches!(sem_leitura, ErrosDoDisco::NaoSei),
            "leitura ausente virou {:?}",
            sem_leitura
        );

        assert_eq!(avaliar_erros(Some(0), Some(0)), ErrosDoDisco::Nenhum);
        assert_eq!(avaliar_erros(Some(3), Some(2)), ErrosDoDisco::Contados(5));

        // Um lado ausente ainda é não saber o total.
        assert!(matches!(avaliar_erros(Some(3), None), ErrosDoDisco::NaoSei));
    }

    #[test]
    fn contador_sem_leitura_de_erro_gera_o_achado_naosei_e_nao_o_critico() {
        // Este teste é a costura que faltava: `avaliar_erros` sozinho não prova
        // que `analisar_discos` de fato o usa. Se alguém reverter a chamada de
        // volta para `unwrap_or(0) + unwrap_or(0)`, este teste falha — nenhum
        // achado de erro nasce (0 não é > 0), e o `assert` abaixo não encontra
        // nem o achado NaoSei nem o Crítico.
        let contador = RawReliability {
            device_id: Some("0".to_string()),
            wear: None,
            temperature: None,
            read_errors_total: None,
            write_errors_total: None,
            power_on_hours: None,
        };

        let achados = achados_de_confiabilidade(&contador);
        let achado = achados
            .iter()
            .find(|a| a.id == "disk_errors_naosei_0")
            .expect("esperava achado disk_errors_naosei_0");

        assert_eq!(achado.severity, FindingSeverity::Ok);
        assert_eq!(achado.fix_location, FixLocation::None);
        assert!(achado.measured.to_lowercase().contains("não foi possível"));

        // E nenhum achado `disk_errors_0` (o crítico) pode coexistir com o
        // NaoSei — são mutuamente exclusivos pela própria definição de
        // `ErrosDoDisco`.
        assert!(!achados.iter().any(|a| a.id == "disk_errors_0"));
    }

    #[test]
    fn contador_com_erros_lidos_ainda_gera_o_achado_critico() {
        // O espelho do teste acima: números de verdade continuam produzindo o
        // achado Crítico de sempre — a extração da função não pode ter mudado
        // esse caminho.
        let contador = RawReliability {
            device_id: Some("0".to_string()),
            wear: None,
            temperature: None,
            read_errors_total: Some(3),
            write_errors_total: Some(2),
            power_on_hours: None,
        };

        let achados = achados_de_confiabilidade(&contador);
        let achado = achados
            .iter()
            .find(|a| a.id == "disk_errors_0")
            .expect("esperava achado disk_errors_0");

        assert_eq!(achado.severity, FindingSeverity::Critical);
        assert!(achado.measured.contains('5'));
        assert!(!achados.iter().any(|a| a.id == "disk_errors_naosei_0"));
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();
        println!("precisa de administrador: {}", r.needs_admin);

        for f in &r.findings {
            println!("  [{:?}] {} — {}", f.severity, f.title, f.measured);
        }

        // Problemas antes do que está certo.
        let ordem: Vec<u8> = r
            .findings
            .iter()
            .map(|f| match f.severity {
                FindingSeverity::Critical => 0,
                FindingSeverity::Important => 1,
                FindingSeverity::Ok => 2,
            })
            .collect();
        assert!(ordem.windows(2).all(|p| p[0] <= p[1]));

        // Ou há achados, ou o relatório diz que faltou permissão. Silêncio não.
        assert!(!r.findings.is_empty() || r.needs_admin);
    }
}
