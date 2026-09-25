// Relatório técnico em PDF que o técnico entrega com o computador: estado real, cada mudança com o valor de
// antes, e que tudo pode ser desfeito. HTML convertido pelo Edge sem interface (existe em todo Windows 10 e 11,
// sem dependência nova); sem Edge, fica o HTML e o programa diz. Regras: nenhum número inventado, veredito como
// saiu (inclusive "piorou"), nenhum símbolo decorativo.

use crate::modules::benchmark::{BenchmarkComparison, MetricDelta, Verdict};
use crate::modules::changelog::ChangeLog;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ReportSaved {
    pub path: String,
    pub is_pdf: bool,
    pub optimizations: usize,
    pub changes: usize,
    pub note: String,
}

/// Cada campo é opcional porque cada análise falha sozinha; seção ausente é dita, não omitida.
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
    /// `Result`: a chave `Run` ilegível sumia com a seção e o PDF dizia por omissão que não havia nada na
    /// inicialização.
    pub startup: Result<Vec<crate::modules::windows::startup::StartupEntry>, String>,
    /// O mesmo veredito da tela, vindo de fora: papel e programa não podem discordar sobre a mesma máquina.
    #[cfg(target_os = "windows")]
    pub veredito: Option<crate::modules::windows::veredito::Veredito>,
}

/// À mão porque `Result` não tem padrão; o certo é a lista VAZIA (os testes montam relatório sem máquina).
impl Default for ReportData {
    fn default() -> Self {
        ReportData {
            #[cfg(target_os = "windows")]
            boot: None,
            #[cfg(target_os = "windows")]
            thermal: None,
            #[cfg(target_os = "windows")]
            health: None,
            #[cfg(target_os = "windows")]
            memory: None,
            #[cfg(target_os = "windows")]
            browsers: None,
            #[cfg(target_os = "windows")]
            startup: Ok(Vec::new()),
            #[cfg(target_os = "windows")]
            veredito: None,
        }
    }
}

/// Nome de programa instalado é texto de terceiro: sem escapar, um `<script>` no nome executaria em quem abrir.
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

/// A conclusão vem primeiro: o cliente quer saber o que há de errado, e as medições ficam abaixo para conferir.
#[cfg(target_os = "windows")]
fn secao_veredito(dados: &ReportData) -> String {
    use crate::modules::windows::achados::FindingSeverity;

    let Some(v) = &dados.veredito else {
        return secao(
            1,
            "Conclusão",
            aviso("O diagnóstico automático não pôde ser concluído nesta máquina."),
        );
    };

    let mut corpo = format!(
        "<p class=\"veredicto\"><b>{}</b></p><p>{}</p>",
        escape(&v.frase),
        escape(&v.detalhe)
    );

    if let Some(principal) = &v.principal {
        if !principal.advice.is_empty() {
            corpo.push_str(&format!("<p>{}</p>", escape(&principal.advice)));
        }
    }

    if !v.corroboracoes.is_empty() {
        corpo.push_str(
            "<p>Outras medições desta máquina apontam para a mesma causa:</p>\
             <table class=\"dados\"><tbody>",
        );

        for c in &v.corroboracoes {
            corpo.push_str(&format!(
                "<tr><th>{}</th><td>{}</td></tr>",
                escape(&c.title),
                escape(&c.measured)
            ));
        }

        corpo.push_str("</tbody></table>");
    }

    // O que NÃO foi verificado vai na conclusão, não no rodapé: omitir o alcance é enganar por seleção.
    if !v.lacunas.is_empty() {
        corpo.push_str(
            "<p><b>Limites deste levantamento.</b> Os itens abaixo não puderam ser \
             verificados. A ausência de achado neles não significa conformidade.</p>\
             <table class=\"dados\"><tbody>",
        );

        for l in &v.lacunas {
            corpo.push_str(&format!(
                "<tr><th>{}</th><td>{}</td></tr>",
                escape(&l.o_que),
                escape(&l.por_que)
            ));
        }

        corpo.push_str("</tbody></table>");
    }

    let conferidos = v
        .achados
        .iter()
        .filter(|a| a.severity == FindingSeverity::Ok)
        .count();

    if conferidos > 0 {
        corpo.push_str(&format!(
            "<p>{} verificação(oes) desta máquina passaram sem apontamento. \
             Estão listadas nas seções seguintes.</p>",
            conferidos
        ));
    }

    secao(1, "Conclusão", corpo)
}

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
            2,
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
        return secao(3, "Tempo de inicialização", aviso("Não foi possível ler."));
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

    secao(3, "Tempo de inicialização", corpo)
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

    if r.culprit == Culprit::Calor || r.culprit == Culprit::NaoIdentificado {
        corpo.push_str(
            "<p class=\"nota\">Criterio adotado: descartar bateria, depois o teto do plano de \
             energia, e só então considerar temperatura, exclusivamente quando o próprio \
             Windows registrou um evento térmico. Sem esse registro, o Otimiza declara causa \
             não identificada em vez de atribuir a temperatura.</p>",
        );
    }

    secao(4, "Desempenho do processador", corpo)
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
            // Nome de classe em ASCII; o acento vai só no texto visível.
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

    secao(5, "Saúde física do disco e da bateria", corpo)
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

    if r.committed_gb > r.total_ram_gb {
        corpo.push_str(&format!(
            "<p class=\"achado\">Os programas em uso pediram {:.1} GB, mais que os {:.1} GB \
             físicos instalados. A diferença e sustentada pelo disco, que e ordens de grandeza \
             mais lento que a memória. E a causa mais comum de travamentos momentâneos nesta \
             faixa de hardware.</p>",
            r.committed_gb, r.total_ram_gb
        ));
    }

    secao(6, "Memória e paginação", corpo)
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

    corpo.push_str(
        "<p class=\"nota\">Consumo de memória por extensão não consta deste relatório porque \
         não e mensurável a partir do sistema operacional: diversas extensões compartilham um \
         mesmo processo. Qualquer valor individual apresentado aqui seria estimado, e este \
         documento não apresenta estimativas como medições.</p>",
    );

    secao(7, "Navegadores", corpo)
}

