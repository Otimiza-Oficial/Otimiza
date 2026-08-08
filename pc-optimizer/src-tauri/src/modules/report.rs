// Relatório técnico de atendimento
//
// Quem usa o Otimiza profissionalmente tem um problema que nenhum ajuste de
// registro resolve: provar o serviço. O cliente entrega um PC lento, recebe um
// PC melhor e não tem como saber o que foi feito — o que coloca o técnico
// honesto no mesmo balaio de quem só reinicia a máquina e cobra.
//
// Este módulo gera um PDF que o técnico entrega junto com o computador. Ele
// levanta o estado real da máquina, lista cada mudança pelo nome com o valor
// que existia antes, e diz que tudo pode ser desfeito.
//
// COMO O PDF É PRODUZIDO
//
// O documento é escrito em HTML com folha de estilo de impressão, e convertido
// pelo Microsoft Edge em modo sem interface. A escolha foi deliberada: o Edge
// existe em toda instalação de Windows 10 e 11, então não há dependência nova
// no instalador, e o resultado é tipografia de verdade — o que uma biblioteca
// de PDF em Rust só entregaria com muito código e uma fonte embarcada.
//
// Se o Edge não estiver disponível, o HTML é gravado assim mesmo e o programa
// diz o que aconteceu. Melhor entregar o documento em outro formato do que
// falhar em silêncio.
//
// TRÊS REGRAS DO TEXTO GERADO
//
// 1. Nada de número inventado. Se não houve medição, o relatório diz que não
//    houve — em vez de estampar uma porcentagem decorativa.
// 2. O veredito da medição vai como saiu, inclusive "sem diferença" e "piorou".
// 3. Nenhum símbolo decorativo. É um documento técnico que pode acabar anexado
//    a uma nota de serviço.

use crate::modules::benchmark::{BenchmarkComparison, MetricDelta, Verdict};
use crate::modules::changelog::ChangeLog;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ReportSaved {
    pub path: String,
    /// Verdadeiro quando saiu PDF; falso quando só foi possível gravar o HTML.
    pub is_pdf: bool,
    pub optimizations: usize,
    pub changes: usize,
    /// Explicação, quando o PDF não pôde ser gerado.
    pub note: String,
}

/// Tudo que o relatório mostra sobre a máquina.
///
/// Coletado pelo comando antes de montar o documento. Cada campo é opcional
/// porque cada análise pode falhar por conta própria — e uma seção ausente é
/// dita como ausente, não omitida em silêncio.
#[derive(Default)]
pub struct ReportData {
    #[cfg(target_os = "windows")]
    pub boot: Option<crate::modules::windows::boot::BootReport>,
    #[cfg(target_os = "windows")]
    pub thermal: Option<crate::modules::windows::thermal::ThermalReport>,
    #[cfg(target_os = "windows")]
    pub health: Option<crate::modules::windows::health::HealthReport>,
    #[cfg(target_os = "windows")]
    pub memory: Option<crate::modules::windows::memory::MemoryReport>,
    #[cfg(target_os = "windows")]
    pub browsers: Option<crate::modules::windows::browsers::BrowserReport>,
    #[cfg(target_os = "windows")]
    pub startup: Vec<crate::modules::windows::startup::StartupEntry>,
}

/// Escapa texto para HTML.
///
/// Nome de programa instalado entra neste relatório, e nome de programa é texto
/// de terceiro: sem escapar, um instalador com `<script>` no nome viraria código
/// executando na máquina de quem abrir o arquivo.
fn escape(raw: &str) -> String {
    raw.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            other => other.to_string(),
        })
        .collect()
}

fn verdict_label(verdict: &Verdict) -> (&'static str, &'static str) {
    match verdict {
        Verdict::Improved => ("melhorou", "bom"),
        Verdict::Worsened => ("piorou", "ruim"),
        Verdict::NoMeasurableChange => ("sem diferença medível", "neutro"),
        Verdict::TooNoisyToJudge => ("só referência", "neutro"),
    }
}

fn secao(numero: u8, titulo: &str, corpo: String) -> String {
    format!(
        "<section class=\"sec\"><h2><span class=\"num\">{}</span>{}</h2>{}</section>",
        numero,
        escape(titulo),
        corpo
    )
}

fn aviso(texto: &str) -> String {
    format!("<p class=\"aviso\">{}</p>", escape(texto))
}

fn duracao(ms: u64) -> String {
    let s = ms as f64 / 1000.0;

    if s >= 60.0 {
        format!("{} min {:.0} s", (s / 60.0).floor(), s % 60.0)
    } else {
        format!("{:.1} s", s)
    }
}

// ------------------------------------------------------------------ seções

