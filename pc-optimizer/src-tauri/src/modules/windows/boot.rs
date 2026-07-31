// Tempo de inicialização
//
// O cliente não percebe 5% de FPS. Percebe muito bem "meu PC ligava em dois
// minutos e agora liga em quarenta segundos". E o Windows mede isso sozinho,
// desde sempre, guardando no log de eventos o tempo total do boot e o nome de
// cada programa que atrasou, com os milissegundos de cada um.
//
// Nenhum otimizador do mercado mostra isso. É o número mais convincente que
// este produto pode dar, porque não é uma promessa: é o próprio Windows
// dizendo "o Discord custou 142 segundos da sua inicialização".
//
// TRÊS COISAS QUE ESTE MÓDULO PRECISA ACERTAR
//
// 1. Ler pelos campos estruturados, nunca pela mensagem. A `Message` do evento
//    vem traduzida ("Duração da Inicialização"); os nomes dentro de
//    `EventData/Data[@Name]` vêm do manifesto do provedor e são fixos em inglês
//    em qualquer idioma do Windows. Este projeto já quebrou uma vez por ler
//    texto localizado, e não vai quebrar de novo pelo mesmo motivo.
//
// 2. Casar por NOME do campo, não por posição. O evento 100 tem 44 campos na
//    versão 2 e bem menos nas versões antigas do Windows. Índice fixo quebra
//    em silêncio numa máquina mais velha, que é exatamente o público daqui.
//
// 3. Admitir quando não há dado. O log exige administrador, e mesmo elevado ele
//    pode estar vazio — em várias máquinas o coletor de desempenho de boot
//    simplesmente para de gravar. Ausência de dado não é "seu boot está ótimo".

use super::{registry, shell};
use serde::{Deserialize, Serialize};

const LOG_DESEMPENHO: &str = "Microsoft-Windows-Diagnostics-Performance/Operational";

/// Uma inicialização medida pelo Windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootMeasurement {
    /// Quando esta inicialização aconteceu, como o Windows registrou.
    pub when: String,
    /// Tempo total até a máquina estar realmente utilizável.
    pub total_ms: u64,
    /// Até a área de trabalho aparecer.
    pub main_path_ms: u64,
    /// Depois da área de trabalho aparecer: os programas de inicialização
    /// brigando pelo disco. É quase sempre a maior fatia, e é o que o dono da
    /// máquina sente como "liga mas não dá para usar".
    pub post_boot_ms: u64,
    /// Contador do Windows, crescente a cada boot. Serve de eixo do histórico.
    pub instance: u32,
    /// O próprio Windows comparando com a média histórica desta máquina.
    pub degraded: bool,
}

/// Um programa que atrasou a inicialização.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootCulprit {
    pub name: String,
    pub path: String,
    /// Quanto este programa levou no total.
    pub total_ms: u64,
    /// Quanto ele piorou em relação ao normal dele nesta máquina.
    pub degradation_ms: u64,
}

/// Como a máquina foi ligada da última vez.
///
/// Vem de um log diferente, que é legível SEM administrador — e responde uma
/// pergunta que aparece muito: "reiniciei e não melhorou".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootType {
    /// Boot completo de verdade.
    Full,
    /// Inicialização Rápida: o Windows não desligou, hibernou o núcleo do
    /// sistema e o restaurou. Nada que dependa de reiniciar tem efeito aqui.
    FastStartup,
    /// Retomada de hibernação.
    Resume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootReport {
    /// Faltou elevação para ler o log de desempenho.
    pub needs_admin: bool,
    pub last: Option<BootMeasurement>,
    /// Da mais recente para a mais antiga. Serve para mostrar evolução real
    /// depois de otimizar, em vez de prometer.
    pub history: Vec<BootMeasurement>,
    pub culprits: Vec<BootCulprit>,
    /// Tipo das últimas inicializações. Lido sem administrador.
    pub recent_types: Vec<(String, BootType)>,
    /// Explicação em português do que foi possível medir e do que não foi.
    pub note: String,
}

/// Limpa texto vindo de metadados de terceiro.
///
/// O nome e a empresa de um executável são texto arbitrário escolhido por quem
/// o compilou. Numa varredura real apareceu `№じ 尐乄鈊~→☆` no campo de empresa
/// de um programa. Isso vai para a tela do cliente e para o relatório que ele
/// recebe, então passa por aqui antes.
pub fn limpar(bruto: &str) -> String {
    let limpo: String = bruto
        .chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect();

    limpo.trim().to_string()
}