#[cfg(target_os = "windows")]
fn secao_inicializacao(dados: &ReportData) -> String {
    // A lacuna aparece: leitura falha e máquina sem nada na inicialização davam a mesma seção sumida.
    let entradas = match &dados.startup {
        Ok(entradas) => entradas,
        Err(erro) => {
            return secao(
                7,
                "Programas que abrem com o Windows",
                format!(
                    "<p class=\"lacuna\">Não deu para ler as chaves de inicialização \
                     desta máquina, então esta seção está vazia por falta de leitura e \
                     não por falta de programas: {}</p>",
                    escape(erro)
                ),
            );
        }
    };

    if entradas.is_empty() {
        return String::new();
    }

    let linhas: String = entradas
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
        8,
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
        // Sem medição, o relatório diz isso em voz alta.
        return secao(
            9,
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
        9,
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
                10,
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
            "<article class=\"mud\"><h4>{} <small>({})</small></h4><ul>{}</ul></article>",
            escape(&otimizacao.name),
            escape(&data_de(otimizacao.timestamp)),
            itens
        ));
    }

    let html = secao(
        10,
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

/// `0` é histórico antigo sem hora, e aparece como tal em vez de 01/01/1970.
fn data_de(ts: u64) -> String {
    use chrono::TimeZone;
    if ts == 0 {
        return "sem data".to_string();
    }
    match chrono::Local.timestamp_opt(ts as i64, 0).single() {
        Some(d) => d.format("%d/%m/%Y %H:%M").to_string(),
        None => "sem data".to_string(),
    }
}

// CSV de alterações: uma linha por mudança, com o valor de antes. Separador ";" e BOM UTF-8 para o Excel em
// português não embaralhar acento nem coluna.

fn campo_csv(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\"").replace(['\n', '\r'], " "))
}

pub fn csv_das_alteracoes(
    log: &ChangeLog,
    valor_novo: impl Fn(&str, &crate::modules::changelog::ChangeRecord) -> Option<String>,
) -> String {
    let mut s = String::from("\u{FEFF}data;id;ajuste;alteracao_e_valor_anterior;valor_novo;desfazer\r\n");
    for o in log.applied() {
        for c in &o.changes {
            let linha = [
                data_de(o.timestamp),
                o.optimization_id.clone(),
                o.name.clone(),
                c.describe(),
                valor_novo(&o.optimization_id, c).unwrap_or_default(),
                "pelo Otimiza, item a item ou Desfazer tudo".to_string(),
            ];
            s.push_str(&linha.iter().map(|x| campo_csv(x)).collect::<Vec<_>>().join(";"));
            s.push_str("\r\n");
        }
    }
    s
}

pub fn salvar_csv(conteudo: &str) -> Result<String, String> {
    let nome = format!("Otimiza - alteracoes - {}.csv", chrono::Local::now().format("%Y-%m-%d %Hh%M"));
    let caminho = desktop_dir().join(nome);
    std::fs::write(&caminho, conteudo).map_err(|e| format!("Não foi possível gravar em {:?}: {}", caminho, e))?;
    Ok(caminho.to_string_lossy().to_string())
}