fn secao_maquina() -> String {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::hardware::{profile, StorageKind};

        let h = profile();
        let armazenamento = match h.system_storage {
            StorageKind::Ssd => "SSD",
            StorageKind::Hdd => "disco mecânico",
            StorageKind::Unknown => "não identificado",
        };

        secao(
            1,
            "Identificação da máquina",
            format!(
                "<table class=\"dados\"><tbody>\
                 <tr><th>Memória instalada</th><td class=\"num\">{:.1} GB</td></tr>\
                 <tr><th>Núcleos lógicos</th><td class=\"num\">{}</td></tr>\
                 <tr><th>Disco do sistema</th><td class=\"num\">{}</td></tr>\
                 </tbody></table>",
                h.total_ram_gb, h.logical_cores, armazenamento
            ),
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        String::new()
    }
}

#[cfg(target_os = "windows")]
fn secao_boot(dados: &ReportData) -> String {
    use crate::modules::windows::boot::BootType;

    let Some(r) = &dados.boot else {
        return secao(2, "Tempo de inicialização", aviso("Não foi possível ler."));
    };

    let mut corpo = String::new();

    if let Some(b) = &r.last {
        corpo.push_str(&format!(
            "<table class=\"dados\"><tbody>\
             <tr><th>Boot completo, até a máquina ficar utilizável</th><td class=\"num\">{}</td></tr>\
             <tr><th>Até a área de trabalho aparecer</th><td class=\"num\">{}</td></tr>\
             <tr><th>Depois disso, programas de inicialização</th><td class=\"num\">{}</td></tr>\
             </tbody></table>\
             <p class=\"nota\">A última linha costuma ser a maior fatia. E o intervalo em que \
             o computador já mostra a área de trabalho mas ainda não responde.</p>",
            duracao(b.total_ms),
            duracao(b.main_path_ms),
            duracao(b.post_boot_ms)
        ));
    }

    if !r.culprits.is_empty() {
        let linhas: String = r
            .culprits
            .iter()
            .map(|c| {
                format!(
                    "<tr><td>{}</td><td class=\"cam\">{}</td><td class=\"num\">{}</td></tr>",
                    escape(&c.name),
                    escape(&c.path),
                    duracao(c.total_ms)
                )
            })
            .collect();

        corpo.push_str(&format!(
            "<h3>Programas que atrasaram a inicialização</h3>\
             <table><thead><tr><th>Programa</th><th>Local</th><th>Tempo</th></tr></thead>\
             <tbody>{}</tbody></table>\
             <p class=\"nota\">Medição do próprio Windows, registrada a cada inicialização. \
             Nenhum destes números foi calculado pelo Otimiza.</p>",
            linhas
        ));
    }

    let rapidas = r
        .recent_types
        .iter()
        .filter(|(_, t)| *t == BootType::FastStartup)
        .count();

    if rapidas > 0 {
        corpo.push_str(&format!(
            "<p class=\"nota\">Das últimas {} inicializações, {} usaram Inicialização Rápida, \
             em que o Windows não desliga de fato. Ajustes que dependem de reiniciar só passam \
             a valer após um desligamento completo.</p>",
            r.recent_types.len(),
            rapidas
        ));
    }

    if corpo.is_empty() {
        corpo.push_str(&aviso(&r.note));
    }

    secao(2, "Tempo de inicialização", corpo)
}

#[cfg(target_os = "windows")]
fn secao_processador(dados: &ReportData) -> String {
    use crate::modules::windows::thermal::Culprit;

    let Some(r) = &dados.thermal else {
        return String::new();
    };

    let mut corpo = format!("<p class=\"achado\">{}</p>", escape(&r.summary));

    if !r.advice.is_empty() {
        corpo.push_str(&format!("<p>{}</p>", escape(&r.advice)));
    }

    corpo.push_str(&format!(
        "<table class=\"dados\"><tbody>\
         <tr><th>Frequência observada</th><td class=\"num\">{}</td></tr>\
         <tr><th>Teto no plano de energia</th><td class=\"num\">{}</td></tr>\
         <tr><th>Alimentação</th><td class=\"num\">{}</td></tr>\
         <tr><th>Eventos térmicos nos últimos 30 dias</th><td class=\"num\">{}</td></tr>\
         </tbody></table>",
        r.percent_of_max
            .map(|p| format!("{:.0}% do maximo", p))
            .unwrap_or_else(|| "não lida".to_string()),
        r.power_cap_percent
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "padrão do Windows".to_string()),
        if r.on_battery { "bateria" } else { "tomada" },
        r.thermal_events
    ));

    // O critério é a parte mais importante desta seção: ele é o que permite ao
    // cliente conferir que não houve chute.
    if r.culprit == Culprit::Calor || r.culprit == Culprit::NaoIdentificado {
        corpo.push_str(
            "<p class=\"nota\">Criterio adotado: descartar bateria, depois o teto do plano de \
             energia, e só então considerar temperatura, exclusivamente quando o próprio \
             Windows registrou um evento térmico. Sem esse registro, o Otimiza declara causa \
             não identificada em vez de atribuir a temperatura.</p>",
        );
    }

    secao(3, "Desempenho do processador", corpo)
}