/// Converte milissegundos em algo que uma pessoa lê.
///
/// Usada pelos testes e pelo relatório de atendimento; a interface formata por
/// conta própria porque precisa do texto em elementos separados.
#[allow(dead_code)]
pub fn formatar_duracao(ms: u64) -> String {
    let segundos = ms as f64 / 1000.0;

    if segundos >= 60.0 {
        let minutos = (segundos / 60.0).floor();
        let resto = segundos - minutos * 60.0;
        format!("{:.0} min {:.0} s", minutos, resto)
    } else {
        format!("{:.1} s", segundos)
    }
}

// --------------------------------------------------------------- leitura

#[derive(Debug, Deserialize, Default)]
struct RawBoot {
    when: Option<String>,
    #[serde(rename = "BootTime")]
    boot_time: Option<String>,
    #[serde(rename = "MainPathBootTime")]
    main_path: Option<String>,
    #[serde(rename = "BootPostBootTime")]
    post_boot: Option<String>,
    #[serde(rename = "SystemBootInstance")]
    instance: Option<String>,
    #[serde(rename = "BootIsDegradation")]
    degradation: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawCulprit {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Path")]
    path: Option<String>,
    #[serde(rename = "TotalTime")]
    total: Option<String>,
    #[serde(rename = "DegradationTime")]
    degradation: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawBootType {
    when: Option<String>,
    #[serde(rename = "BootType")]
    kind: Option<String>,
}

/// Trecho de PowerShell que transforma os campos estruturados de um evento num
/// objeto com os nomes originais.
///
/// `$_.Name` é o atributo do manifesto do provedor — inglês em qualquer idioma.
/// A `Message` do evento nem é tocada.
const EXTRAIR_CAMPOS: &str = "\
    function Campos($e) { \
      $o = [ordered]@{ when = $e.TimeCreated.ToString('s') }; \
      foreach ($d in ([xml]$e.ToXml()).Event.EventData.Data) { \
        if ($d.Name) { $o[$d.Name] = [string]$d.'#text' } }; \
      return $o } ";

fn numero(campo: &Option<String>) -> u64 {
    campo
        .as_deref()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.max(0.0) as u64)
        .unwrap_or(0)
}

fn ler_boots() -> Option<Vec<RawBoot>> {
    let script = format!(
        "{} $e = Get-WinEvent -LogName '{}' -FilterXPath '*[System[EventID=100]]' \
         -MaxEvents 12 -ErrorAction Stop; \
         ConvertTo-Json -Compress -Depth 4 -InputObject @($e | ForEach-Object {{ Campos $_ }})",
        EXTRAIR_CAMPOS, LOG_DESEMPENHO
    );

    let saida = shell::powershell(&script).ok()?;

    if !saida.success || saida.stdout.trim().is_empty() {
        return None;
    }

    serde_json::from_str(&saida.stdout).ok()
}

fn ler_culpados() -> Vec<RawCulprit> {
    let script = format!(
        "{} $e = Get-WinEvent -LogName '{}' -FilterXPath '*[System[EventID=101]]' \
         -MaxEvents 40 -ErrorAction SilentlyContinue; \
         ConvertTo-Json -Compress -Depth 4 -InputObject @($e | ForEach-Object {{ Campos $_ }})",
        EXTRAIR_CAMPOS, LOG_DESEMPENHO
    );

    shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

/// Tipo das últimas inicializações, do log System.
///
/// Este log é legível sem elevação, então esta parte do relatório funciona
/// mesmo quando o resto não funciona.
fn ler_tipos() -> Vec<RawBootType> {
    let script = format!(
        "{} $e = Get-WinEvent -LogName System -FilterXPath \
         \"*[System[Provider[@Name='Microsoft-Windows-Kernel-Boot'] and EventID=27]]\" \
         -MaxEvents 10 -ErrorAction SilentlyContinue; \
         ConvertTo-Json -Compress -Depth 4 -InputObject @($e | ForEach-Object {{ Campos $_ }})",
        EXTRAIR_CAMPOS
    );

    shell::powershell(&script)
        .ok()
        .filter(|s| s.success && !s.stdout.trim().is_empty())
        .and_then(|s| serde_json::from_str(&s.stdout).ok())
        .unwrap_or_default()
}

/// Traduz o código de tipo de boot do Kernel-Boot.
pub fn tipo_do_codigo(codigo: u32) -> Option<BootType> {
    match codigo {
        0 => Some(BootType::Full),
        1 => Some(BootType::FastStartup),
        2 => Some(BootType::Resume),
        _ => None,
    }
}

/// Explica o que foi possível medir — e, quando não foi, por quê.
///
/// Esta função é o coração honesto do módulo. A tentação comercial é tratar
/// ausência de dado como boa notícia; aqui ela é dita como é.
pub fn montar_nota(
    elevado: bool,
    tem_medicao: bool,
    rapidas: usize,
    total_tipos: usize,
) -> String {
    if !elevado {
        return "O Windows guarda o tempo de inicialização num log protegido, que só \
                pode ser lido como administrador. Reabra o Otimiza com permissão para \
                ver quanto o seu PC demora para ligar e quais programas atrasam."
            .to_string();
    }

    if !tem_medicao {
        let mut nota = "O Windows não registrou nenhuma medição de inicialização nesta \
                        máquina. Acontece: o coletor de desempenho de boot para de gravar \
                        em parte dos PCs, e não há como forçá-lo. Não temos como dizer \
                        quanto o seu boot demora, e não vamos inventar um número."
            .to_string();

        if rapidas > 0 {
            nota.push_str(
                " Também vale saber que a Inicialização Rápida está ligada aqui, e ela \
                 reduz o que o Windows mede.",
            );
        }

        return nota;
    }

    let mut nota = String::from(
        "Números medidos pelo próprio Windows a cada inicialização, não estimados por nós.",
    );

    // A Inicialização Rápida é a explicação mais comum para "reiniciei e não
    // mudou nada": a máquina não desligou de verdade.
    if rapidas > 0 && total_tipos > 0 {
        nota.push_str(&format!(
            " Atenção: {} das últimas {} inicializações foram por Inicialização Rápida, \
             em que o Windows não desliga de fato — ele guarda o núcleo do sistema e \
             restaura. Por isso mudança que exige reiniciar às vezes só vale depois de um \
             desligamento completo.",
            rapidas, total_tipos
        ));
    }

    nota
}

/// Relatório completo de inicialização.
pub fn analyze() -> BootReport {
    let elevado = registry::is_elevated();

    // O tipo de boot vem primeiro porque é o único que funciona sem elevação:
    // mesmo sem nenhum outro dado, o relatório tem algo verdadeiro a dizer.
    let recent_types: Vec<(String, BootType)> = ler_tipos()
        .into_iter()
        .filter_map(|t| {
            let codigo = t.kind.as_deref()?.trim().parse::<u32>().ok()?;
            Some((t.when.unwrap_or_default(), tipo_do_codigo(codigo)?))
        })
        .collect();

    let rapidas = recent_types
        .iter()
        .filter(|(_, t)| *t == BootType::FastStartup)
        .count();

    let brutos = if elevado { ler_boots() } else { None };

    let history: Vec<BootMeasurement> = brutos
        .unwrap_or_default()
        .into_iter()
        .filter_map(|b| {
            let total_ms = numero(&b.boot_time);

            // Sem tempo total não há medição, e zero não é medição: seria
            // exatamente o número inventado que este módulo existe para evitar.
            if total_ms == 0 {
                return None;
            }

            Some(BootMeasurement {
                when: b.when.unwrap_or_default(),
                total_ms,
                main_path_ms: numero(&b.main_path),
                post_boot_ms: numero(&b.post_boot),
                instance: numero(&b.instance) as u32,
                degraded: b.degradation.as_deref() == Some("true"),
            })
        })
        .collect();

    let mut culprits: Vec<BootCulprit> = if elevado {
        ler_culpados()
            .into_iter()
            .filter_map(|c| {
                let total_ms = numero(&c.total);
                let name = limpar(&c.name.unwrap_or_default());

                if total_ms == 0 || name.is_empty() {
                    return None;
                }

                Some(BootCulprit {
                    name,
                    // O caminho é a chave confiável: o nome amigável vem vazio
                    // em boa parte dos programas.
                    path: limpar(&c.path.unwrap_or_default()),
                    total_ms,
                    degradation_ms: numero(&c.degradation),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    // O mesmo programa aparece uma vez por boot. Fica o pior caso de cada um,
    // que é o que o cliente sente no dia ruim.
    culprits.sort_by(|a, b| a.path.cmp(&b.path).then(b.total_ms.cmp(&a.total_ms)));
    culprits.dedup_by(|a, b| a.path == b.path && !a.path.is_empty());
    culprits.sort_by(|a, b| b.total_ms.cmp(&a.total_ms));
    culprits.truncate(12);

    let note = montar_nota(elevado, !history.is_empty(), rapidas, recent_types.len());

    BootReport {
        needs_admin: !elevado,
        last: history.first().cloned(),
        history,
        culprits,
        recent_types,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadado_hostil_de_terceiro_e_limpo() {
        // Apareceu de verdade numa varredura: campo de empresa com caracteres
        // de controle e lixo. Isso vai para a tela do cliente e para o
        // relatório que ele recebe.
        let sujo = "Programa\u{0}\u{1}\tRuim\r\n";
        let limpo = limpar(sujo);

        assert!(!limpo.contains('\u{0}'));
        assert!(!limpo.contains('\r'));
        assert!(limpo.starts_with("Programa"));

        // E nome gigante não pode esticar a lista.
        assert!(limpar(&"a".repeat(500)).chars().count() <= 80);
    }

    #[test]
    fn duracao_vira_texto_legivel() {
        assert_eq!(formatar_duracao(1500), "1.5 s");
        assert_eq!(formatar_duracao(45_000), "45.0 s");
        // Acima de um minuto ninguém lê "97,6 s" e entende.
        assert_eq!(formatar_duracao(97_666), "1 min 38 s");
    }

    #[test]
    fn tipo_de_boot_vem_do_numero() {
        assert_eq!(tipo_do_codigo(0), Some(BootType::Full));
        assert_eq!(tipo_do_codigo(1), Some(BootType::FastStartup));
        assert_eq!(tipo_do_codigo(2), Some(BootType::Resume));
        // Código desconhecido não vira palpite.
        assert_eq!(tipo_do_codigo(9), None);
    }

    #[test]
    fn sem_elevacao_a_nota_explica_em_vez_de_dar_boa_noticia() {
        let nota = montar_nota(false, false, 0, 0);

        assert!(nota.contains("administrador"));
        // O que não pode acontecer de jeito nenhum.
        assert!(!nota.to_lowercase().contains("ótimo"));
        assert!(!nota.to_lowercase().contains("rápido"));
    }

    #[test]
    fn elevado_e_sem_dado_admite_que_nao_sabe() {
        let nota = montar_nota(true, false, 0, 5);

        // A frase que separa este produto do resto do mercado.
        assert!(nota.contains("não vamos inventar"));
        assert!(nota.contains("não temos como dizer") || nota.contains("Não temos como dizer"));
    }

    #[test]
    fn inicializacao_rapida_e_avisada_quando_ha_medicao() {
        let nota = montar_nota(true, true, 4, 10);

        // Responde "reiniciei e não melhorou" antes de o cliente perguntar.
        assert!(nota.contains("Inicialização Rápida"));
        assert!(nota.contains("4 das últimas 10"));
    }

    #[test]
    fn numero_invalido_nao_vira_zero_silencioso() {
        assert_eq!(numero(&Some("1234".into())), 1234);
        assert_eq!(numero(&Some("  55 ".into())), 55);
        assert_eq!(numero(&None), 0);
        assert_eq!(numero(&Some("texto".into())), 0);
        // Negativo não existe em duração.
        assert_eq!(numero(&Some("-5".into())), 0);
    }

    #[test]
    fn analisa_esta_maquina() {
        let r = analyze();

        println!("precisa de administrador: {}", r.needs_admin);
        println!("nota: {}", r.note);
        println!("tipos recentes: {:?}", r.recent_types.len());

        for (quando, tipo) in r.recent_types.iter().take(5) {
            println!("  {} -> {:?}", quando, tipo);
        }

        if let Some(ultimo) = &r.last {
            println!(
                "  ultimo boot: {} (ate a area de trabalho {}, depois {})",
                formatar_duracao(ultimo.total_ms),
                formatar_duracao(ultimo.main_path_ms),
                formatar_duracao(ultimo.post_boot_ms)
            );
        }

        for c in r.culprits.iter().take(5) {
            println!("  atraso: {} — {}", c.name, formatar_duracao(c.total_ms));
        }

        // A nota nunca pode ficar vazia: se não há dado, o motivo precisa estar
        // escrito. Silêncio aqui viraria "está tudo bem" na cabeça de quem lê.
        assert!(!r.note.is_empty());

        // Culpados só existem com medição; e todo culpado tem tempo.
        assert!(r.culprits.iter().all(|c| c.total_ms > 0));
        assert!(r.culprits.iter().all(|c| !c.name.is_empty()));

        // Do pior para o melhor.
        assert!(r
            .history
            .iter()
            .all(|m| m.total_ms >= m.main_path_ms));
        assert!(r.culprits.windows(2).all(|p| p[0].total_ms >= p[1].total_ms));

        // Sem elevação não pode haver medição nenhuma — seria dado inventado.
        if r.needs_admin {
            assert!(r.last.is_none());
            assert!(r.culprits.is_empty());
        }
    }

    #[test]
    fn mesmo_programa_nao_aparece_duas_vezes() {
        let r = analyze();

        let mut caminhos: Vec<&str> = r
            .culprits
            .iter()
            .map(|c| c.path.as_str())
            .filter(|p| !p.is_empty())
            .collect();

        let antes = caminhos.len();
        caminhos.sort_unstable();
        caminhos.dedup();

        assert_eq!(
            antes,
            caminhos.len(),
            "o mesmo programa apareceu mais de uma vez na lista"
        );
    }
}
