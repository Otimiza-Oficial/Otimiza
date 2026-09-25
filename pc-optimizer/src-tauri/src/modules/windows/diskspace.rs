// Liberador de espaço: disco com menos de 10% livre se disfarça de "PC lento". Mostra categoria por categoria
// quanto dá para recuperar e deixa escolher.

use super::shell;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceFinding {
    pub id: String,
    pub name: String,
    pub explanation: String,
    pub bytes: u64,
    pub formatted: String,
    pub cleanable: bool,
    pub requires_admin: bool,
    /// Dois usos: o que se perde ao limpar (o cliente precisa saber ANTES), ou, com `cleanable` falso, por que não se
    /// limpa aqui e onde ir. `categoria_perigosa_avisa_e_nao_e_limpa_por_aqui` exige o motivo.
    pub warning: Option<String>,
    pub medida: Medida,
}

/// Tipado: `bytes: 0` sozinho não distingue "não medi" de "medi e deu zero", e a tela pintava "vazio" em verde
/// ao lado de "não foi possível estimar". Comum: o DISM exige elevação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo")]
pub enum Medida {
    Medido,
    /// O `bytes` é 0 por falta de opção: a tela NÃO pode tratá-lo como "vazio".
    NaoConsegui,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskReport {
    pub drive: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub free_percent: f64,
    /// Sem isto o veredito chegou a anunciar disco cheio.
    pub medida_do_espaco: Medida,
    pub pressure: Option<String>,
    pub recoverable_bytes: u64,
    pub findings: Vec<SpaceFinding>,
}

struct Category {
    id: &'static str,
    name: &'static str,
    explanation: &'static str,
    warning: Option<&'static str>,
    requires_admin: bool,
    cleanable: bool,
    paths: fn() -> Vec<PathBuf>,
}

/// As pastas que o Windows enche sozinho e que entram na conta do espaço.
fn pastas_de(id: &str) -> Vec<PathBuf> {
    let var = |nome: &str| std::env::var(nome).ok().map(PathBuf::from);
    let windows = var("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows"));