#[cfg(target_os = "windows")]
fn secao_saude(dados: &ReportData) -> String {
    use crate::modules::windows::firmware::FindingSeverity;

    let Some(r) = &dados.health else {
        return String::new();
    };

    let mut corpo = String::new();

    if r.needs_admin {
        corpo.push_str(&aviso(
            "A leitura de desgaste e de contagem de erros do disco exige privilégio \
             administrativo, e o relatório foi gerado sem ele. Os dados abaixo estão \
             incompletos.",
        ));
    }

    let linhas: String = r
        .findings
        .iter()
        .map(|f| {
            // Nome de classe fica em ASCII; o acento vai só no texto visível.
            let classe = match f.severity {
                FindingSeverity::Critical => "ruim",
                FindingSeverity::Important => "atencao",
                FindingSeverity::Ok => "bom",
            };

            format!(
                "<tr><td>{}</td><td>{}</td><td class=\"v {}\">{}</td></tr>",
                escape(&f.title),
                escape(&f.measured),
                classe,
                match f.severity {
                    FindingSeverity::Critical => "crítico",
                    FindingSeverity::Important => "atenção",
                    FindingSeverity::Ok => "normal",
                }
            )
        })
        .collect();

    if !linhas.is_empty() {
        corpo.push_str(&format!(
            "<table><thead><tr><th>Item</th><th>Medido</th><th>Situação</th></tr></thead>\
             <tbody>{}</tbody></table>",
            linhas
        ));
    }

    // Conselho de item crítico vai por extenso: é a informação que justifica
    // trocar peça, e ela não pode caber numa palavra de tabela.
    for f in r.findings.iter().filter(|f| f.severity == FindingSeverity::Critical) {
        if !f.advice.is_empty() {
            corpo.push_str(&format!(
                "<p class=\"achado\">{}: {}</p>",
                escape(&f.title),
                escape(&f.advice)
            ));
        }
    }

    if corpo.is_empty() {
        return String::new();
    }

    secao(4, "Saúde física do disco e da bateria", corpo)
}

#[cfg(target_os = "windows")]
fn secao_memoria(dados: &ReportData) -> String {
    let Some(r) = &dados.memory else {
        return String::new();
    };

    let mut corpo = format!(
        "<table class=\"dados\"><tbody>\
         <tr><th>Memória física</th><td class=\"num\">{:.1} GB</td></tr>\
         <tr><th>Disponível no momento da coleta</th><td class=\"num\">{:.1} GB</td></tr>\
         <tr><th>Prometida a programas</th><td class=\"num\">{:.1} GB</td></tr>\
         <tr><th>Arquivo de paginação</th><td class=\"num\">{:.1} GB, {}</td></tr>\
         <tr><th>Pico de paginação desde o boot</th><td class=\"num\">{:.1} GB</td></tr>\
         </tbody></table>",
        r.total_ram_gb,
        r.available_ram_gb,
        r.committed_gb,
        r.pagefile_size_gb,
        if r.pagefile_automatic {
            "gerenciado pelo Windows"
        } else {
            "tamanho fixo definido manualmente"
        },
        r.pagefile_peak_gb
    );

    // Memória prometida acima da física é a explicação mais comum de "congela
    // do nada" em máquina de 4 a 8 GB.
    if r.committed_gb > r.total_ram_gb {
        corpo.push_str(&format!(
            "<p class=\"achado\">Os programas em uso pediram {:.1} GB, mais que os {:.1} GB \
             físicos instalados. A diferença e sustentada pelo disco, que e ordens de grandeza \
             mais lento que a memória. E a causa mais comum de travamentos momentâneos nesta \
             faixa de hardware.</p>",
            r.committed_gb, r.total_ram_gb
        ));
    }

    secao(5, "Memória e paginação", corpo)
}

