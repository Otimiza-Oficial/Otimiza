// Relatório de atendimento
//
// Quem usa o Otimiza profissionalmente tem um problema que nenhum ajuste de
// registro resolve: provar o serviço. O cliente entrega um PC lento, recebe um
// PC melhor e não tem como saber o que foi feito — o que coloca o técnico
// honesto no mesmo balaio de quem só reinicia a máquina e cobra.
//
// Este módulo gera um arquivo que o técnico entrega junto com o computador.
// Ele lista cada mudança pelo nome, com o valor que existia antes, e diz que
// tudo pode ser desfeito. É o oposto do "otimizado!" sem prestação de contas.
//
// Três regras governam o texto gerado:
//
// 1. Nada de número inventado. Se não houve medição, o relatório diz que não
//    houve — em vez de estampar uma porcentagem decorativa.
// 2. O veredito da medição vai como saiu, inclusive "sem diferença" e "piorou".
// 3. O arquivo é autossuficiente: abre sem internet, em qualquer navegador, e
//    continua legível daqui a anos.

use crate::modules::benchmark::{BenchmarkComparison, MetricDelta, Verdict};
use crate::modules::changelog::ChangeLog;
use std::path::PathBuf;

/// Onde o relatório foi gravado.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReportSaved {
    pub path: String,
    pub optimizations: usize,
    pub changes: usize,
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

fn linha_metrica(metric: &MetricDelta) -> String {
    let (rotulo, classe) = verdict_label(&metric.verdict);

    // O sinal explícito evita a leitura errada mais comum: em métricas onde
    // menos é melhor, "-12%" é uma boa notícia.
    let variacao = format!("{:+.1}%", metric.change_percent);

    format!(
        "<tr><td>{}</td><td class=\"num\">{:.1} {}</td><td class=\"num\">{:.1} {}</td>\
         <td class=\"num\">{}</td><td class=\"v {}\">{}</td></tr>",
        escape(&metric.label),
        metric.before,
        escape(&metric.unit),
        metric.after,
        escape(&metric.unit),
        variacao,
        classe,
        rotulo
    )
}

fn secao_medicao(comparison: Option<&BenchmarkComparison>) -> String {
    let Some(c) = comparison else {
        // O caso mais importante deste módulo. Sem medição, o relatório precisa
        // dizer isso em voz alta — é exatamente aqui que um produto desonesto
        // colocaria "desempenho melhorado em 40%".
        return "<h2>Medição de desempenho</h2>\
                <p class=\"aviso\">Não foi feita medição antes e depois neste \
                atendimento. Sem as duas medidas não há como afirmar ganho, e este \
                relatório não estima números que não foram medidos.</p>"
            .to_string();
    };

    let linhas: String = c.metrics.iter().map(linha_metrica).collect();

    format!(
        "<h2>Medição de desempenho</h2>\
         <p>{}</p>\
         <table><thead><tr><th>O que foi medido</th><th>Antes</th><th>Depois</th>\
         <th>Variação</th><th>Leitura</th></tr></thead><tbody>{}</tbody></table>\
         <p class=\"nota\">Medidas marcadas como <strong>só referência</strong> oscilam \
         demais sozinhas para provar qualquer coisa, então aparecem sem veredito de \
         propósito.</p>",
        escape(&c.summary),
        linhas
    )
}

fn secao_mudancas(log: &ChangeLog) -> (String, usize, usize) {
    let aplicadas = log.applied();

    if aplicadas.is_empty() {
        return (
            "<h2>O que foi alterado</h2>\
             <p class=\"aviso\">Nenhuma otimização do Otimiza está aplicada nesta \
             máquina no momento.</p>"
                .to_string(),
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
            "<article class=\"mud\"><h3>{}</h3><ul>{}</ul></article>",
            escape(&otimizacao.name),
            itens
        ));
    }

    let html = format!(
        "<h2>O que foi alterado</h2>\
         <p>{} otimização(ões), {} alteração(ões) no total. Cada linha mostra o \
         valor que existia antes — é esse valor que volta se você desfizer.</p>{}",
        aplicadas.len(),
        total_mudancas,
        blocos
    );

    (html, aplicadas.len(), total_mudancas)
}