    match id {
        "temporarios" => [var("TEMP"), Some(windows.join("Temp"))].into_iter().flatten().collect(),
        "windows_update" => vec![windows.join("SoftwareDistribution").join("Download")],
        "entregas_otimizadas" => vec![windows.join("SoftwareDistribution").join("DeliveryOptimization")],
        "relatorios_de_erro" => [var("LOCALAPPDATA"), var("ProgramData")]
            .into_iter()
            .flatten()
            .flat_map(|raiz| {
                let wer = raiz.join("Microsoft").join("Windows").join("WER");
                [wer.join("ReportQueue"), wer.join("ReportArchive")]
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn local_appdata() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
}

fn windows_dir() -> Option<PathBuf> {
    std::env::var("SystemRoot").ok().map(PathBuf::from)
}

fn system_drive() -> String {
    std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string())
}

static CATEGORIES: &[Category] = &[
    Category {
        id: "temp",
        name: "Arquivos temporários",
        explanation: "Sobras de instaladores e de programas que abriram arquivos temporários e não os apagaram.",
        warning: None,
        requires_admin: false,
        cleanable: true,
        // Pastas e apagar vêm de `limpar.rs`, para as duas telas medirem e apagarem a mesma coisa.
        paths: || pastas_de("temporarios"),
    },
    Category {
        id: "update_cache",
        name: "Instaladores de atualizações",
        explanation: "Os instaladores que o Windows guarda depois de aplicar cada atualização. Não são necessários para o sistema funcionar.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || pastas_de("windows_update"),
    },
    Category {
        id: "windows_old",
        name: "Instalação anterior do Windows",
        explanation: "Cópia da versão antiga do Windows, guardada depois de uma atualização grande. Costuma ser a maior sobra do disco.",
        warning: Some(
            "Apagar remove a possibilidade de voltar para a versão anterior do Windows. \
             Esta pasta pertence ao sistema e resiste a remoção comum — use a Limpeza de \
             Disco do Windows, opção \"Instalações anteriores do Windows\".",
        ),
        requires_admin: true,
        // NÃO limpamos: a pasta é do TrustedInstaller e a remoção comum falha no meio.
        cleanable: false,
        paths: || vec![PathBuf::from(format!("{}\\Windows.old", system_drive()))],
    },
    Category {
        id: "error_reports",
        name: "Relatórios de erro",
        explanation: "Despejos de memória que o Windows salva quando um programa trava, para enviar à Microsoft.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || pastas_de("relatorios_de_erro"),
    },
    Category {
        id: "delivery_optimization",
        name: "Cache de compartilhamento de atualizações",
        explanation: "Pedaços de atualizações que o Windows guardou para distribuir a outros computadores.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        // Até a 2.8 apontava para `ProgramData\Microsoft\Network\Downloader`, a fila do BITS: apagava downloads
        // pendentes.
        paths: || pastas_de("entregas_otimizadas"),
    },
    Category {
        id: "update_logs",
        name: "Registros de atualização",
        explanation: "Arquivos de log que o Windows escreve a cada atualização. Só servem para diagnóstico.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || {
            windows_dir()
                .map(|w| vec![w.join("Logs").join("CBS")])
                .unwrap_or_default()
        },
    },
    Category {
        id: "winsxs",
        name: "Componentes antigos do Windows",
        explanation: "Cópias antigas de componentes do Windows, guardadas para permitir desinstalar \
             atualizações já aplicadas. O DISM decide quanto disso já não faz falta.",
        // Aqui o `warning` é o motivo e o caminho, não a perda.
        warning: Some(
            "A limpeza de verdade (`DISM /StartComponentCleanup`) leva de 5 a 25 minutos \
             e mexe no repositório de componentes do sistema — não cabe num clique rápido \
             daqui. Use a ferramenta de reparo (Analisar e limpar WinSxS) para fazer isso \
             com acompanhamento de progresso e sem risco de interrupção pela metade.",
        ),
        requires_admin: true,
        // Longa e mexe em componentes do sistema: cortada no meio é pior que apontar a ferramenta certa.
        cleanable: false,
        // Sem pasta própria: o tamanho vem do DISM, nunca de `directory_size`.
        paths: || vec![],
    },
    Category {
        id: "browser_cache",
        name: "Cache dos navegadores",
        explanation: "Páginas, imagens e scripts que Chrome, Edge e Firefox guardam no disco para \
             abrir os mesmos sites mais rápido depois.",
        warning: Some("Os sites vão carregar uma vez mais devagar."),
        requires_admin: false,
        cleanable: true,
        paths: || {
            let mut p = Vec::new();
            if let Some(la) = local_appdata() {
                p.push(
                    la.join("Google")
                        .join("Chrome")
                        .join("User Data")
                        .join("Default")
                        .join("Cache"),
                );
                p.push(
                    la.join("Microsoft")
                        .join("Edge")
                        .join("User Data")
                        .join("Default")
                        .join("Cache"),
                );

                // A pasta de perfil tem nome aleatório: varre o diretório de perfis.
                let perfis = la.join("Mozilla").join("Firefox").join("Profiles");
                if let Ok(entradas) = fs::read_dir(&perfis) {
                    for entrada in entradas.filter_map(|e| e.ok()) {
                        p.push(entrada.path().join("cache2"));
                    }
                }
            }
            p
        },
    },
    Category {
        id: "crash_dumps",
        name: "Despejos de memória",
        explanation: "Arquivos que o Windows grava quando o sistema trava (tela azul), usados só \
             para diagnóstico técnico. Ninguém abre isso no dia a dia.",
        warning: None,
        requires_admin: true,
        cleanable: true,
        paths: || {
            windows_dir()
                .map(|w| vec![w.join("MEMORY.DMP"), w.join("Minidump")])
                .unwrap_or_default()
        },
    },
    Category {
        id: "store_cache",
        name: "Cache da Microsoft Store",
        explanation: "Dados temporários que a Microsoft Store guarda para listar e abrir aplicativos \
             mais rápido.",
        warning: Some("A Store vai demorar um pouco mais para abrir na primeira vez."),
        requires_admin: false,
        cleanable: true,
        paths: || {
            local_appdata()
                .map(|la| {
                    vec![la
                        .join("Packages")
                        .join("Microsoft.WindowsStore_8wekyb3d8bbwe")
                        .join("LocalCache")]
                })
                .unwrap_or_default()
        },
    },
];

/// Caminho inacessível conta zero, sem derrubar a varredura.
fn directory_size(dir: &std::path::Path) -> u64 {
    if let Ok(meta) = fs::metadata(dir) {
        if meta.is_file() {
            return meta.len();
        }
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => directory_size(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

pub fn format_size(bytes: u64) -> String {
    const GB: f64 = 1_073_741_824.0;
    const MB: f64 = 1_048_576.0;
    let b = bytes as f64;

    if bytes == 0 {
        "vazio".to_string()
    } else if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else {
        format!("{:.0} KB", b / 1024.0)
    }
}

/// `None` quando o volume do sistema não aparece: `(0, 0)` fazia o veredito anunciar "Restam 0.0 GB" como
/// Critical numa máquina que podia ter meio terabyte livre.
pub(crate) fn disk_usage() -> Option<(u64, u64)> {
    let drive = system_drive();
    let disks = sysinfo::Disks::new_with_refreshed_list();

    for disk in &disks {
        let ponto = disk.mount_point().to_string_lossy().to_uppercase();
        if ponto.starts_with(&drive.to_uppercase()) {
            return Some((disk.total_space(), disk.available_space()));
        }
    }

    None
}

/// PURA: a análise real leva minutos e exige administrador. `None` NÃO é zero. Lê português e inglês: o DISM
/// responde no idioma do sistema.
pub fn estimativa_do_winsxs(saida_do_dism: &str) -> Option<u64> {
    for linha in saida_do_dism.lines() {
        let minuscula = linha.to_lowercase();

        if !(minuscula.contains("recuperável")
            || minuscula.contains("recuperavel")
            || minuscula.contains("reclaimable"))
        {
            continue;
        }

        // `continue`, não `?`: "Reclaimable Packages : 12" (sem unidade) vem antes de "Reclaimable : 2.34 GB" e abortava
        // a busca.
        let depois = match linha.split_once(':') {
            Some((_, depois)) => depois,
            None => continue,
        };
        let bruto = depois.trim();

        let (numero, unidade) = match bruto.split_once(' ') {
            Some(par) => par,
            None => continue,
        };
        let valor: f64 = match numero.replace(',', ".").parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let multiplicador: u64 = match unidade.trim().to_uppercase().as_str() {
            "KB" => 1024,
            "MB" => 1024 * 1024,
            "GB" => 1024 * 1024 * 1024,
            "TB" => 1024u64 * 1024 * 1024 * 1024,
            _ => continue,
        };

        return Some((valor * multiplicador as f64) as u64);
    }

    None
}

/// Com prazo: sem resposta a tempo, a categoria diz `NaoConsegui` (o reparo roda a mesma análise com progresso).
/// Mata o DISM ao desistir (seguia minutos pesando no PC fraco) e lembra a última análise por
/// `VALIDADE_DA_ANALISE` (era um `Dism.exe` por clique).
fn saida_do_dism_analyze_component_store() -> Option<String> {
    if let Some(lembrada) = analise_lembrada() {
        return lembrada;
    }

    let saida = match shell::spawn_capturando(
        "Dism",
        &["/Online", "/Cleanup-Image", "/AnalyzeComponentStore"],
    ) {
        Ok(mut filho) => shell::esperar_com_prazo(
            &mut filho,
            "Dism /Online /Cleanup-Image /AnalyzeComponentStore",
            PRAZO_DO_DISM,
        )
        .ok()
        .filter(|saida| saida.success)
        .map(|saida| saida.stdout),
        Err(_) => None,
    };

    lembrar_analise(saida.clone());
    saida
}

const PRAZO_DO_DISM: Duration = Duration::from_secs(3);

/// O repositório de componentes só muda quando o Windows aplica ou remove atualização.
const VALIDADE_DA_ANALISE: Duration = Duration::from_secs(600);

/// A FALHA também se lembra: sem admin o DISM falha sempre.
static ULTIMA_ANALISE: std::sync::Mutex<Option<(std::time::Instant, Option<String>)>> =
    std::sync::Mutex::new(None);

/// À parte, para o prazo ser testável sem esperar dez minutos.
fn ainda_vale(quando: std::time::Instant, agora: std::time::Instant) -> bool {
    agora.duration_since(quando) < VALIDADE_DA_ANALISE
}

fn analise_lembrada() -> Option<Option<String>> {
    let guarda = ULTIMA_ANALISE.lock().ok()?;
    let (quando, saida) = guarda.as_ref()?;

    if ainda_vale(*quando, std::time::Instant::now()) {
        Some(saida.clone())
    } else {
        None
    }
}

fn lembrar_analise(saida: Option<String>) {
    if let Ok(mut guarda) = ULTIMA_ANALISE.lock() {
        *guarda = Some((std::time::Instant::now(), saida));
    }
}

/// Da estimativa do DISM, nunca do tamanho da pasta, que mente pelos hard links com `C:\Windows`. Separada para
/// ser testável sem rodar o DISM.
fn finding_do_winsxs_a_partir_da_saida(c: &Category, saida_do_dism: Option<&str>) -> SpaceFinding {
    let estimativa = saida_do_dism.and_then(estimativa_do_winsxs);

    let (bytes, medida, explanation, formatted) = match estimativa {
        Some(bytes) => (
            bytes,
            Medida::Medido,
            c.explanation.to_string(),
            format_size(bytes),
        ),
        None => (
            0,
            Medida::NaoConsegui,
            "Não foi possível estimar quanto dá para recuperar do repositório de \
             componentes (WinSxS) agora. A pasta aparece grande no Explorer por causa \
             de hard links com o próprio `C:\\Windows` — o número real só vem da análise \
             do DISM, que pode ser rodada pela ferramenta de reparo."
                .to_string(),
            // NUNCA "vazio": não conseguimos medir.
            "não consegui estimar".to_string(),
        ),
    };

    SpaceFinding {
        id: c.id.to_string(),
        name: c.name.to_string(),
        explanation,
        bytes,
        formatted,
        cleanable: false,
        requires_admin: c.requires_admin,
        warning: c.warning.map(|w| w.to_string()),
        medida,
    }
}

fn finding_do_winsxs(c: &Category) -> SpaceFinding {
    let saida = saida_do_dism_analyze_component_store();
    finding_do_winsxs_a_partir_da_saida(c, saida.as_deref())
}

/// O veredito da abertura usa esta: o WinSxS não é limpável e não muda o veredito, e o DISM rodava minutos para
/// um número que ninguém lia.
pub fn scan_para_o_veredito() -> DiskReport {
    varrer(false)
}

fn varrer(medir_o_winsxs: bool) -> DiskReport {
    let findings: Vec<SpaceFinding> = CATEGORIES
        .iter()
        .map(|c| {
            if c.id == "winsxs" {
                return if medir_o_winsxs {
                    finding_do_winsxs(c)
                } else {
                    finding_do_winsxs_a_partir_da_saida(c, None)
                };
            }

            let bytes: u64 = (c.paths)()
                .iter()
                .filter(|p| p.exists())
                .map(|p| directory_size(p))
                .sum();

            SpaceFinding {
                id: c.id.to_string(),
                name: c.name.to_string(),
                explanation: c.explanation.to_string(),
                bytes,
                formatted: format_size(bytes),
                cleanable: c.cleanable && bytes > 0,
                requires_admin: c.requires_admin,
                warning: c.warning.map(|w| w.to_string()),
                // Somar pastas sempre dá um número: zero é zero de verdade.
                medida: Medida::Medido,
            }
        })
        .collect();

    let medido = disk_usage();
    let (total_bytes, free_bytes) = medido.unwrap_or((0, 0));
    let medida_do_espaco = if medido.is_some() {
        Medida::Medido
    } else {
        Medida::NaoConsegui
    };
    let free_percent = if total_bytes > 0 {
        free_bytes as f64 / total_bytes as f64 * 100.0
    } else {
        0.0
    };

    let pressure = if total_bytes > 0 && free_percent < 10.0 {
        Some(format!(
            "Só {:.0}% de espaço livre. Abaixo de 10% o Windows perde folga para o \
             arquivo de paginação e para atualizações, e o PC fica lento por causa \
             disso — não por causa do processador.",
            free_percent
        ))
    } else {
        None
    };

    // Só o limpável aqui: somar o resto prometeria espaço que o usuário não vai ver.
    let recoverable_bytes = findings.iter().filter(|f| f.cleanable).map(|f| f.bytes).sum();

    let mut findings = findings;
    findings.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    DiskReport {
        drive: system_drive(),
        total_bytes,
        free_bytes,
        free_percent,
        medida_do_espaco,
        pressure,
        recoverable_bytes,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_entrega_otimizada_nao_aponta_mais_para_a_fila_do_bits() {
        let c = CATEGORIES.iter().find(|c| c.id == "delivery_optimization").unwrap();
        assert!((c.paths)().iter().all(|p| !p.to_string_lossy().contains("Downloader")));
    }

    /// Só nos testes: `scan()` lê e escreve o `static` `ULTIMA_ANALISE`, e os testes rodam em paralelo. Um laço de
    /// tentativas só reduzia a corrida; teste que falha de vez em quando ensina a reexecutar sem investigar.
    static TRAVA_ULTIMA_ANALISE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A trava guarda `()`: pegar a guarda de dentro do veneno é seguro.
    fn trava_ultima_analise() -> std::sync::MutexGuard<'static, ()> {
        TRAVA_ULTIMA_ANALISE
            .lock()
            .unwrap_or_else(|envenenada| envenenada.into_inner())
    }

    #[test]
    fn toda_categoria_tem_explicacao_em_portugues() {
        for c in CATEGORIES {
            assert!(!c.explanation.trim().is_empty(), "{} sem explicação", c.id);
            assert!(!c.name.trim().is_empty());
        }
    }

    #[test]
    fn categoria_perigosa_avisa_e_nao_e_limpa_por_aqui() {
        for c in CATEGORIES.iter().filter(|c| !c.cleanable) {
            assert!(
                c.warning.is_some(),
                "{} não é limpável e não explica o motivo",
                c.id
            );
        }
    }

    #[test]
    fn ids_sao_unicos() {
        let mut ids: Vec<&str> = CATEGORIES.iter().map(|c| c.id).collect();
        ids.sort();
        let total = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), total, "id repetido torna a limpeza ambígua");
    }

    #[test]
    fn todo_caminho_fica_dentro_de_pasta_conhecida() {
        // Barreira contra uma categoria que apague algo que importa.
        let permitidos = [
            "temp",
            "softwaredistribution",
            "wer",
            "downloader",
            "logs",
            "windows.old",
            "chrome",
            "edge",
            "firefox",
            "memory.dmp",
            "minidump",
            "microsoft.windowsstore",
        ];

        for c in CATEGORIES {
            for caminho in (c.paths)() {
                let p = caminho.to_string_lossy().to_lowercase();
                assert!(
                    permitidos.iter().any(|permitido| p.contains(permitido)),
                    "categoria {} aponta para fora das pastas conhecidas: {}",
                    c.id,
                    p
                );
            }
        }
    }

    #[test]
    fn formata_tamanhos_para_leitura() {
        assert_eq!(format_size(0), "vazio");
        assert_eq!(format_size(524_288_000), "500 MB");
        assert_eq!(format_size(2_147_483_648), "2.0 GB");
    }

    #[test]
    fn nao_promete_espaco_que_nao_vai_entregar() {
        // Trava: `scan()` mexe em `ULTIMA_ANALISE`.
        let _trava = trava_ultima_analise();

        let relatorio = scan_para_o_veredito();
        let soma_limpavel: u64 = relatorio
            .findings
            .iter()
            .filter(|f| f.cleanable)
            .map(|f| f.bytes)
            .sum();

        assert_eq!(relatorio.recoverable_bytes, soma_limpavel);
    }

    #[test]
    fn o_winsxs_usa_a_estimativa_do_dism_e_nunca_o_tamanho_da_pasta() {
        // O WinSxS aparece com 11,5 GB por hard links com `C:\Windows`; o DISM libera de verdade 1 a 5 GB.
        let saida = "\
Versão: 10.0.26100.1

Imagem : C:\\

Tamanho do Repositório de Componentes Reportável ao Windows Explorer : 11.49 GB
Tamanho Real do Repositório de Componentes : 8.21 GB
Recuperável : 2.34 GB
Limpeza do Repositório de Componentes Recomendada : Sim
A operação foi concluída com êxito.";

        let bytes = estimativa_do_winsxs(saida).expect("a linha Recuperavel existe nesta saida");
        assert!(bytes > 2_400_000_000 && bytes < 2_600_000_000, "veio {}", bytes);
    }

    #[test]
    fn o_achado_do_winsxs_usa_o_numero_do_dism_e_nao_o_tamanho_do_disco() {
        // Se alguém trocar a fonte por `directory_size(...)`, o valor não bate e o teste reprova.
        let categoria = CATEGORIES.iter().find(|c| c.id == "winsxs").expect("categoria winsxs existe");
        let saida = "Recuperável : 2.34 GB\n";

        let achado = finding_do_winsxs_a_partir_da_saida(categoria, Some(saida));
        let esperado = estimativa_do_winsxs(saida).expect("saida de teste tem a linha Recuperavel");

        assert_eq!(achado.bytes, esperado);
    }

    #[test]
    fn winsxs_sem_a_linha_de_recuperavel_vira_nao_sei_e_nao_zero() {
        assert_eq!(estimativa_do_winsxs("A operação falhou. Erro: 0x800f0954"), None);
        assert_eq!(estimativa_do_winsxs(""), None);
    }

    #[test]
    fn a_estimativa_entende_o_dism_em_ingles_tambem() {
        let saida = "Reclaimable Packages : 12\nReclaimable : 2.34 GB\n";
        assert!(estimativa_do_winsxs(saida).is_some(), "nao leu a saida em ingles");
    }

    #[test]
    fn toda_categoria_nova_avisa_o_que_se_perde_quando_ha_o_que_perder() {
        // Trava: `scan()` mexe em `ULTIMA_ANALISE`.
        let _trava = trava_ultima_analise();

        let relatorio = scan_para_o_veredito();

        for id in ["browser_cache", "store_cache"] {
            if let Some(f) = relatorio.findings.iter().find(|f| f.id == id) {
                assert!(f.warning.is_some(), "{} nao diz o que se perde", id);
            }
        }
    }

    #[test]
    fn winsxs_sem_estimativa_chega_na_tela_como_nao_medido_e_nao_como_vazio() {
        let categoria = CATEGORIES
            .iter()
            .find(|c| c.id == "winsxs")
            .expect("categoria winsxs existe");

        let sem_dism = finding_do_winsxs_a_partir_da_saida(categoria, None);
        assert_eq!(sem_dism.bytes, 0);
        assert_eq!(
            sem_dism.medida,
            Medida::NaoConsegui,
            "sem a análise do DISM o achado precisa dizer que NÃO MEDIU"
        );

        let com_dism = finding_do_winsxs_a_partir_da_saida(categoria, Some("Recuperável : 2.34 GB
"));
        assert_eq!(com_dism.medida, Medida::Medido);
    }

    #[test]
    fn as_outras_categorias_medem_de_verdade() {
        for f in scan_para_o_veredito().findings.iter().filter(|f| f.id != "winsxs") {
            assert_eq!(f.medida, Medida::Medido, "{} não é medição do disco", f.id);
        }
    }

    #[test]
    fn a_analise_do_dism_e_lembrada_e_nao_refeita_a_cada_clique() {
        let agora = std::time::Instant::now();
        assert!(ainda_vale(agora, agora), "a análise recém-feita vale");

        let velha = agora
            .checked_sub(VALIDADE_DA_ANALISE + Duration::from_secs(1))
            .expect("o relógio da máquina tem passado suficiente");
        assert!(
            !ainda_vale(velha, agora),
            "análise mais velha que a validade precisa ser refeita"
        );
    }

    /// `ainda_vale` sozinho deixava apagar o early-return inteiro com a suíte verde. Sentinela: uma saída que o DISM
    /// nunca produziria prova que a memória foi consultada.
    #[test]
    fn a_analise_lembrada_volta_da_memoria_sem_subir_o_dism_de_novo() {
        const SENTINELA: &str = "SENTINELA DA MEMORIA (nao veio de Dism.exe)\nRecuperável : 1.00 GB\n";

        // Trava: planta na `ULTIMA_ANALISE` e depende de ninguém mexer nela até ler.
        let _trava = trava_ultima_analise();

        *ULTIMA_ANALISE
            .lock()
            .expect("a memória da análise não está envenenada") =
            Some((std::time::Instant::now(), Some(SENTINELA.to_string())));

        let inicio = std::time::Instant::now();
        let saida = saida_do_dism_analyze_component_store();
        let gasto = inicio.elapsed();

        assert_eq!(
            saida.as_deref(),
            Some(SENTINELA),
            "a análise lembrada foi ignorada: a função subiu o DISM de novo em vez de \
             devolver o que já estava na memória — é um Dism.exe por clique em Limpar",
        );

        assert!(
            gasto < PRAZO_DO_DISM,
            "a resposta lembrada levou {:?} — esperou pelo DISM em vez de lembrar",
            gasto
        );

        // Não deixa a sentinela para `varre_esta_maquina`.
        *ULTIMA_ANALISE
            .lock()
            .expect("a memória da análise não está envenenada") = None;
    }

    /// Sem `#[serde(tag = "tipo")]`, `item.medida.tipo` vira `undefined` na tela e o não medido volta a ser "vazio"
    /// verde, com `cargo test` e `tsc` limpos. Afirma a FORMA do JSON.
    #[test]
    fn a_medida_chega_na_tela_como_objeto_com_campo_tipo() {
        assert_eq!(
            serde_json::to_string(&Medida::NaoConsegui).expect("Medida serializa"),
            r#"{"tipo":"NaoConsegui"}"#
        );
        assert_eq!(
            serde_json::to_string(&Medida::Medido).expect("Medida serializa"),
            r#"{"tipo":"Medido"}"#
        );

        let categoria = CATEGORIES
            .iter()
            .find(|c| c.id == "winsxs")
            .expect("categoria winsxs existe");
        let json = serde_json::to_string(&finding_do_winsxs_a_partir_da_saida(categoria, None))
            .expect("o achado serializa");
        assert!(
            json.contains(r#""medida":{"tipo":"NaoConsegui"}"#),
            "o achado não chega com `medida.tipo` na tela: {}",
            json
        );
    }

    #[test]
    fn a_estimativa_entende_virgula_decimal_do_windows_em_portugues() {
        // O DISM em português escreve "2,34 GB": sem este, apagar o `replace(',', ".")` sumiria a categoria para o
        // brasileiro.
        let com_virgula = estimativa_do_winsxs("Recuperável : 2,34 GB
")
            .expect("a saída em português usa vírgula decimal");
        let com_ponto = estimativa_do_winsxs("Recuperável : 2.34 GB
").expect("e a com ponto");

        assert_eq!(com_virgula, com_ponto, "vírgula e ponto medem o mesmo espaço");
        assert!(com_virgula > 2_400_000_000, "veio {}", com_virgula);
    }

    #[test]
    fn varre_esta_maquina() {
        // Trava: `scan()` mexe em `ULTIMA_ANALISE`.
        let _trava = trava_ultima_analise();

        let r = scan_para_o_veredito();
        println!(
            "{} — {} livres de {} ({:.0}%)",
            r.drive,
            format_size(r.free_bytes),
            format_size(r.total_bytes),
            r.free_percent
        );

        for f in &r.findings {
            println!("  {:<40} {}", f.name, f.formatted);
        }
        println!("recuperável: {}", format_size(r.recoverable_bytes));

        assert_eq!(r.findings.len(), CATEGORIES.len());
        let tamanhos: Vec<u64> = r.findings.iter().map(|f| f.bytes).collect();
        assert!(tamanhos.windows(2).all(|par| par[0] >= par[1]));
    }
}