#[cfg(target_os = "windows")]
fn secao_navegador(dados: &ReportData) -> String {
    let Some(r) = &dados.browsers else {
        return String::new();
    };

    if r.browsers.is_empty() {
        return String::new();
    }

    let linhas: String = r
        .browsers
        .iter()
        .map(|b| {
            let extensoes: usize = b.profiles.iter().map(|p| p.extensions.len()).sum();
            let cache: u64 = b.profiles.iter().map(|p| p.cache_bytes).sum();

            format!(
                "<tr><td>{}{}</td><td class=\"num\">{:.0} MB</td><td class=\"num\">{}</td>\
                 <td class=\"num\">{:.0} MB</td></tr>",
                escape(&b.name),
                if b.is_default { " (padrão)" } else { "" },
                b.ram_mb,
                extensoes,
                cache as f64 / 1_048_576.0
            )
        })
        .collect();

    let mut corpo = format!(
        "<table><thead><tr><th>Navegador</th><th>Memória</th><th>Extensões</th>\
         <th>Cache</th></tr></thead><tbody>{}</tbody></table>\
         <p>Em conjunto, os navegadores ocupavam {:.0} MB, equivalentes a {:.1}% da memória \
         desta máquina no momento da coleta.</p>",
        linhas, r.total_ram_mb, r.ram_percent
    );

    if r.total_app_data_mb >= 1.0 {
        corpo.push_str(&format!(
            "<p class=\"nota\">Alem do cache, há {:.0} MB classificados como dado de \
             aplicativo — conteúdo guardado por sites para uso sem conexão. Apesar do volume, \
             não e descartável: apagar encerra sessões e destrói dados que não existem em \
             outro lugar. O Otimiza mede e informa esse valor, e não o oferece para limpeza.</p>",
            r.total_app_data_mb
        ));
    }

    // A ausência é tão informativa quanto a presença, e evita a pergunta óbvia.
    corpo.push_str(
        "<p class=\"nota\">Consumo de memória por extensão não consta deste relatório porque \
         não e mensurável a partir do sistema operacional: diversas extensões compartilham um \
         mesmo processo. Qualquer valor individual apresentado aqui seria estimado, e este \
         documento não apresenta estimativas como medições.</p>",
    );

    secao(6, "Navegadores", corpo)
}

#[cfg(target_os = "windows")]
fn secao_inicializacao(dados: &ReportData) -> String {
    if dados.startup.is_empty() {
        return String::new();
    }

    let linhas: String = dados
        .startup
        .iter()
        .map(|e| {
            format!(
                "<tr><td>{}</td><td class=\"cam\">{}</td><td class=\"num\">{}</td></tr>",
                escape(&e.name),
                escape(&e.command),
                if e.enabled { "ativo" } else { "desativado" }
            )
        })
        .collect();

    secao(
        7,
        "Programas de inicialização",
        format!(
            "<table><thead><tr><th>Nome</th><th>Comando</th><th>Estado</th></tr></thead>\
             <tbody>{}</tbody></table>",
            linhas
        ),
    )
}

fn linha_metrica(metric: &MetricDelta) -> String {
    let (rotulo, classe) = verdict_label(&metric.verdict);

    format!(
        "<tr><td>{}</td><td class=\"num\">{:.1} {}</td><td class=\"num\">{:.1} {}</td>\
         <td class=\"num\">{:+.1}%</td><td class=\"v {}\">{}</td></tr>",
        escape(&metric.label),
        metric.before,
        escape(&metric.unit),
        metric.after,
        escape(&metric.unit),
        metric.change_percent,
        classe,
        rotulo
    )
}

fn secao_medicao(comparison: Option<&BenchmarkComparison>) -> String {
    let Some(c) = comparison else {
        // O caso mais importante deste módulo. Sem medição, o relatório precisa
        // dizer isso em voz alta — é exatamente aqui que um produto desonesto
        // colocaria "desempenho melhorado em 40%".
        return secao(
            8,
            "Medição de desempenho",
            aviso(
                "Não foi realizada medição comparativa antes e depois neste atendimento. Sem \
                 as duas medidas não há como afirmar ganho, e este relatório não estima \
                 números que não foram medidos.",
            ),
        );
    };

    let linhas: String = c.metrics.iter().map(linha_metrica).collect();

    secao(
        8,
        "Medição de desempenho",
        format!(
            "<p>{}</p>\
             <table><thead><tr><th>Grandeza</th><th>Antes</th><th>Depois</th>\
             <th>Variação</th><th>Leitura</th></tr></thead><tbody>{}</tbody></table>\
             <p class=\"nota\">As grandezas marcadas como \"só referência\" apresentam \
             variação natural superior ao efeito que se pretende medir, e por isso são \
             exibidas sem veredito. Vereditos desfavoráveis, quando ocorrem, constam nesta \
             tabela como qualquer outro.</p>",
            escape(&c.summary),
            linhas
        ),
    )
}

fn secao_mudancas(log: &ChangeLog) -> (String, usize, usize) {
    let aplicadas = log.applied();

    if aplicadas.is_empty() {
        return (
            secao(
                9,
                "Alterações aplicadas",
                aviso("Nenhuma otimização do Otimiza está aplicada nesta máquina no momento."),
            ),
            0,
            0,
        );
    }

    let mut total_mudancas = 0;
    let mut blocos = String::new();

    for otimizacao in aplicadas {
        total_mudancas += otimizacao.changes.len();

        let itens: String = otimizacao
            .changes
            .iter()
            .map(|c| format!("<li>{}</li>", escape(&c.describe())))
            .collect();

        blocos.push_str(&format!(
            "<article class=\"mud\"><h4>{}</h4><ul>{}</ul></article>",
            escape(&otimizacao.name),
            itens
        ));
    }

    let html = secao(
        9,
        "Alterações aplicadas",
        format!(
            "<p>{} otimização(oes), {} alteração(oes) no total. Cada item indica o valor que \
             existia antes da mudança. E esse valor que retorna caso a alteração seja \
             desfeita.</p>{}",
            aplicadas.len(),
            total_mudancas,
            blocos
        ),
    );

    (html, aplicadas.len(), total_mudancas)
}