fn secao_maquina() -> String {
    #[cfg(target_os = "windows")]
    {
        use crate::modules::windows::hardware::{profile, StorageKind};

        let h = profile();
        let armazenamento = match h.system_storage {
            StorageKind::Ssd => "SSD",
            StorageKind::Hdd => "HD mecânico",
            StorageKind::Unknown => "não identificado",
        };

        format!(
            "<h2>A máquina</h2><table><tbody>\
             <tr><td>Memória instalada</td><td class=\"num\">{:.1} GB</td></tr>\
             <tr><td>Núcleos lógicos</td><td class=\"num\">{}</td></tr>\
             <tr><td>Disco do sistema</td><td class=\"num\">{}</td></tr>\
             </tbody></table>",
            h.total_ram_gb, h.logical_cores, armazenamento
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        String::new()
    }
}

const ESTILO: &str = "
:root { color-scheme: light }
* { box-sizing: border-box }
body { margin: 0; padding: 48px 24px; background: #f4f3f1; color: #16150f;
  font: 15px/1.6 -apple-system, 'Segoe UI', system-ui, sans-serif; }
main { max-width: 820px; margin: 0 auto; background: #fff; padding: 48px;
  border: 1px solid #ddd9d2; }
header { border-bottom: 2px solid #16150f; padding-bottom: 24px; margin-bottom: 32px }
h1 { margin: 0 0 4px; font-size: 26px; letter-spacing: -0.02em }
header p { margin: 0; color: #6b675e; font-size: 13px }
h2 { margin: 40px 0 12px; font-size: 17px; border-bottom: 1px solid #e4e0d9;
  padding-bottom: 8px }
h3 { margin: 0 0 6px; font-size: 14px }
table { width: 100%; border-collapse: collapse; margin: 14px 0; font-size: 14px }
th { text-align: left; font-size: 11px; text-transform: uppercase;
  letter-spacing: 0.08em; color: #6b675e; padding: 6px 10px;
  border-bottom: 1px solid #ddd9d2 }
td { padding: 8px 10px; border-bottom: 1px solid #efece7 }
.num { text-align: right; font-variant-numeric: tabular-nums;
  font-family: ui-monospace, Consolas, monospace }
.v { text-align: right; font-weight: 600; font-size: 13px }
.v.bom { color: #1a7a4c } .v.ruim { color: #b3261e } .v.neutro { color: #6b675e }
.mud { margin: 14px 0; padding: 14px 16px; background: #faf9f7;
  border-left: 3px solid #16150f }
.mud ul { margin: 0; padding-left: 18px; font-size: 13px; color: #4a463e }
.mud li { margin: 3px 0; font-family: ui-monospace, Consolas, monospace;
  font-size: 12px }
.aviso { padding: 14px 16px; background: #fdf6e3; border-left: 3px solid #b8860b;
  font-size: 14px }
.nota { font-size: 12px; color: #6b675e }
footer { margin-top: 48px; padding-top: 20px; border-top: 1px solid #ddd9d2;
  font-size: 12px; color: #6b675e }
@media print { body { padding: 0; background: #fff }
  main { border: 0; padding: 0 } }
";

/// Monta o HTML completo do relatório.
pub fn build_html(log: &ChangeLog, comparison: Option<&BenchmarkComparison>, data: &str) -> String {
    let (mudancas_html, _, _) = secao_mudancas(log);

    format!(
        "<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>Otimiza — relatório de atendimento</title><style>{}</style></head>\
         <body><main>\
         <header><h1>Relatório de atendimento</h1>\
         <p>Gerado pelo Otimiza em {}</p></header>\
         {}{}{}\
         <footer><p><strong>Tudo o que está aqui pode ser desfeito.</strong> O Otimiza \
         grava o valor anterior antes de escrever qualquer coisa; \"Desfazer tudo\" \
         devolve o que existia, não algo parecido. As exceções são apagar arquivos e \
         limpar o cache de atualizações, que não têm volta e ficam fora do processo \
         automático.</p>\
         <p>Este relatório mostra apenas o que foi realmente feito nesta máquina. Se \
         alguma seção diz que não houve medição ou não houve mudança, é porque não \
         houve.</p></footer>\
         </main></body></html>",
        ESTILO,
        escape(data),
        secao_maquina(),
        secao_medicao(comparison),
        mudancas_html
    )
}

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

        if let Some(path) = crate::modules::windows::registry::read_text("HKCU", shell_folders, "Desktop") {
            let candidate = PathBuf::from(&path);
            if candidate.is_dir() {
                return candidate;
            }
        }
    }

    // Sem registro legível, a pasta do usuário ainda é melhor que a pasta do
    // executável — pelo menos o usuário sabe onde procurar.
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Gera o relatório e grava na Área de Trabalho.
pub fn save(log: &ChangeLog, comparison: Option<&BenchmarkComparison>) -> Result<ReportSaved, String> {
    let agora = chrono::Local::now();
    let data = agora.format("%d/%m/%Y às %H:%M").to_string();
    // Nome de arquivo separado do texto: barra não pode ir para nome de arquivo,
    // e o formato ano-mês-dia mantém os relatórios em ordem na pasta.
    let arquivo = format!("Otimiza - relatorio - {}.html", agora.format("%Y-%m-%d %Hh%M"));

    let html = build_html(log, comparison, &data);
    let (_, optimizations, changes) = secao_mudancas(log);

    let destino = desktop_dir().join(&arquivo);

    std::fs::write(&destino, html)
        .map_err(|e| format!("Não foi possível gravar o relatório em {:?}: {}", destino, e))?;

    Ok(ReportSaved {
        path: destino.to_string_lossy().to_string(),
        optimizations,
        changes,
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

        assert!(secao.contains("Não foi feita medição"));
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

        let (rotulo, _) = verdict_label(&Verdict::NoMeasurableChange);
        assert_eq!(rotulo, "sem diferença medível");
    }

    #[test]
    fn sem_otimizacao_aplicada_o_relatorio_nao_finge() {
        // Lê o histórico real desta máquina sem escrever nada nele.
        let log = ChangeLog::load();
        let (html, quantas, mudancas) = secao_mudancas(&log);

        if quantas == 0 {
            assert!(html.contains("Nenhuma otimização"));
            assert_eq!(mudancas, 0);
        } else {
            // Máquina de desenvolvimento com otimizações aplicadas de verdade:
            // então o relatório tem que listar cada uma.
            assert!(html.contains("alteração(ões) no total"));
        }
    }

    #[test]
    fn html_gerado_e_autossuficiente() {
        let log = ChangeLog::load();
        let html = build_html(&log, None, "30/07/2026 às 14:00");

        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("lang=\"pt-BR\""));
        assert!(html.contains("<style>"));

        // Abrir sem internet é requisito: o cliente pode receber o arquivo por
        // pendrive, ou abrir daqui a dois anos.
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
        assert!(!html.contains("<script"));

        // A promessa central do produto aparece no rodapé.
        assert!(html.contains("pode ser desfeito"));
    }

    #[test]
    fn variacao_negativa_mantem_o_sinal() {
        let metric = MetricDelta {
            key: "hitches_per_minute".to_string(),
            label: "Travadas por minuto".to_string(),
            unit: "/min".to_string(),
            before: 20.0,
            after: 8.0,
            change_percent: -60.0,
            verdict: Verdict::Improved,
            explanation: String::new(),
        };

        let linha = linha_metrica(&metric);
        // Em métrica onde menos é melhor, o sinal é o que evita a leitura errada.
        assert!(linha.contains("-60.0%"));
        assert!(linha.contains("melhorou"));
    }

    #[test]
    fn area_de_trabalho_e_uma_pasta_existente() {
        let dir = desktop_dir();
        println!("área de trabalho: {:?}", dir);
        assert!(dir.is_dir(), "o relatório precisa de uma pasta real onde gravar");
    }
}