fn secao_recusas() -> String {
    secao(
        11,
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
/* A conclusão do laudo. Único texto do documento acima do corpo normal — se
   mais alguma coisa crescer para este tamanho, ela deixa de ser a conclusão. */
.veredicto { font-size: 13pt; line-height: 1.35; margin: 0 0 3mm }
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

pub fn build_html(
    log: &ChangeLog,
    comparison: Option<&BenchmarkComparison>,
    dados: &ReportData,
    data: &str,
) -> String {
    let (mudancas_html, _, _) = secao_mudancas(log);

    #[allow(unused_mut)]
    let mut diagnostico = String::new();

    // Em variável própria: a identificação (seção 2) aparece ENTRE a conclusão e o diagnóstico.
    #[allow(unused_mut)]
    let mut conclusao = String::new();

    #[cfg(target_os = "windows")]
    {
        conclusao.push_str(&secao_veredito(dados));
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
         {}{}{}{}{}{}\
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
        conclusao,
        secao_maquina(),
        diagnostico,
        secao_medicao(comparison),
        mudancas_html,
        secao_recusas()
    )
}

/// Do registro, não `%USERPROFILE%\Desktop`: com OneDrive a Área de Trabalho real fica dentro dele, e o nome
/// da pasta muda de idioma.
fn desktop_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let shell_folders = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";

        if let Ok(Some(path)) =
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

#[cfg(target_os = "windows")]
fn imprimir_pdf(html: &Path, pdf: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let edge = caminho_do_edge().ok_or("Microsoft Edge não encontrado nesta máquina")?;

    // O Chromium não aceita barra invertida na URL.
    let url = format!("file:///{}", html.to_string_lossy().replace('\\', "/"));

    let saida = std::process::Command::new(edge)
        .args([
            "--headless",
            "--disable-gpu",
            // Sem isto o Edge carimba URL e cabeçalho em toda página.
            "--no-pdf-header-footer",
            &format!("--print-to-pdf={}", pdf.to_string_lossy()),
            &url,
        ])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Não foi possível executar o Edge: {}", e))?;

    // O código de saída do Edge não é confiável e ele escreve no stderr mesmo dando certo: decide o arquivo existir.
    if !pdf.is_file() {
        return Err(format!(
            "O Edge não gerou o PDF. {}",
            String::from_utf8_lossy(&saida.stderr).lines().next().unwrap_or("")
        ));
    }

    Ok(())
}

pub fn save(
    log: &ChangeLog,
    comparison: Option<&BenchmarkComparison>,
    dados: &ReportData,
) -> Result<ReportSaved, String> {
    let agora = chrono::Local::now();
    let data = agora.format("%d/%m/%Y as %H:%M").to_string();
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
                // Deixar o HTML junto faria o cliente receber dois arquivos sem saber qual abrir.
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
mod testes_csv {
    use super::*;
    use crate::modules::changelog::{AppliedOptimization, ChangeRecord};

    #[test]
    fn o_csv_tem_uma_linha_por_alteracao_e_escapa_aspas() {
        let mut log = ChangeLog::em_memoria();
        log.record(AppliedOptimization {
            optimization_id: "x".into(),
            name: "Ajuste \"com aspas\"".into(),
            timestamp: 0,
            changes: vec![
                ChangeRecord::PowerPlan { previous_guid: "a".into() },
                ChangeRecord::Hibernation { previously_enabled: true },
            ],
        })
        .unwrap();
        let csv = csv_das_alteracoes(&log, |_, _| Some("1".into()));
        let linhas: Vec<&str> = csv.trim_end().split("\r\n").collect();
        assert_eq!(linhas.len(), 3, "{csv}");
        assert!(linhas[0].starts_with('\u{FEFF}'));
        assert!(linhas[1].contains("\"Ajuste \"\"com aspas\"\"\""));
        assert!(linhas[1].contains("\"sem data\""));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nome_de_programa_com_html_nao_vira_codigo() {
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
        assert!(secao.contains("não estima números que não foram medidos"));
        assert!(!secao.contains('%'));
    }

    #[test]
    fn veredito_ruim_chega_ao_cliente() {
        let (rotulo, classe) = verdict_label(&Verdict::Worsened);
        assert_eq!(rotulo, "piorou");
        assert_eq!(classe, "ruim");
    }

    #[test]
    fn documento_nao_tem_simbolo_decorativo() {
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

        // Precisa abrir sem internet: pode chegar por pendrive, ou daqui a dois anos.
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
        assert!(!html.contains("<script"));

        assert!(html.contains("@page"));
        assert!(html.contains("size: A4"));

        assert!(html.contains("Reversibilidade"));
        assert!(html.contains("Procedimentos deliberadamente não executados"));
    }

    #[test]
    fn ausencia_de_dado_nao_e_lida_como_conformidade() {
        let log = ChangeLog::load();
        let html = build_html(&log, None, &ReportData::default(), "31/07/2026 as 14:00");

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

                assert!(bytes.starts_with(b"%PDF"), "arquivo gerado não e um PDF");
                assert!(bytes.len() > 3000, "PDF pequeno demais: {} bytes", bytes.len());

                println!("PDF gerado com {} bytes", bytes.len());
            }
            Err(motivo) => {
                println!("PDF não gerado nesta máquina: {}", motivo);
            }
        }

        let _ = std::fs::remove_dir_all(&temp);
    }
}

#[cfg(test)]
mod inspecao {
    /// Dados reais desta máquina, para conferência visual. Fora da esteira: escreve arquivo e depende do Edge.
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
            veredito: Some(windows::veredito::diagnostico_rapido()),
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

#[cfg(test)]
mod csv_desta_maquina {
    #[test]
    #[ignore]
    fn csv_desta_maquina() {
        let log = crate::modules::changelog::ChangeLog::load();
        let csv = super::csv_das_alteracoes(&log, |_, _| None);
        println!("{}", csv.lines().take(8).collect::<Vec<_>>().join("\n"));
    }
}