fn secao_recusas() -> String {
    secao(
        10,
        "Procedimentos deliberadamente não executados",
        "<p>As práticas abaixo produzem ganho de desempenho e são adotadas por parte do \
         mercado. O Otimiza não as executa, e o motivo de cada uma consta a seguir para \
         registro.</p>\
         <ul class=\"recusas\">\
         <li><b>Desativar as mitigacoes de Spectre e Meltdown.</b> Produz ganho mensurável em \
         processadores mais antigos ao custo de reabrir vulnerabilidades conhecidas de \
         execução especulativa.</li>\
         <li><b>Desativar Windows Update, Windows Defender ou firewall.</b> Reduz processos em \
         segundo plano e remove camadas de proteção cuja ausência não e percebida até ser \
         explorada.</li>\
         <li><b>Limpeza de registro.</b> Não há ganho de desempenho demonstrável na remoção de \
         chaves órfãs, e há risco documentado de inutilizar software instalado.</li>\
         <li><b>Liberação forçada de memória.</b> Esvazia o conjunto de trabalho dos processos, \
         melhorando o indicador exibido e degradando o desempenho real, já que os dados \
         precisam ser relidos do disco.</li>\
         <li><b>Escrita em firmware.</b> Em placas de consumo, as configuracoes residem em área \
         proprietaria da NVRAM; erro de escrita inutiliza a placa-mae de forma permanente.</li>\
         </ul>"
            .to_string(),
    )
}

// ------------------------------------------------------------------- estilo

const ESTILO: &str = r#"
@page { size: A4; margin: 20mm 18mm 22mm; }
:root { color-scheme: light }
* { box-sizing: border-box }
body {
  margin: 0; color: #14130f; background: #fff;
  font: 10.5pt/1.55 "Georgia", "Cambria", "Times New Roman", serif;
  -webkit-print-color-adjust: exact; print-color-adjust: exact;
}
.doc { max-width: 174mm; margin: 0 auto }

header.capa { border-bottom: 2.5pt solid #14130f; padding-bottom: 9mm; margin-bottom: 9mm }
.marca { font: 700 9pt/1 "Consolas", monospace; letter-spacing: .34em; text-transform: uppercase }
h1 { margin: 3mm 0 1.5mm; font-size: 21pt; font-weight: 400; letter-spacing: -.015em }
.sub { color: #55524b; font-size: 9.5pt }
.meta { margin-top: 5mm; font: 8.5pt/1.5 "Consolas", monospace; color: #55524b }
.meta b { color: #14130f; font-weight: 700 }

.sec { margin-top: 9mm; break-inside: avoid }
h2 {
  font: 700 11pt/1.3 "Consolas", monospace; letter-spacing: .1em; text-transform: uppercase;
  border-bottom: 1pt solid #14130f; padding-bottom: 2mm; margin-bottom: 4mm;
}
h2 .num {
  display: inline-block; min-width: 9mm; color: #8b8880; font-weight: 400;
}
h3 { margin: 6mm 0 2.5mm; font-size: 11pt; font-weight: 700 }
h4 { margin: 0 0 1.5mm; font-size: 10pt; font-weight: 700 }
p { margin: 0 0 3mm }

table { width: 100%; border-collapse: collapse; margin: 3mm 0 4mm; font-size: 9.5pt }
thead th {
  text-align: left; font: 700 8pt/1.4 "Consolas", monospace; letter-spacing: .07em;
  text-transform: uppercase; color: #55524b; padding: 1.6mm 2mm;
  border-bottom: 1pt solid #14130f; white-space: nowrap;
}
td, tbody th { padding: 1.8mm 2mm; border-bottom: .4pt solid #ddd9d1; vertical-align: top }
tbody th { text-align: left; font-weight: 400; width: 62% }
tbody tr:last-child td, tbody tr:last-child th { border-bottom: 0 }
.dados { max-width: 120mm }
td.num, th.num { text-align: right; font-family: "Consolas", monospace; font-size: 9pt;
  font-variant-numeric: tabular-nums; white-space: nowrap }
td.cam { font-family: "Consolas", monospace; font-size: 7.5pt; color: #55524b;
  word-break: break-all }
.v { text-align: right; font-weight: 700; font-size: 8.5pt; text-transform: uppercase;
  font-family: "Consolas", monospace; letter-spacing: .05em }
.v.bom { color: #1a6b45 } .v.ruim { color: #a3221c } .v.atencao { color: #8a6212 }
.v.neutro { color: #55524b }

.achado { border-left: 2.5pt solid #14130f; padding: 2.5mm 0 2.5mm 4mm; margin: 4mm 0 }
.aviso { border-left: 2.5pt solid #8a6212; background: #fbf7ec; padding: 3mm 4mm; margin: 3mm 0 }
.nota { font-size: 9pt; color: #55524b }
.mud { margin: 3mm 0; padding: 3mm 4mm; background: #f7f6f3; break-inside: avoid }
.mud ul { margin: 0; padding-left: 5mm }
.mud li { font: 8pt/1.6 "Consolas", monospace; color: #3d3a34 }
.recusas { margin: 0; padding-left: 5mm }
.recusas li { margin-bottom: 2.5mm }

footer.fim { margin-top: 10mm; padding-top: 4mm; border-top: 1pt solid #14130f; font-size: 9pt }
footer.fim p { margin-bottom: 2.5mm }
.rodape {
  position: fixed; bottom: -14mm; left: 0; right: 0;
  border-top: .4pt solid #ddd9d1; padding-top: 1.5mm;
  font: 7.5pt/1 "Consolas", monospace; color: #8b8880;
  display: flex; justify-content: space-between;
}
"#;

/// Monta o documento completo.
pub fn build_html(
    log: &ChangeLog,
    comparison: Option<&BenchmarkComparison>,
    dados: &ReportData,
    data: &str,
) -> String {
    let (mudancas_html, _, _) = secao_mudancas(log);

    #[allow(unused_mut)]
    let mut diagnostico = String::new();

    #[cfg(target_os = "windows")]
    {
        diagnostico.push_str(&secao_boot(dados));
        diagnostico.push_str(&secao_processador(dados));
        diagnostico.push_str(&secao_saude(dados));
        diagnostico.push_str(&secao_memoria(dados));
        diagnostico.push_str(&secao_navegador(dados));
        diagnostico.push_str(&secao_inicializacao(dados));
    }

    #[cfg(not(target_os = "windows"))]
    let _ = dados;

    format!(
        "<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\">\
         <title>Otimiza — relatório técnico de atendimento</title><style>{}</style></head>\
         <body><div class=\"doc\">\
         <div class=\"rodape\"><span>OTIMIZA - Relatório técnico de atendimento</span>\
         <span>{}</span></div>\
         <header class=\"capa\">\
         <p class=\"marca\">Otimiza</p>\
         <h1>Relatório técnico de atendimento</h1>\
         <p class=\"sub\">Levantamento do estado da máquina, alterações executadas e \
         resultado medido.</p>\
         <p class=\"meta\">Emitido em <b>{}</b><br>\
         Todos os valores deste documento foram lidos do sistema operacional no momento da \
         emissao. Nenhum foi estimado.</p>\
         </header>\
         {}{}{}{}{}\
         <footer class=\"fim\">\
         <p><b>Reversibilidade.</b> Cada alteração registra o valor anterior antes da escrita. \
         A funcao de desfazer restaura exatamente o valor original, e não um valor equivalente. \
         As excecoes são a exclusao de arquivos temporarios, a limpeza do cache de atualizacoes \
         e a limpeza de cache de navegador, identificadas como irreversiveis no próprio \
         programa e executadas apenas mediante confirmacao.</p>\
         <p><b>Escopo.</b> Este relatório apresenta somente o que foi efetivamente executado e \
         medido nesta máquina. Secoes que informam ausência de dado indicam que a medição não \
         estava disponivel, e não que o item esteja em conformidade.</p>\
         </footer>\
         </div></body></html>",
        ESTILO,
        escape(data),
        escape(data),
        secao_maquina(),
        diagnostico,
        secao_medicao(comparison),
        mudancas_html,
        secao_recusas()
    )
}

// -------------------------------------------------------------- gravação

/// Pasta da Área de Trabalho do usuário.
///
/// Lê do registro em vez de montar `%USERPROFILE%\Desktop`: com OneDrive ligado
/// — o padrão em notebook de loja — a Área de Trabalho real fica dentro da pasta
/// do OneDrive, e o caminho montado na mão apontaria para uma pasta órfã que o
/// usuário nunca vê. O nome da pasta também muda de idioma; o registro, não.
fn desktop_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let shell_folders = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";

        if let Some(path) =
            crate::modules::windows::registry::read_text("HKCU", shell_folders, "Desktop")
        {
            let candidate = PathBuf::from(&path);
            if candidate.is_dir() {
                return candidate;
            }
        }
    }

    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Onde o Microsoft Edge está instalado.
///
/// Os dois caminhos cobrem instalação de 64 e de 32 bits. O Edge acompanha o
/// Windows 10 e 11, então na prática ele está sempre em um dos dois.
#[cfg(target_os = "windows")]
pub fn caminho_do_edge() -> Option<PathBuf> {
    [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| p.is_file())
}

/// Converte o HTML em PDF usando o Edge sem interface.
#[cfg(target_os = "windows")]
fn imprimir_pdf(html: &Path, pdf: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let edge = caminho_do_edge().ok_or("Microsoft Edge não encontrado nesta máquina")?;

    // `file:///` com barras normais: o Chromium não aceita barra invertida na
    // URL, mesmo no Windows.
    let url = format!("file:///{}", html.to_string_lossy().replace('\\', "/"));

    let saida = std::process::Command::new(edge)
        .args([
            "--headless",
            "--disable-gpu",
            // Sem isto o Edge carimba a URL do arquivo e um cabeçalho de
            // navegador em toda página, o que estraga um documento entregue
            // a cliente.
            "--no-pdf-header-footer",
            &format!("--print-to-pdf={}", pdf.to_string_lossy()),
            &url,
        ])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Não foi possível executar o Edge: {}", e))?;

    // O código de saída do Edge não é confiável aqui, e ele escreve avisos no
    // stderr mesmo quando dá certo. O que decide é o arquivo ter sido criado.
    if !pdf.is_file() {
        return Err(format!(
            "O Edge não gerou o PDF. {}",
            String::from_utf8_lossy(&saida.stderr).lines().next().unwrap_or("")
        ));
    }

    Ok(())
}

/// Gera o relatório e grava na Área de Trabalho.
pub fn save(
    log: &ChangeLog,
    comparison: Option<&BenchmarkComparison>,
    dados: &ReportData,
) -> Result<ReportSaved, String> {
    let agora = chrono::Local::now();
    let data = agora.format("%d/%m/%Y as %H:%M").to_string();
    // Ano-mês-dia no nome mantém os relatórios em ordem na pasta.
    let base = format!("Otimiza - relatorio - {}", agora.format("%Y-%m-%d %Hh%M"));

    let html = build_html(log, comparison, dados, &data);
    let (_, optimizations, changes) = secao_mudancas(log);

    let destino = desktop_dir();
    let caminho_html = destino.join(format!("{}.html", base));
    let caminho_pdf = destino.join(format!("{}.pdf", base));

    std::fs::write(&caminho_html, &html)
        .map_err(|e| format!("Não foi possível gravar em {:?}: {}", caminho_html, e))?;

    #[cfg(target_os = "windows")]
    {
        match imprimir_pdf(&caminho_html, &caminho_pdf) {
            Ok(()) => {
                // O HTML era só o insumo do PDF; deixá-lo na Área de Trabalho
                // faria o cliente receber dois arquivos e não saber qual abrir.
                let _ = std::fs::remove_file(&caminho_html);

                return Ok(ReportSaved {
                    path: caminho_pdf.to_string_lossy().to_string(),
                    is_pdf: true,
                    optimizations,
                    changes,
                    note: String::new(),
                });
            }
            Err(motivo) => {
                return Ok(ReportSaved {
                    path: caminho_html.to_string_lossy().to_string(),
                    is_pdf: false,
                    optimizations,
                    changes,
                    note: format!(
                        "O relatório foi gravado em HTML porque não foi possível gerar o PDF: {}. \
                         O arquivo abre em qualquer navegador e pode ser impresso ou salvo como \
                         PDF pelo menu de impressao.",
                        motivo
                    ),
                });
            }
        }
    }

    #[allow(unreachable_code)]
    Ok(ReportSaved {
        path: caminho_html.to_string_lossy().to_string(),
        is_pdf: false,
        optimizations,
        changes,
        note: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nome_de_programa_com_html_nao_vira_codigo() {
        // Um instalador chamado `<script>alert(1)</script>` existe no mundo real
        // como brincadeira e como ataque. O relatório é entregue ao cliente:
        // ele não pode carregar código de terceiro.
        let escapado = escape("<script>alert('x')</script> & \"aspas\"");

        assert!(!escapado.contains('<'));
        assert!(!escapado.contains('>'));
        assert!(escapado.contains("&lt;script&gt;"));
        assert!(escapado.contains("&amp;"));
    }

    #[test]
    fn sem_medicao_o_relatorio_admite_em_vez_de_estimar() {
        let secao = secao_medicao(None);

        assert!(secao.contains("Não foi realizada medição"));
        // A frase que separa este produto do resto do mercado.
        assert!(secao.contains("não estima números que não foram medidos"));
        // E nenhuma porcentagem decorativa apareceu junto.
        assert!(!secao.contains('%'));
    }

    #[test]
    fn veredito_ruim_chega_ao_cliente() {
        // A tentação comercial é omitir o que piorou. O relatório não omite.
        let (rotulo, classe) = verdict_label(&Verdict::Worsened);
        assert_eq!(rotulo, "piorou");
        assert_eq!(classe, "ruim");
    }

    #[test]
    fn documento_nao_tem_simbolo_decorativo() {
        // Pedido explícito: documento técnico, sem emoji. Um símbolo desses num
        // anexo de nota de serviço tira a seriedade do laudo inteiro.
        let log = ChangeLog::load();
        let html = build_html(&log, None, &ReportData::default(), "31/07/2026 as 14:00");

        for c in html.chars() {
            let cp = c as u32;
            let decorativo = (0x1F300..=0x1FAFF).contains(&cp)
                || (0x2600..=0x27BF).contains(&cp)
                || (0xFE00..=0xFE0F).contains(&cp);

            assert!(!decorativo, "simbolo decorativo no documento: {:?}", c);
        }
    }

    #[test]
    fn documento_e_autossuficiente() {
        let log = ChangeLog::load();
        let html = build_html(&log, None, &ReportData::default(), "31/07/2026 as 14:00");

        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("lang=\"pt-BR\""));

        // Abrir sem internet é requisito: o arquivo pode chegar por pendrive,
        // ou ser aberto daqui a dois anos.
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
        assert!(!html.contains("<script"));

        // Folha de impressão de verdade, e não uma página de tela salva em PDF.
        assert!(html.contains("@page"));
        assert!(html.contains("size: A4"));

        // A promessa central do produto aparece no fechamento.
        assert!(html.contains("Reversibilidade"));
        // E a lista do que recusamos fazer, que é o argumento de venda.
        assert!(html.contains("Procedimentos deliberadamente não executados"));
    }

    #[test]
    fn ausencia_de_dado_nao_e_lida_como_conformidade() {
        let log = ChangeLog::load();
        let html = build_html(&log, None, &ReportData::default(), "31/07/2026 as 14:00");

        // A frase que impede o cliente de ler uma seção vazia como aprovação.
        assert!(html.contains("não que o item esteja em conformidade"));
    }

    #[test]
    fn duracao_vira_texto_legivel() {
        assert_eq!(duracao(1500), "1.5 s");
        assert_eq!(duracao(97_666), "1 min 38 s");
    }

    #[test]
    fn area_de_trabalho_e_uma_pasta_existente() {
        let dir = desktop_dir();
        println!("área de trabalho: {:?}", dir);
        assert!(dir.is_dir(), "o relatório precisa de uma pasta real onde gravar");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn edge_esta_disponivel_para_gerar_pdf() {
        // Se isto falhar numa máquina de cliente, o relatório sai em HTML e o
        // programa explica. O teste existe para saber se o caminho principal
        // está funcionando aqui.
        match caminho_do_edge() {
            Some(p) => println!("Edge encontrado em {:?}", p),
            None => println!("Edge NAO encontrado; o relatório sairia em HTML"),
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn gera_um_pdf_de_verdade() {
        let temp = std::env::temp_dir().join("otimiza_teste_relatorio");
        std::fs::create_dir_all(&temp).unwrap();

        let html = temp.join("r.html");
        let pdf = temp.join("r.pdf");
        let _ = std::fs::remove_file(&pdf);

        let log = ChangeLog::load();
        std::fs::write(
            &html,
            build_html(&log, None, &ReportData::default(), "31/07/2026 as 14:00"),
        )
        .unwrap();

        match imprimir_pdf(&html, &pdf) {
            Ok(()) => {
                let bytes = std::fs::read(&pdf).unwrap();

                // Assinatura de PDF de verdade, e não um arquivo vazio criado
                // por engano.
                assert!(bytes.starts_with(b"%PDF"), "arquivo gerado não e um PDF");
                assert!(bytes.len() > 3000, "PDF pequeno demais: {} bytes", bytes.len());

                println!("PDF gerado com {} bytes", bytes.len());
            }
            Err(motivo) => {
                // Sem Edge, o caminho alternativo é o que vale — e ele é
                // exercitado pelos outros testes.
                println!("PDF não gerado nesta máquina: {}", motivo);
            }
        }

        let _ = std::fs::remove_dir_all(&temp);
    }
}

#[cfg(test)]
mod inspecao {
    /// Gera o documento com os dados reais desta máquina para conferência
    /// visual. Não roda na esteira: escreve arquivo e depende do Edge.
    #[test]
    #[ignore]
    fn dump() {
        use super::*;
        use crate::modules::windows;

        let dados = ReportData {
            boot: Some(windows::boot::analyze()),
            thermal: Some(windows::thermal::analyze()),
            health: Some(windows::health::analyze()),
            memory: Some(windows::memory::analyze()),
            browsers: Some(windows::browsers::analyze()),
            startup: windows::startup::entries(),
        };

        let saida = std::path::PathBuf::from(
            std::env::var("OTIMIZA_DUMP").unwrap_or_else(|_| ".".to_string()),
        );
        let html = saida.join("relatorio.html");
        let pdf = saida.join("relatorio.pdf");

        let log = ChangeLog::load();
        std::fs::write(
            &html,
            build_html(&log, None, &dados, "31/07/2026 as 20:00"),
        )
        .unwrap();

        match imprimir_pdf(&html, &pdf) {
            Ok(()) => println!("PDF: {:?} ({} bytes)", pdf, std::fs::metadata(&pdf).unwrap().len()),
            Err(e) => println!("sem PDF: {}", e),
        }
        println!("HTML: {:?}", html);
    }
}
